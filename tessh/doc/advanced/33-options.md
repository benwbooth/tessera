# 33. Options
<!-- help: abs-options abs-33 options set shopt errexit nounset xtrace noclobber pipefail -->

Bash's options change how the shell itself behaves: `set -o name` or `set
-x` turns one on, `set +o name` or `set +x` turns it off, and the same
letters work after the `#!` line (`#!/bin/bash -x`) or on the command line
(`bash -v script`). `shopt -s name` sets the newer ones.

tessh has no such switches. Its behavior is fixed, mostly at what bash's
careful scripters turn on, and Python modules cover the rest. A `set -x`
line carried over is an error saying so (`set` alone is Python's set type).

```bash no-run
#!/bin/bash

set -o verbose
# Command echoing on.
command
...
command

set +o verbose
# Command echoing off.
command
# Not echoed.
```

Bash's options, and where tessh stands on each:

| bash | name | tessh |
| --- | --- | --- |
| `-B` / `+B` | braceexpand | **Gap:** no brace expansion; `x{a,b}y` stays as written. Use a list: `["x${c}y" for c in [a b]]` |
| `-C` | noclobber | none: `>` always writes, as bash's `>|` does |
| `-D` | (list `$"…"` strings) | none |
| `-a` | allexport | none: `export` each name, or set `$env[name]` |
| `-b` | notify | none; `jobs` shows finished jobs |
| `-c …` | (run a string) | `tessh -c 'code' args…` |
| `checkjobs` | | none |
| `-e` | errexit | **Gap:** a failing program never stops a script. A Python error always does; for a program, `cmd || exit 1` or check `$?` |
| `-f` | noglob | none needed: a variable's value is never globbed, and a quoted pattern is text |
| `globstar` | | always on: `**/*.py` searches the subfolders |
| `-i` | interactive | `tessh` alone is the REPL |
| `-n` | noexec | `tessh --check script`, and better: every script is compiled whole before it runs |
| `-o posix` | | none |
| `-o pipefail` | | **Gap:** a pipeline's status is its last program's, as without `pipefail` |
| `-p` | privileged | none |
| `-r` | restricted | none |
| `-s` | stdin | `tessh --check` reads stdin; a script is read from a file or `-c` |
| `-t` | (exit after one command) | none |
| `-u` | nounset | always: an unknown `$name` is a `NameError` |
| `-v` | verbose | **Gap:** none |
| `-x` | xtrace | **Gap:** none; `tessh --py` shows the compiled Python, and `sys.settrace` can trace lines (chapter 32) |
| `-` / `--` | end of options | `$argv` is the list as given; `set -- a b` is `argv = [$argv[0] a b]` |

A few of these, tried:

```tessh
#!/usr/bin/env tessh
mkdir -p sub/deep ; touch a.py sub/b.py sub/deep/c.py

echo x{a,b}y                       # No brace expansion: stays as written.
print ["x${c}y" for c in [a b]]   # A list does the job.

echo **/*.py                       # globstar is always on.

pattern = "*.py"
echo $pattern                      # A value is never globbed (noglob for free).

false | true                       # No pipefail: the last program's status.
print $?

try                                # nounset: an unknown name is an error.
  print $nothing_here
except NameError as e
  print $e
```

```output
x{a,b}y
['xay' 'xby']
a.py sub/b.py sub/deep/c.py
*.py
0
name 'nothing_here' is not defined (and there's no environment variable nothing_here)
```
