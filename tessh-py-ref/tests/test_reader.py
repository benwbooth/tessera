import pytest

from tessh.forms import (Caret, Comma, Group, Index, Interp, IStr, KwArg, Line, Num, Op, Quasi,
                         Redir, Splat, Str, Sym, SymLit, Unary, Unquote, Var, Word, Attr, show)
from tessh.reader import ReadError, incomplete, read, read_one


def items(src):
    (l,) = read(src)
    return l.items


def test_barewords_flags_and_paths():
    its = items("ls -la --all --color=auto *.py . .. ./run.sh ../x ~/src g++ c++ /etc")
    assert all(isinstance(i, Sym) for i in its)
    # (/ alone is division where there's something on both sides, else a word.)
    assert isinstance(read_one("/"), Op)
    assert [i.name for i in its][1:5] == ["-la", "--all", "--color=auto", "*.py"]


def test_numbers():
    assert [i.value for i in items("1 -7 0x1f 1_000 2.5e3 3j .5")] == [1, -7, 31, 1000, 2500.0, 3j, 0.5]
    assert isinstance(read_one("1.2.3"), Sym)


def test_operators_only_when_spaced():
    its = items("$a + $b a+b -$x")
    assert isinstance(its[1], Op) and its[1].name == "+"
    assert isinstance(its[3], Sym) and its[3].name == "a+b"
    assert isinstance(its[4], Unary)


def test_variables_and_postfix():
    v = read_one("$x.a[0].b")
    assert isinstance(v, Attr) and v.name == "b"
    assert isinstance(v.obj, Index)
    assert isinstance(read_one("$?"), type(read_one("$?")))


def test_words_glue():
    w = read_one("$HOME/src")
    assert isinstance(w, Word) and isinstance(w.parts[0], Var) and w.parts[1] == "/src"
    w = read_one("${name}.txt")
    assert isinstance(w, Word) and isinstance(w.parts[0], Interp)


def test_strings():
    assert read_one('"a\\tb"').value == "a\tb"
    assert read_one("'raw $x \\n'").value == "raw $x \\n"
    s = read_one('"hi $name, ${n:.2f} $(pwd)"')
    assert isinstance(s, IStr)
    kinds = [type(p).__name__ for p in s.parts]
    assert kinds == ["str", "Interp", "str", "Interp", "str", "Interp"]
    assert s.parts[3].spec == ".2f"
    assert read_one('b"x"').value == b"x"
    assert read_one('"""a\nb"""').value == "a\nb"
    assert read_one('"cost \\$5"').value == "cost $5"


def test_kwargs_splats_and_prefixes():
    its = items("f key=val *$a **$k ^ls :sym `max` x==y")
    assert isinstance(its[1], KwArg) and its[1].name == "key"
    assert isinstance(its[2], Splat) and its[2].n == 1
    assert isinstance(its[3], Splat) and its[3].n == 2
    assert isinstance(its[4], Caret)
    assert isinstance(its[5], SymLit)
    assert isinstance(its[6], Op) and its[6].tick
    assert isinstance(its[7], Sym)


def test_redirections():
    its = items("cmd >out.txt 2>&1 >>log <in 2>err > f")
    assert [type(i).__name__ for i in its[1:6]] == ["Redir"] * 5
    assert its[1].op == ">" and its[2].op == "2>&1" and its[3].op == ">>"
    assert isinstance(its[6], Op) and its[6].name == ">"


def test_commas():
    its = items("cut -f1,3 [1, 2]")
    assert its[1].name == "-f1,3"
    g = its[2]
    assert isinstance(g.items[1], Comma)


def test_quasi_and_unquote():
    q = read_one(":(if ~x ~@ys)")
    assert isinstance(q, Quasi)
    us = [i for i in q.form.items if isinstance(i, Unquote)]
    assert [u.splice for u in us] == [False, True]
    assert isinstance(read_one("$~a"), Var)
    assert read_one("~/x").name == "~/x"


def test_indentation_blocks():
    ls = read("def f x\n  if $x\n    a\n  else\n    b\nf 1\n")
    assert len(ls) == 2
    d = ls[0]
    assert [c.items[0].name for c in d.children] == ["if", "else"]
    assert d.children[0].children[0].items[0].name == "a"


def test_brackets_span_lines():
    ls = read("x = [1\n  2\n     3]\ny = 1")
    assert len(ls) == 2 and len(ls[0].items[2].items) == 3


def test_continuation_and_semicolons():
    ls = read("echo a \\\n  b; echo c")
    assert len(ls) == 2 and len(ls[0].items) == 3


def test_comments():
    ls = read("# hi\nls # list\n  # inner\nx#y")
    assert [show(l) for l in ls] == ["ls", "x#y"]


def test_errors_have_positions():
    with pytest.raises(ReadError) as e:
        read("x = (1 2\ny")
    assert "missing" in e.value.msg
    with pytest.raises(ReadError) as e:
        read("a\n    b\n  c\n")
    assert e.value.line == 3
    with pytest.raises(ReadError) as e:
        read("\tx")
    assert "tab" in e.value.msg
    with pytest.raises(ReadError) as e:
        read('x = "abc')
    assert e.value.line == 1


def test_incomplete():
    assert incomplete("x = (1")
    assert incomplete('x = "a')
    assert incomplete("echo \\")
    assert not incomplete("echo hi")
    assert not incomplete("x = 1)")
