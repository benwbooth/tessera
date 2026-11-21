//! The notebook's file browser (qml/FileBrowser.qml, an `ls` card): a
//! folder's entries with what `ls -l` shows, and the actions its menus run
//! on them (open, rename, trash, new folder, ...), done here rather than as
//! commands typed into the shell.

use serde_json::json;
use std::collections::HashMap;

// (Unix's mode bits and owners; elsewhere, what can be told.)
#[cfg(unix)]
fn mode_of(m: &std::fs::Metadata) -> u32 {
    std::os::unix::fs::MetadataExt::mode(m)
}
#[cfg(not(unix))]
fn mode_of(m: &std::fs::Metadata) -> u32 {
    let base = if m.permissions().readonly() { 0o444 } else { 0o644 };
    if m.is_dir() { base | 0o111 } else { base }
}
#[cfg(unix)]
fn owner_of(m: &std::fs::Metadata) -> (u32, u32) {
    (std::os::unix::fs::MetadataExt::uid(m), std::os::unix::fs::MetadataExt::gid(m))
}
#[cfg(not(unix))]
fn owner_of(_: &std::fs::Metadata) -> (u32, u32) {
    (0, 0)
}
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// uid -> name and gid -> name, from /etc/passwd and /etc/group.
fn names(file: &str) -> &'static HashMap<u32, String> {
    static USERS: OnceLock<HashMap<u32, String>> = OnceLock::new();
    static GROUPS: OnceLock<HashMap<u32, String>> = OnceLock::new();
    let cell = if file == "/etc/passwd" { &USERS } else { &GROUPS };
    cell.get_or_init(|| {
        std::fs::read_to_string(file)
            .unwrap_or_default()
            .lines()
            .filter_map(|l| {
                let f: Vec<&str> = l.split(':').collect();
                Some((f.get(2)?.parse().ok()?, f.first()?.to_string()))
            })
            .collect()
    })
}

/// `ls -l`'s mode column: drwxr-xr-x, with setuid/sticky marks.
fn mode_string(m: u32, is_dir: bool, is_link: bool) -> String {
    let mut s = String::with_capacity(10);
    s.push(if is_link { 'l' } else if is_dir { 'd' } else { '-' });
    for (shift, special, mark) in [(6, 0o4000, 's'), (3, 0o2000, 's'), (0, 0o1000, 't')] {
        let bits = (m >> shift) & 7;
        s.push(if bits & 4 != 0 { 'r' } else { '-' });
        s.push(if bits & 2 != 0 { 'w' } else { '-' });
        let x = bits & 1 != 0;
        s.push(match (m & special != 0, x) {
            (true, true) => mark,
            (true, false) => mark.to_ascii_uppercase(),
            (false, true) => 'x',
            (false, false) => '-',
        });
    }
    s
}

/// What a file is, by its name (for icons and colors, as lsd does).
fn kind_of(name: &str, is_dir: bool, exec: bool) -> &'static str {
    if is_dir {
        return "folder";
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "ico" | "tif" | "tiff" | "avif" | "heic" => "image",
        "mp4" | "mkv" | "webm" | "mov" | "avi" | "m4v" => "video",
        "mp3" | "flac" | "ogg" | "opus" | "wav" | "m4a" | "aac" => "audio",
        "zip" | "tar" | "gz" | "tgz" | "xz" | "bz2" | "zst" | "7z" | "rar" | "deb" | "rpm" | "iso" => "archive",
        "pdf" | "epub" | "djvu" => "document",
        "md" | "txt" | "rst" | "org" | "log" => "text",
        "json" | "toml" | "yaml" | "yml" | "xml" | "ini" | "conf" | "cfg" | "lock" | "nix" | "csv" => "config",
        "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "c" | "h" | "cpp" | "hpp" | "cc" | "go" | "java" | "kt" | "rb" | "lua" | "sh" | "zsh"
        | "fish" | "qml" | "html" | "css" | "hs" | "ml" | "scala" | "swift" | "zig" | "php" | "pl" | "r" | "jl" | "ex" | "exs" => "code",
        _ if exec => "executable",
        _ => "file",
    }
}

// ---- big files, read a part at a time (the viewer streams) ----------------

struct LineIndex {
    size: u64,
    mtime: Option<std::time::SystemTime>,
    /// Where each line starts (the first is 0).
    starts: Vec<u64>,
    done: bool,
    error: String,
}

fn indexes() -> &'static std::sync::Mutex<HashMap<String, std::sync::Arc<std::sync::Mutex<LineIndex>>>> {
    static I: OnceLock<std::sync::Mutex<HashMap<String, std::sync::Arc<std::sync::Mutex<LineIndex>>>>> = OnceLock::new();
    I.get_or_init(Default::default)
}

/// A file's lines, counted in the background (started on the first ask, and
/// again when the file changes): {"lines", "done", "size"} — lines so far
/// while it's still counting.
pub fn line_index(path: &str) -> String {
    let md = std::fs::metadata(path);
    let (size, mtime) = match &md {
        Ok(m) => (m.len(), m.modified().ok()),
        Err(e) => return json!({"lines": 0, "done": true, "size": 0, "error": e.to_string()}).to_string(),
    };
    let ix = {
        let mut all = indexes().lock().unwrap();
        let fresh = all.get(path).is_some_and(|i| {
            let i = i.lock().unwrap();
            i.size == size && i.mtime == mtime
        });
        if !fresh {
            let ix = std::sync::Arc::new(std::sync::Mutex::new(LineIndex { size, mtime, starts: vec![0], done: false, error: String::new() }));
            all.insert(path.to_string(), ix.clone());
            let (p, i2) = (path.to_string(), ix.clone());
            std::thread::spawn(move || {
                use std::io::Read;
                let mut f = match std::fs::File::open(&p) {
                    Ok(f) => f,
                    Err(e) => {
                        let mut i = i2.lock().unwrap();
                        i.error = e.to_string();
                        i.done = true;
                        return;
                    }
                };
                let mut buf = vec![0u8; 4 << 20];
                let mut at: u64 = 0;
                loop {
                    let n = match f.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    let mut found = Vec::new();
                    for (k, &b) in buf[..n].iter().enumerate() {
                        if b == b'\n' {
                            found.push(at + k as u64 + 1);
                        }
                    }
                    at += n as u64;
                    i2.lock().unwrap().starts.extend(found);
                }
                let mut i = i2.lock().unwrap();
                // (A file ending in a newline has no line after it.)
                if i.starts.len() > 1 && *i.starts.last().unwrap() >= at {
                    i.starts.pop();
                }
                i.size = at.max(i.size);
                i.done = true;
            });
        }
        all.get(path).cloned().unwrap()
    };
    let i = ix.lock().unwrap();
    // (Still counting: the lines known to be whole.)
    let lines = if i.done { if i.size == 0 { 0 } else { i.starts.len() } } else { i.starts.len().saturating_sub(1) };
    json!({"lines": lines, "done": i.done, "size": i.size, "error": i.error}).to_string()
}

/// Lines start .. start+count of a file (counted by line_index), as text.
/// (A very long line is cut, so a part never gets huge.)
pub fn read_lines(path: &str, start: usize, count: usize) -> String {
    let Some(ix) = indexes().lock().unwrap().get(path).cloned() else { return String::new() };
    let (from, to) = {
        let i = ix.lock().unwrap();
        if start >= i.starts.len() {
            return String::new();
        }
        let end = start + count;
        let to = if end < i.starts.len() { i.starts[end] } else if i.done { i.size } else { *i.starts.last().unwrap() };
        (i.starts[start], to)
    };
    if to <= from {
        return String::new();
    }
    use std::io::{Read, Seek};
    let Ok(mut f) = std::fs::File::open(path) else { return String::new() };
    if f.seek(std::io::SeekFrom::Start(from)).is_err() {
        return String::new();
    }
    let want = (to - from).min(8 << 20) as usize;
    let mut buf = vec![0u8; want];
    let n = f.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    let text = String::from_utf8_lossy(&buf);
    let mut out = String::with_capacity(text.len());
    for (k, line) in text.split_terminator('\n').enumerate() {
        if k > 0 {
            out.push('\n');
        }
        // (Windows line ends: the \r goes, or the highlighting, counted with
        // it, drifts a character a line from the text shown without it.)
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.chars().count() > 4000 {
            out.extend(line.chars().take(4000));
            out.push_str(" …");
        } else {
            out.push_str(line);
        }
    }
    out
}

// ---- editing a file (a file card's editor) ------------------------------------

/// A file whole, to edit: {"text" (Windows line ends as \n), "crlf", "error"}.
/// (Up to 20 MB: past that it's too big to edit here.)
pub fn read_for_edit(path: &str) -> String {
    const MAX: u64 = 20 << 20;
    match std::fs::metadata(path) {
        Ok(m) if m.len() > MAX => return json!({"error": format!("It's too big to edit here ({} MB; up to 20 MB).", m.len() >> 20)}).to_string(),
        Err(e) => return json!({"error": e.to_string()}).to_string(),
        _ => {}
    }
    match std::fs::read(path) {
        Ok(b) => {
            if b.contains(&0) {
                return json!({"error": "It's a binary file."}).to_string();
            }
            let t = String::from_utf8_lossy(&b);
            let crlf = t.contains("\r\n");
            json!({"text": if crlf { t.replace("\r\n", "\n") } else { t.into_owned() }, "crlf": crlf}).to_string()
        }
        Err(e) => json!({"error": e.to_string()}).to_string(),
    }
}

/// Write an edited file back, in place (its permissions, owner and links kept),
/// with the line ends it had. "" when done, else what went wrong.
pub fn save_edited(path: &str, text: &str, crlf: bool) -> String {
    let body = if crlf { text.replace('\n', "\r\n") } else { text.to_string() };
    match std::fs::OpenOptions::new().write(true).truncate(true).create(true).open(path) {
        Ok(mut f) => {
            use std::io::Write;
            match f.write_all(body.as_bytes()).and_then(|_| f.flush()) {
                Ok(()) => String::new(),
                Err(e) => e.to_string(),
            }
        }
        Err(e) => e.to_string(),
    }
}

/// Whether a file reads as text: by its kind, else (no telling kind) its
/// first bytes: no NULs, and UTF-8.
fn is_text(p: &Path, kind: &str, size: u64) -> bool {
    match kind {
        "text" | "config" | "code" => true,
        "image" | "video" | "audio" | "archive" | "document" | "folder" => false,
        _ => {
            if size == 0 {
                return true;
            }
            use std::io::Read;
            let mut buf = [0u8; 1024];
            let Ok(n) = std::fs::File::open(p).and_then(|mut f| f.read(&mut buf)) else { return false };
            let b = &buf[..n];
            if b.contains(&0) {
                return false;
            }
            // (A character cut off at the end of the sample is fine.)
            match std::str::from_utf8(b) {
                Ok(_) => true,
                Err(e) => e.error_len().is_none(),
            }
        }
    }
}

/// A folder's entries: JSON [{name, path, dir, link, target, exec, hidden,
/// size, items (a folder's), mtime, mode, user, group, kind}].
pub fn list(dir: &str, all: bool) -> String {
    let mut out: Vec<serde_json::Value> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            let hidden = name.starts_with('.');
            if hidden && !all {
                continue;
            }
            let p = e.path();
            let lmd = std::fs::symlink_metadata(&p).ok();
            let link = lmd.as_ref().is_some_and(|m| m.file_type().is_symlink());
            let md = std::fs::metadata(&p).ok().or(lmd.clone());
            let Some(md) = md else { continue };
            let is_dir = md.is_dir();
            let mode = mode_of(&md);
            let exec = !is_dir && mode & 0o111 != 0;
            let items = if is_dir { std::fs::read_dir(&p).map(|r| r.count() as i64).unwrap_or(-1) } else { -1 };
            let target = if link { std::fs::read_link(&p).map(|t| t.to_string_lossy().to_string()).unwrap_or_default() } else { String::new() };
            let mtime = md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
            out.push(json!({
                "name": name, "path": p.to_string_lossy(), "dir": is_dir, "link": link, "target": target, "exec": exec,
                "hidden": hidden, "size": md.len(), "items": items, "mtime": mtime,
                "mode": mode_string(lmd.as_ref().map(mode_of).unwrap_or(mode), is_dir, link),
                "user": names("/etc/passwd").get(&owner_of(&md).0).cloned().unwrap_or_else(|| owner_of(&md).0.to_string()),
                "group": names("/etc/group").get(&owner_of(&md).1).cloned().unwrap_or_else(|| owner_of(&md).1.to_string()),
                "kind": kind_of(&name, is_dir, exec),
                "text": !is_dir && is_text(&p, kind_of(&name, is_dir, exec), md.len()),
            }));
        }
    }
    serde_json::Value::Array(out).to_string()
}

pub fn unused_name(dir: &Path, base: &str) -> PathBuf {
    let p = dir.join(base);
    if !p.exists() {
        return p;
    }
    let (stem, ext) = match base.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (base.to_string(), String::new()),
    };
    (2..).map(|n| dir.join(format!("{stem} ({n}){ext}"))).find(|p| !p.exists()).unwrap()
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for e in std::fs::read_dir(from)?.flatten() {
            copy_tree(&e.path(), &to.join(e.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::copy(from, to).map(|_| ())
    }
}

fn spawn(prog: &str, args: &[&str]) -> Result<(), String> {
    std::process::Command::new(prog)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Couldn't run {prog}: {e}"))
}

/// Run it in `dir`, waiting: "" or what it said went wrong.
fn run_in(cmd: &mut std::process::Command, dir: &Path) -> Result<(), String> {
    let o = cmd.current_dir(dir).stdin(std::process::Stdio::null()).output().map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(())
    } else {
        let e = String::from_utf8_lossy(&o.stderr).trim().to_string();
        Err(if e.is_empty() { "It failed.".into() } else { e })
    }
}

// ---- the desktop's apps for a file (freedesktop: xdg-mime, mimeinfo.cache,
// mimeapps.list, .desktop files) ----

/// Its MIME type, as the desktop has it ("inode/directory" for a folder).
fn mime_of(path: &str) -> String {
    if Path::new(path).is_dir() {
        return "inode/directory".into();
    }
    std::process::Command::new("xdg-mime")
        .args(["query", "filetype", path])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// The folders .desktop files are in: the user's first.
fn app_dirs() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut out = vec![std::env::var("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|_| Path::new(&home).join(".local/share")).join("applications")];
    let data = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    out.extend(data.split(':').filter(|d| !d.is_empty()).map(|d| Path::new(d).join("applications")));
    out
}

fn desktop_file(id: &str) -> Option<PathBuf> {
    app_dirs().into_iter().map(|d| d.join(id)).find(|f| f.is_file())
}

/// A .desktop file's [Desktop Entry] keys.
fn desktop_entry(file: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut inside = false;
    for l in std::fs::read_to_string(file).unwrap_or_default().lines() {
        let l = l.trim();
        if l.starts_with('[') {
            inside = l == "[Desktop Entry]";
            continue;
        }
        if let (true, Some((k, v))) = (inside, l.split_once('=')) {
            out.entry(k.trim().to_string()).or_insert_with(|| v.trim().to_string());
        }
    }
    out
}

/// The apps that open `path`'s kind of file: [{id, name, icon, default}],
/// the default first, then by name. ("[]" for another host's file.)
pub fn apps_for(path: &str) -> String {
    let mime = mime_of(path);
    if mime.is_empty() {
        return "[]".into();
    }
    // (Text of any kind: the plain-text apps too.)
    let mut mimes = vec![mime.clone()];
    if mime.starts_with("text/") && mime != "text/plain" {
        mimes.push("text/plain".into());
    }
    let mut ids: Vec<String> = Vec::new();
    let mut add = |id: &str| {
        let id = id.trim();
        if !id.is_empty() && !ids.iter().any(|x| x == id) {
            ids.push(id.to_string());
        }
    };
    let home = std::env::var("HOME").unwrap_or_default();
    let config = std::env::var("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|_| Path::new(&home).join(".config"));
    // (The user's own choices, mimeapps.list's Default and Added, then every app that says it takes it.)
    let lists: Vec<PathBuf> = std::iter::once(config.join("mimeapps.list")).chain(app_dirs().into_iter().map(|d| d.join("mimeapps.list"))).collect();
    for f in &lists {
        let text = std::fs::read_to_string(f).unwrap_or_default();
        let mut section = String::new();
        for l in text.lines() {
            let l = l.trim();
            if l.starts_with('[') {
                section = l.to_string();
                continue;
            }
            if section == "[Default Applications]" || section == "[Added Associations]" {
                if let Some((k, v)) = l.split_once('=') {
                    if mimes.iter().any(|m| m == k.trim()) {
                        v.split(';').for_each(&mut add);
                    }
                }
            }
        }
    }
    for d in app_dirs() {
        let text = std::fs::read_to_string(d.join("mimeinfo.cache")).unwrap_or_default();
        for l in text.lines() {
            if let Some((k, v)) = l.split_once('=') {
                if mimes.iter().any(|m| m == k.trim()) {
                    v.split(';').for_each(&mut add);
                }
            }
        }
    }
    let default = std::process::Command::new("xdg-mime")
        .args(["query", "default", &mime])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let mut out: Vec<serde_json::Value> = ids
        .iter()
        .filter_map(|id| {
            let e = desktop_entry(&desktop_file(id)?);
            if e.get("Hidden").is_some_and(|v| v == "true") || !e.contains_key("Exec") {
                return None;
            }
            Some(json!({"id": id, "name": e.get("Name").cloned().unwrap_or_else(|| id.trim_end_matches(".desktop").to_string()),
                        "icon": e.get("Icon").cloned().unwrap_or_default(), "default": *id == default}))
        })
        .collect();
    out.sort_by(|a, b| b["default"].as_bool().cmp(&a["default"].as_bool()).then_with(|| a["name"].as_str().unwrap_or("").to_lowercase().cmp(&b["name"].as_str().unwrap_or("").to_lowercase())));
    json!({"mime": mime, "apps": out}).to_string()
}

/// Start the app `id` on `path`: its Exec line with the file put in.
fn open_with(id: &str, path: &str) -> Result<(), String> {
    let file = desktop_file(id).ok_or_else(|| format!("No app {id}."))?;
    let e = desktop_entry(&file);
    let exec = e.get("Exec").ok_or("It has no Exec line.")?;
    let words = shell_words::split(exec).map_err(|e| e.to_string())?;
    let url = format!("file://{}", path.split('/').map(|s| s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect::<String>()).collect::<Vec<_>>().join("/"));
    let mut argv: Vec<String> = Vec::new();
    let mut put = false;
    for w in words {
        match w.as_str() {
            "%f" | "%F" => { argv.push(path.to_string()); put = true }
            "%u" | "%U" => { argv.push(url.clone()); put = true }
            "%i" => { if let Some(i) = e.get("Icon") { argv.push("--icon".into()); argv.push(i.clone()) } }
            "%c" => argv.push(e.get("Name").cloned().unwrap_or_default()),
            "%k" => argv.push(file.to_string_lossy().to_string()),
            _ => argv.push(w.replace("%%", "\u{1}").replace("%f", path).replace("%u", &url).split('%').next().unwrap_or("").replace('\u{1}', "%")),
        }
    }
    if !put {
        argv.push(path.to_string());
    }
    // (A terminal app: in a terminal of its own.)
    if e.get("Terminal").is_some_and(|v| v == "true") {
        let me = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|_| "tessera".into());
        argv = vec![me, "--new-window".into(), "--run".into(), shell_words::join(&argv)];
    }
    let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
    spawn(refs[0], &refs[1..])
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
}

/// Do `op` on `path`: "" when done, else what went wrong.
pub fn action(op: &str, path: &str, arg: &str) -> String {
    let p = Path::new(path);
    let parent = p.parent().unwrap_or(Path::new("/"));
    let r: Result<(), String> = match op {
        "open" => spawn("xdg-open", &[path]),
        // (A recording as a GIF beside it, by agg, in the background; then
        // the GIF opens.)
        "gif" => {
            let gif = p.with_extension("gif").to_string_lossy().into_owned();
            let agg = if on_path("agg") { "agg".to_string() } else { "nix run nixpkgs#agg --".to_string() };
            let q = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
            spawn("sh", &["-c", &format!("{agg} {} {} && xdg-open {}", q(path), q(&gif), q(&gif))])
        }
        // (The file manager, with the item selected: FileManager1 over D-Bus.)
        "reveal" | "reveal_uri" => {
            let uri = if op == "reveal_uri" { path.to_string() } else { format!("file://{path}") };
            let ok = std::process::Command::new("dbus-send")
                .args(["--session", "--dest=org.freedesktop.FileManager1", "--type=method_call", "/org/freedesktop/FileManager1", "org.freedesktop.FileManager1.ShowItems", &format!("array:string:{uri}"), "string:"])
                .status()
                .is_ok_and(|s| s.success());
            if ok { Ok(()) } else { spawn("xdg-open", &[&parent.to_string_lossy()]) }
        }
        "trash" => {
            if on_path("gio") {
                std::process::Command::new("gio").args(["trash", "--", path]).output().map_err(|e| e.to_string()).and_then(|o| {
                    if o.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&o.stderr).trim().to_string()) }
                })
            } else if on_path("trash-put") {
                std::process::Command::new("trash-put").arg(path).status().map_err(|e| e.to_string()).and_then(|s| if s.success() { Ok(()) } else { Err("trash-put failed".into()) })
            } else {
                Err("No trash command (gio or trash-put).".into())
            }
        }
        "rename" => {
            let to = parent.join(arg);
            if arg.is_empty() || arg.contains('/') {
                Err("A name can't be empty or have a /.".into())
            } else if to.exists() {
                Err(format!("{arg} is already there."))
            } else {
                std::fs::rename(p, &to).map_err(|e| e.to_string())
            }
        }
        "mkdirs" => std::fs::create_dir_all(p).map_err(|e| e.to_string()),
        "mkdir" => {
            let to = p.join(if arg.is_empty() { "New Folder" } else { arg });
            if to.exists() { Err(format!("{} is already there.", to.display())) } else { std::fs::create_dir(&to).map_err(|e| e.to_string()) }
        }
        "touch" => {
            let to = p.join(if arg.is_empty() { "New File.txt" } else { arg });
            if to.exists() { Err(format!("{} is already there.", to.display())) } else { std::fs::write(&to, b"").map_err(|e| e.to_string()) }
        }
        // (With an app picked from apps_for: its .desktop file's Exec line.)
        "open_with" => open_with(arg, path),
        // (That app, from now on, for every file of this one's type.)
        "set_default" => {
            let mime = mime_of(path);
            if mime.is_empty() {
                Err("Couldn't tell what kind of file it is.".into())
            } else {
                std::process::Command::new("xdg-mime").args(["default", arg, &mime]).status().map_err(|e| e.to_string()).and_then(|s| if s.success() { Ok(()) } else { Err("xdg-mime couldn't set it.".into()) })
            }
        }
        // (For good, not the trash: the menu asks first.)
        "delete" => if p.is_dir() && !p.is_symlink() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) }.map_err(|e| e.to_string()),
        // (Made a program, or not: x for whoever can read it.)
        "chmod_x" => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::metadata(p).map_err(|e| e.to_string()).and_then(|m| {
                    let mode = m.permissions().mode();
                    let new = if mode & 0o111 != 0 { mode & !0o111 } else { mode | ((mode & 0o444) >> 2) };
                    std::fs::set_permissions(p, std::fs::Permissions::from_mode(new)).map_err(|e| e.to_string())
                })
            }
            #[cfg(not(unix))]
            Err("Not on this system.".into())
        }
        // (A .tar.gz or .zip beside it; an archive unpacked into a folder beside it.)
        "compress" => {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let zip = arg == "zip";
            let out = unused_name(parent, &format!("{name}.{}", if zip { "zip" } else { "tar.gz" }));
            let out_s = out.to_string_lossy().to_string();
            let mut cmd = if zip {
                let mut c = std::process::Command::new(if on_path("zip") { "zip" } else { "nix" });
                if !on_path("zip") {
                    c.args(["run", "nixpkgs#zip", "--"]);
                }
                c.args(["-qr", &out_s, &name]);
                c
            } else {
                let mut c = std::process::Command::new("tar");
                c.args(["-czf", &out_s, "--", &name]);
                c
            };
            run_in(&mut cmd, parent)
        }
        "extract" => {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let stem = [".tar.gz", ".tar.xz", ".tar.bz2", ".tar.zst", ".tgz", ".tar", ".zip", ".7z", ".rar"]
                .iter()
                .find_map(|e| name.strip_suffix(e))
                .unwrap_or(&name)
                .to_string();
            let to = unused_name(parent, &stem);
            std::fs::create_dir_all(&to).map_err(|e| e.to_string()).and_then(|_| {
                let to_s = to.to_string_lossy().to_string();
                let lower = name.to_lowercase();
                let mut cmd = if lower.ends_with(".zip") {
                    let mut c = std::process::Command::new(if on_path("unzip") { "unzip" } else { "nix" });
                    if !on_path("unzip") {
                        c.args(["run", "nixpkgs#unzip", "--"]);
                    }
                    c.args(["-q", path, "-d", &to_s]);
                    c
                } else if lower.ends_with(".7z") || lower.ends_with(".rar") {
                    let mut c = std::process::Command::new(if on_path("7z") { "7z" } else { "nix" });
                    if !on_path("7z") {
                        c.args(["run", "nixpkgs#p7zip", "--"]);
                    }
                    c.args(["x", "-y", &format!("-o{to_s}"), path]);
                    c
                } else {
                    let mut c = std::process::Command::new("tar");
                    c.args(["-xf", path, "-C", &to_s]);
                    c
                };
                run_in(&mut cmd, parent)?;
                // (One thing inside, a folder of its own most often: it goes
                // beside the archive itself, not a folder deeper.)
                let inside: Vec<PathBuf> = std::fs::read_dir(&to).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
                if let [one] = inside.as_slice() {
                    let name = one.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let dest = unused_name(parent, &name);
                    let dest = if dest == to { parent.join(format!("{name}.extracted")) } else { dest };
                    if std::fs::rename(one, &dest).is_ok() {
                        let _ = std::fs::remove_dir(&to);
                    }
                }
                Ok(())
            })
        }
        "duplicate" => {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            copy_tree(p, &unused_name(parent, &name)).map_err(|e| e.to_string())
        }
        "copy" | "move" => {
            let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let dest = Path::new(arg);
            if !dest.is_dir() {
                Err(format!("{arg} isn't a folder."))
            } else if dest.starts_with(p) {
                Err("A folder can't go inside itself.".into())
            } else {
                let to = unused_name(dest, &name);
                if op == "copy" {
                    copy_tree(p, &to).map_err(|e| e.to_string())
                } else {
                    std::fs::rename(p, &to).or_else(|_| copy_tree(p, &to).and_then(|_| if p.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) })).map_err(|e| e.to_string())
                }
            }
        }
        _ => Err(format!("Unknown action {op}")),
    };
    r.err().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes() {
        assert_eq!(mode_string(0o40755, true, false), "drwxr-xr-x");
        assert_eq!(mode_string(0o100644, false, false), "-rw-r--r--");
        assert_eq!(mode_string(0o104755, false, false), "-rwsr-xr-x");
        assert_eq!(mode_string(0o41777, true, false), "drwxrwxrwt");
    }

    #[test]
    fn actions() {
        let d = std::env::temp_dir().join(format!("tessera-files-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        let ds = d.to_string_lossy().to_string();
        assert_eq!(action("mkdir", &ds, "sub"), "");
        assert_eq!(action("touch", &ds, "a.txt"), "");
        assert_eq!(action("duplicate", &format!("{ds}/a.txt"), ""), "");
        assert!(d.join("a (2).txt").exists());
        assert_eq!(action("rename", &format!("{ds}/a (2).txt"), "b.txt"), "");
        assert_eq!(action("move", &format!("{ds}/b.txt"), &format!("{ds}/sub")), "");
        assert!(d.join("sub/b.txt").exists());
        assert!(!action("rename", &format!("{ds}/a.txt"), "sub").is_empty());
        let l: Vec<serde_json::Value> = serde_json::from_str(&list(&ds, false)).unwrap();
        assert_eq!(l.len(), 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn streams_lines() {
        let d = std::env::temp_dir().join(format!("tessera-lines-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("big.txt");
        let body: String = (1..=20000).map(|i| format!("line {i}\n")).collect();
        std::fs::write(&f, body).unwrap();
        let p = f.to_string_lossy().to_string();
        let mut v: serde_json::Value = serde_json::from_str(&super::line_index(&p)).unwrap();
        while !v["done"].as_bool().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            v = serde_json::from_str(&super::line_index(&p)).unwrap();
        }
        assert_eq!(v["lines"], 20000);
        assert_eq!(super::read_lines(&p, 0, 2), "line 1\nline 2");
        assert_eq!(super::read_lines(&p, 19998, 5), "line 19999\nline 20000");
        let crlf = d.join("crlf.txt");
        std::fs::write(&crlf, "a\r\nb\r\n").unwrap();
        let c = crlf.to_string_lossy().to_string();
        let mut v: serde_json::Value = serde_json::from_str(&super::line_index(&c)).unwrap();
        while !v["done"].as_bool().unwrap() {
            std::thread::sleep(std::time::Duration::from_millis(10));
            v = serde_json::from_str(&super::line_index(&c)).unwrap();
        }
        assert_eq!(super::read_lines(&c, 0, 2), "a\nb");
        let _ = std::fs::remove_dir_all(d);
    }


    #[test]
    fn edits_keep_line_ends() {
        let d = std::env::temp_dir().join(format!("tessera-edit-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("w.txt");
        std::fs::write(&f, "a\r\nb\r\n").unwrap();
        let p = f.to_string_lossy().to_string();
        let v: serde_json::Value = serde_json::from_str(&super::read_for_edit(&p)).unwrap();
        assert_eq!(v["text"], "a\nb\n");
        assert_eq!(v["crlf"], true);
        assert_eq!(super::save_edited(&p, "a\nc\n", true), "");
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "a\r\nc\r\n");
        let _ = std::fs::remove_dir_all(d);
    }

}

/// A text file: {"text"} (the first 3 MB of a big one, "truncated" said so:
/// the viewer streams the rest) or {"error"}.
pub fn read_text(path: &str) -> String {
        // (The first 3 MB of a big one, said so: the viewer streams the rest.)
        use std::io::Read;
        const MAX: u64 = 3_000_000;
        let v = match std::fs::File::open(path) {
            Ok(f) => {
                let mut buf = Vec::new();
                let _ = f.take(MAX + 1).read_to_end(&mut buf);
                let cut = buf.len() as u64 > MAX;
                buf.truncate(MAX as usize);
                serde_json::json!({"text": String::from_utf8_lossy(&buf), "truncated": cut})
            }
            Err(e) => serde_json::json!({"error": e.to_string()}),
        };
    v.to_string()
}

/// Write a text file atomically: "" or what went wrong. (Tessera's own
/// files make their folder; a user's file needs it there.)
pub fn write_text(path: &str, text: &str) -> String {
    let p = std::path::PathBuf::from(path);
    if p.starts_with(crate::settings::state_dir()) {
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
    }
    let mut tmp = p.clone().into_os_string();
    tmp.push(".tmp");
    std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, &p)).err().map(|e| e.to_string()).unwrap_or_default()
}
