//! Language servers for the input editor: the command being typed is a
//! document the server checks and explains (bash-language-server with
//! shellcheck for shells, pyright or pylsp in Python, typescript-language-
//! server in Node). Servers are started on demand when they're installed,
//! one per language and folder, and spoken to in JSON-RPC over stdio.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

/// The servers tried for a language, in order.
fn candidates(lang: &str) -> &'static [&'static [&'static str]] {
    match lang {
        "bash" => &[&["bash-language-server", "start"]],
        "python" => &[&["basedpyright-langserver", "--stdio"], &["pyright-langserver", "--stdio"], &["pylsp"]],
        "javascript" => &[&["typescript-language-server", "--stdio"]],
        _ => &[],
    }
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
}

/// The command of the first installed server for `lang`.
pub fn server_for(lang: &str) -> Option<Vec<String>> {
    candidates(lang).iter().find(|c| on_path(c[0])).map(|c| c.iter().map(|s| s.to_string()).collect())
}

fn language_id(lang: &str) -> &'static str {
    match lang {
        "python" => "python",
        "javascript" => "javascript",
        _ => "shellscript",
    }
}

fn extension(lang: &str) -> &'static str {
    match lang {
        "python" => "py",
        "javascript" => "js",
        _ => "sh",
    }
}

struct Server {
    stdin: Mutex<ChildStdin>,
    _child: Mutex<Child>,
    next_id: Mutex<i64>,
    pending: Mutex<HashMap<i64, Sender<Value>>>,
    /// The latest diagnostics per document, and who wants to hear of them.
    diagnostics: Mutex<HashMap<String, Value>>,
    listeners: Mutex<Vec<Sender<(String, Value)>>>,
    versions: Mutex<HashMap<String, i64>>,
    dead: std::sync::atomic::AtomicBool,
}

impl Server {
    fn send(&self, msg: &Value) -> bool {
        let body = msg.to_string();
        let mut w = self.stdin.lock().unwrap();
        w.write_all(format!("Content-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).and_then(|_| w.flush()).is_ok()
    }

    fn notify(&self, method: &str, params: Value) {
        self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    fn request(&self, method: &str, params: Value, timeout: Duration) -> Option<Value> {
        let id = {
            let mut n = self.next_id.lock().unwrap();
            *n += 1;
            *n
        };
        let (tx, rx) = channel();
        self.pending.lock().unwrap().insert(id, tx);
        if !self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})) {
            return None;
        }
        let r = rx.recv_timeout(timeout).ok();
        self.pending.lock().unwrap().remove(&id);
        r
    }
}

fn read_messages(out: impl Read, server: Arc<Server>) {
    let mut r = BufReader::new(out);
    loop {
        let mut len = 0usize;
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).unwrap_or(0) == 0 {
                server.dead.store(true, std::sync::atomic::Ordering::SeqCst);
                return;
            }
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            if let Some(v) = line.strip_prefix("Content-Length:") {
                len = v.trim().parse().unwrap_or(0);
            }
        }
        let mut buf = vec![0u8; len];
        if r.read_exact(&mut buf).is_err() {
            server.dead.store(true, std::sync::atomic::Ordering::SeqCst);
            return;
        }
        let Ok(msg) = serde_json::from_slice::<Value>(&buf) else { continue };
        if let (Some(id), true) = (msg.get("id").and_then(Value::as_i64), msg.get("method").is_none()) {
            if let Some(tx) = server.pending.lock().unwrap().remove(&id) {
                let _ = tx.send(msg.get("result").cloned().unwrap_or(Value::Null));
            }
            continue;
        }
        match msg["method"].as_str() {
            Some("textDocument/publishDiagnostics") => {
                let uri = msg["params"]["uri"].as_str().unwrap_or("").to_string();
                let d = msg["params"]["diagnostics"].clone();
                server.diagnostics.lock().unwrap().insert(uri.clone(), d.clone());
                server.listeners.lock().unwrap().retain(|l| l.send((uri.clone(), d.clone())).is_ok());
            }
            // Requests from the server (configuration, capability registration,
            // progress): answer them plainly so it carries on.
            Some(m) if msg.get("id").is_some() => {
                let result = if m == "workspace/configuration" {
                    json!(msg["params"]["items"].as_array().map(|a| a.iter().map(|_| Value::Null).collect::<Vec<_>>()).unwrap_or_default())
                } else {
                    Value::Null
                };
                server.send(&json!({"jsonrpc": "2.0", "id": msg["id"], "result": result}));
            }
            _ => {}
        }
    }
}

type Key = (String, String); // (language, folder)

fn servers() -> &'static Mutex<HashMap<Key, Arc<Server>>> {
    static S: OnceLock<Mutex<HashMap<Key, Arc<Server>>>> = OnceLock::new();
    S.get_or_init(Default::default)
}

fn start(lang: &str, root: &str) -> Option<Arc<Server>> {
    let key = (lang.to_string(), root.to_string());
    if let Some(s) = servers().lock().unwrap().get(&key) {
        if !s.dead.load(std::sync::atomic::Ordering::SeqCst) {
            return Some(s.clone());
        }
    }
    let cmd = server_for(lang)?;
    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        .current_dir(if std::path::Path::new(root).is_dir() { root } else { "/" })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;
    let server = Arc::new(Server {
        stdin: Mutex::new(stdin),
        _child: Mutex::new(child),
        next_id: Mutex::new(0),
        pending: Mutex::new(HashMap::new()),
        diagnostics: Mutex::new(HashMap::new()),
        listeners: Mutex::new(Vec::new()),
        versions: Mutex::new(HashMap::new()),
        dead: Default::default(),
    });
    {
        let s = server.clone();
        std::thread::spawn(move || read_messages(stdout, s));
    }
    let root_uri = format!("file://{root}");
    server.request(
        "initialize",
        json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "workspaceFolders": [{"uri": root_uri, "name": "shell"}],
            "capabilities": {
                "textDocument": {
                    "hover": {"contentFormat": ["markdown", "plaintext"]},
                    "completion": {"completionItem": {"documentationFormat": ["markdown", "plaintext"], "snippetSupport": false}},
                    "publishDiagnostics": {}
                },
                "workspace": {"configuration": true}
            }
        }),
        Duration::from_secs(20),
    )?;
    server.notify("initialized", json!({}));
    servers().lock().unwrap().insert(key, server.clone());
    Some(server)
}

/// The editor's text as a document of the server's language: a file in the
/// shell's folder that's never written (its name only gives the server
/// context for relative paths).
fn doc_uri(lang: &str, root: &str, editor: &str) -> String {
    format!("file://{}/.tessera-input-{editor}.{}", root.trim_end_matches('/'), extension(lang))
}

/// Send the editor's text (opening the document the first time).
fn sync(server: &Server, lang: &str, uri: &str, text: &str) {
    let mut versions = server.versions.lock().unwrap();
    match versions.get_mut(uri) {
        None => {
            versions.insert(uri.to_string(), 1);
            drop(versions);
            server.notify("textDocument/didOpen", json!({"textDocument": {"uri": uri, "languageId": language_id(lang), "version": 1, "text": text}}));
        }
        Some(v) => {
            *v += 1;
            let v = *v;
            drop(versions);
            server.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": v}, "contentChanges": [{"text": text}]}));
        }
    }
}

/// UTF-16 offset -> LSP position (line, character in UTF-16).
fn position(text: &str, offset: usize) -> (u32, u32) {
    let (mut line, mut col, mut n) = (0u32, 0u32, 0usize);
    for c in text.chars() {
        if n >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += c.len_utf16() as u32;
        }
        n += c.len_utf16();
    }
    (line, col)
}

fn offset(text: &str, line: u64, character: u64) -> usize {
    let mut n = 0usize;
    for (i, l) in text.split('\n').enumerate() {
        if i as u64 == line {
            return n + (character as usize).min(l.encode_utf16().count());
        }
        n += l.encode_utf16().count() + 1;
    }
    n
}

/// What the editor asks for. Each runs on a thread; the answer is JSON.
pub enum Ask {
    /// Diagnostics as completion-style errors: [[start, length, message, severity]].
    Diagnostics,
    /// Markdown for what's at the cursor ("" when nothing).
    Hover(usize),
    /// Completion items at the cursor: [{label, insert, detail, kind, doc}].
    Complete(usize),
}

pub fn ask(lang: &str, root: &str, editor: &str, text: &str, what: Ask) -> Option<String> {
    let server = start(lang, root)?;
    let uri = doc_uri(lang, root, editor);
    match what {
        Ask::Diagnostics => {
            let (tx, rx): (Sender<(String, Value)>, Receiver<(String, Value)>) = channel();
            server.listeners.lock().unwrap().push(tx);
            sync(&server, lang, &uri, text);
            // The server publishes when it has checked this version.
            let deadline = std::time::Instant::now() + Duration::from_secs(4);
            let mut diags = Value::Null;
            while let Some(left) = deadline.checked_duration_since(std::time::Instant::now()) {
                match rx.recv_timeout(left) {
                    Ok((u, d)) if u == uri => {
                        diags = d;
                        break;
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
            if diags.is_null() {
                diags = server.diagnostics.lock().unwrap().get(&uri).cloned().unwrap_or(json!([]));
            }
            let out: Vec<Value> = diags
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|d| {
                            let s = offset(text, d["range"]["start"]["line"].as_u64().unwrap_or(0), d["range"]["start"]["character"].as_u64().unwrap_or(0));
                            let e = offset(text, d["range"]["end"]["line"].as_u64().unwrap_or(0), d["range"]["end"]["character"].as_u64().unwrap_or(0));
                            let code = d["code"].as_str().map(str::to_string).or_else(|| d["code"].as_i64().map(|c| c.to_string())).unwrap_or_default();
                            let src = d["source"].as_str().unwrap_or("");
                            let mut msg = d["message"].as_str().unwrap_or("").to_string();
                            if !code.is_empty() {
                                msg = format!("{msg} ({src} {code})").replace("( ", "(");
                            }
                            json!([s, e.saturating_sub(s).max(1), msg, d["severity"].as_i64().unwrap_or(1)])
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Value::Array(out).to_string())
        }
        Ask::Hover(at) => {
            sync(&server, lang, &uri, text);
            let (line, character) = position(text, at);
            let r = server.request("textDocument/hover", json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}), Duration::from_secs(5))?;
            Some(markdown(&r["contents"]))
        }
        Ask::Complete(at) => {
            sync(&server, lang, &uri, text);
            let (line, character) = position(text, at);
            let r = server.request("textDocument/completion", json!({"textDocument": {"uri": uri}, "position": {"line": line, "character": character}}), Duration::from_secs(5))?;
            let items = r.get("items").cloned().unwrap_or(r);
            let out: Vec<Value> = items
                .as_array()
                .map(|a| {
                    a.iter()
                        .take(200)
                        .map(|i| {
                            let label = i["label"].as_str().unwrap_or("").to_string();
                            let insert = i["insertText"].as_str().or(i["textEdit"]["newText"].as_str()).unwrap_or(&label).to_string();
                            json!({"label": label, "insert": insert, "detail": i["detail"].as_str().unwrap_or(""),
                                   "kind": "lsp", "doc": markdown(&i["documentation"])})
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Value::Array(out).to_string())
        }
    }
}

/// Hover/documentation contents (string, MarkupContent, MarkedString or a
/// list of them) as Markdown.
fn markdown(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(markdown).filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n"),
        Value::Object(o) => {
            if let Some(Value::String(s)) = o.get("value") {
                match o.get("language").and_then(Value::as_str) {
                    Some(l) => format!("```{l}\n{s}\n```"),
                    None => s.clone(),
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions() {
        assert_eq!(position("ab\ncd", 4), (1, 1));
        assert_eq!(offset("ab\ncd", 1, 1), 4);
        assert_eq!(markdown(&json!({"kind": "markdown", "value": "**x**"})), "**x**");
    }

    #[test]
    #[ignore]
    fn bash_server() {
        let root = std::env::temp_dir().display().to_string();
        let text = "for f in $(ls *.txt); do echo $f; done";
        let d = ask("bash", &root, "t", text, Ask::Diagnostics);
        println!("diagnostics: {d:?}");
        let h = ask("bash", &root, "t", "grep -r foo", Ask::Hover(1));
        println!("hover: {:?}", h.map(|s| s.chars().take(200).collect::<String>()));
        let c = ask("bash", &root, "t", "ech", Ask::Complete(3));
        println!("complete: {:?}", c.map(|s| s.chars().take(300).collect::<String>()));
    }
}
