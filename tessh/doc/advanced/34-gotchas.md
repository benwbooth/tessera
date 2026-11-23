# 34. Gotchas
<!-- help: abs-gotchas abs-34 gotchas pitfalls mistakes traps-for-the-unwary -->

The ABS Guide's list of practices that bring excitement into an otherwise
dull life. Some of bash's are gone in tessh (it doesn't split or glob a
variable's value, a missing variable is an error, a CRLF script runs), some
remain (a child can't change its parent's environment, `grep` buffers into
a pipe), and tessh brings a few of its own, mostly Python's. Each in turn.

**Reserved words and characters in names.** A tessh variable is a Python
name: letters, digits and `_`, not starting with a digit. `23skidoo = 1`
and `var-1 = 23` are errors that `tessh --check` reports, rather than
commands bash goes looking for. Python's keywords (`if`, `for`, `class`,
`def`, `in`, `is`…) can't be names; `case` and `match` can, as in bash.
`_` is an ordinary name in a script.

**The same name for a variable and a function.** In bash they live apart,
so this is legal, if confusing:

```bash
do_something ()
{
  echo "This function does something with \"$1\"."
}

do_something=do_something

do_something do_something
```

In tessh there is one set of names: a function is a value like any other,
and assigning the name replaces it. Call the variable something else.

```tessh
def do_something arg
  print "This function does something with \"$arg\"."

what = do_something       # (do_something = do_something would replace
                          #  the function with the text "do_something")
do_something $what
```

```output
This function does something with "do_something".
```

**Whitespace.** Bash is fussy about it in the other direction: `var1 = 23`
runs a command `var1`, and `[ $a -le 5]` is missing a space. In tessh `x =
5` is the usual way to write it (`x=5` works too), and the fussiness is
about what binds to what: a call takes everything after it, so `print $x +
1` is `(print $x) + 1`, which prints `5` and then fails adding 1 to
`print`'s `None`. Write `print ($x + 1)`. A code block needs no final `;`
(bash's `{ ls -l; df; echo "Done."; }`), only its indent.

**Uninitialized variables.** Bash's are empty, not zero. tessh's are an
error, a `NameError`, so the mistake is found where it happens. Give a
variable its starting value before the loop that adds to it; for a
variable that may or may not exist (bash's `[[ -v name ]]`), `"name" in
(globals)`; for an environment variable, `$env.get NAME default`.

**`=` versus `-eq`: text versus numbers.** Bash compares text with `=` and
integers with `-eq`, and the wrong one fails in odd ways. tessh's `==`
and `<` compare values, but a value has a type: a number written in the
script is an `int` or `float`, while `$argv`, captured output and
environment variables are always `str`. Comparing `"273"` with `273` is
simply false, and `"105" < "5"` is true, as in bash's `\<`.

```bash
a=273.0   # Not an integer.

if [ "$a" = 273 ]
then
  echo "Comparison works."
else  
  echo "Comparison does not work."
fi    # Comparison does not work.
```

```output bash
Comparison does not work.
```

```tessh
a = echo 273.0          # Captured: the text "273.0".

if $a == 273            # Text and a number are never equal.
  print "Comparison works."
else
  print "Comparison does not work."
# Comparison does not work.
if (float $a) == 273    # Make it a number first.
  print "Comparison works."
```

```output
Comparison does not work.
Comparison works.
```

### Example 34-1. Numerical and string comparison are not equivalent (`bad-op.sh`)

```bash
#!/bin/bash
# bad-op.sh: Trying to use a string comparison on integers.

echo
number=1

#  The following while-loop has two errors:
#+ one blatant, and the other subtle.

while [ "$number" < 5 ]    # Wrong! Should be:  while [ "$number" -lt 5 ]
do
  echo -n "$number "
  let "number += 1"
done  
#  Attempt to run this bombs with the error message:
#+ bad-op.sh: line 10: 5: No such file or directory
#  Within single brackets, "<" must be escaped,
#+ and even then, it's still wrong for comparing integers.

echo "---------------------"

while [ "$number" \< 5 ]    #  1 2 3 4
do                          #
  echo -n "$number "        #  It *seems* to work, but . . .
  let "number += 1"         #+ it actually does an ASCII comparison,
done                        #+ rather than a numerical one.

echo; echo "---------------------"

# This can cause problems. For example:

lesser=5
greater=105

if [ "$greater" \< "$lesser" ]
then
  echo "$greater is less than $lesser"
fi                          # 105 is less than 5
#  In fact, "105" actually is less than "5"
#+ in a string comparison (ASCII sort order).

echo

exit 0
```

```output bash

---------------------
1 2 3 4 
---------------------
105 is less than 5

```

In tessh `<` is never a redirection inside an expression, and with numbers
it compares numbers. The gotcha comes back with text: as text, "105"
comes before "5". Mixing the two is a `TypeError`, which at least says so.

```tessh
#!/usr/bin/env tessh
# bad-op: comparing numbers, and comparing text.

print
number = 1

while $number < 5               # Numbers: a numerical comparison.
  print $number end=" "
  number += 1
print ; print "---------------------"

lesser = echo 5                 # Captured output is text, not a number,
greater = echo 105              # as $argv and $env are.

if $greater < $lesser
  print "$greater is less than $lesser"
# 105 is less than 5: as text, "105" sorts before "5".

if (int $greater) < (int $lesser)
  print "never printed"         # As numbers, it isn't.

try
  print ($greater < 5)
except TypeError as e
  print $e                      # Text and a number: an error.
print
```

```output

1 2 3 4 
---------------------
105 is less than 5
'<' not supported between instances of 'Output' and 'int'

```

**`let` with text.** `let "a = hello, you"` sets `a` to 0 in bash. tessh
has no `let` in that sense (its `let` makes names local to a block), and
arithmetic on text is a `TypeError`.

**Quoting.** Bash variables sometimes need quotes in `[ ]` and sometimes
must not have them. tessh never splits a value, so it never needs quotes
to keep one together; the rare time you want splitting, ask for it with
`.split` or `.lines`.

**Permissions.** A program the user can't run from the command line won't
run from a script either, in tessh as in bash.

**`-` is not a redirection.** `command1 2> - | command2` writes errors to a
file named `-`. To send errors down the pipe it's `2>&1 |`, in tessh as in
bash.

**Versions.** A script that needs a newer bash checks `$BASH_VERSION`. A
tessh script that needs a newer Python checks `sys.version_info`, a tuple
that compares properly (`$sys.version_info >= (3, 12)`), unlike bash's
text comparison of version strings. A `# /// script` block's
`requires-python` (see the guide's Dependencies section) says it too.

**`#!/bin/sh` scripts.** Bash features in a script run by `sh` may fail on
systems where `sh` isn't bash. A tessh script is always run by tessh.

**Undocumented features** can change under you; that holds for tessh too.

**Misleading exit status.** In bash `((--var))` "fails" when the result is
0, and `local var=$(cmd)` hides `cmd`'s status. tessh has neither: an
arithmetic expression is just a value, and `$?` only changes when a
program runs.

**DOS newlines.** Bash can't run a script saved with `\r\n` line ends
(`#!/bin/bash\r` isn't found). tessh reads `\r\n` as a line end, so the
script itself runs; only the `#!` line fails, as for bash, when run as
`./script`. `tessh script` works either way.

**Heredoc limit strings** must be alone on their line, without spaces in
front (or with tabs only, using `<<-`), in tessh as in bash.

**Several echoes in a function whose output is captured.** In bash a
function returns text by echoing it, so a stray `echo` ends up in the
result:

```bash
add2 ()
{
  echo "Whatever ... "   # Delete this line!
  let "retval = $1 + $2"
    echo $retval
    }

    num1=12
    num2=43
    echo "Sum of $num1 and $num2 = $(add2 $num1 $num2)"

#   Sum of 12 and 43 = Whatever ... 
#   55

#        The "echoes" concatenate.
```

```output bash
Sum of 12 and 43 = Whatever ... 
55
```

A tessh function gives back a value, its last form's, and what it prints
goes to the screen and not into the value. So the stray line prints
first, and the sum is a number.

```tessh
def add2 a b
  print "Whatever ... "   # Prints; doesn't become part of the result.
  $a + $b

num1 = 12
num2 = 43
print "Sum of $num1 and $num2 = $(add2 $num1 $num2)"
```

```output
Whatever ... 
Sum of 12 and 43 = 55
```

**Exporting to the parent.** A script can't change its parent shell's
environment: `export WHATEVER=/home/bozo` in a script is gone when it
exits. The same holds for a tessh script run from bash or from tessh. To
change the current shell, `source` the file instead.

**Subshells.** Variables set in a bash subshell, `( … )`, are gone when it
ends. tessh has no subshells: parentheses only group an expression, a
`do` block runs in the script itself, and every change sticks.

### Example 34-2. Subshell Pitfalls (`subshell-pitfalls.sh`)

```bash
#!/bin/bash
# Pitfalls of variables in a subshell.

outer_variable=outer
echo
echo "outer_variable = $outer_variable"
echo

(
# Begin subshell

echo "outer_variable inside subshell = $outer_variable"
inner_variable=inner  # Set
echo "inner_variable inside subshell = $inner_variable"
outer_variable=inner  # Will value change globally?
echo "outer_variable inside subshell = $outer_variable"

# Will 'exporting' make a difference?
#    export inner_variable
#    export outer_variable
# Try it and see.

# End subshell
)

echo
echo "inner_variable outside subshell = $inner_variable"  # Unset.
echo "outer_variable outside subshell = $outer_variable"  # Unchanged.
echo

exit 0

# What happens if you uncomment lines 19 and 20?
# Does it make a difference?
```

```output bash

outer_variable = outer

outer_variable inside subshell = outer
inner_variable inside subshell = inner
outer_variable inside subshell = inner

inner_variable outside subshell = 
outer_variable outside subshell = outer

```

To get the subshell's behavior in tessh, say so: `let` gives a block its
own copies of the names it lists, and they're gone after it. (A function
does the same for every name assigned in it.)

```tessh
#!/usr/bin/env tessh
# Variables in a block of their own.

outer_variable = outer
print
print "outer_variable = $outer_variable"
print

let outer_variable=$outer_variable inner_variable=None
  # Begin the block: these two names are its own.
  print "outer_variable inside block = $outer_variable"
  inner_variable = inner  # Set
  print "inner_variable inside block = $inner_variable"
  outer_variable = inner  # Will value change globally?
  print "outer_variable inside block = $outer_variable"
  # End the block.

print
print "inner_variable outside block = $((globals).get 'inner_variable' '')"  # Unset.
print "outer_variable outside block = $outer_variable"                         # Unchanged.
print
```

```output

outer_variable = outer

outer_variable inside block = outer
inner_variable inside block = inner
outer_variable inside block = inner

inner_variable outside block = 
outer_variable outside block = outer

```

That a function's assignments are its own has a gotcha of its own, which
is Python's: a name assigned anywhere in a function is local in all of
it, so reading the outer value first fails. `global name` says it's the
script's.

```tessh
x = 1
def bump
  x = $x + 1          # The x read here is already the local one.
try
  bump
except UnboundLocalError as e
  print $e
def bump2
  global x
  x = $x + 1
bump2
print $x
```

```output
cannot access local variable 'x' where it is not associated with a value
2
```

**Piping to `read`.** In bash, `echo … | read a b c` runs `read` in a
subshell, so the variables are set there and lost.

### Example 34-3. Piping the output of echo to a read (`badread.sh`)

```bash
#!/bin/bash
#  badread.sh:
#  Attempting to use 'echo and 'read'
#+ to assign variables non-interactively.

#   shopt -s lastpipe

a=aaa
b=bbb
c=ccc

echo "one two three" | read a b c
# Try to reassign a, b, and c.

echo
echo "a = $a"  # a = aaa
echo "b = $b"  # b = bbb
echo "c = $c"  # c = ccc
# Reassignment failed.

### However . . .
##  Uncommenting line 6:
#   shopt -s lastpipe
##+ fixes the problem!
### This is a new feature in Bash, version 4.2.

# ------------------------------

# Try the following alternative.

var=`echo "one two three"`
set -- $var
a=$1; b=$2; c=$3

echo "-------"
echo "a = $a"  # a = one
echo "b = $b"  # b = two
echo "c = $c"  # c = three 
# Reassignment succeeded.

# ------------------------------

#  Note also that an echo to a 'read' works within a subshell.
#  However, the value of the variable changes *only* within the subshell.

a=aaa          # Starting all over again.
b=bbb
c=ccc

echo; echo
echo "one two three" | ( read a b c;
echo "Inside subshell: "; echo "a = $a"; echo "b = $b"; echo "c = $c" )
# a = one
# b = two
# c = three
echo "-----------------"
echo "Outside subshell: "
echo "a = $a"  # a = aaa
echo "b = $b"  # b = bbb
echo "c = $c"  # c = ccc
echo

exit 0
```

```output bash

a = aaa
b = bbb
c = ccc
-------
a = one
b = two
c = three


Inside subshell: 
a = one
b = two
c = three
-----------------
Outside subshell: 
a = aaa
b = bbb
c = ccc

```

**Gap:** tessh has no `read`. What it reads is a value, so the words go
into names by unpacking: `[a b c] = $text.split`. That always works, with
no subshell to lose them in. The subshell half of the script is a function
used as a pipe stage, which gets the text as its argument and whose names
are its own.

```tessh
#!/usr/bin/env tessh
#  badread: assigning variables from a command's output.

a = aaa
b = bbb
c = ccc

[a b c] = ((echo "one two three").split)
# Reassign a, b, and c: it works, there's no subshell.

print
print "a = $a"  # a = one
print "b = $b"  # b = two
print "c = $c"  # c = three

#  A function in a pipe gets the text; the names it sets are its own.

a = aaa         # Starting all over again.
b = bbb
c = ccc

def inside text
  [a b c] = ($text.split)
  print "Inside function: " ; print "a = $a" ; print "b = $b" ; print "c = $c"

print ; print
echo "one two three" | inside
print "-----------------"
print "Outside function: "
print "a = $a"  # a = aaa
print "b = $b"  # b = bbb
print "c = $c"  # c = ccc
print
```

```output

a = one
b = two
c = three


Inside function: 
a = one
b = two
c = three
-----------------
Outside function: 
a = aaa
b = bbb
c = ccc

```

Piping to any loop has bash's problem too. Anthony Richardson's example
(with Wilbert Berendsen's addendum) looks for big files not read in 30
days:

```bash no-run
# Loop piping troubles.
#  This example by Anthony Richardson,
#+ with addendum by Wilbert Berendsen.


foundone=false
find $HOME -type f -atime +30 -size 100k |
while true
do
   read f
   echo "$f is over 100KB and has not been accessed in over 30 days"
   echo "Consider moving the file to archives."
   foundone=true
   # ------------------------------------
     echo "Subshell level = $BASH_SUBSHELL"
   # Subshell level = 1
   # Yes, we're inside a subshell.
   # ------------------------------------
done
   
#  foundone will always be false here since it is
#+ set to true inside a subshell
if [ $foundone = false ]
then
   echo "No files need archiving."
fi

# =====================Now, here is the correct way:=================

foundone=false
for f in $(find $HOME -type f -atime +30 -size 100k)  # No pipe here.
do
   echo "$f is over 100KB and has not been accessed in over 30 days"
   echo "Consider moving the file to archives."
   foundone=true
done
   
if [ $foundone = false ]
then
   echo "No files need archiving."
fi

# ==================And here is another alternative==================

#  Places the part of the script that reads the variables
#+ within a code block, so they share the same subshell.
#  Thank you, W.B.

find $HOME -type f -atime +30 -size 100k | {
     foundone=false
     while read f
     do
       echo "$f is over 100KB and has not been accessed in over 30 days"
       echo "Consider moving the file to archives."
       foundone=true
     done

     if ! $foundone
     then
       echo "No files need archiving."
     fi
}
```

In tessh a loop over the output's `.lines` is the natural way, and it's
"the correct way" above: `foundone` is set in the script itself. (And
unlike `$(find …)`, a name with a space in it stays one name.)

```tessh
# (Made-up files, one of them 100 KB and last read 60 days ago.)
truncate -s 100k "old report.dat" ; touch -a -d "60 days ago" "old report.dat"
truncate -s 100k new.dat ; echo small >small.txt
home = .                  # ($HOME in the original.)

foundone = False
for f in (find $home -type f -atime +30 -size 100k).lines
  print "$f is over 100KB and has not been accessed in over 30 days"
  print "Consider moving the file to archives."
  foundone = True

if not $foundone
  print "No files need archiving."
```

```output
./old report.dat is over 100KB and has not been accessed in over 30 days
Consider moving the file to archives.
```

**`tail -f | grep`.** `tail -f /var/log/messages | grep "$ERROR_MSG" >>
error.log` writes nothing for a long while, because `grep` buffers its
output when it isn't a terminal. `grep --line-buffered` fixes it, in tessh
as in bash.

**suid scripts.** Setting the suid bit on a script does nothing on Linux,
and suid programs called from scripts are a security risk.

**CGI.** Bash variables have no types and shell scripts are hard to make
safe from crackers. tessh values do have types, and Python's libraries
quote and parse for you, but treat whatever comes in a request as
hostile all the same.

**Double slashes.** Bash doesn't always handle `//` in strings well. tessh
strings are Python's and have no such trouble; a `Path` collapses `//` in
a path.

**Portability.** Scripts written with GNU tools (`tr`, `sed`, `grep`
options) may need fixing on a commercial UNIX. That holds for the programs
a tessh script runs; the Python it does itself is the same everywhere.
