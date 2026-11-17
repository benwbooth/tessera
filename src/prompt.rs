//! What the input editor's prompt bar shows about the folder: git state,
//! user and host. (The bar itself is qml/PromptBar.qml.)

use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// {"branch", "detached", "staged", "modified", "untracked", "conflicts",
///  "ahead", "behind", "user", "host", "ssh"} for `cwd` (git asked at most
/// once a second per folder).
pub fn info(cwd: &str, pid: u32) -> String {
    static CACHE: Mutex<Option<HashMap<String, (Instant, serde_json::Value)>>> = Mutex::new(None);
    {
        let c = CACHE.lock().unwrap();
        if let Some((t, v)) = c.as_ref().and_then(|m| m.get(cwd)) {
            if t.elapsed() < Duration::from_millis(900) {
                return v.to_string();
            }
        }
    }
    let mut v = git(cwd);
    let user = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default();
    let host = std::fs::read_to_string("/etc/hostname").map(|h| h.trim().to_string()).ok().filter(|h| !h.is_empty())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .unwrap_or_default();
    v["user"] = json!(user);
    v["host"] = json!(host);
    v["ssh"] = json!(std::env::var_os("SSH_CONNECTION").is_some());
    v["cloud"] = cloud(pid);
    let mut c = CACHE.lock().unwrap();
    c.get_or_insert_with(HashMap::new).insert(cwd.to_string(), (Instant::now(), v.clone()));
    v.to_string()
}

fn git(cwd: &str) -> serde_json::Value {
    let out = std::process::Command::new("git")
        .args(["-C", cwd, "--no-optional-locks", "status", "--porcelain=v2", "--branch"])
        .stderr(std::process::Stdio::null())
        .output();
    let Ok(o) = out else { return json!({}) };
    if !o.status.success() {
        return json!({});
    }
    parse_status(&String::from_utf8_lossy(&o.stdout))
}

pub fn parse_status(text: &str) -> serde_json::Value {
    let (mut branch, mut detached, mut oid) = (String::new(), false, String::new());
    let (mut ahead, mut behind) = (0i64, 0i64);
    let (mut staged, mut modified, mut untracked, mut conflicts) = (0, 0, 0, 0);
    for line in text.lines() {
        if let Some(b) = line.strip_prefix("# branch.head ") {
            detached = b == "(detached)";
            branch = b.to_string();
        } else if let Some(o) = line.strip_prefix("# branch.oid ") {
            oid = o.chars().take(7).collect();
        } else if let Some(ab) = line.strip_prefix("# branch.ab ") {
            for p in ab.split_whitespace() {
                if let Some(n) = p.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = p.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("? ") {
            untracked += 1;
        } else if line.starts_with("u ") {
            conflicts += 1;
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            let xy: Vec<char> = line[2..4].chars().collect();
            if xy[0] != '.' {
                staged += 1;
            }
            if xy[1] != '.' {
                modified += 1;
            }
        }
    }
    if detached {
        branch = oid;
    }
    json!({"branch": branch, "detached": detached, "ahead": ahead, "behind": behind,
           "staged": staged, "modified": modified, "untracked": untracked, "conflicts": conflicts})
}

#[cfg(test)]
mod tests {
    #[test]
    fn git_status() {
        let v = super::parse_status("# branch.oid abcdef123456\n# branch.head main\n# branch.ab +2 -1\n1 M. N... 100644 100644 100644 a b x.rs\n1 .M N... 100644 100644 100644 a b y.rs\n? new.txt\n");
        assert_eq!(v["branch"], "main");
        assert_eq!((v["ahead"].as_i64(), v["behind"].as_i64()), (Some(2), Some(1)));
        assert_eq!((v["staged"].as_i64(), v["modified"].as_i64(), v["untracked"].as_i64()), (Some(1), Some(1), Some(1)));
    }
}

// ---- Starship ---------------------------------------------------------------

/// Whether `starship` is on PATH.
pub fn has_starship() -> bool {
    static FOUND: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *FOUND.get_or_init(|| {
        std::env::var_os("PATH").map(|p| std::env::split_paths(&p).any(|d| d.join(if cfg!(windows) { "starship.exe" } else { "starship" }).is_file())).unwrap_or(false)
    })
}

/// The shell's environment: as its integration last wrote it (at the
/// prompt), else /proc/<pid>/environ, so Starship and kernels see what the
/// shell has (nix-shell, a virtualenv, AWS_PROFILE, exports, ...).
pub fn shell_env(pid: u32) -> Option<Vec<(String, String)>> {
    if pid == 0 {
        return None;
    }
    // The shell integration writes its exported variables at each prompt
    // (exports since it started included); else what it started with.
    let live = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| "/tmp".into()).join("tessera-env").join(pid.to_string());
    let raw = match std::fs::read(&live) {
        Ok(r) if !r.is_empty() => r,
        _ if cfg!(target_os = "linux") => std::fs::read(format!("/proc/{pid}/environ")).ok()?,
        _ => return None,
    };
    Some(
        raw.split(|&b| b == 0)
            .filter_map(|kv| {
                let s = String::from_utf8_lossy(kv);
                let (k, v) = s.split_once('=')?;
                Some((k.to_string(), v.to_string()))
            })
            .collect(),
    )
}

/// Starship's prompt for this folder and last command, as lines of colored
/// runs: {"lines": [[{t, fg, bg, b}]], "right": [...]} (colors "#rrggbb",
/// "p<N>" for palette entry N, or ""). The final line (the prompt character
/// alone) is left out: the editor stands in for it.
pub fn starship(cwd: &str, status: i32, duration_ms: u64, width: u32, pid: u32) -> String {
    // Tessera's own prompt (built in Preferences), when that's the choice.
    let config = (crate::settings::load().prompt_config == "tessera").then(tessera_config_path);
    starship_with(cwd, status, duration_ms, width, pid, config.as_deref())
}

/// Starship with a given config file (None: the user's own).
pub fn starship_with(cwd: &str, status: i32, duration_ms: u64, width: u32, pid: u32, config: Option<&std::path::Path>) -> String {
    let run = |right: bool| -> String {
        let mut cmd = std::process::Command::new("starship");
        cmd.arg("prompt");
        if right {
            cmd.arg("--right");
        }
        cmd.args(["--status", &status.to_string(), "--cmd-duration", &duration_ms.to_string(), "--terminal-width", &width.max(1).to_string(), "--jobs", "0"]);
        if let Some(env) = shell_env(pid) {
            cmd.env_clear().envs(env);
        }
        cmd.env("TERM", "xterm-256color").env_remove("STARSHIP_SHELL").env("PWD", cwd);
        if let Some(c) = config {
            cmd.env("STARSHIP_CONFIG", c);
        }
        if std::path::Path::new(cwd).is_dir() {
            cmd.current_dir(cwd);
        }
        cmd.stderr(std::process::Stdio::null());
        cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
    };
    let left = run(false);
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
    let right_text = run(true);
    json!({
        "lines": lines.iter().map(|l| runs(l)).collect::<Vec<_>>(),
        "right": runs(right_text.trim_end_matches('\n')),
    })
    .to_string()
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

/// ANSI text -> runs of {t, fg, bg, b}.
pub fn runs(s: &str) -> Vec<serde_json::Value> {
    let (mut fg, mut bg, mut bold) = (String::new(), String::new(), false);
    let mut out: Vec<serde_json::Value> = Vec::new();
    let mut text = String::new();
    let mut chars = s.chars().peekable();
    let flush = |text: &mut String, out: &mut Vec<serde_json::Value>, fg: &str, bg: &str, bold: bool| {
        if !text.is_empty() {
            out.push(json!({"t": std::mem::take(text), "fg": fg, "bg": bg, "b": bold}));
        }
    };
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            // (zsh/bash prompt wrappers, if any.)
            if c != '\u{1}' && c != '\u{2}' {
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
        flush(&mut text, &mut out, &fg, &bg, bold);
        let nums: Vec<i32> = params.split(';').map(|n| n.parse().unwrap_or(0)).collect();
        let mut i = 0;
        while i < nums.len() {
            match nums[i] {
                0 => {
                    fg.clear();
                    bg.clear();
                    bold = false;
                }
                1 => bold = true,
                22 => bold = false,
                39 => fg.clear(),
                49 => bg.clear(),
                n @ 30..=37 => fg = format!("p{}", n - 30),
                n @ 90..=97 => fg = format!("p{}", n - 90 + 8),
                n @ 40..=47 => bg = format!("p{}", n - 40),
                n @ 100..=107 => bg = format!("p{}", n - 100 + 8),
                n @ (38 | 48) => {
                    let target = if n == 38 { &mut fg } else { &mut bg };
                    match nums.get(i + 1) {
                        Some(2) if i + 4 < nums.len() => {
                            *target = format!("#{:02x}{:02x}{:02x}", nums[i + 2], nums[i + 3], nums[i + 4]);
                            i += 4;
                        }
                        Some(5) if i + 2 < nums.len() => {
                            *target = xterm256(nums[i + 2]);
                            i += 2;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
    flush(&mut text, &mut out, &fg, &bg, bold);
    out
}

fn xterm256(n: i32) -> String {
    match n {
        0..=15 => format!("p{n}"),
        16..=231 => {
            let n = n - 16;
            let v = |x: i32| if x == 0 { 0 } else { 55 + x * 40 };
            format!("#{:02x}{:02x}{:02x}", v(n / 36), v((n / 6) % 6), v(n % 6))
        }
        _ => {
            let g = 8 + (n - 232) * 10;
            format!("#{g:02x}{g:02x}{g:02x}")
        }
    }
}

// ---- built-in cloud segments (without Starship) ------------------------------

/// gcloud's active account and project, AWS profile, Kubernetes context.
pub fn cloud(pid: u32) -> serde_json::Value {
    let env: std::collections::HashMap<String, String> = shell_env(pid).map(|e| e.into_iter().collect()).unwrap_or_else(|| std::env::vars().collect());
    let home = std::path::PathBuf::from(env.get("HOME").cloned().unwrap_or_default());
    let mut v = json!({});
    // gcloud: ~/.config/gcloud/active_config names configurations/config_<name>.
    let gdir = env.get("CLOUDSDK_CONFIG").map(std::path::PathBuf::from).unwrap_or_else(|| home.join(".config/gcloud"));
    let active = env.get("CLOUDSDK_ACTIVE_CONFIG_NAME").cloned().or_else(|| std::fs::read_to_string(gdir.join("active_config")).ok()).map(|s| s.trim().to_string());
    if let Some(name) = active.filter(|n| !n.is_empty()) {
        if let Ok(ini) = std::fs::read_to_string(gdir.join("configurations").join(format!("config_{name}"))) {
            let get = |key: &str| ini.lines().find_map(|l| l.split_once('=').filter(|(k, _)| k.trim() == key).map(|(_, v)| v.trim().to_string()));
            v["gcloud"] = json!({"account": get("account").unwrap_or_default(), "project": get("project").unwrap_or_default(), "config": name});
        }
    }
    if let Some(p) = env.get("AWS_PROFILE").or(env.get("AWS_VAULT")) {
        v["aws"] = json!({"profile": p, "region": env.get("AWS_REGION").or(env.get("AWS_DEFAULT_REGION")).cloned().unwrap_or_default()});
    }
    let kube = env.get("KUBECONFIG").and_then(|k| k.split(':').next().map(std::path::PathBuf::from)).unwrap_or_else(|| home.join(".kube/config"));
    if let Ok(text) = std::fs::read_to_string(kube) {
        if let Some(ctx) = text.lines().find_map(|l| l.strip_prefix("current-context:")) {
            let ctx = ctx.trim().trim_matches('"');
            if !ctx.is_empty() {
                v["kube"] = json!(ctx);
            }
        }
    }
    if let Some(venv) = env.get("VIRTUAL_ENV") {
        v["venv"] = json!(venv.rsplit('/').next().unwrap_or(venv));
    }
    if env.contains_key("IN_NIX_SHELL") {
        v["nix"] = json!(env.get("name").cloned().unwrap_or_else(|| "nix-shell".into()));
    }
    v
}

#[cfg(test)]
mod starship_tests {
    #[test]
    fn ansi_runs() {
        let r = super::runs("\u{1b}[38;2;203;166;247m\u{e0b6}\u{1b}[1;48;2;203;166;247;38;2;17;17;27m ~ \u{1b}[0m x\u{1b}[31my");
        assert_eq!(r[0]["fg"], "#cba6f7");
        assert_eq!(r[1]["bg"], "#cba6f7");
        assert_eq!(r[1]["b"], true);
        assert_eq!(r[1]["t"], " ~ ");
        assert_eq!(r[2]["t"], " x");
        assert_eq!(r[3]["fg"], "p1");
    }
}


// ---- the prompt builder ---------------------------------------------------------

/// Tessera's own Starship config (the user's starship.toml is left alone:
/// it may be managed elsewhere).
pub fn tessera_config_path() -> std::path::PathBuf {
    crate::settings::config_dir().join("starship.toml")
}

pub fn tessera_config() -> String {
    std::fs::read_to_string(tessera_config_path()).unwrap_or_else(|_| preset("pastel-powerline"))
}

pub fn save_tessera_config(toml: &str) -> Result<(), String> {
    let p = tessera_config_path();
    let _ = std::fs::create_dir_all(p.parent().unwrap());
    std::fs::write(p, toml).map_err(|e| e.to_string())
}

/// The user's own starship.toml (for starting from it).
pub fn user_config() -> String {
    let p = std::env::var_os("STARSHIP_CONFIG").map(std::path::PathBuf::from).unwrap_or_else(|| {
        std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config/starship.toml")
    });
    std::fs::read_to_string(p).unwrap_or_default()
}

pub fn presets() -> Vec<String> {
    std::process::Command::new("starship")
        .args(["preset", "--list"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_string).collect())
        .unwrap_or_default()
}

pub fn preset(name: &str) -> String {
    std::process::Command::new("starship")
        .args(["preset", name])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

/// A preview of a config: written to a scratch file, run for `cwd`.
pub fn preview(toml: &str, cwd: &str, width: u32, key: &str) -> String {
    let dir = std::env::temp_dir().join("tessera-prompt-preview");
    let _ = std::fs::create_dir_all(&dir);
    let safe: String = key.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').collect();
    let file = dir.join(format!("{safe}.toml"));
    if std::fs::write(&file, toml).is_err() {
        return String::new();
    }
    let r = starship_with(cwd, 1, 3400, width, 0, Some(&file));
    let _ = std::fs::remove_file(&file);
    r
}

/// The modules the builder offers switches for, with a label.
pub const MODULES: &[(&str, &str)] = &[
    ("os", "OS icon"), ("username", "User"), ("hostname", "Host"), ("directory", "Folder"), ("git_branch", "Git branch"),
    ("git_status", "Git status"), ("git_state", "Git state (rebase, merge)"), ("nix_shell", "Nix shell"), ("python", "Python"),
    ("nodejs", "Node.js"), ("rust", "Rust"), ("golang", "Go"), ("java", "Java"), ("docker_context", "Docker context"),
    ("kubernetes", "Kubernetes"), ("aws", "AWS"), ("gcloud", "Google Cloud"), ("azure", "Azure"), ("terraform", "Terraform"),
    ("cmd_duration", "Command duration"), ("status", "Exit status"), ("jobs", "Background jobs"), ("memory_usage", "Memory"),
    ("battery", "Battery"), ("time", "Time"), ("character", "Prompt character"),
];

/// Which modules are on in a config: {module: bool} (on unless disabled;
/// time, memory_usage and status are off unless enabled, as in Starship).
pub fn module_states(toml: &str) -> String {
    let doc = toml.parse::<toml_edit::DocumentMut>().unwrap_or_default();
    let mut out = serde_json::Map::new();
    for (m, label) in MODULES {
        let off_by_default = matches!(*m, "time" | "memory_usage" | "status" | "kubernetes" | "os" | "battery" | "azure");
        let disabled = doc.get(m).and_then(|t| t.get("disabled")).and_then(|d| d.as_bool()).unwrap_or(off_by_default);
        out.insert(m.to_string(), json!({"label": label, "on": !disabled}));
    }
    serde_json::Value::Object(out).to_string()
}

/// The config with a module switched on or off (comments and layout kept).
pub fn set_module(toml: &str, module: &str, on: bool) -> String {
    let mut doc = toml.parse::<toml_edit::DocumentMut>().unwrap_or_default();
    if doc.get(module).and_then(|t| t.as_table()).is_none() {
        doc[module] = toml_edit::table();
    }
    doc[module]["disabled"] = toml_edit::value(!on);
    doc.to_string()
}

#[cfg(test)]
mod builder_tests {
    #[test]
    fn modules() {
        let t = "# mine\n[git_branch]\nsymbol = \"x\"\n";
        let t2 = super::set_module(t, "git_branch", false);
        assert!(t2.contains("# mine") && t2.contains("disabled = true") && t2.contains("symbol"));
        let s: serde_json::Value = serde_json::from_str(&super::module_states(&t2)).unwrap();
        assert_eq!(s["git_branch"]["on"], false);
        assert_eq!(s["directory"]["on"], true);
        assert_eq!(s["time"]["on"], false);
    }
}
