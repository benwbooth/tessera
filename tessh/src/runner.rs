//! Running tessh in the embedded Python: namespaces with tessh's names, and
//! source run form by form (each top-level form compiled, then run, so
//! macros and names defined earlier are there for the next).

use crate::compiler::{CompileError, Compiler, Ctx, State};
use crate::forms::Form;
use crate::reader::{char_cols, read_at, ReadError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
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
    // (PyPI's wheels — numpy, pandas, matplotlib — want the system's
    // libstdc++; on NixOS there's none on the path: this build's, loaded
    // now, is the one they get. Programs tessh runs don't see it.)
    if let Some(lib) = option_env!("TESSH_LIBSTDCXX") {
        if let Ok(c) = std::ffi::CString::new(lib) {
            unsafe { libc::dlopen(c.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
        }
    }
    let m = crate::runtime::module(py)?;
    m.add_function(wrap_pyfunction!(run_module, &m)?)?;
    crate::runtime::sync_pwd(py)?;
    // (Line-buffered even into a pipe or a file, as a shell's echo is: what a
    // script prints stays in order with what its programs and background
    // jobs print, and isn't lost when it execs or is killed.)
    for s in ["stdout", "stderr"] {
        if let Ok(f) = py.import("sys")?.getattr(s) {
            let kw = pyo3::types::PyDict::new(py);
            kw.set_item("line_buffering", true)?;
            let _ = f.call_method("reconfigure", (), Some(&kw));
        }
    }
    crate::runtime::install_import_hint(py)?;
    // (`import name` finds name.tsh too, from tessh and from Python it runs;
    // the folder you're in is on the path, as for `python -c`.)
    let path = py.import("sys")?.getattr("path")?;
    if !path.contains("")? {
        path.call_method1("insert", (0, ""))?;
    }
    PyModule::from_code(py, &std::ffi::CString::new(FINDER).unwrap(), c"<tessh import>", c"__tessh_import__")?;
    let fm = crate::pyforms::module(py)?;
    py.import("sys")?.getattr("modules")?.set_item("__tessh_forms__", fm)?;
    let signal = py.import("signal")?;
    signal.getattr("signal")?.call1((signal.getattr("SIGINT")?, signal.getattr("default_int_handler")?))?;
    Ok(())
}

const FINDER: &str = r#"
import importlib.abc, importlib.machinery, importlib.util, os, sys

class TesshLoader(importlib.abc.Loader):
    def __init__(self, path):
        self.path = path
    def create_module(self, spec):
        return None
    def exec_module(self, module):
        sys.modules["__tessh__"].run_module(self.path, module.__dict__)

class TesshFinder(importlib.abc.MetaPathFinder):
    """name.tsh (or name/__init__.tsh) on the path. The first folder with
    the name decides: a Python module or package there is Python's."""
    def find_spec(self, name, path, target=None):
        base = name.rpartition(".")[2]
        for d in (path if path is not None else sys.path):
            d = d or "."
            if not isinstance(d, str) or not os.path.isdir(d):
                continue
            if any(os.path.isfile(os.path.join(d, base + s)) for s in importlib.machinery.all_suffixes()) or os.path.isfile(os.path.join(d, base, "__init__.py")):
                return None
            f = os.path.join(d, base + ".tsh")
            if os.path.isfile(f):
                return importlib.util.spec_from_file_location(name, f, loader=TesshLoader(f))
            pkg = os.path.join(d, base, "__init__.tsh")
            if os.path.isfile(pkg):
                return importlib.util.spec_from_file_location(name, pkg, loader=TesshLoader(pkg), submodule_search_locations=[os.path.join(d, base)])
        return None

if not any(type(f).__name__ == "TesshFinder" for f in sys.meta_path):
    # (Before the path finder, which would take a folder for a namespace package.)
    at = next((i for i, f in enumerate(sys.meta_path) if getattr(f, "__name__", "") == "PathFinder"), len(sys.meta_path))
    sys.meta_path.insert(at, TesshFinder())
"#;

/// A .tsh module being imported: run in the module's namespace.
#[pyfunction]
fn run_module(py: Python<'_>, path: String, ns: Bound<'_, PyDict>) -> PyResult<()> {
    install(py, &ns)?;
    ns.set_item("__file__", &path)?;
    run_file(py, &path, &ns).map_err(|e| match e {
        TesshError::Py(e) => e,
        other => match other.position() {
            Some((l, c, _, m)) => pyo3::exceptions::PySyntaxError::new_err(format!("{path}:{l}:{}: {m}", c + 1)),
            None => other.into_pyerr(py),
        },
    })
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
    // (builtins.type, builtins.exec …: Python's, whatever the arguments.)
    if !ns.contains("builtins")? {
        ns.set_item("builtins", py.import("builtins")?)?;
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
                crate::forms::Kind::Group { open: '(', items } => (items.clone(), vec![]),
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
    // (set -v: each unit's lines, to stderr, as they're read.)
    let starts: Vec<usize> = units.iter().map(|u| u.0.pos.as_ref().map(|p| p.line as usize).unwrap_or(1)).collect();
    let src_lines: Vec<&str> = text.lines().collect();
    for (k, u) in units.iter().enumerate() {
        let last = k + 1 == n;
        if crate::runtime::VERBOSE.load(std::sync::atomic::Ordering::SeqCst) {
            let from = starts[k].max(1);
            let to = starts.get(k + 1).copied().unwrap_or(src_lines.len() + 1).max(from + 1);
            let shown: Vec<&str> = src_lines.iter().skip(from - 1).take(to - from).copied().collect();
            let _ = py.import("sys").and_then(|s| s.getattr("stderr")).and_then(|e| e.call_method1("write", (shown.join("\n") + "\n",)));
        }
        let module = {
            let mut st = sb.st.lock().unwrap();
            let mut c = Compiler::new(py, Some(ns.clone()), &mut st)?;
            c.unit_module(u, if show && last { Ctx::Tail } else { Ctx::Stmt }, show && last).map_err(|e| TesshError::Compile(e, text.to_string()))?
        };
        // (await, async for and async with at the top level: the unit is a
        // coroutine then, run on the session's event loop.)
        let code = compile.call1((module, filename, "exec", TOP_LEVEL_AWAIT))?;
        let flags: i64 = code.getattr("co_flags")?.extract()?;
        // (A line that runs no command succeeds, as at the prompt: a script
        // ending `false` then `print done` exits 0, as bash's would.)
        // (set -n: checked, not run; at the prompt it's ignored, as bash's.)
        if crate::runtime::NOEXEC.load(std::sync::atomic::Ordering::SeqCst) && !crate::runtime::INTERACTIVE.load(std::sync::atomic::Ordering::SeqCst) {
            continue;
        }
        crate::runtime::begin_line();
        if flags & CO_COROUTINE != 0 {
            let coro = builtins.getattr("eval")?.call1((code, ns))?;
            py.import("__tessh__")?.getattr("run_top")?.call1((coro,))?;
        } else {
            exec.call1((code, ns))?;
        }
        crate::runtime::end_line();
    }
    Ok(())
}

/// Run tessh text in ns, as `eval "…"` does: the last form's value (None
/// for a definition, an assignment or a loop).
pub fn eval_source(py: Python<'_>, text: &str, ns: &Bound<'_, PyDict>, filename: &str) -> Result<Py<PyAny>, TesshError> {
    install(py, ns)?;
    let lines = read_at(text, 1).map_err(TesshError::Read)?;
    let sb = state_of(ns)?;
    let sb = sb.borrow();
    let builtins = py.import("builtins")?;
    let ast = py.import("ast")?;
    let units = {
        let mut st = sb.st.lock().unwrap();
        let mut c = Compiler::new(py, Some(ns.clone()), &mut st)?;
        c.prescan_module(&lines);
        c.units_of(&lines).map_err(|e| TesshError::Compile(e, text.to_string()))?
    };
    let n = units.len();
    let mut value = py.None();
    for (k, u) in units.iter().enumerate() {
        let last = k + 1 == n;
        let module = {
            let mut st = sb.st.lock().unwrap();
            let mut c = Compiler::new(py, Some(ns.clone()), &mut st)?;
            c.unit_module(u, if last { Ctx::Tail } else { Ctx::Stmt }, last).map_err(|e| TesshError::Compile(e, text.to_string()))?
        };
        // (The last form's `__tessh__.show(v)`, as the REPL has it: kept as
        // a value instead.)
        const KEY: &str = "__tessh_eval_value__";
        let mut kept = false;
        if last {
            let body = module.getattr("body")?;
            let stmt = body.get_item(-1)?;
            let call = stmt.getattr("value").ok().filter(|c| ast.getattr("Call").and_then(|t| c.is_instance(&t)).unwrap_or(false));
            if let Some(call) = call.filter(|c| c.getattr("func").and_then(|f| f.getattr("attr")).and_then(|a| a.extract::<String>()).is_ok_and(|a| a == "show")) {
                let kw = PyDict::new(py);
                kw.set_item("id", KEY)?;
                kw.set_item("ctx", ast.getattr("Store")?.call0()?)?;
                let target = ast.getattr("Name")?.call((), Some(&kw))?;
                let kw = PyDict::new(py);
                kw.set_item("targets", vec![target])?;
                kw.set_item("value", call.getattr("args")?.get_item(0)?)?;
                let assign = ast.getattr("Assign")?.call((), Some(&kw))?;
                ast.getattr("copy_location")?.call1((&assign, &stmt))?;
                body.set_item(-1, assign)?;
                ast.getattr("fix_missing_locations")?.call1((&module,))?;
                kept = true;
            }
        }
        let code = builtins.getattr("compile")?.call1((module, filename, "exec", TOP_LEVEL_AWAIT))?;
        let flags: i64 = code.getattr("co_flags")?.extract()?;
        if flags & CO_COROUTINE != 0 {
            let coro = builtins.getattr("eval")?.call1((code, ns))?;
            py.import("__tessh__")?.getattr("run_top")?.call1((coro,))?;
        } else {
            builtins.getattr("exec")?.call1((code, ns))?;
        }
        if kept {
            if let Some(v) = ns.get_item(KEY)? {
                value = v.unbind();
                ns.del_item(KEY)?;
            }
        }
    }
    Ok(value)
}

/// ast.PyCF_ALLOW_TOP_LEVEL_AWAIT, and inspect.CO_COROUTINE.
const TOP_LEVEL_AWAIT: i32 = 0x2000;
const CO_COROUTINE: i64 = 0x80;

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
        if let Err(e) = compile.call1((m, filename, "exec", TOP_LEVEL_AWAIT)) {
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

/// An uncaught CommandFailed: the line it was raised on (the last frame's)
/// and its message.
pub fn unhandled_command(py: Python<'_>, e: &PyErr) -> Option<(Option<u32>, String)> {
    let cls = py.import("__tessh__").ok()?.getattr("CommandFailed").ok()?;
    if !e.value(py).is_instance(&cls).unwrap_or(false) {
        return None;
    }
    let mut line = None;
    let mut tb = e.traceback(py).map(|t| t.into_any());
    while let Some(t) = tb {
        line = t.getattr("tb_lineno").ok().and_then(|l| l.extract::<u32>().ok()).or(line);
        tb = t.getattr("tb_next").ok().filter(|n| !n.is_none());
    }
    Some((line, e.value(py).str().ok()?.to_string()))
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
            // (A CommandFailed nothing caught, from ! or strict mode: one
            // line, where it happened, without a traceback.)
            if let Some(line) = unhandled_command(py, &e) {
                let _ = py.import("sys").and_then(|s| s.getattr("stdout")).and_then(|o| o.call_method0("flush"));
                match line.0 {
                    Some(l) => eprintln!("tessh: {filename}:{l}: Unhandled error: {}", line.1),
                    None => eprintln!("tessh: {filename}: Unhandled error: {}", line.1),
                }
                return;
            }
            // (A builtin's error on a line of its own (cd to a folder that
            // isn't there, popd with nothing pushed): one line, as bash says it.)
            if e.value(py).hasattr("_tessh_builtin").unwrap_or(false) {
                let _ = py.import("sys").and_then(|s| s.getattr("stdout")).and_then(|o| o.call_method0("flush"));
                let name: String = e.value(py).getattr("_tessh_builtin").and_then(|n| n.extract()).unwrap_or_default();
                let msg = e.value(py).str().map(|m| m.to_string()).unwrap_or_default();
                let msg = if msg.starts_with(&format!("{name}:")) { msg } else { format!("{name}: {msg}") };
                let mut line = None;
                let mut tb = e.traceback(py).map(|t| t.into_any());
                while let Some(t) = tb {
                    // (The last frame in this file: the builtin's own Python has frames too.)
                    let here = t.getattr("tb_frame").and_then(|f| f.getattr("f_code")).and_then(|c| c.getattr("co_filename")).and_then(|f| f.extract::<String>()).is_ok_and(|f| f == filename);
                    if here {
                        line = t.getattr("tb_lineno").ok().and_then(|l| l.extract::<u32>().ok()).or(line);
                    }
                    tb = t.getattr("tb_next").ok().filter(|n| !n.is_none());
                }
                let text = match line {
                    Some(l) if filename != "<stdin>" => format!("tessh: {filename}:{l}: {msg}\n"),
                    _ => format!("tessh: {msg}\n"),
                };
                use std::io::Write;
                let _ = std::io::stderr().write_all(text.as_bytes());
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
        // (fd 1 back however f ends: a test that panics in it would leave
        // every later capture, and the reader, waiting.)
        struct Restore(i32);
        impl Drop for Restore {
            fn drop(&mut self) {
                unsafe {
                    libc::dup2(self.0, 1);
                    libc::close(self.0);
                }
            }
        }
        let restore = Restore(saved);
        f();
        let _ = py.import("sys").and_then(|s| s.getattr("stdout")).and_then(|o| o.call_method0("flush"));
        drop(restore);
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
