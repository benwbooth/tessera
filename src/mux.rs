//! Many connections to tesserad over one stream of text lines: how the
//! window reaches a host's tesserad when its sshd won't forward a Unix
//! socket (Windows' OpenSSH, or `AllowStreamLocalForwarding no`). The window
//! runs `tesserad mux` there over SSH and serves the usual local socket
//! itself; each connection to it becomes one here, to the host's tesserad.
//!
//! Lines, so a shell in between (cmd.exe, PowerShell) can't garble them:
//!   o <id>          a new connection
//!   d <id> <base64> bytes on it
//!   c <id>          it closed
//! Both ways the same.

use crate::net::UnixStream;
use base64::Engine;
use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::sync::{Arc, Mutex};

type Out = Arc<Mutex<Box<dyn Write + Send>>>;

fn send(out: &Out, line: &str) -> bool {
    let mut o = out.lock().unwrap();
    o.write_all(line.as_bytes()).is_ok() && o.write_all(b"\n").is_ok() && o.flush().is_ok()
}

/// Copies one connection's bytes to the line stream until it closes.
fn pump(mut from: impl Read, id: u64, out: Out) {
    let b64 = base64::engine::general_purpose::STANDARD;
    let mut buf = vec![0u8; 32 * 1024];
    loop {
        match from.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if !send(&out, &format!("d {id} {}", b64.encode(&buf[..n]))) {
                    break;
                }
            }
        }
    }
    send(&out, &format!("c {id}"));
}

/// One side of the mux: lines in from `input`, out to `out`; `open`
/// makes the connection for an "o" (the host's side: to its tesserad).
/// Returns when `input` ends.
pub fn serve_lines(input: impl BufRead, out: Out, open: Option<&dyn Fn() -> std::io::Result<UnixStream>>, conns: Arc<Mutex<HashMap<u64, UnixStream>>>) {
    let b64 = base64::engine::general_purpose::STANDARD;
    for line in input.lines() {
        let Ok(line) = line else { break };
        let line = line.trim_end_matches('\r');
        let mut it = line.splitn(3, ' ');
        let (op, id) = (it.next().unwrap_or(""), it.next().and_then(|i| i.parse::<u64>().ok()));
        let Some(id) = id else { continue };
        match op {
            "o" => {
                let Some(open) = open else { continue };
                match open() {
                    Ok(s) => {
                        if let Ok(r) = s.try_clone() {
                            conns.lock().unwrap().insert(id, s);
                            let out = out.clone();
                            std::thread::spawn(move || pump(r, id, out));
                        }
                    }
                    Err(_) => {
                        send(&out, &format!("c {id}"));
                    }
                }
            }
            "d" => {
                let data = b64.decode(it.next().unwrap_or("")).unwrap_or_default();
                let mut gone = false;
                if let Some(s) = conns.lock().unwrap().get_mut(&id) {
                    gone = s.write_all(&data).is_err();
                }
                if gone {
                    conns.lock().unwrap().remove(&id);
                    send(&out, &format!("c {id}"));
                }
            }
            "c" => {
                if let Some(s) = conns.lock().unwrap().remove(&id) {
                    let _ = s.shutdown(std::net::Shutdown::Both);
                }
            }
            _ => {}
        }
    }
    for (_, s) in conns.lock().unwrap().drain() {
        let _ = s.shutdown(std::net::Shutdown::Both);
    }
}

/// `tesserad mux`: the host's side, on stdin and stdout.
pub fn run() -> std::io::Result<()> {
    // (tesserad here, started unless it's running.)
    crate::client::ensure_daemon()?;
    let out: Out = Arc::new(Mutex::new(Box::new(std::io::stdout())));
    send(&out, "ready");
    let open = || UnixStream::connect(crate::proto::socket_path());
    serve_lines(std::io::stdin().lock(), out, Some(&open), Arc::default());
    Ok(())
}

/// The window's side: serves `local` (a Unix socket here), carrying each
/// connection to it over `child`'s stdin and stdout (ssh running `tesserad
/// mux` on the host). Returns once the far side said it's ready; the rest
/// runs in threads until ssh ends, and then the local socket goes.
#[cfg(unix)]
pub fn tunnel(mut child: std::process::Child, local: &std::path::Path) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    let stdin = child.stdin.take().ok_or_else(|| std::io::Error::other("no stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| std::io::Error::other("no stdout"))?;
    let mut reader = std::io::BufReader::new(stdout);
    // (Its first line: tesserad there is up.)
    let mut first = String::new();
    reader.read_line(&mut first)?;
    if first.trim() != "ready" {
        let _ = child.kill();
        return Err(std::io::Error::other(format!("tesserad mux didn't start there: {}", first.trim())));
    }
    let _ = std::fs::remove_file(local);
    let listener = std::os::unix::net::UnixListener::bind(local)?;
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(local, std::fs::Permissions::from_mode(0o600));
    }
    let out: Out = Arc::new(Mutex::new(Box::new(stdin)));
    let conns: Arc<Mutex<HashMap<u64, UnixStream>>> = Arc::default();
    {
        let (out, conns, local) = (out.clone(), conns.clone(), local.to_path_buf());
        std::thread::spawn(move || {
            serve_lines(reader, out, None, conns);
            // (ssh ended: nothing reaches tesserad through here any more.)
            let _ = std::fs::remove_file(&local);
            let _ = child.wait();
        });
    }
    std::thread::spawn(move || {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        for s in listener.incoming() {
            let Ok(s) = s else { break };
            let id = NEXT.fetch_add(1, Ordering::SeqCst);
            let Ok(r) = s.try_clone() else { continue };
            conns.lock().unwrap().insert(id, s);
            if !send(&out, &format!("o {id}")) {
                break;
            }
            let out = out.clone();
            std::thread::spawn(move || pump(r, id, out));
        }
    });
    Ok(())
}
