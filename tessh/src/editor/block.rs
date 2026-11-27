//! A rectangle of the text (column selection), outside vi: Alt+drag (or
//! Ctrl+Alt+drag where the terminal hands it on), or Ctrl+Shift+Alt+arrows
//! from the keyboard (VS Code's keys). Its corners are (line, column) in
//! characters, so it can reach past a short line's end.
//!
//! On it: Ctrl+C copies each line's part (one per line), Ctrl+X cuts,
//! Delete and Backspace remove the parts, typing goes on every line at once
//! (a block one column wide or none: a cursor on each line), and Ctrl+V puts
//! a paste of as many lines one on each line, else the same text on each.
//! Escape, a plain arrow or a click ends it.

use super::core::Ed;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// The rectangle: the corner it started at, and the moving one (the cursor's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blk {
    pub al: usize,
    pub ac: usize,
    pub cl: usize,
    pub cc: usize,
}

impl Blk {
    fn lines(&self) -> (usize, usize) {
        (self.al.min(self.cl), self.al.max(self.cl))
    }
    fn cols(&self) -> (usize, usize) {
        (self.ac.min(self.cc), self.ac.max(self.cc))
    }
}

impl Ed {
    /// (line, column in chars) of a position.
    pub fn line_col(&self, p: usize) -> (usize, usize) {
        (self.t.line_of(p), self.t.col_of(p))
    }

    /// Start a rectangle at `p` (the mouse's Alt+press, or the keyboard).
    pub fn blk_start(&mut self, p: usize) {
        let (l, c) = self.line_col(p);
        self.blk = Some(Blk { al: l, ac: c, cl: l, cc: c });
        self.t.anchor = None;
        self.t.cursor = p;
        self.dirty = true;
    }

    /// The moving corner to (line, col); the cursor goes as near as the
    /// line allows.
    pub fn blk_to(&mut self, l: usize, c: usize) {
        let Some(b) = self.blk.as_mut() else { return };
        let l = l.min(self.t.line_count() - 1);
        b.cl = l;
        b.cc = c;
        let ls = self.t.pos_of_line(l);
        self.t.cursor = self.t.pos_at_col(ls, c);
        self.dirty = true;
    }

    /// Each line's part, as byte ranges (for drawing and copying); a block
    /// with no width: the character at its column on each line (where it
    /// types), so it shows.
    pub fn blk_ranges(&self) -> Vec<(usize, usize)> {
        let Some(b) = self.blk else { return vec![] };
        let ((l1, l2), (c1, c2)) = (b.lines(), b.cols());
        let mut out = vec![];
        for l in l1..=l2 {
            let ls = self.t.pos_of_line(l);
            let a = self.t.pos_at_col(ls, c1);
            let e = self.t.line_end(ls);
            let z = if c2 > c1 { self.t.pos_at_col(ls, c2) } else if a < e { self.t.next(a) } else { a };
            out.push((a, z));
        }
        out
    }

    /// The block's text: each line's part, a line each.
    pub fn blk_copy_text(&self) -> String {
        self.blk_text()
    }
    fn blk_text(&self) -> String {
        let Some(b) = self.blk else { return String::new() };
        let ((l1, l2), (c1, c2)) = (b.lines(), b.cols());
        (l1..=l2)
            .map(|l| {
                let ls = self.t.pos_of_line(l);
                self.t.text[self.t.pos_at_col(ls, c1)..self.t.pos_at_col(ls, c2)].to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Replace each line's part with `put(i)` (i: the line's place in the
    /// block), a short line padded out to the block's left edge first. One
    /// undo step. The block is then a column wide nothing, after what went in.
    fn blk_put(&mut self, put: &dyn Fn(usize) -> String) {
        let Some(b) = self.blk else { return };
        let ((l1, l2), (c1, c2)) = (b.lines(), b.cols());
        let mut lines: Vec<String> = self.t.text.split('\n').map(String::from).collect();
        let mut after = c1;
        for (i, l) in (l1..=l2).enumerate() {
            let Some(line) = lines.get_mut(l) else { break };
            let chars: Vec<char> = line.chars().collect();
            let mut head: String = chars.iter().take(c1).collect();
            let pad = c1.saturating_sub(chars.len());
            let ins = put(i);
            if ins.is_empty() && pad > 0 {
                continue; // (Nothing to remove there, nothing to add.)
            }
            head.push_str(&" ".repeat(pad));
            let tail: String = chars.iter().skip(c2.max(c1)).collect();
            after = c1 + ins.chars().count();
            *line = format!("{head}{ins}{tail}");
        }
        let all = lines.join("\n");
        let n = self.t.len();
        self.t.replace(0, n, &all);
        if let Some(b) = self.blk.as_mut() {
            b.ac = after;
            b.cc = after;
        }
        let (l, c) = (b.cl, after);
        self.blk_to(l, c);
        self.changed();
    }

    /// Delete the parts; a block with no width: the character before
    /// (Backspace) or at (Delete) its column on each line.
    fn blk_delete(&mut self, back: bool) {
        let Some(b) = self.blk else { return };
        let (c1, c2) = b.cols();
        if c2 > c1 {
            self.blk_put(&|_| String::new());
            return;
        }
        let nb = if back {
            if c1 == 0 {
                return;
            }
            Blk { ac: c1 - 1, cc: c1, ..b }
        } else {
            Blk { ac: c1, cc: c1 + 1, ..b }
        };
        self.blk = Some(nb);
        self.blk_put(&|_| String::new());
    }

    /// A key while there's a rectangle, or Ctrl+Shift+Alt+arrows to make or
    /// grow one. True when it was the block's.
    pub fn blk_key(&mut self, k: &KeyEvent) -> bool {
        let m = k.modifiers;
        let (ctrl, alt, shift) = (m.contains(KeyModifiers::CONTROL), m.contains(KeyModifiers::ALT), m.contains(KeyModifiers::SHIFT));
        let arrow = matches!(k.code, KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right);
        if ctrl && alt && shift && arrow {
            if self.blk.is_none() {
                self.blk_start(self.t.cursor);
            }
            let b = self.blk.unwrap();
            let (l, c) = match k.code {
                KeyCode::Up => (b.cl.saturating_sub(1), b.cc),
                KeyCode::Down => (b.cl + 1, b.cc),
                KeyCode::Left => (b.cl, b.cc.saturating_sub(1)),
                _ => (b.cl, b.cc + 1),
            };
            self.blk_to(l, c);
            return true;
        }
        if self.blk.is_none() {
            return false;
        }
        match k.code {
            KeyCode::Esc => {
                self.blk = None;
                true
            }
            KeyCode::Char('c') | KeyCode::Char('C') if ctrl && !alt => {
                super::term::Term::copy(&self.blk_text());
                true
            }
            KeyCode::Char('x') | KeyCode::Char('X') if ctrl && !alt => {
                super::term::Term::copy(&self.blk_text());
                self.blk_delete(false);
                true
            }
            KeyCode::Char('v') | KeyCode::Char('V') if ctrl && !alt => {
                let Some(clip) = super::core::clipboard() else { return true };
                let clip = clip.replace("\r\n", "\n");
                let parts: Vec<String> = clip.split('\n').map(String::from).collect();
                let (l1, l2) = self.blk.unwrap().lines();
                if parts.len() == l2 - l1 + 1 {
                    self.blk_put(&|i| parts[i].clone());
                } else {
                    let one = clip.replace('\n', " ");
                    self.blk_put(&|_| one.clone());
                }
                true
            }
            KeyCode::Backspace => {
                self.blk_delete(true);
                true
            }
            KeyCode::Delete => {
                self.blk_delete(false);
                true
            }
            KeyCode::Char(c) if !ctrl && !alt => {
                let s = c.to_string();
                self.blk_put(&|_| s.clone());
                true
            }
            // (Anything else ends the block and does as it always does.)
            _ => {
                self.blk = None;
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEventKind, KeyEventState};

    fn key(code: KeyCode, m: KeyModifiers) -> KeyEvent {
        KeyEvent { code, modifiers: m, kind: KeyEventKind::Press, state: KeyEventState::NONE }
    }

    #[test]
    fn a_block_types_and_deletes_on_every_line() {
        let mut e = Ed::new();
        e.t.set("abcd\nefgh\nij");
        e.blk_start(1);
        let csa = KeyModifiers::CONTROL | KeyModifiers::SHIFT | KeyModifiers::ALT;
        assert!(e.blk_key(&key(KeyCode::Down, csa)));
        assert!(e.blk_key(&key(KeyCode::Down, csa)));
        assert!(e.blk_key(&key(KeyCode::Right, csa)));
        assert!(e.blk_key(&key(KeyCode::Right, csa)));
        assert_eq!(e.blk_text(), "bc\nfg\nj");
        assert!(e.blk_key(&key(KeyCode::Char('X'), KeyModifiers::NONE)));
        assert_eq!(e.t.text, "aXd\neXh\niX");
        assert!(e.blk_key(&key(KeyCode::Backspace, KeyModifiers::NONE)));
        assert_eq!(e.t.text, "ad\neh\ni");
        assert!(e.blk_key(&key(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(e.blk.is_none());
    }
}
