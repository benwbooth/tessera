"""What compiled tessh calls at run time (as `__tessh__`): running programs,
pipes and redirections, words (globs, ~), the environment, and the shell
builtins (cd, export, …).
"""

from __future__ import annotations

import base64
import builtins
import contextlib
import glob as _glob
import io
import json
import os
import shutil
import signal
import subprocess
import sys
import threading


class State:
    status = 0
    interactive = False
    display = None
    oldpwd = None


state = State()
dirstack: list = []
jobs: list = []


# ---- results -----------------------------------------------------------------

class Output(str):
    """A program's standard output (minus the last newline), with its exit
    status. True when the program succeeded."""

    def __new__(cls, text="", code=0, err="", attached=False):
        o = str.__new__(cls, text)
        o.code = code
        o.err = err
        o.attached = attached
        return o

    @property
    def ok(self):
        return self.code == 0

    @property
    def lines(self):
        return self.splitlines()

    def __bool__(self):
        return self.code == 0

    def __repr__(self):
        if self.attached and not str(self):
            return f"<exit {self.code}>"
        r = str.__repr__(self)
        return r if self.code == 0 else f"Output({r}, code={self.code})"


class Job:
    _n = 0

    def __init__(self, procs=None, words=None, thread=None):
        Job._n += 1
        self.n = Job._n
        self.procs = procs or []
        self.words = words or ""
        self.thread = thread
        self.result = None
        self.stopped = False

    @property
    def pid(self):
        return self.procs[0].pid if self.procs else None

    def done(self):
        if self.thread is not None:
            return not self.thread.is_alive()
        return all(p.poll() is not None for p in self.procs)

    def wait(self):
        if self.thread is not None:
            self.thread.join()
            return self.result
        for p in self.procs:
            p.wait()
        code = self.procs[-1].returncode if self.procs else 0
        state.status = code
        return Output("", code, attached=True)

    def __repr__(self):
        what = "done" if self.done() else ("stopped" if self.stopped else "running")
        return f"[{self.n}] {what} {self.words}"


def show(v):
    """Show a value at the REPL (the last form of an input)."""
    if v is None or (isinstance(v, Output) and v.attached):
        return
    if state.display is not None:
        state.display(v)
    else:
        builtins._ = v
        print(repr(v))


def stmt(v):
    return None if isinstance(v, Output) and v.attached else v


def status():
    return state.status


# ---- names ---------------------------------------------------------------------

def lookup(ns, name):
    """$name for a name the compiler didn't know: the module's, a builtin, or
    an environment variable."""
    if name in ns:
        return ns[name]
    b = ns.get("__builtins__", builtins)
    b = b if isinstance(b, dict) else vars(b)
    if name in b:
        return b[name]
    if name in os.environ:
        return os.environ[name]
    raise NameError(f"name '{name}' is not defined (and there's no environment variable {name})")


def free(ns, name):
    """A macro template's free name, where a user's local of the same name
    would hide it."""
    if name in ns:
        return ns[name]
    b = ns.get("__builtins__", builtins)
    b = b if isinstance(b, dict) else vars(b)
    if name in b:
        return b[name]
    raise NameError(f"name '{name}' is not defined")


def auto(v):
    """A name alone on a line: called if it's callable."""
    return v() if callable(v) else v


# ---- words ---------------------------------------------------------------------

class _Glob(str):
    pass


def g(pattern):
    return _Glob(pattern)


def t(word):
    return os.path.expanduser(word) if isinstance(word, str) and word.startswith("~") else word


GLOB_CHARS = set("*?[")


def argv(items):
    out = []
    for x in items:
        if isinstance(x, _Glob):
            p = os.path.expanduser(str(x))
            m = sorted(_glob.glob(p, recursive=True)) if GLOB_CHARS & set(p) else []
            out += m or [p]
        elif isinstance(x, (list, tuple)):
            out += [_word(i) for i in x]
        elif x is None or (isinstance(x, Output) and x.attached and not str(x)):
            continue
        else:
            out.append(_word(x))
    return out


def _word(x):
    if isinstance(x, bytes):
        return x.decode(errors="surrogateescape")
    if isinstance(x, os.PathLike):
        return os.fspath(x)
    return str(x)


# ---- running -------------------------------------------------------------------

class Cmd:
    """A program to run (one stage of a pipe)."""

    def __init__(self, words, redirs=()):
        self.words = words
        self.redirs = list(redirs)

    def __repr__(self):
        return "Cmd(" + " ".join(self.words) + ")"


class CallStage:
    def __init__(self, fn):
        self.fn = fn


class ValueStage:
    def __init__(self, value):
        self.value = value


def call_stage(fn):
    return CallStage(fn)


def value_stage(v):
    return ValueStage(v)


def _resolve_callable(ns, name):
    if "." in name or "/" in name:
        return None
    if name in ns:
        v = ns[name]
        return v if callable(v) else None
    b = ns.get("__builtins__", builtins)
    b = b if isinstance(b, dict) else vars(b)
    v = b.get(name)
    return v if callable(v) else None


def cmd(ns, head, args, redirs=(), mode="attach", force=False):
    """A command: a Python function by that name if there is one (unless
    forced), else a program. mode: attach, capture, bg or stage."""
    words = argv(args)
    if not force and isinstance(head, str):
        f = _resolve_callable(ns, head)
        if f is not None:
            kw = {"_ns": ns} if getattr(f, "_tessh_wants_ns", False) else {}
            if mode == "stage":
                return CallStage(lambda inp: f(*words, inp, **kw))
            if mode == "bg":
                return bg_call(lambda: f(*words, **kw))
            return redirect(lambda: f(*words, **kw), redirs)
    prog = _word(head)
    c = Cmd([prog] + words, redirs)
    if mode == "stage":
        return c
    return pipe([c], mode)


def _open_redirs(redirs, stdin, stdout, stderr, opened):
    for op, target in redirs:
        op = op.lstrip("1") if op.startswith("1>") else op
        if op in (">", ">>"):
            f = open(os.path.expanduser(_word(target)), "a" if op == ">>" else "w")
            opened.append(f)
            stdout = f
        elif op == "<":
            f = open(os.path.expanduser(_word(target)), "rb")
            opened.append(f)
            stdin = f
        elif op in ("2>", "2>>"):
            f = open(os.path.expanduser(_word(target)), "a" if op == "2>>" else "w")
            opened.append(f)
            stderr = f
        elif op in ("&>", "&>>"):
            f = open(os.path.expanduser(_word(target)), "a" if op == "&>>" else "w")
            opened.append(f)
            stdout = stderr = f
        elif op in ("2>&1", "&>&1"):
            stderr = subprocess.STDOUT if stdout in (None, subprocess.PIPE) else stdout
        elif op in (">&2", "1>&2"):
            stdout = sys.stderr.fileno() if stderr is None else stderr
        elif op == "2>&2" or op == "1>&1":
            pass
        else:
            raise ValueError(f"unknown redirection {op}")
    return stdin, stdout, stderr


def _feed(data):
    if isinstance(data, bytes):
        return data
    if isinstance(data, str):
        return data.encode()
    if data is None:
        return b""
    try:
        it = iter(data)
    except TypeError:
        return str(data).encode()
    return "".join(_word(x) + "\n" for x in it).encode()


def _tty():
    try:
        return state.interactive and os.isatty(0) and os.tcgetpgrp(0) == os.getpgrp()
    except OSError:
        return False


def _run_procs(cmds, data, has_data, capture, bg):
    """Run a chain of programs connected by pipes."""
    procs, opened = [], []
    # (What Python printed so far goes first.)
    for f in (sys.stdout, sys.stderr):
        try:
            f.flush()
        except Exception:
            pass
    job_control = _tty() and not capture
    pgid = 0
    prev = None
    try:
        for i, c in enumerate(cmds):
            last = i == len(cmds) - 1
            stdin = prev.stdout if prev is not None else (subprocess.PIPE if has_data else None)
            stdout = subprocess.PIPE if (not last or capture) else None
            stderr = None
            stdin, stdout, stderr = _open_redirs(c.redirs, stdin, stdout, stderr, opened)
            kw = {}
            if job_control or bg:
                kw["process_group"] = pgid
            try:
                p = subprocess.Popen(c.words, stdin=stdin, stdout=stdout, stderr=stderr, **kw)
            except FileNotFoundError:
                print(f"tessh: command not found: {c.words[0]}", file=sys.stderr)
                for q in procs:
                    q.kill()
                state.status = 127
                return Output("", 127, attached=not capture)
            except PermissionError:
                print(f"tessh: permission denied: {c.words[0]}", file=sys.stderr)
                state.status = 126
                return Output("", 126, attached=not capture)
            if (job_control or bg) and pgid == 0:
                pgid = p.pid
            if prev is not None and prev.stdout is not None:
                prev.stdout.close()
            procs.append(p)
            prev = p
        if has_data and procs and procs[0].stdin is not None:
            payload = _feed(data)
            first = procs[0]

            def write():
                try:
                    first.stdin.write(payload)
                    first.stdin.close()
                except (BrokenPipeError, OSError):
                    pass
            threading.Thread(target=write, daemon=True).start()
        if bg:
            j = Job(procs, " ".join(cmds[0].words))
            jobs.append(j)
            if state.interactive:
                print(f"[{j.n}] {j.pid}", file=sys.stderr)
            return j
        if capture:
            out = procs[-1].stdout.read()
            for p in procs:
                _wait(p)
            code = procs[-1].returncode
            state.status = code
            text = out.decode(errors="surrogateescape")
            if text.endswith("\n"):
                text = text[:-1]
            return Output(text, code)
        if job_control:
            code = _wait_foreground(procs, pgid, cmds)
        else:
            for p in procs:
                _wait(p)
            code = procs[-1].returncode
        state.status = code
        return Output("", code, attached=True)
    finally:
        for f in opened:
            f.close()


def _wait(p):
    while True:
        try:
            return p.wait()
        except KeyboardInterrupt:
            # (The program had the Ctrl+C too: let it finish, then stop.)
            try:
                p.wait()
            finally:
                raise


def _wait_foreground(procs, pgid, cmds):
    """Give the terminal to the programs, wait (or see them stopped), take it
    back."""
    old = signal.signal(signal.SIGTTOU, signal.SIG_IGN)
    try:
        os.tcsetpgrp(0, pgid)
    except OSError:
        pass
    code = 0
    stopped = False
    try:
        left = {p.pid: p for p in procs}
        while left:
            try:
                pid, st = os.waitpid(-pgid, os.WUNTRACED)
            except ChildProcessError:
                break
            except InterruptedError:
                continue
            p = left.get(pid)
            if os.WIFSTOPPED(st):
                stopped = True
                break
            if p is not None:
                p.returncode = os.waitstatus_to_exitcode(st)
                del left[pid]
        code = procs[-1].returncode if procs[-1].returncode is not None else 0
        if code < 0:
            code = 128 - code
    finally:
        try:
            os.tcsetpgrp(0, os.getpgrp())
        except OSError:
            pass
        signal.signal(signal.SIGTTOU, old)
    if stopped:
        j = Job(procs, " ".join(cmds[0].words))
        j.stopped = True
        jobs.append(j)
        print(f"\n[{j.n}] stopped {j.words}", file=sys.stderr)
        return 148
    if code == 130:
        raise KeyboardInterrupt
    return code


def pipe(stages, mode="attach"):
    """Run a pipe: programs, Python calls (given what came before as their last
    argument) and, first, a value to feed in."""
    capture = mode == "capture"
    bg = mode == "bg"
    if bg:
        j = Job(words="(pipe)")
        def run():
            j.result = pipe(stages, "capture")
        j.thread = threading.Thread(target=run, daemon=True)
        if all(isinstance(s, Cmd) for s in stages):
            return _run_procs(stages, None, False, False, True)
        jobs.append(j)
        j.thread.start()
        return j
    data, has_data = None, False
    i = 0
    result = None
    while i < len(stages):
        s = stages[i]
        if isinstance(s, ValueStage):
            data, has_data = s.value, True
            result = data
            i += 1
        elif isinstance(s, CallStage):
            data = s.fn(data if has_data else None)
            has_data = True
            result = data
            state.status = 0
            i += 1
        elif isinstance(s, Cmd):
            j = i
            while j < len(stages) and isinstance(stages[j], Cmd):
                j += 1
            last = j == len(stages)
            result = _run_procs(stages[i:j], data, has_data, capture or not last, False)
            data, has_data = result, True
            i = j
        else:
            data, has_data = s, True
            result = s
            i += 1
    return result


def redirect(thunk, redirs):
    """Run a Python call with its standard streams redirected."""
    if not redirs:
        return thunk()
    with contextlib.ExitStack() as st:
        for op, target in redirs:
            op = op.lstrip("1") if op.startswith("1>") else op
            if op in (">", ">>", "&>", "&>>"):
                f = st.enter_context(open(os.path.expanduser(_word(target)), "a" if op.endswith(">>") else "w"))
                st.enter_context(contextlib.redirect_stdout(f))
                if op.startswith("&"):
                    st.enter_context(contextlib.redirect_stderr(f))
            elif op in ("2>", "2>>"):
                f = st.enter_context(open(os.path.expanduser(_word(target)), "a" if op == "2>>" else "w"))
                st.enter_context(contextlib.redirect_stderr(f))
            elif op == "2>&1":
                st.enter_context(contextlib.redirect_stderr(sys.stdout))
            elif op in (">&2",):
                st.enter_context(contextlib.redirect_stdout(sys.stderr))
            elif op == "<":
                f = st.enter_context(open(os.path.expanduser(_word(target))))
                old = sys.stdin
                sys.stdin = f
                st.callback(setattr, sys, "stdin", old)
        return thunk()


def bg_call(thunk):
    j = Job(words="(python)")
    def run():
        j.result = thunk()
    j.thread = threading.Thread(target=run, daemon=True)
    jobs.append(j)
    j.thread.start()
    return j


# ---- the environment -----------------------------------------------------------

class Env:
    """$env: the environment, by attribute or by key."""

    def __getattr__(self, k):
        if k.startswith("__"):
            raise AttributeError(k)
        return os.environ.get(k)

    def __setattr__(self, k, v):
        os.environ[k] = _word(v)

    def __delattr__(self, k):
        os.environ.pop(k, None)

    def __getitem__(self, k):
        return os.environ[k]

    def __setitem__(self, k, v):
        os.environ[k] = _word(v)

    def __delitem__(self, k):
        del os.environ[k]

    def __contains__(self, k):
        return k in os.environ

    def __iter__(self):
        return iter(os.environ)

    def __len__(self):
        return len(os.environ)

    def get(self, k, default=None):
        return os.environ.get(k, default)

    def items(self):
        return os.environ.items()

    def keys(self):
        return os.environ.keys()

    def __repr__(self):
        return f"<env: {len(os.environ)} variables>"


env = Env()


def apply_env(b64):
    """From Tessera: variables set and unset, and a folder (base64 JSON)."""
    a = json.loads(base64.urlsafe_b64decode(b64 + "=" * (-len(b64) % 4)))
    for k, v in (a.get("set") or {}).items():
        os.environ[k] = v
    for k in a.get("unset") or []:
        os.environ.pop(k, None)
    if a.get("cd"):
        _chdir(a["cd"])


def dump_env(b64_env, b64_pwd):
    """For Tessera: the environment and folder, written to two files."""
    dec = lambda s: base64.urlsafe_b64decode(s + "=" * (-len(s) % 4)).decode()
    with open(dec(b64_env), "w") as f:
        f.write("".join(f"{k}={v}\0" for k, v in os.environ.items()))
    with open(dec(b64_pwd), "w") as f:
        f.write(os.getcwd())


# ---- builtins ------------------------------------------------------------------

def command(f):
    """Make a function a command function: its arguments are read as words."""
    f._tessh_command = True
    return f


def _chdir(d):
    old = os.getcwd()
    os.chdir(d)
    state.oldpwd = old
    os.environ["OLDPWD"] = old
    os.environ["PWD"] = os.getcwd()


@command
def cd(*args):
    if len(args) > 1:
        raise TypeError("cd: too many arguments")
    d = args[0] if args else os.path.expanduser("~")
    if d == "-":
        d = state.oldpwd or os.environ.get("OLDPWD") or os.getcwd()
        print(d)
    _chdir(os.path.expanduser(d))


@command
def pushd(d=None):
    d = os.path.expanduser(d) if d else (dirstack.pop() if dirstack else os.path.expanduser("~"))
    dirstack.append(os.getcwd())
    _chdir(d)
    print(" ".join([os.getcwd()] + dirstack[::-1]))


@command
def popd():
    if not dirstack:
        raise IndexError("popd: the folder stack is empty")
    _chdir(dirstack.pop())
    print(" ".join([os.getcwd()] + dirstack[::-1]))


@command
def dirs():
    print(" ".join([os.getcwd()] + dirstack[::-1]))


def _export(*args, _ns=None):
    for a in args:
        if "=" in a:
            k, v = a.split("=", 1)
            os.environ[k] = v
        elif _ns is not None and a in _ns:
            os.environ[a] = _word(_ns[a])
        elif a not in os.environ:
            os.environ[a] = ""


_export._tessh_wants_ns = True
_export._tessh_command = True
export = _export


@command
def unset(*names):
    for n in names:
        os.environ.pop(n, None)


@command
def exit(code=0):
    raise SystemExit(int(code))


def _source(path, *args, _ns=None):
    from .runner import run_file
    run_file(os.path.expanduser(path), ns=_ns, argv=list(args), fresh=False)


_source._tessh_wants_ns = True
_source._tessh_command = True
source = _source


@command
def which(*names):
    found = None
    for n in names:
        p = shutil.which(n)
        print(f"{n}: {p}" if p else f"{n}: not found")
        found = found or p
    return Output(found or "", 0 if found else 1)


@command
def glob(*words):
    return list(words)


@command
def jobs_(*_):
    global jobs
    jobs[:] = [j for j in jobs if not j.done() or j.stopped]
    for j in jobs:
        print(repr(j))
    return None


@command
def fg(n=None):
    if not jobs:
        raise IndexError("fg: no jobs")
    j = jobs[-1] if n is None else next(j for j in jobs if str(j.n) == str(n).lstrip("%"))
    if j.thread is not None:
        return j.wait()
    jobs.remove(j)
    j.stopped = False
    pgid = os.getpgid(j.pid)
    os.killpg(pgid, signal.SIGCONT)
    if _tty():
        code = _wait_foreground(j.procs, pgid, [Cmd(j.words.split())])
    else:
        for p in j.procs:
            p.wait()
        code = j.procs[-1].returncode
    state.status = code
    return Output("", code, attached=True)


@command
def bg(n=None):
    j = jobs[-1] if n is None else next(j for j in jobs if str(j.n) == str(n).lstrip("%"))
    j.stopped = False
    os.killpg(os.getpgid(j.pid), signal.SIGCONT)


BUILTINS = {
    "cd": cd, "pushd": pushd, "popd": popd, "dirs": dirs, "export": export, "unset": unset,
    "exit": exit, "source": source, "which": which, "glob": glob, "jobs": jobs_, "fg": fg,
    "bg": bg, "command": command, "env": env, "Output": Output,
}
