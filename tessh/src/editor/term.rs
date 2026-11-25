//! The terminal, while the editor has it: raw mode, the kitty keyboard
//! protocol (lone Shift and Ctrl taps, Shift+Enter, Ctrl+Enter), the mouse
//! (clicks, drags, the wheel, and motion for tooltips: SGR reports), pasting
//! as one piece, focus reports, and the colors the terminal uses (OSC 10/11),
//! for the panel's tints. No alternate screen: the editor is drawn inline
//! at the bottom, and what scrolls up stays in the terminal's scrollback.
//!
//! The mouse: a terminal with mouse reports on sends the wheel here instead
//! of scrolling its scrollback, and a drag here instead of selecting. So the
//! wheel outside a popup, or a click above the editor, gives the mouse back
//! (reports off) until the next key or focus change: the terminal scrolls and
//! selects as usual. (Shift+drag selects in most terminals regardless.)

use crossterm::event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags};
use std::io::{Read, Write};
use std::time::{Duration, Instant};

pub struct Term {
    pub kitty: bool,
    pub mouse_on: bool,
    pub mouse_allowed: bool,
    pub fg: Option<(u8, u8, u8)>,
    pub bg: Option<(u8, u8, u8)>,
    raw: bool,
    /// The probe's answers didn't all come in time (a terminal that's slow
    /// to answer, or a session no window shows yet): until then, an OSC
    /// reply that comes late is read as one, not typed into the line.
    late_until: Option<Instant>,
    late_buf: Option<String>,
}

const MOUSE_ON: &str = "\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1006h";
const MOUSE_OFF: &str = "\x1b[?1003l\x1b[?1002l\x1b[?1000l\x1b[?1006l";

fn out(s: &str) {
    let mut o = std::io::stdout();
    let _ = o.write_all(s.as_bytes());
    let _ = o.flush();
}

/// Read what the terminal answers within `wait`, until `end` returns true.
fn read_reply(wait: Duration, end: impl Fn(&[u8]) -> bool) -> Vec<u8> {
    let mut got = Vec::new();
    let t = Instant::now();
    let mut fds = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
    while t.elapsed() < wait && !end(&got) {
        let left = wait.saturating_sub(t.elapsed()).as_millis() as i32;
        if unsafe { libc::poll(&mut fds, 1, left.max(1)) } <= 0 {
            break;
        }
        let mut b = [0u8; 256];
        match std::io::stdin().read(&mut b) {
            Ok(n) if n > 0 => got.extend_from_slice(&b[..n]),
            _ => break,
        }
    }
    got
}

/// "rgb:rrrr/gggg/bbbb" (or 2- and 1-digit forms) in an OSC 10/11 reply.
fn parse_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let i = s.find("rgb:")?;
    let rest = &s[i + 4..];
    let end = rest.find(|c: char| !(c.is_ascii_hexdigit() || c == '/')).unwrap_or(rest.len());
    let parts: Vec<&str> = rest[..end].split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    let ch = |p: &str| -> Option<u8> {
        let v = u32::from_str_radix(p, 16).ok()?;
        let max = (1u32 << (4 * p.len() as u32)) - 1;
        Some(((v * 255 + max / 2) / max.max(1)) as u8)
    };
    Some((ch(parts[0])?, ch(parts[1])?, ch(parts[2])?))
}

impl Term {
    pub fn new() -> Term {
        let mouse_allowed = std::env::var("TESSH_MOUSE").map(|v| v != "0").unwrap_or(true);
        let mut t = Term { kitty: false, mouse_on: false, mouse_allowed, fg: None, bg: None, raw: false, late_until: None, late_buf: None };
        t.probe();
        t
    }

    /// Ask the terminal (once): its colors, and whether it has the kitty
    /// keyboard protocol. Primary device attributes (which every terminal
    /// answers) end the wait early.
    fn probe(&mut self) {
        if unsafe { libc::isatty(0) != 1 || libc::isatty(1) != 1 } {
            return;
        }
        let _ = crossterm::terminal::enable_raw_mode();
        out("\x1b]10;?\x1b\\\x1b]11;?\x1b\\\x1b[?u\x1b[c");
        let got = read_reply(Duration::from_millis(400), |g| {
            // (DA1: ESC [ ? … c)
            let s = String::from_utf8_lossy(g);
            s.rfind("\x1b[?").is_some_and(|i| s[i..].contains('c'))
        });
        let _ = crossterm::terminal::disable_raw_mode();
        let s = String::from_utf8_lossy(&got).to_string();
        if !(s.contains("\x1b]10;") && s.contains("\x1b]11;")) {
            self.late_until = Some(Instant::now() + Duration::from_secs(10));
        }
        for part in s.split("\x1b]").skip(1) {
            if let Some(r) = part.strip_prefix("10;") {
                self.fg = parse_rgb(r);
            } else if let Some(r) = part.strip_prefix("11;") {
                self.bg = parse_rgb(r);
            }
        }
        // (kitty's answer to CSI ? u: CSI ? flags u.)
        let kitty_reply = s.split("\x1b[?").any(|p| {
            let d: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
            p[d.len()..].starts_with('u')
        });
        self.kitty = kitty_reply && std::env::var("TESSH_KITTY_KEYS").map(|v| v != "0").unwrap_or(true);
    }

    /// A key that is part of a late OSC 10/11 answer (crossterm reads its
    /// ESC ] as Alt+], the rest as typing, ST's ESC \\ as Alt+\\ or BEL
    /// as Ctrl+G): taken as the answer, colours and all. True when it was.
    pub fn late_key(&mut self, k: &crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        if self.late_until.is_none_or(|t| Instant::now() > t) {
            self.late_until = None;
            self.late_buf = None;
            return false;
        }
        let alt = k.modifiers.contains(KeyModifiers::ALT);
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let Some(buf) = self.late_buf.as_mut() else {
            if alt && k.code == KeyCode::Char(']') {
                self.late_buf = Some(String::new());
                return true;
            }
            return false;
        };
        match k.code {
            KeyCode::Char('\\') if alt => {}
            KeyCode::Char('g') if ctrl => {}
            // (One answer right after the other: ESC ] again.)
            KeyCode::Char(']') if alt => {}
            KeyCode::Char(c) => {
                buf.push(c);
                return true;
            }
            _ => return true,
        }
        let got = self.late_buf.take().unwrap_or_default();
        if let Some(r) = got.strip_prefix("10;") {
            self.fg = parse_rgb(r).or(self.fg);
        } else if let Some(r) = got.strip_prefix("11;") {
            self.bg = parse_rgb(r).or(self.bg);
        }
        if alt && k.code == KeyCode::Char(']') {
            self.late_buf = Some(String::new());
        } else if self.fg.is_some() && self.bg.is_some() {
            self.late_until = None;
        }
        true
    }

    pub fn enter(&mut self) {
        let _ = crossterm::terminal::enable_raw_mode();
        self.raw = true;
        out("\x1b[?2004h\x1b[?1004h");
        if self.kitty {
            let f = KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
                | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES;
            let _ = crossterm::execute!(std::io::stdout(), PushKeyboardEnhancementFlags(f));
        }
        self.set_mouse(true);
    }

    pub fn leave(&mut self) {
        if !self.raw {
            return;
        }
        self.set_mouse(false);
        if self.kitty {
            let _ = crossterm::execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
        }
        out("\x1b[?1004l\x1b[?2004l\x1b[0 q");
        let _ = crossterm::terminal::disable_raw_mode();
        self.raw = false;
    }

    /// Mouse reports on or off (off: the terminal scrolls and selects).
    pub fn set_mouse(&mut self, on: bool) {
        let on = on && self.mouse_allowed;
        if on != self.mouse_on {
            out(if on { MOUSE_ON } else { MOUSE_OFF });
            self.mouse_on = on;
        }
    }

    pub fn size() -> (u16, u16) {
        crossterm::terminal::size().unwrap_or((80, 24))
    }

    /// The cursor's row now (0-based), asked of the terminal.
    pub fn cursor_row() -> u16 {
        crossterm::cursor::position().map(|(_, r)| r).unwrap_or(0)
    }

    /// Copy to the system clipboard through the terminal (OSC 52).
    pub fn copy(text: &str) {
        use base64::Engine;
        out(&format!("\x1b]52;c;{}\x07", base64::engine::general_purpose::STANDARD.encode(text)));
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        self.leave();
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn late_answer_isnt_typed() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let mut t = super::Term { kitty: false, mouse_on: false, mouse_allowed: false, fg: None, bg: None, raw: false, late_until: Some(std::time::Instant::now() + std::time::Duration::from_secs(5)), late_buf: None };
        let key = |c: char, m: KeyModifiers| KeyEvent::new(KeyCode::Char(c), m);
        let mut eaten = true;
        eaten &= t.late_key(&key(']', KeyModifiers::ALT));
        for c in "10;rgb:d3d3/d6d6/dbdb".chars() {
            eaten &= t.late_key(&key(c, KeyModifiers::NONE));
        }
        eaten &= t.late_key(&key(']', KeyModifiers::ALT));
        for c in "11;rgb:0000/0000/0000".chars() {
            eaten &= t.late_key(&key(c, KeyModifiers::NONE));
        }
        eaten &= t.late_key(&key('\\', KeyModifiers::ALT));
        assert!(eaten);
        assert_eq!(t.fg, Some((0xd3, 0xd6, 0xdb)));
        assert_eq!(t.bg, Some((0, 0, 0)));
        // (Both in: typing is typing again.)
        assert!(!t.late_key(&key('a', KeyModifiers::NONE)));
    }

    #[test]
    fn colors() {
        assert_eq!(super::parse_rgb("rgb:ffff/0000/8080\x1b\\"), Some((255, 0, 128)));
        assert_eq!(super::parse_rgb("rgb:1e/1e/2e"), Some((30, 30, 46)));
    }
}
