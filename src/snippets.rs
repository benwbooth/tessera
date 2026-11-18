//! Code typed into the input editor in a language other than the shell's:
//! is it the shell's at all (shell_doubt), which language does it look like
//! (rank, from the AI's guesses and what was used lately), and the shell
//! command that runs it with that language's interpreter (command).
//!
//! A snippet runs with the terminal as its input (so `input()` and the like
//! work): inline (`python3 -c '…'`) where the interpreter takes code as an
//! argument, else written to a file by a here-document on the same command
//! line (so the notebook card shows the code) and run from there. A REPL
//! that only reads code (evcxr) gets it on its input instead.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum How {
    /// The code as the last argument.
    Inline,
    /// The code in a file `snippet.<ext>`, `{f}` in the command (`{b}`: a
    /// binary next to it).
    File(&'static str),
    /// The code on the program's input.
    Stdin,
}

struct Runner {
    program: &'static str,
    cmd: &'static str,
    how: How,
    /// The nixpkgs package that has it (when nothing is installed).
    nix: &'static str,
}

// (A macro, not a fn: the tables are 'static.)
macro_rules! r {
    ($program:expr, $cmd:expr, $how:expr, $nix:expr) => {
        Runner { program: $program, cmd: $cmd, how: $how, nix: $nix }
    };
}

fn runners(lang: &str) -> &'static [Runner] {
    use How::*;
    match lang {
        "python" => &[r!("python3", "python3 -c", Inline, "python3"), r!("python", "python -c", Inline, ""), r!("uv", "uv run --no-project python -c", Inline, "uv")],
        "javascript" => &[r!("node", "node -e", Inline, "nodejs"), r!("deno", "deno eval", Inline, "deno"), r!("bun", "bun -e", Inline, "bun")],
        "typescript" | "tsx" => &[r!("deno", "deno eval --ext=ts", Inline, "deno"), r!("bun", "bun -e", Inline, "bun")],
        "ruby" => &[r!("ruby", "ruby -e", Inline, "ruby")],
        "perl" => &[r!("perl", "perl -e", Inline, "perl")],
        "lua" => &[r!("lua", "lua -e", Inline, "lua"), r!("luajit", "luajit -e", Inline, "luajit")],
        "php" => &[r!("php", "php -r", Inline, "php")],
        "r" => &[r!("Rscript", "Rscript -e", Inline, "R")],
        "julia" => &[r!("julia", "julia -e", Inline, "julia")],
        "elixir" => &[r!("elixir", "elixir -e", Inline, "elixir")],
        "erlang" => &[r!("erl", "erl -noshell -eval {c} -s init stop", Inline, "erlang"), r!("escript", "escript {f}", File("erl"), "erlang")],
        "groovy" => &[r!("groovy", "groovy -e", Inline, "groovy")],
        "powershell" => &[r!("pwsh", "pwsh -c", Inline, "powershell")],
        "fish" => &[r!("fish", "fish -c", Inline, "fish")],
        "nu" => &[r!("nu", "nu -c", Inline, "nushell")],
        "tcl" => &[r!("tclsh", "tclsh", Stdin, "tcl")],
        "awk" => &[r!("awk", "awk", Inline, "gawk")],
        "jq" => &[r!("jq", "jq -n", Inline, "jq")],
        "nix" => &[r!("nix", "nix eval --impure --expr", Inline, "")],
        "octave" | "matlab" => &[r!("octave", "octave --eval", Inline, "octave")],
        "sql" => &[r!("duckdb", "duckdb -c", Inline, "duckdb"), r!("sqlite3", "sqlite3 :memory:", Stdin, "sqlite")],
        "rust" => &[
            r!("evcxr", "evcxr", Stdin, ""),
            r!("rust-script", "rust-script -e", Inline, "rust-script nixpkgs#cargo nixpkgs#rustc"),
            r!("rustc", "rustc -o {b} {f} && {b}", File("rs"), "rustc"),
        ],
        // (yaegi: Go's interpreter, a program's main run at the end by itself.)
        "go" => &[r!("yaegi", "yaegi", Stdin, "yaegi"), r!("go", "go run {f}", File("go"), "go")],
        "c" => &[r!("tcc", "tcc -run -", Stdin, "tinycc"), r!("cc", "cc -o {b} {f} && {b}", File("c"), "gcc")],
        // (cling: C++'s interpreter, statements as they come.)
        "cpp" => &[r!("cling", "cling -std=c++23 -w --nologo", Stdin, "cling"), r!("c++", "c++ -std=c++23 -o {b} {f} && {b}", File("cpp"), "gcc")],
        // (Statements: jbang --code, or JShell; a class: see command.)
        "java" => &[r!("jbang", "jbang --code", Inline, "jbang"), r!("jshell", "jshell --feedback silent -", Stdin, "jdk")],
        "kotlin" => &[r!("kotlin", "kotlin -e", Inline, "kotlin")],
        "scala" => &[r!("scala-cli", "scala-cli run -e", Inline, "scala-cli")],
        "clojure" => &[r!("bb", "bb -e", Inline, "babashka"), r!("clojure", "clojure -M -e", Inline, "clojure")],
        // (A program, with a main: runghc from its input. Else GHCi runs the lines.)
        "haskell" => &[r!("ghci", "ghci -v0", Stdin, "ghc")],
        "ocaml" => &[r!("ocaml", "ocaml -stdin", Stdin, "ocaml")],
        "racket" => &[r!("racket", "racket -e", Inline, "racket")],
        "scheme" => &[r!("guile", "guile -c", Inline, "guile")],
        "common-lisp" | "lisp" => &[r!("sbcl", "sbcl --script", Stdin, "sbcl")],
        "dart" => &[r!("dart", "dart run /dev/stdin", Stdin, "dart")],
        "swift" => &[r!("swift", "swift {f}", File("swift"), "swift")],
        "zig" => &[r!("zig", "zig run {f}", File("zig"), "zig")],
        "nim" => &[r!("nim", "nim --hints:off --eval:{c}", Inline, "nim"), r!("nim", "nim r --hints:off {f}", File("nim"), "nim")],
        "crystal" => &[r!("crystal", "crystal eval", Inline, "crystal")],
        "d" => &[r!("rdmd", "rdmd {f}", File("d"), "dmd")],
        "v" => &[r!("v", "v -e", Inline, "vlang")],
        "fsharp" => &[r!("dotnet", "dotnet fsi {f}", File("fsx"), "dotnet-sdk")],
        "c-sharp" => &[r!("dotnet-script", "dotnet-script {f}", File("csx"), "dotnet-script")],
        "prolog" => &[r!("swipl", "swipl {f}", File("pl"), "swi-prolog")],
        "fortran" => &[r!("gfortran", "gfortran -o {b} {f} && {b}", File("f90"), "gfortran")],
        _ => &[],
    }
}

/// The editor's name for what a model (or a person) calls a language.
pub fn normalize(name: &str) -> String {
    let n = name.trim().trim_matches(|c: char| c == '"' || c == '\'' || c == '`').to_lowercase();
    match n.as_str() {
        "shell" | "sh" | "zsh" | "bash" | "shell script" | "posix shell" | "ksh" => "bash",
        "py" | "python3" | "ipython" => "python",
        "js" | "node" | "nodejs" | "node.js" => "javascript",
        "ts" | "deno" => "typescript",
        "c++" | "cxx" => "cpp",
        "c#" | "csharp" | "cs" => "c-sharp",
        "f#" => "fsharp",
        "golang" => "go",
        "rs" => "rust",
        "rb" => "ruby",
        "pl" => "perl",
        "ps1" | "pwsh" => "powershell",
        "nushell" => "nu",
        "sqlite" | "postgresql" | "mysql" => "sql",
        "lisp" | "commonlisp" => "common-lisp",
        "kt" | "kts" => "kotlin",
        "hs" => "haskell",
        "ex" | "exs" => "elixir",
        "yml" => "yaml",
        "md" => "markdown",
        _ => return n,
    }
    .to_string()
}

/// Can `lang` be run (with an installed program, or one Nix can fetch)?
pub fn runnable(lang: &str) -> bool {
    !runners(lang).is_empty()
}

/// The program `nix run nixpkgs#<pkg>` starts (its mainProgram), where
/// that's the runner: then it's `nix run`, else `nix shell ... -c`.
fn nix_main(pkg: &str) -> Option<&'static str> {
    Some(match pkg {
        "python3" => "python3",
        "nodejs" => "node",
        "deno" => "deno",
        "bun" => "bun",
        "ruby" => "ruby",
        "perl" => "perl",
        "lua" => "lua",
        "php" => "php",
        "julia" => "julia",
        "uv" => "uv",
        "jq" => "jq",
        "duckdb" => "duckdb",
        "rust-script" => "rust-script",
        "jbang" => "jbang",
        "scala-cli" => "scala-cli",
        "babashka" => "bb",
        "ghc" => "ghc",
        "racket" => "racket",
        "guile" => "guile",
        "crystal" => "crystal",
        "elixir" => "elixir",
        "nushell" => "nu",
        "fish" => "fish",
        _ => return None,
    })
}

/// `cmd` (a runner's command, its program first) run from nixpkgs.
fn from_nix(pkg: &str, cmd: &str) -> String {
    let (prog, rest) = cmd.split_once(' ').unwrap_or((cmd, ""));
    if nix_main(pkg) == Some(prog) {
        format!("nix run nixpkgs#{pkg} -- {rest}").trim_end().to_string()
    } else {
        format!("nix shell nixpkgs#{pkg} -c {cmd}")
    }
}

fn on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
}

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Tessera's C++ runner (a script in the cache folder, written when it
/// changes): clang-repl from nixpkgs' latest LLVM, with libc++ and its std
/// module (built once per compiler), the code as its argument.
const CXX_RUNNER: &str = r#"#!/bin/sh
# Tessera's C++ runner: clang-repl (LLVM's C++ interpreter, from nixpkgs)
# with libc++'s std module, so `import std;` works. Usage: tessera-cxx CODE
set -e
dir="${XDG_CACHE_HOME:-$HOME/.cache}/tessera/cxx"
mkdir -p "$dir"
# (GC roots, so Nix keeps them; fetched once.)
for x in clang:llvmPackages_latest.clang-unwrapped^out libcxx:llvmPackages_latest.libcxx^out libcxxdev:llvmPackages_latest.libcxx^dev glibcdev:glibc^dev; do
  name=${x%%:*}; attr=${x#*:}
  # (A ^dev output's link gets -dev on its name.)
  if [ ! -e "$dir/$name" ] && [ ! -e "$dir/$name-dev" ]; then
    echo "tessera: getting $attr from nixpkgs (once)..." >&2
    nix build --quiet "nixpkgs#$attr" --out-link "$dir/$name" >&2
  fi
done
at() { if [ -e "$dir/$1" ]; then readlink -f "$dir/$1"; else readlink -f "$dir/$1-dev"; fi; }
C=$(at clang); L=$(at libcxx); D=$(at libcxxdev); G=$(at glibcdev)
flags="-std=c++23 -nostdinc++ -isystem $D/include/c++/v1 -isystem $G/include"
pcm="$dir/pcm-$(basename "$C")"
if [ ! -f "$pcm/std.pcm" ]; then
  echo "tessera: building C++'s std module (once)..." >&2
  mkdir -p "$pcm"
  "$C/bin/clang++" $flags -Wno-reserved-module-identifier --precompile -o "$pcm/std.pcm.tmp" "$L/share/libc++/v1/std.cppm" >&2
  mv "$pcm/std.pcm.tmp" "$pcm/std.pcm"
fi
xcc=""
for f in $flags -fprebuilt-module-path=$pcm -Wno-main; do xcc="$xcc --Xcc=$f"; done
LD_PRELOAD="$L/lib/libc++.so.1 $L/lib/libc++abi.so.1${LD_PRELOAD:+ $LD_PRELOAD}" exec "$C/bin/clang-repl" $xcc "$@"
"#;

fn cxx_runner() -> Option<String> {
    let base = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    let path = base.join("tessera").join("tessera-cxx");
    if std::fs::read_to_string(&path).ok().as_deref() != Some(CXX_RUNNER) {
        std::fs::create_dir_all(path.parent()?).ok()?;
        std::fs::write(&path, CXX_RUNNER).ok()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).ok()?;
        }
    }
    // (Shown as ~/..., as typed.)
    let home = std::env::var("HOME").unwrap_or_default();
    let p = path.to_string_lossy().to_string();
    Some(if !home.is_empty() && p.starts_with(&home) { format!("~{}", &p[home.len()..]) } else { p })
}

/// The installed g++'s major version.
fn gcc_major() -> Option<u32> {
    if !on_path("g++") {
        return None;
    }
    let o = std::process::Command::new("g++").arg("-dumpversion").output().ok()?;
    String::from_utf8_lossy(&o.stdout).trim().split('.').next()?.parse().ok()
}

/// Go for yaegi: the standard packages it uses imported (unless it says).
fn go_imports(code: &str) -> String {
    if code.contains("import") {
        return code.to_string();
    }
    let used: String = ["fmt", "os", "strings", "math", "time", "sort", "strconv"]
        .into_iter()
        .filter(|p| code.contains(&format!("{p}.")))
        .map(|p| format!("import \"{p}\"\n"))
        .collect();
    format!("{used}{code}")
}

/// Rust for rust-script -e (a block): statements as they are, a program's
/// main called at the end.
fn rust_expr(code: &str) -> String {
    if code.contains("fn main") { format!("{code}\nmain()") } else { code.to_string() }
}

/// Make a bare snippet a whole program where the language needs one
/// (`fmt.Println("hi")` in Go, `printf(...)` in C).
fn complete_program(lang: &str, code: &str) -> String {
    match lang {
        "go" if !code.contains("package ") => {
            if code.contains("func main") {
                format!("package main\n\n{code}\n")
            } else {
                let imports: Vec<&str> = ["fmt", "os", "strings", "math", "time", "sort", "strconv"]
                    .into_iter()
                    .filter(|p| code.contains(&format!("{p}.")))
                    .collect();
                let imp = if imports.is_empty() { String::new() } else { format!("import ({})\n", imports.iter().map(|p| format!("\"{p}\"")).collect::<Vec<_>>().join("; ")) };
                format!("package main\n{imp}\nfunc main() {{\n{code}\n}}\n")
            }
        }
        "c" if !code.contains("main(") => format!("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n\nint main(void) {{\n{code}\nreturn 0;\n}}\n"),
        "cpp" if !code.contains("main(") => format!("#include <bits/stdc++.h>\nusing namespace std;\n\nint main() {{\n{code}\nreturn 0;\n}}\n"),
        "rust" if !code.contains("fn main") => format!("fn main() {{\n{code}\n}}\n"),
        "dart" if !code.contains("main(") => format!("void main() {{\n{code}\n}}\n"),
        "racket" if !code.trim_start().starts_with("#lang") => format!("#lang racket\n{code}\n"),
        "php" => code.trim_start().trim_start_matches("<?php").to_string(),
        _ => code.to_string(),
    }
}

fn snippet_dir() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("tessera-snip")
}

/// The shell command that runs `code` as `lang` (in `cwd`, for `shell`),
/// or "" when the language can't be run. `shell` decides how a file is
/// written: a here-document in sh-like shells, else Tessera writes it.
pub fn command(lang: &str, code: &str, cwd: &str, shell: &str) -> String {
    let list = runners(lang);
    let code_t = code.trim_end();
    // (Python: in a uv project, its environment; modules it imports that
    // aren't installed, fetched by uv for the run. uv itself comes from
    // nixpkgs if need be.)
    if lang == "python" {
        let missing = missing_python_modules(code_t);
        let project = Path::new(cwd).join("pyproject.toml").is_file() && on_path("uv");
        let uv = if on_path("uv") { "uv" } else if on_path("nix") { "nix run nixpkgs#uv --" } else { "" };
        if !uv.is_empty() && (project || !missing.is_empty()) {
            let mut args: Vec<String> = if project { Vec::new() } else { vec!["--no-project".into()] };
            args.extend(missing.iter().map(|m| format!("--with {}", python_package(m))));
            return format!("{uv} run {} python -c {}", args.join(" "), quote(code_t)).replace("run  python", "run python");
        }
    }
    // (JavaScript that imports packages: Bun installs them as it runs.)
    if matches!(lang, "javascript" | "typescript") && !js_packages(code_t).is_empty() {
        let bun = if on_path("bun") { "bun".to_string() } else if on_path("nix") { "nix run nixpkgs#bun --".to_string() } else { String::new() };
        if !bun.is_empty() {
            return file_command(code_t, How::File(if lang == "typescript" { "ts" } else { "mjs" }), &format!("{bun} {{f}}"), "", shell);
        }
    }
    // (Rust's crates: evcxr's :dep lines, or rust-script's cargo-deps.)
    if lang == "rust" {
        let crates = rust_crates(code_t);
        if !crates.is_empty() {
            if on_path("evcxr") {
                let deps: String = crates.iter().map(|c| format!(":dep {c}\n")).collect();
                return file_command(&format!("{deps}{code_t}"), How::Stdin, "evcxr", "", shell);
            }
            let rs = if on_path("rust-script") && on_path("cargo") {
                "rust-script".to_string()
            } else if on_path("nix") {
                "nix shell nixpkgs#rust-script nixpkgs#cargo nixpkgs#rustc -c rust-script".to_string()
            } else {
                return String::new();
            };
            let deps: String = crates.iter().map(|c| format!(" -d {c}")).collect();
            return format!("{rs}{deps} -e {}", quote(&rust_expr(code_t)));
        }
    }
    // (C++: clang-repl, LLVM's interpreter, with libc++'s std module, so
    // `import std;` works and nothing is compiled first. Nix brings it; see
    // CXX_RUNNER. Without Nix: cling, or GCC below.)
    if lang == "cpp" && on_path("nix") && !code_t.lines().any(|l| { let t = l.trim_start(); t.starts_with("export module") || t.starts_with("module ") }) {
        if let Some(runner) = cxx_runner() {
            let mut c = code_t.to_string();
            if !c.contains("#include") && !c.lines().any(|l| l.trim_start().starts_with("import ")) {
                c = format!("import std;\n{c}");
            }
            if c.contains("main(") {
                c.push_str("\nmain();");
            }
            // (~ unquoted, so the shell expands it.)
            let r = if runner.chars().all(|ch| ch.is_ascii_alphanumeric() || "~/._-".contains(ch)) { runner } else { quote(&runner) };
            return format!("{r} {}", quote(&c));
        }
    }
    // (C++ with modules, import std; and the like: GCC 15 or later builds
    // it, in the snippet's folder, where its module cache goes.)
    if lang == "cpp" && code_t.lines().any(|l| { let t = l.trim_start(); t.starts_with("import ") || t.starts_with("export module") || t.starts_with("module ") }) {
        let local = gcc_major().is_some_and(|v| v >= 15);
        let n = if local { "" } else if on_path("nix") { "gcc" } else { return String::new() };
        let prog = if code_t.contains("main(") { code_t.to_string() } else { format!("{}\nint main() {{\n{}\n}}\n", code_t.lines().filter(|l| l.trim_start().starts_with("import ")).collect::<Vec<_>>().join("\n"), code_t.lines().filter(|l| !l.trim_start().starts_with("import ")).collect::<Vec<_>>().join("\n")) };
        return file_command(&prog, How::File("cpp"), "(cd {d} && g++ -std=c++23 -fmodules -fsearch-include-path bits/std.cc snippet.cpp -o snippet) && {b}", n, shell);
    }
    // (An Erlang module: escript runs it.)
    if lang == "erlang" && code_t.contains("-module(") {
        let n = if on_path("escript") { "" } else { "erlang" };
        return file_command(code_t, How::File("erl"), "escript {f}", n, shell);
    }
    // (A Haskell program, main and all: runghc reads it from its input.)
    if lang == "haskell" && code_t.lines().any(|l| l.starts_with("main ") || l.starts_with("main=")) {
        if on_path("runghc") {
            return file_command(code_t, How::Stdin, "runghc", "", shell);
        }
        if on_path("nix") {
            return file_command(code_t, How::Stdin, "runghc", "ghc", shell);
        }
    }
    // (A Java class: JBang from its input, or java's own source launcher.)
    if lang == "java" && code_t.contains("class ") {
        if on_path("jbang") || (!on_path("java") && on_path("nix")) {
            let jb = if on_path("jbang") { "jbang".to_string() } else { from_nix("jbang", "jbang") };
            return file_command(code_t, How::Stdin, &format!("{jb} -"), "", shell);
        }
        if on_path("java") {
            return file_command(code_t, How::File("java"), "java {f}", "", shell);
        }
    }
    // A runner that takes the code as it is (an argument, its input) before
    // one that needs a file: installed, then from nixpkgs; then the others.
    let nix_ok = on_path("nix");
    let direct = |r: &&Runner| !matches!(r.how, How::File(_));
    let pick = list
        .iter()
        .filter(direct)
        .find(|r| on_path(r.program))
        .map(|r| (r, ""))
        .or_else(|| list.iter().filter(direct).find(|r| nix_ok && !r.nix.is_empty()).map(|r| (r, r.nix)))
        .or_else(|| list.iter().find(|r| on_path(r.program)).map(|r| (r, "")))
        .or_else(|| list.iter().find(|r| nix_ok && !r.nix.is_empty()).map(|r| (r, r.nix)));
    let Some((run, nix)) = pick else { return String::new() };
    // (evcxr runs statements as they come; rust-script -e takes them as a
    // block, a main called at its end.)
    let code = match run.program {
        "evcxr" => code.trim_end().to_string(),
        "rust-script" => rust_expr(code.trim_end()),
        // (yaegi: the standard packages it uses imported; cling: a main called.)
        "yaegi" => go_imports(code.trim_end()),
        "cling" => {
            let c = code.trim_end();
            let c = if c.contains("#include") { c.to_string() } else { format!("#include <iostream>\n#include <string>\n#include <vector>\n#include <map>\n#include <algorithm>\n#include <format>\n#include <print>\n{c}") };
            if c.contains("main(") { format!("{c}\nmain();") } else { c }
        }
        "erl" => { let c = code.trim_end(); if c.ends_with('.') { c.to_string() } else { format!("{c}.") } }
        _ => complete_program(lang, code.trim_end()),
    };
    // (Go that imports modules: a module of its own, tidied, fetches them.)
    let go_mods = lang == "go"
        && code.lines().any(|l| {
            let t = l.trim().trim_start_matches("import").trim().trim_start_matches('(').trim().trim_matches('"');
            t.contains('/') && t.split('/').next().is_some_and(|h| h.contains('.'))
        });
    // (Modules to fetch: go itself, from nixpkgs if need be; yaegi can't.)
    if go_mods {
        let gnix = if on_path("go") { "" } else { "go" };
        let prog = complete_program("go", code.trim_end());
        return file_command(&prog, How::File("go"), "go -C {d} mod init snippet >/dev/null 2>&1; go -C {d} mod tidy && go -C {d} run .", gnix, shell);
    }
    match run.how {
        // ({c} in the command: where the code goes, if not at the end.)
        How::Inline if run.cmd.contains("{c}") => {
            let c = if nix.is_empty() { run.cmd.to_string() } else { from_nix(nix, run.cmd) };
            c.replace("{c}", &quote(&code))
        }
        How::Inline if nix.is_empty() => format!("{} {}", run.cmd, quote(&code)),
        How::Inline => format!("{} {}", from_nix(nix, run.cmd), quote(&code)),
        how => file_command(&code, how, run.cmd, nix, shell),
    }
}

/// Run `code` from a snippet file: `{f}` in `cmd` is the file, `{b}` a
/// binary beside it, `{d}` its folder (How::Stdin: the code on its input).
/// A `nix` package runs it inside `nix shell`. In sh-like shells a
/// here-document on the same line writes the file (the card shows the code).
fn file_command(code: &str, how: How, cmd: &str, nix: &str, shell: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) % 100_000;
    let dir = snippet_dir().join(format!("{n}{}", SEQ.fetch_add(1, Ordering::Relaxed)));
    let ext = if let How::File(e) = how { e } else { "txt" };
    let file = dir.join(format!("snippet.{ext}"));
    let q = |p: &Path| quote(&p.to_string_lossy());
    let (f, b, d) = (q(&file), q(&dir.join("snippet")), q(&dir));
    let mut cmd = cmd.replace("{f}", &f).replace("{b}", &b).replace("{d}", &d);
    if !nix.is_empty() {
        cmd = format!("nix shell nixpkgs#{nix} -c sh -c {}", quote(&cmd));
    }
    let sh_like = !matches!(shell, "fish" | "nu" | "nushell" | "elvish" | "xonsh" | "pwsh");
    if !sh_like {
        // (Written here, run from there.)
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(&file, format!("{code}\n"));
        return if matches!(how, How::Stdin) { format!("{cmd} < {f}") } else { cmd };
    }
    let marker = (0..)
        .map(|i| if i == 0 { "TESSERA_END".to_string() } else { format!("TESSERA_END{i}") })
        .find(|m| !code.lines().any(|l| l == m))
        .unwrap();
    match how {
        How::Stdin => format!("{cmd} <<'{marker}'\n{code}\n{marker}"),
        _ => format!("mkdir -p {d} && cat > {f} <<'{marker}' && {cmd}\n{code}\n{marker}"),
    }
}

/// Packages JavaScript imports (not Node's own modules or relative paths).
fn js_packages(code: &str) -> Vec<String> {
    let mut out = Vec::new();
    for l in code.lines() {
        for key in ["from '", "from \"", "require('", "require(\"", "import('", "import(\"", "import '", "import \""] {
            let mut rest = l;
            while let Some(i) = rest.find(key) {
                rest = &rest[i + key.len()..];
                let name: String = rest.chars().take_while(|c| *c != '\'' && *c != '"').collect();
                if !name.is_empty() && !name.starts_with('.') && !name.starts_with('/') && !name.starts_with("node:") && !name.contains(':') && !NODE_BUILTINS.contains(&name.split('/').next().unwrap_or("")) && !out.contains(&name) {
                    out.push(name);
                }
            }
        }
    }
    out
}

const NODE_BUILTINS: &[&str] = &[
    "fs", "path", "os", "http", "https", "url", "util", "crypto", "child_process", "events", "stream", "buffer", "net", "readline", "zlib", "assert", "process", "timers", "dns", "tls", "worker_threads", "querystring",
];

/// Crates Rust code uses that aren't the standard library's.
fn rust_crates(code: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in code.lines().map(str::trim_start) {
        if l.starts_with(":dep") {
            return Vec::new(); // (It says which itself.)
        }
        let rest = l.strip_prefix("use ").or_else(|| l.strip_prefix("extern crate "));
        let Some(rest) = rest else { continue };
        let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if !name.is_empty() && !["std", "core", "alloc", "crate", "self", "super"].contains(&name.as_str()) && !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// The top-level modules `code` imports.
fn python_imports(code: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in code.lines().map(str::trim_start) {
        let names: Vec<&str> = if let Some(r) = l.strip_prefix("import ") {
            r.split(',').map(|x| x.split_whitespace().next().unwrap_or("")).collect()
        } else if let Some(r) = l.strip_prefix("from ") {
            vec![r.split_whitespace().next().unwrap_or("")]
        } else {
            continue;
        };
        for n in names {
            let top = n.split('.').next().unwrap_or("");
            if !top.is_empty() && top.chars().all(|c| c.is_alphanumeric() || c == '_') && !out.iter().any(|o| o == top) {
                out.push(top.to_string());
            }
        }
    }
    out
}

/// Of the modules `code` imports, those the installed Python can't find.
fn missing_python_modules(code: &str) -> Vec<String> {
    let mods = python_imports(code);
    if mods.is_empty() {
        return mods;
    }
    // (No Python to ask: uv's or Nix's runs it, and an import error says what's missing.)
    let Some(py) = ["python3", "python"].into_iter().find(|p| on_path(p)) else { return Vec::new() };
    let check = "import importlib.util, sys\nfor m in sys.argv[1:]:\n    if importlib.util.find_spec(m) is None: print(m)";
    match std::process::Command::new(py).arg("-c").arg(check).args(&mods).output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

/// The PyPI package with a module (most are named the same).
fn python_package(module: &str) -> &str {
    match module {
        "yaml" => "pyyaml",
        "PIL" => "pillow",
        "sklearn" => "scikit-learn",
        "bs4" => "beautifulsoup4",
        "skimage" => "scikit-image",
        "dateutil" => "python-dateutil",
        "dotenv" => "python-dotenv",
        "Crypto" => "pycryptodome",
        "OpenSSL" => "pyopenssl",
        "serial" => "pyserial",
        "usb" => "pyusb",
        "magic" => "python-magic",
        "telegram" => "python-telegram-bot",
        "jwt" => "pyjwt",
        "git" => "gitpython",
        "attr" => "attrs",
        "cv2" => "opencv-python",
        "google" => "protobuf",
        "zmq" => "pyzmq",
        "docx" => "python-docx",
        "pptx" => "python-pptx",
        "fitz" => "pymupdf",
        "Levenshtein" => "levenshtein",
        "mpl_toolkits" => "matplotlib",
        m => m,
    }
}

/// Doesn't `text` look like a shell command? (It has a syntax error, or
/// starts with a word that isn't a command, a builtin, an alias, a path or
/// a variable.) Only then is the AI asked what it is.
pub fn shell_doubt(text: &str, aliases_json: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.starts_with('#') {
        return false;
    }
    if crate::syntax::has_errors("bash", t) {
        return true;
    }
    let aliases: serde_json::Value = serde_json::from_str(aliases_json).unwrap_or_default();
    // Each line's first word (a pipeline's later commands are the shell's anyway).
    for line in t.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let mut words = line.split_whitespace();
        let mut w = words.next().unwrap_or("");
        // (Assignments before a command: `FOO=1 make`.)
        while is_assignment(w) {
            match words.next() {
                Some(n) => w = n,
                None => {
                    w = "";
                    break;
                }
            }
        }
        // (`name(args)` is a call, not a command: a shell function is `name()`.)
        if let Some(i) = w.find('(') {
            if i > 0 && !w.starts_with('$') && !w[i..].starts_with("()") {
                return true;
            }
            if w.ends_with("()") {
                continue;
            }
        }
        if w.is_empty() || known_command(w, &aliases) {
            continue;
        }
        return true;
    }
    false
}

fn is_assignment(w: &str) -> bool {
    w.split_once('=').is_some_and(|(k, _)| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !k.starts_with(|c: char| c.is_ascii_digit()))
}

const SHELL_WORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac", "function", "select", "time", "coproc", "{", "}", "[[", "]]", "!",
    "cd", "echo", "export", "unset", "set", "source", ".", "alias", "unalias", "exit", "return", "local", "declare", "typeset", "readonly", "read", "eval", "exec",
    "pushd", "popd", "dirs", "jobs", "fg", "bg", "wait", "kill", "trap", "type", "which", "hash", "history", "builtin", "command", "test", "[", "true", "false",
    "printf", "print", "shift", "getopts", "ulimit", "umask", "let", "setopt", "unsetopt", "autoload", "bindkey", "zle", "rehash", "whence", "where", "noglob",
    "nocorrect", "disown", "suspend", "logout", "break", "continue", "shopt", "enable", "complete", "compdef", "emulate", "zmodload", "zstyle", "exec", "env",
];

fn known_command(w: &str, aliases: &serde_json::Value) -> bool {
    if SHELL_WORDS.contains(&w) || aliases.get(w).is_some() {
        return true;
    }
    if w.contains('/') || w.starts_with('~') || w.starts_with('$') || w.starts_with('(') || w.starts_with('\\') {
        return true;
    }
    on_path(w)
}

/// The language to show for typed text: the AI's guesses (`reply`, a JSON
/// list of [name, probability], or a bare name), weighted toward languages
/// used lately (`recent`, a JSON list of names, latest first). "" when it's
/// the shell's, or nothing is likely enough.
pub fn rank(reply: &str, recent_json: &str) -> String {
    let recent: Vec<String> = serde_json::from_str::<Vec<String>>(recent_json).unwrap_or_default().iter().map(|s| normalize(s)).collect();
    let body = reply.trim().trim_start_matches("```json").trim_start_matches("```").trim_end_matches("```").trim();
    let mut guesses: Vec<(String, f64)> = Vec::new();
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(serde_json::Value::Array(items)) => {
            for it in items {
                let (n, p) = match &it {
                    serde_json::Value::Array(a) => (a.first().and_then(|v| v.as_str()).unwrap_or(""), a.get(1).and_then(|v| v.as_f64()).unwrap_or(0.5)),
                    serde_json::Value::Object(o) => (
                        o.get("language").or(o.get("name")).and_then(|v| v.as_str()).unwrap_or(""),
                        o.get("p").or(o.get("probability")).or(o.get("score")).and_then(|v| v.as_f64()).unwrap_or(0.5),
                    ),
                    serde_json::Value::String(s) => (s.as_str(), 0.6),
                    _ => continue,
                };
                if !n.is_empty() {
                    guesses.push((normalize(n), if p > 1.0 { p / 100.0 } else { p }));
                }
            }
        }
        _ => {
            let w = body.split(|c: char| c.is_whitespace() || c == ',').next().unwrap_or("");
            if !w.is_empty() {
                guesses.push((normalize(w), 0.6));
            }
        }
    }
    let best = guesses
        .into_iter()
        .map(|(n, p)| {
            let boost = recent.iter().position(|r| *r == n).map(|i| 0.15 / (1.0 + i as f64)).unwrap_or(0.0);
            (n, p, p * (1.0 + boost))
        })
        .max_by(|a, b| a.2.total_cmp(&b.2));
    match best {
        Some((n, p, _)) if n != "bash" && p >= 0.3 && (runnable(&n) || crate::syntax::is_builtin(&n)) => n,
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubts() {
        assert!(shell_doubt("print(\"test\")", "{}"));
        assert!(shell_doubt("print(1)", "{}"));
        assert!(!shell_doubt("greet() { echo hi; }", "{}"));
        assert!(shell_doubt("x = [i*i for i in range(3)]", "{}"));
        assert!(!shell_doubt("cd /tmp && ls -la | grep foo", "{}"));
        assert!(!shell_doubt("FOO=1 BAR=2 env", "{}"));
        assert!(!shell_doubt("export PATH=$HOME/bin:$PATH", "{}"));
        assert!(!shell_doubt("ll", r#"{"ll":"ls -l"}"#));
    }

    #[test]
    fn ranks() {
        assert_eq!(rank(r#"[["python", 0.8], ["bash", 0.1]]"#, "[]"), "python");
        assert_eq!(rank(r#"[["bash", 0.7], ["python", 0.2]]"#, "[]"), "");
        // (Recent use breaks a near tie, and only that.)
        assert_eq!(rank(r#"[["javascript", 0.45], ["typescript", 0.4]]"#, r#"["typescript"]"#), "typescript");
        assert_eq!(rank("Python", "[]"), "python");
        assert_eq!(rank(r#"[["C++", 0.9]]"#, "[]"), "cpp");
        assert_eq!(rank(r#"[["rust", 0.6], ["java", 0.45]]"#, r#"["java"]"#), "rust");
    }

    #[test]
    fn commands() {
        let c = command("python", "print('hi')", "/", "zsh");
        if !c.is_empty() {
            assert!(c.ends_with("'print('\\''hi'\\'')'"), "{c}");
        }
        assert!(complete_program("go", "fmt.Println(1)").contains("import (\"fmt\")"));
        assert_eq!(command("markdown", "# hi", "/", "zsh"), "");
        let m = command("cpp", "import std;\nstd::println(\"{}\", 1);", "/", "zsh");
        // (clang-repl with Nix; else GCC's modules.)
        assert!(m.is_empty() || m.contains("tessera-cxx") || (m.contains("-fmodules") && m.contains("int main()")), "{m}");
        let h = command("haskell", "main = putStrLn \"hi\"", "/", "zsh");
        assert!(h.is_empty() || (h.contains("runghc") && h.contains("<<'TESSERA_END'")), "{h}");
        let h = command("haskell", "print (sum [1..10])", "/", "zsh");
        assert!(h.is_empty() || h.contains("ghci -v0"), "{h}");
        assert_eq!(python_imports("import os, numpy as np\nfrom PIL.Image import open\nx = 1"), ["os", "numpy", "PIL"]);
        assert_eq!(js_packages("import _ from 'lodash'\nconst fs = require('fs')\nimport x from './x'"), ["lodash"]);
        assert_eq!(rust_crates("use std::io;\nuse serde_json::json;\nprintln!(\"{}\", json!(1));"), ["serde_json"]);
        let c = command("python", "import surely_not_a_module_xyz", "/", "zsh");
        assert!(c.is_empty() || c.contains("--with surely_not_a_module_xyz"), "{c}");
        let g = command("go", "import \"github.com/google/uuid\"\nfunc main() { println(uuid.NewString()) }", "/", "zsh");
        assert!(g.is_empty() || g.contains("mod tidy"), "{g}");
        assert!(missing_python_modules("import os\nimport surely_not_a_module_xyz").contains(&"surely_not_a_module_xyz".to_string()));
    }

    #[test]
    #[ignore]
    fn shells_print() {
        for l in ["fish", "nu", "powershell", "xonsh"] {
            eprintln!("{l}: {}", super::command(l, "echo hi", "/", "zsh"));
        }
    }

}





