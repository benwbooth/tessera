"""tessh's debugger: a Debug Adapter Protocol server (`tessh --dap`).

tessh compiles a script to Python whose code keeps the script's own file
name and line numbers, so Python's tracing sees tessh lines: breakpoints
(with conditions, hit counts and log messages), stepping in, over and out,
pausing, the call stack, variables (expandable), and evaluating tessh or
Python in the stopped frame (assignments too) all work on the .tsh file.

The protocol runs on the file descriptors the caller gives (tessh's own
stdin and stdout); the program's output, sent to a pipe, comes back as
output events. The program runs on this (the main) thread under
sys.settrace; requests are read on a thread of their own, which answers
questions about a stopped program while the main thread waits.
"""

import json
import os
import sys
import threading

THREAD_ID = 1


class Debugger:
    def __init__(self, dap_in, dap_out, out_pipe, run, tessh_eval=None):
        # run(program, args) runs the script and gives its exit code.
        # tessh_eval(text, globals) evaluates tessh (or None to use Python).
        self.inp = dap_in
        self.out = dap_out
        self.pipe = out_pipe
        self.run = run
        self.tessh_eval = tessh_eval
        self.seq = 1
        self.wlock = threading.Lock()
        self.cv = threading.Condition()
        self.configured = False
        self.launch = {}
        # path -> {line: {"condition", "hitCondition", "logMessage", "hits"}}
        self.breakpoints = {}
        # How the program goes on: "run", "in", "over", "out", "pause".
        self.mode = "run"
        self.target = None
        self.stopped = False
        self.resume = False
        self.frames = []
        self.refs = {}
        self.terminated = False
        self.stop_on_entry = False
        self.entered = False
        self.program = ""

    # ---- the wire ---------------------------------------------------------------------

    def send(self, msg):
        msg["seq"] = self.seq
        self.seq += 1
        body = json.dumps(msg).encode()
        data = b"Content-Length: %d\r\n\r\n" % len(body) + body
        with self.wlock:
            view = memoryview(data)
            while view:
                n = os.write(self.out, view)
                view = view[n:]

    def event(self, name, body=None):
        self.send({"type": "event", "event": name, "body": body or {}})

    def reply(self, req, body=None, ok=True, message=None):
        m = {"type": "response", "request_seq": req["seq"], "success": ok, "command": req["command"], "body": body or {}}
        if message:
            m["message"] = message
        self.send(m)

    def read_message(self, f):
        length = None
        while True:
            line = f.readline()
            if not line:
                return None
            line = line.strip()
            if not line:
                if length is not None:
                    break
                continue
            k, _, v = line.decode().partition(":")
            if k.lower() == "content-length":
                length = int(v.strip())
        return json.loads(f.read(length))

    # ---- threads ----------------------------------------------------------------------

    def pump_output(self):
        f = os.fdopen(self.pipe, "rb", buffering=0)
        while True:
            data = f.read(65536)
            if not data:
                return
            self.event("output", {"category": "stdout", "output": data.decode("utf-8", "replace")})

    def read_loop(self):
        f = os.fdopen(self.inp, "rb")
        while True:
            m = self.read_message(f)
            if m is None:
                self.terminate()
                return
            if m.get("type") != "request":
                continue
            try:
                self.handle(m)
            except Exception as e:  # (A bad request mustn't stop the debugger.)
                self.reply(m, ok=False, message=f"{type(e).__name__}: {e}")

    # ---- requests ---------------------------------------------------------------------

    def handle(self, req):
        cmd = req["command"]
        a = req.get("arguments") or {}
        if cmd == "initialize":
            self.reply(req, {
                "supportsConfigurationDoneRequest": True,
                "supportsConditionalBreakpoints": True,
                "supportsHitConditionalBreakpoints": True,
                "supportsLogPoints": True,
                "supportsEvaluateForHovers": True,
                "supportsSetVariable": True,
                "supportsSetExpression": True,
                "supportsTerminateRequest": True,
            })
        elif cmd in ("launch", "attach"):
            self.launch = a
            self.stop_on_entry = bool(a.get("stopOnEntry"))
            self.reply(req)
            self.event("initialized")
        elif cmd == "setBreakpoints":
            path = os.path.abspath(a.get("source", {}).get("path", ""))
            bps = {}
            out = []
            lines = a.get("breakpoints")
            if lines is None:
                lines = [{"line": n} for n in a.get("lines", [])]
            for b in lines:
                bps[int(b["line"])] = {"condition": b.get("condition") or "", "hitCondition": b.get("hitCondition") or "", "logMessage": b.get("logMessage") or "", "hits": 0}
                out.append({"verified": True, "line": int(b["line"])})
            with self.cv:
                self.breakpoints[path] = bps
            self.reply(req, {"breakpoints": out})
        elif cmd == "setExceptionBreakpoints":
            self.reply(req, {"breakpoints": []})
        elif cmd == "configurationDone":
            self.reply(req)
            with self.cv:
                self.configured = True
                self.cv.notify_all()
        elif cmd == "threads":
            self.reply(req, {"threads": [{"id": THREAD_ID, "name": "main"}]})
        elif cmd == "stackTrace":
            frames = []
            for i, fr in enumerate(self.frames):
                code = fr.f_code
                path = os.path.abspath(code.co_filename)
                src = {"name": os.path.basename(path), "path": path} if os.path.exists(path) else None
                frames.append({"id": i + 1, "name": code.co_name if code.co_name != "<module>" else os.path.basename(path), "line": fr.f_lineno, "column": 1, "source": src,
                               "presentationHint": "normal" if self.mine(path) else "subtle"})
            self.reply(req, {"stackFrames": frames, "totalFrames": len(frames)})
        elif cmd == "scopes":
            fr = self.frame(a.get("frameId"))
            scopes = []
            if fr is not None:
                scopes.append({"name": "Locals", "variablesReference": self.ref(("locals", fr)), "expensive": False})
                if fr.f_globals is not fr.f_locals:
                    scopes.append({"name": "Globals", "variablesReference": self.ref(("globals", fr)), "expensive": False})
            self.reply(req, {"scopes": scopes})
        elif cmd == "variables":
            self.reply(req, {"variables": self.children(a.get("variablesReference", 0))})
        elif cmd == "evaluate":
            fr = self.frame(a.get("frameId")) or (self.frames[0] if self.frames else None)
            if fr is None:
                self.reply(req, ok=False, message="the program isn't stopped")
                return
            try:
                v, shown = self.evaluate(a.get("expression", ""), fr, a.get("context") == "repl")
                self.reply(req, {"result": shown, "type": type(v).__name__ if shown else "", "variablesReference": self.ref(v) if expandable(v) else 0})
            except Exception as e:
                self.reply(req, ok=False, message=f"{type(e).__name__}: {e}")
        elif cmd in ("setExpression", "setVariable"):
            fr = self.frame(a.get("frameId")) or (self.frames[0] if self.frames else None)
            if fr is None:
                self.reply(req, ok=False, message="the program isn't stopped")
                return
            target = a.get("expression") or a.get("name")
            try:
                v = self.value(a.get("value", ""), fr)
                if cmd == "setVariable" and a.get("variablesReference") in self.refs:
                    self.set_child(self.refs[a["variablesReference"]], target, v)
                else:
                    loc = fr.f_locals
                    loc["__dap_value__"] = v
                    try:
                        exec(f"{target} = __dap_value__", fr.f_globals, loc)
                    finally:
                        loc.pop("__dap_value__", None)
                self.reply(req, {"value": short(v), "type": type(v).__name__, "variablesReference": self.ref(v) if expandable(v) else 0})
            except Exception as e:
                self.reply(req, ok=False, message=f"{type(e).__name__}: {e}")
        elif cmd in ("continue", "next", "stepIn", "stepOut"):
            mode = {"continue": "run", "next": "over", "stepIn": "in", "stepOut": "out"}[cmd]
            self.reply(req, {"allThreadsContinued": True} if cmd == "continue" else None)
            self.go(mode)
        elif cmd == "pause":
            self.reply(req)
            with self.cv:
                if not self.stopped:
                    self.mode = "pause"
        elif cmd in ("disconnect", "terminate"):
            self.reply(req)
            self.terminate()
        else:
            self.reply(req)

    def terminate(self):
        with self.cv:
            self.terminated = True
            self.resume = True
            self.configured = True
            self.cv.notify_all()

    def go(self, mode):
        with self.cv:
            self.mode = mode
            self.target = self.frames[0] if self.frames else None
            self.resume = True
            self.cv.notify_all()
        self.event("continued", {"threadId": THREAD_ID, "allThreadsContinued": True})

    # ---- values -----------------------------------------------------------------------

    def frame(self, fid):
        if isinstance(fid, int) and 1 <= fid <= len(self.frames):
            return self.frames[fid - 1]
        return None

    def ref(self, obj):
        n = len(self.refs) + 1000
        self.refs[n] = obj
        return n

    def children(self, n):
        obj = self.refs.get(n)
        if obj is None:
            return []
        if isinstance(obj, tuple) and len(obj) == 2 and obj[0] in ("locals", "globals"):
            d = obj[1].f_locals if obj[0] == "locals" else obj[1].f_globals
            items = [(k, v) for k, v in d.items() if not hidden(k, v)]
        elif isinstance(obj, dict):
            items = [(repr(k), v) for k, v in list(obj.items())[:500]]
        elif isinstance(obj, (list, tuple)):
            items = [(str(i), v) for i, v in enumerate(obj[:500])]
        elif isinstance(obj, (set, frozenset)):
            items = [(str(i), v) for i, v in enumerate(list(obj)[:500])]
        else:
            items = [(k, v) for k, v in vars(obj).items() if not k.startswith("__")] if hasattr(obj, "__dict__") else []
        return [{"name": k, "value": short(v), "type": type(v).__name__, "variablesReference": self.ref(v) if expandable(v) else 0} for k, v in items]

    def set_child(self, parent, name, v):
        if isinstance(parent, tuple) and len(parent) == 2 and parent[0] in ("locals", "globals"):
            d = parent[1].f_locals if parent[0] == "locals" else parent[1].f_globals
            d[name] = v
        elif isinstance(parent, dict):
            for k in parent:
                if repr(k) == name:
                    parent[k] = v
                    return
        elif isinstance(parent, list):
            parent[int(name)] = v
        else:
            setattr(parent, name, v)

    def value(self, text, fr):
        """An expression's value: Python first, else tessh."""
        try:
            return eval(compile(text, "<debug>", "eval"), fr.f_globals, fr.f_locals)
        except SyntaxError:
            if self.tessh_eval is None:
                raise
            ns = dict(fr.f_globals)
            ns.update(fr.f_locals)
            return self.tessh_eval(text, ns)

    def evaluate(self, text, fr, statement):
        """(value, as shown): a Python expression; in the debug console a
        Python statement too; else tessh (what it assigns kept)."""
        try:
            v = eval(compile(text, "<debug>", "eval"), fr.f_globals, fr.f_locals)
            return v, short(v)
        except SyntaxError:
            pass
        if statement:
            try:
                code = compile(text, "<debug>", "exec")
            except SyntaxError:
                code = None
            if code is not None:
                exec(code, fr.f_globals, fr.f_locals)
                return None, ""
        if self.tessh_eval is None:
            raise SyntaxError(f"not Python: {text}")
        ns = dict(fr.f_globals)
        ns.update(fr.f_locals)
        before = dict(ns)
        v = self.tessh_eval(text, ns)
        for k, val in ns.items():
            if k not in before or before[k] is not val:
                fr.f_locals[k] = val
        return v, "" if v is None else short(v)

    # ---- tracing ----------------------------------------------------------------------

    def mine(self, path):
        """Code to stop in: tessh scripts, and any file with breakpoints."""
        return path.endswith((".tsh", ".tessh")) or path in self.breakpoints or path == self.program

    def trace_call(self, frame, event, arg):
        if self.terminated:
            raise SystemExit(0)
        path = os.path.abspath(frame.f_code.co_filename)
        if self.mine(path):
            return self.trace_line
        return None

    def trace_line(self, frame, event, arg):
        if self.terminated:
            raise SystemExit(0)
        if event != "line":
            return self.trace_line
        path = os.path.abspath(frame.f_code.co_filename)
        reason = None
        if not self.entered:
            self.entered = True
            if self.stop_on_entry:
                reason = "entry"
        if reason is None:
            reason = self.should_stop(frame, path)
        if reason:
            self.stop(frame, reason)
        return self.trace_line

    def should_stop(self, frame, path):
        mode = self.mode
        if mode == "pause":
            return "pause"
        if mode == "in":
            return "step"
        t = self.target
        if mode == "over" and (t is None or frame is t or not ancestor(t, frame)):
            return "step"
        if mode == "out" and (t is None or (frame is not t and not ancestor(t, frame))):
            return "step"
        bp = self.breakpoints.get(path, {}).get(frame.f_lineno)
        if bp is None:
            return None
        if bp["condition"]:
            try:
                if not self.value(bp["condition"], frame):
                    return None
            except Exception:
                return None
        bp["hits"] += 1
        h = bp["hitCondition"].strip()
        if h:
            try:
                n = int(h.lstrip(">=%"))
                if h.startswith("%"):
                    if bp["hits"] % n:
                        return None
                elif h.startswith(">="):
                    if bp["hits"] < n:
                        return None
                elif h.startswith(">"):
                    if bp["hits"] <= n:
                        return None
                elif bp["hits"] != n:
                    return None
            except ValueError:
                pass
        if bp["logMessage"]:
            self.event("output", {"category": "console", "output": self.interpolate(bp["logMessage"], frame) + "\n"})
            return None
        return "breakpoint"

    def interpolate(self, msg, frame):
        out, i = [], 0
        while i < len(msg):
            j = msg.find("{", i)
            if j < 0:
                out.append(msg[i:])
                break
            k = msg.find("}", j)
            if k < 0:
                out.append(msg[i:])
                break
            out.append(msg[i:j])
            try:
                out.append(str(self.value(msg[j + 1:k], frame)))
            except Exception as e:
                out.append(f"<{type(e).__name__}>")
            i = k + 1
        return "".join(out)

    def stop(self, frame, reason):
        fs = []
        f = frame
        while f is not None:
            fs.append(f)
            f = f.f_back
        # (Down to the script: not tessh's own frames, nor the debugger's,
        # under it.)
        at = [i for i, f in enumerate(fs) if os.path.abspath(f.f_code.co_filename) == self.program]
        if at:
            fs = fs[:at[-1] + 1]
        else:
            fs = [f for f in fs if os.path.abspath(f.f_code.co_filename) != os.path.abspath(__file__)] or fs[:1]
        with self.cv:
            self.frames = fs
            self.refs = {}
            self.stopped = True
            self.resume = False
        self.event("stopped", {"reason": reason, "threadId": THREAD_ID, "allThreadsStopped": True})
        with self.cv:
            while not self.resume:
                self.cv.wait()
            self.stopped = False
            self.frames = []
            self.refs = {}
        if self.terminated:
            raise SystemExit(0)

    # ---- running ----------------------------------------------------------------------

    def serve(self):
        pump = threading.Thread(target=self.pump_output, name="dap-output", daemon=True)
        pump.start()
        threading.Thread(target=self.read_loop, name="dap-requests", daemon=True).start()
        with self.cv:
            while not self.configured:
                self.cv.wait()
        if self.terminated:
            return 0
        program = self.launch.get("program") or ""
        self.program = os.path.abspath(program)
        cwd = self.launch.get("cwd")
        if cwd:
            os.chdir(cwd)
        args = [str(x) for x in self.launch.get("args") or []]
        code = 0
        sys.settrace(self.trace_call)
        threading.settrace(self.trace_call)
        try:
            # (The script's exit code; its errors already printed.)
            r = self.run(self.program, args)
            code = r if isinstance(r, int) else 0
        except SystemExit as e:
            code = e.code if isinstance(e.code, int) else 0
        finally:
            sys.settrace(None)
            threading.settrace(None)
        try:
            sys.stdout.flush()
            sys.stderr.flush()
        except Exception:
            pass
        # (All the program's output before the end: its pipe closed, read
        # to the end.)
        try:
            null = os.open(os.devnull, os.O_WRONLY)
            os.dup2(null, 1)
            os.dup2(null, 2)
            os.close(null)
        except OSError:
            pass
        pump.join(timeout=2)
        self.event("exited", {"exitCode": code})
        self.event("terminated")
        return code


def ancestor(a, f):
    """Is frame `a` among `f`'s callers?"""
    f = f.f_back
    while f is not None:
        if f is a:
            return True
        f = f.f_back
    return False


def hidden(k, v):
    import types
    return k.startswith("__") or isinstance(v, types.ModuleType) or k == "__tessh__"


def expandable(v):
    return isinstance(v, (dict, list, tuple, set, frozenset)) and len(v) > 0 or (hasattr(v, "__dict__") and not callable(v) and not isinstance(v, type))


def short(v):
    try:
        s = repr(v)
    except Exception as e:
        s = f"<{type(e).__name__}>"
    return s if len(s) <= 300 else s[:297] + "..."


def serve(dap_in, dap_out, out_pipe, run, tessh_eval=None):
    return Debugger(dap_in, dap_out, out_pipe, run, tessh_eval).serve()
