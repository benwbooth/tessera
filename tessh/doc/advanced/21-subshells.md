# 21. Subshells
<!-- help: abs-subshells abs-21 subshell child-process fork parentheses -->

A subshell is a child process of the shell: a copy of it that runs some
commands and ends. Running a script starts one, and so does a list of
commands in parentheses, `( command1; command2 )`. What the subshell does
to its variables, its folder or its environment stays in it; the parent
doesn't see any of it.

```bash no-run
#!/bin/bash
# subshell-test.sh

(
# Inside parentheses, and therefore a subshell . . .
while [ 1 ]   # Endless loop.
do
  echo "Subshell running . . ."
done
)

#  Script will run forever,
#+ or at least until terminated by a Ctl-C.

exit $?  # End of script (but will never get here).
```

Run it, and `ps` shows two processes of the script: the script and its
subshell.

tessh has no subshells. Parentheses group a form (`(len $x)`), and every
loop, `if` and function runs in the script's own process, so a variable set
in a loop is there after it (chapter 20 shows what that saves). A program
tessh runs is a child process, as in bash; tessh's builtins (`cd`,
`export`, …) and Python's functions are not. When a block of tessh code
really should run apart, so that it can't change the script's names, folder
or environment, Python's `os.fork` makes the child process, as in the next
example. (To keep only *names* apart, a function or a `let` block is enough;
see chapter 24.)

### Example 21-1. Variable scope in a subshell (`subshell.sh`)

```bash
#!/bin/bash
# subshell.sh

echo

echo "We are outside the subshell."
echo "Subshell level OUTSIDE subshell = $BASH_SUBSHELL"
# Bash, version 3, adds the new         $BASH_SUBSHELL variable.
echo; echo

outer_variable=Outer
global_variable=
#  Define global variable for "storage" of
#+ value of subshell variable.

(
echo "We are inside the subshell."
echo "Subshell level INSIDE subshell = $BASH_SUBSHELL"
inner_variable=Inner

echo "From inside subshell, \"inner_variable\" = $inner_variable"
echo "From inside subshell, \"outer\" = $outer_variable"

global_variable="$inner_variable"   #  Will this allow "exporting"
                                    #+ a subshell variable?
)

echo; echo
echo "We are outside the subshell."
echo "Subshell level OUTSIDE subshell = $BASH_SUBSHELL"
echo

if [ -z "$inner_variable" ]
then
  echo "inner_variable undefined in main body of shell"
else
  echo "inner_variable defined in main body of shell"
fi

echo "From main body of shell, \"inner_variable\" = $inner_variable"
#  $inner_variable will show as blank (uninitialized)
#+ because variables defined in a subshell are "local variables".
#  Is there a remedy for this?
echo "global_variable = "$global_variable""  # Why doesn't this work?

echo

# =======================================================================

# Additionally ...

echo "-----------------"; echo

var=41                                                 # Global variable.

( let "var+=1"; echo "\$var INSIDE subshell = $var" )  # 42

echo "\$var OUTSIDE subshell = $var"                   # 41
#  Variable operations inside a subshell, even to a GLOBAL variable
#+ do not affect the value of the variable outside the subshell!


exit 0

#  Question:
#  --------
#  Once having exited a subshell,
#+ is there any way to reenter that very same subshell
#+ to modify or access the subshell variables?
```

```output bash

We are outside the subshell.
Subshell level OUTSIDE subshell = 0


We are inside the subshell.
Subshell level INSIDE subshell = 1
From inside subshell, "inner_variable" = Inner
From inside subshell, "outer" = Outer


We are outside the subshell.
Subshell level OUTSIDE subshell = 0

inner_variable undefined in main body of shell
From main body of shell, "inner_variable" = 
global_variable = 

-----------------

$var INSIDE subshell = 42
$var OUTSIDE subshell = 41
```

`subshell` runs a function in a forked child and waits for it, which is
what bash's parentheses do. As a decorator, it runs the block under it
there and then. (Output is flushed before the fork, so that the child
doesn't print the parent's unwritten output again.) The child gets copies
of the script's variables, so `global` lets it change its own copy, and
nothing comes back. `level` stands in for `$BASH_SUBSHELL`.

```tessh
#!/usr/bin/env tessh
import os sys

level = 0       # (bash's $BASH_SUBSHELL)

def subshell f
  # Run f in a child process and wait for it, as bash's ( … ) does.
  global level
  $sys.stdout.flush
  pid = (os.fork)
  if $pid == 0
    level += 1
    try
      f
    finally
      $sys.stdout.flush
      os._exit 0
  os.waitpid $pid 0

print

print "We are outside the subshell."
print "Subshell level OUTSIDE subshell = $level"
print ; print

outer_variable = Outer
global_variable = ""
#  Define global variable for "storage" of
#+ value of subshell variable.

@subshell
def _
  global inner_variable global_variable
  print "We are inside the subshell."
  print "Subshell level INSIDE subshell = $level"
  inner_variable = Inner

  print "From inside subshell, \"inner_variable\" = $inner_variable"
  print "From inside subshell, \"outer\" = $outer_variable"

  global_variable = $inner_variable   #  Will this allow "exporting"
                                      #+ a subshell variable?

print ; print
print "We are outside the subshell."
print "Subshell level OUTSIDE subshell = $level"
print

if "inner_variable" not in (globals)
  print "inner_variable undefined in main body of shell"
else
  print "inner_variable defined in main body of shell"

#  inner_variable isn't a name here at all
#+ because variables defined in a subshell are local to it.
print "global_variable = $global_variable"  # Why doesn't this work?

print

# =======================================================================

# Additionally ...

print "-----------------" ; print

var = 41                                                 # Global variable.

@subshell
def _
  global var
  var += 1 ; print "\$var INSIDE subshell = $var"          # 42

print "\$var OUTSIDE subshell = $var"                    # 41
#  Variable operations inside a subshell, even to a GLOBAL variable
#+ do not affect the value of the variable outside the subshell!

exit 0
```

```output

We are outside the subshell.
Subshell level OUTSIDE subshell = 0


We are inside the subshell.
Subshell level INSIDE subshell = 1
From inside subshell, "inner_variable" = Inner
From inside subshell, "outer" = Outer


We are outside the subshell.
Subshell level OUTSIDE subshell = 0

inner_variable undefined in main body of shell
global_variable = 

-----------------

$var INSIDE subshell = 42
$var OUTSIDE subshell = 41
```

The ABS Guide's other lines, `From main body of shell, "inner_variable" =`,
have nothing to print in tessh: reading a name that was never set is a
`NameError`, not an empty string.

bash's `$BASH_SUBSHELL` counts how deep in subshells a command is;
`$SHLVL`, the depth of shells started from shells, doesn't change in a
subshell. tessh has neither.

A `cd` in a subshell doesn't change the parent's folder either.

### Example 21-2. List User Profiles (`allprofs.sh`)

```bash no-run
#!/bin/bash
# allprofs.sh: Print all user profiles.

# This script written by Heiner Steven, and modified by the document author.

FILE=.bashrc  #  File containing user profile,
              #+ was ".profile" in original script.

for home in `awk -F: '{print $6}' /etc/passwd`
do
  [ -d "$home" ] || continue    # If no home directory, go to next.
  [ -r "$home" ] || continue    # If not readable, go to next.
  (cd $home; [ -e $FILE ] && less $FILE)
done

#  When script terminates, there is no need to 'cd' back to original directory,
#+ because 'cd $home' takes place in a subshell.

exit 0
```

The tessh version reads a made-up password file, with homes in the current
folder, and uses `cat` for `less`, which needs a terminal. It doesn't need
the `cd` the original keeps in a subshell: the path of the profile is the
home's path `/` the file name. (When a block has to change folders and come
back, `pushd` and `popd` do it.)

```tessh
#!/usr/bin/env tessh
# (A made-up /etc/passwd and home folders, to try it on.)
mkdir -p home/ann home/bob home/cy
print "echo hello from ann" >home/ann/.bashrc
print "alias ll='ls -l'" >home/cy/.bashrc
cat <<END >passwd
root:x:0:0:root:/root:/bin/sh
ann:x:1000:100::./home/ann:/bin/bash
bob:x:1001:100::./home/bob:/bin/bash
cy:x:1002:100::./home/cy:/bin/bash
nobody:x:65534:65534::./home/none:/bin/false
END

FILE = .bashrc  #  File containing user profile,
                #+ was ".profile" in original script.

import os
for home in (awk -F: '{print $6}' passwd).lines   # (/etc/passwd)
  home = (Path $home)
  if not ($home.is_dir)                  # If no home directory, go to next.
    continue
  if not (os.access $home $os.R_OK)      # If not readable, go to next.
    continue
  profile = $home / $FILE
  if ($profile.exists)
    print "== $profile"
    cat $profile                         # (less in the original)

#  No 'cd' was needed, so there's none to undo.

exit 0
```

```output
== home/ann/.bashrc
echo hello from ann
== home/cy/.bashrc
alias ll='ls -l'
```

## A dedicated environment
<!-- help: abs-subshell-env subshell-environment lock-file set-C variable-is-set -->

A subshell gives a group of commands their own environment: `IFS`,
`PATH`, options, even an `exit` that ends only the subshell.

```bash no-run
COMMAND1
COMMAND2
COMMAND3
(
  IFS=:
  PATH=/bin
  unset TERMINFO
  set -C
  shift 5
  COMMAND4
  COMMAND5
  exit 3 # Only exits the subshell!
)
# The parent shell has not been affected, and the environment is preserved.
COMMAND6
COMMAND7
```

In tessh, `NAME=value command` gives one program its own variables, a
function gives a block its own names, and `subshell` above (or a `tessh -c`
or `sh -c` run as a program) gives a block a process of its own, where
`exit` ends only that.

One use of a subshell is testing whether a variable is set, without the
`set -u` error stopping the script.

```bash
if (set -u; : $variable) 2> /dev/null
then
  echo "Variable is set."
fi     #  Variable has been set in current script,
       #+ or is an an internal Bash variable,
       #+ or is present in environment (has been exported).

# Could also be written [[ ${variable-x} != x || ${variable-y} != y ]]
# or                    [[ ${variable-x} != x$variable ]]
# or                    [[ ${variable+x} = x ]]
# or                    [[ ${variable-x} != x ]]
```

```output bash
```

In tessh a name is set or it isn't: it's in `(globals)` (the script's
variables) or in `$env` (the environment). Here it's tried before and
after it's set.

```tessh
def is_set name
  $name in (globals) or $name in $env

for _ in (range 2)
  if (is_set variable)
    print "Variable is set."
  else
    print "Variable is not set."
  variable = ""
```

```output
Variable is not set.
Variable is set.
```

Another is a lock file: with `set -C` (noclobber), `: > lock_file` fails if
the file is already there.

```bash
if (set -C; : > lock_file) 2> /dev/null
then
  :   # lock_file didn't exist: no user running the script
else
  echo "Another user is already running that script."
exit 65
fi

#  Code snippet by Stéphane Chazelas,
#+ with modifications by Paulo Marcel Coelho Aragao.
```

```output bash
```

tessh has no noclobber, but Python's `os.open` with `O_EXCL` makes the
file only if it isn't there, in one step, which is what a lock needs. The
tessh version tries twice, to show both answers.

```tessh
import os
def lock path
  try
    os.close (os.open $path ($os.O_CREAT | $os.O_EXCL | $os.O_WRONLY))
    True       # lock_file didn't exist: no user running the script
  except FileExistsError
    False

for _ in (range 2)
  if (lock lock_file)
    print "Got the lock."
  else
    print "Another user is already running that script."
```

```output
Got the lock.
Another user is already running that script.
```

## Parallel processes
<!-- help: abs-parallel-subshells background-subshell wait parallel -->

Subshells run side by side when they're put in the background.

### Example 21-3. Running parallel processes in subshells

```bash no-run
(cat list1 list2 list3 | sort | uniq > list123) &
(cat list4 list5 list6 | sort | uniq > list456) &
# Merges and sorts both sets of lists simultaneously.
# Running in background ensures parallel execution.
#
# Same effect as
#   cat list1 list2 list3 | sort | uniq > list123 &
#   cat list4 list5 list6 | sort | uniq > list456 &

wait   # Don't execute the next command until subshells finish.

diff list123 list456
```

A trailing `&` puts the whole pipeline in the background, and its value is
a `Job`, whose `.wait` waits for it to finish. **Gap:** tessh has no
`wait` command (to wait for every background job).

```tessh
# (Made-up lists to try it on.)
printf "pear\napple\n" >list1 ; printf "fig\n" >list2 ; printf "apple\nplum\n" >list3
printf "plum\nfig\n" >list4 ; printf "apple\n" >list5 ; printf "kiwi\n" >list6

a = cat list1 list2 list3 | sort | uniq > list123 &
b = cat list4 list5 list6 | sort | uniq > list456 &
# Merges and sorts both sets of lists simultaneously.

$a.wait ; $b.wait   # Don't execute the next command until both finish.

diff list123 list456
```

```output
3c3
< pear
---
> kiwi
```

A pipe into a group, `ls -al | (command)`, gives the group's commands the
input. In tessh a pipe's stage is one program or Python call.

A block in curly brackets doesn't start a subshell in bash, so its
assignments stay. tessh's `do` block is the same.

```bash
var1=23
echo "$var1"   # 23

{ var1=76; }
echo "$var1"   # 76
```

```tessh
var1 = 23
print $var1    # 23

do
  var1 = 76
print $var1    # 76
```

```output
23
76
```
