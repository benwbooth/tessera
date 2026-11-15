//! Preferences and color themes, kept in ~/.config/tessera/.
//!
//! settings.json holds the preferences; themes/*.json holds imported
//! themes (from .itermcolors files), next to the built-in ones below.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    /// Empty = the system's monospace font.
    pub font_family: String,
    /// 0 = the system monospace size.
    pub font_size: f64,
    /// Row height as a multiple of the font's own line height.
    pub line_height: f64,
    /// "fixed" uses `theme`; "system" follows the desktop's light/dark mode.
    pub theme_mode: String,
    pub theme: String,
    pub theme_light: String,
    pub theme_dark: String,
    /// "block", "bar" or "underline".
    pub cursor_style: String,
    pub scrollback: u32,
    pub copy_on_select: bool,
    pub dim_inactive: bool,
    /// Space around the text, in pixels.
    pub padding: f64,
    /// Show the tab strip with a single tab.
    pub always_show_tabs: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            font_family: String::new(),
            font_size: 0.0,
            line_height: 1.1,
            theme_mode: "system".into(),
            theme: "Tessera Dark".into(),
            theme_light: "Tessera Light".into(),
            theme_dark: "Tessera Dark".into(),
            cursor_style: "block".into(),
            scrollback: 10_000,
            copy_on_select: true,
            dim_inactive: true,
            padding: 6.0,
            always_show_tabs: true,
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
    std::fs::read(settings_path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
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
    }
}

pub fn builtin_themes() -> Vec<Theme> {
    vec![
        t("Tessera Dark", "#1b1d20", "#d7dade", "#e8eaed", "#36404d", [
            "#2a2d32", "#ef6b73", "#8fcf7a", "#e8c46b", "#62a8f0", "#c792ea", "#5cc8c8", "#c9ccd1",
            "#5c636e", "#ff8a92", "#a9e392", "#f5d68a", "#86bdf7", "#d8acf2", "#7fdede", "#f2f4f7",
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
            if let Ok(b) = std::fs::read(e.path()) {
                if let Ok(mut th) = serde_json::from_slice::<Theme>(&b) {
                    th.builtin = false;
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
    v.extend(user_themes());
    v
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
        assert!(set(&mut s, "font_size", "13.5"));
        assert_eq!(s.font_size, 13.5);
        assert!(!set(&mut s, "nope", "1"));
        assert!(!set(&mut s, "font_size", "\"big\""));
    }
}
