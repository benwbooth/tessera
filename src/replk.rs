//! Kernels for languages with no Jupyter kernel, Tessera's own (like
//! shellk's shells). A notebook cell in one of them keeps what earlier cells
//! made, in one of two ways:
//!
//! - kept running: the language's interpreter runs a small loop in that
//!   language (a "driver", or its own REPL fed a mark to print), kept alive
//!   between runs, so variables and definitions stay and nothing runs twice;
//! - replayed (compiled languages): each run is the earlier cells that ran
//!   with no error, then this one, as one program; what the earlier ones
//!   printed (the start of its output, the same as last time) isn't shown
//!   again. Their side effects happen again.
//!
//! Each run's output is one stream (stdout and stderr together, in order)
//! and ends with the mark `ESC ] 7777;tessera-done;<status> BEL`.

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

/// How a cell's code reaches a driver: one line of base64, or of hex (for
/// languages with no base64 of their own).
#[derive(Clone, Copy, PartialEq)]
enum Enc {
    B64,
    Hex,
}

enum How {
    /// `prog args.. <driver file>`, fed one encoded line per run.
    Driver { ext: &'static str, enc: Enc, code: &'static str },
    /// The language's own REPL, fed the code as it is, then `mark` (code
    /// that prints the done mark); `init` first, once.
    Raw { init: &'static str, mark: &'static str },
    /// Earlier cells and this one run as one program (snippets::command);
    /// `entry`: a cell with the program's entry point (a main), which runs
    /// but isn't kept for the next runs (two mains wouldn't build); a run
    /// with none gets `stub`, an empty one, where the language needs it;
    /// lines matching `hoist` (imports, where the language wants them first)
    /// go to the top, once each.
    Replay { entry: Option<&'static str>, stub: &'static str, hoist: Option<&'static str> },
}

struct Lang {
    lang: &'static str,
    display: &'static str,
    /// The program and its nixpkgs attribute (fetched when it isn't installed).
    prog: &'static str,
    pkg: &'static str,
    args: &'static [&'static str],
    how: How,
}

const LANGS: &[Lang] = &[
    // ---- kept running: a driver in the language itself ----
    Lang { lang: "perl", display: "Perl", prog: "perl", pkg: "perl", args: &[], how: How::Driver { ext: "pl", enc: Enc::B64, code: PERL } },
    Lang { lang: "php", display: "PHP", prog: "php", pkg: "php", args: &[], how: How::Driver { ext: "php", enc: Enc::B64, code: PHP } },
    Lang { lang: "elixir", display: "Elixir", prog: "elixir", pkg: "elixir", args: &[], how: How::Driver { ext: "exs", enc: Enc::B64, code: ELIXIR } },
    Lang { lang: "erlang", display: "Erlang", prog: "escript", pkg: "erlang", args: &[], how: How::Driver { ext: "erl", enc: Enc::B64, code: ERLANG } },
    Lang { lang: "groovy", display: "Groovy", prog: "groovy", pkg: "groovy", args: &[], how: How::Driver { ext: "groovy", enc: Enc::B64, code: GROOVY } },
    Lang { lang: "tcl", display: "Tcl", prog: "tclsh", pkg: "tcl", args: &[], how: How::Driver { ext: "tcl", enc: Enc::B64, code: TCL } },
    Lang { lang: "clojure", display: "Clojure (Babashka)", prog: "bb", pkg: "babashka", args: &[], how: How::Driver { ext: "clj", enc: Enc::B64, code: CLOJURE } },
    Lang { lang: "racket", display: "Racket", prog: "racket", pkg: "racket", args: &[], how: How::Driver { ext: "rkt", enc: Enc::B64, code: RACKET } },
    Lang { lang: "common-lisp", display: "Common Lisp (SBCL)", prog: "sbcl", pkg: "sbcl", args: &["--script"], how: How::Driver { ext: "lisp", enc: Enc::Hex, code: LISP } },
    Lang { lang: "fennel", display: "Fennel", prog: "fennel", pkg: "luaPackages.fennel", args: &[], how: How::Driver { ext: "fnl", enc: Enc::Hex, code: FENNEL } },
    Lang { lang: "janet", display: "Janet", prog: "janet", pkg: "janet", args: &[], how: How::Driver { ext: "janet", enc: Enc::Hex, code: JANET } },
    Lang { lang: "prolog", display: "Prolog (SWI)", prog: "swipl", pkg: "swi-prolog", args: &[], how: How::Driver { ext: "pl", enc: Enc::B64, code: PROLOG } },
    Lang { lang: "elisp", display: "Emacs Lisp", prog: "emacs", pkg: "emacs-nox", args: &["-Q", "--batch", "-l"], how: How::Driver { ext: "el", enc: Enc::B64, code: ELISP } },
    // ---- kept running: the language's own REPL ----
    Lang {
        lang: "sql",
        display: "SQL (SQLite, in memory)",
        prog: "sqlite3",
        pkg: "sqlite",
        args: &["-batch"],
        how: How::Raw { init: ".mode table\n.headers on\n", mark: ";\n.print \"\\033]7777;tessera-done;0\\007\"\n" },
    },
    Lang {
        lang: "ocaml",
        display: "OCaml",
        prog: "ocaml",
        pkg: "ocaml",
        args: &["-noprompt", "-nopromptcont", "-no-version", "-color", "never"],
        how: How::Raw { init: "", mark: ";;\nlet () = print_string \"\\027]7777;tessera-done;0\\007\";;\n" },
    },
    Lang {
        lang: "sml",
        display: "Standard ML (Poly/ML)",
        prog: "poly",
        pkg: "polyml",
        args: &["-q", "--error-exit"],
        how: How::Raw { init: "PolyML.Compiler.prompt1 := \"\"; PolyML.Compiler.prompt2 := \"\";\n", mark: ";\nprint \"\\^[]7777;tessera-done;0\\^G\";\n" },
    },
    Lang {
        lang: "fsharp",
        display: "F# (dotnet fsi)",
        prog: "dotnet",
        pkg: "dotnet-sdk",
        args: &["fsi", "--nologo", "--quiet", "--readline-"],
        how: How::Raw { init: "", mark: "\n;;\nprintf \"\\u001b]7777;tessera-done;0\\u0007\";;\n" },
    },
    Lang {
        lang: "java",
        display: "Java (JShell)",
        prog: "jshell",
        pkg: "jdk",
        args: &["--feedback", "concise"],
        how: How::Raw {
            init: "/set mode tessera concise -quiet\n/set prompt tessera \"\" \"\"\n/set feedback tessera\n",
            mark: "\nSystem.out.print(\"\\u001b]7777;tessera-done;0\\u0007\");\n",
        },
    },
    // ---- replayed: compiled (or run-once) languages ----
    Lang { lang: "dart", display: "Dart", prog: "dart", pkg: "dart", args: &[], how: How::Replay { entry: Some(r"\bmain\s*\("), stub: "void main() {}", hoist: Some(r"^\s*import\s") } },
    Lang { lang: "swift", display: "Swift", prog: "swift", pkg: "swift", args: &[], how: How::Replay { entry: None, stub: "", hoist: Some(r"^\s*import\s") } },
    Lang { lang: "zig", display: "Zig", prog: "zig", pkg: "zig", args: &[], how: How::Replay { entry: Some(r"\bfn\s+main\s*\("), stub: "pub fn main() void {}", hoist: None } },
    Lang { lang: "nim", display: "Nim", prog: "nim", pkg: "nim", args: &[], how: How::Replay { entry: None, stub: "", hoist: None } },
    Lang { lang: "crystal", display: "Crystal", prog: "crystal", pkg: "crystal", args: &[], how: How::Replay { entry: None, stub: "", hoist: None } },
    Lang { lang: "d", display: "D", prog: "ldc2", pkg: "ldc", args: &[], how: How::Replay { entry: Some(r"\bmain\s*\("), stub: "void main() {}", hoist: None } },
    Lang { lang: "v", display: "V", prog: "v", pkg: "vlang", args: &[], how: How::Replay { entry: Some(r"\bfn\s+main\s*\("), stub: "fn main() {}", hoist: Some(r"^\s*(module|import)\s") } },
    Lang { lang: "c-sharp", display: "C#", prog: "dotnet", pkg: "dotnet-sdk", args: &[], how: How::Replay { entry: None, stub: "", hoist: Some(r"^\s*using\s+[\w.]+\s*;") } },
    Lang { lang: "fortran", display: "Fortran", prog: "gfortran", pkg: "gfortran", args: &[], how: How::Replay { entry: Some(r"(?im)^\s*program\b"), stub: "program main\nend program main", hoist: None } },
    Lang { lang: "hare", display: "Hare", prog: "hare", pkg: "hare", args: &[], how: How::Replay { entry: Some(r"\bfn\s+main\s*\("), stub: "export fn main() void = void;", hoist: Some(r"^\s*use\s") } },
    Lang { lang: "koka", display: "Koka", prog: "koka", pkg: "koka", args: &[], how: How::Replay { entry: Some(r"\bfun\s+main\s*\("), stub: "fun main() ()", hoist: Some(r"^\s*import\s") } },
    Lang { lang: "lean", display: "Lean", prog: "lean", pkg: "lean4", args: &[], how: How::Replay { entry: Some(r"\bdef\s+main\b"), stub: "", hoist: Some(r"^\s*import\s") } },
    Lang { lang: "luau", display: "Luau", prog: "luau", pkg: "luau", args: &[], how: How::Replay { entry: None, stub: "", hoist: None } },
    Lang { lang: "odin", display: "Odin", prog: "odin", pkg: "odin", args: &[], how: How::Replay { entry: Some(r"\bmain\s*::"), stub: "main :: proc() {}", hoist: Some(r"^\s*(package|import)\s") } },
    Lang { lang: "pascal", display: "Pascal", prog: "fpc", pkg: "fpc", args: &[], how: How::Replay { entry: Some(r"(?im)^\s*end\s*\.\s*$"), stub: "begin\nend.", hoist: Some(r"^\s*(program|uses)\s") } },
    Lang { lang: "gleam", display: "Gleam", prog: "gleam", pkg: "gleam", args: &[], how: How::Replay { entry: Some(r"\bpub\s+fn\s+main\s*\("), stub: "pub fn main() { Nil }", hoist: Some(r"^\s*import\s") } },
    Lang { lang: "koto", display: "Koto", prog: "koto", pkg: "koto", args: &[], how: How::Replay { entry: None, stub: "", hoist: None } },
    Lang { lang: "inko", display: "Inko", prog: "inko", pkg: "inko", args: &[], how: How::Replay { entry: Some(r"\bclass\s+async\s+Main\b|\btype\s+async\s+Main\b"), stub: "type async Main {\n  fn async main {}\n}", hoist: Some(r"^\s*import\s") } },
    Lang { lang: "forth", display: "Forth (gforth)", prog: "gforth", pkg: "gforth", args: &[], how: How::Replay { entry: None, stub: "", hoist: None } },
    Lang { lang: "amber", display: "Amber", prog: "amber", pkg: "amber-lang", args: &[], how: How::Replay { entry: None, stub: "", hoist: None } },
];

fn lang_of(spec: &str) -> Option<&'static Lang> {
    let l = spec.strip_prefix("tessera-repl-")?;
    LANGS.iter().find(|x| x.lang == l)
}

pub fn is_spec(name: &str) -> bool {
    lang_of(name).is_some()
}

fn on_path(prog: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join(prog)).find(|f| f.is_file())).map(|f| f.to_string_lossy().to_string())
}

/// The kernels it offers (for the notebook's list): "tessera-repl-<lang>",
/// each where its language can run (installed, or Nix can fetch it).
pub fn specs() -> Vec<Value> {
    LANGS
        .iter()
        .filter(|l| crate::snippets::runnable(l.lang) || on_path(l.prog).is_some())
        .map(|l| {
            // (Kept running is what a kernel does: only a replayed one says how it differs.)
            let display = if matches!(l.how, How::Replay { .. }) { format!("{} (earlier cells run again)", l.display) } else { l.display.to_string() };
            json!({"name": format!("tessera-repl-{}", l.lang), "display": display, "language": l.lang})
        })
        .collect()
}

/// Whether `spec` needs Nix to start (its program isn't installed).
pub fn needs_nix(spec: &str) -> bool {
    lang_of(spec).is_some_and(|l| !matches!(l.how, How::Replay { .. }) && on_path(l.prog).is_none())
}

/// The program: installed, else from Nix (built once, kept).
fn program(l: &Lang, status: &dyn Fn(&str)) -> Option<String> {
    if let Some(p) = on_path(l.prog) {
        return Some(p);
    }
    let argv: Vec<String> = ["nix", "shell", &format!("nixpkgs#{}", l.pkg), "-c", l.prog].iter().map(|s| s.to_string()).collect();
    if let Some(p) = crate::jupyter::nix_kept(&argv) {
        return Some(p);
    }
    status(&format!("Getting {} with Nix (the first time only)…", l.prog));
    crate::jupyter::nix_cached(&argv, &|_| {}).map(|(a, _)| a[0].clone())
}

fn encode(enc: Enc, code: &str) -> String {
    match enc {
        Enc::B64 => base64::engine::general_purpose::STANDARD.encode(code.as_bytes()),
        Enc::Hex => code.bytes().map(|b| format!("{b:02x}")).collect(),
    }
}

struct Run {
    cell: String,
    /// Replay: the cell's code (kept once it runs with no error).
    code: String,
}

pub struct Kernel {
    lang: &'static Lang,
    prog: String,
    dir: PathBuf,
    cwd: String,
    env: HashMap<String, String>,
    child: Mutex<Option<Child>>,
    feed: Mutex<Option<std::process::ChildStdin>>,
    runs: Mutex<VecDeque<Run>>,
    /// Replay: the cells kept, and what they printed last time (stdout, stderr).
    kept: Mutex<(Vec<String>, (Vec<u8>, Vec<u8>))>,
    /// Replay: the run going now (its process group, to interrupt).
    replay_pid: Mutex<u32>,
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

/// Start `tessera-repl-<lang>` for a notebook (key), in the pane's shell's
/// folder and environment (pid).
pub fn start(key: &str, spec: &str, cwd: &str, pid: u32, sink: Sink) {
    stop(key);
    let Some(l) = lang_of(spec) else { return };
    let (key, cwd) = (key.to_string(), cwd.to_string());
    std::thread::spawn(move || {
        sink(json!({"type": "status", "state": "starting", "message": format!("Starting {}…", l.display)}));
        let replay = matches!(l.how, How::Replay { .. });
        let prog = if replay {
            String::new()
        } else {
            match program(l, &|m| sink(json!({"type": "status", "state": "starting", "message": m}))) {
                Some(p) => p,
                None => {
                    sink(json!({"type": "status", "state": "dead", "message": format!("{} isn't installed, and Nix couldn't get it.", l.prog)}));
                    return;
                }
            }
        };
        let env: HashMap<String, String> = crate::prompt::shell_env(pid).map(|e| e.into_iter().collect()).unwrap_or_else(|| std::env::vars().collect());
        let tag: String = key.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("tessera-replk").join(format!("{}-{tag}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        if std::fs::create_dir_all(&dir).is_err() {
            sink(json!({"type": "status", "state": "dead", "message": "Couldn't make its folder."}));
            return;
        }
        let k = Arc::new(Kernel {
            lang: l,
            prog,
            dir,
            cwd: if Path::new(&cwd).is_dir() { cwd } else { std::env::var("HOME").unwrap_or_default() },
            env,
            child: Mutex::new(None),
            feed: Mutex::new(None),
            runs: Mutex::new(VecDeque::new()),
            kept: Mutex::new((Vec::new(), (Vec::new(), Vec::new()))),
            replay_pid: Mutex::new(0),
            count: Mutex::new(0),
            busy: Mutex::new("idle".into()),
            started: Instant::now(),
            sink: sink.clone(),
        });
        if !replay {
            if let Err(e) = k.spawn() {
                sink(json!({"type": "status", "state": "dead", "message": e}));
                return;
            }
        }
        kernels().lock().unwrap().insert(key, k);
        sink(json!({"type": "info", "language": l.lang, "version": ""}));
        sink(json!({"type": "status", "state": "idle"}));
    });
}

impl Kernel {
    fn command(&self, program: &str, args: &[String]) -> Command {
        let mut cmd = Command::new(program);
        cmd.args(args).env_clear().envs(self.env.clone()).env("TERM", "xterm-256color").current_dir(&self.cwd);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        cmd
    }

    /// The interpreter, kept running: its output read for marks.
    fn spawn(self: &Arc<Self>) -> Result<(), String> {
        let mut args: Vec<String> = self.lang.args.iter().map(|s| s.to_string()).collect();
        if let How::Driver { ext, code, .. } = self.lang.how {
            let f = self.dir.join(format!("driver.{ext}"));
            std::fs::write(&f, code).map_err(|e| e.to_string())?;
            args.push(f.to_string_lossy().to_string());
        }
        let (r, w) = std::io::pipe().map_err(|e| e.to_string())?;
        let mut cmd = self.command(&self.prog, &args);
        cmd.stdin(Stdio::piped()).stdout(w.try_clone().map_err(|e| e.to_string())?).stderr(w);
        let mut child = cmd.spawn().map_err(|e| format!("Couldn't start {}: {e}", self.prog))?;
        drop(cmd);
        let mut stdin = child.stdin.take();
        // (A REPL: its settings, then a first run no cell sees, which takes
        // its banner and whatever it says starting up.)
        if let (How::Raw { init, mark }, Some(s)) = (&self.lang.how, stdin.as_mut()) {
            self.runs.lock().unwrap().push_back(Run { cell: String::new(), code: String::new() });
            let _ = s.write_all(format!("{init}{mark}").as_bytes()).and_then(|_| s.flush());
        }
        *self.feed.lock().unwrap() = stdin;
        *self.child.lock().unwrap() = Some(child);
        let k = self.clone();
        std::thread::spawn(move || {
            let mut o = r;
            let mut buf = [0u8; 8192];
            let mut pending: Vec<u8> = Vec::new();
            loop {
                let n = match o.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => n,
                };
                pending.extend_from_slice(&buf[..n]);
                while let Some(i) = pending.windows(DONE.len()).position(|w| w == DONE) {
                    let Some(end) = pending[i..].iter().position(|&b| b == 7) else { break };
                    let status: i64 = String::from_utf8_lossy(&pending[i + DONE.len()..i + end]).trim().parse().unwrap_or(0);
                    let before: Vec<u8> = pending[..i].to_vec();
                    pending.drain(..i + end + 1);
                    k.emit(&before);
                    k.finished(status);
                }
                let keep = (1..DONE.len().min(pending.len() + 1)).rev().find(|&l| pending.ends_with(&DONE[..l])).unwrap_or(0);
                let send = pending.len() - keep;
                if send > 0 {
                    let s: Vec<u8> = pending.drain(..send).collect();
                    k.emit(&s);
                }
            }
            *k.busy.lock().unwrap() = "dead".into();
            // (The runs left can't finish now: said so, so their cards stop.)
            while !k.runs.lock().unwrap().is_empty() {
                k.emit(format!("{} stopped.\n", k.lang.display).as_bytes());
                k.finished(1);
            }
            (k.sink)(json!({"type": "status", "state": "dead", "message": format!("{} stopped.", k.lang.display)}));
        });
        Ok(())
    }

    fn emit(&self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let cell = self.runs.lock().unwrap().front().map(|r| r.cell.clone()).unwrap_or_default();
        if cell.is_empty() {
            return;
        }
        // (A REPL's own echo of the mark's code, at the start: not the cell's.)
        let text = String::from_utf8_lossy(data).to_string();
        (self.sink)(json!({"type": "output", "cell": cell, "output": {"output_type": "stream", "name": "stdout", "text": text}}));
    }

    fn finished(&self, status: i64) {
        let Some(run) = self.runs.lock().unwrap().pop_front() else { return };
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
        (self.sink)(json!({"type": "done", "cell": run.cell, "status": if status == 0 { "ok" } else { "error" }, "exit": status, "count": n, "cwd": self.cwd}));
        if self.runs.lock().unwrap().is_empty() {
            (self.sink)(json!({"type": "status", "state": "idle"}));
        }
    }

    fn send(self: &Arc<Self>, cell: &str, code: &str) -> bool {
        self.runs.lock().unwrap().push_back(Run { cell: cell.to_string(), code: code.to_string() });
        // (Perl: a cell's own `my` variables would end with it; at the top
        // level they're `our`, so the next cells have them.)
        let code = if self.lang.lang == "perl" { perl_toplevel_our(code) } else { code.to_string() };
        let code = code.as_str();
        let text = match &self.lang.how {
            How::Driver { enc, .. } => format!("{}\n", encode(*enc, code)),
            How::Raw { mark, .. } => format!("{}\n{mark}", code.trim_end()),
            How::Replay { .. } => {
                // (One at a time, in order: the next waits for this one.)
                if self.runs.lock().unwrap().len() == 1 {
                    self.replay_next();
                }
                return true;
            }
        };
        let ok = self.feed.lock().unwrap().as_mut().is_some_and(|f| f.write_all(text.as_bytes()).and_then(|_| f.flush()).is_ok());
        if !ok {
            self.runs.lock().unwrap().pop_back();
        }
        ok
    }

    /// Replay: the run at the front of the queue, as one program with the
    /// cells kept before it.
    fn replay_next(self: &Arc<Self>) {
        let Some(code) = self.runs.lock().unwrap().front().map(|r| r.code.clone()) else { return };
        let k = self.clone();
        std::thread::spawn(move || {
            let (kept, before) = k.kept.lock().unwrap().clone();
            let (entry_re, stub, hoist) = match k.lang.how {
                How::Replay { entry, stub, hoist } => (entry, stub, hoist),
                _ => (None, "", None),
            };
            let has_entry = |c: &str| entry_re.is_some_and(|re| regex::Regex::new(re).is_ok_and(|r| r.is_match(c)));
            let mut all = kept.clone();
            all.push(code.clone());
            if !stub.is_empty() && !all.iter().any(|c| has_entry(c)) {
                all.push(stub.to_string());
            }
            let program = match hoist.and_then(|h| regex::Regex::new(h).ok()) {
                Some(re) => {
                    let mut top: Vec<&str> = Vec::new();
                    let mut rest: Vec<String> = Vec::new();
                    for c in &all {
                        let mut body = Vec::new();
                        for l in c.lines() {
                            if re.is_match(l) {
                                if !top.contains(&l) {
                                    top.push(l);
                                }
                            } else {
                                body.push(l);
                            }
                        }
                        rest.push(body.join("\n"));
                    }
                    format!("{}\n\n{}", top.join("\n"), rest.join("\n\n"))
                }
                None => all.join("\n\n"),
            };
            let cmd_line = crate::snippets::command(k.lang.lang, &program, &k.cwd, "bash");
            let (status, out, err) = if cmd_line.is_empty() {
                (127, Vec::new(), format!("Tessera can't run {} here (no {}, and no Nix to fetch it).\n", k.lang.display, k.lang.prog).into_bytes())
            } else {
                k.run_replay(&cmd_line)
            };
            let err = strip_nix_noise(&err);
            // (What the earlier cells printed, the same as last time: not again.)
            let after = |now: &[u8], was: &[u8]| -> Vec<u8> { if !was.is_empty() && now.starts_with(was) { now[was.len()..].to_vec() } else { now.to_vec() } };
            k.emit(&after(&out, &before.0));
            k.emit(&after(&err, &before.1));
            if status == 0 && !has_entry(&code) {
                let mut g = k.kept.lock().unwrap();
                g.0.push(code);
                g.1 = (out, err);
            }
            k.finished(status);
            if !k.runs.lock().unwrap().is_empty() {
                k.replay_next();
            }
        });
    }

    /// The program, run: its status, stdout and stderr.
    fn run_replay(&self, line: &str) -> (i64, Vec<u8>, Vec<u8>) {
        let mut cmd = self.command("bash", &["-c".to_string(), line.to_string()]);
        cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return (127, Vec::new(), format!("Couldn't run it: {e}\n").into_bytes()),
        };
        *self.replay_pid.lock().unwrap() = child.id();
        let mut e = child.stderr.take();
        let errs = std::thread::spawn(move || {
            let mut v = Vec::new();
            if let Some(e) = e.as_mut() {
                let _ = e.read_to_end(&mut v);
            }
            v
        });
        let mut out = Vec::new();
        if let Some(mut o) = child.stdout.take() {
            let _ = o.read_to_end(&mut out);
        }
        let err = errs.join().unwrap_or_default();
        let status = child.wait().ok().and_then(|s| s.code()).unwrap_or(1) as i64;
        *self.replay_pid.lock().unwrap() = 0;
        (status, out, err)
    }

    fn interrupt(&self) {
        // (Replay: the program running; else the interpreter, which a driver
        // takes as an error in the cell, or ends.)
        let pid = match *self.replay_pid.lock().unwrap() {
            0 => self.child.lock().unwrap().as_ref().map(|c| c.id()).unwrap_or(0),
            p => p,
        };
        #[cfg(unix)]
        if pid > 0 {
            unsafe {
                libc::kill(-(pid as i32), libc::SIGINT);
            }
        }
        #[cfg(not(unix))]
        let _ = pid;
    }

    fn stop(&self) {
        *self.feed.lock().unwrap() = None;
        if let Some(mut c) = self.child.lock().unwrap().take() {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(c.id() as i32), libc::SIGTERM);
            }
            let _ = c.kill();
            let _ = c.wait();
        }
        #[cfg(unix)]
        {
            let p = *self.replay_pid.lock().unwrap();
            if p > 0 {
                unsafe {
                    libc::kill(-(p as i32), libc::SIGTERM);
                }
            }
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Perl code with its top-level `my` declarations (outside any braces,
/// strings and comments) made `our`.
fn perl_toplevel_our(code: &str) -> String {
    let b: Vec<char> = code.chars().collect();
    let mut out = String::with_capacity(code.len());
    let (mut depth, mut i) = (0i32, 0usize);
    while i < b.len() {
        let c = b[i];
        match c {
            '#' => {
                while i < b.len() && b[i] != '\n' {
                    out.push(b[i]);
                    i += 1;
                }
                continue;
            }
            '\'' | '"' => {
                out.push(c);
                i += 1;
                while i < b.len() && b[i] != c {
                    if b[i] == '\\' && i + 1 < b.len() {
                        out.push(b[i]);
                        i += 1;
                    }
                    out.push(b[i]);
                    i += 1;
                }
                if i < b.len() {
                    out.push(b[i]);
                    i += 1;
                }
                continue;
            }
            '{' => depth += 1,
            '}' => depth -= 1,
            'm' if depth <= 0
                && b.get(i + 1) == Some(&'y')
                && (i == 0 || !(b[i - 1].is_alphanumeric() || b[i - 1] == '_' || b[i - 1] == '$' || b[i - 1] == '@' || b[i - 1] == '%'))
                && b.get(i + 2).is_some_and(|n| n.is_whitespace() || matches!(n, '$' | '@' | '%' | '(')) =>
            {
                out.push_str("our");
                i += 2;
                continue;
            }
            _ => {}
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Nix's word about fetching what a run needs (the first time): not the program's.
fn strip_nix_noise(err: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(err);
    let mut out = String::new();
    let mut in_list = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim_end();
        if (t.starts_with("these ") || t.starts_with("this ")) && (t.contains(" will be fetched") || t.contains(" will be built")) {
            in_list = true;
            continue;
        }
        if in_list && t.starts_with("  /nix/store/") {
            continue;
        }
        in_list = false;
        if t.starts_with("copying path '/nix/store/") || t.starts_with("building '/nix/store/") || t.starts_with("fetching ") || t.starts_with("error (ignored): SQLite database") {
            continue;
        }
        out.push_str(line);
    }
    out.into_bytes()
}

pub fn execute(key: &str, cell: &str, code: &str) -> bool {
    let Some(k) = get(key) else { return false };
    *k.busy.lock().unwrap() = "busy".into();
    (k.sink)(json!({"type": "status", "state": "busy"}));
    k.send(cell, code)
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
            json!({"key": key, "name": format!("tessera-repl-{}", k.lang.lang), "display": k.lang.display, "language": k.lang.lang,
                   "state": *k.busy.lock().unwrap(), "pid": pid, "rss_kb": rss, "seconds": k.started.elapsed().as_secs()})
        })
        .collect()
}

// ---- the drivers: each reads one encoded line per run, runs it where the
// last left off, and prints the mark with 0 (or 1 after an error) ----

const PERL: &str = r#"use MIME::Base64;
binmode STDOUT, ':utf8';
$| = 1; select(STDERR); $| = 1; select(STDOUT);
while (my $__tessera_l = <STDIN>) {
  my $__tessera_c = decode_base64($__tessera_l);
  utf8::decode($__tessera_c);
  my $__tessera_s = 0;
  { no strict; no warnings; package main; eval $__tessera_c; }
  if ($@) { print "\e[31m$@\e[0m"; $__tessera_s = 1 }
  print "\e]7777;tessera-done;$__tessera_s\a";
}
"#;

const PHP: &str = r#"<?php
$__tessera_in = fopen('php://stdin', 'r');
while (($__tessera_l = fgets($__tessera_in)) !== false) {
  $__tessera_c = base64_decode(trim($__tessera_l));
  $__tessera_c = preg_replace('/^\s*<\?php/', '', $__tessera_c);
  $__tessera_s = 0;
  try { eval($__tessera_c); } catch (\Throwable $__tessera_e) { echo "\033[31m", get_class($__tessera_e), ": ", $__tessera_e->getMessage(), "\033[0m\n"; $__tessera_s = 1; }
  echo "\033]7777;tessera-done;$__tessera_s\007";
  flush();
}
"#;

const ELIXIR: &str = r#"defmodule TesseraRepl do
  def loop(b) do
    case IO.read(:stdio, :line) do
      :eof -> :ok
      {:error, _} -> :ok
      l ->
        c = Base.decode64!(String.trim(l))
        {b, s} =
          try do
            {v, nb} = Code.eval_string(c, b)
            IO.puts(inspect(v, pretty: true))
            {nb, 0}
          rescue
            e -> IO.puts("\e[31m" <> Exception.format(:error, e, __STACKTRACE__) <> "\e[0m"); {b, 1}
          catch
            k, e -> IO.puts("\e[31m" <> Exception.format(k, e, __STACKTRACE__) <> "\e[0m"); {b, 1}
          end
        IO.write("\e]7777;tessera-done;#{s}\a")
        loop(b)
    end
  end
end
TesseraRepl.loop([])
"#;

const ERLANG: &str = r#"#!/usr/bin/env escript
main(_) -> io:setopts([binary, {encoding, unicode}]), loop(erl_eval:new_bindings()).
loop(B) ->
  case io:get_line("") of
    eof -> ok;
    {error, _} -> ok;
    L ->
      C = unicode:characters_to_list(base64:decode(string:trim(L))),
      T = string:trim(C, trailing),
      C2 = case T of "" -> "ok."; _ -> case lists:last(T) of $. -> T; _ -> T ++ "." end end,
      {B2, S} = try
          {ok, Ts, _} = erl_scan:string(C2),
          {ok, Es} = erl_parse:parse_exprs(Ts),
          {value, V, NB} = erl_eval:exprs(Es, B),
          io:format("~tp~n", [V]),
          {NB, 0}
        catch K:E -> io:format("\e[31m~p: ~tp\e[0m~n", [K, E]), {B, 1}
        end,
      io:format("\e]7777;tessera-done;~p~c", [S, 7]),
      loop(B2)
  end.
"#;

const GROOVY: &str = r#"def __tesseraShell = new GroovyShell(new Binding())
def __tesseraIn = new BufferedReader(new InputStreamReader(System.in, 'UTF-8'))
String __tesseraLine
while ((__tesseraLine = __tesseraIn.readLine()) != null) {
  def c = new String(__tesseraLine.trim().decodeBase64(), 'UTF-8')
  int s = 0
  try { def v = __tesseraShell.evaluate(c); if (v != null) println v }
  catch (Throwable e) { println "\u001b[31m${e}\u001b[0m"; s = 1 }
  System.out.print("\u001b]7777;tessera-done;${s}\u0007"); System.out.flush()
}
"#;

const TCL: &str = r#"fconfigure stdout -buffering none
while {[gets stdin __tessera_l] >= 0} {
  set __tessera_c [encoding convertfrom utf-8 [binary decode base64 $__tessera_l]]
  set __tessera_s 0
  if {[catch {uplevel #0 $__tessera_c} __tessera_r]} {
    puts "\x1b\[31m$::errorInfo\x1b\[0m"
    set __tessera_s 1
  } elseif {$__tessera_r ne ""} {
    puts $__tessera_r
  }
  puts -nonewline "\x1b\]7777;tessera-done;$__tessera_s\x07"
  flush stdout
}
"#;

const CLOJURE: &str = r#"(require '[clojure.string :as str])
(loop []
  (when-let [l (read-line)]
    (let [c (String. (.decode (java.util.Base64/getDecoder) (str/trim l)) "UTF-8")
          s (try (let [v (load-string c)] (when-not (nil? v) (prn v)) 0)
                 (catch Throwable e (println (str "\u001b[31m" (or (ex-message e) e) "\u001b[0m")) 1))]
      (print (str "\u001b]7777;tessera-done;" s "\u0007"))
      (flush)
      (recur))))
"#;

const RACKET: &str = r#"#lang racket/base
(require net/base64)
(define ns (make-base-namespace))
(let loop ()
  (define l (read-line))
  (unless (eof-object? l)
    (define c (bytes->string/utf-8 (base64-decode (string->bytes/utf-8 l))))
    (define s
      (with-handlers ([(lambda (e) #t) (lambda (e) (printf "\e[31m~a\e[0m\n" (if (exn? e) (exn-message e) e)) 1)])
        (define in (open-input-string c))
        (let rd ()
          (define f (read in))
          (unless (eof-object? f)
            (call-with-values (lambda () (eval f ns))
              (lambda vs (for-each (lambda (v) (unless (void? v) (println v))) vs)))
            (rd)))
        0))
    (printf "\e]7777;tessera-done;~a\a" s)
    (flush-output)
    (loop)))
"#;

const LISP: &str = r#"(defun tessera-unhex (s)
  (let ((b (make-array (floor (length s) 2) :element-type '(unsigned-byte 8))))
    (dotimes (i (length b)) (setf (aref b i) (parse-integer s :start (* 2 i) :end (+ 2 (* 2 i)) :radix 16)))
    (sb-ext:octets-to-string b :external-format :utf-8)))
(loop for l = (read-line *standard-input* nil) while l do
  (let ((s 0))
    (handler-case
        (with-input-from-string (in (tessera-unhex (string-trim '(#\Space #\Return #\Newline) l)))
          (loop for f = (read in nil in) until (eq f in)
                do (let ((vals (multiple-value-list (eval f))))
                     (when vals (format t "~{~S~^~%~}~%" vals)))))
      (error (e) (format t "~C[31m~A~C[0m~%" #\Esc e #\Esc) (setf s 1)))
    (format t "~C]7777;tessera-done;~D~C" #\Esc s (code-char 7))
    (finish-output)))
"#;

const FENNEL: &str = r#"(local fennel (require :fennel))
(fn unhex [s] (pick-values 1 (s:gsub "%x%x" (fn [h] (string.char (tonumber h 16))))))
(each [l (io.lines)]
  (let [c (unhex (pick-values 1 (l:gsub "%s" "")))
        (ok? err) (pcall (fn []
                           (let [v (fennel.eval c {:env _G :allowedGlobals false})]
                             (when (not= nil v) (print (fennel.view v))))))]
    (when (not ok?) (print (.. "\027[31m" (tostring err) "\027[0m")))
    (io.write (.. "\027]7777;tessera-done;" (if ok? 0 1) "\007"))
    (io.flush)))
"#;

const JANET: &str = r#"(defn unhex [s]
  (def b @"")
  (loop [i :range [0 (length s) 2]]
    (buffer/push-byte b (scan-number (string/slice s i (+ i 2)) 16)))
  (string b))
(while true
  (def l (file/read stdin :line))
  (if (nil? l) (os/exit 0))
  (def c (unhex (string/trim l)))
  (var s 0)
  (try (do (def v (eval-string c)) (unless (nil? v) (pp v)))
       ([e] (print "\e[31m" e "\e[0m") (set s 1)))
  (prin "\e]7777;tessera-done;" s "\x07")
  (flush))
"#;

const PROLOG: &str = r#":- use_module(library(base64)).
:- initialization(main, main).
main :- read_line_to_string(user_input, L), ( L == end_of_file -> halt ; run_line(L), main ).
run_line(L) :-
    base64(C0, L), atom_string(C0, C),
    (   catch(run_cell(C), E, (print_message(error, E), S = 1))
    ->  ( var(S) -> S = 0 ; true )
    ;   format("\e[31mfalse.\e[0m~n"), S = 1
    ),
    format("\e]7777;tessera-done;~w\a", [S]), flush_output.
run_cell(C) :-
    split_string(C, "", " \t\n", [T]),
    (   sub_string(T, 0, 2, _, "?-")
    ->  sub_string(T, 2, _, 0, Q),
        term_string(G, Q, [variable_names(Vs)]),
        ( call(G) -> print_bindings(Vs) ; fail )
    ;   flag(tessera_cell, N, N + 1), atom_concat(tessera_cell_, N, Id),
        open_string(C, St), load_files(Id, [stream(St), silent(true)])
    ).
print_bindings([]) :- format("true.~n").
print_bindings([V|Vs]) :- forall(member(N = X, [V|Vs]), format("~w = ~q~n", [N, X])).
"#;

const ELISP: &str = r#"(setq debug-on-error nil)
(while t
  (let* ((l (condition-case nil (read-from-minibuffer "") (error (kill-emacs 0))))
         (c (decode-coding-string (base64-decode-string l) 'utf-8))
         (s 0) (pos 0) (v nil) (any nil))
    (condition-case e
        (catch 'done
          (while t
            (let ((r (condition-case nil (read-from-string c pos) (end-of-file (throw 'done nil)))))
              (setq v (eval (car r) t) pos (cdr r) any t))))
      (error (setq s 1) (princ (format "\e[31m%s\e[0m\n" (error-message-string e)))))
    (when (and any (= s 0)) (princ (format "%S\n" v)))
    (princ (format "\e]7777;tessera-done;%d\a" s))))
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn run_in(lang: &str, codes: &[&str]) -> Vec<(String, i64)> {
        let (tx, rx) = std::sync::mpsc::channel::<Value>();
        let tx = Mutex::new(tx);
        let sink: Sink = Arc::new(move |v| {
            let _ = tx.lock().unwrap().send(v);
        });
        let key = format!("test-{lang}");
        start(&key, &format!("tessera-repl-{lang}"), "/tmp", 0, sink);
        loop {
            let v = rx.recv_timeout(std::time::Duration::from_secs(1200)).expect("up");
            if v["type"] == "status" && v["state"] == "idle" {
                break;
            }
            assert!(v["state"] != "dead", "{lang}: {v}");
        }
        let mut out = Vec::new();
        for (i, c) in codes.iter().enumerate() {
            let cell = format!("c{i}");
            assert!(execute(&key, &cell, c), "{lang}: not sent");
            let mut text = String::new();
            loop {
                let v = rx.recv_timeout(std::time::Duration::from_secs(1200)).unwrap_or_else(|_| panic!("{lang}: no answer; so far {text:?}"));
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

    /// Each language: a value made in one cell, used in the next (and an
    /// error after). Fetches what isn't installed with Nix: --ignored.
    #[test]
    #[ignore]
    fn languages_keep_state() {
        let only = std::env::var("REPLK_ONLY").unwrap_or_default();
        let cases: &[(&str, &[&str], &str)] = &[
            ("perl", &["my $a = 41; sub f { my $t = 1; $t }", "print $a + f(), \"\\n\";"], "42"),
            ("forth", &["41 constant x", "x 1 + ."], "42"),
            ("php", &["$x = 41;", "echo $x + 1, \"\\n\";"], "42"),
            ("elixir", &["x = 41", "IO.puts(x + 1)"], "42"),
            ("erlang", &["X = 41.", "io:format(\"~p~n\", [X + 1])."], "42"),
            ("groovy", &["x = 41", "println(x + 1)"], "42"),
            ("tcl", &["set x 41", "puts [expr {$x + 1}]"], "42"),
            ("clojure", &["(def x 41)", "(println (inc x))"], "42"),
            ("racket", &["(define x 41)", "(displayln (+ x 1))"], "42"),
            ("common-lisp", &["(defparameter *x* 41)", "(print (+ *x* 1))"], "42"),
            ("fennel", &["(global x 41)", "(print (+ x 1))"], "42"),
            ("janet", &["(def x 41)", "(print (+ x 1))"], "42"),
            ("prolog", &["x(41).", "?- x(Y), Z is Y + 1, format(\"~w~n\", [Z])."], "42"),
            ("elisp", &["(setq x 41)", "(princ (format \"%d\\n\" (1+ x)))"], "42"),
            ("sql", &["create table t(x int); insert into t values (41);", "select x + 1 as y from t;"], "42"),
            ("ocaml", &["let x = 41", "print_int (x + 1); print_newline ()"], "42"),
            ("sml", &["val x = 41", "print (Int.toString (x + 1) ^ \"\\n\")"], "42"),
            ("java", &["int x = 41;", "System.out.println(x + 1);"], "42"),
            ("swift", &["let x = 41", "print(x + 1)"], "42"),
            ("nim", &["let x = 41", "echo x + 1"], "42"),
            ("zig", &["const x: i32 = 41;", "const std = @import(\"std\");\npub fn main() void { std.debug.print(\"{d}\\n\", .{x + 1}); }"], "42"),
            ("dart", &["int x = 41;", "void main() { print(x + 1); }"], "42"),
            ("crystal", &["x = 41\nputs \"first\"", "puts x + 1"], "42"),
            ("fortran", &["module m\n  integer :: x = 41\nend module m", "program p\n  use m\n  print *, x + 1\nend program p"], "42"),
            ("pascal", &["var x: integer;", "begin\n  x := 41;\n  writeln(x + 1);\nend."], "42"),
            ("fsharp", &["let x = 41", "printfn \"%d\" (x + 1)"], "42"),
            ("c-sharp", &["var x = 41;", "using System.Linq;\nConsole.WriteLine(new[] { x }.Select(v => v + 1).First());"], "42"),
            ("v", &["const x = 41", "fn main() { println(x + 1) }"], "42"),
            ("hare", &["def X: int = 41;", "use fmt;\nexport fn main() void = { fmt::println(X + 1)!; };"], "42"),
            ("koka", &["val x = 41", "fun main() println(x + 1)"], "42"),
            ("lean", &["def x : Nat := 41", "#eval x + 1"], "42"),
            ("luau", &["x = 41", "print(x + 1)"], "42"),
            ("odin", &["package main\nx :: 41", "import \"core:fmt\"\nmain :: proc() { fmt.println(x + 1) }"], "42"),
            ("gleam", &["const x = 41", "import gleam/io\nimport gleam/int\npub fn main() { io.println(int.to_string(x + 1)) }"], "42"),
            ("koto", &["x = 41", "print x + 1"], "42"),
            ("inko", &["let X = 41", "import std.stdio (Stdout)\ntype async Main {\n  fn async main {\n    Stdout.new.print((X + 1).to_string)\n  }\n}"], "42"),
            ("amber", &["let x = 41", "echo(x + 1)"], "42"),
            ("d", &["int x = 41;", "import std.stdio;\nvoid main() { writeln(x + 1); }"], "42"),
        ];
        for (lang, codes, want) in cases {
            if !only.is_empty() && !only.split(',').any(|o| o == *lang) {
                continue;
            }
            let r = run_in(lang, codes);
            eprintln!("{lang}: {r:?}");
            assert!(r.last().unwrap().0.contains(want), "{lang}: {r:?}");
            assert_eq!(r.last().unwrap().1, 0, "{lang}: {r:?}");
        }
    }
}
