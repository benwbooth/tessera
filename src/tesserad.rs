//! tesserad: Tessera's backend, the part that keeps running when the window
//! closes, as tmux's server does. It owns the shells (PTYs), the notebook
//! kernels (Jupyter's and Tessera's own shell kernels), each session's cells
//! and notebook, Nix runs and command lookups. The window is a client of it,
//! locally over a Unix socket and on another host through SSH (the window
//! forwards a local socket to the remote tesserad's).
//!
//! No Qt: it builds as a static binary (musl) that can be copied to any
//! Linux host.
//!
//! Usage:
//!   tesserad                 serve (start detached with `tesserad start`)
//!   tesserad start           start one in the background unless running;
//!                            prints the socket's path
//!   tesserad --socket PATH   serve or start at PATH instead of the default
//!   tesserad connect HOST    reach HOST's tesserad over SSH (started, or
//!                            copied there, when missing); prints the local
//!                            socket forwarded to it
//!   tesserad version         this build's version
#![allow(dead_code)]

mod ai;
mod cells;
mod client;
mod complete;
mod daemon;
mod files;
mod find;
mod grammars;
mod history;
mod images;
mod jupyter;
mod launchers;
mod lsp;
mod net;
mod nixcmds;
mod procinfo;
mod prompt;
mod prompt_widgets;
mod proto;
mod remote;
mod rpc;
mod settings;
mod shell_integration;
mod shellk;
mod snapshot;
mod snippets;
mod syntax;
mod tables;

fn main() {
    #[cfg(windows)]
    if std::env::var_os("HOME").is_none() {
        if let Some(p) = std::env::var_os("USERPROFILE") {
            std::env::set_var("HOME", p);
        }
    }
    let mut cmd = String::new();
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            // Which socket to serve (or start).
            "--socket" => {
                if let Some(p) = it.next() {
                    std::env::set_var("TESSERA_SOCKET", p);
                }
            }
            // (An older `tessera daemon` upgrading in place execs
            // "<exe> daemon --resume STATE": daemon::run reads it.)
            "--resume" => {
                it.next();
            }
            _ if cmd.is_empty() && !a.starts_with('-') => cmd = a,
            _ => {}
        }
    }
    match cmd.as_str() {
        "version" | "--version" => println!("tesserad {} {}", env!("CARGO_PKG_VERSION"), proto::exe_identity()),
        // Set up the window's way to HOST's tesserad (SSH; deployed there
        // when missing) and print the local socket that reaches it.
        "connect" => {
            let host = std::env::args().skip_while(|a| a != "connect").nth(1).unwrap_or_default();
            match remote::connect(&host) {
                Ok(_) => println!("{}", remote::local_socket(&host).display()),
                Err(e) => {
                    eprintln!("tesserad: {host}: {e}");
                    std::process::exit(1);
                }
            }
        }
        "start" => match client::ensure_daemon() {
            Ok(()) => println!("{}", proto::socket_path().display()),
            Err(e) => {
                eprintln!("tesserad: {e}");
                std::process::exit(1);
            }
        },
        "" | "daemon" | "serve" => {
            if let Err(e) = daemon::run() {
                eprintln!("tesserad: {e}");
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("tesserad: unknown command {other:?} (start, version, or none to serve)");
            std::process::exit(2);
        }
    }
}
