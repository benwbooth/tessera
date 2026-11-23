# 2. Starting Off With a Sha-Bang
<!-- help: abs-sha-bang abs-2 sha-bang shebang #! script-header -->

At its simplest a script is a list of commands in a file, saved so they
don't have to be typed again. The ABS Guide's first script empties two log
files.

### Example 2-1. cleanup: clean up the log files in /var/log (`ex1.sh`)

```bash no-run
# Cleanup
# Run as root, of course.

cd /var/log
cat /dev/null > messages
cat /dev/null > wtmp
echo "Log files cleaned up."
```

The same commands are a tessh script as they stand. Here it works on a
made-up `log` folder, not the real `/var/log`.

```tessh
# (A made-up /var/log to try it on.)
mkdir -p log ; echo old >log/messages ; echo old >log/wtmp

# Cleanup
# Run as root, of course.

cd log                       # (/var/log in the original)
cat /dev/null > messages
cat /dev/null > wtmp
echo "Log files cleaned up."
```

```output
Log files cleaned up.
```

### Example 2-2. cleanup: an improved clean-up script (`ex1a.sh`)

```bash no-run
#!/bin/bash
# Proper header for a Bash script.

# Cleanup, version 2

# Run as root, of course.
# Insert code here to print error message and exit if not root.

LOG_DIR=/var/log
# Variables are better than hard-coded values.
cd $LOG_DIR

cat /dev/null > messages
cat /dev/null > wtmp


echo "Logs cleaned up."

exit #  The right and proper method of "exiting" from a script.
     #  A bare "exit" (no parameter) returns the exit status
     #+ of the preceding command. 
```

A tessh variable is a Python name, so `LOG_DIR = …` has spaces around `=`
if you like (`LOG_DIR=/var/log` glued works too, as in bash). `$LOG_DIR` reads
it. `exit` with no status exits 0 in tessh; bash's bare `exit` passes on the
status of the command before it.

```tessh
mkdir -p log ; echo old >log/messages ; echo old >log/wtmp

# Cleanup, version 2

LOG_DIR = ./log              # (/var/log in the original)
cd $LOG_DIR

cat /dev/null > messages
cat /dev/null > wtmp

echo "Logs cleaned up."

exit
```

```output
Logs cleaned up.
```

### Example 2-3. cleanup: an enhanced and generalized version (`ex2.sh`)

```bash no-run
#!/bin/bash
# Cleanup, version 3

#  Warning:
#  -------
#  This script uses quite a number of features that will be explained
#+ later on.
#  By the time you've finished the first half of the book,
#+ there should be nothing mysterious about it.



LOG_DIR=/var/log
ROOT_UID=0     # Only users with $UID 0 have root privileges.
LINES=50       # Default number of lines saved.
E_XCD=86       # Can't change directory?
E_NOTROOT=87   # Non-root exit error.


# Run as root, of course.
if [ "$UID" -ne "$ROOT_UID" ]
then
  echo "Must be root to run this script."
  exit $E_NOTROOT
fi  

if [ -n "$1" ]
# Test whether command-line argument is present (non-empty).
then
  lines=$1
else  
  lines=$LINES # Default, if not specified on command-line.
fi  


#  Stephane Chazelas suggests the following,
#+ as a better way of checking command-line arguments,
#+ but this is still a bit advanced for this stage of the tutorial.
#
#    E_WRONGARGS=85  # Non-numerical argument (bad argument format).
#
#    case "$1" in
#    ""      ) lines=50;;
#    *[!0-9]*) echo "Usage: `basename $0` lines-to-cleanup";
#     exit $E_WRONGARGS;;
#    *       ) lines=$1;;
#    esac
#
#* Skip ahead to "Loops" chapter to decipher all this.


cd $LOG_DIR

if [ `pwd` != "$LOG_DIR" ]  # or   if [ "$PWD" != "$LOG_DIR" ]
                            # Not in /var/log?
then
  echo "Can't change to $LOG_DIR."
  exit $E_XCD
fi  # Doublecheck if in right directory before messing with log file.

# Far more efficient is:
#
# cd /var/log || {
#   echo "Cannot change to necessary directory." >&2
#   exit $E_XCD;
# }




tail -n $lines messages > mesg.temp # Save last section of message log file.
mv mesg.temp messages               # Rename it as system log file.


#  cat /dev/null > messages
#* No longer needed, as the above method is safer.

cat /dev/null > wtmp  #  ': > wtmp' and '> wtmp'  have the same effect.
echo "Log files cleaned up."
#  Note that there are other log files in /var/log not affected
#+ by this script.

exit 0
#  A zero return value from the script upon exit indicates success
#+ to the shell.
```

In tessh the tests are Python: `os.getuid` for bash's `$UID`, `len $argv`
for `$#`, and `$argv[1]` for `$1` (`$argv[0]` is the script, as `$0` is).
`if` takes an indented block, with no `then` or `fi`.

```tessh
import os
# (A made-up /var/log, its message log 100 lines long, to try it on.)
mkdir -p log ; seq 100 >log/messages ; echo logins >log/wtmp

LOG_DIR = "$(pwd)/log"       # (/var/log in the original)
ROOT_UID = (os.getuid)       # (0, root, in the original: here, you)
LINES = 50                   # Default number of lines saved.
E_XCD = 86                   # Can't change directory?
E_NOTROOT = 87               # Non-root exit error.

# Run as root, of course.
if (os.getuid) != $ROOT_UID
  print "Must be root to run this script."
  exit $E_NOTROOT

# The number of lines to keep: the first argument, if there is one.
lines = if (len $argv) > 1 and $argv[1] then $argv[1] else $LINES

cd $LOG_DIR

if (pwd) != $LOG_DIR         # Not in the log folder?
  print "Can't change to $LOG_DIR."
  exit $E_XCD

tail -n $lines messages > mesg.temp   # Save last section of message log file.
mv mesg.temp messages                 # Rename it as system log file.

cat /dev/null > wtmp
echo "Log files cleaned up."
wc -l messages wtmp          # (To see what it did.)
exit 0
```

```output
Log files cleaned up.
 50 messages
  0 wtmp
 50 total
```

The `cd` check is there because a failed `cd` in bash only prints an error
and goes on, so the script would cut the logs of whatever folder it was in.
In tessh a `cd` that fails is an error that stops the script, so the check
is a belt with braces; to handle it yourself, catch it. (`cd dir || exit 1`
also works, as in bash, when the folder is there.)

```tessh
try
  cd /no/such/folder
except OSError as e
  print "Cannot change to necessary directory: $e"
```

```output
Cannot change to necessary directory: cd: /no/such/folder: No such file or directory
```

The ABS Guide's better check of the argument, with `case`, is a `match` in
tessh: a pattern for "no argument", one for a number, and anything else.

```bash no-run
E_WRONGARGS=85  # Non-numerical argument (bad argument format).

case "$1" in
""      ) lines=50;;
*[!0-9]*) echo "Usage: `basename $0` lines-to-cleanup";
 exit $E_WRONGARGS;;
*       ) lines=$1;;
esac
```

```tessh
def lines_for args
  match $args
    case [] | [""]
      50
    case [$n] if ($n.isdigit)
      int $n
    case _
      print "Usage: cleanup lines-to-cleanup"
      None
print (lines_for []) (lines_for ["20"]) (lines_for ["x1"])
```

```output
Usage: cleanup lines-to-cleanup
50 20 None
```

## The sha-bang
<!-- help: abs-shebang shebang sha-bang hashbang interpreter env -->

`#!` at the top of a script names the program that runs it. For tessh that's
`#!/usr/bin/env tessh` (`env` finds it on `PATH`). Below the first line,
`#!` is only a comment, in bash and in tessh: a second one doesn't start
another script.

```bash
#!/bin/bash
echo "Part 1 of script."
a=1

#!/bin/bash
# This does *not* launch a new script.

echo "Part 2 of script."
echo $a  # Value of $a stays at 1.
```

```tessh
#!/usr/bin/env tessh
print "Part 1 of script."
a = 1

#!/usr/bin/env tessh
# This does *not* launch a new script.

print "Part 2 of script."
print $a  # Value of $a stays at 1.
```

```output
Part 1 of script.
Part 2 of script.
1
```

The ABS Guide's other headers (`#!/bin/sh`, `#!/usr/bin/perl`,
`#!/bin/sed -f`, `#!/bin/awk -f`) and its tricks (`#!/bin/rm` for a script
that deletes itself, `#!/bin/more` for a README that shows itself) are the
kernel's doing, not the shell's, so they work the same with tessh around.

A script prolog that checks the number of arguments:

```bash no-run
E_WRONG_ARGS=85
script_parameters="-a -h -m -z"
#                  -a = all, -h = help, etc.

if [ $# -ne $Number_of_expected_args ]
then
  echo "Usage: `basename $0` $script_parameters"
  # `basename $0` is the script's filename.
  exit $E_WRONG_ARGS
fi
```

```tessh
E_WRONG_ARGS = 85
script_parameters = "-a -h -m -z"
#                    -a = all, -h = help, etc.
Number_of_expected_args = 1

if (len $argv) - 1 != $Number_of_expected_args
  print "Usage: $(Path $argv[0]).name $script_parameters"
  # (Path $argv[0]).name is the script's file name.
  exit $E_WRONG_ARGS
```

```output
Usage: ex.tsh -a -h -m -z
```

(Python's `argparse` does the whole job, usage message and all, when a
script grows real options: see chapter 15's `getopts`.)

### Invoking the script

As with bash: `tessh scriptname`, or make it executable (`chmod u+rx
scriptname`) with a `#!/usr/bin/env tessh` first line and run
`./scriptname`. The current folder isn't on `PATH`, hence the `./`. Moved to
a folder on `PATH` (`~/.local/bin`, `/usr/local/bin`), it runs by name.
`tessh --check scriptname` finds mistakes without running it, which bash
can't do.

### Preliminary exercises

1. Think of tasks a system administrator would automate with a script.
2. Write a script that shows the date and time, lists the logged-in users
   and the system's uptime, and saves all that to a log file. In tessh:

```tessh no-run
with (open sysinfo.log a) as log
  for cmd in [[date] [who] [uptime]]
    out = (command *$cmd)
    print $out
    print $out file=$log
```
