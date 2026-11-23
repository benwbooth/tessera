//! tessh: a shell language that compiles to Python.
//!
//!     tessh                      the REPL
//!     tessh FILE [ARGS…]         run a script
//!     tessh -c CODE [ARGS…]      run CODE
//!     tessh --check [FILE]       check (stdin without FILE); --json for JSON
//!     tessh --py FILE            print the Python it compiles to
//!     tessh --kernel -f CONN     the Jupyter kernel

mod ai;
mod compiler;
mod deps;
mod editor;
mod forms;
mod help;
mod kernel;
mod pyforms;
mod reader;
mod repl;
mod runner;
mod runtime;
#[cfg(feature = "standalone")]
mod standalone;

#[cfg(test)]
mod tests;

use pyo3::prelude::*;
use std::io::Read;

const USAGE: &str = "tessh                      the REPL
tessh FILE [ARGS…]         run a script
tessh -c CODE [ARGS…]      run CODE
tessh --check [FILE]       check without running (stdin without FILE); --json for JSON
tessh --py FILE            print the Python it compiles to
tessh --kernel -f CONN     the Jupyter kernel
tessh --guide [TOPIC]      the guide (or a topic of it), drawn for the terminal; --keys: the editor's keys
tessh --complete [DIR]     completions for {\"text\", \"cursor\"} on stdin, in DIR (Tessera's editor)
  --with PKG[,PKG…]        first: PyPI packages to have (uv installs them once), as `need` does";

fn exit_code(py: Python<'_>, e: &PyErr) -> Option<i32> {
    if e.is_instance_of::<pyo3::exceptions::PySystemExit>(py) {
        let code = e.value(py).getattr("code").ok()?;
        return Some(if code.is_none() { 0 } else { code.extract::<i32>().unwrap_or(1) });
    }
    None
}

fn main() {
    // (The standalone build: as python3, or Python pointed at uv's.)
    #[cfg(feature = "standalone")]
    {
        standalone::python_main();
        standalone::prepare();
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(args);
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

fn run(args: Vec<String>) -> i32 {
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
                if let Some(d) = args.get(1) {
                    let _ = std::env::set_current_dir(d);
                }
                let mut s = String::new();
                let _ = std::io::stdin().read_to_string(&mut s);
                let q: serde_json::Value = serde_json::from_str(&s).unwrap_or_default();
                let text = q["text"].as_str().unwrap_or("");
                let cursor = q["cursor"].as_u64().map(|c| c as usize).unwrap_or(text.encode_utf16().count());
                let ns = match runner::new_namespace(py, "__main__", vec![]) {
                    Ok(ns) => ns,
                    Err(e) => {
                        e.print(py);
                        return 1;
                    }
                };
                println!("{}", repl::complete_json(py, &ns, text, cursor));
                0
            }
            "--kernel" => {
                let conn = args.iter().position(|x| x == "-f").and_then(|i| args.get(i + 1)).cloned().unwrap_or_default();
                py.detach(|| kernel::run(&conn))
            }
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
                    (None, runner::run_file(py, a, &ns), a.to_string())
                };
                let _ = ns_argv;
                match result {
                    Ok(()) => runtime::STATUS.load(std::sync::atomic::Ordering::SeqCst),
                    Err(runner::TesshError::Py(e)) => {
                        if let Some(c) = exit_code(py, &e) {
                            return c;
                        }
                        if e.is_instance_of::<pyo3::exceptions::PyKeyboardInterrupt>(py) {
                            return 130;
                        }
                        runner::report(py, runner::TesshError::Py(e), &name);
                        1
                    }
                    Err(e) => {
                        runner::report(py, e, &name);
                        1
                    }
                }
            }
        }
    })
}
