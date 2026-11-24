//! The terminal REPL, a daily shell in any terminal: reedline (vendored,
//! tessh/vendor) with tessh's own Enter (runs what's complete; in a block,
//! a new line indented for it, and a blank line ends the block, as IPython
//! does), tree-sitter highlighting (`tree_sitter_tessh::highlight`, the
//! colors Tessera uses), completion with each name's help beside it, grey
//! suggestions from history and the AI, a live syntax check under the
//! input, `# what you want` for the AI to write a command, and shell
//! integration (OSC 133 prompt and command marks, OSC 7 folder, Tessera's
//! command line, environment file and hand-overs from its other kernels).
//!
//! Keys Tessera's editor has that a terminal can't send (a lone tap of left
//! or right Shift or Ctrl; Shift+Enter, Ctrl+Enter in most terminals) have
//! alternatives every terminal sends: see KEYS (shown by F1 on an empty line
//! and in `help keys`). Under the kitty keyboard protocol (kitty, foot,
//! WezTerm, Ghostty, Alacritty, iTerm2) the real keys work too.

#[path = "repl_term.rs"]
mod term;

use crate::compiler::BLOCK_SPECIALS;
use crate::reader::incomplete;
use crate::runner::{self, TesshError};
use crate::runtime;
use crossterm::event::{Event, KeyCode as CKeyCode, KeyEventKind, ModifierKeyCode};
use nu_ansi_term::{Color, Style};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use reedline::{
    default_emacs_keybindings, default_vi_insert_keybindings, default_vi_normal_keybindings, default_vi_visual_keybindings, ColumnarMenu, Completer, CompletionResult, DescriptionMode, EditCommand, EditHook, EditHookContext, EditMode, Emacs,
    FileBackedHistory, Hinter, History, IdeMenu, KeyCode, KeyModifiers, Keybindings, MenuBuilder, Prompt, PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, Reedline, ReedlineEvent, ReedlineMenu, ReedlineRawEvent,
    RepaintSignal, Signal, Span, StatusLine, StyledText, Suggestion, ValidationResult, Validator, Vi,
};
use std::borrow::Cow;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The keys Tessera's editor binds to a lone tap of a modifier, or to a
/// chord most terminals send as plain Enter, and what to press instead.
/// (Tessera key, what it does, the alternative here, kitty-protocol keys.)
pub const KEYS: &[(&str, &str, &str, &str)] = &[
    ("Right Shift (tap)", "New line", "Alt+Enter or Ctrl+J", "right Shift tap, Shift+Enter"),
    ("Shift+Enter", "New line", "Alt+Enter or Ctrl+J", "Shift+Enter"),
    ("Left Shift (tap)", "Take the grey suggestion; with none, ask the AI for one", "→ or End at the end takes it; Ctrl+Space takes it or asks", "left Shift tap"),
    ("Right Ctrl (tap)", "Run in the background (as with a trailing &)", "Alt+&", "right Ctrl tap, Ctrl+Enter"),
    ("Ctrl+Enter", "Run in the background", "Alt+&", "Ctrl+Enter"),
    ("Left Ctrl (tap)", "Switch between the editor and the terminal", "Not needed: here the editor is the terminal, and keys typed while a command runs wait for the prompt", "—"),
];

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
    // (time alone times the block under it; coproc alone, or coproc NAME
    // when NAME isn't a program, runs the block as the coprocess.)
    if h == "time" && words[1..].iter().all(|w| *w == "-p") {
        return true;
    }
    if h == "coproc" && (words.len() == 1 || (words.len() == 2 && crate::reader::is_ident(words[1]) && crate::runtime::which(words[1]).is_none())) {
        return true;
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

// ---- highlighting -----------------------------------------------------------------

/// The style for a highlight kind (`tree_sitter_tessh::highlight`'s).
fn kind_style(kind: &str) -> Option<Style> {
    Some(match kind {
        "keyword" => Color::Magenta.bold(),
        "string" => Style::new().fg(Color::Green),
        "comment" => Style::new().dimmed().italic(),
        "number" | "constant" => Style::new().fg(Color::Magenta),
        "operator" | "attribute" | "escape" => Style::new().fg(Color::Yellow),
        "function" => Color::Blue.bold(),
        "variable" => Style::new().fg(Color::Cyan),
        "punctuation" => Style::new().fg(Color::DarkGray),
        _ => return None,
    })
}

/// The text in runs of one kind each (later spans over earlier ones).
fn runs(text: &str) -> Vec<(usize, usize, Option<&'static str>)> {
    runs_of(text, tree_sitter_tessh::highlight::spans(text))
}

fn runs_of(text: &str, spans: Vec<(usize, usize, &'static str)>) -> Vec<(usize, usize, Option<&'static str>)> {
    let mut kinds: Vec<Option<&'static str>> = vec![None; text.len()];
    for (s, e, k) in spans {
        for x in kinds.iter_mut().take(e.min(text.len())).skip(s) {
            *x = Some(k);
        }
    }
    let mut out = vec![];
    let mut start = 0;
    for i in 1..=text.len() {
        if i == text.len() || kinds[i] != kinds[start] {
            if text.is_char_boundary(i) {
                out.push((start, i, kinds[start]));
                start = i;
            }
        }
    }
    out
}

/// A text with ANSI colors, as the editor shows it (for help pages).
pub fn highlight(text: &str) -> String {
    paint(text, runs(text))
}

/// Bash code with ANSI colors in the same palette (the Advanced guide's
/// "bash (original)" examples): tree-sitter-bash's highlight query, with
/// its variables and expansions too.
pub fn highlight_bash(text: &str) -> String {
    use std::sync::OnceLock;
    use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
    static Q: OnceLock<Option<Query>> = OnceLock::new();
    let lang: tree_sitter::Language = tree_sitter_bash::LANGUAGE.into();
    let q = Q.get_or_init(|| {
        let src = format!("{}\n(variable_name) @variable\n(simple_expansion) @variable\n(expansion) @variable\n", tree_sitter_bash::HIGHLIGHT_QUERY);
        Query::new(&lang, &src).ok()
    });
    let mut p = Parser::new();
    let (Some(q), Ok(())) = (q, p.set_language(&lang)) else { return text.to_string() };
    let Some(tree) = p.parse(text, None) else { return text.to_string() };
    let names = q.capture_names();
    let mut spans = vec![];
    let mut cursor = QueryCursor::new();
    let mut caps = cursor.captures(q, tree.root_node(), text.as_bytes());
    while let Some((m, i)) = caps.next() {
        let c = m.captures()[*i];
        let (s, e) = (c.node.start_byte(), c.node.end_byte().min(text.len()));
        if e > s {
            let k = match names[c.index as usize].split('.').next().unwrap_or("") {
                "property" => "variable",
                "embedded" => "variable",
                k => k,
            };
            // (Only the kinds the palette knows: the names are 'static there.)
            let k: &'static str = ["keyword", "string", "comment", "number", "constant", "operator", "function", "variable"].into_iter().find(|x| *x == k).unwrap_or("");
            if !k.is_empty() {
                spans.push((s, e, k));
            }
        }
    }
    paint(text, runs_of(text, spans))
}

fn paint(text: &str, runs: Vec<(usize, usize, Option<&'static str>)>) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for (s, e, k) in runs {
        match k.and_then(kind_style) {
            Some(st) => out.push_str(&st.paint(&text[s..e]).to_string()),
            None => out.push_str(&text[s..e]),
        }
    }
    out
}

struct TesshHighlighter;

impl reedline::Highlighter for TesshHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        let mut st = StyledText::new();
        for (s, e, k) in runs(line) {
            st.push((k.and_then(kind_style).unwrap_or_default(), line[s..e].to_string()));
        }
        st
    }
}

// ---- state the editor's parts share -----------------------------------------------

#[derive(Default)]
struct Shared {
    /// Single-line history entries, oldest first (for suggestions).
    history: Vec<String>,
    macros: Vec<String>,
    /// The suggestion shown now, and the text it's for.
    hint: String,
    hint_for: String,
    ai: Ai,
}

#[derive(Default)]
struct Ai {
    inline_ready: bool,
    command_ready: bool,
    /// What the hinter would like asked: (task, text, since).
    want: Option<(&'static str, String, Instant)>,
    /// Asked now (a key asked it): skip the pause.
    now: bool,
    asking: bool,
    /// The last answers: (task, text asked) → answer.
    answers: Vec<((&'static str, String), String)>,
    note: String,
}

impl Ai {
    fn answer(&self, task: &'static str, text: &str) -> Option<&str> {
        self.answers.iter().rev().find(|((t, q), _)| *t == task && q == text).map(|(_, a)| a.as_str())
    }
    fn asked(&self, task: &'static str, text: &str) -> bool {
        self.answers.iter().any(|((t, q), _)| *t == task && q == text)
    }
}

type SharedRef = Arc<Mutex<Shared>>;

fn recent(sh: &Shared, n: usize) -> String {
    let h = &sh.history;
    h[h.len().saturating_sub(n)..].join("\n")
}

/// Called every poll by the editor: asks the AI once typing has paused.
fn ai_tick(shared: &SharedRef, repaint: &RepaintSignal) {
    let (task, text, recent_cmds) = {
        let mut sh = shared.lock().unwrap();
        let Some((task, text, since)) = sh.ai.want.clone() else { return };
        let pause = match task {
            "command" => 1400,
            "request" => 250,
            _ => 400,
        };
        if sh.ai.asking || sh.ai.asked(task, &text) || (!sh.ai.now && since.elapsed() < Duration::from_millis(pause)) {
            return;
        }
        sh.ai.now = false;
        sh.ai.asking = true;
        (task, text.clone(), recent(&sh, 10))
    };
    let shared = shared.clone();
    let repaint = repaint.clone();
    std::thread::spawn(move || {
        let q = if task == "inline" { text.clone() } else { text.trim_start_matches('#').trim().to_string() };
        let r = crate::ai::ask(task, &q, &recent_cmds);
        let mut sh = shared.lock().unwrap();
        sh.ai.asking = false;
        let a = match r {
            Ok(a) => a,
            Err(e) => {
                sh.ai.note = format!("AI: {e}");
                String::new()
            }
        };
        sh.ai.answers.push(((task, text), a));
        if sh.ai.answers.len() > 50 {
            sh.ai.answers.remove(0);
        }
        repaint.request_repaint();
    });
}

// ---- suggestions ------------------------------------------------------------------

struct TesshHinter {
    shared: SharedRef,
}

impl Hinter for TesshHinter {
    fn handle(&mut self, line: &str, pos: usize, _history: &dyn History, use_ansi_coloring: bool, _cwd: &str) -> String {
        let mut sh = self.shared.lock().unwrap();
        sh.hint.clear();
        sh.hint_for = line.to_string();
        if pos < line.len() || line.contains('\n') || line.trim().len() < 2 {
            sh.ai.want = None;
            return String::new();
        }
        let shown;
        if line.starts_with('#') {
            // "# what I want": the request finished, then the command.
            let mut s = String::new();
            if let Some(a) = sh.ai.answer("request", line).filter(|a| a.len() > line.len() && a.starts_with(line)) {
                s = a[line.len()..].to_string();
                sh.hint = s.clone();
            }
            if let Some(c) = sh.ai.answer("command", line).filter(|c| !c.is_empty()) {
                s.push_str(&format!("  → {c}"));
            }
            sh.ai.want = if !sh.ai.command_ready {
                None
            } else if sh.ai.inline_ready && !sh.ai.asked("request", line) {
                Some(("request", line.to_string(), Instant::now()))
            } else if !sh.ai.asked("command", line) {
                Some(("command", line.to_string(), sh.ai.want.as_ref().filter(|w| w.1 == line).map(|w| w.2).unwrap_or_else(Instant::now)))
            } else {
                None
            };
            shown = s;
        } else {
            let from_history = sh.history.iter().rev().find(|h| h.len() > line.len() && h.starts_with(line)).map(|h| h[line.len()..].to_string());
            match from_history {
                Some(h) => {
                    sh.ai.want = None;
                    sh.hint = h;
                }
                None => {
                    if let Some(a) = sh.ai.answer("inline", line).filter(|a| a.len() > line.len() && a.starts_with(line)) {
                        sh.hint = a[line.len()..].to_string();
                        sh.ai.want = None;
                    } else if sh.ai.inline_ready && !sh.ai.asked("inline", line) {
                        let since = sh.ai.want.as_ref().filter(|w| w.1 == line).map(|w| w.2).unwrap_or_else(Instant::now);
                        sh.ai.want = Some(("inline", line.to_string(), since));
                    } else {
                        sh.ai.want = None;
                    }
                }
            }
            shown = sh.hint.clone();
        }
        if shown.is_empty() {
            return String::new();
        }
        if use_ansi_coloring {
            Style::new().fg(Color::DarkGray).paint(shown).to_string()
        } else {
            shown
        }
    }

    fn complete_hint(&self) -> String {
        self.shared.lock().unwrap().hint.clone()
    }

    fn next_hint_token(&self) -> String {
        let h = self.shared.lock().unwrap().hint.clone();
        let t = h.trim_start();
        let lead = h.len() - t.len();
        let end = t.find(char::is_whitespace).unwrap_or(t.len());
        h[..lead + end].to_string()
    }
}

// ---- the line under the input: syntax check, AI notes -------------------------------

struct TesshStatus {
    shared: SharedRef,
    cache: (String, String),
}

impl StatusLine for TesshStatus {
    fn status(&mut self, line: &str, _pos: usize, _columns: u16, ansi: bool) -> String {
        let paint = |st: Style, s: String| if ansi { st.paint(s).to_string() } else { s };
        if line.trim().is_empty() {
            return String::new();
        }
        {
            let sh = self.shared.lock().unwrap();
            if !sh.ai.note.is_empty() && sh.hint_for == line {
                return paint(Style::new().fg(Color::DarkGray), sh.ai.note.clone());
            }
            if line.starts_with('#') && !line.contains('\n') {
                if !sh.ai.command_ready {
                    return String::new();
                }
                let s = if sh.ai.asking { "asking the AI…" } else if sh.ai.answer("command", line).is_some_and(|c| !c.is_empty()) { "→ or Ctrl+Space takes the command; Enter asks again" } else { "Enter: the AI writes the command, for you to check" };
                return paint(Style::new().fg(Color::DarkGray), s.into());
            }
        }
        if self.cache.0 != line {
            let msg = if incomplete(line) {
                String::new()
            } else {
                Python::attach(|py| runner::check(py, line, "<input>")).into_iter().next().map(|(l, c, _, m)| if line.contains('\n') { format!("✗ line {l}:{}: {m}", c + 1) } else { format!("✗ {m}") }).unwrap_or_default()
            };
            self.cache = (line.to_string(), msg);
        }
        if self.cache.1.is_empty() {
            return String::new();
        }
        paint(Style::new().fg(Color::Red), self.cache.1.clone())
    }
}

// ---- completion -------------------------------------------------------------------

struct TesshCompleter {
    ns: Py<PyDict>,
}

impl Completer for TesshCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        let (start, end, items) = Python::attach(|py| complete_items(py, self.ns.bind(py), line, pos));
        let s: Vec<Suggestion> = items
            .into_iter()
            .map(|it| {
                let mut desc = it.detail.clone();
                if !it.doc.is_empty() {
                    if !desc.is_empty() {
                        desc.push_str("\n\n");
                    }
                    desc.push_str(&it.doc);
                }
                Suggestion {
                    display_override: (it.label != it.insert).then(|| it.label.clone()),
                    value: it.insert,
                    description: (!desc.is_empty()).then_some(desc),
                    span: Span::new(start, end),
                    append_whitespace: false,
                    ..Default::default()
                }
            })
            .collect();
        CompletionResult::fresh(s)
    }
}

// ---- Enter, Tab, Backspace and the rest that look at the text -----------------------

struct TesshValidator {
    shared: SharedRef,
}

impl Validator for TesshValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let macros = self.shared.lock().unwrap().macros.clone();
        let lines: Vec<&str> = line.split('\n').collect();
        if needs_more(&lines, &macros) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}

struct TesshHook {
    shared: SharedRef,
}

fn edit(cmds: Vec<EditCommand>) -> Option<ReedlineEvent> {
    Some(ReedlineEvent::Edit(cmds))
}

impl TesshHook {
    fn macros(&self) -> Vec<String> {
        self.shared.lock().unwrap().macros.clone()
    }

    /// A new line at the cursor, indented for what's before it.
    fn newline(&self, ctx: &EditHookContext<'_>) -> Option<ReedlineEvent> {
        let before: Vec<&str> = ctx.buffer[..ctx.cursor].split('\n').collect();
        let ind = next_indent(&before, &self.macros());
        edit(vec![EditCommand::InsertNewline, EditCommand::InsertString(" ".repeat(ind))])
    }

    fn enter(&self, ctx: &EditHookContext<'_>) -> Option<ReedlineEvent> {
        let buf = ctx.buffer;
        // "# what I want": the AI writes the command (the main loop asks).
        if buf.starts_with('#') && !buf.contains('\n') && buf.trim_start_matches('#').trim().len() > 1 && self.shared.lock().unwrap().ai.command_ready {
            return Some(ReedlineEvent::ExecuteHostCommand("tessh:ai".into()));
        }
        // (In the middle of the input: a new line there.)
        if ctx.cursor < buf.trim_end_matches(' ').len() {
            return self.newline(ctx);
        }
        let macros = self.macros();
        let mut lines: Vec<String> = buf.split('\n').map(String::from).collect();
        let n = lines.len();
        // (An else, elif, except or finally deeper than its if or try goes
        // back to it.)
        let mut replaced = false;
        if n > 1 {
            let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
            let fixed = fix_indent(&refs[..n - 1], refs[n - 1]);
            if fixed != lines[n - 1] {
                lines[n - 1] = fixed;
                replaced = true;
            }
        }
        let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
        if needs_more(&refs, &macros) {
            let ind = next_indent(&refs, &macros);
            if replaced {
                return edit(vec![EditCommand::Clear, EditCommand::InsertString(format!("{}\n{}", lines.join("\n"), " ".repeat(ind)))]);
            }
            return edit(vec![EditCommand::MoveToEnd { select: false }, EditCommand::InsertNewline, EditCommand::InsertString(" ".repeat(ind))]);
        }
        if replaced {
            return Some(ReedlineEvent::Multiple(vec![ReedlineEvent::Edit(vec![EditCommand::Clear, EditCommand::InsertString(lines.join("\n"))]), ReedlineEvent::Submit]));
        }
        Some(ReedlineEvent::Submit)
    }

    /// The text before the cursor on its line, when it's only spaces.
    fn in_indent<'a>(ctx: &EditHookContext<'a>) -> Option<&'a str> {
        let line = ctx.buffer[..ctx.cursor].rsplit('\n').next().unwrap_or("");
        line.chars().all(|c| c == ' ').then_some(line)
    }

    fn outdent(ctx: &EditHookContext<'_>) -> Option<ReedlineEvent> {
        let start = ctx.buffer[..ctx.cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
        let n = indent_of(&ctx.buffer[start..]).min(2);
        if n == 0 {
            return Some(ReedlineEvent::None);
        }
        let mut cmds = vec![EditCommand::MoveToPosition { position: start, select: false }];
        cmds.extend(std::iter::repeat_n(EditCommand::Delete, n));
        cmds.push(EditCommand::MoveToPosition { position: ctx.cursor.saturating_sub(n).max(start), select: false });
        edit(cmds)
    }
}

impl EditHook for TesshHook {
    fn edit(&mut self, event: &ReedlineEvent, ctx: &EditHookContext<'_>) -> Option<ReedlineEvent> {
        match event {
            ReedlineEvent::Enter | ReedlineEvent::SubmitOrNewline if !ctx.menu_active => self.enter(ctx),
            ReedlineEvent::ExecuteHostCommand(c) => match c.as_str() {
                "tessh:newline" => self.newline(ctx),
                "tessh:background" => {
                    let t = ctx.buffer.trim_end();
                    if t.is_empty() {
                        return Some(ReedlineEvent::None);
                    }
                    let mut cmds = vec![EditCommand::MoveToEnd { select: false }];
                    if !t.ends_with('&') {
                        cmds.push(EditCommand::InsertString(" &".into()));
                    }
                    Some(ReedlineEvent::Multiple(vec![ReedlineEvent::Edit(cmds), ReedlineEvent::Submit]))
                }
                "tessh:accept" => {
                    let mut sh = self.shared.lock().unwrap();
                    if let Some(c) = sh.ai.answer("command", ctx.buffer).filter(|c| !c.is_empty()).map(String::from) {
                        return edit(vec![EditCommand::Clear, EditCommand::InsertString(c)]);
                    }
                    if !sh.hint.is_empty() && sh.hint_for == ctx.buffer {
                        return Some(ReedlineEvent::HistoryHintComplete);
                    }
                    // (Nothing to take: ask the AI now.)
                    if sh.ai.inline_ready && ctx.cursor == ctx.buffer.len() && !ctx.buffer.trim().is_empty() {
                        let task = if ctx.buffer.starts_with('#') { "command" } else { "inline" };
                        sh.ai.answers.retain(|((t, q), _)| !(*t == task && q == ctx.buffer));
                        sh.ai.want = Some((task, ctx.buffer.to_string(), Instant::now()));
                        sh.ai.now = true;
                    }
                    Some(ReedlineEvent::None)
                }
                "tessh:nothing" => Some(ReedlineEvent::None),
                _ => None,
            },
            ReedlineEvent::Edit(cmds) if !ctx.menu_active && cmds.len() == 1 => match &cmds[0] {
                EditCommand::InsertNewline => self.newline(ctx),
                EditCommand::Backspace => {
                    // (In a line's indentation: back a level, two spaces.)
                    let line = Self::in_indent(ctx).filter(|l| !l.is_empty())?;
                    let k = if line.len() % 2 == 0 { 2 } else { 1 };
                    edit(std::iter::repeat_n(EditCommand::Backspace, k).collect())
                }
                _ => None,
            },
            // Tab in the indentation indents; elsewhere it completes.
            ReedlineEvent::UntilFound(v) if !ctx.menu_active && matches!(v.first(), Some(ReedlineEvent::Menu(m)) if m == "completion") => {
                let ind = Self::in_indent(ctx)?;
                if ctx.cursor == 0 && ctx.buffer.is_empty() {
                    return None;
                }
                edit(vec![EditCommand::InsertString(" ".repeat(2 - ind.len() % 2))])
            }
            // Shift+Tab outdents (in the completion list, it goes back).
            ReedlineEvent::MenuPrevious if !ctx.menu_active => Self::outdent(ctx),
            // → at the end takes a "# request"'s command.
            ReedlineEvent::UntilFound(v) if matches!(v.first(), Some(ReedlineEvent::HistoryHintComplete)) && ctx.cursor == ctx.buffer.len() => {
                let sh = self.shared.lock().unwrap();
                let c = sh.ai.answer("command", ctx.buffer).filter(|c| !c.is_empty() && sh.hint.is_empty())?;
                edit(vec![EditCommand::Clear, EditCommand::InsertString(c.to_string())])
            }
            _ => None,
        }
    }
}

// ---- keys -------------------------------------------------------------------------

fn host(c: &str) -> ReedlineEvent {
    ReedlineEvent::ExecuteHostCommand(c.into())
}

fn add_tessh_keys(kb: &mut Keybindings) {
    use KeyCode as K;
    use KeyModifiers as M;
    kb.add_binding(M::NONE, K::Enter, ReedlineEvent::Enter);
    // A new line: Alt+Enter and Ctrl+J in any terminal; Shift+Enter where
    // the terminal tells it from Enter.
    for (m, k) in [(M::ALT, K::Enter), (M::SHIFT, K::Enter), (M::CONTROL, K::Char('j'))] {
        kb.add_binding(m, k, host("tessh:newline"));
    }
    // In the background: Alt+&; Ctrl+Enter where the terminal tells it.
    for (m, k) in [(M::ALT, K::Char('&')), (M::ALT | M::SHIFT, K::Char('&')), (M::CONTROL, K::Enter)] {
        kb.add_binding(m, k, host("tessh:background"));
    }
    // The suggestion, or ask the AI for one (left Shift's tap in Tessera).
    kb.add_binding(M::CONTROL, K::Char(' '), host("tessh:accept"));
    kb.add_binding(M::NONE, K::End, ReedlineEvent::UntilFound(vec![ReedlineEvent::HistoryHintComplete, ReedlineEvent::Edit(vec![EditCommand::MoveToLineEnd { select: false }])]));
    kb.add_binding(M::NONE, K::Tab, ReedlineEvent::UntilFound(vec![ReedlineEvent::Menu("completion".into()), ReedlineEvent::MenuNext]));
    kb.add_binding(M::SHIFT, K::BackTab, ReedlineEvent::MenuPrevious);
    kb.add_binding(M::NONE, K::BackTab, ReedlineEvent::MenuPrevious);
    kb.add_binding(M::NONE, K::Backspace, ReedlineEvent::Edit(vec![EditCommand::Backspace]));
    // Help for the word at the cursor (or the keys, on an empty line).
    kb.add_binding(M::NONE, K::F(1), host("tessh:help"));
    kb.add_binding(M::ALT, K::Char('h'), host("tessh:help"));
    kb.add_binding(M::CONTROL | M::ALT, K::Char('j'), host("tessh:jobs"));
    // A whole history entry at a time (↑/↓ go line by line in one).
    kb.add_binding(M::NONE, K::PageUp, ReedlineEvent::PreviousHistory);
    kb.add_binding(M::NONE, K::PageDown, ReedlineEvent::NextHistory);
}

/// The edit mode (Emacs or vi keys), with lone taps of Shift and Ctrl seen
/// where the terminal reports them (the kitty keyboard protocol).
struct TapMode {
    inner: Box<dyn EditMode>,
    pending: Option<(ModifierKeyCode, Instant)>,
}

impl EditMode for TapMode {
    fn parse_event(&mut self, event: ReedlineRawEvent) -> ReedlineEvent {
        let ev: Event = event.into();
        if let Event::Key(k) = &ev {
            if let CKeyCode::Modifier(m) = k.code {
                match k.kind {
                    KeyEventKind::Release => {
                        let tap = self.pending.take().filter(|(p, at)| *p == m && at.elapsed() < Duration::from_millis(450));
                        return match tap.map(|t| t.0) {
                            Some(ModifierKeyCode::RightShift) => host("tessh:newline"),
                            Some(ModifierKeyCode::LeftShift) => host("tessh:accept"),
                            Some(ModifierKeyCode::RightControl) => host("tessh:background"),
                            _ => ReedlineEvent::None,
                        };
                    }
                    _ => {
                        if self.pending.as_ref().is_none_or(|(p, _)| *p != m) {
                            self.pending = Some((m, Instant::now()));
                        }
                        return ReedlineEvent::None;
                    }
                }
            }
        }
        self.pending = None;
        match ReedlineRawEvent::try_from(ev) {
            Ok(raw) => self.inner.parse_event(raw),
            Err(_) => ReedlineEvent::None,
        }
    }

    fn edit_mode(&self) -> PromptEditMode {
        self.inner.edit_mode()
    }

    fn handle_mode_specific_event(&mut self, event: ReedlineEvent) -> reedline::EventStatus {
        self.inner.handle_mode_specific_event(event)
    }
}

fn edit_mode() -> Box<dyn EditMode> {
    let vi = std::env::var("TESSH_KEYS").is_ok_and(|k| k.eq_ignore_ascii_case("vi"));
    let inner: Box<dyn EditMode> = if vi {
        let mut ins = default_vi_insert_keybindings();
        add_tessh_keys(&mut ins);
        let mut normal = default_vi_normal_keybindings();
        normal.add_binding(KeyModifiers::NONE, KeyCode::Enter, ReedlineEvent::Enter);
        Box::new(Vi::new(ins, normal, default_vi_visual_keybindings()))
    } else {
        let mut kb = default_emacs_keybindings();
        add_tessh_keys(&mut kb);
        Box::new(Emacs::new(kb))
    };
    Box::new(TapMode { inner, pending: None })
}

/// The kitty keyboard protocol's flags tessh asks for: every key as an
/// escape code, with releases, so a lone Shift or Ctrl can be seen.
fn kitty_flags() -> crossterm::event::KeyboardEnhancementFlags {
    use crossterm::event::KeyboardEnhancementFlags as F;
    F::DISAMBIGUATE_ESCAPE_CODES | F::REPORT_EVENT_TYPES | F::REPORT_ALTERNATE_KEYS | F::REPORT_ALL_KEYS_AS_ESCAPE_CODES
}

// ---- the prompt -------------------------------------------------------------------

struct TesshPrompt {
    cwd: String,
    status: i32,
}

impl Prompt for TesshPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Owned(Color::Blue.bold().paint(&self.cwd).to_string())
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, mode: PromptEditMode) -> Cow<'_, str> {
        let color = if self.status == 0 { Color::Green } else { Color::Red };
        let mark = match mode {
            PromptEditMode::Vi(reedline::PromptViMode::Normal) => " ❮ ",
            _ => " ❯ ",
        };
        Cow::Owned(color.paint(mark).to_string())
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        // (As wide as the first line's prompt, a dot under its ❯, so each
        // line's text starts in the same column and its indentation shows.)
        let pad = unicode_width::UnicodeWidthStr::width(self.cwd.as_str());
        Cow::Owned(format!("{}{}", " ".repeat(pad), Style::new().fg(Color::DarkGray).paint(" · ")))
    }
    fn render_prompt_history_search_indicator(&self, search: PromptHistorySearch) -> Cow<'_, str> {
        let prefix = match search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!("({prefix}history: {}) ", search.term))
    }
}

/// The folder for the prompt: ~ for home.
fn short_cwd() -> String {
    let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    match std::env::var("HOME") {
        Ok(h) if !h.is_empty() && (cwd == h || cwd.starts_with(&format!("{h}/"))) => format!("~{}", &cwd[h.len()..]),
        _ => cwd,
    }
}

// ---- history ----------------------------------------------------------------------

/// The history file, in reedline's format; one in rustyline's (an older
/// tessh) is converted.
fn history_file(data: &std::path::Path) -> std::path::PathBuf {
    let f = data.join("history");
    if let Ok(t) = std::fs::read_to_string(&f) {
        if let Some(rest) = t.strip_prefix("#V2\n") {
            let mut out = String::new();
            for l in rest.lines() {
                let mut s = String::new();
                let mut it = l.chars();
                while let Some(c) = it.next() {
                    if c == '\\' {
                        match it.next() {
                            Some('n') => s.push('\n'),
                            Some(o) => s.push(o),
                            None => {}
                        }
                    } else {
                        s.push(c);
                    }
                }
                out.push_str(&s.replace('\n', "<\\n>"));
                out.push('\n');
            }
            let _ = std::fs::write(&f, out);
        }
    }
    f
}

fn read_history(f: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(f).unwrap_or_default().lines().filter(|l| !l.contains("<\\n>") && !l.trim().is_empty()).map(String::from).collect()
}

/// The keys, for F1 on an empty line and `help keys`.
pub fn keys_help() -> String {
    let mut s = String::from("# Keys\n\nAs Tessera's editor, with these for the keys a terminal can't send:\n\n| In Tessera | Does | Here | With the kitty keyboard protocol |\n|---|---|---|---|\n");
    for (k, what, alt, kitty) in KEYS {
        s.push_str(&format!("| {k} | {what} | {alt} | {kitty} |\n"));
    }
    s.push_str(
        "\nThe rest:\n\n\
         - **Enter** runs what's complete. In a block (def, if, for, …) or open brackets it starts a new line, indented; \
         Enter on a blank line ends the block and runs it.\n\
         - **Tab** completes (each name's help beside it); in a line's indentation it indents. **Shift+Tab** outdents.\n\
         - **↑ ↓** go line by line, then through history; **PgUp PgDn** a whole entry at a time. **Ctrl+R** searches history.\n\
         - **→** or **End** at the end takes the grey suggestion (history first, then the AI); **Alt+F** one word of it.\n\
         - **# what you want**, then Enter: the AI writes the command for you to check.\n\
         - **F1** or **Alt+H**: help for the word at the cursor. **Ctrl+Alt+J**: background jobs. **Ctrl+Alt+D**: folders (cd history). **Ctrl+Alt+P**: the prompt bar's choices. **Ctrl+Alt+S**: indentation. **Ctrl+Alt+X**: ask the AI to fix the command that failed. **Shift+F10** or the Menu key: the text's menu (Copy, Paste, …).\n\
         - Emacs keys (Ctrl+A, Ctrl+E, Ctrl+K, Ctrl+W, Ctrl+Y, Alt+B, Alt+F, …); `TESSH_KEYS=vi` for vi's.\n",
    );
    s
}

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

/// A completion: what goes in, what the list shows, its kind ("command",
/// "value", "dir", "file", "variable", "option", "subcommand"), a word on
/// what it is, and its help.
pub struct Item {
    pub insert: String,
    pub label: String,
    pub kind: &'static str,
    pub detail: String,
    pub doc: String,
}

/// Completions for Tessera's input editor (`tessh --complete`): text and a
/// UTF-16 cursor in, {"start", "end" (UTF-16), "items": [{insert, label,
/// kind, detail, doc}]} out, as Tessera's shell completer answers.
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
    let (start, end, items) = complete_items(py, ns, text, pos);
    let u16_of = |b: usize| text[..b].encode_utf16().count();
    let items: Vec<serde_json::Value> = items.into_iter().map(|i| serde_json::json!({"insert": i.insert, "label": i.label, "kind": i.kind, "detail": i.detail, "doc": i.doc})).collect();
    serde_json::json!({"start": u16_of(start), "end": u16_of(end), "items": items})
}

/// Completions at byte offset `pos`: the byte range they replace, and them.
pub fn complete_items(py: Python<'_>, ns: &Bound<'_, PyDict>, text: &str, pos: usize) -> (usize, usize, Vec<Item>) {
    let pos = pos.min(text.len());
    let before = &text[..pos];
    let start = before.rfind(|c: char| " \t\n()[]{}\"'`;$".contains(c)).map(|i| i + 1).unwrap_or(0);
    let word = &before[start..];
    let head = before[..start].trim_end();
    let first = head.is_empty() || head.ends_with(['|', ';', '(', '\n']) || head.ends_with("&&") || head.ends_with("||") || head.ends_with("&&?") || head.ends_with("||?");
    let var = before[..start].ends_with('$');
    let mut items: Vec<Item> = vec![];
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
                    items.push(Item { insert: format!("{root}.{n}"), label: n, kind: if callable { "command" } else { "value" }, detail, doc });
                }
            }
        }
        return (start, pos, items);
    }
    // (A command `complete` was told about: its words, function, files…)
    if !first && !var {
        let own = py.import("__tessh_shell__").and_then(|m| m.getattr("complete_for")).and_then(|f| f.call1((&before[..start], word, ns)));
        if let Ok(list) = own {
            if let Ok(names) = list.extract::<Vec<String>>() {
                for n in names {
                    let dir = n.ends_with('/');
                    items.push(Item { insert: n.clone(), label: n, kind: if dir { "dir" } else { "option" }, detail: "complete".into(), doc: String::new() });
                }
                return (start, pos, items);
            }
        }
        // (A program bash-completion knows (git, systemctl, nix…): what bash would offer.)
        let bash = py.import("__tessh_shell__").and_then(|m| m.getattr("bash_complete")).and_then(|f| f.call1((&before[..start], word, ns)));
        if let Ok(list) = bash {
            if let Ok(names) = list.extract::<Vec<String>>() {
                for n in names {
                    let dir = n.ends_with('/');
                    items.push(Item { insert: n.clone(), label: n, kind: if dir { "dir" } else { "option" }, detail: "bash".into(), doc: String::new() });
                }
                return (start, pos, items);
            }
        }
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
        items.push(Item { insert, label, kind, detail, doc });
    }
    (start, pos, items)
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

// ---- the loop ---------------------------------------------------------------------

/// The word at (or just before) a byte offset.
fn word_at(text: &str, pos: usize) -> &str {
    let pos = pos.min(text.len());
    let stop = |c: char| " \t\n()[]{}\"'`;$|&<>=,".contains(c);
    let start = text[..pos].rfind(stop).map(|i| i + 1).unwrap_or(0);
    let end = text[pos..].find(stop).map(|i| pos + i).unwrap_or(text.len());
    &text[start..end]
}

fn print_md(md: &str) {
    let color = unsafe { libc::isatty(1) } == 1 && std::env::var_os("NO_COLOR").is_none();
    println!("{}", crate::help::terminal(md, color).trim_end());
}

pub fn run() -> i32 {
    unsafe {
        #[cfg(target_os = "linux")]
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
    let data = std::env::var("XDG_DATA_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share")).join("tessh");
    let _ = std::fs::create_dir_all(&data);
    let hist = history_file(&data);
    let shared: SharedRef = Arc::new(Mutex::new(Shared { history: read_history(&hist), ..Default::default() }));
    // (Whether an AI is set up: found in the background, so the prompt
    // doesn't wait on a look for a local model server.)
    {
        let shared = shared.clone();
        std::thread::spawn(move || {
            let inline = crate::ai::ready("inline");
            let command = crate::ai::ready("command");
            let mut sh = shared.lock().unwrap();
            sh.ai.inline_ready = inline;
            sh.ai.command_ready = command;
        });
    }
    let history = match FileBackedHistory::with_file(10000, hist.clone()) {
        Ok(h) => Box::new(h) as Box<dyn History>,
        Err(e) => {
            eprintln!("tessh: history: {e}");
            Box::new(FileBackedHistory::new(10000).expect("history"))
        }
    };
    let completion_menu = IdeMenu::default()
        .with_name("completion")
        .with_marker(" ❯ ")
        .with_default_border()
        .with_description_mode(DescriptionMode::PreferRight)
        .with_min_description_width(30)
        .with_max_description_width(72)
        .with_max_description_height(16)
        .with_max_completion_height(12)
        .with_description_text_style(Style::new().fg(Color::LightGray))
        .with_selected_text_style(Style::new().reverse());
    let history_menu = ColumnarMenu::default().with_name("history_menu").with_marker(" ❯ ");
    let mut rl = Reedline::create()
        .with_history(history)
        .with_history_exclusion_prefix(Some(" ".into()))
        .with_highlighter(Box::new(TesshHighlighter))
        .with_hinter(Box::new(TesshHinter { shared: shared.clone() }))
        .with_completer(Box::new(TesshCompleter { ns: Python::attach(|py| ns.clone_ref(py)) }))
        .with_validator(Box::new(TesshValidator { shared: shared.clone() }))
        .with_edit_hook(Box::new(TesshHook { shared: shared.clone() }))
        .with_status_line(Box::new(TesshStatus { shared: shared.clone(), cache: Default::default() }))
        .with_menu(ReedlineMenu::EngineCompleter(Box::new(completion_menu)))
        .with_menu(ReedlineMenu::HistoryMenu(Box::new(history_menu)))
        .with_edit_mode(edit_mode())
        .with_partial_completions(true)
        .with_quick_completions(false)
        .use_bracketed_paste(true)
        .with_semantic_markers(Some(reedline::Osc133Markers::boxed()));
    let repaint = rl.repaint_signal();
    {
        let shared = shared.clone();
        rl = rl.with_poll_interval(Duration::from_millis(50)).with_idle_callback(Box::new(move || ai_tick(&shared, &repaint)));
    }
    // (The kitty keyboard protocol, where the terminal has it: lone taps of
    // Shift and Ctrl, Shift+Enter, Ctrl+Enter. TESSH_KITTY_KEYS=0: not.)
    let tty = unsafe { libc::isatty(0) == 1 && libc::isatty(1) == 1 };
    let kitty = tty && std::env::var("TESSH_KITTY_KEYS").map(|v| v != "0").unwrap_or(true) && reedline::kitty_protocol_available();
    let config_dir = std::env::var("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"));
    let rc = config_dir.join("tessh/rc.tsh");
    Python::attach(|py| crate::run_login_file(py, ns.bind(py)));
    if rc.is_file() {
        Python::attach(|py| {
            if let Err(e) = runner::run_file(py, &rc.to_string_lossy(), ns.bind(py)) {
                runner::report(py, e, &rc.to_string_lossy());
            }
        });
    }
    // (tessh's own editor; TESSH_EDITOR=reedline: the old one, for now.)
    if tty && std::env::var("TESSH_EDITOR").as_deref() != Ok("reedline") {
        return term::run_term(ns, hist, None);
    }
    let host = hostname();
    let mut code = 0;
    // (What a host command leaves to come back to: the text and the cursor.)
    let mut keep: Option<(String, usize)> = None;
    loop {
        Python::attach(|py| {
            apply_handover(py);
            // (prompt_hooks, direnv's among them: before the environment's written out.)
            runtime::run_prompt_hooks(py);
            shared.lock().unwrap().macros = runner::macro_names(ns.bind(py));
        });
        write_env();
        let cwd = std::env::current_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        osc(&format!("7;file://{host}{cwd}"));
        let prompt = TesshPrompt { cwd: short_cwd(), status: runtime::STATUS.load(Ordering::SeqCst) };
        if let Some((text, pos)) = keep.take() {
            rl.run_edit_commands(&[EditCommand::Clear, EditCommand::InsertString(text), EditCommand::MoveToPosition { position: pos, select: false }]);
        }
        shared.lock().unwrap().ai.note.clear();
        if kitty {
            let _ = crossterm::execute!(std::io::stdout(), crossterm::event::PushKeyboardEnhancementFlags(kitty_flags()));
        }
        let sig = rl.read_line(&prompt);
        if kitty {
            let _ = crossterm::execute!(std::io::stdout(), crossterm::event::PopKeyboardEnhancementFlags);
        }
        let text = match sig {
            Ok(Signal::Success(t)) => t,
            Ok(Signal::CtrlC(_)) | Ok(Signal::ExternalBreak(_)) => continue,
            Ok(Signal::CtrlD) => {
                if Python::attach(|py| crate::jobs::exit_check(py).unwrap_or(true)) {
                    break;
                }
                continue;
            }
            Ok(Signal::HostCommand(c)) => {
                let buf = rl.current_buffer_contents().to_string();
                let pos = rl.current_insertion_point();
                match c.as_str() {
                    "tessh:ai" => {
                        let req = buf.trim_start_matches('#').trim().to_string();
                        let recent_cmds = recent(&shared.lock().unwrap(), 10);
                        eprint!("{}", Style::new().fg(Color::DarkGray).paint("  asking the AI…"));
                        let _ = std::io::stderr().flush();
                        let r = crate::ai::ask("command", &req, &recent_cmds);
                        eprint!("\r\x1b[K");
                        match r {
                            Ok(cmd) if !cmd.is_empty() => {
                                let n = cmd.len();
                                keep = Some((cmd, n));
                            }
                            Ok(_) => {
                                eprintln!("tessh: the AI had no command for that");
                                keep = Some((buf, pos));
                            }
                            Err(e) => {
                                eprintln!("tessh: AI: {e}");
                                keep = Some((buf, pos));
                            }
                        }
                    }
                    "tessh:help" => {
                        let w = word_at(&buf, pos);
                        println!();
                        if w.is_empty() {
                            print_md(&keys_help());
                        } else {
                            let doc = Python::attach(|py| {
                                let v = ns.bind(py).get_item(w).ok().flatten().or_else(|| py.import("builtins").ok().and_then(|b| b.getattr(w).ok()));
                                doc_for(py, w, v.as_ref())
                            });
                            if doc.trim().is_empty() {
                                println!("tessh: no help for {w}");
                            } else if doc.starts_with('#') || doc.contains("\n## ") || doc.contains("\n# ") {
                                // (The guide's part about it: Markdown.)
                                print_md(&doc);
                            } else {
                                // (A signature and docstring: as written, the first line bold.)
                                let (first, rest) = doc.split_once('\n').unwrap_or((&doc, ""));
                                println!("{}\n{}", Style::new().bold().paint(first), rest.trim_end());
                            }
                        }
                        keep = Some((buf, pos));
                    }
                    "tessh:jobs" => {
                        Python::attach(|py| {
                            if let Err(e) = runner::run_source(py, "jobs", ns.bind(py), "<keys>", true) {
                                runner::report(py, e, "<keys>");
                            }
                        });
                        keep = Some((buf, pos));
                    }
                    _ => keep = Some((buf, pos)),
                }
                continue;
            }
            Ok(_) => continue,
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
        if !text.contains('\n') && !text.starts_with(' ') {
            let mut sh = shared.lock().unwrap();
            sh.history.retain(|h| *h != text);
            sh.history.push(text.clone());
        }
        let _ = rl.sync_history();
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;
        // (What it runs with: user, line, environment, NUL-separated.)
        let exec = format!("{}\0{}\0{}", crate::runtime::user_name(), text, crate::runtime::env_list().join("\0"));
        osc(&format!("1337;TesseraExec={}", b64.encode(&exec)));
        osc(&format!("1337;TesseraCommand={}", b64.encode(&text)));
        osc("133;C");
        let exit = Python::attach(|py| -> Option<i32> {
            runtime::begin_line();
            let r = runner::run_source(py, &text, ns.bind(py), "<stdin>", true);
            let flush = || {
                if let Ok(sys) = py.import("sys") {
                    let _ = sys.getattr("stdout").and_then(|o| o.call_method0("flush"));
                    let _ = sys.getattr("stderr").and_then(|o| o.call_method0("flush"));
                }
            };
            match r {
                Ok(()) => runtime::end_line(),
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
    let _ = rl.sync_history();
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
        assert!(h.contains("ls"), "{h:?}");
        assert!(h.contains("\x1b["), "{h:?}");
        assert_eq!(strip(&h), "ls -la $x \"s\" # c");
    }

    fn strip(s: &str) -> String {
        let mut out = String::new();
        let mut esc = false;
        for c in s.chars() {
            if esc {
                if c == 'm' {
                    esc = false;
                }
            } else if c == '\x1b' {
                esc = true;
            } else {
                out.push(c);
            }
        }
        out
    }

    fn ctx(buffer: &str) -> EditHookContext<'_> {
        EditHookContext { buffer, cursor: buffer.len(), menu_active: false }
    }

    #[test]
    fn enter_runs_or_goes_on() {
        let mut h = TesshHook { shared: Default::default() };
        assert_eq!(h.edit(&ReedlineEvent::Enter, &ctx("ls -la")), Some(ReedlineEvent::Submit));
        assert_eq!(
            h.edit(&ReedlineEvent::Enter, &ctx("def f x")),
            Some(ReedlineEvent::Edit(vec![EditCommand::MoveToEnd { select: false }, EditCommand::InsertNewline, EditCommand::InsertString("  ".into())]))
        );
        // (A blank line ends the block: double Enter.)
        assert_eq!(h.edit(&ReedlineEvent::Enter, &ctx("def f x\n  print hi\n  ")), Some(ReedlineEvent::Submit));
        // (else goes back to its if, and opens a block of its own.)
        assert_eq!(h.edit(&ReedlineEvent::Enter, &ctx("if $x\n  a\n  else")), Some(ReedlineEvent::Edit(vec![EditCommand::Clear, EditCommand::InsertString("if $x\n  a\nelse\n  ".into())])));
        // (In a menu, Enter is the menu's.)
        assert_eq!(h.edit(&ReedlineEvent::Enter, &EditHookContext { buffer: "ls", cursor: 2, menu_active: true }), None);
        // (Backspace in indentation takes a level.)
        assert_eq!(h.edit(&ReedlineEvent::Edit(vec![EditCommand::Backspace]), &ctx("if $x\n    ")), Some(ReedlineEvent::Edit(vec![EditCommand::Backspace, EditCommand::Backspace])));
        assert_eq!(h.edit(&ReedlineEvent::Edit(vec![EditCommand::Backspace]), &ctx("ls x")), None);
    }

    #[test]
    fn words() {
        assert_eq!(word_at("print $x.y", 10), "x.y");
        assert_eq!(word_at("ls -la", 1), "ls");
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
