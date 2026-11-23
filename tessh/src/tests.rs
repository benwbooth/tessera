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
fn bare_loop_variables() {
    // (Python habit: a bare word is text, so these said nothing and gave
    // ['xx', 'xx']; now each is an error with its fix.)
    let e = check_one("xs = [1 2]\nprint [x * 2 for x in $xs]");
    assert!(e[0].3.contains("write $x"), "{:?}", e);
    let e = check_one("xs = [1 2]\nprint [$x for x in xs]");
    assert!(e[0].3.contains("write $xs"), "{:?}", e);
    let e = check_one("xs = [1 2]\nprint [$x for x in $xs if x > 1]");
    assert!(e[0].3.contains("write $x"), "{:?}", e);
    let e = check_one("for x in [1 2]\n  print x");
    assert!(e[0].3.contains("write $x"), "{:?}", e);
    // (Its own names elsewhere, a dict key, a nested for's: fine.)
    assert!(check_one("for x in [1 2]\n  def g x\n    $x\n  print {x: $x} ([$y for y in [$x]])").is_empty());
    assert!(check_one("print [[$i $j] for i in [1 2] for j in [a b]]").is_empty());
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

/// Builtins that did their work are a success, so `cd dir || exit 1` goes
/// on; `:` is bash's null command (`: >file`, `while :`).
#[test]
fn builtins_succeed_and_null_command() {
    let (ns, out) = run("cd . || echo failed\ncd . && echo went\nok = (bool (export TESSH_T_X=1))\n: && echo null\nn = 0\nwhile :\n  n += 1\n  if $n > 2\n    break\ne = (:)");
    assert_eq!(out, "went\nnull\n");
    assert!(get::<bool>(&ns, "ok"));
    assert_eq!(get::<i64>(&ns, "n"), 3);
    assert_eq!(get::<String>(&ns, "e"), "");
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
/// `output` block under it shows. (`tessh no-run`: checked, not run;
/// `tessh no-check`: run, not checked first.)
#[test]
fn guide_examples() {
    use crate::runner::{check, new_namespace, run_source, tests::capture_fd1, tests::LOCK, TesshError};
    let mut blocks: Vec<(usize, String, bool, bool, Option<String>)> = Vec::new();
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
            let checked = !info.contains("no-check");
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
            blocks.push((line, code, run, checked, out));
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
    let mut blessed: Vec<(usize, String)> = Vec::new();
    Python::attach(|py| {
        let home = std::env::current_dir().unwrap();
        for (line, code, run, checked, want) in &blocks {
            // (no-check: it runs, but checking alone can't, as a require of a
            // module it writes first.)
            let errs = if *checked { check(py, code, "<guide>") } else { vec![] };
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
                    blessed.push((*line, addr(&out)));
                }
            }
        }
    });
    // (TESSH_BLESS_GUIDE=1: what they print becomes their output blocks.)
    if std::env::var_os("TESSH_BLESS_GUIDE").is_some() && !blessed.is_empty() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/doc/guide.md");
        let text = std::fs::read_to_string(path).unwrap();
        let mut ls: Vec<String> = text.lines().map(String::from).collect();
        for (line, out) in blessed.iter().rev() {
            // (The output fence after the example that starts at line.)
            let mut k = *line;
            while !ls[k].starts_with("```") {
                k += 1;
            }
            k += 1;
            while ls[k].trim().is_empty() {
                k += 1;
            }
            assert_eq!(ls[k], "```output");
            let mut e = k + 1;
            while !ls[e].starts_with("```") {
                e += 1;
            }
            ls.splice(k + 1..e, out.lines().map(String::from));
        }
        std::fs::write(path, ls.join("\n") + "\n").unwrap();
        return;
    }
    assert!(failed.is_empty(), "{} of {} examples:\n\n{}", failed.len(), blocks.len(), failed.join("\n\n"));
}

/// Every tessh example in the Advanced guide (doc/advanced/*.md, the ABS
/// Guide ported) runs and prints its `output` block, as in the guide's
/// test. Each runs in a fresh folder as the script `ex.tsh`; an `exit` ends
/// it. (Its checker with the prebuilt tessh, and --bless, is the porting
/// notes' check-examples.py.)
#[test]
fn advanced_guide_examples() {
    use crate::runner::{check, new_namespace, run_source, tests::capture_fd1, tests::LOCK, TesshError};
    // (The list names every chapter file.)
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/doc/advanced");
    let mut on_disk: Vec<String> = std::fs::read_dir(dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|n| n.ends_with(".md")).collect();
    on_disk.sort();
    let listed: Vec<String> = crate::help::ADVANCED.iter().map(|(n, _)| n.to_string()).collect();
    assert_eq!(listed, on_disk, "doc/advanced.rs lists doc/advanced/*.md in order");
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
    let mut count = 0;
    Python::attach(|py| {
        let home = std::env::current_dir().unwrap();
        for (file, text) in crate::help::ADVANCED {
            let lines: Vec<&str> = text.lines().collect();
            let mut i = 0;
            while i < lines.len() {
                let Some(info) = lines[i].strip_prefix("```tessh") else {
                    i += 1;
                    continue;
                };
                let line = i + 1;
                let mut code = String::new();
                i += 1;
                while i < lines.len() && !lines[i].starts_with("```") {
                    code.push_str(lines[i]);
                    code.push('\n');
                    i += 1;
                }
                i += 1;
                let mut j = i;
                while j < lines.len() && lines[j].trim().is_empty() {
                    j += 1;
                }
                let want = if j < lines.len() && lines[j] == "```output" {
                    let mut w = String::new();
                    j += 1;
                    while j < lines.len() && !lines[j].starts_with("```") {
                        w.push_str(lines[j]);
                        w.push('\n');
                        j += 1;
                    }
                    Some(w)
                } else {
                    None
                };
                count += 1;
                if !info.contains("no-check") {
                    let errs = check(py, &code, "<advanced>");
                    if !errs.is_empty() {
                        failed.push(format!("{file}:{line}: check: {errs:?}\n{code}"));
                        continue;
                    }
                }
                if info.contains("no-run") {
                    continue;
                }
                let d = std::env::temp_dir().join(format!("tessh-advanced-{}-{count}", std::process::id()));
                let _ = std::fs::remove_dir_all(&d);
                std::fs::create_dir_all(&d).unwrap();
                std::env::set_current_dir(&d).unwrap();
                let ns = new_namespace(py, "__main__", vec!["ex.tsh".to_string()]).unwrap();
                let mut err = None;
                let out = capture_fd1(py, || {
                    if let Err(e) = run_source(py, &code, &ns, "<advanced>", false) {
                        err = match e {
                            TesshError::Py(e) if e.is_instance_of::<pyo3::exceptions::PySystemExit>(py) => None,
                            TesshError::Py(e) => Some(format!("{}", e.value(py))),
                            e => Some(format!("{:?}", e.position())),
                        };
                    }
                });
                std::env::set_current_dir(&home).unwrap();
                let _ = std::fs::remove_dir_all(&d);
                if let Some(e) = err {
                    failed.push(format!("{file}:{line}: error: {e}\n{code}--- printed:\n{out}"));
                } else if let Some(w) = want {
                    if addr(&out) != w {
                        failed.push(format!("{file}:{line}: printed\n{out}--- not\n{w}"));
                    }
                }
            }
        }
    });
    assert!(count > 5, "{count} examples");
    assert!(failed.is_empty(), "{} of {} examples:\n\n{}", failed.len(), count, failed.join("\n\n"));
}

#[test]
fn help_topics() {
    let ts = crate::help::topics();
    assert!(ts.len() > 25);
    assert!(crate::help::lookup("pipes").unwrap().starts_with("## Pipes"));
    assert!(crate::help::lookup("|").unwrap().contains("Pipes"));
    assert!(crate::help::lookup("").unwrap().contains("`help macros`"));
    assert!(crate::help::lookup("zzqx").is_none());
    // (The Advanced guide: its contents, a chapter, and words only it has.)
    assert!(crate::help::lookup("advanced").unwrap().contains("`help abs-sha-bang`"));
    assert!(crate::help::lookup("abs-sha-bang").unwrap().starts_with("# 2. Starting Off With a Sha-Bang"));
    assert!(crate::help::advanced_topics().len() > 3);
}

#[test]
fn cd_takes_bash_options() {
    let here = std::env::current_dir().unwrap();
    let d = std::env::temp_dir().join(format!("tessh-cd-{}", std::process::id()));
    std::fs::create_dir_all(d.join("real/sub")).unwrap();
    std::fs::create_dir_all(d.join("cp/proj")).unwrap();
    let _ = std::os::unix::fs::symlink(d.join("real"), d.join("link"));
    let d = d.to_string_lossy().to_string();
    let (ns, out) = run(&format!(
        "cd -P {d}/link\na = $env.PWD\ncd -- {d}/link/sub\nb = $env.PWD\ncd ..\nc = $env.PWD\ncd -\n$env.CDPATH = '{d}/cp'\ncd proj\npushd /\npushd +1\npopd +1\ndirs -l -p"
    ));
    std::env::set_current_dir(&here).unwrap();
    std::env::remove_var("CDPATH");
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(get::<String>(&ns, "a"), format!("{d}/real"), "-P resolves the symlink");
    assert_eq!(get::<String>(&ns, "b"), format!("{d}/link/sub"), "-L keeps it");
    assert_eq!(get::<String>(&ns, "c"), format!("{d}/link"), ".. goes back by name");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], format!("{d}/link/sub"), "cd - says where it went");
    assert_eq!(lines[1], format!("{d}/cp/proj"), "a $CDPATH folder is said");
    assert_eq!(lines[2], format!("/ {d}/cp/proj"));
    assert_eq!(lines[3], format!("{d}/cp/proj /"), "+1 turns the stack");
    assert_eq!(lines[4], format!("{d}/cp/proj"), "popd +1 drops one");
    assert_eq!(&lines[5..], [format!("{d}/cp/proj").as_str()], "dirs -l -p");
}

#[test]
fn print_lays_out_values() {
    let (_, out) = run("print small [1 2] {a: 1} (1,)\nxs = [1 2]\n$xs.append $xs\nprint $xs\nprint (pretty {k: [(str $i) for i in (range 30)]} width=40 color=False)");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], "small [1 2] {a: 1} (1,)", "short values stay on one line, text as text");
    assert_eq!(lines[1], "[1 2 [...]]");
    assert_eq!(lines[2..5], ["{", "  k: [", "    '0' '1' '2' '3' '4' '5' '6' '7' '8'"], "long ones break, indented, plain values packed");
    assert_eq!(lines[lines.len() - 2..], ["  ]", "}"]);
    Python::attach(|py| {
        let v = pyo3::types::PyDict::new(py);
        v.set_item("a", vec![1, 2]).unwrap();
        let c = crate::runtime::pretty_text(py, v.as_any(), Some(true)).unwrap();
        assert!(c.contains("\x1b[36ma\x1b[0m") && c.contains("\x1b[33m1\x1b[0m"), "{c:?}");
    });
}

#[test]
fn env_prints_its_variables() {
    let (ns, out) = run("print $env\ne = (dict (sorted ($env.items)))");
    Python::attach(|py| {
        let e = ns.bind(py).get_item("e").unwrap().unwrap();
        let want = crate::runtime::pretty_text(py, &e, Some(false)).unwrap();
        assert!(out.starts_with('{') && !out.contains("variables>"), "{out}");
        assert_eq!(out.trim_end(), want.trim_end(), "$env prints as the dict of its variables (sorted)");
    });
}

/// What print shows is tessh that reads back as an equal value.
#[test]
fn printed_values_read_back() {
    let defs = "from dataclasses import dataclass\nfrom collections import namedtuple Counter OrderedDict defaultdict deque\nfrom pathlib import Path\n@dataclass\nclass Pt\n  x: int\n  y: object\nNT = (namedtuple 'NT' 'a b')\n";
    let vals = r#"vals = [
  [1 -2 3.5 1e300 None True False (float 'inf') (float '-inf') (complex 1 -2)]
  ['a b' "x\ny" "\$z" "it's" "q\"q" "tab\t" "\x01" '' "é ☃" 'True' '1']
  {a: 1 'b c': 2 1: 3 (True): 4 (None): 5 'True': 6 (1, 2): 7 -1.5: 8 b"k": 9 '': 10}
  (1, 2)
  (1,)
  ()
  {1 2}
  (set)
  (frozenset {3 4})
  (frozenset)
  b"\x00ab\"$\n"
  (Pt x=1 y=([1 2]))
  (Pt 1 {a: [1 {2 3}]})
  (Pt 1 (Pt 2 {1 2}))
  (NT 1 [2 3])
  [[$i (str $i) {k: $i}] for i in (range 40)]
  (Counter 'aab')
  (OrderedDict {b: 1 a: 2})
  (defaultdict $list {a: [1]})
  (deque [1 2] 5)
  (range 3)
  (range 0 9 3)
  (Path '/tmp/x y')
  (dict ($env.items))
]
"#;
    let (ns, _) = run(&format!("{defs}{vals}"));
    let mut checked = 0;
    Python::attach(|py| {
        let vs = ns.bind(py).get_item("vals").unwrap().unwrap();
        for v in vs.try_iter().unwrap() {
            let v = v.unwrap();
            let want = v.repr().unwrap().to_string();
            for w in [200, 30, 8] {
                let kw = pyo3::types::PyDict::new(py);
                kw.set_item("width", w).unwrap();
                kw.set_item("color", false).unwrap();
                let pretty = py.import("__tessh__").unwrap().getattr("pretty").unwrap();
                let t: String = pretty.call((&v,), Some(&kw)).unwrap().extract().unwrap();
                let (ns2, _) = run(&format!("{defs}back = {t}\n"));
                let back = ns2.bind(py).get_item("back").unwrap().unwrap().repr().unwrap().to_string();
                assert_eq!(back, want, "printed at width {w} as\n{t}");
                checked += 1;
            }
        }
    });
    assert!(checked >= 72);
}

#[test]
fn path_words_are_paths() {
    let (ns, out) = run("p = /tmp/x/y.txt\nr = ./run.sh\nh = ~/n\nd = /tmp/$r\nl = [/tmp ./a]\nprint (pretty $l color=False) $r\necho $r ~");
    Python::attach(|py| {
        let ns = ns.bind(py);
        let g = |k: &str| ns.get_item(k).unwrap().unwrap();
        assert_eq!(g("p").get_type().name().unwrap().to_string(), "Path");
        assert_eq!(g("p").getattr("name").unwrap().to_string(), "y.txt");
        assert_eq!(g("r").str().unwrap().to_string(), "./run.sh", "keeps its spelling");
        assert!(g("h").str().unwrap().to_string().ends_with("/n") && !g("h").str().unwrap().to_string().starts_with('~'));
        assert_eq!(g("d").str().unwrap().to_string(), "/tmp/./run.sh");
    });
    let home = std::env::var("HOME").unwrap();
    assert_eq!(out, format!("[/tmp ./a] ./run.sh\n./run.sh {home}\n"), "printed as written; commands get text");
}

#[test]
fn a_lone_folder_goes_there() {
    let here = std::env::current_dir().unwrap();
    let (_, out) = run("import os\n/tmp\nprint (os.getcwd)\n..\nprint (os.getcwd)");
    std::env::set_current_dir(&here).unwrap();
    let tmp = std::fs::canonicalize("/tmp").unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0] == "/tmp" || std::path::Path::new(lines[0]) == tmp, "{out}");
    assert_eq!(lines[1], "/", "{out}");
}

#[test]
fn globs_are_glob_objects() {
    let here = std::env::current_dir().unwrap();
    let d = std::env::temp_dir().join(format!("tessh-glob-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    for f in ["a.txt", "b.txt", "c.py"] {
        std::fs::write(d.join(f), "").unwrap();
    }
    std::env::set_current_dir(&d).unwrap();
    let (ns, out) = run("g = *.txt\nprint *.txt\nprint (len $g) *.none (3 * 4)\nf = ./a.txt\nprint ($f in $g)\necho *.py");
    std::env::set_current_dir(&here).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    Python::attach(|py| assert_eq!(ns.bind(py).get_item("g").unwrap().unwrap().get_type().name().unwrap().to_string(), "Glob"));
    assert_eq!(out, "a.txt b.txt\n2 *.none 12\nTrue\nc.py\n");
}

#[test]
fn backslash_escapes_in_words() {
    let (ns, out) = run("x = 7th\\ Heaven/\nprint $x a\\(b \\$HOME \\*\necho one \\\n  two");
    Python::attach(|py| assert_eq!(ns.bind(py).get_item("x").unwrap().unwrap().str().unwrap().to_string(), "7th Heaven/"));
    assert_eq!(out, "7th Heaven/ a(b $HOME *\none two\n");
}

#[test]
fn redirections_all_of_bash() {
    let d = std::env::temp_dir().join(format!("tessh-redir-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = |n: &str| d.join(n).to_string_lossy().to_string();
    let code = format!(
        "sh -c 'echo o; echo e >&2' >{a} 2>&1\ncat {a}\nsh -c 'echo o; echo e >&2' 2>&1 >{b} | tr a-z A-Z\ncat {b}\necho c >|{c}\ncat <>{c}\nsh -c 'echo three >&3' 3>&1\nsh -c 'echo gone >&2; true' 2>&-\ntr a-z A-Z <<<here\nname = world\ncat <<END\nhi $name\nEND\ncat <<'END'\n$name\nEND\ndiff <(echo x) <(echo y) | head -1\necho up > >(tr a-z A-Z)",
        a = f("a"), b = f("b"), c = f("c")
    );
    let (_, out) = run(&code);
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(out, "o\ne\nE\no\nc\nthree\nHERE\nhi world\n$name\n1c1\nUP\n");
}

#[test]
fn bare_assignment_and_env_prefix() {
    let (_, out) = run("a=1\nb=x c=$a\nprint $a $b $c (dict k=2)\nTESSH_T1=v sh -c 'echo $TESSH_T1'\nprint (\"TESSH_T1\" in $env)\ne=\nprint [$e]");
    assert_eq!(out, "1 x 1 {k: 2}\nv\nFalse\n['']\n");
}

#[test]
fn regexes_and_matching() {
    let (_, out) = run("s = \"Build 42\"\nm = $s =~ re'(\\d+)'\nprint ($m.group 1) (($s =~ re'^build'i) != None) ($s =~ re'^build') ($s !~ re'x')\nf = ./a.py\nprint ($f =~ *.py) ($f =~ *.rs) ($s =~ '42') ($s =~ '42$')\nprint re'a\\d'ms (\"a\\nb\" =~ re'^b'm != None)");
    assert_eq!(out, "42 True None True\nTrue False True False\nre'a\\d'ms True\n");
}
