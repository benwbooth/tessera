//! The terminal REPL: rustyline with auto-indentation and highlighting, and
//! shell integration (OSC 133 prompt and command marks, OSC 7 folder,
//! Tessera's command line, environment file and hand-overs from its other
//! kernels).

use crate::compiler::BLOCK_SPECIALS;
use crate::reader::incomplete;
use crate::runner::{self, TesshError};
use crate::runtime;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Cmd, ConditionalEventHandler, Editor, Event, EventContext, EventHandler, Helper, KeyCode, KeyEvent, Modifiers, Movement, RepeatCount};
use std::borrow::Cow;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

const CONT_KEYWORDS: &[&str] = &["elif", "else", "except", "except*", "finally"];

fn osc(s: &str) {
    if unsafe { libc::isatty(1) } == 1 {
        print!("\x1b]{s}\x07");
        let _ = std::io::stdout().flush();
    }
}

fn indent_of(l: &str) -> usize {
    l.len() - l.trim_start_matches(' ').len()
}

fn head_of(l: &str) -> &str {
    let s = l.trim();
    let end = s.find(|c: char| " \t()[]{}\"'".contains(c)).unwrap_or(s.len());
    &s[..end]
}

fn openers(h: &str) -> &'static [&'static str] {
    match h {
        "if" | "elif" => &["elif", "else"],
        "for" | "while" => &["else"],
        "try" => &["except", "except*", "finally"],
        "except" | "except*" => &["except", "except*", "else", "finally"],
        "else" => &["finally"],
        _ => &[],
    }
}

/// Does this line take an indented block (def, if, …, a macro, or an
/// assignment of one: x = if …)?
pub fn opens_block(line: &str, macros: &[String]) -> bool {
    let s = line.trim();
    if s.is_empty() || s.starts_with('#') {
        return false;
    }
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut h = words[0];
    if words.len() > 2 && ["=", "+=", "-="].contains(&words[1]) {
        h = words[2];
    }
    if h == "async" && words.len() > 1 {
        h = words[1];
    }
    if (h == "def" || h == "class") && words.contains(&"=") {
        return false;
    }
    if (h == "fn" || h == "lambda") && words.len() > 2 {
        return false;
    }
    if (h == "quasi" || h == "do") && words.len() > 1 {
        return false;
    }
    if h == "if" && words.contains(&"then") {
        return false;
    }
    BLOCK_SPECIALS.contains(&h) || CONT_KEYWORDS.contains(&h) || h == "case" || macros.iter().any(|m| m == h)
}

/// A continuation keyword (else, elif, except, finally) goes back to its if
/// or try.
pub fn fix_indent(lines: &[&str], new: &str) -> String {
    let h = head_of(new);
    if !CONT_KEYWORDS.contains(&h) {
        return new.to_string();
    }
    let ind = indent_of(new);
    for prev in lines.iter().rev() {
        if prev.trim().is_empty() {
            continue;
        }
        let pi = indent_of(prev);
        if pi <= ind && openers(head_of(prev)).contains(&h) {
            return " ".repeat(pi) + new.trim_start_matches(' ');
        }
        if pi < ind.saturating_sub(0) && pi == 0 {
            break;
        }
    }
    new.to_string()
}

pub fn next_indent(lines: &[&str], macros: &[String]) -> usize {
    let last = lines.last().copied().unwrap_or("");
    if last.trim().is_empty() {
        return 0;
    }
    let ind = indent_of(last);
    if incomplete(&lines.join("\n")) {
        return if last.trim_end().ends_with(['(', '[', '{']) { ind + 2 } else { ind };
    }
    if opens_block(last, macros) {
        return ind + 2;
    }
    ind
}

/// Keep reading? (Open brackets or strings; or a block, until a blank line.)
pub fn needs_more(lines: &[&str], macros: &[String]) -> bool {
    if incomplete(&lines.join("\n")) {
        return true;
    }
    // (A block goes on to a blank line; brackets alone end when they close.)
    if !lines.iter().any(|l| opens_block(l, macros)) {
        return false;
    }
    if lines.len() == 1 {
        return true;
    }
    !lines.last().unwrap_or(&"").trim().is_empty()
}

// ---- the editor -------------------------------------------------------------------

struct EnterHandler {
    macros: Arc<Mutex<Vec<String>>>,
}

impl ConditionalEventHandler for EnterHandler {
    fn handle(&self, _evt: &Event, _n: RepeatCount, _positive: bool, ctx: &EventContext) -> Option<Cmd> {
        let buf = ctx.line();
        if ctx.pos() < buf.trim_end_matches(' ').len() {
            // (In the middle of the input: a new line there.)
            return Some(Cmd::Insert(1, "\n".into()));
        }
        let macros = self.macros.lock().unwrap().clone();
        let lines: Vec<&str> = buf.split('\n').collect();
        let n = lines.len();
        if n > 1 {
            // (An else, elif, except or finally deeper than its if or try:
            // back a level. Enter again goes on.)
            let fixed = fix_indent(&lines[..n - 1], lines[n - 1]);
            if indent_of(&fixed) < indent_of(lines[n - 1]) {
                return Some(Cmd::Dedent(Movement::WholeLine));
            }
        }
        if needs_more(&lines, &macros) {
            let ind = next_indent(&lines, &macros);
            return Some(Cmd::Insert(1, format!("\n{}", " ".repeat(ind))));
        }
        Some(Cmd::AcceptLine)
    }
}

/// Backspace in a line's leading spaces takes away a level (two spaces).
struct BackspaceHandler;

impl ConditionalEventHandler for BackspaceHandler {
    fn handle(&self, _evt: &Event, _n: RepeatCount, _positive: bool, ctx: &EventContext) -> Option<Cmd> {
        let before = &ctx.line()[..ctx.pos()];
        let line = before.rsplit('\n').next().unwrap_or("");
        if !line.is_empty() && line.chars().all(|c| c == ' ') {
            let k = if line.len() % 2 == 0 { 2 } else { 1 };
            return Some(Cmd::Kill(Movement::BackwardChar(k)));
        }
        None
    }
}

struct TesshHelper {
    ns: Py<PyDict>,
}

impl Helper for TesshHelper {}
impl Validator for TesshHelper {}
impl Hinter for TesshHelper {
    type Hint = String;
}

impl Completer for TesshHelper {
    type Candidate = Pair;

    fn complete(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        let before = &line[..pos];
        let start = before.rfind(|c: char| " \t\n()[]{}\"'`;$".contains(c)).map(|i| i + 1).unwrap_or(0);
        let word = &before[start..];
        let head = before[..start].trim_end();
        let first = head.is_empty() || head.ends_with(['|', ';', '(', '\n']) || head.ends_with("&&") || head.ends_with("||");
        let names = Python::attach(|py| completions(py, self.ns.bind(py), word, first, &before[..start]));
        let quoted = before[..start].ends_with(['\'', '"']);
        Ok((start, names.into_iter().map(|n| Pair { display: n.clone(), replacement: if quoted { n } else { quote_path(&n) } }).collect()))
    }
}

/// Completions for Tessera's input editor (`tessh --complete`): text and a
/// UTF-16 cursor in, {"start", "end" (UTF-16), "items": [{insert, label,
/// kind, detail}]} out, as Tessera's shell completer answers.
/// A value's kind, said plainly.
fn type_word(v: &Bound<'_, PyAny>) -> String {
    let t = v.get_type().name().map(|t| t.to_string()).unwrap_or_default();
    match t.as_str() {
        "builtin_function_or_method" | "function" | "method" | "method-wrapper" | "wrapper_descriptor" | "method_descriptor" | "builtin_method" => "function".into(),
        "type" | "ABCMeta" | "EnumType" => "class".into(),
        "Builtin" => "tessh builtin".into(),
        _ => t,
    }
}

/// The completion panel's help for a name: its signature and docstring (a
/// Python value), or the guide's topic for it (a tessh builtin, a keyword).
fn doc_for(py: Python<'_>, name: &str, v: Option<&Bound<'_, PyAny>>) -> String {
    const MAX: usize = 20_000;
    let mut out = String::new();
    let tessh_own = v.is_none_or(|v| v.get_type().name().is_ok_and(|t| t.to_string() == "Builtin"));
    if let (Some(v), false) = (v, tessh_own) {
        if let Ok(inspect) = py.import("inspect") {
            let callable = v.is_callable();
            if callable {
                if let Ok(sig) = inspect.call_method1("signature", (v,)).and_then(|s| s.str()) {
                    out = format!("{name}{sig}");
                }
            }
            // (A plain value's docstring is its type's: not much use.)
            let own = callable || v.is_instance_of::<pyo3::types::PyModule>() || v.is_instance_of::<pyo3::types::PyType>();
            if own {
                if let Ok(Some(d)) = inspect.call_method1("getdoc", (v,)).and_then(|d| d.extract::<Option<String>>()) {
                    // (print's doc starts with its signature again.)
                    let d = d.trim();
                    let d = match d.split_once("\n\n") {
                        Some((first, rest)) if !out.is_empty() && first.starts_with(&format!("{name}(")) => rest.trim(),
                        _ => d,
                    };
                    if !out.is_empty() {
                        out.push_str("\n\n");
                    }
                    out.push_str(d);
                }
            }
        }
    }
    // (A plain value: what it is.)
    if let (Some(v), true, false) = (v, out.is_empty(), tessh_own) {
        if !v.is_callable() {
            if let Ok(r) = v.repr().map(|r| r.to_string()) {
                let r: String = r.chars().take(2000).collect();
                out = format!("{name} = {r}");
            }
        }
    }
    // (A tessh builtin: its usage, then the guide's part about it.)
    if let (Some(v), true) = (v, tessh_own) {
        if let Ok(d) = v.getattr("_tessh_usage").and_then(|d| d.extract::<String>()) {
            out = d;
        }
    }
    let key = name.to_lowercase();
    if out.is_empty() || tessh_own {
        if let Some(t) = crate::help::topics().into_iter().find(|t| t.keys.iter().any(|k| *k == key)) {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&t.body);
        }
    }
    if out.len() > MAX {
        let mut cut = MAX;
        while !out.is_char_boundary(cut) {
            cut -= 1;
        }
        out.truncate(cut);
        out.push('…');
    }
    out
}

pub fn complete_json(py: Python<'_>, ns: &Bound<'_, PyDict>, text: &str, cursor16: usize) -> serde_json::Value {
    let mut pos = text.len();
    let mut n16 = 0;
    for (i, c) in text.char_indices() {
        if n16 >= cursor16 {
            pos = i;
            break;
        }
        n16 += c.len_utf16();
    }
    let before = &text[..pos];
    let start = before.rfind(|c: char| " \t\n()[]{}\"'`;$".contains(c)).map(|i| i + 1).unwrap_or(0);
    let word = &before[start..];
    let head = before[..start].trim_end();
    let first = head.is_empty() || head.ends_with(['|', ';', '(', '\n']) || head.ends_with("&&") || head.ends_with("||");
    let var = before[..start].ends_with('$');
    let u16_of = |b: usize| text[..b].encode_utf16().count();
    let mut items: Vec<serde_json::Value> = vec![];
    // (obj.attr: what the object has, from a name tessh knows or a module
    // already loaded.)
    if let Some((root, part)) = word.rsplit_once('.').filter(|(r, _)| !r.is_empty() && r.split('.').all(crate::reader::is_ident)) {
        let mut parts = root.split('.');
        let head_name = parts.next().unwrap_or("");
        let mut obj = ns.get_item(head_name).ok().flatten().or_else(|| py.import("builtins").ok().and_then(|b| b.getattr(head_name).ok())).or_else(|| py.import("sys").ok().and_then(|s| s.getattr("modules").ok()).and_then(|m| m.get_item(head_name).ok()));
        for p in parts {
            obj = obj.and_then(|o| o.getattr(p).ok());
        }
        if let Some(o) = obj {
            if let Ok(names) = o.dir() {
                let mut ns_: Vec<String> = names.iter().filter_map(|k| k.extract::<String>().ok()).filter(|n| n.starts_with(part) && (part.starts_with('_') || !n.starts_with('_'))).collect();
                ns_.sort();
                for n in ns_ {
                    let v = o.getattr(n.as_str()).ok();
                    let callable = v.as_ref().is_some_and(|v| v.is_callable());
                    let detail = v.as_ref().map(type_word).unwrap_or_default();
                    let doc = v.as_ref().map(|v| doc_for(py, &n, Some(v))).unwrap_or_default();
                    items.push(serde_json::json!({"insert": format!("{root}.{n}"), "label": n, "kind": if callable { "command" } else { "value" }, "detail": detail, "doc": doc}));
                }
            }
        }
        return serde_json::json!({"start": u16_of(start), "end": u16_of(pos), "items": items});
    }
    let builtins = py.import("builtins").ok();
    for n in completions(py, ns, word, first, &before[..start]) {
        let mut doc = String::new();
        let (kind, detail) = if before[..start].trim() == "help" {
            doc = crate::help::topics().into_iter().find(|t| t.keys.iter().any(|k| *k == n.to_lowercase())).map(|t| t.body).unwrap_or_default();
            ("option", "help topic".to_string())
        } else if n.ends_with('/') {
            ("dir", String::new())
        } else if !first && !var {
            ("file", String::new())
        } else if var {
            ("variable", ns.get_item(n.as_str()).ok().flatten().map(|v| type_word(&v)).unwrap_or_else(|| "environment".into()))
        } else if crate::compiler::BLOCK_SPECIALS.contains(&n.as_str()) || ["return", "import", "from"].contains(&n.as_str()) {
            doc = doc_for(py, &n, None);
            ("subcommand", "keyword".to_string())
        } else if let Some(v) = ns.get_item(n.as_str()).ok().flatten().or_else(|| builtins.as_ref().and_then(|b| b.getattr(n.as_str()).ok())) {
            doc = doc_for(py, &n, Some(&v));
            (if v.is_callable() { "command" } else { "value" }, type_word(&v))
        } else {
            ("command", "program".to_string())
        };
        // (A name that needs quoting comes quoted, so it works as inserted:
        // '7th Heaven'/ — unless it's inside a quote already.)
        let insert = if (kind == "dir" || kind == "file") && !before[..start].ends_with(['\'', '"']) { quote_path(&n) } else { n.clone() };
        // (Shown as it'll go in, quotes and all.)
        let label = if kind == "dir" || kind == "file" { insert.clone() } else { n.clone() };
        items.push(serde_json::json!({"insert": insert, "label": label, "kind": kind, "detail": detail, "doc": doc}));
    }
    serde_json::json!({"start": u16_of(start), "end": u16_of(pos), "items": items})
}

pub fn completions(py: Python<'_>, ns: &Bound<'_, PyDict>, word: &str, first: bool, before: &str) -> Vec<String> {
    let mut out = vec![];
    // (help's topics.)
    if before.trim() == "help" {
        let mut ks: Vec<String> = crate::help::topics().into_iter().flat_map(|t| t.keys).filter(|k| k.starts_with(word)).collect();
        ks.sort();
        ks.dedup();
        return ks;
    }
    let var = before.ends_with('$');
    if word.contains('/') || word.starts_with(['~', '.']) || (!first && !var) {
        // (What's typed is matched as written: a * in it isn't a pattern
        // here (`*` + `*` would be `**`, every file under every folder),
        // and only so many names are read.)
        let pat = format!("{}*", glob::Pattern::escape(&runtime::expanduser(word)));
        if let Ok(it) = glob::glob(&pat) {
            let home = std::env::var("HOME").unwrap_or_default();
            for p in it.flatten().take(2000) {
                let mut s = p.to_string_lossy().to_string();
                if word.starts_with('~') && s.starts_with(&home) {
                    s = format!("~{}", &s[home.len()..]);
                }
                if p.is_dir() {
                    s.push('/');
                }
                out.push(s);
            }
        }
    }
    if first || var {
        let mut names: Vec<String> = ns.keys().iter().filter_map(|k| k.extract::<String>().ok()).collect();
        if let Ok(b) = py.import("builtins").and_then(|b| b.dir()) {
            names.extend(b.iter().filter_map(|k| k.extract::<String>().ok()));
        }
        if !var {
            names.extend(BLOCK_SPECIALS.iter().map(|s| s.to_string()));
            names.extend(["return", "import", "from", "export", "cd"].map(String::from));
        }
        if var {
            names.extend(std::env::vars().map(|(k, _)| k));
        }
        let mut names: Vec<String> = names.into_iter().filter(|n| n.starts_with(word) && !n.starts_with("__")).collect();
        names.sort();
        names.dedup();
        out.extend(names);
        if first && !var {
            let mut cmds = std::collections::BTreeSet::new();
            if let Some(path) = std::env::var_os("PATH") {
                for d in std::env::split_paths(&path) {
                    if let Ok(rd) = std::fs::read_dir(d) {
                        for e in rd.flatten() {
                            let n = e.file_name().to_string_lossy().to_string();
                            if n.starts_with(word) {
                                cmds.insert(n);
                            }
                        }
                    }
                }
            }
            out.extend(cmds);
        }
    }
    let mut seen = std::collections::HashSet::new();
    out.retain(|x| seen.insert(x.clone()));
    out
}

impl Highlighter for TesshHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Owned(highlight(line))
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(&'s self, prompt: &'p str, _default: bool) -> Cow<'b, str> {
        Cow::Borrowed(prompt)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        true
    }
}

/// Colors for a line as it's typed: strings, $variables, numbers, comments,
/// keywords, operators and the head of each line.
pub fn highlight(line: &str) -> String {
    let mut out = String::with_capacity(line.len() * 2);
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut at_head = true;
    let paint = |out: &mut String, color: &str, s: &str| {
        out.push_str(color);
        out.push_str(s);
        out.push_str("\x1b[0m");
    };
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            out.push(c);
            at_head = true;
            i += 1;
            continue;
        }
        if c == ' ' || c == '\t' {
            out.push(c);
            i += 1;
            continue;
        }
        if c == '#' && (i == 0 || " \t\n([{".contains(chars[i - 1])) {
            let j = chars[i..].iter().position(|&x| x == '\n').map(|k| i + k).unwrap_or(chars.len());
            paint(&mut out, "\x1b[2;3m", &chars[i..j].iter().collect::<String>());
            i = j;
            continue;
        }
        if c == '"' || c == '\'' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != c {
                if chars[j] == '\\' && c == '"' {
                    j += 1;
                }
                j += 1;
            }
            let j = (j + 1).min(chars.len());
            paint(&mut out, "\x1b[32m", &chars[i..j].iter().collect::<String>());
            i = j;
            at_head = false;
            continue;
        }
        if "()[]{};,".contains(c) {
            out.push(c);
            if c == '(' || c == ';' {
                at_head = true;
            }
            i += 1;
            continue;
        }
        // A word.
        let mut j = i;
        while j < chars.len() && !" \t\n()[]{};\"'".contains(chars[j]) {
            j += 1;
        }
        let w: String = chars[i..j].iter().collect();
        let color = if w.starts_with('$') {
            "\x1b[36m"
        } else if crate::reader::is_number(&w) {
            "\x1b[35m"
        } else if crate::compiler::is_special(&w) || ["and", "or", "not", "in", "is", "then", "as", "True", "False", "None"].contains(&w.as_str()) {
            "\x1b[1;35m"
        } else if w.chars().all(|c| crate::reader::OPCHARS.contains(c)) {
            "\x1b[33m"
        } else if at_head {
            "\x1b[1;34m"
        } else if w.starts_with('-') {
            "\x1b[33m"
        } else {
            ""
        };
        if color.is_empty() {
            out.push_str(&w);
        } else {
            paint(&mut out, color, &w);
        }
        at_head = w.chars().all(|c| "|&;".contains(c)) && !w.is_empty() && w != "&";
        i = j;
    }
    out
}

struct Prompt {
    raw: String,
    styled: String,
}

impl rustyline::Prompt for Prompt {
    fn raw(&self) -> &str {
        &self.raw
    }
    fn styled(&self) -> &str {
        &self.styled
    }
}

fn env_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".into())).join("tessera-env")
}

/// What Tessera's other kernels handed this shell (folder, exports).
fn apply_handover(py: Python<'_>) {
    let d = env_dir();
    let pid = std::process::id().to_string();
    let f = d.join(format!("{pid}.apply"));
    if let Ok(t) = std::fs::read_to_string(&f) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
            if let Err(e) = runtime::apply_json(py, &v) {
                e.print(py);
            }
        }
        let _ = std::fs::remove_file(&f);
        let _ = std::fs::remove_file(d.join(format!("{pid}.cwd")));
    }
}

/// The exported environment, for Tessera's kernels and prompt.
fn write_env() {
    let d = env_dir();
    if std::fs::create_dir_all(&d).is_err() {
        return;
    }
    let mut out = Vec::new();
    for (k, v) in std::env::vars_os() {
        out.extend(k.as_encoded_bytes());
        out.push(b'=');
        out.extend(v.as_encoded_bytes());
        out.push(0);
    }
    let _ = std::fs::write(d.join(std::process::id().to_string()), out);
}

fn hostname() -> String {
    let mut buf = [0u8; 256];
    unsafe { libc::gethostname(buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
    String::from_utf8_lossy(&buf[..buf.iter().position(|b| *b == 0).unwrap_or(0)]).to_string()
}

pub fn run() -> i32 {
    unsafe {
        libc::prctl(libc::PR_SET_NAME, c"tessh".as_ptr() as libc::c_ulong, 0, 0, 0);
        for s in [libc::SIGTSTP, libc::SIGTTOU, libc::SIGTTIN] {
            libc::signal(s, libc::SIG_IGN);
        }
    }
    runtime::INTERACTIVE.store(true, Ordering::SeqCst);
    let ns: Py<PyDict> = match Python::attach(|py| runner::new_namespace(py, "__main__", vec![]).map(|n| n.unbind())) {
        Ok(ns) => ns,
        Err(e) => {
            eprintln!("tessh: {e}");
            return 1;
        }
    };
    let macros = Arc::new(Mutex::new(Vec::<String>::new()));
    let config = rustyline::Config::builder().indent_size(2).auto_add_history(false).history_ignore_space(true).max_history_size(10000).map(|b| b.build()).unwrap_or_default();
    let mut rl: Editor<TesshHelper, rustyline::history::DefaultHistory> = match Editor::with_config(config) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("tessh: {e}");
            return 1;
        }
    };
    rl.set_helper(Some(TesshHelper { ns: Python::attach(|py| ns.clone_ref(py)) }));
    rl.bind_sequence(KeyEvent(KeyCode::Enter, Modifiers::NONE), EventHandler::Conditional(Box::new(EnterHandler { macros: macros.clone() })));
    rl.bind_sequence(KeyEvent(KeyCode::Backspace, Modifiers::NONE), EventHandler::Conditional(Box::new(BackspaceHandler)));
    // (Alt+Enter: a new line whatever the input is.)
    rl.bind_sequence(KeyEvent(KeyCode::Enter, Modifiers::ALT), EventHandler::Simple(Cmd::Newline));
    let data = std::env::var("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share")).join("tessh");
    let _ = std::fs::create_dir_all(&data);
    let hist = data.join("history");
    let _ = rl.load_history(&hist);
    let config_dir = std::env::var("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"));
    let rc = config_dir.join("tessh/rc.tsh");
    if rc.is_file() {
        Python::attach(|py| {
            if let Err(e) = runner::run_file(py, &rc.to_string_lossy(), ns.bind(py)) {
                runner::report(py, e, &rc.to_string_lossy());
            }
        });
    }
    let host = hostname();
    let mut code = 0;
    loop {
        Python::attach(|py| {
            apply_handover(py);
            *macros.lock().unwrap() = runner::macro_names(ns.bind(py));
        });
        write_env();
        let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        osc(&format!("7;file://{host}{cwd}"));
        let status = runtime::STATUS.load(Ordering::SeqCst);
        let mark = if status == 0 { "\x1b[32m❯\x1b[0m" } else { "\x1b[31m❯\x1b[0m" };
        let tty = unsafe { libc::isatty(1) } == 1;
        let prompt = Prompt {
            raw: format!("{cwd} ❯ "),
            styled: if tty { format!("\x1b]133;A\x07\x1b[1;34m{cwd}\x1b[0m {mark} \x1b]133;B\x07") } else { format!("{cwd} > ") },
        };
        let text = match rl.readline(&prompt) {
            Ok(t) => t,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => {
                if std::env::var_os("TESSH_DEBUG").is_some() {
                    eprintln!("tessh: end of input");
                }
                break;
            }
            Err(e) => {
                eprintln!("tessh: {e}");
                code = 1;
                break;
            }
        };
        let text = text.trim_end().to_string();
        if text.trim().is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(text.as_str());
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;
        // (What it runs with: user, line, environment, NUL-separated.)
        let exec = format!("{}\0{}\0{}", crate::runtime::user_name(), text, crate::runtime::env_list().join("\0"));
        osc(&format!("1337;TesseraExec={}", b64.encode(&exec)));
        osc(&format!("1337;TesseraCommand={}", b64.encode(&text)));
        osc("133;C");
        let exit = Python::attach(|py| -> Option<i32> {
            let r = runner::run_source(py, &text, ns.bind(py), "<stdin>", true);
            let flush = || {
                if let Ok(sys) = py.import("sys") {
                    let _ = sys.getattr("stdout").and_then(|o| o.call_method0("flush"));
                    let _ = sys.getattr("stderr").and_then(|o| o.call_method0("flush"));
                }
            };
            match r {
                Ok(()) => {}
                Err(TesshError::Py(e)) if e.is_instance_of::<pyo3::exceptions::PySystemExit>(py) => {
                    flush();
                    let c = e.value(py).getattr("code").ok().map(|c| if c.is_none() { 0 } else { c.extract::<i32>().unwrap_or(1) }).unwrap_or(0);
                    return Some(c);
                }
                Err(TesshError::Py(e)) if e.is_instance_of::<pyo3::exceptions::PyKeyboardInterrupt>(py) => {
                    eprintln!("^C");
                    runtime::STATUS.store(130, Ordering::SeqCst);
                }
                Err(e) => {
                    runner::report(py, e, "<stdin>");
                    runtime::STATUS.store(1, Ordering::SeqCst);
                }
            }
            flush();
            None
        });
        if let Some(c) = exit {
            osc(&format!("133;D;{c}"));
            code = c;
            break;
        }
        osc(&format!("133;D;{}", runtime::STATUS.load(Ordering::SeqCst)));
    }
    let _ = rl.save_history(&hist);
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_and_indentation() {
        let m: Vec<String> = vec!["unless".into()];
        assert!(opens_block("def f x", &m));
        assert!(opens_block("x = if $a", &m));
        assert!(opens_block("unless $x", &m));
        assert!(!opens_block("def sq x = $x * $x", &m));
        assert!(!opens_block("ls -la", &m));
        assert!(!opens_block("y = if $a then 1 else 2", &m));
        assert!(needs_more(&["def f x"], &m));
        assert!(needs_more(&["def f x", "  print hi"], &m));
        assert!(!needs_more(&["def f x", "  print hi", ""], &m));
        assert!(!needs_more(&["ls"], &m));
        assert!(needs_more(&["x = (1"], &m));
        assert_eq!(next_indent(&["def f x"], &m), 2);
        assert_eq!(next_indent(&["def f x", "  if $x"], &m), 4);
        assert_eq!(next_indent(&["def f x", "  print hi"], &m), 2);
        assert_eq!(fix_indent(&["if $x", "  a"], "  else"), "else");
        assert_eq!(fix_indent(&["def f", "  try", "    a"], "    except"), "  except");
        assert_eq!(fix_indent(&["if $x", "  a"], "  b"), "  b");
    }

    #[test]
    fn highlights() {
        let h = highlight("ls -la $x \"s\" # c");
        assert!(h.contains("\x1b[1;34mls"), "{h:?}");
        assert!(h.contains("\x1b[36m$x"), "{h:?}");
        assert!(h.contains("\x1b[32m\"s\""), "{h:?}");
    }
}

/// A path as tessh reads it back: each part that has a space or another
/// special character in it quoted ('7th Heaven'/, ~/'My Docs'/x), the rest
/// (and a leading ~) bare, so the path can go on after it.
pub fn quote_path(p: &str) -> String {
    let safe = |part: &str| part.chars().all(|c| c.is_alphanumeric() || "_-.+@%=,:~".contains(c));
    p.split('/')
        .map(|part| {
            if part.is_empty() || safe(part) {
                part.to_string()
            } else if !part.contains('\'') {
                format!("'{part}'")
            } else {
                format!("\"{}\"", part.replace('\\', "\\\\").replace('"', "\\\"").replace('$', "\\$"))
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod quote_tests {
    #[test]
    fn quotes_what_needs_it() {
        assert_eq!(super::quote_path("7th Heaven/"), "'7th Heaven'/");
        assert_eq!(super::quote_path("~/My Docs/a.txt"), "~/'My Docs'/a.txt");
        assert_eq!(super::quote_path("src/main.rs"), "src/main.rs");
        assert_eq!(super::quote_path("it's here"), "\"it's here\"");
        assert_eq!(super::quote_path("a$b"), "'a$b'");
    }
}
