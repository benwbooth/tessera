# 4. Introduction to Variables and Parameters
<!-- help: abs-variables abs-4 variables parameters -->

A variable is a name for a piece of data. In bash every variable holds a
string (or an array of them), which arithmetic reads as a number when it
looks like one. A tessh variable is a Python name, and holds any Python
value: a number, a string, a list, a Path, a file, a function. That's the
main change this chapter shows.

## 4.1 Variable Substitution
<!-- help: abs-varsubn variable-substitution $name ${name} lvalue rvalue -->

A variable's name and its value are different things: `variable1` is the
name and `$variable1` the value, in tessh as in bash. The name goes bare
where the variable is made (assigned, a `for` loop's variable, `del`), and
with `$` where it's read.

```bash
variable1=23
echo variable1
echo $variable1
```

```tessh
variable1 = 23
print variable1
print $variable1
```

```output
variable1
23
```

Double quotes fill in `$name` in both, single quotes don't. `${name}` is
the long form of `$name` in both too; in tessh the braces take any Python
expression (`${n + 1}`, `${x!r}`, `${pi:.2f}`).

### Example 4-1. Variable assignment and substitution (`ex9.sh`)

```bash
#!/bin/bash
# ex9.sh

# Variables: assignment and substitution

a=375
hello=$a
#   ^ ^

#-------------------------------------------------------------------------
# No space permitted on either side of = sign when initializing variables.
# What happens if there is a space?

#  "VARIABLE =value"
#           ^
#% Script tries to run "VARIABLE" command with one argument, "=value".

#  "VARIABLE= value"
#            ^
#% Script tries to run "value" command with
#+ the environmental variable "VARIABLE" set to "".
#-------------------------------------------------------------------------


echo hello    # hello
# Not a variable reference, just the string "hello" ...

echo $hello   # 375
#    ^          This *is* a variable reference.
echo ${hello} # 375
#               Likewise a variable reference, as above.

# Quoting . . .
echo "$hello"    # 375
echo "${hello}"  # 375

echo

hello="A B  C   D"
echo $hello   # A B C D
echo "$hello" # A B  C   D
# As we see, echo $hello   and   echo "$hello"   give different results.
# =======================================
# Quoting a variable preserves whitespace.
# =======================================

echo

echo '$hello'  # $hello
#    ^      ^
#  Variable referencing disabled (escaped) by single quotes,
#+ which causes the "$" to be interpreted literally.

# Notice the effect of different types of quoting.


hello=    # Setting it to a null value.
echo "\$hello (null value) = $hello"      # $hello (null value) =
#  Note that setting a variable to a null value is not the same as
#+ unsetting it, although the end result is the same (see below).

# --------------------------------------------------------------

#  It is permissible to set multiple variables on the same line,
#+ if separated by white space.
#  Caution, this may reduce legibility, and may not be portable.

var1=21  var2=22  var3=$V3
echo
echo "var1=$var1   var2=$var2   var3=$var3"

# May cause problems with legacy versions of "sh" . . .

# --------------------------------------------------------------

echo; echo

numbers="one two three"
#           ^   ^
other_numbers="1 2 3"
#               ^ ^
#  If there is whitespace embedded within a variable,
#+ then quotes are necessary.
#  other_numbers=1 2 3                  # Gives an error message.
echo "numbers = $numbers"
echo "other_numbers = $other_numbers"   # other_numbers = 1 2 3
#  Escaping the whitespace also works.
mixed_bag=2\ ---\ Whatever
#           ^    ^ Space after escape (\).

echo "$mixed_bag"         # 2 --- Whatever

echo; echo

echo "uninitialized_variable = $uninitialized_variable"
# Uninitialized variable has null value (no value at all!).
uninitialized_variable=   #  Declaring, but not initializing it --
                          #+ same as setting it to a null value, as above.
echo "uninitialized_variable = $uninitialized_variable"
                          # It still has a null value.

uninitialized_variable=23       # Set it.
unset uninitialized_variable    # Unset it.
echo "uninitialized_variable = $uninitialized_variable"
                                # uninitialized_variable =
                                # It still has a null value.
echo

exit 0
```

```output bash
hello
375
375
375
375

A B C D
A B  C   D

$hello
$hello (null value) = 

var1=21   var2=22   var3=


numbers = one two three
other_numbers = 1 2 3
2 --- Whatever


uninitialized_variable = 
uninitialized_variable = 
uninitialized_variable = 

```

The differences:

- Spaces around `=` are fine in tessh (`a = 375`); glued (`a=375`) works
  too.
- A tessh variable is never split into words: `echo $hello` passes one
  argument, spaces and all, as bash's `echo "$hello"` does. To split, say
  so: `$hello.split` is a list, and a list gives a command one argument per
  item.
- Reading a variable that was never set is an error (a `NameError`), not
  an empty string; it's as if bash's `set -u` were always on. `$env.get
  NAME ""` reads an environment variable that may not be there.
- `del name` removes a tessh variable (`unset` is for environment
  variables). Afterwards it's not there at all.

```tessh
#!/usr/bin/env tessh
# Variables: assignment and substitution

a = 375
hello = $a

print hello      # hello
# Not a variable reference, just the string "hello" ...

print $hello     # 375
print ${hello}   # 375

# Quoting . . .
print "$hello"   # 375
print "${hello}" # 375

print

hello = "A B  C   D"
echo $hello            # A B  C   D: one argument, never split.
echo ($hello.split)    # A B C D: split on purpose, a list of four.
# =======================================
# A variable keeps its whitespace.
# =======================================

print

print '$hello'   # $hello
#  Variable referencing disabled by single quotes.

hello=           # Setting it to a null (empty) value.
print "\$hello (null value) = $hello"     # $hello (null value) =

# --------------------------------------------------------------

#  Several variables on the same line, separated by white space.
#  An unset variable is an error, so $V3 is read with a default.

var1=21  var2=22  var3=($env.get V3 "")
print
print "var1=$var1   var2=$var2   var3=$var3"

# --------------------------------------------------------------

print; print

numbers = "one two three"
other_numbers = "1 2 3"
#  If there is whitespace embedded within a variable,
#+ then quotes are necessary.
print "numbers = $numbers"
print "other_numbers = $other_numbers"   # other_numbers = 1 2 3
#  Escaping the whitespace also works.
mixed_bag=2\ ---\ Whatever
print $mixed_bag         # 2 --- Whatever

print; print

uninitialized_variable=     #  The empty string, a value like any other.
print "uninitialized_variable = $uninitialized_variable"

uninitialized_variable=23       # Set it.
del uninitialized_variable      # Remove it.
try
  print "uninitialized_variable = $uninitialized_variable"
except NameError as e
  print "Unset, it's an error:" $e
print

exit 0
```

```output
hello
375
375
375
375

A B  C   D
A B C D

$hello
$hello (null value) = 

var1=21   var2=22   var3=


numbers = one two three
other_numbers = 1 2 3
2 --- Whatever


uninitialized_variable = 
Unset, it's an error: name 'uninitialized_variable' is not defined

```

An uninitialized bash variable is empty, and arithmetic reads it as 0. In
tessh it doesn't exist until it's assigned: give it a starting value.

```bash
if [ -z "$unassigned" ]
then
  echo "\$unassigned is NULL."
fi     # $unassigned is NULL.

echo "$uninitialized"                                # (blank line)
let "uninitialized += 5"                             # Add 5 to it.
echo "$uninitialized"                                # 5
```

```output bash
$unassigned is NULL.

5
```

```tessh
unassigned = None
if not $unassigned
  print "\$unassigned is NULL."

uninitialized = 0
uninitialized += 5
print $uninitialized
```

```output
$unassigned is NULL.
5
```

## 4.2 Variable Assignment
<!-- help: abs-varassignment assignment = let for read command-substitution -->

`=` assigns, and `==` compares (bash uses `=` for both, by context, and
`-eq` for numbers). A variable is also assigned by a `for` loop, and in
bash by `read` and `let`.

### Example 4-2. Plain Variable Assignment (`ex15.sh`)

```bash no-run
#!/bin/bash
# Naked variables

echo

# When is a variable "naked", i.e., lacking the '$' in front?
# When it is being assigned, rather than referenced.

# Assignment
a=879
echo "The value of \"a\" is $a."

# Assignment using 'let'
let a=16+5
echo "The value of \"a\" is now $a."

echo

# In a 'for' loop (really, a type of disguised assignment):
echo -n "Values of \"a\" in the loop are: "
for a in 7 8 9 11
do
  echo -n "$a "
done

echo
echo

# In a 'read' statement (also a type of assignment):
echo -n "Enter \"a\" "
read a
echo "The value of \"a\" is now $a."

echo

exit 0
```

tessh needs no `let`: arithmetic is everywhere. `read` is Python's
`input`; here a canned answer stands in for what you'd type.

```tessh
#!/usr/bin/env tessh
# Naked variables

print

# When is a variable "naked", i.e., lacking the '$' in front?
# When it is being assigned, rather than referenced.

# Assignment
a = 879
print "The value of \"a\" is $a."

# Arithmetic needs no 'let'
a = 16 + 5
print "The value of \"a\" is now $a."

print

# In a 'for' loop (really, a type of disguised assignment):
print "Values of \"a\" in the loop are: " end=""
for a in [7 8 9 11]
  print "$a " end=""

print
print

# Reading input (also a type of assignment):
print "Enter \"a\" " end=""
a = "42"                 # (a canned answer: a = (input) reads one)
print $a
print "The value of \"a\" is now $a."

print

exit 0
```

```output

The value of "a" is 879.
The value of "a" is now 21.

Values of "a" in the loop are: 7 8 9 11 

Enter "a" 42
The value of "a" is now 42.

```

### Example 4-3. Variable Assignment, plain and fancy (`ex16.sh`)

```bash no-run
#!/bin/bash

a=23              # Simple case
echo $a
b=$a
echo $b

# Now, getting a little bit fancier (command substitution).

a=`echo Hello!`   # Assigns result of 'echo' command to 'a' ...
echo $a
#  Note that including an exclamation mark (!) within a
#+ command substitution construct will not work from the command-line,
#+ since this triggers the Bash "history mechanism."
#  Inside a script, however, the history functions are disabled by default.

a=`ls -l`         # Assigns result of 'ls -l' command to 'a'
echo $a           # Unquoted, however, it removes tabs and newlines.
echo
echo "$a"         # The quoted variable preserves whitespace.
                  # (See the chapter on "Quoting.")

exit 0
```

A command used as a value is captured: `a = echo Hello!` needs neither
backquotes nor `$(…)`. (A program with no arguments goes in parentheses,
`d = (date)`: `d = date` is the word.) The value is text without its last
newline (with `.lines` and `.code` besides). Joining it into one line is a split away.
(`ls -go --time-style=+%F` here, on made-up files, leaves out the owner
and the time, so the output is the same every run.)

```tessh
#!/usr/bin/env tessh
# (Two made-up files to list.)
touch -d 2026-01-01 notes.txt todo.txt

a = 23              # Simple case
print $a
b = $a
print $b

# Now, getting a little bit fancier (command substitution).

a = echo Hello!     # Assigns the output of 'echo' to 'a' ...
print $a

a = ls -go --time-style=+%F   # Assigns the output of 'ls -l' to 'a'.
print *($a.split)    # Split into words, as bash's unquoted $a: no newlines.
print
print $a            # The variable keeps its whitespace.

exit 0
```

```output
23
23
Hello!
total 0 -rw-r--r-- 1 0 2026-01-01 notes.txt -rw-r--r-- 1 0 2026-01-01 todo.txt

total 0
-rw-r--r-- 1 0 2026-01-01 notes.txt
-rw-r--r-- 1 0 2026-01-01 todo.txt
```

The ABS Guide's `R=$(cat /etc/redhat-release)` and `arch=$(uname -m)` are
`R = cat /etc/redhat-release` and `arch = uname -m`.

## 4.3 Bash Variables Are Untyped
<!-- help: abs-untyped untyped types int str typed -->

Bash variables are strings, and arithmetic treats one as a number when it
is all digits (or as 0 when it isn't). tessh's variables are Python's,
which have types: `2334` is an `int`, `"BB35"` a `str`, and adding 1 to a
string is a `TypeError`, not 1. You convert on purpose with `int`, `str`
and `float`.

### Example 4-4. Integer or string? (`int-or-string.sh`)

```bash
#!/bin/bash
# int-or-string.sh

a=2334                   # Integer.
let "a += 1"
echo "a = $a "           # a = 2335
echo                     # Integer, still.


b=${a/23/BB}             # Substitute "BB" for "23".
                         # This transforms $b into a string.
echo "b = $b"            # b = BB35
declare -i b             # Declaring it an integer doesn't help.
echo "b = $b"            # b = BB35

let "b += 1"             # BB35 + 1
echo "b = $b"            # b = 1
echo                     # Bash sets the "integer value" of a string to 0.

c=BB34
echo "c = $c"            # c = BB34
d=${c/BB/23}             # Substitute "23" for "BB".
                         # This makes $d an integer.
echo "d = $d"            # d = 2334
let "d += 1"             # 2334 + 1
echo "d = $d"            # d = 2335
echo


# What about null variables?
e=''                     # ... Or e="" ... Or e=
echo "e = $e"            # e =
let "e += 1"             # Arithmetic operations allowed on a null variable?
echo "e = $e"            # e = 1
echo                     # Null variable transformed into an integer.

# What about undeclared variables?
echo "f = $f"            # f =
let "f += 1"             # Arithmetic operations allowed?
echo "f = $f"            # f = 1
echo                     # Undeclared variable transformed into an integer.
#
# However ...
let "f /= $undecl_var"   # Divide by zero?
#   let: f /= : syntax error: operand expected (error token is " ")
# Syntax error! Variable $undecl_var is not set to zero here!
#
# But still ...
let "f /= 0"
#   let: f /= 0: division by 0 (error token is "0")
# Expected behavior.


#  Bash (usually) sets the "integer value" of null to zero
#+ when performing an arithmetic operation.
#  But, don't try this at home, folks!
#  It's undocumented and probably non-portable behavior.


# Conclusion: Variables in Bash are untyped,
#+ with all attendant consequences.

exit $?
```

```output bash
a = 2335 

b = BB35
b = BB35
b = 1

c = BB34
d = 2334
d = 2335

e = 
e = 1

f = 
f = 1

```

In tessh each surprise of the original is an error you can catch. The
script shows them with `try`; a real script would just convert with
`int` where it means a number.

```tessh
#!/usr/bin/env tessh
a = 2334                 # Integer.
a += 1
print "a = $a "          # a = 2335
print                    # Integer, still.

b = (str $a).replace "23" "BB"   # Substitute "BB" for "23": a string.
print "b = $b"           # b = BB35
try
  b += 1                 # BB35 + 1
except TypeError as e
  print "b += 1:" $e
try
  b = (int $b) + 1
except ValueError as e
  print "int:" $e
print

c = BB34
print "c = $c"           # c = BB34
d = $c.replace "BB" "23" # Substitute "23" for "BB": still a string.
print "d = $d"           # d = 2334
d = (int $d) + 1         # Now a number, on purpose.
print "d = $d"           # d = 2335
print

# What about null variables?
e = ''
print "e = $e"           # e =
try
  e += 1
except TypeError as err
  print "e += 1:" $err
print

# What about undeclared variables?
try
  f += 1
except NameError as err
  print "f += 1:" $err
f = 1
try
  f /= 0
except ZeroDivisionError as err
  print "f /= 0:" $err

# Conclusion: tessh's variables have Python's types.
exit 0
```

```output
a = 2335 

b = BB35
b += 1: can only concatenate str (not "int") to str
int: invalid literal for int() with base 10: 'BB35'

c = BB34
d = 2334
d = 2335

e = 
e += 1: can only concatenate str (not "int") to str

f += 1: name 'f' is not defined
f /= 0: division by zero
```

Bash's `declare` (chapter 9) gives a variable a few attributes, an integer
among them. tessh's variables can carry a type hint (`n: int = 5`), which
Python keeps but doesn't enforce.

## 4.4 Special Variable Types
<!-- help: abs-othertypesv local environment positional $0 $1 $# argv shift export -->

**Local variables** are seen only in a function (or a block). In tessh a
name assigned in a function is local to it, as in Python, unless it's
declared `global`; a `let` block makes names for that block only. Bash's
variables are global unless declared `local`, the other way around.

**Environment variables** are what a process hands to the programs it
starts. tessh's `export NAME=value` (or `$env.NAME = value`) sets one for
the shell and its programs, and `$NAME`, when no tessh variable has that
name, reads one. A program can't change its parent's environment, in tessh
as in bash.

```bash
export GREETING=hello
sh -c 'echo $GREETING'
sh -c 'GREETING=changed'
echo $GREETING
```

```output bash
hello
hello
```

```tessh
export GREETING=hello
sh -c 'echo $GREETING'
sh -c 'GREETING=changed'
print $GREETING $env.GREETING
```

```output
hello
hello hello
```

**Positional parameters** are a script's arguments: bash's `$0`, `$1` …
`${10}`, with `$#` their number and `$*` and `$@` all of them. tessh has
one list, `$argv`: `$argv[0]` is the script, `$argv[1]` the first argument,
`$argv[1:]` all of them, `len $argv[1:]` how many, and `$argv[-1]` the last
(bash's `${!#}`). A function's arguments are its parameters, named.

### Example 4-5. Positional Parameters (`ex17.sh`)

```bash no-run
#!/bin/bash

# Call this script with at least 10 parameters, for example
# ./scriptname 1 2 3 4 5 6 7 8 9 10
MINPARAMS=10

echo

echo "The name of this script is \"$0\"."
# Adds ./ for current directory
echo "The name of this script is \"`basename $0`\"."
# Strips out path name info (see 'basename')

echo

if [ -n "$1" ]              # Tested variable is quoted.
then
 echo "Parameter #1 is $1"  # Need quotes to escape #
fi 

if [ -n "$2" ]
then
 echo "Parameter #2 is $2"
fi 

if [ -n "$3" ]
then
 echo "Parameter #3 is $3"
fi 

# ...


if [ -n "${10}" ]  # Parameters > $9 must be enclosed in {brackets}.
then
 echo "Parameter #10 is ${10}"
fi 

echo "-----------------------------------"
echo "All the command-line parameters are: "$*""

if [ $# -lt "$MINPARAMS" ]
then
  echo
  echo "This script needs at least $MINPARAMS command-line arguments!"
fi  

echo

exit 0
```

The tests are on the list's length, so a missing argument is never an
`IndexError`. The parameters past 9 need no braces. Here a sample command
line stands in when the script gets no arguments.

```tessh
#!/usr/bin/env tessh
MINPARAMS = 10

# (A sample command line, when there are no arguments.)
argv = if (len $argv) > 1 then $argv else ["./scriptname" *(range 1 11)]

print

print "The name of this script is \"$argv[0]\"."
print "The name of this script is \"$(Path $argv[0]).name\"."

print

for n in [1 2 3 10]
  if (len $argv) > $n and $argv[$n] != ""
    print "Parameter #$n is $argv[$n]"

print "-----------------------------------"
print "All the command-line parameters are:" *$argv[1:]

if (len $argv[1:]) < $MINPARAMS
  print
  print "This script needs at least $MINPARAMS command-line arguments!"

print

exit 0
```

```output

The name of this script is "./scriptname".
The name of this script is "scriptname".

Parameter #1 is 1
Parameter #2 is 2
Parameter #3 is 3
Parameter #10 is 10
-----------------------------------
All the command-line parameters are: 1 2 3 4 5 6 7 8 9 10

```

A script can act on the name it was called by, `$0` in bash. The
original's `variable1_=$1_` trick, so a missing argument isn't an empty
assignment, isn't needed: a slice is empty, not an error, and `or` gives
the default.

```tessh
argv = ["./script"]          # (no arguments)
variable1 = ($argv[1:2] or ["default"])[0]
print $variable1
args = len $argv[1:]         # Number of args passed ($#).
print $args
```

```output
default
0
```

### Example 4-6. wh, whois domain name lookup (`ex18.sh`)

```bash no-run
#!/bin/bash
# ex18.sh

# Does a 'whois domain-name' lookup on any of 3 alternate servers:
#                    ripe.net, cw.net, radb.net

# Place this script -- renamed 'wh' -- in /usr/local/bin

# Requires symbolic links:
# ln -s /usr/local/bin/wh /usr/local/bin/wh-ripe
# ln -s /usr/local/bin/wh /usr/local/bin/wh-apnic
# ln -s /usr/local/bin/wh /usr/local/bin/wh-tucows

E_NOARGS=75


if [ -z "$1" ]
then
  echo "Usage: `basename $0` [domain-name]"
  exit $E_NOARGS
fi

# Check script name and call proper server.
case `basename $0` in    # Or:    case ${0##*/} in
    "wh"       ) whois $1@whois.tucows.com;;
    "wh-ripe"  ) whois $1@whois.ripe.net;;
    "wh-apnic" ) whois $1@whois.apnic.net;;
    "wh-cw"    ) whois $1@whois.cw.net;;
    *          ) echo "Usage: `basename $0` [domain-name]";;
esac 

exit $?
```

`match` on the script's name picks the server. The lookup needs the
network, so here, without an argument, it stops at the usage message.

```tessh
#!/usr/bin/env tessh
E_NOARGS = 75
name = (Path $argv[0]).name

if (len $argv) < 2 or not $argv[1]
  print "Usage: $name [domain-name]"
  exit $E_NOARGS
domain = $argv[1]

# Check script name and call proper server.
r = match $name
  case wh
    whois "$domain@whois.tucows.com"
  case wh-ripe
    whois "$domain@whois.ripe.net"
  case wh-apnic
    whois "$domain@whois.apnic.net"
  case wh-cw
    whois "$domain@whois.cw.net"
  case _
    print "Usage: $name [domain-name]"
    None

exit (if $r is None then 0 else $r.code)
```

```output
Usage: ex.tsh [domain-name]
```

Bash's `shift` drops `$1` and moves the others down. In tessh the arguments
are a list, so `$args.pop 0` takes the first one off, and `$args[3:]`
skips three. A shift past the end is an empty list, not an endless loop.

### Example 4-7. Using shift (`ex19.sh`)

```bash
#!/bin/bash
# shft.sh: Using 'shift' to step through all the positional parameters.

#  Name this script something like shft.sh,
#+ and invoke it with some parameters.
#+ For example:
#             sh shft.sh a b c def 83 barndoor

until [ -z "$1" ]  # Until all parameters used up . . .
do
  echo -n "$1 "
  shift
done

echo               # Extra linefeed.

# But, what happens to the "used-up" parameters?
echo "$2"
#  Nothing echoes!
#  When $2 shifts into $1 (and there is no $3 to shift into $2)
#+ then $2 remains empty.
#  So, it is not a parameter *copy*, but a *move*.

exit

#  See also the echo-params.sh script for a "shiftless"
#+ alternative method of stepping through the positional params.
```

```output bash


```

```tessh
#!/usr/bin/env tessh
# Using pop to step through all the arguments.
args = $argv[1:] or [a b c def 83 barndoor]   # (sample arguments)

while $args              # Until all arguments used up . . .
  print ($args.pop 0) end=" "

print                    # Extra linefeed.

# But, what happens to the "used-up" arguments?
print $args              # They're gone: pop *moves* them out.

exit
```

```output
a b c def 83 barndoor 
[]
```

```bash
#!/bin/bash
# shift-past.sh
set -- 1 2 3 4 5
shift 3    # Shift 3 positions.
#  n=3; shift $n
#  Has the same effect.
echo "$1"
```

```tessh
#!/usr/bin/env tessh
args = [1 2 3 4 5]
args = $args[3:]   # Shift 3 positions.
print $args[0]
```

```output
4
```
