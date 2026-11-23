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
    bar: Arc<Mutex<(Vec<Line>, Line, u64)>>,
    bar_seq: u64,
    hovered: Option<Hit>,
    pub mode_name: Box<dyn Fn(&Ed) -> String>,
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
        Editor { term, screen: Screen::new(), ed, bar_fn, bar: Arc::new(Mutex::new((vec![], vec![], 0))), bar_seq: 0, hovered: None, mode_name: Box::new(|_| "INSERT".into()) }
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
        let (cwd, st, du, w, visited) = (self.ed.cwd.clone(), self.ed.status, self.ed.duration_ms, Term::size().0, self.ed.visited.clone());
        let colors = self.term.fg.zip(self.term.bg);
        std::thread::spawn(move || {
            let (l, r) = f(&cwd, st, du, w, &visited, colors);
            let mut s = slot.lock().unwrap();
            if seq >= s.2 {
                *s = (l, r, seq);
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
        let built = view::build(&self.ed, &v, &spans_for);
        if final_ {
            self.screen.finish(&built.frame);
        } else {
            self.screen.draw(&built.frame);
        }
    }

    /// The editor's rows cleared (something's printed where it was; the next
    /// read draws it again under that).
    pub fn clear(&mut self) {
        use std::io::Write;
        let mut o = std::io::stdout();
        let _ = write!(o, "\x1b[{};1H\x1b[J", self.screen.top + 1);
        let _ = o.flush();
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
                    if t.starts_with(&text) && t.trim().chars().count() >= 2 && self.ed.at_suggest_end() && self.ed.ghost.is_empty() && reply.starts_with(&t) && reply.len() > t.len() {
                        self.ed.ghost = reply[t.len()..].lines().next().unwrap_or("").to_string();
                        self.ed.ghost_base = t;
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
            let ready = event::poll(Duration::from_millis(50)).unwrap_or(false);
            if ready {
                match event::read() {
                    Ok(Event::Key(k)) => {
                        // (Any key: the mouse is the editor's again.)
                        if !self.term.mouse_on {
                            self.term.set_mouse(true);
                        }
                        self.ed.key(k, h);
                    }
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
                        let hit = hit.filter(|x| *x != Hit::None || m.row >= self.screen.top);
                        let keep = self.ed.mouse(m, hit, h);
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
                        self.term.set_mouse(true);
                    }
                    Ok(Event::FocusLost) => {
                        self.hovered = None;
                        self.ed.tooltip = None;
                        self.ed.dirty = true;
                    }
                    _ => {}
                }
            }
            {
                let screen = &self.screen;
                self.ed.tick(h, view::tooltip, |c, r| screen.hit(c, r).cloned());
            }
            self.ai_tick();
            if let Some(o) = self.ed.done.take() {
                match &o {
                    Outcome::Submit { .. } | Outcome::Interrupt | Outcome::Eof => {
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
