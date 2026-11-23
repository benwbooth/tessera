//! Syntax highlighting and syntax errors for the input editor, with
//! tree-sitter: shell commands, plus Python and JavaScript for their REPLs.
//! Offsets are UTF-16 (what Qt's text document counts).

use serde_json::json;
use std::cell::RefCell;
use std::sync::OnceLock;
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

struct Lang {
    language: Language,
    query: Query,
}

/// The languages the editor knows: (name, label).
pub const LANGUAGES: &[(&str, &str)] = &[
    ("bash", "Shell (bash/zsh/sh)"),
    ("python", "Python"),
    ("javascript", "JavaScript"),
    ("typescript", "TypeScript"),
    ("rust", "Rust"),
    ("go", "Go"),
    ("c", "C"),
    ("cpp", "C++"),
    ("java", "Java"),
    ("ruby", "Ruby"),
    ("lua", "Lua"),
    ("sql", "SQL"),
    ("json", "JSON"),
    ("yaml", "YAML"),
    ("toml", "TOML"),
    ("html", "HTML"),
    ("css", "CSS"),
    ("markdown", "Markdown"),
    ("tessh", "tessh"),
];

fn make(name: &str) -> Option<Lang> {
    let (language, query): (Language, String) = match name {
        "bash" => (tree_sitter_bash::LANGUAGE.into(), tree_sitter_bash::HIGHLIGHT_QUERY.into()),
        "python" => (tree_sitter_python::LANGUAGE.into(), tree_sitter_python::HIGHLIGHTS_QUERY.into()),
        "javascript" => (tree_sitter_javascript::LANGUAGE.into(), tree_sitter_javascript::HIGHLIGHT_QUERY.into()),
        // (TypeScript's rules add to JavaScript's.)
        "typescript" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            format!("{}\n{}", tree_sitter_javascript::HIGHLIGHT_QUERY, tree_sitter_typescript::HIGHLIGHTS_QUERY),
        ),
        "rust" => (tree_sitter_rust::LANGUAGE.into(), tree_sitter_rust::HIGHLIGHTS_QUERY.into()),
        "go" => (tree_sitter_go::LANGUAGE.into(), tree_sitter_go::HIGHLIGHTS_QUERY.into()),
        "c" => (tree_sitter_c::LANGUAGE.into(), tree_sitter_c::HIGHLIGHT_QUERY.into()),
        "cpp" => (tree_sitter_cpp::LANGUAGE.into(), format!("{}\n{}", tree_sitter_c::HIGHLIGHT_QUERY, tree_sitter_cpp::HIGHLIGHT_QUERY)),
        "java" => (tree_sitter_java::LANGUAGE.into(), tree_sitter_java::HIGHLIGHTS_QUERY.into()),
        "ruby" => (tree_sitter_ruby::LANGUAGE.into(), tree_sitter_ruby::HIGHLIGHTS_QUERY.into()),
        "lua" => (tree_sitter_lua::LANGUAGE.into(), tree_sitter_lua::HIGHLIGHTS_QUERY.into()),
        "sql" => (tree_sitter_sequel::LANGUAGE.into(), tree_sitter_sequel::HIGHLIGHTS_QUERY.into()),
        "json" => (tree_sitter_json::LANGUAGE.into(), tree_sitter_json::HIGHLIGHTS_QUERY.into()),
        "yaml" => (tree_sitter_yaml::LANGUAGE.into(), tree_sitter_yaml::HIGHLIGHTS_QUERY.into()),
        "toml" => (tree_sitter_toml_ng::LANGUAGE.into(), tree_sitter_toml_ng::HIGHLIGHTS_QUERY.into()),
        "html" => (tree_sitter_html::LANGUAGE.into(), tree_sitter_html::HIGHLIGHTS_QUERY.into()),
        "css" => (tree_sitter_css::LANGUAGE.into(), tree_sitter_css::HIGHLIGHTS_QUERY.into()),
        "markdown" => (tree_sitter_md::LANGUAGE.into(), tree_sitter_md::HIGHLIGHT_QUERY_BLOCK.into()),
        "tessh" => (tree_sitter_tessh::LANGUAGE.into(), tree_sitter_tessh::HIGHLIGHTS_QUERY.into()),
        // Anything else: Helix's grammar for it, if there is one.
        other => (crate::grammars::load(other)?, crate::grammars::highlights(other)),
    };
    // (A rule this tree-sitter can't read is left out and the rest kept;
    // with none readable, unhighlighted but still checked.)
    let query = salvaged_query(&language, &query).or_else(|| Query::new(&language, "").ok())?;
    Some(Lang { language, query })
}

/// A highlights query, less the rules this tree-sitter can't read (Helix's
/// are written for its own version: Solidity's used a form this one
/// rejects, and the whole file failed). Each error's rule — the top-level
/// form it's in — is taken out and the query tried again.
fn salvaged_query(language: &Language, src: &str) -> Option<Query> {
    let mut src = src.to_string();
    for _ in 0..40 {
        match Query::new(language, &src) {
            Ok(q) => return Some(q),
            Err(e) => {
                let (start, end) = top_level_form(&src, e.offset)?;
                src.replace_range(start..end, "");
            }
        }
    }
    None
}

/// The byte range of the top-level form (a rule) that `at` is in: from its
/// opening bracket at depth 0 through its captures and predicates' end.
fn top_level_form(src: &str, at: usize) -> Option<(usize, usize)> {
    let b = src.as_bytes();
    let (mut depth, mut start, mut i, mut in_str) = (0i32, None::<usize>, 0usize, false);
    let mut forms: Vec<(usize, usize)> = vec![];
    while i < b.len() {
        let c = b[i];
        if in_str {
            if c == b'\\' {
                i += 1;
            } else if c == b'"' {
                in_str = false;
            }
        } else if c == b';' {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'(' || c == b'[' {
            if depth == 0 && start.is_none() {
                start = Some(i);
            }
            depth += 1;
        } else if c == b')' || c == b']' {
            depth -= 1;
            if depth == 0 {
                // (A rule runs on through its captures and quantifiers to the next form.)
                let mut j = i + 1;
                while j < b.len() && b[j] != b'(' && b[j] != b'[' && b[j] != b';' && b[j] != b'\n' {
                    j += 1;
                }
                if let Some(s) = start.take() {
                    forms.push((s, j));
                }
            }
        }
        i += 1;
    }
    // (Predicates on their own lines belong to the rule before them: the
    // form at `at`, else the last one starting before it.)
    forms.iter().copied().find(|&(s, e)| s <= at && at < e.max(s + 1)).or_else(|| forms.iter().copied().filter(|&(s, _)| s <= at).last()).or(forms.first().copied())
}

fn lang(name: &str) -> Option<&'static Lang> {
    static CACHE: OnceLock<std::sync::Mutex<std::collections::HashMap<String, Option<&'static Lang>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut c = cache.lock().unwrap();
    let name = if is_builtin(name) || crate::grammars::is_known(name) { name } else { "bash" };
    if let Some(l) = c.get(name) {
        return *l;
    }
    // (A language that isn't here yet isn't remembered as missing: it may
    // be unpacked in a moment.)
    let l = make(name).map(|l| &*Box::leak(Box::new(l)));
    if l.is_some() || is_builtin(name) {
        c.insert(name.to_string(), l);
    }
    l
}

/// The editor's language for the program at the prompt (a REPL), or "" when
/// it's a shell.
pub fn language_for(program: &str) -> &'static str {
    let p = program.rsplit(['/', '\\']).next().unwrap_or(program).trim_start_matches('-').to_lowercase();
    let p = p.trim_end_matches(".exe");
    if p.starts_with("python") || p == "ipython" || p == "bpython" || p == "ptpython" {
        "python"
    } else if matches!(p, "node" | "deno" | "bun") {
        "javascript"
    } else if matches!(p, "ts-node" | "tsx") {
        "typescript"
    } else if matches!(p, "irb" | "pry") {
        "ruby"
    } else if p.starts_with("lua") {
        "lua"
    } else if matches!(p, "psql" | "mysql" | "mariadb" | "sqlite3" | "duckdb" | "pgcli" | "mycli" | "litecli" | "clickhouse-client") {
        "sql"
    } else if matches!(p, "evcxr" | "irust") {
        "rust"
    } else if matches!(p, "gore" | "yaegi") {
        "go"
    } else if p == "tessh" {
        "tessh"
    } else if p == "cling" {
        "cpp"
    } else if matches!(p, "jshell") {
        "java"
    } else {
        "bash"
    }
}

/// What a text is written in, when it says so clearly (a shebang, JSON, or
/// several lines with unmistakable marks); "" otherwise (it's a command).
pub fn detect_language(text: &str) -> &'static str {
    let t = text.trim_start();
    if let Some(first) = t.lines().next().filter(|l| l.starts_with("#!")) {
        for (k, v) in [("python", "python"), ("node", "javascript"), ("ruby", "ruby"), ("lua", "lua"), ("bash", "bash"), ("zsh", "bash"), ("sh", "bash")] {
            if first.contains(k) {
                return v;
            }
        }
    }
    let lines: Vec<&str> = t.lines().filter(|l| !l.trim().is_empty()).collect();
    if (t.starts_with('{') || t.starts_with('[')) && serde_json::from_str::<serde_json::Value>(t.trim()).is_ok() {
        return "json";
    }
    if lines.len() < 2 {
        // (One line: only the plainest marks.)
        let l = t.trim();
        for (marks, lang) in [
            (&["print(", "import ", "from ", "def ", "lambda "][..], "python"),
            (&["console.log(", "const ", "let ", "function "][..], "javascript"),
            (&["println!(", "let mut ", "fn main"][..], "rust"),
            (&["fmt.Print"][..], "go"),
            (&["puts ", "p "][..], "ruby"),
            (&["System.out.print"][..], "java"),
            (&["printf(\""][..], "c"),
            (&["std::cout", "#include", "using namespace std", "import std;", "std::print", "std::vector", "std::string", "auto main("][..], "cpp"),
            (&["SELECT ", "select "][..], "sql"),
        ] {
            if marks.iter().any(|m| l.starts_with(m)) {
                return lang;
            }
        }
        // (std:: with a call and no macro bang: C++; Rust's would be println!.)
        if l.contains("std::") && l.contains('(') && !l.contains("!(") && !l.starts_with("use ") && !l.starts_with("let ") {
            return "cpp";
        }
                // (name = value, with spaces: never a shell assignment; most often Python.)
        if let Some((name, rest)) = l.split_once(" = ") {
            if !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') && !name.starts_with(|c: char| c.is_ascii_digit()) && !rest.trim().is_empty() {
                return "python";
            }
        }
        return "";
    }
    let has = |pats: &[&str]| lines.iter().filter(|l| pats.iter().any(|p| l.trim_start().starts_with(p))).count();
    let score = [
        ("python", has(&["def ", "import ", "from ", "class ", "elif ", "print("]) * 2 + lines.iter().filter(|l| l.trim_end().ends_with(':')).count()),
        ("rust", has(&["fn ", "let mut ", "use ", "impl ", "pub fn ", "struct ", "#[derive"]) * 2),
        ("go", has(&["func ", "package ", "import (", ":= "]) * 2 + lines.iter().filter(|l| l.contains(" := ")).count()),
        ("javascript", has(&["const ", "function ", "console.log", "export ", "import {"]) * 2 + lines.iter().filter(|l| l.contains("=> ")).count()),
        ("sql", lines.iter().filter(|l| { let u = l.trim_start().to_uppercase(); ["SELECT ", "INSERT ", "UPDATE ", "DELETE ", "CREATE ", "FROM ", "WHERE "].iter().any(|k| u.starts_with(k)) }).count() * 2),
        ("c", has(&["#include", "int main", "printf("]) * 2),
        ("html", has(&["<!doctype", "<html", "<div", "<p>", "<body"]) * 2),
        ("yaml", lines.iter().filter(|l| { let x = l.trim_start(); (x.starts_with("- ") || x.contains(": ")) && !x.contains('=') && !x.contains('|') }).count()),
    ];
    let best = score.iter().max_by_key(|(_, s)| *s).unwrap();
    // Clear marks on at least half the lines.
    if best.1 >= 3 && best.1 * 2 >= lines.len() { best.0 } else { "" }
}

pub fn is_builtin(name: &str) -> bool {
    LANGUAGES.iter().any(|(n, _)| *n == name)
}

/// The languages for the editor's menu (JSON [[name, label]]).
pub fn languages_json() -> String {
    let mut out: Vec<[String; 2]> = LANGUAGES.iter().map(|(n, l)| [n.to_string(), l.to_string()]).collect();
    for n in crate::grammars::known() {
        if !is_builtin(&n) && n != "tsx" {
            let mut label: String = n.replace(['-', '_'], " ");
            if let Some(f) = label.get_mut(0..1) {
                f.make_ascii_uppercase();
            }
            out.push([n.clone(), label]);
        }
    }
    serde_json::to_string(&out).unwrap_or_default()
}

/// Code as HTML, each highlighted piece in `<span class="t-KIND">` (the
/// capture's first part: keyword, string, …), escaped.
pub fn highlight_html(lang_name: &str, text: &str) -> String {
    let esc = |t: &str| t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let Some(l) = lang(lang_name) else { return esc(text) };
    let mut p = Parser::new();
    if p.set_language(&l.language).is_err() {
        return esc(text);
    }
    // (A time limit too: a guide draws hundreds of blocks. Past it, the
    // block is plain.)
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(300);
    let Some(tree) = parse(&mut p, text, Some(deadline)) else { return esc(text) };
    // (The innermost capture wins: kinds by byte, later captures over earlier.)
    let mut kind: Vec<Option<&str>> = vec![None; text.len()];
    let Some(spans) = capture_spans_within(lang_name, l, tree.root_node(), text, Some(deadline)) else { return esc(text) };
    for (s, e, k) in spans {
        for b in s..e {
            kind[b] = Some(k);
        }
    }
    let mut out = String::new();
    let mut i = 0;
    while i < text.len() {
        let k = kind[i];
        let mut j = i + 1;
        while j < text.len() && (kind[j] == k || !text.is_char_boundary(j)) {
            j += 1;
        }
        match k {
            Some(k) => out.push_str(&format!("<span class=\"t-{k}\">{}</span>", esc(&text[i..j]))),
            None => out.push_str(&esc(&text[i..j])),
        }
        i = j;
    }
    out
}

/// A capture's color kind: its first part ("keyword.control" is a
/// keyword), but prose markup (Markdown's headings, code, links), which
/// has no color of its own, as the code kind it reads like.
fn kind_of(name: &str) -> &str {
    match name {
        "text.title" | "markup.heading" => "keyword",
        "text.literal" | "markup.raw" | "markup.raw.block" => "string",
        "text.uri" | "text.reference" | "markup.link" | "markup.link.url" | "markup.link.label" => "function",
        "text.emphasis" | "text.strong" | "markup.italic" | "markup.bold" => "attribute",
        "markup.list" | "markup.quote" => "punctuation",
        _ => name.split('.').next().unwrap_or(""),
    }
}

/// {"spans": [[start, length, kind]], "errors": [[start, length, message]]}
pub fn analyze(lang_name: &str, text: &str) -> String {
    if lang_name == "markdown" {
        return markdown_analysis(text);
    }
    let Some(l) = lang(lang_name) else { return json!({"spans": [], "errors": []}).to_string() };
    thread_local! { static PARSER: RefCell<Parser> = RefCell::new(Parser::new()); }
    let Some(tree) = PARSER.with(|p| {
        let mut p = p.borrow_mut();
        p.set_language(&l.language).ok()?;
        parse(&mut p, text, Some(std::time::Instant::now() + std::time::Duration::from_secs(2)))
    }) else {
        return json!({"spans": [], "errors": []}).to_string();
    };
    // Byte offset -> UTF-16 offset.
    // (One past the end too: the parse's newline.)
    let mut u16_at = vec![0u32; text.len() + 2];
    let mut n = 0u32;
    for (i, c) in text.char_indices() {
        for b in 0..c.len_utf8() {
            u16_at[i + b] = n;
        }
        n += c.len_utf16() as u32;
    }
    u16_at[text.len()] = n;
    u16_at[text.len() + 1] = n;
    let range = |node: Node| {
        let (s, e) = (u16_at[node.start_byte()], u16_at[node.end_byte()]);
        (s, e.saturating_sub(s))
    };

    let mut spans = Vec::new();
    // (QML: the JavaScript in it, which Helix's rules leave to an injection.)
    if lang_name == "qmljs" {
        js_by_kind(tree.root_node(), &mut |node, kind| {
            let (s, len) = range(node);
            if len > 0 {
                spans.push(json!([s, len, kind]));
            }
        });
    }
    for (s, e, kind) in capture_spans(lang_name, l, tree.root_node(), text) {
        let (s, e) = (u16_at[s], u16_at[e]);
        if e > s {
            spans.push(json!([s, e - s, kind]));
        }
    }

    let mut errors = Vec::new();
    collect_errors(tree.root_node(), &mut |node| {
        let (s, len) = range(node);
        let msg = if node.is_missing() {
            format!("missing {}", node.kind())
        } else {
            "syntax error".to_string()
        };
        // (Something missing at the very end marks the last character.)
        let s = s.min(n.saturating_sub(1));
        errors.push(json!([s, len.max(1), msg]));
    });
    json!({"spans": spans, "errors": errors}).to_string()
}

/// `text` parsed as if it ended with a newline (offsets unchanged; the
/// tree may reach one byte past the end): Helix's Perl grammar never
/// finished on a block of number literals that didn't end with one. Gives
/// up (None) once `deadline` passes.
fn parse(p: &mut Parser, text: &str, deadline: Option<std::time::Instant>) -> Option<tree_sitter::Tree> {
    let bytes = text.as_bytes();
    let pad: &[u8] = if text.ends_with('\n') { b"" } else { b"\n" };
    let mut read = |i: usize, _: tree_sitter::Point| -> &[u8] {
        if i < bytes.len() {
            &bytes[i..]
        } else if i == bytes.len() {
            pad
        } else {
            b""
        }
    };
    let mut over = |_: &tree_sitter::ParseState| if deadline.is_some_and(|d| std::time::Instant::now() > d) { std::ops::ControlFlow::Break(()) } else { std::ops::ControlFlow::Continue(()) };
    p.parse_with_options(&mut read, None, Some(tree_sitter::ParseOptions::new().progress_callback(&mut over)))
}

/// A language's highlight spans as byte ranges: (start, end, kind). None
/// when there's no grammar for it here.
fn byte_spans(lang_name: &str, text: &str) -> Option<Vec<(usize, usize, &'static str)>> {
    let l = lang(lang_name)?;
    let mut p = Parser::new();
    p.set_language(&l.language).ok()?;
    let tree = parse(&mut p, text, Some(std::time::Instant::now() + std::time::Duration::from_secs(2)))?;
    // (Kinds as 'static: the highlighter's known names.)
    Some(capture_spans(lang_name, l, tree.root_node(), text)
        .into_iter()
        .filter_map(|(s, e, k)| KINDS.iter().find(|x| **x == k).map(|k| (s, e, *k)))
        .collect())
}

/// A tree's highlight captures as byte ranges (start, end, kind), later
/// over earlier; tessh's "…" strings split so what they interpolate reads
/// as code.
fn capture_spans(lang_name: &str, l: &'static Lang, root: Node, text: &str) -> Vec<(usize, usize, &'static str)> {
    capture_spans_within(lang_name, l, root, text, None).unwrap_or_default()
}

/// The captures, or None when `deadline` passed first.
fn capture_spans_within(lang_name: &str, l: &'static Lang, root: Node, text: &str, deadline: Option<std::time::Instant>) -> Option<Vec<(usize, usize, &'static str)>> {
    let names = l.query.capture_names();
    let mut out = Vec::new();
    let mut cursor = QueryCursor::new();
    let mut late = false;
    let mut check = |_: &tree_sitter::QueryCursorState| {
        if deadline.is_some_and(|d| std::time::Instant::now() > d) {
            late = true;
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    };
    // (The tree may reach one byte past the end, the newline the parse
    // added: the query reads the text with it, or a rule that looks at a
    // node's text reads past the end and panics.)
    let padded;
    let source: &[u8] = if root.end_byte() > text.len() {
        padded = format!("{text}\n");
        padded.as_bytes()
    } else {
        text.as_bytes()
    };
    let mut caps = cursor.captures_with_options(&l.query, root, source, tree_sitter::QueryCursorOptions::new().progress_callback(&mut check));
    while let Some((m, i)) = caps.next() {
        let c = m.captures()[*i];
        let (s, e) = (c.node.start_byte(), c.node.end_byte().min(text.len()));
        if e > s {
            out.push((s, e, kind_of(names[c.index as usize])));
        }
    }
    drop(caps);
    if late {
        return None;
    }
    // (tessh's strings split as tree-sitter-tessh does it, for Grove too.)
    if lang_name == "tessh" {
        out = tree_sitter_tessh::highlight::split_strings(text, out);
    }
    Some(out)
}

const KINDS: &[&str] = &["keyword", "string", "comment", "number", "constant", "function", "operator", "property", "variable",
                         "embedded", "type", "constructor", "punctuation", "escape", "attribute", "tag", "label", "namespace", "special"];

/// The language a fenced code block's info string names ("sh", "js",
/// "rust,ignore", ...): one there's a grammar for, or None. One Helix has
/// but isn't here yet is unpacked in the background, for next time.
fn fence_language(info: &str) -> Option<String> {
    let w = info.split(|c: char| c.is_whitespace() || c == ',' || c == '{').next().unwrap_or("").trim_start_matches('.').to_lowercase();
    let name = match w.as_str() {
        "" => return None,
        "sh" | "shell" | "zsh" | "console" | "shellsession" | "bash" => "bash",
        "js" | "mjs" | "cjs" | "jsx" | "javascript" => "javascript",
        "ts" | "typescript" => "typescript",
        "py" | "python3" | "python" => "python",
        "rs" | "rust" => "rust",
        "yml" | "yaml" => "yaml",
        "c++" | "cc" | "cxx" | "hpp" | "cpp" => "cpp",
        "h" => "c",
        "rb" => "ruby",
        "golang" => "go",
        "qml" => "qmljs",
        "htm" => "html",
        "jsonc" | "json5" => "json",
        "md" | "markdown" => return None,
        "ps1" | "pwsh" => "powershell",
        "kt" => "kotlin",
        "cs" | "csharp" => "c-sharp",
        "hs" => "haskell",
        "ex" | "exs" => "elixir",
        "dockerfile" | "docker" => "dockerfile",
        "make" | "makefile" => "make",
        "tex" | "latex" => "latex",
        other => other,
    };
    if is_builtin(name) || crate::grammars::available().iter().any(|n| n == name) {
        return Some(name.to_string());
    }
    if crate::grammars::is_known(name) {
        static ASKED: OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> = OnceLock::new();
        if ASKED.get_or_init(Default::default).lock().unwrap().insert(name.to_string()) {
            let n = name.to_string();
            std::thread::spawn(move || {
                let _ = crate::grammars::ensure(&n);
            });
        }
    }
    None
}

/// Markdown as text, highlighted for reading: headings bold and colored by
/// level (their # marks dim), **bold**, *italic* and ~~struck~~ drawn so,
/// `code` and links colored (links underlined, their addresses dim), list,
/// quote and table marks colored, HTML as HTML, and fenced code in its own
/// language. Style-only kinds ("=bold", "=italic", "=strike",
/// "=underline") go over the colors (include/syntax_highlighter.h).
fn markdown_analysis(text: &str) -> String {
    use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser as Md, Tag, TagEnd};
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_FOOTNOTES | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    let b = text.as_bytes();
    let mut spans: Vec<(usize, usize, &str)> = Vec::new();
    let mut styles: Vec<(usize, usize, &str)> = Vec::new();
    // (Embedded code: a language's spans over its range.)
    let embed = |spans: &mut Vec<(usize, usize, &str)>, lang: &str, s: usize, e: usize| {
        if let Some(inner) = text.get(s..e).and_then(|t| byte_spans(lang, t)) {
            spans.extend(inner.into_iter().map(|(a, z, k)| (s + a, s + z, k)));
        }
    };
    let mut code: Option<(usize, usize, Option<String>, usize, usize)> = None; // (block start, block end, lang, first text, last text)
    let mut meta: Option<(usize, usize)> = None;
    for (ev, r) in Md::new_ext(text, opts).into_offset_iter() {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                let kind = match level { HeadingLevel::H1 => "keyword", HeadingLevel::H2 => "function", HeadingLevel::H3 => "property", _ => "type" };
                spans.push((r.start, r.end, kind));
                styles.push((r.start, r.end, "=bold"));
                // (ATX: the leading #s and any closing ones are dim; setext: the underline.)
                let t = &text[r.clone()];
                let hashes = t.len() - t.trim_start_matches('#').len();
                if hashes > 0 {
                    spans.push((r.start, r.start + hashes, "punctuation"));
                    let tail = t.trim_end();
                    let close = tail.len() - tail.trim_end_matches('#').len();
                    if close > 0 && close < tail.len() - hashes {
                        spans.push((r.start + tail.len() - close, r.start + tail.len(), "punctuation"));
                    }
                } else if let Some(nl) = t.trim_end().rfind('\n') {
                    spans.push((r.start + nl + 1, r.end, "punctuation"));
                }
            }
            Event::Start(Tag::Strong) => {
                styles.push((r.start, r.end, "=bold"));
                spans.push((r.start, (r.start + 2).min(r.end), "punctuation"));
                spans.push((r.end.saturating_sub(2).max(r.start), r.end, "punctuation"));
            }
            Event::Start(Tag::Emphasis) => {
                styles.push((r.start, r.end, "=italic"));
                spans.push((r.start, (r.start + 1).min(r.end), "punctuation"));
                spans.push((r.end.saturating_sub(1).max(r.start), r.end, "punctuation"));
            }
            Event::Start(Tag::Strikethrough) => {
                styles.push((r.start, r.end, "=strike"));
                spans.push((r.start, r.end, "comment"));
            }
            Event::Start(Tag::BlockQuote(_)) => {
                spans.push((r.start, r.end, "comment"));
                styles.push((r.start, r.end, "=italic"));
                // (Each line's ">" marks.)
                let mut at = r.start;
                for line in text[r.clone()].split_inclusive('\n') {
                    let lead = line.len() - line.trim_start_matches([' ', '>']).len();
                    if line.trim_start().starts_with('>') {
                        spans.push((at, at + lead, "operator"));
                    }
                    at += line.len();
                }
            }
            Event::Start(Tag::Item) => {
                // (The bullet or number at its start.)
                let t = &text[r.clone()];
                let m = t.find(|c: char| c.is_whitespace()).unwrap_or(0);
                if m > 0 {
                    spans.push((r.start, r.start + m, "operator"));
                }
            }
            Event::TaskListMarker(_) => spans.push((r.start, r.end, "constant")),
            Event::Start(Tag::Link { .. }) | Event::Start(Tag::Image { .. }) => {
                let image = matches!(ev, Event::Start(Tag::Image { .. }));
                let t = &text[r.clone()];
                if t.starts_with('<') {
                    // (An autolink.)
                    spans.push((r.start, r.end, "function"));
                    styles.push((r.start + 1, r.end.saturating_sub(1), "=underline"));
                } else {
                    // [text](url) / ![alt](src) / [text][ref]: the text colored and
                    // underlined, the brackets and the address dim.
                    let lb = if image { 1 } else { 0 };
                    let close = t.rfind("](").or_else(|| t.rfind("][")).unwrap_or(t.len().saturating_sub(1));
                    spans.push((r.start, r.end, "comment"));
                    spans.push((r.start + lb + 1, r.start + close, if image { "attribute" } else { "function" }));
                    styles.push((r.start + lb + 1, r.start + close, "=underline"));
                    if image {
                        spans.push((r.start, r.start + 1, "operator"));
                    }
                }
            }
            Event::Code(_) => {
                spans.push((r.start, r.end, "string"));
                let t = &text[r.clone()];
                let ticks = t.len() - t.trim_start_matches('`').len();
                spans.push((r.start, r.start + ticks, "punctuation"));
                spans.push((r.end - ticks.min(r.end - r.start), r.end, "punctuation"));
            }
            Event::Html(_) | Event::InlineHtml(_) => {
                // (Its text as text: the grammar colors the tags and attributes.)
                spans.push((r.start, r.end, "variable"));
                embed(&mut spans, "html", r.start, r.end);
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match &kind { CodeBlockKind::Fenced(info) => fence_language(info), CodeBlockKind::Indented => None };
                spans.push((r.start, r.end, "string"));
                if let CodeBlockKind::Fenced(info) = &kind {
                    // (The fences dim, the language name as a type.)
                    let first = text[r.clone()].find('\n').map(|i| r.start + i).unwrap_or(r.end);
                    spans.push((r.start, first, "punctuation"));
                    if let Some(i) = text[r.start..first].find(info.as_ref()).filter(|_| !info.is_empty()) {
                        spans.push((r.start + i, r.start + i + info.len(), "type"));
                    }
                    let body = text[r.clone()].trim_end_matches(['\n', '\r', ' ']);
                    if let Some(nl) = body.rfind('\n') {
                        let last = &body[nl + 1..];
                        if last.trim_start().starts_with("```") || last.trim_start().starts_with("~~~") {
                            spans.push((r.start + nl + 1, r.start + body.len(), "punctuation"));
                        }
                    }
                }
                code = Some((r.start, r.end, lang, usize::MAX, 0));
            }
            Event::Text(_) if code.is_some() => {
                if let Some(c) = code.as_mut() {
                    c.3 = c.3.min(r.start);
                    c.4 = c.4.max(r.end);
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((_, _, Some(lang), s, e)) = code.take() {
                    if s < e {
                        // (Its code back to plain first: unmatched parts read as text.)
                        spans.push((s, e, "variable"));
                        embed(&mut spans, &lang, s, e);
                    }
                }
            }
            Event::Start(Tag::MetadataBlock(_)) => meta = Some((r.start, r.end)),
            Event::End(TagEnd::MetadataBlock(_)) => {
                if let Some((s, e)) = meta.take() {
                    spans.push((s, e, "punctuation"));
                    let body_s = text[s..e].find('\n').map(|i| s + i + 1).unwrap_or(e);
                    let body_e = text[s..e].trim_end().rfind('\n').map(|i| s + i).unwrap_or(e);
                    if body_s < body_e {
                        spans.push((body_s, body_e, "variable"));
                        embed(&mut spans, "yaml", body_s, body_e);
                    }
                }
            }
            Event::Start(Tag::Table(_)) => {
                // (Every pipe, and the row of dashes under the head.)
                for (i, c) in text[r.clone()].char_indices() {
                    if c == '|' && (i == 0 || b[r.start + i - 1] != b'\\') {
                        spans.push((r.start + i, r.start + i + 1, "punctuation"));
                    }
                }
                let t = &text[r.clone()];
                if let Some(nl) = t.find('\n') {
                    let end = t[nl + 1..].find('\n').map(|j| nl + 1 + j).unwrap_or(t.len());
                    spans.push((r.start + nl + 1, r.start + end, "punctuation"));
                }
            }
            Event::Start(Tag::TableHead) => styles.push((r.start, r.end, "=bold")),
            Event::Rule => spans.push((r.start, r.end, "punctuation")),
            Event::FootnoteReference(_) => spans.push((r.start, r.end, "constant")),
            Event::Start(Tag::FootnoteDefinition(_)) => {
                let t = &text[r.clone()];
                if let Some(i) = t.find("]:") {
                    spans.push((r.start, r.start + i + 2, "constant"));
                }
            }
            Event::Text(_) => {
                // (Backslash escapes: \* \_ \# ...)
                let t = &b[r.clone()];
                // (An escape's text can start just after its backslash.)
                if r.start > 0 && b[r.start - 1] == b'\\' && t.first().is_some_and(|c| c.is_ascii_punctuation()) {
                    spans.push((r.start - 1, r.start + 1, "escape"));
                }
                let mut i = 0;
                while i + 1 < t.len() {
                    if t[i] == b'\\' && t[i + 1].is_ascii_punctuation() {
                        spans.push((r.start + i, r.start + i + 2, "escape"));
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            _ => {}
        }
    }
    // Byte offsets -> UTF-16 offsets.
    let mut u16_at = vec![0u32; text.len() + 1];
    let mut n = 0u32;
    for (i, c) in text.char_indices() {
        for k in 0..c.len_utf8() {
            u16_at[i + k] = n;
        }
        n += c.len_utf16() as u32;
    }
    u16_at[text.len()] = n;
    let to = |(s, e, k): (usize, usize, &str)| {
        let (s, e) = (s.min(text.len()), e.min(text.len()));
        (e > s).then(|| json!([u16_at[s], u16_at[e] - u16_at[s], k]))
    };
    let out: Vec<serde_json::Value> = spans.into_iter().chain(styles).filter_map(to).collect();
    json!({"spans": out, "errors": []}).to_string()
}

/// JavaScript's pieces by their node kinds (strings, numbers, constants,
/// calls, keywords), for a grammar built on JavaScript's whose own rules
/// don't color them (QML's).
fn js_by_kind<'a>(node: Node<'a>, f: &mut dyn FnMut(Node<'a>, &'static str)) {
    const KEYWORDS: &[&str] = &["if", "else", "return", "const", "let", "var", "function", "for", "while", "do", "break", "continue", "new",
                                "typeof", "instanceof", "in", "of", "try", "catch", "finally", "throw", "switch", "case", "default", "delete", "void", "this"];
    let kind = node.kind();
    let k: Option<&'static str> = match kind {
        "string" | "template_string" | "regex" => Some("string"),
        "number" => Some("number"),
        "true" | "false" | "null" | "undefined" => Some("constant"),
        "comment" => Some("comment"),
        _ if !node.is_named() && KEYWORDS.contains(&kind) => Some("keyword"),
        _ => None,
    };
    if let Some(k) = k {
        f(node, k);
        if k != "string" {
            return;
        }
    }
    if kind == "call_expression" {
        if let Some(func) = node.child_by_field_name("function") {
            let name = if func.kind() == "member_expression" { func.child_by_field_name("property") } else { Some(func) };
            if let Some(n) = name.filter(|n| matches!(n.kind(), "identifier" | "property_identifier")) {
                f(n, "function");
            }
        }
    }
    let mut c = node.walk();
    for child in node.children(&mut c) {
        js_by_kind(child, f);
    }
}

fn collect_errors(node: Node, f: &mut dyn FnMut(Node)) {
    if node.is_error() || node.is_missing() {
        f(node);
        return;
    }
    if !node.has_error() {
        return;
    }
    let mut c = node.walk();
    for child in node.children(&mut c) {
        collect_errors(child, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_reads_well() {
        let out = super::analyze("markdown", "## Two\n\n**b** *i* ~~s~~ `c` [l](u) \\*\n\n```rust\nfn f() {}\n```\n");
        for want in ["\"function\"", "\"=bold\"", "\"=italic\"", "\"=strike\"", "\"=underline\"", "\"string\"", "\"escape\"", "\"keyword\"", "\"punctuation\""] {
            assert!(out.contains(want), "{want} in {out}");
        }
    }

    #[test]
    fn tessh_interpolation_is_code() {
        let h = super::highlight_html("tessh", r#"print "a $name b $(len $xs) ${n:.2f} ${f}.txt $self.x \n" 'raw $n'"#);
        for want in [
            r#"<span class="t-string">"a </span><span class="t-variable">$name</span>"#,
            r#"<span class="t-embedded">$(</span><span class="t-function">len</span> <span class="t-variable">$xs</span><span class="t-embedded">)</span>"#,
            r#"<span class="t-embedded">${</span><span class="t-variable">n</span><span class="t-attribute">:.2f</span><span class="t-embedded">}</span>"#,
            r#"<span class="t-embedded">}</span><span class="t-string">.txt </span><span class="t-variable">$self.x</span>"#,
            r#"<span class="t-escape">\n</span><span class="t-string">"</span>"#,
            r#"<span class="t-string">'raw $n'</span>"#,
        ] {
            assert!(h.contains(want), "{want}\n in\n{h}");
        }
        // (A string inside an interpolation doesn't end the outer one.)
        let h = super::highlight_html("tessh", r#"print "n: ${len "a b"} done""#);
        let want = r#"<span class="t-embedded">${</span><span class="t-function">len</span> <span class="t-string">"a b"</span><span class="t-embedded">}</span><span class="t-string"> done"</span>"#;
        assert!(h.contains(want), "{want}\n in\n{h}");
    }

    #[test]
    fn markdown_has_colors() {
        let out = super::analyze("markdown", "# Title\n\n```\ncode\n```\n");
        assert!(out.contains("\"keyword\""), "{out}");
        assert!(out.contains("\"string\""), "{out}");
        assert!(!out.contains("\"text\""), "{out}");
    }

    #[test]
    fn bash_spans_and_errors() {
        let v: serde_json::Value = serde_json::from_str(&analyze("bash", "echo \"hi\" | grep h # note")).unwrap();
        let kinds: Vec<&str> = v["spans"].as_array().unwrap().iter().map(|s| s[2].as_str().unwrap()).collect();
        assert!(kinds.contains(&"string"), "{kinds:?}");
        assert!(kinds.contains(&"comment"), "{kinds:?}");
        assert!(v["errors"].as_array().unwrap().is_empty());
        let bad: serde_json::Value = serde_json::from_str(&analyze("bash", "if [ x ]; then echo")).unwrap();
        assert!(!bad["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn utf16_offsets() {
        let v: serde_json::Value = serde_json::from_str(&analyze("bash", "echo 終端 \"x\"")).unwrap();
        let s = v["spans"].as_array().unwrap().iter().find(|s| s[2] == "string").unwrap();
        assert_eq!(s[0], 8);
    }

    #[test]
    fn languages() {
        assert_eq!(language_for("/usr/bin/python3.12"), "python");
        assert_eq!(language_for("-zsh"), "bash");
        assert_eq!(language_for("node"), "javascript");
        assert_eq!(language_for("psql"), "sql");
        assert_eq!(detect_language("#!/usr/bin/env python3\nprint(1)"), "python");
        assert_eq!(detect_language("def f(x):\n    return x\nimport os\n"), "python");
        assert_eq!(detect_language("git status && ls -la"), "");
        assert_eq!(detect_language("{\"a\": 1}"), "json");
        for (n, _) in LANGUAGES {
            assert!(lang(n).is_some(), "{n} loads");
        }
    }
}

/// Does `text` fail to parse as `lang_name`?
pub fn has_errors(lang_name: &str, text: &str) -> bool {
    let Some(l) = lang(lang_name) else { return false };
    let mut p = Parser::new();
    if p.set_language(&l.language).is_err() {
        return false;
    }
    parse(&mut p, text, Some(std::time::Instant::now() + std::time::Duration::from_secs(2))).is_some_and(|t| t.root_node().has_error())
}

/// The shell's own syntax check: `zsh -n` (bash, fish) reads the line
/// without running it. Its complaints, on the lines they name: JSON
/// [[start, length, message]] in UTF-16 units; [] when it parses (or the
/// shell isn't there).
pub fn shell_check(shell: &str, text: &str) -> String {
    if shell.rsplit('/').next() == Some("tessh") {
        return tessh_check(text);
    }
    let sh = match shell.rsplit('/').next().unwrap_or(shell) {
        s @ ("zsh" | "bash" | "fish" | "sh" | "dash" | "ksh") => s.to_string(),
        _ => "bash".to_string(),
    };
    let out = std::process::Command::new(&sh)
        .arg("-n")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            if let Some(mut i) = c.stdin.take() {
                let _ = i.write_all(text.as_bytes());
                let _ = i.write_all(b"\n");
            }
            c.wait_with_output()
        });
    let Ok(out) = out else { return "[]".into() };
    if out.status.success() {
        return "[]".into();
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let lines: Vec<&str> = text.split('\n').collect();
    let starts: Vec<usize> = lines.iter().scan(0usize, |acc, l| {
        let s = *acc;
        *acc += l.encode_utf16().count() + 1;
        Some(s)
    }).collect();
    let mut found = Vec::new();
    for e in err.lines().filter(|l| !l.trim().is_empty()).take(3) {
        // "zsh: parse error near `)'", "bash: line 1: syntax error ...", "fish: ...: Missing end ..."
        let line_no = e
            .split(|c: char| c == ':' || c == ' ')
            .filter_map(|w| w.trim().parse::<usize>().ok())
            .next()
            .unwrap_or(lines.len())
            .clamp(1, lines.len().max(1));
        let i = line_no - 1;
        let msg = e.trim_start_matches(&format!("{sh}:")).trim().trim_start_matches(|c: char| c.is_ascii_digit() || c == ':').trim();
        let msg = msg.strip_prefix("line ").map(|r| r.trim_start_matches(|c: char| c.is_ascii_digit() || c == ':').trim()).unwrap_or(msg);
        found.push(serde_json::json!([starts.get(i).copied().unwrap_or(0), lines.get(i).map(|l| l.encode_utf16().count()).unwrap_or(0).max(1), msg]));
    }
    serde_json::Value::Array(found).to_string()
}

/// tessh's own check (`tessh --check --json`): the reader, the compiler and
/// macros, so it finds what the flat tree-sitter grammar can't (indentation,
/// a misplaced else, a bad parameter). [[start, length, message]] in UTF-16.
pub fn tessh_check(text: &str) -> String {
    let Some(prog) = tessh_program() else { return "[]".into() };
    let out = std::process::Command::new(prog)
        .args(["--check", "--json"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .and_then(|mut c| {
            use std::io::Write;
            if let Some(mut i) = c.stdin.take() {
                let _ = i.write_all(text.as_bytes());
            }
            c.wait_with_output()
        });
    let Ok(out) = out else { return "[]".into() };
    let Ok(v) = serde_json::from_slice::<Vec<serde_json::Value>>(&out.stdout) else { return "[]".into() };
    let lines: Vec<&str> = text.split('\n').collect();
    let mut found = Vec::new();
    for e in v {
        let line = e["line"].as_u64().unwrap_or(1).max(1) as usize;
        let (col, end) = (e["col"].as_u64().unwrap_or(0) as usize, e["end_col"].as_u64().unwrap_or(0) as usize);
        // (Columns in characters: to UTF-16 units from the line's start.)
        let start: usize = lines.iter().take(line - 1).map(|l| l.encode_utf16().count() + 1).sum();
        let l = lines.get(line - 1).copied().unwrap_or("");
        let u16_at = |c: usize| l.chars().take(c).map(char::len_utf16).sum::<usize>();
        let (a, b) = (u16_at(col), u16_at(end.max(col + 1)));
        found.push(serde_json::json!([start + a, (b - a).max(1), e["message"]]));
    }
    serde_json::Value::Array(found).to_string()
}

/// The tessh program: next to Tessera's own, else on PATH.
pub fn tessh_program() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    for p in [dir.join("tessh"), dir.join("../libexec/tessera/tessh")] {
        if p.is_file() {
            return Some(p);
        }
    }
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join("tessh")).find(|f| f.is_file()))
}

#[cfg(test)]
mod shell_check_tests {
    #[test]
    fn checks() {
        if std::process::Command::new("bash").arg("--version").output().is_err() {
            return;
        }
        assert_eq!(super::shell_check("bash", "echo hi && ls -la"), "[]");
        assert_ne!(super::shell_check("bash", "echo (hi"), "[]");
        assert_ne!(super::shell_check("bash", "if true; then echo x"), "[]");
    }
}

#[cfg(test)]
mod salvage {
    /// A highlights query with a rule this tree-sitter can't read keeps the rest.
    #[test]
    fn bad_rule_left_out() {
        let lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        let src = format!("{}\n((no_such_node) @keyword)\n(identifier) @variable\n", tree_sitter_rust::HIGHLIGHTS_QUERY);
        assert!(tree_sitter::Query::new(&lang, &src).is_err());
        let q = super::salvaged_query(&lang, &src).expect("salvaged");
        assert!(q.pattern_count() > 10);
    }
}

#[cfg(test)]
mod no_hang {
    /// Helix's Perl grammar never finished parsing this (no newline at the
    /// end); parsed as if it had one, it's quick. (Skipped without Helix's Perl.)
    #[test]
    fn last_node_read_with_its_newline() {
        // (Rust's rules read identifiers' text: a block ending in one read
        // one byte past the end.)
        for code in ["let x = y", "fn main() { foo }\nSOME_CONST", "x"] {
            let h = super::highlight_html("rust", code);
            assert!(h.contains(code.rsplit(['\n', ' ']).next().unwrap()), "{h}");
        }
    }

    #[test]
    fn perl_number_literals() {
        if super::lang("perl").is_none() || !crate::grammars::available().iter().any(|n| n == "perl") {
            return;
        }
        let code = "0\n12345\n4_294_967_296       # underscore for legibility\n0xff                # hex\n0Xdead_beef         # more hex\n0b01_1011           # binary\n0B110               # more binary\n0377                # octal (begins with 0 not followed by [bBxX])\n0o12_345            # alternative octal (introduced in Perl 5.33.5)";
        let t = std::time::Instant::now();
        let h = super::highlight_html("perl", code);
        assert!(t.elapsed() < std::time::Duration::from_secs(2), "{:?}", t.elapsed());
        assert!(h.contains("0xff") && h.ends_with("5.33.5)</span>") || h.ends_with("5.33.5)"), "{h}");
    }
}


#[cfg(test)]
mod grammar_probe {
    /// $PROBE_LANG's highlighting of $PROBE_TEXT, with its grammar's
    /// trial (`cargo test grammar_probe -- --ignored --nocapture`).
    #[test]
    #[ignore]
    fn probe() {
        let (lang, text) = (std::env::var("PROBE_LANG").unwrap(), std::env::var("PROBE_TEXT").unwrap_or_else(|_| "a = 1\n".into()));
        eprintln!("ensure: {:?}", crate::grammars::ensure(&lang));
        eprintln!("load: {}", crate::grammars::load(&lang).is_some());
        let q = crate::grammars::load(&lang).map(|l| tree_sitter::Query::new(&l, &crate::grammars::highlights(&lang)).map(|_| ()).map_err(|e| e.to_string()));
        eprintln!("query: {q:?}");
        eprintln!("{lang}: {}", super::highlight_html(&lang, &text));
    }
}
