//! The assistant panel's conversations, kept under the state folder
//! (`<state>/assistant/<id>.json`, one file each): reopened from the panel's
//! history dropdown and found by its search.

use serde_json::{json, Value};
use std::path::PathBuf;

fn dir() -> PathBuf {
    crate::settings::state_dir().join("assistant")
}

/// Ids are made by the panel; keep them to a safe file name.
fn path(id: &str) -> Option<PathBuf> {
    if id.is_empty() || id.len() > 64 || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return None;
    }
    Some(dir().join(format!("{id}.json")))
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// What a message shows (the question as typed, not with the terminal's lines added).
fn shown(m: &Value) -> &str {
    m.get("shown").and_then(Value::as_str).filter(|s| !s.is_empty()).or_else(|| m.get("content").and_then(Value::as_str)).unwrap_or("")
}

/// A title from the first question: its first line, at most 80 characters.
fn title_of(messages: &[Value]) -> String {
    let first = messages.iter().find(|m| m.get("role").and_then(Value::as_str) == Some("user")).map(shown).unwrap_or("");
    let line = first.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    if line.chars().count() > 80 {
        format!("{}…", line.chars().take(79).collect::<String>().trim_end())
    } else if line.is_empty() {
        "New conversation".into()
    } else {
        line.to_string()
    }
}

/// Save a conversation (its messages, [{role, content, shown?, status?}]).
/// An empty one is removed rather than kept.
pub fn save(id: &str, messages: &[Value]) -> Result<(), String> {
    let p = path(id).ok_or("bad conversation id")?;
    if messages.is_empty() {
        let _ = std::fs::remove_file(&p);
        return Ok(());
    }
    let created = std::fs::read(&p)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v.get("created").and_then(Value::as_u64))
        .unwrap_or_else(now);
    let doc = json!({ "id": id, "title": title_of(messages), "created": created, "updated": now(), "messages": messages });
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("json.tmp");
    std::fs::write(&tmp, serde_json::to_vec(&doc).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

pub fn load(id: &str) -> Option<Value> {
    serde_json::from_slice(&std::fs::read(path(id)?).ok()?).ok()
}

pub fn delete(id: &str) {
    if let Some(p) = path(id) {
        let _ = std::fs::remove_file(p);
    }
}

fn all() -> Vec<Value> {
    let mut out: Vec<Value> = std::fs::read_dir(dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| serde_json::from_slice(&std::fs::read(e.path()).ok()?).ok())
        .collect();
    out.sort_by_key(|v: &Value| std::cmp::Reverse(v.get("updated").and_then(Value::as_u64).unwrap_or(0)));
    out
}

fn summary(v: &Value) -> Value {
    json!({
        "id": v.get("id").cloned().unwrap_or(Value::Null),
        "title": v.get("title").cloned().unwrap_or(Value::Null),
        "updated": v.get("updated").cloned().unwrap_or(Value::Null),
        "count": v.get("messages").and_then(Value::as_array).map_or(0, |a| a.len()),
    })
}

/// Every conversation, newest first: [{id, title, updated, count}].
pub fn list() -> Vec<Value> {
    all().iter().map(summary).collect()
}

/// Conversations whose text has every word of `query` (any case), newest
/// first, each with a snippet around the first match: [{id, title, updated,
/// count, snippet}].
pub fn search(query: &str) -> Vec<Value> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.is_empty() {
        return list();
    }
    let mut out = Vec::new();
    for v in all() {
        let msgs = v.get("messages").and_then(Value::as_array).cloned().unwrap_or_default();
        let texts: Vec<&str> = msgs.iter().filter(|m| m.get("role").and_then(Value::as_str) != Some("error")).map(shown).collect();
        let title = v.get("title").and_then(Value::as_str).unwrap_or("");
        let hay = format!("{}\n{}", title, texts.join("\n")).to_lowercase();
        if !words.iter().all(|w| hay.contains(w.as_str())) {
            continue;
        }
        let mut s = summary(&v);
        s["snippet"] = json!(snippet(&texts, &words[0]));
        out.push(s);
    }
    out
}

/// The line around the first message that has `word`, trimmed to ~100 characters.
fn snippet(texts: &[&str], word: &str) -> String {
    for t in texts {
        let lower = t.to_lowercase();
        let Some(at) = lower.find(word) else { continue };
        // Byte offsets in the lowercase copy can differ from the original's;
        // work in characters of the lowercase copy (good enough to show).
        let chars: Vec<char> = lower.chars().collect();
        let at_c = lower[..at].chars().count();
        let orig: Vec<char> = t.chars().collect();
        let src = if orig.len() == chars.len() { &orig } else { &chars };
        let start = at_c.saturating_sub(40);
        let end = (at_c + word.chars().count() + 60).min(src.len());
        let mut s: String = src[start..end].iter().collect::<String>().replace("```", " ").split_whitespace().collect::<Vec<_>>().join(" ");
        if start > 0 {
            s = format!("…{s}");
        }
        if end < src.len() {
            s.push('…');
        }
        return s;
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_and_snippets() {
        let m = vec![json!({"role": "user", "content": "how do I list files\n\n(The terminal's last lines:)", "shown": "how do I list files"})];
        assert_eq!(title_of(&m), "how do I list files");
        assert_eq!(title_of(&[]), "New conversation");
        assert_eq!(snippet(&["use ls -la to see them"], "ls"), "use ls -la to see them");
        assert!(path("../x").is_none());
        assert!(path("chat-abc123").is_some());
    }
}
