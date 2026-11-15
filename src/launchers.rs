//! What the + button's menu offers: installed shells and SSH hosts.

use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct Shell {
    pub name: String,
    pub path: String,
}

#[derive(Serialize)]
pub struct Launchers {
    pub shells: Vec<Shell>,
    pub ssh_hosts: Vec<String>,
}

pub fn launchers() -> Launchers {
    Launchers { shells: shells(), ssh_hosts: ssh_hosts() }
}

/// Login shells from /etc/shells plus common ones on PATH, one per name.
fn shells() -> Vec<Shell> {
    let mut paths: Vec<String> = std::fs::read_to_string("/etc/shells")
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect();
    let path_var = std::env::var("PATH").unwrap_or_default();
    for name in ["bash", "zsh", "fish", "nu", "xonsh", "elvish", "pwsh", "dash", "ksh", "tcsh"] {
        for dir in path_var.split(':') {
            let p = Path::new(dir).join(name);
            if p.is_file() {
                paths.push(p.display().to_string());
                break;
            }
        }
    }
    let mut out: Vec<Shell> = Vec::new();
    for p in paths {
        let path = Path::new(&p);
        if !path.exists() {
            continue;
        }
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        if name.is_empty() || name == "nologin" || name == "sh" {
            continue;
        }
        if let Some(existing) = out.iter_mut().find(|s| s.name == name) {
            // Prefer the system profile's copy on NixOS.
            if p.starts_with("/run/current-system") {
                existing.path = p.clone();
            }
            continue;
        }
        out.push(Shell { name, path: p });
    }
    out
}

/// Host aliases from ~/.ssh/config (and files it Includes), without patterns.
fn ssh_hosts() -> Vec<String> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let mut hosts = Vec::new();
    let mut seen = std::collections::HashSet::new();
    read_ssh_config(&home.join(".ssh/config"), &home, &mut hosts, &mut seen, 0);
    hosts
}

fn read_ssh_config(file: &Path, home: &Path, hosts: &mut Vec<String>, seen: &mut std::collections::HashSet<PathBuf>, depth: u32) {
    if depth > 4 || !seen.insert(file.to_path_buf()) {
        return;
    }
    let Ok(text) = std::fs::read_to_string(file) else { return };
    for line in text.lines() {
        let line = line.trim();
        let mut parts = line.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let key = parts.next().unwrap_or("").to_ascii_lowercase();
        let rest = parts.next().unwrap_or("").trim();
        match key.as_str() {
            "host" => {
                for h in rest.split_whitespace() {
                    if !h.contains(['*', '?', '!']) && !hosts.iter().any(|x| x == h) {
                        hosts.push(h.to_string());
                    }
                }
            }
            "include" => {
                for pat in rest.split_whitespace() {
                    let p = if let Some(r) = pat.strip_prefix("~/") {
                        home.join(r)
                    } else if pat.starts_with('/') {
                        PathBuf::from(pat)
                    } else {
                        home.join(".ssh").join(pat)
                    };
                    // Simple globs: a trailing * in the file name.
                    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                    if let (Some(prefix), Some(dir)) = (name.strip_suffix('*'), p.parent()) {
                        if let Ok(rd) = std::fs::read_dir(dir) {
                            let mut files: Vec<PathBuf> = rd
                                .flatten()
                                .map(|e| e.path())
                                .filter(|f| f.file_name().is_some_and(|n| n.to_string_lossy().starts_with(prefix)))
                                .collect();
                            files.sort();
                            for f in files {
                                read_ssh_config(&f, home, hosts, seen, depth + 1);
                            }
                        }
                    } else {
                        read_ssh_config(&p, home, hosts, seen, depth + 1);
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ssh_config_hosts() {
        let dir = std::env::temp_dir().join(format!("tessera-ssh-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".ssh/conf.d")).unwrap();
        std::fs::write(dir.join(".ssh/config"), "Include conf.d/*\nHost web db\n  User x\nHost *.internal\nHost=box\n").unwrap();
        std::fs::write(dir.join(".ssh/conf.d/a"), "Host nas\n").unwrap();
        let mut hosts = Vec::new();
        let mut seen = std::collections::HashSet::new();
        super::read_ssh_config(&dir.join(".ssh/config"), &dir, &mut hosts, &mut seen, 0);
        assert_eq!(hosts, vec!["nas", "web", "db", "box"]);
        let _ = std::fs::remove_dir_all(dir);
    }
}
