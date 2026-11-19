import os
import pytest

from tessh.runner import check, new_namespace, run_source, to_python


def test_barewords_and_variables(run):
    ns, out = run("name = world\nprint hello $name")
    assert out == "hello world\n"
    assert ns["name"] == "world"


def test_application_binds_tighter(run):
    _, out = run("def fib n\n  if $n < 2\n    $n\n  else\n    fib ($n - 1) + fib ($n - 2)\nprint (fib 15)")
    assert out == "610\n"


def test_implicit_return_and_if_value(run):
    ns, _ = run("def sign n\n  if $n < 0\n    neg\n  elif $n == 0\n    zero\n  else\n    'pos'\nr = [(sign -1) (sign 0) (sign 3)]")
    # (A lone bareword on a line is a command: neg and zero aren't programs.)
    assert ns["r"][2] == "pos"


def test_inline_if_and_one_line_def(run):
    ns, _ = run("def sq x = $x * $x\ny = if (sq 3) > 5 then big else small")
    assert ns["y"] == "big"


def test_precedence(run):
    ns, _ = run("a = 1 + 2 * 3 ** 2\nb = not 1 == 2\nc = 1 < 2 < 3\nd = (- 5 3)\ne = (+ 1 2 3)")
    assert (ns["a"], ns["b"], ns["c"], ns["d"], ns["e"]) == (19, True, True, 2, 6)


def test_word_ops_next_to_barewords_are_words(run):
    _, out = run("x = 3\nprint it is $x\nprint ($x in [1 2 3])")
    assert out == "it is 3\nTrue\n"


def test_literals(run):
    ns, _ = run("l = [1 2 3]\nm = [1 + 1, 3]\nd = {a: 1 b: (len $l)}\ns = {1 2}\nt = (1, 2)\ne = {}")
    assert ns["l"] == [1, 2, 3] and ns["m"] == [2, 3]
    assert ns["d"] == {"a": 1, "b": 3} and ns["s"] == {1, 2} and ns["t"] == (1, 2) and ns["e"] == {}


def test_comprehensions(run):
    ns, _ = run("xs = [$i * 2 for i in (range 5) if $i > 1]\nd = {$k: $v for [k v] in ([[a 1] [b 2]])}\ns = (sum ($x for x in $xs))")
    assert ns["xs"] == [4, 6, 8] and ns["d"] == {"a": 1, "b": 2} and ns["s"] == 18


def test_indexing_and_slices(run):
    ns, _ = run("xs = [1 2 3 4 5]\na = $xs[1:3]\nb = $xs[-1]\nn = 2\nc = $xs[:$n]\nd = {k: 1}\ne = $d[k]")
    assert ns["a"] == [2, 3] and ns["b"] == 5 and ns["c"] == [1, 2] and ns["e"] == 1


def test_strings_interpolate(run):
    ns, _ = run('n = 3.14159\nx = "pi ${n:.2f} $(len [1 2])"\ny = \'raw $n\'')
    assert ns["x"] == "pi 3.14 2" and ns["y"] == "raw $n"


def test_destructuring_and_augmented(run):
    ns, _ = run("[a b] = [1 2]\nc, d = 3, 4\na += 10\n[h *rest] = [1 2 3]")
    assert (ns["a"], ns["b"], ns["c"], ns["d"], ns["rest"]) == (11, 2, 3, 4, [2, 3])


def test_classes_and_methods(run):
    _, out = run("class P\n  def __init__ self x\n    $self.x = $x\n  def double self = $self.x * 2\nprint ((P 21).double)")
    assert out == "42\n"


def test_try_except_value(run):
    ns, _ = run("r = try\n  int abc\nexcept ValueError as e\n  bad\nfinally\n  pass")
    assert ns["r"] is not None


def test_match(run):
    ns, _ = run("def m x\n  match $x\n    case [go $dir]\n      $dir\n    case {k: $v}\n      $v\n    case (int)\n      'int'\n    case _\n      'other'\nr = [(m [go north]) (m {k: 5}) (m 3) (m 'z')]")
    assert ns["r"] == ["north", 5, "int", "other"]


def test_lambda_and_custom_operators(run):
    ns, _ = run("f = fn [x y] ($x + $y)\na = $f 1 2\ndef cat a b = $a + $b\nb = [1] `cat` [2]\ndefop <> 6 left cat\nc = [1] <> [2] <> [3]")
    assert ns["a"] == 3 and ns["b"] == [1, 2] and ns["c"] == [1, 2, 3]


def test_imports(run):
    ns, _ = run("import os.path json\nfrom collections import OrderedDict as OD\nj = json.dumps {a: 1}")
    assert ns["j"] == '{"a": 1}' and ns["OD"].__name__ == "OrderedDict"


def test_decorators_and_generators(run):
    ns, _ = run("import functools\n@functools.cache\ndef sq x = $x * $x\ndef gen n\n  for i in (range $n)\n    yield $i\nr = (list (gen 3))\ns = sq 4")
    assert ns["r"] == [0, 1, 2] and ns["s"] == 16


def test_commands_capture_and_status(run):
    ns, out = run("x = echo hi there\nok = (true)\nbad = (false)\necho attached")
    assert ns["x"] == "hi there" and ns["x"].ok
    assert bool(ns["bad"]) is False
    assert out == "attached\n"


def test_pipes(run):
    ns, out = run("n = echo a b c | wc -w\necho x y | tr a-z A-Z\nu = echo hi | str.upper\nv = 'one\ntwo' | wc -l")
    assert ns["n"].strip() == "3" and out == "X Y\n" and ns["u"] == "HI"
    assert ns["v"].strip() == "1"


def test_bitwise_or_with_value_on_right(run):
    ns, _ = run("a = {1}\nb = {2}\nc = $a | $b")
    assert ns["c"] == {1, 2}


def test_and_or_status(run):
    _, out = run("false || echo or\ntrue && echo and\nfalse && echo never")
    assert out == "or\nand\n"


def test_redirections(run, tmp_path):
    f = tmp_path / "o.txt"
    run(f"echo hi >{f}\necho more >>{f}\nprint py >>{f}")
    assert f.read_text() == "hi\nmore\npy\n"


def test_globs(run, tmp_path):
    (tmp_path / "a.py").write_text("")
    (tmp_path / "b.py").write_text("")
    os.chdir(tmp_path)
    ns, out = run("files = glob *.py\nls *.py")
    assert ns["files"] == ["a.py", "b.py"] and out == "a.py\nb.py\n"


def test_cd_and_env(run, tmp_path):
    ns, _ = run(f"cd {tmp_path}\nhere = (pwd)\nexport TESSH_T=1\nv = $TESSH_T\nw = $env.TESSH_T")
    assert ns["here"] == str(tmp_path) and ns["v"] == "1" and ns["w"] == "1"


def test_lists_splat_into_commands(run):
    _, out = run("args = [a b]\necho $args c")
    assert out == "a b c\n"


def test_command_function_decorator(run):
    _, out = run("@command\ndef greet *names\n  print hi (' '.join $names)\ngreet *.nothing x")
    assert out == "hi *.nothing x\n"


def test_check_finds_errors():
    assert check("x = 1\nprint $x") == []
    (e,) = check("x = (1\n")
    assert e[0] == 1 and "missing" in e[3]
    (e,) = check("return 5")
    assert "outside function" in e[3]
    (e,) = check("else\n  x")
    assert e[0] == 1


def test_to_python():
    assert "print('a', x)" in to_python("x = 1\nprint a $x")


def test_uv_preamble_parsed():
    from tessh.deps import metadata
    m = metadata("# /// script\n# dependencies = [\"rich\"]\n# ///\nprint hi\n")
    assert m == {"dependencies": ["rich"]}
