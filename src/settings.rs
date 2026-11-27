//! Preferences and color themes, kept in ~/.config/tessera/.
//!
//! settings.json holds the preferences; themes/*.json holds imported
//! themes (from .itermcolors files), next to the built-in ones below.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Global preferences (iTerm2's General, Appearance, Pointer panels) plus
/// the profiles.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    /// Format version (2: profiles default to one theme, as in iTerm2).
    pub version: u32,
    // -- General > Startup / Closing
    /// Reopen the shells (and their layout) that were open when the window
    /// closed. Off: closing the window ends them.
    pub restore_sessions: bool,
    /// Local mode: no tesserad. The window keeps its shells, kernels and
    /// cells in its own process (on a socket only it serves); they end with
    /// it. Off (the default): tesserad keeps them running between windows.
    /// Takes effect the next time Tessera opens (or `tessera --local`).
    pub local_mode: bool,
    pub confirm_close_multiple: bool,
    pub confirm_quit: bool,
    // -- General > Selection
    pub copy_on_select: bool,
    pub copy_trailing_newline: bool,
    /// Programs may set the clipboard (OSC 52).
    pub clipboard_access: bool,
    pub triple_click_wrapped: bool,
    /// Characters besides letters and digits that count as part of a word.
    pub word_chars: String,
    // -- General > Window
    /// A program's title escape changes only its own tab's title.
    pub tab_titles_per_tab: bool,
    // -- Appearance > Tabs
    pub always_show_tabs: bool,
    /// "top" or "bottom".
    pub tab_position: String,
    pub show_tab_numbers: bool,
    pub tab_close_buttons: bool,
    pub show_activity: bool,
    pub stretch_tabs: bool,
    // -- Appearance > Panes / Dimming
    pub pane_titles: bool,
    pub dim_inactive: bool,
    pub dim_amount: f64,
    pub dim_background_windows: bool,
    // -- Pointer
    pub middle_click_pastes: bool,
    /// "menu" or "paste".
    pub right_click: String,
    pub focus_follows_mouse: bool,
    // -- Edit > Paste Special
    pub warn_multiline_paste: bool,
    // -- Shell > Undo Close: seconds a closed session stays revivable (0: off).
    pub undo_close_seconds: u32,
    // -- The input editor under the terminal at a prompt, and its keys
    // ("emacs", "vi", "vscode").
    pub input_editor: bool,
    pub editor_keys: String,
    /// The input editor's indentation per language, as picked on its indent
    /// chip: {lang: {"mode": "spaces"|"tabs"|"mixed", "size": n}} (a
    /// language not here has its usual one).
    pub editor_indent_langs: serde_json::Value,
    /// Indentation detected from the text (tabs, spaces and their size)
    /// over the language's setting, once the text has indented lines.
    pub editor_detect_indent: bool,
    /// Lines down the indentation, one per level, as VS Code draws them.
    pub editor_indent_guides: bool,
    /// How wide a tab is drawn in the input editor, in columns (with tabs a
    /// level is one; mixed puts one for every this many columns).
    pub editor_tab_width: u32,
    /// Line numbers in the input editor once it has more than one line.
    pub editor_line_numbers: bool,
    /// A file card's terminal editor ("" = the shell's $VISUAL or $EDITOR).
    pub terminal_editor: String,
    /// The guide page's contents list, how wide (dragged at its edge).
    pub guide_contents_width: u32,
    // -- Notebook: Markdown output as a page ("rich") or as styled
    // monospace text, the way a terminal Markdown viewer shows it ("text").
    pub markdown_style: String,
    // -- Notebook: what a notebook runs cells in until one is picked in it: a
    // kernel's name ("tessh", "python3", "tessera-sh-zsh"), a language
    // ("python", "fish"), or "tessera-shell" (the pane's own shell).
    pub notebook_kernel: String,
    // -- Tessera's prompt above the editor, and its segments in order ("|"
    // puts the rest on the right).
    pub editor_prompt: bool,
    // -- "auto" (Starship when installed), "starship", or "builtin".
    pub prompt_engine: String,
    // -- Starship's config: "own" (the user's starship.toml) or "tessera"
    // (the one built in Preferences › Prompt).
    pub prompt_config: String,
    // -- AI (src/ai.rs): providers and which model does what.
    pub ai: AiSettings,
    pub prompt_segments: Vec<String>,
    // -- The prompt bar as arranged on it (dragged, added with +): widget
    // names, "star:<module>" for a Starship module, "starship" /
    // "starship_right" for Starship's whole prompt, "|" for the right side.
    // Empty: from prompt_engine and prompt_segments.
    pub prompt_layout: Vec<String>,
    // -- Each widget's look: {name: {fg, bg, pill, prefix, suffix, bold}}.
    pub prompt_styles: serde_json::Value,
    // -- The notebook's file browser (ls cards), as last set: {view, sort,
    // desc, folders_first, hidden}.
    pub file_browser: serde_json::Value,
    // -- SSH hosts without tesserad, as answered: {host: "tesserad"
    // (install it there) | "ssh" (plain ssh from here)}.
    pub ssh_hosts: serde_json::Value,
    // -- Pointer: Ctrl+click opens URLs and files.
    pub ctrl_click_opens: bool,
    // -- Snippets (Edit > Snippets): text sent as if typed.
    pub snippets: Vec<Snippet>,
    // -- Profiles
    pub default_profile: String,
    pub profiles: Vec<Profile>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: 0,
            restore_sessions: true,
            local_mode: false,
            confirm_close_multiple: true,
            confirm_quit: false,
            copy_on_select: true,
            copy_trailing_newline: false,
            clipboard_access: true,
            triple_click_wrapped: true,
            word_chars: "/-+\\~_.".into(),
            tab_titles_per_tab: true,
            always_show_tabs: true,
            tab_position: "top".into(),
            show_tab_numbers: false,
            tab_close_buttons: false,
            show_activity: true,
            stretch_tabs: false,
            pane_titles: false,
            dim_inactive: true,
            dim_amount: 0.22,
            dim_background_windows: false,
            middle_click_pastes: true,
            right_click: "menu".into(),
            focus_follows_mouse: false,
            warn_multiline_paste: false,
            undo_close_seconds: 5,
            input_editor: true,
            editor_keys: "emacs".into(),
            editor_indent_langs: serde_json::json!({}),
            editor_detect_indent: true,
            editor_indent_guides: true,
            editor_tab_width: 4,
            editor_line_numbers: true,
            terminal_editor: String::new(),
            guide_contents_width: 230,
            markdown_style: "rich".into(),
            notebook_kernel: "tessh".into(),
            editor_prompt: true,
            prompt_engine: "auto".into(),
            prompt_config: "own".into(),
            ai: AiSettings::default(),
            prompt_segments: ["dir", "git", "venv", "nix", "status", "duration", "|", "kube", "aws", "gcloud", "host", "time"].map(String::from).to_vec(),
            prompt_layout: Vec::new(),
            prompt_styles: serde_json::json!({}),
            file_browser: serde_json::json!({}),
            ssh_hosts: serde_json::json!({}),
            ctrl_click_opens: true,
            snippets: vec![Snippet { title: "Git status".into(), text: "git status\\n".into() }],
            default_profile: "Default".into(),
            profiles: vec![Profile::default()],
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Snippet {
    pub title: String,
    /// \\n, \\t and \\e are a newline, tab and escape.
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Trigger {
    pub regex: String,
    /// "highlight", "notify", "bell" or "send".
    pub action: String,
    /// A color for highlight, a title for notify, text for send.
    pub param: String,
    pub enabled: bool,
}

impl Default for Trigger {
    fn default() -> Self {
        Self { regex: String::new(), action: "highlight".into(), param: "#d9b43b".into(), enabled: true }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct KeyMapping {
    /// A key sequence such as "Ctrl+Alt+K".
    pub key: String,
    /// "send_text", "send_hex", "send_escape", "ignore" or "action".
    pub action: String,
    pub value: String,
}

/// One profile (iTerm2's Profiles panel, tab by tab).
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Profile {
    // -- General
    pub name: String,
    /// "login" (the login shell) or "custom" (`command`, run through the shell).
    pub command_mode: String,
    pub command: String,
    /// "home", "recycle" (the current tab's directory) or "custom".
    pub initial_dir_mode: String,
    pub directory: String,
    /// Typed into new sessions once they start (a newline is added).
    pub send_text: String,
    /// Empty: the program's title, else the folder name.
    pub tab_title: String,
    pub badge: String,
    // -- Colors
    pub theme_mode: String,
    pub theme: String,
    pub theme_light: String,
    pub theme_dark: String,
    pub faint_opacity: f64,
    /// 0 (off) to 1: text too close to its background is moved toward
    /// black or white until it reaches the contrast this asks for.
    pub minimum_contrast: f64,
    pub cursor_guide: bool,
    // -- Text
    pub font_family: String,
    pub font_size: f64,
    pub line_height: f64,
    pub char_spacing: f64,
    pub cursor_style: String,
    pub cursor_blink: bool,
    pub bold_font: bool,
    pub italic: bool,
    pub blink_text: bool,
    // -- Window
    /// 0 (opaque) to 0.9.
    pub transparency: f64,
    pub keep_backgrounds_opaque: bool,
    pub columns: u32,
    pub rows: u32,
    pub padding: f64,
    // -- Terminal
    pub scrollback: u32,
    pub unlimited_scrollback: bool,
    pub term: String,
    pub silence_bell: bool,
    pub visual_bell: bool,
    pub bell_icon: bool,
    pub notify_bell: bool,
    pub notify_exit: bool,
    /// Shell integration mark indicators at each prompt.
    pub show_marks: bool,
    pub mouse_reporting: bool,
    pub alternate_scroll: bool,
    pub allow_title: bool,
    pub bracketed_paste: bool,
    // -- Session
    /// "close", "restart" or "keep".
    pub on_exit: String,
    /// "never", "always" or "jobs" (when something besides the shell runs).
    pub confirm_close: String,
    /// Shell jobs that don't count as running jobs for "jobs".
    pub ignored_jobs: String,
    /// Log every session's output to a file in `log_folder`.
    pub auto_log: bool,
    pub log_folder: String,
    // -- Keys
    /// Alt sends Esc+key ("esc") or the key itself ("normal").
    pub alt_mode: String,
    pub key_mappings: Vec<KeyMapping>,
    // -- Advanced
    pub triggers: Vec<Trigger>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            command_mode: "login".into(),
            command: String::new(),
            initial_dir_mode: "recycle".into(),
            directory: String::new(),
            send_text: String::new(),
            tab_title: String::new(),
            badge: String::new(),
            theme_mode: "fixed".into(),
            theme: "Tessera Dark".into(),
            theme_light: "Tessera Light".into(),
            theme_dark: "Tessera Dark".into(),
            faint_opacity: 0.6,
            minimum_contrast: 0.1,
            cursor_guide: false,
            font_family: String::new(),
            font_size: 0.0,
            line_height: 1.1,
            char_spacing: 1.0,
            cursor_style: "block".into(),
            cursor_blink: false,
            bold_font: true,
            italic: true,
            blink_text: true,
            transparency: 0.0,
            keep_backgrounds_opaque: true,
            columns: 100,
            rows: 30,
            padding: 6.0,
            scrollback: 10_000,
            unlimited_scrollback: false,
            term: "xterm-256color".into(),
            silence_bell: false,
            visual_bell: true,
            bell_icon: true,
            notify_bell: false,
            notify_exit: false,
            show_marks: true,
            mouse_reporting: true,
            alternate_scroll: true,
            allow_title: true,
            bracketed_paste: true,
            on_exit: "close".into(),
            confirm_close: "jobs".into(),
            ignored_jobs: "rlogin, ssh, slogin, telnet, sh, bash, zsh, fish, nu, tmux, screen".into(),
            auto_log: false,
            log_folder: "~/tessera-logs".into(),
            alt_mode: "esc".into(),
            key_mappings: Vec::new(),
            triggers: Vec::new(),
        }
    }
}

impl Settings {
    pub fn profile(&self, name: &str) -> Profile {
        self.profiles
            .iter()
            .find(|p| p.name == name)
            .or_else(|| self.profiles.iter().find(|p| p.name == self.default_profile))
            .or(self.profiles.first())
            .cloned()
            .unwrap_or_default()
    }

    fn unique_name(&self, base: &str) -> String {
        if !self.profiles.iter().any(|p| p.name == base) {
            return base.to_string();
        }
        (2..).map(|n| format!("{base} {n}")).find(|n| !self.profiles.iter().any(|p| &p.name == n)).unwrap()
    }

    /// Add a profile copied from `from` (or a fresh one). Returns its name.
    pub fn add_profile(&mut self, from: Option<&str>) -> String {
        let mut p = match from {
            Some(f) => self.profile(f),
            None => Profile::default(),
        };
        p.name = self.unique_name(if from.is_some() { &p.name } else { "New Profile" });
        let name = p.name.clone();
        self.profiles.push(p);
        name
    }

    pub fn remove_profile(&mut self, name: &str) {
        if self.profiles.len() > 1 {
            self.profiles.retain(|p| p.name != name);
            if self.default_profile == name {
                self.default_profile = self.profiles[0].name.clone();
            }
        }
    }

    /// Set one profile field (JSON value). Renaming keeps the default pointing at it.
    pub fn set_profile_field(&mut self, name: &str, key: &str, value: &str) -> bool {
        let Some(i) = self.profiles.iter().position(|p| p.name == name) else { return false };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(value) else { return false };
        if key == "name" {
            let Some(new) = v.as_str().map(str::trim).filter(|n| !n.is_empty()) else { return false };
            if new != name && self.profiles.iter().any(|p| p.name == new) {
                return false;
            }
            if self.default_profile == name {
                self.default_profile = new.to_string();
            }
        }
        let mut obj = serde_json::to_value(&self.profiles[i]).unwrap();
        let map = obj.as_object_mut().unwrap();
        if !map.contains_key(key) {
            return false;
        }
        map.insert(key.into(), v);
        match serde_json::from_value::<Profile>(obj) {
            Ok(p) => {
                self.profiles[i] = p;
                true
            }
            Err(_) => false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    pub background: String,
    pub foreground: String,
    pub cursor: String,
    #[serde(default)]
    pub cursor_text: String,
    pub selection_background: String,
    #[serde(default)]
    pub selection_foreground: String,
    /// The 16 ANSI colors: black, red, green, yellow, blue, magenta, cyan,
    /// white, then the bright versions.
    pub ansi: Vec<String>,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub dark: bool,
    /// Where it comes from ("Tessera", "iTerm2-Color-Schemes", "Imported").
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub license: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AiSettings {
    pub providers: Vec<crate::ai::Provider>,
    /// "provider/model" for each use; empty: off.
    pub inline_model: String,
    pub chat_model: String,
    pub check_model: String,
    /// Grey suggestions in the editor as you type (Tab or → takes them).
    pub inline: bool,
    /// The AI looks over the command before you run it.
    pub check: bool,
    /// Lines starting with "# " become a command (Enter asks, Enter again runs).
    pub natural_language: bool,
    /// The first-run look for a local model server happened.
    pub autodetected: bool,
    /// Use Hearth's AI hub when it runs here (the desktop's models and
    /// memory manager); the models above when it doesn't.
    pub hearth: bool,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            providers: Vec::new(),
            inline_model: String::new(),
            chat_model: String::new(),
            check_model: String::new(),
            inline: true,
            check: true,
            natural_language: true,
            autodetected: false,
            hearth: true,
        }
    }
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(std::env::temp_dir)
        .join("tessera")
}

fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

fn themes_dir() -> PathBuf {
    config_dir().join("themes")
}

pub fn load() -> Settings {
    let Some(bytes) = std::fs::read(settings_path()).ok() else { return Settings::default() };
    let Ok(raw) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return Settings::default() };
    let mut s: Settings = serde_json::from_value(raw.clone()).unwrap_or_default();
    // Settings from before profiles: the per-terminal fields lived at the top.
    if raw.get("profiles").is_none() {
        if let Some(obj) = raw.as_object() {
            let mut p = serde_json::to_value(Profile::default()).unwrap();
            for (k, v) in obj {
                if let Some(slot) = p.get_mut(k) {
                    *slot = v.clone();
                }
            }
            if let Ok(p) = serde_json::from_value::<Profile>(p) {
                s.profiles = vec![p];
            }
        }
    }
    if s.profiles.is_empty() {
        s.profiles.push(Profile::default());
    }
    if s.version < 2 {
        // Version 1 followed the desktop's light/dark mode by default, so a
        // light theme picked on a dark desktop never showed. Use the theme
        // the person picked as the one theme (like iTerm2's default).
        for p in &mut s.profiles {
            if p.theme_mode == "system" {
                p.theme = if p.theme_light != "Tessera Light" {
                    p.theme_light.clone()
                } else if p.theme_dark != "Tessera Dark" {
                    p.theme_dark.clone()
                } else {
                    "Tessera Dark".into()
                };
                p.theme_mode = "fixed".into();
            }
        }
        s.version = 2;
        let _ = save(&s);
    }
    s
}

// ---- window arrangements -------------------------------------------------

fn arrangements_path() -> PathBuf {
    config_dir().join("arrangements.json")
}

/// Saved arrangements: name -> layout JSON (written by the QML side).
pub fn arrangements() -> serde_json::Map<String, serde_json::Value> {
    std::fs::read(arrangements_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn save_arrangements(m: &serde_json::Map<String, serde_json::Value>) -> std::io::Result<()> {
    std::fs::create_dir_all(config_dir())?;
    std::fs::write(arrangements_path(), serde_json::to_vec_pretty(m).unwrap())
}

pub fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir)
        .join("tessera")
}

/// The layout to bring back at the next start (state, not a preference).
fn last_layout_path() -> PathBuf {
    state_dir().join("last-layout.json")
}

pub fn last_layout() -> String {
    std::fs::read_to_string(last_layout_path()).unwrap_or_default()
}

pub fn save_last_layout(json: &str) {
    let p = last_layout_path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(p, json);
}

pub fn save(s: &Settings) -> std::io::Result<()> {
    std::fs::create_dir_all(config_dir())?;
    let tmp = settings_path().with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(s).unwrap())?;
    std::fs::rename(tmp, settings_path())
}

/// Set one field from QML: `key` is a field name, `value` its JSON value.
pub fn set(s: &mut Settings, key: &str, value: &str) -> bool {
    let mut obj = serde_json::to_value(&*s).unwrap();
    let Ok(v) = serde_json::from_str::<serde_json::Value>(value) else { return false };
    let Some(map) = obj.as_object_mut() else { return false };
    if !map.contains_key(key) {
        return false;
    }
    map.insert(key.to_string(), v);
    match serde_json::from_value::<Settings>(obj) {
        Ok(n) => {
            *s = n;
            true
        }
        Err(_) => false,
    }
}

fn parse_hex(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

pub fn hex_rgb(s: &str) -> Option<libghostty_vt::style::RgbColor> {
    parse_hex(s).map(|(r, g, b)| libghostty_vt::style::RgbColor { r, g, b })
}

fn is_dark(bg: &str) -> bool {
    let (r, g, b) = parse_hex(bg).unwrap_or((0, 0, 0));
    (0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64) < 128.0
}

fn t(name: &str, bg: &str, fg: &str, cursor: &str, sel: &str, ansi: [&str; 16]) -> Theme {
    Theme {
        name: name.into(),
        background: bg.into(),
        foreground: fg.into(),
        cursor: cursor.into(),
        cursor_text: bg.into(),
        selection_background: sel.into(),
        selection_foreground: String::new(),
        ansi: ansi.iter().map(|s| s.to_string()).collect(),
        builtin: true,
        dark: is_dark(bg),
        source: "Tessera".into(),
        license: String::new(),
    }
}

/// The permissively licensed iTerm2-Color-Schemes (themes/README.md).
fn bundled_pack() -> &'static [Theme] {
    static PACK: std::sync::OnceLock<Vec<Theme>> = std::sync::OnceLock::new();
    PACK.get_or_init(|| {
        let mut v: Vec<Theme> = serde_json::from_str(include_str!("../themes/iterm2-color-schemes.json")).unwrap_or_default();
        for t in &mut v {
            t.builtin = true;
            t.dark = is_dark(&t.background);
            t.source = "iTerm2-Color-Schemes".into();
        }
        v
    })
}

pub fn builtin_themes() -> Vec<Theme> {
    vec![
        t("Tessera Dark", "#1b1d20", "#d7dade", "#e8eaed", "#36404d", [
            "#2a2d32", "#ef6b73", "#8fcf7a", "#e8c46b", "#62a8f0", "#c792ea", "#5cc8c8", "#c9ccd1",
            "#5c636e", "#ff8a92", "#a9e392", "#f5d68a", "#86bdf7", "#d8acf2", "#7fdede", "#f2f4f7",
        ]),
        t("Tessera OLED", "#000000", "#d3d6db", "#eef0f3", "#253248", [
            "#000000", "#ff5f6d", "#7ee081", "#ffd166", "#5eb0ff", "#d38cff", "#4fd6d6", "#c9ccd1",
            "#555b64", "#ff7b86", "#9ff0a1", "#ffe08a", "#85c3ff", "#e3adff", "#7ce8e8", "#ffffff",
        ]),
        t("Tessera Light", "#fbfbfa", "#2b2f36", "#2b2f36", "#cfe0f7", [
            "#2b2f36", "#c8323c", "#3d8a2c", "#a67c00", "#2862c9", "#9a3fb5", "#1e8a8f", "#c4c7cc",
            "#6b717b", "#e0434d", "#4ea33a", "#c49200", "#3a78e0", "#b356cf", "#2aa3a8", "#e6e8eb",
        ]),
        t("Classic", "#000000", "#c7c7c7", "#c7c7c7", "#3a4659", [
            "#000000", "#c91b00", "#00c200", "#c7c400", "#0225c7", "#ca30c7", "#00c5c7", "#c7c7c7",
            "#686868", "#ff6e67", "#5ffa68", "#fffc67", "#6871ff", "#ff77ff", "#60fdff", "#ffffff",
        ]),
        t("Solarized Dark", "#002b36", "#839496", "#93a1a1", "#073642", [
            "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5",
            "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
        ]),
        t("Solarized Light", "#fdf6e3", "#657b83", "#586e75", "#eee8d5", [
            "#073642", "#dc322f", "#859900", "#b58900", "#268bd2", "#d33682", "#2aa198", "#eee8d5",
            "#002b36", "#cb4b16", "#586e75", "#657b83", "#839496", "#6c71c4", "#93a1a1", "#fdf6e3",
        ]),
        t("Dracula", "#282a36", "#f8f8f2", "#f8f8f2", "#44475a", [
            "#21222c", "#ff5555", "#50fa7b", "#f1fa8c", "#bd93f9", "#ff79c6", "#8be9fd", "#f8f8f2",
            "#6272a4", "#ff6e6e", "#69ff94", "#ffffa5", "#d6acff", "#ff92df", "#a4ffff", "#ffffff",
        ]),
        t("Nord", "#2e3440", "#d8dee9", "#d8dee9", "#434c5e", [
            "#3b4252", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#88c0d0", "#e5e9f0",
            "#4c566a", "#bf616a", "#a3be8c", "#ebcb8b", "#81a1c1", "#b48ead", "#8fbcbb", "#eceff4",
        ]),
        t("Gruvbox Dark", "#282828", "#ebdbb2", "#ebdbb2", "#504945", [
            "#282828", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a", "#a89984",
            "#928374", "#fb4934", "#b8bb26", "#fabd2f", "#83a598", "#d3869b", "#8ec07c", "#ebdbb2",
        ]),
        t("Gruvbox Light", "#fbf1c7", "#3c3836", "#3c3836", "#ebdbb2", [
            "#fbf1c7", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a", "#7c6f64",
            "#928374", "#9d0006", "#79740e", "#b57614", "#076678", "#8f3f71", "#427b58", "#3c3836",
        ]),
        t("One Dark", "#282c34", "#abb2bf", "#528bff", "#3e4451", [
            "#282c34", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#abb2bf",
            "#5c6370", "#e06c75", "#98c379", "#e5c07b", "#61afef", "#c678dd", "#56b6c2", "#ffffff",
        ]),
        t("Catppuccin Mocha", "#1e1e2e", "#cdd6f4", "#f5e0dc", "#45475a", [
            "#45475a", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#bac2de",
            "#585b70", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#f5c2e7", "#94e2d5", "#a6adc8",
        ]),
        t("Catppuccin Latte", "#eff1f5", "#4c4f69", "#dc8a78", "#ccd0da", [
            "#5c5f77", "#d20f39", "#40a02b", "#df8e1d", "#1e66f5", "#ea76cb", "#179299", "#acb0be",
            "#6c6f85", "#d20f39", "#40a02b", "#df8e1d", "#1e66f5", "#ea76cb", "#179299", "#bcc0cc",
        ]),
        t("Tokyo Night", "#1a1b26", "#c0caf5", "#c0caf5", "#33467c", [
            "#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6",
            "#414868", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#c0caf5",
        ]),
        t("Tango Light", "#ffffff", "#2e3436", "#2e3436", "#c6dcf5", [
            "#2e3436", "#cc0000", "#4e9a06", "#c4a000", "#3465a4", "#75507b", "#06989a", "#d3d7cf",
            "#555753", "#ef2929", "#8ae234", "#fce94f", "#729fcf", "#ad7fa8", "#34e2e2", "#eeeeec",
        ]),
    ]
}

pub fn user_themes() -> Vec<Theme> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(themes_dir()) {
        for e in rd.flatten() {
            let Ok(b) = std::fs::read(e.path()) else { continue };
            if let Ok(mut th) = serde_json::from_slice::<Theme>(&b) {
                // One imported theme.
                th.builtin = false;
                th.dark = is_dark(&th.background);
                if th.source.is_empty() {
                    th.source = "Imported".into();
                }
                out.push(th);
            } else if let Ok(pack) = serde_json::from_slice::<Vec<Theme>>(&b) {
                // A downloaded pack: kept as a whole (not deletable one by one).
                for mut th in pack {
                    th.builtin = true;
                    th.dark = is_dark(&th.background);
                    out.push(th);
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn all_themes() -> Vec<Theme> {
    let mut v = builtin_themes();
    let mut seen: std::collections::HashSet<String> = v.iter().map(|t| t.name.to_lowercase()).collect();
    for t in bundled_pack().iter().cloned().chain(user_themes()) {
        if seen.insert(t.name.to_lowercase()) {
            v.push(t);
        }
    }
    v
}

/// Download the complete iTerm2-Color-Schemes collection (git) and save it
/// as one pack file in the themes folder. Returns how many it holds.
pub fn download_pack() -> Result<usize, String> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("tessera")
        .join("iTerm2-Color-Schemes");
    let ok = if cache.join(".git").exists() {
        std::process::Command::new("git").arg("-C").arg(&cache).args(["pull", "-q", "--depth", "1"]).status()
    } else {
        let _ = std::fs::create_dir_all(cache.parent().unwrap());
        std::process::Command::new("git")
            .args(["clone", "-q", "--depth", "1", "https://github.com/mbadolato/iTerm2-Color-Schemes.git"])
            .arg(&cache)
            .status()
    };
    match ok {
        Ok(s) if s.success() => {}
        Ok(_) => return Err("git couldn't download the collection".into()),
        Err(e) => return Err(format!("couldn't run git: {e}")),
    }
    let mut pack = Vec::new();
    let rd = std::fs::read_dir(cache.join("ghostty")).map_err(|e| e.to_string())?;
    let mut files: Vec<PathBuf> = rd.flatten().map(|e| e.path()).collect();
    files.sort();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        if let Some(t) = parse_ghostty_theme(&f.file_name().unwrap_or_default().to_string_lossy(), &text) {
            pack.push(t);
        }
    }
    std::fs::create_dir_all(themes_dir()).map_err(|e| e.to_string())?;
    std::fs::write(themes_dir().join("iterm2-color-schemes-all.json"), serde_json::to_vec(&pack).unwrap()).map_err(|e| e.to_string())?;
    Ok(pack.len())
}

/// A theme in Ghostty's format (key = value, palette = N=#rrggbb).
pub fn parse_ghostty_theme(name: &str, text: &str) -> Option<Theme> {
    let mut kv = std::collections::HashMap::new();
    let mut pal: [Option<String>; 16] = Default::default();
    for line in text.lines() {
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if k == "palette" {
            if let Some((i, c)) = v.split_once('=') {
                if let Ok(i) = i.trim().parse::<usize>() {
                    if i < 16 {
                        pal[i] = Some(c.trim().to_string());
                    }
                }
            }
        } else {
            kv.insert(k.to_string(), v.to_string());
        }
    }
    let hex = |c: &str| if c.starts_with('#') { c.to_string() } else { format!("#{c}") };
    let ansi: Vec<String> = pal.iter().map(|c| c.as_deref().map(hex)).collect::<Option<_>>()?;
    let bg = hex(kv.get("background")?);
    let fg = hex(kv.get("foreground")?);
    Some(Theme {
        name: name.to_string(),
        cursor: kv.get("cursor-color").map(|c| hex(c)).unwrap_or_else(|| fg.clone()),
        cursor_text: kv.get("cursor-text").map(|c| hex(c)).unwrap_or_else(|| bg.clone()),
        selection_background: kv.get("selection-background").map(|c| hex(c)).unwrap_or_else(|| ansi[8].clone()),
        selection_foreground: kv.get("selection-foreground").map(|c| hex(c)).unwrap_or_default(),
        dark: is_dark(&bg),
        background: bg,
        foreground: fg,
        ansi,
        builtin: true,
        source: "iTerm2-Color-Schemes".into(),
        license: String::new(),
    })
}

pub fn find_theme(name: &str) -> Theme {
    let all = all_themes();
    all.iter().find(|t| t.name == name).cloned().unwrap_or_else(|| all[0].clone())
}

/// Import an iTerm2 .itermcolors file (an XML property list) as a theme.
pub fn import_itermcolors(path: &str) -> Result<Theme, String> {
    let xml = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let colors = parse_plist_colors(&xml).ok_or("not an .itermcolors file")?;
    let get = |k: &str| colors.iter().find(|(n, _)| n == k).map(|(_, c)| c.clone());
    let mut ansi = Vec::new();
    for i in 0..16 {
        ansi.push(get(&format!("Ansi {i} Color")).ok_or(format!("missing Ansi {i} Color"))?);
    }
    let bg = get("Background Color").ok_or("missing Background Color")?;
    let fg = get("Foreground Color").ok_or("missing Foreground Color")?;
    let name = std::path::Path::new(path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported".into());
    let th = Theme {
        name,
        cursor: get("Cursor Color").unwrap_or_else(|| fg.clone()),
        cursor_text: get("Cursor Text Color").unwrap_or_else(|| bg.clone()),
        selection_background: get("Selection Color").unwrap_or_else(|| ansi[8].clone()),
        selection_foreground: get("Selected Text Color").unwrap_or_default(),
        dark: is_dark(&bg),
        background: bg,
        foreground: fg,
        ansi,
        builtin: false,
        source: "Imported".into(),
        license: String::new(),
    };
    std::fs::create_dir_all(themes_dir()).map_err(|e| e.to_string())?;
    let file: String = th.name.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();
    std::fs::write(themes_dir().join(format!("{file}.json")), serde_json::to_vec_pretty(&th).unwrap())
        .map_err(|e| e.to_string())?;
    Ok(th)
}

/// Pull `<key>Name</key><dict>…Red/Green/Blue Component…</dict>` pairs out
/// of a plist. Enough for .itermcolors without an XML dependency.
fn parse_plist_colors(xml: &str) -> Option<Vec<(String, String)>> {
    let mut out = Vec::new();
    let mut rest = xml;
    let top = rest.find("<dict>")?;
    rest = &rest[top + 6..];
    loop {
        let Some(k) = rest.find("<key>") else { break };
        let after = &rest[k + 5..];
        let Some(ke) = after.find("</key>") else { break };
        let name = after[..ke].trim().to_string();
        let after = &after[ke + 6..];
        let ds = after.find("<dict>");
        let next_key = after.find("<key>");
        match (ds, next_key) {
            (Some(ds), Some(nk)) if ds < nk => {
                let body_start = ds + 6;
                let Some(de) = after[body_start..].find("</dict>") else { break };
                let body = &after[body_start..body_start + de];
                let comp = |c: &str| -> Option<f64> {
                    let i = body.find(&format!("<key>{c} Component</key>"))?;
                    let b = &body[i..];
                    let rs = b.find("<real>")? + 6;
                    let re = b[rs..].find("</real>")?;
                    b[rs..rs + re].trim().parse().ok()
                };
                if let (Some(r), Some(g), Some(b)) = (comp("Red"), comp("Green"), comp("Blue")) {
                    let to = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                    out.push((name, format!("#{:02x}{:02x}{:02x}", to(r), to(g), to(b))));
                }
                rest = &after[body_start + de + 7..];
            }
            _ => rest = after,
        }
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_pack_loads() {
        let all = all_themes();
        assert!(all.len() > 250, "{}", all.len());
        assert!(all.iter().any(|t| t.name == "Tessera OLED"));
        // No duplicate names.
        let mut names: Vec<String> = all.iter().map(|t| t.name.to_lowercase()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), all.len());
    }

    #[test]
    fn ghostty_format() {
        let mut s = String::new();
        for i in 0..16 {
            s.push_str(&format!("palette = {i}=#0{:x}0000\n", i));
        }
        s.push_str("background = #000000\nforeground = fafafa\n");
        let t = parse_ghostty_theme("X", &s).unwrap();
        assert_eq!(t.foreground, "#fafafa");
        assert_eq!(t.ansi.len(), 16);
        assert!(t.dark);
    }

    #[test]
    fn plist() {
        let xml = r#"<?xml version="1.0"?><plist><dict>
            <key>Ansi 0 Color</key><dict><key>Blue Component</key><real>0.5</real>
            <key>Green Component</key><real>0</real><key>Red Component</key><real>1</real></dict>
            <key>Background Color</key><dict><key>Alpha Component</key><real>1</real>
            <key>Blue Component</key><real>0</real><key>Green Component</key><real>0</real>
            <key>Red Component</key><real>0</real></dict></dict></plist>"#;
        let c = parse_plist_colors(xml).unwrap();
        assert_eq!(c[0], ("Ansi 0 Color".into(), "#ff0080".into()));
        assert_eq!(c[1], ("Background Color".into(), "#000000".into()));
    }

    #[test]
    fn set_field() {
        let mut s = Settings::default();
        assert!(set(&mut s, "dim_amount", "0.3"));
        assert_eq!(s.dim_amount, 0.3);
        assert!(!set(&mut s, "nope", "1"));
        assert!(!set(&mut s, "dim_amount", "\"big\""));
    }

    #[test]
    fn old_follow_system_becomes_the_picked_theme() {
        let mut p = Profile::default();
        p.theme_mode = "system".into();
        p.theme_light = "Solarized Light".into();
        let mut s = Settings { profiles: vec![p], ..Settings::default() };
        s.version = 1;
        let dir = std::env::temp_dir().join(format!("tessera-test-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        save(&s).unwrap();
        let l = load();
        assert_eq!(l.version, 2);
        assert_eq!(l.profiles[0].theme_mode, "fixed");
        assert_eq!(l.profiles[0].theme, "Solarized Light");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn profiles() {
        let mut s = Settings::default();
        let n = s.add_profile(Some("Default"));
        assert_eq!(n, "Default 2");
        assert!(s.set_profile_field(&n, "font_size", "14"));
        assert_eq!(s.profile(&n).font_size, 14.0);
        assert!(s.set_profile_field("Default", "name", "\"Main\""));
        assert_eq!(s.default_profile, "Main");
        assert!(!s.set_profile_field("Main", "name", "\"Default 2\""));
        s.remove_profile("Main");
        assert_eq!(s.default_profile, "Default 2");
    }
}
