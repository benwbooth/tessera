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
