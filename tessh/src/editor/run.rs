//! The editor's loop: keys, the mouse, pastes and resizes in; the frame out
//! when something changed; timers and the AI's answers between events.

use super::core::{Ed, Hooks, Outcome};
use super::screen::Screen;
use super::term::Term;
use super::view::{self, Palette, ViewIn};
use super::{Hit, Line};
use crossterm::event::{self, Event};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// What the prompt bar shows: lines (left) and the right side.
pub type BarFn = Arc<dyn Fn(&str, i32, u64, u16, &[String], Option<((u8, u8, u8), (u8, u8, u8))>) -> (Vec<Line>, Line) + Send + Sync>;

pub struct Editor {
    pub term: Term,
    pub screen: Screen,
    pub ed: Ed,
    bar_fn: BarFn,
    /// The bar: its lines, its right side, its ask's number, and the folder it's for.
    bar: Arc<Mutex<(Vec<Line>, Line, u64, String)>>,
    bar_seq: u64,
    hovered: Option<Hit>,
    pub mode_name: Box<dyn Fn(&Ed) -> String>,
}

/// TESSH_KEYLOG=<file>: what the keyboard and the terminal sent, and what
/// was made of it (for working out a tap that didn't happen).
pub fn keylog(msg: &str) {
    use std::io::Write;
    static FILE: std::sync::OnceLock<Option<Mutex<std::fs::File>>> = std::sync::OnceLock::new();
    let f = FILE.get_or_init(|| std::env::var("TESSH_KEYLOG").ok().and_then(|p| std::fs::OpenOptions::new().create(true).append(true).open(p).ok()).map(Mutex::new));
    if let Some(f) = f {
        static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        let t = T0.get_or_init(Instant::now).elapsed().as_millis();
        let _ = writeln!(f.lock().unwrap(), "{t:>7} {msg}");
    }
}

fn pause_of(task: &str) -> u64 {
    match task {
        "inline" => 220,
        "request" => 250,
        "command" => 1400,
        _ => 0,
    }
}

impl Editor {
    pub fn new(bar_fn: BarFn) -> Editor {
        let term = Term::new();
        let mut ed = Ed::new();
        ed.kitty = term.kitty;
        Editor { term, screen: Screen::new(), ed, bar_fn, bar: Arc::new(Mutex::new((vec![], vec![], 0, String::new()))), bar_seq: 0, hovered: None, mode_name: Box::new(|ed: &Ed| if ed.vi.pending.is_empty() { ed.vi.mode_name().to_string() } else { format!("{} {}", ed.vi.mode_name(), ed.vi.pending) }) }
    }

    fn palette(&self) -> Palette {
        Palette { fg: self.term.fg, bg: self.term.bg }
    }

    /// Work the prompt bar out in the background (Starship and git take a
    /// while); the last one shows until then.
    fn refresh_bar(&mut self) {
        self.bar_seq += 1;
        let seq = self.bar_seq;
        let (f, slot) = (self.bar_fn.clone(), self.bar.clone());
        // (Its first row has the options menu's ▾ at its left: made that much narrower.)
        let (cwd, st, du, w, visited) = (self.ed.cwd.clone(), self.ed.status, self.ed.duration_ms, Term::size().0.saturating_sub((super::view::MENU_BUTTON_W + super::view::PLUS_W) as u16), self.ed.visited.clone());
        let colors = self.term.fg.zip(self.term.bg);
        // (A folder been in before: its last bar at once, till the new one's
        // worked out, not the last folder's. One not been in: the last bar
        // with this folder put in place of that one's, when it shows it whole.)
        static SEEN: Mutex<Option<std::collections::HashMap<(String, u16), (Vec<Line>, Line)>>> = Mutex::new(None);
        let seen = SEEN.lock().unwrap().as_ref().and_then(|m| m.get(&(cwd.clone(), w)).cloned());
        {
            let mut s = self.bar.lock().unwrap();
            if seq >= s.2 && s.3 != cwd {
                let quick = seen.or_else(|| swap_folder(&s.0, &s.3, &cwd, w as usize).map(|l| (l, s.1.clone())));
                if let Some((l, r)) = quick {
                    *s = (l, r, seq, cwd.clone());
                }
            }
        }
        std::thread::spawn(move || {
            let (l, r) = f(&cwd, st, du, w, &visited, colors);
            let mut seen = SEEN.lock().unwrap();
            let m = seen.get_or_insert_with(Default::default);
            if m.len() > 200 {
                m.clear();
            }
            m.insert((cwd.clone(), w), (l.clone(), r.clone()));
            drop(seen);
            let mut s = slot.lock().unwrap();
            if seq >= s.2 {
                *s = (l, r, seq, cwd);
            }
        });
    }

    fn draw(&mut self, h: &mut dyn Hooks, final_: bool) {
        let spans = h.highlight(&self.ed.t.text);
        let (bar, right) = {
            let b = self.bar.lock().unwrap();
            (b.0.clone(), b.1.clone())
        };
        let ai = self.ed.ai.lock().unwrap();
        let (chat, busy) = (ai.command_ready, ai.busy.clone());
        drop(ai);
        let mode = (self.mode_name)(&self.ed);
        let v = ViewIn {
            pal: self.palette(),
            cols: self.screen.cols(),
            rows: self.screen.rows(),
            bar: &bar,
            bar_right: &right,
            hovered: self.hovered.as_ref(),
            spans: &spans,
            chat_ready: chat,
            ai_busy: busy,
            mode_name: mode,
            final_,
            tooltip_pos: None,
            screen_top: self.screen.top,
        };
        let mut hl = |s: &str| h.highlight(s);
        let cache = std::cell::RefCell::new(&mut hl);
        let spans_for = |s: &str| (cache.borrow_mut())(s);
        let mut built = view::build(&self.ed, &v, &spans_for);
        // The ls listing, above it all (not when the prompt's done: it's
        // been printed by then, freeze_ls).
        if !final_ {
            let pal = self.palette();
            let cols = self.screen.cols();
            let room = self.screen.rows().saturating_sub(built.frame.rows.len() + 1);
            let hov = match &self.hovered {
                Some(Hit::Ls(x)) => Some(x.clone()),
                _ => None,
            };
            let focus = self.ed.ls_focus;
            if let Some(ls) = self.ed.ls.as_mut() {
                let (rows, at) = ls.panel(&pal, cols, room, hov.as_ref(), focus);
                let n = rows.len();
                let f = &mut built.frame;
                f.cursor = if focus { at.or(f.cursor.map(|(r, c)| (r + n, c))) } else { f.cursor.map(|(r, c)| (r + n, c)) };
                f.rows.splice(0..0, rows);
                f.bare = n;
                if let Some((lines, cur)) = ls.dialog(&pal, cols) {
                    let w = lines.first().map(|l| super::width(l)).unwrap_or(0);
                    let col = cols.saturating_sub(w) / 2;
                    f.overlay = Some(super::screen::Overlay { lines, col, cursor: cur });
                }
            }
        }
        if final_ {
            self.screen.finish(&built.frame);
        } else {
            self.screen.draw(&built.frame);
        }
    }

    /// The ls listing printed whole, as text, where it was (it stays in
    /// the scrollback): done with it.
    pub fn freeze_ls(&mut self) {
        let Some(mut ls) = self.ed.ls.take() else { return };
        self.ed.ls_focus = false;
        if matches!(self.ed.menu.as_ref().map(|m| &m.kind), Some(super::core::MenuKind::Ls(_))) {
            self.ed.menu = None;
        }
        let cols = self.screen.cols();
        let lines = ls.frozen(&self.palette(), cols);
        self.screen.clear();
        let mut o = std::io::stdout().lock();
        use std::io::Write;
        for l in &lines {
            let _ = write!(o, "{}\r\n", super::screen::line_text(l, cols));
        }
        let _ = o.flush();
        drop(o);
        self.screen.begin();
    }

    /// The editor's rows cleared (something's printed where it was; the next
    /// read draws it again under that).
    pub fn clear(&mut self) {
        self.screen.clear();
        self.term.leave();
    }

    /// Ask the AI what's waited its pause; take what's come back.
    fn ai_tick(&mut self) {
        let recent: String = {
            let h = &self.ed.history;
            h[h.len().saturating_sub(10)..].join("\n")
        };
        let mut start = vec![];
        {
            let mut ai = self.ed.ai.lock().unwrap();
            let now = Instant::now();
            let due: Vec<(&'static str, String)> = ai.want.iter().filter(|w| now.duration_since(w.2) >= Duration::from_millis(pause_of(w.0))).map(|w| (w.0, w.1.clone())).collect();
            ai.want.retain(|w| now.duration_since(w.2) < Duration::from_millis(pause_of(w.0)));
            for d in due {
                if !ai.asking.contains(&d) {
                    ai.asking.push(d.clone());
                    start.push(d);
                }
            }
        }
        for (task, text) in start {
            let ai = self.ed.ai.clone();
            let recent = recent.clone();
            std::thread::spawn(move || {
                let (t, q) = match task {
                    "inline" => ("inline", text.clone()),
                    "fix" => ("command", format!("This command failed; give the corrected command: {text}")),
                    _ => (task, text.trim_start_matches('#').trim().to_string()),
                };
                let r = crate::ai::ask(t, &q, &recent);
                let mut a = ai.lock().unwrap();
                a.asking.retain(|x| !(x.0 == task && x.1 == text));
                a.answers.push(((task, text), r));
                if a.answers.len() > 50 {
                    a.answers.remove(0);
                }
            });
        }
        // Answers in.
        let answers: Vec<((&'static str, String), Result<String, String>)> = {
            let mut ai = self.ed.ai.lock().unwrap();
            let fresh: Vec<_> = ai.answers.iter().filter(|a| !a.0 .1.starts_with('\u{0}')).cloned().collect();
            // (Each taken once: marked.)
            for a in ai.answers.iter_mut() {
                if !a.0 .1.starts_with('\u{0}') {
                    a.0 .1.insert(0, '\u{0}');
                }
            }
            fresh
        };
        for ((task, text), r) in answers {
            self.ed.dirty = true;
            let busy = self.ed.ai.lock().unwrap().busy.clone();
            match (task, r) {
                ("command" | "fix", r) if !busy.is_empty() && (task == busy || (task == "command" && busy == "command")) => {
                    self.ed.ai.lock().unwrap().busy.clear();
                    match r {
                        Ok(c) if !c.trim().is_empty() => {
                            self.ed.set_text(c.trim());
                            self.ed.ai_note.clear();
                        }
                        Ok(_) => self.ed.ai_note = "the AI had no command for that".into(),
                        Err(e) => self.ed.ai_note = e,
                    }
                }
                ("inline", Ok(reply)) => {
                    let t = self.ed.suggest_base();
                    // (Over what the completions have in common, when it goes on from that.)
                    let free = self.ed.ghost.is_empty() || (self.ed.ghost_comp && reply.get(t.len()..).is_some_and(|r| r.starts_with(&self.ed.ghost)));
                    if t.starts_with(&text) && t.trim().chars().count() >= 2 && self.ed.at_suggest_end() && free && reply.starts_with(&t) && reply.len() > t.len() {
                        self.ed.ghost = reply[t.len()..].lines().next().unwrap_or("").to_string();
                        self.ed.ghost_base = t;
                        self.ed.ghost_comp = false;
                        self.ed.ai_note.clear();
                    }
                }
                ("request", Ok(reply)) => {
                    let t = self.ed.t.text.clone();
                    if t == text && self.ed.t.cursor == t.len() {
                        let body = t.trim_start_matches('#').trim_start().to_string();
                        let r = reply.trim().trim_start_matches('#').trim_start().lines().next().unwrap_or("").to_string();
                        if r.to_lowercase().starts_with(&body.to_lowercase()) && r.len() > body.len() {
                            self.ed.ghost = r[body.len()..].to_string();
                            self.ed.ghost_base = t;
                        }
                    }
                }
                ("command", Ok(cmd)) => {
                    if self.ed.t.text == text && !cmd.trim().is_empty() && (self.ed.ghost.is_empty() || self.ed.ghost.starts_with("  →")) {
                        self.ed.nl_command = cmd.trim().to_string();
                        self.ed.ghost = format!("  → {}", cmd.trim());
                        self.ed.ghost_base = text;
                    }
                }
                (_, Err(e)) if task != "inline" => self.ed.ai_note = format!("AI: {e}"),
                _ => {}
            }
        }
    }

    pub fn read(&mut self, h: &mut dyn Hooks) -> Outcome {
        self.term.enter();
        self.screen.begin();
        self.refresh_bar();
        self.ed.macros = h.macros();
        self.ed.done = None;
        self.ed.dirty = true;
        let mut last_bar = 0;
        loop {
            if self.ed.pending_tap.is_some_and(|p| p.1.elapsed() >= Duration::from_millis(60)) {
                keylog(&format!("  tap fires {:?}", self.ed.pending_tap.map(|p| p.0)));
            }
            self.ed.fire_tap(h);
            {
                let b = self.bar.lock().unwrap().2;
                if b != last_bar {
                    last_bar = b;
                    self.ed.dirty = true;
                }
            }
            if self.ed.dirty {
                self.ed.dirty = false;
                self.draw(h, false);
            }
            let wait = if self.ed.pending_tap.is_some() { 15 } else { 50 };
            let ready = event::poll(Duration::from_millis(wait)).unwrap_or(false);
            if ready {
                match event::read() {
                    Ok(Event::Key(k)) if self.term.late_key(&k) => {
                        keylog(&format!("late answer {:?} {:?}", k.code, k.modifiers));
                        self.ed.dirty = true;
                    }
                    Ok(Event::Key(k)) => {
                        keylog(&format!("tty {:?} {:?} {:?}", k.code, k.modifiers, k.kind));
                        // (Any key: the mouse is the editor's again.)
                        if !self.term.mouse_on {
                            self.term.set_mouse(true);
                        }
                        self.ed.key(k, h);
                    }
                    Ok(Event::Paste(s)) if self.ed.ls_paste(&s) => {}
                    Ok(Event::Paste(s)) => {
                        let s = s.replace("\r\n", "\n").replace('\r', "\n");
                        self.ed.t.insert(&s);
                        self.ed.changed();
                        self.ed.dirty = true;
                    }
                    Ok(Event::Mouse(m)) => {
                        let hit = self.screen.hit(m.column, m.row).cloned();
                        let hov = hit.clone().filter(|x| *x != Hit::None);
                        if hov != self.hovered {
                            self.hovered = hov;
                            self.ed.dirty = true;
                        }
                        let hit = if self.screen.in_popup(m.column, m.row) { Some(hit.unwrap_or_default()) } else { hit.filter(|x| *x != Hit::None || m.row >= self.screen.top) };
                        if matches!(m.kind, crossterm::event::MouseEventKind::Down(_)) && matches!(hit, Some(Hit::Chip(_)) | Some(Hit::Widget(_))) {
                            self.ed.menu_x = Some(self.screen.run_start(m.column, m.row));
                        }
                        // (The listing's own: its rows, a drag begun there, a click while its dialog's open.)
                        let to_ls = self.ed.ls.as_ref().is_some_and(|l| l.dragging() || (l.dialog_open() && matches!(m.kind, crossterm::event::MouseEventKind::Down(_))))
                            || matches!(hit, Some(Hit::Ls(_)));
                        let keep = if to_ls { self.ed.ls_mouse(m, hit) } else { self.ed.mouse(m, hit, h) };
                        if !keep {
                            self.term.set_mouse(false);
                        }
                    }
                    Ok(Event::Resize(c, r)) => {
                        self.screen.resized(c, r);
                        self.refresh_bar();
                        self.ed.dirty = true;
                    }
                    Ok(Event::FocusGained) => {
                        keylog("focus in");
                        self.term.set_mouse(true);
                    }
                    Ok(Event::FocusLost) => {
                        keylog("focus out");
                        self.ed.pending_tap = None;
                        self.ed.shift_side = 0;
                        self.ed.ctrl_side = 0;
                        self.hovered = None;
                        self.ed.tooltip = None;
                        self.ed.dirty = true;
                    }
                    _ => {}
                }
            }
            if self.ed.cmd.expire(Instant::now()) {
                self.ed.dirty = true;
            }
            {
                self.ed.input_idle = !event::poll(Duration::ZERO).unwrap_or(false);
                // (The key shown first: a slow list, git's branches, never
                // holds up what was typed.)
                if self.ed.input_idle && self.ed.comp.typing && self.ed.dirty {
                    self.ed.dirty = false;
                    self.draw(h, false);
                }
                let screen = &self.screen;
                self.ed.tick(h, view::tooltip, |c, r| screen.hit(c, r).cloned());
            }
            self.ai_tick();
            self.ed.ls_tick();
            if let Some(o) = self.ed.done.take() {
                match &o {
                    Outcome::Submit { .. } | Outcome::Interrupt | Outcome::Eof => {
                        self.freeze_ls();
                        self.ed.close_comp();
                        self.ed.menu = None;
                        self.ed.hover = None;
                        self.ed.tooltip = None;
                        self.ed.ghost.clear();
                        self.ed.problem.clear();
                        self.ed.errors.clear();
                        self.draw(h, true);
                        self.term.leave();
                    }
                    Outcome::Host(_) => self.clear(),
                }
                return o;
            }
        }
    }
}

/// A folder as the bar shows it: under home, from ~.
fn shown_folder(p: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    match p.strip_prefix(home.as_str()) {
        Some(rest) if !home.is_empty() && (rest.is_empty() || rest.starts_with('/')) => format!("~{rest}"),
        _ => p.to_string(),
    }
}

/// The bar drawn for folder `old`, with `new` in its place: where `old`
/// shows whole (not cut short by the prompt's settings), the gap the bar
/// stretches (its $fill: the longest run of spaces) taking up the
/// difference so the line stays as wide. None when it can't be done
/// as it would look.
fn swap_folder(lines: &[Line], old: &str, new: &str, cols: usize) -> Option<Vec<Line>> {
    use unicode_width::UnicodeWidthStr;
    if old.is_empty() || lines.is_empty() {
        return None;
    }
    let (from, to) = (shown_folder(old), shown_folder(new));
    let mut out = lines.to_vec();
    for line in out.iter_mut() {
        let text: String = line.iter().map(|s| s.text.as_str()).collect();
        let Some(at) = text.find(&from) else { continue };
        let end = at + from.len();
        if text[end..].starts_with('/') || text[..at].ends_with(|c: char| c.is_alphanumeric() || c == '/') {
            continue;
        }
        // (The spans the folder runs over: its first takes the new one, the
        // rest give up their part of the old.)
        let mut pos = 0;
        let mut put = false;
        for sp in line.iter_mut() {
            let (a, b) = (pos, pos + sp.text.len());
            pos = b;
            if b <= at || a >= end {
                continue;
            }
            let (s0, e0) = (at.max(a) - a, end.min(b) - a);
            let mut t = String::new();
            t.push_str(&sp.text[..s0]);
            if !put {
                t.push_str(&to);
                put = true;
            }
            t.push_str(&sp.text[e0..]);
            sp.text = t;
        }
        line.retain(|sp| !sp.text.is_empty());
        // (The line as wide as it was: the stretched gap shrinks or grows.)
        let d = to.width() as isize - from.width() as isize;
        let gap = line.iter().enumerate().filter_map(|(i, sp)| {
            let t = &sp.text;
            let mut best: Option<(usize, usize)> = None;
            let mut run_start = None;
            for (j, c) in t.char_indices().chain(std::iter::once((t.len(), 'x'))) {
                if c == ' ' {
                    run_start.get_or_insert(j);
                } else if let Some(st) = run_start.take() {
                    if best.is_none_or(|(bs, be)| j - st > be - bs) {
                        best = Some((st, j));
                    }
                }
            }
            best.map(|(st, en)| (i, st, en))
        }).max_by_key(|(_, st, en)| en - st);
        match gap {
            Some((i, st, en)) if en - st >= 2 => {
                let n = en - st;
                let want = n as isize - d;
                if want < 1 {
                    return None;
                }
                let t = &line[i].text;
                line[i].text = format!("{}{}{}", &t[..st], " ".repeat(want as usize), &t[en..]);
            }
            _ => {
                if super::width(line) > cols {
                    return None;
                }
            }
        }
        return Some(out);
    }
    None
}

#[cfg(test)]
mod swap_tests {
    use super::*;
    use crate::editor::{Span, Sty};
    #[test]
    fn swaps_the_folder_and_keeps_the_width() {
        let home = std::env::var("HOME").unwrap_or_default();
        let l: Line = vec![Span::new(" ~/a/My Drive ", Sty::default()), Span::new("          ", Sty::default()), Span::new(" v24 ", Sty::default())];
        let w = crate::editor::width(&l);
        let out = swap_folder(&[l], &format!("{home}/a/My Drive"), &format!("{home}/a/My Drive/health"), 80).unwrap();
        let t: String = out[0].iter().map(|s| s.text.as_str()).collect();
        assert!(t.contains("~/a/My Drive/health "), "{t:?}");
        assert_eq!(crate::editor::width(&out[0]), w);
        assert!(swap_folder(&out, &format!("{home}/elsewhere"), "/x", 80).is_none());
    }
}
