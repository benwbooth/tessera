# Gaps: what tessh can't do yet
<!-- help: abs-gaps gaps missing todo bash-features-missing -->

Porting the ABS Guide's examples turned up what bash has and tessh hasn't.
Each one is marked **Gap** where it comes up; here they are together, with
what to use meanwhile. Some are by design, where Python already does the
job: those are together under "By design", with the Python for each.

## Fixed along the way
<!-- help: abs-gaps-fixed -->

These came up while porting and were fixed in tessh, so the examples use
them as bash would:

- `$name1`, `$x2y`: a name with digits in it is the whole name.
- `cd dir || exit 1`, `cd dir && make`, `if cd dir`: a builtin gives a
  status as bash's do, 0 when it worked and 1 (its message on standard
  error) when it didn't. On a line of its own, where nothing checks it, a
  failing builtin stops the script instead.
- `:`, bash's null command (`: >file`, `while :`).
- `if false`, `while true`: a lone word as a condition runs the command.
- `if ! cmd`, and `! cmd` sets `$?` as bash's does.
- `$CMD args`, and `command prog args`: run the program the text names.
- bash's builtins that change the shell, with bash's options (`help NAME`
  describes each): `read`, `getopts` (with `$OPTIND`, `$OPTARG`), `shift`,
  `eval` (of tessh text), `exec program`, `type`, `command -v`/`-V`,
  `hash`, `builtin`, `trap`, `ulimit`, `umask`, `times`, `history`,
  `fc -l`; and job control: `jobs`, `fg`, `bg`, `wait`, `disown`, `kill`
  (a builtin taking `%job` specs), `suspend`, `logout`. A `Job` has `.pid`
  (bash's `$!`), `.pids` and `.status`. A builtin used as a value gives
  what it prints, and in a pipe its output goes down the pipe
  (chapter 15).
- `exec` with only redirections (`exec >log 2>&1`, `exec 3<file`),
  `source file args…`, `select` (`for x in (select …)`), `readonly`,
  `trap … ERR`, `complete` and `compgen`, `$LINENO`, `$SHLVL`, and `!!`,
  `!$`, `!n`, `!prefix` at the prompt.
- `set -x`/`+x` (after `$PS4`), `set -v`, `set -o pipefail`, `set -o
  name`/`+o name`, `set -o` alone, `set -- a b`; `$PIPESTATUS`; `trap …
  DEBUG` (with `$BASH_COMMAND`) and `trap … RETURN`; `coproc`; `mapfile`
  and `readarray`; `time` as a builtin (`\time` is the program); `while
  read line <file` reads the file once, line after line; and history's
  `!?text?`, `!#`, word designators, modifiers and `^old^new^`.
- `alias` and `unalias`, as bash's.
- Blocks redirected, piped and in the background: `do >log`, `do <file`,
  `do | sort`, `do &` with the block under it (bash's `{ …; } >f`, `done
  <file`, `fi >log`, `for … done | sort`), and `subshell` with a block
  (bash's `( … )`, a fork of tessh), with the same after it. A function
  first in a pipe sends what it prints down it (`f | sort`).
- `a |& b`; redirections first (`<file cmd`, `2>/dev/null cmd`); a line of
  only redirections (`>file`); a program redirected to or from a named pipe
  in the background doesn't stop tessh until the other end opens.
- `*[1 2]` splats the list, and `f languages=[fr]` gives it a list.
- A `def`'s or `class`'s first-line string is its docstring.
- `~/.config/tessh/logout.tsh` when a login shell ends; `$HISTCMD`;
  `complete -o filenames` and `-o nospace`.
- `declare`, `local`, `shopt`, `bind`, `caller`, `enable`: not
  in tessh, but each now says what to do instead, rather than "command not
  found".
- `$PWD` is right when tessh starts.
- A heredoc's word is the whole word (`<<End-Of-Session`).
- A redirection on a function call (`f >file`, `f <file`, `f 2>file`) reaches
  the programs the function runs.
- A line ending in `|`, `&&` or `||` goes on to the next.
- Globs: `[!ab]*`, `[^ab]*`, `.b*` (dot files), `$x =~ [0-9]`; `.*` without
  `.` and `..`; a glob in a list given to a program.
- `echo *$range` splats any iterable; `case [1 *_]`; octal escapes (`"\033"`).
- "command not found" goes to the command's `2>`.
- Output is line-buffered, so it stays in order with programs' when piped.
- Errors instead of silent mistakes: `if [ -n "$x" ]` (a list, always
  true), a `for`/`while`/`if` with no indented block (bash's missing
  `done`), `set +u`, a method not called given to a program.

## By design: Python does it
<!-- help: abs-by-design by-design parameter-expansion bashisms -->

Where Python already does something well, tessh uses Python's way and
doesn't port bash's syntax for it. `${…}` holds a tessh expression and
nothing else, so bash's forms inside it aren't read as bash's.

| bash | tessh |
|---|---|
| `${x#pre}`, `${x%.gz}` | `$x.removeprefix "pre"`, `$x.removesuffix ".gz"` |
| `${x%.*}`, `${x%%.*}` | `($x.rpartition ".")[0]`, `($x.partition ".")[0]` |
| `${x#*.}`, `${0##*/}` | `($x.partition ".")[2]`, `($0.rpartition "/")[2]` (or `os.path.basename`) |
| `${x/a/b}`, `${x//a/b}` | `$x.replace "a" "b" 1`, `$x.replace "a" "b"` |
| a glob pattern in any of those | `re.sub`, or `fnmatch` |
| `${#x}` | `len $x` |
| `${x:-d}`, `${x:=d}` | `$x or "d"`; an environment variable: `$env.get "X" "d"` |
| `${x:?msg}` | `$env["X"]` (a KeyError), or `raise SystemExit "msg"` |
| `${x:+w}` | `if $x then "w" else ""` |
| `${x^^}`, `${x,,}`, `${x^}` | `$x.upper()`, `$x.lower()`, `$x.capitalize()` |
| `${x:2:3}` | `$x[2:5]` |
| `${!name}`, `${!pre*}` | `(globals)[$name]`, `[$n for n in (globals) if $n.startswith "pre"]` |
| `file{1,2}`, `{a..e}` | `["file$i" for i in [1 2]]`, `[chr $c for c in range (ord "a") (ord "f")]`; `{…}` glued to text is a set literal |
| `{b*,c*}` | `glob b* c*` |
| `[[:digit:]]` in a glob or regex | `[0-9]`, `\d`, `str.isdigit` (`re'[[:digit:]]'` means something else to Python) |
| `~+`, `~-` | `(pwd)`, `$env.OLDPWD` |
| `$!` | `$j.pid`, after `j = cmd &` |
| `$IFS` word splitting | none: a value is one word; `.split` when you want several (`$IFS` is only `read`'s) |
| `$RANDOM`, `$UID`, `$SECONDS`… | Python's forms, below (Variables) |
| `/dev/tcp/host/port` | `socket` (chapter 29) |
| `case` fall-through (`;&`, `;;&`) | none in `match`; test the cases with `if` |
| `break N`, `continue N` | a function and `return`, or `for … else` |
| `eval $code` (code in a variable) | `eval "$code"`; `eval $x` with a bare value is Python's `eval` |
| `set -a`, `-b`, `-h`, `-k`, `-m`, `-P`, `-t`; `shopt` | `export NAME=value`; jobs are reported at the next prompt; there's no hashing, no `-k` words; `cd` keeps links (`os.path.realpath` for the real path); `shopt` says what to do for each |
| `set +u` | none: an unset name is always an error; `$env.get "X"` or `globals().get` when it may not be set |
| `PS1`, `PS2`, `PROMPT_COMMAND` | the prompt bar, set in Tessera's settings (its widgets, or Starship); `@before_prompt` for `PROMPT_COMMAND` |
| `$HISTFILE`, `$HISTTIMEFORMAT` | tessh's history is always its own file (`$XDG_DATA_HOME/tessh/history`), so bash's doesn't get tessh's lines; it keeps no times |
| restricted mode (`bash -r`) | none: with Python underneath it couldn't mean much |
| a bareword next to `and`/`or` | a word, by design (`print it is ready`): quote it, `$x == "PREV" and …` |

## Builtins
<!-- help: abs-gaps-builtins read eval exec trap wait type -->

- A failing *program* stops a script only in strict mode (`strict`, `set
  -e`, `tessh -e`) or with `(cmd)!`, by design: a failing builtin on a line
  of its own stops it anyway (chapters 32, 33).

## Variables
<!-- help: abs-gaps-variables $UID $RANDOM $LINENO $SECONDS -->

None of bash's internal variables are tessh variables; each has a Python
form, listed in chapter 9: `$UID`/`$EUID` (`os.getuid`), `$$`/`$PPID`
(`os.getpid`, `os.getppid`), `$RANDOM` (`random.randrange 32768`),
`$SECONDS` (`time.monotonic`), `$FUNCNAME`/`caller` (`inspect`),
`$HOSTNAME`/`$OSTYPE` (`socket`, `sys.platform`), `$BASH_VERSION`, `$_`,
`$-`, `$DIRSTACK` (`dirs` only prints), `$BASH_SUBSHELL`,
`$GLOBIGNORE`, `$PS1`, `$PS2`, `$PS4`, `$PROMPT_COMMAND`, `$TMOUT`
(`read -t` times one read). `$LINENO` is tessh's too, `$SHLVL` counts
tessh, `$PS3` is `select`'s prompt, `$REPLY` is set by `read`, and `$IFS`
is where `read` splits; otherwise tessh never splits a value into words,
by design.

## Words and expansion
<!-- help: abs-gaps-expansion -->

- `\z` (a backslash before an ordinary letter) outside quotes stays `\z`;
  `echo foo\` glued to the end of a line doesn't continue it (with a space,
  `\` does).
- A pattern held in text: `glob $p` doesn't glob it; `Glob $p` does.

## Structure
<!-- help: abs-gaps-structure -->

- A `do` or function stage *after* the first in a pipe gets the text as its
  last argument, Python's way (`cmd | f`), not on its standard input.
- Captured as a value, a command's words stop at Python operators:
  `x = echo 1 + 2` is `(echo 1) + 2`, and `y = expr 7 % 2` the same. Quote
  the operator: `y = expr 7 "%" 2` (A-58). On a line of its own, as a
  command, they're words.

## Elsewhere
<!-- help: abs-gaps-elsewhere -->

- A Python call redirected to a named pipe (`print hi >fifo`) opens it in
  tessh, so it waits there for the other end; a program doesn't.
- Starting a program costs about 20 ms, several times what bash takes:
  loops that run thousands of tiny programs (`expr`) are slow; do the work
  in Python instead.
