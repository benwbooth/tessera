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
    /// Longer help for the panel under the list (a command's tldr page).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub doc: String,
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

/// GNU info pages (for commands documented there rather than in man):
/// options as "‘-a’" / "‘--all’" lines with the description indented below.
pub fn parse_info(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let mut flags = Vec::new();
        while i < lines.len() {
            let t = lines[i].trim();
            let q = t.strip_prefix('\u{2018}').and_then(|r| r.strip_suffix('\u{2019}')).or_else(|| t.strip_prefix('`').and_then(|r| r.strip_suffix('\'')));
            match q {
                Some(f) if f.starts_with('-') => {
                    flags.push(f.to_string());
                    i += 1;
                }
                _ => break,
            }
        }
        if flags.is_empty() {
            i += 1;
            continue;
        }
        let mut desc = String::new();
        while i < lines.len() && !lines[i].trim().is_empty() && lines[i].starts_with("     ") {
            if !desc.is_empty() {
                desc.push(' ');
            }
            desc.push_str(lines[i].trim());
            i += 1;
            if desc.len() > 240 {
                break;
            }
        }
        let desc = desc.split_whitespace().collect::<Vec<_>>().join(" ");
        for f in flags {
            let name: String = f.chars().take_while(|c| !matches!(c, '=' | '[' | ' ')).collect();
            if name.len() >= 2 && !out.iter().any(|(n, _)| *n == name) {
                out.push((name, desc.clone()));
            }
        }
    }
    out
}

fn info_options(page: &str) -> Vec<(String, String)> {
    let Ok(o) = std::process::Command::new("info").args(["--output=-", page]).stderr(std::process::Stdio::null()).output() else { return Vec::new() };
    if !o.status.success() {
        return Vec::new();
    }
    parse_info(&String::from_utf8_lossy(&o.stdout))
}

// ---- help: tldr-pages ----------------------------------------------------------

fn tldr_dir() -> PathBuf {
    fish_download_dir().with_file_name("tldr")
}

/// Fetch tldr-pages once, in the background (common, Linux, macOS and
/// Windows pages; English).
fn ensure_tldr() {
    static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if tldr_dir().join("pages").exists() || STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        let _ = update_tldr();
    });
}

/// Download (or update) tldr-pages. Returns how many pages there are.
pub fn update_tldr() -> Result<usize, String> {
    let dir = tldr_dir();
    let d = dir.to_string_lossy().into_owned();
    let git = |args: &[&str]| -> Result<(), String> {
        let ok = std::process::Command::new("git").args(args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map_err(|e| e.to_string())?;
        if ok.success() { Ok(()) } else { Err(format!("git {} failed", args.first().unwrap_or(&""))) }
    };
    if dir.join(".git").exists() {
        git(&["-C", &d, "pull", "-q", "--depth", "1"])?;
    } else {
        let _ = std::fs::create_dir_all(dir.parent().unwrap());
        let tmp = format!("{d}.part");
        let _ = std::fs::remove_dir_all(&tmp);
        git(&["clone", "-q", "--depth", "1", "--filter=blob:none", "--sparse", "https://github.com/tldr-pages/tldr.git", &tmp])?;
        git(&["-C", &tmp, "sparse-checkout", "set", "pages/common", "pages/linux", "pages/osx", "pages/windows"])?;
        std::fs::rename(&tmp, &dir).map_err(|e| e.to_string())?;
    }
    Ok(["common", "linux", "osx", "windows"].iter().map(|p| std::fs::read_dir(dir.join("pages").join(p)).map(|r| r.count()).unwrap_or(0)).sum())
}

pub fn tldr_count() -> usize {
    ["common", "linux", "osx", "windows"].iter().map(|p| std::fs::read_dir(tldr_dir().join("pages").join(p)).map(|r| r.count()).unwrap_or(0)).sum()
}

/// A command's tldr page: its description and examples (what, command).
pub fn tldr_page(cmd: &str) -> Option<(String, Vec<(String, String)>)> {
    ensure_tldr();
    let platform = if cfg!(target_os = "macos") { "osx" } else if cfg!(windows) { "windows" } else { "linux" };
    let base = tldr_dir().join("pages");
    let text = [platform, "common"].iter().find_map(|p| std::fs::read_to_string(base.join(p).join(format!("{cmd}.md"))).ok())?;
    Some(parse_tldr(&text))
}

pub fn parse_tldr(text: &str) -> (String, Vec<(String, String)>) {
    let mut desc = Vec::new();
    let mut examples = Vec::new();
    let mut what: Option<String> = None;
    for line in text.lines() {
        if let Some(d) = line.strip_prefix("> ") {
            if !d.starts_with("More information") {
                desc.push(d.trim().to_string());
            }
        } else if let Some(w) = line.strip_prefix("- ") {
            what = Some(w.trim_end_matches(':').replace(['[', ']'], ""));
        } else if let Some(c) = line.strip_prefix('`').and_then(|l| l.strip_suffix('`')) {
            if let Some(w) = what.take() {
                // "{{path/to/file}}" placeholders read as path/to/file.
                examples.push((w, c.replace("{{", "").replace("}}", "")));
            }
        }
    }
    (desc.join(" "), examples)
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
    let opts = parse_man(&text);
    if opts.is_empty() {
        return info_options(page);
    }
    opts
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
                doc: String::new(),
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

// ---- cd: places to go --------------------------------------------------------

fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_default()
}

/// A path as typed: ~ for home.
fn tilde(p: &str) -> String {
    let h = home_dir();
    if !h.is_empty() && (p == h || p.starts_with(&format!("{h}/"))) { format!("~{}", &p[h.len()..]) } else { p.to_string() }
}

/// Folders by how often and how lately they were visited: zoxide's
/// database (if it's installed), and the history (where commands ran, and
/// where cd commands went). Highest first.
pub fn frecent_dirs() -> Vec<(String, f64)> {
    static CACHE: std::sync::Mutex<Option<(std::time::Instant, Vec<(String, f64)>)>> = std::sync::Mutex::new(None);
    if let Some((at, v)) = CACHE.lock().unwrap().as_ref() {
        if at.elapsed() < std::time::Duration::from_secs(20) {
            return v.clone();
        }
    }
    let mut score: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    if let Ok(o) = std::process::Command::new("zoxide").args(["query", "--list", "--score"]).stderr(std::process::Stdio::null()).output() {
        for l in String::from_utf8_lossy(&o.stdout).lines() {
            let l = l.trim();
            if let Some((sc, path)) = l.split_once(' ') {
                if let Ok(v) = sc.trim().parse::<f64>() {
                    *score.entry(path.trim().to_string()).or_default() += v;
                }
            }
        }
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // (zoxide's weights: within an hour x4, a day x2, a week x0.5, older x0.25.)
    let weight = |t: u64| {
        if t == 0 {
            return 0.25;
        }
        match now.saturating_sub(t) {
            a if a < 3600 => 4.0,
            a if a < 86400 => 2.0,
            a if a < 604800 => 0.5,
            _ => 0.25,
        }
    };
    let home = home_dir();
    for e in crate::history::all() {
        let cwd = e.cwd.strip_prefix("file://").map(|r| r.find('/').map(|i| r[i..].to_string()).unwrap_or_default()).unwrap_or(e.cwd.clone());
        if !cwd.is_empty() {
            *score.entry(cwd.clone()).or_default() += weight(e.t);
        }
        // (cd somewhere: there, as the shell would resolve it.)
        let mut w = e.cmd.split_whitespace();
        if matches!(w.next(), Some("cd" | "pushd")) {
            if let Some(a) = w.next().filter(|a| !a.starts_with('-')) {
                let a = a.trim_matches(|c| c == '\'' || c == '"');
                let p = if let Some(r) = a.strip_prefix("~") { format!("{home}{r}") } else if a.starts_with('/') { a.to_string() } else if !cwd.is_empty() { format!("{cwd}/{a}") } else { continue };
                let p = Path::new(&p).components().fold(PathBuf::new(), |mut acc, c| {
                    match c {
                        std::path::Component::ParentDir => {
                            acc.pop();
                        }
                        std::path::Component::CurDir => {}
                        c => acc.push(c),
                    }
                    acc
                });
                *score.entry(p.to_string_lossy().trim_end_matches('/').to_string()).or_default() += weight(e.t) * 2.0;
            }
        }
    }
    let mut v: Vec<(String, f64)> = score.into_iter().filter(|(p, _)| !p.is_empty() && Path::new(p).is_dir()).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.truncate(400);
    *CACHE.lock().unwrap() = Some((std::time::Instant::now(), v.clone()));
    v
}

/// Folders up to three levels under `dir` (a bounded walk), hidden ones and
/// build output left out.
fn folders_below(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut level = vec![dir.to_path_buf()];
    for _ in 0..3 {
        let mut next = Vec::new();
        for d in level {
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let n = e.file_name();
                let n = n.to_string_lossy();
                if n.starts_with('.') || matches!(n.as_ref(), "node_modules" | "target" | "__pycache__" | "result") {
                    continue;
                }
                if e.file_type().is_ok_and(|t| t.is_dir()) {
                    next.push(e.path());
                    if out.len() + next.len() > 3000 {
                        out.extend(next);
                        return out;
                    }
                }
            }
        }
        out.extend(next.iter().cloned());
        level = next;
    }
    out
}

/// Where `cd <typed>` might go besides the folder's own subfolders:
/// frequent and recent places, folders further down, CDPATH's.
fn cd_places(typed: &str, cwd: &str) -> Vec<Item> {
    let t = typed.trim_end_matches('/');
    let mut out: Vec<(u32, f64, Item)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut add = |path: &str, rank: u32, weight: f64, detail: &str, out: &mut Vec<(u32, f64, Item)>| {
        let p = path.trim_end_matches('/');
        if p.is_empty() || p == cwd.trim_end_matches('/') || !seen.insert(p.to_string()) {
            return;
        }
        let shown = format!("{}/", tilde(p));
        out.push((rank, weight, Item { insert: shell_quote_path(&shown).replace("\\~", "~"), label: shown, detail: detail.to_string(), kind: "dir", doc: String::new() }));
    };
    // How well a folder matches, as zoxide does: what's typed matches its
    // name (closely, else letter by letter); typed with slashes, each part
    // is in one of its folders, in order, the last part in its name.
    let matches = |p: &str| -> Option<u32> {
        if t.is_empty() {
            return Some(50);
        }
        let comps: Vec<String> = tilde(p).split('/').filter(|c| !c.is_empty()).map(|c| c.to_lowercase()).collect();
        let name = comps.last()?.clone();
        if !t.contains('/') {
            return score(&name, t);
        }
        let parts: Vec<String> = t.split('/').filter(|x| !x.is_empty()).map(|x| x.to_lowercase()).collect();
        let (last, rest) = parts.split_last()?;
        if !name.contains(last.as_str()) {
            return None;
        }
        let mut i = 0;
        for part in rest {
            while i < comps.len() - 1 && !comps[i].contains(part.as_str()) {
                i += 1;
            }
            if i >= comps.len() - 1 {
                return None;
            }
            i += 1;
        }
        Some(if name.starts_with(last.as_str()) { 5 } else { 15 })
    };
    // (Temporary folders only when asked for.)
    let tmp = std::env::temp_dir().to_string_lossy().to_string();
    let want_tmp = t.starts_with("/tmp") || t.starts_with(&tmp);
    for (p, w) in frecent_dirs() {
        if !want_tmp && (p.starts_with("/tmp/") || p.starts_with(&format!("{tmp}/"))) {
            continue;
        }
        if let Some(m) = matches(&p) {
            add(&p, m, w, "often", &mut out);
        }
    }
    if !t.is_empty() && !t.contains('/') {
        for d in folders_below(Path::new(cwd)) {
            // (The folder's own subfolders are listed already.)
            if d.parent() == Some(Path::new(cwd)) {
                continue;
            }
            let p = d.to_string_lossy().to_string();
            let name = p.rsplit('/').next().unwrap_or(&p).to_string();
            if let Some(m) = score(&name, t).filter(|s| *s < 100) {
                add(&p, m + 30, 0.0, "below", &mut out);
            }
        }
        if let Ok(cdpath) = std::env::var("CDPATH") {
            for base in cdpath.split(':').filter(|b| !b.is_empty()) {
                for e in std::fs::read_dir(base).into_iter().flatten().flatten() {
                    let p = e.path().to_string_lossy().to_string();
                    if e.path().is_dir() && score(&e.file_name().to_string_lossy(), t).is_some_and(|s| s < 100) {
                        add(&p, 40, 0.0, "CDPATH", &mut out);
                    }
                }
            }
        }
    }
    // Best match, then most visited.
    out.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.total_cmp(&a.1)));
    let limit = if t.is_empty() { 15 } else { 60 };
    out.into_iter().take(limit).map(|(_, _, i)| i).collect()
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

// ---- aliases ----------------------------------------------------------------------

/// The shell's `alias` output (zsh: name=value; bash: alias name='value';
/// fish: alias name 'value') as name -> value.
pub fn parse_aliases(text: &str) -> std::collections::BTreeMap<String, String> {
    let unquote = |v: &str| -> String {
        let v = v.trim();
        if v.len() >= 2 && v.starts_with('\'') && v.ends_with('\'') {
            v[1..v.len() - 1].replace("'\\''", "'")
        } else if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
            v[1..v.len() - 1].to_string()
        } else {
            v.to_string()
        }
    };
    let mut out = std::collections::BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        let (name, value) = if let Some(rest) = line.strip_prefix("alias ") {
            match rest.split_once('=') {
                // bash: name='value'
                Some((n, v)) if !n.contains(' ') => (n.trim_start_matches("-- ").to_string(), unquote(v)),
                // fish: name value
                _ => match rest.split_once(' ') {
                    Some((n, v)) => (n.to_string(), unquote(v)),
                    None => continue,
                },
            }
        } else if let Some((n, v)) = line.split_once('=') {
            (n.trim_matches('\'').to_string(), unquote(v))
        } else {
            continue;
        };
        if !name.is_empty() && !value.is_empty() {
            out.insert(name, value);
        }
    }
    out
}

/// What an alias runs: the first word of its value (after assignments and
/// sudo-likes); None when it isn't one.
fn alias_target(aliases: &serde_json::Value, cmd: &str) -> Option<String> {
    let v = aliases.get(cmd)?.as_str()?;
    let w = words(v);
    let first = w.first()?.text.clone();
    (first != cmd && !first.is_empty()).then_some(first)
}

// ---- the API ---------------------------------------------------------------------

/// Completions at `cursor` (a UTF-16 offset) in `text`, for a shell in `cwd`.
/// {"start": u16, "end": u16, "items": [...], "pending": bool}: replace
/// start..end with an item's `insert`. `pending`: help is still being read.
thread_local! {
    static SHELL_ENV: std::cell::RefCell<Option<Vec<(String, String)>>> = const { std::cell::RefCell::new(None) };
}

/// The shell's environment for the next completions (variables).
pub fn set_shell_env(env: Option<Vec<(String, String)>>) {
    SHELL_ENV.with(|e| *e.borrow_mut() = env);
}

fn shell_vars() -> Vec<(String, String)> {
    let mut v = SHELL_ENV.with(|e| e.borrow().clone()).unwrap_or_else(|| std::env::vars().collect());
    v.sort();
    v
}

pub fn complete(text: &str, cursor_u16: usize, cwd: &str, aliases: &serde_json::Value) -> String {
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
                .map(|(n, d)| Item { label: n.to_string(), insert: n.to_string(), detail: d.to_string(), kind: "command", doc: String::new() })
                .collect();
            // The shell's aliases (what they run as the detail).
            if let Some(map) = aliases.as_object() {
                cands.extend(map.iter().map(|(n, v)| Item { label: n.clone(), insert: n.clone(), detail: format!("alias for {}", v.as_str().unwrap_or("")), kind: "command", doc: String::new() }));
            }
            cands.extend(path_commands().iter().map(|n| Item { label: n.clone(), insert: n.clone(), detail: String::new(), kind: "command", doc: String::new() }));
            let mut seen = HashSet::new();
            cands.retain(|i| seen.insert(i.label.clone()));
            items = rank(cands, &typed, |i| i.label.clone());
            items.truncate(60);
            let names: Vec<String> = items.iter().filter(|i| i.detail.is_empty()).take(30).map(|i| i.label.clone()).collect();
            let descs = describe_commands(&names);
            for (n, i) in items.iter_mut().enumerate() {
                // tldr-pages: what it is, and its common uses.
                if n < 30 {
                    let real = if i.detail.starts_with("alias for ") { alias_target(aliases, &i.label).unwrap_or(i.label.clone()) } else { i.label.clone() };
                    if let Some((desc, examples)) = tldr_page(&real) {
                        if i.detail.is_empty() {
                            i.detail = desc.clone();
                        } else if i.detail.starts_with("alias for ") {
                            i.detail = format!("{} \u{2014} {desc}", i.detail);
                        }
                        let mut doc = desc;
                        for (what, cmd) in examples.iter().take(6) {
                            doc.push_str(&format!("\n\u{2022} {what}\n    {cmd}"));
                        }
                        i.doc = doc;
                    }
                }
                if i.detail.is_empty() {
                    i.detail = descs.get(&i.label).cloned().unwrap_or_default();
                }
            }
        }
    } else {
        let typed_cmd = ws[0].text.rsplit('/').next().unwrap_or("").to_string();
        // An alias's help is its command's (ls -> lsd).
        let cmd = alias_target(aliases, &typed_cmd).unwrap_or(typed_cmd);
        let args: Vec<&str> = ws[1..ws.len() - 1].iter().map(|w| w.text.as_str()).collect();
        let h = help(&cmd);
        pending |= h.is_none();
        let h = h.unwrap_or_default();
        // The subcommand, if one was given (the first argument that isn't an option).
        let sub = args.iter().find(|a| !a.starts_with('-')).map(|s| s.to_string());
        let sub_known = sub.as_ref().is_some_and(|s| h.subcommands.iter().any(|(n, _)| n == s));
        if (cmd == "export" || cmd == "unset") && !typed.contains('=') {
            // (Variables to set or clear: the shell's, with their values.)
            let set = cmd == "export";
            items = rank(
                shell_vars()
                    .into_iter()
                    .map(|(k, v)| Item {
                        label: k.clone(),
                        insert: if set { format!("{k}=") } else { k.clone() },
                        detail: v.chars().take(80).collect(),
                        kind: "variable",
                        doc: String::new(),
                    })
                    .collect(),
                &typed,
                |i| i.label.clone(),
            );
        } else if let Some(var) = typed.strip_prefix('$') {
            items = rank(
                shell_vars().into_iter().map(|(k, v)| Item { label: format!("${k}"), insert: format!("${k}"), detail: v.chars().take(80).collect(), kind: "variable", doc: String::new() }).collect(),
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
            let cands = opts.into_iter().map(|(f, d)| Item { insert: f.clone(), label: f, detail: d, kind: "option", doc: String::new() }).collect();
            items = rank(cands, &typed, |i| i.label.clone());
        } else {
            if sub.is_none() && !h.subcommands.is_empty() {
                items.extend(rank(
                    h.subcommands.iter().map(|(n, d)| Item { label: n.clone(), insert: n.clone(), detail: d.clone(), kind: "subcommand", doc: String::new() }).collect(),
                    &typed,
                    |i| i.label.clone(),
                ));
            }
            if args.is_empty() && !h.values.is_empty() {
                items.extend(rank(
                    h.values.iter().map(|(n, d)| Item { label: n.clone(), insert: n.clone(), detail: d.clone(), kind: "value", doc: String::new() }).collect(),
                    &typed,
                    |i| i.label.clone(),
                ));
            }
            let git_ref = cmd == "git" && sub.as_deref().is_some_and(|s| ["checkout", "switch", "merge", "rebase", "branch", "log", "diff", "reset", "cherry-pick", "show", "push", "pull"].contains(&s));
            if git_ref {
                items.extend(rank(
                    git_branches(cwd).into_iter().map(|b| Item { insert: b.clone(), label: b, detail: String::new(), kind: "branch", doc: String::new() }).collect(),
                    &typed,
                    |i| i.label.clone(),
                ));
            }
            if ["ssh", "scp", "sftp", "mosh", "rsync"].contains(&cmd.as_str()) && !typed.contains('/') {
                items.extend(rank(
                    crate::launchers::launchers().ssh_hosts.into_iter().map(|h| Item { insert: h.clone(), label: h, detail: "ssh host".into(), kind: "host", doc: String::new() }).collect(),
                    &typed,
                    |i| i.label.clone(),
                ));
            }
            let dirs_only = matches!(cmd.as_str(), "cd" | "pushd" | "rmdir" | "z");
            items.extend(rank(paths(&typed, cwd, dirs_only), name_of(&typed), |i| i.label.clone()));
            // (cd: also where you go often or lately, folders further down, CDPATH.)
            if matches!(cmd.as_str(), "cd" | "pushd" | "z") && args.is_empty() {
                let places = cd_places(&typed, cwd);
                // (A subfolder matching only letter by letter gives way to
                // a place that matches properly.)
                if !places.is_empty() && !name_of(&typed).is_empty() {
                    let t = name_of(&typed).to_string();
                    items.retain(|i| i.kind != "dir" || score(i.label.trim_end_matches('/'), &t).is_some_and(|s| s < 100));
                }
                let have: std::collections::HashSet<String> = items.iter().map(|i| i.insert.clone()).collect();
                items.extend(places.into_iter().filter(|i| !have.contains(&i.insert)));
            }
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
    fn aliases() {
        let z = parse_aliases("ll='lsd -l'\nls=lsd\n");
        assert_eq!(z["ls"], "lsd");
        assert_eq!(z["ll"], "lsd -l");
        let b = parse_aliases("alias ll='lsd -l'\nalias ls='lsd'\n");
        assert_eq!(b["ll"], "lsd -l");
        let f = parse_aliases("alias ls 'lsd'\nalias g git\n");
        assert_eq!((f["ls"].as_str(), f["g"].as_str()), ("lsd", "git"));
        let m = serde_json::json!({"ls": "lsd --group-dirs first", "ll": "sudo lsd -l"});
        assert_eq!(alias_target(&m, "ls").as_deref(), Some("lsd"));
        assert_eq!(alias_target(&m, "ll").as_deref(), Some("lsd"));
    }

    #[test]
    fn tldr_and_info() {
        let (d, ex) = parse_tldr("# tar\n\n> Archiving utility.\n> More information: <x>.\n\n- [c]reate an archive:\n\n`tar cf {{target.tar}} {{file}}`\n");
        assert_eq!(d, "Archiving utility.");
        assert_eq!(ex, vec![("create an archive".to_string(), "tar cf target.tar file".to_string())]);
        let o = parse_info("‘-a’\n‘--all’\n     Show all\n     entries.\n\n");
        assert_eq!(o, vec![("-a".to_string(), "Show all entries.".to_string()), ("--all".to_string(), "Show all entries.".to_string())]);
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
        let v: serde_json::Value = serde_json::from_str(&complete("cat su", 6, &dir.to_string_lossy(), &serde_json::Value::Null)).unwrap();
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
                r = super::complete(line, line.encode_utf16().count(), &std::env::current_dir().unwrap().to_string_lossy(), &serde_json::Value::Null);
                if !wait || !r.contains("\"pending\":true") { break }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let v: serde_json::Value = serde_json::from_str(&r).unwrap();
            let items: Vec<String> = v["items"].as_array().unwrap().iter().take(4).map(|i| format!("{} | {}", i["label"].as_str().unwrap(), i["detail"].as_str().unwrap().chars().take(60).collect::<String>())).collect();
            println!("{line:?} -> {items:#?}");
        }
    }
}

