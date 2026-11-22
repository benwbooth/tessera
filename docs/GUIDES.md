# Language guides

The input editor's and notebook's **Guide** chip opens the guide for the
current language in a tab. tessh's guide is Tessera's own
(`tessh/doc/guide.md`, built in). Every other guide is that language's own
documentation, **not bundled**: the first time it's opened, Tessera fetches
it from the official site, converts it to Markdown (HTML pages through
`htmd`; Markdown sources cleaned of site-only template tags) and keeps it in
`$XDG_STATE_HOME/tessera/guides/LANG.md`, and the drawn page next to it
(`LANG.body.html`), so it opens at once the next time. **Refresh** on the
page fetches it again.

Why fetched and not bundled: together the documents are tens of megabytes,
and their licenses differ (GNU FDL, CC BY-SA, and some that allow reading
but not redistribution). Fetched on the reader's own machine, nothing is
redistributed with Tessera. Each kept file starts with its source, its
license and the date it was fetched.

The sources are in `src/guides.rs` (`GUIDES`). To check them all:
`TESSERA_GUIDES_OUT=/tmp/g cargo test fetch_all -- --ignored`.

| Language | Source | License |
| --- | --- | --- |
| python, python3 | [The Python Tutorial](https://docs.python.org/3/tutorial/index.html) | [PSF License 2.0](https://docs.python.org/3/license.html) |
| rust | [The Rust Programming Language](https://doc.rust-lang.org/book/print.html) | [MIT or Apache-2.0](https://github.com/rust-lang/book/blob/main/LICENSE-MIT) |
| bash, sh | [Bash Reference Manual](https://www.gnu.org/software/bash/manual/bash.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| zsh | [A User's Guide to the Z-Shell](https://zsh.sourceforge.io/Guide/zshguide.html) | [Freely distributable (© Peter Stephenson)](https://zsh.sourceforge.io/Guide/zshguide.html) |
| fish | [fish Tutorial](https://fishshell.com/docs/current/tutorial.html) | [GPL-2.0](https://fishshell.com/docs/current/license.html) |
| lua | [Lua 5.4 Reference Manual](https://www.lua.org/manual/5.4/manual.html) | [MIT (Lua license)](https://www.lua.org/license.html) |
| go, golang | [Effective Go and the Go Spec](https://go.dev/doc/effective_go) | [CC BY 4.0 (code BSD-3-Clause)](https://go.dev/copyright) |
| javascript, node, js | [MDN JavaScript Guide](https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/introduction/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| typescript, tsx, ts | [The TypeScript Handbook](https://www.typescriptlang.org/docs/handbook/intro.html) | [CC BY 4.0](https://github.com/microsoft/TypeScript-Website/blob/v2/LICENSE) |
| c | [GNU C Language Intro and Reference Manual](https://www.gnu.org/software/c-intro-and-ref/manual/c-intro-and-ref.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| cpp, c++, cxx | [C++ language (cppreference.com)](https://en.cppreference.com/w/cpp/language/basic_concepts) | [CC BY-SA 3.0](https://en.cppreference.com/w/Cppreference:Copyright/CC-BY-SA) |
| java | [Learn Java: Language Basics (dev.java)](https://dev.java/learn/language/constructs/basics/) | [© Oracle; read from dev.java, not redistributed](https://www.oracle.com/legal/copyright.html) |
| ruby | [Ruby Syntax (docs.ruby-lang.org)](https://docs.ruby-lang.org/en/master/syntax_rdoc.html) | [Ruby License or BSD-2-Clause](https://www.ruby-lang.org/en/about/license.txt) |
| perl | [Perl documentation (perlintro and friends)](https://perldoc.perl.org/perlintro) | [Artistic License or GPL](https://perldoc.perl.org/perlartistic) |
| php | [PHP Manual: Getting Started and Language Reference](https://www.php.net/manual/en/introduction.php) | [CC BY 3.0](https://www.php.net/manual/en/copyright.php) |
| r | [An Introduction to R](https://cran.r-project.org/doc/manuals/r-release/R-intro.html) | [Verbatim copying permitted (R Core Team)](https://cran.r-project.org/doc/manuals/r-release/R-intro.html) |
| julia | [The Julia Manual](https://docs.julialang.org/en/v1/manual/getting-started/) | [MIT](https://github.com/JuliaLang/julia/blob/master/LICENSE.md) |
| elixir | [Elixir Getting Started](https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/introduction.md) | [Apache-2.0](https://github.com/elixir-lang/elixir/blob/main/LICENSE) |
| erlang | [Getting Started with Erlang](https://www.erlang.org/doc/system/getting_started.html) | [Apache-2.0](https://www.erlang.org/doc/system/license.html) |
| groovy | [Groovy Language Documentation](https://groovy-lang.org/syntax.html) | [Apache-2.0](https://groovy-lang.org/) |
| powershell, pwsh | [PowerShell 101](https://raw.githubusercontent.com/MicrosoftDocs/PowerShell-Docs/main/reference/docs-conceptual/learn/ps101/01-getting-started.md) | [CC BY 4.0](https://github.com/MicrosoftDocs/PowerShell-Docs/blob/main/LICENSE) |
| nu, nushell | [The Nushell Book](https://raw.githubusercontent.com/nushell/nushell.github.io/main/book/quick_tour.md) | [MIT](https://github.com/nushell/nushell.github.io/blob/main/LICENSE) |
| tcl | [Tcl Tutorial](https://www.tcl-lang.org/man/tcl8.5/tutorial/tcltutorial.html) | [Tcl/Tk license (BSD-style)](https://www.tcl-lang.org/software/tcltk/license.html) |
| awk | [GAWK: Effective AWK Programming](https://www.gnu.org/software/gawk/manual/gawk.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| jq | [jq Manual](https://jqlang.org/manual/) | [CC BY 3.0 (code MIT)](https://github.com/jqlang/jq/blob/master/COPYING) |
| nix | [Nix language basics (nix.dev)](https://nix.dev/tutorials/nix-language) | [CC BY-SA 4.0](https://github.com/NixOS/nix.dev/blob/master/LICENSE) |
| octave, matlab | [GNU Octave: Getting Started](https://docs.octave.org/latest/Getting-Started.html) | [Verbatim copying permitted (GNU Octave manual)](https://docs.octave.org/latest/) |
| sql, sqlite, duckdb | [SQL As Understood By SQLite](https://www.sqlite.org/lang_select.html) | [Public domain](https://www.sqlite.org/copyright.html) |
| haskell, ghc | [A Gentle Introduction to Haskell 98](https://www.haskell.org/tutorial/) | [Free to copy (Hudak, Peterson, Fasel)](https://www.haskell.org/tutorial/) |
| ocaml | [The OCaml Manual: An Introduction to OCaml](https://ocaml.org/manual/latest/coreexamples.html) | [CC BY-SA 4.0](https://ocaml.org/manual/latest/) |
| racket | [The Racket Guide: Racket Essentials](https://docs.racket-lang.org/guide/to-scheme.html) | [MIT or Apache-2.0](https://download.racket-lang.org/license.html) |
| scheme | [Guile: Hello Guile! and Programming in Scheme](https://www.gnu.org/software/guile/manual/html_node/Hello-Guile_0021.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| common-lisp, lisp, commonlisp | [The Common Lisp Cookbook](https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/getting-started.md) | [See the Cookbook's license](https://github.com/LispCookbook/cl-cookbook) |
| dart | [Dart Language Tour](https://dart.dev/language) | [CC BY 4.0 (code BSD-3-Clause)](https://dart.dev/terms) |
| swift | [The Swift Programming Language](https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/GuidedTour/GuidedTour.md) | [Apache-2.0 with Runtime Library Exception](https://github.com/swiftlang/swift-book/blob/main/LICENSE.txt) |
| zig | [Zig Language Reference](https://ziglang.org/documentation/master/) | [MIT](https://github.com/ziglang/zig/blob/master/LICENSE) |
| nim | [Nim Tutorial](https://nim-lang.org/docs/tut1.html) | [MIT](https://github.com/nim-lang/Nim/blob/devel/copying.txt) |
| crystal | [Crystal Tutorials: Language Introduction](https://raw.githubusercontent.com/crystal-lang/crystal-book/master/docs/tutorials/basics/10_hello_world.md) | [Apache-2.0](https://github.com/crystal-lang/crystal-book/blob/master/LICENSE) |
| v | [V Documentation](https://raw.githubusercontent.com/vlang/v/master/doc/docs.md) | [MIT](https://github.com/vlang/v/blob/master/LICENSE) |
| fsharp, f# | [Tour of F#](https://raw.githubusercontent.com/dotnet/docs/main/docs/fsharp/tour.md) | [CC BY 4.0](https://github.com/dotnet/docs/blob/main/LICENSE) |
| c-sharp, csharp, c#, cs | [A tour of the C# language](https://raw.githubusercontent.com/dotnet/docs/main/docs/csharp/tour-of-csharp/overview.md) | [CC BY 4.0](https://github.com/dotnet/docs/blob/main/LICENSE) |
| kotlin | [Kotlin Tour](https://raw.githubusercontent.com/JetBrains/kotlin-web-site/master/docs/topics/tour/kotlin-tour-hello-world.md) | [Apache-2.0](https://github.com/JetBrains/kotlin-web-site/blob/master/LICENSE) |
| scala | [Scala 3 Book](https://docs.scala-lang.org/scala3/book/introduction.html) | [CC BY-SA 4.0](https://github.com/scala/docs.scala-lang/blob/main/LICENSE.md) |
| clojure | [Learn Clojure (clojure.org)](https://clojure.org/guides/learn/clojure) | [CC BY-SA 3.0](https://clojure.org/community/license) |
| prolog | [SWI-Prolog Manual: Getting started](https://www.swi-prolog.org/pldoc/man?section=quickstart) | [BSD-2-Clause](https://www.swi-prolog.org/license.html) |
| fortran | [Fortran Quickstart Tutorial (fortran-lang.org)](https://raw.githubusercontent.com/fortran-lang/webpage/main/source/learn/quickstart/hello_world.md) | [MIT](https://github.com/fortran-lang/webpage/blob/main/LICENSE) |
| elvish | [Tour of Elvish](https://elv.sh/learn/tour.html) | [BSD-2-Clause](https://github.com/elves/elvish/blob/master/LICENSE) |
| fennel | [Fennel Tutorial](https://fennel-lang.org/tutorial) | [MIT](https://git.sr.ht/~technomancy/fennel/tree/main/item/LICENSE) |
| forth | [Gforth Tutorial](https://www.complang.tuwien.ac.at/forth/gforth/Docs-html/Tutorial.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| hare | [Hare: An introduction](https://harelang.org/tutorials/introduction) | [CC BY-ND 4.0](https://harelang.org/) |
| janet, janet-simple | [Janet Documentation](https://janet-lang.org/docs/index.html) | [MIT](https://github.com/janet-lang/janet/blob/master/LICENSE) |
| koka | [The Koka Programming Language](https://koka-lang.github.io/koka/doc/book.html) | [Apache-2.0](https://github.com/koka-lang/koka/blob/master/LICENSE) |
| lean | [Functional Programming in Lean](https://lean-lang.org/functional_programming_in_lean/) | [CC BY 4.0](https://lean-lang.org/functional_programming_in_lean/) |
| luau | [Luau: Syntax and Types](https://raw.githubusercontent.com/luau-lang/site/master/src/content/docs/getting-started/intro.md) | [MIT](https://github.com/luau-lang/site/blob/master/LICENSE) |
| odin | [Odin Overview](https://odin-lang.org/docs/overview/) | [BSD-3-Clause](https://github.com/odin-lang/Odin/blob/master/LICENSE) |
| pascal | [Free Pascal Reference Guide](https://www.freepascal.org/docs-html/ref/ref.html) | [Free Pascal documentation license (free to copy)](https://www.freepascal.org/docs.html) |
| ada | [Introduction to Ada (learn.adacore.com)](https://learn.adacore.com/courses/intro-to-ada/index.html) | [CC BY-SA 4.0](https://github.com/AdaCore/learn/blob/main/LICENSE) |
| gleam | [The Gleam Language Tour](https://tour.gleam.run/everything/) | [Apache-2.0](https://github.com/gleam-lang/language-tour/blob/main/LICENCE) |
| koto | [The Koto Language Guide](https://koto.dev/docs/next/language/) | [MIT](https://github.com/koto-lang/koto/blob/main/LICENSE) |
| inko | [The Inko Manual](https://docs.inko-lang.org/manual/latest/) | [MPL-2.0](https://github.com/inko-lang/inko/blob/main/LICENSE) |
| amber | [Amber Documentation](https://docs.amber-lang.com/) | [GPL-3.0](https://github.com/amber-lang/amber/blob/master/LICENSE) |
| uxntal | [Uxntal (XXIIVV wiki)](https://wiki.xxiivv.com/site/uxntal.html) | [CC BY-NC-SA 4.0; read from the wiki, not redistributed](https://wiki.xxiivv.com/site/about.html) |
| nasm | [NASM Manual: The NASM Language](https://www.nasm.us/doc/nasmdoc2.html) | [BSD-2-Clause](https://www.nasm.us/doc/nasmdoc1.html) |
| gas | [Using as: Syntax](https://sourceware.org/binutils/docs/as/Syntax.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| llvm | [LLVM Language Reference Manual](https://llvm.org/docs/LangRef.html) | [Apache-2.0 with LLVM Exception](https://llvm.org/LICENSE.txt) |
| elisp | [An Introduction to Programming in Emacs Lisp](https://www.gnu.org/software/emacs/manual/html_mono/eintr.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| jsonnet | [Jsonnet Tutorial](https://jsonnet.org/learning/tutorial.html) | [Apache-2.0](https://github.com/google/jsonnet/blob/master/LICENSE) |
| nickel | [The Nickel User Manual](https://raw.githubusercontent.com/tweag/nickel/master/doc/manual/introduction.md) | [MIT](https://github.com/tweag/nickel/blob/master/LICENSE) |
| dhall | [Dhall Language Tour](https://raw.githubusercontent.com/dhall-lang/dhall-lang/master/docs/tutorials/Language-Tour.md) | [BSD-3-Clause](https://github.com/dhall-lang/dhall-lang/blob/master/LICENSE) |
| cue | [A Tour of CUE](https://cuelang.org/docs/tour/basics/json-superset/) | [Apache-2.0](https://github.com/cue-lang/cuelang.org/blob/master/LICENSE) |
| pkl | [Pkl Language Tutorial](https://pkl-lang.org/main/current/language-tutorial/index.html) | [Apache-2.0](https://github.com/apple/pkl/blob/main/LICENSE.txt) |
| rego | [Rego: Policy Language](https://www.openpolicyagent.org/docs/policy-language) | [Apache-2.0](https://github.com/open-policy-agent/opa/blob/main/LICENSE) |
| d | [The D Language Tour: D's Basics](https://raw.githubusercontent.com/dlang-tour/english/master/basics/imports-and-modules.md) | [Boost-1.0 (D Language Tour)](https://github.com/dlang-tour/core/blob/master/LICENSE.txt) |
| sml | [SML Help (Standard ML)](https://smlhelp.github.io/book/docs/start/install/) | [© the SML Help authors; read from the site, not redistributed](https://github.com/smlhelp/book) |
| prql | [The PRQL Language Book](https://prql-lang.org/book/print.html) | [Apache-2.0](https://github.com/PRQL/prql/blob/main/LICENSE) |
| scss, sass | [Sass Basics](https://sass-lang.com/guide/) | [MIT](https://github.com/sass/sass-site/blob/main/LICENSE) |
| make | [GNU make Manual](https://www.gnu.org/software/make/manual/make.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| cmake | [CMake Tutorial](https://cmake.org/cmake/help/latest/guide/tutorial/index.html) | [BSD-3-Clause](https://cmake.org/licensing/) |
| typst | [Typst Tutorial](https://typst.app/docs/tutorial/) | [Apache-2.0](https://github.com/typst/typst/blob/main/LICENSE) |
| dot, graphviz | [The DOT Language (Graphviz)](https://graphviz.org/doc/info/lang.html) | [EPL-2.0](https://graphviz.org/license/) |
| latex, tex | [Learn LaTeX](https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-01.md) | [CC BY-SA 4.0](https://github.com/learnlatex/learnlatex.github.io/blob/main/LICENSE.md) |
| openscad | [The OpenSCAD Language (OpenSCAD User Manual)](https://en.wikibooks.org/wiki/OpenSCAD_User_Manual/The_OpenSCAD_Language) | [CC BY-SA 4.0 (Wikibooks)](https://en.wikibooks.org/wiki/OpenSCAD_User_Manual) |
| json | [Introducing JSON (json.org)](https://www.json.org/json-en.html) | [© JSON.org; read from json.org, not redistributed](https://www.json.org/json-en.html) |
| yaml | [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/) | [Free to copy (the YAML spec's own terms)](https://yaml.org/spec/1.2.2/) |
| toml | [TOML v1.0.0](https://raw.githubusercontent.com/toml-lang/toml/1.0.0/toml.md) | [MIT](https://github.com/toml-lang/toml/blob/main/LICENSE) |
| html | [MDN: Structuring content with HTML](https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/basic_html_syntax/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| css | [MDN: CSS styling basics](https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/styling_basics/what_is_css/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| markdown | [CommonMark: Markdown Reference](https://commonmark.org/help/) | [CC BY-SA 4.0](https://commonmark.org/help/) |
