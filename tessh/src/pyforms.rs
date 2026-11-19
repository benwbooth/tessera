//! Forms as Python objects, for macros: the `__tessh_forms__` module.
//!
//! A macro gets its arguments as `Form` objects and returns forms (or plain
//! values, which become literals). Templates (quasi) are kept here by number
//! and filled in when they run.

use crate::compiler::CompileError;
use crate::forms::{show, Form, Kind, Part, Pos};
use pyo3::exceptions::{PyIndexError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyBytes, PyComplex, PyDict, PyFloat, PyInt, PyList, PyString, PyTuple};
use std::cell::RefCell;
use std::sync::Mutex;

#[pyclass(name = "Form", module = "__tessh_forms__", frozen)]
#[derive(Clone)]
pub struct PyForm {
    pub f: Form,
}

fn pos_tuple(py: Python<'_>, p: Option<Pos>) -> Py<PyAny> {
    match p {
        Some(p) => (p.line, p.col, p.end_line, p.end_col).into_pyobject(py).unwrap().into_any().unbind(),
        None => py.None(),
    }
}

fn pos_from(p: Option<(u32, u32, u32, u32)>) -> Option<Pos> {
    p.map(|(line, col, end_line, end_col)| Pos { line, col, end_line, end_col })
}

fn forms_tuple<'py>(py: Python<'py>, v: &[Form]) -> PyResult<Bound<'py, PyTuple>> {
    let objs: Vec<Py<PyAny>> = v.iter().map(|f| to_py(py, f.clone())).collect::<PyResult<_>>()?;
    PyTuple::new(py, objs)
}

#[pymethods]
impl PyForm {
    /// Sym, Var, Num, Str, Word, Op, Group, Line, …
    #[getter]
    fn kind(&self) -> &'static str {
        self.f.type_name()
    }

    #[getter]
    fn name(&self) -> Option<String> {
        match &self.f.kind {
            Kind::Sym { name, .. } | Kind::Op { name, .. } | Kind::Attr { name, .. } | Kind::KwArg { name, .. } => Some(name.clone()),
            Kind::Var(s) => s.sym_name().map(String::from),
            _ => None,
        }
    }

    #[getter]
    fn items<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyTuple>>> {
        match &self.f.kind {
            Kind::Line { items, .. } | Kind::Group { items, .. } | Kind::Index { items, .. } => Ok(Some(forms_tuple(py, items)?)),
            _ => Ok(None),
        }
    }

    #[getter]
    fn children<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
        match &self.f.kind {
            Kind::Line { children, .. } => forms_tuple(py, children),
            _ => PyTuple::new(py, Vec::<Py<PyAny>>::new()),
        }
    }

    /// A Str's text, a Num's number, a KwArg's or Splat's value form.
    #[getter]
    fn value(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match &self.f.kind {
            Kind::Str { value, .. } => Ok(PyString::new(py, value).into_any().unbind()),
            Kind::Num(t) => Ok(py.import("ast")?.getattr("literal_eval")?.call1((t.replace('_', ""),))?.unbind()),
            Kind::KwArg { value, .. } | Kind::Splat { value, .. } => to_py(py, (**value).clone()),
            _ => Ok(py.None()),
        }
    }

    /// The form inside: a Var's symbol, an Unquote's, a Caret's word, an
    /// Attr's or Index's object, a Unary's operand, a Quasi's form.
    #[getter]
    fn inner(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let f = match &self.f.kind {
            Kind::Var(x) | Kind::Caret(x) | Kind::SymLit(x) | Kind::Quasi(x) => x,
            Kind::Unquote { form, .. } => form,
            Kind::Attr { obj, .. } | Kind::Index { obj, .. } => obj,
            Kind::Unary { operand, .. } => operand,
            Kind::Interp { expr, .. } => expr,
            Kind::Redir { target: Some(t), .. } => t,
            _ => return Ok(py.None()),
        };
        to_py(py, (**f).clone())
    }

    #[getter]
    fn op(&self) -> Option<String> {
        match &self.f.kind {
            Kind::Op { name, .. } | Kind::Unary { op: name, .. } | Kind::Redir { op: name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    #[getter]
    fn bracket(&self) -> Option<String> {
        match &self.f.kind {
            Kind::Group { open, .. } => Some(open.to_string()),
            _ => None,
        }
    }

    #[getter]
    fn pos(&self, py: Python<'_>) -> Py<PyAny> {
        pos_tuple(py, self.f.pos)
    }

    #[getter]
    fn marks(&self) -> Vec<u32> {
        self.f.marks().to_vec()
    }

    /// `:name x`: a symbol as a getter, as Clojure's keywords: x["name"] for a
    /// mapping, else x.name; `default` (None) when it's missing.
    #[pyo3(signature = (x, default=None))]
    fn __call__(&self, py: Python<'_>, x: Bound<'_, PyAny>, default: Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
        let Kind::Sym { name, .. } = &self.f.kind else {
            return Err(PyTypeError::new_err(format!("a {} form can't be called (only a symbol, :name)", self.f.type_name())));
        };
        let default = default.unwrap_or_else(|| py.None());
        let mapping = py.import("collections.abc")?.getattr("Mapping")?;
        if x.is_instance(&mapping)? {
            return Ok(x.call_method1("get", (name.as_str(), default))?.unbind());
        }
        Ok(match x.getattr(name.as_str()) {
            Ok(v) => v.unbind(),
            Err(e) if e.is_instance_of::<pyo3::exceptions::PyAttributeError>(py) => default,
            Err(e) => return Err(e),
        })
    }

    fn __len__(&self) -> usize {
        match &self.f.kind {
            Kind::Line { items, .. } | Kind::Group { items, .. } => items.len(),
            _ => 0,
        }
    }

    fn __getitem__(&self, py: Python<'_>, i: isize) -> PyResult<Py<PyAny>> {
        let items = match &self.f.kind {
            Kind::Line { items, .. } | Kind::Group { items, .. } => items,
            _ => return Err(PyTypeError::new_err("only a Line or Group has items")),
        };
        let n = items.len() as isize;
        let k = if i < 0 { i + n } else { i };
        if k < 0 || k >= n {
            return Err(PyIndexError::new_err("item index out of range"));
        }
        to_py(py, items[k as usize].clone())
    }

    fn __repr__(&self) -> String {
        format!("<{} {}>", self.f.type_name(), show(&self.f))
    }

    fn __str__(&self) -> String {
        show(&self.f)
    }

    fn __eq__(&self, other: &Bound<'_, PyAny>) -> bool {
        match other.extract::<PyRef<PyForm>>() {
            Ok(o) => match (&self.f.kind, &o.f.kind) {
                (Kind::Sym { name: a, marks: ma }, Kind::Sym { name: b, marks: mb }) => a == b && ma == mb,
                _ => show(&self.f) == show(&o.f) && self.f.type_name() == o.f.type_name(),
            },
            Err(_) => false,
        }
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        show(&self.f).hash(&mut h);
        self.f.marks().hash(&mut h);
        h.finish()
    }
}

pub fn to_py(py: Python<'_>, f: Form) -> PyResult<Py<PyAny>> {
    Ok(Py::new(py, PyForm { f })?.into_any())
}

/// A Python value as a form (forms as they are; other values as literals).
pub fn lift(v: &Bound<'_, PyAny>) -> PyResult<Form> {
    if let Ok(f) = v.extract::<PyRef<PyForm>>() {
        return Ok(f.f.clone());
    }
    if v.is_none() {
        return Ok(Form::sym("None"));
    }
    if v.is_instance_of::<PyBool>() {
        return Ok(Form::sym(if v.extract::<bool>()? { "True" } else { "False" }));
    }
    if v.is_instance_of::<PyInt>() || v.is_instance_of::<PyFloat>() || v.is_instance_of::<PyComplex>() {
        return Ok(Form::new(Kind::Num(v.repr()?.to_string()), None));
    }
    if v.is_instance_of::<PyString>() {
        return Ok(Form::new(Kind::Str { value: v.extract()?, kind: '"' }, None));
    }
    if let Ok(b) = v.cast::<PyBytes>() {
        return Ok(Form::new(Kind::Str { value: b.as_bytes().iter().map(|c| *c as char).collect(), kind: 'b' }, None));
    }
    if v.is_instance_of::<PyList>() {
        let items: Vec<Form> = v.try_iter()?.map(|x| lift(&x?)).collect::<PyResult<_>>()?;
        return Ok(Form::group('[', items, None));
    }
    if v.is_instance_of::<PyTuple>() {
        let mut items = vec![];
        for x in v.try_iter()? {
            items.push(lift(&x?)?);
            items.push(Form::new(Kind::Comma, None));
        }
        return Ok(Form::group('(', items, None));
    }
    if let Ok(d) = v.cast::<PyDict>() {
        let mut items = vec![];
        for (k, x) in d.iter() {
            items.push(Form::new(Kind::Word(vec![Part::F(lift(&k)?), Part::Text(":".into())]), None));
            items.push(lift(&x)?);
        }
        return Ok(Form::group('{', items, None));
    }
    Err(PyTypeError::new_err(format!("can't make a form of {}", v.get_type().name()?)))
}

/// A macro's result: one form, or several (a list).
pub fn from_py_many(v: &Bound<'_, PyAny>) -> Result<Vec<Form>, CompileError> {
    if v.is_instance_of::<PyList>() {
        let mut out = vec![];
        for x in v.try_iter()? {
            out.push(lift(&x?)?);
        }
        return Ok(out);
    }
    Ok(vec![lift(v)?])
}

pub fn as_lines(v: Vec<Form>) -> Vec<Form> {
    v.into_iter()
        .map(|x| if matches!(x.kind, Kind::Line { .. }) { x } else { let p = x.pos; Form::line(vec![x], vec![], p) })
        .collect()
}

fn as_sym(v: &Bound<'_, PyAny>) -> PyResult<Form> {
    if let Ok(s) = v.extract::<String>() {
        return Ok(Form::sym(&s));
    }
    let f = lift(v)?;
    match f.kind {
        Kind::Sym { .. } => Ok(f),
        Kind::Var(s) => Ok(*s),
        Kind::Line { ref items, ref children } if items.len() == 1 && children.is_empty() && items[0].sym_name().is_some() => Ok(items[0].clone()),
        _ => Err(PyTypeError::new_err(format!("not a symbol: {}", show(&f)))),
    }
}

fn lift_many(v: &Bound<'_, PyAny>) -> PyResult<Vec<Form>> {
    if let Ok(f) = v.extract::<PyRef<PyForm>>() {
        if let Kind::Group { open: '[', items } = &f.f.kind {
            return Ok(items.clone());
        }
        return Ok(vec![f.f.clone()]);
    }
    v.try_iter()?.map(|x| lift(&x?)).collect()
}

// ---- templates -------------------------------------------------------------------

static TEMPLATES: Mutex<Vec<Form>> = Mutex::new(Vec::new());

pub fn register_template(f: Form) -> usize {
    let mut t = TEMPLATES.lock().unwrap();
    t.push(f);
    t.len() - 1
}

fn hole(k: usize, splice: bool, pos: Option<Pos>) -> Form {
    Form::new(Kind::Unquote { form: Box::new(Form::new(Kind::Num(format!("#{k}")), None)), splice }, pos)
}

fn hole_index(f: &Form) -> Option<(usize, bool)> {
    if let Kind::Unquote { form, splice } = &f.kind {
        if let Kind::Num(t) = &form.kind {
            return t.strip_prefix('#').and_then(|k| k.parse().ok()).map(|k| (k, *splice));
        }
    }
    None
}

/// The template with its unquotes replaced by numbered holes; the unquoted
/// forms (the code that fills them) in `holes`.
pub fn make_template(f: &Form, holes: &mut Vec<Form>) -> Form {
    f.clone().map(&mut |x| match &x.kind {
        Kind::Unquote { form, splice } if hole_index(&x).is_none() => {
            holes.push((**form).clone());
            hole(holes.len() - 1, *splice, x.pos)
        }
        _ => x,
    })
}

fn fill_list(items: &[Form], vals: &[Bound<'_, PyAny>], lines: bool) -> PyResult<Vec<Form>> {
    let mut out = vec![];
    for x in items {
        if let Some((k, splice)) = hole_index(x) {
            if splice {
                let fs = lift_many(&vals[k])?;
                out.extend(if lines { as_lines(fs) } else { fs });
                continue;
            }
        }
        if lines {
            if let Kind::Line { items: li, children } = &x.kind {
                if li.len() == 1 && children.is_empty() {
                    if let Some((k, _)) = hole_index(&li[0]) {
                        out.extend(as_lines(lift_many(&vals[k])?));
                        continue;
                    }
                }
            }
        }
        out.push(fill(x, vals)?);
    }
    Ok(out)
}

fn fill(f: &Form, vals: &[Bound<'_, PyAny>]) -> PyResult<Form> {
    if let Some((k, _)) = hole_index(f) {
        return lift(&vals[k]);
    }
    let b = |x: &Form| -> PyResult<Box<Form>> { Ok(Box::new(fill(x, vals)?)) };
    let parts = |ps: &[Part]| -> PyResult<Vec<Part>> {
        ps.iter().map(|p| Ok(match p { Part::F(x) => Part::F(fill(x, vals)?), t => t.clone() })).collect()
    };
    let kind = match &f.kind {
        Kind::Var(s) => match hole_index(s) {
            Some((k, _)) => Kind::Var(Box::new(as_sym(&vals[k])?)),
            None => Kind::Var(b(s)?),
        },
        Kind::Line { items, children } => Kind::Line { items: fill_list(items, vals, false)?, children: fill_list(children, vals, true)? },
        Kind::Group { open, items } => Kind::Group { open: *open, items: fill_list(items, vals, false)? },
        Kind::Index { obj, items } => Kind::Index { obj: b(obj)?, items: fill_list(items, vals, false)? },
        Kind::IStr(ps) => Kind::IStr(parts(ps)?),
        Kind::Word(ps) => Kind::Word(parts(ps)?),
        Kind::Interp { expr, conv, spec, braced } => Kind::Interp { expr: b(expr)?, conv: *conv, spec: spec.clone(), braced: *braced },
        Kind::Redir { op, target } => Kind::Redir { op: op.clone(), target: match target { Some(t) => Some(b(t)?), None => None } },
        Kind::KwArg { name, value } => Kind::KwArg { name: name.clone(), value: b(value)? },
        Kind::Splat { value, n } => Kind::Splat { value: b(value)?, n: *n },
        Kind::Caret(x) => Kind::Caret(b(x)?),
        Kind::Quasi(x) => Kind::Quasi(b(x)?),
        Kind::Attr { obj, name } => Kind::Attr { obj: b(obj)?, name: name.clone() },
        Kind::Unary { op, operand } => Kind::Unary { op: op.clone(), operand: b(operand)? },
        k => k.clone(),
    };
    Ok(Form::new(kind, f.pos))
}

#[pyfunction]
fn template(py: Python<'_>, id: usize, values: Vec<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let t = TEMPLATES.lock().unwrap().get(id).cloned().ok_or_else(|| PyValueError::new_err("no such template"))?;
    let f = fill(&t, &values)?;
    to_py(py, f)
}

// ---- the macro being expanded ---------------------------------------------------

thread_local! {
    static HEAD: RefCell<Vec<Form>> = const { RefCell::new(Vec::new()) };
}

pub fn with_head<T>(head: Form, f: impl FnOnce() -> T) -> T {
    HEAD.with(|h| h.borrow_mut().push(head));
    let r = f();
    HEAD.with(|h| h.borrow_mut().pop());
    r
}

pub fn compile_error_from(_py: Python<'_>, _e: &PyErr) -> Option<CompileError> {
    None
}

// ---- the module --------------------------------------------------------------------

#[pyfunction]
#[pyo3(name = "Sym", signature = (name, pos=None))]
fn mk_sym(py: Python<'_>, name: String, pos: Option<(u32, u32, u32, u32)>) -> PyResult<Py<PyAny>> {
    to_py(py, Form::new(Kind::Sym { name, marks: vec![] }, pos_from(pos)))
}

#[pyfunction]
#[pyo3(name = "Var", signature = (sym, pos=None))]
fn mk_var(py: Python<'_>, sym: Bound<'_, PyAny>, pos: Option<(u32, u32, u32, u32)>) -> PyResult<Py<PyAny>> {
    to_py(py, Form::new(Kind::Var(Box::new(as_sym(&sym)?)), pos_from(pos)))
}

#[pyfunction]
#[pyo3(name = "Line", signature = (items, children=Vec::new(), pos=None))]
fn mk_line(py: Python<'_>, items: Vec<Bound<'_, PyAny>>, children: Vec<Bound<'_, PyAny>>, pos: Option<(u32, u32, u32, u32)>) -> PyResult<Py<PyAny>> {
    let items = items.iter().map(lift).collect::<PyResult<_>>()?;
    let children = as_lines(children.iter().map(lift).collect::<PyResult<_>>()?);
    to_py(py, Form::line(items, children, pos_from(pos)))
}

#[pyfunction]
#[pyo3(name = "Group", signature = (bracket, items, pos=None))]
fn mk_group(py: Python<'_>, bracket: String, items: Vec<Bound<'_, PyAny>>, pos: Option<(u32, u32, u32, u32)>) -> PyResult<Py<PyAny>> {
    let open = bracket.chars().next().filter(|c| "([{".contains(*c)).ok_or_else(|| PyValueError::new_err("bracket is ( [ or {"))?;
    let items = items.iter().map(lift).collect::<PyResult<_>>()?;
    to_py(py, Form::group(open, items, pos_from(pos)))
}

#[pyfunction]
#[pyo3(name = "Op", signature = (name, pos=None))]
fn mk_op(py: Python<'_>, name: String, pos: Option<(u32, u32, u32, u32)>) -> PyResult<Py<PyAny>> {
    to_py(py, Form::new(Kind::Op { name, tick: false }, pos_from(pos)))
}

#[pyfunction]
#[pyo3(name = "Attr", signature = (obj, name, pos=None))]
fn mk_attr(py: Python<'_>, obj: Bound<'_, PyAny>, name: String, pos: Option<(u32, u32, u32, u32)>) -> PyResult<Py<PyAny>> {
    to_py(py, Form::new(Kind::Attr { obj: Box::new(lift(&obj)?), name }, pos_from(pos)))
}

#[pyfunction]
#[pyo3(name = "KwArg", signature = (name, value, pos=None))]
fn mk_kwarg(py: Python<'_>, name: String, value: Bound<'_, PyAny>, pos: Option<(u32, u32, u32, u32)>) -> PyResult<Py<PyAny>> {
    to_py(py, Form::new(Kind::KwArg { name, value: Box::new(lift(&value)?) }, pos_from(pos)))
}

#[pyfunction]
#[pyo3(name = "Splat", signature = (value, n=1, pos=None))]
fn mk_splat(py: Python<'_>, value: Bound<'_, PyAny>, n: u8, pos: Option<(u32, u32, u32, u32)>) -> PyResult<Py<PyAny>> {
    to_py(py, Form::new(Kind::Splat { value: Box::new(lift(&value)?), n }, pos_from(pos)))
}

#[pyfunction]
#[pyo3(name = "Comma")]
fn mk_comma(py: Python<'_>) -> PyResult<Py<PyAny>> {
    to_py(py, Form::new(Kind::Comma, None))
}

#[pyfunction]
#[pyo3(name = "lift")]
fn py_lift(py: Python<'_>, v: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    to_py(py, lift(&v)?)
}

#[pyfunction]
#[pyo3(name = "sym")]
fn py_sym(py: Python<'_>, name: String) -> PyResult<Py<PyAny>> {
    to_py(py, Form::sym(&name))
}

#[pyfunction]
#[pyo3(name = "var")]
fn py_var(py: Python<'_>, s: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    to_py(py, Form::new(Kind::Var(Box::new(as_sym(&s)?)), None))
}

static GENSYMS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[pyfunction]
#[pyo3(signature = (hint="g"))]
fn gensym(py: Python<'_>, hint: &str) -> PyResult<Py<PyAny>> {
    let n = GENSYMS.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
    to_py(py, Form::sym(&format!("_g{n}_{hint}")))
}

/// A symbol in the context of `like` (by default the macro call's): on
/// purpose unhygienic, for anaphoric macros.
#[pyfunction]
#[pyo3(signature = (name, like=None))]
fn capture(py: Python<'_>, name: String, like: Option<Bound<'_, PyAny>>) -> PyResult<Py<PyAny>> {
    let marks = match like {
        Some(l) => lift(&l)?.marks().to_vec(),
        None => HEAD.with(|h| h.borrow().last().map(|f| f.marks().to_vec()).unwrap_or_default()),
    };
    to_py(py, Form::new(Kind::Sym { name, marks }, None))
}

#[pyfunction]
#[pyo3(name = "show")]
fn py_show(v: Bound<'_, PyAny>) -> PyResult<String> {
    let fs = from_py_many(&v).map_err(|e| PyValueError::new_err(e.msg))?;
    Ok(fs.iter().map(show).collect::<Vec<_>>().join("\n"))
}

pub fn module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let m = PyModule::new(py, "__tessh_forms__")?;
    m.add_class::<PyForm>()?;
    m.add_function(wrap_pyfunction!(template, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_sym, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_var, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_line, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_group, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_op, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_attr, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_kwarg, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_splat, &m)?)?;
    m.add_function(wrap_pyfunction!(mk_comma, &m)?)?;
    m.add_function(wrap_pyfunction!(py_lift, &m)?)?;
    m.add_function(wrap_pyfunction!(py_sym, &m)?)?;
    m.add_function(wrap_pyfunction!(py_var, &m)?)?;
    m.add_function(wrap_pyfunction!(gensym, &m)?)?;
    m.add_function(wrap_pyfunction!(capture, &m)?)?;
    m.add_function(wrap_pyfunction!(py_show, &m)?)?;
    Ok(m)
}

/// The names macros use, put in every tessh namespace.
pub const MACRO_HELPERS: &[&str] = &["var", "sym", "gensym", "capture", "lift"];
