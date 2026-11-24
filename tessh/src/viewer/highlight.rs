//! Code highlighted for the viewer: the grammars Tessera's editor has built
//! in (src/syntax.rs: same crates, same capture kinds), in the colors tessh's
//! own editor uses for tessh (repl::kind_style), and Helix's ~250 more as
//! Tessera loads and fetches them (src/grammars.rs, shared); then bat's
//! syntaxes as Grove has them. A file's lines each styled on its own (so
//! any line can be shown, picked or scrolled alone).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator};

/// The language a file's name says, as Tessera's viewer reads it
/// (qml/FileView.qml's detectedLang): tree-sitter's and Helix's names; ""
/// when the name doesn't say.
pub fn lang_for(path: &std::path::Path) -> &'static str {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    match name.as_str() {
        "Makefile" | "makefile" | "GNUmakefile" => return "make",
        "Dockerfile" | "Containerfile" => return "dockerfile",
        "CMakeLists.txt" => return "cmake",
        ".bashrc" | ".zshrc" | ".profile" | ".bash_profile" | ".zprofile" | ".envrc" => return "bash",
        "Cargo.lock" | "uv.lock" | "poetry.lock" => return "toml",
        "flake.lock" | "package-lock.json" => return "json",
        _ => {}
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "rs" => "rust",
        "py" | "pyi" | "pyw" | "xsh" => "python",
        "js" | "mjs" | "cjs" | "jsx" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "ino" => "cpp",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "rb" | "rake" | "gemspec" => "ruby",
        "lua" => "lua",
        "sh" | "bash" | "zsh" | "ksh" => "bash",
        "fish" => "fish",
        "json" | "jsonc" | "json5" | "ipynb" | "jsonl" | "ndjson" | "geojson" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "html" | "htm" | "xhtml" => "html",
        "css" => "css",
        "scss" => "scss",
        "md" | "markdown" | "mdx" => "markdown",
        "sql" => "sql",
        "nix" => "nix",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "scala" | "sc" => "scala",
        "swift" => "swift",
        "zig" => "zig",
        "php" => "php",
        "pl" | "pm" => "perl",
        "r" => "r",
        "jl" => "julia",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "cs" => "c-sharp",
        "fs" | "fsx" => "fsharp",
        "dart" => "dart",
        "qml" => "qmljs",
        "ini" | "cfg" | "conf" => "ini",
        "dockerfile" => "dockerfile",
        "make" | "mk" => "make",
        "cmake" => "cmake",
        "tex" | "sty" | "cls" => "latex",
        "vim" => "vim",
        "nu" => "nu",
        "ps1" | "psm1" => "powershell",
        "diff" | "patch" => "diff",
        "xml" | "xaml" | "xsd" | "xsl" | "xslt" | "svg" | "plist" | "csproj" | "vbproj" | "fsproj" | "vcxproj" | "props" | "targets" | "resx" | "nuspec"
        | "wsdl" | "rss" | "atom" | "kml" | "gpx" | "ui" | "qrc" => "xml",
        "tsh" | "tessh" => "tessh",
        _ => "",
    }
}

/// What a text's first line says, when its name doesn't (FileView's sniff):
/// <!doctype html, <?xml or a tag, a #! line.
pub fn lang_from_shebang(head: &str) -> &'static str {
    let s = head.trim_start_matches('\u{feff}').trim_start();
    let lower: String = s.chars().take(64).collect::<String>().to_lowercase();
    if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        return "html";
    }
    if lower.starts_with("<?xml") || lower.starts_with("<!--") {
        return "xml";
    }
    let first = head.lines().next().unwrap_or("");
    let Some(rest) = first.strip_prefix("#!") else { return "" };
    let words: Vec<&str> = rest.split_whitespace().collect();
    let prog = match words.as_slice() {
        [env, p, ..] if env.ends_with("/env") => p.trim_start_matches("-S"),
        [p, ..] => p,
        [] => "",
    };
    let prog = prog.rsplit('/').next().unwrap_or(prog).trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    match prog {
        "python" => "python",
        "bash" | "sh" | "zsh" | "dash" | "ksh" => "bash",
        "fish" => "fish",
        "node" => "javascript",
        "deno" => "typescript",
        "perl" => "perl",
        "ruby" => "ruby",
        "lua" => "lua",
        "php" => "php",
        "tessh" => "tessh",
        "nu" => "nu",
        "pwsh" => "powershell",
        "Rscript" => "r",
        "julia" => "julia",
        "elixir" => "elixir",
        _ => "",
    }
}

/// Whether the viewer may fetch a Helix grammar it lacks (view: yes, once
/// per language, as Tessera does; cat: never waits on the network).
pub static FETCH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

struct Lang {
    language: Language,
    query: Query,
}

fn make(name: &str) -> Option<Lang> {
    let (language, query): (Language, String) = match name {
        "bash" => (tree_sitter_bash::LANGUAGE.into(), format!("{}\n(variable_name) @variable\n(simple_expansion) @variable\n(expansion) @variable\n", tree_sitter_bash::HIGHLIGHT_QUERY)),
        "python" => (tree_sitter_python::LANGUAGE.into(), tree_sitter_python::HIGHLIGHTS_QUERY.into()),
        "javascript" => (tree_sitter_javascript::LANGUAGE.into(), tree_sitter_javascript::HIGHLIGHT_QUERY.into()),
        "typescript" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            format!("{}\n{}", tree_sitter_javascript::HIGHLIGHT_QUERY, tree_sitter_typescript::HIGHLIGHTS_QUERY),
        ),
        "tsx" => (
            tree_sitter_typescript::LANGUAGE_TSX.into(),
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
        // Anything else: Helix's grammar for it, as Tessera loads it (tried in
        // a child first, so one that crashes can't take tessh with it).
        other => (crate::grammars::load(other)?, crate::grammars::highlights(other)),
    };
    let query = salvaged_query(&language, &query).or_else(|| Query::new(&language, "").ok())?;
    Some(Lang { language, query })
}

const BUILTIN: &[&str] = &["bash", "python", "javascript", "typescript", "tsx", "rust", "go", "c", "cpp", "java", "ruby", "lua", "sql", "json", "yaml", "toml", "html", "css", "markdown", "tessh"];

fn lang(name: &str) -> Option<&'static Lang> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<&'static Lang>>>> = OnceLock::new();
    let mut c = CACHE.get_or_init(Default::default).lock().unwrap();
    if let Some(l) = c.get(name) {
        return *l;
    }
    // (A Helix language that isn't here yet isn't remembered as missing: it
    // may be fetched in a moment.)
    let l = make(name).map(|l| &*Box::leak(Box::new(l)));
    if l.is_some() || BUILTIN.contains(&name) {
        c.insert(name.to_string(), l);
    }
    l
}

/// A highlights query less the rules this tree-sitter can't read (Helix's
/// are written for its own version), each error's rule taken out and the
/// query tried again: Tessera's syntax.rs salvaged_query.
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

/// The byte range of the top-level form (a rule) that `at` is in (Tessera's).
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
    forms.iter().copied().find(|&(s, e)| s <= at && at < e.max(s + 1)).or_else(|| forms.iter().copied().filter(|&(s, _)| s <= at).last()).or(forms.first().copied())
}

/// A Helix language made usable: fetched and unpacked from Helix's release
/// the first time (when FETCH allows), and tried. Whether it's there.
fn ensure_helix(name: &str) -> bool {
    if BUILTIN.contains(&name) || !crate::grammars::is_known(name) {
        return BUILTIN.contains(&name);
    }
    let here = crate::grammars::available().iter().any(|n| n == name);
    if !here && !FETCH.load(std::sync::atomic::Ordering::Relaxed) {
        return false;
    }
    if !here {
        eprint!("\x1b[2mview: getting Helix's {name} grammar (once)…\x1b[0m\r");
    }
    let ok = crate::grammars::ensure(name).is_ok();
    if !here {
        eprint!("\x1b[2K");
    }
    ok
}

/// A capture's color kind, as Tessera's syntax.rs reads it: its first part,
/// but prose markup as the code kind it reads like.
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

/// The SGR for a kind: tessh's editor colors, and for the kinds tessh's own
/// grammar doesn't have, the nearest of them.
fn sgr(kind: &str) -> &'static str {
    match kind {
        "keyword" => "\x1b[1;35m",
        "string" => "\x1b[32m",
        "comment" => "\x1b[2;3m",
        "number" | "constant" | "boolean" => "\x1b[35m",
        "operator" | "attribute" | "escape" => "\x1b[33m",
        "function" | "method" => "\x1b[1;34m",
        "variable" | "property" | "embedded" | "label" | "special" => "\x1b[36m",
        "punctuation" | "delimiter" => "\x1b[90m",
        "type" | "constructor" | "namespace" | "module" => "\x1b[33m",
        "tag" => "\x1b[34m",
        _ => "",
    }
}

/// `text` highlighted as `lang`, one string per line (each styled alone,
/// ending reset); None when there's no grammar for it or the parse takes too
/// long.
pub fn highlight_lines(lang_name: &str, text: &str) -> Option<Vec<String>> {
    let l = lang(lang_name)?;
    let mut p = Parser::new();
    p.set_language(&l.language).ok()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut timeout = |_: &tree_sitter::ParseState| if std::time::Instant::now() > deadline { std::ops::ControlFlow::Break(()) } else { std::ops::ControlFlow::Continue(()) };
    let bytes = text.as_bytes();
    let tree = p.parse_with_options(&mut |i, _| if i < bytes.len() { &bytes[i..] } else { &[] }, None, Some(tree_sitter::ParseOptions::new().progress_callback(&mut timeout)))?;
    let names = l.query.capture_names();
    // (Each byte's kind, later captures over earlier ones, as Tessera's.)
    let mut kinds: Vec<&'static str> = vec![""; text.len()];
    let mut cursor = QueryCursor::new();
    let padded;
    let source: &[u8] = if tree.root_node().end_byte() > text.len() {
        padded = format!("{text}\n");
        padded.as_bytes()
    } else {
        bytes
    };
    let mut spans: Vec<(usize, usize, &'static str)> = vec![];
    let mut caps = cursor.captures(&l.query, tree.root_node(), source);
    while let Some((m, i)) = caps.next() {
        let c = m.captures()[*i];
        let (s, e) = (c.node.start_byte(), c.node.end_byte().min(text.len()));
        if e > s {
            let k = sgr(kind_of(names[c.index as usize]));
            if !k.is_empty() {
                spans.push((s, e, k));
            }
        }
    }
    drop(caps);
    if lang_name == "tessh" {
        // (tessh's "…" strings split, so what they interpolate reads as code.)
        let ks: Vec<(usize, usize, &'static str)> = spans.iter().map(|&(s, e, k)| (s, e, kind_name(k))).collect();
        spans = tree_sitter_tessh::highlight::split_strings(text, ks).into_iter().map(|(s, e, k)| (s, e, sgr(k))).collect();
    }
    for (s, e, k) in spans {
        for x in &mut kinds[s..e] {
            *x = k;
        }
    }
    Some(paint_lines(text, &kinds))
}

/// Back from an SGR to its kind (tessh's string splitting goes by kind).
fn kind_name(sgr_code: &'static str) -> &'static str {
    ["keyword", "string", "comment", "number", "operator", "function", "variable", "punctuation", "type", "tag"].into_iter().find(|k| sgr(k) == sgr_code).unwrap_or("")
}

/// The text's lines, each byte in its kind's style.
fn paint_lines(text: &str, kinds: &[&'static str]) -> Vec<String> {
    let mut out = vec![];
    let mut start = 0;
    for line in text.split('\n') {
        let end = start + line.len();
        let mut s = String::with_capacity(line.len() + 16);
        let mut cur = "";
        for (k, ch) in line.char_indices() {
            let at = start + k;
            let want = kinds.get(at).copied().unwrap_or("");
            if want != cur {
                if !cur.is_empty() {
                    s.push_str("\x1b[0m");
                }
                s.push_str(want);
                cur = want;
            }
            s.push(ch);
        }
        if !cur.is_empty() {
            s.push_str("\x1b[0m");
        }
        out.push(s);
        start = end + 1;
    }
    out
}

// ---- every other language: bat's syntaxes (as Grove's Quick Look) --------

fn syntaxes() -> &'static syntect::parsing::SyntaxSet {
    static S: OnceLock<syntect::parsing::SyntaxSet> = OnceLock::new();
    S.get_or_init(two_face::syntax::extra_newlines)
}

/// bat's syntax for a file, found as Grove finds it: the whole name
/// (Makefile, .bashrc), the extension, close relatives, the first line.
fn syntax_for<'a>(ss: &'a syntect::parsing::SyntaxSet, name: &str, text: &str) -> Option<&'a syntect::parsing::SyntaxReference> {
    let ext = std::path::Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    ss.find_syntax_by_extension(name)
        .or_else(|| ss.find_syntax_by_extension(&name.to_lowercase()))
        .or_else(|| if ext.is_empty() { None } else { ss.find_syntax_by_extension(&ext) })
        .or_else(|| if ext.is_empty() { None } else { ss.find_syntax_by_token(&ext) })
        .or_else(|| match ext.as_str() {
            "vbs" | "vb" | "bas" | "frm" | "cls" | "vba" => ss.find_syntax_by_name("ASP"),
            "s" | "inc" | "a51" | "nasm" => ss.find_syntax_by_extension("asm"),
            "ahk" | "au3" => ss.find_syntax_by_name("Batch File"),
            _ => None,
        })
        .or_else(|| text.lines().next().and_then(|l| ss.find_syntax_by_first_line(l)))
        .filter(|s| s.name != "Plain Text")
}

/// A scope's kind (the palette's): the innermost scope that says one.
fn scope_kind(stack: &syntect::parsing::ScopeStack) -> &'static str {
    for sc in stack.as_slice().iter().rev() {
        let n = sc.build_string();
        let k = match () {
            _ if n.starts_with("comment") => "comment",
            _ if n.starts_with("string") || n.starts_with("markup.raw") => "string",
            _ if n.starts_with("constant.numeric") => "number",
            _ if n.starts_with("constant.character.escape") => "escape",
            _ if n.starts_with("constant") => "constant",
            _ if n.starts_with("keyword.operator") => "operator",
            _ if n.starts_with("keyword") || n.starts_with("storage.modifier") || n.starts_with("markup.heading") => "keyword",
            _ if n.starts_with("storage.type") || n.starts_with("entity.name.type") || n.starts_with("support.type") || n.starts_with("support.class") || n.starts_with("entity.name.class") || n.starts_with("entity.other.inherited-class") => "type",
            _ if n.starts_with("entity.name.function") || n.starts_with("support.function") || n.starts_with("variable.function") || n.starts_with("meta.function-call") || n.starts_with("markup.underline.link") => "function",
            _ if n.starts_with("entity.name.tag") => "tag",
            _ if n.starts_with("entity.other.attribute-name") || n.starts_with("markup.bold") || n.starts_with("markup.italic") => "attribute",
            _ if n.starts_with("variable") || n.starts_with("support.variable") || n.starts_with("entity.name") => "variable",
            _ if n.starts_with("punctuation") || n.starts_with("markup.list") || n.starts_with("markup.quote") => "punctuation",
            _ if n.starts_with("markup.inserted") => "string",
            _ if n.starts_with("markup.deleted") => "tag",
            _ => continue,
        };
        return k;
    }
    ""
}

/// `text` in bat's syntax for `name`: its lines styled, and the syntax's
/// name. None when none fits.
pub fn highlight_bat(name: &str, text: &str) -> Option<(Vec<String>, String)> {
    let ss = syntaxes();
    let syn = syntax_for(ss, name, text)?;
    let mut state = syntect::parsing::ParseState::new(syn);
    let mut stack = syntect::parsing::ScopeStack::new();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut out = Vec::new();
    for (n, line) in syntect::util::LinesWithEndings::from(text).enumerate() {
        // (Too slow: the rest plain, as it is.)
        if n % 256 == 0 && std::time::Instant::now() > deadline {
            out.extend(text.split_inclusive('\n').skip(n).map(|l| l.trim_end_matches('\n').to_string()));
            break;
        }
        let ops = state.parse_line(line, ss).unwrap_or_default();
        let body = line.trim_end_matches('\n');
        let mut s = String::with_capacity(body.len() + 16);
        let mut at = 0;
        let mut cur = "";
        let emit = |s: &mut String, cur: &mut &'static str, k: &'static str, piece: &str| {
            if piece.is_empty() {
                return;
            }
            let want = sgr(k);
            if want != *cur {
                if !cur.is_empty() {
                    s.push_str("\x1b[0m");
                }
                s.push_str(want);
                *cur = want;
            }
            s.push_str(piece);
        };
        for (i, op) in ops {
            let i = i.min(body.len());
            if i > at && body.is_char_boundary(i) {
                emit(&mut s, &mut cur, scope_kind(&stack), &body[at..i]);
                at = i;
            }
            let _ = stack.apply(&op);
        }
        if at < body.len() {
            emit(&mut s, &mut cur, scope_kind(&stack), &body[at..]);
        }
        if !cur.is_empty() {
            s.push_str("\x1b[0m");
        }
        out.push(s);
    }
    if text.ends_with('\n') || text.is_empty() {
        out.push(String::new());
    }
    Some((out, syn.name.clone()))
}

/// What a text is when its name doesn't say, by Grove's clear-cut content
/// rules (langguess.rs): an extension (json, xml, svg, html, ini, csv, tsv,
/// properties, md), or None.
pub fn guess_ext(text: &str) -> Option<&'static str> {
    let t = text.trim_start_matches('\u{feff}').trim();
    if t.len() < 2 {
        return None;
    }
    let lower: String = t.chars().take(256).collect::<String>().to_lowercase();
    if lower.starts_with("<svg") || (lower.starts_with("<?xml") && lower.contains("<svg")) {
        return Some("svg");
    }
    if lower.starts_with("<!doctype html") || lower.starts_with("<html") {
        return Some("html");
    }
    if lower.starts_with("<?xml") || (lower.starts_with('<') && t.ends_with('>') && t.contains("</")) {
        return Some("xml");
    }
    if t.starts_with('{') || t.starts_with('[') {
        let mut de = serde_json::Deserializer::from_str(t).into_iter::<serde_json::Value>();
        match de.next() {
            Some(Ok(_)) => return Some("json"),
            Some(Err(e)) if e.is_eof() && t.len() > 64 * 1024 => return Some("json"),
            _ => {}
        }
    }
    let lines: Vec<&str> = t.lines().filter(|l| !l.trim().is_empty()).take(200).collect();
    let sections = lines.iter().filter(|l| { let l = l.trim(); l.starts_with('[') && l.ends_with(']') && !l.contains(' ') }).count();
    let pairs = lines.iter().filter(|l| { let l = l.trim(); !l.starts_with('#') && !l.starts_with(';') && l.contains('=') && !l.starts_with('[') }).count();
    if sections >= 1 && sections + pairs + lines.iter().filter(|l| l.trim_start().starts_with(['#', ';'])).count() == lines.len() && pairs >= 1 {
        return Some("ini");
    }
    if lines.len() >= 3 {
        for (sep, kind) in [('\t', "tsv"), (',', "csv")] {
            let counts: Vec<usize> = lines.iter().map(|l| l.matches(sep).count()).collect();
            if counts[0] >= 1 && counts.iter().all(|&c| c == counts[0]) {
                return Some(kind);
            }
        }
    }
    // (Java .properties: every line a comment or key=value, the key one word.)
    let props = lines.len() >= 2 && lines.iter().all(|l| { let l = l.trim(); l.starts_with('#') || l.starts_with('!') || l.split_once('=').is_some_and(|(k, _)| !k.trim().is_empty() && !k.trim().contains(' ')) });
    let md_headings = lines.iter().filter(|l| l.starts_with("# ") || l.starts_with("## ") || l.starts_with("### ")).count();
    if !t.starts_with("#!") && md_headings >= 2 && (t.contains("](") || t.contains("```") || t.contains("\n- ")) {
        return Some("md");
    }
    if props {
        return Some("properties");
    }
    None
}

/// A text file's lines highlighted, and what it's written in, as Tessera
/// finds it: the name (FileView's table), the first line (a tag, a #!), then
/// Linguist's rules and Magika's model (src/detect.rs); highlighted with
/// Tessera's built-in grammars or Helix's. Else bat's syntaxes (Grove's), by
/// the name or what the content looks like; (plain lines, "") when nothing
/// fits.
pub fn highlight_file(path: &std::path::Path, text: &str) -> (Vec<String>, String) {
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let head: String = text.chars().take(16 * 1024).collect();
    let mut lang = lang_for(path).to_string();
    if lang.is_empty() {
        lang = lang_from_shebang(&head).to_string();
    }
    if lang.is_empty() {
        lang = crate::detect::language_of_file(&path.to_string_lossy(), false, &head);
    }
    if !lang.is_empty() && ensure_helix(&lang) {
        if let Some(l) = highlight_lines(&lang, text) {
            return (l, label(&lang));
        }
    }
    if let Some((l, n)) = highlight_bat(&name, text) {
        return (l, n);
    }
    // (A name that says plain text stays plain, as in Grove.)
    let plain = name.to_lowercase().ends_with(".txt") || name.to_lowercase().ends_with(".log");
    if !plain {
        if let Some(ext) = guess_ext(text) {
            if let Some((l, n)) = highlight_bat(&format!("guessed.{ext}"), text) {
                return (l, n);
            }
        }
    }
    (text.split('\n').map(String::from).collect(), String::new())
}

/// A tree-sitter language's name as people write it.
pub fn label(lang: &str) -> String {
    match lang {
        "javascript" => "JavaScript", "typescript" => "TypeScript", "tsx" => "TSX", "cpp" => "C++", "json" => "JSON", "yaml" => "YAML",
        "toml" => "TOML", "html" => "HTML", "css" => "CSS", "sql" => "SQL", "bash" => "Shell", "c" => "C", "go" => "Go", "rust" => "Rust",
        "python" => "Python", "java" => "Java", "ruby" => "Ruby", "lua" => "Lua", "markdown" => "Markdown", "tessh" => "tessh",
        other => {
            let mut c = other.chars();
            return c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default();
        }
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_is_colored_line_by_line() {
        let lines = highlight_lines("rust", "fn main() {\n    let s = \"hi\";\n}").unwrap();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("\x1b[1;35mfn"), "{:?}", lines[0]);
        assert!(lines[1].contains("\x1b[32m\"hi\""), "{:?}", lines[1]);
        assert_eq!(crate::editor::pager::strip(&lines[1]), "    let s = \"hi\";");
    }

    #[test]
    fn names_say_the_language() {
        assert_eq!(lang_for(std::path::Path::new("a/b.py")), "python");
        assert_eq!(lang_for(std::path::Path::new("Makefile")), "make");
        assert_eq!(lang_from_shebang("#!/usr/bin/env python3"), "python");
        assert!(highlight_lines("nosuch", "x").is_none());
    }

    #[test]
    fn bat_covers_what_tessera_has_no_grammar_for() {
        let (l, n) = highlight_file(std::path::Path::new("x.nix"), "{ a = 1; }\n");
        assert_eq!(n, "Nix");
        assert!(l[0].contains("\x1b["), "{:?}", l[0]);
        let (_, n) = highlight_file(std::path::Path::new("Dockerfile"), "FROM alpine\nRUN ls\n");
        assert!(n.contains("Dockerfile"), "{n}");
        // (No name to go by: the content says JSON.)
        let (_, n) = highlight_file(std::path::Path::new("noext"), "{\"a\": [1, 2]}");
        assert_eq!(n, "JSON");
    }

    #[test]
    fn helix_grammars_load_as_tesseras_do() {
        // (Only where Helix's runtime or Tessera's unpacked copy has Haskell.)
        if !crate::grammars::available().iter().any(|n| n == "haskell") {
            return;
        }
        let (l, n) = highlight_file(std::path::Path::new("Main.hs"), "main :: IO ()\nmain = putStrLn \"hi\"\n");
        assert_eq!(n, "Haskell");
        assert!(l[1].contains("\x1b[32m\"hi\""), "{:?}", l[1]);
    }
}
