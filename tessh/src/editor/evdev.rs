//! Lone Shift and Ctrl taps in terminals that can't report them (no kitty
//! keyboard protocol): the keyboards themselves, read from /dev/input (the
//! user in the `input` group; keyd's virtual keyboard included). Only which
//! key went down or up and when is kept, nothing else, and only while the
//! editor waits for input; the caller acts on a tap only while its terminal
//! has focus (focus reports, and: a key the keyboard sends that doesn't
//! reach this terminal soon after means someone's typing elsewhere).
//! Not over SSH (the keyboard isn't on this machine); TESSH_EVDEV=0: off.

use std::io::Read;
use std::os::fd::AsRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::Instant;

pub const LEFT_SHIFT: u16 = 42;
pub const RIGHT_SHIFT: u16 = 54;
pub const LEFT_CTRL: u16 = 29;
pub const RIGHT_CTRL: u16 = 97;

pub struct Ev {
    pub code: u16,
    pub down: bool,
    pub at: Instant,
}

impl Ev {
    pub fn is_modifier(&self) -> bool {
        // (Shift, Ctrl, Alt, AltGr, Super, Caps Lock, Num and Scroll Lock:
        // keys that send nothing to the terminal by themselves.)
        matches!(self.code, 29 | 42 | 54 | 56 | 58 | 69 | 70 | 97 | 100 | 125 | 126 | 127)
    }
    /// A key that types something in a terminal (the main block, arrows,
    /// Home/End…): if it doesn't reach ours, it went to another window.
    pub fn types(&self) -> bool {
        !self.is_modifier() && (1..=88).contains(&self.code) || (102..=111).contains(&self.code)
    }
}

pub struct Evdev {
    pub rx: Receiver<Ev>,
    pub active: Arc<AtomicBool>,
}

/// The keyboards: devices with keys and key repeat (EV=…13 with the repeat
/// bit), from /proc/bus/input/devices.
fn keyboards() -> Vec<String> {
    let text = std::fs::read_to_string("/proc/bus/input/devices").unwrap_or_default();
    let mut out = vec![];
    for dev in text.split("\n\n") {
        let ev = dev.lines().find_map(|l| l.strip_prefix("B: EV=")).and_then(|v| u64::from_str_radix(v.trim(), 16).ok()).unwrap_or(0);
        // (EV_KEY 1, EV_REP 0x14: a keyboard; a mouse has keys but no repeat.)
        if ev & 0x2 == 0 || ev & (1 << 0x14) == 0 {
            continue;
        }
        let keys = dev.lines().find_map(|l| l.strip_prefix("B: KEY=")).unwrap_or("");
        // (Has the letter keys: the lowest word's bits for KEY_Q..KEY_P etc.)
        let low = keys.split_whitespace().last().and_then(|w| u64::from_str_radix(w, 16).ok()).unwrap_or(0);
        if low & (1 << 16) == 0 {
            continue;
        }
        if let Some(h) = dev.lines().find_map(|l| l.strip_prefix("H: Handlers=")) {
            for w in h.split_whitespace() {
                if w.starts_with("event") {
                    out.push(format!("/dev/input/{w}"));
                }
            }
        }
    }
    out
}

pub fn start() -> Option<Evdev> {
    if std::env::var("TESSH_EVDEV").map(|v| v == "0").unwrap_or(false) {
        return None;
    }
    if std::env::var_os("SSH_CONNECTION").is_some() || std::env::var_os("SSH_TTY").is_some() {
        return None;
    }
    let (mut paths, mut files) = (vec![], vec![]);
    for p in keyboards() {
        if let Ok(f) = std::fs::File::open(&p) {
            files.push(f);
            paths.push(p);
        }
    }
    if files.is_empty() {
        return None;
    }
    let (tx, rx) = channel();
    let active = Arc::new(AtomicBool::new(false));
    let on = active.clone();
    std::thread::Builder::new()
        .name("tessh-keys".into())
        .spawn(move || {
            let mut fds: Vec<libc::pollfd> = files.iter().map(|f| libc::pollfd { fd: f.as_raw_fd(), events: libc::POLLIN, revents: 0 }).collect();
            // struct input_event: timeval (16 bytes on 64-bit), u16 type, u16 code, i32 value.
            let size = std::mem::size_of::<libc::timeval>() + 8;
            let mut buf = vec![0u8; size * 64];
            let mut scanned = Instant::now();
            loop {
                // (Keyboards plugged in since: every few seconds.)
                if scanned.elapsed().as_secs() >= 5 {
                    scanned = Instant::now();
                    for p in keyboards() {
                        if !paths.contains(&p) {
                            if let Ok(f) = std::fs::File::open(&p) {
                                fds.push(libc::pollfd { fd: f.as_raw_fd(), events: libc::POLLIN, revents: 0 });
                                files.push(f);
                                paths.push(p);
                            }
                        }
                    }
                }
                if unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 5000) } <= 0 {
                    continue;
                }
                for (i, p) in fds.iter_mut().enumerate() {
                    if p.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                        // (Unplugged: stop asking it.)
                        p.fd = -1;
                        continue;
                    }
                    if p.revents & libc::POLLIN == 0 {
                        continue;
                    }
                    let n = match files[i].read(&mut buf) {
                        Ok(n) => n,
                        Err(_) => continue,
                    };
                    if !on.load(Ordering::Relaxed) {
                        continue;
                    }
                    let at = Instant::now();
                    for e in buf[..n].chunks_exact(size) {
                        let o = size - 8;
                        let ty = u16::from_ne_bytes([e[o], e[o + 1]]);
                        let code = u16::from_ne_bytes([e[o + 2], e[o + 3]]);
                        let value = i32::from_ne_bytes([e[o + 4], e[o + 5], e[o + 6], e[o + 7]]);
                        // (EV_KEY; 1 down, 0 up, 2 repeat.)
                        if ty == 1 && value != 2 && tx.send(Ev { code, down: value == 1, at }).is_err() {
                            return;
                        }
                    }
                }
            }
        })
        .ok()?;
    Some(Evdev { rx, active })
}
