//! Shells as notebook kernels, Tessera's own (no Jupyter): a shell picked in
//! the editor's language list runs the input in a process of its own, kept
//! alive between runs (its variables, functions and folder stay), and each
//! run's output becomes a card. Nothing takes over the terminal.
//!
//! The shell runs a small loop (in its own language) that reads each run's
//! code from a pipe Tessera writes (base64, one line each), runs it in the
//! shell itself, then writes its exported environment and folder to files
//! and prints a mark with the exit status. bash, zsh, sh, dash, ksh, mksh,
//! fish, xonsh and PowerShell work that way; nushell, elvish and tcsh, which
//! can't run code given as text in the shell they're in, run each time
//! afresh, in the environment and folder the last run left.
//!
//! Before a run, the pane's shell's environment and folder go into it (what
//! changed since); after it, what the run changed goes back to the pane's
//! shell (jupyter::write_apply), so the folder and variables follow along
//! whichever shell runs.

use base64::Engine;
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

pub type Sink = crate::jupyter::Sink;

const DONE: &[u8] = b"\x1b]7777;tessera-done;";

/// The shells it can run, by the names their programs go by: (program, nixpkgs attr, display).
fn known(shell: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match shell {
        "bash" => ("bash", "bashInteractive", "bash"),
        "zsh" => ("zsh", "zsh", "zsh"),
        "sh" => ("sh", "bashInteractive", "sh"),
        "dash" => ("dash", "dash", "dash"),
        "ksh" => ("ksh", "ksh", "ksh"),
        "mksh" => ("mksh", "mksh", "mksh"),
        "fish" => ("fish", "fish", "fish"),
        "xonsh" => ("xonsh", "xonsh", "xonsh"),
        "pwsh" | "powershell" => ("pwsh", "powershell", "PowerShell"),
        "nu" => ("nu", "nushell", "nushell"),
        "elvish" => ("elvish", "elvish", "elvish"),
        "tcsh" => ("tcsh", "tcsh", "tcsh"),
        _ => return None,
    })
}

fn stateless(shell: &str) -> bool {
    matches!(shell, "nu" | "elvish" | "tcsh")
}

/// The kernels it offers (for the notebook's list): "tessera-sh-<shell>".
pub fn specs() -> Vec<Value> {
    ["bash", "zsh", "fish", "nu", "xonsh", "pwsh", "sh", "dash", "ksh", "mksh", "elvish", "tcsh"]
        .iter()
        .filter_map(|s| known(s).map(|(_, _, d)| json!({"name": format!("tessera-sh-{s}"), "display": d, "language": s})))
        .collect()
}

pub fn is_spec(name: &str) -> bool {
    name.strip_prefix("tessera-sh-").is_some_and(|s| known(s).is_some())
}

fn on_path(prog: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(prog)).find(|f| f.is_file())).map(|f| f.to_string_lossy().to_string())
}

/// The program: installed, else from Nix (built once, kept).
fn program(shell: &str, status: &dyn Fn(&str)) -> Option<String> {
    let (prog, pkg, _) = known(shell)?;
    if let Some(p) = on_path(prog) {
        return Some(p);
    }
    let argv: Vec<String> = ["nix", "shell", &format!("nixpkgs#{pkg}"), "-c", prog].iter().map(|s| s.to_string()).collect();
    if let Some(p) = crate::jupyter::nix_kept(&argv) {
        return Some(p);
    }
    status(&format!("Getting {prog} with Nix (the first time only)…"));
    crate::jupyter::nix_cached(&argv, &|_| {}).map(|(a, _)| a[0].clone())
}

fn q(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The loop the shell runs: read a line of base64 from `fifo`, run it, write
/// the environment and folder to `dir`, print the mark with the status.
fn loop_script(shell: &str, dir: &Path) -> String {
    let d = dir.to_string_lossy().to_string();
    let fifo = dir.join("in").to_string_lossy().to_string();
    match shell {
        "fish" => format!(
            "set -g __tessera_d {d}\n\
             while read -l __tessera_l\n\
             set -l __tessera_c (printf '%s' $__tessera_l | base64 -d | string collect)\n\
             eval $__tessera_c\n\
             set -l __tessera_s $status\n\
             command env -0 > $__tessera_d/env 2>/dev/null\n\
             printf '%s' $PWD > $__tessera_d/pwd\n\
             printf '\\e]7777;tessera-done;%s\\a' $__tessera_s\n\
             end < {fifo}\n",
            d = q(&d),
            fifo = q(&fifo)
        ),
        "xonsh" => format!(
            "import base64 as _tb, os as _to, sys as _ts\n\
             _td = {d:?}\n\
             with open({fifo:?}) as _tf:\n\
             \x20   for _tl in _tf:\n\
             \x20       _tc = _tb.b64decode(_tl.strip()).decode()\n\
             \x20       try:\n\
             \x20           execx(_tc, 'exec', globals())\n\
             \x20           _tst = __xonsh__.env.get('LAST_RETURN_CODE', 0)\n\
             \x20       except BaseException as _te:\n\
             \x20           print(repr(_te), file=_ts.stderr)\n\
             \x20           _tst = 1\n\
             \x20       _ = open(_td + '/env', 'w').write(''.join(k + '=' + v + '\\0' for k, v in __xonsh__.env.detype().items()))\n\
             \x20       _ = open(_td + '/pwd', 'w').write(_to.getcwd())\n\
             \x20       _ts.stderr.flush()\n\
             \x20       _ = _ts.stdout.write('\\x1b]7777;tessera-done;%s\\x07' % _tst)\n\
             \x20       _ts.stdout.flush()\n"
        ),
        "pwsh" | "powershell" => format!(
            "$__tessera_d = {d}\n\
             $__tessera_r = [IO.StreamReader]::new({fifo})\n\
             while ($null -ne ($__tessera_l = $__tessera_r.ReadLine())) {{\n\
             \x20 $__tessera_c = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($__tessera_l))\n\
             \x20 $global:LASTEXITCODE = 0\n\
             \x20 $__tessera_ok = $true\n\
             \x20 try {{ Invoke-Expression $__tessera_c | Out-Host; $__tessera_ok = $? }} catch {{ Write-Host $_ -ForegroundColor Red; $__tessera_ok = $false }}\n\
             \x20 $__tessera_s = if ($LASTEXITCODE) {{ $LASTEXITCODE }} elseif ($__tessera_ok) {{ 0 }} else {{ 1 }}\n\
             \x20 try {{ [Environment]::CurrentDirectory = (Get-Location -PSProvider FileSystem).ProviderPath }} catch {{}}\n\
             \x20 [IO.File]::WriteAllText(\"$__tessera_d/env\", ((Get-ChildItem env: | ForEach-Object {{ \"$($_.Name)=$($_.Value)`0\" }}) -join ''))\n\
             \x20 [IO.File]::WriteAllText(\"$__tessera_d/pwd\", (Get-Location).ProviderPath)\n\
             \x20 [Console]::Out.Write(\"$([char]27)]7777;tessera-done;$__tessera_s$([char]7)\")\n\
             \x20 [Console]::Out.Flush()\n\
             }}\n",
            d = format!("'{}'", d.replace('\'', "''")),
            fifo = format!("'{}'", fifo.replace('\'', "''"))
        ),
        // bash, zsh, sh, dash, ksh, mksh.
        _ => format!(
            "__tessera_d={d}\n\
             while IFS= read -r __tessera_l; do\n\
             __tessera_c=$(printf '%s' \"$__tessera_l\" | base64 -d)\n\
             eval \"$__tessera_c\"\n\
             __tessera_s=$?\n\
             command env -0 > \"$__tessera_d/env\" 2>/dev/null\n\
             printf '%s' \"$PWD\" > \"$__tessera_d/pwd\"\n\
             printf '\\033]7777;tessera-done;%s\\007' \"$__tessera_s\"\n\
             done < {fifo}\n",
            d = q(&d),
            fifo = q(&fifo)
        ),
    }
}

/// Code that sets the environment and folder, in the shell's words (the
/// pane's shell's, before a run).
fn prelude(shell: &str, set: &[(String, String)], unset: &[String], cd: Option<&str>) -> String {
    let ok = |a: &str| !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    let ps = |s: &str| format!("'{}'", s.replace('\'', "''"));
    let mut out = Vec::new();
    for (a, b) in set.iter().filter(|(a, _)| ok(a)) {
        out.push(match shell {
            "fish" => format!("set -gx {a} {}", q(b)),
            "xonsh" => format!("${a} = {b:?}"),
            "pwsh" | "powershell" => format!("$env:{a} = {}", ps(b)),
            _ => format!("export {a}={}", q(b)),
        });
    }
    for a in unset.iter().filter(|a| ok(a)) {
        out.push(match shell {
            "fish" => format!("set -e {a}"),
            "xonsh" => format!("__xonsh__.env.pop({a:?}, None)"),
            "pwsh" | "powershell" => format!("Remove-Item -ErrorAction SilentlyContinue env:{a}"),
            _ => format!("unset {a}"),
        });
    }
    if let Some(d) = cd {
        out.push(match shell {
            "xonsh" => format!("cd {}", q(d)),
            "pwsh" | "powershell" => format!("Set-Location -LiteralPath {}", ps(d)),
            "fish" => format!("cd {}", q(d)),
            _ => format!("cd -- {}", q(d)),
        });
    }
    out.join("\n")
}

const SKIP: &[&str] = &["PS1", "PS2", "PROMPT", "RPROMPT", "SHLVL", "_", "OLDPWD", "PWD", "COLUMNS", "LINES", "TERM", "TESSERA_SHELL_INTEGRATION", "LAST_RETURN_CODE", "XONSH_VERSION", "XONSH_INTERACTIVE", "fish_pid"];

fn skip(k: &str) -> bool {
    SKIP.contains(&k) || k.starts_with("__tessera") || k.starts_with("BASH_FUNC_") || k.starts_with("ZSH") || k.starts_with("__fish") || k.starts_with("PSModule") || k.starts_with("POWERSHELL")
}

fn read_env(path: &Path) -> Option<HashMap<String, String>> {
    let raw = std::fs::read(path).ok()?;
    Some(
        raw.split(|&b| b == 0)
            .filter_map(|kv| {
                let s = String::from_utf8_lossy(kv);
                let (a, b) = s.split_once('=')?;
                Some((a.to_string(), b.to_string()))
            })
            .collect(),
    )
}

struct Run {
    /// "" for one of Tessera's own (the environment brought in).
    cell: String,
}

pub struct Kernel {
    shell: String,
    display: String,
    prog: String,
    dir: PathBuf,
    child: Mutex<Option<Child>>,
    /// The pipe the loop reads (opened once it's up).
    feed: Mutex<Option<std::fs::File>>,
    runs: Mutex<VecDeque<Run>>,
    /// Its environment and folder after the last run.
    state: Mutex<(HashMap<String, String>, String)>,
    /// What the pane's shell had when last brought in (to send only changes).
    given: Mutex<(HashMap<String, String>, String)>,
    pane_pid: Mutex<u32>,
    count: Mutex<i64>,
    busy: Mutex<String>,
    started: Instant,
    sink: Sink,
}

static KERNELS: OnceLock<Mutex<HashMap<String, Arc<Kernel>>>> = OnceLock::new();

fn kernels() -> &'static Mutex<HashMap<String, Arc<Kernel>>> {
    KERNELS.get_or_init(Default::default)
}

pub fn has(key: &str) -> bool {
    kernels().lock().unwrap().contains_key(key)
}

fn get(key: &str) -> Option<Arc<Kernel>> {
    kernels().lock().unwrap().get(key).cloned()
}

/// Start `tessera-sh-<shell>` for a notebook (key), in the pane's shell's
/// folder and environment (pid).
pub fn start(key: &str, spec: &str, cwd: &str, pid: u32, sink: Sink) {
    stop(key);
    let Some(shell) = spec.strip_prefix("tessera-sh-").map(|s| if s == "powershell" { "pwsh".to_string() } else { s.to_string() }) else { return };
    let (key, cwd) = (key.to_string(), cwd.to_string());
    std::thread::spawn(move || {
        let display = known(&shell).map(|k| k.2).unwrap_or("shell").to_string();
        sink(json!({"type": "status", "state": "starting", "message": format!("Starting {display}…")}));
        let Some(prog) = program(&shell, &|m| sink(json!({"type": "status", "state": "starting", "message": m}))) else {
            sink(json!({"type": "status", "state": "dead", "message": format!("{display} isn't installed, and Nix couldn't get it.")}));
            return;
        };
        let env: HashMap<String, String> = crate::prompt::shell_env(pid).map(|e| e.into_iter().collect()).unwrap_or_else(|| std::env::vars().collect());
        let tag: String = key.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("tessera-shellk").join(format!("{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if std::fs::create_dir_all(&dir).is_err() {
            sink(json!({"type": "status", "state": "dead", "message": "Couldn't make its folder."}));
            return;
        }
        let k = Arc::new(Kernel {
            shell: shell.clone(),
            display,
            prog,
            dir: dir.clone(),
            child: Mutex::new(None),
            feed: Mutex::new(None),
            runs: Mutex::new(VecDeque::new()),
            state: Mutex::new((env.clone(), cwd.clone())),
            given: Mutex::new((env, cwd.clone())),
            pane_pid: Mutex::new(pid),
            count: Mutex::new(0),
            busy: Mutex::new("idle".into()),
            started: Instant::now(),
            sink: sink.clone(),
        });
        if !stateless(&shell) {
            if let Err(e) = k.spawn_loop(&cwd) {
                sink(json!({"type": "status", "state": "dead", "message": e}));
                return;
            }
            // (A first, empty run: its environment as it really is, so a
            // run's changes are told from that, not from what it was given.)
            let noop = match shell.as_str() {
                "xonsh" => "pass",
                "pwsh" => "$null = 0",
                "fish" => "true",
                _ => ":",
            };
            let _ = k.send("", noop);
        }
        kernels().lock().unwrap().insert(key, k);
        sink(json!({"type": "info", "language": shell, "version": ""}));
        sink(json!({"type": "status", "state": "idle"}));
    });
}

impl Kernel {
    fn spawn_loop(self: &Arc<Self>, cwd: &str) -> Result<(), String> {
        let fifo = self.dir.join("in");
        let c = std::ffi::CString::new(fifo.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
        if unsafe { libc::mkfifo(c.as_ptr(), 0o600) } != 0 {
            return Err("Couldn't make its pipe.".into());
        }
        let script = loop_script(&self.shell, &self.dir);
        let mut cmd = Command::new(&self.prog);
        match self.shell.as_str() {
            "fish" => cmd.args(["--no-config", "-c", &script]),
            "xonsh" => cmd.args(["--no-rc", "-c", &script]),
            "pwsh" => cmd.args(["-NoProfile", "-NonInteractive", "-NoLogo", "-Command", &script]),
            "zsh" => cmd.args(["-f", "-c", &script]),
            "bash" => cmd.args(["--noprofile", "--norc", "-c", &script]),
            _ => cmd.args(["-c", &script]),
        };
        let (env, _) = self.state.lock().unwrap().clone();
        cmd.env_clear().envs(env).env("HISTFILE", "/dev/null").env("TERM", "xterm-256color").env("NO_COLOR", "").env_remove("NO_COLOR");
        if Path::new(cwd).is_dir() {
            cmd.current_dir(cwd);
        }
        cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        let mut child = cmd.spawn().map_err(|e| format!("Couldn't start {}: {e}", self.prog))?;
        let out = child.stdout.take();
        let err = child.stderr.take();
        *self.child.lock().unwrap() = Some(child);
        // (Opening the pipe waits for the loop to open it too.)
        let feed = std::fs::OpenOptions::new().write(true).open(&fifo).map_err(|e| e.to_string())?;
        *self.feed.lock().unwrap() = Some(feed);
        if let Some(mut e) = err {
            let k = self.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                while let Ok(n) = e.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    k.emit("stderr", &buf[..n]);
                }
            });
        }
        if let Some(mut o) = out {
            let k = self.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                let mut pending: Vec<u8> = Vec::new();
                loop {
                    let n = match o.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    pending.extend_from_slice(&buf[..n]);
                    // Marks: what's before goes to the run, then it's done.
                    while let Some(i) = pending.windows(DONE.len()).position(|w| w == DONE) {
                        let Some(end) = pending[i..].iter().position(|&b| b == 7) else { break };
                        let status: i64 = String::from_utf8_lossy(&pending[i + DONE.len()..i + end]).trim().parse().unwrap_or(0);
                        let before: Vec<u8> = pending[..i].to_vec();
                        pending.drain(..i + end + 1);
                        k.emit("stdout", &before);
                        k.finished(status);
                    }
                    // (Keep what might be the start of a mark.)
                    let keep = (1..DONE.len().min(pending.len() + 1)).rev().find(|&l| pending.ends_with(&DONE[..l])).unwrap_or(0);
                    let send = pending.len() - keep;
                    if send > 0 {
                        let s: Vec<u8> = pending.drain(..send).collect();
                        k.emit("stdout", &s);
                    }
                }
                *k.busy.lock().unwrap() = "dead".into();
                (k.sink)(json!({"type": "status", "state": "dead", "message": format!("{} stopped.", k.display)}));
            });
        }
        Ok(())
    }

    fn emit(&self, stream: &str, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let cell = self.runs.lock().unwrap().front().map(|r| r.cell.clone()).unwrap_or_default();
        if cell.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(data).to_string();
        (self.sink)(json!({"type": "output", "cell": cell, "output": {"output_type": "stream", "name": stream, "text": text}}));
    }

    /// A run ended: its status, and what it changed back to the pane's shell.
    fn finished(&self, status: i64) {
        let run = self.runs.lock().unwrap().pop_front();
        let after = (read_env(&self.dir.join("env")), std::fs::read_to_string(self.dir.join("pwd")).ok());
        let (was_env, was_pwd) = self.state.lock().unwrap().clone();
        if let (Some(env), Some(pwd)) = after {
            *self.state.lock().unwrap() = (env.clone(), pwd.clone());
            // (A run of the notebook's: what it changed goes to the pane's shell.)
            if run.as_ref().is_some_and(|r| !r.cell.is_empty()) {
                let set: Vec<(String, String)> = env.iter().filter(|(a, b)| !skip(a) && was_env.get(*a) != Some(b)).map(|(a, b)| (a.clone(), b.clone())).collect();
                let unset: Vec<String> = was_env.keys().filter(|a| !skip(a) && !env.contains_key(*a)).cloned().collect();
                let cd = (!pwd.is_empty() && pwd != was_pwd).then(|| pwd.clone());
                if !set.is_empty() || !unset.is_empty() || cd.is_some() {
                    // (The pane's shell will have them: not to be sent back here.)
                    {
                        let mut g = self.given.lock().unwrap();
                        for (a, b) in &set {
                            g.0.insert(a.clone(), b.clone());
                        }
                        for a in &unset {
                            g.0.remove(a);
                        }
                        if let Some(d) = &cd {
                            g.1 = d.clone();
                        }
                    }
                    let pid = *self.pane_pid.lock().unwrap();
                    if pid > 0 {
                        crate::jupyter::write_apply(pid, set, unset, cd);
                    }
                }
            }
        }
        let Some(run) = run else { return };
        if run.cell.is_empty() {
            return;
        }
        let n = {
            let mut c = self.count.lock().unwrap();
            *c += 1;
            *c
        };
        if self.runs.lock().unwrap().is_empty() {
            *self.busy.lock().unwrap() = "idle".into();
        }
        (self.sink)(json!({"type": "done", "cell": run.cell, "status": if status == 0 { "ok" } else { "error" }, "exit": status, "count": n}));
        if self.runs.lock().unwrap().is_empty() {
            (self.sink)(json!({"type": "status", "state": "idle"}));
        }
    }

    fn send(self: &Arc<Self>, cell: &str, code: &str) -> bool {
        if stateless(&self.shell) {
            return self.run_afresh(cell, code);
        }
        self.runs.lock().unwrap().push_back(Run { cell: cell.to_string() });
        let line = base64::engine::general_purpose::STANDARD.encode(code.as_bytes());
        let ok = self.feed.lock().unwrap().as_mut().is_some_and(|f| writeln!(f, "{line}").and_then(|_| f.flush()).is_ok());
        if !ok {
            self.runs.lock().unwrap().pop_back();
        }
        ok
    }

    /// nushell, elvish, tcsh: each run its own process, in the environment
    /// and folder the last one left (nushell's written back by it).
    fn run_afresh(self: &Arc<Self>, cell: &str, code: &str) -> bool {
        let k = self.clone();
        let (cell, code) = (cell.to_string(), code.to_string());
        self.runs.lock().unwrap().push_back(Run { cell: cell.clone() });
        std::thread::spawn(move || {
            let (env, cwd) = k.state.lock().unwrap().clone();
            let d = k.dir.to_string_lossy().to_string();
            let script = match k.shell.as_str() {
                // (Not in a try: a block's environment doesn't outlive it. An
                // error stops it before the write-back, and nothing changes.)
                "nu" => format!(
                    "{code}\n^env -0 | save -f {e}\n$env.PWD | save -f {w}\nprint -n $\"\\e]7777;tessera-done;($env.LAST_EXIT_CODE? | default 0)\\a\"",
                    e = format!("'{d}/env'"),
                    w = format!("'{d}/pwd'")
                ),
                _ => code.clone(),
            };
            let mut cmd = Command::new(&k.prog);
            if k.shell == "nu" {
                cmd.args(["--no-config-file", "-c", &script]);
            } else {
                cmd.args(["-c", &script]);
            }
            cmd.env_clear().envs(env).env("TERM", "xterm-256color");
            if Path::new(&cwd).is_dir() {
                cmd.current_dir(&cwd);
            }
            let out = cmd.stdin(Stdio::null()).output();
            match out {
                Ok(o) => {
                    let mut stdout = o.stdout;
                    let mut status = o.status.code().unwrap_or(1) as i64;
                    if let Some(i) = stdout.windows(DONE.len()).position(|w| w == DONE) {
                        let end = stdout[i..].iter().position(|&b| b == 7).map(|e| i + e).unwrap_or(stdout.len());
                        status = String::from_utf8_lossy(&stdout[i + DONE.len()..end]).trim().parse().unwrap_or(status);
                        stdout.truncate(i);
                    } else {
                        // (Not written back: the folder and environment stay as they were.)
                        let _ = std::fs::write(k.dir.join("pwd"), &cwd);
                        let raw: Vec<u8> = k.state.lock().unwrap().0.iter().flat_map(|(a, b)| format!("{a}={b}\0").into_bytes()).collect();
                        let _ = std::fs::write(k.dir.join("env"), raw);
                    }
                    k.emit("stdout", &stdout);
                    k.emit("stderr", &o.stderr);
                    k.finished(status);
                }
                Err(e) => {
                    k.emit("stderr", format!("Couldn't run {}: {e}\n", k.prog).as_bytes());
                    let _ = std::fs::write(k.dir.join("pwd"), &cwd);
                    k.finished(127);
                }
            }
        });
        true
    }

    /// The pane's shell's environment and folder, what changed since last time.
    fn sync(self: &Arc<Self>, pid: u32, cwd: &str) {
        *self.pane_pid.lock().unwrap() = pid;
        let Some(env) = crate::prompt::shell_env(pid).map(|e| e.into_iter().collect::<HashMap<String, String>>()) else { return };
        let (set, unset, cd) = {
            let mut g = self.given.lock().unwrap();
            let set: Vec<(String, String)> = env.iter().filter(|(a, b)| !skip(a) && g.0.get(*a) != Some(b)).map(|(a, b)| (a.clone(), b.clone())).collect();
            let unset: Vec<String> = g.0.keys().filter(|a| !skip(a) && !env.contains_key(*a)).cloned().collect();
            let cd = (!cwd.is_empty() && cwd != g.1 && Path::new(cwd).is_dir()).then(|| cwd.to_string());
            *g = (env, if cwd.is_empty() { g.1.clone() } else { cwd.to_string() });
            (set, unset, cd)
        };
        if set.is_empty() && unset.is_empty() && cd.is_none() {
            return;
        }
        if stateless(&self.shell) {
            let mut s = self.state.lock().unwrap();
            for (a, b) in set {
                s.0.insert(a, b);
            }
            for a in unset {
                s.0.remove(&a);
            }
            if let Some(d) = cd {
                s.1 = d;
            }
            return;
        }
        let code = prelude(&self.shell, &set, &unset, cd.as_deref());
        let _ = self.send("", &code);
    }

    fn interrupt(&self) {
        // (The command it's running: the shell's children, not the shell.)
        let pid = self.child.lock().unwrap().as_ref().map(|c| c.id()).unwrap_or(0);
        if pid == 0 {
            return;
        }
        let kids = std::fs::read_to_string(format!("/proc/{pid}/task/{pid}/children")).unwrap_or_default();
        for k in kids.split_whitespace().filter_map(|x| x.parse::<i32>().ok()) {
            unsafe {
                libc::kill(k, libc::SIGINT);
            }
        }
    }

    fn stop(&self) {
        *self.feed.lock().unwrap() = None;
        if let Some(mut c) = self.child.lock().unwrap().take() {
            unsafe {
                libc::kill(-(c.id() as i32), libc::SIGTERM);
            }
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

pub fn execute(key: &str, cell: &str, code: &str) -> bool {
    let Some(k) = get(key) else { return false };
    *k.busy.lock().unwrap() = "busy".into();
    (k.sink)(json!({"type": "status", "state": "busy"}));
    k.send(cell, code)
}

pub fn sync(key: &str, pid: u32, cwd: &str) -> bool {
    let Some(k) = get(key) else { return false };
    // (A folder handed to the pane's shell, not applied there yet, counts.)
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(|| "/tmp".into()).join("tessera-env");
    let pending = std::fs::read_to_string(dir.join(format!("{pid}.cwd"))).ok().filter(|d| !d.is_empty());
    k.sync(pid, pending.as_deref().unwrap_or(cwd));
    true
}

pub fn interrupt(key: &str) {
    if let Some(k) = get(key) {
        k.interrupt();
    }
}

pub fn stop(key: &str) {
    let k = kernels().lock().unwrap().remove(key);
    if let Some(k) = k {
        k.stop();
        (k.sink)(json!({"type": "status", "state": "dead", "message": "Stopped."}));
    }
}

pub fn stop_all() {
    let keys: Vec<String> = kernels().lock().unwrap().keys().cloned().collect();
    for k in keys {
        stop(&k);
    }
}

/// For the notebook's kernels list (as jupyter::running's entries).
pub fn running() -> Vec<Value> {
    kernels()
        .lock()
        .unwrap()
        .iter()
        .map(|(key, k)| {
            let pid = k.child.lock().unwrap().as_ref().map(|c| c.id()).unwrap_or(0);
            let rss = std::fs::read_to_string(format!("/proc/{pid}/status"))
                .ok()
                .and_then(|s| s.lines().find(|l| l.starts_with("VmRSS:")).and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok()))
                .unwrap_or(0);
            json!({"key": key, "name": format!("tessera-sh-{}", k.shell), "display": k.display, "language": k.shell,
                   "state": *k.busy.lock().unwrap(), "pid": pid, "rss_kb": rss, "seconds": k.started.elapsed().as_secs()})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_in(shell: &str, codes: &[&str]) -> Vec<(String, i64)> {
        let (tx, rx) = std::sync::mpsc::channel::<Value>();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |v| {
            let _ = tx.lock().unwrap().send(v);
        });
        let key = format!("test-{shell}");
        start(&key, &format!("tessera-sh-{shell}"), "/", 0, sink);
        loop {
            let v = rx.recv_timeout(std::time::Duration::from_secs(600)).expect("up");
            if v["type"] == "status" && v["state"] == "idle" {
                break;
            }
            assert!(v["state"] != "dead", "{v}");
        }
        let mut out = Vec::new();
        for (i, c) in codes.iter().enumerate() {
            let cell = format!("c{i}");
            assert!(execute(&key, &cell, c));
            let mut text = String::new();
            loop {
                let v = rx.recv_timeout(std::time::Duration::from_secs(60)).expect("event");
                if v["type"] == "output" {
                    text.push_str(v["output"]["text"].as_str().unwrap_or(""));
                }
                if v["type"] == "done" {
                    out.push((text.trim().to_string(), v["exit"].as_i64().unwrap_or(-1)));
                    break;
                }
            }
        }
        stop(&key);
        out
    }

    #[test]
    fn bash_keeps_state() {
        let r = run_in("bash", &["x=41; cd /tmp", "echo $((x+1)) $PWD", "false"]);
        assert_eq!(r[1].0, "42 /tmp", "{r:?}");
        assert_eq!(r[2].1, 1);
    }

    /// The other shells (fetched with Nix when missing): --ignored.
    #[test]
    #[ignore]
    fn shells_keep_state() {
        let cases: &[(&str, &[&str], &str)] = &[
            ("zsh", &["x=41; cd /tmp", "echo $((x+1)) $PWD"], "42 /tmp"),
            ("fish", &["set x 41; cd /tmp", "echo (math $x + 1) $PWD"], "42 /tmp"),
            ("xonsh", &["x = 41\ncd /tmp", "print(x + 1, $PWD)"], "42 /tmp"),
            ("pwsh", &["$x = 41; Set-Location /tmp", "\"$($x + 1) $PWD\""], "42 /tmp"),
            ("nu", &["cd /tmp; $env.X = '42'", "print $\"($env.X) ($env.PWD)\""], "42 /tmp"),
        ];
        for (sh, codes, want) in cases {
            let r = run_in(sh, codes);
            eprintln!("{sh}: {r:?}");
            assert_eq!(r.last().unwrap().0, *want, "{sh}: {r:?}");
        }
    }
}
