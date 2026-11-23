//! Drawing the editor inline, at the bottom like a shell prompt: the rows it
//! has are [top, top + height) on the screen. Taller (a popup opening above
//! the editor, more lines): when that runs past the screen's bottom, the
//! screen scrolls up first, so what was above goes into the scrollback (never
//! drawn over), and the editor stays where it was with the popup over it.
//! Shorter: the editor moves up into the room, blank rows under it, as after
//! `clear`. Only rows that changed are written.

use super::{Col, Hit, Line, Sty};
use std::io::Write;
use unicode_width::UnicodeWidthStr;

pub struct Frame {
    pub rows: Vec<Line>,
    /// The cursor: (row, column) in rows; None hides it.
    pub cursor: Option<(usize, usize)>,
    /// Paint the rows' empty ends with this (the editor's panel tint).
    pub fill: Option<Col>,
}

pub struct Screen {
    pub top: u16,
    height: u16,
    last: Vec<String>,
    hits: Vec<Vec<Hit>>,
    cols: u16,
    rows: u16,
    started: bool,
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

/// A line as the terminal gets it, cut to `cols` columns, and each column's
/// meaning for the mouse.
fn encode(l: &Line, cols: usize, fill: Option<Col>) -> (String, Vec<Hit>) {
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
    match fill {
        Some(c) => out.push_str(&sgr(&Sty { bg: Some(c), ..Default::default() })),
        None => out.push_str("\x1b[0m"),
    }
    // (The rest of the row: the fill color, or cleared.)
    out.push_str("\x1b[K\x1b[0m");
    hits.resize(cols, Hit::None);
    (out, hits)
}

impl Screen {
    pub fn new() -> Screen {
        Screen { top: 0, height: 0, last: vec![], hits: vec![], cols: 80, rows: 24, started: false }
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
        self.started = true;
    }

    /// The terminal changed size: the rows it reflowed are drawn again from
    /// the editor's top (kept on the screen).
    pub fn resized(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        self.top = self.top.min(rows.saturating_sub(1));
        let mut o = std::io::stdout();
        let _ = write!(o, "\x1b[{};1H\x1b[J", self.top + 1);
        self.last.clear();
        self.height = 0;
    }

    pub fn cols(&self) -> usize {
        self.cols as usize
    }
    pub fn rows(&self) -> usize {
        self.rows as usize
    }

    /// What's at a screen cell (the mouse): None when it's not the editor's.
    pub fn hit(&self, col: u16, row: u16) -> Option<&Hit> {
        if row < self.top {
            return None;
        }
        self.hits.get((row - self.top) as usize).and_then(|r| r.get(col as usize))
    }
    /// The screen row of a frame row.
    pub fn screen_row(&self, r: usize) -> u16 {
        self.top + r as u16
    }

    pub fn draw(&mut self, f: &Frame) {
        let mut o = std::io::stdout().lock();
        let cols = self.cols as usize;
        // (Never taller than the screen: the top rows go.)
        let skip = f.rows.len().saturating_sub(self.rows as usize);
        let rows = &f.rows[skip..];
        let h = rows.len() as u16;
        let _ = write!(o, "\x1b[?25l");
        if self.top + h > self.rows {
            // Scroll what's above up into the scrollback.
            let over = self.top + h - self.rows;
            let _ = write!(o, "\x1b[{};1H{}", self.rows, "\n".repeat(over as usize));
            self.top -= over;
            self.last.clear();
        }
        let mut hits = Vec::with_capacity(rows.len());
        for (i, l) in rows.iter().enumerate() {
            let (s, hm) = encode(l, cols, f.fill);
            hits.push(hm);
            if self.last.get(i) != Some(&s) {
                let _ = write!(o, "\x1b[{};1H{}", self.top + i as u16 + 1, s);
            }
        }
        // (Rows no longer used: cleared.)
        if h < self.height {
            let _ = write!(o, "\x1b[{};1H\x1b[J", self.top + h + 1);
        }
        self.last = rows.iter().map(|l| encode(l, cols, f.fill).0).collect();
        self.hits = hits;
        self.height = h;
        if let Some((r, c)) = f.cursor {
            if r >= skip {
                let _ = write!(o, "\x1b[{};{}H\x1b[2 q\x1b[?25h", self.top + (r - skip) as u16 + 1, c + 1);
            }
        }
        let _ = o.flush();
    }

    /// Done with this prompt: the final look stays (the prompt and what was
    /// run, as a shell leaves them) and the cursor goes under it.
    pub fn finish(&mut self, f: &Frame) {
        let mut plain = Frame { rows: f.rows.clone(), cursor: None, fill: None };
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
        self.started = false;
        self.height = 0;
        self.hits.clear();
    }
}

/// Columns a string takes.
pub fn cols_of(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}
