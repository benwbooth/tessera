"""tessh forms to Python's AST.

Everything is an expression: each compile step gives a Res, the statements
that must run first and the expression for the value. Commands compile to
calls into the runtime (`__tessh__`), with how they run (attached to the
terminal, or their output captured) decided by where they are.
"""

from __future__ import annotations

import ast
import builtins
import itertools
import re
from contextlib import contextmanager

from . import forms as F
from .forms import (Attr, Caret, Comma, Form, Group, Index, Interp, IStr, KwArg, Line, Num, Op,
                    Quasi, Redir, Semi, Splat, Status, Str, Sym, SymLit, Unary, Unquote, Var, Word)

RT = "__tessh__"
FM = "__tessh_forms__"
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*\Z")
DOTTED = re.compile(r"[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*\Z")
GLOB_CHARS = set("*?[")


class CompileError(SyntaxError):
    def __init__(self, msg, form=None):
        super().__init__(msg)
        self.msg = msg
        pos = getattr(form, "pos", None) if form is not None else None
        if pos and len(pos) == 4:
            self.line, self.col, self.end_line, self.end_col = pos
        else:
            self.line, self.col, self.end_line, self.end_col = 1, 0, 1, 1
        if self.end_line != self.line:
            self.end_col = self.col + 1

    def __str__(self):
        return f"{self.line}:{self.col + 1}: {self.msg}"


ASSIGN_OPS = {"=": None, "+=": ast.Add, "-=": ast.Sub, "*=": ast.Mult, "/=": ast.Div, "//=": ast.FloorDiv,
              "%=": ast.Mod, "**=": ast.Pow, "@=": ast.MatMult, "&=": ast.BitAnd, "|=": ast.BitOr,
              "^=": ast.BitXor, "<<=": ast.LShift, ">>=": ast.RShift}
BINOPS = {"+": ast.Add, "-": ast.Sub, "*": ast.Mult, "/": ast.Div, "//": ast.FloorDiv, "%": ast.Mod,
          "@": ast.MatMult, "**": ast.Pow, "<<": ast.LShift, ">>": ast.RShift, "^": ast.BitXor,
          "&": ast.BitAnd, "|": ast.BitOr}
CMPOPS = {"==": ast.Eq, "!=": ast.NotEq, "<": ast.Lt, ">": ast.Gt, "<=": ast.LtE, ">=": ast.GtE,
          "in": ast.In, "not in": ast.NotIn, "is": ast.Is, "is not": ast.IsNot}
PREC = {":=": 0, "or": 1, "||": 1, "and": 2, "&&": 2, "|": 4, "^": 7, "&": 8, "<<": 9, ">>": 9,
        "+": 10, "-": 10, "*": 11, "/": 11, "//": 11, "%": 11, "@": 11, "**": 13}
for _c in CMPOPS:
    PREC[_c] = 5
RIGHT = {"**", ":="}
PREFIX = {"-": 12, "+": 12, "not": 3, "!": 3, "~": 12}
WORD_OPS = {"and", "or", "not", "in", "is"}
SPACED_REDIRS = {">", ">>", "<", "1>", "1>>", "2>", "2>>", "&>", "&>>"}
CONTINUATIONS = {"elif", "else", "except", "except*", "finally"}
BLOCK_SPECIALS = {"def", "class", "if", "while", "for", "try", "with", "match", "defmacro", "quasi",
                  "do", "async", "fn", "lambda"}
SPECIALS = BLOCK_SPECIALS | {"return", "yield", "await", "raise", "assert", "del", "pass", "break",
                             "continue", "global", "nonlocal", "import", "from", "quote", "defop",
                             "elif", "else", "except", "except*", "finally", "case"}
PY_BUILTINS = set(dir(builtins))


class Res:
    __slots__ = ("stmts", "expr")

    def __init__(self, stmts=(), expr=None):
        self.stmts = list(stmts)
        self.expr = expr


def const(v):
    return ast.Constant(v)


def name(n, store=False):
    return ast.Name(n, ast.Store() if store else ast.Load())


def rt(attr):
    return ast.Attribute(name(RT), attr, ast.Load())


def call(f, args=(), kws=()):
    return ast.Call(f, list(args), list(kws))


def _pure(e):
    return isinstance(e, (ast.Constant, ast.Name)) or e is None


class Scope:
    def __init__(self, kind, parent=None):
        self.kind = kind
        self.parent = parent
        self.names = {}      # (name, marks) -> python name
        self.known = set()   # names bound somewhere in this scope (prescan)
        self.commands = set()


class Macro:
    def __init__(self, name, fn, params):
        self.name = name
        self.fn = fn
        self.params = params


class Compiler:
    def __init__(self, ns=None, check=False):
        self.ns = ns
        self.check = check
        self.scope = Scope("module")
        self.macros = {}
        self.ops = {}
        self._tmp = itertools.count(1)
        self._marks = itertools.count(1)
        self._scratch = None
        self.depth = 0

    # ---- helpers --------------------------------------------------------------

    def err(self, msg, form=None):
        return CompileError(msg, form)

    def tmp(self, hint="t"):
        return f"_tessh_{hint}{next(self._tmp)}"

    def at(self, node, form):
        pos = getattr(form, "pos", None)
        if pos and len(pos) == 4 and isinstance(node, (ast.expr, ast.stmt, ast.excepthandler)):
            node.lineno, node.col_offset, node.end_lineno, node.end_col_offset = pos
        return node

    @contextmanager
    def new_scope(self, kind):
        self.scope = Scope(kind, self.scope)
        try:
            yield self.scope
        finally:
            self.scope = self.scope.parent

    def bind(self, s: Sym) -> str:
        """A binding of s in the current scope: its Python name (renamed when
        a macro made it: it has marks)."""
        if not IDENT.match(s.name):
            raise self.err(f"'{s.name}' can't be a name (names are letters, digits and _)", s)
        key = (s.name, s.marks)
        py = self.scope.names.get(key)
        if py is None:
            py = s.name if not s.marks else "_h" + "_".join(map(str, sorted(s.marks))) + "_" + s.name
            self.scope.names[key] = py
            self.scope.known.add(py)
        return py

    def _binding(self, s: Sym):
        sc = self.scope
        first = True
        while sc is not None:
            if sc.kind != "class" or first:
                py = sc.names.get((s.name, s.marks))
                if py is not None:
                    return py
            first = False
            sc = sc.parent
        return None

    def _user_shadows(self, n):
        """Does a local (not module) binding of n hide the module's n?"""
        sc = self.scope
        while sc is not None and sc.kind != "module":
            if (n, frozenset()) in sc.names or n in sc.known:
                return True
            sc = sc.parent
        return False

    def is_known(self, s: Sym) -> bool:
        """Is s (its first dotted part) a Python name the compiler knows?"""
        root = s.name.split(".")[0]
        if not IDENT.match(root):
            return False
        if self._binding(Sym(root, s.marks)) is not None:
            return True
        sc = self.scope
        while sc is not None:
            if root in sc.commands:
                return False
            if root in sc.known:
                return True
            sc = sc.parent
        if self.ns is not None and root in self.ns:
            return not getattr(self.ns[root], "_tessh_command", False)
        if root in (RT, FM):
            return True
        from .runtime import BUILTINS
        if root in BUILTINS:
            return not getattr(BUILTINS[root], "_tessh_command", False)
        return root in PY_BUILTINS

    def resolve(self, s: Sym, form=None):
        py = self._binding(s)
        if py is not None:
            return self.at(name(py), form or s)
        if s.marks and self._user_shadows(s.name):
            return call(rt("free"), [call(name("globals")), const(s.name)])
        return self.at(name(s.name), form or s)

    def name_load(self, s: Sym, form=None):
        parts = s.name.split(".")
        if not all(IDENT.match(p) for p in parts):
            raise self.err(f"'{s.name}' isn't a name", form or s)
        e = self.resolve(Sym(parts[0], s.marks, pos=s.pos), form)
        for p in parts[1:]:
            e = ast.Attribute(e, p, ast.Load())
        return e

    def gather(self, results):
        """Statements and expressions of several results, in order: earlier
        values are kept in temporaries when a later one needs statements."""
        stmts, exprs = [], []
        for r in results:
            if r.stmts:
                for i, e in enumerate(exprs):
                    if not _pure(e):
                        t = self.tmp()
                        stmts.append(ast.Assign([name(t, True)], e))
                        exprs[i] = name(t)
                stmts += r.stmts
            exprs.append(r.expr)
        return stmts, exprs

    def as_stmts(self, r: Res):
        out = list(r.stmts)
        if r.expr is not None and not _pure(r.expr):
            out.append(ast.Expr(r.expr))
        return out

    # ---- prescan --------------------------------------------------------------

    def prescan(self, lines, scope):
        prev_deco = []
        for l in lines:
            items = l.items
            if not items:
                continue
            head = items[0]
            hn = head.name if isinstance(head, Sym) else None
            tgt = self._assign_index(items)
            if tgt is not None:
                for t in items[:tgt]:
                    self._target_names(t, scope.known)
            elif hn and hn.startswith("@"):
                prev_deco.append(hn[1:])
                continue
            elif hn in ("def", "class") and len(items) > 1 and isinstance(items[1], Sym):
                (scope.commands if "command" in prev_deco else scope.known).add(items[1].name)
            elif hn == "async" and len(items) > 2 and isinstance(items[2], Sym):
                scope.known.add(items[2].name)
            elif hn == "import":
                for i, it in enumerate(items[1:], 1):
                    if isinstance(it, Sym) and it.name != "as":
                        if i + 1 < len(items) and isinstance(items[i + 1], Sym) and items[i + 1].name == "as":
                            continue
                        scope.known.add(it.name.split(".")[0])
            elif hn == "from":
                try:
                    k = [i for i, x in enumerate(items) if isinstance(x, Sym) and x.name == "import"][0]
                except IndexError:
                    k = len(items)
                for i, it in enumerate(items[k + 1:], k + 1):
                    if isinstance(it, Sym) and it.name != "as":
                        if i + 1 < len(items) and isinstance(items[i + 1], Sym) and items[i + 1].name == "as":
                            continue
                        scope.known.add(it.name)
            elif hn == "for":
                for t in items[1:]:
                    if isinstance(t, Sym) and t.name == "in":
                        break
                    self._target_names(t, scope.known)
            elif hn in ("with", "except"):
                for i, it in enumerate(items):
                    if isinstance(it, Sym) and it.name == "as" and i + 1 < len(items):
                        self._target_names(items[i + 1], scope.known)
            elif hn in ("global", "nonlocal"):
                for it in items[1:]:
                    if isinstance(it, Sym):
                        scope.known.add(it.name)
            prev_deco = []
            if hn in ("if", "elif", "else", "while", "for", "try", "except", "finally", "with", "match", "case", "do"):
                self.prescan(l.children, scope)
                if hn == "match":
                    for c in l.children:
                        self.prescan(c.children, scope)

    def _target_names(self, t, out):
        if isinstance(t, Sym) and IDENT.match(t.name):
            out.add(t.name)
        elif isinstance(t, Var) and isinstance(t.sym, Sym):
            out.add(t.sym.name)
        elif isinstance(t, Group):
            for x in t.items:
                self._target_names(x, out)
        elif isinstance(t, Splat):
            self._target_names(t.value, out)
        elif isinstance(t, Sym) and t.name.startswith("*") and IDENT.match(t.name.lstrip("*")):
            out.add(t.name.lstrip("*"))

    def _assign_index(self, items):
        for i, it in enumerate(items):
            if isinstance(it, Op) and not it.tick and it.name in ASSIGN_OPS:
                if i == 0:
                    return None
                lhs = items[:i]
                if len(lhs) == 1 or any(isinstance(x, Comma) for x in lhs):
                    if all(isinstance(x, (Sym, Var, Attr, Index, Group, Comma, Splat)) for x in lhs):
                        return i
                return None
            if isinstance(it, Op):
                return None
        return None

    # ---- blocks ---------------------------------------------------------------

    def units(self, lines):
        """Lines with their continuations (elif, else, except, finally) and
        decorators: [(line, conts, decos)]."""
        out = []
        decos = []
        for l in lines:
            head = l.items[0] if l.items else None
            hn = head.name if isinstance(head, Sym) else None
            if hn in CONTINUATIONS:
                if not out or not self._takes_cont(out[-1], hn):
                    raise self.err(f"'{hn}' without a matching if, for, while or try above it", head)
                out[-1][1].append(l)
                continue
            if hn and hn.startswith("@") and len(hn) > 1:
                decos.append(l)
                continue
            if decos and hn not in ("def", "class", "async"):
                raise self.err("a decorator must come just before def or class", decos[0])
            out.append((l, [], decos))
            decos = []
        if decos:
            raise self.err("a decorator must come just before def or class", decos[0])
        return out

    def _takes_cont(self, unit, cont):
        l, conts, _ = unit
        head = l.items[0]
        hn = head.name if isinstance(head, Sym) else None
        if hn == "async" and len(l.items) > 1 and isinstance(l.items[1], Sym):
            hn = l.items[1].name
        last = conts[-1].items[0].name if conts else hn
        if hn == "if":
            return cont in ("elif", "else") and last != "else"
        if hn in ("for", "while"):
            return cont == "else" and last != "else"
        if hn == "try":
            if cont in ("except", "except*"):
                return last in ("try", "except", "except*")
            if cont == "else":
                return last in ("except", "except*")
            if cont == "finally":
                return last != "finally"
        return False

    def compile_body(self, lines, ctx):
        us = self.units(lines)
        if not us:
            return Res(expr=const(None))
        stmts = []
        for u in us[:-1]:
            stmts += self.as_stmts(self.compile_unit(u, "stmt"))
        r = self.compile_unit(us[-1], ctx)
        return Res(stmts + r.stmts, r.expr)

    def compile_unit(self, unit, ctx):
        l, conts, decos = unit
        r = self.compile_items(list(l.items), l.children, ctx, conts=conts, decos=decos, form=l)
        for s in r.stmts:
            if not hasattr(s, "lineno") and l.pos:
                self.at(s, l)
        return r

    def compile_line(self, l: Line, ctx):
        return self.compile_items(list(l.items), l.children, ctx, form=l)

    # ---- forms ----------------------------------------------------------------

    def compile_items(self, items, children, ctx, conts=(), decos=(), form=None):
        if not items:
            if children:
                return self.compile_body(children, ctx)
            return Res(expr=const(None))
        i = self._assign_index(items)
        if i is not None:
            return self.compile_assign(items[:i], items[i], items[i + 1:], children, ctx)
        head = items[0]
        if isinstance(head, Sym):
            if head.name in SPECIALS and self._binding(Sym(head.name)) is None:
                return self.special(head.name, items, children, ctx, conts, decos)
            if head.name in self.macros:
                return self.expand_macro(self.macros[head.name], items, children, ctx)
        if conts:
            raise self.err("elif, else, except and finally go after if, for, while or try", conts[0])
        if decos:
            raise self.err("a decorator must come just before def or class", decos[0])
        return self.compile_shell(items, children, ctx)

    def compile_shell(self, items, children, ctx):
        # (A trailing & runs it in the background.)
        if len(items) > 1 and isinstance(items[-1], Op) and items[-1].name == "&":
            return self.compile_bg(items[:-1], children)
        semis = _split(items, lambda x: isinstance(x, Semi))
        if len(semis) > 1:
            stmts = []
            parts = [p for p in semis if p]
            for p in parts[:-1]:
                stmts += self.as_stmts(self.compile_items(p, (), "stmt"))
            r = self.compile_items(parts[-1], children, ctx)
            return Res(stmts + r.stmts, r.expr)
        sub = "cond" if ctx in ("stmt", "tail", "cond") else "value"
        for opname, is_and in (("||", False), ("&&", True)):
            parts = _split(items, lambda x: isinstance(x, Op) and x.name == opname and not x.tick)
            if len(parts) > 1:
                if any(not p for p in parts):
                    raise self.err(f"'{opname}' needs something on both sides", items[0])
                rs = [self.compile_shell(p, children if k == len(parts) - 1 else (), sub) for k, p in enumerate(parts)]
                return self.bool_chain(is_and, rs)
        if isinstance(items[0], Op) and items[0].name == "!" and len(items) > 1:
            r = self.compile_shell(items[1:], children, sub)
            return Res(r.stmts, ast.UnaryOp(ast.Not(), r.expr))
        segs = self._pipe_split(items)
        if len(segs) > 1:
            return self.compile_pipe(segs, children, ctx)
        return self.compile_segment(items, children, ctx)

    def _pipe_split(self, items):
        segs, cur = [], []
        for k, x in enumerate(items):
            if isinstance(x, Op) and x.name == "|" and not x.tick and cur and k + 1 < len(items) and self._is_stage_head(items[k + 1]):
                segs.append(cur)
                cur = []
            else:
                cur.append(x)
        segs.append(cur)
        return segs

    def _is_stage_head(self, x):
        if isinstance(x, Sym):
            return x.name not in WORD_OPS and x.name not in ("True", "False", "None")
        return isinstance(x, (Caret, Word)) and not (isinstance(x, Word) and not isinstance(x.parts[0], str))

    def bool_chain(self, is_and, rs):
        if all(not r.stmts for r in rs[1:]):
            return Res(rs[0].stmts, ast.BoolOp(ast.And() if is_and else ast.Or(), [r.expr for r in rs]))
        t = self.tmp("b")
        stmts = rs[0].stmts + [ast.Assign([name(t, True)], rs[0].expr)]
        cur = stmts
        for r in rs[1:]:
            test = name(t) if is_and else ast.UnaryOp(ast.Not(), name(t))
            node = ast.If(test, r.stmts + [ast.Assign([name(t, True)], r.expr)], [])
            cur.append(node)
            cur = node.body
        return Res(stmts, name(t))

    def segment_kind(self, items):
        head = items[0]
        if isinstance(head, Caret):
            return "command"
        if isinstance(head, Sym):
            if head.name in SPECIALS and self._binding(Sym(head.name)) is None or head.name in self.macros:
                return "special"
            if head.name in WORD_OPS or head.name in ("True", "False", "None", "..."):
                return "expr"
            if self.is_known(head):
                return "expr"
            return "command"
        if isinstance(head, Word) and isinstance(head.parts[0], str):
            return "command"
        if isinstance(head, Word):
            # ($HOME/bin/x, $dir/run: a program by its path.)
            if any(isinstance(p, str) and "/" in p for p in head.parts):
                return "command"
        return "expr"

    def compile_segment(self, items, children, ctx):
        kind = self.segment_kind(items)
        if kind == "special":
            return self.compile_items(items, children, ctx)
        if kind == "command":
            return self.compile_command(items, children, "capture" if ctx == "value" else "attach")
        if len(items) == 1 and isinstance(items[0], Sym) and self.is_known(items[0]) and not children:
            return Res(expr=self.at(call(rt("auto"), [self.name_load(items[0])]), items[0]))
        if len(items) == 1 and isinstance(items[0], Attr) and not children:
            # ($x.method, (f a).method: called, as a bareword alone would be.)
            r = self.compile_value(items[0])
            return Res(r.stmts, self.at(call(rt("auto"), [r.expr]), items[0]))
        return self.compile_expr(items, ctx, children)

    # ---- commands -------------------------------------------------------------

    def compile_command(self, items, children, mode):
        head = items[0]
        force = False
        if isinstance(head, Caret):
            force = True
            head = head.word
        if isinstance(head, Sym):
            prog = call(rt("t"), [const(head.name)]) if head.name.startswith("~") else const(head.name)
            pres = Res(expr=prog)
        else:
            pres = self.cmd_word(head)
        args, redirs = [], []
        rest = list(items[1:])
        k = 0
        while k < len(rest):
            x = rest[k]
            if isinstance(x, Op) and not x.tick and x.name in SPACED_REDIRS and k + 1 < len(rest):
                redirs.append((x.name, self.cmd_word(rest[k + 1])))
                k += 2
                continue
            if isinstance(x, Redir):
                redirs.append((x.op, self.cmd_word(x.target) if x.target is not None else Res(expr=const(None))))
            else:
                args.append(self.cmd_word(x))
            k += 1
        for c in children:
            if len(c.items) == 1 and not c.children:
                x = c.items[0]
                if isinstance(x, Redir):
                    redirs.append((x.op, self.cmd_word(x.target) if x.target is not None else Res(expr=const(None))))
                else:
                    args.append(self.cmd_word(x))
            else:
                args.append(self.compile_line(c, "value"))
        rs = [pres] + args + [r for _, r in redirs]
        stmts, exprs = self.gather(rs)
        prog_e, arg_es, red_es = exprs[0], exprs[1:1 + len(args)], exprs[1 + len(args):]
        red_list = ast.List([ast.Tuple([const(op), e], ast.Load()) for (op, _), e in zip(redirs, red_es)], ast.Load())
        e = call(rt("cmd"), [call(name("globals")), prog_e, ast.List(arg_es, ast.Load()), red_list, const(mode), const(force)])
        return Res(stmts, self.at(e, items[0]))

    def cmd_word(self, x) -> Res:
        """An argument of a command: a string, a glob, or a value (a list
        becomes several arguments)."""
        if isinstance(x, (Sym, Op)):
            text = x.name
            if isinstance(x, Op) and x.tick:
                text = f"`{text}`"
            if GLOB_CHARS & set(text):
                return Res(expr=call(rt("g"), [const(text)]))
            if text.startswith("~"):
                return Res(expr=call(rt("t"), [const(text)]))
            return Res(expr=const(text))
        if isinstance(x, Num):
            return Res(expr=const(x.text))
        if isinstance(x, Str):
            return Res(expr=const(x.value))
        if isinstance(x, Word):
            r = self.word_value(x, glob_ok=False)
            texts = [p for p in x.parts if isinstance(p, str)]
            if any(GLOB_CHARS & set(p) for p in texts):
                return Res(r.stmts, call(rt("g"), [r.expr]))
            if isinstance(x.parts[0], str) and x.parts[0].startswith("~"):
                return Res(r.stmts, call(rt("t"), [r.expr]))
            return r
        if isinstance(x, KwArg):
            v = self.cmd_word(x.value)
            return Res(v.stmts, ast.JoinedStr([const(x.name + "="), ast.FormattedValue(v.expr, -1, None)]))
        if isinstance(x, Unary):
            v = self.compile_value(x.operand)
            return Res(v.stmts, ast.JoinedStr([const(x.op), ast.FormattedValue(v.expr, -1, None)]))
        if isinstance(x, Unquote) and isinstance(x.form, Sym):
            return Res(expr=call(rt("t"), [const("~" + x.form.name)]))
        if isinstance(x, SymLit):
            return Res(expr=const(":" + x.sym.name))
        if isinstance(x, Comma):
            return Res(expr=const(","))
        if isinstance(x, Caret):
            return Res(expr=const(F.show(x)))
        if isinstance(x, Splat):
            return self.compile_value(x.value)
        if isinstance(x, Line):
            return self.compile_line(x, "value")
        return self.compile_value(x)

    def compile_pipe(self, segs, children, ctx):
        stages = []
        for k, seg in enumerate(segs):
            if not seg:
                raise self.err("'|' needs something on both sides")
            last = k == len(segs) - 1
            stages.append(self.compile_stage(seg, children if last else (), k == 0))
        stmts, exprs = self.gather(stages)
        mode = "capture" if ctx == "value" else "attach"
        return Res(stmts, self.at(call(rt("pipe"), [ast.List(exprs, ast.Load()), const(mode)]), segs[0][0]))

    def compile_stage(self, items, children, first):
        kind = self.segment_kind(items)
        if kind == "command":
            return self.compile_command(items, children, "stage")
        if kind == "special":
            r = self.compile_items(items, children, "value")
            return Res(r.stmts, call(rt("value_stage"), [r.expr]) if first else call(rt("call_stage"), [r.expr]))
        head = items[0]
        is_call = isinstance(head, Sym) and self.is_known(head) or len(items) > 1 or children
        if not is_call:
            if not first:
                raise self.err("only a program or a function can take a pipe's input", head)
            v = self.compile_value(head)
            return Res(v.stmts, call(rt("value_stage"), [v.expr]))
        func = self.compile_head(head)
        a = self.compile_call_args(items[1:], children)
        stmts, exprs = self.gather([func, a[0]])
        args = a[1] + [name("_tessh_in")]
        lam = ast.Lambda(ast.arguments([], [ast.arg("_tessh_in")], None, [], [], None, []), call(exprs[0], args, a[2]))
        return Res(stmts, call(rt("call_stage"), [lam]))

    def compile_bg(self, items, children):
        segs = self._pipe_split(items)
        if len(segs) > 1:
            r = self.compile_pipe(segs, children, "stmt")
            r.expr.args[1] = const("bg")
            return r
        if self.segment_kind(items) == "command":
            return self.compile_command(items, children, "bg")
        r = self.compile_items(items, children, "value")
        if r.stmts:
            raise self.err("only a command or a call can run in the background", items[0])
        return Res(expr=call(rt("bg_call"), [ast.Lambda(_noargs(), r.expr)]))

    # ---- expressions ----------------------------------------------------------

    def compile_expr(self, items, ctx="value", children=()):
        if not items:
            raise self.err("expected an expression")
        if len(items) > 1 and (any(isinstance(x, Op) and not x.tick and x.name in ("||", "&&") for x in items) or len(self._pipe_split(items)) > 1):
            return self.compile_shell(items, children, ctx)
        head = items[0]
        if isinstance(head, Sym) and (head.name in SPECIALS and self._binding(Sym(head.name)) is None or head.name in self.macros):
            return self.compile_items(items, children, ctx)
        if isinstance(head, Op) and not head.tick and len(items) > 1 and (head.name in BINOPS or head.name in CMPOPS or head.name in self.ops or head.name in ("!",)):
            if not any(isinstance(x, Op) and (x.name in BINOPS or x.name in CMPOPS) for x in items[1:]):
                return self.lisp_op(head, items[1:])
        if any(isinstance(x, Comma) for x in items):
            parts = [p for p in _split(items, lambda x: isinstance(x, Comma))]
            if parts and not parts[-1]:
                parts = parts[:-1]
            rs = [self.compile_expr(p, "value") for p in parts]
            stmts, exprs = self.gather(rs)
            return Res(stmts, ast.Tuple(exprs, ast.Load()))
        toks = self.tokens(items)
        tree, pos = self.parse(toks, 0, 0)
        if pos != len(toks):
            raise self.err("unexpected operator", toks[pos][2] if toks[pos][0] == "op" else items[0])
        if children and tree[0] != "chunk":
            raise self.err("indented arguments can only follow a call", children[0])
        return self.compile_tree(tree, ctx, children)

    def lisp_op(self, op, args):
        rs = [self.compile_value(a) for a in args]
        stmts, exprs = self.gather(rs)
        n = op.name
        if len(exprs) == 1 and n in PREFIX:
            return Res(stmts, self._unary(n, exprs[0]))
        if n in CMPOPS:
            return Res(stmts, ast.Compare(exprs[0], [CMPOPS[n]() for _ in exprs[1:]], exprs[1:]))
        e = exprs[0]
        for x in exprs[1:]:
            e = self._binop(n, e, x, op)
        return Res(stmts, e)

    def tokens(self, items):
        toks = []
        for x in items:
            if isinstance(x, Op):
                if x.tick or x.name in BINOPS or x.name in CMPOPS or x.name in self.ops or x.name in (":=", "!", "||", "&&"):
                    toks.append(["op", ("`" + x.name) if x.tick else x.name, x])
                    continue
                toks.append(["x", Sym(x.name, pos=x.pos), None])
                continue
            if isinstance(x, Sym) and x.name in WORD_OPS:
                toks.append(["op", x.name, x])
                continue
            toks.append(["x", x, None])
        # not in, is not
        merged = []
        for t in toks:
            if merged and t[0] == "op" and merged[-1][0] == "op":
                if merged[-1][1] == "not" and t[1] == "in" and len(merged) > 1 and merged[-2][0] == "x":
                    merged[-1][1] = "not in"
                    continue
                if merged[-1][1] == "is" and t[1] == "not":
                    merged[-1][1] = "is not"
                    continue
            merged.append(t)
        # Operators with nothing on a side are words.
        out = []
        want_operand = True
        for k, t in enumerate(merged):
            if t[0] == "op":
                if want_operand:
                    if t[1] in PREFIX:
                        t = ["pre", t[1], t[2]]
                        out.append(t)
                        continue
                    out.append(["x", Sym(t[1], pos=t[2].pos), None])
                    want_operand = False
                    continue
                out.append(t)
                want_operand = True
            else:
                out.append(t)
                want_operand = False
        while out and out[-1][0] in ("op", "pre"):
            t = out.pop()
            out.append(["x", Sym(t[1].lstrip("`"), pos=t[2].pos), None])
            # (merge below)
            break
        # Consecutive operands make one chunk (an application).
        chunks = []
        for t in out:
            if t[0] == "x":
                if chunks and chunks[-1][0] == "chunk":
                    chunks[-1][1].append(t[1])
                else:
                    chunks.append(["chunk", [t[1]], None])
            else:
                chunks.append(t)
        return chunks

    def _prec(self, opname):
        if opname.startswith("`"):
            return 6, "left"
        if opname in self.ops:
            return self.ops[opname][0], self.ops[opname][1]
        return PREC[opname], ("right" if opname in RIGHT else "left")

    def parse(self, toks, k, min_prec):
        if k >= len(toks):
            raise self.err("expected something after the operator", toks[-1][2] if toks else None)
        t = toks[k]
        if t[0] == "pre":
            p = PREFIX[t[1]]
            operand, k = self.parse(toks, k + 1, p if t[1] in ("not", "!") else p)
            lhs = ("un", t[1], operand, t[2])
        elif t[0] == "chunk":
            lhs = ("chunk", t[1])
            k += 1
        else:
            raise self.err("expected an operand", t[2])
        while k < len(toks):
            t = toks[k]
            if t[0] != "op":
                break
            p, assoc = self._prec(t[1])
            if p < min_prec:
                break
            if t[1] in CMPOPS:
                ops, rhss = [], []
                while k < len(toks) and toks[k][0] == "op" and toks[k][1] in CMPOPS:
                    ops.append(toks[k])
                    rhs, k = self.parse(toks, k + 1, p + 1)
                    rhss.append(rhs)
                lhs = ("cmp", lhs, ops, rhss)
                continue
            nxt = p + 1 if assoc != "right" else p
            rhs, k = self.parse(toks, k + 1, nxt)
            lhs = ("bin", t[1], lhs, rhs, t[2])
            if assoc == "none" and k < len(toks) and toks[k][0] == "op" and toks[k][1] == t[1]:
                raise self.err(f"'{t[1]}' can't be chained", toks[k][2])
        return lhs, k

    def compile_tree(self, tree, ctx, children=()):
        kind = tree[0]
        if kind == "chunk":
            return self.compile_operand(tree[1], ctx, children)
        if kind == "un":
            sub = "cond" if tree[1] in ("not", "!") and ctx == "cond" else "value"
            r = self.compile_tree(tree[2], sub)
            return Res(r.stmts, self.at(self._unary(tree[1], r.expr), tree[3]))
        if kind == "cmp":
            rs = [self.compile_tree(tree[1], "value")] + [self.compile_tree(x, "value") for x in tree[3]]
            stmts, exprs = self.gather(rs)
            return Res(stmts, self.at(ast.Compare(exprs[0], [CMPOPS[o[1]]() for o in tree[2]], exprs[1:]), tree[2][0][2]))
        if kind == "bin":
            op = tree[1]
            if op in ("and", "&&", "or", "||"):
                sub = "cond" if ctx == "cond" else "value"
                flat = self._flatten(tree, {"and", "&&"} if op in ("and", "&&") else {"or", "||"})
                return self.bool_chain(op in ("and", "&&"), [self.compile_tree(x, sub) for x in flat])
            if op == ":=":
                target = tree[2]
                if target[0] != "chunk" or len(target[1]) != 1 or not isinstance(target[1][0], (Sym, Var)):
                    raise self.err("':=' needs a name on its left", tree[4])
                s = target[1][0] if isinstance(target[1][0], Sym) else target[1][0].sym
                v = self.compile_tree(tree[3], "value")
                return Res(v.stmts, ast.NamedExpr(name(self.bind(s), True), v.expr))
            rs = [self.compile_tree(tree[2], "value"), self.compile_tree(tree[3], "value")]
            stmts, (a, b) = self.gather(rs)
            return Res(stmts, self.at(self._binop(op, a, b, tree[4]), tree[4]))
        raise AssertionError(tree)

    def _flatten(self, tree, ops):
        if tree[0] == "bin" and tree[1] in ops:
            return self._flatten(tree[2], ops) + self._flatten(tree[3], ops)
        return [tree]

    def _unary(self, op, e):
        return ast.UnaryOp({"-": ast.USub, "+": ast.UAdd, "not": ast.Not, "!": ast.Not, "~": ast.Invert}[op](), e)

    def _binop(self, op, a, b, form):
        if op.startswith("`"):
            return call(self.name_load(Sym(op[1:], pos=form.pos), form), [a, b])
        if op in self.ops:
            return call(self.name_load(self.ops[op][2], form), [a, b])
        if op in BINOPS:
            return ast.BinOp(a, BINOPS[op](), b)
        raise self.err(f"unknown operator '{op}'", form)

    def compile_operand(self, chunk, ctx, children=()):
        if len(chunk) == 1 and not children:
            return self.compile_value(chunk[0], ctx)
        return self.compile_application(chunk, children, ctx)

    def compile_head(self, head) -> Res:
        if isinstance(head, Sym):
            return Res(expr=self.name_load(head))
        return self.compile_value(head)

    def compile_application(self, items, children, ctx):
        head = items[0]
        if isinstance(head, Sym):
            if head.name in SPECIALS and self._binding(Sym(head.name)) is None or head.name in self.macros:
                return self.compile_items(items, children, ctx)
            if not self.is_known(head):
                return self.compile_command(items, children, "capture" if ctx == "value" else "attach")
        elif isinstance(head, (Caret, Word)) and self.segment_kind(items) == "command":
            return self.compile_command(items, children, "capture" if ctx == "value" else "attach")
        f = self.compile_head(head)
        a = self.compile_call_args(items[1:], children)
        stmts, exprs = self.gather([f, a[0]])
        c = self.at(call(exprs[0], a[1], a[2]), head)
        if a[3]:
            red = ast.List([ast.Tuple([const(op), e], ast.Load()) for op, e in a[3]], ast.Load())
            c = call(rt("redirect"), [ast.Lambda(_noargs(), c), red])
        return Res(stmts, c)

    def compile_call_args(self, items, children):
        """(Res of the statements, args, keywords, redirections)."""
        rs, slots = [], []
        for x in items:
            self._call_arg(x, rs, slots)
        for c in children:
            if len(c.items) == 1 and not c.children:
                self._call_arg(c.items[0], rs, slots)
            else:
                rs.append(self.compile_line(c, "value"))
                slots.append(("arg", None))
        stmts, exprs = self.gather(rs)
        args, kws, reds = [], [], []
        for (kind, extra), e in zip(slots, exprs):
            if kind == "arg":
                args.append(e)
            elif kind == "star":
                args.append(ast.Starred(e, ast.Load()))
            elif kind == "kw":
                kws.append(ast.keyword(extra, e))
            elif kind == "kwstar":
                kws.append(ast.keyword(None, e))
            elif kind == "redir":
                reds.append((extra, e))
        return Res(stmts), args, kws, reds

    def _call_arg(self, x, rs, slots):
        if isinstance(x, KwArg):
            rs.append(self.compile_value(x.value))
            slots.append(("kw", x.name))
        elif isinstance(x, Splat):
            rs.append(self.compile_value(x.value))
            slots.append(("star" if x.n == 1 else "kwstar", None))
        elif isinstance(x, Redir):
            rs.append(self.cmd_word(x.target) if x.target is not None else Res(expr=const(None)))
            slots.append(("redir", x.op))
        elif isinstance(x, Comma):
            raise self.err("arguments are separated by spaces, not commas", x)
        else:
            rs.append(self.compile_value(x))
            slots.append(("arg", None))

    # ---- values ---------------------------------------------------------------

    def compile_value(self, x, ctx="value") -> Res:
        if isinstance(x, Sym):
            if x.name in ("True", "False", "None"):
                return Res(expr=self.at(const({"True": True, "False": False, "None": None}[x.name]), x))
            if x.name == "...":
                return Res(expr=const(...))
            if x.name.startswith("~"):
                return Res(expr=call(rt("t"), [const(x.name)]))
            return Res(expr=self.at(const(x.name), x))
        if isinstance(x, Num):
            return Res(expr=self.at(const(x.value), x))
        if isinstance(x, Str):
            return Res(expr=self.at(const(x.value), x))
        if isinstance(x, IStr):
            return self.istr_value(x)
        if isinstance(x, Word):
            return self.word_value(x)
        if isinstance(x, Var):
            return Res(expr=self.var_load(x))
        if isinstance(x, Status):
            return Res(expr=call(rt("status")))
        if isinstance(x, Attr):
            r = self.compile_value(x.obj)
            return Res(r.stmts, self.at(ast.Attribute(r.expr, x.name, ast.Load()), x))
        if isinstance(x, Index):
            r = self.compile_value(x.obj)
            s = self.compile_subscript(x.items, x)
            stmts, (o, i) = self.gather([r, s])
            return Res(stmts, self.at(ast.Subscript(o, i, ast.Load()), x))
        if isinstance(x, Unary):
            r = self.compile_value(x.operand)
            return Res(r.stmts, self.at(self._unary(x.op, r.expr), x))
        if isinstance(x, Group):
            return self.compile_group(x, ctx)
        if isinstance(x, Splat):
            r = self.compile_value(x.value)
            if x.n == 2:
                raise self.err("** only goes in a call or a dict", x)
            return Res(r.stmts, ast.Starred(r.expr, ast.Load()))
        if isinstance(x, KwArg):
            raise self.err(f"keyword argument {x.name}= outside a call", x)
        if isinstance(x, SymLit):
            return Res(expr=self.quote_ast(x.sym, quasi=False))
        if isinstance(x, Quasi):
            return Res(expr=self.quote_ast(_group_form(x.form), quasi=True))
        if isinstance(x, Unquote):
            if isinstance(x.form, Sym):
                return Res(expr=call(rt("t"), [const("~" + x.form.name)]))
            raise self.err("~ outside a quasi template", x)
        if isinstance(x, Op):
            return Res(expr=const(x.name))
        if isinstance(x, Line):
            return self.compile_line(x, ctx)
        if isinstance(x, Caret):
            return self.compile_command([x], (), "capture")
        if isinstance(x, Redir):
            raise self.err("a redirection only goes after a command or a call", x)
        if isinstance(x, (Comma, Semi)):
            raise self.err("unexpected separator", x)
        raise self.err(f"can't compile {F.show(x)}", x)

    def var_load(self, v: Var):
        s = v.sym
        if isinstance(s, Unquote):
            raise self.err("$~ outside a quasi template", v)
        if self.is_known(s) or self._binding(s) is not None or s.marks:
            return self.name_load(s, v)
        if self.ns is not None and s.name in self.ns:
            return self.name_load(s, v)
        return self.at(call(rt("lookup"), [call(name("globals")), const(s.name)]), v)

    def istr_value(self, x: IStr) -> Res:
        rs, kinds = [], []
        for p in x.parts:
            if isinstance(p, str):
                kinds.append(p)
            else:
                rs.append(self.compile_interp(p))
                kinds.append(p)
        stmts, exprs = self.gather(rs)
        vals, it = [], iter(exprs)
        for p in kinds:
            if isinstance(p, str):
                vals.append(const(p))
            else:
                vals.append(self._fmt(next(it), p))
        return Res(stmts, self.at(ast.JoinedStr(vals), x))

    def _fmt(self, e, ip: Interp):
        conv = ord(ip.conv) if ip.conv else -1
        spec = ast.JoinedStr([const(ip.spec)]) if ip.spec else None
        return ast.FormattedValue(e, conv, spec)

    def compile_interp(self, ip: Interp) -> Res:
        e = ip.expr
        if isinstance(e, Group):
            if ip.braced:
                items = list(e.items)
                if len(items) == 1 and isinstance(items[0], Sym) and DOTTED.match(items[0].name):
                    s = items[0]
                    root = s.name.split(".")[0]
                    if self.is_known(Sym(root)) or (self.ns is not None and root in self.ns):
                        return Res(expr=self.name_load(s))
                    if "." not in s.name:
                        return Res(expr=call(rt("lookup"), [call(name("globals")), const(s.name)]))
                return self.compile_expr(items, "value")
            return self.compile_group(e, "value")
        return self.compile_value(e)

    def word_value(self, x: Word, glob_ok=True) -> Res:
        rs, kinds = [], []
        for p in x.parts:
            if isinstance(p, str):
                kinds.append(p)
            elif isinstance(p, Str):
                kinds.append(p.value if isinstance(p.value, str) else p.value.decode())
            elif isinstance(p, IStr):
                for q in p.parts:
                    if isinstance(q, str):
                        kinds.append(q)
                    else:
                        rs.append(self.compile_interp(q))
                        kinds.append(q)
            elif isinstance(p, Interp):
                rs.append(self.compile_interp(p))
                kinds.append(p)
            else:
                rs.append(self.compile_value(p))
                kinds.append(None)
        stmts, exprs = self.gather(rs)
        vals, it = [], iter(exprs)
        for p in kinds:
            if isinstance(p, str):
                vals.append(const(p))
            elif isinstance(p, Interp):
                vals.append(self._fmt(next(it), p))
            else:
                vals.append(ast.FormattedValue(next(it), -1, None))
        e = self.at(ast.JoinedStr(vals), x)
        if glob_ok and isinstance(x.parts[0], str) and x.parts[0].startswith("~"):
            e = call(rt("t"), [e])
        return Res(stmts, e)

    def compile_subscript(self, items, form):
        items = list(items)
        if any(isinstance(x, Comma) for x in items):
            parts = [p for p in _split(items, lambda x: isinstance(x, Comma)) if p]
            rs = [self.compile_subscript(p, form) for p in parts]
            stmts, exprs = self.gather(rs)
            return Res(stmts, ast.Tuple(exprs, ast.Load()))
        # Slices: pieces split at ':' (in words like 1:3 and $a:$b, or spaced).
        toks = []
        for x in items:
            if isinstance(x, Sym) and ":" in x.name and not x.name.endswith("::") or isinstance(x, Sym) and x.name == ":":
                for k, piece in enumerate(x.name.split(":")):
                    if k:
                        toks.append(":")
                    if piece:
                        toks.append(_piece(piece, x))
            elif isinstance(x, Word) and any(isinstance(p, str) and ":" in p for p in x.parts):
                cur = []
                for p in x.parts:
                    if isinstance(p, str) and ":" in p:
                        for k, piece in enumerate(p.split(":")):
                            if k:
                                if cur:
                                    toks.append(cur[0] if len(cur) == 1 else Word(tuple(cur), pos=x.pos))
                                cur = []
                                toks.append(":")
                            if piece:
                                cur.append(piece)
                    else:
                        cur.append(p)
                if cur:
                    toks.append(cur[0] if len(cur) == 1 and not isinstance(cur[0], str) else (_piece(cur[0], x) if len(cur) == 1 else Word(tuple(cur), pos=x.pos)))
            else:
                toks.append(x)
        if ":" not in toks:
            return self.compile_expr(items, "value")
        parts, cur = [], []
        for t in toks:
            if t == ":":
                parts.append(cur)
                cur = []
            else:
                cur.append(t)
        parts.append(cur)
        if len(parts) > 3:
            raise self.err("a slice has at most three parts", form)
        rs = [self.compile_expr(p, "value") if p else Res(expr=None) for p in parts]
        stmts, exprs = self.gather(rs)
        exprs += [None] * (3 - len(exprs))
        return Res(stmts, ast.Slice(*exprs))

    def compile_group(self, g: Group, ctx="value"):
        items = list(g.items)
        if g.kind == "(":
            return self.compile_paren(items, ctx, g)
        if g.kind == "[":
            return self.compile_list(items, g)
        return self.compile_brace(items, g)

    def compile_paren(self, items, ctx, g):
        if not items:
            return Res(expr=ast.Tuple([], ast.Load()))
        if any(isinstance(x, Semi) for x in items):
            return self.compile_shell(items, (), ctx)
        if _top_for(items):
            elt, gens = self.comprehension(items, g)
            return Res(expr=ast.GeneratorExp(elt[0], gens)) if not isinstance(elt, tuple) or len(elt) == 1 else Res(expr=ast.GeneratorExp(elt[0], gens))
        if any(isinstance(x, Comma) for x in items):
            return self.compile_expr(items, "value")
        return self.compile_items(items, (), ctx)

    def compile_list(self, items, g):
        if _top_for(items):
            elt, gens = self.comprehension(items, g)
            return Res(expr=ast.ListComp(elt[0], gens))
        if any(isinstance(x, Comma) for x in items):
            parts = [p for p in _split(items, lambda x: isinstance(x, Comma)) if p]
            rs = [self.compile_expr(p, "value") for p in parts]
        elif _has_binop(items):
            rs = [self.compile_expr(items, "value")]
        else:
            rs = [self.compile_value(x) for x in items]
        stmts, exprs = self.gather(rs)
        return Res(stmts, self.at(ast.List(exprs, ast.Load()), g))

    def compile_brace(self, items, g):
        if not items:
            return Res(expr=ast.Dict([], []))
        if _top_for(items):
            elt, gens = self.comprehension(items, g, dict_ok=True)
            if len(elt) == 2:
                return Res(expr=ast.DictComp(elt[0], elt[1], gens))
            return Res(expr=ast.SetComp(elt[0], gens))
        entries = self.dict_entries(items)
        if entries is None:
            if any(isinstance(x, Comma) for x in items):
                parts = [p for p in _split(items, lambda x: isinstance(x, Comma)) if p]
                rs = [self.compile_expr(p, "value") for p in parts]
            else:
                rs = [self.compile_value(x) for x in items]
            stmts, exprs = self.gather(rs)
            return Res(stmts, ast.Set(exprs))
        rs, keys = [], []
        for k, v in entries:
            if k is None:
                keys.append(None)
                rs.append(self.compile_value(v[0].value))
            else:
                keys.append(True)
                rs.append(k)
                rs.append(self.compile_expr(v, "value"))
        stmts, exprs = self.gather(rs)
        ks, vs, it = [], [], iter(exprs)
        for k in keys:
            if k is None:
                ks.append(None)
                vs.append(next(it))
            else:
                ks.append(next(it))
                vs.append(next(it))
        return Res(stmts, self.at(ast.Dict(ks, vs), g))

    def _key(self, x):
        """If x is a dict key (it ends with a ':'), the key's Res."""
        if isinstance(x, Sym) and x.name.endswith(":") and len(x.name) > 1 and not x.name.endswith("::"):
            t = x.name[:-1]
            from .reader import NUMBER, _number
            return Res(expr=const(_number(t) if NUMBER.match(t) else t))
        if isinstance(x, Word) and isinstance(x.parts[-1], str) and x.parts[-1].endswith(":"):
            parts = x.parts[:-1] + ((x.parts[-1][:-1],) if x.parts[-1][:-1] else ())
            if len(parts) == 1 and not isinstance(parts[0], str):
                return self.compile_value(parts[0])
            return self.word_value(Word(tuple(parts), pos=x.pos))
        return None

    def dict_entries(self, items):
        """[(key Res | None for **, value items)] or None if it's a set."""
        if any(isinstance(x, Comma) for x in items):
            parts = [p for p in _split(items, lambda x: isinstance(x, Comma)) if p]
        else:
            parts = None
        entries = []
        if parts is not None:
            for p in parts:
                if isinstance(p[0], Splat) and p[0].n == 2 and len(p) == 1:
                    entries.append((None, p))
                    continue
                k = self._key(p[0])
                if k is None and len(p) > 2 and isinstance(p[1], Sym) and p[1].name == ":":
                    k = self.compile_value(p[0])
                    p = [p[0]] + p[2:]
                if k is None:
                    return None
                if len(p) < 2:
                    raise self.err("a key needs a value", p[0])
                entries.append((k, p[1:]))
            return entries
        cur_key, cur = None, []
        seen_key = False
        k_i = 0
        while k_i < len(items):
            x = items[k_i]
            k = self._key(x)
            if k is None and k_i + 1 < len(items) and isinstance(items[k_i + 1], Sym) and items[k_i + 1].name == ":":
                k = self.compile_value(x)
                k_i += 1
            if k is not None or (isinstance(x, Splat) and x.n == 2):
                if seen_key:
                    if not cur and cur_key is not None:
                        raise self.err("a key needs a value", x)
                    entries.append((cur_key, cur))
                seen_key = True
                if k is None:
                    entries.append((None, [x]))
                    seen_key = False
                    cur_key, cur = None, []
                else:
                    cur_key, cur = k, []
            else:
                if not seen_key:
                    return None if not entries else self._bad_dict(x)
                cur.append(x)
            k_i += 1
        if seen_key:
            if not cur:
                raise self.err("a key needs a value", items[-1])
            entries.append((cur_key, cur))
        return entries

    def _bad_dict(self, x):
        raise self.err("expected a key (key: value)", x)

    def comprehension(self, items, g, dict_ok=False):
        k = _top_for(items)
        elt_items = items[:k]
        rest = items[k:]
        gens = []
        # (Its names are its own.)
        with self.new_scope("comp"):
            clauses = []
            cur = None
            for x in rest:
                if isinstance(x, Sym) and x.name in ("for", "if") or (isinstance(x, Sym) and x.name == "async"):
                    if x.name == "async":
                        continue
                    cur = [x.name, []]
                    clauses.append(cur)
                else:
                    cur[1].append(x)
            stmts_all = []
            for kind, body in clauses:
                if kind == "for":
                    ins = [i for i, y in enumerate(body) if isinstance(y, Sym) and y.name == "in"]
                    if not ins:
                        raise self.err("for needs in", g)
                    it = self.compile_expr(body[ins[0] + 1:], "value")
                    if it.stmts:
                        raise self.err("this can't go in a comprehension", g)
                    target = self.make_target(body[:ins[0]], g)
                    gens.append(ast.comprehension(target.expr, it.expr, [], 0))
                else:
                    c = self.compile_expr(body, "value")
                    if not gens:
                        raise self.err("if before for", g)
                    gens[-1].ifs.append(c.expr)
            if dict_ok and elt_items and self._key(elt_items[0]) is not None:
                kr = self._key(elt_items[0])
                vr = self.compile_expr(elt_items[1:], "value")
                return (kr.expr, vr.expr), gens
            if dict_ok and len(elt_items) > 2 and isinstance(elt_items[1], Sym) and elt_items[1].name == ":":
                return (self.compile_value(elt_items[0]).expr, self.compile_expr(elt_items[2:], "value").expr), gens
            e = self.compile_expr(elt_items, "value")
            if e.stmts:
                raise self.err("this can't go in a comprehension", g)
            return (e.expr,), gens

    # ---- assignment -----------------------------------------------------------

    def make_target(self, items, form, ctx=ast.Store):
        items = list(items)
        if any(isinstance(x, Comma) for x in items):
            parts = [p for p in _split(items, lambda x: isinstance(x, Comma)) if p]
            rs = [self.make_target(p, form, ctx) for p in parts]
            stmts, exprs = self.gather(rs)
            return Res(stmts, ast.Tuple(exprs, ctx()))
        if len(items) != 1:
            if all(isinstance(x, (Sym, Var, Group, Splat, Attr, Index)) for x in items):
                rs = [self.make_target([x], form, ctx) for x in items]
                stmts, exprs = self.gather(rs)
                return Res(stmts, ast.Tuple(exprs, ctx()))
            raise self.err("can't assign to this", items[0] if items else form)
        x = items[0]
        if isinstance(x, Var) and isinstance(x.sym, Sym):
            x = x.sym
        if isinstance(x, Sym):
            if x.name.startswith("*") and IDENT.match(x.name.lstrip("*")):
                inner = self.make_target([Sym(x.name.lstrip("*"), x.marks, pos=x.pos)], form, ctx)
                return Res(inner.stmts, ast.Starred(inner.expr, ctx()))
            if "." in x.name and DOTTED.match(x.name):
                root, _, attr = x.name.rpartition(".")
                return Res(expr=ast.Attribute(self.name_load(Sym(root, x.marks, pos=x.pos)), attr, ctx()))
            if ctx is ast.Del:
                return Res(expr=ast.Name(self._binding(x) or x.name, ast.Del()))
            return Res(expr=self.at(ast.Name(self.bind(x), ctx()), x))
        if isinstance(x, Attr):
            r = self.compile_value(x.obj)
            return Res(r.stmts, ast.Attribute(r.expr, x.name, ctx()))
        if isinstance(x, Index):
            r = self.compile_value(x.obj)
            s = self.compile_subscript(x.items, x)
            stmts, (o, i) = self.gather([r, s])
            return Res(stmts, ast.Subscript(o, i, ctx()))
        if isinstance(x, Group) and x.kind in ("[", "("):
            sub = [y for y in x.items]
            if not sub:
                raise self.err("nothing to assign to", x)
            r = self.make_target(sub if any(isinstance(y, Comma) for y in sub) else sub, form, ctx)
            if not isinstance(r.expr, ast.Tuple):
                r.expr = ast.Tuple([r.expr], ctx())
            return r
        if isinstance(x, Splat):
            inner = self.make_target([x.value], form, ctx)
            return Res(inner.stmts, ast.Starred(inner.expr, ctx()))
        raise self.err("can't assign to this", x)

    def compile_assign(self, lhs, op, rhs, children, ctx):
        if not rhs and not children:
            raise self.err(f"nothing after '{op.name}'", op)
        value = self.compile_expr(rhs, "value", children) if rhs else self.compile_body(children, "value")
        target = self.make_target(lhs, op)
        stmts = value.stmts + target.stmts
        if op.name == "=":
            node = ast.Assign([target.expr], value.expr)
        else:
            if isinstance(target.expr, (ast.Tuple, ast.Starred)):
                raise self.err(f"'{op.name}' takes one target", op)
            node = ast.AugAssign(target.expr, ASSIGN_OPS[op.name](), value.expr)
        stmts.append(self.at(node, op))
        result = const(None)
        if isinstance(target.expr, ast.Name):
            result = name(target.expr.id)
        return Res(stmts, result)

    # ---- special forms --------------------------------------------------------

    def special(self, n, items, children, ctx, conts, decos):
        head = items[0]
        if n in ("elif", "else", "except", "except*", "finally"):
            raise self.err(f"'{n}' without a matching if, for, while or try above it", head)
        if n == "case":
            raise self.err("'case' goes inside a match", head)
        if decos and n not in ("def", "class", "async"):
            raise self.err("a decorator must come just before def or class", decos[0])
        if conts and n not in ("if", "for", "while", "try", "async"):
            raise self.err(f"'{conts[0].items[0].name}' can't follow {n}", conts[0])
        f = getattr(self, "sf_" + n.replace("*", "_star"))
        return f(items, children, ctx, conts=conts, decos=decos)

    def sf_pass(self, items, children, ctx, **kw):
        return Res([ast.Pass()], const(None))

    def sf_break(self, items, children, ctx, **kw):
        return Res([self.at(ast.Break(), items[0])], const(None))

    def sf_continue(self, items, children, ctx, **kw):
        return Res([self.at(ast.Continue(), items[0])], const(None))

    def sf_return(self, items, children, ctx, **kw):
        if len(items) == 1 and not children:
            return Res([self.at(ast.Return(None), items[0])], const(None))
        v = self.compile_expr(items[1:], "value", children) if len(items) > 1 else self.compile_body(children, "value")
        return Res(v.stmts + [self.at(ast.Return(v.expr), items[0])], const(None))

    def sf_yield(self, items, children, ctx, **kw):
        if len(items) > 1 and isinstance(items[1], Sym) and items[1].name == "from":
            v = self.compile_expr(items[2:], "value", children)
            return Res(v.stmts, self.at(ast.YieldFrom(v.expr), items[0]))
        if len(items) == 1:
            return Res(expr=self.at(ast.Yield(None), items[0]))
        v = self.compile_expr(items[1:], "value", children)
        return Res(v.stmts, self.at(ast.Yield(v.expr), items[0]))

    def sf_await(self, items, children, ctx, **kw):
        v = self.compile_expr(items[1:], "value", children)
        return Res(v.stmts, self.at(ast.Await(v.expr), items[0]))

    def name_slot(self, items):
        """Where Python expects a class or name: a bareword is the name."""
        if len(items) == 1 and isinstance(items[0], Sym) and DOTTED.match(items[0].name) and items[0].name not in ("True", "False", "None"):
            return Res(expr=self.name_load(items[0]))
        if len(items) == 1 and isinstance(items[0], Group) and items[0].kind == "(" and any(isinstance(x, Comma) for x in items[0].items):
            parts = [p for p in _split(list(items[0].items), lambda x: isinstance(x, Comma)) if p]
            rs = [self.name_slot(p) for p in parts]
            stmts, exprs = self.gather(rs)
            return Res(stmts, ast.Tuple(exprs, ast.Load()))
        if len(items) > 1 and isinstance(items[0], Sym) and DOTTED.match(items[0].name):
            # (A call: the head's a name even if it isn't known yet.)
            f = Res(expr=self.name_load(items[0]))
            a = self.compile_call_args(items[1:], ())
            stmts, exprs = self.gather([f, a[0]])
            return Res(stmts, call(exprs[0], a[1], a[2]))
        return self.compile_expr(items, "value")

    def sf_raise(self, items, children, ctx, **kw):
        if len(items) == 1:
            return Res([self.at(ast.Raise(None, None), items[0])], const(None))
        rest = items[1:]
        fr = [i for i, x in enumerate(rest) if isinstance(x, Sym) and x.name == "from"]
        cause = None
        if fr:
            c = self.name_slot(rest[fr[0] + 1:])
            rest = rest[:fr[0]]
            cause = c
        e = self.name_slot(rest)
        stmts = e.stmts + (cause.stmts if cause else [])
        return Res(stmts + [self.at(ast.Raise(e.expr, cause.expr if cause else None), items[0])], const(None))

    def sf_assert(self, items, children, ctx, **kw):
        rest = items[1:]
        msg = None
        if any(isinstance(x, Comma) for x in rest):
            parts = _split(rest, lambda x: isinstance(x, Comma))
            rest, msg_items = parts[0], parts[1]
            msg = self.compile_expr(msg_items, "value")
        elif len(rest) == 2 and not _has_binop(rest):
            msg = self.compile_value(rest[1])
            rest = rest[:1]
        c = self.compile_expr(rest, "value")
        stmts = c.stmts + (msg.stmts if msg else [])
        return Res(stmts + [self.at(ast.Assert(c.expr, msg.expr if msg else None), items[0])], const(None))

    def sf_del(self, items, children, ctx, **kw):
        rs = [self.make_target([x], items[0], ast.Del) for x in items[1:] if not isinstance(x, Comma)]
        stmts, exprs = self.gather(rs)
        return Res(stmts + [self.at(ast.Delete(exprs), items[0])], const(None))

    def sf_global(self, items, children, ctx, **kw):
        names = [x.name for x in items[1:] if isinstance(x, Sym)]
        for n in names:
            self.scope.names[(n, frozenset())] = n
        return Res([ast.Global(names)], const(None))

    def sf_nonlocal(self, items, children, ctx, **kw):
        names = [x.name for x in items[1:] if isinstance(x, Sym)]
        for n in names:
            self.scope.names[(n, frozenset())] = n
        return Res([ast.Nonlocal(names)], const(None))

    def sf_import(self, items, children, ctx, **kw):
        out = []
        rest = list(items[1:])
        k = 0
        while k < len(rest):
            x = rest[k]
            if not isinstance(x, Sym) or not DOTTED.match(x.name):
                raise self.err("import takes module names", x)
            alias = None
            if k + 2 < len(rest) + 1 and k + 1 < len(rest) and isinstance(rest[k + 1], Sym) and rest[k + 1].name == "as":
                if k + 2 >= len(rest):
                    raise self.err("as needs a name", rest[k + 1])
                alias = rest[k + 2]
                k += 3
            else:
                k += 1
            if alias is not None:
                out.append(ast.alias(x.name, self.bind(alias)))
            else:
                py = self.bind(Sym(x.name.split(".")[0], x.marks, pos=x.pos))
                if py != x.name.split(".")[0]:
                    raise self.err("a macro can't import a dotted name without as", x)
                out.append(ast.alias(x.name, None))
        if not out:
            raise self.err("import what?", items[0])
        return Res([self.at(ast.Import(out), items[0])], const(None))

    def sf_from(self, items, children, ctx, **kw):
        if len(items) < 4 or not isinstance(items[2], Sym) or items[2].name != "import":
            raise self.err("from module import names…", items[0])
        mod = items[1]
        mtext = mod.name if isinstance(mod, Sym) else F.show(mod)
        level = len(mtext) - len(mtext.lstrip("."))
        module = mtext.lstrip(".") or None
        names = []
        rest = list(items[3:])
        if children:
            for c in children:
                rest += list(c.items)
        k = 0
        while k < len(rest):
            x = rest[k]
            if isinstance(x, Comma):
                k += 1
                continue
            if isinstance(x, Group) and x.kind == "(":
                rest[k:k + 1] = list(x.items)
                continue
            if isinstance(x, Op) and x.name == "*":
                names.append(ast.alias("*", None))
                k += 1
                continue
            if not isinstance(x, Sym):
                raise self.err("import takes names", x)
            if k + 1 < len(rest) and isinstance(rest[k + 1], Sym) and rest[k + 1].name == "as":
                names.append(ast.alias(x.name, self.bind(rest[k + 2])))
                k += 3
            else:
                py = self.bind(x)
                names.append(ast.alias(x.name, None if py == x.name else py))
                k += 1
        return Res([self.at(ast.ImportFrom(module, names, level), items[0])], const(None))

    def sf_do(self, items, children, ctx, **kw):
        if len(items) > 1:
            lines = [Line(tuple(items[1:]), pos=items[1].pos)] + list(children)
        else:
            lines = list(children)
        return self.compile_body(lines, ctx)

    def _cond(self, items, form):
        if not items:
            raise self.err("a condition is missing", form)
        return self.compile_expr(items, "cond")

    def _branch_ctx(self, ctx):
        return "stmt" if ctx == "stmt" else ("tail" if ctx == "tail" else "value")

    def sf_if(self, items, children, ctx, conts=(), **kw):
        head = items[0]
        then = [i for i, x in enumerate(items) if isinstance(x, Sym) and x.name == "then"]
        if then and not children:
            k = then[0]
            els = [i for i, x in enumerate(items) if isinstance(x, Sym) and x.name == "else" and i > k]
            cond = self._cond(items[1:k], head)
            a_items = items[k + 1:els[0]] if els else items[k + 1:]
            a = self.compile_expr(a_items, self._branch_ctx(ctx))
            b = self.compile_expr(items[els[0] + 1:], self._branch_ctx(ctx)) if els else Res(expr=const(None))
            if not a.stmts and not b.stmts:
                return Res(cond.stmts, ast.IfExp(cond.expr, a.expr, b.expr))
            t = self.tmp("if")
            node = ast.If(cond.expr, a.stmts + [ast.Assign([name(t, True)], a.expr)], b.stmts + [ast.Assign([name(t, True)], b.expr)])
            return Res(cond.stmts + [node], name(t))
        branches = [(items[1:], children, head)]
        else_body = None
        for c in conts:
            cn = c.items[0].name
            if cn == "elif":
                branches.append((list(c.items[1:]), c.children, c.items[0]))
            else:
                else_body = c.children if len(c.items) == 1 else [Line(tuple(c.items[1:]), c.children, pos=c.pos)]
        bctx = self._branch_ctx(ctx)
        t = None if ctx == "stmt" else self.tmp("if")

        def body(lines):
            r = self.compile_body(lines, bctx)
            if t is None:
                return self.as_stmts(r) or [ast.Pass()]
            return r.stmts + [ast.Assign([name(t, True)], r.expr)]

        orelse = body(else_body) if else_body is not None else ([] if t is None else [ast.Assign([name(t, True)], const(None))])
        pre = []
        for cond_items, lines, h in reversed(branches):
            c = self._cond(cond_items, h)
            node = self.at(ast.If(c.expr, body(lines), orelse), h)
            if c.stmts and (h is not head):
                orelse = c.stmts + [node]
            elif c.stmts:
                pre = c.stmts
                orelse = [node]
            else:
                orelse = [node]
        return Res(pre + orelse, name(t) if t else const(None))

    def sf_while(self, items, children, ctx, conts=(), **kw):
        c = self._cond(items[1:], items[0])
        body = self.as_stmts(self.compile_body(children, "stmt")) or [ast.Pass()]
        orelse = []
        for k in conts:
            orelse = self.as_stmts(self.compile_body(k.children, "stmt"))
        if c.stmts:
            if orelse:
                raise self.err("while … else needs a simple condition", items[0])
            node = ast.While(const(True), c.stmts + [ast.If(ast.UnaryOp(ast.Not(), c.expr), [ast.Break()], [])] + body, [])
        else:
            node = ast.While(c.expr, body, orelse)
        return Res([self.at(node, items[0])], const(None))

    def sf_for(self, items, children, ctx, conts=(), is_async=False, **kw):
        ins = [i for i, x in enumerate(items) if isinstance(x, Sym) and x.name == "in"]
        if not ins:
            raise self.err("for name in iterable", items[0])
        k = ins[0]
        it = self.compile_expr(items[k + 1:], "value")
        target = self.make_target(items[1:k], items[0])
        body = self.as_stmts(self.compile_body(children, "stmt")) or [ast.Pass()]
        orelse = []
        for c in conts:
            orelse = self.as_stmts(self.compile_body(c.children, "stmt"))
        cls = ast.AsyncFor if is_async else ast.For
        node = cls(target.expr, it.expr, body, orelse)
        return Res(it.stmts + target.stmts + [self.at(node, items[0])], const(None))

    def sf_with(self, items, children, ctx, is_async=False, **kw):
        parts = [p for p in _split(list(items[1:]), lambda x: isinstance(x, Comma)) if p]
        withitems, stmts = [], []
        for p in parts:
            a = [i for i, x in enumerate(p) if isinstance(x, Sym) and x.name == "as"]
            e = self.compile_expr(p[:a[0]] if a else p, "value")
            stmts += e.stmts
            tgt = None
            if a:
                tr = self.make_target(p[a[0] + 1:], items[0])
                stmts += tr.stmts
                tgt = tr.expr
            withitems.append(ast.withitem(e.expr, tgt))
        t = None if ctx == "stmt" else self.tmp("w")
        r = self.compile_body(children, self._branch_ctx(ctx))
        body = (self.as_stmts(r) or [ast.Pass()]) if t is None else r.stmts + [ast.Assign([name(t, True)], r.expr)]
        cls = ast.AsyncWith if is_async else ast.With
        pre = [] if t is None else [ast.Assign([name(t, True)], const(None))]
        return Res(stmts + pre + [self.at(cls(withitems, body), items[0])], name(t) if t else const(None))

    def sf_try(self, items, children, ctx, conts=(), **kw):
        t = None if ctx == "stmt" else self.tmp("try")
        bctx = self._branch_ctx(ctx)

        def body(lines):
            r = self.compile_body(lines, bctx)
            if t is None:
                return self.as_stmts(r) or [ast.Pass()]
            return r.stmts + [ast.Assign([name(t, True)], r.expr)]

        main = body(children)
        handlers, orelse, final = [], [], []
        star = False
        for c in conts:
            cn = c.items[0].name
            if cn in ("except", "except*"):
                star = star or cn == "except*"
                ci = list(c.items[1:])
                a = [i for i, x in enumerate(ci) if isinstance(x, Sym) and x.name == "as"]
                typ = self.name_slot(ci[:a[0]] if a else ci).expr if (ci[:a[0]] if a else ci) else None
                nm = self.bind(ci[a[0] + 1] if isinstance(ci[a[0] + 1], Sym) else ci[a[0] + 1].sym) if a else None
                handlers.append(self.at(ast.ExceptHandler(typ, nm, body(c.children)), c.items[0]))
            elif cn == "else":
                orelse = body(c.children)
            elif cn == "finally":
                final = self.as_stmts(self.compile_body(c.children, "stmt")) or [ast.Pass()]
        if not handlers and not final:
            raise self.err("try needs except or finally", items[0])
        cls = ast.TryStar if star else ast.Try
        pre = [] if t is None else [ast.Assign([name(t, True)], const(None))]
        return Res(pre + [self.at(cls(main, handlers, orelse, final), items[0])], name(t) if t else const(None))

    def sf_async(self, items, children, ctx, conts=(), decos=(), **kw):
        if len(items) < 2 or not isinstance(items[1], Sym) or items[1].name not in ("def", "for", "with"):
            raise self.err("async def, async for or async with", items[0])
        n = items[1].name
        if n == "def":
            return self.sf_def(items[1:], children, ctx, decos=decos, is_async=True)
        if n == "for":
            return self.sf_for(items[1:], children, ctx, conts=conts, is_async=True)
        return self.sf_with(items[1:], children, ctx, is_async=True)

    def parse_params(self, items, form):
        """[(kind, Sym, default Res|None, annotation)] kinds: pos, posonly,
        vararg, kwonly, kwarg."""
        out = []
        star = False
        for x in items:
            if isinstance(x, Op) and x.name == "*":
                star = True
                out.append(("star", None, None, None))
            elif isinstance(x, Op) and x.name == "/":
                out = [("posonly" if k == "pos" else k, s, d, a) for k, s, d, a in out]
            elif isinstance(x, Sym):
                n = x.name
                ann = None
                if ":" in n.lstrip("*") and not n.endswith(":"):
                    n, ann = n.split(":", 1)
                if n.startswith("**"):
                    out.append(("kwarg", Sym(n[2:], x.marks, pos=x.pos), None, ann))
                elif n.startswith("*"):
                    star = True
                    out.append(("vararg", Sym(n[1:], x.marks, pos=x.pos), None, ann))
                else:
                    out.append(("kwonly" if star else "pos", Sym(n, x.marks, pos=x.pos), None, ann))
            elif isinstance(x, KwArg):
                out.append(("kwonly" if star else "pos", Sym(x.name, pos=x.pos), self.compile_value(x.value), None))
            elif isinstance(x, Splat) and isinstance(x.value, Var):
                s = x.value.sym
                if x.n == 1:
                    star = True
                out.append(("vararg" if x.n == 1 else "kwarg", s, None, None))
            elif isinstance(x, Var) and isinstance(x.sym, Sym):
                out.append(("kwonly" if star else "pos", x.sym, None, None))
            else:
                raise self.err(f"'{F.show(x)}' isn't a parameter", x)
        return out

    def build_arguments(self, params):
        """In the function's scope: bind the parameters, make ast.arguments."""
        posonly, args, defaults, kwonly, kw_defaults = [], [], [], [], []
        vararg = kwarg = None
        stmts = []
        for kind, s, d, ann in params:
            if kind == "star":
                continue
            a = ast.arg(self.bind(s), self._annotation(ann))
            if kind in ("pos", "posonly"):
                (posonly if kind == "posonly" else args).append(a)
                if d is not None:
                    stmts += d.stmts
                    defaults.append(d.expr)
                elif defaults:
                    raise self.err(f"parameter {s.name} without a default after one with a default", s)
            elif kind == "kwonly":
                kwonly.append(a)
                kw_defaults.append(d.expr if d is not None else None)
                if d is not None:
                    stmts += d.stmts
            elif kind == "vararg":
                vararg = a
            elif kind == "kwarg":
                kwarg = a
        return ast.arguments(posonly, args, vararg, kwonly, kw_defaults, kwarg, defaults), stmts

    def _annotation(self, text):
        if not text:
            return None
        try:
            return ast.parse(text, mode="eval").body
        except SyntaxError:
            raise self.err(f"bad annotation {text}")

    def _is_generator(self, stmts):
        class V(ast.NodeVisitor):
            found = False

            def visit_Yield(self, n):
                self.found = True

            def visit_YieldFrom(self, n):
                self.found = True

            def visit_FunctionDef(self, n):
                pass

            visit_AsyncFunctionDef = visit_FunctionDef
            visit_ClassDef = visit_FunctionDef

            def visit_Lambda(self, n):
                pass

        v = V()
        for s in stmts:
            v.visit(s)
        return v.found

    def function_body(self, children, inline=None):
        self.prescan(children, self.scope)
        if inline is not None:
            r = self.compile_items(inline, (), "tail")
            if children:
                raise self.err("a one-line def has no block", children[0])
        else:
            r = self.compile_body(children, "tail")
        if self._is_generator(r.stmts + ([ast.Expr(r.expr)] if r.expr is not None else [])):
            return self.as_stmts(r) or [ast.Pass()]
        return r.stmts + [ast.Return(r.expr)]

    def sf_def(self, items, children, ctx, decos=(), is_async=False, **kw):
        head = items[0]
        if len(items) < 2 or not isinstance(items[1], Sym):
            raise self.err("def name parameters…", head)
        nsym = items[1]
        rest = list(items[2:])
        inline = None
        eq = [i for i, x in enumerate(rest) if isinstance(x, Op) and x.name == "=" and not x.tick]
        if eq:
            inline = rest[eq[0] + 1:]
            rest = rest[:eq[0]]
        returns = None
        arrow = [i for i, x in enumerate(rest) if isinstance(x, Op) and x.name == "->"]
        if arrow:
            returns = self.name_slot(rest[arrow[0] + 1:]).expr
            rest = rest[:arrow[0]]
        deco_rs = [self.decorator(d) for d in decos]
        params = self.parse_params(rest, head)
        pyname = self.bind(nsym)
        with self.new_scope("function"):
            arguments, dstmts = self.build_arguments(params)
            body = self.function_body(children, inline)
        stmts, deco_es = self.gather(deco_rs)
        cls = ast.AsyncFunctionDef if is_async else ast.FunctionDef
        node = cls(pyname, arguments, body, deco_es, returns, None, [])
        return Res(stmts + dstmts + [self.at(node, head)], name(pyname))

    def decorator(self, line):
        items = list(line.items)
        h = items[0]
        if isinstance(h, Sym):
            items[0] = Sym(h.name[1:], h.marks, pos=h.pos)
        if items[0].name == "command" and len(items) == 1:
            return Res(expr=rt("command"))
        return self.name_slot(items)

    def sf_fn(self, items, children, ctx, **kw):
        head = items[0]
        if len(items) < 2 or not isinstance(items[1], Group) or items[1].kind != "[":
            raise self.err("fn [parameters] body", head)
        params = self.parse_params(list(items[1].items), head)
        body_items = list(items[2:])
        with self.new_scope("function"):
            arguments, dstmts = self.build_arguments(params)
            if body_items and not children:
                r = self.compile_items(body_items, (), "tail")
                if not r.stmts:
                    lam = ast.Lambda(arguments, r.expr)
                    return Res(dstmts, self.at(lam, head))
                body = r.stmts + [ast.Return(r.expr)]
            else:
                lines = ([Line(tuple(body_items), pos=body_items[0].pos)] if body_items else []) + list(children)
                body = self.function_body(lines)
        t = self.tmp("fn")
        node = ast.FunctionDef(t, arguments, body, [], None, None, [])
        return Res(dstmts + [self.at(node, head)], name(t))

    sf_lambda = sf_fn

    def sf_class(self, items, children, ctx, decos=(), **kw):
        head = items[0]
        if len(items) < 2 or not isinstance(items[1], Sym):
            raise self.err("class Name bases…", head)
        deco_rs = [self.decorator(d) for d in decos]
        bases, kws, rs = [], [], []
        for x in items[2:]:
            if isinstance(x, KwArg):
                v = self.name_slot([x.value])
                rs.append(v)
                kws.append(x.name)
            else:
                rs.append(self.name_slot([x]))
                kws.append(None)
        stmts, exprs = self.gather(deco_rs + rs)
        deco_es, base_es = exprs[:len(deco_rs)], exprs[len(deco_rs):]
        pyname = self.bind(items[1])
        with self.new_scope("class"):
            self.prescan(children, self.scope)
            body = self.as_stmts(self.compile_body(children, "stmt")) or [ast.Pass()]
        node = ast.ClassDef(pyname, [e for e, k in zip(base_es, kws) if k is None],
                            [ast.keyword(k, e) for e, k in zip(base_es, kws) if k is not None], body, deco_es, [])
        return Res(stmts + [self.at(node, head)], name(pyname))

    # ---- match ----------------------------------------------------------------

    def sf_match(self, items, children, ctx, **kw):
        subj = self.compile_expr(items[1:], "value")
        t = None if ctx == "stmt" else self.tmp("m")
        bctx = self._branch_ctx(ctx)
        cases = []
        for c in children:
            ci = list(c.items)
            if not ci or not isinstance(ci[0], Sym) or ci[0].name != "case":
                raise self.err("match takes case lines", c)
            pat_items = ci[1:]
            guard = None
            g = [i for i, x in enumerate(pat_items) if isinstance(x, Sym) and x.name == "if"]
            if g:
                guard = self.compile_expr(pat_items[g[0] + 1:], "value").expr
                pat_items = pat_items[:g[0]]
            pat = self.pattern(pat_items, c)
            r = self.compile_body(c.children, bctx)
            body = (self.as_stmts(r) or [ast.Pass()]) if t is None else r.stmts + [ast.Assign([name(t, True)], r.expr)]
            cases.append(ast.match_case(pat, guard, body))
        pre = [] if t is None else [ast.Assign([name(t, True)], const(None))]
        return Res(subj.stmts + pre + [self.at(ast.Match(subj.expr, cases), items[0])], name(t) if t else const(None))

    def pattern(self, items, form):
        items = list(items)
        if not items:
            raise self.err("a pattern is missing", form)
        alts = _split(items, lambda x: isinstance(x, Op) and x.name == "|")
        if len(alts) > 1:
            return ast.MatchOr([self.pattern(a, form) for a in alts])
        a = [i for i, x in enumerate(items) if isinstance(x, Sym) and x.name == "as"]
        if a:
            inner = self.pattern(items[:a[0]], form)
            tgt = items[a[0] + 1]
            s = tgt.sym if isinstance(tgt, Var) else tgt
            return ast.MatchAs(inner, self.bind(s))
        if len(items) > 1:
            head = items[0]
            if not isinstance(head, Sym) or not DOTTED.match(head.name):
                raise self.err("a class pattern starts with the class", head)
            return self._class_pattern(head, items[1:])
        x = items[0]
        if isinstance(x, Sym):
            if x.name == "_":
                return ast.MatchAs(None, None)
            if x.name in ("None", "True", "False"):
                return ast.MatchSingleton({"None": None, "True": True, "False": False}[x.name])
            if "." in x.name and DOTTED.match(x.name):
                return ast.MatchValue(self.name_load(x))
            return ast.MatchValue(const(x.name))
        if isinstance(x, Var):
            if x.sym.name == "_":
                return ast.MatchAs(None, None)
            return ast.MatchAs(None, self.bind(x.sym))
        if isinstance(x, (Num, Str)):
            return ast.MatchValue(const(x.value))
        if isinstance(x, Unary) and x.op == "-" and isinstance(x.operand, Num):
            return ast.MatchValue(ast.UnaryOp(ast.USub(), const(x.operand.value)))
        if isinstance(x, Splat) and x.n == 1:
            s = x.value.sym if isinstance(x.value, Var) else None
            return ast.MatchStar(None if s is None or s.name == "_" else self.bind(s))
        if isinstance(x, Group) and x.kind == "[":
            sub = list(x.items)
            parts = [p for p in _split(sub, lambda y: isinstance(y, Comma)) if p] if any(isinstance(y, Comma) for y in sub) else [[y] for y in sub]
            return ast.MatchSequence([self.pattern(p, form) for p in parts])
        if isinstance(x, Group) and x.kind == "(":
            sub = list(x.items)
            if not sub:
                return ast.MatchSequence([])
            if isinstance(sub[0], Sym) and DOTTED.match(sub[0].name) and (len(sub) > 1 or sub[0].name[:1].isupper() or sub[0].name in PY_BUILTINS):
                return self._class_pattern(sub[0], sub[1:])
            return self.pattern(sub, form)
        if isinstance(x, Group) and x.kind == "{":
            keys, pats, rest = [], [], None
            sub = [y for y in x.items if not isinstance(y, Comma)]
            k = 0
            while k < len(sub):
                y = sub[k]
                if isinstance(y, Splat) and y.n == 2:
                    rest = self.bind(y.value.sym)
                    k += 1
                    continue
                kr = self._key(y)
                if kr is None or k + 1 >= len(sub):
                    raise self.err("a mapping pattern is {key: pattern …}", y)
                keys.append(kr.expr)
                pats.append(self.pattern([sub[k + 1]], form))
                k += 2
            return ast.MatchMapping(keys, pats, rest)
        if isinstance(x, Sym) or isinstance(x, Attr):
            return ast.MatchValue(self.compile_value(x).expr)
        raise self.err(f"'{F.show(x)}' isn't a pattern", x)

    def _class_pattern(self, head, args):
        pos, kwn, kwp = [], [], []
        for y in args:
            if isinstance(y, KwArg):
                kwn.append(y.name)
                kwp.append(self.pattern([y.value], y))
            else:
                pos.append(self.pattern([y], y))
        return ast.MatchClass(self.name_load(head), pos, kwn, kwp)

    # ---- quoting and macros ---------------------------------------------------

    def sf_quote(self, items, children, ctx, **kw):
        if len(items) == 2 and not children:
            return Res(expr=self.quote_ast(items[1], quasi=False))
        if children and len(items) == 1:
            ls = [self.quote_ast(c, quasi=False) for c in children]
            return Res(expr=ls[0] if len(ls) == 1 else ast.List(ls, ast.Load()))
        return Res(expr=self.quote_ast(Line(tuple(items[1:]), tuple(children), pos=items[1].pos if len(items) > 1 else None), quasi=False))

    def sf_quasi(self, items, children, ctx, **kw):
        if len(items) == 1:
            if not children:
                raise self.err("quasi needs a template", items[0])
            ls = [self.quote_ast(c, quasi=True) for c in children]
            return Res(expr=ls[0] if len(ls) == 1 else ast.List(ls, ast.Load()))
        if len(items) == 2 and not children:
            return Res(expr=self.quote_ast(_group_form(items[1]), quasi=True))
        return Res(expr=self.quote_ast(Line(tuple(items[1:]), tuple(children), pos=items[1].pos), quasi=True))

    def _unquote_value(self, u: Unquote):
        f = u.form
        if isinstance(f, Sym):
            return self.name_load(f)
        if isinstance(f, Group) and f.kind == "(":
            return self.compile_group(f, "value").expr
        return self.compile_value(f).expr

    def quote_ast(self, form, quasi):
        """Python that builds `form` (for quasi: with its unquotes filled in)."""
        from dataclasses import fields

        def fm(attr):
            return ast.Attribute(name(FM), attr, ast.Load())

        def build(x):
            if quasi and isinstance(x, Unquote):
                if x.splice:
                    raise self.err("~@ splices into a list of items", x)
                return call(fm("lift"), [self._unquote_value(x)])
            if quasi and isinstance(x, Var) and isinstance(x.sym, Unquote):
                return call(fm("Var"), [call(fm("as_sym"), [self._unquote_value(x.sym)])])
            if isinstance(x, Form):
                args, kws = [], []
                for fl in fields(x):
                    if fl.name == "pos":
                        continue
                    v = getattr(x, fl.name)
                    if fl.name == "marks":
                        continue
                    if isinstance(x, Line) and fl.name == "children":
                        args.append(seq(v, lines=True))
                    else:
                        args.append(value(v))
                if x.pos is not None:
                    kws.append(ast.keyword("pos", const(x.pos)))
                return call(fm(type(x).__name__), args, kws)
            return value(x)

        def value(v):
            if isinstance(v, Form):
                return build(v)
            if isinstance(v, tuple):
                return seq(v)
            return const(v)

        def seq(v, lines=False):
            elts = []
            for y in v:
                if quasi and isinstance(y, Unquote) and y.splice:
                    elts.append(ast.Starred(call(fm("as_lines" if lines else "lift_many"), [self._unquote_value(y)]), ast.Load()))
                elif quasi and lines and isinstance(y, Line) and len(y.items) == 1 and isinstance(y.items[0], Unquote) and y.items[0].splice and not y.children:
                    elts.append(ast.Starred(call(fm("as_lines"), [self._unquote_value(y.items[0])]), ast.Load()))
                elif quasi and lines and isinstance(y, Line) and len(y.items) == 1 and isinstance(y.items[0], Unquote) and not y.children:
                    elts.append(ast.Starred(call(fm("as_lines"), [self._unquote_value(y.items[0])]), ast.Load()))
                else:
                    elts.append(build(y))
            return ast.Tuple(elts, ast.Load())

        return build(form)

    def macro_ns(self):
        if self.ns is not None:
            return self.ns
        if self._scratch is None:
            from .runner import new_namespace
            self._scratch = new_namespace()
        return self._scratch

    def sf_defmacro(self, items, children, ctx, **kw):
        head = items[0]
        if len(items) < 2 or not isinstance(items[1], Sym):
            raise self.err("defmacro name parameters…", head)
        mname = items[1].name
        params = []
        for x in items[2:]:
            if not isinstance(x, Sym):
                raise self.err("macro parameters are names, *rest or name:expr", x)
            if x.name.startswith("*"):
                params.append(("rest", x.name[1:]))
            elif x.name.endswith(":expr"):
                params.append(("expr", x.name[:-5]))
            else:
                params.append(("one", x.name))
        fake = [Sym(("*" if k == "rest" else "") + n, pos=head.pos) for k, n in params]
        pyname = f"_tessh_macro_{mname}"
        saved = self.scope
        self.scope = Scope("module")
        try:
            with self.new_scope("function"):
                arguments, _ = self.build_arguments(self.parse_params(fake, head))
                body = self.function_body(children)
        finally:
            self.scope = saved
        fn_ast = ast.FunctionDef(pyname, arguments, body, [], None, None, [])
        self.at(fn_ast, head)
        mod = ast.Module([fn_ast], [])
        ast.fix_missing_locations(mod)
        ns = self.macro_ns()
        try:
            exec(compile(mod, "<macro " + mname + ">", "exec"), ns)
        except SyntaxError as e:
            raise self.err(f"in macro {mname}: {e.msg}", head)
        self.macros[mname] = Macro(mname, ns[pyname], params)
        return Res(expr=const(None))

    def expand_macro(self, m: Macro, items, children, ctx):
        head = items[0]
        args = self.macro_args(m, items[1:], children, head)
        mark = next(self._marks)
        args = [F.toggle_mark(a, mark) for a in args]
        head_t = F.toggle_mark(head, mark)
        saved = F.current_head[0]
        F.current_head[0] = head_t
        self.depth += 1
        if self.depth > 200:
            raise self.err(f"macro {m.name} expands forever", head)
        try:
            out = m.fn(*args)
        except CompileError:
            raise
        except Exception as e:
            raise self.err(f"macro {m.name}: {type(e).__name__}: {e}", head) from e
        finally:
            F.current_head[0] = saved
        try:
            if isinstance(out, (list, tuple)) and not isinstance(out, Form):
                out = [F.lift(x) for x in out]
            else:
                out = F.lift(out)
            out = F.toggle_mark(out, mark)
            out = _positioned(out, head.pos)
            if isinstance(out, list):
                return self.compile_body(F.as_lines(out), ctx)
            if isinstance(out, Line):
                return self.compile_line(out, ctx)
            return self.compile_items([out], (), ctx)
        finally:
            self.depth -= 1

    def macro_args(self, m, inline, children, head):
        inline = list(inline)
        rest_children = list(children)
        args = []
        for k, (kind, n) in enumerate(m.params):
            if kind == "one":
                if inline:
                    args.append(inline.pop(0))
                elif rest_children:
                    c = rest_children.pop(0)
                    args.append(c.items[0] if len(c.items) == 1 and not c.children else c)
                else:
                    raise self.err(f"macro {m.name} needs {n}", head)
            elif kind == "expr":
                if not inline:
                    raise self.err(f"macro {m.name} needs {n}", head)
                args.append(inline[0] if len(inline) == 1 else Group("(", tuple(inline), pos=inline[0].pos))
                inline = []
            else:
                args += inline + rest_children
                inline, rest_children = [], []
        if inline or rest_children:
            raise self.err(f"macro {m.name} got too many arguments", (inline or rest_children)[0])
        return args

    def sf_defop(self, items, children, ctx, **kw):
        if len(items) != 5:
            raise self.err("defop OP precedence associativity function", items[0])
        op, prec, assoc, fn = items[1:]
        if not isinstance(op, Op):
            raise self.err("an operator is made of + - * / < > = ! & | % ^ ~ @ ? : .", op)
        if not isinstance(prec, Num) or not 1 <= prec.value <= 13:
            raise self.err("precedence is 1 to 13", prec)
        if not isinstance(assoc, Sym) or assoc.name not in ("left", "right", "none"):
            raise self.err("associativity is left, right or none", assoc)
        if not isinstance(fn, Sym):
            raise self.err("the function is a name", fn)
        self.ops[op.name] = (prec.value, assoc.name, fn)
        return Res(expr=const(None))

    # ---- whole programs -------------------------------------------------------

    def compile_unit_module(self, unit, ctx="stmt", show=False):
        r = self.compile_unit(unit, ctx)
        body = list(r.stmts)
        if show:
            body.append(ast.Expr(call(rt("show"), [r.expr if r.expr is not None else const(None)])))
        elif r.expr is not None and not _pure(r.expr):
            body.append(ast.Expr(r.expr))
        if not body:
            body = [ast.Pass()]
        l = unit[0]
        for s in body:
            if l.pos and not hasattr(s, "lineno"):
                self.at(s, l)
        mod = ast.Module(body, [])
        ast.fix_missing_locations(mod)
        return mod

    def compile_module(self, lines):
        self.prescan(lines, self.scope)
        body = []
        for u in self.units(lines):
            body += self.compile_unit_module(u).body
        mod = ast.Module(body or [ast.Pass()], [])
        ast.fix_missing_locations(mod)
        return mod


# ---- small helpers ---------------------------------------------------------------

def _noargs():
    return ast.arguments([], [], None, [], [], None, [])


def _split(items, pred):
    out, cur = [], []
    for x in items:
        if pred(x):
            out.append(cur)
            cur = []
        else:
            cur.append(x)
    out.append(cur)
    return out


def _top_for(items):
    for k, x in enumerate(items):
        if k > 0 and isinstance(x, Sym) and x.name == "for":
            return k
    return 0


def _has_binop(items):
    for k, x in enumerate(items):
        if isinstance(x, Op) and (x.name in BINOPS or x.name in CMPOPS) and 0 < k < len(items) - 1:
            return True
    return False


def _piece(text, form):
    from .reader import NUMBER, _number
    if NUMBER.match(text):
        return Num(_number(text), text, pos=form.pos)
    return Sym(text, pos=form.pos)


def _group_form(g):
    """:(a b c) quasiquotes the form a b c (a Line), not the group."""
    if isinstance(g, Group) and g.kind == "(":
        return Line(g.items, (), pos=g.pos)
    return g


def _positioned(form, pos):
    """Forms a macro made up take the macro call's position."""
    if pos is None:
        return form

    def f(x):
        if isinstance(x, Form) and x.pos is None:
            from dataclasses import replace
            return replace(x, pos=pos)
        return x
    return F.map_forms(f, form)
