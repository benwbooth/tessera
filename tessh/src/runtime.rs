//! What compiled tessh calls at run time, as the `__tessh__` module: running
//! programs (pipes, redirections, job control), words (globs, ~), the
//! environment, and the shell builtins (cd, export, …).

use pyo3::exceptions::{PyIndexError, PyNameError, PyOSError, PySystemExit, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList, PyString, PyTuple};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Mutex;

pub static STATUS: AtomicI32 = AtomicI32::new(0);
pub static INTERACTIVE: AtomicBool = AtomicBool::new(false);
/// Run attached programs with their output captured and passed on through
/// Python's sys.stdout (the kernel: the notebook shows it).
pub static PASS_THROUGH: AtomicBool = AtomicBool::new(false);
static OLDPWD: Mutex<Option<String>> = Mutex::new(None);
static DIRSTACK: Mutex<Vec<String>> = Mutex::new(Vec::new());

const PRELUDE: &str = r#"
class Output(str):
    """A program's standard output (minus the last newline), with its exit
    status. True when the program succeeded."""
    def __new__(cls, text="", code=0, err="", attached=False):
        o = str.__new__(cls, text)
        o.code = code
        o.err = err
        o.attached = attached
        return o
    ok = property(lambda s: s.code == 0)
    lines = property(lambda s: s.splitlines())
    def __bool__(self):
        return self.code == 0
    def __repr__(self):
        if self.attached and not str(self):
            return f"<exit {self.code}>"
        r = str.__repr__(self)
        return r if self.code == 0 else f"Output({r}, code={self.code})"

class _Glob(str):
    pass
"#;

fn prelude(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    static P: pyo3::sync::PyOnceLock<Py<PyModule>> = pyo3::sync::PyOnceLock::new();
    Ok(P.get_or_try_init(py, || -> PyResult<Py<PyModule>> {
        Ok(PyModule::from_code(py, &std::ffi::CString::new(PRELUDE).unwrap(), c"<tessh>", c"__tessh_prelude__")?.unbind())
    })?
    .bind(py)
    .clone())
}

fn output(py: Python<'_>, text: &str, code: i32, attached: bool) -> PyResult<Py<PyAny>> {
    Ok(prelude(py)?.getattr("Output")?.call1((text, code, "", attached))?.unbind())
}

fn decode(py: Python<'_>, b: &[u8]) -> PyResult<String> {
    PyBytes::new(py, b).call_method1("decode", ("utf-8", "surrogateescape"))?.extract()
}

// ---- names ----------------------------------------------------------------------

fn builtins_dict<'py>(py: Python<'py>, ns: &Bound<'py, PyDict>) -> PyResult<Bound<'py, PyDict>> {
    match ns.get_item("__builtins__")? {
        Some(b) => match b.cast_into::<PyDict>() {
            Ok(d) => Ok(d),
            Err(e) => Ok(e.into_inner().getattr("__dict__")?.cast_into::<PyDict>()?),
        },
        None => Ok(py.import("builtins")?.getattr("__dict__")?.cast_into::<PyDict>()?),
    }
}

/// $name for a name the compiler didn't know: the module's, a builtin, or an
/// environment variable.
#[pyfunction]
fn lookup<'py>(py: Python<'py>, ns: Bound<'py, PyDict>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    if let Some(v) = ns.get_item(name)? {
        return Ok(v);
    }
    if let Some(v) = builtins_dict(py, &ns)?.get_item(name)? {
        return Ok(v);
    }
    if let Ok(v) = std::env::var(name) {
        return Ok(PyString::new(py, &v).into_any());
    }
    Err(PyNameError::new_err(format!("name '{name}' is not defined (and there's no environment variable {name})")))
}

/// A macro template's free name, where a local of the same name would hide it.
#[pyfunction]
fn free<'py>(py: Python<'py>, ns: Bound<'py, PyDict>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    if let Some(v) = ns.get_item(name)? {
        return Ok(v);
    }
    if let Some(v) = builtins_dict(py, &ns)?.get_item(name)? {
        return Ok(v);
    }
    Err(PyNameError::new_err(format!("name '{name}' is not defined")))
}

/// A name alone on a line: called if it's callable.
#[pyfunction]
fn auto<'py>(v: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    if v.is_callable() {
        v.call0()
    } else {
        Ok(v)
    }
}

static DISPLAY: Mutex<Option<Py<PyAny>>> = Mutex::new(None);

pub fn set_display(f: Py<PyAny>) {
    *DISPLAY.lock().unwrap() = Some(f);
}

/// Show a value at the REPL (the last form of an input).
#[pyfunction]
fn show(py: Python<'_>, v: Bound<'_, PyAny>) -> PyResult<()> {
    if v.is_none() || is_attached_output(&v) {
        return Ok(());
    }
    let d = DISPLAY.lock().unwrap().as_ref().map(|d| d.clone_ref(py));
    match d {
        Some(d) => {
            d.call1(py, (v,))?;
        }
        None => {
            py.import("builtins")?.setattr("_", &v)?;
            let r = v.repr()?.to_string();
            py.import("builtins")?.getattr("print")?.call1((r,))?;
        }
    }
    Ok(())
}

fn is_attached_output(v: &Bound<'_, PyAny>) -> bool {
    v.getattr("attached").and_then(|a| a.extract::<bool>()).unwrap_or(false) && v.extract::<String>().map(|s| s.is_empty()).unwrap_or(false)
}

#[pyfunction]
fn status() -> i32 {
    STATUS.load(Ordering::SeqCst)
}

// ---- words ----------------------------------------------------------------------

#[pyfunction]
fn g(py: Python<'_>, pattern: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    Ok(prelude(py)?.getattr("_Glob")?.call1((pattern.str()?,))?.unbind())
}

pub fn expanduser(s: &str) -> String {
    if s == "~" || s.starts_with("~/") {
        if let Ok(h) = std::env::var("HOME") {
            return format!("{h}{}", &s[1..]);
        }
    } else if let Some(rest) = s.strip_prefix('~') {
        let (user, tail) = rest.split_once('/').map(|(u, t)| (u, format!("/{t}"))).unwrap_or((rest, String::new()));
        if let Ok(c) = std::ffi::CString::new(user) {
            let pw = unsafe { libc::getpwnam(c.as_ptr()) };
            if !pw.is_null() {
                let dir = unsafe { std::ffi::CStr::from_ptr((*pw).pw_dir) }.to_string_lossy().to_string();
                return dir + &tail;
            }
        }
    }
    s.to_string()
}

#[pyfunction]
fn t(v: Bound<'_, PyAny>) -> PyResult<Bound<'_, PyAny>> {
    match v.extract::<String>() {
        Ok(s) if s.starts_with('~') => Ok(PyString::new(v.py(), &expanduser(&s)).into_any()),
        _ => Ok(v),
    }
}

fn glob_expand(p: &str) -> Vec<String> {
    let p = expanduser(p);
    if !p.contains(['*', '?', '[']) {
        return vec![p];
    }
    let opts = glob::MatchOptions { case_sensitive: true, require_literal_separator: true, require_literal_leading_dot: true };
    let mut m: Vec<String> = glob::glob_with(&p, opts).map(|it| it.filter_map(|x| x.ok()).map(|x| x.to_string_lossy().to_string()).collect()).unwrap_or_default();
    m.sort();
    if m.is_empty() {
        vec![p]
    } else {
        m
    }
}

fn word(v: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(b) = v.cast::<PyBytes>() {
        return decode(v.py(), b.as_bytes());
    }
    if v.hasattr("__fspath__")? {
        return v.call_method0("__fspath__")?.extract();
    }
    Ok(v.str()?.to_string())
}

/// The arguments: globs expanded, lists spread.
fn argv(py: Python<'_>, items: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    let globc = prelude(py)?.getattr("_Glob")?;
    let mut out = vec![];
    for x in items.try_iter()? {
        let x = x?;
        if x.is_instance(&globc)? {
            out.extend(glob_expand(&x.extract::<String>()?));
        } else if x.is_instance_of::<PyList>() || x.is_instance_of::<PyTuple>() {
            for y in x.try_iter()? {
                out.push(word(&y?)?);
            }
        } else if x.is_none() || is_attached_output(&x) {
            continue;
        } else {
            out.push(word(&x)?);
        }
    }
    Ok(out)
}

// ---- running ----------------------------------------------------------------------

#[pyclass(module = "__tessh__")]
pub struct Stage {
    /// cmd, call or value
    kind: &'static str,
    words: Vec<String>,
    redirs: Vec<(String, Option<String>)>,
    obj: Option<Py<PyAny>>,
}

#[pymethods]
impl Stage {
    fn __repr__(&self) -> String {
        match self.kind {
            "cmd" => format!("Cmd({})", self.words.join(" ")),
            k => format!("<{k} stage>"),
        }
    }
}

#[pyfunction]
fn call_stage(f: Py<PyAny>) -> Stage {
    Stage { kind: "call", words: vec![], redirs: vec![], obj: Some(f) }
}

#[pyfunction]
fn value_stage(v: Py<PyAny>) -> Stage {
    Stage { kind: "value", words: vec![], redirs: vec![], obj: Some(v) }
}

fn redirs_of(r: &Bound<'_, PyAny>) -> PyResult<Vec<(String, Option<String>)>> {
    let mut out = vec![];
    for x in r.try_iter()? {
        let x = x?;
        let op: String = x.get_item(0)?.extract()?;
        let t = x.get_item(1)?;
        out.push((op, if t.is_none() { None } else { Some(expanduser(&word(&t)?)) }));
    }
    Ok(out)
}

fn is_command_fn(v: &Bound<'_, PyAny>) -> bool {
    v.hasattr("_tessh_command").unwrap_or(false)
}

fn resolve_callable<'py>(py: Python<'py>, ns: &Bound<'py, PyDict>, name: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
    if name.contains(['.', '/']) {
        return Ok(None);
    }
    let v = match ns.get_item(name)? {
        Some(v) => Some(v),
        None => builtins_dict(py, ns)?.get_item(name)?,
    };
    Ok(v.filter(|v| v.is_callable()))
}

/// A command: a Python function by that name if there is one (unless
/// forced), else a program. mode: attach, capture, bg or stage.
#[pyfunction]
#[pyo3(signature = (ns, head, args, redirs, mode="attach", force=false))]
fn cmd<'py>(py: Python<'py>, ns: Bound<'py, PyDict>, head: Bound<'py, PyAny>, args: Bound<'py, PyAny>, redirs: Bound<'py, PyAny>, mode: &str, force: bool) -> PyResult<Py<PyAny>> {
    let words = argv(py, &args)?;
    let red = redirs_of(&redirs)?;
    let head_s = word(&head)?;
    if !force {
        if let Some(f) = resolve_callable(py, &ns, &head_s)? {
            let kw = PyDict::new(py);
            if f.hasattr("_tessh_wants_ns")? {
                kw.set_item("_ns", &ns)?;
            }
            let mut fargs: Vec<Bound<'py, PyAny>> = vec![f.clone()];
            fargs.extend(words.iter().map(|w| PyString::new(py, w).into_any()));
            let fargs = PyTuple::new(py, fargs)?;
            if mode == "stage" {
                let partial = py.import("functools")?.getattr("partial")?;
                let p = partial.call(fargs, Some(&kw))?;
                return Ok(Py::new(py, call_stage(p.unbind()))?.into_any());
            }
            let thunk_f = f.clone();
            let kw2 = kw.clone();
            let call = move |py: Python<'py>| -> PyResult<Bound<'py, PyAny>> { thunk_f.call(PyTuple::new(py, &words)?, Some(&kw2)) };
            if mode == "bg" {
                let partial = py.import("functools")?.getattr("partial")?;
                let p = partial.call(fargs, Some(&kw))?;
                return bg_call(py, p);
            }
            return with_redirs(py, &red, || call(py)).map(|v| v.unbind());
        }
    }
    let mut all = vec![head_s];
    all.extend(words);
    let stage = Stage { kind: "cmd", words: all, redirs: red, obj: None };
    if mode == "stage" {
        return Ok(Py::new(py, stage)?.into_any());
    }
    run_stages(py, vec![stage], mode)
}

/// Run a pipe: programs, Python calls (given what came before as their last
/// argument) and, first, a value to feed in.
#[pyfunction]
#[pyo3(signature = (stages, mode="attach"))]
fn pipe(py: Python<'_>, stages: Vec<Bound<'_, PyAny>>, mode: &str) -> PyResult<Py<PyAny>> {
    let mut ss = vec![];
    for s in stages {
        match s.extract::<PyRef<Stage>>() {
            Ok(st) => ss.push(Stage { kind: st.kind, words: st.words.clone(), redirs: st.redirs.clone(), obj: st.obj.as_ref().map(|o| o.clone_ref(py)) }),
            Err(_) => ss.push(Stage { kind: "value", words: vec![], redirs: vec![], obj: Some(s.unbind()) }),
        }
    }
    run_stages(py, ss, mode)
}

fn run_stages(py: Python<'_>, stages: Vec<Stage>, mode: &str) -> PyResult<Py<PyAny>> {
    let capture = mode == "capture";
    if mode == "bg" {
        if stages.iter().all(|s| s.kind == "cmd") {
            return run_procs(py, &stages, None, false, true);
        }
        // (A pipe with Python in it: in a thread.)
        let tessh = py.import("__tessh__")?;
        let objs: Vec<Py<PyAny>> = stages.into_iter().map(|s| Py::new(py, s).map(|x| x.into_any())).collect::<PyResult<_>>()?;
        let partial = py.import("functools")?.getattr("partial")?;
        let p = partial.call1((tessh.getattr("pipe")?, objs, "capture"))?;
        return bg_call(py, p);
    }
    let mut data: Option<Py<PyAny>> = None;
    let mut result: Py<PyAny> = py.None();
    let mut i = 0;
    while i < stages.len() {
        let s = &stages[i];
        match s.kind {
            "value" => {
                let v = s.obj.as_ref().unwrap().clone_ref(py);
                data = Some(v.clone_ref(py));
                result = v;
                i += 1;
            }
            "call" => {
                let arg = data.take().unwrap_or_else(|| py.None());
                let v = s.obj.as_ref().unwrap().call1(py, (arg,))?;
                data = Some(v.clone_ref(py));
                result = v;
                STATUS.store(0, Ordering::SeqCst);
                i += 1;
            }
            _ => {
                let mut j = i;
                while j < stages.len() && stages[j].kind == "cmd" {
                    j += 1;
                }
                let last = j == stages.len();
                let r = run_procs(py, &stages[i..j], data.take(), capture || !last, false)?;
                data = Some(r.clone_ref(py));
                result = r;
                i = j;
            }
        }
    }
    Ok(result)
}

fn feed_bytes(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if let Ok(b) = data.cast::<PyBytes>() {
        return Ok(b.as_bytes().to_vec());
    }
    if let Ok(s) = data.cast::<PyString>() {
        return Ok(s.to_string().into_bytes());
    }
    if data.is_none() {
        return Ok(vec![]);
    }
    match data.try_iter() {
        Ok(it) => {
            let mut out = vec![];
            for x in it {
                out.extend(word(&x?)?.into_bytes());
                out.push(b'\n');
            }
            let _ = py;
            Ok(out)
        }
        Err(_) => Ok(data.str()?.to_string().into_bytes()),
    }
}

fn flush_python(py: Python<'_>) {
    if let Ok(sys) = py.import("sys") {
        for s in ["stdout", "stderr"] {
            if let Ok(f) = sys.getattr(s) {
                let _ = f.call_method0("flush");
            }
        }
    }
}

fn tty_ok() -> bool {
    INTERACTIVE.load(Ordering::SeqCst) && unsafe { libc::isatty(0) == 1 && libc::tcgetpgrp(0) == libc::getpgrp() }
}

enum Out {
    Inherit,
    Pipe(std::io::PipeWriter),
    File(File),
}

fn open_redir(op: &str, target: &Option<String>) -> PyResult<File> {
    let t = target.clone().ok_or_else(|| PyValueError::new_err(format!("{op} needs a file")))?;
    let r = if op == "<" {
        File::open(&t)
    } else if op.ends_with(">>") {
        OpenOptions::new().create(true).append(true).open(&t)
    } else {
        File::create(&t)
    };
    r.map_err(|e| PyOSError::new_err(format!("{t}: {e}")))
}

fn to_stdio(o: &Out) -> PyResult<Stdio> {
    Ok(match o {
        Out::Inherit => Stdio::inherit(),
        Out::Pipe(w) => Stdio::from(w.try_clone()?),
        Out::File(f) => Stdio::from(f.try_clone()?),
    })
}

/// Run a chain of programs connected by pipes.
fn run_procs(py: Python<'_>, cmds: &[Stage], data: Option<Py<PyAny>>, capture: bool, bg: bool) -> PyResult<Py<PyAny>> {
    flush_python(py);
    let pass = PASS_THROUGH.load(Ordering::SeqCst) && !capture && !bg;
    let capture = capture || pass;
    let job_control = tty_ok() && !capture;
    let feed = match &data {
        Some(d) => Some(feed_bytes(py, d.bind(py))?),
        None => None,
    };
    let mut children: Vec<Child> = vec![];
    let mut pgid: i32 = 0;
    let mut prev_reader: Option<std::io::PipeReader> = None;
    let mut cap_reader: Option<std::io::PipeReader> = None;
    let mut first_stdin: Option<std::process::ChildStdin> = None;
    for (k, c) in cmds.iter().enumerate() {
        let last = k + 1 == cmds.len();
        let mut command = Command::new(&c.words[0]);
        command.args(&c.words[1..]);
        // stdin
        let mut stdin: Option<Stdio> = None;
        if let Some(r) = prev_reader.take() {
            stdin = Some(Stdio::from(r));
        } else if k == 0 && feed.is_some() {
            stdin = Some(Stdio::piped());
        }
        // stdout
        let mut out = Out::Inherit;
        let mut reader = None;
        if !last || capture {
            let (r, w) = std::io::pipe()?;
            reader = Some(r);
            out = Out::Pipe(w);
        }
        let mut err = Out::Inherit;
        let mut err_to_out = false;
        for (op, target) in &c.redirs {
            let op = op.strip_prefix('1').filter(|o| o.starts_with('>')).unwrap_or(op);
            match op {
                ">" | ">>" => out = Out::File(open_redir(op, target)?),
                "<" => stdin = Some(Stdio::from(open_redir(op, target)?)),
                "2>" | "2>>" => err = Out::File(open_redir(&op[1..], target)?),
                "&>" | "&>>" => {
                    let f = open_redir(&op[1..], target)?;
                    err = Out::File(f.try_clone()?);
                    out = Out::File(f);
                }
                "2>&1" | "&>&1" => err_to_out = true,
                ">&2" => out = Out::File(unsafe { <File as std::os::fd::FromRawFd>::from_raw_fd(libc::dup(2)) }),
                _ => {}
            }
        }
        if let Some(s) = stdin {
            command.stdin(s);
        }
        command.stdout(to_stdio(&out)?);
        command.stderr(if err_to_out {
            match &out {
                Out::Inherit => Stdio::from(unsafe { <File as std::os::fd::FromRawFd>::from_raw_fd(libc::dup(1)) }),
                o => to_stdio(o)?,
            }
        } else if pass && matches!(err, Out::Inherit) {
            // (The kernel: stderr through Python's too, after the run.)
            Stdio::inherit()
        } else {
            to_stdio(&err)?
        });
        if job_control || bg {
            command.process_group(pgid);
        }
        drop(out);
        match command.spawn() {
            Ok(mut ch) => {
                if pgid == 0 && (job_control || bg) {
                    pgid = ch.id() as i32;
                }
                if k == 0 {
                    first_stdin = ch.stdin.take();
                }
                children.push(ch);
            }
            Err(e) => {
                for ch in children.iter_mut() {
                    let _ = ch.kill();
                }
                let (code, what) = match e.kind() {
                    std::io::ErrorKind::NotFound => (127, "command not found"),
                    std::io::ErrorKind::PermissionDenied => (126, "permission denied"),
                    _ => (126, "can't run"),
                };
                eprintln!("tessh: {what}: {}", c.words[0]);
                STATUS.store(code, Ordering::SeqCst);
                return output(py, "", code, !capture);
            }
        }
        // (The parent's copies of the pipe's write end must close.)
        drop(command);
        if last && capture {
            cap_reader = reader;
        } else {
            prev_reader = reader;
        }
    }
    if let (Some(bytes), Some(mut w)) = (feed, first_stdin.take()) {
        std::thread::spawn(move || {
            let _ = w.write_all(&bytes);
        });
    }
    if bg {
        let words = cmds[0].words.join(" ");
        let job = add_job(children, pgid, words.clone(), false);
        if INTERACTIVE.load(Ordering::SeqCst) {
            eprintln!("[{job}] {pgid}");
        }
        return Ok(Py::new(py, JobRef { n: job })?.into_any());
    }
    if let Some(mut r) = cap_reader {
        let mut buf = vec![];
        let (code, children) = py.detach(|| {
            let _ = r.read_to_end(&mut buf);
            let mut children = children;
            let mut code = 0;
            for ch in children.iter_mut() {
                code = ch.wait().map(exit_code).unwrap_or(1);
            }
            (code, children)
        });
        drop(children);
        STATUS.store(code, Ordering::SeqCst);
        let mut text = decode(py, &buf)?;
        if pass {
            let sys = py.import("sys")?;
            sys.getattr("stdout")?.call_method1("write", (text.as_str(),))?;
            flush_python(py);
            py.check_signals()?;
            return output(py, "", code, true);
        }
        if text.ends_with('\n') {
            text.pop();
        }
        py.check_signals()?;
        return output(py, &text, code, false);
    }
    let code = if job_control {
        let words = cmds[0].words.join(" ");
        py.detach(|| wait_foreground(children, pgid, words))
    } else {
        py.detach(|| {
            let mut code = 0;
            let mut children = children;
            for ch in children.iter_mut() {
                code = ch.wait().map(exit_code).unwrap_or(1);
            }
            code
        })
    };
    STATUS.store(code, Ordering::SeqCst);
    py.check_signals()?;
    if code == 130 && job_control {
        return Err(pyo3::exceptions::PyKeyboardInterrupt::new_err(""));
    }
    output(py, "", code, true)
}

fn exit_code(s: std::process::ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    match s.code() {
        Some(c) => c,
        None => 128 + s.signal().unwrap_or(0),
    }
}

/// Give the terminal to the programs, wait (or see them stopped), take it back.
fn wait_foreground(children: Vec<Child>, pgid: i32, words: String) -> i32 {
    unsafe {
        let old = libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::tcsetpgrp(0, pgid);
        let pids: Vec<i32> = children.iter().map(|c| c.id() as i32).collect();
        let last = *pids.last().unwrap_or(&0);
        let mut left: std::collections::HashSet<i32> = pids.iter().copied().collect();
        let mut code = 0;
        let mut stopped = false;
        while !left.is_empty() {
            let mut st = 0;
            let pid = libc::waitpid(-pgid, &mut st, libc::WUNTRACED);
            if pid < 0 {
                if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                break;
            }
            if libc::WIFSTOPPED(st) {
                stopped = true;
                break;
            }
            left.remove(&pid);
            if pid == last {
                code = if libc::WIFEXITED(st) { libc::WEXITSTATUS(st) } else { 128 + libc::WTERMSIG(st) };
            }
        }
        libc::tcsetpgrp(0, libc::getpgrp());
        libc::signal(libc::SIGTTOU, old);
        if stopped {
            let n = add_job(children, pgid, words.clone(), true);
            eprintln!("\n[{n}] stopped {words}");
            return 148;
        }
        // (Reaped already: the Child handles must not wait again.)
        std::mem::forget(children);
        code
    }
}

// ---- jobs -----------------------------------------------------------------------------

struct JobInfo {
    n: usize,
    pgid: i32,
    pids: Vec<i32>,
    words: String,
    stopped: bool,
    thread: Option<Py<PyAny>>,
    children: Vec<Child>,
}

static JOBS: Mutex<Vec<JobInfo>> = Mutex::new(Vec::new());
static JOB_N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn add_job(children: Vec<Child>, pgid: i32, words: String, stopped: bool) -> usize {
    let n = JOB_N.fetch_add(1, Ordering::SeqCst) + 1;
    let pids = children.iter().map(|c| c.id() as i32).collect();
    JOBS.lock().unwrap().push(JobInfo { n, pgid, pids, words, stopped, thread: None, children });
    n
}

fn job_done(j: &mut JobInfo, py: Python<'_>) -> bool {
    if let Some(t) = &j.thread {
        return !t.call_method0(py, "is_alive").and_then(|v| v.extract::<bool>(py)).unwrap_or(false);
    }
    !j.stopped && j.children.iter_mut().all(|c| c.try_wait().map(|s| s.is_some()).unwrap_or(true))
}

#[pyclass(module = "__tessh__", name = "Job")]
pub struct JobRef {
    #[pyo3(get)]
    n: usize,
}

#[pymethods]
impl JobRef {
    /// Wait for it; its result (a Python job) or exit status.
    fn wait(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fg(py, Some(self.n.to_string()))
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let mut jobs = JOBS.lock().unwrap();
        match jobs.iter_mut().find(|j| j.n == self.n) {
            Some(j) => {
                let what = if job_done(j, py) { "done" } else if j.stopped { "stopped" } else { "running" };
                format!("[{}] {what} {}", j.n, j.words)
            }
            None => format!("[{}] done", self.n),
        }
    }
}

fn bg_call(py: Python<'_>, thunk: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let threading = py.import("threading")?;
    let holder = PyList::empty(py);
    let code = std::ffi::CString::new("def _run(f, out):\n    out.append(f())\n").unwrap();
    let helper = PyModule::from_code(py, &code, c"<tessh bg>", c"__tessh_bg__")?.getattr("_run")?;
    let kw = PyDict::new(py);
    kw.set_item("target", helper)?;
    kw.set_item("args", (thunk, &holder))?;
    kw.set_item("daemon", true)?;
    let th = threading.getattr("Thread")?.call((), Some(&kw))?;
    th.call_method0("start")?;
    let n = JOB_N.fetch_add(1, Ordering::SeqCst) + 1;
    th.setattr("_tessh_result", holder)?;
    JOBS.lock().unwrap().push(JobInfo { n, pgid: 0, pids: vec![], words: "(python)".into(), stopped: false, thread: Some(th.unbind()), children: vec![] });
    Ok(Py::new(py, JobRef { n })?.into_any())
}

#[pyfunction]
#[pyo3(name = "bg_call")]
fn py_bg_call(py: Python<'_>, thunk: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    bg_call(py, thunk)
}

// ---- redirections of Python calls ------------------------------------------------------

fn with_redirs<'py, T>(py: Python<'py>, redirs: &[(String, Option<String>)], f: impl FnOnce() -> PyResult<T>) -> PyResult<T> {
    if redirs.is_empty() {
        return f();
    }
    let sys = py.import("sys")?;
    let open = py.import("builtins")?.getattr("open")?;
    let saved = (sys.getattr("stdout")?, sys.getattr("stderr")?, sys.getattr("stdin")?);
    let mut opened = vec![];
    for (op, target) in redirs {
        let op = op.strip_prefix('1').filter(|o| o.starts_with('>')).unwrap_or(op);
        let file = |mode: &str| -> PyResult<Bound<'py, PyAny>> {
            let t = target.clone().ok_or_else(|| PyValueError::new_err(format!("{op} needs a file")))?;
            open.call1((t, mode))
        };
        match op {
            ">" | ">>" => {
                let fl = file(if op == ">>" { "a" } else { "w" })?;
                sys.setattr("stdout", &fl)?;
                opened.push(fl);
            }
            "&>" | "&>>" => {
                let fl = file(if op == "&>>" { "a" } else { "w" })?;
                sys.setattr("stdout", &fl)?;
                sys.setattr("stderr", &fl)?;
                opened.push(fl);
            }
            "2>" | "2>>" => {
                let fl = file(if op == "2>>" { "a" } else { "w" })?;
                sys.setattr("stderr", &fl)?;
                opened.push(fl);
            }
            "2>&1" => sys.setattr("stderr", sys.getattr("stdout")?)?,
            ">&2" => sys.setattr("stdout", sys.getattr("stderr")?)?,
            "<" => {
                let fl = file("r")?;
                sys.setattr("stdin", &fl)?;
                opened.push(fl);
            }
            _ => {}
        }
    }
    let r = f();
    sys.setattr("stdout", saved.0)?;
    sys.setattr("stderr", saved.1)?;
    sys.setattr("stdin", saved.2)?;
    for fl in opened {
        let _ = fl.call_method0("close");
    }
    r
}

#[pyfunction]
fn redirect(py: Python<'_>, thunk: Bound<'_, PyAny>, redirs: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let red = redirs_of(&redirs)?;
    with_redirs(py, &red, || thunk.call0()).map(|v| v.unbind())
}

// ---- the environment -------------------------------------------------------------------

/// $env: the environment, by attribute or by key.
#[pyclass(module = "__tessh__", name = "Env")]
pub struct Env;

#[pymethods]
impl Env {
    fn __getattr__(&self, k: &str) -> PyResult<Option<String>> {
        if k.starts_with("__") {
            return Err(pyo3::exceptions::PyAttributeError::new_err(k.to_string()));
        }
        Ok(std::env::var(k).ok())
    }
    fn __setattr__(&self, k: &str, v: Bound<'_, PyAny>) -> PyResult<()> {
        unsafe { std::env::set_var(k, word(&v)?) };
        sync_os_environ(v.py(), k)
    }
    fn __delattr__(&self, py: Python<'_>, k: &str) -> PyResult<()> {
        unsafe { std::env::remove_var(k) };
        sync_os_environ(py, k)
    }
    fn __getitem__(&self, k: &str) -> PyResult<String> {
        std::env::var(k).map_err(|_| pyo3::exceptions::PyKeyError::new_err(k.to_string()))
    }
    fn __setitem__(&self, k: &str, v: Bound<'_, PyAny>) -> PyResult<()> {
        self.__setattr__(k, v)
    }
    fn __delitem__(&self, py: Python<'_>, k: &str) -> PyResult<()> {
        self.__delattr__(py, k)
    }
    fn __contains__(&self, k: &str) -> bool {
        std::env::var_os(k).is_some()
    }
    fn __len__(&self) -> usize {
        std::env::vars_os().count()
    }
    fn __iter__(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let keys: Vec<String> = std::env::vars_os().map(|(k, _)| k.to_string_lossy().to_string()).collect();
        Ok(PyList::new(py, keys)?.call_method0("__iter__")?.unbind())
    }
    #[pyo3(signature = (k, default=None))]
    fn get(&self, py: Python<'_>, k: &str, default: Option<Py<PyAny>>) -> Py<PyAny> {
        match std::env::var(k) {
            Ok(v) => PyString::new(py, &v).into_any().unbind(),
            Err(_) => default.unwrap_or_else(|| py.None()),
        }
    }
    fn items(&self) -> Vec<(String, String)> {
        std::env::vars_os().map(|(k, v)| (k.to_string_lossy().to_string(), v.to_string_lossy().to_string())).collect()
    }
    fn __repr__(&self) -> String {
        format!("<env: {} variables>", std::env::vars_os().count())
    }
}

/// Keep Python's os.environ (its own copy) in step with the process's.
fn sync_os_environ(py: Python<'_>, k: &str) -> PyResult<()> {
    let env = py.import("os")?.getattr("environ")?;
    let data = env.getattr("_data")?;
    let key = env.call_method1("encodekey", (k,))?;
    match std::env::var(k) {
        Ok(v) => data.set_item(key, env.call_method1("encodevalue", (v,))?)?,
        Err(_) => {
            let _ = data.del_item(key);
        }
    }
    Ok(())
}

pub fn setenv(py: Python<'_>, k: &str, v: Option<&str>) -> PyResult<()> {
    match v {
        Some(v) => py.import("os")?.getattr("environ")?.set_item(k, v)?,
        None => {
            let env = py.import("os")?.getattr("environ")?;
            if env.contains(k)? {
                env.del_item(k)?;
            }
        }
    }
    Ok(())
}

fn chdir(py: Python<'_>, d: &str) -> PyResult<()> {
    let old = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    std::env::set_current_dir(d).map_err(|e| PyOSError::new_err(format!("cd: {d}: {e}")))?;
    *OLDPWD.lock().unwrap() = Some(old.clone());
    setenv(py, "OLDPWD", Some(&old))?;
    let new = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    setenv(py, "PWD", Some(&new))
}

fn b64d(s: &str) -> PyResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s.trim_end_matches('=')).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// From Tessera: variables set and unset, and a folder (base64 JSON).
#[pyfunction]
fn apply_env(py: Python<'_>, b64: &str) -> PyResult<()> {
    let v: serde_json::Value = serde_json::from_slice(&b64d(b64)?).map_err(|e| PyValueError::new_err(e.to_string()))?;
    apply_json(py, &v)
}

pub fn apply_json(py: Python<'_>, v: &serde_json::Value) -> PyResult<()> {
    if let Some(set) = v["set"].as_object() {
        for (k, x) in set {
            setenv(py, k, x.as_str())?;
        }
    }
    if let Some(un) = v["unset"].as_array() {
        for k in un.iter().filter_map(|x| x.as_str()) {
            setenv(py, k, None)?;
        }
    }
    if let Some(d) = v["cd"].as_str().filter(|d| !d.is_empty()) {
        chdir(py, d)?;
    }
    Ok(())
}

/// For Tessera: the environment and folder, written to two files.
#[pyfunction]
fn dump_env(b64_env: &str, b64_pwd: &str) -> PyResult<()> {
    let e = String::from_utf8_lossy(&b64d(b64_env)?).to_string();
    let w = String::from_utf8_lossy(&b64d(b64_pwd)?).to_string();
    let mut out = Vec::new();
    for (k, v) in std::env::vars_os() {
        out.extend(k.as_encoded_bytes());
        out.push(b'=');
        out.extend(v.as_encoded_bytes());
        out.push(0);
    }
    std::fs::write(e, out)?;
    std::fs::write(w, std::env::current_dir()?.to_string_lossy().as_bytes())?;
    Ok(())
}

// ---- builtins ------------------------------------------------------------------------------

pub const COMMAND_BUILTINS: &[&str] = &["cd", "pushd", "popd", "dirs", "export", "unset", "exit", "source", "which", "glob", "jobs", "fg", "bg"];

pub fn is_command_builtin(n: &str) -> bool {
    COMMAND_BUILTINS.contains(&n)
}

/// A shell builtin: a command function (its arguments are read as words).
#[pyclass(module = "__tessh__", name = "Builtin", frozen)]
pub struct Builtin {
    name: &'static str,
}

#[pymethods]
impl Builtin {
    #[getter]
    fn _tessh_command(&self) -> bool {
        true
    }

    #[getter]
    fn _tessh_wants_ns(&self) -> PyResult<bool> {
        if self.name == "export" || self.name == "source" {
            Ok(true)
        } else {
            Err(pyo3::exceptions::PyAttributeError::new_err("_tessh_wants_ns"))
        }
    }

    #[pyo3(signature = (*args, _ns=None))]
    fn __call__(&self, py: Python<'_>, args: Vec<Bound<'_, PyAny>>, _ns: Option<Bound<'_, PyDict>>) -> PyResult<Py<PyAny>> {
        let words: Vec<String> = args.iter().map(word).collect::<PyResult<_>>()?;
        let none = py.None();
        match self.name {
            "cd" => {
                if words.len() > 1 {
                    return Err(PyTypeError::new_err("cd: too many arguments"));
                }
                let mut d = words.first().cloned().unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/".into()));
                if d == "-" {
                    d = OLDPWD.lock().unwrap().clone().or_else(|| std::env::var("OLDPWD").ok()).unwrap_or_else(|| ".".into());
                    println!("{d}");
                }
                chdir(py, &expanduser(&d))?;
                Ok(none)
            }
            "pushd" => {
                let here = std::env::current_dir()?.to_string_lossy().to_string();
                let d = match words.first() {
                    Some(d) => expanduser(d),
                    None => DIRSTACK.lock().unwrap().pop().unwrap_or_else(|| std::env::var("HOME").unwrap_or_default()),
                };
                chdir(py, &d)?;
                DIRSTACK.lock().unwrap().push(here);
                print_dirs();
                Ok(none)
            }
            "popd" => {
                let d = DIRSTACK.lock().unwrap().pop().ok_or_else(|| PyIndexError::new_err("popd: the folder stack is empty"))?;
                chdir(py, &d)?;
                print_dirs();
                Ok(none)
            }
            "dirs" => {
                print_dirs();
                Ok(none)
            }
            "export" => {
                for a in &words {
                    match a.split_once('=') {
                        Some((k, v)) => setenv(py, k, Some(v))?,
                        None => {
                            let v = match &_ns {
                                Some(ns) => ns.get_item(a)?.map(|v| word(&v)).transpose()?,
                                None => None,
                            };
                            let v = v.or_else(|| std::env::var(a).ok()).unwrap_or_default();
                            setenv(py, a, Some(&v))?;
                        }
                    }
                }
                Ok(none)
            }
            "unset" => {
                for a in &words {
                    setenv(py, a, None)?;
                }
                Ok(none)
            }
            "exit" => {
                let code: i32 = words.first().map(|c| c.parse().unwrap_or(1)).unwrap_or(0);
                Err(PySystemExit::new_err(code))
            }
            "source" => {
                let path = words.first().ok_or_else(|| PyTypeError::new_err("source: which file?"))?;
                let ns = _ns.ok_or_else(|| PyTypeError::new_err("source: no namespace"))?;
                crate::runner::run_file(py, &expanduser(path), &ns).map_err(|e| e.into_pyerr(py))?;
                Ok(none)
            }
            "which" => {
                let mut found = None;
                for n in &words {
                    let p = which(n);
                    match &p {
                        Some(p) => println!("{n}: {p}"),
                        None => println!("{n}: not found"),
                    }
                    found = found.or(p);
                }
                output(py, found.as_deref().unwrap_or(""), if found.is_some() { 0 } else { 1 }, false)
            }
            "glob" => Ok(PyList::new(py, words)?.into_any().unbind()),
            "jobs" => {
                let mut jobs = JOBS.lock().unwrap();
                jobs.retain_mut(|j| j.stopped || !job_done(j, py));
                for j in jobs.iter() {
                    println!("[{}] {} {}", j.n, if j.stopped { "stopped" } else { "running" }, j.words);
                }
                Ok(none)
            }
            "fg" => fg(py, words.first().cloned()),
            "bg" => {
                let mut jobs = JOBS.lock().unwrap();
                let j = pick_job(&mut jobs, words.first())?;
                j.stopped = false;
                unsafe { libc::killpg(j.pgid, libc::SIGCONT) };
                Ok(none)
            }
            _ => Ok(none),
        }
    }

    fn __repr__(&self) -> String {
        format!("<tessh builtin {}>", self.name)
    }
}

fn print_dirs() {
    let here = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let stack = DIRSTACK.lock().unwrap();
    let mut v = vec![here];
    v.extend(stack.iter().rev().cloned());
    println!("{}", v.join(" "));
}

fn pick_job<'a>(jobs: &'a mut [JobInfo], n: Option<&String>) -> PyResult<&'a mut JobInfo> {
    match n {
        None => jobs.last_mut().ok_or_else(|| PyIndexError::new_err("no jobs")),
        Some(n) => {
            let k: usize = n.trim_start_matches('%').parse().map_err(|_| PyValueError::new_err(format!("no job {n}")))?;
            jobs.iter_mut().find(|j| j.n == k).ok_or_else(|| PyIndexError::new_err(format!("no job {n}")))
        }
    }
}

fn fg(py: Python<'_>, n: Option<String>) -> PyResult<Py<PyAny>> {
    let job = {
        let mut jobs = JOBS.lock().unwrap();
        let k = pick_job(&mut jobs, n.as_ref())?.n;
        let i = jobs.iter().position(|j| j.n == k).unwrap();
        jobs.remove(i)
    };
    if let Some(th) = job.thread {
        let th = th.bind(py);
        py.detach(|| ());
        th.call_method0("join")?;
        let res = th.getattr("_tessh_result")?;
        return Ok(if res.len()? > 0 { res.get_item(0)?.unbind() } else { py.None() });
    }
    unsafe { libc::killpg(job.pgid, libc::SIGCONT) };
    let words = job.words.clone();
    let code = if tty_ok() {
        py.detach(|| wait_foreground(job.children, job.pgid, words))
    } else {
        let mut children = job.children;
        py.detach(|| {
            let mut code = 0;
            for c in children.iter_mut() {
                code = c.wait().map(exit_code).unwrap_or(0);
            }
            code
        })
    };
    let _ = job.pids;
    STATUS.store(code, Ordering::SeqCst);
    output(py, "", code, true)
}

pub fn which(n: &str) -> Option<String> {
    if n.contains('/') {
        return std::path::Path::new(n).is_file().then(|| n.to_string());
    }
    let path = std::env::var_os("PATH")?;
    for d in std::env::split_paths(&path) {
        let p = d.join(n);
        if let Ok(m) = std::fs::metadata(&p) {
            use std::os::unix::fs::PermissionsExt;
            if m.is_file() && m.permissions().mode() & 0o111 != 0 {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    None
}

/// Make a function a command function: its arguments are read as words.
#[pyfunction]
fn command(f: Bound<'_, PyAny>) -> PyResult<Bound<'_, PyAny>> {
    f.setattr("_tessh_command", true)?;
    Ok(f)
}

pub fn module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "__tessh__")?;
    m.add_class::<Stage>()?;
    m.add_class::<JobRef>()?;
    m.add_class::<Env>()?;
    m.add_class::<Builtin>()?;
    for f in [
        wrap_pyfunction!(lookup, &m)?,
        wrap_pyfunction!(free, &m)?,
        wrap_pyfunction!(auto, &m)?,
        wrap_pyfunction!(show, &m)?,
        wrap_pyfunction!(status, &m)?,
        wrap_pyfunction!(g, &m)?,
        wrap_pyfunction!(t, &m)?,
        wrap_pyfunction!(cmd, &m)?,
        wrap_pyfunction!(pipe, &m)?,
        wrap_pyfunction!(call_stage, &m)?,
        wrap_pyfunction!(value_stage, &m)?,
        wrap_pyfunction!(py_bg_call, &m)?,
        wrap_pyfunction!(redirect, &m)?,
        wrap_pyfunction!(apply_env, &m)?,
        wrap_pyfunction!(dump_env, &m)?,
        wrap_pyfunction!(command, &m)?,
    ] {
        m.add_function(f)?;
    }
    m.add("Output", prelude(py)?.getattr("Output")?)?;
    m.add("env", Py::new(py, Env)?)?;
    for n in COMMAND_BUILTINS {
        m.add(*n, Py::new(py, Builtin { name: n })?)?;
    }
    py.import("sys")?.getattr("modules")?.set_item("__tessh__", &m)?;
    Ok(m)
}

/// The names every tessh namespace gets.
pub fn install(py: Python<'_>, ns: &Bound<'_, PyDict>) -> PyResult<()> {
    let m = py.import("__tessh__")?;
    for n in COMMAND_BUILTINS.iter().chain(["command", "env", "Output"].iter()) {
        if !ns.contains(*n)? {
            ns.set_item(*n, m.getattr(*n)?)?;
        }
    }
    let _ = PyBytes::new(py, b"");
    Ok(())
}
