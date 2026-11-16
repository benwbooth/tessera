//! The local socket between the window and the daemon: a Unix domain socket
//! everywhere (Windows 10 and later have them too, through uds_windows).

#[cfg(unix)]
pub use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(windows)]
pub use uds_windows::{UnixListener, UnixStream};

/// The handle Qt watches for readability (a socket notifier).
pub fn raw(s: &UnixStream) -> i64 {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        s.as_raw_fd() as i64
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawSocket;
        s.as_raw_socket() as i64
    }
}
