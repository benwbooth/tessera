# tessh guide
<!-- help: guide tessh intro overview start -->

tessh is a shell language that is Python underneath. You write it like a
shell: one command per line, words separated by spaces, no commas and no
call parentheses. Every line compiles to Python, so all of Python and every
Python library work in it. Indentation gives the structure.

```tessh
print hello world
name = tessh
print "hi from $name"
echo a program runs too
```

```output
hello world
hi from tessh
a program runs too
```

`help TOPIC` shows a topic, for example `help pipes`. `help $obj` shows
Python's help for a value, such as `help $str.split`. Every example here is
checked: the `output` under it is what it printed.

# Words and values

## Words
<!-- help: words bareword barewords tokens -->

A **bareword** is text without quotes. As an argument it's the string it
spells. Flags, paths, globs and file names are all barewords.

```tessh
print hello -a --all --color=auto ./run.sh ../x /etc/hosts foo.txt g++
x = hello
print (type $x).__name__ $x
```

```output
hello -a --all --color=auto ./run.sh ../x /etc/hosts foo.txt g++
str hello
```

Operator characters glued to other text are part of the word. Spaced, they're
operators.

```tessh
print -la *.nothing a+b c++
print (1 + 2)
```

```output
-la *.nothing a+b c++
3
```

A bareword written as a path — starting with `/`, `~`, `./` or `../` — is a
**Path**: a `pathlib` path, so `$p.name`, `$p.parent`, `$p / more` and
`$p.exists` work. It keeps how it was written, which is what a command, `print`
and `"$p"` get, and `~` is your home. `+` joins it as text; `Path 'x y'` makes
one from text.

```tessh
p = /tmp/notes/todo.txt
print (type $p).__name__ $p.name $p.parent $p.suffix ($p / more)
r = ./run.sh
print $r "in $r" ($r + .bak) (Path 'a b' / c)
```

```output
Path todo.txt /tmp/notes .txt /tmp/notes/todo.txt/more
./run.sh in ./run.sh ./run.sh.bak a b/c
```

`True`, `False` and `None` are Python's constants.

```tessh
print True False None
```

```output
True False None
```

## Numbers
<!-- help: numbers number int float complex hex -->

Anything Python reads as a number is one.

```tessh
print 42 -7 0x1f 0b101 0o17 1_000 2.5e3 3j
print (type 2.5).__name__ (type 42).__name__
```

```output
42 -7 31 5 15 1000 2500.0 3j
float int
```

## Strings
<!-- help: strings string quotes interpolation fstring raw bytes -->

Double quotes interpolate `$name`, `$(form)` and `${expr}` and understand the
usual escapes. Single quotes are raw: no escapes and no interpolation.

```tessh
name = world
n = 3.14159
print "hello $name"
print "two plus two is $(2 + 2)"
print "pi is ${n:.2f}"
print "tab:\t| newline escaped: \\n | dollar: \$name"
print 'raw: $name \n'
```

```output
hello world
two plus two is 4
pi is 3.14
tab:	| newline escaped: \n | dollar: $name
raw: $name \n
```

`${…}` takes any expression, a `!r` or `!s` conversion and a format spec, as
f-strings do. Use it to glue text right after a variable.

```tessh
items = [a b]
file = notes
print "${file}.txt" "${items!r}" "${len $items}"
```

```output
notes.txt ['a', 'b'] 2
```

Triple quotes span lines. `b"…"` is bytes and `r"…"` a raw string.

```tessh
text = """one
two"""
print $text
print b"bytes" r"raw\n"
```

```output
one
two
b'bytes' raw\n
```

## Variables
<!-- help: variables variable dollar $ sigil attribute index -->

`$name` is the variable `name`. Attribute access, indexing and slicing glue
on.

```tessh
xs = [10 20 30 40]
d = {k: v}
print $xs[0] $xs[1:3] $xs[-1] $d[k]
print $xs.__len__ ("a,b".split ",")
```

```output
10 [20 30] 40 v
<method-wrapper '__len__' of list object at ADDR> ['a' 'b']
```

Call a method by giving it its arguments, or by putting it alone on a line or
alone in parentheses.

```tessh
s = "Hello"
print ($s.upper) ($s.replace l L) ($s.count l)
```

```output
HELLO HeLLo 2
```

A `$name` that isn't a Python name is the environment variable, and `$?` is
the last command's exit status.

```tessh
export GREETING=hey
print $GREETING
false
print $?
true
print $?
```

```output
hey
1
0
```

Glued text joins into one word: `$HOME/src`, `out-$n`. After a variable a
glued `.name` is an attribute, so write `${n}.log` for "n, then `.log`".

```tessh
n = 7
print out-${n}.log file_$n
```

```output
out-7.log file_7
```

# Calls and structure

## Calls
<!-- help: calls call application arguments kwargs keyword splat apply -->

A call is the function then its arguments, spaced: `f a b c`.

```tessh
def add a b
  $a + $b
print (add 1 2)
print (max 3 9 4)
```

```output
3
9
```

Applying a function binds tighter than any operator, as in Haskell. Wrap a
calculation in parentheses to pass its value.

```tessh
n = len [1 2] + 1         # (len [1 2]) + 1
print $n (len [1 2 3]) ((len [1 2]) * 10)
```

```output
3 3 20
```

So `print $x + 1` is `(print $x) + 1`: write `print ($x + 1)`.

Keyword arguments are `name=value`, glued. `*$xs` and `**$kw` splat.

```tessh
print a b c sep=-
args = [1 2 3]
opts = {sep: "/"}
print *$args **$opts
```

```output
a-b-c
1/2/3
```

A function with no arguments is called when it stands alone on its line or
alone in parentheses.

```tessh
import os
here = (os.getcwd)
print ((len $here) > 0)
```

```output
True
```

Indented lines under a call are more arguments, one per line.

```tessh
print
  one
  two
  sep=", "
```

```output
one, two
```

An operator at the head of parentheses applies it Lisp-style.

```tessh
print (+ 1 2 3) (- 10 4) (- 5) (* 2 3 4)
```

```output
6 6 -5 24
```

## Lines
<!-- help: lines indentation indent semicolon continuation comments comment -->

Each line is a form. `;` separates forms on one line, a trailing `\` or an
open bracket continues a line, and `#` starts a comment.

```tessh
a = 1 ; b = 2   # two forms on one line
total = $a + \
  $b
nums = [1 2
        3 4]
print $total $nums
```

```output
3 [1 2 3 4]
```

Commas separate only when a space, a bracket or the end follows. Inside words
they're text.

```tessh
print [1, 2] cut-f1,3
```

```output
[1 2] cut-f1,3
```

Indent with spaces; tabs are an error. `elif`, `else`, `except` and `finally`
continue the form above them at the same indentation.

## Operators
<!-- help: operators operator precedence infix arithmetic comparison and or not walrus ternary -->

Python's operators, spaced, with Python's precedence. Comparisons chain.

```tessh
import operator
x = 4
print (1 + 2 * 3 ** 2) (7 // 2) (7 % 3) (2 ** 10) (7 / 2)
print (1 < $x < 5) ($x in [1 4]) ($x not in [1]) ($x is None) ($x is not None)
print (5 & 3) (5 ^ 3) (1 << 4) (256 >> 2) (operator.invert 5)
print (True and False) (True or False) (not True)
print (True && False) (True || False) (! False)
```

```output
19 3 1 1024 3.5
True True True False True
1 6 16 64 -6
False True False
False True True
```

From the loosest to the tightest: `||` `or`, `&&` `and`, `not` `!`, `|`,
comparisons, backtick and custom operators, `^`, `&`, `<<` `>>`, `+` `-`,
`*` `/` `//` `%` `@`, prefix `-x` `+x`, `**`, application, then glued
`.attr` and `[i]`. (`~` is your home folder and a macro's unquote, so
bitwise not is `operator.invert`.)

Prefix minus: spaced, or glued to a variable or a bracket.

```tessh
x = 5
print (- $x) -$x -(2 + 1)
```

```output
-5 -5 -3
```

`if … then … else …` is the conditional expression, and `:=` works inside
parentheses.

```tessh
n = 12
print (if $n > 10 then big else small)
if (m := $n * 2) > 20
  print $m
```

```output
big
24
```

In a command, operators other than the shell's are words.

```tessh
echo 1 + 2 is three
```

```output
1 + 2 is three
```

## Custom operators
<!-- help: defop backtick backticks custom-operators infix-function -->

Backticks make any function infix.

```tessh
print (3 `max` 9) ([1] `list.__add__` [2])
```

```output
9 [1 2]
```

`defop` defines a symbolic operator: its precedence (1 loosest to 13
tightest, 6 is the custom level), its associativity (`left`, `right` or
`none`) and its function.

```tessh
def concat a b
  $a + $b
defop <> 6 left concat
def power a b
  $a ** $b
defop ^^ 12 right power
print ([1] <> [2] <> [3]) (2 ^^ 3 ^^ 2)
```

```output
[1 2 3] 512
```

# Commands

## Commands
<!-- help: commands command programs program external run attached captured output -->

A line whose first word isn't a Python name runs a program. Its arguments
are words: barewords, `$values`, `(forms)`.

```tessh
echo hello from echo
name = tessh
echo hi $name (1 + 1)
```

```output
hello from echo
hi tessh 2
```

A command on a line of its own is **attached**: its output goes straight to
the terminal. Used as a value, it's **captured**: you get its output as text,
without the final newline.

```tessh
who = echo captured
print got $who
out = printf "a\nb\nc\n"
print $out.lines
```

```output
got captured
['a' 'b' 'c']
```

What a command returns is an `Output`: a `str` with `.code` (the exit
status), `.ok` and `.lines`. Its truth is the exit status. Errors it prints go
to the terminal; `2>&1` captures them too.

```tessh
r = sh -c "echo out; exit 3"
print $r $r.code $r.ok (bool $r)
ok = (true)
print (bool $ok) $ok.code
both = sh -c "echo out; echo err >&2" 2>&1
print $both.lines
```

```output
out 3 False False
True 0
['out' 'err']
```

A list inserts one argument per item. Quoted strings aren't split.

```tessh
args = [one two]
printf "<%s>" $args
echo
printf "%s|" "a b" c
echo
```

```output
<one><two>
a b|c|
```

`\name` runs the program even when a Python function has that name.

```tessh
def echo *words
  print python echo
echo x
\echo program echo
```

```output
python echo
program echo
```

A name tessh doesn't know is looked up when it runs, so functions defined in
other ways still work. Your own function reads its arguments as words, globs
included, with `@command`.

```tessh
@command
def shout *words
  print (str.upper (' '.join $words))
shout hello *.nothing-matches
```

```output
HELLO *.NOTHING-MATCHES
```

## Globs
<!-- help: globs glob wildcard wildcards tilde home -->

`*`, `?` and `[…]` in a command's words expand to the matching names, sorted.
No match leaves the word as written. `~` at the start is your home folder.

```tessh
touch a.py b.py c.txt
echo *.py
echo ?.txt
echo *.none
print (glob *.py)
print ((echo ~/x) == "$HOME/x")
```

```output
a.py b.py
c.txt
*.none
['a.py' 'b.py']
True
```

In a Python call, a bareword is just a string; use `glob` to expand one.

## Pipes
<!-- help: pipes pipe | stage stages pipeline -->

`|` pipes one program's output into the next, as in a shell.

```tessh
printf "b\na\nc\n" | sort | head -2
n = printf "x\ny\n" | wc -l
print lines: ($n.strip)
```

```output
a
b
lines: 2
```

A stage can be a Python call. It gets what came before as its last argument:
the text, or the earlier Python value itself.

```tessh
echo hello | str.upper | print
print (printf "3\n1\n2\n" | str.split | sorted)
import json
echo '{"a": [1, 2]}' | json.loads | print
```

```output
HELLO
['1' '2' '3']
{a: [1 2]}
```

The first stage can be a value: a string is written as it is and any other
iterable one item per line.

```tessh
text = "one two three"
$text | wc -w
[c b a] | sort
```

```output
3
a
b
c
```

When the right side is a value rather than a program or function, `|` is
Python's bitwise or.

```tessh
print ({1 2} | {2 3}) (5 | 2)
a = {x: 1}
b = {y: 2}
print ($a | $b)
```

```output
{1 2 3} 7
{x: 1 y: 2}
```

## And, or, then
<!-- help: && || ; sequence and-or -->

`&&` runs the next command if the last one succeeded, `||` if it failed, `;`
either way.

```tessh
true && echo yes
false && echo never
false || echo fallback
false ; echo anyway
```

```output
yes
fallback
anyway
```

## Redirections
<!-- help: redirections redirection redirect > >> < 2>&1 stderr stdout -->

`>f` writes, `>>f` appends, `<f` reads, `2>f` and `2>>f` send errors,
`&>f` both, `2>&1` errors to output and `>&2` output to errors. Glued to the
file they work anywhere, Python calls included. Spaced (`> f`) they work in
commands.

```tessh
echo first >out.txt
echo second >> out.txt
print from python >>out.txt
cat <out.txt
sh -c "echo oops >&2" 2>err.txt
cat err.txt
sh -c "echo both >&2" 2>&1 | tr a-z A-Z
sh -c "echo o; echo e >&2" &>all.txt
sh -c "echo e2 >&2" 2>>all.txt
cat all.txt
echo to-stderr >&2 2>/dev/null
```

```output
first
second
from python
oops
BOTH
o
e
e2
```

## Jobs
<!-- help: jobs job background & fg bg wait ctrl-z -->

A trailing `&` runs a command in the background, and its value is a `Job`.
`jobs` lists them, `fg` brings one back and `bg` continues a stopped one.
At the REPL, Ctrl+Z stops what's running and Ctrl+C interrupts it.

```tessh
j = sleep 0.5 &
jobs
fg
print done
```

```output
[1] running sleep 0.5
done
```

## Environment and folders
<!-- help: environment env export unset cd pushd popd dirs which source argv folders -->

`export` sets environment variables for this shell and the programs it runs,
and `unset` removes them. `$env` is the environment as a mapping.

```tessh
export COLOR=blue SIZE=10
sh -c 'echo $COLOR $SIZE'
$env.SIZE = 11
print $env.SIZE $env["COLOR"] ("COLOR" in $env)
unset COLOR
print ("COLOR" in $env)
```

```output
blue 10
11 blue True
False
```

`cd` changes folder (`cd` alone goes home, `cd -` goes back), and `pushd`,
`popd` and `dirs` keep a stack. They take bash's options: `cd -P` goes by the
real folders (`-L`, keeping symlinks in the path, is the default), `$CDPATH`
is searched, and `pushd`/`popd`/`dirs` take `+N`/`-N`, `-n`, `-v` and the rest.

```tessh
mkdir -p sub/deeper
start = (pwd)
cd sub
print ((pwd).endswith "/sub")
cd deeper
cd ../..
print ((pwd) == $start)
pushd sub >/dev/null
popd >/dev/null
print ((pwd) == $start)
```

```output
True
True
True
```

`which` finds a program: it prints where, and its value is the path.

```tessh no-run
which git
path = which git
```

`source file.tsh` runs a tessh file here, its names kept. A script's
arguments are in `$argv`.

```tessh
print "x_from_file = 41 + 1" >lib.tsh
source lib.tsh
print $x_from_file
```

```output
42
```

# Python, written as tessh

## Assignment
<!-- help: assignment assign = += destructuring unpacking del -->

```tessh
x = 5
x += 1
x *= 2
[a b] = [1 2]
c, d = 3, 4
[head *rest] = [1 2 3 4]
print $x $a $b $c $d $head $rest
```

```output
12 1 2 3 4 1 [2 3 4]
```

Attributes and items are targets too, and `del` removes.

```tessh
d = {}
$d[k] = 1
$d["j"] = 2
del $d[k]
import types
o = (types.SimpleNamespace)
$o.name = thing
print $d $o.name
```

```output
{j: 2} thing
```

The augmented forms are `+= -= *= /= //= %= **= @= &= |= ^= <<= >>=`.

## Functions
<!-- help: functions function def return lambda fn parameters defaults args kwargs implicit async await -->

`def name params…` with an indented body. The value of the last form is
returned.

```tessh
def greet who greeting=hi *more punct="!" **extra
  "$greeting, $who$punct" + (str $more) + (str $extra)
print (greet Ben)
print (greet Ben hello x y punct=? z=1)
```

```output
hi, Ben!(){}
hello, Ben?('x', 'y'){'z': 1}
```

A one-line `def` puts the body after `=`. `return` leaves early.

```tessh
def sq x = $x * $x
def first_even xs
  for x in $xs
    if $x % 2 == 0
      return $x
  None
print (sq 7) (first_even [1 3 4 5]) (first_even [1])
```

```output
49 4 None
```

`fn [params] body` is a lambda. A block body works too.

```tessh
add = fn [a b] ($a + $b)
print ($add 2 3) (list (map (fn [x] ($x * 10)) [1 2]))
pick = fn [x]
  if $x then yes else no
print ($pick 1)
```

```output
5 [10 20]
yes
```

Decorators go on the lines before. `yield` makes a generator.

```tessh
import functools
@functools.cache
def fib n
  if $n < 2 then $n else (fib ($n - 1)) + (fib ($n - 2))
def count_up n
  for i in (range $n)
    yield $i
print (fib 50) (list (count_up 3))
```

```output
12586269025 [0 1 2]
```

`global` and `nonlocal` work as in Python.

```tessh
total = 0
def bump
  global total
  total += 1
bump ; bump
def counter
  n = 0
  def inc
    nonlocal n
    n += 1
    $n
  $inc
c = (counter)
c ; c
print $total (c)
```

```output
2 3
```

`async def`, `await`, `async for` and `async with` work as in Python.

```tessh
import asyncio
async def twice x
  await (asyncio.sleep 0)
  $x * 2
print (asyncio.run (twice 21))
```

```output
42
```

## Conditionals
<!-- help: if elif else conditionals conditional then -->

`if` is an expression: its value is the branch's.

```tessh
def sign n
  if $n < 0
    'neg'
  elif $n == 0
    'zero'
  else
    'pos'
print (sign -2) (sign 0) (sign 5)
size = if (len [1 2 3]) > 2 then big else small
print $size
```

```output
neg zero pos
big
```

An `if` block can be a value directly.

```tessh
n = 7
kind = if $n % 2 == 0
  'even'
else
  'odd'
print $kind
```

```output
odd
```

A command as the condition uses its exit status.

```tessh
if grep -q b <<< "abc"
  print never
if printf "abc" | grep -q b
  print found
```

```output
found
```

## Loops
<!-- help: loops loop for while break continue else range -->

```tessh
for i in (range 3)
  print $i
for [k v] in ({a: 1 b: 2}.items)
  print $k $v
n = 3
while $n > 0
  n -= 1
  if $n == 1
    continue
  print n is $n
for x in [1 2 3]
  if $x == 5
    break
else
  print no five
```

```output
0
1
2
a 1
b 2
n is 2
n is 0
no five
```

`while` takes an `else` too, run when the loop ends without `break`.

```tessh
n = 2
while $n > 0
  n -= 1
else
  print ended at $n
```

```output
ended at 0
```

## Collections
<!-- help: collections list lists dict dicts set tuple slices slice literals pretty pprint print -->

`[1 2 3]` is a list; with commas each part is an expression. `{a: 1}` is a
dict with string keys from barewords, `{$k: 1}` uses a variable. `{1 2}` is a
set, `()` the empty tuple, `(1, 2)` a tuple and `{}` an empty dict.

```tessh
k = key
print [1 2 3] [1 + 1, 3] {a: 1 b: [2 3]} {$k: 1} {1 2} () (1, 2) {}
xs = (list (range 10))
print $xs[2:5] $xs[::3] $xs[-2:] $xs[:2]
```

```output
[1 2 3] [2 3] {a: 1 b: [2 3]} {key: 1} {1 2} () (1, 2) {}
[2 3 4] [0 3 6 9] [8 9] [0 1]
```

`print` shows a list, dict, set, tuple or record (a dataclass or
namedtuple, as `(Point x=1 y=2)`) as tessh: paste what it prints back in and
you get an equal value. One that doesn't fit on a line is laid out over
indented lines, and it's colored by type on a terminal or in a notebook (not
when piped, or with `NO_COLOR`). A cell's or the REPL's value is shown the
same way. What has no such form, like a function or a file, shows as Python
shows it. `pretty x` gives the text, with `width=` and `color=` to choose.

## Comprehensions
<!-- help: comprehensions comprehension generator genexpr -->

Written as in Python, inside brackets.

```tessh
xs = [1 2 3 4]
print [$x * 2 for x in $xs if $x > 1]
print {$x: $x ** 2 for x in $xs}
print {$x % 2 for x in $xs}
print (sum ($x for x in $xs))
print [[$i $j] for i in [1 2] for j in [a b]]
```

```output
[4 6 8]
{1: 1 2: 4 3: 9 4: 16}
{0 1}
10
[[1 'a'] [1 'b'] [2 'a'] [2 'b']]
```

As everywhere, a name is bare where it's made (`for x`) and has `$` where
it's read (`$x`, `$xs`). Python's `[x * 2 for x in xs]` is an error saying
so: a bare `x` or `xs` would be the text, and the same goes for a `for`
loop's body.

## Classes
<!-- help: classes class self methods inheritance dataclass -->

```tessh
class Animal
  def __init__ self name
    $self.name = $name
  def speak self = "$self.name makes a sound"

class Dog Animal
  def speak self = "$self.name barks"

for a in [(Animal cat) (Dog rex)]
  print ($a.speak)
print (isinstance (Dog x) $Animal)
```

```output
cat makes a sound
rex barks
True
```

Bases, keywords (`metaclass=…`), class attributes, annotated fields and
decorators work as in Python.

```tessh
from dataclasses import dataclass
@dataclass
class Point
  x: int = 0
  y: int = 0
print (Point 1 2) (Point y=5)

class Registry type
  names = []
  def __new__ mcs name bases ns
    $mcs.names.append $name
    type.__new__ $mcs $name $bases $ns
class Plugin metaclass=Registry
  pass
print $Registry.names
```

```output
(Point x=1 y=2) (Point x=0 y=5)
['Plugin']
```

## Errors
<!-- help: errors error exceptions exception try except finally raise assert traceback -->

`try` is an expression too.

```tessh
r = try
  int abc
except ValueError as e
  print caught: $e
  -1
else
  0
finally
  print cleanup
print $r
```

```output
caught: invalid literal for int() with base 10: 'abc'
cleanup
-1
```

`raise`, `raise … from …` and `assert`:

```tessh
def check n
  assert ($n > 0) "n must be positive"
  $n
try
  check 0
except AssertionError as e
  print $e
try
  raise (ValueError bad)
except ValueError as e
  print got $e.args
```

```output
n must be positive
got ('bad',)
```

`raise X from Y` chains, a bare `except` catches anything, and several
classes go in a tuple.

```tessh
try
  try
    1 / 0
  except ZeroDivisionError as e
    raise (ValueError "wrapped") from $e
except (KeyError, ValueError) as e
  print $e "<-" (type $e.__cause__).__name__
try
  int not-a-number
except
  print "anything goes"
```

```output
wrapped <- ZeroDivisionError
anything goes
```

Where Python wants a class (after `except`, as a base), a bareword is the
name. Elsewhere pass a class with `$`: `isinstance $x $int`.

Errors point at tessh lines: a traceback shows the tessh source with carets
under the part that failed.

## With
<!-- help: with context-manager open files -->

```tessh
with (open data.txt w) as f
  $f.write "line one\n"
with (open data.txt) as f
  text = ($f.read)
  print ($text.strip)
```

```output
line one
```

Several context managers go on one line, separated by commas.

```tessh
with (open a.txt w) as a, (open b.txt w) as b
  $a.write A
  $b.write B
print (cat a.txt b.txt)
```

```output
AB
```

## Match
<!-- help: match case patterns pattern -->

Barewords in a pattern match strings, `$name` captures, `_` is anything.

```tessh
def route cmd
  match $cmd
    case [go $dir]
      "going $dir"
    case [take *$items]
      "taking $items"
    case {kind: circle r: $r}
      "circle of $r"
    case (int) | (float)
      "a number"
    case [$n] if $n > 10
      "big $n"
    case _
      "what?"
print (route [go north])
print (route [take lamp key])
print (route {kind: circle r: 2})
print (route 3.5) (route [11]) (route [1])
```

Class patterns take positional or keyword sub-patterns, `as $name` names a
part, and a dotted name matches a constant.

```tessh
from dataclasses import dataclass
import enum
@dataclass
class Pt
  x: int
  y: int
class Color enum.Enum
  RED = 1
  BLUE = 2
def what v
  match $v
    case (Pt 0 0)
      'origin'
    case (Pt x=0 y=$y)
      "on the y axis at $y"
    case [1 2] as $whole
      "the list $whole"
    case Color.RED
      'red'
print (what (Pt 0 0)) "/" (what (Pt 0 5)) "/" (what [1 2]) "/" (what $Color.RED)
```

```output
origin / on the y axis at 5 / the list [1, 2] / red
```

```output
going north
taking ['lamp', 'key']
circle of 2
a number big 11 what?
```

## Imports
<!-- help: import imports from modules module -->

```tessh
import os sys math
from os.path import join basename
from collections import Counter as C
print (join a b) (basename /x/y.txt) ((C abca).most_common 1)
print (math.sqrt 16)
```

```output
a/b y.txt [('a', 2)]
4.0
```

Packages from PyPI (numpy, pandas, matplotlib, ...) come with `need`: uv
installs them once into a cache, and they import from the next line. A module
that isn't there says so: `No module named 'numpy': need numpy installs it`.

## Do
<!-- help: do block progn -->

`do` makes several forms into one; its value is the last one's.

```tessh
r = do
  a = 2
  $a * 21
print $r
```

```output
42
```

## Statements
<!-- help: statements pass break continue return yield await del global nonlocal -->

`pass`, `break`, `continue`, `return`, `yield`, `yield from`, `await`,
`del`, `global`, `nonlocal`, `raise` and `assert` are forms like any other.

```tessh
def gen
  yield 1
  yield from [2 3]
print (list (gen))
def nothing
  pass
print (nothing)
```

```output
[1 2 3]
None
```

# From Clojure

## Methods
<!-- help: methods method .method dot interop field .-field -->

As in Clojure, a word starting with `.` at the head of a call is a method:
`.name obj args…` is `$obj.name args…`, the object first.

```tessh
s = "  Hello  "
print (.strip $s) (.replace $s l L) (.count $s l)
print (.upper (.strip $s))
```

```output
Hello   HeLLo   2
HELLO
```

`.-name obj` reads a field rather than calling it. A method you call for
what it does stands on its own line.

```tessh
import datetime
d = datetime.date 2024 3 9
print (.-year $d) (.isoformat $d)
words = [b a c]
.sort $words
print $words
```

```output
2024 2024-03-09
['a' 'b' 'c']
```

Alone in parentheses, `(.name)` is a function that calls the method on its
first argument, and `(.-name)` one that reads the field. (A bare `.name`
as an argument is a word, since `.bashrc` is a file.)

```tessh
up = (.upper)
print ($up hi) (list (map (.title) [ann bob]))
print (list (map (.-real) [1 2]))
```

```output
HI ['Ann' 'Bob']
[1 2]
```

A `.name` stage in a pipe calls the method on what came in.

```tessh
echo " a,b,c " | .strip | .split "," | print
```

```output
['a' 'b' 'c']
```

## Threading
<!-- help: threading thread -> ->> chain chaining pipeline-value -->

`-> value step…` puts the value in as each step's first argument: `(f a)`
becomes `(f value a)`, and a bare step `f` or `.name` becomes `(f value)`.
`->>` puts it in last. So methods chain with `->`, and functions that take
the collection last chain with `->>`.

```tessh
s = "  Hello World  "
print (-> $s .strip .lower (.replace world there) (.split " "))
print (->> [3 1 2] sorted (map $inc) list)
```

```output
['hello' 'there']
[2 3 4]
```

Steps can go on indented lines.

```tessh
r = ->> (range 10)
  filter (fn ($% % 2))
  map (fn ($% * $%))
  sum
print $r
```

```output
165
```

Mix them by nesting a `->>` step inside `->`. (A `.name` step under `->>`
gets the value as its last argument, not as the object.)

```tessh
print (-> "a-b" (.split "-") (->> (map $str.upper)) list)
```

```output
['A' 'B']
```

`some->` and `some->>` stop at the first `None`: the value is `None` and
the later steps don't run.

```tessh
cfg = {db: {host: "h1"}}
print (some-> $cfg (.get db) (.get host) .upper)
print (some-> $cfg (.get web) (.get host) .upper)
```

```output
H1
None
```

`as-> value name step…` names the value, so it can go anywhere in a step;
each step's result becomes the name's new value.

```tessh
print (as-> [3 1 2] xs (sorted $xs) ($xs + [9]) (len $xs))
```

```output
4
```

## Doto
<!-- help: doto builder -->

`doto obj step…` calls each step with the object first, then gives back the
object itself. Steps can go on indented lines.

```tessh
xs = doto []
  .append 1
  .append 2
  .extend [3 4]
print $xs
```

```output
[1 2 3 4]
```

## Keywords
<!-- help: keywords keyword :name getter getters -->

`:name` used as a function gets `name`: the item from a dict (`None`, or the
second argument, when it's missing) or the attribute from anything else.
That makes it a ready-made `key=` and `map` function.

```tessh
p = {name: Ann age: 31}
print (:name $p) (:town $p) (:town $p Oslo)
people = [{name: Bo age: 40} $p]
print (list (map :name $people)) (:name (max $people key=:age))
```

```output
Ann None Oslo
['Bo' 'Ann'] Bo
```

```tessh
import datetime
print (:year (datetime.date 2024 1 2))
```

```output
2024
```

(A `:name` is still a symbol, the form a macro gets: see `help forms`.)

## Destructuring
<!-- help: destructuring destructure keys {} pick -->

`{a b} = value` takes names out of a dict by key, or out of an object by
attribute. `name=default` is for a key that may be missing, and `**rest`
(last) gets the other keys. A missing key without a default is a `KeyError`.

```tessh
p = {name: Ann age: 31 town: Oslo}
{name age} = $p
print $name $age
{name job=none **more} = $p
print $job $more
import datetime
{year month} = datetime.date 2024 5 1
print $year $month
```

```output
Ann 31
none {age: 31 town: 'Oslo'}
2024 5
```

## Short functions
<!-- help: short-fn % $% $%1 $%& anonymous -->

`fn body` without a parameter list makes its parameters from `$%` (or
`$%1`), `$%2`… and `$%&` for the rest, like Clojure's `#(…)`. A body that
starts with a list goes in parentheses, since `fn [` starts a parameter
list. Don't nest short functions.

```tessh
print (list (map (fn ($% * 10)) [1 2 3]))
add = fn ($%1 + $%2)
print ($add 2 3)
count = fn (len $%&)
print ($count a b c)
```

```output
[10 20 30]
5
3
```

## Discard
<!-- help: discard #_ comment-out -->

`#_` drops the form after it, in a line or in brackets. First on a line it
drops the whole line with its indented block.

```tessh
print 1 #_ 2 3 #_(4 5) 6
print [a #_[b c] d]
#_ print not this
  print nor this
print done
```

```output
1 3 6
['a' 'd']
done
```

## Core functions
<!-- help: core seq sequences first second rest take drop frequencies group_by partition comp juxt partial get_in assoc_in update_in -->

Clojure's sequence and map functions, with underscores for its dashes.
Each takes the function first and the collection last, so they work as
`->>` steps and pipe stages. They return lists, except `iterate` and
`repeatedly` without a count, which never end: `take` some. (`last` is
the function: `\last` runs the program of that name.)

```tessh
print (first [1 2 3]) (second [1 2 3]) (last [1 2 3]) (rest [1 2 3]) (butlast [1 2 3])
print (take 3 (iterate $inc 10)) (drop 2 [1 2 3 4]) (take_while (fn ($% < 3)) [1 2 5 1])
print (frequencies "abca") (distinct [1 2 1 3])
print (group_by $len [a bb c dd]) (partition 2 [1 2 3 4 5]) (partition_all 2 [1 2 3])
print (partition_by (fn ($% > 0)) [1 2 -1 3]) (interpose "," [a b c]) (interleave [1 2] [a b])
print (flatten [1 [2 [3 4]]]) (mapcat (fn ([$% $%])) [1 2]) (keep (fn (if $% > 1 then $% else None)) [1 2 3])
```

```output
1 2 3 [2 3] [1 2]
[10 11 12] [3 4] [1 2]
{a: 2 b: 1 c: 1} [1 2 3]
{1: ['a' 'c'] 2: ['bb' 'dd']} [[1 2] [3 4]] [[1 2] [3]]
[[1 2] [-1] [3]] ['a' ',' 'b' ',' 'c'] [1 'a' 2 'b']
[1 2 3 4] [1 1 2 2] [2 3]
```

`comp`, `juxt`, `complement`, `partial`, `constantly`, `identity`, `inc`,
`dec`, `some` (the first true result) and `every`:

```tessh
print ((comp $str $inc) 1) ((juxt $min $max) [3 1 2]) ((complement $bool) 0)
print ((partial $pow 2) 10) ((constantly 7) 1 2) (identity x) (inc 1) (dec 1)
print (some (fn (if $% > 1 then $% * 10 else None)) [1 2 3]) (every (fn ($% > 0)) [1 2])
print (repeatedly (constantly x) 3)
```

```output
2 [1 3] True
1024 7 x 2 0
20 True
['x' 'x' 'x']
```

`get_in`, `assoc_in` and `update_in` reach into nested data by a path.
`assoc_in` and `update_in` return a changed copy and leave the original
alone. Also `zipmap`, `select_keys` and `merge_with`.

```tessh
d = {a: {b: [10 20]}}
print (get_in $d [a b 1]) (get_in $d [a z] none)
print (assoc_in $d [a c] 5) (update_in $d [a b] $len)
print (zipmap [x y] [1 2]) (select_keys {x: 1 y: 2} [y]) (merge_with $max {a: 1} {a: 5 b: 2})
print $d
```

```output
20 none
{a: {b: [10 20] c: 5}} {a: {b: 2}}
{x: 1 y: 2} {y: 2} {a: 5 b: 2}
{a: {b: [10 20]}}
```

# From Hy

## Modules
<!-- help: modules module .tsh import-tsh packages package __init__.tsh -->

`import name` finds `name.tsh` as well as Python's modules: a tessh file is
a module, and a folder with an `__init__.tsh` a package. The folder you're
in is on the path, and a script's own folder comes first.

```tessh
echo 'def hello who = "hello, $who"' > greeting.tsh
echo 'answer = 42' >> greeting.tsh
import greeting
print (greeting.hello Ann) $greeting.answer
from greeting import hello
print (hello Bo)
```

```output
hello, Ann 42
hello, Bo
```

Python code that tessh runs imports tessh modules the same way.

```tessh
echo 'import greeting2' > helper.py
echo 'VALUE = greeting2.hello("python")' >> helper.py
echo 'def hello who = "hi from tessh, $who"' > greeting2.tsh
import helper
print $helper.VALUE
```

```output
hi from tessh, python
```

(Where a folder has both `name.py` and `name.tsh`, Python's is imported.)

## Require
<!-- help: require macros-from defmacro-import -->

`require module` brings in that module's macros (and `defop` operators) for
the code after it, imported while the code compiles. `require module name…`
takes only those macros.

```tessh no-check
py """
open("loud.tsh", "w").write('''
defmacro loudly *body
  quasi
    print "--"
    ~@body
    print "--"
''')
"""
require loud
loudly
  print inside
```

```output
--
inside
--
```

## Let
<!-- help: let scope scoped block-scope local -->

`let name=value …` then a block (or a form on the same line) makes names
that only that block sees, each bound in order, so a later one can use an
earlier one. The value is the block's last form's. Outside it, a name of
the same name is untouched.

```tessh
x = outside
r = let x=2 y=($x * 10)
  $x + $y
print $r $x
print (let a=5 ($a * $a))
```

```output
22 outside
25
```

Assigning to a `let` name in its block changes the block's own.

```tessh
let total=0
  for n in [1 2 3]
    total += $n
  print total $total
print ("total" in (globals))
```

```output
total 6
False
```

## Python inline
<!-- help: py python inline-python embed raw-python -->

`py "code"` is Python as written: an expression's value, or statements
(defining functions, classes, …) when it's more. Use single or triple
quotes, so tessh doesn't read `$` or `{…}` in it, and indent a triple-quoted
block as you like. Names are shared with the tessh around it.

```tessh
xs = [3 1 2]
print (py "sorted(xs, reverse=True)")
print (py "{k: v for k, v in zip('ab', xs)}")
```

```output
[3 2 1]
{a: 3 b: 1}
```

```tessh
py '''
import re
def words(s):
    return re.findall(r"\w+", s)
'''
print (words "one, two; three")
```

```output
['one' 'two' 'three']
```

# Macros

## Macros
<!-- help: macros macro defmacro quasi quasiquote unquote splice hygiene hygienic -->

A macro runs while the code compiles: it takes forms and returns forms.
`quasi` with an indented template builds them; `~x` inserts a form,
`~@xs` splices a list of them and `$~x` makes a variable from a name.

```tessh
defmacro unless cond:expr *body
  quasi
    if (not ~cond)
      ~@body
x = 2
unless $x > 3
  print small
  print really
```

```output
small
really
```

Parameters: `name` takes one item, `*name` the rest (indented lines
included), and `name:expr` the rest of the line as one expression.

Macros are **hygienic**. Names a macro binds never clash with yours, and
names it uses mean what they meant where it was defined.

```tessh
defmacro swap a b
  quasi
    tmp = $~a
    ~a = $~b
    ~b = $tmp
tmp = 1
other = 2
swap tmp other
print $tmp $other
```

```output
2 1
```

`capture name` breaks hygiene on purpose, for anaphoric macros.

```tessh
defmacro aif cond:expr *body
  it = capture "it"
  quasi
    ~it = ~cond
    if $~it
      ~@body
aif (len [1 2 3])
  print got $it
```

```output
got 3
```

`:(form)` is an inline template and `~(expr)` inserts a value computed when
the macro runs. Numbers, strings and lists a macro returns become literals.

```tessh
defmacro twice x
  :(~x + ~x)
defmacro answer
  quasi
    ~(6 * 7)
print (twice 21) (answer)
```

```output
42 42
```

## Forms
<!-- help: forms form quote symbol symbols macroexpand gensym Form -->

`quote form` and `:name` are forms not evaluated. A macro gets its arguments
as `Form` objects: `.kind`, `.name`, `.items`, `.children`, `.value`,
`.inner`, `.op`, `.bracket`, indexing and `len`. `str` shows one as tessh.

```tessh
f = quote (print a $b 1)
print $f.kind (len $f) $f[0].name $f[2].kind $f[3].value
s = :sym
print (str $f) $s.kind
```

```output
Group 4 print Var 1
(print a $b 1) Sym
```

A macro can look at its forms and build code from them.

```tessh
defmacro show_args *args
  names = [(str $a) for a in $args]
  quasi
    print ~names
show_args a $b (1 + 2)
```

```output
['a' '$b' '(1 + 2)']
```

`macroexpand` shows what a form becomes, and `gensym "hint"` makes a fresh
name (hygiene makes it rarely needed).

```tessh
defmacro inc v
  quasi
    ~v = $~v + 1
print (str (macroexpand (quote (inc n))))
print ("tmp" in (str (gensym tmp)))
```

```output
n = $n + 1
True
```

Macro code runs before the template: compute with the forms, then insert
the results (`~name` for a name, `~(expr)` for anything else). A name the
macro makes for *your* code to use goes through `capture`; `sym "name"` makes
a hygienic one and `var s` makes `$s`. Strings and numbers become literals.

```tessh
defmacro getter name
  key = $name.name
  quasi
    def ~(capture "get_$key") obj
      $obj[~key]
getter color
print (get_color {color: teal})
```

```output
teal
```

# Running tessh

## Dependencies
<!-- help: dependencies need --with uv pep723 packages pip script-metadata -->

`need` names PyPI packages a script, a cell or the REPL uses: uv installs
them once into a cache, and they import from the next line. Quote a version
range, so its `>` isn't a redirection.

```tessh no-run
need requests "rich>=13"
import requests
print (requests.get "https://example.com").status_code
```

From the command line, `--with` (any number of times, or comma-separated)
does the same before the script runs, as `uv run --with` does.

```
tessh --with requests --with "rich>=13" fetch.tsh
tessh --with requests,rich -c 'import rich; rich.print "[bold]hi"'
```

A PEP 723 inline metadata block, the one `uv run` reads, works too, and
says which Python it wants:

```tessh no-run
# /// script
# requires-python = ">=3.11"
# dependencies = ["requests", "rich>=13"]
# ///
import requests
print (requests.get "https://example.com").status_code
```

## The REPL
<!-- help: repl interactive keys prompt history rc tab completion -->

`tessh` alone starts the REPL.

- A line runs as soon as it's complete. Open brackets and strings carry on to
  the next line.
- After a line that opens a block (`def`, `if`, `for`, `x = if …`, a macro)
  Enter indents the next line, and a blank line runs the block.
- An `else`, `elif`, `except` or `finally` typed too deep moves back to its
  `if` or `try` on Enter; Enter again goes on.
- Backspace in the leading spaces removes a whole level. Alt+Enter always
  adds a line.
- Tab completes names, programs and paths. History is kept in
  `~/.local/share/tessh/history`, and `~/.config/tessh/rc.tsh` runs first.
- Ctrl+C interrupts, Ctrl+Z stops a program (`fg` brings it back), Ctrl+D
  exits.

## Scripts and checks
<!-- help: scripts script running run check --check --py -c shebang -->

```text
tessh file.tsh args…      run a script (#!/usr/bin/env tessh works)
tessh -c 'code'           run a string
tessh --check [file]      check without running; line:col: message per problem
tessh --check --json      the same as JSON
tessh --py file.tsh       print the Python it compiles to
tessh --kernel -f conn    the Jupyter kernel
```

## Notebooks
<!-- help: notebooks notebook kernel jupyter display input rich html plots matplotlib -->

In Tessera's notebook, tessh is the default kernel. A cell's last value
shows with its richest form: HTML, Markdown, images, matplotlib figures.
`display x` shows a value partway through, and `input` asks in the card.
The folder and exported variables go back and forth with the pane's shell.

Programs a cell runs run in the pane's terminal, as a shell cell's do, with
the cell's folder and variables. What they print goes in the card in order
with the rest; a terminal app (vim, less, htop: one that takes the whole
screen) gets the live terminal in the card while it runs. The cell goes on
when it exits, with its status in `$?`.

```tessh no-run
need pandas matplotlib
import pandas as pd
df = pd.DataFrame {a: [1 2 3] b: [4 5 6]}
display $df
import matplotlib.pyplot as plt
plt.plot [1 4 9]
name = input "Your name? "
```
