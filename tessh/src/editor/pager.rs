//! The guide, full screen: the terminal's other screen (what was there comes
//! back after), the Markdown drawn as `help::terminal` draws it, scrolled
//! with the keys and the wheel, searched with / (n and N for the next and
//! previous), [ and ] for the headings, q to go back.

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use std::io::Write;

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

struct View {
    lines: Vec<String>,
    plain: Vec<String>,
    lower: Vec<String>,
    /// Lines that start a part (a heading: drawn bold, or underlined).
    heads: Vec<usize>,
    top: usize,
    query: String,
    typing: Option<String>,
    note: String,
}

impl View {
    fn new(md: &str) -> View {
        let text = crate::help::terminal(md, true);
        let lines: Vec<String> = text.lines().map(String::from).collect();
        let plain: Vec<String> = lines.iter().map(|l| strip(l)).collect();
        let lower = plain.iter().map(|l| l.to_lowercase()).collect();
        // (A heading: a bold line after a blank one, or one over a rule.)
        let heads = (0..lines.len())
            .filter(|&i| {
                let p = plain[i].trim();
                !p.is_empty() && (lines[i].starts_with("\x1b[1") || plain.get(i + 1).is_some_and(|n| n.trim_start().starts_with('─') || n.trim_start().starts_with('═'))) && (i == 0 || plain[i - 1].trim().is_empty())
            })
            .collect();
        View { lines, plain, lower, heads, top: 0, query: String::new(), typing: None, note: String::new() }
    }

    fn find(&mut self, from: usize, back: bool) {
        let q = self.query.to_lowercase();
        if q.is_empty() {
            return;
        }
        let n = self.lines.len();
        for k in 1..=n {
            let i = if back { (from + n - k) % n } else { (from + k) % n };
            if self.lower[i].contains(&q) {
                self.top = i;
                self.note.clear();
                return;
            }
        }
        self.note = format!("Not found: {}", self.query);
    }

    fn draw(&self, cols: usize, rows: usize) {
        let mut o = std::io::stdout().lock();
        let h = rows.saturating_sub(1);
        let _ = write!(o, "\x1b[?25l\x1b[H");
        let q = self.query.to_lowercase();
        for r in 0..h {
            let _ = write!(o, "\x1b[{};1H\x1b[0m\x1b[2K", r + 1);
            let Some(l) = self.lines.get(self.top + r) else { continue };
            let i = self.top + r;
            if !q.is_empty() && self.lower[i].contains(&q) {
                // (A line with a match: plain, the matches reversed.)
                let p = &self.plain[i];
                let lp = &self.lower[i];
                let mut at = 0;
                let mut out = String::new();
                while let Some(k) = lp[at..].find(&q) {
                    let s = at + k;
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
        let pct = ((self.top + h).min(total) * 100) / total;
        let status = match &self.typing {
            Some(t) => format!("/{t}"),
            None if !self.note.is_empty() => self.note.clone(),
            None => format!(" tessh guide  {pct}%   / search  n N next  [ ] parts  ↑↓ PgUp PgDn  q back"),
        };
        let status: String = status.chars().take(cols).collect();
        let _ = write!(o, "\x1b[{};1H\x1b[0m\x1b[2K\x1b[7m{status}{}\x1b[0m", rows, " ".repeat(cols.saturating_sub(unicode_width::UnicodeWidthStr::width(status.as_str()))));
        if self.typing.is_some() {
            let _ = write!(o, "\x1b[{};{}H\x1b[?25h", rows, status.chars().count() + 1);
        }
        let _ = o.flush();
    }
}

/// Show the guide (or any Markdown) full screen, at the first place `at`
/// is found, until q.
pub fn show(md: &str, at: &str) {
    let mut o = std::io::stdout();
    let _ = crossterm::terminal::enable_raw_mode();
    let _ = write!(o, "\x1b[?1049h\x1b[?1000h\x1b[?1006h\x1b[H\x1b[2J");
    let _ = o.flush();
    let mut v = View::new(md);
    if !at.trim().is_empty() {
        v.query = at.trim().to_string();
        // (A heading with the word first, else anywhere.)
        let q = v.query.to_lowercase();
        if let Some(&i) = v.heads.iter().find(|&&i| v.lower[i].contains(&q)) {
            v.top = i;
        } else {
            let n = v.lines.len();
            v.find(n.saturating_sub(1), false);
        }
    }
    loop {
        let (c, r) = crossterm::terminal::size().unwrap_or((80, 24));
        let (cols, rows) = (c as usize, r as usize);
        let page = rows.saturating_sub(2).max(1);
        let max_top = v.lines.len().saturating_sub(rows.saturating_sub(1));
        v.top = v.top.min(max_top);
        v.draw(cols, rows);
        let Ok(ev) = event::read() else { break };
        match ev {
            Event::Key(k) if k.kind != event::KeyEventKind::Release => {
                if let Some(t) = v.typing.as_mut() {
                    match k.code {
                        KeyCode::Enter => {
                            v.query = v.typing.take().unwrap();
                            let n = v.lines.len();
                            let from = if v.top == 0 { n.saturating_sub(1) } else { v.top - 1 };
                            v.find(from, false);
                        }
                        KeyCode::Esc => v.typing = None,
                        KeyCode::Backspace => {
                            if t.pop().is_none() {
                                v.typing = None;
                            }
                        }
                        KeyCode::Char(ch) if !k.modifiers.contains(KeyModifiers::CONTROL) => t.push(ch),
                        _ => {}
                    }
                    continue;
                }
                v.note.clear();
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if ctrl => break,
                    KeyCode::Down | KeyCode::Char('j') | KeyCode::Enter => v.top += 1,
                    KeyCode::Up | KeyCode::Char('k') => v.top = v.top.saturating_sub(1),
                    KeyCode::PageDown | KeyCode::Char(' ') | KeyCode::Char('f') => v.top += page,
                    KeyCode::Char('d') => v.top += page / 2,
                    KeyCode::PageUp | KeyCode::Char('b') => v.top = v.top.saturating_sub(page),
                    KeyCode::Char('u') => v.top = v.top.saturating_sub(page / 2),
                    KeyCode::Home | KeyCode::Char('g') => v.top = 0,
                    KeyCode::End | KeyCode::Char('G') => v.top = max_top,
                    KeyCode::Char('/') => v.typing = Some(String::new()),
                    KeyCode::Char('n') => {
                        let t = v.top;
                        v.find(t, false);
                    }
                    KeyCode::Char('N') => {
                        let t = v.top;
                        v.find(t, true);
                    }
                    KeyCode::Char(']') => {
                        if let Some(&i) = v.heads.iter().find(|&&i| i > v.top) {
                            v.top = i;
                        }
                    }
                    KeyCode::Char('[') => {
                        if let Some(&i) = v.heads.iter().rev().find(|&&i| i < v.top) {
                            v.top = i;
                        }
                    }
                    _ => {}
                }
            }
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollDown => v.top += 3,
                MouseEventKind::ScrollUp => v.top = v.top.saturating_sub(3),
                _ => {}
            },
            _ => {}
        }
    }
    let _ = write!(o, "\x1b[?1006l\x1b[?1000l\x1b[?1049l\x1b[?25h");
    let _ = o.flush();
    let _ = crossterm::terminal::disable_raw_mode();
}
