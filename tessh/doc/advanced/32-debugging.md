# 32. Debugging
<!-- help: abs-debugging abs-32 debugging debug trap traceback check xtrace pdb -->

Bash has no debugger and only a few debugging switches, and its error
messages often point somewhere other than the mistake. tessh is different
here more than anywhere: it compiles the whole script before running it, so
`tessh --check` finds syntax errors with a line and column; at run time a
mistake is a Python exception with a traceback that quotes the tessh line
and puts carets under the part that failed; `tessh --py` shows the Python a
script becomes; and Python's own tools (`pdb`, `sys.settrace`, `inspect`,
`atexit`, `signal`) work on tessh scripts and report tessh line numbers.

What tessh does not have is most of bash's switches: `set -u`, `set -x`,
`set -v` and `trap` don't exist (a `set -x` line is an error that says
so). `set -e` is tessh's strict mode (`strict`, chapter 33). This chapter shows what to use instead.

### Example 32-1. A buggy script (`ex74.sh`)

```bash
#!/bin/bash
# ex74.sh

# This is a buggy script.
# Where, oh where is the error?

a=37

if [$a -gt 27 ]
then
  echo $a
fi  

exit $?   # 0! Why?
```

```output bash
```

Bash prints only `./ex74.sh: line 9: [37: command not found` and exits 0:
`[$a` glued together is the command `[37`, which doesn't exist. Carried
over to tessh as it stands, `[$a -gt 27 ]` is a Python list, `[37, '-gt',
27]`, and a list with anything in it is true. tessh says so before running
anything:

```tessh no-check no-run
#!/usr/bin/env tessh
# This is a buggy script.
# Where, oh where is the error?

a = 37

if [$a -gt 27 ]
  echo $a
```

```text
$ tessh ex74.tsh
tessh: ex74.tsh:6:4: a list as the condition is always true (unless empty): for bash's [ … ] test, write `test …` or the test in Python
```

The test in tessh is a Python comparison:

```tessh
for a in [37 5]
  if $a > 27             # The comparison.
    print "> says $a is more than 27"
  if test $a -gt 27      # Or the test program, as bash's [ ] is.
    print "test says $a is more than 27"
```

```output
> says 37 is more than 27
test says 37 is more than 27
```

### Example 32-2. Missing keyword (`missing-keyword.sh`)

```bash
#!/bin/bash
# missing-keyword.sh
# What error message will this script generate? And why?

for a in 1 2 3
do
  echo "$a"
# done     # Required keyword 'done' commented out in line 8.

exit 0     # Will not exit here!

# === #

# From command line, after script terminates:
  echo $?    # 2
```

```output bash
```

Bash reports `line 16: syntax error: unexpected end of file`: where it
noticed, not where the mistake was. tessh has no `done`; the loop's body is
its indented lines. Forget the indent and you get the tessh form of this
mistake, which tessh reports at the loop, before running anything:

```tessh no-run no-check
#!/usr/bin/env tessh
# missing-indent: what will this script print? And why?

for a in [1 2 3]
print "$a"       # The loop's body, not indented.

exit 0
```

```text
$ tessh --check missing-indent.tsh
missing-indent.tsh:3:1: 'for' needs an indented block under it (bash's do … done)
```

An unclosed bracket or string, on the other hand, is reported where it
opens, by `tessh --check` without running anything, and by `tessh` before
it runs any of the script:

```tessh no-run no-check
for a in [1 2 3
  print "$a"

exit 0
```

```text
$ tessh --check missing-bracket.tsh
missing-bracket.tsh:1:10: missing ']' to close the '['
```

### Example 32-3. test24: another buggy script (`ex75.sh`)

```bash no-run
#!/bin/bash

#  This script is supposed to delete all filenames in current directory
#+ containing embedded spaces.
#  It doesn't work.
#  Why not?


badname=`ls | grep ' '`

# Try this:
# echo "$badname"

rm "$badname"

exit 0
```

The same bug survives into tessh: a value is never split into words, so
`rm $badname` gets one argument, both names with a newline between them,
and fails. `print [$badname]` (the list shows the newline) finds it.

```tessh
#!/usr/bin/env tessh
# (Made-up files, two with spaces in their names, to try it on.)
touch "a b.txt" "c d.txt" plain.txt

#  This script is supposed to delete all filenames in current directory
#+ containing embedded spaces. It doesn't work. Why not?

badname = ls | grep ' '

# Try this:
print [$badname]

rm $badname 2>/dev/null   # (rm: cannot remove 'a b.txt'$'\n''c d.txt')
ls
```

```output
['a b.txt\nc d.txt']
a b.txt
c d.txt
plain.txt
```

The right ways: a glob, which gives each name as its own argument (as
`rm *\ *` does in bash), or the output's `.lines`.

```tessh
touch "a b.txt" "c d.txt" plain.txt
rm *\ *                   # Or: rm *" "*   or: rm $badname.lines
ls
```

```output
plain.txt
```

A buggy script bombs with a syntax error, runs but does the wrong thing (a
logic error), or does the right thing with nasty side effects (a logic
bomb). The tools for finding out why follow.

**Echo statements** at critical points show what's really in the
variables. Better is one that echoes only when debugging is on, as Stefano
Falsetto's `debecho`:

```bash
### debecho (debug-echo), by Stefano Falsetto ###
### Will echo passed parameters only if DEBUG is set to a value. ###
debecho () {
  if [ ! -z "$DEBUG" ]; then
     echo "$1" >&2
     #         ^^^ to stderr
  fi
}

DEBUG=on
Whatever=whatnot
debecho $Whatever   # whatnot

DEBUG=
Whatever=notwhat
debecho $Whatever   # (Will not echo.)
```

```output bash
```

```tessh
### debecho (debug-echo), by Stefano Falsetto ###
### Will echo passed parameters only if DEBUG is set to a value. ###
def debecho *words
  if $DEBUG
    print *$words >&2      # to stderr

DEBUG = on
Whatever = whatnot
debecho $Whatever 2>&1     # whatnot  (2>&1 to see it here)

DEBUG=
Whatever = notwhat
debecho $Whatever 2>&1     # (Will not echo.)
```

```output
whatnot
```

Python's `logging` module is the grown-up `debecho`: `logging.debug "x is
$x"` prints only when `logging.basicConfig level=$logging.DEBUG` has turned
it on, with the time and level in front if you like. **`tee`** in a pipe
shows what passes through at that point, in tessh as in bash.

**Option flags.** Bash's are switches on the whole shell:

- `sh -n script` (`set -n`) checks the syntax without running it. tessh's
  is `tessh --check script` (`--json` for a program to read): it compiles
  the whole script, as tessh always does before running one, and prints
  `file:line:col: message` per problem.
- `sh -v script` (`set -v`) echoes each line before running it.
  **Gap:** tessh has nothing like it.
- `sh -x script` (`set -x`) echoes each command, expanded, as it runs.
  **Gap:** tessh has no `-x` either. `tessh --py script` shows what each
  line compiles to, and `sys.settrace` can do the echoing, as below.
- `set -u` (`set -o nounset`) makes an unset variable an error. In tessh it
  always is.

`set -x` carried over from bash is an error in tessh ("tessh has no set
options but -e"), as are `set -u` and the rest, so a script doesn't
quietly run without what it asked for. `set -e` works: it's strict mode. (`set` alone is Python's set type.)

```bash no-run
set -u   # Or   set -o nounset

# Setting a variable to null will not trigger the error/abort.
# unset_var=

echo $unset_var   # Unset (and undeclared) variable.

echo "Should not echo!"

# sh t2.sh
# t2.sh: line 6: unset_var: unbound variable
```

```tessh no-run
print $unset_var   # Unset (and undeclared) variable.

print "Should not echo!"
```

```text
$ tessh t2.tsh
Traceback (most recent call last):
  File "t2.tsh", line 1, in <module>
    print $unset_var   # Unset (and undeclared) variable.
          ^^^^^^^^^^
NameError: name 'unset_var' is not defined (and there's no environment variable unset_var)
```

For `-x`, Python calls a function of yours before each line it runs, once
`sys.settrace` has it. The frame it gets has the line number, and the line
number is the tessh script's, so `linecache` gives back the tessh line
itself. (It traces the script's own lines as written, not expanded as
bash's `-x` does.)

```tessh
import sys linecache

def xtrace frame event arg     # Called by Python as each line runs.
  if $event == "line" and $frame.f_code.co_filename == $__file__
    print "+" (.strip (linecache.getline $__file__ $frame.f_lineno))
  $xtrace

def set_x                      # Trace from here on, in this script.
  sys.settrace $xtrace
  (sys._getframe 1).f_trace = $xtrace

set_x
a = 5
for i in (range 2)
  a += $i
print $a
```

```output
+ a = 5
+ for i in (range 2)
+ a += $i
+ for i in (range 2)
+ a += $i
+ for i in (range 2)
+ print $a
6
```

(It prints on stdout here, to show; `-x` writes to stderr. Take out
`set_x` and it's gone.)

**An `assert`** tests a condition at a critical point and stops the script
if it's false, an idea borrowed from C.

### Example 32-4. Testing a condition with an assert (`assert.sh`)

```bash no-run
#!/bin/bash
# assert.sh

#######################################################################
assert ()                 #  If condition false,
{                         #+ exit from script
                          #+ with appropriate error message.
  E_PARAM_ERR=98
  E_ASSERT_FAILED=99


  if [ -z "$2" ]          #  Not enough parameters passed
  then                    #+ to assert() function.
    return $E_PARAM_ERR   #  No damage done.
  fi

  lineno=$2

  if [ ! $1 ] 
  then
    echo "Assertion failed:  \"$1\""
    echo "File \"$0\", line $lineno"    # Give name of file and line number.
    exit $E_ASSERT_FAILED
  # else
  #   return
  #   and continue executing the script.
  fi  
} # Insert a similar assert() function into a script you need to debug.    
#######################################################################


a=5
b=4
condition="$a -lt $b"     #  Error message and exit from script.
                          #  Try setting "condition" to something else
                          #+ and see what happens.

assert "$condition" $LINENO
# The remainder of the script executes only if the "assert" does not fail.


# Some commands.
# Some more commands . . .
echo "This statement echoes only if the \"assert\" does not fail."
# . . .
# More commands . . .

exit $?
```

(The bash version prints the path it was run by, so it isn't run here.) The
same function in tessh needs no `$LINENO` passed to it: `inspect.stack`
tells it who called it, from which file and line, and the text of that
line.

```tessh
#!/usr/bin/env tessh
# assert: exit with a message if a condition is false.
import inspect

#######################################################################
def assert_ condition          #  If condition false,
                               #+ exit from script
                               #+ with appropriate error message.
  E_ASSERT_FAILED = 99

  if not $condition
    caller = (inspect.stack)[1]          # Who called us, and where.
    print "Assertion failed:  \"$(($caller.code_context[0]).strip)\""
    print "File \"$((Path $caller.filename).name)\", line $caller.lineno"
    exit $E_ASSERT_FAILED
# Insert a similar function into a script you need to debug.
#######################################################################

a = 5
b = 4

#  Error message and exit from script.
#  Try another condition and see what happens.
assert_ ($a < $b)
# The remainder of the script executes only if the assert does not fail.

print "This statement echoes only if the \"assert\" does not fail."
```

```output
Assertion failed:  "assert_ ($a < $b)"
File "ex.tsh", line 24
```

Python's own `assert` does all of this already, with the traceback for the
file and line:

```tessh no-run
a = 5
b = 4
assert ($a < $b) "a should be less than b"
print "This statement echoes only if the assert does not fail."
```

```text
$ tessh assert.tsh
Traceback (most recent call last):
  File "assert.tsh", line 3, in <module>
    assert ($a < $b) "a should be less than b"
               ^
AssertionError: a should be less than b
```

**`$LINENO` and `caller`.** **Gap:** tessh has neither by name.
`(inspect.currentframe).f_lineno` is the current line,
`(inspect.stack)[1]` the caller's file, line and function, and
`traceback.print_stack` prints the whole chain of calls.

**Python's debugger.** `breakpoint` on a line of its own stops the script
there in `pdb`, which lists the tessh source and steps through it line by
line (`n`, `s`, `c`, `l`, `w`). Its `p` takes Python, so a variable is
`p total`, without the `$`.

```text
$ tessh total.tsh
> total.tsh(2)<module>()
-> breakpoint
(Pdb) l
  1     variable = 29
  2  -> breakpoint
  3     variable *= 3
  4     print $variable
(Pdb) p variable
29
```

**Trapping at exit.** Bash's `trap` runs a command when the script gets a
signal, and its `EXIT` "signal" when the script ends. A signal is a message
from the kernel or another process telling a program to do something,
usually to stop: Control-C sends `INT`. tessh has no `trap`. **Gap:** for
the same jobs it uses Python: `atexit.register` for `EXIT`, `try`/`finally`
for cleaning up however a block ends, `KeyboardInterrupt` for Control-C,
and `signal.signal` for the rest.

### Example 32-5. Trapping at exit (`ex76.sh`)

```bash
#!/bin/bash
# Hunting variables with a trap.

trap 'echo Variable Listing --- a = $a  b = $b' EXIT
#  EXIT is the name of the signal generated upon exit from a script.
#
#  The command specified by the "trap" doesn't execute until
#+ the appropriate signal is sent.

echo "This prints before the \"trap\" --"
echo "even though the script sees the \"trap\" first."
echo

a=39

b=36

exit 0
#  Note that commenting out the 'exit' command makes no difference,
#+ since the script exits in any case after running out of commands.
```

`atexit.register` takes a function, run when the script ends, by `exit` or
by running out of lines. It reads `a` and `b` then, not when it was
registered.

```tessh
#!/usr/bin/env tessh
# Hunting variables with an exit handler.
import atexit

def listing
  print Variable Listing --- a = $a b = $b
atexit.register $listing
#  The function doesn't run until the script exits.

print "This prints before the \"trap\" --"
print "even though the script sees the \"trap\" first."
print

a = 39

b = 36

exit 0
#  Commenting out the 'exit' makes no difference.
```

```output
This prints before the "trap" --
even though the script sees the "trap" first.

Variable Listing --- a = 39 b = 36
```

Bash's `trap '' 2` ignores Control-C and `trap 'echo …' 2` runs a command
for it:

```bash
trap '' 2
# Ignore interrupt 2 (Control-C), with no action specified. 

trap 'echo "Control-C disabled."' 2
# Message when Control-C pressed.
```

```output bash
```

`signal.signal` takes the signal and a function of two arguments (the
signal's number and the frame it interrupted), or `SIG_IGN` to ignore it.
Here the script sends itself the Control-C, with `os.kill`, to show each.

```tessh
import os signal

signal.signal $signal.SIGINT $signal.SIG_IGN
# Ignore interrupt 2 (Control-C), with no action specified.
os.kill (os.getpid) $signal.SIGINT
print "still here"

signal.signal $signal.SIGINT (fn [signum frame] (print "Control-C disabled."))
# Message when Control-C pressed.
os.kill (os.getpid) $signal.SIGINT
```

```output
still here
Control-C disabled.
```

### Example 32-6. Cleaning up after Control-C (`online.sh`)

```bash no-run
#!/bin/bash
# logon.sh: A quick 'n dirty script to check whether you are on-line yet.

umask 177  # Make sure temp files are not world readable.


TRUE=1
LOGFILE=/var/log/messages
#  Note that $LOGFILE must be readable
#+ (as root, chmod 644 /var/log/messages).
TEMPFILE=temp.$$
#  Create a "unique" temp file name, using process id of the script.
#     Using 'mktemp' is an alternative.
#     For example:
#     TEMPFILE=`mktemp temp.XXXXXX`
KEYWORD=address
#  At logon, the line "remote IP address xxx.xxx.xxx.xxx"
#                      appended to /var/log/messages.
ONLINE=22
USER_INTERRUPT=13
CHECK_LINES=100
#  How many lines in log file to check.

trap 'rm -f $TEMPFILE; exit $USER_INTERRUPT' TERM INT
#  Cleans up the temp file if script interrupted by control-c.

echo

while [ $TRUE ]  #Endless loop.
do
  tail -n $CHECK_LINES $LOGFILE> $TEMPFILE
  #  Saves last 100 lines of system log file as temp file.
  #  Necessary, since newer kernels generate many log messages at log on.
  search=`grep $KEYWORD $TEMPFILE`
  #  Checks for presence of the "IP address" phrase,
  #+ indicating a successful logon.

  if [ ! -z "$search" ] #  Quotes necessary because of possible spaces.
  then
     echo "On-line"
     rm -f $TEMPFILE    #  Clean up temp file.
     exit $ONLINE
  else
     echo -n "."        #  The -n option to echo suppresses newline,
                        #+ so you get continuous rows of dots.
  fi

  sleep 1  
done  


#  Note: if you change the KEYWORD variable to "Exit",
#+ this script can be used while on-line
#+ to check for an unexpected logoff.

# Exercise: Change the script, per the above note,
#           and prettify it.

exit 0


# Nick Drage suggests an alternate method:

while true
  do ifconfig ppp0 | grep UP 1> /dev/null && echo "connected" && exit 0
  echo -n "."   # Prints dots (.....) until connected.
  sleep 2
done

# Problem: Hitting Control-C to terminate this process may be insufficient.
#+         (Dots may keep on echoing.)
# Exercise: Fix this.



# Stephane Chazelas has yet another alternative:

CHECK_INTERVAL=1

while ! tail -n 1 "$LOGFILE" | grep -q "$KEYWORD"
do echo -n .
   sleep $CHECK_INTERVAL
done
echo "On-line"

# Exercise: Discuss the relative strengths and weaknesses
#           of each of these various approaches.
```

The cleanup goes in a `finally`, which runs however the loop ends: by
`exit`, by Control-C (a `KeyboardInterrupt`), or by `kill` once a `SIGTERM`
handler turns that into an `exit` too. So the "On-line" branch doesn't
need its own `rm`. To try it, the script watches a made-up log, and the
logon "happens" on its third look.

```tessh
#!/usr/bin/env tessh
# logon: a quick 'n dirty script to check whether you are on-line yet.
import os signal sys

os.umask 0o177   # Make sure temp files are not world readable.

# (A made-up system log to try it on.)
print "kernel: eth0 up\nchat: dialing" >messages

LOGFILE = ./messages          # /var/log/messages in the original.
TEMPFILE = "temp.$(os.getpid)"
#  A "unique" temp file name, using the process id of the script.
#  (tempfile.mkstemp is better.)
KEYWORD = address
#  At logon, the line "remote IP address xxx.xxx.xxx.xxx"
#  is appended to the log.
ONLINE = 22
USER_INTERRUPT = 13
CHECK_LINES = 100             #  How many lines in log file to check.

# kill sends TERM: make it an exit, so the finally below cleans up.
signal.signal $signal.SIGTERM (fn [signum frame] (sys.exit $USER_INTERRUPT))

print
looks = 0
try
  while True                  # Endless loop.
    tail -n $CHECK_LINES $LOGFILE >$TEMPFILE
    #  Saves last 100 lines of system log file as temp file.
    search = grep $KEYWORD $TEMPFILE
    #  Checks for the "IP address" phrase: a successful logon.
    if $search
      print "On-line"
      exit $ONLINE
    else
      print "." end=""        # A continuous row of dots.
    looks += 1
    if $looks == 3            # (The logon, for the demo.)
      print "pppd: remote IP address 192.0.2.1" >>$LOGFILE
    sleep 0.1                 # (1 second in the original.)
except KeyboardInterrupt      # Control-C.
  exit $USER_INTERRUPT
finally
  rm -f $TEMPFILE             # Clean up temp file, whatever happened.
```

```output

...On-line
```

Stephane Chazelas's shorter loop, with no temp file, carries straight over:

```tessh no-run
CHECK_INTERVAL = 1
while not (tail -n 1 $LOGFILE | grep -q $KEYWORD)
  print . end=""
  sleep $CHECK_INTERVAL
print "On-line"
```

### Example 32-7. A Simple Implementation of a Progress Bar (`progress-bar2.sh`)

```bash no-run
#! /bin/bash
# progress-bar2.sh
# Author: Graham Ewart (with reformatting by ABS Guide author).
# Used in ABS Guide with permission (thanks!).

# Invoke this script with bash. It doesn't work with sh.

interval=1
long_interval=10

{
     trap "exit" SIGUSR1
     sleep $interval; sleep $interval
     while true
     do
       echo -n '.'     # Use dots.
       sleep $interval
     done; } &         # Start a progress bar as a background process.

pid=$!
trap "echo !; kill -USR1 $pid; wait $pid"  EXIT        # To handle ^C.

echo -n 'Long-running process '
sleep $long_interval
echo ' Finished!'

kill -USR1 $pid
wait $pid              # Stop the progress bar.
trap EXIT

exit $?
```

(Its dots depend on timing, so neither version's output can be checked.)
Bash runs the bar as a background subshell and stops it with a signal. In
tessh it's a thread, stopped with a `threading.Event`: `$stop.wait n`
sleeps up to n seconds and says whether it was set meanwhile. `finally`
stops the bar however the work ends, Control-C included.

```tessh
#!/usr/bin/env tessh
# progress-bar2, after Graham Ewart's.
import threading

interval = 0.2           # (1 in the original)
long_interval = 2        # (10 in the original)
stop = (threading.Event)

def progress_bar
  if $stop.wait ($interval * 2)
    return
  while not ($stop.wait $interval)
    print '.' end='' flush=True      # Use dots.

bar = threading.Thread target=$progress_bar
$bar.start               # Start a progress bar alongside.

try
  print 'Long-running process ' end='' flush=True
  sleep $long_interval
  print ' Finished!'
except KeyboardInterrupt # To handle ^C.
  print !
finally
  $stop.set              # Stop the progress bar.
  $bar.join
```

```text
(one run)
Long-running process ........ Finished!
```

**The DEBUG trap.** `trap '…' DEBUG` runs a command before every command
in the script, which can trace a variable. `sys.settrace` is tessh's way,
as with `-x` above.

### Example 32-8. Tracing a variable (`vartrace.sh`)

```bash
#!/bin/bash

trap 'echo "VARIABLE-TRACE> \$variable = \"$variable\""' DEBUG
# Echoes the value of $variable after every command.

variable=29; line=$LINENO

echo "  Just initialized \$variable to $variable in line number $line."

let "variable *= 3"; line=$LINENO
echo "  Just multiplied \$variable by 3 in line number $line."

exit 0

#  The "trap 'command1 . . . command2 . . .' DEBUG" construct is
#+ more appropriate in the context of a complex script,
#+ where inserting multiple "echo $variable" statements might be
#+ awkward and time-consuming.

# Thanks, Stephane Chazelas for the pointer.


Output of script:

VARIABLE-TRACE> $variable = ""
VARIABLE-TRACE> $variable = "29"
  Just initialized $variable to 29.
VARIABLE-TRACE> $variable = "29"
VARIABLE-TRACE> $variable = "87"
  Just multiplied $variable by 3.
VARIABLE-TRACE> $variable = "87"
```

```output bash
VARIABLE-TRACE> $variable = ""
VARIABLE-TRACE> $variable = "29"
VARIABLE-TRACE> $variable = "29"
  Just initialized $variable to 29 in line number 6.
VARIABLE-TRACE> $variable = "29"
VARIABLE-TRACE> $variable = "87"
VARIABLE-TRACE> $variable = "87"
  Just multiplied $variable by 3 in line number 10.
VARIABLE-TRACE> $variable = "87"
```

The trace function runs before each line of the script (a line, where
bash's trap runs per command), and the line number comes from the frame
rather than from `$LINENO`. Python, like bash, stops twice on a line with
two statements.

```tessh
#!/usr/bin/env tessh
import sys

def vartrace frame event arg
  if $event == "line" and $frame.f_code.co_filename == $__file__
    print "VARIABLE-TRACE> \$variable = \"$((globals).get 'variable' '')\""
  $vartrace
sys.settrace $vartrace
(sys._getframe).f_trace = $vartrace
# Echoes the value of $variable before every line.

variable = 29 ; line = (sys._getframe).f_lineno

print "  Just initialized \$variable to $variable in line number $line."

variable *= 3 ; line = (sys._getframe).f_lineno
print "  Just multiplied \$variable by 3 in line number $line."
```

```output
VARIABLE-TRACE> $variable = ""
VARIABLE-TRACE> $variable = ""
VARIABLE-TRACE> $variable = "29"
VARIABLE-TRACE> $variable = "29"
  Just initialized $variable to 29 in line number 12.
VARIABLE-TRACE> $variable = "29"
VARIABLE-TRACE> $variable = "87"
VARIABLE-TRACE> $variable = "87"
  Just multiplied $variable by 3 in line number 16.
```

Besides debugging, `trap` has other uses, such as disabling keys in a
script, or coordinating processes:

### Example 32-9. Running multiple processes (on an SMP box) (`multiple-processes.sh`)

```bash no-run
#!/bin/bash
# parent.sh
# Running multiple processes on an SMP box.
# Author: Tedman Eng

#  This is the first of two scripts,
#+ both of which must be present in the current working directory.




LIMIT=$1         # Total number of process to start
NUMPROC=4        # Number of concurrent threads (forks?)
PROCID=1         # Starting Process ID
echo "My PID is $$"

function start_thread() {
        if [ $PROCID -le $LIMIT ] ; then
                ./child.sh $PROCID&
                let "PROCID++"
        else
           echo "Limit reached."
           wait
           exit
        fi
}

while [ "$NUMPROC" -gt 0 ]; do
        start_thread;
        let "NUMPROC--"
done


while true
do

trap "start_thread" SIGRTMIN

done

exit 0



# ======== Second script follows ========


#!/bin/bash
# child.sh
# Running multiple processes on an SMP box.
# This script is called by parent.sh.
# Author: Tedman Eng

temp=$RANDOM
index=$1
shift
let "temp %= 5"
let "temp += 4"
echo "Starting $index  Time:$temp" "$@"
sleep ${temp}
echo "Ending $index"
kill -s SIGRTMIN $PPID

exit 0


# ======================= SCRIPT AUTHOR'S NOTES ======================= #
#  It's not completely bug free.
#  I ran it with limit = 500 and after the first few hundred iterations,
#+ one of the concurrent threads disappeared!
#  Not sure if this is collisions from trap signals or something else.
#  Once the trap is received, there's a brief moment while executing the
#+ trap handler but before the next trap is set.  During this time, it may
#+ be possible to miss a trap signal, thus miss spawning a child process.

#  No doubt someone may spot the bug and will be writing 
#+ . . . in the future.



# ===================================================================== #



# ----------------------------------------------------------------------#



#################################################################
# The following is the original script written by Vernia Damiano.
# Unfortunately, it doesn't work properly.
#################################################################

#!/bin/bash

#  Must call script with at least one integer parameter
#+ (number of concurrent processes).
#  All other parameters are passed through to the processes started.


INDICE=8        # Total number of process to start
TEMPO=5         # Maximum sleep time per process
E_BADARGS=65    # No arg(s) passed to script.

if [ $# -eq 0 ] # Check for at least one argument passed to script.
then
  echo "Usage: `basename $0` number_of_processes [passed params]"
  exit $E_BADARGS
fi

NUMPROC=$1              # Number of concurrent process
shift
PARAMETRI=( "$@" )      # Parameters of each process

function avvia() {
         local temp
         local index
         temp=$RANDOM
         index=$1
         shift
         let "temp %= $TEMPO"
         let "temp += 1"
         echo "Starting $index Time:$temp" "$@"
         sleep ${temp}
         echo "Ending $index"
         kill -s SIGRTMIN $$
}

function parti() {
         if [ $INDICE -gt 0 ] ; then
              avvia $INDICE "${PARAMETRI[@]}" &
                let "INDICE--"
         else
                trap : SIGRTMIN
         fi
}

trap parti SIGRTMIN

while [ "$NUMPROC" -gt 0 ]; do
         parti;
         let "NUMPROC--"
done

wait
trap - SIGRTMIN

exit $?

: <<SCRIPT_AUTHOR_COMMENTS
I had the need to run a program, with specified options, on a number of
different files, using a SMP machine. So I thought [I'd] keep running
a specified number of processes and start a new one each time . . . one
of these terminates.

The "wait" instruction does not help, since it waits for a given process
or *all* process started in background. So I wrote [this] bash script
that can do the job, using the "trap" instruction.
  --Vernia Damiano
SCRIPT_AUTHOR_COMMENTS
```

Both scripts keep N children running by having each finished child signal
the parent, and both lose signals now and then, as their authors say.
Python has the whole job ready-made and without the race:
`concurrent.futures.ThreadPoolExecutor max_workers=N` runs the jobs given
to it at most N at a time, starting the next as one finishes, and the
`with` block waits for all of them. Each job here runs its `sleep` as a
real process (the "child"), so they do run side by side on several CPUs;
for Python work that needs several CPUs, `ProcessPoolExecutor` is the
same with processes. The parameters after the first are passed on to each
job, as in Vernia Damiano's version. The order of the lines depends on
timing.

```tessh
#!/usr/bin/env tessh
# parent: running multiple processes on an SMP box,
# after Tedman Eng's and Vernia Damiano's scripts.
from concurrent.futures import ThreadPoolExecutor
import random

args = $argv[1:] or [6 --fast]    # (sample arguments, when none are given)
LIMIT = int $args[0]              # Total number of processes to start
PARAMS = $args[1:]                # Passed on to each of them
NUMPROC = 4                       # Number running at once

def child index *params           # (child.sh in the original)
  temp = (random.randrange 5) + 4
  print "Starting $index  Time:$temp" *$params
  sleep ($temp / 10)              # (Tenths of the original's seconds.)
  print "Ending $index"

with (ThreadPoolExecutor max_workers=$NUMPROC) as pool
  for PROCID in (range 1 ($LIMIT + 1))
    $pool.submit $child $PROCID *$PARAMS
  print "Limit reached."
# (Leaving the with block waits for them all.)
```

```text
(one run)
Starting 1  Time:5 --fast
Starting 3  Time:8 --fast
Starting 4  Time:5 --fast
Limit reached.
Starting 2  Time:8 --fast
Ending 1
Starting 5  Time:6 --fast
Ending 4
Starting 6  Time:8 --fast
Ending 2
Ending 3
Ending 5
Ending 6
```

`trap '' SIGNAL` turns a signal off for the rest of the script and `trap
SIGNAL` turns it back on, to keep a critical part from being interrupted:

```bash no-run
trap '' 2  # Signal 2 is Control-C, now disabled.
command
command
command
trap 2     # Reenables Control-C
```

`signal.signal` gives back the handler it replaces, so putting it back is
one more call:

```tessh
import signal
old = signal.signal $signal.SIGINT $signal.SIG_IGN   # Control-C, now disabled.
echo command
echo command
signal.signal $signal.SIGINT $old                     # Reenables Control-C.
```

```output
command
command
```

**Bash's debugging variables.** Bash 3 added variables for its debugger.
Their tessh counterparts:

| bash | tessh |
| --- | --- |
| `$BASH_ARGC`, the number of arguments | `(len $argv) - 1` |
| `$BASH_ARGV`, the last argument | `$argv[-1]` |
| `$BASH_COMMAND`, the command running | none (a trace function sees the line) |
| `$BASH_EXECUTION_STRING`, the `-c` string | none |
| `$BASH_LINENO`, the caller's line | `(inspect.stack)[1].lineno` |
| `$BASH_REMATCH`, what `=~` matched | the match `=~ re'…'` gives back, `.group n` |
| `$BASH_SOURCE`, the script's name | `$__file__` |
| `$BASH_SUBSHELL`, the subshell level | none (tessh has no subshells) |
