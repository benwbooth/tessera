//! Jupyter kernels for notebook documents (.ipynb): finding the installed
//! ones (kernelspecs), starting one, and talking to it the way Jupyter does
//! (signed JSON messages over ZeroMQ: code to the shell socket, outputs back
//! on iopub). The ZeroMQ part is the little of ZMTP 3.0 a client needs.

use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ---- ZMTP 3.0 (NULL security): enough for DEALER and SUB sockets ------------

pub struct Zmtp {
    stream: TcpStream,
}

impl Zmtp {
    /// Connect and greet as `socket_type` ("DEALER", "SUB").
    pub fn connect(port: u16, socket_type: &str, wait: Duration) -> std::io::Result<Zmtp> {
        let start = Instant::now();
        let stream = loop {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(s) => break s,
                Err(e) if start.elapsed() < wait => {
                    let _ = e;
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => return Err(e),
            }
        };
        stream.set_nodelay(true)?;
        let mut z = Zmtp { stream };
        // Greeting: signature, version 3.0, NULL mechanism, client, filler.
        let mut g = vec![0xFFu8, 0, 0, 0, 0, 0, 0, 0, 1, 0x7F, 3, 0];
        let mut mech = b"NULL".to_vec();
        mech.resize(20, 0);
        g.extend_from_slice(&mech);
        g.push(0);
        g.resize(64, 0);
        z.stream.write_all(&g)?;
        let mut peer = [0u8; 64];
        z.stream.read_exact(&mut peer)?;
        if peer[0] != 0xFF || peer[9] != 0x7F || peer[10] < 3 {
            return Err(std::io::Error::other("not a ZMTP 3 peer"));
        }
        // READY with our socket type.
        let mut body = vec![5u8];
        body.extend_from_slice(b"READY");
        let prop = |body: &mut Vec<u8>, name: &str, value: &[u8]| {
            body.push(name.len() as u8);
            body.extend_from_slice(name.as_bytes());
            body.extend_from_slice(&(value.len() as u32).to_be_bytes());
            body.extend_from_slice(value);
        };
        prop(&mut body, "Socket-Type", socket_type.as_bytes());
        if socket_type == "DEALER" {
            prop(&mut body, "Identity", b"");
        }
        z.write_frame(&body, false, true)?;
        // Their READY (a command frame).
        loop {
            let (cmd, _more, _body) = z.read_frame()?;
            if cmd {
                break;
            }
        }
        if socket_type == "SUB" {
            // Subscribe to everything (3.0 style: a message starting with 1).
            z.write_frame(&[1], false, false)?;
        }
        Ok(z)
    }

    pub fn try_clone(&self) -> std::io::Result<Zmtp> {
        Ok(Zmtp { stream: self.stream.try_clone()? })
    }

    fn write_frame(&mut self, body: &[u8], more: bool, command: bool) -> std::io::Result<()> {
        let mut flags = 0u8;
        if more {
            flags |= 1;
        }
        if command {
            flags |= 4;
        }
        let mut head = Vec::with_capacity(9);
        if body.len() > 255 {
            head.push(flags | 2);
            head.extend_from_slice(&(body.len() as u64).to_be_bytes());
        } else {
            head.push(flags);
            head.push(body.len() as u8);
        }
        self.stream.write_all(&head)?;
        self.stream.write_all(body)
    }

    fn read_frame(&mut self) -> std::io::Result<(bool, bool, Vec<u8>)> {
        let mut f = [0u8; 1];
        self.stream.read_exact(&mut f)?;
        let len = if f[0] & 2 != 0 {
            let mut l = [0u8; 8];
            self.stream.read_exact(&mut l)?;
            u64::from_be_bytes(l) as usize
        } else {
            let mut l = [0u8; 1];
            self.stream.read_exact(&mut l)?;
            l[0] as usize
        };
        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body)?;
        Ok((f[0] & 4 != 0, f[0] & 1 != 0, body))
    }

    pub fn send(&mut self, parts: &[Vec<u8>]) -> std::io::Result<()> {
        let mut out = Vec::new();
        for (i, p) in parts.iter().enumerate() {
            let more = i + 1 < parts.len();
            let flags = if more { 1u8 } else { 0 };
            if p.len() > 255 {
                out.push(flags | 2);
                out.extend_from_slice(&(p.len() as u64).to_be_bytes());
            } else {
                out.push(flags);
                out.push(p.len() as u8);
            }
            out.extend_from_slice(p);
        }
        self.stream.write_all(&out)
    }

    /// The next message (its parts); commands in between are skipped.
    pub fn recv(&mut self) -> std::io::Result<Vec<Vec<u8>>> {
        let mut parts = Vec::new();
        loop {
            let (cmd, more, body) = self.read_frame()?;
            if cmd {
                continue;
            }
            parts.push(body);
            if !more {
                return Ok(parts);
            }
        }
    }

    pub fn shutdown(&self) {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
    }
}

// ---- Jupyter messages ------------------------------------------------------

fn random_hex(bytes: usize) -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut s = String::new();
    while s.len() < bytes * 2 {
        let mut h = RandomState::new().build_hasher();
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0));
        s.push_str(&format!("{:016x}", h.finish()));
    }
    s.truncate(bytes * 2);
    s
}

/// Now as ISO 8601 (UTC), the way Jupyter dates its messages.
fn iso_now() -> String {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let secs = t.as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // Days to a civil date (Howard Hinnant's algorithm).
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

#[derive(Clone)]
struct Signer {
    key: Vec<u8>,
    session: String,
}

impl Signer {
    fn sign(&self, parts: &[&[u8]]) -> String {
        if self.key.is_empty() {
            return String::new();
        }
        let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&self.key).expect("hmac key");
        for p in parts {
            mac.update(p);
        }
        mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }

    /// A message ready to send; returns its id too.
    fn message(&self, msg_type: &str, content: Value) -> (String, Vec<Vec<u8>>) {
        let id = random_hex(16);
        let header = json!({"msg_id": id, "username": "tessera", "session": self.session, "date": iso_now(),
                            "msg_type": msg_type, "version": "5.3"})
        .to_string()
        .into_bytes();
        let parent = b"{}".to_vec();
        let meta = b"{}".to_vec();
        let content = content.to_string().into_bytes();
        let sig = self.sign(&[&header, &parent, &meta, &content]);
        (id, vec![b"<IDS|MSG>".to_vec(), sig.into_bytes(), header, parent, meta, content])
    }

    /// (msg_type, parent msg_id, content) of a received message, if it's signed right.
    fn parse(&self, parts: &[Vec<u8>]) -> Option<(String, String, Value)> {
        let at = parts.iter().position(|p| p == b"<IDS|MSG>")?;
        let rest = &parts[at + 1..];
        if rest.len() < 5 {
            return None;
        }
        if !self.key.is_empty() && self.sign(&[&rest[1], &rest[2], &rest[3], &rest[4]]).as_bytes() != rest[0].as_slice() {
            return None;
        }
        let header: Value = serde_json::from_slice(&rest[1]).ok()?;
        let parent: Value = serde_json::from_slice(&rest[2]).unwrap_or(Value::Null);
        let content: Value = serde_json::from_slice(&rest[4]).unwrap_or(Value::Null);
        Some((
            header["msg_type"].as_str().unwrap_or("").to_string(),
            parent["msg_id"].as_str().unwrap_or("").to_string(),
            content,
        ))
    }
}

// ---- kernelspecs -------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Spec {
    pub name: String,
    pub display: String,
    pub language: String,
    pub argv: Vec<String>,
    pub env: HashMap<String, String>,
    pub interrupt_message: bool,
}

fn data_dirs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(p) = std::env::var_os("JUPYTER_PATH") {
        v.extend(std::env::split_paths(&p));
    }
    if let Some(h) = std::env::var_os("HOME").map(PathBuf::from) {
        v.push(h.join(".local/share/jupyter"));
        v.push(h.join("Library/Jupyter"));
        v.push(h.join(".nix-profile/share/jupyter"));
    }
    if let Some(a) = std::env::var_os("APPDATA").map(PathBuf::from) {
        v.push(a.join("jupyter"));
    }
    if let Ok(u) = std::env::var("USER") {
        v.push(PathBuf::from(format!("/etc/profiles/per-user/{u}/share/jupyter")));
    }
    for d in ["/run/current-system/sw/share/jupyter", "/usr/local/share/jupyter", "/usr/share/jupyter", "/opt/homebrew/share/jupyter"] {
        v.push(PathBuf::from(d));
    }
    v
}

fn read_spec(name: &str, path: &Path) -> Option<Spec> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path.join("kernel.json")).ok()?).ok()?;
    let argv: Vec<String> = v["argv"].as_array()?.iter().filter_map(|a| a.as_str().map(String::from)).collect();
    if argv.is_empty() {
        return None;
    }
    let env = v["env"].as_object().map(|o| o.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect()).unwrap_or_default();
    Some(Spec {
        name: name.to_string(),
        display: v["display_name"].as_str().unwrap_or(name).to_string(),
        language: v["language"].as_str().unwrap_or("").to_string(),
        argv: argv.into_iter().map(|a| a.replace("{resource_dir}", &path.to_string_lossy())).collect(),
        env,
        interrupt_message: v["interrupt_mode"].as_str() == Some("message"),
    })
}

fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(program).is_file()))
}

/// Python packages the Nix kernel comes with.
pub const NIX_PYTHON_PACKAGES: &str = "ipykernel numpy pandas matplotlib";

/// Every kernel there is: installed kernelspecs, then Python with ipykernel
/// (when no spec names it), then Python from Nix (fetched the first time).
pub fn specs() -> Vec<Spec> {
    static CACHE: Mutex<Option<(Instant, Vec<Spec>)>> = Mutex::new(None);
    if let Some((at, v)) = CACHE.lock().unwrap().as_ref() {
        if at.elapsed() < Duration::from_secs(30) {
            return v.clone();
        }
    }
    let mut out: Vec<Spec> = Vec::new();
    for d in data_dirs() {
        let Ok(rd) = std::fs::read_dir(d.join("kernels")) else { continue };
        let mut names: Vec<_> = rd.flatten().collect();
        names.sort_by_key(|e| e.file_name());
        for e in names {
            let name = e.file_name().to_string_lossy().into_owned();
            if out.iter().any(|s| s.name == name) {
                continue;
            }
            if let Some(s) = read_spec(&name, &e.path()) {
                out.push(s);
            }
        }
    }
    if !out.iter().any(|s| s.name == "python3") {
        for py in ["python3", "python"] {
            let ok = on_path(py)
                && Command::new(py).args(["-c", "import ipykernel"]).stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success());
            if ok {
                out.push(Spec {
                    name: "python3".into(),
                    display: "Python 3 (ipykernel)".into(),
                    language: "python".into(),
                    argv: [py, "-m", "ipykernel_launcher", "-f", "{connection_file}"].map(String::from).to_vec(),
                    env: HashMap::new(),
                    interrupt_message: false,
                });
                break;
            }
        }
    }
    if on_path("nix") {
        let expr = format!(
            "(builtins.getFlake \"nixpkgs\").legacyPackages.${{builtins.currentSystem}}.python3.withPackages (p: with p; [ {NIX_PYTHON_PACKAGES} ])"
        );
        out.push(Spec {
            name: "tessera-nix-python".into(),
            display: "Python 3 (from Nix)".into(),
            language: "python".into(),
            argv: ["nix", "shell", "--impure", "--expr", &expr, "-c", "python3", "-m", "ipykernel_launcher", "-f", "{connection_file}"]
                .map(String::from)
                .to_vec(),
            env: HashMap::new(),
            interrupt_message: false,
        });
    }
    *CACHE.lock().unwrap() = Some((Instant::now(), out.clone()));
    out
}

pub fn specs_json() -> String {
    let v: Vec<Value> = specs().iter().map(|s| json!({"name": s.name, "display": s.display, "language": s.language})).collect();
    Value::Array(v).to_string()
}

// ---- a running kernel --------------------------------------------------------

/// What a kernel reports, as JSON for the notebook view:
///   {"type":"status","state":"starting|idle|busy|dead","message":...}
///   {"type":"output","cell":id,"output":{an nbformat output}}
///   {"type":"clear","cell":id,"wait":bool}
///   {"type":"count","cell":id,"count":n}
///   {"type":"done","cell":id,"status":"ok|error|aborted","count":n}
///   {"type":"info","language":...,"banner":...}
///   {"type":"complete","request":id,"matches":[...],"start":n,"end":n}
pub type Sink = Arc<dyn Fn(Value) + Send + Sync>;

pub struct Kernel {
    signer: Signer,
    shell: Mutex<Option<Zmtp>>,
    control: Mutex<Option<Zmtp>>,
    child: Mutex<Option<Child>>,
    /// Message id -> the cell (or completion request) it was for.
    pending: Arc<Mutex<HashMap<String, String>>>,
    spec: Spec,
    conn_file: PathBuf,
    gone: Arc<AtomicBool>,
    /// It has answered kernel_info: it runs code now.
    ready: AtomicBool,
}

fn free_port() -> std::io::Result<u16> {
    Ok(std::net::TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

/// iopub content as an nbformat output.
fn as_output(msg_type: &str, c: &Value) -> Option<Value> {
    match msg_type {
        "stream" => Some(json!({"output_type": "stream", "name": c["name"], "text": c["text"]})),
        "display_data" | "update_display_data" => Some(json!({"output_type": "display_data", "data": c["data"], "metadata": c["metadata"]})),
        "execute_result" => {
            Some(json!({"output_type": "execute_result", "data": c["data"], "metadata": c["metadata"], "execution_count": c["execution_count"]}))
        }
        "error" => Some(json!({"output_type": "error", "ename": c["ename"], "evalue": c["evalue"], "traceback": c["traceback"]})),
        _ => None,
    }
}

impl Kernel {
    pub fn start(spec_name: &str, cwd: &str, sink: Sink) -> Result<Arc<Kernel>, String> {
        let spec = specs().into_iter().find(|s| s.name == spec_name).ok_or_else(|| format!("No kernel named {spec_name}"))?;
        sink(json!({"type": "status", "state": "starting", "message": format!("Starting {}…", spec.display)}));
        let ports: Vec<u16> = (0..5).map(|_| free_port()).collect::<Result<_, _>>().map_err(|e| e.to_string())?;
        let key = random_hex(32);
        let conn = json!({"transport": "tcp", "ip": "127.0.0.1", "shell_port": ports[0], "iopub_port": ports[1],
                          "stdin_port": ports[2], "control_port": ports[3], "hb_port": ports[4],
                          "key": key, "signature_scheme": "hmac-sha256", "kernel_name": spec.name});
        let dir = crate::settings::state_dir().join("kernels");
        let _ = std::fs::create_dir_all(&dir);
        let conn_file = dir.join(format!("kernel-{}.json", random_hex(6)));
        {
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create(true).truncate(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
            let mut f = opts.open(&conn_file).map_err(|e| e.to_string())?;
            f.write_all(conn.to_string().as_bytes()).map_err(|e| e.to_string())?;
        }
        let argv: Vec<String> = spec.argv.iter().map(|a| a.replace("{connection_file}", &conn_file.to_string_lossy())).collect();
        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..]).envs(&spec.env).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
        if !cwd.is_empty() && Path::new(cwd).is_dir() {
            cmd.current_dir(cwd);
        }
        #[cfg(unix)]
        {
            // Its own process group: Ctrl+C in a terminal doesn't reach it.
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        let mut child = cmd.spawn().map_err(|e| format!("Couldn't start {}: {e}", argv[0]))?;
        // Its complaints, for when it dies before answering.
        let stderr_tail: Arc<Mutex<String>> = Arc::default();
        if let Some(mut err) = child.stderr.take() {
            let tail = stderr_tail.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 4096];
                while let Ok(n) = err.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let mut t = tail.lock().unwrap();
                    t.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if t.len() > 8000 {
                        let cut = t.len() - 4000;
                        let cut = (cut..t.len()).find(|&i| t.is_char_boundary(i)).unwrap_or(t.len());
                        t.drain(..cut);
                    }
                }
            });
        }
        let signer = Signer { key: key.into_bytes(), session: random_hex(16) };
        let kernel = Arc::new(Kernel {
            signer,
            shell: Mutex::new(None),
            control: Mutex::new(None),
            child: Mutex::new(Some(child)),
            pending: Arc::default(),
            spec,
            conn_file,
            gone: Arc::default(),
            ready: AtomicBool::new(false),
        });
        // Connect (Nix may take minutes to fetch Python the first time), then
        // ask who it is; its reply means it's ready.
        let k = kernel.clone();
        std::thread::spawn(move || {
            let wait = Duration::from_secs(if k.spec.name == "tessera-nix-python" { 900 } else { 120 });
            let start = Instant::now();
            let connect = |port: u16, ty: &str| -> Result<Zmtp, String> {
                loop {
                    if k.gone.load(Ordering::SeqCst) {
                        return Err("stopped".into());
                    }
                    if let Some(c) = k.child.lock().unwrap().as_mut() {
                        if let Ok(Some(st)) = c.try_wait() {
                            return Err(format!("The kernel quit ({st}).\n{}", stderr_tail.lock().unwrap().trim()));
                        }
                    }
                    match Zmtp::connect(port, ty, Duration::from_millis(500)) {
                        Ok(z) => return Ok(z),
                        Err(_) if start.elapsed() < wait => continue,
                        Err(e) => return Err(format!("Couldn't reach the kernel: {e}")),
                    }
                }
            };
            let result = (|| -> Result<(), String> {
                let iopub = connect(ports[1], "SUB")?;
                let shell = connect(ports[0], "DEALER")?;
                let control = connect(ports[3], "DEALER")?;
                let shell_reader = shell.try_clone().map_err(|e| e.to_string())?;
                *k.shell.lock().unwrap() = Some(shell);
                *k.control.lock().unwrap() = Some(control);
                k.spawn_readers(iopub, shell_reader, sink.clone());
                // (Repeated until it answers: the SUB socket may miss the first replies.)
                for _ in 0..40 {
                    if k.ready.load(Ordering::SeqCst) || k.gone.load(Ordering::SeqCst) {
                        break;
                    }
                    k.send_shell("kernel_info_request", json!({}), "info");
                    std::thread::sleep(Duration::from_millis(250));
                }
                Ok(())
            })();
            if let Err(e) = result {
                if !k.gone.load(Ordering::SeqCst) {
                    sink(json!({"type": "status", "state": "dead", "message": e}));
                }
                k.stop();
                return;
            }
            // Watch for it dying.
            let k2 = k.clone();
            loop {
                std::thread::sleep(Duration::from_millis(500));
                if k2.gone.load(Ordering::SeqCst) {
                    break;
                }
                let exited = k2.child.lock().unwrap().as_mut().map(|c| c.try_wait().ok().flatten().is_some()).unwrap_or(true);
                if exited {
                    k2.gone.store(true, Ordering::SeqCst);
                    let tail = stderr_tail.lock().unwrap().clone();
                    let last: Vec<&str> = tail.trim().lines().rev().take(6).collect::<Vec<_>>().into_iter().rev().collect();
                    sink(json!({"type": "status", "state": "dead", "message": format!("The kernel died.\n{}", last.join("\n"))}));
                    break;
                }
            }
        });
        Ok(kernel)
    }

    fn spawn_readers(self: &Arc<Self>, mut iopub: Zmtp, mut shell: Zmtp, sink: Sink) {
        let k = self.clone();
        let s2 = sink.clone();
        std::thread::spawn(move || {
            while let Ok(parts) = iopub.recv() {
                let Some((ty, parent, c)) = k.signer.parse(&parts) else { continue };
                let cell = k.pending.lock().unwrap().get(&parent).cloned();
                match ty.as_str() {
                    "status" => {
                        let st = c["execution_state"].as_str().unwrap_or("");
                        if st == "idle" || st == "busy" {
                            if k.ready.load(Ordering::SeqCst) {
                                s2(json!({"type": "status", "state": st}));
                            }
                        }
                    }
                    "execute_input" => {
                        if let Some(cell) = cell {
                            s2(json!({"type": "count", "cell": cell, "count": c["execution_count"]}));
                        }
                    }
                    "clear_output" => {
                        if let Some(cell) = cell {
                            s2(json!({"type": "clear", "cell": cell, "wait": c["wait"].as_bool().unwrap_or(false)}));
                        }
                    }
                    _ => {
                        if let (Some(cell), Some(out)) = (cell, as_output(&ty, &c)) {
                            s2(json!({"type": "output", "cell": cell, "output": out}));
                        }
                    }
                }
            }
        });
        let k = self.clone();
        std::thread::spawn(move || {
            while let Ok(parts) = shell.recv() {
                let Some((ty, parent, c)) = k.signer.parse(&parts) else { continue };
                let Some(what) = k.pending.lock().unwrap().remove(&parent) else { continue };
                match ty.as_str() {
                    "kernel_info_reply" => {
                        if !k.ready.swap(true, Ordering::SeqCst) {
                            let li = &c["language_info"];
                            sink(json!({"type": "info", "language": li["name"], "version": li["version"],
                                        "extension": li["file_extension"], "banner": c["banner"], "implementation": c["implementation"]}));
                            sink(json!({"type": "status", "state": "idle"}));
                        }
                    }
                    "execute_reply" => {
                        let status = c["status"].as_str().unwrap_or("ok");
                        sink(json!({"type": "done", "cell": what, "status": status, "count": c["execution_count"]}));
                    }
                    "complete_reply" => {
                        sink(json!({"type": "complete", "request": what, "matches": c["matches"],
                                    "start": c["cursor_start"], "end": c["cursor_end"]}));
                    }
                    _ => {}
                }
            }
        });
    }

    fn send_shell(&self, msg_type: &str, content: Value, tag: &str) -> bool {
        let (id, parts) = self.signer.message(msg_type, content);
        self.pending.lock().unwrap().insert(id, tag.to_string());
        let mut g = self.shell.lock().unwrap();
        g.as_mut().is_some_and(|s| s.send(&parts).is_ok())
    }

    pub fn execute(&self, cell: &str, code: &str) -> bool {
        self.send_shell(
            "execute_request",
            json!({"code": code, "silent": false, "store_history": true, "user_expressions": {}, "allow_stdin": false, "stop_on_error": true}),
            cell,
        )
    }

    pub fn complete(&self, request: &str, code: &str, pos: usize) -> bool {
        self.send_shell("complete_request", json!({"code": code, "cursor_pos": pos}), request)
    }

    pub fn interrupt(&self) {
        if self.spec.interrupt_message {
            let (_, parts) = self.signer.message("interrupt_request", json!({}));
            if let Some(c) = self.control.lock().unwrap().as_mut() {
                let _ = c.send(&parts);
            }
            return;
        }
        #[cfg(unix)]
        if let Some(c) = self.child.lock().unwrap().as_ref() {
            unsafe {
                libc::kill(c.id() as i32, libc::SIGINT);
            }
        }
    }

    /// Ask it to quit, then make sure.
    pub fn stop(&self) {
        let was_gone = self.gone.swap(true, Ordering::SeqCst);
        if !was_gone {
            let (_, parts) = self.signer.message("shutdown_request", json!({"restart": false}));
            if let Some(c) = self.control.lock().unwrap().as_mut() {
                let _ = c.send(&parts);
            }
        }
        if let Some(mut c) = self.child.lock().unwrap().take() {
            let deadline = Instant::now() + Duration::from_millis(1500);
            while Instant::now() < deadline {
                if let Ok(Some(_)) = c.try_wait() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = c.kill();
            let _ = c.wait();
        }
        for s in [&self.shell, &self.control] {
            if let Some(z) = s.lock().unwrap().take() {
                z.shutdown();
            }
        }
        let _ = std::fs::remove_file(&self.conn_file);
    }
}

impl Drop for Kernel {
    fn drop(&mut self) {
        self.stop();
    }
}

// ---- the running kernels, by the notebook's key ----------------------------

static KERNELS: Mutex<Option<HashMap<String, Arc<Kernel>>>> = Mutex::new(None);

pub fn start(key: &str, spec: &str, cwd: &str, sink: Sink) {
    stop(key);
    let (key, spec, cwd) = (key.to_string(), spec.to_string(), cwd.to_string());
    std::thread::spawn(move || match Kernel::start(&spec, &cwd, sink.clone()) {
        Ok(k) => {
            KERNELS.lock().unwrap().get_or_insert_with(HashMap::new).insert(key, k);
        }
        Err(e) => sink(json!({"type": "status", "state": "dead", "message": e})),
    });
}

fn get(key: &str) -> Option<Arc<Kernel>> {
    KERNELS.lock().unwrap().as_ref()?.get(key).cloned()
}

pub fn execute(key: &str, cell: &str, code: &str) -> bool {
    get(key).is_some_and(|k| k.execute(cell, code))
}

pub fn complete(key: &str, request: &str, code: &str, pos: usize) -> bool {
    get(key).is_some_and(|k| k.complete(request, code, pos))
}

pub fn interrupt(key: &str) {
    if let Some(k) = get(key) {
        k.interrupt();
    }
}

pub fn stop(key: &str) {
    let k = KERNELS.lock().unwrap().as_mut().and_then(|m| m.remove(key));
    if let Some(k) = k {
        std::thread::spawn(move || k.stop());
    }
}

/// Every kernel ends with the app.
pub fn stop_all() {
    let all: Vec<Arc<Kernel>> = KERNELS.lock().unwrap().as_mut().map(|m| m.drain().map(|(_, k)| k).collect()).unwrap_or_default();
    for k in all {
        k.stop();
    }
}

/// Text with ANSI colors (SGR) as HTML for Qt rich text: each run in a span,
/// colors as var(--vt-palette-N) for the theme's (or #rrggbb). Unlike a
/// terminal, a background stops at the text: it never fills a line.
pub fn ansi_to_html(text: &str) -> String {
    #[derive(Clone, Default, PartialEq)]
    struct Style {
        fg: Option<String>,
        bg: Option<String>,
        bold: bool,
        dim: bool,
        italic: bool,
        underline: bool,
        inverse: bool,
    }
    fn palette(n: u32) -> String {
        format!("var(--vt-palette-{n})")
    }
    fn open(st: &Style) -> String {
        let (fg, bg) = if st.inverse { (st.bg.clone(), st.fg.clone().or(Some(palette(7)))) } else { (st.fg.clone(), st.bg.clone()) };
        let mut css = String::new();
        if let Some(f) = fg {
            css += &format!("color:{f};");
        }
        if let Some(b) = bg {
            css += &format!("background-color:{b};");
        }
        if st.bold {
            css += "font-weight:bold;";
        }
        if st.italic {
            css += "font-style:italic;";
        }
        if st.underline {
            css += "text-decoration:underline;";
        }
        if st.dim {
            css += "opacity:0.7;";
        }
        if css.is_empty() {
            String::new()
        } else {
            format!("<span style=\"{css}\">")
        }
    }
    let esc = |c: char, out: &mut String| match c {
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '&' => out.push_str("&amp;"),
        '"' => out.push_str("&quot;"),
        '\n' => out.push_str("<br>"),
        '\t' => out.push_str("    "),
        '\r' => {}
        c if (c as u32) < 0x20 => {}
        c => out.push(c),
    };
    let mut out = String::from("<span style=\"white-space:pre-wrap\">");
    let mut st = Style::default();
    let mut opened = false;
    let mut chars = text.chars().peekable();
    // Text pending in the current style (spans are opened lazily, and never
    // across a line break, so a background stays on its line's text).
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    chars.next();
                    let mut params = String::new();
                    let mut fin = ' ';
                    for d in chars.by_ref() {
                        if ('@'..='~').contains(&d) {
                            fin = d;
                            break;
                        }
                        params.push(d);
                    }
                    if fin != 'm' {
                        continue;
                    }
                    let nums: Vec<u32> = if params.is_empty() { vec![0] } else { params.split([';', ':']).map(|p| p.parse().unwrap_or(0)).collect() };
                    let mut i = 0;
                    let mut new = st.clone();
                    while i < nums.len() {
                        let n = nums[i];
                        match n {
                            0 => new = Style::default(),
                            1 => new.bold = true,
                            2 => new.dim = true,
                            3 => new.italic = true,
                            4 => new.underline = true,
                            7 => new.inverse = true,
                            22 => {
                                new.bold = false;
                                new.dim = false
                            }
                            23 => new.italic = false,
                            24 => new.underline = false,
                            27 => new.inverse = false,
                            30..=37 => new.fg = Some(palette(n - 30)),
                            90..=97 => new.fg = Some(palette(n - 90 + 8)),
                            40..=47 => new.bg = Some(palette(n - 40)),
                            100..=107 => new.bg = Some(palette(n - 100 + 8)),
                            39 => new.fg = None,
                            49 => new.bg = None,
                            38 | 48 => {
                                let color = match nums.get(i + 1) {
                                    Some(5) => {
                                        i += 2;
                                        nums.get(i).map(|&k| palette(k))
                                    }
                                    Some(2) => {
                                        let (r, g, b) = (nums.get(i + 2).copied().unwrap_or(0), nums.get(i + 3).copied().unwrap_or(0), nums.get(i + 4).copied().unwrap_or(0));
                                        i += 4;
                                        Some(format!("#{:02x}{:02x}{:02x}", r.min(255), g.min(255), b.min(255)))
                                    }
                                    _ => None,
                                };
                                if n == 38 {
                                    new.fg = color;
                                } else {
                                    new.bg = color;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    if new != st {
                        if opened {
                            out.push_str("</span>");
                            opened = false;
                        }
                        st = new;
                    }
                }
                Some(']') => {
                    // OSC (a title, a link): skipped to its end.
                    chars.next();
                    while let Some(d) = chars.next() {
                        if d == '\x07' || (d == '\x1b' && chars.peek() == Some(&'\\')) {
                            if d == '\x1b' {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        if c == '\n' {
            if opened {
                out.push_str("</span>");
                opened = false;
            }
            out.push_str("<br>");
            continue;
        }
        if !opened {
            let o = open(&st);
            if !o.is_empty() {
                out.push_str(&o);
                opened = true;
            }
        }
        esc(c, &mut out);
    }
    if opened {
        out.push_str("</span>");
    }
    out.push_str("</span>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_and_parsing() {
        let s = Signer { key: b"secret".to_vec(), session: "s".into() };
        let (id, parts) = s.message("execute_request", json!({"code": "1+1"}));
        assert_eq!(parts[0], b"<IDS|MSG>");
        let (ty, parent, c) = s.parse(&parts).unwrap();
        assert_eq!((ty.as_str(), parent.as_str(), c["code"].as_str()), ("execute_request", "", Some("1+1")));
        assert_eq!(id.len(), 32);
        // Tampered: refused.
        let mut bad = parts.clone();
        bad[5] = b"{\"code\":\"2\"}".to_vec();
        assert!(s.parse(&bad).is_none());
        // HMAC-SHA256 of "abc" with key "key" (known value).
        let k = Signer { key: b"key".to_vec(), session: String::new() };
        assert_eq!(k.sign(&[b"a", b"bc"]), "9c196e32dc0175f86f4b1cb89289d6619de6bee699e4c378e68309ed97a1a6ab");
    }

    #[test]
    fn ansi_html() {
        let h = ansi_to_html("a \x1b[31mred\x1b[0m\n\x1b[43mx\x1b[49m <b>");
        assert!(h.contains("<span style=\"color:var(--vt-palette-1);\">red</span>"), "{h}");
        assert!(h.contains("<span style=\"background-color:var(--vt-palette-3);\">x</span>"), "{h}");
        assert!(h.contains("&lt;b&gt;"), "{h}");
        // A background never runs past its line.
        let h = ansi_to_html("\x1b[43mab\ncd\x1b[0m");
        assert_eq!(h.matches("background-color").count(), 2, "{h}");
    }

    #[test]
    fn dates() {
        let d = iso_now();
        assert_eq!(d.len(), 27, "{d}");
        assert!(d.starts_with("20") && d.ends_with('Z'));
    }

    /// A real kernel (Python from Nix): run with --ignored.
    #[test]
    #[ignore]
    fn runs_code_in_a_kernel() {
        let (tx, rx) = std::sync::mpsc::channel::<Value>();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |v| {
            let _ = tx.lock().unwrap().send(v);
        });
        let k = Kernel::start("tessera-nix-python", "/tmp", sink).unwrap();
        let next = |want: &str| loop {
            let v = rx.recv_timeout(Duration::from_secs(120)).expect("kernel event");
            eprintln!("{v}");
            if v["type"] == want || v["state"] == "dead" {
                return v;
            }
        };
        next("info");
        assert!(k.execute("c1", "print('hi'); 6*7"));
        let mut outs = Vec::new();
        loop {
            let v = next("done");
            if v["type"] == "done" {
                assert_eq!(v["status"], "ok");
                break;
            }
        }
        while let Ok(v) = rx.recv_timeout(Duration::from_millis(300)) {
            outs.push(v);
        }
        eprintln!("{outs:?}");
        k.execute("c2", "1/0");
        loop {
            let v = next("done");
            if v["type"] == "done" {
                assert_eq!(v["status"], "error");
                break;
            }
        }
        k.stop();
    }
}
