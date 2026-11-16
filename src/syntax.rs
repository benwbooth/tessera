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
        let kind = names[c.index as usize].split('.').next().unwrap_or("");
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
