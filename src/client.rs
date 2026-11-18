//! Connecting to tesserad, starting it when it isn't running; on another
//! host, through SSH (remote.rs).

use crate::net::UnixStream;
use crate::proto::*;
use serde_json::Value;
use std::time::{Duration, Instant};

/// A connection to `host`'s tesserad ("" for this computer's).
pub fn connect_to(host: &str) -> std::io::Result<UnixStream> {
    if host.is_empty() {
        connect()
    } else {
        crate::remote::connect(host)
    }
}

/// One request to `host`'s tesserad (rpc.rs): its result.
pub fn rpc(host: &str, method: &str, params: Value) -> Result<Value, String> {
    let mut s = connect_to(host).map_err(|e| e.to_string())?;
    // (Long ones, like a Nix lookup building its index, answer later.)
    let _ = s.set_read_timeout(Some(Duration::from_secs(120)));
    write_json(&mut s, T_RPC, &serde_json::json!({"method": method, "params": params})).map_err(|e| e.to_string())?;
    match read_frame(&mut s).map_err(|e| e.to_string())? {
        (T_RPC_REPLY, p) => serde_json::from_slice(&p).map_err(|e| e.to_string()),
        (T_ERROR, p) => Err(serde_json::from_slice::<String>(&p).unwrap_or_default()),
        (t, _) => Err(format!("tesserad: unexpected reply {t}")),
    }
}

/// tesserad on this computer, started unless it's running.
pub fn ensure_daemon() -> std::io::Result<()> {
    connect().map(drop)
}

/// The tesserad to run here: the one next to this program (the window's
/// build), this program when it's tesserad, else `tessera daemon`.
pub fn daemon_exe() -> std::io::Result<std::path::PathBuf> {
    let exe = std::env::current_exe()?;
    let name = if cfg!(windows) { "tesserad.exe" } else { "tesserad" };
    if exe.file_name().is_some_and(|n| n == name) {
        return Ok(exe);
    }
    let beside = exe.with_file_name(name);
    Ok(if beside.is_file() { beside } else { exe })
}

// ---- kernels' events (rpc.rs's T_SUBSCRIBE), per host ----------------------

type Deliver = std::sync::Arc<dyn Fn(String, Value) + Send + Sync>;

#[derive(Default)]
struct Subs {
    /// Each host's subscription id (its event connection is up).
    ids: std::collections::HashMap<String, u64>,
    /// The notebooks' kernel keys this window has there (re-claimed when
    /// the connection comes back).
    claimed: std::collections::HashMap<String, std::collections::BTreeSet<String>>,
    deliver: Option<Deliver>,
}

fn subs() -> &'static std::sync::Mutex<Subs> {
    static S: std::sync::OnceLock<std::sync::Mutex<Subs>> = std::sync::OnceLock::new();
    S.get_or_init(Default::default)
}

/// A kernel key as the window has it: "host//key" for another host's.
pub fn split_key(key: &str) -> (String, String) {
    match key.split_once("//") {
        Some((h, k)) => (h.to_string(), k.to_string()),
        None => (String::new(), key.to_string()),
    }
}

fn join_key(host: &str, key: &str) -> String {
    if host.is_empty() { key.to_string() } else { format!("{host}//{key}") }
}

/// Where kernels' events go (the window's Qt thread).
pub fn set_event_sink(deliver: Deliver) {
    subs().lock().unwrap().deliver = Some(deliver);
}

/// This window's subscription on `host` (its event connection started now
/// if it isn't up): its id, for kernel.start/attach.
pub fn subscription(host: &str) -> u64 {
    if let Some(id) = subs().lock().unwrap().ids.get(host) {
        return *id;
    }
    let Ok(mut s) = connect_to(host) else { return 0 };
    if write_frame(&mut s, T_SUBSCRIBE, b"").is_err() {
        return 0;
    }
    let id = match read_frame(&mut s) {
        Ok((T_SUBSCRIBED, p)) => serde_json::from_slice::<u64>(&p).unwrap_or(0),
        _ => return 0,
    };
    subs().lock().unwrap().ids.insert(host.to_string(), id);
    let host = host.to_string();
    std::thread::spawn(move || {
        while let Ok((tag, p)) = read_frame(&mut s) {
            if tag != T_EVENT {
                continue;
            }
            let v: Value = serde_json::from_slice(&p).unwrap_or_default();
            let deliver = subs().lock().unwrap().deliver.clone();
            if let (Some(d), Some(k)) = (deliver, v["key"].as_str()) {
                d(join_key(&host, k), v["event"].clone());
            }
        }
        // (Gone: tesserad restarted, the network dropped. Back as soon as it
        // can be, with the kernels this window had.)
        subs().lock().unwrap().ids.remove(&host);
        for _ in 0..600 {
            std::thread::sleep(Duration::from_secs(1));
            let keys: Vec<String> = subs().lock().unwrap().claimed.get(&host).map(|k| k.iter().cloned().collect()).unwrap_or_default();
            if keys.is_empty() {
                return;
            }
            if subscription(&host) == 0 {
                continue;
            }
            for k in keys {
                let _ = attach_kernels(&join_key(&host, &k));
            }
            return;
        }
    });
    id
}

/// Start kernel `spec` for `key` (unless it runs already); its events come
/// to the event sink. Returns what tesserad said ({running, events}).
pub fn start_kernel(key: &str, spec: &str, cwd: &str, pid: u32, session: &str) -> Value {
    let (host, k) = split_key(key);
    let sub = subscription(&host);
    subs().lock().unwrap().claimed.entry(host.clone()).or_default().insert(k.clone());
    let r = rpc(&host, "kernel.start", serde_json::json!({"key": k, "spec": spec, "cwd": cwd, "pid": pid, "sub": sub, "session": session}));
    match r {
        Ok(v) => {
            deliver_buffered(&host, &k, &v["events"]);
            v
        }
        Err(e) => {
            if let Some(d) = subs().lock().unwrap().deliver.clone() {
                d(key.to_string(), serde_json::json!({"type": "status", "state": "dead", "message": e}));
            }
            Value::Null
        }
    }
}

fn deliver_buffered(host: &str, key: &str, events: &Value) {
    let Some(d) = subs().lock().unwrap().deliver.clone() else { return };
    for e in events.as_array().into_iter().flatten() {
        // (kernel.attach's are {key, event}; kernel.start's, the events.)
        match (e["key"].as_str(), e.get("event")) {
            (Some(k), Some(ev)) => d(join_key(host, k), ev.clone()),
            _ => d(join_key(host, key), e.clone()),
        }
    }
}

/// A notebook (key) takes its kernels back after the window was away:
/// {kernels: [running ones, keys as the window has them]}; what they said
/// meanwhile is delivered as events.
pub fn attach_kernels(key: &str) -> Value {
    let (host, k) = split_key(key);
    let sub = subscription(&host);
    subs().lock().unwrap().claimed.entry(host.clone()).or_default().insert(k.clone());
    let Ok(mut v) = rpc(&host, "kernel.attach", serde_json::json!({"key": k, "sub": sub})) else { return serde_json::json!({"kernels": []}) };
    if let Some(list) = v["kernels"].as_array_mut() {
        for x in list.iter_mut() {
            if let Some(kk) = x["key"].as_str().map(String::from) {
                x["key"] = Value::String(join_key(&host, &kk));
            }
            // (Each kernel's own key under this notebook: also claimed.)
            if let Some(kk) = x["key"].as_str() {
                let (_, base) = split_key(kk);
                subs().lock().unwrap().claimed.entry(host.clone()).or_default().insert(base);
            }
        }
    }
    let events = v["events"].take();
    deliver_buffered(&host, &k, &events);
    v
}

/// The kernel for `key` is done with (stopped, or the notebook closed).
pub fn release_kernel(key: &str) {
    let (host, k) = split_key(key);
    if let Some(c) = subs().lock().unwrap().claimed.get_mut(&host) {
        c.remove(&k);
    }
}

pub fn connect() -> std::io::Result<UnixStream> {
    let path = socket_path();
    if let Ok(s) = UnixStream::connect(&path) {
        return Ok(s);
    }
    spawn_daemon()?;
    let start = Instant::now();
    loop {
        match UnixStream::connect(&path) {
            Ok(s) => return Ok(s),
            Err(e) if start.elapsed() > Duration::from_secs(3) => return Err(e),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

fn spawn_daemon() -> std::io::Result<()> {
    let exe = daemon_exe()?;
    let log = log_path();
    let stderr = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log)
        .map(std::process::Stdio::from)
        .unwrap_or_else(|_| std::process::Stdio::null());
    let mut cmd = std::process::Command::new(exe);
    // Shells get the desktop session's environment, not whatever this GUI
    // was started from (a `nix develop` shell under devloop, say: its SHELL
    // is a bash without completion and its build variables would leak in).
    // (Over SSH, `tesserad start` keeps the login's environment: there's
    // no desktop session there.)
    if std::env::var_os("TESSERA_REMOTE").is_none() {
        cmd.env_clear().envs(session_env());
    }
    cmd.arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(stderr);
    // Its own session, so it survives the GUI and its terminal.
    #[cfg(unix)]
    unsafe {
        use std::os::unix::process::CommandExt;
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window, and not ended with the GUI's process group.
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
    cmd.spawn()?;
    Ok(())
}

fn log_path() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir)
        .join("tessera");
    let _ = std::fs::create_dir_all(&base);
    base.join("daemon.log")
}

/// Which host each session the window has is on (its pane's), for the
/// calls that only name the session.
fn session_hosts() -> &'static std::sync::Mutex<std::collections::HashMap<String, String>> {
    static H: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, String>>> = std::sync::OnceLock::new();
    H.get_or_init(Default::default)
}

pub fn set_session_host(id: &str, host: &str) {
    if !host.is_empty() {
        session_hosts().lock().unwrap().insert(id.to_string(), host.to_string());
    }
}

pub fn host_of(id: &str) -> String {
    session_hosts().lock().unwrap().get(id).cloned().unwrap_or_default()
}

/// A session's connection: to its host's tesserad, never starting one
/// just to ask about it.
fn connect_for(id: &str) -> std::io::Result<UnixStream> {
    let host = host_of(id);
    if host.is_empty() {
        UnixStream::connect(socket_path())
    } else {
        crate::remote::connect(&host)
    }
}

pub fn list_sessions(host: &str) -> std::io::Result<Vec<SessionInfo>> {
    // Don't start a daemon just to learn there's nothing to restore.
    let mut s = if host.is_empty() { UnixStream::connect(socket_path())? } else { crate::remote::connect(host)? };
    write_frame(&mut s, T_LIST, b"")?;
    let (tag, payload) = read_frame(&mut s)?;
    if tag != T_SESSIONS {
        return Ok(vec![]);
    }
    Ok(serde_json::from_slice(&payload)?)
}

/// End a (detached) session.
pub fn kill_session(id: &str) {
    if let Ok(mut s) = connect_for(id) {
        let _ = write_json(&mut s, T_KILL_ID, &id);
    }
}

/// The foreground program of a session when it isn't the shell ("" if idle).
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct ProcInfo {
    pub job: String,
    pub cwd: String,
    pub shell: String,
    /// The shell's process id (0 from an older daemon).
    #[serde(default)]
    pub pid: u32,
}

/// What a session is doing; from a daemon older than T_INFO, just the job.
pub fn proc_info(id: &str) -> std::io::Result<ProcInfo> {
    use std::sync::atomic::{AtomicBool, Ordering};
    static OLD_DAEMON: AtomicBool = AtomicBool::new(false);
    if !OLD_DAEMON.load(Ordering::Relaxed) {
        let mut s = connect_for(id)?;
        s.set_read_timeout(Some(Duration::from_millis(500)))?;
        write_json(&mut s, T_INFO, &id)?;
        match read_frame(&mut s) {
            Ok((T_INFO_REPLY, payload)) => return Ok(serde_json::from_slice(&payload).unwrap_or_default()),
            Ok((T_ERROR, _)) => OLD_DAEMON.store(true, Ordering::Relaxed),
            _ => {}
        }
    }
    Ok(ProcInfo { job: session_job(id)?, ..Default::default() })
}

pub fn session_job(id: &str) -> std::io::Result<String> {
    let mut s = connect_for(id)?;
    s.set_read_timeout(Some(Duration::from_millis(500)))?;
    write_json(&mut s, T_JOB, &id)?;
    let (tag, payload) = read_frame(&mut s)?;
    if tag != T_JOB_REPLY {
        return Ok(String::new());
    }
    Ok(serde_json::from_slice(&payload).unwrap_or_default())
}

/// Bring the running daemon up to this build. "none" (no daemon yet),
/// "current", "upgraded" (swapped in place; every shell kept), "outdated"
/// (a build from before in-place upgrades: it has to be restarted), or
/// "newer" (the daemon is a newer build than this window: leave it).
pub fn sync_daemon() -> &'static str {
    let ask_version = || -> Option<Result<String, ()>> {
        let mut s = UnixStream::connect(socket_path()).ok()?;
        s.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
        write_json(&mut s, T_VERSION, &()).ok()?;
        match read_frame(&mut s) {
            Ok((T_VERSION_REPLY, p)) => Some(Ok(serde_json::from_slice(&p).unwrap_or_default())),
            Ok((T_ERROR, _)) => Some(Err(())),
            _ => None,
        }
    };
    let Ok(exe) = daemon_exe() else { return "current" };
    let mine = exe_identity_of(&exe);
    let theirs = match ask_version() {
        None => return "none",
        Some(Err(())) => return "outdated",
        Some(Ok(id)) => id,
    };
    if theirs == mine || mine.is_empty() {
        return "current";
    }
    // Only ever forward (two installed builds mustn't swap back and forth).
    let mtime = |id: &str| id.split(':').nth(1).and_then(|m| m.parse::<u128>().ok()).unwrap_or(0);
    if mtime(&theirs) > mtime(&mine) {
        return "newer";
    }
    let sent = (|| -> std::io::Result<String> {
        let mut s = UnixStream::connect(socket_path())?;
        s.set_read_timeout(Some(Duration::from_secs(5)))?;
        write_json(&mut s, T_UPGRADE, &serde_json::json!({ "exe": exe.display().to_string() }))?;
        let (_, p) = read_frame(&mut s)?;
        Ok(serde_json::from_slice::<String>(&p).unwrap_or_default())
    })();
    // (Deferred: kernels are running; it swaps itself in when they're done.)
    match sent.as_deref() {
        Ok("deferred") | Err(_) => return "current",
        _ => {}
    }
    // The new build answers on the same socket once it has taken over.
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(Ok(id)) = ask_version() {
            if id == mine {
                return "upgraded";
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    "current"
}

/// End the daemon (and its shells): for a build too old to upgrade itself.
pub fn stop_daemon() -> bool {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let Ok(s) = UnixStream::connect(socket_path()) else { return false };
        let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let ok = unsafe { libc::getsockopt(s.as_raw_fd(), libc::SOL_SOCKET, libc::SO_PEERCRED, (&mut cred as *mut libc::ucred).cast(), &mut len) } == 0;
        drop(s);
        if !ok || cred.pid <= 0 {
            return false;
        }
        unsafe { libc::kill(cred.pid, libc::SIGTERM) };
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(3) {
            if UnixStream::connect(socket_path()).is_err() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        unsafe { libc::kill(cred.pid, libc::SIGKILL) };
        let _ = std::fs::remove_file(socket_path());
        true
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// The desktop session's environment: systemd's user manager has it (what
/// apps started from the desktop get). Falls back to a cleaned-up copy of our
/// own. Tessera's own settings (TESSERA_SOCKET, XDG_STATE_HOME) are kept.
fn session_env() -> Vec<(String, String)> {
    // (Linux desktops only: elsewhere the app's own environment is the
    // session's.)
    if !cfg!(target_os = "linux") {
        return std::env::vars().collect();
    }
    let mut env: Vec<(String, String)> = std::process::Command::new("systemctl")
        .args(["--user", "show-environment"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| parse_systemd_env(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default();
    if !env.iter().any(|(k, _)| k == "PATH") {
        env = std::env::vars()
            .filter(|(k, _)| {
                ["HOME", "USER", "LOGNAME", "LANG", "PATH", "DISPLAY", "WAYLAND_DISPLAY", "DBUS_SESSION_BUS_ADDRESS", "SSH_AUTH_SOCK", "EDITOR", "TZ"]
                    .contains(&k.as_str())
                    || k.starts_with("LC_")
                    || k.starts_with("XDG_")
            })
            .collect();
    }
    for k in ["TESSERA_SOCKET", "XDG_STATE_HOME", "XDG_RUNTIME_DIR"] {
        if let Ok(v) = std::env::var(k) {
            env.retain(|(e, _)| e != k);
            env.push((k.to_string(), v));
        }
    }
    // The account's own login shell (SHELL in a dev shell is its build bash).
    if let Some(shell) = login_shell() {
        env.retain(|(e, _)| e != "SHELL");
        env.push(("SHELL".into(), shell));
    }
    env
}

/// `systemctl --user show-environment` prints KEY=value, quoting values with
/// special characters as $'...'.
fn parse_systemd_env(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            let v = match v.strip_prefix("$'").and_then(|v| v.strip_suffix('\'')) {
                Some(q) => {
                    let mut out = String::new();
                    let mut it = q.chars();
                    while let Some(c) = it.next() {
                        if c != '\\' {
                            out.push(c);
                            continue;
                        }
                        match it.next() {
                            Some('n') => out.push('\n'),
                            Some('t') => out.push('\t'),
                            Some('e') => out.push('\x1b'),
                            Some(o) => out.push(o),
                            None => {}
                        }
                    }
                    out
                }
                None => v.to_string(),
            };
            Some((k.to_string(), v))
        })
        .collect()
}

pub fn login_shell() -> Option<String> {
    #[cfg(windows)]
    {
        // PowerShell 7 when installed, else Windows PowerShell.
        let pwsh = std::env::var_os("PATH")
            .map(|p| std::env::split_paths(&p).map(|d| d.join("pwsh.exe")).any(|f| f.exists()))
            .unwrap_or(false);
        return Some(if pwsh { "pwsh.exe".into() } else { "powershell.exe".into() });
    }
    #[cfg(unix)]
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() || (*pw).pw_shell.is_null() {
            return None;
        }
        let s = std::ffi::CStr::from_ptr((*pw).pw_shell).to_string_lossy().into_owned();
        (!s.is_empty()).then_some(s)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn systemd_env() {
        let v = super::parse_systemd_env("A=1\nB=$'x\\ny \\'q\\''\nPATH=/bin:/usr/bin\n");
        assert_eq!(v[0], ("A".into(), "1".into()));
        assert_eq!(v[1], ("B".into(), "x\ny 'q'".into()));
        assert_eq!(v[2].1, "/bin:/usr/bin");
    }
}
