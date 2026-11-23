# Appendix B. Reference Cards
<!-- help: abs-refcards abs-b reference-cards cheat-sheet quick-reference -->

The ABS Guide's reference cards summarize bash's special variables, test
operators, parameter substitutions, string operations and bracket
constructs. Here they are again, each with what tessh writes instead.
The chapters say more.

## B.1 Special shell variables
<!-- help: abs-refcard-variables $0 $1 $# $* $@ $? $$ $! $_ argv -->

| bash | meaning | tessh |
|---|---|---|
| `$0` | the script's name | `$argv[0]` |
| `$1`, `$2` … `$9`, `${10}` | positional parameters | `$argv[1]`, `$argv[2]` … `$argv[10]` |
| `$#`, `${#*}`, `${#@}` | how many | `len $argv[1:]` |
| `"$*"` | all, as one word | `" ".join $argv[1:]` |
| `"$@"` | all, as separate words | `$argv[1:]` (a list: one argument per item) |
| `$?` | the last command's status | `$?`, or a command value's `.code` |
| `$$` | the script's process ID | `os.getpid` |
| `$-` | the shell's option flags | none (tessh has no `set` options) |
| `$_` | the last argument of the last command | none: keep the command as a list |
| `$!` | the last background job's PID | `j = cmd &` gives a `Job` |

## B.2 Test operators: comparisons
<!-- help: abs-refcard-comparison -eq -ne -lt -gt = != -z -n comparison -->

| bash (numbers) | bash (strings) | meaning | tessh (both) |
|---|---|---|---|
| `-eq`, `(( a == b ))` | `=`, `==` | equal | `==` |
| `-ne` | `!=` | not equal | `!=` |
| `-lt`, `(( < ))` | `\<` (`<` in `[[ ]]`) | less than | `<` |
| `-le`, `(( <= ))` | | less than or equal | `<=` |
| `-gt`, `(( > ))` | `\>` (`>` in `[[ ]]`) | greater than | `>` |
| `-ge`, `(( >= ))` | | greater than or equal | `>=` |
| | `-z` | string is empty | `not $s` |
| | `-n` | string isn't empty | `$s` (true when not empty) |

tessh compares numbers as numbers and strings as strings; `4 == "4"` is
false, so read a number with `int`.

## B.3 Test operators: files
<!-- help: abs-refcard-files -e -f -d -s -r -w -x -nt -ot -ef file-tests -->

With `p = Path name` (or a bareword path such as `./x`), and `import os`:

| bash | true if… | tessh |
|---|---|---|
| `-e` | it exists | `($p.exists)` |
| `-f` | regular file | `($p.is_file)` |
| `-d` | directory | `($p.is_dir)` |
| `-s` | not empty | `(os.path.getsize $p) > 0` |
| `-h`, `-L` | symbolic link | `($p.is_symlink)` |
| `-b`, `-c` | block, character device | `($p.is_block_device)`, `($p.is_char_device)` |
| `-p`, `-S` | pipe, socket | `($p.is_fifo)`, `($p.is_socket)` |
| `-t fd` | a terminal | `os.isatty fd` |
| `-r`, `-w`, `-x` | readable, writable, runnable | `os.access $p $os.R_OK` (`W_OK`, `X_OK`) |
| `-g`, `-u`, `-k` | sgid, suid, sticky | `(os.stat $p).st_mode & $stat.S_ISGID` (`S_ISUID`, `S_ISVTX`) |
| `-O`, `-G` | yours, your group's | `(os.stat $p).st_uid == (os.getuid)` (`st_gid`, `os.getgid`) |
| `-N` | changed since last read | `(os.stat $p).st_mtime > (os.stat $p).st_atime` |
| `F1 -nt F2`, `-ot` | newer, older | `(os.path.getmtime F1) > (os.path.getmtime F2)` |
| `F1 -ef F2` | the same file | `os.path.samefile F1 F2` |
| `!` | not | `not` |

## B.4 Parameter substitution and expansion
<!-- help: abs-refcard-paramsub ${var:-default} ${var:=} ${var:+} ${var:?} default-value -->

A tessh variable that was never set is an error, not an empty string, so
"not set" and "empty" are told apart in different ways: an environment
variable with `$env.get NAME default`, a tessh one with `(globals).get
"name" default`, and "empty" by the value's truth.

| bash | meaning | tessh |
|---|---|---|
| `${var}` | var's value | `$var`, `${var}` |
| `${var-$DEFAULT}` | `$DEFAULT` if var isn't set | `$env.get var $DEFAULT` |
| `${var:-$DEFAULT}` | … or is empty | `$var or $DEFAULT` |
| `${var=$DEFAULT}`, `${var:=$DEFAULT}` | the same, and assign it | `var = $var or $DEFAULT` |
| `${var+$OTHER}`, `${var:+$OTHER}` | `$OTHER` if var is set (not empty), else "" | `if $var then $OTHER else ""` |
| `${var?$ERR_MSG}`, `${var:?$ERR_MSG}` | stop with `$ERR_MSG` if not set | `if not $var` … `raise (SystemExit $ERR_MSG)` |
| `${!varprefix*}`, `${!varprefix@}` | the names starting with varprefix | `[$n for n in (globals) if ($n.startswith "varprefix")]` |
| `${!variable}` | indirect reference | `(globals)[$variable]`, `$env[$variable]` |

## B.5 String operations
<!-- help: abs-refcard-strings ${#string} ${string:pos:len} ${string#} ${string%} ${string/} expr substring -->

| bash | meaning | tessh |
|---|---|---|
| `${#string}` | length | `len $string` |
| `${string:position}` | from position on (from 0) | `$string[position:]` |
| `${string:position:length}` | length characters from position | `$string[position:position + length]` |
| `${string#substring}` | strip the shortest match from the front | `$string.removeprefix "text"`; for a pattern, `re.sub` with a non-greedy `*?` (`r"^a.*?C"`) |
| `${string##substring}` | … the longest | `re.sub` with a greedy `*` (`r"^a.*C"`) |
| `${string%substring}` | strip the shortest match from the back | `$string.removesuffix "text"`, `re.sub r"pat$" "" $string` |
| `${string%%substring}` | … the longest | `re.sub r"pat$" "" $string` |
| `${string/substring/replacement}` | replace the first match | `$string.replace old new 1`, `re.sub pat new $string count=1` |
| `${string//substring/replacement}` | replace all | `$string.replace old new`, `re.sub pat new $string` |
| `${string/#substring/replacement}` | replace at the front | `re.sub r"^pat" new $string` |
| `${string/%substring/replacement}` | replace at the back | `re.sub r"pat$" new $string` |
| `expr match "$string" '$re'`, `expr "$string" : '$re'` | length of the match at the start | `len ((re.match pat $string).group 0)` |
| `expr index "$string" $chars` | where the first of chars is (from 1) | `$string.find c` (from 0, -1 if none) |
| `expr substr $string $position $length` | substring (from 1) | `$string[position - 1:position - 1 + length]` |
| `expr match "$string" '\($re\)'` | the match, from the start | `(re.match pat $string).group 1` |
| `expr "$string" : '.*\($re\)'` | the match, from the end | `(re.search r".*(pat)" $string).group 1` |

```bash
string=abcABC123ABCabc
echo ${#string} ${string:3} ${string:3:2}
echo ${string#a*C} ${string##a*C}
echo ${string/abc/xyz} ${string//abc/xyz}
expr index "$string" C
```

```tessh
import re
string = abcABC123ABCabc
print (len $string) $string[3:] $string[3:5]
print (re.sub r"^a.*?C" "" $string) (re.sub r"^a.*C" "" $string)
print ($string.replace abc xyz 1) ($string.replace abc xyz)
print (($string.find "C") + 1)
```

```output
15 ABC123ABCabc AB
123ABCabc abc
xyzABC123ABCabc xyzABC123ABCxyz
6
```

## B.6 Miscellaneous constructs
<!-- help: abs-refcard-misc brackets braces parentheses quoting command-substitution -->

| bash | meaning | tessh |
|---|---|---|
| `if [ CONDITION ]`, `if [[ CONDITION ]]` | test | `if CONDITION` (a Python expression), or `if test …` |
| `Array[1]=element1`, `Array=(e1 e2 e3)` | arrays | `Array = [e1 e2 e3]`, `$Array[1] = element1` |
| `[a-z]` | a range in a regular expression or glob | the same, in `re'…'` and in globs |
| `${variable}` | parameter substitution | `${expr}`, any expression |
| `${!variable}` | indirect reference | `(globals)[$variable]` |
| `{ cmd1; cmd2; }` | block of code | indented lines (`do` for a value) |
| `{a,b,c}`, `{a..z}` | brace expansion | none: a list or comprehension |
| `{}` | find's and xargs's placeholder | the same (the programs') |
| `( cmd1; cmd2 )` | subshell | `let` (names for a block only) |
| `result=$(COMMAND)`, `` result=`COMMAND` `` | command substitution | `result = COMMAND` |
| `>(COMMAND)`, `<(COMMAND)` | process substitution | the same |
| `(( var = 78 ))`, `var=$(( 20 + 5 ))` | arithmetic | `var = 78`, `var = 20 + 5` |
| `(( var++ ))`, `(( var-- ))` | increment, decrement | `var += 1`, `var -= 1` |
| `(( var0 = var1<98?9:21 ))` | conditional | `var0 = if $var1 < 98 then 9 else 21` |
| `"$variable"` | weak quoting | `"$variable"` (or `$variable`: never split) |
| `'string'` | strong quoting | `'string'` |
