//! What the editor looks like, as ComposeBox draws it, in rows of text:
//! popups above (completion with the chosen item's help over the list,
//! history search, a button's list, help for a word, tooltips), the toolbar
//! row (its note on the left, its buttons on the right), the prompt bar, and
//! the editor: ❯, line numbers once there are several lines, indent guides,
//! highlighting, the selection, errors underlined, the grey suggestion.

use super::core::{Ed, Keys, MenuKind};
use super::screen::{cols_of, Frame};
use super::{Col, Hit, Line, Span, Sty};

pub type Rgb = (u8, u8, u8);

/// The terminal's colors, when it said (OSC 10/11), for the tints.
#[derive(Clone, Copy)]
pub struct Palette {
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
}

impl Palette {
    /// Theme.mix(bg, fg, t), when the colors are known.
    pub fn mix(&self, t: f32) -> Option<Col> {
        let (f, b) = (self.fg?, self.bg?);
        let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
        Some(Col::Rgb(m(b.0, f.0), m(b.1, f.1), m(b.2, f.2)))
    }
    pub fn panel(&self) -> Option<Col> {
        self.mix(0.045)
    }
    pub fn popup(&self) -> Option<Col> {
        self.mix(0.08)
    }
    pub fn dimmed(&self) -> Sty {
        match self.mix(0.55) {
            Some(c) => Sty::fg(c),
            None => Sty::default().dim(),
        }
    }
    /// The grey of a suggestion (mix(text, bg, 0.45)).
    pub fn ghost(&self) -> Sty {
        match self.mix(0.55) {
            Some(c) => Sty::fg(c),
            None => Sty { fg: Some(Col::Ansi(8)), ..Default::default() },
        }
    }
    pub fn accent_bg(&self) -> Col {
        match (self.fg, self.bg) {
            (Some(_), Some(b)) => {
                // (Theme.mix(bg, accent, 0.35), the accent a blue.)
                let a = (0x61u8, 0xafu8, 0xefu8);
                let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * 0.35).round() as u8;
                Col::Rgb(m(b.0, a.0), m(b.1, a.1), m(b.2, a.2))
            }
            _ => Col::Ansi(4),
        }
    }
}

pub fn kind_color(kind: &str) -> Option<Sty> {
    let a = |n| Some(Sty::fg(Col::Ansi(n)));
    match kind {
        "keyword" | "label" => a(5),
        "string" | "character" => a(2),
        "comment" | "punctuation" => Some(Sty::default().dim()),
        "number" | "constant" | "boolean" | "type" | "constructor" | "attribute" | "namespace" | "module" => a(3),
        "function" | "markup" => a(4),
        "operator" | "property" | "escape" | "special" => a(6),
        "variable" | "tag" => a(1),
        "embedded" => a(13),
        _ => None,
    }
}

pub struct ViewIn<'a> {
    pub pal: Palette,
    pub cols: usize,
    pub rows: usize,
    pub bar: &'a [Line],
    pub bar_right: &'a Line,
    pub hovered: Option<&'a Hit>,
    pub spans: &'a [(usize, usize, &'static str)],
    pub chat_ready: bool,
    pub ai_busy: String,
    pub mode_name: String,
    /// The final look (after Enter): no toolbar, popups or suggestion.
    pub final_: bool,
    pub tooltip_pos: Option<(u16, u16)>,
    pub screen_top: u16,
}

fn chip(label: &str, name: &'static str, v: &ViewIn) -> Vec<Span> {
    let hov = v.hovered == Some(&Hit::Chip(name));
    let bg = v.pal.mix(if hov { 0.17 } else { 0.10 });
    let mut st = v.pal.dimmed().with_bg(bg);
    st.bold = true;
    st.underline = hov;
    if bg.is_none() {
        st = Sty { fg: Some(Col::Ansi(if hov { 15 } else { 7 })), underline: hov, bold: true, ..Default::default() };
        return vec![Span::new(format!("[{label}]"), st).hit(Hit::Chip(name))];
    }
    vec![Span::new(format!(" {label} "), st).hit(Hit::Chip(name))]
}

/// The toolbar's buttons, as ComposeBox's modeTag row shows them.
pub fn chips(ed: &Ed, v: &ViewIn) -> Vec<Span> {
    let mut out: Vec<Span> = vec![];
    let mut add = |s: Vec<Span>| {
        if !out.is_empty() {
            out.push(Span::new(" ", Sty::default()));
        }
        out.extend(s);
    };
    if ed.status != 0 && ed.t.is_empty() && v.chat_ready && v.ai_busy.is_empty() {
        add(chip("✦ Fix", "fix", v));
    }
    add(chip("\u{f07b} Folders", "folders", v));
    add(chip("↺ History", "history", v));
    add(chip("Guide", "guide", v));
    let ind = ed.indent();
    let mode = match ind.mode {
        super::text::IndentMode::Spaces => "Spaces: ",
        super::text::IndentMode::Tabs => "Tabs: ",
        super::text::IndentMode::Mixed => "Mixed: ",
    };
    add(chip(&format!("{mode}{}{}", ind.size, if ed.detected().is_some() { " · detected" } else { "" }), "indent", v));
    if ed.jobs.1 > 0 {
        add(chip(&format!("⧗ Jobs {}", ed.jobs.0), "jobs", v));
    }
    let keys = match ed.keys {
        Keys::Vi => format!("vi · {}", v.mode_name.to_lowercase()),
        Keys::Emacs => "Emacs".into(),
        Keys::Vscode => "VS Code".into(),
    };
    add(chip(&keys, "keys", v));
    out
}

/// A tooltip's text for a button or widget.
pub fn tooltip(ed: &Ed, hit: &Hit) -> String {
    match hit {
        Hit::Chip(n) => match *n {
            "fix" => "Ask the AI to correct the command that just failed (Ctrl+Alt+X)".into(),
            "folders" => "cd history: this session's folders, then recent and frequent ones (Ctrl+Alt+D)".into(),
            "history" => "Command history (Ctrl+R)".into(),
            "lang" => "Input language: tessh (highlighting, checks, completion)".into(),
            "guide" => "The tessh guide: every feature, with examples. Opens at the selected word, or the word at the cursor (Shift+F1)".into(),
            "indent" => format!(
                "{}Indentation for tessh: spaces, tabs or mixed (a tab for every 8 columns), and the size of a level. Kept for each language. (Ctrl+Alt+S)",
                if ed.detected().is_some() { "Detected from the text. " } else { "" }
            ),
            "jobs" => format!("{} background job{} running ({} in all): click to show them (Ctrl+Alt+J)", ed.jobs.0, if ed.jobs.0 == 1 { "" } else { "s" }, ed.jobs.1),
            "keys" => "Editor keys: click for the next, Emacs, vi, VS Code (Ctrl+Alt+E)".into(),
            "hint" => keys_table(ed),
            "note" => if !ed.problem.is_empty() { ed.problem.clone() } else { ed.ai_note.clone() },
            _ => String::new(),
        },
        Hit::Widget(w) => format!("{} (the prompt bar's choices: Ctrl+Alt+P)", super::promptbar::widget_title(w)),
        Hit::Path(p) => super::promptbar::path_title(p),
        Hit::Ls(h) => super::ls::tooltip(ed, h),
        _ => String::new(),
    }
}

/// The editor's keys, for the hint's tooltip (keysHelp).
pub fn keys_table(ed: &Ed) -> String {
    let mut groups: Vec<(&str, Vec<(&str, &str)>)> = vec![
        ("Running", vec![("Enter", "Run the command (in a block: a new line; a blank line runs it)"), ("Right Ctrl", "Run it in the background (tap; or Ctrl+Enter, Alt+&)")]),
        (
            "Editing",
            vec![
                ("Right Shift", "New line (tap; or Shift+Enter, Alt+Enter, Ctrl+J); with completions shown, the highlighted one"),
                ("Left Shift", "Take the grey suggestion (tap; or → / End at the end of the line, Alt+/)"),
                ("Tab", "Complete; in the indentation, indent"),
                ("Shift+Tab", "Outdent"),
            ],
        ),
        ("Finding", vec![("Ctrl+R", "Search history"), ("Ctrl+Alt+D", "Folders: cd history"), ("Ctrl+Alt+J", "Background jobs"), ("Ctrl+Alt+P", "The prompt bar's choices (branches, contexts…)"), ("Ctrl+Alt+L", "The ls listing above the prompt, by keys")]),
        ("More", vec![("F1", "Help for the word at the cursor (on an empty line: these keys)"), ("Shift+F1", "The guide, at the word"), ("Shift+F10", "The text's menu (Copy, Paste, …); or the Menu key"), ("Ctrl+Alt+S", "Indentation: spaces or tabs, and its size"), ("Ctrl+Alt+E", "Editor keys: Emacs, vi, VS Code"), ("Ctrl+Alt+X", "Ask the AI to fix the command that failed")]),
    ];
    if ed.ai.lock().unwrap().command_ready {
        groups.push(("AI", vec![("# …", "Say what you want: the AI writes the command, for you to check")]));
    }
    let mut s = String::new();
    for (g, rows) in groups {
        s.push_str(g);
        s.push('\n');
        for (k, d) in rows {
            s.push_str(&format!("  {k:<12} {d}\n"));
        }
    }
    s.trim_end().to_string()
}

/// A box around lines (rounded corners), `w` columns wide inside.
fn boxed(lines: Vec<Line>, w: usize, v: &ViewIn) -> Vec<Line> {
    let bg = v.pal.popup();
    let edge = Sty { fg: Some(Col::Ansi(8)), bg, ..Default::default() };
    let mut out = vec![vec![Span::new(format!("╭{}╮", "─".repeat(w)), edge)]];
    for l in lines {
        let lw = super::width(&l);
        let mut row = vec![Span::new("│", edge)];
        for mut s in l {
            s.sty = s.sty.with_bg(bg);
            row.push(s);
        }
        row.push(Span::new(" ".repeat(w.saturating_sub(lw)), Sty { bg, ..Default::default() }));
        row.push(Span::new("│", edge));
        out.push(row);
    }
    out.push(vec![Span::new(format!("╰{}╯", "─".repeat(w)), edge)]);
    out
}

/// Text in lines of at most `w` columns, broken between words where it can
/// (a word longer than a line is split); newlines kept.
fn wrap_text(t: &str, w: usize) -> Vec<String> {
    let w = w.max(1);
    let mut out = vec![];
    for para in t.split('\n') {
        let mut line = String::new();
        let mut lw = 0;
        for word in para.split_inclusive(' ') {
            let ww = cols_of(word);
            let bare = cols_of(word.trim_end_matches(' '));
            if lw + bare > w && lw > 0 {
                out.push(line.trim_end().to_string());
                line = String::new();
                lw = 0;
            }
            if bare > w {
                for c in word.chars() {
                    let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
                    if lw + cw > w {
                        out.push(std::mem::take(&mut line));
                        lw = 0;
                    }
                    line.push(c);
                    lw += cw;
                }
                continue;
            }
            line.push_str(word);
            lw += ww;
        }
        out.push(line.trim_end().to_string());
    }
    out
}

/// Cut a line of spans to `w` columns (an ellipsis when cut).
fn fit(l: Line, w: usize) -> Line {
    if super::width(&l) <= w {
        return l;
    }
    // (Too long: cut so the … takes the last column. It went one past the
    // edge when the cut fell where one span ended, and that row's scrollbar
    // and border were pushed off: the jagged edge.)
    let room = w.saturating_sub(1);
    let mut out = vec![];
    let mut used = 0;
    for mut s in l {
        let mut t = String::new();
        let mut whole = true;
        for c in s.text.chars() {
            let cw = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if used + cw > room {
                whole = false;
                break;
            }
            t.push(c);
            used += cw;
        }
        if whole {
            out.push(s);
            continue;
        }
        t.push('…');
        s.text = t;
        out.push(s);
        break;
    }
    out
}

/// Wrap plain text to `w` columns.
pub fn wrap(text: &str, w: usize) -> Vec<String> {
    let mut out = vec![];
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_inclusive(' ') {
            if cols_of(&line) + cols_of(word) > w && !line.is_empty() {
                out.push(line.trim_end().to_string());
                line.clear();
            }
            let mut wd = word.to_string();
            while cols_of(&wd) > w {
                let cut: String = wd.chars().take(w).collect();
                out.push(cut.clone());
                wd = wd[cut.len()..].to_string();
            }
            line.push_str(&wd);
        }
        out.push(line.trim_end().to_string());
    }
    out
}

fn kind_glyph(kind: &str) -> (&'static str, Option<Col>) {
    match kind {
        "history" => ("↺", None),
        "command" | "program" => ("❯", Some(Col::Ansi(4))),
        "subcommand" => ("▸", Some(Col::Ansi(5))),
        "option" | "opt" => ("–", Some(Col::Ansi(6))),
        "dir" => ("▢", Some(Col::Ansi(3))),
        "file" => ("▫", None),
        "branch" => ("⑂", Some(Col::Ansi(2))),
        "host" => ("@", None),
        "variable" => ("$", None),
        _ => ("•", None),
    }
}

/// The popup over the editor, if one's open: its lines and left column.
fn popup(ed: &Ed, v: &ViewIn, anchor_col: usize, spans_for: &dyn Fn(&str) -> Vec<(usize, usize, &'static str)>) -> Option<(Vec<Line>, usize)> {
    let cols = v.cols;
    let sel_bg = v.pal.accent_bg();
    // vi's command line (ExLine), just above the editor, its choices above it.
    if ed.keys == Keys::Vi && ed.cmd.visible() {
        let w = cols.saturating_sub(2);
        let mut lines: Vec<Line> = vec![];
        for (i, (name, desc)) in ed.cmd.choices.iter().enumerate().take(8) {
            let bg = if i == ed.cmd.pick { Some(sel_bg) } else { None };
            let mut row = vec![Span::new(format!(" {name}"), Sty { bg, bold: true, ..Default::default() }), Span::new(format!("  {desc}"), v.pal.dimmed().with_bg(bg))];
            let rw = super::width(&row);
            row.push(Span::new(" ".repeat(w.saturating_sub(rw)), Sty { bg, ..Default::default() }));
            lines.push(fit(row, w));
        }
        if ed.cmd.is_open() {
            lines.push(vec![Span::new(format!("{}{}", ed.cmd.kind, ed.cmd.text), Sty::default())]);
        } else if let Some((m, err)) = &ed.cmd.message {
            lines.push(fit(vec![Span::new(m.clone(), if *err { Sty::fg(Col::Ansi(9)) } else { Sty::default() })], w));
        }
        return Some((boxed(lines, w, v), 0));
    }
    if let Some(m) = &ed.menu {
        let shown = ed.menu_shown();
        let w = (cols.saturating_sub(4)).min(60);
        let mut lines: Vec<Line> = vec![];
        if m.items.len() > 10 && !matches!(m.kind, MenuKind::Indent) {
            let q = if m.filter.is_empty() { Span::new("Filter…", v.pal.dimmed()) } else { Span::new(m.filter.text.clone(), Sty::default()) };
            lines.push(vec![Span::new(" ⌕ ", v.pal.dimmed()), q]);
        }
        let room = 14usize;
        ed.list_room.set(room);
        let top = m.top.min(shown.len().saturating_sub(room));
        let sel_at = shown.iter().position(|&i| i == m.sel).unwrap_or(0);
        let top = if sel_at < top { sel_at } else if sel_at >= top + room { sel_at + 1 - room } else { top };
        ed.list_top.set(top);
        let bar = scrollbar(shown.len(), room, top, &v.pal);
        for (k, &i) in shown.iter().skip(top).take(room).enumerate() {
            let it = &m.items[i];
            let mut st = if it.section { v.pal.dimmed().bold() } else if it.enabled { Sty::default() } else { v.pal.dimmed() };
            let on = i == m.sel && !it.section;
            if on || v.hovered == Some(&Hit::Row(i)) {
                st.bg = Some(if on { sel_bg } else { v.pal.mix(0.14).unwrap_or(Col::Ansi(8)) });
            }
            let mark = if it.checked { "✓ " } else { "  " };
            let text = format!("{mark}{}", it.label);
            let iw = if bar.is_empty() { w } else { w - 1 };
            let pad = iw.saturating_sub(cols_of(&text));
            let mut row = fit(vec![Span::new(format!("{text}{}", " ".repeat(pad)), st).hit(Hit::Row(i))], iw);
            if let Some(b) = bar.get(k) {
                row.push(b.clone());
            }
            lines.push(row);
        }
        if shown.is_empty() {
            lines.push(vec![Span::new("  Nothing matches.", v.pal.dimmed())]);
        }
        let x = ed.menu_x.unwrap_or(anchor_col).min(cols.saturating_sub(w + 2));
        return Some((boxed(lines, w, v), x));
    }
    if ed.comp.open && ed.comp.hist {
        let w = cols.saturating_sub(2);
        let n = ed.comp.items.len();
        let count = if n >= 300 { "300+".to_string() } else { format!("{n} command{}", if n == 1 { "" } else { "s" }) };
        let q = &ed.comp.query;
        let mut search = vec![Span::new(" ⌕ ", v.pal.dimmed())];
        if q.is_empty() {
            search.push(Span::new("Search history (every word, any order)", v.pal.dimmed()));
        } else {
            search.push(Span::new(q.text.clone(), Sty::default()));
        }
        let sw = super::width(&search);
        search.push(Span::new(" ".repeat(w.saturating_sub(sw + cols_of(&count) + 1)), Sty::default()));
        search.push(Span::new(count, v.pal.dimmed()));
        let mut lines = vec![search, vec![Span::new("─".repeat(w), Sty { fg: Some(Col::Ansi(8)), ..Default::default() })]];
        let room = 14usize.min(v.rows.saturating_sub(12)).max(3);
        ed.list_room.set(room);
        let top = if ed.comp.index < ed.comp.top { ed.comp.index } else if ed.comp.index >= ed.comp.top + room { ed.comp.index + 1 - room } else { ed.comp.top.min(n.saturating_sub(room)) };
        ed.list_top.set(top);
        let bar = scrollbar(n, room, top, &v.pal);
        let iw = if bar.is_empty() { w } else { w - 1 };
        for (k, (i, it)) in ed.comp.items.iter().enumerate().skip(top).take(room).enumerate() {
            let on = i == ed.comp.index;
            let bg = if on { Some(sel_bg) } else if v.hovered == Some(&Hit::Row(i)) { v.pal.mix(0.14) } else { None };
            let mut row = vec![Span::new(" ↺ ", v.pal.dimmed().with_bg(bg)).hit(Hit::Row(i))];
            row.extend(colored(&it.label, &spans_for(&it.label), bg, Hit::Row(i)));
            let rw = super::width(&row);
            row.push(Span::new(" ".repeat(iw.saturating_sub(rw)), Sty { bg, ..Default::default() }).hit(Hit::Row(i)));
            let mut row = fit(row, iw);
            if let Some(b) = bar.get(k) {
                row.push(b.clone());
            }
            lines.push(row);
        }
        if n == 0 {
            lines.push(vec![Span::new("   No commands match.", v.pal.dimmed())]);
        }
        return Some((boxed(lines, w, v), 0));
    }
    // (Kept up while keys come: it's worked out again as soon as they stop.)
    if ed.comp.open && !ed.comp.items.is_empty() {
        let w = cols.saturating_sub(4).min(80);
        let items = &ed.comp.items;
        let it = &items[ed.comp.index.min(items.len() - 1)];
        let any_doc = items.iter().any(|i| !i.doc.is_empty());
        let any_long = any_doc || items.iter().any(|i| i.detail.chars().count() > 60);
        let mut lines: Vec<Line> = vec![];
        if any_long {
            let text = if !it.doc.is_empty() { it.doc.clone() } else { it.detail.clone() };
            let h = if any_doc { 13 } else { 5 };
            let wrapped = wrap(&text, w.saturating_sub(2));
            let scroll = ed.comp.doc_scroll.min(wrapped.len().saturating_sub(h));
            for i in 0..h {
                let l = wrapped.get(scroll + i).cloned().unwrap_or_default();
                lines.push(vec![Span::new(format!(" {l}"), Sty::default()).hit(Hit::Doc)]);
            }
            lines.push(vec![Span::new("─".repeat(w), Sty { fg: Some(Col::Ansi(8)), ..Default::default() })]);
        }
        let room = 10usize;
        let n = items.len();
        let top = if ed.comp.index < ed.comp.top { ed.comp.index } else if ed.comp.index >= ed.comp.top + room { ed.comp.index + 1 - room } else { ed.comp.top.min(n.saturating_sub(room)) };
        let label_w = (w * 45 / 100).max(8);
        for (i, it) in items.iter().enumerate().skip(top).take(room) {
            let on = i == ed.comp.index;
            let bg = if on { Some(sel_bg) } else if v.hovered == Some(&Hit::Row(i)) { v.pal.mix(0.14) } else { None };
            let (g, gc) = kind_glyph(&it.kind);
            let gst = match gc {
                Some(c) => Sty::fg(c),
                None => v.pal.dimmed(),
            }
            .with_bg(bg);
            let label = fit(vec![Span::new(it.label.clone(), Sty { bg, ..Default::default() })], label_w);
            let lw = super::width(&label);
            let mut row = vec![Span::new(format!(" {g} "), gst).hit(Hit::Row(i))];
            for s in label {
                row.push(s.hit(Hit::Row(i)));
            }
            let detail = it.detail.lines().next().unwrap_or("").to_string();
            let room_d = w.saturating_sub(3 + lw + 2);
            row.push(Span::new("  ", Sty { bg, ..Default::default() }).hit(Hit::Row(i)));
            for s in fit(vec![Span::new(detail, v.pal.dimmed().with_bg(bg))], room_d) {
                row.push(s.hit(Hit::Row(i)));
            }
            let rw = super::width(&row);
            row.push(Span::new(" ".repeat(w.saturating_sub(rw)), Sty { bg, ..Default::default() }).hit(Hit::Row(i)));
            lines.push(row);
        }
        let x = anchor_col.saturating_sub(3).min(cols.saturating_sub(w + 2));
        return Some((boxed(lines, w, v), x));
    }
    if let Some((_, doc)) = &ed.hover {
        let w = cols.saturating_sub(4).min(78);
        let wrapped = wrap(doc.trim(), w.saturating_sub(2));
        let lines: Vec<Line> = wrapped.into_iter().take(15).map(|l| vec![Span::new(format!(" {l}"), Sty::default()).hit(Hit::Doc)]).collect();
        let x = anchor_col.saturating_sub(2).min(cols.saturating_sub(w + 2));
        return Some((boxed(lines, w, v), x));
    }
    if let Some((c, _, tip)) = &ed.tooltip {
        let w = tip.lines().map(cols_of).max().unwrap_or(0).min(cols.saturating_sub(4)).max(1);
        let lines: Vec<Line> = tip.lines().flat_map(|l| wrap(l, w)).map(|l| vec![Span::new(l, Sty::default())]).collect();
        let x = (*c as usize).saturating_sub(w / 2).min(cols.saturating_sub(w + 2));
        return Some((boxed(lines, w, v), x));
    }
    None
}

/// A list's scrollbar, a cell a row: ↑ and ↓ at its ends (a click moves a
/// row), the track (a click pages) and the thumb where the rows shown are,
/// which drags; none when everything shows. Drawn as cells of color, a
/// track a shade lighter than the popup and a thumb lighter still, so no
/// font draws it differently (only when the terminal's colors aren't
/// known: ░ and █).
fn scrollbar(n: usize, room: usize, top: usize, pal: &Palette) -> Vec<Span> {
    if n <= room || room == 0 {
        return vec![];
    }
    let arrows = room >= 4;
    let track_n = if arrows { room - 2 } else { room };
    let (at, size) = thumb_of(n, room, top);
    let (track, thumb) = (pal.mix(0.20), pal.mix(0.50));
    let cell = |on: bool| match (on, track, thumb) {
        (false, Some(t), _) => Span::new(" ", Sty { bg: Some(t), ..Default::default() }),
        (true, _, Some(t)) => Span::new(" ", Sty { bg: Some(t), ..Default::default() }),
        (false, _, _) => Span::new("░", Sty::fg(Col::Ansi(8))),
        (true, _, _) => Span::new("█", Sty::fg(Col::Ansi(7))),
    };
    let arrow = |s: &str| Span::new(s.to_string(), Sty { bg: track, ..Default::default() });
    let mut out = vec![];
    if arrows {
        out.push(arrow("↑").hit(Hit::ScrollArrow(true, room)));
    }
    for k in 0..track_n {
        let on = k >= at && k < at + size;
        out.push(if on { cell(true).hit(Hit::Thumb(n, room)) } else { cell(false).hit(Hit::Scroll(k < at)) });
    }
    if arrows {
        out.push(arrow("↓").hit(Hit::ScrollArrow(false, room)));
    }
    out
}

/// A scrollbar's thumb for `n` rows, `room` shown from `top`: where it
/// starts on the track and how long it is (the track is `room` less the
/// arrows).
pub fn thumb_of(n: usize, room: usize, top: usize) -> (usize, usize) {
    let track = if room >= 4 { room - 2 } else { room };
    let size = (track * room).div_ceil(n.max(1)).clamp(1, track.max(1));
    let at = (top.min(n.saturating_sub(room)) * (track - size)) / n.saturating_sub(room).max(1);
    (at, size)
}

/// A text colored by highlight spans.
fn colored(text: &str, spans: &[(usize, usize, &'static str)], bg: Option<Col>, hit: Hit) -> Line {
    let mut kinds: Vec<Option<&'static str>> = vec![None; text.len()];
    for &(s, e, k) in spans {
        for x in kinds.iter_mut().take(e.min(text.len())).skip(s) {
            *x = Some(k);
        }
    }
    let mut out = vec![];
    let mut start = 0;
    for i in 1..=text.len() {
        if i == text.len() || (kinds[i] != kinds[start] && text.is_char_boundary(i)) {
            let st = kinds[start].and_then(kind_color).unwrap_or_default().with_bg(bg);
            out.push(Span::new(text[start..i].replace('\n', "⏎"), st).hit(hit.clone()));
            start = i;
        }
    }
    out
}

pub struct Built {
    pub frame: Frame,
    /// Rows above the toolbar taken by a popup.
    pub popup_rows: usize,
}

/// The whole editor as rows.
pub fn build(ed: &Ed, v: &ViewIn, spans_for: &dyn Fn(&str) -> Vec<(usize, usize, &'static str)>) -> Built {
    let cols = v.cols.max(10);
    let fill = if v.final_ { None } else { v.pal.panel() };
    let mut rows: Vec<Line> = vec![];

    // ---- the editor's text rows (built first: the popup anchors to them) ----
    let text = &ed.t.text;
    let ind = ed.indent();
    let tw = ind.tab_width().max(1);
    let lines: Vec<(usize, usize)> = {
        let mut v = vec![];
        let mut a = 0;
        for l in text.split('\n') {
            v.push((a, a + l.len()));
            a += l.len() + 1;
        }
        v
    };
    let multi = lines.len() > 1 && ed.line_numbers;
    let gutter = if multi { lines.len().to_string().len() + 1 } else { 0 };
    let text_x = 2 + gutter;
    let width = cols.saturating_sub(text_x + 1).max(4);
    let cur_line = ed.t.line_of(ed.t.cursor);
    let sel = ed.t.selection();
    // (vi's V-BLOCK: each line's part.)
    let block: Vec<(usize, usize)> = if ed.keys == Keys::Vi && ed.vi.visual == Some(super::vi::BLOCK) { ed.vi.block_ranges(&ed.t) } else { vec![] };
    // (Indent levels per line, blank lines as deep as the shallower of the lines around.)
    let size = ind.size.max(1);
    let depth: Vec<usize> = {
        let raw: Vec<Option<usize>> = lines.iter().map(|&(a, b)| {
            let l = &text[a..b];
            (!l.trim().is_empty()).then(|| ind.cols_of(l))
        }).collect();
        (0..raw.len()).map(|i| raw[i].unwrap_or_else(|| {
            let up = (0..i).rev().find_map(|j| raw[j]).unwrap_or(0);
            let down = (i + 1..raw.len()).find_map(|j| raw[j]).unwrap_or(0);
            up.min(down)
        })).collect()
    };
    let cur_level = (depth.get(cur_line).copied().unwrap_or(0) / size).saturating_sub(1);
    let block_of = |level: usize, line: usize| -> bool {
        // (The cursor's block: the run of lines deeper than this level around it.)
        let lim = level * size;
        if depth.get(cur_line).copied().unwrap_or(0) <= lim || depth.get(line).copied().unwrap_or(0) <= lim {
            return false;
        }
        let (lo, hi) = (line.min(cur_line), line.max(cur_line));
        (lo..=hi).all(|i| depth[i] > lim)
    };
    let ghost_shown = !v.final_ && !ed.ghost.is_empty() && ed.at_suggest_end();
    let err_at = |p: usize| ed.errors.iter().any(|e| p >= e.0 && p < e.0 + e.1.max(1));
    let mut text_rows: Vec<Line> = vec![];
    let mut cur_abs: Option<(usize, usize)> = None;
    let mut anchor_col = text_x;
    for (li, &(a, b)) in lines.iter().enumerate() {
        let mut row: Line = vec![];
        let mut col = 0usize;
        let push_cell = |row: &mut Line, s: String, st: Sty, hit: Hit, col: &mut usize, rows_out: &mut Vec<Line>| {
            let w = cols_of(&s);
            if *col + w > width {
                rows_out.push(std::mem::take(row));
                *col = 0;
            }
            *col += w;
            row.push(Span::new(s, st).hit(hit));
        };
        let mut wrapped: Vec<Line> = vec![];
        let lead_end = a + (text[a..b].len() - text[a..b].trim_start_matches([' ', '\t']).len());
        let mut p = a;
        let mut vcol = 0usize;
        while p < b {
            let ch = text[p..].chars().next().unwrap();
            let n = p + ch.len_utf8();
            if p == ed.t.cursor {
                cur_abs = Some((text_rows.len() + wrapped.len(), col));
            }
            if p == ed.comp.start && ed.comp.open {
                anchor_col = text_x + col;
            }
            let kind = v.spans.iter().rev().find(|s| p >= s.0 && p < s.1).map(|s| s.2);
            let mut st = kind.and_then(kind_color).unwrap_or_default();
            if err_at(p) {
                st.curl = Some(Col::Ansi(9));
            }
            if let Some((sa, sb)) = sel {
                if p >= sa && p < sb {
                    st.bg = Some(v.pal.accent_bg());
                }
            }
            if block.iter().any(|&(ba, bb)| p >= ba && p < bb) {
                st.bg = Some(v.pal.accent_bg());
            }
            let cell = if ch == '\t' {
                let w = tw - vcol % tw;
                " ".repeat(w)
            } else {
                ch.to_string()
            };
            // Indent guides: a line at each level down the indented lines.
            // (Level k's at column k × the indent: the first at column 0,
            // under the block's opening line's first character.)
            let guide = !v.final_ && ed.indent_guides && p < lead_end && ch == ' ' && vcol % size == 0 && vcol / size < depth[li] / size;
            if guide && sel.is_none_or(|(sa, sb)| p < sa || p >= sb) {
                let level = vcol / size;
                let on = block_of(level, li) && level == cur_level;
                let g = match v.pal.mix(if on { 0.4 } else { 0.22 }) {
                    Some(c) => Sty::fg(c),
                    None => Sty::default().dim(),
                };
                push_cell(&mut row, "│".into(), g, Hit::Text(p), &mut col, &mut wrapped);
            } else {
                push_cell(&mut row, cell.clone(), st, Hit::Text(p), &mut col, &mut wrapped);
            }
            vcol += cols_of(&cell).max(1);
            p = n;
        }
        // (A blank line inside a block: its guides too.)
        if !v.final_ && ed.indent_guides && a == b && depth[li] >= size {
            let levels = depth[li] / size;
            for level in 0..levels {
                let on = block_of(level, li) && level == cur_level;
                let g = match v.pal.mix(if on { 0.4 } else { 0.22 }) {
                    Some(cc) => Sty::fg(cc),
                    None => Sty::default().dim(),
                };
                push_cell(&mut row, "│".into(), g, Hit::Text(a), &mut col, &mut wrapped);
                if level + 1 < levels {
                    for _ in 1..size {
                        push_cell(&mut row, " ".into(), Sty::default(), Hit::Text(a), &mut col, &mut wrapped);
                    }
                }
            }
            if ed.t.cursor == a {
                cur_abs = Some((text_rows.len() + wrapped.len(), 0));
            }
        }
        if ed.t.cursor == b && (cur_abs.is_none() || li == cur_line) && ed.t.cursor >= a {
            if cur_abs.is_none_or(|(r, _)| r < text_rows.len()) {
                cur_abs = Some((text_rows.len() + wrapped.len(), col));
            }
        }
        if ed.comp.open && ed.comp.start == b && li == lines.len() - 1 {
            anchor_col = text_x + col;
        }
        // The suggestion, greyed, after the cursor (at the text's end, or before closers).
        if ghost_shown && li == lines.len() - 1 {
            let g = ed.ghost.clone();
            push_cell(&mut row, g, v.pal.ghost(), Hit::Text(b), &mut col, &mut wrapped);
        }
        // (The rest of the row: a click there is the line's end.)
        if !v.final_ {
            row.push(Span::new(" ".repeat(width.saturating_sub(col)), Sty { bg: fill, ..Default::default() }).hit(Hit::Text(b)));
        }
        wrapped.push(row);
        for (k, w) in wrapped.into_iter().enumerate() {
            // Each row: ❯ on the first, the line number on a line's first row.
            let mut full: Line = vec![];
            if li == 0 && k == 0 {
                let st = Sty::fg(Col::Ansi(if ed.status != 0 { 1 } else { 2 }));
                full.push(Span::new("❯ ", st).hit(Hit::Text(a)));
            } else {
                full.push(Span::new("  ", Sty::default()).hit(Hit::Text(a)));
            }
            if multi {
                let numst = if li == cur_line { Sty::default() } else { v.pal.dimmed() };
                let num = if k == 0 { format!("{:>w$} ", li + 1, w = gutter - 1) } else { " ".repeat(gutter) };
                full.push(Span::new(num, numst).hit(Hit::Text(a)));
            }
            full.extend(w);
            text_rows.push(full);
        }
    }
    let (cur_r, cur_c) = cur_abs.unwrap_or((0, 0));
    // The placeholder, on an empty editor.
    if ed.t.is_empty() && !v.final_ && !(ed.keys == Keys::Vi && v.mode_name == "NORMAL") {
        let hint = if v.chat_ready { "Type a command, or # and a request for the AI — hover for keys" } else { "Type a command — hover for keys" };
        let room = cols.saturating_sub(text_x + 2);
        let mut st = v.pal.dimmed();
        st.dim = v.pal.bg.is_none();
        text_rows[0] = vec![Span::new("❯ ", Sty::fg(Col::Ansi(if ed.status != 0 { 1 } else { 2 }))).hit(Hit::Text(0))];
        text_rows[0].extend(fit(vec![Span::new(hint, st).hit(Hit::Chip("hint"))], room));
    }
    // (Ten rows at most; past that a window around the cursor.)
    let max_rows = 10usize.max(v.rows / 3);
    let mut skip = 0;
    if text_rows.len() > max_rows {
        skip = cur_r.saturating_sub(max_rows - 1).min(text_rows.len() - max_rows);
    }
    let shown: Vec<Line> = text_rows.into_iter().skip(skip).take(max_rows).collect();

    // ---- the toolbar row ----
    if !v.final_ {
        // (A note wraps onto more rows, never cut short. A problem with the
        // text goes under the editor instead: the prompt line stays put.)
        let note: Option<(String, Sty)> = if !v.ai_busy.is_empty() {
            let what = if v.ai_busy == "fix" { "fixing the command…" } else { "writing the command…" };
            Some((format!("◜ {what}  esc cancels"), Sty::fg(Col::Ansi(4))))
        } else if !ed.ai_note.is_empty() {
            Some((format!("✦ {}", ed.ai_note), Sty::fg(Col::Ansi(3))))
        } else {
            None
        };
        let right = chips(ed, v);
        let rw = super::width(&right);
        let room = cols.saturating_sub(rw + 3).max(20).min(cols.saturating_sub(2).max(1));
        let lines = note.as_ref().map(|(t, _)| wrap_text(t, room)).unwrap_or_else(|| vec![String::new()]);
        for (i, text) in lines.into_iter().enumerate() {
            let mut row: Line = vec![Span::new(if i == 0 { " " } else { "   " }, Sty::default())];
            if let Some((_, sty)) = &note {
                let sp = Span::new(text, sty.clone());
                row.push(if v.ai_busy.is_empty() { sp.hit(Hit::Chip("note")) } else { sp });
            }
            if i == 0 {
                let lw = super::width(&row);
                if lw + rw + 1 <= cols {
                    row.push(Span::new(" ".repeat(cols.saturating_sub(lw + rw + 1)), Sty::default()));
                    row.extend(right.iter().cloned());
                } else {
                    // (Too narrow for both: the chips get a row of their own.)
                    rows.push(row);
                    row = vec![Span::new(" ".repeat(cols.saturating_sub(rw + 1)), Sty::default())];
                    row.extend(right.iter().cloned());
                }
            }
            rows.push(row);
        }
    }
    // ---- the prompt bar ----
    for (i, l) in v.bar.iter().enumerate() {
        let mut row = fit(l.clone(), cols);
        if i == 0 && !v.bar_right.is_empty() {
            let lw = super::width(&row);
            let rw = super::width(v.bar_right);
            if lw + rw + 2 <= cols {
                row.push(Span::new(" ".repeat(cols - lw - rw), Sty::default()));
                row.extend(v.bar_right.iter().cloned());
            }
        }
        rows.push(row);
    }
    let editor_top = rows.len();
    rows.extend(shown);
    // ---- a problem with the text: under it, whole, wrapped ----
    if !v.final_ && !ed.problem.is_empty() {
        let sty = Sty { fg: Some(Col::Ansi(9)), bold: true, ..Default::default() };
        for (i, t) in wrap_text(&format!("⚠ {}", ed.problem), cols.saturating_sub(4).max(10)).into_iter().enumerate() {
            rows.push(vec![Span::new(if i == 0 { "  " } else { "    " }, Sty::default()), Span::new(t, sty.clone()).hit(Hit::Chip("note"))]);
        }
    }
    let cursor = if v.final_ { None } else { Some((editor_top + cur_r.saturating_sub(skip), text_x + cur_c)) };

    // ---- the popup: drawn over what's around the editor, never moving it ----
    let mut overlay = None;
    if !v.final_ {
        if let Some((lines, x)) = popup(ed, v, anchor_col, spans_for) {
            let mut ocur = None;
            if ed.keys == Keys::Vi && ed.cmd.is_open() {
                let n = ed.cmd.choices.len().min(8);
                ocur = Some((1 + n, 1 + 1 + cols_of(&ed.cmd.text[..ed.cmd.cursor.min(ed.cmd.text.len())])));
            } else if ed.comp.hist && ed.comp.open {
                ocur = Some((1, 1 + 3 + cols_of(&ed.comp.query.text[..ed.comp.query.cursor])));
            }
            overlay = Some(super::screen::Overlay { lines, col: x, cursor: ocur });
        }
    }
    let popup_rows = 0;
    Built { frame: Frame { rows, cursor, fill, overlay, bare: 0 }, popup_rows }
}

#[cfg(test)]
mod fit_tests {
    use super::*;

    #[test]
    fn a_cut_row_is_never_wider_than_its_room() {
        // (Spans that fill the room exactly, then more: the … goes in the room.)
        let l = vec![Span::new("abcde", Sty::default()), Span::new("fghij", Sty::default()), Span::new("k", Sty::default())];
        for w in 1..12 {
            let f = fit(l.clone(), w);
            assert!(super::super::width(&f) <= w, "{w}: {:?}", f.iter().map(|s| s.text.clone()).collect::<Vec<_>>());
        }
        assert_eq!(fit(l.clone(), 10).iter().map(|s| s.text.as_str()).collect::<String>(), "abcdefghi…");
        assert_eq!(fit(l, 11).iter().map(|s| s.text.as_str()).collect::<String>(), "abcdefghijk");
    }
}
