//! Every program in nixpkgs, for the editor's command completion: a
//! command you don't have comes up with the package that has it, to run as
//! `nix run nixpkgs#<pkg> --` (or `nix shell nixpkgs#<pkg> -c <cmd>` when
//! it isn't the package's own program).
//!
//! Where it comes from: nix-index's database, the prebuilt one from
//! nix-community/nix-index-database (weekly, on GitHub), fetched into
//! ~/.cache/nix-index/files when it's missing or over a week old (where
//! nix-locate looks too, so it's fresh for that as well). From it, with
//! nix-locate, an index of command -> packages (~/.cache/tessera/
//! nix-commands.tsv), made again whenever the database changes. Both in
//! the background; until they're there, nothing is offered.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

const RELEASE: &str = "https://github.com/nix-community/nix-index-database/releases/latest/download/index-x86_64-linux";

fn db_dir() -> PathBuf {
    std::env::var_os("NIX_INDEX_DATABASE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/nix-index"))
}

fn index_file() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/tessera/nix-commands.tsv")
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
}

fn age(p: &std::path::Path) -> Option<Duration> {
    std::fs::metadata(p).ok()?.modified().ok().and_then(|m| SystemTime::now().duration_since(m).ok())
}

static WORKING: AtomicBool = AtomicBool::new(false);

/// Make sure the database and the index are there and fresh (in the background).
pub fn ensure() {
    // (Looked at once an hour at most.)
    static LAST: Mutex<Option<std::time::Instant>> = Mutex::new(None);
    {
        let mut l = LAST.lock().unwrap();
        if l.is_some_and(|t| t.elapsed() < Duration::from_secs(3600)) {
            return;
        }
        *l = Some(std::time::Instant::now());
    }
    if !cfg!(target_os = "linux") || !cfg!(target_arch = "x86_64") || WORKING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        let db = db_dir().join("files");
        // The database: fetched when missing or over a week old.
        if age(&db).is_none_or(|a| a > Duration::from_secs(7 * 86400)) {
            let _ = std::fs::create_dir_all(db_dir());
            let tmp = db_dir().join("files.tessera-download");
            let ok = std::process::Command::new("curl")
                .args(["-sSfL", "--retry", "2", "-o"])
                .arg(&tmp)
                .arg(RELEASE)
                .status()
                .is_ok_and(|s| s.success());
            if ok && std::fs::metadata(&tmp).is_ok_and(|m| m.len() > 1_000_000) {
                let _ = std::fs::rename(&tmp, &db);
            } else {
                let _ = std::fs::remove_file(&tmp);
            }
        }
        // The index: made again when the database is newer.
        let idx = index_file();
        let stale = match (std::fs::metadata(&db).and_then(|m| m.modified()), std::fs::metadata(&idx).and_then(|m| m.modified())) {
            (Ok(d), Ok(i)) => d > i,
            (Ok(_), Err(_)) => true,
            _ => false,
        };
        if stale {
            build_index(&idx);
        }
        WORKING.store(false, Ordering::SeqCst);
    });
}

/// nix-locate's executables in bin/: "attr.out size x /nix/store/...-name/bin/cmd".
fn build_index(idx: &std::path::Path) {
    let mut cmd = if on_path("nix-locate") {
        std::process::Command::new("nix-locate")
    } else {
        let mut c = std::process::Command::new("nix");
        c.args(["shell", "--quiet", "nixpkgs#nix-index", "-c", "nix-locate"]);
        c
    };
    // (nix-locate's regex takes no $: deeper paths are left out below.)
    cmd.args(["--at-root", "--type", "x", "--type", "s", "--regex", "/bin/[^/]+"]).env("NIX_INDEX_DATABASE", db_dir());
    let Ok(out) = cmd.stderr(std::process::Stdio::null()).output() else { return };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut by_cmd: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for line in text.lines() {
        let mut f = line.split_whitespace();
        let (Some(attr), Some(path)) = (f.next(), f.last()) else { continue };
        let attr = attr.strip_suffix(".out").or_else(|| attr.strip_suffix(".bin")).unwrap_or(attr);
        // (Only <store path>/bin/<name>; wrapped programs' .foo-wrapped aren't commands.)
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() != 6 || parts[4] != "bin" {
            continue;
        }
        let name = parts[5];
        if name.starts_with('.') || name.is_empty() {
            continue;
        }
        let v = by_cmd.entry(name.to_string()).or_default();
        if !v.iter().any(|a| a == attr) {
            v.push(attr.to_string());
        }
    }
    let mut lines = String::new();
    for (name, mut attrs) in by_cmd {
        // The likeliest first: named for the command, then plain names, then short ones.
        attrs.sort_by_key(|a| (a != &name, !a.starts_with(name.as_str()), a.contains('.'), a.len()));
        attrs.truncate(8);
        lines.push_str(&format!("{name}\t{}\n", attrs.join(" ")));
    }
    if lines.is_empty() {
        return;
    }
    let _ = std::fs::create_dir_all(idx.parent().unwrap());
    let tmp = idx.with_extension("tmp");
    if std::fs::write(&tmp, lines).is_ok() {
        let _ = std::fs::rename(&tmp, idx);
    }
}

/// The index, read once (and again when it's remade): (command, packages), sorted.
fn index() -> Vec<(String, Vec<String>)> {
    static CACHE: OnceLock<Mutex<(Option<SystemTime>, std::sync::Arc<Vec<(String, Vec<String>)>>)>> = OnceLock::new();
    let cell = CACHE.get_or_init(|| Mutex::new((None, Default::default())));
    let modified = std::fs::metadata(index_file()).and_then(|m| m.modified()).ok();
    let mut c = cell.lock().unwrap();
    if c.0 != modified {
        let text = std::fs::read_to_string(index_file()).unwrap_or_default();
        let v: Vec<(String, Vec<String>)> = text
            .lines()
            .filter_map(|l| {
                let (n, a) = l.split_once('\t')?;
                Some((n.to_string(), a.split(' ').map(String::from).collect()))
            })
            .collect();
        *c = (modified, std::sync::Arc::new(v));
    }
    (*c.1).clone()
}

/// How to run `cmd` from `attr`.
pub fn run_line(cmd: &str, attr: &str) -> String {
    let last = attr.rsplit('.').next().unwrap_or(attr);
    if last == cmd {
        format!("nix run nixpkgs#{attr} --")
    } else {
        format!("nix shell nixpkgs#{attr} -c {cmd}")
    }
}

/// Commands starting with `prefix` that nixpkgs has: (command, package),
/// one per package (a command in several packages comes once for each).
pub fn lookup(prefix: &str, limit: usize) -> Vec<(String, String)> {
    ensure();
    if prefix.len() < 2 {
        return Vec::new();
    }
    let idx = index();
    let at = idx.partition_point(|(n, _)| n.as_str() < prefix);
    let mut out = Vec::new();
    for (n, attrs) in idx[at..].iter().take_while(|(n, _)| n.starts_with(prefix)) {
        for a in attrs.iter().take(if n == prefix { 8 } else { 3 }) {
            out.push((n.clone(), a.clone()));
        }
        if out.len() >= limit {
            break;
        }
    }
    // (The exact name first.)
    out.sort_by_key(|(n, _)| (n != prefix, n.len()));
    out.truncate(limit);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn run_lines() {
        assert_eq!(super::run_line("cowsay", "cowsay"), "nix run nixpkgs#cowsay --");
        assert_eq!(super::run_line("rg", "ripgrep"), "nix shell nixpkgs#ripgrep -c rg");
        assert_eq!(super::run_line("python3", "python3Packages.foo"), "nix shell nixpkgs#python3Packages.foo -c python3");
    }

    /// Fetch and index for real (slow; downloads ~110 MB the first time).
    #[test]
    #[ignore]
    fn builds_and_finds() {
        super::ensure();
        while super::WORKING.load(std::sync::atomic::Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        let r = super::lookup("cowsa", 10);
        eprintln!("{r:?}");
        assert!(r.iter().any(|(n, a)| n == "cowsay" && a.contains("cowsay")));
        let r = super::lookup("rg", 10);
        eprintln!("{r:?}");
    }
}
