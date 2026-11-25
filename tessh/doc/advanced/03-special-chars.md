# 3. Special Characters
<!-- help: abs-special-chars abs-3 special-characters metacharacters punctuation -->

A character is special when it means more than itself: `#` starts a
comment, `|` makes a pipe, `$` reads a variable. The ABS Guide goes through
bash's special characters one by one. tessh shares many of them, since its
command lines are bash's, but because the rest of a script is Python some
mean something else, and a few aren't needed at all. This chapter goes
through the same list.

## 3.1 Comments, separators and dots
<!-- help: abs-comments-separators comment # ; ;; dot source -->

`#` starts a comment, at the start of a line or after a command (with a
space before it). Quoted or escaped, it's an ordinary character. Where bash
has `#` in `${PATH#*:}` and in `$(( 2#101011 ))`, tessh uses Python: a
string method and `int text base` (or a `0b` literal).

```bash
echo "The # here does not begin a comment."
echo 'The # here does not begin a comment.'
echo The \# here does not begin a comment.
echo The # here begins a comment.

PATH=/usr/local/bin:/bin:/usr/bin
echo ${PATH#*:}       # Parameter substitution, not a comment.
echo $(( 2#101011 ))  # Base conversion, not a comment.
```

```tessh
echo "The # here does not begin a comment."
echo 'The # here does not begin a comment.'
echo The \# here does 'not' begin a comment.
echo The # here begins a comment.

path = "/usr/local/bin:/bin:/usr/bin"
print ($path.partition ":")[2]   # Everything after the first colon.
print (int "101011" 2)           # Base conversion (or write 0b101011).
```

```output
The # here does not begin a comment.
The # here does not begin a comment.
The # here does not begin a comment.
The
/bin:/usr/bin
43
```

`;` puts two commands on one line, in both. An `if` in tessh has no `then`
or `fi` to separate, so the ABS example's `if [ -x "$filename" ]; then`
becomes an indented block:

```bash
filename=notes.txt
echo hello; echo there

if [ -e "$filename" ]; then    #  Note the space after the semicolon.
  echo "File $filename exists."; cp $filename $filename.bak
else
  echo "File $filename not found."; touch $filename
fi; echo "File test complete."
```

```tessh
import os
filename = notes.txt
echo hello; echo there

if (os.path.exists $filename)
  echo "File $filename exists."; cp $filename "${filename}.bak"
else
  echo "File $filename not found."; touch $filename
echo "File test complete."
```

```output
hello
there
File notes.txt not found.
File test complete.
```

`;;` ends a `case` branch in bash; tessh's `match` has a `case` per branch
and an indented body, so there's nothing to end. Bash 4's `;&` and `;;&`
(go on into the next branch, or test the next patterns too) have no
counterpart: a Python `match` runs one branch. Put the shared code in a
function, or join patterns with `|`.

```bash
variable=xyz
case "$variable" in
  abc)  echo "\$variable = abc" ;;
  xyz)  echo "\$variable = xyz" ;;
esac
```

```tessh
variable = xyz
match $variable
  case abc
    print "\$variable = abc"
  case xyz
    print "\$variable = xyz"
```

```output
$variable = xyz
```

`.` is bash's `source`. tessh has `source file.tsh` (and `import name` for
a `.tsh` module), not `.`. As part of a name, a leading dot hides a file
from `ls` (and from tessh's globs, as from bash's); `.` is the current
folder and `..` its parent, in both. In a regular expression `.` is any one
character: in tessh `re'a.c'`.

## 3.2 Quotes, commas, slashes and backquotes
<!-- help: abs-quote-chars quotes comma backslash slash backquote backticks -->

- `"…"` and `'…'` quote, as in bash: `$name` and `$(…)` are filled in
  between double quotes, not between single ones. Chapter 5 has the
  details; the main difference is that a tessh variable is never split into
  words, quoted or not.
- `\` escapes one character, as in bash: `\$5`, `a\;b`, `\*`.
- `/` separates the parts of a path and divides numbers. A bareword that
  starts with `/`, `./`, `../` or `~` is a Path, so `$p.parent`, `$p.name`
  and `$p / more` work.
- `` `command` `` is bash's old command substitution. tessh has no
  backquoted commands: a command used as a value is captured (`x = (date)`,
  `"today is $(date +%A)"`). Backticks in tessh make a function infix
  (``3 `max` 9``).
- `,` is bash's arithmetic comma operator (`let "t2 = ((a = 9, 15 / 3))"`:
  every part is done, the last is the value). tessh has no comma operator; a
  `do` block is the same idea, its value the last form's.

```bash
let "t2 = ((a = 9, 15 / 3))"
echo $a $t2
```

```tessh
t2 = do
  a = 9
  15 // 3
print $a $t2
```

```output
9 5
```

`,` also builds words in brace expansion, `/{,usr/}bin/*calc`. tessh has no
brace expansion (see 3.6): a list comprehension or two globs do it.

```bash no-run
for file in /{,usr/}bin/*calc
#             ^    Find all executable files ending in "calc"
#+                 in /bin and /usr/bin directories.
do
        if [ -x "$file" ]
        then
          echo $file
        fi
done
```

```tessh no-run
import os
for file in (glob /bin/*calc /usr/bin/*calc)
  if (os.access $file $os.X_OK)
    print $file
```

(`glob` takes several patterns. It's `no-run` here because what it finds
depends on the system.)

## 3.3 The null command, `!` and wildcards
<!-- help: abs-colon-bang-wildcards colon null-command : ! not wildcard * ? -->

`:` does nothing and succeeds, in tessh as in bash, so `while :` and
`: >file` work. In tessh `pass` is the do-nothing statement for an empty
block, and `while True` the endless loop. bash's `: ${username=$(whoami)}`
(give a default) is `username = $env.get "USER" "nobody"` or similar, and
`: ${HOSTNAME?} ${USER?}` (stop if not set) is a plain test that raises.

```bash
: > data.xxx   # File "data.xxx" now empty.
echo $?
not_empty ()
{
  :
} # Contains a : (null command), and so is not empty.
not_empty && echo "function ran"
ls -l data.xxx | cut -d' ' -f5
```

```tessh
import os
: > data.xxx   # File "data.xxx" now empty.
print $?
def not_empty
  pass         # Python's null statement.
not_empty
print "function ran"
print (os.path.getsize data.xxx)
```

```output
0
function ran
0
```

`:` also separates the fields of `/etc/passwd` and `$PATH`; in tessh
`$PATH.split ":"` gives the folders as a list.

`!` turns a command's exit status around (`! grep -q x file`), and `!=` is
"not equal". tessh has `!` before a command (`! grep -q x file && …`) and
`not` for any value, a command's result included: `if not (grep -q x
file)`. History expansion (`!!`, `!$`, `^old^new^`…) is the REPL's
business: tessh has it at the prompt, not in a script (Appendix L).

`*` is the wildcard of globbing, and `?` matches one character, in both.
In tessh a pattern written bare is a Glob, a value you can loop over and
test, and the same characters in `re'…'` are a regular expression's. `*` is
also multiplication and `**` power, as in bash's `$(( ))`; in tessh that's
any arithmetic, not only inside `(( ))`.

```tessh
touch a.py b.py c.txt
echo *.py ?.txt
print (2 * 3) (2 ** 10) ("-" * 5)
```

```output
a.py b.py c.txt
6 1024 -----
```

`?` is also part of bash's C-style "trinary" operator in `(( ))`. tessh's
is `if … then … else …`, which works anywhere:

```bash
var1=50
(( var0 = var1<98?9:21 ))
echo $var0
```

```tessh
var1 = 50
var0 = if $var1 < 98 then 9 else 21
print $var0
```

```output
9
```

## 3.4 The dollar sign
<!-- help: abs-dollar dollar $ $? $$ $* $@ variable-substitution -->

- `$name` is a variable's value in both. In tessh it's a Python value, not
  text, and an environment variable when no Python name has that name.
- `$` ends a line in a regular expression, as in tessh's `re'…'`.
- `${…}`: bash's parameter substitution (`${var:-default}`, `${var#pat}`…)
  is Python in tessh (`$var or default`, `$var.removeprefix …`), and
  `${expr}` is any Python expression, as in an f-string.
- `$'…'` gives escapes (`$'\t'`, `$'\x41'`) in bash; tessh's double-quoted
  strings already take them: `"\t"`, `"\x41"`.
- `$*` and `$@` are the arguments: tessh's `$argv[1:]`, a list.
- `$?` is the last command's exit status, in both.
- `$$` is the shell's process ID: `os.getpid` in tessh.

```bash
var1=5
var2=23skidoo

echo $var1     # 5
echo $var2     # 23skidoo
echo $'tab\there' $'\x41'
```

```tessh
var1 = 5
var2 = 23skidoo

print $var1  # 5
print $var2  # 23skidoo
print "tab\there" "\x41"
```

```output
5
23skidoo
tab	here A
```

## 3.5 Groups, blocks and brackets
<!-- help: abs-groups-blocks subshell code-block ( ) { } [ ] [[ ]] (( )) $[ ] -->

`( … )` runs commands in a subshell, whose variables vanish when it ends.
tessh runs no subshells for its own code; a `let` block is the way to have
names that only the block sees.

```bash
a=123
( a=321; )
echo "a = $a"   # a = 123
# "a" within parentheses acts like a local variable.
```

```tessh
a = 123
let a=321
  pass
print "a = $a"   # a = 123
```

```output
a = 123
```

`{ … }` groups commands without a subshell, so its variables stay. That's
just lines of code in tessh (or a `do` block, when you want one value).
`( … )` also makes a bash array, `Array=(element1 element2)`; tessh's is a
list, `Array = [element1 element2]`, and `$Array[1]` an element.

```bash
a=123
{ a=321; }
echo "a = $a"   # a = 321   (value inside code block)
```

```tessh
a = 123
a = 321
print "a = $a"   # a = 321
```

```output
a = 321
```

A bash code block can have its input or output redirected as a whole. In
tessh a function call can (`f >file`), and for a file you read from, `with
open` is the usual way.

### Example 3-1. Code blocks and I/O redirection (`ex8.sh`)

```bash no-run
#!/bin/bash
# Reading lines in /etc/fstab.

File=/etc/fstab

{
read line1
read line2
} < $File

echo "First line in $File is:"
echo "$line1"
echo
echo "Second line in $File is:"
echo "$line2"

exit 0

# Now, how do you parse the separate fields of each line?
# Hint: use awk, or . . .
# . . . Hans-Joerg Diers suggests using the "set" Bash builtin.
```

The `with` block is the code block, and `$f.readline` is `read` (with the
newline left on, so it's stripped). The answer to the closing question in
tessh is `.split`: a line's fields are a list.

```tessh
#!/usr/bin/env tessh
# (A made-up /etc/fstab to try it on.)
printf "UUID=4b1c / ext4 defaults 0 1\nUUID=9e0a /home btrfs subvol=home 0 2\n" >fstab

File = ./fstab               # (/etc/fstab in the original)

with (open $File) as f
  line1 = ($f.readline).rstrip "\n"
  line2 = ($f.readline).rstrip "\n"

print "First line in $File is:"
print $line1
print
print "Second line in $File is:"
print $line2
print "Its fields:" (.split $line2)

exit 0
```

```output
First line in ./fstab is:
UUID=4b1c / ext4 defaults 0 1

Second line in ./fstab is:
UUID=9e0a /home btrfs subvol=home 0 2
Its fields: ['UUID=9e0a' '/home' 'btrfs' 'subvol=home' '0' '2']
```

### Example 3-2. Saving the output of a code block to a file (`rpm-check.sh`)

```bash no-run
#!/bin/bash
# rpm-check.sh

#  Queries an rpm file for description, listing,
#+ and whether it can be installed.
#  Saves output to a file.
# 
#  This script illustrates using a code block.

SUCCESS=0
E_NOARGS=65

if [ -z "$1" ]
then
  echo "Usage: `basename $0` rpm-file"
  exit $E_NOARGS
fi  

{ # Begin code block.
  echo
  echo "Archive Description:"
  rpm -qpi $1       # Query description.
  echo
  echo "Archive Listing:"
  rpm -qpl $1       # Query listing.
  echo
  rpm -i --test $1  # Query whether rpm file can be installed.
  if [ "$?" -eq $SUCCESS ]
  then
    echo "$1 can be installed."
  else
    echo "$1 cannot be installed."
  fi  
  echo              # End code block.
} > "$1.test"       # Redirects output of everything in block to file.

echo "Results of rpm test in file $1.test"

# See rpm man page for explanation of options.

exit 0
```

The block becomes a function, and its call is redirected, as bash's block
is: everything it prints, its programs' output included, goes to the
file. `rpm -i --test`'s exit status is its value's `.ok`. Without an
argument the script stops with its usage message, which is what it does
here.

```tessh
#!/usr/bin/env tessh
SUCCESS = 0
E_NOARGS = 65

if (len $argv) < 2 or not $argv[1]
  print "Usage: $(Path $argv[0]).name rpm-file"
  exit $E_NOARGS
rpm_file = $argv[1]

def report                    # Begin code block.
  print
  print "Archive Description:"
  rpm -qpi $rpm_file          # Query description.
  print
  print "Archive Listing:"
  rpm -qpl $rpm_file          # Query listing.
  print
  rpm -i --test $rpm_file     # Query whether rpm file can be installed.
  if $? == $SUCCESS
    print "$rpm_file can be installed."
  else
    print "$rpm_file cannot be installed."
  print                       # End code block.

report >"${rpm_file}.test"    # Redirects output of everything in block to file.

print "Results of rpm test in file ${rpm_file}.test"
exit 0
```

```output
Usage: ex.tsh rpm-file
```

Bash runs a braced block that's part of a pipe in a subshell, so `ls | {
read firstline; read secondline; }` can't set variables for the rest of the
script. In tessh a pipe's last stage can be a Python function, which runs
in the script itself:

```tessh
printf "a.txt\nb.txt\nc.txt\n" | .splitlines | print
lines = printf "a.txt\nb.txt\nc.txt\n"
[firstline secondline *_] = $lines.lines
print "First line is $firstline; second line is $secondline"
```

```output
['a.txt' 'b.txt' 'c.txt']
First line is a.txt; second line is b.txt
```

`{}` is also `xargs -i`'s and `find -exec`'s placeholder, with `\;` ending
`-exec`; those are the programs' own syntax and work the same in tessh.

`[ … ]` is bash's `test`, and `[[ … ]]` its better test keyword. In tessh a
test is any Python expression (`$a == $b`, `(Path $f).is_file`, `$x > 3`);
chapter 7 shows them all. `[ ]` is also an array's index and a regular
expression's set of characters, in both.

`$[ … ]` and `(( … ))` are bash's integer arithmetic. tessh's arithmetic is
Python's, everywhere, and not only for integers.

```bash
a=3
b=7
echo $[$a+$b]   # 10
echo $[$a*$b]   # 21
echo $(( a / b )) $(( 7 / 2 ))
```

```tessh
a = 3
b = 7
print ($a + $b)   # 10
print ($a * $b)   # 21
print ($a // $b) (7 // 2) (7 / 2)
```

```output bash
10
21
0 3
```

```output
10
21
0 3 3.5
```

## 3.6 Brace expansion
<!-- help: abs-brace-expansion brace-expansion {a,b} {a..z} -->

Bash expands `{a,b,c}` and `{a..z}` into several words before a command
runs. By design, tessh has no brace expansion (Python makes lists well); `{…}` is a dict or a set, and
glued to text it's not expanded (`echo file{1,2}` prints `file{'1,2'}`).
Python makes the lists (a comprehension, `range`, `string`'s alphabets), and a
list given to a command is one argument per item.

```bash
echo \"{These,words,are,quoted}\"   # " prefix and suffix
echo {a..z} # a b c d e f g h i j k l m n o p q r s t u v w x y z
echo {0..3} # 0 1 2 3
touch file1 file2 file3 file22.txt
cp file22.{txt,backup}
echo file22.*
echo {file1,file2}\ :{\ A," B",' C'}
```

```tessh
import string
echo ['"' + $w + '"' for w in [These words are quoted]]
echo (list $string.ascii_lowercase)
echo (list (range 4))
touch file1 file2 file3 file22.txt
cp ["file22.$ext" for ext in [txt backup]]
echo file22.*
echo ["$f :$s" for f in [file1 file2] for s in [" A" " B" " C"]]
```

```output
"These" "words" "are" "quoted"
a b c d e f g h i j k l m n o p q r s t u v w x y z
0 1 2 3
file22.backup file22.txt
file1 : A file1 : B file1 : C file2 : A file2 : B file2 : C
```

The ABS Guide's `base64_charset=( {A..Z} {a..z} {0..9} + / = )` is
`list (string.ascii_uppercase + string.ascii_lowercase + string.digits + "+/=")`.

## 3.7 Redirection and pipes
<!-- help: abs-redirection-pipes > >> &> >& < <> | >| pipe filter -->

The redirections are bash's, in tessh: `>file`, `>>file`, `&>file`,
`>&2`, `<file`, `[i]<>file`, `<<` (a here document), `<<<` (a here string),
`<(…)` and `>(…)` (process substitution). `>|` writes even over an existing
file with `noclobber` set (`set -C`), in tessh too. Chapter 20 has them all.

`&>/dev/null` is the usual way to ask whether a command works without
seeing its output. Asking whether a command exists, tessh has `which`, whose
value is the path (or `None`, false):

```bash
command_test () { type "$1" &>/dev/null; }
#                                      ^

cmd=rmdir            # Legitimate command.
command_test $cmd; echo $?   # 0

cmd=bogus_command    # Illegitimate command
command_test $cmd; echo $?   # 1
```

```tessh
import shutil
def command_test cmd = (shutil.which $cmd) is not None

cmd = rmdir                  # Legitimate command.
print (command_test $cmd)    # True

cmd = bogus_command          # Illegitimate command
print (command_test $cmd)    # False
```

```output bash
0
1
```

```output
True
False
```

`<` and `>` compare strings in bash's `[[ ]]` (in dictionary order) and
numbers in `(( ))`. In tessh they compare any two values of a kind, text
included:

```bash
veg1=carrots
veg2=tomatoes

if [[ "$veg1" < "$veg2" ]]
then
  echo "Although $veg1 precede $veg2 in the dictionary,"
  echo -n "this does not necessarily imply anything "
  echo "about my culinary preferences."
else
  echo "What kind of dictionary are you using, anyhow?"
fi
```

```tessh
veg1 = carrots
veg2 = tomatoes

if $veg1 < $veg2
  print "Although $veg1 precede $veg2 in the dictionary,"
  print "this does not necessarily imply anything " end=""
  print "about my culinary preferences."
else
  print "What kind of dictionary are you using, anyhow?"
```

```output
Although carrots precede tomatoes in the dictionary,
this does not necessarily imply anything about my culinary preferences.
```

`\<` and `\>` are word boundaries in a `grep` pattern, `grep '\<the\>'
file`: the same in tessh, since it's grep's. In a tessh `re'…'` it's `\b`.

`|` passes one command's output to the next. In tessh a stage can also be
a Python function or method, and the first stage a value:

```bash
echo ls -l | sh | head -1 | cut -c1-5
printf "b\na\nb\n" > 1.lst ; printf "c\na\n" > 2.lst
cat *.lst | sort | uniq
# Merges and sorts all ".lst" files, then deletes duplicate lines.
echo hello | tr 'a-z' 'A-Z'
```

```tessh
echo ls -l | sh | head -1 | cut -c1-5
printf "b\na\nb\n" > 1.lst ; printf "c\na\n" > 2.lst
cat *.lst | sort | uniq
# The same, in Python:
print *(sorted {*(cat *.lst).lines}) sep="\n"
echo hello | str.upper | print
```

```output bash
total
a
b
c
HELLO
```

```output
total
a
b
c
a
b
c
HELLO
```

The ABS Guide's `uppercase.sh` (`tr 'a-z' 'A-Z'`) is a filter, a program
that reads its input and writes it changed. A tessh script is a filter the
same way, reading `sys.stdin`:

```tessh
print "import sys\nfor line in \$sys.stdin\n  print (\$line.upper) end=''" >uppercase.tsh
printf "total 2\n-rw-r--r-- 1 bozo bozo 109 1.txt\n" | tessh uppercase.tsh
```

```output
TOTAL 2
-RW-R--R-- 1 BOZO BOZO 109 1.TXT
```

A bash pipe's stages run in subshells, so `echo "new_value" | read variable`
leaves `variable` as it was. tessh has no subshells: its `read` runs in
the shell, so `echo "new_value" | read variable` does set `variable`. A
value also comes out of a pipe by capturing it, or by a function at its
end:

```bash
variable="initial_value"
echo "new_value" | read variable
echo "variable = $variable"     # variable = initial_value
```

```tessh
variable = "initial_value"
variable = echo "new_value" | tr a-z A-Z
print "variable = $variable"
```

```output bash
variable = initial_value
```

```output
variable = NEW_VALUE
```

`||` and `&&` are "or" and "and", between commands (by exit status) and in
tests. tessh has both, and `or` and `and` too.

## 3.8 Background jobs
<!-- help: abs-background & background-job job -->

A trailing `&` runs a command in the background. In tessh a function call
can go in the background too, and its value is a `Job`; `fg` and `wait`
wait for it, and `jobs`, `kill` and the rest of bash's job control work on
it (chapter 15.1).

### Example 3-3. Running a loop in the background (`background-loop.sh`)

```bash
#!/bin/bash
# background-loop.sh

for i in 1 2 3 4 5 6 7 8 9 10            # First loop.
do
  echo -n "$i "
done & # Run this loop in background.
       # Will sometimes execute after second loop.

echo   # This 'echo' sometimes will not display.

for i in 11 12 13 14 15 16 17 18 19 20   # Second loop.
do
  echo -n "$i "
done  

echo   # This 'echo' sometimes will not display.

# ======================================================

# The expected output from the script:
# 1 2 3 4 5 6 7 8 9 10 
# 11 12 13 14 15 16 17 18 19 20 

# Sometimes, though, you get:
# 11 12 13 14 15 16 17 18 19 20 
# 1 2 3 4 5 6 7 8 9 10 bozo $
# (The second 'echo' doesn't execute. Why?)

# Occasionally also:
# 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20
# (The first 'echo' doesn't execute. Why?)

# Very rarely something like:
# 11 12 13 1 2 3 4 5 6 7 8 9 10 14 15 16 17 18 19 20 
# The foreground loop preempts the background one.

exit 0

#  Nasimuddin Ansari suggests adding    sleep 1
#+ after the   echo -n "$i"   in lines 6 and 14,
#+ for some real fun.
```

A loop can't take `&` in tessh, but a function holding it can. The order of
the output is up to chance, as in bash. `fg` waits for the job, so the
script doesn't end before it does; `wait` does too, as in bash.

```tessh
#!/usr/bin/env tessh
def first_loop
  for i in (range 1 11)        # First loop.
    print $i end=" "

first_loop &                   # Run this loop in background.

print                          # Order not guaranteed.

for i in (range 11 21)         # Second loop.
  print $i end=" "

print
fg                             # Wait for the background loop.
exit 0
```

```text
(one run)
1 2 3 4 5 6 7 8 9 10 
11 12 13 14 15 16 17 18 19 20 
```

## 3.9 Dashes, tildes and the rest
<!-- help: abs-dash-tilde-misc dash - -- ~ = + % ^ whitespace ifs control-characters -->

`-` starts a command's options (`ls -al`); in tessh the same, as words.
bash's test operators `-eq`, `-ot`, `-a` are Python's `==`, a comparison of
`.stat.st_mtime`s, and `and` in tessh (chapter 7), and bash's
`${param1:-$DEFAULTVAL}` is `$param1 or $DEFAULTVAL`. `--` ends a program's
options, so `rm -- -badname` removes a file whose name starts with a dash.

As a file name, `-` means standard input or output to many programs (`cat
-`, `tar cf -`, `diff file2 -`). That's the program's doing, so it's the
same in tessh:

```bash
printf "Linux\nBSD\n" > file2 ; printf "Linux rocks\nGNU\n" > file1
grep Linux file1 | diff file2 -
echo "whatever" | cat -
```

```tessh
printf "Linux\nBSD\n" > file2 ; printf "Linux rocks\nGNU\n" > file1
grep Linux file1 | diff file2 -
echo "whatever" | cat -
```

```output
1,2c1
< Linux
< BSD
---
> Linux rocks
whatever
```

### Example 3-4. Backup of all files changed in last day (`ex58.sh`)

```bash no-run
#!/bin/bash

#  Backs up all files in current directory modified within last 24 hours
#+ in a "tarball" (tarred and gzipped file).

BACKUPFILE=backup-$(date +%m-%d-%Y)
#                 Embeds date in backup filename.
#                 Thanks, Joshua Tschida, for the idea.
archive=${1:-$BACKUPFILE}
#  If no backup-archive filename specified on command-line,
#+ it will default to "backup-MM-DD-YYYY.tar.gz."

tar cvf - `find . -mtime -1 -type f -print` > $archive.tar
gzip $archive.tar
echo "Directory $PWD backed up in archive file \"$archive.tar.gz\"."


#  Stephane Chazelas points out that the above code will fail
#+ if there are too many files found
#+ or if any filenames contain blank characters.

# He suggests the following alternatives:
# -------------------------------------------------------------------
#   find . -mtime -1 -type f -print0 | xargs -0 tar rvf "$archive.tar"
#      using the GNU version of "find".


#   find . -mtime -1 -type f -exec tar rvf "$archive.tar" '{}' \;
#         portable to other UNIX flavors, but much slower.
# -------------------------------------------------------------------


exit 0
```

`find`'s output, as `.lines`, goes to `tar` as a list: one argument per
file, so the blanks in names that Stephane Chazelas warns about do no harm.
(Too many files for one command line would still be a problem; his
`find … -print0 | xargs -0` works in tessh as it is.) `$argv[1:]` holds
the archive name, if given.

```tessh
#!/usr/bin/env tessh
# (Made-up files to back up, one of them old.)
touch notes.txt "my essay.txt" ; touch -d "3 days ago" old.txt

BACKUPFILE = "backup-$(date +%m-%d-%Y)"
#                 Embeds date in backup filename.
archive = ($argv[1:] or [$BACKUPFILE])[0]
#  If no backup-archive filename specified on command-line,
#+ it will default to "backup-MM-DD-YYYY.tar.gz."

files = (find . -mtime -1 -type f -print).lines
tar cvf - $files > "${archive}.tar"
gzip "${archive}.tar"
print "Directory $(pwd) backed up in archive file \"${archive}.tar.gz\"."
tar tzf "${archive}.tar.gz"
exit 0
```

```text
(one run)
./notes.txt
./my essay.txt
Directory /tmp/abs-ex-2tz4k8qa backed up in archive file "backup-11-23-2026.tar.gz".
./notes.txt
./my essay.txt
```

(`tar v` lists to standard error when the archive goes to standard output.)

`cd -` goes back to the folder you were in, in both. A minus is also
subtraction, `=` assignment (and, in bash's tests, string equality; tessh's
is `==`), `+` addition (and in bash `set +o`, an option turned off), and `%`
the remainder of a division, in both.

```bash
a=28
echo $a   # 28
let "z = 5 % 3"
echo $z  # 2
```

```tessh
a = 28
print $a   # 28
z = 5 % 3
print $z   # 2
```

```output
28
2
```

`~` is your home folder, at the start of a word, in tessh as in bash, and
`~bozo` bozo's. By design, bash's `~+` (the current folder, `$PWD`) and `~-`
(the last one, `$OLDPWD`) aren't expanded by tessh; use `(pwd)` and
`$OLDPWD`.

`=~` matches a regular expression in bash's `[[ ]]`. In tessh it matches a
`re'…'` (giving the match), a Glob, or a piece of text. `^` is the start of
a line in a regular expression, in both; bash 4's `${var^^}` (upper case)
is `$var.upper` in tessh.

### Control characters

A control character is typed with Ctrl and a key, and can be written in a
string as an escape. The ABS Guide lists what each does at a terminal:
Ctrl-C interrupts, Ctrl-D ends input (and logs out), Ctrl-Z stops a job,
Ctrl-S and Ctrl-Q freeze and resume output, Ctrl-H is a backspace, Ctrl-J a
newline, Ctrl-L clears the screen, Ctrl-M is a carriage return, and the rest
edit the command line. The tessh REPL handles Ctrl+C, Ctrl+Z and Ctrl+D as
bash does. In a string, `"\b"` (or `"\x08"`) is Ctrl-H, `"\n"` Ctrl-J,
`"\r"` Ctrl-M, `"\v"` Ctrl-K and `"\f"` Ctrl-L, as with bash's `$'…'`.
`cat -v` shows them:

```bash
a=$'\b\b'                 # Two Ctl-H's -- backspaces
echo -n "abcdef$a " | cat -v
echo
var=$'\x0aThis is the bottom line\x0bThis is the top line\x0a'
echo "$var" | cat -v
```

```tessh
a = "\b\b"                # Two Ctl-H's -- backspaces
echo -n "abcdef$a " | cat -v
echo
var = "\x0aThis is the bottom line\x0bThis is the top line\x0a"
echo $var | cat -v
```

```output
abcdef^H^H 

This is the bottom line^KThis is the top line

```

### Whitespace

Spaces, tabs and blank lines separate words and commands. Blank lines
don't change anything, so they set off parts of a script. In bash,
whitespace is not allowed around `=` in an assignment; in tessh it is (and
glued `a=1` works too). Indentation matters in tessh: it's what makes a
block. `$IFS`, bash's field separators, has no counterpart, since tessh
doesn't split variables into words; `.split` splits text where you ask.
Quoting keeps whitespace in a string, in both.
