//! The REPL on tessh's own editor (crate::editor): what the editor asks of
//! tessh (highlighting, completion, the check, help, macros), its settings
//! (Tessera's, so it matches the user's Tessera, with tessh's own choices
//! over them), the history file, and the loop: read, run, again. A child of
//! repl (its helpers: doc_for, complete_items, the shell integration).

use super::*;
use crate::editor::core::{Hooks, Item as EdItem, Keys, MenuItem, MenuKind, Outcome};
use crate::editor::run::{BarFn, Editor};
use crate::editor::text::{Indent, IndentMode};
use crate::editor::{Line, Span, Sty};

struct TesshHooks {
    ns: Py<PyDict>,
    checked: (String, Vec<(usize, usize, String)>),
}

/// A (line, column in characters) → byte offset.
fn offset(text: &str, line: u32, col: u32) -> usize {
    let mut a = 0;
    for (i, l) in text.split('\n').enumerate() {
        if i + 1 == line as usize {
            return a + l.char_indices().nth(col as usize).map(|(b, _)| b).unwrap_or(l.len());
        }
        a += l.len() + 1;
    }
    text.len()
}

impl Hooks for TesshHooks {
    fn highlight(&mut self, text: &str) -> Vec<(usize, usize, &'static str)> {
        tree_sitter_tessh::highlight::spans(text)
    }
    fn complete(&mut self, text: &str, pos: usize) -> (usize, usize, Vec<EdItem>) {
        let (s, e, items) = Python::attach(|py| complete_items(py, self.ns.bind(py), text, pos));
        (s, e, items.into_iter().map(|i| EdItem { label: i.label, insert: i.insert, kind: i.kind.to_string(), detail: i.detail, doc: i.doc }).collect())
    }
    fn check(&mut self, text: &str) -> Vec<(usize, usize, String)> {
        if self.checked.0 == text {
            return self.checked.1.clone();
        }
        let out = if incomplete(text) {
            vec![]
        } else {
            Python::attach(|py| runner::check(py, text, "<input>"))
                .into_iter()
                .map(|(l, c, e, m)| {
                    let a = offset(text, l, c);
                    let b = offset(text, l, e.max(c + 1));
                    (a, b.saturating_sub(a).max(1), if text.contains('\n') { format!("line {l}: {m}") } else { m })
                })
                .collect()
        };
        self.checked = (text.to_string(), out.clone());
        out
    }
    fn hover(&mut self, text: &str, pos: usize) -> String {
        let w = word_at(text, pos);
        if w.is_empty() {
            return String::new();
        }
        Python::attach(|py| {
            let v = self.ns.bind(py).get_item(w).ok().flatten().or_else(|| py.import("builtins").ok().and_then(|b| b.getattr(w).ok()));
            doc_for(py, w, v.as_ref())
        })
    }
    fn macros(&mut self) -> Vec<String> {
        Python::attach(|py| runner::macro_names(self.ns.bind(py)))
    }
}

// ---- settings ----------------------------------------------------------------------

fn config_dir() -> std::path::PathBuf {
    std::env::var("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"))
}
fn read_json(p: &std::path::Path) -> serde_json::Value {
    std::fs::read_to_string(p).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(serde_json::json!({}))
}
/// tessh's own choices (keys, indentation), kept apart from Tessera's file.
fn own_file() -> std::path::PathBuf {
    config_dir().join("tessh/editor.json")
}
fn save_own(key: &str, v: serde_json::Value) {
    let f = own_file();
    let mut j = read_json(&f);
    if v.is_null() {
        j.as_object_mut().map(|o| o.remove(key));
    } else {
        j[key] = v;
    }
    let _ = std::fs::create_dir_all(f.parent().unwrap());
    let _ = std::fs::write(&f, serde_json::to_string_pretty(&j).unwrap_or_default());
}

fn apply_settings(ed: &mut crate::editor::core::Ed) {
    let t = read_json(&config_dir().join("tessera/settings.json"));
    let own = read_json(&own_file());
    let get = |k: &str| own.get(k).filter(|v| !v.is_null()).or_else(|| t.get(k)).cloned();
    let keys = std::env::var("TESSH_KEYS").ok().or_else(|| get("editor_keys").and_then(|v| v.as_str().map(String::from))).unwrap_or_else(|| "emacs".into());
    ed.keys = Keys::parse(&keys);
    ed.line_numbers = get("editor_line_numbers").and_then(|v| v.as_bool()).unwrap_or(true);
    ed.indent_guides = get("editor_indent_guides").and_then(|v| v.as_bool()).unwrap_or(true);
    ed.detect = get("editor_detect_indent").and_then(|v| v.as_bool()).unwrap_or(true);
    let langs = get("editor_indent_langs").unwrap_or(serde_json::json!({}));
    let m = langs.get("tessh").cloned().unwrap_or(serde_json::json!({}));
    let mode = match m.get("mode").and_then(|v| v.as_str()) {
        Some("tabs") => IndentMode::Tabs,
        Some("mixed") => IndentMode::Mixed,
        _ => IndentMode::Spaces,
    };
    let size = m.get("size").and_then(|v| v.as_u64()).filter(|n| *n > 0).unwrap_or(2) as usize;
    ed.indent_pref = Indent { mode, size };
    ed.indent_default = Indent { mode: IndentMode::Spaces, size: 2 };
    ed.indent_own = m.get("mode").is_some() || m.get("size").is_some();
}

/// The indentation choice (the Spaces chip's menu): kept per language, as
/// Tessera keeps it.
fn set_indent_pref(key: &str, value: serde_json::Value) {
    let own = read_json(&own_file());
    let mut langs = own.get("editor_indent_langs").cloned().unwrap_or_else(|| read_json(&config_dir().join("tessera/settings.json")).get("editor_indent_langs").cloned().unwrap_or(serde_json::json!({})));
    let mut m = langs.get("tessh").cloned().unwrap_or(serde_json::json!({}));
    if value.is_null() {
        m.as_object_mut().map(|o| o.remove(key));
    } else {
        m[key] = value;
    }
    langs["tessh"] = m;
    save_own("editor_indent_langs", langs);
}

fn indent_menu(ed: &crate::editor::core::Ed) -> Vec<MenuItem> {
    let p = &ed.indent_pref;
    let it = |label: String, cmd: &str, checked: bool, enabled: bool| MenuItem { label, cmd: cmd.into(), section: false, checked, enabled };
    let mut v = vec![
        it("Indent with spaces".into(), "mode=spaces", p.mode == IndentMode::Spaces, true),
        it("Indent with tabs".into(), "mode=tabs", p.mode == IndentMode::Tabs, true),
        it("Mixed (a tab for every 8 columns)".into(), "mode=mixed", p.mode == IndentMode::Mixed, true),
        MenuItem { label: "Size".into(), section: true, ..Default::default() },
    ];
    for n in [1, 2, 4, 8] {
        v.push(it(n.to_string(), &format!("size={n}"), p.size == n, true));
    }
    v.push(it("Detect from the text".into(), "detect", ed.detect, true));
    v.push(it("Use tessh's default (spaces, 2)".into(), "default", false, ed.indent_own));
    v
}

// ---- history -----------------------------------------------------------------------

fn read_all_history(f: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(f).unwrap_or_default().lines().filter(|l| !l.trim().is_empty()).map(|l| l.replace("<\\n>", "\n")).collect()
}
fn append_history(f: &std::path::Path, text: &str) {
    use std::io::Write;
    if let Ok(mut o) = std::fs::OpenOptions::new().create(true).append(true).open(f) {
        let _ = writeln!(o, "{}", text.replace('\n', "<\\n>"));
    }
}

// ---- the prompt bar ------------------------------------------------------------------

fn real_bar() -> BarFn {
    std::sync::Arc::new(|cwd: &str, status: i32, du: u64, w: u16, v: &[String], c: Option<((u8, u8, u8), (u8, u8, u8))>| {
        let b = crate::editor::promptbar::bar(&crate::editor::promptbar::BarCtx { cwd: cwd.into(), status, duration_ms: du, width: w, visited: v.to_vec(), colors: c });
        (b.left, b.right)
    })
}

fn bar_ctx(ed: &crate::editor::core::Ed) -> crate::editor::promptbar::BarCtx {
    crate::editor::promptbar::BarCtx { cwd: ed.cwd.clone(), status: ed.status, duration_ms: ed.duration_ms, width: 80, visited: ed.visited.clone(), colors: None }
}

fn menu_of(cs: Vec<crate::editor::promptbar::Choice>) -> Vec<MenuItem> {
    cs.into_iter().map(|c| MenuItem { enabled: !c.section, label: c.label, cmd: c.cmd, section: c.section, checked: c.current }).collect()
}

// ---- the loop ------------------------------------------------------------------------

fn run_line(py: Python<'_>, ns: &Bound<'_, PyDict>, text: &str) -> Option<i32> {
    let r = runner::run_source(py, text, ns, "<stdin>", true);
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
}

pub fn run_term(ns: Py<PyDict>, hist: std::path::PathBuf, bar: Option<BarFn>) -> i32 {
    let mut editor = Editor::new(bar.unwrap_or_else(real_bar));
    apply_settings(&mut editor.ed);
    editor.ed.history = read_all_history(&hist);
    {
        let ai = editor.ed.ai.clone();
        std::thread::spawn(move || {
            let inline = crate::ai::ready("inline");
            let command = crate::ai::ready("command");
            let mut a = ai.lock().unwrap();
            a.inline_ready = inline;
            a.command_ready = command;
        });
    }
    let mut hooks = TesshHooks { ns: Python::attach(|py| ns.clone_ref(py)), checked: Default::default() };
    let host = hostname();
    let mut code = 0;
    let mut last_cmd = String::new();
    let mut started = Instant::now();
    let mut fresh = true;
    loop {
        if fresh {
            Python::attach(|py| apply_handover(py));
            write_env();
            let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
            osc(&format!("7;file://{host}{cwd}"));
            osc("133;A");
            editor.ed.status = runtime::STATUS.load(Ordering::SeqCst);
            if editor.ed.visited.first() != Some(&cwd) {
                editor.ed.visited.retain(|p| *p != cwd);
                editor.ed.visited.insert(0, cwd.clone());
                editor.ed.visited.truncate(30);
            }
            editor.ed.cwd = cwd;
            fresh = false;
        }
        let out = editor.read(&mut hooks);
        let mut run: Option<(String, bool, bool)> = None;
        match out {
            Outcome::Eof => break,
            Outcome::Interrupt => {
                fresh = true;
                continue;
            }
            Outcome::Submit { text, background } => run = Some((text, background, true)),
            Outcome::Host(c) => {
                let (what, arg) = c.split_once(':').unwrap_or((c.as_str(), ""));
                match what {
                    "guide" => crate::editor::pager::show(&crate::help::guide(), arg),
                    "help" => match crate::help::lookup(arg) {
                        Some(md) => print_md(&md),
                        None => println!("tessh: no help for {arg}"),
                    },
                    "jobs" => run = Some(("jobs".into(), false, false)),
                    "clear" => {
                        print!("\x1b[H\x1b[2J");
                        let _ = std::io::stdout().flush();
                    }
                    "keys" => save_own("editor_keys", serde_json::json!(arg)),
                    "run" => run = Some((arg.to_string(), false, false)),
                    "indent" => {
                        match arg.split_once('=') {
                            Some(("mode", m)) => set_indent_pref("mode", serde_json::json!(m)),
                            Some(("size", n)) => set_indent_pref("size", serde_json::json!(n.parse::<u64>().unwrap_or(2))),
                            _ if arg == "detect" => save_own("editor_detect_indent", serde_json::json!(!editor.ed.detect)),
                            _ if arg == "default" => {
                                set_indent_pref("mode", serde_json::Value::Null);
                                set_indent_pref("size", serde_json::Value::Null);
                            }
                            _ => {}
                        }
                        editor.ed.indent_picked = true;
                        apply_settings(&mut editor.ed);
                    }
                    "chip" => match arg {
                        "history" => editor.ed.open_history(),
                        "keys" => {
                            editor.ed.keys = editor.ed.keys.next();
                            save_own("editor_keys", serde_json::json!(editor.ed.keys.name()));
                        }
                        "indent" => {
                            let items = indent_menu(&editor.ed);
                            editor.ed.open_menu(MenuKind::Indent, items);
                        }
                        "guide" => {
                            let w = editor.ed.guide_word();
                            crate::editor::pager::show(&crate::help::guide(), &w);
                        }
                        "jobs" => run = Some(("jobs".into(), false, false)),
                        "fix" => {
                            let what = format!("{last_cmd} (exit status {})", editor.ed.status);
                            editor.ed.ask_command(what, "fix");
                        }
                        "folders" => {
                            let items = menu_of(crate::editor::promptbar::cd_history(&bar_ctx(&editor.ed)));
                            editor.ed.open_menu(MenuKind::Choices("dir".into()), items);
                        }
                        _ => {}
                    },
                    "widget" if arg == "status" => {
                        let what = format!("{last_cmd} (exit status {})", editor.ed.status);
                        editor.ed.ask_command(what, "fix");
                    }
                    "widget" => {
                        let items = menu_of(crate::editor::promptbar::choices(arg, &bar_ctx(&editor.ed)));
                        if !items.is_empty() {
                            editor.ed.open_menu(MenuKind::Choices(arg.to_string()), items);
                        }
                    }
                    _ => {}
                }
            }
        }
        let Some((text, background, from_editor)) = run else { continue };
        let text = if background && !text.trim_end().ends_with('&') { format!("{} &", text.trim_end()) } else { text.trim_end().to_string() };
        if text.trim().is_empty() {
            fresh = true;
            continue;
        }
        if from_editor {
            editor.ed.t.reset();
            editor.ed.hist_index = None;
            if !text.starts_with(' ') {
                editor.ed.history.retain(|h| *h != text);
                editor.ed.history.push(text.clone());
                append_history(&hist, &text);
            }
            last_cmd = text.clone();
        }
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;
        let exec = format!("{}\0{}\0{}", crate::runtime::user_name(), text, crate::runtime::env_list().join("\0"));
        osc(&format!("1337;TesseraExec={}", b64.encode(&exec)));
        osc(&format!("1337;TesseraCommand={}", b64.encode(&text)));
        osc("133;C");
        started = Instant::now();
        let exit = Python::attach(|py| run_line(py, ns.bind(py), &text));
        editor.ed.duration_ms = started.elapsed().as_millis() as u64;
        if let Some(c) = exit {
            osc(&format!("133;D;{c}"));
            code = c;
            break;
        }
        osc(&format!("133;D;{}", runtime::STATUS.load(Ordering::SeqCst)));
        fresh = true;
    }
    let _ = started;
    code
}
