//! The prompt bar's widgets beyond Tessera's own (qml/PromptBar.qml):
//! Starship's modules one by one (`starship module <name>`), the modules a
//! Starship format lists (to split a whole Starship prompt into them), and
//! what clicking a widget offers (folders to go to, branches to switch to,
//! contexts and profiles), each a shell command.
//!
//! Starship comes from PATH, or from nixpkgs when it isn't installed (built
//! once in the background; the modules show once it's there).

use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

fn on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(name)).find(|f| f.is_file()))
}

/// Starship: installed, or fetched from nixpkgs (None until that's done).
pub fn starship_bin() -> Option<PathBuf> {
    if let Some(p) = on_path("starship") {
        return Some(p);
    }
    static NIX: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
    static STARTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let cell = NIX.get_or_init(|| Mutex::new(None));
    if let Some(p) = cell.lock().unwrap().clone() {
        return Some(p);
    }
    if on_path("nix").is_some() && !STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        std::thread::spawn(move || {
            let out = std::process::Command::new("nix").args(["build", "nixpkgs#starship", "--no-link", "--print-out-paths"]).output();
            if let Ok(o) = out {
                let dir = String::from_utf8_lossy(&o.stdout).lines().last().unwrap_or("").trim().to_string();
                let bin = Path::new(&dir).join("bin/starship");
                if bin.is_file() {
                    *NIX.get().unwrap().lock().unwrap() = Some(bin);
                }
            }
        });
    }
    None
}

/// Every module Starship has (`starship module --list`).
pub fn module_list() -> Vec<String> {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    if let Some(l) = LIST.get() {
        return l.clone();
    }
    let Some(bin) = starship_bin() else { return Vec::new() };
    let out = std::process::Command::new(bin).args(["module", "--list"]).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let list: Vec<String> = out
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
        .map(String::from)
        .collect();
    if !list.is_empty() {
        let _ = LIST.set(list.clone());
    }
    list
}

/// The config Starship's prompt uses here (Tessera's or the user's own).
fn config() -> Option<PathBuf> {
    (crate::settings::load().prompt_config == "tessera").then(crate::prompt::tessera_config_path)
}

/// One module, drawn: colored runs (as prompt::runs), [] when it shows nothing.
pub fn module(name: &str, cwd: &str, status: i32, duration_ms: u64, pid: u32) -> Vec<serde_json::Value> {
    let Some(bin) = starship_bin() else { return Vec::new() };
    let mut cmd = std::process::Command::new(bin);
    cmd.args(["module", name, "--status", &status.to_string(), "--cmd-duration", &duration_ms.to_string(), "--jobs", "0"]);
    if let Some(env) = crate::prompt::shell_env(pid) {
        cmd.env_clear().envs(env);
    }
    cmd.env("TERM", "xterm-256color").env_remove("STARSHIP_SHELL").env("PWD", cwd);
    if let Some(c) = config() {
        cmd.env("STARSHIP_CONFIG", c);
    }
    if Path::new(cwd).is_dir() {
        cmd.current_dir(cwd);
    }
    cmd.stderr(std::process::Stdio::null());
    let text = cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let text = text.trim_end_matches(['\n', ' ']);
    if text.trim().is_empty() {
        return Vec::new();
    }
    crate::prompt::runs(text)
}

/// Several modules at once (in parallel): {name: runs}.
pub fn modules(names: &[String], cwd: &str, status: i32, duration_ms: u64, pid: u32) -> serde_json::Value {
    let handles: Vec<_> = names
        .iter()
        .map(|n| {
            let (n, cwd) = (n.clone(), cwd.to_string());
            std::thread::spawn(move || (n.clone(), module(&n, &cwd, status, duration_ms, pid)))
        })
        .collect();
    let mut out = serde_json::Map::new();
    for h in handles {
        if let Ok((n, r)) = h.join() {
            out.insert(n, json!(r));
        }
    }
    serde_json::Value::Object(out)
}

/// The modules the prompt's format shows, in order (for splitting the
/// whole Starship prompt into widgets). Line breaks, the prompt character
/// and fill are left out; `|` marks where the right prompt starts.
pub fn format_modules() -> Vec<String> {
    let Some(bin) = starship_bin() else { return Vec::new() };
    let mut cmd = std::process::Command::new(bin);
    cmd.args(["print-config", "format", "right_format"]);
    if let Some(c) = config() {
        cmd.env("STARSHIP_CONFIG", c);
    }
    let text = cmd.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    parse_format(&text)
}

fn parse_format(text: &str) -> Vec<String> {
    // ("$all" stands for the list in the comment Starship prints.)
    let all = text.lines().find_map(|l| l.strip_prefix("# $all is shorthand for ")).unwrap_or("").to_string();
    let mut out = Vec::new();
    for line in text.lines().filter(|l| !l.starts_with('#')) {
        let Some((key, val)) = line.split_once('=') else { continue };
        let key = key.trim();
        if key != "format" && key != "right_format" {
            continue;
        }
        if key == "right_format" {
            out.push("|".to_string());
        }
        let val = val.replace("$all", &all);
        let b = val.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'$' {
                let rest = &val[i + 1..];
                let name: String = if let Some(r) = rest.strip_prefix('{') {
                    r.chars().take_while(|c| *c != '}').collect()
                } else {
                    rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect()
                };
                i += 1 + name.len();
                if !name.is_empty() && !["line_break", "character", "fill", "all"].contains(&name.as_str()) {
                    let w = format!("star:{name}");
                    if !out.contains(&w) {
                        out.push(w);
                    }
                }
            } else {
                i += 1;
            }
        }
    }
    if out.last().is_some_and(|l| l == "|") {
        out.pop();
    }
    out
}

fn sh_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "/._-+~:@".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn run(cwd: &str, prog: &str, args: &[&str]) -> String {
    let mut c = std::process::Command::new(prog);
    c.args(args).stderr(std::process::Stdio::null());
    if Path::new(cwd).is_dir() {
        c.current_dir(cwd);
    }
    c.output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}

/// What clicking a widget offers: [{label, cmd, current, section}] (cmd: a
/// shell command run as if typed; "" for a heading).
pub fn choices(widget: &str, cwd: &str, pid: u32) -> String {
    let w = widget.strip_prefix("star:").unwrap_or(widget);
    let env: std::collections::HashMap<String, String> = crate::prompt::shell_env(pid).map(|e| e.into_iter().collect()).unwrap_or_else(|| std::env::vars().collect());
    let home = PathBuf::from(env.get("HOME").cloned().unwrap_or_default());
    let mut out: Vec<serde_json::Value> = Vec::new();
    let item = |label: String, cmd: String, current: bool| json!({"label": label, "cmd": cmd, "current": current});
    let head = |t: &str| json!({"label": t, "cmd": "", "section": true});
    match w {
        "dir" | "directory" => {
            let here = PathBuf::from(cwd);
            // Where you've been lately and often first (the cd history: zoxide
            // and the commands' folders), then this folder's, then up.
            let recent: Vec<String> = crate::complete::frecent_dirs()
                .into_iter()
                .map(|(p, _)| p)
                .filter(|p| p != cwd && !p.starts_with("/tmp/") && !p.starts_with("/nix/store"))
                .take(15)
                .collect();
            if !recent.is_empty() {
                let h = home.to_string_lossy().to_string();
                out.push(head("Recent and frequent"));
                for p in recent {
                    let shown = if !h.is_empty() && (p == h || p.starts_with(&format!("{h}/"))) { format!("~{}", &p[h.len()..]) } else { p.clone() };
                    out.push(item(shown, format!("cd {}", sh_quote(&p)), false));
                }
            }
            let show = |p: &Path| {
                let s = p.to_string_lossy().to_string();
                let h = home.to_string_lossy().to_string();
                if !h.is_empty() && (s == h || s.starts_with(&format!("{h}/"))) { format!("~{}", &s[h.len()..]) } else { s }
            };
            let mut subs: Vec<PathBuf> = std::fs::read_dir(&here)
                .map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir() && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.'))).collect())
                .unwrap_or_default();
            subs.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
            if !subs.is_empty() {
                out.push(head("Folders here"));
                for p in subs.iter().take(60) {
                    out.push(item(format!("{}/", p.file_name().unwrap_or_default().to_string_lossy()), format!("cd {}", sh_quote(&p.to_string_lossy())), false));
                }
            }
            out.push(head("Up"));
            let mut p = here.parent();
            while let Some(d) = p {
                out.push(item(show(d), format!("cd {}", sh_quote(&d.to_string_lossy())), false));
                p = d.parent();
            }
            if here != home && !home.as_os_str().is_empty() {
                out.push(item("~ (home)".into(), "cd".into(), false));
            }
            out.push(item("Previous folder".into(), "cd -".into(), false));
        }
        "git" | "git_branch" | "git_status" | "git_commit" | "git_state" | "git_metrics" => {
            let current = run(cwd, "git", &["branch", "--show-current"]).trim().to_string();
            let branches = run(cwd, "git", &["for-each-ref", "--sort=-committerdate", "--format=%(refname:short)", "refs/heads"]);
            if !branches.trim().is_empty() {
                out.push(head("Switch to branch"));
                for b in branches.lines().take(40) {
                    out.push(item(b.to_string(), format!("git switch {}", sh_quote(b)), b == current));
                }
            }
            out.push(head("Git"));
            for (l, c) in [("Status", "git status"), ("Log", "git log --oneline -20"), ("Diff", "git diff"), ("Pull", "git pull"), ("Stash", "git stash")] {
                out.push(item(l.into(), c.into(), false));
            }
        }
        "kube" | "kubernetes" => {
            let current = run(cwd, "kubectl", &["config", "current-context"]).trim().to_string();
            out.push(head("Kubernetes context"));
            for c in run(cwd, "kubectl", &["config", "get-contexts", "-o", "name"]).lines() {
                out.push(item(c.into(), format!("kubectl config use-context {}", sh_quote(c)), c == current));
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
                out.push(item("No profile".into(), "unset AWS_PROFILE".into(), false));
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
                out.push(item("Deactivate the virtualenv".into(), "deactivate".into(), false));
            }
            for v in [".venv", "venv"] {
                if Path::new(cwd).join(v).join("bin/activate").is_file() {
                    out.push(item(format!("Activate {v}"), format!("source {v}/bin/activate"), false));
                }
            }
        }
        "nix" | "nix_shell" => {
            if env.contains_key("IN_NIX_SHELL") {
                out.push(item("Leave the Nix shell".into(), "exit".into(), false));
            } else if Path::new(cwd).join("flake.nix").is_file() {
                out.push(item("nix develop".into(), "nix develop".into(), false));
            }
        }
        "direnv" => {
            out.push(item("Allow".into(), "direnv allow".into(), false));
            out.push(item("Reload".into(), "direnv reload".into(), false));
        }
        "jobs" => {
            out.push(item("List jobs".into(), "jobs -l".into(), false));
            out.push(item("Bring the last to the front".into(), "fg".into(), false));
        }
        "package" | "nodejs" | "rust" | "golang" => {
            for (f, l, c) in [("package.json", "npm install", "npm install"), ("Cargo.toml", "cargo build", "cargo build"), ("go.mod", "go build", "go build ./...")] {
                if Path::new(cwd).join(f).is_file() {
                    out.push(item(l.into(), c.into(), false));
                }
            }
        }
        _ => {}
    }
    serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        let t = "# $all is shorthand for $username$directory$git_branch$line_break$character\nformat = \"$all\"\nright_format = \"$time\"\n";
        assert_eq!(parse_format(t), ["star:username", "star:directory", "star:git_branch", "|", "star:time"]);
        let t2 = "format = \"$directory([](fg:pink)$git_branch[](fg:pink) )$fill${custom.foo}$line_break$character\"\nright_format = \"\"\n";
        assert_eq!(parse_format(t2), ["star:directory", "star:git_branch", "star:custom.foo"]);
    }

    #[test]
    fn dir_choices() {
        let c: Vec<serde_json::Value> = serde_json::from_str(&choices("dir", "/usr", 0)).unwrap();
        assert!(c.iter().any(|x| x["cmd"] == "cd /"));
        assert!(c.iter().any(|x| x["cmd"] == "cd /usr/bin"));
    }
}
