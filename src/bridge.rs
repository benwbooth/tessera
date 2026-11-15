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
        ) -> Result<Box<Session>>;

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
        /// Native text rendering + QML registration of TerminalView.
        fn tessera_prepare_qt();
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type Controller = super::ControllerRust;

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
    }
}

pub use crate::session::{session_open, Session};

#[derive(Default)]
pub struct ControllerRust;

impl ffi::Controller {
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

    fn screenshot_delay(&self) -> i32 {
        std::env::var("TESSERA_SHOT_DELAY").ok().and_then(|v| v.parse().ok()).unwrap_or(3500)
    }

    fn startup_command(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_RUN").unwrap_or_default())
    }
}
