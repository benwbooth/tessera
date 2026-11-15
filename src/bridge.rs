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
        ) -> Result<Box<Session>>;

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

    fn set_setting(mut self: Pin<&mut Self>, key: &ffi::QString, value: &ffi::QString) -> bool {
        let mut s = crate::settings::load();
        if !crate::settings::set(&mut s, &key.to_string(), &value.to_string()) {
            return false;
        }
        let ok = crate::settings::save(&s).is_ok();
        let r = self.as_ref().rust().revision + 1;
        self.as_mut().set_revision(r);
        ok
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
