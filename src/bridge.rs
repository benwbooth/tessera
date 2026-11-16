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
        /// Dirty rows where a shell prompt starts (mark indicators).
        mark_rows: Vec<u16>,
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
        fn save_snapshot(self: &Session, path: &str) -> bool;
        fn preload(self: &mut Session, path: &str) -> bool;

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
        /// A full-screen program is running (the alternate screen is up).
        fn full_screen(self: &Session) -> bool;
        /// The shell is at its prompt (from shell integration's marks).
        fn at_prompt(self: &Session) -> bool;

        /// The fd to watch for readability (daemon socket or tmux wake pipe).
        fn fd(self: &Session) -> i64;
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
        /// Ctrl+click target: a hyperlink, URL or existing file (file://), or "".
        fn link_at(self: &Session, x: f32, y: f32, cell_w: u32, cell_h: u32, pad: u32) -> String;
        /// Shell integration: scroll to the previous (-1) / next (+1) prompt.
        fn jump_to_prompt(self: &mut Session, dir: i32) -> bool;
        /// Shell integration: select the last command's output; returns it.
        fn select_last_output(self: &mut Session) -> String;
        /// Put this row (0 = top of the scrollback) at the top of the view.
        fn scroll_to_row(self: &mut Session, row: u64);
        /// Whether wheel/touchpad scrolling moves through the scrollback here
        /// (not sent to a program as mouse or arrow keys).
        fn scrolls_locally(self: &Session, qt_mods: u32) -> bool;
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
        /// Selection dragged past the top/bottom: scroll a row and extend it.
        fn autoscroll_tick(self: &mut Session, x: f32, y: f32, width: u32, height: u32, cell_w: u32, cell_h: u32, pad: u32, rows_per_tick: u32) -> bool;
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

    impl cxx_qt::Threading for Controller {}

    unsafe extern "C++" {
        include!("bootstrap.h");
        /// The application (QApplication, for the desktop's native menus),
        /// native text rendering, and QML registration of TerminalView.
        fn tessera_create_app(session: &QString);
        fn tessera_show_windows();
        fn tessera_session_id() -> QString;
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
        /// Progress of "Get All iTerm2-Color-Schemes" ("" when idle).
        #[qproperty(QString, pack_status)]
        /// Progress of downloading fish's completions for command help.
        #[qproperty(QString, help_status)]
        type Controller = super::ControllerRust;

        /// Download the complete iTerm2-Color-Schemes collection (background).
        #[qinvokable]
        fn download_theme_pack(self: Pin<&mut Controller>);
        /// For tests: the value of TESSERA_TEST_<name> (empty if unset).
        #[qinvokable]
        fn test_env(self: &Controller, name: &QString) -> QString;

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
        /// {job, cwd, shell} of a session, as JSON.
        #[qinvokable]
        fn session_info(self: &Controller, id: &QString) -> QString;
        #[qinvokable]
        fn home_dir(self: &Controller) -> QString;
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
        /// The input editor's syntax analysis (src/syntax.rs) as JSON.
        #[qinvokable]
        fn syntax(self: &Controller, lang: &QString, text: &QString) -> QString;
        /// The input editor's language for the program at the prompt.
        #[qinvokable]
        fn language_for(self: &Controller, program: &QString) -> QString;
        /// Input editor completions at a UTF-16 cursor (src/complete.rs), JSON.
        #[qinvokable]
        fn complete(self: &Controller, text: &QString, cursor: i32, cwd: &QString) -> QString;
        /// Download fish's completion files (more commands and subcommands
        /// get help text); progress in help_status.
        #[qinvokable]
        fn download_command_help(self: Pin<&mut Controller>);
        #[qinvokable]
        fn command_help_count(self: &Controller) -> i32;
        /// The prompt bar's facts about a folder (git, user, host), JSON.
        #[qinvokable]
        fn prompt_info(self: &Controller, cwd: &QString, pid: i32) -> QString;
        #[qinvokable]
        fn has_starship(self: &Controller) -> bool;
        /// Ask Starship for the prompt (in the background); the answer comes
        /// as starship_ready(key, json) (see prompt::starship).
        #[qinvokable]
        fn request_starship(self: Pin<&mut Controller>, key: &QString, cwd: &QString, status: i32, duration_ms: f64, width: i32, pid: i32);
        #[qsignal]
        fn starship_ready(self: Pin<&mut Controller>, key: QString, json: QString);

        /// An AI request (src/ai.rs) in the background. `task`: "inline",
        /// "command", "check", "fix" or "chat" (which model, system prompt and
        /// limits); `messages_json`: [{role, content}] after the system prompt;
        /// `ctx_json`: {cwd, shell}. Streams ai_delta(key, text), ends with
        /// ai_done(key, reply, error). A new request with the same key cancels
        /// the last one.
        #[qinvokable]
        fn ai_run(self: Pin<&mut Controller>, key: &QString, task: &QString, messages_json: &QString, ctx_json: &QString);
        #[qinvokable]
        fn ai_cancel(self: &Controller, key: &QString);
        /// Whether a task has a model set up (and its provider a key, if it needs one).
        #[qinvokable]
        fn ai_ready(self: &Controller, task: &QString) -> bool;
        #[qsignal]
        fn ai_delta(self: Pin<&mut Controller>, key: QString, text: QString);
        #[qsignal]
        fn ai_done(self: Pin<&mut Controller>, key: QString, reply: QString, error: QString);
        /// Providers Tessera knows (JSON), for adding one.
        #[qinvokable]
        fn ai_presets(self: &Controller) -> QString;
        #[qinvokable]
        fn ai_set_key(self: &Controller, provider: &QString, key: &QString) -> QString;
        #[qinvokable]
        fn ai_has_key(self: &Controller, provider: &QString) -> bool;
        /// A provider's models, in the background: ai_models_ready(provider, json list or "error: ...").
        #[qinvokable]
        fn ai_list_models(self: Pin<&mut Controller>, provider_json: &QString);
        #[qsignal]
        fn ai_models_ready(self: Pin<&mut Controller>, provider: QString, json: QString);
        /// First run: set up Ollama if it's running here (background; settings change).
        #[qinvokable]
        fn ai_autodetect(self: Pin<&mut Controller>);
        /// Command history (Tessera's merged with the shells'), newest last, JSON array.
        #[qinvokable]
        fn history_all(self: &Controller, limit: i32) -> QString;
        #[qinvokable]
        fn history_add(self: &Controller, cmd: &QString, cwd: &QString);
        /// History matches for the dropdown (completion JSON).
        #[qinvokable]
        fn history_search(self: &Controller, query: &QString) -> QString;
        /// How the running daemon compared with this build at startup:
        /// "none", "current", "upgraded", "outdated", "newer".
        #[qinvokable]
        fn daemon_state(self: &Controller) -> QString;
        /// End an outdated daemon (and its shells). False if it couldn't be.
        #[qinvokable]
        fn stop_daemon(self: &Controller) -> bool;
        /// Windows: open an elevated window (the UAC prompt first).
        #[qinvokable]
        fn new_admin_window(self: &Controller);
        /// This window is elevated (opened with New Administrator Window).
        #[qinvokable]
        fn is_admin(self: &Controller) -> bool;
        /// False in a window opened with New Window (it mustn't take over the
        /// first window's sessions).
        #[qinvokable]
        fn restore_allowed(self: &Controller) -> bool;
        /// For the + menu: {shells: [{name, path}], ssh_hosts: [..]}.
        #[qinvokable]
        fn launchers(self: &Controller) -> QString;
        /// Where restore state lives (layout, saved screens).
        #[qinvokable]
        fn state_folder(self: &Controller) -> QString;
        /// Delete saved screens except these session ids (JSON array).
        #[qinvokable]
        fn prune_snapshots(self: &Controller, keep: &QString);
        /// Remember the desktop's id for this window (Wayland session restore).
        #[qinvokable]
        fn save_window_session(self: &Controller);
        /// End a detached session by id (Undo Close ran out).
        #[qinvokable]
        fn kill_session(self: &Controller, id: &QString);
        /// A desktop notification.
        #[qinvokable]
        fn notify(self: &Controller, title: &QString, body: &QString);
        /// Install shell integration for "bash", "zsh" or "fish": writes the
        /// script and adds a line to the shell's startup file. Returns what
        /// happened (or what to add by hand when the file can't be changed).
        #[qinvokable]
        fn install_shell_integration(self: &Controller, shell: &QString) -> QString;
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
use cxx_qt::Threading;
use std::sync::Arc;

#[derive(Default)]
pub struct ControllerRust {
    revision: i32,
    pack_status: ffi::QString,
    help_status: ffi::QString,
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

    fn session_info(&self, id: &ffi::QString) -> ffi::QString {
        let info = crate::client::proc_info(&id.to_string()).unwrap_or_default();
        qs(&serde_json::to_string(&info).unwrap_or_default())
    }

    fn home_dir(&self) -> ffi::QString {
        qs(&std::env::var("HOME").unwrap_or_default())
    }

    fn is_admin(&self) -> bool {
        std::env::var_os("TESSERA_ADMIN").is_some()
    }

    fn new_admin_window(&self) {
        #[cfg(windows)]
        if let Ok(exe) = std::env::current_exe() {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::UI::Shell::ShellExecuteW;
            use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
            let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(std::iter::once(0)).collect::<Vec<u16>>();
            let verb = wide("runas".as_ref());
            let file = wide(exe.as_os_str());
            let args = wide("--admin".as_ref());
            unsafe {
                ShellExecuteW(std::ptr::null_mut(), verb.as_ptr(), file.as_ptr(), args.as_ptr(), std::ptr::null(), SW_SHOWNORMAL);
            }
        }
    }

    fn syntax(&self, lang: &ffi::QString, text: &ffi::QString) -> ffi::QString {
        qs(&crate::syntax::analyze(&lang.to_string(), &text.to_string()))
    }

    fn complete(&self, text: &ffi::QString, cursor: i32, cwd: &ffi::QString) -> ffi::QString {
        let cwd = cwd.to_string();
        // An OSC 7 URL (file://host/path) or a plain path.
        let cwd = match cwd.strip_prefix("file://") {
            Some(rest) => percent_decode(&rest[rest.find('/').unwrap_or(rest.len())..]),
            None => cwd,
        };
        let cwd = if cwd.is_empty() { std::env::var("HOME").unwrap_or_default() } else { cwd };
        qs(&crate::complete::complete(&text.to_string(), cursor.max(0) as usize, &cwd))
    }

    fn download_command_help(mut self: Pin<&mut Self>) {
        self.as_mut().set_help_status(qs("Downloading fish's completions and tldr-pages\u{2026}"));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let fish = crate::complete::download_fish_completions();
            let tldr = crate::complete::update_tldr();
            let _ = qt.queue(move |mut c| {
                let msg = match (fish, tldr) {
                    (Ok(f), Ok(t)) => format!("Up to date: completions for {f} commands (fish-shell), examples for {t} (tldr-pages)."),
                    (Err(e), _) | (_, Err(e)) => format!("Couldn't download: {e}"),
                };
                c.as_mut().set_help_status(qs(&msg));
            });
        });
    }

    fn daemon_state(&self) -> ffi::QString {
        qs(&std::env::var("TESSERA_DAEMON_STATE").unwrap_or_default())
    }

    fn stop_daemon(&self) -> bool {
        crate::client::stop_daemon()
    }

    fn history_all(&self, limit: i32) -> ffi::QString {
        qs(&crate::history::all_json(limit.max(1) as usize))
    }

    fn history_add(&self, cmd: &ffi::QString, cwd: &ffi::QString) {
        let cwd = cwd.to_string();
        let cwd = match cwd.strip_prefix("file://") {
            Some(rest) => percent_decode(&rest[rest.find('/').unwrap_or(rest.len())..]),
            None => cwd,
        };
        crate::history::add(&cmd.to_string(), &cwd);
    }

    fn history_search(&self, query: &ffi::QString) -> ffi::QString {
        qs(&crate::history::search(&query.to_string()))
    }

    fn prompt_info(&self, cwd: &ffi::QString, pid: i32) -> ffi::QString {
        qs(&crate::prompt::info(&local_path(cwd), pid.max(0) as u32))
    }

    fn ai_run(self: Pin<&mut Self>, key: &ffi::QString, task: &ffi::QString, messages_json: &ffi::QString, ctx_json: &ffi::QString) {
        use std::sync::atomic::{AtomicBool, Ordering};
        let (key, task) = (key.to_string(), task.to_string());
        let mut msgs: Vec<serde_json::Value> = serde_json::from_str(&messages_json.to_string()).unwrap_or_default();
        let ctx: serde_json::Value = serde_json::from_str(&ctx_json.to_string()).unwrap_or_default();
        let s = crate::settings::load();
        let model_ref = match task.as_str() {
            "inline" => s.ai.inline_model.clone(),
            "check" => if s.ai.check_model.is_empty() { s.ai.inline_model.clone() } else { s.ai.check_model.clone() },
            _ => s.ai.chat_model.clone(),
        };
        let qt = self.qt_thread();
        let Some((provider, model)) = crate::ai::resolve(&s.ai.providers, &model_ref) else {
            let _ = qt.queue(move |c| c.ai_done(qs(&key), qs(""), qs("No AI model set up (Preferences › AI).")));
            return;
        };
        let cancel = Arc::new(AtomicBool::new(false));
        if let Some(old) = ai_jobs().lock().unwrap().insert(key.clone(), cancel.clone()) {
            old.store(true, Ordering::Relaxed);
        }
        msgs.insert(0, serde_json::json!({"role": "system", "content": crate::ai::system_prompt(&task, &ctx)}));
        let opts = match task.as_str() {
            "inline" => crate::ai::Options { max_tokens: 80, temperature: 0.1, timeout: std::time::Duration::from_secs(8) },
            "check" => crate::ai::Options { max_tokens: 80, temperature: 0.1, timeout: std::time::Duration::from_secs(15) },
            "command" | "fix" => crate::ai::Options { max_tokens: 300, temperature: 0.2, timeout: std::time::Duration::from_secs(60) },
            _ => crate::ai::Options { max_tokens: 4096, temperature: 0.4, timeout: std::time::Duration::from_secs(600) },
        };
        if std::env::var_os("TESSERA_AI_DEBUG").is_some() {
            eprintln!("tessera ai: {task} with {}/{model} ({} messages)", provider.id, msgs.len());
        }
        std::thread::spawn(move || {
            let stream = task == "chat";
            let q2 = qt.clone();
            let k2 = key.clone();
            let result = crate::ai::chat(&provider, &model, &msgs, &opts, &cancel, |piece| {
                if stream {
                    let (k, p) = (k2.clone(), piece.to_string());
                    let _ = q2.queue(move |c| c.ai_delta(qs(&k), qs(&p)));
                }
            });
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            {
                let mut jobs = ai_jobs().lock().unwrap();
                if jobs.get(&key).is_some_and(|c| Arc::ptr_eq(c, &cancel)) {
                    jobs.remove(&key);
                }
            }
            if std::env::var_os("TESSERA_AI_DEBUG").is_some() {
                eprintln!("tessera ai: {task} done: {:?}", result.as_ref().map(|r| r.chars().take(80).collect::<String>()));
            }
            let (reply, err) = match result {
                // (One command, for the tasks that want one.)
                Ok(r) if matches!(task.as_str(), "inline" | "command" | "fix") => (crate::ai::command_from(&r), String::new()),
                Ok(r) => (r, String::new()),
                Err(e) => (String::new(), e),
            };
            let _ = qt.queue(move |c| c.ai_done(qs(&key), qs(&reply), qs(&err)));
        });
    }

    fn ai_cancel(&self, key: &ffi::QString) {
        if let Some(c) = ai_jobs().lock().unwrap().remove(&key.to_string()) {
            c.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn ai_ready(&self, task: &ffi::QString) -> bool {
        let s = crate::settings::load();
        let r = match task.to_string().as_str() {
            "inline" => s.ai.inline_model.clone(),
            "check" => if s.ai.check_model.is_empty() { s.ai.inline_model.clone() } else { s.ai.check_model.clone() },
            _ => s.ai.chat_model.clone(),
        };
        crate::ai::resolve(&s.ai.providers, &r).is_some_and(|(p, _)| p.local || crate::ai::has_key(&p))
    }

    fn ai_presets(&self) -> ffi::QString {
        qs(&serde_json::to_string(&crate::ai::presets()).unwrap_or_default())
    }

    fn ai_set_key(&self, provider: &ffi::QString, key: &ffi::QString) -> ffi::QString {
        match crate::ai::set_key(&provider.to_string(), key.to_string().trim()) {
            Ok(()) => qs(""),
            Err(e) => qs(&e),
        }
    }

    fn ai_has_key(&self, provider: &ffi::QString) -> bool {
        let id = provider.to_string();
        let s = crate::settings::load();
        let p = s.ai.providers.iter().find(|p| p.id == id).cloned().or_else(|| crate::ai::presets().into_iter().find(|p| p.id == id));
        p.is_some_and(|p| crate::ai::has_key(&p))
    }

    fn ai_list_models(self: Pin<&mut Self>, provider_json: &ffi::QString) {
        let p: crate::ai::Provider = serde_json::from_str(&provider_json.to_string()).unwrap_or_default();
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let json = match crate::ai::models(&p) {
                Ok(m) => serde_json::to_string(&m).unwrap_or_default(),
                Err(e) => format!("error: {e}"),
            };
            let id = p.id.clone();
            let _ = qt.queue(move |c| c.ai_models_ready(qs(&id), qs(&json)));
        });
    }

    fn ai_autodetect(self: Pin<&mut Self>) {
        if crate::settings::load().ai.autodetected {
            return;
        }
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let found = crate::ai::autodetect();
            let _ = qt.queue(move |mut c| {
                let mut s = crate::settings::load();
                s.ai.autodetected = true;
                if let Some((p, small, big)) = found {
                    if !s.ai.providers.iter().any(|x| x.id == p.id) {
                        s.ai.providers.push(p);
                    }
                    if s.ai.inline_model.is_empty() {
                        s.ai.inline_model = small;
                    }
                    if s.ai.chat_model.is_empty() {
                        s.ai.chat_model = big;
                    }
                }
                let _ = crate::settings::save(&s);
                let r = c.as_ref().rust().revision + 1;
                c.as_mut().set_revision(r);
            });
        });
    }

    fn has_starship(&self) -> bool {
        crate::prompt::has_starship()
    }

    fn request_starship(self: Pin<&mut Self>, key: &ffi::QString, cwd: &ffi::QString, status: i32, duration_ms: f64, width: i32, pid: i32) {
        let (key, cwd) = (key.to_string(), local_path(cwd));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let json = crate::prompt::starship(&cwd, status, duration_ms.max(0.0) as u64, width.max(20) as u32, pid.max(0) as u32);
            let _ = qt.queue(move |c| c.starship_ready(qs(&key), qs(&json)));
        });
    }

    fn command_help_count(&self) -> i32 {
        (crate::complete::fish_completion_count() + crate::complete::tldr_count()) as i32
    }

    fn language_for(&self, program: &ffi::QString) -> ffi::QString {
        qs(crate::syntax::language_for(&program.to_string()))
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

    fn launchers(&self) -> ffi::QString {
        qs(&serde_json::to_string(&crate::launchers::launchers()).unwrap())
    }

    fn state_folder(&self) -> ffi::QString {
        qs(&crate::settings::state_dir().display().to_string())
    }

    fn prune_snapshots(&self, keep: &ffi::QString) {
        let keep: Vec<String> = serde_json::from_str(&keep.to_string()).unwrap_or_default();
        let dir = crate::settings::state_dir().join("snapshots");
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                let id = name.trim_end_matches(".vt");
                if !keep.iter().any(|k| k == id) {
                    let _ = std::fs::remove_file(e.path());
                }
            }
        }
    }

    fn save_window_session(&self) {
        if std::env::var_os("TESSERA_NO_RESTORE").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_none() {
            return;
        }
        let id = ffi::tessera_session_id().to_string();
        if !id.is_empty() && !id.contains('_') {
            let dir = crate::settings::state_dir();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("window-session"), id);
        }
    }

    fn download_theme_pack(mut self: Pin<&mut Self>) {
        self.as_mut().set_pack_status(qs("Downloading iTerm2-Color-Schemes\u{2026}"));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let result = crate::settings::download_pack();
            let _ = qt.queue(move |mut c| {
                let msg = match result {
                    Ok(n) => format!("Added the full collection: {n} themes."),
                    Err(e) => format!("Couldn't download: {e}"),
                };
                c.as_mut().set_pack_status(qs(&msg));
                let r = c.as_ref().rust().revision + 1;
                c.as_mut().set_revision(r);
            });
        });
    }

    fn kill_session(&self, id: &ffi::QString) {
        crate::client::kill_session(&id.to_string());
    }

    fn notify(&self, title: &ffi::QString, body: &ffi::QString) {
        let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tessera.svg");
        let _ = std::process::Command::new("notify-send")
            .args(["-a", "Tessera", "-i", icon, &title.to_string(), &body.to_string()])
            .spawn();
    }

    fn install_shell_integration(&self, shell: &ffi::QString) -> ffi::QString {
        qs(&crate::shell_integration::install(&shell.to_string()))
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

    fn test_env(&self, name: &ffi::QString) -> ffi::QString {
        ffi::QString::from(&std::env::var(format!("TESSERA_TEST_{name}")).unwrap_or_default())
    }

    fn startup_command(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_RUN").unwrap_or_default())
    }
}

/// A folder from QML: an OSC 7 URL (file://host/path) or a plain path; home
/// when empty.
fn local_path(s: &ffi::QString) -> String {
    let s = s.to_string();
    let p = match s.strip_prefix("file://") {
        Some(rest) => percent_decode(&rest[rest.find('/').unwrap_or(rest.len())..]),
        None => s,
    };
    if p.is_empty() { std::env::var("HOME").unwrap_or_default() } else { p }
}

fn ai_jobs() -> &'static std::sync::Mutex<std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>>> {
    static JOBS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>>>> = std::sync::OnceLock::new();
    JOBS.get_or_init(Default::default)
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
