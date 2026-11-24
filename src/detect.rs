//! What a text or file is written in, for a file card whose name its own
//! table doesn't know, and the input editor's multi-line text when its own
//! rules (syntax::detect_language) don't. Two guessers, both only ever in
//! background threads, never on the UI thread:
//!
//! - Hyperpolyglot (GitHub Linguist's detector in Rust): ~500 languages by
//!   file name, extension, #! line, Linguist's heuristics for extensions
//!   several languages share, and its classifier among those. Sure when the
//!   name says (it never guesses from content alone).
//! - Google's Magika model (a small network run on the CPU by tract): from
//!   the content, for names Linguist doesn't know (no extension, no #!).
//!   Loaded once, on first use, in whichever background thread asks first.
//!
//! Plain text stays plain: a name Linguist calls Text (.txt) is only
//! coloured when Magika is at least 90% sure of something else.

use std::sync::{Mutex, OnceLock};

fn session() -> &'static Mutex<Option<magika::Session>> {
    static S: OnceLock<Mutex<Option<magika::Session>>> = OnceLock::new();
    S.get_or_init(|| {
        let s = magika::Session::new().map_err(|e| eprintln!("tessera: magika: {e}")).ok().map(|mut s| {
            // (Its scores too: how sure it is, for plain-text names.)
            s.options_mut().debug_mode = true;
            s
        });
        Mutex::new(s)
    })
}

/// Magika's label as the name Tessera highlights it by ("" for none: plain
/// text, binary, unknown).
fn tessera_name(label: &str) -> &'static str {
    match label {
        "python" => "python",
        "javascript" => "javascript",
        "typescript" => "typescript",
        "rust" => "rust",
        "go" => "go",
        "c" => "c",
        "cpp" => "cpp",
        "cs" => "c-sharp",
        "java" => "java",
        "kotlin" => "kotlin",
        "scala" => "scala",
        "swift" => "swift",
        "dart" => "dart",
        "ruby" | "gemfile" | "gemspec" => "ruby",
        "lua" => "lua",
        "php" => "php",
        "perl" => "perl",
        "r" => "r",
        "julia" => "julia",
        "haskell" => "haskell",
        "ocaml" => "ocaml",
        "elixir" => "elixir",
        "erlang" => "erlang",
        "clojure" => "clojure",
        "lisp" => "commonlisp",
        "zig" => "zig",
        "groovy" | "gradle" => "groovy",
        "shell" => "bash",
        "powershell" => "powershell",
        "batch" => "batch",
        "awk" => "awk",
        "tcl" => "tcl",
        "html" => "html",
        "css" => "css",
        "scss" => "scss",
        "vue" => "vue",
        "xml" | "xsd" | "svg" | "csproj" | "vcxproj" | "rdf" | "gml" | "kml" | "gpx" | "sgml" => "xml",
        "json" | "geojson" | "ipynb" => "json",
        "jsonl" => "json",
        "yaml" => "yaml",
        "toml" => "toml",
        "ini" => "ini",
        "markdown" => "markdown",
        "rst" => "rst",
        "latex" | "bib" => "latex",
        "sql" => "sql",
        "dockerfile" => "dockerfile",
        "makefile" => "make",
        "cmake" => "cmake",
        "diff" => "diff",
        "proto" | "textproto" => "proto",
        "hcl" => "hcl",
        "solidity" => "solidity",
        "verilog" => "verilog",
        "vhdl" => "vhdl",
        "fortran" => "fortran",
        "pascal" => "pascal",
        "matlab" => "matlab",
        "objectivec" => "objc",
        "csv" => "csv",
        "tsv" => "tsv",
        "llvm_ir" => "llvm",
        "asm" => "asm",
        "jinja" => "jinja2",
        "handlebars" => "handlebars",
        "twig" => "twig",
        "erb" => "embedded-template",
        "m4" => "m4",
        "prolog" => "prolog",
        "cobol" => "cobol",
        "smali" => "smali",
        "po" => "po",
        "ignorefile" | "gitattributes" => "gitignore",
        "gitmodules" => "git-config",
        "htaccess" => "apache",
        _ => "",
    }
}

/// Linguist's name for a language as the name Tessera highlights it by
/// ("" for none, or a grammar Tessera can't get).
fn linguist_name(name: &str) -> String {
    let n = match name {
        "Text" => return String::new(),
        "C++" | "Cuda" | "Metal" | "Objective-C++" | "Unified Parallel C" => "cpp",
        "C#" => "c-sharp",
        "F#" => "fsharp",
        "Shell" | "Tcsh" | "OpenRC runscript" | "Gentoo Ebuild" | "Gentoo Eclass" | "Alpine Abuild" => "bash",
        "Java Properties" => "properties",
        "Emacs Lisp" => "elisp",
        "Makefile" => "make",
        "TeX" => "latex",
        "QML" => "qmljs",
        "JSON with Comments" | "Jupyter Notebook" | "JSONLD" => "json",
        "Git Config" => "git-config",
        "Git Attributes" => "gitattributes",
        "Ignore List" => "gitignore",
        "Gettext Catalog" => "po",
        "Protocol Buffer" => "proto",
        "Cap'n Proto" => "capnp",
        "reStructuredText" => "rst",
        "Assembly" => "nasm",
        "Unix Assembly" => "gas",
        "Linker Script" => "ld",
        "HTML+ERB" | "JavaScript+ERB" => "embedded-template",
        "HTML+EEX" => "eex",
        "HTML+Django" => "htmldjango",
        "SVG" | "XML Property List" | "XSLT" | "Maven POM" | "Ant Build System" | "XProc" | "COLLADA" | "Web Ontology Language" => "xml",
        "PLSQL" | "PLpgSQL" | "TSQL" | "SQLPL" | "HiveQL" => "sql",
        "Graphviz (DOT)" => "dot",
        "Standard ML" => "sml",
        "SystemVerilog" => "verilog",
        "EditorConfig" | "desktop" => "ini",
        "Gradle" => "groovy",
        "Starlark" => "python",
        "Raku" | "Pod 6" => "perl",
        "Awk" => "awk",
        "Nu" => "nu",
        "SSH Config" => "sshclientconfig",
        "RobotFramework" => "robot",
        "Regular Expression" => "regex",
        "Open Policy Agent" => "rego",
        "Literate Haskell" => "haskell",
        "Literate Agda" => "agda",
        "JSX" => "javascript",
        "Python console" => "python",
        "Racket" => "scheme",
        "Diff" | "Darcs Patch" => "diff",
        _ => "",
    };
    // (Else its name as Helix's grammars are named: Rust → rust, Fortran
    // Free Form → fortran-free-form, the last only if there is one.)
    let n = if n.is_empty() { name.to_lowercase().replace(' ', "-") } else { n.to_string() };
    if crate::grammars::is_known(&n) { n } else { String::new() }
}

/// What a file with this name is written in, by Linguist's rules (its name,
/// extension, #! line and heuristics; `head` is its start, for the last
/// two). None: Linguist doesn't know the name and the head has no #!; Some("")
/// for a language Tessera doesn't colour, or plain text.
fn linguist(file_name: &str, head: &[u8]) -> Option<String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let base = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
    if base.is_empty() || base == "." || base == ".." {
        return None;
    }
    // (Hyperpolyglot reads a file: the head, as UTF-8, in a scratch folder
    // under the file's own name. Never the file itself: it would read all
    // of it, and give up on bytes that aren't UTF-8.)
    let dir = std::env::temp_dir().join(format!("tessera-lang-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(base);
    let mut text = String::from_utf8_lossy(&head[..head.len().min(64 * 1024)]).into_owned();
    if text.ends_with('\u{FFFD}') {
        // (A character cut in two at the end.)
        text.pop();
    }
    let r = std::fs::write(&path, text.as_bytes()).ok().and_then(|_| hyperpolyglot::detect(&path).ok().flatten());
    let _ = std::fs::remove_dir_all(&dir);
    let d = r?;
    if std::env::var_os("TESSERA_DEBUG_LANG").is_some() {
        eprintln!("tessera: hyperpolyglot {base}: {} ({})", d.language(), d.variant());
    }
    Some(linguist_name(d.language()))
}

/// Magika's verdict, with how sure it is (0 to 1).
fn magika_guess(t: &magika::FileType) -> (&'static str, f32) {
    let score = t.debug_info().and_then(|d| d.model()).map(|m| m.output[m.best_guess()]).unwrap_or(1.0);
    (tessera_name(t.info().label), score)
}

/// What these bytes are written in ("" when Magika can't say, or it's not
/// a language Tessera highlights).
pub fn language_of(bytes: &[u8]) -> String {
    if bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return String::new();
    }
    // (A #! line: Linguist knows ~100 interpreters.)
    if bytes.starts_with(b"#!") {
        if let Some(l) = linguist("snippet", bytes).filter(|l| !l.is_empty()) {
            return l;
        }
    }
    let mut s = session().lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = s.as_mut() else { return String::new() };
    match s.identify_content(bytes) {
        Ok(t) => tessera_name(t.info().label).to_string(),
        Err(_) => String::new(),
    }
}

/// Names that say "plain text".
fn plain_name(name: &str) -> bool {
    let n = name.to_lowercase();
    matches!(n.rsplit_once('.').map(|(_, e)| e), Some("txt" | "text"))
}

/// The same for a file: here (`local`: read from disk) or on another host
/// (`head`: its first lines, as the card read them). Linguist's rules first
/// (by its name, and its start for names several languages share), then
/// Magika's model (by its start and end) when the name says nothing; a
/// plain-text name only for a language Magika is at least 90% sure of, or
/// Java properties by a strict rule.
pub fn language_of_file(path: &str, local: bool, head: &str) -> String {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let head = if local { read_head(path) } else { head.as_bytes().to_vec() };
    let plain = plain_name(name);
    if let Some(l) = linguist(name, &head) {
        if !l.is_empty() || !plain {
            return l;
        }
    }
    let mut s = session().lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = s.as_mut() else { return String::new() };
    let t = if local {
        s.identify_file(path)
    } else if head.iter().all(|b| b.is_ascii_whitespace()) {
        return String::new();
    } else {
        s.identify_content(&head[..]).map(magika::FileType::Regular)
    };
    match t {
        Ok(t) => {
            let (l, score) = magika_guess(&t);
            if std::env::var_os("TESSERA_DEBUG_LANG").is_some() {
                eprintln!("tessera: magika {name}: {} ({score:.2})", t.info().label);
            }
            if !plain || score >= 0.9 {
                return l.to_string();
            }
            // (Settings saved as .txt, as JXplorer does: Linguist calls any
            // .txt Text, and Magika says txt. A strict rule instead.)
            if looks_like_properties(&String::from_utf8_lossy(&head)) { "properties".to_string() } else { String::new() }
        }
        Err(_) => String::new(),
    }
}

/// Java .properties content: every line that isn't blank or a # / !
/// comment is key=value or key: value (no spaces in the key, no [section]),
/// at least five of them.
fn looks_like_properties(text: &str) -> bool {
    let mut pairs = 0;
    for l in text.lines().map(str::trim).filter(|l| !l.is_empty()).take(400) {
        if l.starts_with('#') || l.starts_with('!') {
            continue;
        }
        let Some(i) = l.find(['=', ':']) else { return false };
        let key = l[..i].trim_end();
        if key.is_empty() || key.contains(char::is_whitespace) || key.starts_with('[') {
            return false;
        }
        pairs += 1;
    }
    pairs >= 5
}

/// A file's first 64 KB (empty if it can't be read).
fn read_head(path: &str) -> Vec<u8> {
    use std::io::Read;
    let mut buf = Vec::new();
    if let Ok(f) = std::fs::File::open(path) {
        let _ = f.take(64 * 1024).read_to_end(&mut buf);
    }
    buf
}

#[cfg(test)]
mod tests {
    #[test]
    fn linguist_by_name() {
        use super::linguist;
        // (A Java .properties file: Magika says txt; Linguist knows it.)
        assert_eq!(linguist("app.properties", b"# settings\nhost=example.org\nport=389\n").as_deref(), Some("properties"));
        assert_eq!(linguist("Jenkinsfile", b"pipeline { agent any }\n").as_deref(), Some("groovy"));
        assert_eq!(linguist("build.gradle", b"plugins { id 'java' }\n").as_deref(), Some("groovy"));
        assert_eq!(linguist(".gitignore", b"target/\n").as_deref(), Some("gitignore"));
        assert_eq!(linguist("x.ex", b"defmodule X do\nend\n").as_deref(), Some("elixir"));
        // (Shared extensions: Linguist's heuristics.)
        assert_eq!(linguist("a.pl", b"#!/usr/bin/perl\nuse strict;\nmy $x = 1;\nprint \"$x\\n\";\n").as_deref(), Some("perl"));
        assert_eq!(linguist("a.pl", b"parent(tom, bob).\nancestor(X, Y) :- parent(X, Y).\n").as_deref(), Some("prolog"));
        // (A #! line, no extension.)
        assert_eq!(linguist("deploy", b"#!/usr/bin/env ruby\nputs 1\n").as_deref(), Some("ruby"));
        // (Plain text stays plain; a name Linguist doesn't know: no answer.)
        assert_eq!(linguist("notes.txt", b"key=value\n").as_deref(), Some(""));
        assert_eq!(linguist("data.zzq", b"hello\n"), None);
        assert!(super::looks_like_properties("# JXplorer\na=1\nb.c=2\nd: 3\ne=\nf=x y\n"));
        assert!(!super::looks_like_properties("Buy milk.\na=1\nb=2\nc=3\nd=4\ne=5\n"));
        assert!(!super::looks_like_properties("[core]\na=1\nb=2\nc=3\nd=4\ne=5\n"));
    }

    #[test]
    #[ignore] // (loads the model: cargo test -- --ignored)
    fn knows_common_languages() {
        let xml = br#"<?xml version="1.0" encoding="utf-8" ?>
<nlog xmlns="http://www.nlog-project.org/schemas/NLog.xsd">
  <targets>
    <target name="logfile" xsi:type="File" fileName="./applog.txt"/>
  </targets>
  <rules>
    <logger name="*" minlevel="Info" writeTo="logfile" />
  </rules>
</nlog>
"#;
        assert_eq!(super::language_of(xml), "xml");
        let py = b"import os\n\ndef main():\n    for name in os.listdir('.'):\n        print(name)\n\nif __name__ == '__main__':\n    main()\n";
        assert_eq!(super::language_of(py), "python");
        let rs = b"use std::collections::HashMap;\n\nfn main() {\n    let mut m = HashMap::new();\n    m.insert(1, \"one\");\n    println!(\"{:?}\", m);\n}\n";
        assert_eq!(super::language_of(rs), "rust");
    }
}
