//! Running tessh in the embedded Python: namespaces with tessh's names, and
//! source run form by form (each top-level form compiled, then run, so
//! macros and names defined earlier are there for the next).

use crate::compiler::{CompileError, Compiler, Ctx, State};
use crate::forms::Form;
use crate::reader::{char_cols, read_at, ReadError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::sync::Mutex;

pub enum TesshError {
    Read(ReadError),
    Compile(CompileError, String),
    Py(PyErr),
}

impl TesshError {
    /// (line, column in characters, end column, message), for a syntax error.
    pub fn position(&self) -> Option<(u32, u32, u32, String)> {
        match self {
            TesshError::Read(e) => Some((e.line, e.col, e.end_col, e.msg.clone())),
            TesshError::Compile(e, text) => {
                let p = e.pos.unwrap_or_default();
                let line = p.line.max(1);
                let col = char_cols(text, line, p.col);
                let end = if p.end_line == p.line && p.end_col > p.col { char_cols(text, line, p.end_col) } else { col + 1 };
                Some((line, col, end, e.msg.clone()))
            }
            TesshError::Py(_) => None,
        }
    }

    pub fn into_pyerr(self, py: Python<'_>) -> PyErr {
        match self {
            TesshError::Py(e) => e,
            other => {
                let (l, c, _, m) = other.position().unwrap();
                let _ = py;
                pyo3::exceptions::PySyntaxError::new_err(format!("{l}:{}: {m}", c + 1))
            }
        }
    }
}

impl From<PyErr> for TesshError {
    fn from(e: PyErr) -> Self {
        TesshError::Py(e)
    }
}

/// The compiler's state for a namespace (macros, operators, names).
#[pyclass(module = "__tessh__")]
pub struct StateBox {
    pub st: Mutex<State>,
}

/// Python, set up for tessh: its modules registered, Ctrl+C a
/// KeyboardInterrupt.
pub fn init(py: Python<'_>) -> PyResult<()> {
    if py.import("sys")?.getattr("modules")?.contains("__tessh__")? {
        return Ok(());
    }
    crate::runtime::module(py)?;
    let fm = crate::pyforms::module(py)?;
    py.import("sys")?.getattr("modules")?.set_item("__tessh_forms__", fm)?;
    let signal = py.import("signal")?;
    signal.getattr("signal")?.call1((signal.getattr("SIGINT")?, signal.getattr("default_int_handler")?))?;
    Ok(())
}

pub fn new_namespace<'py>(py: Python<'py>, name: &str, argv: Vec<String>) -> PyResult<Bound<'py, PyDict>> {
    init(py)?;
    let ns = PyDict::new(py);
    ns.set_item("__name__", name)?;
    ns.set_item("__builtins__", py.import("builtins")?)?;
    install(py, &ns)?;
    ns.set_item("argv", PyList::new(py, argv)?)?;
    Ok(ns)
}

/// Put tessh's names in a namespace (a fresh one, or the kernel's).
pub fn install(py: Python<'_>, ns: &Bound<'_, PyDict>) -> PyResult<()> {
    init(py)?;
    ns.set_item("__tessh__", py.import("__tessh__")?)?;
    let fm = py.import("__tessh_forms__")?;
    ns.set_item("__tessh_forms__", &fm)?;
    crate::runtime::install(py, ns)?;
    for n in crate::pyforms::MACRO_HELPERS {
        if !ns.contains(*n)? {
            ns.set_item(*n, fm.getattr(*n)?)?;
        }
    }
    if !ns.contains("argv")? {
        ns.set_item("argv", PyList::empty(py))?;
    }
    if !ns.contains("__tessh_state__")? {
        ns.set_item("__tessh_state__", Py::new(py, StateBox { st: Mutex::new(State::default()) })?)?;
    }
    if !ns.contains("macroexpand")? {
        let nsref = ns.clone().unbind();
        let f = pyo3::types::PyCFunction::new_closure(py, Some(c"macroexpand"), Some(c"The form a macro call expands to (one step)."), move |args, _kw| -> PyResult<Py<PyAny>> {
            let py = args.py();
            let form = crate::pyforms::lift(&args.get_item(0)?)?;
            let ns = nsref.bind(py);
            let sb = state_of(ns)?;
            let sb = sb.borrow();
            let st = sb.st.lock().unwrap();
            let (items, children) = match &form.kind {
                crate::forms::Kind::Line { items, children } => (items.clone(), children.clone()),
                _ => (vec![form.clone()], vec![]),
            };
            let Some(m) = items.first().and_then(|h| h.sym_name()).and_then(|n| st.macros.get(n)) else { return crate::pyforms::to_py(py, form) };
            let mut stub = State::default();
            let c = Compiler::new(py, None, &mut stub)?;
            let a = c.macro_args(m, &items[1..], &children, &items[0]).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.msg))?;
            let a: Vec<Py<PyAny>> = a.into_iter().map(|f| crate::pyforms::to_py(py, f)).collect::<PyResult<_>>()?;
            Ok(m.func.bind(py).call1(pyo3::types::PyTuple::new(py, a)?)?.unbind())
        })?;
        ns.set_item("macroexpand", f)?;
    }
    Ok(())
}

pub fn scratch_namespace(py: Python<'_>) -> PyResult<Bound<'_, PyDict>> {
    static SCRATCH: pyo3::sync::PyOnceLock<Py<PyDict>> = pyo3::sync::PyOnceLock::new();
    Ok(SCRATCH.get_or_try_init(py, || new_namespace(py, "__tessh_check__", vec![]).map(|d| d.unbind()))?.bind(py).clone())
}

fn state_of<'py>(ns: &Bound<'py, PyDict>) -> PyResult<Bound<'py, StateBox>> {
    Ok(ns.get_item("__tessh_state__")?.ok_or_else(|| pyo3::exceptions::PyRuntimeError::new_err("not a tessh namespace"))?.cast_into::<StateBox>()?)
}

/// The macro names a namespace has (for the REPL's indentation).
pub fn macro_names(ns: &Bound<'_, PyDict>) -> Vec<String> {
    state_of(ns).map(|s| s.borrow().st.lock().unwrap().macros.keys().cloned().collect()).unwrap_or_default()
}

/// Let tracebacks show the tessh source of a cell or REPL entry.
fn remember_source(py: Python<'_>, filename: &str, text: &str) -> PyResult<()> {
    let lines: Vec<String> = text.split_inclusive('\n').map(String::from).collect();
    let lc = py.import("linecache")?;
    lc.getattr("cache")?.set_item(filename, (text.len(), py.None(), lines, filename))?;
    Ok(())
}

/// Run tessh source in ns. With show, the last form's value is shown (the
/// REPL, the kernel).
pub fn run_source(py: Python<'_>, text: &str, ns: &Bound<'_, PyDict>, filename: &str, show: bool) -> Result<(), TesshError> {
    install(py, ns)?;
    crate::deps::ensure_from_source(py, text)?;
    if filename.starts_with('<') {
        remember_source(py, filename, text)?;
    }
    let lines = read_at(text, 1).map_err(TesshError::Read)?;
    let sb = state_of(ns)?;
    let sb = sb.borrow();
    let builtins = py.import("builtins")?;
    let (compile, exec) = (builtins.getattr("compile")?, builtins.getattr("exec")?);
    let units = {
        let mut st = sb.st.lock().unwrap();
        let mut c = Compiler::new(py, Some(ns.clone()), &mut st)?;
        c.prescan_module(&lines);
        c.units_of(&lines).map_err(|e| TesshError::Compile(e, text.to_string()))?
    };
    let n = units.len();
    for (k, u) in units.iter().enumerate() {
        let last = k + 1 == n;
        let module = {
            let mut st = sb.st.lock().unwrap();
            let mut c = Compiler::new(py, Some(ns.clone()), &mut st)?;
            c.unit_module(u, if show && last { Ctx::Tail } else { Ctx::Stmt }, show && last).map_err(|e| TesshError::Compile(e, text.to_string()))?
        };
        let code = compile.call1((module, filename, "exec"))?;
        exec.call1((code, ns))?;
    }
    Ok(())
}

pub fn run_file(py: Python<'_>, path: &str, ns: &Bound<'_, PyDict>) -> Result<(), TesshError> {
    let mut text = std::fs::read_to_string(path).map_err(|e| TesshError::Py(pyo3::exceptions::PyOSError::new_err(format!("{path}: {e}"))))?;
    if text.starts_with("#!") {
        text.replace_range(0..1, "#");
    }
    if !ns.contains("__file__")? {
        ns.set_item("__file__", path)?;
    }
    run_source(py, &text, ns, path, false)
}

fn compile_all<'py>(py: Python<'py>, lines: &[Form], ns: Option<Bound<'py, PyDict>>, st: &mut State) -> Result<Bound<'py, PyAny>, CompileError> {
    let mut c = Compiler::new(py, ns, st)?;
    c.module(lines)
}

/// The Python a tessh text compiles to.
pub fn to_python(py: Python<'_>, text: &str) -> Result<String, TesshError> {
    init(py)?;
    let lines = read_at(text, 1).map_err(TesshError::Read)?;
    let mut st = State::default();
    let m = compile_all(py, &lines, None, &mut st).map_err(|e| TesshError::Compile(e, text.to_string()))?;
    Ok(py.import("ast")?.getattr("unparse")?.call1((m,))?.extract()?)
}

/// Problems in tessh source, without running it (macro definitions run):
/// [(line, col, end_col, message)], columns in characters.
pub fn check(py: Python<'_>, text: &str, filename: &str) -> Vec<(u32, u32, u32, String)> {
    if let Err(e) = init(py) {
        return vec![(1, 0, 1, e.to_string())];
    }
    let lines = match read_at(text, 1) {
        Ok(l) => l,
        Err(e) => return vec![(e.line, e.col, e.end_col, e.msg)],
    };
    let mut st = State::default();
    let m = match compile_all(py, &lines, None, &mut st) {
        Ok(m) => m,
        Err(e) => return vec![TesshError::Compile(e, text.to_string()).position().unwrap()],
    };
    let compile = py.import("builtins").and_then(|b| b.getattr("compile"));
    if let Ok(compile) = compile {
        if let Err(e) = compile.call1((m, filename, "exec")) {
            let v = e.value(py);
            let line: u32 = v.getattr("lineno").and_then(|x| x.extract()).unwrap_or(1);
            let off: u32 = v.getattr("offset").and_then(|x| x.extract()).unwrap_or(1);
            let msg: String = v.getattr("msg").and_then(|x| x.extract()).unwrap_or_else(|_| e.to_string());
            let col = char_cols(text, line, off.saturating_sub(1));
            return vec![(line, col, col + 1, msg)];
        }
    }
    vec![]
}

/// Print an error: tessh's own as file:line:col, a Python one as a traceback
/// (its frames point at tessh lines).
pub fn report(py: Python<'_>, e: TesshError, filename: &str) {
    match e {
        TesshError::Py(e) => {
            if e.is_instance_of::<pyo3::exceptions::PySyntaxError>(py) {
                eprintln!("tessh: {filename}: {}", e.value(py));
                return;
            }
            e.print(py);
        }
        other => {
            let (l, c, _, m) = other.position().unwrap();
            eprintln!("tessh: {filename}:{l}:{}: {m}", c + 1);
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub static LOCK: Mutex<()> = Mutex::new(());

    /// Run tessh; its namespace and what it printed (fd 1, so programs too).
    pub fn run(src: &str) -> (Py<PyDict>, String) {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        Python::attach(|py| {
            let ns = new_namespace(py, "__main__", vec![]).unwrap();
            let out = capture_fd1(py, || {
                if let Err(e) = run_source(py, src, &ns, "<test>", false) {
                    match e {
                        TesshError::Py(e) => panic!("{src}\n{}", e.value(py)),
                        e => panic!("{src}\n{:?}", e.position()),
                    }
                }
            });
            (ns.unbind(), out)
        })
    }

    pub fn capture_fd1(py: Python<'_>, f: impl FnOnce()) -> String {
        use std::io::Read;
        let _ = py.import("sys").and_then(|s| s.getattr("stdout")).and_then(|o| o.call_method0("flush"));
        let (mut r, w) = std::io::pipe().unwrap();
        let saved = unsafe { libc::dup(1) };
        unsafe { libc::dup2(std::os::fd::AsRawFd::as_raw_fd(&w), 1) };
        drop(w);
        let reader = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = r.read_to_string(&mut s);
            s
        });
        f();
        let _ = py.import("sys").and_then(|s| s.getattr("stdout")).and_then(|o| o.call_method0("flush"));
        unsafe {
            libc::dup2(saved, 1);
            libc::close(saved);
        }
        reader.join().unwrap()
    }

    pub fn get<T: for<'a> FromPyObject<'a, 'a>>(ns: &Py<PyDict>, k: &str) -> T
    where
        for<'a> <T as FromPyObject<'a, 'a>>::Error: std::fmt::Debug,
    {
        Python::attach(|py| {
            let v = ns.bind(py).get_item(k).unwrap().unwrap_or_else(|| panic!("no {k}"));
            v.extract::<T>().unwrap()
        })
    }

    pub fn repr(ns: &Py<PyDict>, k: &str) -> String {
        Python::attach(|py| ns.bind(py).get_item(k).unwrap().unwrap_or_else(|| panic!("no {k}")).repr().unwrap().to_string())
    }

    pub fn check_one(src: &str) -> Vec<(u32, u32, u32, String)> {
        let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        Python::attach(|py| check(py, src, "<test>"))
    }
}
