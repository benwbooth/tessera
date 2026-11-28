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
        Self::connect_as(port, socket_type, wait, b"")
    }

    /// The same, with a DEALER identity: the shell and stdin sockets share
    /// one, so the kernel's input requests come to the stdin socket.
    pub fn connect_as(port: u16, socket_type: &str, wait: Duration, identity: &[u8]) -> std::io::Result<Zmtp> {
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
        // Greeting: signature, version 3.1, NULL mechanism, client, filler.
        let mut g = vec![0xFFu8, 0, 0, 0, 0, 0, 0, 0, 1, 0x7F, 3, 1];
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
            prop(&mut body, "Identity", identity);
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
            // Subscribe to everything: the SUBSCRIBE command (ZMTP 3.1: zmq.rs,
            // in Deno, takes only this) and, for a 3.0 peer, a message
            // starting with 1.
            if peer[11] >= 1 {
                let mut cmd = vec![9u8];
                cmd.extend_from_slice(b"SUBSCRIBE");
                z.write_frame(&cmd, false, true)?;
            } else {
                z.write_frame(&[1], false, false)?;
            }
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
        self.reply(msg_type, content, b"{}")
    }

    /// A message in answer to another (its header as the parent).
    fn reply(&self, msg_type: &str, content: Value, parent_header: &[u8]) -> (String, Vec<Vec<u8>>) {
        let id = random_hex(16);
        let header = json!({"msg_id": id, "username": "tessera", "session": self.session, "date": iso_now(),
                            "msg_type": msg_type, "version": "5.3"})
        .to_string()
        .into_bytes();
        let parent = parent_header.to_vec();
        let meta = b"{}".to_vec();
        let content = content.to_string().into_bytes();
        let sig = self.sign(&[&header, &parent, &meta, &content]);
        (id, vec![b"<IDS|MSG>".to_vec(), sig.into_bytes(), header, parent, meta, content])
    }

    /// A received message's own header (raw), if it's signed right.
    fn header_of(&self, parts: &[Vec<u8>]) -> Option<Vec<u8>> {
        let at = parts.iter().position(|p| p == b"<IDS|MSG>")?;
        let rest = &parts[at + 1..];
        (rest.len() >= 5).then(|| rest[1].clone())
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

/// Where `nix` is: on PATH, else where its installers put it (a shell
/// started over SSH without a login profile may not have it on PATH; it's
/// added to this process's PATH then, for every `nix` run after).
pub fn nix_program() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join("nix")).find(|f| f.is_file())) {
        return Some(p);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let found = [PathBuf::from("/nix/var/nix/profiles/default/bin/nix"), home.join(".nix-profile/bin/nix"), PathBuf::from("/run/current-system/sw/bin/nix")]
        .into_iter()
        .find(|f| f.is_file())?;
    if let Some(dir) = found.parent() {
        let mut paths = vec![dir.to_path_buf()];
        paths.extend(std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default());
        if let Ok(p) = std::env::join_paths(paths) {
            std::env::set_var("PATH", p);
        }
    }
    Some(found)
}

/// Whether starting kernel `name` needs Nix (it comes from nixpkgs).
pub fn needs_nix(name: &str) -> bool {
    specs().iter().find(|s| s.name == name).is_some_and(|s| s.argv.first().is_some_and(|a| a == "nix" || a == "@kernel-json"))
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
    if on_path("nix") {
        for f in FETCHED {
            if !out.iter().any(|s| s.name == f.name) {
                out.push(f.spec());
            }
        }
    }
    // tessh, Tessera's shell language: the one next to Tessera (or on PATH),
    // else from Tessera's flake.
    if !out.iter().any(|s| s.name == "tessh") {
        let argv = match crate::syntax::tessh_program() {
            Some(p) => vec![p.to_string_lossy().to_string(), "--kernel".into(), "-f".into(), CF.into()],
            None if on_path("nix") => ["nix", "shell", "--quiet", "github:benwbooth/tessera#tessh", "-c", "tessh", "--kernel", "-f", CF].map(String::from).to_vec(),
            None => vec![],
        };
        if !argv.is_empty() {
            out.push(Spec { name: "tessh".into(), display: "tessh".into(), language: "tessh".into(), argv, env: HashMap::new(), interrupt_message: false });
        }
    }
    *CACHE.lock().unwrap() = Some((Instant::now(), out.clone()));
    out
}

// ---- kernels Tessera fetches (Nix, uv) ----------------------------------------

/// How a fetched kernel starts.
enum How {
    /// A nixpkgs package's own kernelspec (share/jupyter/kernels/<name>),
    /// read when it starts (the package built then).
    KernelJson { attr: &'static str, kernel: &'static str },
    /// A command in a `nix shell` of these packages.
    Shell { attrs: &'static [&'static str], argv: &'static [&'static str] },
    /// Python from nixpkgs with these packages, `python3 <args>`, in a
    /// `nix shell` with the runtime packages too.
    PyNix { packages: &'static [&'static str], runtime: &'static [&'static str], args: &'static [&'static str] },
    /// A kernel from PyPI through uv (`uv run --with`), the runtime from nixpkgs.
    Uv { pip: &'static [&'static str], runtime: &'static [&'static str], args: &'static [&'static str] },
    /// R with IRkernel.
    R,
}

struct Fetched {
    name: &'static str,
    display: &'static str,
    language: &'static str,
    how: How,
    interrupt_message: bool,
}

const CF: &str = "{connection_file}";

/// One kernel per language, where nixpkgs or PyPI has one (each fetched
/// the first time it starts).
const FETCHED: &[Fetched] = &[
    Fetched { name: "tessera-bash", display: "Bash (bash_kernel, Nix)", language: "bash", how: How::PyNix { packages: &["bash-kernel"], runtime: &["bashInteractive"], args: &["-m", "bash_kernel", "-f", CF] }, interrupt_message: false },
    Fetched { name: "tessera-zsh", display: "Zsh (zsh-jupyter-kernel, uv)", language: "zsh", how: How::Uv { pip: &["zsh-jupyter-kernel"], runtime: &["zsh"], args: &["-m", "zsh_jupyter_kernel", "-f", CF] }, interrupt_message: false },
    Fetched { name: "tessera-powershell", display: "PowerShell (powershell_kernel, uv)", language: "powershell", how: How::Uv { pip: &["powershell_kernel", "ipykernel"], runtime: &["powershell"], args: &["-m", "powershell_kernel", "-f", CF] }, interrupt_message: false },
    Fetched { name: "tessera-xonsh", display: "xonsh (uv)", language: "xonsh", how: How::Uv { pip: &["xonsh", "xontrib-jupyter"], runtime: &[], args: &["-m", "xonsh_jupyter.kernel", "-f", CF] }, interrupt_message: false },
    Fetched { name: "tessera-c", display: "C23 (xeus-cpp, Nix)", language: "c", how: How::KernelJson { attr: "xeus-cpp", kernel: "xc23" }, interrupt_message: false },
    Fetched { name: "tessera-cpp", display: "C++23 (xeus-cpp, Nix)", language: "cpp", how: How::KernelJson { attr: "xeus-cpp", kernel: "xcpp23" }, interrupt_message: false },
    Fetched { name: "tessera-r", display: "R (IRkernel, Nix)", language: "r", how: How::R, interrupt_message: false },
    Fetched { name: "tessera-ruby", display: "Ruby (IRuby, Nix)", language: "ruby", how: How::Shell { attrs: &["iruby"], argv: &["iruby", "kernel", CF] }, interrupt_message: false },
    Fetched { name: "tessera-haskell", display: "Haskell (IHaskell, Nix)", language: "haskell", how: How::Shell { attrs: &["ihaskell"], argv: &["ihaskell", "kernel", CF] }, interrupt_message: false },
    Fetched { name: "tessera-rust", display: "Rust (evcxr, Nix)", language: "rust", how: How::Shell { attrs: &["evcxr", "cargo", "rustc"], argv: &["evcxr_jupyter", "--control_file", CF] }, interrupt_message: false },
    Fetched { name: "tessera-go", display: "Go (gophernotes, Nix)", language: "go", how: How::Shell { attrs: &["gophernotes", "go"], argv: &["gophernotes", CF] }, interrupt_message: false },
    Fetched { name: "tessera-deno", display: "TypeScript/JavaScript (Deno, Nix)", language: "typescript", how: How::Shell { attrs: &["deno"], argv: &["deno", "jupyter", "--kernel", "--conn", CF] }, interrupt_message: false },
    Fetched { name: "tessera-julia", display: "Julia (IJulia, Nix)", language: "julia", how: How::Shell { attrs: &["julia"], argv: &["julia", "-e", "import Pkg; Base.find_package(\"IJulia\") === nothing && Pkg.add(\"IJulia\"); import IJulia; IJulia.run_kernel()", CF] }, interrupt_message: false },
    Fetched { name: "tessera-lua", display: "Lua (ilua, uv)", language: "lua", how: How::Uv { pip: &["ilua", "setuptools"], runtime: &["lua"], args: &["-m", "ilua.app", "-c", CF] }, interrupt_message: true },
    Fetched { name: "tessera-nix", display: "Nix (nix-kernel, Nix)", language: "nix", how: How::PyNix { packages: &["nix-kernel"], runtime: &[], args: &["-m", "nix-kernel", "-f", CF] }, interrupt_message: false },
    Fetched { name: "tessera-octave", display: "Octave (octave_kernel, uv)", language: "octave", how: How::Uv { pip: &["octave_kernel"], runtime: &["octave"], args: &["-m", "octave_kernel", "-f", CF] }, interrupt_message: false },
    Fetched { name: "tessera-kotlin", display: "Kotlin (kotlin-jupyter-kernel, uv)", language: "kotlin", how: How::Uv { pip: &["kotlin-jupyter-kernel"], runtime: &["jdk"], args: &["-m", "run_kotlin_kernel", CF] }, interrupt_message: false },
    Fetched { name: "tessera-scala", display: "Scala (almond, Nix)", language: "scala", how: How::Shell { attrs: &["coursier", "jdk"], argv: &["cs", "launch", "--fork", "almond", "--", "--connection-file", CF] }, interrupt_message: false },
    Fetched { name: "tessera-scheme", display: "Scheme (calysto_scheme, uv)", language: "scheme", how: How::Uv { pip: &["calysto_scheme"], runtime: &[], args: &["-m", "calysto_scheme", "-f", CF] }, interrupt_message: false },
    Fetched { name: "tessera-gnuplot", display: "gnuplot (gnuplot_kernel, uv)", language: "gnuplot", how: How::Uv { pip: &["gnuplot_kernel"], runtime: &["gnuplot"], args: &["-m", "gnuplot_kernel", "-f", CF] }, interrupt_message: false },
];

/// Hand changes to a shell (pid): its folder (cd), exports set and unset,
/// applied there before its next command and prompt (shell/tessera.*), and
/// its environment as the kernels see it updated now.
pub fn write_apply(pid: u32, set: Vec<(String, String)>, unset: Vec<String>, cd: Option<String>) {
    let dir = env_dir();
    // In the shell's own words: nushell and xonsh read JSON (merged with any
    // not applied yet), PowerShell its own, fish its own, the rest POSIX.
    let shell = crate::procinfo::shell_name(pid);
    let name_ok = |a: &str| !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let set: Vec<(String, String)> = set.into_iter().filter(|(a, _)| name_ok(a)).collect();
    let unset: Vec<String> = unset.into_iter().filter(|a| name_ok(a)).collect();
    let apply = dir.join(format!("{pid}.apply"));
    if let Some(d) = &cd {
        let _ = std::fs::write(dir.join(format!("{pid}.cwd")), d);
    }
    if matches!(shell.as_str(), "nu" | "xonsh" | "tessh") {
        let mut a: Value = std::fs::read_to_string(&apply).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| json!({"set": {}, "unset": []}));
        for (k, v) in &set {
            a["set"][k] = json!(v);
            if let Some(u) = a["unset"].as_array_mut() {
                u.retain(|x| x.as_str() != Some(k));
            }
        }
        for k in &unset {
            if let Some(m) = a["set"].as_object_mut() {
                m.remove(k);
            }
            if let Some(u) = a["unset"].as_array_mut() {
                u.push(json!(k));
            }
        }
        if let Some(d) = &cd {
            a["cd"] = json!(d);
        }
        let _ = std::fs::write(&apply, a.to_string());
    } else {
        let ps = |s: &str| format!("'{}'", s.replace('\'', "''"));
        let mut lines = Vec::new();
        for (a, b) in &set {
            lines.push(match shell.as_str() {
                "fish" => format!("set -gx {a} {}", sh_quote(b)),
                "pwsh" => format!("$env:{a} = {}", ps(b)),
                "tcsh" | "csh" => format!("setenv {a} {}", sh_quote(b)),
                "elvish" => format!("set-env {a} {}", sh_quote(b)),
                _ => format!("export {a}={}", sh_quote(b)),
            });
        }
        for a in &unset {
            lines.push(match shell.as_str() {
                "fish" => format!("set -e {a}"),
                "pwsh" => format!("Remove-Item -ErrorAction SilentlyContinue env:{a}"),
                "tcsh" | "csh" => format!("unsetenv {a}"),
                "elvish" => format!("unset-env {a}"),
                _ => format!("unset {a}"),
            });
        }
        if let Some(d) = &cd {
            lines.push(match shell.as_str() {
                "fish" => format!("builtin cd {}", sh_quote(d)),
                "pwsh" => format!("Set-Location -LiteralPath {}", ps(d)),
                "tcsh" | "csh" | "elvish" => format!("cd {}", sh_quote(d)),
                _ => format!("builtin cd -- {}", sh_quote(d)),
            });
        }
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&apply) {
            let _ = writeln!(f, "{}", lines.join("\n"));
        }
    }
    // (The shell's exported environment as the kernels will see it, until
    // its next prompt writes it again.)
    if let Some(mut env) = crate::prompt::shell_env(pid).map(|e| e.into_iter().collect::<HashMap<String, String>>()) {
        for (a, b) in set {
            env.insert(a, b);
        }
        for a in unset {
            env.remove(&a);
        }
        let mut out = Vec::new();
        for (a, b) in env {
            out.extend_from_slice(format!("{a}={b}").as_bytes());
            out.push(0);
        }
        let _ = std::fs::write(dir.join(pid.to_string()), out);
    }
}


/// The program of a `nix shell ... -c prog` that's been built and kept
/// already (nix_cached), without building it.
pub fn nix_kept(argv: &[String]) -> Option<String> {
    let c = argv.iter().position(|a| a == "-c")?;
    let what: Vec<String> = argv[2..c].to_vec();
    let prog = argv.get(c + 1)?;
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    what.hash(&mut h);
    let list = PathBuf::from(std::env::var_os("HOME")?).join(".cache/tessera/kernel-envs").join(format!("{:016x}.paths", h.finish()));
    let t = std::fs::read_to_string(list).ok()?;
    t.lines().map(|p| format!("{p}/bin/{prog}")).find(|e| Path::new(e).exists())
}

/// A `nix shell <installables> -c prog args` command as prog run from the
/// built environment: its bin folders for PATH. Built (and remembered) the
/// first time; after that it starts at once, and is built again in the
/// background once a week, for updates.
pub fn nix_cached(argv: &[String], status: &dyn Fn(&str)) -> Option<(Vec<String>, String)> {
    if argv.len() < 4 || argv[0] != "nix" || argv[1] != "shell" {
        return None;
    }
    let c = argv.iter().position(|a| a == "-c")?;
    let what: Vec<String> = argv[2..c].to_vec();
    let rest = argv[c + 1..].to_vec();
    let prog = rest.first()?.clone();
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    what.hash(&mut h);
    let dir = PathBuf::from(std::env::var_os("HOME")?).join(".cache/tessera/kernel-envs");
    let _ = std::fs::create_dir_all(&dir);
    let list = dir.join(format!("{:016x}.paths", h.finish()));
    let build = |what: &[String], list: &Path| -> bool {
        let mut cmd = Command::new("nix");
        cmd.arg("build").arg("--no-link").arg("--print-out-paths");
        cmd.args(what.iter().filter(|a| *a != "--quiet"));
        let Ok(out) = cmd.stdin(Stdio::null()).stderr(Stdio::null()).output() else { return false };
        let paths = String::from_utf8_lossy(&out.stdout).to_string();
        if !out.status.success() || paths.trim().is_empty() {
            return false;
        }
        // (Kept from garbage collection while in use: a root per path.)
        for (i, p) in paths.lines().enumerate() {
            let root = list.with_extension(format!("root{i}"));
            let _ = std::fs::remove_file(&root);
            let _ = Command::new("nix-store").args(["--add-root"]).arg(&root).args(["--realise", p]).stdout(Stdio::null()).stderr(Stdio::null()).status();
        }
        std::fs::write(list, paths).is_ok()
    };
    let read = |list: &Path| -> Option<Vec<String>> {
        let t = std::fs::read_to_string(list).ok()?;
        let v: Vec<String> = t.lines().filter(|l| !l.is_empty()).map(String::from).collect();
        (!v.is_empty() && v.iter().all(|p| Path::new(p).exists())).then_some(v)
    };
    let paths = match read(&list) {
        Some(v) => {
            let old = std::fs::metadata(&list).and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|a| a > Duration::from_secs(7 * 86400));
            if old {
                let (what, list) = (what.clone(), list.clone());
                std::thread::spawn(move || {
                    let _ = build(&what, &list);
                });
            }
            v
        }
        None => {
            status("Getting it with Nix (the first time takes a while)…");
            if !build(&what, &list) {
                return None;
            }
            read(&list)?
        }
    };
    let bins: Vec<String> = paths.iter().map(|p| format!("{p}/bin")).filter(|b| Path::new(b).is_dir()).collect();
    let exe = bins.iter().map(|b| format!("{b}/{prog}")).find(|e| Path::new(e).exists()).unwrap_or(prog);
    let mut argv = vec![exe];
    argv.extend(rest.into_iter().skip(1));
    Some((argv, bins.join(":")))
}

fn nix_shell(attrs: &[&str]) -> Vec<String> {
    let mut v = vec!["nix".to_string(), "shell".into(), "--quiet".into()];
    v.extend(attrs.iter().map(|a| format!("nixpkgs#{a}")));
    v.push("-c".into());
    v
}

impl Fetched {
    fn spec(&self) -> Spec {
        let s = |a: &[&str]| a.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let argv = match &self.how {
            // (Resolved when it starts: see resolve_argv.)
            How::KernelJson { attr, kernel } => vec!["@kernel-json".to_string(), attr.to_string(), kernel.to_string()],
            How::Shell { attrs, argv } => [nix_shell(attrs), s(argv)].concat(),
            How::PyNix { packages, runtime, args } => {
                // (One environment: --expr can't be mixed with nixpkgs#... in nix shell.)
                let expr = format!(
                    "with (builtins.getFlake \"nixpkgs\").legacyPackages.${{builtins.currentSystem}}; buildEnv {{ name = \"tessera-kernel\"; paths = [ (python3.withPackages (p: with p; [ {} ])) {} ]; }}",
                    packages.join(" "),
                    runtime.join(" ")
                );
                let mut v = vec!["nix".to_string(), "shell".into(), "--quiet".into(), "--impure".into(), "--expr".into(), expr];
                v.push("-c".into());
                v.push("python3".into());
                [v, s(args)].concat()
            }
            How::Uv { pip, runtime, args } => {
                let mut v = nix_shell(&[&["uv"][..], runtime].concat());
                // (3.12: some kernels' packages don't build on newer Pythons yet.)
                v.extend(["uv", "run", "--quiet", "--no-project", "--python", "3.12"].map(String::from));
                for p in pip.iter() {
                    v.push("--with".into());
                    v.push(p.to_string());
                }
                v.push("python".into());
                [v, s(args)].concat()
            }
            How::R => {
                let expr = "with (builtins.getFlake \"nixpkgs\").legacyPackages.${builtins.currentSystem}; rWrapper.override { packages = [ rPackages.IRkernel ]; }".to_string();
                let mut v = vec!["nix".to_string(), "shell".into(), "--quiet".into(), "--impure".into(), "--expr".into(), expr, "-c".into()];
                v.extend(s(&["R", "--slave", "-e", "IRkernel::main()", "--args", CF]));
                v
            }
        };
        // (A shell kernel's history isn't yours: it mustn't go in ~/.bash_history.)
        let mut env = HashMap::new();
        if matches!(self.language, "bash" | "zsh" | "xonsh" | "powershell") {
            env.insert("HISTFILE".to_string(), "/dev/null".to_string());
        }
        Spec { name: self.name.into(), display: self.display.into(), language: self.language.into(), argv, env, interrupt_message: self.interrupt_message }
    }
}

/// A kernelspec inside a nixpkgs package: built now (the first time), its
/// kernel.json read. (argv, env)
fn resolve_kernel_json(attr: &str, kernel: &str) -> Result<(Vec<String>, HashMap<String, String>), String> {
    let out = Command::new("nix").args(["build", "--quiet", "--no-link", "--print-out-paths", &format!("nixpkgs#{attr}")]).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!("Couldn't get {attr} from nixpkgs: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")));
    }
    for dir in String::from_utf8_lossy(&out.stdout).lines() {
        let p = Path::new(dir).join("share/jupyter/kernels").join(kernel);
        if let Some(spec) = read_spec(kernel, &p) {
            return Ok((spec.argv, spec.env));
        }
    }
    Err(format!("{attr} has no kernel named {kernel}"))
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
    stdin: Mutex<Option<Zmtp>>,
    /// Requests answered, oldest first (forgotten after 200).
    answered: Mutex<std::collections::VecDeque<String>>,
    /// The cell its last run was for (its own stdout goes there).
    running: Arc<Mutex<String>>,
    /// The header of the input request being answered.
    asking: Mutex<Option<Vec<u8>>>,
    /// That request as the notebook was told it: told again to a window
    /// that comes back while it's unanswered (a tessh program running in
    /// the terminal waits on it; the window it was for may be gone).
    asked: Mutex<Option<Value>>,
    /// Where its events go (to say it was stopped).
    sink: Sink,
    /// busy / idle, as it last said, and since when; when it started.
    state: Mutex<String>,
    state_at: Mutex<Instant>,
    /// Cells sent that it hasn't said a word about yet: message id ->
    /// (cell, code, when sent or last woken, wakes). (ipykernel can sit on a
    /// request that comes while it's finishing starting up, idle, until
    /// something else wakes it: the card waited for good. See
    /// wake_if_stuck.)
    unacked: Arc<Mutex<HashMap<String, (String, String, Instant, u32)>>>,
    /// Its ports (shell, iopub, stdin, control, hb).
    ports: Vec<u16>,
    started: Instant,
    /// The environment and folder it has (as started, then as synced).
    synced: Mutex<(HashMap<String, String>, String)>,
    /// A shell kernel's own environment and folder, as last looked at (just
    /// before a cell: what the cell changes is told from it), and whether a
    /// look is under way.
    seen: Arc<Mutex<Option<(HashMap<String, String>, String)>>>,
    looking: Arc<std::sync::atomic::AtomicBool>,
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
    pub fn start(spec_name: &str, cwd: &str, shell_env: Option<Vec<(String, String)>>, sink: Sink) -> Result<Arc<Kernel>, String> {
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
        let (spec_argv, spec_env) = if spec.argv.first().map(String::as_str) == Some("@kernel-json") {
            sink(json!({"type": "status", "state": "starting", "message": format!("Getting {} (the first time takes a while)…", spec.display)}));
            let (a, mut e) = resolve_kernel_json(&spec.argv[1], &spec.argv[2])?;
            e.extend(spec.env.clone());
            (a, e)
        } else {
            (spec.argv.clone(), spec.env.clone())
        };
        let argv: Vec<String> = spec_argv.iter().map(|a| a.replace("{connection_file}", &conn_file.to_string_lossy())).collect();
        // (`nix shell ... -c prog`: the environment built once and kept, the
        // program run from it straight away.)
        let (argv, nix_path) = match nix_cached(&argv, &|m: &str| sink(json!({"type": "status", "state": "starting", "message": m}))) {
            Some((a, p)) => (a, Some(p)),
            None => (argv, None),
        };
        let mut cmd = Command::new(&argv[0]);
        // (The shell's environment, when the pane gave one: see start.)
        if let Some(env) = &shell_env {
            cmd.env_clear().envs(env.iter().cloned());
        }
        let mut spec_env = spec_env;
        if let Some(p) = nix_path {
            let base = shell_env.as_ref().and_then(|e| e.iter().find(|(k, _)| k == "PATH").map(|(_, v)| v.clone())).or_else(|| std::env::var("PATH").ok()).unwrap_or_default();
            spec_env.insert("PATH".into(), format!("{p}:{base}"));
        }
        cmd.args(&argv[1..]).envs(&spec_env).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped());
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
        let running: Arc<Mutex<String>> = Arc::default();
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
            stdin: Mutex::new(None),
            answered: Mutex::default(),
            sink: sink.clone(),
            state: Mutex::new("starting".into()),
            state_at: Mutex::new(Instant::now()),
            unacked: Arc::default(),
            ports: ports.clone(),
            started: Instant::now(),
            asking: Mutex::new(None),
            asked: Mutex::new(None),
            running,
            child: Mutex::new(Some(child)),
            pending: Arc::default(),
            spec,
            conn_file,
            gone: Arc::default(),
            ready: AtomicBool::new(false),
            seen: Arc::new(Mutex::new(None)),
            looking: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            synced: Mutex::new((shell_env.clone().map(|e| e.into_iter().collect()).unwrap_or_else(|| std::env::vars().collect()), cwd.to_string())),
        });
        // Connect (Nix may take minutes to fetch Python the first time), then
        // ask who it is; its reply means it's ready.
        let k = kernel.clone();
        std::thread::spawn(move || {
            // (Fetched ones download and install the first time.)
            let wait = Duration::from_secs(if k.spec.name.starts_with("tessera-") { 1800 } else { 120 });
            let start = Instant::now();
            let identity = format!("tessera-{}", random_hex(8)).into_bytes();
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
                    match Zmtp::connect_as(port, ty, Duration::from_millis(500), &identity) {
                        Ok(z) => return Ok(z),
                        Err(_) if start.elapsed() < wait => continue,
                        Err(e) => return Err(format!("Couldn't reach the kernel: {e}")),
                    }
                }
            };
            let result = (|| -> Result<(), String> {
                let dbg = std::env::var_os("TESSERA_KERNEL_DEBUG").is_some();
                let iopub = connect(ports[1], "SUB")?;
                if dbg {
                    eprintln!("{}: iopub connected", k.spec.name);
                }
                let shell = connect(ports[0], "DEALER")?;
                if dbg {
                    eprintln!("{}: shell connected", k.spec.name);
                }
                // (stdin is optional too: IHaskell has none; no input() then.)
                let stdin = Zmtp::connect_as(ports[2], "DEALER", Duration::from_secs(3), &identity).ok();
                // (Control is optional: IRuby has none. A few seconds once
                // the rest are up; interrupt falls back to a signal.)
                let control = Zmtp::connect_as(ports[3], "DEALER", Duration::from_secs(3), &identity).ok();
                if dbg {
                    eprintln!("{}: all connected", k.spec.name);
                }
                let shell_reader = shell.try_clone().map_err(|e| e.to_string())?;
                let stdin_reader = stdin.as_ref().and_then(|s| s.try_clone().ok());
                *k.shell.lock().unwrap() = Some(shell);
                *k.control.lock().unwrap() = control;
                *k.stdin.lock().unwrap() = stdin;
                k.spawn_readers(iopub, shell_reader, sink.clone());
                if let Some(r) = stdin_reader {
                    k.spawn_stdin_reader(r, sink.clone());
                }
                // (ilua crashes on a request that comes before its Lua is
                // connected, kernel_info too: give it time first.)
                if k.spec.name == "tessera-lua" {
                    std::thread::sleep(Duration::from_secs(3));
                }
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

    /// The kernel asking for input (Python's input(), R's readline, ...):
    /// {"type": "input", cell, prompt, password}; input_reply answers.
    fn spawn_stdin_reader(self: &Arc<Self>, mut stdin: Zmtp, sink: Sink) {
        let k = self.clone();
        std::thread::spawn(move || {
            while let Ok(parts) = stdin.recv() {
                let Some((ty, parent, c)) = k.signer.parse(&parts) else { continue };
                if ty != "input_request" {
                    continue;
                }
                *k.asking.lock().unwrap() = k.signer.header_of(&parts);
                let cell = k.pending.lock().unwrap().get(&parent).cloned().unwrap_or_default();
                let e = json!({"type": "input", "cell": cell, "prompt": c["prompt"], "password": c["password"].as_bool().unwrap_or(false), "tty": c["tessera_tty"]});
                *k.asked.lock().unwrap() = Some(e.clone());
                sink(e);
            }
        });
    }

    /// Answer the kernel's input request.
    pub fn input_reply(&self, value: &str) -> bool {
        let Some(parent) = self.asking.lock().unwrap().take() else { return false };
        *self.asked.lock().unwrap() = None;
        let (_, parts) = self.signer.reply("input_reply", json!({"value": value}), &parent);
        self.stdin.lock().unwrap().as_mut().is_some_and(|s| s.send(&parts).is_ok())
    }

    fn spawn_readers(self: &Arc<Self>, mut iopub: Zmtp, mut shell: Zmtp, sink: Sink) {
        let k = self.clone();
        std::thread::spawn(move || {
            while !k.gone.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(500));
                k.wake_if_stuck();
            }
        });
        let k = self.clone();
        let s2 = sink.clone();
        std::thread::spawn(move || {
            while let Ok(parts) = iopub.recv() {
                if std::env::var_os("TESSERA_KERNEL_DEBUG").is_some() {
                    eprintln!("iopub {} parts: {:?}", parts.len(), parts.iter().map(|p| String::from_utf8_lossy(&p[..p.len().min(80)]).to_string()).collect::<Vec<_>>());
                }
                let Some((ty, parent, c)) = k.signer.parse(&parts) else {
                    if std::env::var_os("TESSERA_KERNEL_DEBUG").is_some() {
                        let at = parts.iter().position(|p| p == b"<IDS|MSG>").unwrap_or(0);
                        let r = &parts[at + 1..];
                        if r.len() >= 5 {
                            eprintln!("iopub refused: sig {} want {} header {}", String::from_utf8_lossy(&r[0]), k.signer.sign(&[&r[1], &r[2], &r[3], &r[4]]), String::from_utf8_lossy(&r[1]));
                        }
                    }
                    continue;
                };
                let cell = k.pending.lock().unwrap().get(&parent).cloned();
                // (Heard about: it has it.)
                if !parent.is_empty() {
                    k.unacked.lock().unwrap().remove(&parent);
                }
                match ty.as_str() {
                    "status" => {
                        let st = c["execution_state"].as_str().unwrap_or("");
                        if st == "idle" || st == "busy" {
                            *k.state.lock().unwrap() = st.to_string();
                            *k.state_at.lock().unwrap() = Instant::now();
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
                    // (tessh: a program a cell runs and what with.)
                    "tessera_exec" => {
                        if let Some(cell) = cell {
                            s2(json!({"type": "exec", "cell": cell, "exec": c}));
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
        self.spawn_shell_reader(shell, sink);
    }

    /// Replies on the shell connection (kernel_info, execute, complete).
    fn spawn_shell_reader(self: &Arc<Self>, mut shell: Zmtp, sink: Sink) {
        let k = self.clone();
        std::thread::spawn(move || {
            while let Ok(parts) = shell.recv() {
                if std::env::var_os("TESSERA_KERNEL_DEBUG").is_some() {
                    eprintln!("shell {} parts: {:?}", parts.len(), parts.iter().map(|p| String::from_utf8_lossy(&p[..p.len().min(100)]).to_string()).collect::<Vec<_>>());
                }
                let Some((ty, parent, c)) = k.signer.parse(&parts) else { continue };
                // (Kept a while after the reply: some kernels, Deno's, send
                // output after it.)
                let Some(what) = k.pending.lock().unwrap().get(&parent).cloned() else { continue };
                {
                    let mut done = k.answered.lock().unwrap();
                    done.push_back(parent.clone());
                    while done.len() > 200 {
                        if let Some(old) = done.pop_front() {
                            k.pending.lock().unwrap().remove(&old);
                        }
                    }
                }
                match ty.as_str() {
                    "kernel_info_reply" => {
                        if !k.ready.swap(true, Ordering::SeqCst) {
                            let li = &c["language_info"];
                            sink(json!({"type": "info", "language": li["name"], "version": li["version"],
                                        "extension": li["file_extension"], "banner": c["banner"], "implementation": c["implementation"]}));
                            // (C and C++: the standard library there from the start.)
                            if let Some(p) = preamble(&k.spec.language) {
                                k.execute_quiet(p);
                            }
                            sink(json!({"type": "status", "state": "idle"}));
                        }
                    }
                    "execute_reply" => {
                        let status = c["status"].as_str().unwrap_or("ok");
                        // (Skipped: queued behind a cell that failed.)
                        if status == "aborted" {
                            sink(json!({"type": "output", "cell": what, "output": {"output_type": "error", "ename": "Not run", "evalue": "", "traceback": ["Not run: a cell before it failed."]}}));
                        }
                        // (tessh: the last program's exit status, for the card.)
                        // (tessh: also the folder the cell started in.)
                        sink(json!({"type": "done", "cell": what, "status": status, "count": c["execution_count"],
                                    "exit": c["tessera_exit"].as_i64().unwrap_or(0), "cwd": c["tessera_cwd"].as_str().unwrap_or("")}));
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
        self.send_shell_id(msg_type, content, tag).is_some()
    }

    /// As send_shell; the message's id when it went.
    fn send_shell_id(&self, msg_type: &str, content: Value, tag: &str) -> Option<String> {
        let (id, parts) = self.signer.message(msg_type, content);
        self.pending.lock().unwrap().insert(id.clone(), tag.to_string());
        let mut g = self.shell.lock().unwrap();
        g.as_mut().is_some_and(|s| s.send(&parts).is_ok()).then_some(id)
    }

    fn send_execute(&self, cell: &str, code: &str) -> bool {
        let id = self.send_shell_id(
            "execute_request",
            // (tessera_tty: this side runs a tessh kernel's programs in the pane's terminal.)
            json!({"code": code, "silent": false, "store_history": true, "user_expressions": {}, "allow_stdin": true, "stop_on_error": true, "tessera_tty": true}),
            cell,
        );
        if let Some(id) = &id {
            self.unacked.lock().unwrap().insert(id.clone(), (cell.to_string(), code.to_string(), Instant::now(), 0));
        }
        id.is_some()
    }

    /// Cells it's sat on (see `unacked`): once it has been idle 2 s with one
    /// unanswered, it's woken by a connection of another (ipykernel picks
    /// up the waiting request then; the answers come back as usual), up to
    /// three times; still nothing, the cell's given up on.
    fn wake_if_stuck(&self) {
        let idle_for = { if *self.state.lock().unwrap() == "idle" { self.state_at.lock().unwrap().elapsed() } else { Duration::ZERO } };
        if idle_for < Duration::from_secs(2) {
            return;
        }
        let mut give_up = Vec::new();
        let mut wake = false;
        {
            let mut u = self.unacked.lock().unwrap();
            for (id, v) in u.iter_mut() {
                if v.2.elapsed() < Duration::from_secs(2) {
                    continue;
                }
                if v.3 >= 3 {
                    give_up.push((id.clone(), v.0.clone()));
                } else {
                    v.3 += 1;
                    v.2 = Instant::now();
                    wake = true;
                }
            }
            for (id, _) in &give_up {
                u.remove(id);
            }
        }
        if wake {
            if std::env::var_os("TESSERA_KERNEL_DEBUG").is_some() {
                eprintln!("{}: a request unanswered while idle: woken", self.spec.name);
            }
            let identity = format!("tessera-wake-{}", random_hex(6)).into_bytes();
            if let Ok(mut z) = Zmtp::connect_as(self.ports[0], "DEALER", Duration::from_secs(2), &identity) {
                let (_, parts) = self.signer.message("kernel_info_request", json!({}));
                let _ = z.send(&parts);
                std::thread::sleep(Duration::from_millis(300));
                z.shutdown();
            }
        }
        for (_, cell) in give_up {
            (self.sink)(json!({"type": "output", "cell": cell, "output": {"output_type": "error", "ename": "No answer", "evalue": "", "traceback": ["The kernel never started this: run it again, or restart the kernel."]}}));
            (self.sink)(json!({"type": "done", "cell": cell, "status": "error", "count": Value::Null, "exit": 0}));
        }
    }

    pub fn execute(&self, cell: &str, code: &str) -> bool {
        *self.running.lock().unwrap() = cell.to_string();
        // (import std; in C++: the library's all there already (the
        // preamble); an interpreter can't import modules.)
        let cleaned;
        let code = if is_cpp(&self.spec.language) && code.lines().any(|l| l.trim_start().starts_with("import std")) {
            cleaned = code.lines().map(|l| if l.trim_start().starts_with("import std") { "" } else { l }).collect::<Vec<_>>().join("\n");
            cleaned.as_str()
        } else {
            code
        };
        self.send_execute(cell, code)
    }

    /// Code run quietly (no output, no count): the sync below.
    pub fn execute_quiet(&self, code: &str) -> bool {
        // (Not "silent": xeus-cpp dies on that. Output for no cell is dropped.)
        self.send_shell("execute_request", json!({"code": code, "silent": false, "store_history": false, "user_expressions": {}, "allow_stdin": false, "stop_on_error": false}), "")
    }

    /// The shell's environment and folder (now) into the kernel, as its
    /// own: what changed since it last had them, set in its language.
    pub fn sync(&self, env: &HashMap<String, String>, cwd: &str) -> bool {
        let mut had = self.synced.lock().unwrap();
        let set: Vec<(&String, &String)> = env.iter().filter(|(k, v)| had.0.get(*k) != Some(v) && !SKIP_ENV.contains(&k.as_str())).collect();
        let unset: Vec<&String> = had.0.keys().filter(|k| !env.contains_key(*k) && !SKIP_ENV.contains(&k.as_str())).collect();
        let cd = (!cwd.is_empty() && cwd != had.1 && Path::new(cwd).is_dir()).then_some(cwd);
        if set.is_empty() && unset.is_empty() && cd.is_none() {
            return false;
        }
        let Some(code) = sync_code(&self.spec.language, &set, &unset, cd) else { return false };
        *had = (env.clone(), cwd.to_string());
        drop(had);
        self.execute_quiet(&code)
    }

    pub fn complete(&self, request: &str, code: &str, pos: usize) -> bool {
        self.send_shell("complete_request", json!({"code": code, "cursor_pos": pos}), request)
    }

    pub fn interrupt(&self) {
        if self.spec.interrupt_message {
            let (_, parts) = self.signer.message("interrupt_request", json!({}));
            if let Some(c) = self.control.lock().unwrap().as_mut() {
                let _ = c.send(&parts);
                return;
            }
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

fn is_cpp(language: &str) -> bool {
    matches!(language.to_ascii_lowercase().as_str(), "c++" | "cpp" | "cxx")
}

/// What a kernel runs (quietly) as it comes up: for C++ the whole standard
/// library (as import std; would give), for C its usual headers.
fn preamble(language: &str) -> Option<&'static str> {
    if is_cpp(language) {
        return Some("#if __has_include(<bits/stdc++.h>)\n#include <bits/stdc++.h>\n#else\n#include <algorithm>\n#include <chrono>\n#include <cmath>\n#include <cstdio>\n#include <cstdlib>\n#include <format>\n#include <fstream>\n#include <functional>\n#include <iostream>\n#include <map>\n#include <memory>\n#include <numeric>\n#include <optional>\n#include <print>\n#include <ranges>\n#include <set>\n#include <sstream>\n#include <string>\n#include <string_view>\n#include <unordered_map>\n#include <unordered_set>\n#include <utility>\n#include <vector>\n#endif");
    }
    if language.eq_ignore_ascii_case("c") {
        return Some("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <math.h>\n#include <stdbool.h>\n#include <stdint.h>\n#include <ctype.h>\n#include <time.h>");
    }
    None
}

/// Variables a kernel keeps its own (the shell's would confuse it).
const SKIP_ENV: &[&str] = &["PS1", "PS2", "PROMPT", "RPROMPT", "SHLVL", "_", "OLDPWD", "PWD", "COLUMNS", "LINES", "TERM", "JPY_PARENT_PID", "TESSERA_SHELL_INTEGRATION"];

fn c_str(s: &str) -> String {
    let mut o = String::from("\"");
    for b in s.bytes() {
        match b {
            b'"' => o.push_str("\\\""),
            b'\\' => o.push_str("\\\\"),
            0x20..=0x7e => o.push(b as char),
            _ => o.push_str(&format!("\\{b:03o}")),
        }
    }
    o.push('"');
    o
}

fn hs_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => o.push_str(&format!("\\{}\\&", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn sq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Setting variables, clearing them and changing folder, in `language`
/// (None: it can't, so it keeps what it started with).
fn sync_code(language: &str, set: &[(&String, &String)], unset: &[&String], cd: Option<&str>) -> Option<String> {
    let js = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let mut out: Vec<String> = Vec::new();
    match language.to_ascii_lowercase().as_str() {
        "python" | "xonsh" => {
            out.push("import os as _tessera_os".into());
            if !set.is_empty() {
                let m: serde_json::Map<String, Value> = set.iter().map(|(k, v)| (k.to_string(), json!(v))).collect();
                out.push(format!("_tessera_os.environ.update({})", Value::Object(m)));
            }
            for k in unset {
                out.push(format!("_tessera_os.environ.pop({}, None)", js(k)));
            }
            if let Some(d) = cd {
                out.push(format!("_tessera_os.chdir({})", js(d)));
            }
            out.push("del _tessera_os".into());
        }
        "tessh" => {
            // (Base64 JSON, applied by tessh's runtime: no quoting to get wrong.)
            use base64::Engine;
            let m: serde_json::Map<String, Value> = set.iter().map(|(k, v)| (k.to_string(), json!(v))).collect();
            let a = json!({"set": m, "unset": unset, "cd": cd});
            out.push(format!("__tessh__.apply_env '{}'", base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(a.to_string())));
        }
        "r" => {
            for (k, v) in set {
                out.push(format!("Sys.setenv({} = {})", js(k), js(v)));
            }
            if !unset.is_empty() {
                out.push(format!("Sys.unsetenv(c({}))", unset.iter().map(|k| js(k)).collect::<Vec<_>>().join(", ")));
            }
            if let Some(d) = cd {
                out.push(format!("setwd({})", js(d)));
            }
            out.push("invisible(NULL)".into());
        }
        "julia" => {
            let jl = |s: &str| js(s).replace('$', "\\$");
            for (k, v) in set {
                out.push(format!("ENV[{}] = {}", jl(k), jl(v)));
            }
            for k in unset {
                out.push(format!("delete!(ENV, {})", jl(k)));
            }
            if let Some(d) = cd {
                out.push(format!("cd({})", jl(d)));
            }
            out.push("nothing".into());
        }
        "ruby" => {
            let rb = |s: &str| js(s).replace('#', "\\#");
            for (k, v) in set {
                out.push(format!("ENV[{}] = {}", rb(k), rb(v)));
            }
            for k in unset {
                out.push(format!("ENV.delete({})", rb(k)));
            }
            if let Some(d) = cd {
                out.push(format!("Dir.chdir({})", rb(d)));
            }
            out.push("nil".into());
        }
        "bash" | "zsh" | "sh" => {
            for (k, v) in set {
                out.push(format!("export {k}={}", sq(v)));
            }
            for k in unset {
                out.push(format!("unset {k}"));
            }
            if let Some(d) = cd {
                out.push(format!("cd {}", sq(d)));
            }
        }
        "powershell" => {
            let ps = |s: &str| format!("'{}'", s.replace('\'', "''"));
            for (k, v) in set {
                out.push(format!("${{env:{k}}} = {}", ps(v)));
            }
            for k in unset {
                out.push(format!("Remove-Item -ErrorAction SilentlyContinue Env:{k}"));
            }
            if let Some(d) = cd {
                out.push(format!("Set-Location -LiteralPath {}", ps(d)));
            }
        }
        "typescript" | "javascript" => {
            for (k, v) in set {
                out.push(format!("Deno.env.set({}, {});", js(k), js(v)));
            }
            for k in unset {
                out.push(format!("Deno.env.delete({});", js(k)));
            }
            if let Some(d) = cd {
                out.push(format!("Deno.chdir({});", js(d)));
            }
        }
        "rust" => {
            for (k, v) in set {
                out.push(format!("std::env::set_var({k:?}, {v:?});"));
            }
            for k in unset {
                out.push(format!("std::env::remove_var({k:?});"));
            }
            if let Some(d) = cd {
                out.push(format!("std::env::set_current_dir({d:?}).ok();"));
            }
        }
        "go" => {
            out.push("import \"os\"".into());
            for (k, v) in set {
                out.push(format!("os.Setenv({}, {})", js(k), js(v)));
            }
            for k in unset {
                out.push(format!("os.Unsetenv({})", js(k)));
            }
            if let Some(d) = cd {
                out.push(format!("os.Chdir({})", js(d)));
            }
        }
        "haskell" => {
            out.push("import qualified System.Environment as TesseraEnv".into());
            out.push("import qualified System.Directory as TesseraDir".into());
            for (k, v) in set {
                out.push(format!("TesseraEnv.setEnv {} {}", hs_str(k), hs_str(v)));
            }
            for k in unset {
                out.push(format!("TesseraEnv.unsetEnv {}", hs_str(k)));
            }
            if let Some(d) = cd {
                out.push(format!("TesseraDir.setCurrentDirectory {}", hs_str(d)));
            }
        }
        "c" | "c++" | "cpp" => {
            out.push("#include <stdlib.h>".into());
            out.push("#include <unistd.h>".into());
            for (k, v) in set {
                out.push(format!("setenv({}, {}, 1);", c_str(k), c_str(v)));
            }
            for k in unset {
                out.push(format!("unsetenv({});", c_str(k)));
            }
            if let Some(d) = cd {
                out.push(format!("(void)chdir({});", c_str(d)));
            }
        }
        "octave" | "matlab" => {
            let m = |s: &str| format!("'{}'", s.replace('\'', "''"));
            for (k, v) in set {
                out.push(format!("setenv({}, {});", m(k), m(v)));
            }
            for k in unset {
                out.push(format!("unsetenv({});", m(k)));
            }
            if let Some(d) = cd {
                out.push(format!("cd({});", m(d)));
            }
        }
        _ => return None,
    }
    Some(out.join("\n"))
}

// ---- the running kernels, by the notebook's key ----------------------------

static KERNELS: Mutex<Option<HashMap<String, Arc<Kernel>>>> = Mutex::new(None);

/// Start a kernel for `key` in `cwd`, with a shell's environment (its pid;
/// 0: Tessera's own).
pub fn start(key: &str, spec: &str, cwd: &str, pid: u32, sink: Sink) {
    stop(key);
    let (key, spec, cwd) = (key.to_string(), spec.to_string(), cwd.to_string());
    std::thread::spawn(move || match Kernel::start(&spec, &cwd, crate::prompt::shell_env(pid), sink.clone()) {
        Ok(k) => {
            KERNELS.lock().unwrap().get_or_insert_with(HashMap::new).insert(key, k);
        }
        Err(e) => sink(json!({"type": "status", "state": "dead", "message": e})),
    });
}

fn get(key: &str) -> Option<Arc<Kernel>> {
    KERNELS.lock().unwrap().as_ref()?.get(key).cloned()
}

pub fn has(key: &str) -> bool {
    get(key).is_some()
}

pub fn execute(key: &str, cell: &str, code: &str) -> bool {
    get(key).is_some_and(|k| k.execute(cell, code))
}

/// The input request a kernel is waiting on, if any, marked as told again.
pub fn pending_ask(key: &str) -> Option<Value> {
    let mut e = get(key)?.asked.lock().unwrap().clone()?;
    e["again"] = json!(true);
    Some(e)
}

/// Answer a kernel's input request (what was typed).
pub fn input(key: &str, value: &str) -> bool {
    get(key).is_some_and(|k| k.input_reply(value))
}

/// Bring a kernel up to the shell's environment and folder (before a run).
/// (A folder handed to the shell by another kernel, not applied there yet,
/// counts as the shell's.) A shell's kernel is then looked at, so what the
/// cell changes can go back (export_to_shell).
pub fn sync(key: &str, pid: u32, cwd: &str) -> bool {
    let Some(k) = get(key) else { return false };
    let Some(env) = crate::prompt::shell_env(pid) else { return false };
    let pending = std::fs::read_to_string(env_dir().join(format!("{pid}.cwd"))).ok().filter(|d| !d.is_empty());
    let sent = k.sync(&env.into_iter().collect(), pending.as_deref().unwrap_or(cwd));
    if let Some(look) = look_at(&k, "before") {
        let (seen, looking) = (k.seen.clone(), k.looking.clone());
        looking.store(true, Ordering::SeqCst);
        std::thread::spawn(move || {
            if let Some(now) = look() {
                *seen.lock().unwrap() = Some(now);
            }
            looking.store(false, Ordering::SeqCst);
        });
    }
    sent
}

fn env_dir() -> std::path::PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| "/tmp".into()).join("tessera-env")
}

fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Ask a shell's kernel for its exported environment and folder (queued
/// after whatever it's running); the returned closure waits for the answer.
fn look_at(k: &Arc<Kernel>, what: &str) -> Option<Box<dyn FnOnce() -> Option<(HashMap<String, String>, String)> + Send>> {
    let dir = env_dir();
    let _ = std::fs::create_dir_all(&dir);
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let (envf, pwdf) = (dir.join(format!("kernel-{}-{n}-{what}.env", std::process::id())), dir.join(format!("kernel-{}-{n}-{what}.pwd", std::process::id())));
    let (e, w) = (envf.to_string_lossy().to_string(), pwdf.to_string_lossy().to_string());
    let code = match k.spec.language.to_ascii_lowercase().as_str() {
        "bash" | "zsh" | "sh" => format!("command env -0 > {} 2>/dev/null; printf '%s' \"$PWD\" > {}", sh_quote(&e), sh_quote(&w)),
        "xonsh" => format!("import os as _t\nopen({e:?}, 'w').write(''.join(f'{{a}}={{b}}\\0' for a, b in _t.environ.items()))\nopen({w:?}, 'w').write(_t.getcwd())\ndel _t"),
        "tessh" => {
            use base64::Engine;
            let b = |s: &str| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s);
            format!("__tessh__.dump_env '{}' '{}'", b(&e), b(&w))
        }
        "powershell" => format!(
            "[IO.File]::WriteAllText({e:?}, ((Get-ChildItem env: | ForEach-Object {{ \"$($_.Name)=$($_.Value)`0\" }}) -join ''))\n[IO.File]::WriteAllText({w:?}, (Get-Location).Path)"
        ),
        _ => return None,
    };
    if !k.execute_quiet(&code) {
        return None;
    }
    Some(Box::new(move || {
        let start = std::time::Instant::now();
        while !pwdf.exists() && start.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(50));
        }
        std::thread::sleep(Duration::from_millis(50));
        let got = (std::fs::read(&envf), std::fs::read_to_string(&pwdf));
        let _ = std::fs::remove_file(&envf);
        let _ = std::fs::remove_file(&pwdf);
        let (Ok(raw), Ok(pwd)) = got else { return None };
        let env = raw
            .split(|&b| b == 0)
            .filter_map(|kv| {
                let s = String::from_utf8_lossy(kv);
                let (a, b) = s.split_once('=')?;
                Some((a.to_string(), b.to_string()))
            })
            .collect();
        Some((env, pwd))
    }))
}

/// After a run in a shell's kernel (bash, zsh, xonsh, PowerShell): what the
/// cell changed (its folder, exports set or unset) goes back to the pane's
/// shell (pid), which applies it before its next command (shell/tessera.*),
/// and so to the other kernels: switching shells keeps the folder and
/// variables.
pub fn export_to_shell(key: &str, pid: u32) {
    let Some(k) = get(key) else { return };
    if pid == 0 {
        return;
    }
    let Some(look) = look_at(&k, "after") else { return };
    std::thread::spawn(move || {
        // (The look from before the cell first.)
        let start = std::time::Instant::now();
        while k.looking.load(Ordering::SeqCst) && start.elapsed() < Duration::from_secs(20) {
            std::thread::sleep(Duration::from_millis(30));
        }
        let Some((now, pwd)) = look() else { return };
        let Some((was, was_pwd)) = k.seen.lock().unwrap().replace((now.clone(), pwd.clone())) else { return };
        let skip = |n: &str| SKIP_ENV.contains(&n);
        let set: Vec<(String, String)> = now.iter().filter(|(a, b)| !skip(a) && was.get(*a) != Some(b)).map(|(a, b)| (a.clone(), b.clone())).collect();
        let unset: Vec<String> = was.keys().filter(|a| !skip(a) && !now.contains_key(*a)).cloned().collect();
        let cd = (!pwd.is_empty() && pwd != was_pwd).then(|| pwd.clone());
        if set.is_empty() && unset.is_empty() && cd.is_none() {
            return;
        }
        // (The kernel has these now: no need to send them back to it.)
        {
            let mut had = k.synced.lock().unwrap();
            for (a, b) in &set {
                had.0.insert(a.clone(), b.clone());
            }
            for a in &unset {
                had.0.remove(a);
            }
            if let Some(d) = &cd {
                had.1 = d.clone();
            }
        }
        write_apply(pid, set, unset, cd);
    });
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
        // (Its notebook hears it's gone: the next run starts another.)
        (k.sink)(json!({"type": "status", "state": "dead", "message": "Stopped."}));
        std::thread::spawn(move || k.stop());
    }
}

/// The kernels running now, every notebook's: JSON [{key, name, display,
/// language, state, pid, rss_kb, seconds}] (memory from /proc).
pub fn running() -> String {
    let m = KERNELS.lock().unwrap();
    let mut out: Vec<Value> = Vec::new();
    if let Some(m) = m.as_ref() {
        for (key, k) in m.iter() {
            let pid = k.child.lock().unwrap().as_ref().map(|c| c.id()).unwrap_or(0);
            // (Its memory: the process and what it started, nix shell's child among them.)
            let rss = |p: u32| -> u64 {
                std::fs::read_to_string(format!("/proc/{p}/status"))
                    .ok()
                    .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1)?.parse().ok()))
                    .unwrap_or(0)
            };
            let kids = |p: u32| -> Vec<u32> {
                std::fs::read_to_string(format!("/proc/{p}/task/{p}/children")).unwrap_or_default().split_whitespace().filter_map(|x| x.parse().ok()).collect()
            };
            let mut total = 0;
            let mut stack = vec![pid];
            while let Some(p) = stack.pop() {
                if p == 0 {
                    continue;
                }
                total += rss(p);
                stack.extend(kids(p));
            }
            out.push(json!({"key": key, "name": k.spec.name, "display": k.spec.display, "language": k.spec.language,
                            "state": *k.state.lock().unwrap(), "pid": pid, "rss_kb": total, "seconds": k.started.elapsed().as_secs()}));
        }
    }
    out.sort_by(|a, b| a["display"].as_str().cmp(&b["display"].as_str()));
    Value::Array(out).to_string()
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
        let k = Kernel::start("tessera-nix-python", "/tmp", None, sink).unwrap();
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

    /// Every fetched kernel (Nix, uv): started, a line run, its output
    /// shown. Slow the first time (downloads). TESSERA_KERNELS=a,b picks.
    #[test]
    #[ignore]
    fn fetched_kernels_run() {
        let code = |lang: &str| -> &str {
            match lang {
                "bash" => "echo hi-bash",
                "zsh" => "print hi-zsh",
                "powershell" => "Write-Output hi-ps",
                "xonsh" => "echo hi-xonsh",
                "c" => "printf(\"hi-c\\n\"); fflush(stdout);",
                "cpp" => "import std;\nstd::println(\"hi-cpp {}\", 6 * 7);",
                "r" => "cat(\"hi-r\\n\")",
                "ruby" => "puts \"hi-ruby\"",
                "haskell" => "putStrLn \"hi-haskell\"",
                "rust" => "println!(\"hi-rust\");",
                "go" => "import \"fmt\"\nfmt.Println(\"hi-go\")",
                "typescript" => "console.log(\"hi-ts\")",
                "julia" => "println(\"hi-julia\")",
                "lua" => "print(\"hi-lua\")",
                "nix" => "\"hi\" + \"-nix\"",
                "octave" => "disp('hi-octave')",
                "kotlin" => "println(\"hi-kotlin\")",
                "scala" => "println(\"hi-scala\")",
                "scheme" => "(display \"hi-scheme\")",
                "gnuplot" => "print \"hi-gnuplot\"",
                _ => "",
            }
        };
        let only: Vec<String> = std::env::var("TESSERA_KERNELS").map(|v| v.split(',').map(String::from).collect()).unwrap_or_default();
        let handles: Vec<_> = FETCHED
            .iter()
            .filter(|f| only.is_empty() || only.iter().any(|o| o == f.name || o == f.language))
            .map(|f| {
                let (name, lang) = (f.name.to_string(), f.language.to_string());
                let c = code(&lang).to_string();
                std::thread::spawn(move || {
                    let (tx, rx) = std::sync::mpsc::channel::<Value>();
                    let tx = Mutex::new(tx);
                    let sink: Sink = Arc::new(move |v| {
                        let _ = tx.lock().unwrap().send(v);
                    });
                    let t0 = Instant::now();
                    let k = match Kernel::start(&name, "/tmp", None, sink) {
                        Ok(k) => k,
                        Err(e) => return format!("{name}: FAILED to start: {e}"),
                    };
                    // Up: its info, or idle.
                    let mut up = false;
                    let mut last = String::new();
                    let up_wait = std::env::var("TESSERA_KERNEL_WAIT").ok().and_then(|s| s.parse().ok()).unwrap_or(1200);
                    while let Ok(v) = rx.recv_timeout(Duration::from_secs(up_wait)) {
                        if v["state"] == "dead" {
                            k.stop();
                            return format!("{name}: DIED: {}", v["message"]);
                        }
                        if v["type"] == "info" || v["state"] == "idle" {
                            up = true;
                            break;
                        }
                        last = v.to_string();
                    }
                    if !up {
                        k.stop();
                        return format!("{name}: never came up ({last})");
                    }
                    k.execute("c1", &c);
                    let mut text = String::new();
                    let mut status = String::from("timeout");
                    while let Ok(v) = rx.recv_timeout(Duration::from_secs(600)) {
                        if std::env::var_os("TESSERA_KERNEL_DEBUG").is_some() {
                            eprintln!("{name} event: {v}");
                        }
                        if v["type"] == "output" {
                            let o = &v["output"];
                            text.push_str(o["text"].as_str().unwrap_or(""));
                            text.push_str(o["data"]["text/plain"].as_str().unwrap_or(""));
                            if o["output_type"] == "error" {
                                text.push_str(&format!("[{}: {}]", o["ename"], o["evalue"]));
                            }
                        }
                        if v["type"] == "done" {
                            status = v["status"].as_str().unwrap_or("").to_string();
                            // (Output can come just after.)
                            while let Ok(v) = rx.recv_timeout(Duration::from_millis(1500)) {
                                if v["type"] == "output" {
                                    text.push_str(v["output"]["text"].as_str().unwrap_or(""));
                                    text.push_str(v["output"]["data"]["text/plain"].as_str().unwrap_or(""));
                                }
                            }
                            break;
                        }
                        if v["state"] == "dead" {
                            status = format!("dead {}", v["message"]);
                            break;
                        }
                    }
                    k.stop();
                    format!("{name}: {status} in {:.0}s: {:?}", t0.elapsed().as_secs_f64(), text.trim())
                })
            })
            .collect();
        for h in handles {
            eprintln!("{}", h.join().unwrap_or_else(|_| "panicked".into()));
        }
    }

    /// A kernel asking for input, answered (Python from Nix): run with --ignored.
    #[test]
    #[ignore]
    fn answers_input() {
        let (tx, rx) = std::sync::mpsc::channel::<Value>();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |v| {
            let _ = tx.lock().unwrap().send(v);
        });
        let k = Kernel::start("tessera-nix-python", "/tmp", None, sink).unwrap();
        loop {
            let v = rx.recv_timeout(Duration::from_secs(600)).expect("up");
            if v["type"] == "info" {
                break;
            }
        }
        assert!(k.execute("c1", "x = input('name? '); print('hi', x)"));
        let mut text = String::new();
        loop {
            let v = rx.recv_timeout(Duration::from_secs(60)).expect("event");
            if v["type"] == "input" {
                assert_eq!(v["prompt"], "name? ");
                assert!(k.input_reply("Ben"));
            }
            if v["type"] == "output" {
                text.push_str(v["output"]["text"].as_str().unwrap_or(""));
            }
            if v["type"] == "done" {
                break;
            }
        }
        while let Ok(v) = rx.recv_timeout(Duration::from_millis(800)) {
            if v["type"] == "output" {
                text.push_str(v["output"]["text"].as_str().unwrap_or(""));
            }
        }
        k.stop();
        assert!(text.contains("hi Ben"), "{text:?}");
    }

    /// A shell kernel's cd and export handed back to the pane's shell (bash
    /// from Nix): run with --ignored.
    #[test]
    #[ignore]
    fn hands_back_folder_and_exports() {
        let (tx, rx) = std::sync::mpsc::channel::<Value>();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |v| {
            let _ = tx.lock().unwrap().send(v);
        });
        let k = Kernel::start("tessera-bash", "/", None, sink).unwrap();
        loop {
            if rx.recv_timeout(Duration::from_secs(900)).expect("up")["type"] == "info" {
                break;
            }
        }
        KERNELS.lock().unwrap().get_or_insert_with(HashMap::new).insert("test-hand".into(), k.clone());
        assert!(k.execute("c0", "export TESSERA_T0=x"));
        loop {
            if rx.recv_timeout(Duration::from_secs(60)).expect("event")["type"] == "done" {
                break;
            }
        }
        // (As the notebook does: synced (and looked at) before the cell.)
        let pid = std::process::id();
        let env: Vec<u8> = std::env::vars().flat_map(|(a, b)| format!("{a}={b}\0").into_bytes()).collect();
        let _ = std::fs::write(super::env_dir().join(pid.to_string()), env);
        super::sync("test-hand", pid, "/");
        assert!(k.execute("c1", "cd /tmp && export TESSERA_T1='a b' && unset TESSERA_T0"));
        loop {
            if rx.recv_timeout(Duration::from_secs(60)).expect("event")["type"] == "done" {
                break;
            }
        }
        let apply = super::env_dir().join(format!("{pid}.apply"));
        let _ = std::fs::remove_file(&apply);
        super::export_to_shell("test-hand", pid);
        let start = std::time::Instant::now();
        while !apply.exists() && start.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(100));
        }
        std::thread::sleep(Duration::from_millis(300));
        let text = std::fs::read_to_string(&apply).unwrap_or_default();
        let _ = std::fs::remove_file(&apply);
        let _ = std::fs::remove_file(super::env_dir().join(format!("{pid}.cwd")));
        let _ = std::fs::remove_file(super::env_dir().join(pid.to_string()));
        k.stop();
        eprintln!("{text}");
        assert!(text.contains("export TESSERA_T1='a b'"), "{text}");
        assert!(text.contains("unset TESSERA_T0"), "{text}");
        assert!(text.contains("builtin cd -- '/tmp'"), "{text}");
        assert!(!text.contains("PATH="), "only what the cell changed: {text}");
    }


    /// How long the bash kernel takes to start and to run: --ignored --nocapture.
    #[test]
    #[ignore]
    fn bash_timing() {
        let (tx, rx) = std::sync::mpsc::channel::<Value>();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |v| {
            let _ = tx.lock().unwrap().send(v);
        });
        let t = std::time::Instant::now();
        let k = Kernel::start("tessera-bash", "/", None, sink).unwrap();
        eprintln!("spawned {:?}", t.elapsed());
        loop {
            let v = rx.recv_timeout(Duration::from_secs(900)).expect("up");
            if v["type"] == "info" {
                break;
            }
        }
        eprintln!("info {:?}", t.elapsed());
        for n in 0..3 {
            let t = std::time::Instant::now();
            assert!(k.execute(&format!("c{n}"), "echo hi"));
            loop {
                if rx.recv_timeout(Duration::from_secs(60)).expect("event")["type"] == "done" {
                    break;
                }
            }
            eprintln!("run {n} {:?}", t.elapsed());
        }
        k.stop();
    }

}
