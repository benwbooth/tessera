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

#[cfg(test)]
mod tests {
    use super::*;

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
