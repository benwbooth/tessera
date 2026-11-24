//! The shell builtins a login shell needs beyond cd and jobs: trap, ulimit,
//! umask, read, getopts, shift, history, times, type, hash, exec, eval,
//! builtin and `command -v`, written in Python (the `__tessh_shell__`
//! module, below). And bash's builtins tessh does its own way (alias,
//! declare, local, readonly, shopt, …): each says what that way is, rather
//! than "command not found".

use crate::runtime::{output, set_status};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule, PyString, PyTuple};
use std::os::unix::process::CommandExt;

/// Each builtin's usage, for `help NAME` and the completion panel.
pub fn usage(name: &str) -> Option<&'static str> {
    Some(match name {
        "wait" => "wait [-n] [%job | pid …]\n\nWait for background jobs: all of them without one (then the status is 0), or each one named (its exit status; a Python job's result). -n waits for whichever ends first. A job's value waits the same way: j = cmd & … then $j.wait on a line of its own, or r = ($j.wait). Ctrl+C stops the waiting, not the job.",
        "disown" => "disown [-ahr] [%job | pid …]\n\nTake a job off the list (the current one without a job), so it goes on running after tessh ends instead of getting SIGHUP. -h keeps it on the list but spares it the SIGHUP; -a does every job, -r every running one.",
        "kill" => "kill [-s SIG | -n NUM | -SIG] %job | pid …\nkill -l [NUM | SIG …]\n\nSend a signal (TERM without one) to jobs (%1, %%, %-, %name, $job: every program in it) and processes. A stopped job sent TERM or HUP is continued, so it can act on it. -l lists the signals, or names one by number (an exit status above 128 too). \\kill runs the program instead.",
        "suspend" => "suspend [-f]\n\nAt the prompt: stop tessh, as Ctrl+Z stops a program, until the shell that started it brings it back (fg). A login shell only with -f.",
        "logout" => "logout [status]\n\nEnd a login shell (exit, for one that isn't).",
        "trap" => "trap 'tessh text' SIGNAL …\ntrap - SIGNAL …   trap '' SIGNAL …   trap [-p] [SIGNAL …]   trap -l\n\nRun the text (tessh, in this shell: a function's name calls it) when a signal comes, or EXIT when tessh ends (a script, -c, the REPL). - puts a signal back as it was, '' ignores it; -p, or trap alone, lists the traps, -l the signals. Signals are named INT, SIGINT or 2. bash's ERR, DEBUG and RETURN aren't in tessh: try/except CommandFailed catches a failing command (strict mode), and Python's sys.settrace traces.",
        "ulimit" => "ulimit [-SHa] [-cdefilmnqrstuvx] [limit]\n\nShow or set a resource limit for tessh and the programs it runs: -c core file size, -d data, -f file size (the default), -l locked memory, -m resident memory, -n open files, -s stack, -t CPU seconds, -u processes, -v virtual memory, -e nice, -i pending signals, -q message queues, -r real-time priority, -x file locks. Sizes are in KiB (-c and -f too). limit is a number, unlimited, soft or hard. -S the soft limit, -H the hard one (setting without either sets both, as bash does); -a shows them all. Python: import resource.",
        "umask" => "umask [-p] [-S] [mode]\n\nThe file mode mask: the permissions new files don't get. Alone, it prints it (0022); -S as u=rwx,g=rx,o=rx. mode is octal (077) or symbolic (u=rwx,g=,o=, g-w, a+r).",
        "read" => "read [-rs] [-p prompt] [-n N | -N N] [-t secs] [-d delim] [-a list] [-u fd] [name …]\n\nRead a line from standard input (what <, <<< or a pipe gives this line) into names: split at $IFS (spaces, tabs and newlines without one), the last name getting the rest. No name: $REPLY, the whole line. -a: a list of all the words. -r keeps backslashes (without it, \\ joins lines and quotes the next character), -s doesn't echo (a password), -n N stops after N characters, -N after exactly N, -t gives up after secs (status 142), -d reads to delim instead of a newline, -p shows a prompt (on a terminal). The status is 1 at the end of the input, so `while read line` loops over lines. It sets the module's names; in Python, input reads a line.",
        "getopts" => "getopts optstring name [arg …]\n\nbash's option parser, for a script's $argv (or the args): each call sets $name to the next option letter and $OPTARG to its argument (a letter followed by : in optstring takes one; None for one that doesn't), and $OPTIND to the index of the next argument; at the end it fails, so `while getopts \"ab:\" opt` loops. An unknown option gives ?, with a message (none, and $OPTARG the letter, when optstring starts with :). Then `shift ($OPTIND - 1)` leaves the rest. Python's argparse does more.",
        "shift" => "shift [n]\n\nDrop the first n (1) arguments after the script's name from $argv: $argv[1] goes, and the next takes its place. Fails, changing nothing, if there aren't that many.",
        "times" => "times\n\nThe CPU time tessh has used (user, then system) and, on the next line, its children's. Python: os.times.",
        "history" => "history [N]\nhistory -c   history -d N   history -s text\n\nThe commands typed at tessh's prompt, numbered, the last N of them with N. -c clears the history file, -d N deletes entry N, -s adds text. The file is $XDG_DATA_HOME/tessh/history (~/.local/share/tessh/history); the editor's ↑ and Ctrl+R search it with bash's and zsh's.",
        "fc" => "fc -l [-nr] [first [last]]\n\nList past commands, as history does. To run one again or edit it, ↑ or Ctrl+R at the prompt.",
        "caller" => "caller\n\nNot in tessh: a function's caller is Python's frame. traceback.print_stack shows the calls (import traceback), inspect.stack gives them as values.",
        "builtin" => "builtin name [arg …]\n\nRun tessh's builtin of that name, even when a function of yours has the name (def cd … that ends with builtin cd $dir).",
        "enable" => "enable\n\nNot in tessh: its builtins can't be turned off. A function of the same name takes a builtin's place, and \\name runs the program of that name.",
        "type" => "type [-afptP] name …\n\nWhat each name is as a command: a tessh keyword, a function (yours, or a command function), a variable, a tessh builtin, a Python builtin, or a program (and where). -t says only which (keyword, function, variable, builtin or file), -p only a program's path, -P looks only for a program, -a shows everything the name is. Given a value instead of words (type $x), it's Python's type.",
        "hash" => "hash [-r] [-t name …] [name …]\n\ntessh doesn't remember where programs are: it looks each one up on $PATH as it runs it, so hash -r has nothing to forget. hash name checks it's there (and fails if not); -t prints where. Given a value (hash $x), it's Python's hash.",
        "exec" => "exec [-cl] [-a name] program [arg …]\n\nReplace tessh with the program (exec zsh, exec tmux): it never comes back. -c gives it an empty environment, -l starts it as a login shell (a - before its name), -a gives it another name. Given code (exec $code, exec \"x = 1\"), it's Python's exec. exec with only redirections (exec >log) isn't in tessh: redirect a block with with or a function.",
        "eval" => "eval text …\n\nRun the words joined as tessh text, here, as bash's eval does: eval \"echo hi\", eval $\"(ssh-agent -c)\" (tessh doesn't read sh: give it tessh, or run sh -c). Its value is the last form's, so x = (eval \"1 + 2\") is 3. Given a value (eval $code), it's Python's eval.",
        "readonly" | "declare" | "typeset" | "local" | "alias" | "unalias" | "shopt" | "bind" | "complete" | "compgen" => return idiom(name),
        _ => return None,
    })
}

/// bash's builtins tessh does another way: what to do instead.
fn idiom(name: &str) -> Option<&'static str> {
    Some(match name {
        "alias" | "unalias" => "tessh has no aliases: a command function does what one does, and can take its arguments anywhere:\n\n  @command\n  def ll *args\n    ls -l *$args\n\nA def replaces it; del ll removes it. (help commands)",
        "declare" | "typeset" => "tessh's variables are Python's: name = value sets one, of any type (bash's -i, -a and -A are an int, a list and a dict), export NAME=value sets an environment variable (declare -x), a name set in a def is the def's own (declare in a function), and print $name or pretty $name shows one (declare -p).",
        "local" => "a name set in a def is already the def's own (Python's scopes): no local is needed. global name or nonlocal name sets one outside instead.",
        "readonly" => "tessh has no read-only names: by Python's convention a NAME in capitals is a constant (typing.Final marks one for a type checker). export NAME=value sets an environment variable.",
        "shopt" => "tessh has no shell options: globs, strings and folders work one way (help globs), strict on is bash's set -e, and help abs-options says what to do for the rest.",
        "bind" => "tessh's editor keys are its own, not readline's: help keys (or tessh --keys) lists them.",
        "complete" => "tessh completes programs, their options, files, folders and Python names by itself; completions can't be added to it per command.",
        "compgen" => "tessh has no compgen: glob \"pre*\" lists files, which finds a program, and dir($module) lists names.",
        _ => return None,
    })
}

/// Whether the builtin is given the namespace it runs in (to set names, run text there).
pub fn wants_ns(name: &str) -> bool {
    matches!(name, "trap" | "read" | "getopts" | "shift" | "type" | "eval" | "builtin" | "hash")
}

/// Run builtin `name` with its words: the Python function `sh_name`, or the
/// message for one tessh does another way.
pub fn run(py: Python<'_>, name: &str, words: &[String], ns: Option<Bound<'_, PyDict>>) -> PyResult<Py<PyAny>> {
    if let Some(msg) = idiom(name) {
        return Err(pyo3::exceptions::PyNotImplementedError::new_err(format!("{name}: {msg}")));
    }
    let m = py.import("__tessh_shell__")?;
    let f = m.getattr(format!("sh_{name}").as_str())?;
    let kw = PyDict::new(py);
    if wants_ns(name) {
        kw.set_item("_ns", ns)?;
    }
    let args = PyTuple::new(py, words.iter().map(|w| PyString::new(py, w)))?;
    let v = f.call(args, Some(&kw))?;
    // (An Output: its status is the shell's.)
    if let Ok(c) = v.getattr("code").and_then(|c| c.extract::<i32>()) {
        if v.hasattr("attached").unwrap_or(false) {
            set_status(c);
        }
    }
    Ok(v.unbind())
}

/// `command -v name`, `command -V name`, `command -p prog …`.
pub fn command_opts(py: Python<'_>, args: &[Bound<'_, PyAny>]) -> PyResult<Py<PyAny>> {
    let words: Vec<String> = args.iter().map(|a| a.str().map(|s| s.to_string())).collect::<PyResult<_>>()?;
    // (The namespace of whatever called command: the frame above.)
    let ns = py.import("sys")?.getattr("_getframe")?.call1((0,))?.getattr("f_globals")?;
    let f = py.import("__tessh_shell__")?.getattr("command_opts")?;
    Ok(f.call1((words, ns))?.unbind())
}

/// Tessh text, run in ns (a trap's, eval's): the last form's value.
#[pyfunction]
fn eval_tessh(py: Python<'_>, text: &str, ns: Bound<'_, PyDict>) -> PyResult<Py<PyAny>> {
    crate::runner::eval_source(py, text, &ns, "<eval>").map_err(|e| e.into_pyerr(py))
}

/// exec: this process becomes the program; on success it never returns.
#[pyfunction]
#[pyo3(signature = (prog, argv0, args, clear_env=false))]
fn exec_program(py: Python<'_>, prog: String, argv0: String, args: Vec<String>, clear_env: bool) -> PyResult<()> {
    let path = if prog.contains('/') { prog.clone() } else { crate::runtime::which(&prog).ok_or_else(|| pyo3::exceptions::PyFileNotFoundError::new_err(format!("exec: {prog}: not found")))? };
    if let Ok(sys) = py.import("sys") {
        for s in ["stdout", "stderr"] {
            if let Ok(f) = sys.getattr(s) {
                let _ = f.call_method0("flush");
            }
        }
    }
    let mut c = std::process::Command::new(&path);
    c.arg0(&argv0).args(&args);
    if clear_env {
        c.env_clear();
    }
    // (The program starts with every signal as the system has it: what tessh
    // and Python ignore or catch (Ctrl+Z, SIGPIPE, …) would carry over.
    // Kept, to put back if exec fails.)
    let mut old: Vec<(i32, libc::sigaction)> = vec![];
    unsafe {
        for s in 1..32 {
            if s == libc::SIGKILL || s == libc::SIGSTOP {
                continue;
            }
            let mut new: libc::sigaction = std::mem::zeroed();
            new.sa_sigaction = libc::SIG_DFL;
            let mut was: libc::sigaction = std::mem::zeroed();
            if libc::sigaction(s, &new, &mut was) == 0 {
                old.push((s, was));
            }
        }
        let mut none: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut none);
        libc::sigprocmask(libc::SIG_SETMASK, &none, std::ptr::null_mut());
    }
    let e = c.exec();
    unsafe {
        for (s, was) in old {
            libc::sigaction(s, &was, std::ptr::null_mut());
        }
    }
    Err(pyo3::exceptions::PyOSError::new_err(format!("exec: {path}: {e}")))
}

/// A signal back to what tessh has it as: Ctrl+C's KeyboardInterrupt, the
/// REPL's ignored Ctrl+Z, SIGHUP passed on to the jobs, the rest the system's.
#[pyfunction]
fn restore_signal(py: Python<'_>, sig: i32) -> PyResult<()> {
    let signal = py.import("signal")?;
    if sig == libc::SIGINT {
        signal.getattr("signal")?.call1((sig, signal.getattr("default_int_handler")?))?;
        return Ok(());
    }
    // (Through Python, so it forgets its handler; then the real disposition.)
    signal.getattr("signal")?.call1((sig, signal.getattr("SIG_DFL")?))?;
    let interactive = crate::runtime::INTERACTIVE.load(std::sync::atomic::Ordering::SeqCst);
    unsafe {
        if sig == libc::SIGHUP {
            crate::jobs::on_hangup();
        } else if interactive && [libc::SIGTSTP, libc::SIGTTOU, libc::SIGTTIN].contains(&sig) {
            libc::signal(sig, libc::SIG_IGN);
        } else if sig == libc::SIGPIPE || sig == libc::SIGXFSZ {
            libc::signal(sig, libc::SIG_IGN);
        }
    }
    Ok(())
}

#[pyfunction]
fn builtin_named<'py>(py: Python<'py>, name: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
    crate::runtime::builtin_named(py, name)
}

#[pyfunction]
fn is_login() -> bool {
    crate::LOGIN.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn add(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let py = m.py();
    m.add_function(wrap_pyfunction!(eval_tessh, m)?)?;
    m.add_function(wrap_pyfunction!(exec_program, m)?)?;
    m.add_function(wrap_pyfunction!(restore_signal, m)?)?;
    m.add_function(wrap_pyfunction!(builtin_named, m)?)?;
    m.add_function(wrap_pyfunction!(is_login, m)?)?;
    let code = std::ffi::CString::new(SHELL_PY).unwrap();
    let sh = PyModule::from_code(py, &code, c"<tessh builtins>", c"__tessh_shell__")?;
    py.import("sys")?.getattr("modules")?.set_item("__tessh_shell__", &sh)?;
    // (An interactive tessh's jobs get SIGHUP when it ends, or its terminal does.)
    py.import("atexit")?.call_method1("register", (m.getattr("hup_jobs")?,))?;
    crate::jobs::on_hangup();
    // ($SHELL, when nothing set it: this tessh, as bash sets it.)
    if std::env::var_os("SHELL").is_none() {
        if let Ok(exe) = std::env::current_exe() {
            crate::runtime::setenv(py, "SHELL", Some(&exe.to_string_lossy()))?;
        }
    }
    let _ = output;
    Ok(())
}

const SHELL_PY: &str = r#####"
import os, sys

def _tessh():
    import __tessh__
    return __tessh__

def _status(code):
    return _tessh().Output("", code, "", True)

def _err(msg):
    sys.stdout.flush()
    print("tessh: " + msg, file=sys.stderr, flush=True)

def _split_opts(args, with_value=""):
    """bash-style options: (letters, {letter: value}, rest). with_value: the
    letters that take a value (glued, -p'> ', or the next word)."""
    flags, vals, i = [], {}, 0
    args = list(args)
    while i < len(args):
        a = args[i]
        if a == "--":
            i += 1
            break
        if not a.startswith("-") or a == "-":
            break
        j = 1
        while j < len(a):
            c = a[j]
            if c in with_value:
                v = a[j + 1:]
                if not v:
                    i += 1
                    if i >= len(args):
                        raise ValueError(f"-{c}: option requires an argument")
                    v = args[i]
                vals[c] = v
                break
            flags.append(c)
            j += 1
        i += 1
    return flags, vals, args[i:]

def staged(f, *rest, _last=False, **kw):
    """A builtin in a pipe: what came in (rest's last) is its input, and what
    it prints its output, as text for the next stage (unless it's last)."""
    import io, contextlib
    words, data = rest[:-1], rest[-1]
    old_in = sys.stdin
    if data is not None:
        text = data.decode("utf-8", "replace") if isinstance(data, bytes) else str(data)
        sys.stdin = io.StringIO(text + ("\n" if text and not text.endswith("\n") else ""))
    try:
        if _last:
            return f(*words, **kw)
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            v = f(*words, **kw)
        # (Its last newline kept: the next program reads lines.)
        code = getattr(v, "code", 0) if v is not None else 0
        return _tessh().Output(buf.getvalue(), code, "", False)
    finally:
        sys.stdin = old_in

def captured(f, *words, **kw):
    """A builtin used as a value: what it prints, as text (with its status),
    unless it gives a value of its own (which's path, glob's list)."""
    import io, contextlib
    Output = _tessh().Output
    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        v = f(*words, **kw)
    if v is None or (isinstance(v, Output) and getattr(v, "attached", False) and not str(v)):
        out = buf.getvalue()
        if out.endswith("\n"):
            out = out[:-1]
        return Output(out, getattr(v, "code", 0) if v is not None else 0, getattr(v, "err", "") if v is not None else "", False)
    return v

# ---- trap ------------------------------------------------------------------------------

_traps = {}
_trap_ns = {}
_exit_hooked = False

def _signum(s):
    u = s.upper()
    if u in ("EXIT", "SIGEXIT", "0"):
        return "EXIT"
    if u in ("ERR", "DEBUG", "RETURN"):
        return u
    return _tessh().signal_number(s)

def _signame(k):
    if k == "EXIT":
        return "EXIT"
    names = {v: n for n, v in _tessh().signal_names()}
    return "SIG" + names[k] if k in names else str(k)

def _run_trap(k):
    text = _traps.get(k)
    if not text:
        return
    try:
        _tessh().eval_tessh(text, _trap_ns[k])
    except (SystemExit, KeyboardInterrupt):
        raise
    except BaseException as e:
        _err(f"trap {_signame(k)}: {type(e).__name__}: {e}")

def _on_exit():
    k = "EXIT"
    if k in _traps:
        try:
            _run_trap(k)
        except KeyboardInterrupt:
            pass
        except SystemExit:
            pass
        _traps.pop(k, None)

def _on_signal(signum, frame):
    _run_trap(signum)

def sh_trap(*args, _ns=None):
    import signal, atexit, shlex
    global _exit_hooked
    args = list(args)
    if args and args[0] == "--":
        args = args[1:]
    if not args or args[0] == "-p":
        keys = [_signum(a) for a in args[1:]] if args else list(_traps)
        for k in keys:
            if k in _traps:
                print(f"trap -- {shlex.quote(_traps[k])} {_signame(k)}")
        return None
    if args[0] == "-l":
        rows = [f"{v:2}) SIG{n}" for n, v in _tessh().signal_names()]
        for i in range(0, len(rows), 5):
            print("\t".join(rows[i:i + 5]))
        return None
    if len(args) == 1 or args[0].isdigit():
        action, sigs = "-", args
    else:
        action, sigs = args[0], args[1:]
    for s in sigs:
        k = _signum(s)
        if k is None:
            raise ValueError(f"trap: {s}: invalid signal specification")
        if k in ("ERR", "DEBUG", "RETURN"):
            raise NotImplementedError(f"trap: {k}: not in tessh. try/except CommandFailed catches a failing command (with strict on, bash's set -e), and Python's sys.settrace traces each line.")
        if k == "EXIT":
            _tessh().set_exit_trap(action != "-")
            if action == "-":
                _traps.pop(k, None)
            else:
                _traps[k] = action
                _trap_ns[k] = _ns
                if not _exit_hooked:
                    atexit.register(_on_exit)
                    _exit_hooked = True
            continue
        if k in (signal.SIGKILL, signal.SIGSTOP):
            raise ValueError(f"trap: {_signame(k)}: can't be caught or ignored")
        if action == "-":
            _tessh().restore_signal(k)
            _traps.pop(k, None)
        elif action == "":
            signal.signal(k, signal.SIG_IGN)
            _traps[k] = ""
        else:
            signal.signal(k, _on_signal)
            _traps[k] = action
            _trap_ns[k] = _ns
    return None

# ---- ulimit, umask, times ------------------------------------------------------------

_LIMITS = [
    ("c", "RLIMIT_CORE", "core file size", "kbytes", 1024),
    ("d", "RLIMIT_DATA", "data seg size", "kbytes", 1024),
    ("e", "RLIMIT_NICE", "scheduling priority", "", 1),
    ("f", "RLIMIT_FSIZE", "file size", "kbytes", 1024),
    ("i", "RLIMIT_SIGPENDING", "pending signals", "", 1),
    ("l", "RLIMIT_MEMLOCK", "max locked memory", "kbytes", 1024),
    ("m", "RLIMIT_RSS", "max memory size", "kbytes", 1024),
    ("n", "RLIMIT_NOFILE", "open files", "", 1),
    ("q", "RLIMIT_MSGQUEUE", "POSIX message queues", "bytes", 1),
    ("r", "RLIMIT_RTPRIO", "real-time priority", "", 1),
    ("s", "RLIMIT_STACK", "stack size", "kbytes", 1024),
    ("t", "RLIMIT_CPU", "cpu time", "seconds", 1),
    ("u", "RLIMIT_NPROC", "max user processes", "", 1),
    ("v", "RLIMIT_AS", "virtual memory", "kbytes", 1024),
    ("x", "RLIMIT_LOCKS", "file locks", "", 1),
]

def sh_ulimit(*args):
    import resource
    table = {o: (getattr(resource, r), d, u, k) for o, r, d, u, k in _LIMITS if hasattr(resource, r)}
    letters = "".join(table)
    hard = soft = every = False
    opts, value = [], None
    for a in args:
        if a.startswith("-") and len(a) > 1 and value is None:
            for c in a[1:]:
                if c == "H":
                    hard = True
                elif c == "S":
                    soft = True
                elif c == "a":
                    every = True
                elif c in table:
                    opts.append(c)
                else:
                    raise ValueError(f"ulimit: -{c}: invalid option\nulimit: usage: ulimit [-SHa{letters}] [limit]")
        elif value is None:
            value = a
        else:
            raise ValueError(f"ulimit: {a}: too many arguments\nulimit: usage: ulimit [-SHa{letters}] [limit]")
    def shown(c):
        r, d, u, k = table[c]
        s, h = resource.getrlimit(r)
        v = h if hard and not soft else s
        return "unlimited" if v == resource.RLIM_INFINITY else str(v // k)
    def row(c):
        r, d, u, k = table[c]
        unit = f"({u}, -{c})" if u else f"(-{c})"
        return f"{d:<26}{unit:>16} {shown(c)}"
    if every:
        for c in table:
            print(row(c))
        return None
    if not opts:
        opts = ["f"]
    if value is None:
        for c in opts:
            print(row(c) if len(opts) > 1 else shown(c))
        return None
    if len(opts) > 1:
        raise ValueError("ulimit: one limit at a time when setting one")
    r, d, u, k = table[opts[0]]
    s, h = resource.getrlimit(r)
    if value == "unlimited":
        n = resource.RLIM_INFINITY
    elif value == "hard":
        n = h
    elif value == "soft":
        n = s
    else:
        try:
            n = int(value) * k
        except ValueError:
            raise ValueError(f"ulimit: {value}: invalid number (a number, unlimited, soft or hard)") from None
    new_s = n if soft or not hard else s
    new_h = n if hard or not soft else h
    try:
        resource.setrlimit(r, (new_s, new_h))
    except (ValueError, OSError) as e:
        raise OSError(f"ulimit: {d}: can't change the limit: {e}") from None
    return None

def _umask_now():
    m = os.umask(0)
    os.umask(m)
    return m

def _umask_symbolic(m):
    allowed = ~m & 0o777
    parts = []
    for who, shift in (("u", 6), ("g", 3), ("o", 0)):
        bits = (allowed >> shift) & 7
        parts.append(who + "=" + "".join(ch for ch, b in (("r", 4), ("w", 2), ("x", 1)) if bits & b))
    return ",".join(parts)

def sh_umask(*args):
    import re
    flags, _, rest = _split_opts(args)
    for f in flags:
        if f not in "pS":
            raise ValueError(f"umask: -{f}: invalid option\numask: usage: umask [-p] [-S] [mode]")
    m = _umask_now()
    if not rest:
        text = _umask_symbolic(m) if "S" in flags else f"{m:04o}"
        print(("umask -S " if "S" in flags else "umask ") + text if "p" in flags else text)
        return None
    mode = rest[0]
    if re.fullmatch(r"[0-7]{1,4}", mode):
        os.umask(int(mode, 8) & 0o777)
        return None
    allowed = ~m & 0o777
    for clause in mode.split(","):
        mm = re.fullmatch(r"([ugoa]*)([=+-])([rwx]*)", clause)
        if not mm:
            raise ValueError(f"umask: {mode}: invalid mode (octal, as 022, or u=rwx,g=rx,o=)")
        who, op, perms = mm.groups()
        who = who or "a"
        bits = sum(b for ch, b in (("r", 4), ("w", 2), ("x", 1)) if ch in perms)
        mask = 0
        for w, shift in (("u", 6), ("g", 3), ("o", 0)):
            if w in who or "a" in who:
                mask |= 7 << shift
                spread = bits << shift
                if op == "=":
                    allowed = (allowed & ~(7 << shift)) | spread
                elif op == "+":
                    allowed |= spread
                else:
                    allowed &= ~spread
    os.umask(~allowed & 0o777)
    return None

def sh_times():
    t = os.times()
    f = lambda s: f"{int(s // 60)}m{s % 60:.3f}s"
    print(f"{f(t.user)} {f(t.system)}\n{f(t.children_user)} {f(t.children_system)}")
    return None

# ---- read, getopts, shift --------------------------------------------------------------

def _ifs_split(s, ifs, n=None):
    import re
    ws = "".join(c for c in ifs if c in " \t\n")
    other = "".join(c for c in ifs if c not in " \t\n")
    if ws:
        s = s.strip(ws)
    if not s:
        return []
    alts = []
    if other:
        alts.append((f"[{re.escape(ws)}]*" if ws else "") + f"[{re.escape(other)}]" + (f"[{re.escape(ws)}]*" if ws else ""))
    if ws:
        alts.append(f"[{re.escape(ws)}]+")
    if not alts:
        return [s]
    return re.split("|".join(alts), s, maxsplit=0 if n is None else max(n - 1, 0))

def sh_read(*args, _ns=None):
    import select, time
    flags, vals, names = _split_opts(args, "pnNtdaui")
    for f in flags:
        if f not in "rse":
            raise ValueError(f"read: -{f}: invalid option\nread: usage: read [-ers] [-a array] [-d delim] [-n nchars] [-N nchars] [-p prompt] [-t timeout] [-u fd] [name …]")
    raw, silent = "r" in flags, "s" in flags
    delim = vals.get("d", "\n")
    delim = delim[0] if delim else "\0"
    nchars = exact = None
    try:
        if "n" in vals:
            nchars = int(vals["n"])
        if "N" in vals:
            nchars = exact = int(vals["N"])
        timeout = float(vals["t"]) if "t" in vals else None
        fd = int(vals["u"]) if "u" in vals else 0
    except ValueError as e:
        raise ValueError(f"read: {e}") from None
    if exact is not None:
        delim = None
    redirected = fd == 0 and sys.stdin is not None and sys.stdin is not sys.__stdin__
    stream = sys.stdin if redirected or (fd == 0 and sys.__stdin__ is None) else None
    tty = stream is None and os.isatty(fd)
    if "p" in vals and (tty or (stream is not None and getattr(stream, "isatty", lambda: False)())):
        sys.stderr.write(vals["p"])
        sys.stderr.flush()
    old = None
    if tty and (silent or nchars is not None):
        import termios
        old = termios.tcgetattr(fd)
        new = termios.tcgetattr(fd)
        if silent:
            new[3] &= ~termios.ECHO
        if nchars is not None:
            new[3] &= ~termios.ICANON
            new[6][termios.VMIN] = 1
            new[6][termios.VTIME] = 0
        termios.tcsetattr(fd, termios.TCSANOW, new)
    import codecs
    dec = codecs.getincrementaldecoder("utf-8")("replace")
    deadline = None if timeout is None else time.monotonic() + timeout
    def getc():
        if stream is not None:
            return stream.read(1)
        while True:
            if deadline is not None:
                left = deadline - time.monotonic()
                if left <= 0 or not select.select([fd], [], [], left)[0]:
                    raise TimeoutError
            b = os.read(fd, 1)
            if not b:
                return dec.decode(b"", True)
            ch = dec.decode(b)
            if ch:
                return ch
    out, eof, timed = [], False, False
    try:
        if stream is not None and delim == "\n" and nchars is None and raw:
            line = stream.readline()
            eof = not line.endswith("\n")
            out = list(line[:-1] if not eof else line)
        else:
            while True:
                if nchars is not None and len(out) >= nchars:
                    break
                ch = getc()
                if ch == "":
                    eof = True
                    break
                if ch == delim:
                    break
                if ch == "\\" and not raw:
                    nxt = getc()
                    if nxt == "":
                        eof = True
                        break
                    if nxt == "\n":
                        continue
                    out.append(nxt)
                    continue
                out.append(ch)
    except TimeoutError:
        timed = True
    except KeyboardInterrupt:
        raise
    finally:
        if old is not None:
            import termios
            termios.tcsetattr(fd, termios.TCSANOW, old)
            if silent and tty:
                sys.stderr.write("\n")
                sys.stderr.flush()
    text = "".join(out)
    ns = _ns if _ns is not None else {}
    ifs = ns.get("IFS")
    if not isinstance(ifs, str):
        ifs = os.environ.get("IFS", " \t\n")
    if "a" in vals:
        ns[vals["a"]] = _ifs_split(text, ifs)
    elif not names:
        ns["REPLY"] = text
    else:
        parts = _ifs_split(text, ifs, len(names))
        for k, name in enumerate(names):
            v = parts[k] if k < len(parts) else ""
            if k == len(names) - 1 and ifs:
                ws = "".join(c for c in ifs if c in " \t\n")
                v = v.rstrip(ws) if ws else v
            ns[name] = v
    if timed:
        return _status(142)
    return _status(1 if eof else 0)

def sh_getopts(optstring=None, name=None, *args, _ns=None):
    if optstring is None or name is None:
        raise ValueError("getopts: usage: getopts optstring name [arg …]")
    ns = _ns
    argv = list(args) if args else [str(a) for a in list(ns.get("argv") or [])[1:]]
    try:
        optind = int(ns.get("OPTIND", 1))
    except (TypeError, ValueError):
        optind = 1
    state = ns.get("__tessh_getopts__")
    pos = state[1] if isinstance(state, tuple) and state[0] == optind else 0
    silent = optstring.startswith(":")
    prog = os.path.basename(str((ns.get("argv") or ["tessh"])[0])) if ns.get("argv") else "tessh"
    def done():
        ns[name] = "?"
        ns["OPTARG"] = None
        ns["OPTIND"] = optind
        ns["__tessh_getopts__"] = (optind, 0)
        return _status(1)
    if optind - 1 >= len(argv):
        return done()
    a = argv[optind - 1]
    if pos == 0:
        if a == "--":
            optind += 1
            return done()
        if not a.startswith("-") or a == "-":
            return done()
        pos = 1
    c = a[pos]
    pos += 1
    i = optstring.find(c)
    ns["OPTARG"] = None
    if c == ":" or i < 0:
        if silent:
            ns["OPTARG"] = c
        else:
            _err(f"{prog}: illegal option -- {c}")
        ns[name] = "?"
    elif optstring[i + 1:i + 2] == ":":
        if pos < len(a):
            ns["OPTARG"] = a[pos:]
            optind += 1
            pos = 0
        elif optind < len(argv):
            ns["OPTARG"] = argv[optind]
            optind += 2
            pos = 0
        else:
            optind += 1
            pos = 0
            if silent:
                ns[name] = ":"
                ns["OPTARG"] = c
            else:
                _err(f"{prog}: option requires an argument -- {c}")
                ns[name] = "?"
            ns["OPTIND"] = optind
            ns["__tessh_getopts__"] = (optind, pos)
            return _status(0)
        ns[name] = c
    else:
        ns[name] = c
    if pos and pos >= len(a):
        optind += 1
        pos = 0
    ns["OPTIND"] = optind
    ns["__tessh_getopts__"] = (optind, pos)
    return _status(0)

def sh_shift(n="1", *rest, _ns=None):
    if rest:
        raise ValueError("shift: too many arguments\nshift: usage: shift [n]")
    try:
        n = int(n)
    except ValueError:
        raise ValueError(f"shift: {n}: numeric argument required") from None
    if n < 0:
        raise ValueError(f"shift: {n}: shift count out of range")
    a = _ns.get("argv")
    if not isinstance(a, list):
        a = list(a or [])
        _ns["argv"] = a
    if n > max(len(a) - 1, 0):
        return _status(1)
    del a[1:1 + n]
    return None

# ---- history, fc -----------------------------------------------------------------------

def _history_file():
    d = os.environ.get("XDG_DATA_HOME") or os.path.join(os.path.expanduser("~"), ".local", "share")
    return os.path.join(d, "tessh", "history")

def _history():
    try:
        with open(_history_file(), encoding="utf-8", errors="replace") as f:
            return [l.rstrip("\n").replace("<\\n>", "\n") for l in f if l.strip()]
    except FileNotFoundError:
        return []

def _write_history(entries):
    p = _history_file()
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8") as f:
        for e in entries:
            f.write(e.replace("\n", "<\\n>") + "\n")

def _show_history(entries, first=1, numbers=True):
    for k, e in enumerate(entries, first):
        lines = e.split("\n")
        print((f"{k:5}  " if numbers else "\t") + lines[0])
        for l in lines[1:]:
            print(("       " if numbers else "\t") + l)

def sh_history(*args):
    flags, vals, rest = _split_opts(args, "d")
    h = _history()
    if "c" in flags:
        _write_history([])
        return None
    if "d" in vals:
        try:
            k = int(vals["d"])
            k = k - 1 if k > 0 else len(h) + k
            del h[k]
        except (ValueError, IndexError):
            raise ValueError(f"history: {vals['d']}: history position out of range") from None
        _write_history(h)
        return None
    if "s" in flags:
        h.append(" ".join(rest))
        _write_history(h)
        return None
    for f in flags:
        if f in "wanr":
            return None
        raise ValueError(f"history: -{f}: invalid option\nhistory: usage: history [N] | -c | -d N | -s text")
    if rest:
        try:
            n = int(rest[0])
        except ValueError:
            raise ValueError(f"history: {rest[0]}: numeric argument required") from None
        _show_history(h[-n:] if n else [], len(h) - min(n, len(h)) + 1)
    else:
        _show_history(h)
    return None

def sh_fc(*args):
    flags, _, rest = _split_opts(args, "e")
    if "l" not in flags:
        raise NotImplementedError("fc: tessh edits and runs a past command at the prompt: ↑ or Ctrl+R finds it. fc -l (or history) lists them.")
    h = _history()
    def at(s):
        try:
            k = int(s)
            return k - 1 if k > 0 else len(h) + k
        except ValueError:
            for k in range(len(h) - 1, -1, -1):
                if h[k].startswith(s):
                    return k
            raise ValueError(f"fc: {s}: no command found") from None
    first = at(rest[0]) if rest else max(len(h) - 16, 0)
    last = at(rest[1]) if len(rest) > 1 else len(h) - 1
    lo, hi = sorted((max(first, 0), max(last, 0)))
    sel = list(enumerate(h[lo:hi + 1], lo + 1))
    if "r" in flags:
        sel.reverse()
    for k, e in sel:
        print(("\t" if "n" in flags else f"{k}\t") + e.replace("\n", "\n\t"))
    return None

def sh_caller(*args):
    raise NotImplementedError("caller: " + "tessh's functions are Python's: traceback.print_stack shows the calls (import traceback), and inspect.stack gives them as values.")

def sh_enable(*args):
    raise NotImplementedError("enable: tessh's builtins can't be turned off: a function of the same name takes a builtin's place (def cd …), and \\name runs the program of that name.")

def sh_builtin(name=None, *args, _ns=None):
    if name is None:
        return None
    b = _tessh().builtin_named(name)
    if b is None:
        raise ValueError(f"builtin: {name}: not a tessh builtin")
    kw = {"_ns": _ns} if getattr(b, "_tessh_wants_ns", False) else {}
    return b(*args, **kw)

# ---- type, command -v, hash ------------------------------------------------------------

_DUALS = ("type", "hash", "exec", "eval")

def _keywords():
    import keyword
    return set(keyword.kwlist) | {"let", "do", "defmacro", "defop", "match", "case"}

def _which_all(name):
    if "/" in name:
        return [name] if os.path.isfile(name) and os.access(name, os.X_OK) else []
    out = []
    for d in os.environ.get("PATH", "").split(os.pathsep):
        p = os.path.join(d or ".", name)
        if os.path.isfile(p) and os.access(p, os.X_OK) and p not in out:
            out.append(p)
    return out

def _what(name, ns, every=False, path_only=False):
    """[(kind, description, path or None)] for a command name, first first."""
    import builtins
    T = _tessh()
    out = []
    if not path_only:
        if name in _keywords():
            out.append(("keyword", f"{name} is a tessh keyword", None))
        v = ns.get(name, None) if ns is not None and name in ns else None
        if ns is not None and name in ns:
            if (type(v).__name__ == "Builtin" and type(v).__module__ == "__tessh__") or getattr(v, "__module__", None) in ("__tessh__", "__tessh_shell__", "__tessh_prelude__"):
                out.append(("builtin", f"{name} is a tessh builtin", None))
            elif callable(v):
                what = "command function" if getattr(v, "_tessh_command", False) else "function"
                where = ""
                try:
                    import inspect
                    f = inspect.getsourcefile(v)
                    line = inspect.getsourcelines(v)[1]
                    where = f" ({f}:{line})" if f and not f.startswith("<") else ""
                except (TypeError, OSError):
                    pass
                out.append(("function", f"{name} is a {what}{where}", None))
            else:
                r = repr(v)
                r = r if len(r) <= 60 else r[:57] + "…"
                out.append(("variable", f"{name} is a variable: {r}", None))
        elif name in _DUALS:
            out.append(("builtin", f"{name} is a tessh builtin (and, given a value, Python's {name})", None))
        elif hasattr(builtins, name) and not name.startswith("_"):
            out.append(("builtin", f"{name} is a Python builtin", None))
    for p in _which_all(name):
        out.append(("file", f"{name} is {p}", p))
        if not every:
            break
    return out if every else out[:1]

def sh_type(*args, _ns=None):
    flags, _, names = _split_opts(args)
    for f in flags:
        if f not in "aftpP":
            raise ValueError(f"type: -{f}: invalid option\ntype: usage: type [-afptP] name …")
    every, terse, path, force = "a" in flags, "t" in flags, "p" in flags, "P" in flags
    code = 0
    for n in names:
        found = _what(n, _ns, every, force)
        if not found:
            if not (terse or path):
                _err(f"type: {n}: not found")
            code = 1
            continue
        for kind, text, p in found:
            if path or force:
                if p:
                    print(p)
            elif terse:
                print(kind)
            else:
                print(text)
    return _status(code) if code else None

def command_opts(words, ns):
    flags, _, rest = _split_opts(words)
    if "p" in flags and not ("v" in flags or "V" in flags):
        return _tessh().command(*rest)
    code = 0
    for n in rest:
        found = _what(n, ns)
        if not found:
            if "V" in flags:
                _err(f"command: {n}: not found")
            code = 1
            continue
        kind, text, p = found[0]
        print(text if "V" in flags else (p or n))
    return _status(code)

def sh_hash(*args, _ns=None):
    flags, vals, names = _split_opts(args, "p")
    if not args:
        print("hash: tessh doesn't remember where programs are: it looks each one up on $PATH as it runs it")
        return None
    if "r" in flags or "d" in flags or "l" in flags or "p" in vals:
        return None
    code = 0
    for n in names:
        ps = _which_all(n)
        if not ps:
            _err(f"hash: {n}: not found")
            code = 1
        elif "t" in flags:
            print(ps[0] if len(names) == 1 else f"{n}\t{ps[0]}")
    return _status(code) if code else None

# ---- exec, eval ------------------------------------------------------------------------

def sh_exec(*args):
    flags, vals, rest = _split_opts(args, "a")
    for f in flags:
        if f not in "cl":
            raise ValueError(f"exec: -{f}: invalid option\nexec: usage: exec [-cl] [-a name] program [arg …]")
    if not rest:
        raise NotImplementedError("exec: which program? exec with only redirections (exec >log, exec 3<file) isn't in tessh: redirect a block (with open …) or a function's call instead")
    prog = rest[0]
    argv0 = vals.get("a", prog)
    if "l" in flags:
        argv0 = "-" + os.path.basename(argv0)
    _tessh().exec_program(prog, argv0, list(rest[1:]), "c" in flags)

def sh_eval(*args, _ns=None):
    text = " ".join(args)
    if not text.strip():
        return None
    return _tessh().eval_tessh(text, _ns)
"#####;
