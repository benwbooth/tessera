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
    // A query a grammar's version can't read leaves it unhighlighted (but
    // still checked).
    let query = Query::new(&language, &query).or_else(|_| Query::new(&language, "")).ok()?;
    Some(Lang { language, query })
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
    let Some(tree) = p.parse(text, None) else { return esc(text) };
    // (The innermost capture wins: kinds by byte, later captures over earlier.)
    let mut kind: Vec<Option<&str>> = vec![None; text.len()];
    let names = l.query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut caps = cursor.captures(&l.query, tree.root_node(), text.as_bytes());
    while let Some((m, i)) = caps.next() {
        let c = m.captures()[*i];
        let k = kind_of(names[c.index as usize]);
        for b in c.node.start_byte()..c.node.end_byte().min(text.len()) {
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
    let Some(l) = lang(lang_name) else { return json!({"spans": [], "errors": []}).to_string() };
    thread_local! { static PARSER: RefCell<Parser> = RefCell::new(Parser::new()); }
    let Some(tree) = PARSER.with(|p| {
        let mut p = p.borrow_mut();
        p.set_language(&l.language).ok()?;
        p.parse(text, None)
    }) else {
        return json!({"spans": [], "errors": []}).to_string();
    };
    // Byte offset -> UTF-16 offset.
    let mut u16_at = vec![0u32; text.len() + 1];
    let mut n = 0u32;
    for (i, c) in text.char_indices() {
        for b in 0..c.len_utf8() {
            u16_at[i + b] = n;
        }
        n += c.len_utf16() as u32;
    }
    u16_at[text.len()] = n;
    let range = |node: Node| {
        let (s, e) = (u16_at[node.start_byte()], u16_at[node.end_byte()]);
        (s, e.saturating_sub(s))
    };

    let mut spans = Vec::new();
    let names = l.query.capture_names();
    let mut cursor = QueryCursor::new();
    let mut caps = cursor.captures(&l.query, tree.root_node(), text.as_bytes());
    while let Some((m, i)) = caps.next() {
        let c = m.captures()[*i];
        let kind = kind_of(names[c.index as usize]);
        let (s, len) = range(c.node);
        if len > 0 {
            spans.push(json!([s, len, kind]));
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
    p.parse(text, None).is_some_and(|t| t.root_node().has_error())
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
