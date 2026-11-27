//! The editor's state and what keys, the mouse and time do to it: the same
//! order of handling as ComposeBox's Keys.onPressed (suggestion, Escape, F1,
//! the completion list, Tab, Ctrl+Space, Ctrl+R, the keys every mode has,
//! VS Code's smart keys, then Emacs, vi or VS Code), its timers (the list
//! opening once typing pauses, the syntax check, the AI), and its popups.

use super::text::{detect_indent, Indent, IndentMode, Text};
use super::Hit;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, ModifierKeyCode, MouseButton, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Keys {
    Emacs,
    Vi,
    Vscode,
}

impl Keys {
    pub fn parse(s: &str) -> Keys {
        match s {
            "vi" | "vim" => Keys::Vi,
            "vscode" => Keys::Vscode,
            _ => Keys::Emacs,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Keys::Emacs => "emacs",
            Keys::Vi => "vi",
            Keys::Vscode => "vscode",
        }
    }
    pub fn next(self) -> Keys {
        match self {
            Keys::Emacs => Keys::Vi,
            Keys::Vi => Keys::Vscode,
            Keys::Vscode => Keys::Emacs,
        }
    }
}

/// A completion item, as tessh's completer gives them.
#[derive(Clone, Debug, Default)]
pub struct Item {
    pub label: String,
    pub insert: String,
    pub kind: String,
    pub detail: String,
    pub doc: String,
}

/// What the editor asks of tessh.
pub trait Hooks {
    /// Highlight kinds: (start, end, kind) byte spans.
    fn highlight(&mut self, text: &str) -> Vec<(usize, usize, &'static str)>;
    /// Completions at a position: (start, end, items).
    fn complete(&mut self, text: &str, pos: usize) -> (usize, usize, Vec<Item>);
    /// Syntax errors: (start, len, message) byte ranges.
    fn check(&mut self, text: &str) -> Vec<(usize, usize, String)>;
    /// Help for the name at a position (F1, pointing at it).
    fn hover(&mut self, text: &str, pos: usize) -> String;
    /// Macro names (blocks).
    fn macros(&mut self) -> Vec<String>;
}

/// What a read ends with.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Submit { text: String, background: bool },
    /// Ctrl+C on an empty line.
    Interrupt,
    /// Ctrl+D on an empty line.
    Eof,
    /// Something for tessh to do with the text kept (help printed, a
    /// widget's command run, the guide, jobs, clear the screen, Fix).
    Host(String),
}

#[derive(Default)]
pub struct Comp {
    pub open: bool,
    pub start: usize,
    pub end: usize,
    pub items: Vec<Item>,
    pub index: usize,
    /// Up/Down used: Enter takes the item.
    pub chosen: bool,
    /// Keys coming: the list waits for a pause.
    pub typing: bool,
    /// History search (Ctrl+R, the History button).
    pub hist: bool,
    pub query: Text,
    pub doc_scroll: usize,
    pub top: usize,
    /// The text and cursor the items were worked out for: a list for other
    /// text is never taken (worked out again first). "cd " lists at once,
    /// and typed on quickly, a Tab took that list's first folder.
    pub for_text: String,
    pub for_cursor: usize,
}

/// A list from a button or widget: the folders, a widget's choices, the
/// indent menu.
pub struct Menu {
    pub kind: MenuKind,
    pub items: Vec<MenuItem>,
    pub sel: usize,
    pub filter: Text,
    pub top: usize,
}

#[derive(Clone, PartialEq, Debug)]
pub enum MenuKind {
    Choices(String),
    Indent,
    /// The text's right-click menu.
    Context,
    /// The prompt bar's widgets with choices (Ctrl+Alt+P).
    Widgets,
    /// The ls listing's menus ("item", "folder", "columns", "sort", …).
    Ls(String),
    /// The ▾ at the prompt bar's left (Ctrl+Alt+O): folders, history, the
    /// guide, indentation, the editor's keys; each with its shortcut.
    Options,
}

#[derive(Clone, Debug, Default)]
pub struct MenuItem {
    pub label: String,
    pub cmd: String,
    pub section: bool,
    pub checked: bool,
    pub enabled: bool,
    /// Its shortcut, shown at the right.
    pub key: String,
}

pub struct Ai {
    pub inline_ready: bool,
    pub command_ready: bool,
    /// Asks waiting for their pause: (task, text, since).
    pub want: Vec<(&'static str, String, Instant)>,
    pub asking: Vec<(&'static str, String)>,
    pub answers: Vec<((&'static str, String), Result<String, String>)>,
    /// "command" or "fix" while the AI writes one (Escape cancels).
    pub busy: String,
}

pub struct Ed {
    pub t: Text,
    pub keys: Keys,
    pub indent_pref: Indent,
    pub indent_default: Indent,
    pub indent_own: bool,
    pub detect: bool,
    pub indent_picked: bool,
    pub line_numbers: bool,
    pub indent_guides: bool,
    pub history: Vec<String>,
    pub hist_index: Option<usize>,
    pub draft: String,
    pub from_history: bool,
    pub comp: Comp,
    pub menu: Option<Menu>,
    pub ghost: String,
    pub ghost_base: String,
    pub nl_command: String,
    pub errors: Vec<(usize, usize, String)>,
    pub problem: String,
    pub checked: String,
    pub hover: Option<(usize, String)>,
    pub tooltip: Option<(u16, u16, String)>,
    pub ai_note: String,
    pub ai: std::sync::Arc<std::sync::Mutex<Ai>>,
    pub status: i32,
    pub duration_ms: u64,
    pub cwd: String,
    pub visited: Vec<String>,
    pub jobs: (usize, usize),
    // timers
    pub edited: Option<Instant>,
    pub analyzed: bool,
    pub pointer: Option<(u16, u16, Instant)>,
    // mouse
    pub drag: Option<(usize, u8)>,
    pub last_click: Option<(Instant, u16, u16, u8)>,
    // taps
    pub shift_side: u8,
    pub shift_at: Instant,
    pub ctrl_side: u8,
    pub ctrl_at: Instant,
    pub kitty: bool,
    pub macros: Vec<String>,
    pub done: Option<Outcome>,
    pub dirty: bool,
    /// What the mouse was over when it went down (a click acts on release
    /// only where it went down, as buttons do).
    pub pressed: Option<Hit>,
    /// A popup list's scrollbar thumb being dragged: the row it was taken
    /// at, the list's top then, its length and rows shown.
    pub bar_drag: Option<(u16, usize, usize, usize)>,
    /// The rows the open popup list shows (the view sets it as it draws).
    pub list_room: std::cell::Cell<usize>,
    /// The row at its top as last drawn (what the wheel and arrows go from).
    pub list_top: std::cell::Cell<usize>,
    /// Where a button's list opens (the column clicked).
    pub menu_x: Option<usize>,
    /// A tap waiting out its moment ('n' new line, 'g' suggestion, 'b'
    /// background): a key right after it cancels it (keyd sends a Ctrl tap
    /// just before a tapped Enter).
    pub pending_tap: Option<(char, Instant)>,
    /// No more keys waiting (set by the loop before each tick): the list
    /// is worked out again at once, not after a pause.
    pub input_idle: bool,
    /// vi's modes, the : commands and their line (ViKeys, ExCommands, ExLine).
    pub vi: super::vi::Vi,
    pub ex: super::ex::Ex,
    pub cmd: super::vi::CmdLine,
    /// The last ls's listing, above the prompt until the next command
    /// (Tessera's folder view: editor/ls.rs), and whether it has the keys.
    pub ls: Option<Box<super::ls::Ls>>,
    pub ls_focus: bool,
    /// The shared history read again in the background (other windows and
    /// sessions' commands): taken in when it comes.
    pub history_in: std::sync::Arc<std::sync::Mutex<Option<Vec<String>>>>,
}

const CONT: &[&str] = &["else", "elif", "except", "except*", "finally"];

fn openers_of(h: &str) -> &'static [&'static str] {
    match h {
        "if" | "elif" => &["elif", "else"],
        "for" | "while" => &["else"],
        "try" => &["except", "finally"],
        "except" => &["except", "else", "finally"],
        "else" => &["finally"],
        _ => &[],
    }
}

pub fn tessh_head(line: &str) -> &str {
    let w: Vec<&str> = line.split_whitespace().collect();
    let mut h = w.first().copied().unwrap_or("");
    if w.len() > 2 && ["=", "+=", "-="].contains(&w[1]) {
        h = w[2];
    }
    if h == "async" && w.len() > 1 {
        h = w[1];
    }
    h
}

/// Does a line take an indented block (ComposeBox.tesshOpens, with macros).
pub fn tessh_opens(line: &str, macros: &[String]) -> bool {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        return false;
    }
    if t.ends_with(['(', '[', '{']) {
        return true;
    }
    crate::repl::opens_block(t, macros)
}

impl Ed {
    pub fn new() -> Ed {
        let ind = Indent { mode: IndentMode::Spaces, size: 2 };
        Ed {
            t: Text::new(),
            keys: Keys::Emacs,
            indent_pref: ind.clone(),
            indent_default: ind,
            indent_own: false,
            detect: true,
            indent_picked: false,
            line_numbers: true,
            indent_guides: true,
            history: vec![],
            hist_index: None,
            draft: String::new(),
            from_history: false,
            comp: Comp::default(),
            menu: None,
            ghost: String::new(),
            ghost_base: String::new(),
            nl_command: String::new(),
            errors: vec![],
            problem: String::new(),
            checked: String::new(),
            hover: None,
            tooltip: None,
            ai_note: String::new(),
            ai: std::sync::Arc::new(std::sync::Mutex::new(Ai { inline_ready: false, command_ready: false, want: vec![], asking: vec![], answers: vec![], busy: String::new() })),
            status: 0,
            duration_ms: 0,
            cwd: String::new(),
            visited: vec![],
            jobs: (0, 0),
            edited: None,
            analyzed: true,
            pointer: None,
            drag: None,
            last_click: None,
            shift_side: 0,
            shift_at: Instant::now(),
            ctrl_side: 0,
            ctrl_at: Instant::now(),
            kitty: false,
            macros: vec![],
            done: None,
            dirty: true,
            pressed: None,
            bar_drag: None,
            list_room: std::cell::Cell::new(14),
            list_top: std::cell::Cell::new(0),
            menu_x: None,
            pending_tap: None,
            input_idle: true,
            vi: {
                let mut v = super::vi::Vi::new();
                v.clip_set = Some(Box::new(|s: &str| super::term::Term::copy(s)));
                v
            },
            ex: super::ex::Ex::new(),
            cmd: super::vi::CmdLine::new(),
            ls: None,
            ls_focus: false,
            history_in: Default::default(),
        }
    }

    // ---- indentation ------------------------------------------------------------------
    pub fn detected(&self) -> Option<Indent> {
        if !self.detect || self.indent_picked {
            return None;
        }
        detect_indent(&self.t.text, self.indent_pref.size)
    }
    pub fn indent(&self) -> Indent {
        self.detected().unwrap_or_else(|| self.indent_pref.clone())
    }

    // ---- text changes -----------------------------------------------------------------
    /// After any edit: the timers start over, as ComposeBox's onTextChanged.
    pub fn changed(&mut self) {
        self.edited = Some(Instant::now());
        self.analyzed = false;
        self.problem.clear();
        self.errors.clear();
        self.hover = None;
        if !self.comp.hist {
            self.comp.typing = true;
        }
        if self.t.is_empty() {
            self.indent_picked = false;
        }
        let text = self.t.text.clone();
        if text.trim() != self.checked {
            self.ai_note.clear();
        }
        if text.starts_with('#') && !text.contains('\n') && text.trim_start_matches('#').trim().len() > 0 {
            self.nl_command.clear();
            let full = format!("{}{}", self.ghost_base, self.ghost);
            if !self.ghost.is_empty() && !self.ghost.starts_with("  →") && full.starts_with(&text) && full.len() > text.len() {
                self.ghost = full[text.len()..].to_string();
            } else {
                self.ghost.clear();
            }
            self.ghost_base = text.clone();
            self.want("request", &text);
            self.want("command", &text);
            return;
        }
        self.nl_command.clear();
        self.refresh_ghost();
    }

    pub fn set_text(&mut self, s: &str) {
        self.t.set(s);
        self.changed();
    }

    /// The suggestion: kept while what's typed leads into it, else the newest
    /// history entry that starts with it, else the AI is asked.
    pub fn refresh_ghost(&mut self) {
        let before = &self.t.text[..self.t.cursor];
        if before.rfind('\n').is_some_and(|i| before[i + 1..].trim().is_empty()) {
            self.ghost.clear();
            self.ghost_base.clear();
            return;
        }
        let t = self.suggest_base();
        let full = format!("{}{}", self.ghost_base, self.ghost);
        if !self.ghost.is_empty() && t.len() > self.ghost_base.len() && full.starts_with(&t) && full.len() > t.len() {
            self.ghost = full[t.len()..].to_string();
            self.ghost_base = t;
        } else {
            self.ghost = self.history_ghost(&t);
            self.ghost_base = t.clone();
            if self.ghost.is_empty() {
                self.want("inline", &t);
            }
        }
    }
    /// What follows the cursor when it's only closers typed in pairs.
    pub fn closers(&self) -> &str {
        let rest = &self.t.text[self.t.cursor..];
        if !rest.is_empty() && rest.chars().all(|c| ")]}\"'`".contains(c)) {
            rest
        } else {
            ""
        }
    }
    pub fn at_suggest_end(&self) -> bool {
        self.t.cursor == self.t.len() || !self.closers().is_empty()
    }
    pub fn suggest_base(&self) -> String {
        if self.closers().is_empty() {
            self.t.text.clone()
        } else {
            self.t.text[..self.t.cursor].to_string()
        }
    }
    fn history_ghost(&self, t: &str) -> String {
        if t.trim().chars().count() < 2 || t.contains('\n') || !self.at_suggest_end() {
            return String::new();
        }
        self.history.iter().rev().find(|h| h.len() > t.len() && h.starts_with(t) && !h.contains('\n')).map(|h| h[t.len()..].to_string()).unwrap_or_default()
    }
    pub fn ghost_takes_closers(&self) -> bool {
        let c = self.closers();
        !c.is_empty() && self.ghost.contains(c)
    }
    pub fn accept_ghost(&mut self) -> bool {
        if !self.nl_command.is_empty() && self.t.text.starts_with('#') {
            let c = std::mem::take(&mut self.nl_command);
            self.ghost.clear();
            self.set_text(&c);
            return true;
        }
        if self.ghost.is_empty() || self.ghost.starts_with("  →") {
            return false;
        }
        let g = std::mem::take(&mut self.ghost);
        let at = self.t.cursor;
        if !self.closers().is_empty() {
            let n = self.closers().len();
            if g.contains(&self.t.text[at..]) {
                self.t.replace(at, at + n, &g);
            } else {
                self.t.replace(at, at, &g);
            }
        } else {
            let n = self.t.len();
            self.t.replace(n, n, &g);
        }
        self.t.cursor = self.t.len();
        self.changed();
        self.ghost.clear();
        true
    }
    /// Ask the AI for a suggestion now (a left Shift tap, Ctrl+Space with
    /// nothing to take).
    pub fn ask_ghost_now(&mut self) {
        let t = self.suggest_base();
        let mut ai = self.ai.lock().unwrap();
        ai.answers.retain(|((k, q), _)| !(*k == "inline" && *q == t));
        ai.want.retain(|w| w.0 != "inline");
        ai.want.push(("inline", t, Instant::now() - Duration::from_secs(5)));
    }
    fn want(&mut self, task: &'static str, text: &str) {
        let mut ai = self.ai.lock().unwrap();
        let ok = match task {
            "inline" => ai.inline_ready && text.trim().chars().count() >= 2 && !text.starts_with('#') && !text.contains('\n'),
            "request" => ai.inline_ready,
            _ => ai.command_ready,
        };
        ai.want.retain(|w| w.0 != task);
        if ok && !ai.answers.iter().any(|((k, q), _)| *k == task && q == text) {
            ai.want.push((task, text.to_string(), Instant::now()));
        }
    }

    // ---- completion -------------------------------------------------------------------
    pub fn open_comp(&mut self, h: &mut dyn Hooks) {
        self.comp.open = true;
        self.comp.chosen = false;
        self.comp.index = 0;
        self.comp.typing = false;
        self.comp.hist = false;
        self.request_complete(h);
    }
    pub fn close_comp(&mut self) {
        self.comp.open = false;
        self.comp.chosen = false;
        self.comp.hist = false;
        self.comp.doc_scroll = 0;
    }
    pub fn open_history(&mut self) {
        self.comp.hist = true;
        self.comp.open = true;
        self.comp.chosen = true;
        self.comp.index = 0;
        self.comp.top = 0;
        let q = self.t.text.trim().to_string();
        self.comp.query.set(&q);
        self.comp.query.select(0, q.len());
        self.search_history();
    }
    /// History, every word in any order, newest first.
    pub fn search_history(&mut self) {
        let q = self.comp.query.text.to_lowercase();
        let words: Vec<&str> = q.split_whitespace().collect();
        let mut seen = std::collections::HashSet::new();
        self.comp.items = self
            .history
            .iter()
            .rev()
            .filter(|h| seen.insert(h.as_str()))
            .filter(|h| {
                let l = h.to_lowercase();
                words.iter().all(|w| l.contains(w))
            })
            .take(300)
            .map(|h| Item { label: one_line(h), insert: h.clone(), kind: "history".into(), ..Default::default() })
            .collect();
        if self.comp.index >= self.comp.items.len() {
            self.comp.index = 0;
        }
    }
    pub fn request_complete(&mut self, h: &mut dyn Hooks) {
        if self.comp.hist {
            self.search_history();
            return;
        }
        let (s, e, items) = h.complete(&self.t.text, self.t.cursor);
        self.comp.for_text = self.t.text.clone();
        self.comp.for_cursor = self.t.cursor;
        self.comp.start = s;
        self.comp.end = e;
        self.comp.items = items;
        self.comp.doc_scroll = 0;
        if self.comp.index >= self.comp.items.len() {
            self.comp.index = 0;
        }
        if self.comp.items.is_empty() {
            self.close_comp();
        }
    }
    pub fn accept_comp(&mut self, i: Option<usize>, h: &mut dyn Hooks) {
        let Some(it) = self.comp.items.get(i.unwrap_or(self.comp.index)).cloned() else { return };
        if self.comp.hist {
            self.close_comp();
            self.set_text(&it.insert);
            return;
        }
        let (s, e) = (self.comp.start.min(self.t.len()), self.comp.end.min(self.t.len()));
        let mut ins = it.insert.clone();
        // A finished word gets its space; a folder or "--opt=" keeps going.
        let more = it.kind == "dir" || it.kind == "partial" || ins.ends_with('=') || ins.ends_with('/');
        if !more && self.t.char_at(e) != Some(' ') {
            ins.push(' ');
        }
        self.t.replace(s, e, &ins);
        self.changed();
        if more {
            self.comp.index = 0;
            self.request_complete(h);
            self.comp.typing = false;
        } else {
            self.close_comp();
        }
    }
    /// Tab on the list, as in bash: an item picked with Up/Down goes in;
    /// else the one that fits, or as much as all that fit have in common.
    /// Several left with nothing more in common: they stay shown, to type
    /// on or pick from (the first is never just taken).
    pub fn tab_comp(&mut self, h: &mut dyn Hooks) {
        if self.comp.items.is_empty() {
            return;
        }
        if self.comp.chosen || self.comp.hist || self.comp.items.len() == 1 {
            self.accept_comp(None, h);
            return;
        }
        let (s, c) = (self.comp.start.min(self.t.cursor), self.t.cursor);
        let typed = self.t.text.get(s..c).unwrap_or("").to_string();
        let (i, lcp) = match tab_common(&typed, &self.comp.items) {
            TabFit::One(i) => (Some(i), String::new()),
            TabFit::Common(l) => (None, l),
            TabFit::Nothing => return,
        };
        if let Some(i) = i {
            self.accept_comp(Some(i), h);
            return;
        }
        self.t.replace(s, c, &lcp);
        self.changed();
        self.comp.index = 0;
        self.request_complete(h);
        self.comp.typing = false;
    }
    fn comp_key(&mut self, k: &KeyEvent, h: &mut dyn Hooks) -> bool {
        if !self.comp.open || self.comp.items.is_empty() {
            return false;
        }
        if !self.comp.hist && (self.comp.for_text != self.t.text || self.comp.for_cursor != self.t.cursor) {
            self.comp.index = 0;
            self.request_complete(h);
            if !self.comp.open || self.comp.items.is_empty() {
                return false;
            }
            // (Tab on a list not yet seen: one thing fits, in it goes;
            // several, they're shown, the next Tab takes one.)
            if k.code == KeyCode::Tab && !k.modifiers.contains(KeyModifiers::SHIFT) {
                self.comp.typing = false;
                self.tab_comp(h);
                return true;
            }
        }
        let n = self.comp.items.len();
        let shift = k.modifiers.contains(KeyModifiers::SHIFT);
        if shift && matches!(k.code, KeyCode::PageDown | KeyCode::PageUp) {
            let d = if k.code == KeyCode::PageDown { 10 } else { -10isize };
            self.comp.doc_scroll = (self.comp.doc_scroll as isize + d).max(0) as usize;
            return true;
        }
        match k.code {
            KeyCode::Down => {
                self.comp.index = (self.comp.index + 1) % n;
                self.comp.chosen = true;
            }
            KeyCode::Up => {
                self.comp.index = (self.comp.index + n - 1) % n;
                self.comp.chosen = true;
            }
            KeyCode::PageDown => {
                self.comp.index = (self.comp.index + 8).min(n - 1);
                self.comp.chosen = true;
            }
            KeyCode::PageUp => {
                self.comp.index = self.comp.index.saturating_sub(8);
                self.comp.chosen = true;
            }
            KeyCode::Tab if !shift => self.tab_comp(h),
            KeyCode::Esc => self.close_comp(),
            KeyCode::Enter => {
                // (Enter never takes a completion, Tab does: it runs the
                // line as typed. Ctrl+R's history list is a pick, though.)
                let history = self.comp.items.get(self.comp.index).is_some_and(|i| i.kind == "history");
                if history && !shift {
                    self.accept_comp(None, h);
                    return true;
                }
                self.close_comp();
                return false;
            }
            _ => return false,
        }
        self.comp.doc_scroll = 0;
        true
    }

    // ---- history ------------------------------------------------------------------------
    pub fn history_step(&mut self, d: i32) {
        let n = self.history.len();
        if n == 0 {
            return;
        }
        match self.hist_index {
            None => {
                if d > 0 {
                    return;
                }
                self.draft = self.t.text.clone();
                self.hist_index = Some(n - 1);
            }
            Some(i) => {
                let j = i as i64 + d as i64;
                if j >= n as i64 {
                    self.hist_index = None;
                    let d = std::mem::take(&mut self.draft);
                    self.set_text(&d);
                    self.from_history = true;
                    return;
                }
                self.hist_index = Some(j.max(0) as usize);
            }
        }
        let s = self.history[self.hist_index.unwrap()].clone();
        self.set_text(&s);
        // (A line from history: the list stays shut, or it'd take the next Up/Down.)
        self.from_history = true;
        self.close_comp();
    }

    // ---- running ------------------------------------------------------------------------
    pub fn submit(&mut self, background: bool) {
        let t = self.t.text.clone();
        let chat = self.ai.lock().unwrap().command_ready;
        // "# what I want": the AI writes the command, for you to check first.
        if chat && t.starts_with('#') && !t.contains('\n') && t.trim_start_matches('#').trim().len() > 0 {
            self.ask_command(t.trim_start_matches('#').trim().to_string(), "command");
            return;
        }
        self.ghost.clear();
        self.hist_index = None;
        self.draft.clear();
        self.close_comp();
        self.menu = None;
        self.done = Some(Outcome::Submit { text: t, background });
    }
    pub fn ask_command(&mut self, what: String, kind: &str) {
        let mut ai = self.ai.lock().unwrap();
        ai.busy = kind.to_string();
        let task: &'static str = if kind == "fix" { "fix" } else { "command" };
        ai.want.retain(|w| w.0 != task);
        ai.want.push((task, what, Instant::now() - Duration::from_secs(5)));
    }

    /// A new line keeping the indentation, one deeper after an opener;
    /// between a bracket pair the closer moves down.
    pub fn newline(&mut self) {
        self.ghost.clear();
        self.ghost_base.clear();
        self.tessh_outdent();
        let ind = self.indent();
        let p = self.t.cursor;
        let ls = self.t.line_start(p);
        let before = self.t.text[ls..p].trim_end().to_string();
        let cur = self.t.indent_at(p).to_string();
        let opens = tessh_opens(&before, &self.macros);
        let deeper = if opens { ind.indent_for(ind.cols_of(&cur) + ind.size) } else { cur.clone() };
        let next = self.t.char_at(p);
        let pair = |o: char| match o {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            _ => '\0',
        };
        if opens && next.is_some_and(|n| ")]}".contains(n)) && before.chars().last().map(pair) == next {
            self.t.replace(p, p, &format!("\n{deeper}\n{cur}"));
            self.t.cursor = p + 1 + deeper.len();
        } else {
            self.t.replace(p, p, &format!("\n{deeper}"));
        }
        self.changed();
    }
    /// An else (elif, except, finally) first on its line goes back to its if
    /// or try.
    pub fn tessh_outdent(&mut self) {
        let p = self.t.cursor;
        let a = self.t.line_start(p);
        let line = self.t.text[a..self.t.line_end(p)].to_string();
        let lead = line.len() - line.trim_start_matches(' ').len();
        let word = tessh_head(&line);
        if lead == 0 || !CONT.contains(&word) || tessh_head(line.trim()) != line.trim().split_whitespace().next().unwrap_or("") {
            return;
        }
        let above = &self.t.text[..a.saturating_sub(1)];
        let mut want = lead;
        for l in above.split('\n').rev() {
            if l.trim().is_empty() {
                continue;
            }
            let li = l.len() - l.trim_start_matches(' ').len();
            if li > lead {
                continue;
            }
            if openers_of(tessh_head(l)).contains(&word) {
                want = li;
                break;
            }
            if li < lead && li == 0 {
                break;
            }
        }
        if want < lead {
            let keep = self.t.cursor;
            self.t.replace(a, a + lead - want, "");
            self.t.cursor = keep - (lead - want);
        }
    }
    /// Enter: runs what's complete; else a new line (tessh's rules: open
    /// brackets, or a block until a blank line, as IPython does).
    fn enter(&mut self) {
        // (A terminal that reports Shift and Ctrl taps, and Shift+Enter: Enter
        // runs it, as in Tessera; new lines are right Shift's. Else tessh's
        // rule, below, so blocks can be written without those keys.)
        if self.kitty {
            self.submit(false);
            return;
        }
        if self.t.cursor < self.t.text.trim_end_matches(' ').len() {
            self.newline();
            return;
        }
        self.tessh_outdent();
        let lines: Vec<&str> = self.t.text.split('\n').collect();
        if crate::repl::needs_more(&lines, &self.macros) {
            self.t.cursor = self.t.len();
            self.newline();
            return;
        }
        // (A blank last line ending a block goes.)
        let trimmed = self.t.text.trim_end().to_string();
        if trimmed != self.t.text {
            self.t.set(&trimmed);
        }
        self.submit(false);
    }

    // ---- keys ---------------------------------------------------------------------------
    pub fn key(&mut self, k: KeyEvent, h: &mut dyn Hooks) {
        self.dirty = true;
        self.tooltip = None;
        // Lone taps (the kitty protocol reports modifiers alone).
        if let KeyCode::Modifier(m) = k.code {
            self.modifier(m, k.kind, h);
            return;
        }
        if k.kind == KeyEventKind::Release {
            return;
        }
        self.pending_tap = None;
        self.shift_side = 0;
        self.ctrl_side = 0;
        if self.menu.is_some() {
            self.menu_key(&k, h);
            return;
        }
        // (The ls listing above, when it has the keys: Ctrl+Alt+L.)
        if self.ls_key(&k) {
            return;
        }
        if self.comp.hist {
            self.hist_key(&k, h);
            return;
        }
        if self.keys == Keys::Vi && self.cmd.is_open() {
            let ind = self.indent();
            let before = self.t.text.clone();
            let out = self.cmd.key(&k, &mut self.t, &ind, &mut self.vi, &mut self.ex);
            self.vi_out(out);
            if self.t.text != before {
                self.changed();
            }
            return;
        }
        self.cmd.dismiss();
        if !matches!(k.code, KeyCode::Up | KeyCode::Down) {
            self.from_history = false;
        }
        let m = k.modifiers;
        let ctrl = m.contains(KeyModifiers::CONTROL);
        let alt = m.contains(KeyModifiers::ALT);
        let shift = m.contains(KeyModifiers::SHIFT);
        let plain = !(ctrl || alt || shift);
        let before = (self.t.text.clone(), self.t.cursor);
        let done = self.dispatch(&k, ctrl, alt, shift, plain, h);
        if !done {
            if let KeyCode::Char(c) = k.code {
                if !ctrl && !alt && !self.vi_normal() {
                    self.type_char(c);
                }
            }
        }
        if self.t.text != before.0 {
            self.changed();
            // (Typing a closing word first on a line moves it back out.)
            if let KeyCode::Char(_) = k.code {
                let p = self.t.cursor;
                let line = &self.t.text[self.t.line_start(p)..p];
                if line.starts_with(' ') && CONT.contains(&line.trim()) {
                    self.tessh_outdent();
                }
            }
        } else if self.t.cursor != before.1 {
            self.t.prune_pairs();
            self.refresh_ghost();
        }
    }

    fn dispatch(&mut self, k: &KeyEvent, ctrl: bool, alt: bool, shift: bool, plain: bool, h: &mut dyn Hooks) -> bool {
        // (Ctrl+Alt: the editor's own.)
        if ctrl && alt {
            match k.code {
                KeyCode::Char('e') | KeyCode::Char('E') => {
                    self.keys = self.keys.next();
                    self.done = Some(Outcome::Host(format!("keys:{}", self.keys.name())));
                    return true;
                }
                KeyCode::Char('j') | KeyCode::Char('J') => {
                    self.done = Some(Outcome::Host("jobs".into()));
                    return true;
                }
                KeyCode::Char('i') | KeyCode::Char('I') => return true,
                // (The options menu: the ▾ at the prompt bar's left.)
                KeyCode::Char('o') | KeyCode::Char('O') => {
                    self.close_comp();
                    self.toggle_options();
                    return true;
                }
                // (The buttons and the prompt bar, without the mouse.)
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    self.click(Hit::Chip("indent"), h);
                    return true;
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    self.click(Hit::Chip("fix"), h);
                    return true;
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    let open = matches!(&self.menu, Some(m) if m.kind == MenuKind::Widgets);
                    self.menu = None;
                    self.close_comp();
                    if !open {
                        self.done = Some(Outcome::Host("widgets".into()));
                    }
                    return true;
                }
                // (The cd history, as the Folders button: again closes it.)
                KeyCode::Char('d') | KeyCode::Char('D') => {
                    let open = matches!(&self.menu, Some(m) if m.kind == MenuKind::Choices("dir".into()));
                    self.menu = None;
                    self.close_comp();
                    if !open {
                        self.done = Some(Outcome::Host("chip:folders".into()));
                    }
                    return true;
                }
                _ => {}
            }
        }
        if !self.ghost.is_empty() && self.at_suggest_end() && plain && matches!(k.code, KeyCode::Right | KeyCode::End) && self.accept_ghost() {
            return true;
        }
        if k.code == KeyCode::Esc {
            if !self.ghost.is_empty() {
                self.ghost.clear();
                return true;
            }
            let mut ai = self.ai.lock().unwrap();
            if !ai.busy.is_empty() {
                ai.busy.clear();
                return true;
            }
            drop(ai);
            if self.hover.is_some() {
                self.hover = None;
                return true;
            }
        }
        // (The right-click menu: Shift+F10 or the Menu key, at the cursor.)
        if k.code == KeyCode::Menu || (k.code == KeyCode::F(10) && shift) {
            self.menu_x = None;
            self.open_context();
            return true;
        }
        if k.code == KeyCode::F(1) {
            if shift {
                self.done = Some(Outcome::Host(format!("guide:{}", self.guide_word())));
            } else {
                let d = h.hover(&self.t.text, self.t.cursor);
                self.hover = (!d.trim().is_empty()).then(|| (self.t.cursor, d));
                if self.hover.is_none() && self.t.text.trim().is_empty() {
                    self.done = Some(Outcome::Host("help:keys".into()));
                }
            }
            return true;
        }
        if self.comp_key(k, h) {
            return true;
        }
        if k.code == KeyCode::Tab && !shift && !ctrl {
            let p = self.t.cursor;
            let lead = &self.t.text[self.t.line_start(p)..p];
            if !lead.trim().is_empty() {
                self.open_comp(h);
                // (As in bash: what fits goes in at once, as far as it's
                // certain.)
                if self.comp.open {
                    self.tab_comp(h);
                }
                return true;
            }
        }
        // (Alt+/: the left Shift tap's other half, the AI asked now.)
        if alt && !ctrl && k.code == KeyCode::Char('/') {
            if !self.accept_ghost() {
                self.ask_ghost_now();
            }
            return true;
        }
        if k.code == KeyCode::Char(' ') && ctrl {
            self.open_comp(h);
            return true;
        }
        if ctrl && !shift && matches!(k.code, KeyCode::Char('r') | KeyCode::Char('R')) && !(self.keys == Keys::Vi && self.vi_normal()) {
            if self.comp.hist {
                self.close_comp();
            } else {
                self.open_history();
            }
            return true;
        }
        if self.common_key(k, ctrl, alt, shift, h) {
            return true;
        }
        if !(self.keys == Keys::Vi && self.vi_normal()) && self.smart_key(k, ctrl, alt) {
            return true;
        }
        let used = match self.keys {
            Keys::Vi => self.vi_key(k, h),
            Keys::Vscode => self.vscode_key(k, ctrl, alt, shift),
            Keys::Emacs => self.emacs_key(k, ctrl, alt, shift),
        };
        if used {
            return true;
        }
        // (What a text field does with the rest: moving, selecting, deleting.)
        if self.field_key(k, ctrl, alt, shift) {
            return true;
        }
        if k.code == KeyCode::Tab {
            let u = self.indent().unit();
            self.t.insert(&u);
            return true;
        }
        false
    }

    pub fn vi_normal(&self) -> bool {
        self.keys == Keys::Vi && self.vi.normal
    }
    fn vi_key(&mut self, k: &KeyEvent, _h: &mut dyn Hooks) -> bool {
        let ind = self.indent();
        let out = self.vi.key(k, &mut self.t, &ind, &mut self.ex);
        if out == super::vi::ViOut::None {
            // (Insert mode keeps the readline keys.)
            if self.vi.normal {
                return false;
            }
            let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
            let alt = k.modifiers.contains(KeyModifiers::ALT);
            let shift = k.modifiers.contains(KeyModifiers::SHIFT);
            return self.emacs_key(k, ctrl, alt, shift);
        }
        self.vi_out(out);
        true
    }
    /// What vi or its command line asks for.
    fn vi_out(&mut self, out: super::vi::ViOut) {
        use super::vi::ViOut;
        match out {
            ViOut::HistoryStep(d) => self.history_step(d),
            ViOut::Submit => self.submit(false),
            ViOut::Clear => self.set_text(""),
            ViOut::None | ViOut::Handled => {}
            o => {
                self.cmd.apply(&o, &self.ex);
            }
        }
    }

    fn common_key(&mut self, k: &KeyEvent, ctrl: bool, alt: bool, shift: bool, _h: &mut dyn Hooks) -> bool {
        if k.code == KeyCode::Enter {
            if shift || alt {
                self.newline();
                return true;
            }
            if ctrl {
                self.submit_background();
                return true;
            }
            self.enter();
            return true;
        }
        // (Ctrl+J: a new line, in every terminal.)
        if ctrl && !shift && k.code == KeyCode::Char('j') {
            self.newline();
            return true;
        }
        // (Alt+&: in the background.)
        if alt && k.code == KeyCode::Char('&') {
            self.submit_background();
            return true;
        }
        if ctrl && !shift && k.code == KeyCode::Char('c') {
            if let Some(s) = self.t.selection() {
                super::term::Term::copy(&self.t.text[s.0..s.1]);
                return true;
            }
            if !self.t.is_empty() {
                self.set_text("");
                self.hist_index = None;
            } else {
                self.done = Some(Outcome::Interrupt);
            }
            return true;
        }
        if ctrl && !shift && k.code == KeyCode::Char('d') && self.t.is_empty() {
            self.done = Some(Outcome::Eof);
            return true;
        }
        if ctrl && !shift && k.code == KeyCode::Char('l') {
            self.done = Some(Outcome::Host("clear".into()));
            return true;
        }
        let p = self.t.cursor;
        let first = !self.t.text[..p].contains('\n');
        let last = !self.t.text[p..].contains('\n');
        let vin = self.keys == Keys::Vi && self.vi_normal();
        if k.code == KeyCode::Up && !shift && !ctrl && !alt && first && !vin {
            self.history_step(-1);
            return true;
        }
        if k.code == KeyCode::Down && !shift && !ctrl && !alt && last && !vin {
            self.history_step(1);
            return true;
        }
        if k.code == KeyCode::PageUp && !shift {
            self.history_step(-1);
            return true;
        }
        if k.code == KeyCode::PageDown && !shift {
            self.history_step(1);
            return true;
        }
        false
    }

    pub fn submit_background(&mut self) {
        if self.t.text.trim().is_empty() {
            return;
        }
        self.submit(true);
    }

    fn type_char(&mut self, c: char) {
        let s = c.to_string();
        self.t.insert(&s);
    }

    /// VS Code's: indentation by level, pairs, typing over closers.
    fn smart_key(&mut self, k: &KeyEvent, ctrl: bool, alt: bool) -> bool {
        if ctrl || alt {
            return false;
        }
        let ind = self.indent();
        let p = self.t.cursor;
        match k.code {
            KeyCode::BackTab => {
                self.t.shift_lines(-1, &ind);
                return true;
            }
            KeyCode::Tab => {
                let ls = self.t.line_start(p);
                let lead = self.t.text[ls..p].trim().is_empty();
                if self.t.selected().contains('\n') {
                    self.t.shift_lines(1, &ind);
                    return true;
                }
                if lead {
                    let c = ind.cols_of(&self.t.text[ls..p]);
                    self.t.set_indent(ls, c / ind.size.max(1) * ind.size + ind.size, &ind);
                    self.t.cursor = ls + self.t.indent_at(ls).len();
                    return true;
                }
                return false;
            }
            KeyCode::Backspace if p > 0 && self.t.selection().is_none() => {
                let o = self.t.char_before(p).unwrap_or(' ');
                if let Some(ki) = self.t.pair_closing_at(p) {
                    if self.t.pairs[ki].open == self.t.prev(p) && self.t.char_at(p) == Some(pair_of(o)) {
                        let n = self.t.next(p);
                        self.t.replace(self.t.prev(p), n, "");
                        return true;
                    }
                }
                let ls = self.t.line_start(p);
                let lead = &self.t.text[ls..p];
                if !lead.is_empty() && lead.chars().all(|c| c == ' ' || c == '\t') {
                    let c = ind.cols_of(lead);
                    let size = ind.size.max(1);
                    let rest = self.t.indent_at(ls).len() - lead.len();
                    self.t.set_indent(ls, (c.div_ceil(size) * size).saturating_sub(size), &ind);
                    self.t.cursor = ls + self.t.indent_at(ls).len() - rest;
                    return true;
                }
                return false;
            }
            KeyCode::Char(ch) => {
                // Over a closer typed for you: step past it.
                if ")]}\"'`".contains(ch) && self.t.char_at(p) == Some(ch) && self.t.selection().is_none() {
                    if let Some(ki) = self.t.pair_closing_at(p) {
                        self.t.pairs.remove(ki);
                        self.t.cursor = self.t.next(p);
                        return true;
                    }
                    if "\"'`".contains(ch) {
                        return false;
                    }
                }
                if "([{\"'`".contains(ch) {
                    let close = pair_of(ch);
                    if let Some((a, b)) = self.t.selection() {
                        let sel = self.t.text[a..b].to_string();
                        self.t.replace(a, b, &format!("{ch}{sel}{close}"));
                        let o = a + ch.len_utf8();
                        self.t.select(o, o + sel.len());
                        return true;
                    }
                    let quote = "\"'`".contains(ch);
                    let prev = self.t.char_before(p).unwrap_or(' ');
                    let next = self.t.char_at(p).unwrap_or('\n');
                    if !";:.,=}])> \t\n".contains(next) {
                        return false;
                    }
                    if quote && (prev.is_alphanumeric() || prev == '_' || prev == '\\' || prev == '$' || self.t.in_string(p)) {
                        return false;
                    }
                    self.t.replace(p, p, &format!("{ch}{close}"));
                    self.t.cursor = p + ch.len_utf8();
                    self.t.pairs.push(super::text::Pair { open: p, close: p + ch.len_utf8() });
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    fn emacs_key(&mut self, k: &KeyEvent, ctrl: bool, alt: bool, shift: bool) -> bool {
        let p = self.t.cursor;
        if ctrl && !shift {
            match k.code {
                KeyCode::Char('a') => self.t.move_to(self.t.line_start(p), false),
                KeyCode::Char('e') => self.t.move_to(self.t.line_end(p), false),
                KeyCode::Char('b') => self.t.move_to(self.t.prev(p), false),
                KeyCode::Char('f') => self.t.move_to(self.t.next(p), false),
                KeyCode::Char('k') => {
                    let e = self.t.line_end(p);
                    let e = if e == p { self.t.next(p) } else { e };
                    self.t.kill(p, e, false);
                }
                KeyCode::Char('u') => self.t.kill(self.t.line_start(p), p, false),
                KeyCode::Char('w') => self.t.kill(self.t.word_left(p), p, false),
                KeyCode::Char('y') => {
                    let s = self.t.kill_ring.clone();
                    self.t.insert(&s);
                }
                KeyCode::Char('h') => {
                    if p > 0 {
                        self.t.replace(self.t.prev(p), p, "");
                    }
                }
                KeyCode::Char('p') => self.history_step(-1),
                KeyCode::Char('n') => self.history_step(1),
                KeyCode::Char('_') | KeyCode::Char('/') | KeyCode::Char('7') => self.t.undo(),
                KeyCode::Char('t') => {
                    if p > 0 && p < self.t.len() {
                        let a = self.t.prev(p);
                        let b = self.t.next(p);
                        let s: String = self.t.text[a..p].to_string();
                        let n: String = self.t.text[p..b].to_string();
                        self.t.replace(a, b, &(n + &s));
                    }
                }
                _ => return false,
            }
            return true;
        }
        if alt && !ctrl {
            match k.code {
                KeyCode::Char('f') => self.t.move_to(self.t.word_right(p), false),
                KeyCode::Char('b') => self.t.move_to(self.t.word_left(p), false),
                KeyCode::Char('d') => self.t.kill(p, self.t.word_right(p), false),
                KeyCode::Backspace => self.t.kill(self.t.word_left(p), p, false),
                _ => return false,
            }
            return true;
        }
        false
    }

    fn vscode_key(&mut self, k: &KeyEvent, ctrl: bool, alt: bool, shift: bool) -> bool {
        let p = self.t.cursor;
        if ctrl && shift && matches!(k.code, KeyCode::Char('k') | KeyCode::Char('K')) {
            let a = self.t.line_start(p);
            let b = self.t.line_end(p);
            let b = if b < self.t.len() { b + 1 } else { b };
            self.t.kill(a, b, false);
            return true;
        }
        if alt && matches!(k.code, KeyCode::Up | KeyCode::Down) {
            // Move the line up or down.
            let t = self.t.text.clone();
            let (a, b) = (self.t.line_start(p), self.t.line_end(p));
            let line = &t[a..b];
            let col = p - a;
            if k.code == KeyCode::Up && a > 0 {
                let pa = self.t.line_start(a - 1);
                let s = format!("{}{}\n{}{}", &t[..pa], line, &t[pa..a - 1], &t[b..]);
                self.t.set(&s);
                self.t.cursor = pa + col;
            } else if k.code == KeyCode::Down && b < t.len() {
                let nb = self.t.line_end(b + 1);
                let next = &t[b + 1..nb];
                let s = format!("{}{}\n{}{}", &t[..a], next, line, &t[nb..]);
                self.t.set(&s);
                self.t.cursor = a + next.len() + 1 + col;
            }
            return true;
        }
        if ctrl && matches!(k.code, KeyCode::Char('/') | KeyCode::Char('7')) {
            let a = self.t.line_start(p);
            if self.t.text[a..].starts_with("# ") {
                self.t.replace(a, a + 2, "");
                self.t.cursor = p.saturating_sub(2).max(a);
            } else {
                self.t.replace(a, a, "# ");
                self.t.cursor = p + 2;
            }
            return true;
        }
        if ctrl && !shift {
            match k.code {
                KeyCode::Char('a') => {
                    self.t.select(0, self.t.len());
                    return true;
                }
                KeyCode::Char('x') => {
                    if let Some((a, b)) = self.t.selection() {
                        super::term::Term::copy(&self.t.text[a..b]);
                        self.t.replace(a, b, "");
                    }
                    return true;
                }
                KeyCode::Char('v') => return false,
                _ => {}
            }
        }
        false
    }

    /// A text field's own keys (Qt's TextEdit): arrows, Home/End (and with
    /// Shift selecting), Ctrl+arrows by word, Delete, Backspace, undo.
    fn field_key(&mut self, k: &KeyEvent, ctrl: bool, _alt: bool, shift: bool) -> bool {
        let p = self.t.cursor;
        let t = &self.t;
        let to = match k.code {
            KeyCode::Left if ctrl => t.word_left(p),
            KeyCode::Right if ctrl => t.word_right(p),
            KeyCode::Left => {
                if let (Some((a, _)), false) = (t.selection(), shift) {
                    a
                } else {
                    t.prev(p)
                }
            }
            KeyCode::Right => {
                if let (Some((_, b)), false) = (t.selection(), shift) {
                    b
                } else {
                    t.next(p)
                }
            }
            KeyCode::Home if ctrl => 0,
            KeyCode::End if ctrl => t.len(),
            KeyCode::Home => {
                // (Smart Home: the line's text first, then its start.)
                let ls = t.line_start(p);
                let txt = ls + t.indent_at(ls).len();
                if p == txt {
                    ls
                } else {
                    txt
                }
            }
            KeyCode::End => t.line_end(p),
            KeyCode::Up | KeyCode::Down => {
                let ls = t.line_start(p);
                let col = crate::editor::screen::cols_of(&t.text[ls..p]);
                let target = if k.code == KeyCode::Up {
                    if ls == 0 {
                        return true;
                    }
                    t.line_start(ls - 1)
                } else {
                    let le = t.line_end(p);
                    if le >= t.len() {
                        return true;
                    }
                    le + 1
                };
                let le = t.line_end(target);
                let mut q = target;
                while q < le && crate::editor::screen::cols_of(&t.text[target..t.next(q)]) <= col {
                    q = t.next(q);
                }
                q
            }
            KeyCode::Delete => {
                if let Some((a, b)) = self.t.selection() {
                    self.t.replace(a, b, "");
                } else if p < self.t.len() {
                    let n = if ctrl { self.t.word_right(p) } else { self.t.next(p) };
                    self.t.replace(p, n, "");
                }
                return true;
            }
            KeyCode::Backspace => {
                if let Some((a, b)) = self.t.selection() {
                    self.t.replace(a, b, "");
                } else if p > 0 {
                    let a = if ctrl { self.t.word_left(p) } else { self.t.prev(p) };
                    self.t.replace(a, p, "");
                }
                return true;
            }
            KeyCode::Char('z') if ctrl && !shift => {
                self.t.undo();
                return true;
            }
            KeyCode::Char('z') | KeyCode::Char('Z') if ctrl && shift => {
                self.t.redo();
                return true;
            }
            KeyCode::Char('y') if ctrl => {
                self.t.redo();
                return true;
            }
            _ => return false,
        };
        self.t.move_to(to, shift);
        true
    }

    /// The word the guide opens at: the selection (one line), else the word
    /// at the cursor.
    pub fn guide_word(&self) -> String {
        let sel = self.t.selected().trim();
        if !sel.is_empty() && sel.len() <= 60 && !sel.contains('\n') {
            return sel.to_string();
        }
        let (a, b) = self.t.word_around(self.t.cursor);
        let w = &self.t.text[a..b];
        if w.chars().any(|c| c.is_alphanumeric()) {
            w.to_string()
        } else {
            String::new()
        }
    }

    // ---- lone modifier taps ---------------------------------------------------------------
    fn modifier(&mut self, m: ModifierKeyCode, kind: KeyEventKind, h: &mut dyn Hooks) {
        let quick = |at: Instant| at.elapsed() < Duration::from_millis(450);
        match (m, kind) {
            (ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift, KeyEventKind::Press) => {
                // (keyd's overload: its layer's left Shift, then the tapped key;
                // the later press replaces a tap still waiting.)
                self.pending_tap = None;
                self.shift_side = if m == ModifierKeyCode::LeftShift { 1 } else { 2 };
                self.shift_at = Instant::now();
            }
            (ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift, KeyEventKind::Release) => {
                let side = self.shift_side;
                self.shift_side = 0;
                if side != 0 && quick(self.shift_at) {
                    self.pending_tap = Some((if side == 2 { 'n' } else { 'g' }, Instant::now()));
                }
            }
            (ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl, KeyEventKind::Press) => {
                self.pending_tap = None;
                self.ctrl_side = if m == ModifierKeyCode::LeftControl { 1 } else { 2 };
                self.ctrl_at = Instant::now();
            }
            (ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl, KeyEventKind::Release) => {
                let side = self.ctrl_side;
                self.ctrl_side = 0;
                // (Right Ctrl: in the background. Left: nothing here, the
                // editor is the terminal.)
                if side == 2 && quick(self.ctrl_at) {
                    self.pending_tap = Some(('b', Instant::now()));
                }
            }
            _ => {
                self.shift_side = 0;
                self.ctrl_side = 0;
            }
        }
        let _ = h;
    }

    // ---- popups' keys -------------------------------------------------------------------
    fn hist_key(&mut self, k: &KeyEvent, h: &mut dyn Hooks) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Esc => {
                self.close_comp();
                return;
            }
            KeyCode::Enter => {
                self.accept_comp(None, h);
                return;
            }
            KeyCode::Char('r') if ctrl => {
                self.comp.index = (self.comp.index + 1).min(self.comp.items.len().saturating_sub(1));
                return;
            }
            KeyCode::Char('c') | KeyCode::Char('g') if ctrl => {
                self.close_comp();
                return;
            }
            _ => {}
        }
        if self.comp_key(k, h) {
            return;
        }
        let before = self.comp.query.text.clone();
        let q = &mut self.comp.query;
        match k.code {
            KeyCode::Char(c) if !ctrl => q.insert(&c.to_string()),
            KeyCode::Backspace => {
                if let Some((a, b)) = q.selection() {
                    q.replace(a, b, "");
                } else if q.cursor > 0 {
                    let a = q.prev(q.cursor);
                    q.replace(a, q.cursor, "");
                }
            }
            KeyCode::Left => q.move_to(q.prev(q.cursor), false),
            KeyCode::Right => q.move_to(q.next(q.cursor), false),
            KeyCode::Home => q.move_to(0, false),
            KeyCode::End => q.move_to(q.len(), false),
            KeyCode::Char('u') if ctrl => q.set(""),
            _ => {}
        }
        if self.comp.query.text != before {
            self.comp.index = 0;
            self.comp.top = 0;
            self.search_history();
        }
    }

    /// The text's right-click menu, as a text field has it.
    pub fn open_context(&mut self) {
        let sel = self.t.selection().is_some();
        let it = |label: &str, cmd: &str, enabled: bool| MenuItem { label: label.into(), cmd: cmd.into(), section: false, checked: false, enabled, key: String::new() };
        let items = vec![
            it("Undo", "undo", true),
            it("Redo", "redo", true),
            it("Cut", "cut", sel),
            it("Copy", "copy", sel),
            it("Paste", "paste", true),
            it("Delete", "delete", sel),
            it("Select All", "all", !self.t.is_empty()),
        ];
        self.open_menu(MenuKind::Context, items);
    }

    /// The options menu (the ▾ at the prompt bar's left, Ctrl+Alt+O): what
    /// the toolbar's buttons were, each with its shortcut. Again: closed.
    pub fn toggle_options(&mut self) {
        if matches!(&self.menu, Some(m) if m.kind == MenuKind::Options) {
            self.menu = None;
            return;
        }
        let it = |label: String, cmd: &str, key: &str| MenuItem { label, cmd: cmd.into(), enabled: true, key: key.into(), ..Default::default() };
        let sep = || MenuItem { label: "─".repeat(24), section: true, ..Default::default() };
        let mut items = vec![];
        if self.status != 0 && self.t.is_empty() && self.ai.lock().unwrap().command_ready {
            items.push(it("✦ Fix the failed command".into(), "chip:fix", "Ctrl+Alt+X"));
            items.push(sep());
        }
        items.push(it("\u{f07b} Folders (cd history)…".into(), "chip:folders", "Ctrl+Alt+D"));
        items.push(it("↺ Command history…".into(), "chip:history", "Ctrl+R"));
        if self.jobs.1 > 0 {
            items.push(it(format!("⧗ Background jobs ({} running)", self.jobs.0), "jobs", "Ctrl+Alt+J"));
        }
        items.push(it("tessh guide".into(), "chip:guide", "Shift+F1"));
        items.push(it("Prompt bar choices…".into(), "widgets", "Ctrl+Alt+P"));
        items.push(sep());
        let ind = self.indent();
        let mode = match ind.mode {
            super::text::IndentMode::Spaces => "Spaces",
            super::text::IndentMode::Tabs => "Tabs",
            super::text::IndentMode::Mixed => "Mixed",
        };
        let det = if self.detected().is_some() { " · detected" } else { "" };
        items.push(it(format!("Indentation: {mode} {}{det}…", ind.size), "chip:indent", "Ctrl+Alt+S"));
        items.push(sep());
        items.push(MenuItem { label: "Editor keys".into(), section: true, key: "Ctrl+Alt+E".into(), ..Default::default() });
        for (k, name) in [(Keys::Emacs, "Emacs"), (Keys::Vi, "vi"), (Keys::Vscode, "VS Code")] {
            items.push(MenuItem { checked: self.keys == k, ..it(name.into(), &format!("chip:keys={}", k.name()), "") });
        }
        items.push(sep());
        items.push(it("Keyboard shortcuts".into(), "help:keys", "F1"));
        self.menu_x = Some(0);
        self.open_menu(MenuKind::Options, items);
    }

    pub fn open_menu(&mut self, kind: MenuKind, items: Vec<MenuItem>) {
        let sel = items.iter().position(|i| !i.section && i.enabled).unwrap_or(0);
        self.close_comp();
        self.menu = Some(Menu { kind, items, sel, filter: Text::new(), top: 0 });
    }
    pub fn menu_shown(&self) -> Vec<usize> {
        let Some(m) = &self.menu else { return vec![] };
        let f = m.filter.text.to_lowercase();
        (0..m.items.len()).filter(|&i| f.is_empty() || (!m.items[i].section && m.items[i].label.to_lowercase().contains(&f))).collect()
    }
    fn menu_key(&mut self, k: &KeyEvent, h: &mut dyn Hooks) {
        let shown = self.menu_shown();
        let Some(m) = self.menu.as_mut() else { return };
        let pos = shown.iter().position(|&i| i == m.sel).unwrap_or(0);
        let ok = |m: &Menu, i: usize| !m.items[i].section && m.items[i].enabled;
        match k.code {
            KeyCode::Esc => {
                self.menu = None;
            }
            KeyCode::Down | KeyCode::Up => {
                let n = shown.len();
                if n == 0 {
                    return;
                }
                let mut p = pos;
                for _ in 0..n {
                    p = if k.code == KeyCode::Down { (p + 1) % n } else { (p + n - 1) % n };
                    if ok(m, shown[p]) {
                        break;
                    }
                }
                m.sel = shown[p];
            }
            KeyCode::Enter => {
                let i = m.sel;
                self.pick_menu(i, h);
            }
            KeyCode::Backspace => {
                let f = &mut m.filter;
                if f.cursor > 0 {
                    let a = f.prev(f.cursor);
                    f.replace(a, f.cursor, "");
                }
                let first = self.menu_shown().into_iter().find(|&i| ok(self.menu.as_ref().unwrap(), i));
                if let (Some(m), Some(i)) = (self.menu.as_mut(), first) {
                    m.sel = i;
                }
            }
            KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) && m.kind != MenuKind::Indent && m.kind != MenuKind::Options => {
                m.filter.insert(&c.to_string());
                let first = self.menu_shown().into_iter().find(|&i| ok(self.menu.as_ref().unwrap(), i));
                if let (Some(m), Some(i)) = (self.menu.as_mut(), first) {
                    m.sel = i;
                }
            }
            _ => {}
        }
    }
    pub fn pick_menu(&mut self, i: usize, _h: &mut dyn Hooks) {
        let Some(m) = self.menu.take() else { return };
        let Some(it) = m.items.get(i) else { return };
        if it.section || !it.enabled {
            self.menu = Some(m);
            return;
        }
        match m.kind {
            MenuKind::Choices(_) => {
                if !it.cmd.is_empty() {
                    self.done = Some(Outcome::Host(format!("run:{}", it.cmd)));
                }
            }
            MenuKind::Indent => self.done = Some(Outcome::Host(format!("indent:{}", it.cmd))),
            MenuKind::Widgets => self.done = Some(Outcome::Host(format!("widget:{}", it.cmd))),
            MenuKind::Ls(_) => self.ls_pick(&it.cmd),
            MenuKind::Options => self.done = Some(Outcome::Host(it.cmd.clone())),
            MenuKind::Context => {
                let sel = self.t.selection();
                match it.cmd.as_str() {
                    "undo" => self.t.undo(),
                    "redo" => self.t.redo(),
                    "cut" | "copy" | "delete" => {
                        if let Some((a, b)) = sel {
                            if it.cmd != "delete" {
                                super::term::Term::copy(&self.t.text[a..b]);
                            }
                            if it.cmd != "copy" {
                                self.t.replace(a, b, "");
                            }
                        }
                    }
                    "paste" => {
                        if let Some(c) = clipboard() {
                            self.t.insert(&c.replace("\r\n", "\n"));
                        }
                    }
                    "all" => self.t.select(0, self.t.len()),
                    _ => {}
                }
                self.changed();
            }
        }
    }

    // ---- the mouse ------------------------------------------------------------------------
    /// A mouse event over `hit` (None: outside the editor). Returns false
    /// when the mouse should go back to the terminal.
    pub fn mouse(&mut self, e: MouseEvent, hit: Option<Hit>, h: &mut dyn Hooks) -> bool {
        self.dirty = true;
        match e.kind {
            MouseEventKind::Moved => {
                self.pointer = Some((e.column, e.row, Instant::now()));
                if !matches!(hit, Some(Hit::Text(_))) {
                    self.hover = self.hover.take().filter(|_| matches!(hit, Some(Hit::Doc)));
                }
                if self.tooltip.as_ref().is_some_and(|t| (t.0, t.1) != (e.column, e.row)) {
                    self.tooltip = None;
                }
                true
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                let up = e.kind == MouseEventKind::ScrollUp;
                match hit {
                    Some(Hit::Doc) => {
                        self.comp.doc_scroll = if up { self.comp.doc_scroll.saturating_sub(3) } else { self.comp.doc_scroll + 3 };
                        true
                    }
                    // (Over a list or its scrollbar: the list, its pick kept
                    // among the rows shown, so it gets to the end.)
                    Some(Hit::Row(_) | Hit::Scroll(_) | Hit::Thumb(..) | Hit::ScrollArrow(..)) => {
                        let top = self.list_top.get();
                        self.list_to(if up { top.saturating_sub(3) } else { top + 3 }, self.list_room.get());
                        true
                    }
                    // (Elsewhere: the terminal's own scrolling.)
                    _ => false,
                }
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.pressed = hit.clone();
                match hit {
                    Some(Hit::Text(p)) => {
                        let now = Instant::now();
                        let n = match self.last_click {
                            Some((t, c, r, n)) if t.elapsed() < Duration::from_millis(400) && c == e.column && r == e.row => n % 3 + 1,
                            _ => 1,
                        };
                        self.last_click = Some((now, e.column, e.row, n));
                        self.close_comp();
                        self.menu = None;
                        match n {
                            2 => {
                                let (a, b) = self.t.word_around(p);
                                self.t.select(a, b);
                            }
                            3 => {
                                let (a, b) = (self.t.line_start(p), self.t.line_end(p));
                                self.t.select(a, b);
                            }
                            _ => {
                                if e.modifiers.contains(KeyModifiers::SHIFT) {
                                    self.t.move_to(p, true);
                                } else {
                                    self.t.move_to(p, false);
                                }
                            }
                        }
                        self.drag = Some((self.t.anchor.unwrap_or(p), n));
                        self.refresh_ghost();
                        true
                    }
                    Some(Hit::Thumb(n, room)) => {
                        let top = self.list_top.get();
                        self.bar_drag = Some((e.row, top.min(n.saturating_sub(room)), n, room));
                        true
                    }
                    None => {
                        // (Above the editor: the terminal selects there.)
                        self.close_comp();
                        self.menu = None;
                        false
                    }
                    _ => true,
                }
            }
            MouseEventKind::Drag(MouseButton::Left) if self.bar_drag.is_some() => {
                // (The thumb follows the pointer, and the list the thumb.)
                let (r0, top0, n, room) = self.bar_drag.unwrap();
                let track = if room >= 4 { room - 2 } else { room };
                let (_, size) = super::view::thumb_of(n, room, top0);
                let span = n.saturating_sub(room);
                let d = e.row as isize - r0 as isize;
                let moved = d * span as isize / (track.saturating_sub(size)).max(1) as isize;
                self.list_to((top0 as isize + moved).clamp(0, span as isize) as usize, room);
                true
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let (Some((anchor, n)), Some(Hit::Text(p))) = (self.drag, hit) {
                    if n == 2 {
                        let (a, b) = self.t.word_around(p);
                        let (wa, wb) = self.t.word_around(anchor);
                        self.t.select(wa.min(a), wb.max(b));
                    } else {
                        self.t.select(anchor, p);
                    }
                }
                true
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.drag = None;
                if self.bar_drag.take().is_some() {
                    self.pressed = None;
                    return true;
                }
                // (A selection made: copied, as terminals do.)
                if let Some((a, b)) = self.t.selection() {
                    super::term::Term::copy(&self.t.text[a..b]);
                }
                let pressed = self.pressed.take();
                if pressed.is_some() && pressed == hit {
                    self.click(hit.unwrap(), h);
                }
                true
            }
            MouseEventKind::Down(MouseButton::Right) => {
                if let Some(Hit::Text(p)) = hit {
                    // (Outside the selection: the cursor goes there first.)
                    if self.t.selection().is_none_or(|(a, b)| p < a || p > b) {
                        self.t.move_to(p, false);
                    }
                    self.menu_x = Some(e.column as usize);
                    self.open_context();
                }
                true
            }
            MouseEventKind::Down(MouseButton::Middle) => {
                if let Some(Hit::Text(p)) = hit {
                    self.t.move_to(p, false);
                }
                true
            }
            _ => true,
        }
    }

    /// A popup list (a menu, or the history or completion list) scrolled so
    /// row `top` is its first of `room`, its pick kept among those shown.
    fn list_to(&mut self, top: usize, room: usize) {
        if self.menu.is_some() {
            let shown = self.menu_shown();
            let m = self.menu.as_mut().unwrap();
            let top = top.min(shown.len().saturating_sub(room));
            m.top = top;
            let p = shown.iter().position(|&i| i == m.sel).unwrap_or(0);
            if p < top {
                m.sel = shown[top];
            } else if p >= top + room {
                m.sel = shown[(top + room - 1).min(shown.len() - 1)];
            }
        } else if self.comp.open {
            let n = self.comp.items.len();
            let top = top.min(n.saturating_sub(room));
            self.comp.top = top;
            if self.comp.index < top {
                self.comp.index = top;
            } else if self.comp.index >= top + room {
                self.comp.index = (top + room - 1).min(n.saturating_sub(1));
            }
        }
    }

    fn click(&mut self, hit: Hit, h: &mut dyn Hooks) {
        match hit {
            Hit::Row(i) => {
                if self.menu.is_some() {
                    self.pick_menu(i, h);
                } else if self.comp.open {
                    self.comp.index = i;
                    self.accept_comp(Some(i), h);
                }
            }
            Hit::Chip("menu") => {
                self.close_comp();
                self.toggle_options();
            }
            Hit::Chip(name) => {
                // (Its list open: the click closes it. Another's: that goes first.)
                let open = match &self.menu {
                    Some(m) if m.kind == MenuKind::Indent => "indent",
                    Some(m) if m.kind == MenuKind::Choices("dir".into()) => "folders",
                    _ if self.comp.open && self.comp.hist => "history",
                    _ => "",
                };
                self.menu = None;
                self.close_comp();
                if open != name {
                    self.done = Some(Outcome::Host(format!("chip:{name}")));
                }
            }
            // (A list's scrollbar: a page up or down.)
            Hit::Scroll(up) => {
                let page = 13;
                if let Some(m) = self.menu.as_mut() {
                    m.top = if up { m.top.saturating_sub(page) } else { m.top + page };
                    let shown = self.menu_shown();
                    let m = self.menu.as_mut().unwrap();
                    if let Some(p) = shown.iter().position(|&i| i == m.sel) {
                        let to = if up { p.saturating_sub(page) } else { (p + page).min(shown.len().saturating_sub(1)) };
                        m.sel = shown[to];
                    }
                } else if self.comp.open {
                    let n = self.comp.items.len();
                    self.comp.index = if up { self.comp.index.saturating_sub(page) } else { (self.comp.index + page).min(n.saturating_sub(1)) };
                    self.comp.top = if up { self.comp.top.saturating_sub(page) } else { self.comp.top + page };
                }
            }
            Hit::ScrollArrow(up, room) => {
                let top = self.list_top.get();
                self.list_to(if up { top.saturating_sub(1) } else { top + 1 }, room);
            }
            Hit::Path(p) => self.done = Some(Outcome::Host(format!("run:cd {}", crate::repl::quote_path(&p)))),
            Hit::Widget(w) => {
                let same = matches!(&self.menu, Some(m) if m.kind == MenuKind::Choices(w.clone()));
                self.menu = None;
                self.close_comp();
                if !same {
                    self.done = Some(Outcome::Host(format!("widget:{w}")));
                }
            }
            _ => {}
        }
    }

    // ---- time ---------------------------------------------------------------------------
    /// What waiting does (called every ~50 ms, and after each key): the list
    /// opens or updates once no more keys are waiting, errors show,
    /// tooltips come up.
    /// A tap whose moment passed with no key after it: done now.
    fn shift_takes_item(&mut self, h: &mut dyn Hooks) -> bool {
        if !self.comp.hist && (self.comp.for_text != self.t.text || self.comp.for_cursor != self.t.cursor) {
            self.request_complete(h);
        }
        let Some(it) = self.comp.items.get(self.comp.index) else { return false };
        if self.comp.chosen || self.comp.hist {
            return true;
        }
        let s = self.comp.start.min(self.t.cursor);
        let typed = self.t.text.get(s..self.t.cursor).unwrap_or("").to_lowercase();
        // (Nothing typed yet, after "cd " or a bracket: picked first.)
        !typed.is_empty() && it.insert.to_lowercase().starts_with(&typed)
    }
    pub fn fire_tap(&mut self, h: &mut dyn Hooks) {
        let Some((t, at)) = self.pending_tap else { return };
        if at.elapsed() < Duration::from_millis(60) {
            return;
        }
        self.pending_tap = None;
        self.dirty = true;
        match t {
            // (Right Shift with the list up: the item highlighted in it, as
            // Tab would once there's no doubt left.)
            // (Only an item picked with Up/Down or starting with what's
            // typed: a list up on a loose match leaves it a new line.)
            'n' if self.comp.open && self.shift_takes_item(h) => self.accept_comp(None, h),
            'n' => self.newline(),
            'g' => {
                if !self.accept_ghost() {
                    self.ask_ghost_now();
                }
            }
            _ => self.submit_background(),
        }
    }

    pub fn tick(&mut self, h: &mut dyn Hooks, tooltip_for: fn(&Ed, &Hit) -> String, hit_at: impl Fn(u16, u16) -> Option<Hit>) {
        // (Not while stepping through it with ↑↓: the places would move.)
        if self.hist_index.is_none() {
            let fresh = self.history_in.lock().unwrap().take();
            if let Some(hs) = fresh {
                self.history = hs;
                if self.comp.open && self.comp.hist {
                    self.search_history();
                }
                self.dirty = true;
            }
        }
        let since = self.edited.map(|t| t.elapsed());
        if let Some(d) = since {
            // (At once when the keys stop coming; a steady stream of them,
            // a paste typed out, every 300 ms.)
            if self.comp.typing && (self.input_idle || d >= Duration::from_millis(300)) {
                self.comp.typing = false;
                self.dirty = true;
                self.comp_auto(h);
            }
            if !self.analyzed && d >= Duration::from_millis(650) {
                self.analyzed = true;
                self.analyze(h);
                self.dirty = true;
            }
        }
        if let Some((c, r, at)) = self.pointer {
            let under = hit_at(c, r);
            // (How long it rests first: the listing's as Tessera's tooltips wait.)
            let wait = match &under {
                Some(Hit::Ls(x)) => super::ls::tip_delay(x),
                _ => 550,
            };
            if at.elapsed() >= Duration::from_millis(wait) {
                self.pointer = None;
                match under {
                    Some(Hit::Text(p)) if self.hover.is_none() && !self.comp.open && self.menu.is_none() && !self.t.text.trim().is_empty() => {
                        let d = h.hover(&self.t.text, p);
                        if !d.trim().is_empty() {
                            self.hover = Some((p, d));
                            self.dirty = true;
                        }
                    }
                    Some(hit) if !matches!(hit, Hit::Text(_) | Hit::None | Hit::Row(_) | Hit::Doc) => {
                        let tip = tooltip_for(self, &hit);
                        if !tip.is_empty() {
                            self.tooltip = Some((c, r, tip));
                            self.dirty = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Open by itself on a word being typed (not after a space, not in a
    /// word's middle), and keep an open list current.
    fn comp_auto(&mut self, h: &mut dyn Hooks) {
        let p = self.t.cursor;
        let prev = self.t.char_before(p).unwrap_or(' ');
        let next = self.t.char_at(p).unwrap_or(' ');
        let in_word = !prev.is_whitespace() && next.is_whitespace();
        let t = &self.t.text;
        let builtin_arg = p == t.len() && {
            let s = t.trim_start();
            // (Options between, cd -L : the folders still.)
            ["cd", "pushd", "ls", "ll", "la", "export", "unset"].iter().any(|b| {
                s.strip_prefix(b).is_some_and(|r| r.starts_with(char::is_whitespace) && r.ends_with(char::is_whitespace) && r.split_whitespace().all(|w| w.starts_with('-')))
            })
        };
        if builtin_arg && !(self.keys == Keys::Vi && self.vi_normal()) {
            self.comp.open = true;
            self.comp.index = 0;
            self.request_complete(h);
            return;
        }
        if self.comp.open && self.comp.hist {
            return;
        }
        if self.from_history {
            return;
        }
        if self.comp.open {
            if in_word || prev == '/' {
                self.request_complete(h);
            } else {
                self.close_comp();
            }
        } else if in_word && !(self.keys == Keys::Vi && self.vi_normal()) && !self.t.text.starts_with('#') {
            self.comp.open = true;
            self.comp.index = 0;
            self.comp.chosen = false;
            self.request_complete(h);
        }
    }

    pub fn analyze(&mut self, h: &mut dyn Hooks) {
        if self.t.text.trim().is_empty() || self.t.text.starts_with('#') {
            self.errors.clear();
            self.problem.clear();
            return;
        }
        self.errors = h.check(&self.t.text);
        self.problem = self.errors.first().map(|e| e.2.clone()).unwrap_or_default();
    }
}

fn pair_of(c: char) -> char {
    match c {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        o => o,
    }
}

impl Default for Text {
    fn default() -> Self {
        Text::new()
    }
}

/// The system clipboard, from the tools that read it (a terminal can't be
/// asked for it reliably).
pub fn clipboard() -> Option<String> {
    let tries: &[&[&str]] = if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        &[&["wl-paste", "-n"], &["xclip", "-o", "-selection", "clipboard"], &["xsel", "-ob"]]
    } else {
        &[&["xclip", "-o", "-selection", "clipboard"], &["xsel", "-ob"], &["wl-paste", "-n"], &["pbpaste"]]
    };
    for t in tries {
        if let Ok(o) = std::process::Command::new(t[0]).args(&t[1..]).stderr(std::process::Stdio::null()).output() {
            if o.status.success() {
                return Some(String::from_utf8_lossy(&o.stdout).into_owned());
            }
        }
    }
    None
}

/// A command as one line for a list: newlines as ⏎, tabs as a space, other
/// control characters (a CR, an escape) as ·. (A tab counted no columns
/// but showed one, and pushed that row's scrollbar off the box's edge.)
fn one_line(h: &str) -> String {
    let mut out = String::with_capacity(h.len());
    for c in h.chars() {
        match c {
            '\n' => out.push_str(" ⏎ "),
            '\t' => out.push(' '),
            '\r' => {}
            c if c.is_control() => out.push('·'),
            c => out.push(c),
        }
    }
    out
}

/// What Tab can put in for the typed word.
#[derive(Debug, PartialEq)]
pub enum TabFit {
    /// Just this item fits.
    One(usize),
    /// Several fit, all starting with this (longer than the word).
    Common(String),
    /// Several fit with no more in common, or none starts with the word
    /// (fuzzy matches only): pick or type on.
    Nothing,
}

/// The items starting with the typed word (else, ignoring case): one, or
/// what they all start with. (By their inserted text: quoting included.)
pub fn tab_common(typed: &str, items: &[Item]) -> TabFit {
    let fits = |fold: bool| -> Vec<usize> {
        let t = if fold { typed.to_lowercase() } else { typed.to_string() };
        (0..items.len())
            .filter(|&i| {
                let ins = &items[i].insert;
                if fold { ins.to_lowercase().starts_with(&t) } else { ins.starts_with(&t) }
            })
            .collect()
    };
    let (mut fit, mut fold) = (fits(false), false);
    if fit.is_empty() {
        (fit, fold) = (fits(true), true);
    }
    // (The same text twice, from two sources, is one.)
    let mut seen = std::collections::HashSet::new();
    fit.retain(|&i| seen.insert(items[i].insert.trim_end().to_string()));
    match fit.len() {
        0 => TabFit::Nothing,
        1 => TabFit::One(fit[0]),
        _ => {
            let first: Vec<char> = items[fit[0]].insert.chars().collect();
            let mut n = first.len();
            for &i in &fit[1..] {
                let m = items[i].insert.chars().zip(first.iter()).take_while(|(a, b)| if fold { a.to_lowercase().eq(b.to_lowercase()) } else { a == *b }).count();
                n = n.min(m);
            }
            let common: String = first[..n].iter().collect();
            if common.chars().count() > typed.chars().count() { TabFit::Common(common) } else { TabFit::Nothing }
        }
    }
}

#[cfg(test)]
mod tab_tests {
    use super::*;
    fn it(s: &str) -> Item {
        Item { label: s.into(), insert: s.into(), ..Default::default() }
    }
    #[test]
    fn tab_like_bash() {
        let items = [it("Documents/"), it("Downloads/"), it("Desktop/")];
        assert_eq!(tab_common("D", &items), TabFit::Nothing);
        assert_eq!(tab_common("Do", &items), TabFit::Nothing);
        assert_eq!(tab_common("Doc", &items), TabFit::One(0));
        assert_eq!(tab_common("De", &items), TabFit::One(2));
        let items = [it("Documents/"), it("Documentation/")];
        assert_eq!(tab_common("D", &items), TabFit::Common("Document".into()));
        assert_eq!(tab_common("doc", &items), TabFit::Common("Document".into()));
        // (Fuzzy matches only: nothing taken.)
        assert_eq!(tab_common("mnt", &[it("Documents/"), it("Music/")]), TabFit::Nothing);
    }
}
