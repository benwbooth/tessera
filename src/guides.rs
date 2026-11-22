//! Guides for the editor's languages: each language's own documentation
//! (its tutorial or reference, from its official site), fetched the first
//! time its guide is opened, turned into Markdown and kept in the state
//! folder (`guides/LANG.md`), then shown like the tessh guide: contents,
//! find, highlighted code. Nothing is bundled: the documents keep their own
//! licenses, recorded at the top of each one and in docs/GUIDES.md.

use std::path::PathBuf;

/// Where a guide comes from.
pub struct Guide {
    /// The editor's language name, and other names for the same guide.
    pub langs: &'static [&'static str],
    pub title: &'static str,
    /// Its license, and where the license is stated.
    pub license: &'static str,
    pub license_url: &'static str,
    /// Pages, in order: HTML pages, or Markdown (a `.md` URL).
    pub pages: &'static [&'static str],
    /// Then the pages an index page links to: (index URL, links' CSS selector).
    pub index: Option<(&'static str, &'static str)>,
    /// The part of an HTML page that's the document (CSS selector; "" the body).
    pub select: &'static str,
    /// Parts to leave out (CSS selector, besides scripts, buttons and the like).
    pub drop: &'static str,
    /// The language of code blocks that don't say theirs.
    pub code: &'static str,
    /// Elements that are code drawn some other way (tables of tokens, lines
    /// with <br>): made code blocks of their text (CSS selector).
    pub code_sel: &'static str,
}

const fn g(langs: &'static [&'static str], title: &'static str, license: &'static str, license_url: &'static str, pages: &'static [&'static str], select: &'static str, code: &'static str) -> Guide {
    Guide { langs, title, license, license_url, pages, index: None, select, drop: "", code, code_sel: "" }
}

const fn gi(langs: &'static [&'static str], title: &'static str, license: &'static str, license_url: &'static str, index: (&'static str, &'static str), select: &'static str, code: &'static str) -> Guide {
    Guide { langs, title, license, license_url, pages: &[], index: Some(index), select, drop: "", code, code_sel: "" }
}

impl Guide {
    const fn pages(mut self, p: &'static [&'static str]) -> Self {
        self.pages = p;
        self
    }
    const fn code_sel(mut self, c: &'static str) -> Self {
        self.code_sel = c;
        self
    }
    const fn drop(mut self, d: &'static str) -> Self {
        self.drop = d;
        self
    }
}

const GNU_DROP: &str = ".contents, .shortcontents, .contents-heading, .shortcontents-heading, a.copiable-link, .nav-panel, .header, .navigation, hr, .chapter-toc, .section-toc";

pub static GUIDES: &[Guide] = &[
    gi(&["python", "python3"], "The Python Tutorial", "PSF License 2.0", "https://docs.python.org/3/license.html",
        ("https://docs.python.org/3/tutorial/index.html", "div.toctree-wrapper li.toctree-l1 > a"), "div.body", "python"),
    g(&["rust"], "The Rust Programming Language", "MIT or Apache-2.0", "https://github.com/rust-lang/book/blob/main/LICENSE-MIT",
        &["https://doc.rust-lang.org/book/print.html"], "main", "rust").drop(".boring, .buttons"),
    g(&["bash", "sh"], "Bash Reference Manual", "GNU FDL 1.3", "https://www.gnu.org/licenses/fdl-1.3.html",
        &["https://www.gnu.org/software/bash/manual/bash.html | https://tiswww.case.edu/php/chet/bash/bashref.html"], "body", "bash").drop(GNU_DROP),
    gi(&["zsh"], "A User's Guide to the Z-Shell", "Freely distributable (© Peter Stephenson)", "https://zsh.sourceforge.io/Guide/zshguide.html",
        ("https://zsh.sourceforge.io/Guide/zshguide.html", "a[href^='zshguide0']"), "body", "bash"),
    g(&["fish"], "fish Tutorial", "GPL-2.0", "https://fishshell.com/docs/current/license.html",
        &["https://fishshell.com/docs/current/tutorial.html"], "div.body, div[role=main]", "fish"),
    g(&["lua"], "Lua 5.4 Reference Manual", "MIT (Lua license)", "https://www.lua.org/license.html",
        &["https://www.lua.org/manual/5.4/manual.html"], "body", "lua"),
    g(&["go", "golang"], "Effective Go and the Go Spec", "CC BY 4.0 (code BSD-3-Clause)", "https://go.dev/copyright",
        &["https://go.dev/doc/effective_go", "https://go.dev/ref/spec"], "#main-content article, article, main", "go"),
    g(&["javascript", "node", "js"], "MDN JavaScript Guide", "CC BY-SA 2.5", "https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license", &[
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/introduction/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/grammar_and_types/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/control_flow_and_error_handling/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/loops_and_iteration/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/functions/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/expressions_and_operators/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/numbers_and_strings/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/regular_expressions/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/indexed_collections/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/keyed_collections/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/working_with_objects/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/using_classes/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/using_promises/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/iterators_and_generators/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/modules/index.md",
    ], "", "javascript"),
    g(&["typescript", "tsx", "ts"], "The TypeScript Handbook", "CC BY 4.0", "https://github.com/microsoft/TypeScript-Website/blob/v2/LICENSE", &[
        "https://www.typescriptlang.org/docs/handbook/intro.html",
        "https://www.typescriptlang.org/docs/handbook/2/basic-types.html",
        "https://www.typescriptlang.org/docs/handbook/2/everyday-types.html",
        "https://www.typescriptlang.org/docs/handbook/2/narrowing.html",
        "https://www.typescriptlang.org/docs/handbook/2/functions.html",
        "https://www.typescriptlang.org/docs/handbook/2/objects.html",
        "https://www.typescriptlang.org/docs/handbook/2/generics.html",
        "https://www.typescriptlang.org/docs/handbook/2/keyof-types.html",
        "https://www.typescriptlang.org/docs/handbook/2/typeof-types.html",
        "https://www.typescriptlang.org/docs/handbook/2/conditional-types.html",
        "https://www.typescriptlang.org/docs/handbook/2/mapped-types.html",
        "https://www.typescriptlang.org/docs/handbook/2/classes.html",
        "https://www.typescriptlang.org/docs/handbook/2/modules.html",
    ], "article", "typescript"),
    g(&["c"], "GNU C Language Intro and Reference Manual", "GNU FDL 1.3", "https://www.gnu.org/licenses/fdl-1.3.html",
        &["https://www.gnu.org/software/c-intro-and-ref/manual/c-intro-and-ref.html"], "body", "c").drop(GNU_DROP),
    g(&["cpp", "c++", "cxx"], "C++ language (cppreference.com)", "CC BY-SA 3.0", "https://en.cppreference.com/w/Cppreference:Copyright/CC-BY-SA", &[
        "https://en.cppreference.com/w/cpp/language/basic_concepts",
        "https://en.cppreference.com/w/cpp/language/types",
        "https://en.cppreference.com/w/cpp/language/declarations",
        "https://en.cppreference.com/w/cpp/language/initialization",
        "https://en.cppreference.com/w/cpp/language/expressions",
        "https://en.cppreference.com/w/cpp/language/statements",
        "https://en.cppreference.com/w/cpp/language/functions",
        "https://en.cppreference.com/w/cpp/language/lambda",
        "https://en.cppreference.com/w/cpp/language/classes",
        "https://en.cppreference.com/w/cpp/language/templates",
        "https://en.cppreference.com/w/cpp/language/exceptions",
    ], "#mw-content-text", "cpp").drop(".t-navbar, .editsection, #toc, .printfooter"),
    gi(&["java"], "Learn Java: Language Basics (dev.java)", "© Oracle; read from dev.java, not redistributed", "https://www.oracle.com/legal/copyright.html",
        ("https://dev.java/learn/language/constructs/basics/", "main a[href^='/learn/language/constructs/basics/']"), "main", "java").code_sel("readonly-code-editor")
        .pages(&["https://dev.java/learn/language/constructs/basics/"]),
    gi(&["ruby"], "Ruby Syntax (docs.ruby-lang.org)", "Ruby License or BSD-2-Clause", "https://www.ruby-lang.org/en/about/license.txt",
        ("https://docs.ruby-lang.org/en/master/syntax_rdoc.html", "main a[href^='syntax/']"), "main", "ruby")
        .pages(&["https://www.ruby-lang.org/en/documentation/quickstart/", "https://www.ruby-lang.org/en/documentation/quickstart/2/", "https://www.ruby-lang.org/en/documentation/quickstart/3/", "https://www.ruby-lang.org/en/documentation/quickstart/4/"]),
    g(&["perl"], "Perl documentation (perlintro and friends)", "Artistic License or GPL", "https://perldoc.perl.org/perlartistic", &[
        "https://perldoc.perl.org/perlintro", "https://perldoc.perl.org/perldata", "https://perldoc.perl.org/perlsyn",
        "https://perldoc.perl.org/perlop", "https://perldoc.perl.org/perlsub", "https://perldoc.perl.org/perlreftut",
        "https://perldoc.perl.org/perlretut", "https://perldoc.perl.org/perlootut",
    ], "#perldocdiv, main", "perl").drop("a.permalink, h1#toc, h1#toc + ul, #more, #breadcrumbs"),
    g(&["php"], "PHP Manual: Getting Started and Language Reference", "CC BY 3.0", "https://www.php.net/manual/en/copyright.php", &[
        "https://www.php.net/manual/en/introduction.php", "https://www.php.net/manual/en/tutorial.firstpage.php",
        "https://www.php.net/manual/en/tutorial.useful.php", "https://www.php.net/manual/en/tutorial.forms.php",
        "https://www.php.net/manual/en/language.basic-syntax.php", "https://www.php.net/manual/en/language.types.intro.php",
        "https://www.php.net/manual/en/language.variables.basics.php", "https://www.php.net/manual/en/language.operators.php",
        "https://www.php.net/manual/en/language.control-structures.php", "https://www.php.net/manual/en/functions.user-defined.php",
        "https://www.php.net/manual/en/language.oop5.basic.php",
    ], "#layout-content", "php"),
    g(&["r"], "An Introduction to R", "Verbatim copying permitted (R Core Team)", "https://cran.r-project.org/doc/manuals/r-release/R-intro.html",
        &["https://cran.r-project.org/doc/manuals/r-release/R-intro.html"], "body", "r").drop(GNU_DROP),
    g(&["julia"], "The Julia Manual", "MIT", "https://github.com/JuliaLang/julia/blob/master/LICENSE.md", &[
        "https://docs.julialang.org/en/v1/manual/getting-started/", "https://docs.julialang.org/en/v1/manual/variables/",
        "https://docs.julialang.org/en/v1/manual/integers-and-floating-point-numbers/", "https://docs.julialang.org/en/v1/manual/mathematical-operations/",
        "https://docs.julialang.org/en/v1/manual/strings/", "https://docs.julialang.org/en/v1/manual/functions/",
        "https://docs.julialang.org/en/v1/manual/control-flow/", "https://docs.julialang.org/en/v1/manual/variables-and-scoping/",
        "https://docs.julialang.org/en/v1/manual/types/", "https://docs.julialang.org/en/v1/manual/methods/",
        "https://docs.julialang.org/en/v1/manual/constructors/", "https://docs.julialang.org/en/v1/manual/modules/",
        "https://docs.julialang.org/en/v1/manual/arrays/",
    ], "#documenter-page, article", "julia"),
    g(&["elixir"], "Elixir Getting Started", "Apache-2.0", "https://github.com/elixir-lang/elixir/blob/main/LICENSE", &[
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/introduction.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/basic-types.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/lists-and-tuples.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/pattern-matching.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/case-cond-and-if.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/anonymous-functions.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/binaries-strings-and-charlists.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/keywords-and-maps.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/modules-and-functions.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/recursion.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/enumerable-and-streams.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/processes.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/structs.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/protocols.md",
        "https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/comprehensions.md",
    ], "", "elixir"),
    g(&["erlang"], "Getting Started with Erlang", "Apache-2.0", "https://www.erlang.org/doc/system/license.html", &[
        "https://www.erlang.org/doc/system/getting_started.html", "https://www.erlang.org/doc/system/seq_prog.html",
        "https://www.erlang.org/doc/system/conc_prog.html", "https://www.erlang.org/doc/system/robustness.html",
        "https://www.erlang.org/doc/system/records_macros.html",
    ], "#content, main", "erlang"),
    g(&["groovy"], "Groovy Language Documentation", "Apache-2.0", "https://groovy-lang.org/", &[
        "https://groovy-lang.org/syntax.html", "https://groovy-lang.org/operators.html", "https://groovy-lang.org/objectorientation.html",
        "https://groovy-lang.org/closures.html", "https://groovy-lang.org/semantics.html",
    ], "#content", "groovy").drop("#toc"),
    g(&["powershell", "pwsh"], "PowerShell 101", "CC BY 4.0", "https://github.com/MicrosoftDocs/PowerShell-Docs/blob/main/LICENSE", &[
        "https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/01-getting-started.md",
        "https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/02-help-system.md",
        "https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/03-discovering-objects.md",
        "https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/04-pipelines.md",
        "https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/05-formatting-aliases-providers-comparison.md",
        "https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/06-flow-control.md",
        "https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/09-functions.md",
        "https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/10-script-modules.md",
    ], "", "powershell"),
    g(&["nu", "nushell"], "The Nushell Book", "MIT", "https://github.com/nushell/nushell.github.io/blob/main/LICENSE", &[
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/quick_tour.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/moving_around.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/thinking_in_nu.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/types_of_data.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/pipelines.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/working_with_strings.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/working_with_lists.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/working_with_tables.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/custom_commands.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/variables.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/control_flow.md",
        "https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/scripts.md",
    ], "", "nu"),
    gi(&["tcl"], "Tcl Tutorial", "Tcl/Tk license (BSD-style)", "https://www.tcl-lang.org/software/tcltk/license.html",
        ("https://www.tcl-lang.org/man/tcl8.5/tutorial/tcltutorial.html", "a[href^='Tcl']"), "body", "tcl"),
    g(&["awk"], "GAWK: Effective AWK Programming", "GNU FDL 1.3", "https://www.gnu.org/licenses/fdl-1.3.html",
        &["https://www.gnu.org/software/gawk/manual/gawk.html"], "body", "awk").drop(GNU_DROP),
    g(&["jq"], "jq Manual", "CC BY 3.0 (code MIT)", "https://github.com/jqlang/jq/blob/master/COPYING",
        &["https://jqlang.org/manual/"], "main", "jq"),
    g(&["nix"], "Nix language basics (nix.dev)", "CC BY-SA 4.0", "https://github.com/NixOS/nix.dev/blob/master/LICENSE",
        &["https://nix.dev/tutorials/nix-language"], "article, div[role=main]", "nix"),
    gi(&["octave", "matlab"], "GNU Octave: Getting Started", "Verbatim copying permitted (GNU Octave manual)", "https://docs.octave.org/latest/",
        ("https://docs.octave.org/latest/Getting-Started.html", "ul.mini-toc a, ul.menu a"), "body", "octave").drop(GNU_DROP),
    g(&["sql", "sqlite", "duckdb"], "SQL As Understood By SQLite", "Public domain", "https://www.sqlite.org/copyright.html", &[
        "https://www.sqlite.org/lang_select.html", "https://www.sqlite.org/lang_insert.html", "https://www.sqlite.org/lang_update.html",
        "https://www.sqlite.org/lang_delete.html", "https://www.sqlite.org/lang_createtable.html", "https://www.sqlite.org/lang_expr.html",
        "https://www.sqlite.org/lang_with.html", "https://www.sqlite.org/lang_aggfunc.html", "https://www.sqlite.org/lang_corefunc.html",
        "https://www.sqlite.org/windowfunctions.html", "https://www.sqlite.org/lang_transaction.html",
    ], "div.fancy, body", "sql").drop(".nosearch, .menu, .tooltip, svg"),
    gi(&["haskell", "ghc"], "A Gentle Introduction to Haskell 98", "Free to copy (Hudak, Peterson, Fasel)", "https://www.haskell.org/tutorial/",
        ("https://www.haskell.org/tutorial/", "a[href$='.html']"), "body", "haskell").code_sel("tt:has(br)"),
    g(&["ocaml"], "The OCaml Manual: An Introduction to OCaml", "CC BY-SA 4.0", "https://ocaml.org/manual/latest/", &[
        "https://ocaml.org/manual/latest/coreexamples.html", "https://ocaml.org/manual/latest/moduleexamples.html",
        "https://ocaml.org/manual/latest/objectexamples.html", "https://ocaml.org/manual/latest/lablexamples.html",
        "https://ocaml.org/manual/latest/polymorphism.html",
    ], "div.content, main, body", "ocaml").drop("#part-menu, .navbar, header, footer"),
    gi(&["racket"], "The Racket Guide: Racket Essentials", "MIT or Apache-2.0", "https://download.racket-lang.org/license.html",
        ("https://docs.racket-lang.org/guide/to-scheme.html", "table.toctable a, .toclink"), "div.maincolumn", "racket").drop(".tocset, .navsettop, .navsetbottom, .tocview, .tocsub, .RForeground").code_sel(".SCodeFlow, .SInteraction").pages(&["https://docs.racket-lang.org/guide/to-scheme.html"]),
    g(&["scheme"], "Guile: Hello Guile! and Programming in Scheme", "GNU FDL 1.3", "https://www.gnu.org/licenses/fdl-1.3.html", &[
        "https://www.gnu.org/software/guile/manual/html_node/Hello-Guile_0021.html",
        "https://www.gnu.org/software/guile/manual/html_node/Running-Guile-Interactively.html",
        "https://www.gnu.org/software/guile/manual/html_node/About-Data.html",
        "https://www.gnu.org/software/guile/manual/html_node/About-Procedures.html",
        "https://www.gnu.org/software/guile/manual/html_node/About-Expressions.html",
        "https://www.gnu.org/software/guile/manual/html_node/About-Closure.html",
    ], "body", "scheme").drop(GNU_DROP),
    g(&["common-lisp", "lisp", "commonlisp"], "The Common Lisp Cookbook", "See the Cookbook's license", "https://github.com/LispCookbook/cl-cookbook", &[
        "https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/getting-started.md",
        "https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/data-structures.md",
        "https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/strings.md",
        "https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/functions.md",
        "https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/iteration.md",
        "https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/clos.md",
        "https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/error_handling.md",
    ], "", "common-lisp"),
    g(&["dart"], "Dart Language Tour", "CC BY 4.0 (code BSD-3-Clause)", "https://dart.dev/terms", &[
        "https://dart.dev/language", "https://dart.dev/language/variables", "https://dart.dev/language/operators",
        "https://dart.dev/language/built-in-types", "https://dart.dev/language/records", "https://dart.dev/language/collections",
        "https://dart.dev/language/functions", "https://dart.dev/language/loops", "https://dart.dev/language/branches",
        "https://dart.dev/language/classes", "https://dart.dev/language/async",
    ], "main article, main", "dart"),
    g(&["swift"], "The Swift Programming Language", "Apache-2.0 with Runtime Library Exception", "https://github.com/swiftlang/swift-book/blob/main/LICENSE.txt", &[
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/GuidedTour/GuidedTour.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/TheBasics.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/BasicOperators.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/StringsAndCharacters.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/CollectionTypes.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/ControlFlow.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/Functions.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/Closures.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/Enumerations.md",
        "https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/LanguageGuide/ClassesAndStructures.md",
    ], "", "swift"),
    g(&["zig"], "Zig Language Reference", "MIT", "https://github.com/ziglang/zig/blob/master/LICENSE",
        &["https://ziglang.org/documentation/master/"], "#contents, main", "zig").drop("#toc, #navigation"),
    g(&["nim"], "Nim Tutorial", "MIT", "https://github.com/nim-lang/Nim/blob/devel/copying.txt", &[
        "https://nim-lang.org/docs/tut1.html", "https://nim-lang.org/docs/tut2.html", "https://nim-lang.org/docs/tut3.html",
    ], "#content, .container", "nim").drop("#global-links, .twelve-columns.theme-select-wrapper, #searchInputDiv, .three.columns"),
    g(&["crystal"], "Crystal Tutorials: Language Introduction", "Apache-2.0", "https://github.com/crystal-lang/crystal-book/blob/master/LICENSE", &[
        "https://raw.githubusercontent.com/crystal-lang/crystal-book/master/docs/tutorials/basics/10_hello_world.md",
        "https://raw.githubusercontent.com/crystal-lang/crystal-book/master/docs/tutorials/basics/20_variables.md",
        "https://raw.githubusercontent.com/crystal-lang/crystal-book/master/docs/tutorials/basics/30_math.md",
        "https://raw.githubusercontent.com/crystal-lang/crystal-book/master/docs/tutorials/basics/40_strings.md",
        "https://raw.githubusercontent.com/crystal-lang/crystal-book/master/docs/tutorials/basics/50_control_flow.md",
        "https://raw.githubusercontent.com/crystal-lang/crystal-book/master/docs/tutorials/basics/60_methods.md",
    ], "", "crystal"),
    g(&["v"], "V Documentation", "MIT", "https://github.com/vlang/v/blob/master/LICENSE",
        &["https://raw.githubusercontent.com/vlang/v/master/doc/docs.md"], "", "v"),
    g(&["fsharp", "f#"], "Tour of F#", "CC BY 4.0", "https://github.com/dotnet/docs/blob/main/LICENSE",
        &["https://raw.githubusercontent.com/dotnet/docs/main/docs/fsharp/tour.md"], "", "fsharp"),
    g(&["c-sharp", "csharp", "c#", "cs"], "A tour of the C# language", "CC BY 4.0", "https://github.com/dotnet/docs/blob/main/LICENSE", &[
        "https://raw.githubusercontent.com/dotnet/docs/main/docs/csharp/tour-of-csharp/overview.md",
        "https://raw.githubusercontent.com/dotnet/docs/main/docs/csharp/tour-of-csharp/strategy.md",
    ], "", "c-sharp"),
    g(&["kotlin"], "Kotlin Tour", "Apache-2.0", "https://github.com/JetBrains/kotlin-web-site/blob/master/LICENSE", &[
        "https://raw.githubusercontent.com/JetBrains/kotlin-web-site/master/docs/topics/tour/kotlin-tour-hello-world.md",
        "https://raw.githubusercontent.com/JetBrains/kotlin-web-site/master/docs/topics/tour/kotlin-tour-basic-types.md",
        "https://raw.githubusercontent.com/JetBrains/kotlin-web-site/master/docs/topics/tour/kotlin-tour-collections.md",
        "https://raw.githubusercontent.com/JetBrains/kotlin-web-site/master/docs/topics/tour/kotlin-tour-control-flow.md",
        "https://raw.githubusercontent.com/JetBrains/kotlin-web-site/master/docs/topics/tour/kotlin-tour-functions.md",
        "https://raw.githubusercontent.com/JetBrains/kotlin-web-site/master/docs/topics/tour/kotlin-tour-classes.md",
        "https://raw.githubusercontent.com/JetBrains/kotlin-web-site/master/docs/topics/tour/kotlin-tour-null-safety.md",
    ], "", "kotlin"),
    g(&["scala"], "Scala 3 Book", "CC BY-SA 4.0", "https://github.com/scala/docs.scala-lang/blob/main/LICENSE.md", &[
        "https://docs.scala-lang.org/scala3/book/introduction.html", "https://docs.scala-lang.org/scala3/book/taste-intro.html",
        "https://docs.scala-lang.org/scala3/book/taste-hello-world.html", "https://docs.scala-lang.org/scala3/book/taste-vars-data-types.html",
        "https://docs.scala-lang.org/scala3/book/taste-control-structures.html", "https://docs.scala-lang.org/scala3/book/taste-modeling.html",
        "https://docs.scala-lang.org/scala3/book/taste-methods.html", "https://docs.scala-lang.org/scala3/book/taste-functions.html",
        "https://docs.scala-lang.org/scala3/book/taste-objects.html", "https://docs.scala-lang.org/scala3/book/taste-collections.html",
    ], ".content-primary, main", "scala").drop(".tabsection > .nav-tab, .tabs-header"),
    g(&["clojure"], "Learn Clojure (clojure.org)", "CC BY-SA 3.0", "https://clojure.org/community/license", &[
        "https://clojure.org/guides/learn/clojure", "https://clojure.org/guides/learn/syntax", "https://clojure.org/guides/learn/functions",
        "https://clojure.org/guides/learn/sequential_colls", "https://clojure.org/guides/learn/hashed_colls", "https://clojure.org/guides/learn/flow",
        "https://clojure.org/guides/learn/namespaces",
    ], ".clj-content-container, main", "clojure").drop("#toc, .clj-prev-next-container"),
    g(&["prolog"], "SWI-Prolog Manual: Getting started", "BSD-2-Clause", "https://www.swi-prolog.org/license.html", &[
        "https://www.swi-prolog.org/pldoc/man?section=quickstart", "https://www.swi-prolog.org/pldoc/man?section=syntax",
    ], ".manual, #contents, body", "prolog").drop(".navhdr, #header, #footer, .menubar"),
    g(&["fortran"], "Fortran Quickstart Tutorial (fortran-lang.org)", "MIT", "https://github.com/fortran-lang/webpage/blob/main/LICENSE", &[
        "https://raw.githubusercontent.com/fortran-lang/webpage/main/source/learn/quickstart/hello_world.md",
        "https://raw.githubusercontent.com/fortran-lang/webpage/main/source/learn/quickstart/variables.md",
        "https://raw.githubusercontent.com/fortran-lang/webpage/main/source/learn/quickstart/arrays_strings.md",
        "https://raw.githubusercontent.com/fortran-lang/webpage/main/source/learn/quickstart/operators_control_flow.md",
        "https://raw.githubusercontent.com/fortran-lang/webpage/main/source/learn/quickstart/organising_code.md",
        "https://raw.githubusercontent.com/fortran-lang/webpage/main/source/learn/quickstart/derived_types.md",
    ], "", "fortran"),
    g(&["elvish"], "Tour of Elvish", "BSD-2-Clause", "https://github.com/elves/elvish/blob/master/LICENSE",
        &["https://elv.sh/learn/tour.html"], "article, main", "elvish"),
    g(&["fennel"], "Fennel Tutorial", "MIT", "https://git.sr.ht/~technomancy/fennel/tree/main/item/LICENSE",
        &["https://fennel-lang.org/tutorial"], "main, body", "fennel"),
    gi(&["forth"], "Gforth Tutorial", "GNU FDL 1.3", "https://www.gnu.org/licenses/fdl-1.3.html",
        ("https://www.complang.tuwien.ac.at/forth/gforth/Docs-html/Tutorial.html", "ul.mini-toc a, ul.menu a"), "body", "forth").drop(GNU_DROP),
    g(&["hare"], "Hare: An introduction", "CC BY-ND 4.0", "https://harelang.org/", &["https://harelang.org/tutorials/introduction"], "main", "hare"),
    gi(&["janet", "janet-simple"], "Janet Documentation", "MIT", "https://github.com/janet-lang/janet/blob/master/LICENSE",
        ("https://janet-lang.org/docs/index.html", "nav a[href^='/docs/'], .toc a[href^='/docs/']"), "main, .main", "janet").pages(&["https://janet-lang.org/docs/index.html"]),
    g(&["koka"], "The Koka Programming Language", "Apache-2.0", "https://github.com/koka-lang/koka/blob/master/LICENSE",
        &["https://koka-lang.github.io/koka/doc/book.html"], "body", "koka").drop(".toc, .sidepanel, #sidetoc"),
    gi(&["lean"], "Functional Programming in Lean", "CC BY 4.0", "https://lean-lang.org/functional_programming_in_lean/",
        ("https://lean-lang.org/functional_programming_in_lean/", "a[href*='/#']"), "main", "lean"),
    g(&["luau"], "Luau: Syntax and Types", "MIT", "https://github.com/luau-lang/site/blob/master/LICENSE", &[
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/getting-started/intro.md",
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/getting-started/syntax.md",
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/types/overview.md",
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/types/basic-types.md",
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/types/tables.md",
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/types/generics.md",
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/types/unions-and-intersections.md",
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/types/refinements.md",
        "https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/types/object-oriented-programs.md",
    ], "", "luau"),
    g(&["odin"], "Odin Overview", "BSD-3-Clause", "https://github.com/odin-lang/Odin/blob/master/LICENSE",
        &["https://odin-lang.org/docs/overview/"], "main, article", "odin").drop("#TableOfContents, nav"),
    gi(&["pascal"], "Free Pascal Reference Guide", "Free Pascal documentation license (free to copy)", "https://www.freepascal.org/docs.html",
        ("https://www.freepascal.org/docs-html/ref/ref.html", "a[href^='refch'], a[href^='refse']"), "body", "pascal").code_sel("div.verbatim"),
    gi(&["ada"], "Introduction to Ada (learn.adacore.com)", "CC BY-SA 4.0", "https://github.com/AdaCore/learn/blob/main/LICENSE",
        ("https://learn.adacore.com/courses/intro-to-ada/index.html", "div.toctree-wrapper li.toctree-l1 > a"), "div[role=main], div.body", "ada"),
    g(&["gleam"], "The Gleam Language Tour", "Apache-2.0", "https://github.com/gleam-lang/language-tour/blob/main/LICENCE",
        &["https://tour.gleam.run/everything/"], "main, body", "gleam"),
    g(&["koto"], "The Koto Language Guide", "MIT", "https://github.com/koto-lang/koto/blob/main/LICENSE",
        &["https://koto.dev/docs/next/language/"], "main, article", "koto"),
    gi(&["inko"], "The Inko Manual", "MPL-2.0", "https://github.com/inko-lang/inko/blob/main/LICENSE",
        ("https://docs.inko-lang.org/manual/latest/", "a[href^='getting-started/'], a[href^='references/syntax'], a[href^='references/']"), "main, article", "inko"),
    gi(&["amber"], "Amber Documentation", "GPL-3.0", "https://github.com/amber-lang/amber/blob/master/LICENSE",
        ("https://docs.amber-lang.com/", "a[href^='/getting_started'], a[href^='/basic_syntax'], a[href^='/advanced_syntax']"), "main, article", "amber"),
    g(&["uxntal"], "Uxntal (XXIIVV wiki)", "CC BY-NC-SA 4.0; read from the wiki, not redistributed", "https://wiki.xxiivv.com/site/about.html",
        &["https://wiki.xxiivv.com/site/uxntal.html", "https://wiki.xxiivv.com/site/uxntal_syntax.html", "https://wiki.xxiivv.com/site/uxntal_opcodes.html"], "main", "uxntal"),
    g(&["nasm"], "NASM Manual: The NASM Language", "BSD-2-Clause", "https://www.nasm.us/doc/nasmdoc1.html",
        &["https://www.nasm.us/doc/nasmdoc2.html", "https://www.nasm.us/doc/nasmdoc3.html"], "body", "nasm").drop(".navbar, #navbar, .nav, ul.nav"),
    g(&["gas"], "Using as: Syntax", "GNU FDL 1.3", "https://www.gnu.org/licenses/fdl-1.3.html", &[
        "https://sourceware.org/binutils/docs/as/Syntax.html", "https://sourceware.org/binutils/docs/as/Preprocessing.html",
        "https://sourceware.org/binutils/docs/as/Whitespace.html", "https://sourceware.org/binutils/docs/as/Comments.html",
        "https://sourceware.org/binutils/docs/as/Symbol-Intro.html", "https://sourceware.org/binutils/docs/as/Statements.html",
        "https://sourceware.org/binutils/docs/as/Constants.html", "https://sourceware.org/binutils/docs/as/Sections.html",
        "https://sourceware.org/binutils/docs/as/Symbols.html", "https://sourceware.org/binutils/docs/as/Expressions.html",
    ], "body", "gas").drop(GNU_DROP),
    g(&["llvm"], "LLVM Language Reference Manual", "Apache-2.0 with LLVM Exception", "https://llvm.org/LICENSE.txt",
        &["https://llvm.org/docs/LangRef.html"], "div[role=main], div.body", "llvm").drop("#contents, .contents"),
    g(&["elisp"], "An Introduction to Programming in Emacs Lisp", "GNU FDL 1.3", "https://www.gnu.org/licenses/fdl-1.3.html",
        &["https://www.gnu.org/software/emacs/manual/html_mono/eintr.html"], "body", "elisp").drop(GNU_DROP),
    g(&["jsonnet"], "Jsonnet Tutorial", "Apache-2.0", "https://github.com/google/jsonnet/blob/master/LICENSE",
        &["https://jsonnet.org/learning/tutorial.html"], "body", "jsonnet").drop(".navbar, #navbar, .menu, .tab-header, .bigarrow, .tab-window-output, nav, footer, #header").code_sel(".tab-window-input"),
    g(&["nickel"], "The Nickel User Manual", "MIT", "https://github.com/tweag/nickel/blob/master/LICENSE", &[
        "https://raw.githubusercontent.com/tweag/nickel/master/doc/manual/introduction.md",
        "https://raw.githubusercontent.com/tweag/nickel/master/doc/manual/tutorial.md",
        "https://raw.githubusercontent.com/tweag/nickel/master/doc/manual/syntax.md",
        "https://raw.githubusercontent.com/tweag/nickel/master/doc/manual/merging.md",
        "https://raw.githubusercontent.com/tweag/nickel/master/doc/manual/contracts.md",
    ], "", "nickel"),
    g(&["dhall"], "Dhall Language Tour", "BSD-3-Clause", "https://github.com/dhall-lang/dhall-lang/blob/master/LICENSE",
        &["https://raw.githubusercontent.com/dhall-lang/dhall-lang/master/docs/tutorials/Language-Tour.md"], "", "dhall"),
    g(&["cue"], "A Tour of CUE", "Apache-2.0", "https://github.com/cue-lang/cuelang.org/blob/master/LICENSE", &[
        "https://cuelang.org/docs/tour/basics/json-superset/",
        "https://cuelang.org/docs/tour/basics/types-are-values/",
        "https://cuelang.org/docs/tour/basics/unification/",
        "https://cuelang.org/docs/tour/basics/constraints/",
        "https://cuelang.org/docs/tour/basics/definitions/",
        "https://cuelang.org/docs/tour/basics/validation/",
        "https://cuelang.org/docs/tour/basics/order-irrelevance/",
        "https://cuelang.org/docs/tour/basics/folding-structs/",
        "https://cuelang.org/docs/tour/types/types/",
        "https://cuelang.org/docs/tour/types/bottom/",
        "https://cuelang.org/docs/tour/types/top/",
        "https://cuelang.org/docs/tour/types/numbers/",
        "https://cuelang.org/docs/tour/types/stringlit/",
        "https://cuelang.org/docs/tour/types/stringraw/",
        "https://cuelang.org/docs/tour/types/bytes/",
        "https://cuelang.org/docs/tour/types/structs/",
        "https://cuelang.org/docs/tour/types/closed/",
        "https://cuelang.org/docs/tour/types/definitions/",
        "https://cuelang.org/docs/tour/types/value-constraints/",
        "https://cuelang.org/docs/tour/types/disjunctions/",
        "https://cuelang.org/docs/tour/types/defaults/",
        "https://cuelang.org/docs/tour/types/sumstruct/",
        "https://cuelang.org/docs/tour/types/bounds/",
        "https://cuelang.org/docs/tour/types/bounddef/",
        "https://cuelang.org/docs/tour/types/lists/",
        "https://cuelang.org/docs/tour/types/templates/",
        "https://cuelang.org/docs/tour/references/scopes/",
        "https://cuelang.org/docs/tour/references/selectors/",
        "https://cuelang.org/docs/tour/references/aliases/",
        "https://cuelang.org/docs/tour/references/emit/",
        "https://cuelang.org/docs/tour/references/cycle/",
        "https://cuelang.org/docs/tour/references/cycleref/",
        "https://cuelang.org/docs/tour/references/hidden/",
        "https://cuelang.org/docs/tour/expressions/operators/",
        "https://cuelang.org/docs/tour/expressions/interpolation/",
        "https://cuelang.org/docs/tour/expressions/dynamic-fields/",
        "https://cuelang.org/docs/tour/expressions/conditional/",
        "https://cuelang.org/docs/tour/expressions/listcomp/",
        "https://cuelang.org/docs/tour/expressions/fieldcomp/",
        "https://cuelang.org/docs/tour/expressions/query-projection/",
        "https://cuelang.org/docs/tour/expressions/regexp/",
        "https://cuelang.org/docs/tour/expressions/coalesce/",
        "https://cuelang.org/docs/tour/packages/packages/",
        "https://cuelang.org/docs/tour/packages/imports/",
        "https://cuelang.org/docs/tour/packages/standard-library/",
    ], "main article, main", "cue"),
    gi(&["pkl"], "Pkl Language Tutorial", "Apache-2.0", "https://github.com/apple/pkl/blob/main/LICENSE.txt",
        ("https://pkl-lang.org/main/current/language-tutorial/index.html", "article a[href$='.html']"), "article.doc, article", "pkl"),
    g(&["rego"], "Rego: Policy Language", "Apache-2.0", "https://github.com/open-policy-agent/opa/blob/main/LICENSE",
        &["https://www.openpolicyagent.org/docs/policy-language"], "article, main", "rego"),
    g(&["d"], "The D Language Tour: D's Basics", "Boost-1.0 (D Language Tour)", "https://github.com/dlang-tour/core/blob/master/LICENSE.txt", &[
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/imports-and-modules.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/basic-types.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/memory.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/type-qualifiers.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/controlling-flow.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/functions.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/structs.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/arrays.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/slices.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/alias-strings.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/loops.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/foreach.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/ranges.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/associative-arrays.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/classes.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/interfaces.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/templates.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/delegates.md",
        "https://raw.githubusercontent.com/dlang-tour/english/master/basics/exceptions.md",
    ], "", "d"),
    g(&["sml"], "SML Help (Standard ML)", "© the SML Help authors; read from the site, not redistributed", "https://github.com/smlhelp/book", &[
        "https://smlhelp.github.io/book/docs/start/install/", "https://smlhelp.github.io/book/docs/types/",
        "https://smlhelp.github.io/book/docs/concepts/basic/", "https://smlhelp.github.io/book/docs/examples/basics/",
        "https://smlhelp.github.io/book/docs/debugging/errors/", "https://smlhelp.github.io/book/docs/libs/basis/",
    ], "article, main", "sml"),
    g(&["prql"], "The PRQL Language Book", "Apache-2.0", "https://github.com/PRQL/prql/blob/main/LICENSE",
        &["https://prql-lang.org/book/print.html"], "main", "prql").drop(".buttons"),
    g(&["scss", "sass"], "Sass Basics", "MIT", "https://github.com/sass/sass-site/blob/main/LICENSE",
        &["https://sass-lang.com/guide/"], "main", "scss").drop(".sl-c-callout--impl-status, nav, .sl-l-medium-holy-grail__navigation"),
    g(&["make"], "GNU make Manual", "GNU FDL 1.3", "https://www.gnu.org/licenses/fdl-1.3.html",
        &["https://www.gnu.org/software/make/manual/make.html"], "body", "make").drop(GNU_DROP),
    gi(&["cmake"], "CMake Tutorial", "BSD-3-Clause", "https://cmake.org/licensing/",
        ("https://cmake.org/cmake/help/latest/guide/tutorial/index.html", "div.toctree-wrapper a"), "div[role=main], div.body", "cmake"),
    g(&["typst"], "Typst Tutorial", "Apache-2.0", "https://github.com/typst/typst/blob/main/LICENSE", &[
        "https://typst.app/docs/tutorial/", "https://typst.app/docs/tutorial/writing-in-typst/", "https://typst.app/docs/tutorial/formatting/",
        "https://typst.app/docs/tutorial/advanced-styling/", "https://typst.app/docs/tutorial/making-a-template/",
    ], "main", "typst").drop("nav, .page-nav, .previews"),
    g(&["dot", "graphviz"], "The DOT Language (Graphviz)", "EPL-2.0", "https://graphviz.org/license/", &[
        "https://graphviz.org/doc/info/lang.html", "https://graphviz.org/doc/info/attrs.html", "https://graphviz.org/doc/info/shapes.html",
    ], "main .td-content, main", "dot").drop(".td-page-meta, .td-toc, nav"),
    g(&["latex", "tex"], "Learn LaTeX", "CC BY-SA 4.0", "https://github.com/learnlatex/learnlatex.github.io/blob/main/LICENSE.md", &[
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-01.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-02.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-03.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-04.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-05.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-06.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-07.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-08.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-09.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-10.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-11.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-12.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-13.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-14.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-15.md",
        "https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-16.md",
    ], "", "latex"),
    g(&["openscad"], "The OpenSCAD Language (OpenSCAD User Manual)", "CC BY-SA 4.0 (Wikibooks)", "https://en.wikibooks.org/wiki/OpenSCAD_User_Manual",
        &["https://en.wikibooks.org/wiki/OpenSCAD_User_Manual/The_OpenSCAD_Language"], "#mw-content-text", "openscad")
        .drop(".mw-editsection, .toc, #toc, .navbox, .noprint, .mw-references-wrap"),
    g(&["json"], "Introducing JSON (json.org)", "© JSON.org; read from json.org, not redistributed", "https://www.json.org/json-en.html",
        &["https://www.json.org/json-en.html"], "body", "json"),
    g(&["yaml"], "YAML 1.2.2 Specification", "Free to copy (the YAML spec's own terms)", "https://yaml.org/spec/1.2.2/",
        &["https://yaml.org/spec/1.2.2/"], "main, article, body", "yaml").drop("#toc, #markdown-toc, nav"),
    g(&["toml"], "TOML v1.0.0", "MIT", "https://github.com/toml-lang/toml/blob/main/LICENSE",
        &["https://raw.githubusercontent.com/toml-lang/toml/1.0.0/toml.md"], "", "toml"),
    g(&["html"], "MDN: Structuring content with HTML", "CC BY-SA 2.5", "https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license", &[
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/basic_html_syntax/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/webpage_metadata/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/headings_and_paragraphs/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/lists/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/structuring_documents/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/creating_links/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/html_images/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/html_forms/index.md",
    ], "", "html"),
    g(&["css"], "MDN: CSS styling basics", "CC BY-SA 2.5", "https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license", &[
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/styling_basics/what_is_css/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/styling_basics/getting_started/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/styling_basics/basic_selectors/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/styling_basics/box_model/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/styling_basics/handling_conflicts/index.md",
        "https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/styling_basics/values_and_units/index.md",
    ], "", "css"),
    g(&["markdown"], "CommonMark: Markdown Reference", "CC BY-SA 4.0", "https://commonmark.org/help/",
        &["https://commonmark.org/help/"], "body", "markdown").drop(".navbar, nav, header, footer"),
];

/// The guide for a language (its name in the editor), if there is one.
pub fn find(lang: &str) -> Option<&'static Guide> {
    GUIDES.iter().find(|g| g.langs.contains(&lang))
}

/// Guides there are (JSON {lang: title}), tessh's own included.
pub fn list_json() -> String {
    let mut m = serde_json::Map::new();
    m.insert("tessh".into(), "tessh guide".into());
    for g in GUIDES {
        for l in g.langs {
            m.insert(l.to_string(), g.title.into());
        }
    }
    serde_json::Value::Object(m).to_string()
}

fn cache_dir() -> PathBuf {
    crate::settings::state_dir().join("guides")
}

fn cache_path(g: &Guide) -> PathBuf {
    cache_dir().join(format!("{}.md", g.langs[0]))
}

/// A guide as a page (a <nav> and a <main>, the tessh guide's way), fetched
/// and converted first if it isn't kept yet (or `refresh`). Slow: call it in
/// the background.
pub fn page_html(lang: &str, refresh: bool) -> Result<String, String> {
    if lang == "tessh" {
        return Ok(crate::cells::tessh_manual_html());
    }
    let g = find(lang).ok_or_else(|| format!("No guide for {lang}."))?;
    let r = page_html_of(g, refresh);
    set_progress(g.langs[0], "");
    r
}

/// What a guide being fetched is doing now ("" when nothing): for the page
/// to show while it waits.
pub fn progress(lang: &str) -> String {
    let key = find(lang).map(|g| g.langs[0]).unwrap_or(lang);
    progress_map().lock().unwrap().get(key).cloned().unwrap_or_default()
}

fn progress_map() -> &'static std::sync::Mutex<std::collections::HashMap<String, String>> {
    static P: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, String>>> = std::sync::OnceLock::new();
    P.get_or_init(Default::default)
}

fn set_progress(key: &str, what: &str) {
    let mut m = progress_map().lock().unwrap();
    if what.is_empty() {
        m.remove(key);
    } else {
        m.insert(key.to_string(), what.to_string());
    }
}

/// The drawn page is kept too (`LANG.body.html`, marked with the drawing's
/// version), so a guide opens at once the second time.
const RENDER_VERSION: &str = "<!-- tessera-guide-render 3 -->";
/// Marks the kept Markdown: a guide converted by an older Tessera is
/// fetched and converted again.
const CONVERT_VERSION: &str = "<!-- tessera-guide-convert 2 -->";

fn page_html_of(g: &Guide, refresh: bool) -> Result<String, String> {
    let path = cache_path(g);
    let body_path = path.with_extension("body.html");
    if !refresh {
        let newer = |a: &std::path::Path, b: &std::path::Path| match (std::fs::metadata(a).and_then(|m| m.modified()), std::fs::metadata(b).and_then(|m| m.modified())) {
            (Ok(x), Ok(y)) => x >= y,
            _ => false,
        };
        if newer(&body_path, &path) {
            if let Ok(b) = std::fs::read_to_string(&body_path) {
                if let Some(body) = b.strip_prefix(RENDER_VERSION).and_then(|b| b.strip_prefix(CONVERT_VERSION)) {
                    return Ok(body.to_string());
                }
            }
        }
    }
    let kept = std::fs::read_to_string(&path).ok();
    let md = match kept {
        Some(md) if !refresh && md.starts_with(CONVERT_VERSION) => md,
        _ => {
            // (A copy from an older conversion is fetched again, but still
            // shown when the site can't be reached.)
            let md = match fetch_markdown(g) {
                Ok(md) => format!("{CONVERT_VERSION}\n{md}"),
                Err(e) => match kept {
                    Some(old) if !refresh => old,
                    _ => return Err(e),
                },
            };
            let _ = std::fs::create_dir_all(cache_dir());
            let tmp = path.with_extension("md.part");
            if std::fs::write(&tmp, &md).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
            md
        }
    };
    // (The grammars its code is in, unpacked first if need be, as the
    // editor does: the guide's language and those its code blocks name.)
    let mut langs: Vec<String> = vec![g.code.to_string()];
    for l in md.lines().filter_map(|l| l.strip_prefix("```")) {
        let l = crate::manual::code_lang(l, "");
        if !l.is_empty() && !langs.contains(&l) && langs.len() < 6 {
            langs.push(l);
        }
    }
    let mut grammars_ok = true;
    for l in &langs {
        if !crate::syntax::is_builtin(l) && crate::grammars::is_known(l) {
            set_progress(g.langs[0], &format!("Getting the {l} grammar…"));
            grammars_ok &= crate::grammars::ensure(l).is_ok();
        }
    }
    set_progress(g.langs[0], "Drawing the page…");
    let body = crate::manual::manual_html(&md, g.code);
    let tmp = body_path.with_extension("part");
    // (Kept only for this conversion: an older one is redrawn after it's fetched again.)
    // (Not kept when a grammar couldn't be had: it's drawn again next time.)
    let mark = if md.starts_with(CONVERT_VERSION) { CONVERT_VERSION } else { "" };
    if grammars_ok && std::fs::write(&tmp, format!("{RENDER_VERSION}{mark}{body}")).is_ok() {
        let _ = std::fs::rename(&tmp, &body_path);
    }
    Ok(body)
}

/// A guide's page written out whole (`head`: its style and script), for
/// the guide tab to load: its file URL.
pub fn page_file(lang: &str, refresh: bool, head: &str) -> Result<String, String> {
    let body = page_html(lang, refresh)?;
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let name: String = lang.chars().filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_').collect();
    let path = dir.join(format!("{name}.page.html"));
    std::fs::write(&path, format!("<!doctype html><html><head><meta charset='utf-8'>{head}</head><body>{body}</body></html>")).map_err(|e| e.to_string())?;
    let p = path.to_string_lossy().replace('%', "%25").replace(' ', "%20").replace('#', "%23").replace('?', "%3F");
    Ok(if p.starts_with('/') { format!("file://{p}") } else { format!("file:///{}", p.replace('\\', "/")) })
}

/// Today's date (UTC), for the note at the top of a guide.
fn today() -> String {
    let days = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86400).unwrap_or(0) as i64;
    // (Civil from days, Howard Hinnant's way.)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

/// A page from whichever of its addresses answers first ("A | B": B is
/// another copy of the same page; both are asked at once, so a site that
/// doesn't answer costs no time).
fn get_any(urls: &str) -> Result<String, String> {
    let urls: Vec<String> = urls.split(" | ").map(|u| u.trim().to_string()).collect();
    if urls.len() == 1 {
        return get(&urls[0]);
    }
    let (tx, rx) = std::sync::mpsc::channel();
    for u in urls.clone() {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(get(&u));
        });
    }
    drop(tx);
    let mut errs = vec![];
    for r in rx {
        match r {
            Ok(t) => return Ok(t),
            Err(e) => errs.push(e),
        }
    }
    Err(errs.join("; "))
}

fn get(url: &str) -> Result<String, String> {
    use ureq::config::IpFamily;
    let fetch = |family: IpFamily| -> Result<Vec<u8>, ureq::Error> {
        let agent: ureq::Agent = ureq::Agent::config_builder().ip_family(family).timeout_connect(Some(std::time::Duration::from_secs(8))).timeout_global(Some(std::time::Duration::from_secs(90))).build().into();
        let mut r = agent.get(url).header("User-Agent", concat!("Tessera/", env!("CARGO_PKG_VERSION"), " (language guide)")).call()?;
        r.body_mut().with_config().limit(32 << 20).read_to_vec()
    };
    // (A host with an IPv6 address that can't be reached from here: again
    // over IPv4.)
    let bytes = match fetch(IpFamily::Any) {
        Ok(b) => b,
        Err(ureq::Error::Io(_)) | Err(ureq::Error::ConnectionFailed) | Err(ureq::Error::HostNotFound) | Err(ureq::Error::Timeout(_)) => fetch(IpFamily::Ipv4Only).map_err(|e| format!("{url}: {e}"))?,
        Err(e) => return Err(format!("{url}: {e}")),
    };
    Ok(match String::from_utf8(bytes) {
        Ok(s) => s,
        // (Older pages in Latin-1.)
        Err(e) => e.into_bytes().iter().map(|&b| b as char).collect(),
    })
}

/// An address relative to a page, made whole.
pub fn join(base: &str, href: &str) -> String {
    if href.contains("://") || href.starts_with("mailto:") || href.starts_with("data:") {
        return href.to_string();
    }
    let scheme_end = base.find("://").map(|i| i + 3).unwrap_or(0);
    let origin_end = base[scheme_end..].find('/').map(|i| i + scheme_end).unwrap_or(base.len());
    if let Some(rest) = href.strip_prefix("//") {
        return format!("{}{rest}", &base[..scheme_end]);
    }
    if href.starts_with('/') {
        return format!("{}{href}", &base[..origin_end]);
    }
    let page = base.split(['#', '?']).next().unwrap_or(base);
    if href.starts_with('#') {
        return format!("{page}{href}");
    }
    if href.starts_with('?') {
        return format!("{page}{href}");
    }
    let dir = &page[..page.rfind('/').map(|i| i + 1).unwrap_or(page.len()).max(origin_end.min(page.len()))];
    let mut parts: Vec<&str> = dir[origin_end.min(dir.len())..].split('/').filter(|s| !s.is_empty()).collect();
    let (path, tail) = match href.find(['#', '?']) {
        Some(i) => (&href[..i], &href[i..]),
        None => (href, ""),
    };
    let segs: Vec<&str> = path.split('/').collect();
    for s in &segs {
        match *s {
            "." => {}
            ".." => {
                parts.pop();
            }
            "" => {}
            s => parts.push(s),
        }
    }
    let trailing = if path.ends_with('/') && !path.is_empty() { "/" } else { "" };
    let joined = parts.join("/");
    format!("{}/{}{}{}", &base[..origin_end], joined, if joined.is_empty() { "" } else { trailing }, tail).replace("//#", "/#")
}

fn strip_fragment(u: &str) -> &str {
    u.split('#').next().unwrap_or(u)
}

/// The guide's pages, fetched, as one Markdown document, with a note of
/// where it's from and its license at the top.
pub fn fetch_markdown(g: &Guide) -> Result<String, String> {
    let mut urls: Vec<String> = g.pages.iter().map(|s| s.to_string()).collect();
    if let Some((index, links)) = g.index {
        let html = get(index)?;
        let doc = scraper::Html::parse_document(&html);
        let sel = scraper::Selector::parse(links).map_err(|e| format!("{links}: {e}"))?;
        let host = |u: &str| u.split('/').nth(2).unwrap_or("").to_string();
        for a in doc.select(&sel) {
            if let Some(h) = a.value().attr("href") {
                let u = strip_fragment(&join(index, h)).to_string();
                if host(&u) == host(index) && u != strip_fragment(index) && !urls.contains(&u) {
                    urls.push(u);
                }
            }
        }
    }
    if urls.is_empty() {
        return Err("The guide has no pages.".into());
    }
    let page_ids: Vec<(String, usize)> = urls.iter().enumerate().flat_map(|(i, u)| u.split(" | ").map(move |u| (strip_fragment(u.trim()).to_string(), i))).collect();
    let mut out = String::new();
    let home = g.pages.first().copied().or(g.index.map(|i| i.0)).unwrap_or("").split(" | ").next().unwrap_or("").trim();
    out.push_str(&format!(
        "> **{}**, from [{}]({}), converted to Markdown by Tessera on {}. License: [{}]({}).\n\n",
        g.title,
        home.split('/').nth(2).unwrap_or(home),
        home.replace("raw.githubusercontent.com", "github.com").replacen("/main/", "/blob/main/", 1).replacen("/master/", "/blob/master/", 1),
        today(),
        g.license,
        g.license_url
    ));
    // (Fetched and converted side by side, eight at a time.)
    let key = g.langs[0];
    let n = urls.len();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done = std::sync::atomic::AtomicUsize::new(0);
    let mut parts: Vec<Option<Result<String, String>>> = (0..n).map(|_| None).collect();
    set_progress(key, &format!("Fetching {n} page{}…", if n == 1 { "" } else { "s" }));
    {
        let slots: Vec<std::sync::Mutex<Option<Result<String, String>>>> = (0..n).map(|_| std::sync::Mutex::new(None)).collect();
        std::thread::scope(|sc| {
            for _ in 0..n.min(8) {
                sc.spawn(|| loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    if i >= n {
                        break;
                    }
                    let r = get_any(&urls[i]).and_then(|text| {
                        let u = urls[i].split(" | ").next().unwrap_or(&urls[i]).trim();
                        let md = if u.ends_with(".md") { clean_markdown(&expand_includes(&text, u)) } else { html_to_markdown(&text, g)? };
                        Ok(relink(&md, u, &page_ids))
                    });
                    *slots[i].lock().unwrap() = Some(r);
                    let k = done.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    set_progress(key, &format!("Fetched {k} of {n} page{}…", if n == 1 { "" } else { "s" }));
                });
            }
        });
        for (o, s) in parts.iter_mut().zip(slots) {
            *o = s.into_inner().unwrap();
        }
    }
    let mut got = 0;
    let mut failed = vec![];
    for (i, part) in parts.into_iter().enumerate() {
        match part.unwrap_or_else(|| Err("not fetched".into())) {
            Ok(md) => {
                out.push_str(&format!("<a id=\"page-{i}\"></a>\n\n{}\n\n", md.trim()));
                got += 1;
            }
            Err(e) if i == 0 => return Err(e),
            Err(e) => failed.push(e),
        }
    }
    if got == 0 {
        return Err("Nothing could be fetched.".into());
    }
    if !failed.is_empty() {
        out.push_str(&format!("\n\n> {} page(s) couldn't be fetched: {}\n", failed.len(), failed.join("; ")));
    }
    Ok(out)
}

/// Code a Markdown page pulls in from its repository's files (Microsoft's
/// docs: `[!code-fsharp[Name](~/path#L1-L9)]` and `:::code source="path"
/// id="Region":::`), fetched and put in as code blocks.
fn expand_includes(md: &str, page: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;
    static RES: OnceLock<[Regex; 2]> = OnceLock::new();
    let [bang, colons] = RES.get_or_init(|| [
        Regex::new(r"(?m)^[ \t>]*\[!code-([A-Za-z0-9#+-]+)\[[^\]]*\]\(([^)#\s]+)(?:#([^)\s]+))?\)\]\s*$").unwrap(),
        Regex::new(r#"(?m)^[ \t>]*:::code\s+([^\n]*?):::\s*$"#).unwrap(),
    ]);
    if !md.contains("[!code-") && !md.contains(":::code") {
        return md.to_string();
    }
    // (The repository's root, for "~/" paths: raw.githubusercontent.com/OWNER/REPO/BRANCH/.)
    let root: String = page.splitn(7, '/').take(6).collect::<Vec<_>>().join("/") + "/";
    let mut files: std::collections::HashMap<String, Option<String>> = Default::default();
    let mut file = |path: &str| -> Option<String> {
        let url = match path.strip_prefix("~/") {
            Some(p) => format!("{root}{p}"),
            None => join(page, path),
        };
        files.entry(url.clone()).or_insert_with(|| get(&url).ok()).clone()
    };
    let attr = |a: &str, name: &str| Regex::new(&format!(r#"\b{name}="([^"]*)""#)).ok().and_then(|r| r.captures(a).map(|c| c[1].to_string()));
    let mut out = String::with_capacity(md.len());
    for line in md.split_inclusive('\n') {
        let (lang, path, part) = if let Some(c) = bang.captures(line) {
            (c[1].to_string(), c[2].to_string(), c.get(3).map(|m| m.as_str().to_string()).unwrap_or_default())
        } else if let Some(c) = colons.captures(line) {
            let a = &c[1];
            let part = attr(a, "id").or_else(|| attr(a, "range").map(|r| format!("L{}", r.replace('-', "-L")))).unwrap_or_default();
            (attr(a, "language").unwrap_or_default(), attr(a, "source").unwrap_or_default(), part)
        } else {
            out.push_str(line);
            continue;
        };
        let Some(text) = (if path.is_empty() { None } else { file(&path) }) else {
            out.push_str(line);
            continue;
        };
        let code = include_part(&text, &part);
        out.push_str(&format!("\n```{}\n{}\n```\n", lang.to_lowercase(), code.trim_end()));
    }
    out
}

/// Lines `L5-L9`, or the region between `<Name>` and `</Name>` (in any
/// comment), or the whole file.
fn include_part(text: &str, part: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    if let Some(r) = part.strip_prefix('L') {
        let (a, b) = r.split_once("-L").or_else(|| r.split_once('-')).unwrap_or((r, r));
        if let (Ok(a), Ok(b)) = (a.parse::<usize>(), b.trim_start_matches('L').parse::<usize>()) {
            let (a, b) = (a.max(1) - 1, b.min(lines.len()));
            if a < b {
                return textwrap_dedent(&lines[a..b].join("\n"));
            }
        }
    }
    if !part.is_empty() {
        let (open, close) = (format!("<{part}>"), format!("</{part}>"));
        if let Some(s) = lines.iter().position(|l| l.contains(&open)) {
            if let Some(e) = lines[s + 1..].iter().position(|l| l.contains(&close)) {
                return textwrap_dedent(&lines[s + 1..s + 1 + e].join("\n"));
            }
        }
    }
    text.to_string()
}

/// Markdown from a repository, cleaned of what only its site understands:
/// front matter (its title kept as the heading), template tags and macros.
fn clean_markdown(text: &str) -> String {
    use regex::Regex;
    let mut text = text.replace("\r\n", "\n");
    let mut title = None;
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            for l in rest[..end].lines() {
                if let Some(t) = l.strip_prefix("title:") {
                    title = Some(t.trim().trim_matches(['"', '\'']).to_string());
                }
            }
            text = rest[end + 4..].trim_start_matches(|c| c != '\n').to_string();
        }
    }
    let re = |p: &str| Regex::new(p).unwrap();
    // MDN's macros ({{Glossary("x")}} → x; the rest dropped), Jekyll/Liquid
    // tags, DocC directives, Writerside attribute lines, HTML comments.
    text = re(r#"\{\{\s*(?:Glossary|domxref|jsxref|cssxref|HTMLElement|htmlelement|HTTPHeader|httpheader|CSSxRef|JSxRef|DOMxRef|glossary)\(\s*"([^"]*)"(?:\s*,\s*"([^"]*)")?[^}]*\}\}"#)
        .replace_all(&text, |c: &regex::Captures| c.get(2).filter(|m| !m.as_str().is_empty()).unwrap_or(c.get(1).unwrap()).as_str().to_string())
        .into_owned();
    text = re(r"\{\{[^}]*\}\}").replace_all(&text, "").into_owned();
    text = re(r"\{%[^%]*%\}").replace_all(&text, "").into_owned();
    text = re(r"(?s)<!--.*?-->").replace_all(&text, "").into_owned();
    text = re(r"(?m)^\{[^}\n]*\}\s*$").replace_all(&text, "").into_owned();
    text = re(r"(?s)@Comment\s*\{.*?\n\}").replace_all(&text, "").into_owned();
    text = re(r"(?m)^\s*@(?:Metadata|Options|TechnologyRoot|Image|Row|Column)\b.*$").replace_all(&text, "").into_owned();
    text = re(r"<doc:([A-Za-z0-9_]+)(?:#[^>]*)?>").replace_all(&text, "$1").into_owned();
    // (Microsoft's "> [!NOTE]" callouts read fine as quotes.)
    text = re(r"(?m)^>\s*\[!(\w+)\]\s*$").replace_all(&text, "> **$1**").into_owned();
    let has_h1 = text.lines().any(|l| l.starts_with("# "));
    match title {
        Some(t) if !has_h1 => format!("# {t}\n\n{}", text.trim_start()),
        _ => text,
    }
}

/// An HTML page's document part as Markdown.
fn html_to_markdown(html: &str, g: &Guide) -> Result<String, String> {
    let doc = scraper::Html::parse_document(html);
    let mut part = None;
    for s in g.select.split(',').map(str::trim).filter(|s| !s.is_empty()).chain(["main", "body"]) {
        if let Ok(sel) = scraper::Selector::parse(s) {
            if let Some(e) = doc.select(&sel).next() {
                part = Some(e);
                break;
            }
        }
    }
    let Some(part) = part else { return Ok(String::new()) };
    let mut inner = part.inner_html();
    // (Leave out what isn't the document: its HTML cut out of the part's.)
    let drop = format!("script, style, noscript, button, template, a.headerlink, a.anchor, .headerlink, .anchorjs-link, .copybutton, .sr-only, .visually-hidden{}{}", if g.drop.is_empty() { "" } else { ", " }, g.drop);
    // (Code drawn as tables of tokens or lines with <br>: its text, a line
    // per row or break.)
    if !g.code_sel.is_empty() {
        if let Ok(sel) = scraper::Selector::parse(g.code_sel) {
            let esc = |t: &str| t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
            for e in part.select(&sel) {
                let mut text = String::new();
                code_text(*e, &mut text, e.html().contains("<br"));
                let text = textwrap_dedent(&text.replace('\u{a0}', " ").lines().map(str::trim_end).collect::<Vec<_>>().join("\n"));
                inner = inner.replacen(&e.html(), &format!("<pre><code>{}</code></pre>", esc(&text)), 1);
            }
        }
    }
    let mut cut: Vec<String> = vec![];
    for s in drop.split(',').map(str::trim) {
        if let Ok(sel) = scraper::Selector::parse(s) {
            for e in part.select(&sel) {
                cut.push(e.html());
            }
        }
    }
    cut.sort_by_key(|c| std::cmp::Reverse(c.len()));
    for c in cut {
        if !c.is_empty() {
            inner = inner.replacen(&c, "", 1);
        }
    }
    let inner = keep_ids(&code_in_pre(&inner));
    let conv = htmd::HtmlToMarkdown::builder().skip_tags(vec!["script", "style", "svg", "head", "form", "input", "select", "iframe"]).build();
    let md = conv.convert(&inner).map_err(|e| e.to_string())?;
    // (A page with no top heading of its own (cppreference keeps its
    // title outside the article) gets its title as one, for the contents.)
    let md = if md.lines().any(|l| l.starts_with("# ")) {
        md
    } else {
        let title = scraper::Selector::parse("h1#firstHeading, title").ok().and_then(|s| doc.select(&s).next()).map(|t| t.text().collect::<String>()).unwrap_or_default();
        let title = title.split(" - ").next().unwrap_or("").split(" | ").next().unwrap_or("").split(" \u{2014} ").next().unwrap_or("").trim().to_string();
        if title.is_empty() { md } else { format!("# {title}\n\n{md}") }
    };
    let md = regex::Regex::new("\u{27e6}#([^\u{27e7}]*)\u{27e7}").unwrap().replace_all(&md, |c: &regex::Captures| format!("<a id=\"{}\"></a>", c[1].replace('\\', ""))).into_owned();
    // (Pilcrows and the like that headings carried as links.)
    Ok(md.replace(" ¶", "").replace("¶", "").replace("\u{a0}", " "))
}

fn code_text(n: ego_tree::NodeRef<scraper::Node>, out: &mut String, brs: bool) {
    for c in n.children() {
        match c.value() {
            scraper::Node::Text(t) if brs => out.push_str(&t.replace('\n', "")),
            scraper::Node::Text(t) => out.push_str(t),
            // (Code kept in a comment for a script to show: dev.java.)
            scraper::Node::Comment(t) => out.push_str(&textwrap_dedent(t)),
            scraper::Node::Element(e) => {
                let name = e.name();
                if name == "br" {
                    out.push('\n');
                    continue;
                }
                code_text(c, out, brs);
                if matches!(name, "tr" | "p" | "div") && !out.ends_with('\n') {
                    out.push('\n');
                }
            }
            _ => {}
        }
    }
}

/// The page's own ids kept (as empty elements Markdown passes along), so
/// its links to its parts still land: ids of headings, sections, terms,
/// and named anchors.
fn keep_ids(html: &str) -> String {
    let re = regex::Regex::new(r#"<(h[1-6]|section|div|dt|p|li|a)\b([^>]*)>"#).unwrap();
    let id_re = regex::Regex::new(r#"\s(?:id|name)\s*=\s*"([^"]+)""#).unwrap();
    re.replace_all(html, |c: &regex::Captures| {
        let whole = &c[0];
        let Some(id) = id_re.captures(&c[2]).map(|m| m[1].to_string()) else { return whole.to_string() };
        let id = id.replace(['"', '<', '>', '&'], "");
        // (A mark in the text, made an anchor after the conversion: htmd
        // keeps no empty elements.)
        let anchor = format!("\u{27e6}#{id}\u{27e7}");
        if &c[1] == "a" { format!("{anchor}{whole}") } else { format!("{whole}{anchor}") }
    })
    .into_owned()
}

/// Lines with their common indent taken off.
fn textwrap_dedent(t: &str) -> String {
    let lines: Vec<&str> = t.trim_matches('\n').lines().collect();
    let pad = lines.iter().filter(|l| !l.trim().is_empty()).map(|l| l.len() - l.trim_start().len()).min().unwrap_or(0);
    lines.iter().map(|l| if l.len() >= pad { &l[pad..] } else { l.trim_start() }).collect::<Vec<_>>().join("\n").trim_end().to_string()
}

/// `<pre>` without a `<code>` inside gets one, so its text stays a code
/// block (Sphinx, texinfo); a language from its class (or its wrapper's)
/// goes along.
fn code_tags_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?i)</?code\b[^>]*>").unwrap())
}

fn code_in_pre(html: &str) -> String {
    let lang_re = regex::Regex::new(r#"(?:language|lang|highlight|brush:?)-?\s*([A-Za-z0-9_+#-]+)"#).unwrap();
    let mut out = String::with_capacity(html.len() + 256);
    let mut rest = html;
    let mut last_class = String::new();
    while let Some(i) = rest.find("<pre") {
        let before = &rest[..i];
        // (A wrapper's class, like Sphinx's "highlight-python3".)
        last_class = match before.rfind("class=\"") {
            Some(k) if before.len() - k < 400 => {
                let c = &before[k + 7..];
                c[..c.find('"').unwrap_or(0)].to_string()
            }
            _ => String::new(),
        };
        out.push_str(before);
        let Some(gt) = rest[i..].find('>') else { break };
        let open = &rest[i..i + gt + 1];
        let after = &rest[i + gt + 1..];
        if !(open.starts_with("<pre>") || open.starts_with("<pre ")) {
            out.push_str(open);
            rest = after;
            continue;
        }
        let Some(end) = after.find("</pre>") else {
            out.push_str(open);
            rest = after;
            continue;
        };
        let body = &after[..end];
        // (One code element, the pre's whole text: texinfo puts several in
        // one pre, with text between.)
        let first_code = body.trim_start().starts_with("<code");
        let code_class = if first_code { body.trim_start().split('>').next().and_then(|t| t.split("class=\"").nth(1)).map(|c| c[..c.find('"').unwrap_or(0)].to_string()).unwrap_or_default() } else { String::new() };
        let body = &code_tags_re().replace_all(body, "").into_owned();
        {
            let class = open.split("class=\"").nth(1).map(|c| c[..c.find('"').unwrap_or(0)].to_string()).unwrap_or_default();
            let class = if code_class.starts_with("language-") || code_class.starts_with("lang-") { code_class.clone() } else { class };
            let lang = lang_re.captures(&class).or_else(|| lang_re.captures(&last_class)).map(|c| c[1].to_string()).unwrap_or_default();
            out.push_str("<pre>");
            if lang.is_empty() || lang == "default" || lang == "none" {
                out.push_str("<code>");
            } else {
                out.push_str(&format!("<code class=\"language-{lang}\">"));
            }
            out.push_str(body);
            out.push_str("</code>");
        }
        out.push_str("</pre>");
        rest = &after[end + 6..];
    }
    out.push_str(rest);
    out
}

/// Links made whole; those to the guide's own pages kept in the guide
/// (`#page-N`, or the heading they name).
fn relink(md: &str, page: &str, pages: &[(String, usize)]) -> String {
    let re = regex::Regex::new(r#"\]\(\s*<?([^)\s>]*)>?(\s+"[^"]*")?\)"#).unwrap();
    re.replace_all(md, |c: &regex::Captures| {
        let href = &c[1];
        let title = c.get(2).map(|m| m.as_str()).unwrap_or("");
        if href.is_empty() {
            return c[0].to_string();
        }
        if href.starts_with('#') {
            return format!("]({href}{title})");
        }
        let full = join(page, href);
        let (base, frag) = match full.split_once('#') {
            Some((b, f)) => (b.to_string(), Some(f.to_string())),
            None => (full.clone(), None),
        };
        if let Some((_, n)) = pages.iter().find(|(p, _)| *p == base || p.trim_end_matches('/') == base.trim_end_matches('/')) {
            return match frag {
                Some(f) if !f.is_empty() => format!("](#{f}{title})"),
                _ => format!("](#page-{n}{title})"),
            };
        }
        format!("]({full}{title})")
    })
    .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins() {
        assert_eq!(join("https://a.org/x/y/z.html", "w.html"), "https://a.org/x/y/w.html");
        assert_eq!(join("https://a.org/x/y/z.html", "../w.html#f"), "https://a.org/x/w.html#f");
        assert_eq!(join("https://a.org/x/y/", "w/"), "https://a.org/x/y/w/");
        assert_eq!(join("https://a.org/x/y/z.html", "/r"), "https://a.org/r");
        assert_eq!(join("https://a.org/x/y/z.html", "//b.org/q"), "https://b.org/q");
        assert_eq!(join("https://a.org/x/y/z.html", "#f"), "https://a.org/x/y/z.html#f");
        assert_eq!(join("https://a.org", "w.html"), "https://a.org/w.html");
    }

    #[test]
    fn pre_gets_code() {
        let h = code_in_pre("<div class=\"highlight-python3\"><pre>x = 1</pre></div><pre><code>y</code></pre>");
        assert_eq!(h, "<div class=\"highlight-python3\"><pre><code class=\"language-python3\">x = 1</code></pre></div><pre><code>y</code></pre>");
    }

    #[test]
    fn relinks() {
        let pages = vec![("https://a.org/t/one.html".to_string(), 0), ("https://a.org/t/two.html".to_string(), 1)];
        let md = relink("[a](two.html) [b](two.html#sec) [c](https://x.org/) [d](../img.png \"t\") [e](#here)", "https://a.org/t/one.html", &pages);
        assert_eq!(md, "[a](#page-1) [b](#sec) [c](https://x.org/) [d](https://a.org/img.png \"t\") [e](#here)");
    }

    #[test]
    fn markdown_cleaned() {
        let md = clean_markdown("---\ntitle: Grammar and types\nslug: x\n---\n\n{{jsSidebar}}\nUse {{Glossary(\"Hoisting\", \"hoisted\")}} vars.\n");
        assert!(md.starts_with("# Grammar and types\n"), "{md}");
        assert!(md.contains("Use hoisted vars.") && !md.contains("{{"), "{md}");
    }

    /// Every guide fetched and converted, into $TESSERA_GUIDES_OUT (a
    /// check of the sources: `cargo test fetch_all -- --ignored`).
    #[test]
    #[ignore]
    fn fetch_all() {
        let out = PathBuf::from(std::env::var("TESSERA_GUIDES_OUT").unwrap_or_else(|_| "/tmp/tessera-guides".into()));
        std::fs::create_dir_all(&out).unwrap();
        let only = std::env::var("TESSERA_GUIDES_ONLY").unwrap_or_default();
        let mut bad = vec![];
        for g in GUIDES {
            if !only.is_empty() && !only.split(',').any(|o| g.langs.contains(&o)) {
                continue;
            }
            match fetch_markdown(g) {
                Ok(md) => {
                    let heads = md.lines().filter(|l| l.starts_with('#')).count();
                    let code = md.matches("\n```").count() / 2;
                    eprintln!("{:12} {:>8} bytes {:>4} headings {:>4} code", g.langs[0], md.len(), heads, code);
                    std::fs::write(out.join(format!("{}.md", g.langs[0])), md).unwrap();
                }
                Err(e) => {
                    eprintln!("{:12} FAILED {e}", g.langs[0]);
                    bad.push(g.langs[0]);
                }
            }
        }
        eprintln!("failed: {bad:?}");
    }
}
