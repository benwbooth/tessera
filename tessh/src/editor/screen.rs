//! Drawing the editor inline, at the bottom like a shell prompt: its rows
//! are [top, top + height) on the screen; more lines grow it downward (the
//! screen scrolls when it reaches the bottom, as any prompt's would).
//!
//! A popup (completion, a list, a tooltip, help) never moves the editor: it
//! goes under it when the screen has the room there, else over it, drawn on
//! top of the output above. When it goes, the cells it covered are blanked
//! (what the output had there can't be asked of the terminal), the rest of
//! those lines left as they were. Only rows that changed are written.

use super::{Col, Hit, Line, Sty};
use std::io::Write;
use unicode_width::UnicodeWidthStr;

pub struct Overlay {
    pub lines: Vec<Line>,
    /// Its left column.
    pub col: usize,
    /// The cursor in it (a search field, vi's : line), (row, column).
    pub cursor: Option<(usize, usize)>,
}

pub struct Frame {
    pub rows: Vec<Line>,
    /// The cursor: (row, column) in rows; None hides it.
    pub cursor: Option<(usize, usize)>,
    /// Paint the rows' empty ends with this (the editor's panel tint).
    pub fill: Option<Col>,
    pub overlay: Option<Overlay>,
}

pub struct Screen {
    pub top: u16,
    height: u16,
    last: Vec<String>,
    hits: Vec<Vec<Hit>>,
    /// The popup as drawn: (screen row, column, width, encoded) per row.
    pop: Vec<(u16, usize, usize, String)>,
    pop_hits: Vec<(u16, Vec<Hit>)>,
    cols: u16,
    rows: u16,
    /// Where the cursor was left (None: hidden), so an unchanged frame
    /// writes nothing at all.
    cursor_at: Option<(u16, u16)>,
}

pub fn sgr(s: &Sty) -> String {
    let mut p: Vec<String> = vec!["0".into()];
    let col = |c: Col, fg: bool| match c {
        Col::Ansi(n) if n < 8 => format!("{}", if fg { 30 } else { 40 } + n as u32),
        Col::Ansi(n) => format!("{}", if fg { 90 } else { 100 } + (n as u32 & 7)),
        Col::Rgb(r, g, b) => format!("{};2;{r};{g};{b}", if fg { 38 } else { 48 }),
    };
    if s.bold {
        p.push("1".into());
    }
    if s.dim {
        p.push("2".into());
    }
    if s.italic {
        p.push("3".into());
    }
    if let Some(c) = s.curl {
        p.push("4:3".into());
        p.push(match c {
            Col::Ansi(n) => format!("58;5;{n}"),
            Col::Rgb(r, g, b) => format!("58;2;{r};{g};{b}"),
        });
    } else if s.underline {
        p.push("4".into());
    }
    if let Some(c) = s.fg {
        p.push(col(c, true));
    }
    if let Some(c) = s.bg {
        p.push(col(c, false));
    }
    format!("\x1b[{}m", p.join(";"))
}

/// A line as the terminal gets it, cut to `cols` columns; each column's
/// meaning for the mouse; and the width used.
fn encode(l: &Line, cols: usize) -> (String, Vec<Hit>, usize) {
    let mut out = String::new();
    let mut hits = Vec::with_capacity(cols);
    let mut w = 0;
    for s in l {
        if w >= cols {
            break;
        }
        let mut piece = String::new();
        for ch in s.text.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if w + cw > cols {
                break;
            }
            piece.push(if ch == '\t' || ch == '\n' { ' ' } else { ch });
            for _ in 0..cw {
                hits.push(s.hit.clone());
            }
            w += cw;
        }
        out.push_str(&sgr(&s.sty));
        out.push_str(&piece);
    }
    (out, hits, w)
}

/// A whole row: the rest of it cleared (or the panel's color).
fn encode_row(l: &Line, cols: usize, fill: Option<Col>) -> (String, Vec<Hit>) {
    let (mut out, mut hits, _) = encode(l, cols);
    match fill {
        Some(c) => out.push_str(&sgr(&Sty { bg: Some(c), ..Default::default() })),
        None => out.push_str("\x1b[0m"),
    }
    out.push_str("\x1b[K\x1b[0m");
    hits.resize(cols, Hit::None);
    (out, hits)
}

impl Screen {
    pub fn new() -> Screen {
        Screen { top: 0, height: 0, last: vec![], hits: vec![], pop: vec![], pop_hits: vec![], cols: 80, rows: 24, cursor_at: None }
    }

    /// Start a new prompt at the cursor's row.
    pub fn begin(&mut self) {
        let (c, r) = super::term::Term::size();
        self.cols = c;
        self.rows = r;
        // (Not at a line's start, as after output with no newline: a new line first.)
        if crossterm::cursor::position().map(|(x, _)| x > 0).unwrap_or(false) {
            print!("\r\n");
        }
        self.top = super::term::Term::cursor_row();
        self.height = 0;
        self.last.clear();
        self.hits.clear();
        self.pop.clear();
        self.pop_hits.clear();
    }

    /// The terminal changed size: drawn again from the editor's top.
    pub fn resized(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        self.top = self.top.min(rows.saturating_sub(1));
        let mut o = std::io::stdout();
        let _ = write!(o, "\x1b[{};1H\x1b[J", self.top + 1);
        self.last.clear();
        self.pop.clear();
        self.pop_hits.clear();
        self.height = 0;
    }

    pub fn cols(&self) -> usize {
        self.cols as usize
    }
    pub fn rows(&self) -> usize {
        self.rows as usize
    }
    /// Rows free under the editor.
    pub fn below(&self) -> usize {
        self.rows.saturating_sub(self.top + self.height) as usize
    }

    /// What's at a screen cell (the mouse): None when it's not the editor's.
    pub fn hit(&self, col: u16, row: u16) -> Option<&Hit> {
        if let Some((_, h)) = self.pop_hits.iter().find(|(r, _)| *r == row) {
            if let Some(x) = h.get(col as usize).filter(|x| **x != Hit::None) {
                return Some(x);
            }
        }
        if row < self.top {
            return None;
        }
        self.hits.get((row - self.top) as usize).and_then(|r| r.get(col as usize))
    }
    /// Whether a screen cell is in the popup.
    pub fn in_popup(&self, col: u16, row: u16) -> bool {
        self.pop.iter().any(|(r, c, w, _)| *r == row && (col as usize) >= *c && (col as usize) < c + w)
    }

    /// Draw a frame: only the rows that changed, the popup's rows only
    /// where they changed, nothing at all when nothing did (a pointer move
    /// over the same thing). No blank-then-redraw, so nothing flickers.
    pub fn draw(&mut self, f: &Frame) {
        use std::fmt::Write as _;
        let mut b = String::new();
        let cols = self.cols as usize;
        let skip = f.rows.len().saturating_sub(self.rows as usize);
        let rows = &f.rows[skip..];
        let h = rows.len() as u16;
        let old_pop = std::mem::take(&mut self.pop);
        let mut full = false;
        if self.top + h > self.rows {
            // (Taller than the room left: the screen scrolls up, as for any
            // prompt; what was above goes into the scrollback.)
            let over = self.top + h - self.rows;
            for (r, c, w, _) in &old_pop {
                let _ = write!(b, "\x1b[{};{}H\x1b[0m{}", r + 1, c + 1, " ".repeat(*w));
            }
            let _ = write!(b, "\x1b[{};1H{}", self.rows, "\n".repeat(over as usize));
            self.top -= over;
            self.last.clear();
            full = true;
        }
        // The popup as it will be: under the editor if it fits there, else over it.
        let mut new_pop: Vec<(u16, usize, usize, String)> = vec![];
        let mut pop_hits = vec![];
        let mut cursor = f.cursor.filter(|(r, _)| *r >= skip).map(|(r, c)| (self.top + (r - skip) as u16, c));
        if let Some(ov) = &f.overlay {
            let n = ov.lines.len();
            let below = self.rows.saturating_sub(self.top + h) as usize;
            let start: i32 = if below >= n { (self.top + h) as i32 } else { self.top as i32 - n as i32 };
            for (i, l) in ov.lines.iter().enumerate() {
                let r = start + i as i32;
                if r < 0 || r >= self.rows as i32 {
                    continue;
                }
                let r = r as u16;
                let (s, hm, w) = encode(l, cols.saturating_sub(ov.col));
                let mut row = vec![Hit::None; ov.col];
                row.extend(hm);
                row.resize(cols, Hit::None);
                pop_hits.push((r, row));
                new_pop.push((r, ov.col, w, s));
            }
            if let Some((cr, cc)) = ov.cursor {
                let r = start + cr as i32;
                if r >= 0 {
                    cursor = Some((r as u16, ov.col + cc));
                }
            }
        }
        // (The old popup's rows outside the editor that the new one doesn't
        // draw the same: blanked.)
        if !full {
            for old in &old_pop {
                let (r, c, w, _) = old;
                if (*r < self.top || *r >= self.top + h) && !new_pop.contains(old) {
                    let _ = write!(b, "\x1b[{};{}H\x1b[0m{}", r + 1, c + 1, " ".repeat(*w));
                }
            }
        }
        let mut hits = Vec::with_capacity(rows.len());
        let mut encoded = Vec::with_capacity(rows.len());
        let mut redrawn: Vec<u16> = vec![];
        for (i, l) in rows.iter().enumerate() {
            let (s, hm) = encode_row(l, cols, f.fill);
            hits.push(hm);
            let r = self.top + i as u16;
            // (A row the old popup covered, and the new doesn't the same way: drawn again.)
            let uncovered = old_pop.iter().any(|o| o.0 == r && !new_pop.contains(o));
            if self.last.get(i) != Some(&s) || uncovered {
                let _ = write!(b, "\x1b[{};1H{}", r + 1, s);
                redrawn.push(r);
            }
            encoded.push(s);
        }
        if h < self.height {
            let _ = write!(b, "\x1b[{};1H\x1b[J", self.top + h + 1);
            redrawn.extend(self.top + h..self.top + self.height);
        }
        self.last = encoded;
        self.hits = hits;
        self.height = h;
        // The popup's rows: those new or changed, or under a row just drawn.
        for p in &new_pop {
            if full || !old_pop.contains(p) || redrawn.contains(&p.0) {
                let _ = write!(b, "\x1b[{};{}H{}\x1b[0m", p.0 + 1, p.1 + 1, p.3);
            }
        }
        self.pop = new_pop;
        self.pop_hits = pop_hits;
        let cursor = cursor.map(|(r, c)| (r, c as u16));
        if b.is_empty() && cursor == self.cursor_at {
            return;
        }
        let mut o = std::io::stdout().lock();
        // (One frame, synchronized output: terminals that know it show it whole.)
        let _ = write!(o, "\x1b[?2026h\x1b[?25l{b}");
        if let Some((r, c)) = cursor {
            let _ = write!(o, "\x1b[{};{}H\x1b[2 q\x1b[?25h", r + 1, c + 1);
        }
        let _ = write!(o, "\x1b[?2026l");
        self.cursor_at = cursor;
        let _ = o.flush();
    }

    /// The editor and its popup off the screen (something's printed there).
    pub fn clear(&mut self) {
        let mut o = std::io::stdout();
        for (r, c, w, _) in std::mem::take(&mut self.pop) {
            let _ = write!(o, "\x1b[{};{}H\x1b[0m{}", r + 1, c + 1, " ".repeat(w));
        }
        let _ = write!(o, "\x1b[{};1H\x1b[J", self.top + 1);
        let _ = o.flush();
        self.pop_hits.clear();
        self.height = 0;
        self.last.clear();
        self.cursor_at = None;
    }

    /// The column a run of the same hit starts at on a row (a button's left edge).
    pub fn run_start(&self, col: u16, row: u16) -> usize {
        let Some(h) = self.hit(col, row).cloned() else { return col as usize };
        let mut c = col;
        while c > 0 && self.hit(c - 1, row) == Some(&h) {
            c -= 1;
        }
        c as usize
    }

    /// Done with this prompt: the final look stays (the prompt and what was
    /// run, as a shell leaves them) and the cursor goes under it.
    pub fn finish(&mut self, f: &Frame) {
        let mut plain = Frame { rows: f.rows.clone(), cursor: None, fill: None, overlay: None };
        if plain.rows.is_empty() {
            plain.rows.push(vec![]);
        }
        self.last.clear();
        self.draw(&plain);
        let mut o = std::io::stdout();
        if self.top + self.height >= self.rows {
            // (At the bottom: a new line scrolls it up.)
            let _ = write!(o, "\x1b[{};1H\r\n\x1b[?25h", self.rows);
        } else {
            let _ = write!(o, "\x1b[{};1H\x1b[J\x1b[?25h", self.top + self.height + 1);
        }
        let _ = o.flush();
        self.height = 0;
        self.hits.clear();
        self.pop_hits.clear();
    }
}

/// Columns a string takes.
pub fn cols_of(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}
