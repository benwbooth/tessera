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
/// Where programs run when tessh has no terminal (the kernel: in the
/// notebook's pane terminal, so a terminal app gets one). Given the shell
/// command and the folder, it gives the exit status, or None to run it here.
pub static TTY_HOOK: std::sync::OnceLock<fn(Python<'_>, &str, &str) -> PyResult<Option<i32>>> = std::sync::OnceLock::new();
/// Told of each program (or pipeline) run, before it runs: {command, argv,
/// cwd, user, env} (the kernel: the notebook's card shows what it ran with).
pub static EXEC_HOOK: std::sync::OnceLock<fn(serde_json::Value)> = std::sync::OnceLock::new();

/// The user tessh runs as.
pub fn user_name() -> String {
    let uid = unsafe { libc::getuid() };
    let pw = unsafe { libc::getpwuid(uid) };
    if pw.is_null() {
        return std::env::var("USER").unwrap_or_else(|_| uid.to_string());
    }
    unsafe { std::ffi::CStr::from_ptr((*pw).pw_name) }.to_string_lossy().to_string()
}

/// The environment programs get, NAME=value, sorted.
pub fn env_list() -> Vec<String> {
    let mut env: Vec<String> = std::env::vars_os().map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy())).collect();
    env.sort();
    env
}

/// What a pipeline runs with, for EXEC_HOOK.
fn exec_record(cmds: &[Stage]) -> serde_json::Value {
    let line = cmds
        .iter()
        .map(|c| {
            let mut w: Vec<String> = c.words.iter().map(|w| sh_quote(w)).collect();
            for (op, t) in &c.redirs {
                w.push(match t {
                    Some(t) => format!("{op} {}", sh_quote(t)),
                    None => op.clone(),
                });
            }
            w.join(" ")
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    serde_json::json!({"command": line, "argv": cmds.iter().map(|c| c.words.clone()).collect::<Vec<_>>(),
                       "cwd": cwd, "user": user_name(), "env": env_list()})
}

/// The environment when the hook was set: what differs is passed along.
pub static HOOK_ENV: std::sync::OnceLock<std::collections::HashMap<String, String>> = std::sync::OnceLock::new();
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

# ---- pretty printing ------------------------------------------------------

_notebook = False   # (the kernel sets it: cards show color)

def _wants_color(f=None):
    import os, sys
    if os.environ.get("NO_COLOR") or os.environ.get("TERM") == "dumb":
        return False
    if _notebook:
        return True
    f = f if f is not None else sys.stdout
    try:
        return f.isatty()
    except Exception:
        return False

def _width():
    import os, shutil
    if _notebook:
        return 80   # (a card is narrower than the terminal $COLUMNS speaks for)
    try:
        return int(os.environ["COLUMNS"])
    except (KeyError, ValueError):
        pass
    return shutil.get_terminal_size((100, 24)).columns

_COLORS = {"str": "32", "num": "33", "key": "36", "const": "35", "punct": "90", "name": "34"}

def _shown_len(s):
    import re
    return len(re.sub("\x1b\\[[0-9;]*m", "", s))

def _kind(x):
    import dataclasses, os, pathlib, collections
    t = type(x)
    if t in (list, tuple, set, frozenset, dict):
        return t.__name__
    if isinstance(x, tuple) and hasattr(t, "_fields"):
        return "record"
    if dataclasses.is_dataclass(x) and not isinstance(x, type) and t.__dataclass_params__.repr:
        return "record"
    # (The environment: its variables, as a dict.)
    if t.__name__ == "Env" and t.__module__ == "__tessh__" or x is os.environ:
        return "env"
    if t in (collections.Counter, collections.OrderedDict, collections.defaultdict, collections.deque, range) or isinstance(x, pathlib.PurePath):
        return "call"
    return None

class _Name(str):
    """A function's name, shown as its variable: (defaultdict $list {…})."""

def _str_lit(x):
    """A str as tessh reads it back: 'raw' when it can be, else "escaped"."""
    if "'" not in x and x.isprintable():
        return "'" + x + "'"
    out = []
    for ch in x:
        if ch in '\\"$':
            out.append("\\" + ch)
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        elif ch == "\r":
            out.append("\\r")
        elif not ch.isprintable():
            o = ord(ch)
            out.append(f"\\x{o:02x}" if o < 0x100 else f"\\u{o:04x}" if o < 0x10000 else f"\\U{o:08x}")
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'

def _bytes_lit(x):
    out = []
    for b in x:
        ch = chr(b)
        if ch in '\\"':
            out.append("\\" + ch)
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        elif 32 <= b < 127:
            out.append(ch)
        else:
            out.append(f"\\x{b:02x}")
    return 'b"' + "".join(out) + '"'

def pretty(x, color=None, width=None, indent=0, _tree_of=None):
    """x as tessh code that reads back as an equal value: [1 2], {a: 1},
    {1 2}, (1, 2), 'text', (Point x=1 y=2) for a record (a dataclass or
    namedtuple).  What has no such form (a function, a file) is shown as
    Python shows it.  Lists, dicts, sets, tuples and records that don't fit
    in width are laid out over indented lines, colored by type when color is
    true (None, the default: when the output is a terminal or a notebook).
    print uses it for these.  pretty {a: [1 2]} -> "{a: [1 2]}" """
    import math
    if color is None:
        color = _wants_color()
    if width is None:
        width = _width()
    def c(kind, t):
        return f"\x1b[{_COLORS[kind]}m{t}\x1b[0m" if color else t
    def parts(x):
        """(open, [(key or None, value)], close, tuple-like) for a container."""
        k = _kind(x)
        if k == "list":
            return c("punct", "["), [(None, v) for v in x], c("punct", "]"), False
        if k == "tuple":
            return c("punct", "("), [(None, v) for v in x], c("punct", ")"), True
        if k in ("set", "frozenset"):
            try:
                items = sorted(x)
            except TypeError:
                items = list(x)
            if not x:
                return c("punct", "(") + c("name", k), [], c("punct", ")"), False
            if k == "set":
                return c("punct", "{"), [(None, v) for v in items], c("punct", "}"), False
            return c("punct", "(") + c("name", "frozenset") + " " + c("punct", "{"), [(None, v) for v in items], c("punct", "})"), False
        if k == "dict":
            return c("punct", "{"), [(("key", kk), v) for kk, v in x.items()], c("punct", "}"), False
        if k == "env":
            return c("punct", "{"), [(("key", kk), v) for kk, v in sorted(x.items())], c("punct", "}"), False
        if k == "call":
            import collections, pathlib
            t = type(x)
            if isinstance(x, pathlib.PurePath):
                name, args = "Path", [str(x)]
            elif t is range:
                name, args = "range", ([x.start] if x.start else []) + [x.stop] + ([x.step] if x.step != 1 else [])
                if x.step != 1 and not x.start:
                    args = [0] + args
            elif t is collections.deque:
                name, args = "deque", [list(x)] + ([x.maxlen] if x.maxlen is not None else [])
            elif t is collections.defaultdict:
                f = x.default_factory
                fname = getattr(f, "__name__", None) if f is not None else "None"
                if f is not None and (fname is None or fname == "<lambda>"):
                    return None
                name, args = "defaultdict", [_Name(fname), dict(x)]
            else:
                name, args = t.__name__, [dict(x)]
            return c("punct", "(") + c("name", name), [(None, v) for v in args], c("punct", ")"), "record"
        if k == "record":
            import dataclasses
            if dataclasses.is_dataclass(x):
                fs = [(("field", f.name), getattr(x, f.name)) for f in dataclasses.fields(x) if f.repr]
            else:
                fs = [(("field", n), getattr(x, n)) for n in type(x)._fields]
            return c("punct", "(") + c("name", type(x).__name__), fs, c("punct", ")"), "record"
        return None
    def scalar(x):
        if x is None or x is True or x is False:
            return c("const", repr(x))
        if type(x) is int:
            return c("num", repr(x))
        if type(x) is float:
            if math.isfinite(x):
                return c("num", repr(x))
            return c("punct", "(") + "float " + c("str", "'" + repr(x) + "'") + c("punct", ")")
        if type(x) is complex:
            return c("punct", "(") + "complex " + c("num", repr(x.real)) + " " + c("num", repr(x.imag)) + c("punct", ")")
        if type(x) is _Name:
            return c("name", "$" + x if x != "None" else x)
        if type(x) is str:
            return c("str", _str_lit(x))
        if type(x) is bytes:
            return c("str", _bytes_lit(x))
        return repr(x)
    def key(k):
        kind, v = k
        if kind == "field":
            return c("key", v) + c("punct", "=")
        if type(v) is str and v.isidentifier() and v.isascii():
            return c("key", v) + c("punct", ":") + " "
        if v is None or v is True or v is False:
            return c("punct", "(") + c("const", repr(v)) + c("punct", ")") + c("punct", ":") + " "
        if type(v) is str:
            return c("key", _str_lit(v)) + c("punct", ":") + " "
        return flat(v) + c("punct", ":") + " "
    def field_value(k, v, text):
        # (A record's field: a [list], {dict} or {set} reads there in parentheses.)
        if k and k[0] == "field" and _kind(v) in ("list", "dict", "set"):
            return c("punct", "(") + text + c("punct", ")")
        return text
    seen = set()
    def flat(x):
        p = parts(x)
        if p is None:
            return scalar(x)
        o, items, e, kind = p
        if id(x) in seen:
            return o + "..." + e
        seen.add(id(x))
        texts = [(key(k) if k else "") + field_value(k, v, flat(v)) for k, v in items]
        seen.discard(id(x))
        if kind == "record":
            return o + "".join(" " + t for t in texts) + e
        if kind is True:
            if len(texts) == 1:
                return o + texts[0] + c("punct", ",") + e
            return o + c("punct", ", ").join(texts) + e
        return o + " ".join(texts) + e
    def lay(x, at, col):
        """x with its lines indented by at, starting at column col."""
        one = flat(x)
        p = parts(x)
        if p is None or col + _shown_len(one) < width or not p[1] or id(x) in seen:
            return one
        seen.add(id(x))
        o, items, e, kind = p
        comma = c("punct", ",") if kind is True else ""
        pad = " " * (at + 2)
        lines = []
        # (Plain values with no keys: as many to a line as fit.)
        if kind != "record" and all(k is None and parts(v) is None for k, v in items):
            row = ""
            for _, v in items:
                t = scalar(v) + comma
                if row and _shown_len(pad + row + " " + t) > width:
                    lines.append(pad + row)
                    row = ""
                row = row + " " + t if row else t
            lines.append(pad + row)
            items = []
        for k, v in items:
            head = key(k) if k else ""
            wrapped = k is not None and k[0] == "field" and _kind(v) in ("list", "dict", "set")
            body = lay(v, at + 2, at + 2 + _shown_len(head) + (1 if wrapped else 0))
            lines.append(pad + head + field_value(k, v, body) + comma)
        seen.discard(id(x))
        return o + "\n" + "\n".join(lines) + "\n" + " " * at + e
    if _tree_of is not None:
        return _tree_of(x, parts, flat, key, field_value, lay, width)
    return lay(x, indent, indent)

def _tree(x, parts, flat, key, field_value, lay, width):
    """x for a notebook card to fold: {"s": text} for a plain value, or
    {"o": open, "c": close, "f": the whole on a line (or a count), "e":
    open at first, "items": [{"k": key, "v": node, "w": wrapped}], "t": tuple}.
    Texts are colored (ANSI)."""
    seen = set()
    def node(v, col, at):
        p = parts(v)
        if p is None or id(v) in seen:
            return {"s": flat(v)}
        o, items, e, kind = p
        one = flat(v)
        n = len(items)
        # (Folded, it's the whole on a line where that fits, else a count.)
        short = one if col + _shown_len(one) < width else o + "\x1b[90m… " + str(n) + (" item" if n == 1 else " items") + " …\x1b[0m" + e
        # (Plain values with no keys: as many to a line as fit, as pretty does.)
        if kind != "record" and items and all(k is None and parts(vv) is None for k, vv in items):
            comma = "\x1b[90m,\x1b[0m" if kind is True and _wants_color() else ("," if kind is True else "")
            rows, row = [], ""
            for _, vv in items:
                t = flat(vv) + comma
                if row and at + 2 + _shown_len(row + " " + t) > width:
                    rows.append(row)
                    row = ""
                row = row + " " + t if row else t
            rows.append(row)
            return {"o": o, "c": e, "f": short, "e": col + _shown_len(one) >= width, "rows": rows, "t": kind is True}
        seen.add(id(v))
        kids = []
        for k, vv in items:
            head = key(k) if k else ""
            wrapped = k is not None and k[0] == "field" and _kind(vv) in ("list", "dict", "set")
            kids.append({"k": head, "v": node(vv, at + 2 + _shown_len(head), at + 2), "w": wrapped})
        seen.discard(id(v))
        return {"o": o, "c": e, "f": short, "e": n > 0 and col + _shown_len(one) >= width, "items": kids, "t": kind is True}
    return node(x, 0, 0)

def tree(x, color=True, width=None):
    """x as a notebook card folds it (see _tree)."""
    return pretty(x, color=color, width=width, _tree_of=_tree)

def print(*args, sep=" ", end="\n", file=None, flush=False):
    """print: values separated by spaces and a newline after them, as
    Python's print; lists, dicts, sets, tuples and records are laid out by
    pretty (indented over lines when long, colored on a terminal or in a
    notebook).  print $config"""
    import builtins, sys
    out = file if file is not None else sys.stdout
    if file is None or file is sys.stdout:
        color = _wants_color(out)
        args = [pretty(a, color=color) if _kind(a) else a for a in args]
    builtins.print(*args, sep=sep, end=end, file=out, flush=flush)

class _Glob(str):
    pass

def pick(d, keys, defaults, rest):
    """{a b c=0 **more} = d"""
    mapping = hasattr(d, "keys") and hasattr(d, "__getitem__")
    out = []
    for k in keys:
        if k in defaults:
            out.append(d.get(k, defaults[k]) if mapping else getattr(d, k, defaults[k]))
        else:
            out.append(d[k] if mapping else getattr(d, k))
    if rest:
        out.append({k: v for k, v in (d.items() if mapping else vars(d).items()) if k not in keys})
    return tuple(out)

# ---- Clojure's core functions, Python-sized -------------------------------------

import functools as _ft, itertools as _it

partial = _ft.partial

def identity(x):
    """x itself.  map identity $xs"""
    return x

def constantly(x):
    """A function that always returns x, whatever it's called with."""
    return lambda *a, **k: x

def comp(*fs):
    """comp f g h: a function doing h, then g, then f."""
    if not fs:
        return identity
    def composed(*a, **k):
        v = fs[-1](*a, **k)
        for f in reversed(fs[:-1]):
            v = f(v)
        return v
    return composed

def complement(f):
    """A function returning not f x.  filter (complement even) $xs"""
    return lambda *a, **k: not f(*a, **k)

def juxt(*fs):
    """juxt f g: a function returning [f x, g x]."""
    return lambda *a, **k: [f(*a, **k) for f in fs]

def inc(x):
    """x + 1."""
    return x + 1

def dec(x):
    """x - 1."""
    return x - 1

def first(xs):
    """The first item of xs, or None if it's empty.  first [1 2 3] -> 1"""
    return next(iter(xs), None)

def second(xs):
    """The second item of xs, or None.  second [1 2 3] -> 2"""
    return next(_it.islice(xs, 1, None), None)

def last(xs):
    """The last item of xs, or None if it's empty.  last [1 2 3] -> 3"""
    if hasattr(xs, "__getitem__") and hasattr(xs, "__len__"):
        return xs[-1] if len(xs) else None
    v = None
    for v in xs:
        pass
    return v

def rest(xs):
    """All of xs but the first, as a list.  rest [1 2 3] -> [2 3]"""
    return list(_it.islice(xs, 1, None))

def butlast(xs):
    """All of xs but the last, as a list.  butlast [1 2 3] -> [1 2]"""
    return list(xs)[:-1]

def take(n, xs):
    """The first n items of xs (any iterable, endless ones too), as a list.  take 2 [1 2 3] -> [1 2]"""
    return list(_it.islice(xs, n))

def drop(n, xs):
    """xs without its first n items, as a list.  drop 1 [1 2 3] -> [2 3]"""
    return list(_it.islice(xs, n, None))

def take_while(pred, xs):
    """Items of xs from the start while pred is true.  take_while (fn x ($x < 3)) [1 2 3 1] -> [1 2]"""
    return list(_it.takewhile(pred, xs))

def drop_while(pred, xs):
    """xs from the first item where pred is false.  drop_while (fn x ($x < 3)) [1 2 3 1] -> [3 1]"""
    return list(_it.dropwhile(pred, xs))

def some(pred, xs):
    """The first true pred x, else None."""
    for x in xs:
        v = pred(x)
        if v:
            return v
    return None

def every(pred, xs):
    """Whether pred is true for every item of xs (true for none)."""
    return all(pred(x) for x in xs)

def keep(f, xs):
    """f of each x, leaving out the Nones."""
    return [v for v in map(f, xs) if v is not None]

def mapcat(f, *xss):
    """f of each item (of several lists, side by side), the results joined as one list.  mapcat (fn x [$x $x]) [1 2] -> [1 1 2 2]"""
    return [y for ys in map(f, *xss) for y in ys]

def frequencies(xs):
    """How many times each item appears: {item: count}.  frequencies 'abca' -> {a: 2 b: 1 c: 1}"""
    out = {}
    for x in xs:
        out[x] = out.get(x, 0) + 1
    return out

def group_by(f, xs):
    """Items grouped by f of each: {f x: [x …]}, in order.  group_by len [a bb c] -> {1: [a c] 2: [bb]}"""
    out = {}
    for x in xs:
        out.setdefault(f(x), []).append(x)
    return out

def partition(n, xs, step=None, pad=None):
    """Lists of n (every step); a short last one only with pad."""
    xs = list(xs)
    step = step or n
    out = []
    for i in range(0, len(xs), step):
        chunk = xs[i:i + n]
        if len(chunk) < n:
            if pad is None:
                break
            chunk = (chunk + list(pad))[:n]
        out.append(chunk)
    return out

def partition_all(n, xs, step=None):
    """Lists of n (every step), the last one short if they don't come out even."""
    xs = list(xs)
    return [xs[i:i + n] for i in range(0, len(xs), step or n)]

def partition_by(f, xs):
    """Runs of xs where f gives the same value, each a list.  partition_by even [1 3 2 4 5] -> [[1 3] [2 4] [5]]"""
    return [list(g) for _, g in _it.groupby(xs, f)]

def interpose(sep, xs):
    """xs with sep between each two items.  interpose , [a b c] -> [a , b , c]"""
    out = []
    for i, x in enumerate(xs):
        if i:
            out.append(sep)
        out.append(x)
    return out

def interleave(*xss):
    """The first of each list, then the second of each, …, until the shortest runs out.  interleave [1 2] [a b] -> [1 a 2 b]"""
    return [x for xs in zip(*xss) for x in xs]

def distinct(xs):
    """xs without repeats, in first-seen order.  distinct [1 2 1 3] -> [1 2 3]"""
    return list(dict.fromkeys(xs))

def flatten(xs):
    """Nested lists and tuples as one list."""
    out = []
    for x in xs:
        if isinstance(x, (list, tuple)):
            out.extend(flatten(x))
        else:
            out.append(x)
    return out

def iterate(f, x):
    """x, f x, f (f x), … (endless: take some)."""
    while True:
        yield x
        x = f(x)

def repeatedly(f, n=None):
    """f called n times (endless without n)."""
    if n is not None:
        return [f() for _ in range(n)]
    return (f() for _ in _it.count())

def zipmap(keys, vals):
    """A dict from keys to vals, side by side.  zipmap [a b] [1 2] -> {a: 1 b: 2}"""
    return dict(zip(keys, vals))

def select_keys(d, keys):
    """A dict with only those keys of d that it has.  select_keys {a: 1 b: 2} [a] -> {a: 1}"""
    return {k: d[k] for k in keys if k in d}

def merge_with(f, *ds):
    """The dicts merged; a key in more than one gets f of their values.  merge_with + {a: 1} {a: 2} -> {a: 3}"""
    out = {}
    for d in ds:
        for k, v in d.items():
            out[k] = f(out[k], v) if k in out else v
    return out

def get_in(d, path, default=None):
    """d[a][b][c] for path [a b c], or default where it's missing."""
    for k in path:
        try:
            d = d[k]
        except (KeyError, IndexError, TypeError):
            return default
    return d

def assoc_in(d, path, v):
    """A copy of d with d[a][b] = v (missing dicts made)."""
    path = list(path)
    if not path:
        return v
    out = dict(d) if isinstance(d, dict) else list(d) if isinstance(d, list) else {}
    k = path[0]
    inner = out[k] if (isinstance(out, dict) and k in out) or (isinstance(out, list) and -len(out) <= k < len(out)) else {}
    out[k] = assoc_in(inner, path[1:], v)
    return out

def update_in(d, path, f, *args):
    """A copy of d with d[a][b] = f d[a][b] args…"""
    return assoc_in(d, path, f(get_in(d, path), *args))
"#;

/// Clojure-style functions every namespace gets (the prelude's).
pub const CORE: &[&str] = &["partial", "identity", "constantly", "comp", "complement", "juxt", "inc", "dec", "first", "second", "last", "rest", "butlast", "take", "drop", "take_while", "drop_while", "some", "every", "keep", "mapcat", "frequencies", "group_by", "partition", "partition_all", "partition_by", "interpose", "interleave", "distinct", "flatten", "iterate", "repeatedly", "zipmap", "select_keys", "merge_with", "get_in", "assoc_in", "update_in"];

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

/// The notebook's way to show a value, in the kernel.
pub fn display_hook(py: Python<'_>) -> Option<Py<PyAny>> {
    DISPLAY.lock().unwrap().as_ref().map(|d| d.clone_ref(py))
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
            let r = pretty_text(py, &v, None)?;
            py.import("builtins")?.getattr("print")?.call1((r,))?;
        }
    }
    Ok(())
}

/// A value as pretty shows it (color: None for "when it's a terminal or notebook").
pub fn pretty_text(py: Python<'_>, v: &Bound<'_, PyAny>, color: Option<bool>) -> PyResult<String> {
    let kw = PyDict::new(py);
    kw.set_item("color", color)?;
    prelude(py)?.getattr("pretty")?.call((v,), Some(&kw))?.extract()
}

/// In the kernel: values go to notebook cards, which show color.
pub fn set_notebook(py: Python<'_>) -> PyResult<()> {
    prelude(py)?.setattr("_notebook", true)
}

/// A value as a notebook card folds it (JSON), if it's one that folds.
pub fn tree_json(py: Python<'_>, v: &Bound<'_, PyAny>) -> Option<String> {
    let p = prelude(py).ok()?;
    if !p.getattr("_kind").ok()?.call1((v,)).ok()?.is_truthy().ok()? {
        return None;
    }
    let t = p.getattr("tree").ok()?.call1((v,)).ok()?;
    py.import("json").ok()?.call_method1("dumps", (t,)).ok()?.extract().ok()
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

/// A word for a POSIX shell (and fish): quoted unless plain.
fn sh_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./=:@%+,".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// Run a chain of programs connected by pipes.
fn run_procs(py: Python<'_>, cmds: &[Stage], data: Option<Py<PyAny>>, capture: bool, bg: bool) -> PyResult<Py<PyAny>> {
    flush_python(py);
    if let Some(hook) = EXEC_HOOK.get() {
        hook(exec_record(cmds));
    }
    // (No terminal here: programs whose output is shown run in the one there
    // is, so whichever is a terminal app gets it.)
    if let Some(hook) = TTY_HOOK.get() {
        if !capture && !bg && data.is_none() && cmds.iter().all(|c| c.redirs.is_empty()) {
            // (Variables set or unset here since: given to each program.)
            let mut envs: Vec<String> = vec![];
            if let Some(base) = HOOK_ENV.get() {
                let now: std::collections::HashMap<String, String> = std::env::vars().collect();
                let mut set: Vec<_> = now.iter().filter(|(k, v)| base.get(*k) != Some(*v)).collect();
                set.sort();
                for k in base.keys().filter(|k| !now.contains_key(*k)) {
                    envs.push(format!("-u {}", sh_quote(k)));
                }
                for (k, v) in set {
                    envs.push(sh_quote(&format!("{k}={v}")));
                }
            }
            let pre = if envs.is_empty() { String::new() } else { format!("env {} ", envs.join(" ")) };
            let line = cmds.iter().map(|c| pre.clone() + &c.words.iter().map(|w| sh_quote(w)).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join(" | ");
            let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(code) = hook(py, &line, &cwd)? {
                STATUS.store(code, Ordering::SeqCst);
                return output(py, "", code, true);
            }
        }
    }
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
        // (What tessh ignores, Ctrl+Z, Ctrl+C at the prompt, SIGPIPE, the
        // program mustn't: ignored signals carry across exec.)
        unsafe {
            command.pre_exec(|| {
                for s in [libc::SIGTSTP, libc::SIGTTOU, libc::SIGTTIN, libc::SIGINT, libc::SIGQUIT, libc::SIGPIPE, libc::SIGXFSZ] {
                    libc::signal(s, libc::SIG_DFL);
                }
                Ok(())
            });
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
        let reader = std::thread::spawn(move || {
            let mut buf = vec![];
            let _ = r.read_to_end(&mut buf);
            buf
        });
        let code = wait_all(py, children)?;
        let buf = py.detach(|| reader.join().unwrap_or_default());
        STATUS.store(code, Ordering::SeqCst);
        let mut text = decode(py, &buf)?;
        if pass {
            let sys = py.import("sys")?;
            sys.getattr("stdout")?.call_method1("write", (text.as_str(),))?;
            flush_python(py);
            return output(py, "", code, true);
        }
        if text.ends_with('\n') {
            text.pop();
        }
        return output(py, &text, code, false);
    }
    let code = if job_control {
        let words = cmds[0].words.join(" ");
        py.detach(|| wait_foreground(children, pgid, words))
    } else {
        wait_all(py, children)?
    };
    STATUS.store(code, Ordering::SeqCst);
    py.check_signals()?;
    if code == 130 && job_control {
        return Err(pyo3::exceptions::PyKeyboardInterrupt::new_err(""));
    }
    output(py, "", code, true)
}

/// Wait for the programs, letting Ctrl+C (a KeyboardInterrupt) through: they
/// get SIGINT, then SIGKILL if they linger.
fn wait_all(py: Python<'_>, mut children: Vec<Child>) -> PyResult<i32> {
    let mut code = 0;
    loop {
        let done = py.detach(|| {
            let mut all = true;
            for ch in children.iter_mut() {
                match ch.try_wait() {
                    Ok(Some(_)) => {}
                    Ok(None) => all = false,
                    Err(_) => {}
                }
            }
            if !all {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            all
        });
        if done {
            for ch in children.iter_mut() {
                code = ch.wait().map(exit_code).unwrap_or(1);
            }
            return Ok(code);
        }
        if let Err(e) = py.check_signals() {
            for ch in children.iter() {
                unsafe { libc::kill(ch.id() as i32, libc::SIGINT) };
            }
            py.detach(|| {
                let start = std::time::Instant::now();
                while start.elapsed() < std::time::Duration::from_millis(500) {
                    if children.iter_mut().all(|c| matches!(c.try_wait(), Ok(Some(_)))) {
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                for c in children.iter_mut() {
                    let _ = c.kill();
                    let _ = c.wait();
                }
            });
            STATUS.store(130, Ordering::SeqCst);
            return Err(e);
        }
    }
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
    go(py, d, false).map(|_| ())
}

/// The folder as the shell names it: $PWD while it's still where we are
/// (it keeps the symlinks you went through), else the real path.
fn logical_pwd() -> String {
    use std::os::unix::fs::MetadataExt;
    let real = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    match std::env::var("PWD") {
        Ok(p) if p.starts_with('/') => {
            let same = |a: &str, b: &str| match (std::fs::metadata(a), std::fs::metadata(b)) {
                (Ok(x), Ok(y)) => x.dev() == y.dev() && x.ino() == y.ino(),
                _ => false,
            };
            if same(&p, ".") { p } else { real }
        }
        _ => real,
    }
}

/// `a/./b/../c` as `a/c`, by the names alone (as cd -L reads them).
fn tidy_path(p: &str) -> String {
    let mut parts: Vec<&str> = vec![];
    for x in p.split('/') {
        match x {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            x => parts.push(x),
        }
    }
    format!("/{}", parts.join("/"))
}

/// Go to d: by the path as written (-L, the default, as bash does) or the
/// real one (-P, symlinks resolved). Sets $OLDPWD and $PWD; the new $PWD.
fn go(py: Python<'_>, d: &str, physical: bool) -> PyResult<String> {
    let old = logical_pwd();
    let fail = |e: std::io::Error| {
        let msg = e.to_string();
        let msg = msg.split(" (os error").next().unwrap_or(&msg).to_string();
        PyOSError::new_err(format!("cd: {d}: {msg}"))
    };
    let new = if physical {
        std::env::set_current_dir(d).map_err(fail)?;
        std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
    } else {
        let want = tidy_path(&if d.starts_with('/') { d.to_string() } else { format!("{old}/{d}") });
        // (Where the names don't lead anywhere, the path as the system reads it.)
        match std::env::set_current_dir(&want) {
            Ok(()) => want,
            Err(_) => {
                std::env::set_current_dir(d).map_err(fail)?;
                std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default()
            }
        }
    };
    *OLDPWD.lock().unwrap() = Some(old.clone());
    setenv(py, "OLDPWD", Some(&old))?;
    setenv(py, "PWD", Some(&new))?;
    Ok(new)
}

/// cd's target: d, or where $CDPATH finds it (and whether to say so).
fn cdpath(d: &str) -> (String, bool) {
    let rel = !(d.starts_with('/') || d == "." || d == ".." || d.starts_with("./") || d.starts_with("../"));
    if let (true, Ok(cp)) = (rel, std::env::var("CDPATH")) {
        for base in cp.split(':') {
            let cand = if base.is_empty() { format!("./{d}") } else { format!("{}/{d}", base.trim_end_matches('/')) };
            if std::path::Path::new(&cand).is_dir() {
                return (cand, !base.is_empty() && base != ".");
            }
        }
    }
    (d.to_string(), false)
}

/// The folder stack, as dirs shows it: the current folder, then the stack
/// from its top.
fn dir_list() -> Vec<String> {
    let mut v = vec![logical_pwd()];
    v.extend(DIRSTACK.lock().unwrap().iter().rev().cloned());
    v
}

fn set_dir_list(v: &[String]) {
    *DIRSTACK.lock().unwrap() = v.iter().skip(1).rev().cloned().collect();
}

/// +N (from the left, the current folder 0) or -N (from the right) as an index.
fn stack_index(cmd: &str, a: &str, len: usize) -> PyResult<Option<usize>> {
    let (sign, n) = a.split_at(1);
    if !(sign == "+" || sign == "-") || n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(None);
    }
    let n: usize = n.parse().unwrap_or(usize::MAX);
    let i = if sign == "+" { Some(n).filter(|n| *n < len) } else { len.checked_sub(n + 1) };
    i.map(Some).ok_or_else(|| PyIndexError::new_err(format!("{cmd}: {a}: directory stack index out of range")))
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

pub const COMMAND_BUILTINS: &[&str] = &["cd", "pushd", "popd", "dirs", "export", "unset", "exit", "source", "which", "glob", "jobs", "fg", "bg", "need"];

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
    /// Its usage, for help and the completion panel.
    #[getter]
    fn _tessh_usage(&self) -> &'static str {
        match self.name {
            "cd" => "cd [-L|-P [-e]] [--] [dir]\n\nGo to dir: home without one, the last folder for - (and says where), ~ and ~user for homes, and $CDPATH's folders for a name that isn't there. -L (the default) keeps the path as written, symlinks and all; -P goes by the real folders. The folder is this shell's, and every program it runs starts there.",
            "pushd" => "pushd [-n] [dir | +N | -N]\n\nGo to dir, keeping the folder you were in on the stack (dirs lists it, popd goes back). Alone, swap the top two; +N/-N turn the stack so its Nth folder (from the left or right of dirs) is on top. -n changes the stack only.",
            "popd" => "popd [-n] [+N | -N]\n\nBack to the folder on top of the stack (pushd put it there). +N/-N drop the Nth folder (from the left or right of dirs) instead; -n changes the stack only.",
            "dirs" => "dirs [-clpv] [+N | -N]\n\nThe folder stack, the current folder first. -c clears it, -l spells out ~, -p puts one folder a line, -v numbers them, +N/-N shows just that one.",
            "export" => "export NAME=value …\n\nSet environment variables for this shell and every program it runs. export NAME alone exports the tessh variable $NAME's value.",
            "unset" => "unset NAME …\n\nRemove environment variables.",
            "exit" => "exit [status]\n\nEnd tessh (a script, or the REPL) with status, 0 without one.",
            "source" => "source file\n\nRun a tessh file here: its names and folder changes stay in this shell.",
            "which" => "which name …\n\nWhere each program is on PATH (name: path, or name: not found); its value is the first path found.",
            "glob" => "glob pattern …\n\nThe names matching the patterns (*, ?, […]), sorted, as a list.",
            "jobs" => "jobs\n\nThe background jobs (commands run with a trailing &): [n] running or stopped, and the command.",
            "fg" => "fg [n]\n\nBring background job n (the latest without one) to the foreground and wait for it.",
            "bg" => "bg [n]\n\nLet a stopped job n (the latest without one) run on in the background.",
            "need" => "need package \"package>=version\" …\n\nPyPI packages this script, cell or REPL uses: uv installs them once into a cache, and they import from the next line. Quote a version range, so its > isn't a redirection.",
            _ => "A tessh builtin.",
        }
    }

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
            // need pkg "pkg>=1" …: PyPI packages, installed with uv (once, kept
            // in a cache) and importable from the next line.
            "need" => {
                if words.is_empty() {
                    return Err(PyTypeError::new_err("need package…"));
                }
                crate::deps::ensure(py, &words, None)?;
                Ok(none)
            }
            // cd [-L|-P [-e]] [--] [dir|-]: bash's.
            "cd" => {
                let usage = "cd: usage: cd [-L|[-P [-e]]] [-@] [dir]";
                let mut physical = false;
                let mut rest = &words[..];
                while let Some(a) = rest.first() {
                    if a == "--" {
                        rest = &rest[1..];
                        break;
                    }
                    if !a.starts_with('-') || a == "-" {
                        break;
                    }
                    for c in a[1..].chars() {
                        match c {
                            'L' => physical = false,
                            'P' => physical = true,
                            'e' => {}
                            '@' => return Err(PyValueError::new_err(format!("cd: -@: extended attributes aren't on this system\n{usage}"))),
                            c => return Err(PyValueError::new_err(format!("cd: -{c}: invalid option\n{usage}"))),
                        }
                    }
                    rest = &rest[1..];
                }
                if rest.len() > 1 {
                    return Err(PyTypeError::new_err("cd: too many arguments"));
                }
                let (d, say) = match rest.first().map(String::as_str) {
                    None => (std::env::var("HOME").map_err(|_| PyOSError::new_err("cd: HOME not set"))?, false),
                    Some("-") => {
                        let d = OLDPWD.lock().unwrap().clone().or_else(|| std::env::var("OLDPWD").ok());
                        (d.ok_or_else(|| PyOSError::new_err("cd: OLDPWD not set"))?, true)
                    }
                    Some("") => return Ok(none),
                    Some(d) => cdpath(&expanduser(d)),
                };
                let new = go(py, &expanduser(&d), physical)?;
                if say {
                    py_print(py, &new)?;
                }
                Ok(none)
            }
            // pushd [-n] [dir|+N|-N], popd [-n] [+N|-N], dirs [-clpv] [+N|-N]: bash's.
            "pushd" | "popd" => {
                let cmd = self.name;
                let mut stay = false;
                let mut arg: Option<&String> = None;
                for a in &words {
                    if a == "-n" {
                        stay = true;
                    } else if a == "--" {
                    } else if arg.is_none() {
                        arg = Some(a);
                    } else {
                        return Err(PyTypeError::new_err(format!("{cmd}: too many arguments")));
                    }
                }
                let mut v = dir_list();
                let len = v.len();
                let at = match arg {
                    Some(a) => stack_index(cmd, a, len)?,
                    None => None,
                };
                if cmd == "popd" {
                    if len < 2 {
                        return Err(PyIndexError::new_err("popd: directory stack empty"));
                    }
                    if let (Some(a), None) = (arg, at) {
                        return Err(PyValueError::new_err(format!("popd: {a}: invalid argument\npopd: usage: popd [-n] [+N | -N]")));
                    }
                    let i = at.unwrap_or(0);
                    if i == 0 && !stay {
                        go(py, &v[1], false)?;
                        v.remove(0);
                        v[0] = logical_pwd();
                    } else {
                        v.remove(if i == 0 { 1 } else { i });
                    }
                } else {
                    match (arg, at) {
                        (None, _) => {
                            if len < 2 {
                                return Err(PyIndexError::new_err("pushd: no other directory"));
                            }
                            v.swap(0, 1);
                        }
                        (Some(_), Some(i)) => {
                            if stay {
                                v[1..].rotate_left(i.saturating_sub(1));
                            } else {
                                v.rotate_left(i);
                            }
                        }
                        (Some(d), None) => {
                            let (d, _) = cdpath(&expanduser(d));
                            v.insert(if stay { 1 } else { 0 }, d);
                        }
                    }
                    if !stay && v[0] != logical_pwd() {
                        let new = go(py, &v[0], false)?;
                        v[0] = new;
                    }
                }
                set_dir_list(&v);
                print_dirs(py, &[])?;
                Ok(none)
            }
            "dirs" => {
                print_dirs(py, &words)?;
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
                        Some(p) => py_print(py, &format!("{n}: {p}"))?,
                        None => py_print(py, &format!("{n}: not found"))?,
                    }
                    found = found.or(p);
                }
                output(py, found.as_deref().unwrap_or(""), if found.is_some() { 0 } else { 1 }, false)
            }
            "glob" => Ok(PyList::new(py, words)?.into_any().unbind()),
            "jobs" => {
                let mut jobs = JOBS.lock().unwrap();
                jobs.retain_mut(|j| j.stopped || !job_done(j, py));
                let lines: Vec<String> = jobs.iter().map(|j| format!("[{}] {} {}", j.n, if j.stopped { "stopped" } else { "running" }, j.words)).collect();
                drop(jobs);
                for l in lines {
                    py_print(py, &l)?;
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

fn print_dirs(py: Python<'_>, opts: &[String]) -> PyResult<()> {
    let (mut long, mut lines, mut numbered) = (false, false, false);
    let mut only: Option<String> = None;
    let v = dir_list();
    for a in opts {
        if let Some(i) = stack_index("dirs", a, v.len())? {
            only = Some(v[i].clone());
            continue;
        }
        if !a.starts_with('-') || a.len() < 2 {
            return Err(PyValueError::new_err(format!("dirs: {a}: invalid argument\ndirs: usage: dirs [-clpv] [+N] [-N]")));
        }
        for c in a[1..].chars() {
            match c {
                'c' => {
                    DIRSTACK.lock().unwrap().clear();
                    return Ok(());
                }
                'l' => long = true,
                'p' => lines = true,
                'v' => {
                    lines = true;
                    numbered = true
                }
                c => return Err(PyValueError::new_err(format!("dirs: -{c}: invalid option\ndirs: usage: dirs [-clpv] [+N] [-N]"))),
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    let show = |d: &str| match d.strip_prefix(home.as_str()) {
        Some(t) if !long && !home.is_empty() && (t.is_empty() || t.starts_with('/')) => format!("~{t}"),
        _ => d.to_string(),
    };
    if let Some(d) = only {
        return py_print(py, &show(&d));
    }
    let shown: Vec<String> = v.iter().enumerate().map(|(i, d)| if numbered { format!("{i:2}  {}", show(d)) } else { show(d) }).collect();
    py_print(py, &shown.join(if lines { "\n" } else { " " }))
}

/// A builtin's output, through Python's print: redirections and the
/// notebook see it.
fn py_print(py: Python<'_>, s: &str) -> PyResult<()> {
    py.import("builtins")?.getattr("print")?.call1((s,))?;
    Ok(())
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
    m.add("pick", prelude(py)?.getattr("pick")?)?;
    for n in CORE {
        m.add(*n, prelude(py)?.getattr(*n)?)?;
    }
    m.add("print", prelude(py)?.getattr("print")?)?;
    m.add("pretty", prelude(py)?.getattr("pretty")?)?;
    m.add("env", Py::new(py, Env)?)?;
    for n in COMMAND_BUILTINS {
        m.add(*n, Py::new(py, Builtin { name: n })?)?;
    }
    crate::help::add(&m)?;
    py.import("sys")?.getattr("modules")?.set_item("__tessh__", &m)?;
    Ok(m)
}

/// The names every tessh namespace gets.
pub fn install(py: Python<'_>, ns: &Bound<'_, PyDict>) -> PyResult<()> {
    let m = py.import("__tessh__")?;
    for n in COMMAND_BUILTINS.iter().chain(["command", "env", "Output", "help", "print", "pretty"].iter()).chain(CORE.iter()) {
        if !ns.contains(*n)? {
            ns.set_item(*n, m.getattr(*n)?)?;
        }
    }
    let _ = PyBytes::new(py, b"");
    Ok(())
}
