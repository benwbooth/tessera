//! What a text or file is written in, by Google's Magika model (a small
//! network run on the CPU in Rust, by tract): for a file card whose name
//! doesn't say, and the input editor's multi-line text when its own rules
//! (syntax::detect_language) don't. The model is loaded once, on first use,
//! in whichever background thread asks first; never on the UI thread.

use std::sync::{Mutex, OnceLock};

fn session() -> &'static Mutex<Option<magika::Session>> {
    static S: OnceLock<Mutex<Option<magika::Session>>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(magika::Session::new().map_err(|e| eprintln!("tessera: magika: {e}")).ok()))
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

/// What these bytes are written in ("" when Magika can't say, or it's not
/// a language Tessera highlights).
pub fn language_of(bytes: &[u8]) -> String {
    if bytes.iter().all(|b| b.is_ascii_whitespace()) {
        return String::new();
    }
    let mut s = session().lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = s.as_mut() else { return String::new() };
    match s.identify_content(bytes) {
        Ok(t) => tessera_name(t.info().label).to_string(),
        Err(_) => String::new(),
    }
}

/// The same for a file here (its start and end, as Magika reads them).
pub fn language_of_file(path: &str) -> String {
    let mut s = session().lock().unwrap_or_else(|e| e.into_inner());
    let Some(s) = s.as_mut() else { return String::new() };
    match s.identify_file(path) {
        Ok(t) => tessera_name(t.info().label).to_string(),
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
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
