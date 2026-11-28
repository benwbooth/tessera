//! `tessh --lsp`: a language server for editors (Turbine, VS Code, Helix),
//! the Language Server Protocol on stdin and stdout. It gives
//!
//! - diagnostics on open and change: what `tessh --check` finds;
//! - completion: keywords, tessh's and Python's builtins, the file's own
//!   functions, classes and variables (a function's parameters inside it),
//!   commands on PATH as a line's first word, and a program's arguments
//!   (options, subcommands, values) and paths, as the REPL completes them;
//! - hover: the guide's topic for a keyword, a builtin's usage or docs, the
//!   file's own functions' `def` lines and the comments above them;
//! - document symbols (functions, macros, operators, classes, variables,
//!   nested as the file nests them) and go to definition within the file.
//!
//! Positions are UTF-16 (LSP's default). The file's own definitions are
//! read from its lines (`def`, `async def`, `defmacro`, `defop`, `class`,
//! a top-level `name = …`), so they're there while the file has errors.

use crate::{compiler, help, runner, runtime};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};

pub fn run(py: Python<'_>) -> i32 {
    // The protocol keeps the real stdout; anything else printing (Python's
    // warnings, a builtin's import) goes to stderr instead of into it.
    let out_fd = unsafe {
        let fd = libc::dup(1);
        libc::dup2(2, 1);
        if fd >= 0 {
            libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
        }
        fd
    };
    if out_fd < 0 {
        eprintln!("tessh --lsp: can't keep its stdout");
        return 1;
    }
    let out: std::fs::File = unsafe { std::os::fd::FromRawFd::from_raw_fd(out_fd) };
    let mut server = match Server::new(py, out) {
        Ok(s) => s,
        Err(e) => {
            e.print(py);
            return 1;
        }
    };
    let mut input = BufReader::new(std::io::stdin().lock());
    while let Some(msg) = read_message(&mut input) {
        if let Some(code) = server.handle(py, msg) {
            return code;
        }
    }
    // (stdin closed without an exit: as exit without shutdown.)
    1
}

/// One message (Content-Length framed), or None at the end of input.
fn read_message(r: &mut impl BufRead) -> Option<Value> {
    loop {
        let mut len = None;
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).ok()? == 0 {
                return None;
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                if k.eq_ignore_ascii_case("content-length") {
                    len = v.trim().parse::<usize>().ok();
                }
            }
        }
        let Some(len) = len else { continue };
        let mut body = vec![0; len];
        r.read_exact(&mut body).ok()?;
        if let Ok(v) = serde_json::from_slice(&body) {
            return Some(v);
        }
    }
}

struct Server {
    out: std::fs::File,
    docs: HashMap<String, String>,
    ns: Py<PyDict>,
    /// The commands on PATH (read once).
    commands: Option<Vec<String>>,
    shut_down: bool,
}

impl Server {
    fn new(py: Python<'_>, out: std::fs::File) -> PyResult<Server> {
        runner::init(py)?;
        let ns = runner::new_namespace(py, "__main__", vec![])?.unbind();
        Ok(Server { out, docs: HashMap::new(), ns, commands: None, shut_down: false })
    }

    fn send(&mut self, v: Value) {
        let body = v.to_string();
        let _ = write!(self.out, "Content-Length: {}\r\n\r\n{body}", body.len());
        let _ = self.out.flush();
    }

    fn reply(&mut self, id: &Value, result: Value) {
        self.send(json!({"jsonrpc": "2.0", "id": id, "result": result}));
    }

    /// A message handled; Some(code) when the server should exit.
    fn handle(&mut self, py: Python<'_>, msg: Value) -> Option<i32> {
        let method = msg["method"].as_str().unwrap_or("").to_string();
        let id = msg.get("id").cloned();
        let p = &msg["params"];
        // (A reply to something we asked: we ask nothing.)
        if method.is_empty() {
            return None;
        }
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("").to_string();
        match method.as_str() {
            "initialize" => {
                let caps = json!({
                    "capabilities": {
                        "positionEncoding": "utf-16",
                        "textDocumentSync": {"openClose": true, "change": 1, "save": {"includeText": false}},
                        "completionProvider": {"triggerCharacters": ["$"]},
                        "hoverProvider": true,
                        "documentSymbolProvider": true,
                        "definitionProvider": true,
                    },
                    "serverInfo": {"name": "tessh", "version": env!("CARGO_PKG_VERSION")},
                });
                self.reply(id.as_ref()?, caps);
            }
            "initialized" | "$/cancelRequest" | "$/setTrace" | "workspace/didChangeConfiguration" | "textDocument/didSave" => {}
            "shutdown" => {
                self.shut_down = true;
                self.reply(id.as_ref()?, Value::Null);
            }
            "exit" => return Some(if self.shut_down { 0 } else { 1 }),
            "textDocument/didOpen" => {
                let text = p["textDocument"]["text"].as_str().unwrap_or("").to_string();
                self.docs.insert(uri.clone(), text);
                self.publish(py, &uri);
            }
            "textDocument/didChange" => {
                // (Full sync: the last change is the whole text.)
                if let Some(text) = p["contentChanges"].as_array().and_then(|a| a.last()).and_then(|c| c["text"].as_str()) {
                    self.docs.insert(uri.clone(), text.to_string());
                    self.publish(py, &uri);
                }
            }
            "textDocument/didClose" => {
                self.docs.remove(&uri);
                self.send(json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {"uri": uri, "diagnostics": []}}));
            }
            "textDocument/completion" | "textDocument/hover" | "textDocument/documentSymbol" | "textDocument/definition" => {
                let id = id?;
                let text = self.docs.get(&uri).cloned().unwrap_or_default();
                let (line, ch) = (p["position"]["line"].as_u64().unwrap_or(0) as usize, p["position"]["character"].as_u64().unwrap_or(0) as u32);
                let r = match method.as_str() {
                    "textDocument/completion" => self.completion(py, &uri, &text, line, ch),
                    "textDocument/hover" => self.hover(py, &text, line, ch),
                    "textDocument/documentSymbol" => document_symbols(&text),
                    _ => definition(&uri, &text, line, ch),
                };
                self.reply(&id, r);
            }
            _ => {
                if let Some(id) = id {
                    self.send(json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("tessh --lsp: no {method}")}}));
                }
            }
        }
        None
    }

    fn publish(&mut self, py: Python<'_>, uri: &str) {
        let text = self.docs.get(uri).cloned().unwrap_or_default();
        let name = uri.strip_prefix("file://").map(percent_decode).unwrap_or_else(|| uri.to_string());
        let diags = diagnostics(py, &text, &name);
        self.send(json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": {"uri": uri, "diagnostics": diags}}));
    }

    fn completion(&mut self, py: Python<'_>, uri: &str, text: &str, line: usize, ch: u32) -> Value {
        let l = text.lines().nth(line).unwrap_or("");
        let at = byte_of_utf16(l, ch);
        let before = &l[..at];
        let start = before.rfind(|c: char| !is_word_char(c)).map(|i| i + before[i..].chars().next().unwrap().len_utf8()).unwrap_or(0);
        let word = &before[start..];
        let var = before[..start].ends_with('$');
        let lead = before[..start].trim_end();
        let first = !var && (lead.is_empty() || lead.ends_with(['(', '|', ';', '{']) || lead.ends_with("&&") || lead.ends_with("||"));
        // (A program's arguments, or a path anywhere: the REPL's completer.)
        if !var && (!first || word.starts_with(['/', '~', '.']) || word.contains('/')) {
            if let Some(r) = self.argument_completion(py, uri, text, l, line, at) {
                return r;
            }
        }
        let mut items: Vec<Value> = vec![];
        let mut seen = std::collections::HashSet::new();
        let mut add = |items: &mut Vec<Value>, label: &str, kind: u32, detail: &str| {
            if label.starts_with(word) && !label.starts_with("__") && seen.insert(label.to_string()) {
                items.push(json!({"label": label, "kind": kind, "detail": detail}));
            }
        };
        let defs = definitions(text);
        // (The file's own names first: its functions, classes, variables,
        // and inside a function, its parameters.)
        for d in &defs {
            if d.indent == 0 || !var {
                let kind = match d.kind {
                    SymKind::Class => 7,
                    SymKind::Variable => 6,
                    _ => 3,
                };
                if var && kind != 6 && d.kind != SymKind::Function {
                    continue;
                }
                add(&mut items, &d.name, kind, &d.signature);
            }
        }
        for d in defs.iter().filter(|d| d.kind == SymKind::Function && d.line < line && line <= d.end_line) {
            for prm in &d.params {
                add(&mut items, prm, 6, &format!("parameter of {}", d.name));
            }
        }
        if !var {
            for k in keywords() {
                add(&mut items, k, 14, "keyword");
            }
        }
        // (tessh's builtins, then Python's.)
        let ns = self.ns.bind(py);
        for k in ns.keys().iter().filter_map(|k| k.extract::<String>().ok()) {
            let detail = if runtime::is_command_builtin(&k) { "tessh builtin command" } else { "tessh builtin" };
            add(&mut items, &k, 3, detail);
        }
        if let Ok(b) = py.import("builtins").and_then(|b| b.dir()) {
            for k in b.iter().filter_map(|k| k.extract::<String>().ok()) {
                add(&mut items, &k, 3, "Python builtin");
            }
        }
        if var {
            for (k, _) in std::env::vars() {
                add(&mut items, &k, 6, "environment variable");
            }
        }
        // (Commands as a line's first word, once something's typed: PATH
        // has thousands.)
        let mut incomplete = false;
        if first && !word.is_empty() {
            incomplete = true;
            let cmds = self.commands.get_or_insert_with(path_commands);
            for c in cmds.iter() {
                add(&mut items, c, 3, "command");
            }
        }
        json!({"isIncomplete": incomplete, "items": items})
    }

    /// A program's arguments (its options, subcommands and values, from
    /// carapace, bash-completion, `complete` or its man page, and files) or
    /// a path being typed, as the REPL completes them, with paths from the
    /// file's folder. None where the file's own names fit better: a tessh
    /// function's or builtin's arguments that aren't paths.
    fn argument_completion(&mut self, py: Python<'_>, uri: &str, text: &str, l: &str, line: usize, at: usize) -> Option<Value> {
        let before = &l[..at];
        let (wstart, word) = crate::repl::comp_word(before);
        let words = crate::carapace::line_words(&before[..wstart]);
        let ns = self.ns.bind(py);
        let pathlike = word.starts_with(['/', '~', '.']) || word.contains('/');
        // (A path as a line's first word: a script to run, a value.)
        let none = String::new();
        let cmd = words.first().unwrap_or(&none);
        let ours = cmd.is_empty() || keywords().any(|k| k == cmd) || (ns.contains(cmd.as_str()).unwrap_or(false) && !runtime::is_command_builtin(cmd)) || definitions(text).iter().any(|d| &d.name == cmd) || cmd.contains('=') || cmd.starts_with(['$', '[', '{', '"', '\'']);
        let program = !ours && (cmd.contains('/') || runtime::is_command_builtin(cmd) || self.commands.get_or_insert_with(path_commands).binary_search(cmd).is_ok());
        if !program && !pathlike {
            return None;
        }
        // (Paths from the file's folder, as it's usually run from there.)
        let here = std::env::current_dir().ok();
        if let Some(dir) = uri.strip_prefix("file://").map(percent_decode).and_then(|p| std::path::Path::new(&p).parent().map(|d| d.to_path_buf())) {
            let _ = std::env::set_current_dir(&dir);
        }
        let (s, e, items) = crate::repl::complete_items(py, ns, l, at);
        if let Some(h) = here {
            let _ = std::env::set_current_dir(h);
        }
        let items: Vec<_> = items.into_iter().filter(|i| program || matches!(i.kind, "dir" | "file")).collect();
        if items.is_empty() && !program {
            return None;
        }
        let range = json!({"start": {"line": line, "character": utf16_col(l, s)}, "end": {"line": line, "character": utf16_col(l, e)}});
        let items: Vec<Value> = items
            .into_iter()
            .enumerate()
            .map(|(n, i)| {
                let kind = match i.kind {
                    "dir" => 19,
                    "file" => 17,
                    "option" | "partial" => 5,
                    "subcommand" => 2,
                    "variable" => 6,
                    "value" => 12,
                    _ => 3,
                };
                let mut v = json!({"label": i.label, "kind": kind, "detail": i.detail, "filterText": i.insert, "sortText": format!("{n:05}"), "textEdit": {"range": range, "newText": i.insert}});
                if !i.doc.is_empty() {
                    v["documentation"] = json!({"kind": "plaintext", "value": i.doc});
                }
                v
            })
            .collect();
        Some(json!({"isIncomplete": true, "items": items}))
    }

    fn hover(&mut self, py: Python<'_>, text: &str, line: usize, ch: u32) -> Value {
        let l = text.lines().nth(line).unwrap_or("");
        let Some((s, e)) = word_at(l, byte_of_utf16(l, ch)) else { return Value::Null };
        let w = &l[s..e];
        let md = self.hover_markdown(py, text, w, line);
        match md {
            Some(md) => json!({
                "contents": {"kind": "markdown", "value": md},
                "range": {"start": {"line": line, "character": utf16_col(l, s)}, "end": {"line": line, "character": utf16_col(l, e)}},
            }),
            None => Value::Null,
        }
    }

    fn hover_markdown(&self, py: Python<'_>, text: &str, w: &str, line: usize) -> Option<String> {
        // (The file's own definition: its line, and the comments over it.)
        if let Some(d) = find_definition(text, w, line) {
            let mut md = format!("```tessh\n{}\n```", d.signature);
            if !d.doc.is_empty() {
                md.push_str("\n\n");
                md.push_str(&d.doc);
            }
            return Some(md);
        }
        // (A keyword or anything the guide has a topic for, by its key.)
        let lw = w.to_lowercase();
        let is_kw = keywords().any(|k| k == w);
        if is_kw || (!self.ns.bind(py).contains(w).unwrap_or(false) && help::topics().iter().any(|t| t.keys.contains(&lw))) {
            if let Some(md) = help::lookup(w) {
                return Some(md);
            }
        }
        // (A builtin: tessh's usage, else Python's signature and docs.)
        let ns = self.ns.bind(py);
        let obj = ns.get_item(w).ok().flatten().or_else(|| py.import("builtins").ok()?.getattr(w).ok())?;
        if let Ok(u) = obj.getattr("_tessh_usage").and_then(|u| u.extract::<String>()) {
            let (head, rest) = u.split_once("\n\n").unwrap_or((&u, ""));
            return Some(format!("```tessh\n{head}\n```\n\n{rest}").trim_end().to_string());
        }
        let inspect = py.import("inspect").ok()?;
        let sig = inspect.call_method1("signature", (&obj,)).ok().map(|s| s.to_string()).unwrap_or_default();
        let doc: String = inspect.call_method1("getdoc", (&obj,)).ok().and_then(|d| d.extract().ok()).unwrap_or_default();
        if sig.is_empty() && doc.is_empty() {
            return None;
        }
        let mut md = format!("```python\n{w}{sig}\n```");
        if !doc.is_empty() {
            md.push_str("\n\n");
            md.push_str(&doc);
        }
        Some(md)
    }
}

/// What `tessh --check` finds, as LSP diagnostics.
pub fn diagnostics(py: Python<'_>, text: &str, name: &str) -> Vec<Value> {
    let lines: Vec<&str> = text.lines().collect();
    runner::check(py, text, name)
        .into_iter()
        .map(|(l, c, e, m)| {
            let line = (l.max(1) - 1) as usize;
            let src = lines.get(line).copied().unwrap_or("");
            let col16 = |chars: u32| -> u32 { src.chars().take(chars as usize).map(char::len_utf16).sum::<usize>() as u32 };
            let (sc, ec) = (col16(c), col16(e.max(c + 1)));
            json!({
                "range": {"start": {"line": line, "character": sc}, "end": {"line": line, "character": ec.max(sc + 1)}},
                "severity": 1,
                "source": "tessh",
                "message": m,
            })
        })
        .collect()
}

/// tessh's keywords (never words: `and`, `in`… are always keywords).
fn keywords() -> impl Iterator<Item = &'static str> {
    compiler::BLOCK_SPECIALS.iter().chain(compiler::OTHER_SPECIALS).chain(compiler::WORD_OPS).chain(compiler::CONTINUATIONS).copied().chain(["True", "False", "None", "defop"])
}

fn path_commands() -> Vec<String> {
    let mut cmds = std::collections::BTreeSet::new();
    if let Some(path) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&path) {
            if let Ok(rd) = std::fs::read_dir(d) {
                for e in rd.flatten() {
                    use std::os::unix::fs::PermissionsExt;
                    let exec = std::fs::metadata(e.path()).map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0).unwrap_or(false);
                    if exec {
                        cmds.insert(e.file_name().to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    cmds.into_iter().collect()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '+')
}

/// The name at byte `at` in a line (a `$` before it left out; a trailing
/// `.` or `-` too).
fn word_at(l: &str, at: usize) -> Option<(usize, usize)> {
    let name_char = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-');
    let at = at.min(l.len());
    let s = l[..at].rfind(|c: char| !name_char(c)).map(|i| i + l[i..].chars().next().unwrap().len_utf8()).unwrap_or(0);
    let e = l[at..].find(|c: char| !name_char(c)).map(|i| at + i).unwrap_or(l.len());
    let e = s + l[s..e].trim_end_matches('-').len();
    (s < e).then_some((s, e))
}

fn utf16_col(line: &str, byte: usize) -> u32 {
    line[..byte.min(line.len())].encode_utf16().count() as u32
}

fn byte_of_utf16(line: &str, col: u32) -> usize {
    let mut n = 0u32;
    for (i, c) in line.char_indices() {
        if n >= col {
            return i;
        }
        n += c.len_utf16() as u32;
    }
    line.len()
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum SymKind {
    Function,
    Macro,
    Operator,
    Class,
    Variable,
}

#[derive(Debug, Clone)]
pub struct Def {
    pub name: String,
    pub kind: SymKind,
    /// Its line (0-based), and the last line of its block.
    pub line: usize,
    pub end_line: usize,
    pub indent: usize,
    /// Byte range of the name on its line.
    pub name_at: (usize, usize),
    /// The `def` line as written (a one-line def without its body).
    pub signature: String,
    pub params: Vec<String>,
    /// The `#` comments right above it.
    pub doc: String,
}

/// The file's definitions, in order, read from its lines.
pub fn definitions(text: &str) -> Vec<Def> {
    let lines: Vec<&str> = text.lines().collect();
    let indent_of = |l: &str| l.len() - l.trim_start().len();
    let mut out = vec![];
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        let indent = indent_of(l);
        let (kind, rest) = if let Some(r) = t.strip_prefix("async def ") {
            (SymKind::Function, r)
        } else if let Some(r) = t.strip_prefix("def ") {
            (SymKind::Function, r)
        } else if let Some(r) = t.strip_prefix("defmacro ") {
            (SymKind::Macro, r)
        } else if let Some(r) = t.strip_prefix("defop ") {
            (SymKind::Operator, r)
        } else if let Some(r) = t.strip_prefix("class ") {
            (SymKind::Class, r)
        } else if indent == 0 {
            // (A top-level `name = …`.)
            match t.split_once(" = ") {
                Some((n, _)) if !n.trim().is_empty() && n.trim().chars().all(|c| c.is_alphanumeric() || c == '_') && !n.trim().starts_with(|c: char| c.is_ascii_digit()) && !keywords().any(|k| k == n.trim()) => (SymKind::Variable, t),
                _ => continue,
            }
        } else {
            continue;
        };
        let rest = rest.trim_start();
        let name: String = if kind == SymKind::Variable {
            rest.split(['=', ' ']).next().unwrap_or("").to_string()
        } else {
            rest.split_whitespace().next().unwrap_or("").trim_end_matches(':').to_string()
        };
        if name.is_empty() {
            continue;
        }
        let ns = l.len() - rest.len();
        let name_at = (ns, ns + name.len());
        let head = t.split_once(" = ").filter(|_| kind == SymKind::Function).map(|(h, _)| h).unwrap_or(t).trim_end();
        let signature = if kind == SymKind::Variable { t.to_string() } else { head.trim_end_matches(':').to_string() };
        let params: Vec<String> = if kind == SymKind::Function || kind == SymKind::Macro {
            head.split_whitespace()
                .skip_while(|w| *w == "async" || *w == "def" || *w == "defmacro")
                .skip(1)
                .map(|p| p.trim_start_matches('*').split('=').next().unwrap_or("").trim_end_matches(':').to_string())
                .filter(|p| !p.is_empty() && p.chars().all(|c| c.is_alphanumeric() || c == '_'))
                .collect()
        } else {
            vec![]
        };
        // (Its block: the lines after it indented more, blank lines inside.)
        let mut end = i;
        for (j, m) in lines.iter().enumerate().skip(i + 1) {
            if m.trim().is_empty() {
                continue;
            }
            if indent_of(m) <= indent {
                break;
            }
            end = j;
        }
        let mut doc: Vec<&str> = vec![];
        for m in lines[..i].iter().rev() {
            match m.trim_start().strip_prefix('#') {
                Some(c) if !m.trim_start().starts_with("#!") => doc.push(c.strip_prefix(' ').unwrap_or(c)),
                _ => break,
            }
        }
        doc.reverse();
        out.push(Def { name, kind, line: i, end_line: end, indent, name_at, signature, params, doc: doc.join("\n") });
    }
    out
}

/// The definition a name at a line refers to: the nearest one above it at
/// the same or an outer level, else any in the file.
fn find_definition(text: &str, w: &str, line: usize) -> Option<Def> {
    let w = w.trim_start_matches('$');
    let defs = definitions(text);
    let enclosing_or_top = |d: &&Def| d.indent == 0 || defs.iter().any(|o| o.line < d.line && d.line <= o.end_line && o.line < line && line <= o.end_line);
    defs.iter().filter(|d| d.name == w && d.line <= line).filter(enclosing_or_top).last().or_else(|| defs.iter().find(|d| d.name == w)).cloned().or_else(|| {
        // (A parameter: its function's def line.)
        defs.iter().filter(|d| d.line <= line && line <= d.end_line && d.params.iter().any(|p| p == w)).last().map(|d| {
            let l = text.lines().nth(d.line).unwrap_or("");
            let at = l.find(&format!(" {w}")).map(|i| i + 1).or_else(|| l.find(&format!("*{w}")).map(|i| i + 1)).unwrap_or(d.name_at.0);
            Def { name: w.to_string(), kind: SymKind::Variable, line: d.line, end_line: d.line, indent: d.indent, name_at: (at, at + w.len()), signature: format!("{w}: parameter of {}", d.signature), params: vec![], doc: String::new() }
        })
    })
}

fn definition(uri: &str, text: &str, line: usize, ch: u32) -> Value {
    let l = text.lines().nth(line).unwrap_or("");
    let Some((s, e)) = word_at(l, byte_of_utf16(l, ch)) else { return Value::Null };
    match find_definition(text, &l[s..e], line) {
        Some(d) => {
            let dl = text.lines().nth(d.line).unwrap_or("");
            json!({"uri": uri, "range": {"start": {"line": d.line, "character": utf16_col(dl, d.name_at.0)}, "end": {"line": d.line, "character": utf16_col(dl, d.name_at.1)}}})
        }
        None => Value::Null,
    }
}

/// The file's definitions as LSP DocumentSymbols, nested by their blocks.
pub fn document_symbols(text: &str) -> Value {
    let defs = definitions(text);
    let lines: Vec<&str> = text.lines().collect();
    fn build(defs: &[Def], lines: &[&str], from: usize, to: usize, parent_end: Option<usize>) -> (Vec<Value>, usize) {
        let mut out = vec![];
        let mut i = from;
        while i < to {
            let d = &defs[i];
            if let Some(pe) = parent_end {
                if d.line > pe {
                    break;
                }
            }
            // (Its children: the definitions inside its block.)
            let mut j = i + 1;
            while j < to && defs[j].line <= d.end_line {
                j += 1;
            }
            let (children, _) = build(defs, lines, i + 1, j, Some(d.end_line));
            let l = lines.get(d.line).copied().unwrap_or("");
            let el = lines.get(d.end_line).copied().unwrap_or("");
            let kind = match d.kind {
                SymKind::Function if parent_end.is_some() && defs.iter().any(|p| p.kind == SymKind::Class && p.line < d.line && d.line <= p.end_line) => 6,
                SymKind::Function | SymKind::Macro => 12,
                SymKind::Operator => 25,
                SymKind::Class => 5,
                SymKind::Variable => 13,
            };
            let mut sym = json!({
                "name": d.name,
                "detail": d.signature,
                "kind": kind,
                "range": {"start": {"line": d.line, "character": 0}, "end": {"line": d.end_line, "character": utf16_col(el, el.len())}},
                "selectionRange": {"start": {"line": d.line, "character": utf16_col(l, d.name_at.0)}, "end": {"line": d.line, "character": utf16_col(l, d.name_at.1)}},
            });
            if !children.is_empty() {
                sym["children"] = Value::Array(children);
            }
            out.push(sym);
            i = j;
        }
        (out, i)
    }
    Value::Array(build(&defs, &lines, 0, defs.len(), None).0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "# Say hello.\ndef greet who greeting=hi\n  \"$greeting, $who\"\n\nclass Point\n  def norm self\n    0\n\nname = world\ndef sq x = $x * $x\nprint (greet $name)\n";

    #[test]
    fn definitions_from_lines() {
        let d = definitions(SRC);
        let names: Vec<(&str, SymKind)> = d.iter().map(|d| (d.name.as_str(), d.kind)).collect();
        assert_eq!(names, [("greet", SymKind::Function), ("Point", SymKind::Class), ("norm", SymKind::Function), ("name", SymKind::Variable), ("sq", SymKind::Function)]);
        assert_eq!(d[0].params, ["who", "greeting"]);
        assert_eq!(d[0].doc, "Say hello.");
        assert_eq!((d[0].line, d[0].end_line), (1, 2));
        assert_eq!(d[4].signature, "def sq x");
        assert_eq!(&SRC.lines().nth(1).unwrap()[d[0].name_at.0..d[0].name_at.1], "greet");
    }

    #[test]
    fn symbols_nest_and_definition_found() {
        let s = document_symbols(SRC);
        assert_eq!(s[1]["name"], "Point");
        assert_eq!(s[1]["children"][0]["name"], "norm");
        assert_eq!(s[1]["children"][0]["kind"], 6);
        let r = definition("file:///x.tsh", SRC, 10, 9);
        assert_eq!(r["range"]["start"], json!({"line": 1, "character": 4}));
        // ($who in greet's body: the parameter on its def line.)
        let r = definition("file:///x.tsh", SRC, 2, 17);
        assert_eq!(r["range"]["start"], json!({"line": 1, "character": 10}));
    }

    #[test]
    fn positions_are_utf16() {
        let l = "é😀 $name";
        assert_eq!(byte_of_utf16(l, 3), "é😀".len());
        assert_eq!(utf16_col(l, l.len()), 9);
        let at = byte_of_utf16(l, 6);
        let (s, e) = word_at(l, at).unwrap();
        assert_eq!(&l[s..e], "name");
    }

    /// A program's arguments and paths: the REPL's completer, from the
    /// file's folder, replacing the whole word (dashes, folders and all);
    /// a tessh function's arguments: the file's names, as before.
    #[test]
    fn arguments_and_paths_complete() {
        let _g = runner::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("tessh-lsp-args-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/foo.txt"), "").unwrap();
        std::fs::write(dir.join("sub/fob.rs"), "").unwrap();
        let uri = format!("file://{}/demo.tsh", dir.display());
        let text = "cat sub/fo\ndef f x = $x\nf gr\n./sub/f\ncd su\n";
        let cwd = std::env::current_dir().unwrap();
        Python::attach(|py| {
            let mut sv = Server::new(py, std::fs::File::create("/dev/null").unwrap()).unwrap();
            let edits = |r: &Value| -> Vec<(u64, String)> { r["items"].as_array().unwrap().iter().filter_map(|i| Some((i["textEdit"]["range"]["start"]["character"].as_u64()?, i["textEdit"]["newText"].as_str()?.to_string()))).collect() };
            let r = sv.completion(py, &uri, text, 0, 10);
            assert_eq!(edits(&r), [(4, "sub/fob.rs".to_string()), (4, "sub/foo.txt".to_string())], "{r}");
            let r = sv.completion(py, &uri, text, 3, 7);
            assert_eq!(edits(&r), [(0, "./sub/fob.rs".to_string()), (0, "./sub/foo.txt".to_string())], "{r}");
            let r = sv.completion(py, &uri, text, 4, 5);
            assert_eq!(edits(&r), [(3, "sub/".to_string())], "{r}");
            let r = sv.completion(py, &uri, text, 2, 4);
            assert!(edits(&r).is_empty() && r["items"].as_array().unwrap().iter().any(|i| i["label"] == "group_by"), "{r}");
        });
        assert_eq!(std::env::current_dir().unwrap(), cwd, "the folder is put back");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn diagnostics_as_ranges() {
        let _g = runner::tests::LOCK.lock().unwrap_or_else(|e| e.into_inner());
        Python::attach(|py| {
            assert!(diagnostics(py, SRC, "<test>").is_empty());
            let d = diagnostics(py, "x = 1\ny = (é + \n", "<test>");
            assert_eq!(d.len(), 1, "{d:?}");
            assert_eq!(d[0]["range"]["start"]["line"], 1);
            assert_eq!(d[0]["severity"], 1);
        });
    }
}
