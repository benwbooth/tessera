//! The guide, full screen, as Tessera's guide tab has it: a search box on
//! top (searching as you type, "3 of 12"), the contents down the left (each
//! part a click away, the one being read highlighted), the guide beside it,
//! laid out for the room it has. On the terminal's other screen: what was
//! there comes back after. Keys: / or a click on the box to search, Enter /
//! n / N for the next and previous match, [ ] for the parts, the arrows,
//! PgUp PgDn, g G, the wheel; q or Esc goes back.

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use std::io::Write;
use unicode_width::UnicodeWidthStr;

fn strip(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            if it.peek() == Some(&'[') {
                it.next();
                while let Some(&n) = it.peek() {
                    it.next();
                    if n.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else if it.peek() == Some(&']') {
                while let Some(n) = it.next() {
                    if n == '\x07' {
                        break;
                    }
                    if n == '\x1b' {
                        it.next();
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn cut(s: &str, w: usize) -> String {
    let mut out = String::new();
    let mut n = 0;
    for c in s.chars() {
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if n + cw > w {
            break;
        }
        n += cw;
        out.push(c);
    }
    out
}

struct Part {
    level: usize,
    title: String,
    line: usize,
}

struct Guide {
    md: String,
    /// The guides there are (title, Markdown), the one shown, and where
    /// each was left (the bundled guide: tessh's and the Advanced one).
    guides: Vec<(&'static str, String)>,
    which: usize,
    tops: Vec<usize>,
    /// The examples showing their bash version (by number, per guide); the
    /// others show tessh's. And the laid-out lines that are tab bars.
    bash: Vec<std::collections::HashSet<usize>>,
    tab_lines: Vec<(usize, usize)>,
    /// The guide chips on the top line (start and end column, guide).
    chips: Vec<(usize, usize, usize)>,
    width: usize,
    lines: Vec<String>,
    plain: Vec<String>,
    lower: Vec<String>,
    parts: Vec<Part>,
    top: usize,
    toc_top: usize,
    query: String,
    searching: bool,
    matches: Vec<usize>,
    at: Option<usize>,
    /// Text picked with the mouse: (line, char) where the drag began and
    /// where it is now, in the laid-out lines.
    sel: Option<((usize, usize), (usize, usize))>,
    dragging: bool,
    /// The right-click menu: where (column, row), and the item pointed at.
    menu: Option<(usize, usize, usize)>,
}

const MENU: [&str; 2] = ["Copy", "Select All"];

/// An example's tab bar as text: "tessh" at columns 2–8, "bash (original)"
/// at 11–27.
const TAB_BAR: &str = "   tessh    bash (original) ";

impl Guide {
    fn new(md: &str) -> Guide {
        Guide::of(vec![("", md.to_string())], 0)
    }

    fn of(guides: Vec<(&'static str, String)>, which: usize) -> Guide {
        let n = guides.len();
        let md = guides[which].1.clone();
        let mut g = Guide { md, guides, which, tops: vec![0; n], bash: vec![Default::default(); n], tab_lines: vec![], chips: vec![], width: 0, lines: vec![], plain: vec![], lower: vec![], parts: vec![], top: 0, toc_top: 0, query: String::new(), searching: false, matches: vec![], at: None, sel: None, dragging: false, menu: None };
        g.layout(80);
        g
    }

    /// Show guide `w`, where it was left.
    fn switch(&mut self, w: usize) {
        if w == self.which || w >= self.guides.len() {
            return;
        }
        self.tops[self.which] = self.top;
        self.which = w;
        self.md = self.guides[w].1.clone();
        self.top = self.tops[w];
        self.toc_top = 0;
        self.relayout(false);
    }

    /// Example `n`'s tab: its bash version or tessh's, the guide where it was.
    fn show_tab(&mut self, n: usize, bash: bool) {
        let set = &mut self.bash[self.which];
        if bash == set.contains(&n) {
            return;
        }
        if bash {
            set.insert(n);
        } else {
            set.remove(&n);
        }
        self.relayout(true);
    }

    /// Laid out again at the same width; `stay`: at the same line, not the
    /// part's start.
    fn relayout(&mut self, stay: bool) {
        let (w, top) = (self.width, self.top);
        self.width = 0;
        self.layout(w);
        if stay {
            self.top = top;
        }
    }

    /// Laid out for `width` columns: the lines, and where each part starts.
    fn layout(&mut self, width: usize) {
        if width == self.width {
            return;
        }
        let keep = self.parts.iter().rev().find(|p| p.line <= self.top).map(|p| p.title.clone());
        self.width = width;
        // (Laid out again, the lines move: a pick no longer means the same text.)
        self.sel = None;
        // (Each example's two versions as tabs, the one picked shown.)
        let set = &self.bash[self.which];
        let md = if self.md.contains("```bash") { crate::help::tab_examples(&self.md, &|n| set.contains(&n)).0 } else { self.md.clone() };
        let text = crate::help::terminal_at(&md, true, width.clamp(20, 100));
        self.lines = text.lines().map(String::from).collect();
        self.plain = self.lines.iter().map(|l| strip(l)).collect();
        self.tab_lines.clear();
        for i in 0..self.plain.len() {
            let Some(k) = self.plain[i].find(crate::help::TAB_MARK) else { continue };
            let Ok(n) = self.plain[i][k + crate::help::TAB_MARK.len()..].trim().parse::<usize>() else { continue };
            let (t, b) = if set.contains(&n) { ("\x1b[2m", "\x1b[7;1m") } else { ("\x1b[7;1m", "\x1b[2m") };
            self.lines[i] = format!("  {t} tessh \x1b[0m  {b} bash (original) \x1b[0m");
            self.plain[i] = TAB_BAR.to_string();
            self.tab_lines.push((i, n));
        }
        self.lower = self.plain.iter().map(|l| l.to_lowercase()).collect();
        // The parts: the Markdown's headings, found in the laid-out lines in order.
        self.parts.clear();
        let mut from = 0;
        let mut fence = false;
        for l in md.lines() {
            if l.trim_start().starts_with("```") {
                fence = !fence;
            }
            if fence || !l.starts_with('#') {
                continue;
            }
            let level = l.chars().take_while(|c| *c == '#').count();
            let title = l[level..].trim().replace('`', "");
            if title.is_empty() || level > 3 {
                continue;
            }
            if let Some(i) = (from..self.plain.len()).find(|&i| self.plain[i].trim().trim_matches(|c: char| !c.is_alphanumeric() && c != ')' && c != '(').contains(title.trim_matches(|c: char| !c.is_alphanumeric()))) {
                self.parts.push(Part { level, title, line: i });
                from = i + 1;
            }
        }
        if let Some(t) = keep {
            if let Some(p) = self.parts.iter().find(|p| p.title == t) {
                self.top = p.line;
            }
        }
        self.search();
    }

    fn search(&mut self) {
        let q = self.query.to_lowercase();
        self.matches = if q.is_empty() { vec![] } else { (0..self.lines.len()).filter(|&i| self.lower[i].contains(&q)).collect() };
        self.at = None;
    }
    /// The next (or previous) match from the top line, shown.
    fn step(&mut self, back: bool, include_top: bool) {
        if self.matches.is_empty() {
            return;
        }
        let t = self.top;
        let i = if back {
            self.matches.iter().rposition(|&m| m < t).unwrap_or(self.matches.len() - 1)
        } else {
            self.matches.iter().position(|&m| if include_top { m >= t } else { m > t }).unwrap_or(0)
        };
        self.at = Some(i);
        self.top = self.matches[i].saturating_sub(2);
    }
    fn current_part(&self) -> Option<usize> {
        self.parts.iter().rposition(|p| p.line <= self.top + 1)
    }

    /// The pick in order, start first.
    fn sel_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let (a, b) = self.sel?;
        if a == b {
            return None;
        }
        Some(if a <= b { (a, b) } else { (b, a) })
    }
    /// The text picked, lines joined with newlines.
    fn sel_text(&self) -> String {
        let Some(((l0, c0), (l1, c1))) = self.sel_range() else { return String::new() };
        let mut out = vec![];
        for i in l0..=l1.min(self.plain.len().saturating_sub(1)) {
            let chars: Vec<char> = self.plain[i].chars().collect();
            let from = if i == l0 { c0.min(chars.len()) } else { 0 };
            let to = if i == l1 { c1.min(chars.len()) } else { chars.len() };
            out.push(chars[from..to.max(from)].iter().collect::<String>().trim_end().to_string());
        }
        out.join("\n")
    }
    /// The (line, char) under a cell of the guide's text, given its left edge.
    fn pos_at(&self, col: usize, row: usize, x0: usize, rows: usize) -> (usize, usize) {
        let body = rows.saturating_sub(2);
        let r = row.clamp(1, body.max(1)) - 1;
        let i = (self.top + r).min(self.lines.len().saturating_sub(1));
        let want = col.saturating_sub(x0);
        let mut w = 0;
        let mut n = 0;
        for c in self.plain.get(i).map(String::as_str).unwrap_or("").chars() {
            let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if w + cw > want {
                break;
            }
            w += cw;
            n += 1;
        }
        (i, n)
    }
    fn copy(&mut self) {
        let t = self.sel_text();
        if !t.is_empty() {
            super::term::Term::copy(&t);
        }
    }

    fn sidebar_w(cols: usize) -> usize {
        if cols < 70 {
            0
        } else {
            (cols / 4).clamp(20, 34)
        }
    }

    fn draw(&mut self, cols: usize, rows: usize) {
        let sw = Self::sidebar_w(cols);
        let cw = cols.saturating_sub(sw + if sw > 0 { 2 } else { 0 });
        self.layout(cw.saturating_sub(1));
        let body = rows.saturating_sub(2);
        self.top = self.top.min(self.lines.len().saturating_sub(body));
        let mut o = std::io::stdout().lock();
        // (One frame, synchronized output: shown whole, no flicker.)
        let _ = write!(o, "\x1b[?2026h\x1b[?25l");
        // The search box.
        let count = if self.query.is_empty() {
            String::new()
        } else if self.matches.is_empty() {
            "no matches  ".into()
        } else {
            match self.at {
                Some(i) => format!("{} of {}  ", i + 1, self.matches.len()),
                None => format!("{} matches  ", self.matches.len()),
            }
        };
        let field = if self.query.is_empty() && !self.searching { "\x1b[2mSearch the guide (/)\x1b[22m".to_string() } else { self.query.clone() };
        // (More than one guide: a chip for each at the right, the one shown lit; Tab goes round.)
        let chips: Vec<String> = if self.guides.len() > 1 { self.guides.iter().map(|(t, _)| format!(" {t} ")).collect() } else { vec![] };
        let chips_w: usize = chips.iter().map(|c| c.width() + 1).sum();
        let fw = cols.saturating_sub(6 + count.width() + chips_w);
        let _ = write!(o, "\x1b[1;1H\x1b[0m\x1b[2K \x1b[7m ⌕ {}{}\x1b[27m {}", cut(&field, fw), " ".repeat(fw.saturating_sub(strip(&field).width())), count);
        self.chips.clear();
        let mut x = cols.saturating_sub(chips_w);
        for (k, c) in chips.iter().enumerate() {
            let sty = if k == self.which { "\x1b[7;1m" } else { "\x1b[2m" };
            let _ = write!(o, "\x1b[1;{}H{sty}{c}\x1b[0m", x + 1);
            self.chips.push((x, x + c.width(), k));
            x += c.width() + 1;
        }
        // The contents, and the guide beside them.
        let cur = self.current_part();
        if let Some(c) = cur {
            if c < self.toc_top {
                self.toc_top = c;
            } else if c >= self.toc_top + body {
                self.toc_top = c + 1 - body;
            }
        }
        let q = self.query.to_lowercase();
        for r in 0..body {
            let _ = write!(o, "\x1b[{};1H\x1b[0m\x1b[2K", r + 2);
            if sw > 0 {
                if let Some((i, p)) = self.parts.iter().enumerate().nth(self.toc_top + r) {
                    let ind = "  ".repeat(p.level.saturating_sub(1));
                    let t = cut(&format!(" {ind}{}", p.title), sw);
                    let pad = " ".repeat(sw.saturating_sub(t.width()));
                    if Some(i) == cur {
                        let _ = write!(o, "\x1b[7m{t}{pad}\x1b[27m");
                    } else if p.level == 1 {
                        let _ = write!(o, "\x1b[1m{t}\x1b[22m{pad}");
                    } else {
                        let _ = write!(o, "{t}{pad}");
                    }
                } else {
                    let _ = write!(o, "{}", " ".repeat(sw));
                }
                let _ = write!(o, "\x1b[2m│\x1b[22m ");
            }
            let i = self.top + r;
            let Some(l) = self.lines.get(i) else { continue };
            let picked = self.sel_range().filter(|((l0, _), (l1, _))| (*l0..=*l1).contains(&i));
            if let Some(((l0, c0), (l1, c1))) = picked {
                // (A line in the pick: the picked part reversed.)
                let chars: Vec<char> = self.plain[i].chars().collect();
                let from = if i == l0 { c0.min(chars.len()) } else { 0 };
                let to = if i == l1 { c1.min(chars.len()) } else { chars.len() };
                let (a, b, c): (String, String, String) =
                    (chars[..from].iter().collect(), chars[from..to.max(from)].iter().collect(), chars[to.max(from)..].iter().collect());
                let b = if b.is_empty() && i != l1 { " ".to_string() } else { b };
                let _ = write!(o, "{a}\x1b[7m{b}\x1b[27m{c}");
            } else if !q.is_empty() && self.lower[i].contains(&q) {
                // (A line with a match: the matches reversed.)
                let (p, lp) = (&self.plain[i], &self.lower[i]);
                let mut at = 0;
                let mut out = String::new();
                while let Some(k) = lp[at..].find(&q) {
                    let s = at + k;
                    if !p.is_char_boundary(s) || !p.is_char_boundary(s + q.len()) {
                        break;
                    }
                    out.push_str(&p[at..s]);
                    out.push_str("\x1b[7m");
                    out.push_str(&p[s..s + q.len()]);
                    out.push_str("\x1b[27m");
                    at = s + q.len();
                }
                out.push_str(&p[at..]);
                let _ = write!(o, "{out}");
            } else {
                let _ = write!(o, "{l}");
            }
        }
        let total = self.lines.len().max(1);
        let pct = ((self.top + body).min(total) * 100) / total;
        let more = if self.guides.len() > 1 { "   Tab other guide" } else { "" };
        let tabs = if self.tab_lines.is_empty() { "" } else { "   t bash/tessh" };
        let hint = format!(" {pct}%   / search   Enter n N next   [ ] parts{more}{tabs}   ↑↓ PgUp PgDn   y copies the part (or drag, right click)   q back");
        let _ = write!(o, "\x1b[{};1H\x1b[0m\x1b[2K\x1b[2m{}\x1b[0m", rows, cut(&hint, cols));
        // The right-click menu, over the guide.
        if let Some((mx, my, at)) = self.menu {
            let w = 14;
            let (x, y) = (mx.min(cols.saturating_sub(w + 2)), my.min(rows.saturating_sub(MENU.len() + 2)));
            let _ = write!(o, "\x1b[{};{}H\x1b[0m╭{}╮", y + 1, x + 1, "─".repeat(w));
            for (k, item) in MENU.iter().enumerate() {
                let off = k == 0 && self.sel_range().is_none();
                let label = format!(" {item:<w$}", w = w - 1);
                let sty = if off { "\x1b[2m" } else if k == at { "\x1b[7m" } else { "" };
                let _ = write!(o, "\x1b[{};{}H│{sty}{label}\x1b[0m│", y + 2 + k, x + 1);
            }
            let _ = write!(o, "\x1b[{};{}H╰{}╯", y + 2 + MENU.len(), x + 1, "─".repeat(w));
        }
        if self.searching {
            let _ = write!(o, "\x1b[1;{}H\x1b[?25h", 5 + self.query.width());
        }
        let _ = write!(o, "\x1b[?2026l");
        let _ = o.flush();
    }
}

/// A right-click menu item: Copy, Select All.
fn run_item(g: &mut Guide, i: usize) {
    match i {
        0 => g.copy(),
        _ => {
            let last = g.plain.len().saturating_sub(1);
            g.sel = Some(((0, 0), (last, g.plain.get(last).map(|l| l.chars().count()).unwrap_or(0))));
        }
    }
}

/// The bundled guides full screen, tessh's and the Advanced tessh
/// Scripting Guide (`which`: 0 or 1), at the part about `at`, until q.
pub fn show_guides(at: &str, which: usize) {
    run(Guide::of(vec![("tessh guide", crate::help::guide()), ("Advanced guide", crate::help::advanced_guide())], which), at);
}

/// Show the guide (or any Markdown) full screen, at the part about `at`
/// when there is one, until q.
pub fn show(md: &str, at: &str) {
    run(Guide::new(md), at);
}

fn run(mut g: Guide, at: &str) {
    let mut o = std::io::stdout();
    let _ = crossterm::terminal::enable_raw_mode();
    let _ = write!(o, "\x1b[?1049h\x1b[?1002h\x1b[?1006h\x1b[H\x1b[2J");
    let _ = o.flush();
    let (c0, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let sw = Guide::sidebar_w(c0 as usize);
    g.layout((c0 as usize).saturating_sub(sw + if sw > 0 { 3 } else { 1 }));
    let at = at.trim();
    if !at.is_empty() {
        let w = at.to_lowercase();
        if let Some(p) = g.parts.iter().find(|p| p.title.to_lowercase().split(|c: char| !c.is_alphanumeric()).any(|x| x == w)) {
            g.top = p.line;
        } else {
            g.query = at.to_string();
            g.search();
            g.step(false, true);
        }
    }
    loop {
        let (c, r) = crossterm::terminal::size().unwrap_or((80, 24));
        let (cols, rows) = (c as usize, r as usize);
        g.draw(cols, rows);
        let page = rows.saturating_sub(3).max(1);
        let Ok(ev) = event::read() else { break };
        match ev {
            Event::Key(k) if k.kind != event::KeyEventKind::Release => {
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                if let Some((x, y, at)) = g.menu {
                    match k.code {
                        KeyCode::Up => g.menu = Some((x, y, at.saturating_sub(1))),
                        KeyCode::Down => g.menu = Some((x, y, (at + 1).min(MENU.len() - 1))),
                        KeyCode::Enter => {
                            g.menu = None;
                            run_item(&mut g, at);
                        }
                        _ => g.menu = None,
                    }
                    continue;
                }
                // (Ctrl+C with text picked copies it; with none, goes back.)
                if ctrl && matches!(k.code, KeyCode::Char('c')) && g.sel_range().is_some() {
                    g.copy();
                    continue;
                }
                // (Without the mouse: y copies the part being read, Ctrl+A
                // picks it all, Shift+F10 or the Menu key opens the menu.)
                if !g.searching && k.code == KeyCode::Char('y') && !ctrl {
                    if let Some(i) = g.current_part() {
                        let a = g.parts[i].line;
                        let b = g.parts.get(i + 1).map(|p| p.line).unwrap_or(g.plain.len()).max(a + 1) - 1;
                        g.sel = Some(((a, 0), (b, g.plain.get(b).map(|l| l.chars().count()).unwrap_or(0))));
                        g.copy();
                    }
                    continue;
                }
                if ctrl && matches!(k.code, KeyCode::Char('a')) && !g.searching {
                    run_item(&mut g, 1);
                    continue;
                }
                if k.code == KeyCode::Menu || (k.code == KeyCode::F(10) && k.modifiers.contains(KeyModifiers::SHIFT)) {
                    g.menu = Some((2, 2, 0));
                    continue;
                }
                if k.code == KeyCode::Esc && !g.searching && g.sel.is_some() {
                    g.sel = None;
                    continue;
                }
                if g.searching {
                    match k.code {
                        KeyCode::Enter => {
                            // (Enter ends the typing, as in less: then n and N
                            // go to the next and previous match.)
                            // (Typing found the first already; Enter stays on it.)
                            if g.at.is_none() {
                                g.step(k.modifiers.contains(KeyModifiers::SHIFT), true);
                            }
                            g.searching = false;
                        }
                        KeyCode::Esc => g.searching = false,
                        KeyCode::Down => g.step(false, false),
                        KeyCode::Up => g.step(true, false),
                        KeyCode::Backspace => {
                            g.query.pop();
                            g.search();
                            g.step(false, true);
                        }
                        KeyCode::Char('u') if ctrl => {
                            g.query.clear();
                            g.search();
                        }
                        KeyCode::Char('c') if ctrl => break,
                        KeyCode::Char(ch) if !ctrl => {
                            // (As you type: the first match from where you are.)
                            g.query.push(ch);
                            g.search();
                            g.step(false, true);
                        }
                        _ => {}
                    }
                    continue;
                }
                match k.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if ctrl => break,
                    KeyCode::Tab | KeyCode::BackTab if g.guides.len() > 1 => {
                        let n = g.guides.len();
                        let w = if k.code == KeyCode::BackTab { (g.which + n - 1) % n } else { (g.which + 1) % n };
                        g.switch(w);
                    }
                    // (Every example to its bash version, or all back to tessh's.)
                    KeyCode::Char('t') if !g.tab_lines.is_empty() => {
                        let w = g.which;
                        if g.bash[w].is_empty() {
                            let all: Vec<usize> = g.tab_lines.iter().map(|&(_, n)| n).collect();
                            g.bash[w].extend(all);
                        } else {
                            g.bash[w].clear();
                        }
                        g.relayout(true);
                    }
                    KeyCode::Down | KeyCode::Char('j') => g.top += 1,
                    KeyCode::Up | KeyCode::Char('k') => g.top = g.top.saturating_sub(1),
                    KeyCode::PageDown | KeyCode::Char(' ') | KeyCode::Char('f') => g.top += page,
                    KeyCode::Char('d') => g.top += page / 2,
                    KeyCode::PageUp | KeyCode::Char('b') => g.top = g.top.saturating_sub(page),
                    KeyCode::Char('u') => g.top = g.top.saturating_sub(page / 2),
                    KeyCode::Home | KeyCode::Char('g') => g.top = 0,
                    KeyCode::End | KeyCode::Char('G') => g.top = usize::MAX / 2,
                    KeyCode::Char('/') => {
                        g.searching = true;
                    }
                    KeyCode::Char('n') | KeyCode::Enter => g.step(false, false),
                    KeyCode::Char('N') => g.step(true, false),
                    KeyCode::Char(']') => {
                        if let Some(p) = g.parts.iter().find(|p| p.line > g.top + 1) {
                            g.top = p.line;
                        }
                    }
                    KeyCode::Char('[') => {
                        let cur = g.current_part();
                        let prev = match cur {
                            Some(i) if g.parts[i].line + 1 < g.top => Some(i),
                            Some(i) => i.checked_sub(1),
                            None => None,
                        };
                        if let Some(i) = prev {
                            g.top = g.parts[i].line;
                        }
                    }
                    _ => {}
                }
            }
            Event::Mouse(m) => {
                let sw = Guide::sidebar_w(cols);
                let over_toc = sw > 0 && (m.column as usize) < sw && m.row >= 1;
                let x0 = if sw > 0 { sw + 2 } else { 0 };
                let in_text = !over_toc && m.row >= 1 && (m.row as usize) <= rows.saturating_sub(2);
                if let Some((x, y, _)) = g.menu {
                    // (The menu open: a click on an item does it, anywhere else closes it.)
                    let (w, col, row) = (14usize, m.column as usize, m.row as usize);
                    let (x, y) = (x.min(cols.saturating_sub(w + 2)), y.min(rows.saturating_sub(MENU.len() + 2)));
                    let item = (row > y && row <= y + MENU.len() && col > x && col <= x + w).then(|| row - y - 1);
                    match m.kind {
                        MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                            if let (Some(i), Some(mm)) = (item, g.menu.as_mut()) {
                                mm.2 = i;
                            }
                        }
                        MouseEventKind::Down(_) => {
                            g.menu = None;
                            if let Some(i) = item {
                                run_item(&mut g, i);
                            }
                        }
                        _ => {}
                    }
                    continue;
                }
                // (A click on an example's tab bar shows that version.)
                if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) && in_text {
                    let line = g.top + m.row as usize - 1;
                    if let Some(&(_, n)) = g.tab_lines.iter().find(|&&(l, _)| l == line) {
                        let c = (m.column as usize).saturating_sub(x0);
                        if (2..9).contains(&c) {
                            g.show_tab(n, false);
                        } else if (11..28).contains(&c) {
                            g.show_tab(n, true);
                        }
                        continue;
                    }
                }
                // (A guide chip on the top line.)
                if matches!(m.kind, MouseEventKind::Down(MouseButton::Left)) && m.row == 0 {
                    if let Some(&(_, _, w)) = g.chips.iter().find(|&&(a, b, _)| (a..b).contains(&(m.column as usize))) {
                        g.switch(w);
                        continue;
                    }
                }
                match m.kind {
                    MouseEventKind::Down(MouseButton::Left) if in_text => {
                        let p = g.pos_at(m.column as usize, m.row as usize, x0, rows);
                        g.sel = Some((p, p));
                        g.dragging = true;
                        g.searching = false;
                    }
                    MouseEventKind::Drag(MouseButton::Left) if g.dragging => {
                        // (Past the top or bottom: the guide scrolls along.)
                        if m.row == 0 {
                            g.top = g.top.saturating_sub(1);
                        } else if m.row as usize >= rows.saturating_sub(1) {
                            g.top += 1;
                        }
                        let p = g.pos_at(m.column as usize, m.row as usize, x0, rows);
                        if let Some(s) = g.sel.as_mut() {
                            s.1 = p;
                        }
                    }
                    MouseEventKind::Up(MouseButton::Left) => g.dragging = false,
                    MouseEventKind::Down(MouseButton::Right) if in_text => {
                        // (Outside the pick: nothing picked, so Copy is off.)
                        let p = g.pos_at(m.column as usize, m.row as usize, x0, rows);
                        if g.sel_range().is_some_and(|(a, b)| p < a || p > b) {
                            g.sel = None;
                        }
                        g.menu = Some((m.column as usize, m.row as usize, if g.sel_range().is_some() { 0 } else { 1 }));
                    }
                    MouseEventKind::ScrollDown if over_toc => g.toc_top = (g.toc_top + 3).min(g.parts.len().saturating_sub(1)),
                    MouseEventKind::ScrollUp if over_toc => g.toc_top = g.toc_top.saturating_sub(3),
                    MouseEventKind::ScrollDown => g.top += 3,
                    MouseEventKind::ScrollUp => g.top = g.top.saturating_sub(3),
                    MouseEventKind::Down(MouseButton::Left) => {
                        if m.row == 0 {
                            g.searching = true;
                        } else if over_toc && (m.row as usize) <= rows.saturating_sub(2) {
                            let i = g.toc_top + m.row as usize - 1;
                            if let Some(p) = g.parts.get(i) {
                                g.top = p.line;
                                g.searching = false;
                            }
                        } else {
                            g.searching = false;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    let _ = write!(o, "\x1b[?1006l\x1b[?1002l\x1b[?1049l\x1b[?25h");
    let _ = o.flush();
    let _ = crossterm::terminal::disable_raw_mode();
}
