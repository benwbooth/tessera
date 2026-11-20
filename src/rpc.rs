//! tesserad's requests besides the shells: notebook kernels, Nix runs and
//! command lookups, the shell hand-back, each session's cells and notebook.
//! The window sends `T_RPC` {method, params} and gets `T_RPC_REPLY` with
//! the result (or `T_ERROR`); kernels' events go to the window that claimed
//! the kernel over its `T_SUBSCRIBE` connection, and wait in a buffer while
//! no window has it (the window closed), to be handed over on attach.

use crate::net::UnixStream;
use crate::proto::*;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

/// Events kept for a kernel no window has (each is a status, a stream
/// chunk, an output...): the oldest go past this.
const MAX_BUFFERED: usize = 20_000;

#[derive(Default)]
struct KeyState {
    /// The subscriber (window) that gets this kernel's events.
    sub: Option<u64>,
    buf: VecDeque<Value>,
}

#[derive(Default)]
struct Hub {
    keys: HashMap<String, KeyState>,
    subs: HashMap<u64, UnixStream>,
}

fn hub() -> &'static Mutex<Hub> {
    static HUB: OnceLock<Mutex<Hub>> = OnceLock::new();
    HUB.get_or_init(Default::default)
}

static NEXT_SUB: AtomicU64 = AtomicU64::new(1);

/// The sink a kernel for `key` reports to: its window's connection, else
/// the buffer.
fn sink(key: &str) -> crate::jupyter::Sink {
    let key = key.to_string();
    Arc::new(move |v: Value| {
        let mut h = hub().lock().unwrap();
        let h = &mut *h;
        let st = h.keys.entry(key.clone()).or_default();
        if let Some(id) = st.sub {
            if let Some(s) = h.subs.get_mut(&id) {
                if write_json(s, T_EVENT, &json!({"key": key, "event": v})).is_ok() {
                    return;
                }
                h.subs.remove(&id);
            }
            st.sub = None;
        }
        st.buf.push_back(v);
        while st.buf.len() > MAX_BUFFERED {
            st.buf.pop_front();
        }
    })
}

/// `key` (and its language kernels, "key~lang") now report to `sub`; what
/// they said meanwhile comes back.
fn claim(key: &str, sub: u64) -> Vec<Value> {
    let mut h = hub().lock().unwrap();
    let st = h.keys.entry(key.to_string()).or_default();
    st.sub = (sub > 0).then_some(sub);
    st.buf.drain(..).collect()
}

/// Which session each notebook's kernels belong to (the pane it's kept
/// with): they end with it.
fn owners() -> &'static Mutex<HashMap<String, String>> {
    static O: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    O.get_or_init(Default::default)
}

fn own(key: &str, session: &str) {
    if session.is_empty() {
        return;
    }
    let base = key.split('~').next().unwrap_or(key).to_string();
    owners().lock().unwrap().insert(base, session.to_string());
}

/// A session's shell ended: its notebook's kernels go too.
pub fn session_ended(session: &str) {
    let keys: Vec<String> = owners().lock().unwrap().iter().filter(|(_, s)| *s == session).map(|(k, _)| k.clone()).collect();
    for k in keys {
        owners().lock().unwrap().remove(&k);
        stop_tree(&k);
    }
}

fn forget_key(key: &str) {
    hub().lock().unwrap().keys.remove(key);
}

/// A window's event connection: it stays open, events are written to it;
/// when it closes its kernels go back to buffering.
pub fn subscribe(mut stream: UnixStream) -> std::io::Result<()> {
    let id = NEXT_SUB.fetch_add(1, Ordering::SeqCst);
    write_json(&mut stream, T_SUBSCRIBED, &id)?;
    hub().lock().unwrap().subs.insert(id, stream.try_clone()?);
    // (Nothing comes from the window on it: a read ends when it closes.)
    let _ = read_frame(&mut stream);
    let mut h = hub().lock().unwrap();
    h.subs.remove(&id);
    for st in h.keys.values_mut() {
        if st.sub == Some(id) {
            st.sub = None;
        }
    }
    Ok(())
}

/// Whether any kernel runs (tesserad stays up for them).
pub fn kernels_running() -> bool {
    running().as_array().is_some_and(|a| !a.is_empty())
}

fn kernel_running(key: &str) -> bool {
    crate::shellk::has(key) || crate::jupyter::has(key)
}

/// Every running kernel (Jupyter's and the shell kernels), as the
/// notebook's kernels list wants them.
fn running() -> Value {
    let mut all: Vec<Value> = serde_json::from_str(&crate::jupyter::running()).unwrap_or_default();
    all.extend(crate::shellk::running());
    Value::Array(all)
}

/// Stop every kernel (local mode's window is closing: they'd outlive it).
pub fn stop_all_kernels() {
    let keys: Vec<String> = running().as_array().into_iter().flatten().filter_map(|k| k["key"].as_str().map(String::from)).collect();
    for k in keys {
        crate::shellk::stop(&k);
        crate::jupyter::stop(&k);
        forget_key(&k);
    }
}

/// Stop every kernel of a notebook (its key, and "key~lang" ones).
fn stop_tree(key: &str) {
    let prefix = format!("{key}~");
    let keys: Vec<String> = running().as_array().into_iter().flatten().filter_map(|k| k["key"].as_str().map(String::from)).filter(|k| k == key || k.starts_with(&prefix)).collect();
    for k in keys.iter().chain(std::iter::once(&key.to_string())) {
        crate::shellk::stop(k);
        crate::jupyter::stop(k);
        forget_key(k);
    }
}

/// The window's Starship config, when a request carries it.
fn set_window_config(p: &Value) {
    if let Some(c) = p["config"].as_str() {
        crate::prompt::set_window_config(Some(c.to_string()));
    }
}

pub fn handle(mut stream: UnixStream, payload: &[u8]) -> std::io::Result<()> {
    let req: Value = serde_json::from_slice(payload).unwrap_or_default();
    let method = req["method"].as_str().unwrap_or("");
    match call(method, &req["params"]) {
        Ok(v) => write_json(&mut stream, T_RPC_REPLY, &v),
        Err(e) => write_json(&mut stream, T_ERROR, &e),
    }
}

/// What `nix` is for when it's missing on this host: the message the window
/// shows, with the installer to run.
pub const NO_NIX: &str = "This needs Nix, which isn't installed on this host. Install it with the Determinate Systems installer:\n\ncurl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install";

pub fn has_nix() -> bool {
    crate::jupyter::nix_program().is_some()
}

fn call(method: &str, p: &Value) -> Result<Value, String> {
    let s = |k: &str| p[k].as_str().unwrap_or("").to_string();
    let n = |k: &str| p[k].as_u64().unwrap_or(0);
    let pid = || n("pid") as u32;
    Ok(match method {
        "ping" => json!("pong"),
        "host.info" => json!({
            "version": env!("CARGO_PKG_VERSION"),
            "state_dir": crate::settings::state_dir().display().to_string(),
            "home": std::env::var("HOME").unwrap_or_default(),
            "nix": has_nix(),
            "no_nix": NO_NIX,
        }),
        // ---- kernels ----
        "kernel.specs" => {
            let mut all: Vec<Value> = serde_json::from_str(&crate::jupyter::specs_json()).unwrap_or_default();
            all.extend(crate::shellk::specs());
            Value::Array(all)
        }
        "kernel.running" => running(),
        // Start `spec` for `key` unless it already runs (a window coming back
        // to it): either way it reports to `sub` from now on.
        "kernel.start" => {
            let key = s("key");
            own(&key, &s("session"));
            let events = claim(&key, n("sub"));
            if kernel_running(&key) && !p["restart"].as_bool().unwrap_or(false) {
                return Ok(json!({"running": true, "events": events}));
            }
            let spec = s("spec");
            let shell = crate::shellk::is_spec(&spec);
            if !has_nix() && if shell { crate::shellk::needs_nix(&spec) } else { crate::jupyter::needs_nix(&spec) } {
                sink(&key)(json!({"type": "status", "state": "dead", "message": NO_NIX, "no_nix": true}));
                return Ok(json!({"running": false, "events": events}));
            }
            if shell {
                crate::shellk::start(&key, &spec, &s("cwd"), pid(), sink(&key));
            } else {
                crate::jupyter::start(&key, &spec, &s("cwd"), pid(), sink(&key));
            }
            json!({"running": false, "events": events})
        }
        // A window takes a notebook's kernels back: what each said meanwhile,
        // and which are still running (with their state).
        "kernel.attach" => {
            let key = s("key");
            own(&key, &s("session"));
            let sub = n("sub");
            let prefix = format!("{key}~");
            let all = running();
            let mine: Vec<Value> = all.as_array().into_iter().flatten().filter(|k| k["key"].as_str().is_some_and(|x| x == key || x.starts_with(&prefix))).cloned().collect();
            let mut events = Vec::new();
            let mut keys: Vec<String> = mine.iter().filter_map(|k| k["key"].as_str().map(String::from)).collect();
            // (Keys that only have events left: a kernel that ended.)
            keys.extend(hub().lock().unwrap().keys.keys().filter(|k| **k == key || k.starts_with(&prefix)).cloned());
            keys.sort();
            keys.dedup();
            for k in keys {
                for e in claim(&k, sub) {
                    events.push(json!({"key": k, "event": e}));
                }
            }
            json!({"kernels": mine, "events": events})
        }
        "kernel.input" => json!(crate::jupyter::input(&s("key"), &s("value"))),
        "kernel.sync" => {
            let key = s("key");
            if crate::shellk::has(&key) {
                json!(crate::shellk::sync(&key, pid(), &s("cwd")))
            } else {
                json!(crate::jupyter::sync(&key, pid(), &s("cwd")))
            }
        }
        "kernel.export" => {
            let key = s("key");
            if !crate::shellk::has(&key) {
                crate::jupyter::export_to_shell(&key, pid());
            }
            Value::Null
        }
        "kernel.execute" => {
            let key = s("key");
            if crate::shellk::has(&key) {
                json!(crate::shellk::execute(&key, &s("cell"), &s("code")))
            } else {
                json!(crate::jupyter::execute(&key, &s("cell"), &s("code")))
            }
        }
        "kernel.complete" => json!(crate::jupyter::complete(&s("key"), &s("request"), &s("code"), n("pos") as usize)),
        "kernel.interrupt" => {
            crate::shellk::interrupt(&s("key"));
            crate::jupyter::interrupt(&s("key"));
            Value::Null
        }
        "kernel.stop" => {
            let key = s("key");
            crate::shellk::stop(&key);
            crate::jupyter::stop(&key);
            forget_key(&key);
            Value::Null
        }
        // A notebook closed for good: it and its language kernels.
        "kernel.stop_tree" => {
            stop_tree(&s("key"));
            Value::Null
        }
        // ---- background jobs (src/jobs.rs) ----
        "job.start" => crate::jobs::start(p),
        "job.list" => crate::jobs::list(&s("owner")),
        "job.output" => crate::jobs::output(n("id"), n("from"), p["exec"].as_bool().unwrap_or(false)),
        "job.stop" => json!(crate::jobs::stop(n("id"))),
        "job.forget" => {
            crate::jobs::forget(n("id"));
            Value::Null
        }
        // ---- the pane's shell ----
        "shell.chain" => json!(crate::procinfo::shell_chain(pid())),
        "shell.launch_line" => json!(crate::shell_integration::launch_line(&s("shell"), n("parent") as u32)),
        "shell.hand_back" => {
            crate::shell_integration::hand_back(n("child") as u32, n("parent") as u32);
            Value::Null
        }
        "shell.env" => json!(crate::prompt::shell_env(pid()).unwrap_or_default()),
        // ---- the pane's editor and prompt bar (they look at the host) ----
        "complete" => {
            crate::complete::set_shell_env(crate::prompt::shell_env(pid()));
            let cwd = s("cwd");
            let cwd = if cwd.is_empty() { std::env::var("HOME").unwrap_or_default() } else { cwd };
            let out = crate::complete::complete(&s("text"), n("cursor") as usize, &cwd, &p["aliases"]);
            serde_json::from_str(&out).unwrap_or(Value::String(out))
        }
        "complete.doc" => json!(crate::complete::item_doc(&s("kind"), &s("label"), &s("line"))),
        "complete.exit_meaning" => json!(crate::complete::exit_meaning(&s("command"), p["status"].as_i64().unwrap_or(0) as i32)),
        "complete.download" => {
            let fish = crate::complete::download_fish_completions();
            let tldr = crate::complete::update_tldr();
            json!({"fish": fish.as_ref().ok(), "tldr": tldr.as_ref().ok(), "error": fish.err().or(tldr.err()).map(|e| e.to_string())})
        }
        "complete.counts" => json!({"fish": crate::complete::fish_completion_count(), "tldr": crate::complete::tldr_count()}),
        "prompt.has_starship" => json!(crate::prompt::has_starship()),
        "prompt.info" => serde_json::from_str(&crate::prompt::info(&s("cwd"), pid())).unwrap_or(Value::Null),
        "prompt.starship" => {
            set_window_config(p);
            serde_json::from_str(&crate::prompt::starship(&s("cwd"), p["status"].as_i64().unwrap_or(0) as i32, n("duration"), n("width").max(1) as u32, pid())).unwrap_or(Value::Null)
        }
        "prompt.choices" => serde_json::from_str(&crate::prompt_widgets::choices(&s("widget"), &s("cwd"), pid())).unwrap_or(Value::Null),
        "prompt.modules" => {
            set_window_config(p);
            let names: Vec<String> = serde_json::from_value(p["names"].clone()).unwrap_or_default();
            crate::prompt_widgets::modules(&names, &s("cwd"), p["status"].as_i64().unwrap_or(0) as i32, n("duration"), pid())
        }
        // ---- language servers (the pane's editor), run here ----
        "lsp.available" => json!(crate::lsp::server_for(&s("lang")).map(|c| c[0].clone()).unwrap_or_default()),
        "lsp.ask" => {
            let at = n("cursor") as usize;
            let ask = match s("what").as_str() {
                "hover" => crate::lsp::Ask::Hover(at),
                "complete" => crate::lsp::Ask::Complete(at),
                _ => crate::lsp::Ask::Diagnostics,
            };
            // (Each editor its own document: the key names it.)
            let doc: String = s("key").chars().filter(|c| c.is_ascii_alphanumeric()).collect();
            json!(crate::lsp::ask(&s("lang"), &s("root"), &doc, &s("text"), ask).unwrap_or_default())
        }
        // ---- files on this host (a remote pane's browser, viewer, tables) ----
        "files" => {
            let path = s("path");
            let out = match s("op").as_str() {
                "list" => crate::files::list(&path, p["all"].as_bool().unwrap_or(false)),
                "action" => crate::files::action(&s("action"), &path, &s("arg")),
                "read_for_edit" => crate::files::read_for_edit(&path),
                "save_edited" => crate::files::save_edited(&path, &s("text"), p["crlf"].as_bool().unwrap_or(false)),
                "read_text" => crate::files::read_text(&path),
                // (For another host's pane: whether a word in its output names
                // a file there; a free name for something copied in.)
                "exists" => json!(std::path::Path::new(&path).exists()).to_string(),
                "unused_name" => crate::files::unused_name(std::path::Path::new(&path), &s("arg")).display().to_string(),
                "write_text" => crate::files::write_text(&path, &s("text")),
                "line_index" => crate::files::line_index(&path),
                "read_lines" => crate::files::read_lines(&path, n("start") as usize, n("count") as usize),
                "table_open" => crate::tables::open(&path),
                "table_query" => crate::tables::query(&path, &s("spec")),
                "table_edit_cell" => crate::tables::edit_cell(&path, p["row"].as_i64().unwrap_or(0), &s("column"), &s("value")),
                "table_add_row" => crate::tables::add_row(&path),
                "table_delete_rows" => crate::tables::delete_rows(&path, &s("rows")),
                "table_exec" => crate::tables::exec(&path, &s("sql")),
                "table_save" => crate::tables::save(&path, &s("dest")),
                "table_close" => {
                    crate::tables::close(&path);
                    String::new()
                }
                o => return Err(format!("tesserad: no file request {o:?}")),
            };
            Value::String(out)
        }
        // ---- running code and Nix commands ----
        "snippet.command" => json!(crate::snippets::command(&s("lang"), &s("code"), &s("cwd"), &s("shell"))),
        "nix.lookup" => {
            if !has_nix() {
                return Ok(json!([]));
            }
            let found = crate::nixcmds::lookup(&s("prefix"), n("limit").max(1) as usize);
            json!(found.into_iter().map(|(c, a)| json!([c, a])).collect::<Vec<_>>())
        }
        "nix.run_line" => json!(crate::nixcmds::run_line(&s("cmd"), &s("attr"))),
        // ---- each session's cells and notebook ----
        "cells.load" => {
            // (Taking over a gone session's first: see daemon::adopt_cells.)
            if !s("from").is_empty() && s("from") != s("session") {
                crate::daemon::adopt_cells(&s("session"), &s("from"));
            }
            json!(crate::cells::load_text(&s("session")))
        }
        "cells.forget" => {
            crate::cells::Cells::forget(&s("session"));
            stop_tree(&format!("nb-{}", s("session")));
            Value::Null
        }
        "notebook.read" => match std::fs::read_to_string(crate::cells::notebook_path(&s("session"))) {
            Ok(t) => json!({"text": t}),
            Err(e) => json!({"error": e.to_string()}),
        },
        "notebook.write" => {
            let path = crate::cells::notebook_path(&s("session"));
            if let Some(d) = path.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let tmp = path.with_extension("ipynb.tmp");
            let r = std::fs::write(&tmp, s("text")).and_then(|_| std::fs::rename(&tmp, &path));
            json!(r.err().map(|e| e.to_string()).unwrap_or_default())
        }
        _ => return Err(format!("tesserad: no request {method:?}")),
    })
}
