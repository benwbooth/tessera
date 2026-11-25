//! The prompt bar above the editor: Tessera's (qml/PromptBar.qml,
//! src/prompt.rs, src/prompt_widgets.rs) as lines of spans the editor draws.
//! A row of widgets left and right of a divider: Tessera's own (the folder,
//! git, the last command's status and time, the user and host, the clock,
//! cloud contexts), any Starship module ("star:<module>"), or Starship's
//! whole prompt ("starship", "starship_right"), arranged and styled as the
//! user's Tessera has it (~/.config/tessera/settings.json: prompt_layout,
//! prompt_styles, prompt_engine, prompt_config, prompt_segments).
//!
//! A click acts on a widget (`choices`: folders to go to, branches to switch
//! to, contexts and profiles, each a shell command run as if typed); a click
//! on a parent folder of the path goes there (Hit::Path).
//!
//! Not here yet (PromptBar has them): dragging widgets to rearrange them,
//! the style popup and widget menu (right click), the + that adds a widget,
//! and splitting Starship's prompt into its modules.

use super::{Col, Hit, Line, Span, Sty};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

type Rgb = (u8, u8, u8);

/// What the bar is drawn for.
#[derive(Clone, Debug, Default)]
pub struct BarCtx {
    pub cwd: String,
    /// The last command's exit status and how long it took.
    pub status: i32,
    pub duration_ms: u64,
    /// The terminal's width in columns.
    pub width: u16,
    /// This session's folders, the latest first.
    pub visited: Vec<String>,
    /// The terminal's (foreground, background), from OSC 10/11, for the
    /// tints Tessera mixes from them (unknown: plain ANSI colors instead).
    pub colors: Option<(Rgb, Rgb)>,
}

/// The bar: its left side's lines (wrapped to the width, widgets side by
/// side, a taller one's neighbours centered on it as Tessera does) and its
/// right side (one line; Tessera centers it on the bar's height). Each
/// widget's spans carry Hit::Widget(name); a folder of a path shown,
/// Hit::Path(that folder) (the shell's folder itself: Hit::Widget("dir"),
/// its choices).
#[derive(Clone, Debug, Default)]
pub struct Bar {
    pub left: Vec<Line>,
    pub right: Line,
}

/// One of a widget's choices: `cmd` is run as if typed; a section is a
/// heading (cmd ""); `current` is the one in effect (a ✓).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub label: String,
    pub cmd: String,
    pub section: bool,
    pub current: bool,
}

// ---- settings (Tessera's) ---------------------------------------------------

/// The prompt's settings, as the user's Tessera has them.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Whether Tessera shows the prompt above the editor at all.
    pub editor_prompt: bool,
    /// "auto" (Starship when installed), "starship", "builtin".
    pub engine: String,
    /// Starship's config: "own" (the user's starship.toml) or "tessera".
    pub config: String,
    pub segments: Vec<String>,
    /// As arranged on the bar (empty: from engine and segments).
    pub layout: Vec<String>,
    /// {name: {fg, bg, prefix, suffix, bold}}.
    pub styles: serde_json::Value,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            editor_prompt: true,
            engine: "auto".into(),
            config: "own".into(),
            segments: ["dir", "git", "venv", "nix", "status", "duration", "|", "kube", "aws", "gcloud", "host", "time"].map(String::from).to_vec(),
            layout: Vec::new(),
            styles: serde_json::json!({}),
        }
    }
}

fn home() -> String {
    std::env::var("HOME").unwrap_or_default()
}

/// Tessera's config folder (~/.config/tessera).
fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(std::env::temp_dir)
        .join("tessera")
}

fn state_dir() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/state")))
        .unwrap_or_else(std::env::temp_dir)
        .join("tessera")
}

/// Tessera's settings.json, the prompt's part (read each time: Preferences
/// may have changed it).
pub fn settings() -> Settings {
    let raw = std::fs::read(config_dir().join("settings.json")).ok().and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok());
    settings_from(raw.as_ref())
}

fn settings_from(raw: Option<&serde_json::Value>) -> Settings {
    let mut s = Settings::default();
    let Some(v) = raw else { return s };
    let strs = |k: &str| v.get(k).and_then(|x| x.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect::<Vec<_>>());
    if let Some(b) = v.get("editor_prompt").and_then(|x| x.as_bool()) {
        s.editor_prompt = b;
    }
    if let Some(e) = v.get("prompt_engine").and_then(|x| x.as_str()) {
        s.engine = e.to_string();
    }
    if let Some(c) = v.get("prompt_config").and_then(|x| x.as_str()) {
        s.config = c.to_string();
    }
    if let Some(l) = strs("prompt_segments") {
        s.segments = l;
    }
    if let Some(l) = strs("prompt_layout") {
        s.layout = l;
    }
    if let Some(st) = v.get("prompt_styles").filter(|x| x.is_object()) {
        s.styles = st.clone();
    }
    s
}

/// Whether the bar is shown (Tessera's editor_prompt).
pub fn enabled() -> bool {
    settings().editor_prompt
}

fn use_starship(s: &Settings) -> bool {
    s.engine == "starship" || (s.engine == "auto" && starship_bin().is_some())
}

/// The bar's widgets in order, "|" before the right side's.
pub fn layout(s: &Settings) -> Vec<String> {
    if !s.layout.is_empty() {
        return s.layout.clone();
    }
    if use_starship(s) {
        return ["starship", "|", "starship_right"].map(String::from).to_vec();
    }
    s.segments.clone()
}

/// The Starship config to use: Tessera's own when that's the choice, else
/// the user's (None).
fn starship_config(s: &Settings) -> Option<PathBuf> {
    (s.config == "tessera").then(|| config_dir().join("starship.toml"))
}

// ---- runs: text with one look ------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
struct R {
    t: String,
    sty: Sty,
    hit: Hit,
    /// Starship's $fill: stretched to the room its line has.
    fill: bool,
}

impl R {
    fn new(t: impl Into<String>, sty: Sty) -> R {
        R { t: t.into(), sty, hit: Hit::None, fill: false }
    }
    fn with_text(&self, t: String) -> R {
        R { t, ..self.clone() }
    }
}

fn wd(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

fn wdc(c: &[char]) -> usize {
    c.iter().map(|ch| unicode_width::UnicodeWidthChar::width(*ch).unwrap_or(0)).sum()
}

fn line_w(l: &[R]) -> usize {
    l.iter().map(|r| wd(&r.t)).sum()
}

/// Powerline / Nerd Font caps: a pill's rounded or pointed ends.
const CAPS: &str = "\u{e0b0}\u{e0b2}\u{e0b4}\u{e0b6}\u{e0bc}\u{e0be}\u{e0b8}\u{e0ba}";

fn is_cap(t: &str) -> bool {
    let mut c = t.chars();
    matches!((c.next(), c.next()), (Some(x), None) if CAPS.contains(x))
}

/// "p<N>" (palette entry N), "#rrggbb", else none.
fn col(c: &str) -> Option<Col> {
    if let Some(n) = c.strip_prefix('p') {
        return n.parse::<u8>().ok().map(Col::Ansi);
    }
    let h = c.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some(Col::Rgb(p(0)?, p(2)?, p(4)?))
}

fn xterm256(n: i32) -> Col {
    match n {
        0..=15 => Col::Ansi(n as u8),
        16..=231 => {
            let n = n - 16;
            let v = |x: i32| if x == 0 { 0 } else { (55 + x * 40) as u8 };
            Col::Rgb(v(n / 36), v((n / 6) % 6), v(n % 6))
        }
        _ => {
            let g = (8 + (n.clamp(232, 255) - 232) * 10) as u8;
            Col::Rgb(g, g, g)
        }
    }
}

/// ANSI text (Starship's) -> runs, by its SGR colors.
fn runs(s: &str) -> Vec<R> {
    let mut sty = Sty::default();
    let mut out: Vec<R> = Vec::new();
    let mut text = String::new();
    let mut chars = s.chars().peekable();
    let flush = |text: &mut String, out: &mut Vec<R>, sty: Sty| {
        if !text.is_empty() {
            out.push(R::new(std::mem::take(text), sty));
        }
    };
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            // (zsh/bash prompt wrappers, if any.)
            if c != '\u{1}' && c != '\u{2}' && c != '\n' && c != '\r' {
                text.push(c);
            }
            continue;
        }
        if chars.peek() != Some(&'[') {
            continue;
        }
        chars.next();
        let mut params = String::new();
        let mut fin = ' ';
        for p in chars.by_ref() {
            if p.is_ascii_alphabetic() {
                fin = p;
                break;
            }
            params.push(p);
        }
        if fin != 'm' {
            continue;
        }
        flush(&mut text, &mut out, sty);
        let nums: Vec<i32> = params.split(';').map(|n| n.parse().unwrap_or(0)).collect();
        let mut i = 0;
        while i < nums.len() {
            match nums[i] {
                0 => sty = Sty::default(),
                1 => sty.bold = true,
                2 => sty.dim = true,
                3 => sty.italic = true,
                4 => sty.underline = true,
                22 => {
                    sty.bold = false;
                    sty.dim = false;
                }
                23 => sty.italic = false,
                24 => sty.underline = false,
                39 => sty.fg = None,
                49 => sty.bg = None,
                n @ 30..=37 => sty.fg = Some(Col::Ansi((n - 30) as u8)),
                n @ 90..=97 => sty.fg = Some(Col::Ansi((n - 90 + 8) as u8)),
                n @ 40..=47 => sty.bg = Some(Col::Ansi((n - 40) as u8)),
                n @ 100..=107 => sty.bg = Some(Col::Ansi((n - 100 + 8) as u8)),
                n @ (38 | 48) => {
                    let mut set = None;
                    match nums.get(i + 1) {
                        Some(2) if i + 4 < nums.len() => {
                            let b = |x: i32| x.clamp(0, 255) as u8;
                            set = Some(Col::Rgb(b(nums[i + 2]), b(nums[i + 3]), b(nums[i + 4])));
                            i += 4;
                        }
                        Some(5) if i + 2 < nums.len() => {
                            set = Some(xterm256(nums[i + 2]));
                            i += 2;
                        }
                        _ => {}
                    }
                    if set.is_some() {
                        if n == 38 {
                            sty.fg = set;
                        } else {
                            sty.bg = set;
                        }
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
    flush(&mut text, &mut out, sty);
    out
}

fn strip(s: &str) -> String {
    let mut out = String::new();
    let mut esc = false;
    for c in s.chars() {
        if esc {
            if c.is_ascii_alphabetic() {
                esc = false;
            }
            continue;
        }
        if c == '\u{1b}' {
            esc = true;
            continue;
        }
        out.push(c);
    }
    out
}

/// Split `runs` so the `len` characters from `start` are a run of their own,
/// marked fill.
fn mark_fill(runs: &mut Vec<R>, start: usize, len: usize) {
    let mut out = Vec::new();
    let mut at = 0;
    for r in runs.drain(..) {
        let t: Vec<char> = r.t.chars().collect();
        let (a, b) = (at, at + t.len());
        at = b;
        let (fs, fe) = (start.max(a), (start + len).min(b));
        if fs >= fe {
            out.push(r);
            continue;
        }
        let piece = |x: usize, y: usize, fill: bool| R { t: t[x - a..y - a].iter().collect(), fill: fill || r.fill, ..r.clone() };
        if fs > a {
            out.push(piece(a, fs, false));
        }
        out.push(piece(fs, fe, true));
        if fe < b {
            out.push(piece(fe, b, false));
        }
    }
    // (One fill run, though the fill spanned several: it's the one stretched.)
    let mut merged: Vec<R> = Vec::new();
    for r in out {
        if r.fill {
            if let Some(last) = merged.last_mut().filter(|l| l.fill) {
                last.t.push_str(&r.t);
                continue;
            }
        }
        merged.push(r);
    }
    *runs = merged;
}

// ---- Starship ----------------------------------------------------------------

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(name)).find(|f| f.is_file()))
}

fn starship_bin() -> Option<PathBuf> {
    on_path(if cfg!(windows) { "starship.exe" } else { "starship" })
}

fn starship_cmd(bin: &Path, cwd: &str, cfg: Option<&Path>) -> std::process::Command {
    let mut cmd = std::process::Command::new(bin);
    cmd.env("TERM", "xterm-256color").env_remove("STARSHIP_SHELL").env("PWD", cwd);
    if let Some(c) = cfg {
        cmd.env("STARSHIP_CONFIG", c);
    }
    if Path::new(cwd).is_dir() {
        cmd.current_dir(cwd);
    }
    cmd.stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    cmd
}

fn starship_run(ctx: &BarCtx, cwd: &str, right: bool, width: usize, cfg: Option<&Path>) -> String {
    let Some(bin) = starship_bin() else { return String::new() };
    let mut cmd = starship_cmd(&bin, cwd, cfg);
    cmd.arg("prompt");
    if right {
        cmd.arg("--right");
    }
    cmd.args(["--status", &ctx.status.to_string(), "--cmd-duration", &ctx.duration_ms.to_string(), "--terminal-width", &width.max(1).to_string(), "--jobs", "0"]);
    cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

/// Starship's prompt (left) as lines of runs, `width` wide: the final line
/// (the prompt character alone) left out, the editor stands in for it; its
/// $fill marked (found by asking again a little wider and seeing where a
/// line grew), to be stretched to the room the line has.
fn starship_lines(ctx: &BarCtx, cwd: &str, width: usize, cfg: Option<&Path>) -> Vec<Vec<R>> {
    // (The wider ask that finds $fill, at the same time as the first, not
    // after it: a line as wide as the bar, the usual case with a $fill,
    // needs it, and waiting doubled the bar's time.)
    let (left, wider) = std::thread::scope(|sc| {
        let wider = sc.spawn(|| starship_run(ctx, cwd, false, width + FILL_MORE, cfg));
        (starship_run(ctx, cwd, false, width, cfg), wider.join().unwrap_or_default())
    });
    starship_parse(&left, width, |w| if w == width + FILL_MORE { wider.clone() } else { starship_run(ctx, cwd, false, w, cfg) })
}

/// How much wider the second ask is, to find $fill.
const FILL_MORE: usize = 8;

fn starship_parse(left: &str, width: usize, run_at: impl Fn(usize) -> String) -> Vec<Vec<R>> {
    let mut lines: Vec<&str> = left.split('\n').collect();
    while lines.first().is_some_and(|l| strip(l).trim().is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|l| strip(l).trim().is_empty()) {
        lines.pop();
    }
    // The last line: just the prompt character ("❯ ")?
    if lines.len() > 1 && strip(lines.last().unwrap()).trim().chars().count() <= 2 {
        lines.pop();
    }
    let mut out: Vec<Vec<R>> = lines.iter().map(|l| runs(l)).collect();
    let plain = |l: &str| -> Vec<char> { strip(l).chars().filter(|c| *c != '\u{1}' && *c != '\u{2}' && *c != '\r').collect() };
    if lines.iter().any(|l| plain(l).len() + 1 >= width) {
        const MORE: usize = FILL_MORE;
        let wider = run_at(width + MORE);
        let mut wl: Vec<&str> = wider.split('\n').collect();
        while wl.first().is_some_and(|l| strip(l).trim().is_empty()) {
            wl.remove(0);
        }
        for (i, l) in lines.iter().enumerate() {
            let (a, b) = (plain(l), wl.get(i).map(|w| plain(w)).unwrap_or_default());
            if b.len() != a.len() + MORE {
                continue;
            }
            let p = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
            let c = b[p];
            let mut start = p;
            while start > 0 && a[start - 1] == c {
                start -= 1;
            }
            if p > start {
                mark_fill(&mut out[i], start, p - start);
            }
        }
    }
    out
}

fn starship_right(ctx: &BarCtx, cwd: &str, cfg: Option<&Path>) -> Vec<R> {
    let t = starship_run(ctx, cwd, true, ctx.width as usize, cfg);
    runs(t.trim_end_matches(['\n', '\r']))
}

/// One Starship module, drawn ([] when it shows nothing).
fn starship_module(name: &str, ctx: &BarCtx, cwd: &str, cfg: Option<&Path>) -> Vec<R> {
    let Some(bin) = starship_bin() else { return Vec::new() };
    let mut cmd = starship_cmd(&bin, cwd, cfg);
    cmd.args(["module", name, "--status", &ctx.status.to_string(), "--cmd-duration", &ctx.duration_ms.to_string(), "--jobs", "0"]);
    let text = cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let text = text.trim_end_matches(['\n', ' ']);
    if strip(text).trim().is_empty() {
        return Vec::new();
    }
    runs(text)
}

// ---- facts: git, user, host, cloud --------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Git {
    branch: String,
    detached: bool,
    ahead: i64,
    behind: i64,
    staged: u32,
    modified: u32,
    untracked: u32,
    conflicts: u32,
}

fn parse_status(text: &str) -> Git {
    let mut g = Git::default();
    let mut oid = String::new();
    for line in text.lines() {
        if let Some(b) = line.strip_prefix("# branch.head ") {
            g.detached = b == "(detached)";
            g.branch = b.to_string();
        } else if let Some(o) = line.strip_prefix("# branch.oid ") {
            oid = o.chars().take(7).collect();
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for p in ab.split_whitespace() {
                if let Some(n) = p.strip_prefix('+') {
                    g.ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = p.strip_prefix('-') {
                    g.behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("? ") {
            g.untracked += 1;
        } else if line.starts_with("u ") {
            g.conflicts += 1;
        } else if (line.starts_with("1 ") || line.starts_with("2 ")) && line.len() >= 4 {
            let xy: Vec<char> = line[2..4].chars().collect();
            if xy[0] != '.' {
                g.staged += 1;
            }
            if xy.get(1).is_some_and(|c| *c != '.') {
                g.modified += 1;
            }
        }
    }
    if g.detached {
        g.branch = oid;
    }
    g
}

/// Git's state for `cwd` (asked at most once a second per folder).
fn git(cwd: &str) -> Git {
    static CACHE: Mutex<Option<HashMap<String, (Instant, Git)>>> = Mutex::new(None);
    if let Some((t, g)) = CACHE.lock().unwrap().as_ref().and_then(|m| m.get(cwd)) {
        if t.elapsed() < Duration::from_millis(900) {
            return g.clone();
        }
    }
    let out = std::process::Command::new("git")
        .args(["-C", cwd, "--no-optional-locks", "status", "--porcelain=v2", "--branch"])
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();
    let g = match out {
        Ok(o) if o.status.success() => parse_status(&String::from_utf8_lossy(&o.stdout)),
        _ => Git::default(),
    };
    let mut c = CACHE.lock().unwrap();
    let m = c.get_or_insert_with(HashMap::new);
    if m.len() > 200 {
        m.clear();
    }
    m.insert(cwd.to_string(), (Instant::now(), g.clone()));
    g
}

#[derive(Clone, Debug, Default)]
struct Cloud {
    /// (account, project)
    gcloud: Option<(String, String)>,
    /// (profile, region)
    aws: Option<(String, String)>,
    kube: Option<String>,
    venv: Option<String>,
    nix: Option<String>,
}

fn cloud(env: &HashMap<String, String>) -> Cloud {
    let home = PathBuf::from(env.get("HOME").cloned().unwrap_or_default());
    let mut v = Cloud::default();
    // gcloud: ~/.config/gcloud/active_config names configurations/config_<name>.
    let gdir = env.get("CLOUDSDK_CONFIG").map(PathBuf::from).unwrap_or_else(|| home.join(".config/gcloud"));
    let active = env.get("CLOUDSDK_ACTIVE_CONFIG_NAME").cloned().or_else(|| std::fs::read_to_string(gdir.join("active_config")).ok()).map(|s| s.trim().to_string());
    if let Some(name) = active.filter(|n| !n.is_empty()) {
        if let Ok(ini) = std::fs::read_to_string(gdir.join("configurations").join(format!("config_{name}"))) {
            let get = |key: &str| ini.lines().find_map(|l| l.split_once('=').filter(|(k, _)| k.trim() == key).map(|(_, v)| v.trim().to_string()));
            v.gcloud = Some((get("account").unwrap_or_default(), get("project").unwrap_or_default()));
        }
    }
    if let Some(p) = env.get("AWS_PROFILE").or(env.get("AWS_VAULT")) {
        v.aws = Some((p.clone(), env.get("AWS_REGION").or(env.get("AWS_DEFAULT_REGION")).cloned().unwrap_or_default()));
    }
    let kube = env.get("KUBECONFIG").and_then(|k| k.split(':').next().map(PathBuf::from)).unwrap_or_else(|| home.join(".kube/config"));
    if let Ok(text) = std::fs::read_to_string(kube) {
        if let Some(ctx) = text.lines().find_map(|l| l.strip_prefix("current-context:")) {
            let ctx = ctx.trim().trim_matches('"');
            if !ctx.is_empty() {
                v.kube = Some(ctx.to_string());
            }
        }
    }
    if let Some(venv) = env.get("VIRTUAL_ENV") {
        v.venv = Some(venv.rsplit('/').next().unwrap_or(venv).to_string());
    }
    if env.contains_key("IN_NIX_SHELL") {
        v.nix = Some(env.get("name").cloned().unwrap_or_else(|| "nix-shell".into()));
    }
    v
}

#[derive(Clone, Debug, Default)]
struct Info {
    git: Git,
    user: String,
    host: String,
    ssh: bool,
    cloud: Cloud,
}

fn hostname() -> String {
    if let Some(h) = std::fs::read_to_string("/etc/hostname").ok().map(|h| h.trim().to_string()).filter(|h| !h.is_empty()) {
        return h;
    }
    if let Ok(h) = std::env::var("HOSTNAME") {
        return h;
    }
    let mut buf = [0u8; 256];
    // SAFETY: the buffer is 256 bytes and its length is passed.
    if unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) } == 0 {
        let n = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
        return String::from_utf8_lossy(&buf[..n]).into_owned();
    }
    String::new()
}

fn info(cwd: &str, need_git: bool, need_cloud: bool) -> Info {
    let env: HashMap<String, String> = std::env::vars().collect();
    Info {
        git: if need_git { git(cwd) } else { Git::default() },
        user: env.get("USER").or(env.get("USERNAME")).cloned().unwrap_or_default(),
        host: hostname(),
        ssh: env.contains_key("SSH_CONNECTION"),
        cloud: if need_cloud { cloud(&env) } else { Cloud::default() },
    }
}

// ---- colors from the terminal's --------------------------------------------------

fn mix(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let m = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round().clamp(0.0, 255.0) as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

fn luma(c: Rgb) -> f64 {
    (0.299 * c.0 as f64 + 0.587 * c.1 as f64 + 0.114 * c.2 as f64) / 255.0
}

fn rgb(c: Rgb) -> Col {
    Col::Rgb(c.0, c.1, c.2)
}

/// Theme.bg: the terminal's background (else black, ANSI 0).
fn theme_bg(ctx: &BarCtx) -> Col {
    ctx.colors.map(|(_, bg)| rgb(bg)).unwrap_or(Col::Ansi(0))
}

/// Theme.mix(Theme.bg, Theme.fg, 0.12): a pill's tint (else ANSI 0).
fn tint(ctx: &BarCtx) -> Col {
    ctx.colors.map(|(fg, bg)| rgb(mix(bg, fg, 0.12))).unwrap_or(Col::Ansi(0))
}

/// Theme.textDim (else bright black, ANSI 8).
fn text_dim(ctx: &BarCtx) -> Col {
    let Some((fg, bg)) = ctx.colors else { return Col::Ansi(8) };
    let l = luma(bg);
    let chrome = if l < 0.002 {
        bg
    } else if l < 0.06 {
        mix(bg, fg, 0.07)
    } else if l < 0.5 {
        mix(bg, (0, 0, 0), 0.2)
    } else {
        mix(bg, (0, 0, 0), 0.05)
    };
    rgb(mix(fg, chrome, 0.42))
}

// ---- Tessera's own widgets ----------------------------------------------------------

/// One of Tessera's widgets: text, colors (no bg: plain text), whether its
/// look is a pill.
#[derive(Clone, Debug)]
struct Seg {
    text: String,
    fg: Col,
    bg: Option<Col>,
    branch_icon: bool,
    bold: Option<bool>,
}

/// The shell's folder as a path (file:// URLs too).
fn plain_cwd(cwd: &str) -> String {
    match cwd.strip_prefix("file://") {
        Some(r) => r.find('/').map(|i| r[i..].to_string()).unwrap_or_default(),
        None => cwd.to_string(),
    }
}

/// ~ for home. (Always the whole path: a shortened one reads like a
/// relative path.)
fn tilde(p: &str, home: &str) -> String {
    if !home.is_empty() && (p == home || p.starts_with(&format!("{home}/"))) {
        format!("~{}", &p[home.len()..])
    } else {
        p.to_string()
    }
}

fn seconds(ms: u64) -> String {
    if ms < 60000 {
        let s = ms as f64 / 1000.0;
        return if ms < 10000 { format!("{s:.1}s") } else { format!("{s:.0}s") };
    }
    let m = ms / 60000;
    let s = ((ms % 60000) as f64 / 1000.0).round() as u64;
    if m < 60 {
        format!("{m}m {s}s")
    } else {
        format!("{}h {}m", m / 60, m % 60)
    }
}

fn hhmm() -> String {
    // SAFETY: localtime_r writes the zeroed tm it's given.
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        libc::localtime_r(&t, &mut tm);
        format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
    }
}

fn segment(name: &str, ctx: &BarCtx, i: &Info, home: &str) -> Option<Seg> {
    let seg = |text: String, fg: Col, bg: Option<Col>| Seg { text, fg, bg, branch_icon: false, bold: None };
    let c = &i.cloud;
    match name {
        "dir" => Some(seg(format!("\u{f07c}  {}", tilde(&plain_cwd(&ctx.cwd), home)), theme_bg(ctx), Some(Col::Ansi(5)))),
        "git" => {
            let g = &i.git;
            if g.branch.is_empty() {
                return None;
            }
            let mut t = format!("{}{}", if g.detached { "➦ " } else { "" }, g.branch);
            let mut marks = Vec::new();
            for (n, m) in [(g.conflicts as i64, "="), (g.staged as i64, "+"), (g.modified as i64, "!"), (g.untracked as i64, "?"), (g.ahead, "⇡"), (g.behind, "⇣")] {
                if n != 0 {
                    marks.push(format!("{m}{n}"));
                }
            }
            if !marks.is_empty() {
                t += &format!("  {}", marks.join(" "));
            }
            let dirty = g.staged + g.modified + g.untracked + g.conflicts > 0;
            Some(Seg { branch_icon: !g.detached, ..seg(t, Col::Ansi(if dirty { 3 } else { 2 }), Some(tint(ctx))) })
        }
        "status" => (ctx.status != 0).then(|| seg(format!("✘ {}", ctx.status), Col::Ansi(1), None)),
        "duration" => (ctx.duration_ms >= 2000).then(|| seg(format!("took {}", seconds(ctx.duration_ms)), Col::Ansi(3), None)),
        // (Only over SSH, like Starship.)
        "host" => i.ssh.then(|| seg(format!("{}@{}", i.user, i.host), Col::Ansi(6), None)),
        "gcloud" => c.gcloud.as_ref().map(|(acct, proj)| {
            let who = acct.split('@').next().unwrap_or("");
            let p = if proj.is_empty() { String::new() } else { format!("({proj})") };
            seg(format!("\u{2601} {who}{p}"), Col::Ansi(6), Some(tint(ctx)))
        }),
        "aws" => c.aws.as_ref().map(|(p, r)| seg(format!("\u{f270} {p}{}", if r.is_empty() { String::new() } else { format!(" ({r})") }), Col::Ansi(3), None)),
        "kube" => c.kube.as_ref().map(|k| seg(format!("\u{2388} {k}"), Col::Ansi(4), None)),
        "venv" => c.venv.as_ref().map(|v| seg(format!("\u{e73c} {v}"), Col::Ansi(3), None)),
        "nix" => c.nix.as_ref().map(|n| seg(format!("\u{f313} {n}"), Col::Ansi(4), None)),
        "user" => (!i.user.is_empty()).then(|| seg(i.user.clone(), Col::Ansi(6), None)),
        "time" => Some(seg(hhmm(), text_dim(ctx), None)),
        _ => None,
    }
}

/// A widget's look with its style (prompt_styles) laid over.
fn styled(st: Option<&serde_json::Value>, mut x: Seg) -> Seg {
    let Some(st) = st else { return x };
    let s = |k: &str| st.get(k).and_then(|v| v.as_str()).filter(|v| !v.is_empty());
    if let Some(c) = s("fg").and_then(col) {
        x.fg = c;
    }
    match s("bg") {
        Some("none") => x.bg = None,
        Some(b) => {
            if let Some(c) = col(b) {
                x.bg = Some(c);
            }
        }
        None => {}
    }
    if let Some(p) = s("prefix") {
        x.text = format!("{p}{}", x.text);
    }
    if let Some(p) = s("suffix") {
        x.text.push_str(p);
    }
    if let Some(b) = st.get("bold").and_then(|v| v.as_bool()) {
        x.bold = Some(b);
    }
    x
}

/// Starship's runs with a widget's style laid over.
fn styled_runs(st: Option<&serde_json::Value>, runs: Vec<R>) -> Vec<R> {
    let Some(st) = st else { return runs };
    let s = |k: &str| st.get(k).and_then(|v| v.as_str()).filter(|v| !v.is_empty());
    let bold = st.get("bold").and_then(|v| v.as_bool());
    let fg = s("fg");
    let bg = s("bg");
    let mut r: Vec<R> = runs
        .into_iter()
        .map(|mut x| {
            if let Some(f) = fg {
                x.sty.fg = col(f);
            }
            match bg {
                Some("none") => x.sty.bg = None,
                Some(b) => x.sty.bg = col(b),
                None => {}
            }
            if let Some(b) = bold {
                x.sty.bold = b;
            }
            x
        })
        .collect();
    let edge_fg = r.first().and_then(|x| x.sty.fg);
    let extra = |t: &str| {
        let sty = Sty { fg: fg.and_then(col).or(edge_fg), bg: bg.filter(|b| *b != "none").and_then(col), bold: bold.unwrap_or(false), ..Default::default() };
        R::new(t, sty)
    };
    if let Some(p) = s("prefix") {
        r.insert(0, extra(p));
    }
    if let Some(p) = s("suffix") {
        r.push(extra(p));
    }
    r
}

/// A widget as runs: a pill (a background) between rounded caps, else text.
/// (Git's branch mark: Powerline's branch glyph.)
fn seg_runs(s: &Seg) -> Vec<R> {
    let text = if s.branch_icon { format!("\u{e0a0} {}", s.text) } else { s.text.clone() };
    match s.bg {
        Some(bg) => {
            let cap = Sty::fg(bg);
            let body = Sty { fg: Some(s.fg), bg: Some(bg), bold: s.bold.unwrap_or(true), ..Default::default() };
            vec![R::new("\u{e0b6}", cap), R::new(text, body), R::new("\u{e0b4}", cap)]
        }
        None => vec![R::new(text, Sty { fg: Some(s.fg), bold: s.bold.unwrap_or(false), ..Default::default() })],
    }
}

// ---- the path's folders, each a click away -----------------------------------------

/// The folders of a path shown in `t` (Tessera's or Starship's directory:
/// "~/src/tessera", "/usr/bin", "…/a/b"), matched against the shell's folder:
/// [(start, end, folder, last)] in characters of `t`, or [].
fn path_parts(t: &str, cwd: &str, home: &str) -> Vec<(usize, usize, String, bool)> {
    if cwd.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = t.chars().collect();
    let ws = |i: usize| chars[i].is_whitespace();
    // (Each word's start, with the longest stretch first, so a folder with
    // a space in its name ("My Drive") is one path; then shorter, so a
    // prompt that says more after the path ("… on main") still finds it.)
    for s in 0..chars.len() {
        if ws(s) || (s > 0 && !ws(s - 1)) {
            continue;
        }
        let ends: Vec<usize> = (s + 1..=chars.len()).rev().filter(|&e| !ws(e - 1) && (e == chars.len() || ws(e))).collect();
        for e in ends {
            if let Some(p) = path_at(&chars, s, e, cwd, home) {
                return p;
            }
        }
    }
    Vec::new()
}

/// chars[s..e] as the shell's folder (or its tail): its folders, each from
/// the end matched against cwd and its parents, or None.
fn path_at(chars: &[char], s: usize, e: usize, cwd: &str, home: &str) -> Option<Vec<(usize, usize, String, bool)>> {
    let base = |d: &str| d.rsplit('/').next().unwrap_or("").to_string();
    let parent = |d: &str| match d.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => d[..i].to_string(),
    };
    let tok: String = chars[s..e].iter().collect();
    if !tok.contains('/') && tok != "~" && tok != base(cwd) {
        return None;
    }
    let comps: Vec<&str> = tok.split('/').collect();
    // The shell's folder and its parents, from the end.
    let mut dir = cwd.trim_end_matches('/').to_string();
    if dir.is_empty() {
        dir = "/".into();
    }
    let mut parts: Vec<(usize, usize, String, bool)> = Vec::new();
    let mut pos = e as isize;
    for j in (0..comps.len()).rev() {
        let c = comps[j];
        let start = pos - c.chars().count() as isize;
        if c.is_empty() && j == 0 && dir == "/" {
            parts.insert(0, (s, s + 1, "/".into(), false));
            break;
        }
        if c == "~" {
            if dir != home {
                return None;
            }
            parts.insert(0, (start as usize, pos as usize, home.to_string(), false));
            break;
        }
        if c == "…" || c == "..." {
            break;
        }
        if c != base(&dir) {
            return None;
        }
        if !c.is_empty() {
            parts.insert(0, (start as usize, pos as usize, dir.clone(), false));
        }
        dir = parent(&dir);
        pos = start - 1;
    }
    if parts.is_empty() {
        return None;
    }
    parts.last_mut().unwrap().3 = true;
    Some(parts)
}

/// A widget's runs with its name on them, and the folders of a path in
/// them runs of their own: a parent goes there (Hit::Path), the shell's
/// folder opens its choices (Hit::Widget("dir")).
fn with_hits(runs: Vec<R>, name: &str, cwd: &str, home: &str) -> Vec<R> {
    let mut out = Vec::new();
    for mut r in runs {
        r.hit = Hit::Widget(name.to_string());
        let parts = if is_cap(&r.t) || r.fill { Vec::new() } else { path_parts(&r.t, cwd, home) };
        if parts.is_empty() {
            out.push(r);
            continue;
        }
        let chars: Vec<char> = r.t.chars().collect();
        let mut at = 0;
        for (s, e, p, last) in parts {
            if s > at {
                out.push(r.with_text(chars[at..s].iter().collect()));
            }
            let mut piece = r.with_text(chars[s..e].iter().collect());
            piece.hit = if last { Hit::Widget("dir".into()) } else { Hit::Path(p) };
            out.push(piece);
            at = e;
        }
        if at < chars.len() {
            out.push(r.with_text(chars[at..].iter().collect()));
        }
    }
    out
}

// ---- lines too long for the bar: wrapped, never cut short -------------------------

/// Runs past `room` columns go on to another line, a long one (the folder)
/// broken after a "/" (or a space), its pill closed and opened again around
/// the break. (Starship's $fill counts nothing: it's stretched afterwards.)
fn wrap(lines: Vec<Vec<R>>, room: usize) -> Vec<Vec<R>> {
    if room <= 5 {
        return lines;
    }
    let mut out = Vec::new();
    for line in lines {
        let mut cur: Vec<R> = Vec::new();
        let mut w = 0usize;
        for i in 0..line.len() {
            let r = &line[i];
            let rw = if r.fill { 0 } else { wd(&r.t) };
            // (The pill it's in: its caps, to close it here and open it on
            // the next line. Its closing cap has to fit too.)
            let (mut open, mut close) = (None, None);
            if r.sty.bg.is_some() && !is_cap(&r.t) {
                open = line[..i].iter().rev().find(|x| is_cap(&x.t)).cloned();
                close = line[i + 1..].iter().find(|x| is_cap(&x.t)).cloned();
            }
            let cap_w = close.as_ref().map(|c| wd(&c.t)).unwrap_or(0);
            if w + rw + cap_w <= room || is_cap(&r.t) {
                cur.push(r.clone());
                w += rw;
                continue;
            }
            // (Blank, as Starship's padding: cut to what fits, never a line
            // of its own.)
            if r.t.trim().is_empty() {
                let ch: Vec<char> = r.t.chars().collect();
                let mut n = ch.len();
                while n > 0 && w + wdc(&ch[..n]) > room {
                    n -= 1;
                }
                if n > 0 {
                    cur.push(r.with_text(ch[..n].iter().collect()));
                    w += wdc(&ch[..n]);
                }
                continue;
            }
            let open_n = open.is_some() as usize;
            let mut rest: Vec<char> = r.t.chars().collect();
            while !rest.is_empty() {
                let left = room as isize - w as isize - cap_w as isize;
                // (The longest piece that fits, ending after a "/" or a space.)
                let mut cut: isize = -1;
                for k in 1..=rest.len() {
                    if k < rest.len() && rest[k - 1] != '/' && rest[k - 1] != ' ' {
                        continue;
                    }
                    if wdc(&rest[..k]) as isize <= left {
                        cut = k as isize;
                    } else {
                        break;
                    }
                }
                if cut < 0 && cur.len() <= open_n {
                    // (Nowhere to break and a line to itself: as much as fits.)
                    cut = 1;
                    while (cut as usize) < rest.len() && wdc(&rest[..cut as usize + 1]) as isize <= left {
                        cut += 1;
                    }
                }
                // (Only spaces past the break: they're dropped, not wrapped.)
                if cut > 0 && (cut as usize) < rest.len() && rest[cut as usize..].iter().all(|c| c.is_whitespace()) {
                    rest.truncate(cut as usize);
                }
                if cut as usize >= rest.len() && cut > 0 {
                    w += wdc(&rest);
                    cur.push(r.with_text(rest.iter().collect()));
                    break;
                }
                if cut > 0 {
                    cur.push(r.with_text(rest[..cut as usize].iter().collect()));
                }
                if let Some(c) = &close {
                    // (The pill's open cap with nothing after it yet: it
                    // moves to the next line; else the pill is closed here.)
                    if cur.last().is_some_and(|l| is_cap(&l.t)) {
                        cur.pop();
                    } else if !cur.is_empty() {
                        cur.push(c.clone());
                    }
                }
                out.push(std::mem::take(&mut cur));
                if let Some(o) = &open {
                    cur.push(o.clone());
                }
                w = open.as_ref().map(|o| wd(&o.t)).unwrap_or(0);
                rest.drain(..cut.max(0) as usize);
            }
        }
        out.push(cur);
    }
    out.retain(|l| !l.is_empty());
    out
}

/// Starship's $fill stretched to what's left of `room` on its line, its
/// characters repeated and cut there.
fn stretch_fill(lines: &mut [Vec<R>], room: usize) {
    for line in lines.iter_mut() {
        let fixed: usize = line.iter().filter(|r| !r.fill).map(|r| wd(&r.t)).sum();
        let n = room.saturating_sub(fixed);
        for r in line.iter_mut().filter(|r| r.fill) {
            let pat: Vec<char> = r.t.chars().filter(|c| unicode_width::UnicodeWidthChar::width(*c).unwrap_or(0) > 0).collect();
            let mut t = String::new();
            let mut tw = 0;
            if !pat.is_empty() {
                'fill: loop {
                    for c in &pat {
                        let cw = unicode_width::UnicodeWidthChar::width(*c).unwrap_or(1);
                        if tw + cw > n {
                            break 'fill;
                        }
                        t.push(*c);
                        tw += cw;
                    }
                }
            }
            r.t = t;
            r.fill = false;
        }
        line.retain(|r| !r.t.is_empty());
    }
}

// ---- the bar ------------------------------------------------------------------------

struct Block {
    lines: Vec<Vec<R>>,
    /// Starship's whole prompt: its $fill stretched to the room.
    fill: bool,
}

fn pad(n: usize) -> R {
    R::new(" ".repeat(n), Sty::default())
}

fn is_pad(r: &R) -> bool {
    r.hit == Hit::None && r.sty.bg.is_none() && r.t.chars().all(|c| c == ' ')
}

fn to_line(l: Vec<R>) -> Line {
    l.into_iter().filter(|r| !r.t.is_empty()).map(|r| Span { text: r.t, sty: r.sty, hit: r.hit }).collect()
}

/// The blocks laid out: the right side one line (a space between widgets);
/// the left side's widgets side by side, each wrapped to the room it has
/// before the right side (one with too little room starts a row of its own).
fn arrange(left: Vec<Block>, right: Vec<Block>, width: usize) -> Bar {
    let mut rl: Vec<R> = Vec::new();
    for b in right {
        for l in b.lines {
            if l.is_empty() {
                continue;
            }
            if !rl.is_empty() {
                rl.push(pad(1));
            }
            rl.extend(l);
        }
    }
    let rw = line_w(&rl);
    let full = width.saturating_sub(if rw > 0 { rw + 2 } else { 0 });
    let mut rows: Vec<Vec<Vec<Vec<R>>>> = vec![Vec::new()];
    let mut x = 0usize;
    for b in left {
        if b.lines.iter().all(|l| l.is_empty()) {
            continue;
        }
        let natural = b.lines.iter().map(|l| l.iter().filter(|r| !r.fill).map(|r| wd(&r.t)).sum::<usize>()).max().unwrap_or(0);
        if x > 0 && x + natural > full && full.saturating_sub(x) < 20 {
            rows.push(Vec::new());
            x = 0;
        }
        let room = full.saturating_sub(x);
        let mut lines = wrap(b.lines, room);
        if b.fill {
            stretch_fill(&mut lines, room);
        }
        let bw = lines.iter().map(|l| line_w(l)).max().unwrap_or(0);
        rows.last_mut().unwrap().push(lines);
        x += bw + 1;
    }
    let mut out: Vec<Line> = Vec::new();
    for row in rows.into_iter().filter(|r| !r.is_empty()) {
        let h = row.iter().map(|b| b.len()).max().unwrap_or(0);
        for li in 0..h {
            let mut line: Vec<R> = Vec::new();
            for (k, b) in row.iter().enumerate() {
                if k > 0 {
                    line.push(pad(1));
                }
                let bw = b.iter().map(|l| line_w(l)).max().unwrap_or(0);
                let off = (h - b.len()) / 2;
                if li >= off && li < off + b.len() {
                    let l = &b[li - off];
                    line.extend(l.iter().cloned());
                    let lw = line_w(l);
                    if bw > lw {
                        line.push(pad(bw - lw));
                    }
                } else {
                    line.push(pad(bw));
                }
            }
            while line.last().is_some_and(is_pad) {
                line.pop();
            }
            out.push(to_line(line));
        }
    }
    Bar { left: out, right: to_line(rl) }
}

/// The bar for this folder and last command, as the user's Tessera draws
/// it. Runs git and Starship (in parallel where it can): call it off the
/// UI thread.
pub fn bar(ctx: &BarCtx) -> Bar {
    let s = settings();
    if !s.editor_prompt {
        return Bar::default();
    }
    bar_with(ctx, &s)
}

fn bar_with(ctx: &BarCtx, s: &Settings) -> Bar {
    let layout = layout(s);
    let cfg = starship_config(s);
    let cfg = cfg.as_deref();
    let home = home();
    let cwd = plain_cwd(&ctx.cwd);
    let width = ctx.width as usize;
    let divider = layout.iter().position(|w| w == "|");
    let is_right = |i: usize| divider.is_some_and(|d| i > d);
    let has = |n: &str| layout.iter().any(|w| w == n);
    let need_git = has("git");
    let need_cloud = ["gcloud", "aws", "kube", "venv", "nix"].iter().any(|n| has(n));
    let mods: Vec<String> = layout.iter().filter_map(|w| w.strip_prefix("star:").map(String::from)).collect();

    // Everything but Starship's whole left prompt, at once: its room
    // depends on how wide the rest is.
    let (info, star_mods, star_right) = std::thread::scope(|sc| {
        let info = sc.spawn(|| info(&cwd, need_git, need_cloud));
        let right = has("starship_right").then(|| sc.spawn(|| starship_right(ctx, &cwd, cfg)));
        let ms: Vec<_> = mods.iter().map(|m| (m.clone(), sc.spawn(|| starship_module(m, ctx, &cwd, cfg)))).collect();
        let info = info.join().unwrap_or_default();
        let right = right.and_then(|h| h.join().ok()).unwrap_or_default();
        let ms: HashMap<String, Vec<R>> = ms.into_iter().filter_map(|(n, h)| h.join().ok().map(|r| (n, r))).collect();
        (info, ms, right)
    });

    let style = |n: &str| s.styles.get(n).filter(|v| v.is_object());
    // Each widget's runs (Starship's whole left prompt: later).
    let mut blocks: Vec<Option<Block>> = Vec::new();
    for w in &layout {
        let b = if w == "|" || w == "starship" {
            None
        } else if w == "starship_right" {
            Some(styled_runs(style(w), star_right.clone()))
        } else if let Some(m) = w.strip_prefix("star:") {
            star_mods.get(m).filter(|r| !r.is_empty()).map(|r| styled_runs(style(w), r.clone()))
        } else {
            segment(w, ctx, &info, &home).map(|x| seg_runs(&styled(style(w), x)))
        };
        blocks.push(b.filter(|r| !r.is_empty()).map(|r| Block { lines: vec![with_hits(r, w, &cwd, &home)], fill: false }));
    }
    // Starship's whole prompt: as wide as the room the other widgets leave
    // it on the left (its $fill pads to it); on the right, no fill.
    if let Some(si) = layout.iter().position(|w| w == "starship") {
        let cols = if is_right(si) {
            1
        } else {
            let others: usize = layout
                .iter()
                .enumerate()
                .filter(|(i, _)| !is_right(*i) && *i != si)
                .filter_map(|(i, _)| blocks[i].as_ref())
                .map(|b| b.lines.iter().map(|l| line_w(l)).max().unwrap_or(0) + 1)
                .sum();
            let rw: usize = layout.iter().enumerate().filter(|(i, _)| is_right(*i)).filter_map(|(i, _)| blocks[i].as_ref()).map(|b| line_w(&b.lines[0]) + 1).sum();
            width.saturating_sub(others + if rw > 0 { rw + 1 } else { 0 }).max(1)
        };
        let lines: Vec<Vec<R>> = starship_lines(ctx, &cwd, cols, cfg).into_iter().map(|l| with_hits(styled_runs(style("starship"), l), "starship", &cwd, &home)).collect();
        if !lines.is_empty() {
            blocks[si] = Some(Block { lines, fill: true });
        }
    }
    let (mut left, mut right) = (Vec::new(), Vec::new());
    for (i, b) in blocks.into_iter().enumerate() {
        if let Some(b) = b {
            if is_right(i) {
                right.push(b);
            } else {
                left.push(b);
            }
        }
    }
    arrange(left, right, width)
}

// ---- titles, for tooltips --------------------------------------------------------------

const BUILTINS: &[(&str, &str, &str)] = &[
    ("dir", "Folder", "The current folder; click for folders to go to"),
    ("git", "Git", "Branch and changes; click to switch branches"),
    ("status", "Exit status", "The last command's, when it failed; click to fix"),
    ("duration", "Command time", "How long the last command took (over 2 s)"),
    ("venv", "Python virtualenv", ""),
    ("nix", "Nix shell", ""),
    ("kube", "Kubernetes context", "Click to switch"),
    ("aws", "AWS profile", "Click to switch"),
    ("gcloud", "gcloud account", "Click to switch configuration"),
    ("host", "user@host (over SSH)", ""),
    ("user", "User", ""),
    ("time", "Clock", ""),
    ("starship", "Starship's whole prompt", "As your starship.toml draws it"),
    ("starship_right", "Starship's right prompt", ""),
];

/// A widget's name as Tessera shows it ("Git", "git branch" for star:git_branch).
pub fn widget_name(name: &str) -> String {
    if let Some(m) = name.strip_prefix("star:") {
        return m.replace('_', " ");
    }
    BUILTINS.iter().find(|b| b.0 == name).map(|b| b.1.to_string()).unwrap_or_else(|| name.to_string())
}

/// A widget's tooltip ("Git — click for choices"). The exit status's click
/// fixes the command instead (choices("status") is empty).
pub fn widget_title(name: &str) -> String {
    if name == "status" {
        return format!("{} — click to fix", widget_name(name));
    }
    format!("{} — click for choices", widget_name(name))
}

/// A folder of the path's tooltip: "cd <folder>", or for the shell's own
/// folder (Hit::Widget("dir") on it) what its click shows.
pub fn path_title(path: &str) -> String {
    format!("cd {path}")
}

pub const HERE_TITLE: &str = "Recent and frequent folders, and this one's";

// ---- choices --------------------------------------------------------------------------

fn sh_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "/._-+~:@".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn run(cwd: &str, prog: &str, args: &[&str]) -> String {
    let mut c = std::process::Command::new(prog);
    c.args(args).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    if Path::new(cwd).is_dir() {
        c.current_dir(cwd);
    }
    c.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

fn item(label: impl Into<String>, cmd: impl Into<String>, current: bool) -> Choice {
    Choice { label: label.into(), cmd: cmd.into(), section: false, current }
}

fn head(label: &str) -> Choice {
    Choice { label: label.into(), cmd: String::new(), section: true, current: false }
}

/// What clicking a widget offers (empty: nothing; "status" is a fix, not
/// choices). The folder's: this session's folders first ("This tab"), then
/// recent and frequent ones, the folders here, and up.
pub fn choices(widget: &str, ctx: &BarCtx) -> Vec<Choice> {
    let cwd = plain_cwd(&ctx.cwd);
    let mut c = widget_choices(widget, &cwd);
    // (The folder's choices: this session's own folders first.)
    if c.iter().any(|o| o.section && o.label == "Up") && ctx.visited.len() > 1 {
        let home = home();
        let mine: Vec<Choice> = ctx.visited.iter().filter(|p| **p != cwd).map(|p| item(tilde(p, &home), format!("cd {}", sh_quote(p)), false)).collect();
        let cmds: std::collections::HashSet<String> = mine.iter().map(|o| o.cmd.clone()).collect();
        let mut all = vec![head("This tab")];
        all.extend(mine);
        all.extend(c.into_iter().filter(|o| o.section || !cmds.contains(&o.cmd)));
        // (A heading left with nothing under it goes.)
        c = all.iter().enumerate().filter(|(i, o)| !o.section || all.get(i + 1).is_some_and(|n| !n.section)).map(|(_, o)| o.clone()).collect();
    }
    c
}

/// The cd history (the toolbar's Folders button): the folder widget's choices.
pub fn cd_history(ctx: &BarCtx) -> Vec<Choice> {
    choices("dir", ctx)
}

fn widget_choices(widget: &str, cwd: &str) -> Vec<Choice> {
    let w = widget.strip_prefix("star:").unwrap_or(widget);
    let env: HashMap<String, String> = std::env::vars().collect();
    let home = PathBuf::from(env.get("HOME").cloned().unwrap_or_default());
    let h = home.to_string_lossy().to_string();
    let mut out: Vec<Choice> = Vec::new();
    match w {
        "dir" | "directory" => {
            let here = PathBuf::from(cwd);
            // Where you've been lately and often first, then this folder's,
            // then up.
            let recent: Vec<String> = frecent_dirs().into_iter().map(|(p, _)| p).filter(|p| p != cwd && !p.starts_with("/tmp/") && !p.starts_with("/nix/store")).take(15).collect();
            if !recent.is_empty() {
                out.push(head("Recent and frequent"));
                // (Full paths, ~ for home: where each is, plainly.)
                for p in recent {
                    out.push(item(tilde(&p, &h), format!("cd {}", sh_quote(&p)), false));
                }
            }
            let show = |p: &Path| tilde(&p.to_string_lossy(), &h);
            let mut subs: Vec<PathBuf> = std::fs::read_dir(&here)
                .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir() && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))).collect())
                .unwrap_or_default();
            subs.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
            if !subs.is_empty() {
                out.push(head("Folders here"));
                for p in subs.iter().take(60) {
                    out.push(item(format!("{}/", show(p)), format!("cd {}", sh_quote(&p.to_string_lossy())), false));
                }
            }
            out.push(head("Up"));
            let mut p = here.parent();
            while let Some(d) = p {
                out.push(item(show(d), format!("cd {}", sh_quote(&d.to_string_lossy())), false));
                p = d.parent();
            }
            if here != home && !home.as_os_str().is_empty() {
                out.push(item("~ (home)", "cd", false));
            }
            out.push(item("Previous folder", "cd -", false));
        }
        "git" | "git_branch" | "git_status" | "git_commit" | "git_state" | "git_metrics" => {
            let current = run(cwd, "git", &["branch", "--show-current"]).trim().to_string();
            let branches = run(cwd, "git", &["for-each-ref", "--sort=-committerdate", "--format=%(refname:short)", "refs/heads"]);
            if !branches.trim().is_empty() {
                out.push(head("Switch to branch"));
                for b in branches.lines().take(40) {
                    out.push(item(b, format!("git switch {}", sh_quote(b)), b == current));
                }
            }
            out.push(head("Git"));
            for (l, c) in [("Status", "git status"), ("Log", "git log --oneline -20"), ("Diff", "git diff"), ("Pull", "git pull"), ("Stash", "git stash")] {
                out.push(item(l, c, false));
            }
        }
        "kube" | "kubernetes" => {
            let current = run(cwd, "kubectl", &["config", "current-context"]).trim().to_string();
            out.push(head("Kubernetes context"));
            for c in run(cwd, "kubectl", &["config", "get-contexts", "-o", "name"]).lines() {
                out.push(item(c, format!("kubectl config use-context {}", sh_quote(c)), c == current));
            }
        }
        "aws" => {
            let current = env.get("AWS_PROFILE").cloned().unwrap_or_default();
            out.push(head("AWS profile"));
            let mut names: Vec<String> = Vec::new();
            for f in [".aws/config", ".aws/credentials"] {
                for l in std::fs::read_to_string(home.join(f)).unwrap_or_default().lines() {
                    if let Some(n) = l.trim().strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
                        let n = n.trim_start_matches("profile ").trim().to_string();
                        if !names.contains(&n) {
                            names.push(n);
                        }
                    }
                }
            }
            for n in names {
                out.push(item(n.clone(), format!("export AWS_PROFILE={}", sh_quote(&n)), n == current));
            }
            if !current.is_empty() {
                out.push(item("No profile", "unset AWS_PROFILE", false));
            }
        }
        "gcloud" => {
            let gdir = home.join(".config/gcloud");
            let current = std::fs::read_to_string(gdir.join("active_config")).unwrap_or_default().trim().to_string();
            out.push(head("gcloud configuration"));
            if let Ok(d) = std::fs::read_dir(gdir.join("configurations")) {
                let mut names: Vec<String> = d.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_prefix("config_").map(String::from)).collect();
                names.sort();
                for n in names {
                    out.push(item(n.clone(), format!("gcloud config configurations activate {}", sh_quote(&n)), n == current));
                }
            }
        }
        "venv" | "python" | "conda" => {
            if env.contains_key("VIRTUAL_ENV") {
                out.push(item("Deactivate the virtualenv", "deactivate", false));
            }
            for v in [".venv", "venv"] {
                if Path::new(cwd).join(v).join("bin/activate").is_file() {
                    out.push(item(format!("Activate {v}"), format!("source {v}/bin/activate"), false));
                }
            }
        }
        "nix" | "nix_shell" => {
            if env.contains_key("IN_NIX_SHELL") {
                out.push(item("Leave the Nix shell", "exit", false));
            } else if Path::new(cwd).join("flake.nix").is_file() {
                out.push(item("nix develop", "nix develop", false));
            }
        }
        "direnv" => {
            out.push(item("Allow", "direnv allow", false));
            out.push(item("Reload", "direnv reload", false));
        }
        "jobs" => {
            out.push(item("List jobs", "jobs -l", false));
            out.push(item("Bring the last to the front", "fg", false));
        }
        "package" | "nodejs" | "rust" | "golang" => {
            for (f, l, c) in [("package.json", "npm install", "npm install"), ("Cargo.toml", "cargo build", "cargo build"), ("go.mod", "go build", "go build ./...")] {
                if Path::new(cwd).join(f).is_file() {
                    out.push(item(l, c, false));
                }
            }
        }
        // Starship's whole prompt: the choices of what it shows that apply
        // here (git in a repository, contexts, environments), then its config.
        "starship" | "starship_right" => {
            let in_git = !run(cwd, "git", &["rev-parse", "--git-dir"]).trim().is_empty();
            for (name, title) in [("git", ""), ("kube", ""), ("aws", ""), ("gcloud", ""), ("venv", "Python"), ("nix", "Nix")] {
                if name == "git" && !in_git {
                    continue;
                }
                let c = widget_choices(name, cwd);
                if !title.is_empty() && !c.is_empty() {
                    out.push(head(title));
                }
                out.extend(c);
            }
            let cfg = env.get("STARSHIP_CONFIG").cloned().unwrap_or_else(|| home.join(".config/starship.toml").to_string_lossy().into_owned());
            out.push(head("Starship"));
            out.push(item("Edit starship.toml", format!("${{EDITOR:-vi}} {}", sh_quote(&cfg)), false));
            // (A heading left with nothing under it goes.)
            let keep: Vec<bool> = (0..out.len()).map(|i| !out[i].section || out.get(i + 1).is_some_and(|n| !n.section)).collect();
            let mut k = keep.into_iter();
            out.retain(|_| k.next().unwrap_or(true));
        }
        _ => {}
    }
    out
}

// ---- recent and frequent folders (Tessera's complete::frecent_dirs) ------------------

/// A command from a history: when (Unix seconds, 0 unknown) and where ("" unknown).
struct Hist {
    cmd: String,
    t: u64,
    cwd: String,
}

fn read_lossy(p: &Path) -> Option<String> {
    std::fs::read(p).ok().map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// The histories Tessera reads (zsh, bash, fish, its own with each
/// command's folder) and tessh's own, each command once at its latest.
fn histories() -> Vec<Hist> {
    let h = PathBuf::from(home());
    let mut all: Vec<Hist> = Vec::new();
    let zsh = std::env::var_os("HISTFILE").map(PathBuf::from).filter(|p| p.to_string_lossy().contains("zsh")).unwrap_or_else(|| h.join(".zsh_history"));
    if let Some(text) = read_lossy(&zsh) {
        for line in text.lines() {
            let (t, cmd) = match line.strip_prefix(": ").and_then(|r| r.split_once(';')) {
                Some((meta, cmd)) => (meta.split(':').next().and_then(|s| s.trim().parse().ok()).unwrap_or(0), cmd),
                None => (0, line),
            };
            all.push(Hist { cmd: cmd.to_string(), t, cwd: String::new() });
        }
    }
    if let Some(text) = read_lossy(&h.join(".bash_history")) {
        let mut t = 0;
        for line in text.lines() {
            if let Some(ts) = line.strip_prefix('#').and_then(|s| s.trim().parse::<u64>().ok()) {
                t = ts;
                continue;
            }
            all.push(Hist { cmd: line.to_string(), t, cwd: String::new() });
            t = 0;
        }
    }
    if let Some(text) = read_lossy(&h.join(".local/share/fish/fish_history")) {
        for line in text.lines() {
            if let Some(c) = line.strip_prefix("- cmd: ") {
                all.push(Hist { cmd: c.to_string(), t: 0, cwd: String::new() });
            } else if let Some(w) = line.trim().strip_prefix("when: ") {
                if let Some(e) = all.last_mut() {
                    e.t = w.trim().parse().unwrap_or(0);
                }
            }
        }
    }
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    if let Some(text) = read_lossy(&data.join("tessh/history")) {
        for line in text.lines().filter(|l| !l.contains("<\\n>")) {
            all.push(Hist { cmd: line.to_string(), t: 0, cwd: String::new() });
        }
    }
    if let Some(text) = read_lossy(&state_dir().join("history.jsonl")) {
        for l in text.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(l) {
                let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
                all.push(Hist { cmd: s("cmd"), t: v.get("t").and_then(|x| x.as_u64()).unwrap_or(0), cwd: s("cwd") });
            }
        }
    }
    // (Each command once, at its latest.)
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<Hist> = all.into_iter().rev().filter(|e| seen.insert((e.cmd.clone(), e.cwd.clone()))).collect();
    out.reverse();
    out
}

/// Every command, as Tessera's input editor has them (src/history.rs): the
/// shells' history files, tessh's own, and Tessera's (every window's and
/// tessh's, with times), merged by time, each once at its latest; newest
/// last.
pub fn shared_history() -> Vec<String> {
    let h = PathBuf::from(home());
    let mut lists: Vec<Vec<(u64, String)>> = vec![];
    let zsh = std::env::var_os("HISTFILE").map(PathBuf::from).filter(|p| p.to_string_lossy().contains("zsh")).unwrap_or_else(|| h.join(".zsh_history"));
    if let Some(text) = read_lossy(&zsh) {
        let mut v: Vec<(u64, String)> = vec![];
        let mut cont = false;
        for line in text.lines() {
            if cont {
                if let Some(last) = v.last_mut() {
                    last.1.push('\n');
                    last.1.push_str(line.strip_suffix('\\').unwrap_or(line));
                }
            } else {
                let (t, cmd) = match line.strip_prefix(": ").and_then(|r| r.split_once(';')) {
                    Some((meta, cmd)) => (meta.split(':').next().and_then(|s| s.trim().parse().ok()).unwrap_or(0), cmd),
                    None => (0, line),
                };
                v.push((t, cmd.strip_suffix('\\').unwrap_or(cmd).to_string()));
            }
            cont = line.ends_with('\\');
        }
        lists.push(v);
    }
    if let Some(text) = read_lossy(&h.join(".bash_history")) {
        let mut v = vec![];
        let mut t = 0;
        for line in text.lines() {
            if let Some(ts) = line.strip_prefix('#').and_then(|s| s.trim().parse::<u64>().ok()) {
                t = ts;
                continue;
            }
            v.push((t, line.to_string()));
            t = 0;
        }
        lists.push(v);
    }
    if let Some(text) = read_lossy(&h.join(".local/share/fish/fish_history")) {
        let mut v: Vec<(u64, String)> = vec![];
        for line in text.lines() {
            if let Some(c) = line.strip_prefix("- cmd: ") {
                v.push((0, c.to_string()));
            } else if let Some(w) = line.trim().strip_prefix("when: ") {
                if let Some(e) = v.last_mut() {
                    e.0 = w.trim().parse().unwrap_or(0);
                }
            }
        }
        lists.push(v);
    }
    let data = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| h.join(".local/share"));
    if let Some(text) = read_lossy(&data.join("tessh/history")) {
        lists.push(text.lines().filter(|l| !l.trim().is_empty()).map(|l| (0, l.replace("<\\n>", "\n"))).collect());
    }
    if let Some(text) = read_lossy(&state_dir().join("history.jsonl")) {
        lists.push(
            text.lines()
                .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                .map(|v| (v.get("t").and_then(|x| x.as_u64()).unwrap_or(0), v.get("cmd").and_then(|x| x.as_str()).unwrap_or("").to_string()))
                .collect(),
        );
    }
    // (By time where known; one without keeps its place after the timed one before it.)
    let mut all: Vec<(u64, usize, String)> = vec![];
    let mut seq = 0;
    for list in lists {
        let mut last = 0;
        for (t, c) in list {
            let t = if t == 0 { last } else { t };
            last = t;
            all.push((t, seq, c));
            seq += 1;
        }
    }
    all.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = all.into_iter().rev().map(|x| x.2).filter(|c| !c.trim().is_empty() && seen.insert(c.trim().to_string())).collect();
    out.truncate(50_000);
    out.reverse();
    out
}

/// A command run here, into Tessera's history: every window and tessh
/// session sees it, with its folder and time.
pub fn record_history(cmd: &str, cwd: &str) {
    let cmd = cmd.trim_end();
    if cmd.trim().is_empty() {
        return;
    }
    let d = state_dir();
    let _ = std::fs::create_dir_all(&d);
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(d.join("history.jsonl")) {
        use std::io::Write;
        let _ = writeln!(f, "{}", serde_json::json!({"cmd": cmd, "t": t, "cwd": cwd, "lang": "tessh"}));
    }
}

/// Folders by how recently and often they've been used: zoxide's, the
/// folders commands ran in (Tessera's history), and where `cd`s went.
pub fn frecent_dirs() -> Vec<(String, f64)> {
    static CACHE: Mutex<Option<(Instant, Vec<(String, f64)>)>> = Mutex::new(None);
    if let Some((at, v)) = CACHE.lock().unwrap().as_ref() {
        if at.elapsed() < Duration::from_secs(20) {
            return v.clone();
        }
    }
    let mut score: HashMap<String, f64> = HashMap::new();
    if let Ok(o) = std::process::Command::new("zoxide").args(["query", "--list", "--score"]).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output() {
        for l in String::from_utf8_lossy(&o.stdout).lines() {
            if let Some((sc, path)) = l.trim().split_once(' ') {
                if let Ok(v) = sc.trim().parse::<f64>() {
                    *score.entry(path.trim().to_string()).or_default() += v;
                }
            }
        }
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // (zoxide's weights: within an hour x4, a day x2, a week x0.5, older x0.25.)
    let weight = |t: u64| {
        if t == 0 {
            return 0.25;
        }
        match now.saturating_sub(t) {
            a if a < 3600 => 4.0,
            a if a < 86400 => 2.0,
            a if a < 604800 => 0.5,
            _ => 0.25,
        }
    };
    let home = home();
    for e in histories() {
        let cwd = plain_cwd(&e.cwd);
        if !cwd.is_empty() {
            *score.entry(cwd.clone()).or_default() += weight(e.t);
        }
        // (cd somewhere: there, as the shell would resolve it.)
        let mut w = e.cmd.split_whitespace();
        if matches!(w.next(), Some("cd" | "pushd")) {
            if let Some(a) = w.next().filter(|a| !a.starts_with('-')) {
                let a = a.trim_matches(|c| c == '\'' || c == '"');
                let p = if let Some(r) = a.strip_prefix('~') {
                    format!("{home}{r}")
                } else if a.starts_with('/') {
                    a.to_string()
                } else if !cwd.is_empty() {
                    format!("{cwd}/{a}")
                } else {
                    continue;
                };
                let p = Path::new(&p).components().fold(PathBuf::new(), |mut acc, c| {
                    match c {
                        std::path::Component::ParentDir => {
                            acc.pop();
                        }
                        std::path::Component::CurDir => {}
                        c => acc.push(c),
                    }
                    acc
                });
                *score.entry(p.to_string_lossy().trim_end_matches('/').to_string()).or_default() += weight(e.t) * 2.0;
            }
        }
    }
    let mut v: Vec<(String, f64)> = score.into_iter().filter(|(p, _)| !p.is_empty() && Path::new(p).is_dir()).collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.truncate(400);
    *CACHE.lock().unwrap() = Some((Instant::now(), v.clone()));
    v
}

// ---- tests ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn text(l: &Line) -> String {
        l.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn ansi_runs_to_spans() {
        let r = runs("\u{1b}[38;2;203;166;247m\u{e0b6}\u{1b}[1;48;2;203;166;247;38;2;17;17;27m ~ \u{1b}[0m x\u{1b}[31my\u{1b}[38;5;9mz\u{1b}[38;5;196mw");
        assert_eq!(r[0].t, "\u{e0b6}");
        assert_eq!(r[0].sty.fg, Some(Col::Rgb(0xcb, 0xa6, 0xf7)));
        assert_eq!(r[1].sty.bg, Some(Col::Rgb(0xcb, 0xa6, 0xf7)));
        assert_eq!(r[1].sty.fg, Some(Col::Rgb(17, 17, 27)));
        assert!(r[1].sty.bold);
        assert_eq!(r[1].t, " ~ ");
        assert_eq!(r[2].t, " x");
        assert_eq!(r[2].sty, Sty::default());
        assert_eq!(r[3].sty.fg, Some(Col::Ansi(1)));
        assert_eq!(r[4].sty.fg, Some(Col::Ansi(9)));
        assert_eq!(r[5].sty.fg, Some(Col::Rgb(255, 0, 0)));
        assert_eq!(col("p5"), Some(Col::Ansi(5)));
        assert_eq!(col("#0a0b0c"), Some(Col::Rgb(10, 11, 12)));
        assert_eq!(col(""), None);
    }

    #[test]
    fn git_porcelain() {
        let g = parse_status("# branch.oid abcdef123456\n# branch.head main\n# branch.ab +2 -1\n1 M. N... 100644 100644 100644 a b x.rs\n1 .M N... 100644 100644 100644 a b y.rs\n? new.txt\nu UU N... 1 2 3 4 a b c z.rs\n");
        assert_eq!(g.branch, "main");
        assert!(!g.detached);
        assert_eq!((g.ahead, g.behind), (2, 1));
        assert_eq!((g.staged, g.modified, g.untracked, g.conflicts), (1, 1, 1, 1));
        let d = parse_status("# branch.oid abcdef123456\n# branch.head (detached)\n");
        assert!(d.detached);
        assert_eq!(d.branch, "abcdef1");
        // A segment from it.
        let ctx = BarCtx { cwd: "/x".into(), width: 80, ..Default::default() };
        let info = Info { git: g, ..Default::default() };
        let s = segment("git", &ctx, &info, "").unwrap();
        assert_eq!(s.text, "main  =1 +1 !1 ?1 ⇡2 ⇣1");
        assert_eq!(s.fg, Col::Ansi(3));
        assert!(s.branch_icon);
    }

    #[test]
    fn path_hits() {
        let ctx = BarCtx { cwd: "/home/u/src/tessera".into(), width: 80, colors: Some(((200, 200, 200), (20, 20, 20))), ..Default::default() };
        let seg = segment("dir", &ctx, &Info::default(), "/home/u").unwrap();
        assert_eq!(seg.text, "\u{f07c}  ~/src/tessera");
        assert_eq!(seg.fg, Col::Rgb(20, 20, 20));
        let r = with_hits(seg_runs(&seg), "dir", &ctx.cwd, "/home/u");
        let t: Vec<(&str, &Hit)> = r.iter().map(|x| (x.t.as_str(), &x.hit)).collect();
        let w = Hit::Widget("dir".into());
        assert_eq!(
            t,
            [
                ("\u{e0b6}", &w),
                ("\u{f07c}  ", &w),
                ("~", &Hit::Path("/home/u".into())),
                ("/", &w),
                ("src", &Hit::Path("/home/u/src".into())),
                ("/", &w),
                ("tessera", &w),
                ("\u{e0b4}", &w),
            ]
        );
        // Starship's way: "/usr/bin" outside home, and a truncated one.
        let p = path_parts(" /usr/bin ", "/usr/bin", "/home/u");
        assert_eq!(p, [(1, 2, "/".into(), false), (2, 5, "/usr".into(), false), (6, 9, "/usr/bin".into(), true)]);
        let p = path_parts("…/a/b", "/x/y/a/b", "/home/u");
        assert_eq!(p, [(2, 3, "/x/y/a".into(), false), (4, 5, "/x/y/a/b".into(), true)]);
        assert!(path_parts("main", "/x/y", "/home/u").is_empty());
        // A folder with a space in its name, and more after the path.
        let p = path_parts(" ~/My Drive/CD backups ", "/home/u/My Drive/CD backups", "/home/u");
        assert_eq!(p, [(1, 2, "/home/u".into(), false), (3, 11, "/home/u/My Drive".into(), false), (12, 22, "/home/u/My Drive/CD backups".into(), true)]);
        let p = path_parts("~/src/tessera on main", "/home/u/src/tessera", "/home/u");
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn wrapping() {
        let ctx = BarCtx { cwd: "/home/u/projects/some-long-folder/deeper/still".into(), width: 24, ..Default::default() };
        let seg = segment("dir", &ctx, &Info::default(), "/home/u").unwrap();
        let r = with_hits(seg_runs(&seg), "dir", &ctx.cwd, "/home/u");
        let lines = wrap(vec![r], 24);
        let ls: Vec<String> = lines.iter().map(|l| l.iter().map(|r| r.t.as_str()).collect()).collect();
        for l in &ls {
            assert!(wd(l) <= 24, "{l:?} too wide");
            // (Each line's pill is closed and opened.)
            assert!(l.starts_with('\u{e0b6}') && l.ends_with('\u{e0b4}'), "{l:?}");
        }
        assert!(ls.len() >= 2);
        let joined: String = ls.iter().map(|l| l.trim_start_matches('\u{e0b6}').trim_end_matches('\u{e0b4}')).collect();
        assert_eq!(joined, "\u{f07c}  ~/projects/some-long-folder/deeper/still");
        // Broken after a "/".
        assert!(ls[0].trim_end_matches('\u{e0b4}').ends_with('/'));
        // The folders keep their hits.
        assert!(lines.iter().flatten().any(|r| r.t == "deeper" && r.hit == Hit::Path("/home/u/projects/some-long-folder/deeper".into())));

        // Plain text breaks at a space; a word too long for any line is cut.
        let lines = wrap(vec![vec![R::new("aaaa bbbb cccc", Sty::default()), R::new("xxxxxxxxxxxxxxxx", Sty::default())]], 10);
        let ls: Vec<String> = lines.iter().map(|l| l.iter().map(|r| r.t.as_str()).collect()).collect();
        assert_eq!(ls, ["aaaa bbbb ", "cccc", "xxxxxxxxxx", "xxxxxx"]);
    }

    #[test]
    fn fill_and_arrange() {
        // Starship's line with $fill ("─" run) found by asking wider.
        let at = |w: usize| format!("\u{1b}[34m~/x\u{1b}[0m{}\u{1b}[33m12:00\u{1b}[0m\n❯ ", "─".repeat(w - 8));
        let lines = starship_parse(&at(20), 20, at);
        assert_eq!(lines.len(), 1, "the prompt character's line is dropped");
        assert!(lines[0].iter().any(|r| r.fill && r.t.chars().all(|c| c == '─')));
        let bar = arrange(vec![Block { lines, fill: true }], vec![Block { lines: vec![vec![R::new("R", Sty::default())]], fill: false }], 30);
        assert_eq!(text(&bar.right), "R");
        assert_eq!(bar.left.len(), 1);
        // (Stretched to the room before the right side: 30 - (1 + 2).)
        assert_eq!(super::super::width(&bar.left[0]), 27);
        assert!(text(&bar.left[0]).starts_with("~/x─") && text(&bar.left[0]).ends_with("─12:00"));

        // Widgets side by side, a space apart; a shorter one centered on a taller one.
        let a = Block { lines: vec![vec![R::new("AA", Sty::default())]], fill: false };
        let mut b1 = R::new("B1", Sty::default());
        b1.hit = Hit::Widget("b".into());
        let b2 = R { t: "B2".into(), ..b1.clone() };
        let b3 = R { t: "B3".into(), ..b1.clone() };
        let mut a = a;
        a.lines[0][0].hit = Hit::Widget("a".into());
        let bar = arrange(vec![a, Block { lines: vec![vec![b1], vec![b2], vec![b3]], fill: false }], vec![], 40);
        let ls: Vec<String> = bar.left.iter().map(text).collect();
        assert_eq!(ls, ["   B1", "AA B2", "   B3"]);
        assert_eq!(bar.left[1][0].hit, Hit::Widget("a".into()));
    }

    #[test]
    fn settings_and_styles() {
        let v = serde_json::json!({"prompt_layout": ["dir", "|", "time"], "prompt_engine": "builtin", "prompt_styles": {"dir": {"bg": "none", "fg": "p2", "prefix": "> "}}});
        let s = settings_from(Some(&v));
        assert_eq!(layout(&s), ["dir", "|", "time"]);
        let ctx = BarCtx { cwd: "/tmp".into(), width: 80, ..Default::default() };
        let seg = styled(s.styles.get("dir"), segment("dir", &ctx, &Info::default(), "/home/u").unwrap());
        assert_eq!(seg.bg, None);
        assert_eq!(seg.fg, Col::Ansi(2));
        assert!(seg.text.starts_with("> \u{f07c}"));
        let d = settings_from(Some(&serde_json::json!({"prompt_engine": "builtin"})));
        assert_eq!(layout(&d)[0], "dir");
        let bar = bar_with(&ctx, &s);
        assert!(text(&bar.left[0]).contains("/tmp"));
        assert_eq!(text(&bar.right).len(), 5);
        assert_eq!(widget_title("git"), "Git — click for choices");
        assert_eq!(widget_title("star:git_branch"), "git branch — click for choices");
    }

    #[test]
    fn dir_choices() {
        let ctx = BarCtx { cwd: "/usr".into(), visited: vec!["/usr".into(), "/etc".into()], ..Default::default() };
        let c = choices("dir", &ctx);
        assert_eq!(c[0], head("This tab"));
        assert_eq!(c[1].cmd, "cd /etc");
        assert!(c.iter().any(|x| x.cmd == "cd /"));
        assert!(c.iter().any(|x| x.cmd == "cd /usr/bin"));
        assert_eq!(c.iter().filter(|x| x.cmd == "cd /etc").count(), 1);
        assert_eq!(cd_history(&ctx).len(), c.len());
    }
}
