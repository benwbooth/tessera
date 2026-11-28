//! The Rust <-> C++/QML boundary.
//!
//! The terminal view itself is a small C++ QQuickItem (cpp/terminal_item.cpp)
//! that paints rows with QPainter (native FreeType text) and composites them
//! as textures. Everything else — the daemon connection, libghostty-vt, input
//! encoding — lives in Rust behind the `Session` type.

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
        /// Pictures on screen (kitty graphics), in device pixels from the grid.
        images: Vec<ImagePlace>,
    }

    #[namespace = "tessera"]
    /// A picture's place: which image (and its generation, so a changed
    /// image is fetched again), where, how big, which part of it, and its
    /// layer (z < 0 under the text).
    struct ImagePlace {
        id: u32,
        generation: u64,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        src_x: u32,
        src_y: u32,
        src_w: u32,
        src_h: u32,
        z: i32,
    }

    #[namespace = "tessera"]
    struct ImagePixels {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    }

    #[namespace = "tessera"]
    extern "Rust" {
        type Session;

        /// Open a session: `attach_id` reattaches to one in tesserad, empty
        /// creates a new shell in `cwd` (on `host`: "" here, else over SSH).
        fn session_open(
            attach_id: &str,
            cwd: &str,
            cols: u16,
            rows: u16,
            cell_w: u32,
            cell_h: u32,
            scrollback: u32,
            command: &str,
            term: &str,
            startup_input: &str,
            host: &str,
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
        /// Start (a path) or stop ("") recording the output (asciicast);
        /// the recording's path ("" when it couldn't start).
        fn record_to_file(self: &mut Session, path: &str) -> String;
        fn is_recording(self: &Session) -> bool;
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
        fn app_owns_click(self: &Session, view_row: u32) -> bool;
        /// A full-screen program is running (the alternate screen is up).
        fn full_screen(self: &Session) -> bool;
        /// An image's pixels (RGBA) for drawing it.
        fn image_pixels(self: &Session, id: u32) -> ImagePixels;
        /// The last finished command's output as JSON {kind, text}.
        fn last_output(self: &Session) -> String;
        /// Notebook cells: their facts, as JSON.
        fn cells(self: &Session) -> String;
        /// A cell's output: format "html", "text" or "image" (base64).
        fn cell_output(self: &Session, id: u64, format: &str) -> String;
        /// The shell is at its prompt (from shell integration's marks).
        fn at_prompt(self: &Session) -> bool;

        /// The fd to watch for readability (its tesserad socket).
        fn fd(self: &Session) -> i64;
        /// Watched too: kept cells read in the background have come.
        fn wake_fd(self: &Session) -> i64;
        fn id(self: &Session) -> String;
        /// Read whatever is available. Returns SIG_* bits.
        fn on_readable(self: &mut Session) -> u32;
        /// JSON array of events for QML (title, folder, cells, ...).
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
            native_scan: u32,
        ) -> bool;
        /// Whether the program asked for every key, lone modifiers too (the
        /// kitty keyboard protocol's report-all flag).
        fn reports_all_keys(self: &Session) -> bool;
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
        fn clear_screen(self: &mut Session);
        fn clear_selection(self: &mut Session);
        fn title(self: &Session) -> String;
        fn pwd(self: &Session) -> String;
        fn frame(self: &mut Session, out: &mut Frame, force_full: bool) -> bool;
        fn focus(self: &mut Session, focused: bool);
        /// Ends the shell (closing a pane); dropping without this detaches.
        fn kill(self: &mut Session);
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
        /// Qt WebEngine (HTML output in notebook cells): before the app exists.
        fn tessera_webengine_initialize();
        fn tessera_show_windows();
        fn tessera_test_wheel(x: f64, y: f64, dy: i32, px: i32, phase: i32);
        fn tessera_test_mouse(kind: i32, x: f64, y: f64, button: i32, mods: i32);
        fn tessera_test_key(key: i32, mods: i32, text: &QString, scan: i32);
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
        /// A notebook named on the command line (tessera x.ipynb), or "".
        #[qinvokable]
        fn open_doc(self: &Controller) -> QString;
        /// Hand memory freed since back to the system (glibc keeps it
        /// otherwise), on a thread of its own.
        #[qinvokable]
        fn trim_memory(self: &Controller);

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
        fn session_job(self: &Controller, reply: &QString, id: &QString);
        /// {job, cwd, shell} of a session, as JSON.
        #[qinvokable]
        fn session_info(self: &Controller, reply: &QString, id: &QString);
        #[qinvokable]
        fn home_dir(self: &Controller) -> QString;
        /// A folder's entries for the notebook's file view: JSON
        /// [{name, path, dir, link, exec, hidden, size, mtime}] (folders first).
        #[qinvokable]
        fn list_dir(self: &Controller, reply: &QString, path: &QString, all: bool);
        /// A file browser's action on a path (src/files.rs): "open", "reveal",
        /// "trash", "rename" (arg: new name), "mkdir"/"touch" (arg: name),
        /// "duplicate", "copy"/"move" (arg: to folder). "" or the error.
        #[qinvokable]
        fn file_action(self: &Controller, reply: &QString, op: &QString, path: &QString, arg: &QString);
        /// A program on this computer's PATH: its path, or "" (in the background).
        #[qinvokable]
        fn find_program(self: &Controller, reply: &QString, name: &QString);
        /// The desktop's apps for a file's kind: {"mime", "apps": [{id, name,
        /// icon, default}]} (none for another host's file).
        #[qinvokable]
        fn file_apps(self: &Controller, reply: &QString, path: &QString);
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
        /// The shell's own syntax check (zsh -n, bash -n, fish -n, tessh
        /// --check), in the background: JSON [[start, length, message]]
        /// (UTF-16), [] when it parses.
        #[qinvokable]
        fn shell_syntax(self: &Controller, reply: &QString, shell: &QString, text: &QString);
        /// A command line laid out and colored for a card's details, as Qt
        /// rich text (src/cmdline.rs); argv_json: tessh's [[program, args...]]
        /// or "". Pure text work (no files, no processes).
        #[qinvokable]
        fn pretty_command(self: &Controller, line: &QString, argv_json: &QString, dark: bool, text_color: &QString) -> QString;
        /// The input editor's language for the program at the prompt.
        #[qinvokable]
        fn language_for(self: &Controller, program: &QString) -> QString;
        /// The languages the editor knows: JSON [[name, label]].
        #[qinvokable]
        fn syntax_languages(self: &Controller) -> QString;
        /// What a text is clearly written in ("" when it's just a command).
        #[qinvokable]
        fn detect_language(self: &Controller, text: &QString) -> QString;
        /// What a text is written in, by the Magika model (src/detect.rs), in
        /// the background: the tree-sitter name, "" when it can't say.
        #[qinvokable]
        fn detect_content(self: &Controller, reply: &QString, text: &QString);
        /// A help text's Markdown as Qt rich text, code highlighted
        /// (<span class="t-KIND">; manual::doc_html).
        #[qinvokable]
        fn doc_html(self: &Controller, text: &QString, lang: &QString) -> QString;
        /// The same for a file (by its name, Linguist's rules; then by its
        /// start and end on this computer, `head`, its first lines, on another).
        #[qinvokable]
        fn detect_file(self: &Controller, reply: &QString, path: &QString, head: &QString);
        /// Typed text that doesn't look like a shell command (src/snippets.rs).
        #[qinvokable]
        fn shell_doubt(self: &Controller, text: &QString, aliases: &QString) -> bool;
        /// The language from the AI's guesses, weighted by recent use ("" for the shell's).
        #[qinvokable]
        fn rank_languages(self: &Controller, reply: &QString, recent: &QString) -> QString;
        /// Every language code can be run in once (a snippet), [[name, shown]].
        #[qinvokable]
        fn runnable_languages(self: &Controller) -> QString;
        /// The shell command that runs code in a language ("" if it can't).
        #[qinvokable]
        fn snippet_command(self: &Controller, reply: &QString, lang: &QString, code: &QString, cwd: &QString, shell: &QString, host: &QString);
        /// Fetch every language guide now, for reading offline; progress in
        /// help_status ("…guides…").
        #[qinvokable]
        fn download_guides(self: Pin<&mut Controller>);
        /// Download Helix's grammars (~250 languages) for the editor;
        /// progress in help_status. (Used after the next start.)
        #[qinvokable]
        fn download_grammars(self: Pin<&mut Controller>);
        #[qinvokable]
        fn grammar_count(self: &Controller) -> i32;
        /// Make a language usable (unpack it from Helix's archive, fetching
        /// that once if need be), in the background: language_ready(name, error).
        #[qinvokable]
        fn ensure_language(self: Pin<&mut Controller>, name: &QString);
        #[qinvokable]
        fn language_installed(self: &Controller, name: &QString) -> bool;
        #[qsignal]
        fn language_ready(self: Pin<&mut Controller>, name: QString, error: QString);
        /// Input editor completions at a UTF-16 cursor (src/complete.rs), JSON.
        #[qinvokable]
        fn complete(self: &Controller, reply: &QString, text: &QString, cursor: i32, cwd: &QString, aliases: &QString, pid: i32, host: &QString);
        /// tessh's own completions (`tessh --complete`), in the background:
        /// the same {"start", "end", "items"} as complete.
        #[qinvokable]
        fn complete_tessh(self: &Controller, reply: &QString, text: &QString, cursor: i32, cwd: &QString);
        /// Download fish's completion files (more commands and subcommands
        /// get help text); progress in help_status.
        #[qinvokable]
        fn download_command_help(self: Pin<&mut Controller>);
        #[qinvokable]
        fn command_help_count(self: &Controller) -> i32;
        /// The prompt bar's facts about a folder (git, user, host), JSON, in
        /// the background: prompt_info_ready(key, json).
        #[qinvokable]
        fn request_prompt_info(self: Pin<&mut Controller>, key: &QString, cwd: &QString, pid: i32, host: &QString);
        #[qsignal]
        fn prompt_info_ready(self: Pin<&mut Controller>, key: QString, json: QString);
        #[qinvokable]
        fn has_starship(self: &Controller) -> bool;
        /// Whether Starship is on `host` (the pane's prompt is drawn there).
        #[qinvokable]
        fn has_starship_on(self: &Controller, reply: &QString, host: &QString);
        /// Ask Starship for the prompt (in the background); the answer comes
        /// as starship_ready(key, json) (see prompt::starship).
        #[qinvokable]
        fn request_starship(self: Pin<&mut Controller>, key: &QString, cwd: &QString, status: i32, duration_ms: f64, width: i32, pid: i32, host: &QString);
        #[qsignal]
        fn starship_ready(self: Pin<&mut Controller>, key: QString, json: QString);
        /// Another launch (`tessera -e CMD`) asks for a tab running CMD:
        /// JSON {cmd, cwd, hold}.
        #[qsignal]
        fn exec_requested(self: Pin<&mut Controller>, json: QString);
        /// The answer to a call that ran in the background (src/background.rs;
        /// qml/Async.qml matches `key` to its callback).
        #[qsignal]
        fn async_reply(self: Pin<&mut Controller>, key: QString, json: QString);
        /// The prompt bar's widgets (src/prompt_widgets.rs): every Starship
        /// module (JSON list; [] until Starship is there), the modules the
        /// prompt's format shows, and what clicking a widget offers.
        #[qinvokable]
        fn prompt_star_modules(self: &Controller) -> QString;
        #[qinvokable]
        fn prompt_format_modules(self: &Controller) -> QString;
        #[qinvokable]
        fn prompt_choices(self: &Controller, reply: &QString, widget: &QString, cwd: &QString, pid: i32, host: &QString);
        /// Draw Starship modules one by one, in the background:
        /// prompt_modules_ready(key, {name: runs}).
        #[qinvokable]
        fn request_prompt_modules(self: Pin<&mut Controller>, key: &QString, names: &QString, cwd: &QString, status: i32, duration_ms: f64, pid: i32, host: &QString);
        #[qsignal]
        fn prompt_modules_ready(self: Pin<&mut Controller>, key: QString, json: QString);
        /// The prompt builder (Preferences › Prompt): Starship's presets,
        /// Tessera's own config, the user's, module switches, previews.
        #[qinvokable]
        fn prompt_presets(self: &Controller) -> QString;
        #[qinvokable]
        fn prompt_preset(self: &Controller, name: &QString) -> QString;
        #[qinvokable]
        fn prompt_tessera_config(self: &Controller) -> QString;
        #[qinvokable]
        fn prompt_user_config(self: &Controller) -> QString;
        #[qinvokable]
        fn save_prompt_config(self: Pin<&mut Controller>, toml: &QString) -> QString;
        #[qinvokable]
        fn prompt_modules(self: &Controller, toml: &QString) -> QString;
        #[qinvokable]
        fn prompt_set_module(self: &Controller, toml: &QString, module: &QString, on: bool) -> QString;
        /// A preview of a config, in the background: starship_ready(key, json).
        #[qinvokable]
        fn request_prompt_preview(self: Pin<&mut Controller>, key: &QString, toml: &QString, cwd: &QString, width: i32);

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
        /// The assistant's saved conversations (src/chats.rs): newest first,
        /// [{id, title, updated, count}].
        #[qinvokable]
        fn chats_list(self: &Controller, reply: &QString);
        /// Conversations with every word of `query`, with a snippet:
        /// [{id, title, updated, count, snippet}].
        #[qinvokable]
        fn chats_search(self: &Controller, reply: &QString, query: &QString);
        /// One conversation's messages (JSON array; "[]" if there's none).
        #[qinvokable]
        fn chat_load(self: &Controller, reply: &QString, id: &QString);
        /// Keep a conversation (an empty one is removed).
        #[qinvokable]
        fn chat_save(self: &Controller, id: &QString, messages_json: &QString);
        #[qinvokable]
        fn chat_delete(self: &Controller, id: &QString);
        /// For tests: a wheel notch at a point in the window.
        #[qinvokable]
        fn test_wheel(self: &Controller, x: f64, y: f64, dy: i32, px: i32, phase: i32);
        /// For tests: a mouse press (0), move (1) or release (2) in the window.
        #[qinvokable]
        fn test_mouse(self: &Controller, kind: i32, x: f64, y: f64, button: i32, mods: i32);
        /// For tests: a key press and release (Qt key code, modifiers, text, native scan code).
        #[qinvokable]
        fn test_key(self: &Controller, key: i32, mods: i32, text: &QString, scan: i32);
        /// Ask the language server for the editor's language (src/lsp.rs), in
        /// the background: `what` "diagnostics", "hover" or "complete" at a
        /// UTF-16 cursor. The answer: lsp_ready(key, what, json).
        #[qinvokable]
        fn lsp_ask(self: Pin<&mut Controller>, key: &QString, lang: &QString, cwd: &QString, text: &QString, what: &QString, cursor: i32, host: &QString);
        #[qinvokable]
        fn lsp_available(self: &Controller, reply: &QString, lang: &QString, host: &QString);
        #[qsignal]
        fn lsp_ready(self: Pin<&mut Controller>, key: QString, what: QString, json: QString);
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

        // ---- Jupyter notebooks (src/jupyter.rs) ----
        /// The kernels there are: [{name, display, language}].
        #[qinvokable]
        fn jupyter_kernels(self: &Controller, reply: &QString, host: &QString);
        /// Another host's kernel list came (jupyter_kernels gave what it had,
        /// and asked there in the background).
        #[qsignal]
        fn kernels_ready(self: Pin<&mut Controller>, host: QString);
        /// The kernels running now (every notebook's), JSON (jupyter::running).
        #[qinvokable]
        fn jupyter_running(self: &Controller, reply: &QString);
        /// Start kernel `spec` for notebook `key` (one per key); it reports
        /// with jupyter_event(key, json).
        #[qinvokable]
        fn jupyter_start(self: Pin<&mut Controller>, key: &QString, spec: &QString, cwd: &QString, pid: i32, session: &QString);
        /// A notebook takes its kernels back (they ran on in tesserad while
        /// the window was closed): {kernels: [running]}; what they said
        /// meanwhile comes as jupyter_event.
        #[qinvokable]
        fn jupyter_attach(self: Pin<&mut Controller>, reply: &QString, key: &QString, session: &QString);
        /// A pane's notebook, kept by its host's tesserad with the session.
        #[qinvokable]
        fn notebook_read(self: &Controller, reply: &QString, host: &QString, session: &QString);
        #[qinvokable]
        fn notebook_write(self: &Controller, host: &QString, session: &QString, text: &QString);
        /// Files dropped on another host's pane: copied into `dir` there (scp,
        /// names not taken), in the background: files_uploaded(key, [paths
        /// there], error).
        #[qinvokable]
        fn upload_files(self: Pin<&mut Controller>, key: &QString, host: &QString, dir: &QString, paths: &QString);
        #[qsignal]
        fn files_uploaded(self: Pin<&mut Controller>, key: QString, paths: QString, error: QString);
        /// `host`'s home folder (its tesserad's HOME), for paths there.
        #[qinvokable]
        fn host_home(self: &Controller, reply: &QString, host: &QString);
        /// The user said to install tesserad on `host` (it had none): the next
        /// connect_host does.
        #[qinvokable]
        fn allow_tesserad_install(self: &Controller, host: &QString);
        /// The last error setting up the SSH connection to `host` ("" if none).
        #[qinvokable]
        fn host_error(self: &Controller, host: &QString) -> QString;
        /// Connect to `host`'s tesserad (SSH; deployed there when missing),
        /// in the background: host_connected(host, error) when done.
        #[qinvokable]
        fn connect_host(self: Pin<&mut Controller>, host: &QString);
        #[qsignal]
        fn host_connected(self: Pin<&mut Controller>, host: QString, error: QString);
        /// Bring a kernel up to a shell's environment and folder (src/jupyter.rs sync).
        #[qinvokable]
        fn jupyter_sync(self: &Controller, key: &QString, pid: i32, cwd: &QString);
        /// After a run in a shell's kernel: its folder and exports back to
        /// the pane's shell (jupyter::export_to_shell).
        #[qinvokable]
        fn jupyter_export(self: &Controller, key: &QString, pid: i32);
        /// The line that starts a shell in the pane's shell (with Tessera's
        /// integration), and handing a nested shell's changes back down.
        #[qinvokable]
        fn shell_launch_line(self: &Controller, reply: &QString, shell: &QString, parent: i32, host: &QString);
        #[qinvokable]
        fn shell_hand_back(self: &Controller, child: i32, parent: i32, host: &QString);
        /// The shells the one in front (pid) was started in: [{name, pid}].
        #[qinvokable]
        fn shell_chain(self: &Controller, reply: &QString, pid: i32, host: &QString);
        /// The completion panel's help for an item (complete::item_doc), read
        /// on `host` ("" here): a command's examples and man page, an option's
        /// paragraph, a subcommand's page; "" when there's none.
        #[qinvokable]
        fn item_doc(self: &Controller, reply: &QString, host: &QString, kind: &QString, label: &QString, line: &QString);
        /// A finished command tesserad kept for `session` ({"id", "status",
        /// "screen"}; "" while it isn't kept, i.e. still running): for a
        /// window that came back while it ran, and never saw it start.
        #[qinvokable]
        fn kept_cell(self: &Controller, reply: &QString, host: &QString, session: &QString, id: i32);
        /// What an exit status means for a command (complete::exit_meaning).
        #[qinvokable]
        fn exit_meaning(self: &Controller, reply: &QString, command: &QString, status: i32, host: &QString);
        /// Answer a kernel's input request (jupyter_event {"type": "input"}).
        #[qinvokable]
        fn jupyter_input(self: &Controller, key: &QString, value: &QString);
        #[qinvokable]
        fn jupyter_execute(self: &Controller, reply: &QString, key: &QString, cell: &QString, code: &QString);
        /// A background job request (src/jobs.rs: job.start, job.list,
        /// job.output, job.stop, job.forget) to `host`'s tesserad; the
        /// answer's JSON ({"error"} if it failed).
        #[qinvokable]
        fn job_call(self: &Controller, reply: &QString, host: &QString, method: &QString, params: &QString);
        /// A local process's user and environment (cells::Exec as JSON, "" if gone).
        #[qinvokable]
        fn process_exec(self: &Controller, reply: &QString, pid: i32);
        #[qinvokable]
        fn jupyter_complete(self: &Controller, key: &QString, request: &QString, code: &QString, pos: i32);
        #[qinvokable]
        fn jupyter_interrupt(self: &Controller, key: &QString);
        #[qinvokable]
        fn jupyter_stop(self: &Controller, key: &QString);
        #[qsignal]
        fn jupyter_event(self: Pin<&mut Controller>, key: QString, json: QString);
        /// A text file: {"text": ...} or {"error": ...}.
        #[qinvokable]
        fn read_text_file(self: &Controller, reply: &QString, path: &QString);
        /// A big file read a part at a time: its lines counted in the
        /// background ({"lines", "done", "size"}), and a range of them.
        #[qinvokable]
        fn file_line_index(self: &Controller, reply: &QString, path: &QString);
        #[qinvokable]
        fn file_lines(self: &Controller, reply: &QString, path: &QString, start: i32, count: i32);
        /// A CSV/TSV file as a table (tables.rs): read in the background
        /// ({"columns", "numeric", "rows", "done"}), a page of it by a spec,
        /// and let go.
        /// A file whole, to edit ({"text", "crlf", "error"}), and saved back.
        #[qinvokable]
        fn file_read_for_edit(self: &Controller, reply: &QString, path: &QString);
        #[qinvokable]
        fn file_save_edited(self: &Controller, reply: &QString, path: &QString, text: &QString, crlf: bool);
        #[qinvokable]
        fn table_open(self: &Controller, reply: &QString, path: &QString);
        #[qinvokable]
        fn table_query(self: &Controller, reply: &QString, path: &QString, spec: &QString);
        #[qinvokable]
        fn table_close(self: &Controller, path: &QString);
        /// Editing it: a cell, a row added or rows deleted, SQL that changes
        /// it, and saved back (dest "": the file itself).
        #[qinvokable]
        fn table_edit_cell(self: &Controller, reply: &QString, path: &QString, row: i64, column: &QString, value: &QString);
        #[qinvokable]
        fn table_add_row(self: &Controller, reply: &QString, path: &QString);
        #[qinvokable]
        fn table_delete_rows(self: &Controller, reply: &QString, path: &QString, rows: &QString);
        #[qinvokable]
        fn table_exec(self: &Controller, reply: &QString, path: &QString, sql: &QString);
        #[qinvokable]
        fn table_save(self: &Controller, reply: &QString, path: &QString, dest: &QString);
        /// Write a text file (atomically); "" or what went wrong.
        #[qinvokable]
        fn write_text_file(self: &Controller, reply: &QString, path: &QString, text: &QString);
        /// Text with ANSI colors as HTML (colors as var(--vt-palette-N)).
        #[qinvokable]
        fn ansi_html(self: &Controller, text: &QString) -> QString;
        /// Markdown as HTML.
        #[qinvokable]
        fn markdown_html(self: &Controller, text: &QString) -> QString;
        /// The tessh guide as a page, in the background: HTML, a <nav> and a <main>.
        #[qinvokable]
        fn tessh_manual_html(self: &Controller, reply: &QString);
        /// A language's guide as a page, in the background (fetched and kept
        /// the first time), written out with `head` (its style): the page's
        /// file URL, or "error: ...". (A file: loadHtml shows at most 2 MB.)
        #[qinvokable]
        fn guide_page(self: &Controller, reply: &QString, lang: &QString, refresh: bool, head: &QString);
        /// Every kept guide searched, in the background: an HTML page of the hits.
        #[qinvokable]
        fn guides_search(self: &Controller, reply: &QString, query: &QString);
        /// A language's guides when it has more than one: JSON [[key, title]].
        #[qinvokable]
        fn guide_choices(self: &Controller, lang: &QString) -> QString;
        /// What a guide being fetched is doing now ("" when nothing).
        #[qinvokable]
        fn guide_progress(self: &Controller, lang: &QString) -> QString;
        /// The languages with a guide: JSON {lang: title}.
        #[qinvokable]
        fn guide_list(self: &Controller) -> QString;
        #[qinvokable]
        fn ai_has_key(self: &Controller, provider: &QString) -> bool;
        /// A provider's models, in the background: ai_models_ready(provider, json list or "error: ...").
        #[qinvokable]
        fn ai_list_models(self: Pin<&mut Controller>, provider_json: &QString);
        #[qsignal]
        fn ai_models_ready(self: Pin<&mut Controller>, provider: QString, json: QString);
        /// Hearth's AI hub, in the background: ai_hearth_ready(json {running, default, fast}).
        #[qinvokable]
        fn ai_hearth_info(self: Pin<&mut Controller>);
        #[qsignal]
        fn ai_hearth_ready(self: Pin<&mut Controller>, json: QString);
        /// First run: set up Ollama if it's running here (background; settings change).
        #[qinvokable]
        fn ai_autodetect(self: Pin<&mut Controller>);
        /// Command history (Tessera's merged with the shells'), newest last, JSON array.
        #[qinvokable]
        fn history_all(self: &Controller, reply: &QString, limit: i32);
        /// {command: language} for history entries not in a shell, JSON.
        #[qinvokable]
        fn history_langs(self: &Controller, reply: &QString, limit: i32);
        #[qinvokable]
        fn history_add(self: &Controller, cmd: &QString, cwd: &QString, lang: &QString);
        /// History matches for the dropdown (completion JSON).
        /// `lang`: the editor's language, for entries that don't say theirs.
        #[qinvokable]
        fn history_search(self: &Controller, reply: &QString, query: &QString, lang: &QString);
        /// How the running daemon compared with this build at startup:
        /// "none", "current", "upgraded", "waiting", "outdated", "newer".
        #[qinvokable]
        fn daemon_state(self: &Controller) -> QString;
        /// End an outdated daemon (and its shells). False if it couldn't be.
        #[qinvokable]
        fn stop_daemon(self: &Controller, reply: &QString);
        /// Bring `host`'s tesserad, an older build that hasn't swapped itself
        /// in (kernels kept it), up to this one: its kernels stopped, then the
        /// upgrade. {"ok"} or {"ok": false, "error"}.
        #[qinvokable]
        fn update_daemon(self: &Controller, reply: &QString, host: &QString);
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
        fn restorable_sessions(self: &Controller, reply: &QString, host: &QString);

        /// Where to save a screenshot and quit (--screenshot), or empty.
        #[qinvokable]
        fn screenshot_path(self: &Controller) -> QString;

        /// Text typed into the first terminal at startup (--run), or empty.
        #[qinvokable]
        fn startup_command(self: &Controller) -> QString;
        /// A command this window was started to run in a tab (`-e`), JSON
        /// {cmd, cwd, hold}, or empty.
        #[qinvokable]
        fn exec_request(self: &Controller) -> QString;
        /// Take commands from later `tessera -e` launches (execRequested).
        #[qinvokable]
        fn start_gui_link(self: &Controller);
        /// This window was just used: later `tessera -e` launches come here.
        #[qinvokable]
        fn gui_link_touch(self: &Controller);
        /// The xdg-activation token the next requestActivate() uses (a
        /// `tessera -e` launcher's), or "" to clear it. Qt's Wayland window
        /// takes it from XDG_ACTIVATION_TOKEN when asked to activate.
        #[qinvokable]
        fn set_activation_token(self: &Controller, token: QString);

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
    /// Run `f` in the background (`ordered`: after the calls queued before
    /// it) and send what it returns back as async_reply(key, ...).
    fn answer(&self, key: &ffi::QString, ordered: bool, f: impl FnOnce() -> String + Send + 'static) {
        self.answer_on(key, if ordered { Some("kernel") } else { None }, f)
    }
    /// The same on a lane of its own (None: a thread of its own).
    fn answer_on(&self, key: &ffi::QString, lane: Option<&'static str>, f: impl FnOnce() -> String + Send + 'static) {
        let key = key.to_string();
        let qt = self.qt_thread();
        let job = move || {
            let r = f();
            let _ = qt.queue(move |c| c.async_reply(qs(&key), qs(&r)));
        };
        match lane {
            Some(l) => crate::background::in_order_on(l, job),
            None => crate::background::spawn(job),
        }
    }
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

    fn session_job(&self, reply: &ffi::QString, id: &ffi::QString) {
        let id = id.to_string();
        self.answer_on(reply, None, move || crate::client::session_job(&id).unwrap_or_default())
    }

    fn session_info(&self, reply: &ffi::QString, id: &ffi::QString) {
        let id = id.to_string();
        self.answer_on(reply, None, move || serde_json::to_string(&crate::client::proc_info(&id).unwrap_or_default()).unwrap_or_default())
    }

    fn list_dir(&self, reply: &ffi::QString, path: &ffi::QString, all: bool) {
        let path = path.to_string();
        self.answer_on(reply, None, move || on_host(&path, "list", serde_json::json!({"all": all}), |p| crate::files::list(p, all)))
    }

    fn file_action(&self, reply: &ffi::QString, op: &ffi::QString, path: &ffi::QString, arg: &ffi::QString) {
        let (op, path, arg) = (op.to_string(), path.to_string(), arg.to_string());
        self.answer_on(reply, Some("files"), move || file_action_now(op, &path, arg))
    }

    fn find_program(&self, reply: &ffi::QString, name: &ffi::QString) {
        let name = name.to_string();
        self.answer_on(reply, Some("files"), move || {
            std::env::var_os("PATH")
                .and_then(|p| std::env::split_paths(&p).map(|d| d.join(&name)).find(|f| f.is_file()))
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default()
        })
    }

    fn file_apps(&self, reply: &ffi::QString, path: &ffi::QString) {
        let path = path.to_string();
        self.answer_on(reply, Some("files"), move || if host_path(&path).0.is_empty() { crate::files::apps_for(&path) } else { "{\"apps\": []}".into() })
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

    fn complete_tessh(&self, reply: &ffi::QString, text: &ffi::QString, cursor: i32, cwd: &ffi::QString) {
        let (text, cwd) = (text.to_string(), cwd.to_string());
        // (Asks that a newer one has replaced are skipped, not worked out:
        // the editor only takes the latest.)
        static LATEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let me = LATEST.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        self.answer_on(reply, Some("complete"), move || {
            let none = r#"{"start":0,"end":0,"items":[]}"#.to_string();
            if LATEST.load(std::sync::atomic::Ordering::SeqCst) != me {
                return none;
            }
            let dir = if std::path::Path::new(&cwd).is_dir() { cwd } else { std::env::var("HOME").unwrap_or_else(|_| "/".into()) };
            crate::syntax::tessh_complete(&serde_json::json!({"text": text, "cursor": cursor.max(0), "cwd": dir})).unwrap_or(none)
        })
    }

    fn complete(&self, reply: &ffi::QString, text: &ffi::QString, cursor: i32, cwd: &ffi::QString, aliases: &ffi::QString, pid: i32, host: &ffi::QString) {
        let aliases: serde_json::Value = serde_json::from_str(&aliases.to_string()).unwrap_or_default();
        let cwd = cwd.to_string();
        // An OSC 7 URL (file://host/path) or a plain path.
        let cwd = match cwd.strip_prefix("file://") {
            Some(rest) => percent_decode(&rest[rest.find('/').unwrap_or(rest.len())..]),
            None => cwd,
        };
        // (Where the pane's shell is: its commands, files, Nix and help. Not
        // answered in time over SSH: "pending", and the editor asks again.)
        let params = serde_json::json!({"text": text.to_string(), "cursor": cursor.max(0), "cwd": cwd, "aliases": aliases, "pid": pid.max(0)});
        let host = host.to_string();
        self.answer_on(reply, None, move || match crate::client::rpc_within(&host, "complete", params, std::time::Duration::from_millis(400)) {
            Ok(v) => v.to_string(),
            Err(_) => r#"{"start":0,"end":0,"items":[],"pending":true}"#.to_string(),
        })
    }

    fn download_command_help(mut self: Pin<&mut Self>) {
        self.as_mut().set_help_status(qs("Downloading fish's completions and tldr-pages\u{2026}"));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            // (Kept where the completions are worked out: this computer's tesserad.)
            let r = crate::client::rpc_within("", "complete.download", serde_json::json!({}), std::time::Duration::from_secs(900));
            let _ = qt.queue(move |mut c| {
                let msg = match r {
                    Ok(v) if v["error"].is_null() => format!("Up to date: completions for {} commands (fish-shell), examples for {} (tldr-pages).", v["fish"], v["tldr"]),
                    Ok(v) => format!("Couldn't download: {}", v["error"].as_str().unwrap_or("")),
                    Err(e) => format!("Couldn't download: {e}"),
                };
                c.as_mut().set_help_status(qs(&msg));
            });
        });
    }

    fn daemon_state(&self) -> ffi::QString {
        qs(&std::env::var("TESSERA_DAEMON_STATE").unwrap_or_default())
    }

    fn update_daemon(&self, reply: &ffi::QString, host: &ffi::QString) {
        let host = host.to_string();
        self.answer(reply, false, move || {
            use serde_json::json;
            // (A build that knows it has a newer one waiting stops its kernels
            // and jobs itself and swaps in; one from before, its kernels
            // stopped here, is sent this build.)
            let knows = crate::client::rpc(&host, "daemon.upgrade", json!({})).is_ok();
            if !knows {
                let kernels = crate::client::rpc(&host, "kernel.running", json!({})).unwrap_or_default();
                for k in kernels.as_array().into_iter().flatten().filter_map(|k| k["key"].as_str()) {
                    let _ = crate::client::rpc(&host, "kernel.stop", json!({"key": k}));
                }
            }
            if !host.is_empty() {
                std::thread::sleep(std::time::Duration::from_secs(4));
                return json!({"ok": true}).to_string();
            }
            let start = std::time::Instant::now();
            while start.elapsed() < std::time::Duration::from_secs(20) {
                if crate::client::daemon_current() {
                    return json!({"ok": true}).to_string();
                }
                if !knows && crate::client::sync_daemon() == "outdated" {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            json!({"ok": false, "error": "tesserad didn't update"}).to_string()
        });
    }

    fn stop_daemon(&self, reply: &ffi::QString) {
        self.answer_on(reply, None, move || crate::client::stop_daemon().to_string())
    }

    fn history_all(&self, reply: &ffi::QString, limit: i32) {
        self.answer_on(reply, None, move || crate::history::all_json(limit.max(1) as usize))
    }

    fn history_langs(&self, reply: &ffi::QString, limit: i32) {
        self.answer_on(reply, None, move || crate::history::langs_json(limit.max(1) as usize))
    }

    fn history_add(&self, cmd: &ffi::QString, cwd: &ffi::QString, lang: &ffi::QString) {
        let (cmd, cwd, lang) = (cmd.to_string(), cwd.to_string(), lang.to_string());
        let cwd = match cwd.strip_prefix("file://") {
            Some(rest) => percent_decode(&rest[rest.find('/').unwrap_or(rest.len())..]),
            None => cwd,
        };
        crate::background::in_order_on("history", move || crate::history::add(&cmd, &cwd, &lang));
    }

    fn jupyter_running(&self, reply: &ffi::QString) {
        self.answer(reply, false, move || {
            // (Jupyter's kernels and Tessera's shell kernels: this computer's
            // tesserad's, then each connected host's, "host//key" with its
            // host. A host's list is what it said last, asked again in the
            // background: this is asked every few seconds.)
            let mut all: Vec<serde_json::Value> = crate::client::rpc("", "kernel.running", serde_json::json!({})).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
            static REMOTE: std::sync::Mutex<Option<std::collections::HashMap<String, Vec<serde_json::Value>>>> = std::sync::Mutex::new(None);
            static ASKING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            let hosts = crate::remote::connected_hosts();
            if !ASKING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                let hosts = hosts.clone();
                std::thread::spawn(move || {
                    let mut got = std::collections::HashMap::new();
                    for h in hosts {
                        let list = crate::client::rpc_within(&h, "kernel.running", serde_json::json!({}), std::time::Duration::from_secs(5)).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default();
                        got.insert(h, list);
                    }
                    *REMOTE.lock().unwrap() = Some(got);
                    ASKING.store(false, std::sync::atomic::Ordering::SeqCst);
                });
            }
            if let Some(m) = REMOTE.lock().unwrap().as_ref() {
                for h in &hosts {
                    for k in m.get(h).into_iter().flatten() {
                        let mut k = k.clone();
                        if let Some(key) = k["key"].as_str().map(String::from) {
                            k["key"] = serde_json::Value::String(format!("{h}//{key}"));
                        }
                        k["host"] = serde_json::Value::String(h.clone());
                        all.push(k);
                    }
                }
            }
            serde_json::Value::Array(all).to_string()
        })
    }

    fn jupyter_kernels(&self, reply: &ffi::QString, host: &ffi::QString) {
        let host = host.to_string();
        if host.is_empty() {
            return self.answer(reply, false, || crate::client::rpc("", "kernel.specs", serde_json::json!({})).unwrap_or_else(|_| serde_json::json!([])).to_string());
        }
        // (Another host's: looking for its kernels can take a while there,
        // so never on this thread. What it said last, and asked again.)
        type Cache = std::sync::Mutex<std::collections::HashMap<String, (serde_json::Value, bool)>>;
        static CACHE: std::sync::OnceLock<Cache> = std::sync::OnceLock::new();
        let cache = CACHE.get_or_init(Default::default);
        let mut c = cache.lock().unwrap();
        let entry = c.entry(host.clone()).or_insert((serde_json::json!([]), false));
        let have = entry.0.to_string();
        if !entry.1 {
            entry.1 = true;
            let qt = self.qt_thread();
            std::thread::spawn(move || {
                let got = crate::client::rpc(&host, "kernel.specs", serde_json::json!({}));
                let cache = CACHE.get().unwrap();
                let mut c = cache.lock().unwrap();
                let e = c.entry(host.clone()).or_insert((serde_json::json!([]), false));
                e.1 = false;
                if let Ok(v) = got {
                    let changed = e.0 != v;
                    e.0 = v;
                    if changed {
                        let _ = qt.queue(move |c| c.kernels_ready(qs(&host)));
                    }
                }
            });
        }
        drop(c);
        self.answer(reply, false, move || have)
    }

    fn jupyter_start(self: Pin<&mut Self>, key: &ffi::QString, spec: &ffi::QString, cwd: &ffi::QString, pid: i32, session: &ffi::QString) {
        ensure_kernel_events(&self);
        let (key, spec, cwd, session) = (key.to_string(), spec.to_string(), local_path(cwd), session.to_string());
        // (Over SSH it can take a moment: not on the Qt thread.)
        std::thread::spawn(move || {
            crate::client::start_kernel(&key, &spec, &cwd, pid.max(0) as u32, &session);
        });
    }

    fn jupyter_attach(self: Pin<&mut Self>, reply: &ffi::QString, key: &ffi::QString, session: &ffi::QString) {
        ensure_kernel_events(&self);
        let (key, session) = (key.to_string(), session.to_string());
        self.answer(reply, true, move || crate::client::attach_kernels(&key, &session).to_string())
    }

    // (In order: a read sees the writes queued before it.)
    fn notebook_read(&self, reply: &ffi::QString, host: &ffi::QString, session: &ffi::QString) {
        let (host, session) = (host.to_string(), session.to_string());
        self.answer(reply, true, move || {
            let r = crate::client::rpc_read(&host, "notebook.read", serde_json::json!({"session": session}));
            r.unwrap_or_else(|e| serde_json::json!({"error": e})).to_string()
        });
    }

    fn notebook_write(&self, host: &ffi::QString, session: &ffi::QString, text: &ffi::QString) {
        let (host, session, text) = (host.to_string(), session.to_string(), text.to_string());
        crate::background::in_order(move || { let _ = crate::client::rpc(&host, "notebook.write", serde_json::json!({"session": session, "text": text})); });
    }

    fn upload_files(self: Pin<&mut Self>, key: &ffi::QString, host: &ffi::QString, dir: &ffi::QString, paths: &ffi::QString) {
        let qt = self.qt_thread();
        let (key, host, dir) = (key.to_string(), host.to_string(), dir.to_string());
        let paths: Vec<String> = serde_json::from_str(&paths.to_string()).unwrap_or_default();
        std::thread::spawn(move || {
            let mut done = Vec::new();
            let mut err = String::new();
            for p in paths {
                match crate::remote::upload(&host, std::path::Path::new(&p), &dir) {
                    Ok(there) => done.push(there),
                    Err(e) => {
                        err = e;
                        break;
                    }
                }
            }
            let j = serde_json::to_string(&done).unwrap_or_default();
            let _ = qt.queue(move |c| c.files_uploaded(qs(&key), qs(&j), qs(&err)));
        });
    }

    fn host_home(&self, reply: &ffi::QString, host: &ffi::QString) {
        // (Asked once a host; "" until it's connected: no ~ for now.)
        let host = host.to_string();
        self.answer_on(reply, None, move || crate::client::host_home(&host))
    }

    fn allow_tesserad_install(&self, host: &ffi::QString) {
        crate::remote::allow_install(&host.to_string());
    }

    fn host_error(&self, host: &ffi::QString) -> ffi::QString {
        qs(&crate::remote::last_error(&host.to_string()))
    }

    fn connect_host(self: Pin<&mut Self>, host: &ffi::QString) {
        let qt = self.qt_thread();
        let host = host.to_string();
        std::thread::spawn(move || {
            let err = crate::remote::connect(&host).err().map(|e| e.to_string()).unwrap_or_default();
            let _ = qt.queue(move |c| c.host_connected(qs(&host), qs(&err)));
        });
    }

    // (Kernel calls go in order, after the ones before: a sync lands
    // before the execute that follows it.)
    fn jupyter_input(&self, key: &ffi::QString, value: &ffi::QString) {
        let (host, k) = crate::client::split_key(&key.to_string());
        let value = value.to_string();
        crate::background::in_order(move || { let _ = crate::client::rpc(&host, "kernel.input", serde_json::json!({"key": k, "value": value})); });
    }

    fn jupyter_sync(&self, key: &ffi::QString, pid: i32, cwd: &ffi::QString) {
        let (host, k) = crate::client::split_key(&key.to_string());
        let cwd = local_path(cwd);
        crate::background::in_order(move || { let _ = crate::client::rpc(&host, "kernel.sync", serde_json::json!({"key": k, "pid": pid.max(0), "cwd": cwd})); });
    }

    fn item_doc(&self, reply: &ffi::QString, host: &ffi::QString, kind: &ffi::QString, label: &ffi::QString, line: &ffi::QString) {
        let (host, kind, label, line) = (host.to_string(), kind.to_string(), label.to_string(), line.to_string());
        self.answer_on(reply, Some("doc"), move || {
            if host.is_empty() {
                return crate::complete::item_doc(&kind, &label, &line);
            }
            let r = crate::client::rpc_within(&host, "complete.doc", serde_json::json!({"kind": kind, "label": label, "line": line}), std::time::Duration::from_secs(10));
            r.ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
        })
    }

    fn kept_cell(&self, reply: &ffi::QString, host: &ffi::QString, session: &ffi::QString, id: i32) {
        let (host, session) = (host.to_string(), session.to_string());
        self.answer_on(reply, None, move || {
            let r = crate::client::rpc_within(&host, "cells.load", serde_json::json!({"session": session}), std::time::Duration::from_secs(5));
            let text = r.ok().as_ref().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
            crate::cells::kept_summary(&text, id.max(0) as u64)
        })
    }

    fn exit_meaning(&self, reply: &ffi::QString, command: &ffi::QString, status: i32, host: &ffi::QString) {
        let (command, host) = (command.to_string(), host.to_string());
        self.answer_on(reply, None, move || {
            let r = crate::client::rpc_within(&host, "complete.exit_meaning", serde_json::json!({"command": command, "status": status}), std::time::Duration::from_secs(3));
            r.ok().as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string()
        })
    }

    fn shell_launch_line(&self, reply: &ffi::QString, shell: &ffi::QString, parent: i32, host: &ffi::QString) {
        let (shell, host) = (shell.to_string(), host.to_string());
        self.answer_on(reply, Some("kernel"), move || {
            let r = crate::client::rpc(&host, "shell.launch_line", serde_json::json!({"shell": shell, "parent": parent.max(0)}));
            r.ok().as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string()
        })
    }

    fn shell_chain(&self, reply: &ffi::QString, pid: i32, host: &ffi::QString) {
        let host = host.to_string();
        self.answer_on(reply, Some("kernel"), move || {
            let r = crate::client::rpc(&host, "shell.chain", serde_json::json!({"pid": pid.max(0)}));
            r.ok().as_ref().and_then(|v| v.as_str()).unwrap_or("[]").to_string()
        })
    }

    fn shell_hand_back(&self, child: i32, parent: i32, host: &ffi::QString) {
        let host = host.to_string();
        crate::background::in_order(move || { let _ = crate::client::rpc(&host, "shell.hand_back", serde_json::json!({"child": child.max(0), "parent": parent.max(0)})); });
    }

    fn jupyter_export(&self, key: &ffi::QString, pid: i32) {
        // (Tessera's shell kernels hand back by themselves, after each run.)
        let (host, k) = crate::client::split_key(&key.to_string());
        crate::background::in_order(move || { let _ = crate::client::rpc(&host, "kernel.export", serde_json::json!({"key": k, "pid": pid.max(0)})); });
    }

    /// Answers `reply` with whether the kernel took it (true/false).
    fn job_call(&self, reply: &ffi::QString, host: &ffi::QString, method: &ffi::QString, params: &ffi::QString) {
        let (host, method) = (host.to_string(), method.to_string());
        let params: serde_json::Value = serde_json::from_str(&params.to_string()).unwrap_or_default();
        self.answer(reply, false, move || {
            if !method.starts_with("job.") {
                return serde_json::json!({"error": "not a job request"}).to_string();
            }
            // (A signal to what runs in a local pane: sent from here. tesserad
            // holds off upgrading while a notebook kernel runs, so an older
            // one may not know the request yet.)
            if method == "job.signal_front" && host.is_empty() {
                let pid = params["pid"].as_u64().unwrap_or(0) as u32;
                return serde_json::json!(crate::procinfo::signal_foreground(pid, params["sig"].as_str().unwrap_or(""))).to_string();
            }
            match crate::client::rpc(&host, &method, params) {
                Ok(v) => v.to_string(),
                Err(e) => serde_json::json!({"error": e}).to_string(),
            }
        });
    }

    fn process_exec(&self, reply: &ffi::QString, pid: i32) {
        self.answer(reply, false, move || crate::cells::Exec::of_process(pid.max(0) as u32).map(|e| e.to_json().to_string()).unwrap_or_default());
    }

    fn jupyter_execute(&self, reply: &ffi::QString, key: &ffi::QString, cell: &ffi::QString, code: &ffi::QString) {
        let (host, k) = crate::client::split_key(&key.to_string());
        let (cell, code) = (cell.to_string(), code.to_string());
        self.answer(reply, true, move || {
            let ok = crate::client::rpc(&host, "kernel.execute", serde_json::json!({"key": k, "cell": cell, "code": code})).ok().and_then(|v| v.as_bool()).unwrap_or(false);
            ok.to_string()
        });
    }

    fn jupyter_complete(&self, key: &ffi::QString, request: &ffi::QString, code: &ffi::QString, pos: i32) {
        // (The editor counts UTF-16 units; the kernel, code points.)
        let code = code.to_string();
        let mut units = 0usize;
        let mut chars = 0usize;
        for ch in code.chars() {
            if units >= pos.max(0) as usize {
                break;
            }
            units += ch.len_utf16();
            chars += 1;
        }
        let (host, k) = crate::client::split_key(&key.to_string());
        let request = request.to_string();
        crate::background::in_order(move || { let _ = crate::client::rpc(&host, "kernel.complete", serde_json::json!({"key": k, "request": request, "code": code, "pos": chars})); });
    }

    fn jupyter_interrupt(&self, key: &ffi::QString) {
        let (host, k) = crate::client::split_key(&key.to_string());
        crate::background::in_order(move || { let _ = crate::client::rpc(&host, "kernel.interrupt", serde_json::json!({"key": k})); });
    }

    fn jupyter_stop(&self, key: &ffi::QString) {
        let key = key.to_string();
        crate::client::release_kernel(&key);
        crate::background::in_order(move || {
            let (host, k) = crate::client::split_key(&key);
            let _ = crate::client::rpc(&host, "kernel.stop", serde_json::json!({"key": k}));
        });
    }

    fn file_read_for_edit(&self, reply: &ffi::QString, path: &ffi::QString) {
        let path = path.to_string();
        self.answer_on(reply, None, move || on_host(&path, "read_for_edit", serde_json::json!({}), |p| crate::files::read_for_edit(p)))
    }

    fn file_save_edited(&self, reply: &ffi::QString, path: &ffi::QString, text: &ffi::QString, crlf: bool) {
        let path = path.to_string();
        let text = text.to_string();
        self.answer_on(reply, Some("files"), move || on_host(&path, "save_edited", serde_json::json!({"text": text, "crlf": crlf}), |p| crate::files::save_edited(p, &text, crlf)))
    }

    fn table_open(&self, reply: &ffi::QString, path: &ffi::QString) {
        let path = path.to_string();
        self.answer_on(reply, Some("files"), move || on_host(&path, "table_open", serde_json::json!({}), |p| crate::tables::open(p)))
    }

    fn table_query(&self, reply: &ffi::QString, path: &ffi::QString, spec: &ffi::QString) {
        let path = path.to_string();
        let spec = spec.to_string();
        self.answer_on(reply, Some("files"), move || on_host(&path, "table_query", serde_json::json!({"spec": spec}), |p| crate::tables::query(p, &spec)))
    }

    fn table_edit_cell(&self, reply: &ffi::QString, path: &ffi::QString, row: i64, column: &ffi::QString, value: &ffi::QString) {
        let path = path.to_string();
        let (column, value) = (column.to_string(), value.to_string());
        self.answer_on(reply, Some("files"), move || on_host(&path, "table_edit_cell", serde_json::json!({"row": row, "column": column, "value": value}), |p| crate::tables::edit_cell(p, row, &column, &value)))
    }

    fn table_add_row(&self, reply: &ffi::QString, path: &ffi::QString) {
        let path = path.to_string();
        self.answer_on(reply, Some("files"), move || on_host(&path, "table_add_row", serde_json::json!({}), |p| crate::tables::add_row(p)))
    }

    fn table_delete_rows(&self, reply: &ffi::QString, path: &ffi::QString, rows: &ffi::QString) {
        let path = path.to_string();
        let rows = rows.to_string();
        self.answer_on(reply, Some("files"), move || on_host(&path, "table_delete_rows", serde_json::json!({"rows": rows}), |p| crate::tables::delete_rows(p, &rows)))
    }

    fn table_exec(&self, reply: &ffi::QString, path: &ffi::QString, sql: &ffi::QString) {
        let path = path.to_string();
        let sql = sql.to_string();
        self.answer_on(reply, Some("files"), move || on_host(&path, "table_exec", serde_json::json!({"sql": sql}), |p| crate::tables::exec(p, &sql)))
    }

    fn table_save(&self, reply: &ffi::QString, path: &ffi::QString, dest: &ffi::QString) {
        let path = path.to_string();
        // (Saved where the table is: a destination on its host.)
        let d = if dest.to_string().is_empty() { String::new() } else { host_path(&dest.to_string()).1 };
        self.answer_on(reply, Some("files"), move || on_host(&path, "table_save", serde_json::json!({"dest": d}), |p| crate::tables::save(p, &d)))
    }

    fn table_close(&self, path: &ffi::QString) {
        let path = path.to_string();
        crate::background::in_order_on("files", move || {
            let _ = on_host(&path, "table_close", serde_json::json!({}), |p| {
                crate::tables::close(p);
                String::new()
            });
        });
    }

    fn file_line_index(&self, reply: &ffi::QString, path: &ffi::QString) {
        let path = path.to_string();
        self.answer_on(reply, None, move || on_host(&path, "line_index", serde_json::json!({}), |p| crate::files::line_index(p)))
    }

    fn file_lines(&self, reply: &ffi::QString, path: &ffi::QString, start: i32, count: i32) {
        let path = path.to_string();
        let (start, count) = (start.max(0) as usize, count.max(0) as usize);
        self.answer_on(reply, None, move || on_host(&path, "read_lines", serde_json::json!({"start": start, "count": count}), |p| crate::files::read_lines(p, start, count)))
    }

    fn read_text_file(&self, reply: &ffi::QString, path: &ffi::QString) {
        let path = path.to_string();
        self.answer_on(reply, None, move || on_host(&path, "read_text", serde_json::json!({}), |p| crate::files::read_text(p)))
    }

    fn write_text_file(&self, reply: &ffi::QString, path: &ffi::QString, text: &ffi::QString) {
        let path = path.to_string();
        let text = text.to_string();
        self.answer_on(reply, Some("files"), move || on_host(&path, "write_text", serde_json::json!({"text": text}), |p| crate::files::write_text(p, &text)))
    }

    fn ansi_html(&self, text: &ffi::QString) -> ffi::QString {
        qs(&crate::jupyter::ansi_to_html(&text.to_string()))
    }

    fn markdown_html(&self, text: &ffi::QString) -> ffi::QString {
        qs(&crate::cells::markdown_html(&text.to_string()))
    }

    fn tessh_manual_html(&self, reply: &ffi::QString) {
        self.answer_on(reply, None, crate::cells::tessh_manual_html)
    }

    fn guide_page(&self, reply: &ffi::QString, lang: &ffi::QString, refresh: bool, head: &ffi::QString) {
        let (lang, head) = (lang.to_string(), head.to_string());
        self.answer_on(reply, None, move || crate::guides::page_file(&lang, refresh, &head).unwrap_or_else(|e| format!("error: {e}")))
    }

    fn guides_search(&self, reply: &ffi::QString, query: &ffi::QString) {
        let query = query.to_string();
        self.answer_on(reply, None, move || crate::guides::search_html(&query))
    }

    fn guide_choices(&self, lang: &ffi::QString) -> ffi::QString {
        qs(&crate::guides::choices_json(&lang.to_string()))
    }

    fn guide_progress(&self, lang: &ffi::QString) -> ffi::QString {
        qs(&crate::guides::progress(&lang.to_string()))
    }

    fn guide_list(&self) -> ffi::QString {
        qs(&crate::guides::list_json())
    }

    fn history_search(&self, reply: &ffi::QString, query: &ffi::QString, lang: &ffi::QString) {
        let (query, lang) = (query.to_string(), lang.to_string());
        self.answer_on(reply, None, move || crate::history::search(&query, &lang))
    }


    fn request_prompt_info(self: Pin<&mut Self>, key: &ffi::QString, cwd: &ffi::QString, pid: i32, host: &ffi::QString) {
        let (key, cwd, host) = (key.to_string(), local_path(cwd), host.to_string());
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let r = crate::client::rpc_within(&host, "prompt.info", serde_json::json!({"cwd": cwd, "pid": pid.max(0)}), std::time::Duration::from_secs(5));
            let json = r.map(|v| v.to_string()).unwrap_or_else(|_| "{}".into());
            let _ = qt.queue(move |c| c.prompt_info_ready(qs(&key), qs(&json)));
        });
    }

    fn ai_run(self: Pin<&mut Self>, key: &ffi::QString, task: &ffi::QString, messages_json: &ffi::QString, ctx_json: &ffi::QString) {
        use std::sync::atomic::{AtomicBool, Ordering};
        let (key, task) = (key.to_string(), task.to_string());
        let mut msgs: Vec<serde_json::Value> = serde_json::from_str(&messages_json.to_string()).unwrap_or_default();
        let ctx: serde_json::Value = serde_json::from_str(&ctx_json.to_string()).unwrap_or_default();
        let s = crate::settings::load();
        let (rtask, model_ref) = crate::ai::task_model(&s.ai, &task);
        // (A block being typed: its last line is completed, the rest is context.)
        let block = if task == "inline" && !crate::ai::is_completion_model(&model_ref) {
            msgs.last().and_then(|m| m["content"].as_str()).and_then(crate::ai::inline_block)
        } else {
            None
        };
        if let (Some((ask, _)), Some(m)) = (&block, msgs.last_mut()) {
            m["content"] = serde_json::json!(ask);
        }
        let qt = self.qt_thread();
        let cancel = Arc::new(AtomicBool::new(false));
        if let Some(old) = ai_jobs().lock().unwrap().insert(key.clone(), cancel.clone()) {
            old.store(true, Ordering::Relaxed);
        }
        msgs.insert(0, serde_json::json!({"role": "system", "content": crate::ai::system_prompt(&task, &ctx)}));
        // (tessh is new to every model: its condensed guide goes first, the
        // same text each time. Suggestions for plain commands go without.)
        if ctx["shell"] == "tessh" {
            let typed = match &block {
                Some((_, t)) => t.as_str(),
                None => msgs.last().and_then(|m| m["content"].as_str()).and_then(|c| c.rsplit_once("Complete this command line: ")).map(|(_, t)| t).unwrap_or(""),
            };
            if task != "inline" || crate::ai::looks_tessh(typed) {
                msgs.insert(0, serde_json::json!({"role": "system", "content": crate::ai::tessh_primer()}));
            }
        }
        let opts = match task.as_str() {
            "inline" | "request" => crate::ai::Options { max_tokens: 80, temperature: 0.1, timeout: std::time::Duration::from_secs(8) },
            "classify" => crate::ai::Options { max_tokens: 8, temperature: 0.0, timeout: std::time::Duration::from_secs(10) },
            "language" => crate::ai::Options { max_tokens: 60, temperature: 0.0, timeout: std::time::Duration::from_secs(8) },
            "exit" => crate::ai::Options { max_tokens: 60, temperature: 0.1, timeout: std::time::Duration::from_secs(20) },
            "check" => crate::ai::Options { max_tokens: 80, temperature: 0.1, timeout: std::time::Duration::from_secs(15) },
            "command" | "fix" => crate::ai::Options { max_tokens: 300, temperature: 0.2, timeout: std::time::Duration::from_secs(60) },
            _ => crate::ai::Options { max_tokens: 4096, temperature: 0.4, timeout: std::time::Duration::from_secs(600) },
        };
        std::thread::spawn(move || {
            // (Hearth's hub or the set model: decided here, off the UI thread.)
            let (provider, model, fallback) = match crate::ai::route(&s.ai, &rtask, &model_ref) {
                Ok(r) => r,
                Err(e) => {
                    {
                        let mut jobs = ai_jobs().lock().unwrap();
                        if jobs.get(&key).is_some_and(|c| Arc::ptr_eq(c, &cancel)) {
                            jobs.remove(&key);
                        }
                    }
                    let _ = qt.queue(move |c| c.ai_done(qs(&key), qs(""), qs(&e)));
                    return;
                }
            };
            if std::env::var_os("TESSERA_AI_DEBUG").is_some() {
                eprintln!("tessera ai: {task} with {}/{model} ({} messages)", provider.id, msgs.len());
            }
            let stream = task == "chat";
            let q2 = qt.clone();
            let k2 = key.clone();
            let send = |piece: &str| {
                if stream {
                    let (k, p) = (k2.clone(), piece.to_string());
                    let _ = q2.queue(move |c| c.ai_delta(qs(&k), qs(&p)));
                }
            };
            // (A completion model continues the command line itself: no chat.)
            let completion = task == "inline" && crate::ai::is_completion_model(&model_ref);
            let ask = |p: &crate::ai::Provider, m: &str| {
                if completion {
                    let (prompt, typed) = crate::ai::completion_prompt(msgs.last().and_then(|m| m["content"].as_str()).unwrap_or(""), &ctx);
                    let o = crate::ai::Options { max_tokens: 48, temperature: 0.1, timeout: std::time::Duration::from_secs(8) };
                    crate::ai::complete(p, m, &prompt, &o, &cancel).map(|more| format!("{typed}{more}"))
                } else {
                    crate::ai::chat(p, m, &msgs, &opts, &cancel, send)
                }
            };
            let mut result = ask(&provider, &model);
            // (Hearth's hub doesn't have that model: ask its server directly.)
            let missing = matches!(&result, Err(e) if e.contains("HTTP 404"));
            if let (true, Some((p, m))) = (missing && !cancel.load(Ordering::Relaxed), &fallback) {
                result = ask(p, m.as_str());
            }
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
                Ok(r) if block.is_some() => (crate::ai::block_reply(&r, &block.as_ref().unwrap().1, ctx["shell"] == "tessh"), String::new()),
                Ok(r) if task == "inline" && ctx["shell"] == "tessh" => {
                    let typed = msgs.last().and_then(|m| m["content"].as_str()).and_then(|c| c.rsplit_once("Complete this command line: ")).map(|(_, t)| t).unwrap_or("");
                    (crate::ai::tessh_suggestion(&crate::ai::command_from(&r), typed), String::new())
                }
                Ok(r) if matches!(task.as_str(), "inline" | "command" | "fix") => (crate::ai::command_from(&r), String::new()),
                Ok(r) => (r, String::new()),
                Err(e) => (String::new(), e),
            };
            let _ = qt.queue(move |c| c.ai_done(qs(&key), qs(&reply), qs(&err)));
        });
    }

    fn lsp_ask(self: Pin<&mut Self>, key: &ffi::QString, lang: &ffi::QString, cwd: &ffi::QString, text: &ffi::QString, what: &ffi::QString, cursor: i32, host: &ffi::QString) {
        let (key, lang, root, text, what, host) = (key.to_string(), lang.to_string(), local_path(cwd), text.to_string(), what.to_string(), host.to_string());
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            // (The language servers run where the pane's files are: its host's
            // tesserad.)
            let params = serde_json::json!({"key": key, "lang": lang, "root": root, "text": text, "what": what, "cursor": cursor.max(0)});
            let r = crate::client::rpc_within(&host, "lsp.ask", params, std::time::Duration::from_secs(30)).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
            let _ = qt.queue(move |c| c.lsp_ready(qs(&key), qs(&what), qs(&r)));
        });
    }

    fn lsp_available(&self, reply: &ffi::QString, lang: &ffi::QString, host: &ffi::QString) {
        let (lang, host) = (lang.to_string(), host.to_string());
        self.answer_on(reply, None, move || {
            let r = crate::client::rpc_within(&host, "lsp.available", serde_json::json!({"lang": lang}), std::time::Duration::from_secs(2));
            r.ok().as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string()
        })
    }

    fn test_key(&self, key: i32, mods: i32, text: &ffi::QString, scan: i32) {
        ffi::tessera_test_key(key, mods, text, scan);
    }

    fn test_mouse(&self, kind: i32, x: f64, y: f64, button: i32, mods: i32) {
        ffi::tessera_test_mouse(kind, x, y, button, mods);
    }

    fn test_wheel(&self, x: f64, y: f64, dy: i32, px: i32, phase: i32) {
        ffi::tessera_test_wheel(x, y, dy, px, phase);
    }

    fn ai_cancel(&self, key: &ffi::QString) {
        if let Some(c) = ai_jobs().lock().unwrap().remove(&key.to_string()) {
            c.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }

    fn chats_list(&self, reply: &ffi::QString) {
        self.answer_on(reply, None, move || serde_json::to_string(&crate::chats::list()).unwrap_or_default())
    }

    fn chats_search(&self, reply: &ffi::QString, query: &ffi::QString) {
        let query = query.to_string();
        self.answer_on(reply, None, move || serde_json::to_string(&crate::chats::search(&query)).unwrap_or_default())
    }

    fn chat_load(&self, reply: &ffi::QString, id: &ffi::QString) {
        let id = id.to_string();
        self.answer_on(reply, None, move || crate::chats::load(&id).and_then(|v| v.get("messages").cloned()).unwrap_or_else(|| serde_json::json!([])).to_string())
    }

    fn chat_save(&self, id: &ffi::QString, messages_json: &ffi::QString) {
        let msgs: Vec<serde_json::Value> = serde_json::from_str(&messages_json.to_string()).unwrap_or_default();
        if let Err(e) = crate::chats::save(&id.to_string(), &msgs) {
            eprintln!("tessera: saving the assistant conversation: {e}");
        }
    }

    fn chat_delete(&self, id: &ffi::QString) {
        crate::chats::delete(&id.to_string());
    }

    fn ai_ready(&self, task: &ffi::QString) -> bool {
        let s = crate::settings::load();
        let (t, r) = crate::ai::task_model(&s.ai, &task.to_string());
        crate::ai::route(&s.ai, &t, &r).is_ok_and(|(p, _, _)| p.local || crate::ai::has_key(&p))
    }

    fn ai_hearth_info(self: Pin<&mut Self>) {
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let json = crate::ai::hearth_info().to_string();
            let _ = qt.queue(move |c| c.ai_hearth_ready(qs(&json)));
        });
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

    fn prompt_presets(&self) -> ffi::QString {
        qs(&serde_json::to_string(&crate::prompt::presets()).unwrap_or_default())
    }

    fn prompt_preset(&self, name: &ffi::QString) -> ffi::QString {
        qs(&crate::prompt::preset(&name.to_string()))
    }

    fn prompt_tessera_config(&self) -> ffi::QString {
        qs(&crate::prompt::tessera_config())
    }

    fn prompt_user_config(&self) -> ffi::QString {
        qs(&crate::prompt::user_config())
    }

    fn save_prompt_config(mut self: Pin<&mut Self>, toml: &ffi::QString) -> ffi::QString {
        let r = crate::prompt::save_tessera_config(&toml.to_string());
        self.as_mut().bump();
        qs(&r.err().unwrap_or_default())
    }

    fn prompt_modules(&self, toml: &ffi::QString) -> ffi::QString {
        qs(&crate::prompt::module_states(&toml.to_string()))
    }

    fn prompt_set_module(&self, toml: &ffi::QString, module: &ffi::QString, on: bool) -> ffi::QString {
        qs(&crate::prompt::set_module(&toml.to_string(), &module.to_string(), on))
    }

    fn request_prompt_preview(self: Pin<&mut Self>, key: &ffi::QString, toml: &ffi::QString, cwd: &ffi::QString, width: i32) {
        let (key, toml, cwd) = (key.to_string(), toml.to_string(), local_path(cwd));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let json = crate::prompt::preview(&toml, &cwd, width.max(30) as u32, &key);
            let _ = qt.queue(move |c| c.starship_ready(qs(&key), qs(&json)));
        });
    }

    fn has_starship(&self) -> bool {
        crate::prompt::has_starship()
    }

    fn has_starship_on(&self, reply: &ffi::QString, host: &ffi::QString) {
        let host = host.to_string();
        self.answer_on(reply, None, move || {
            if host.is_empty() {
                return crate::prompt::has_starship().to_string();
            }
            // (Asked once a host.)
            static KNOWN: std::sync::Mutex<Option<std::collections::HashMap<String, bool>>> = std::sync::Mutex::new(None);
            if let Some(v) = KNOWN.lock().unwrap().as_ref().and_then(|m| m.get(&host)) {
                return v.to_string();
            }
            match crate::client::rpc_within(&host, "prompt.has_starship", serde_json::json!({}), std::time::Duration::from_secs(2)) {
                Ok(v) => {
                    let has = v.as_bool().unwrap_or(false);
                    KNOWN.lock().unwrap().get_or_insert_with(Default::default).insert(host, has);
                    has.to_string()
                }
                Err(_) => "false".into(),
            }
        })
    }

    fn request_starship(self: Pin<&mut Self>, key: &ffi::QString, cwd: &ffi::QString, status: i32, duration_ms: f64, width: i32, pid: i32, host: &ffi::QString) {
        let (key, cwd, host) = (key.to_string(), local_path(cwd), host.to_string());
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let params = serde_json::json!({"cwd": cwd, "status": status, "duration": duration_ms.max(0.0) as u64, "width": width.max(1), "pid": pid.max(0), "config": crate::prompt::window_config_text()});
            let json = crate::client::rpc(&host, "prompt.starship", params).map(|v| v.to_string()).unwrap_or_else(|_| "{}".into());
            let _ = qt.queue(move |c| c.starship_ready(qs(&key), qs(&json)));
        });
    }

    fn prompt_star_modules(&self) -> ffi::QString {
        qs(&serde_json::to_string(&crate::prompt_widgets::module_list()).unwrap_or_default())
    }

    fn prompt_format_modules(&self) -> ffi::QString {
        qs(&serde_json::to_string(&crate::prompt_widgets::format_modules()).unwrap_or_default())
    }

    fn prompt_choices(&self, reply: &ffi::QString, widget: &ffi::QString, cwd: &ffi::QString, pid: i32, host: &ffi::QString) {
        let (widget, cwd, host) = (widget.to_string(), local_path(cwd), host.to_string());
        self.answer_on(reply, None, move || {
            let r = crate::client::rpc_within(&host, "prompt.choices", serde_json::json!({"widget": widget, "cwd": cwd, "pid": pid.max(0)}), std::time::Duration::from_secs(5));
            r.map(|v| v.to_string()).unwrap_or_else(|_| "[]".into())
        })
    }

    fn request_prompt_modules(self: Pin<&mut Self>, key: &ffi::QString, names: &ffi::QString, cwd: &ffi::QString, status: i32, duration_ms: f64, pid: i32, host: &ffi::QString) {
        let (key, cwd, host) = (key.to_string(), local_path(cwd), host.to_string());
        let names: serde_json::Value = serde_json::from_str(&names.to_string()).unwrap_or_default();
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let params = serde_json::json!({"names": names, "cwd": cwd, "status": status, "duration": duration_ms.max(0.0) as u64, "pid": pid.max(0), "config": crate::prompt::window_config_text()});
            let json = crate::client::rpc(&host, "prompt.modules", params).map(|v| v.to_string()).unwrap_or_else(|_| "{}".into());
            let _ = qt.queue(move |c| c.prompt_modules_ready(qs(&key), qs(&json)));
        });
    }

    fn command_help_count(&self) -> i32 {
        let r = crate::client::rpc("", "complete.counts", serde_json::json!({})).unwrap_or_default();
        (r["fish"].as_u64().unwrap_or(0) + r["tldr"].as_u64().unwrap_or(0)) as i32
    }

    fn syntax_languages(&self) -> ffi::QString {
        qs(&crate::syntax::languages_json())
    }

    fn download_guides(mut self: Pin<&mut Self>) {
        self.as_mut().set_help_status(qs("Downloading the language guides\u{2026}"));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let q2 = qt.clone();
            let (got, failed) = crate::guides::download_all(move |done, total, bad| {
                let msg = format!("Downloading the language guides\u{2026} {done} of {total}{}", if bad > 0 { format!(" ({bad} couldn't be fetched)") } else { String::new() });
                let _ = q2.queue(move |mut c| c.as_mut().set_help_status(qs(&msg)));
            });
            let _ = qt.queue(move |mut c| {
                let msg = if failed.is_empty() {
                    format!("All language guides are kept for reading offline ({got} fetched now).")
                } else {
                    format!("Language guides kept for reading offline; {} couldn't be fetched: {}.", failed.len(), failed.join(", "))
                };
                c.as_mut().set_help_status(qs(&msg));
            });
        });
    }

    fn download_grammars(mut self: Pin<&mut Self>) {
        self.as_mut().set_help_status(qs("Downloading Helix's grammars\u{2026}"));
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let r = crate::grammars::ensure_all();
            let _ = qt.queue(move |mut c| {
                let msg = match r {
                    Ok(n) => format!("All {n} languages are here (for working offline)."),
                    Err(e) => format!("Couldn't download the grammars: {e}"),
                };
                c.as_mut().set_help_status(qs(&msg));
            });
        });
    }

    fn ensure_language(self: Pin<&mut Self>, name: &ffi::QString) {
        let name = name.to_string();
        // (One fetch at a time per language.)
        static BUSY: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
        {
            let mut b = BUSY.lock().unwrap();
            if b.contains(&name) {
                return;
            }
            b.push(name.clone());
        }
        let qt = self.qt_thread();
        std::thread::spawn(move || {
            let r = crate::grammars::ensure(&name);
            BUSY.lock().unwrap().retain(|n| n != &name);
            let err = r.err().unwrap_or_default();
            let _ = qt.queue(move |c| c.language_ready(qs(&name), qs(&err)));
        });
    }

    fn language_installed(&self, name: &ffi::QString) -> bool {
        crate::syntax::is_builtin(&name.to_string()) || crate::grammars::available().contains(&name.to_string())
    }

    fn grammar_count(&self) -> i32 {
        crate::grammars::count() as i32
    }

    fn pretty_command(&self, line: &ffi::QString, argv_json: &ffi::QString, dark: bool, text_color: &ffi::QString) -> ffi::QString {
        qs(&crate::cmdline::pretty_html(&line.to_string(), &argv_json.to_string(), dark, &text_color.to_string()))
    }

    fn shell_syntax(&self, reply: &ffi::QString, shell: &ffi::QString, text: &ffi::QString) {
        let (shell, text) = (shell.to_string(), text.to_string());
        self.answer_on(reply, Some("shell-check"), move || crate::syntax::shell_check(&shell, &text))
    }

    fn detect_language(&self, text: &ffi::QString) -> ffi::QString {
        qs(crate::syntax::detect_language(&text.to_string()))
    }

    fn detect_content(&self, reply: &ffi::QString, text: &ffi::QString) {
        let text = text.to_string();
        self.answer_on(reply, Some("detect"), move || crate::detect::language_of(text.as_bytes()))
    }

    fn doc_html(&self, text: &ffi::QString, lang: &ffi::QString) -> ffi::QString {
        qs(&crate::manual::doc_html(&text.to_string(), &lang.to_string()))
    }

    fn detect_file(&self, reply: &ffi::QString, path: &ffi::QString, head: &ffi::QString) {
        let (host, p) = host_path(&path.to_string());
        let head = head.to_string();
        self.answer_on(reply, Some("detect"), move || crate::detect::language_of_file(&p, host.is_empty(), &head))
    }

    fn shell_doubt(&self, text: &ffi::QString, aliases: &ffi::QString) -> bool {
        crate::snippets::shell_doubt(&text.to_string(), &aliases.to_string())
    }

    fn rank_languages(&self, reply: &ffi::QString, recent: &ffi::QString) -> ffi::QString {
        qs(&crate::snippets::rank(&reply.to_string(), &recent.to_string()))
    }

    fn runnable_languages(&self) -> ffi::QString {
        qs(&crate::snippets::runnable_languages_json())
    }

    fn snippet_command(&self, reply: &ffi::QString, lang: &ffi::QString, code: &ffi::QString, cwd: &ffi::QString, shell: &ffi::QString, host: &ffi::QString) {
        // (Worked out where it runs: what's installed there, its Nix.)
        let (lang, code, cwd, shell, host) = (lang.to_string(), code.to_string(), local_path(cwd), shell.to_string(), host.to_string());
        self.answer_on(reply, None, move || {
            let r = crate::client::rpc(&host, "snippet.command", serde_json::json!({"lang": lang, "code": code, "cwd": cwd, "shell": shell}));
            r.ok().as_ref().and_then(|v| v.as_str()).unwrap_or("").to_string()
        })
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
        let id = id.to_string();
        crate::background::in_order(move || {
            let host = crate::client::host_of(&id);
            crate::client::kill_session(&id);
            let _ = crate::client::rpc(&host, "cells.forget", serde_json::json!({"session": id}));
        });
    }

    fn notify(&self, title: &ffi::QString, body: &ffi::QString) {
        let icon = concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tessera.svg");
        let _ = std::process::Command::new("notify-send")
            .args(["-a", "Tessera", "-i", icon, &title.to_string(), &body.to_string()])
            .spawn();
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

    fn restorable_sessions(&self, reply: &ffi::QString, host: &ffi::QString) {
        let host = host.to_string();
        self.answer_on(reply, None, move || {
            let ids: Vec<String> = crate::client::list_sessions(&host).unwrap_or_default().into_iter().filter(|s| !s.attached).map(|s| s.id).collect();
            serde_json::to_string(&ids).unwrap()
        })
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

    fn trim_memory(&self) {
        #[cfg(target_env = "gnu")]
        std::thread::spawn(|| unsafe {
            libc::malloc_trim(0);
        });
    }

    fn open_doc(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_OPEN_DOC").unwrap_or_default())
    }

    fn startup_command(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_RUN").unwrap_or_default())
    }

    fn exec_request(&self) -> ffi::QString {
        ffi::QString::from(&std::env::var("TESSERA_EXEC").unwrap_or_default())
    }

    fn start_gui_link(&self) {
        let qt = self.qt_thread();
        crate::gui_link::listen(move |json| {
            let _ = qt.queue(move |c| c.exec_requested(qs(&json)));
        });
    }

    fn gui_link_touch(&self) {
        crate::gui_link::touch();
    }

    fn set_activation_token(&self, token: ffi::QString) {
        let t = token.to_string();
        if t.is_empty() {
            std::env::remove_var("XDG_ACTIVATION_TOKEN");
        } else {
            std::env::set_var("XDG_ACTIVATION_TOKEN", t);
        }
    }
}

/// A folder from QML: an OSC 7 URL (file://host/path) or a plain path; home
/// when empty.
/// Kernels' events reach QML (jupyter_event) from tesserad's event
/// connections, set up once.
fn ensure_kernel_events(c: &Pin<&mut ffi::Controller>) {
    static SET: std::sync::Once = std::sync::Once::new();
    let qt = c.qt_thread();
    SET.call_once(move || {
        crate::client::set_event_sink(std::sync::Arc::new(move |k: String, v: serde_json::Value| {
            let j = v.to_string();
            let _ = qt.queue(move |c| c.jupyter_event(qs(&k), qs(&j)));
        }));
    });
}

/// As JavaScript's encodeURIComponent (an SSH host in an ssh:// path).
fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// A path the window has: "ssh://<host, URL-encoded>/<path>" for a file on
/// another host (a remote pane's), else a plain path or file:// URL here.
/// (host, path), "" host for this computer.
fn host_path(s: &str) -> (String, String) {
    if let Some(rest) = s.strip_prefix("ssh://") {
        let (h, p) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
        return (percent_decode(h), if p.is_empty() { "/".to_string() } else { p.to_string() });
    }
    (String::new(), local_path(&qs(s)))
}

/// A file request: here (`local`), or by the file's host's tesserad (rpc.rs
/// "files"), which runs the same function there. A few seconds at most over
/// SSH (it's the window's thread).
/// A file action (open, trash, copy...), here or on the file's host: "" or
/// what went wrong. (In the background: copies over scp take a while.)
fn file_action_now(op: String, path: &str, arg: String) -> String {
    // (Another host's file, for this desktop: opened or made a GIF of as
    // a copy fetched here; revealed in the file manager over sftp:// when
    // the host is a plain name, else the copy. The rest happen there.)
    let (host, p) = host_path(path);
    if !host.is_empty() && matches!(op.as_str(), "open" | "gif" | "reveal") {
        if op == "reveal" && !host.contains(char::is_whitespace) {
            let uri = format!("sftp://{host}{p}");
            return crate::files::action("reveal_uri", &uri, "");
        }
        return match crate::remote::fetch(&host, &p) {
            Ok(local) => crate::files::action(&op, &local.to_string_lossy(), &arg),
            Err(e) => e,
        };
    }
    // (Copied or moved between this computer and a host, or two hosts:
    // through scp; a move then removes the original.)
    if matches!(op.as_str(), "copy" | "move") {
        let (dhost, dpath) = host_path(&arg);
        if dhost != host {
            let r: Result<(), String> = (|| {
                if host.is_empty() {
                    crate::remote::upload(&dhost, std::path::Path::new(&p), &dpath)?;
                } else if dhost.is_empty() {
                    crate::remote::download(&host, &p, std::path::Path::new(&dpath))?;
                } else {
                    let tmp = std::env::temp_dir().join(format!("tessera-copy-{}", std::process::id()));
                    let _ = std::fs::create_dir_all(&tmp);
                    let here = crate::remote::download(&host, &p, &tmp)?;
                    let r = crate::remote::upload(&dhost, &here, &dpath);
                    let _ = std::fs::remove_dir_all(&tmp);
                    r?;
                }
                if op == "move" {
                    let e = if host.is_empty() {
                        crate::files::action("trash", &p, "")
                    } else {
                        let r = crate::client::rpc_within(&host, "files", serde_json::json!({"op": "action", "action": "trash", "path": p, "arg": ""}), std::time::Duration::from_secs(10))?;
                        r.as_str().unwrap_or("").to_string()
                    };
                    if !e.is_empty() {
                        return Err(e);
                    }
                }
                Ok(())
            })();
            return r.err().unwrap_or_default();
        }
    }
    // (A folder on the same host, as a path there.)
    let arg = if !host.is_empty() && arg.starts_with("ssh://") { host_path(&arg).1 } else { arg };
    on_host(path, "action", serde_json::json!({"action": op, "arg": arg}), |p| crate::files::action(&op, p, &arg))
}

// (Runs in the background: never on the window's thread.)
fn on_host(path: &str, op: &str, mut params: serde_json::Value, local: impl FnOnce(&str) -> String) -> String {
    let (host, p) = host_path(path);
    if host.is_empty() {
        return local(&p);
    }
    params["op"] = serde_json::Value::String(op.to_string());
    params["path"] = serde_json::Value::String(p);
    match crate::client::rpc_within(&host, "files", params, std::time::Duration::from_secs(10)) {
        // (A listing's paths are the host's: they stay on it.)
        Ok(serde_json::Value::String(s)) if op == "list" => {
            let prefix = format!("ssh://{}", percent_encode(&host));
            let mut v: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
            for e in v.as_array_mut().into_iter().flatten() {
                if let Some(p) = e["path"].as_str().map(String::from) {
                    e["path"] = serde_json::Value::String(format!("{prefix}{p}"));
                }
            }
            v.to_string()
        }
        Ok(serde_json::Value::String(s)) => s,
        Ok(v) => v.to_string(),
        Err(e) => serde_json::json!({"error": e}).to_string(),
    }
}

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
