"""The tessh reader, made with parsy (a parser-combinator library).

Two parsers: one over characters that reads each logical line's items
(brackets may span lines), and one over the list of lines that makes blocks
from their indentation. `read(text)` gives a list of Line forms.
"""

from __future__ import annotations

import bisect
import re
from dataclasses import dataclass, replace

import parsy as P
from parsy import generate, regex, string

from .forms import (Attr, Caret, Comma, Form, Group, Index, Interp, IStr, KwArg, Line, Num, Op,
                    Quasi, Redir, Semi, Splat, Status, Str, Sym, SymLit, Unary, Unquote, Var, Word,
                    map_forms)


class ReadError(SyntaxError):
    """A syntax error at (line, col) (line from 1, col from 0)."""

    def __init__(self, msg, line=1, col=0, end_col=None):
        super().__init__(msg)
        self.msg = msg
        self.line = line
        self.col = col
        self.end_col = end_col if end_col is not None else col + 1

    def __str__(self):
        return f"{self.line}:{self.col + 1}: {self.msg}"


IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*\Z")
NUMBER = re.compile(
    r"[-+]?(?:0[xX](?:_?[0-9a-fA-F])+|0[oO](?:_?[0-7])+|0[bB](?:_?[01])+"
    r"|(?:(?:[0-9](?:_?[0-9])*)?\.[0-9](?:_?[0-9])*|[0-9](?:_?[0-9])*\.?)(?:[eE][-+]?[0-9](?:_?[0-9])*)?[jJ]?)\Z"
)
OPCHARS = set("+-*/<>=!&|%^~@?:.")
# Operator-character tokens that are words: paths and the like.
NOT_OPS = {"~", "?", ":", "@@"}
REDIR_WHOLE = re.compile(r"(?:[012]?>&[012]|&>&?[012])\Z")
REDIR_OP = re.compile(r"(?:[12]?>>|[12]?>|&>>|&>|<)(?![<>=&])")
SPACED_REDIRS = {">", ">>", "<", "1>", "1>>", "2>", "2>>", "&>", "&>>"}

WS_CHARS = " \t\r\n"
CLOSERS = ")]}"


def _pos(start, end):
    return (start, end)


# ---- characters ----------------------------------------------------------------

ws = regex(r"(?:[ \t]|\\\r?\n)+").desc("space")
gws = regex(r"(?:\s|#[^\n]*|\\\r?\n)+").desc("space")
comment = regex(r"#[^\n]*")
newline = regex(r"\r?\n")
ident = regex(r"[A-Za-z_][A-Za-z0-9_]*")
peek_char = P.peek(P.any_char).optional()

ESCAPES = {"n": "\n", "t": "\t", "r": "\r", "0": "\0", "a": "\a", "b": "\b", "f": "\f",
           "v": "\v", "\\": "\\", '"': '"', "'": "'", "$": "$", "`": "`", "e": "\x1b"}


def items_of(sep):
    """Items separated by `sep` (terms, commas and semicolons)."""
    @generate
    def p():
        out = []
        yield sep.optional()
        while True:
            c = yield peek_char
            if c is None or c in CLOSERS or c in "\r\n" and sep is ws:
                break
            if c == "#":
                if sep is ws:
                    break
                yield gws
                continue
            if c == ",":
                start = yield P.index
                yield string(",")
                out.append(Comma(pos=_pos(start, start + 1)))
            elif c == ";":
                start = yield P.index
                yield string(";")
                out.append(Semi(pos=_pos(start, start + 1)))
            else:
                t = yield term
                out.append(t)
            got = yield sep.optional()
            if got is None:
                c = yield peek_char
                if c is not None and c not in CLOSERS and c not in ",;\r\n#":
                    yield P.fail("space")
        return out
    return p


line_items = P.forward_declaration()
group_items = P.forward_declaration()


def group(open_, close):
    @generate
    def p():
        start = yield P.index
        yield string(open_)
        items = yield group_items
        yield gws.optional()
        yield string(close).desc(f"'{close}' to close the '{open_}'")
        end = yield P.index
        return Group(open_, tuple(items), pos=_pos(start, end))
    return p


paren = group("(", ")")
bracket = group("[", "]")
brace = group("{", "}")


def with_postfix(form_parser):
    """A Var or Group, then any glued .name and [items]."""
    return form_parser.bind(postfix_after)


def postfix_after(f):
    @generate
    def p():
        nonlocal f
        if not isinstance(f, (Var, Group, Attr, Index, Str, IStr)):
            return f
        while True:
            c = yield peek_char
            if c == ".":
                at = yield P.peek(regex(r"\.[A-Za-z_]")).optional()
                if at is None:
                    break
                yield string(".")
                name = yield ident
                end = yield P.index
                f = Attr(f, name, pos=_pos(f.pos[0], end))
            elif c == "[":
                g = yield bracket
                f = Index(f, g.items, pos=_pos(f.pos[0], g.pos[1]))
            else:
                break
        return f
    return p


@generate
def braced_interp():
    """${expr!conv:spec}: the inside read on its own."""
    start = yield P.index
    yield string("${")
    body_start = yield P.index
    body = yield _balanced_body("}")
    yield string("}").desc("'}' to close the '${'")
    end = yield P.index
    expr_text, conv, spec = _split_format(body)
    try:
        items = (group_items << gws.optional()).parse(expr_text)
    except P.ParseError as e:
        raise ReadError(f"in ${{…}}: expected {_expected(e)}", 1, 0) from None
    items = _shift(items, body_start)
    g = Group("(", tuple(items), pos=_pos(body_start, body_start + len(expr_text)))
    return Interp(g, conv, spec, True, pos=_pos(start, end))


def _balanced_body(close):
    @P.Parser
    def p(stream, index):
        depth, i, quote = 0, index, None
        while i < len(stream):
            ch = stream[i]
            if quote:
                if ch == "\\":
                    i += 2
                    continue
                if ch == quote:
                    quote = None
            elif ch in "\"'":
                quote = ch
            elif ch in "([{":
                depth += 1
            elif ch in ")]}":
                if depth == 0:
                    if ch == close:
                        return P.Result.success(i, stream[index:i])
                    break
                depth -= 1
            i += 1
        return P.Result.failure(i, f"'{close}'")
    return p


def _split_format(body: str):
    """'expr!r:>10' -> ('expr', 'r', '>10'), at the top level only."""
    depth, quote, conv, spec, cut = 0, None, None, None, len(body)
    for i, ch in enumerate(body):
        if quote:
            if ch == quote:
                quote = None
            continue
        if ch in "\"'":
            quote = ch
        elif ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        elif depth == 0 and ch == ":" and (i + 1 == len(body) or body[i + 1] not in "="):
            spec = body[i + 1:]
            cut = i
            break
    expr = body[:cut]
    m = re.search(r"!([rsa])\s*\Z", expr)
    if m:
        conv = m.group(1)
        expr = expr[:m.start()]
    return expr, conv, spec


@generate
def dollar():
    """What follows a $: a variable, $?, $(form), ${expr} or $~unquote; else
    a plain '$'."""
    start = yield P.index
    two = yield P.peek(regex(r"\$.?", re.S)).optional()
    nxt = two[1:] if two else ""
    if nxt == "?":
        yield string("$?")
        return Status(pos=_pos(start, start + 2))
    if nxt == "(":
        yield string("$")
        g = yield paren
        return Interp(g, pos=_pos(start, g.pos[1]))
    if nxt == "{":
        return (yield braced_interp)
    if nxt == "~":
        yield string("$")
        u = yield unquote
        return Var(u, pos=_pos(start, u.pos[1]))
    if nxt and (nxt.isalpha() or nxt == "_"):
        yield string("$")
        name = yield ident
        end = yield P.index
        return Var(Sym(name, pos=_pos(start + 1, end)), pos=_pos(start, end))
    yield string("$")
    return "$"


dollar_piece = dollar.bind(postfix_after)
var_with_postfix = dollar.bind(lambda v: postfix_after(v) if isinstance(v, Var) else P.fail("variable"))


@generate
def unquote():
    start = yield P.index
    yield string("~")
    splice = yield string("@").optional()
    c = yield peek_char
    if c == "(":
        inner = yield with_postfix(paren)
    elif c == "$":
        inner = yield var_with_postfix
    else:
        name = yield ident
        end = yield P.index
        inner = Sym(name, pos=_pos(end - len(name), end))
    end = yield P.index
    return Unquote(inner, bool(splice), pos=_pos(start, end))


@generate
def string_lit():
    start = yield P.index
    prefix = (yield regex(r"(?:[bB][rR]?|[rR][bB]?)?(?=[\"'])")).lower()
    quote = yield regex(r'"""|\'\'\'|"|\'')
    raw = "r" in prefix
    is_bytes = "b" in prefix
    q = quote[0]
    parts: list = []
    buf = []
    stream_parser = P.any_char
    while True:
        if len(quote) == 3:
            end_q = yield string(quote).optional()
            if end_q:
                break
        else:
            c = yield peek_char
            if c == q:
                yield string(q)
                break
            if c is None:
                yield P.fail(f"{quote} to close the string")
        c = yield peek_char
        if c is None:
            yield P.fail(f"{quote} to close the string")
        if q == "'" and not is_bytes:
            # (Single quotes: raw, nothing escaped.)
            buf.append((yield stream_parser))
            continue
        if c == "\\" and not raw:
            yield string("\\")
            e = yield P.any_char
            if e == "\n":
                continue
            if e == "x":
                h = yield regex(r"[0-9a-fA-F]{2}")
                buf.append(chr(int(h, 16)))
            elif e == "u":
                h = yield regex(r"[0-9a-fA-F]{4}")
                buf.append(chr(int(h, 16)))
            elif e == "U":
                h = yield regex(r"[0-9a-fA-F]{8}")
                buf.append(chr(int(h, 16)))
            elif e in ESCAPES:
                buf.append(ESCAPES[e])
            else:
                buf.append("\\" + e)
            continue
        if c == "$" and not raw and not is_bytes and q == '"':
            d = yield P.peek(regex(r"\$(?:[A-Za-z_?({~])")).optional()
            if d:
                if d == "$~":
                    buf.append((yield P.any_char))
                    continue
                v = yield dollar_piece
                if buf:
                    parts.append("".join(buf))
                    buf = []
                parts.append(v if isinstance(v, Interp) else Interp(v, pos=v.pos))
                continue
        buf.append((yield stream_parser))
    end = yield P.index
    if buf:
        parts.append("".join(buf))
    if is_bytes:
        text = "".join(p for p in parts if isinstance(p, str))
        return Str(text.encode("latin-1", "backslashreplace"), "b", pos=_pos(start, end))
    if not any(isinstance(p, Interp) for p in parts):
        kind = "r" if raw else q
        return Str("".join(parts), kind, pos=_pos(start, end))
    return IStr(tuple(parts), pos=_pos(start, end))


@generate
def tick_op():
    start = yield P.index
    yield string("`")
    name = yield regex(r"[^`\s]+")
    yield string("`").desc("'`' to close the operator")
    end = yield P.index
    return Op(name, True, pos=_pos(start, end))


RAW = regex(r"[^\s()\[\]{}\"'`;$,]+")


def _literal_brackets(open_, close):
    """A [..] or {..} glued inside a word (a glob, brace text): kept as text."""
    @P.Parser
    def p(stream, index):
        if index >= len(stream) or stream[index] != open_:
            return P.Result.failure(index, open_)
        depth, i = 0, index
        while i < len(stream):
            ch = stream[i]
            if ch == open_:
                depth += 1
            elif ch == close:
                depth -= 1
                if depth == 0:
                    return P.Result.success(i + 1, stream[index:i + 1])
            elif ch in " \t\r\n":
                break
            i += 1
        return P.Result.failure(index, close)
    return p


lit_bracket = _literal_brackets("[", "]")
lit_brace = _literal_brackets("{", "}")


@generate
def term():
    """One item: glued pieces, classified (a word, number, variable, …)."""
    start = yield P.index
    pieces: list = []
    c = yield peek_char
    if c == "`":
        return (yield tick_op)
    while True:
        c = yield peek_char
        if c is None or c in WS_CHARS or c in CLOSERS or c == ";":
            break
        if c == ",":
            sep = yield P.peek(regex(r",(?:[\s)\]}]|\Z)")).optional()
            if sep is not None:
                break
            yield string(",")
            pieces.append(",")
            continue
        prev = pieces[-1] if pieces else None
        if c in "\"'" or (c in "bBrR" and (prev is None or not isinstance(prev, str) or not prev[-1:].isalnum())):
            s = yield string_lit.bind(postfix_after).optional()
            if s is not None:
                pieces.append(s)
                continue
        if c == "$":
            pieces.append((yield dollar_piece))
            continue
        if c == "(":
            pieces.append((yield with_postfix(paren)))
            continue
        if c == "[":
            if prev is None:
                pieces.append((yield with_postfix(bracket)))
            else:
                t = yield lit_bracket.optional()
                if t is None:
                    yield P.fail("']'")
                pieces.append(t)
            continue
        if c == "{":
            if prev is None:
                pieces.append((yield with_postfix(brace)))
            else:
                t = yield lit_brace.optional()
                pieces.append(t if t is not None else (yield string("{")))
            continue
        if c == "~" and prev is None:
            u = yield unquote.optional()
            if u is not None:
                more = yield P.peek(P.any_char).optional()
                if more is None or more in WS_CHARS or more in CLOSERS or more in ",;":
                    return u
                # (~name/…: a path, ~user's home.)
                pieces.append(_show_text(u))
                continue
        if c == "`":
            pieces.append((yield string("`")))
            continue
        pieces.append((yield RAW))
    end = yield P.index
    if not pieces:
        yield P.fail("an item")
    return classify(_merge(pieces), _pos(start, end))


def _show_text(u: Unquote) -> str:
    from .forms import show
    return show(u)


def _merge(pieces):
    out = []
    for p in pieces:
        if isinstance(p, str) and out and isinstance(out[-1], str):
            out[-1] += p
        else:
            out.append(p)
    return out


def classify(pieces: list, pos) -> Form:
    first = pieces[0]
    rest = pieces[1:]
    if len(pieces) == 1 and not isinstance(first, str):
        return first
    if isinstance(first, str):
        t = first
        if not rest:
            if NUMBER.match(t):
                return Num(_number(t), t, pos=pos)
            if REDIR_WHOLE.match(t):
                return Redir(t, pos=pos)
            if set(t) <= OPCHARS and t not in NOT_OPS and set(t) != {"."}:
                return Op(t, pos=pos)
            m = REDIR_OP.match(t)
            if m and m.end() < len(t) and not t.startswith(("<<", "<=", ">=")):
                tgt = t[m.end():]
                return Redir(m.group(0), classify([tgt], (pos[0] + m.end(), pos[1])), pos=pos)
            if t.startswith("^") and len(t) > 1:
                return Caret(classify([t[1:]], (pos[0] + 1, pos[1])), pos=pos)
            if t.startswith(":") and IDENT.match(t[1:]):
                return SymLit(Sym(t[1:], pos=(pos[0] + 1, pos[1])), pos=pos)
            m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)=(?!=)(.+)\Z", t, re.S)
            if m:
                return KwArg(m.group(1), classify([m.group(2)], (pos[0] + m.end(1) + 1, pos[1])), pos=pos)
            return Sym(t, pos=pos)
        # Prefixes glued to a form.
        if len(rest) == 1 and not isinstance(rest[0], str):
            r = rest[0]
            if t in ("-", "+") and isinstance(r, (Var, Group, Attr, Index)):
                return Unary(t, r, pos=pos)
            if t == "~" and isinstance(r, (Var, Attr, Index)):
                return Unary("~", r, pos=pos)
            if t in ("*", "**") and isinstance(r, (Var, Group, Attr, Index)):
                return Splat(r, len(t), pos=pos)
            if t == ":" and isinstance(r, Group) and r.kind == "(":
                return Quasi(r, pos=pos)
            if t == "~@" and isinstance(r, (Group, Var)):
                return Unquote(r, True, pos=pos)
            if t == "^":
                return Caret(r, pos=pos)
        m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)=\Z", t)
        if m:
            value = rest[0] if len(rest) == 1 else Word(tuple(rest), pos=pos)
            return KwArg(m.group(1), value, pos=pos)
        m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)=(?!=)(.+)\Z", t, re.S)
        if m:
            return KwArg(m.group(1), Word((m.group(2), *rest), pos=pos), pos=pos)
        m = REDIR_OP.match(t)
        if m and not t.startswith(("<<", "<=", ">=")):
            tail = [t[m.end():]] if m.end() < len(t) else []
            tgt = tail + rest
            return Redir(m.group(0), tgt[0] if len(tgt) == 1 and not isinstance(tgt[0], str) else Word(tuple(tgt), pos=pos), pos=pos)
        if t.startswith("^") and len(t) > 1:
            return Caret(Word((t[1:], *rest), pos=pos), pos=pos)
    return Word(tuple(pieces), pos=pos)


def _number(t: str):
    s = t.replace("_", "")
    if s.lower().endswith("j"):
        return complex(s)
    try:
        return int(s, 0)
    except ValueError:
        pass
    try:
        return int(s)
    except ValueError:
        return float(s)


line_items.become(items_of(ws))
group_items.become(items_of(gws))


# ---- lines ---------------------------------------------------------------------

@dataclass
class RawLine:
    indent: int
    items: list
    start: int
    end: int


@generate
def logical_line():
    start = yield P.index
    ind = yield regex(r"[ \t]*")
    if "\t" in ind:
        yield P.fail("spaces (tabs can't indent)")
    items = yield line_items
    yield ws.optional()
    yield comment.optional()
    end = yield P.index
    yield newline | P.eof
    return RawLine(len(ind), items, start, end)


@generate
def lines_parser():
    out = []
    while True:
        done = yield P.eof.result(True).optional()
        if done:
            break
        l = yield logical_line
        if l.items:
            out.append(l)
    return out


def _split_semis(lines):
    out = []
    for l in lines:
        cur = []
        for it in l.items:
            if isinstance(it, Semi):
                if cur:
                    out.append(RawLine(l.indent, cur, cur[0].pos[0], cur[-1].pos[1]))
                cur = []
            else:
                cur.append(it)
        if cur:
            out.append(RawLine(l.indent, cur, cur[0].pos[0], cur[-1].pos[1]))
    return out


def _block(level):
    """The lines at indentation `level`, each with the deeper lines under it."""
    @generate
    def one():
        l = yield P.test_item(lambda x: x.indent == level, f"a line at indentation {level}")
        nxt = yield P.peek(P.test_item(lambda x: x.indent > level, "")).optional()
        kids = []
        if nxt is not None:
            kids = yield _block(nxt.indent)
        return Line(tuple(l.items), tuple(kids), pos=_pos(l.items[0].pos[0], l.items[-1].pos[1]))
    return one.at_least(1)


# ---- positions -----------------------------------------------------------------

def _shift(forms, base):
    def f(x):
        if isinstance(x, Form) and x.pos is not None and len(x.pos) == 2:
            return replace(x, pos=(x.pos[0] + base, x.pos[1] + base))
        return x
    return map_forms(f, forms)


class _Lines:
    def __init__(self, text):
        self.starts = [0] + [m.end() for m in re.finditer(r"\n", text)]

    def at(self, i):
        n = bisect.bisect_right(self.starts, i) - 1
        return n + 1, i - self.starts[n]


def _to_line_cols(forms, lines: _Lines, first_line=1, first_col=0):
    def f(x):
        if isinstance(x, Form) and x.pos is not None and len(x.pos) == 2:
            (l1, c1), (l2, c2) = lines.at(x.pos[0]), lines.at(x.pos[1])
            if l1 == 1:
                c1 += first_col
            if l2 == 1:
                c2 += first_col
            return replace(x, pos=(l1 + first_line - 1, c1, l2 + first_line - 1, c2))
        return x
    return map_forms(f, forms)


def _expected(e: P.ParseError) -> str:
    ex = sorted(e.expected)
    if len(ex) == 1:
        return ex[0]
    return "one of " + ", ".join(ex)


def read(text: str, first_line: int = 1) -> list:
    """The Lines of a tessh text (raises ReadError)."""
    lines_at = _Lines(text)
    try:
        raw = lines_parser.parse(text)
    except P.ParseError as e:
        msg = _message(text, e)
        at = e.index
        if msg.startswith("missing"):
            opener = _unclosed(text)
            if opener is not None:
                at = opener
        line, col = lines_at.at(at)
        raise ReadError(msg, line + first_line - 1, col) from None
    except ReadError as e:
        raise
    raw = _split_semis(raw)
    if not raw:
        return []
    if raw[0].indent != 0:
        line, col = lines_at.at(raw[0].start)
        raise ReadError("unexpected indentation", line + first_line - 1, raw[0].indent)
    try:
        tree = (_block(0) << P.eof).parse(raw)
    except P.ParseError as e:
        bad = raw[e.index] if e.index < len(raw) else raw[-1]
        line, _ = lines_at.at(bad.items[0].pos[0])
        raise ReadError("this line's indentation doesn't match any line above it", line + first_line - 1, bad.indent) from None
    return list(_to_line_cols(tree, lines_at, first_line))


def _message(text, e: P.ParseError) -> str:
    ex = set(e.expected)
    for want in ex:
        if want.startswith("'") and "close" in want:
            return f"missing {want}"
        if "close the string" in want:
            return f"missing {want}"
    at = text[e.index:e.index + 1]
    if at in CLOSERS:
        return f"unmatched '{at}'"
    if "space" in ex and at:
        return f"unexpected '{at}'"
    if "spaces (tabs can't indent)" in ex:
        return "tabs can't indent (use spaces)"
    return f"expected {_expected(e)}" + (f", not '{at}'" if at and at != "\n" else "")


def _unclosed(text):
    """Where the bracket or quote that's never closed opens."""
    stack = []
    i = 0
    n = len(text)
    while i < n:
        ch = text[i]
        if ch == "#" and (i == 0 or text[i - 1] in " \t\n([{"):
            while i < n and text[i] != "\n":
                i += 1
            continue
        if ch in "\"'":
            q = text[i:i + 3] if text[i:i + 3] in ('"""', "'''") else ch
            start = i
            i += len(q)
            while i < n and not text.startswith(q, i):
                i += 2 if text[i] == "\\" and q[0] == '"' else 1
            if i >= n:
                return start
            i += len(q)
            continue
        if ch in "([{":
            stack.append(i)
        elif ch in ")]}" and stack:
            stack.pop()
        i += 1
    return stack[-1] if stack else None


def read_one(text: str) -> Form:
    """One item (for tests and macros)."""
    ls = read(text)
    if len(ls) != 1 or len(ls[0].items) != 1:
        raise ReadError("expected one item")
    return ls[0].items[0]


def incomplete(text: str) -> bool:
    """Does the text stop in the middle (an open bracket or string, or a
    trailing backslash)? For the REPL."""
    if re.search(r"\\\r?\n?\Z", text):
        return True
    try:
        lines_parser.parse(text)
    except P.ParseError as e:
        msg = _message(text, e)
        return e.index >= len(text.rstrip()) and msg.startswith("missing")
    except ReadError:
        return False
    return False
