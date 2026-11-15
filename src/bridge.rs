//! The Rust <-> C++/QML boundary.
//!
//! The terminal view itself is a small C++ QQuickItem (cpp/terminal_item.cpp)
//! that paints rows with QPainter (native FreeType text) and composites them
//! as textures. Everything else — the daemon connection, libghostty-vt, input
//! encoding, tmux control mode — lives in Rust behind the `Session` type.

#[cxx_qt::bridge]
pub mod ffi {
    #[namespace = "tessera"]
    /// A horizontal run of cells on one row sharing colors and attributes.
    struct Run {
        row: u16,
        col: u16,
        /// Width in cells (a wide character is 2).
        width: u16,
        text: String,
        fg: u32,
        bg: u32,
        /// FLAG_* bits (see session.rs).
        flags: u32,
    }

    #[namespace = "tessera"]
    /// What changed since the last frame. Only dirty rows carry runs.
    struct Frame {
        cols: u16,
        rows: u16,
        default_fg: u32,
        default_bg: u32,
        dirty_rows: Vec<u16>,
        runs: Vec<Run>,
        cursor_visible: bool,
        cursor_x: i32,
        cursor_y: i32,
        /// 0 block, 1 bar, 2 underline, 3 hollow block
        cursor_style: u8,
        cursor_color: u32,
        cursor_text: String,
        cursor_wide: bool,
        scroll_total: u64,
        scroll_offset: u64,
        scroll_len: u64,
    }

    #[namespace = "tessera"]
    extern "Rust" {
        type Session;

        /// Open a session. `attach_id` reattaches to a daemon session,
        /// `tmux_pane` ("<gateway>:<pane>") binds to a tmux control-mode
        /// pane; both empty creates a new shell in `cwd`.
        fn session_open(
            attach_id: &str,
            tmux_pane: &str,
            cwd: &str,
            cols: u16,
            rows: u16,
            cell_w: u32,
            cell_h: u32,
            scrollback: u32,
            command: &str,
            term: &str,
            startup_input: &str,
        ) -> Result<Box<Session>>;

        /// Show a line of text in the terminal itself (not sent to the program).
        fn show_message(self: &mut Session, text: &str);

        /// Profile/global options that apply live (JSON: mouse_reporting,
        /// alternate_scroll, allow_title, bracketed_paste, alt_mode,
        /// word_chars, triple_click_wrapped, clipboard_access,
        /// copy_trailing_newline).
        fn set_options(self: &mut Session, json: &str);
        /// Full reset (RIS), for a terminal left in a strange state.
        fn reset(self: &mut Session);
        /// Start (non-empty path) or stop (empty) logging output to a file.
        fn log_to_file(self: &mut Session, path: &str) -> bool;
        fn is_logging(self: &Session) -> bool;
        /// The selection with colors as HTML, and as VT control sequences.
        fn selected_html(self: &Session) -> String;
        fn selected_vt(self: &Session) -> String;
        /// Everything in the scrollback and on screen, as plain text.
        fn contents_text(self: &Session) -> String;

        /// Apply a color theme (settings::Theme JSON).
        fn set_theme(self: &mut Session, theme_json: &str);
        /// Default cursor: 0 block, 1 bar, 2 underline.
        fn set_cursor_style(self: &mut Session, style: u8);
        /// Whether mouse events go to the program (it asked for them and
        /// Shift isn't held), so the view knows a right click isn't a menu.
        fn wants_mouse(self: &Session, qt_mods: u32) -> bool;

        /// The fd to watch for readability (daemon socket or tmux wake pipe).
        fn fd(self: &Session) -> i32;
        fn id(self: &Session) -> String;
        /// Read whatever is available. Returns SIG_* bits.
        fn on_readable(self: &mut Session) -> u32;
        /// JSON array of events for QML (title, tmux windows/layouts, ...).
        fn take_events(self: &mut Session) -> String;
        fn resize(self: &mut Session, cols: u16, rows: u16, cell_w: u32, cell_h: u32);
        /// Qt key event. Returns true when it was consumed.
        fn key_event(
            self: &mut Session,
            qt_key: i32,
            qt_mods: u32,
            text: &str,
            release: bool,
            native_vk: u32,
        ) -> bool;
        fn paste(self: &mut Session, text: &str);
        fn send_text(self: &mut Session, text: &str);
        fn scroll_lines(self: &mut Session, delta: i32);
        fn scroll_to_bottom(self: &mut Session);
        /// action 0 press, 1 release, 2 move; button 0 none, 1 left, 2 right, 3 middle,
        /// 4/5 wheel up/down. Returns true when the frame changed (selection).
        fn mouse_event(
            self: &mut Session,
            action: u8,
            button: u8,
            x: f32,
            y: f32,
            qt_mods: u32,
            width: u32,
            height: u32,
            cell_w: u32,
            cell_h: u32,
            pad: u32,
        ) -> bool;
        fn selected_text(self: &Session) -> String;
        /// Find in scrollback: number of matches (-1: bad regex).
        fn find(self: &mut Session, query: &str, regex: bool) -> i32;
        fn find_refresh(self: &mut Session) -> i32;
        /// Previous (-1, upward) / next (+1) match; returns its index or -1.
        fn find_step(self: &mut Session, dir: i32) -> i32;
        fn find_index(self: &Session) -> i32;
        fn find_clear(self: &mut Session);
        fn select_all(self: &mut Session);
        fn clear_scrollback(self: &mut Session);
        fn clear_selection(self: &mut Session);
        fn title(self: &Session) -> String;
        fn pwd(self: &Session) -> String;
        fn frame(self: &mut Session, out: &mut Frame, force_full: bool) -> bool;
        fn focus(self: &mut Session, focused: bool);
        /// Ends the shell (closing a pane); dropping without this detaches.
        fn kill(self: &mut Session);
        /// tmux pane id, or -1 when this isn't a tmux pane.
        fn tmux_pane_id(self: &Session) -> i32;
        /// Gateway key of the tmux connection this session belongs to (or runs).
        fn tmux_gateway(self: &Session) -> String;
        /// Run a tmux command through this session's gateway.
        fn tmux_command(self: &mut Session, command: &str);
        /// Set a tmux pane's grid (tmux decides pane sizes, not the view).
        fn tmux_set_grid(self: &mut Session, cols: u16, rows: u16, cell_w: u32, cell_h: u32);
    }

    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    unsafe extern "C++" {
        include!("bootstrap.h");
        /// The application (QApplication, for the desktop's native menus),
        /// native text rendering, and QML registration of TerminalView.
        fn tessera_create_app();
        fn tessera_exec() -> i32;
        /// The app icon for window decorations that read it from the window.
        fn tessera_set_icon(path: &QString);
        /// Fixed-pitch font families installed, as a JSON array.
        fn tessera_monospace_families() -> QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        /// Goes up whenever the settings or the theme list change.
        #[qproperty(i32, revision)]
        type Controller = super::ControllerRust;

        /// Preferences as JSON (settings::Settings).
        #[qinvokable]
        fn settings(self: &Controller) -> QString;
        /// Change one preference: `value` is JSON. Saved right away.
        #[qinvokable]
        fn set_setting(self: Pin<&mut Controller>, key: &QString, value: &QString) -> bool;
        /// One profile, fully resolved (the default one if there's no such profile).
        #[qinvokable]
        fn profile(self: &Controller, name: &QString) -> QString;
        /// Change one field of a profile (`value` is JSON).
        #[qinvokable]
        fn set_profile(self: Pin<&mut Controller>, name: &QString, key: &QString, value: &QString) -> bool;
        /// New profile copied from `from` (empty: a fresh one). Returns its name.
        #[qinvokable]
        fn add_profile(self: Pin<&mut Controller>, from: &QString) -> QString;
        #[qinvokable]
        fn remove_profile(self: Pin<&mut Controller>, name: &QString);
        /// Saved window arrangements as a JSON object (name -> layout).
        #[qinvokable]
        fn arrangements(self: &Controller) -> QString;
        #[qinvokable]
        fn save_arrangement(self: Pin<&mut Controller>, name: &QString, layout: &QString);
        #[qinvokable]
        fn delete_arrangement(self: Pin<&mut Controller>, name: &QString);
        /// The layout saved when the window last closed (restored at start).
        #[qinvokable]
        fn last_layout(self: &Controller) -> QString;
        #[qinvokable]
        fn save_last_layout(self: &Controller, layout: &QString);
        /// Write text to a file (path or file:// URL).
        #[qinvokable]
        fn write_file(self: &Controller, path: &QString, text: &QString) -> bool;
        /// The program running in a daemon session's foreground, if it isn't
        /// the shell itself ("" when the shell is idle).
        #[qinvokable]
        fn session_job(self: &Controller, id: &QString) -> QString;
        /// Open a window holding this tab (JSON layout with session ids): a
        /// tab torn off by dragging it out of the tab bar.
        #[qinvokable]
        fn new_window_with_tab(self: &Controller, tab: &QString);
        /// The tab a torn-off window starts with (JSON), or empty.
        #[qinvokable]
        fn initial_tab(self: &Controller) -> QString;
        /// Open another window (a new process that starts fresh, restoring nothing).
        #[qinvokable]
        fn new_window(self: &Controller);
        /// False in a window opened with New Window (it mustn't take over the
        /// first window's sessions).
        #[qinvokable]
        fn restore_allowed(self: &Controller) -> bool;
        /// Every theme (built in, then imported) as a JSON array.
        #[qinvokable]
        fn themes(self: &Controller) -> QString;
        /// One theme by name (the first built-in one if there's no such theme).
        #[qinvokable]
        fn theme(self: &Controller, name: &QString) -> QString;
        /// Import an .itermcolors file (path or file:// URL). Returns the new
        /// theme's name, or "error: ..." when it can't be read.
        #[qinvokable]
        fn import_theme(self: Pin<&mut Controller>, path: &QString) -> QString;
        #[qinvokable]
        fn delete_theme(self: Pin<&mut Controller>, name: &QString);
        #[qinvokable]
        fn monospace_families(self: &Controller) -> QString;
        /// The folder with settings.json and themes/.
        #[qinvokable]
        fn config_folder(self: &Controller) -> QString;

        /// JSON list of detached daemon session ids to restore at startup.
        #[qinvokable]
        fn restorable_sessions(self: &Controller) -> QString;

        /// Where to save a screenshot and quit (--screenshot), or empty.
        #[qinvokable]
        fn screenshot_path(self: &Controller) -> QString;

        /// Text typed into the first terminal at startup (--run), or empty.
        #[qinvokable]
        fn startup_command(self: &Controller) -> QString;

        /// Testing: text typed into the focused terminal a few seconds later
        /// (--then), and how long to wait before the screenshot (ms).
        #[qinvokable]
        fn then_command(self: &Controller) -> QString;
        #[qinvokable]
        fn screenshot_delay(self: &Controller) -> i32;
        /// Testing: open the find bar with this query (--find).
        #[qinvokable]
        fn find_query(self: &Controller) -> QString;
        /// Testing: open Preferences at this page (--prefs).
        #[qinvokable]
        fn prefs_page(self: &Controller) -> QString;
    }
}

pub use crate::session::{session_open, Session};

#[derive(Default)]
pub struct ControllerRust {
    revision: i32,
}

use std::pin::Pin;
use cxx_qt::CxxQtType;

fn qs(s: &str) -> ffi::QString {
    ffi::QString::from(s)
}

impl ffi::Controller {
    fn settings(&self) -> ffi::QString {
        qs(&serde_json::to_string(&crate::settings::load()).unwrap())
    }

    fn set_setting(self: Pin<&mut Self>, key: &ffi::QString, value: &ffi::QString) -> bool {
        let mut s = crate::settings::load();
        if !crate::settings::set(&mut s, &key.to_string(), &value.to_string()) {
            return false;
        }
        let ok = crate::settings::save(&s).is_ok();
        self.bump();
        ok
    }

    fn bump(mut self: Pin<&mut Self>) {
        let r = self.as_ref().rust().revision + 1;
        self.as_mut().set_revision(r);
    }

    fn profile(&self, name: &ffi::QString) -> ffi::QString {
        qs(&serde_json::to_string(&crate::settings::load().profile(&name.to_string())).unwrap())
    }

    fn set_profile(self: Pin<&mut Self>, name: &ffi::QString, key: &ffi::QString, value: &ffi::QString) -> bool {
        let mut s = crate::settings::load();
        if !s.set_profile_field(&name.to_string(), &key.to_string(), &value.to_string()) {
            return false;
        }
        let ok = crate::settings::save(&s).is_ok();
        self.bump();
        ok
    }

    fn add_profile(self: Pin<&mut Self>, from: &ffi::QString) -> ffi::QString {
        let mut s = crate::settings::load();
        let from = from.to_string();
        let name = s.add_profile(if from.is_empty() { None } else { Some(&from) });
        let _ = crate::settings::save(&s);
        self.bump();
        qs(&name)
    }

    fn remove_profile(self: Pin<&mut Self>, name: &ffi::QString) {
        let mut s = crate::settings::load();
        s.remove_profile(&name.to_string());
        let _ = crate::settings::save(&s);
        self.bump();
    }

    fn arrangements(&self) -> ffi::QString {
        qs(&serde_json::to_string(&crate::settings::arrangements()).unwrap())
    }

    fn save_arrangement(self: Pin<&mut Self>, name: &ffi::QString, layout: &ffi::QString) {
        let mut m = crate::settings::arrangements();
        if let Ok(v) = serde_json::from_str(&layout.to_string()) {
            m.insert(name.to_string(), v);
            let _ = crate::settings::save_arrangements(&m);
        }
        self.bump();
    }

    fn delete_arrangement(self: Pin<&mut Self>, name: &ffi::QString) {
        let mut m = crate::settings::arrangements();
        m.remove(&name.to_string());
        let _ = crate::settings::save_arrangements(&m);
        self.bump();
    }

    fn last_layout(&self) -> ffi::QString {
        qs(&crate::settings::last_layout())
    }

    fn save_last_layout(&self, layout: &ffi::QString) {
        crate::settings::save_last_layout(&layout.to_string());
    }

    fn write_file(&self, path: &ffi::QString, text: &ffi::QString) -> bool {
        let p = path.to_string();
        let p = percent_decode(p.strip_prefix("file://").unwrap_or(&p));
        std::fs::write(p, text.to_string()).is_ok()
    }

    fn session_job(&self, id: &ffi::QString) -> ffi::QString {
        qs(&crate::client::session_job(&id.to_string()).unwrap_or_default())
    }

    fn new_window(&self) {
        if let Ok(exe) = std::env::current_exe() {
            let mut cmd = std::process::Command::new(exe);
            cmd.arg("--new-window");
            for k in ["TESSERA_SCREENSHOT", "TESSERA_RUN", "TESSERA_THEN", "TESSERA_PREFS"] {
                cmd.env_remove(k);
            }
            let _ = cmd.spawn();
        }
    }

    fn new_window_with_tab(&self, tab: &ffi::QString) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let file = dir.join(format!("tessera-tab-{}-{}.json", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        if std::fs::write(&file, tab.to_string()).is_err() {
            return;
        }
        if let Ok(exe) = std::env::current_exe() {
            let mut cmd = std::process::Command::new(exe);
            cmd.arg("--new-window").arg("--tab").arg(&file);
            for k in ["TESSERA_SCREENSHOT", "TESSERA_RUN", "TESSERA_THEN", "TESSERA_PREFS"] {
                cmd.env_remove(k);
            }
            let _ = cmd.spawn();
        }
    }

    fn initial_tab(&self) -> ffi::QString {
        let Some(path) = std::env::var_os("TESSERA_TAB_FILE") else { return qs("") };
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        qs(&text)
    }

    fn restore_allowed(&self) -> bool {
        std::env::var_os("TESSERA_NO_RESTORE").is_none()
    }

    fn themes(&self) -> ffi::QString {
        qs(&serde_json::to_string(&crate::settings::all_themes()).unwrap())
    }

    fn theme(&self, name: &ffi::QString) -> ffi::QString {
        qs(&serde_json::to_string(&crate::settings::find_theme(&name.to_string())).unwrap())
    }

    fn import_theme(mut self: Pin<&mut Self>, path: &ffi::QString) -> ffi::QString {
        let p = path.to_string();
        let p = p.strip_prefix("file://").unwrap_or(&p).to_string();
        let p = percent_decode(&p);
        match crate::settings::import_itermcolors(&p) {
            Ok(t) => {
                let r = self.as_ref().rust().revision + 1;
                self.as_mut().set_revision(r);
                qs(&t.name)
            }
            Err(e) => qs(&format!("error: {e}")),
        }
    }

    fn delete_theme(mut self: Pin<&mut Self>, name: &ffi::QString) {
        let name = name.to_string();
        let dir = crate::settings::config_dir().join("themes");
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let matches = std::fs::read(e.path())
                    .ok()
                    .and_then(|b| serde_json::from_slice::<crate::settings::Theme>(&b).ok())
                    .is_some_and(|t| t.name == name);
                if matches {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
        let r = self.as_ref().rust().revision + 1;
        self.as_mut().set_revision(r);
    }

    fn monospace_families(&self) -> ffi::QString {
        ffi::tessera_monospace_families()
    }

    fn config_folder(&self) -> ffi::QString {
        qs(&crate::settings::config_dir().display().to_string())
    }

    fn restorable_sessions(&self) -> ffi::QString {
        let ids: Vec<String> = crate::client::list_sessions()
            .unwrap_or_default()
            .into_iter()
            .filter(|s| !s.attached)
            .map(|s| s.id)
            .collect();
        ffi::QString::from(&serde_json::to_string(&ids).unwrap())
    }

    fn screenshot_path(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_SCREENSHOT").unwrap_or_default())
    }

    fn then_command(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_THEN").unwrap_or_default())
    }

    fn find_query(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_FIND").unwrap_or_default())
    }

    fn prefs_page(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_PREFS").unwrap_or_default())
    }

    fn screenshot_delay(&self) -> i32 {
        std::env::var("TESSERA_SHOT_DELAY").ok().and_then(|v| v.parse().ok()).unwrap_or(3500)
    }

    fn startup_command(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_RUN").unwrap_or_default())
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
