# Tessera shell integration for xonsh: prompt and command marks (OSC 133),
# the working directory (OSC 7), the command line (OSC 1337
# TesseraCommand), the exported environment at each prompt (for Tessera's
# kernels and prompt), and what another shell handed over (folder, exports).
# (Loaded with `xonsh --rc tessera.xsh`: the user's own rc files first.)
import os as _tessera_os
import json as _tessera_json
import base64 as _tessera_b64
import socket as _tessera_socket

for _tessera_rc in ["~/.config/xonsh/rc.xsh", "~/.xonshrc"]:
    _tessera_rc = _tessera_os.path.expanduser(_tessera_rc)
    if _tessera_os.path.isfile(_tessera_rc):
        source @(_tessera_rc)
del _tessera_rc

_tessera_running = [False]


def _tessera_dir():
    return _tessera_os.path.join(_tessera_os.environ.get("XDG_RUNTIME_DIR", "/tmp"), "tessera-env")


def _tessera_out(s):
    import sys
    sys.stdout.write(s)
    sys.stdout.flush()


@events.on_precommand
def _tessera_precommand(cmd, **kw):
    _tessera_running[0] = True
    line = _tessera_b64.b64encode(cmd.rstrip("\n").encode()).decode()
    # What it runs with (OSC 1337 TesseraExec): user, line, environment.
    try:
        import getpass as _gp
        _env = "\0".join(k + "=" + str(v) for k, v in _tessera_os.environ.items())
        _ex = _tessera_b64.b64encode((_gp.getuser() + "\0" + cmd.rstrip("\n") + "\0" + _env).encode()).decode()
        _tessera_out("\x1b]1337;TesseraExec=" + _ex + "\x07")
    except Exception:
        pass
    _tessera_out("\x1b]1337;TesseraCommand=" + line + "\x07\x1b]133;C\x07")


@events.on_postcommand
def _tessera_postcommand(cmd, rtn, out, ts, **kw):
    if _tessera_running[0]:
        _tessera_out("\x1b]133;D;%s\x07" % (rtn if rtn is not None else 0))
    _tessera_running[0] = False


@events.on_pre_prompt
def _tessera_pre_prompt(**kw):
    d = _tessera_dir()
    pid = str(_tessera_os.getpid())
    f = _tessera_os.path.join(d, pid + ".apply")
    if _tessera_os.path.isfile(f):
        try:
            with open(f) as h:
                a = _tessera_json.load(h)
            for k, v in (a.get("set") or {}).items():
                __xonsh__.env[k] = v
            for k in a.get("unset") or []:
                __xonsh__.env.pop(k, None)
            if a.get("cd"):
                cd @(a["cd"])
        except Exception:
            pass
        for x in (f, _tessera_os.path.join(d, pid + ".cwd")):
            try:
                _tessera_os.remove(x)
            except OSError:
                pass
    try:
        _tessera_os.makedirs(d, mode=0o700, exist_ok=True)
        env = __xonsh__.env.detype()
        with open(_tessera_os.path.join(d, pid), "w") as h:
            h.write("".join(k + "=" + v + "\0" for k, v in env.items()))
    except Exception:
        pass
    _tessera_out("\x1b]7;file://%s%s\x07\x1b]133;A\x07" % (_tessera_socket.gethostname(), _tessera_os.getcwd()))
