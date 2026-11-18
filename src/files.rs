//! The notebook's file browser (qml/FileBrowser.qml, an `ls` card): a
//! folder's entries with what `ls -l` shows, and the actions its menus run
//! on them (open, rename, trash, new folder, ...), done here rather than as
//! commands typed into the shell.

use serde_json::json;
use std::collections::HashMap;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
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
            let mode = md.permissions().mode();
            let exec = !is_dir && mode & 0o111 != 0;
            let items = if is_dir { std::fs::read_dir(&p).map(|r| r.count() as i64).unwrap_or(-1) } else { -1 };
            let target = if link { std::fs::read_link(&p).map(|t| t.to_string_lossy().to_string()).unwrap_or_default() } else { String::new() };
            let mtime = md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
            out.push(json!({
                "name": name, "path": p.to_string_lossy(), "dir": is_dir, "link": link, "target": target, "exec": exec,
                "hidden": hidden, "size": md.len(), "items": items, "mtime": mtime,
                "mode": mode_string(lmd.map(|m| m.mode()).unwrap_or(mode), is_dir, link),
                "user": names("/etc/passwd").get(&md.uid()).cloned().unwrap_or_else(|| md.uid().to_string()),
                "group": names("/etc/group").get(&md.gid()).cloned().unwrap_or_else(|| md.gid().to_string()),
                "kind": kind_of(&name, is_dir, exec),
            }));
        }
    }
    serde_json::Value::Array(out).to_string()
}

fn unused_name(dir: &Path, base: &str) -> PathBuf {
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
        "reveal" => {
            let uri = format!("file://{path}");
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
}
