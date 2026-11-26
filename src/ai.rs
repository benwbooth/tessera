//! AI for the input editor and the assistant sidebar: chat requests to any
//! OpenAI-compatible server (Ollama, llama.cpp, LM Studio, vLLM, LocalAI,
//! OpenAI/Codex, Gemini's OpenAI endpoint, OpenRouter, Groq, Mistral,
//! DeepSeek, xAI, ...) or Anthropic's Messages API (Claude), streamed.
//!
//! Models are named "provider-id/model" (e.g. "ollama/qwen3.5:4b"). Keys live
//! in the config folder's secrets.json (readable only by you), or are read
//! from an environment variable ("env:OPENAI_API_KEY").

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Provider {
    pub id: String,
    pub name: String,
    /// "openai" (any OpenAI-compatible server) or "anthropic".
    pub kind: String,
    pub base_url: String,
    /// Sent as reasoning_effort (e.g. "none" turns thinking off on Ollama);
    /// empty: not sent.
    pub reasoning_effort: String,
    /// Local servers: no key, cheap enough for checks as you type.
    pub local: bool,
}

/// Providers Tessera knows how to talk to, for the "Add provider" menu.
pub fn presets() -> Vec<Provider> {
    let p = |id: &str, name: &str, kind: &str, url: &str, local: bool, effort: &str| Provider {
        id: id.into(),
        name: name.into(),
        kind: kind.into(),
        base_url: url.into(),
        reasoning_effort: effort.into(),
        local,
    };
    vec![
        p("hearth", "Hearth AI hub", "openai", "", true, ""),
        p("ollama", "Ollama", "openai", "http://localhost:11434/v1", true, "none"),
        p("llamacpp", "llama.cpp server", "openai", "http://localhost:8080/v1", true, ""),
        p("lmstudio", "LM Studio", "openai", "http://localhost:1234/v1", true, ""),
        p("vllm", "vLLM", "openai", "http://localhost:8000/v1", true, ""),
        p("localai", "LocalAI", "openai", "http://localhost:8080/v1", true, ""),
        p("anthropic", "Claude (Anthropic)", "anthropic", "https://api.anthropic.com/v1", false, ""),
        p("openai", "OpenAI / Codex", "openai", "https://api.openai.com/v1", false, ""),
        p("gemini", "Gemini (Google)", "openai", "https://generativelanguage.googleapis.com/v1beta/openai", false, ""),
        p("openrouter", "OpenRouter", "openai", "https://openrouter.ai/api/v1", false, ""),
        p("groq", "Groq", "openai", "https://api.groq.com/openai/v1", false, ""),
        p("mistral", "Mistral", "openai", "https://api.mistral.ai/v1", false, ""),
        p("deepseek", "DeepSeek", "openai", "https://api.deepseek.com/v1", false, ""),
        p("xai", "xAI (Grok)", "openai", "https://api.x.ai/v1", false, ""),
    ]
}

// ---- keys ----------------------------------------------------------------------

fn secrets_file() -> PathBuf {
    crate::settings::config_dir().join("secrets.json")
}

fn secrets() -> serde_json::Map<String, Value> {
    std::fs::read_to_string(secrets_file())
        .ok()
        .and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// Store a provider's key ("" removes it). The file is readable only by you.
pub fn set_key(provider: &str, key: &str) -> Result<(), String> {
    let mut s = secrets();
    if key.is_empty() {
        s.remove(provider);
    } else {
        s.insert(provider.to_string(), json!(key));
    }
    let path = secrets_file();
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    std::fs::write(&path, serde_json::to_vec_pretty(&Value::Object(s)).unwrap()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// The key for a provider: stored, "env:NAME", or the usual variable.
pub fn key(provider: &Provider) -> String {
    if provider.id == "hearth" {
        return hearth_hub().map(|h| h.token).unwrap_or_default();
    }
    let stored = secrets().get(&provider.id).and_then(|v| v.as_str()).unwrap_or("").to_string();
    if let Some(var) = stored.strip_prefix("env:") {
        return std::env::var(var).unwrap_or_default();
    }
    if !stored.is_empty() {
        return stored;
    }
    let var = match provider.id.as_str() {
        "anthropic" => "ANTHROPIC_API_KEY",
        "openai" => "OPENAI_API_KEY",
        "gemini" => "GEMINI_API_KEY",
        "openrouter" => "OPENROUTER_API_KEY",
        "groq" => "GROQ_API_KEY",
        "mistral" => "MISTRAL_API_KEY",
        "deepseek" => "DEEPSEEK_API_KEY",
        "xai" => "XAI_API_KEY",
        _ => return String::new(),
    };
    std::env::var(var).unwrap_or_default()
}

pub fn has_key(provider: &Provider) -> bool {
    !key(provider).is_empty()
}

// ---- HTTP ------------------------------------------------------------------------

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .http_status_as_error(false)
        .build()
        .into()
}

/// The models a provider offers.
pub fn models(provider: &Provider) -> Result<Vec<String>, String> {
    let live;
    let provider = if provider.id == "hearth" {
        live = hearth_provider().ok_or_else(|| HEARTH_OFF.to_string())?;
        &live
    } else {
        provider
    };
    let base = provider.base_url.trim_end_matches('/');
    let k = key(provider);
    let a = agent(Duration::from_secs(8));
    let mut req = a.get(&format!("{base}/models"));
    if provider.kind == "anthropic" {
        req = req.header("x-api-key", &k).header("anthropic-version", "2023-06-01");
    } else if !k.is_empty() {
        req = req.header("Authorization", &format!("Bearer {k}"));
    }
    let mut resp = req.call().map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let body: Value = resp.body_mut().read_json().map_err(|e| e.to_string())?;
    if status >= 400 {
        return Err(error_text(&body, status));
    }
    let mut out: Vec<String> = body["data"]
        .as_array()
        .map(|a| a.iter().filter_map(|m| m["id"].as_str().map(|s| s.trim_start_matches("models/").to_string())).collect())
        .unwrap_or_default();
    out.sort();
    Ok(out)
}

fn error_text(body: &Value, status: u16) -> String {
    let msg = body["error"]["message"].as_str().or(body["error"].as_str()).or(body["message"].as_str()).unwrap_or("");
    if msg.is_empty() { format!("HTTP {status}") } else { format!("{msg} (HTTP {status})") }
}

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout: Duration,
}

/// One chat request. `messages`: [{role: system|user|assistant, content}].
/// `on_delta` gets the text as it streams; returns the whole reply.
pub fn chat(provider: &Provider, model: &str, messages: &[Value], opts: &Options, cancel: &AtomicBool, mut on_delta: impl FnMut(&str)) -> Result<String, String> {
    let base = provider.base_url.trim_end_matches('/');
    let k = key(provider);
    let a = agent(if opts.timeout.is_zero() { Duration::from_secs(300) } else { opts.timeout });
    let max = if opts.max_tokens == 0 { 2048 } else { opts.max_tokens };
    let mut resp = if provider.kind == "anthropic" {
        // System messages go in "system"; the rest alternate.
        let system: Vec<&str> = messages.iter().filter(|m| m["role"] == "system").filter_map(|m| m["content"].as_str()).collect();
        let rest: Vec<&Value> = messages.iter().filter(|m| m["role"] != "system").collect();
        let body = json!({"model": model, "max_tokens": max, "temperature": opts.temperature, "stream": true,
                          "system": system.join("\n\n"), "messages": rest});
        a.post(&format!("{base}/messages"))
            .header("x-api-key", &k)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .send_json(&body)
            .map_err(|e| e.to_string())?
    } else {
        let mut body = json!({"model": model, "messages": messages, "stream": true, "temperature": opts.temperature, "max_tokens": max});
        if !provider.reasoning_effort.is_empty() {
            body["reasoning_effort"] = json!(provider.reasoning_effort);
        }
        let mut req = a.post(&format!("{base}/chat/completions")).header("content-type", "application/json");
        if !k.is_empty() {
            req = req.header("Authorization", &format!("Bearer {k}"));
        }
        if provider.id == "openrouter" {
            req = req.header("X-Title", "Tessera");
        }
        req.send_json(&body).map_err(|e| e.to_string())?
    };
    let status = resp.status().as_u16();
    if status >= 400 {
        let body: Value = resp.body_mut().read_json().unwrap_or(json!({}));
        return Err(error_text(&body, status));
    }
    let reader = BufReader::new(resp.body_mut().as_reader());
    let mut full = String::new();
    for line in reader.lines() {
        if cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        let Ok(line) = line else { break };
        let Some(data) = line.strip_prefix("data:") else { continue };
        let data = data.trim();
        if data == "[DONE]" {
            break;
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else { continue };
        let piece = if provider.kind == "anthropic" {
            if v["type"] == "error" {
                return Err(v["error"]["message"].as_str().unwrap_or("error").to_string());
            }
            v["delta"]["text"].as_str().unwrap_or("").to_string()
        } else {
            if let Some(e) = v.get("error") {
                return Err(e["message"].as_str().unwrap_or("error").to_string());
            }
            v["choices"][0]["delta"]["content"].as_str().unwrap_or("").to_string()
        };
        if !piece.is_empty() {
            full.push_str(&piece);
            on_delta(&piece);
        }
    }
    Ok(strip_thinking(&full))
}

/// Models that think out loud put it in <think>…</think>: not part of the answer.
pub fn strip_thinking(s: &str) -> String {
    let mut out = s.to_string();
    while let (Some(a), Some(b)) = (out.find("<think>"), out.find("</think>")) {
        if b < a {
            break;
        }
        out.replace_range(a..b + "</think>".len(), "");
    }
    out.trim().to_string()
}

// ---- configuration ------------------------------------------------------------------

/// The provider and model for a "provider/model" name.
pub fn resolve(providers: &[Provider], model_ref: &str) -> Option<(Provider, String)> {
    let (pid, model) = model_ref.split_once('/')?;
    if pid == "hearth" {
        return hearth_provider().map(|p| (p, model.to_string()));
    }
    let p = providers.iter().find(|p| p.id == pid).cloned().or_else(|| presets().into_iter().find(|p| p.id == pid))?;
    Some((p, model.to_string()))
}

// ---- Hearth's AI hub -----------------------------------------------------------------
//
// On the Hearth desktop, hearth-ai serves the desktop's models to apps as an
// OpenAI-compatible server on 127.0.0.1 (the same models, cloud rule and
// graphics-memory manager as Hearth's assistant). It says where, with a
// token, in $XDG_RUNTIME_DIR/hearth/ai-api.json while it runs. "default" and
// "fast" are the models picked in Hearth's Settings › AI › Apps.

const HEARTH_OFF: &str = "Hearth's AI hub isn't running here";

#[derive(Clone, Debug, PartialEq)]
pub struct HearthHub {
    pub url: String,
    pub token: String,
}

fn hearth_file() -> Option<PathBuf> {
    if let Some(f) = std::env::var_os("HEARTH_AI_API_FILE") {
        return Some(PathBuf::from(f));
    }
    std::env::var_os("XDG_RUNTIME_DIR").map(|d| PathBuf::from(d).join("hearth").join("ai-api.json"))
}

/// The hub, when it's running (its file is there and its process alive).
/// A small tmpfs file: cheap enough to read per request.
pub fn hearth_hub() -> Option<HearthHub> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(hearth_file()?).ok()?).ok()?;
    let url = v["url"].as_str()?.to_string();
    let token = v["token"].as_str().unwrap_or("").to_string();
    #[cfg(target_os = "linux")]
    if let Some(pid) = v["pid"].as_u64() {
        if !std::path::Path::new(&format!("/proc/{pid}")).exists() {
            return None; // left behind by a hub that died
        }
    }
    Some(HearthHub { url, token })
}

pub fn hearth_provider() -> Option<Provider> {
    let hub = hearth_hub()?;
    Some(Provider { id: "hearth".into(), name: "Hearth AI hub".into(), kind: "openai".into(), base_url: hub.url, reasoning_effort: String::new(), local: true })
}

/// Tasks answered as you type use the hub's "fast" model; the rest "default".
pub fn hearth_alias(task: &str) -> &'static str {
    if matches!(task, "inline" | "classify" | "language" | "request" | "exit" | "check") { "fast" } else { "default" }
}

/// Who answers a task: Hearth's hub when it's on and running (ai.hearth),
/// else the model set for it in Preferences.
pub fn route(ai: &crate::settings::AiSettings, task: &str, model_ref: &str) -> Result<(Provider, String), String> {
    if ai.hearth {
        if let Some(p) = hearth_provider() {
            return Ok((p, hearth_alias(task).to_string()));
        }
    }
    if model_ref.starts_with("hearth/") {
        return resolve(&ai.providers, model_ref).ok_or_else(|| format!("{HEARTH_OFF} (Preferences › AI)."));
    }
    resolve(&ai.providers, model_ref).ok_or_else(|| "No AI model set up (Preferences › AI).".to_string())
}

/// For Preferences: what "default" and "fast" stand for, from the hub's
/// model list ({running, default: {label, available}, fast: {...}}).
pub fn hearth_info() -> Value {
    let Some(p) = hearth_provider() else { return json!({"running": false}) };
    let a = agent(Duration::from_secs(8));
    let resp = a.get(&format!("{}/models", p.base_url.trim_end_matches('/'))).header("Authorization", &format!("Bearer {}", key(&p))).call();
    let mut resp = match resp {
        Ok(r) => r,
        Err(e) => return json!({"running": false, "error": e.to_string()}),
    };
    let status = resp.status().as_u16();
    let body: Value = resp.body_mut().read_json().unwrap_or(json!({}));
    if status >= 400 {
        return json!({"running": true, "error": error_text(&body, status)});
    }
    let mut out = json!({"running": true, "url": p.base_url});
    for m in body["data"].as_array().into_iter().flatten() {
        if let Some(id @ ("default" | "fast")) = m["id"].as_str() {
            out[id] = m["hearth"].clone();
        }
    }
    out
}

/// First run: Ollama on this machine, if it's there, with its smallest model
/// for suggestions and its largest for chat.
pub fn autodetect() -> Option<(Provider, String, String)> {
    let ollama = presets().into_iter().find(|p| p.id == "ollama")?;
    let a = agent(Duration::from_millis(800));
    let mut resp = a.get("http://localhost:11434/api/tags").call().ok()?;
    let v: Value = resp.body_mut().read_json().ok()?;
    let mut models: Vec<(u64, String)> = v["models"]
        .as_array()?
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?.to_string();
            let fam = m["details"]["family"].as_str().unwrap_or("").to_lowercase();
            // Text models only (not OCR, embeddings, translation).
            if fam.contains("ocr") || fam.contains("bert") || name.contains("embed") || name.contains("ocr") || name.contains("translate") {
                return None;
            }
            Some((m["size"].as_u64().unwrap_or(0), name))
        })
        .collect();
    if models.is_empty() {
        return None;
    }
    models.sort();
    Some((ollama, format!("ollama/{}", models[0].1), format!("ollama/{}", models.last().unwrap().1)))
}

// ---- jobs (for the UI) -------------------------------------------------------------

/// The system prompt for each use, with what Tessera knows about where the
/// user is.
pub fn system_prompt(task: &str, ctx: &Value) -> String {
    let os = std::env::consts::OS;
    let shell = ctx["shell"].as_str().unwrap_or("bash");
    let cwd = ctx["cwd"].as_str().unwrap_or("");
    let mut base = format!("You are built into Tessera, a terminal. The user's OS is {os}, shell {shell}, current folder {cwd}.");
    if let Some(a) = ctx["aliases"].as_object().filter(|a| !a.is_empty()) {
        let list: Vec<String> = a.iter().map(|(k, v)| format!("{k}='{}'", v.as_str().unwrap_or(""))).collect();
        base.push_str(&format!(" Aliases in their shell: {} (an alias runs what it stands for: judge options by that program).", list.join(", ")));
    }
    match task {
        "inline" => format!(
            "{base} Complete the command the user is typing. Reply with ONLY the complete command line (including what they typed), \
             no explanation, no quotes, no code fence. If you have no good completion, reply with exactly what they typed."
        ),
        "request" => format!(
            "{base} The user is writing, in plain words, what they want done in their terminal (a request the AI will turn into a command). \
             Finish their sentence with the ending they most likely mean, from their recent commands and folder: add a few words \
             (what to find, where, how). Reply with ONLY the whole request: what they typed, exactly as typed, then your ending. \
             One line, no quotes, no explanation. Only if it's plainly finished already, reply with exactly what they typed."
        ),
        "command" => format!(
            "{base} Turn the user's request into one shell command (or a short pipeline) for their shell. \
             Reply with ONLY the command, no explanation, no code fence."
        ),
        "check" => format!(
            "{base} Check the command the user is about to run. If it has a real problem (a mistake, a flag the program doesn't have, \
             something destructive or risky they may not intend), reply with one short sentence naming it. Otherwise reply with exactly OK. \
             Commands that are fine but unusual, and anything you're not sure about, are OK. You can't see their files: \
             never say a file, folder or path doesn't exist, isn't local or can't be used — any path may be real."
        ),
        "classify" => format!(
            "{base} Say what kind of text the user shows you (a command's output). Reply with exactly one word from: \
             json, csv, tsv, html, markdown, svg, yaml, xml, diff, log, code, text."
        ),
        "language" => format!(
            "{base} The user typed this into their terminal's input line. Say which language it is written in: their shell ({shell}, \
             call it bash) or a programming language. Reply with ONLY a JSON list of up to 3 guesses, most likely first, each \
             [name, probability] with lowercase names (bash, python, javascript, typescript, ruby, perl, lua, php, r, julia, go, rust, c, cpp, \
             java, kotlin, haskell, sql, ...). A command that runs a program is bash."
        ),
        "exit" => format!(
            "{base} A command the user ran exited with a status that isn't 0. Say what that exit status means for that particular \
             program (its documented meaning, e.g. from its manual), judged from the command, the status and its output: a short \
             phrase, under 15 words, about this program and this status only. Reply with ONLY that phrase. If you don't know, \
             reply with exactly unknown."
        ),
        "fix" => format!(
            "{base} The user's last command failed. From the command, its exit status and output, reply with ONLY a corrected command, \
             no explanation, no code fence."
        ),
        _ => format!(
            "{base} You are their assistant in the terminal: answer questions, explain output, and suggest commands. \
             Put each command you suggest in its own ```bash code block: the user can run it with one click (they approve each one), \
             and its output comes back to you. Keep answers short and practical."
        ),
    }
}

/// A command from a model's reply: the first code block's contents, or the
/// first non-empty line, without a "$ " prompt.
pub fn command_from(reply: &str) -> String {
    let r = strip_thinking(reply);
    let body = if let Some(start) = r.find("```") {
        let after = &r[start + 3..];
        let after = after.split_once('\n').map(|(_, b)| b).unwrap_or(after);
        after.split("```").next().unwrap_or(after)
    } else {
        &r
    };
    let line = body.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("");
    line.trim_start_matches("$ ").trim_matches('`').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replies() {
        assert_eq!(strip_thinking("<think>hmm</think> git commit"), "git commit");
        assert_eq!(command_from("```bash\n$ ls -la\n```\nThat lists."), "ls -la");
        assert_eq!(command_from("git status"), "git status");
    }

    #[test]
    fn model_refs() {
        let (p, m) = resolve(&[], "ollama/qwen3.5:4b").unwrap();
        assert_eq!((p.id.as_str(), m.as_str()), ("ollama", "qwen3.5:4b"));
        let (p, m) = resolve(&[], "openrouter/anthropic/claude-sonnet").unwrap();
        assert_eq!((p.kind.as_str(), m.as_str()), ("openai", "anthropic/claude-sonnet"));
    }
}

#[cfg(test)]
mod live {
    #[test]
    #[ignore]
    fn ollama_stream() {
        let (p, m) = super::resolve(&[], "ollama/qwen3.5:4b").unwrap();
        let cancel = std::sync::atomic::AtomicBool::new(false);
        let ctx = serde_json::json!({"cwd": "/home/ben/src/tessera", "shell": "zsh"});
        for (task, text) in [("inline", "git comm"), ("command", "find rust files changed in the last day"), ("check", "rm -rf $DIR/")] {
            let t = std::time::Instant::now();
            let msgs = vec![serde_json::json!({"role": "system", "content": super::system_prompt(task, &ctx)}), serde_json::json!({"role": "user", "content": text})];
            let mut n = 0;
            let r = super::chat(&p, &m, &msgs, &super::Options { max_tokens: 80, temperature: 0.2, ..Default::default() }, &cancel, |_| n += 1);
            println!("{task}: {r:?} ({n} pieces, {:?})", t.elapsed());
        }
        println!("models: {:?}", super::models(&p));
        println!("autodetect: {:?}", super::autodetect().map(|a| (a.1, a.2)));
    }
}
