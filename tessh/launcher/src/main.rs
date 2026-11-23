//! The static `tessh`: one file that runs on any Linux of its architecture,
//! NixOS (no system program loader) and bare containers included.
//!
//! It carries, appended to itself, tessh built for musl against Python's
//! stable ABI, musl's program loader, libgcc_s, and uv (static). On the first run they're
//! unpacked to `~/.cache/tessh/rt-<hash>/`, and tessh's loader path is set
//! to that folder's loader. The Python comes from uv: its musl build of
//! CPython (python-build-standalone), installed once with `uv python
//! install`, which version picked by `--python X.Y`, $TESSH_PYTHON,
//! ~/.config/tessh/python, or else the newest uv offers (3.12 or later;
//! looked at again once a day, so a new CPython in uv is taken up). Each
//! version gets `rt-<hash>/py/<name>/`: tessh hard-linked into bin (with
//! `python3` links to it), libpython and the standard library linked into
//! lib. A real loader runs it, so C extensions (musllinux wheels: numpy,
//! pandas) load as in any Python.
//!
//! Two builds of tessh come along: for musl (above: runs anywhere) and for
//! glibc. Where the system has glibc's loader (any usual distribution;
//! NixOS with nix-ld), the glibc one runs, with uv's glibc CPython: the
//! Python and wheels the system's own uv uses. It's tried once (tessh
//! --version) and, if it doesn't start, musl from then on (the choice is
//! kept in rt-<hash>/libc). TESSH_LIBC=gnu or =musl picks one.
//!
//! The file is this program, then the payload (a zstd-compressed tar), then
//! a 64-byte trailer: `TESSH-PAYLOAD <offset, 20 digits> <hash>`.

use std::ffi::OsString;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const TRAILER: u64 = 64;
const PLACEHOLDER: &[u8] = b"/TESSH-INTERP-";
const MIN_MINOR: u32 = 12;

fn die(msg: &str) -> ! {
    eprintln!("tessh: {msg}");
    std::process::exit(127);
}

fn trailer(exe: &Path) -> std::io::Result<(u64, u64, String)> {
    let mut f = fs::File::open(exe)?;
    let size = f.metadata()?.len();
    let bad = || std::io::Error::other("no payload (this tessh launcher was built without one)");
    if size < TRAILER {
        return Err(bad());
    }
    f.seek(SeekFrom::End(-(TRAILER as i64)))?;
    let mut t = String::new();
    f.take(TRAILER).read_to_string(&mut t)?;
    let mut w = t.split_whitespace();
    if w.next() != Some("TESSH-PAYLOAD") {
        return Err(bad());
    }
    let off: u64 = w.next().and_then(|o| o.parse().ok()).ok_or_else(bad)?;
    let hash = w.next().filter(|h| h.chars().all(|c| c.is_ascii_hexdigit())).ok_or_else(bad)?.to_string();
    Ok((off, size - TRAILER - off, hash))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|h| !h.is_empty()).map(PathBuf::from)
}

fn cache_base() -> PathBuf {
    if let Some(c) = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).filter(|p| p.is_absolute()) {
        return c.join("tessh");
    }
    if let Some(h) = home() {
        return h.join(".cache/tessh");
    }
    PathBuf::from(format!("/tmp/tessh-{}", unsafe { libc_getuid() }))
}

extern "C" {
    #[link_name = "getuid"]
    fn libc_getuid() -> u32;
}

/// Sets the loader path in tessh (its PT_INTERP, a long placeholder).
fn patch_loader(bin: &Path, loader: &Path) -> std::io::Result<()> {
    let mut data = fs::read(bin)?;
    let new = loader.as_os_str().as_bytes();
    let mut found = false;
    let mut i = 0;
    while let Some(k) = data[i..].windows(PLACEHOLDER.len()).position(|w| w == PLACEHOLDER) {
        let at = i + k;
        let len = data[at..].iter().position(|b| *b == 0).unwrap_or(0);
        if new.len() >= len {
            return Err(std::io::Error::other(format!("the cache folder's path is too long for tessh ({} > {} bytes): set XDG_CACHE_HOME to a shorter one", new.len(), len - 1)));
        }
        data[at..at + new.len()].copy_from_slice(new);
        data[at + new.len()..at + len].fill(0);
        found = true;
        i = at + len;
    }
    if !found {
        return Err(std::io::Error::other("tessh has no loader placeholder"));
    }
    fs::write(bin, data)
}

/// The payload, unpacked once into `rt` (made in a temporary folder and
/// renamed, so two starting at once don't trip over each other).
fn unpack(exe: &Path, off: u64, len: u64, rt: &Path) -> std::io::Result<()> {
    if rt.join(".done").exists() {
        return Ok(());
    }
    let parent = rt.parent().unwrap_or(Path::new("/"));
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(".unpack-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let mut f = fs::File::open(exe)?;
    f.seek(SeekFrom::Start(off))?;
    let dec = ruzstd::decoding::StreamingDecoder::new(f.take(len)).map_err(|e| std::io::Error::other(format!("payload: {e}")))?;
    let mut ar = tar::Archive::new(dec);
    ar.set_preserve_permissions(true);
    ar.unpack(&tmp)?;
    let loader_name = fs::read_dir(tmp.join("lib"))?.flatten().map(|e| e.file_name()).find(|n| n.as_bytes().starts_with(b"ld-musl-")).ok_or_else(|| std::io::Error::other("payload has no loader"))?;
    patch_loader(&tmp.join("bin/tessh"), &rt.join("lib").join(&loader_name))?;
    fs::write(tmp.join(".done"), b"")?;
    if fs::rename(&tmp, rt).is_err() {
        let _ = fs::remove_dir_all(&tmp);
        if !rt.join(".done").exists() {
            return Err(std::io::Error::other(format!("couldn't set up {}", rt.display())));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Libc {
    Gnu,
    Musl,
}

impl Libc {
    fn suffix(self) -> &'static str {
        match self {
            Libc::Gnu => "gnu",
            Libc::Musl => "musl",
        }
    }
    /// This libc's tessh in the unpacked payload.
    fn tessh(self, rt: &Path) -> PathBuf {
        rt.join(match self {
            Libc::Gnu => "bin/tessh-gnu",
            Libc::Musl => "bin/tessh",
        })
    }
}

/// glibc's program loader, where the system has one.
fn gnu_loader() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "/lib/ld-linux-aarch64.so.1",
        _ => "/lib64/ld-linux-x86-64.so.2",
    }
}

/// Which tessh to run: TESSH_LIBC, else the one found to work here before,
/// else glibc's where the system has its loader.
fn pick_libc(rt: &Path) -> (Libc, bool) {
    match std::env::var("TESSH_LIBC").as_deref() {
        Ok("musl") => return (Libc::Musl, true),
        Ok("gnu") | Ok("glibc") => return (Libc::Gnu, true),
        _ => {}
    }
    if !Libc::Gnu.tessh(rt).is_file() {
        return (Libc::Musl, true);
    }
    match fs::read_to_string(rt.join("libc")).as_deref().map(str::trim) {
        Ok("musl") => (Libc::Musl, true),
        Ok("gnu") => (Libc::Gnu, true),
        _ if Path::new(gnu_loader()).exists() => (Libc::Gnu, false),
        _ => (Libc::Musl, true),
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Py {
    name: String,
    dir: PathBuf,
    version: (u32, u32, u32),
    pre: bool,
}

/// `cpython-3.14.6-linux-x86_64-musl` → (3, 14, 6), not a pre-release.
/// (Free-threaded and debug builds aren't stable-ABI ones: left out.)
fn parse_name(name: &str, libc: Libc) -> Option<((u32, u32, u32), bool)> {
    let v = name.strip_prefix("cpython-")?.strip_suffix(&format!("-linux-{}-{}", std::env::consts::ARCH, libc.suffix()))?;
    if v.contains('+') {
        return None;
    }
    let mut parts = v.splitn(3, '.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let p = parts.next()?;
    let digits: String = p.chars().take_while(|c| c.is_ascii_digit()).collect();
    Some(((major, minor, digits.parse().ok()?), digits.len() < p.len()))
}

/// What was asked for (3, 3.14, 3.14.6), as numbers.
fn parse_want(w: &str) -> Option<Vec<u32>> {
    let v: Option<Vec<u32>> = w.trim().split('.').map(|x| x.parse().ok()).collect();
    v.filter(|v| !v.is_empty() && v.len() <= 3)
}

fn matches(want: &[u32], v: (u32, u32, u32)) -> bool {
    let have = [v.0, v.1, v.2];
    v.0 == 3 && v.1 >= MIN_MINOR && want.iter().zip(have).all(|(a, b)| *a == b)
}

/// The best of these: the newest that matches; a pre-release only when
/// nothing released does.
fn best(mut c: Vec<Py>, want: &[u32]) -> Option<Py> {
    c.retain(|p| matches(want, p.version));
    let stable = c.iter().any(|p| !p.pre);
    c.retain(|p| !p.pre || !stable);
    c.into_iter().max_by_key(|p| (p.version, !p.pre))
}

fn uv_python_dirs() -> Vec<PathBuf> {
    let mut v = vec![];
    if let Some(d) = std::env::var_os("UV_PYTHON_INSTALL_DIR") {
        v.push(PathBuf::from(d));
    }
    if let Some(d) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute()) {
        v.push(d.join("uv/python"));
    }
    if let Some(h) = home() {
        v.push(h.join(".local/share/uv/python"));
    }
    // (Where tessh puts one it fetched itself: see fetch_python.)
    v.push(cache_base().join("python"));
    v
}

fn installed(dirs: &[PathBuf], libc: Libc) -> Vec<Py> {
    let mut out = vec![];
    for d in dirs {
        let Ok(rd) = fs::read_dir(d) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if let Some((version, pre)) = parse_name(&name, libc) {
                let dir = e.path();
                if dir.join("lib").is_dir() {
                    out.push(Py { name, dir, version, pre });
                }
            }
        }
    }
    out
}

fn uv_version(uv: &Path) -> Option<Vec<u32>> {
    let o = Command::new(uv).arg("--version").output().ok().filter(|o| o.status.success())?;
    let t = String::from_utf8_lossy(&o.stdout).to_string();
    t.split_whitespace().nth(1)?.split('.').map(|x| x.parse().ok()).collect()
}

/// The uv to use: $UV, else the newer of the one carried in this file
/// (rt/bin/uv) and one on PATH (a newer uv knows newer Pythons).
fn find_uv(rt: &Path) -> Option<PathBuf> {
    if let Some(u) = std::env::var_os("UV").map(PathBuf::from).filter(|p| p.is_file()) {
        return Some(u);
    }
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if let Some(h) = home() {
        dirs.push(h.join(".local/bin"));
        dirs.push(h.join(".cargo/bin"));
    }
    let mut found: Vec<PathBuf> = dirs.into_iter().map(|d| d.join("uv")).filter(|p| p.is_file()).take(1).collect();
    found.push(rt.join("bin/uv"));
    found.into_iter().filter(|p| p.is_file()).filter_map(|p| uv_version(&p).map(|v| (v, p))).max_by(|a, b| a.0.cmp(&b.0)).map(|(_, p)| p)
}

/// Installs the CPython wanted with uv, when uv offers one newer than
/// `have` (or there's none yet); then the folders to look in.
fn install(rt: &Path, libc: Libc, want: &[u32], want_text: &str, have: Option<&Py>) -> Option<Vec<PathBuf>> {
    let Some(uv) = find_uv(rt) else {
        if have.is_some() {
            return None;
        }
        die("tessh gets its Python from uv, and uv isn't installed. Install it (one static file) with\n\n    curl -LsSf https://astral.sh/uv/install.sh | sh\n\nor see https://docs.astral.sh/uv/ , then run tessh again.");
    };
    let arch = std::env::consts::ARCH;
    let request = format!("cpython-{}-linux-{arch}-{}", if want_text.is_empty() { "3".to_string() } else { want_text.to_string() }, libc.suffix());
    // (uv's own list of what it can install: no network needed.)
    let out = Command::new(&uv).args(["python", "list", "--only-downloads", "--all-platforms", "--output-format", "json", &request]).output();
    let list: Vec<serde_json::Value> = out.ok().filter(|o| o.status.success()).and_then(|o| serde_json::from_slice(&o.stdout).ok()).unwrap_or_default();
    let offered: Vec<Py> = list
        .iter()
        .filter_map(|x| {
            let name = x["key"].as_str()?.to_string();
            let (version, pre) = parse_name(&name, libc)?;
            Some(Py { name, dir: PathBuf::new(), version, pre })
        })
        .collect();
    let url_of = |name: &str| list.iter().find(|x| x["key"].as_str() == Some(name)).and_then(|x| x["url"].as_str()).map(String::from);
    let pick = match best(offered, want) {
        Some(p) => p,
        None if have.is_some() => return None,
        None => {
            let what = if want_text.is_empty() { format!("3.{MIN_MINOR} or later") } else { want_text.to_string() };
            die(&format!("uv ({}) has no {} build of CPython {what} to offer. A newer uv may: `uv self update`.", uv.display(), libc.suffix()));
        }
    };
    if have.is_some_and(|h| (h.version, !h.pre) >= (pick.version, !pick.pre)) {
        return None;
    }
    let v = pick.version;
    match have {
        Some(h) => eprintln!("tessh: uv has Python {}.{}.{} now (this was {}.{}.{}): getting it…", v.0, v.1, v.2, h.version.0, h.version.1, h.version.2),
        None => eprintln!("tessh: getting Python {}.{}.{} with uv (once)…", v.0, v.1, v.2),
    }
    // (uv's complaints are shown only if the fallback fails too.)
    let out = Command::new(&uv).args(["python", "install", "--no-bin", "--quiet", &pick.name]).stdin(std::process::Stdio::null()).output();
    let ok = out.as_ref().is_ok_and(|o| o.status.success());
    let uv_said = out.map(|o| String::from_utf8_lossy(&o.stderr).trim().to_string()).unwrap_or_default();
    if !ok {
        // (uv tells the system's libc from /bin/sh's program loader, and
        // gives up where there's none: a fully static system. Then the
        // same build, fetched from where uv would have.)
        let own = cache_base().join("python");
        match url_of(&pick.name).ok_or_else(|| "uv gave no address for it".to_string()).and_then(|u| {
            eprintln!("tessh: uv couldn't install it here; fetching it directly…");
            fetch_python(&u, &own.join(&pick.name))
        }) {
            Ok(()) => return Some(vec![own]),
            Err(e) if have.is_some() => {
                eprintln!("tessh: couldn't get Python {}.{}.{} ({e}); staying on the one there is", v.0, v.1, v.2);
                return None;
            }
            Err(e) => die(&format!("couldn't get Python {}.{}.{}: {e}\n(uv said: {uv_said})", v.0, v.1, v.2)),
        }
    }
    let mut dirs = uv_python_dirs();
    if let Ok(o) = Command::new(&uv).args(["python", "dir"]).output() {
        let d = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !d.is_empty() {
            dirs.insert(0, PathBuf::from(d));
        }
    }
    Some(dirs)
}

/// A python-build-standalone install_only archive, unpacked to `dest`.
fn fetch_python(url: &str, dest: &Path) -> Result<(), String> {
    let parent = dest.parent().ok_or("no folder")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = parent.join(format!(".fetch-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let resp = ureq::get(url).call().map_err(|e| e.to_string())?;
    let gz = flate2::read::GzDecoder::new(resp.into_body().into_reader());
    tar::Archive::new(gz).unpack(&tmp).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(dest);
    fs::rename(tmp.join("python"), dest).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(&tmp);
    Ok(())
}

/// With no version asked for, whether to see (once a day) if uv offers a
/// newer CPython than the newest one installed.
fn check_due(f: &Path) -> bool {
    let age = fs::metadata(&f).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok());
    age.is_none_or(|a| a.as_secs() > 24 * 3600)
}

fn symlink(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// `rt/py/<name>`: tessh with this Python linked in beside it.
fn version_dir(rt: &Path, py: &Py, libc: Libc) -> std::io::Result<PathBuf> {
    let vdir = rt.join("py").join(&py.name);
    if vdir.join(".done").exists() && fs::metadata(vdir.join("lib/libpython3.so")).is_ok() {
        return Ok(vdir);
    }
    let lib = py.dir.join("lib");
    let (mut libpy, mut stdlib) = (None, None);
    for e in fs::read_dir(&lib)?.flatten() {
        let n = e.file_name().to_string_lossy().to_string();
        if n.starts_with("libpython3.") && n.ends_with(".so.1.0") {
            libpy = Some(e.path());
        } else if n.starts_with("python3.") && e.path().join("os.py").exists() {
            stdlib = Some(n);
        }
    }
    let (Some(libpy), Some(stdlib)) = (libpy, stdlib) else {
        return Err(std::io::Error::other(format!("{} doesn't look like a CPython install", py.dir.display())));
    };
    fs::create_dir_all(rt.join("py"))?;
    let tmp = rt.join("py").join(format!(".new-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("bin"))?;
    fs::create_dir_all(tmp.join("lib"))?;
    if fs::hard_link(libc.tessh(rt), tmp.join("bin/tessh")).is_err() {
        fs::copy(libc.tessh(rt), tmp.join("bin/tessh"))?;
    }
    for n in ["python", "python3", &stdlib] {
        symlink(Path::new("tessh"), &tmp.join("bin").join(n))?;
    }
    // (uv beside it, for `need` and --with.)
    if rt.join("bin/uv").is_file() {
        symlink(&rt.join("bin/uv"), &tmp.join("bin/uv"))?;
    }
    symlink(&libpy, &tmp.join("lib/libpython3.so"))?;
    symlink(&lib.join(&stdlib), &tmp.join("lib").join(&stdlib))?;
    if py.dir.join("include").is_dir() {
        symlink(&py.dir.join("include"), &tmp.join("include"))?;
    }
    // (musl's loader and libgcc_s, for the musl build; the glibc one has the system's.)
    if libc == Libc::Musl {
        for e in fs::read_dir(rt.join("lib"))?.flatten() {
            symlink(&e.path(), &tmp.join("lib").join(e.file_name()))?;
        }
    }
    fs::write(tmp.join(".done"), b"")?;
    let _ = fs::remove_dir_all(&vdir);
    if fs::rename(&tmp, &vdir).is_err() {
        let _ = fs::remove_dir_all(&tmp);
    }
    Ok(vdir)
}

/// This libc's tessh with the Python wanted (installed with uv when needed).
fn python_for(rt: &Path, libc: Libc, want: &[u32], want_text: &str) -> PathBuf {
    let have = best(installed(&uv_python_dirs(), libc), want);
    let checked = cache_base().join(format!("python-checked-{}", libc.suffix()));
    let due = want_text.is_empty() && check_due(&checked);
    let py = match have {
        Some(p) if !due => p,
        have => {
            let dirs = install(rt, libc, want, want_text, have.as_ref());
            if due {
                let _ = fs::write(&checked, b"");
            }
            match dirs {
                Some(d) => best(installed(&d, libc), want).or(have).unwrap_or_else(|| die("uv installed Python, but tessh can't find where")),
                None => have.unwrap_or_else(|| die("no Python")),
            }
        }
    };
    version_dir(rt, &py, libc).unwrap_or_else(|e| die(&format!("couldn't set up Python {}: {e}", py.name)))
}

fn config_want() -> Option<String> {
    let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).or_else(|| home().map(|h| h.join(".config")))?;
    let t = fs::read_to_string(base.join("tessh/python")).ok()?;
    t.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#')).map(String::from)
}

fn main() {
    let exe = fs::read_link("/proc/self/exe").unwrap_or_else(|e| die(&format!("can't find myself: {e}")));
    let (off, len, hash) = trailer(&exe).unwrap_or_else(|e| die(&e.to_string()));
    let mut args: Vec<OsString> = std::env::args_os().collect();
    if args.is_empty() {
        args.push("tessh".into());
    }
    // (--python X.Y first: which Python; taken out here.)
    let mut want_text: Option<String> = None;
    match args.get(1).map(|a| a.to_string_lossy().to_string()) {
        Some(a) if a == "--python" && args.len() > 2 => {
            want_text = Some(args[2].to_string_lossy().to_string());
            args.drain(1..3);
        }
        Some(a) if a.starts_with("--python=") => {
            want_text = Some(a["--python=".len()..].to_string());
            args.remove(1);
        }
        _ => {}
    }
    let want_text = want_text.or_else(|| std::env::var("TESSH_PYTHON").ok().filter(|s| !s.trim().is_empty())).or_else(config_want).unwrap_or_default();
    let want = if want_text.is_empty() { vec![3] } else { parse_want(&want_text).unwrap_or_else(|| die(&format!("Python version {want_text:?}: say it as 3.14 or 3.14.6"))) };
    if want.first() != Some(&3) || want.get(1).is_some_and(|m| *m < MIN_MINOR) {
        die(&format!("tessh needs Python 3.{MIN_MINOR} or later (asked for {want_text})"));
    }
    let rt = cache_base().join(format!("rt-{hash}"));
    if let Err(e) = unpack(&exe, off, len, &rt) {
        die(&format!("couldn't unpack into {}: {e}", rt.display()));
    }
    let (mut libc, mut settled) = pick_libc(&rt);
    let vdir = loop {
        let vdir = python_for(&rt, libc, &want, &want_text);
        if settled {
            break vdir;
        }
        // (glibc's tessh, the first time: does it start here? If not, musl's.)
        let ok = Command::new(vdir.join("bin/tessh")).arg("--version").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success());
        let _ = fs::write(rt.join("libc"), if ok { "gnu" } else { "musl" });
        if ok {
            break vdir;
        }
        libc = Libc::Musl;
        settled = true;
    };
    let err = Command::new(vdir.join("bin/tessh")).arg0(&args[0]).args(&args[1..]).exec();
    die(&format!("couldn't run {}: {err}", vdir.join("bin/tessh").display()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn py(name: &str) -> Py {
        let (version, pre) = parse_name(name, Libc::Musl).unwrap();
        Py { name: name.into(), dir: PathBuf::new(), version, pre }
    }

    #[test]
    fn names() {
        let a = std::env::consts::ARCH;
        assert_eq!(parse_name(&format!("cpython-3.14.6-linux-{a}-musl"), Libc::Musl), Some(((3, 14, 6), false)));
        assert_eq!(parse_name(&format!("cpython-3.14.6-linux-{a}-gnu"), Libc::Gnu), Some(((3, 14, 6), false)));
        assert_eq!(parse_name(&format!("cpython-3.14.6-linux-{a}-gnu"), Libc::Musl), None);
        assert_eq!(parse_name(&format!("cpython-3.15.0b2-linux-{a}-musl"), Libc::Musl), Some(((3, 15, 0), true)));
        assert_eq!(parse_name(&format!("cpython-3.14.6+freethreaded-linux-{a}-musl"), Libc::Musl), None);
    }

    #[test]
    fn picks() {
        let a = std::env::consts::ARCH;
        let all = vec![py(&format!("cpython-3.15.0b2-linux-{a}-musl")), py(&format!("cpython-3.14.6-linux-{a}-musl")), py(&format!("cpython-3.13.9-linux-{a}-musl")), py(&format!("cpython-3.11.9-linux-{a}-musl"))];
        assert_eq!(best(all.clone(), &[3]).unwrap().version, (3, 14, 6));
        assert_eq!(best(all.clone(), &[3, 13]).unwrap().version, (3, 13, 9));
        assert_eq!(best(all.clone(), &[3, 15]).unwrap().version, (3, 15, 0));
        assert_eq!(best(all.clone(), &[3, 11]), None);
        assert_eq!(best(all, &[3, 16]), None);
    }
}
