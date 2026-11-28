//! tessh: a shell language that compiles to Python.
//!
//!     tessh                      the REPL
//!     tessh FILE [ARGS…]         run a script
//!     tessh -c CODE [ARGS…]      run CODE
//!     tessh --check [FILE]       check (stdin without FILE); --json for JSON
//!     tessh --py FILE            print the Python it compiles to
//!     tessh --dap                the debugger, for editors (Debug Adapter Protocol on stdin and stdout)
//!     tessh --lsp                the language server, for editors (Language Server Protocol on stdin and stdout)
//!     tessh --kernel -f CONN     the Jupyter kernel

mod ai;
mod compiler;
mod dap;
mod deps;
mod editor;
mod forms;
mod help;
mod kernel;
mod login;
mod lsp;
mod pyforms;
mod reader;
mod repl;
mod runner;
mod jobs;
mod runtime;
mod shell;
#[cfg(feature = "standalone")]
mod standalone;
mod viewer;
// Helix's grammars and the language a file is in, as Tessera finds them
// (its own files, shared).
#[allow(dead_code)]
#[path = "../../src/grammars.rs"]
mod grammars;
#[allow(dead_code)]
#[path = "../../src/detect.rs"]
mod detect;
// Options from man and info pages, as Tessera's completer reads them.
#[allow(dead_code)]
#[path = "../../src/manopts.rs"]
mod manopts;
// carapace's completions, when it's installed (as Tessera's completer).
#[allow(dead_code)]
#[path = "../../src/carapace.rs"]
mod carapace;

#[cfg(test)]
mod tests;

use pyo3::prelude::*;
use std::io::Read;

const USAGE: &str = "tessh                      the REPL
tessh FILE [ARGS…]         run a script
tessh -c CODE [ARGS…]      run CODE
tessh -s [ARGS…]           run the script on stdin (also: stdin not a terminal, no FILE)
  -l, --login              first: a login shell (the profile's environment, then login.tsh)
  -i                       first: interactive (accepted; the REPL needs a terminal)
  -e                       first: strict mode (set -e): a program failing on a line of its own raises
tessh --check [FILE]       check without running (stdin without FILE); --json for JSON
tessh --py FILE            print the Python it compiles to
tessh --dap                the debugger, for editors (Debug Adapter Protocol on stdin and stdout)
tessh --lsp                the language server, for editors (Language Server Protocol on stdin and stdout)
tessh --kernel -f CONN     the Jupyter kernel
tessh --guide [TOPIC]      the guide (or a topic of it), drawn for the terminal; --keys: the editor's keys
tessh --complete [DIR]     completions for {\"text\", \"cursor\"} on stdin, in DIR (Tessera's editor)
tessh --complete --serve   the same, one {\"text\", \"cursor\", \"cwd\"} a line, kept running
  --with PKG[,PKG…]        first: PyPI packages to have (uv installs them once), as `need` does";

/// A login shell: its login.tsh runs in the first namespace (the REPL's, or
/// the script's).
pub static LOGIN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// login.tsh in NS, for a login shell.
pub fn run_login_file(py: Python<'_>, ns: &Bound<'_, pyo3::types::PyDict>) {
    if !LOGIN.load(std::sync::atomic::Ordering::SeqCst) {
        return;
    }
    let f = login::login_file();
    if f.is_file() {
        if let Err(e) = runner::run_file(py, &f.to_string_lossy(), ns) {
            runner::report(py, e, &f.to_string_lossy());
        }
    }
    // (logout.tsh when it ends: exit, logout, Ctrl+D, a hangup.)
    let ns2 = ns.clone().unbind();
    let at_end = pyo3::types::PyCFunction::new_closure(py, None, None, move |args, _kw| -> PyResult<()> {
        let py = args.py();
        let f = login::logout_file();
        if f.is_file() {
            if let Err(e) = runner::run_file(py, &f.to_string_lossy(), ns2.bind(py)) {
                runner::report(py, e, &f.to_string_lossy());
            }
        }
        Ok(())
    });
    if let (Ok(at_end), Ok(atexit)) = (at_end, py.import("atexit")) {
        let _ = atexit.call_method1("register", (at_end,));
    }
}

/// The script on stdin (`tessh -s ARGS…`, `cmd | tessh`): ARGS are sys.argv[1:].
fn run_stdin(args: Vec<String>) -> i32 {
    let mut text = String::new();
    if let Err(e) = std::io::stdin().read_to_string(&mut text) {
        eprintln!("tessh: stdin: {e}");
        return 1;
    }
    let mut argv = vec!["-".to_string()];
    argv.extend(args);
    Python::attach(|py| {
        if let Err(e) = runner::init(py) {
            e.print(py);
            return 1;
        }
        let ns = match runner::new_namespace(py, "__main__", argv) {
            Ok(ns) => ns,
            Err(e) => {
                e.print(py);
                return 1;
            }
        };
        run_login_file(py, &ns);
        finish(py, runner::run_source(py, &text, &ns, "<stdin>", false), "<stdin>")
    })
}

/// A script's status from its result (an exit, ^C, an error reported).
fn finish(py: Python<'_>, result: Result<(), runner::TesshError>, name: &str) -> i32 {
    match result {
        Ok(()) => runtime::STATUS.load(std::sync::atomic::Ordering::SeqCst),
        Err(runner::TesshError::Py(e)) => {
            if let Some(c) = exit_code(py, &e) {
                return c;
            }
            if e.is_instance_of::<pyo3::exceptions::PyKeyboardInterrupt>(py) {
                return 130;
            }
            // (An uncaught CommandFailed exits with the command's status, as
            // bash's set -e does.)
            let code = runner::unhandled_command(py, &e).and_then(|_| e.value(py).getattr("status").ok()?.extract::<i32>().ok()).filter(|c| *c != 0).unwrap_or(1);
            runner::report(py, runner::TesshError::Py(e), name);
            code
        }
        Err(e) => {
            runner::report(py, e, name);
            1
        }
    }
}

fn exit_code(py: Python<'_>, e: &PyErr) -> Option<i32> {
    if e.is_instance_of::<pyo3::exceptions::PySystemExit>(py) {
        let code = e.value(py).getattr("code").ok()?;
        return Some(if code.is_none() { 0 } else { code.extract::<i32>().unwrap_or(1) });
    }
    None
}

fn main() {
    // (A grammar tried in a child before it's trusted: this program again.)
    grammars::check_from_args();
    // (The standalone build: as python3, or Python pointed at uv's.)
    #[cfg(feature = "standalone")]
    {
        standalone::python_main();
        standalone::prepare();
    }
    let mut argv = std::env::args();
    let argv0 = argv.next().unwrap_or_default();
    let (opts, args) = login::parse(&argv0, argv.collect());
    if args.is_empty() {
        login::probe_guard();
    }
    let code = run(opts, args);
    // (Embedded Python never finalizes: run its atexit work, flush its output.)
    Python::attach(|py| {
        if let Ok(a) = py.import("atexit") {
            let _ = a.call_method0("_run_exitfuncs");
        }
        if let Ok(sys) = py.import("sys") {
            for s in ["stdout", "stderr"] {
                if let Ok(f) = sys.getattr(s) {
                    let _ = f.call_method0("flush");
                }
            }
        }
    });
    std::process::exit(code);
}

fn run(opts: login::Opts, args: Vec<String>) -> i32 {
    // (`ssh host cmd`, scp, rsync: their -c is sh's.)
    if args.first().map(String::as_str) == Some("-c") && login::sh_for_c() {
        let mut a = args;
        if opts.login {
            a.insert(0, "-l".into());
        }
        return login::run_sh_c(&a);
    }
    if opts.strict {
        runtime::STRICT.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    if opts.login {
        login::import_profile();
        LOGIN.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    // (--with PKG, any number, before the rest: packages to have, as uv run's.)
    let mut args = args;
    let mut with: Vec<String> = vec![];
    loop {
        match args.first().map(String::as_str) {
            Some("--with") if args.len() > 1 => {
                with.extend(args[1].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
                args.drain(..2);
            }
            Some(w) if w.starts_with("--with=") => {
                with.extend(w["--with=".len()..].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()));
                args.remove(0);
            }
            _ => break,
        }
    }
    if !with.is_empty() {
        let failed = Python::attach(|py| {
            let r = runner::init(py).and_then(|_| deps::ensure(py, &with, None));
            if let Err(e) = &r {
                e.print(py);
            }
            r.is_err()
        });
        if failed {
            return 1;
        }
    }
    let a = args.first().map(String::as_str).unwrap_or("");
    let tty_in = unsafe { libc::isatty(0) } == 1;
    // (-i: the REPL, which needs a terminal; without one, as bash -i | …
    // reads its commands, the script on stdin.)
    let _ = opts.interactive;
    if opts.stdin || (a.is_empty() && !tty_in) {
        return run_stdin(args);
    }
    if a.is_empty() {
        return repl::run();
    }
    match a {
        "-h" | "--help" => {
            println!("{USAGE}");
            return 0;
        }
        "--guide" | "--keys" => {
            let md = if a == "--keys" { Some(repl::keys_help()) } else { help::lookup(&args[1..].join(" ")).or_else(|| (args.len() == 1).then(help::guide)) };
            let md = if args.len() == 1 && a == "--guide" { help::guide() } else { md.unwrap_or_else(|| format!("No topic {:?} in the guide: `tessh --guide` has it all.", args[1..].join(" "))) };
            // (The whole guide at a terminal: full screen, searchable; else printed.)
            if a == "--guide" && args.len() == 1 && unsafe { libc::isatty(0) == 1 && libc::isatty(1) == 1 } {
                editor::pager::show_guides("", 0);
                return 0;
            }
            let color = unsafe { libc::isatty(1) } == 1 && std::env::var_os("NO_COLOR").is_none();
            print!("{}", help::terminal(&md, color));
            return 0;
        }
        "-V" | "--version" => {
            println!("tessh {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        _ => {}
    }
    Python::attach(|py| {
        if let Err(e) = runner::init(py) {
            e.print(py);
            return 1;
        }
        match a {
            "--check" => {
                let as_json = args.iter().any(|x| x == "--json");
                let rest: Vec<&String> = args[1..].iter().filter(|x| *x != "--json").collect();
                let (text, name) = match rest.first() {
                    Some(f) => match std::fs::read_to_string(f) {
                        Ok(t) => (t, f.to_string()),
                        Err(e) => {
                            eprintln!("tessh: {f}: {e}");
                            return 2;
                        }
                    },
                    None => {
                        let mut s = String::new();
                        let _ = std::io::stdin().read_to_string(&mut s);
                        (s, "<stdin>".to_string())
                    }
                };
                let problems = runner::check(py, &text, &name);
                if as_json {
                    let v: Vec<serde_json::Value> = problems.iter().map(|(l, c, e, m)| serde_json::json!({"line": l, "col": c, "end_col": e, "message": m})).collect();
                    println!("{}", serde_json::Value::Array(v));
                } else {
                    for (l, c, _, m) in &problems {
                        println!("{name}:{l}:{}: {m}", c + 1);
                    }
                }
                if problems.is_empty() { 0 } else { 1 }
            }
            "--complete" => {
                let ns = match runner::new_namespace(py, "__main__", vec![]) {
                    Ok(ns) => ns,
                    Err(e) => {
                        e.print(py);
                        return 1;
                    }
                };
                let answer = |q: &serde_json::Value| {
                    if let Some(d) = q["cwd"].as_str().filter(|d| !d.is_empty()) {
                        if std::env::set_current_dir(d).is_ok() {
                            std::env::set_var("PWD", d);
                        }
                    }
                    let text = q["text"].as_str().unwrap_or("");
                    let cursor = q["cursor"].as_u64().map(|c| c as usize).unwrap_or(text.encode_utf16().count());
                    repl::complete_json(py, &ns, text, cursor)
                };
                // (--serve: one ask a line, one answer a line, for as long as
                // stdin is open: Python, bash-completion's bash and the
                // caches kept warm between asks.)
                if args.get(1).map(String::as_str) == Some("--serve") {
                    use std::io::{BufRead, Write};
                    for line in std::io::stdin().lock().lines() {
                        let Ok(line) = line else { break };
                        let q: serde_json::Value = serde_json::from_str(&line).unwrap_or_default();
                        let mut out = std::io::stdout().lock();
                        if writeln!(out, "{}", answer(&q)).and_then(|_| out.flush()).is_err() {
                            break;
                        }
                    }
                    return 0;
                }
                if let Some(d) = args.get(1) {
                    let _ = std::env::set_current_dir(d);
                }
                let mut s = String::new();
                let _ = std::io::stdin().read_to_string(&mut s);
                let q: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
                println!("{}", answer(&q));
                0
            }
            "--kernel" => {
                let conn = args.iter().position(|x| x == "-f").and_then(|i| args.get(i + 1)).cloned().unwrap_or_default();
                py.detach(|| kernel::run(&conn))
            }
            "--dap" => dap::run(py),
            "--lsp" => lsp::run(py),
            "--py" => {
                let Some(f) = args.get(1) else {
                    eprintln!("tessh: --py FILE");
                    return 2;
                };
                let text = std::fs::read_to_string(f).unwrap_or_default();
                match runner::to_python(py, &text) {
                    Ok(s) => {
                        println!("{s}");
                        0
                    }
                    Err(e) => {
                        runner::report(py, e, f);
                        1
                    }
                }
            }
            "" => 0,
            _ => {
                let (ns_argv, result, name) = if a == "-c" {
                    let Some(code) = args.get(1) else {
                        eprintln!("tessh: -c needs code");
                        return 2;
                    };
                    let mut v = vec!["-c".to_string()];
                    v.extend(args[2..].iter().cloned());
                    let ns = match runner::new_namespace(py, "__main__", v) {
                        Ok(ns) => ns,
                        Err(e) => {
                            e.print(py);
                            return 1;
                        }
                    };
                    run_login_file(py, &ns);
                    (None::<()>, runner::run_source(py, code, &ns, "<string>", false), "<string>".to_string())
                } else {
                    let ns = match runner::new_namespace(py, "__main__", args.clone()) {
                        Ok(ns) => ns,
                        Err(e) => {
                            e.print(py);
                            return 1;
                        }
                    };
                    // (The script's folder first on the path, as Python does.)
                    let dir = std::path::Path::new(a).canonicalize().ok().and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string()));
                    if let (Some(d), Ok(path)) = (dir, py.import("sys").and_then(|s| s.getattr("path"))) {
                        let _ = path.call_method1("insert", (0, d));
                    }
                    run_login_file(py, &ns);
                    (None, runner::run_file(py, a, &ns), a.to_string())
                };
                let _ = ns_argv;
                finish(py, result, &name)
            }
        }
    })
}
