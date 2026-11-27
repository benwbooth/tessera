#![cfg_attr(windows, windows_subsystem = "windows")]
mod ai;
mod background;
mod bridge;
mod cells;
mod chats;
mod client;
mod cmdline;
mod complete;
mod manopts;
mod carapace;
mod contrast;
mod daemon;
mod detect;
mod files;
mod find;
mod kde;
mod grammars;
mod gui_link;
mod guides;
mod history;
mod images;
mod image_places;
mod jobs;
mod jupyter;
mod keymap;
mod net;
mod nixcmds;
mod procinfo;
mod launchers;
mod lsp;
mod manual;
mod prompt;
mod prompt_widgets;
mod proto;
mod record;
mod mux;
mod remote;
mod rpc;
mod settings;
mod shell_integration;
mod replk;
mod shellk;
mod snapshot;
mod syntax;
mod tables;
mod session;
mod snippets;

use cxx_qt_lib::{QQmlApplicationEngine, QString, QUrl};

fn main() {
    // (As a child trying a grammar: that, and nothing else.)
    grammars::check_from_args();
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
    // tessh's guides as pages, as the Guide tab shows them, for the
    // documentation site (scripts/tessh-docs-site.sh): no window.
    if args.get(1).map(String::as_str) == Some("--guide-html") {
        let (Some(lang), Some(out)) = (args.get(2), args.get(3)) else {
            eprintln!("usage: tessera --guide-html tessh|tessh-advanced OUT.html");
            std::process::exit(2);
        };
        match guides::page_html(lang, false).and_then(|body| std::fs::write(out, body).map_err(|e| e.to_string())) {
            Ok(()) => return,
            Err(e) => {
                eprintln!("tessera --guide-html: {e}");
                std::process::exit(1);
            }
        }
    }
    // Options for the QML side (screenshots, a first command for testing).
    // A command to run (`-e CMD…`, the terminals' usual option, or `-- CMD…`),
    // where (`--working-directory`), and whether its tab stays (`--hold`).
    let mut exec_argv: Vec<String> = vec![];
    let mut exec_dir: Option<String> = None;
    let mut hold = false;
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
            // Local mode for this window (as Preferences › General's).
            "--local" => std::env::set_var("TESSERA_LOCAL", "1"),
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
            // A notebook to open (tessera notes.ipynb).
            a if a.to_ascii_lowercase().ends_with(".ipynb") => {
                let abs = std::env::current_dir().unwrap_or_default().join(a);
                std::env::set_var("TESSERA_OPEN_DOC", abs);
            }
            "-e" | "--" | "-x" | "--execute" => {
                exec_argv = args[i + 1..].to_vec();
                break;
            }
            "--hold" | "-H" => hold = true,
            "--working-directory" | "--workdir" | "-d" => {
                exec_dir = args.get(i + 1).cloned();
                i += 1;
            }
            a if a.starts_with("--working-directory=") => exec_dir = Some(a["--working-directory=".len()..].to_string()),
            _ => {}
        }
        i += 1;
    }
    // A command: a new tab in the window used last, if there is one
    // (src/gui_link.rs); else this new window runs it in its first tab.
    if !exec_argv.is_empty() || exec_dir.is_some() {
        let cwd = exec_dir.unwrap_or_else(|| std::env::current_dir().map(|d| d.to_string_lossy().into_owned()).unwrap_or_default());
        let e = if exec_argv.is_empty() { gui_link::Exec { cmd: String::new(), cwd, hold: true, token: gui_link::launch_token() } } else { gui_link::Exec::new(&exec_argv, cwd, hold) };
        if std::env::var_os("TESSERA_NO_RESTORE").is_none() && gui_link::send(&e) {
            return;
        }
        // (This window's own start uses the token from the environment.)
        std::env::set_var("TESSERA_EXEC", gui_link::Exec { token: String::new(), ..e }.to_json());
    }

    set_qt_env("QT_QUICK_CONTROLS_STYLE", std::env::var("TESSERA_STYLE").unwrap_or_else(|_| "Basic".into()));
    // Wayland: the desktop remembers where the window was (KDE Plasma 6.4+,
    // GNOME 48+); the session id it gave last time lets it put it back.
    // Only the first window does this (not New Window / torn-off tabs).
    let main_window = std::env::var_os("TESSERA_NO_RESTORE").is_none() && std::env::var_os("TESSERA_SCREENSHOT").is_none();
    let wayland = main_window && std::env::var_os("WAYLAND_DISPLAY").is_some();
    let session_file = settings::state_dir().join("window-session");
    if wayland && std::env::var_os("QT_WAYLAND_ENABLE_XX_SESSION_MANAGER").is_none() {
        set_qt_env("QT_WAYLAND_ENABLE_XX_SESSION_MANAGER", "1");
    }
    let session = if wayland { std::fs::read_to_string(&session_file).unwrap_or_default().trim().to_string() } else { String::new() };
    // Browser views (the guide, Markdown and HTML cards) could come back
    // black after the window was hidden or covered, until clicked (as
    // Almanac's message pane): Chromium stops drawing a page in a window it
    // thinks is in the background and, on Wayland, doesn't always start
    // again; and its GPU surface shared with Qt could be lost while the
    // window was hidden. Keep it drawing, and composite pages on the CPU,
    // handed to Qt as pictures (Qt Quick's own drawing stays on the GPU).
    {
        let flags = std::env::var("QTWEBENGINE_CHROMIUM_FLAGS").unwrap_or_default();
        let mut add: Vec<String> = ["--disable-renderer-backgrounding", "--disable-backgrounding-occluded-windows", "--disable-background-timer-throttling", "--disable-gpu-compositing"]
            .iter()
            .filter(|f| !flags.contains(*f))
            .map(|f| f.to_string())
            .collect();
        if !flags.contains("CalculateNativeWinOcclusion") {
            add.push("--disable-features=CalculateNativeWinOcclusion".into());
        }
        if !add.is_empty() {
            std::env::set_var("QTWEBENGINE_CHROMIUM_FLAGS", format!("{flags} {}", add.join(" ")).trim());
        }
    }
    bridge::ffi::tessera_webengine_initialize();
    // (Its own Qt's plugins, looked in first: bootstrap.h.)
    if let Some(p) = option_env!("TESSERA_QT_PLUGINS").filter(|p| std::path::Path::new(p).is_dir()) {
        std::env::set_var("TESSERA_OWN_QT_PLUGINS", p);
    }
    bridge::ffi::tessera_create_app(&QString::from(session.as_str()));
    if main_window {
        kde::remember_window_position();
    }
    bridge::ffi::tessera_set_icon(&QString::from(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tessera.svg")));
    // Local mode: no tesserad; the backend runs in this process, on a
    // socket of its own (its shells find it there too, through
    // TESSERA_SOCKET), and ends with the window.
    // (A window this one opens, New Window or a torn-off tab, shares its
    // backend while it's open: a torn-off tab's shells are there.)
    let local = local_mode();
    let shared = local.as_ref().and_then(|_| std::env::var_os("TESSERA_SOCKET")).map(std::path::PathBuf::from).filter(|p| {
        p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("tessera-local-")) && net::UnixStream::connect(p).is_ok()
    });
    let local = if shared.is_some() { None } else { local };
    if shared.is_some() {
        std::env::set_var("TESSERA_DAEMON_STATE", "current");
    } else if let Some(sock) = &local {
        std::env::set_var("TESSERA_SOCKET", sock);
        std::env::set_var("TESSERA_LOCAL", "1");
        if let Err(e) = daemon::run_in_process(sock.clone()) {
            eprintln!("tessera: can't start the local backend at {}: {e}", sock.display());
            std::process::exit(1);
        }
        std::env::set_var("TESSERA_DAEMON_STATE", "current");
    } else {
        // A daemon from an older build: swap the new one in (shells kept).
        std::env::set_var("TESSERA_DAEMON_STATE", client::sync_daemon());
    }
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        if let Some(qml) = option_env!("TESSERA_QT_QML") {
            if std::path::Path::new(qml).exists() {
                engine.as_mut().add_import_path(&QString::from(qml));
            }
        }
        engine.load(&QUrl::from("qrc:/qt/qml/Tessera/qml/Main.qml"));
    }
    // (Shells and programs started from here get the desktop's own.)
    restore_launch_env();
    bridge::ffi::tessera_show_windows();
    let code = bridge::ffi::tessera_exec();
    gui_link::remove();
    // (Notebook saves and kernel stops queued at the end still go out.)
    background::drain(std::time::Duration::from_secs(5));
    if let Some(sock) = &local {
        daemon::end_in_process(sock);
    }
    // (Kernels run on in tesserad, like the shells.)
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

/// Local mode's socket, when this window runs without tesserad: asked for
/// (--local, TESSERA_LOCAL=1) or set in Preferences (local_mode). Never for
/// an administrator window (its daemon is locked down on purpose) or one
/// already pointed at a socket of its own.
fn local_mode() -> Option<std::path::PathBuf> {
    if !cfg!(unix) || std::env::var_os("TESSERA_ADMIN").is_some() {
        return None;
    }
    let asked = std::env::var("TESSERA_LOCAL").map(|v| v != "0" && !v.is_empty());
    if !asked.unwrap_or_else(|_| settings::load().local_mode) {
        return None;
    }
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
    Some(dir.join(format!("tessera-local-{}.sock", std::process::id())))
}

/// Sets an environment variable Qt reads while starting, noting what it was
/// (as `__APP_ORIG_<name>`: empty = unset, else "=value") so
/// `restore_launch_env` puts it back once the window is loaded: programs this
/// app starts get the desktop's own. (QT_QUICK_CONTROLS_STYLE=Basic leaking
/// from our apps made KDE apps opened from them draw plain light controls,
/// 2026-11-25.)
fn set_qt_env(name: &str, value: impl AsRef<std::ffi::OsStr>) {
    let note = format!("__APP_ORIG_{name}");
    if std::env::var_os(&note).is_none() {
        let was = std::env::var_os(name)
            .map(|v| {
                let mut s = std::ffi::OsString::from("=");
                s.push(v);
                s
            })
            .unwrap_or_default();
        #[allow(unused_unsafe)]
        unsafe { std::env::set_var(&note, was) };
    }
    #[allow(unused_unsafe)]
    unsafe { std::env::set_var(name, value) };
}

/// Puts back what `set_qt_env` (or the C++ start-up) changed, once the main
/// QML has loaded: Qt has read them all by then.
#[allow(dead_code)]
fn restore_launch_env() {
    for (k, v) in std::env::vars_os() {
        let Some(name) = k.to_str().and_then(|k| k.strip_prefix("__APP_ORIG_")).map(str::to_owned) else { continue };
        let v = v.to_string_lossy().into_owned();
        #[allow(unused_unsafe)]
        unsafe {
            match v.strip_prefix('=') {
                Some(old) => std::env::set_var(&name, old),
                None => std::env::remove_var(&name),
            }
            std::env::remove_var(&k);
        }
    }
}
