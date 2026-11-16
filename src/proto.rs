//! The wire protocol between the GUI and the PTY daemon.
//!
//! Every message is a frame: `u32` little-endian length (of tag + payload),
//! a one-byte tag, then the payload. Terminal bytes travel raw; the few
//! control messages carry JSON.
//!
//! One connection = one attachment to one session (or a one-shot `List`).

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::path::PathBuf;

pub const T_CREATE: u8 = 1;
pub const T_ATTACH: u8 = 2;
pub const T_INPUT: u8 = 3;
pub const T_RESIZE: u8 = 4;
pub const T_KILL: u8 = 5;
pub const T_LIST: u8 = 6;
pub const T_JOB: u8 = 7;
/// End a session by id (Undo Close's grace period ran out).
pub const T_KILL_ID: u8 = 8;
/// What a session is doing: {job, cwd, shell} (T_INFO_REPLY).
pub const T_INFO: u8 = 9;

pub const T_ATTACHED: u8 = 101;
pub const T_OUTPUT: u8 = 102;
pub const T_EXITED: u8 = 103;
pub const T_SESSIONS: u8 = 104;
pub const T_ERROR: u8 = 105;
pub const T_JOB_REPLY: u8 = 106;
pub const T_INFO_REPLY: u8 = 108;
/// Another window attached the session: this client is no longer it.
pub const T_TAKEN: u8 = 107;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Create {
    pub cols: u16,
    pub rows: u16,
    pub cwd: String,
    /// Empty: the login shell. Otherwise run through the user's shell.
    #[serde(default)]
    pub command: String,
    /// TERM for the session (default xterm-256color).
    #[serde(default)]
    pub term: String,
    /// Typed into the new shell right away (a profile's "Send text at start").
    #[serde(default)]
    pub input: String,
    /// Scrollback lines the daemon's copy of the screen keeps.
    #[serde(default)]
    pub scrollback: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attach {
    pub id: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Resize {
    pub cols: u16,
    pub rows: u16,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SessionInfo {
    pub id: String,
    pub attached: bool,
    pub created: u64,
}

pub fn write_frame(w: &mut impl Write, tag: u8, payload: &[u8]) -> io::Result<()> {
    let len = (payload.len() + 1) as u32;
    let mut buf = Vec::with_capacity(payload.len() + 5);
    buf.extend_from_slice(&len.to_le_bytes());
    buf.push(tag);
    buf.extend_from_slice(payload);
    w.write_all(&buf)
}

pub fn write_json(w: &mut impl Write, tag: u8, v: &impl Serialize) -> io::Result<()> {
    write_frame(w, tag, &serde_json::to_vec(v).unwrap())
}

/// Blocking read of one frame.
pub fn read_frame(r: &mut impl Read) -> io::Result<(u8, Vec<u8>)> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len)?;
    let len = u32::from_le_bytes(len) as usize;
    if len == 0 || len > 64 << 20 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "bad frame length"));
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    let tag = buf[0];
    buf.remove(0);
    Ok((tag, buf))
}

/// Incremental frame decoder for the GUI's non-blocking socket.
#[derive(Default)]
pub struct FrameReader {
    buf: Vec<u8>,
}

impl FrameReader {
    pub fn push(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    pub fn next(&mut self) -> Option<(u8, Vec<u8>)> {
        if self.buf.len() < 5 {
            return None;
        }
        let len = u32::from_le_bytes(self.buf[..4].try_into().unwrap()) as usize;
        if self.buf.len() < 4 + len {
            return None;
        }
        let tag = self.buf[4];
        let payload = self.buf[5..4 + len].to_vec();
        self.buf.drain(..4 + len);
        Some((tag, payload))
    }
}

/// The administrator window's daemon socket, in a folder only
/// administrators can open (see `daemon::lock_down_admin_dir`).
pub fn admin_socket_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("Tessera").join("admin").join("daemon.sock")
}

pub fn socket_path() -> PathBuf {
    if let Some(p) = std::env::var_os("TESSERA_SOCKET") {
        return PathBuf::from(p);
    }
    #[cfg(unix)]
    {
        if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
            return PathBuf::from(dir).join("tessera-daemon.sock");
        }
        let uid = unsafe { libc::getuid() };
        PathBuf::from(format!("/tmp/tessera-{uid}.sock"))
    }
    #[cfg(not(unix))]
    {
        let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("Tessera");
        let _ = std::fs::create_dir_all(&base);
        base.join("daemon.sock")
    }
}
