//! Completions for the input editor: commands, subcommands, options with
//! their help text, paths, git branches, ssh hosts and environment variables.
//!
//! Help text comes from fish's completion files when there are any (fish's
//! own folders, or the copy Preferences can download) and from man pages
//! (rendered once per command, parsed, cached in memory). Nothing runs the
//! command itself (`--help` could do anything).

use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Serialize, Debug, PartialEq)]
pub struct Item {
    pub label: String,
    /// What replaces the word (a path keeps its folder, a dir gets "/").
    pub insert: String,
    pub detail: String,
    pub kind: &'static str, // command, subcommand, option, file, dir, branch, host, variable, value
}

/// Help for one command: its options and subcommands (each subcommand's
/// own options under its name).
#[derive(Default, Clone, Debug)]
pub struct Help {
    pub options: Vec<(String, String)>,
    pub subcommands: Vec<(String, String)>,
    pub sub_options: HashMap<String, Vec<(String, String)>>,
    /// Values for the first argument (fish's -a without a subcommand context).
    pub values: Vec<(String, String)>,
}

// ---- the line ---------------------------------------------------------------

#[derive(Debug, Clone)]
struct Word {
    text: String, // unquoted
    start: usize, // byte offset in the line
}

/// The words of the simple command the cursor is in (after the last ; | &&
/// || ( or newline), quotes removed. The last word is the one being typed
/// (empty when the line ends in a space).
fn words(line: &str) -> Vec<Word> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut start = None;
    let mut quote: Option<char> = None;
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else if c == '\\' && q == '"' {
                if let Some((_, n)) = chars.next() {
                    cur.push(n);
                }
            } else {
                cur.push(c);
            }
            continue;
        }
        match c {
            '\'' | '"' => {
                start.get_or_insert(i);
                quote = Some(c);
            }
            '\\' => {
                start.get_or_insert(i);
                if let Some((_, n)) = chars.next() {
                    cur.push(n);
                }
            }
            ' ' | '\t' => {
                if let Some(s) = start.take() {
                    out.push(Word { text: std::mem::take(&mut cur), start: s });
                }
            }
            ';' | '|' | '&' | '\n' | '(' | '`' => {
                out.clear();
                cur.clear();
                start = None;
            }
            _ => {
                start.get_or_insert(i);
                cur.push(c);
            }
        }
    }
    out.push(Word { text: cur, start: start.unwrap_or(line.len()) });
    // Leading assignments (FOO=1 cmd) and sudo-likes aren't the command.
    while out.len() > 1 && (out[0].text.contains('=') && !out[0].text.starts_with('-') || is_prefix_command(&out[0].text)) {
        out.remove(0);
    }
    out
}

fn is_prefix_command(w: &str) -> bool {
    matches!(w, "sudo" | "doas" | "env" | "nice" | "time" | "exec" | "command" | "builtin" | "nohup" | "watch" | "xargs" | "strace")
}

// ---- sources -----------------------------------------------------------------

const BUILTINS: &[(&str, &str)] = &[
    ("cd", "change the current folder"),
    ("echo", "print arguments"),
    ("export", "set an environment variable"),
    ("source", "run a script in this shell"),
    ("alias", "define or list aliases"),
    ("unset", "remove a variable or function"),
    ("history", "show the command history"),
    ("pushd", "push a folder onto the stack"),
    ("popd", "pop a folder off the stack"),
    ("exit", "leave the shell"),
    ("jobs", "list background jobs"),
    ("fg", "bring a job to the foreground"),
    ("bg", "resume a job in the background"),
    ("type", "how a name would be interpreted"),
    ("set", "set shell options"),
    ("read", "read a line into variables"),
    ("printf", "print formatted text"),
    ("test", "evaluate a condition"),
    ("eval", "run arguments as a command"),
    ("trap", "run a command on a signal"),
    ("wait", "wait for jobs to finish"),
    ("kill", "send a signal to a process"),
    ("umask", "set the file creation mask"),
    ("ulimit", "set resource limits"),
    ("for", "loop over words"),
    ("while", "loop while a command succeeds"),
    ("if", "run commands on a condition"),
    ("case", "match a word against patterns"),
    ("function", "define a function"),
];

/// Executables on PATH (cached for a minute).
fn path_commands() -> Arc<Vec<String>> {
    static CACHE: Mutex<Option<(std::time::Instant, Arc<Vec<String>>)>> = Mutex::new(None);
    let mut c = CACHE.lock().unwrap();
    if let Some((t, v)) = c.as_ref() {
        if t.elapsed().as_secs() < 60 {
            return v.clone();
        }
    }
    let mut set = HashSet::new();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') {
                    continue;
                }
                if cfg!(windows) {
                    let lower = name.to_lowercase();
                    if let Some(stem) = [".exe", ".cmd", ".bat", ".ps1"].iter().find_map(|x| lower.strip_suffix(x)) {
                        set.insert(stem.to_string());
                    }
                } else if is_executable(&e.path()) {
                    set.insert(name);
                }
            }
        }
    }
    let mut v: Vec<String> = set.into_iter().collect();
    v.sort();
    let v = Arc::new(v);
    *c = Some((std::time::Instant::now(), v.clone()));
    v
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(p).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false)
}
#[cfg(not(unix))]
fn is_executable(p: &Path) -> bool {
    p.is_file()
}

/// One-line descriptions of commands (`whatis`), cached.
fn describe_commands(names: &[String]) -> HashMap<String, String> {
    static CACHE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let missing: Vec<&String> = {
        let c = cache.lock().unwrap();
        names.iter().filter(|n| !c.contains_key(*n)).collect()
    };
    if !missing.is_empty() && !cfg!(windows) {
        let out = std::process::Command::new("whatis")
            .args(missing.iter().map(|s| s.as_str()))
            .stderr(std::process::Stdio::null())
            .output();
        let mut c = cache.lock().unwrap();
        for m in &missing {
            c.insert((*m).clone(), String::new());
        }
        if let Ok(o) = out {
            for line in String::from_utf8_lossy(&o.stdout).lines() {
                // "ls (1)               - list directory contents"
                let Some((head, desc)) = line.split_once(" - ") else { continue };
                let name = head.split_whitespace().next().unwrap_or("");
                if let Some(slot) = c.get_mut(name) {
                    if slot.is_empty() {
                        *slot = desc.trim().to_string();
                    }
                }
            }
        }
    }
    let c = cache.lock().unwrap();
    names.iter().map(|n| (n.clone(), c.get(n).cloned().unwrap_or_default())).collect()
}

// ---- help: fish completion files ---------------------------------------------

/// Where fish's completion files may be.
fn fish_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![fish_download_dir().join("share").join("completions")];
    if let Some(h) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(&h).join(".config/fish/completions"));
    }
    let data = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    for d in data.split(':').filter(|d| !d.is_empty()) {
        dirs.push(Path::new(d).join("fish/vendor_completions.d"));
        dirs.push(Path::new(d).join("fish/completions"));
    }
    for d in ["/run/current-system/sw/share/fish/completions", "/usr/share/fish/completions", "/opt/homebrew/share/fish/completions"] {
        dirs.push(PathBuf::from(d));
    }
    dirs
}

pub fn fish_download_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("tessera")
        .join("fish-shell")
}

/// Fetch fish's completion files (only that folder of its repository).
/// Returns how many commands they cover.
pub fn download_fish_completions() -> Result<usize, String> {
    let dir = fish_download_dir();
    let git = |args: &[&str]| -> Result<(), String> {
        let ok = std::process::Command::new("git").args(args).status().map_err(|e| format!("couldn't run git: {e}"))?;
        if ok.success() { Ok(()) } else { Err(format!("git {} failed", args.first().unwrap_or(&""))) }
    };
    if dir.join(".git").exists() {
        git(&["-C", &dir.to_string_lossy(), "pull", "-q", "--depth", "1"])?;
    } else {
        let _ = std::fs::create_dir_all(dir.parent().unwrap());
        git(&["clone", "-q", "--depth", "1", "--filter=blob:none", "--sparse", "https://github.com/fish-shell/fish-shell.git", &dir.to_string_lossy()])?;
        git(&["-C", &dir.to_string_lossy(), "sparse-checkout", "set", "share/completions"])?;
    }
    HELP.get().map(|h| h.lock().unwrap().clear());
    Ok(std::fs::read_dir(dir.join("share/completions")).map(|r| r.count()).unwrap_or(0))
}

pub fn fish_completion_count() -> usize {
    std::fs::read_dir(fish_download_dir().join("share/completions")).map(|r| r.count()).unwrap_or(0)
}

fn fish_help(cmd: &str) -> Option<Help> {
    let file = fish_dirs().into_iter().map(|d| d.join(format!("{cmd}.fish"))).find(|p| p.is_file())?;
    let text = std::fs::read_to_string(file).ok()?;
    Some(parse_fish(cmd, &text))
}

/// fish `complete` lines: -c cmd, -s/-l/-o options, -d description, -a
/// arguments, -n condition (subcommand contexts understood; other conditions
/// are taken as applying everywhere).
pub fn parse_fish(cmd: &str, text: &str) -> Help {
    let mut h = Help::default();
    // Continuation lines.
    let joined = text.replace("\\\n", " ");
    for line in joined.lines() {
        let line = line.trim();
        if !line.starts_with("complete ") {
            continue;
        }
        let Ok(args) = shell_words::split(line) else { continue };
        let (mut target, mut desc, mut cond, mut values) = (String::new(), String::new(), String::new(), String::new());
        let mut flags: Vec<String> = Vec::new();
        let mut i = 1;
        while i < args.len() {
            let a = &args[i];
            let next = args.get(i + 1).cloned().unwrap_or_default();
            let (key, inline) = match a.split_once('=') {
                Some((k, v)) if k.starts_with("--") => (k.to_string(), Some(v.to_string())),
                _ => (a.clone(), None),
            };
            let take = |i: &mut usize| -> String {
                if let Some(v) = inline.clone() { v } else { *i += 1; next.clone() }
            };
            match key.as_str() {
                "-c" | "--command" => target = take(&mut i),
                "-d" | "--description" => desc = take(&mut i),
                "-n" | "--condition" => cond = take(&mut i),
                "-a" | "--arguments" => values = take(&mut i),
                "-s" | "--short-option" => flags.push(format!("-{}", take(&mut i))),
                "-l" | "--long-option" => flags.push(format!("--{}", take(&mut i))),
                "-o" | "--old-option" => flags.push(format!("-{}", take(&mut i))),
                "-w" | "--wraps" | "-k" | "--keep-order" | "-f" | "--no-files" | "-F" | "--force-files" | "-r" | "--require-parameter" | "-x" | "--exclusive" => {
                    if matches!(key.as_str(), "-w" | "--wraps") {
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        if !target.is_empty() && target != cmd {
            continue;
        }
        // Which subcommand(s) this line is about.
        let subs: Vec<String> = cond
            .split(';')
            .filter_map(|c| {
                let c = c.trim();
                c.strip_prefix("__fish_seen_subcommand_from ").map(|r| r.split_whitespace().map(str::to_string).collect::<Vec<_>>())
            })
            .flatten()
            .collect();
        let top_sub = cond.contains("__fish_use_subcommand") || cond.contains("__fish_is_first_arg") || cond.contains("__fish_is_first_token");
        if !flags.is_empty() {
            let label = flags.join(", ");
            let opts: Vec<(String, String)> = flags.iter().map(|f| (f.clone(), desc.clone())).collect();
            let _ = label;
            if subs.is_empty() {
                h.options.extend(opts);
            } else {
                for s in &subs {
                    h.sub_options.entry(s.clone()).or_default().extend(opts.iter().cloned());
                }
            }
        } else if !values.is_empty() && !values.contains('(') && !values.contains('$') {
            for v in values.split_whitespace() {
                let (v, d) = match v.split_once('\t') {
                    Some((v, d)) => (v.to_string(), d.to_string()),
                    None => (v.to_string(), desc.clone()),
                };
                if top_sub {
                    h.subcommands.push((v, d));
                } else if subs.is_empty() {
                    h.values.push((v, d));
                }
            }
        }
    }
    h
}

// ---- help: man pages ---------------------------------------------------------

/// Options from a rendered man page: lines starting with an option (indented
/// a little), and the description on the same line after a gap or on the
/// lines indented below it.
pub fn parse_man(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim_start();
        let ind = indent(line);
        if t.starts_with('-') && t.len() > 1 && ind > 0 && ind <= 14 && !t.starts_with("--- ") && !t.starts_with("-- ") {
            // The option part: up to two spaces (or the end).
            let (head, rest) = match t.find("  ") {
                Some(p) => (&t[..p], t[p..].trim()),
                None => (t, ""),
            };
            let mut desc = rest.to_string();
            if desc.is_empty() {
                let mut j = i + 1;
                while j < lines.len() && !lines[j].trim().is_empty() && indent(lines[j]) > ind {
                    if !desc.is_empty() {
                        desc.push(' ');
                    }
                    desc.push_str(lines[j].trim());
                    j += 1;
                    if desc.len() > 300 {
                        break;
                    }
                }
            }
            for flag in head.split(", ").flat_map(|p| p.split(" | ")) {
                let flag = flag.trim();
                // "-o, --output=FILE" -> "--output" (the argument shows in the detail).
                let name: String = flag.chars().take_while(|c| !matches!(c, '=' | '[' | ' ' | '<')).collect();
                if name.len() < 2 || !name.starts_with('-') || name.chars().nth(1).is_some_and(|c| c.is_ascii_digit() && name.len() > 3) {
                    continue;
                }
                let arg = flag[name.len()..].trim();
                let detail = if arg.is_empty() { desc.clone() } else { format!("{arg}  {desc}") };
                if !out.iter().any(|(n, _)| *n == name) {
                    out.push((name, detail));
                }
            }
        }
        i += 1;
    }
    for (_, d) in out.iter_mut() {
        // (Justified text has runs of spaces.)
        *d = d.split_whitespace().collect::<Vec<_>>().join(" ");
        if d.len() > 240 {
            let cut = (0..=240).rev().find(|&k| d.is_char_boundary(k)).unwrap_or(0);
            d.truncate(cut);
            d.push('…');
        }
    }
    out
}

fn man_options(page: &str) -> Vec<(String, String)> {
    if cfg!(windows) {
        return Vec::new();
    }
    let out = std::process::Command::new("man")
        .args(["-P", "cat", page])
        .env("MANWIDTH", "400")
        .env("MAN_KEEP_FORMATTING", "0")
        .env("GROFF_NO_SGR", "1")
        .stderr(std::process::Stdio::null())
        .output();
    let Ok(o) = out else { return Vec::new() };
    // Strip overstrike (x\bx bold, _\bx underline).
    let raw = String::from_utf8_lossy(&o.stdout);
    let mut text = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c == '\u{8}' {
            text.pop();
        } else {
            text.push(c);
        }
    }
    parse_man(&text)
}

// ---- help: cache ------------------------------------------------------------

static HELP: OnceLock<Mutex<HashMap<String, Option<Arc<Help>>>>> = OnceLock::new();

/// A command's help: from the cache, or None while it's being read (in the
/// background; ask again shortly).
fn help(cmd: &str) -> Option<Arc<Help>> {
    let cache = HELP.get_or_init(Default::default);
    {
        let c = cache.lock().unwrap();
        if let Some(entry) = c.get(cmd) {
            return entry.clone();
        }
    }
    cache.lock().unwrap().insert(cmd.to_string(), None);
    let name = cmd.to_string();
    std::thread::spawn(move || {
        let mut h = fish_help(&name).unwrap_or_default();
        // Man page options fill in what fish doesn't have (and its descriptions
        // are usually fuller).
        for (flag, desc) in man_options(&name) {
            match h.options.iter_mut().find(|(f, _)| *f == flag) {
                Some(slot) if slot.1.is_empty() => slot.1 = desc,
                Some(_) => {}
                None => h.options.push((flag, desc)),
            }
        }
        if name == "git" {
            git_subcommands(&mut h);
        }
        HELP.get().unwrap().lock().unwrap().insert(name, Some(Arc::new(h)));
    });
    None
}

/// A subcommand's own options (git-commit(1), cargo-build(1), ...).
fn sub_help(cmd: &str, sub: &str) -> Option<Arc<Help>> {
    help(&format!("{cmd}-{sub}"))
}

fn git_subcommands(h: &mut Help) {
    if !h.subcommands.is_empty() {
        return;
    }
    let Ok(o) = std::process::Command::new("git").args(["help", "-a", "--no-verbose"]).stderr(std::process::Stdio::null()).output() else { return };
    let mut seen = HashSet::new();
    for line in String::from_utf8_lossy(&o.stdout).lines() {
        let l = line.trim();
        let mut parts = l.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or("");
        if name.is_empty() || name.contains(' ') || !name.chars().all(|c| c.is_ascii_lowercase() || c == '-') {
            continue;
        }
        if seen.insert(name.to_string()) {
            h.subcommands.push((name.to_string(), parts.next().unwrap_or("").trim().to_string()));
        }
    }
}

// ---- dynamic values -----------------------------------------------------------

fn git_branches(cwd: &str) -> Vec<String> {
    std::process::Command::new("git")
        .args(["-C", cwd, "for-each-ref", "--format=%(refname:short)", "refs/heads", "refs/remotes", "refs/tags"])
        .stderr(std::process::Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn paths(prefix: &str, cwd: &str, dirs_only: bool) -> Vec<Item> {
    let home = std::env::var("HOME").unwrap_or_default();
    let expanded = if let Some(rest) = prefix.strip_prefix("~/") { format!("{home}/{rest}") } else if prefix == "~" { format!("{home}/") } else { prefix.to_string() };
    let (dir_part, name_part) = match expanded.rfind('/') {
        Some(i) => (&expanded[..=i], &expanded[i + 1..]),
        None => ("", expanded.as_str()),
    };
    let shown_dir = match prefix.rfind('/') {
        Some(i) => &prefix[..=i],
        None => "",
    };
    let base = if dir_part.starts_with('/') { PathBuf::from(dir_part) } else { Path::new(cwd).join(dir_part) };
    let Ok(rd) = std::fs::read_dir(&base) else { return Vec::new() };
    let mut items: Vec<Item> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') && !name_part.starts_with('.') {
                return None;
            }
            let is_dir = e.path().is_dir();
            if dirs_only && !is_dir {
                return None;
            }
            let label = if is_dir { format!("{name}/") } else { name.clone() };
            Some(Item {
                insert: format!("{shown_dir}{}", shell_quote_path(&label)),
                label,
                detail: String::new(),
                kind: if is_dir { "dir" } else { "file" },
            })
        })
        .collect();
    items.sort_by(|a, b| (a.kind != "dir").cmp(&(b.kind != "dir")).then(a.label.to_lowercase().cmp(&b.label.to_lowercase())));
    items
}

fn shell_quote_path(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if " '\"\\$`!&|;()<>*?[]{}#".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

// ---- matching ------------------------------------------------------------------

/// How well `cand` matches what was typed: prefix first, then substring,
/// then letters in order (fuzzy). None: not at all.
fn score(cand: &str, typed: &str) -> Option<u32> {
    if typed.is_empty() {
        return Some(0);
    }
    let c = cand.to_lowercase();
    let t = typed.to_lowercase();
    if c.starts_with(&t) {
        return Some(0);
    }
    if let Some(p) = c.find(&t) {
        return Some(10 + p as u32);
    }
    let mut it = c.chars();
    let mut gaps = 0;
    for ch in t.chars() {
        let mut found = false;
        for x in it.by_ref() {
            if x == ch {
                found = true;
                break;
            }
            gaps += 1;
        }
        if !found {
            return None;
        }
    }
    Some(100 + gaps)
}

fn rank(mut items: Vec<Item>, typed: &str, key: impl Fn(&Item) -> String) -> Vec<Item> {
    let mut scored: Vec<(u32, Item)> = items.drain(..).filter_map(|i| score(&key(&i), typed).map(|s| (s, i))).collect();
    // Loose (fuzzy) matches only when nothing matches more closely.
    if scored.iter().any(|(s, _)| *s < 100) {
        scored.retain(|(s, _)| *s < 100);
    }
    scored.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.label.len().cmp(&b.1.label.len())).then(a.1.label.cmp(&b.1.label)));
    scored.into_iter().map(|(_, i)| i).take(200).collect()
}

// ---- the API ---------------------------------------------------------------------

/// Completions at `cursor` (a UTF-16 offset) in `text`, for a shell in `cwd`.
/// {"start": u16, "end": u16, "items": [...], "pending": bool}: replace
/// start..end with an item's `insert`. `pending`: help is still being read.
pub fn complete(text: &str, cursor_u16: usize, cwd: &str) -> String {
    // UTF-16 cursor -> byte offset.
    let mut byte = text.len();
    let mut n = 0;
    for (i, c) in text.char_indices() {
        if n >= cursor_u16 {
            byte = i;
            break;
        }
        n += c.len_utf16();
    }
    let line = &text[..byte];
    let ws = words(line);
    let cur = ws.last().cloned().unwrap_or(Word { text: String::new(), start: byte });
    let to_u16 = |b: usize| text[..b].encode_utf16().count();
    let (start, end) = (to_u16(cur.start.min(byte)), to_u16(byte));
    let typed = cur.text.clone();
    let mut pending = false;
    let mut items: Vec<Item> = Vec::new();

    if ws.len() <= 1 {
        // The command itself.
        if typed.contains('/') || typed.starts_with('.') || typed.starts_with('~') {
            items = rank(paths(&typed, cwd, false), name_of(&typed), |i| i.label.clone());
        } else {
            let mut cands: Vec<Item> = BUILTINS
                .iter()
                .map(|(n, d)| Item { label: n.to_string(), insert: n.to_string(), detail: d.to_string(), kind: "command" })
                .collect();
            cands.extend(path_commands().iter().map(|n| Item { label: n.clone(), insert: n.clone(), detail: String::new(), kind: "command" }));
            let mut seen = HashSet::new();
            cands.retain(|i| seen.insert(i.label.clone()));
            items = rank(cands, &typed, |i| i.label.clone());
            items.truncate(60);
            let names: Vec<String> = items.iter().filter(|i| i.detail.is_empty()).take(30).map(|i| i.label.clone()).collect();
            let descs = describe_commands(&names);
            for i in items.iter_mut() {
                if i.detail.is_empty() {
                    i.detail = descs.get(&i.label).cloned().unwrap_or_default();
                }
            }
        }
    } else {
        let cmd = ws[0].text.rsplit('/').next().unwrap_or("").to_string();
        let args: Vec<&str> = ws[1..ws.len() - 1].iter().map(|w| w.text.as_str()).collect();
        let h = help(&cmd);
        pending |= h.is_none();
        let h = h.unwrap_or_default();
        // The subcommand, if one was given (the first argument that isn't an option).
        let sub = args.iter().find(|a| !a.starts_with('-')).map(|s| s.to_string());
        let sub_known = sub.as_ref().is_some_and(|s| h.subcommands.iter().any(|(n, _)| n == s));
        if let Some(var) = typed.strip_prefix('$') {
            items = rank(
                std::env::vars().map(|(k, v)| Item { label: format!("${k}"), insert: format!("${k}"), detail: v.chars().take(80).collect(), kind: "variable" }).collect(),
                var,
                |i| i.label[1..].to_string(),
            );
        } else if typed.starts_with('-') {
            let mut opts: Vec<(String, String)> = Vec::new();
            if let Some(s) = sub.as_ref().filter(|_| sub_known) {
                if let Some(o) = h.sub_options.get(s) {
                    opts.extend(o.iter().cloned());
                }
                match sub_help(&cmd, s) {
                    Some(sh) => opts.extend(sh.options.iter().cloned()),
                    None => pending = true,
                }
            }
            opts.extend(h.options.iter().cloned());
            let mut seen = HashSet::new();
            opts.retain(|(f, _)| seen.insert(f.clone()));
            let cands = opts.into_iter().map(|(f, d)| Item { insert: f.clone(), label: f, detail: d, kind: "option" }).collect();
            items = rank(cands, &typed, |i| i.label.clone());
        } else {
            if sub.is_none() && !h.subcommands.is_empty() {
                items.extend(rank(
                    h.subcommands.iter().map(|(n, d)| Item { label: n.clone(), insert: n.clone(), detail: d.clone(), kind: "subcommand" }).collect(),
                    &typed,
                    |i| i.label.clone(),
                ));
            }
            if args.is_empty() && !h.values.is_empty() {
                items.extend(rank(
                    h.values.iter().map(|(n, d)| Item { label: n.clone(), insert: n.clone(), detail: d.clone(), kind: "value" }).collect(),
                    &typed,
                    |i| i.label.clone(),
                ));
            }
            let git_ref = cmd == "git" && sub.as_deref().is_some_and(|s| ["checkout", "switch", "merge", "rebase", "branch", "log", "diff", "reset", "cherry-pick", "show", "push", "pull"].contains(&s));
            if git_ref {
                items.extend(rank(
                    git_branches(cwd).into_iter().map(|b| Item { insert: b.clone(), label: b, detail: String::new(), kind: "branch" }).collect(),
                    &typed,
                    |i| i.label.clone(),
                ));
            }
            if ["ssh", "scp", "sftp", "mosh", "rsync"].contains(&cmd.as_str()) && !typed.contains('/') {
                items.extend(rank(
                    crate::launchers::launchers().ssh_hosts.into_iter().map(|h| Item { insert: h.clone(), label: h, detail: "ssh host".into(), kind: "host" }).collect(),
                    &typed,
                    |i| i.label.clone(),
                ));
            }
            let dirs_only = matches!(cmd.as_str(), "cd" | "pushd" | "rmdir" | "z");
            items.extend(rank(paths(&typed, cwd, dirs_only), name_of(&typed), |i| i.label.clone()));
        }
    }
    json!({"start": start, "end": end, "items": items, "pending": pending}).to_string()
}

/// The part of a path after its last "/" (what the file names are matched against).
fn name_of(p: &str) -> &str {
    p.rsplit('/').next().unwrap_or(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_of_the_current_command() {
        let w = words("ls -la | sudo grep -i \"hello wor");
        let t: Vec<&str> = w.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(t, ["grep", "-i", "hello wor"]);
        let w = words("git commit ");
        assert_eq!(w.last().unwrap().text, "");
        assert_eq!(w.len(), 3);
    }

    #[test]
    fn man_page_options() {
        let page = "DESCRIPTION\n     List things.\n\n     -a, --all\n            do not ignore entries starting with .\n\n     --block-size=SIZE\n            scale sizes by SIZE\n\n     -c     sort by ctime\n";
        let o = parse_man(page);
        assert!(o.contains(&("-a".into(), "do not ignore entries starting with .".into())), "{o:?}");
        assert!(o.contains(&("--all".into(), "do not ignore entries starting with .".into())));
        assert!(o.iter().any(|(f, d)| f == "--block-size" && d.starts_with("=SIZE")), "{o:?}");
        assert!(o.contains(&("-c".into(), "sort by ctime".into())));
    }

    #[test]
    fn fish_file() {
        let f = "complete -c foo -n __fish_use_subcommand -a build -d 'Compile it'\n\
                 complete -c foo -n '__fish_seen_subcommand_from build' -l release -d 'Optimized'\n\
                 complete -c foo -s v -l verbose -d 'More output'\n";
        let h = parse_fish("foo", f);
        assert_eq!(h.subcommands, vec![("build".to_string(), "Compile it".to_string())]);
        assert!(h.options.contains(&("-v".into(), "More output".into())));
        assert!(h.options.contains(&("--verbose".into(), "More output".into())));
        assert_eq!(h.sub_options["build"], vec![("--release".to_string(), "Optimized".to_string())]);
    }

    #[test]
    fn fuzzy_order() {
        assert_eq!(score("--verbose", "--verb"), Some(0));
        assert!(score("--no-verbose", "verb").unwrap() > 0);
        assert!(score("checkout", "cko").is_some());
        assert!(score("abc", "x").is_none());
    }

    #[test]
    fn paths_complete_in_a_folder() {
        let dir = std::env::temp_dir().join(format!("tessera-complete-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub dir")).unwrap();
        std::fs::write(dir.join("file.txt"), "").unwrap();
        let v: serde_json::Value = serde_json::from_str(&complete("cat su", 6, &dir.to_string_lossy())).unwrap();
        let first = &v["items"][0];
        assert_eq!(first["insert"], "sub\\ dir/");
        assert_eq!(v["start"], 4);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[cfg(test)]
mod live {
    #[test]
    #[ignore]
    fn real_commands() {
        for (line, wait) in [("ls --co", true), ("git sw", true), ("git commit --am", true), ("gre", false), ("cd ~/src/tes", false)] {
            let mut r = String::new();
            for _ in 0..40 {
                r = super::complete(line, line.encode_utf16().count(), &std::env::current_dir().unwrap().to_string_lossy());
                if !wait || !r.contains("\"pending\":true") { break }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let v: serde_json::Value = serde_json::from_str(&r).unwrap();
            let items: Vec<String> = v["items"].as_array().unwrap().iter().take(4).map(|i| format!("{} | {}", i["label"].as_str().unwrap(), i["detail"].as_str().unwrap().chars().take(60).collect::<String>())).collect();
            println!("{line:?} -> {items:#?}");
        }
    }
}
