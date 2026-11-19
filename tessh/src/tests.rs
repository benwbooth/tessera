//! Compile-and-run tests, and macro hygiene.
use pyo3::prelude::*;

use crate::runner::tests::{check_one, get, repr, run};

#[test]
fn barewords_and_variables() {
    let (ns, out) = run("name = world\nprint hello $name");
    assert_eq!(out, "hello world\n");
    assert_eq!(get::<String>(&ns, "name"), "world");
}

#[test]
fn application_binds_tighter() {
    let (_, out) = run("def fib n\n  if $n < 2\n    $n\n  else\n    fib ($n - 1) + fib ($n - 2)\nprint (fib 15)");
    assert_eq!(out, "610\n");
}

#[test]
fn implicit_return_and_if_value() {
    let (ns, _) = run("def sign n\n  if $n < 0\n    'neg'\n  elif $n == 0\n    'zero'\n  else\n    'pos'\nr = [(sign -1) (sign 0) (sign 3)]");
    assert_eq!(repr(&ns, "r"), "['neg', 'zero', 'pos']");
}

#[test]
fn inline_if_and_one_line_def() {
    let (ns, _) = run("def sq x = $x * $x\ny = if (sq 3) > 5 then big else small");
    assert_eq!(get::<String>(&ns, "y"), "big");
}

#[test]
fn precedence() {
    let (ns, _) = run("a = 1 + 2 * 3 ** 2\nb = not 1 == 2\nc = 1 < 2 < 3\nd = (- 5 3)\ne = (+ 1 2 3)");
    assert_eq!(get::<i64>(&ns, "a"), 19);
    assert!(get::<bool>(&ns, "b"));
    assert!(get::<bool>(&ns, "c"));
    assert_eq!(get::<i64>(&ns, "d"), 2);
    assert_eq!(get::<i64>(&ns, "e"), 6);
}

#[test]
fn word_ops_next_to_barewords_are_words() {
    let (_, out) = run("x = 3\nprint it is $x\nprint ($x in [1 2 3])");
    assert_eq!(out, "it is 3\nTrue\n");
}

#[test]
fn literals() {
    let (ns, _) = run("l = [1 2 3]\nm = [1 + 1, 3]\nd = {a: 1 b: (len $l)}\ns = {1 2}\nt = (1, 2)\ne = {}");
    assert_eq!(repr(&ns, "l"), "[1, 2, 3]");
    assert_eq!(repr(&ns, "m"), "[2, 3]");
    assert_eq!(repr(&ns, "d"), "{'a': 1, 'b': 3}");
    assert_eq!(repr(&ns, "s"), "{1, 2}");
    assert_eq!(repr(&ns, "t"), "(1, 2)");
    assert_eq!(repr(&ns, "e"), "{}");
}

#[test]
fn comprehensions() {
    let (ns, _) = run("xs = [$i * 2 for i in (range 5) if $i > 1]\nd = {$k: $v for [k v] in ([[a 1] [b 2]])}\ns = (sum ($x for x in $xs))");
    assert_eq!(repr(&ns, "xs"), "[4, 6, 8]");
    assert_eq!(repr(&ns, "d"), "{'a': 1, 'b': 2}");
    assert_eq!(get::<i64>(&ns, "s"), 18);
}

#[test]
fn indexing_and_slices() {
    let (ns, _) = run("xs = [1 2 3 4 5]\na = $xs[1:3]\nb = $xs[-1]\nn = 2\nc = $xs[:$n]\nd = {k: 1}\ne = $d[k]");
    assert_eq!(repr(&ns, "a"), "[2, 3]");
    assert_eq!(get::<i64>(&ns, "b"), 5);
    assert_eq!(repr(&ns, "c"), "[1, 2]");
    assert_eq!(get::<i64>(&ns, "e"), 1);
}

#[test]
fn strings_interpolate() {
    let (ns, _) = run("n = 3.14159\nx = \"pi ${n:.2f} $(len [1 2])\"\ny = 'raw $n'");
    assert_eq!(get::<String>(&ns, "x"), "pi 3.14 2");
    assert_eq!(get::<String>(&ns, "y"), "raw $n");
}

#[test]
fn destructuring_and_augmented() {
    let (ns, _) = run("[a b] = [1 2]\nc, d = 3, 4\na += 10\n[h *rest] = [1 2 3]");
    assert_eq!(get::<i64>(&ns, "a"), 11);
    assert_eq!(get::<i64>(&ns, "d"), 4);
    assert_eq!(repr(&ns, "rest"), "[2, 3]");
}

#[test]
fn classes_and_methods() {
    let (_, out) = run("class P\n  def __init__ self x\n    $self.x = $x\n  def double self = $self.x * 2\nprint ((P 21).double)");
    assert_eq!(out, "42\n");
}

#[test]
fn try_except_value() {
    let (ns, _) = run("r = try\n  int abc\nexcept ValueError as e\n  'bad'\nfinally\n  pass");
    assert_eq!(get::<String>(&ns, "r"), "bad");
}

#[test]
fn match_patterns() {
    let (ns, _) = run("def m x\n  match $x\n    case [go $dir]\n      $dir\n    case {k: $v}\n      $v\n    case (int)\n      'int'\n    case _\n      'other'\nr = [(m [go north]) (m {k: 5}) (m 3) (m 'z')]");
    assert_eq!(repr(&ns, "r"), "['north', 5, 'int', 'other']");
}

#[test]
fn lambda_and_custom_operators() {
    let (ns, _) = run("f = fn [x y] ($x + $y)\na = $f 1 2\ndef cat a b = $a + $b\nb = [1] `cat` [2]\ndefop <> 6 left cat\nc = [1] <> [2] <> [3]");
    assert_eq!(get::<i64>(&ns, "a"), 3);
    assert_eq!(repr(&ns, "b"), "[1, 2]");
    assert_eq!(repr(&ns, "c"), "[1, 2, 3]");
}

#[test]
fn imports() {
    let (ns, _) = run("import os.path json\nfrom collections import OrderedDict as OD\nj = json.dumps {a: 1}");
    assert_eq!(get::<String>(&ns, "j"), "{\"a\": 1}");
}

#[test]
fn decorators_and_generators() {
    let (ns, _) = run("import functools\n@functools.cache\ndef sq x = $x * $x\ndef gen n\n  for i in (range $n)\n    yield $i\nr = (list (gen 3))\ns = sq 4");
    assert_eq!(repr(&ns, "r"), "[0, 1, 2]");
    assert_eq!(get::<i64>(&ns, "s"), 16);
}

#[test]
fn commands_capture_and_status() {
    let (ns, out) = run("x = echo hi there\nok = (bool (true))\nbad = (bool (false))\necho attached");
    assert_eq!(get::<String>(&ns, "x"), "hi there");
    assert!(!get::<bool>(&ns, "bad"));
    assert!(get::<bool>(&ns, "ok"));
    assert_eq!(out, "attached\n");
}

#[test]
fn pipes() {
    let (ns, out) = run("n = echo a b c | wc -w\necho x y | tr a-z A-Z\nu = echo hi | str.upper\nv = 'one\ntwo' | wc -l");
    assert_eq!(get::<String>(&ns, "n").trim(), "3");
    assert_eq!(out, "X Y\n");
    assert_eq!(get::<String>(&ns, "u"), "HI");
    assert_eq!(get::<String>(&ns, "v").trim(), "1");
}

#[test]
fn bitwise_or_with_value_on_right() {
    let (ns, _) = run("a = {1}\nb = {2}\nc = $a | $b");
    assert_eq!(repr(&ns, "c"), "{1, 2}");
}

#[test]
fn and_or_status() {
    let (_, out) = run("false || echo or\ntrue && echo and\nfalse && echo never");
    assert_eq!(out, "or\nand\n");
}

#[test]
fn backslash_forces_the_program() {
    let (_, out) = run("def echo *a = print python\n\\echo program\necho x");
    assert_eq!(out, "program\npython\n");
}

#[test]
fn redirections() {
    let d = std::env::temp_dir().join(format!("tessh-test-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("o.txt");
    let f = f.to_string_lossy();
    run(&format!("echo hi >{f}\necho more >>{f}\nprint py >>{f}"));
    assert_eq!(std::fs::read_to_string(&*f).unwrap(), "hi\nmore\npy\n");
}

#[test]
fn globs_cd_and_env() {
    let d = std::env::temp_dir().join(format!("tessh-glob-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("a.py"), "").unwrap();
    std::fs::write(d.join("b.py"), "").unwrap();
    let ds = d.to_string_lossy();
    let (ns, out) = run(&format!("cd {ds}\nfiles = glob *.py\nls *.py\nhere = (pwd)\nexport TESSH_T=1\nv = $TESSH_T\nw = $env.TESSH_T\ncd -"));
    assert_eq!(repr(&ns, "files"), "['a.py', 'b.py']");
    assert!(out.starts_with("a.py\nb.py\n"), "{out}");
    assert_eq!(get::<String>(&ns, "here"), ds);
    assert_eq!(get::<String>(&ns, "v"), "1");
    assert_eq!(get::<String>(&ns, "w"), "1");
}

#[test]
fn lists_splat_into_commands() {
    let (_, out) = run("args = [a b]\necho $args c");
    assert_eq!(out, "a b c\n");
}

#[test]
fn command_function_decorator() {
    let (_, out) = run("@command\ndef greet *names\n  print hi (' '.join $names)\ngreet *.nothing x");
    assert_eq!(out, "hi *.nothing x\n");
}

#[test]
fn tracebacks_point_at_tessh_lines() {
    let _g = crate::runner::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
    pyo3::Python::attach(|py| {
        let ns = crate::runner::new_namespace(py, "__main__", vec![]).unwrap();
        let r = crate::runner::run_source(py, "x = 1\n\ndef f\n  1 / 0\nf", &ns, "<tb>", false);
        let Err(crate::runner::TesshError::Py(e)) = r else { panic!("no error") };
        let tb = py.import("traceback").unwrap().getattr("extract_tb").unwrap().call1((e.traceback(py).unwrap(),)).unwrap();
        let last = tb.get_item(-1).unwrap();
        assert_eq!(last.getattr("lineno").unwrap().extract::<u32>().unwrap(), 4);
        assert_eq!(last.getattr("line").unwrap().extract::<String>().unwrap(), "1 / 0");
    });
}

#[test]
fn check_finds_errors() {
    assert!(check_one("x = 1\nprint $x").is_empty());
    let e = check_one("x = (1\n");
    assert_eq!(e[0].0, 1);
    assert!(e[0].3.contains("missing"), "{:?}", e);
    let e = check_one("return 5");
    assert!(e[0].3.contains("outside function"), "{:?}", e);
    let e = check_one("else\n  x");
    assert_eq!(e[0].0, 1);
    let e = check_one("x = 1\nprint (é ) a\n  y = )");
    assert_eq!(e[0].0, 3, "{:?}", e);
}

#[test]
fn to_python() {
    let s = pyo3::Python::attach(|py| crate::runner::to_python(py, "x = 1\nprint a $x").ok().unwrap());
    assert!(s.contains("print('a', x)"), "{s}");
}

// ---- hygiene ----------------------------------------------------------------------

#[test]
fn swap_does_not_capture() {
    let (_, out) = run("defmacro swap a b\n  quasi\n    tmp = $~a\n    ~a = $~b\n    ~b = $tmp\ntmp = 1\nother = 2\nswap tmp other\nprint $tmp $other");
    assert_eq!(out, "2 1\n");
}

#[test]
fn macro_binding_does_not_clobber_user_variable() {
    let (ns, _) = run("defmacro my_or a b\n  quasi\n    t = ~a\n    if $t then $t else ~b\nt = 5\nr = my_or 0 $t");
    assert_eq!(get::<i64>(&ns, "r"), 5);
    assert_eq!(get::<i64>(&ns, "t"), 5);
}

#[test]
fn free_names_mean_the_definition_site() {
    let (ns, _) = run("defmacro mklist x\n  :(list ~x)\ndef f\n  list = 7\n  mklist \"ab\"\nr = (f)");
    assert_eq!(repr(&ns, "r"), "['a', 'b']");
}

#[test]
fn capture_is_unhygienic_on_purpose() {
    let (_, out) = run("defmacro aif cond:expr *body\n  it = capture \"it\"\n  quasi\n    ~it = ~cond\n    if $~it\n      ~@body\naif (len [1 2 3])\n  print got $it");
    assert_eq!(out, "got 3\n");
}

#[test]
fn expr_parameter_and_splice() {
    let (_, out) = run("defmacro unless cond:expr *body\n  quasi\n    if (not ~cond)\n      ~@body\nx = 2\nunless $x > 3\n  print a\n  print b");
    assert_eq!(out, "a\nb\n");
}

#[test]
fn nested_macros_keep_hygiene() {
    let (_, out) = run("defmacro inc v\n  quasi\n    tmp = $~v + 1\n    ~v = $tmp\ndefmacro inc2 v\n  quasi\n    inc ~v\n    inc ~v\ntmp = 10\nn = 0\ninc2 n\nprint $n $tmp");
    assert_eq!(out, "2 10\n");
}

#[test]
fn unquote_expression() {
    let (ns, _) = run("defmacro const3\n  quasi\n    ~(1 + 2)\nr = const3");
    assert_eq!(get::<i64>(&ns, "r"), 3);
}

#[test]
fn macro_errors_have_positions() {
    let e = check_one("defmacro m a\n  quasi\n    ~a\nm\n");
    assert_eq!(e[0].0, 4, "{:?}", e);
    assert!(e[0].3.contains("needs a"), "{:?}", e);
}

/// Every example in the guide (doc/guide.md) runs, and prints what the
/// `output` block under it shows. (`tessh no-run`: checked, not run.)
#[test]
fn guide_examples() {
    use crate::runner::{check, new_namespace, run_source, tests::capture_fd1, tests::LOCK, TesshError};
    let mut blocks: Vec<(usize, String, bool, Option<String>)> = Vec::new();
    let lines: Vec<&str> = crate::help::GUIDE.lines().collect();
    let mut i = 0;
    let fence = |i: &mut usize| {
        let mut s = String::new();
        *i += 1;
        while *i < lines.len() && !lines[*i].starts_with("```") {
            s.push_str(lines[*i]);
            s.push('\n');
            *i += 1;
        }
        *i += 1;
        s
    };
    while i < lines.len() {
        if let Some(info) = lines[i].strip_prefix("```tessh") {
            let line = i + 1;
            let run = !info.contains("no-run");
            let code = fence(&mut i);
            let mut j = i;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            let out = if j < lines.len() && lines[j] == "```output" {
                i = j;
                Some(fence(&mut i))
            } else {
                None
            };
            blocks.push((line, code, run, out));
        } else {
            i += 1;
        }
    }
    assert!(blocks.len() > 50, "{} examples", blocks.len());
    let addr = |s: &str| {
        let mut o = String::new();
        let mut rest = s;
        while let Some(k) = rest.find("at 0x") {
            o.push_str(&rest[..k + 3]);
            o.push_str("ADDR");
            rest = rest[k + 3..].trim_start_matches(|c: char| c == 'x' || c.is_ascii_hexdigit());
        }
        o + rest
    };
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut failed = Vec::new();
    Python::attach(|py| {
        let home = std::env::current_dir().unwrap();
        for (line, code, run, want) in &blocks {
            let errs = check(py, code, "<guide>");
            if !errs.is_empty() {
                failed.push(format!("guide.md:{line}: check: {errs:?}\n{code}"));
                continue;
            }
            if !run {
                continue;
            }
            let dir = std::env::temp_dir().join(format!("tessh-guide-{}-{line}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            std::env::set_current_dir(&dir).unwrap();
            let ns = new_namespace(py, "__main__", vec![]).unwrap();
            let mut err = None;
            let out = capture_fd1(py, || {
                if let Err(e) = run_source(py, code, &ns, "<guide>", false) {
                    err = Some(match e {
                        TesshError::Py(e) => format!("{}", e.value(py)),
                        e => format!("{:?}", e.position()),
                    });
                }
            });
            std::env::set_current_dir(&home).unwrap();
            let _ = std::fs::remove_dir_all(&dir);
            if let Some(e) = err {
                failed.push(format!("guide.md:{line}: error: {e}\n{code}--- printed:\n{out}"));
            } else if let Some(w) = want {
                if addr(&out) != *w {
                    failed.push(format!("guide.md:{line}: printed\n{out}--- not\n{w}"));
                }
            }
        }
    });
    assert!(failed.is_empty(), "{} of {} examples:\n\n{}", failed.len(), blocks.len(), failed.join("\n\n"));
}

#[test]
fn help_topics() {
    let ts = crate::help::topics();
    assert!(ts.len() > 25);
    assert!(crate::help::lookup("pipes").unwrap().starts_with("## Pipes"));
    assert!(crate::help::lookup("|").unwrap().contains("Pipes"));
    assert!(crate::help::lookup("").unwrap().contains("`help macros`"));
    assert!(crate::help::lookup("zzqx").is_none());
}

#[test]
fn programs_needing_a_terminal() {
    let w = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
    for yes in ["vim x", "less f", "sudo vim /etc/x", "env A=1 htop", "python", "python3 -q", "ssh host", "ssh -p 22 host", "docker run -it img", "bash"] {
        assert!(crate::runtime::needs_tty(&w(yes)), "{yes}");
    }
    for no in ["ls", "python x.py", "ssh host uptime", "docker ps", "bash -c true", "git status", "grep vim x"] {
        assert!(!crate::runtime::needs_tty(&w(no)), "{no}");
    }
}
