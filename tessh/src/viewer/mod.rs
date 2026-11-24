//! tessh's file viewer: most files, full screen on the terminal's other
//! screen, in the guide's pager (editor/pager.rs: search as you type, the
//! mouse, picking and copying, all of it from the keyboard too). Code and
//! text highlighted (Tessera's grammars, then bat's ~200 syntaxes as Grove
//! has them), Markdown rendered, JSON pretty-printed, YAML and TOML checked
//! (and reformatted on Tab), CSV and TSV as a table, pictures as kitty,
//! sixel or iTerm2 graphics (half blocks otherwise), a PDF's text, an
//! archive's list, anything else as a hex dump; a folder goes to ls.
//!
//! Three ways in: `view file…`; a file that isn't a program typed as a
//! command (run_file: what it can show it shows, what it can't, media and
//! office documents, goes to the desktop's app for it, xdg-open or open);
//! and `cat file…` at a terminal (cat_tty: colored, pictures drawn,
//! binaries not dumped on the screen; piped or with options, plain cat).

pub mod highlight;
pub mod image;
mod render;

use std::io::Read;
use std::path::{Path, PathBuf};

/// What a file is, for the viewer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Dir,
    Image,
    Pdf,
    /// An archive: its list.
    Archive,
    /// One compressed file (x.log.gz): shown as what's inside.
    Compressed,
    Markdown,
    Json,
    Yaml,
    Toml,
    /// CSV or TSV (the delimiter).
    Table(u8),
    Text,
    /// Media, office documents, fonts…: an app of their own shows them.
    Outside,
    Binary,
}

/// The first bytes of a file (enough to tell what it is).
fn head(p: &Path) -> Vec<u8> {
    let mut buf = vec![0u8; 8192];
    let Ok(mut f) = std::fs::File::open(p) else { return vec![] };
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..]) {
            Ok(0) | Err(_) => break,
            Ok(k) => n += k,
        }
    }
    buf.truncate(n);
    buf
}

/// The name's extension, lowercase ("" for none).
fn ext_of(p: &Path) -> String {
    p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default()
}

/// Whether bytes read as text: no NUL (but in UTF-16), and little that
/// isn't UTF-8 (Tessera's rule: under one in twenty).
pub fn is_text(b: &[u8]) -> bool {
    if b.starts_with(&[0xff, 0xfe]) || b.starts_with(&[0xfe, 0xff]) {
        return true;
    }
    if b.contains(&0) {
        return false;
    }
    // (The last few bytes may be a char cut in half.)
    let s = String::from_utf8_lossy(&b[..b.len().saturating_sub(4).max(b.len().min(4))]);
    let bad = s.chars().filter(|&c| c == '\u{fffd}' || (c.is_control() && !matches!(c, '\n' | '\r' | '\t' | '\x0c' | '\x1b' | '\x08'))).count();
    bad * 20 <= s.chars().count().max(1)
}

const OUTSIDE: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "wav", "m4a", "aac", "wma", "aiff", "mid", "midi", "mp4", "m4v", "mkv", "webm", "avi", "mov", "wmv", "flv", "mpg",
    "mpeg", "3gp", "ts", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "odg", "rtf", "epub", "mobi", "djvu", "ttf", "otf", "woff",
    "woff2", "tif", "tiff", "heic", "heif", "avif", "ico", "psd", "xcf", "kra", "blend", "dmg", "exe", "msi", "sqlite", "db",
];

/// What a file is: by its first bytes, then its name, then whether it
/// reads as text (and what that text looks like, as Grove guesses).
pub fn classify(p: &Path) -> Kind {
    if p.is_dir() {
        return Kind::Dir;
    }
    let b = head(p);
    let ext = ext_of(p);
    let name = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    let tar_name = name.contains(".tar.") || matches!(ext.as_str(), "tgz" | "tbz" | "tbz2" | "txz" | "tzst" | "tar");
    if b.starts_with(b"\x89PNG") || b.starts_with(&[0xff, 0xd8, 0xff]) || b.starts_with(b"GIF8") || (b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP")) || (b.starts_with(b"BM") && ext == "bmp") {
        return Kind::Image;
    }
    if b.starts_with(b"%PDF") {
        return Kind::Pdf;
    }
    // (Zip: an office document or an ebook is its app's; a zip, jar or wheel is a list.)
    if b.starts_with(b"PK\x03\x04") || b.starts_with(b"PK\x05\x06") {
        return if OUTSIDE.contains(&ext.as_str()) { Kind::Outside } else { Kind::Archive };
    }
    let compressed = b.starts_with(&[0x1f, 0x8b]) || b.starts_with(b"BZh") || b.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0]) || b.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]);
    if compressed {
        return if tar_name { Kind::Archive } else { Kind::Compressed };
    }
    if b.starts_with(&[b'7', b'z', 0xbc, 0xaf, 0x27, 0x1c]) || b.starts_with(b"Rar!") || b.starts_with(b"MSCF") || b.starts_with(b"!<arch>") || b.get(257..262) == Some(b"ustar") || (b.len() < 512 && ext == "tar") {
        return Kind::Archive;
    }
    if matches!(ext.as_str(), "iso" | "7z" | "rar" | "cab" | "deb" | "rpm" | "cpio" | "xar" | "lha" | "lzh" | "arj") {
        return Kind::Archive;
    }
    if OUTSIDE.contains(&ext.as_str()) && !is_text(&b) {
        return Kind::Outside;
    }
    if !is_text(&b) {
        return Kind::Binary;
    }
    match ext.as_str() {
        "md" | "markdown" | "mdown" | "mkd" => return Kind::Markdown,
        "json" | "ipynb" | "geojson" | "jsonc" => return Kind::Json,
        "yaml" | "yml" => return Kind::Yaml,
        "toml" => return Kind::Toml,
        "csv" => return Kind::Table(render::delimiter(&name, &b)),
        "tsv" | "tab" => return Kind::Table(b'\t'),
        _ => {}
    }
    match name.as_str() {
        "flake.lock" | "package-lock.json" | "composer.lock" => return Kind::Json,
        "cargo.lock" | "uv.lock" | "poetry.lock" | "pyproject.toml" => return Kind::Toml,
        _ => {}
    }
    // (No name that says: the content, by Grove's clear-cut rules; a .txt stays text.)
    if ext.is_empty() || matches!(ext.as_str(), "lock" | "dat" | "out" | "data") {
        let t = String::from_utf8_lossy(&b);
        match highlight::guess_ext(&t) {
            Some("json") => return Kind::Json,
            Some("md") => return Kind::Markdown,
            Some("csv") => return Kind::Table(b','),
            Some("tsv") => return Kind::Table(b'\t'),
            _ => {}
        }
    }
    Kind::Text
}

/// A size as people say it: 512 B, 4.2 KB, 13 MB.
pub fn human_size(n: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u + 1 < units.len() {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} B")
    } else if v < 10.0 {
        format!("{v:.1} {}", units[u])
    } else {
        format!("{v:.0} {}", units[u])
    }
}

/// Whether there's a desktop to open things on (macOS always; elsewhere a
/// Wayland or X display).
pub fn has_desktop() -> bool {
    cfg!(target_os = "macos") || ["WAYLAND_DISPLAY", "DISPLAY"].iter().any(|v| std::env::var_os(v).is_some_and(|x| !x.is_empty()))
}

/// Open a file in the desktop's app for it (xdg-open, or open on macOS),
/// detached: it lives on after tessh, and says nothing on this terminal.
pub fn open_outside(p: &Path) {
    use std::os::unix::process::CommandExt;
    let prog = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
    let mut c = std::process::Command::new(prog);
    c.arg(p).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    unsafe {
        c.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let _ = c.spawn();
}

/// A folder: listed by ls (tessh's when it has one, else the system's),
/// on this terminal.
fn list_dir(p: &Path) -> i32 {
    match std::process::Command::new("ls").arg("-la").arg("--color=auto").arg(p).status() {
        Ok(s) => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!("view: ls: {e}");
            1
        }
    }
}

/// Show one file full screen until q (a folder: listed). `hex`: as bytes,
/// whatever it is. Its status: 0, or 1 when it couldn't be read.
pub fn view(p: &Path, hex: bool) -> i32 {
    if !p.exists() {
        eprintln!("view: {}: no such file or folder", p.display());
        return 1;
    }
    let kind = if hex { Kind::Binary } else { classify(p) };
    // (Shown full screen: a Helix grammar it lacks may be fetched, once.)
    highlight::FETCH.store(true, std::sync::atomic::Ordering::Relaxed);
    if kind == Kind::Dir {
        return list_dir(p);
    }
    if kind == Kind::Image {
        match image::show(p) {
            Ok(()) => return 0,
            // (A picture `image` can't read: its bytes.)
            Err(_) => return show(render::render(p, Kind::Binary)),
        }
    }
    show(render::render(p, kind))
}

fn show(g: Result<crate::editor::pager::Guide, String>) -> i32 {
    match g {
        Ok(g) => {
            crate::editor::pager::run(g, "");
            0
        }
        Err(e) => {
            eprintln!("view: {e}");
            1
        }
    }
}

/// A file typed as a command, or clicked in a listing: what the viewer
/// shows well it shows; media, office documents and the like go to the
/// desktop's app for them (on a desktop; without one, the viewer shows what
/// it can); a folder is listed.
pub fn open_path(p: &Path) -> i32 {
    match classify(p) {
        Kind::Outside if has_desktop() => {
            open_outside(p);
            0
        }
        _ => view(p, false),
    }
}

/// Whether `word`, typed as a command, names a file to show rather than a
/// program: a regular file without execute permission, written as a path
/// (with a /), or a name in this folder that no program on PATH has.
pub fn file_command(word: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let p = PathBuf::from(word);
    let meta = std::fs::metadata(&p).ok()?;
    if !meta.is_file() || meta.permissions().mode() & 0o111 != 0 {
        return None;
    }
    if !word.contains('/') && on_path(word) {
        return None;
    }
    Some(p)
}

fn on_path(name: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::env::var_os("PATH").is_some_and(|path| std::env::split_paths(&path).any(|d| std::fs::metadata(d.join(name)).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)))
}

/// The `view` builtin: view [-x|--hex] file… Each file in turn (q shows the
/// next); a folder is listed.
pub fn view_builtin(words: &[String]) -> i32 {
    let mut hex = false;
    let mut files = vec![];
    let mut opts = true;
    for w in words {
        match w.as_str() {
            "--" if opts => opts = false,
            "-x" | "--hex" if opts => hex = true,
            "-h" | "--help" if opts => {
                println!("{}", USAGE);
                return 0;
            }
            s if opts && s.starts_with('-') && s.len() > 1 => {
                eprintln!("view: unknown option {s}\n{USAGE}");
                return 2;
            }
            _ => files.push(PathBuf::from(w)),
        }
    }
    if files.is_empty() {
        eprintln!("{USAGE}");
        return 2;
    }
    if unsafe { libc::isatty(1) } != 1 {
        // (Not a terminal: what cat would give, so view works in a pipe too.)
        let mut status = 0;
        for f in &files {
            match std::fs::read(f) {
                Ok(b) => {
                    use std::io::Write;
                    let _ = std::io::stdout().write_all(&b);
                }
                Err(e) => {
                    eprintln!("view: {}: {e}", f.display());
                    status = 1;
                }
            }
        }
        return status;
    }
    let mut status = 0;
    for f in &files {
        status = status.max(view(f, hex));
    }
    status
}

pub const USAGE: &str = "view [-x|--hex] file…\n\nShow files full screen: code and text highlighted, Markdown rendered, JSON pretty-printed, YAML and TOML checked (Tab: reformatted), CSV and TSV as a table, pictures (kitty, sixel or iTerm2 graphics, else blocks), a PDF's text, an archive's list, anything else as a hex dump (-x: any file's bytes); a folder is listed. / searches, drag or Shift+arrows pick, y copies, Tab switches views, o opens the file in its own app, q goes back. Typing a file that isn't a program at the prompt shows it the same way.";

/// `cat file…` at a terminal, no options: each file as it is, colored;
/// a picture drawn; a binary file not dumped on the screen (a note says
/// view shows it). None: not for this (piped, options, not all files, or a
/// file with its own escapes): plain cat runs.
pub fn cat_tty(words: &[String]) -> Option<i32> {
    if words.first().map(String::as_str) != Some("cat") || words.len() < 2 {
        return None;
    }
    if unsafe { libc::isatty(1) } != 1 {
        return None;
    }
    let files: Vec<PathBuf> = words[1..].iter().map(PathBuf::from).collect();
    if words[1..].iter().any(|w| w.starts_with('-')) || !files.iter().all(|f| f.is_file()) {
        return None;
    }
    let kinds: Vec<Kind> = files.iter().map(|f| classify(f)).collect();
    // (A text with escapes of its own, colored logs: cat's, so they show as meant.)
    for (f, k) in files.iter().zip(&kinds) {
        if matches!(k, Kind::Text | Kind::Markdown | Kind::Json | Kind::Yaml | Kind::Toml | Kind::Table(_)) && head(f).contains(&0x1b) {
            return None;
        }
    }
    use std::io::Write;
    let mut status = 0;
    for (f, k) in files.iter().zip(kinds) {
        match k {
            Kind::Image => {
                if let Err(e) = image::print_inline(f) {
                    eprintln!("cat: {e}");
                    status = 1;
                }
            }
            Kind::Text | Kind::Markdown | Kind::Json | Kind::Yaml | Kind::Toml | Kind::Table(_) => match render::read_text(f) {
                Ok(t) => {
                    let mut o = std::io::stdout().lock();
                    let big = t.len() > render::HIGHLIGHT_MAX;
                    let clean = render::sanitize(&t);
                    let lines = if big { clean.split('\n').map(String::from).collect() } else { highlight::highlight_file(f, &clean).0 };
                    let n = lines.len();
                    for (i, l) in lines.into_iter().enumerate() {
                        // (The last "line" after a final newline is nothing.)
                        if i + 1 == n && l.is_empty() {
                            break;
                        }
                        let _ = writeln!(o, "{l}");
                    }
                    let _ = o.flush();
                }
                Err(e) => {
                    eprintln!("cat: {e}");
                    status = 1;
                }
            },
            _ => {
                let size = std::fs::metadata(f).map(|m| human_size(m.len())).unwrap_or_default();
                eprintln!("\x1b[2mcat: {} is binary ({size}): not shown on the terminal. `view {}` shows it; piped, cat passes it on.\x1b[0m", f.display(), f.display());
            }
        }
    }
    Some(status)
}

/// A command at the terminal that's for the viewer: a file that isn't a
/// program typed alone (interactive only), or cat at a terminal. Its
/// status, or None to run it as usual.
pub fn intercept(words: &[String], interactive: bool) -> Option<i32> {
    if interactive && words.len() == 1 {
        if let Some(p) = file_command(&words[0]) {
            return Some(open_path(&p));
        }
    }
    cat_tty(words)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str, bytes: &[u8]) -> PathBuf {
        let d = std::env::temp_dir().join(format!("tessh-viewer-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let p = d.join(name);
        std::fs::write(&p, bytes).unwrap();
        p
    }

    #[test]
    fn kinds_by_content_and_name() {
        assert_eq!(classify(&tmp("a.md", b"# Hi\n")), Kind::Markdown);
        assert_eq!(classify(&tmp("a.json", b"{}")), Kind::Json);
        assert_eq!(classify(&tmp("a.csv", b"a,b\n1,2\n")), Kind::Table(b','));
        assert_eq!(classify(&tmp("a.tsv", b"a\tb\n")), Kind::Table(b'\t'));
        assert_eq!(classify(&tmp("a.rs", b"fn main() {}\n")), Kind::Text);
        assert_eq!(classify(&tmp("a.png", b"\x89PNG\r\n\x1a\n....")), Kind::Image);
        assert_eq!(classify(&tmp("a.pdf", b"%PDF-1.4\n")), Kind::Pdf);
        assert_eq!(classify(&tmp("a.zip", b"PK\x05\x06\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0")), Kind::Archive);
        assert_eq!(classify(&tmp("a.docx", b"PK\x03\x04rest")), Kind::Outside);
        assert_eq!(classify(&tmp("a.log.gz", &[0x1f, 0x8b, 8, 0])), Kind::Compressed);
        assert_eq!(classify(&tmp("a.bin", &[0, 1, 2, 3, 255])), Kind::Binary);
        assert_eq!(classify(&tmp("noext", b"{\"a\": 1}")), Kind::Json);
        assert_eq!(classify(&std::env::temp_dir()), Kind::Dir);
    }

    #[test]
    fn a_file_typed_is_shown_only_when_it_isnt_a_program() {
        use std::os::unix::fs::PermissionsExt;
        let p = tmp("notes.txt", b"hi\n");
        let s = p.to_string_lossy().to_string();
        assert_eq!(file_command(&s), Some(p.clone()));
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(file_command(&s), None, "a program runs");
        assert_eq!(file_command("/no/such/file"), None);
        assert_eq!(file_command(&std::env::temp_dir().to_string_lossy()), None, "a folder isn't a file");
    }

    #[test]
    fn sizes_read_as_people_say_them() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(4300), "4.2 KB");
        assert_eq!(human_size(13 * 1024 * 1024), "13 MB");
    }
}
