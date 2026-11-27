<!-- tessh for AI models: a condensed guide.md, given to models that suggest,
     write or check tessh lines (Tessera's suggestions, # commands, fixes and
     assistant). Keep it short (it's sent with every request) and in step
     with guide.md. -->
# tessh in brief

tessh is the user's shell. Ordinary commands, pipes, redirections, globs,
`&&`, `||`, `&`, `cd`, `export`, `sudo …` work as in bash. Its own lines
compile to Python, so all of Python and its libraries work, written like a
shell:

- Words are separated by spaces. No commas between arguments, no call
  parentheses: `print a b sep=-`, `max 3 9 4`, `json.loads $text`.
- A bareword is a string (`hello`, `-la`, `~/src`); an unquoted pattern
  (`*.py`) is a glob, a quoted one (`'*.py'`) is text. Numbers are
  numbers. `$name` is a variable; `$x.attr`, `$x[i]`, `$x[1:3]` glue on.
- `(…)` groups: it calls a function or captures a command's output:
  `n = (len $xs)`, `files = (ls)`, `print (glob *.py)`.
  A method call with no arguments is `($s.upper)`, `($d.items)`, never
  `$s.upper()`; `$d.items` alone is the method, not its result.
- `name = value` assigns (spaces around `=`); `name=value` glued is a keyword
  argument or, before a command, an environment variable.
- Strings: `"hello $name, $(2 + 2), ${n:.2f}"` interpolate; `'raw'` doesn't.
- Lists `[1 2 3]`, dicts `{a: 1 b: [2 3]}`, sets `{1 2}`, tuples `(1, 2)`.
  Comprehensions: `[$x * 2 for x in $xs if $x > 1]`.
- Blocks: the header line has no `:` and no do/then/end; the body is
  indented on the next lines. Never write bash's `do … done`, `then … fi`,
  `[[ … ]]`, or `$(…)` outside a string (use `(…)`).
- `$?` is the last exit status. `(expr)?` turns an error into a false
  value: `n = (int $s)? || 0`. Regex: `$line =~ re'(\d+)'`.
- A pipe stage can be a Python callable: it gets the text so far.

```tessh
ls -la ~/src
git log --oneline -n 5 | head -3
sudo nixos-rebuild switch --flake /etc/nixos
name = world
print hello $name
for p in *.py
  print $p.name ($p.stat).st_size
for f in (glob *.png)
  convert $f -resize 50% small-$f
for i in (range 3)
  print $i
for [k v] in ($d.items)
  print $k $v
if $n > 1
  print many
elif $n == 1
  print one
else
  print none
while $n > 0
  n -= 1
def greet who greeting="hi"
  "$greeting, $who!"
print (greet Ben)
import json
data = json.loads (cat package.json)
print $data["name"] ($data.get version "?")
echo hello | str.upper | print
count = printf "a\nb\n" | wc -l
lines = ((cat notes.txt).splitlines)
with (open out.txt w) as f
  $f.write "done\n"
size = if (len $xs) > 2 then big else small
export PORT=8080
cd ~/src/tessera && cargo build 2>&1 | tail -5
```
