//! Notebook cells: each command (between shell integration's C and D marks)
//! with its command line, folder, time, exit status and output, kept so the
//! notebook view can show each one's output in its own pane.

use serde_json::json;
use std::collections::VecDeque;

pub struct Cell {
    pub id: u64,
    pub command: String,
    pub cwd: String,
    pub started: u64, // unix ms
    pub duration_ms: u64,
    pub status: i64,
    pub done: bool,
    /// The output as the terminal got it (escape sequences and all).
    pub raw: Vec<u8>,
    /// What the output is: json, csv, html, ... ("" plain text).
    pub kind: String,
    /// The image file the command printed (shown as the picture).
    pub image: Option<Vec<u8>>,
    pub truncated: bool,
}

const MAX_CELLS: usize = 400;
const MAX_CELL_BYTES: usize = 4 << 20;
const MAX_TOTAL_BYTES: usize = 96 << 20;

#[derive(Default)]
pub struct Cells {
    pub list: VecDeque<Cell>,
    next: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Cells {
    /// A command started.
    pub fn start(&mut self, command: String, cwd: String) -> u64 {
        self.next += 1;
        self.list.push_back(Cell {
            id: self.next,
            command,
            cwd,
            started: now_ms(),
            duration_ms: 0,
            status: 0,
            done: false,
            raw: Vec::new(),
            kind: String::new(),
            image: None,
            truncated: false,
        });
        while self.list.len() > MAX_CELLS {
            self.list.pop_front();
        }
        // Old outputs go first when there's too much.
        let mut total: usize = self.list.iter().map(|c| c.raw.len()).sum();
        for c in self.list.iter_mut() {
            if total <= MAX_TOTAL_BYTES {
                break;
            }
            total -= c.raw.len();
            c.raw = Vec::new();
            c.truncated = true;
        }
        self.next
    }

    pub fn running(&mut self) -> Option<&mut Cell> {
        self.list.back_mut().filter(|c| !c.done)
    }

    pub fn output(&mut self, data: &[u8]) {
        if let Some(c) = self.running() {
            if c.raw.len() + data.len() <= MAX_CELL_BYTES {
                c.raw.extend_from_slice(data);
            } else {
                c.truncated = true;
            }
        }
    }

    /// The command ended: returns the cell's id.
    pub fn finish(&mut self, status: i64, kind: &str) -> Option<u64> {
        let c = self.running()?;
        c.done = true;
        c.status = status;
        c.duration_ms = now_ms().saturating_sub(c.started);
        c.kind = kind.to_string();
        Some(c.id)
    }

    pub fn get(&self, id: u64) -> Option<&Cell> {
        self.list.iter().find(|c| c.id == id)
    }

    /// Every cell's facts (not its output), oldest first.
    pub fn summary(&self) -> String {
        let cells: Vec<serde_json::Value> = self
            .list
            .iter()
            .map(|c| {
                json!({"id": c.id, "command": c.command, "cwd": c.cwd, "started": c.started, "duration": c.duration_ms,
                       "status": c.status, "done": c.done, "kind": if c.image.is_some() { "image" } else { &c.kind },
                       "size": c.raw.len(), "truncated": c.truncated})
            })
            .collect();
        serde_json::to_string(&cells).unwrap_or_else(|_| "[]".into())
    }
}

/// A cell's output as HTML with its colors and styles (the bytes replayed in
/// a scratch terminal `cols` wide, then formatted).
pub fn to_html(raw: &[u8], cols: u16) -> String {
    use libghostty_vt::fmt::{Format, Formatter, FormatterOptions};
    use libghostty_vt::{terminal::Options, Terminal};
    let lines = raw.iter().filter(|&&b| b == b'\n').count() as u32 + 2;
    let Ok(mut t) = Terminal::new(Options { cols: cols.max(20), rows: 4, max_scrollback: crate::session::scrollback_bytes(lines.max(100)) }) else {
        return String::new();
    };
    // (A newline at the start keeps the first line clear of a prompt's leftovers.)
    t.vt_write(raw);
    let opts = FormatterOptions::new().with_format(Format::Html).with_trim(true).with_unwrap(true).with_style(true);
    let Ok(mut f) = Formatter::new(&t, opts) else { return String::new() };
    let html = f.format_alloc(None).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    for_qt(&html)
}

/// Qt's rich text has no inline divs: styled runs become spans (the outer
/// block stays a div). Palette colors (var(--vt-palette-N)) are left for
/// the view to fill in from the theme.
fn for_qt(html: &str) -> String {
    let mut out = html.replace("<div style=\"display: inline;", "<span style=\"");
    // Every </div> but the last closes a span now.
    let n = out.matches("</div>").count();
    if n > 1 {
        out = out.replacen("</div>", "</span>", n - 1);
    }
    out
}

/// Takes off the frame a pager like bat draws around a file (rules of ─┬┴,
/// a "File:" header, line numbers and a │ before each line; its own wrapped
/// lines joined back up), leaving the file. Other text comes back as it was.
pub fn strip_frame(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let is_rule = |l: &str| {
        let t = l.trim();
        !t.is_empty() && t.chars().all(|c| matches!(c, '─' | '┬' | '┼' | '┴' | '━' | '┯' | '┷'))
    };
    // "  12 │ text", or "     │ more" (a wrapped line), or "     │ File: x".
    let gutter = |l: &str| -> Option<(bool, String)> {
        let (head, rest) = l.split_once('│')?;
        let h = head.trim();
        if !(h.is_empty() || h.chars().all(|c| c.is_ascii_digit())) {
            return None;
        }
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        Some((!h.is_empty(), rest.to_string()))
    };
    let rules = lines.iter().filter(|l| is_rule(l)).count();
    // (Mostly framed: lines outside it, like an error for a missing file, go.)
    let others: Vec<&&str> = lines.iter().filter(|l| !is_rule(l) && !l.trim().is_empty()).collect();
    let in_frame = others.iter().filter(|l| gutter(l).is_some()).count();
    if rules < 2 || in_frame < 2 || in_frame * 10 < others.len() * 7 {
        return text.to_string();
    }
    let numbered = lines.iter().any(|l| gutter(l).is_some_and(|g| g.0));
    let mut out: Vec<String> = Vec::new();
    let mut after_rule = false;
    let mut joinable = false;
    for l in &lines {
        if is_rule(l) {
            after_rule = true;
            joinable = false;
            continue;
        }
        if l.trim().is_empty() {
            continue;
        }
        let Some((num, body)) = gutter(l) else {
            joinable = false;
            continue;
        };
        // (Each file's header, between two rules.)
        if after_rule && !num && (body.starts_with("File:") || body.starts_with("STDIN") || body.starts_with("Size:")) {
            continue;
        }
        after_rule = false;
        // (Numbered: a line without a number continues the one before.)
        if numbered && !num && joinable {
            out.last_mut().unwrap().push_str(&body);
        } else {
            out.push(body);
            joinable = true;
        }
    }
    out.join("\n") + "\n"
}

/// Markdown as HTML (GitHub's flavor: tables, task lists, strikethrough).
pub fn markdown_html(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_FOOTNOTES;
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(md, opts));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bat_frame() {
        let framed = "───────┬────────\n       │ File: README.md\n───────┼────────\n   1   │ # Title\n   2   │ some long li\n       │ ne wrapped\n   3   │ \n   4   │ - item\n───────┴────────\n";
        assert_eq!(strip_frame(framed), "# Title\nsome long line wrapped\n\n- item\n");
        assert_eq!(strip_frame("plain | text\n"), "plain | text\n");
        let html = markdown_html(&strip_frame(framed));
        assert!(html.contains("<h1>Title</h1>") && html.contains("<li>item</li>"), "{html}");
    }

    #[test]
    fn bat_frame_with_an_error() {
        let t = "[bat error]: 'cat': No such file or directory (os error 2)\n\u{2500}\u{2500}\u{2500}\u{2500}\u{252c}\u{2500}\u{2500}\n    \u{2502} File: README.md\n\u{2500}\u{2500}\u{2500}\u{2500}\u{253c}\u{2500}\u{2500}\n   1 \u{2502} # Kog\n   2 \u{2502} \n   3 \u{2502} Some text\n\u{2500}\u{2500}\u{2500}\u{2500}\u{2534}\u{2500}\u{2500}\n";
        assert_eq!(strip_frame(t), "# Kog\n\nSome text\n");
    }

    #[test]
    fn cells_and_html() {
        let mut c = Cells::default();
        let id = c.start("ls".into(), "/tmp".into());
        c.output(b"\x1b[31mred\x1b[0m plain\r\n");
        assert_eq!(c.finish(0, ""), Some(id));
        let s: serde_json::Value = serde_json::from_str(&c.summary()).unwrap();
        assert_eq!(s[0]["command"], "ls");
        let html = to_html(&c.get(id).unwrap().raw, 40);
        assert!(html.contains("red") && html.contains("plain"), "{html}");
        assert!(html.contains("color"), "colors kept: {html}");
        assert!(html.contains("<span"), "spans for Qt: {html}");
        assert_eq!(html.matches("</div>").count(), 1);
    }
}

