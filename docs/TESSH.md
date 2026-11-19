# tessh

tessh is Tessera's shell language. You write it like a shell: one command per
line, words separated by spaces, no commas and no call parentheses. Underneath
it is Python. Every line compiles to Python, so the whole Python language and
every Python library work in it. Structure comes from indentation, as in a
Lisp where lines and indents take the place of most parentheses (like wisp or
sweet-expressions). There are infix operators with Python's precedence, shell
operators (`|`, `&&`, `||`, `>`, `>>`, `<`, `2>&1`, `&`), and hygienic macros.

```tessh
# Lines are commands. A name that isn't a Python function runs a program.
ls -la ~/src
git log --oneline -n 5 | head -3

# Barewords are strings, and $ refers to a variable.
name = world
print hello $name                 # hello world

def greet who greeting="hi"
  print "$greeting, $who!"

greet Ben                          # hi, Ben!
greet Ben greeting=hello           # hello, Ben!

# Infix operators. Applying a function binds tighter than any operator.
def fib n
  if $n < 2
    $n
  else
    fib ($n - 1) + fib ($n - 2)

print (fib 20)                     # 6765

# Python, all of it.
import json
from pathlib import Path
data = json.loads (cat package.json)    # (cat …) captures its output
for p in (Path .).glob "*.py"
  print $p.name ($p.stat).st_size
```

It runs as a REPL in a terminal (`tessh`), as a script (`tessh file.tsh`), and
as a Jupyter kernel in Tessera's notebook mode (the default kernel there). The
guide, [tessh/doc/guide.md](../tessh/doc/guide.md), has an example for each
part of the language, and `help TOPIC` shows it from inside tessh. `tessh --check` checks a file or
stdin without running it.

## 1. Words and tokens

Spaces separate words. Brackets `( ) [ ] { }` and quotes always end a word.

| Written | Means |
|---|---|
| `hello`, `-a`, `--all`, `--color=auto`, `*.py`, `foo.txt`, `g++` | a **bareword**. In argument position it's the string `"hello"` |
| `/`, `/etc/hosts`, `.`, `..`, `./run.sh`, `../x`, `~/src` | paths are barewords too |
| `42`, `-7`, `0x1f`, `1_000`, `2.5e3`, `3j` | a number (anything Python reads as a number literal) |
| `"text $name $(expr) ${expr:fmt}"` | a string with interpolation and the usual `\n \t \\ \" \$` escapes |
| `'text'` | a raw string: no escapes, no interpolation |
| `"""…"""`, `'''…'''` | multi-line versions of the two |
| `b"…"`, `r"…"` | bytes, raw (as in Python) |
| `$name` | the variable `name` |
| `$name.attr`, `$name[i]`, `$name.a[1:3].b` | attribute access, indexing and slicing, glued on |
| `$?` | exit status of the last command |
| `True` `False` `None` | Python's constants |
| `name=value` | a keyword argument (when `name` is an identifier and the `=` is glued) |
| `*$xs`, `**$kw` | splat arguments into a call |
| `\name` | run the program `name` even when a Python function `name` exists (`\cat` is the program, not a `cat` you defined) |
| `~`, `~/x` | your home folder (at the start of a word) |
| `:name` | a symbol literal (for macros) |
| `:(form)` | a quasiquoted form (for macros, see §7) |
| `` `f` `` | backticks make any function an infix operator: ``$a `max` $b`` is `max a b` |
| `# …` | a comment, from a `#` at the start of a word to the end of the line |

Glued text joins into one word: `$HOME/src`, `out-$n.txt`, `${name}.txt`. After
a variable, a glued `.name` is attribute access, so write `${file}.txt` when you
mean "file, then `.txt`".

Identifiers (variable, parameter and function names) are Python's:
`[A-Za-z_][A-Za-z0-9_]*`. Program names can be anything.

Operator characters only form an **operator** when the token stands alone with
spaces around it. Glued to other characters, they're part of a word: `-la`,
`*.py`, `a+b` and `c++` are words, while `$a + $b` is addition.

## 2. Lines, indentation and forms

A **form** is a line, or the inside of a pair of parentheses. Lines indented
under a line belong to it: each indented line is one more argument (each one a
form of its own), and for `def`, `if`, `for` and the like they're the body.

```tessh
subprocess.run
  [ls -la]
  check=True
```

is `subprocess.run [ls -la] check=True`.

- A line alone with one bareword is a call or a command: `ls`, `pwd`, `exit`.
  Any other single item on a line is its value: `$x`, `42`, `(f $y)`.
- An indented line that is an argument and has one item is that item (so a
  bareword there is a word, `-it`, not a command); with several items it is a
  form.
- Commas separate only when a space, a closing bracket or the end follows them:
  `[1, 2]` is a list, but `cut -f1,3` passes `-f1,3`.
- `elif`, `else`, `except` and `finally` lines continue the `if`, `for`,
  `while` or `try` above them at the same indentation.
- Lines continue inside open brackets and after a trailing `\`.
- `;` separates forms on one line: `cd /tmp; ls`.
- Indent with spaces. Tabs are an error.
- `@decorator` lines go just before a `def` or `class`.

## 3. Heads, arguments and the two modes

The first item of a form is its **head**. The head decides how to read the rest.

**Special forms and macros** (`def`, `if`, `for`, `=`, …, and any macro you
define) read their parts in their own way (§5).

**Expression mode.** When the head is a Python name the compiler knows about,
the form is an expression. "Knows about" covers local variables and parameters,
names bound anywhere at the top level of the file or cell, names already defined
in the REPL or kernel session, and Python's and tessh's builtins. A dotted name
like `os.path.join` counts when its first part is known. A head that is a
`$variable`, a literal, a bracket group or an operator also makes an expression.
In expression mode:

- Spaced operators split the form, with Python's precedence (§4).
- Each operand made of several items is a call: the head applied to the other
  items. `f a b` is `f("a", "b")`, and `fib ($n - 1) + fib ($n - 2)` is
  `fib(n - 1) + fib(n - 2)`. Function application binds tighter than any
  operator, as in Haskell, so `print $x + 1` is `print(x) + 1`. Write
  `print ($x + 1)`.
- An operand with a single item is its value. A bareword is then a string:
  `x = hello` sets `x` to `"hello"`. The exception is a bareword alone in
  parentheses, which is a call or a command: `x = (pwd)`.
- An operator with nothing on one side becomes a word (`ls *`, `cd -`).

**Command mode.** When the head is a bareword that isn't a known name, the form
runs a program. Its arguments are all **words**:

- Barewords stay strings. Globs (`*`, `?`, `[…]`) expand to the matching file
  names, with no match leaving the word as it is. `~` expands at the start of a
  word.
- `$x` inserts the value as one argument. A list or tuple adds one argument per
  item, as in fish. Quoted strings are never globbed.
- `(form)` inserts the value of the form.
- Only the shell operators are operators (§6). `+`, `*`, `>` and the rest are
  words, except that `>`, `>>` and `<` are redirections.
- `key=value` stays the single word `key=value` (`make CC=clang`).

At run time, a head the compiler didn't know about is looked up again in the
module's names and in builtins before running a program, so names that appear
later (an `exec`, `from x import *`) still work. Then the words are passed as
strings.

Some tessh builtins are **command functions**: `cd`, `pushd`, `popd`, `export`,
`unset`, `exit`, `source`, `which`, `jobs`. They're Python functions that read
their arguments in command mode, so `cd ~/src/*foo*` globs. Your own functions
become command functions with the `@command` decorator.

### What a command returns

A command runs one of two ways, depending on where it appears:

- **Attached**, connected to the terminal, when it stands as a statement (a
  line of its own, or the left side of `&&`, `||` or `;` there), or as the
  condition of `if`, `elif` or `while`. Its value is an `Output` holding no
  text.
- **Captured** anywhere else that a value is used: an argument, the right side
  of `=`, inside `(…)` used as a value. Its value is an `Output` holding the
  program's standard output, minus the final newline.

`Output` is a `str` subclass, so `.split()` and the rest work on it. It also has
`.code`, `.ok` and `.lines`, and its truth is the exit status: `if (grep -q x f)`
and `make && ./run` behave as in a shell. Every command also sets `$?`.

```tessh
branch = git rev-parse --abbrev-ref HEAD
if git diff --quiet
  print clean
files = (ls *.py).lines
```

## 4. Infix operators

From the loosest binding to the tightest:

| Operators | Notes |
|---|---|
| `\|\|` `or` | short-circuit or |
| `&&` `and` | short-circuit and |
| `not` `!` | prefix |
| `\|` | pipe (§6); as in a shell, `a \| b && c` is `(a \| b) && c` |
| `==` `!=` `<` `>` `<=` `>=` `in` `not in` `is` `is not` | comparisons, chained as in Python (`1 < $x < 5`) |
| `` `f` `` and user-defined symbolic operators | left associative by default (§8) |
| `^` `&` | bitwise xor and and (bitwise or: `\|` with a value on its right, §6) |
| `<<` `>>` | shifts (in expression mode only) |
| `+` `-` | |
| `*` `/` `//` `%` `@` | |
| `-x` `+x` | prefix (`- $x`, or glued: `-$x`). `~` is the home folder and the macro unquote, so bitwise not is `operator.invert` |
| `**` | right associative |
| application | `f a b` |
| `.attr` `[i]` | glued postfix |

`:=` (walrus) works inside parentheses. `if c then a else b` is the conditional
expression. An operator at the head of a form applies it to the rest, Lisp
style: `(+ 1 2 3)` is `6` and `(- $x)` is `-x`.

## 5. Special forms

Everything is an expression. A `def` evaluates to the function, an `if` to its
branch's value, a `for` to `None`. A function returns the value of its last form
unless it `return`s earlier or is a generator.

```tessh
x = 5                        # assignment; also $o.attr = …, $d[k] = …
[a b] = [1 2]                # destructuring (or: a, b = 1, 2)
x += 1                       # augmented: += -= *= /= //= %= **= @= &= ^= <<= >>=

def name a b=1 *rest k=2 **kw   # parameters: bare names, defaults glued
  body…
async def fetch url
  await (session.get $url)
fn [x y] ($x + $y)           # lambda; a block body works too
@functools.cache             # decorators
def slow n
  …

class Point Base metaclass=ABCMeta
  def __init__ self x y
    $self.x = $x
    $self.y = $y

if $x > 3                    # elif / else
  …
y = if $ok then yes else no
while $n > 0                 # and else
  n -= 1
for [k v] in ($d.items)      # also: for k, v in …; async for; else
  print $k $v
with (open $path) as f, (lock) as l
  …
try
  risky
except ValueError as e
  print $e
else
  …
finally
  …
match $cmd                   # case patterns: literals, names, _, [a b *rest],
  case [go $dir]             # barewords match strings, $name captures; also
    print $dir               # {key: p}, (Class p) / Class x=p, Mod.CONST, p | q,
                             # p as $n, [a *$rest], guards: case [$n] if $n > 0
  case _
    print "?"

return x ; yield x ; yield from x ; await x
raise ValueError "bad" ; raise X from Y
assert ($x > 0) "x must be positive"
del $d[k] ; pass ; break ; continue
global n ; nonlocal n
import os sys ; import numpy as np ; from os.path import join exists
do                           # several forms as one; value of the last
  …
```

Where Python expects a class or a name (the type after `except`, class bases,
`import`, `global`, decorators) a bareword is the name. Elsewhere a bareword is
a string, so pass a class as a value with `$`: `isinstance $x $int`.

Comprehensions are written as in Python, inside brackets:
`[$x * 2 for x in $xs if $x > 1]`, `{$k: $v for [k v] in $pairs}`,
`(sum ($x for x in $xs))`.

Literals: `[1 2 3]` is a list (or `[1, 2, 3]`: once there are commas, each
comma-separated part is an expression, as in `[$a + 1, 2]`). `{a: 1 b: 2}` is
`{"a": 1, "b": 2}` (barewords as keys are strings; `{$k: 1}` uses the
variable). `{1 2 3}` is a set, `()` an empty tuple, and `(1, 2)` a tuple.

### From Clojure

- `.name obj args…` calls a method (`$obj.name args…`) and `.-name obj` reads
  a field, only at a call's head or a pipe stage's (where the input is the
  object). `(.name)` and `(.-name)` alone are functions of the object. A bare
  `.name` argument stays a word (`ls .config`).
- `-> x (f a) g` is `(g (f x a))`, and `->> x (f a) g` is `(g (f a x))`. Steps
  may be indented lines. `some->` and `some->>` stop at `None`. `as-> x n step…`
  rebinds `n` to each step's result. `doto obj step…` calls each step with the
  object first and returns the object.
- `:name` (a Sym form) called on a value is `x.get("name", default)` for a
  Mapping, else `getattr(x, "name", default)`; `default` is None.
- `{a b c=0 **rest} = value` assigns from keys (attributes for a non-mapping);
  a missing key without a default raises KeyError.
- `fn body` without `[params]`: parameters from `$%`/`$%1`…`$%9` and `$%&`
  (varargs), hygienic. Not nestable.
- `#_` drops the next form; first on a line, the line and its block. (`#__…`
  is still a comment.)
- Core functions, installed in every namespace (a name you define wins):
  `partial identity constantly comp complement juxt inc dec first second rest
  butlast take drop take_while drop_while some every keep mapcat frequencies
  group_by partition partition_all partition_by interpose interleave distinct
  flatten iterate repeatedly zipmap select_keys merge_with get_in assoc_in
  update_in`. Function first, collection last; lists, except endless
  `iterate`/`repeatedly`. `last` hides the program of that name (`\last` runs it).

### From Hy

- `import name` also finds `name.tsh` (and a package's `__init__.tsh`) on
  sys.path, through a meta-path finder placed before Python's path finder;
  the first folder with the name decides, and a `.py` module or package there
  is Python's. `""` (the folder you're in) is on the path, and a script's own
  folder is put first. Python code tessh runs imports tessh modules too.
- `require module [name…]` imports the module while compiling and copies its
  macros (all, or the ones named) and, with no names, its `defop` operators
  into this namespace's compiler state. It compiles to `pass`.
- `let a=1 b=(f $a) …` with a block, or a form after the bindings: each name
  gets a fresh hygiene mark in the block (bound in order), so outside it the
  same name is untouched. Its value is the block's last form's.
- `py "code"`: the string parsed as Python, as an expression if it is one,
  else as statements (common indentation removed). Single or triple quotes:
  an interpolating string is an error.

## 6. Shell operators

| Written | Meaning |
|---|---|
| `a \| b` | pipe `a`'s output into `b` |
| `a && b`, `a \|\| b` | run `b` if `a` succeeded (failed) |
| `a ; b` | one after the other |
| `cmd &` | in the background (the value is a `Job`; `jobs` lists them) |
| `>f` `>>f` `<f` `2>f` `2>>f` `&>f` `2>&1` `>&2` | redirections. Glued to the file they work anywhere. Spaced (`> f`) they work in command mode only |

Each stage of a pipe can be:

- a **program**: its standard input is the stream before it.
- a **Python call** (`f a b`, or `f` alone): called with what came before added
  as its *last* argument. That's the program's text, or the earlier Python
  value as it is. `ls | str.upper`, `cat data.json | json.loads`.
- a **value**, first only: fed to the next stage. `$text | wc -l`. A `str` or
  `bytes` value is written as it is, any other iterable one line per item, and
  anything else as `str(value)`.

`$a | $b`, where the right side is a value rather than a program or function
name, is Python's bitwise or, so `$set1 | $set2` and `$d1 | $d2` work.

Redirections apply to Python calls too: `print hi >out.txt`.

## 7. Macros

Macros run while the code compiles. They take forms as data and return forms.
They're **hygienic**:

1. Names a macro's template binds (with `=`, `def`, `for`, parameters and so
   on) are renamed, so they never capture or clobber the user's variables.
2. Free names in a macro's template mean what they meant where the macro was
   defined, even if the user has a local variable with the same name.

```tessh
defmacro swap a b
  quasi
    tmp = $~a
    ~a = $~b
    ~b = $tmp

tmp = 1 ; other = 2
swap tmp other           # the macro's tmp is not the user's tmp
print $tmp $other        # 2 1

defmacro unless cond:expr *body
  quasi
    if (not ~cond)
      ~@body

unless ($x > 3)
  print small
```

- `defmacro name params…` with a body. Parameters are `name`, which takes one
  item of the line, `*name`, which takes the rest of the items (including the
  indented lines), and `name:expr`, which takes the rest of the line as one
  infix expression.
- `quasi` with an indented template, or `:(form)` inline, builds forms. Inside
  it, `~x` inserts a value, `~(expr)` inserts an expression's value, `~@xs`
  splices a list, and `$~x` makes a variable reference from the symbol in `x`. A macro-time `int`, `str` and so on becomes a literal.
- `quote form` and `:name` give a form without evaluating it.
- Macros get forms as `Form` objects. `$f.kind` is the form's kind: `Sym`,
  `Var`, `Num`, `Str`, `IStr`, `Word`, `Op`, `Group`, `Line`, `Attr`, `Index`,
  `KwArg`, `Splat` and the like. The fields are `name`, `items` (and indexing,
  `len`), `children`, `value`, `inner` (the form inside a `$`, `~` or `.attr`),
  `op` and `bracket`. `str form` shows a form as tessh. The constructors are
  `Sym`, `Var`, `Line`, `Group`, `Op`, `Attr`, `KwArg` and `Splat` in
  `__tessh_forms__`. Plain values a macro returns (numbers, strings, lists,
  dicts) become literals.
- The helper `var sym` makes `$sym`, `sym "text"` makes a symbol, and
  `capture name` makes a symbol in the *user's* context (on purpose
  unhygienic, for anaphoric macros).
- `gensym "hint"` gives a fresh symbol, though hygiene makes it rarely needed.
- `macroexpand form` shows an expansion.

Hygiene is implemented with marks, following Kohlbecker, Friedman, Felleisen and
Duba. Each expansion gets a fresh mark that is toggled on the macro's input and
again on its output, so only symbols the macro introduced keep the mark. A binder
with marks gets a fresh Python name. A marked reference resolves to a marked
binder in scope if there is one, and otherwise to the name where the macro was
defined.

## 8. Custom operators

```tessh
def concat a b
  $a + $b
print ([1] `concat` [2])         # [1, 2]

defop <> 6 left concat           # a symbolic operator: precedence, associativity, function
print ([1] <> [2] <> [3])
```

Precedence levels run from 1 (`||`) to 13 (`**`): 1 `||`, 2 `&&`, 3 `not`, 4 `|`, 5
comparisons, 6 backtick and custom, 7 `^`, 8 `&`, 9 shifts, 10 `+ -`, 11 `* /`,
12 prefix, 13 `**`.
Associativity is `left`, `right` or `none`.

## 9. Environment, folders and jobs

- `$HOME`, `$PATH`, …: a `$name` that isn't a Python name falls back to the
  environment variable.
- `$env` is the environment as a mapping: `$env.PATH`, `$env["X"]`,
  `$env.X = 1`.
- `export NAME=value` and `unset NAME`.
- `cd dir` (`cd` goes home and `cd -` goes back), `pushd`, `popd`.
- A trailing `&` makes a background `Job`; `jobs` lists them.
- `$argv` holds the script's arguments.
- `source file.tsh` runs a tessh file in the current namespace. The REPL first
  runs `~/.config/tessh/rc.tsh`.

## 10. Dependencies with uv (PEP 723)

`need pkg "spec" …` (a command builtin) installs the packages with
`uv pip install --target` into a cache folder keyed by the set and the
Python version, then puts it on `sys.path`; `tessh --with PKG[,PKG…]` before
the other arguments does the same first. Both share the cache with the
PEP 723 block below.

A script or notebook cell can name the PyPI packages it needs in an inline
metadata block, the way `uv run` reads Python scripts:

```tessh
# /// script
# dependencies = ["requests", "rich>=13"]
# ///
import requests
print (requests.get "https://example.com").status_code
```

tessh installs them with uv, into a folder cached by the set of packages and
built for the running Python, then puts that folder on `sys.path`. uv comes
from `PATH`, or from Nix when it isn't installed. `requires-python` is checked,
and tessh warns when it isn't met.

## 11. Running it

- `tessh` starts the REPL. A line runs as soon as it's complete, and open
  brackets or strings run on to the next line. A line whose head takes a block
  (`def`, `if`, `for`, `x = if …`, or a macro) indents the next line and keeps
  reading until a blank line. Enter keeps the indentation. An `else`, `elif`,
  `except` or `finally` typed deeper than its `if` or `try` moves back to it
  on Enter, and Enter again goes on. Backspace in the leading spaces removes
  a level, and Alt+Enter is a new line whatever the input. Input is
  highlighted as it's typed, and Tab completes names, programs and paths. The REPL emits shell-integration marks: OSC 133 for prompts and
  commands, OSC 7 for the folder, and Tessera's command line and environment
  files. Folder and exports handed over from Tessera's other kernels are
  applied before each prompt.
- `tessh file.tsh args…` runs a script (`#!/usr/bin/env tessh` works).
- `tessh -c 'code'` runs a string.
- `tessh --check [file]` checks without running anything except macro
  definitions. It prints `line:col: message` per problem and exits 1 if there
  are any. `--check --json` prints `[{"line", "col", "end_col", "message"}]`.
- `tessh --py file.tsh` prints the Python it compiles to.
- `tessh --kernel -f conn.json` is the Jupyter kernel. In Tessera it's the
  **tessh** kernel: a notebook's language, or a language picked in the input
  editor. Folders and exported variables go between it and the pane's shell
  as they do with the other kernels. Output, its own and that of the programs
  it runs, shows as it comes. Values show with their rich representations:
  `_repr_html_`, images, `_repr_mimebundle_`, and matplotlib figures as PNGs.
  `display x` shows a value partway through a cell, and `input` asks the
  notebook.
- Errors point at tessh lines. The compiler gives every Python node its tessh
  line and column, so Python's tracebacks show the tessh source with carets
  under the right columns, in a script, at the REPL or in a cell.

## 12. Grammar summary

```
file      := line*
line      := INDENT? item* ('#' comment)? NEWLINE  (children: more-indented lines)
item      := term | OP | REDIR
term      := atom postfix*
atom      := NUMBER | STRING | WORD | '$' NAME | '$?' | '\' WORD | ':' NAME
           | ':(' form ')' | '`' NAME '`' | NAME '=' term | '*' term | '**' term
           | '(' items ')' | '[' items ']' | '{' items '}'
postfix   := '.' NAME | '[' items ']'          (glued, after a variable or bracket)
WORD      := (wordchar | '$' NAME | '${' expr '}' | '$(' form ')')+
```

## 13. Implementation

tessh is a Rust program (`tessh/`) that embeds Python with PyO3:

- `reader.rs`: the grammar, written with chumsky (a Rust parser-combinator
  library). It reads logical lines: indentation, then items, with groups and
  strings that may span lines and `${…}` read on their own. Blocks are made
  from the lines' indentation.
- `compiler.rs`: forms to Python's AST, built as Python `ast` objects with
  tessh positions, then compiled and run one top-level form at a time.
- `pyforms.rs`: forms as Python objects for macros, and quasi templates.
- `runtime.rs`: the `__tessh__` module: commands, pipes, redirections, job
  control, globs, the environment and the builtins.
- `repl.rs` (rustyline) and `kernel.rs` (the Jupyter protocol on zeromq).

`tree-sitter-tessh/` is the same language made flat, for highlighting and
syntax errors in Tessera's editors. Tessera runs `tessh --check --json` when
typing pauses, for what the flat grammar can't see.
