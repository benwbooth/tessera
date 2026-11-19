//! tessh as a Jupyter kernel: the messaging protocol (ZeroMQ sockets on
//! tokio), with tessh running in the embedded Python on the main thread (so
//! an interrupt is a KeyboardInterrupt). Standard output and error, Python's
//! and programs', are captured at the file descriptors and sent on as they
//! come; values are shown with their rich representations (HTML, images,
//! …), matplotlib's figures as PNGs.

use crate::runner::{self, TesshError};
use crate::runtime;
use base64::Engine;
use bytes::Bytes;
use hmac::{Hmac, Mac};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyString};
use serde_json::{json, Map, Value};
use sha2::Sha256;
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use tokio::sync::mpsc as tmpsc;
use zeromq::{Socket, SocketRecv, SocketSend, ZmqMessage};

const DELIM: &[u8] = b"<IDS|MSG>";

#[derive(Clone)]
struct Signer {
    key: Vec<u8>,
}

impl Signer {
    fn sign(&self, parts: &[&[u8]]) -> String {
        if self.key.is_empty() {
            return String::new();
        }
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key).unwrap();
        for p in parts {
            mac.update(p);
        }
        hex::encode(mac.finalize().into_bytes())
    }
}

#[derive(Clone, Debug)]
struct Msg {
    ids: Vec<Bytes>,
    header: Value,
    parent: Value,
    content: Value,
}

fn parse(frames: Vec<Bytes>) -> Option<Msg> {
    let d = frames.iter().position(|f| f.as_ref() == DELIM)?;
    let ids = frames[..d].to_vec();
    let get = |i: usize| -> Value { frames.get(d + i).and_then(|f| serde_json::from_slice(f).ok()).unwrap_or(json!({})) };
    Some(Msg { ids, header: get(2), parent: get(3), content: get(5) })
}

static SESSION: Mutex<String> = Mutex::new(String::new());

fn now() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = t.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:06}Z", rem / 3600, rem % 3600 / 60, rem % 60, t.subsec_micros())
}

fn header(msg_type: &str) -> Value {
    json!({"msg_id": uuid::Uuid::new_v4().to_string(), "username": "tessh", "session": SESSION.lock().unwrap().clone(),
           "date": now(), "msg_type": msg_type, "version": "5.3"})
}

fn build(signer: &Signer, ids: &[Bytes], msg_type: &str, parent: &Value, content: Value) -> ZmqMessage {
    let h = serde_json::to_vec(&header(msg_type)).unwrap();
    let p = serde_json::to_vec(parent).unwrap();
    let m = b"{}".to_vec();
    let c = serde_json::to_vec(&content).unwrap();
    let sig = signer.sign(&[&h, &p, &m, &c]);
    let mut frames: Vec<Bytes> = ids.to_vec();
    frames.push(Bytes::from_static(DELIM));
    frames.push(Bytes::from(sig));
    frames.extend([Bytes::from(h), Bytes::from(p), Bytes::from(m), Bytes::from(c)]);
    ZmqMessage::try_from(frames).unwrap()
}

/// What goes out on iopub: (msg_type, content, parent header).
type IoMsg = (String, Value, Value);

static IOPUB: Mutex<Option<tmpsc::UnboundedSender<IoMsg>>> = Mutex::new(None);
static PARENT: Mutex<Value> = Mutex::new(Value::Null);

fn publish(msg_type: &str, content: Value) {
    let parent = PARENT.lock().unwrap().clone();
    if let Some(tx) = IOPUB.lock().unwrap().as_ref() {
        let _ = tx.send((msg_type.to_string(), content, parent));
    }
}

// ---- output at the file descriptors -------------------------------------------------

static PENDING: AtomicU64 = AtomicU64::new(0);

/// fds 1 and 2 into pipes, read and sent on as stream messages.
fn capture_fds() {
    for (fd, name) in [(1, "stdout"), (2, "stderr")] {
        let Ok((mut r, w)) = std::io::pipe() else { continue };
        unsafe { libc::dup2(std::os::fd::AsRawFd::as_raw_fd(&w), fd) };
        drop(w);
        std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                PENDING.fetch_add(1, Ordering::SeqCst);
                let n = match r.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                let text = String::from_utf8_lossy(&buf[..n]).to_string();
                publish("stream", json!({"name": name, "text": text}));
                PENDING.fetch_sub(1, Ordering::SeqCst);
            }
        });
    }
}

/// Wait until what was written to fds 1 and 2 has gone out.
fn drain(py: Python<'_>) {
    if let Ok(sys) = py.import("sys") {
        for s in ["stdout", "stderr"] {
            let _ = sys.getattr(s).and_then(|f| f.call_method0("flush"));
        }
    }
    py.detach(|| {
        let start = std::time::Instant::now();
        loop {
            let mut waiting = 0i32;
            for fd in [1, 2] {
                let _ = fd;
            }
            // (Each reader counts 1 while blocked reading; more means it's
            // sending.)
            if PENDING.load(Ordering::SeqCst) <= 2 {
                std::thread::sleep(std::time::Duration::from_millis(15));
                if PENDING.load(Ordering::SeqCst) <= 2 {
                    break;
                }
            }
            waiting += 1;
            let _ = waiting;
            if start.elapsed() > std::time::Duration::from_secs(2) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
}

// ---- rich display ----------------------------------------------------------------------

fn py_to_json(py: Python<'_>, v: &Bound<'_, PyAny>) -> Value {
    if let Ok(s) = v.cast::<PyString>() {
        return Value::String(s.to_string());
    }
    if let Ok(b) = v.cast::<PyBytes>() {
        return Value::String(base64::engine::general_purpose::STANDARD.encode(b.as_bytes()));
    }
    py.import("json").and_then(|j| j.call_method1("dumps", (v,))).and_then(|s| s.extract::<String>()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null)
}

/// A value's representations: text/plain and whatever rich ones it has.
fn mimebundle(py: Python<'_>, obj: &Bound<'_, PyAny>) -> (Map<String, Value>, Map<String, Value>) {
    let mut data = Map::new();
    let mut meta = Map::new();
    if let Ok(r) = obj.repr() {
        data.insert("text/plain".into(), Value::String(r.to_string()));
    }
    if let Ok(f) = obj.getattr("_repr_mimebundle_") {
        if let Ok(b) = f.call0() {
            let (d, m) = if let Ok((d, m)) = b.extract::<(Bound<PyDict>, Bound<PyDict>)>() { (Some(d), Some(m)) } else { (b.cast_into::<PyDict>().ok(), None) };
            if let Some(d) = d {
                for (k, v) in d.iter() {
                    if let Ok(k) = k.extract::<String>() {
                        data.insert(k, py_to_json(py, &v));
                    }
                }
            }
            if let Some(m) = m {
                for (k, v) in m.iter() {
                    if let Ok(k) = k.extract::<String>() {
                        meta.insert(k, py_to_json(py, &v));
                    }
                }
            }
        }
    }
    for (meth, mime) in [("_repr_html_", "text/html"), ("_repr_markdown_", "text/markdown"), ("_repr_svg_", "image/svg+xml"), ("_repr_png_", "image/png"), ("_repr_jpeg_", "image/jpeg"), ("_repr_latex_", "text/latex"), ("_repr_json_", "application/json"), ("_repr_javascript_", "application/javascript")] {
        if data.contains_key(mime) {
            continue;
        }
        if let Ok(f) = obj.getattr(meth) {
            if let Ok(v) = f.call0() {
                if !v.is_none() {
                    let v = if let Ok((d, _)) = v.extract::<(Bound<PyAny>, Bound<PyAny>)>() { d } else { v };
                    data.insert(mime.into(), py_to_json(py, &v));
                }
            }
        }
    }
    // (A matplotlib figure: a PNG.)
    if !data.contains_key("image/png") && obj.hasattr("savefig").unwrap_or(false) && obj.get_type().name().map(|n| n.to_string() == "Figure").unwrap_or(false) {
        if let Some(png) = figure_png(py, obj) {
            data.insert("image/png".into(), Value::String(png));
        }
    }
    (data, meta)
}

fn figure_png(py: Python<'_>, fig: &Bound<'_, PyAny>) -> Option<String> {
    let io = py.import("io").ok()?;
    let buf = io.call_method0("BytesIO").ok()?;
    let kw = PyDict::new(py);
    kw.set_item("format", "png").ok()?;
    kw.set_item("bbox_inches", "tight").ok()?;
    fig.call_method("savefig", (&buf,), Some(&kw)).ok()?;
    let b: Vec<u8> = buf.call_method0("getvalue").ok()?.extract().ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(b))
}

static COUNT: AtomicU64 = AtomicU64::new(1);

#[pyfunction]
fn display_hook(py: Python<'_>, obj: Bound<'_, PyAny>) -> PyResult<()> {
    drain(py);
    let (data, meta) = mimebundle(py, &obj);
    py.import("builtins")?.setattr("_", &obj)?;
    publish("execute_result", json!({"execution_count": COUNT.load(Ordering::SeqCst), "data": data, "metadata": meta}));
    Ok(())
}

/// display(obj, …): show values as the cell runs.
#[pyfunction]
#[pyo3(signature = (*objs))]
fn display(py: Python<'_>, objs: Vec<Bound<'_, PyAny>>) -> PyResult<()> {
    drain(py);
    for o in objs {
        let (data, meta) = mimebundle(py, &o);
        publish("display_data", json!({"data": data, "metadata": meta, "transient": {}}));
    }
    Ok(())
}

/// matplotlib's open figures, shown and closed (an inline backend).
fn show_figures(py: Python<'_>) {
    let Ok(modules) = py.import("sys").and_then(|s| s.getattr("modules")) else { return };
    if !modules.contains("matplotlib.pyplot").unwrap_or(false) {
        return;
    }
    let Ok(plt) = py.import("matplotlib.pyplot") else { return };
    let Ok(nums) = plt.call_method0("get_fignums").and_then(|n| n.extract::<Vec<i64>>()) else { return };
    for n in nums {
        if let Ok(fig) = plt.call_method1("figure", (n,)) {
            if let Some(png) = figure_png(py, &fig) {
                publish("display_data", json!({"data": {"image/png": png, "text/plain": format!("<Figure {n}>")}, "metadata": {}, "transient": {}}));
            }
        }
    }
    let _ = plt.call_method1("close", ("all",));
}

// ---- input() -----------------------------------------------------------------------------

struct StdinBridge {
    allowed: AtomicBool,
    ids: Mutex<Vec<Bytes>>,
    req: Mutex<Option<tmpsc::UnboundedSender<(Vec<Bytes>, Value, Value)>>>,
    reply: Mutex<Option<mpsc::Receiver<String>>>,
}

static STDIN: StdinBridge = StdinBridge { allowed: AtomicBool::new(false), ids: Mutex::new(Vec::new()), req: Mutex::new(None), reply: Mutex::new(None) };

#[pyfunction]
#[pyo3(signature = (prompt=""))]
fn kernel_input(py: Python<'_>, prompt: &str) -> PyResult<String> {
    if !STDIN.allowed.load(Ordering::SeqCst) {
        return Err(pyo3::exceptions::PyEOFError::new_err("this cell can't read input"));
    }
    drain(py);
    let parent = PARENT.lock().unwrap().clone();
    let ids = STDIN.ids.lock().unwrap().clone();
    if let Some(tx) = STDIN.req.lock().unwrap().as_ref() {
        let _ = tx.send((ids, parent, json!({"prompt": prompt, "password": false})));
    }
    loop {
        let got = py.detach(|| STDIN.reply.lock().unwrap().as_ref().map(|rx| rx.recv_timeout(std::time::Duration::from_millis(50))));
        match got {
            Some(Ok(v)) => return Ok(v),
            Some(Err(mpsc::RecvTimeoutError::Timeout)) => py.check_signals()?,
            _ => return Err(pyo3::exceptions::PyEOFError::new_err("no input")),
        }
    }
}

// ---- running cells ----------------------------------------------------------------------

enum Job {
    Execute(Msg, tokio::sync::oneshot::Sender<Value>),
    Complete(Msg, tokio::sync::oneshot::Sender<Value>),
    IsComplete(Msg, tokio::sync::oneshot::Sender<Value>),
}

fn execute(py: Python<'_>, ns: &Bound<'_, PyDict>, msg: &Msg) -> Value {
    let code = msg.content["code"].as_str().unwrap_or("").to_string();
    let silent = msg.content["silent"].as_bool().unwrap_or(false);
    let store = msg.content["store_history"].as_bool().unwrap_or(true) && !silent;
    STDIN.allowed.store(msg.content["allow_stdin"].as_bool().unwrap_or(false), Ordering::SeqCst);
    *STDIN.ids.lock().unwrap() = msg.ids.clone();
    let count = COUNT.load(Ordering::SeqCst);
    if !silent {
        publish("execute_input", json!({"code": code, "execution_count": count}));
    }
    let filename = format!("<cell {count}>");
    let r = runner::run_source(py, &code, ns, &filename, !silent);
    show_figures(py);
    drain(py);
    if store {
        COUNT.fetch_add(1, Ordering::SeqCst);
    }
    let ok = json!({"status": "ok", "execution_count": count, "user_expressions": {}, "payload": []});
    let (ename, evalue, tb) = match r {
        Ok(()) => return ok,
        Err(TesshError::Py(e)) => {
            if e.is_instance_of::<pyo3::exceptions::PySystemExit>(py) {
                return ok;
            }
            let ename = e.get_type(py).name().map(|n| n.to_string()).unwrap_or_default();
            let evalue = e.value(py).str().map(|s| s.to_string()).unwrap_or_default();
            let tb = py
                .import("traceback")
                .and_then(|t| t.call_method1("format_exception", (e.value(py),)))
                .and_then(|l| l.extract::<Vec<String>>())
                .unwrap_or_else(|_| vec![format!("{ename}: {evalue}")]);
            // (tessh's own frames left out.)
            let tb: Vec<String> = tb.into_iter().filter(|l| !l.contains("<macro ") || true).collect();
            (ename, evalue, vec![tb.concat().trim_end().to_string()])
        }
        Err(e) => {
            let (l, c, end, m) = e.position().unwrap();
            let line = code.lines().nth(l.saturating_sub(1) as usize).unwrap_or("").to_string();
            let caret = format!("{}{}", " ".repeat(c as usize), "^".repeat((end.saturating_sub(c)).max(1) as usize));
            ("SyntaxError".into(), format!("{l}:{}: {m}", c + 1), vec![format!("\x1b[31mSyntaxError\x1b[0m: {l}:{}: {m}", c + 1), format!("  {line}"), format!("  {caret}")])
        }
    };
    publish("error", json!({"ename": ename, "evalue": evalue, "traceback": tb}));
    json!({"status": "error", "execution_count": count, "ename": ename, "evalue": evalue, "traceback": tb})
}

fn complete(py: Python<'_>, ns: &Bound<'_, PyDict>, msg: &Msg) -> Value {
    let code = msg.content["code"].as_str().unwrap_or("");
    let chars: Vec<char> = code.chars().collect();
    let pos = (msg.content["cursor_pos"].as_u64().unwrap_or(chars.len() as u64) as usize).min(chars.len());
    let before: String = chars[..pos].iter().collect();
    let start_b = before.rfind(|c: char| " \t\n()[]{}\"'`;$".contains(c)).map(|i| i + 1).unwrap_or(0);
    let word = &before[start_b..];
    let line_before = before[..start_b].rsplit('\n').next().unwrap_or("");
    let head = line_before.trim_end();
    let first = head.is_empty() || head.ends_with(['|', ';', '(']) || head.ends_with("&&") || head.ends_with("||");
    let matches = crate::repl::completions(py, ns, word, first, &before[..start_b]);
    let start = before[..start_b].chars().count();
    json!({"status": "ok", "matches": matches, "cursor_start": start, "cursor_end": pos, "metadata": {}})
}

fn is_complete(ns: &Bound<'_, PyDict>, msg: &Msg) -> Value {
    let code = msg.content["code"].as_str().unwrap_or("");
    let lines: Vec<&str> = code.split('\n').collect();
    let macros = runner::macro_names(ns);
    if crate::repl::needs_more(&lines, &macros) {
        return json!({"status": "incomplete", "indent": " ".repeat(crate::repl::next_indent(&lines, &macros))});
    }
    json!({"status": "complete"})
}

// ---- the sockets ---------------------------------------------------------------------------

pub fn run(conn_file: &str) -> i32 {
    let conn: Value = match std::fs::read_to_string(conn_file).ok().and_then(|t| serde_json::from_str(&t).ok()) {
        Some(c) => c,
        None => {
            eprintln!("tessh: can't read the connection file {conn_file}");
            return 2;
        }
    };
    *SESSION.lock().unwrap() = uuid::Uuid::new_v4().to_string();
    let signer = Signer { key: conn["key"].as_str().unwrap_or("").as_bytes().to_vec() };
    let ip = conn["ip"].as_str().unwrap_or("127.0.0.1").to_string();
    let transport = conn["transport"].as_str().unwrap_or("tcp").to_string();
    let addr = move |k: &str| format!("{transport}://{ip}:{}", conn[k].as_u64().unwrap_or(0));
    let (jobs_tx, jobs_rx) = mpsc::channel::<Job>();
    let (io_tx, io_rx) = tmpsc::unbounded_channel::<IoMsg>();
    *IOPUB.lock().unwrap() = Some(io_tx);
    let (in_req_tx, in_req_rx) = tmpsc::unbounded_channel();
    let (in_rep_tx, in_rep_rx) = mpsc::channel::<String>();
    *STDIN.req.lock().unwrap() = Some(in_req_tx);
    *STDIN.reply.lock().unwrap() = Some(in_rep_rx);
    let ready = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    let r2 = ready.clone();
    let addrs = (addr("shell_port"), addr("control_port"), addr("stdin_port"), addr("iopub_port"), addr("hb_port"));
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().unwrap();
        rt.block_on(async move {
            let mut shell = zeromq::RouterSocket::new();
            let mut control = zeromq::RouterSocket::new();
            let mut stdin = zeromq::RouterSocket::new();
            let mut iopub = zeromq::PubSocket::new();
            let mut hb = zeromq::RepSocket::new();
            for (s, a) in [(&mut shell as &mut dyn Bindable, &addrs.0), (&mut control, &addrs.1), (&mut stdin, &addrs.2)] {
                s.bind_to(a).await;
            }
            let _ = iopub.bind(&addrs.3).await;
            let _ = hb.bind(&addrs.4).await;
            {
                let (l, c) = &*r2;
                *l.lock().unwrap() = true;
                c.notify_all();
            }
            // Heartbeat: echo.
            tokio::spawn(async move {
                while let Ok(m) = hb.recv().await {
                    let _ = hb.send(m).await;
                }
            });
            // iopub: everything published, in order.
            let s_io = signer.clone();
            tokio::spawn(async move {
                let mut io_rx = io_rx;
                while let Some((t, content, parent)) = io_rx.recv().await {
                    let topic = Bytes::from(format!("kernel.{}", t));
                    let _ = iopub.send(build(&s_io, &[topic], &t, &parent, content)).await;
                }
            });
            // stdin: input requests out, replies back.
            let s_in = signer.clone();
            tokio::spawn(async move {
                let mut in_req_rx = in_req_rx;
                loop {
                    tokio::select! {
                        Some((ids, parent, content)) = in_req_rx.recv() => {
                            let _ = stdin.send(build(&s_in, &ids, "input_request", &parent, content)).await;
                        }
                        Ok(m) = stdin.recv() => {
                            if let Some(msg) = parse(m.into_vec()) {
                                if msg.header["msg_type"] == "input_reply" {
                                    let _ = in_rep_tx.send(msg.content["value"].as_str().unwrap_or("").to_string());
                                }
                            }
                        }
                    }
                }
            });
            // control: interrupts and shutdown (alongside a running cell).
            let s_ctl = signer.clone();
            tokio::spawn(async move {
                while let Ok(m) = control.recv().await {
                    let Some(msg) = parse(m.into_vec()) else { continue };
                    let t = msg.header["msg_type"].as_str().unwrap_or("").to_string();
                    let reply = |name: &str, content: Value| build(&s_ctl, &msg.ids, name, &msg.header, content);
                    match t.as_str() {
                        "interrupt_request" => {
                            unsafe { libc::kill(libc::getpid(), libc::SIGINT) };
                            let _ = control.send(reply("interrupt_reply", json!({"status": "ok"}))).await;
                        }
                        "shutdown_request" => {
                            let _ = control.send(reply("shutdown_reply", json!({"status": "ok", "restart": msg.content["restart"]}))).await;
                            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            std::process::exit(0);
                        }
                        "kernel_info_request" => {
                            let _ = control.send(reply("kernel_info_reply", kernel_info())).await;
                        }
                        _ => {}
                    }
                }
            });
            // shell: requests, one at a time.
            let send_status = |state: &str, parent: &Value| {
                if let Some(tx) = IOPUB.lock().unwrap().as_ref() {
                    let _ = tx.send(("status".into(), json!({"execution_state": state}), parent.clone()));
                }
            };
            send_status("starting", &Value::Null);
            while let Ok(m) = shell.recv().await {
                let Some(msg) = parse(m.into_vec()) else { continue };
                let t = msg.header["msg_type"].as_str().unwrap_or("").to_string();
                send_status("busy", &msg.header);
                let (reply_type, content) = match t.as_str() {
                    "kernel_info_request" => ("kernel_info_reply", kernel_info()),
                    "execute_request" | "complete_request" | "is_complete_request" => {
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        let job = match t.as_str() {
                            "execute_request" => Job::Execute(msg.clone(), tx),
                            "complete_request" => Job::Complete(msg.clone(), tx),
                            _ => Job::IsComplete(msg.clone(), tx),
                        };
                        let _ = jobs_tx.send(job);
                        let v = rx.await.unwrap_or(json!({"status": "error"}));
                        (match t.as_str() { "execute_request" => "execute_reply", "complete_request" => "complete_reply", _ => "is_complete_reply" }, v)
                    }
                    "inspect_request" => ("inspect_reply", json!({"status": "ok", "found": false, "data": {}, "metadata": {}})),
                    "history_request" => ("history_reply", json!({"status": "ok", "history": []})),
                    "comm_info_request" => ("comm_info_reply", json!({"status": "ok", "comms": {}})),
                    "shutdown_request" => {
                        let _ = shell.send(build(&signer, &msg.ids, "shutdown_reply", &msg.header, json!({"status": "ok", "restart": msg.content["restart"]}))).await;
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        std::process::exit(0);
                    }
                    _ => {
                        send_status("idle", &msg.header);
                        continue;
                    }
                };
                let _ = shell.send(build(&signer, &msg.ids, reply_type, &msg.header, content)).await;
                send_status("idle", &msg.header);
            }
        });
    });
    {
        let (l, c) = &*ready;
        let mut g = l.lock().unwrap();
        while !*g {
            g = c.wait(g).unwrap();
        }
    }
    capture_fds();
    let jobs_rx = Mutex::new(jobs_rx);
    // The main thread runs Python.
    Python::attach(|py| -> PyResult<()> {
        runner::init(py)?;
        let ns = runner::new_namespace(py, "__main__", vec![])?;
        let d = wrap_pyfunction!(display, py)?;
        ns.set_item("display", d)?;
        let b = py.import("builtins")?;
        b.setattr("input", wrap_pyfunction!(kernel_input, py)?)?;
        runtime::set_display(wrap_pyfunction!(display_hook, py)?.into_any().unbind());
        let sys = py.import("sys")?;
        for s in ["stdout", "stderr"] {
            let kw = PyDict::new(py);
            kw.set_item("line_buffering", true)?;
            let _ = sys.getattr(s)?.call_method("reconfigure", (), Some(&kw));
        }
        if std::env::var_os("MPLBACKEND").is_none() {
            std::env::set_var("MPLBACKEND", "Agg");
            runtime::setenv(py, "MPLBACKEND", Some("Agg"))?;
        }
        loop {
            let job = match py.detach(|| jobs_rx.lock().unwrap().recv()) {
                Ok(j) => j,
                Err(_) => break,
            };
            match job {
                Job::Execute(msg, tx) => {
                    *PARENT.lock().unwrap() = msg.header.clone();
                    let v = execute(py, &ns, &msg);
                    let _ = tx.send(v);
                }
                Job::Complete(msg, tx) => {
                    let _ = tx.send(complete(py, &ns, &msg));
                }
                Job::IsComplete(msg, tx) => {
                    let _ = tx.send(is_complete(&ns, &msg));
                }
            }
            // (An interrupt that came between cells mustn't hit the next.)
            let _ = py.check_signals();
        }
        Ok(())
    })
    .map(|_| 0)
    .unwrap_or_else(|e| {
        eprintln!("tessh kernel: {e}");
        1
    })
}

fn kernel_info() -> Value {
    json!({"status": "ok", "protocol_version": "5.3", "implementation": "tessh", "implementation_version": env!("CARGO_PKG_VERSION"),
           "language_info": {"name": "tessh", "version": env!("CARGO_PKG_VERSION"), "mimetype": "text/x-tessh", "file_extension": ".tsh",
                             "pygments_lexer": "text", "codemirror_mode": "text"},
           "banner": format!("tessh {}: a shell language on Python", env!("CARGO_PKG_VERSION")), "help_links": []})
}

#[async_trait::async_trait]
trait Bindable {
    async fn bind_to(&mut self, a: &str);
}

#[async_trait::async_trait]
impl Bindable for zeromq::RouterSocket {
    async fn bind_to(&mut self, a: &str) {
        if let Err(e) = self.bind(a).await {
            eprintln!("tessh kernel: can't listen on {a}: {e}");
        }
    }
}
