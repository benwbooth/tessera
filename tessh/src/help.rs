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

/// Markdown for a terminal: headings bold, code highlighted and indented,
/// output dim. Plain (color false): as it is.
pub fn terminal(md: &str, color: bool) -> String {
    if !color {
        return md.to_string();
    }
    let mut out = String::new();
    let mut fence: Option<String> = None;
    for line in md.lines() {
        if let Some(info) = line.strip_prefix("```") {
            fence = if fence.is_some() { None } else { Some(info.trim().to_string()) };
            continue;
        }
        match &fence {
            Some(lang) if lang.starts_with("tessh") => {
                out.push_str("    ");
                out.push_str(&crate::repl::highlight(line));
            }
            Some(_) => {
                out.push_str("    \x1b[2m");
                out.push_str(line);
                out.push_str("\x1b[0m");
            }
            None => {
                if let Some(h) = line.strip_prefix("## ").or_else(|| line.strip_prefix("# ")) {
                    out.push_str("\x1b[1;36m");
                    out.push_str(&inline(h, "\x1b[1;36m"));
                    out.push_str("\x1b[0m");
                } else {
                    out.push_str(&inline(&line.replace("\\|", "|"), ""));
                }
            }
        }
        out.push('\n');
    }
    out
}

/// `code` in yellow, **bold** in bold.
fn inline(s: &str, base: &str) -> String {
    let mut out = String::new();
    let mut code = false;
    let mut bold = false;
    let mut rest = s;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix('`') {
            code = !code;
            out.push_str(if code { "\x1b[33m" } else { "\x1b[0m" });
            if !code {
                out.push_str(base);
            }
            rest = r;
        } else if let (false, Some(r)) = (code, rest.strip_prefix("**")) {
            bold = !bold;
            out.push_str(if bold { "\x1b[1m" } else { "\x1b[22m" });
            rest = r;
        } else {
            let c = rest.chars().next().unwrap();
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out
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
