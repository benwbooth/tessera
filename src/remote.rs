//! tesserad on another host, through SSH. The window keeps one SSH
//! connection per host (OpenSSH's connection sharing: a master started once,
//! so a password or key is asked once) and forwards a local Unix socket to
//! the remote tesserad's: everything that works with the local tesserad
//! (shells, kernels, cells, Nix) then works there, unchanged.
//!
//! When the host has no tesserad (the probe exits 127), a static build of
//! it is copied over with scp, to ~/.local/bin (or ~ when that can't be
//! written), and started. Reconnecting finds the one already running, with
//! its shells and kernels.

use crate::net::UnixStream;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Where this host's tesserad listens there (like tmux's /tmp/tmux-UID).
const REMOTE_SOCKET: &str = "${TESSERA_REMOTE_SOCKET:-/tmp/tessera-$(id -u)/tesserad.sock}";

/// (Tests: variables for every command run there, `env A=1 B=2 ...`, so a
/// test "host" over ssh localhost gets folders of its own.)
fn remote_env() -> String {
    std::env::var("TESSERA_TEST_SSH_ENV").ok().filter(|e| !e.is_empty()).map(|e| format!("env {e} ")).unwrap_or_default()
}

/// The local socket forwarded to `host`'s tesserad, and the SSH control
/// socket, both short (a Unix socket's path is limited to ~100 bytes).
fn paths(host: &str) -> (PathBuf, PathBuf) {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    host.hash(&mut h);
    let base = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("tessera-ssh");
    let _ = std::fs::create_dir_all(&base);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700));
    }
    let tag = format!("{:012x}", h.finish() & 0xffff_ffff_ffff);
    (base.join(format!("{tag}.sock")), base.join(format!("{tag}.ctl")))
}

/// One setup per host at a time (two panes opening together share it).
fn lock(host: &str) -> std::sync::Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, std::sync::Arc<Mutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(Default::default).lock().unwrap().entry(host.to_string()).or_default().clone()
}

/// What went wrong last for each host (the window shows it).
fn errors() -> &'static Mutex<HashMap<String, String>> {
    static E: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    E.get_or_init(Default::default)
}

pub fn last_error(host: &str) -> String {
    errors().lock().unwrap().get(host).cloned().unwrap_or_default()
}

fn fail(host: &str, msg: String) -> std::io::Error {
    errors().lock().unwrap().insert(host.to_string(), msg.clone());
    std::io::Error::other(msg)
}

/// A connection to `host`'s tesserad: through the forwarded socket, set up
/// (and tesserad started or deployed there) when it isn't yet.
pub fn connect(host: &str) -> std::io::Result<UnixStream> {
    let (local, ctl) = paths(host);
    if let Ok(s) = UnixStream::connect(&local) {
        return Ok(s);
    }
    let l = lock(host);
    let _g = l.lock().unwrap();
    if let Ok(s) = UnixStream::connect(&local) {
        return Ok(s);
    }
    setup(host, &local, &ctl)?;
    errors().lock().unwrap().remove(host);
    UnixStream::connect(&local)
}

/// `host` is `user@host`, an alias from ~/.ssh/config, and maybe options
/// after it ("-p 2222 box"): ssh's arguments for it.
fn host_args(host: &str) -> Vec<String> {
    shell_words::split(host).unwrap_or_else(|_| vec![host.to_string()])
}

fn ssh(ctl: &Path, host: &str) -> Command {
    let mut c = Command::new("ssh");
    c.arg("-S").arg(ctl).args(["-o", "ControlMaster=no"]);
    c.args(host_args(host));
    c.stdin(Stdio::null());
    c
}

fn master_up(ctl: &Path, host: &str) -> bool {
    let mut c = Command::new("ssh");
    c.arg("-S").arg(ctl).args(["-O", "check"]).args(host_args(host));
    c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    c.status().is_ok_and(|s| s.success())
}

fn setup(host: &str, local: &Path, ctl: &Path) -> std::io::Result<()> {
    // 1. The shared connection (asks for a password, if any, through
    // SSH_ASKPASS: there's no terminal here).
    if !master_up(ctl, host) {
        let _ = std::fs::remove_file(ctl);
        let mut c = Command::new("ssh");
        c.args(["-M", "-N", "-o", "ControlPersist=no", "-o", "ServerAliveInterval=30", "-o", "StreamLocalBindUnlink=yes"]);
        c.arg("-S").arg(ctl).args(host_args(host));
        c.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
        // (Its own session: not ended with the window's terminal or group.)
        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            c.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        let mut child = c.spawn().map_err(|e| fail(host, format!("Couldn't run ssh: {e}")))?;
        let start = Instant::now();
        loop {
            if master_up(ctl, host) {
                break;
            }
            if let Ok(Some(st)) = child.try_wait() {
                let mut err = String::new();
                if let Some(mut e) = child.stderr.take() {
                    use std::io::Read;
                    let _ = e.read_to_string(&mut err);
                }
                return Err(fail(host, format!("ssh {host} failed ({st}): {}", err.trim())));
            }
            if start.elapsed() > Duration::from_secs(90) {
                let _ = child.kill();
                return Err(fail(host, format!("ssh {host}: no connection after 90 seconds")));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // (The master runs on by itself; nothing reads its stderr now.)
        drop(child.stderr.take());
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
    // 2. tesserad there: started (it prints its socket), else deployed.
    let remote_sock = match start_remote(ctl, host)? {
        Started::Socket(s) => s,
        Started::Missing { os, arch } => {
            deploy(ctl, host, &os, &arch)?;
            match start_remote(ctl, host)? {
                Started::Socket(s) => s,
                Started::Missing { .. } => return Err(fail(host, format!("tesserad was copied to {host} but still can't be run there"))),
            }
        }
    };
    // 3. The local socket, forwarded to it.
    let _ = std::fs::remove_file(local);
    let fwd = Command::new("ssh")
        .arg("-S")
        .arg(ctl)
        .args(["-O", "forward", "-L"])
        .arg(format!("{}:{}", local.display(), remote_sock))
        .args(host_args(host))
        .stdin(Stdio::null())
        .output()?;
    if !fwd.status.success() {
        return Err(fail(host, format!("Couldn't forward tesserad's socket from {host}: {}", String::from_utf8_lossy(&fwd.stderr).trim())));
    }
    let start = Instant::now();
    while !local.exists() && start.elapsed() < Duration::from_secs(5) {
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

enum Started {
    Socket(String),
    Missing { os: String, arch: String },
}

/// The script run there: tesserad from PATH or where Tessera puts it;
/// started unless running (its socket printed), or exit 127 with
/// `uname -sm` when there's none.
fn start_script() -> String {
    format!(
        r#"T=$(command -v tesserad 2>/dev/null); for d in "$HOME/.local/bin" "$HOME"; do [ -z "$T" ] && [ -x "$d/tesserad" ] && T="$d/tesserad"; done; if [ -z "$T" ]; then uname -sm; exit 127; fi; S="{REMOTE_SOCKET}"; mkdir -p "$(dirname "$S")" && chmod 700 "$(dirname "$S")"; TESSERA_REMOTE=1 exec "$T" start --socket "$S""#
    )
}

fn start_remote(ctl: &Path, host: &str) -> std::io::Result<Started> {
    // (sh -c: whatever the login shell there is, fish or nu included.)
    let out = ssh(ctl, host).arg("--").arg(format!("{}sh -c {}", remote_env(), shell_words::quote(&start_script()))).output()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    match out.status.code() {
        Some(0) if !text.is_empty() => Ok(Started::Socket(text.lines().last().unwrap_or("").to_string())),
        Some(127) => {
            let mut it = text.split_whitespace();
            Ok(Started::Missing { os: it.next().unwrap_or("").to_string(), arch: it.next().unwrap_or("").to_string() })
        }
        _ => Err(fail(host, format!("Couldn't start tesserad on {host}: {}", String::from_utf8_lossy(&out.stderr).trim()))),
    }
}

/// Copy a static tesserad for `arch` to the host: ~/.local/bin, or ~.
fn deploy(ctl: &Path, host: &str, os: &str, arch: &str) -> std::io::Result<()> {
    if os != "Linux" {
        return Err(fail(host, format!("{host} runs {os} {arch}: Tessera has a tesserad to copy there only for Linux. Install tesserad there and connect again.")));
    }
    let Some(bin) = static_build(arch) else {
        return Err(fail(host, format!("{host} has no tesserad, and there's no static build of it for {arch} here to copy (set TESSERA_TESSERAD_STATIC to one).")));
    };
    let dir_out = ssh(ctl, host)
        .arg("--")
        .arg(format!(r#"{}sh -c 'mkdir -p "$HOME/.local/bin" 2>/dev/null; if [ -w "$HOME/.local/bin" ]; then echo "$HOME/.local/bin"; else echo "$HOME"; fi'"#, remote_env()))
        .output()?;
    let dir = String::from_utf8_lossy(&dir_out.stdout).trim().to_string();
    if dir.is_empty() {
        return Err(fail(host, format!("Couldn't find a folder for tesserad on {host}")));
    }
    let tmp = format!("{dir}/.tesserad.{}", std::process::id());
    let mut scp = Command::new("scp");
    scp.args(["-q", "-o", "ControlMaster=no", "-o"]).arg(format!("ControlPath={}", ctl.display()));
    // (Options given with the host, a port say, go to scp too; its port flag is -P.)
    let mut args = host_args(host);
    let target = args.pop().unwrap_or_default();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-p" {
            args[i] = "-P".into();
        }
        i += 1;
    }
    scp.args(&args).arg(&bin).arg(format!("{target}:{tmp}"));
    let st = scp.stdin(Stdio::null()).output()?;
    if !st.status.success() {
        return Err(fail(host, format!("Couldn't copy tesserad to {host}: {}", String::from_utf8_lossy(&st.stderr).trim())));
    }
    let mv = ssh(ctl, host).arg("--").arg(format!("chmod 755 {q} && mv -f {q} {d}/tesserad", q = shell_words::quote(&tmp), d = shell_words::quote(&dir))).output()?;
    if !mv.status.success() {
        return Err(fail(host, format!("Couldn't install tesserad on {host}: {}", String::from_utf8_lossy(&mv.stderr).trim())));
    }
    Ok(())
}

/// A tesserad that runs on any Linux with `arch` (uname -m): statically
/// linked. TESSERA_TESSERAD_STATIC (a file, or a folder of
/// tesserad-<arch>-linux), the package's own (libexec), or this build's
/// tesserad when it's static and for that arch.
pub fn static_build(arch: &str) -> Option<PathBuf> {
    let arch = match arch {
        "arm64" => "aarch64",
        "amd64" => "x86_64",
        a => a,
    };
    let name = format!("tesserad-{arch}-linux");
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("TESSERA_TESSERAD_STATIC").map(PathBuf::from) {
        if p.is_dir() {
            cands.push(p.join(&name));
        } else {
            cands.push(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        cands.push(dir.join(&name));
        cands.push(dir.join("../libexec/tessera").join(&name));
        cands.push(dir.join("tesserad"));
    }
    if let Some(h) = std::env::var_os("HOME") {
        cands.push(PathBuf::from(h).join(".cache/tessera").join(&name));
    }
    if let Some(p) = cands.into_iter().find(|p| p.is_file() && elf_static_for(p, arch)) {
        return Some(p);
    }
    download(arch)
}

/// The release's static tesserad for `arch` (this version's), fetched once
/// into ~/.cache/tessera: for builds that carry none (a dev build).
fn download(arch: &str) -> Option<PathBuf> {
    let version = env!("CARGO_PKG_VERSION");
    let dir = PathBuf::from(std::env::var_os("HOME")?).join(".cache/tessera");
    let file = dir.join(format!("tesserad-{version}-{arch}-linux"));
    if file.is_file() && elf_static_for(&file, arch) {
        return Some(file);
    }
    let url = format!("https://github.com/benwbooth/tessera/releases/download/v{version}/tesserad-{arch}-linux");
    let tmp = dir.join(format!(".tesserad-{}", std::process::id()));
    let ok = std::fs::create_dir_all(&dir).is_ok()
        && Command::new("curl").args(["-fsSL", "--max-time", "120", "-o"]).arg(&tmp).arg(&url).stdin(Stdio::null()).status().is_ok_and(|s| s.success());
    if ok && elf_static_for(&tmp, arch) && std::fs::rename(&tmp, &file).is_ok() {
        return Some(file);
    }
    let _ = std::fs::remove_file(&tmp);
    None
}

/// Whether `p` is an ELF executable for `arch` with no dynamic loader.
fn elf_static_for(p: &Path, arch: &str) -> bool {
    let Ok(b) = std::fs::read(p) else { return false };
    if b.len() < 64 || &b[..4] != b"\x7fELF" || b[4] != 2 || b[5] != 1 {
        return false;
    }
    let machine = u16::from_le_bytes([b[18], b[19]]);
    let want = match arch {
        "x86_64" => 62,
        "aarch64" => 183,
        _ => return false,
    };
    if machine != want {
        return false;
    }
    let phoff = u64::from_le_bytes(b[32..40].try_into().unwrap()) as usize;
    let phentsize = u16::from_le_bytes([b[54], b[55]]) as usize;
    let phnum = u16::from_le_bytes([b[56], b[57]]) as usize;
    // (PT_INTERP: it needs a dynamic loader, so a libc like this one's.)
    !(0..phnum).any(|i| {
        let o = phoff + i * phentsize;
        b.get(o..o + 4).is_some_and(|t| u32::from_le_bytes(t.try_into().unwrap()) == 3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_args_split() {
        assert_eq!(super::host_args("-p 2222 me@box"), vec!["-p", "2222", "me@box"]);
        assert_eq!(super::host_args("nas"), vec!["nas"]);
    }

    /// The whole way over real SSH, to this computer as the "host" (needs
    /// `ssh localhost` with a key, and TESSERA_TESSERAD_STATIC set to a
    /// static tesserad): it isn't there, so it's copied over, started, and
    /// answers through the forwarded socket; a second connection finds it
    /// running. Its home and state are a scratch folder's.
    /// cargo test -p tesserad -- --ignored ssh_localhost
    #[test]
    #[ignore]
    fn ssh_localhost_deploys_and_attaches() {
        use crate::proto::*;
        let scratch = std::env::temp_dir().join(format!("tsr-{}", std::process::id()));
        let home = scratch.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let rsock = scratch.join("r").join("tessera-test.sock");
        std::fs::create_dir_all(rsock.parent().unwrap()).unwrap();
        std::env::set_var("XDG_RUNTIME_DIR", &scratch);
        std::env::set_var(
            "TESSERA_TEST_SSH_ENV",
            format!("HOME={h} XDG_STATE_HOME={h}/state PATH=/run/current-system/sw/bin:/usr/bin:/bin TESSERA_REMOTE_SOCKET={s}", h = home.display(), s = rsock.display()),
        );
        let host = "localhost";
        let mut s = super::connect(host).unwrap_or_else(|e| panic!("connect: {e}"));
        assert!(home.join(".local/bin/tesserad").is_file(), "copied to ~/.local/bin");
        write_json(&mut s, T_RPC, &serde_json::json!({"method": "host.info", "params": {}})).unwrap();
        let (tag, p) = read_frame(&mut s).unwrap();
        assert_eq!(tag, T_RPC_REPLY);
        let info: serde_json::Value = serde_json::from_slice(&p).unwrap();
        assert_eq!(info["home"].as_str(), Some(home.to_str().unwrap()));
        assert!(info["state_dir"].as_str().unwrap().starts_with(home.to_str().unwrap()));
        // A shell there, left running; found again on a new connection.
        let mut c = super::connect(host).unwrap();
        write_json(&mut c, T_CREATE, &Create { cols: 80, rows: 24, cwd: String::new(), command: "sleep 60".into(), term: String::new(), input: String::new(), scrollback: 100 }).unwrap();
        let (tag, p) = read_frame(&mut c).unwrap();
        assert_eq!(tag, T_ATTACHED);
        let id: String = serde_json::from_slice(&p).unwrap();
        drop(c);
        std::thread::sleep(Duration::from_millis(300));
        let mut l = super::connect(host).unwrap();
        write_frame(&mut l, T_LIST, b"").unwrap();
        let (_, p) = read_frame(&mut l).unwrap();
        let list: Vec<SessionInfo> = serde_json::from_slice(&p).unwrap();
        assert!(list.iter().any(|x| x.id == id && !x.attached), "the shell runs on there, detached");
        // (Done: that shell, then tesserad there exits by itself.)
        let mut k = super::connect(host).unwrap();
        write_json(&mut k, T_KILL_ID, &id).unwrap();
        let (_, ctl) = super::paths(host);
        let _ = Command::new("ssh").arg("-S").arg(&ctl).args(["-O", "exit", host]).output();
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn this_build_is_not_static() {
        // (The test binary links glibc dynamically: not one to copy.)
        let exe = std::env::current_exe().unwrap();
        assert!(!super::elf_static_for(&exe, std::env::consts::ARCH) || cfg!(target_env = "musl"));
    }
}
