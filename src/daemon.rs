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
use crate::net::{UnixListener, UnixStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct Shared {
    client: Option<UnixStream>,
    /// Which connection `client` is (so a detaching one only clears itself).
    client_gen: u64,
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
    /// Scrollback lines its emulator keeps (for a resumed one after an upgrade).
    lines: u32,
}

type Registry = Arc<Mutex<HashMap<String, Arc<DSession>>>>;

/// The elevated daemon's folder: owned by Administrators and open only to
/// them and SYSTEM, so a program running without elevation can't connect and
/// drive the administrator shells. Fails (and the daemon doesn't start) when
/// that can't be done, e.g. without elevation.
pub fn lock_down_admin_dir(sock: &std::path::Path) -> std::io::Result<()> {
    let dir = sock.parent().ok_or_else(|| std::io::Error::other("no folder"))?;
    std::fs::create_dir_all(dir)?;
    #[cfg(windows)]
    {
        let icacls = |args: &[&str]| -> std::io::Result<()> {
            use std::os::windows::process::CommandExt;
            let ok = std::process::Command::new("icacls")
                .arg(dir)
                .args(args)
                .creation_flags(0x0800_0000)
                .status()?
                .success();
            if ok { Ok(()) } else { Err(std::io::Error::other(format!("icacls {args:?} failed"))) }
        };
        // S-1-5-32-544: Administrators; S-1-5-18: SYSTEM.
        icacls(&["/setowner", "*S-1-5-32-544"])?;
        icacls(&["/inheritance:r", "/grant:r", "*S-1-5-32-544:(OI)(CI)F", "*S-1-5-18:(OI)(CI)F"])?;
        let _ = std::fs::remove_file(sock);
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = sock;
        Err(std::io::Error::other("administrator windows are a Windows feature"))
    }
}

static CONNECTIONS: AtomicUsize = AtomicUsize::new(0);

/// This daemon's build (see proto::exe_identity), fixed at startup.
static IDENTITY: std::sync::OnceLock<String> = std::sync::OnceLock::new();
/// The listening socket's fd (handed to the next build on an upgrade).
#[cfg(unix)]
static LISTENER_FD: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

pub fn run() -> std::io::Result<()> {
    IDENTITY.get_or_init(exe_identity);
    let path = socket_path();
    // Started by the previous build's upgrade: take over its sockets and shells.
    #[cfg(unix)]
    if let Some(state) = std::env::args().skip_while(|a| a != "--resume").nth(1) {
        let registry: Registry = Arc::default();
        let listener = upgrade::resume(&state, &registry)?;
        return serve(listener, registry, path);
    }
    if std::env::var_os("TESSERA_ADMIN").is_some() {
        lock_down_admin_dir(&path)?;
    }
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
    serve(listener, Arc::default(), path)
}

fn serve(listener: UnixListener, registry: Registry, path: std::path::PathBuf) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        LISTENER_FD.store(listener.as_raw_fd(), Ordering::SeqCst);
    }

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
        let mut b = CommandBuilder::new(&shell);
        if cfg!(windows) {
            b.args(["/C", c.command.as_str()]);
        } else {
            b.args(["-l", "-c", c.command.as_str()]);
        }
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
    crate::shell_integration::inject(&mut cmd);
    // In a Flatpak the shell runs on the host, not in the sandbox.
    if std::env::var_os("FLATPAK_ID").is_some() {
        cmd = host_command(&cmd);
    }
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
        shared: Mutex::new(Shared { client: None, client_gen: 0 }),
        emu: Mutex::new(emu_tx),
        size: Mutex::new((c.cols.max(1), c.rows.max(1))),
        lines: if c.scrollback > 0 { c.scrollback } else { 10_000 },
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
/// The running program (empty at a prompt), the folder it's in (the
/// program's, else the shell's) and the shell's name.
fn proc_info(s: &DSession) -> crate::client::ProcInfo {
    let comm = crate::procinfo::name;
    let cwd = crate::procinfo::cwd;
    let pgid = foreground_pgid(s);
    let job = foreground_job(s);
    let mut dir = pgid.map(cwd).unwrap_or_default();
    if dir.is_empty() {
        dir = s.shell_pid.map(cwd).unwrap_or_default();
    }
    // The shell is whatever runs at the prompt (a login bash may exec zsh).
    let shell = if job.is_empty() { pgid.or(s.shell_pid).map(comm).unwrap_or_default() } else { s.shell_pid.map(comm).unwrap_or_default() };
    // (A shell started in it, switched to from the editor, is the shell now:
    // its prompt, its environment. Under nix shell: the shell nix started.)
    let job_shell = pgid.filter(|_| !job.is_empty()).map(crate::procinfo::shell_name).unwrap_or_default();
    let nested = if is_shell(&job_shell) {
        pgid.map(|p| (job_shell.clone(), p))
    } else if job == "nix" {
        pgid.and_then(|p| std::fs::read_to_string(format!("/proc/{p}/task/{p}/children")).ok()).and_then(|c| {
            c.split_whitespace().filter_map(|x| x.parse::<u32>().ok()).map(|c| (crate::procinfo::shell_name(c), c)).find(|(n, _)| is_shell(n))
        })
    } else {
        None
    };
    if let Some((name, pid)) = nested {
        let dir = cwd(pid);
        return crate::client::ProcInfo { job: String::new(), cwd: if dir.is_empty() { s.shell_pid.map(cwd).unwrap_or_default() } else { dir }, shell: name, pid };
    }
    crate::client::ProcInfo { job, cwd: dir, shell, pid: s.shell_pid.unwrap_or(0) }
}

fn is_shell(name: &str) -> bool {
    crate::procinfo::is_shell(name)
}

fn foreground_job(s: &DSession) -> String {
    let Some(pgid) = foreground_pgid(s) else { return String::new() };
    if Some(pgid) == s.shell_pid {
        return String::new();
    }
    crate::procinfo::name(pgid)
}

/// `cmd` run on the host through flatpak-spawn (its environment, folder
/// and arguments carried over; the default program is the user's login shell
/// there).
fn host_command(cmd: &CommandBuilder) -> CommandBuilder {
    let mut h = CommandBuilder::new("flatpak-spawn");
    h.arg("--host");
    h.arg("--watch-bus");
    if let Some(dir) = cmd.get_cwd() {
        h.arg(format!("--directory={}", dir.to_string_lossy()));
        h.cwd(dir);
    }
    for (k, v) in cmd.iter_extra_env_as_str() {
        // (The sandbox's data folders mean nothing on the host.)
        if k == "XDG_DATA_DIRS" {
            continue;
        }
        h.arg(format!("--env={k}={v}"));
    }
    if cmd.is_default_prog() {
        h.args(["sh", "-c", "exec \"$(getent passwd \"$(id -un)\" | cut -d: -f7)\" -l"]);
    } else {
        for a in cmd.get_argv() {
            h.arg(a);
        }
    }
    h
}

/// The terminal's foreground process group (none on Windows).
fn foreground_pgid(s: &DSession) -> Option<u32> {
    #[cfg(unix)]
    {
        s.master.lock().unwrap().process_group_leader().map(|p| p as u32)
    }
    #[cfg(not(unix))]
    {
        let _ = s;
        None
    }
}

/// The emulator thread: libghostty types stay on the thread that made them.
fn emulate(rx: mpsc::Receiver<Emu>, cols: u16, rows: u16, lines: u32) {
    use libghostty_vt::{terminal::Options, Terminal};
    let Ok(mut term) = Terminal::new(Options { cols, rows, max_scrollback: crate::session::scrollback_bytes(lines) }) else { return };
    crate::images::enable(&mut term);
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
        T_VERSION => {
            return write_json(&mut stream, T_VERSION_REPLY, IDENTITY.get_or_init(exe_identity));
        }
        T_UPGRADE => {
            #[derive(serde::Deserialize)]
            struct Up {
                exe: String,
            }
            let up: Up = serde_json::from_slice(&payload)?;
            #[cfg(unix)]
            {
                write_json(&mut stream, T_UPGRADE_REPLY, &"upgrading")?;
                drop(stream);
                let err = upgrade::exec_new_build(registry, &up.exe);
                eprintln!("tessera daemon: upgrade failed: {err}");
                return Ok(());
            }
            #[cfg(not(unix))]
            {
                let _ = up;
                return write_json(&mut stream, T_ERROR, &"upgrading a running daemon needs Unix");
            }
        }
        T_INFO => {
            let id: String = serde_json::from_slice(&payload)?;
            let found = registry.lock().unwrap().get(&id).cloned();
            let info = found.map(|s| proc_info(&s)).unwrap_or_default();
            return write_json(&mut stream, T_INFO_REPLY, &info);
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

    let my_gen = GEN.fetch_add(1, Ordering::SeqCst);
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
        sh.client_gen = my_gen;
    }
    attached_loop(&session, stream, my_gen);
    Ok(())
}

static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// An attached window's input (keys, resizes) until it goes away.
fn attached_loop(session: &Arc<DSession>, mut stream: UnixStream, my_gen: u64) {
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
    if sh.client.is_some() && sh.client_gen == my_gen {
        sh.client = None;
    }
}


fn ioe(e: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

/// Upgrading a running daemon to a new build without ending any shell: the
/// old one hands its listening socket, every PTY and every attached window's
/// connection to the new executable across exec (same process, so the shells
/// stay its children), with each screen as a snapshot for the new emulators.
#[cfg(unix)]
mod upgrade {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::os::fd::{AsRawFd, FromRawFd, RawFd};

    #[derive(Serialize, Deserialize)]
    struct SessionState {
        id: String,
        fd: RawFd,
        pid: Option<u32>,
        created: u64,
        cols: u16,
        rows: u16,
        lines: u32,
        snapshot: String,
        client_fd: Option<RawFd>,
    }

    #[derive(Serialize, Deserialize)]
    struct State {
        listener_fd: RawFd,
        sessions: Vec<SessionState>,
    }

    fn set_cloexec(fd: RawFd, on: bool) {
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFD);
            if flags >= 0 {
                let f = if on { flags | libc::FD_CLOEXEC } else { flags & !libc::FD_CLOEXEC };
                libc::fcntl(fd, libc::F_SETFD, f);
            }
        }
    }

    /// Hands everything to `exe` and execs it. Returns only on failure.
    pub fn exec_new_build(registry: &Registry, exe: &str) -> String {
        use std::os::unix::process::CommandExt;
        let dir = crate::settings::state_dir().join("upgrade");
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return e.to_string();
        }
        let reg = registry.lock().unwrap();
        let sessions: Vec<Arc<DSession>> = reg.values().cloned().collect();
        // Every session's lock stays held to the end: no output is forwarded
        // (or lost) in between; it waits in the PTY for the new build.
        let mut guards = Vec::new();
        let mut states = Vec::new();
        for s in &sessions {
            let g = s.shared.lock().unwrap();
            let (tx, rx) = mpsc::channel();
            let _ = s.emu.lock().unwrap().send(Emu::Snapshot(tx));
            let snap = rx.recv_timeout(Duration::from_secs(3)).unwrap_or_default();
            let snap_path = dir.join(format!("{}.vt", s.id));
            if let Err(e) = std::fs::write(&snap_path, &snap) {
                return e.to_string();
            }
            let Some(fd) = s.master.lock().unwrap().as_raw_fd() else { return "a PTY without an fd".into() };
            set_cloexec(fd, false);
            let client_fd = g.client.as_ref().map(|c| c.as_raw_fd());
            if let Some(c) = client_fd {
                set_cloexec(c, false);
            }
            let (cols, rows) = *s.size.lock().unwrap();
            states.push(SessionState {
                id: s.id.clone(),
                fd,
                pid: s.shell_pid,
                created: s.created,
                cols,
                rows,
                lines: s.lines,
                snapshot: snap_path.display().to_string(),
                client_fd,
            });
            guards.push(g);
        }
        let listener_fd = LISTENER_FD.load(Ordering::SeqCst);
        set_cloexec(listener_fd, false);
        let state_path = dir.join("state.json");
        let state = State { listener_fd, sessions: states };
        if let Err(e) = std::fs::write(&state_path, serde_json::to_vec(&state).unwrap_or_default()) {
            return e.to_string();
        }
        eprintln!("tessera daemon: upgrading to {exe} with {} sessions", state.sessions.len());
        let err = std::process::Command::new(exe).arg("daemon").arg("--resume").arg(&state_path).exec();
        // Still here: the old build carries on.
        set_cloexec(listener_fd, true);
        for s in &state.sessions {
            set_cloexec(s.fd, true);
            if let Some(c) = s.client_fd {
                set_cloexec(c, true);
            }
        }
        drop(guards);
        err.to_string()
    }

    /// The new build's side: everything the old one handed over.
    pub fn resume(state_path: &str, registry: &Registry) -> std::io::Result<UnixListener> {
        let state: State = serde_json::from_slice(&std::fs::read(state_path)?).map_err(ioe)?;
        let _ = std::fs::remove_file(state_path);
        set_cloexec(state.listener_fd, true);
        let listener = unsafe { UnixListener::from_raw_fd(state.listener_fd) };
        for st in state.sessions {
            set_cloexec(st.fd, true);
            let master = FdMaster { fd: st.fd };
            let reader = master.try_clone_reader().map_err(ioe)?;
            let writer = master.take_writer().map_err(ioe)?;
            let (emu_tx, emu_rx) = mpsc::channel::<Emu>();
            let (cols, rows, lines) = (st.cols.max(2), st.rows.max(1), st.lines);
            std::thread::spawn(move || emulate(emu_rx, cols, rows, lines));
            if let Ok(snap) = std::fs::read(&st.snapshot) {
                let _ = emu_tx.send(Emu::Output(snap));
            }
            let _ = std::fs::remove_file(&st.snapshot);
            let my_gen = GEN.fetch_add(1, Ordering::SeqCst);
            let client = st.client_fd.map(|c| {
                set_cloexec(c, true);
                unsafe { UnixStream::from_raw_fd(c) }
            });
            let s = Arc::new(DSession {
                id: st.id.clone(),
                shell_pid: st.pid,
                created: st.created,
                master: Mutex::new(Box::new(master)),
                writer: Mutex::new(writer),
                child: Mutex::new(Box::new(PidChild { pid: st.pid.unwrap_or(0) })),
                shared: Mutex::new(Shared { client: None, client_gen: my_gen }),
                emu: Mutex::new(emu_tx),
                size: Mutex::new((st.cols.max(1), st.rows.max(1))),
                lines: st.lines,
            });
            registry.lock().unwrap().insert(s.id.clone(), s.clone());
            // The window that was attached stays attached.
            if let Some(c) = client {
                if let Ok(input) = c.try_clone() {
                    s.shared.lock().unwrap().client = Some(c);
                    let s2 = s.clone();
                    CONNECTIONS.fetch_add(1, Ordering::SeqCst);
                    std::thread::spawn(move || {
                        attached_loop(&s2, input, my_gen);
                        CONNECTIONS.fetch_sub(1, Ordering::SeqCst);
                    });
                }
            }
            let registry = registry.clone();
            std::thread::spawn(move || pump_output(s, reader, registry));
        }
        eprintln!("tessera daemon: resumed after an upgrade");
        Ok(listener)
    }

    /// A PTY master known only by its fd (after exec there's no
    /// portable-pty object for it).
    struct FdMaster {
        fd: RawFd,
    }

    fn dup_file(fd: RawFd) -> anyhow::Result<std::fs::File> {
        let d = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 0) };
        if d < 0 {
            anyhow::bail!(std::io::Error::last_os_error());
        }
        Ok(unsafe { std::fs::File::from_raw_fd(d) })
    }

    impl MasterPty for FdMaster {
        fn resize(&self, size: PtySize) -> anyhow::Result<()> {
            let ws = libc::winsize { ws_row: size.rows, ws_col: size.cols, ws_xpixel: size.pixel_width, ws_ypixel: size.pixel_height };
            if unsafe { libc::ioctl(self.fd, libc::TIOCSWINSZ, &ws) } != 0 {
                anyhow::bail!(std::io::Error::last_os_error());
            }
            Ok(())
        }
        fn get_size(&self) -> anyhow::Result<PtySize> {
            let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
            if unsafe { libc::ioctl(self.fd, libc::TIOCGWINSZ, &mut ws) } != 0 {
                anyhow::bail!(std::io::Error::last_os_error());
            }
            Ok(PtySize { rows: ws.ws_row, cols: ws.ws_col, pixel_width: ws.ws_xpixel, pixel_height: ws.ws_ypixel })
        }
        fn try_clone_reader(&self) -> anyhow::Result<Box<dyn Read + Send>> {
            Ok(Box::new(dup_file(self.fd)?))
        }
        fn take_writer(&self) -> anyhow::Result<Box<dyn Write + Send>> {
            Ok(Box::new(dup_file(self.fd)?))
        }
        fn process_group_leader(&self) -> Option<libc::pid_t> {
            let p = unsafe { libc::tcgetpgrp(self.fd) };
            (p > 0).then_some(p)
        }
        fn as_raw_fd(&self) -> Option<RawFd> {
            Some(self.fd)
        }
        fn tty_name(&self) -> Option<std::path::PathBuf> {
            None
        }
    }

    /// A shell this process started (still its child after exec).
    #[derive(Debug, Clone)]
    struct PidChild {
        pid: u32,
    }

    impl portable_pty::ChildKiller for PidChild {
        fn kill(&mut self) -> std::io::Result<()> {
            if self.pid > 0 {
                unsafe { libc::kill(self.pid as i32, libc::SIGHUP) };
            }
            Ok(())
        }
        fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
            Box::new(self.clone())
        }
    }

    fn status(st: libc::c_int) -> portable_pty::ExitStatus {
        if libc::WIFEXITED(st) {
            portable_pty::ExitStatus::with_exit_code(libc::WEXITSTATUS(st) as u32)
        } else {
            portable_pty::ExitStatus::with_signal(&format!("signal {}", libc::WTERMSIG(st)))
        }
    }

    impl portable_pty::Child for PidChild {
        fn try_wait(&mut self) -> std::io::Result<Option<portable_pty::ExitStatus>> {
            let mut st = 0;
            let r = unsafe { libc::waitpid(self.pid as i32, &mut st, libc::WNOHANG) };
            if r < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok((r > 0).then(|| status(st)))
        }
        fn wait(&mut self) -> std::io::Result<portable_pty::ExitStatus> {
            let mut st = 0;
            loop {
                let r = unsafe { libc::waitpid(self.pid as i32, &mut st, 0) };
                if r >= 0 {
                    return Ok(status(st));
                }
                let e = std::io::Error::last_os_error();
                if e.kind() != std::io::ErrorKind::Interrupted {
                    return Err(e);
                }
            }
        }
        fn process_id(&self) -> Option<u32> {
            Some(self.pid)
        }
    }
}
