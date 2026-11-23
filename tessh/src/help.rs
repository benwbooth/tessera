//! `help`: the tessh guide (doc/guide.md, built in), by topic. In a terminal
//! it's drawn with colors and highlighted examples; in a notebook it's
//! Markdown. `help $obj` is Python's help for a value.
use pyo3::prelude::*;
use pyo3::types::PyString;

pub const GUIDE: &str = include_str!("../doc/guide.md");

/// The Advanced tessh Scripting Guide (doc/advanced/*.md, the ABS Guide
/// ported): its files, by name, in order.
pub const ADVANCED: &[(&str, &str)] = include!("../doc/advanced.rs");

/// The Advanced guide as one Markdown text.
pub fn advanced() -> String {
    let mut s = String::new();
    for (_, md) in ADVANCED {
        s.push_str(md.trim_end());
        s.push_str("\n\n");
    }
    s
}

pub struct Topic {
    pub title: String,
    pub part: String,
    pub keys: Vec<String>,
    /// The topic's Markdown, its heading first.
    pub body: String,
}

/// The guide's topics in order: a `#` or `##` heading with a
/// `<!-- help: key… -->` line under it. (`#` lines in code are code.)
pub fn topics() -> Vec<Topic> {
    topics_of(GUIDE)
}

/// The Advanced guide's topics: its chapters and their sections.
pub fn advanced_topics() -> Vec<Topic> {
    topics_of(&advanced())
}

fn topics_of(guide: &str) -> Vec<Topic> {
    let mut out: Vec<Topic> = Vec::new();
    let mut part = String::new();
    let mut fence = false;
    let mut pending: Option<(String, String)> = None; // a heading, waiting for its keys line
    for line in guide.lines() {
        if line.starts_with("```") {
            fence = !fence;
        }
        if !fence && !line.starts_with("```") {
            if let Some(k) = line.strip_prefix("<!-- help:").and_then(|r| r.strip_suffix("-->")) {
                if let Some((title, heading)) = pending.take() {
                    let keys = k.split_whitespace().map(|s| s.to_lowercase()).collect();
                    out.push(Topic { title, part: part.clone(), keys, body: heading + "\n" });
                }
                continue;
            }
            let h2 = line.strip_prefix("## ");
            let h1 = line.strip_prefix("# ");
            if h2.is_some() || h1.is_some() {
                let title = h2.or(h1).unwrap().trim().to_string();
                if h1.is_some() && !out.is_empty() {
                    part = title.clone();
                }
                pending = Some((title, line.to_string()));
                continue;
            }
        }
        if let Some((_, heading)) = pending.take() {
            // (A part's heading, with no topic of its own.)
            let _ = heading;
        }
        if let Some(t) = out.last_mut() {
            t.body.push_str(line);
            t.body.push('\n');
        }
    }
    for t in &mut out {
        let b = t.body.trim_end().to_string();
        t.body = b + "\n";
    }
    out
}

/// What `help word` shows: a topic it names, else the topics that mention it.
/// The tessh guide first, then the Advanced guide (`help advanced` is its
/// contents, `help abs-loops` a chapter of it).
pub fn lookup(word: &str) -> Option<String> {
    let ts = topics();
    let w = word.trim().to_lowercase();
    if w.is_empty() {
        return Some(overview(&ts));
    }
    if w == "guide" || w == "all" {
        return Some(guide());
    }
    if w == "keys" || w == "editor" {
        return Some(crate::repl::keys_help());
    }
    let named = |ts: &[Topic]| ts.iter().position(|t| t.keys.contains(&w) || t.title.to_lowercase() == w);
    if let Some(i) = named(&ts) {
        return Some(clean(&ts[i].body));
    }
    let adv = advanced_topics();
    if let Some(i) = named(&adv) {
        return Some(if i == 0 { overview(&adv) } else { clean(&adv[i].body) });
    }
    let hits_in = |ts: &[Topic]| -> Vec<usize> { (0..ts.len()).filter(|&i| ts[i].keys.iter().any(|k| k.starts_with(&w)) || mentions(&ts[i].body, word.trim())).collect() };
    let (ts, hits) = match hits_in(&ts) {
        h if !h.is_empty() => (ts, h),
        _ => {
            let h = hits_in(&adv);
            (adv, h)
        }
    };
    let hits: Vec<&Topic> = hits.into_iter().map(|i| &ts[i]).collect();
    match hits.len() {
        0 => None,
        1 => Some(clean(&hits[0].body)),
        _ => {
            let mut s = format!("# `{}` is in\n\n", word.trim());
            for t in hits {
                s.push_str(&format!("- **{}**: `help {}`\n", t.title, t.keys[0]));
            }
            Some(s)
        }
    }
}

/// Whether a topic's examples or code spans use the word, as a word.
fn mentions(body: &str, word: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut fence = false;
    for line in body.lines() {
        if line.starts_with("```") {
            fence = !fence;
            continue;
        }
        let code: Vec<&str> = if fence { vec![line] } else { line.split('`').skip(1).step_by(2).collect() };
        for c in code {
            let mut start = 0;
            while let Some(i) = c[start..].find(word) {
                let a = start + i;
                let b = a + word.len();
                let before = c[..a].chars().next_back();
                let after = c[b..].chars().next();
                let wordy = word.chars().all(is_word);
                if !wordy || (!before.is_some_and(is_word) && !after.is_some_and(is_word)) {
                    return true;
                }
                start = b;
            }
        }
    }
    false
}

fn overview(ts: &[Topic]) -> String {
    let mut s = clean(&ts[0].body);
    if ts.get(1).is_some_and(|t| t.part.is_empty()) {
        s.push('\n');
    }
    let mut part = "";
    for t in &ts[1..] {
        if t.part != part {
            part = &t.part;
            s.push_str(&format!("\n## {part}\n\n"));
        }
        s.push_str(&format!("- `help {}`: {}\n", t.keys[0], t.title));
    }
    s
}

/// The Markdown shown: no `help:` lines, no fence flags.
fn clean(md: &str) -> String {
    md.lines()
        .filter(|l| !l.starts_with("<!-- help:"))
        .map(|l| if l.starts_with("```tessh") { "```tessh" } else { l })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// The terminal's width, for wrapping (at most 100 columns: lines that
/// long read badly).
fn width() -> usize {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    let cols = if unsafe { libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) } == 0 && ws.ws_col > 0 { ws.ws_col as usize } else { std::env::var("COLUMNS").ok().and_then(|c| c.parse().ok()).unwrap_or(80) };
    cols.clamp(20, 100)
}

/// How wide a styled text shows (escape sequences take no room).
fn visible(s: &str) -> usize {
    let mut n = 0;
    let mut esc = false;
    for c in s.chars() {
        if esc {
            if c.is_ascii_alphabetic() {
                esc = false;
            }
        } else if c == '\x1b' {
            esc = true;
        } else {
            n += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        }
    }
    n
}

/// A styled paragraph in lines of `width`, each after `first` (the first)
/// or `rest` (the others).
fn wrap(text: &str, width: usize, first: &str, rest: &str) -> String {
    let mut out = String::new();
    let mut line = String::from(first);
    let mut used = visible(first);
    let mut empty = true;
    for w in text.split(' ').filter(|w| !w.is_empty()) {
        let n = visible(w);
        if !empty && used + 1 + n > width {
            out.push_str(&line);
            out.push('\n');
            line = String::from(rest);
            used = visible(rest);
            empty = true;
        }
        if !empty {
            line.push(' ');
            used += 1;
        }
        line.push_str(w);
        used += n;
        empty = false;
    }
    out.push_str(&line);
    out.push('\n');
    out
}

/// A table's rows, boxed, columns as wide as their widest cell (wrapped
/// when that's too wide for the terminal).
fn table(rows: &[Vec<String>], width: usize) -> String {
    let rows: Vec<Vec<String>> = rows.iter().map(|r| r.iter().map(|c| inline(c.trim(), "")).collect()).collect();
    let ncol = rows.iter().map(Vec::len).max().unwrap_or(0);
    if ncol == 0 {
        return String::new();
    }
    let mut w: Vec<usize> = (0..ncol).map(|i| rows.iter().filter_map(|r| r.get(i)).map(|c| visible(c)).max().unwrap_or(0).max(1)).collect();
    // (Too wide: the widest columns give way.)
    let room = width.saturating_sub(3 * ncol + 1).max(ncol * 6);
    while w.iter().sum::<usize>() > room {
        let (i, _) = w.iter().enumerate().max_by_key(|(_, x)| **x).unwrap();
        if w[i] <= 6 {
            break;
        }
        w[i] -= 1;
    }
    let dim = |s: String| format!("\x1b[2m{s}\x1b[0m");
    let rule = |l: &str, m: &str, r: &str| dim(format!("{l}{}{r}", w.iter().map(|x| "─".repeat(x + 2)).collect::<Vec<_>>().join(m)));
    let mut out = rule("┌", "┬", "┐") + "\n";
    for (ri, r) in rows.iter().enumerate() {
        let cells: Vec<Vec<String>> = (0..ncol).map(|i| wrap(r.get(i).map(String::as_str).unwrap_or(""), w[i], "", "").trim_end_matches('\n').split('\n').map(String::from).collect()).collect();
        let h = cells.iter().map(Vec::len).max().unwrap_or(1);
        for k in 0..h {
            out.push_str(&dim("│".into()));
            for i in 0..ncol {
                let c = cells[i].get(k).cloned().unwrap_or_default();
                let pad = w[i].saturating_sub(visible(&c));
                if ri == 0 {
                    out.push_str(&format!(" \x1b[1m{c}\x1b[0m{} ", " ".repeat(pad)));
                } else {
                    out.push_str(&format!(" {c}\x1b[0m{} ", " ".repeat(pad)));
                }
                out.push_str(&dim("│".into()));
            }
            out.push('\n');
        }
        if ri == 0 && rows.len() > 1 {
            out.push_str(&(rule("├", "┼", "┤") + "\n"));
        }
    }
    out + &rule("└", "┴", "┘") + "\n"
}

/// Markdown for a terminal: headings as banners, paragraphs and lists
/// wrapped to the terminal, tables boxed, tessh code highlighted as the
/// editor does it, other code dim. Plain (color false): as it is.
pub fn terminal(md: &str, color: bool) -> String {
    terminal_at(md, color, width())
}

/// Markdown for a terminal, `width` columns wide.
pub fn terminal_at(md: &str, color: bool, width: usize) -> String {
    if !color {
        return md.to_string();
    }
    let mut out = String::new();
    let mut fence: Option<String> = None;
    let mut para: Vec<String> = vec![];
    let mut tbl: Vec<Vec<String>> = vec![];
    // (A list item's text so far: its marker and the text.)
    let mut item: Option<(String, String)> = None;
    fn flush(out: &mut String, para: &mut Vec<String>, item: &mut Option<(String, String)>, tbl: &mut Vec<Vec<String>>, width: usize) {
        if !para.is_empty() {
            let text = inline(&para.join(" "), "");
            out.push_str(&wrap(&text, width, "", ""));
            para.clear();
        }
        if let Some((marker, text)) = item.take() {
            let lead = marker.len() - marker.trim_start().len();
            let bullet = if marker.trim_start().starts_with(['-', '*', '+']) { format!("{}\x1b[36m•\x1b[0m ", " ".repeat(lead)) } else { format!("{}\x1b[36m{}\x1b[0m ", " ".repeat(lead), marker.trim()) };
            let hang = " ".repeat(lead + visible(marker.trim()) + 1);
            out.push_str(&wrap(&inline(&text, ""), width, &bullet, &hang));
        }
        if !tbl.is_empty() {
            out.push_str(&table(tbl, width));
            tbl.clear();
        }
    }
    for line in md.lines() {
        if let Some(info) = line.trim_start().strip_prefix("```") {
            flush(&mut out, &mut para, &mut item, &mut tbl, width);
            fence = if fence.is_some() { None } else { Some(info.trim().to_string()) };
            continue;
        }
        match &fence {
            Some(lang) if lang.starts_with("tessh") => {
                out.push_str("  \x1b[2m│\x1b[0m ");
                out.push_str(&crate::repl::highlight(line));
                out.push('\n');
                continue;
            }
            Some(lang) if lang.starts_with("bash") || lang.starts_with("sh") => {
                out.push_str("  \x1b[2m│\x1b[0m ");
                out.push_str(&crate::repl::highlight_bash(line));
                out.push('\n');
                continue;
            }
            Some(_) => {
                out.push_str("  \x1b[2m│ ");
                out.push_str(line);
                out.push_str("\x1b[0m\n");
                continue;
            }
            None => {}
        }
        let t = line.trim_start();
        if t.starts_with("<!--") {
            continue;
        }
        if t.is_empty() {
            flush(&mut out, &mut para, &mut item, &mut tbl, width);
            if !out.ends_with("\n\n") && !out.is_empty() {
                out.push('\n');
            }
            continue;
        }
        if let Some(h) = line.strip_prefix("# ") {
            flush(&mut out, &mut para, &mut item, &mut tbl, width);
            let title = inline(h.trim(), "\x1b[1;7;36m");
            let pad = width.saturating_sub(visible(&title) + 2);
            out.push_str(&format!("\x1b[1;7;36m {title}{} \x1b[0m\n", " ".repeat(pad)));
            continue;
        }
        if let Some(h) = line.strip_prefix("## ").or_else(|| line.strip_prefix("### ")) {
            flush(&mut out, &mut para, &mut item, &mut tbl, width);
            let title = inline(h.trim(), "\x1b[1;36m");
            out.push_str(&format!("\x1b[1;36m{title}\x1b[0m\n\x1b[2;36m{}\x1b[0m\n", "─".repeat(visible(&title).min(width))));
            continue;
        }
        if t.starts_with('|') {
            if !tbl.is_empty() || para.is_empty() {
                flush(&mut out, &mut para, &mut item, &mut Vec::new(), width);
                let cells: Vec<String> = t.trim_matches('|').replace("\\|", "\u{2}").split('|').map(|c| c.replace('\u{2}', "|")).collect();
                if !cells.iter().all(|c| c.trim().chars().all(|ch| "-: ".contains(ch))) {
                    tbl.push(cells);
                }
                continue;
            }
        }
        if let Some(q) = t.strip_prefix("> ").or_else(|| (t == ">").then_some("")) {
            flush(&mut out, &mut para, &mut item, &mut tbl, width);
            out.push_str(&wrap(&inline(q, "\x1b[3m"), width, "\x1b[2m▌\x1b[0m \x1b[3m", "\x1b[2m▌\x1b[0m \x1b[3m"));
            out.push_str("\x1b[0m");
            continue;
        }
        if t == "---" || t == "***" {
            flush(&mut out, &mut para, &mut item, &mut tbl, width);
            out.push_str(&format!("\x1b[2m{}\x1b[0m\n", "─".repeat(width)));
            continue;
        }
        // A list item: "- ", "* ", "1. ".
        let lead = line.len() - t.len();
        let num = t.split_once(". ").filter(|(n, _)| !n.is_empty() && n.len() <= 3 && n.chars().all(|c| c.is_ascii_digit()));
        let marker = if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") { Some((t[..1].to_string(), &t[2..])) } else { num.map(|(n, rest)| (format!("{n}."), rest)) };
        if let Some((m, rest)) = marker {
            flush(&mut out, &mut para, &mut item, &mut tbl, width);
            item = Some((format!("{}{m}", " ".repeat(lead)), rest.to_string()));
            continue;
        }
        // (A list item's next line.)
        if let Some((_, text)) = item.as_mut() {
            text.push(' ');
            text.push_str(t);
            continue;
        }
        para.push(line.replace("\\|", "|"));
    }
    flush(&mut out, &mut para, &mut item, &mut tbl, width);
    out
}

/// `code` in yellow, **bold** in bold, *italic* and _italic_ in italics,
/// [links](url) underlined; `base` is the style to go back to.
fn inline(s: &str, base: &str) -> String {
    let mut out = String::new();
    let mut code = false;
    let mut bold = false;
    let mut ital = false;
    let mut rest = s;
    let mut prev = ' ';
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('`') {
            code = !code;
            out.push_str(if code { "\x1b[33m" } else { "\x1b[39m" });
            if !code {
                out.push_str(base);
            }
            rest = r;
            prev = '`';
            continue;
        }
        if !code {
            if let Some(r) = rest.strip_prefix("**") {
                bold = !bold;
                out.push_str(if bold { "\x1b[1m" } else { "\x1b[22m" });
                if !bold {
                    out.push_str(base);
                }
                rest = r;
                continue;
            }
            let c = rest.chars().next().unwrap();
            if (c == '*' || c == '_') && (ital || (!prev.is_alphanumeric() && rest[1..].chars().next().is_some_and(|n| !n.is_whitespace()))) {
                let closes = ital && !rest[1..].chars().next().is_some_and(|n| n.is_alphanumeric());
                if !ital || closes {
                    ital = !ital;
                    out.push_str(if ital { "\x1b[3m" } else { "\x1b[23m" });
                    rest = &rest[1..];
                    prev = c;
                    continue;
                }
            }
            if c == '[' {
                if let Some(close) = rest.find("](") {
                    if let Some(end) = rest[close..].find(')') {
                        let text = &rest[1..close];
                        out.push_str(&format!("\x1b[4m{}\x1b[24m", inline(text, base)));
                        rest = &rest[close + end + 1..];
                        prev = ')';
                        continue;
                    }
                }
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        prev = c;
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// The mark that stands for an example's tab bar in [`tab_examples`]'s
/// Markdown (an invisible separator, then its number).
pub const TAB_MARK: &str = "\u{2063}tabs ";

/// The Advanced guide's Markdown with each example's two versions (a bash
/// block, maybe its `output bash`, then the tessh block and its output) as
/// one tab bar, `TAB_MARK` and the example's number on a line of their
/// own, and the version `bash(n)` picks: the prose between them goes
/// before. The number of examples is the second value.
pub fn tab_examples(md: &str, bash: &dyn Fn(usize) -> bool) -> (String, usize) {
    let lines: Vec<&str> = md.lines().collect();
    // (The fenced blocks: info, first line, last line.)
    let mut blocks: Vec<(String, usize, usize)> = vec![];
    let mut i = 0;
    while i < lines.len() {
        if let Some(info) = lines[i].strip_prefix("```") {
            let start = i;
            i += 1;
            while i < lines.len() && !lines[i].starts_with("```") {
                i += 1;
            }
            blocks.push((info.trim().to_string(), start, i.min(lines.len() - 1)));
        }
        i += 1;
    }
    let lang = |k: usize| blocks[k].0.split_whitespace().next().unwrap_or("").to_string();
    let blank_between = |a: usize, b: usize| lines[a + 1..b].iter().all(|l| l.trim().is_empty());
    let mut out: Vec<String> = Vec::with_capacity(lines.len() + 64);
    let mut at = 0;
    let mut n = 0;
    let mut k = 0;
    while k < blocks.len() {
        if lang(k) != "bash" {
            k += 1;
            continue;
        }
        let mut j = k + 1;
        let bout = (j < blocks.len() && blocks[j].0 == "output bash" && blank_between(blocks[k].2, blocks[j].1)).then_some(j);
        if bout.is_some() {
            j += 1;
        }
        let be = bout.map(|b| blocks[b].2).unwrap_or(blocks[k].2);
        if j >= blocks.len() || lang(j) != "tessh" || lines[be + 1..blocks[j].1].iter().any(|l| l.starts_with('#')) {
            k += 1;
            continue;
        }
        let tout = (j + 1 < blocks.len() && blocks[j + 1].0 == "output" && blank_between(blocks[j].2, blocks[j + 1].1)).then_some(j + 1);
        let (bs, ts) = (blocks[k].1, blocks[j].1);
        let te = tout.map(|o| blocks[o].2).unwrap_or(blocks[j].2);
        out.extend(lines[at..bs].iter().map(|l| l.to_string()));
        out.extend(lines[be + 1..ts].iter().map(|l| l.to_string()));
        out.push(String::new());
        out.push(format!("{TAB_MARK}{n}"));
        out.push(String::new());
        let (a, b) = if bash(n) { (bs, be) } else { (ts, te) };
        out.extend(lines[a..=b].iter().map(|l| l.to_string()));
        out.push(String::new());
        at = te + 1;
        n += 1;
        k = tout.unwrap_or(j) + 1;
    }
    out.extend(lines[at..].iter().map(|l| l.to_string()));
    (out.join("\n"), n)
}

/// The whole Advanced guide, as the full-screen guide shows it.
pub fn advanced_guide() -> String {
    clean(&advanced())
}

/// The whole guide, for `tessh --guide` and `help guide`.
pub fn guide() -> String {
    clean(GUIDE)
}

/// A help page as a value: Markdown in a notebook, text elsewhere.
#[pyclass(module = "__tessh__", name = "HelpPage", frozen)]
pub struct HelpPage {
    md: String,
}

#[pymethods]
impl HelpPage {
    fn _repr_markdown_(&self) -> String {
        self.md.clone()
    }
    fn __repr__(&self) -> String {
        self.md.clone()
    }
}

/// help [topic | $value]: the guide, a topic of it, or Python's help.
#[pyfunction]
#[pyo3(signature = (*args))]
pub fn help(py: Python<'_>, args: Vec<Bound<'_, PyAny>>) -> PyResult<()> {
    let md = match args.first() {
        None => lookup(""),
        Some(a) if a.is_instance_of::<PyString>() => {
            let w: String = args.iter().map(|a| a.str().map(|s| s.to_string())).collect::<PyResult<Vec<_>>>()?.join(" ");
            match lookup(&w) {
                Some(md) => Some(md),
                // (Not in the guide: Python's own help, which knows keywords and modules.)
                None => {
                    py.import("builtins")?.getattr("help")?.call1((w,))?;
                    return Ok(());
                }
            }
        }
        Some(a) => {
            py.import("builtins")?.getattr("help")?.call1((a,))?;
            return Ok(());
        }
    };
    let md = md.unwrap_or_default();
    if let Some(d) = crate::runtime::display_hook(py) {
        d.call1(py, (Py::new(py, HelpPage { md })?,))?;
        return Ok(());
    }
    let tty = unsafe { libc::isatty(0) == 1 && libc::isatty(1) == 1 };
    // (The whole guide, or the Advanced one, at a terminal: full screen,
    // searchable, both guides there and each example's two versions tabs.)
    let word = args.first().and_then(|a| a.extract::<String>().ok()).unwrap_or_default().to_lowercase();
    if tty && (md.len() > 20000 || word == "advanced") {
        let which = if word == "advanced" { 1 } else { 0 };
        if md.len() > 20000 && word != "guide" && word != "all" && which == 0 {
            py.detach(|| crate::editor::pager::show(&md, ""));
        } else {
            py.detach(|| crate::editor::pager::show_guides("", which));
        }
        return Ok(());
    }
    let color = unsafe { libc::isatty(1) } == 1 && std::env::var_os("NO_COLOR").is_none();
    let text = terminal(&md, color);
    let print = py.import("builtins")?.getattr("print")?;
    print.call((text.trim_end(),), None)?;
    Ok(())
}

pub fn add(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<HelpPage>()?;
    m.add_function(wrap_pyfunction!(help, m)?)?;
    Ok(())
}
