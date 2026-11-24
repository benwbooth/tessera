# 9. Another Look at Variables
<!-- help: abs-variables2 abs-9 internal-variables special-variables declare random -->

Bash has dozens of variables of its own, `declare` to give a variable
properties, and `$RANDOM`. tessh has almost none of the variables: what
they tell you comes from Python's modules (`os`, `sys`, `time`,
`platform`, `inspect`), which name it plainly. Its variables are Python's,
typed by their values, and its random numbers come from `random`.

## 9.1 Internal Variables
<!-- help: abs-internal-variables builtin-variables $BASH $UID $IFS $PPID $SECONDS $LINENO $PIPESTATUS $REPLY $$ $! $_ -->

What bash keeps in a variable, tessh mostly gets from a call. The ones a
tessh script has are `$?` (the last command's status), `$argv` (the
script's arguments, `$argv[0]` its name) and `$env` (the environment,
where `$HOME`, `$PATH`, `$EDITOR`, `$USER`, `$SHELL`, `$TERM` and the
other exported variables are). The rest:

| bash | tessh |
|---|---|
| `$BASH` | none for tessh itself; `sys.executable` is the Python under it |
| `$BASH_VERSION`, `$BASH_VERSINFO` | none for tessh; `sys.version_info` is Python's |
| `$BASH_ENV` | none; `~/.config/tessh/rc.tsh` is for the REPL |
| `$BASHPID`, `$$` | `os.getpid` |
| `$PPID` | `os.getppid` |
| `$BASH_SUBSHELL`, `$SHLVL` | `$env.SHLVL`, one more than the shell that started tessh; no `$BASH_SUBSHELL` (tessh has no subshells) |
| `$CDPATH` | `$CDPATH`, which `cd` searches, as in bash |
| `$DIRSTACK` | `dirs` |
| `$OLDPWD` | `$OLDPWD` in the environment, which `cd` sets; `cd -` goes there |
| `$PWD` | `(pwd)` or `os.getcwd` |
| `$UID`, `$EUID` | `os.getuid`, `os.geteuid` |
| `$GROUPS` | `os.getgroups` |
| `$FUNCNAME` | `(inspect.currentframe).f_code.co_name` |
| `$LINENO` | `(inspect.currentframe).f_lineno` |
| `$HOSTNAME` | `socket.gethostname` |
| `$HOSTTYPE`, `$MACHTYPE` | `platform.machine` |
| `$OSTYPE` | `sys.platform` |
| `$IFS` | `$IFS`, where `read` splits a line; tessh doesn't split words otherwise (`str.split` does, where you say) |
| `$GLOBIGNORE` | none: filter a Glob's names yourself |
| `$PIPESTATUS` | `$PIPESTATUS`: a list of the last pipe's programs' statuses |
| `$REPLY` | `$REPLY`, set by `read` with no name; `input` returns the line |
| `$SECONDS` | `time.monotonic`, less the time it was at the start |
| `$TMOUT` | none: `read -t secs`, or `select` or `signal.alarm` (examples 9-2 and 9-4) |
| `$PS3` | `select`'s prompt, as in bash |
| `$PS4` | `$PS4`, shown before each command `set -x` traces (`+ ` if unset) |
| `$PS1`, `$PS2`, `$PROMPT_COMMAND`, `$IGNOREEOF`, `$SHELLOPTS`, `$-` | none (`@before_prompt` is `$PROMPT_COMMAND`) |
| `$0`, `$1`…, `$#` | `$argv[0]`, `$argv[1]`…, `(len $argv) - 1` |
| `$*`, `$@` | `" ".join $argv[1:]`, `$argv[1:]` |
| `$!` | `$j.pid`, after `j = cmd &`; `subprocess.Popen` has `.pid` too |
| `$_` | none |
| `$RANDOM` | `random.randrange 32768` (section 9.3) |

**Gap:** none of these is a tessh variable, so a script ported from bash
has to import and call. For most that's a plus (`os.getuid` says what it
is, and can't be set by mistake), but `$LINENO`, `$FUNCNAME`, `$SECONDS`
and `$_` have no short form, and `$-` has no equivalent at all.
`$PIPESTATUS` and `$PS4` (with `set -x`) are there as in bash, and so is
`set -o pipefail`.

A subshell has its own PID in bash, which `$BASHPID` shows and `$$` hides.
tessh has no subshells (a block in parentheses is a value, not a process),
so `os.getpid` is the same everywhere in a script.

`$FUNCNAME` is the name of the function running. Python keeps it in the
frame, with the line number (bash's `$LINENO`):

```bash
xyz23 ()
{
  echo "$FUNCNAME now executing."  # xyz23 now executing.
}

xyz23

echo "FUNCNAME = $FUNCNAME"        # FUNCNAME =
                                   # Null value outside a function.
```

```output bash
xyz23 now executing.
FUNCNAME = 
```

```tessh
import inspect
def xyz23
  print "$((inspect.currentframe).f_code.co_name) now executing."

xyz23

print "FUNCNAME =" (inspect.currentframe).f_code.co_name
                                   # <module>: the script's own code.
print "At line number $((inspect.currentframe).f_lineno)."
```

```output
xyz23 now executing.
FUNCNAME = <module>
At line number 9.
```

`$IFS`, the internal field separator, is what bash splits unquoted
`$values` on: space, tab and newline unless you change it. tessh never
splits a value into words behind your back. A string is one argument; to
get several, split it, with `str.split` (on runs of whitespace) or
`str.split sep` (on each `sep`).

### Example 9-1. $IFS and whitespace (`ifs.sh`)

```bash
#!/bin/bash
# ifs.sh


var1="a+b+c"
var2="d-e-f"
var3="g,h,i"

IFS=+
# The plus sign will be interpreted as a separator.
echo $var1     # a b c
echo $var2     # d-e-f
echo $var3     # g,h,i

echo

IFS="-"
# The plus sign reverts to default interpretation.
# The minus sign will be interpreted as a separator.
echo $var1     # a+b+c
echo $var2     # d e f
echo $var3     # g,h,i

echo

IFS=","
# The comma will be interpreted as a separator.
# The minus sign reverts to default interpretation.
echo $var1     # a+b+c
echo $var2     # d-e-f
echo $var3     # g h i

echo

IFS=" "
# The space character will be interpreted as a separator.
# The comma reverts to default interpretation.
echo $var1     # a+b+c
echo $var2     # d-e-f
echo $var3     # g,h,i

# ======================================================== #

# However ...
# $IFS treats whitespace differently than other characters.

output_args_one_per_line()
{
  for arg
  do
    echo "[$arg]"
  done #  ^    ^   Embed within brackets, for your viewing pleasure.
}

echo; echo "IFS=\" \""
echo "-------"

IFS=" "
var=" a  b c   "
#    ^ ^^   ^^^
output_args_one_per_line $var  # output_args_one_per_line `echo " a  b c   "`
# [a]
# [b]
# [c]


echo; echo "IFS=:"
echo "-----"

IFS=:
var=":a::b:c:::"               # Same pattern as above,
#    ^ ^^   ^^^                #+ but substituting ":" for " "  ...
output_args_one_per_line $var
# []
# [a]
# []
# [b]
# [c]
# []
# []

# Note "empty" brackets.
# The same thing happens with the "FS" field separator in awk.


echo

exit
```

```output bash
a b c
d-e-f
g,h,i

a+b+c
d e f
g,h,i

a+b+c
d-e-f
g h i

a+b+c
d-e-f
g,h,i

IFS=" "
-------
[a]
[b]
[c]

IFS=:
-----
[]
[a]
[]
[b]
[c]
[]
[]

```

In tessh the separator goes to `split`, and the four rounds are a loop. A
list given to a command is one argument per item. `str.split` with no
argument works as bash's whitespace does: runs count as one, and nothing
comes of the ends. With a separator every one counts, the last one too, so
Python finds eight fields in `:a::b:c:::` where bash finds seven.

```tessh
#!/usr/bin/env tessh
var1 = "a+b+c"
var2 = "d-e-f"
var3 = "g,h,i"

for sep in ["+" "-" "," " "]
  # Split on $sep; the others are only text.
  echo ($var1.split $sep)
  echo ($var2.split $sep)
  echo ($var3.split $sep)
  echo

# However ...
# whitespace is split differently than other characters.

def output_args_one_per_line *args
  for arg in $args
    echo "[$arg]"       # Embed within brackets, for your viewing pleasure.

print 'IFS=" "'
print "-------"

var = " a  b c   "
output_args_one_per_line *($var.split)
# [a]
# [b]
# [c]

print ; print "IFS=:"
print "-----"

var = ":a::b:c:::"
output_args_one_per_line *($var.split ":")
# Note "empty" brackets: one more than bash, after the last ":".

print
```

```output
a b c
d-e-f
g,h,i

a+b+c
d e f
g,h,i

a+b+c
d-e-f
g h i

a+b+c
d-e-f
g,h,i

IFS=" "
-------
[a]
[b]
[c]

IFS=:
-----
[]
[a]
[]
[b]
[c]
[]
[]
[]

```

### Timed input

`$TMOUT` makes bash's prompt, and a `read` in a script, give up after that
many seconds. tessh has no `$TMOUT`, but its `read -t secs` gives up
after that long (status 142); Python's `input` waits as long as it takes.
The ABS Guide's three ways around that follow, the Python way. Here, where no one
types, a program or a pipe stands in for the user.

### Example 9-2. Timed Input (`timed-input.sh`)

```bash no-run
#!/bin/bash
# timed-input.sh

# TMOUT=3    Also works, as of newer versions of Bash.

TIMER_INTERRUPT=14
TIMELIMIT=3  # Three seconds in this instance.
             # May be set to different value.

PrintAnswer()
{
  if [ "$answer" = TIMEOUT ]
  then
    echo $answer
  else       # Don't want to mix up the two instances. 
    echo "Your favorite veggie is $answer"
    kill $!  #  Kills no-longer-needed TimerOn function
             #+ running in background.
             #  $! is PID of last job running in background.
  fi

}  


TimerOn()
{
  sleep $TIMELIMIT && kill -s 14 $$ &
  # Waits 3 seconds, then sends sigalarm to script.
}  


Int14Vector()
{
  answer="TIMEOUT"
  PrintAnswer
  exit $TIMER_INTERRUPT
}  

trap Int14Vector $TIMER_INTERRUPT
# Timer interrupt (14) subverted for our purposes.

echo "What is your favorite vegetable "
TimerOn
read answer
PrintAnswer


#  Admittedly, this is a kludgy implementation of timed input.
#  However, the "-t" option to "read" simplifies this task.
#  See the "t-out.sh" script.
#  However, what about timing not just single user input,
#+ but an entire script?

#  If you need something really elegant ...
#+ consider writing the application in C or C++,
#+ using appropriate library functions, such as 'alarm' and 'setitimer.'

exit 0
```

The "really elegant" way the comment wishes for is at hand in tessh:
Python's `signal.alarm` is the C library's `alarm`, and `signal.signal`
does what bash's `trap` does (tessh's `trap` would do here too). No
background timer, so no `$!` to kill: `signal.alarm 0`
turns the alarm off.

```tessh
#!/usr/bin/env tessh
import signal subprocess

TIMER_INTERRUPT = 14
TIMELIMIT = 1      # One second here (three in the original).
answer = None

# (The user, played by a program that answers in two seconds: too late.)
user = subprocess.Popen [sh -c "sleep 2; echo carrot"] stdout=$subprocess.PIPE text=True

def PrintAnswer
  if $answer == TIMEOUT
    print $answer
  else           # Don't want to mix up the two instances.
    print "Your favorite veggie is $answer"
    signal.alarm 0   # The alarm isn't needed any more.

def Int14Vector signum frame
  global answer
  answer = TIMEOUT
  PrintAnswer
  exit $TIMER_INTERRUPT

signal.signal $signal.SIGALRM $Int14Vector
# Timer interrupt (14) subverted for our purposes.

print "What is your favorite vegetable "
signal.alarm $TIMELIMIT                  # (TimerOn)
answer = ($user.stdout.readline).strip   # (read answer)
PrintAnswer
```

```output
What is your favorite vegetable 
TIMEOUT
```

### Example 9-3. Once more, timed input (`timeout.sh`)

```bash no-run
#!/bin/bash
# timeout.sh

#  Written by Stephane Chazelas,
#+ and modified by the document author.

INTERVAL=5                # timeout interval

timedout_read() {
  timeout=$1
  varname=$2
  old_tty_settings=`stty -g`
  stty -icanon min 0 time ${timeout}0
  eval read $varname      # or just  read $varname
  stty "$old_tty_settings"
  # See man page for "stty."
}

echo; echo -n "What's your name? Quick! "
timedout_read $INTERVAL your_name

#  This may not work on every terminal type.
#  The maximum timeout depends on the terminal.
#+ (it is often 25.5 seconds).

echo

if [ ! -z "$your_name" ]  # If name input before timeout ...
then
  echo "Your name is $your_name."
else
  echo "Timed out."
fi

echo

# The behavior of this script differs somewhat from "timed-input.sh."
# At each keystroke, the counter resets.

exit 0
```

`stty` sets the terminal; Python's `termios` does it from inside the
script: no canonical mode, `VMIN` 0 and `VTIME` in tenths of a second.
The function returns the name instead of setting a variable named by its
argument, so there's no `eval`. It needs a terminal to type at, so it
doesn't run here.

```tessh no-run
#!/usr/bin/env tessh
import os sys termios operator

INTERVAL = 5                # timeout interval

def timedout_read timeout
  fd = (sys.stdin.fileno)
  old_tty_settings = termios.tcgetattr $fd      # stty -g
  new = termios.tcgetattr $fd
  $new[3] &= operator.invert $termios.ICANON    # stty -icanon
  $new[6][$termios.VMIN] = 0                    #   min 0
  $new[6][$termios.VTIME] = $timeout * 10       #   time ${timeout}0
  termios.tcsetattr $fd $termios.TCSANOW $new
  try
    line = ""
    # Each key read waits up to $timeout seconds; nothing in time ends it.
    while (c := (os.read $fd 1).decode) not in ["" "\n"]
      line += $c
    $line
  finally
    termios.tcsetattr $fd $termios.TCSANOW $old_tty_settings

print ; print "What's your name? Quick! " end="" flush=True
your_name = timedout_read $INTERVAL

print

if $your_name       # If name input before timeout ...
  print "Your name is $your_name."
else
  print "Timed out."

print
```

### Example 9-4. Timed read (`t-out.sh`)

```bash no-run
#!/bin/bash
# t-out.sh [time-out]
# Inspired by a suggestion from "syngin seven" (thanks).


TIMELIMIT=4         # 4 seconds

read -t $TIMELIMIT variable <&1
#                           ^^^
#  In this instance, "<&1" is needed for Bash 1.x and 2.x,
#  but unnecessary for Bash 3+.

echo

if [ -z "$variable" ]  # Is null?
then
  echo "Timed out, variable still unset."
else  
  echo "variable = $variable"
fi  

exit 0
```

`read -t` is a wait on the input with a time limit. tessh's `read` has
`-t` too (`read -t 4 variable`); here it's done the Python way, with
`select.select`. `read_t` returns `None` when nothing came in time. A pipe no one
writes to plays the user who types nothing; for a real one, give it `0`,
the standard input.

```tessh
#!/usr/bin/env tessh
import os select

TIMELIMIT = 1         # 1 second (4 in the original)

def read_t seconds fd
  # A line from fd, or None if none comes within the time.
  [ready _ _] = select.select [$fd] [] [] $seconds
  if $ready then (-> (os.read $fd 1024) .decode .strip) else None

# (A pipe nobody writes to, standing in for the keyboard.)
[keys typist] = (os.pipe)
variable = read_t $TIMELIMIT $keys

print

if not $variable  # Is null?
  print "Timed out, variable still unset."
else
  print "variable = $variable"
```

```output

Timed out, variable still unset.
```

`$UID` is the user's ID number, 0 for root. In tessh it's `os.getuid`.

### Example 9-5. Am I root? (`am-i-root.sh`)

```bash
#!/bin/bash
# am-i-root.sh:   Am I root or not?

ROOT_UID=0   # Root has $UID 0.

if [ "$UID" -eq "$ROOT_UID" ]  # Will the real "root" please stand up?
then
  echo "You are root."
else
  echo "You are just an ordinary user (but mom loves you just the same)."
fi

exit 0


# ============================================================= #
# Code below will not execute, because the script already exited.

# An alternate method of getting to the root of matters:

ROOTUSER_NAME=root

username=`id -nu`              # Or...   username=`whoami`
if [ "$username" = "$ROOTUSER_NAME" ]
then
  echo "Rooty, toot, toot. You are root."
else
  echo "You are just a regular fella."
fi
```

```tessh
#!/usr/bin/env tessh
import os pwd

ROOT_UID = 0   # Root has UID 0.

if (os.getuid) == $ROOT_UID   # Will the real "root" please stand up?
  print "You are root."
else
  print "You are just an ordinary user (but mom loves you just the same)."

exit 0


# ============================================================= #
# Code below will not execute, because the script already exited.

# An alternate method of getting to the root of matters:

ROOTUSER_NAME = root

username = (pwd.getpwuid (os.getuid)).pw_name   # Or... username = (id -nu)
if $username == $ROOTUSER_NAME
  print "Rooty, toot, toot. You are root."
else
  print "You are just a regular fella."
```

```output
You are just an ordinary user (but mom loves you just the same).
```

### Positional parameters

`$1`, `$2`… are the script's arguments, `$#` how many, and `$*` and `$@`
all of them. tessh has them as a list, `$argv`, with the script's name
first: `$argv[1]`, `(len $argv) - 1`, and `$argv[1:]` for the lot. A list
needs no quoting to keep its items apart, and joining them into one string
is a separate, visible step: `" ".join $argv[1:]`.

### Example 9-6. arglist: Listing arguments with $* and $@ (`arglist.sh`)

```bash no-run
#!/bin/bash
# arglist.sh
# Invoke this script with several arguments, such as "one two three" ...

E_BADARGS=85

if [ ! -n "$1" ]
then
  echo "Usage: `basename $0` argument1 argument2 etc."
  exit $E_BADARGS
fi  

echo

index=1          # Initialize count.

echo "Listing args with \"\$*\":"
for arg in "$*"  # Doesn't work properly if "$*" isn't quoted.
do
  echo "Arg #$index = $arg"
  let "index+=1"
done             # $* sees all arguments as single word. 
echo "Entire arg list seen as single word."

echo

index=1          # Reset count.
                 # What happens if you forget to do this?

echo "Listing args with \"\$@\":"
for arg in "$@"
do
  echo "Arg #$index = $arg"
  let "index+=1"
done             # $@ sees arguments as separate words. 
echo "Arg list seen as separate words."

echo

index=1          # Reset count.

echo "Listing args with \$* (unquoted):"
for arg in $*
do
  echo "Arg #$index = $arg"
  let "index+=1"
done             # Unquoted $* sees arguments as separate words. 
echo "Arg list seen as separate words."

exit 0
```

`enumerate` counts, so there's no index to reset. Bash's unquoted `$*`
splits arguments that have spaces in them; the tessh version says so,
joining and splitting again.

```tessh
#!/usr/bin/env tessh
E_BADARGS = 85

args = $argv[1:] or [one "two three" four]   # (sample arguments, when none are given)

if not $args
  print "Usage: $(Path $argv[0]).name argument1 argument2 etc."
  exit $E_BADARGS

print

print "Listing args joined into one string (bash's \"\$*\"):"
for [index arg] in (enumerate [(" ".join $args)] 1)
  print "Arg #$index = $arg"
print "Entire arg list seen as single word."

print

print "Listing args as a list (bash's \"\$@\"):"
for [index arg] in (enumerate $args 1)
  print "Arg #$index = $arg"
print "Arg list seen as separate words."

print

print "Listing args joined and split at spaces (bash's \$* unquoted):"
for [index arg] in (enumerate ((" ".join $args).split) 1)
  print "Arg #$index = $arg"
print "Arg list seen as separate words."

exit 0
```

```output

Listing args joined into one string (bash's "$*"):
Arg #1 = one two three four
Entire arg list seen as single word.

Listing args as a list (bash's "$@"):
Arg #1 = one
Arg #2 = two three
Arg #3 = four
Arg list seen as separate words.

Listing args joined and split at spaces (bash's $* unquoted):
Arg #1 = one
Arg #2 = two
Arg #3 = three
Arg #4 = four
Arg list seen as separate words.
```

After bash's `shift`, `$@` is the arguments but the first. In tessh that's
a slice.

```bash no-run
#!/bin/bash
# Invoke with ./scriptname 1 2 3 4 5

echo "$@"    # 1 2 3 4 5
shift
echo "$@"    # 2 3 4 5
shift
echo "$@"    # 3 4 5

# Each "shift" loses parameter $1.
# "$@" then contains the remaining parameters.
```

```tessh
#!/usr/bin/env tessh
args = $argv[1:] or [1 2 3 4 5]   # (sample arguments, when none are given)

echo $args    # 1 2 3 4 5
args = $args[1:]
echo $args    # 2 3 4 5
args = $args[1:]
echo $args    # 3 4 5
```

```output
1 2 3 4 5
2 3 4 5
3 4 5
```

How `$*` and `$@` come out depends on whether they're quoted and on
`$IFS`, which the next example takes through every case. In tessh each
case is a join, a split, or neither, written out; `ifs_split` does bash's
splitting on a separator (no field after the last one).

### Example 9-7. Inconsistent $* and $@ behavior (`incompat.sh`)

```bash
#!/bin/bash

#  Erratic behavior of the "$*" and "$@" internal Bash variables,
#+ depending on whether or not they are quoted.
#  Demonstrates inconsistent handling of word splitting and linefeeds.


set -- "First one" "second" "third:one" "" "Fifth: :one"
# Setting the script arguments, $1, $2, $3, etc.

echo

echo 'IFS unchanged, using "$*"'
c=0
for i in "$*"               # quoted
do echo "$((c+=1)): [$i]"   # This line remains the same in every instance.
                            # Echo args.
done
echo ---

echo 'IFS unchanged, using $*'
c=0
for i in $*                 # unquoted
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS unchanged, using "$@"'
c=0
for i in "$@"
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS unchanged, using $@'
c=0
for i in $@
do echo "$((c+=1)): [$i]"
done
echo ---

IFS=:
echo 'IFS=":", using "$*"'
c=0
for i in "$*"
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS=":", using $*'
c=0
for i in $*
do echo "$((c+=1)): [$i]"
done
echo ---

var=$*
echo 'IFS=":", using "$var" (var=$*)'
c=0
for i in "$var"
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS=":", using $var (var=$*)'
c=0
for i in $var
do echo "$((c+=1)): [$i]"
done
echo ---

var="$*"
echo 'IFS=":", using $var (var="$*")'
c=0
for i in $var
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS=":", using "$var" (var="$*")'
c=0
for i in "$var"
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS=":", using "$@"'
c=0
for i in "$@"
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS=":", using $@'
c=0
for i in $@
do echo "$((c+=1)): [$i]"
done
echo ---

var=$@
echo 'IFS=":", using $var (var=$@)'
c=0
for i in $var
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS=":", using "$var" (var=$@)'
c=0
for i in "$var"
do echo "$((c+=1)): [$i]"
done
echo ---

var="$@"
echo 'IFS=":", using "$var" (var="$@")'
c=0
for i in "$var"
do echo "$((c+=1)): [$i]"
done
echo ---

echo 'IFS=":", using $var (var="$@")'
c=0
for i in $var
do echo "$((c+=1)): [$i]"
done

echo

# Try this script with ksh or zsh -y.

exit 0

#  This example script written by Stephane Chazelas,
#+ and slightly modified by the document author.
```

```tessh
#!/usr/bin/env tessh
args = ["First one" second "third:one" "" "Fifth: :one"]
# The script's arguments (bash's set -- …), as a list.

def show label items end="---"
  print $label
  for [c i] in (enumerate $items 1)
    print "$c: [$i]"        # This line remains the same in every instance.
  print $end

def ifs_split text sep=None
  # Bash's word splitting: at runs of whitespace, or at each sep
  # (and no empty field after a last one).
  if $sep is None
    return ($text.split)
  fields = $text.split $sep
  if $fields[-1] == "" then $fields[:-1] else $fields

print

show 'IFS unchanged, using "$*"' [(" ".join $args)]
show 'IFS unchanged, using $*' (ifs_split (" ".join $args))
show 'IFS unchanged, using "$@"' $args
show 'IFS unchanged, using $@' (ifs_split (" ".join $args))

# IFS=":"   ("$*" joins with its first character.)
show 'IFS=":", using "$*"' [(":".join $args)]
show 'IFS=":", using $*' (ifs_split (":".join $args) ":")

var = ":".join $args
show 'IFS=":", using "$var" (var=$*)' [$var]
show 'IFS=":", using $var (var=$*)' (ifs_split $var ":")
show 'IFS=":", using $var (var="$*")' (ifs_split $var ":")
show 'IFS=":", using "$var" (var="$*")' [$var]

show 'IFS=":", using "$@"' $args
show 'IFS=":", using $@' (ifs_split (":".join $args) ":")

var = " ".join $args     # (Assigning $@ joins with spaces, whatever IFS is.)
show 'IFS=":", using $var (var=$@)' (ifs_split $var ":")
show 'IFS=":", using "$var" (var=$@)' [$var]
show 'IFS=":", using "$var" (var="$@")' [$var]
show 'IFS=":", using $var (var="$@")' (ifs_split $var ":") end=""
```

```output

IFS unchanged, using "$*"
1: [First one second third:one  Fifth: :one]
---
IFS unchanged, using $*
1: [First]
2: [one]
3: [second]
4: [third:one]
5: [Fifth:]
6: [:one]
---
IFS unchanged, using "$@"
1: [First one]
2: [second]
3: [third:one]
4: []
5: [Fifth: :one]
---
IFS unchanged, using $@
1: [First]
2: [one]
3: [second]
4: [third:one]
5: [Fifth:]
6: [:one]
---
IFS=":", using "$*"
1: [First one:second:third:one::Fifth: :one]
---
IFS=":", using $*
1: [First one]
2: [second]
3: [third]
4: [one]
5: []
6: [Fifth]
7: [ ]
8: [one]
---
IFS=":", using "$var" (var=$*)
1: [First one:second:third:one::Fifth: :one]
---
IFS=":", using $var (var=$*)
1: [First one]
2: [second]
3: [third]
4: [one]
5: []
6: [Fifth]
7: [ ]
8: [one]
---
IFS=":", using $var (var="$*")
1: [First one]
2: [second]
3: [third]
4: [one]
5: []
6: [Fifth]
7: [ ]
8: [one]
---
IFS=":", using "$var" (var="$*")
1: [First one:second:third:one::Fifth: :one]
---
IFS=":", using "$@"
1: [First one]
2: [second]
3: [third:one]
4: []
5: [Fifth: :one]
---
IFS=":", using $@
1: [First one]
2: [second]
3: [third]
4: [one]
5: []
6: [Fifth]
7: [ ]
8: [one]
---
IFS=":", using $var (var=$@)
1: [First one second third]
2: [one  Fifth]
3: [ ]
4: [one]
---
IFS=":", using "$var" (var=$@)
1: [First one second third:one  Fifth: :one]
---
IFS=":", using "$var" (var="$@")
1: [First one second third:one  Fifth: :one]
---
IFS=":", using $var (var="$@")
1: [First one second third]
2: [one  Fifth]
3: [ ]
4: [one]

```

### Example 9-8. $* and $@ when $IFS is empty (`ifs-empty.sh`)

```bash
#!/bin/bash

#  If $IFS set, but empty,
#+ then "$*" and "$@" do not echo positional params as expected.

mecho ()       # Echo positional parameters.
{
echo "$1,$2,$3";
}


IFS=""         # Set, but empty.
set a b c      # Positional parameters.

mecho "$*"     # abc,,
#                   ^^
mecho $*       # a,b,c

mecho $@       # a,b,c
mecho "$@"     # a,b,c

#  The behavior of $* and $@ when $IFS is empty depends
#+ on which Bash or sh version being run.
#  It is therefore inadvisable to depend on this "feature" in a script.


# Thanks, Stephane Chazelas.

exit
```

Joining with the empty string is `"".join`; passing the list's items as
separate arguments is `*$args`. Neither depends on a version.

```tessh
#!/usr/bin/env tessh
def mecho a="" b="" c=""     # Echo the first three arguments.
  print "$a,$b,$c"

args = [a b c]               # (set a b c)

mecho ("".join $args)        # abc,,   (bash's "$*" with IFS="")
mecho *$args                 # a,b,c   (bash's $*)

mecho *$args                 # a,b,c   (bash's $@)
mecho *$args                 # a,b,c   (bash's "$@")
```

```output
abc,,
a,b,c
a,b,c
a,b,c
```

### Other special parameters

`$!` is the PID of the last job put in the background. tessh's
`j = cmd &` gives a Job (for `jobs`, `fg`, `wait` and `kill`), and
`$j.pid` is its process ID. `subprocess.Popen` starts a program in the
background too and has its `.pid`, and `.wait timeout=…` does the ABS
Guide's "kill the job if it hangs" trick without a timing loop:

```bash no-run
TIMEOUT=30   # Timeout value in seconds
count=0

possibly_hanging_job & {
        while ((count < TIMEOUT )); do
                eval '[ ! -d "/proc/$!" ] && ((count = TIMEOUT))'
                # /proc is where information about running processes is found.
                # "-d" tests whether it exists (whether directory exists).
                # So, we're waiting for the job in question to show up.
                ((count++))
                sleep 1
        done
        eval '[ -d "/proc/$!" ] && kill -15 $!'
        # If the hanging job is running, kill it.
}
```

```tessh
import subprocess

TIMEOUT = 1     # Timeout value in seconds (30 in the original)

job = subprocess.Popen [sleep "100"]   # (possibly_hanging_job; a list of strings)
print "started" (type $job.pid).__name__
try
  $job.wait timeout=$TIMEOUT
except subprocess.TimeoutExpired
  $job.terminate                       # If the hanging job is running, kill it.
  print "killed it:" ($job.wait)
```

```output
started int
killed it: -15
```

`$_` is the last argument of the command before. tessh doesn't keep it.

### Example 9-9. Underscore variable

```bash no-run
#!/bin/bash

echo $_              #  /bin/bash
                     #  Just called /bin/bash to run the script.
                     #  Note that this will vary according to
                     #+ how the script is invoked.

du >/dev/null        #  So no output from command.
echo $_              #  du

ls -al >/dev/null    #  So no output from command.
echo $_              #  -al  (last argument)

:
echo $_              #  :
```

**Gap:** there's no `$_`. A script that wants a command's last argument
has the command as a list, and takes it from there.

```tessh
#!/usr/bin/env tessh
print (Path $argv[0]).name   # (The script, where bash's first $_ is the shell.)

cmd = [du]
env $cmd >/dev/null          #  So no output from command.
print $cmd[-1]               #  du

cmd = [ls -al]
env $cmd >/dev/null          #  (env runs the program a list names.)
print $cmd[-1]               #  -al  (last argument)
```

```output
ex.tsh
du
-al
```

`$$` is the script's PID, often used to make a unique temporary file name;
in tessh it's `os.getpid`, and Python's `tempfile` makes one safely.

### $REPLY, $SECONDS and $PWD

`read` with no variable name puts the line in `$REPLY`, in tessh too
(`read -p "prompt"`). Python's `input "prompt"` returns the line instead,
which is what the example uses. Here the answers are canned, there being
no one to type them.

```bash no-run
#!/bin/bash
# reply.sh

# REPLY is the default value for a 'read' command.

echo
echo -n "What is your favorite vegetable? "
read

echo "Your favorite vegetable is $REPLY."
#  REPLY holds the value of last "read" if and only if
#+ no variable supplied.

echo
echo -n "What is your favorite fruit? "
read fruit
echo "Your favorite fruit is $fruit."
echo "but..."
echo "Value of \$REPLY is still $REPLY."
#  $REPLY is still set to its previous value because
#+ the variable $fruit absorbed the new "read" value.

echo

exit 0
```

```tessh
#!/usr/bin/env tessh
import io sys
# (The answers someone would type.)
$sys.stdin = io.StringIO "broccoli\nmango\n"

print
REPLY = input "What is your favorite vegetable? "
print                 # (The Enter the user would type.)

print "Your favorite vegetable is $REPLY."

print
fruit = input "What is your favorite fruit? "
print
print "Your favorite fruit is $fruit."
print "but..."
print "Value of REPLY is still $REPLY."
# Nothing sets a variable you didn't name.

print
```

```output

What is your favorite vegetable? 
Your favorite vegetable is broccoli.

What is your favorite fruit? 
Your favorite fruit is mango.
but...
Value of REPLY is still broccoli.

```

`$SECONDS` counts the seconds since the script started. Python's
`time.monotonic` is a clock that only goes forward; the difference from
its value at the start is the same count.

```bash no-run
#!/bin/bash

TIME_LIMIT=10
INTERVAL=1

echo
echo "Hit Control-C to exit before $TIME_LIMIT seconds."
echo

while [ "$SECONDS" -le "$TIME_LIMIT" ]
do   #   $SECONDS is an internal shell variable.
  if [ "$SECONDS" -eq 1 ]
  then
    units=second
  else  
    units=seconds
  fi

  echo "This script has been running $SECONDS $units."
  #  On a slow or overburdened machine, the script may skip a count
  #+ every once in a while.
  sleep $INTERVAL
done

echo -e "\a"  # Beep!

exit 0
```

```tessh
#!/usr/bin/env tessh
import time

TIME_LIMIT = 3      # (10 in the original)
INTERVAL = 1

start = (time.monotonic)
def SECONDS = int ((time.monotonic) - $start)

print
print "Hit Control-C to exit before $TIME_LIMIT seconds."
print

while (SECONDS) <= $TIME_LIMIT
  units = if (SECONDS) == 1 then second else seconds
  print "This script has been running $(SECONDS) $units."
  #  On a slow or overburdened machine, the script may skip a count
  #+ every once in a while.
  sleep $INTERVAL

print "\a"  # Beep!
```

```output

Hit Control-C to exit before 3 seconds.

This script has been running 0 seconds.
This script has been running 1 second.
This script has been running 2 seconds.
This script has been running 3 seconds.

```

`$PWD` is the folder you're in. The ABS Guide checks it after a `cd`
before deleting everything there. In tessh a `cd` that fails stops the
script (chapter 2), so the check is a second safety net; `(pwd)` is
always up to date, where `$PWD` is only what was exported.

```bash no-run
#!/bin/bash

E_WRONG_DIRECTORY=85

clear # Clear the screen.

TargetDirectory=/home/bozo/projects/GreatAmericanNovel

cd $TargetDirectory
echo "Deleting stale files in $TargetDirectory."

if [ "$PWD" != "$TargetDirectory" ]
then    # Keep from wiping out wrong directory by accident.
  echo "Wrong directory!"
  echo "In $PWD, rather than $TargetDirectory!"
  echo "Bailing out!"
  exit $E_WRONG_DIRECTORY
fi  

rm -rf *
rm .[A-Za-z0-9]*    # Delete dotfiles.
# rm -f .[^.]* ..?*   to remove filenames beginning with multiple dots.
# (shopt -s dotglob; rm -f *)   will also work.
# Thanks, S.C. for pointing this out.

#  A filename (`basename`) may contain all characters in the 0 - 255 range,
#+ except "/".
#  Deleting files beginning with weird characters, such as -
#+ is left as an exercise. (Hint: rm ./-weirdname or rm -- -weirdname)
result=$?   # Result of delete operations. If successful = 0.

echo
ls -al              # Any files left?
echo "Done."
echo "Old files deleted in $TargetDirectory."
echo

# Various other operations here, as necessary.

exit $result
```

```tessh
#!/usr/bin/env tessh
# (A made-up project folder, with a dotfile in it, to try it on.)
mkdir -p GreatAmericanNovel/drafts
touch GreatAmericanNovel/ch1 GreatAmericanNovel/ch2 GreatAmericanNovel/.notes

E_WRONG_DIRECTORY = 85

# clear      (Clear the screen: not here.)

TargetDirectory = "$(pwd)/GreatAmericanNovel"   # (~bozo/projects/… in the original)
name = (Path $TargetDirectory).name

cd $TargetDirectory
print "Deleting stale files in $name."

if (pwd) != $TargetDirectory    # Keep from wiping out wrong directory by accident.
  print "Wrong directory!"
  print "In $(pwd), rather than $TargetDirectory!"
  print "Bailing out!"
  exit $E_WRONG_DIRECTORY

rm -rf *
rm .[A-Za-z0-9]*    # Delete dotfiles.
result = $?         # Result of delete operations. If successful = 0.

print
ls -A               # Any files left?
print "Done."
print "Old files deleted in $name."
print

exit $result
```

```output
Deleting stale files in GreatAmericanNovel.

Done.
Old files deleted in GreatAmericanNovel.

```


## 9.2 Typing variables: declare or typeset
<!-- help: abs-declare declare typeset readonly integer-variables -->

Bash's `declare` (or `typeset`) gives a variable properties: `-i` an
integer, `-r` read-only, `-a` an array, `-x` exported, and `-f` lists
functions. tessh's variables are Python's: the value has the type, and
there's no need to declare one. What each option was for:

- `-i`: make the value an `int` (`int "23"`). Arithmetic on numbers needs
  no `let` or `$((…))`; `int stringval` is an error rather than 0.
- `-r`: tessh's `readonly NAME=value`. A frozen dataclass's fields can't
  be changed either, which does the job for a group of constants.
- `-a`: a list, `[…]`.
- `-x`: `export NAME=value`, or `$env.NAME = value`.
- `-f`: `inspect.getsource` shows a function's tessh source.

```bash
declare -i intvar

intvar=23
echo "$intvar"   # 23
intvar=stringval
echo "$intvar"   # 0

n=6/3
echo "n = $n"       # n = 6/3

declare -i n
n=6/3
echo "n = $n"       # n = 2
```

```output bash
23
0
n = 6/3
n = 2
```

```tessh
intvar = int 23
print $intvar    # 23
try
  intvar = int stringval
except ValueError as e
  print $e       # (an error, not 0)

n = "6/3"
print "n = $n"      # n = 6/3   (text is text)

n = 6 // 3
print "n = $n"      # n = 2     (// divides integers; 6 / 3 is 2.0)
```

```output
23
invalid literal for int() with base 10: 'stringval'
n = 6/3
n = 2
```

### Example 9-10. Using declare to type variables (`ex20.sh`)

```bash
#!/bin/bash

func1 ()
{
  echo This is a function.
}

declare -f        # Lists the function above.

echo

declare -i var1   # var1 is an integer.
var1=2367
echo "var1 declared as $var1"
var1=var1+1       # Integer declaration eliminates the need for 'let'.
echo "var1 incremented by 1 is $var1."
# Attempt to change variable declared as integer.
echo "Attempting to change var1 to floating point value, 2367.1."
var1=2367.1       # Results in error message, with no change to variable.
echo "var1 is still $var1"

echo

declare -r var2=13.36         # 'declare' permits setting a variable property
                              #+ and simultaneously assigning it a value.
echo "var2 declared as $var2" # Attempt to change readonly variable.
var2=13.37                    # Generates error message, and exit from script.

echo "var2 is still $var2"    # This line will not execute.

exit 0                        # Script will not exit here.
```

```output bash
func1 () 
{ 
    echo This is a function.
}

var1 declared as 2367
var1 incremented by 1 is 2368.
Attempting to change var1 to floating point value, 2367.1.
var1 is still 2368

var2 declared as 13.36
var2 is still 13.36
```

(The comments are wrong about bash 5: the failed assignment prints an
error and the script goes on.)

The functions the script defined are the functions among its globals whose
module is the script's. Turning `2367.1` into an `int` is an error you can
catch, and assigning to a frozen dataclass's field raises
`FrozenInstanceError`, which ends the script unless caught.

```tessh
#!/usr/bin/env tessh
import inspect
from dataclasses import dataclass FrozenInstanceError

def func1
  echo This is a function.

# Lists the function above (declare -f).
for f in (list ((globals).values))
  if (inspect.isfunction $f) and $f.__module__ == $__name__
    print (inspect.getsource $f) end=""

print

var1 = int 2367    # var1 is an integer.
print "var1 declared as $var1"
var1 = $var1 + 1
print "var1 incremented by 1 is $var1."
# Attempt to make var1 an integer from a floating point value.
print "Attempting to change var1 to floating point value, 2367.1."
try
  var1 = int "2367.1"      # An error, with no change to variable.
except ValueError as e
  print $e
print "var1 is still $var1"

print

@dataclass frozen=True
class Readonly
  var2: float

ro = Readonly 13.36           # Sets the value and makes it read-only.
print "var2 declared as $ro.var2"
try
  $ro.var2 = 13.37            # Raises FrozenInstanceError ...
  print "var2 is still $ro.var2"    # ... so this line doesn't run.
except FrozenInstanceError as e
  print $e
```

```output
def func1
  echo This is a function.

var1 declared as 2367
var1 incremented by 1 is 2368.
Attempting to change var1 to floating point value, 2367.1.
invalid literal for int() with base 10: '2367.1'
var1 is still 2368

var2 declared as 13.36
cannot assign to field 'var2'
```

`declare` inside a function makes the variable local to it. In tessh
(and Python) that's the rule for every assignment in a `def`; `global`
says otherwise.

```bash
foo ()
{
FOO="bar"
}

bar ()
{
foo
echo $FOO
}

bar   # Prints bar.

foo (){
declare FOO="bar"
}

bar ()
{
foo
echo $FOO
}

bar  # Prints nothing.

# Thank you, Michael Iatrou, for pointing this out.
```

```output bash
bar
bar
```

(Bash prints `bar` twice there: the first `bar` left `FOO` set. Run
alone, the second prints nothing.)

```tessh
def foo
  global FOO
  FOO = "bar"

def bar
  foo
  print $FOO

bar   # Prints bar.

def foo
  FOO = "baz"      # Local to foo, as with declare.

bar   # Prints bar: the global FOO, which this foo didn't change.
```

```output
bar
bar
```

## 9.3 $RANDOM: generate random integer
<!-- help: abs-random $RANDOM random-numbers random seed randint choice -->

> Anyone who attempts to generate random numbers by deterministic means
> is, of course, living in a state of sin. --John von Neumann

`$RANDOM` gives a new pseudorandom integer from 0 to 32767 each time it's
read. tessh has no such variable; Python's `random` module has
`random.randrange 32768` for the same range, `random.randint lo hi` for any
range, `random.choice` to pick from a list, `random.random` for a float in
[0, 1), and `random.seed n` to start a repeatable series (bash's
`RANDOM=n`). It's a Mersenne Twister, a better generator than bash's.
For keys and passwords use `secrets`, never `random`.

**Gap:** no `$RANDOM`.

The examples here call `random.seed` first so that they print the same
numbers each run, which a checked guide needs. Leave it out (or call it
with no argument) for new numbers every time. The bash versions aren't
run here, their output being random.

### Example 9-11. Generating random numbers (`ex21.sh`)

```bash no-run
#!/bin/bash

# $RANDOM returns a different random integer at each invocation.
# Nominal range: 0 - 32767 (signed 16-bit integer).

MAXCOUNT=10
count=1

echo
echo "$MAXCOUNT random numbers:"
echo "-----------------"
while [ "$count" -le $MAXCOUNT ]      # Generate 10 ($MAXCOUNT) random integers.
do
  number=$RANDOM
  echo $number
  let "count += 1"  # Increment count.
done
echo "-----------------"

# If you need a random int within a certain range, use the 'modulo' operator.
# This returns the remainder of a division operation.

RANGE=500

echo

number=$RANDOM
let "number %= $RANGE"
#           ^^
echo "Random number less than $RANGE  ---  $number"

echo



#  If you need a random integer greater than a lower bound,
#+ then set up a test to discard all numbers below that.

FLOOR=200

number=0   #initialize
while [ "$number" -le $FLOOR ]
do
  number=$RANDOM
done
echo "Random number greater than $FLOOR ---  $number"
echo

   # Let's examine a simple alternative to the above loop, namely
   #       let "number = $RANDOM + $FLOOR"
   # That would eliminate the while-loop and run faster.
   # But, there might be a problem with that. What is it?



# Combine above two techniques to retrieve random number between two limits.
number=0   #initialize
while [ "$number" -le $FLOOR ]
do
  number=$RANDOM
  let "number %= $RANGE"  # Scales $number down within $RANGE.
done
echo "Random number between $FLOOR and $RANGE ---  $number"
echo



# Generate binary choice, that is, "true" or "false" value.
BINARY=2
T=1
number=$RANDOM

let "number %= $BINARY"
#  Note that    let "number >>= 14"    gives a better random distribution
#+ (right shifts out everything except last binary digit).
if [ "$number" -eq $T ]
then
  echo "TRUE"
else
  echo "FALSE"
fi  

echo


# Generate a toss of the dice.
SPOTS=6   # Modulo 6 gives range 0 - 5.
          # Incrementing by 1 gives desired range of 1 - 6.
          # Thanks, Paulo Marcel Coelho Aragao, for the simplification.
die1=0
die2=0
# Would it be better to just set SPOTS=7 and not add 1? Why or why not?

# Tosses each die separately, and so gives correct odds.

    let "die1 = $RANDOM % $SPOTS +1" # Roll first one.
    let "die2 = $RANDOM % $SPOTS +1" # Roll second one.
    #  Which arithmetic operation, above, has greater precedence --
    #+ modulo (%) or addition (+)?


let "throw = $die1 + $die2"
echo "Throw of the dice = $throw"
echo


exit 0
```

`RANDOM` here is a function giving what bash's variable does, so the
script keeps its shape; `random.randrange` and `random.randint` (in the
comments) do the range arithmetic for you, without the slight bias of `%`.

```tessh
#!/usr/bin/env tessh
import random
random.seed 21                         # (Same numbers each run.)

def RANDOM = random.randrange 32768    # 0 - 32767, as bash's $RANDOM.

MAXCOUNT = 10

print
print "$MAXCOUNT random numbers:"
print "-----------------"
for count in (range $MAXCOUNT)         # Generate 10 ($MAXCOUNT) random integers.
  print (RANDOM)
print "-----------------"

# If you need a random int within a certain range, use the 'modulo' operator.
RANGE = 500

print

number = (RANDOM) % $RANGE             # (or: random.randrange $RANGE)
print "Random number less than $RANGE  ---  $number"

print

#  If you need a random integer greater than a lower bound,
#+ then set up a test to discard all numbers below that.
FLOOR = 200

number = 0   # initialize
while $number <= $FLOOR
  number = (RANDOM)
print "Random number greater than $FLOOR ---  $number"
print

# Combine above two techniques to retrieve random number between two limits.
number = 0   # initialize
while $number <= $FLOOR
  number = (RANDOM) % $RANGE           # Scales $number down within $RANGE.
print "Random number between $FLOOR and $RANGE ---  $number"
print                                  # (or: random.randint ($FLOOR + 1) ($RANGE - 1))

# Generate binary choice, that is, "true" or "false" value.
BINARY = 2
T = 1
number = (RANDOM) % $BINARY            # (or: random.choice [True False])
if $number == $T
  print "TRUE"
else
  print "FALSE"

print

# Generate a toss of the dice.
SPOTS = 6    # Modulo 6 gives range 0 - 5; incrementing by 1 gives 1 - 6.

# Tosses each die separately, and so gives correct odds.
die1 = (RANDOM) % $SPOTS + 1           # Roll first one.  (random.randint 1 6)
die2 = (RANDOM) % $SPOTS + 1           # Roll second one.

throw = $die1 + $die2
print "Throw of the dice = $throw"
print
```

```output

10 random numbers:
-----------------
10810
27393
27396
18432
31398
14156
31106
12031
15470
208
-----------------

Random number less than 500  ---  425

Random number greater than 200 ---  24316

Random number between 200 and 500 ---  497

FALSE

Throw of the dice = 7

```

### Example 9-12. Picking a random card from a deck (`pick-card.sh`)

```bash no-run
#!/bin/bash
# pick-card.sh

# This is an example of choosing random elements of an array.


# Pick a card, any card.

Suites="Clubs
Diamonds
Hearts
Spades"

Denominations="2
3
4
5
6
7
8
9
10
Jack
Queen
King
Ace"

# Note variables spread over multiple lines.


suite=($Suites)                # Read into array variable.
denomination=($Denominations)

num_suites=${#suite[*]}        # Count how many elements.
num_denominations=${#denomination[*]}

echo -n "${denomination[$((RANDOM%num_denominations))]} of "
echo ${suite[$((RANDOM%num_suites))]}


# $bozo sh pick-cards.sh
# Jack of Clubs


# Thank you, "jipe," for pointing out this use of $RANDOM.
exit 0
```

`str.split` reads the lines into a list, and `random.choice` picks one
item, so the counting isn't needed.

```tessh
#!/usr/bin/env tessh
import random
random.seed 52                 # (Same card each run.)

# Pick a card, any card.

Suites = """Clubs
Diamonds
Hearts
Spades"""

Denominations = """2
3
4
5
6
7
8
9
10
Jack
Queen
King
Ace"""

suite = ($Suites.split)        # Read into a list.
denomination = ($Denominations.split)

print "$(random.choice $denomination) of $(random.choice $suite)"
```

```output
6 of Clubs
```

### Example 9-13. Brownian Motion Simulation (`brownian.sh`)

```bash no-run
#!/bin/bash
# brownian.sh
# Author: Mendel Cooper
# Reldate: 10/26/07
# License: GPL3

#  ----------------------------------------------------------------
#  This script models Brownian motion:
#+ the random wanderings of tiny particles in a fluid,
#+ as they are buffeted by random currents and collisions.
#+ This is colloquially known as the "Drunkard's Walk."

#  It can also be considered as a stripped-down simulation of a
#+ Galton Board, a slanted board with a pattern of pegs,
#+ down which rolls a succession of marbles, one at a time.
#+ At the bottom is a row of slots or catch basins in which
#+ the marbles come to rest at the end of their journey.
#  Think of it as a kind of bare-bones Pachinko game.
#  As you see by running the script,
#+ most of the marbles cluster around the center slot.
#+ This is consistent with the expected binomial distribution.
#  As a Galton Board simulation, the script
#+ disregards such parameters as
#+ board tilt-angle, rolling friction of the marbles,
#+ angles of impact, and elasticity of the pegs.
#  To what extent does this affect the accuracy of the simulation?
#  ----------------------------------------------------------------

PASSES=500            #  Number of particle interactions / marbles.
ROWS=10               #  Number of "collisions" (or horiz. peg rows).
RANGE=3               #  0 - 2 output range from $RANDOM.
POS=0                 #  Left/right position.
RANDOM=$$             #  Seeds the random number generator from PID
                      #+ of script.

declare -a Slots      # Array holding cumulative results of passes.
NUMSLOTS=21           # Number of slots at bottom of board.


Initialize_Slots () { # Zero out all elements of the array.
for i in $( seq $NUMSLOTS )
do
  Slots[$i]=0
done

echo                  # Blank line at beginning of run.
  }


Show_Slots () {
echo; echo
echo -n " "
for i in $( seq $NUMSLOTS )   # Pretty-print array elements.
do
  printf "%3d" ${Slots[$i]}   # Allot three spaces per result.
done

echo # Row of slots:
echo " |__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|"
echo "                                ||"
echo #  Note that if the count within any particular slot exceeds 99,
     #+ it messes up the display.
     #  Running only(!) 500 passes usually avoids this.
  }


Move () {              # Move one unit right / left, or stay put.
  Move=$RANDOM         # How random is $RANDOM? Well, let's see ...
  let "Move %= RANGE"  # Normalize into range of 0 - 2.
  case "$Move" in
    0 ) ;;                   # Do nothing, i.e., stay in place.
    1 ) ((POS--));;          # Left.
    2 ) ((POS++));;          # Right.
    * ) echo -n "Error ";;   # Anomaly! (Should never occur.)
  esac
  }


Play () {                    # Single pass (inner loop).
i=0
while [ "$i" -lt "$ROWS" ]   # One event per row.
do
  Move
  ((i++));
done

SHIFT=11                     # Why 11, and not 10?
let "POS += $SHIFT"          # Shift "zero position" to center.
(( Slots[$POS]++ ))          # DEBUG: echo $POS

# echo -n "$POS "

  }


Run () {                     # Outer loop.
p=0
while [ "$p" -lt "$PASSES" ]
do
  Play
  (( p++ ))
  POS=0                      # Reset to zero. Why?
done
  }


# --------------
# main ()
Initialize_Slots
Run
Show_Slots
# --------------

exit $?

#  Exercises:
#  ---------
#  1) Show the results in a vertical bar graph, or as an alternative,
#+    a scattergram.
#  2) Alter the script to use /dev/urandom instead of $RANDOM.
#     Will this make the results more random?
#  3) Provide some sort of "animation" or graphic output
#     for each marble played.
```

The slots are a dict from slot number to count; `match` is the `case`.
Functions that change the script's `POS` and `Slots` say `global`.

```tessh
#!/usr/bin/env tessh
import random

PASSES = 500          #  Number of particle interactions / marbles.
ROWS = 10             #  Number of "collisions" (or horiz. peg rows).
RANGE = 3             #  0 - 2 output range.
POS = 0               #  Left/right position.
random.seed 1007      #  (A fixed seed, for the same run each time;
                      #+ the bash seeds with its PID.)

NUMSLOTS = 21         # Number of slots at bottom of board.
Slots = {}            # Slot number -> marbles in it.

def Initialize_Slots  # Zero out all the slots.
  global Slots
  Slots = {$i: 0 for i in (range 1 ($NUMSLOTS + 1))}
  print               # Blank line at beginning of run.

def Show_Slots
  print ; print
  print " " end=""
  for i in (range 1 ($NUMSLOTS + 1))  # Pretty-print the counts.
    printf "%3d" $Slots[$i]           # Allot three spaces per result.
  print # Row of slots:
  print " |__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|"
  print "                                ||"
  print

def Move              # Move one unit right / left, or stay put.
  global POS
  match (random.randrange $RANGE)
    case 0
      pass            # Do nothing, i.e., stay in place.
    case 1
      POS -= 1        # Left.
    case 2
      POS += 1        # Right.
    case _
      print "Error " end=""   # Anomaly! (Should never occur.)

def Play              # Single pass (inner loop).
  global POS
  for i in (range $ROWS)     # One event per row.
    Move
  SHIFT = 11                 # Why 11, and not 10?
  POS += $SHIFT              # Shift "zero position" to center.
  $Slots[$POS] += 1

def Run               # Outer loop.
  global POS
  for p in (range $PASSES)
    Play
    POS = 0           # Reset to zero. Why?

# --------------
Initialize_Slots
Run
Show_Slots
# --------------
```

```output



   0  0  0  4  5 15 26 34 44 84 77 65 69 42 22  7  5  1  0  0  0
 |__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|__|
                                ||

```

Ways to get a random number in a range, from the ABS Guide, and Python's
way beside each:

```bash no-run
#  Generate random number between 6 and 30.
   rnumber=$((RANDOM%25+6))

#  Generate random number in the same 6 - 30 range,
#+ but the number must be evenly divisible by 3.
   rnumber=$(((RANDOM%30/3+1)*3))

#  Note that this will not work all the time.
#  It fails if $RANDOM%30 returns 0.

#  Frank Wang suggests the following alternative:
   rnumber=$(( RANDOM%27/3*3+6 ))
```

```tessh
import random
random.seed 6

#  Generate random number between 6 and 30.
rnumber = random.randint 6 30
print $rnumber

#  Generate random number in the same 6 - 30 range,
#+ but the number must be evenly divisible by 3.
rnumber = random.randrange 6 31 3      # start, stop (not included), step
print $rnumber
```

```output
24
9
```

Bill Gradwohl's function returns a random number between two values that
is a multiple of a third, and the script around it checks that every
possible answer comes up.

### Example 9-14. Random between values (`random-between.sh`)

```bash no-run
#!/bin/bash
# random-between.sh
# Random number between two specified values. 
# Script by Bill Gradwohl, with minor modifications by the document author.
# Corrections in lines 187 and 189 by Anthony Le Clezio.
# Used with permission.


randomBetween() {
   #  Generates a positive or negative random number
   #+ between $min and $max
   #+ and divisible by $divisibleBy.
   #  Gives a "reasonably random" distribution of return values.
   #
   #  Bill Gradwohl - Oct 1, 2003

   syntax() {
   # Function embedded within function.
      echo
      echo    "Syntax: randomBetween [min] [max] [multiple]"
      echo
      echo -n "Expects up to 3 passed parameters, "
      echo    "but all are completely optional."
      echo    "min is the minimum value"
      echo    "max is the maximum value"
      echo -n "multiple specifies that the answer must be "
      echo     "a multiple of this value."
      echo    "    i.e. answer must be evenly divisible by this number."
      echo    
      echo    "If any value is missing, defaults area supplied as: 0 32767 1"
      echo -n "Successful completion returns 0, "
      echo     "unsuccessful completion returns"
      echo    "function syntax and 1."
      echo -n "The answer is returned in the global variable "
      echo    "randomBetweenAnswer"
      echo -n "Negative values for any passed parameter are "
      echo    "handled correctly."
   }

   local min=${1:-0}
   local max=${2:-32767}
   local divisibleBy=${3:-1}
   # Default values assigned, in case parameters not passed to function.

   local x
   local spread

   # Let's make sure the divisibleBy value is positive.
   [ ${divisibleBy} -lt 0 ] && divisibleBy=$((0-divisibleBy))

   # Sanity check.
   if [ $# -gt 3 -o ${divisibleBy} -eq 0 -o  ${min} -eq ${max} ]; then 
      syntax
      return 1
   fi

   # See if the min and max are reversed.
   if [ ${min} -gt ${max} ]; then
      # Swap them.
      x=${min}
      min=${max}
      max=${x}
   fi

   #  If min is itself not evenly divisible by $divisibleBy,
   #+ then fix the min to be within range.
   if [ $((min/divisibleBy*divisibleBy)) -ne ${min} ]; then 
      if [ ${min} -lt 0 ]; then
         min=$((min/divisibleBy*divisibleBy))
      else
         min=$((((min/divisibleBy)+1)*divisibleBy))
      fi
   fi

   #  If max is itself not evenly divisible by $divisibleBy,
   #+ then fix the max to be within range.
   if [ $((max/divisibleBy*divisibleBy)) -ne ${max} ]; then 
      if [ ${max} -lt 0 ]; then
         max=$((((max/divisibleBy)-1)*divisibleBy))
      else
         max=$((max/divisibleBy*divisibleBy))
      fi
   fi

   #  ---------------------------------------------------------------------
   #  Now, to do the real work.

   #  Note that to get a proper distribution for the end points,
   #+ the range of random values has to be allowed to go between
   #+ 0 and abs(max-min)+divisibleBy, not just abs(max-min)+1.

   #  The slight increase will produce the proper distribution for the
   #+ end points.

   #  Changing the formula to use abs(max-min)+1 will still produce
   #+ correct answers, but the randomness of those answers is faulty in
   #+ that the number of times the end points ($min and $max) are returned
   #+ is considerably lower than when the correct formula is used.
   #  ---------------------------------------------------------------------

   spread=$((max-min))
   #  Omair Eshkenazi points out that this test is unnecessary,
   #+ since max and min have already been switched around.
   [ ${spread} -lt 0 ] && spread=$((0-spread))
   let spread+=divisibleBy
   randomBetweenAnswer=$(((RANDOM%spread)/divisibleBy*divisibleBy+min))   

   return 0

   #  However, Paulo Marcel Coelho Aragao points out that
   #+ when $max and $min are not divisible by $divisibleBy,
   #+ the formula fails.
   #
   #  He suggests instead the following formula:
   #    rnumber = $(((RANDOM%(max-min+1)+min)/divisibleBy*divisibleBy))

}

# Let's test the function.
min=-14
max=20
divisibleBy=3


#  Generate an array of expected answers and check to make sure we get
#+ at least one of each answer if we loop long enough.

declare -a answer
minimum=${min}
maximum=${max}
   if [ $((minimum/divisibleBy*divisibleBy)) -ne ${minimum} ]; then 
      if [ ${minimum} -lt 0 ]; then
         minimum=$((minimum/divisibleBy*divisibleBy))
      else
         minimum=$((((minimum/divisibleBy)+1)*divisibleBy))
      fi
   fi


   #  If max is itself not evenly divisible by $divisibleBy,
   #+ then fix the max to be within range.

   if [ $((maximum/divisibleBy*divisibleBy)) -ne ${maximum} ]; then 
      if [ ${maximum} -lt 0 ]; then
         maximum=$((((maximum/divisibleBy)-1)*divisibleBy))
      else
         maximum=$((maximum/divisibleBy*divisibleBy))
      fi
   fi


#  We need to generate only positive array subscripts,
#+ so we need a displacement that that will guarantee
#+ positive results.

disp=$((0-minimum))
for ((i=${minimum}; i<=${maximum}; i+=divisibleBy)); do
   answer[i+disp]=0
done


# Now loop a large number of times to see what we get.
loopIt=1000   #  The script author suggests 100000,
              #+ but that takes a good long while.

for ((i=0; i<${loopIt}; ++i)); do

   #  Note that we are specifying min and max in reversed order here to
   #+ make the function correct for this case.

   randomBetween ${max} ${min} ${divisibleBy}

   # Report an error if an answer is unexpected.
   [ ${randomBetweenAnswer} -lt ${min} -o ${randomBetweenAnswer} -gt ${max} ] \
   && echo MIN or MAX error - ${randomBetweenAnswer}!
   [ $((randomBetweenAnswer%${divisibleBy})) -ne 0 ] \
   && echo DIVISIBLE BY error - ${randomBetweenAnswer}!

   # Store the answer away statistically.
   answer[randomBetweenAnswer+disp]=$((answer[randomBetweenAnswer+disp]+1))
done



# Let's check the results

for ((i=${minimum}; i<=${maximum}; i+=divisibleBy)); do
   [ ${answer[i+disp]} -eq 0 ] \
   && echo "We never got an answer of $i." \
   || echo "${i} occurred ${answer[i+disp]} times."
done


exit 0
```

The tessh function returns its answer (or `None`, after the syntax
message) instead of setting a global, and the parameters' defaults are in
the `def` line. Bash's `/` rounds toward zero and Python's `//` rounds
down, which differ for negative numbers: `-14 / 3` is -4 in bash and
`-14 // 3` is -5 in Python. The script depends on bash's way, so `div`
does it. The answers are counted in a dict, so no displacement is needed
to keep the subscripts positive.

```tessh
#!/usr/bin/env tessh
import random
random.seed 2003                 # (Same counts each run.)

def div a b = int ($a / $b)      # Bash's integer division: toward zero.

def randomBetween min=0 max=32767 divisibleBy=1 *extra
  #  Generates a positive or negative random number
  #+ between $min and $max
  #+ and divisible by $divisibleBy.
  #  Gives a "reasonably random" distribution of return values.
  #
  #  Bill Gradwohl - Oct 1, 2003

  def syntax
    # Function embedded within function.
    print
    print "Syntax: randomBetween [min] [max] [multiple]"
    print
    print "Expects up to 3 passed parameters, but all are completely optional."
    print "min is the minimum value"
    print "max is the maximum value"
    print "multiple specifies that the answer must be a multiple of this value."
    print "    i.e. answer must be evenly divisible by this number."
    print
    print "If any value is missing, defaults are supplied as: 0 32767 1"
    print "Returns the answer, or None after showing this."
    print "Negative values for any passed parameter are handled correctly."

  # Let's make sure the divisibleBy value is positive.
  divisibleBy = abs $divisibleBy

  # Sanity check.
  if $extra or $divisibleBy == 0 or $min == $max
    syntax
    return None

  # See if the min and max are reversed.
  if $min > $max
    min, max = $max, $min      # Swap them.

  #  If min is itself not evenly divisible by $divisibleBy,
  #+ then fix the min to be within range.
  if (div $min $divisibleBy) * $divisibleBy != $min
    if $min < 0
      min = (div $min $divisibleBy) * $divisibleBy
    else
      min = ((div $min $divisibleBy) + 1) * $divisibleBy

  #  If max is itself not evenly divisible by $divisibleBy,
  #+ then fix the max to be within range.
  if (div $max $divisibleBy) * $divisibleBy != $max
    if $max < 0
      max = ((div $max $divisibleBy) - 1) * $divisibleBy
    else
      max = (div $max $divisibleBy) * $divisibleBy

  #  Now, to do the real work.
  #  To get a proper distribution for the end points, the range of
  #+ random values has to go from 0 to abs(max-min)+divisibleBy,
  #+ not just abs(max-min)+1.
  spread = $max - $min + $divisibleBy
  (div (random.randrange $spread) $divisibleBy) * $divisibleBy + $min

# Let's test the function.
min = -14
max = 20
divisibleBy = 3

#  The expected answers: every multiple of 3 in the range.
expected = [$i for i in (range $min ($max + 1)) if $i % $divisibleBy == 0]
answer = {$i: 0 for i in $expected}

# Now loop a large number of times to see what we get.
loopIt = 1000    #  The script author suggests 100000,
                 #+ but that takes a good long while.

for i in (range $loopIt)
  #  Note that we are specifying min and max in reversed order here to
  #+ make the function correct for this case.
  r = randomBetween $max $min $divisibleBy

  # Report an error if an answer is unexpected.
  if $r < $min or $r > $max
    print "MIN or MAX error - $r!"
  if $r % $divisibleBy != 0
    print "DIVISIBLE BY error - $r!"

  # Store the answer away statistically.
  $answer[$r] += 1

# Let's check the results
for [i n] in ($answer.items)
  if $n == 0
    print "We never got an answer of $i."
  else
    print "$i occurred $n times."
```

```output
-12 occurred 110 times.
-9 occurred 85 times.
-6 occurred 95 times.
-3 occurred 104 times.
0 occurred 85 times.
3 occurred 89 times.
6 occurred 97 times.
9 occurred 83 times.
12 occurred 69 times.
15 occurred 84 times.
18 occurred 99 times.
```

### Example 9-15. Rolling a single die with RANDOM (`random-test.sh`)

```bash no-run
#!/bin/bash
# How random is RANDOM?

RANDOM=$$       # Reseed the random number generator using script process ID.

PIPS=6          # A die has 6 pips.
MAXTHROWS=600   # Increase this if you have nothing better to do with your time.
throw=0         # Number of times the dice have been cast.

ones=0          #  Must initialize counts to zero,
twos=0          #+ since an uninitialized variable is null, NOT zero.
threes=0
fours=0
fives=0
sixes=0

print_result ()
{
echo
echo "ones =   $ones"
echo "twos =   $twos"
echo "threes = $threes"
echo "fours =  $fours"
echo "fives =  $fives"
echo "sixes =  $sixes"
echo
}

update_count()
{
case "$1" in
  0) ((ones++));;   # Since a die has no "zero", this corresponds to 1.
  1) ((twos++));;   # And this to 2.
  2) ((threes++));; # And so forth.
  3) ((fours++));;
  4) ((fives++));;
  5) ((sixes++));;
esac
}

echo


while [ "$throw" -lt "$MAXTHROWS" ]
do
  let "die1 = RANDOM % $PIPS"
  update_count $die1
  let "throw += 1"
done  

print_result

exit $?

#  The scores should distribute evenly, assuming RANDOM is random.
#  With $MAXTHROWS at 600, all should cluster around 100,
#+ plus-or-minus 20 or so.
#
#  Keep in mind that RANDOM is a ***pseudorandom*** generator,
#+ and not a spectacularly good one at that.

#  Randomness is a deep and complex subject.
#  Sufficiently long "random" sequences may exhibit
#+ chaotic and other "non-random" behavior.

# Exercise (easy):
# ---------------
# Rewrite this script to flip a coin 1000 times.
# Choices are "HEADS" and "TAILS."
```

Six counters become one dict, so `update_count` is one line and needs no
`case`; `collections.Counter` would do the counting too.

```tessh
#!/usr/bin/env tessh
import random
random.seed 600        # (Bash reseeds with the PID; this is the same each run.)

PIPS = 6               # A die has 6 pips.
MAXTHROWS = 600        # Increase this if you have nothing better to do with your time.

names = [ones twos threes fours fives sixes]
counts = {$name: 0 for name in $names}   # Every count starts at zero.

def print_result
  print
  print "ones =   $counts[ones]"
  print "twos =   $counts[twos]"
  print "threes = $counts[threes]"
  print "fours =  $counts[fours]"
  print "fives =  $counts[fives]"
  print "sixes =  $counts[sixes]"
  print

def update_count die
  $counts[$names[$die]] += 1    # A die has no "zero": 0 counts as a one.

print

for throw in (range $MAXTHROWS)
  update_count (random.randrange $PIPS)

print_result

#  The scores should distribute evenly, assuming random is random.
#  With $MAXTHROWS at 600, all should cluster around 100,
#+ plus-or-minus 20 or so.
```

```output


ones =   104
twos =   91
threes = 84
fours =  99
fives =  119
sixes =  103

```

Setting `RANDOM` seeds bash's generator, and the same seed gives the same
series. `random.seed` does that in Python.

### Example 9-16. Reseeding RANDOM (`seeding-random.sh`)

```bash no-run
#!/bin/bash
# seeding-random.sh: Seeding the RANDOM variable.
# v 1.1, reldate 09 Feb 2013

MAXCOUNT=25       # How many numbers to generate.
SEED=

random_numbers ()
{
local count=0
local number

while [ "$count" -lt "$MAXCOUNT" ]
do
  number=$RANDOM
  echo -n "$number "
  let "count++"
done  
}

echo; echo

SEED=1
RANDOM=$SEED      # Setting RANDOM seeds the random number generator.
echo "Random seed = $SEED"
random_numbers


RANDOM=$SEED      # Same seed for RANDOM . . .
echo; echo "Again, with same random seed ..."
echo "Random seed = $SEED"
random_numbers    # . . . reproduces the exact same number series.
                  #
                  # When is it useful to duplicate a "random" series?

echo; echo

SEED=2
RANDOM=$SEED      # Trying again, but with a different seed . . .
echo "Random seed = $SEED"
random_numbers    # . . . gives a different number series.

echo; echo

# RANDOM=$$  seeds RANDOM from process id of script.
# It is also possible to seed RANDOM from 'time' or 'date' commands.

# Getting fancy...
SEED=$(head -1 /dev/urandom | od -N 1 | awk '{ print $2 }'| sed s/^0*//)
#  Pseudo-random output fetched
#+ from /dev/urandom (system pseudo-random device-file),
#+ then converted to line of printable (octal) numbers by "od",
#+ then "awk" retrieves just one number for SEED,
#+ finally "sed" removes any leading zeros.
RANDOM=$SEED
echo "Random seed = $SEED"
random_numbers

echo; echo

exit 0
```

`random.seed` with no argument seeds from the system's own random source,
as the "Getting fancy" pipeline does by hand; `os.urandom` gives the bytes
themselves. That last part prints different numbers each run, so it's a
block of its own.

```tessh
#!/usr/bin/env tessh
import random

MAXCOUNT = 10       # How many numbers to generate (25 in the original).

def random_numbers
  for count in (range $MAXCOUNT)
    print (random.randrange 32768) end=" "

print ; print

SEED = 1
random.seed $SEED   # Seeds the random number generator.
print "Random seed = $SEED"
random_numbers

random.seed $SEED   # Same seed . . .
print ; print "Again, with same random seed ..."
print "Random seed = $SEED"
random_numbers      # . . . reproduces the exact same number series.
                    #
                    # When is it useful to duplicate a "random" series?

print ; print

SEED = 2
random.seed $SEED   # Trying again, but with a different seed . . .
print "Random seed = $SEED"
random_numbers      # . . . gives a different number series.

print ; print
```

```output


Random seed = 1
8805 4135 16716 7727 32468 29457 30949 24878 13759 6151 
Again, with same random seed ...
Random seed = 1
8805 4135 16716 7727 32468 29457 30949 24878 13759 6151 

Random seed = 2
3706 6002 5562 23662 11081 20194 16487 13907 2341 10379 

```

```tessh
import random os

def random_numbers
  for count in (range 10)
    print (random.randrange 32768) end=" "

# Getting fancy... a seed from /dev/urandom.
SEED = int.from_bytes (os.urandom 1)
random.seed $SEED                    # (random.seed with no seed does this.)
print "Random seed = $SEED"
random_numbers
print
```

```text
Random seed = 135
29380 9113 12497 24914 19870 2981 27560 23441 8133 20367 
```

`awk` has its own generator, `rand()`, which the next script uses. tessh
runs `awk` the same way; Python's `random.random` is the same thing
without starting a program.

### Example 9-17. Pseudorandom numbers, using awk (`random2.sh`)

```bash
#!/bin/bash
#  random2.sh: Returns a pseudorandom number in the range 0 - 1,
#+ to 6 decimal places. For example: 0.822725
#  Uses the awk rand() function.

AWKSCRIPT=' { srand(); print rand() } '
#           Command(s)/parameters passed to awk
# Note that srand() reseeds awk's random number generator.


echo -n "Random number between 0 and 1 = "

echo | awk "$AWKSCRIPT"
# What happens if you leave out the 'echo'?

exit 0


# Exercises:
# ---------

# 1) Using a loop construct, print out 10 different random numbers.
#      (Hint: you must reseed the srand() function with a different seed
#+     in each pass through the loop. What happens if you omit this?)

# 2) Using an integer multiplier as a scaling factor, generate random numbers 
#+   in the range of 10 to 100.

# 3) Same as exercise #2, above, but generate random integers this time.
```

```tessh
#!/usr/bin/env tessh
import random

AWKSCRIPT = ' { srand(); print rand() } '
# Note that srand() reseeds awk's random number generator.

print "Random number between 0 and 1 = " end="" flush=True
echo | awk $AWKSCRIPT

# The same without awk: a float in [0, 1), to 6 decimal places.
print "Random number between 0 and 1 = ${(random.random):.6f}"

# Exercise 1, in Python: 10 numbers, no reseeding needed.
print [(round (random.random) 6) for _ in (range 10)]
```

```text
Random number between 0 and 1 = 0.0532963
Random number between 0 and 1 = 0.428712
[0.711204 0.052358 0.330163 0.95113 0.441513 0.214745 0.818045 0.373286 0.502113 0.689941]
```

The `date` command can make a seed too (`date +%N`, the nanoseconds); in
Python, `time.time_ns`.
