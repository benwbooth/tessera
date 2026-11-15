//! Connecting to the PTY daemon, starting it when it isn't running.

use crate::proto::*;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

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
    use std::os::unix::process::CommandExt;
    let exe = std::env::current_exe()?;
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
    cmd.env_clear().envs(session_env());
    cmd.arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(stderr);
    // Its own session, so it survives the GUI and its terminal.
    unsafe {
        cmd.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
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

pub fn list_sessions() -> std::io::Result<Vec<SessionInfo>> {
    // Don't start a daemon just to learn there's nothing to restore.
    let mut s = UnixStream::connect(socket_path())?;
    write_frame(&mut s, T_LIST, b"")?;
    let (tag, payload) = read_frame(&mut s)?;
    if tag != T_SESSIONS {
        return Ok(vec![]);
    }
    Ok(serde_json::from_slice(&payload)?)
}

/// The foreground program of a session when it isn't the shell ("" if idle).
pub fn session_job(id: &str) -> std::io::Result<String> {
    let mut s = UnixStream::connect(socket_path())?;
    s.set_read_timeout(Some(Duration::from_millis(500)))?;
    write_json(&mut s, T_JOB, &id)?;
    let (tag, payload) = read_frame(&mut s)?;
    if tag != T_JOB_REPLY {
        return Ok(String::new());
    }
    Ok(serde_json::from_slice(&payload).unwrap_or_default())
}

/// The desktop session's environment: systemd's user manager has it (what
/// apps started from the desktop get). Falls back to a cleaned-up copy of our
/// own. Tessera's own settings (TESSERA_SOCKET, XDG_STATE_HOME) are kept.
fn session_env() -> Vec<(String, String)> {
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
