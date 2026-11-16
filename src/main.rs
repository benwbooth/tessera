#![cfg_attr(windows, windows_subsystem = "windows")]
mod ai;
mod bridge;
mod cells;
mod client;
mod complete;
mod contrast;
mod daemon;
mod find;
mod kde;
mod history;
mod images;
mod keymap;
mod net;
mod procinfo;
mod launchers;
mod lsp;
mod prompt;
mod proto;
mod settings;
mod shell_integration;
mod snapshot;
mod syntax;
mod session;
mod tmux;

use cxx_qt_lib::{QQmlApplicationEngine, QString, QUrl};

fn main() {
    // Windows has no HOME: settings and state live under the user profile
    // (~/.config/tessera and so on), as on the other systems.
    #[cfg(windows)]
    if std::env::var_os("HOME").is_none() {
        if let Some(p) = std::env::var_os("USERPROFILE") {
            std::env::set_var("HOME", p);
        }
    }
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("daemon") {
        if let Err(e) = daemon::run() {
            eprintln!("tessera daemon: {e}");
            std::process::exit(1);
        }
        return;
    }
    // Options for the QML side (screenshots, a first command for testing).
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                if let Some(p) = args.get(i + 1) {
                    let abs = std::env::current_dir().unwrap_or_default().join(p);
                    std::env::set_var("TESSERA_SCREENSHOT", abs);
                }
                i += 1;
            }
            "--new-window" => std::env::set_var("TESSERA_NO_RESTORE", "1"),
            // An elevated window (Windows): its shells live in their own
            // daemon, which only administrators can reach.
            "--admin" => {
                std::env::set_var("TESSERA_NO_RESTORE", "1");
                std::env::set_var("TESSERA_ADMIN", "1");
                let sock = crate::proto::admin_socket_path();
                // Locked down before connecting too, so a daemon some other
                // (unelevated) program put there is never used.
                if let Err(e) = daemon::lock_down_admin_dir(&sock) {
                    eprintln!("tessera: can't prepare the administrator window: {e}");
                    std::process::exit(1);
                }
                std::env::set_var("TESSERA_SOCKET", sock);
            }
            "--find" => {
                if let Some(q) = args.get(i + 1) {
                    std::env::set_var("TESSERA_FIND", q);
                }
                i += 1;
            }
            "--tab" => {
                if let Some(f) = args.get(i + 1) {
                    std::env::set_var("TESSERA_TAB_FILE", f);
                }
                i += 1;
            }
            "--prefs" => {
                if let Some(c) = args.get(i + 1) {
                    std::env::set_var("TESSERA_PREFS", c);
                }
                i += 1;
            }
            "--then" => {
                if let Some(c) = args.get(i + 1) {
                    std::env::set_var("TESSERA_THEN", c);
                }
                i += 1;
            }
            "--run" => {
                if let Some(c) = args.get(i + 1) {
                    std::env::set_var("TESSERA_RUN", c);
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    std::env::set_var("QT_QUICK_CONTROLS_STYLE", std::env::var("TESSERA_STYLE").unwrap_or_else(|_| "Basic".into()));
    // Wayland: the desktop remembers where the window was (KDE Plasma 6.4+,
    // GNOME 48+); the session id it gave last time lets it put it back.
    // Only the first window does this (not New Window / torn-off tabs).
    let main_window = std::env::var_os("TESSERA_NO_RESTORE").is_none() && std::env::var_os("TESSERA_SCREENSHOT").is_none();
    let wayland = main_window && std::env::var_os("WAYLAND_DISPLAY").is_some();
    let session_file = settings::state_dir().join("window-session");
    if wayland && std::env::var_os("QT_WAYLAND_ENABLE_XX_SESSION_MANAGER").is_none() {
        std::env::set_var("QT_WAYLAND_ENABLE_XX_SESSION_MANAGER", "1");
    }
    let session = if wayland { std::fs::read_to_string(&session_file).unwrap_or_default().trim().to_string() } else { String::new() };
    bridge::ffi::tessera_webengine_initialize();
    bridge::ffi::tessera_create_app(&QString::from(session.as_str()));
    if main_window {
        kde::remember_window_position();
    }
    bridge::ffi::tessera_set_icon(&QString::from(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tessera.svg")));
    // A daemon from an older build: swap the new one in (shells kept).
    std::env::set_var("TESSERA_DAEMON_STATE", client::sync_daemon());
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        if let Some(qml) = option_env!("TESSERA_QT_QML") {
            if std::path::Path::new(qml).exists() {
                engine.as_mut().add_import_path(&QString::from(qml));
            }
        }
        engine.load(&QUrl::from("qrc:/qt/qml/Tessera/qml/Main.qml"));
    }
    bridge::ffi::tessera_show_windows();
    let code = bridge::ffi::tessera_exec();
    if wayland {
        // Qt reads "-session id_key", so an id with "_" in it can't be passed back.
        let id = bridge::ffi::tessera_session_id().to_string();
        if !id.is_empty() && !id.contains('_') {
            let _ = std::fs::create_dir_all(settings::state_dir());
            let _ = std::fs::write(&session_file, id);
        }
    }
    std::process::exit(code);
}
