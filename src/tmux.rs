//! tmux control mode (`tmux -CC`), the way iTerm2 does it.
//!
//! When a shell prints the control-mode DCS (`ESC P 1000 p`), that session
//! becomes a *gateway*: its output is tmux's line protocol instead of screen
//! content. Each tmux window becomes a native tab and each tmux pane a native
//! split, backed by its own libghostty terminal fed from `%output`. Keys go
//! back as `send-keys -H`. Closing/splitting/new tab in those tabs turn into
//! tmux commands, and tmux answers with `%layout-change`, which rebuilds the
//! splits.

use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::rc::Rc;

pub const DCS_START: &[u8] = b"\x1bP1000p";

/// Per-pane state shared between the gateway (producer) and the pane's
/// `Session` (consumer). A pipe wakes the pane's view, so the C++ side can
/// watch every session the same way (one fd, readable = work to do).
pub struct PaneShared {
    pub pending: Vec<u8>,
    held: Vec<u8>,
    capturing: bool,
    pub grid: (u16, u16),
    pub grid_changed: bool,
    pub closed: bool,
    wake_r: OwnedFd,
    wake_w: OwnedFd,
}

impl PaneShared {
    fn new() -> Self {
        let mut fds = [0i32; 2];
        unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_NONBLOCK | libc::O_CLOEXEC) };
        Self {
            pending: Vec::new(),
            held: Vec::new(),
            capturing: false,
            grid: (80, 24),
            grid_changed: false,
            closed: false,
            wake_r: unsafe { OwnedFd::from_raw_fd(fds[0]) },
            wake_w: unsafe { OwnedFd::from_raw_fd(fds[1]) },
        }
    }
    fn wake(&self) {
        unsafe { libc::write(self.wake_w.as_raw_fd(), b"x".as_ptr().cast(), 1) };
    }
    pub fn wake_fd(&self) -> i32 {
        self.wake_r.as_raw_fd()
    }
    pub fn drain_wake(&self) {
        let mut buf = [0u8; 256];
        while unsafe { libc::read(self.wake_r.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len()) } > 0 {}
    }
    fn output(&mut self, data: &[u8]) {
        if self.capturing {
            self.held.extend_from_slice(data);
        } else {
            self.pending.extend_from_slice(data);
            self.wake();
        }
    }
}

enum Cmd {
    Ignore,
    ListWindows,
    Capture(u32),
    Cursors,
}

pub struct Gateway {
    pub key: String,
    out: UnixStream,
    line: Vec<u8>,
    pending: VecDeque<Cmd>,
    block: Option<(Cmd, Vec<Vec<u8>>)>,
    pub panes: HashMap<u32, Rc<RefCell<PaneShared>>>,
    pub events: Vec<Value>,
    pub ended: bool,
    client_size: (u16, u16),
}

thread_local! {
    static GATEWAYS: RefCell<HashMap<String, Rc<RefCell<Gateway>>>> = RefCell::default();
}

pub fn lookup(key: &str) -> Option<Rc<RefCell<Gateway>>> {
    GATEWAYS.with(|g| g.borrow().get(key).cloned())
}

pub fn start(out: UnixStream, cols: u16, rows: u16) -> Rc<RefCell<Gateway>> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static N: AtomicU32 = AtomicU32::new(1);
    let key = format!("tmux{}", N.fetch_add(1, Ordering::Relaxed));
    let gw = Rc::new(RefCell::new(Gateway {
        key: key.clone(),
        out,
        line: Vec::new(),
        pending: VecDeque::new(),
        block: None,
        panes: HashMap::new(),
        events: vec![json!({"type": "tmuxStart", "gateway": key})],
        ended: false,
        client_size: (0, 0),
    }));
    {
        let mut g = gw.borrow_mut();
        g.set_client_size(cols, rows);
        g.command("list-windows -F \"#{window_id}\t#{window_layout}\t#{window_name}\t#{window_active}\"", Cmd::ListWindows);
    }
    GATEWAYS.with(|g| g.borrow_mut().insert(key, gw.clone()));
    gw
}

pub fn unregister(key: &str) {
    GATEWAYS.with(|g| g.borrow_mut().remove(key));
}

impl Gateway {
    fn command(&mut self, cmd: &str, kind: Cmd) {
        if let Some(path) = std::env::var_os("TESSERA_TMUX_LOG") {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "> {cmd}");
            }
        }
        let mut line = cmd.as_bytes().to_vec();
        line.push(b'\n');
        crate::session::write_blocking(&mut self.out, crate::proto::T_INPUT, &line);
        self.pending.push_back(kind);
    }

    /// A command typed by the user (new-window, split-window, ...).
    pub fn user_command(&mut self, cmd: &str) {
        self.command(cmd, Cmd::Ignore);
    }

    pub fn set_client_size(&mut self, cols: u16, rows: u16) {
        if (cols, rows) != self.client_size && cols > 0 && rows > 0 {
            self.client_size = (cols, rows);
            self.command(&format!("refresh-client -C {cols}x{rows}"), Cmd::Ignore);
        }
    }

    pub fn send_keys(&mut self, pane: u32, bytes: &[u8]) {
        for chunk in bytes.chunks(256) {
            let mut cmd = format!("send-keys -t %{pane} -H");
            for b in chunk {
                cmd.push_str(&format!(" {b:02x}"));
            }
            self.command(&cmd, Cmd::Ignore);
        }
    }

    pub fn detach(&mut self) {
        self.command("detach-client", Cmd::Ignore);
    }

    /// Feed gateway output. Once tmux exits, returns whatever followed the
    /// end of control mode (normal shell output again).
    pub fn feed(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        self.line.extend_from_slice(data);
        loop {
            if self.ended {
                // Control mode ends with ST after %exit.
                let rest = match find(&self.line, b"\x1b\\") {
                    Some(i) => self.line[i + 2..].to_vec(),
                    None => std::mem::take(&mut self.line),
                };
                self.line.clear();
                return Some(rest);
            }
            let Some(nl) = self.line.iter().position(|&b| b == b'\n') else { return None };
            let mut line: Vec<u8> = self.line.drain(..=nl).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            self.handle_line(line);
        }
    }

    fn handle_line(&mut self, line: Vec<u8>) {
        if let Some(path) = std::env::var_os("TESSERA_TMUX_LOG") {
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(f, "< {}", String::from_utf8_lossy(&line).chars().take(200).collect::<String>());
            }
        }
        if let Some((_, lines)) = self.block.as_mut() {
            if line.starts_with(b"%end ") || line.starts_with(b"%error ") {
                let (cmd, lines) = self.block.take().unwrap();
                let ok = line.starts_with(b"%end ");
                self.finish(cmd, lines, ok);
            } else {
                lines.push(line);
            }
            return;
        }
        let text = String::from_utf8_lossy(&line).into_owned();
        let mut parts = text.splitn(2, ' ');
        let head = parts.next().unwrap_or("");
        let rest = parts.next().unwrap_or("");
        match head {
            "%begin" => {
                // The last field is 1 for replies to this client's commands;
                // 0 marks output of the commands that started tmux.
                let ours = rest.rsplit(' ').next().map(|f| f.trim() != "0").unwrap_or(true);
                let cmd = if ours { self.pending.pop_front().unwrap_or(Cmd::Ignore) } else { Cmd::Ignore };
                self.block = Some((cmd, Vec::new()));
            }
            "%output" => {
                // Work on raw bytes: the data isn't necessarily UTF-8.
                let after = &line[b"%output ".len().min(line.len())..];
                if let Some(sp) = after.iter().position(|&b| b == b' ') {
                    let id = parse_pane(&String::from_utf8_lossy(&after[..sp]));
                    if let Some(p) = id.and_then(|id| self.panes.get(&id)) {
                        p.borrow_mut().output(&unescape(&after[sp + 1..]));
                    }
                }
            }
            "%extended-output" => {
                // %extended-output %ID MS ... : DATA
                if let Some(colon) = find(&line, b" : ") {
                    let id = rest.split(' ').next().and_then(parse_pane);
                    if let Some(p) = id.and_then(|id| self.panes.get(&id)) {
                        p.borrow_mut().output(&unescape(&line[colon + 3..]));
                    }
                }
            }
            "%layout-change" => {
                let mut f = rest.split(' ');
                let (Some(win), Some(layout)) = (f.next().and_then(parse_window), f.next()) else { return };
                self.apply_layout(win, layout, None, None);
            }
            "%window-add" | "%unlinked-window-add" => {
                self.command(
                    "list-windows -F \"#{window_id}\t#{window_layout}\t#{window_name}\t#{window_active}\"",
                    Cmd::ListWindows,
                );
            }
            "%window-close" | "%unlinked-window-close" => {
                if let Some(win) = parse_window(rest.trim()) {
                    self.events.push(json!({"type": "windowClose", "gateway": self.key, "window": win}));
                }
            }
            "%window-renamed" => {
                let mut f = rest.splitn(2, ' ');
                if let (Some(win), Some(name)) = (f.next().and_then(parse_window), f.next()) {
                    self.events.push(json!({"type": "windowRename", "gateway": self.key, "window": win, "name": name}));
                }
            }
            "%exit" => {
                self.ended = true;
                for p in self.panes.values() {
                    let mut p = p.borrow_mut();
                    p.closed = true;
                    p.wake();
                }
                self.events.push(json!({"type": "tmuxExit", "gateway": self.key}));
            }
            _ => {}
        }
    }

    fn finish(&mut self, cmd: Cmd, lines: Vec<Vec<u8>>, ok: bool) {
        match cmd {
            Cmd::Ignore => {
                if !ok {
                    let msg: Vec<String> = lines.iter().map(|l| String::from_utf8_lossy(l).into_owned()).collect();
                    self.events.push(json!({"type": "tmuxError", "gateway": self.key, "message": msg.join("\n")}));
                }
            }
            Cmd::ListWindows => {
                for l in lines {
                    let l = String::from_utf8_lossy(&l).into_owned();
                    let f: Vec<&str> = l.split('\t').collect();
                    if f.len() < 3 {
                        continue;
                    }
                    let Some(win) = parse_window(f[0]) else { continue };
                    let active = f.get(3) == Some(&"1");
                    self.apply_layout(win, f[1], Some(f[2]), Some(active));
                }
            }
            Cmd::Capture(pane) => {
                if let Some(p) = self.panes.get(&pane) {
                    let mut p = p.borrow_mut();
                    let mut data = b"\x1b[H\x1b[2J".to_vec();
                    for (i, l) in lines.iter().enumerate() {
                        if i > 0 {
                            data.extend_from_slice(b"\r\n");
                        }
                        data.extend_from_slice(l);
                    }
                    p.pending.extend_from_slice(&data);
                }
            }
            Cmd::Cursors => {
                for l in lines {
                    let l = String::from_utf8_lossy(&l).into_owned();
                    let f: Vec<&str> = l.split('\t').collect();
                    if f.len() < 3 {
                        continue;
                    }
                    let Some(id) = parse_pane(f[0]) else { continue };
                    let (x, y) = (f[1].parse::<u32>().unwrap_or(0), f[2].parse::<u32>().unwrap_or(0));
                    if let Some(p) = self.panes.get(&id) {
                        let mut p = p.borrow_mut();
                        if !p.capturing {
                            continue;
                        }
                        p.capturing = false;
                        let pos = format!("\x1b[{};{}H", y + 1, x + 1);
                        p.pending.extend_from_slice(pos.as_bytes());
                        let held = std::mem::take(&mut p.held);
                        p.pending.extend_from_slice(&held);
                        p.wake();
                    }
                }
            }
        }
    }

    /// Update pane grids from a layout, register new panes (capturing their
    /// contents) and tell QML. Returns true when captures were requested.
    fn apply_layout(&mut self, win: u32, layout: &str, name: Option<&str>, active: Option<bool>) -> bool {
        let Some((tree, panes)) = parse_layout(layout) else { return false };
        let mut new = Vec::new();
        for (id, w, h) in panes {
            let p = self.panes.entry(id).or_insert_with(|| {
                new.push(id);
                Rc::new(RefCell::new(PaneShared::new()))
            });
            let mut p = p.borrow_mut();
            if p.grid != (w, h) {
                p.grid = (w, h);
                p.grid_changed = true;
                p.wake();
            }
        }
        for &id in &new {
            self.panes[&id].borrow_mut().capturing = true;
            self.command(&format!("capture-pane -p -e -t %{id} -S -2000"), Cmd::Capture(id));
        }
        if !new.is_empty() {
            // Cursor positions, then the output held during capture is released.
            self.command("list-panes -s -F \"#{pane_id}\t#{cursor_x}\t#{cursor_y}\"", Cmd::Cursors);
        }
        let mut ev = json!({"type": "window", "gateway": self.key, "window": win, "layout": tree});
        if let Some(n) = name {
            ev["name"] = json!(n);
        }
        if let Some(a) = active {
            ev["active"] = json!(a);
        }
        self.events.push(ev);
        !new.is_empty()
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn parse_pane(s: &str) -> Option<u32> {
    s.strip_prefix('%')?.parse().ok()
}

fn parse_window(s: &str) -> Option<u32> {
    s.strip_prefix('@')?.parse().ok()
}

/// tmux escapes bytes < 32 and backslash in %output as \ooo.
fn unescape(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'\\' && i + 3 < data.len() + 0 && data[i + 1..i + 4].iter().all(|b| (b'0'..=b'7').contains(b)) {
            let v = (data[i + 1] - b'0') as u32 * 64 + (data[i + 2] - b'0') as u32 * 8 + (data[i + 3] - b'0') as u32;
            out.push(v as u8);
            i += 4;
        } else {
            out.push(data[i]);
            i += 1;
        }
    }
    out
}

/// Parse a tmux layout string (`csum,WxH,X,Y{...}` / `[...]` / `,ID`) into
/// a JSON tree for QML plus the list of (pane, cols, rows).
pub fn parse_layout(s: &str) -> Option<(Value, Vec<(u32, u16, u16)>)> {
    let s = s.split_once(',')?.1; // drop checksum
    let mut panes = Vec::new();
    let mut p = Parser { s: s.as_bytes(), i: 0 };
    let tree = p.node(&mut panes)?;
    Some((tree, panes))
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn num(&mut self) -> Option<u32> {
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn node(&mut self, panes: &mut Vec<(u32, u16, u16)>) -> Option<Value> {
        let w = self.num()?;
        self.eat(b'x').then_some(())?;
        let h = self.num()?;
        self.eat(b',').then_some(())?;
        self.num()?;
        self.eat(b',').then_some(())?;
        self.num()?;
        let (open, close, kind) = match self.s.get(self.i) {
            Some(b'{') => (b'{', b'}', "h"),
            Some(b'[') => (b'[', b']', "v"),
            _ => {
                self.eat(b',').then_some(())?;
                let id = self.num()?;
                panes.push((id, w as u16, h as u16));
                return Some(json!({"t": "pane", "id": id, "w": w, "h": h}));
            }
        };
        self.eat(open);
        let mut children = Vec::new();
        loop {
            children.push(self.node(panes)?);
            if self.eat(b',') {
                continue;
            }
            self.eat(close).then_some(())?;
            break;
        }
        Some(json!({"t": kind, "w": w, "h": h, "c": children}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_nested() {
        let (tree, panes) = parse_layout("b25d,159x48,0,0{79x48,0,0,1,79x48,80,0[79x24,80,0,2,79x23,80,25,3]}").unwrap();
        assert_eq!(panes, vec![(1, 79, 48), (2, 79, 24), (3, 79, 23)]);
        assert_eq!(tree["t"], "h");
        assert_eq!(tree["c"][1]["t"], "v");
    }

    #[test]
    fn layout_single() {
        let (tree, panes) = parse_layout("bb62,159x48,0,0,1").unwrap();
        assert_eq!(panes, vec![(1, 159, 48)]);
        assert_eq!(tree["t"], "pane");
    }

    #[test]
    fn octal() {
        assert_eq!(unescape(b"a\\033[1mb\\134"), b"a\x1b[1mb\\");
    }
}
