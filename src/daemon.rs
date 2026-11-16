//! The PTY daemon: owns every shell so they outlive the GUI.
//!
//! Like iTerm2's session server, it keeps shells running when the window
//! closes (or the GUI crashes or is rebuilt). The next GUI start lists the
//! detached sessions and attaches to them again, replaying the recent output
//! so the screen comes back.
//!
//! Runs as `tessera daemon`; the GUI starts it on demand.

use crate::proto::*;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Shared {
    client: Option<UnixStream>,
}

/// Work for a session's emulator thread, which keeps its own copy of the
/// screen (libghostty) so a window attaching later gets it exactly.
enum Emu {
    Output(Vec<u8>),
    Resize(u16, u16),
    Snapshot(mpsc::Sender<Vec<u8>>),
}

struct DSession {
    id: String,
    shell_pid: Option<u32>,
    created: u64,
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
    shared: Mutex<Shared>,
    emu: Mutex<mpsc::Sender<Emu>>,
    /// The PTY's current size: resizing to the same size is skipped (every
    /// resize makes the shell redraw its prompt).
    size: Mutex<(u16, u16)>,
}

type Registry = Arc<Mutex<HashMap<String, Arc<DSession>>>>;

static CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

pub fn run() -> std::io::Result<()> {
    let path = socket_path();
    if path.exists() {
        if UnixStream::connect(&path).is_ok() {
            eprintln!("tessera daemon: already running at {}", path.display());
            return Ok(());
        }
        let _ = std::fs::remove_file(&path);
    }
    let listener = UnixListener::bind(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    eprintln!("tessera daemon: listening at {}", path.display());

    let registry: Registry = Arc::default();

    // Exit once nothing is left: no shells and nobody connected. The next GUI
    // start launches a fresh daemon (handy after a rebuild, too).
    {
        let registry = registry.clone();
        let path = path.clone();
        std::thread::spawn(move || {
            let mut idle = 0;
            loop {
                std::thread::sleep(Duration::from_secs(1));
                let empty = registry.lock().unwrap().is_empty()
                    && CONNECTIONS.load(Ordering::SeqCst) == 0;
                idle = if empty { idle + 1 } else { 0 };
                if idle >= 5 {
                    let _ = std::fs::remove_file(&path);
                    std::process::exit(0);
                }
            }
        });
    }

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let registry = registry.clone();
        CONNECTIONS.fetch_add(1, Ordering::SeqCst);
        std::thread::spawn(move || {
            if let Err(e) = handle_client(stream, &registry) {
                eprintln!("tessera daemon: client: {e}");
            }
            CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
        });
    }
    Ok(())
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn new_id() -> String {
    use std::sync::atomic::AtomicU64;
    static N: AtomicU64 = AtomicU64::new(0);
    format!("{:x}-{}-{}", now(), std::process::id(), N.fetch_add(1, Ordering::SeqCst))
}

fn spawn_session(c: &Create, registry: &Registry) -> std::io::Result<Arc<DSession>> {
    let pty = native_pty_system();
    let pair = pty
        .openpty(PtySize { rows: c.rows.max(1), cols: c.cols.max(1), pixel_width: 0, pixel_height: 0 })
        .map_err(ioe)?;
    let mut cmd = if c.command.trim().is_empty() {
        CommandBuilder::new_default_prog()
    } else {
        // A profile's command, run through the user's shell so PATH and the
        // like are set up as in a normal session.
        let shell = crate::client::login_shell()
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/sh".into());
        let mut b = CommandBuilder::new(shell);
        b.args(["-l", "-c", c.command.as_str()]);
        b
    };
    let cwd = if !c.cwd.is_empty() && std::path::Path::new(&c.cwd).is_dir() {
        c.cwd.clone()
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".into())
    };
    cmd.cwd(cwd);
    cmd.env("TERM", if c.term.is_empty() { "xterm-256color" } else { c.term.as_str() });
    cmd.env("COLORTERM", "truecolor");
    cmd.env("TERM_PROGRAM", "Tessera");
    cmd.env("TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
    let child = pair.slave.spawn_command(cmd).map_err(ioe)?;
    let shell_pid = child.process_id();
    drop(pair.slave);
    let reader = pair.master.try_clone_reader().map_err(ioe)?;
    let mut writer = pair.master.take_writer().map_err(ioe)?;
    if !c.input.is_empty() {
        let _ = writer.write_all(c.input.as_bytes());
    }
    let (emu_tx, emu_rx) = mpsc::channel::<Emu>();
    {
        let (cols, rows, lines) = (c.cols.max(2), c.rows.max(1), if c.scrollback > 0 { c.scrollback } else { 10_000 });
        std::thread::spawn(move || emulate(emu_rx, cols, rows, lines));
    }
    let s = Arc::new(DSession {
        id: new_id(),
        shell_pid,
        created: now(),
        master: Mutex::new(pair.master),
        writer: Mutex::new(writer),
        child: Mutex::new(child),
        shared: Mutex::new(Shared { client: None }),
        emu: Mutex::new(emu_tx),
        size: Mutex::new((c.cols.max(1), c.rows.max(1))),
    });
    registry.lock().unwrap().insert(s.id.clone(), s.clone());

    // Reader: PTY output -> replay ring + attached client.
    {
        let s = s.clone();
        let registry = registry.clone();
        std::thread::spawn(move || pump_output(s, reader, registry));
    }
    Ok(s)
}

fn pump_output(s: Arc<DSession>, mut reader: Box<dyn Read + Send>, registry: Registry) {
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let mut sh = s.shared.lock().unwrap();
        let _ = s.emu.lock().unwrap().send(Emu::Output(buf[..n].to_vec()));
        if let Some(c) = sh.client.as_mut() {
            if write_frame(c, T_OUTPUT, &buf[..n]).is_err() {
                sh.client = None;
            }
        }
    }
    let code = s
        .child
        .lock()
        .unwrap()
        .wait()
        .map(|st| st.exit_code() as i32)
        .unwrap_or(-1);
    registry.lock().unwrap().remove(&s.id);
    let mut sh = s.shared.lock().unwrap();
    if let Some(c) = sh.client.as_mut() {
        let _ = write_json(c, T_EXITED, &code);
        let _ = c.shutdown(std::net::Shutdown::Both);
    }
    sh.client = None;
}

/// The foreground process group's program, when it isn't the shell.
fn foreground_job(s: &DSession) -> String {
    let Some(pgid) = s.master.lock().unwrap().process_group_leader() else { return String::new() };
    if Some(pgid as u32) == s.shell_pid {
        return String::new();
    }
    std::fs::read_to_string(format!("/proc/{pgid}/comm")).map(|c| c.trim().to_string()).unwrap_or_default()
}

/// The emulator thread: libghostty types stay on the thread that made them.
fn emulate(rx: mpsc::Receiver<Emu>, cols: u16, rows: u16, lines: u32) {
    use libghostty_vt::{terminal::Options, Terminal};
    let Ok(mut term) = Terminal::new(Options { cols, rows, max_scrollback: crate::session::scrollback_bytes(lines) }) else { return };
    for msg in rx {
        match msg {
            Emu::Output(b) => term.vt_write(&b),
            Emu::Resize(c, r) => {
                let _ = term.resize(c.max(2), r.max(1), 8, 16);
            }
            Emu::Snapshot(reply) => {
                let _ = reply.send(crate::snapshot::snapshot(&term));
            }
        }
    }
}

fn resize(s: &DSession, cols: u16, rows: u16) {
    {
        let mut size = s.size.lock().unwrap();
        if *size == (cols.max(1), rows.max(1)) {
            return;
        }
        *size = (cols.max(1), rows.max(1));
    }
    let _ = s.emu.lock().unwrap().send(Emu::Resize(cols.max(2), rows.max(1)));
    let _ = s.master.lock().unwrap().resize(PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    });
}

fn handle_client(mut stream: UnixStream, registry: &Registry) -> std::io::Result<()> {
    let (tag, payload) = read_frame(&mut stream)?;
    let reattach = tag == T_ATTACH;
    let session = match tag {
        T_LIST => {
            let list: Vec<SessionInfo> = registry
                .lock()
                .unwrap()
                .values()
                .map(|s| SessionInfo {
                    id: s.id.clone(),
                    attached: s.shared.lock().unwrap().client.is_some(),
                    created: s.created,
                })
                .collect();
            return write_json(&mut stream, T_SESSIONS, &list);
        }
        T_KILL_ID => {
            let id: String = serde_json::from_slice(&payload)?;
            if let Some(s) = registry.lock().unwrap().get(&id).cloned() {
                let _ = s.child.lock().unwrap().kill();
            }
            return Ok(());
        }
        T_JOB => {
            let id: String = serde_json::from_slice(&payload)?;
            let found = registry.lock().unwrap().get(&id).cloned();
            let job = found.map(|s| foreground_job(&s)).unwrap_or_default();
            return write_json(&mut stream, T_JOB_REPLY, &job);
        }
        T_CREATE => {
            let c: Create = serde_json::from_slice(&payload)?;
            spawn_session(&c, registry)?
        }
        T_ATTACH => {
            let a: Attach = serde_json::from_slice(&payload)?;
            let found = registry.lock().unwrap().get(&a.id).cloned();
            let Some(s) = found else {
                return write_json(&mut stream, T_ERROR, &format!("no session {}", a.id));
            };
            resize(&s, a.cols, a.rows);
            s
        }
        _ => return write_json(&mut stream, T_ERROR, &"expected create, attach or list"),
    };

    // Attach: hand over the replay and become the live client, atomically
    // with respect to the output pump so nothing is lost or duplicated.
    {
        let mut sh = session.shared.lock().unwrap();
        if let Some(mut old) = sh.client.take() {
            let _ = write_frame(&mut old, T_TAKEN, b"");
            let _ = old.shutdown(std::net::Shutdown::Both);
        }
        let mut out = stream.try_clone()?;
        write_json(&mut out, T_ATTACHED, &session.id)?;
        // The screen as it is now (already reflowed to this window's size),
        // not a replay of old output. Output can't slip in between: the pump
        // waits on this lock.
        // (Only when reattaching: a new shell starts from a blank screen, and
        // a snapshot would move the cursor over whatever the window shows.)
        if reattach {
            let (tx, rx) = mpsc::channel();
            let _ = session.emu.lock().unwrap().send(Emu::Snapshot(tx));
            if let Ok(snap) = rx.recv_timeout(Duration::from_secs(3)) {
                if !snap.is_empty() {
                    write_frame(&mut out, T_OUTPUT, &snap)?;
                }
            }
        }
        sh.client = Some(out);
    }

    loop {
        let (tag, payload) = match read_frame(&mut stream) {
            Ok(f) => f,
            Err(_) => break, // GUI went away: the session stays, detached.
        };
        match tag {
            T_INPUT => {
                let _ = session.writer.lock().unwrap().write_all(&payload);
            }
            T_RESIZE => {
                if let Ok(r) = serde_json::from_slice::<Resize>(&payload) {
                    resize(&session, r.cols, r.rows);
                }
            }
            T_KILL => {
                let _ = session.child.lock().unwrap().kill();
                break;
            }
            _ => {}
        }
    }
    // Detach only if we're still the attached client.
    let mut sh = session.shared.lock().unwrap();
    if let Some(c) = &sh.client {
        if same_socket(c, &stream) {
            sh.client = None;
        }
    }
    Ok(())
}

fn same_socket(a: &UnixStream, b: &UnixStream) -> bool {
    use std::os::fd::AsRawFd;
    // try_clone gives a new fd for the same socket; compare inodes.
    let ino = |s: &UnixStream| unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        libc::fstat(s.as_raw_fd(), &mut st);
        st.st_ino
    };
    ino(a) == ino(b)
}

fn ioe(e: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(e.to_string())
}
