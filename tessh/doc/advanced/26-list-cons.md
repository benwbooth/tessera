# 26. List Constructs
<!-- help: abs-list-cons abs-26 and-list or-list && || chaining -->

An *and list*, `command-1 && command-2 && …`, runs each command while the
ones before it succeed (exit status 0), and stops at the first that fails.
An *or list*, `command-1 || command-2 || …`, runs each while the ones
before it fail, and stops at the first that succeeds. Either can stand in
for an `if`. The status of the list is that of the last command it ran.

tessh has both, for programs and for Python values alike: `&&` and `and`
are the same, and so are `||` and `or`. A program counts as true when it
succeeds; a Python value by its truth (non-empty, non-zero, not `None`).
`exit` and `print` work in them too.

### Example 26-1. Using an and list to test for command-line arguments (`ex64.sh`)

```bash
#!/bin/bash
# and list

if [ ! -z "$1" ] && echo "Argument #1 = $1" && [ ! -z "$2" ] && \
#                ^^                         ^^               ^^
echo "Argument #2 = $2"
then
  echo "At least 2 arguments passed to script."
  # All the chained commands return true.
else
  echo "Fewer than 2 arguments passed to script."
  # At least one of the chained commands returns false.
fi  
# Note that "if [ ! -z $1 ]" works, but its alleged equivalent,
#   "if [ -n $1 ]" does not.
#     However, quoting fixes this.
#  if "[ -n "$1" ]" works.
#           ^  ^    Careful!
# It is always best to QUOTE the variables being tested.


# This accomplishes the same thing, using "pure" if/then statements.
if [ ! -z "$1" ]
then
  echo "Argument #1 = $1"
fi
if [ ! -z "$2" ]
then
  echo "Argument #2 = $2"
  echo "At least 2 arguments passed to script."
else
  echo "Fewer than 2 arguments passed to script."
fi
# It's longer and more ponderous than using an "and list".


exit $?
```

```output bash
Fewer than 2 arguments passed to script.
Fewer than 2 arguments passed to script.
```

The script's body is a function here, run twice: with no arguments, as
bash ran it, and with two. The and list tests with `test`, as the
original does with `[`, so that each `echo` prints as it goes. Note the
`\` continuing the condition: it can't follow the `&&` on the same line
with a comment, in tessh or in bash.

```tessh
#!/usr/bin/env tessh
def main args
  a1 = if (len $args) > 0 then $args[0] else ""
  a2 = if (len $args) > 1 then $args[1] else ""

  if test -n $a1 && echo "Argument #1 = $a1" && test -n $a2 && \
     echo "Argument #2 = $a2"
    print "At least 2 arguments passed to script."
    # All the chained commands return true.
  else
    print "Fewer than 2 arguments passed to script."
    # At least one of the chained commands returns false.

  # This accomplishes the same thing, using "pure" if statements.
  if $a1
    print "Argument #1 = $a1"
  if $a2
    print "Argument #2 = $a2"
    print "At least 2 arguments passed to script."
  else
    print "Fewer than 2 arguments passed to script."

main $argv[1:]
print "---"
main [one two]      # (sample arguments)

exit $?
```

```output
Fewer than 2 arguments passed to script.
Fewer than 2 arguments passed to script.
---
Argument #1 = one
Argument #2 = two
At least 2 arguments passed to script.
Argument #1 = one
Argument #2 = two
At least 2 arguments passed to script.
```

### Example 26-2. Another command-line arg test using an and list (`and-list2.sh`)

```bash
#!/bin/bash

ARGS=1        # Number of arguments expected.
E_BADARGS=85  # Exit value if incorrect number of args passed.

test $# -ne $ARGS && \
#    ^^^^^^^^^^^^ condition #1
echo "Usage: `basename $0` $ARGS argument(s)" && exit $E_BADARGS
#                                             ^^
#  If condition #1 tests true (wrong number of args passed to script),
#+ then the rest of the line executes, and script terminates.

# Line below executes only if the above test fails.
echo "Correct number of arguments passed to this script."

exit 0

# To check exit value, do a "echo $?" after script termination.
```

```output bash
Usage: ex.sh 1 argument(s)
```

The `test` is given the number of arguments, `(len $argv) - 1`. Run here
without arguments, the usage message is what it prints.

```tessh
#!/usr/bin/env tessh
ARGS = 1        # Number of arguments expected.
E_BADARGS = 85  # Exit value if incorrect number of args passed.

test ((len $argv) - 1) -ne $ARGS && \
  echo "Usage: $(Path $argv[0]).name $ARGS argument(s)" && exit $E_BADARGS
#  If condition #1 tests true (wrong number of args passed to script),
#+ then the rest of the line executes, and script terminates.

# Line below executes only if the above test fails.
print "Correct number of arguments passed to this script."

exit 0
```

```output
Usage: ex.tsh 1 argument(s)
```

An and list can also give a default, though in tessh `or` says it better.
(**Bug:** a bareword right after `or` or `and` is taken for an argument of
what's before it, so the default is quoted.)

```bash no-run
arg1=$@ && [ -z "$arg1" ] && arg1=DEFAULT
# Set $arg1 to command-line arguments, if any.
# But . . . set to DEFAULT if not specified on command-line.
```

```tessh
arg1 = (' '.join $argv[1:]) or "DEFAULT"
# The command-line arguments, if any; else DEFAULT.
print $arg1
```

```output
DEFAULT
```

### Example 26-3. Using or lists in combination with an and list (`ex65.sh`)

```bash no-run
#!/bin/bash

#  delete.sh, a not-so-cunning file deletion utility.
#  Usage: delete filename

E_BADARGS=85

if [ -z "$1" ]
then
  echo "Usage: `basename $0` filename"
  exit $E_BADARGS  # No arg? Bail out.
else  
  file=$1          # Set filename.
fi  


[ ! -f "$file" ] && echo "File \"$file\" not found. \
Cowardly refusing to delete a nonexistent file."
# AND LIST, to give error message if file not present.
# Note echo message continuing on to a second line after an escape.

[ ! -f "$file" ] || (rm -f $file; echo "File \"$file\" deleted.")
# OR LIST, to delete file if present.

# Note logic inversion above.
# AND LIST executes on true, OR LIST on false.

exit $?
```

The file to delete is made up, and the script is run on it twice: once
when it's there, once when it isn't. `($f.is_file)` is a Python value
in the lists, and `rm` a program.

```tessh
#!/usr/bin/env tessh
E_BADARGS = 85
touch doomed.txt                # (A made-up file to delete.)

def delete args
  if not $args
    print "Usage: delete filename"
    return $E_BADARGS   # No arg? Bail out.
  file = $args[0]       # Set filename.
  f = Path $file

  ($f.is_file) || print "File \"$file\" not found. \
Cowardly refusing to delete a nonexistent file."
  # OR LIST, to give error message if file not present.

  ($f.is_file) && rm -f $file && print "File \"$file\" deleted."
  # AND LIST, to delete file if present.

delete [doomed.txt]
delete [doomed.txt]

exit $?
```

```output
File "doomed.txt" deleted.
File "doomed.txt" not found. Cowardly refusing to delete a nonexistent file.
```

Snippets from an old init script, using lists to check for a program
before running it and to skip files in a loop:

```bash no-run
# ==> The following snippets from the /etc/rc.d/init.d/single
#+==> script by Miquel van Smoorenburg
#+==> illustrate use of "and" and "or" lists.
# ==> "Arrowed" comments added by document author.

[ -x /usr/bin/clear ] && /usr/bin/clear
  # ==> If /usr/bin/clear exists, then invoke it.
  # ==> Checking for the existence of a command before calling it
  #+==> avoids error messages and other awkward consequences.

  # ==> . . .

# If they want to run something in single user mode, might as well run it...
for i in /etc/rc1.d/S[0-9][0-9]* ; do
        # Check if the script is there.
        [ -x "$i" ] || continue
  # ==> If corresponding file in $PWD *not* found,
  #+==> then "continue" by jumping to the top of the loop.

        # Reject backup files and files generated by rpm.
        case "$1" in
                *.rpmsave|*.rpmorig|*.rpmnew|*~|*.orig)
                        continue;;
        esac
        [ "$i" = "/etc/rc1.d/S00single" ] && continue
  # ==> Set script name, but don't execute it yet.
        $i start
done
```

In tessh a `continue` goes in an `if` (it's a statement, not a command);
the `case` on file names is a `=~` with a glob, each in parentheses
(**bug:** without them, `$n =~ *.orig or …` takes the glob for a command).
(The `clear` line is the
same in tessh, `test -x /usr/bin/clear && /usr/bin/clear`, but it would
clear the screen here.) Here on a made-up
`rc1.d`. (The original's `case "$1"` tests the script's argument, where
`$i` is surely meant.)

```tessh
import os
# (A made-up /etc/rc1.d to try it on.)
mkdir -p rc1.d
for name in [S10net S20web S20web.rpmsave S00single K01old]
  print "echo $name started" >rc1.d/$name
  chmod +x rc1.d/$name

for i in (Glob 'rc1.d/S[0-9][0-9]*')
  # Check if the script is there.
  if not (os.access $i $os.X_OK)
    continue
  # Reject backup files and files generated by rpm.
  name = $i.name
  if ($name =~ *.rpmsave) or ($name =~ *.rpmorig) or ($name =~ *.rpmnew) \
     or ($name =~ *~) or ($name =~ *.orig)
    continue
  if $name == S00single
    continue
  sh $i start
```

```output
S10net started
S20web started
```

Combinations of the two group left to right, so `a && b || c` is
`(a && b) || c`; they get hard to follow quickly.

```bash
false && true || echo false         # false

# Same result as
( false && true ) || echo false     # false
# But NOT
false && ( true || echo false )     # (nothing echoed)

#  Note left-to-right grouping and evaluation of statements.

#  It's usually best to avoid such complexities.

#  Thanks, S.C.
```

```tessh
false && true || echo false         # false

# Same result as
(false && true) || echo false       # false
# But NOT
false && (true || echo false)       # (nothing echoed)
```

```output
false
false
```
