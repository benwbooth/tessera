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

When a page's own site fails or hasn't answered in 5 seconds, the Internet
Archive's latest copy of it is asked too (`web.archive.org/web/2id_/URL`:
the page as it was saved, without the archive's toolbar), and the first to
answer is used. A page taken from the archive says so at its top, with the
day it was saved; **Refresh** later fetches it from its site again. The
archive turns away bursts, so at most two of its pages are asked for at a
time.

C++ has two guides, LearnCpp.com (the tutorial, shown first) and
cppreference (the reference): chips at the top of the page switch between
them, and the tab keeps the choice (`guide:cpp/cppreference`). Each is kept
on its own (`learncpp.md`, `cppreference.md`).

The Guide chip, and Shift+F1 in the editor, open the guide at the selected
word or the word at the cursor (found in it once it's there). An open
guide takes a new theme, colours or font at once.

Code is highlighted with Tessera's grammars, and Helix's for the rest.
A Helix grammar is compiled code, so each is first tried in a child
process on sample text (strings, comments, heredocs...); one that crashes
or hangs there is never loaded, and its code is shown plain (Helix's HCL
grammar crashes on any string). The verdicts are kept in
`~/.local/share/tessera/helix-runtime/checked.tsv`. A highlights rule this
tree-sitter can't read is left out and the rest kept (Helix's Solidity
rules had one).

Preferences › General › Language guides › **Download All** fetches every
guide now, for reading offline (about 100 MB; the ones already kept are
skipped). A guide kept over a month, or taken from the archive over a day
ago, is shown at once and fetched again in the background for next time.
Files no guide uses any more are removed.

**All Guides** beside the find box (or Ctrl+Enter in it) looks for the
words in every kept guide: the hits under each guide's title, with the
heading they're under; one opens that guide at the words.

Why fetched and not bundled: together the documents are tens of megabytes,
and their licenses differ (GNU FDL, CC BY-SA, and some that allow reading
but not redistribution). Fetched on the reader's own machine, nothing is
redistributed with Tessera. Each kept file starts with its source, its
license and the date it was fetched.

All of them are checked by one test that fetches, converts and draws
every guide as the app does and flags any that fail, take over a minute,
or come out without contents or highlighted code:
`cargo test guides::sweep -- --ignored --nocapture` (with a scratch
`XDG_STATE_HOME`; `SWEEP_ONLY=lua,rust` for some).

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
| cpp, c++, cxx | [LearnCpp.com](https://www.learncpp.com/) | [© LearnCpp.com; read from the site, not redistributed](https://www.learncpp.com/about/) |
| cpp, c++, cxx | [C++ language (cppreference.com)](https://en.cppreference.com/w/cpp/language/basic_concepts) | [CC BY-SA 3.0](https://en.cppreference.com/w/Cppreference:Copyright/CC-BY-SA) |
| java | [Learn Java: Language Basics (dev.java)](https://dev.java/learn/language/constructs/basics/) | [© Oracle; read from dev.java, not redistributed](https://www.oracle.com/legal/copyright.html) |
| ruby | [Ruby Syntax (docs.ruby-lang.org)](https://docs.ruby-lang.org/en/master/syntax_rdoc.html) | [Ruby License or BSD-2-Clause](https://www.ruby-lang.org/en/about/license.txt) |
| perl | [Perl documentation (perlintro and friends)](https://perldoc.perl.org/perlintro) | [Artistic License or GPL](https://perldoc.perl.org/perlartistic) |
| php, php-only | [PHP Manual: Getting Started and Language Reference](https://www.php.net/manual/en/introduction.php) | [CC BY 3.0](https://www.php.net/manual/en/copyright.php) |
| r | [An Introduction to R](https://cran.r-project.org/doc/manuals/r-release/R-intro.html) | [Verbatim copying permitted (R Core Team)](https://cran.r-project.org/doc/manuals/r-release/R-intro.html) |
| julia | [The Julia Manual](https://docs.julialang.org/en/v1/manual/getting-started/) | [MIT](https://github.com/JuliaLang/julia/blob/master/LICENSE.md) |
| elixir, iex | [Elixir Getting Started](https://raw.githubusercontent.com/elixir-lang/elixir/main/lib/elixir/pages/getting-started/introduction.md) | [Apache-2.0](https://github.com/elixir-lang/elixir/blob/main/LICENSE) |
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
| ocaml, ocaml_interface | [The OCaml Manual: An Introduction to OCaml](https://ocaml.org/manual/latest/coreexamples.html) | [CC BY-SA 4.0](https://ocaml.org/manual/latest/) |
| racket | [The Racket Guide: Racket Essentials](https://docs.racket-lang.org/guide/to-scheme.html) | [MIT or Apache-2.0](https://download.racket-lang.org/license.html) |
| scheme, guile | [Guile: Hello Guile! and Hello Scheme!](https://www.gnu.org/software/guile/manual/guile.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| common-lisp, lisp, commonlisp | [The Common Lisp Cookbook](https://raw.githubusercontent.com/LispCookbook/cl-cookbook/master/getting-started.md) | [See the Cookbook's license](https://github.com/LispCookbook/cl-cookbook) |
| dart | [Dart Language Tour](https://dart.dev/language) | [CC BY 4.0 (code BSD-3-Clause)](https://dart.dev/terms) |
| swift | [The Swift Programming Language](https://raw.githubusercontent.com/swiftlang/swift-book/main/TSPL.docc/GuidedTour/GuidedTour.md) | [Apache-2.0 with Runtime Library Exception](https://github.com/swiftlang/swift-book/blob/main/LICENSE.txt) |
| zig | [Zig Language Reference](https://ziglang.org/documentation/master/) | [MIT](https://github.com/ziglang/zig/blob/master/LICENSE) |
| nim | [Nim Tutorial](https://nim-lang.org/docs/tut1.html) | [MIT](https://github.com/nim-lang/Nim/blob/devel/copying.txt) |
| crystal | [Crystal Tutorials: Language Introduction](https://raw.githubusercontent.com/crystal-lang/crystal-book/master/docs/tutorials/basics/10_hello_world.md) | [Apache-2.0](https://github.com/crystal-lang/crystal-book/blob/master/LICENSE) |
| v | [V Documentation](https://raw.githubusercontent.com/vlang/v/master/doc/docs.md) | [MIT](https://github.com/vlang/v/blob/master/LICENSE) |
| fsharp, f# | [F#: a tour and the language reference](https://raw.githubusercontent.com/dotnet/docs/main/docs/fsharp/tour.md) | [CC BY 4.0](https://github.com/dotnet/docs/blob/main/LICENSE) |
| c-sharp, csharp, c#, cs | [C#: a tour and the fundamentals](https://raw.githubusercontent.com/dotnet/docs/main/docs/csharp/tour-of-csharp/overview.md) | [CC BY 4.0](https://github.com/dotnet/docs/blob/main/LICENSE) |
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
| scss, sass | [Sass: basics and documentation](https://sass-lang.com/guide/) | [MIT](https://github.com/sass/sass-site/blob/main/LICENSE) |
| make | [GNU make Manual](https://www.gnu.org/software/make/manual/make.html) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| cmake | [CMake Tutorial](https://cmake.org/cmake/help/latest/guide/tutorial/index.html) | [BSD-3-Clause](https://cmake.org/licensing/) |
| typst | [Typst Tutorial](https://typst.app/docs/tutorial/) | [Apache-2.0](https://github.com/typst/typst/blob/main/LICENSE) |
| dot, graphviz | [The DOT Language (Graphviz)](https://graphviz.org/doc/info/lang.html) | [EPL-2.0](https://graphviz.org/license/) |
| latex, tex | [Learn LaTeX](https://raw.githubusercontent.com/learnlatex/learnlatex.github.io/main/en/lesson-01.md) | [CC BY-SA 4.0](https://github.com/learnlatex/learnlatex.github.io/blob/main/LICENSE.md) |
| openscad | [The OpenSCAD Language (OpenSCAD User Manual)](https://en.wikibooks.org/wiki/OpenSCAD_User_Manual/The_OpenSCAD_Language) | [CC BY-SA 4.0 (Wikibooks)](https://en.wikibooks.org/wiki/OpenSCAD_User_Manual) |
| dockerfile | [Dockerfile reference](https://docs.docker.com/reference/dockerfile/) | [Apache-2.0](https://github.com/docker/docs/blob/main/LICENSE) |
| hcl | [HCL native syntax and OpenTofu configuration](https://raw.githubusercontent.com/hashicorp/hcl/main/hclsyntax/spec.md) | [MPL-2.0](https://github.com/hashicorp/hcl/blob/main/LICENSE) |
| graphql | [Learn GraphQL (graphql.org)](https://graphql.org/learn/introduction) | [MIT](https://github.com/graphql/graphql.github.io/blob/source/LICENSE) |
| proto | [Protocol Buffers: Language Guide (proto 3)](https://raw.githubusercontent.com/protocolbuffers/protocolbuffers.github.io/main/content/programming-guides/proto3.md) | [CC BY 4.0](https://github.com/protocolbuffers/protocolbuffers.github.io/blob/main/LICENSE) |
| nginx | [nginx: Beginner's Guide and How nginx processes a request](https://nginx.org/en/docs/beginners_guide.html) | [BSD-2-Clause](https://nginx.org/LICENSE) |
| just | [The just manual](https://raw.githubusercontent.com/casey/just/master/README.md) | [CC0-1.0](https://github.com/casey/just/blob/master/LICENSE) |
| regex | [MDN: Regular expressions](https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/javascript/guide/regular_expressions/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| xml | [MDN: XML introduction](https://raw.githubusercontent.com/mdn/content/main/files/en-us/web/xml/guides/xml_introduction/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| wat, wast | [MDN: Understanding WebAssembly text format](https://raw.githubusercontent.com/mdn/content/main/files/en-us/webassembly/guides/understanding_the_text_format/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| elm | [An Introduction to Elm](https://raw.githubusercontent.com/evancz/guide.elm-lang.org/master/book/core_language.md) | [BSD-3-Clause](https://github.com/evancz/guide.elm-lang.org/blob/master/LICENSE) |
| purescript | [PureScript language reference](https://raw.githubusercontent.com/purescript/documentation/master/language/Syntax.md) | [CC BY-NC-SA 3.0; read from the repository, not redistributed](https://github.com/purescript/documentation/blob/master/LICENSE.md) |
| solidity | [Solidity documentation](https://docs.soliditylang.org/en/latest/introduction-to-smart-contracts.html) | [GPL-3.0](https://github.com/ethereum/solidity/blob/develop/LICENSE.txt) |
| glsl | [The OpenGL Shading Language 4.60](https://registry.khronos.org/OpenGL/specs/gl/GLSLangSpec.4.60.html) | [© Khronos Group; read from the registry, not redistributed](https://registry.khronos.org/OpenGL/specs/gl/GLSLangSpec.4.60.html) |
| wgsl, wesl | [Tour of WGSL](https://google.github.io/tour-of-wgsl/) | [Apache-2.0](https://github.com/google/tour-of-wgsl/blob/main/LICENSE) |
| verilog | [Programmable Logic: Verilog (Wikibooks)](https://en.wikibooks.org/wiki/Programmable_Logic/Verilog_Module_Structure) | [CC BY-SA 4.0](https://en.wikibooks.org/wiki/Programmable_Logic/Verilog) |
| vhdl | [Programmable Logic: VHDL, and VHDL for FPGA Design (Wikibooks)](https://en.wikibooks.org/wiki/Programmable_Logic/VHDL_General_Syntax) | [CC BY-SA 4.0](https://en.wikibooks.org/wiki/Programmable_Logic/VHDL) |
| vue | [Vue.js Guide: Essentials](https://raw.githubusercontent.com/vuejs/docs/main/src/guide/essentials/application.md) | [MIT](https://github.com/vuejs/docs/blob/main/LICENSE) |
| svelte | [Svelte documentation](https://svelte.dev/docs/svelte/overview) | [MIT](https://github.com/sveltejs/svelte/blob/main/LICENSE.md) |
| astro | [Astro basics](https://raw.githubusercontent.com/withastro/docs/main/src/content/docs/en/basics/project-structure.mdx) | [MIT](https://github.com/withastro/docs/blob/main/LICENSE) |
| kdl | [The KDL Document Language, version 2](https://raw.githubusercontent.com/kdl-org/kdl/main/draft-marchan-kdl2.md) | [CC BY-SA 4.0](https://github.com/kdl-org/kdl/blob/main/LICENSE.md) |
| gdscript | [GDScript reference (Godot)](https://docs.godotengine.org/en/stable/tutorials/scripting/gdscript/gdscript_basics.html) | [CC BY 3.0](https://docs.godotengine.org/en/stable/about/license.html) |
| gitignore | [Git documentation: gitignore](https://git-scm.com/docs/gitignore) | [GPL-2.0](https://git-scm.com/about/free-and-open-source) |
| gitattributes | [Git documentation: gitattributes](https://git-scm.com/docs/gitattributes) | [GPL-2.0](https://git-scm.com/about/free-and-open-source) |
| git-config | [Git documentation: git config](https://git-scm.com/docs/git-config) | [GPL-2.0](https://git-scm.com/about/free-and-open-source) |
| gitcommit | [Git documentation: git commit](https://git-scm.com/docs/git-commit) | [GPL-2.0](https://git-scm.com/about/free-and-open-source) |
| git-rebase | [Git documentation: git rebase](https://git-scm.com/docs/git-rebase) | [GPL-2.0](https://git-scm.com/about/free-and-open-source) |
| mermaid | [Mermaid diagram syntax](https://raw.githubusercontent.com/mermaid-js/mermaid/develop/packages/mermaid/src/docs/syntax/flowchart.md) | [MIT](https://github.com/mermaid-js/mermaid/blob/develop/LICENSE) |
| meson | [The Meson Build system: Tutorial and Syntax](https://raw.githubusercontent.com/mesonbuild/meson/master/docs/markdown/Tutorial.md) | [Apache-2.0](https://github.com/mesonbuild/meson/blob/master/COPYING) |
| qmljs, qml | [QML Reference (Qt 6)](https://doc.qt.io/qt-6/qtqml-syntax-basics.html) | [GNU FDL 1.3](https://doc.qt.io/qt-6/licensing.html) |
| jinja2, jinja | [Jinja Template Designer Documentation](https://jinja.palletsprojects.com/en/stable/templates/) | [BSD-3-Clause](https://github.com/pallets/jinja/blob/main/LICENSE.txt) |
| twig | [Twig for Template Designers](https://twig.symfony.com/doc/3.x/templates.html) | [BSD-3-Clause](https://github.com/twigphp/Twig/blob/3.x/LICENSE) |
| cairo | [The Cairo Programming Language](https://book.cairo-lang.org/print.html) | [MIT or Apache-2.0](https://github.com/cairo-book/cairo-book/blob/main/LICENSE) |
| hurl | [Hurl documentation](https://hurl.dev/docs/tutorial/your-first-hurl-file.html) | [Apache-2.0](https://github.com/Orange-OpenSource/hurl/blob/master/LICENSE) |
| caddyfile | [The Caddyfile](https://raw.githubusercontent.com/caddyserver/website/master/src/docs/markdown/caddyfile-tutorial.md) | [Apache-2.0](https://github.com/caddyserver/website/blob/master/LICENSE) |
| rst | [reStructuredText: Primer and Quick Start](https://docutils.sourceforge.io/docs/user/rst/quickstart.html) | [Public domain (Docutils) and BSD-2-Clause (Sphinx)](https://docutils.sourceforge.io/COPYING.html) |
| json5 | [JSON5](https://raw.githubusercontent.com/json5/json5/main/README.md) | [MIT](https://github.com/json5/json5/blob/main/LICENSE.md) |
| gherkin | [Gherkin reference (Cucumber)](https://raw.githubusercontent.com/cucumber/docs/main/content/docs/gherkin/reference.md) | [MIT](https://github.com/cucumber/docs/blob/main/LICENSE) |
| agda | [Agda: A Taste of Agda and the language](https://agda.readthedocs.io/en/latest/getting-started/a-taste-of-agda.html) | [MIT](https://github.com/agda/agda/blob/master/LICENSE) |
| gotmpl | [Go text/template](https://pkg.go.dev/text/template) | [BSD-3-Clause](https://go.dev/LICENSE) |
| gomod, gowork | [Go Modules Reference](https://go.dev/ref/mod) | [CC BY 4.0](https://go.dev/copyright) |
| cel | [CEL language definition](https://raw.githubusercontent.com/google/cel-spec/master/doc/langdef.md) | [Apache-2.0](https://github.com/google/cel-spec/blob/master/LICENSE) |
| org | [The compact Org guide](https://orgmode.org/guide/) | [GNU FDL 1.3](https://www.gnu.org/licenses/fdl-1.3.html) |
| jsdoc | [Use JSDoc](https://jsdoc.app/about-getting-started) | [CC BY-SA 3.0](https://github.com/jsdoc/jsdoc.github.io/blob/master/LICENSE) |
| thrift | [Thrift interface description language](https://thrift.apache.org/docs/idl) | [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) |
| capnp | [Cap'n Proto: Schema Language](https://capnproto.org/language.html) | [MIT](https://github.com/capnproto/capnproto/blob/v2/LICENSE) |
| bicep | [Bicep file structure and syntax](https://learn.microsoft.com/en-us/azure/azure-resource-manager/bicep/file) | [CC BY 4.0](https://github.com/MicrosoftDocs/azure-docs/blob/main/LICENSE) |
| smithy | [Smithy IDL](https://smithy.io/2.0/spec/idl.html) | [Apache-2.0](https://github.com/smithy-lang/smithy/blob/main/LICENSE) |
| tlaplus | [Learn TLA+](https://learntla.com/core/index.html) | [© Hillel Wayne; read from the site, not redistributed](https://learntla.com/) |
| ron | [RON: Rusty Object Notation](https://raw.githubusercontent.com/ron-rs/ron/master/README.md) | [MIT or Apache-2.0](https://github.com/ron-rs/ron) |
| rescript | [ReScript Language Manual](https://rescript-lang.org/docs/manual/introduction) | [MIT](https://github.com/rescript-lang/rescript-lang.org/blob/master/LICENSE) |
| pug | [Pug language reference](https://pugjs.org/language/attributes.html) | [MIT](https://github.com/pugjs/pug/blob/master/packages/pug/LICENSE) |
| yara | [Writing YARA rules](https://yara.readthedocs.io/en/stable/writingrules.html) | [BSD-3-Clause](https://github.com/VirusTotal/yara/blob/master/COPYING) |
| ledger | [Ledger 3 manual](https://www.ledger-cli.org/3.0/doc/ledger3.html) | [BSD-3-Clause (Ledger)](https://github.com/ledger/ledger/blob/master/LICENSE.md) |
| pest | [pest: Grammar syntax](https://raw.githubusercontent.com/pest-parser/book/master/src/grammars/syntax.md) | [MIT or Apache-2.0](https://github.com/pest-parser/book) |
| mojo | [Mojo Manual](https://docs.modular.com/mojo/manual/basics) | [© Modular; read from the site, not redistributed](https://docs.modular.com/) |
| json | [MDN: Working with JSON](https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/scripting/json/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| yaml, yml | [YAML 1.2.2 Specification](https://yaml.org/spec/1.2.2/) | [Free to copy (the YAML spec's own terms)](https://yaml.org/spec/1.2.2/) |
| toml | [TOML v1.0.0](https://raw.githubusercontent.com/toml-lang/toml/1.0.0/toml.md) | [MIT](https://github.com/toml-lang/toml/blob/main/LICENSE) |
| html | [MDN: Structuring content with HTML](https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/structuring_content/basic_html_syntax/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| css | [MDN: CSS styling basics](https://raw.githubusercontent.com/mdn/content/main/files/en-us/learn_web_development/core/styling_basics/what_is_css/index.md) | [CC BY-SA 2.5](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Attrib_copyright_license) |
| markdown, markdown_inline | [GitHub Docs: Basic writing and formatting syntax](https://raw.githubusercontent.com/github/docs/main/content/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax.md) | [CC BY 4.0](https://github.com/github/docs/blob/main/LICENSE) |
