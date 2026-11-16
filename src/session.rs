//! One terminal: a libghostty-vt `Terminal` plus where its bytes come from
//! (a daemon PTY, or a tmux pane through a control-mode gateway).
//!
//! Everything here runs on the Qt GUI thread; the C++ view watches `fd()`
//! with a QSocketNotifier and calls `on_readable`, then `frame` to paint.

use crate::bridge::ffi::{Frame, Run};
use crate::keymap;
use crate::proto::*;
use crate::tmux::{self, Gateway, PaneShared};
use libghostty_vt::{
    fmt::{Format, Formatter, FormatterOptions},
    key, mouse,
    render::{CellIterator, CursorVisualStyle, Dirty, RenderState, RowIterator},
    screen::{CellWide, RowSemanticPrompt, Screen},
    selection::{
        gesture::{Autoscroll, AutoscrollTickEvent, DragEvent, Geometry, Gesture, PressEvent, ReleaseEvent},
        Selection,
    },
    style::RgbColor,
    terminal::{
        ConformanceLevel, DeviceAttributeFeature, DeviceAttributes, DeviceType, Mode, Options, Point,
        PointCoordinate, PrimaryDeviceAttributes, ScrollViewport, SecondaryDeviceAttributes,
    },
    Terminal,
};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::io::{ErrorKind, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub const SIG_DIRTY: u32 = 1;
pub const SIG_EXITED: u32 = 2;
pub const SIG_EVENTS: u32 = 4;

pub const FLAG_BOLD: u32 = 1;
pub const FLAG_ITALIC: u32 = 2;
pub const FLAG_UNDERLINE: u32 = 4;
pub const FLAG_STRIKE: u32 = 8;
pub const FLAG_FAINT: u32 = 16;
pub const FLAG_HAS_BG: u32 = 32;
pub const FLAG_INVISIBLE: u32 = 64;

/// Find highlight colors (as iTerm2: yellow matches, orange current one).
const MATCH_BG: u32 = 0xf2cc60;
const MATCH_CURRENT_BG: u32 = 0xff9632;
const MATCH_FG: u32 = 0x1a1a1a;

const SCROLLBACK: usize = 10_000;

/// libghostty limits scrollback in bytes, not lines: a row takes about
/// 9.3 bytes per column (measured). Budget for 300 columns so at least
/// `lines` rows are kept at any usual width (more when narrower).
pub fn scrollback_bytes(lines: u32) -> usize {
    if lines >= 10_000_000 {
        return usize::MAX;
    }
    (lines as usize).saturating_mul(2_900) + (1 << 20)
}

enum Backend {
    Daemon { stream: UnixStream, reader: FrameReader, id: String },
    Tmux { gw: Rc<RefCell<Gateway>>, pane: u32, shared: Rc<RefCell<PaneShared>> },
}

pub struct Session {
    backend: Backend,
    term: Terminal<'static, 'static>,
    rs: RenderState<'static>,
    rows_it: RowIterator<'static>,
    cells_it: CellIterator<'static>,
    key_enc: key::Encoder<'static>,
    key_ev: key::Event<'static>,
    mouse_enc: mouse::Encoder<'static>,
    mouse_ev: mouse::Event<'static>,
    gesture: Gesture<'static>,
    press: PressEvent<'static>,
    release: ReleaseEvent<'static>,
    drag: DragEvent<'static>,
    autotick: AutoscrollTickEvent<'static>,
    selecting: bool,
    buttons: u8,
    outbox: Rc<RefCell<Vec<u8>>>,
    title_changed: Rc<Cell<bool>>,
    pwd_changed: Rc<Cell<bool>>,
    bell: Rc<Cell<bool>>,
    size: Rc<Cell<(u16, u16, u32, u32)>>,
    events: Vec<Value>,
    /// Set while this (daemon) session is a tmux control-mode gateway.
    gateway: Option<Rc<RefCell<Gateway>>>,
    carry: Vec<u8>,
    exited: bool,
    last_cursor_row: Option<u16>,
    last_full_size: (u16, u16),
    selected: String,
    t0: Instant,
    /// Selection colors from the theme (None: reverse video).
    sel_bg: Option<u32>,
    sel_fg: Option<u32>,
    force_full: bool,
    opts: Options2,
    selected_html: String,
    selected_vt: String,
    log: Option<std::fs::File>,
    clipboard: Rc<RefCell<Option<String>>>,
    find: Option<FindState>,
    taken: bool,
    /// Minimum-contrast results per (fg, bg) pair.
    contrast_cache: std::collections::HashMap<(u32, u32), u32>,
    triggers: Vec<(regex::Regex, TriggerSpec)>,
    /// Screen row up to which output has been checked by triggers.
    trigger_row: usize,
    /// Trigger highlights: (row, start, end, color), sorted by row.
    highlights: Vec<(usize, u16, u16, u32)>,
}

struct FindState {
    re: regex::Regex,
    matches: Vec<crate::find::Match>,
    current: Option<usize>,
}

/// Behaviors from the profile and global settings.
#[derive(serde::Deserialize, Clone)]
#[serde(default)]
struct Options2 {
    mouse_reporting: bool,
    alternate_scroll: bool,
    allow_title: bool,
    bracketed_paste: bool,
    alt_mode: String,
    word_chars: String,
    triple_click_wrapped: bool,
    clipboard_access: bool,
    copy_trailing_newline: bool,
    /// iTerm2's Minimum Contrast, 0 (off) to 1.
    minimum_contrast: f64,
    triggers: Vec<TriggerSpec>,
}

/// A trigger: when output matches `regex`, do `action` (highlight, notify,
/// send, bell) with `param` (a color for highlight, text for send/notify).
#[derive(serde::Deserialize, Clone, Default)]
#[serde(default)]
struct TriggerSpec {
    regex: String,
    action: String,
    param: String,
    enabled: bool,
}

impl Default for Options2 {
    fn default() -> Self {
        Self {
            mouse_reporting: true,
            alternate_scroll: true,
            allow_title: true,
            bracketed_paste: true,
            alt_mode: "esc".into(),
            word_chars: "/-+\\~_.".into(),
            triple_click_wrapped: true,
            clipboard_access: true,
            copy_trailing_newline: false,
            minimum_contrast: 0.0,
            triggers: Vec::new(),
        }
    }
}

pub fn session_open(
    attach_id: &str,
    tmux_pane: &str,
    cwd: &str,
    cols: u16,
    rows: u16,
    cell_w: u32,
    cell_h: u32,
    scrollback: u32,
    command: &str,
    term_name: &str,
    startup_input: &str,
) -> Result<Box<Session>, String> {
    let (cols, rows) = (cols.max(2), rows.max(1));
    let backend = if !tmux_pane.is_empty() {
        let (key, pane) = tmux_pane.rsplit_once(':').ok_or("bad tmux pane")?;
        let pane: u32 = pane.trim_start_matches('%').parse().map_err(|_| "bad tmux pane id")?;
        let gw = tmux::lookup(key).ok_or("tmux connection is gone")?;
        let shared = gw.borrow().panes.get(&pane).cloned().ok_or("unknown tmux pane")?;
        Backend::Tmux { gw, pane, shared }
    } else {
        let mut stream = crate::client::connect().map_err(|e| format!("daemon: {e}"))?;
        if attach_id.is_empty() {
            write_json(&mut stream, T_CREATE, &Create { cols, rows, cwd: cwd.to_string(), command: command.to_string(), term: term_name.to_string(), input: startup_input.to_string() })
        } else {
            write_json(&mut stream, T_ATTACH, &Attach { id: attach_id.to_string(), cols, rows })
        }
        .map_err(|e| e.to_string())?;
        stream.set_nonblocking(true).map_err(|e| e.to_string())?;
        Backend::Daemon { stream, reader: FrameReader::default(), id: attach_id.to_string() }
    };

    let (cols, rows) = match &backend {
        Backend::Tmux { shared, .. } => shared.borrow().grid,
        _ => (cols, rows),
    };
    let mut term = Terminal::new(Options { cols, rows, max_scrollback: scrollback_bytes(if scrollback > 0 { scrollback } else { SCROLLBACK as u32 }) }).map_err(|e| e.to_string())?;
    term.resize(cols, rows, cell_w.max(1), cell_h.max(1)).map_err(|e| e.to_string())?;

    let outbox: Rc<RefCell<Vec<u8>>> = Rc::default();
    let title_changed = Rc::new(Cell::new(true));
    let pwd_changed = Rc::new(Cell::new(false));
    let bell = Rc::new(Cell::new(false));
    let size = Rc::new(Cell::new((cols, rows, cell_w, cell_h)));
    let e = |e: libghostty_vt::Error| e.to_string();
    {
        let outbox = outbox.clone();
        term.on_pty_write(move |_t, data| outbox.borrow_mut().extend_from_slice(data)).map_err(e)?;
    }
    {
        let f = title_changed.clone();
        term.on_title_changed(move |_t| f.set(true)).map_err(e)?;
    }
    {
        let f = pwd_changed.clone();
        term.on_pwd_changed(move |_t| f.set(true)).map_err(e)?;
    }
    {
        let f = bell.clone();
        term.on_bell(move |_t| f.set(true)).map_err(e)?;
    }
    {
        let size = size.clone();
        term.on_size(move |_t| {
            let (c, r, w, h) = size.get();
            Some(libghostty_vt::terminal::SizeReportSize { rows: r, columns: c, cell_width: w, cell_height: h })
        })
        .map_err(e)?;
    }
    term.on_device_attributes(|_t| {
        Some(DeviceAttributes {
            primary: PrimaryDeviceAttributes::new(
                ConformanceLevel::VT220,
                &[
                    DeviceAttributeFeature::COLUMNS_132,
                    DeviceAttributeFeature::SELECTIVE_ERASE,
                    DeviceAttributeFeature::ANSI_COLOR,
                ],
            ),
            secondary: SecondaryDeviceAttributes { device_type: DeviceType::VT220, firmware_version: 1, rom_cartridge: 0 },
            tertiary: Default::default(),
        })
    })
    .map_err(e)?;
    let clipboard: Rc<RefCell<Option<String>>> = Rc::default();
    {
        let clip = clipboard.clone();
        term.on_clipboard_write(move |_t, w| {
            if let Some(c) = w.contents().find(|c| c.mime.starts_with("text/plain")).or_else(|| w.contents().next()) {
                *clip.borrow_mut() = Some(c.data.to_string());
            }
            Ok(())
        })
        .map_err(e)?;
    }
    term.on_xtversion(|_t| Some(concat!("Tessera ", env!("CARGO_PKG_VERSION")))).map_err(e)?;

    let mut press = PressEvent::new().map_err(e)?;
    press.set_repeat_interval(Duration::from_millis(500)).map_err(e)?;

    let mut s = Box::new(Session {
        backend,
        term,
        rs: RenderState::new().map_err(e)?,
        rows_it: RowIterator::new().map_err(e)?,
        cells_it: CellIterator::new().map_err(e)?,
        key_enc: key::Encoder::new().map_err(e)?,
        key_ev: key::Event::new().map_err(e)?,
        mouse_enc: mouse::Encoder::new().map_err(e)?,
        mouse_ev: mouse::Event::new().map_err(e)?,
        gesture: Gesture::new().map_err(e)?,
        press,
        release: ReleaseEvent::new().map_err(e)?,
        drag: DragEvent::new().map_err(e)?,
        autotick: AutoscrollTickEvent::new().map_err(e)?,
        selecting: false,
        buttons: 0,
        outbox,
        title_changed,
        pwd_changed,
        bell,
        size,
        events: Vec::new(),
        gateway: None,
        carry: Vec::new(),
        exited: false,
        last_cursor_row: None,
        last_full_size: (0, 0),
        selected: String::new(),
        t0: Instant::now(),
        sel_bg: None,
        sel_fg: None,
        force_full: true,
        opts: Options2::default(),
        selected_html: String::new(),
        selected_vt: String::new(),
        log: None,
        clipboard,
        find: None,
        taken: false,
        contrast_cache: std::collections::HashMap::new(),
        triggers: Vec::new(),
        trigger_row: 0,
        highlights: Vec::new(),
    });
    // A tmux pane may already have its captured contents waiting.
    if matches!(s.backend, Backend::Tmux { .. }) {
        s.on_readable();
    }
    Ok(s)
}

/// Write one frame to a non-blocking socket, waiting out short stalls.
pub fn write_blocking(stream: &mut UnixStream, tag: u8, payload: &[u8]) {
    let len = (payload.len() + 1) as u32;
    let mut buf = Vec::with_capacity(payload.len() + 5);
    buf.extend_from_slice(&len.to_le_bytes());
    buf.push(tag);
    buf.extend_from_slice(payload);
    let mut off = 0;
    let start = Instant::now();
    while off < buf.len() {
        match stream.write(&buf[off..]) {
            Ok(0) => return,
            Ok(n) => off += n,
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                if start.elapsed() > Duration::from_secs(2) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(_) => return,
        }
    }
}

fn rgb(c: RgbColor) -> u32 {
    (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32
}

impl Session {
    pub fn fd(&self) -> i32 {
        match &self.backend {
            Backend::Daemon { stream, .. } => stream.as_raw_fd(),
            Backend::Tmux { shared, .. } => shared.borrow().wake_fd(),
        }
    }

    pub fn id(&self) -> String {
        match &self.backend {
            Backend::Daemon { id, .. } => id.clone(),
            Backend::Tmux { gw, pane, .. } => format!("{}:%{pane}", gw.borrow().key),
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        match &mut self.backend {
            Backend::Daemon { stream, .. } => write_blocking(stream, T_INPUT, bytes),
            Backend::Tmux { gw, pane, .. } => gw.borrow_mut().send_keys(*pane, bytes),
        }
    }

    pub fn on_readable(&mut self) -> u32 {
        let mut sig = 0;
        match &mut self.backend {
            Backend::Daemon { stream, reader, .. } => {
                let mut buf = vec![0u8; 256 * 1024];
                let mut budget = 8 << 20;
                loop {
                    match stream.read(&mut buf) {
                        Ok(0) => {
                            self.exited = true;
                            break;
                        }
                        Ok(n) => {
                            reader.push(&buf[..n]);
                            budget -= n.min(budget);
                            if budget == 0 {
                                break;
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(_) => {
                            self.exited = true;
                            break;
                        }
                    }
                }
                let mut frames = Vec::new();
                while let Some(f) = reader.next() {
                    frames.push(f);
                }
                for (tag, payload) in frames {
                    match tag {
                        T_ATTACHED => {
                            if let (Ok(new_id), Backend::Daemon { id, .. }) =
                                (serde_json::from_slice::<String>(&payload), &mut self.backend)
                            {
                                *id = new_id;
                                self.events.push(json!({"type": "attached", "id": id.clone()}));
                            }
                        }
                        T_OUTPUT => {
                            if let Some(f) = self.log.as_mut() {
                                let _ = f.write_all(&payload);
                            }
                            self.process_output(&payload);
                            self.run_triggers();
                            sig |= SIG_DIRTY;
                        }
                        T_EXITED => self.exited = true,
                        T_TAKEN => {
                            // Moved to another window: not an exit of the shell.
                            self.events.push(json!({"type": "taken"}));
                            self.exited = true;
                            self.taken = true;
                        }
                        T_ERROR => {
                            let msg: String = serde_json::from_slice(&payload).unwrap_or_default();
                            self.term.vt_write(format!("\r\n[tessera: {msg}]\r\n").as_bytes());
                            self.exited = true;
                        }
                        _ => {}
                    }
                }
            }
            Backend::Tmux { shared, .. } => {
                let (data, grid, closed) = {
                    let mut sh = shared.borrow_mut();
                    sh.drain_wake();
                    let data = std::mem::take(&mut sh.pending);
                    let grid = if std::mem::take(&mut sh.grid_changed) { Some(sh.grid) } else { None };
                    (data, grid, sh.closed)
                };
                if let Some((c, r)) = grid {
                    let (_, _, w, h) = self.size.get();
                    let _ = self.term.resize(c, r, w, h);
                    self.size.set((c, r, w, h));
                    self.events.push(json!({"type": "grid", "cols": c, "rows": r}));
                    sig |= SIG_DIRTY;
                }
                if !data.is_empty() {
                    if let Some(f) = self.log.as_mut() {
                        let _ = f.write_all(&data);
                    }
                    self.term.vt_write(&data);
                    self.run_triggers();
                    sig |= SIG_DIRTY;
                }
                if closed {
                    self.exited = true;
                }
            }
        }
        // Replies to queries (DA, DSR, ...) go back to the program — except
        // in tmux panes, where tmux itself answers its programs.
        let out = std::mem::take(&mut *self.outbox.borrow_mut());
        if !out.is_empty() && self.gateway.is_none() && matches!(self.backend, Backend::Daemon { .. }) {
            self.write_input(&out);
        }
        if self.title_changed.replace(false) && self.opts.allow_title {
            self.events.push(json!({"type": "title", "title": self.title()}));
        }
        if let Some(text) = self.clipboard.borrow_mut().take() {
            if self.opts.clipboard_access {
                self.events.push(json!({"type": "clipboard", "text": text}));
            }
        }
        if self.pwd_changed.replace(false) {
            self.events.push(json!({"type": "pwd", "pwd": self.pwd()}));
        }
        if self.bell.replace(false) {
            self.events.push(json!({"type": "bell"}));
        }
        if let Some(gw) = &self.gateway {
            self.events.append(&mut gw.borrow_mut().events);
        }
        if self.exited {
            sig |= SIG_EXITED;
        }
        if !self.events.is_empty() {
            sig |= SIG_EVENTS;
        }
        sig
    }

    fn process_output(&mut self, data: &[u8]) {
        if let Some(gw) = self.gateway.clone() {
            let rest = gw.borrow_mut().feed(data);
            self.events.append(&mut gw.borrow_mut().events);
            if let Some(rest) = rest {
                tmux::unregister(&gw.borrow().key);
                self.gateway = None;
                self.term.vt_write(b"\r\n[tmux detached]\r\n");
                self.term.vt_write(&rest);
            }
            return;
        }
        let mut buf = std::mem::take(&mut self.carry);
        buf.extend_from_slice(data);
        if let Some(i) = buf.windows(tmux::DCS_START.len()).position(|w| w == tmux::DCS_START) {
            self.term.vt_write(&buf[..i]);
            let rest = buf[i + tmux::DCS_START.len()..].to_vec();
            self.start_gateway();
            if !rest.is_empty() {
                self.process_output(&rest);
            }
            return;
        }
        // Hold back a trailing partial DCS start so a split read still matches.
        let keep = (1..tmux::DCS_START.len())
            .rev()
            .find(|&k| buf.len() >= k && buf[buf.len() - k..] == tmux::DCS_START[..k])
            .unwrap_or(0);
        let cut = buf.len() - keep;
        self.term.vt_write(&buf[..cut]);
        self.carry = buf[cut..].to_vec();
    }

    fn start_gateway(&mut self) {
        let Backend::Daemon { stream, .. } = &self.backend else { return };
        let Ok(out) = stream.try_clone() else { return };
        let (c, r, _, _) = self.size.get();
        self.term.vt_write(
            b"\r\n\x1b[2m[tmux control mode: tmux windows open as tabs. Press Esc here to detach.]\x1b[0m\r\n",
        );
        self.gateway = Some(tmux::start(out, c, r));
    }

    pub fn take_events(&mut self) -> String {
        let ev = std::mem::take(&mut self.events);
        serde_json::to_string(&ev).unwrap_or_else(|_| "[]".into())
    }

    pub fn resize(&mut self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) {
        let (cols, rows) = (cols.max(2), rows.max(1));
        if self.size.get() == (cols, rows, cell_w, cell_h) {
            return;
        }
        if matches!(self.backend, Backend::Tmux { .. }) {
            // tmux owns pane sizes; only the cell size changes here.
            let (c, r, _, _) = self.size.get();
            self.size.set((c, r, cell_w, cell_h));
            let _ = self.term.resize(c, r, cell_w, cell_h);
            return;
        }
        self.size.set((cols, rows, cell_w, cell_h));
        let _ = self.term.resize(cols, rows, cell_w.max(1), cell_h.max(1));
        if let Some(gw) = &self.gateway {
            gw.borrow_mut().set_client_size(cols, rows);
        }
        if let Backend::Daemon { stream, .. } = &mut self.backend {
            let payload = serde_json::to_vec(&Resize { cols, rows }).unwrap();
            write_blocking(stream, T_RESIZE, &payload);
        }
    }

    pub fn key_event(&mut self, qt_key: i32, qt_mods: u32, text: &str, release: bool, _native_vk: u32) -> bool {
        if let Some(gw) = &self.gateway {
            if !release && qt_key == 0x0100_0000 {
                gw.borrow_mut().detach();
            }
            return true;
        }
        let printable = !text.is_empty() && text.chars().all(|c| !c.is_control());
        let Some((k, ucp)) = keymap::key(qt_key, qt_mods) else {
            if printable && !release {
                self.write_input(text.as_bytes());
                return true;
            }
            return false;
        };
        let mods = keymap::mods(qt_mods);
        let mut consumed = key::Mods::empty();
        if printable && mods.contains(key::Mods::SHIFT) {
            consumed |= key::Mods::SHIFT;
        }
        self.key_ev
            .set_action(if release { key::Action::Release } else { key::Action::Press })
            .set_key(k)
            .set_mods(mods)
            .set_consumed_mods(consumed)
            .set_unshifted_codepoint(ucp)
            .set_utf8(if printable && !release { Some(text) } else { None });
        let mut out = Vec::new();
        let alt_esc = self.opts.alt_mode != "normal";
        let _ = self.key_enc
            .set_options_from_terminal(&self.term)
            .set_alt_esc_prefix(alt_esc)
            .encode_to_vec(&self.key_ev, &mut out);
        if out.is_empty() && printable && !release {
            out.extend_from_slice(text.as_bytes());
        }
        if !out.is_empty() {
            self.scroll_to_bottom();
            self.write_input(&out);
        }
        true
    }

    pub fn paste(&mut self, text: &str) {
        let bracketed = self.opts.bracketed_paste && self.term.mode(Mode::BRACKETED_PASTE).unwrap_or(false);
        let mut data = text.replace("\r\n", "\r").replace('\n', "\r").into_bytes();
        let mut out = vec![0u8; data.len() + 16];
        let n = libghostty_vt::paste::encode(&mut data, bracketed, &mut out).unwrap_or(0);
        out.truncate(n);
        self.scroll_to_bottom();
        self.write_input(&out);
    }

    pub fn send_text(&mut self, text: &str) {
        self.write_input(text.as_bytes());
    }

    pub fn scroll_lines(&mut self, delta: i32) {
        self.term.scroll_viewport(ScrollViewport::Delta(delta as isize));
    }

    pub fn scroll_to_row(&mut self, row: u64) {
        self.term.scroll_viewport(ScrollViewport::Row(row as usize));
    }

    pub fn scrolls_locally(&self, qt_mods: u32) -> bool {
        !self.wants_mouse(qt_mods) && !matches!(self.term.active_screen(), Ok(Screen::Alternate))
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_viewport(ScrollViewport::Bottom);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn mouse_event(
        &mut self,
        action: u8,
        button: u8,
        x: f32,
        y: f32,
        qt_mods: u32,
        width: u32,
        height: u32,
        cell_w: u32,
        cell_h: u32,
        pad: u32,
    ) -> bool {
        let mods = keymap::mods(qt_mods);
        let tracking = self.opts.mouse_reporting && self.term.is_mouse_tracking().unwrap_or(false);
        let report = tracking && !mods.contains(key::Mods::SHIFT);
        match action {
            0 if button > 0 && button < 4 => self.buttons |= 1 << button,
            1 if button > 0 && button < 4 => self.buttons &= !(1 << button),
            _ => {}
        }

        // Wheel without mouse reporting: scrollback, or arrow keys in
        // full-screen programs (alternate scroll, like iTerm2).
        if (button == 4 || button == 5) && !report {
            let alt = matches!(self.term.active_screen(), Ok(Screen::Alternate));
            if alt && !self.opts.alternate_scroll {
                return false;
            }
            if alt {
                let seq: &[u8] = if button == 4 { b"\x1b[A" } else { b"\x1b[B" };
                let app = self.term.mode(Mode::DECCKM).unwrap_or(false);
                let seq = if app { if button == 4 { b"\x1bOA".as_slice() } else { b"\x1bOB" } } else { seq };
                self.write_input(&seq.repeat(3));
                return false;
            }
            self.scroll_lines(if button == 4 { -3 } else { 3 });
            return true;
        }

        if report {
            let enc_size = mouse::EncoderSize {
                screen_width: width,
                screen_height: height,
                cell_width: cell_w.max(1),
                cell_height: cell_h.max(1),
                padding_top: pad,
                padding_bottom: pad,
                padding_left: pad,
                padding_right: pad,
            };
            self.mouse_enc
                .set_options_from_terminal(&self.term)
                .set_size(enc_size)
                .set_any_button_pressed(self.buttons != 0)
                .set_track_last_cell(true);
            let btn = match button {
                1 => Some(mouse::Button::Left),
                2 => Some(mouse::Button::Right),
                3 => Some(mouse::Button::Middle),
                4 => Some(mouse::Button::Four),
                5 => Some(mouse::Button::Five),
                _ => [(2u8, mouse::Button::Left), (4, mouse::Button::Right), (8, mouse::Button::Middle)]
                    .into_iter()
                    .find(|(m, _)| self.buttons & m != 0)
                    .map(|(_, b)| b),
            };
            self.mouse_ev.set_mods(mods).set_position(mouse::Position { x, y });
            let mut out = Vec::new();
            let actions: &[mouse::Action] = match (action, button) {
                (0, 4 | 5) => &[mouse::Action::Press, mouse::Action::Release],
                (0, _) => &[mouse::Action::Press],
                (1, _) => &[mouse::Action::Release],
                _ => &[mouse::Action::Motion],
            };
            for &a in actions {
                self.mouse_ev.set_action(a).set_button(btn);
                let _ = self.mouse_enc.encode_to_vec(&self.mouse_ev, &mut out);
            }
            self.write_input(&out);
            return false;
        }

        // Selection.
        if button != 1 && !(action == 2 && self.selecting) {
            return false;
        }
        let (cols, rows) = (self.term.cols().unwrap_or(1), self.term.rows().unwrap_or(1));
        let cx = ((x - pad as f32).max(0.0) / cell_w.max(1) as f32).min(cols as f32 - 1.0) as u16;
        let cy = ((y - pad as f32).max(0.0) / cell_h.max(1) as f32).min(rows as f32 - 1.0) as u32;
        let point = Point::Viewport(PointCoordinate { x: cx, y: cy });
        let rect = mods.contains(key::Mods::ALT) && (mods.contains(key::Mods::CTRL) || mods.contains(key::Mods::SUPER));
        match action {
            0 => {
                let Ok(grid) = self.term.grid_ref(point) else { return false };
                let sel = self
                    .press
                    .set_repeat_distance(cell_w as f64)
                    .and_then(|p| p.set_time(self.t0.elapsed()))
                    .and_then(|p| p.set_position(x as f64, y as f64))
                    .and_then(|p| p.apply(&mut self.gesture, &self.term, grid));
                self.selecting = true;
                if let Ok(sel) = sel {
                    let _ = self.term.set_selection(sel.as_ref());
                    self.selected = sel.map(|s| format_selection(&self.term, &s)).unwrap_or_default();
                }
                true
            }
            2 => {
                let Ok(grid) = self.term.grid_ref(point) else { return false };
                let geometry = Geometry { columns: cols as u32, cell_width: cell_w.max(1), padding_left: pad, screen_height: height.max(1) };
                let sel = self
                    .drag
                    .set_rectangle(rect)
                    .and_then(|d| d.set_position(x as f64, y as f64))
                    .and_then(|d| d.apply(&mut self.gesture, &self.term, grid, geometry));
                if let Ok(sel) = sel {
                    let _ = self.term.set_selection(sel.as_ref());
                    self.selected = sel.as_ref().map(|s| format_selection(&self.term, s)).unwrap_or_default();
                    self.selected_html = sel.as_ref().map(|s| format_selection_as(&self.term, s, Format::Html, true)).unwrap_or_default();
                    self.selected_vt = sel.as_ref().map(|s| format_selection_as(&self.term, s, Format::Vt, true)).unwrap_or_default();
                }
                true
            }
            _ => {
                let grid = self.term.grid_ref(point).ok();
                let _ = self.release.apply(&mut self.gesture, &self.term, grid);
                self.selecting = false;
                true
            }
            // (Rich copies are made when the selection is finished: see
            // finish_selection, called by the view on release.)
        }
    }

    /// While a selection is dragged past the top or bottom, scroll a row
    /// and extend the selection. Returns false once there's nothing to do.
    #[allow(clippy::too_many_arguments)]
    pub fn autoscroll_tick(&mut self, x: f32, y: f32, width: u32, height: u32, cell_w: u32, cell_h: u32, pad: u32, rows_per_tick: u32) -> bool {
        if !self.selecting {
            return false;
        }
        let up = match self.gesture.autoscroll(&self.term) {
            Ok(Autoscroll::Up) => true,
            Ok(Autoscroll::Down) => false,
            _ => return false,
        };
        let n = rows_per_tick.clamp(1, 40) as isize;
        self.term.scroll_viewport(ScrollViewport::Delta(if up { -n } else { n }));
        let (cols, rows) = (self.term.cols().unwrap_or(1), self.term.rows().unwrap_or(1));
        let cx = ((x - pad as f32).max(0.0) / cell_w.max(1) as f32).min(cols as f32 - 1.0) as u16;
        let cy = if up { 0 } else { rows as u32 - 1 };
        let rect = false;
        let geometry = Geometry { columns: cols as u32, cell_width: cell_w.max(1), padding_left: pad, screen_height: height.max(1) };
        let _ = (width, cell_h);
        let sel = self
            .autotick
            .set_rectangle(rect)
            .and_then(|t| t.set_position(x as f64, y as f64))
            .and_then(|t| t.apply(&mut self.gesture, &self.term, PointCoordinate { x: cx, y: cy }, geometry));
        if let Ok(sel) = sel {
            let _ = self.term.set_selection(sel.as_ref());
            self.selected = sel.as_ref().map(|s| format_selection(&self.term, s)).unwrap_or_default();
            self.selected_html = sel.as_ref().map(|s| format_selection_as(&self.term, s, Format::Html, true)).unwrap_or_default();
            self.selected_vt = sel.as_ref().map(|s| format_selection_as(&self.term, s, Format::Vt, true)).unwrap_or_default();
        }
        true
    }

    pub fn selected_text(&self) -> String {
        if self.opts.copy_trailing_newline {
            self.selected.clone()
        } else {
            self.selected.trim_end_matches(['\n', '\r']).to_string()
        }
    }

    pub fn selected_html(&self) -> String {
        self.selected_html.clone()
    }

    pub fn selected_vt(&self) -> String {
        self.selected_vt.clone()
    }

    pub fn contents_text(&self) -> String {
        match self.term.select_all() {
            Ok(Some(sel)) => format_selection_as(&self.term, &sel, Format::Plain, false),
            _ => String::new(),
        }
    }

    pub fn set_options(&mut self, json: &str) {
        if let Ok(o) = serde_json::from_str::<Options2>(json) {
            if o.minimum_contrast != self.opts.minimum_contrast {
                self.contrast_cache.clear();
                self.force_full = true;
            }
            self.triggers = o
                .triggers
                .iter()
                .filter(|t| t.enabled && !t.regex.is_empty())
                .filter_map(|t| regex::Regex::new(&t.regex).ok().map(|r| (r, t.clone())))
                .collect();
            self.opts = o;
            let chars: Vec<char> = self.opts.word_chars.chars().collect();
            let _ = self.press.set_word_boundary_codepoints(&boundary_chars(&chars));
            let _ = self.drag.set_word_boundary_codepoints(&boundary_chars(&chars));
            use libghostty_vt::selection::gesture::{Behavior, Behaviors};
            let b = Behaviors::new()
                .with_single_click_behavior(Behavior::Cell)
                .with_double_click_behavior(Behavior::Word)
                .with_triple_click_behavior(Behavior::Line);
            let _ = self.press.set_behaviors(&b);
        }
    }

    pub fn show_message(&mut self, text: &str) {
        self.term.vt_write(text.replace('\n', "\r\n").as_bytes());
        self.force_full = true;
    }

    pub fn reset(&mut self) {
        self.highlights.clear();
        self.term.reset();
        self.force_full = true;
        if self.gateway.is_none() {
            self.write_input(b"\x0c");
        }
    }

    pub fn log_to_file(&mut self, path: &str) -> bool {
        if path.is_empty() {
            self.log = None;
            return true;
        }
        let path = match path.strip_prefix("~/") {
            Some(rest) => format!("{}/{rest}", std::env::var("HOME").unwrap_or_default()),
            None => path.to_string(),
        };
        if let Some(dir) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(f) => {
                self.log = Some(f);
                true
            }
            Err(_) => false,
        }
    }

    pub fn is_logging(&self) -> bool {
        self.log.is_some()
    }

    pub fn select_all(&mut self) {
        let (text, html, vt) = match self.term.select_all() {
            Ok(Some(sel)) => {
                let _ = self.term.set_selection(Some(&sel));
                (
                    format_selection(&self.term, &sel),
                    format_selection_as(&self.term, &sel, Format::Html, true),
                    format_selection_as(&self.term, &sel, Format::Vt, true),
                )
            }
            _ => Default::default(),
        };
        self.selected = text;
        self.selected_html = html;
        self.selected_vt = vt;
    }

    /// Like iTerm2's Clear Buffer: drop the scrollback and clear the screen
    /// (the shell redraws its prompt on Ctrl+L).
    pub fn clear_scrollback(&mut self) {
        self.term.vt_write(b"\x1b[H\x1b[2J\x1b[3J");
        if self.gateway.is_none() {
            self.write_input(b"\x0c");
        }
    }

    pub fn clear_selection(&mut self) {
        let _ = self.term.set_selection(None);
        self.selected.clear();
    }

    pub fn title(&self) -> String {
        self.term.title().map(str::to_string).unwrap_or_default()
    }

    pub fn pwd(&self) -> String {
        let p = self.term.pwd().map(str::to_string).unwrap_or_default();
        // OSC 7 gives file://host/path.
        match p.strip_prefix("file://") {
            Some(rest) => rest.find('/').map(|i| rest[i..].to_string()).unwrap_or_default(),
            None => p,
        }
    }

    pub fn focus(&mut self, focused: bool) {
        if self.term.mode(Mode::FOCUS_EVENT).unwrap_or(false) && self.gateway.is_none() {
            self.write_input(if focused { b"\x1b[I" } else { b"\x1b[O" });
        }
    }

    pub fn kill(&mut self) {
        match &mut self.backend {
            Backend::Daemon { stream, .. } => write_blocking(stream, T_KILL, b""),
            Backend::Tmux { gw, pane, .. } => gw.borrow_mut().user_command(&format!("kill-pane -t %{pane}")),
        }
    }

    /// Start or update a search. Returns the number of matches, or -1 when
    /// the regex is invalid. The current match is the newest one on or above
    /// the bottom of the view (iTerm2 searches upward).
    pub fn find(&mut self, query: &str, regex: bool) -> i32 {
        if query.is_empty() {
            self.find_clear();
            return 0;
        }
        let Ok(re) = crate::find::compile(query, regex) else { return -1 };
        let matches = crate::find::search(&crate::find::rows(&self.term), &re);
        let bottom = self.view_top() + self.term.rows().unwrap_or(1) as usize;
        let current = matches.iter().rposition(|m| m.row < bottom).or(if matches.is_empty() { None } else { Some(0) });
        let n = matches.len() as i32;
        self.find = Some(FindState { re, matches, current });
        self.reveal_current();
        self.force_full = true;
        n
    }

    /// Search again (new output), keeping the current match where it was.
    pub fn find_refresh(&mut self) -> i32 {
        let Some(f) = self.find.as_ref() else { return 0 };
        let keep = f.current.map(|i| f.matches[i]);
        let matches = crate::find::search(&crate::find::rows(&self.term), &f.re);
        let current = keep
            .and_then(|k| matches.iter().position(|m| *m == k))
            .or_else(|| if matches.is_empty() { None } else { Some(matches.len() - 1) });
        let n = matches.len() as i32;
        if let Some(f) = self.find.as_mut() {
            f.matches = matches;
            f.current = current;
        }
        self.force_full = true;
        n
    }

    /// Move to the previous (dir < 0, older, upward) or next match. Returns
    /// its index (0 = oldest), or -1 with no matches.
    pub fn find_step(&mut self, dir: i32) -> i32 {
        let Some(f) = self.find.as_mut() else { return -1 };
        if f.matches.is_empty() {
            return -1;
        }
        let n = f.matches.len();
        let i = match f.current {
            Some(i) => (i as i64 + dir as i64).rem_euclid(n as i64) as usize,
            None => n - 1,
        };
        f.current = Some(i);
        self.reveal_current();
        self.force_full = true;
        i as i32
    }

    pub fn find_index(&self) -> i32 {
        self.find.as_ref().and_then(|f| f.current).map(|i| i as i32).unwrap_or(-1)
    }

    pub fn find_clear(&mut self) {
        if self.find.take().is_some() {
            self.force_full = true;
        }
    }

    /// Row of the top of the view, counted from the top of the scrollback.
    fn view_top(&self) -> usize {
        crate::find::screen_row(&self.term, Point::Viewport(PointCoordinate { x: 0, y: 0 })).unwrap_or(0)
    }

    /// Scroll so the current match is in view (centered when it wasn't).
    fn reveal_current(&mut self) {
        let Some(m) = self.find.as_ref().and_then(|f| f.current.map(|i| f.matches[i])) else { return };
        let rows = self.term.rows().unwrap_or(1) as usize;
        let top = self.view_top();
        if m.row < top || m.row >= top + rows {
            self.term.scroll_viewport(ScrollViewport::Row(m.row.saturating_sub(rows / 2)));
        }
    }

    /// Text of one screen row (0 = top of the scrollback), and each
    /// character's cell column.
    fn row_text(&self, row: usize) -> (String, Vec<u16>) {
        let cols = self.term.cols().unwrap_or(0);
        let mut text = String::new();
        let mut at = Vec::new();
        let mut buf = ['\0'; 16];
        for x in 0..cols {
            let Ok(r) = self.term.grid_ref(Point::Screen(PointCoordinate { x, y: row as u32 })) else { break };
            if let Ok(c) = r.cell() {
                if matches!(c.wide(), Ok(CellWide::SpacerTail | CellWide::SpacerHead)) {
                    continue;
                }
            }
            let n = r.graphemes(&mut buf).unwrap_or(0);
            if n == 0 {
                text.push(' ');
                at.push(x);
            } else {
                for c in &buf[..n] {
                    text.push(*c);
                    at.push(x);
                }
            }
        }
        (text, at)
    }

    fn total_rows(&self) -> usize {
        self.term.total_rows().unwrap_or(0)
    }

    /// What Ctrl+click at this spot opens: an OSC 8 hyperlink, a URL in the
    /// text, or an existing file named there (file:// URL). Empty if nothing.
    #[allow(clippy::too_many_arguments)]
    pub fn link_at(&self, x: f32, y: f32, cell_w: u32, cell_h: u32, pad: u32) -> String {
        let (cols, rows) = (self.term.cols().unwrap_or(1), self.term.rows().unwrap_or(1));
        let cx = ((x - pad as f32).max(0.0) / cell_w.max(1) as f32).min(cols as f32 - 1.0) as u16;
        let cy = ((y - pad as f32).max(0.0) / cell_h.max(1) as f32).min(rows as f32 - 1.0) as u32;
        if let Ok(r) = self.term.grid_ref(Point::Viewport(PointCoordinate { x: cx, y: cy })) {
            let mut buf = vec![0u8; 2048];
            if let Ok(n) = r.hyperlink_uri(&mut buf) {
                if n > 0 {
                    return String::from_utf8_lossy(&buf[..n]).into_owned();
                }
            }
        }
        let row = self.view_top() + cy as usize;
        let (text, at) = self.row_text(row);
        let Some(i) = at.iter().position(|&c| c >= cx) else { return String::new() };
        let byte = text.char_indices().nth(i).map(|(b, _)| b).unwrap_or(0);
        let url = regex::Regex::new(r#"(?:https?|ftp|file)://[^\s"'<>()\[\]]+|www\.[^\s"'<>()\[\]]+"#).unwrap();
        for m in url.find_iter(&text) {
            if m.start() <= byte && byte < m.end() {
                let u = m.as_str().trim_end_matches(['.', ',', ';', ':', '!', '?']);
                return if u.starts_with("www.") { format!("https://{u}") } else { u.to_string() };
            }
        }
        // A path: the word under the pointer, if it names an existing file.
        let is_path = |c: char| c.is_alphanumeric() || "/._-~+@%=:".contains(c);
        let chars: Vec<char> = text.chars().collect();
        if i >= chars.len() || !is_path(chars[i]) {
            return String::new();
        }
        let (mut a, mut b) = (i, i);
        while a > 0 && is_path(chars[a - 1]) {
            a -= 1;
        }
        while b + 1 < chars.len() && is_path(chars[b + 1]) {
            b += 1;
        }
        let word: String = chars[a..=b].iter().collect();
        let word = word.trim_end_matches(['.', ',', ';', ':']);
        // file:line:col (compilers, grep -n): open the file.
        let plain = word.split(':').next().unwrap_or(word);
        let home = std::env::var("HOME").unwrap_or_default();
        let path = if let Some(rest) = plain.strip_prefix("~/") {
            std::path::PathBuf::from(&home).join(rest)
        } else if plain.starts_with('/') {
            std::path::PathBuf::from(plain)
        } else {
            let base = self.pwd();
            if base.is_empty() { return String::new() }
            std::path::PathBuf::from(base).join(plain)
        };
        if !plain.is_empty() && path.exists() {
            return format!("file://{}", path.display());
        }
        String::new()
    }

    /// Screen rows where a shell prompt starts (shell integration, OSC 133).
    fn prompt_rows(&self) -> Vec<usize> {
        let total = self.total_rows();
        (0..total)
            .filter(|&y| {
                self.term
                    .grid_ref(Point::Screen(PointCoordinate { x: 0, y: y as u32 }))
                    .ok()
                    .and_then(|r| r.row().ok())
                    .and_then(|r| r.semantic_prompt().ok())
                    == Some(RowSemanticPrompt::Prompt)
            })
            .collect()
    }

    /// Scroll to the previous (dir < 0) or next shell prompt. False if none.
    pub fn jump_to_prompt(&mut self, dir: i32) -> bool {
        let top = self.view_top();
        let rows = self.prompt_rows();
        let target = if dir < 0 { rows.iter().rev().find(|&&r| r < top).copied() } else { rows.iter().find(|&&r| r > top).copied() };
        match target {
            Some(r) => {
                self.term.scroll_viewport(ScrollViewport::Row(r));
                self.force_full = true;
                true
            }
            None => {
                if dir > 0 {
                    self.term.scroll_viewport(ScrollViewport::Bottom);
                }
                false
            }
        }
    }

    /// Select the output of the last finished command (between the last two
    /// prompts, without the command line). Returns the text.
    pub fn select_last_output(&mut self) -> String {
        let prompts = self.prompt_rows();
        if prompts.len() < 2 {
            return String::new();
        }
        let (p1, p2) = (prompts[prompts.len() - 2], prompts[prompts.len() - 1]);
        // The command line itself: rows from p1 that hold input or prompt.
        let mut start = p1 + 1;
        while start < p2 {
            let out = self
                .term
                .grid_ref(Point::Screen(PointCoordinate { x: 0, y: start as u32 }))
                .ok()
                .and_then(|r| r.row().ok())
                .and_then(|r| r.semantic_prompt().ok());
            if out != Some(RowSemanticPrompt::Continuation) {
                break;
            }
            start += 1;
        }
        let mut end = p2.saturating_sub(1);
        while end > start && self.row_text(end).0.trim().is_empty() {
            end -= 1;
        }
        if start > end || start >= p2 {
            return String::new();
        }
        let cols = self.term.cols().unwrap_or(1);
        let (Ok(a), Ok(b)) = (
            self.term.grid_ref(Point::Screen(PointCoordinate { x: 0, y: start as u32 })),
            self.term.grid_ref(Point::Screen(PointCoordinate { x: cols - 1, y: end as u32 })),
        ) else {
            return String::new();
        };
        let sel = Selection::new(a, b, false);
        let _ = self.term.set_selection(Some(&sel));
        self.selected = format_selection(&self.term, &sel);
        self.selected_html = format_selection_as(&self.term, &sel, Format::Html, true);
        self.selected_vt = format_selection_as(&self.term, &sel, Format::Vt, true);
        self.force_full = true;
        self.selected.clone()
    }

    /// Check finished lines of new output against the profile's triggers.
    fn run_triggers(&mut self) {
        if self.triggers.is_empty() {
            return;
        }
        let total = self.total_rows();
        let rows = self.term.rows().unwrap_or(1) as usize;
        let cursor = (total.saturating_sub(rows)) + self.term.cursor_y().unwrap_or(0) as usize;
        if self.trigger_row > total {
            self.trigger_row = 0;
        }
        let from = self.trigger_row.max(cursor.saturating_sub(500));
        let mut sends = Vec::new();
        for row in from..cursor {
            let (text, at) = self.row_text(row);
            for (re, t) in &self.triggers {
                for m in re.find_iter(&text) {
                    match t.action.as_str() {
                        "highlight" => {
                            let i0 = text[..m.start()].chars().count();
                            let i1 = text[..m.end()].chars().count();
                            let start = at.get(i0).copied().unwrap_or(0);
                            let end = at.get(i1.saturating_sub(1)).copied().map(|c| c + 1).unwrap_or(start + 1);
                            let color = crate::settings::hex_rgb(&t.param).map(rgb).unwrap_or(0xd9b43b);
                            self.highlights.push((row, start, end, color));
                        }
                        "notify" => self.events.push(json!({"type": "notify", "title": if t.param.is_empty() { "Trigger" } else { t.param.as_str() }, "body": m.as_str()})),
                        "bell" => self.bell.set(true),
                        "send" => sends.push(t.param.replace("\\n", "\n").replace("\\r", "\r")),
                        _ => {}
                    }
                }
            }
        }
        self.trigger_row = cursor;
        if self.highlights.len() > 4000 {
            let cut = self.highlights.len() - 4000;
            self.highlights.drain(..cut);
        }
        self.highlights.sort_by_key(|h| h.0);
        for s in sends {
            self.write_input(s.as_bytes());
        }
        if !self.highlights.is_empty() {
            self.force_full = true;
        }
    }

    pub fn set_theme(&mut self, theme_json: &str) {
        let Ok(th) = serde_json::from_str::<crate::settings::Theme>(theme_json) else { return };
        use crate::settings::hex_rgb;
        let _ = self.term.set_default_fg_color(hex_rgb(&th.foreground));
        let _ = self.term.set_default_bg_color(hex_rgb(&th.background));
        let _ = self.term.set_default_cursor_color(hex_rgb(&th.cursor));
        if let Ok(mut pal) = self.term.default_color_palette() {
            for (i, c) in th.ansi.iter().take(16).enumerate() {
                if let Some(c) = hex_rgb(c) {
                    pal.0[i] = c;
                }
            }
            let _ = self.term.set_default_color_palette(Some(pal));
        }
        self.contrast_cache.clear();
        self.sel_bg = hex_rgb(&th.selection_background).map(rgb);
        self.sel_fg = hex_rgb(&th.selection_foreground).map(rgb);
        self.force_full = true;
    }

    pub fn set_cursor_style(&mut self, style: u8) {
        use libghostty_vt::terminal::CursorStyle;
        let st = match style {
            1 => CursorStyle::Bar,
            2 => CursorStyle::Underline,
            _ => CursorStyle::Block,
        };
        let _ = self.term.set_default_cursor_style(Some(st));
        // Apply it now too (as a DECSCUSR reset would), unless a program set its own.
        self.term.vt_write(b"\x1b[0 q");
        self.force_full = true;
    }

    pub fn wants_mouse(&self, qt_mods: u32) -> bool {
        self.opts.mouse_reporting
            && self.term.is_mouse_tracking().unwrap_or(false)
            && !keymap::mods(qt_mods).contains(key::Mods::SHIFT)
    }

    pub fn tmux_pane_id(&self) -> i32 {
        match &self.backend {
            Backend::Tmux { pane, .. } => *pane as i32,
            _ => -1,
        }
    }

    pub fn tmux_gateway(&self) -> String {
        match (&self.backend, &self.gateway) {
            (Backend::Tmux { gw, .. }, _) | (_, Some(gw)) => gw.borrow().key.clone(),
            _ => String::new(),
        }
    }

    pub fn tmux_command(&mut self, command: &str) {
        let gw = match (&self.backend, &self.gateway) {
            (Backend::Tmux { gw, .. }, _) | (_, Some(gw)) => gw.clone(),
            _ => return,
        };
        gw.borrow_mut().user_command(command);
    }

    pub fn tmux_set_grid(&mut self, _cols: u16, _rows: u16, cell_w: u32, cell_h: u32) {
        let (c, r, _, _) = self.size.get();
        self.size.set((c, r, cell_w, cell_h));
        let _ = self.term.resize(c, r, cell_w.max(1), cell_h.max(1));
    }

    /// Fill `out` with the rows that changed. Returns false when nothing did.
    pub fn frame(&mut self, out: &mut Frame, force_full: bool) -> bool {
        out.dirty_rows.clear();
        out.runs.clear();
        let view_top = if self.find.is_some() || !self.highlights.is_empty() { self.view_top() } else { 0 };
        out.mark_rows.clear();
        let Ok(snap) = self.rs.update(&self.term) else { return false };
        let dirty = snap.dirty().unwrap_or(Dirty::Full);
        let cols = snap.cols().unwrap_or(0);
        let rows = snap.rows().unwrap_or(0);
        let full = force_full || std::mem::take(&mut self.force_full) || dirty == Dirty::Full || self.last_full_size != (cols, rows);
        let colors = match snap.colors() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let cursor = snap.cursor_viewport().ok().flatten();
        let cursor_visible = snap.cursor_visible().unwrap_or(false) && cursor.is_some();
        let cursor_row = cursor.filter(|_| cursor_visible).map(|c| c.y);
        let cursor_moved = cursor_row != self.last_cursor_row;
        if !full && dirty == Dirty::Clean && !cursor_moved {
            return false;
        }
        let old_cursor_row = self.last_cursor_row;
        self.last_cursor_row = cursor_row;
        self.last_full_size = (cols, rows);

        out.cols = cols;
        out.rows = rows;
        out.default_fg = rgb(colors.foreground);
        out.default_bg = rgb(colors.background);
        out.cursor_visible = cursor_visible;
        out.cursor_x = cursor.map(|c| c.x as i32).unwrap_or(-1);
        out.cursor_y = cursor.map(|c| c.y as i32).unwrap_or(-1);
        out.cursor_style = match snap.cursor_visual_style() {
            Ok(CursorVisualStyle::Bar) => 1,
            Ok(CursorVisualStyle::Underline) => 2,
            Ok(CursorVisualStyle::BlockHollow) => 3,
            _ => 0,
        };
        out.cursor_color = rgb(colors.cursor.unwrap_or(colors.foreground));
        out.cursor_text.clear();
        out.cursor_wide = false;

        let Ok(mut row_it) = self.rows_it.update(&snap) else { return false };
        let mut y: u16 = 0;
        let mut text = String::new();
        while let Some(row) = row_it.next() {
            let want = full
                || row.dirty().unwrap_or(true)
                || Some(y) == cursor_row
                || Some(y) == old_cursor_row;
            if want {
                out.dirty_rows.push(y);
                if row.raw_row().ok().and_then(|r| r.semantic_prompt().ok()) == Some(RowSemanticPrompt::Prompt) {
                    out.mark_rows.push(y);
                }
                let abs_row = view_top + y as usize;
                let row_highlights: Vec<(u16, u16, u32)> = if self.highlights.is_empty() {
                    Vec::new()
                } else {
                    let first = self.highlights.partition_point(|h| h.0 < abs_row);
                    self.highlights[first..].iter().take_while(|h| h.0 == abs_row).map(|h| (h.1, h.2, h.3)).collect()
                };
                // Find matches on this row: (start, end, is_current).
                let row_matches: Vec<(u16, u16, bool)> = match self.find.as_ref() {
                    Some(f) => {
                        let abs = view_top + y as usize;
                        let first = f.matches.partition_point(|m| m.row < abs);
                        f.matches[first..]
                            .iter()
                            .enumerate()
                            .take_while(|(_, m)| m.row == abs)
                            .map(|(k, m)| (m.start, m.end, f.current == Some(first + k)))
                            .collect()
                    }
                    None => Vec::new(),
                };
                if let Ok(mut cell_it) = self.cells_it.update(row) {
                    let mut x: u16 = 0;
                    let mut cur: Option<Run> = None;
                    while let Some(cell) = cell_it.next() {
                        let raw = cell.raw_cell().ok();
                        let wide = raw.and_then(|c| c.wide().ok()).unwrap_or(CellWide::Narrow);
                        if matches!(wide, CellWide::SpacerTail | CellWide::SpacerHead) {
                            x += 1;
                            continue;
                        }
                        let width: u16 = if wide == CellWide::Wide { 2 } else { 1 };
                        text.clear();
                        if cell.graphemes_len().unwrap_or(0) > 0 {
                            let _ = cell.graphemes_utf8(&mut text);
                        }
                        if text.is_empty() {
                            text.push(' ');
                        }
                        let mut fg = cell.fg_color().ok().flatten().map(rgb).unwrap_or(out.default_fg);
                        let bg_set = cell.bg_color().ok().flatten();
                        let mut bg = bg_set.map(rgb).unwrap_or(out.default_bg);
                        let mut has_bg = bg_set.is_some();
                        let mut flags = 0;
                        if cell.has_styling().unwrap_or(false) {
                            if let Ok(st) = cell.style() {
                                if st.inverse {
                                    std::mem::swap(&mut fg, &mut bg);
                                    has_bg = true;
                                }
                                if st.bold {
                                    flags |= FLAG_BOLD;
                                }
                                if st.italic {
                                    flags |= FLAG_ITALIC;
                                }
                                if st.faint {
                                    flags |= FLAG_FAINT;
                                }
                                if st.strikethrough {
                                    flags |= FLAG_STRIKE;
                                }
                                if st.invisible {
                                    flags |= FLAG_INVISIBLE;
                                }
                                if !matches!(st.underline, libghostty_vt::style::Underline::None) {
                                    flags |= FLAG_UNDERLINE;
                                }
                            }
                        }
                        if cell.is_selected().unwrap_or(false) {
                            match self.sel_bg {
                                Some(sb) => {
                                    bg = sb;
                                    if let Some(sf) = self.sel_fg {
                                        fg = sf;
                                    }
                                }
                                None => std::mem::swap(&mut fg, &mut bg),
                            }
                            has_bg = true;
                        }
                        let target = crate::contrast::target(self.opts.minimum_contrast);
                        if target > 1.0 && flags & FLAG_INVISIBLE == 0 {
                            fg = *self
                                .contrast_cache
                                .entry((fg, bg))
                                .or_insert_with(|| crate::contrast::ensure(fg, bg, target));
                        }
                        if let Some(&(_, _, color)) = row_highlights.iter().find(|(a, b, _)| *a <= x && x < *b) {
                            bg = color;
                            fg = if crate::contrast::luminance(color) > 0.4 { 0x1a1a1a } else { 0xffffff };
                            has_bg = true;
                        }
                        if let Some(&(_, _, cur)) = row_matches.iter().find(|(a, b, _)| *a <= x && x < *b) {
                            bg = if cur { MATCH_CURRENT_BG } else { MATCH_BG };
                            fg = MATCH_FG;
                            has_bg = true;
                        }
                        if has_bg {
                            flags |= FLAG_HAS_BG;
                        }
                        if Some(y) == cursor_row && cursor.map(|c| c.x) == Some(x) {
                            out.cursor_text = text.clone();
                            out.cursor_wide = width == 2;
                        }
                        // Plain ASCII cells join a run; anything else (wide,
                        // symbols that may fall back to another font) gets
                        // its own so it lands exactly on its cell.
                        let joinable = width == 1 && text.len() == 1 && text.as_bytes()[0].is_ascii();
                        match cur.as_mut() {
                            Some(r) if joinable && r.fg == fg && r.bg == bg && r.flags == flags && r.col + r.width == x => {
                                r.text.push_str(&text);
                                r.width += 1;
                            }
                            _ => {
                                if let Some(r) = cur.take() {
                                    out.runs.push(r);
                                }
                                let r = Run { row: y, col: x, width, text: text.clone(), fg, bg, flags };
                                if joinable {
                                    cur = Some(r);
                                } else {
                                    out.runs.push(r);
                                }
                            }
                        }
                        x += 1; // the spacer cell after a wide character is its own (skipped) cell
                    }
                    if let Some(r) = cur.take() {
                        out.runs.push(r);
                    }
                }
                let _ = row.set_dirty(false);
            }
            y += 1;
        }
        let _ = snap.set_dirty(Dirty::Clean);
        if let Ok(sb) = self.term.scrollbar() {
            out.scroll_total = sb.total;
            out.scroll_offset = sb.offset;
            out.scroll_len = sb.len;
        }
        true
    }
}

/// Word boundaries for double-click: the usual separators, minus the
/// characters the user counts as part of words.
fn boundary_chars(word_chars: &[char]) -> Vec<char> {
    " \t'\"`()[]{}<>|;:,.=!?*&^%$#@~/\\+-_"
        .chars()
        .filter(|c| !word_chars.contains(c))
        .collect()
}

fn format_selection_as(term: &Terminal<'static, 'static>, sel: &Selection<'_>, fmt: Format, style: bool) -> String {
    let opts = FormatterOptions::new().with_format(fmt).with_selection(sel).with_trim(true).with_unwrap(true).with_style(style);
    let Ok(mut f) = Formatter::new(term, opts) else { return String::new() };
    f.format_alloc(None).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

fn format_selection(term: &Terminal<'static, 'static>, sel: &Selection<'_>) -> String {
    let opts = FormatterOptions::new().with_format(Format::Plain).with_selection(sel).with_trim(true).with_unwrap(true);
    let Ok(mut f) = Formatter::new(term, opts) else { return String::new() };
    f.format_alloc(None).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

impl Drop for Session {
    fn drop(&mut self) {
        // A daemon session just detaches (the shell keeps running); a gateway
        // going away ends its tmux tabs.
        if let Some(gw) = self.gateway.take() {
            tmux::unregister(&gw.borrow().key);
        }
    }
}

#[cfg(test)]
mod scrollback_tests {
    use libghostty_vt::{terminal::Options, Terminal};

    #[test]
    fn keeps_at_least_the_configured_lines() {
        for (cols, lines) in [(80u16, 1000u32), (200, 5000), (300, 2000)] {
            let mut t = Terminal::new(Options { cols, rows: 24, max_scrollback: super::scrollback_bytes(lines) }).unwrap();
            for i in 0..(lines as usize * 3) {
                t.vt_write(format!("line {i}\r\n").as_bytes());
            }
            assert!(t.scrollback_rows().unwrap() >= lines as usize, "{cols} cols: {} < {lines}", t.scrollback_rows().unwrap());
        }
    }
}
