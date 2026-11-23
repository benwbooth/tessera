//! The guide, full screen, as Tessera's guide tab has it: a search box on
//! top (searching as you type, "3 of 12"), the contents down the left (each
//! part a click away, the one being read highlighted), the guide beside it,
//! laid out for the room it has. On the terminal's other screen: what was
//! there comes back after. Keys: / or a click on the box to search, Enter /
//! n / N for the next and previous match, [ ] for the parts, the arrows,
//! PgUp PgDn, g G, the wheel; q or Esc goes back.

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use std::io::Write;
use unicode_width::UnicodeWidthStr;

fn strip(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            if it.peek() == Some(&'[') {
                it.next();
                while let Some(&n) = it.peek() {
                    it.next();
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if it.peek() == Some(&']') {
                while let Some(n) = it.next() {
                    if n == '\x07' {
                        break;
                    }
                    if n == '\x1b' {
                        it.next();
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn cut(s: &str, w: usize) -> String {
    let mut out = String::new();
    let mut n = 0;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if n + cw > w {
            break;
        }
        n += cw;
        out.push(c);
    }
    out
}

struct Part {
    level: usize,
    title: String,
    line: usize,
}

struct Guide {
    md: String,
    width: usize,
    lines: Vec<String>,
    plain: Vec<String>,
    lower: Vec<String>,
    parts: Vec<Part>,
    top: usize,
    toc_top: usize,
    query: String,
    searching: bool,
    matches: Vec<usize>,
    at: Option<usize>,
}

impl Guide {
    fn new(md: &str) -> Guide {
        let mut g = Guide { md: md.to_string(), width: 0, lines: vec![], plain: vec![], lower: vec![], parts: vec![], top: 0, toc_top: 0, query: String::new(), searching: false, matches: vec![], at: None };
        g.layout(80);
        g
    }

    /// Laid out for `width` columns: the lines, and where each part starts.
    fn layout(&mut self, width: usize) {
        if width == self.width {
            return;
        }
        let keep = self.parts.iter().rev().find(|p| p.line <= self.top).map(|p| p.title.clone());
        self.width = width;
        let text = crate::help::terminal_at(&self.md, true, width.clamp(20, 100));
        self.lines = text.lines().map(String::from).collect();
        self.plain = self.lines.iter().map(|l| strip(l)).collect();
        self.lower = self.plain.iter().map(|l| l.to_lowercase()).collect();
        // The parts: the Markdown's headings, found in the laid-out lines in order.
        self.parts.clear();
        let mut from = 0;
        let mut fence = false;
        for l in self.md.lines() {
            if l.trim_start().starts_with("```") {
                fence = !fence;
            }
            if fence || !l.starts_with('#') {
                continue;
            }
            let level = l.chars().take_while(|c| *c == '#').count();
            let title = l[level..].trim().replace('`', "");
            if title.is_empty() || level > 3 {
                continue;
            }
            if let Some(i) = (from..self.plain.len()).find(|&i| self.plain[i].trim().trim_matches(|c: char| !c.is_alphanumeric() && c != ')' && c != '(').contains(title.trim_matches(|c: char| !c.is_alphanumeric()))) {
                self.parts.push(Part { level, title, line: i });
                from = i + 1;
            }
        }
        if let Some(t) = keep {
            if let Some(p) = self.parts.iter().find(|p| p.title == t) {
                self.top = p.line;
            }
        }
        self.search();
    }

    fn search(&mut self) {
        let q = self.query.to_lowercase();
        self.matches = if q.is_empty() { vec![] } else { (0..self.lines.len()).filter(|&i| self.lower[i].contains(&q)).collect() };
        self.at = None;
    }
    /// The next (or previous) match from the top line, shown.
    fn step(&mut self, back: bool, include_top: bool) {
        if self.matches.is_empty() {
            return;
        }
        let t = self.top;
        let i = if back {
            self.matches.iter().rposition(|&m| m < t).unwrap_or(self.matches.len() - 1)
        } else {
            self.matches.iter().position(|&m| if include_top { m >= t } else { m > t }).unwrap_or(0)
        };
        self.at = Some(i);
        self.top = self.matches[i].saturating_sub(2);
    }
    fn current_part(&self) -> Option<usize> {
        self.parts.iter().rposition(|p| p.line <= self.top + 1)
    }

    fn sidebar_w(cols: usize) -> usize {
        if cols < 70 {
            0
        } else {
            (cols / 4).clamp(20, 34)
        }
    }

    fn draw(&mut self, cols: usize, rows: usize) {
        let sw = Self::sidebar_w(cols);
        let cw = cols.saturating_sub(sw + if sw > 0 { 2 } else { 0 });
        self.layout(cw.saturating_sub(1));
        let body = rows.saturating_sub(2);
        self.top = self.top.min(self.lines.len().saturating_sub(body));
        let mut o = std::io::stdout().lock();
        let _ = write!(o, "\x1b[?25l");
        // The search box.
        let count = if self.query.is_empty() {
            String::new()
        } else if self.matches.is_empty() {
            "no matches  ".into()
        } else {
            match self.at {
                Some(i) => format!("{} of {}  ", i + 1, self.matches.len()),
                None => format!("{} matches  ", self.matches.len()),
            }
        };
        let field = if self.query.is_empty() && !self.searching { "\x1b[2mSearch the guide (/)\x1b[22m".to_string() } else { self.query.clone() };
        let fw = cols.saturating_sub(6 + count.width());
        let _ = write!(o, "\x1b[1;1H\x1b[0m\x1b[2K \x1b[7m ⌕ {}{}\x1b[27m {}", cut(&field, fw), " ".repeat(fw.saturating_sub(strip(&field).width())), count);
        // The contents, and the guide beside them.
        let cur = self.current_part();
        if let Some(c) = cur {
            if c < self.toc_top {
                self.toc_top = c;
            } else if c >= self.toc_top + body {
                self.toc_top = c + 1 - body;
            }
        }
        let q = self.query.to_lowercase();
        for r in 0..body {
            let _ = write!(o, "\x1b[{};1H\x1b[0m\x1b[2K", r + 2);
            if sw > 0 {
                if let Some((i, p)) = self.parts.iter().enumerate().nth(self.toc_top + r) {
                    let ind = "  ".repeat(p.level.saturating_sub(1));
                    let t = cut(&format!(" {ind}{}", p.title), sw);
                    let pad = " ".repeat(sw.saturating_sub(t.width()));
                    if Some(i) == cur {
                        let _ = write!(o, "\x1b[7m{t}{pad}\x1b[27m");
                    } else if p.level == 1 {
                        let _ = write!(o, "\x1b[1m{t}\x1b[22m{pad}");
                    } else {
                        let _ = write!(o, "{t}{pad}");
                    }
                } else {
                    let _ = write!(o, "{}", " ".repeat(sw));
                }
                let _ = write!(o, "\x1b[2m│\x1b[22m ");
            }
            let i = self.top + r;
            let Some(l) = self.lines.get(i) else { continue };
            if !q.is_empty() && self.lower[i].contains(&q) {
                // (A line with a match: the matches reversed.)
                let (p, lp) = (&self.plain[i], &self.lower[i]);
                let mut at = 0;
                let mut out = String::new();
                while let Some(k) = lp[at..].find(&q) {
                    let s = at + k;
                    if !p.is_char_boundary(s) || !p.is_char_boundary(s + q.len()) {
                        break;
                    }
                    out.push_str(&p[at..s]);
                    out.push_str("\x1b[7m");
                    out.push_str(&p[s..s + q.len()]);
                    out.push_str("\x1b[27m");
                    at = s + q.len();
                }
                out.push_str(&p[at..]);
                let _ = write!(o, "{out}");
            } else {
                let _ = write!(o, "{l}");
            }
        }
        let total = self.lines.len().max(1);
        let pct = ((self.top + body).min(total) * 100) / total;
        let hint = format!(" {pct}%   / search   Enter n N next   [ ] parts   ↑↓ PgUp PgDn   q back");
        let _ = write!(o, "\x1b[{};1H\x1b[0m\x1b[2K\x1b[2m{}\x1b[0m", rows, cut(&hint, cols));
        if self.searching {
            let _ = write!(o, "\x1b[1;{}H\x1b[?25h", 5 + self.query.width());
        }
        let _ = o.flush();
    }
}

/// Show the guide (or any Markdown) full screen, at the part about `at`
/// when there is one, until q.
pub fn show(md: &str, at: &str) {
    let mut o = std::io::stdout();
    let _ = crossterm::terminal::enable_raw_mode();
    let _ = write!(o, "\x1b[?1049h\x1b[?1000h\x1b[?1006h\x1b[H\x1b[2J");
    let _ = o.flush();
    let mut g = Guide::new(md);
    let (c0, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let sw = Guide::sidebar_w(c0 as usize);
    g.layout((c0 as usize).saturating_sub(sw + if sw > 0 { 3 } else { 1 }));
    let at = at.trim();
    if !at.is_empty() {
        let w = at.to_lowercase();
        if let Some(p) = g.parts.iter().find(|p| p.title.to_lowercase().split(|c: char| !c.is_alphanumeric()).any(|x| x == w)) {
            g.top = p.line;
        } else {
            g.query = at.to_string();
            g.search();
            g.step(false, true);
        }
    }
    loop {
        let (c, r) = crossterm::terminal::size().unwrap_or((80, 24));
        let (cols, rows) = (c as usize, r as usize);
        g.draw(cols, rows);
        let page = rows.saturating_sub(3).max(1);
        let Ok(ev) = event::read() else { break };
        match ev {
            Event::Key(k) if k.kind != event::KeyEventKind::Release => {
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                if g.searching {
                    match k.code {
                        KeyCode::Enter => {
                            let first = g.at.is_none();
                            g.step(k.modifiers.contains(KeyModifiers::SHIFT), first);
                        }
                        KeyCode::Esc => g.searching = false,
                        KeyCode::Down => g.step(false, false),
                        KeyCode::Up => g.step(true, false),
                        KeyCode::Backspace => {
                            g.query.pop();
                            g.search();
                            g.step(false, true);
                        }
                        KeyCode::Char('u') if ctrl => {
                            g.query.clear();
                            g.search();
                        }
                        KeyCode::Char('c') if ctrl => break,
                        KeyCode::Char(ch) if !ctrl => {
                            // (As you type: the first match from where you are.)
                            g.query.push(ch);
                            g.search();
                            g.step(false, true);
                        }
                        _ => {}
                    }
                    continue;
                }
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if ctrl => break,
                    KeyCode::Down | KeyCode::Char('j') => g.top += 1,
                    KeyCode::Up | KeyCode::Char('k') => g.top = g.top.saturating_sub(1),
                    KeyCode::PageDown | KeyCode::Char(' ') | KeyCode::Char('f') => g.top += page,
                    KeyCode::Char('d') => g.top += page / 2,
                    KeyCode::PageUp | KeyCode::Char('b') => g.top = g.top.saturating_sub(page),
                    KeyCode::Char('u') => g.top = g.top.saturating_sub(page / 2),
                    KeyCode::Home | KeyCode::Char('g') => g.top = 0,
                    KeyCode::End | KeyCode::Char('G') => g.top = usize::MAX / 2,
                    KeyCode::Char('/') => {
                        g.searching = true;
                    }
                    KeyCode::Char('n') | KeyCode::Enter => g.step(false, false),
                    KeyCode::Char('N') => g.step(true, false),
                    KeyCode::Char(']') => {
                        if let Some(p) = g.parts.iter().find(|p| p.line > g.top + 1) {
                            g.top = p.line;
                        }
                    }
                    KeyCode::Char('[') => {
                        let cur = g.current_part();
                        let prev = match cur {
                            Some(i) if g.parts[i].line + 1 < g.top => Some(i),
                            Some(i) => i.checked_sub(1),
                            None => None,
                        };
                        if let Some(i) = prev {
                            g.top = g.parts[i].line;
                        }
                    }
                    _ => {}
                }
            }
            Event::Mouse(m) => {
                let sw = Guide::sidebar_w(cols);
                let over_toc = sw > 0 && (m.column as usize) < sw && m.row >= 1;
                match m.kind {
                    MouseEventKind::ScrollDown if over_toc => g.toc_top = (g.toc_top + 3).min(g.parts.len().saturating_sub(1)),
                    MouseEventKind::ScrollUp if over_toc => g.toc_top = g.toc_top.saturating_sub(3),
                    MouseEventKind::ScrollDown => g.top += 3,
                    MouseEventKind::ScrollUp => g.top = g.top.saturating_sub(3),
                    MouseEventKind::Down(MouseButton::Left) => {
                        if m.row == 0 {
                            g.searching = true;
                        } else if over_toc && (m.row as usize) <= rows.saturating_sub(2) {
                            let i = g.toc_top + m.row as usize - 1;
                            if let Some(p) = g.parts.get(i) {
                                g.top = p.line;
                                g.searching = false;
                            }
                        } else {
                            g.searching = false;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    let _ = write!(o, "\x1b[?1006l\x1b[?1000l\x1b[?1049l\x1b[?25h");
    let _ = o.flush();
    let _ = crossterm::terminal::disable_raw_mode();
}
