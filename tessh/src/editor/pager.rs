//! The guide, full screen, as Tessera's guide tab has it: a search box on
//! top (searching as you type, "3 of 12"), the contents down the left (each
//! part a click away, the one being read highlighted), the guide beside it,
//! laid out for the room it has. On the terminal's other screen: what was
//! there comes back after. Keys: / or a click on the box to search, Enter /
//! n / N for the next and previous match, [ ] for the parts, the arrows,
//! PgUp PgDn, g G, the wheel; q or Esc goes back.
//!
//! Drawn as MS-DOS Edit drew its windows: a gray menu bar (File, Edit,
//! Search, View, Help: F10 or Alt and a letter, or a click), a blue window
//! with a double frame and its title in the border, the contents in a
//! window of their own (a black window by default; Options picks the
//! colors, kept in ~/.config/tessh/editor.json), scrollbars with arrows (the text's, the contents',
//! and one in the bottom border for long lines), a cyan status bar, and
//! menus and the keys (F1) over it all with drop shadows. Text is picked by
//! dragging, or as a rectangle of columns with Alt (or Ctrl) held, R, or
//! Alt+Shift and the arrows.
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
    /// The pick is a rectangle: its corners are (line, column), and each
    /// line's columns between them are picked. `rect_mode`: drags and
    /// Shift+arrows pick rectangles (else Alt+drag and Alt+Shift+arrows do).
    rect: bool,
    rect_mode: bool,
    /// The right-click menu: where (column, row), and the item pointed at.
    menu: Option<(usize, usize, usize)>,
    /// The menu bar's menu open (which, the item lit), and where the open
    /// menu's and the right-click menu's boxes are (x, y, width, items).
    open: Option<(usize, usize)>,
    open_box: Option<(usize, usize, usize, usize)>,
    ctx_box: Option<(usize, usize, usize, usize)>,
    /// Where the menu titles, the search box and the status bar's keys are.
    menu_x: Vec<(usize, usize)>,
    search_x: (usize, usize),
    status_x: Vec<(usize, usize, Option<Act>)>,
    /// The keys dialog shown; the contents hidden (C); a word for the status
    /// bar until the next key ("Copied the path").
    help: bool,
    side_hidden: bool,
    note: String,
}

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
        let mut g = Guide { guides, which, name: name.to_string(), info: info.to_string(), file: None, left: 0, gutters: vec![], widest: 0, tops: vec![0; n], bash: vec![Default::default(); n], tab_lines: vec![], chips: vec![], width: 0, lines: vec![], plain: vec![], lower: vec![], parts: vec![], top: 0, toc_top: 0, query: String::new(), searching: false, matches: vec![], at: None, sel: None, dragging: false, rect: false, rect_mode: false, menu: None, open: None, open_box: None, ctx_box: None, menu_x: vec![], search_x: (0, 0), status_x: vec![], help: false, side_hidden: false, note: String::new() };
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
    /// A rectangle's: its top-left and bottom-right corners (line, column),
    /// the right one past its last column.
    fn sel_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let (a, b) = self.sel?;
        if self.rect {
            let (c0, c1) = (a.1.min(b.1), a.1.max(b.1));
            return (c0 != c1).then_some(((a.0.min(b.0), c0), (a.0.max(b.0), c1)));
        }
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
        if self.rect {
            // (Columns c0 to c1 of the line, as chars; never its gutter.)
            let (mut from, mut to, mut w) = (len, len, 0);
            for (k, ch) in line.chars().enumerate() {
                if w >= c0 && from == len {
                    from = k;
                }
                if w >= c1 {
                    to = k;
                    break;
                }
                w += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            }
            let from = from.max(gutter).min(len);
            let _ = l0;
            let _ = l1;
            return (from, to.max(from));
        }
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
        self.rect = false;
        self.sel = Some(((i, a), (i, b)));
        self.dragging = false;
    }
    /// Line i, picked (a code line without its gutter).
    fn pick_line(&mut self, i: usize) {
        let Some(line) = self.plain.get(i) else { return };
        let n = line.trim_end().chars().count();
        self.rect = false;
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
    /// The (line, column of the text) under a cell of the screen.
    fn col_at(&self, col: usize, row: usize, geo: Geom) -> (usize, usize) {
        let r = row.clamp(2, (geo.bh + 1).max(2)) - 2;
        let i = (self.top + r).min(self.lines.len().saturating_sub(1));
        let g = self.gutters.get(i).copied().unwrap_or(0);
        let want = col.saturating_sub(geo.x0 + 1);
        // (Scrolled sideways: the text after the gutter is further along.)
        (i, if self.unwrapped() && want >= g { want + self.left } else { want })
    }
    /// The (line, char) under a cell of the screen.
    fn pos_at(&self, col: usize, row: usize, geo: Geom) -> (usize, usize) {
        let (i, want) = self.col_at(col, row, geo);
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

    /// The contents window's width (0: none, when there are no parts, the
    /// screen is narrow, or C hid it).
    fn sidebar_w(&self, cols: usize) -> usize {
        if cols < 70 || !self.has_parts() || self.side_hidden {
            return 0;
        }
        // (Dragged at its edge: that width, kept while tessh runs.)
        match SIDE_W.load(std::sync::atomic::Ordering::Relaxed) {
            0 => (cols / 4).clamp(22, 36),
            w => w.clamp(14, cols.saturating_sub(30)),
        }
    }

    /// Where everything is for a screen of `cols`×`rows`.
    fn geom(&self, cols: usize, rows: usize) -> Geom {
        let sw = self.sidebar_w(cols);
        Geom { sw, x0: sw, cw: cols.saturating_sub(sw + 2).max(1), bh: rows.saturating_sub(4).max(1) }
    }

    /// The menus: their titles (the hot letter first) and items.
    fn menus(&self) -> Vec<(&'static str, Vec<Item>)> {
        let mut file = vec![];
        if self.file.is_some() {
            file.push(Item::new("Open in its app", "O", Act::OpenApp));
            file.push(Item::new("Copy path", "", Act::CopyPath));
            file.push(Item::sep());
        }
        file.push(Item::new("Exit", "Q", Act::Quit));
        let edit = vec![
            Item::new("Copy", "Ctrl+C", Act::Copy).on(self.sel_range().is_some()),
            Item::new("Select all", "Ctrl+A", Act::SelectAll),
            Item::new("Copy all", "", Act::CopyAll),
            Item::sep(),
            Item::new("Rectangle select", "R", Act::RectMode).check(self.rect_mode),
        ];
        let search = vec![
            Item::new("Find…", "/", Act::Find),
            Item::new("Next match", "N", Act::Next).on(!self.matches.is_empty()),
            Item::new("Previous match", "Shift+N", Act::Prev).on(!self.matches.is_empty()),
        ];
        let mut view: Vec<Item> = self.guides.iter().enumerate().filter(|_| self.guides.len() > 1).map(|(k, (t, _))| Item::new_owned(t.clone(), if k == (self.which + 1) % self.guides.len() { "Tab" } else { "" }, Act::View(k)).radio(k == self.which)).collect();
        if !view.is_empty() {
            view.push(Item::sep());
        }
        if matches!(self.content(), Content::Lines(_)) {
            view.push(Item::new("Wrap long lines", "W", Act::Wrap).check(!self.unwrapped()));
        }
        if self.has_parts() {
            view.push(Item::new("Contents", "C", Act::Sidebar).check(!self.side_hidden));
        }
        if !self.tab_lines.is_empty() {
            view.push(Item::new("bash examples", "T", Act::Bash).check(!self.bash[self.which].is_empty()));
        }
        let cur = scheme_index();
        let options = SCHEMES.iter().enumerate().map(|(k, sc)| Item::new(sc.name, "", Act::Scheme(k)).radio(k == cur)).collect();
        let help = vec![Item::new("Keys and mouse…", "F1", Act::Help)];
        vec![("File", file), ("Edit", edit), ("Search", search), ("View", view), ("Options", options), ("Help", help)]
    }

    /// Do a menu's (or a key's) action; true when it's time to go back.
    fn act(&mut self, a: Act) -> bool {
        match a {
            Act::Quit => return true,
            Act::OpenApp => {
                if let Some(p) = self.file.clone() {
                    crate::viewer::open_outside(&p);
                    self.note = "Opened in its app".into();
                }
            }
            Act::CopyPath => {
                if let Some(p) = &self.file {
                    super::term::Term::copy(&p.to_string_lossy());
                    self.note = "Copied the path".into();
                }
            }
            Act::Copy => self.copy(),
            Act::SelectAll => self.select_all(),
            Act::CopyAll => {
                self.select_all();
                self.copy();
            }
            Act::RectMode => {
                self.rect_mode = !self.rect_mode;
                self.note = if self.rect_mode { "Rectangle select: drag or Shift+arrows pick columns" } else { "Rectangle select off (Alt+drag still picks columns)" }.into();
            }
            Act::Find => self.searching = true,
            Act::Next => self.step(false, false),
            Act::Prev => self.step(true, false),
            Act::View(k) => self.switch(k),
            Act::Wrap => {
                let w = self.which;
                if let Content::Lines(l) = &mut self.guides[w].1 {
                    l.wrap = !l.wrap;
                }
                self.relayout(true);
            }
            Act::Sidebar => self.side_hidden = !self.side_hidden,
            Act::Bash => {
                let w = self.which;
                if self.bash[w].is_empty() {
                    let all: Vec<usize> = self.tab_lines.iter().map(|&(_, n)| n).collect();
                    self.bash[w].extend(all);
                } else {
                    self.bash[w].clear();
                }
                self.relayout(true);
            }
            Act::Help => self.help = true,
            Act::Scheme(k) => {
                set_scheme(k);
                self.note = format!("Colors: {} (kept for next time)", SCHEMES[k].name);
            }
        }
        false
    }

    fn draw(&mut self, cols: usize, rows: usize) {
        let g = self.geom(cols, rows);
        self.layout(g.cw);
        let t = theme();
        self.top = self.top.min(self.lines.len().saturating_sub(g.bh));
        let unwrapped = self.unwrapped();
        if unwrapped {
            self.left = self.left.min(self.widest.saturating_sub(g.cw));
        } else {
            self.left = 0;
        }
        let mut o = String::with_capacity(cols * rows * 4);
        // (One frame, synchronized output: shown whole, no flicker.)
        o.push_str("\x1b[?2026h\x1b[?25l");
        // ---- the menu bar, the search box at its right ----
        o.push_str(&format!("\x1b[1;1H{}{}", t.bar, " ".repeat(cols)));
        self.menu_x.clear();
        let mut x = 1;
        for (k, (title, _)) in self.menus().iter().enumerate() {
            let open = self.open.is_some_and(|(m, _)| m == k);
            let (sty, hot) = if open { (t.bar_sel, t.bar_sel_hot) } else { (t.bar, t.bar_hot) };
            let (h, rest) = title.split_at(1);
            o.push_str(&format!("\x1b[1;{}H{sty} {hot}{h}{sty}{rest} ", x + 1));
            self.menu_x.push((x, x + title.len() + 2));
            x += title.len() + 2;
        }
        let count = if self.query.is_empty() {
            String::new()
        } else if self.matches.is_empty() {
            "no matches".into()
        } else {
            match self.at {
                Some(i) => format!("{} of {}", i + 1, self.matches.len()),
                None => format!("{} matches", self.matches.len()),
            }
        };
        let fw = (cols / 3).clamp(16, 40);
        let sx = cols.saturating_sub(fw + count.width() + 4).max(x + 1);
        let (fsty, field) = if self.query.is_empty() && !self.searching { (t.field_dim, format!("Find in {} (/)", self.name)) } else { (t.field, self.query.clone()) };
        let field = cut(&field, fw.saturating_sub(2));
        o.push_str(&format!("\x1b[1;{}H{} ⌕ {fsty} {field}{}{} {count}", sx + 1, t.bar, " ".repeat(fw.saturating_sub(1 + field.width())), t.bar));
        self.search_x = (sx, sx + fw + 3);
        // ---- the windows' frames ----
        let title = if self.name == "the guide" { if self.guides.len() > 1 { "tessh guides".to_string() } else { "Guide".to_string() } } else { self.name.clone() };
        self.frame(&mut o, g.x0, 1, cols - g.x0, rows - 2, true, &title, self.guides.len() > 1);
        // (The views on the top border, the one shown lit.)
        self.chips.clear();
        if self.guides.len() > 1 {
            let labels: Vec<String> = self.guides.iter().map(|(l, _)| format!(" {l} ")).collect();
            let total: usize = labels.iter().map(|l| l.width() + 1).sum();
            let mut cx = cols.saturating_sub(total + 3);
            for (k, l) in labels.iter().enumerate() {
                let sty = if k == self.which { t.title } else { t.frame };
                o.push_str(&format!("\x1b[2;{}H{sty}{l}", cx + 1));
                self.chips.push((cx, cx + l.width(), k));
                cx += l.width() + 1;
            }
        }
        if g.sw > 0 {
            self.frame(&mut o, 0, 1, g.sw, rows - 2, false, "Contents", false);
        }
        // ---- the contents ----
        let cur = self.current_part();
        if let Some(c) = cur {
            if c < self.toc_top {
                self.toc_top = c;
            } else if c >= self.toc_top + g.bh {
                self.toc_top = c + 1 - g.bh;
            }
        }
        self.toc_top = self.toc_top.min(self.parts.len().saturating_sub(g.bh));
        if g.sw > 0 {
            let w = g.sw - 2;
            for r in 0..g.bh {
                o.push_str(&format!("\x1b[{};2H{}", r + 3, t.base));
                match self.parts.get(self.toc_top + r) {
                    Some(p) => {
                        let ind = "  ".repeat(p.level.saturating_sub(1));
                        let s = cut(&format!(" {ind}{}", p.title), w);
                        let pad = " ".repeat(w.saturating_sub(s.width()));
                        let sty = if Some(self.toc_top + r) == cur { t.sel } else if p.level == 1 { t.bright } else { t.base };
                        o.push_str(&format!("{sty}{s}{pad}"));
                    }
                    None => o.push_str(&" ".repeat(w)),
                }
            }
            self.vbar(&mut o, g.sw - 1, self.parts.len(), g.bh, self.toc_top);
        }
        // ---- the text ----
        let q = self.query.to_lowercase();
        let sel = self.sel_range();
        for r in 0..g.bh {
            o.push_str(&format!("\x1b[{};{}H{}", r + 3, g.x0 + 2, t.base));
            let i = self.top + r;
            let Some(l) = self.lines.get(i) else {
                o.push_str(&" ".repeat(g.cw));
                continue;
            };
            let picked = sel.filter(|((l0, _), (l1, _))| (*l0..=*l1).contains(&i));
            let shown = if let Some(r @ (_, (l1, _))) = picked {
                // (A line in the pick: the picked part lit; a gutter never is.)
                let chars: Vec<char> = self.plain[i].chars().collect();
                let (from, to) = self.picked_span(i, r);
                let (a, b, c): (String, String, String) = (chars[..from].iter().collect(), chars[from..to].iter().collect(), chars[to..].iter().collect());
                let b = if b.is_empty() && i != l1 && !self.rect { " ".to_string() } else { b };
                format!("{a}{}{b}{}{c}", t.sel, t.base)
            } else if !q.is_empty() && self.lower[i].contains(&q) {
                let (p, lp) = (&self.plain[i], &self.lower[i]);
                let mut at = 0;
                let mut out = String::new();
                while let Some(k) = lp[at..].find(&q) {
                    let s = at + k;
                    if !p.is_char_boundary(s) || !p.is_char_boundary(s + q.len()) {
                        break;
                    }
                    out.push_str(&p[at..s]);
                    out.push_str(t.found);
                    out.push_str(&p[s..s + q.len()]);
                    out.push_str(t.base);
                    at = s + q.len();
                }
                out.push_str(&p[at..]);
                out
            } else {
                l.clone()
            };
            let gw = self.gutters.get(i).copied().unwrap_or(0).min(g.cw);
            let vis = if unwrapped && self.left > 0 { format!("{}{}", slice(&shown, 0, gw), slice(&shown, gw + self.left, g.cw - gw)) } else { slice(&shown, 0, g.cw) };
            let w = strip(&vis).width();
            o.push_str(&recolor(&vis, t));
            o.push_str(t.base);
            o.push_str(&" ".repeat(g.cw.saturating_sub(w)));
        }
        self.vbar(&mut o, cols - 1, self.lines.len(), g.bh, self.top);
        // (The bottom border: a sideways scrollbar when the lines are wider.)
        if unwrapped && self.widest > g.cw && g.cw > 6 {
            self.hbar(&mut o, g.x0 + 1, rows - 2, g.cw, self.widest, self.left);
        }
        // ---- the status bar ----
        let total = self.lines.len().max(1);
        let pct = ((self.top + g.bh).min(total) * 100) / total;
        let right = format!(" {:>5}/{:<5} {pct:>3}% ", (self.top + 1).min(total), total);
        let mut left = if !self.note.is_empty() { format!(" {}", self.note) } else if !self.info.is_empty() { format!(" {}", strip(&self.info)) } else { String::new() };
        self.status_x.clear();
        let mut keys = String::new();
        let mut kx = 0usize;
        for (label, act) in [("F1 Help", Act::Help), ("F10 Menu", Act::Find), ("/ Find", Act::Find), ("Q Exit", Act::Quit)] {
            let s = format!(" {label} ");
            if label == "F10 Menu" {
                self.status_x.push((kx, kx + s.width(), None));
            } else {
                self.status_x.push((kx, kx + s.width(), Some(act)));
            }
            kx += s.width() + 1;
            keys.push_str(&s);
            keys.push('│');
        }
        if left.is_empty() || cols > 70 {
            left = format!("{keys}{left}");
        } else {
            self.status_x.clear();
        }
        let room = cols.saturating_sub(right.width());
        let lcut = cut(&left, room);
        o.push_str(&format!("\x1b[{rows};1H{}{lcut}{}{right}", t.status, " ".repeat(room.saturating_sub(lcut.width()))));
        // ---- a menu, the right-click menu, the keys: over it all, shadowed ----
        if let Some((m, sel_i)) = self.open {
            let menus = self.menus();
            if let (Some((_, items)), Some(&(mx, _))) = (menus.get(m), self.menu_x.get(m)) {
                let (bx, w) = box_menu(&mut o, items, mx, 1, sel_i, cols, rows, t);
                self.open_box = Some((bx, 1, w, items.len()));
            }
        }
        if let Some((mx, my, at)) = self.menu {
            let items = self.context_items();
            let x = mx.min(cols.saturating_sub(24));
            let y = my.min(rows.saturating_sub(items.len() + 3));
            let (bx, w) = box_menu(&mut o, &items, x, y, at, cols, rows, t);
            self.ctx_box = Some((bx, y, w, items.len()));
        }
        if self.help {
            help_box(&mut o, cols, rows, t, self.file.is_some(), self.guides.len() > 1);
        }
        if self.searching {
            o.push_str(&format!("\x1b[1;{}H\x1b[?25h", self.search_x.0 + 5 + self.query.width().min(fw.saturating_sub(3))));
        }
        o.push_str("\x1b[?2026l");
        let mut out = std::io::stdout().lock();
        let _ = out.write_all(o.as_bytes());
        let _ = out.flush();
    }

    /// A window's frame: double lines for the main one, single for the
    /// contents; its title in the top border, reversed.
    /// `left`: the title at the left (the views' chips take the right).
    #[allow(clippy::too_many_arguments)]
    fn frame(&self, o: &mut String, x: usize, y: usize, w: usize, h: usize, main: bool, title: &str, left: bool) {
        let t = theme();
        let (tl, tr, bl, br, hz, vt) = if main { ('╔', '╗', '╚', '╝', '═', '║') } else { ('┌', '┐', '└', '┘', '─', '│') };
        let inner = w.saturating_sub(2);
        o.push_str(&format!("\x1b[{};{}H{}{tl}{}{tr}", y + 1, x + 1, t.frame, hz.to_string().repeat(inner)));
        for r in 1..h.saturating_sub(1) {
            o.push_str(&format!("\x1b[{};{}H{}{vt}", y + 1 + r, x + 1, t.frame));
        }
        o.push_str(&format!("\x1b[{};{}H{}{bl}{}{br}", y + h, x + 1, t.frame, hz.to_string().repeat(inner)));
        let title = cut(&format!(" {title} "), inner.saturating_sub(4));
        let tx = if left { x + 3 } else { x + 1 + (inner.saturating_sub(title.width())) / 2 };
        o.push_str(&format!("\x1b[{};{}H{}{title}", y + 1, tx + 1, if main { t.title } else { t.frame }));
    }

    /// A vertical scrollbar in column `x`, down the body: ↑, the track
    /// (░) with its thumb (█), ↓.
    fn vbar(&self, o: &mut String, x: usize, total: usize, h: usize, pos: usize) {
        let t = theme();
        o.push_str(&format!("\x1b[3;{}H{}↑", x + 1, t.button));
        let track = h.saturating_sub(2);
        let th = thumb(total, h, pos, track);
        for r in 0..track {
            let on = th.is_some_and(|(a, b)| (a..b).contains(&r));
            o.push_str(&format!("\x1b[{};{}H{}", r + 4, x + 1, if on { format!("{}█", t.track) } else { format!("{}░", t.track) }));
        }
        o.push_str(&format!("\x1b[{};{}H{}↓", h + 2, x + 1, t.button));
    }

    /// A sideways scrollbar along row `y` from column `x`, `w` wide.
    fn hbar(&self, o: &mut String, x: usize, y: usize, w: usize, total: usize, pos: usize) {
        let t = theme();
        let track = w.saturating_sub(2);
        let th = thumb(total, w, pos, track);
        o.push_str(&format!("\x1b[{};{}H{}←{}", y + 1, x + 1, t.button, t.track));
        for c in 0..track {
            o.push(if th.is_some_and(|(a, b)| (a..b).contains(&c)) { '█' } else { '░' });
        }
        o.push_str(&format!("{}→", t.button));
    }

    /// The right-click menu's items.
    fn context_items(&self) -> Vec<Item> {
        let mut v = vec![
            Item::new("Copy", "Ctrl+C", Act::Copy).on(self.sel_range().is_some()),
            Item::new("Select all", "Ctrl+A", Act::SelectAll),
            Item::new("Copy all", "", Act::CopyAll),
            Item::new("Rectangle select", "R", Act::RectMode).check(self.rect_mode),
            Item::sep(),
            Item::new("Find…", "/", Act::Find),
        ];
        if self.file.is_some() {
            v.push(Item::new("Open in its app", "O", Act::OpenApp));
        }
        v
    }
}

/// Where the parts of the screen are: the contents window's width (0: none)
/// and the main window's left edge, its text's width, the body's height.
#[derive(Clone, Copy)]
struct Geom {
    sw: usize,
    x0: usize,
    cw: usize,
    bh: usize,
}

/// What a menu item or a key does.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Act {
    Quit,
    OpenApp,
    CopyPath,
    Copy,
    SelectAll,
    CopyAll,
    RectMode,
    Find,
    Next,
    Prev,
    View(usize),
    Wrap,
    Sidebar,
    Bash,
    Help,
    Scheme(usize),
}

/// A menu item: its label, its key, what it does, whether it can now, and
/// its mark (✓ or •). A separator has no action.
#[derive(Clone)]
struct Item {
    label: String,
    key: &'static str,
    act: Option<Act>,
    on: bool,
    mark: Option<char>,
}

impl Item {
    fn new(label: &str, key: &'static str, act: Act) -> Item {
        Item { label: label.to_string(), key, act: Some(act), on: true, mark: None }
    }
    fn new_owned(label: String, key: &'static str, act: Act) -> Item {
        Item { label, key, act: Some(act), on: true, mark: None }
    }
    fn sep() -> Item {
        Item { label: String::new(), key: "", act: None, on: false, mark: None }
    }
    fn on(mut self, on: bool) -> Item {
        self.on = on;
        self
    }
    fn check(mut self, c: bool) -> Item {
        self.mark = c.then_some('✓');
        self
    }
    fn radio(mut self, c: bool) -> Item {
        self.mark = c.then_some('•');
        self
    }
}

/// The colors the viewer is drawn in: its chrome as MS-DOS Edit drew it (a
/// gray menu bar, white frames, a cyan status bar, drop shadows) around a
/// window whose colors the scheme picks (Options menu; kept in
/// ~/.config/tessh/editor.json as viewer_colors). Built from each scheme's
/// RGB colors: 24-bit where the terminal has it, else the nearest of its 16.
struct Theme {
    base: &'static str,
    bright: &'static str,
    frame: &'static str,
    title: &'static str,
    sel: &'static str,
    found: &'static str,
    bar: &'static str,
    bar_hot: &'static str,
    bar_sel: &'static str,
    bar_sel_hot: &'static str,
    field: &'static str,
    field_dim: &'static str,
    status: &'static str,
    button: &'static str,
    track: &'static str,
    menu: &'static str,
    menu_hot: &'static str,
    menu_sel: &'static str,
    menu_off: &'static str,
    shadow: &'static str,
    /// The window's text and background as SGR parameters (for recolor),
    /// and the same swapped (a reversed run).
    fg: &'static str,
    bg: &'static str,
    rev: &'static str,
    /// Text colors that vanish on this window, and their stand-ins; or one
    /// color for all text (the monochrome schemes).
    remap: &'static [(&'static str, &'static str)],
    mono: Option<&'static str>,
}

/// A color: an RGB, or None for the terminal's own (foreground or background).
type C = Option<(u8, u8, u8)>;

/// A color scheme: its name and colors (a color pair is fg, bg).
struct Scheme {
    name: &'static str,
    win: (C, C),
    bright: C,
    frame: C,
    sel: (C, C),
    found: (C, C),
    chrome: (C, C),
    hot: C,
    status: (C, C),
    track: C,
    shadow: (C, C),
    remap: &'static [(&'static str, &'static str)],
    mono: C,
}

const BLACK: C = Some((0, 0, 0));
const GRAY: C = Some((170, 170, 170));
const WHITE: C = Some((255, 255, 255));
const CYAN: C = Some((0, 170, 170));
const BLUE: C = Some((0, 0, 170));
const AMBER: C = Some((255, 176, 0));
const PHOSPHOR: C = Some((51, 255, 51));

const SCHEMES: &[Scheme] = &[
    Scheme { name: "Classic", win: (GRAY, BLACK), bright: WHITE, frame: GRAY, sel: (BLACK, GRAY), found: (BLACK, CYAN), chrome: (BLACK, GRAY), hot: WHITE, status: (BLACK, CYAN), track: Some((85, 85, 85)), shadow: (Some((70, 70, 70)), Some((28, 28, 28))), remap: &[], mono: None },
    Scheme { name: "Edit (blue)", win: (GRAY, BLUE), bright: WHITE, frame: WHITE, sel: (BLUE, GRAY), found: (BLACK, CYAN), chrome: (BLACK, GRAY), hot: WHITE, status: (BLACK, CYAN), track: GRAY, shadow: (Some((85, 85, 85)), BLACK), remap: &[("34", "96"), ("90", "36")], mono: None },
    Scheme { name: "Terminal", win: (None, None), bright: None, frame: None, sel: (BLACK, GRAY), found: (BLACK, CYAN), chrome: (BLACK, GRAY), hot: WHITE, status: (BLACK, CYAN), track: None, shadow: (Some((70, 70, 70)), Some((40, 40, 40))), remap: &[], mono: None },
    Scheme { name: "Amber", win: (AMBER, BLACK), bright: Some((255, 214, 110)), frame: AMBER, sel: (BLACK, AMBER), found: (BLACK, Some((255, 214, 110))), chrome: (BLACK, AMBER), hot: Some((255, 250, 220)), status: (BLACK, Some((200, 136, 0))), track: Some((128, 88, 0)), shadow: (Some((90, 60, 0)), Some((30, 20, 0))), remap: &[], mono: AMBER },
    Scheme { name: "Green", win: (PHOSPHOR, BLACK), bright: Some((180, 255, 180)), frame: PHOSPHOR, sel: (BLACK, PHOSPHOR), found: (BLACK, Some((180, 255, 180))), chrome: (BLACK, PHOSPHOR), hot: Some((230, 255, 230)), status: (BLACK, Some((30, 190, 30))), track: Some((20, 120, 20)), shadow: (Some((10, 80, 10)), Some((0, 30, 0))), remap: &[], mono: PHOSPHOR },
    Scheme {
        name: "Light",
        win: (Some((30, 30, 30)), Some((240, 240, 236))),
        bright: BLACK,
        frame: Some((80, 80, 80)),
        sel: (WHITE, Some((0, 90, 190))),
        found: (BLACK, Some((255, 220, 90))),
        chrome: (BLACK, Some((200, 200, 200))),
        hot: Some((150, 0, 0)),
        status: (BLACK, Some((150, 200, 210))),
        track: Some((150, 150, 150)),
        shadow: (Some((120, 120, 120)), Some((110, 110, 110))),
        remap: &[("33", "38;2;150;100;0"), ("93", "38;2;150;100;0"), ("36", "38;2;0;115;135"), ("96", "38;2;0;115;135"), ("32", "38;2;0;125;0"), ("92", "38;2;0;125;0"), ("37", "38;2;70;70;70"), ("97", "38;2;0;0;0")],
        mono: None,
    },
];

/// Whether the terminal takes 24-bit color.
fn truecolor() -> bool {
    let ct = std::env::var("COLORTERM").unwrap_or_default();
    ct.contains("truecolor") || ct.contains("24bit") || std::env::var("TESSH_TRUECOLOR").is_ok_and(|v| v == "1")
}

/// A color as SGR parameters, foreground or background: 24-bit, or the
/// nearest of the 16 (VGA's, as Edit had them).
fn param(c: C, fg: bool) -> String {
    let Some((r, g, b)) = c else { return if fg { "39".into() } else { "49".into() } };
    if truecolor() {
        return format!("{};2;{r};{g};{b}", if fg { 38 } else { 48 });
    }
    const VGA: [(i32, i32, i32); 16] = [(0, 0, 0), (170, 0, 0), (0, 170, 0), (170, 85, 0), (0, 0, 170), (170, 0, 170), (0, 170, 170), (170, 170, 170), (85, 85, 85), (255, 85, 85), (85, 255, 85), (255, 255, 85), (85, 85, 255), (255, 85, 255), (85, 255, 255), (255, 255, 255)];
    let (r, g, b) = (r as i32, g as i32, b as i32);
    let i = (0..16).min_by_key(|&i| { let (x, y, z) = VGA[i]; (x - r).pow(2) + (y - g).pow(2) + (z - b).pow(2) }).unwrap_or(0);
    let n = if i < 8 { i + 30 } else { i + 82 };
    (if fg { n } else { n + 10 }).to_string()
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn pair((f, b): (C, C)) -> &'static str {
    leak(format!("\x1b[0;{};{}m", param(f, true), param(b, false)))
}

fn build(s: &Scheme) -> Theme {
    let (cf, cb) = s.chrome;
    let field = (WHITE, BLACK);
    Theme {
        base: pair(s.win),
        bright: leak(format!("\x1b[0;1;{};{}m", param(s.bright, true), param(s.win.1, false))),
        frame: pair((s.frame, s.win.1)),
        title: pair(s.chrome),
        sel: pair(s.sel),
        found: pair(s.found),
        bar: pair(s.chrome),
        bar_hot: pair((s.hot, cb)),
        bar_sel: pair((cb, cf)),
        bar_sel_hot: pair((s.hot, cf)),
        field: pair(field),
        field_dim: pair((Some((120, 120, 120)), BLACK)),
        status: pair(s.status),
        button: pair(s.chrome),
        track: pair((s.track, s.win.1)),
        menu: pair(s.chrome),
        menu_hot: pair((s.hot, cb)),
        menu_sel: pair((cb, cf)),
        menu_off: pair((Some((100, 100, 100)), cb)),
        shadow: pair(s.shadow),
        fg: leak(param(s.win.0, true)),
        bg: leak(param(s.win.1, false)),
        // (Reversed: the window's colors swapped, as Edit lights a pick; with
        // the terminal's own colors, the terminal's reverse.)
        rev: if s.win.0.is_none() { "7" } else { leak(format!("{};{}", param(s.win.1.or(BLACK), true), param(s.win.0, false))) },
        remap: s.remap,
        mono: s.mono.map(|c| leak(param(Some(c), true))),
    }
}

/// The scheme in use (its index in SCHEMES).
static SCHEME: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(usize::MAX);

/// tessh's own settings file (the editor's keys and indentation are kept
/// there too).
fn settings_file() -> std::path::PathBuf {
    let config = std::env::var_os("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config"));
    config.join("tessh/editor.json")
}

fn scheme_index() -> usize {
    use std::sync::atomic::Ordering;
    let i = SCHEME.load(Ordering::Relaxed);
    if i != usize::MAX {
        return i;
    }
    // ($TESSH_VIEWER_COLORS, else the one picked last, else Classic.)
    let want = std::env::var("TESSH_VIEWER_COLORS").ok().or_else(|| {
        let t = std::fs::read_to_string(settings_file()).ok()?;
        serde_json::from_str::<serde_json::Value>(&t).ok()?.get("viewer_colors")?.as_str().map(String::from)
    });
    let i = want.and_then(|w| SCHEMES.iter().position(|s| s.name.eq_ignore_ascii_case(&w) || s.name.split(' ').next().is_some_and(|n| n.eq_ignore_ascii_case(&w)))).unwrap_or(0);
    SCHEME.store(i, Ordering::Relaxed);
    i
}

/// Use scheme `i` from now on, and remember it.
fn set_scheme(i: usize) {
    SCHEME.store(i.min(SCHEMES.len() - 1), std::sync::atomic::Ordering::Relaxed);
    let f = settings_file();
    let mut j: serde_json::Value = std::fs::read_to_string(&f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(serde_json::json!({}));
    if !j.is_object() {
        j = serde_json::json!({});
    }
    j["viewer_colors"] = serde_json::Value::String(SCHEMES[i.min(SCHEMES.len() - 1)].name.to_string());
    if let Some(d) = f.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&f, serde_json::to_string_pretty(&j).unwrap_or_default());
}

fn theme() -> &'static Theme {
    static T: std::sync::OnceLock<Vec<Theme>> = std::sync::OnceLock::new();
    &T.get_or_init(|| SCHEMES.iter().map(build).collect())[scheme_index()]
}

/// A laid-out line's styles redone for the window: a reset goes back to the
/// window's colors (not the terminal's), a reverse swaps them, colors that
/// vanish on it are swapped for ones that don't, and a monochrome scheme
/// has every text color its one.
fn recolor(s: &str, t: &Theme) -> String {
    let mut out = String::with_capacity(s.len() + 32);
    let mut rest = s;
    while let Some(i) = rest.find("\x1b[") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 2..];
        let Some(end) = after.find(|c: char| !(c.is_ascii_digit() || c == ';' || c == ':')) else {
            out.push_str(&rest[i..]);
            return out;
        };
        if !after[end..].starts_with('m') {
            out.push_str(&rest[i..i + 2 + end + 1.min(after.len() - end)]);
            rest = &after[(end + 1).min(after.len())..];
            continue;
        }
        let params: Vec<&str> = after[..end].split(';').collect();
        let mut mapped: Vec<String> = vec![];
        let mut k = 0;
        while k < params.len() {
            let p = params[k];
            // (24-bit and 256 colors pass through whole, or become the one color.)
            let span = match (p, params.get(k + 1)) {
                ("38" | "48", Some(&"2")) => 5,
                ("38" | "48", Some(&"5")) => 3,
                _ => 0,
            };
            if span > 0 {
                match t.mono {
                    Some(m) if p == "38" => mapped.push(m.to_string()),
                    _ => mapped.extend(params[k..(k + span).min(params.len())].iter().map(|x| x.to_string())),
                }
                k += span;
                continue;
            }
            let is_fg = matches!(p.parse::<u32>(), Ok(30..=37 | 90..=97));
            match p {
                "" | "0" => mapped.push(format!("0;{};{}", t.fg, t.bg)),
                "39" => mapped.push(t.fg.to_string()),
                "49" => mapped.push(t.bg.to_string()),
                "7" => mapped.push(t.rev.to_string()),
                "27" => mapped.push(format!("{};{}", t.fg, t.bg)),
                _ if is_fg && t.mono.is_some() => mapped.push(t.mono.unwrap_or_default().to_string()),
                x => mapped.push(t.remap.iter().find(|(a, _)| *a == x).map(|(_, b)| b.to_string()).unwrap_or_else(|| x.to_string())),
            }
            k += 1;
        }
        out.push_str("\x1b[");
        out.push_str(&mapped.join(";"));
        out.push('m');
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// A scrollbar's thumb on a track `track` long: its first and past-last
/// cell, when there's more than `seen` of `total` to scroll.
fn thumb(total: usize, seen: usize, pos: usize, track: usize) -> Option<(usize, usize)> {
    if track == 0 || total <= seen {
        return None;
    }
    let size = (track * seen / total).clamp(1, track);
    let at = pos.min(total - seen) * (track - size) / (total - seen).max(1);
    Some((at, at + size))
}

/// The position for the thumb starting at track cell `cell`.
fn pos_for_thumb(total: usize, seen: usize, cell: usize, track: usize) -> usize {
    let size = (track * seen / total.max(1)).clamp(1, track.max(1));
    let room = track.saturating_sub(size).max(1);
    cell.min(room) * total.saturating_sub(seen) / room
}

/// A drop-down menu at (x, y) (its title's cell; the box below it), item
/// `at` lit, a shadow two columns right and one row down. Its left edge and
/// width.
fn box_menu(o: &mut String, items: &[Item], x: usize, y: usize, at: usize, cols: usize, rows: usize, t: &Theme) -> (usize, usize) {
    let lw = items.iter().map(|i| i.label.width()).max().unwrap_or(0);
    let kw = items.iter().map(|i| i.key.width()).max().unwrap_or(0);
    let w = lw + kw + if kw > 0 { 6 } else { 4 };
    let x = x.min(cols.saturating_sub(w + 2));
    let h = items.len() + 2;
    let y = y.min(rows.saturating_sub(h + 1));
    o.push_str(&format!("\x1b[{};{}H{}┌{}┐", y + 2, x + 1, t.menu, "─".repeat(w - 2)));
    for (k, it) in items.iter().enumerate() {
        let row = y + 3 + k;
        if it.act.is_none() {
            o.push_str(&format!("\x1b[{row};{}H{}├{}┤", x + 1, t.menu, "─".repeat(w - 2)));
        } else {
            let sty = if !it.on { t.menu_off } else if k == at { t.menu_sel } else { t.menu };
            let mark = it.mark.map(|c| c.to_string()).unwrap_or_else(|| " ".into());
            let pad = " ".repeat(lw - it.label.width());
            let key = if kw > 0 { format!("  {:>kw$}", it.key) } else { String::new() };
            // (The first letter lit, as Edit's hot letters.)
            let (h1, rest) = it.label.split_at(it.label.chars().next().map(|c| c.len_utf8()).unwrap_or(0));
            let hot = if it.on && k != at { t.menu_hot } else { sty };
            o.push_str(&format!("\x1b[{row};{}H{}│{sty}{mark}{hot}{h1}{sty}{rest}{pad}{key} {}│", x + 1, t.menu, t.menu));
        }
        o.push_str(&format!("{}  ", t.shadow));
    }
    o.push_str(&format!("\x1b[{};{}H{}└{}┘{}  ", y + 3 + items.len(), x + 1, t.menu, "─".repeat(w - 2), t.shadow));
    o.push_str(&format!("\x1b[{};{}H{}{}", y + 4 + items.len(), x + 3, t.shadow, " ".repeat(w)));
    (x, w)
}

/// The keys (F1): a dialog in the middle, shadowed, with an OK button.
fn help_box(o: &mut String, cols: usize, rows: usize, t: &Theme, file: bool, views: bool) {
    let mut lines: Vec<String> = vec![
        "↑ ↓ PgUp PgDn Home End   scroll (j k space b g G too)".into(),
        "Wheel, scrollbars        scroll; drag a thumb, click an arrow".into(),
        "/  then Enter, n, N      find as you type; next, previous".into(),
        "[ ]                      previous, next part".into(),
        "Drag, double, triple     pick text, a word, a line".into(),
        "Alt+drag (or Ctrl+drag)  pick a rectangle of columns".into(),
        "Shift+↑ ↓                pick whole lines".into(),
        "Alt+Shift+arrows         pick a rectangle from the keyboard".into(),
        "R                        rectangle select for drags and Shift+arrows".into(),
        "Ctrl+C or y, Ctrl+A      copy the pick, pick everything".into(),
        "Right click, Shift+F10   the Edit menu where you are".into(),
        "← → (unwrapped), W       sideways; wrap long lines".into(),
        "F10, Alt+F E S V O H     the menus (arrows, Enter, Esc)".into(),
        "Options menu             the colors (kept for next time)".into(),
        "C                        show or hide the contents".into(),
    ];
    if views {
        lines.push("Tab, Shift+Tab           the next, previous view".into());
    }
    if file {
        lines.push("O                        open the file in its own app".into());
    }
    lines.push("Q or Esc                 back to the prompt".into());
    let w = lines.iter().map(|l| l.width()).max().unwrap_or(40) + 6;
    let w = w.min(cols.saturating_sub(4));
    let h = lines.len() + 6;
    let x = cols.saturating_sub(w) / 2;
    let y = rows.saturating_sub(h) / 2;
    o.push_str(&format!("\x1b[{};{}H{}╔{}╗", y + 1, x + 1, t.menu, "═".repeat(w - 2)));
    let title = " Keys and mouse ";
    o.push_str(&format!("\x1b[{};{}H{}{title}", y + 1, x + 1 + (w - title.width()) / 2, t.menu));
    let body: Vec<String> = std::iter::once(String::new()).chain(lines.into_iter()).chain([String::new(), String::new(), String::new()]).collect();
    for (k, l) in body.iter().enumerate() {
        let row = y + 2 + k;
        let s = if k == body.len() - 2 {
            let ok = "< OK >";
            format!("{}{}{}{}", " ".repeat((w - 2 - ok.len()) / 2), t.menu_sel, ok, t.menu)
        } else {
            format!("  {}", cut(l, w - 4))
        };
        let pad = (w - 2).saturating_sub(strip(&s).width());
        o.push_str(&format!("\x1b[{row};{}H{}║{s}{}{}║{}  ", x + 1, t.menu, t.menu, " ".repeat(pad), t.shadow));
    }
    o.push_str(&format!("\x1b[{};{}H{}╚{}╝{}  ", y + 2 + body.len(), x + 1, t.menu, "═".repeat(w - 2), t.shadow));
    o.push_str(&format!("\x1b[{};{}H{}{}", y + 3 + body.len(), x + 3, t.shadow, " ".repeat(w)));
}

impl Guide {
    fn select_all(&mut self) {
        let last = self.plain.len().saturating_sub(1);
        self.rect = false;
        self.sel = Some(((0, 0), (last, self.plain.get(last).map(|l| l.chars().count()).unwrap_or(0))));
    }
    /// Shift+↓ / Shift+↑: whole lines picked from the keyboard, from the
    /// top line shown (or the pick's end), the view following. In
    /// rectangle mode, a rectangle instead.
    fn pick_lines(&mut self, down: bool, body: usize) {
        if self.rect_mode {
            return self.pick_rect(if down { (0, 1) } else { (0, -1) }, body, usize::MAX);
        }
        let last = self.plain.len().saturating_sub(1);
        let len = |g: &Guide, i: usize| g.plain.get(i).map(|l| l.chars().count()).unwrap_or(0);
        let (a, end) = match self.sel {
            Some(((a, _), (b, _))) if self.sel_range().is_some() && !self.rect => (a, b),
            _ => {
                let a = self.top.min(last);
                self.rect = false;
                self.sel = Some(((a, 0), (a, len(self, a))));
                return;
            }
        };
        let b = if down { (end + 1).min(last) } else { end.saturating_sub(1) };
        let (from, to) = if b >= a { ((a, 0), (b, len(self, b))) } else { ((a, len(self, a)), (b, 0)) };
        self.sel = Some((from, to));
        self.follow(b, body);
    }
    fn follow(&mut self, line: usize, body: usize) {
        if line < self.top {
            self.top = line;
        } else if line >= self.top + body {
            self.top = line + 1 - body;
        }
    }
    /// Alt+Shift+arrows: a rectangle from the keyboard, from the first text
    /// column of the top line shown (or the rectangle's moving corner).
    fn pick_rect(&mut self, (dx, dy): (isize, isize), body: usize, room: usize) {
        let last = self.plain.len().saturating_sub(1);
        let (anchor, end) = match self.sel {
            Some((a, b)) if self.rect => (a, b),
            _ => {
                let l = self.top.min(last);
                let c = self.gutters.get(l).copied().unwrap_or(0) + self.left;
                ((l, c), (l, c))
            }
        };
        let line = (end.0 as isize + dy).clamp(0, last as isize) as usize;
        let col = (end.1 as isize + dx).max(0) as usize;
        let col = if dx == 0 && dy != 0 && end.1 == anchor.1 { col + 1 } else { col };
        self.rect = true;
        self.sel = Some((anchor, (line, col)));
        self.follow(line, body);
        // (Sideways too, when the corner goes past the edge.)
        if room != usize::MAX && self.unwrapped() {
            let g = self.gutters.get(line).copied().unwrap_or(0);
            if col < g + self.left {
                self.left = col.saturating_sub(g);
            } else if col >= self.left + room {
                self.left = col + 1 - room;
            }
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

/// What a drag is moving.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Drag {
    None,
    Text,
    VThumb(usize),
    SThumb(usize),
    HThumb(usize),
    Resize,
}

pub(crate) fn run(mut g: Guide, at: &str) {
    let mut o = std::io::stdout();
    let _ = crossterm::terminal::enable_raw_mode();
    let _ = write!(o, "\x1b[?1049h\x1b[?1002h\x1b[?1006h\x1b[H\x1b[2J");
    let _ = o.flush();
    let (c0, r0) = crossterm::terminal::size().unwrap_or((80, 24));
    g.layout(g.geom(c0 as usize, r0 as usize).cw);
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
    let mut drag = Drag::None;
    // (The last left press, for double and triple clicks: when, where, and
    // how many in a row.)
    let mut last_click: Option<(std::time::Instant, (usize, usize), u8)> = None;
    loop {
        let (c, r) = crossterm::terminal::size().unwrap_or((80, 24));
        let (cols, rows) = (c as usize, r as usize);
        g.draw(cols, rows);
        let geo = g.geom(cols, rows);
        let page = geo.bh.saturating_sub(1).max(1);
        let Ok(ev) = event::read() else { break };
        match ev {
            Event::Key(k) if k.kind != event::KeyEventKind::Release => {
                let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
                let alt = k.modifiers.contains(KeyModifiers::ALT);
                let shift = k.modifiers.contains(KeyModifiers::SHIFT);
                if !matches!(k.code, KeyCode::Modifier(_)) {
                    g.note.clear();
                }
                // (The keys box: any key closes it.)
                if g.help {
                    g.help = false;
                    continue;
                }
                // (A menu open: ← → the next menu, ↑ ↓ its items, Enter does one.)
                if let Some((m, i)) = g.open {
                    let menus = g.menus();
                    let n = menus.len();
                    let items = &menus[m].1;
                    let next = |from: usize, d: isize| {
                        let len = items.len() as isize;
                        let mut j = from as isize;
                        for _ in 0..len {
                            j = (j + d).rem_euclid(len.max(1));
                            if items.get(j as usize).is_some_and(|x| x.act.is_some() && x.on) {
                                break;
                            }
                        }
                        j as usize
                    };
                    match k.code {
                        KeyCode::Left => g.open = Some(((m + n - 1) % n, 0)),
                        KeyCode::Right => g.open = Some(((m + 1) % n, 0)),
                        KeyCode::Up => g.open = Some((m, next(i, -1))),
                        KeyCode::Down => g.open = Some((m, next(i, 1))),
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            g.open = None;
                            if let Some(it) = items.get(i).filter(|x| x.on) {
                                if let Some(a) = it.act {
                                    if g.act(a) {
                                        break;
                                    }
                                }
                            }
                        }
                        KeyCode::Char(ch) if !ctrl => {
                            // (An item's first letter does it.)
                            let hit = items.iter().find(|x| x.on && x.act.is_some() && x.label.chars().next().is_some_and(|c| c.eq_ignore_ascii_case(&ch))).and_then(|x| x.act);
                            g.open = None;
                            if let Some(a) = hit {
                                if g.act(a) {
                                    break;
                                }
                            }
                        }
                        _ => g.open = None,
                    }
                    continue;
                }
                if let Some((x, y, at)) = g.menu {
                    let items = g.context_items();
                    match k.code {
                        KeyCode::Up => g.menu = Some((x, y, at.saturating_sub(1))),
                        KeyCode::Down => g.menu = Some((x, y, (at + 1).min(items.len() - 1))),
                        KeyCode::Enter => {
                            g.menu = None;
                            if let Some(a) = items.get(at).filter(|x| x.on).and_then(|x| x.act) {
                                if g.act(a) {
                                    break;
                                }
                            }
                        }
                        _ => g.menu = None,
                    }
                    continue;
                }
                // (The menus: F10, or Alt and a title's first letter.)
                if k.code == KeyCode::F(10) && !shift {
                    g.open = Some((0, 0));
                    g.searching = false;
                    continue;
                }
                if alt && !shift {
                    if let KeyCode::Char(ch) = k.code {
                        if let Some(m) = g.menus().iter().position(|(t, _)| t.starts_with(ch.to_ascii_uppercase())) {
                            g.open = Some((m, 0));
                            g.searching = false;
                            continue;
                        }
                    }
                }
                if k.code == KeyCode::F(1) {
                    g.help = true;
                    continue;
                }
                // (Ctrl+C with text picked copies it; with none, goes back.)
                if ctrl && matches!(k.code, KeyCode::Char('c')) && g.sel_range().is_some() {
                    g.copy();
                    continue;
                }
                // (Alt+Shift+arrows: a rectangle from the keyboard.)
                if alt && shift && matches!(k.code, KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right) {
                    let d = match k.code {
                        KeyCode::Up => (0, -1),
                        KeyCode::Down => (0, 1),
                        KeyCode::Left => (-1, 0),
                        _ => (1, 0),
                    };
                    g.pick_rect(d, geo.bh, geo.cw);
                    continue;
                }
                if g.rect_mode && shift && matches!(k.code, KeyCode::Left | KeyCode::Right) {
                    g.pick_rect(if k.code == KeyCode::Left { (-1, 0) } else { (1, 0) }, geo.bh, geo.cw);
                    continue;
                }
                // (Without the mouse: y copies the pick (or the part being
                // read, or what's shown), Ctrl+A picks it all, Shift+F10 or
                // the Menu key opens the right-click menu.)
                if !g.searching && k.code == KeyCode::Char('y') && !ctrl {
                    if g.sel_range().is_some() {
                        g.copy();
                    } else if g.parts.is_empty() {
                        let a = g.top.min(g.plain.len().saturating_sub(1));
                        let b = (g.top + geo.bh).min(g.plain.len()).max(a + 1) - 1;
                        g.rect = false;
                        g.sel = Some(((a, 0), (b, g.plain.get(b).map(|l| l.chars().count()).unwrap_or(0))));
                        g.copy();
                    } else if let Some(i) = g.current_part() {
                        let a = g.parts[i].line;
                        let b = g.parts.get(i + 1).map(|p| p.line).unwrap_or(g.plain.len()).max(a + 1) - 1;
                        g.rect = false;
                        g.sel = Some(((a, 0), (b, g.plain.get(b).map(|l| l.chars().count()).unwrap_or(0))));
                        g.copy();
                    }
                    continue;
                }
                if ctrl && matches!(k.code, KeyCode::Char('a')) && !g.searching {
                    g.act(Act::SelectAll);
                    continue;
                }
                if k.code == KeyCode::Menu || (k.code == KeyCode::F(10) && shift) {
                    g.menu = Some((geo.x0 + 3, 3, 0));
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
                            if g.at.is_none() {
                                g.step(shift, true);
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
                    KeyCode::Char('t') if !g.tab_lines.is_empty() => {
                        g.act(Act::Bash);
                    }
                    KeyCode::Down | KeyCode::Up if shift => g.pick_lines(k.code == KeyCode::Down, geo.bh),
                    KeyCode::Left | KeyCode::Char('h') if g.unwrapped() => g.left = g.left.saturating_sub(8),
                    KeyCode::Right | KeyCode::Char('l') if g.unwrapped() => g.left += 8,
                    KeyCode::Char('w') if matches!(g.content(), Content::Lines(_)) => {
                        g.act(Act::Wrap);
                    }
                    KeyCode::Char('o') if g.file.is_some() => {
                        g.act(Act::OpenApp);
                    }
                    KeyCode::Char('r') => {
                        g.act(Act::RectMode);
                    }
                    KeyCode::Char('c') if g.has_parts() => {
                        g.act(Act::Sidebar);
                    }
                    KeyCode::Down | KeyCode::Char('j') => g.top += 1,
                    KeyCode::Up | KeyCode::Char('k') => g.top = g.top.saturating_sub(1),
                    KeyCode::PageDown | KeyCode::Char(' ') | KeyCode::Char('f') => g.top += page,
                    KeyCode::Char('d') => g.top += page / 2,
                    KeyCode::PageUp | KeyCode::Char('b') => g.top = g.top.saturating_sub(page),
                    KeyCode::Char('u') => g.top = g.top.saturating_sub(page / 2),
                    KeyCode::Home | KeyCode::Char('g') => g.top = 0,
                    KeyCode::End | KeyCode::Char('G') => g.top = usize::MAX / 2,
                    KeyCode::Char('/') => g.searching = true,
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
                let (col, row) = (m.column as usize, m.row as usize);
                let down = matches!(m.kind, MouseEventKind::Down(MouseButton::Left));
                g.note.clear();
                if g.help {
                    if matches!(m.kind, MouseEventKind::Down(_)) {
                        g.help = false;
                    }
                    continue;
                }
                // (A menu open: an item pointed at is lit, clicked it's done;
                // a click elsewhere closes it, on another title opens that.)
                if g.open.is_some() || g.menu.is_some() {
                    let (bx, by, bw, n) = if g.open.is_some() { g.open_box.unwrap_or_default() } else { g.ctx_box.unwrap_or_default() };
                    let item = (row >= by + 2 && row < by + 2 + n && col > bx && col < bx + bw).then(|| row - by - 2);
                    match m.kind {
                        MouseEventKind::Moved | MouseEventKind::Drag(_) => {
                            if let Some(i) = item {
                                if let Some(o) = g.open.as_mut() {
                                    o.1 = i;
                                } else if let Some(mm) = g.menu.as_mut() {
                                    mm.2 = i;
                                }
                            }
                            // (Dragged along the bar: the menu under the pointer.)
                            if row == 0 && g.open.is_some() {
                                if let Some(k) = g.menu_x.iter().position(|&(a, b)| (a..b).contains(&col)) {
                                    g.open = Some((k, 0));
                                }
                            }
                        }
                        MouseEventKind::Down(_) | MouseEventKind::Up(MouseButton::Left) if item.is_some() => {
                            let items = if g.open.is_some() { g.menus().get(g.open.unwrap().0).map(|x| x.1.clone()).unwrap_or_default() } else { g.context_items() };
                            g.open = None;
                            g.menu = None;
                            if let Some(a) = items.get(item.unwrap()).filter(|x| x.on).and_then(|x| x.act) {
                                if g.act(a) {
                                    break;
                                }
                            }
                        }
                        MouseEventKind::Down(_) => {
                            let title = (row == 0).then(|| g.menu_x.iter().position(|&(a, b)| (a..b).contains(&col))).flatten();
                            g.menu = None;
                            g.open = match (title, g.open) {
                                (Some(k), Some((m0, _))) if k != m0 => Some((k, 0)),
                                (Some(k), None) => Some((k, 0)),
                                _ => None,
                            };
                        }
                        _ => {}
                    }
                    continue;
                }
                // ---- drags in progress ----
                if let MouseEventKind::Drag(MouseButton::Left) = m.kind {
                    match drag {
                        Drag::VThumb(held) => {
                            g.top = pos_for_thumb(g.lines.len(), geo.bh, row.saturating_sub(3 + held), geo.bh.saturating_sub(2));
                            continue;
                        }
                        Drag::SThumb(held) => {
                            g.toc_top = pos_for_thumb(g.parts.len(), geo.bh, row.saturating_sub(3 + held), geo.bh.saturating_sub(2));
                            continue;
                        }
                        Drag::HThumb(held) => {
                            g.left = pos_for_thumb(g.widest, geo.cw, col.saturating_sub(geo.x0 + 2 + held), geo.cw.saturating_sub(2));
                            continue;
                        }
                        Drag::Resize => {
                            SIDE_W.store((col + 1).clamp(14, cols.saturating_sub(30)), std::sync::atomic::Ordering::Relaxed);
                            continue;
                        }
                        Drag::Text => {
                            // (Past the top or bottom: the text scrolls along.)
                            if row < 2 {
                                g.top = g.top.saturating_sub(1);
                            } else if row >= rows.saturating_sub(2) {
                                g.top += 1;
                            }
                            let p = if g.rect { g.col_at(col, row, geo) } else { g.pos_at(col, row, geo) };
                            if let Some(s) = g.sel.as_mut() {
                                s.1 = p;
                            }
                            continue;
                        }
                        Drag::None => {}
                    }
                }
                if let MouseEventKind::Up(_) = m.kind {
                    drag = Drag::None;
                    g.dragging = false;
                    continue;
                }
                let body = row >= 2 && row < 2 + geo.bh;
                let in_side = geo.sw > 0 && col < geo.sw;
                let in_text = body && col > geo.x0 && col < cols - 1;
                // ---- the menu bar, the search box ----
                if row == 0 && down {
                    if let Some(k) = g.menu_x.iter().position(|&(a, b)| (a..b).contains(&col)) {
                        g.open = Some((k, 0));
                        g.searching = false;
                    } else if (g.search_x.0..g.search_x.1).contains(&col) {
                        g.searching = true;
                    }
                    continue;
                }
                // ---- the top border: a view's chip ----
                if row == 1 && down {
                    if let Some(&(_, _, w)) = g.chips.iter().find(|&&(a, b, _)| (a..b).contains(&col)) {
                        g.switch(w);
                    }
                    continue;
                }
                // ---- the status bar's keys ----
                if row + 1 == rows && down {
                    match g.status_x.iter().find(|&&(a, b, _)| (a..b).contains(&col)) {
                        Some(&(_, _, Some(a))) => {
                            if g.act(a) {
                                break;
                            }
                        }
                        Some(&(_, _, None)) => g.open = Some((0, 0)),
                        None => {}
                    }
                    continue;
                }
                // ---- the sideways scrollbar, in the bottom border ----
                if row + 2 == rows && down && g.unwrapped() && g.widest > geo.cw && col > geo.x0 && col < cols - 1 {
                    let track = geo.cw.saturating_sub(2);
                    let cell = col - geo.x0 - 1;
                    if cell == 0 {
                        g.left = g.left.saturating_sub(4);
                    } else if cell + 1 >= geo.cw {
                        g.left += 4;
                    } else if let Some((a, b)) = thumb(g.widest, geo.cw, g.left, track) {
                        let c = cell - 1;
                        if c < a {
                            g.left = g.left.saturating_sub(geo.cw);
                        } else if c >= b {
                            g.left += geo.cw;
                        } else {
                            drag = Drag::HThumb(c - a);
                        }
                    }
                    continue;
                }
                // ---- the vertical scrollbars: the text's, the contents' ----
                let bar_hit = |total: usize, pos: usize| -> Option<Result<usize, usize>> {
                    // (Ok(new position) for an arrow or the track, Err(held) for the thumb.)
                    let r = row - 2;
                    if r == 0 {
                        return Some(Ok(pos.saturating_sub(1)));
                    }
                    if r + 1 >= geo.bh {
                        return Some(Ok(pos + 1));
                    }
                    let (a, b) = thumb(total, geo.bh, pos, geo.bh.saturating_sub(2))?;
                    let c = r - 1;
                    Some(if c < a { Ok(pos.saturating_sub(page)) } else if c >= b { Ok(pos + page) } else { Err(c - a) })
                };
                if body && down && col + 1 == cols {
                    match bar_hit(g.lines.len(), g.top) {
                        Some(Ok(p)) => g.top = p,
                        Some(Err(h)) => drag = Drag::VThumb(h),
                        None => {}
                    }
                    continue;
                }
                if body && down && geo.sw > 0 && col + 1 == geo.sw {
                    match bar_hit(g.parts.len(), g.toc_top) {
                        Some(Ok(p)) => g.toc_top = p.min(g.parts.len().saturating_sub(geo.bh)),
                        Some(Err(h)) => drag = Drag::SThumb(h),
                        None => {}
                    }
                    continue;
                }
                // ---- the main window's left edge: dragged, resizes the contents ----
                if down && geo.sw > 0 && col == geo.x0 && row >= 1 {
                    drag = Drag::Resize;
                    continue;
                }
                // (A click on an example's tab bar shows that version.)
                if down && in_text {
                    let line = g.top + row - 2;
                    if let Some(&(_, n)) = g.tab_lines.iter().find(|&&(l, _)| l == line) {
                        let c = col - geo.x0 - 1;
                        if (2..9).contains(&c) {
                            g.show_tab(n, false);
                        } else if (11..28).contains(&c) {
                            g.show_tab(n, true);
                        }
                        continue;
                    }
                }
                match m.kind {
                    MouseEventKind::Down(MouseButton::Left) if in_text => {
                        g.searching = false;
                        // (Alt or Ctrl held, or rectangle mode: a rectangle of columns.)
                        let rect = g.rect_mode || m.modifiers.intersects(KeyModifiers::ALT | KeyModifiers::CONTROL);
                        if rect {
                            let p = g.col_at(col, row, geo);
                            g.rect = true;
                            g.sel = Some((p, p));
                            drag = Drag::Text;
                            g.dragging = true;
                            continue;
                        }
                        let p = g.pos_at(col, row, geo);
                        let n = match last_click {
                            Some((t, q, n)) if q.0 == p.0 && t.elapsed() < std::time::Duration::from_millis(450) => n % 3 + 1,
                            _ => 1,
                        };
                        last_click = Some((std::time::Instant::now(), p, n));
                        // (Twice: the word; three times: the line.)
                        g.rect = false;
                        match n {
                            2 => g.pick_word(p),
                            3 => g.pick_line(p.0),
                            _ => {
                                g.sel = Some((p, p));
                                g.dragging = true;
                                drag = Drag::Text;
                            }
                        }
                    }
                    MouseEventKind::Down(MouseButton::Right) if in_text => {
                        // (Outside the pick: nothing picked, so Copy is off.)
                        let p = if g.rect { g.col_at(col, row, geo) } else { g.pos_at(col, row, geo) };
                        if g.sel_range().is_some_and(|(a, b)| if g.rect { p.0 < a.0 || p.0 > b.0 || p.1 < a.1 || p.1 >= b.1 } else { p < a || p > b }) {
                            g.sel = None;
                        }
                        g.menu = Some((col, row, if g.sel_range().is_some() { 0 } else { 1 }));
                    }
                    MouseEventKind::ScrollDown if in_side => g.toc_top = (g.toc_top + 3).min(g.parts.len().saturating_sub(geo.bh)),
                    MouseEventKind::ScrollUp if in_side => g.toc_top = g.toc_top.saturating_sub(3),
                    // (Shift+wheel, or a sideways wheel: ← →.)
                    MouseEventKind::ScrollDown | MouseEventKind::ScrollUp if g.unwrapped() && m.modifiers.contains(KeyModifiers::SHIFT) => {
                        g.left = if m.kind == MouseEventKind::ScrollDown { g.left + 8 } else { g.left.saturating_sub(8) }
                    }
                    MouseEventKind::ScrollRight if g.unwrapped() => g.left += 8,
                    MouseEventKind::ScrollLeft if g.unwrapped() => g.left = g.left.saturating_sub(8),
                    MouseEventKind::ScrollDown => g.top += 3,
                    MouseEventKind::ScrollUp => g.top = g.top.saturating_sub(3),
                    MouseEventKind::Down(MouseButton::Left) if in_side && body => {
                        let i = g.toc_top + row - 2;
                        if let Some(p) = g.parts.get(i) {
                            g.top = p.line;
                            g.searching = false;
                        }
                    }
                    MouseEventKind::Down(MouseButton::Left) => g.searching = false,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    let _ = write!(o, "\x1b[0m\x1b[?1006l\x1b[?1002l\x1b[?1049l\x1b[?25h");
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
    fn a_rectangle_picks_the_same_columns_of_each_line() {
        let mut g = Guide::views(vec![("text".into(), Content::Lines(Lines { lines: vec!["  1 │ abcdef".into(), "  2 │ ghijkl".into(), "  3 │ mn".into()], gutter: 6, wrap: false, parts: vec![] }))], 0, "x", "x");
        g.layout(40);
        g.rect = true;
        g.sel = Some(((2, 8), (0, 10)));
        assert_eq!(g.sel_text(), "cd\nij\n");
        // (From the keyboard: from the top line's first column of text.)
        g.sel = None;
        g.rect = false;
        g.pick_rect((1, 0), 10, 30);
        g.pick_rect((1, 0), 10, 30);
        g.pick_rect((0, 1), 10, 30);
        assert_eq!(g.sel_text(), "ab\ngh");
    }

    #[test]
    fn resets_go_back_to_the_window_not_the_terminal() {
        std::env::set_var("TESSH_VIEWER_COLORS", "classic");
        let t = theme();
        assert_eq!(SCHEMES[scheme_index()].name, "Classic");
        // (Blue on Edit's blue window: brightened; a monochrome scheme: its color.)
        let blue = build(&SCHEMES[1]);
        assert!(recolor("\x1b[34mx", &blue).starts_with("\x1b[96m"));
        let amber = build(&SCHEMES[3]);
        assert_eq!(recolor("\x1b[1;32mx", &amber), format!("\x1b[1;{}mx", amber.mono.unwrap()));
        let s = recolor("\x1b[1;34mfn\x1b[0m x\x1b[38;2;1;2;3my\x1b[39m", t);
        assert!(s.starts_with("\x1b[1;34mfn"), "{s:?}");
        assert!(s.contains(&format!("\x1b[0;{};{}m x", t.fg, t.bg)), "{s:?}");
        assert!(s.contains("\x1b[38;2;1;2;3my"), "{s:?}");
    }

    #[test]
    fn a_pick_in_code_leaves_out_the_gutter() {
        let mut g = Guide::new("Text.\n\n```tessh\nif True\n  print hi\n```\n");
        let a = g.plain.iter().position(|l| l.contains("if True")).unwrap();
        g.sel = Some(((a, 0), (a + 1, g.plain[a + 1].chars().count())));
        assert_eq!(g.sel_text(), "if True\n  print hi");
    }
}
