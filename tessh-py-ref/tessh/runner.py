"""Running tessh: a namespace with tessh's builtins, and source run form by
form (each top-level form compiled, then run, so macros and names defined
earlier are there for the next)."""

from __future__ import annotations

import ast
import builtins
import os
import sys
import types

from . import forms, runtime
from .compiler import CompileError, Compiler
from .reader import ReadError, read

MACRO_HELPERS = {
    "var": forms.var, "sym": forms.sym, "gensym": forms.gensym, "capture": forms.capture,
    "lift": forms.lift,
}


def new_namespace(name="__main__", argv=None) -> dict:
    ns = {"__name__": name, "__builtins__": builtins}
    install(ns, argv)
    return ns


def install(ns: dict, argv=None):
    """Put tessh's names in a namespace (a fresh one, or IPython's)."""
    ns.setdefault("__tessh__", runtime)
    ns.setdefault("__tessh_forms__", forms)
    for k, v in runtime.BUILTINS.items():
        ns.setdefault(k, v)
    for k, v in MACRO_HELPERS.items():
        ns.setdefault(k, v)
    ns.setdefault("argv", list(argv or []))
    if "__tessh_compiler__" not in ns:
        ns["__tessh_compiler__"] = Compiler(ns)
    comp = ns["__tessh_compiler__"]

    def macroexpand(form):
        """The form a macro call expands to (one step)."""
        if isinstance(form, forms.Line) and form.items and isinstance(form.items[0], forms.Sym) and form.items[0].name in comp.macros:
            m = comp.macros[form.items[0].name]
            return m.fn(*comp.macro_args(m, form.items[1:], form.children, form.items[0]))
        return form

    ns.setdefault("macroexpand", macroexpand)
    return ns


def compiler_of(ns) -> Compiler:
    return install(ns)["__tessh_compiler__"]


def run_source(text, ns, filename="<tessh>", show=False, first_line=1):
    """Run tessh source in ns. With show, the last form's value is shown (the
    REPL, the kernel). Returns the value of the last form."""
    from .deps import ensure_from_source
    ensure_from_source(text)
    comp = compiler_of(ns)
    lines = read(text, first_line=first_line)
    comp.prescan(lines, comp.scope)
    units = comp.units(lines)
    for k, u in enumerate(units):
        last = k == len(units) - 1
        mod = comp.compile_unit_module(u, "tail" if (show and last) else "stmt", show=show and last)
        code = compile(mod, filename, "exec")
        exec(code, ns)


def run_file(path, ns=None, argv=None, fresh=True):
    with open(path, encoding="utf-8") as f:
        text = f.read()
    if text.startswith("#!"):
        text = "#" + text[1:]
    if ns is None:
        ns = new_namespace("__main__", [path] + list(argv or []))
        ns["__file__"] = path
    run_source(text, ns, filename=path)
    return ns


def to_python(text) -> str:
    comp = Compiler(new_namespace(), check=True)
    comp.ns = None
    mod = comp.compile_module(read(text))
    return ast.unparse(mod)


def check(text, filename="<tessh>"):
    """Problems in tessh source, without running it (macro definitions run):
    [(line, col, end_col, message)]."""
    try:
        lines = read(text)
    except ReadError as e:
        return [(e.line, e.col, e.end_col, e.msg)]
    comp = Compiler(None, check=True)
    try:
        mod = comp.compile_module(lines)
    except CompileError as e:
        return [(e.line, e.col, e.end_col, e.msg)]
    except ReadError as e:
        return [(e.line, e.col, e.end_col, e.msg)]
    try:
        compile(mod, filename, "exec")
    except SyntaxError as e:
        line = e.lineno or 1
        col = (e.offset or 1) - 1
        return [(line, col, col + 1, e.msg)]
    return []


def error_position(e):
    """(line, col, end_col, message) of a tessh error, else None."""
    if isinstance(e, (ReadError, CompileError)):
        return (e.line, e.col, e.end_col, e.msg)
    return None
