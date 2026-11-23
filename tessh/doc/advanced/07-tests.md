# 7. Tests
<!-- help: abs-tests abs-7 tests test if conditions -->

A language has to be able to test a condition and act on the answer. Bash
has the `test` command, its `[ … ]` and `[[ … ]]` forms, `(( … ))` for
arithmetic, and `if`/`then`/`else` to act on the exit status they give.
tessh's `if` takes any Python value (true or false by Python's rules) or a
command (true when its status is 0), so most of bash's test machinery
becomes ordinary expressions.

## 7.1 Test Constructs
<!-- help: abs-testconstructs if then else elif test [ [[ (( truth -->

An `if` in bash runs a list of commands and looks at the exit status: 0
is true. `[` is a command, the same as `test`, and `[[ … ]]` a keyword
with fewer surprises. `(( … ))` and `let` are true when the arithmetic
comes out non-zero, the opposite of how a status reads.

In tessh, `if` with a command (a program, a pipe) looks at its status, as
bash's does; with anything else it looks at the value. So the tests are
Python's: `==`, `<`, `in`, `and`, `or`, `not`, methods such as
`.startswith`, and the file tests of `pathlib` and `os.path`.
**Warning:** `[ … ]` at the start of a tessh line is a list, not bash's
`[` command, so `if [ -n "$x" ]` is always true (a list with something in
it). Use `test`, the same program, or better a Python test. (`tessh --check`
doesn't warn about it yet: a bug.)

```bash
(( 0 && 1 ))                 # Logical AND
echo $?     # 1     ***
let "num = (( 0 && 1 ))"
echo $num   # 0

(( 200 || 11 ))              # Logical OR
echo $?     # 0     ***
let "num = (( 200 || 11 ))"
echo $num   # 1

(( 200 | 11 ))               # Bitwise OR
echo $?                      # 0     ***
let "num = (( 200 | 11 ))"
echo $num                    # 203

var=-2 && (( var+=2 ))
echo $?                   # 1
var=-2 && (( var+=2 )) && echo $var
                          # Will not echo $var!
```

```output bash
1
0
0
1
0
203
1
```

Python's `and` and `or` give one of their operands, not 0 or 1; `bool`
makes it one. Arithmetic is never a status: `var += 2` is an assignment,
and the test, when you want one, is written as one.

```tessh
num = 0 and 1                # Logical AND
print $num (bool $num)       # 0 False

num = 200 or 11              # Logical OR
print $num (bool $num)       # 200 True

num = 200 | 11               # Bitwise OR
print $num                   # 203

var = -2
var += 2
if $var
  print $var
else
  print "var is 0, which is false"
```

```output
0 False
200 True
203
var is 0, which is false
```

An `if` can test any command, not just a test. "if grep" is the most
useful of them: `grep -q` prints nothing and only answers.

```bash
echo "Bash is here" > file
printf "same\n" > a ; printf "same\n" > b
if cmp a b &> /dev/null  # Suppress output.
then echo "Files a and b are identical."
else echo "Files a and b differ."
fi

# The very useful "if-grep" construct:
# ----------------------------------- 
if grep -q Bash file
  then echo "File contains at least one occurrence of Bash."
fi

word=Linux
letter_sequence=inu
if echo "$word" | grep -q "$letter_sequence"
# The "-q" option to grep suppresses output.
then
  echo "$letter_sequence found in $word"
else
  echo "$letter_sequence not found in $word"
fi
```

```tessh
echo "Bash is here" > file
printf "same\n" > a ; printf "same\n" > b
if cmp a b &> /dev/null  # Suppress output.
  print "Files a and b are identical."
else
  print "Files a and b differ."

# The very useful "if-grep" construct:
if grep -q Bash file
  print "File contains at least one occurrence of Bash."

word = Linux
letter_sequence = inu
if $letter_sequence in $word     # No grep needed for a string.
  print "$letter_sequence found in $word"
else
  print "$letter_sequence not found in $word"
```

```output
Files a and b are identical.
File contains at least one occurrence of Bash.
inu found in Linux
```

### Example 7-1. What is truth? (`ex10.sh`)

```bash
#!/bin/bash

#  Tip:
#  If you're unsure how a certain condition might evaluate,
#+ test it in an if-test.

echo

echo "Testing \"0\""
if [ 0 ]      # zero
then
  echo "0 is true."
else          # Or else ...
  echo "0 is false."
fi            # 0 is true.

echo

echo "Testing \"1\""
if [ 1 ]      # one
then
  echo "1 is true."
else
  echo "1 is false."
fi            # 1 is true.

echo

echo "Testing \"-1\""
if [ -1 ]     # minus one
then
  echo "-1 is true."
else
  echo "-1 is false."
fi            # -1 is true.

echo

echo "Testing \"NULL\""
if [ ]        # NULL (empty condition)
then
  echo "NULL is true."
else
  echo "NULL is false."
fi            # NULL is false.

echo

echo "Testing \"xyz\""
if [ xyz ]    # string
then
  echo "Random string is true."
else
  echo "Random string is false."
fi            # Random string is true.

echo

echo "Testing \"\$xyz\""
if [ $xyz ]   # Tests if $xyz is null, but...
              # it's only an uninitialized variable.
then
  echo "Uninitialized variable is true."
else
  echo "Uninitialized variable is false."
fi            # Uninitialized variable is false.

echo

echo "Testing \"-n \$xyz\""
if [ -n "$xyz" ]            # More pedantically correct.
then
  echo "Uninitialized variable is true."
else
  echo "Uninitialized variable is false."
fi            # Uninitialized variable is false.

echo


xyz=          # Initialized, but set to null value.

echo "Testing \"-n \$xyz\""
if [ -n "$xyz" ]
then
  echo "Null variable is true."
else
  echo "Null variable is false."
fi            # Null variable is false.


echo


# When is "false" true?

echo "Testing \"false\""
if [ "false" ]              #  It seems that "false" is just a string ...
then
  echo "\"false\" is true." #+ and it tests true.
else
  echo "\"false\" is false."
fi            # "false" is true.

echo

echo "Testing \"\$false\""  # Again, uninitialized variable.
if [ "$false" ]
then
  echo "\"\$false\" is true."
else
  echo "\"\$false\" is false."
fi            # "$false" is false.
              # Now, we get the expected result.

#  What would happen if we tested the uninitialized variable "$true"?

echo

exit 0
```

```output bash

Testing "0"
0 is true.

Testing "1"
1 is true.

Testing "-1"
-1 is true.

Testing "NULL"
NULL is false.

Testing "xyz"
Random string is true.

Testing "$xyz"
Uninitialized variable is false.

Testing "-n $xyz"
Uninitialized variable is false.

Testing "-n $xyz"
Null variable is false.

Testing "false"
"false" is true.

Testing "$false"
"$false" is false.

```

tessh's truth is Python's: `0`, `0.0`, `""`, `None`, `False` and empty
collections are false, and everything else true. So 0 is false here,
unlike bash's `[ 0 ]` (a non-empty string); the string `"0"` and the
string `"false"` are true. An uninitialized variable is an error, so the
script uses `None`, "no value", for it. The tests are in a loop over the
values, since each is the same `if`.

```tessh
#  Tip:
#  If you're unsure how a certain condition might evaluate,
#+ test it in an if-test.

print

xyz = None       # "Uninitialized": no value.
for [label value] in [["0" 0] ["1" 1] ["-1" -1] ["NULL" ""] ["xyz" "xyz"]
                      ["\$xyz" $xyz] ["string 0" "0"] ["false" "false"]
                      ["False" False] ["[]" []]]
  print "Testing \"$label\""
  if $value
    print "$label is true."
  else
    print "$label is false."
  print

exit 0
```

```output

Testing "0"
0 is false.

Testing "1"
1 is true.

Testing "-1"
-1 is true.

Testing "NULL"
NULL is false.

Testing "xyz"
xyz is true.

Testing "$xyz"
$xyz is false.

Testing "string 0"
string 0 is true.

Testing "false"
false is true.

Testing "False"
False is false.

Testing "[]"
[] is false.

```

`test`, `/usr/bin/test`, `[ ]` and `/usr/bin/[` are the same test in bash
(the first and third built in, the others programs).

### Example 7-2. Equivalence of test, `/usr/bin/test`, `[ ]`, and `/usr/bin/[` (`ex11.sh`)

```bash no-run
#!/bin/bash

echo

if test -z "$1"
then
  echo "No command-line arguments."
else
  echo "First command-line argument is $1."
fi

echo

if /usr/bin/test -z "$1"      # Equivalent to "test" builtin.
#  ^^^^^^^^^^^^^              # Specifying full pathname.
then
  echo "No command-line arguments."
else
  echo "First command-line argument is $1."
fi

echo

if [ -z "$1" ]                # Functionally identical to above code blocks.
#   if [ -z "$1"                should work, but...
#+  Bash responds to a missing close-bracket with an error message.
then
  echo "No command-line arguments."
else
  echo "First command-line argument is $1."
fi

echo


if /usr/bin/[ -z "$1" ]       # Again, functionally identical to above.
# if /usr/bin/[ -z "$1"       # Works, but gives an error message.
#                             # Note:
#                               This has been fixed in Bash, version 3.x.
then
  echo "No command-line arguments."
else
  echo "First command-line argument is $1."
fi

echo

exit 0
```

(`no-run`: not every system has `/usr/bin/test`; NixOS, where this was
written, doesn't.) In tessh `test` is the program, found on `PATH`; a
program at a path held in a variable runs as `$name args`. `[` can't be used
at the start of a line (it's a list), and the plain Python test is the
tessh way.

```tessh
import shutil
arg1 = ($argv[1:2] or [""])[0]   # The first argument, or "".

print

if test -z $arg1
  print "No command-line arguments."
else
  print "First command-line argument is $arg1."

print

test_program = shutil.which "test"     # /usr/bin/test, or wherever it is.
if $test_program -z $arg1
  print "No command-line arguments."
else
  print "First command-line argument is $arg1."

print

if not $arg1                     # Python's test: the empty string is false.
  print "No command-line arguments."
else
  print "First command-line argument is $arg1."

print

exit 0
```

```output

No command-line arguments.

No command-line arguments.

No command-line arguments.

```

`[[ … ]]` does no globbing or word splitting inside, allows `&&`, `||`,
`<` and `>`, and reads octal and hex numbers in arithmetic comparisons.
tessh's tests do all that as a matter of course. Numbers are numbers:
`0o17` (Python's octal) and `0x0f` are 15, and text is turned into a
number with `int`, in any base.

```bash
decimal=15
octal=017   # = 15 (decimal)
hex=0x0f    # = 15 (decimal)

if [ "$decimal" -eq "$octal" ]
then
  echo "$decimal equals $octal"
else
  echo "$decimal is not equal to $octal"       # 15 is not equal to 017
fi      # Doesn't evaluate within [ single brackets ]!

if [[ "$decimal" -eq "$octal" ]]
then
  echo "$decimal equals $octal"                # 15 equals 017
else
  echo "$decimal is not equal to $octal"
fi      # Evaluates within [[ double brackets ]]!

if [[ "$decimal" -eq "$hex" ]]
then
  echo "$decimal equals $hex"                  # 15 equals 0x0f
else
  echo "$decimal is not equal to $hex"
fi      # [[ $hexadecimal ]] also evaluates!
```

```output bash
15 is not equal to 017
15 equals 017
15 equals 0x0f
```

```tessh
decimal = 15
octal = "017"     # Text, as read from a file or an argument.
hex = "0x0f"

if $decimal == $octal
  print "$decimal equals $octal"
else
  print "$decimal is not equal to $octal"      # A number is not a string.

if $decimal == (int $octal 8)
  print "$decimal equals $octal"               # 15 equals 017

if $decimal == (int $hex 16)
  print "$decimal equals $hex"                 # 15 equals 0x0f

print (0o17 == 0x0f == 15)                     # Written as numbers.
```

```output
15 is not equal to 017
15 equals 017
15 equals 0x0f
True
```

Any command can be the test, and a test can stand without an `if` in an
and-or list. In tessh a failed `cd` is an error rather than a status, so
`try` catches it; the and-or list works with Python tests as well as
commands.

```bash
dir=/home/bozo

if cd "$dir" 2>/dev/null; then   # "2>/dev/null" hides error message.
  echo "Now in $dir."
else
  echo "Can't change to $dir."
fi

var1=20
var2=22
[ "$var1" -ne "$var2" ] && echo "$var1 is not equal to $var2"

home=/home/bozo
[ -d "$home" ] || echo "$home directory does not exist."
```

```tessh
import os
dir = /home/bozo

try
  cd $dir
  print "Now in $dir."
except OSError
  print "Can't change to $dir."

var1 = 20
var2 = 22
$var1 != $var2 && print "$var1 is not equal to $var2"

home = /home/bozo
(os.path.isdir $home) || print "$home directory does not exist."
```

```output
Can't change to /home/bozo.
20 is not equal to 22
/home/bozo directory does not exist.
```

`(( … ))` is true when the arithmetic in it isn't 0.

### Example 7-3. Arithmetic Tests using `(( ))` (`arith-tests.sh`)

```bash
#!/bin/bash
# arith-tests.sh
# Arithmetic tests.

# The (( ... )) construct evaluates and tests numerical expressions.
# Exit status opposite from [ ... ] construct!

(( 0 ))
echo "Exit status of \"(( 0 ))\" is $?."         # 1

(( 1 ))
echo "Exit status of \"(( 1 ))\" is $?."         # 0

(( 5 > 4 ))                                      # true
echo "Exit status of \"(( 5 > 4 ))\" is $?."     # 0

(( 5 > 9 ))                                      # false
echo "Exit status of \"(( 5 > 9 ))\" is $?."     # 1

(( 5 == 5 ))                                     # true
echo "Exit status of \"(( 5 == 5 ))\" is $?."    # 0
# (( 5 = 5 ))  gives an error message.

(( 5 - 5 ))                                      # 0
echo "Exit status of \"(( 5 - 5 ))\" is $?."     # 1

(( 5 / 4 ))                                      # Division o.k.
echo "Exit status of \"(( 5 / 4 ))\" is $?."     # 0

(( 1 / 2 ))                                      # Division result < 1.
echo "Exit status of \"(( 1 / 2 ))\" is $?."     # Rounded off to 0.
                                                 # 1

(( 1 / 0 )) 2>/dev/null                          # Illegal division by 0.
#           ^^^^^^^^^^^
echo "Exit status of \"(( 1 / 0 ))\" is $?."     # 1

# What effect does the "2>/dev/null" have?
# What would happen if it were removed?
# Try removing it, then rerunning the script.

# ======================================= #

# (( ... )) also useful in an if-then test.

var1=5
var2=4

if (( var1 > var2 ))
then #^      ^      Note: Not $var1, $var2. Why?
  echo "$var1 is greater than $var2"
fi     # 5 is greater than 4

exit 0
```

```output bash
Exit status of "(( 0 ))" is 1.
Exit status of "(( 1 ))" is 0.
Exit status of "(( 5 > 4 ))" is 0.
Exit status of "(( 5 > 9 ))" is 1.
Exit status of "(( 5 == 5 ))" is 0.
Exit status of "(( 5 - 5 ))" is 1.
Exit status of "(( 5 / 4 ))" is 0.
Exit status of "(( 1 / 2 ))" is 1.
Exit status of "(( 1 / 0 ))" is 1.
5 is greater than 4
```

tessh has no `(( ))`: any arithmetic is an expression, and `bool` says
whether it's true. Two differences show: `/` is true division (`1 / 2` is
0.5, which is true; `//` divides whole numbers, as bash does), and
dividing by zero is a `ZeroDivisionError`, caught here with `try`.
(`(5 == 5)`, a comparison, is already `True` or `False`.)

```tessh
# Arithmetic tests: an expression's truth.

def show label value
  print "\"$label\" is ${bool $value}."

show "0" 0
show "1" 1
show "5 > 4" (5 > 4)
show "5 > 9" (5 > 9)
show "5 == 5" (5 == 5)
show "5 - 5" (5 - 5)
show "5 / 4" (5 / 4)
show "1 / 2" (1 / 2)            # True division: 0.5.
show "1 // 2" (1 // 2)          # Whole-number division: 0.
try
  show "1 / 0" (1 / 0)
except ZeroDivisionError as e
  print "\"1 / 0\" is an error: $e."

# ======================================= #

var1 = 5
var2 = 4

if $var1 > $var2
  print "$var1 is greater than $var2"

exit 0
```

```output
"0" is False.
"1" is True.
"5 > 4" is True.
"5 > 9" is False.
"5 == 5" is True.
"5 - 5" is False.
"5 / 4" is True.
"1 / 2" is True.
"1 // 2" is False.
"1 / 0" is an error: division by zero.
5 is greater than 4
```

## 7.2 File test operators
<!-- help: abs-fto file-tests -e -f -d -s -r -w -x -h -L -nt -ot -ef pathlib os.path -->

Bash's file tests and their tessh counterparts. `p = Path name` makes a
`pathlib` path (a bareword like `./x` or `/etc/x` already is one); the
methods are called with parentheses around them, `($p.is_file)`, since a
method alone on a line or in parentheses is called.

| bash | true if… | tessh |
|---|---|---|
| `-e f`, `-a f` | it exists | `($p.exists)`, `os.path.exists f` |
| `-f f` | it's a regular file | `($p.is_file)` |
| `-s f` | it's not empty | `(os.path.getsize f) > 0` |
| `-d f` | it's a directory | `($p.is_dir)` |
| `-b f` | it's a block device | `($p.is_block_device)` |
| `-c f` | it's a character device | `($p.is_char_device)` |
| `-p f` | it's a pipe | `($p.is_fifo)` |
| `-h f`, `-L f` | it's a symbolic link | `($p.is_symlink)` |
| `-S f` | it's a socket | `($p.is_socket)` |
| `-t fd` | the descriptor is a terminal | `os.isatty fd` |
| `-r f`, `-w f`, `-x f` | you may read, write, run it | `os.access f $os.R_OK` (`W_OK`, `X_OK`) |
| `-g f`, `-u f`, `-k f` | set-group-id, set-user-id, sticky | `(os.stat f).st_mode & $stat.S_ISGID` (`S_ISUID`, `S_ISVTX`) |
| `-O f`, `-G f` | you own it, it's your group's | `(os.stat f).st_uid == (os.getuid)` (`st_gid`, `getgid`) |
| `-N f` | changed since last read | `st.st_mtime > st.st_atime` |
| `f1 -nt f2`, `f1 -ot f2` | newer, older | compare `(os.path.getmtime f1)` |
| `f1 -ef f2` | the same file | `os.path.samefile f1 f2` |
| `! test` | not | `not` |

```bash
touch older ; sleep 0.01 ; touch newer ; ln -s older link ; mkdir folder
for f in older link folder nowhere; do
  [ -e "$f" ] && echo "$f exists"
  [ -f "$f" ] && echo "$f is a regular file"
  [ -d "$f" ] && echo "$f is a directory"
  [ -h "$f" ] && echo "$f is a symbolic link"
done
[ newer -nt older ] && echo "newer is newer than older"
[ link -ef older ] && echo "link and older are the same file"
```

```tessh
import os
touch older ; sleep 0.01 ; touch newer ; ln -s older link ; mkdir folder
for f in [older link folder nowhere]
  p = Path $f
  ($p.exists) && print "$f exists"
  ($p.is_file) && print "$f is a regular file"
  ($p.is_dir) && print "$f is a directory"
  ($p.is_symlink) && print "$f is a symbolic link"
(os.path.getmtime newer) > (os.path.getmtime older) && print "newer is newer than older"
(os.path.samefile link older) && print "link and older are the same file"
```

```output
older exists
older is a regular file
link exists
link is a regular file
link is a symbolic link
folder exists
folder is a directory
newer is newer than older
link and older are the same file
```

The ABS Guide's `show_input_type` tells a pipe from a terminal with `[ -p
/dev/fd/0 ]`. A tessh script asks the same of its own standard input:

```bash
show_input_type()
{
   [ -p /dev/fd/0 ] && echo PIPE || echo STDIN
}

show_input_type "Input"                           # STDIN
echo "Input" | show_input_type                    # PIPE
```

```tessh
print 'import os stat ; print (if (stat.S_ISFIFO (os.fstat 0).st_mode) then "PIPE" else "STDIN")' >show_input_type.tsh

tessh show_input_type.tsh                         # STDIN
echo "Input" | tessh show_input_type.tsh          # PIPE
```

```output
STDIN
PIPE
```

### Example 7-4. Testing for broken links (`broken-link.sh`)

```bash no-run
#!/bin/bash
# broken-link.sh
# Written by Lee bigelow <ligelowbee@yahoo.com>
# Used in ABS Guide with permission.

#  A pure shell script to find dead symlinks and output them quoted
#+ so they can be fed to xargs and dealt with :)
#+ eg. sh broken-link.sh /somedir /someotherdir|xargs rm
#
#  This, however, is a better method:
#
#  find "somedir" -type l -print0|\
#  xargs -r0 file|\
#  grep "broken symbolic"|
#  sed -e 's/^\|: *broken symbolic.*$/"/g'
#
#+ but that wouldn't be pure Bash, now would it.
#  Caution: beware the /proc file system and any circular links!
################################################################


#  If no args are passed to the script set directories-to-search 
#+ to current directory.  Otherwise set the directories-to-search 
#+ to the args passed.
######################

[ $# -eq 0 ] && directorys=`pwd` || directorys=$@


#  Setup the function linkchk to check the directory it is passed 
#+ for files that are links and don't exist, then print them quoted.
#  If one of the elements in the directory is a subdirectory then 
#+ send that subdirectory to the linkcheck function.
##########

linkchk () {
    for element in $1/*; do
      [ -h "$element" -a ! -e "$element" ] && echo \"$element\"
      [ -d "$element" ] && linkchk $element
    # Of course, '-h' tests for symbolic link, '-d' for directory.
    done
}

#  Send each arg that was passed to the script to the linkchk() function
#+ if it is a valid directoy.  If not, then print the error message
#+ and usage info.
##################
for directory in $directorys; do
    if [ -d $directory ]
	then linkchk $directory
	else 
	    echo "$directory is not a directory"
	    echo "Usage: $0 dir1 dir2 ..."
    fi
done

exit $?
```

(`no-run`: it looks for broken links in the folder it runs in.) The tests
are `Path` methods, and the folders a list, so names with spaces are safe.
`$directory/*` is a Glob of `Path`s, as `$1/*` is in bash (hidden files
left out). Here a made-up tree
of links, and sample arguments, stand in.

```tessh
# (A made-up tree with two broken links, and sample arguments.)
mkdir -p "links/sub dir" ; touch links/file
ln -s file links/good ; ln -s nowhere links/dead ; ln -s ../gone "links/sub dir/dead too"

#  The folders to search: the arguments, or else the current folder.
directorys = $argv[1:] or [links not-a-dir]   # (sample arguments)

#  Check a folder for links that lead nowhere, and print them quoted.
#  Go into the folders in it the same way.
def linkchk directory
  for element in $directory/*
    if ($element.is_symlink) and not ($element.exists)
      print "\"$element\""
    if ($element.is_dir)
      linkchk $element

for directory in $directorys
  if ((Path $directory).is_dir)
    linkchk $directory
  else
    print "$directory is not a directory"
    print "Usage: $(Path $argv[0]).name dir1 dir2 ..."
```

```output
"links/dead"
"links/sub dir/dead too"
not-a-dir is not a directory
Usage: ex.tsh dir1 dir2 ...
```

## 7.3 Other Comparison Operators
<!-- help: abs-comparison-ops comparison -eq -ne -lt -gt -z -n == != < > -a -o -->

Bash compares numbers with `-eq`, `-ne`, `-gt`, `-ge`, `-lt`, `-le` (or
`<` and the rest inside `(( ))`) and strings with `=`, `==`, `!=`, `<`,
`>` (escaped in `[ ]`), `-z` (empty) and `-n` (not empty). tessh has one
set, Python's: `==`, `!=`, `<`, `<=`, `>`, `>=`, for numbers and strings
alike, and the value's own truth for "empty". Comparisons chain (`0 < $a
< 5`). A number and a string are never equal, so read numbers with `int`.

| bash | tessh |
|---|---|
| `[ "$a" -eq "$b" ]`, `(( a == b ))` | `$a == $b` |
| `-ne`, `-gt`, `-ge`, `-lt`, `-le` | `!=`, `>`, `>=`, `<`, `<=` |
| `[ "$a" = "$b" ]`, `[ "$a" == "$b" ]` | `$a == $b` |
| `[[ $a == z* ]]` (a pattern) | `$a =~ z*` (a Glob), or `$a.startswith "z"` |
| `[[ "$a" < "$b" ]]`, `[ "$a" \< "$b" ]` | `$a < $b` |
| `[ -z "$s" ]`, `[ -n "$s" ]` | `not $s`, `$s` (or `$s == ""`) |
| `-a`, `-o`, `&&`, `\|\|` in `[[ ]]` | `and`, `or` (or `&&`, `\|\|`) |

```bash
a=zebra
[[ $a == z* ]] && echo "pattern: starts with z"
[[ $a == "z*" ]] || echo "literal: isn't z*"
[ "$a" == "z*" ] || echo "literal: isn't z*"
```

```output bash
pattern: starts with z
literal: isn't z*
literal: isn't z*
```

```tessh
a = zebra
$a =~ z* && print "pattern: starts with z"
$a == "z*" || print "literal: isn't z*"
$a =~ "z*" || print "literal: doesn't contain z*"
```

```output
pattern: starts with z
literal: isn't z*
literal: doesn't contain z*
```

### Example 7-5. Arithmetic and string comparisons (`ex13.sh`)

```bash
#!/bin/bash

a=4
b=5

#  Here "a" and "b" can be treated either as integers or strings.
#  There is some blurring between the arithmetic and string comparisons,
#+ since Bash variables are not strongly typed.

#  Bash permits integer operations and comparisons on variables
#+ whose value consists of all-integer characters.
#  Caution advised, however.

echo

if [ "$a" -ne "$b" ]
then
  echo "$a is not equal to $b"
  echo "(arithmetic comparison)"
fi

echo

if [ "$a" != "$b" ]
then
  echo "$a is not equal to $b."
  echo "(string comparison)"
  #     "4"  != "5"
  # ASCII 52 != ASCII 53
fi

# In this particular instance, both "-ne" and "!=" work.

echo

exit 0
```

```output bash

4 is not equal to 5
(arithmetic comparison)

4 is not equal to 5.
(string comparison)

```

In tessh `a` and `b` are numbers, and `!=` compares them as numbers; their
strings compare as strings. There's no blurring: `4 == "4"` is false.

```tessh
a = 4
b = 5

print

if $a != $b
  print "$a is not equal to $b"
  print "(arithmetic comparison)"

print

if (str $a) != (str $b)
  print "$a is not equal to $b."
  print "(string comparison)"
  #     "4"  != "5"

print "Is 4 == \"4\"?" (4 == "4")

print

exit 0
```

```output

4 is not equal to 5
(arithmetic comparison)

4 is not equal to 5.
(string comparison)
Is 4 == "4"? False

```

A tested string must be quoted in bash's `[ ]`: an unquoted empty one
vanishes, and one with spaces becomes several words, and either can give
the wrong answer. A tessh variable is never split or dropped, so even the
`test` program gets the right one argument.

### Example 7-6. Testing whether a string is null (`str-test.sh`)

```bash
#!/bin/bash
#  str-test.sh: Testing null strings and unquoted strings,
#+ but not strings and sealing wax, not to mention cabbages and kings . . .

# Using   if [ ... ]

# If a string has not been initialized, it has no defined value.
# This state is called "null" (not the same as zero!).

if [ -n $string1 ]    # string1 has not been declared or initialized.
then
  echo "String \"string1\" is not null."
else  
  echo "String \"string1\" is null."
fi                    # Wrong result.
# Shows $string1 as not null, although it was not initialized.

echo

# Let's try it again.

if [ -n "$string1" ]  # This time, $string1 is quoted.
then
  echo "String \"string1\" is not null."
else  
  echo "String \"string1\" is null."
fi                    # Quote strings within test brackets!

echo

if [ $string1 ]       # This time, $string1 stands naked.
then
  echo "String \"string1\" is not null."
else  
  echo "String \"string1\" is null."
fi                    # This works fine.
# The [ ... ] test operator alone detects whether the string is null.
# However it is good practice to quote it (if [ "$string1" ]).
#
# As Stephane Chazelas points out,
#    if [ $string1 ]    has one argument, "]"
#    if [ "$string1" ]  has two arguments, the empty "$string1" and "]" 


echo


string1=initialized

if [ $string1 ]       # Again, $string1 stands unquoted.
then
  echo "String \"string1\" is not null."
else  
  echo "String \"string1\" is null."
fi                    # Again, gives correct result.
# Still, it is better to quote it ("$string1"), because . . .


string1="a = b"

if [ $string1 ]       # Again, $string1 stands unquoted.
then
  echo "String \"string1\" is not null."
else  
  echo "String \"string1\" is null."
fi                    # Not quoting "$string1" now gives wrong result!

exit 0   # Thank you, also, Florian Wisser, for the "heads-up".
```

```output bash
String "string1" is not null.

String "string1" is null.

String "string1" is null.

String "string1" is not null.
String "string1" is null.
```

The tessh version tests each value twice: with the `test` program, and
with Python's truth. Both give the right answer every time. (Unset, the
variable would be an error; empty, `""`, stands in for it.)

```tessh
def check string1
  if test -n $string1         # One argument, never split or dropped.
    print "test: String \"string1\" is not null."
  else
    print "test: String \"string1\" is null."
  if $string1                 # Python: the empty string is false.
    print "Python: String \"string1\" is not null."
  else
    print "Python: String \"string1\" is null."

check ""
print
check initialized
print
check "a = b"

exit 0
```

```output
test: String "string1" is null.
Python: String "string1" is null.

test: String "string1" is not null.
Python: String "string1" is not null.

test: String "string1" is not null.
Python: String "string1" is not null.
```

### Example 7-7. zmore (`ex14.sh`)

```bash no-run
#!/bin/bash
# zmore

# View gzipped files with 'more' filter.

E_NOARGS=85
E_NOTFOUND=86
E_NOTGZIP=87

if [ $# -eq 0 ] # same effect as:  if [ -z "$1" ]
# $1 can exist, but be empty:  zmore "" arg2 arg3
then
  echo "Usage: `basename $0` filename" >&2
  # Error message to stderr.
  exit $E_NOARGS
  # Returns 85 as exit status of script (error code).
fi  

filename=$1

if [ ! -f "$filename" ]   # Quoting $filename allows for possible spaces.
then
  echo "File $filename not found!" >&2   # Error message to stderr.
  exit $E_NOTFOUND
fi  

if [ ${filename##*.} != "gz" ]
# Using bracket in variable substitution.
then
  echo "File $1 is not a gzipped file!"
  exit $E_NOTGZIP
fi  

zcat $1 | more

# Uses the 'more' filter.
# May substitute 'less' if desired.

exit $?   # Script returns exit status of pipe.
#  Actually "exit $?" is unnecessary, as the script will, in any case,
#+ return the exit status of the last command executed.
```

`${filename##*.}`, the suffix, is a Path's `.suffix`. `print … file=$sys.stderr`
writes to standard error (`>&2` works too). A sample gzipped file stands
in for the argument; `more` just copies when its output isn't a terminal.

```tessh
import sys
# View gzipped files with 'more' filter.

E_NOARGS = 85
E_NOTFOUND = 86
E_NOTGZIP = 87

# (A made-up gzipped file, and a sample argument.)
printf "line one\nline two\n" | gzip >notes.txt.gz
args = $argv[1:] or [notes.txt.gz]

if not $args
  print "Usage: $(Path $argv[0]).name filename" file=$sys.stderr
  exit $E_NOARGS

filename = Path $args[0]

if not ($filename.is_file)
  print "File $filename not found!" file=$sys.stderr
  exit $E_NOTFOUND

if $filename.suffix != ".gz"
  print "File $filename is not a gzipped file!"
  exit $E_NOTGZIP

zcat $filename | more

exit $?   # The status of the pipe.
```

```output
line one
line two
```

`-a` and `-o` are "and" and "or" inside `[ ]`; `&&` and `||` join tests
outside, or inside `[[ ]]`. The difference rihad points out: `-a` and `-o`
evaluate both sides, `&&` and `||` stop as soon as the answer is known.
Python's `and` and `or` stop early too.

```bash
[ 1 -eq 1 ] && [ -n "`echo true 1>&2`" ]   # true
[ 1 -eq 2 ] && [ -n "`echo true 1>&2`" ]   # (no output)
# ^^^^^^^ False condition. So far, everything as expected.

# However ...
[ 1 -eq 2 -a -n "`echo true 1>&2`" ]       # true
# ^^^^^^^ False condition. So, why "true" output?

# Is it because both condition clauses within brackets evaluate?
[[ 1 -eq 2 && -n "`echo true 1>&2`" ]]     # (no output)
# No, that's not it.

# Apparently && and || "short-circuit" while -a and -o do not.
```

```output bash
```

(Bash's `true`s go to standard error; the tessh version prints them.)

```tessh
def noisy
  print true
  True
1 == 1 and (noisy)     # true
1 == 2 and (noisy)     # (no output): and stops at the first false.
```

```output
true
```

## 7.4 Nested if/then Condition Tests
<!-- help: abs-nestedifthen nested-if elif and -->

`if`s nest, and a nested pair is the same as one test joined with `and`.
In tessh a chained comparison is shorter still.

```bash
a=3

if [ "$a" -gt 0 ]
then
  if [ "$a" -lt 5 ]
  then
    echo "The value of \"a\" lies somewhere between 0 and 5."
  fi
fi

# Same result as:

if [ "$a" -gt 0 ] && [ "$a" -lt 5 ]
then
  echo "The value of \"a\" lies somewhere between 0 and 5."
fi
```

```tessh
a = 3

if $a > 0
  if $a < 5
    print "The value of \"a\" lies somewhere between 0 and 5."

# Same result as:

if 0 < $a < 5
  print "The value of \"a\" lies somewhere between 0 and 5."
```

```output
The value of "a" lies somewhere between 0 and 5.
The value of "a" lies somewhere between 0 and 5.
```

`elif` is "else if", in both; a tessh `if` also has an expression form,
`if … then … else …`.

## 7.5 Testing Your Knowledge of Tests
<!-- help: abs-testtest xinitrc exercise -->

The ABS Guide ends with a piece of an old `xinitrc` to explain: run
`~/.Xclients` if it's a file, or else the system's `Xclients`, or else
start a clock and a terminal, and a browser on the documentation if both
it and the page are there. In tessh:

```bash no-run
if [ -f $HOME/.Xclients ]; then
  exec $HOME/.Xclients
elif [ -f /etc/X11/xinit/Xclients ]; then
  exec /etc/X11/xinit/Xclients
else
     # failsafe settings.  Although we should never get here
     # (we provide fallbacks in Xclients as well) it can't hurt.
     xclock -geometry 100x100-5+5 &
     xterm -geometry 80x50-50+150 &
     if [ -f /usr/bin/netscape -a -f /usr/share/doc/HTML/index.html ]; then
             netscape /usr/share/doc/HTML/index.html &
     fi
fi
```

```tessh no-run
import os
if (os.path.isfile ~/.Xclients)
  os.execv ~/.Xclients [~/.Xclients]
elif (os.path.isfile /etc/X11/xinit/Xclients)
  os.execv /etc/X11/xinit/Xclients [/etc/X11/xinit/Xclients]
else
  # failsafe settings.
  xclock -geometry 100x100-5+5 &
  xterm -geometry 80x50-50+150 &
  if (os.path.isfile /usr/bin/netscape) and (os.path.isfile /usr/share/doc/HTML/index.html)
    netscape /usr/share/doc/HTML/index.html &
```

**Gap:** tessh has no `exec` builtin to replace the shell with a program;
Python's `os.execv` does it.
