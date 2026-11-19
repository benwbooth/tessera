"""The forms the reader makes and macros take and return.

Every form is immutable and carries `pos`: (line, col, end_line, end_col),
lines from 1 and columns from 0, or None for forms a macro made up.
"""

from __future__ import annotations

import itertools
from dataclasses import dataclass, field, fields, replace


@dataclass(frozen=True, eq=False)
class Form:
    pos: tuple | None = field(default=None, kw_only=True, repr=False, compare=False)

    def __repr__(self):
        return f"<{type(self).__name__} {show(self)}>"


@dataclass(frozen=True, eq=False, repr=False)
class Sym(Form):
    """A bareword. Where it means a name, `marks` are its hygiene marks."""
    name: str
    marks: frozenset = frozenset()

    def __eq__(self, other):
        return isinstance(other, Sym) and other.name == self.name and other.marks == self.marks

    def __hash__(self):
        return hash((self.name, self.marks))


@dataclass(frozen=True, eq=False, repr=False)
class Var(Form):
    """$name (sym is a Sym, or an Unquote inside a quasiquote)."""
    sym: Form


@dataclass(frozen=True, eq=False, repr=False)
class Status(Form):
    """$?"""


@dataclass(frozen=True, eq=False, repr=False)
class Num(Form):
    value: object
    text: str


@dataclass(frozen=True, eq=False, repr=False)
class Str(Form):
    """A string with nothing to interpolate. kind: '"', "'", 'b', 'r'."""
    value: str | bytes
    kind: str = '"'


@dataclass(frozen=True, eq=False, repr=False)
class Interp(Form):
    """$name, $(form), ${expr!conv:spec} inside a string or word."""
    expr: Form
    conv: str | None = None
    spec: str | None = None
    braced: bool = False


@dataclass(frozen=True, eq=False, repr=False)
class IStr(Form):
    """A double-quoted string with interpolations: parts are str | Interp."""
    parts: tuple


@dataclass(frozen=True, eq=False, repr=False)
class Word(Form):
    """Glued pieces: str (bareword text, globbed and ~-expanded in commands)
    or forms (Str, IStr, Var, Attr, Index, Group, Interp, Status)."""
    parts: tuple


@dataclass(frozen=True, eq=False, repr=False)
class Op(Form):
    """A spaced operator, or `name` (tick) for a function used infix."""
    name: str
    tick: bool = False


@dataclass(frozen=True, eq=False, repr=False)
class Redir(Form):
    """A redirection glued to its file (>out.txt), or whole (2>&1)."""
    op: str
    target: Form | None = None


@dataclass(frozen=True, eq=False, repr=False)
class KwArg(Form):
    name: str
    value: Form


@dataclass(frozen=True, eq=False, repr=False)
class Splat(Form):
    value: Form
    n: int = 1


@dataclass(frozen=True, eq=False, repr=False)
class Caret(Form):
    """^word: a program even if a Python name has its name."""
    word: Form


@dataclass(frozen=True, eq=False, repr=False)
class SymLit(Form):
    """:name"""
    sym: Sym


@dataclass(frozen=True, eq=False, repr=False)
class Quasi(Form):
    form: Form


@dataclass(frozen=True, eq=False, repr=False)
class Unquote(Form):
    form: Form
    splice: bool = False


@dataclass(frozen=True, eq=False, repr=False)
class Attr(Form):
    obj: Form
    name: str


@dataclass(frozen=True, eq=False, repr=False)
class Index(Form):
    obj: Form
    items: tuple


@dataclass(frozen=True, eq=False, repr=False)
class Unary(Form):
    """-$x, ~$x, -(…): a prefix operator glued to its operand."""
    op: str
    operand: Form


@dataclass(frozen=True, eq=False, repr=False)
class Group(Form):
    """( … ), [ … ] or { … }: kind is the opening bracket."""
    kind: str
    items: tuple


@dataclass(frozen=True, eq=False, repr=False)
class Comma(Form):
    pass


@dataclass(frozen=True, eq=False, repr=False)
class Semi(Form):
    pass


@dataclass(frozen=True, eq=False, repr=False)
class Line(Form):
    """A line: its items, and the lines indented under it."""
    items: tuple
    children: tuple = ()


# ---- building forms (for macros) ---------------------------------------------

_gensyms = itertools.count(1)


def sym(name: str) -> Sym:
    return Sym(name)


def var(s) -> Var:
    return Var(as_sym(s))


def as_sym(x) -> Sym:
    if isinstance(x, Sym):
        return x
    if isinstance(x, Var):
        return as_sym(x.sym)
    if isinstance(x, str):
        return Sym(x)
    if isinstance(x, Line) and len(x.items) == 1 and not x.children:
        return as_sym(x.items[0])
    raise TypeError(f"not a symbol: {show(x) if isinstance(x, Form) else x!r}")


def gensym(hint: str = "g") -> Sym:
    return Sym(f"_g{next(_gensyms)}_{hint}")


def lift(v) -> Form:
    """A macro-time value as a form."""
    if isinstance(v, Form):
        return v
    if isinstance(v, bool) or v is None:
        return Sym(repr(v))
    if isinstance(v, (int, float, complex)):
        return Num(v, repr(v))
    if isinstance(v, (str, bytes)):
        return Str(v, "b" if isinstance(v, bytes) else '"')
    if isinstance(v, list):
        return Group("[", tuple(lift(x) for x in v))
    if isinstance(v, tuple):
        items = []
        for x in v:
            items += [lift(x), Comma()]
        return Group("(", tuple(items))
    if isinstance(v, dict):
        items = []
        for k, x in v.items():
            items += [Word((lift(k), ":")), lift(x)]
        return Group("{", tuple(items))
    raise TypeError(f"can't make a form of {type(v).__name__}")


def lift_many(v) -> list:
    if isinstance(v, Form):
        if isinstance(v, Group) and v.kind == "[":
            return list(v.items)
        return [v]
    return [lift(x) for x in v]


def map_forms(f, form):
    """Apply f to every form inside form (bottom up), keeping the rest."""
    if isinstance(form, (list, tuple)):
        return type(form)(map_forms(f, x) for x in form)
    if not isinstance(form, Form):
        return form
    changes = {}
    for fl in fields(form):
        if fl.name == "pos":
            continue
        v = getattr(form, fl.name)
        if isinstance(v, (Form, tuple)):
            nv = map_forms(f, v)
            if nv is not v:
                changes[fl.name] = nv
    if changes:
        form = replace(form, **changes)
    return f(form)


def toggle_mark(form, mark):
    def f(x):
        if isinstance(x, Sym):
            return replace(x, marks=x.marks ^ {mark})
        return x
    return map_forms(f, form)


def strip_marks(form):
    return map_forms(lambda x: replace(x, marks=frozenset()) if isinstance(x, Sym) and x.marks else x, form)


# ---- showing forms as tessh ---------------------------------------------------

def show(form, indent: int = 0) -> str:
    if isinstance(form, (list, tuple)):
        return "\n".join(show(x, indent) for x in form)
    s = _show(form)
    if isinstance(form, Line):
        s = " " * indent + s
        for c in form.children:
            s += "\n" + show(c, indent + 2)
    return s


def _show_part(p):
    return p if isinstance(p, str) else _show(p)


def _show(f) -> str:
    if isinstance(f, Line):
        return " ".join(_show(i) for i in f.items)
    if isinstance(f, Sym):
        return f.name
    if isinstance(f, Var):
        return "$" + _show(f.sym)
    if isinstance(f, Status):
        return "$?"
    if isinstance(f, Num):
        return f.text
    if isinstance(f, Str):
        if f.kind == "b":
            return "b" + repr(f.value)[1:]
        if f.kind in ("'", "r") and "'" not in f.value:
            return f"'{f.value}'"
        return '"' + f.value.replace("\\", "\\\\").replace('"', '\\"').replace("$", "\\$").replace("\n", "\\n") + '"'
    if isinstance(f, IStr):
        out = ""
        for p in f.parts:
            if isinstance(p, str):
                out += p.replace("\\", "\\\\").replace('"', '\\"').replace("$", "\\$").replace("\n", "\\n")
            else:
                out += _show(p)
        return '"' + out + '"'
    if isinstance(f, Interp):
        if not isinstance(f.expr, Group):
            return _show(f.expr)
        inner = _show(f.expr) if not isinstance(f.expr, Group) else " ".join(_show(i) for i in f.expr.items)
        if not f.braced:
            return "$(" + inner + ")"
        return "${" + inner + (f"!{f.conv}" if f.conv else "") + (f":{f.spec}" if f.spec else "") + "}"
    if isinstance(f, Word):
        return "".join(_show_part(p) for p in f.parts)
    if isinstance(f, Op):
        return f"`{f.name}`" if f.tick else f.name
    if isinstance(f, Redir):
        return f.op + (_show(f.target) if f.target is not None else "")
    if isinstance(f, KwArg):
        return f"{f.name}={_show(f.value)}"
    if isinstance(f, Splat):
        return "*" * f.n + _show(f.value)
    if isinstance(f, Caret):
        return "^" + _show(f.word)
    if isinstance(f, SymLit):
        return ":" + f.sym.name
    if isinstance(f, Quasi):
        return ":" + _show(f.form) if isinstance(f.form, Group) else "quasi " + _show(f.form)
    if isinstance(f, Unquote):
        return ("~@" if f.splice else "~") + _show(f.form)
    if isinstance(f, Attr):
        return f"{_show(f.obj)}.{f.name}"
    if isinstance(f, Index):
        return f"{_show(f.obj)}[{' '.join(_show(i) for i in f.items)}]"
    if isinstance(f, Unary):
        return f.op + _show(f.operand)
    if isinstance(f, Group):
        close = {"(": ")", "[": "]", "{": "}"}[f.kind]
        out = ""
        for i in f.items:
            if isinstance(i, Comma):
                out += ","
            elif isinstance(i, Semi):
                out += ";"
            elif isinstance(i, Line):
                out += (" " if out else "") + "(" + show(i) + ")"
            else:
                out += (" " if out else "") + _show(i)
        return f.kind + out + close
    if isinstance(f, Comma):
        return ","
    if isinstance(f, Semi):
        return ";"
    return repr(f)


def as_lines(v) -> list:
    """Forms (or a list of them) as Lines, for splicing into a block."""
    if isinstance(v, Form):
        v = [v]
    out = []
    for x in v:
        if isinstance(x, Line):
            out.append(x)
        elif isinstance(x, Form):
            out.append(Line((x,), pos=x.pos))
        else:
            out.append(Line((lift(x),)))
    return out


# The head symbol of the macro call being expanded (for capture).
current_head: list = [None]


def capture(name: str, like=None) -> Sym:
    """A symbol in the context of `like` (by default, the macro call's): on
    purpose unhygienic, for anaphoric macros."""
    ref = like if like is not None else current_head[0]
    return Sym(name, ref.marks if isinstance(ref, Sym) else frozenset())
