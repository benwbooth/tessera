//! The guide, full screen, as Tessera's guide tab has it: a search box on
//! top (searching as you type, "3 of 12"), the contents down the left (each
//! part a click away, the one being read highlighted), the guide beside it,
//! laid out for the room it has. On the terminal's other screen: what was
//! there comes back after. Keys: / or a click on the box to search, Enter /
//! n / N for the next and previous match, [ ] for the parts, the arrows,
//! PgUp PgDn, g G, the wheel; q or Esc goes back.
//!
//! The file viewer (`crate::viewer`) shows a file in it too: its views
//! (rendered, source, …) as the chips the guides have, the lines it laid out
//! (a line-number gutter, ← → when they're wider than the screen, w to wrap
//! them), and the file's name and kind at the foot.

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
use std::io::Write;
use unicode_width::UnicodeWidthStr;

pub(crate) fn strip(s: &str) -> String {
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

pub(crate) fn cut(s: &str, w: usize) -> String {
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

/// The columns `from` to `from + w` of a line with ANSI styles: the styles
/// before `from` still apply, and it ends with them reset.
pub(crate) fn slice(s: &str, from: usize, w: usize) -> String {
    let mut out = String::new();
    let mut col = 0;
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c == '\x1b' {
            // (An escape: kept, wherever it is, so the colors carry on.)
            let mut e = String::from(c);
            if it.peek() == Some(&'[') {
                e.push(it.next().unwrap());
                while let Some(n) = it.next() {
                    e.push(n);
                    if n.is_ascii_alphabetic() || n == '~' {
                        break;
                    }
                }
            } else if it.peek() == Some(&']') {
                while let Some(n) = it.next() {
                    e.push(n);
                    if n == '\x07' {
                        break;
                    }
                    if n == '\x1b' {
                        if let Some(x) = it.next() {
                            e.push(x);
                        }
                        break;
                    }
                }
            }
            out.push_str(&e);
            continue;
        }
        let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if col >= from && col + cw <= from + w {
            out.push(c);
        } else if col < from && col + cw > from {
            // (A wide char cut in half at the left edge: a space for its right half.)
            out.push(' ');
        }
        col += cw;
        if col >= from + w && it.peek() != Some(&'\x1b') {
            break;
        }
    }
    out.push_str("\x1b[0m");
    out
}

/// Text the viewer laid out: its lines (with styles), the columns of each
/// line that are a gutter (line numbers: never picked, kept in place when
/// the text scrolls sideways), whether long lines wrap, and its parts
/// (level, title, line) for the contents column.
#[derive(Clone, Default)]
pub(crate) struct Lines {
    pub lines: Vec<String>,
    pub gutter: usize,
    pub wrap: bool,
    pub parts: Vec<(usize, String, usize)>,
}

/// What a guide or a view of a file shows: Markdown (laid out for the room
/// there is) or lines laid out already.
#[derive(Clone)]
pub(crate) enum Content {
    Markdown(String),
    Lines(Lines),
}

struct Part {
    level: usize,
    title: String,
    line: usize,
}

pub(crate) struct Guide {
    /// The guides there are (title, Markdown), the one shown, and where
    /// each was left (the bundled guide: tessh's and the Advanced one); a
    /// file's views (rendered, source…).
    guides: Vec<(String, Content)>,
    which: usize,
    /// The search box's hint ("the guide", a file's name), and what the
    /// foot says first (a file's name, kind and size).
    name: String,
    info: String,
    /// A file shown: o opens it in its own app.
    file: Option<std::path::PathBuf>,
    /// How far the lines are scrolled sideways (unwrapped lines only).
    left: usize,
    /// Each laid-out line's gutter width (0 for Markdown's but a code
    /// block's), the widest line, and the source line each laid-out one is.
    gutters: Vec<usize>,
    widest: usize,
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
    /// The scrollbar's thumb being dragged: the row in the thumb held.
    thumb_drag: Option<usize>,
}

const MENU: [&str; 3] = ["Copy", "Select All", "Copy all"];

/// The contents column's width as dragged (0: as wide as suits the terminal).
static SIDE_W: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// An example's tab bar as text: "tessh" at columns 2–8, "bash (original)"
/// at 11–27.
const TAB_BAR: &str = "   tessh    bash (original) ";

impl Guide {
    fn new(md: &str) -> Guide {
        Guide::of(vec![("", md.to_string())], 0)
    }

    fn of(guides: Vec<(&'static str, String)>, which: usize) -> Guide {
        Guide::views(guides.into_iter().map(|(t, md)| (t.to_string(), Content::Markdown(md))).collect(), which, "the guide", "")
    }

    /// Views of something (one chip each when there's more than one): the
    /// search box says "Search `name`", the foot starts with `info`.
    pub(crate) fn views(guides: Vec<(String, Content)>, which: usize, name: &str, info: &str) -> Guide {
        let n = guides.len();
        let mut g = Guide { guides, which, name: name.to_string(), info: info.to_string(), file: None, left: 0, gutters: vec![], widest: 0, tops: vec![0; n], bash: vec![Default::default(); n], tab_lines: vec![], chips: vec![], width: 0, lines: vec![], plain: vec![], lower: vec![], parts: vec![], top: 0, toc_top: 0, query: String::new(), searching: false, matches: vec![], at: None, sel: None, dragging: false, menu: None, thumb_drag: None };
        g.layout(80);
        g
    }

    /// A file's views: o opens the file in its own app.
    pub(crate) fn with_file(mut self, p: &std::path::Path) -> Guide {
        self.file = Some(p.to_path_buf());
        self
    }

    fn content(&self) -> &Content {
        &self.guides[self.which].1
    }
    /// The view shown is lines that don't wrap: ← → scroll them.
    fn unwrapped(&self) -> bool {
        matches!(self.content(), Content::Lines(l) if !l.wrap)
    }
    /// Whether there are parts to list (the contents column shows then).
    fn has_parts(&self) -> bool {
        match self.content() {
            Content::Markdown(md) => md.lines().any(|l| l.starts_with("# ") || l.starts_with("## ") || l.starts_with("### ")),
            Content::Lines(l) => !l.parts.is_empty(),
        }
    }

    /// Show guide `w`, where it was left.
    fn switch(&mut self, w: usize) {
        if w == self.which || w >= self.guides.len() {
            return;
        }
        self.tops[self.which] = self.top;
        self.which = w;
        self.top = self.tops[w];
        self.left = 0;
        self.toc_top = 0;
        self.relayout(false);
    }

    /// Every example's tab, as example `n`'s was clicked: their bash
    /// versions or tessh's, with `n`'s tab bar kept where it was on screen.
    fn show_tab(&mut self, n: usize, bash: bool) {
        let w = self.which;
        let all: Vec<usize> = self.tab_lines.iter().map(|&(_, k)| k).collect();
        if all.iter().all(|k| bash == self.bash[w].contains(k)) {
            return;
        }
        let row = self.tab_lines.iter().find(|&&(_, k)| k == n).map(|&(l, _)| l as isize - self.top as isize);
        if bash {
            self.bash[w].extend(all);
        } else {
            self.bash[w].clear();
        }
        self.relayout(true);
        if let (Some(row), Some(&(l, _))) = (row, self.tab_lines.iter().find(|&&(_, k)| k == n)) {
            self.top = (l as isize - row).max(0) as usize;
        }
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
        self.tab_lines.clear();
        let md = match self.content().clone() {
            Content::Markdown(md) => md,
            Content::Lines(l) => {
                self.layout_lines(&l, width);
                return;
            }
        };
        // (Each example's two versions as tabs, the one picked shown.)
        let set = &self.bash[self.which];
        let md = if md.contains("```bash") { crate::help::tab_examples(&md, &|n| set.contains(&n)).0 } else { md };
        let text = crate::help::terminal_at(&md, true, width.clamp(20, 100));
        self.lines = text.lines().map(String::from).collect();
        self.plain = self.lines.iter().map(|l| strip(l)).collect();
        self.gutters = self.plain.iter().map(|l| if l.starts_with("  │ ") { 4 } else if l == "  │" { 3 } else { 0 }).collect();
        self.widest = 0;
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

    /// Lines laid out already: wrapped at `width` (a piece after the first
    /// with a blank gutter), or as they are.
    fn layout_lines(&mut self, l: &Lines, width: usize) {
        let keep = self.parts.iter().rev().find(|p| p.line <= self.top).map(|p| p.title.clone());
        let mut starts = Vec::with_capacity(l.lines.len());
        self.lines.clear();
        if l.wrap {
            let room = width.saturating_sub(l.gutter).max(10);
            for line in &l.lines {
                starts.push(self.lines.len());
                let head = slice(line, 0, l.gutter);
                let plain = strip(line);
                let body_w = plain.width().saturating_sub(l.gutter);
                let mut from = l.gutter;
                let mut first = true;
                loop {
                    let piece = slice(line, from, room);
                    self.lines.push(if first { format!("{head}{piece}") } else { format!("{}{piece}", " ".repeat(l.gutter)) });
                    first = false;
                    from += room;
                    if from >= l.gutter + body_w {
                        break;
                    }
                }
            }
        } else {
            starts = (0..l.lines.len()).collect();
            self.lines = l.lines.clone();
        }
        self.plain = self.lines.iter().map(|x| strip(x)).collect();
        self.widest = self.plain.iter().map(|x| x.width()).max().unwrap_or(0);
        self.gutters = vec![l.gutter; self.lines.len()];
        self.lower = self.plain.iter().map(|x| x.to_lowercase()).collect();
        self.parts = l.parts.iter().filter_map(|(level, title, at)| starts.get(*at).map(|&line| Part { level: *level, title: title.clone(), line })).collect();
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
        // (Else from the view: a match is shown two lines below the top, so
        // next goes from that line, not finding the one shown again.)
        let t = self.top;
        let shown = t + 2;
        let n = self.matches.len();
        let i = if let (Some(at), false) = (self.at, include_top) {
            // (From the match last shown, wherever the view could put it.)
            if back { (at + n - 1) % n } else { (at + 1) % n }
        } else if back {
            self.matches.iter().rposition(|&m| m < shown).unwrap_or(n - 1)
        } else if include_top {
            self.matches.iter().position(|&m| m >= t).unwrap_or(0)
        } else {
            self.matches.iter().position(|&m| m > shown).unwrap_or(0)
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
    /// The chars of line `i` in the pick: a code block's line without its
    /// "  │ " gutter, which is the guide's, not the code's.
    fn picked_span(&self, i: usize, ((l0, c0), (l1, c1)): ((usize, usize), (usize, usize))) -> (usize, usize) {
        let line = &self.plain[i];
        let len = line.chars().count();
        let gutter = self.gutters.get(i).copied().unwrap_or(0).min(len);
        let from = if i == l0 { c0.min(len) } else { 0 }.max(gutter);
        let to = if i == l1 { c1.min(len) } else { len };
        (from, to.max(from))
    }
    /// The word at (line, char): letters, digits and _ (or the run of
    /// other non-blank chars there), picked.
    fn pick_word(&mut self, (i, c): (usize, usize)) {
        let Some(line) = self.plain.get(i) else { return };
        let chars: Vec<char> = line.chars().collect();
        let (lo, _) = self.picked_span(i, ((i, 0), (i, chars.len())));
        let c = c.min(chars.len().saturating_sub(1)).max(lo);
        let Some(&at) = chars.get(c) else { return };
        let word = |x: char| x.is_alphanumeric() || x == '_';
        let same = |x: char| if word(at) { word(x) } else if at.is_whitespace() { x.is_whitespace() } else { !word(x) && !x.is_whitespace() };
        let mut a = c;
        while a > lo && same(chars[a - 1]) {
            a -= 1;
        }
        let mut b = c;
        while b < chars.len() && same(chars[b]) {
            b += 1;
        }
        self.sel = Some(((i, a), (i, b)));
        self.dragging = false;
    }
    /// Line i, picked (a code line without its gutter).
    fn pick_line(&mut self, i: usize) {
        let Some(line) = self.plain.get(i) else { return };
        let n = line.trim_end().chars().count();
        self.sel = Some(((i, 0), (i, n)));
        self.dragging = false;
    }
    /// The text picked, lines joined with newlines.
    fn sel_text(&self) -> String {
        let Some(r @ (_, (l1, _))) = self.sel_range() else { return String::new() };
        let mut out = vec![];
        for i in r.0 .0..=l1.min(self.plain.len().saturating_sub(1)) {
            let chars: Vec<char> = self.plain[i].chars().collect();
            let (from, to) = self.picked_span(i, r);
            out.push(chars[from..to].iter().collect::<String>().trim_end().to_string());
        }
        out.join("\n")
    }
    /// The (line, char) under a cell of the guide's text, given its left edge.
    fn pos_at(&self, col: usize, row: usize, x0: usize, rows: usize) -> (usize, usize) {
        let body = rows.saturating_sub(2);
        let r = row.clamp(1, body.max(1)) - 1;
        let i = (self.top + r).min(self.lines.len().saturating_sub(1));
        let g = self.gutters.get(i).copied().unwrap_or(0);
        let want = col.saturating_sub(x0);
        // (Scrolled sideways: the text after the gutter is further along.)
        let want = if self.unwrapped() && want >= g { want + self.left } else { want };
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

    /// The scrollbar's thumb: its first and past-last row in the body,
    /// when there's more than a screen to scroll.
    fn thumb(&self, rows: usize) -> Option<(usize, usize)> {
        let body = rows.saturating_sub(2);
        let total = self.lines.len();
        if body == 0 || total <= body {
            return None;
        }
        let size = (body * body / total).clamp(1, body);
        let at = self.top.min(total - body) * (body - size) / (total - body).max(1);
        Some((at, at + size))
    }
    /// The top line for the scrollbar's thumb starting at body row `r`.
    fn top_for_thumb(&self, r: usize, rows: usize) -> usize {
        let body = rows.saturating_sub(2);
        let total = self.lines.len();
        let size = (body * body / total.max(1)).clamp(1, body.max(1));
        let room = body.saturating_sub(size).max(1);
        r.min(room) * total.saturating_sub(body) / room
    }

    fn sidebar_w(&self, cols: usize) -> usize {
        if cols < 70 || !self.has_parts() {
            return 0;
        }
        // (Dragged at its edge: that width, kept while tessh runs.)
        match SIDE_W.load(std::sync::atomic::Ordering::Relaxed) {
            0 => (cols / 4).clamp(20, 34),
            w => w.clamp(12, cols.saturating_sub(30)),
        }
    }

    fn draw(&mut self, cols: usize, rows: usize) {
        let sw = self.sidebar_w(cols);
        let cw = cols.saturating_sub(sw + if sw > 0 { 2 } else { 0 });
        self.layout(cw.saturating_sub(1));
        let body = rows.saturating_sub(2);
        self.top = self.top.min(self.lines.len().saturating_sub(body));
        let unwrapped = self.unwrapped();
        if unwrapped {
            self.left = self.left.min(self.widest.saturating_sub(cw.saturating_sub(1)));
        } else {
            self.left = 0;
        }
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
        let field = if self.query.is_empty() && !self.searching { format!("\x1b[2mSearch {} (/)\x1b[22m", self.name) } else { self.query.clone() };
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
            let shown = if let Some(r @ (_, (l1, _))) = picked {
                // (A line in the pick: the picked part reversed; a code
                // block's gutter never is.)
                let chars: Vec<char> = self.plain[i].chars().collect();
                let (from, to) = self.picked_span(i, r);
                let (a, b, c): (String, String, String) =
                    (chars[..from].iter().collect(), chars[from..to].iter().collect(), chars[to..].iter().collect());
                let b = if b.is_empty() && i != l1 { " ".to_string() } else { b };
                format!("{a}\x1b[7m{b}\x1b[27m{c}")
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
                out
            } else {
                l.clone()
            };
            // (Never past the edge, where the terminal would wrap it; scrolled
            // sideways, the gutter stays and the rest moves.)
            let room = cw.saturating_sub(1).max(1);
            if unwrapped && self.left > 0 {
                let g = self.gutters.get(i).copied().unwrap_or(0).min(room);
                let _ = write!(o, "{}{}", slice(&shown, 0, g), slice(&shown, g + self.left, room - g));
            } else if self.widest > room || unwrapped {
                let _ = write!(o, "{}", slice(&shown, 0, room));
            } else {
                let _ = write!(o, "{shown}");
            }
        }
        // The scrollbar, down the right edge (more than a screen's worth):
        // its thumb is where you are and how much of it shows.
        if let Some((t0, t1)) = self.thumb(rows) {
            for r in 0..body {
                let on = (t0..t1).contains(&r);
                let _ = write!(o, "\x1b[{};{}H\x1b[0m{}", r + 2, cols, if on { "\x1b[1m┃\x1b[22m" } else { "\x1b[2m│\x1b[22m" });
            }
        }
        let total = self.lines.len().max(1);
        let pct = ((self.top + body).min(total) * 100) / total;
        let hint = if self.file.is_some() || !self.info.is_empty() {
            // (A file: its name and kind first, then the keys there are for it.)
            let views = if self.guides.len() > 1 { format!("   Tab {}", self.guides[(self.which + 1) % self.guides.len()].0) } else { String::new() };
            let side = match self.content() {
                Content::Lines(l) if l.wrap => "   w no wrap",
                Content::Lines(_) if self.widest > cw => "   ← → sideways   w wrap",
                Content::Lines(_) => "   w wrap",
                _ => "",
            };
            let parts = if self.parts.is_empty() { "" } else { "   [ ] parts" };
            let open = if self.file.is_some() { "   o open in its app" } else { "" };
            format!(" {}  {pct}%   / search   n N next{parts}{views}{side}   ↑↓ PgUp PgDn   Shift+↑↓ pick lines   y copy{open}   q back", self.info)
        } else {
            let more = if self.guides.len() > 1 { "   Tab other guide" } else { "" };
            let tabs = if self.tab_lines.is_empty() { "" } else { "   t bash/tessh" };
            format!(" {pct}%   / search   Enter n N next   [ ] parts{more}{tabs}   ↑↓ PgUp PgDn   y copies the part (or drag, right click)   q back")
        };
        let _ = write!(o, "\x1b[{};1H\x1b[0m\x1b[2K\x1b[2m{}\x1b[0m", rows, cut(&hint, cols));
        // The right-click menu, over the guide.
        if let Some((mx, my, at)) = self.menu {
            let w = 17;
            let (x, y) = (mx.min(cols.saturating_sub(w + 2)), my.min(rows.saturating_sub(MENU.len() + 2)));
            let _ = write!(o, "\x1b[{};{}H\x1b[0m╭{}╮", y + 1, x + 1, "─".repeat(w));
            for (k, item) in MENU.iter().enumerate() {
                let off = k == 0 && self.sel_range().is_none();
                let item = if k == 2 && self.file.is_some() { &"Open in its app" } else { item };
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

/// A right-click menu item: Copy, Select All, and Copy all (Open in its
/// app, for a file).
fn run_item(g: &mut Guide, i: usize) {
    match i {
        0 => g.copy(),
        1 => g.select_all(),
        _ => match g.file.clone() {
            Some(p) => crate::viewer::open_outside(&p),
            None => {
                g.select_all();
                g.copy();
            }
        },
    }
}

impl Guide {
    fn select_all(&mut self) {
        let last = self.plain.len().saturating_sub(1);
        self.sel = Some(((0, 0), (last, self.plain.get(last).map(|l| l.chars().count()).unwrap_or(0))));
    }
    /// Shift+↓ / Shift+↑: whole lines picked from the keyboard, from the
    /// top line shown (or the pick's end), the view following.
    fn pick_lines(&mut self, down: bool, body: usize) {
        let last = self.plain.len().saturating_sub(1);
        let len = |g: &Guide, i: usize| g.plain.get(i).map(|l| l.chars().count()).unwrap_or(0);
        let (a, end) = match self.sel {
            Some(((a, _), (b, _))) if self.sel_range().is_some() => (a, b),
            _ => {
                let a = self.top.min(last);
                self.sel = Some(((a, 0), (a, len(self, a))));
                return;
            }
        };
        let b = if down { (end + 1).min(last) } else { end.saturating_sub(1) };
        let (from, to) = if b >= a { ((a, 0), (b, len(self, b))) } else { ((a, len(self, a)), (b, 0)) };
        self.sel = Some((from, to));
        if b < self.top {
            self.top = b;
        } else if b >= self.top + body {
            self.top = b + 1 - body;
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

pub(crate) fn run(mut g: Guide, at: &str) {
    let mut o = std::io::stdout();
    let _ = crossterm::terminal::enable_raw_mode();
    let _ = write!(o, "\x1b[?1049h\x1b[?1002h\x1b[?1006h\x1b[H\x1b[2J");
    let _ = o.flush();
    let (c0, _) = crossterm::terminal::size().unwrap_or((80, 24));
    let sw = g.sidebar_w(c0 as usize);
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
    let mut resizing = false;
    // (The last left press, for double and triple clicks: when, where, and
    // how many in a row.)
    let mut last_click: Option<(std::time::Instant, (usize, usize), u8)> = None;
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
                    if g.sel_range().is_some() {
                        g.copy();
                    } else if g.parts.is_empty() {
                        // (No parts: y copies what's shown.)
                        let a = g.top.min(g.plain.len().saturating_sub(1));
                        let b = (g.top + rows.saturating_sub(2)).min(g.plain.len()).max(a + 1) - 1;
                        g.sel = Some(((a, 0), (b, g.plain.get(b).map(|l| l.chars().count()).unwrap_or(0))));
                        g.copy();
                    } else if let Some(i) = g.current_part() {
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
                    KeyCode::Down | KeyCode::Up if k.modifiers.contains(KeyModifiers::SHIFT) => g.pick_lines(k.code == KeyCode::Down, rows.saturating_sub(2)),
                    KeyCode::Left | KeyCode::Char('h') if g.unwrapped() => g.left = g.left.saturating_sub(8),
                    KeyCode::Right | KeyCode::Char('l') if g.unwrapped() => g.left += 8,
                    // (A file's lines: wrapped, or as they are with ← →.)
                    KeyCode::Char('w') if matches!(g.content(), Content::Lines(_)) => {
                        let w = g.which;
                        if let Content::Lines(l) = &mut g.guides[w].1 {
                            l.wrap = !l.wrap;
                        }
                        g.relayout(true);
                    }
                    KeyCode::Char('o') if g.file.is_some() => {
                        if let Some(p) = g.file.clone() {
                            crate::viewer::open_outside(&p);
                        }
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
                let sw = g.sidebar_w(cols);
                let over_toc = sw > 0 && (m.column as usize) < sw && m.row >= 1;
                let x0 = if sw > 0 { sw + 2 } else { 0 };
                let in_text = !over_toc && m.row >= 1 && (m.row as usize) <= rows.saturating_sub(2);
                if let Some((x, y, _)) = g.menu {
                    // (The menu open: a click on an item does it, anywhere else closes it.)
                    let (w, col, row) = (17usize, m.column as usize, m.row as usize);
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
                // (The scrollbar, at the right edge: its thumb dragged, or a
                // click above or below it, a page.)
                let on_bar = m.column as usize + 1 == cols && m.row >= 1 && (m.row as usize) <= rows.saturating_sub(2);
                match m.kind {
                    MouseEventKind::Down(MouseButton::Left) if on_bar && g.thumb(rows).is_some() => {
                        let r = m.row as usize - 1;
                        let (t0, t1) = g.thumb(rows).unwrap();
                        if r < t0 {
                            g.top = g.top.saturating_sub(page);
                        } else if r >= t1 {
                            g.top += page;
                        } else {
                            g.thumb_drag = Some(r - t0);
                        }
                        continue;
                    }
                    MouseEventKind::Drag(MouseButton::Left) if g.thumb_drag.is_some() => {
                        let held = g.thumb_drag.unwrap_or(0);
                        g.top = g.top_for_thumb((m.row as usize).saturating_sub(1).saturating_sub(held), rows);
                        continue;
                    }
                    MouseEventKind::Up(_) if g.thumb_drag.is_some() => {
                        g.thumb_drag = None;
                        continue;
                    }
                    _ => {}
                }
                // (The contents column's edge, the │: dragged, it resizes the column.)
                match m.kind {
                    MouseEventKind::Down(MouseButton::Left) if sw > 0 && m.row >= 1 && (m.column as usize) == sw => {
                        resizing = true;
                        continue;
                    }
                    MouseEventKind::Drag(MouseButton::Left) if resizing => {
                        SIDE_W.store((m.column as usize).clamp(12, cols.saturating_sub(30)), std::sync::atomic::Ordering::Relaxed);
                        continue;
                    }
                    MouseEventKind::Up(_) if resizing => {
                        resizing = false;
                        continue;
                    }
                    _ => {}
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
                        let n = match last_click {
                            Some((t, q, n)) if q.0 == p.0 && t.elapsed() < std::time::Duration::from_millis(450) => n % 3 + 1,
                            _ => 1,
                        };
                        last_click = Some((std::time::Instant::now(), p, n));
                        g.searching = false;
                        // (Twice: the word; three times: the line.)
                        match n {
                            2 => g.pick_word(p),
                            3 => g.pick_line(p.0),
                            _ => {
                                g.sel = Some((p, p));
                                g.dragging = true;
                            }
                        }
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
                    // (Shift+wheel, or a sideways wheel: ← →.)
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollUp if g.unwrapped() && m.modifiers.contains(KeyModifiers::SHIFT) => {
                        g.left = if m.kind == MouseEventKind::ScrollDown { g.left + 8 } else { g.left.saturating_sub(8) }
                    }
                    MouseEventKind::ScrollRight if g.unwrapped() => g.left += 8,
                    MouseEventKind::ScrollLeft if g.unwrapped() => g.left = g.left.saturating_sub(8),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_tab_switches_every_example_and_stays_put() {
        let mut g = Guide::new(include_str!("../../doc/advanced/02-sha-bang.md"));
        assert!(g.tab_lines.len() > 2);
        let (line, n) = g.tab_lines[2];
        g.top = line - 3;
        g.show_tab(n, true);
        let all: Vec<usize> = g.tab_lines.iter().map(|&(_, k)| k).collect();
        assert!(all.iter().all(|k| g.bash[0].contains(k)), "every example shows bash");
        let now = g.tab_lines.iter().find(|&&(_, k)| k == n).unwrap().0;
        assert_eq!(now - g.top, 3, "the clicked tab bar stays on its row");
        g.show_tab(all[0], false);
        assert!(g.bash[0].is_empty(), "and back to tessh, all of them");
    }

    #[test]
    fn double_click_picks_a_word_triple_a_line() {
        let mut g = Guide::new("Some words here.\n\n```tessh\nprint hello_world\n```\n");
        let a = g.plain.iter().position(|l| l.contains("Some words")).unwrap();
        let c = g.plain[a].find("words").unwrap() + 1;
        g.pick_word((a, c));
        assert_eq!(g.sel_text(), "words");
        let k = g.plain.iter().position(|l| l.contains("hello_world")).unwrap();
        let c = g.plain[k].chars().position(|x| x == 'w').unwrap();
        g.pick_word((k, c));
        assert_eq!(g.sel_text(), "hello_world");
        g.pick_line(k);
        assert_eq!(g.sel_text(), "print hello_world");
    }

    #[test]
    fn a_pick_in_code_leaves_out_the_gutter() {
        let mut g = Guide::new("Text.\n\n```tessh\nif True\n  print hi\n```\n");
        let a = g.plain.iter().position(|l| l.contains("if True")).unwrap();
        g.sel = Some(((a, 0), (a + 1, g.plain[a + 1].chars().count())));
        assert_eq!(g.sel_text(), "if True\n  print hi");
    }
}
