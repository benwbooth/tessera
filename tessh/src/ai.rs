//! The AI for the terminal editor, as Tessera's input editor has it: `# what
//! you want` becomes a command, and grey suggestions as you type. It uses
//! Tessera's AI settings when there are any (~/.config/tessera/settings.json:
//! the providers, "inline_model" for suggestions, "chat_model" for requests;
//! keys from its secrets.json or the usual variables), else $TESSH_AI_MODEL
//! ("provider/model", e.g. "ollama/qwen3.5:4b" or "anthropic/claude-…"),
//! else a local Ollama if one answers. Requests go out with Python's urllib
//! (no streaming: the reply is short), on whatever thread asks.

use pyo3::prelude::*;

const CODE: &str = r#"
import json, os, re, urllib.request

PRESETS = {
    "ollama": ("openai", "http://localhost:11434/v1", "none", True),
    "llamacpp": ("openai", "http://localhost:8080/v1", "", True),
    "lmstudio": ("openai", "http://localhost:1234/v1", "", True),
    "vllm": ("openai", "http://localhost:8000/v1", "", True),
    "localai": ("openai", "http://localhost:8080/v1", "", True),
    "anthropic": ("anthropic", "https://api.anthropic.com/v1", "", False),
    "openai": ("openai", "https://api.openai.com/v1", "", False),
    "gemini": ("openai", "https://generativelanguage.googleapis.com/v1beta/openai", "", False),
    "openrouter": ("openai", "https://openrouter.ai/api/v1", "", False),
    "groq": ("openai", "https://api.groq.com/openai/v1", "", False),
    "mistral": ("openai", "https://api.mistral.ai/v1", "", False),
    "deepseek": ("openai", "https://api.deepseek.com/v1", "", False),
    "xai": ("openai", "https://api.x.ai/v1", "", False),
}
KEY_VARS = {"anthropic": "ANTHROPIC_API_KEY", "openai": "OPENAI_API_KEY", "gemini": "GEMINI_API_KEY",
            "openrouter": "OPENROUTER_API_KEY", "groq": "GROQ_API_KEY", "mistral": "MISTRAL_API_KEY",
            "deepseek": "DEEPSEEK_API_KEY", "xai": "XAI_API_KEY"}

def _tessera_dir():
    return os.path.join(os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config"), "tessera")

def _load(name):
    try:
        with open(os.path.join(_tessera_dir(), name)) as f:
            return json.load(f)
    except Exception:
        return {}

_ollama = None
def _ollama_models():
    global _ollama
    if _ollama is None:
        _ollama = []
        try:
            with urllib.request.urlopen("http://localhost:11434/api/tags", timeout=0.4) as r:
                ms = json.load(r).get("models", [])
            ms = [m for m in ms if not any(w in m.get("name", "") for w in ("embed", "ocr"))]
            _ollama = [m["name"] for m in sorted(ms, key=lambda m: m.get("size", 0))]
        except Exception:
            pass
    return _ollama

def settings():
    s = dict(_load("settings.json").get("ai", {}) or {})
    m = os.environ.get("TESSH_AI_MODEL", "").strip()
    if m:
        s["chat_model"] = m
        s["inline_model"] = os.environ.get("TESSH_AI_INLINE_MODEL", "").strip() or m
    if not s.get("chat_model") and not s.get("inline_model") and os.environ.get("TESSH_AI", "") != "0":
        ms = _ollama_models()
        if ms:
            s["inline_model"] = "ollama/" + ms[0]
            s["chat_model"] = "ollama/" + ms[-1]
    if os.environ.get("TESSH_AI", "") == "0":
        s["chat_model"] = s["inline_model"] = ""
    return s

def _provider(pid, s):
    for p in s.get("providers", []) or []:
        if p.get("id") == pid:
            return p
    if pid in PRESETS:
        kind, url, effort, local = PRESETS[pid]
        return {"id": pid, "kind": kind, "base_url": url, "reasoning_effort": effort, "local": local}
    return None

def _key(p):
    if p.get("token"):
        return p["token"]
    stored = _load("secrets.json").get(p["id"], "")
    if isinstance(stored, str) and stored.startswith("env:"):
        return os.environ.get(stored[4:], "")
    if stored:
        return stored
    v = KEY_VARS.get(p["id"])
    return os.environ.get(v, "") if v else ""

def _resolve(task):
    s = settings()
    ref = s.get("inline_model" if task in ("inline", "request") else "chat_model") or ""
    if task in ("inline", "request") and s.get("inline") is False:
        return None
    if task == "command" and s.get("natural_language") is False:
        return None
    # Under Hearth (Tessera's "hearth" setting, on by default): commands use
    # Hearth's assistant model; suggestions Tessera's model, loaded through
    # the hub with the desktop's others (empty: the hub's "fast").
    hub = _hub() if s.get("hearth", True) is not False else None
    if hub:
        if task == "command" or not ref:
            return hub, ("default" if task == "command" else "fast")
        pid = ref.split("/", 1)[0]
        p = _provider(pid, s)
        if p is None or p.get("local"):
            return hub, ref
    if "/" not in ref:
        return None
    pid, model = ref.split("/", 1)
    p = _provider(pid, s)
    if p is None or not (p.get("local") or _key(p)):
        return None
    return p, model

# tessh/doc/ai.md (a condensed guide for models), set from Rust.
PRIMER = ""
BLOCKS = {"for", "if", "elif", "else", "while", "def", "with", "try", "except", "finally", "class", "match", "case"}

def looks_tessh(line):
    """tessh's own (Python-ish) line rather than a plain command: only then
    does a suggestion get the primer (it makes a request about 2x slower)."""
    words = line.split()
    return bool(words and words[0] in (BLOCKS | {"import", "from", "return"})) or any(c in line for c in "$([{") \
        or " = " in line or " == " in line

def tessh_suggestion(reply, typed):
    """Without the bash or Python a model adds anyway (as Tessera's
    ai::tessh_suggestion): a block line loses a top-level ':' and
    do/then/done/fi/end; any line loses do/done."""
    block = (reply.split() or [""])[0] in BLOCKS
    start = min(len(typed), len(reply))
    cut = len(reply)
    for kw in (("do", "then", "done", "fi", "end") if block else ("do", "done")):
        m = re.search(r"(?: |; |;)" + kw + r"(?!\w)", reply[start:])
        if m:
            cut = min(cut, start + m.start())
    if block:
        depth, quote = 0, None
        for i, c in enumerate(reply):
            if quote:
                if c == quote:
                    quote = None
            elif c in "\"'":
                quote = c
            elif c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
            elif c == ":" and depth <= 0 and i >= start:
                cut = min(cut, i)
                break
    out = reply[:cut].rstrip().rstrip(";").rstrip()
    return out if out.startswith(typed) else typed

def _hub():
    """Hearth's AI hub, when it runs here (README "AI" > Apps in hearth)."""
    path = os.environ.get("HEARTH_AI_API_FILE") or os.path.join(os.environ.get("XDG_RUNTIME_DIR") or "/tmp", "hearth", "ai-api.json")
    try:
        with open(path) as f:
            d = json.load(f)
        if d.get("pid") and not os.path.exists("/proc/%d" % d["pid"]):
            return None
        return {"id": "hearth", "kind": "openai", "base_url": d["url"], "local": True, "token": d.get("token", "")}
    except Exception:
        return None

def ready(task):
    return _resolve(task) is not None

def system_prompt(task, cwd):
    base = ("You are built into tessh, a shell language (bash-like commands work: pipes, redirections, globs, "
            "&&, ||) that also runs Python. The user's OS is %s, current folder %s. Its own lines are tessh, "
            "not Python or bash: block lines (if, for, def, while) end with no ':' and no do/then." % (os.uname().sysname, cwd))
    return base + {
        "inline": " Complete the command the user is typing. Reply with ONLY the complete command line (including what they typed), "
                  "no explanation, no quotes, no code fence. If you have no good completion, reply with exactly what they typed.",
        "request": " The user is writing, in plain words, what they want done in their terminal (a request the AI will turn into a command). "
                   "Finish their sentence with the ending they most likely mean, from their recent commands and folder: add a few words "
                   "(what to find, where, how). Reply with ONLY the whole request: what they typed, exactly as typed, then your ending. "
                   "One line, no quotes, no explanation. Only if it's plainly finished already, reply with exactly what they typed.",
        "command": " Turn the user's request into one shell command (or a short pipeline). "
                   "Reply with ONLY the command, no explanation, no code fence.",
    }.get(task, "")

def strip_thinking(s):
    return re.sub(r"<think>.*?</think>", "", s, flags=re.S).strip()

def command_from(reply):
    r = strip_thinking(reply)
    if "```" in r:
        after = r.split("```", 1)[1]
        after = after.split("\n", 1)[1] if "\n" in after else after
        r = after.split("```", 1)[0]
    for line in r.splitlines():
        line = line.strip()
        if line:
            return line.removeprefix("$ ").strip("`")
    return ""

def ask(task, text, cwd, recent):
    got = _resolve(task)
    if got is None:
        raise RuntimeError("no AI set up: see `help ai`")
    p, model = got
    base = p.get("base_url", "").rstrip("/")
    key = _key(p)
    user = text
    head, last = text.rsplit("\n", 1) if task == "inline" and "\n" in text else ("", text)
    if head:
        # A block being typed: complete its last line, the rest as context.
        user = (("Recent commands:\n" + recent + "\n\n") if recent else "") + \
            "The block so far (the cursor is at the end of its last line):\n" + text + \
            "\n\nComplete ONLY its last line. Reply with ONLY that line, completed: " + last.strip()
    elif recent:
        if task == "inline":
            user = "Recent commands:\n" + recent + "\n\nComplete this command line: " + text
        elif task == "command":
            user = "Recent commands:\n" + recent + "\n\nWhat I want: " + text
        elif task == "request":
            user = "Recent commands:\n" + recent + "\n\nComplete this request: " + text
    system = system_prompt(task, cwd)
    primer = PRIMER if PRIMER and (task != "inline" or looks_tessh(text)) else ""
    if p.get("kind") == "anthropic":
        body = {"model": model, "max_tokens": 512, "temperature": 0.2, "system": (primer + "\n\n" + system).strip(),
                "messages": [{"role": "user", "content": user}]}
        req = urllib.request.Request(base + "/messages", data=json.dumps(body).encode(), method="POST",
                                     headers={"x-api-key": key, "anthropic-version": "2023-06-01", "content-type": "application/json"})
    else:
        body = {"model": model, "temperature": 0.2, "max_tokens": 512,
                "messages": ([{"role": "system", "content": primer}] if primer else []) +
                            [{"role": "system", "content": system}, {"role": "user", "content": user}]}
        if p.get("reasoning_effort"):
            body["reasoning_effort"] = p["reasoning_effort"]
        h = {"content-type": "application/json"}
        if key:
            h["Authorization"] = "Bearer " + key
        req = urllib.request.Request(base + "/chat/completions", data=json.dumps(body).encode(), method="POST", headers=h)
    with urllib.request.urlopen(req, timeout=60 if task == "command" else 20) as r:
        v = json.load(r)
    if p.get("kind") == "anthropic":
        out = "".join(c.get("text", "") for c in v.get("content", []))
    else:
        out = v["choices"][0]["message"].get("content") or ""
    out = strip_thinking(out)
    if task == "inline" and head:
        # (The completed line, wherever it is in the reply, at the line's own indent.)
        want = last.strip()
        line = next((l.strip() for l in strip_thinking(out).replace("`", "").splitlines() if l.strip().startswith(want)), want)
        done = tessh_suggestion(line, want)
        return head + "\n" + last[:len(last) - len(last.lstrip())] + done
    if task == "inline":
        return tessh_suggestion(command_from(out), text)
    return command_from(out) if task == "command" else out.splitlines()[0].strip() if out else ""
"#;

fn module(py: Python<'_>) -> PyResult<Bound<'_, PyModule>> {
    let sys = py.import("sys")?;
    if let Ok(m) = sys.getattr("modules")?.get_item("__tessh_ai__") {
        return Ok(m.cast_into()?);
    }
    let m = PyModule::from_code(py, &std::ffi::CString::new(CODE).unwrap(), c"<tessh ai>", c"__tessh_ai__")?;
    let doc = include_str!("../doc/ai.md");
    m.setattr("PRIMER", doc.split_once("-->").map(|(_, rest)| rest.trim()).unwrap_or(doc))?;
    sys.getattr("modules")?.set_item("__tessh_ai__", &m)?;
    Ok(m)
}

/// Whether a model is set up for this task ("inline", "request", "command").
pub fn ready(task: &str) -> bool {
    Python::attach(|py| module(py).and_then(|m| m.call_method1("ready", (task,))?.extract::<bool>()).unwrap_or(false))
}

/// The model's answer: a command ("command", "inline") or the finished
/// request ("request").
pub fn ask(task: &str, text: &str, recent: &str) -> Result<String, String> {
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    Python::attach(|py| {
        module(py)
            .and_then(|m| m.call_method1("ask", (task, text, cwd, recent))?.extract::<String>())
            .map_err(|e| e.value(py).to_string())
    })
}
