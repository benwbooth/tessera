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
as a Jupyter kernel in Tessera's notebook mode. `tessh --check` checks a file or
stdin without running it.

## 1. Words and tokens

Spaces separate words. Brackets `( ) [ ] { }` and quotes always end a word.

| Written | Means |
|---|---|
| `hello`, `-la`, `*.py`, `foo.txt`, `g++` | a **bareword**. In argument position it's the string `"hello"` |
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
| `^name` | run the program `name` even when a Python name `name` exists |
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
`.code`, `.err` and `.ok`, and its truth is the exit status: `if (grep -q x f)`
and `make && ./run` behave as in a shell. Every command also sets `$?`.

```tessh
branch = git rev-parse --abbrev-ref HEAD
if git diff --quiet
  print clean
files = (ls *.py).split
```

## 4. Infix operators

From the loosest binding to the tightest:

| Operators | Notes |
|---|---|
| `\|` | pipe (§6) |
| `\|\|` `or` | short-circuit or |
| `&&` `and` | short-circuit and |
| `not` | prefix |
| `==` `!=` `<` `>` `<=` `>=` `in` `not in` `is` `is not` | comparisons, chained as in Python (`1 < $x < 5`) |
| `` `f` `` and user-defined symbolic operators | left associative by default (§8) |
| `^` `&` | bitwise xor and and (bitwise or: `\|` with a value on its right, §6) |
| `<<` `>>` | shifts (in expression mode only) |
| `+` `-` | |
| `*` `/` `//` `%` `@` | |
| `-x` `+x` `~x` | prefix (`- $x`, or glued: `-$x`) |
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
  case [go dir]              # {key: p}, Class a b / Class x=p, p | q, p as n,
    print $dir               # and guards: case [n] if $n > 0
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

Comprehensions are written as in Python, inside brackets:
`[$x * 2 for x in $xs if $x > 1]`, `{$k: $v for [k v] in $pairs}`,
`(sum ($x for x in $xs))`.

Literals: `[1 2 3]` is a list (or `[1, 2, 3]`: once there are commas, each
comma-separated part is an expression, as in `[$a + 1, 2]`). `{a: 1 b: 2}` is
`{"a": 1, "b": 2}` (barewords as keys are strings; `{$k: 1}` uses the
variable). `{1 2 3}` is a set, `()` an empty tuple, and `(1, 2)` a tuple.

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
- Forms are `tessh.forms` objects: `Sym`, `Var`, `Op`, `Str`, `Num`, `Word`,
  `Group` (`()`, `[]`, `{}`) and `Line`. The helper `var sym` makes `$sym`,
  `sym "text"` makes a symbol, and `capture name` makes a symbol in the
  *user's* context (on purpose unhygienic, for anaphoric macros).
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

Precedence levels run from 1 (`|`) to 13 (`**`); backtick operators sit at 6.
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

- `tessh` starts the REPL. A line runs as soon as it's complete. A line whose
  head takes a block (`def`, `if`, `for`, …, or a macro) keeps reading until a
  blank line. The REPL emits shell-integration marks: OSC 133 for prompts and
  commands, OSC 7 for the folder, and Tessera's command line and environment
  files. Folder and exports handed over from Tessera's other kernels are
  applied before each prompt.
- `tessh file.tsh args…` runs a script (`#!/usr/bin/env tessh` works).
- `tessh -c 'code'` runs a string.
- `tessh --check [file]` checks without running anything except macro
  definitions. It prints `line:col: message` per problem and exits 1 if there
  are any. `--check --json` prints `[{"line", "col", "end_col", "message"}]`.
- `tessh --py file.tsh` prints the Python it compiles to.
- `python -m tessh.kernel -f conn.json` is the Jupyter kernel (kernelspec
  `tessh`). In Tessera it's in the input editor's language list as **tessh**.

## 12. Grammar summary

```
file      := line*
line      := INDENT? item* ('#' comment)? NEWLINE  (children: more-indented lines)
item      := term | OP | REDIR
term      := atom postfix*
atom      := NUMBER | STRING | WORD | '$' NAME | '$?' | '^' WORD | ':' NAME
           | ':(' form ')' | '`' NAME '`' | NAME '=' term | '*' term | '**' term
           | '(' items ')' | '[' items ']' | '{' items '}'
postfix   := '.' NAME | '[' items ']'          (glued, after a variable or bracket)
WORD      := (wordchar | '$' NAME | '${' expr '}' | '$(' form ')')+
```

The reader (`tessh/reader.py`) is built with the parsy parser-combinator
library: one parser for the characters of a line's items, and one for the
layout, which runs over the list of lines and makes blocks from their
indentation. The tree-sitter grammar (`tree-sitter-tessh/`) is the same
language, made flat for highlighting.
