//! Command history for the input editor: Tessera's own (kept in the state
//! folder, one JSON line per command, shared by every window) merged with the
//! shells' history files (zsh, bash, fish), newest last, each command once.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Entry {
    pub cmd: String,
    /// Unix seconds (0: unknown).
    #[serde(default)]
    pub t: u64,
    #[serde(default)]
    pub cwd: String,
    /// The editor's language it was run in ("" for a shell's history file,
    /// or before this was kept: the editor's language now).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lang: String,
}

fn own_file() -> PathBuf {
    crate::settings::state_dir().join("history.jsonl")
}

fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Record a command run from the editor.
pub fn add(cmd: &str, cwd: &str, lang: &str) {
    let cmd = cmd.trim_end();
    if cmd.trim().is_empty() {
        return;
    }
    let _ = std::fs::create_dir_all(crate::settings::state_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(own_file()) {
        let line = serde_json::to_string(&Entry { cmd: cmd.to_string(), t: now(), cwd: cwd.to_string(), lang: lang.to_string() }).unwrap_or_default();
        let _ = writeln!(f, "{line}");
    }
}

/// zsh: ": 1700000000:0;command" (extended) or plain lines; a line ending in
/// a backslash continues.
pub fn parse_zsh(text: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut cur: Option<Entry> = None;
    for line in text.lines() {
        if let Some(e) = cur.as_mut() {
            e.cmd.push('\n');
            e.cmd.push_str(line.strip_suffix('\\').unwrap_or(line));
            if !line.ends_with('\\') {
                out.push(cur.take().unwrap());
            }
            continue;
        }
        let (t, cmd) = match line.strip_prefix(": ").and_then(|r| r.split_once(';')) {
            Some((meta, cmd)) => (meta.split(':').next().and_then(|s| s.trim().parse().ok()).unwrap_or(0), cmd),
            None => (0, line),
        };
        let e = Entry { cmd: cmd.strip_suffix('\\').unwrap_or(cmd).to_string(), t, cwd: String::new(), lang: String::new() };
        if cmd.ends_with('\\') {
            cur = Some(e);
        } else {
            out.push(e);
        }
    }
    out.extend(cur);
    out
}

/// bash: plain lines, each optionally after a "#1700000000" timestamp line.
pub fn parse_bash(text: &str) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut t = 0;
    for line in text.lines() {
        if let Some(ts) = line.strip_prefix('#').and_then(|s| s.trim().parse::<u64>().ok()) {
            t = ts;
            continue;
        }
        out.push(Entry { cmd: line.to_string(), t, cwd: String::new(), lang: String::new() });
        t = 0;
    }
    out
}

/// fish: "- cmd: ...\n  when: 1700000000".
pub fn parse_fish(text: &str) -> Vec<Entry> {
    let mut out: Vec<Entry> = Vec::new();
    for line in text.lines() {
        if let Some(c) = line.strip_prefix("- cmd: ") {
            out.push(Entry { cmd: c.replace("\\n", "\n").replace("\\\\", "\\"), t: 0, cwd: String::new(), lang: String::new() });
        } else if let Some(w) = line.trim().strip_prefix("when: ") {
            if let Some(e) = out.last_mut() {
                e.t = w.trim().parse().unwrap_or(0);
            }
        }
    }
    out
}

fn read_lossy(p: &PathBuf) -> Option<String> {
    std::fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Every command, oldest first, each once (at its latest use).
pub fn all() -> Vec<Entry> {
    let mut lists: Vec<Vec<Entry>> = Vec::new();
    let h = home();
    let zsh = std::env::var_os("HISTFILE").map(PathBuf::from).filter(|p| p.to_string_lossy().contains("zsh")).unwrap_or_else(|| h.join(".zsh_history"));
    for (path, parse) in [
        (zsh, parse_zsh as fn(&str) -> Vec<Entry>),
        (h.join(".bash_history"), parse_bash),
        (h.join(".local/share/fish/fish_history"), parse_fish),
    ] {
        if let Some(text) = read_lossy(&path) {
            lists.push(parse(&text));
        }
    }
    // (tessh's own file, from before it wrote here too: its lines, a
    // command's line breaks as <\n>.)
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    if let Some(text) = read_lossy(&data.join("tessh/history")) {
        lists.push(text.lines().filter(|l| !l.trim().is_empty()).map(|l| Entry { cmd: l.replace("<\\n>", "\n"), t: 0, cwd: String::new(), lang: "tessh".into() }).collect());
    }
    if let Some(text) = read_lossy(&own_file()) {
        lists.push(text.lines().filter_map(|l| serde_json::from_str::<Entry>(l).ok()).collect());
    }
    merge(lists)
}

/// Merge histories: by time where known (an entry without one keeps its
/// place after the timed entry before it), then each command once, at its
/// latest position.
pub fn merge(lists: Vec<Vec<Entry>>) -> Vec<Entry> {
    let mut all: Vec<(u64, usize, Entry)> = Vec::new();
    let mut seq = 0usize;
    for list in lists {
        let mut last_t = 0;
        for mut e in list {
            if e.t == 0 {
                e.t = last_t;
            } else {
                last_t = e.t;
            }
            let t = e.t;
            all.push((t, seq, e));
            seq += 1;
        }
    }
    all.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<Entry> = Vec::new();
    for (_, _, e) in all.into_iter().rev() {
        let key = e.cmd.trim().to_string();
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(e);
    }
    out.reverse();
    let n = out.len();
    if n > 50_000 {
        out.drain(..n - 50_000);
    }
    out
}

/// For the history dropdown: matches for `query`, newest first, as
/// completion items ({label, insert, detail, kind}).
pub fn search(query: &str, default_lang: &str) -> String {
    let q = query.trim().to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();
    let now = now();
    let items: Vec<serde_json::Value> = all()
        .into_iter()
        .rev()
        .filter(|e| {
            let c = e.cmd.to_lowercase();
            words.iter().all(|w| c.contains(w))
        })
        .take(300)
        .map(|e| {
            let label = e.cmd.replace('\n', " \u{21b5} ");
            let lang = if e.lang.is_empty() { default_lang } else { e.lang.as_str() };
            json!({"label": label, "insert": e.cmd, "detail": when(now, e.t, &e.cwd), "kind": "history", "spans": label_spans(&e.cmd, lang)})
        })
        .collect();
    json!({"start": 0, "end": 0, "items": items, "pending": false, "history": true}).to_string()
}

/// The command's syntax colors ([[start, length, kind]], UTF-16) for its
/// one-line label, where each line break is " ↵ ".
fn label_spans(cmd: &str, lang: &str) -> serde_json::Value {
    let a: serde_json::Value = serde_json::from_str(&crate::syntax::analyze(lang, cmd)).unwrap_or_default();
    // (UTF-16 offset in the command -> offset in the label.)
    let mut at = Vec::with_capacity(cmd.len() + 1);
    let mut l = 0u64;
    for c in cmd.chars() {
        for _ in 0..c.len_utf16() {
            at.push(l);
        }
        l += if c == '\n' { 3 } else { c.len_utf16() as u64 };
    }
    at.push(l);
    let map = |o: u64| at.get(o as usize).copied().unwrap_or(l);
    let spans: Vec<serde_json::Value> = a["spans"]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(|s| {
                    let (st, len, kind) = (s[0].as_u64()?, s[1].as_u64()?, s[2].as_str()?);
                    let (b, e) = (map(st), map(st + len));
                    Some(json!([b, e.saturating_sub(b), kind]))
                })
                .collect()
        })
        .unwrap_or_default();
    serde_json::Value::Array(spans)
}

fn when(now: u64, t: u64, cwd: &str) -> String {
    let ago = if t == 0 || t > now {
        String::new()
    } else {
        let d = now - t;
        match d {
            0..=59 => "just now".into(),
            60..=3599 => format!("{} min ago", d / 60),
            3600..=86_399 => format!("{} h ago", d / 3600),
            _ => format!("{} days ago", d / 86_400),
        }
    };
    let home = home().display().to_string();
    let cwd = if !home.is_empty() && cwd.starts_with(&home) { format!("~{}", &cwd[home.len()..]) } else { cwd.to_string() };
    match (ago.is_empty(), cwd.is_empty()) {
        (false, false) => format!("{ago} \u{b7} {cwd}"),
        (false, true) => ago,
        (true, false) => cwd,
        _ => String::new(),
    }
}

/// Every command (newest last) for Up/Down in the editor, as a JSON array.
pub fn all_json(limit: usize) -> String {
    let a = all();
    let start = a.len().saturating_sub(limit);
    serde_json::to_string(&a[start..].iter().map(|e| &e.cmd).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        let z = parse_zsh(": 100:0;ls -la\n: 200:0;for x in a; do\\\necho $x\\\ndone\n");
        assert_eq!(z[0], Entry { cmd: "ls -la".into(), t: 100, cwd: String::new(), lang: String::new() });
        assert_eq!(z[1].cmd, "for x in a; do\necho $x\ndone");
        let b = parse_bash("#150\ngit status\nmake\n");
        assert_eq!((b[0].t, b[1].t), (150, 0));
        let f = parse_fish("- cmd: cargo build\n  when: 300\n");
        assert_eq!(f[0].t, 300);
    }

    #[test]
    fn merged_once_in_time_order() {
        let a = vec![Entry { cmd: "ls".into(), t: 100, cwd: String::new(), lang: String::new() }, Entry { cmd: "make".into(), t: 300, cwd: String::new(), lang: String::new() }];
        let b = vec![Entry { cmd: "git".into(), t: 200, cwd: String::new(), lang: String::new() }, Entry { cmd: "ls".into(), t: 400, cwd: String::new(), lang: String::new() }];
        let m: Vec<String> = merge(vec![a, b]).into_iter().map(|e| e.cmd).collect();
        assert_eq!(m, ["git", "make", "ls"]);
    }
}
