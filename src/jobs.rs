//! Background jobs: commands a notebook runs on their own (a cell ending in
//! `&`, or "Run in background"), kept by tesserad so they go on when the
//! window closes, on this computer or an SSH host. Each keeps its output
//! (stdout and stderr in the order they came), its status, and what it ran
//! with (command line, folder, user, environment).

use serde_json::{json, Value};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};

/// Output kept per job; past it the oldest goes (`dropped` counts it).
const MAX_OUT: usize = 4 << 20;
/// Finished jobs kept (the oldest finished go first).
const MAX_DONE: usize = 100;

struct Job {
    id: u64,
    /// The notebook it belongs to (its kernel key) and its card.
    owner: String,
    cell: String,
    command: String,
    pid: u32,
    started: u64,
    ended: u64,
    status: Option<i32>,
    /// (stream: 1 stdout, 2 stderr; text), in order.
    out: Vec<(u8, String)>,
    /// Bytes of output so far (the offset after the last chunk), and how
    /// many of them were let go.
    total: u64,
    dropped: u64,
    exec: Value,
}

#[derive(Default)]
struct Jobs {
    next: u64,
    list: Vec<Arc<Mutex<Job>>>,
}

fn jobs() -> &'static Mutex<Jobs> {
    static J: OnceLock<Mutex<Jobs>> = OnceLock::new();
    J.get_or_init(|| Mutex::new(Jobs::default()))
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn find(id: u64) -> Option<Arc<Mutex<Job>>> {
    jobs().lock().unwrap().list.iter().find(|j| j.lock().unwrap().id == id).cloned()
}

fn user_name() -> String {
    #[cfg(unix)]
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if !pw.is_null() {
            return std::ffi::CStr::from_ptr((*pw).pw_name).to_string_lossy().into_owned();
        }
    }
    std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default()
}

/// The program that runs a job's line: tessh for a tessh notebook, else the
/// pane's shell (what `pid` runs), else sh.
fn runner(lang: &str, pid: u32) -> String {
    if lang == "tessh" {
        if let Some(p) = crate::syntax::tessh_program() {
            return p.display().to_string();
        }
        return "tessh".into();
    }
    #[cfg(target_os = "linux")]
    if pid > 0 {
        if let Ok(exe) = std::fs::read_link(format!("/proc/{pid}/exe")) {
            let name = exe.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if ["bash", "zsh", "fish", "sh", "dash", "ksh"].contains(&name.as_str()) {
                return exe.display().to_string();
            }
        }
    }
    let _ = pid;
    "sh".into()
}

fn append(job: &Arc<Mutex<Job>>, stream: u8, text: String) {
    let mut j = job.lock().unwrap();
    j.total += text.len() as u64;
    match j.out.last_mut() {
        Some((s, t)) if *s == stream && t.len() < 64 << 10 => t.push_str(&text),
        _ => j.out.push((stream, text)),
    }
    let mut size: usize = j.out.iter().map(|(_, t)| t.len()).sum();
    while size > MAX_OUT && j.out.len() > 1 {
        let (_, t) = j.out.remove(0);
        size -= t.len();
        j.dropped += t.len() as u64;
    }
}

fn read_into(job: Arc<Mutex<Job>>, stream: u8, mut r: impl Read + Send + 'static) {
    std::thread::spawn(move || {
        let mut buf = [0u8; 16384];
        // (A character cut between reads waits for the rest.)
        let mut carry: Vec<u8> = Vec::new();
        loop {
            match r.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    carry.extend_from_slice(&buf[..n]);
                    let cut = match std::str::from_utf8(&carry) {
                        Ok(_) => carry.len(),
                        Err(e) if e.error_len().is_none() => e.valid_up_to(),
                        Err(_) => carry.len(),
                    };
                    let text = String::from_utf8_lossy(&carry[..cut]).into_owned();
                    carry.drain(..cut);
                    append(&job, stream, text);
                }
            }
        }
        if !carry.is_empty() {
            append(&job, stream, String::from_utf8_lossy(&carry).into_owned());
        }
    });
}

/// Start one: {command, lang ("tessh" or a shell's), cwd, pid (the pane's
/// shell, for its environment), owner, cell}. Gives {id, pid, exec} or {error}.
pub fn start(p: &Value) -> Value {
    let s = |k: &str| p[k].as_str().unwrap_or("").to_string();
    let command = s("command");
    let shell_pid = p["pid"].as_u64().unwrap_or(0) as u32;
    let cwd = {
        let c = s("cwd");
        if !c.is_empty() && std::path::Path::new(&c).is_dir() { c } else { std::env::var("HOME").unwrap_or_else(|_| "/".into()) }
    };
    // (The pane's shell's environment, exports since it started included.)
    let mut env: Vec<(String, String)> = crate::prompt::shell_env(shell_pid).unwrap_or_else(|| std::env::vars().collect());
    env.retain(|(k, _)| !k.starts_with("BASH_FUNC_") && k != "_");
    // (Python's output as it comes, not when its buffer fills.)
    if !env.iter().any(|(k, _)| k == "PYTHONUNBUFFERED") {
        env.push(("PYTHONUNBUFFERED".into(), "1".into()));
    }
    env.sort();
    let prog = runner(&s("lang"), shell_pid);
    let mut cmd = Command::new(&prog);
    cmd.arg("-c").arg(&command).current_dir(&cwd).env_clear().envs(env.iter().cloned());
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // (Its own group: Stop ends everything it started.)
        cmd.process_group(0);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return json!({"error": format!("{prog}: {e}")}),
    };
    let pid = child.id();
    let exec = json!({"command": command, "argv": [[prog, "-c", command]], "cwd": cwd, "user": user_name(),
                      "env": env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>(), "pid": pid});
    let id = {
        let mut js = jobs().lock().unwrap();
        js.next += 1;
        js.next
    };
    let job = Arc::new(Mutex::new(Job {
        id,
        owner: s("owner"),
        cell: s("cell"),
        command,
        pid,
        started: now_ms(),
        ended: 0,
        status: None,
        out: Vec::new(),
        total: 0,
        dropped: 0,
        exec: exec.clone(),
    }));
    {
        let mut js = jobs().lock().unwrap();
        js.list.push(job.clone());
        // (Old finished ones go.)
        let done: Vec<u64> = js.list.iter().filter_map(|j| j.lock().ok().filter(|j| j.status.is_some()).map(|j| j.id)).collect();
        if done.len() > MAX_DONE {
            let gone: Vec<u64> = done[..done.len() - MAX_DONE].to_vec();
            js.list.retain(|j| !gone.contains(&j.lock().unwrap().id));
        }
    }
    let (out, err) = (child.stdout.take(), child.stderr.take());
    if let Some(o) = out {
        read_into(job.clone(), 1, o);
    }
    if let Some(e) = err {
        read_into(job.clone(), 2, e);
    }
    let j2 = job.clone();
    std::thread::spawn(move || {
        let st = child.wait();
        let code = match st {
            #[cfg(unix)]
            Ok(s) => {
                use std::os::unix::process::ExitStatusExt;
                s.code().unwrap_or_else(|| 128 + s.signal().unwrap_or(0))
            }
            #[cfg(not(unix))]
            Ok(s) => s.code().unwrap_or(1),
            Err(_) => 1,
        };
        // (Its last output read first.)
        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut j = j2.lock().unwrap();
        j.status = Some(code);
        j.ended = now_ms();
    });
    json!({"id": id, "pid": pid, "exec": exec})
}

fn facts(j: &Job) -> Value {
    json!({"id": j.id, "owner": j.owner, "cell": j.cell, "command": j.command, "pid": j.pid, "started": j.started,
           "ended": j.ended, "running": j.status.is_none(), "status": j.status, "size": j.total})
}

/// Every job (`owner` given: that notebook's), oldest first.
pub fn list(owner: &str) -> Value {
    let js = jobs().lock().unwrap();
    Value::Array(js.list.iter().map(|j| j.lock().unwrap()).filter(|j| owner.is_empty() || j.owner == owner).map(|j| facts(&j)).collect())
}

/// A job's output after offset `from` (what was let go is skipped): {facts,
/// out: [[stream, text]], next, exec}.
pub fn output(id: u64, from: u64, with_exec: bool) -> Value {
    let Some(job) = find(id) else { return json!({"gone": true}) };
    let j = job.lock().unwrap();
    let mut at = j.dropped;
    let mut out = Vec::new();
    for (s, t) in &j.out {
        let end = at + t.len() as u64;
        if end > from {
            let skip = from.saturating_sub(at) as usize;
            let skip = (0..=skip.min(t.len())).rev().find(|&i| t.is_char_boundary(i)).unwrap_or(0);
            out.push(json!([s, &t[skip..]]));
        }
        at = end;
    }
    let mut v = facts(&j);
    v["out"] = Value::Array(out);
    v["next"] = json!(j.total);
    if with_exec {
        v["exec"] = j.exec.clone();
    }
    v
}

/// Stop it: TERM to its group, KILL after 3 s if it's still there.
pub fn stop(id: u64) -> bool {
    let Some(job) = find(id) else { return false };
    let (pid, running) = {
        let j = job.lock().unwrap();
        (j.pid, j.status.is_none())
    };
    if !running {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(-(pid as i32), libc::SIGTERM);
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(3));
        if job.lock().unwrap().status.is_none() {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    });
    true
}

/// Let a finished one go.
pub fn forget(id: u64) {
    jobs().lock().unwrap().list.retain(|j| {
        let j = j.lock().unwrap();
        j.id != id || j.status.is_none()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_done(id: u64) -> Value {
        for _ in 0..200 {
            let v = output(id, 0, true);
            if !v["running"].as_bool().unwrap_or(true) {
                return v;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("job {id} didn't end");
    }

    #[test]
    fn runs_and_keeps_output() {
        let r = start(&json!({"command": "echo out; echo err >&2; exit 3", "lang": "sh", "cwd": "/", "owner": "nb", "cell": "c1"}));
        let id = r["id"].as_u64().unwrap();
        assert_eq!(r["exec"]["cwd"], "/");
        let v = wait_done(id);
        assert_eq!(v["status"], 3);
        let all: String = v["out"].as_array().unwrap().iter().map(|c| c[1].as_str().unwrap()).collect();
        assert!(all.contains("out\n") && all.contains("err\n"), "{all}");
        assert!(v["out"].as_array().unwrap().iter().any(|c| c[0] == 2));
        // (From an offset: only what's after.)
        let next = v["next"].as_u64().unwrap();
        assert!(output(id, next, false)["out"].as_array().unwrap().is_empty());
        assert!(list("nb").as_array().unwrap().iter().any(|j| j["id"] == id));
        forget(id);
        assert!(output(id, 0, false)["gone"].as_bool().unwrap());
    }

    #[test]
    fn stops() {
        let r = start(&json!({"command": "sleep 30", "lang": "sh", "cwd": "/tmp", "owner": "nb2"}));
        let id = r["id"].as_u64().unwrap();
        assert!(stop(id));
        let v = wait_done(id);
        assert_eq!(v["status"], 128 + 15);
    }
}
