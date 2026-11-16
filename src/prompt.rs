//! What the input editor's prompt bar shows about the folder: git state,
//! user and host. (The bar itself is qml/PromptBar.qml.)

use serde_json::json;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// {"branch", "detached", "staged", "modified", "untracked", "conflicts",
///  "ahead", "behind", "user", "host", "ssh"} for `cwd` (git asked at most
/// once a second per folder).
pub fn info(cwd: &str) -> String {
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
