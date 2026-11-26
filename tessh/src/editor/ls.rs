//! `ls` at tessh's terminal prompt: Tessera's folder view (qml/FileBrowser.qml,
//! an ls card, with NotebookView.qml's lsPlan for the command's flags) drawn
//! with text. On the surface it reads as ls output: names with icons and
//! lsd's colors in columns going down then across, `ls -l` as a table, or
//! `ls -m`'s commas. Underneath, while it's the last thing above the prompt:
//! a click on a folder goes into it (back, forward and up, a path bar once
//! away from where it started), a click on a file opens it, a right click
//! has the file manager's actions, the table's headings sort (Shift+click:
//! then by that), drag to move them, drag an edge to size one, ▸ opens a
//! folder in place, and a row of buttons switches the view, folders first
//! and hidden files (kept for the next ls, as Tessera keeps them). Hovering
//! shows what Tessera's tooltips show. Every one of those by keys too:
//! Ctrl+Alt+L moves into the listing (the keys are in its ⌨ tooltip).
//!
//! It's the editor's, drawn above the prompt bar, until the next command
//! runs: then it's printed whole, as text, into the scrollback.
//! (Tessera's tiles view is the cards': not here; a listing left on tiles
//! shows as the list.)

use super::core::{Ed, MenuItem, MenuKind, Outcome};
use super::text::Text;
use super::view::Palette;
use super::{Col, Hit, Line, Span, Sty};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

/// Tessera's own: a folder's entries, and the menus' actions on them.
#[allow(dead_code)]
#[path = "../../../src/files.rs"]
pub mod files;

// ---- a folder's entries (files::list) ------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub link: bool,
    pub target: String,
    pub exec: bool,
    pub size: u64,
    pub items: i64,
    pub mtime: i64,
    pub ctime: Option<i64>,
    pub atime: Option<i64>,
    pub mode: String,
    pub user: String,
    pub group: String,
    pub uid: Option<u64>,
    pub gid: Option<u64>,
    pub ino: Option<u64>,
    pub blocks: Option<u64>,
    pub nlink: Option<u64>,
    pub kind: String,
    pub special: String,
    pub text: bool,
}

impl Entry {
    fn from(v: &Value) -> Entry {
        let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
        let b = |k: &str| v[k].as_bool().unwrap_or(false);
        Entry {
            name: s("name"),
            path: s("path"),
            dir: b("dir"),
            link: b("link"),
            target: s("target"),
            exec: b("exec"),
            size: v["size"].as_u64().unwrap_or(0),
            items: v["items"].as_i64().unwrap_or(-1),
            mtime: v["mtime"].as_i64().unwrap_or(0),
            ctime: v["ctime"].as_i64(),
            atime: v["atime"].as_i64(),
            mode: s("mode"),
            user: s("user"),
            group: s("group"),
            uid: v["uid"].as_u64(),
            gid: v["gid"].as_u64(),
            ino: v["ino"].as_u64(),
            blocks: v["blocks"].as_u64(),
            nlink: v["nlink"].as_u64(),
            kind: s("kind"),
            special: s("special"),
            text: b("text"),
        }
    }
}

pub fn read_dir(dir: &str, all: bool) -> Vec<Entry> {
    serde_json::from_str::<Vec<Value>>(&files::list(dir, all)).unwrap_or_default().iter().map(Entry::from).collect()
}

// ---- the command: shellWords and lsPlan (NotebookView.qml) -------------------------

/// A shell pattern for one name (ls *.txt, -I, --hide); * and ? don't
/// match a leading dot, as in the shell. Backup: -B's ~ at the end.
#[derive(Clone, Debug)]
pub enum Pat {
    Glob(glob::Pattern, bool),
    Backup,
}

impl Pat {
    pub fn glob(g: &str) -> Pat {
        let p = glob::Pattern::new(g).unwrap_or_else(|_| glob::Pattern::new(&glob::Pattern::escape(g)).unwrap());
        Pat::Glob(p, g.starts_with('.'))
    }
    pub fn matches(&self, name: &str) -> bool {
        match self {
            Pat::Backup => name.ends_with('~'),
            Pat::Glob(p, dot) => p.matches_with(name, glob::MatchOptions { case_sensitive: true, require_literal_separator: false, require_literal_leading_dot: !dot }),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Word {
    pub text: String,
    pub glob: bool,
}

/// A command's words as the shell reads them: quotes and backslashes
/// taken off ('7th Heaven'/ is one word); a pattern outside quotes marked.
pub fn shell_words(c: &str) -> Vec<Word> {
    let mut out = vec![];
    let (mut w, mut has, mut glob, mut q) = (String::new(), false, false, None::<char>);
    let mut it = c.chars();
    while let Some(ch) = it.next() {
        match q {
            Some('\'') => {
                if ch == '\'' {
                    q = None
                } else {
                    w.push(ch)
                }
                continue;
            }
            Some(_) => {
                if ch == '"' {
                    q = None
                } else if ch == '\\' {
                    match it.next() {
                        Some(n) => w.push(n),
                        None => w.push('\\'),
                    }
                } else {
                    w.push(ch)
                }
                continue;
            }
            None => {}
        }
        if ch == '\'' || ch == '"' {
            q = Some(ch);
            has = true;
        } else if ch == '\\' {
            match it.next() {
                Some(n) => w.push(n),
                None => w.push('\\'),
            }
            has = true;
        } else if ch.is_whitespace() {
            if has {
                out.push(Word { text: std::mem::take(&mut w), glob });
            }
            w.clear();
            has = false;
            glob = false;
        } else {
            if "*?[".contains(ch) {
                glob = true;
            }
            w.push(ch);
            has = true;
        }
    }
    if has {
        out.push(Word { text: w, glob });
    }
    out
}

/// ls's ways of showing entries (from the command).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Opts {
    pub inode: bool,
    pub blocks: bool,
    pub numeric: bool,
    pub no_owner: bool,
    pub no_group: bool,
    /// "" (mtime), "ctime", "atime".
    pub time: String,
    pub full_time: bool,
    /// No -h with -l: sizes exact.
    pub bytes: bool,
    /// "" (a / after folders), "F", "p", "none".
    pub classify: String,
    /// "", "c", "escape", "literal", "shell", "shell-always".
    pub quote: String,
    pub commas: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Group {
    pub dir: String,
    pub names: Vec<String>,
    pub patterns: Vec<Pat>,
    pub all: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Plan {
    pub all: bool,
    pub long: bool,
    pub sort: String,
    pub reverse: bool,
    pub folders_first: bool,
    pub self_only: bool,
    pub recursive: bool,
    pub ignore: Vec<Pat>,
    pub opts: Opts,
    /// One folder, its contents (ls, ls dir).
    pub single: String,
    /// Entries picked from folders (ls a b, ls *.txt, ls -d x).
    pub groups: Vec<Group>,
    pub headers: bool,
    /// -h (with -l: sizes as people read them, not exact).
    pub human: bool,
}

/// An ls's words after its name as the folder view shows them; None for
/// what it can't (--help, a pattern in a folder's name).
pub fn plan(args: &[Word], base: &str, home: &str) -> Option<Plan> {
    let mut o = Plan::default();
    let mut human = false;
    let mut opts = true;
    let mut operands: Vec<Word> = vec![];
    let mut i = 0;
    let has = |a: &str, c: char| a.contains(c);
    while i < args.len() {
        let a = args[i].text.clone();
        if opts && a == "--" {
            opts = false;
            i += 1;
            continue;
        }
        if opts && a.starts_with("--") && a.len() > 2 {
            let (k, v) = match a.find('=') {
                Some(j) if j > 0 => (a[..j].to_string(), Some(a[j + 1..].to_string())),
                _ => (a.clone(), None),
            };
            match (k.as_str(), v.as_deref()) {
                ("--help" | "--version", _) => return None,
                ("--all" | "--almost-all", _) => o.all = true,
                ("--directory", _) => o.self_only = true,
                ("--recursive", _) => o.recursive = true,
                ("--reverse", _) => o.reverse = true,
                ("--group-directories-first", _) => o.folders_first = true,
                ("--ignore-backups", _) => o.ignore.push(Pat::Backup),
                ("--ignore" | "--hide", _) => {
                    let pat = match &v {
                        Some(v) => Some(v.clone()),
                        None => {
                            i += 1;
                            args.get(i).map(|w| w.text.clone())
                        }
                    };
                    if let Some(p) = pat.filter(|p| !p.is_empty()) {
                        o.ignore.push(Pat::glob(&p));
                    }
                }
                ("--sort", Some(v)) => match v {
                    "time" => o.sort = "date".into(),
                    "size" => o.sort = "size".into(),
                    "extension" => o.sort = "type".into(),
                    "none" => o.sort = "none".into(),
                    "name" | "version" | "width" => o.sort = String::new(),
                    _ => {}
                },
                ("--format", v) => {
                    o.long = matches!(v, Some("long" | "verbose"));
                    o.opts.commas = v == Some("commas");
                }
                ("--full-time", _) => {
                    o.long = true;
                    o.opts.full_time = true;
                }
                ("--time-style", Some(v)) => o.opts.full_time = v.contains("iso") && v != "iso",
                ("--time", Some(v)) => o.opts.time = if matches!(v, "ctime" | "status") { "ctime" } else if matches!(v, "atime" | "access" | "use") { "atime" } else { "" }.into(),
                ("--inode", _) => o.opts.inode = true,
                ("--size", _) => o.opts.blocks = true,
                ("--numeric-uid-gid", _) => {
                    o.long = true;
                    o.opts.numeric = true;
                }
                ("--no-group", _) => o.opts.no_group = true,
                ("--human-readable" | "--si", _) => human = true,
                ("--classify", _) => o.opts.classify = "F".into(),
                ("--file-type", _) => o.opts.classify = "p".into(),
                ("--indicator-style", Some(v)) => o.opts.classify = if v == "classify" { "F" } else if v == "none" { "none" } else { "p" }.into(),
                ("--quote-name", _) => o.opts.quote = "c".into(),
                ("--escape", _) => o.opts.quote = "escape".into(),
                ("--literal", _) => o.opts.quote = "literal".into(),
                ("--quoting-style", Some(v)) => {
                    o.opts.quote = if v == "c" || v == "c-maybe" {
                        "c"
                    } else if v == "escape" {
                        "escape"
                    } else if v.starts_with("shell") {
                        if v == "shell-always" {
                            "shell-always"
                        } else {
                            "shell"
                        }
                    } else {
                        "literal"
                    }
                    .into()
                }
                (k, None) if ["--width", "--tabsize", "--time", "--time-style", "--block-size", "--quoting-style", "--indicator-style"].contains(&k) => i += 1,
                _ => {}
            }
            i += 1;
            continue;
        }
        if opts && a.starts_with('-') && a.len() > 1 {
            let cs: Vec<char> = a.chars().collect();
            let mut j = 1;
            while j < cs.len() {
                let c = cs[j];
                match c {
                    'a' | 'A' => o.all = true,
                    'l' | 'g' | 'o' | 'n' => {
                        o.long = true;
                        if c == 'g' {
                            o.opts.no_owner = true;
                        }
                        if c == 'o' {
                            o.opts.no_group = true;
                        }
                        if c == 'n' {
                            o.opts.numeric = true;
                        }
                    }
                    '1' | 'C' | 'x' | 'm' => {
                        o.long = false;
                        o.opts.commas = c == 'm';
                    }
                    'i' => o.opts.inode = true,
                    's' => o.opts.blocks = true,
                    'G' => o.opts.no_group = true,
                    'h' => human = true,
                    'F' => o.opts.classify = "F".into(),
                    'p' => o.opts.classify = "p".into(),
                    'Q' => o.opts.quote = "c".into(),
                    'b' => o.opts.quote = "escape".into(),
                    'N' => o.opts.quote = "literal".into(),
                    'c' | 'u' => {
                        o.opts.time = if c == 'c' { "ctime" } else { "atime" }.into();
                        if has(&a, 't') || !has(&a, 'l') {
                            o.sort = "date".into();
                        }
                    }
                    'd' => o.self_only = true,
                    'R' => o.recursive = true,
                    'r' => o.reverse = true,
                    'B' => o.ignore.push(Pat::Backup),
                    't' => o.sort = "date".into(),
                    'S' => o.sort = "size".into(),
                    'X' => o.sort = "type".into(),
                    'U' => o.sort = "none".into(),
                    'f' => {
                        o.sort = "none".into();
                        o.all = true;
                    }
                    'v' => o.sort = String::new(),
                    'I' | 'T' | 'w' => {
                        let rest: String = cs[j + 1..].iter().collect();
                        let val = if !rest.is_empty() {
                            Some(rest)
                        } else {
                            i += 1;
                            args.get(i).map(|w| w.text.clone())
                        };
                        if c == 'I' {
                            if let Some(v) = val.filter(|v| !v.is_empty()) {
                                o.ignore.push(Pat::glob(&v));
                            }
                        }
                        break;
                    }
                    _ => {}
                }
                j += 1;
            }
            i += 1;
            continue;
        }
        operands.push(args[i].clone());
        i += 1;
    }
    o.human = human;
    o.opts.bytes = o.long && !human;
    let abs = |t: &str| -> String {
        let p = if t == "~" || t.starts_with("~/") { format!("{home}{}", &t[1..]) } else { t.to_string() };
        let p = if p.starts_with('/') { p } else { format!("{base}/{p}") };
        let p = p.trim_end_matches('/').to_string();
        if p.is_empty() {
            "/".into()
        } else {
            p
        }
    };
    let parent_of = |p: &str| -> String {
        match p.rfind('/') {
            Some(0) | None => "/".into(),
            Some(i) => p[..i].to_string(),
        }
    };
    let name_of = |p: &str| -> String { p.rsplit('/').next().unwrap_or("").to_string() };
    o.headers = o.recursive;
    if operands.is_empty() {
        if o.self_only && base != "/" {
            o.groups = vec![Group { dir: parent_of(base), names: vec![name_of(base)], patterns: vec![], all: false }];
        } else {
            o.single = base.to_string();
        }
        return Some(o);
    }
    if operands.len() == 1 && !operands[0].glob && !o.self_only {
        o.single = abs(&operands[0].text);
        return Some(o);
    }
    let mut by: Vec<Group> = vec![];
    for w in &operands {
        let p = abs(&w.text);
        if p == "/" {
            if o.self_only {
                return None;
            }
            o.groups.push(Group { dir: "/".into(), names: vec![], patterns: vec![Pat::glob(".*"), Pat::glob("*")], all: true });
            continue;
        }
        let (dir, name) = (parent_of(&p), name_of(&p));
        if w.glob && dir.replacen(base, "", 1).contains(['*', '?', '[']) {
            return None;
        }
        let gi = match by.iter().position(|g| g.dir == dir) {
            Some(i) => i,
            None => {
                by.push(Group { dir: dir.clone(), ..Default::default() });
                by.len() - 1
            }
        };
        let g = &mut by[gi];
        if w.glob {
            g.patterns.push(Pat::glob(&name));
        } else {
            g.names.push(name.clone());
        }
        if name.starts_with('.') {
            g.all = true;
        }
    }
    o.groups.extend(by);
    o.headers = o.recursive || operands.len() > 1 || operands.iter().any(|w| w.glob);
    Some(o)
}

/// The names that are ls: ls, its usual short forms, and dir. (Not lsd,
/// eza or exa: programs of their own, run as they are.)
pub const NAMES: [&str; 5] = ["ls", "ll", "la", "l", "dir"];

/// An ls the folder view shows (builtinKind's "files"): the plan, or None.
pub fn command(c: &str, base: &str, home: &str) -> Option<Plan> {
    let c = c.trim();
    if c.is_empty() || c.contains(['|', ';', '&', '<', '>', '`', '$', '(']) {
        return None;
    }
    let sw = shell_words(c);
    let first = sw.first()?.text.clone();
    if !NAMES.contains(&first.as_str()) {
        return None;
    }
    let mut p = plan(&sw[1..], base, home)?;
    if first == "ll" {
        p.long = true;
    }
    if first == "la" {
        p.long = true;
        p.all = true;
    }
    p.opts.bytes = p.long && !p.human;
    Some(p)
}

// ---- how an entry looks (lsd's way) ----------------------------------------------------

fn tint(e: &Entry) -> Sty {
    let a = |n| Sty::fg(Col::Ansi(n));
    let mut s = if e.dir {
        a(4)
    } else if e.link {
        a(6)
    } else if e.exec {
        a(2)
    } else {
        match e.kind.as_str() {
            "image" | "video" => a(5),
            "audio" => a(6),
            "archive" => a(1),
            "document" => a(3),
            _ => Sty::default(),
        }
    };
    s.bold = e.dir || e.exec;
    s
}

fn icon(e: &Entry) -> &'static str {
    if e.dir {
        return if e.link { "\u{f482}" } else { "\u{f115}" };
    }
    if e.link {
        return "\u{f481}";
    }
    match e.kind.as_str() {
        "image" => "\u{f1c5}",
        "video" => "\u{f03d}",
        "audio" => "\u{f001}",
        "archive" => "\u{f410}",
        "document" => "\u{f1c1}",
        "text" => "\u{f15c}",
        "config" => "\u{e615}",
        "code" => "\u{f121}",
        "executable" => "\u{f489}",
        _ => "\u{f15b}",
    }
}

fn human(n: u64) -> String {
    if n < 1024 {
        return format!("{n} B");
    }
    let u = ["KB", "MB", "GB", "TB"];
    let mut x = n as f64;
    let mut i = 0usize;
    x /= 1024.0;
    while x >= 1024.0 && i < u.len() - 1 {
        x /= 1024.0;
        i += 1;
    }
    if x < 10.0 {
        format!("{x:.1} {}", u[i])
    } else {
        format!("{} {}", x.round() as u64, u[i])
    }
}

fn local(t: i64) -> libc::tm {
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        let tt = t as libc::time_t;
        libc::localtime_r(&tt, &mut tm);
        tm
    }
}

/// "Mon Nov  3 14:05:09 2026" (Qt's "ddd MMM d HH:mm:ss yyyy", the day padded).
pub fn when(t: i64) -> String {
    const D: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let tm = local(t);
    format!("{} {} {:>2} {:02}:{:02}:{:02} {}", D[tm.tm_wday.clamp(0, 6) as usize], M[tm.tm_mon.clamp(0, 11) as usize], tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec, tm.tm_year + 1900)
}

fn full_time(t: i64) -> String {
    let tm = local(t);
    format!("{}-{:02}-{:02} {:02}:{:02}:{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec)
}

fn w(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

/// Cut to `n` columns, … at the end when cut.
fn elide(s: &str, n: usize) -> String {
    if w(s) <= n {
        return s.to_string();
    }
    if n == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut c = 0;
    for ch in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if c + cw > n - 1 {
            break;
        }
        out.push(ch);
        c += cw;
    }
    out.push('…');
    out
}

fn pad(s: &str, n: usize, right: bool) -> String {
    let k = n.saturating_sub(w(s));
    if right {
        format!("{}{s}", " ".repeat(k))
    } else {
        format!("{s}{}", " ".repeat(k))
    }
}

/// localeCompare with {numeric: true}: digit runs as numbers, case aside.
fn natural(a: &str, b: &str) -> Ordering {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].is_ascii_digit() && b[j].is_ascii_digit() {
            let (si, sj) = (i, j);
            while i < a.len() && a[i].is_ascii_digit() {
                i += 1;
            }
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            let x: String = a[si..i].iter().collect();
            let y: String = b[sj..j].iter().collect();
            let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
            let o = x.len().cmp(&y.len()).then(x.cmp(y));
            if o != Ordering::Equal {
                return o;
            }
        } else {
            let lo = |c: char| c.to_lowercase().next().unwrap_or(c);
            let o = lo(a[i]).cmp(&lo(b[j]));
            if o != Ordering::Equal {
                return o;
            }
            i += 1;
            j += 1;
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

fn ext(e: &Entry) -> String {
    if e.dir {
        return String::new();
    }
    match e.name.rfind('.') {
        Some(i) if i > 0 => e.name[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}

// ---- the card's choices (settings: file_browser) ---------------------------------------

fn config_dir() -> std::path::PathBuf {
    std::env::var("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"))
}
fn read_json(p: &std::path::Path) -> Value {
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(json!({}))
}
/// The last choices: Tessera's (its settings), with tessh's own over them.
fn saved_choices() -> Value {
    let mut s = read_json(&config_dir().join("tessera/settings.json")).get("file_browser").cloned().filter(|v| v.is_object()).unwrap_or(json!({}));
    if let Some(own) = read_json(&config_dir().join("tessh/editor.json")).get("file_browser").and_then(|v| v.as_object()) {
        for (k, v) in own {
            s[k] = v.clone();
        }
    }
    s
}
fn save_choice(k: &str, v: &Value) {
    if std::env::var_os("TESSH_LS_NO_SAVE").is_some() {
        return;
    }
    let f = config_dir().join("tessh/editor.json");
    let mut j = read_json(&f);
    if !j["file_browser"].is_object() {
        j["file_browser"] = json!({});
    }
    j["file_browser"][k] = v.clone();
    let _ = std::fs::create_dir_all(f.parent().unwrap());
    let _ = std::fs::write(&f, serde_json::to_string_pretty(&j).unwrap_or_default());
}

// ---- a pane: one folder ----------------------------------------------------------------

pub const COLS: [&str; 9] = ["name", "size", "date", "mode", "nlink", "user", "group", "ino", "blocks"];

#[derive(Clone, Debug)]
pub struct TCol {
    pub key: &'static str,
    pub title: String,
    pub right: bool,
    pub w: usize,
    pub x: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Pane {
    pub start_dir: String,
    pub dir: String,
    pub back: Vec<String>,
    pub forward: Vec<String>,
    pub raw: Vec<Entry>,
    pub listed: bool,
    pub only_names: Vec<String>,
    pub name_patterns: Vec<Pat>,
    pub extra_all: bool,
    pub hide_dirs: bool,
    pub collapse_empty: bool,
    pub show_bar: bool,
    pub expand_all: bool,
    pub open_dirs: HashMap<String, bool>,
    pub kids: HashMap<String, Vec<Entry>>,
    /// "name:" over it (ls a b, ls *, -R).
    pub label: Option<String>,
    pub selected: String,
    cache: Option<Arc<Vec<Entry>>>,
    rows_cache: Option<Arc<Vec<(Entry, usize)>>>,
    pub tcols: Vec<TCol>,
    /// The list's grid as last laid out: (columns, rows).
    pub grid: (usize, usize),
}

impl Pane {
    fn new(dir: &str) -> Pane {
        Pane { start_dir: dir.into(), dir: dir.into(), ..Default::default() }
    }
    fn picked(&self, e: &Entry) -> bool {
        if self.only_names.is_empty() && self.name_patterns.is_empty() {
            return true;
        }
        self.only_names.contains(&e.name) || self.name_patterns.iter().any(|p| p.matches(&e.name))
    }
    fn shown(&self, e: &Entry, ignore: &[Pat]) -> bool {
        self.picked(e) && !ignore.iter().any(|p| p.matches(&e.name))
    }
    pub fn navigated(&self) -> bool {
        !self.back.is_empty() || self.dir != self.start_dir
    }
    fn touch(&mut self) {
        self.cache = None;
        self.rows_cache = None;
    }
}

/// What a click or key works on.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LsHit {
    /// A pane's room around its entries.
    Space(usize),
    /// An entry: (pane, index in the list, or the table's row).
    Entry(usize, usize),
    /// ▸/▾ of the table's folder (pane, row).
    Arrow(usize, usize),
    Head(usize, &'static str),
    /// A heading's right edge: drag to size it, double-click to fit it.
    Edge(usize, &'static str),
    Chip(&'static str),
    /// ← → ↑ and "cd here" (pane, which).
    Nav(usize, &'static str),
    Crumb(usize, String),
    Error,
    /// The scrollbar: above the thumb (true) or below it.
    Bar(bool),
    Button(&'static str),
    Dialog,
}

/// Where the keys are.
#[derive(Clone, PartialEq, Debug)]
pub enum Focus {
    Entry,
    Head(usize),
    Chip(usize),
    Nav(usize),
}

enum Then {
    Nothing,
    CloseAsk,
    Apps(String),
    Held(String, String),
    Paste(Vec<String>, String, String),
}

enum Done {
    Act(String, Then),
    Apps(String, String, Vec<(String, String, bool)>),
    Clip(bool),
    Paste(String, Vec<String>, String),
}

struct Ask {
    op: &'static str,
    entry: Option<Entry>,
    field: Text,
    pane: usize,
}

/// The listing: the panes of one ls, its choices, what's open over it.
pub struct Ls {
    pub plan: Plan,
    pub base: String,
    saved: Value,
    mine: Value,
    pub panes: Vec<Pane>,
    pub fpane: usize,
    pub focus: Focus,
    follow: bool,
    pub scroll: usize,
    held: Option<(String, String)>,
    pub error: String,
    ask: Option<Ask>,
    confirm: Option<(Entry, usize)>,
    props: Option<Entry>,
    menu_entry: Option<(usize, Entry)>,
    apps: Vec<(String, String, bool)>,
    apps_mime: String,
    apps_for: String,
    clip_files: bool,
    jobs: Arc<Mutex<Vec<Done>>>,
    press: Option<(LsHit, u16, usize)>,
    sizing: Option<(&'static str, usize)>,
    dragging: Option<(usize, &'static str, u16)>,
    last_down: Option<(Instant, LsHit)>,
    typed: (String, Instant),
    /// The last panel: its rows in all, the room it had, the focused thing's (row, column).
    total: usize,
    room: usize,
    sel_at: Option<(usize, usize)>,
    capped: bool,
    /// The width it was last drawn at.
    cols_last: usize,
}

impl Ls {
    /// A listing for an ls run in `base` (the plan from `command`).
    pub fn new(plan: Plan, base: &str) -> Ls {
        let saved = saved_choices();
        let mut mine = saved.clone();
        mine.as_object_mut().map(|o| o.remove("view"));
        // (ls -t, -S, -r ...: the command's order, not the last one picked.)
        if !plan.sort.is_empty() || plan.reverse || plan.folders_first {
            if let Some(o) = mine.as_object_mut() {
                o.remove("sort");
                o.remove("desc");
                o.remove("folders_first");
            }
        }
        let mut ls = Ls {
            plan,
            base: base.into(),
            saved,
            mine,
            panes: vec![],
            fpane: 0,
            focus: Focus::Entry,
            follow: false,
            scroll: 0,
            held: None,
            error: String::new(),
            ask: None,
            confirm: None,
            props: None,
            menu_entry: None,
            apps: vec![],
            apps_mime: String::new(),
            apps_for: String::new(),
            clip_files: false,
            jobs: Arc::new(Mutex::new(vec![])),
            press: None,
            sizing: None,
            dragging: None,
            last_down: None,
            typed: (String::new(), Instant::now()),
            total: 0,
            room: 0,
            sel_at: None,
            capped: false,
            cols_last: 80,
        };
        ls.make_panes();
        ls
    }

    /// The panes, as the card has them: the entries named (each folder's
    /// that has some), then a pane for each folder (ls a b, ls *; -R:
    /// every folder under them, right after its own, at most 200).
    fn make_panes(&mut self) {
        let mut panes = vec![];
        for (i, g) in self.plan.groups.iter().enumerate() {
            let mut p = Pane::new(&g.dir);
            p.only_names = g.names.clone();
            p.name_patterns = g.patterns.clone();
            p.extra_all = g.all;
            p.hide_dirs = !self.plan.self_only;
            p.collapse_empty = !self.plan.self_only;
            p.show_bar = i == 0;
            panes.push(p);
        }
        self.panes = panes;
        for p in 0..self.panes.len() {
            self.list(p);
        }
        let mut dirs: Vec<String> = vec![];
        if !self.plan.single.is_empty() {
            dirs.push(self.plan.single.clone());
        }
        if !self.plan.self_only {
            for p in 0..self.panes.len() {
                let mut m = self.matched_dirs(p);
                m.sort();
                for d in m {
                    if !dirs.contains(&d) {
                        dirs.push(d);
                    }
                }
            }
        }
        let groups = self.panes.len();
        let mut k = 0;
        while k < dirs.len() && self.panes.len() < groups + 200 {
            let mut p = Pane::new(&dirs[k]);
            p.show_bar = k == 0 && groups == 0;
            p.label = self.plan.headers.then(|| self.label(&dirs[k]));
            self.panes.push(p);
            let pi = self.panes.len() - 1;
            self.list(pi);
            let view = self.view();
            if self.plan.recursive {
                if view == "details" {
                    self.panes[pi].expand_all = true;
                    if !self.plan.single.is_empty() {
                        self.panes[pi].label = None;
                    }
                } else {
                    let mut subs = self.sub_dirs(pi);
                    subs.sort();
                    let mut at = k + 1;
                    for d in subs {
                        if !dirs.contains(&d) && dirs.len() < 200 {
                            dirs.insert(at, d);
                            at += 1;
                        }
                    }
                }
            }
            k += 1;
        }
    }

    fn label(&self, d: &str) -> String {
        let b = self.base.trim_end_matches('/');
        let l = if d == b {
            ".".to_string()
        } else if let Some(r) = d.strip_prefix(&format!("{b}/")) {
            r.to_string()
        } else {
            d.to_string()
        };
        format!("{l}:")
    }

    // ---- the choices ------------------------------------------------------------------
    fn mine_str(&self, k: &str) -> Option<String> {
        self.mine.get(k).and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(String::from)
    }
    pub fn view(&self) -> String {
        if self.plan.opts.commas {
            return "commas".into();
        }
        let v = self.mine_str("view").unwrap_or_else(|| if self.plan.long { "details".into() } else { self.saved.get("view").and_then(|v| v.as_str()).unwrap_or("list").to_string() });
        if v == "details" {
            v
        } else {
            "list".into()
        }
    }
    fn sort_by(&self) -> String {
        self.mine_str("sort").unwrap_or_else(|| if self.plan.sort.is_empty() { "name".into() } else { self.plan.sort.clone() })
    }
    fn desc(&self) -> bool {
        if self.mine_str("sort").is_some() {
            self.mine.get("desc").and_then(|v| v.as_bool()).unwrap_or(false)
        } else {
            (self.plan.sort == "date" || self.plan.sort == "size") != self.plan.reverse
        }
    }
    fn folders_first(&self) -> bool {
        self.mine.get("folders_first").and_then(|v| v.as_bool()).unwrap_or(self.plan.folders_first)
    }
    fn show_hidden(&self, p: usize) -> bool {
        self.plan.all || self.panes.get(p).is_some_and(|x| x.extra_all) || self.mine.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false)
    }
    fn sort_keys(&self) -> Vec<(String, bool)> {
        self.mine.get("sortKeys").and_then(|v| v.as_array()).map(|a| a.iter().map(|k| (k["key"].as_str().unwrap_or("").to_string(), k["desc"].as_bool().unwrap_or(false))).collect()).unwrap_or_default()
    }
    fn col_shown(&self, k: &str) -> Option<bool> {
        self.mine.get("colShown").and_then(|v| v.get(k)).and_then(|v| v.as_bool())
    }
    fn col_w(&self, k: &str) -> usize {
        self.mine.get("colW").and_then(|v| v.get(k)).and_then(|v| v.as_f64()).map(|x| x.round() as usize).unwrap_or(0)
    }
    fn col_order(&self) -> Vec<String> {
        self.mine.get("colOrder").and_then(|v| v.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
    }
    fn save(&mut self, k: &str, v: Value) {
        if !self.mine.is_object() {
            self.mine = json!({});
        }
        self.mine[k] = v.clone();
        if !self.saved.is_object() {
            self.saved = json!({});
        }
        self.saved[k] = v.clone();
        save_choice(k, &v);
        self.touch();
    }
    fn touch(&mut self) {
        for p in &mut self.panes {
            p.touch();
        }
    }
    fn set_view(&mut self, v: &str) {
        let had = self.view();
        self.save("view", json!(v));
        // (-R: in the table a tree, in the list a pane for each folder.)
        if self.plan.recursive && had != self.view() {
            self.make_panes();
        }
    }
    pub fn col_on(&self, k: &str) -> bool {
        if k == "name" {
            return true;
        }
        if let Some(b) = self.col_shown(k) {
            return b;
        }
        let o = &self.plan.opts;
        k == "size" || k == "date" || k == "mode" || (k == "user" && !o.no_owner) || (k == "group" && !o.no_group) || (k == "ino" && o.inode) || (k == "blocks" && o.blocks)
    }
    fn col_title(&self, k: &str) -> String {
        match k {
            "name" => "Name",
            "size" => "Size",
            "mode" => "Permissions",
            "nlink" => "Links",
            "user" => "Owner",
            "group" => "Group",
            "ino" => "Inode",
            "blocks" => "Blocks",
            "date" => match self.plan.opts.time.as_str() {
                "ctime" => "Changed",
                "atime" => "Accessed",
                _ => "Modified",
            },
            _ => k,
        }
        .to_string()
    }
    fn set_col_on(&mut self, k: &str, on: bool) {
        let mut m = self.mine.get("colShown").cloned().filter(|v| v.is_object()).unwrap_or(json!({}));
        m[k] = json!(on);
        self.save("colShown", m);
    }
    fn set_col_w(&mut self, k: &str, wd: usize) {
        let mut m = self.mine.get("colW").cloned().filter(|v| v.is_object()).unwrap_or(json!({}));
        if wd > 0 {
            m[k] = json!(wd);
        } else {
            m.as_object_mut().map(|o| o.remove(k));
        }
        self.save("colW", m);
    }
    /// A column moved to before another ("": to the end).
    fn move_col(&mut self, k: &str, before: &str) {
        let mut o = self.col_order();
        if o.is_empty() {
            o = COLS.iter().map(|s| s.to_string()).collect();
        }
        for c in COLS {
            if !o.iter().any(|x| x == c) {
                o.push(c.into());
            }
        }
        o.retain(|c| c != k);
        let j = if before.is_empty() { o.len() } else { o.iter().position(|c| c == before).unwrap_or(o.len()) };
        o.insert(j, k.into());
        self.save("colOrder", json!(o));
    }
    /// A heading's click: sorted by it, then the other way, then not (back
    /// to the command's order); Shift+click does the same to it as one more key.
    fn sort_on(&mut self, key: &str, add: bool) {
        let mut ks = self.sort_keys();
        let j = ks.iter().position(|k| k.0 == key);
        let first = key == "date" || key == "size";
        let next = match j {
            None => Some((key.to_string(), first)),
            Some(j) if ks[j].1 == first => Some((key.to_string(), !first)),
            _ => None,
        };
        if add {
            match (j, next.clone()) {
                (None, Some(n)) => ks.push(n),
                (Some(j), Some(n)) => ks[j] = n,
                (Some(j), None) => {
                    ks.remove(j);
                }
                _ => {}
            }
        } else {
            let only = ks.len() == 1 && j == Some(0);
            ks = match next {
                Some(n) if only || j.is_none() => vec![n],
                _ if j.is_some() && !only => vec![(key.to_string(), first)],
                _ => vec![],
            };
        }
        self.save("sortKeys", Value::Array(ks.into_iter().map(|(k, d)| json!({"key": k, "desc": d})).collect()));
    }
    fn sort_mark(&self, key: &str) -> String {
        let ks = self.sort_keys();
        match ks.iter().position(|k| k.0 == key) {
            None => String::new(),
            Some(j) => format!("{}{}", if ks[j].1 { " ▼" } else { " ▲" }, if ks.len() > 1 { (j + 1).to_string() } else { String::new() }),
        }
    }

    // ---- an entry's text ---------------------------------------------------------------
    fn quoted(&self, n: &str) -> String {
        match self.plan.opts.quote.as_str() {
            "c" => format!("\"{}\"", n.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\t', "\\t")),
            "escape" => n.replace('\\', "\\\\").replace(' ', "\\ ").replace('\n', "\\n").replace('\t', "\\t"),
            q @ ("shell" | "shell-always") => {
                if q == "shell" && !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || "_./+,:@%=-".contains(c)) {
                    n.to_string()
                } else {
                    format!("'{}'", n.replace('\'', "'\\''"))
                }
            }
            _ => n.to_string(),
        }
    }
    fn mark(&self, e: &Entry) -> &'static str {
        if e.dir {
            return "/";
        }
        if self.plan.opts.classify != "F" {
            return "";
        }
        if e.link {
            "@"
        } else if e.special == "fifo" {
            "|"
        } else if e.special == "socket" {
            "="
        } else if e.exec {
            "*"
        } else {
            ""
        }
    }
    pub fn plain_name(&self, e: &Entry) -> String {
        format!("{}{}", self.quoted(&e.name), self.mark(e))
    }
    fn size_text(&self, e: &Entry) -> String {
        if self.plan.opts.bytes {
            return e.size.to_string();
        }
        if e.dir {
            if e.items >= 0 {
                format!("{} {}", e.items, if e.items == 1 { "item" } else { "items" })
            } else {
                String::new()
            }
        } else {
            human(e.size)
        }
    }
    fn owner(&self, e: &Entry) -> String {
        match (self.plan.opts.numeric, e.uid) {
            (true, Some(u)) => u.to_string(),
            _ => e.user.clone(),
        }
    }
    fn group_of(&self, e: &Entry) -> String {
        match (self.plan.opts.numeric, e.gid) {
            (true, Some(g)) => g.to_string(),
            _ => e.group.clone(),
        }
    }
    fn time_of(&self, e: &Entry) -> i64 {
        match self.plan.opts.time.as_str() {
            "ctime" => e.ctime.unwrap_or(e.mtime),
            "atime" => e.atime.unwrap_or(e.mtime),
            _ => e.mtime,
        }
    }
    fn stamp(&self, t: i64) -> String {
        if self.plan.opts.full_time {
            full_time(t)
        } else {
            when(t)
        }
    }
    fn cell(&self, e: &Entry, key: &str) -> String {
        match key {
            "size" => self.size_text(e),
            "date" => self.stamp(self.time_of(e)),
            "mode" => e.mode.clone(),
            "user" => self.owner(e),
            "group" => self.group_of(e),
            "nlink" => e.nlink.map(|x| x.to_string()).unwrap_or_default(),
            "ino" => e.ino.map(|x| x.to_string()).unwrap_or_default(),
            "blocks" => e.blocks.map(|x| x.to_string()).unwrap_or_default(),
            _ => String::new(),
        }
    }
    /// -i and -s before a name in the list: inode and blocks, padded.
    fn prefix(&self, e: &Entry, wi: usize, wb: usize) -> String {
        let mut p = String::new();
        if self.plan.opts.inode {
            p += &pad(&e.ino.map(|x| x.to_string()).unwrap_or("?".into()), wi, true);
            p.push(' ');
        }
        if self.plan.opts.blocks {
            p += &pad(&e.blocks.map(|x| x.to_string()).unwrap_or("?".into()), wb, true);
            p.push(' ');
        }
        p
    }

    // ---- reading and order -------------------------------------------------------------
    fn list(&mut self, p: usize) {
        let all = self.show_hidden(p);
        let pane = &mut self.panes[p];
        pane.raw = if pane.dir.is_empty() { vec![] } else { read_dir(&pane.dir, all) };
        pane.listed = true;
        pane.kids.clear();
        pane.touch();
    }
    fn refresh(&mut self) {
        for p in 0..self.panes.len() {
            self.list(p);
        }
    }
    fn matched_dirs(&self, p: usize) -> Vec<String> {
        let pane = &self.panes[p];
        if pane.only_names.is_empty() && pane.name_patterns.is_empty() {
            return vec![];
        }
        pane.raw.iter().filter(|e| e.dir && pane.shown(e, &self.plan.ignore)).map(|e| e.path.clone()).collect()
    }
    fn sub_dirs(&self, p: usize) -> Vec<String> {
        let pane = &self.panes[p];
        let hidden = self.show_hidden(p);
        pane.raw.iter().filter(|e| e.dir && pane.shown(e, &self.plan.ignore) && (hidden || !e.name.starts_with('.'))).map(|e| e.path.clone()).collect()
    }
    fn arrange(&self, mut a: Vec<Entry>) -> Vec<Entry> {
        let name = |x: &Entry| x.name.to_lowercase().trim_start_matches('.').to_string();
        let by_name = |x: &Entry, y: &Entry| natural(&name(x), &name(y));
        let key_cmp = |k: &str, x: &Entry, y: &Entry| -> Ordering {
            match k {
                "name" => by_name(x, y),
                "size" => {
                    let s = |e: &Entry| if e.dir { e.items as f64 } else { e.size as f64 };
                    s(x).partial_cmp(&s(y)).unwrap_or(Ordering::Equal)
                }
                "date" => self.time_of(x).cmp(&self.time_of(y)),
                "type" => ext(x).cmp(&ext(y)),
                "mode" => x.mode.cmp(&y.mode),
                "user" => self.owner(x).to_lowercase().cmp(&self.owner(y).to_lowercase()),
                "group" => self.group_of(x).to_lowercase().cmp(&self.group_of(y).to_lowercase()),
                "ino" => x.ino.unwrap_or(0).cmp(&y.ino.unwrap_or(0)),
                "blocks" => x.blocks.unwrap_or(0).cmp(&y.blocks.unwrap_or(0)),
                "nlink" => x.nlink.unwrap_or(0).cmp(&y.nlink.unwrap_or(0)),
                _ => Ordering::Equal,
            }
        };
        let keys = self.sort_keys();
        let (sort_by, desc, ff) = (self.sort_by(), self.desc(), self.folders_first());
        a.sort_by(|x, y| {
            if ff && x.dir != y.dir {
                return if x.dir { Ordering::Less } else { Ordering::Greater };
            }
            if !keys.is_empty() {
                for (k, d) in &keys {
                    let r = key_cmp(k, x, y);
                    if r != Ordering::Equal {
                        return if *d { r.reverse() } else { r };
                    }
                }
                return by_name(x, y);
            }
            let mut r = key_cmp(&sort_by, x, y);
            if r == Ordering::Equal && sort_by != "name" && sort_by != "none" {
                r = by_name(x, y);
            }
            if desc {
                r.reverse()
            } else {
                r
            }
        });
        a
    }
    /// What a pane shows, in order.
    pub fn entries(&mut self, p: usize) -> Arc<Vec<Entry>> {
        if let Some(c) = &self.panes[p].cache {
            return c.clone();
        }
        let pane = &self.panes[p];
        let list: Vec<Entry> = pane.raw.iter().filter(|e| pane.shown(e, &self.plan.ignore) && !(pane.hide_dirs && e.dir)).cloned().collect();
        let c = Arc::new(self.arrange(list));
        self.panes[p].cache = Some(c.clone());
        c
    }
    fn is_open(&self, p: usize, path: &str) -> bool {
        let pane = &self.panes[p];
        pane.open_dirs.get(path).copied().unwrap_or(pane.expand_all)
    }
    fn toggle_open(&mut self, p: usize, e: &Entry) {
        let o = !self.is_open(p, &e.path);
        self.panes[p].open_dirs.insert(e.path.clone(), o);
        self.panes[p].rows_cache = None;
    }
    /// The table's rows: the entries, an open folder's under it a level in
    /// (its listing read once it's within `budget` rows; -R's tree fills in
    /// as it's scrolled).
    fn rows(&mut self, p: usize, budget: usize) -> Arc<Vec<(Entry, usize)>> {
        if let Some(c) = &self.panes[p].rows_cache {
            if c.len() >= budget.min(200_000) || !self.unread_open(p, c) {
                return c.clone();
            }
        }
        let ents = self.entries(p);
        let mut out: Vec<(Entry, usize)> = vec![];
        let mut stack: Vec<(Arc<Vec<Entry>>, usize, usize)> = vec![(ents, 0, 0)];
        while let Some((list, i, depth)) = stack.pop() {
            if i >= list.len() || out.len() >= 200_000 {
                continue;
            }
            let e = list[i].clone();
            stack.push((list.clone(), i + 1, depth));
            out.push((e.clone(), depth));
            if e.dir && self.is_open(p, &e.path) {
                if !self.panes[p].kids.contains_key(&e.path) && out.len() < budget {
                    let k = read_dir(&e.path, self.show_hidden(p));
                    self.panes[p].kids.insert(e.path.clone(), k);
                }
                if let Some(k) = self.panes[p].kids.get(&e.path) {
                    let k: Vec<Entry> = k.iter().filter(|x| !self.plan.ignore.iter().any(|q| q.matches(&x.name))).cloned().collect();
                    stack.push((Arc::new(self.arrange(k)), 0, depth + 1));
                }
            }
        }
        let c = Arc::new(out);
        self.panes[p].rows_cache = Some(c.clone());
        c
    }
    fn unread_open(&self, p: usize, rows: &[(Entry, usize)]) -> bool {
        rows.iter().any(|(e, _)| e.dir && self.is_open(p, &e.path) && !self.panes[p].kids.contains_key(&e.path))
    }
    /// The pane's items as the keys and clicks count them.
    fn items(&mut self, p: usize) -> Vec<Entry> {
        if self.view() == "details" {
            let b = self.room + self.scroll + 200;
            self.rows(p, b).iter().map(|r| r.0.clone()).collect()
        } else {
            self.entries(p).to_vec()
        }
    }

    // ---- going places --------------------------------------------------------------------
    fn go(&mut self, p: usize, d: &str) {
        let pane = &mut self.panes[p];
        if d == pane.dir {
            return;
        }
        pane.back.push(pane.dir.clone());
        pane.forward.clear();
        pane.dir = d.into();
        pane.selected.clear();
        pane.open_dirs.clear();
        self.list(p);
    }
    fn go_back(&mut self, p: usize) {
        let pane = &mut self.panes[p];
        let Some(d) = pane.back.pop() else { return };
        pane.forward.insert(0, pane.dir.clone());
        pane.dir = d;
        self.list(p);
    }
    fn go_forward(&mut self, p: usize) {
        let pane = &mut self.panes[p];
        if pane.forward.is_empty() {
            return;
        }
        let d = pane.forward.remove(0);
        pane.back.push(pane.dir.clone());
        pane.dir = d;
        self.list(p);
    }
    fn up(&mut self, p: usize) {
        let d = self.panes[p].dir.clone();
        let t = d.trim_end_matches('/');
        let up = match t.rfind('/') {
            Some(0) | None => "/".to_string(),
            Some(i) => t[..i].to_string(),
        };
        self.go(p, &up);
    }

    fn act(&self, op: &str, path: &str, arg: &str, then: Then) {
        let jobs = self.jobs.clone();
        let (op, path, arg) = (op.to_string(), path.to_string(), arg.to_string());
        std::thread::spawn(move || {
            let r = files::action(&op, &path, &arg);
            jobs.lock().unwrap().push(Done::Act(r, then));
        });
    }
    fn load_apps(&mut self, path: &str) {
        self.apps.clear();
        self.apps_mime.clear();
        self.apps_for = path.into();
        let jobs = self.jobs.clone();
        let path = path.to_string();
        std::thread::spawn(move || {
            let j: Value = serde_json::from_str(&files::apps_for(&path)).unwrap_or(json!({}));
            let apps = j["apps"].as_array().map(|a| a.iter().map(|x| (x["id"].as_str().unwrap_or("").to_string(), x["name"].as_str().unwrap_or("").to_string(), x["default"].as_bool().unwrap_or(false))).collect()).unwrap_or_default();
            jobs.lock().unwrap().push(Done::Apps(path, j["mime"].as_str().unwrap_or("").to_string(), apps));
        });
    }
    /// Whether the desktop's clipboard has files (for Paste), asked in the background.
    fn probe_clip(&self) {
        let jobs = self.jobs.clone();
        std::thread::spawn(move || {
            let has = desktop_files().is_some_and(|(_, f)| !f.is_empty());
            jobs.lock().unwrap().push(Done::Clip(has));
        });
    }
    fn copy_file(&mut self, path: &str, cut: bool) {
        self.held = Some((if cut { "move" } else { "copy" }.into(), path.into()));
        let path = path.to_string();
        std::thread::spawn(move || put_desktop_files(&[path], cut));
    }
    fn can_paste(&self) -> bool {
        self.held.is_some() || self.clip_files
    }
    /// Into `dest`: what was copied or cut here, else the desktop clipboard's files.
    fn paste_into(&mut self, dest: &str) {
        if let Some((op, path)) = self.held.clone() {
            self.act(&op, &path, dest, Then::Held(op.clone(), path.clone()));
            return;
        }
        let jobs = self.jobs.clone();
        let dest = dest.to_string();
        std::thread::spawn(move || {
            if let Some((cut, files)) = desktop_files() {
                jobs.lock().unwrap().push(Done::Paste(if cut { "move".into() } else { "copy".into() }, files, dest));
            }
        });
    }

    /// What's come back from the background; whether anything did.
    fn take_done(&mut self) -> (bool, bool) {
        let done: Vec<Done> = std::mem::take(&mut *self.jobs.lock().unwrap());
        let (mut any, mut menus) = (false, false);
        for d in done {
            any = true;
            match d {
                Done::Act(r, then) => {
                    let ok = r.is_empty();
                    if let Some(prog) = r.strip_prefix("needs-nix:") {
                        self.error = nix_help(prog);
                    } else {
                        self.error = r;
                    }
                    if ok {
                        self.refresh();
                    }
                    match then {
                        Then::CloseAsk if ok => self.ask = None,
                        Then::Apps(p) if ok => {
                            self.load_apps(&p);
                        }
                        Then::Held(op, path) if ok && op == "move" && self.held == Some((op.clone(), path.clone())) => self.held = None,
                        Then::Paste(mut rest, op, dest) if ok && !rest.is_empty() => {
                            let next = rest.remove(0);
                            self.act(&op, &next, &dest, Then::Paste(rest, op.clone(), dest.clone()));
                        }
                        _ => {}
                    }
                }
                Done::Apps(path, mime, apps) => {
                    if path == self.apps_for {
                        self.apps = apps;
                        self.apps_mime = mime;
                        menus = true;
                    }
                }
                Done::Clip(has) => {
                    self.clip_files = has;
                    menus = true;
                }
                Done::Paste(op, mut files, dest) => {
                    files.retain(|p| !p.is_empty() && *p != dest && parent(p) != dest);
                    if !files.is_empty() {
                        let first = files.remove(0);
                        self.act(&op, &first, &dest, Then::Paste(files, op.clone(), dest.clone()));
                    }
                }
            }
        }
        (any, menus)
    }

    // ---- drawing -------------------------------------------------------------------------
    fn chips(&self) -> Vec<(&'static str, String, bool, String)> {
        let v = self.view();
        let mut c = vec![("list", "☰".to_string(), v == "list", "List (as ls)".to_string()), ("details", "≡".to_string(), v == "details", "Details (as ls -l)".to_string())];
        if v == "details" {
            c.push(("columns", "Columns ▾".into(), false, "Which columns show (or right-click the header)".into()));
        }
        c.push(("first", "\u{f115} first".into(), self.folders_first(), "Folders first".into()));
        c.push(("hidden", ".hidden".into(), self.plan.all || self.mine.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false), "Show hidden files".into()));
        c.push(("keys", "⌨ Ctrl+Alt+L".into(), false, String::new()));
        c
    }
    fn nav_items(&self, p: usize) -> Vec<(String, LsHit, bool)> {
        let pane = &self.panes[p];
        let mut v = vec![
            (" ← ".to_string(), LsHit::Nav(p, "back"), !pane.back.is_empty()),
            (" → ".to_string(), LsHit::Nav(p, "forward"), !pane.forward.is_empty()),
            (" ↑ ".to_string(), LsHit::Nav(p, "up"), pane.dir != "/"),
        ];
        let parts: Vec<&str> = pane.dir.split('/').filter(|s| !s.is_empty()).collect();
        v.push(("/".into(), LsHit::Crumb(p, "/".into()), true));
        let mut acc = String::new();
        for (i, part) in parts.iter().enumerate() {
            acc.push('/');
            acc.push_str(part);
            v.push((format!("{}{part}", if i > 0 { "/" } else { "" }), LsHit::Crumb(p, acc.clone()), true));
        }
        v.push(("   cd here".into(), LsHit::Nav(p, "cd"), true));
        // (Too long for the row: the first folders folded into …, so "cd here" shows.)
        let room = self.cols_last.max(20).saturating_sub(2);
        while v.iter().map(|x| w(&x.0)).sum::<usize>() > room && v.len() > 7 {
            let folded = matches!(v[4].1, LsHit::Space(_));
            v.remove(if folded { 5 } else { 4 });
            if !folded {
                v.insert(4, ("…".into(), LsHit::Space(p), false));
            }
        }
        v
    }

    /// The whole listing as rows (`live`: with its buttons, hovering and the
    /// selection; else as it's left in the scrollback).
    fn render(&mut self, pal: &Palette, cols: usize, hovered: Option<&LsHit>, live: bool, focused: bool, budget: usize) -> Vec<Line> {
        let cols = cols.max(20);
        self.cols_last = cols;
        let dim = pal.dimmed();
        let mut out: Vec<Line> = vec![];
        self.sel_at = None;
        let view = self.view();
        let sel_bg = accent(pal, 0.3);
        let hov_bg = pal.mix(0.07);
        let focus_bg = accent(pal, 0.45);
        let mut first_shown = true;
        for p in 0..self.panes.len() {
            let ents = self.entries(p);
            let pane_listed = self.panes[p].listed;
            if self.panes[p].collapse_empty && pane_listed && ents.is_empty() {
                continue;
            }
            if let Some(l) = self.panes[p].label.clone() {
                if !first_shown {
                    out.push(vec![]);
                }
                out.push(vec![Span::new(l, dim)]);
            }
            first_shown = false;
            // The toolbar (live only).
            if live && self.panes[p].show_bar {
                let mut row: Line = vec![];
                let mut x = 0;
                for (i, (name, label, on, _)) in self.chips().into_iter().enumerate() {
                    if name == "keys" {
                        let t = format!(" {label} ");
                        let gap = cols.saturating_sub(x + w(&t) + 1);
                        row.push(Span::new(" ".repeat(gap), Sty::default()).hit(Hit::Ls(LsHit::Space(p))));
                        let hov = hovered == Some(&LsHit::Chip(name));
                        let f = focused && self.fpane == p && self.focus == Focus::Chip(i);
                        let mut st = dim;
                        st.underline = hov;
                        if f {
                            st.bg = Some(focus_bg);
                        }
                        if f {
                            self.sel_at = Some((out.len(), x + gap));
                        }
                        row.push(Span::new(t, st).hit(Hit::Ls(LsHit::Chip(name))));
                        continue;
                    }
                    if x > 0 {
                        row.push(Span::new(" ", Sty::default()).hit(Hit::Ls(LsHit::Space(p))));
                        x += 1;
                    }
                    let hov = hovered == Some(&LsHit::Chip(name));
                    let f = focused && self.fpane == p && self.focus == Focus::Chip(i);
                    let bg = if f { Some(focus_bg) } else if on { Some(accent(pal, 0.4)) } else { pal.mix(if hov { 0.17 } else { 0.10 }) };
                    let mut st = Sty { bg, underline: hov, ..Default::default() };
                    if pal.bg.is_none() && !on && !f {
                        st.fg = Some(Col::Ansi(if hov { 15 } else { 7 }));
                    }
                    let t = format!(" {label} ");
                    if f {
                        self.sel_at = Some((out.len(), x));
                    }
                    x += w(&t);
                    row.push(Span::new(t, st).hit(Hit::Ls(LsHit::Chip(name))));
                }
                out.push(row);
            }
            // The path bar, once it's been somewhere. (Left in the scrollback:
            // where it was, as a heading; nothing there to click.)
            if !live && self.panes[p].navigated() {
                out.push(vec![Span::new(format!("{}:", self.panes[p].dir), dim)]);
            }
            if live && self.panes[p].navigated() {
                let mut row: Line = vec![Span::new(" ", Sty::default()).hit(Hit::Ls(LsHit::Space(p)))];
                let mut x = 1;
                for (i, (t, hit, on)) in self.nav_items(p).into_iter().enumerate() {
                    let hov = live && hovered == Some(&hit);
                    let f = live && focused && self.fpane == p && self.focus == Focus::Nav(i);
                    let mut st = match &hit {
                        LsHit::Crumb(..) if hov => Sty::fg(Col::Ansi(4)),
                        LsHit::Nav(_, "cd") if hov => Sty::fg(Col::Ansi(2)),
                        _ if !on => Sty { fg: pal.mix(0.25).or(Some(Col::Ansi(8))), ..Default::default() },
                        _ if hov => Sty::default(),
                        _ => dim,
                    };
                    st.underline = hov && on;
                    if f {
                        st.bg = Some(focus_bg);
                        self.sel_at = Some((out.len(), x));
                    }
                    x += w(&t);
                    row.push(Span::new(t, st).hit(Hit::Ls(hit)));
                }
                out.push(row);
            }
            let room = |row: &mut Line, x: usize, to: usize| {
                if to > x {
                    row.push(Span::new(" ".repeat(to - x), Sty::default()).hit(Hit::Ls(LsHit::Space(p))));
                }
            };
            // The entries.
            if view == "commas" {
                let (wi, wb) = self.long_w(&ents);
                let mut row: Line = vec![];
                let mut x = 0;
                for (i, e) in ents.iter().enumerate() {
                    let t = format!("{}{}{}", self.prefix(e, wi, wb), self.plain_name(e), if i + 1 < ents.len() { "," } else { "" });
                    let tw = w(&t);
                    if x > 0 && x + 1 + tw > cols {
                        out.push(std::mem::take(&mut row));
                        x = 0;
                    }
                    if x > 0 {
                        row.push(Span::new(" ", Sty::default()).hit(Hit::Ls(LsHit::Space(p))));
                        x += 1;
                    }
                    let mut st = tint(e);
                    st.bold = false;
                    if live && self.panes[p].selected == e.path {
                        st.bg = Some(sel_bg);
                        if focused && self.fpane == p && self.focus == Focus::Entry {
                            self.sel_at = Some((out.len(), x));
                        }
                    }
                    row.push(Span::new(t, st).hit(Hit::Ls(LsHit::Space(p))));
                    x += tw;
                }
                if !row.is_empty() {
                    out.push(row);
                }
            } else if view == "details" {
                let rows = self.rows(p, budget);
                let tcols = self.table_cols(p, &rows, cols);
                self.panes[p].tcols = tcols.clone();
                // The header: each a sort; its edge sizes it.
                let mut row: Line = vec![];
                let mut x = 0;
                for (ci, c) in tcols.iter().enumerate() {
                    let cx = 2 + c.x;
                    room(&mut row, x, cx);
                    x = x.max(cx);
                    let title = elide(&format!("{}{}", c.title, self.sort_mark(c.key)), c.w);
                    let t = pad(&title, c.w, c.right);
                    let hov = live && hovered == Some(&LsHit::Head(p, c.key));
                    let f = live && focused && self.fpane == p && self.focus == Focus::Head(ci);
                    let dragged = self.dragging.is_some_and(|d| d.0 == p && d.1 == c.key);
                    let mut st = if hov { Sty::fg(Col::Ansi(4)) } else { dim };
                    st.bold = true;
                    if dragged || f {
                        st.bg = Some(accent(pal, 0.25));
                    }
                    if f {
                        self.sel_at = Some((out.len(), cx));
                    }
                    row.push(Span::new(t, st).hit(Hit::Ls(LsHit::Head(p, c.key))));
                    x += c.w;
                    let ew = 2.min(cols.saturating_sub(x));
                    let target = self.drop_before(p);
                    let drop_here = match target.as_deref() {
                        Some("") => ci + 1 == tcols.len(),
                        Some(t) => tcols.get(ci + 1).is_some_and(|n| n.key == t),
                        None => false,
                    };
                    let edge = if drop_here { "┃ " } else { "  " };
                    let est = if drop_here { Sty::fg(Col::Ansi(4)) } else { Sty::default() };
                    row.push(Span::new(&edge[..edge.char_indices().nth(ew).map(|(i, _)| i).unwrap_or(edge.len())], est).hit(Hit::Ls(LsHit::Edge(p, c.key))));
                    x += ew;
                }
                out.push(row);
                if rows.is_empty() {
                    if pane_listed && self.panes[p].raw.is_empty() {
                        out.push(vec![Span::new("(empty)", dim)]);
                    }
                }
                let name_col = tcols.iter().find(|c| c.key == "name").cloned();
                for (ri, (e, depth)) in rows.iter().enumerate() {
                    let hit = LsHit::Entry(p, ri);
                    let hov = live && hovered == Some(&hit);
                    let sel = live && self.panes[p].selected == e.path;
                    let bg = if sel { Some(sel_bg) } else if hov { hov_bg } else { None };
                    let mut row: Line = vec![];
                    let mut x = 0;
                    let put = |row: &mut Line, x: &mut usize, at: usize, t: String, st: Sty, h: LsHit| {
                        if at > *x {
                            row.push(Span::new(" ".repeat(at - *x), Sty { bg, ..Default::default() }).hit(Hit::Ls(hit.clone())));
                            *x = at;
                        }
                        *x += w(&t);
                        row.push(Span::new(t, st.with_bg(bg)).hit(Hit::Ls(h)));
                    };
                    if let Some(nc) = &name_col {
                        let ax = nc.x + depth * 2;
                        if e.dir && live {
                            let ah = LsHit::Arrow(p, ri);
                            let st = if live && hovered == Some(&ah) { Sty::fg(Col::Ansi(4)) } else { dim };
                            put(&mut row, &mut x, ax, if self.is_open(p, &e.path) { "▾ ".into() } else { "▸ ".into() }, st, ah);
                        }
                        let nx = ax + 2;
                        let nw = (nc.w + 2).saturating_sub(depth * 2 + 2);
                        let label = format!("{} {}", icon(e), self.plain_name(e));
                        let link = if e.link { format!(" ⇒ {}", e.target) } else { String::new() };
                        let full = format!("{label}{link}");
                        let shown = elide(&full, nw);
                        let (a, b) = if w(&shown) > w(&label) && shown.starts_with(&label) { (label.clone(), shown[label.len()..].to_string()) } else { (shown, String::new()) };
                        let mut st = tint(e);
                        st.underline = hov;
                        if sel && focused && self.fpane == p && self.focus == Focus::Entry {
                            self.sel_at = Some((out.len(), nx));
                        }
                        put(&mut row, &mut x, nx, a, st, hit.clone());
                        if !b.is_empty() {
                            let at = x;
                            put(&mut row, &mut x, at, b, dim, hit.clone());
                        }
                    }
                    for c in tcols.iter().filter(|c| c.key != "name") {
                        let t = pad(&elide(&self.cell(e, c.key), c.w), c.w, c.right);
                        put(&mut row, &mut x, 2 + c.x, t, dim, hit.clone());
                    }
                    if live {
                        let end = cols.min(tcols.last().map(|c| 2 + c.x + c.w).unwrap_or(cols));
                        if end > x {
                            row.push(Span::new(" ".repeat(end - x), Sty { bg, ..Default::default() }).hit(Hit::Ls(hit.clone())));
                        }
                    }
                    out.push(row);
                }
            } else {
                // ls's own layout: as many columns as fit, filled going down.
                let (wi, wb) = self.long_w(&ents);
                let n = ents.len();
                let widths: Vec<usize> = ents.iter().map(|e| w(&self.plain_name(e)) + w(&self.prefix(e, wi, wb)) + 3).collect();
                let (gc, gr, gw) = grid(&widths, cols);
                self.panes[p].grid = (gc, gr);
                if n == 0 && pane_listed && self.panes[p].raw.is_empty() {
                    out.push(vec![Span::new("(empty)", dim)]);
                }
                for line in 0..gr {
                    let mut row: Line = vec![];
                    let mut x = 0;
                    let mut cx = 0;
                    for (c, cw) in gw.iter().enumerate() {
                        let i = c * gr + line;
                        if i >= n {
                            break;
                        }
                        let e = &ents[i];
                        room(&mut row, x, cx);
                        let hit = LsHit::Entry(p, i);
                        let hov = live && hovered == Some(&hit);
                        let sel = live && self.panes[p].selected == e.path;
                        let bg = if sel { Some(sel_bg) } else if hov { hov_bg } else { None };
                        let rect = cw.saturating_sub(1).min(cols.saturating_sub(cx));
                        let pre = self.prefix(e, wi, wb);
                        if !pre.is_empty() {
                            row.push(Span::new(pre.clone(), dim.with_bg(bg)).hit(Hit::Ls(hit.clone())));
                        }
                        let mut st = tint(e);
                        st.underline = hov;
                        let t = elide(&format!("{} {}", icon(e), self.plain_name(e)), rect.saturating_sub(w(&pre)));
                        let tw = w(&pre) + w(&t);
                        if sel && focused && self.fpane == p && self.focus == Focus::Entry {
                            self.sel_at = Some((out.len(), cx));
                        }
                        row.push(Span::new(t, st.with_bg(bg)).hit(Hit::Ls(hit.clone())));
                        if live && rect > tw {
                            row.push(Span::new(" ".repeat(rect - tw), Sty { bg, ..Default::default() }).hit(Hit::Ls(hit.clone())));
                        }
                        // (Left in the scrollback there's no padding span: what
                        // was written ends at tw, and room() fills to the next
                        // column. Counting the padding there ran the columns together.)
                        x = cx + if live { rect.max(tw) } else { tw };
                        cx += cw;
                    }
                    if live {
                        room(&mut row, x, cols);
                    }
                    out.push(row);
                }
            }
            if live && !self.error.is_empty() && p == self.fpane.min(self.panes.len() - 1) {
                let t = elide(&self.error, cols.saturating_sub(2));
                let mut row: Line = vec![Span::new(" ".repeat(cols.saturating_sub(w(&t) + 1)), Sty::default()).hit(Hit::Ls(LsHit::Space(p)))];
                row.push(Span::new(t, Sty::fg(Col::Ansi(9))).hit(Hit::Ls(LsHit::Error)));
                out.push(row);
            }
        }
        // (Every pane collapsed: an ls of names that aren't there.)
        if live {
            for r in out.iter_mut() {
                let rw = super::width(r);
                if rw < cols {
                    r.push(Span::new(" ".repeat(cols - rw), Sty::default()).hit(Hit::Ls(LsHit::Space(self.fpane))));
                }
            }
        }
        out
    }

    fn long_w(&self, ents: &[Entry]) -> (usize, usize) {
        let mut wi = 1;
        let mut wb = 1;
        for e in ents {
            wi = wi.max(e.ino.map(|x| x.to_string()).unwrap_or("?".into()).len());
            wb = wb.max(e.blocks.map(|x| x.to_string()).unwrap_or("?".into()).len());
        }
        (wi, wb)
    }

    /// The table's columns: shown ones in their order, each as wide as its
    /// widest cell (or as sized by hand); the name's what's left.
    fn table_cols(&self, _p: usize, rows: &[(Entry, usize)], cols: usize) -> Vec<TCol> {
        let mut nat: HashMap<&str, usize> = [("name", 8), ("size", 4), ("date", if self.plan.opts.full_time { 19 } else { 24 }), ("mode", 11), ("nlink", 5), ("user", 5), ("group", 5), ("ino", 5), ("blocks", 6)].into_iter().collect();
        let ks: Vec<&str> = ["size", "user", "group", "nlink", "ino", "blocks"].into_iter().filter(|k| self.col_on(k)).collect();
        for (e, depth) in rows {
            let nl = depth * 2 + 5 + w(&self.plain_name(e));
            if nl > nat["name"] {
                nat.insert("name", nl);
            }
            for k in &ks {
                let v = w(&self.cell(e, k));
                if v > nat[k] {
                    nat.insert(k, v);
                }
            }
        }
        let mut order: Vec<&'static str> = self.col_order().iter().filter_map(|k| COLS.iter().find(|c| **c == k.as_str()).copied()).collect();
        for c in COLS {
            if !order.contains(&c) {
                order.push(c);
            }
        }
        order.retain(|k| self.col_on(k));
        let mut out: Vec<TCol> = order
            .into_iter()
            .map(|k| {
                let title = self.col_title(k);
                let wd = match self.sizing {
                    Some((s, sw)) if s == k => sw,
                    _ => {
                        let cw = self.col_w(k);
                        if cw > 0 {
                            cw
                        } else {
                            nat[k].max(w(&title) + 2)
                        }
                    }
                };
                TCol { key: k, title, right: ["size", "nlink", "ino", "blocks"].contains(&k), w: wd, x: 0 }
            })
            .collect();
        if let Some(ni) = out.iter().position(|c| c.key == "name") {
            if self.col_w("name") == 0 && !matches!(self.sizing, Some(("name", _))) {
                let rest: usize = out.iter().filter(|c| c.key != "name").map(|c| c.w + 2).sum();
                out[ni].w = out[ni].w.min(cols.saturating_sub(1 + rest + 2)).max(16);
            }
        }
        let mut x = 0;
        for c in out.iter_mut() {
            c.x = x;
            x += c.w + 2;
        }
        out
    }

    /// Where a dragged heading would go: before which column ("" the end).
    fn drop_before(&self, p: usize) -> Option<String> {
        let (dp, key, at) = self.dragging?;
        if dp != p {
            return None;
        }
        for c in &self.panes[p].tcols {
            if c.key != key && (at as usize) < 2 + c.x + c.w / 2 {
                return Some(c.key.to_string());
            }
        }
        Some(String::new())
    }

    /// The panel above the prompt: at most `room` rows (past that its own
    /// scrolling, a bar on the right), and the focused thing's place.
    pub fn panel(&mut self, pal: &Palette, cols: usize, room: usize, hovered: Option<&LsHit>, focused: bool) -> (Vec<Line>, Option<(usize, usize)>) {
        let room = room.max(3);
        self.room = room;
        let budget = self.scroll + room + 200;
        let mut rows = self.render(pal, cols, hovered, true, focused, budget);
        self.capped = rows.len() > room;
        if self.capped {
            rows = self.render(pal, cols.saturating_sub(1), hovered, true, focused, budget);
        }
        self.total = rows.len();
        if self.follow {
            if let Some((r, _)) = self.sel_at {
                if r < self.scroll {
                    self.scroll = r;
                } else if r >= self.scroll + room {
                    self.scroll = r + 1 - room;
                }
            }
            self.follow = false;
        }
        self.scroll = self.scroll.min(rows.len().saturating_sub(room));
        if !self.capped {
            self.scroll = 0;
            return (rows, self.sel_at);
        }
        let n = rows.len();
        let thumb = (room * room).div_ceil(n).max(1);
        let at = (self.scroll * room) / n.max(1);
        let mut out = vec![];
        for (i, mut r) in rows.into_iter().skip(self.scroll).take(room).enumerate() {
            let on = i >= at && i < at + thumb;
            let (t, st) = if on { ("┃", Sty::fg(Col::Ansi(7))) } else { ("│", pal.dimmed()) };
            r.push(Span::new(t, st).hit(Hit::Ls(LsHit::Bar(i < at))));
            out.push(r);
        }
        let sel = self.sel_at.and_then(|(r, c)| (r >= self.scroll && r < self.scroll + room).then(|| (r - self.scroll, c)));
        (out, sel)
    }

    /// As it's left in the scrollback: all of it, without its buttons.
    pub fn frozen(&mut self, pal: &Palette, cols: usize) -> Vec<Line> {
        self.dragging = None;
        self.sizing = None;
        let rows = self.render(pal, cols, None, false, false, 20_000);
        rows.into_iter().map(|r| r.into_iter().map(|mut s| { s.hit = Hit::None; s }).collect()).collect()
    }

    /// Something open over it (a name to type, delete to confirm, properties).
    pub fn dialog_open(&self) -> bool {
        self.ask.is_some() || self.confirm.is_some() || self.props.is_some()
    }

    /// That dialog, as a popup: its lines and where the cursor goes in it.
    pub fn dialog(&self, pal: &Palette, cols: usize) -> Option<(Vec<Line>, Option<(usize, usize)>)> {
        let inner = cols.saturating_sub(4).min(56).max(20);
        let dim = pal.dimmed();
        let mut lines: Vec<Line> = vec![];
        let mut cur = None;
        let d = |l: Line| -> Line { l.into_iter().map(|s| if s.hit == Hit::None { s.hit(Hit::Ls(LsHit::Dialog)) } else { s }).collect() };
        if let Some(a) = &self.ask {
            let title = match a.op {
                "rename" => "Rename",
                "mkdir" => "New folder",
                _ => "New file",
            };
            lines.push(d(vec![Span::new(format!(" {title}"), Sty::default().bold())]));
            let t = &a.field;
            let sel = t.selection();
            let mut row: Line = vec![Span::new(" ", Sty::default())];
            let fbg = pal.bg.map(|b| Col::Rgb(b.0, b.1, b.2));
            let mut fl: Line = vec![];
            match sel {
                Some((x, y)) => {
                    fl.push(Span::new(t.text[..x].to_string(), Sty { bg: fbg, ..Default::default() }));
                    fl.push(Span::new(t.text[x..y].to_string(), Sty { bg: Some(accent(pal, 0.45)), ..Default::default() }));
                    fl.push(Span::new(t.text[y..].to_string(), Sty { bg: fbg, ..Default::default() }));
                }
                None => fl.push(Span::new(t.text.clone(), Sty { bg: fbg, underline: pal.bg.is_none(), ..Default::default() })),
            }
            let fw = super::width(&fl);
            fl.push(Span::new(" ".repeat(inner.saturating_sub(fw + 2)), Sty { bg: fbg, underline: pal.bg.is_none(), ..Default::default() }));
            row.extend(fl);
            lines.push(d(row));
            cur = Some((lines.len(), 1 + 1 + w(&t.text[..t.cursor.min(t.text.len())])));
            if !self.error.is_empty() {
                for l in wrap(&self.error, inner.saturating_sub(2)) {
                    lines.push(d(vec![Span::new(format!(" {l}"), Sty::fg(Col::Ansi(9)))]));
                }
            }
            lines.push(d(vec![Span::new(" Enter: done · Esc: cancel", dim)]));
        } else if let Some((e, on)) = &self.confirm {
            let q = format!("Delete {}{} for good?", e.name, if e.dir { " and everything in it" } else { "" });
            for l in wrap(&q, inner.saturating_sub(2)) {
                lines.push(d(vec![Span::new(format!(" {l}"), Sty::default().bold())]));
            }
            for l in wrap("It doesn't go to the trash: it can't be brought back.", inner.saturating_sub(2)) {
                lines.push(d(vec![Span::new(format!(" {l}"), dim)]));
            }
            lines.push(d(vec![]));
            let b = |label: &str, name: &'static str, i: usize, danger: bool| {
                let mut st = Sty { bg: if *on == i { Some(if danger { danger_bg(pal) } else { accent(pal, 0.45) }) } else { pal.mix(0.14) }, ..Default::default() };
                if pal.bg.is_none() && *on == i {
                    st.underline = true;
                    st.bold = true;
                }
                Span::new(format!(" {label} "), st).hit(Hit::Ls(LsHit::Button(name)))
            };
            let bw = w(" Cancel ") + 1 + w(" Delete for good ");
            lines.push(d(vec![Span::new(" ".repeat(inner.saturating_sub(bw + 1)), Sty::default()), b("Cancel", "cancel", 0, false), Span::new(" ", Sty::default()), b("Delete for good", "delete", 1, true)]));
        } else if let Some(e) = &self.props {
            let mut st = tint(e);
            st.bold = true;
            lines.push(d(vec![Span::new(format!(" {} {}", icon(e), self.plain_name(e)), st)]));
            let rows = [
                ("Where", e.path.clone()),
                ("Kind", format!("{}{}", e.kind, if e.link { format!(" (link to {})", e.target) } else { String::new() })),
                ("Size", format!("{}{}", self.size_text(e), if e.dir { String::new() } else { format!(" ({} bytes)", e.size) })),
                ("Modified", when(e.mtime)),
                ("Permissions", e.mode.clone()),
                ("Owner", format!("{} : {}", e.user, e.group)),
            ];
            for (k, v) in rows {
                for (i, l) in wrap_any(&v, inner.saturating_sub(15)).into_iter().enumerate() {
                    lines.push(d(vec![Span::new(format!(" {:<12} ", if i == 0 { k } else { "" }), dim), Span::new(l, Sty::default())]));
                }
            }
        } else {
            return None;
        }
        let bg = pal.popup();
        let edge = Sty { fg: Some(Col::Ansi(8)), bg, ..Default::default() };
        let mut out = vec![vec![Span::new(format!("╭{}╮", "─".repeat(inner)), edge).hit(Hit::Ls(LsHit::Dialog))]];
        for l in lines {
            let lw = super::width(&l);
            let mut row = vec![Span::new("│", edge).hit(Hit::Ls(LsHit::Dialog))];
            row.extend(l.into_iter().map(|s| Span { sty: s.sty.with_bg(bg), ..s }));
            row.push(Span::new(" ".repeat(inner.saturating_sub(lw)), Sty { bg, ..Default::default() }).hit(Hit::Ls(LsHit::Dialog)));
            row.push(Span::new("│", edge).hit(Hit::Ls(LsHit::Dialog)));
            out.push(row);
        }
        out.push(vec![Span::new(format!("╰{}╯", "─".repeat(inner)), edge).hit(Hit::Ls(LsHit::Dialog))]);
        Some((out, cur.map(|(r, c): (usize, usize)| (r, c + 1))))
    }

    // ---- what's where -------------------------------------------------------------------
    fn item(&mut self, p: usize, i: usize) -> Option<Entry> {
        if p >= self.panes.len() {
            return None;
        }
        self.items(p).get(i).cloned()
    }
    fn selected_entry(&mut self) -> Option<Entry> {
        let p = self.fpane.min(self.panes.len().saturating_sub(1));
        let s = self.panes.get(p)?.selected.clone();
        if s.is_empty() {
            return None;
        }
        self.items(p).into_iter().find(|e| e.path == s)
    }
    pub fn capped(&self) -> bool {
        self.capped
    }
    pub fn dragging(&self) -> bool {
        self.press.is_some()
    }
}

fn parent(p: &str) -> String {
    match p.rfind('/') {
        Some(0) | None => "/".into(),
        Some(i) => p[..i].to_string(),
    }
}

/// ls's column layout in cells: (columns, rows, each column's width).
pub fn grid(widths: &[usize], avail: usize) -> (usize, usize, Vec<usize>) {
    let n = widths.len();
    if n == 0 {
        return (1, 0, vec![0]);
    }
    let minw = *widths.iter().min().unwrap();
    let maxw = *widths.iter().max().unwrap();
    let avail = avail.max(10);
    let mut best = (1, n, vec![maxw + 2]);
    let most = n.min(avail / (minw + 2).max(1));
    for cols in 2..=most {
        let rows = n.div_ceil(cols);
        if n.div_ceil(rows) < cols {
            continue;
        }
        let mut ws = vec![];
        let mut total = 0;
        for c in 0..cols {
            if total > avail {
                break;
            }
            let m = widths[c * rows..((c + 1) * rows).min(n)].iter().copied().max().unwrap_or(0);
            ws.push(m + 2);
            total += m + 2;
        }
        if total > avail || ws.len() < cols {
            continue;
        }
        best = (cols, rows, ws);
    }
    best
}

fn accent(pal: &Palette, t: f32) -> Col {
    match pal.bg {
        Some(b) => {
            let a = (0x61u8, 0xafu8, 0xefu8);
            let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
            Col::Rgb(m(b.0, a.0), m(b.1, a.1), m(b.2, a.2))
        }
        None => Col::Ansi(8),
    }
}
fn danger_bg(pal: &Palette) -> Col {
    match pal.bg {
        Some(b) => {
            let a = (0xe0u8, 0x6cu8, 0x75u8);
            let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * 0.45).round() as u8;
            Col::Rgb(m(b.0, a.0), m(b.1, a.1), m(b.2, a.2))
        }
        None => Col::Ansi(1),
    }
}

fn wrap(s: &str, n: usize) -> Vec<String> {
    let mut out = vec![];
    let mut line = String::new();
    for word in s.split(' ') {
        if !line.is_empty() && w(&line) + 1 + w(word) > n {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    out.push(line);
    out
}
fn wrap_any(s: &str, n: usize) -> Vec<String> {
    let n = n.max(4);
    let mut out = vec![];
    let mut line = String::new();
    for ch in s.chars() {
        if w(&line) + 1 > n {
            out.push(std::mem::take(&mut line));
        }
        line.push(ch);
    }
    out.push(line);
    out
}

fn nix_help(prog: &str) -> String {
    format!("{prog} isn't installed, and neither is Nix, which gets it when it's needed: https://nixos.org/download")
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
}

/// The desktop's clipboard, files on it: (cut, paths). Wayland's wl-paste, else X's xclip.
fn desktop_files() -> Option<(bool, Vec<String>)> {
    let run = |prog: &str, args: &[&str]| -> Option<String> {
        if !on_path(prog) {
            return None;
        }
        let mut c = std::process::Command::new(prog);
        c.args(args).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null());
        let child = c.stdout(std::process::Stdio::piped()).spawn().ok()?;
        // (A clipboard owner that doesn't answer: given up on.)
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait_with_output());
        });
        let o = rx.recv_timeout(Duration::from_millis(800)).ok()?.ok()?;
        o.status.success().then(|| String::from_utf8_lossy(&o.stdout).to_string())
    };
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let get = |t: &str| if wayland { run("wl-paste", &["-n", "-t", t]) } else if std::env::var_os("DISPLAY").is_some() { run("xclip", &["-o", "-selection", "clipboard", "-t", t]) } else { None };
    let types = if wayland { run("wl-paste", &["-l"]) } else { run("xclip", &["-o", "-selection", "clipboard", "-t", "TARGETS"]) }?;
    let uris = |t: &str| -> Vec<String> {
        t.lines()
            .filter(|l| l.starts_with("file://"))
            .map(|l| {
                let p = l.trim_start_matches("file://");
                let p = p.find('/').map(|i| &p[i..]).unwrap_or(p);
                pct_decode(p.trim_end_matches('\r'))
            })
            .collect()
    };
    if types.lines().any(|t| t == "x-special/gnome-copied-files") {
        let t = get("x-special/gnome-copied-files")?;
        return Some((t.lines().next() == Some("cut"), uris(&t)));
    }
    if types.lines().any(|t| t == "text/uri-list") {
        let t = get("text/uri-list")?;
        let cut = types.lines().any(|t| t == "application/x-kde-cutselection") && get("application/x-kde-cutselection").is_some_and(|c| c.trim() == "1");
        return Some((cut, uris(&t)));
    }
    Some((false, vec![]))
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() + 0 && i + 2 <= b.len() - 1 {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn pct_encode(p: &str) -> String {
    p.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~/".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// Files onto the desktop's clipboard (a file manager pastes them).
fn put_desktop_files(paths: &[String], cut: bool) {
    let list: String = paths.iter().map(|p| format!("file://{}\r\n", pct_encode(p))).collect();
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let (prog, args): (&str, Vec<&str>) = if wayland && on_path("wl-copy") {
        ("wl-copy", vec!["-t", "text/uri-list"])
    } else if std::env::var_os("DISPLAY").is_some() && on_path("xclip") {
        ("xclip", vec!["-i", "-selection", "clipboard", "-t", "text/uri-list"])
    } else {
        return;
    };
    let _ = cut;
    if let Ok(mut c) = std::process::Command::new(prog).args(&args).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn() {
        use std::io::Write;
        if let Some(mut i) = c.stdin.take() {
            let _ = i.write_all(list.as_bytes());
        }
        let _ = c.wait();
    }
}

/// How long the pointer rests before a tooltip, as Tessera's.
pub fn tip_delay(h: &LsHit) -> u64 {
    match h {
        LsHit::Entry(..) => 900,
        LsHit::Head(..) => 800,
        LsHit::Arrow(..) | LsHit::Chip(_) => 600,
        _ => 550,
    }
}

/// The keys, for the ⌨ button's tooltip.
pub fn keys_help() -> String {
    [
        "The listing, by keys (Ctrl+Alt+L goes in; again, or Esc, back to the editor)",
        "  ↑ ↓ ← →           Move (up past the first: the headings, the path, the buttons)",
        "  Enter             Open: a folder goes in, a text file shows, others open in their app",
        "  Shift+F10, Menu   Its menu (on the buttons, the path or the headings: the folder's)",
        "  Backspace, Alt+←  Back        Alt+→  Forward        Alt+↑  Up a folder",
        "  F2  Del  Shift+Del  Rename, move to the trash, delete for good",
        "  Ctrl+C Ctrl+X Ctrl+V  Copy, cut, paste      F5  Read it again",
        "  PgUp PgDn Home End  Further      A letter: the next name starting with it",
        "On a heading: Enter sorts by it (again: the other way), Shift+Enter: then by it,",
        "  Alt+← Alt+→ moves it, + and - size it, = fits it, Shift+F10: the columns",
        "In the table: → opens a folder in place, ← folds it (or goes to its folder)",
    ]
    .join("\n")
}

/// A tooltip's text, as Tessera's.
pub fn tooltip(ed: &Ed, h: &LsHit) -> String {
    let Some(ls) = ed.ls.as_ref() else { return String::new() };
    match h {
        LsHit::Entry(p, i) => {
            if ls.view() == "details" {
                return String::new();
            }
            let e = match ls.panes.get(*p).and_then(|x| x.cache.as_ref()).and_then(|c| c.get(*i)) {
                Some(e) => e,
                None => return String::new(),
            };
            format!("{}  {} {}  {}\n{}{}", e.mode, e.user, e.group, ls.size_text(e), when(e.mtime), if e.link { format!("\n→ {}", e.target) } else { String::new() })
        }
        LsHit::Arrow(p, i) => {
            let open = ls.panes.get(*p).and_then(|x| x.rows_cache.as_ref()).and_then(|r| r.get(*i)).is_some_and(|(e, _)| ls.is_open(*p, &e.path));
            if open { "Fold it (←)" } else { "Show what's in it, here (→)" }.into()
        }
        LsHit::Head(_, k) => {
            let t = ls.col_title(k).to_lowercase();
            format!("Sort by {t} (again: the other way; Shift+click: then by it). Drag to move it; right-click for the columns.")
        }
        LsHit::Chip("keys") => keys_help(),
        LsHit::Chip(n) => ls.chips().into_iter().find(|c| c.0 == *n).map(|c| c.3).unwrap_or_default(),
        LsHit::Error => format!("{} (click to dismiss)", ls.error),
        _ => String::new(),
    }
}

fn item(label: &str, cmd: &str) -> MenuItem {
    MenuItem { label: label.into(), cmd: cmd.into(), section: false, checked: false, enabled: true, key: String::new() }
}
fn sep() -> MenuItem {
    MenuItem { label: "────────────".into(), section: true, ..Default::default() }
}

// ---- the editor's side: keys, the mouse, menus ------------------------------------------

impl Ed {
    /// A new ls's listing above the prompt (the last one goes to the scrollback first).
    pub fn show_ls(&mut self, ls: Ls) {
        self.ls = Some(Box::new(ls));
        self.ls_focus = false;
        self.dirty = true;
    }

    /// What the background finished: the listing read again, menus updated.
    pub fn ls_tick(&mut self) {
        let Some(ls) = self.ls.as_mut() else { return };
        let (any, menus) = ls.take_done();
        if any {
            self.dirty = true;
        }
        if menus {
            if let Some(MenuKind::Ls(k)) = self.menu.as_ref().map(|m| m.kind.clone()) {
                let sel = self.menu.as_ref().map(|m| m.sel).unwrap_or(0);
                if let Some(items) = self.ls_menu_items(&k) {
                    let n = items.len();
                    if let Some(m) = self.menu.as_mut() {
                        m.items = items;
                        m.sel = sel.min(n.saturating_sub(1));
                    }
                }
            }
        }
    }

    fn ls_menu_items(&mut self, which: &str) -> Option<Vec<MenuItem>> {
        let ls = self.ls.as_mut()?;
        let it = item;
        Some(match which {
            "item" => {
                let (_, e) = ls.menu_entry.clone()?;
                let apps = !ls.apps.is_empty();
                let kind = if e.dir {
                    "folders".to_string()
                } else if !ext(&e).is_empty() {
                    format!(".{} files", ext(&e))
                } else if !ls.apps_mime.is_empty() {
                    format!("{} files", ls.apps_mime)
                } else {
                    "these".into()
                };
                let archive = !e.dir && {
                    let n = e.name.to_lowercase();
                    [".tar", ".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".tgz", ".zip", ".7z", ".rar"].iter().any(|x| n.ends_with(x))
                };
                let mut v = vec![it("Open", "open"), MenuItem { enabled: apps, ..it("Open with ▸", "openwith") }, MenuItem { enabled: apps, ..it(&format!("Always open {kind} with ▸"), "always") }];
                if !e.dir {
                    v.push(it("View in a card", "card"));
                    v.push(it("View in the viewer", "viewer"));
                }
                v.push(it(if e.dir { "Open in the file manager" } else { "Show in the file manager" }, "reveal"));
                if e.dir || e.exec {
                    v.push(it(if e.dir { "Go here in the shell (cd)" } else { "Run it" }, "cd"));
                }
                v.push(sep());
                v.push(it("Cut", "cut"));
                v.push(it("Copy", "copy"));
                if e.dir {
                    v.push(MenuItem { enabled: ls.can_paste(), ..it("Paste into it", "pasteinto") });
                }
                v.push(it("Copy the path", "copypath"));
                v.push(it("Copy the name", "copyname"));
                v.push(it("Insert the path in the editor", "insert"));
                v.push(sep());
                v.push(it("Rename…", "rename"));
                v.push(it("Duplicate", "duplicate"));
                if !e.dir {
                    v.push(it(if e.exec { "Not a program (chmod -x)" } else { "Make it a program (chmod +x)" }, "chmod"));
                }
                v.push(it("Compress ▸", "compress"));
                if archive {
                    v.push(it("Extract here", "extract"));
                }
                if e.dir {
                    v.push(it("Disk usage…", "du"));
                }
                v.push(sep());
                v.push(it("Move to the trash", "trash"));
                v.push(it("Delete for good…", "delete"));
                v.push(sep());
                v.push(it("Properties", "props"));
                v
            }
            "apps" => ls.apps.iter().map(|(id, name, d)| it(&format!("{name}{}", if *d { "  (default)" } else { "" }), &format!("app:{id}"))).collect(),
            "always" => ls.apps.iter().map(|(id, name, d)| MenuItem { checked: *d, ..it(name, &format!("default:{id}")) }).collect(),
            "compress" => {
                let (_, e) = ls.menu_entry.clone()?;
                vec![it(&format!("To {}.tar.gz", e.name), "tgz"), it(&format!("To {}.zip", e.name), "zip")]
            }
            "folder" => {
                let v = ls.view();
                vec![
                    it("Open in the file manager", "fopen"),
                    it("Go here in the shell (cd)", "fcd"),
                    sep(),
                    it("New folder…", "mkdir"),
                    it("New file…", "touch"),
                    MenuItem { enabled: ls.can_paste(), ..it("Paste", "fpaste") },
                    it("Copy the folder's path", "fcopypath"),
                    it("Disk usage…", "fdu"),
                    sep(),
                    MenuItem { checked: v == "list", ..it("List (as ls)", "view:list") },
                    MenuItem { checked: v == "details", ..it("Details (as ls -l)", "view:details") },
                    it("Sort…", "sort"),
                    MenuItem { checked: ls.folders_first(), ..it("Folders first", "first") },
                    MenuItem { checked: ls.show_hidden(ls.fpane), ..it("Show hidden files", "hidden") },
                    it("Refresh", "refresh"),
                ]
            }
            "sort" => {
                let s = ls.sort_by();
                let mut v: Vec<MenuItem> = [("name", "Name"), ("size", "Size"), ("date", "Date modified"), ("type", "Type")].iter().map(|(k, l)| MenuItem { checked: s == *k, ..it(l, &format!("sort:{k}")) }).collect();
                v.push(MenuItem { checked: ls.desc(), ..it("Descending", "desc") });
                v
            }
            "columns" => {
                let mut v: Vec<MenuItem> = COLS.iter().filter(|k| **k != "name").map(|k| MenuItem { checked: ls.col_on(k), ..it(&ls.col_title(k), &format!("col:{k}")) }).collect();
                v.push(sep());
                v.push(it("Fit every column", "fit"));
                v.push(it("Columns as ls -l has them", "lscols"));
                v.push(MenuItem { enabled: !ls.sort_keys().is_empty(), ..it("Sort as the command did", "cmdsort") });
                v
            }
            _ => return None,
        })
    }

    fn ls_open_menu(&mut self, which: &str, x: Option<usize>) {
        if which == "item" {
            if let Some(ls) = self.ls.as_mut() {
                if let Some((_, e)) = ls.menu_entry.clone() {
                    ls.load_apps(&e.path);
                }
            }
        }
        if which == "item" || which == "folder" {
            if let Some(ls) = self.ls.as_ref() {
                ls.probe_clip();
            }
        }
        if let Some(items) = self.ls_menu_items(which) {
            if items.is_empty() {
                return;
            }
            self.tooltip = None;
            self.menu_x = x.or(self.menu_x);
            self.open_menu(MenuKind::Ls(which.into()), items);
        }
    }

    /// Opening an entry: a folder goes in; a text file that isn't a program
    /// shows (the viewer); anything else opens with its app.
    fn ls_open_entry(&mut self, p: usize, e: &Entry) {
        let Some(ls) = self.ls.as_mut() else { return };
        if e.dir {
            ls.go(p, &e.path);
        } else if e.text && !e.exec || !std::env::var_os("DISPLAY").or(std::env::var_os("WAYLAND_DISPLAY")).is_some() {
            // (No desktop to open it with: the viewer shows what it can.)
            self.done = Some(Outcome::Host(format!("lsview:{}", e.path)));
        } else {
            ls.act("open", &e.path, "", Then::Nothing);
        }
    }

    fn ls_du(&mut self, path: &str) {
        let q = crate::repl::quote_path(path);
        if on_path("gdu") {
            self.done = Some(Outcome::Host(format!("run:gdu {q}")));
        } else if on_path("nix") {
            self.done = Some(Outcome::Host(format!("run:nix run nixpkgs#gdu -- {q}")));
        } else if let Some(ls) = self.ls.as_mut() {
            ls.error = nix_help("gdu");
        }
    }

    fn ls_ask(&mut self, op: &'static str, e: Option<Entry>) {
        let Some(ls) = self.ls.as_mut() else { return };
        let mut field = Text::new();
        let name = match op {
            "rename" => e.as_ref().map(|e| e.name.clone()).unwrap_or_default(),
            "mkdir" => "New Folder".into(),
            _ => "New File.txt".into(),
        };
        field.set(&name);
        // (The name without its extension selected, as file managers do.)
        let end = match name.rfind('.') {
            Some(d) if op != "mkdir" && d > 0 && !e.as_ref().is_some_and(|e| e.dir) => d,
            _ => name.len(),
        };
        field.select(0, end);
        ls.error.clear();
        ls.ask = Some(Ask { op, entry: e, field, pane: ls.fpane });
    }

    /// A menu item picked.
    pub fn ls_pick(&mut self, cmd: &str) {
        self.dirty = true;
        let Some(ls) = self.ls.as_mut() else { return };
        let me = ls.menu_entry.clone();
        let p = me.as_ref().map(|m| m.0).unwrap_or(ls.fpane).min(ls.panes.len().saturating_sub(1));
        let dir = ls.panes.get(p).map(|x| x.dir.clone()).unwrap_or_default();
        let e = me.map(|m| m.1);
        let (c, arg) = cmd.split_once(':').unwrap_or((cmd, ""));
        match (c, e) {
            ("open", Some(e)) => self.ls_open_entry(p, &e),
            ("openwith", _) => self.ls_open_menu("apps", None),
            ("always", _) => self.ls_open_menu("always", None),
            ("compress", _) => self.ls_open_menu("compress", None),
            ("app", Some(e)) => ls.act("open_with", &e.path, arg, Then::Nothing),
            ("default", Some(e)) => ls.act("set_default", &e.path, arg, Then::Apps(e.path.clone())),
            ("card", Some(e)) => self.done = Some(Outcome::Host(format!("lscard:{}", e.path))),
            ("viewer", Some(e)) => self.done = Some(Outcome::Host(format!("lsview:{}", e.path))),
            ("reveal", Some(e)) => ls.act(if e.dir { "open" } else { "reveal" }, &e.path, "", Then::Nothing),
            ("cd", Some(e)) => {
                let q = crate::repl::quote_path(&e.path);
                self.done = Some(Outcome::Host(if e.dir { format!("lscd:{}", e.path) } else { format!("run:{q}") }));
            }
            ("cut" | "copy", Some(e)) => ls.copy_file(&e.path, c == "cut"),
            ("pasteinto", Some(e)) => ls.paste_into(&e.path),
            ("copypath", Some(e)) => super::term::Term::copy(&e.path),
            ("copyname", Some(e)) => super::term::Term::copy(&e.name),
            ("insert", Some(e)) => {
                self.t.insert(&format!("{} ", crate::repl::quote_path(&e.path)));
                self.ls_focus = false;
                self.changed();
            }
            ("rename", Some(e)) => self.ls_ask("rename", Some(e)),
            ("duplicate", Some(e)) => ls.act("duplicate", &e.path, "", Then::Nothing),
            ("chmod", Some(e)) => ls.act("chmod_x", &e.path, "", Then::Nothing),
            ("tgz", Some(e)) => ls.act("compress", &e.path, "tar.gz", Then::Nothing),
            ("zip", Some(e)) => ls.act("compress", &e.path, "zip", Then::Nothing),
            ("extract", Some(e)) => ls.act("extract", &e.path, "", Then::Nothing),
            ("du", Some(e)) => self.ls_du(&e.path),
            ("trash", Some(e)) => ls.act("trash", &e.path, "", Then::Nothing),
            ("delete", Some(e)) => ls.confirm = Some((e, 0)),
            ("props", Some(e)) => ls.props = Some(e),
            ("fopen", _) => ls.act("open", &dir, "", Then::Nothing),
            ("fcd", _) => self.done = Some(Outcome::Host(format!("lscd:{dir}"))),
            ("mkdir", _) => self.ls_ask("mkdir", None),
            ("touch", _) => self.ls_ask("touch", None),
            ("fpaste", _) => ls.paste_into(&dir),
            ("fcopypath", _) => super::term::Term::copy(&dir),
            ("fdu", _) => self.ls_du(&dir),
            ("view", _) => ls.set_view(arg),
            ("sort", _) if arg.is_empty() => self.ls_open_menu("sort", None),
            ("sort", _) => ls.save("sort", json!(arg)),
            ("desc", _) => {
                let d = ls.desc();
                ls.save("desc", json!(!d));
            }
            ("first", _) => {
                let f = ls.folders_first();
                ls.save("folders_first", json!(!f));
            }
            ("hidden", _) => {
                let h = ls.mine.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false);
                ls.save("hidden", json!(!h));
                ls.refresh();
            }
            ("refresh", _) => ls.refresh(),
            ("col", _) => {
                let on = ls.col_on(arg);
                ls.set_col_on(arg, !on);
            }
            ("fit", _) => ls.save("colW", json!({})),
            ("lscols", _) => {
                ls.save("colShown", json!({}));
                ls.save("colOrder", json!([]));
                ls.save("colW", json!({}));
            }
            ("cmdsort", _) => ls.save("sortKeys", json!([])),
            _ => {}
        }
    }

    /// A button of the listing's toolbar.
    fn ls_chip(&mut self, name: &str, x: Option<usize>) {
        let Some(ls) = self.ls.as_mut() else { return };
        match name {
            "list" | "details" => ls.set_view(name),
            "columns" => self.ls_open_menu("columns", x),
            "first" => {
                let f = ls.folders_first();
                ls.save("folders_first", json!(!f));
            }
            "hidden" => {
                let h = ls.mine.get("hidden").and_then(|v| v.as_bool()).unwrap_or(false);
                ls.save("hidden", json!(!h));
                ls.refresh();
            }
            "keys" => {
                self.ls_focus = !self.ls_focus;
                if self.ls_focus {
                    self.ls_focus_start();
                }
            }
            _ => {}
        }
    }

    fn ls_nav(&mut self, p: usize, hit: &LsHit) {
        let Some(ls) = self.ls.as_mut() else { return };
        match hit {
            LsHit::Nav(_, "back") => ls.go_back(p),
            LsHit::Nav(_, "forward") => ls.go_forward(p),
            LsHit::Nav(_, "up") => {
                if ls.panes[p].dir != "/" {
                    ls.up(p)
                }
            }
            LsHit::Nav(_, "cd") => {
                let d = ls.panes[p].dir.clone();
                self.done = Some(Outcome::Host(format!("lscd:{d}")));
            }
            LsHit::Crumb(_, path) => ls.go(p, path),
            _ => {}
        }
    }

    /// Into the listing by keys: the selection, else the first entry shown.
    fn ls_focus_start(&mut self) {
        let Some(ls) = self.ls.as_mut() else { return };
        ls.follow = true;
        if let Some(p) = (0..ls.panes.len()).find(|&p| !ls.panes[p].selected.is_empty()) {
            ls.fpane = p;
            ls.focus = Focus::Entry;
            return;
        }
        for p in 0..ls.panes.len() {
            let it = ls.items(p);
            if let Some(e) = it.first() {
                ls.fpane = p;
                ls.focus = Focus::Entry;
                ls.panes[p].selected = e.path.clone();
                return;
            }
        }
        ls.fpane = 0;
        ls.focus = Focus::Chip(0);
    }

    /// A key, when the listing has it (or Ctrl+Alt+L, or a dialog's open):
    /// whether it was the listing's.
    pub fn ls_key(&mut self, k: &KeyEvent) -> bool {
        if self.ls.is_none() {
            self.ls_focus = false;
            return false;
        }
        let m = k.modifiers;
        let (ctrl, alt, shift) = (m.contains(KeyModifiers::CONTROL), m.contains(KeyModifiers::ALT), m.contains(KeyModifiers::SHIFT));
        // A dialog's keys.
        if self.ls.as_ref().unwrap().dialog_open() {
            self.ls_dialog_key(k);
            return true;
        }
        if ctrl && alt && matches!(k.code, KeyCode::Char('l') | KeyCode::Char('L')) {
            self.ls_focus = !self.ls_focus;
            if self.ls_focus {
                self.close_comp();
                self.ls_focus_start();
            }
            return true;
        }
        if !self.ls_focus {
            return false;
        }
        self.dirty = true;
        let ls = self.ls.as_mut().unwrap();
        ls.follow = true;
        // (Type-ahead: what's typed in a row, until another key.)
        if !matches!(k.code, KeyCode::Char(_)) || ctrl || alt {
            ls.typed.0.clear();
        }
        if ls.panes.is_empty() {
            self.ls_focus = false;
            return true;
        }
        let p = ls.fpane.min(ls.panes.len() - 1);
        let view = ls.view();
        match ls.focus.clone() {
            Focus::Chip(i) => {
                let chips = ls.chips();
                match k.code {
                    KeyCode::Left => ls.focus = Focus::Chip(i.saturating_sub(1)),
                    KeyCode::Right => ls.focus = Focus::Chip((i + 1).min(chips.len() - 1)),
                    KeyCode::Down => self.ls_step(1),
                    KeyCode::Up => self.ls_step(-1),
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(c) = chips.get(i) {
                            let x = ls.sel_at.map(|s| s.1);
                            self.ls_chip(c.0, x);
                        }
                    }
                    KeyCode::F(10) if shift => self.ls_folder_menu(p, None),
                    KeyCode::Menu => self.ls_folder_menu(p, None),
                    _ => return self.ls_common_key(k, p),
                }
            }
            Focus::Nav(i) => {
                let items = ls.nav_items(p);
                match k.code {
                    KeyCode::Left => ls.focus = Focus::Nav(i.saturating_sub(1)),
                    KeyCode::Right => ls.focus = Focus::Nav((i + 1).min(items.len() - 1)),
                    KeyCode::Down => self.ls_step(1),
                    KeyCode::Up => self.ls_step(-1),
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some((_, hit, on)) = items.get(i).cloned() {
                            if on {
                                self.ls_nav(p, &hit);
                                if let Some(ls) = self.ls.as_mut() {
                                    let n = ls.nav_items(p).len();
                                    if let Focus::Nav(j) = ls.focus {
                                        ls.focus = Focus::Nav(j.min(n - 1));
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::F(10) if shift => self.ls_folder_menu(p, None),
                    KeyCode::Menu => self.ls_folder_menu(p, None),
                    _ => return self.ls_common_key(k, p),
                }
            }
            Focus::Head(i) => {
                let tc = ls.panes[p].tcols.clone();
                let n = tc.len().max(1);
                let key = tc.get(i.min(n - 1)).map(|c| c.key);
                match k.code {
                    KeyCode::Left if alt => {
                        if let Some(key) = key {
                            if i > 0 {
                                let before = tc[i - 1].key;
                                ls.move_col(key, before);
                                ls.focus = Focus::Head(i - 1);
                            }
                        }
                    }
                    KeyCode::Right if alt => {
                        if let Some(key) = key {
                            if i + 1 < n {
                                let before = tc.get(i + 2).map(|c| c.key).unwrap_or("");
                                ls.move_col(key, before);
                                ls.focus = Focus::Head(i + 1);
                            }
                        }
                    }
                    KeyCode::Left => ls.focus = Focus::Head(i.saturating_sub(1)),
                    KeyCode::Right => ls.focus = Focus::Head((i + 1).min(n - 1)),
                    KeyCode::Down => self.ls_step(1),
                    KeyCode::Up => self.ls_step(-1),
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if let Some(key) = key {
                            ls.sort_on(key, shift);
                        }
                    }
                    KeyCode::Char('+') | KeyCode::Char('-') => {
                        if let Some(c) = tc.get(i) {
                            let wd = if k.code == KeyCode::Char('+') { c.w + 1 } else { c.w.saturating_sub(1).max(3) };
                            ls.set_col_w(c.key, wd);
                        }
                    }
                    KeyCode::Char('=') => {
                        if let Some(key) = key {
                            ls.set_col_w(key, 0);
                        }
                    }
                    KeyCode::F(10) if shift => {
                        let x = ls.sel_at.map(|s| s.1);
                        self.ls_open_menu("columns", x);
                    }
                    KeyCode::Menu => {
                        let x = ls.sel_at.map(|s| s.1);
                        self.ls_open_menu("columns", x);
                    }
                    _ => return self.ls_common_key(k, p),
                }
            }
            Focus::Entry => {
                let items = ls.items(p);
                let cur = items.iter().position(|e| e.path == ls.panes[p].selected);
                let n = items.len();
                let (gc, gr) = ls.panes[p].grid;
                let set = |ls: &mut Ls, i: usize| {
                    if let Some(e) = items.get(i) {
                        ls.panes[p].selected = e.path.clone();
                    }
                };
                match k.code {
                    KeyCode::Down if !alt => match cur {
                        Some(i) if i + 1 < n => set(ls, i + 1),
                        None if n > 0 => set(ls, 0),
                        _ => self.ls_step(1),
                    },
                    KeyCode::Up if !alt => match cur {
                        Some(i) if i > 0 => set(ls, i - 1),
                        _ => self.ls_step(-1),
                    },
                    KeyCode::Right if !alt => {
                        if view == "details" {
                            if let Some(i) = cur {
                                let e = items[i].clone();
                                if e.dir && !ls.is_open(p, &e.path) {
                                    ls.toggle_open(p, &e);
                                } else if i + 1 < n {
                                    set(ls, i + 1);
                                }
                            }
                        } else if view == "list" && gc > 1 {
                            if let Some(i) = cur {
                                if i + gr < n {
                                    set(ls, i + gr);
                                }
                            }
                        } else if let Some(i) = cur {
                            if i + 1 < n {
                                set(ls, i + 1);
                            }
                        }
                    }
                    KeyCode::Left if !alt => {
                        if view == "details" {
                            if let Some(i) = cur {
                                let e = items[i].clone();
                                if e.dir && ls.is_open(p, &e.path) {
                                    ls.toggle_open(p, &e);
                                } else {
                                    // (Its folder's row.)
                                    let rows = ls.rows(p, ls.room + ls.scroll + 200);
                                    let d = rows.get(i).map(|r| r.1).unwrap_or(0);
                                    if d > 0 {
                                        if let Some(j) = (0..i).rev().find(|&j| rows[j].1 < d) {
                                            set(ls, j);
                                        }
                                    }
                                }
                            }
                        } else if view == "list" && gc > 1 {
                            if let Some(i) = cur {
                                if i >= gr {
                                    set(ls, i - gr);
                                }
                            }
                        } else if let Some(i) = cur {
                            if i > 0 {
                                set(ls, i - 1);
                            }
                        }
                    }
                    KeyCode::Home => set(ls, 0),
                    KeyCode::End => set(ls, n.saturating_sub(1)),
                    KeyCode::PageDown | KeyCode::PageUp => {
                        let r = ls.room.max(2) - 1;
                        let to = match cur {
                            Some(i) if k.code == KeyCode::PageDown => (i + r).min(n.saturating_sub(1)),
                            Some(i) => i.saturating_sub(r),
                            None => 0,
                        };
                        set(ls, to);
                    }
                    KeyCode::Enter => {
                        if let Some(i) = cur {
                            let e = items[i].clone();
                            self.ls_open_entry(p, &e);
                        }
                    }
                    KeyCode::F(10) if shift => self.ls_entry_menu(p, cur, None),
                    KeyCode::Menu => self.ls_entry_menu(p, cur, None),
                    KeyCode::F(2) => {
                        if let Some(i) = cur {
                            self.ls_ask("rename", Some(items[i].clone()));
                        }
                    }
                    KeyCode::Delete => {
                        if let Some(i) = cur {
                            let e = items[i].clone();
                            if shift {
                                ls.confirm = Some((e, 0));
                            } else {
                                ls.act("trash", &e.path, "", Then::Nothing);
                            }
                        }
                    }
                    KeyCode::Char(c) if (c == 'c' || c == 'x') && ctrl && !alt => {
                        if let Some(i) = cur {
                            let path = items[i].path.clone();
                            ls.copy_file(&path, c == 'x');
                        }
                    }
                    KeyCode::Char(c) if !ctrl && !alt && !c.is_control() && c != ' ' => {
                        // (Typing: the next name starting with what's typed.)
                        let now = Instant::now();
                        if now.duration_since(ls.typed.1) > Duration::from_millis(1000) {
                            ls.typed.0.clear();
                        }
                        ls.typed.0.extend(c.to_lowercase());
                        ls.typed.1 = now;
                        let t = ls.typed.0.clone();
                        let start = if t.chars().count() == 1 { cur.map(|i| i + 1).unwrap_or(0) } else { cur.unwrap_or(0) };
                        if let Some(j) = (0..n).map(|k| (start + k) % n).find(|&j| items[j].name.to_lowercase().starts_with(&t)) {
                            set(ls, j);
                        }
                    }
                    _ => return self.ls_common_key(k, p),
                }
            }
        }
        true
    }

    /// Keys that work wherever the listing's focus is.
    fn ls_common_key(&mut self, k: &KeyEvent, p: usize) -> bool {
        let m = k.modifiers;
        let (ctrl, alt) = (m.contains(KeyModifiers::CONTROL), m.contains(KeyModifiers::ALT));
        let ls = self.ls.as_mut().unwrap();
        match k.code {
            KeyCode::Esc | KeyCode::Tab => {
                self.ls_focus = false;
            }
            KeyCode::Backspace => ls.go_back(p),
            KeyCode::Left if alt => ls.go_back(p),
            KeyCode::Right if alt => ls.go_forward(p),
            KeyCode::Up if alt => {
                if ls.panes[p].dir != "/" {
                    ls.up(p)
                }
            }
            KeyCode::F(5) => ls.refresh(),
            KeyCode::Char('v') | KeyCode::Char('V') if ctrl && !alt => {
                let d = ls.selected_entry().filter(|e| e.dir && ls.focus == Focus::Entry).map(|e| e.path).unwrap_or_else(|| ls.panes[p].dir.clone());
                ls.paste_into(&d);
            }
            KeyCode::PageDown | KeyCode::PageUp => {
                let r = ls.room.max(2) - 1;
                ls.scroll = if k.code == KeyCode::PageDown { ls.scroll + r } else { ls.scroll.saturating_sub(r) };
                ls.follow = false;
            }
            // (Anything else: the editor's again, the key with it.)
            KeyCode::Char(_) if !ctrl && !alt => {
                self.ls_focus = false;
                return false;
            }
            _ => {}
        }
        true
    }

    /// The focus a step up or down past the end of what it's on: entries,
    /// the table's headings, the path, the buttons, the panes before and after.
    fn ls_step(&mut self, d: i32) {
        let Some(ls) = self.ls.as_mut() else { return };
        let details = ls.view() == "details";
        // The stops, in order: (pane, focus).
        let mut stops: Vec<(usize, Focus)> = vec![];
        for p in 0..ls.panes.len() {
            if ls.panes[p].show_bar {
                stops.push((p, Focus::Chip(0)));
            }
            if ls.panes[p].navigated() {
                stops.push((p, Focus::Nav(0)));
            }
            if details {
                stops.push((p, Focus::Head(0)));
            }
            if !ls.items(p).is_empty() {
                stops.push((p, Focus::Entry));
            }
        }
        let norm = |f: &Focus| match f {
            Focus::Chip(_) => Focus::Chip(0),
            Focus::Nav(_) => Focus::Nav(0),
            Focus::Head(_) => Focus::Head(0),
            Focus::Entry => Focus::Entry,
        };
        let here = (ls.fpane, norm(&ls.focus));
        let Some(i) = stops.iter().position(|s| *s == here) else { return };
        let j = i as i32 + d;
        if j < 0 || j as usize >= stops.len() {
            return;
        }
        let (p, f) = stops[j as usize].clone();
        let keep = match (&ls.focus, &f) {
            (Focus::Head(x), Focus::Head(_)) => Focus::Head(*x),
            _ => f.clone(),
        };
        ls.fpane = p;
        ls.focus = keep;
        if f == Focus::Entry {
            let items = ls.items(p);
            let pick = if d > 0 { items.first() } else { items.last() };
            if let Some(e) = pick {
                ls.panes[p].selected = e.path.clone();
            }
        }
    }

    fn ls_entry_menu(&mut self, p: usize, i: Option<usize>, x: Option<usize>) {
        let Some(ls) = self.ls.as_mut() else { return };
        let Some(i) = i else {
            self.ls_folder_menu(p, x);
            return;
        };
        let Some(e) = ls.item(p, i) else { return };
        ls.panes[p].selected = e.path.clone();
        ls.fpane = p;
        ls.menu_entry = Some((p, e));
        let x = x.or(ls.sel_at.map(|s| s.1));
        self.ls_open_menu("item", x);
    }
    fn ls_folder_menu(&mut self, p: usize, x: Option<usize>) {
        let Some(ls) = self.ls.as_mut() else { return };
        ls.menu_entry = None;
        ls.fpane = p.min(ls.panes.len().saturating_sub(1));
        self.ls_open_menu("folder", x);
    }

    fn ls_dialog_key(&mut self, k: &KeyEvent) {
        self.dirty = true;
        let ls = self.ls.as_mut().unwrap();
        if ls.props.is_some() {
            if matches!(k.code, KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q')) {
                ls.props = None;
            }
            return;
        }
        if let Some((e, on)) = ls.confirm.clone() {
            match k.code {
                KeyCode::Esc => ls.confirm = None,
                KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => ls.confirm = Some((e, 1 - on)),
                KeyCode::Enter | KeyCode::Char(' ') => {
                    ls.confirm = None;
                    if on == 1 {
                        ls.act("delete", &e.path, "", Then::Nothing);
                    }
                }
                _ => {}
            }
            return;
        }
        let Some(a) = ls.ask.as_mut() else { return };
        let t = &mut a.field;
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        match k.code {
            KeyCode::Esc => {
                ls.ask = None;
                ls.error.clear();
            }
            KeyCode::Enter => {
                let (op, name, pane) = (a.op, a.field.text.clone(), a.pane);
                let target = match op {
                    "rename" => a.entry.as_ref().map(|e| e.path.clone()).unwrap_or_default(),
                    _ => ls.panes.get(pane).map(|p| p.dir.clone()).unwrap_or_default(),
                };
                ls.act(op, &target, &name, Then::CloseAsk);
            }
            KeyCode::Left => {
                let q = if t.selection().is_some() && !shift { t.selection().unwrap().0 } else { t.prev(t.cursor) };
                t.move_to(q, shift);
            }
            KeyCode::Right => {
                let q = if t.selection().is_some() && !shift { t.selection().unwrap().1 } else { t.next(t.cursor) };
                t.move_to(q, shift);
            }
            KeyCode::Home => t.move_to(0, shift),
            KeyCode::End => t.move_to(t.len(), shift),
            KeyCode::Backspace => match t.selection() {
                Some((x, y)) => t.replace(x, y, ""),
                None if t.cursor > 0 => {
                    let x = t.prev(t.cursor);
                    t.replace(x, t.cursor, "");
                }
                _ => {}
            },
            KeyCode::Delete => match t.selection() {
                Some((x, y)) => t.replace(x, y, ""),
                None if t.cursor < t.len() => {
                    let y = t.next(t.cursor);
                    t.replace(t.cursor, y, "");
                }
                _ => {}
            },
            KeyCode::Char('a') if ctrl => t.select(0, t.len()),
            KeyCode::Char('u') if ctrl => t.replace(0, t.cursor, ""),
            KeyCode::Char(c) if !ctrl => t.insert(&c.to_string()),
            _ => {}
        }
    }

    /// A paste while the listing has the keys: file paths (dragged in from
    /// a file manager) are copied into the folder, as a drop is; else no.
    pub fn ls_paste(&mut self, s: &str) -> bool {
        if let Some(a) = self.ls.as_mut().and_then(|l| l.ask.as_mut()) {
            a.field.insert(s.lines().next().unwrap_or(""));
            self.dirty = true;
            return true;
        }
        if !self.ls_focus {
            return false;
        }
        let Some(ls) = self.ls.as_mut() else { return false };
        let paths: Vec<String> = s
            .split(['\n', '\r'])
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(|l| {
                if let Some(r) = l.strip_prefix("file://") {
                    pct_decode(r.find('/').map(|i| &r[i..]).unwrap_or(r))
                } else {
                    l.trim_matches('\'').to_string()
                }
            })
            .collect();
        if paths.is_empty() || !paths.iter().all(|p| p.starts_with('/') && std::path::Path::new(p).exists()) {
            return false;
        }
        let p = ls.fpane.min(ls.panes.len().saturating_sub(1));
        let into = ls.selected_entry().filter(|e| e.dir).map(|e| e.path).unwrap_or_else(|| ls.panes[p].dir.clone());
        let mut rest: Vec<String> = paths.into_iter().filter(|x| *x != into && parent(x) != into).collect();
        if !rest.is_empty() {
            let first = rest.remove(0);
            ls.act("copy", &first, &into, Then::Paste(rest, "copy".into(), into.clone()));
        }
        self.dirty = true;
        true
    }

    /// A mouse event over the listing (or a drag that started there).
    pub fn ls_mouse(&mut self, e: MouseEvent, hit: Option<Hit>) -> bool {
        self.dirty = true;
        if self.ls.is_none() {
            return true;
        }
        let lh = match &hit {
            Some(Hit::Ls(h)) => Some(h.clone()),
            _ => None,
        };
        let shift = e.modifiers.contains(KeyModifiers::SHIFT);
        match e.kind {
            MouseEventKind::Moved => {
                self.pointer = Some((e.column, e.row, Instant::now()));
                self.hover = None;
                if self.tooltip.as_ref().is_some_and(|t| (t.0, t.1) != (e.column, e.row)) {
                    self.tooltip = None;
                }
                true
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let ls = self.ls.as_mut().unwrap();
                // (At its top already, up: the mouse back to the terminal,
                // which scrolls its scrollback from here, to what came
                // before (an earlier ls, all of it). It looked wiped.)
                if !ls.capped || (e.kind == MouseEventKind::ScrollUp && ls.scroll == 0) {
                    return false;
                }
                ls.scroll = if e.kind == MouseEventKind::ScrollUp { ls.scroll.saturating_sub(3) } else { ls.scroll + 3 };
                true
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.tooltip = None;
                self.menu = None;
                self.close_comp();
                let ls = self.ls.as_mut().unwrap();
                if ls.dialog_open() {
                    return self.ls_dialog_click(&lh.unwrap_or(LsHit::Space(0)));
                }
                let Some(h) = lh else { return true };
                let double = ls.last_down.as_ref().is_some_and(|(t, x)| t.elapsed() < Duration::from_millis(400) && *x == h);
                ls.last_down = Some((Instant::now(), h.clone()));
                if let LsHit::Edge(p, k) = h {
                    if double {
                        ls.set_col_w(k, 0);
                        ls.sizing = None;
                        ls.press = None;
                        return true;
                    }
                    let w0 = ls.panes[p].tcols.iter().find(|c| c.key == k).map(|c| c.w).unwrap_or(8);
                    ls.press = Some((LsHit::Edge(p, k), e.column, w0));
                    ls.sizing = Some((k, w0));
                    return true;
                }
                ls.press = Some((h, e.column, 0));
                true
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                let ls = self.ls.as_mut().unwrap();
                match ls.press.clone() {
                    Some((LsHit::Edge(_, k), c0, w0)) => {
                        let wd = (w0 as i32 + e.column as i32 - c0 as i32).max(3) as usize;
                        ls.sizing = Some((k, wd));
                    }
                    Some((LsHit::Head(p, k), c0, _)) => {
                        if ls.dragging.is_some() || (e.column as i32 - c0 as i32).abs() > 1 {
                            ls.dragging = Some((p, k, e.column));
                        }
                    }
                    _ => {}
                }
                true
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let ls = self.ls.as_mut().unwrap();
                let press = ls.press.take();
                if let Some((k, wd)) = ls.sizing.take() {
                    if matches!(press, Some((LsHit::Edge(..), _, _))) {
                        ls.set_col_w(k, wd);
                    }
                    return true;
                }
                if let Some((p, k, _)) = ls.dragging {
                    let to = ls.drop_before(p).unwrap_or_default();
                    ls.dragging = None;
                    if to != k {
                        ls.move_col(k, &to);
                    }
                    return true;
                }
                if let (Some((ph, _, _)), Some(h)) = (press, lh) {
                    if ph == h {
                        self.ls_click(h, shift, e.column as usize);
                    }
                }
                true
            }
            MouseEventKind::Down(MouseButton::Right) => {
                self.tooltip = None;
                self.close_comp();
                let Some(h) = lh else { return true };
                let x = Some(e.column as usize);
                let ls = self.ls.as_mut().unwrap();
                if ls.dialog_open() {
                    return true;
                }
                match h {
                    LsHit::Entry(p, i) if ls.view() != "commas" => {
                        if let Some(en) = ls.item(p, i) {
                            ls.panes[p].selected = en.path.clone();
                            ls.fpane = p;
                            ls.menu_entry = Some((p, en));
                            self.ls_open_menu("item", x);
                        }
                    }
                    LsHit::Head(..) | LsHit::Edge(..) => self.ls_open_menu("columns", x),
                    LsHit::Dialog | LsHit::Button(_) => {}
                    other => {
                        let p = match other {
                            LsHit::Space(p) | LsHit::Entry(p, _) | LsHit::Arrow(p, _) | LsHit::Nav(p, _) | LsHit::Crumb(p, _) => p,
                            _ => ls.fpane,
                        };
                        self.ls_folder_menu(p, x);
                    }
                }
                true
            }
            _ => true,
        }
    }

    fn ls_dialog_click(&mut self, h: &LsHit) -> bool {
        let ls = self.ls.as_mut().unwrap();
        match h {
            LsHit::Button("cancel") => ls.confirm = None,
            LsHit::Button("delete") => {
                if let Some((e, _)) = ls.confirm.take() {
                    ls.act("delete", &e.path, "", Then::Nothing);
                }
            }
            LsHit::Dialog | LsHit::Button(_) => {
                // (Properties: a click closes it, as anywhere.)
            }
            _ => {
                ls.ask = None;
                ls.confirm = None;
                ls.props = None;
            }
        }
        true
    }

    /// A click (press and release on the same thing).
    fn ls_click(&mut self, h: LsHit, shift: bool, x: usize) {
        let Some(ls) = self.ls.as_mut() else { return };
        match h {
            LsHit::Entry(p, i) => {
                if ls.view() == "commas" {
                    ls.panes[p].selected.clear();
                    return;
                }
                if let Some(e) = ls.item(p, i) {
                    ls.panes[p].selected = e.path.clone();
                    ls.fpane = p;
                    self.ls_open_entry(p, &e);
                }
            }
            LsHit::Arrow(p, i) => {
                if let Some(e) = ls.item(p, i) {
                    ls.toggle_open(p, &e);
                }
            }
            LsHit::Head(_, k) => ls.sort_on(k, shift),
            LsHit::Chip(n) => self.ls_chip(n, Some(x)),
            LsHit::Nav(p, _) | LsHit::Crumb(p, _) => {
                let on = ls.nav_items(p).into_iter().any(|(_, hh, on)| hh == h && on);
                if on {
                    self.ls_nav(p, &h);
                }
            }
            LsHit::Space(p) => {
                if let Some(pane) = ls.panes.get_mut(p) {
                    pane.selected.clear();
                }
            }
            LsHit::Error => ls.error.clear(),
            LsHit::Bar(up) => {
                let r = ls.room.max(2) - 1;
                ls.scroll = if up { ls.scroll.saturating_sub(r) } else { ls.scroll + r };
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(cmd: &str) -> Plan {
        command(cmd, "/base", "/home/u").expect(cmd)
    }

    #[test]
    fn plans() {
        let a = p("ls");
        assert_eq!(a.single, "/base");
        assert!(!a.long && !a.all);
        let a = p("ls -la ~/x/");
        assert!(a.long && a.all);
        assert_eq!(a.single, "/home/u/x");
        assert!(a.opts.bytes);
        let a = p("ls -lh");
        assert!(!a.opts.bytes);
        let a = p("ll");
        assert!(a.long);
        let a = p("la");
        assert!(a.long && a.all);
        let a = p("ls -tr");
        assert_eq!(a.sort, "date");
        assert!(a.reverse);
        let a = p("ls -S --group-directories-first");
        assert_eq!(a.sort, "size");
        assert!(a.folders_first);
        let a = p("ls *.txt sub");
        assert_eq!(a.groups.len(), 1);
        assert_eq!(a.groups[0].dir, "/base");
        assert_eq!(a.groups[0].names, vec!["sub".to_string()]);
        assert!(a.groups[0].patterns[0].matches("a.txt"));
        assert!(!a.groups[0].patterns[0].matches(".a.txt"));
        assert!(a.headers);
        let a = p("ls -d");
        assert_eq!(a.groups[0].dir, "/");
        assert_eq!(a.groups[0].names, vec!["base".to_string()]);
        let a = p("ls -I '*.o' -B -m");
        assert!(a.ignore[0].matches("x.o"));
        assert!(a.ignore[1].matches("x~"));
        assert!(a.opts.commas);
        let a = p("ls --sort=time --time=ctime -iQF");
        assert_eq!(a.sort, "date");
        assert_eq!(a.opts.time, "ctime");
        assert!(a.opts.inode);
        assert_eq!(a.opts.quote, "c");
        assert_eq!(a.opts.classify, "F");
        assert!(command("ls --help", "/", "/").is_none());
        assert!(command("ls | wc", "/", "/").is_none());
        assert!(command("cat x", "/", "/").is_none());
        assert!(command("eza", "/", "/").is_none() && command("lsd -l", "/", "/").is_none());
        assert!(command("ls $HOME", "/", "/").is_none());
        let a = p("ls 'My Docs'");
        assert_eq!(a.single, "/base/My Docs");
        let a = p("ls -R");
        assert!(a.recursive && a.headers);
    }

    #[test]
    fn layout() {
        // (ls's columns: as many as fit, filled going down.)
        let (c, r, ws) = grid(&[5, 5, 5, 5, 5, 5], 30);
        assert_eq!((c, r), (3, 2));
        assert_eq!(ws, vec![7, 7, 7]);
        let (c, r, _) = grid(&[40, 5], 30);
        assert_eq!((c, r), (1, 2));
        assert_eq!(human(1023), "1023 B");
        assert_eq!(human(1536), "1.5 KB");
        assert_eq!(human(20 * 1024 * 1024), "20 MB");
        assert_eq!(natural("file10", "file9"), Ordering::Greater);
        assert_eq!(natural("B", "a"), Ordering::Greater);
    }

    fn sandbox(name: &str) -> String {
        let d = std::env::temp_dir().join(format!("tessh-ls-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("sub")).unwrap();
        std::fs::write(d.join("b.txt"), "hello").unwrap();
        std::fs::write(d.join("a.rs"), "fn main() {}").unwrap();
        std::fs::write(d.join(".hidden"), "").unwrap();
        std::fs::write(d.join("sub/inner.md"), "# x").unwrap();
        d.to_string_lossy().to_string()
    }

    fn text_of(rows: &[Line]) -> Vec<String> {
        rows.iter().map(|r| r.iter().map(|s| s.text.as_str()).collect::<String>().trim_end().to_string()).collect()
    }

    #[test]
    fn listing() {
        std::env::set_var("TESSH_LS_NO_SAVE", "1");
        let d = sandbox("list");
        let pal = Palette { fg: None, bg: None };
        let mut ls = Ls::new(command("ls", &d, "/").unwrap(), &d);
        ls.mine = json!({});
        ls.saved = json!({"view": "list"});
        ls.touch();
        let rows = text_of(&ls.frozen(&pal, 80));
        assert_eq!(rows.len(), 1, "{rows:?}");
        let r = &rows[0];
        // (By name, a folder's /, no hidden files.)
        let (a, b, s) = (r.find("a.rs").unwrap(), r.find("b.txt").unwrap(), r.find("sub/").unwrap());
        assert!(a < b && b < s, "{r}");
        assert!(!r.contains(".hidden"));
        // (Left in the scrollback, the columns keep their widths, as live:
        // each name starts where it does in the live view.)
        let (live, _) = ls.panel(&pal, 80, 40, None, false);
        let lr = text_of(&live).into_iter().find(|l| l.contains("a.rs")).unwrap();
        for n in ["b.txt", "sub/"] {
            assert_eq!(r.find(n), lr.find(n), "{n}: frozen {r:?}, live {lr:?}");
        }
        // -a: hidden too. -l: the table, sizes exact.
        let mut ls = Ls::new(command("ls -la", &d, "/").unwrap(), &d);
        ls.mine = json!({});
        ls.touch();
        let rows = text_of(&ls.frozen(&pal, 120));
        assert!(rows[0].contains("Name") && rows[0].contains("Size") && rows[0].contains("Permissions"), "{rows:?}");
        assert!(rows.iter().any(|r| r.contains(".hidden")));
        assert!(rows.iter().any(|r| r.contains("b.txt") && r.contains(" 5 ")), "{rows:?}");
        // A heading's click: sorted by it; again the other way.
        ls.sort_on("size", false);
        let rows = text_of(&ls.frozen(&pal, 120));
        // (Largest first; a folder counts its items, as Tessera's.)
        let order: Vec<usize> = ["a.rs", "b.txt", "sub/"].iter().map(|n| rows.iter().position(|r| r.contains(n)).unwrap()).collect();
        assert!(order[0] < order[1] && order[1] < order[2], "largest first: {rows:?}");
        assert!(rows[0].contains("Size ▼"), "{rows:?}");
        // ▸ opens a folder in place.
        let i = ls.items(0).iter().position(|e| e.name == "sub").unwrap();
        let e = ls.items(0)[i].clone();
        ls.toggle_open(0, &e);
        let (live, _) = ls.panel(&pal, 120, 40, None, false);
        assert!(text_of(&live).iter().any(|r| r.contains("▾") && r.contains("sub/")));
        let rows = text_of(&ls.frozen(&pal, 120));
        assert!(rows.iter().any(|r| r.contains("inner.md")), "{rows:?}");
        // Into a folder: the path bar.
        ls.go(0, &format!("{d}/sub"));
        let (live, _) = ls.panel(&pal, 120, 40, None, false);
        assert!(text_of(&live).iter().any(|r| r.contains("cd here")));
        // (Left in the scrollback: nothing that looks clickable.)
        let rows = text_of(&ls.frozen(&pal, 120));
        assert_eq!(rows[0], format!("{d}/sub:"), "{rows:?}");
        assert!(!rows.iter().any(|r| r.contains("cd here") || r.contains('▸') || r.contains('▾')), "{rows:?}");
        ls.go_back(0);
        assert_eq!(ls.panes[0].dir, d);
        // -m: commas.
        let mut ls = Ls::new(command("ls -m", &d, "/").unwrap(), &d);
        let rows = text_of(&ls.frozen(&pal, 80));
        assert!(rows[0].contains("a.rs, b.txt, sub/"), "{rows:?}");
        // ls *.txt sub: the picked names, then sub's pane under its heading.
        let mut ls = Ls::new(command("ls *.txt sub", &d, "/").unwrap(), &d);
        ls.mine = json!({});
        ls.saved = json!({"view": "list"});
        ls.touch();
        let rows = text_of(&ls.frozen(&pal, 80));
        assert!(rows[0].contains("b.txt") && !rows[0].contains("a.rs"), "{rows:?}");
        assert!(rows.iter().any(|r| r == "sub:"), "{rows:?}");
        assert!(rows.iter().any(|r| r.contains("inner.md")), "{rows:?}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn keys_move_and_open() {
        std::env::set_var("TESSH_LS_NO_SAVE", "1");
        let d = sandbox("keys");
        let pal = Palette { fg: None, bg: None };
        let mut ed = Ed::new();
        let mut ls = Ls::new(command("ls -1", &d, "/").unwrap(), &d);
        ls.mine = json!({});
        ls.saved = json!({});
        ls.touch();
        ed.show_ls(ls);
        let key = |c: KeyCode, m: KeyModifiers| KeyEvent::new(c, m);
        assert!(ed.ls_key(&key(KeyCode::Char('l'), KeyModifiers::CONTROL | KeyModifiers::ALT)));
        assert!(ed.ls_focus);
        let sel = |ed: &Ed| ed.ls.as_ref().unwrap().panes[0].selected.rsplit('/').next().unwrap().to_string();
        assert_eq!(sel(&ed), "a.rs");
        ed.ls_key(&key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(sel(&ed), "b.txt");
        ed.ls_key(&key(KeyCode::Char('s'), KeyModifiers::NONE));
        assert_eq!(sel(&ed), "sub");
        // Enter on a folder goes in; Backspace back.
        ed.ls_key(&key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(ed.ls.as_ref().unwrap().panes[0].dir, format!("{d}/sub"));
        let (rows, _) = ed.ls.as_mut().unwrap().panel(&pal, 80, 20, None, true);
        assert!(text_of(&rows).iter().any(|r| r.contains("inner.md")));
        ed.ls_key(&key(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(ed.ls.as_ref().unwrap().panes[0].dir, d);
        // Up past the first: the buttons; Enter on ≡: the table.
        ed.ls_key(&key(KeyCode::Home, KeyModifiers::NONE));
        ed.ls_key(&key(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(ed.ls.as_ref().unwrap().focus, Focus::Chip(0));
        ed.ls_key(&key(KeyCode::Right, KeyModifiers::NONE));
        ed.ls_key(&key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(ed.ls.as_ref().unwrap().view(), "details");
        // A text file: the viewer.
        ed.ls_key(&key(KeyCode::Down, KeyModifiers::NONE));
        ed.ls_key(&key(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(ed.ls.as_ref().unwrap().focus, Focus::Entry);
        ed.ls_key(&key(KeyCode::Char('b'), KeyModifiers::NONE));
        ed.ls_key(&key(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(ed.done, Some(Outcome::Host(format!("lsview:{d}/b.txt"))));
        // Shift+F10: its menu; Rename… asks, with the name's stem selected.
        ed.done = None;
        ed.ls_key(&key(KeyCode::F(10), KeyModifiers::SHIFT));
        assert!(matches!(ed.menu.as_ref().map(|m| m.kind.clone()), Some(MenuKind::Ls(k)) if k == "item"));
        ed.menu = None;
        ed.ls_key(&key(KeyCode::F(2), KeyModifiers::NONE));
        assert!(ed.ls.as_ref().unwrap().dialog_open());
        assert_eq!(ed.ls.as_ref().unwrap().ask.as_ref().unwrap().field.selected(), "b");
        ed.ls_key(&key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!ed.ls.as_ref().unwrap().dialog_open());
        // Esc: the editor's again.
        ed.ls_key(&key(KeyCode::Esc, KeyModifiers::NONE));
        assert!(!ed.ls_focus);
        let _ = std::fs::remove_dir_all(&d);
    }
}
