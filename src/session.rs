//! One terminal: a libghostty-vt `Terminal` plus where its bytes come from
//! (a PTY in tesserad, here or on an SSH host).
//!
//! Everything here runs on the Qt GUI thread; the C++ view watches `fd()`
//! with a QSocketNotifier and calls `on_readable`, then `frame` to paint.

use crate::bridge::ffi::{Frame, Run};
use crate::keymap;
use crate::proto::*;
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
use crate::net::UnixStream;
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

pub use crate::snapshot::scrollback_bytes;

enum Backend {
    /// A shell in tesserad, this computer's ("" host) or another's over SSH.
    Daemon { stream: UnixStream, reader: FrameReader, id: String, host: String },
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
    /// Shell integration's prompt marks (OSC 133): the last one seen
    /// ('A' prompt, 'B' input, 'C' command running, 'D' done), and the tail
    /// of the last read in case a mark is split across reads.
    mark: u8,
    mark_tail: Vec<u8>,
    /// When the running command started (mark C), for its duration.
    command_started: Option<std::time::Instant>,
    /// A TesseraAliases report being read (it may span reads).
    alias_buf: Option<Vec<u8>>,
    /// An iTerm2 inline image (OSC 1337 File=) being read.
    file_buf: Option<Vec<u8>>,
    /// A command's output that began as an image file, until it ends.
    raw_image: Option<Vec<u8>>,
    /// The command just started: its first output bytes are checked.
    output_starts: bool,
    /// The running command's output (for a rich view), and the last
    /// finished one's: (kind, text).
    cmd_out: Option<Vec<u8>>,
    /// Outputs of commands still running under a nested one (a shell started
    /// in the shell: its commands are cells of their own).
    cmd_outer: Vec<Vec<u8>>,
    last_rich: (String, String),
    /// Notebook cells (one per command), the next command's line (from the
    /// shell integration), and the picture the running command printed.
    cells: crate::cells::Cells,
    /// The session whose kept cells a restored screen brings back.
    cells_from: Option<String>,
    /// Kept cells being read (in the background, never on the window's
    /// thread): they come on this, and a byte on `wake` says so.
    cells_coming: Option<std::sync::mpsc::Receiver<String>>,
    wake: (UnixStream, UnixStream),
    pending_command: String,
    command_buf: Option<Vec<u8>>,
    /// Shell integration's reports of what each command runs with.
    execs: crate::cells::ExecScan,
    cell_image: Option<Vec<u8>>,
    exited: bool,
    last_cursor_row: Option<u16>,
    /// And its column: a cursor moving along its row (backspacing over
    /// blanks) changes nothing else, but must be drawn.
    last_cursor_x: Option<u16>,
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
    /// A recording of the output (asciicast), while one is made.
    rec: Option<crate::record::Recorder>,
    clipboard: Rc<RefCell<Option<String>>>,
    find: Option<FindState>,
    taken: bool,
    /// Minimum-contrast results per (fg, bg) pair.
    contrast_cache: std::collections::HashMap<(u32, u32), u32>,
    /// Each viewport row's kitty placeholder runs (kept for rows not re-read).
    ph_rows: Vec<Vec<crate::images::PhRun>>,
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
    cwd: &str,
    cols: u16,
    rows: u16,
    cell_w: u32,
    cell_h: u32,
    scrollback: u32,
    command: &str,
    term_name: &str,
    startup_input: &str,
    host: &str,
) -> Result<Box<Session>, String> {
    let (cols, rows) = (cols.max(2), rows.max(1));
    let backend = {
        let mut stream = crate::client::connect_to(host).map_err(|e| if host.is_empty() { format!("tesserad: {e}") } else { format!("{host}: {e}") })?;
        if attach_id.is_empty() {
            write_json(&mut stream, T_CREATE, &Create { cols, rows, cwd: cwd.to_string(), command: command.to_string(), term: term_name.to_string(), input: startup_input.to_string(), scrollback })
        } else {
            write_json(&mut stream, T_ATTACH, &Attach { id: attach_id.to_string(), cols, rows })
        }
        .map_err(|e| e.to_string())?;
        stream.set_nonblocking(true).map_err(|e| e.to_string())?;
        Backend::Daemon { stream, reader: FrameReader::default(), id: attach_id.to_string(), host: host.to_string() }
    };

    let mut term = Terminal::new(Options { cols, rows, max_scrollback: scrollback_bytes(if scrollback > 0 { scrollback } else { SCROLLBACK as u32 }) }).map_err(|e| e.to_string())?;
    crate::images::enable(&mut term);
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

    let s = Box::new(Session {
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
        mark: 0,
        mark_tail: Vec::new(),
        command_started: None,
        alias_buf: None,
        file_buf: None,
        raw_image: None,
        output_starts: false,
        cmd_out: None,
        cmd_outer: Vec::new(),
        last_rich: (String::new(), String::new()),
        cells: Default::default(),
        cells_from: None,
        cells_coming: None,
        wake: {
            let (a, b) = UnixStream::pair().map_err(|e| e.to_string())?;
            a.set_nonblocking(true).map_err(|e| e.to_string())?;
            (a, b)
        },
        pending_command: String::new(),
        command_buf: None,
        execs: crate::cells::ExecScan::default(),
        cell_image: None,
        exited: false,
        last_cursor_row: None,
        last_cursor_x: None,
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
        rec: None,
        clipboard,
        find: None,
        taken: false,
        contrast_cache: std::collections::HashMap::new(),
        ph_rows: Vec::new(),
        triggers: Vec::new(),
        trigger_row: 0,
        highlights: Vec::new(),
    });
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
    pub fn fd(&self) -> i64 {
        match &self.backend {
            Backend::Daemon { stream, .. } => crate::net::raw(stream),
        }
    }

    pub fn id(&self) -> String {
        match &self.backend {
            Backend::Daemon { id, .. } => id.clone(),
        }
    }

    fn write_input(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        match &mut self.backend {
            Backend::Daemon { stream, .. } => write_blocking(stream, T_INPUT, bytes),
        }
    }

    /// What the window watches besides fd(): kept cells have come.
    pub fn wake_fd(&self) -> i64 {
        crate::net::raw(&self.wake.0)
    }

    pub fn on_readable(&mut self) -> u32 {
        let mut sig = 0;
        {
            let mut b = [0u8; 16];
            while matches!((&self.wake.0).read(&mut b), Ok(n) if n > 0) {}
        }
        if let Some(text) = self.cells_coming.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.cells_coming = None;
            if self.cells.restore_text(&text) {
                self.events.push(json!({"type": "cell", "id": 0, "done": true}));
                sig |= SIG_EVENTS;
            }
        }
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
                            if let (Ok(new_id), Backend::Daemon { id, host, .. }) =
                                (serde_json::from_slice::<String>(&payload), &mut self.backend)
                            {
                                *id = new_id;
                                crate::client::set_session_host(id, host);
                                self.events.push(json!({"type": "attached", "id": id.clone()}));
                                // The notebook's cells from before (a restart, or a reboot),
                                // as tesserad kept them.
                                let (id, host) = (id.clone(), host.clone());
                                let from = self.cells_from.take().unwrap_or_else(|| id.clone());
                                // (Read in the background: on_readable takes them when
                                // they come. Cells started meanwhile have later ids,
                                // and restore_text keeps them.)
                                let (tx, rx) = std::sync::mpsc::channel();
                                self.cells_coming = Some(rx);
                                if let Ok(mut wake) = self.wake.1.try_clone() {
                                    std::thread::spawn(move || {
                                        let kept = crate::client::rpc_read(&host, "cells.load", json!({"session": id, "from": from}));
                                        let _ = tx.send(kept.ok().as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string());
                                        let _ = wake.write_all(&[1]);
                                    });
                                }
                            }
                        }
                        // (The next cell's id, from tesserad, which keeps them.)
                        T_CELL => {
                            if let Ok(n) = serde_json::from_slice::<u64>(&payload) {
                                self.cells.given = Some(n);
                            }
                        }
                        T_OUTPUT => {
                            if let Some(f) = self.log.as_mut() {
                                let _ = f.write_all(&payload);
                            }
                            if let Some(r) = self.rec.as_mut() {
                                r.output(&payload);
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
        }
        // Replies to queries (DA, DSR, ...) go back to the program.
        let out = std::mem::take(&mut *self.outbox.borrow_mut());
        if !out.is_empty() {
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
        if self.exited {
            sig |= SIG_EXITED;
        }
        if !self.events.is_empty() {
            sig |= SIG_EVENTS;
        }
        sig
    }

    /// Follow the prompt marks in `data`; an event when the state changes.
    fn scan_marks(&mut self, data: &[u8]) {
        const PAT: &[u8] = b"\x1b]133;";
        let mut hay = std::mem::take(&mut self.mark_tail);
        hay.extend_from_slice(data);
        let mut last = None;
        let mut i = 0;
        while let Some(p) = hay[i..].windows(PAT.len()).position(|w| w == PAT) {
            let at = i + p + PAT.len();
            if at >= hay.len() {
                break;
            }
            match hay[at] {
                b'C' => {
                    self.command_started = Some(std::time::Instant::now());
                    last = Some(b'C');
                }
                b'D' => {
                    // "D;<exit status>": the command finished.
                    let status: String = hay[at + 1..].iter().skip_while(|&&c| c == b';').take_while(|c| c.is_ascii_digit()).map(|&c| c as char).collect();
                    if let Some(t) = self.command_started.take() {
                        self.events.push(json!({"type": "command", "status": status.parse::<i64>().unwrap_or(0), "ms": t.elapsed().as_millis() as u64}));
                    }
                    last = Some(b'D');
                }
                b'A' | b'B' => last = Some(hay[at]),
                _ => {}
            }
            i = at;
        }
        let keep = hay.len().min(PAT.len());
        self.mark_tail = hay[hay.len() - keep..].to_vec();
        if let Some(m) = last {
            // A then B at once is "at the prompt, ready for input".
            if m != self.mark {
                self.mark = m;
                self.events.push(json!({"type": "prompt", "state": (m as char).to_string()}));
            }
        }
    }

    /// The shell is waiting for a command (shell integration says so).
    pub fn at_prompt(&self) -> bool {
        matches!(self.mark, b'A' | b'B' | b'D')
    }

    /// The shell integration's alias report (OSC 1337;TesseraAliases=<base64>).
    fn scan_aliases(&mut self, data: &[u8]) {
        const PAT: &[u8] = b"\x1b]1337;TesseraAliases=";
        let mut rest: &[u8] = data;
        loop {
            if let Some(buf) = self.alias_buf.as_mut() {
                match rest.iter().position(|&b| b == 7 || b == 0x1b) {
                    Some(end) => {
                        buf.extend_from_slice(&rest[..end]);
                        let b64 = self.alias_buf.take().unwrap();
                        use base64::Engine;
                        if let Ok(raw) = base64::engine::general_purpose::STANDARD.decode(b64.iter().filter(|c| !c.is_ascii_whitespace()).copied().collect::<Vec<u8>>()) {
                            let map = crate::complete::parse_aliases(&String::from_utf8_lossy(&raw));
                            self.events.push(json!({"type": "aliases", "map": map}));
                        }
                        rest = &rest[end..];
                    }
                    None => {
                        if buf.len() < 1 << 20 {
                            buf.extend_from_slice(rest);
                        } else {
                            self.alias_buf = None;
                        }
                        return;
                    }
                }
            }
            match rest.windows(PAT.len()).position(|w| w == PAT) {
                Some(i) => {
                    self.alias_buf = Some(Vec::new());
                    rest = &rest[i + PAT.len()..];
                }
                None => return,
            }
        }
    }

    /// Pictures in the output, made into kitty images the terminal shows:
    /// iTerm2's inline images, and a command's output that is an image file.
    fn transform_images(&mut self, data: &[u8]) -> Vec<u8> {
        const FILE: &[u8] = b"\x1b]1337;File=";
        const START: &[u8] = b"\x1b]133;C";
        const END: &[u8] = b"\x1b]133;D";
        let find = |hay: &[u8], pat: &[u8]| hay.windows(pat.len()).position(|w| w == pat);
        let cols = self.size.get().0 as u32;
        let mut out = Vec::with_capacity(data.len());
        let mut rest: &[u8] = data;
        loop {
            // An image file a command is printing: collect it until it ends.
            if let Some(buf) = self.raw_image.as_mut() {
                // (The end mark may straddle two reads: search from just
                // before where this read starts.)
                let from = buf.len().saturating_sub(END.len());
                buf.extend_from_slice(rest);
                if let Some(i) = find(&buf[from..], END) {
                    let i = from + i;
                    let img = crate::images::un_onlcr(&buf[..i]);
                    if crate::images::is_image(&img) {
                        self.cell_image = Some(img.clone());
                    }
                    let tail = buf[i..].to_vec();
                    self.raw_image = None;
                    match crate::images::kitty_display(&img, cols.saturating_sub(2), self.size.get().2) {
                        Some(k) => out.extend_from_slice(&k),
                        None => out.extend_from_slice(b"[tessera: couldn't read this image]\r\n"),
                    }
                    out.extend_from_slice(&self.transform_images(&tail));
                    return out;
                }
                if buf.len() > 64 << 20 {
                    // Too big to be a picture to show: let it through.
                    out.extend_from_slice(&self.raw_image.take().unwrap());
                }
                return out;
            }
            // An iTerm2 inline image being read.
            if let Some(buf) = self.file_buf.as_mut() {
                let end = rest.iter().position(|&b| b == 7).map(|i| (i, 1)).or_else(|| find(rest, b"\x1b\\").map(|i| (i, 2)));
                match end {
                    Some((i, n)) => {
                        buf.extend_from_slice(&rest[..i]);
                        let payload = self.file_buf.take().unwrap();
                        if let Some(img) = crate::images::iterm2_file(&payload) {
                            if let Some(k) = crate::images::kitty_display(&img, cols.saturating_sub(2), self.size.get().2) {
                                out.extend_from_slice(&k);
                            }
                        }
                        rest = &rest[i + n..];
                    }
                    None => {
                        if buf.len() < 64 << 20 {
                            buf.extend_from_slice(rest);
                        } else {
                            self.file_buf = None;
                        }
                        return out;
                    }
                }
                continue;
            }
            // The first output after a command starts: an image file?
            if self.output_starts && !rest.is_empty() {
                self.output_starts = false;
                if crate::images::output_is_image(rest) {
                    self.raw_image = Some(Vec::new());
                    continue;
                }
            }
            let next_file = find(rest, FILE);
            let next_start = find(rest, START);
            match (next_file, next_start) {
                (Some(f), s) if s.is_none_or(|s| f < s) => {
                    out.extend_from_slice(&rest[..f]);
                    self.file_buf = Some(Vec::new());
                    rest = &rest[f + FILE.len()..];
                }
                (_, Some(s)) => {
                    // Through the end of the mark (BEL or ST), then the output.
                    let after = &rest[s..];
                    let end = after.iter().position(|&b| b == 7).map(|i| i + 1).or_else(|| find(after, b"\x1b\\").map(|i| i + 2)).unwrap_or(after.len());
                    out.extend_from_slice(&rest[..s + end]);
                    rest = &rest[s + end..];
                    self.output_starts = true;
                }
                _ => {
                    out.extend_from_slice(rest);
                    return out;
                }
            }
        }
    }

    /// Keep each command's output (between its marks), and when it ends,
    /// tell the window if it's something with a rich view (JSON, a table, ...).
    fn capture_output(&mut self, data: &[u8]) {
        const START: &[u8] = b"\x1b]133;C";
        const END: &[u8] = b"\x1b]133;D";
        let find = |hay: &[u8], pat: &[u8]| hay.windows(pat.len()).position(|w| w == pat);
        const CMD: &[u8] = b"\x1b]1337;TesseraCommand=";
        self.execs.feed(data);
        let base = data.as_ptr() as usize;
        let mut rest = data;
        loop {
            // A command-line report cut off by the last read: finish it.
            if let Some(buf) = self.command_buf.as_mut() {
                match rest.iter().position(|&b| b == 7 || b == 0x1b) {
                    Some(e) => {
                        buf.extend_from_slice(&rest[..e]);
                        let b64 = self.command_buf.take().unwrap();
                        self.pending_command = decode_b64_text(&b64);
                        rest = &rest[e..];
                    }
                    None => {
                        if buf.len() < 1 << 20 {
                            buf.extend_from_slice(rest);
                        } else {
                            self.command_buf = None;
                        }
                        return;
                    }
                }
            }
            // (A command starting inside the running one: a nested shell's.
            // The running one waits, and goes on when that one's done.)
            if self.cmd_out.is_some() {
                let e = find(rest, END);
                let n = [find(rest, CMD), find(rest, START)].into_iter().flatten().min();
                if let Some(n) = n.filter(|&n| e.is_none_or(|e| n < e)) {
                    let mut buf = self.cmd_out.take().unwrap();
                    buf.extend_from_slice(&rest[..n]);
                    self.cells.output(&rest[..n]);
                    self.cmd_outer.push(buf);
                    rest = &rest[n..];
                    continue;
                }
            }
            if let Some(buf) = self.cmd_out.as_mut() {
                match find(rest, END) {
                    Some(i) => {
                        buf.extend_from_slice(&rest[..i]);
                        let raw = self.cmd_out.take().unwrap();
                        self.cells.output(&rest[..i]);
                        let status: i64 = {
                            let after = &rest[i + END.len()..];
                            let digits: String = after.iter().skip_while(|&&c| c == b';').take_while(|c| c.is_ascii_digit()).map(|&c| c as char).collect();
                            digits.parse().unwrap_or(0)
                        };
                        let text = crate::cells::strip_frame(&strip_zsh_marker(crate::images::plain_text(&raw)));
                        // (The file's name says best what cat showed; else the text itself.)
                        let named = self.cells.list.iter().rev().find(|c| !c.done).map(|c| crate::images::kind_from_command(&c.command)).unwrap_or("");
                        let kind = if named.is_empty() { crate::images::detect_text(&text) } else { named };

                        if !kind.is_empty() {
                            self.events.push(json!({"type": "rich", "kind": kind, "size": text.len()}));
                        }
                        self.last_rich = (kind.to_string(), text);
                        let image = self.cell_image.take();
                        if let Some(id) = self.cells.finish(status, kind) {
                            if let Some(c) = self.cells.list.iter_mut().find(|c| c.id == id) {
                                c.image = image;
                            }
                            // (Kept on disk by tesserad, which sees the same output.)
                            self.events.push(json!({"type": "cell", "id": id, "done": true}));
                        }
                        rest = &rest[i + END.len()..];
                        // (Back to the command it ran under, if any.)
                        if let Some(outer) = self.cmd_outer.pop() {
                            self.cmd_out = Some(outer);
                        }
                    }
                    None => {
                        self.cells.output(rest);
                        if buf.len() + rest.len() <= 4 << 20 {
                            buf.extend_from_slice(rest);
                        } else {
                            self.cmd_out = None;
                        }
                        return;
                    }
                }
            }
            // In stream order: a command's line comes just before its start.
            let (c, s) = (find(rest, CMD), find(rest, START));
            if let Some(c) = c.filter(|&c| s.is_none_or(|s| c < s)) {
                let payload = &rest[c + CMD.len()..];
                match payload.iter().position(|&b| b == 7 || b == 0x1b) {
                    Some(e) => {
                        self.pending_command = decode_b64_text(&payload[..e]);
                        rest = &payload[e..];
                    }
                    None => {
                        self.command_buf = Some(payload.to_vec());
                        return;
                    }
                }
                continue;
            }
            match s {
                Some(s) => {
                    let after = &rest[s..];
                    let end = after.iter().position(|&b| b == 7).map(|i| i + 1).unwrap_or(after.len());
                    self.cmd_out = Some(Vec::new());
                    let cmd = std::mem::take(&mut self.pending_command);
                    let cwd = self.pwd();
                    let exec = self.execs.take_at(rest.as_ptr() as usize - base + s);
                    let id = self.cells.start(cmd, cwd);
                    self.cells.set_exec(id, exec);
                    self.events.push(json!({"type": "cell", "id": id, "done": false}));
                    rest = &rest[s + end..];
                }
                None => return,
            }
        }
    }

    /// The last finished command's output: {kind, text} (kind "" for plain).
    pub fn last_output(&self) -> String {
        json!({"kind": self.last_rich.0, "text": self.last_rich.1}).to_string()
    }

    /// Notebook cells: their facts (JSON list).
    pub fn cells(&self) -> String {
        self.cells.summary()
    }

    /// A cell's output: "html" (colors kept), "text", or "image" (base64 of
    /// the picture it printed); "exec": what it ran with (JSON).
    pub fn cell_output(&self, id: u64, format: &str) -> String {
        if format == "exec" {
            return self.cells.exec_json(id);
        }
        let Some(c) = self.cells.get(id) else { return String::new() };
        match format {
            "image" => {
                use base64::Engine;
                c.image.as_ref().map(|i| base64::engine::general_purpose::STANDARD.encode(i)).unwrap_or_default()
            }
            "html" => crate::cells::to_html(&c.raw, self.size.get().0),
            "markdown" => crate::cells::markdown_html(&crate::cells::strip_frame(&strip_zsh_marker(crate::images::plain_text(&c.raw)))),
            _ => crate::cells::strip_frame(&strip_zsh_marker(crate::images::plain_text(&c.raw))),
        }
    }

    fn process_output(&mut self, data: &[u8]) {
        self.scan_marks(data);
        self.scan_aliases(data);
        let transformed = self.transform_images(data);
        self.capture_output(&transformed);
        self.term.vt_write(&transformed);
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
        if let Some(r) = self.rec.as_mut() {
            if (self.size.get().0, self.size.get().1) != (cols, rows) {
                r.resize(cols, rows);
            }
        }
        self.size.set((cols, rows, cell_w, cell_h));
        let _ = self.term.resize(cols, rows, cell_w.max(1), cell_h.max(1));
        let Backend::Daemon { stream, .. } = &mut self.backend;
        let payload = serde_json::to_vec(&Resize { cols, rows }).unwrap();
        write_blocking(stream, T_RESIZE, &payload);
    }

    pub fn key_event(&mut self, qt_key: i32, qt_mods: u32, text: &str, release: bool, _native_vk: u32) -> bool {
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

    /// Save the screen and scrollback (VT sequences) for after a reboot.
    pub fn save_snapshot(&self, path: &str) -> bool {
        let data = crate::snapshot::snapshot(&self.term);
        if let Some(d) = std::path::Path::new(path).parent() {
            let _ = std::fs::create_dir_all(d);
        }
        std::fs::write(path, data).is_ok()
    }

    /// Show a saved screen above a new shell's output (its session is gone).
    pub fn preload(&mut self, path: &str) -> bool {
        let Ok(data) = std::fs::read(path) else { return false };
        self.term.vt_write(&data);
        // (Its notebook cells come back too, once the new shell has an id.)
        self.cells_from = std::path::Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned());
        // Continue below the last line with anything on it.
        let rows = self.term.rows().unwrap_or(1) as usize;
        let top = self.total_rows().saturating_sub(rows);
        let last = (0..rows).rev().find(|&r| !self.row_text(top + r).0.trim().is_empty());
        let goto = match last {
            Some(r) if r + 1 < rows => format!("\x1b[{};1H", r + 2),
            Some(_) => "\r\n".to_string(),
            None => "\x1b[1;1H".to_string(),
        };
        self.term.vt_write(format!("\x1b[0m{goto}\x1b[2m\u{2500}\u{2500} session restored \u{2500}\u{2500}\x1b[0m\r\n").as_bytes());
        let _ = std::fs::remove_file(path);
        self.force_full = true;
        true
    }

    pub fn show_message(&mut self, text: &str) {
        self.term.vt_write(text.replace('\n', "\r\n").as_bytes());
        self.force_full = true;
    }

    pub fn reset(&mut self) {
        self.highlights.clear();
        self.term.reset();
        self.force_full = true;
        self.write_input(b"\x0c");
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

    /// Start (a path) or stop ("") recording the output as an asciicast;
    /// stopping returns the recording's path.
    pub fn record_to_file(&mut self, path: &str) -> String {
        if path.is_empty() {
            return match self.rec.take() {
                Some(r) => {
                    let p = r.path.clone();
                    r.finish();
                    p
                }
                None => String::new(),
            };
        }
        let (cols, rows, _, _) = self.size.get();
        // (It starts with the screen as it is.)
        let screen = match (
            self.term.grid_ref(Point::Active(PointCoordinate { x: 0, y: 0 })),
            self.term.grid_ref(Point::Active(PointCoordinate { x: cols.saturating_sub(1), y: rows.saturating_sub(1) as u32 })),
        ) {
            (Ok(a), Ok(b)) => {
                let sel = Selection::new(a, b, false);
                let vt = format_selection_as(&self.term, &sel, Format::Vt, true);
                let cur = self.term.cursor_x().ok().zip(self.term.cursor_y().ok());
                let at = cur.map(|(x, y)| format!("\x1b[{};{}H", y + 1, x + 1)).unwrap_or_default();
                format!("\x1b[2J\x1b[H{}{at}", vt.replace("\r\n", "\n").replace('\n', "\r\n"))
            }
            _ => String::new(),
        };
        match crate::record::Recorder::start(path, cols, rows, &screen) {
            Some(r) => {
                self.rec = Some(r);
                r_path(&self.rec)
            }
            None => String::new(),
        }
    }

    pub fn is_recording(&self) -> bool {
        self.rec.is_some()
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
        self.write_input(b"\x0c");
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
        if self.term.mode(Mode::FOCUS_EVENT).unwrap_or(false) {
            self.write_input(if focused { b"\x1b[I" } else { b"\x1b[O" });
        }
    }

    pub fn kill(&mut self) {
        match &mut self.backend {
            Backend::Daemon { stream, id, host, .. } => {
                let (id, host) = (id.clone(), host.clone());
                std::thread::spawn(move || {
                    let _ = crate::client::rpc(&host, "cells.forget", json!({"session": id}));
                });
                write_blocking(stream, T_KILL, b"")
            }
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
        // (Another host's shell: the path is the host's, looked for there.)
        let Backend::Daemon { host, .. } = &self.backend;
        let host = host.clone();
        let home = if host.is_empty() { std::env::var("HOME").unwrap_or_default() } else { crate::client::host_home(&host) };
        let path = if let Some(rest) = plain.strip_prefix("~/") {
            std::path::PathBuf::from(&home).join(rest)
        } else if plain.starts_with('/') {
            std::path::PathBuf::from(plain)
        } else {
            let base = self.pwd();
            if base.is_empty() { return String::new() }
            std::path::PathBuf::from(base).join(plain)
        };
        if plain.is_empty() {
            return String::new();
        }
        if !host.is_empty() {
            let p = path.display().to_string();
            let there = crate::client::rpc_within(&host, "files", json!({"op": "exists", "path": p}), std::time::Duration::from_secs(2));
            return if there.ok().as_ref().and_then(|v| v.as_str()) == Some("true") { crate::client::ssh_path(&host, &p) } else { String::new() };
        }
        if path.exists() {
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

    pub fn image_pixels(&self, id: u32) -> crate::bridge::ffi::ImagePixels {
        crate::image_places::rgba(&self.term, id)
    }

    pub fn full_screen(&self) -> bool {
        matches!(self.term.active_screen(), Ok(Screen::Alternate))
    }

    pub fn wants_mouse(&self, qt_mods: u32) -> bool {
        self.opts.mouse_reporting
            && self.term.is_mouse_tracking().unwrap_or(false)
            && !keymap::mods(qt_mods).contains(key::Mods::SHIFT)
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
        let cursor_x = cursor.filter(|_| cursor_visible).map(|c| c.x);
        let cursor_moved = cursor_row != self.last_cursor_row || cursor_x != self.last_cursor_x;
        if !full && dirty == Dirty::Clean && !cursor_moved {
            return false;
        }
        let old_cursor_row = self.last_cursor_row;
        self.last_cursor_row = cursor_row;
        self.last_cursor_x = cursor_x;
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
                if self.ph_rows.len() <= y as usize {
                    self.ph_rows.resize(y as usize + 1, Vec::new());
                }
                self.ph_rows[y as usize].clear();
                let has_ph = row.raw_row().ok().and_then(|r| r.has_kitty_virtual_placeholder().ok()).unwrap_or(false);
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
                        // (A kitty placeholder: the image draws there, not the character.)
                        if has_ph && text.starts_with(crate::images::PLACEHOLDER) {
                            let st = cell.style().ok();
                            let none = libghostty_vt::style::StyleColor::None;
                            crate::images::placeholder_cell(&mut self.ph_rows[y as usize], x, &text, st.map(|s| s.fg_color).unwrap_or(none), st.map(|s| s.underline_color).unwrap_or(none));
                            text.clear();
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
                        if target > 1.0 && flags & FLAG_INVISIBLE == 0 && !crate::contrast::is_drawing(&text) {
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
        let (_, _, cw, ch) = self.size.get();
        crate::image_places::placements(&self.term, cw, ch, &mut out.images);
        self.ph_rows.truncate(rows as usize);
        crate::image_places::placeholder_places(&self.term, &self.ph_rows, cw, ch, &mut out.images);
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

use crate::cells::{decode_b64_text, strip_zsh_marker};


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

fn r_path(r: &Option<crate::record::Recorder>) -> String {
    r.as_ref().map(|r| r.path.clone()).unwrap_or_default()
}
