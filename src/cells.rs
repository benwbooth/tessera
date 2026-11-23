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
    /// It switched to the alternate screen: a terminal app (vim, less, htop),
    /// not a program printing lines.
    pub screen: bool,
    /// What it ran with (shell integration's TesseraExec): its user, the
    /// command line as the shell expanded it, its environment.
    pub exec: Option<Exec>,
}

/// What a command ran with, as the shell reported it just before it ran.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Exec {
    pub user: String,
    /// The command line after the shell's expansion (zsh: aliases expanded).
    pub line: String,
    /// NAME=value, as `env` lists them.
    pub env: Vec<String>,
}

impl Exec {
    /// The report's payload: base64 of user NUL line NUL env (`env -0`'s
    /// NUL-separated list, or `env`'s lines where there's no -0).
    pub fn parse(b64: &[u8]) -> Option<Exec> {
        use base64::Engine;
        let clean: Vec<u8> = b64.iter().filter(|c| !c.is_ascii_whitespace()).copied().collect();
        let raw = base64::engine::general_purpose::STANDARD.decode(clean).ok()?;
        let text = String::from_utf8_lossy(&raw);
        let mut parts = text.splitn(3, '\0');
        let user = parts.next()?.trim().to_string();
        let line = parts.next().unwrap_or("").trim_end_matches('\n').to_string();
        let rest = parts.next().unwrap_or("");
        let sep = if rest.contains('\0') { '\0' } else { '\n' };
        let mut env: Vec<String> = rest.split(sep).filter(|e| e.contains('=')).map(|e| e.to_string()).collect();
        env.sort();
        Some(Exec { user, line, env })
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({"user": self.user, "line": self.line, "env": self.env})
    }

    pub fn from_json(v: &serde_json::Value) -> Option<Exec> {
        let o = v.as_object()?;
        Some(Exec {
            user: o.get("user").and_then(|u| u.as_str()).unwrap_or("").to_string(),
            line: o.get("line").and_then(|u| u.as_str()).unwrap_or("").to_string(),
            env: o.get("env").and_then(|e| e.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default(),
        })
    }

    /// A local shell's, when its integration didn't say: its environment
    /// (exports at its last prompt, or as it started) and its user.
    pub fn of_process(pid: u32) -> Option<Exec> {
        if pid == 0 {
            return None;
        }
        // (Its exports at the last prompt, as its integration wrote them; else /proc.)
        let mut env: Vec<String> = crate::prompt::shell_env(pid)?.into_iter().map(|(k, v)| format!("{k}={v}")).collect();
        env.sort();
        let status = std::fs::read_to_string(format!("/proc/{pid}/status")).unwrap_or_default();
        let uid = status.lines().find_map(|l| l.strip_prefix("Uid:")).and_then(|l| l.split_whitespace().next()).unwrap_or("");
        let user = std::fs::read_to_string("/etc/passwd")
            .ok()
            .and_then(|p| p.lines().find_map(|l| {
                let f: Vec<&str> = l.split(':').collect();
                (f.len() > 2 && f[2] == uid).then(|| f[0].to_string())
            }))
            .unwrap_or_else(|| uid.to_string());
        Some(Exec { user, line: String::new(), env })
    }
}

/// Finds shell integration's TesseraExec reports in the output, each with
/// where it ended, so the command that starts after it gets it (even with
/// several commands in one read).
#[derive(Default)]
pub struct ExecScan {
    buf: Option<Vec<u8>>,
    /// This read's: (offset just past it, the report).
    ready: Vec<(usize, Exec)>,
    /// The last one from an earlier read not yet taken.
    carried: Option<Exec>,
}

impl ExecScan {
    pub fn feed(&mut self, data: &[u8]) {
        const PAT: &[u8] = b"\x1b]1337;TesseraExec=";
        if let Some((_, e)) = self.ready.pop() {
            self.carried = Some(e);
        }
        self.ready.clear();
        let find = |hay: &[u8], pat: &[u8]| hay.windows(pat.len()).position(|w| w == pat);
        let mut at = 0;
        loop {
            let rest = &data[at..];
            if let Some(buf) = self.buf.as_mut() {
                match rest.iter().position(|&b| b == 7 || b == 0x1b) {
                    Some(e) => {
                        buf.extend_from_slice(&rest[..e]);
                        let b = self.buf.take().unwrap();
                        if let Some(x) = Exec::parse(&b) {
                            self.ready.push((at + e, x));
                        }
                        at += e;
                        continue;
                    }
                    None => {
                        if buf.len() + rest.len() <= 4 << 20 {
                            buf.extend_from_slice(rest);
                        } else {
                            self.buf = None;
                        }
                        return;
                    }
                }
            }
            let Some(i) = find(rest, PAT) else { return };
            self.buf = Some(Vec::new());
            at += i + PAT.len();
        }
    }

    /// The report for a command starting at `pos` in this read: the last
    /// one before it.
    pub fn take_at(&mut self, pos: usize) -> Option<Exec> {
        match self.ready.iter().rposition(|(o, _)| *o <= pos) {
            Some(k) => {
                let e = self.ready.remove(k).1;
                self.ready.drain(..k);
                self.carried = None;
                Some(e)
            }
            None => self.carried.take(),
        }
    }
}

/// Output that switches to the alternate screen (after `before`, the bytes
/// already seen, in case the sequence was split).
fn enters_alt_screen(before: &[u8], data: &[u8]) -> bool {
    let tail = &before[before.len().saturating_sub(8)..];
    let mut buf = Vec::with_capacity(tail.len() + data.len());
    buf.extend_from_slice(tail);
    buf.extend_from_slice(data);
    // (And a terminal app drawn in place, not on the alternate screen
    // (claude, fzf, a REPL): it turns on bracketed paste or focus reports.)
    [b"\x1b[?1049h".as_slice(), b"\x1b[?1047h", b"\x1b[?47h", b"\x1b[?2004h", b"\x1b[?1004h"]
        .iter()
        .any(|s| buf.windows(s.len()).any(|w| w == *s))
}

const MAX_CELLS: usize = 400;
const MAX_CELL_BYTES: usize = 4 << 20;
const MAX_TOTAL_BYTES: usize = 96 << 20;

#[derive(Default)]
pub struct Cells {
    pub list: VecDeque<Cell>,
    next: u64,
    /// The id tesserad gave the next cell (T_CELL), used instead of counting.
    pub given: Option<u64>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

impl Cells {
    /// A command started.
    pub fn start(&mut self, command: String, cwd: String) -> u64 {
        self.next = match self.given.take() {
            Some(id) => id.max(self.next + 1),
            None => self.next + 1,
        };
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
            screen: false,
            exec: None,
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

    /// The command running now: the latest that isn't done (one started
    /// under another is done first).
    pub fn running(&mut self) -> Option<&mut Cell> {
        self.list.iter_mut().rev().find(|c| !c.done)
    }

    pub fn output(&mut self, data: &[u8]) {
        if let Some(c) = self.running() {
            if !c.screen && enters_alt_screen(&c.raw, data) {
                c.screen = true;
            }
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

    pub fn set_exec(&mut self, id: u64, exec: Option<Exec>) {
        if let Some(c) = self.list.iter_mut().find(|c| c.id == id) {
            c.exec = exec;
        }
    }

    /// What cell `id` ran with: {command, cwd, user, line, env} ("" if no such cell).
    pub fn exec_json(&self, id: u64) -> String {
        let Some(c) = self.get(id) else { return String::new() };
        let mut v = c.exec.as_ref().map(|e| e.to_json()).unwrap_or_else(|| json!({}));
        v["command"] = json!(c.command);
        v["cwd"] = json!(c.cwd);
        v.to_string()
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
                       "size": c.raw.len(), "truncated": c.truncated, "screen": c.screen})
            })
            .collect();
        serde_json::to_string(&cells).unwrap_or_else(|_| "[]".into())
    }
}

// ---- kept on disk: a session's cells survive a restart of the app ----------
// One JSON line per finished cell (output base64), appended as each ends.

fn store_path(session: &str) -> std::path::PathBuf {
    crate::settings::state_dir().join("cells").join(format!("{session}.jsonl"))
}

/// The pane's notebook (its cells written there, the order, what's shown
/// how), kept by the GUI next to the cells.
pub fn notebook_path(session: &str) -> std::path::PathBuf {
    crate::settings::state_dir().join("notebooks").join(format!("{session}.ipynb"))
}

fn cell_line(c: &Cell) -> String {
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD;
    json!({"id": c.id, "command": c.command, "cwd": c.cwd, "started": c.started, "duration": c.duration_ms,
           "status": c.status, "kind": c.kind, "truncated": c.truncated, "raw": b.encode(&c.raw),
           "image": c.image.as_ref().map(|i| b.encode(i)), "exec": c.exec.as_ref().map(|e| e.to_json())})
    .to_string()
}

fn line_cell(l: &str) -> Option<Cell> {
    use base64::Engine;
    let b = base64::engine::general_purpose::STANDARD;
    let v: serde_json::Value = serde_json::from_str(l).ok()?;
    let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
    Some(Cell {
        id: v["id"].as_u64()?,
        command: s("command"),
        cwd: s("cwd"),
        started: v["started"].as_u64().unwrap_or(0),
        duration_ms: v["duration"].as_u64().unwrap_or(0),
        status: v["status"].as_i64().unwrap_or(0),
        done: true,
        raw: b.decode(v["raw"].as_str().unwrap_or("")).unwrap_or_default(),
        kind: s("kind"),
        image: v["image"].as_str().and_then(|i| b.decode(i).ok()),
        truncated: v["truncated"].as_bool().unwrap_or(false),
        screen: false,
        exec: Exec::from_json(&v["exec"]),
    })
    .map(|mut c| {
        c.screen = enters_alt_screen(&[], &c.raw);
        c
    })
}

/// Kept cell `id` of a session's kept text, as {"id", "status", "screen"}
/// ("" if it isn't there).
pub fn kept_summary(text: &str, id: u64) -> String {
    text.lines()
        .rev()
        .filter(|l| l.contains(&format!("\"id\":{id},")))
        .filter_map(line_cell)
        .find(|c| c.id == id)
        .map(|c| json!({"id": c.id, "status": c.status, "screen": c.screen}).to_string())
        .unwrap_or_default()
}

/// A session's kept cells as stored (one JSON line each), for a window.
pub fn load_text(session: &str) -> String {
    std::fs::read_to_string(store_path(session)).unwrap_or_default()
}

impl Cells {
    /// Keep the finished cell `id` (appended to the session's file).
    pub fn persist(&self, session: &str, id: u64) {
        use std::io::Write;
        let Some(c) = self.get(id) else { return };
        let path = store_path(session);
        if let Some(d) = path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
            let _ = writeln!(f, "{}", cell_line(c));
        }
    }

    /// Bring back a session's kept cells (`from`: the session they were
    /// kept under, when a new shell takes over an old one's screen).
    pub fn restore(&mut self, from: &str, session: &str) -> bool {
        let src = store_path(from);
        let Ok(text) = std::fs::read_to_string(&src) else { return false };
        let mut list: VecDeque<Cell> = text.lines().filter_map(line_cell).collect();
        let total_lines = list.len();
        while list.len() > MAX_CELLS {
            list.pop_front();
        }
        let mut total: usize = list.iter().map(|c| c.raw.len()).sum();
        for c in list.iter_mut() {
            if total <= MAX_TOTAL_BYTES {
                break;
            }
            total -= c.raw.len();
            c.raw = Vec::new();
            c.truncated = true;
        }
        if list.is_empty() {
            return false;
        }
        self.next = self.next.max(list.iter().map(|c| c.id).max().unwrap_or(0));
        // (Restored cells come before any this session has made already.)
        let mine = std::mem::take(&mut self.list);
        self.list = list;
        for mut c in mine {
            self.next += 1;
            c.id = self.next;
            self.list.push_back(c);
        }
        let dst = store_path(session);
        // Rewritten when it has grown past what's kept, or moved to the new id.
        if from != session || total_lines > self.list.len() {
            let body: String = self.list.iter().filter(|c| c.done).map(|c| cell_line(c) + "\n").collect();
            if std::fs::write(&dst, body).is_ok() && from != session {
                let _ = std::fs::remove_file(&src);
            }
        }
        // (The pane's notebook, kept with the session, moves with its cells.)
        if from != session {
            let _ = std::fs::rename(notebook_path(from), notebook_path(session));
        }
        true
    }

    /// Bring back cells tesserad kept (cells.load's text): ids as they are,
    /// before any this window has made already.
    pub fn restore_text(&mut self, text: &str) -> bool {
        let mut list: VecDeque<Cell> = text.lines().filter_map(line_cell).collect();
        while list.len() > MAX_CELLS {
            list.pop_front();
        }
        if list.is_empty() {
            return false;
        }
        let top = list.iter().map(|c| c.id).max().unwrap_or(0);
        let mine = std::mem::take(&mut self.list);
        self.list = list;
        for c in mine {
            if c.id > top {
                self.list.push_back(c);
            }
        }
        self.next = self.next.max(top);
        true
    }

    /// The session ended for good: its kept cells go.
    pub fn forget(session: &str) {
        let _ = std::fs::remove_file(store_path(session));
        let _ = std::fs::remove_file(notebook_path(session));
    }
}

/// A cell's output as HTML with its colors and styles (the bytes replayed in
/// a scratch terminal `cols` wide, then formatted).
pub fn to_html(raw: &[u8], cols: u16) -> String {
    use libghostty_vt::fmt::{Format, Formatter, FormatterOptions};
    use libghostty_vt::{terminal::Options, Terminal};
    let lines = raw.iter().filter(|&&b| b == b'\n').count() as u32 + 2;
    let Ok(mut t) = Terminal::new(Options { cols: cols.max(20), rows: 4, max_scrollback: crate::snapshot::scrollback_bytes(lines.max(100)) }) else {
        return String::new();
    };
    // (A newline at the start keeps the first line clear of a prompt's leftovers.)
    t.vt_write(&strip_prompt_sp(raw));
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

/// The tessh guide (tessh/doc/guide.md) as a manual: a contents list
/// (`<nav>`) and the guide (`<main>`), headings with ids, tessh examples
/// highlighted (`pre.tessh`) and their output (`pre.output`).
pub fn tessh_manual_html() -> String {
    const GUIDE: &str = include_str!("../tessh/doc/guide.md");
    let md: String = GUIDE.lines().filter(|l| !l.starts_with("<!-- help:")).collect::<Vec<_>>().join("\n");
    crate::manual::manual_html(&md, "")
}

/// The Advanced tessh Scripting Guide (tessh/doc/advanced, the ABS Guide
/// ported to tessh) as a manual, as the tessh guide is.
pub fn tessh_advanced_manual_html() -> String {
    const FILES: &[(&str, &str)] = include!("../tessh/doc/advanced.rs");
    let md: String = FILES.iter().flat_map(|(_, t)| t.lines().chain(["", ""])).filter(|l| !l.starts_with("<!-- help:")).collect::<Vec<_>>().join("\n");
    crate::manual::manual_html(&md, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tessh_advanced_manual() {
        let m = tessh_advanced_manual_html();
        assert!(m.starts_with("<nav>") && m.contains("<main>") && m.contains("Sha-Bang"), "{}", &m[..200]);
        assert!(m.contains("<pre class='tessh'><code>") && m.contains("<pre class='output'>") && !m.contains("<!-- help:"));
    }

    #[test]
    fn tessh_manual() {
        let m = tessh_manual_html();
        assert!(m.starts_with("<nav>") && m.contains("<main>"), "{}", &m[..200]);
        assert!(m.contains("href='#threading'") && m.contains("<h2 id='threading'>"), "headings have ids");
        assert!(m.contains("<pre class='tessh'><code>") && m.contains("<span class=\"t-"), "examples are highlighted");
        assert!(m.contains("<pre class='output'>") && !m.contains("<!-- help:"));
    }

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
    fn cells_kept_on_disk() {
        let dir = std::env::temp_dir().join(format!("tessera-cells-{}", std::process::id()));
        std::env::set_var("XDG_STATE_HOME", &dir);
        let mut c = Cells::default();
        let id = c.start("echo hi".into(), "/tmp".into());
        c.output(b"hi\r\n");
        c.finish(0, "");
        c.persist("old", id);
        let mut n = Cells::default();
        assert!(n.restore("old", "new"));
        assert_eq!(n.list.len(), 1);
        assert_eq!(n.list[0].raw, b"hi\r\n");
        assert!(!store_path("old").exists() && store_path("new").exists());
        let id2 = n.start("ls".into(), "/".into());
        assert_eq!(id2, id + 1);
        Cells::forget("new");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn exec_reports() {
        use base64::Engine;
        let b = base64::engine::general_purpose::STANDARD;
        let one = b.encode(b"ben\0ls -la /tmp\0HOME=/home/ben\0PATH=/bin\0");
        let two = b.encode(b"root\0id\0A=1\nB=2\n");
        let mut s = ExecScan::default();
        let data = format!("\x1b]1337;TesseraExec={one}\x07\x1b]133;C\x07out\x1b]133;D;0\x07\x1b]1337;TesseraExec={two}\x07\x1b]133;C\x07");
        s.feed(data.as_bytes());
        let c1 = data.find("\x1b]133;C").unwrap();
        let c2 = data.rfind("\x1b]133;C").unwrap();
        let e1 = s.take_at(c1).unwrap();
        assert_eq!((e1.user.as_str(), e1.line.as_str()), ("ben", "ls -la /tmp"));
        assert_eq!(e1.env, vec!["HOME=/home/ben", "PATH=/bin"]);
        let e2 = s.take_at(c2).unwrap();
        assert_eq!((e2.user.as_str(), e2.env.clone()), ("root", vec!["A=1".to_string(), "B=2".to_string()]));
        // (Cut across reads.)
        let data = format!("\x1b]1337;TesseraExec={one}\x07");
        let (a, z) = data.split_at(20);
        s.feed(a.as_bytes());
        s.feed(z.as_bytes());
        s.feed(b"\x1b]133;C\x07");
        assert_eq!(s.take_at(0).unwrap().line, "ls -la /tmp");
        assert!(s.take_at(0).is_none());
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


pub fn decode_b64_text(b64: &[u8]) -> String {
    use base64::Engine;
    let clean: Vec<u8> = b64.iter().filter(|c| !c.is_ascii_whitespace()).copied().collect();
    base64::engine::general_purpose::STANDARD.decode(clean).map(|r| String::from_utf8_lossy(&r).trim().to_string()).unwrap_or_default()
}

/// A command's output without zsh's partial-line mark (PROMPT_SP: its
/// PROMPT_EOL_MARK, "%" or "#" in reverse video, padded to the line's end,
/// then "\r \r"), which it prints after output that didn't end in a newline
/// (Perl's `print 1`), on the same line as it.
pub fn strip_prompt_sp(raw: &[u8]) -> std::borrow::Cow<'_, [u8]> {
    static RE: std::sync::OnceLock<regex::bytes::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::bytes::Regex::new(r"(?-u)(?:\x1b\[[0-9;]*m)*[%#](?:\x1b\[[0-9;]*m)* +\r +\r").unwrap());
    re.replace_all(raw, &b""[..])
}

/// Without zsh's partial-line marker ("%" and padding) before its prompt.
pub fn strip_zsh_marker(mut text: String) -> String {
    while let Some(last) = text.trim_end_matches('\n').rsplit('\n').next().map(str::to_string) {
        let t = last.trim();
        if (t == "%" || t == "#") && text.trim_end_matches('\n').len() >= last.len() {
            let keep = text.trim_end_matches('\n').len() - last.len();
            text.truncate(keep);
        } else {
            break;
        }
    }
    text
}

/// A command running when tesserad swaps in a new build: its cell goes on
/// in the new one (moved to the background, its end recorded).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RunningCell {
    pub id: u64,
    pub command: String,
    pub cwd: String,
    pub started: u64,
    pub screen: bool,
}

impl RunningCell {
    /// What the shell `pid`'s terminal runs in the foreground, when that
    /// isn't the shell (a build from before handed nothing over). Its id is
    /// the one after the kept cells', as that build numbered it.
    #[cfg(target_os = "linux")]
    pub fn of_shell(pid: u32) -> Option<RunningCell> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // (Fields after the command's closing paren: state ppid pgrp session tty_nr tpgid.)
        let tpgid: i64 = stat.rsplit_once(')')?.1.split_whitespace().nth(5)?.parse().ok()?;
        if tpgid <= 0 || tpgid == pid as i64 {
            return None;
        }
        let argv = std::fs::read(format!("/proc/{tpgid}/cmdline")).ok()?;
        let command = argv.split(|b| *b == 0).filter(|a| !a.is_empty()).map(|a| String::from_utf8_lossy(a).into_owned()).collect::<Vec<_>>().join(" ");
        let cwd = std::fs::read_link(format!("/proc/{tpgid}/cwd")).map(|p| p.display().to_string()).unwrap_or_default();
        Some(RunningCell { id: 0, command, cwd, started: now_ms(), screen: false })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn of_shell(_pid: u32) -> Option<RunningCell> {
        None
    }
}

/// tesserad's side of the cells: it reads every byte the shell prints, so
/// it keeps each session's cells (their file) whether a window is attached
/// or not, numbering them for the window too (T_CELL).
pub struct Recorder {
    session: String,
    cells: Cells,
    pending_command: String,
    command_buf: Option<Vec<u8>>,
    /// The running command's output, and those of commands it runs under.
    cmd_out: Option<Vec<u8>>,
    cmd_outer: Vec<Vec<u8>>,
    /// The folder from the last OSC 7 (and a report cut off by a read).
    cwd: String,
    osc7_buf: Option<Vec<u8>>,
    /// The shell's pid, for its folder when OSC 7 hasn't said.
    pid: u32,
    execs: ExecScan,
}

impl Recorder {
    pub fn new(session: &str, pid: u32) -> Recorder {
        let mut cells = Cells::default();
        // (Carrying on after an upgrade, or a session taken over: the ids go
        // on from the kept ones.)
        cells.next = load_text(session).lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()?["id"].as_u64()).max().unwrap_or(0);
        Recorder { session: session.to_string(), cells, pending_command: String::new(), command_buf: None, cmd_out: None, cmd_outer: Vec::new(), cwd: String::new(), osc7_buf: None, pid, execs: ExecScan::default() }
    }

    /// Takes `from`'s cells and notebook (a session that's gone, its screen
    /// brought back in this one).
    pub fn adopt(&mut self, from: &str) {
        if from.is_empty() || from == self.session {
            return;
        }
        let mut c = Cells::default();
        if c.restore(from, &self.session) {
            self.cells.next = self.cells.next.max(c.next);
        } else {
            let _ = std::fs::rename(notebook_path(from), notebook_path(&self.session));
        }
    }

    pub fn next_id(&self) -> u64 {
        self.cells.next
    }

    /// The session its cells are kept with.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Numbers new cells from past `next` (a pane's new shell, whose cells
    /// are kept with those of the shell it took over from).
    pub fn skip_ids(&mut self, next: u64) {
        self.cells.next = self.cells.next.max(next);
    }

    /// The command running now, for the next build (an upgrade).
    pub fn running_cell(&self) -> Option<RunningCell> {
        let c = self.cells.list.iter().rev().find(|c| !c.done)?;
        Some(RunningCell { id: c.id, command: c.command.clone(), cwd: c.cwd.clone(), started: c.started, screen: c.screen })
    }

    /// The last build's running command (see running_cell) goes on here: its
    /// cell, with its id (0: the one after the kept ones).
    pub fn resume_running(&mut self, r: RunningCell) {
        if r.id > 0 {
            self.cells.given = Some(r.id);
        }
        let id = self.cells.start(r.command, r.cwd);
        if let Some(c) = self.cells.list.iter_mut().find(|c| c.id == id) {
            c.started = r.started;
            c.screen = r.screen;
        }
        // (Its output goes on to it, and its end mark is seen.)
        self.cmd_out = Some(Vec::new());
    }

    /// The command running now: (its id, command, folder, what it ran with as JSON).
    pub fn running_info(&self) -> Option<(u64, String, String, String)> {
        let c = self.cells.list.iter().rev().find(|c| !c.done)?;
        Some((c.id, c.command.clone(), c.cwd.clone(), self.cells.exec_json(c.id)))
    }

    /// Cell `id`'s state: (a terminal app, done, status).
    pub fn cell_state(&self, id: u64) -> Option<(bool, bool, i64)> {
        self.cells.get(id).map(|c| (c.screen, c.done, c.status))
    }

    /// Cell `id`'s output as text so far: (text, a terminal app (alternate
    /// screen), done, status).
    pub fn cell_text(&self, id: u64) -> Option<(String, bool, bool, i64)> {
        let c = self.cells.get(id)?;
        let text = strip_frame(&strip_zsh_marker(crate::images::plain_text(&strip_prompt_sp(&c.raw))));
        Some((text, c.screen, c.done, c.status))
    }

    fn folder(&self) -> String {
        if !self.cwd.is_empty() {
            return self.cwd.clone();
        }
        crate::procinfo::cwd(self.pid)
    }

    fn take_osc7(&mut self, payload: &[u8]) {
        let p = String::from_utf8_lossy(payload).to_string();
        self.cwd = match p.strip_prefix("file://") {
            Some(rest) => rest.find('/').map(|i| percent_decode(&rest[i..])).unwrap_or_default(),
            None => p,
        };
    }

    /// The shell's output, in order. Returns where cells start in it: (the
    /// offset of each start mark, the new cell's id).
    pub fn feed(&mut self, data: &[u8]) -> Vec<(usize, u64)> {
        const START: &[u8] = b"\x1b]133;C";
        const END: &[u8] = b"\x1b]133;D";
        const CMD: &[u8] = b"\x1b]1337;TesseraCommand=";
        const OSC7: &[u8] = b"\x1b]7;";
        let find = |hay: &[u8], pat: &[u8]| hay.windows(pat.len()).position(|w| w == pat);
        let mut starts = Vec::new();
        self.execs.feed(data);
        // (OSC 7 anywhere: the folder, as the window's terminal tracks it.)
        {
            let mut rest = data;
            loop {
                if let Some(buf) = self.osc7_buf.as_mut() {
                    match rest.iter().position(|&b| b == 7 || b == 0x1b) {
                        Some(e) => {
                            buf.extend_from_slice(&rest[..e]);
                            let p = self.osc7_buf.take().unwrap();
                            self.take_osc7(&p);
                            rest = &rest[e..];
                        }
                        None => {
                            if buf.len() < 8192 {
                                buf.extend_from_slice(rest);
                            } else {
                                self.osc7_buf = None;
                            }
                            break;
                        }
                    }
                }
                let Some(i) = find(rest, OSC7) else { break };
                let after = &rest[i + OSC7.len()..];
                match after.iter().position(|&b| b == 7 || b == 0x1b) {
                    Some(e) => {
                        let p = after[..e].to_vec();
                        self.take_osc7(&p);
                        rest = &after[e..];
                    }
                    None => {
                        self.osc7_buf = Some(after.to_vec());
                        break;
                    }
                }
            }
        }
        let base = data.as_ptr() as usize;
        let mut rest = data;
        loop {
            if let Some(buf) = self.command_buf.as_mut() {
                match rest.iter().position(|&b| b == 7 || b == 0x1b) {
                    Some(e) => {
                        buf.extend_from_slice(&rest[..e]);
                        let b64 = self.command_buf.take().unwrap();
                        self.pending_command = decode_b64_text(&b64);
                        rest = &rest[e..];
                    }
                    None => {
                        if buf.len() < 1 << 20 {
                            buf.extend_from_slice(rest);
                        } else {
                            self.command_buf = None;
                        }
                        return starts;
                    }
                }
            }
            if self.cmd_out.is_some() {
                let e = find(rest, END);
                let n = [find(rest, CMD), find(rest, START)].into_iter().flatten().min();
                if let Some(n) = n.filter(|&n| e.is_none_or(|e| n < e)) {
                    let mut buf = self.cmd_out.take().unwrap();
                    buf.extend_from_slice(&rest[..n]);
                    self.cells.output(&rest[..n]);
                    self.cmd_outer.push(buf);
                    rest = &rest[n..];
                    continue;
                }
            }
            if let Some(buf) = self.cmd_out.as_mut() {
                match find(rest, END) {
                    Some(i) => {
                        buf.extend_from_slice(&rest[..i]);
                        let raw = self.cmd_out.take().unwrap();
                        self.cells.output(&rest[..i]);
                        let after = &rest[i + END.len()..];
                        let digits: String = after.iter().skip_while(|&&c| c == b';').take_while(|c| c.is_ascii_digit()).map(|&c| c as char).collect();
                        let status: i64 = digits.parse().unwrap_or(0);
                        let image = {
                            let img = crate::images::un_onlcr(&raw);
                            (crate::images::output_is_image(&raw) && crate::images::is_image(&img)).then_some(img)
                        };
                        let text = strip_frame(&strip_zsh_marker(crate::images::plain_text(&strip_prompt_sp(&raw))));
                        let named = self.cells.list.iter().rev().find(|c| !c.done).map(|c| crate::images::kind_from_command(&c.command)).unwrap_or("");
                        let kind = if named.is_empty() { crate::images::detect_text(&text) } else { named };
                        if let Some(id) = self.cells.finish(status, kind) {
                            if let Some(c) = self.cells.list.iter_mut().find(|c| c.id == id) {
                                c.image = image;
                            }
                            self.cells.persist(&self.session, id);
                        }
                        rest = &rest[i + END.len()..];
                        if let Some(outer) = self.cmd_outer.pop() {
                            self.cmd_out = Some(outer);
                        }
                    }
                    None => {
                        self.cells.output(rest);
                        // (Past 4 MB only its start is kept, but it's still
                        // followed to its end: dropped, it never ended, and a
                        // notebook card ran on forever after an ls -lR.)
                        let room = (4usize << 20).saturating_sub(buf.len());
                        buf.extend_from_slice(&rest[..rest.len().min(room)]);
                        return starts;
                    }
                }
            }
            let (c, s) = (find(rest, CMD), find(rest, START));
            if let Some(c) = c.filter(|&c| s.is_none_or(|s| c < s)) {
                let payload = &rest[c + CMD.len()..];
                match payload.iter().position(|&b| b == 7 || b == 0x1b) {
                    Some(e) => {
                        self.pending_command = decode_b64_text(&payload[..e]);
                        rest = &payload[e..];
                    }
                    None => {
                        self.command_buf = Some(payload.to_vec());
                        return starts;
                    }
                }
                continue;
            }
            match s {
                Some(s) => {
                    let after = &rest[s..];
                    let end = after.iter().position(|&b| b == 7).map(|i| i + 1).unwrap_or(after.len());
                    self.cmd_out = Some(Vec::new());
                    let cmd = std::mem::take(&mut self.pending_command);
                    let cwd = self.folder();
                    let at = rest.as_ptr() as usize - base + s;
                    let exec = self.execs.take_at(at).or_else(|| Exec::of_process(self.pid));
                    let id = self.cells.start(cmd, cwd);
                    self.cells.set_exec(id, exec);
                    starts.push((at, id));
                    rest = &rest[s + end..];
                }
                None => return starts,
            }
        }
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
