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

/// The local socket forwarded to `host`'s tesserad.
pub fn local_socket(host: &str) -> PathBuf {
    paths(host).0
}

/// A connection through `host`'s forwarded socket when it's set up already;
/// an error (at once) otherwise.
pub fn connect_ready(host: &str) -> std::io::Result<UnixStream> {
    UnixStream::connect(paths(host).0).map_err(|e| std::io::Error::new(e.kind(), format!("not connected to {host} yet ({e})")))
}

static CHECKED: Mutex<std::collections::BTreeSet<String>> = Mutex::new(std::collections::BTreeSet::new());

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

/// The hosts this window has connected to (and still reaches).
pub fn connected_hosts() -> Vec<String> {
    CHECKED.lock().unwrap().iter().filter(|h| UnixStream::connect(paths(h).0).is_ok()).cloned().collect()
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
    // (Once per run the whole way, though it answers: tesserad there is
    // brought up to this build's.)
    let checked = |h: &str| CHECKED.lock().unwrap().contains(h);
    if checked(host) && answers(&local) {
        return UnixStream::connect(&local);
    }
    let l = lock(host);
    let _g = l.lock().unwrap();
    if checked(host) && answers(&local) {
        return UnixStream::connect(&local);
    }
    CHECKED.lock().unwrap().insert(host.to_string());
    setup(host, &local, &ctl)?;
    errors().lock().unwrap().remove(host);
    UnixStream::connect(&local)
}

/// Whether tesserad answers through the forwarded socket. (A forward can be
/// up with nothing at its far end: tesserad there ended, its folder gone.)
fn answers(local: &Path) -> bool {
    use crate::proto::*;
    let Ok(mut s) = UnixStream::connect(local) else { return false };
    let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
    write_json(&mut s, T_VERSION, &()).is_ok() && matches!(read_frame(&mut s), Ok((T_VERSION_REPLY, _)))
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
    let (remote_sock, which) = match start_remote(ctl, host)? {
        Started::Socket(s, w) => (s, w),
        Started::Missing { os, arch } => {
            deploy(ctl, host, &os, &arch)?;
            match start_remote(ctl, host)? {
                Started::Socket(s, w) => (s, w),
                Started::Missing { .. } => return Err(fail(host, format!("tesserad was copied to {host} but still can't be run there"))),
            }
        }
    };
    // 3. The local socket, forwarded to it. (A forward from before reaches
    // it again once tesserad there is up at the same socket: kept. One that
    // still doesn't answer is dropped and made again.) Where the sshd won't
    // forward a Unix socket (Windows', or AllowStreamLocalForwarding no):
    // `tesserad mux` there, over one SSH channel.
    let windows = which.as_ref().is_some_and(|w| platform(&w.2, &w.3).is_some_and(|p| p.0 == "windows"));
    let force_mux = windows || std::env::var_os("TESSERA_TEST_FORCE_MUX").is_some();
    if !answers(local) && (force_mux || forward(host, ctl, local, &remote_sock).is_err()) {
        let path = which.as_ref().map(|w| w.0.clone()).ok_or_else(|| fail(host, format!("tesserad on {host} can't be reached: its socket isn't forwarded, and its path isn't known")))?;
        let mut c = ssh(ctl, host);
        c.arg("--").arg(if windows { windows_command(&format!("& {} mux", ps_quote(&path))) } else { format!("{}TESSERA_REMOTE=1 {} mux --socket {}", remote_env(), shell_words::quote(&path), shell_words::quote(&remote_sock)) });
        c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        let child = c.spawn()?;
        #[cfg(unix)]
        crate::mux::tunnel(child, local).map_err(|e| fail(host, format!("Couldn't reach tesserad on {host}: {e}")))?;
        #[cfg(not(unix))]
        {
            drop(child);
            return Err(fail(host, format!("{host} can't forward tesserad's socket, and this window can't carry it another way yet")));
        }
        let start = Instant::now();
        while !answers(local) {
            if start.elapsed() > Duration::from_secs(10) {
                return Err(fail(host, format!("tesserad on {host} doesn't answer through its connection")));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    // 4. A tesserad Tessera put there that isn't this build's: the new one
    // copied over and swapped in (in place: its shells carry on; with
    // kernels running, once they're done).
    if let Some((path, sum, os, arch)) = which {
        if let Err(e) = update(ctl, host, local, &path, &sum, &os, &arch) {
            eprintln!("tessera: {host}: tesserad not updated: {e}");
        }
    }
    Ok(())
}

/// The local socket forwarded to tesserad's on the host (ssh -O forward on
/// the shared connection); an error when the sshd won't, or it doesn't
/// answer.
fn forward(host: &str, ctl: &Path, local: &Path, remote_sock: &str) -> std::io::Result<()> {
    {
        let _ = Command::new("ssh")
            .arg("-S")
            .arg(ctl)
            .args(["-O", "cancel", "-L"])
            .arg(format!("{}:{}", local.display(), remote_sock))
            .args(host_args(host))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
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
        // (It answers before this says it's up: the master can be slow to bind
        // it, and drops the cancelled forward's file a moment later.)
        let start = Instant::now();
        while !answers(local) {
            if start.elapsed() > Duration::from_secs(5) {
                return Err(fail(host, format!("tesserad on {host} doesn't answer through the forwarded socket {}", local.display())));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    Ok(())
}

enum Started {
    /// Its socket, and which tesserad runs it: (path, `cksum` "crc size",
    /// os, arch).
    Socket(String, Option<(String, String, String, String)>),
    Missing { os: String, arch: String },
}

/// The script run there: tesserad from PATH or where Tessera puts it;
/// started unless running (its socket printed), or exit 127 with
/// `uname -sm` when there's none.
fn start_script() -> String {
    format!(
        r#"T=$(command -v tesserad 2>/dev/null); for d in "$HOME/.local/bin" "$HOME"; do [ -z "$T" ] && [ -x "$d/tesserad" ] && T="$d/tesserad"; done; if [ -z "$T" ]; then uname -sm; exit 127; fi; echo "TESSERAD $T $(cksum < "$T") $(uname -sm)"; S="{REMOTE_SOCKET}"; mkdir -p "$(dirname "$S")" && chmod 700 "$(dirname "$S")"; TESSERA_REMOTE=1 exec "$T" start --socket "$S""#
    )
}

fn start_remote(ctl: &Path, host: &str) -> std::io::Result<Started> {
    // (sh -c: whatever the login shell there is, fish or nu included.)
    let out = ssh(ctl, host).arg("--").arg(format!("{}sh -c {}", remote_env(), shell_words::quote(&start_script()))).output()?;
    // Windows: cmd.exe or PowerShell has no sh (WSL's, if any, isn't the
    // host's own): PowerShell's version of it.
    let err = String::from_utf8_lossy(&out.stderr).to_string();
    let out = if out.status.code() != Some(0) && out.status.code() != Some(127) && (err.contains("not recognized") || err.contains("CommandNotFound") || err.contains("is not recognised")) {
        ssh(ctl, host).arg("--").arg(windows_command(&windows_start_script())).output()?
    } else {
        out
    };
    let text = String::from_utf8_lossy(&out.stdout).replace('\r', "").trim().to_string();
    match out.status.code() {
        Some(0) if !text.is_empty() => {
            // TESSERAD <path, spaces and all> <sum> <size> <os> <arch>
            let which = text.lines().find_map(|l| {
                let w: Vec<&str> = l.strip_prefix("TESSERAD ")?.rsplitn(5, ' ').collect();
                (w.len() == 5).then(|| (w[4].to_string(), format!("{} {}", w[3], w[2]), w[1].to_string(), w[0].to_string()))
            });
            Ok(Started::Socket(text.lines().last().unwrap_or("").to_string(), which))
        }
        Some(127) => {
            let mut it = text.lines().last().unwrap_or("").split_whitespace();
            Ok(Started::Missing { os: it.next().unwrap_or("").to_string(), arch: it.next().unwrap_or("").to_string() })
        }
        _ => Err(fail(host, format!("Couldn't start tesserad on {host}: {}", String::from_utf8_lossy(&out.stderr).trim()))),
    }
}

/// A PowerShell script as a command line any Windows shell (cmd.exe,
/// PowerShell) runs as it is: -EncodedCommand (UTF-16, base64), so nothing
/// in it needs quoting for either.
fn windows_command(script: &str) -> String {
    use base64::Engine;
    let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    format!("powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand {}", base64::engine::general_purpose::STANDARD.encode(utf16))
}

/// A string for PowerShell, in single quotes.
fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Where Tessera puts tesserad on Windows.
const WINDOWS_DIR: &str = "(Join-Path $env:LOCALAPPDATA 'Tessera\\bin')";

/// start_script, for Windows: tesserad from PATH or Tessera's folder, its
/// SHA-256 (no cksum there), started (out of the SSH session's job: it
/// stays when the connection ends).
fn windows_start_script() -> String {
    format!(
        r#"$ErrorActionPreference = 'Stop'
$dir = {WINDOWS_DIR}
$t = $null
$c = Get-Command tesserad -ErrorAction SilentlyContinue
if ($c) {{ $t = $c.Source }} elseif (Test-Path (Join-Path $dir 'tesserad.exe')) {{ $t = Join-Path $dir 'tesserad.exe' }}
$arch = $env:PROCESSOR_ARCHITECTURE
if (-not $t) {{ "Windows $arch"; exit 127 }}
$h = (Get-FileHash -Algorithm SHA256 -LiteralPath $t).Hash.ToLower()
$n = (Get-Item -LiteralPath $t).Length
"TESSERAD $t sha256:$h $n Windows $arch"
$env:TESSERA_REMOTE = '1'
& $t start
exit $LASTEXITCODE
"#
    )
}

/// scp through the host's shared connection, and the host as scp names it.
/// (scp speaks SFTP: remote paths are taken as they are, spaces and all,
/// not through a shell.)
fn scp_to(ctl: &Path, host: &str) -> (Command, String) {
    let mut scp = Command::new("scp");
    scp.args(["-q", "-o", "ControlMaster=no", "-o"]).arg(format!("ControlPath={}", ctl.display()));
    // (Options given with the host, a port say, go to scp too; its port flag is -P.)
    let mut args = host_args(host);
    let target = args.pop().unwrap_or_default();
    for a in args.iter_mut() {
        if a == "-p" {
            *a = "-P".into();
        }
    }
    scp.args(&args);
    (scp, target)
}

/// A copy here of `path` on `host` (for what happens on this desktop:
/// opening it, a GIF of a recording): ~/.cache/tessera/remote/<host>/<path>,
/// fetched again each time.
pub fn fetch(host: &str, path: &str) -> Result<PathBuf, String> {
    let (_, ctl) = paths(host);
    connect(host).map_err(|e| e.to_string())?;
    let safe: String = host.chars().map(|c| if c.is_ascii_alphanumeric() || "@.-_".contains(c) { c } else { '_' }).collect();
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or("no HOME")?;
    let local = home.join(".cache/tessera/remote").join(safe).join(path.trim_start_matches('/'));
    if let Some(d) = local.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let (mut scp, target) = scp_to(&ctl, host);
    let out = scp.arg("-r").arg(format!("{target}:{path}")).arg(&local).stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("Couldn't copy it from {host}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(local)
}

/// Copy `local` (a file or folder here) into `dir` on `host`, under a name
/// not taken there: its path there.
pub fn upload(host: &str, local: &Path, dir: &str) -> Result<String, String> {
    let (_, ctl) = paths(host);
    connect(host).map_err(|e| e.to_string())?;
    let name = local.file_name().map(|n| n.to_string_lossy().to_string()).ok_or("no name")?;
    let to = crate::client::rpc_within(host, "files", serde_json::json!({"op": "unused_name", "path": dir, "arg": name}), Duration::from_secs(10))?;
    let to = to.as_str().unwrap_or("").to_string();
    if to.is_empty() {
        return Err(format!("No place for {name} in {dir} on {host}"));
    }
    let (mut scp, target) = scp_to(&ctl, host);
    let out = scp.arg("-r").arg(local).arg(format!("{target}:{to}")).stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("Couldn't copy {name} to {host}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(to)
}

/// Copy `path` on `host` into `dir` here, under a name not taken here.
pub fn download(host: &str, path: &str, dir: &Path) -> Result<PathBuf, String> {
    let (_, ctl) = paths(host);
    connect(host).map_err(|e| e.to_string())?;
    let name = Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).ok_or("no name")?;
    let to = crate::files::unused_name(dir, &name);
    let (mut scp, target) = scp_to(&ctl, host);
    let out = scp.arg("-r").arg(format!("{target}:{path}")).arg(&to).stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("Couldn't copy {name} from {host}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(to)
}

/// "sha256:<hex> <size>" for a file here (what Windows hosts report).
fn local_sha256(p: &Path) -> Option<String> {
    use sha2::Digest;
    let b = std::fs::read(p).ok()?;
    let h = sha2::Sha256::digest(&b);
    Some(format!("sha256:{} {}", h.iter().map(|x| format!("{x:02x}")).collect::<String>(), b.len()))
}

/// `cksum`'s "crc size" for a file here.
fn local_cksum(p: &Path) -> Option<String> {
    let f = std::fs::File::open(p).ok()?;
    let out = Command::new("cksum").stdin(f).output().ok()?;
    let t = String::from_utf8_lossy(&out.stdout);
    let w: Vec<&str> = t.split_whitespace().collect();
    (w.len() >= 2).then(|| format!("{} {}", w[0], w[1]))
}

fn update(ctl: &Path, host: &str, local: &Path, path: &str, sum: &str, os: &str, arch: &str) -> std::io::Result<()> {
    use crate::proto::*;
    let Ok(bin) = build_for(os, arch) else { return Ok(()) };
    let same = if sum.starts_with("sha256:") { local_sha256(&bin).as_deref() == Some(sum) } else { local_cksum(&bin).as_deref() == Some(sum) };
    if same {
        return Ok(());
    }
    // (Only one Tessera copied there itself: not a tesserad installed another way.)
    let windows = platform(os, arch).is_some_and(|p| p.0 == "windows");
    let dir = deploy_dir(ctl, host, windows)?;
    let ours = if windows { path.eq_ignore_ascii_case(&format!("{dir}\\tesserad.exe")) } else { path == format!("{dir}/tesserad") };
    if !ours {
        return Ok(());
    }
    deploy(ctl, host, os, arch)?;
    // (Windows can't swap a running build in place: the new one is there for
    // the next start, once this one has no shells left.)
    if windows {
        return Ok(());
    }
    let mut s = UnixStream::connect(local)?;
    let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
    write_json(&mut s, T_UPGRADE, &serde_json::json!({"exe": path}))?;
    let _ = read_frame(&mut s);
    // (The new build answers on the same socket once it has taken over.)
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(10) && !answers(local) {
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

/// Where Tessera puts tesserad on `host`: ~/.local/bin, or ~ when that
/// can't be written.
fn deploy_dir(ctl: &Path, host: &str, windows: bool) -> std::io::Result<String> {
    let cmd = if windows {
        windows_command(&format!("$d = {WINDOWS_DIR}; New-Item -ItemType Directory -Force -Path $d | Out-Null; $d"))
    } else {
        format!(r#"{}sh -c 'mkdir -p "$HOME/.local/bin" 2>/dev/null; if [ -w "$HOME/.local/bin" ]; then echo "$HOME/.local/bin"; else echo "$HOME"; fi'"#, remote_env())
    };
    let out = ssh(ctl, host).arg("--").arg(cmd).output()?;
    let dir = String::from_utf8_lossy(&out.stdout).replace('\r', "").trim().to_string();
    if dir.is_empty() {
        return Err(fail(host, format!("Couldn't find a folder for tesserad on {host}")));
    }
    Ok(dir)
}

/// Copy a static tesserad for `arch` to the host: ~/.local/bin, or ~.
fn deploy(ctl: &Path, host: &str, os: &str, arch: &str) -> std::io::Result<()> {
    // (This computer's build for the host's platform, else the release's.)
    let bin = build_for(os, arch).map_err(|e| fail(host, format!("{host} ({os} {arch}) has no tesserad, and {e}.")))?;
    let windows = platform(os, arch).is_some_and(|p| p.0 == "windows");
    let dir = deploy_dir(ctl, host, windows)?;
    // (scp takes Windows paths with forward slashes.)
    let tmp = if windows { format!("{}/tesserad.new.{}.exe", dir.replace('\\', "/"), std::process::id()) } else { format!("{dir}/.tesserad.{}", std::process::id()) };
    let (mut scp, target) = scp_to(ctl, host);
    scp.arg(&bin).arg(format!("{target}:{tmp}"));
    let st = scp.stdin(Stdio::null()).output()?;
    if !st.status.success() {
        return Err(fail(host, format!("Couldn't copy tesserad to {host}: {}", String::from_utf8_lossy(&st.stderr).trim())));
    }
    let mv = if windows {
        // (A running tesserad.exe can't be replaced, but it can be renamed:
        // it carries on as the old file, and the new one starts next time.)
        let exe = format!("{dir}\\tesserad.exe");
        let script = format!(
            "$e = {e}; $o = {e} + '.old'; Remove-Item -LiteralPath $o -Force -ErrorAction SilentlyContinue; if (Test-Path -LiteralPath $e) {{ Rename-Item -LiteralPath $e -NewName 'tesserad.exe.old' }}; Move-Item -Force -LiteralPath {t} -Destination $e",
            e = ps_quote(&exe),
            t = ps_quote(&tmp)
        );
        ssh(ctl, host).arg("--").arg(windows_command(&script)).output()?
    } else {
        ssh(ctl, host).arg("--").arg(format!("chmod 755 {q} && mv -f {q} {d}/tesserad", q = shell_words::quote(&tmp), d = shell_words::quote(&dir))).output()?
    };
    if !mv.status.success() {
        return Err(fail(host, format!("Couldn't install tesserad on {host}: {}", String::from_utf8_lossy(&mv.stderr).trim())));
    }
    Ok(())
}

/// The platform a host runs, as tesserad's builds are named: ("linux" |
/// "macos" | "windows", "x86_64" | "aarch64"), from `uname -sm` (or what
/// Windows says).
pub fn platform(os: &str, arch: &str) -> Option<(&'static str, &'static str)> {
    let os = match os.to_ascii_lowercase().as_str() {
        "linux" => "linux",
        "darwin" | "macos" => "macos",
        o if o.starts_with("windows") || o.starts_with("mingw") || o.starts_with("msys") || o.starts_with("cygwin") => "windows",
        _ => return None,
    };
    let arch = match arch.to_ascii_lowercase().as_str() {
        "x86_64" | "amd64" | "x64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => return None,
    };
    Some((os, arch))
}

/// The file name of tesserad's build for a platform.
pub fn build_name(os: &str, arch: &str) -> String {
    if os == "windows" { format!("tesserad-{arch}-windows.exe") } else { format!("tesserad-{arch}-{os}") }
}

/// A tesserad for a host running `os`/`arch` (uname -sm): one of this
/// computer's (TESSERA_TESSERAD_STATIC, a file or a folder of builds; the
/// package's own in libexec; this build when it fits), else the one the
/// release has, downloaded.
pub fn build_for(os: &str, arch: &str) -> Result<PathBuf, String> {
    let (os, arch) = platform(os, arch).ok_or_else(|| format!("there's no tesserad for {os} {arch}"))?;
    let name = build_name(os, arch);
    if let Some(p) = local_build(os, arch, &name) {
        return Ok(p);
    }
    download_release(os, arch, &name)
}

fn local_build(os: &str, arch: &str, name: &str) -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("TESSERA_TESSERAD_STATIC").map(PathBuf::from) {
        if p.is_dir() {
            cands.push(p.join(name));
        } else {
            cands.push(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        cands.push(dir.join(name));
        cands.push(dir.join("../libexec/tessera").join(name));
        cands.push(dir.join(if cfg!(windows) { "tesserad.exe" } else { "tesserad" }));
    }
    cands.into_iter().find(|p| p.is_file() && runs_on(p, os, arch))
}

/// The release's tesserad for a platform, into ~/.cache/tessera/<tag>/:
/// this version's release (v<version>), then tesserad-dev (built by hand
/// from the latest commit). TESSERA_TESSERAD_TAG names another to try first
/// (a dev build: tesserad-dev). A copy already there is checked for a newer
/// one (curl -z), so tesserad-dev's stays current.
fn download_release(os: &str, arch: &str, name: &str) -> Result<PathBuf, String> {
    let home = PathBuf::from(std::env::var_os("HOME").ok_or("no HOME")?);
    let mut tags: Vec<String> = Vec::new();
    if let Ok(t) = std::env::var("TESSERA_TESSERAD_TAG") {
        tags.push(t);
    }
    for t in [format!("v{}", env!("CARGO_PKG_VERSION")), "tesserad-dev".to_string()] {
        if !tags.contains(&t) {
            tags.push(t);
        }
    }
    let mut tried = Vec::new();
    for tag in tags {
        let dir = home.join(".cache/tessera").join(&tag);
        let file = dir.join(name);
        let url = format!("https://github.com/benwbooth/tessera/releases/download/{tag}/{name}");
        let tmp = dir.join(format!(".{name}.{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let mut curl = Command::new("curl");
        curl.args(["-fsSL", "--max-time", "300", "-o"]).arg(&tmp);
        // (Only when the release's is newer than the copy here.)
        if file.is_file() {
            curl.arg("-z").arg(&file);
        }
        let got = curl.arg(&url).stdin(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success());
        if got && tmp.is_file() && runs_on(&tmp, os, arch) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
            }
            let _ = std::fs::rename(&tmp, &file);
        }
        let _ = std::fs::remove_file(&tmp);
        if file.is_file() && runs_on(&file, os, arch) {
            return Ok(file);
        }
        tried.push(url);
    }
    Err(format!("there's no {name} here, and none to download ({})", tried.join(", ")))
}

/// Whether `p` is a tesserad that runs on `os`/`arch`: a static ELF for
/// Linux (any distribution), a Mach-O for macOS, a PE .exe for Windows.
fn runs_on(p: &Path, os: &str, arch: &str) -> bool {
    match os {
        "linux" => elf_static_for(p, arch),
        "macos" => macho_for(p, arch),
        "windows" => pe_for(p, arch),
        _ => false,
    }
}

fn macho_for(p: &Path, arch: &str) -> bool {
    let Ok(b) = std::fs::read(p) else { return false };
    if b.len() < 8 || b[..4] != [0xcf, 0xfa, 0xed, 0xfe] {
        return false;
    }
    let cpu = u32::from_le_bytes(b[4..8].try_into().unwrap());
    cpu == match arch {
        "x86_64" => 0x0100_0007,
        "aarch64" => 0x0100_000c,
        _ => return false,
    }
}

fn pe_for(p: &Path, arch: &str) -> bool {
    let Ok(b) = std::fs::read(p) else { return false };
    if b.len() < 0x40 || &b[..2] != b"MZ" {
        return false;
    }
    let pe = u32::from_le_bytes(b[0x3c..0x40].try_into().unwrap()) as usize;
    if b.get(pe..pe + 4) != Some(&b"PE\0\0"[..]) {
        return false;
    }
    let machine = b.get(pe + 4..pe + 6).map(|m| u16::from_le_bytes([m[0], m[1]])).unwrap_or(0);
    machine == match arch {
        "x86_64" => 0x8664,
        "aarch64" => 0xaa64,
        _ => return false,
    }
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
    /// The same, through `tesserad mux` (as for a host whose sshd won't
    /// forward a Unix socket: Windows').
    /// cargo test -p tesserad -- --ignored ssh_localhost
    #[test]
    #[ignore]
    fn ssh_localhost_through_mux() {
        std::env::set_var("TESSERA_TEST_FORCE_MUX", "1");
        ssh_localhost("mux");
    }

    #[test]
    #[ignore]
    fn ssh_localhost_deploys_and_attaches() {
        ssh_localhost("fwd");
    }

    fn ssh_localhost(tag: &str) {
        use crate::proto::*;
        let scratch = std::env::temp_dir().join(format!("tsr-{tag}-{}", std::process::id()));
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
        // Files both ways: a file here copied there (a free name the second
        // time), seen there, and fetched back.
        let here = scratch.join("dropped.txt");
        std::fs::write(&here, "from here").unwrap();
        let there = super::upload(host, &here, home.to_str().unwrap()).unwrap();
        assert_eq!(there, home.join("dropped.txt").display().to_string());
        let again = super::upload(host, &here, home.to_str().unwrap()).unwrap();
        assert_ne!(again, there, "a name not taken");
        let exists = crate::client::rpc(host, "files", serde_json::json!({"op": "exists", "path": there})).unwrap();
        assert_eq!(exists.as_str(), Some("true"));
        let back_dir = scratch.join("back");
        std::fs::create_dir_all(&back_dir).unwrap();
        let back = super::download(host, &there, &back_dir).unwrap();
        assert_eq!(std::fs::read_to_string(back).unwrap(), "from here");
        // (Its copy under this test's HOME, not the real one's cache.)
        std::env::set_var("HOME", &scratch);
        let copy = super::fetch(host, &there).unwrap();
        assert!(copy.starts_with(&scratch));
        assert_eq!(std::fs::read_to_string(copy).unwrap(), "from here");
        // (Done: that shell, then tesserad there exits by itself.)
        let mut k = super::connect(host).unwrap();
        write_json(&mut k, T_KILL_ID, &id).unwrap();
        let (_, ctl) = super::paths(host);
        let _ = Command::new("ssh").arg("-S").arg(&ctl).args(["-O", "exit", host]).output();
        let _ = std::fs::remove_dir_all(&scratch);
    }

    #[test]
    fn platforms_and_their_builds() {
        assert_eq!(platform("Linux", "x86_64"), Some(("linux", "x86_64")));
        assert_eq!(platform("Linux", "aarch64"), Some(("linux", "aarch64")));
        assert_eq!(platform("Darwin", "arm64"), Some(("macos", "aarch64")));
        assert_eq!(platform("Darwin", "x86_64"), Some(("macos", "x86_64")));
        assert_eq!(platform("Windows_NT", "AMD64"), Some(("windows", "x86_64")));
        assert_eq!(platform("FreeBSD", "amd64"), None);
        assert_eq!(build_name("macos", "aarch64"), "tesserad-aarch64-macos");
        assert_eq!(build_name("windows", "x86_64"), "tesserad-x86_64-windows.exe");
        let d = std::env::temp_dir().join(format!("tsr-bin-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        // A Mach-O for arm64, and a PE for x86-64: their headers.
        let mut macho = vec![0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 0x01];
        macho.resize(64, 0);
        std::fs::write(d.join("m"), &macho).unwrap();
        assert!(runs_on(&d.join("m"), "macos", "aarch64"));
        assert!(!runs_on(&d.join("m"), "macos", "x86_64"));
        assert!(!runs_on(&d.join("m"), "linux", "aarch64"));
        let mut pe = vec![0u8; 0x100];
        pe[..2].copy_from_slice(b"MZ");
        pe[0x3c] = 0x80;
        pe[0x80..0x84].copy_from_slice(b"PE\0\0");
        pe[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes());
        std::fs::write(d.join("w"), &pe).unwrap();
        assert!(runs_on(&d.join("w"), "windows", "x86_64"));
        assert!(!runs_on(&d.join("w"), "windows", "aarch64"));
        let _ = std::fs::remove_dir_all(&d);
    }

    /// The PowerShell sent to Windows hosts, written out for a parser to
    /// check (TESSERA_DUMP_PS=<folder>); and its command line decodes back.
    #[test]
    fn windows_scripts() {
        use base64::Engine;
        let s = windows_start_script();
        assert!(s.contains("Get-FileHash") && s.contains("& $t start"));
        let cmd = windows_command(&s);
        let b64 = cmd.rsplit(' ').next().unwrap();
        let raw = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        let back = String::from_utf16(&raw.chunks(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect::<Vec<_>>()).unwrap();
        assert_eq!(back, s);
        assert_eq!(ps_quote("C:\\Users\\O'Neil"), "'C:\\Users\\O''Neil'");
        if let Some(d) = std::env::var_os("TESSERA_DUMP_PS") {
            std::fs::write(std::path::Path::new(&d).join("start.ps1"), &s).unwrap();
            std::fs::write(std::path::Path::new(&d).join("dir.ps1"), format!("$d = {WINDOWS_DIR}; New-Item -ItemType Directory -Force -Path $d | Out-Null; $d")).unwrap();
        }
    }

    #[test]
    fn this_build_is_not_static() {
        // (The test binary links glibc dynamically: not one to copy.)
        let exe = std::env::current_exe().unwrap();
        assert!(!super::elf_static_for(&exe, std::env::consts::ARCH) || cfg!(target_env = "musl"));
    }
}
