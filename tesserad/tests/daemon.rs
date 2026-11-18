//! tesserad end to end: a real tesserad on a scratch socket, driven the way
//! the window does. Shells keep their cells while no window is attached;
//! kernels keep running and hand over what they printed when a window comes
//! back.
#![cfg(unix)]

#[path = "../../src/proto.rs"]
#[allow(dead_code)]
mod proto;

use proto::*;
use serde_json::{json, Value};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

struct Daemon {
    child: Child,
    sock: PathBuf,
    dir: PathBuf,
}

impl Daemon {
    fn start(name: &str) -> Daemon {
        // (Short: a socket's path is limited to ~100 bytes.)
        let dir = std::env::temp_dir().join(format!("tsd-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let sock = dir.join("tessera-test.sock");
        let child = Command::new(env!("CARGO_BIN_EXE_tesserad"))
            .arg("--socket")
            .arg(&sock)
            .env("XDG_STATE_HOME", dir.join("state"))
            .env("XDG_CONFIG_HOME", dir.join("config"))
            .env("XDG_RUNTIME_DIR", &dir)
            .env("SHELL", "/bin/sh")
            .spawn()
            .unwrap();
        let t = Instant::now();
        while UnixStream::connect(&sock).is_err() {
            assert!(t.elapsed() < Duration::from_secs(5), "tesserad didn't start");
            std::thread::sleep(Duration::from_millis(20));
        }
        Daemon { child, sock, dir }
    }

    fn conn(&self) -> UnixStream {
        let s = UnixStream::connect(&self.sock).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        s
    }

    fn rpc(&self, method: &str, params: Value) -> Value {
        let mut s = self.conn();
        write_json(&mut s, T_RPC, &json!({"method": method, "params": params})).unwrap();
        let (tag, p) = read_frame(&mut s).unwrap();
        assert_eq!(tag, T_RPC_REPLY, "{method}: {}", String::from_utf8_lossy(&p));
        serde_json::from_slice(&p).unwrap()
    }

    /// A new shell running `script` (sh): (its id, the attached connection).
    fn create(&self, script: &str) -> (String, UnixStream) {
        let mut s = self.conn();
        write_json(&mut s, T_CREATE, &Create { cols: 80, rows: 24, cwd: self.dir.display().to_string(), command: script.into(), term: String::new(), input: String::new(), scrollback: 100 }).unwrap();
        let (tag, p) = read_frame(&mut s).unwrap();
        assert_eq!(tag, T_ATTACHED);
        (serde_json::from_slice(&p).unwrap(), s)
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn wait_for(what: &str, secs: u64, mut f: impl FnMut() -> bool) {
    let t = Instant::now();
    while !f() {
        assert!(t.elapsed() < Duration::from_secs(secs), "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A command as the shell integration reports it: its line, start, output, end.
fn command_script(line: &str, output: &str, status: i32) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(line);
    format!(r"printf '\033]1337;TesseraCommand={b64}\007\033]133;C\007{output}\n\033]133;D;{status}\007'")
}

#[test]
fn cells_are_kept_while_no_window_is_attached() {
    let d = Daemon::start("cells");
    // The window goes away at once; the command runs a moment later.
    let script = format!("sleep 0.5; {}; sleep 30", command_script("make test", "all passed", 3));
    let (id, s) = d.create(&script);
    drop(s);
    let kept = |d: &Daemon| d.rpc("cells.load", json!({"session": id})).as_str().unwrap_or("").to_string();
    wait_for("the cell", 10, || !kept(&d).is_empty());
    let line: Value = serde_json::from_str(kept(&d).lines().next().unwrap()).unwrap();
    assert_eq!(line["command"], "make test");
    assert_eq!(line["status"], 3);
    assert_eq!(line["id"], 1);
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD.decode(line["raw"].as_str().unwrap()).unwrap();
    assert!(String::from_utf8_lossy(&raw).contains("all passed"));
    // (The folder it ran in: the shell's.)
    assert_eq!(line["cwd"].as_str().unwrap(), d.dir.canonicalize().unwrap().to_str().unwrap());
}

#[test]
fn an_attached_window_gets_each_cells_id_before_its_start() {
    let d = Daemon::start("ids");
    let script = format!("sleep 0.3; {}; {}; sleep 30", command_script("one", "1", 0), command_script("two", "2", 0));
    let (_id, mut s) = d.create(&script);
    let mut seen = Vec::new();
    let mut out = Vec::new();
    let t = Instant::now();
    while (seen.len() < 2 || String::from_utf8_lossy(&out).matches("\x1b]133;D").count() < 2) && t.elapsed() < Duration::from_secs(10) {
        let (tag, p) = read_frame(&mut s).unwrap();
        match tag {
            T_CELL => {
                seen.push(serde_json::from_slice::<u64>(&p).unwrap());
                out.push(b'|');
            }
            T_OUTPUT => out.extend_from_slice(&p),
            _ => {}
        }
    }
    assert_eq!(seen, vec![1, 2]);
    // (Each id comes just before the output with that cell's start mark.)
    let text = String::from_utf8_lossy(&out);
    let marks: Vec<usize> = text.match_indices("|\x1b]133;C").map(|(i, _)| i).collect();
    assert_eq!(marks.len(), 2, "{text:?}");
}

#[test]
fn kernels_run_on_and_hand_over_what_they_said() {
    let d = Daemon::start("kernels");
    let key = "nb-test";
    // No window: started with no subscriber, everything is kept.
    let r = d.rpc("kernel.start", json!({"key": key, "spec": "tessera-sh-sh", "cwd": d.dir.display().to_string(), "pid": 0, "sub": 0}));
    assert_eq!(r["running"], false);
    let running = |d: &Daemon| d.rpc("kernel.running", json!({})).as_array().map(|a| a.iter().any(|k| k["key"] == key)).unwrap_or(false);
    wait_for("the kernel", 20, || running(&d));
    // (It reports idle once it's up: then it takes code.)
    let mut ran = false;
    wait_for("the run to be taken", 20, || {
        ran = ran || d.rpc("kernel.execute", json!({"key": key, "cell": "c1", "code": "echo out-one"})).as_bool() == Some(true);
        ran
    });
    std::thread::sleep(Duration::from_millis(1500));
    // A window comes back: it subscribes and attaches.
    let mut sub = d.conn();
    write_frame(&mut sub, T_SUBSCRIBE, b"").unwrap();
    let (tag, p) = read_frame(&mut sub).unwrap();
    assert_eq!(tag, T_SUBSCRIBED);
    let sid: u64 = serde_json::from_slice(&p).unwrap();
    let a = d.rpc("kernel.attach", json!({"key": key, "sub": sid}));
    assert!(a["kernels"].as_array().unwrap().iter().any(|k| k["key"] == key), "{a}");
    let said = a["events"].to_string();
    assert!(said.contains("out-one"), "what it printed meanwhile: {said}");
    assert!(said.contains("\"done\""), "{said}");
    // From now on, live over the subscription.
    assert_eq!(d.rpc("kernel.execute", json!({"key": key, "cell": "c2", "code": "echo out-two"})).as_bool(), Some(true));
    let t = Instant::now();
    let mut got = String::new();
    while !got.contains("out-two") && t.elapsed() < Duration::from_secs(10) {
        let (tag, p) = read_frame(&mut sub).unwrap();
        if tag == T_EVENT {
            got.push_str(&String::from_utf8_lossy(&p));
        }
    }
    assert!(got.contains("out-two"), "{got}");
    assert!(got.contains(key));
}

#[test]
fn a_sessions_kernels_end_with_it() {
    let d = Daemon::start("owned");
    let (id, s) = d.create("sleep 30");
    drop(s);
    let key = "nb-owned";
    d.rpc("kernel.start", json!({"key": key, "spec": "tessera-sh-sh", "cwd": "/", "pid": 0, "sub": 0, "session": id}));
    let running = |d: &Daemon| d.rpc("kernel.running", json!({})).as_array().map(|a| a.iter().any(|k| k["key"] == key)).unwrap_or(false);
    wait_for("the kernel", 20, || running(&d));
    let mut k = d.conn();
    write_json(&mut k, T_KILL_ID, &id).unwrap();
    wait_for("the kernel to stop with its session", 10, || !running(&d));
}

#[test]
fn notebooks_are_kept_by_session() {
    let d = Daemon::start("nb");
    assert_eq!(d.rpc("notebook.write", json!({"session": "abc", "text": "{\"cells\": []}"})), json!(""));
    assert_eq!(d.rpc("notebook.read", json!({"session": "abc"}))["text"], "{\"cells\": []}");
    assert!(Path::new(&d.dir.join("state/tessera/notebooks/abc.ipynb")).is_file());
    d.rpc("cells.forget", json!({"session": "abc"}));
    assert!(d.rpc("notebook.read", json!({"session": "abc"}))["error"].is_string());
}

#[test]
fn host_info_and_missing_nix() {
    let d = Daemon::start("info");
    let info = d.rpc("host.info", json!({}));
    assert!(info["no_nix"].as_str().unwrap().contains("install.determinate.systems"));
    assert!(info["state_dir"].as_str().unwrap().starts_with(d.dir.to_str().unwrap()));
}
