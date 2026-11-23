//! tessh forms to Python's AST (built as Python `ast` objects).
//!
//! Everything is an expression: each step gives a `Res`, the statements that
//! must run first and the expression for the value. Commands compile to calls
//! into the runtime (`__tessh__`), attached to the terminal or captured
//! depending on where they are. Every node carries its tessh position, so
//! Python's tracebacks point at tessh lines and columns.

use crate::forms::{show, Form, Kind, Part, Pos};
use crate::reader::{is_ident, is_number};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyInt, PyList, PyModule, PyTuple};
use pyo3::BoundObject;
use std::collections::{HashMap, HashSet};

pub type Obj<'py> = Bound<'py, PyAny>;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub msg: String,
    pub pos: Option<Pos>,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.pos {
            Some(p) => write!(f, "{}:{}: {}", p.line, p.col + 1, self.msg),
            None => write!(f, "{}", self.msg),
        }
    }
}

impl From<PyErr> for CompileError {
    fn from(e: PyErr) -> Self {
        CompileError { msg: format!("internal error: {e}"), pos: None }
    }
}

type R<T> = Result<T, CompileError>;

fn err<T>(msg: impl Into<String>, f: Option<&Form>) -> R<T> {
    Err(CompileError { msg: msg.into(), pos: f.and_then(|f| f.pos) })
}

// ---- a loop variable written bare ----------------------------------------------------
// `for x in $xs` makes x; reading it is `$x`. A bare `x` where it's read is
// the text "x", and Python habit ([x * 2 for x in xs]) gave ['xx', 'xx']
// without a word: those are errors, with the fix.

/// The names a for's target binds (x, or [k v], or a, b).
fn bound_names(target: &[Form]) -> Vec<String> {
    let mut out = vec![];
    for t in target {
        match &t.kind {
            Kind::Sym { name, .. } if crate::reader::is_ident(name) => out.push(name.clone()),
            Kind::Group { items, .. } => out.extend(bound_names(items)),
            _ => {}
        }
    }
    out
}

/// A bare word naming one of `names` where it would be read as text: an
/// argument, a list item, an operand. Not a command's own name (head), not
/// what an assignment or a nested for binds, not inside a def or the like
/// (their own names).
fn bare_read<'a>(items: &'a [Form], names: &[String], head_ok: bool) -> Option<&'a Form> {
    if names.is_empty() {
        return None;
    }
    // (An expression, with operators: every item is a value; a call or a
    // command: its first item is the name of what runs.)
    let has_op = items.iter().any(|x| matches!(x.kind, Kind::Op { .. }));
    let assign_at = items.iter().position(|x| matches!(&x.kind, Kind::Op { name, .. } if ASSIGN_OPS.iter().any(|(o, _)| o == name)));
    let mut skip_to_in = false;
    for (i, x) in items.iter().enumerate() {
        if skip_to_in {
            if x.is_sym("in") {
                skip_to_in = false;
            }
            continue;
        }
        if x.is_sym("for") {
            skip_to_in = true;
            continue;
        }
        if assign_at.is_some_and(|a| i < a) {
            continue;
        }
        if i == 0 && head_ok && !has_op {
            continue;
        }
        if let Some(f) = bare_in(x, names) {
            return Some(f);
        }
    }
    None
}

fn bare_in<'a>(x: &'a Form, names: &[String]) -> Option<&'a Form> {
    match &x.kind {
        Kind::Sym { name, .. } if names.iter().any(|n| n == name) => Some(x),
        Kind::Group { open, items } => bare_read(items, names, *open == '('),
        Kind::Line { items, children } => {
            const OWN_NAMES: &[&str] = &["def", "fn", "lambda", "class", "defmacro", "import", "from", "quasi", "async"];
            if items.first().and_then(|h| h.sym_name()).is_some_and(|h| OWN_NAMES.contains(&h)) {
                return None;
            }
            bare_read(items, names, true).or_else(|| children.iter().find_map(|c| bare_in(c, names)))
        }
        Kind::Unary { operand, .. } => bare_in(operand, names),
        Kind::Index { obj, items } => bare_in(obj, names).or_else(|| bare_read(items, names, false)),
        Kind::Attr { obj, .. } => bare_in(obj, names),
        Kind::KwArg { value, .. } | Kind::Splat { value, .. } => bare_in(value, names),
        _ => None,
    }
}

fn bare_loop_var<T>(f: &Form) -> R<T> {
    let n = f.sym_name().unwrap_or("");
    err(format!("`{n}` is the loop's variable here, but a bare word is text: write ${n} (or '{n}' for the text)"), Some(f))
}

/// `for x in xs`: the letters of "xs", never what was meant.
fn bare_iterable(iter: &[Form]) -> R<()> {
    if let [one] = iter {
        if let Some(n) = one.sym_name().filter(|n| crate::reader::is_ident(n)) {
            return err(format!("`{n}` here is the text '{n}' (it would loop over its letters): write ${n} for the variable, or '{n}' for the text"), Some(one));
        }
    }
    Ok(())
}

const RT: &str = "__tessh__";
const FM: &str = "__tessh_forms__";

const BINOPS: &[(&str, &str)] = &[("+", "Add"), ("-", "Sub"), ("*", "Mult"), ("/", "Div"), ("//", "FloorDiv"), ("%", "Mod"), ("@", "MatMult"), ("**", "Pow"), ("<<", "LShift"), (">>", "RShift"), ("^", "BitXor"), ("&", "BitAnd"), ("|", "BitOr"), ("=~", "Match"), ("!~", "NotMatch")];
const CMPOPS: &[(&str, &str)] = &[("==", "Eq"), ("!=", "NotEq"), ("<", "Lt"), (">", "Gt"), ("<=", "LtE"), (">=", "GtE"), ("in", "In"), ("not in", "NotIn"), ("is", "Is"), ("is not", "IsNot")];
const ASSIGN_OPS: &[(&str, &str)] = &[("=", ""), ("+=", "Add"), ("-=", "Sub"), ("*=", "Mult"), ("/=", "Div"), ("//=", "FloorDiv"), ("%=", "Mod"), ("**=", "Pow"), ("@=", "MatMult"), ("&=", "BitAnd"), ("|=", "BitOr"), ("^=", "BitXor"), ("<<=", "LShift"), (">>=", "RShift")];
const WORD_OPS: &[&str] = &["and", "or", "not", "in", "is"];
const SPACED_REDIRS: &[&str] = &[">", ">>", "<", "1>", "1>>", "2>", "2>>", "&>", "&>>"];
const CONTINUATIONS: &[&str] = &["elif", "else", "except", "except*", "finally"];
pub const BLOCK_SPECIALS: &[&str] = &["def", "class", "if", "while", "for", "try", "with", "match", "defmacro", "quasi", "do", "async", "fn", "lambda", "let"];
const OTHER_SPECIALS: &[&str] = &["return", "yield", "await", "raise", "assert", "del", "pass", "break", "continue", "global", "nonlocal", "import", "from", "quote", "defop", "->", "->>", "some->", "some->>", "as->", "doto", "py", "require", "elif", "else", "except", "except*", "finally", "case"];

fn binop(n: &str) -> Option<&'static str> {
    BINOPS.iter().find(|(o, _)| *o == n).map(|(_, a)| *a)
}
fn cmpop(n: &str) -> Option<&'static str> {
    CMPOPS.iter().find(|(o, _)| *o == n).map(|(_, a)| *a)
}
fn assign_op(n: &str) -> Option<&'static str> {
    ASSIGN_OPS.iter().find(|(o, _)| *o == n).map(|(_, a)| *a)
}
pub fn is_special(n: &str) -> bool {
    BLOCK_SPECIALS.contains(&n) || OTHER_SPECIALS.contains(&n)
}
fn prec(n: &str) -> u8 {
    match n {
        ":=" => 0,
        "or" | "||" => 1,
        "and" | "&&" => 2,
        "|" => 4,
        "^" => 7,
        "&" => 8,
        "<<" | ">>" => 9,
        "+" | "-" => 10,
        "*" | "/" | "//" | "%" | "@" => 11,
        "**" => 13,
        "=~" | "!~" => 5,
        _ if cmpop(n).is_some() => 5,
        _ => 6,
    }
}
fn prefix_prec(n: &str) -> Option<u8> {
    match n {
        "-" | "+" | "~" => Some(12),
        "not" | "!" => Some(3),
        _ => None,
    }
}
fn is_dotted(s: &str) -> bool {
    !s.is_empty() && s.split('.').all(is_ident)
}

#[derive(Clone, Copy, PartialEq)]
pub enum Ctx {
    Stmt,
    Tail,
    Value,
    Cond,
}

pub struct Res<'py> {
    pub stmts: Vec<Obj<'py>>,
    pub expr: Option<Obj<'py>>,
}

impl<'py> Res<'py> {
    fn e(expr: Obj<'py>) -> Self {
        Res { stmts: vec![], expr: Some(expr) }
    }
    fn s(stmts: Vec<Obj<'py>>, expr: Obj<'py>) -> Self {
        Res { stmts, expr: Some(expr) }
    }
}

#[derive(Default)]
pub struct Scope {
    kind: &'static str,
    names: HashMap<(String, Vec<u32>), String>,
    known: HashSet<String>,
    commands: HashSet<String>,
    is_gen: bool,
}

impl Scope {
    fn new(kind: &'static str) -> Self {
        Scope { kind, ..Default::default() }
    }
}

pub struct Macro {
    pub name: String,
    pub func: Py<PyAny>,
    /// (kind: one, expr, rest; name)
    pub params: Vec<(&'static str, String)>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum Assoc {
    Left,
    Right,
    None,
}

/// What lasts between compiles in one namespace (a REPL session, a kernel).
pub struct State {
    pub macros: HashMap<String, Macro>,
    pub ops: HashMap<String, (u8, Assoc, Form)>,
    pub module: Scope,
    tmp: u64,
    marks: u32,
}

impl Default for State {
    fn default() -> Self {
        State { macros: HashMap::new(), ops: HashMap::new(), module: Scope::new("module"), tmp: 0, marks: 0 }
    }
}

pub struct Compiler<'py, 's> {
    pub py: Python<'py>,
    ast: Bound<'py, PyModule>,
    pub ns: Option<Bound<'py, PyDict>>,
    pub st: &'s mut State,
    scopes: Vec<Scope>,
    depth: u32,
    py_builtins: HashSet<String>,
}

fn split<'a>(items: &'a [Form], pred: impl Fn(&Form) -> bool) -> Vec<&'a [Form]> {
    let mut out = vec![];
    let mut start = 0;
    for (i, x) in items.iter().enumerate() {
        if pred(x) {
            out.push(&items[start..i]);
            start = i + 1;
        }
    }
    out.push(&items[start..]);
    out
}

fn comma_parts(items: &[Form]) -> Vec<&[Form]> {
    let mut parts = split(items, |x| x.is_comma());
    if parts.last().is_some_and(|p| p.is_empty()) {
        parts.pop();
    }
    parts
}

fn top_for(items: &[Form]) -> usize {
    items.iter().enumerate().skip(1).find(|(_, x)| x.is_sym("for")).map(|(k, _)| k).unwrap_or(0)
}

fn has_binop(items: &[Form]) -> bool {
    items.iter().enumerate().any(|(k, x)| k > 0 && k + 1 < items.len() && x.op_name().is_some_and(|n| binop(n).is_some() || cmpop(n).is_some()))
}

fn line_parts(f: &Form) -> (&[Form], &[Form]) {
    match &f.kind {
        Kind::Line { items, children } => (items, children),
        _ => (std::slice::from_ref(f), &[]),
    }
}

/// :(a b c) quasiquotes the form a b c (a Line), not the group.
fn group_form(g: &Form) -> Form {
    match &g.kind {
        Kind::Group { open: '(', items } => Form::line(items.clone(), vec![], g.pos),
        _ => g.clone(),
    }
}

enum Tok {
    X(Form),
    Op(String, Form),
    Pre(String, Form),
    Chunk(Vec<Form>),
}

enum Tree {
    Chunk(Vec<Form>),
    Un(String, Box<Tree>, Form),
    Cmp(Box<Tree>, Vec<(String, Form)>, Vec<Tree>),
    Bin(String, Box<Tree>, Box<Tree>, Form),
}

type Unit = (Form, Vec<Form>, Vec<Form>);

impl<'py, 's> Compiler<'py, 's> {
    pub fn new(py: Python<'py>, ns: Option<Bound<'py, PyDict>>, st: &'s mut State) -> PyResult<Self> {
        let builtins: Vec<String> = py.import("builtins")?.dir()?.extract()?;
        Ok(Compiler { py, ast: py.import("ast")?, ns, st, scopes: vec![], depth: 0, py_builtins: builtins.into_iter().collect() })
    }

    // ---- AST nodes ------------------------------------------------------------

    fn n(&self, kind: &str, kw: Vec<(&str, Obj<'py>)>) -> R<Obj<'py>> {
        let d = PyDict::new(self.py);
        for (k, v) in kw {
            d.set_item(k, v)?;
        }
        Ok(self.ast.getattr(kind)?.call((), Some(&d))?)
    }
    fn n0(&self, kind: &str) -> R<Obj<'py>> {
        Ok(self.ast.getattr(kind)?.call0()?)
    }
    fn c<T: IntoPyObject<'py>>(&self, v: T) -> R<Obj<'py>> {
        let v = v.into_pyobject(self.py).map_err(Into::into)?.into_any().into_bound();
        self.n("Constant", vec![("value", v)])
    }
    fn none(&self) -> Obj<'py> {
        self.py.None().into_bound(self.py)
    }
    fn cnone(&self) -> R<Obj<'py>> {
        self.n("Constant", vec![("value", self.none())])
    }
    fn s(&self, v: &str) -> Obj<'py> {
        pyo3::types::PyString::new(self.py, v).into_any()
    }
    fn list(&self, v: Vec<Obj<'py>>) -> R<Obj<'py>> {
        Ok(PyList::new(self.py, v)?.into_any())
    }
    fn ctx(&self, store: bool) -> R<Obj<'py>> {
        self.n0(if store { "Store" } else { "Load" })
    }
    fn name(&self, id: &str, store: bool) -> R<Obj<'py>> {
        self.n("Name", vec![("id", self.s(id)), ("ctx", self.ctx(store)?)])
    }
    fn attr(&self, v: Obj<'py>, a: &str, store: bool) -> R<Obj<'py>> {
        self.n("Attribute", vec![("value", v), ("attr", self.s(a)), ("ctx", self.ctx(store)?)])
    }
    fn rt(&self, a: &str) -> R<Obj<'py>> {
        self.attr(self.name(RT, false)?, a, false)
    }
    fn call(&self, f: Obj<'py>, args: Vec<Obj<'py>>, kws: Vec<Obj<'py>>) -> R<Obj<'py>> {
        self.n("Call", vec![("func", f), ("args", self.list(args)?), ("keywords", self.list(kws)?)])
    }
    fn rcall(&self, a: &str, args: Vec<Obj<'py>>) -> R<Obj<'py>> {
        self.call(self.rt(a)?, args, vec![])
    }
    fn globals(&self) -> R<Obj<'py>> {
        self.call(self.name("globals", false)?, vec![], vec![])
    }
    fn keyword(&self, arg: Option<&str>, v: Obj<'py>) -> R<Obj<'py>> {
        self.n("keyword", vec![("arg", arg.map(|a| self.s(a)).unwrap_or_else(|| self.none())), ("value", v)])
    }
    fn starred(&self, v: Obj<'py>, store: bool) -> R<Obj<'py>> {
        self.n("Starred", vec![("value", v), ("ctx", self.ctx(store)?)])
    }
    fn tuple(&self, v: Vec<Obj<'py>>, store: bool) -> R<Obj<'py>> {
        self.n("Tuple", vec![("elts", self.list(v)?), ("ctx", self.ctx(store)?)])
    }
    fn assign(&self, target: &str, v: Obj<'py>) -> R<Obj<'py>> {
        self.n("Assign", vec![("targets", self.list(vec![self.name(target, true)?])?), ("value", v)])
    }
    fn expr_stmt(&self, v: Obj<'py>) -> R<Obj<'py>> {
        self.n("Expr", vec![("value", v)])
    }
    fn iff(&self, test: Obj<'py>, body: Vec<Obj<'py>>, orelse: Vec<Obj<'py>>) -> R<Obj<'py>> {
        self.n("If", vec![("test", test), ("body", self.list(body)?), ("orelse", self.list(orelse)?)])
    }
    fn unary(&self, op: &str, v: Obj<'py>) -> R<Obj<'py>> {
        let o = match op {
            "-" => "USub",
            "+" => "UAdd",
            "~" => "Invert",
            _ => "Not",
        };
        self.n("UnaryOp", vec![("op", self.n0(o)?), ("operand", v)])
    }
    fn pass(&self) -> R<Obj<'py>> {
        self.n0("Pass")
    }
    fn noargs(&self) -> R<Obj<'py>> {
        self.n("arguments", vec![("posonlyargs", self.list(vec![])?), ("args", self.list(vec![])?), ("kwonlyargs", self.list(vec![])?), ("kw_defaults", self.list(vec![])?), ("defaults", self.list(vec![])?)])
    }
    fn lambda(&self, args: Obj<'py>, body: Obj<'py>) -> R<Obj<'py>> {
        self.n("Lambda", vec![("args", args), ("body", body)])
    }

    /// Put a form's position on a node.
    fn at(&self, node: Obj<'py>, f: &Form) -> Obj<'py> {
        if let Some(p) = f.pos {
            if p.line > 0 {
                let _ = node.setattr("lineno", p.line);
                let _ = node.setattr("col_offset", p.col);
                let _ = node.setattr("end_lineno", p.end_line.max(p.line));
                let _ = node.setattr("end_col_offset", if p.end_line > p.line { p.end_col } else { p.end_col.max(p.col) });
            }
        }
        node
    }

    fn is_pure(&self, e: &Obj<'py>) -> bool {
        let t = e.get_type().name().map(|n| n.to_string()).unwrap_or_default();
        t == "Constant" || t == "Name"
    }

    fn tmp(&mut self, hint: &str) -> String {
        self.st.tmp += 1;
        format!("_tessh_{hint}{}", self.st.tmp)
    }

    // ---- scopes and names -----------------------------------------------------

    fn scope(&mut self) -> &mut Scope {
        match self.scopes.last_mut() {
            Some(s) => s,
            None => &mut self.st.module,
        }
    }

    fn push(&mut self, kind: &'static str) {
        self.scopes.push(Scope::new(kind));
    }

    fn pop(&mut self) -> Scope {
        self.scopes.pop().unwrap_or_default()
    }

    /// A binding of s in the current scope: its Python name (renamed when a
    /// macro made it: it has marks).
    fn bind(&mut self, s: &Form) -> R<String> {
        let (name, marks) = match &s.kind {
            Kind::Sym { name, marks } => (name.clone(), marks.clone()),
            Kind::Var(inner) => return self.bind(inner),
            _ => return err(format!("'{}' isn't a name", show(s)), Some(s)),
        };
        if !is_ident(&name) {
            return err(format!("'{name}' can't be a name (names are letters, digits and _)"), Some(s));
        }
        let key = (name.clone(), marks.clone());
        let sc = self.scope();
        if let Some(p) = sc.names.get(&key) {
            return Ok(p.clone());
        }
        let py = if marks.is_empty() { name } else { format!("_h{}_{}", marks.iter().map(|m| m.to_string()).collect::<Vec<_>>().join("_"), name) };
        sc.names.insert(key, py.clone());
        sc.known.insert(py.clone());
        Ok(py)
    }

    fn binding(&self, name: &str, marks: &[u32]) -> Option<String> {
        let key = (name.to_string(), marks.to_vec());
        let n = self.scopes.len();
        for (i, sc) in self.scopes.iter().enumerate().rev() {
            if sc.kind != "class" || i + 1 == n {
                if let Some(p) = sc.names.get(&key) {
                    return Some(p.clone());
                }
            }
        }
        self.st.module.names.get(&key).cloned()
    }

    fn user_shadows(&self, n: &str) -> bool {
        self.scopes.iter().any(|sc| sc.names.contains_key(&(n.to_string(), vec![])) || sc.known.contains(n))
    }

    fn ns_get(&self, n: &str) -> Option<Obj<'py>> {
        self.ns.as_ref().and_then(|ns| ns.get_item(n).ok().flatten())
    }

    fn is_command_fn(v: &Obj<'py>) -> bool {
        v.hasattr("_tessh_command").unwrap_or(false)
    }

    /// Is s (its first dotted part) a Python name the compiler knows?
    fn is_known(&self, s: &Form) -> bool {
        let (name, marks) = match &s.kind {
            Kind::Sym { name, marks } => (name.as_str(), marks.as_slice()),
            _ => return false,
        };
        let root = name.split('.').next().unwrap_or("");
        if !is_ident(root) {
            return false;
        }
        if self.binding(root, marks).is_some() {
            return true;
        }
        for sc in self.scopes.iter().rev().chain(std::iter::once(&self.st.module)) {
            if sc.commands.contains(root) {
                return false;
            }
            if sc.known.contains(root) {
                return true;
            }
        }
        if let Some(v) = self.ns_get(root) {
            return !Self::is_command_fn(&v);
        }
        if root == RT || root == FM {
            return true;
        }
        if crate::runtime::is_command_builtin(root) {
            return false;
        }
        self.py_builtins.contains(root)
    }

    fn resolve(&self, name: &str, marks: &[u32], f: &Form) -> R<Obj<'py>> {
        if let Some(p) = self.binding(name, marks) {
            return Ok(self.at(self.name(&p, false)?, f));
        }
        if !marks.is_empty() && self.user_shadows(name) {
            return self.rcall("free", vec![self.globals()?, self.c(name)?]);
        }
        Ok(self.at(self.name(name, false)?, f))
    }

    fn name_load(&self, s: &Form) -> R<Obj<'py>> {
        let (name, marks) = match &s.kind {
            Kind::Sym { name, marks } => (name.as_str(), marks.as_slice()),
            Kind::Var(inner) => return self.name_load(inner),
            _ => return err(format!("'{}' isn't a name", show(s)), Some(s)),
        };
        let parts: Vec<&str> = name.split('.').collect();
        if !parts.iter().all(|p| is_ident(p)) {
            return err(format!("'{name}' isn't a name"), Some(s));
        }
        let mut e = self.resolve(parts[0], marks, s)?;
        for p in &parts[1..] {
            e = self.at(self.attr(e, p, false)?, s);
        }
        Ok(e)
    }

    /// Statements and expressions of several results, in order: earlier
    /// values are kept in temporaries when a later one needs statements.
    fn gather(&mut self, rs: Vec<Res<'py>>) -> R<(Vec<Obj<'py>>, Vec<Option<Obj<'py>>>)> {
        let mut stmts = vec![];
        let mut exprs: Vec<Option<Obj<'py>>> = vec![];
        for r in rs {
            if !r.stmts.is_empty() {
                for e in exprs.iter_mut() {
                    if let Some(x) = e {
                        if !self.is_pure(x) {
                            let t = self.tmp("t");
                            stmts.push(self.assign(&t, x.clone())?);
                            *e = Some(self.name(&t, false)?);
                        }
                    }
                }
                stmts.extend(r.stmts);
            }
            exprs.push(r.expr);
        }
        Ok((stmts, exprs))
    }

    fn gather_e(&mut self, rs: Vec<Res<'py>>) -> R<(Vec<Obj<'py>>, Vec<Obj<'py>>)> {
        let (s, es) = self.gather(rs)?;
        let mut out = vec![];
        for e in es {
            out.push(match e {
                Some(e) => e,
                None => self.cnone()?,
            });
        }
        Ok((s, out))
    }

    fn as_stmts(&self, r: Res<'py>) -> R<Vec<Obj<'py>>> {
        let mut out = r.stmts;
        if let Some(e) = r.expr {
            if !self.is_pure(&e) {
                out.push(self.expr_stmt(e)?);
            }
        }
        Ok(out)
    }

    fn val(&self, e: Option<Obj<'py>>) -> R<Obj<'py>> {
        match e {
            Some(e) => Ok(e),
            None => self.cnone(),
        }
    }

    // ---- prescan --------------------------------------------------------------

    pub fn prescan_module(&mut self, lines: &[Form]) {
        let mut sc = std::mem::take(&mut self.st.module);
        self.prescan(lines, &mut sc);
        self.st.module = sc;
    }

    fn prescan(&self, lines: &[Form], sc: &mut Scope) {
        let mut decos: Vec<String> = vec![];
        for l in lines {
            let (items, children) = line_parts(l);
            if items.is_empty() {
                continue;
            }
            let hn = items[0].sym_name().unwrap_or("");
            if let Some(i) = self.assign_index(items) {
                for t in &items[..i] {
                    target_names(t, &mut sc.known);
                }
            } else if hn.starts_with('@') {
                decos.push(hn[1..].to_string());
                continue;
            } else if (hn == "def" || hn == "class") && items.len() > 1 {
                if let Some(n) = items[1].sym_name() {
                    if decos.iter().any(|d| d == "command") {
                        sc.commands.insert(n.to_string());
                    } else {
                        sc.known.insert(n.to_string());
                    }
                }
            } else if hn == "async" && items.len() > 2 {
                if let Some(n) = items[2].sym_name() {
                    sc.known.insert(n.to_string());
                }
            } else if hn == "import" || hn == "from" {
                let start = if hn == "from" { items.iter().position(|x| x.is_sym("import")).map(|k| k + 1).unwrap_or(items.len()) } else { 1 };
                let mut k = start;
                while k < items.len() {
                    if let Some(n) = items[k].sym_name() {
                        if k + 2 < items.len() && items[k + 1].is_sym("as") {
                            if let Some(a) = items[k + 2].sym_name() {
                                sc.known.insert(a.to_string());
                            }
                            k += 3;
                            continue;
                        }
                        let n = if hn == "import" { n.split('.').next().unwrap_or(n) } else { n };
                        sc.known.insert(n.to_string());
                    }
                    k += 1;
                }
            } else if hn == "for" {
                for t in &items[1..] {
                    if t.is_sym("in") {
                        break;
                    }
                    target_names(t, &mut sc.known);
                }
            } else if hn == "with" || hn == "except" {
                for (k, it) in items.iter().enumerate() {
                    if it.is_sym("as") && k + 1 < items.len() {
                        target_names(&items[k + 1], &mut sc.known);
                    }
                }
            } else if hn == "global" || hn == "nonlocal" {
                for it in &items[1..] {
                    if let Some(n) = it.sym_name() {
                        sc.known.insert(n.to_string());
                    }
                }
            }
            decos.clear();
            if ["if", "elif", "else", "while", "for", "try", "except", "finally", "with", "match", "case", "do"].contains(&hn) {
                self.prescan(children, sc);
                if hn == "match" {
                    for c in children {
                        self.prescan(line_parts(c).1, sc);
                    }
                }
            }
        }
    }

    fn assign_index(&self, items: &[Form]) -> Option<usize> {
        for (i, it) in items.iter().enumerate() {
            if let Some(n) = it.op_name() {
                if assign_op(n).is_some() {
                    if i == 0 {
                        return None;
                    }
                    let lhs = &items[..i];
                    if (lhs.len() == 1 || lhs.iter().any(|x| x.is_comma()))
                        && lhs.iter().all(|x| matches!(x.kind, Kind::Sym { .. } | Kind::Var(_) | Kind::Attr { .. } | Kind::Index { .. } | Kind::Group { .. } | Kind::Comma | Kind::Splat { .. }))
                    {
                        return Some(i);
                    }
                    return None;
                }
                return None;
            }
            if matches!(it.kind, Kind::Op { .. }) {
                return None;
            }
        }
        None
    }

    // ---- blocks ---------------------------------------------------------------

    /// Lines with their continuations (elif, else, except, finally) and
    /// decorators.
    fn units(&self, lines: &[Form]) -> R<Vec<Unit>> {
        let mut out: Vec<Unit> = vec![];
        let mut decos: Vec<Form> = vec![];
        for l in lines {
            let (items, _) = line_parts(l);
            let hn = items.first().and_then(|h| h.sym_name()).unwrap_or("");
            if CONTINUATIONS.contains(&hn) {
                match out.last_mut() {
                    Some(u) if takes_cont(self, u, hn) => {
                        u.1.push(l.clone());
                        continue;
                    }
                    _ => return err(format!("'{hn}' without a matching if, for, while or try above it"), items.first()),
                }
            }
            if hn.len() > 1 && hn.starts_with('@') {
                decos.push(l.clone());
                continue;
            }
            if !decos.is_empty() && !["def", "class", "async"].contains(&hn) {
                return err("a decorator must come just before def or class", Some(&decos[0]));
            }
            out.push((l.clone(), vec![], std::mem::take(&mut decos)));
        }
        if !decos.is_empty() {
            return err("a decorator must come just before def or class", Some(&decos[0]));
        }
        Ok(out)
    }

    pub fn compile_body(&mut self, lines: &[Form], ctx: Ctx) -> R<Res<'py>> {
        let us = self.units(lines)?;
        if us.is_empty() {
            return Ok(Res::e(self.cnone()?));
        }
        let mut stmts = vec![];
        let n = us.len();
        for (k, u) in us.into_iter().enumerate() {
            if k + 1 < n {
                let r = self.compile_unit(&u, Ctx::Stmt)?;
                stmts.extend(self.as_stmts(r)?);
            } else {
                let r = self.compile_unit(&u, ctx)?;
                stmts.extend(r.stmts);
                return Ok(Res { stmts, expr: r.expr });
            }
        }
        unreachable!()
    }

    fn compile_unit(&mut self, u: &Unit, ctx: Ctx) -> R<Res<'py>> {
        let (items, children) = line_parts(&u.0);
        let r = self.compile_items(items, children, ctx, &u.1, &u.2)?;
        for s in &r.stmts {
            if !s.hasattr("lineno").unwrap_or(true) || s.getattr("lineno").map(|v| v.is_none()).unwrap_or(true) {
                self.at(s.clone(), &u.0);
            }
        }
        Ok(r)
    }

    fn compile_line(&mut self, l: &Form, ctx: Ctx) -> R<Res<'py>> {
        let (items, children) = line_parts(l);
        self.compile_items(items, children, ctx, &[], &[])
    }

    // ---- forms ----------------------------------------------------------------

    fn special_head(&self, f: &Form) -> Option<String> {
        let n = f.sym_name()?;
        if (is_special(n) && self.binding(n, &[]).is_none()) || self.st.macros.contains_key(n) {
            Some(n.to_string())
        } else {
            None
        }
    }

    pub fn compile_items(&mut self, items: &[Form], children: &[Form], ctx: Ctx, conts: &[Form], decos: &[Form]) -> R<Res<'py>> {
        if items.is_empty() {
            if !children.is_empty() {
                return self.compile_body(children, ctx);
            }
            return Ok(Res::e(self.cnone()?));
        }
        // (`name: type` and `name: type = value`: an annotated name.)
        if let Kind::Sym { name, marks } = &items[0].kind {
            if let Some(stem) = name.strip_suffix(':').filter(|s| is_ident(s)) {
                if items.len() >= 2 {
                    let target = Form::new(Kind::Sym { name: stem.to_string(), marks: marks.clone() }, items[0].pos.clone());
                    return self.compile_annassign(&target, &items[1..], children);
                }
            }
        }
        if let Some(i) = self.assign_index(items) {
            return self.compile_assign(&items[..i], &items[i], &items[i + 1..], children, conts);
        }
        // (name=value first on a line, as bash has it: alone (or several),
        // assignments, as name = value; before a program, its environment.)
        if matches!(items[0].kind, Kind::KwArg { .. }) {
            return self.compile_bare_assign(items, children, ctx, conts);
        }
        if let Some(n) = items[0].op_name().filter(|o| *o == "->" || *o == "->>") {
            return self.special(n, items, children, ctx, conts, decos);
        }
        if let Some(n) = items[0].sym_name() {
            if is_special(n) && self.binding(n, &[]).is_none() {
                return self.special(n, items, children, ctx, conts, decos);
            }
            if self.st.macros.contains_key(n) {
                return self.expand_macro(n, items, children, ctx);
            }
        }
        if let Some(c) = conts.first() {
            return err("elif, else, except and finally go after if, for, while or try", Some(c));
        }
        if let Some(d) = decos.first() {
            return err("a decorator must come just before def or class", Some(d));
        }
        self.compile_shell(items, children, ctx)
    }

    fn compile_shell(&mut self, items: &[Form], children: &[Form], ctx: Ctx) -> R<Res<'py>> {
        if items.len() > 1 && items[items.len() - 1].is_op("&") {
            return self.compile_bg(&items[..items.len() - 1], children);
        }
        let semis = split(items, |x| matches!(x.kind, Kind::Semi));
        if semis.len() > 1 {
            let parts: Vec<&[Form]> = semis.into_iter().filter(|p| !p.is_empty()).collect();
            let mut stmts = vec![];
            for p in &parts[..parts.len() - 1] {
                let r = self.compile_items(p, &[], Ctx::Stmt, &[], &[])?;
                stmts.extend(self.as_stmts(r)?);
            }
            let r = self.compile_items(parts[parts.len() - 1], children, ctx, &[], &[])?;
            stmts.extend(r.stmts);
            return Ok(Res { stmts, expr: r.expr });
        }
        let sub = if ctx == Ctx::Value { Ctx::Value } else { Ctx::Cond };
        for (opname, is_and) in [("||", false), ("&&", true)] {
            let parts = split(items, |x| x.is_op(opname));
            if parts.len() > 1 {
                if parts.iter().any(|p| p.is_empty()) {
                    return err(format!("'{opname}' needs something on both sides"), items.first());
                }
                let n = parts.len();
                let mut rs = vec![];
                for (k, p) in parts.into_iter().enumerate() {
                    rs.push(self.compile_shell(p, if k + 1 == n { children } else { &[] }, sub)?);
                }
                return self.bool_chain(is_and, rs);
            }
        }
        if items[0].is_op("!") && items.len() > 1 {
            let r = self.compile_shell(&items[1..], children, sub)?;
            let e = self.val(r.expr)?;
            return Ok(Res::s(r.stmts, self.unary("not", e)?));
        }
        let segs = self.pipe_split(items);
        if segs.len() > 1 {
            return self.compile_pipe(&segs, children, ctx);
        }
        self.compile_segment(items, children, ctx)
    }

    fn pipe_split<'a>(&self, items: &'a [Form]) -> Vec<&'a [Form]> {
        let mut segs = vec![];
        let mut start = 0;
        for (k, x) in items.iter().enumerate() {
            if x.is_op("|") && k > start && k + 1 < items.len() && is_stage_head(&items[k + 1]) {
                segs.push(&items[start..k]);
                start = k + 1;
            }
        }
        segs.push(&items[start..]);
        segs
    }

    fn bool_chain(&mut self, is_and: bool, rs: Vec<Res<'py>>) -> R<Res<'py>> {
        let mut rs = rs;
        if rs.iter().skip(1).all(|r| r.stmts.is_empty()) {
            let first_stmts = std::mem::take(&mut rs[0].stmts);
            let mut vals = vec![];
            for r in rs {
                vals.push(self.val(r.expr)?);
            }
            let op = self.n0(if is_and { "And" } else { "Or" })?;
            return Ok(Res::s(first_stmts, self.n("BoolOp", vec![("op", op), ("values", self.list(vals)?)])?));
        }
        let t = self.tmp("b");
        let mut it = rs.into_iter();
        let first = it.next().unwrap();
        let mut stmts = first.stmts;
        stmts.push(self.assign(&t, self.val(first.expr)?)?);
        // (Each later part runs inside the one before.)
        let mut nested: Vec<(Obj<'py>, Vec<Obj<'py>>)> = vec![];
        for r in it {
            let test = if is_and { self.name(&t, false)? } else { self.unary("not", self.name(&t, false)?)? };
            let mut body = r.stmts;
            body.push(self.assign(&t, self.val(r.expr)?)?);
            nested.push((test, body));
        }
        let mut inner: Vec<Obj<'py>> = vec![];
        for (test, mut body) in nested.into_iter().rev() {
            body.extend(inner);
            inner = vec![self.iff(test, body, vec![])?];
        }
        stmts.extend(inner);
        Ok(Res::s(stmts, self.name(&t, false)?))
    }

    fn segment_kind(&self, items: &[Form]) -> &'static str {
        let head = &items[0];
        match &head.kind {
            Kind::Caret(_) => "command",
            Kind::Sym { name, .. } => {
                if self.special_head(head).is_some() {
                    "special"
                } else if WORD_OPS.contains(&name.as_str()) || ["True", "False", "None", "..."].contains(&name.as_str()) || self.is_known(head) || walrus(items) || member(head).is_some() {
                    "expr"
                } else {
                    "command"
                }
            }
            Kind::Word(ps) => {
                if matches!(ps.first(), Some(Part::Text(_))) || ps.iter().any(|p| matches!(p, Part::Text(t) if t.contains('/'))) {
                    "command"
                } else {
                    "expr"
                }
            }
            _ => "expr",
        }
    }

    fn compile_segment(&mut self, items: &[Form], children: &[Form], ctx: Ctx) -> R<Res<'py>> {
        match self.segment_kind(items) {
            "special" => self.compile_items(items, children, ctx, &[], &[]),
            "command" => self.compile_command(items, children, if ctx == Ctx::Value { "capture" } else { "attach" }),
            _ => {
                if items.len() == 1 && children.is_empty() {
                    if let Some((m, field)) = member(&items[0]) {
                        return Ok(Res::e(self.at(self.member_fn(m, field)?, &items[0])));
                    }
                    if let Kind::Sym { name, .. } = &items[0].kind {
                        if self.is_known(&items[0]) && !["True", "False", "None", "..."].contains(&name.as_str()) {
                            let e = self.rcall("auto", vec![self.name_load(&items[0])?])?;
                            return Ok(Res::e(self.at(e, &items[0])));
                        }
                    }
                    if let Kind::Attr { .. } = items[0].kind {
                        // ($x.method, (f a).method: called, as a bareword alone would be.)
                        let r = self.compile_value(&items[0], Ctx::Value)?;
                        let e = self.rcall("auto", vec![self.val(r.expr)?])?;
                        return Ok(Res::s(r.stmts, self.at(e, &items[0])));
                    }
                }
                self.compile_expr(items, ctx, children)
            }
        }
    }

    // ---- commands -------------------------------------------------------------

    fn compile_command(&mut self, items: &[Form], children: &[Form], mode: &str) -> R<Res<'py>> {
        let mut head = &items[0];
        let mut force = false;
        if let Kind::Caret(w) = &head.kind {
            force = true;
            head = w;
        }
        let pres = match &head.kind {
            Kind::Sym { name, .. } => {
                if name.starts_with('~') {
                    Res::e(self.rcall("t", vec![self.c(name.as_str())?])?)
                } else {
                    Res::e(self.c(name.as_str())?)
                }
            }
            _ => self.cmd_word(head)?,
        };
        let mut args = vec![];
        let mut redirs: Vec<(String, Res<'py>)> = vec![];
        let rest = &items[1..];
        let mut k = 0;
        while k < rest.len() {
            let x = &rest[k];
            if let Some(op) = x.op_name().or_else(|| x.sym_name()) {
                if (SPACED_REDIRS.contains(&op) || crate::reader::redir_op(op) == Some(op.len())) && k + 1 < rest.len() {
                    redirs.push((op.to_string(), self.cmd_word(&rest[k + 1])?));
                    k += 2;
                    continue;
                }
            }
            if let (Kind::Redir { op, target }, None) = (&x.kind, self.procsub_parts(x)) {
                let t = match target {
                    Some(t) => self.cmd_word(t)?,
                    None => Res::e(self.cnone()?),
                };
                redirs.push((op.clone(), t));
            } else {
                args.push(self.cmd_word(x)?);
            }
            k += 1;
        }
        for c in children {
            let (ci, cc) = line_parts(c);
            if ci.len() == 1 && cc.is_empty() {
                if let (Kind::Redir { op, target }, None) = (&ci[0].kind, self.procsub_parts(&ci[0])) {
                    let t = match target {
                        Some(t) => self.cmd_word(t)?,
                        None => Res::e(self.cnone()?),
                    };
                    redirs.push((op.clone(), t));
                } else {
                    args.push(self.cmd_word(&ci[0])?);
                }
            } else {
                args.push(self.compile_line(c, Ctx::Value)?);
            }
        }
        let nargs = args.len();
        let ops: Vec<String> = redirs.iter().map(|(o, _)| o.clone()).collect();
        let mut rs = vec![pres];
        rs.extend(args);
        rs.extend(redirs.into_iter().map(|(_, r)| r));
        let (stmts, exprs) = self.gather_e(rs)?;
        let prog = exprs[0].clone();
        let arg_es = exprs[1..1 + nargs].to_vec();
        let mut red = vec![];
        for (op, e) in ops.iter().zip(exprs[1 + nargs..].iter()) {
            red.push(self.tuple(vec![self.c(op.as_str())?, e.clone()], false)?);
        }
        let e = self.rcall("cmd", vec![self.globals()?, prog, self.list_node(arg_es)?, self.list_node(red)?, self.c(mode)?, self.c(force)?])?;
        let r = Res::s(stmts, self.at(e, &items[0]));
        // (Its <(…) and >(…) closed and waited for when it's done; in a pipe, the pipe's.)
        if mode != "stage" && self.has_procsub(items, children) {
            return self.with_procsubs(r);
        }
        Ok(r)
    }

    /// `<(…)` or `>(…)` with something run in it (each part of a pipe a
    /// program, command or call): process substitution, as in bash. With a
    /// value in it, the file it names, as before. (direction, what's in it)
    fn procsub_parts<'f>(&self, x: &'f Form) -> Option<(&'f str, &'f [Form])> {
        let Kind::Redir { op, target: Some(t) } = &x.kind else { return None };
        if op != "<" && op != ">" {
            return None;
        }
        let Kind::Group { open: '(', items } = &t.kind else { return None };
        if items.is_empty() {
            return None;
        }
        // (Something run: a program, a command, a function called; not a
        // value — $f, "name", a path, x + "y".)
        let runs = |s: &[Form]| match self.segment_kind(s) {
            "command" => true,
            "expr" => {
                s[0].sym_name().is_some_and(|n| !WORD_OPS.contains(&n) && !["True", "False", "None", "..."].contains(&n))
                    && self.is_known(&s[0])
                    && !s[1..].iter().any(|x| matches!(x.kind, Kind::Op { .. }))
            }
            _ => false,
        };
        let segs = self.pipe_split(items);
        segs.iter().all(|s| !s.is_empty() && runs(s)).then_some((op.as_str(), items.as_slice()))
    }

    fn compile_procsub(&mut self, dir: &str, items: &[Form]) -> R<Res<'py>> {
        let segs = self.pipe_split(items);
        let mut stages = vec![];
        for (k, seg) in segs.iter().enumerate() {
            stages.push(self.compile_stage(seg, &[], k == 0)?);
        }
        let (stmts, exprs) = self.gather_e(stages)?;
        Ok(Res::s(stmts, self.rcall("procsub", vec![self.c(dir)?, self.list_node(exprs)?])?))
    }

    fn has_procsub(&self, items: &[Form], children: &[Form]) -> bool {
        items.iter().chain(children.iter().filter_map(|c| match line_parts(c) { (ci, cc) if ci.len() == 1 && cc.is_empty() => ci.first(), _ => None })).any(|x| self.procsub_parts(x).is_some())
    }

    /// r, with the <(…)/>(…) it opens closed and waited for after.
    fn with_procsubs(&mut self, r: Res<'py>) -> R<Res<'py>> {
        let m = self.tmp("ps");
        let mut stmts = vec![self.assign(&m, self.rcall("ps_mark", vec![])?)?];
        stmts.extend(r.stmts);
        let e = self.rcall("ps_done", vec![self.name(&m, false)?, self.val(r.expr)?])?;
        Ok(Res::s(stmts, e))
    }

    fn list_node(&self, elts: Vec<Obj<'py>>) -> R<Obj<'py>> {
        self.n("List", vec![("elts", self.list(elts)?), ("ctx", self.ctx(false)?)])
    }

    /// An argument of a command: a string, a glob, or a value (a list becomes
    /// several arguments).
    fn cmd_word(&mut self, x: &Form) -> R<Res<'py>> {
        if let Some((d, it)) = self.procsub_parts(x) {
            return self.compile_procsub(d, it);
        }
        let glob = |t: &str| t.contains(['*', '?', '[']);
        match &x.kind {
            Kind::Sym { name, .. } | Kind::Op { name, tick: false } => {
                if glob(name) {
                    Ok(Res::e(self.rcall("g", vec![self.c(name.as_str())?])?))
                } else if name.starts_with('~') {
                    Ok(Res::e(self.rcall("t", vec![self.c(name.as_str())?])?))
                } else {
                    Ok(Res::e(self.c(name.as_str())?))
                }
            }
            Kind::Op { name, tick: true } => Ok(Res::e(self.c(format!("`{name}`"))?)),
            Kind::Num(t) => Ok(Res::e(self.c(t.as_str())?)),
            Kind::Str { value, .. } => Ok(Res::e(self.c(value.as_str())?)),
            Kind::Word(ps) => {
                let r = self.word_value(x, false)?;
                let texts: Vec<&String> = ps.iter().filter_map(|p| if let Part::Text(t) = p { Some(t) } else { None }).collect();
                let e = self.val(r.expr)?;
                if texts.iter().any(|t| glob(t)) {
                    Ok(Res::s(r.stmts, self.rcall("g", vec![e])?))
                } else if matches!(ps.first(), Some(Part::Text(t)) if t.starts_with('~')) {
                    Ok(Res::s(r.stmts, self.rcall("t", vec![e])?))
                } else {
                    Ok(Res::s(r.stmts, e))
                }
            }
            Kind::KwArg { name, value } => {
                let v = self.cmd_word(value)?;
                let e = self.val(v.expr)?;
                Ok(Res::s(v.stmts, self.joined(vec![self.c(format!("{name}="))?, self.fmt(e, None, None)?])?))
            }
            Kind::Unary { op, operand } => {
                let v = self.compile_value(operand, Ctx::Value)?;
                let e = self.val(v.expr)?;
                Ok(Res::s(v.stmts, self.joined(vec![self.c(op.as_str())?, self.fmt(e, None, None)?])?))
            }
            Kind::Unquote { form, .. } if form.sym_name().is_some() => Ok(Res::e(self.rcall("t", vec![self.c(format!("~{}", form.sym_name().unwrap()))?])?)),
            Kind::SymLit(s) => Ok(Res::e(self.c(format!(":{}", show(s)))?)),
            Kind::Comma => Ok(Res::e(self.c(",")?)),
            Kind::Caret(_) => Ok(Res::e(self.c(show(x))?)),
            Kind::Splat { value, .. } => self.compile_value(value, Ctx::Value),
            Kind::Line { .. } => self.compile_line(x, Ctx::Value),
            _ => self.compile_value(x, Ctx::Value),
        }
    }

    fn compile_pipe(&mut self, segs: &[&[Form]], children: &[Form], ctx: Ctx) -> R<Res<'py>> {
        let mut stages = vec![];
        for (k, seg) in segs.iter().enumerate() {
            if seg.is_empty() {
                return err("'|' needs something on both sides", None);
            }
            let last = k + 1 == segs.len();
            stages.push(self.compile_stage(seg, if last { children } else { &[] }, k == 0)?);
        }
        let (stmts, exprs) = self.gather_e(stages)?;
        let mode = if ctx == Ctx::Value { "capture" } else { "attach" };
        let e = self.rcall("pipe", vec![self.list_node(exprs)?, self.c(mode)?])?;
        let r = Res::s(stmts, self.at(e, &segs[0][0]));
        if segs.iter().any(|s| self.has_procsub(s, &[])) || self.has_procsub(&[], children) {
            return self.with_procsubs(r);
        }
        Ok(r)
    }

    fn compile_stage(&mut self, items: &[Form], children: &[Form], first: bool) -> R<Res<'py>> {
        match self.segment_kind(items) {
            "command" => return self.compile_command(items, children, "stage"),
            "special" => {
                let r = self.compile_items(items, children, Ctx::Value, &[], &[])?;
                let e = self.val(r.expr)?;
                return Ok(Res::s(r.stmts, self.rcall(if first { "value_stage" } else { "call_stage" }, vec![e])?));
            }
            _ => {}
        }
        let head = &items[0];
        if let Some((m, field)) = member(head) {
            // (.method args: the input is the object.)
            let (stmts, args, kws, _) = self.compile_call_args(&items[1..], children)?;
            let obj = self.attr(self.name("_tessh_in", false)?, m, false)?;
            let body = if field { obj } else { self.call(obj, args, kws)? };
            let lam = self.lambda(self.one_arg("_tessh_in")?, body)?;
            return Ok(Res::s(stmts, self.rcall("call_stage", vec![lam])?));
        }
        let is_call = (head.sym_name().is_some() && self.is_known(head)) || items.len() > 1 || !children.is_empty();
        if !is_call {
            if !first {
                return err("only a program or a function can take a pipe's input", Some(head));
            }
            let v = self.compile_value(head, Ctx::Value)?;
            let e = self.val(v.expr)?;
            return Ok(Res::s(v.stmts, self.rcall("value_stage", vec![e])?));
        }
        let f = self.compile_head(head)?;
        let (astmts, mut args, kws, _) = self.compile_call_args(&items[1..], children)?;
        let (stmts, exprs) = self.gather_e(vec![f, Res { stmts: astmts, expr: None }])?;
        // (The first of a pipe has no input to be given: `gen | sort`, <(gen).)
        if !first {
            args.push(self.name("_tessh_in", false)?);
        }
        let lam_args = self.n("arguments", vec![("posonlyargs", self.list(vec![])?), ("args", self.list(vec![self.n("arg", vec![("arg", self.s("_tessh_in"))])?])?), ("kwonlyargs", self.list(vec![])?), ("kw_defaults", self.list(vec![])?), ("defaults", self.list(vec![])?)])?;
        let lam = self.lambda(lam_args, self.call(exprs[0].clone(), args, kws)?)?;
        Ok(Res::s(stmts, self.rcall("call_stage", vec![lam])?))
    }

    fn compile_bg(&mut self, items: &[Form], children: &[Form]) -> R<Res<'py>> {
        let segs = self.pipe_split(items);
        if segs.len() > 1 {
            let r = self.compile_pipe(&segs, children, Ctx::Stmt)?;
            let e = self.val(r.expr)?;
            let args = e.getattr("args")?;
            args.set_item(1, self.c("bg")?)?;
            return Ok(Res::s(r.stmts, e));
        }
        if self.segment_kind(items) == "command" {
            return self.compile_command(items, children, "bg");
        }
        let r = self.compile_items(items, children, Ctx::Value, &[], &[])?;
        if !r.stmts.is_empty() {
            return err("only a command or a call can run in the background", items.first());
        }
        let lam = self.lambda(self.noargs()?, self.val(r.expr)?)?;
        Ok(Res::e(self.rcall("bg_call", vec![lam])?))
    }

    // ---- expressions ----------------------------------------------------------

    pub fn compile_expr(&mut self, items: &[Form], ctx: Ctx, children: &[Form]) -> R<Res<'py>> {
        if items.is_empty() {
            return err("expected an expression", children.first());
        }
        if items.len() > 1 && (items.iter().any(|x| x.is_op("||") || x.is_op("&&")) || items[items.len() - 1].is_op("&") || self.pipe_split(items).len() > 1) {
            return self.compile_shell(items, children, ctx);
        }
        if self.special_head(&items[0]).is_some() {
            return self.compile_items(items, children, ctx, &[], &[]);
        }
        if let Some(op) = items[0].op_name() {
            if items.len() > 1 && (binop(op).is_some() || cmpop(op).is_some() || self.st.ops.contains_key(op) || op == "!") && !items[1..].iter().any(|x| x.op_name().is_some_and(|n| binop(n).is_some() || cmpop(n).is_some())) {
                return self.lisp_op(&items[0], &items[1..]);
            }
        }
        if items.iter().any(|x| x.is_comma()) {
            let mut rs = vec![];
            for p in comma_parts(items) {
                rs.push(self.compile_expr(p, Ctx::Value, &[])?);
            }
            let (stmts, exprs) = self.gather_e(rs)?;
            return Ok(Res::s(stmts, self.tuple(exprs, false)?));
        }
        let toks = self.tokens(items);
        let (tree, k) = self.parse(&toks, 0, 0)?;
        if k != toks.len() {
            let f = match &toks[k] {
                Tok::Op(_, f) | Tok::Pre(_, f) => Some(f.clone()),
                _ => items.first().cloned(),
            };
            return err("unexpected operator", f.as_ref());
        }
        if !children.is_empty() && !matches!(tree, Tree::Chunk(_)) {
            return err("indented arguments can only follow a call", children.first());
        }
        self.compile_tree(tree, ctx, children)
    }

    fn lisp_op(&mut self, op: &Form, args: &[Form]) -> R<Res<'py>> {
        let mut rs = vec![];
        for a in args {
            rs.push(self.compile_value(a, Ctx::Value)?);
        }
        let (stmts, exprs) = self.gather_e(rs)?;
        let n = op.op_name().unwrap_or("");
        if exprs.len() == 1 && prefix_prec(n).is_some() {
            return Ok(Res::s(stmts, self.unary(n, exprs[0].clone())?));
        }
        if let Some(c) = cmpop(n) {
            let ops: Vec<Obj> = (1..exprs.len()).map(|_| self.n0(c)).collect::<R<_>>()?;
            return Ok(Res::s(stmts, self.n("Compare", vec![("left", exprs[0].clone()), ("ops", self.list(ops)?), ("comparators", self.list(exprs[1..].to_vec())?)])?));
        }
        let mut e = exprs[0].clone();
        for x in &exprs[1..] {
            e = self.binop_node(n, e, x.clone(), op)?;
        }
        Ok(Res::s(stmts, e))
    }

    fn tokens(&self, items: &[Form]) -> Vec<Tok> {
        let mut toks: Vec<Tok> = vec![];
        for x in items {
            match &x.kind {
                Kind::Op { name, tick } => {
                    if *tick {
                        toks.push(Tok::Op(format!("`{name}"), x.clone()));
                    } else if binop(name).is_some() || cmpop(name).is_some() || self.st.ops.contains_key(name) || [":=", "!", "||", "&&"].contains(&name.as_str()) {
                        toks.push(Tok::Op(name.clone(), x.clone()));
                    } else {
                        toks.push(Tok::X(Form::sym_at(name, &[], x.pos)));
                    }
                }
                Kind::Sym { name, .. } if WORD_OPS.contains(&name.as_str()) => toks.push(Tok::Op(name.clone(), x.clone())),
                _ => toks.push(Tok::X(x.clone())),
            }
        }
        // (and, or, in, is next to a bareword are words: print it is ready.)
        let bare = |t: Option<&Tok>| matches!(t, Some(Tok::X(f)) if f.sym_name().is_some_and(|n| !["True", "False", "None", "..."].contains(&n)));
        let mut words = vec![];
        for k in 0..toks.len() {
            if let Tok::Op(n, f) = &toks[k] {
                if matches!(f.kind, Kind::Sym { .. }) && ["and", "or", "in", "is"].contains(&n.as_str()) && (bare(if k > 0 { toks.get(k - 1) } else { None }) || bare(toks.get(k + 1))) {
                    words.push(k);
                }
            }
        }
        for k in words {
            if let Tok::Op(_, f) = &toks[k] {
                toks[k] = Tok::X(f.clone());
            }
        }
        // not in, is not
        let mut merged: Vec<Tok> = vec![];
        for t in toks {
            if let Tok::Op(n, f) = &t {
                if let Some(Tok::Op(pn, _)) = merged.last() {
                    if pn == "not" && n == "in" && merged.len() > 1 && matches!(merged[merged.len() - 2], Tok::X(_)) {
                        let l = merged.len() - 1;
                        merged[l] = Tok::Op("not in".into(), f.clone());
                        continue;
                    }
                    if pn == "is" && n == "not" {
                        let l = merged.len() - 1;
                        merged[l] = Tok::Op("is not".into(), f.clone());
                        continue;
                    }
                }
            }
            merged.push(t);
        }
        // Operators with nothing on a side are words.
        let mut out: Vec<Tok> = vec![];
        let mut want = true;
        for t in merged {
            match t {
                Tok::Op(n, f) => {
                    if want {
                        if prefix_prec(&n).is_some() {
                            out.push(Tok::Pre(n, f));
                            continue;
                        }
                        out.push(Tok::X(Form::sym_at(&n, &[], f.pos)));
                        want = false;
                    } else {
                        out.push(Tok::Op(n, f));
                        want = true;
                    }
                }
                x => {
                    out.push(x);
                    want = false;
                }
            }
        }
        if let Some(Tok::Op(n, f)) | Some(Tok::Pre(n, f)) = out.last() {
            let w = Form::sym_at(n.trim_start_matches('`'), &[], f.pos);
            out.pop();
            out.push(Tok::X(w));
        }
        let mut chunks: Vec<Tok> = vec![];
        for t in out {
            match t {
                Tok::X(f) => match chunks.last_mut() {
                    Some(Tok::Chunk(v)) => v.push(f),
                    _ => chunks.push(Tok::Chunk(vec![f])),
                },
                t => chunks.push(t),
            }
        }
        chunks
    }

    fn op_prec(&self, n: &str) -> (u8, Assoc) {
        if n.starts_with('`') {
            return (6, Assoc::Left);
        }
        if let Some((p, a, _)) = self.st.ops.get(n) {
            return (*p, *a);
        }
        (prec(n), if n == "**" || n == ":=" { Assoc::Right } else { Assoc::Left })
    }

    fn parse(&self, toks: &[Tok], k: usize, min: u8) -> R<(Tree, usize)> {
        let (mut lhs, mut k) = match toks.get(k) {
            None => return err("expected something after the operator", None),
            Some(Tok::Pre(n, f)) => {
                let p = prefix_prec(n).unwrap_or(12);
                let (operand, k2) = self.parse(toks, k + 1, p)?;
                (Tree::Un(n.clone(), Box::new(operand), f.clone()), k2)
            }
            Some(Tok::Chunk(v)) => (Tree::Chunk(v.clone()), k + 1),
            Some(Tok::Op(_, f)) => return err("expected an operand", Some(f)),
            Some(Tok::X(f)) => (Tree::Chunk(vec![f.clone()]), k + 1),
        };
        while let Some(Tok::Op(n, f)) = toks.get(k) {
            let (p, assoc) = self.op_prec(n);
            if p < min {
                break;
            }
            if cmpop(n).is_some() {
                let mut ops = vec![];
                let mut rhss = vec![];
                while let Some(Tok::Op(n2, f2)) = toks.get(k) {
                    if cmpop(n2).is_none() {
                        break;
                    }
                    ops.push((n2.clone(), f2.clone()));
                    let (r, k2) = self.parse(toks, k + 1, p + 1)?;
                    rhss.push(r);
                    k = k2;
                }
                lhs = Tree::Cmp(Box::new(lhs), ops, rhss);
                continue;
            }
            let next = if assoc == Assoc::Right { p } else { p + 1 };
            let (rhs, k2) = self.parse(toks, k + 1, next)?;
            lhs = Tree::Bin(n.clone(), Box::new(lhs), Box::new(rhs), f.clone());
            k = k2;
            if assoc == Assoc::None {
                if let Some(Tok::Op(n2, f2)) = toks.get(k) {
                    if n2 == n {
                        return err(format!("'{n}' can't be chained"), Some(f2));
                    }
                }
            }
        }
        Ok((lhs, k))
    }

    fn compile_tree(&mut self, t: Tree, ctx: Ctx, children: &[Form]) -> R<Res<'py>> {
        match t {
            Tree::Chunk(v) => self.compile_operand(&v, ctx, children),
            Tree::Un(op, x, f) => {
                let sub = if (op == "not" || op == "!") && ctx == Ctx::Cond { Ctx::Cond } else { Ctx::Value };
                let r = self.compile_tree(*x, sub, &[])?;
                let e = self.val(r.expr)?;
                Ok(Res::s(r.stmts, self.at(self.unary(&op, e)?, &f)))
            }
            Tree::Cmp(l, ops, rhss) => {
                let mut rs = vec![self.compile_tree(*l, Ctx::Value, &[])?];
                for r in rhss {
                    rs.push(self.compile_tree(r, Ctx::Value, &[])?);
                }
                let (stmts, exprs) = self.gather_e(rs)?;
                let opn: Vec<Obj> = ops.iter().map(|(o, _)| self.n0(cmpop(o).unwrap())).collect::<R<_>>()?;
                let e = self.n("Compare", vec![("left", exprs[0].clone()), ("ops", self.list(opn)?), ("comparators", self.list(exprs[1..].to_vec())?)])?;
                Ok(Res::s(stmts, self.at(e, &ops[0].1)))
            }
            Tree::Bin(op, l, r, f) => {
                if ["and", "&&", "or", "||"].contains(&op.as_str()) {
                    let is_and = op == "and" || op == "&&";
                    let sub = if ctx == Ctx::Cond { Ctx::Cond } else { Ctx::Value };
                    let mut flat = vec![];
                    flatten(Tree::Bin(op, l, r, f), is_and, &mut flat);
                    let mut rs = vec![];
                    for x in flat {
                        rs.push(self.compile_tree(x, sub, &[])?);
                    }
                    return self.bool_chain(is_and, rs);
                }
                if op == ":=" {
                    let target = match *l {
                        Tree::Chunk(v) if v.len() == 1 && matches!(v[0].kind, Kind::Sym { .. } | Kind::Var(_)) => v[0].clone(),
                        _ => return err("':=' needs a name on its left", Some(&f)),
                    };
                    let v = self.compile_tree(*r, Ctx::Value, &[])?;
                    let py = self.bind(&target)?;
                    let e = self.n("NamedExpr", vec![("target", self.name(&py, true)?), ("value", self.val(v.expr)?)])?;
                    return Ok(Res::s(v.stmts, e));
                }
                let a = self.compile_tree(*l, Ctx::Value, &[])?;
                let b = self.compile_tree(*r, Ctx::Value, &[])?;
                let (stmts, exprs) = self.gather_e(vec![a, b])?;
                let e = self.binop_node(&op, exprs[0].clone(), exprs[1].clone(), &f)?;
                Ok(Res::s(stmts, self.at(e, &f)))
            }
        }
    }

    fn binop_node(&self, op: &str, a: Obj<'py>, b: Obj<'py>, f: &Form) -> R<Obj<'py>> {
        if let Some(n) = op.strip_prefix('`') {
            return self.call(self.name_load(&Form::sym_at(n, &[], f.pos))?, vec![a, b], vec![]);
        }
        if let Some((_, _, func)) = self.st.ops.get(op) {
            let func = func.clone();
            return self.call(self.name_load(&func)?, vec![a, b], vec![]);
        }
        // (=~: a match — a regex, a glob, or text as it is; !~: none.)
        if op == "=~" {
            return self.rcall("matches", vec![a, b]);
        }
        if op == "!~" {
            let m = self.rcall("matches", vec![a, b])?;
            return self.n("UnaryOp", vec![("op", self.n0("Not")?), ("operand", m)]);
        }
        match binop(op) {
            Some(n) => self.n("BinOp", vec![("left", a), ("op", self.n0(n)?), ("right", b)]),
            None => err(format!("unknown operator '{op}'"), Some(f)),
        }
    }

    fn compile_operand(&mut self, chunk: &[Form], ctx: Ctx, children: &[Form]) -> R<Res<'py>> {
        if chunk.len() == 1 && children.is_empty() {
            return self.compile_value(&chunk[0], ctx);
        }
        self.compile_application(chunk, children, ctx)
    }

    fn compile_head(&mut self, head: &Form) -> R<Res<'py>> {
        if head.sym_name().is_some() {
            return Ok(Res::e(self.name_load(head)?));
        }
        self.compile_value(head, Ctx::Value)
    }

    fn compile_application(&mut self, items: &[Form], children: &[Form], ctx: Ctx) -> R<Res<'py>> {
        let head = &items[0];
        let mode = if ctx == Ctx::Value { "capture" } else { "attach" };
        // (True, False, None, ... alone: the constant.)
        if items.len() == 1 && children.is_empty() && head.sym_name().is_some_and(|n| ["True", "False", "None", "..."].contains(&n)) {
            return self.compile_value(head, ctx);
        }
        if let Some((m, field)) = member(head) {
            return self.compile_member(m, field, &items[1..], children, head);
        }
        if head.sym_name().is_some() && !walrus(items) {
            if self.special_head(head).is_some() {
                return self.compile_items(items, children, ctx, &[], &[]);
            }
            if !self.is_known(head) {
                return self.compile_command(items, children, mode);
            }
        } else if matches!(head.kind, Kind::Caret(_) | Kind::Word(_)) && self.segment_kind(items) == "command" {
            return self.compile_command(items, children, mode);
        }
        let f = self.compile_head(head)?;
        let (astmts, args, kws, reds) = self.compile_call_args(&items[1..], children)?;
        let (stmts, exprs) = self.gather_e(vec![f, Res { stmts: astmts, expr: None }])?;
        let mut c = self.at(self.call(exprs[0].clone(), args, kws)?, head);
        if !reds.is_empty() {
            let mut red = vec![];
            for (op, e) in reds {
                red.push(self.tuple(vec![self.c(op.as_str())?, e], false)?);
            }
            c = self.rcall("redirect", vec![self.lambda(self.noargs()?, c)?, self.list_node(red)?])?;
        }
        if self.has_procsub(&items[1..], children) {
            return self.with_procsubs(Res::s(stmts, c));
        }
        Ok(Res::s(stmts, c))
    }

    fn one_arg(&self, name: &str) -> R<Obj<'py>> {
        self.n("arguments", vec![("posonlyargs", self.list(vec![])?), ("args", self.list(vec![self.n("arg", vec![("arg", self.s(name))])?])?), ("kwonlyargs", self.list(vec![])?), ("kw_defaults", self.list(vec![])?), ("defaults", self.list(vec![])?)])
    }

    /// `.method obj args…` and `.-field obj`, as in Clojure.
    fn compile_member(&mut self, m: &str, field: bool, rest: &[Form], children: &[Form], head: &Form) -> R<Res<'py>> {
        if rest.is_empty() && children.is_empty() {
            return Ok(Res::e(self.at(self.member_fn(m, field)?, head)));
        }
        let (stmts, mut args, kws, reds) = self.compile_call_args(rest, children)?;
        if args.is_empty() || args[0].get_type().name().map(|n| n.to_string() == "Starred").unwrap_or(false) {
            return err(format!("'{}' needs the object first", show(head)), Some(head));
        }
        if !reds.is_empty() {
            return err("a method call can't redirect: wrap it in a do block", Some(head));
        }
        let obj = args.remove(0);
        let a = self.attr(obj, m, false)?;
        if field {
            if !args.is_empty() || !kws.is_empty() {
                return err(format!("'.-{m}' takes only the object"), Some(head));
            }
            return Ok(Res::s(stmts, self.at(a, head)));
        }
        Ok(Res::s(stmts, self.at(self.call(a, args, kws)?, head)))
    }

    /// `.method` as a value: a function calling that method on its first
    /// argument (`.-field`: reading that field).
    fn member_fn(&self, m: &str, field: bool) -> R<Obj<'py>> {
        let o = self.name("_tessh_o", false)?;
        if field {
            return self.lambda(self.one_arg("_tessh_o")?, self.attr(o, m, false)?);
        }
        let arg = |n: &str| self.n("arg", vec![("arg", self.s(n))]);
        let args = self.n("arguments", vec![("posonlyargs", self.list(vec![])?), ("args", self.list(vec![arg("_tessh_o")?])?), ("vararg", arg("_tessh_a")?), ("kwonlyargs", self.list(vec![])?), ("kw_defaults", self.list(vec![])?), ("kwarg", arg("_tessh_k")?), ("defaults", self.list(vec![])?)])?;
        let call = self.call(self.attr(o, m, false)?, vec![self.starred(self.name("_tessh_a", false)?, false)?], vec![self.keyword(None, self.name("_tessh_k", false)?)?])?;
        self.lambda(args, call)
    }

    /// ->, ->>, some->, some->>, as->, doto.
    fn sf_thread(&mut self, n: &str, items: &[Form], children: &[Form], ctx: Ctx) -> R<Res<'py>> {
        let head = &items[0];
        let Some(init) = items.get(1) else { return err(format!("{n} value steps…"), Some(head)) };
        let mut k = 2;
        let name = if n == "as->" {
            k = 3;
            match items.get(2) {
                Some(x) if x.sym_name().is_some_and(is_ident) => x.clone(),
                _ => return err("as-> value name steps…", Some(head)),
            }
        } else {
            self.st.marks += 1;
            Form::sym_at("v", &[self.st.marks], head.pos)
        };
        let mut steps: Vec<Form> = items[k.min(items.len())..].to_vec();
        for c in children {
            let (ci, cc) = line_parts(c);
            if !cc.is_empty() {
                return err(format!("a step of {n} goes on one line"), Some(c));
            }
            steps.push(if ci.len() == 1 { ci[0].clone() } else { Form::group('(', ci.to_vec(), c.pos) });
        }
        let first = !n.ends_with("->>");
        if n == "->" || n == "->>" {
            let mut acc = init.clone();
            for s in &steps {
                acc = thread_into(s, acc, first);
            }
            return self.compile_value(&acc, ctx);
        }
        let pos = head.pos;
        let var = Form::new(Kind::Var(Box::new(name.clone())), pos);
        let eq = || Form::new(Kind::Op { name: "=".into(), tick: false }, pos);
        let mut lines = vec![Form::line(vec![name.clone(), eq(), init.clone()], vec![], pos)];
        for s in &steps {
            lines.push(match n {
                "as->" => Form::line(vec![name.clone(), eq(), s.clone()], vec![], s.pos),
                "doto" => Form::line(vec![thread_into(s, var.clone(), true)], vec![], s.pos),
                _ => {
                    let set = Form::line(vec![name.clone(), eq(), thread_into(s, var.clone(), first)], vec![], s.pos);
                    Form::line(vec![Form::sym("if"), var.clone(), Form::sym("is"), Form::sym("not"), Form::sym("None")], vec![set], s.pos)
                }
            });
        }
        lines.push(Form::line(vec![var], vec![], pos));
        self.compile_body(&lines, ctx)
    }

    /// `py "code"`: Python as it is: an expression's value, or statements.
    fn sf_py(&mut self, items: &[Form]) -> R<Res<'py>> {
        let head = &items[0];
        let code = match items.get(1).map(|x| &x.kind) {
            Some(Kind::Str { value, kind }) if *kind != 'b' && items.len() == 2 => value.clone(),
            Some(Kind::IStr(_)) => return err("py takes Python as a plain string: use single quotes (Python has its own {…})", items.get(1)),
            _ => return err("py \"Python code\"", Some(head)),
        };
        let code = dedent(&code);
        let line = head.pos.map(|p| p.line).unwrap_or(1);
        let parse = self.ast.getattr("parse")?;
        let bump = |n: &Obj<'py>| -> R<()> {
            self.ast.getattr("increment_lineno")?.call1((n, line.saturating_sub(1)))?;
            Ok(())
        };
        let kw = |mode: &str| -> R<Bound<'py, PyDict>> {
            let d = PyDict::new(self.py);
            d.set_item("mode", mode)?;
            Ok(d)
        };
        if let Ok(t) = parse.call((code.trim(),), Some(&kw("eval")?)) {
            let body = t.getattr("body")?;
            bump(&body)?;
            return Ok(Res::e(body));
        }
        let t = match parse.call((code.as_str(),), Some(&kw("exec")?)) {
            Ok(t) => t,
            Err(e) => return err(format!("py: {}", e.value(self.py)), Some(head)),
        };
        let mut stmts = vec![];
        for s in t.getattr("body")?.try_iter()? {
            let s = s?;
            bump(&s)?;
            stmts.push(s);
        }
        Ok(Res::s(stmts, self.cnone()?))
    }

    /// `let a=1 b=(f $a)` then a block (or the rest of the line): names that
    /// only the block sees. Bound in order, so a later one can use an earlier.
    fn sf_let(&mut self, items: &[Form], children: &[Form], ctx: Ctx) -> R<Res<'py>> {
        let head = &items[0];
        self.st.marks += 1;
        let mark = self.st.marks;
        let mut names: Vec<String> = vec![];
        let rename = |f: Form, names: &[String]| -> Form {
            f.map(&mut |x| match &x.kind {
                Kind::Sym { name, marks } if marks.is_empty() && names.iter().any(|n| n == name || name.strip_prefix('*') == Some(n.as_str())) => Form::sym_at(name, &[mark], x.pos),
                _ => x,
            })
        };
        let mut lines = vec![];
        let mut k = 1;
        while let Some(Kind::KwArg { name, value }) = items.get(k).map(|x| &x.kind) {
            let v = rename((**value).clone(), &names);
            names.push(name.clone());
            let eq = Form::new(Kind::Op { name: "=".into(), tick: false }, items[k].pos);
            lines.push(Form::line(vec![Form::sym_at(name, &[mark], items[k].pos), eq, v], vec![], items[k].pos));
            k += 1;
        }
        if names.is_empty() {
            return err("let name=value … then the block that sees them", Some(head));
        }
        if k < items.len() {
            lines.push(rename(Form::line(items[k..].to_vec(), vec![], items[k].pos), &names));
        }
        for c in children {
            lines.push(rename(c.clone(), &names));
        }
        if lines.len() == names.len() {
            return err("let needs a block (or a form after its names)", Some(head));
        }
        self.compile_body(&lines, ctx)
    }

    /// `require mod` (or `require mod name…`): that module's macros (and
    /// operators), here, while this compiles. The module is imported now.
    fn sf_require(&mut self, items: &[Form]) -> R<Res<'py>> {
        let head = &items[0];
        let Some(module) = items.get(1).and_then(|x| x.sym_name()).filter(|n| is_dotted(n)) else { return err("require module [macro …]", Some(head)) };
        let want: Vec<String> = items[2..].iter().filter_map(|x| x.sym_name().map(String::from)).collect();
        let m = match self.py.import("importlib")?.getattr("import_module")?.call1((module,)) {
            Ok(m) => m,
            Err(e) => return err(format!("require {module}: {}", e.value(self.py)), Some(head)),
        };
        let Ok(st) = m.getattr("__tessh_state__") else { return err(format!("{module} isn't a tessh module (no macros)"), Some(head)) };
        let sb = st.cast_into::<crate::runner::StateBox>().map_err(|e| PyErr::from(e))?;
        let sb = sb.borrow();
        let theirs = sb.st.lock().unwrap();
        for w in &want {
            if !theirs.macros.contains_key(w) {
                return err(format!("{module} has no macro {w}"), Some(head));
            }
        }
        for (n, mac) in theirs.macros.iter() {
            if want.is_empty() || want.contains(n) {
                self.st.macros.insert(n.clone(), Macro { name: mac.name.clone(), func: mac.func.clone_ref(self.py), params: mac.params.clone() });
            }
        }
        if want.is_empty() {
            for (n, op) in theirs.ops.iter() {
                self.st.ops.insert(n.clone(), op.clone());
            }
        }
        Ok(Res::s(vec![self.pass()?], self.cnone()?))
    }

    /// `{a b c=0 **rest} = value`: names from a dict's keys (or an object's
    /// attributes).
    fn destructure(&mut self, ks: &[Form], g: &Form, value: Res<'py>) -> R<Res<'py>> {
        let mut targets = vec![];
        let mut keys = vec![];
        let mut dkeys = vec![];
        let mut dvals = vec![];
        let mut rest = false;
        for x in ks {
            match &x.kind {
                Kind::Comma => {}
                Kind::Sym { name, marks } if name.strip_prefix("**").is_some_and(is_ident) && !rest => {
                    targets.push(Form::sym_at(&name[2..], marks, x.pos));
                    rest = true;
                }
                Kind::Sym { name, .. } if is_ident(name) && !rest => {
                    targets.push(x.clone());
                    keys.push(name.clone());
                }
                Kind::KwArg { name, value } if !rest => {
                    targets.push(Form::sym_at(name, &[], x.pos));
                    keys.push(name.clone());
                    dkeys.push(name.clone());
                    dvals.push(self.compile_value(value, Ctx::Value)?);
                }
                Kind::Splat { value, n: 2 } if value.sym_name().is_some_and(is_ident) && !rest => {
                    targets.push((**value).clone());
                    rest = true;
                }
                _ => return err("{name …} = takes names, name=default and **rest last", Some(x)),
            }
        }
        if targets.is_empty() {
            return err("nothing to assign to", Some(g));
        }
        let mut rs = vec![value];
        rs.extend(dvals);
        let (mut stmts, es) = self.gather_e(rs)?;
        let kl = self.list_node(keys.iter().map(|k| self.c(k.as_str())).collect::<R<_>>()?)?;
        let dk = self.list(dkeys.iter().map(|k| self.c(k.as_str())).collect::<R<_>>()?)?;
        let dict = self.n("Dict", vec![("keys", dk), ("values", self.list(es[1..].to_vec())?)])?;
        let call = self.rcall("pick", vec![es[0].clone(), kl, dict, self.c(rest)?])?;
        let mut ts = vec![];
        for t in &targets {
            let r = self.make_target(std::slice::from_ref(t), g, false, false)?;
            stmts.extend(r.stmts);
            ts.push(self.val(r.expr)?);
        }
        let tuple = self.n("Tuple", vec![("elts", self.list(ts)?), ("ctx", self.n0("Store")?)])?;
        stmts.push(self.at(self.n("Assign", vec![("targets", self.list(vec![tuple])?), ("value", call)])?, g));
        Ok(Res::s(stmts, self.cnone()?))
    }

    /// (statements, args, keywords, redirections)
    #[allow(clippy::type_complexity)]
    fn compile_call_args(&mut self, items: &[Form], children: &[Form]) -> R<(Vec<Obj<'py>>, Vec<Obj<'py>>, Vec<Obj<'py>>, Vec<(String, Obj<'py>)>)> {
        let mut rs = vec![];
        let mut slots: Vec<(&'static str, String)> = vec![];
        for x in items {
            self.call_arg(x, &mut rs, &mut slots)?;
        }
        for c in children {
            let (ci, cc) = line_parts(c);
            if ci.len() == 1 && cc.is_empty() {
                self.call_arg(&ci[0], &mut rs, &mut slots)?;
            } else {
                rs.push(self.compile_line(c, Ctx::Value)?);
                slots.push(("arg", String::new()));
            }
        }
        let (stmts, exprs) = self.gather_e(rs)?;
        let (mut args, mut kws, mut reds) = (vec![], vec![], vec![]);
        for ((kind, extra), e) in slots.into_iter().zip(exprs) {
            match kind {
                "arg" => args.push(e),
                "star" => args.push(self.starred(e, false)?),
                "kw" => kws.push(self.keyword(Some(&extra), e)?),
                "kwstar" => kws.push(self.keyword(None, e)?),
                _ => reds.push((extra, e)),
            }
        }
        Ok((stmts, args, kws, reds))
    }

    fn call_arg(&mut self, x: &Form, rs: &mut Vec<Res<'py>>, slots: &mut Vec<(&'static str, String)>) -> R<()> {
        match &x.kind {
            Kind::KwArg { name, value } => {
                rs.push(self.compile_value(value, Ctx::Value)?);
                slots.push(("kw", name.clone()));
            }
            Kind::Splat { value, n } => {
                rs.push(self.compile_value(value, Ctx::Value)?);
                slots.push((if *n == 1 { "star" } else { "kwstar" }, String::new()));
            }
            Kind::Redir { .. } if self.procsub_parts(x).is_some() => {
                rs.push(self.cmd_word(x)?);
                slots.push(("arg", String::new()));
            }
            Kind::Redir { op, target } => {
                rs.push(match target {
                    Some(t) => self.cmd_word(t)?,
                    None => Res::e(self.cnone()?),
                });
                slots.push(("redir", op.clone()));
            }
            Kind::Comma => return err("arguments are separated by spaces, not commas", Some(x)),
            // (A glob: a Glob, which a function iterates for the names.)
            _ if is_glob_word(x) => {
                rs.push(self.cmd_word(x)?);
                slots.push(("arg", String::new()));
            }
            _ => {
                rs.push(self.compile_value(x, Ctx::Value)?);
                slots.push(("arg", String::new()));
            }
        }
        Ok(())
    }

    // ---- values ---------------------------------------------------------------

    fn num(&self, t: &str) -> R<Obj<'py>> {
        let v = self.ast.getattr("literal_eval")?.call1((t.replace('_', ""),))?;
        self.n("Constant", vec![("value", v)])
    }

    pub fn compile_value(&mut self, x: &Form, ctx: Ctx) -> R<Res<'py>> {
        let e = match &x.kind {
            Kind::Sym { name, .. } => match name.as_str() {
                "True" => self.c(true)?,
                "False" => self.c(false)?,
                "None" => self.cnone()?,
                "..." => self.n("Constant", vec![("value", self.py.Ellipsis().into_bound(self.py))])?,
                n if n.contains(['*', '?', '[']) => self.rcall("g", vec![self.c(n)?])?,
                n if is_path_word(n) => self.rcall("path_word", vec![self.c(n)?])?,
                n if n.starts_with('~') => self.rcall("t", vec![self.c(n)?])?,
                n => self.c(n)?,
            },
            Kind::Num(t) => self.num(t)?,
            Kind::Str { value, kind } => {
                if *kind == 'x' {
                    return Ok(Res::e(self.rcall("regex", vec![self.c(value.as_str())?])?));
                }
                if *kind == 'b' {
                    let bytes: Vec<u8> = value.chars().map(|c| c as u32 as u8).collect();
                    self.c(pyo3::types::PyBytes::new(self.py, &bytes))?
                } else {
                    self.c(value.as_str())?
                }
            }
            // (A word with a glob in it: a Glob of what it spells.)
            Kind::Word(_) if is_glob_word(x) => {
                let w = self.word_value(x, false)?;
                let e = self.val(w.expr)?;
                return Ok(Res::s(w.stmts, self.rcall("g", vec![e])?));
            }
            Kind::IStr(_) | Kind::Word(_) => return self.word_value(x, true),
            Kind::Var(_) => self.var_load(x)?,
            Kind::Status => self.rcall("status", vec![])?,
            Kind::Attr { obj, name } => {
                let r = self.compile_value(obj, Ctx::Value)?;
                let e = self.attr(self.val(r.expr)?, name, false)?;
                return Ok(Res::s(r.stmts, self.at(e, x)));
            }
            Kind::Index { obj, items } => {
                let r = self.compile_value(obj, Ctx::Value)?;
                let s = self.compile_subscript(items, x)?;
                let (stmts, es) = self.gather_e(vec![r, s])?;
                let e = self.n("Subscript", vec![("value", es[0].clone()), ("slice", es[1].clone()), ("ctx", self.ctx(false)?)])?;
                return Ok(Res::s(stmts, self.at(e, x)));
            }
            Kind::Unary { op, operand } => {
                let r = self.compile_value(operand, Ctx::Value)?;
                let e = self.unary(op, self.val(r.expr)?)?;
                return Ok(Res::s(r.stmts, self.at(e, x)));
            }
            Kind::Group { .. } => return self.compile_group(x, ctx),
            Kind::Splat { value, n } => {
                if *n == 2 {
                    return err("** only goes in a call or a dict", Some(x));
                }
                let r = self.compile_value(value, Ctx::Value)?;
                let e = self.starred(self.val(r.expr)?, false)?;
                return Ok(Res::s(r.stmts, e));
            }
            Kind::KwArg { name, .. } => return err(format!("keyword argument {name}= outside a call"), Some(x)),
            Kind::SymLit(s) => return self.quote(s, false),
            Kind::Quasi(g) => return self.quote(&group_form(g), true),
            Kind::Unquote { form, .. } => match form.sym_name() {
                Some(n) => self.rcall("t", vec![self.c(format!("~{n}"))?])?,
                None => return err("~ outside a quasi template", Some(x)),
            },
            Kind::Op { name, .. } => self.c(name.as_str())?,
            Kind::Line { .. } => return self.compile_line(x, ctx),
            Kind::Caret(_) => return self.compile_command(std::slice::from_ref(x), &[], "capture"),
            Kind::Redir { .. } => return err("a redirection only goes after a command or a call", Some(x)),
            Kind::Comma | Kind::Semi => return err("unexpected separator", Some(x)),
            Kind::Interp { .. } => return self.compile_interp(x),
        };
        Ok(Res::e(self.at(e, x)))
    }

    fn var_load(&mut self, v: &Form) -> R<Obj<'py>> {
        let s = match &v.kind {
            Kind::Var(s) => s,
            _ => v,
        };
        if matches!(s.kind, Kind::Unquote { .. }) {
            return err("$~ outside a quasi template", Some(v));
        }
        let name = s.sym_name().unwrap_or("");
        if self.is_known(s) || !s.marks().is_empty() || self.ns_get(name).is_some() {
            let e = self.name_load(s)?;
            return Ok(self.at(e, v));
        }
        let e = self.rcall("lookup", vec![self.globals()?, self.c(name)?])?;
        Ok(self.at(e, v))
    }

    fn joined(&self, vals: Vec<Obj<'py>>) -> R<Obj<'py>> {
        self.n("JoinedStr", vec![("values", self.list(vals)?)])
    }

    fn fmt(&self, e: Obj<'py>, conv: Option<char>, spec: Option<&str>) -> R<Obj<'py>> {
        let conv = conv.map(|c| c as i64).unwrap_or(-1);
        let spec = match spec {
            Some(s) => self.joined(vec![self.c(s)?])?,
            None => self.none(),
        };
        self.n("FormattedValue", vec![("value", e), ("conversion", PyInt::new(self.py, conv).into_any()), ("format_spec", spec)])
    }

    fn compile_interp(&mut self, ip: &Form) -> R<Res<'py>> {
        let Kind::Interp { expr, braced, .. } = &ip.kind else { return self.compile_value(ip, Ctx::Value) };
        if let Kind::Group { items, .. } = &expr.kind {
            if *braced {
                if items.len() == 1 {
                    if let Some(n) = items[0].sym_name() {
                        if is_dotted(n) {
                            let root = n.split('.').next().unwrap();
                            if self.is_known(&Form::sym(root)) || self.ns_get(root).is_some() {
                                return Ok(Res::e(self.name_load(&items[0])?));
                            }
                            if !n.contains('.') {
                                return Ok(Res::e(self.rcall("lookup", vec![self.globals()?, self.c(n)?])?));
                            }
                        }
                    }
                }
                return self.compile_expr(items, Ctx::Value, &[]);
            }
            return self.compile_group(expr, Ctx::Value);
        }
        self.compile_value(expr, Ctx::Value)
    }

    /// A string with interpolations, or a word, as an f-string.
    fn word_value(&mut self, x: &Form, tilde: bool) -> R<Res<'py>> {
        let parts = match &x.kind {
            Kind::Word(ps) | Kind::IStr(ps) => ps.clone(),
            _ => return self.compile_value(x, Ctx::Value),
        };
        enum Slot {
            T(String),
            I(Option<char>, Option<String>),
            V,
        }
        let mut rs = vec![];
        let mut slots = vec![];
        for p in &parts {
            match p {
                Part::Text(t) => slots.push(Slot::T(t.clone())),
                Part::F(f) => match &f.kind {
                    Kind::Str { value, .. } => slots.push(Slot::T(value.clone())),
                    Kind::IStr(qs) => {
                        for q in qs {
                            match q {
                                Part::Text(t) => slots.push(Slot::T(t.clone())),
                                Part::F(ip) => {
                                    if let Kind::Interp { conv, spec, .. } = &ip.kind {
                                        rs.push(self.compile_interp(ip)?);
                                        slots.push(Slot::I(*conv, spec.clone()));
                                    }
                                }
                            }
                        }
                    }
                    Kind::Interp { conv, spec, .. } => {
                        rs.push(self.compile_interp(f)?);
                        slots.push(Slot::I(*conv, spec.clone()));
                    }
                    _ => {
                        rs.push(self.compile_value(f, Ctx::Value)?);
                        slots.push(Slot::V);
                    }
                },
            }
        }
        let (stmts, exprs) = self.gather_e(rs)?;
        let mut it = exprs.into_iter();
        let mut vals = vec![];
        for s in slots {
            vals.push(match s {
                Slot::T(t) => self.c(t)?,
                Slot::I(c, sp) => self.fmt(it.next().unwrap(), c, sp.as_deref())?,
                Slot::V => self.fmt(it.next().unwrap(), None, None)?,
            });
        }
        let mut e = self.at(self.joined(vals)?, x);
        // (/tmp/$x, ~/$name, ./$f: a path, its parts filled in.)
        if tilde && matches!(&x.kind, Kind::Word(ps) if matches!(ps.first(), Some(Part::Text(t)) if is_path_start(t))) {
            e = self.rcall("path_word", vec![e])?;
        } else if tilde && matches!(&x.kind, Kind::Word(ps) if matches!(ps.first(), Some(Part::Text(t)) if t.starts_with('~'))) {
            e = self.rcall("t", vec![e])?;
        }
        Ok(Res::s(stmts, e))
    }

    fn compile_subscript(&mut self, items: &[Form], f: &Form) -> R<Res<'py>> {
        if items.iter().any(|x| x.is_comma()) {
            let mut rs = vec![];
            for p in comma_parts(items) {
                rs.push(self.compile_subscript(p, f)?);
            }
            let (stmts, es) = self.gather_e(rs)?;
            return Ok(Res::s(stmts, self.tuple(es, false)?));
        }
        // Slices: pieces split at ':' (in words like 1:3 and $a:$b, or spaced).
        let mut toks: Vec<Option<Form>> = vec![]; // None: a ':'
        let piece = |t: &str, f: &Form| -> Form {
            if is_number(t) { Form::new(Kind::Num(t.to_string()), f.pos) } else { Form::sym_at(t, &[], f.pos) }
        };
        for x in items {
            match &x.kind {
                Kind::Sym { name, .. } if name.contains(':') => {
                    for (k, p) in name.split(':').enumerate() {
                        if k > 0 {
                            toks.push(None);
                        }
                        if !p.is_empty() {
                            toks.push(Some(piece(p, x)));
                        }
                    }
                }
                Kind::Word(ps) if ps.iter().any(|p| matches!(p, Part::Text(t) if t.contains(':'))) => {
                    let mut cur: Vec<Part> = vec![];
                    let flush = |cur: &mut Vec<Part>, toks: &mut Vec<Option<Form>>| {
                        if cur.is_empty() {
                            return;
                        }
                        let f = if cur.len() == 1 {
                            match cur.pop().unwrap() {
                                Part::F(f) => f,
                                Part::Text(t) => piece(&t, x),
                            }
                        } else {
                            Form::new(Kind::Word(std::mem::take(cur)), x.pos)
                        };
                        cur.clear();
                        toks.push(Some(f));
                    };
                    for p in ps {
                        match p {
                            Part::Text(t) if t.contains(':') => {
                                for (k, s) in t.split(':').enumerate() {
                                    if k > 0 {
                                        flush(&mut cur, &mut toks);
                                        toks.push(None);
                                    }
                                    if !s.is_empty() {
                                        cur.push(Part::Text(s.to_string()));
                                    }
                                }
                            }
                            p => cur.push(p.clone()),
                        }
                    }
                    flush(&mut cur, &mut toks);
                }
                _ => toks.push(Some(x.clone())),
            }
        }
        if !toks.iter().any(|t| t.is_none()) {
            return self.compile_expr(items, Ctx::Value, &[]);
        }
        let mut parts: Vec<Vec<Form>> = vec![vec![]];
        for t in toks {
            match t {
                None => parts.push(vec![]),
                Some(f) => parts.last_mut().unwrap().push(f),
            }
        }
        if parts.len() > 3 {
            return err("a slice has at most three parts", Some(f));
        }
        let mut rs = vec![];
        for p in &parts {
            rs.push(if p.is_empty() { Res { stmts: vec![], expr: None } } else { self.compile_expr(p, Ctx::Value, &[])? });
        }
        let (stmts, es) = self.gather(rs)?;
        let mut kw = vec![];
        for (k, e) in es.into_iter().enumerate() {
            kw.push((["lower", "upper", "step"][k], e.unwrap_or_else(|| self.none())));
        }
        Ok(Res::s(stmts, self.n("Slice", kw)?))
    }

    fn compile_group(&mut self, g: &Form, ctx: Ctx) -> R<Res<'py>> {
        let Kind::Group { open, items } = &g.kind else { unreachable!() };
        match open {
            '(' => {
                if items.is_empty() {
                    return Ok(Res::e(self.tuple(vec![], false)?));
                }
                if items.iter().any(|x| matches!(x.kind, Kind::Semi)) {
                    return self.compile_shell(items, &[], ctx);
                }
                if top_for(items) > 0 {
                    let (elt, gens) = self.comprehension(items, g, false)?;
                    return Ok(Res::e(self.n("GeneratorExp", vec![("elt", elt[0].clone()), ("generators", self.list(gens)?)])?));
                }
                if items.iter().any(|x| x.is_comma()) {
                    return self.compile_expr(items, Ctx::Value, &[]);
                }
                self.compile_items(items, &[], ctx, &[], &[])
            }
            '[' => {
                if top_for(items) > 0 {
                    let (elt, gens) = self.comprehension(items, g, false)?;
                    return Ok(Res::e(self.n("ListComp", vec![("elt", elt[0].clone()), ("generators", self.list(gens)?)])?));
                }
                let mut rs = vec![];
                if items.iter().any(|x| x.is_comma()) {
                    for p in comma_parts(items) {
                        rs.push(self.compile_expr(p, Ctx::Value, &[])?);
                    }
                } else if has_binop(items) {
                    rs.push(self.compile_expr(items, Ctx::Value, &[])?);
                } else {
                    for x in items {
                        rs.push(self.compile_value(x, Ctx::Value)?);
                    }
                }
                let (stmts, es) = self.gather_e(rs)?;
                Ok(Res::s(stmts, self.at(self.list_node(es)?, g)))
            }
            _ => self.compile_brace(items, g),
        }
    }

    fn compile_brace(&mut self, items: &[Form], g: &Form) -> R<Res<'py>> {
        if items.is_empty() {
            return Ok(Res::e(self.n("Dict", vec![("keys", self.list(vec![])?), ("values", self.list(vec![])?)])?));
        }
        if top_for(items) > 0 {
            let (elt, gens) = self.comprehension(items, g, true)?;
            if elt.len() == 2 {
                return Ok(Res::e(self.n("DictComp", vec![("key", elt[0].clone()), ("value", elt[1].clone()), ("generators", self.list(gens)?)])?));
            }
            return Ok(Res::e(self.n("SetComp", vec![("elt", elt[0].clone()), ("generators", self.list(gens)?)])?));
        }
        match self.dict_entries(items)? {
            None => {
                let mut rs = vec![];
                if items.iter().any(|x| x.is_comma()) {
                    for p in comma_parts(items) {
                        rs.push(self.compile_expr(p, Ctx::Value, &[])?);
                    }
                } else {
                    for x in items {
                        rs.push(self.compile_value(x, Ctx::Value)?);
                    }
                }
                let (stmts, es) = self.gather_e(rs)?;
                Ok(Res::s(stmts, self.n("Set", vec![("elts", self.list(es)?)])?))
            }
            Some(entries) => {
                let mut rs = vec![];
                let mut kinds = vec![];
                for (k, v) in entries {
                    match k {
                        None => {
                            let Kind::Splat { value, .. } = &v[0].kind else { unreachable!() };
                            rs.push(self.compile_value(value, Ctx::Value)?);
                            kinds.push(false);
                        }
                        Some(k) => {
                            rs.push(k);
                            rs.push(self.compile_expr(&v, Ctx::Value, &[])?);
                            kinds.push(true);
                        }
                    }
                }
                let (stmts, es) = self.gather_e(rs)?;
                let mut it = es.into_iter();
                let (mut ks, mut vs) = (vec![], vec![]);
                for has_key in kinds {
                    if has_key {
                        ks.push(it.next().unwrap());
                        vs.push(it.next().unwrap());
                    } else {
                        ks.push(self.none());
                        vs.push(it.next().unwrap());
                    }
                }
                Ok(Res::s(stmts, self.at(self.n("Dict", vec![("keys", self.list(ks)?), ("values", self.list(vs)?)])?, g)))
            }
        }
    }

    /// If x is a dict key (it ends with ':'), the key.
    fn key(&mut self, x: &Form) -> R<Option<Res<'py>>> {
        match &x.kind {
            Kind::Sym { name, .. } if name.len() > 1 && name.ends_with(':') && !name.ends_with("::") => {
                let t = &name[..name.len() - 1];
                Ok(Some(Res::e(if is_number(t) { self.num(t)? } else { self.c(t)? })))
            }
            Kind::Word(ps) => {
                if let Some(Part::Text(last)) = ps.last() {
                    if last.ends_with(':') {
                        let mut parts = ps[..ps.len() - 1].to_vec();
                        let rest = &last[..last.len() - 1];
                        if !rest.is_empty() {
                            parts.push(Part::Text(rest.to_string()));
                        }
                        if parts.len() == 1 {
                            if let Part::F(f) = &parts[0] {
                                return Ok(Some(self.compile_value(f, Ctx::Value)?));
                            }
                        }
                        return Ok(Some(self.word_value(&Form::new(Kind::Word(parts), x.pos), false)?));
                    }
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    #[allow(clippy::type_complexity)]
    fn dict_entries(&mut self, items: &[Form]) -> R<Option<Vec<(Option<Res<'py>>, Vec<Form>)>>> {
        let mut entries = vec![];
        if items.iter().any(|x| x.is_comma()) {
            for p in comma_parts(items) {
                if p.is_empty() {
                    continue;
                }
                if matches!(p[0].kind, Kind::Splat { n: 2, .. }) && p.len() == 1 {
                    entries.push((None, p.to_vec()));
                    continue;
                }
                let mut p = p.to_vec();
                let mut k = self.key(&p[0])?;
                if k.is_none() && p.len() > 2 && p[1].is_sym(":") {
                    k = Some(self.compile_value(&p[0], Ctx::Value)?);
                    p.remove(1);
                }
                let Some(k) = k else { return Ok(None) };
                if p.len() < 2 {
                    return err("a key needs a value", Some(&p[0]));
                }
                entries.push((Some(k), p[1..].to_vec()));
            }
            return Ok(Some(entries));
        }
        let mut cur_key: Option<Res<'py>> = None;
        let mut cur: Vec<Form> = vec![];
        let mut seen = false;
        let mut i = 0;
        while i < items.len() {
            let x = &items[i];
            let mut k = self.key(x)?;
            if k.is_none() && i + 1 < items.len() && items[i + 1].is_sym(":") {
                k = Some(self.compile_value(x, Ctx::Value)?);
                i += 1;
            }
            let dstar = matches!(x.kind, Kind::Splat { n: 2, .. });
            if k.is_some() || dstar {
                if seen {
                    if cur.is_empty() {
                        return err("a key needs a value", Some(x));
                    }
                    entries.push((cur_key.take(), std::mem::take(&mut cur)));
                }
                if dstar {
                    entries.push((None, vec![x.clone()]));
                    seen = false;
                } else {
                    seen = true;
                    cur_key = k;
                }
            } else {
                if !seen {
                    if entries.is_empty() {
                        return Ok(None);
                    }
                    return err("expected a key (key: value)", Some(x));
                }
                cur.push(x.clone());
            }
            i += 1;
        }
        if seen {
            if cur.is_empty() {
                return err("a key needs a value", items.last());
            }
            entries.push((cur_key, cur));
        }
        Ok(Some(entries))
    }

    fn comprehension(&mut self, items: &[Form], g: &Form, dict_ok: bool) -> R<(Vec<Obj<'py>>, Vec<Obj<'py>>)> {
        let k = top_for(items);
        let elt_items = &items[..k];
        let mut clauses: Vec<(&str, Vec<Form>)> = vec![];
        for x in &items[k..] {
            if x.is_sym("for") || x.is_sym("if") {
                clauses.push((if x.is_sym("for") { "for" } else { "if" }, vec![]));
            } else if x.is_sym("async") {
                continue;
            } else if let Some(c) = clauses.last_mut() {
                c.1.push(x.clone());
            }
        }
        // (Its variables, read bare in its element, its ifs or a later for's
        // iterable.)
        {
            let mut names: Vec<String> = vec![];
            for (kind, body) in &clauses {
                if *kind == "for" {
                    if let Some(ins) = body.iter().position(|y| y.is_sym("in")) {
                        bare_iterable(&body[ins + 1..])?;
                        if let Some(f) = bare_read(&body[ins + 1..], &names, false) {
                            return bare_loop_var(f);
                        }
                        names.extend(bound_names(&body[..ins]));
                    }
                } else if let Some(f) = bare_read(body, &names, false) {
                    return bare_loop_var(f);
                }
            }
            if let Some(f) = bare_read(elt_items, &names, false) {
                return bare_loop_var(f);
            }
        }
        self.push("comp");
        let result = (|| -> R<(Vec<Obj<'py>>, Vec<Obj<'py>>)> {
            let mut gens: Vec<Obj<'py>> = vec![];
            for (kind, body) in clauses {
                if kind == "for" {
                    let Some(ins) = body.iter().position(|y| y.is_sym("in")) else { return err("for needs in", Some(g)) };
                    let it = self.compile_expr(&body[ins + 1..], Ctx::Value, &[])?;
                    if !it.stmts.is_empty() {
                        return err("this can't go in a comprehension", Some(g));
                    }
                    let target = self.make_target(&body[..ins], g, false, false)?;
                    gens.push(self.n("comprehension", vec![("target", self.val(target.expr)?), ("iter", self.val(it.expr)?), ("ifs", self.list(vec![])?), ("is_async", PyInt::new(self.py, 0).into_any())])?);
                } else {
                    let c = self.compile_expr(&body, Ctx::Value, &[])?;
                    let Some(last) = gens.last() else { return err("if before for", Some(g)) };
                    last.getattr("ifs")?.call_method1("append", (self.val(c.expr)?,))?;
                }
            }
            if dict_ok && !elt_items.is_empty() {
                if let Some(kr) = self.key(&elt_items[0])? {
                    let vr = self.compile_expr(&elt_items[1..], Ctx::Value, &[])?;
                    return Ok((vec![self.val(kr.expr)?, self.val(vr.expr)?], gens));
                }
                if elt_items.len() > 2 && elt_items[1].is_sym(":") {
                    let kr = self.compile_value(&elt_items[0], Ctx::Value)?;
                    let vr = self.compile_expr(&elt_items[2..], Ctx::Value, &[])?;
                    return Ok((vec![self.val(kr.expr)?, self.val(vr.expr)?], gens));
                }
            }
            let e = self.compile_expr(elt_items, Ctx::Value, &[])?;
            if !e.stmts.is_empty() {
                return err("this can't go in a comprehension", Some(g));
            }
            Ok((vec![self.val(e.expr)?], gens))
        })();
        self.pop();
        result
    }

    // ---- assignment -----------------------------------------------------------

    fn make_target(&mut self, items: &[Form], f: &Form, del: bool, _x: bool) -> R<Res<'py>> {
        let ctx_name = if del { "Del" } else { "Store" };
        if items.iter().any(|x| x.is_comma()) {
            let mut rs = vec![];
            for p in comma_parts(items) {
                rs.push(self.make_target(p, f, del, false)?);
            }
            let (stmts, es) = self.gather_e(rs)?;
            return Ok(Res::s(stmts, self.n("Tuple", vec![("elts", self.list(es)?), ("ctx", self.n0(ctx_name)?)])?));
        }
        if items.len() != 1 {
            if !items.is_empty() && items.iter().all(|x| matches!(x.kind, Kind::Sym { .. } | Kind::Var(_) | Kind::Group { .. } | Kind::Splat { .. } | Kind::Attr { .. } | Kind::Index { .. })) {
                let mut rs = vec![];
                for x in items {
                    rs.push(self.make_target(std::slice::from_ref(x), f, del, false)?);
                }
                let (stmts, es) = self.gather_e(rs)?;
                return Ok(Res::s(stmts, self.n("Tuple", vec![("elts", self.list(es)?), ("ctx", self.n0(ctx_name)?)])?));
            }
            return err("can't assign to this", items.first().or(Some(f)));
        }
        let mut x = &items[0];
        if let Kind::Var(s) = &x.kind {
            if s.sym_name().is_some() {
                x = s;
            }
        }
        match &x.kind {
            Kind::Sym { name, marks } => {
                if name.starts_with('*') && is_ident(name.trim_start_matches('*')) {
                    let inner = self.make_target(&[Form::sym_at(name.trim_start_matches('*'), marks, x.pos)], f, del, false)?;
                    let e = self.n("Starred", vec![("value", self.val(inner.expr)?), ("ctx", self.n0(ctx_name)?)])?;
                    return Ok(Res::s(inner.stmts, e));
                }
                if name.contains('.') && is_dotted(name) {
                    let (root, attr) = name.rsplit_once('.').unwrap();
                    let base = self.name_load(&Form::sym_at(root, marks, x.pos))?;
                    return Ok(Res::e(self.n("Attribute", vec![("value", base), ("attr", self.s(attr)), ("ctx", self.n0(ctx_name)?)])?));
                }
                let py = if del { self.binding(name, marks).unwrap_or(name.clone()) } else { self.bind(x)? };
                Ok(Res::e(self.at(self.n("Name", vec![("id", self.s(&py)), ("ctx", self.n0(ctx_name)?)])?, x)))
            }
            Kind::Attr { obj, name } => {
                let r = self.compile_value(obj, Ctx::Value)?;
                let e = self.n("Attribute", vec![("value", self.val(r.expr)?), ("attr", self.s(name)), ("ctx", self.n0(ctx_name)?)])?;
                Ok(Res::s(r.stmts, e))
            }
            Kind::Index { obj, items } => {
                let r = self.compile_value(obj, Ctx::Value)?;
                let s = self.compile_subscript(items, x)?;
                let (stmts, es) = self.gather_e(vec![r, s])?;
                Ok(Res::s(stmts, self.n("Subscript", vec![("value", es[0].clone()), ("slice", es[1].clone()), ("ctx", self.n0(ctx_name)?)])?))
            }
            Kind::Group { open: '[' | '(', items: sub } => {
                if sub.is_empty() {
                    return err("nothing to assign to", Some(x));
                }
                let r = self.make_target(sub, f, del, false)?;
                let e = self.val(r.expr)?;
                let is_tuple = e.get_type().name().map(|n| n.to_string() == "Tuple").unwrap_or(false);
                let e = if is_tuple { e } else { self.n("Tuple", vec![("elts", self.list(vec![e])?), ("ctx", self.n0(ctx_name)?)])? };
                Ok(Res::s(r.stmts, e))
            }
            Kind::Splat { value, .. } => {
                let inner = self.make_target(std::slice::from_ref(value), f, del, false)?;
                let e = self.n("Starred", vec![("value", self.val(inner.expr)?), ("ctx", self.n0(ctx_name)?)])?;
                Ok(Res::s(inner.stmts, e))
            }
            _ => err("can't assign to this", Some(x)),
        }
    }

    fn compile_annassign(&mut self, target: &Form, rest: &[Form], children: &[Form]) -> R<Res<'py>> {
        let eq = rest.iter().position(|x| x.is_op("="));
        let ann = &rest[..eq.unwrap_or(rest.len())];
        if ann.is_empty() {
            return err("a type goes after the ':'", Some(target));
        }
        let text = ann.iter().map(show).collect::<Vec<_>>().join(" ");
        let annotation = self.annotation(&Some(text))?;
        let mut stmts = vec![];
        let value = match eq {
            Some(e) => {
                let v = if rest.len() > e + 1 { self.compile_expr(&rest[e + 1..], Ctx::Value, children)? } else { self.compile_body(children, Ctx::Value)? };
                stmts.extend(v.stmts);
                Some(self.val(v.expr)?)
            }
            None => None,
        };
        let py = self.bind(target)?;
        let has = value.is_some();
        let node = self.n("AnnAssign", vec![("target", self.name(&py, true)?), ("annotation", annotation), ("value", value.unwrap_or_else(|| self.none())), ("simple", PyInt::new(self.py, 1).into_any())])?;
        stmts.push(self.at(node, target));
        Ok(Res::s(stmts, if has { self.name(&py, false)? } else { self.cnone()? }))
    }

    /// `a=1`, `a=1 b=2`: each as `a = 1`. `NAME=value … program …`: the
    /// program run with those environment variables (words, as in a
    /// command), only while it runs, as bash does.
    fn compile_bare_assign(&mut self, items: &[Form], children: &[Form], ctx: Ctx, conts: &[Form]) -> R<Res<'py>> {
        let k = items.iter().take_while(|x| matches!(x.kind, Kind::KwArg { .. })).count();
        if k == items.len() {
            let mut stmts = vec![];
            for (j, x) in items.iter().enumerate() {
                let Kind::KwArg { name, value } = &x.kind else { unreachable!() };
                let target = Form::sym_at(name, &[], x.pos);
                let eq = Form::new(Kind::Op { name: "=".into(), tick: false }, x.pos);
                let last = j + 1 == items.len();
                let r = self.compile_assign(&[target], &eq, &[(**value).clone()], if last { children } else { &[] }, if last { conts } else { &[] })?;
                stmts.extend(self.as_stmts(r)?);
            }
            return Ok(Res::s(stmts, self.cnone()?));
        }
        let mut stmts = vec![];
        let mut kws = vec![];
        for x in &items[..k] {
            let Kind::KwArg { name, value } = &x.kind else { unreachable!() };
            let v = self.cmd_word(value)?;
            stmts.extend(v.stmts);
            kws.push(self.keyword(Some(name), self.val(v.expr)?)?);
        }
        let ctx_expr = self.call(self.rt("env_with")?, vec![], kws)?;
        let item = self.n("withitem", vec![("context_expr", ctx_expr)])?;
        let t = if ctx == Ctx::Stmt { None } else { Some(self.tmp("w")) };
        let line = Form::line(items[k..].to_vec(), children.to_vec(), items[k].pos);
        let body = self.branch(&[line], ctx, &t)?;
        if let Some(t) = &t {
            stmts.push(self.assign(t, self.cnone()?)?);
        }
        let node = self.n("With", vec![("items", self.list(vec![item])?), ("body", self.list(body)?)])?;
        stmts.push(self.at(node, &items[0]));
        Ok(Res::s(stmts, match t {
            Some(t) => self.name(&t, false)?,
            None => self.cnone()?,
        }))
    }

    fn compile_assign(&mut self, lhs: &[Form], op: &Form, rhs: &[Form], children: &[Form], conts: &[Form]) -> R<Res<'py>> {
        let opn = op.op_name().unwrap_or("=");
        if rhs.is_empty() && children.is_empty() {
            return err(format!("nothing after '{opn}'"), Some(op));
        }
        let value = if !conts.is_empty() {
            self.compile_items(rhs, children, Ctx::Value, conts, &[])?
        } else if rhs.is_empty() {
            self.compile_body(children, Ctx::Value)?
        } else {
            self.compile_expr(rhs, Ctx::Value, children)?
        };
        if let (true, [g]) = (opn == "=", lhs) {
            if let Kind::Group { open: '{', items: ks } = &g.kind {
                return self.destructure(ks, g, value);
            }
        }
        let target = self.make_target(lhs, op, false, false)?;
        let mut stmts = value.stmts;
        stmts.extend(target.stmts);
        let t = self.val(target.expr)?;
        let tname = t.get_type().name().map(|n| n.to_string()).unwrap_or_default();
        let node = if opn == "=" {
            self.n("Assign", vec![("targets", self.list(vec![t.clone()])?), ("value", self.val(value.expr)?)])?
        } else {
            if tname == "Tuple" || tname == "Starred" {
                return err(format!("'{opn}' takes one target"), Some(op));
            }
            self.n("AugAssign", vec![("target", t.clone()), ("op", self.n0(assign_op(opn).unwrap())?), ("value", self.val(value.expr)?)])?
        };
        stmts.push(self.at(node, op));
        let result = if tname == "Name" { self.name(&t.getattr("id")?.extract::<String>()?, false)? } else { self.cnone()? };
        Ok(Res::s(stmts, result))
    }

    // ---- special forms --------------------------------------------------------

    fn special(&mut self, n: &str, items: &[Form], children: &[Form], ctx: Ctx, conts: &[Form], decos: &[Form]) -> R<Res<'py>> {
        let head = &items[0];
        if CONTINUATIONS.contains(&n) {
            return err(format!("'{n}' without a matching if, for, while or try above it"), Some(head));
        }
        if n == "case" {
            return err("'case' goes inside a match", Some(head));
        }
        if !decos.is_empty() && !["def", "class", "async"].contains(&n) {
            return err("a decorator must come just before def or class", Some(&decos[0]));
        }
        if !conts.is_empty() && !["if", "for", "while", "try", "async"].contains(&n) {
            return err(format!("'{}' can't follow {n}", show(&line_parts(&conts[0]).0[0])), Some(&conts[0]));
        }
        match n {
            "pass" => Ok(Res::s(vec![self.pass()?], self.cnone()?)),
            "break" | "continue" => Ok(Res::s(vec![self.at(self.n0(if n == "break" { "Break" } else { "Continue" })?, head)], self.cnone()?)),
            "return" => {
                let v = if items.len() > 1 { Some(self.compile_expr(&items[1..], Ctx::Value, children)?) } else if !children.is_empty() { Some(self.compile_body(children, Ctx::Value)?) } else { None };
                let (mut stmts, val) = match v {
                    Some(r) => (r.stmts, r.expr.unwrap_or_else(|| self.none())),
                    None => (vec![], self.none()),
                };
                stmts.push(self.at(self.n("Return", vec![("value", val)])?, head));
                Ok(Res::s(stmts, self.cnone()?))
            }
            "yield" => {
                self.scope().is_gen = true;
                if items.len() > 1 && items[1].is_sym("from") {
                    let v = self.compile_expr(&items[2..], Ctx::Value, children)?;
                    let e = self.n("YieldFrom", vec![("value", self.val(v.expr)?)])?;
                    return Ok(Res::s(v.stmts, self.at(e, head)));
                }
                if items.len() == 1 {
                    return Ok(Res::e(self.at(self.n("Yield", vec![])?, head)));
                }
                let v = self.compile_expr(&items[1..], Ctx::Value, children)?;
                let e = self.n("Yield", vec![("value", self.val(v.expr)?)])?;
                Ok(Res::s(v.stmts, self.at(e, head)))
            }
            "await" => {
                let v = self.compile_expr(&items[1..], Ctx::Value, children)?;
                let e = self.n("Await", vec![("value", self.val(v.expr)?)])?;
                Ok(Res::s(v.stmts, self.at(e, head)))
            }
            "raise" => self.sf_raise(items),
            "assert" => self.sf_assert(items),
            "del" => {
                let mut rs = vec![];
                for x in &items[1..] {
                    if !x.is_comma() {
                        rs.push(self.make_target(std::slice::from_ref(x), head, true, false)?);
                    }
                }
                let (mut stmts, es) = self.gather_e(rs)?;
                stmts.push(self.at(self.n("Delete", vec![("targets", self.list(es)?)])?, head));
                Ok(Res::s(stmts, self.cnone()?))
            }
            "global" | "nonlocal" => {
                let names: Vec<String> = items[1..].iter().filter_map(|x| x.sym_name().map(String::from)).collect();
                for nm in &names {
                    self.scope().names.insert((nm.clone(), vec![]), nm.clone());
                }
                let l = self.list(names.iter().map(|s| self.s(s)).collect())?;
                Ok(Res::s(vec![self.n(if n == "global" { "Global" } else { "Nonlocal" }, vec![("names", l)])?], self.cnone()?))
            }
            "import" => self.sf_import(items),
            "from" => self.sf_from(items, children),
            "do" => {
                let mut lines = vec![];
                if items.len() > 1 {
                    lines.push(Form::line(items[1..].to_vec(), vec![], items[1].pos));
                }
                lines.extend(children.iter().cloned());
                self.compile_body(&lines, ctx)
            }
            "if" => self.sf_if(items, children, ctx, conts),
            "while" => self.sf_while(items, children, conts),
            "for" => self.sf_for(items, children, conts, false),
            "with" => self.sf_with(items, children, ctx, false),
            "try" => self.sf_try(items, children, ctx, conts),
            "async" => {
                let next = items.get(1).and_then(|x| x.sym_name()).unwrap_or("");
                match next {
                    "def" => self.sf_def(&items[1..], children, decos, true),
                    "for" => self.sf_for(&items[1..], children, conts, true),
                    "with" => self.sf_with(&items[1..], children, ctx, true),
                    _ => err("async def, async for or async with", Some(head)),
                }
            }
            "def" => self.sf_def(items, children, decos, false),
            "fn" | "lambda" => self.sf_fn(items, children),
            "class" => self.sf_class(items, children, decos),
            "match" => self.sf_match(items, children, ctx),
            "quote" | "quasi" => {
                let quasi = n == "quasi";
                if items.len() == 1 {
                    if children.is_empty() {
                        return err(format!("{n} needs a template"), Some(head));
                    }
                    if children.len() == 1 {
                        return self.quote(&children[0], quasi);
                    }
                    let mut rs = vec![];
                    for c in children {
                        rs.push(self.quote(c, quasi)?);
                    }
                    let (stmts, es) = self.gather_e(rs)?;
                    return Ok(Res::s(stmts, self.list_node(es)?));
                }
                if items.len() == 2 && children.is_empty() {
                    let f = if quasi { group_form(&items[1]) } else { items[1].clone() };
                    return self.quote(&f, quasi);
                }
                self.quote(&Form::line(items[1..].to_vec(), children.to_vec(), items[1].pos), quasi)
            }
            "defmacro" => self.sf_defmacro(items, children),
            "defop" => self.sf_defop(items),
            "->" | "->>" | "some->" | "some->>" | "as->" | "doto" => self.sf_thread(n, items, children, ctx),
            "py" => self.sf_py(items),
            "let" => self.sf_let(items, children, ctx),
            "require" => self.sf_require(items),
            _ => err(format!("'{n}' isn't handled"), Some(head)),
        }
    }

    /// Where Python expects a class or name: a bareword is the name.
    fn name_slot(&mut self, items: &[Form]) -> R<Res<'py>> {
        if items.len() == 1 {
            if let Some(n) = items[0].sym_name() {
                if is_dotted(n) && !["True", "False", "None"].contains(&n) {
                    return Ok(Res::e(self.name_load(&items[0])?));
                }
            }
            if let Kind::Group { open: '(', items: sub } = &items[0].kind {
                if sub.iter().any(|x| x.is_comma()) {
                    let mut rs = vec![];
                    for p in comma_parts(sub) {
                        rs.push(self.name_slot(p)?);
                    }
                    let (stmts, es) = self.gather_e(rs)?;
                    return Ok(Res::s(stmts, self.tuple(es, false)?));
                }
            }
        }
        if items.len() > 1 && items[0].sym_name().is_some_and(is_dotted) {
            let f = Res::e(self.name_load(&items[0])?);
            let (astmts, args, kws, _) = self.compile_call_args(&items[1..], &[])?;
            let (stmts, es) = self.gather_e(vec![f, Res { stmts: astmts, expr: None }])?;
            return Ok(Res::s(stmts, self.call(es[0].clone(), args, kws)?));
        }
        self.compile_expr(items, Ctx::Value, &[])
    }

    fn sf_raise(&mut self, items: &[Form]) -> R<Res<'py>> {
        let head = &items[0];
        if items.len() == 1 {
            return Ok(Res::s(vec![self.at(self.n("Raise", vec![])?, head)], self.cnone()?));
        }
        let rest = &items[1..];
        let (exc_items, cause_items) = match rest.iter().position(|x| x.is_sym("from")) {
            Some(k) => (&rest[..k], Some(&rest[k + 1..])),
            None => (rest, None),
        };
        let e = self.name_slot(exc_items)?;
        let mut stmts = e.stmts;
        let mut kw = vec![("exc", self.val(e.expr)?)];
        if let Some(c) = cause_items {
            let c = self.name_slot(c)?;
            stmts.extend(c.stmts);
            kw.push(("cause", self.val(c.expr)?));
        }
        stmts.push(self.at(self.n("Raise", kw)?, head));
        Ok(Res::s(stmts, self.cnone()?))
    }

    fn sf_assert(&mut self, items: &[Form]) -> R<Res<'py>> {
        let rest = &items[1..];
        let (cond_items, msg) = if rest.iter().any(|x| x.is_comma()) {
            let parts = split(rest, |x| x.is_comma());
            (parts[0], Some(self.compile_expr(parts[1], Ctx::Value, &[])?))
        } else if rest.len() == 2 && !has_binop(rest) {
            (&rest[..1], Some(self.compile_value(&rest[1], Ctx::Value)?))
        } else {
            (rest, None)
        };
        let c = self.compile_expr(cond_items, Ctx::Value, &[])?;
        let mut stmts = c.stmts;
        let mut kw = vec![("test", self.val(c.expr)?)];
        if let Some(m) = msg {
            stmts.extend(m.stmts);
            kw.push(("msg", self.val(m.expr)?));
        }
        stmts.push(self.at(self.n("Assert", kw)?, &items[0]));
        Ok(Res::s(stmts, self.cnone()?))
    }

    fn alias(&self, name: &str, asname: Option<&str>) -> R<Obj<'py>> {
        self.n("alias", vec![("name", self.s(name)), ("asname", asname.map(|a| self.s(a)).unwrap_or_else(|| self.none()))])
    }

    fn sf_import(&mut self, items: &[Form]) -> R<Res<'py>> {
        let mut out = vec![];
        let rest = &items[1..];
        let mut k = 0;
        while k < rest.len() {
            let x = &rest[k];
            let Some(n) = x.sym_name().filter(|n| is_dotted(n)) else { return err("import takes module names", Some(x)) };
            if k + 1 < rest.len() && rest[k + 1].is_sym("as") {
                let Some(a) = rest.get(k + 2) else { return err("as needs a name", Some(&rest[k + 1])) };
                let py = self.bind(a)?;
                out.push(self.alias(n, Some(&py))?);
                k += 3;
            } else {
                let root = n.split('.').next().unwrap();
                let py = self.bind(&Form::sym_at(root, x.marks(), x.pos))?;
                if py != root {
                    return err("a macro can't import a dotted name without as", Some(x));
                }
                out.push(self.alias(n, None)?);
                k += 1;
            }
        }
        if out.is_empty() {
            return err("import what?", Some(&items[0]));
        }
        Ok(Res::s(vec![self.at(self.n("Import", vec![("names", self.list(out)?)])?, &items[0])], self.cnone()?))
    }

    fn sf_from(&mut self, items: &[Form], children: &[Form]) -> R<Res<'py>> {
        if items.len() < 4 || !items[2].is_sym("import") {
            return err("from module import names…", Some(&items[0]));
        }
        let mtext = show(&items[1]);
        let level = mtext.len() - mtext.trim_start_matches('.').len();
        let module = mtext.trim_start_matches('.');
        let mut rest: Vec<Form> = items[3..].to_vec();
        for c in children {
            rest.extend(line_parts(c).0.iter().cloned());
        }
        let mut names = vec![];
        let mut k = 0;
        while k < rest.len() {
            let x = rest[k].clone();
            if x.is_comma() {
                k += 1;
                continue;
            }
            if let Kind::Group { open: '(', items: sub } = &x.kind {
                rest.splice(k..k + 1, sub.iter().cloned());
                continue;
            }
            if x.is_op("*") {
                names.push(self.alias("*", None)?);
                k += 1;
                continue;
            }
            let Some(n) = x.sym_name() else { return err("import takes names", Some(&x)) };
            if k + 2 < rest.len() + 1 && rest.get(k + 1).is_some_and(|y| y.is_sym("as")) {
                let py = self.bind(&rest[k + 2])?;
                names.push(self.alias(n, Some(&py))?);
                k += 3;
            } else {
                let py = self.bind(&x)?;
                names.push(self.alias(n, if py == n { None } else { Some(&py) })?);
                k += 1;
            }
        }
        let m = if module.is_empty() { self.none() } else { self.s(module) };
        let node = self.n("ImportFrom", vec![("module", m), ("names", self.list(names)?), ("level", self.c(level)?.getattr("value")?)])?;
        Ok(Res::s(vec![self.at(node, &items[0])], self.cnone()?))
    }

    fn branch_ctx(ctx: Ctx) -> Ctx {
        match ctx {
            Ctx::Stmt => Ctx::Stmt,
            Ctx::Tail => Ctx::Tail,
            _ => Ctx::Value,
        }
    }

    fn cond(&mut self, items: &[Form], f: &Form) -> R<Res<'py>> {
        if items.is_empty() {
            return err("a condition is missing", Some(f));
        }
        self.compile_expr(items, Ctx::Cond, &[])
    }

    /// A branch's statements, assigning its value to t when there's a t.
    fn branch(&mut self, lines: &[Form], ctx: Ctx, t: &Option<String>) -> R<Vec<Obj<'py>>> {
        let r = self.compile_body(lines, Self::branch_ctx(ctx))?;
        match t {
            None => {
                let s = self.as_stmts(r)?;
                Ok(if s.is_empty() { vec![self.pass()?] } else { s })
            }
            Some(t) => {
                let mut s = r.stmts;
                s.push(self.assign(t, self.val(r.expr)?)?);
                Ok(s)
            }
        }
    }

    fn sf_if(&mut self, items: &[Form], children: &[Form], ctx: Ctx, conts: &[Form]) -> R<Res<'py>> {
        let head = &items[0];
        if let Some(k) = items.iter().position(|x| x.is_sym("then")) {
            if children.is_empty() {
                let els = items.iter().enumerate().skip(k).find(|(_, x)| x.is_sym("else")).map(|(i, _)| i);
                let c = self.cond(&items[1..k], head)?;
                let a = self.compile_expr(&items[k + 1..els.unwrap_or(items.len())], Self::branch_ctx(ctx), &[])?;
                let b = match els {
                    Some(e) => self.compile_expr(&items[e + 1..], Self::branch_ctx(ctx), &[])?,
                    None => Res::e(self.cnone()?),
                };
                if a.stmts.is_empty() && b.stmts.is_empty() {
                    let e = self.n("IfExp", vec![("test", self.val(c.expr)?), ("body", self.val(a.expr)?), ("orelse", self.val(b.expr)?)])?;
                    return Ok(Res::s(c.stmts, e));
                }
                let t = self.tmp("if");
                let mut body = a.stmts;
                body.push(self.assign(&t, self.val(a.expr)?)?);
                let mut orelse = b.stmts;
                orelse.push(self.assign(&t, self.val(b.expr)?)?);
                let mut stmts = c.stmts;
                stmts.push(self.iff(self.val(c.expr)?, body, orelse)?);
                return Ok(Res::s(stmts, self.name(&t, false)?));
            }
        }
        let mut branches: Vec<(Vec<Form>, Vec<Form>, Form)> = vec![(items[1..].to_vec(), children.to_vec(), head.clone())];
        let mut else_body: Option<Vec<Form>> = None;
        for c in conts {
            let (ci, cc) = line_parts(c);
            if ci[0].is_sym("elif") {
                branches.push((ci[1..].to_vec(), cc.to_vec(), ci[0].clone()));
            } else {
                else_body = Some(if ci.len() == 1 { cc.to_vec() } else { vec![Form::line(ci[1..].to_vec(), cc.to_vec(), c.pos)] });
            }
        }
        let t = if ctx == Ctx::Stmt { None } else { Some(self.tmp("if")) };
        let mut orelse = match &else_body {
            Some(b) => self.branch(b, ctx, &t)?,
            None => match &t {
                Some(t) => vec![self.assign(t, self.cnone()?)?],
                None => vec![],
            },
        };
        let mut pre = vec![];
        let nb = branches.len();
        for (k, (cond_items, lines, h)) in branches.into_iter().enumerate().rev() {
            let c = self.cond(&cond_items, &h)?;
            let body = self.branch(&lines, ctx, &t)?;
            let node = self.at(self.iff(self.val(c.expr)?, body, orelse)?, &h);
            if k == 0 {
                pre = c.stmts;
                orelse = vec![node];
            } else {
                let mut o = c.stmts;
                o.push(node);
                orelse = o;
            }
            let _ = nb;
        }
        pre.extend(orelse);
        Ok(Res::s(pre, match t {
            Some(t) => self.name(&t, false)?,
            None => self.cnone()?,
        }))
    }

    fn sf_while(&mut self, items: &[Form], children: &[Form], conts: &[Form]) -> R<Res<'py>> {
        let c = self.cond(&items[1..], &items[0])?;
        let body = self.branch(children, Ctx::Stmt, &None)?;
        let mut orelse = vec![];
        for k in conts {
            orelse = self.branch(line_parts(k).1, Ctx::Stmt, &None)?;
        }
        let node = if !c.stmts.is_empty() {
            if !orelse.is_empty() {
                return err("while … else needs a simple condition", Some(&items[0]));
            }
            let mut b = c.stmts;
            b.push(self.iff(self.unary("not", self.val(c.expr)?)?, vec![self.n0("Break")?], vec![])?);
            b.extend(body);
            self.n("While", vec![("test", self.c(true)?), ("body", self.list(b)?), ("orelse", self.list(vec![])?)])?
        } else {
            self.n("While", vec![("test", self.val(c.expr)?), ("body", self.list(body)?), ("orelse", self.list(orelse)?)])?
        };
        Ok(Res::s(vec![self.at(node, &items[0])], self.cnone()?))
    }

    fn sf_for(&mut self, items: &[Form], children: &[Form], conts: &[Form], is_async: bool) -> R<Res<'py>> {
        let Some(k) = items.iter().position(|x| x.is_sym("in")) else { return err("for name in iterable", Some(&items[0])) };
        bare_iterable(&items[k + 1..])?;
        if let Some(f) = children.iter().find_map(|c| bare_in(c, &bound_names(&items[1..k]))) {
            return bare_loop_var(f);
        }
        let it = self.compile_expr(&items[k + 1..], Ctx::Value, &[])?;
        let target = self.make_target(&items[1..k], &items[0], false, false)?;
        let body = self.branch(children, Ctx::Stmt, &None)?;
        let mut orelse = vec![];
        for c in conts {
            orelse = self.branch(line_parts(c).1, Ctx::Stmt, &None)?;
        }
        let node = self.n(if is_async { "AsyncFor" } else { "For" }, vec![("target", self.val(target.expr)?), ("iter", self.val(it.expr)?), ("body", self.list(body)?), ("orelse", self.list(orelse)?)])?;
        let mut stmts = it.stmts;
        stmts.extend(target.stmts);
        stmts.push(self.at(node, &items[0]));
        Ok(Res::s(stmts, self.cnone()?))
    }

    fn sf_with(&mut self, items: &[Form], children: &[Form], ctx: Ctx, is_async: bool) -> R<Res<'py>> {
        let mut withitems = vec![];
        let mut stmts = vec![];
        for p in comma_parts(&items[1..]) {
            let a = p.iter().position(|x| x.is_sym("as"));
            let e = self.compile_expr(&p[..a.unwrap_or(p.len())], Ctx::Value, &[])?;
            stmts.extend(e.stmts);
            let mut kw = vec![("context_expr", self.val(e.expr)?)];
            if let Some(a) = a {
                let tr = self.make_target(&p[a + 1..], &items[0], false, false)?;
                stmts.extend(tr.stmts);
                kw.push(("optional_vars", self.val(tr.expr)?));
            }
            withitems.push(self.n("withitem", kw)?);
        }
        let t = if ctx == Ctx::Stmt { None } else { Some(self.tmp("w")) };
        let body = self.branch(children, ctx, &t)?;
        if let Some(t) = &t {
            stmts.push(self.assign(t, self.cnone()?)?);
        }
        let node = self.n(if is_async { "AsyncWith" } else { "With" }, vec![("items", self.list(withitems)?), ("body", self.list(body)?)])?;
        stmts.push(self.at(node, &items[0]));
        Ok(Res::s(stmts, match t {
            Some(t) => self.name(&t, false)?,
            None => self.cnone()?,
        }))
    }

    fn sf_try(&mut self, items: &[Form], children: &[Form], ctx: Ctx, conts: &[Form]) -> R<Res<'py>> {
        let t = if ctx == Ctx::Stmt { None } else { Some(self.tmp("try")) };
        let main = self.branch(children, ctx, &t)?;
        let (mut handlers, mut orelse, mut fin) = (vec![], vec![], vec![]);
        let mut star = false;
        for c in conts {
            let (ci, cc) = line_parts(c);
            let cn = ci[0].sym_name().unwrap_or("");
            if cn == "except" || cn == "except*" {
                star |= cn == "except*";
                let rest = &ci[1..];
                let a = rest.iter().position(|x| x.is_sym("as"));
                let typ_items = &rest[..a.unwrap_or(rest.len())];
                let typ = if typ_items.is_empty() { self.none() } else { let r = self.name_slot(typ_items)?; self.val(r.expr)? };
                let nm = match a {
                    Some(a) => match rest.get(a + 1) {
                        Some(x) => { let b = self.bind(x)?; self.s(&b) }
                        None => return err("as needs a name", Some(&rest[a])),
                    },
                    None => self.none(),
                };
                let body = self.branch(cc, ctx, &t)?;
                handlers.push(self.at(self.n("ExceptHandler", vec![("type", typ), ("name", nm), ("body", self.list(body)?)])?, &ci[0]));
            } else if cn == "else" {
                orelse = self.branch(cc, ctx, &t)?;
            } else if cn == "finally" {
                fin = self.branch(cc, Ctx::Stmt, &None)?;
            }
        }
        if handlers.is_empty() && fin.is_empty() {
            return err("try needs except or finally", Some(&items[0]));
        }
        let mut stmts = vec![];
        if let Some(t) = &t {
            stmts.push(self.assign(t, self.cnone()?)?);
        }
        let node = self.n(if star { "TryStar" } else { "Try" }, vec![("body", self.list(main)?), ("handlers", self.list(handlers)?), ("orelse", self.list(orelse)?), ("finalbody", self.list(fin)?)])?;
        stmts.push(self.at(node, &items[0]));
        Ok(Res::s(stmts, match t {
            Some(t) => self.name(&t, false)?,
            None => self.cnone()?,
        }))
    }

    // ---- functions and classes -------------------------------------------------

    /// [(kind, name sym, default, annotation)]: pos, posonly, vararg, kwonly, kwarg, star.
    #[allow(clippy::type_complexity)]
    fn parse_params(&mut self, items: &[Form]) -> R<Vec<(&'static str, Option<Form>, Option<Res<'py>>, Option<String>)>> {
        let mut out: Vec<(&'static str, Option<Form>, Option<Res<'py>>, Option<String>)> = vec![];
        let mut star = false;
        for x in items {
            match &x.kind {
                Kind::Op { name, tick: false } if name == "*" => {
                    star = true;
                    out.push(("star", None, None, None));
                }
                Kind::Op { name, tick: false } if name == "/" => {
                    for p in out.iter_mut() {
                        if p.0 == "pos" {
                            p.0 = "posonly";
                        }
                    }
                }
                Kind::Sym { name, marks } => {
                    let mut n = name.as_str();
                    let mut ann = None;
                    if !n.ends_with(':') {
                        if let Some((a, b)) = n.trim_start_matches('*').split_once(':') {
                            let stars = n.len() - n.trim_start_matches('*').len();
                            ann = Some(b.to_string());
                            n = &name[..stars + a.len()];
                        }
                    }
                    if let Some(r) = n.strip_prefix("**") {
                        out.push(("kwarg", Some(Form::sym_at(r, marks, x.pos)), None, ann));
                    } else if let Some(r) = n.strip_prefix('*') {
                        star = true;
                        out.push(("vararg", Some(Form::sym_at(r, marks, x.pos)), None, ann));
                    } else {
                        out.push((if star { "kwonly" } else { "pos" }, Some(Form::sym_at(n, marks, x.pos)), None, ann));
                    }
                }
                Kind::KwArg { name, value } => {
                    let d = self.compile_value(value, Ctx::Value)?;
                    out.push((if star { "kwonly" } else { "pos" }, Some(Form::sym_at(name, &[], x.pos)), Some(d), None));
                }
                Kind::Splat { value, n } => {
                    let Kind::Var(s) = &value.kind else { return err(format!("'{}' isn't a parameter", show(x)), Some(x)) };
                    if *n == 1 {
                        star = true;
                    }
                    out.push((if *n == 1 { "vararg" } else { "kwarg" }, Some((**s).clone()), None, None));
                }
                Kind::Var(s) => out.push((if star { "kwonly" } else { "pos" }, Some((**s).clone()), None, None)),
                _ => return err(format!("'{}' isn't a parameter", show(x)), Some(x)),
            }
        }
        Ok(out)
    }

    fn annotation(&self, text: &Option<String>) -> R<Obj<'py>> {
        match text {
            None => Ok(self.none()),
            Some(t) => Ok(self.ast.getattr("parse")?.call1((t.as_str(), "<annotation>", "eval"))?.getattr("body")?),
        }
    }

    /// In the function's scope: bind the parameters, make ast.arguments.
    #[allow(clippy::type_complexity)]
    fn build_arguments(&mut self, params: Vec<(&'static str, Option<Form>, Option<Res<'py>>, Option<String>)>) -> R<(Obj<'py>, Vec<Obj<'py>>)> {
        let (mut posonly, mut args, mut defaults, mut kwonly, mut kw_defaults) = (vec![], vec![], vec![], vec![], vec![]);
        let (mut vararg, mut kwarg) = (self.none(), self.none());
        let mut stmts = vec![];
        for (kind, s, d, ann) in params {
            let Some(s) = s else { continue };
            let py = self.bind(&s)?;
            let a = self.at(self.n("arg", vec![("arg", self.s(&py)), ("annotation", self.annotation(&ann)?)])?, &s);
            match kind {
                "pos" | "posonly" => {
                    if kind == "posonly" { posonly.push(a) } else { args.push(a) }
                    match d {
                        Some(d) => {
                            stmts.extend(d.stmts);
                            defaults.push(self.val(d.expr)?);
                        }
                        None if !defaults.is_empty() => return err(format!("parameter {} without a default after one with a default", show(&s)), Some(&s)),
                        None => {}
                    }
                }
                "kwonly" => {
                    kwonly.push(a);
                    match d {
                        Some(d) => {
                            stmts.extend(d.stmts);
                            kw_defaults.push(self.val(d.expr)?);
                        }
                        None => kw_defaults.push(self.none()),
                    }
                }
                "vararg" => vararg = a,
                _ => kwarg = a,
            }
        }
        let node = self.n("arguments", vec![("posonlyargs", self.list(posonly)?), ("args", self.list(args)?), ("vararg", vararg), ("kwonlyargs", self.list(kwonly)?), ("kw_defaults", self.list(kw_defaults)?), ("kwarg", kwarg), ("defaults", self.list(defaults)?)])?;
        Ok((node, stmts))
    }

    fn function_body(&mut self, children: &[Form], inline: Option<&[Form]>) -> R<Vec<Obj<'py>>> {
        let mut sc = std::mem::take(self.scope());
        self.prescan(children, &mut sc);
        *self.scope() = sc;
        let r = match inline {
            Some(items) => {
                if !children.is_empty() {
                    return err("a one-line def has no block", children.first());
                }
                self.compile_items(items, &[], Ctx::Tail, &[], &[])?
            }
            None => self.compile_body(children, Ctx::Tail)?,
        };
        if self.scope().is_gen {
            let s = self.as_stmts(r)?;
            return Ok(if s.is_empty() { vec![self.pass()?] } else { s });
        }
        let mut s = r.stmts;
        let v = self.val(r.expr)?;
        s.push(self.n("Return", vec![("value", v)])?);
        Ok(s)
    }

    fn decorator(&mut self, line: &Form) -> R<Res<'py>> {
        let mut items = line_parts(line).0.to_vec();
        if let Kind::Sym { name, marks } = &items[0].kind {
            items[0] = Form::sym_at(&name[1..], marks, items[0].pos);
        }
        if items.len() == 1 && items[0].is_sym("command") && self.ns_get("command").is_none() {
            return Ok(Res::e(self.rt("command")?));
        }
        self.name_slot(&items)
    }

    fn sf_def(&mut self, items: &[Form], children: &[Form], decos: &[Form], is_async: bool) -> R<Res<'py>> {
        let head = &items[0];
        let Some(nsym) = items.get(1).filter(|x| x.sym_name().is_some()) else { return err("def name parameters…", Some(head)) };
        let mut rest: &[Form] = &items[2..];
        let mut inline = None;
        if let Some(eq) = rest.iter().position(|x| x.is_op("=")) {
            inline = Some(&rest[eq + 1..]);
            rest = &rest[..eq];
        }
        let mut returns = self.none();
        if let Some(a) = rest.iter().position(|x| x.is_op("->")) {
            let r = self.name_slot(&rest[a + 1..])?;
            returns = self.val(r.expr)?;
            rest = &rest[..a];
        }
        let mut deco_rs = vec![];
        for d in decos {
            deco_rs.push(self.decorator(d)?);
        }
        let params = self.parse_params(rest)?;
        let py = self.bind(nsym)?;
        self.push("function");
        let result = (|| -> R<(Obj<'py>, Vec<Obj<'py>>, Vec<Obj<'py>>)> {
            let (arguments, dstmts) = self.build_arguments(params)?;
            let body = self.function_body(children, inline)?;
            Ok((arguments, dstmts, body))
        })();
        self.pop();
        let (arguments, dstmts, body) = result?;
        let (mut stmts, deco_es) = self.gather_e(deco_rs)?;
        stmts.extend(dstmts);
        let node = self.n(if is_async { "AsyncFunctionDef" } else { "FunctionDef" }, vec![("name", self.s(&py)), ("args", arguments), ("body", self.list(body)?), ("decorator_list", self.list(deco_es)?), ("returns", returns), ("type_params", self.list(vec![])?)])?;
        stmts.push(self.at(node, head));
        Ok(Res::s(stmts, self.name(&py, false)?))
    }

    fn sf_fn(&mut self, items: &[Form], children: &[Form]) -> R<Res<'py>> {
        let head = &items[0];
        if (items.len() > 1 && !matches!(items[1].kind, Kind::Group { open: '[', .. })) || (items.len() == 1 && !children.is_empty()) {
            // fn body with $% (or $%1 $%2…, and $%& for the rest): the parameters.
            let (mut most, mut rest) = (0u32, false);
            let mut scan = |f: Form| {
                if let Kind::Var(v) = &f.kind {
                    match v.sym_name() {
                        Some("%") => most = most.max(1),
                        Some("%&") => rest = true,
                        Some(n) if n.len() == 2 && n.starts_with('%') => most = most.max(n[1..].parse().unwrap_or(0)),
                        _ => {}
                    }
                }
                f
            };
            for x in items[1..].iter().chain(children) {
                x.clone().map(&mut scan);
            }
            self.st.marks += 1;
            let mark = self.st.marks;
            let mut rename = |f: Form| match &f.kind {
                Kind::Var(v) => match v.sym_name() {
                    Some(n) if n.starts_with('%') => {
                        let p = match n { "%" => "p1".to_string(), "%&" => "rest".to_string(), _ => format!("p{}", &n[1..]) };
                        Form::new(Kind::Var(Box::new(Form::sym_at(&p, &[mark], v.pos))), f.pos)
                    }
                    _ => f,
                },
                _ => f,
            };
            let mut params: Vec<Form> = (1..=most).map(|i| Form::sym_at(&format!("p{i}"), &[mark], head.pos)).collect();
            if rest {
                params.push(Form::sym_at("*rest", &[mark], head.pos));
            }
            let mut new_items = vec![head.clone(), Form::group('[', params, head.pos)];
            new_items.extend(items[1..].iter().map(|x| x.clone().map(&mut rename)));
            let kids: Vec<Form> = children.iter().map(|x| x.clone().map(&mut rename)).collect();
            return self.sf_fn(&new_items, &kids);
        }
        let Some(Kind::Group { open: '[', items: pitems }) = items.get(1).map(|x| &x.kind) else { return err("fn [parameters] body", Some(head)) };
        let params = self.parse_params(pitems)?;
        let body_items = &items[2..];
        self.push("function");
        let result = (|| -> R<(Obj<'py>, Vec<Obj<'py>>, Result<Obj<'py>, Vec<Obj<'py>>>)> {
            let (arguments, dstmts) = self.build_arguments(params)?;
            if !body_items.is_empty() && children.is_empty() {
                let r = self.compile_items(body_items, &[], Ctx::Tail, &[], &[])?;
                if r.stmts.is_empty() && !self.scope().is_gen {
                    return Ok((arguments, dstmts, Ok(self.val(r.expr)?)));
                }
                let mut body = r.stmts;
                body.push(self.n("Return", vec![("value", self.val(r.expr)?)])?);
                return Ok((arguments, dstmts, Err(body)));
            }
            let mut lines = vec![];
            if !body_items.is_empty() {
                lines.push(Form::line(body_items.to_vec(), vec![], body_items[0].pos));
            }
            lines.extend(children.iter().cloned());
            let body = self.function_body(&lines, None)?;
            Ok((arguments, dstmts, Err(body)))
        })();
        self.pop();
        let (arguments, dstmts, body) = result?;
        match body {
            Ok(e) => Ok(Res::s(dstmts, self.at(self.lambda(arguments, e)?, head))),
            Err(body) => {
                let t = self.tmp("fn");
                let node = self.n("FunctionDef", vec![("name", self.s(&t)), ("args", arguments), ("body", self.list(body)?), ("decorator_list", self.list(vec![])?), ("returns", self.none()), ("type_params", self.list(vec![])?)])?;
                let mut stmts = dstmts;
                stmts.push(self.at(node, head));
                Ok(Res::s(stmts, self.name(&t, false)?))
            }
        }
    }

    fn sf_class(&mut self, items: &[Form], children: &[Form], decos: &[Form]) -> R<Res<'py>> {
        let head = &items[0];
        let Some(nsym) = items.get(1).filter(|x| x.sym_name().is_some()) else { return err("class Name bases…", Some(head)) };
        let mut rs = vec![];
        for d in decos {
            rs.push(self.decorator(d)?);
        }
        let nd = rs.len();
        let mut kwnames = vec![];
        for x in &items[2..] {
            if let Kind::KwArg { name, value } = &x.kind {
                rs.push(self.name_slot(std::slice::from_ref(value))?);
                kwnames.push(Some(name.clone()));
            } else {
                rs.push(self.name_slot(std::slice::from_ref(x))?);
                kwnames.push(None);
            }
        }
        let (stmts0, es) = self.gather_e(rs)?;
        let deco_es = es[..nd].to_vec();
        let (mut bases, mut kws) = (vec![], vec![]);
        for (e, k) in es[nd..].iter().zip(kwnames) {
            match k {
                Some(k) => kws.push(self.keyword(Some(&k), e.clone())?),
                None => bases.push(e.clone()),
            }
        }
        let py = self.bind(nsym)?;
        self.push("class");
        let body = (|| -> R<Vec<Obj<'py>>> {
            let mut sc = std::mem::take(self.scope());
            self.prescan(children, &mut sc);
            *self.scope() = sc;
            let r = self.compile_body(children, Ctx::Stmt)?;
            let s = self.as_stmts(r)?;
            Ok(if s.is_empty() { vec![self.pass()?] } else { s })
        })();
        self.pop();
        let body = body?;
        let node = self.n("ClassDef", vec![("name", self.s(&py)), ("bases", self.list(bases)?), ("keywords", self.list(kws)?), ("body", self.list(body)?), ("decorator_list", self.list(deco_es)?), ("type_params", self.list(vec![])?)])?;
        let mut stmts = stmts0;
        stmts.push(self.at(node, head));
        Ok(Res::s(stmts, self.name(&py, false)?))
    }

    // ---- match ----------------------------------------------------------------

    fn sf_match(&mut self, items: &[Form], children: &[Form], ctx: Ctx) -> R<Res<'py>> {
        let subj = self.compile_expr(&items[1..], Ctx::Value, &[])?;
        let t = if ctx == Ctx::Stmt { None } else { Some(self.tmp("m")) };
        let mut cases = vec![];
        for c in children {
            let (ci, cc) = line_parts(c);
            if ci.is_empty() || !ci[0].is_sym("case") {
                return err("match takes case lines", Some(c));
            }
            let mut pat_items = &ci[1..];
            let g = pat_items.iter().position(|x| x.is_sym("if"));
            if let Some(g) = g {
                pat_items = &pat_items[..g];
            }
            // (The pattern first: the guard sees the names it binds.)
            let pat = self.pattern(pat_items, c)?;
            let mut guard = self.none();
            if let Some(g) = g {
                let r = self.compile_expr(&ci[1 + g + 1..], Ctx::Value, &[])?;
                guard = self.val(r.expr)?;
            }
            let body = self.branch(cc, ctx, &t)?;
            cases.push(self.n("match_case", vec![("pattern", pat), ("guard", guard), ("body", self.list(body)?)])?);
        }
        let mut stmts = subj.stmts;
        if let Some(t) = &t {
            stmts.push(self.assign(t, self.cnone()?)?);
        }
        let node = self.n("Match", vec![("subject", self.val(subj.expr)?), ("cases", self.list(cases)?)])?;
        stmts.push(self.at(node, &items[0]));
        Ok(Res::s(stmts, match t {
            Some(t) => self.name(&t, false)?,
            None => self.cnone()?,
        }))
    }

    fn pattern(&mut self, items: &[Form], f: &Form) -> R<Obj<'py>> {
        if items.is_empty() {
            return err("a pattern is missing", Some(f));
        }
        let alts = split(items, |x| x.is_op("|"));
        if alts.len() > 1 {
            let mut ps = vec![];
            for a in alts {
                ps.push(self.pattern(a, f)?);
            }
            return self.n("MatchOr", vec![("patterns", self.list(ps)?)]);
        }
        if let Some(a) = items.iter().position(|x| x.is_sym("as")) {
            let inner = self.pattern(&items[..a], f)?;
            let Some(tgt) = items.get(a + 1) else { return err("as needs a name", Some(&items[a])) };
            let py = self.bind(tgt)?;
            return self.n("MatchAs", vec![("pattern", inner), ("name", self.s(&py))]);
        }
        if items.len() > 1 {
            if !items[0].sym_name().is_some_and(is_dotted) {
                return err("a class pattern starts with the class", Some(&items[0]));
            }
            return self.class_pattern(&items[0], &items[1..]);
        }
        let x = &items[0];
        match &x.kind {
            Kind::Sym { name, .. } => match name.as_str() {
                "_" => self.n("MatchAs", vec![]),
                "None" => self.n("MatchSingleton", vec![("value", self.none())]),
                "True" | "False" => self.n("MatchSingleton", vec![("value", PyBool::new(self.py, name == "True").to_owned().into_any())]),
                n if n.contains('.') && is_dotted(n) => self.n("MatchValue", vec![("value", self.name_load(x)?)]),
                n => self.n("MatchValue", vec![("value", self.c(n)?)]),
            },
            Kind::Var(s) => {
                if s.is_sym("_") {
                    return self.n("MatchAs", vec![]);
                }
                let py = self.bind(s)?;
                self.n("MatchAs", vec![("name", self.s(&py))])
            }
            Kind::Num(t) => self.n("MatchValue", vec![("value", self.num(t)?)]),
            Kind::Str { value, .. } => self.n("MatchValue", vec![("value", self.c(value.as_str())?)]),
            Kind::Splat { value, n: 1 } => {
                let name = match &value.kind {
                    Kind::Var(s) if !s.is_sym("_") => { let b = self.bind(s)?; self.s(&b) }
                    _ => self.none(),
                };
                self.n("MatchStar", vec![("name", name)])
            }
            Kind::Group { open: '[', items: sub } => {
                let parts: Vec<Vec<Form>> = if sub.iter().any(|y| y.is_comma()) { comma_parts(sub).into_iter().map(|p| p.to_vec()).collect() } else { sub.iter().map(|y| vec![y.clone()]).collect() };
                let mut ps = vec![];
                for p in parts {
                    ps.push(self.pattern(&p, f)?);
                }
                self.n("MatchSequence", vec![("patterns", self.list(ps)?)])
            }
            Kind::Group { open: '(', items: sub } => {
                if sub.is_empty() {
                    return self.n("MatchSequence", vec![("patterns", self.list(vec![])?)]);
                }
                if let Some(n) = sub[0].sym_name() {
                    if is_dotted(n) && (sub.len() > 1 || n.starts_with(|c: char| c.is_uppercase()) || self.py_builtins.contains(n)) {
                        return self.class_pattern(&sub[0], &sub[1..]);
                    }
                }
                self.pattern(sub, f)
            }
            Kind::Group { open: '{', items: sub } => {
                let sub: Vec<Form> = sub.iter().filter(|y| !y.is_comma()).cloned().collect();
                let (mut keys, mut pats) = (vec![], vec![]);
                let mut rest = self.none();
                let mut k = 0;
                while k < sub.len() {
                    if let Kind::Splat { value, n: 2 } = &sub[k].kind {
                        let b = self.bind(value)?;
                        rest = self.s(&b);
                        k += 1;
                        continue;
                    }
                    let Some(kr) = self.key(&sub[k])? else { return err("a mapping pattern is {key: pattern …}", Some(&sub[k])) };
                    if k + 1 >= sub.len() {
                        return err("a key needs a pattern", Some(&sub[k]));
                    }
                    keys.push(self.val(kr.expr)?);
                    pats.push(self.pattern(std::slice::from_ref(&sub[k + 1]), f)?);
                    k += 2;
                }
                self.n("MatchMapping", vec![("keys", self.list(keys)?), ("patterns", self.list(pats)?), ("rest", rest)])
            }
            _ => err(format!("'{}' isn't a pattern", show(x)), Some(x)),
        }
    }

    fn class_pattern(&mut self, head: &Form, args: &[Form]) -> R<Obj<'py>> {
        let (mut pos, mut kwn, mut kwp) = (vec![], vec![], vec![]);
        for y in args {
            if let Kind::KwArg { name, value } = &y.kind {
                kwn.push(self.s(name));
                kwp.push(self.pattern(std::slice::from_ref(value), y)?);
            } else {
                pos.push(self.pattern(std::slice::from_ref(y), y)?);
            }
        }
        self.n("MatchClass", vec![("cls", self.name_load(head)?), ("patterns", self.list(pos)?), ("kwd_attrs", self.list(kwn)?), ("kwd_patterns", self.list(kwp)?)])
    }

    // ---- quoting and macros ---------------------------------------------------

    /// Python that builds `form`: the template kept here (by number), its
    /// unquotes filled in when it runs.
    fn quote(&mut self, form: &Form, quasi: bool) -> R<Res<'py>> {
        let mut holes: Vec<Form> = vec![];
        let template = if quasi { crate::pyforms::make_template(form, &mut holes) } else { form.clone() };
        let id = crate::pyforms::register_template(template);
        let mut rs = vec![];
        for h in &holes {
            rs.push(match &h.kind {
                Kind::Sym { .. } => Res::e(self.name_load(h)?),
                Kind::Group { open: '(', .. } => self.compile_group(h, Ctx::Value)?,
                _ => self.compile_value(h, Ctx::Value)?,
            });
        }
        let (stmts, es) = self.gather_e(rs)?;
        let f = self.attr(self.name(FM, false)?, "template", false)?;
        Ok(Res::s(stmts, self.call(f, vec![self.c(id)?, self.list_node(es)?], vec![])?))
    }

    fn macro_ns(&self) -> R<Bound<'py, PyDict>> {
        match &self.ns {
            Some(ns) => Ok(ns.clone()),
            None => Ok(crate::runner::scratch_namespace(self.py)?),
        }
    }

    fn sf_defmacro(&mut self, items: &[Form], children: &[Form]) -> R<Res<'py>> {
        let head = &items[0];
        let Some(mname) = items.get(1).and_then(|x| x.sym_name()) else { return err("defmacro name parameters…", Some(head)) };
        let mut params = vec![];
        for x in &items[2..] {
            let Some(n) = x.sym_name() else { return err("macro parameters are names, *rest or name:expr", Some(x)) };
            if let Some(r) = n.strip_prefix('*') {
                params.push(("rest", r.to_string()));
            } else if let Some(r) = n.strip_suffix(":expr") {
                params.push(("expr", r.to_string()));
            } else {
                params.push(("one", n.to_string()));
            }
        }
        let fake: Vec<Form> = params.iter().map(|(k, n)| Form::sym_at(&format!("{}{n}", if *k == "rest" { "*" } else { "" }), &[], head.pos)).collect();
        let pyname = format!("_tessh_macro_{mname}");
        let saved = std::mem::take(&mut self.scopes);
        self.push("function");
        let r = (|| -> R<(Obj<'py>, Vec<Obj<'py>>)> {
            let p = self.parse_params(&fake)?;
            let (arguments, _) = self.build_arguments(p)?;
            let body = self.function_body(children, None)?;
            Ok((arguments, body))
        })();
        self.scopes = saved;
        let (arguments, body) = r?;
        let fdef = self.n("FunctionDef", vec![("name", self.s(&pyname)), ("args", arguments), ("body", self.list(body)?), ("decorator_list", self.list(vec![])?), ("returns", self.none()), ("type_params", self.list(vec![])?)])?;
        self.at(fdef.clone(), head);
        let module = self.n("Module", vec![("body", self.list(vec![fdef])?), ("type_ignores", self.list(vec![])?)])?;
        let module = self.ast.getattr("fix_missing_locations")?.call1((module,))?;
        let ns = self.macro_ns()?;
        let builtins = self.py.import("builtins")?;
        let code = builtins.getattr("compile")?.call1((module, format!("<macro {mname}>"), "exec")).map_err(|e| CompileError { msg: format!("in macro {mname}: {e}"), pos: head.pos })?;
        builtins.getattr("exec")?.call1((code, &ns)).map_err(|e| CompileError { msg: format!("in macro {mname}: {e}"), pos: head.pos })?;
        let func = ns.get_item(&pyname)?.ok_or_else(|| CompileError { msg: "macro vanished".into(), pos: head.pos })?.unbind();
        self.st.macros.insert(mname.to_string(), Macro { name: mname.to_string(), func, params });
        Ok(Res::e(self.cnone()?))
    }

    pub fn macro_args(&self, m: &Macro, inline: &[Form], children: &[Form], head: &Form) -> R<Vec<Form>> {
        let mut inline: std::collections::VecDeque<Form> = inline.iter().cloned().collect();
        let mut kids: std::collections::VecDeque<Form> = children.iter().cloned().collect();
        let mut args = vec![];
        for (kind, n) in &m.params {
            match *kind {
                "one" => {
                    if let Some(x) = inline.pop_front() {
                        args.push(x);
                    } else if let Some(c) = kids.pop_front() {
                        let (ci, cc) = line_parts(&c);
                        args.push(if ci.len() == 1 && cc.is_empty() { ci[0].clone() } else { c.clone() });
                    } else {
                        return err(format!("macro {} needs {n}", m.name), Some(head));
                    }
                }
                "expr" => {
                    if inline.is_empty() {
                        return err(format!("macro {} needs {n}", m.name), Some(head));
                    }
                    let v: Vec<Form> = inline.drain(..).collect();
                    args.push(if v.len() == 1 { v[0].clone() } else { Form::group('(', v.clone(), v[0].pos) });
                }
                _ => {
                    args.extend(inline.drain(..));
                    args.extend(kids.drain(..));
                }
            }
        }
        if let Some(x) = inline.front().or(kids.front()) {
            return err(format!("macro {} got too many arguments", m.name), Some(x));
        }
        Ok(args)
    }

    fn expand_macro(&mut self, name: &str, items: &[Form], children: &[Form], ctx: Ctx) -> R<Res<'py>> {
        let head = &items[0];
        let m = self.st.macros.get(name).unwrap();
        let args = self.macro_args(m, &items[1..], children, head)?;
        let func = m.func.clone_ref(self.py);
        self.st.marks += 1;
        let mark = self.st.marks;
        let head_t = head.clone().toggle_mark(mark);
        let mut pyargs = vec![];
        for a in args {
            pyargs.push(crate::pyforms::to_py(self.py, a.toggle_mark(mark))?);
        }
        self.depth += 1;
        if self.depth > 200 {
            self.depth -= 1;
            return err(format!("macro {name} expands forever"), Some(head));
        }
        let out = crate::pyforms::with_head(head_t, || func.bind(self.py).call1(PyTuple::new(self.py, pyargs)?));
        let out = match out {
            Ok(o) => o,
            Err(e) => {
                self.depth -= 1;
                if let Some(ce) = crate::pyforms::compile_error_from(self.py, &e) {
                    return Err(ce);
                }
                return err(format!("macro {name}: {e}"), Some(head));
            }
        };
        let result = (|| -> R<Res<'py>> {
            let outs = crate::pyforms::from_py_many(&out)?;
            let outs: Vec<Form> = outs.into_iter().map(|f| f.toggle_mark(mark).positioned(head.pos)).collect();
            if outs.len() != 1 || out.is_instance_of::<PyList>() {
                return self.compile_body(&crate::pyforms::as_lines(outs), ctx);
            }
            let f = outs.into_iter().next().unwrap();
            if matches!(f.kind, Kind::Line { .. }) {
                return self.compile_line(&f, ctx);
            }
            self.compile_items(std::slice::from_ref(&f), &[], ctx, &[], &[])
        })();
        self.depth -= 1;
        result
    }

    fn sf_defop(&mut self, items: &[Form]) -> R<Res<'py>> {
        if items.len() != 5 {
            return err("defop OP precedence associativity function", Some(&items[0]));
        }
        let Some(op) = items[1].op_name() else { return err("an operator is made of + - * / < > = ! & | % ^ ~ @ ? : .", Some(&items[1])) };
        let p: u8 = match &items[2].kind {
            Kind::Num(t) => t.parse().unwrap_or(0),
            _ => 0,
        };
        if !(1..=13).contains(&p) {
            return err("precedence is 1 to 13", Some(&items[2]));
        }
        let assoc = match items[3].sym_name() {
            Some("left") => Assoc::Left,
            Some("right") => Assoc::Right,
            Some("none") => Assoc::None,
            _ => return err("associativity is left, right or none", Some(&items[3])),
        };
        if items[4].sym_name().is_none() {
            return err("the function is a name", Some(&items[4]));
        }
        self.st.ops.insert(op.to_string(), (p, assoc, items[4].clone()));
        Ok(Res::e(self.cnone()?))
    }

    // ---- whole programs -------------------------------------------------------

    pub fn units_of(&self, lines: &[Form]) -> R<Vec<Unit>> {
        self.units(lines)
    }

    /// One top-level unit as a Python module (show: display its value).
    pub fn unit_module(&mut self, u: &Unit, ctx: Ctx, show_value: bool) -> R<Obj<'py>> {
        // (Definitions, imports and assignments aren't shown, as in Python.)
        let items = line_parts(&u.0).0;
        let quiet = self.assign_index(items).is_some()
            || items.first().and_then(|h| h.sym_name()).is_some_and(|n| ["def", "class", "async", "import", "from", "defmacro", "defop", "global", "nonlocal", "del", "pass", "for", "while"].contains(&n) || n.starts_with('@'));
        let show_value = show_value && !quiet;
        let r = self.compile_unit(u, ctx)?;
        let mut body = r.stmts;
        if show_value {
            let v = self.val(r.expr)?;
            body.push(self.expr_stmt(self.rcall("show", vec![v])?)?);
        } else if let Some(e) = r.expr {
            if !self.is_pure(&e) {
                body.push(self.expr_stmt(e)?);
            }
        }
        if body.is_empty() {
            body.push(self.pass()?);
        }
        for s in &body {
            if s.getattr("lineno").map(|v| v.is_none()).unwrap_or(true) {
                self.at(s.clone(), &u.0);
            }
        }
        let m = self.n("Module", vec![("body", self.list(body)?), ("type_ignores", self.list(vec![])?)])?;
        Ok(self.ast.getattr("fix_missing_locations")?.call1((m,))?)
    }

    pub fn module(&mut self, lines: &[Form]) -> R<Obj<'py>> {
        self.prescan_module(lines);
        let mut body: Vec<Obj<'py>> = vec![];
        for u in self.units(lines)? {
            let m = self.unit_module(&u, Ctx::Stmt, false)?;
            for s in m.getattr("body")?.try_iter()? {
                body.push(s?);
            }
        }
        let m = self.n("Module", vec![("body", self.list(body)?), ("type_ignores", self.list(vec![])?)])?;
        Ok(self.ast.getattr("fix_missing_locations")?.call1((m,))?)
    }
}

fn takes_cont(c: &Compiler, u: &Unit, cont: &str) -> bool {
    let (items0, _) = line_parts(&u.0);
    let items = match c.assign_index(items0) {
        Some(i) if i + 1 < items0.len() => &items0[i + 1..],
        _ => items0,
    };
    let mut hn = items.first().and_then(|h| h.sym_name()).unwrap_or("");
    if hn == "async" {
        hn = items.get(1).and_then(|h| h.sym_name()).unwrap_or("");
    }
    let last = u.1.last().and_then(|l| line_parts(l).0.first().and_then(|h| h.sym_name())).unwrap_or(hn);
    match hn {
        "if" => (cont == "elif" || cont == "else") && last != "else",
        "for" | "while" => cont == "else" && last != "else",
        "try" => match cont {
            "except" | "except*" => ["try", "except", "except*"].contains(&last),
            "else" => last == "except" || last == "except*",
            "finally" => last != "finally",
            _ => false,
        },
        _ => false,
    }
}

/// Python code from an indented string: its common indentation off.
fn dedent(code: &str) -> String {
    let code = code.strip_prefix('\n').unwrap_or(code);
    let ind = code.lines().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    code.lines().map(|l| if l.len() >= ind { &l[ind..] } else { l.trim_start() }).collect::<Vec<_>>().join("\n")
}

/// `.name` (a method) or `.-name` (a field), as a call's head.
fn member(f: &Form) -> Option<(&str, bool)> {
    let n = f.sym_name()?.strip_prefix('.')?;
    if let Some(field) = n.strip_prefix('-') {
        return is_ident(field).then_some((field, true));
    }
    is_ident(n).then_some((n, false))
}

/// A threading step with the value put in: first (->) or last (->>).
fn thread_into(step: &Form, x: Form, first: bool) -> Form {
    match &step.kind {
        Kind::Group { open: '(', items } if !items.is_empty() => {
            let mut v = items.clone();
            if first {
                v.insert(1, x);
            } else {
                v.push(x);
            }
            Form::group('(', v, step.pos)
        }
        _ => Form::group('(', vec![step.clone(), x], step.pos),
    }
}

fn is_stage_head(x: &Form) -> bool {
    match &x.kind {
        Kind::Sym { name, .. } => !WORD_OPS.contains(&name.as_str()) && !["True", "False", "None"].contains(&name.as_str()),
        Kind::Caret(_) => true,
        Kind::Word(ps) => matches!(ps.first(), Some(Part::Text(_))),
        _ => false,
    }
}

fn flatten(t: Tree, is_and: bool, out: &mut Vec<Tree>) {
    match t {
        Tree::Bin(op, l, r, f) => {
            let same = if is_and { op == "and" || op == "&&" } else { op == "or" || op == "||" };
            if same {
                flatten(*l, is_and, out);
                flatten(*r, is_and, out);
            } else {
                out.push(Tree::Bin(op, l, r, f));
            }
        }
        t => out.push(t),
    }
}

fn target_names(t: &Form, out: &mut HashSet<String>) {
    match &t.kind {
        Kind::Sym { name, .. } => {
            let n = name.trim_start_matches('*');
            if is_ident(n) {
                out.insert(n.to_string());
            }
        }
        Kind::Var(s) => target_names(s, out),
        Kind::Group { items, .. } => {
            for x in items {
                target_names(x, out);
            }
        }
        Kind::Splat { value, .. } => target_names(value, out),
        _ => {}
    }
}

/// `name := …`: an assignment expression, whoever `name` is.
fn walrus(items: &[Form]) -> bool {
    items.len() > 2 && matches!(&items[1].kind, Kind::Op { name, .. } if name == ":=")
}

/// A bare word written as a path: /…, ~ or ~/…, ./…, ../…, . or .. — not a
/// glob (that's a list of names).
pub fn is_path_word(n: &str) -> bool {
    is_path_start(n) && !n.contains(['*', '?', '['])
}

/// How a path starts.
fn is_path_start(t: &str) -> bool {
    t.starts_with('/') && t.len() > 1 && !t.starts_with("//") || t == "~" || t.starts_with("~/") || t.starts_with("./") || t.starts_with("../") || t == "." || t == ".."
}

/// A bare word with glob characters (*, ?, […]): *, *.txt, src/**/x, a$n*.
fn is_glob_word(x: &Form) -> bool {
    let glob = |t: &str| t.contains(['*', '?', '[']);
    match &x.kind {
        Kind::Sym { name, .. } => glob(name),
        Kind::Op { name, tick: false } => name == "*",
        Kind::Word(ps) => ps.iter().any(|p| matches!(p, Part::Text(t) if glob(t))),
        _ => false,
    }
}
