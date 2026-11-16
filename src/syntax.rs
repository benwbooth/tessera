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

fn lang(name: &str) -> Option<&'static Lang> {
    fn make(language: Language, highlights: &str) -> Option<Lang> {
        let query = Query::new(&language, highlights).ok()?;
        Some(Lang { language, query })
    }
    static BASH: OnceLock<Option<Lang>> = OnceLock::new();
    static PYTHON: OnceLock<Option<Lang>> = OnceLock::new();
    static JS: OnceLock<Option<Lang>> = OnceLock::new();
    match name {
        "python" => PYTHON.get_or_init(|| make(tree_sitter_python::LANGUAGE.into(), tree_sitter_python::HIGHLIGHTS_QUERY)).as_ref(),
        "javascript" => JS.get_or_init(|| make(tree_sitter_javascript::LANGUAGE.into(), tree_sitter_javascript::HIGHLIGHT_QUERY)).as_ref(),
        _ => BASH.get_or_init(|| make(tree_sitter_bash::LANGUAGE.into(), tree_sitter_bash::HIGHLIGHT_QUERY)).as_ref(),
    }
}

/// The editor's language for the program at the prompt.
pub fn language_for(program: &str) -> &'static str {
    let p = program.rsplit(['/', '\\']).next().unwrap_or(program).trim_start_matches('-').to_lowercase();
    let p = p.trim_end_matches(".exe");
    if p.starts_with("python") || p == "ipython" || p == "bpython" {
        "python"
    } else if p == "node" || p == "deno" || p == "bun" {
        "javascript"
    } else {
        "bash"
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
    }
}
