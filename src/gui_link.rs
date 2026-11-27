//! Running a command in the Tessera window that's already open:
//! `tessera -e CMD…` (the terminals' usual option; file managers' "Run in
//! Terminal" use it, through the desktop file's X-TerminalArgExec) opens a
//! new tab in the window used last, which comes to the front, instead of a
//! second window. Each window listens on its own socket,
//! $XDG_RUNTIME_DIR/tessera-gui/<pid>.sock, whose time is touched whenever
//! the window is activated: the newest socket is the window used last.
//! Nothing listening (no window, or `--new-window`): a new window starts
//! with the command in its first tab.

use serde_json::json;
use std::path::PathBuf;

/// A command for a new tab: what to run, where, and whether the tab stays
/// (its shell) once the command ends (`--hold`).
#[derive(Clone, Debug, Default)]
pub struct Exec {
    pub cmd: String,
    pub cwd: String,
    pub hold: bool,
    /// The launcher's xdg-activation token (XDG_ACTIVATION_TOKEN, as arbor
    /// and other compositors hand to what they start): with it, the window
    /// that takes the command may come to the front although it isn't the
    /// focused one (without it, Wayland only marks it as wanting attention).
    pub token: String,
}

impl Exec {
    /// From `-e`'s words: the tab's program. Held: after it, its exit status
    /// and then the user's own shell, there (whatever that shell's syntax,
    /// the command runs through sh with its words as they were given).
    pub fn new(argv: &[String], cwd: String, hold: bool) -> Exec {
        let cmd = if hold {
            let script = r#""$@"; s=$?; printf '\n[%s ended with status %s]\n' "$1" "$s"; exec "${SHELL:-/bin/sh}""#;
            let mut v = vec!["sh".to_string(), "-c".into(), script.into(), "sh".into()];
            v.extend(argv.iter().cloned());
            shell_words::join(v)
        } else if argv.len() == 1 && argv[0].contains(char::is_whitespace) {
            // `-e "ls -l"`: one word holding a whole command line.
            shell_words::join(["sh", "-c", argv[0].as_str()])
        } else {
            shell_words::join(argv)
        };
        Exec { cmd, cwd, hold, token: launch_token() }
    }
    pub fn to_json(&self) -> String {
        json!({"cmd": self.cmd, "cwd": self.cwd, "hold": self.hold, "token": self.token}).to_string()
    }
}

/// The activation token this launch was given, if any.
pub fn launch_token() -> String {
    std::env::var("XDG_ACTIVATION_TOKEN").unwrap_or_default()
}

fn dir() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("tessera-gui")
}

fn own_socket() -> PathBuf {
    dir().join(format!("{}.sock", std::process::id()))
}

/// Hand `e` to the window used last. False when no window took it.
#[cfg(unix)]
pub fn send(e: &Exec) -> bool {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    let Ok(rd) = std::fs::read_dir(dir()) else { return false };
    let mut socks: Vec<(std::time::SystemTime, PathBuf)> = rd
        .flatten()
        .filter(|f| f.path().extension().is_some_and(|x| x == "sock"))
        .filter_map(|f| Some((f.metadata().ok()?.modified().ok()?, f.path())))
        .collect();
    socks.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, p) in socks {
        let Ok(mut s) = UnixStream::connect(&p) else {
            // A window that's gone (crashed): its socket goes too.
            let _ = std::fs::remove_file(&p);
            continue;
        };
        let _ = s.set_read_timeout(Some(std::time::Duration::from_secs(3)));
        if writeln!(s, "{}", e.to_json()).is_err() {
            continue;
        }
        let mut answer = String::new();
        if BufReader::new(&s).read_line(&mut answer).is_ok() && answer.trim() == "ok" {
            return true;
        }
    }
    false
}

#[cfg(not(unix))]
pub fn send(_e: &Exec) -> bool {
    false
}

/// This window takes commands from other launches: each arrives as JSON
/// ({cmd, cwd, hold, token}) through `on`.
#[cfg(unix)]
pub fn listen(on: impl Fn(String) + Send + 'static) {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixListener;
    let d = dir();
    if std::fs::create_dir_all(&d).is_err() {
        return;
    }
    let _ = std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700));
    let path = own_socket();
    let _ = std::fs::remove_file(&path);
    let Ok(listener) = UnixListener::bind(&path) else { return };
    let _ = std::thread::Builder::new().name("tessera-gui-link".into()).spawn(move || {
        for conn in listener.incoming().flatten() {
            let mut line = String::new();
            if BufReader::new(&conn).read_line(&mut line).is_err() || line.trim().is_empty() {
                continue;
            }
            on(line.trim().to_string());
            let mut w = &conn;
            let _ = w.write_all(b"ok\n");
        }
    });
}

#[cfg(not(unix))]
pub fn listen(_on: impl Fn(String) + Send + 'static) {}

/// This window was just used: the next `tessera -e` comes here.
#[cfg(unix)]
pub fn touch() {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(own_socket().as_os_str().as_bytes()) else { return };
    unsafe {
        libc::utimensat(libc::AT_FDCWD, c.as_ptr(), std::ptr::null(), 0);
    }
}

#[cfg(not(unix))]
pub fn touch() {}

/// The window closes: its socket goes.
pub fn remove() {
    let _ = std::fs::remove_file(own_socket());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn commands() {
        let w = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(Exec::new(&w(&["ls", "-l", "a b"]), String::new(), false).cmd, "ls -l 'a b'");
        assert_eq!(Exec::new(&w(&["ls -l"]), String::new(), false).cmd, "sh -c 'ls -l'");
        let held = Exec::new(&w(&["./run.sh"]), String::new(), true).cmd;
        assert!(held.starts_with("sh -c ") && held.ends_with(" sh ./run.sh"), "{held}");
    }
}
