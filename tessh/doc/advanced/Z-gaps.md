# Gaps: what tessh can't do yet
<!-- help: abs-gaps gaps missing todo bash-features-missing -->

Porting the ABS Guide's examples turned up what bash has and tessh hasn't.
Each one is marked **Gap** where it comes up; here they are together, with
what to use meanwhile. Some are by design (tessh's values are Python's, so
there's no word splitting and no `$IFS`), and those say so.

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
  `done`), `set -e`/`set -x`, a method not called given to a program.

## Builtins
<!-- help: abs-gaps-builtins read eval exec trap wait type readonly -->

- **`read`**: no builtin. `input` reads a line; splitting it into names is
  `[a b c] = (input).split`, and `-s`, `-n`, `-t`, `-p` need `termios`,
  `tty` and `select` (chapter 15).
- **`eval`** of tessh text: none. Write it to a file and `source` it, or
  use a dict or `globals()`; `env *(shlex.split $text)` runs a command line
  held in text (chapters 15, A-30).
- **`exec`**: neither `exec prog` (use `os.execvp`, after
  `sys.stdout.flush`) nor `exec >file` / `exec 3<file` for the rest of a
  script (reassign `$sys.stdout`, or `os.dup2`) (chapters 15, 20).
- **`trap`**: none. `try`/`finally`, `atexit`, `signal.signal` and
  `KeyboardInterrupt` do its jobs (chapter 32).
- **`set -e`, `-u`, `-x`, `-v`, `-o pipefail`, `shopt`**: none (a `set -x`
  line is an error). Python errors stop a script and unset names are
  always errors; a failing *program* doesn't stop one (no errexit), and a
  pipeline's status is its last stage's (no pipefail, no `$PIPESTATUS`).
  `sys.settrace` traces (chapters 32, 33).
- **`wait`** for all jobs, **`$!`**, a Job's process ID, `kill %1`,
  `disown`: a `Job` has only `.n` and `.wait` (chapters 15, 16, 29).
- **`type`**, `command -v`, `hash`: nothing says whether a name is a
  builtin, a function or a program (`which` finds programs).
- **`readonly`** / `declare -r`: no read-only names (chapters 9, 15).
- **`source file args`**: the arguments are dropped.
- **`select`** menus, **`coproc`**, **`mapfile`**: a few lines of Python
  each (chapters 11, 37).
- **`time`** keyword: `time` runs `/usr/bin/time`, so a pipeline, a Python
  call or a block can't be timed with it (chapter 16).
- **`alias`**: none; `@command def` (chapter 25).
- **`builtin cd`** once `cd` is a function: keep `real = $cd` beforehand.

## Variables
<!-- help: abs-gaps-variables $UID $RANDOM $LINENO $SECONDS -->

None of bash's internal variables are tessh variables; each has a Python
form, listed in chapter 9: `$UID`/`$EUID` (`os.getuid`), `$$`/`$PPID`
(`os.getpid`, `os.getppid`), `$RANDOM` (`random.randrange 32768`),
`$SECONDS` (`time.monotonic`), `$LINENO`/`$FUNCNAME`/`caller` (`inspect`),
`$HOSTNAME`/`$OSTYPE` (`socket`, `sys.platform`), `$BASH_VERSION`, `$_`,
`$-`, `$REPLY`, `$DIRSTACK` (`dirs` only prints), `$BASH_SUBSHELL`,
`$SHLVL`, `$GLOBIGNORE`, `$PS1`–`$PS4`, `$PROMPT_COMMAND`. `$IFS` is by
design: tessh never splits a value into words.

## Words and expansion
<!-- help: abs-gaps-expansion brace-expansion posix-classes -->

- **Brace expansion** (`file{1,2}`, `{a..z}`, `{b*,c*}`): none; a list
  comprehension or `glob` with several patterns. `{…}` glued to text is a
  set literal, without an error (chapters 3, 18, 33).
- **POSIX classes** (`[[:digit:]]`): not in globs, and `re'[[:digit:]]'`
  silently means something else (use `\d`, or `str.isdigit`) (chapter 18).
- **`${s#pat}`, `##`, `%`, `%%` and `${s/pat/rep}` with glob patterns**:
  `removeprefix`, `removesuffix`, `partition`, `re.sub`, or `fnmatch` by hand
  (chapter 10). `${!prefix*}`, `${!name}`: `globals()`.
- **`~+`, `~-`** (current and previous folder): `(pwd)` and `$env.OLDPWD`.
- `\z` (a backslash before an ordinary letter) outside quotes stays `\z`;
  `echo foo\` glued to the end of a line doesn't continue it (with a space,
  `\` does).
- A pattern held in text: `glob $p` doesn't glob it; `Glob $p` does.
- `*[1 2]` (splatting a list literal) is read as a glob; give the list a
  name, or write `*(list …)`.
- A keyword argument with a list value, `f languages=[fr]`, is read as a
  glob; write `languages=([fr])`.

## Structure
<!-- help: abs-gaps-structure subshell group pipeline-stage -->

- **A function or loop as a pipeline stage**: `f | sort` pipes `f`'s
  return value, not what it prints; `for … done | sort` has no form.
  Collect the lines in a list and pipe that, or redirect the call to a
  file (chapters 11, 12, 19).
- **Command groups**: `( a; b ) | c` and `{ a; b; } >f` have no form; a
  function with its call redirected does the second (chapters 16, 19, 20).
- **Subshells**: `( … )` is a Python form, not a child process; `os.fork`
  (a small `@subshell` helper is in chapter 21).
- **Redirection after a block** (`done <file`, `fi >log`) and a trailing
  `&` on a block: put the block in a function (chapters 3, 20).
- `<file cmd` (redirection first) and a bare `>file` line are errors.
- No `|&`.
- `break N`, `continue N`: a function and `return`, or `for … else`.
- `case` fall-through (`;&`, `;;&`): no `match` counterpart.
- A bareword next to `and`/`or` is a word, by design (`print it is
  ready`), so `$x == PREV and …` and `$x or default` need the word quoted.
- Captured as a value, a command's words stop at Python operators:
  `x = echo 1 + 2` is `(echo 1) + 2`, and `y = expr 7 % 2` the same. Quote
  the operator: `y = expr 7 "%" 2` (A-58). On a line of its own, as a
  command, they're words.

## Elsewhere
<!-- help: abs-gaps-elsewhere dev-tcp completion history prompt -->

- **`/dev/tcp/host/port`**: not a redirection; Python's `socket` (chapter 29).
- **Programmable completion** (`complete`, `compgen`): the REPL completes
  names, programs and paths, not a script's own options (Appendix J).
- **History**: no `history`, `fc` or `!` expansion (Appendix L).
- **The prompt** can't be set (`PS1`), and there's no logout file
  (Appendices H, M).
- **Restricted mode**: none, and with Python underneath it couldn't mean
  much (chapter 22).
- A background command redirected to or from a named pipe opens it before
  starting, so it can block the shell until the other end opens (A-14).
- A script's exit status is its last *program's*, even when Python lines
  come after it: end with `exit 0` where it matters.
- Docstrings in tessh functions are dropped.
- Starting a program costs about 20 ms, several times what bash takes:
  loops that run thousands of tiny programs (`expr`) are slow; do the work
  in Python instead.
