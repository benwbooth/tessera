# 15. Internal Commands and Builtins
<!-- help: abs-internal abs-15 builtins builtin internal-commands keywords -->

A builtin is a command bash carries out itself, without starting a program:
for speed, or because it has to change the shell's own state (`cd`,
`export`, `read` into a variable). A keyword (`for`, `while`, `!`) is part
of the syntax rather than a command.

tessh has only a handful of builtins: `cd`, `pushd`, `popd`, `dirs`,
`export`, `unset`, `exit`, `source`, `which`, `glob`, `jobs`, `fg`, `bg` and
`need` (`help NAME` describes each). Most of bash's others do things that in
tessh are plain Python: a variable is set with `=`, arithmetic is Python's,
text is formatted with f-strings, arguments are a list. Where that leaves a
real hole, this chapter says so.

When a shell runs a program it *forks*: the new process is the child, and
the parent goes on (or waits for it). A child can't change its parent's
variables, which is behind several of the surprises below.

### Example 15-1. A script that spawns multiple instances of itself (`spawn.sh`)

```bash no-run
#!/bin/bash
# spawn.sh


PIDS=$(pidof sh $0)  # Process IDs of the various instances of this script.
P_array=( $PIDS )    # Put them in an array (why?).
echo $PIDS           # Show process IDs of parent and child processes.
let "instances = ${#P_array[*]} - 1"  # Count elements, less 1.
                                      # Why subtract 1?
echo "$instances instance(s) of this script running."
echo "[Hit Ctl-C to exit.]"; echo


sleep 1              # Wait.
sh $0                # Play it again, Sam.

exit 0               # Not necessary; script will never get to here.
                     # Why not?

#  After exiting with a Ctl-C,
#+ do all the spawned instances of the script die?
#  If so, why?

# Note:
# ----
# Be careful not to run this script too long.
# It will eventually eat up too many system resources.

#  Is having a script spawn multiple instances of itself
#+ an advisable scripting technique.
#  Why or why not?
```

`pgrep -f` finds the processes running this script (bash's `pidof sh $0`).
A tessh script runs again as `tessh script`; `/proc/self/exe` is the tessh
running now, and `env` runs a program named by a value. This one stops at
three instances instead of waiting for Ctrl+C.

```tessh
import os
# spawn.tsh

PIDS = (pgrep -f $argv[0]).lines   # Process IDs of the various instances of this script.
instances = len $PIDS              # (Printed in the original; they change every run.)
print "$instances instance(s) of this script running."
print "[Hit Ctl-C to exit.]" ; print

sleep 0.1                          # Wait.
if $instances < 3                  # (Three are enough here.)
  env (os.readlink /proc/self/exe) $argv[0]   # Play it again, Sam.
exit 0
```

```output
1 instance(s) of this script running.
[Hit Ctl-C to exit.]

2 instance(s) of this script running.
[Hit Ctl-C to exit.]

3 instance(s) of this script running.
[Hit Ctl-C to exit.]

```

**Gap:** a program named by a value can't head a command: `\$exe args`
calls the value as a function. `env $exe args` (or
`subprocess.run [$exe …]`) is the way round.

bash's `echo` is a builtin, and `/bin/echo` the program; they behave much
the same. In tessh `echo` is always the program (`print` is the Python
function, and `\echo` runs the program even when a function has its name).

```bash
#!/bin/bash

echo "This line uses the \"echo\" builtin."
/bin/echo "This line uses the /bin/echo system command."
```

```output bash
This line uses the "echo" builtin.
```

```tessh
print "This line uses Python's \"print\"."
/bin/echo "This line uses the /bin/echo system command."
```

```output
This line uses Python's "print".
```

## I/O
<!-- help: abs-builtins-io echo printf read REPLY input -->

**echo** prints its arguments; `-e` makes it understand escapes and `-n`
leaves off the newline. `print` is tessh's: its strings always understand
escapes, `end=""` leaves off the newline and `sep=` sets what goes between
words. Testing whether a variable holds some text, which the ABS Guide does
with `echo "$VAR" | grep -q txt`, is `=~` with the text:

```bash
VAR=notes.txt
if echo "$VAR" | grep -q txt   # if [[ $VAR = *txt* ]]
then
  echo "$VAR contains the substring sequence \"txt\""
fi
```

```tessh
VAR = notes.txt
if $VAR =~ 'txt'
  print "$VAR contains the substring sequence \"txt\""
```

```output
notes.txt contains the substring sequence "txt"
```

bash's `echo $(command)` loses the output's line breaks, because the
unquoted output is split into words. tessh doesn't split values, so a
command's output keeps its lines; `.split` makes words of it when you want
them. A string with a line break in it prints on two lines, however it was
made.

**printf** formats, as C's `printf()` does. tessh runs the `printf`
program, which formats the same way; Python's own formatting, `"${x:.2f}"`
in a string or `format`, does it without a program.

### Example 15-2. printf in action (`ex47.sh`)

```bash
#!/bin/bash
# printf demo

declare -r PI=3.14159265358979     # Read-only variable, i.e., a constant.
declare -r DecimalConstant=31373

Message1="Greetings,"
Message2="Earthling."

echo

printf "Pi to 2 decimal places = %1.2f" $PI
echo
printf "Pi to 9 decimal places = %1.9f" $PI  # It even rounds off correctly.

printf "\n"                                  # Prints a line feed,
                                             # Equivalent to 'echo' . . .

printf "Constant = \t%d\n" $DecimalConstant  # Inserts tab (\t).

printf "%s %s \n" $Message1 $Message2

echo

# ==========================================#
# Simulation of C function, sprintf().
# Loading a variable with a formatted string.

echo 

Pi12=$(printf "%1.12f" $PI)
echo "Pi to 12 decimal places = $Pi12"      # Roundoff error!

Msg=`printf "%s %s \n" $Message1 $Message2`
echo $Msg; echo $Msg

#  As it happens, the 'sprintf' function can now be accessed
#+ as a loadable module to Bash,
#+ but this is not portable.

exit 0
```

`declare -r` makes a constant; tessh's variables are Python's, which can
always be changed. `printf` as a value (`Pi12 = printf …`) is bash's
`$(printf …)`, sprintf included.

```tessh
# printf demo

PI = 3.14159265358979     # (A constant: see the gap below.)
DecimalConstant = 31373

Message1 = "Greetings,"
Message2 = "Earthling."

print

printf "Pi to 2 decimal places = %1.2f" $PI
print
printf "Pi to 9 decimal places = %1.9f" $PI  # It even rounds off correctly.

printf "\n"                                  # Prints a line feed.

printf "Constant = \t%d\n" $DecimalConstant  # Inserts tab (\t).

printf "%s %s \n" $Message1 $Message2

print

# ==========================================#
# Simulation of C function, sprintf().
# Loading a variable with a formatted string.

print

Pi12 = printf "%1.12f" $PI
print "Pi to 12 decimal places = $Pi12"      # Roundoff error!

Msg = "$Message1 $Message2 "                 # (Python formats without printf.)
print ($Msg.strip) ; print ($Msg.strip)      # (bash's unquoted $Msg loses the last space.)

exit 0
```

```output

Pi to 2 decimal places = 3.14
Pi to 9 decimal places = 3.141592654
Constant = 	31373
Greetings, Earthling. 


Pi to 12 decimal places = 3.141592653590
Greetings, Earthling.
Greetings, Earthling.
```

**Gap:** no read-only variables (`declare -r`, `readonly`): a name can
always be assigned again. `typing.Final` marks one for a type checker, but
nothing stops the assignment when it runs.

`printf` to standard error makes a handy error function. In tessh the
function formats with Python's `%`, and since a failed `cd` is an error
(not a status), `try` is how to catch it:

```bash
E_BADDIR=85

var=nonexistent_directory

error()
{
  printf "$@" >&2
  # Formats positional params passed, and sends them to stderr.
  echo
  exit $E_BADDIR
}

cd $var || error $"Can't cd to %s." "$var"

# Thanks, S.C.
```

```tessh
E_BADDIR = 85

var = nonexistent_directory

def error fmt *args
  print ($fmt % $args) >&2   # Formats the arguments passed, and sends them to stderr.
  print
  exit $E_BADDIR

try
  cd $var
except OSError
  error "Can't cd to %s." $var
```

```output

```

**read** reads a line from standard input into variables. tessh has no
`read`: `input` reads a line (`input "prompt"` shows the prompt first), and
`$sys.stdin` is the input as a Python file. These examples answer with
canned input (the `io.StringIO` at the top stands in for the keyboard), so
they run without anyone typing; take that line out and they ask you.

### Example 15-3. Variable assignment, using read (`ex36.sh`)

```bash no-run
#!/bin/bash
# "Reading" variables.

echo -n "Enter the value of variable 'var1': "
# The -n option to echo suppresses newline.

read var1
# Note no '$' in front of var1, since it is being set.

echo "var1 = $var1"


echo

# A single 'read' statement can set multiple variables.
echo -n "Enter the values of variables 'var2' and 'var3' "
echo =n "(separated by a space or tab): "
read var2 var3
echo "var2 = $var2      var3 = $var3"
#  If you input only one value,
#+ the other variable(s) will remain unset (null).

exit 0
```

One `read` can fill several variables: the line is split at whitespace and
the last one gets what's left. In tessh, split the line and unpack it.
(Python wants exactly as many parts as names; `read` leaves the extra
variables empty. The original's `echo =n` is a typo for `echo -n`.)

```tessh
import sys io
sys.stdin = io.StringIO "23\nfirst second\n"   # (Canned answers.)

var1 = input "Enter the value of variable 'var1': "
# (input shows the prompt and leaves the cursor after it.)

print "var1 = $var1"

print

# One input line can set several variables: split it.
[var2 var3] = ((input "Enter the values of variables 'var2' and 'var3' (separated by a space or tab): ").split None 1)
print "var2 = $var2      var3 = $var3"

exit 0
```

```output
Enter the value of variable 'var1': var1 = 23

Enter the values of variables 'var2' and 'var3' (separated by a space or tab): var2 = first      var3 = second
```

(The typed answers don't show in the output: a terminal echoes what you
type, and here nobody typed.)

**Gap:** no `read`: `input` reads a line, but splitting it into several
names, the last taking the rest and missing ones left empty, is done by
hand.

### Example 15-4. What happens when read has no variable (`read-novar.sh`)

```bash no-run
#!/bin/bash
# read-novar.sh

echo

# -------------------------- #
echo -n "Enter a value: "
read var
echo "\"var\" = "$var""
# Everything as expected here.
# -------------------------- #

echo

# ------------------------------------------------------------------- #
echo -n "Enter another value: "
read           #  No variable supplied for 'read', therefore...
               #+ Input to 'read' assigned to default variable, $REPLY.
var="$REPLY"
echo "\"var\" = "$var""
# This is equivalent to the first code block.
# ------------------------------------------------------------------- #

echo
echo "========================="
echo


#  This example is similar to the "reply.sh" script.
#  However, this one shows that $REPLY is available
#+ even after a 'read' to a variable in the conventional way.


# ================================================================= #

#  In some instances, you might wish to discard the first value read.
#  In such cases, simply ignore the $REPLY variable.

{ # Code block.
read            # Line 1, to be discarded.
read line2      # Line 2, saved in variable.
  } <$0
echo "Line 2 of this script is:"
echo "$line2"   #   # read-novar.sh
echo            #   #!/bin/bash  line discarded.

# See also the soundcard-on.sh script.

exit 0
```

`read` with no name puts the line in `$REPLY`. tessh's `input` returns the
line, so it's always given a name. The second half reads the script's own
first two lines, keeping the second; in tessh that's a file opened with
`open` and read with `.readline`.

```tessh
#!/usr/bin/env tessh
# read-novar.tsh
import sys io
sys.stdin = io.StringIO "first answer\nsecond answer\n"   # (Canned answers.)

print

# -------------------------- #
var = input "Enter a value: "
print "\"var\" = $var"
# Everything as expected here.
# -------------------------- #

print

# -------------------------- #
REPLY = input "Enter another value: "   # (tessh has no default $REPLY.)
var = $REPLY
print "\"var\" = $var"
# -------------------------- #

print
print "========================="
print

#  In some instances, you might wish to discard the first value read.
with (open $argv[0]) as f
  $f.readline                           # Line 1, to be discarded.
  line2 = (($f.readline).rstrip "\n")   # Line 2, saved in variable.
print "Line 2 of this script is:"
print $line2
print

exit 0
```

```output

Enter a value: "var" = first answer

Enter another value: "var" = second answer

=========================

Line 2 of this script is:
# read-novar.tsh

```

### Example 15-5. Multi-line input to read (`read-r.sh`)

```bash no-run
#!/bin/bash

echo

echo "Enter a string terminated by a \\, then press <ENTER>."
echo "Then, enter a second string (no \\ this time), and again press <ENTER>."

read var1     # The "\" suppresses the newline, when reading $var1.
              #     first line \
              #     second line

echo "var1 = $var1"
#     var1 = first line second line

#  For each line terminated by a "\"
#+ you get a prompt on the next line to continue feeding characters into var1.

echo; echo

echo "Enter another string terminated by a \\ , then press <ENTER>."
read -r var2  # The -r option causes the "\" to be read literally.
              #     first line \

echo "var2 = $var2"
#     var2 = first line \

# Data entry terminates with the first <ENTER>.

echo 

exit 0
```

Without `-r`, `read` takes a `\` at the end of a line to mean "carry on on
the next line". Python's `readline` takes lines as they are, which is
`read -r`; a small function does the joining.

```tessh
import sys io
sys.stdin = io.StringIO "first line \\\nsecond line\nfirst line \\\n"   # (Canned answers.)

def read_joined f=$sys.stdin
  # A line, carried on past each line ending in "\" (read without -r).
  line = (($f.readline).rstrip "\n")
  while $line.endswith "\\"
    line = $line[:-1] + (($f.readline).rstrip "\n")
  $line

print

print "Enter a string terminated by a \\, then press <ENTER>."
print "Then, enter a second string (no \\ this time), and again press <ENTER>."

var1 = (read_joined)   # The "\" suppresses the newline, when reading $var1.
                       #     first line \
                       #     second line

print "var1 = $var1"
#     var1 = first line second line

print ; print

print "Enter another string terminated by a \\ , then press <ENTER>."
var2 = (($sys.stdin.readline).rstrip "\n")   # Read as it is, like read -r.
                                             #     first line \

print "var2 = $var2"
#     var2 = first line \

print

exit 0
```

```output

Enter a string terminated by a \, then press <ENTER>.
Then, enter a second string (no \ this time), and again press <ENTER>.
var1 = first line second line


Enter another string terminated by a \ , then press <ENTER>.
var2 = first line \

```

`read` has options for a prompt (`-p`), for not showing what's typed
(`-s`), for taking a number of keystrokes without waiting for Enter
(`-n N`), for a time limit (`-t`) and for reading another file descriptor
(`-u`).

```bash no-run
# Read a keypress without hitting ENTER.

read -s -n1 -p "Hit a key " keypress
echo; echo "Keypress was "\"$keypress\""."

# -s option means do not echo input.
# -n N option means accept only N characters of input.
# -p option means echo the following prompt before reading input.

# Using these options is tricky, since they need to be in the correct order.
```

In Python these take the terminal module: `tty.setcbreak` hands over each
key as it's pressed, without showing it, and `termios` puts the terminal
back after.

```tessh no-run
import sys os tty termios

def read_keys n prompt=""
  # n keystrokes, without Enter and without echoing them (read -s -n n -p prompt).
  print $prompt end="" flush=True
  fd = ($sys.stdin.fileno)
  saved = termios.tcgetattr $fd
  try
    tty.setcbreak $fd
    (os.read $fd $n).decode
  finally
    termios.tcsetattr $fd $termios.TCSADRAIN $saved

keypress = read_keys 1 "Hit a key "
print ; print "Keypress was \"$keypress\"."
```

(`getpass.getpass` reads a whole line without echoing it, and
`select.select` waits for input with a time limit.)

**Gap:** `read`'s `-s`, `-n`, `-t` and `-p` have no short tessh form; the
terminal has to be set up by hand with `tty` and `termios`, as above.

### Example 15-6. Detecting the arrow keys (`arrow-detect.sh`)

```bash no-run
#!/bin/bash
# arrow-detect.sh: Detects the arrow keys, and a few more.
# Thank you, Sandro Magi, for showing me how.

# --------------------------------------------
# Character codes generated by the keypresses.
arrowup='\[A'
arrowdown='\[B'
arrowrt='\[C'
arrowleft='\[D'
insert='\[2'
delete='\[3'
# --------------------------------------------

SUCCESS=0
OTHER=65

echo -n "Press a key...  "
# May need to also press ENTER if a key not listed above pressed.
read -n3 key                      # Read 3 characters.

echo -n "$key" | grep "$arrowup"  #Check if character code detected.
if [ "$?" -eq $SUCCESS ]
then
  echo "Up-arrow key pressed."
  exit $SUCCESS
fi

echo -n "$key" | grep "$arrowdown"
if [ "$?" -eq $SUCCESS ]
then
  echo "Down-arrow key pressed."
  exit $SUCCESS
fi

echo -n "$key" | grep "$arrowrt"
if [ "$?" -eq $SUCCESS ]
then
  echo "Right-arrow key pressed."
  exit $SUCCESS
fi

echo -n "$key" | grep "$arrowleft"
if [ "$?" -eq $SUCCESS ]
then
  echo "Left-arrow key pressed."
  exit $SUCCESS
fi

echo -n "$key" | grep "$insert"
if [ "$?" -eq $SUCCESS ]
then
  echo "\"Insert\" key pressed."
  exit $SUCCESS
fi

echo -n "$key" | grep "$delete"
if [ "$?" -eq $SUCCESS ]
then
  echo "\"Delete\" key pressed."
  exit $SUCCESS
fi


echo " Some other key pressed."

exit $OTHER

# ========================================= #

#  Mark Alexander came up with a simplified
#+ version of the above script (Thank you!).
#  It eliminates the need for grep.

#!/bin/bash

  uparrow=$'\x1b[A'
  downarrow=$'\x1b[B'
  leftarrow=$'\x1b[D'
  rightarrow=$'\x1b[C'

  read -s -n3 -p "Hit an arrow key: " x

  case "$x" in
  $uparrow)
     echo "You pressed up-arrow"
     ;;
  $downarrow)
     echo "You pressed down-arrow"
     ;;
  $leftarrow)
     echo "You pressed left-arrow"
     ;;
  $rightarrow)
     echo "You pressed right-arrow"
     ;;
  esac

exit $?

# ========================================= #

# Antonio Macchi has a simpler alternative.

#!/bin/bash

while true
do
  read -sn1 a
  test "$a" == `echo -en "\e"` || continue
  read -sn1 a
  test "$a" == "[" || continue
  read -sn1 a
  case "$a" in
    A)  echo "up";;
    B)  echo "down";;
    C)  echo "right";;
    D)  echo "left";;
  esac
done

# ========================================= #

#  Exercise:
#  --------
#  1) Add detection of the "Home," "End," "PgUp," and "PgDn" keys.
```

The keys here are canned (an arrow key sends three characters: Escape, `[`
and a letter); read them from the keyboard with `read_keys 3` above.
Rather than one `grep` per key, the codes go in a list and a loop tests
them with `in`.

```tessh
# arrow-detect.tsh: Detects the arrow keys, and a few more.

# --------------------------------------------
# Character codes generated by the keypresses.
keys = [
  ["\x1b[A" "Up-arrow"]
  ["\x1b[B" "Down-arrow"]
  ["\x1b[C" "Right-arrow"]
  ["\x1b[D" "Left-arrow"]
  ["\x1b[2" "\"Insert\""]
  ["\x1b[3" "\"Delete\""]
]
# --------------------------------------------

SUCCESS = 0
OTHER = 65

def detect key
  for [code name] in $keys
    if $code in $key                 # Check if character code detected.
      print "$name key pressed."
      return $SUCCESS
  print " Some other key pressed."
  $OTHER

for key in ["\x1b[A" "\x1b[3~" "\x1b[D" "x\n"]   # (Canned keypresses.)
  print "Press a key...  " end=""
  detect $key
```

```output
Press a key...  Up-arrow key pressed.
Press a key...  "Delete" key pressed.
Press a key...  Left-arrow key pressed.
Press a key...   Some other key pressed.
```

Mark Alexander's shorter version, in the same file, is a `case` on the
three characters; that's tessh's `match`, with the codes as patterns:

```tessh
def arrow x
  match $x
    case "\x1b[A"
      print "You pressed up-arrow"
    case "\x1b[B"
      print "You pressed down-arrow"
    case "\x1b[D"
      print "You pressed left-arrow"
    case "\x1b[C"
      print "You pressed right-arrow"

arrow "\x1b[D"
```

```output
You pressed left-arrow
```

`read` also reads from a file redirected into it. Only the first line is
read, and when it's given several names, the line is split among them.

### Example 15-7. Using read with file redirection (`read-redir.sh`)

```bash no-run
#!/bin/bash

read var1 <data-file
echo "var1 = $var1"
# var1 set to the entire first line of the input file "data-file"

read var2 var3 <data-file
echo "var2 = $var2   var3 = $var3"
# Note non-intuitive behavior of "read" here.
# 1) Rewinds back to the beginning of input file.
# 2) Each variable is now set to a corresponding string,
#    separated by whitespace, rather than to an entire line of text.
# 3) The final variable gets the remainder of the line.
# 4) If there are more variables to be set than whitespace-terminated strings
#    on the first line of the file, then the excess variables remain empty.

echo "------------------------------------------------"

# How to resolve the above problem with a loop:
while read line
do
  echo "$line"
done <data-file
# Thanks, Heiner Steven for pointing this out.

echo "------------------------------------------------"

# Use $IFS (Internal Field Separator variable) to split a line of input to
# "read", if you do not want the default to be whitespace.

echo "List of all users:"
OIFS=$IFS; IFS=:       # /etc/passwd uses ":" for field separator.
while read name passwd uid gid fullname ignore
do
  echo "$name ($fullname)"
done </etc/passwd   # I/O redirection.
IFS=$OIFS              # Restore original $IFS.
# This code snippet also by Heiner Steven.



#  Setting the $IFS variable within the loop itself
#+ eliminates the need for storing the original $IFS
#+ in a temporary variable.
#  Thanks, Dim Segebart, for pointing this out.
echo "------------------------------------------------"
echo "List of all users:"

while IFS=: read name passwd uid gid fullname ignore
do
  echo "$name ($fullname)"
done </etc/passwd   # I/O redirection.

echo
echo "\$IFS still $IFS"

exit 0
```

In tessh a file is read with `open` (or `cat` as a value): `.readline`
for one line, a `for` for all of them, `.split` to break a line at a
separator. There is no `$IFS`, so there's nothing to save and restore: the
separator is the argument to `.split`. (The users come from a made-up
`passwd` file, not the real `/etc/passwd`.)

```tessh
# (A made-up data file and /etc/passwd to try it on.)
print "This is line one of the data file.\nAnd this is line two." >data-file
print "root:x:0:0:root:/root:/bin/bash\nbozo:x:1000:100:Bozo Bozeman:/home/bozo:/bin/bash" >passwd

var1 = -> (open data-file) .readline .strip
print "var1 = $var1"
# var1 set to the entire first line of the input file "data-file"

[var2 var3] = -> (open data-file) .readline .strip (.split None 1)
print "var2 = $var2   var3 = $var3"
# Each variable is set to a whitespace-separated part; the last gets the rest.

print "------------------------------------------------"

# All the lines, with a loop:
for line in (open data-file)
  print ($line.strip)

print "------------------------------------------------"

# Split a line at ":" rather than at whitespace.

print "List of all users:"
for line in (open passwd)                      # (/etc/passwd in the original)
  [name passwd uid gid fullname *ignore] = $line.split ":"
  print "$name ($fullname)"

exit 0
```

```output
var1 = This is line one of the data file.
var2 = This   var3 = is line one of the data file.
------------------------------------------------
This is line one of the data file.
And this is line two.
------------------------------------------------
List of all users:
root (root)
bozo (Bozo Bozeman)
```

The ABS Guide warns that piping into `read` (`echo … | read var`) doesn't
set `var`, and that a `while read` loop at the end of a pipe runs in a
subshell, so what it sets is lost when the loop ends:

### Example 15-8. Problems reading from a pipe (`readpipe.sh`)

```bash
#!/bin/sh
# readpipe.sh
# This example contributed by Bjon Eriksson.

### shopt -s lastpipe

last="(null)"
cat $0 |
while read line
do
    echo "{$line}"
    last=$line
done

echo
echo "++++++++++++++++++++++"
printf "\nAll done, last: $last\n" #  The output of this line
                                   #+ changes if you uncomment line 5.
                                   #  (Bash, version -ge 4.2 required.)

exit 0  # End of code.
        # (Partial) output of script follows.
        # The 'echo' supplies extra brackets.

#############################################

./readpipe.sh 

{#!/bin/sh}
{last="(null)"}
{cat $0 |}
{while read line}
{do}
{echo "{$line}"}
{last=$line}
{done}
{printf "nAll done, last: $lastn"}


All done, last: (null)

The variable (last) is set within the loop/subshell
but its value does not persist outside the loop.
```

```output bash
{#!/bin/sh}
{# readpipe.sh}
{# This example contributed by Bjon Eriksson.}
{}
{### shopt -s lastpipe}
{}
{last="(null)"}
{cat $0 |}
{while read line}
{do}
{echo "{$line}"}
{last=$line}
{done}
{}
{echo}
{echo "++++++++++++++++++++++"}
{printf "nAll done, last: $lastn" #  The output of this line}
{#+ changes if you uncomment line 5.}
{#  (Bash, version -ge 4.2 required.)}
{}
{exit 0  # End of code.}
{# (Partial) output of script follows.}
{# The 'echo' supplies extra brackets.}
{}
{#############################################}
{}
{./readpipe.sh}
{}
{{#!/bin/sh}}
{{last="(null)"}}
{{cat $0 |}}
{{while read line}}
{{do}}
{{echo "{$line}"}}
{{last=$line}}
{{done}}
{{printf "nAll done, last: $lastn"}}
{}
{}
{All done, last: (null)}
{}
{The variable (last) is set within the loop/subshell}
{but its value does not persist outside the loop.}

++++++++++++++++++++++

All done, last: (null)
```

A tessh `for` over a command's lines runs in the script itself, so `last`
keeps its value. (A Python function as a pipe stage also runs in the
script.) `read` without `-r` also eats the backslashes in the script's
lines, `\n` becoming `n`; `.lines` keeps them.

```tessh
# readpipe.tsh

last = "(null)"
for line in (cat $argv[0]).lines
  print "{$line}"
  last = $line

print
print "++++++++++++++++++++++"
print "\nAll done, last: $last"

exit 0
```

```output
{# readpipe.tsh}
{}
{last = "(null)"}
{for line in (cat $argv[0]).lines}
{  print "{$line}"}
{  last = $line}
{}
{print}
{print "++++++++++++++++++++++"}
{print "\nAll done, last: $last"}
{}
{exit 0}

++++++++++++++++++++++

All done, last: exit 0
```

## Filesystem
<!-- help: abs-builtins-filesystem cd pwd pushd popd dirs DIRSTACK -->

**cd**, **pwd**, **pushd**, **popd** and **dirs** are tessh builtins too,
with bash's options (`cd -P`, `cd -`, `pushd +N`, `dirs -v`…), and work as
the ABS Guide describes. `(cd /source && tar cf - .) | (cd /dest && tar xpf
-)` changes folder only for each side of the pipe; in tessh, `tar -C dir`
does the same. tessh's `pwd` is the program, run as a value: `(pwd)`.

### Example 15-9. Changing the current working directory (`ex37.sh`)

```bash no-run
#!/bin/bash

dir1=/usr/local
dir2=/var/spool

pushd $dir1
# Will do an automatic 'dirs' (list directory stack to stdout).
echo "Now in directory `pwd`." # Uses back-quoted 'pwd'.

# Now, do some stuff in directory 'dir1'.
pushd $dir2
echo "Now in directory `pwd`."

# Now, do some stuff in directory 'dir2'.
echo "The top entry in the DIRSTACK array is $DIRSTACK."
popd
echo "Now back in directory `pwd`."

# Now, do some more stuff in directory 'dir1'.
popd
echo "Now back in original working directory `pwd`."

exit 0

# What happens if you don't 'popd' -- then exit the script?
# Which directory do you end up in? Why?
```

The same commands, with the folder stack shown by `dirs` (`pushd` and
`popd` show it too). The script starts in `/` here, so the output is the
same wherever it runs.

```tessh
dir1 = /usr/local
dir2 = /var/spool
cd /                 # (To start somewhere fixed.)

pushd $dir1
# Will do an automatic 'dirs' (list directory stack to stdout).
print "Now in directory $(pwd)."

# Now, do some stuff in directory 'dir1'.
pushd $dir2
print "Now in directory $(pwd)."

# Now, do some stuff in directory 'dir2'.
print "The top entry in the stack is $(pwd)."   # (No $DIRSTACK: the top is the current folder.)
popd
print "Now back in directory $(pwd)."

# Now, do some more stuff in directory 'dir1'.
popd
print "Now back in original working directory $(pwd)."

exit 0
```

```output
/usr/local /
Now in directory /usr/local.
/var/spool /usr/local /
Now in directory /var/spool.
The top entry in the stack is /var/spool.
/usr/local /
Now back in directory /usr/local.
/
Now back in original working directory /.
```

## Variables
<!-- help: abs-builtins-variables let eval set unset export declare typeset readonly getopts OPTIND OPTARG shift -->

**let** does arithmetic on variables. tessh's arithmetic is Python's, with
`=`, `+=` and the rest of the augmented assignments; there's no `++` or
`--`, and `/` gives a float (`//` is bash's whole-number division).

### Example 15-10. Letting let do arithmetic. (`ex46.sh`)

```bash
#!/bin/bash

echo

let a=11            # Same as 'a=11'
let a=a+5           # Equivalent to  let "a = a + 5"
                    # (Double quotes and spaces make it more readable.)
echo "11 + 5 = $a"  # 16

let "a <<= 3"       # Equivalent to  let "a = a << 3"
echo "\"\$a\" (=16) left-shifted 3 places = $a"
                    # 128

let "a /= 4"        # Equivalent to  let "a = a / 4"
echo "128 / 4 = $a" # 32

let "a -= 5"        # Equivalent to  let "a = a - 5"
echo "32 - 5 = $a"  # 27

let "a *=  10"      # Equivalent to  let "a = a * 10"
echo "27 * 10 = $a" # 270

let "a %= 8"        # Equivalent to  let "a = a % 8"
echo "270 modulo 8 = $a  (270 / 8 = 33, remainder $a)"
                    # 6


# Does "let" permit C-style operators?
# Yes, just as the (( ... )) double-parentheses construct does.

let a++             # C-style (post) increment.
echo "6++ = $a"     # 6++ = 7
let a--             # C-style decrement.
echo "7-- = $a"     # 7-- = 6
# Of course, ++a, etc., also allowed . . .
echo


# Trinary operator.

# Note that $a is 6, see above.
let "t = a<7?7:11"   # True
echo $t  # 7

let a++
let "t = a<7?7:11"   # False
echo $t  #     11

exit
```

```tessh
print

a = 11              # Same as 'let a=11'
a = $a + 5          # Equivalent to  let "a = a + 5"
print "11 + 5 = $a"  # 16

a <<= 3             # Equivalent to  let "a = a << 3"
print "\"\$a\" (=16) left-shifted 3 places = $a"
                    # 128

a //= 4             # Equivalent to  let "a = a / 4" (// keeps it whole)
print "128 / 4 = $a" # 32

a -= 5              # Equivalent to  let "a = a - 5"
print "32 - 5 = $a"  # 27

a *= 10             # Equivalent to  let "a = a * 10"
print "27 * 10 = $a" # 270

a %= 8              # Equivalent to  let "a = a % 8"
print "270 modulo 8 = $a  (270 / 8 = 33, remainder $a)"
                    # 6

# C-style ++ and --: += 1 and -= 1.
a += 1
print "6++ = $a"     # 6++ = 7
a -= 1
print "7-- = $a"     # 7-- = 6
print

# Trinary operator: if … then … else.

# Note that $a is 6, see above.
t = if $a < 7 then 7 else 11   # True
print $t  # 7

a += 1
t = if $a < 7 then 7 else 11   # False
print $t  #     11

exit
```

```output

11 + 5 = 16
"$a" (=16) left-shifted 3 places = 128
128 / 4 = 32
32 - 5 = 27
27 * 10 = 270
270 modulo 8 = 6  (270 / 8 = 33, remainder 6)
6++ = 7
7-- = 6

7
11
```

`let`'s exit status is 1 when the last value it works out is 0, so `let
var++` with `var` at 0 "fails". An assignment in tessh has no status.

**eval** puts its arguments together into a line and runs that as a
command, after expanding the variables in it: text becomes code. Each `eval`
expands once more: with `a='$b' b='$c' c=d`, `echo $a` prints `$b`, `eval
echo $a` prints `$c` and `eval eval echo $a` prints `d`.

tessh has no `eval` for tessh text. Python's `exec` and `eval` run Python
text, `source` runs a file of tessh, and most of what scripts use `eval` for
(choosing a variable by name, building a command) is done more safely with
a dict, a list of arguments, or `$env`.

### Example 15-11. Showing the effect of eval (`ex43.sh`)

```bash no-run
#!/bin/bash
# Exercising "eval" ...

y=`eval ls -l`  #  Similar to y=`ls -l`
echo $y         #+ but linefeeds removed because "echoed" variable is unquoted.
echo
echo "$y"       #  Linefeeds preserved when variable is quoted.

echo; echo

y=`eval df`     #  Similar to y=`df`
echo $y         #+ but linefeeds removed.

#  When LF's not preserved, it may make it easier to parse output,
#+ using utilities such as "awk".

echo
echo "==========================================================="
echo

eval "`seq 3 | sed -e 's/.*/echo var&=ABCDEFGHIJ/'`"
# var1=ABCDEFGHIJ
# var2=ABCDEFGHIJ
# var3=ABCDEFGHIJ

echo
echo "==========================================================="
echo


# Now, showing how to do something useful with "eval" . . .
# (Thank you, E. Choroba!)

version=3.4     #  Can we split the version into major and minor
                #+ part in one command?
echo "version = $version"
eval major=${version/./;minor=}     #  Replaces '.' in version by ';minor='
                                    #  The substitution yields '3; minor=4'
                                    #+ so eval does minor=4, major=3
echo Major: $major, minor: $minor   #  Major: 3, minor: 4
```

A command's output is a value without `eval`. To run lines of tessh that a
command printed, write them to a file and `source` it. Splitting a version
number into two names is `.split` and unpacking.

```tessh
# Exercising "eval" ...

y = ls -l           #  (A value, eval or not.)
print (" ".join ($y.split))   # Words, like bash's unquoted echo $y.
print
print $y            #  Linefeeds preserved.

print ; print

y = df --output=target /   #  (df in the original: its numbers change.)
print (" ".join ($y.split))

print
print "==========================================================="
print

code = seq 3 | sed -e 's/.*/echo var&=ABCDEFGHIJ/'
print $code >generated.tsh     # (No eval of tessh text: source a file of it.)
source generated.tsh
# var1=ABCDEFGHIJ
# var2=ABCDEFGHIJ
# var3=ABCDEFGHIJ

print
print "==========================================================="
print

version = "3.4"     #  Can we split the version into major and minor
                    #+ part in one command?
print "version = $version"
[major minor] = $version.split "."
print "Major: $major, minor: $minor"   #  Major: 3, minor: 4
```

```output
total 0

total 0


Mounted on /

===========================================================

var1=ABCDEFGHIJ
var2=ABCDEFGHIJ
var3=ABCDEFGHIJ

===========================================================

version = 3.4
Major: 3, minor: 4
```

**Gap:** no `eval` of tessh text: Python's `exec` runs Python, and
`source` needs a file.

### Example 15-12. Using eval to select among variables (`arr-choice.sh`)

```bash
#!/bin/bash
# arr-choice.sh

#  Passing arguments to a function to select
#+ one particular variable out of a group.

arr0=( 10 11 12 13 14 15 )
arr1=( 20 21 22 23 24 25 )
arr2=( 30 31 32 33 34 35 )
#       0  1  2  3  4  5      Element number (zero-indexed)


choose_array ()
{
  eval array_member=\${arr${array_number}[element_number]}
  #                 ^       ^^^^^^^^^^^^
  #  Using eval to construct the name of a variable,
  #+ in this particular case, an array name.

  echo "Element $element_number of array $array_number is $array_member"
} #  Function can be rewritten to take parameters.

array_number=0    # First array.
element_number=3
choose_array      # 13

array_number=2    # Third array.
element_number=4
choose_array      # 34

array_number=3    # Null array (arr3 not allocated).
element_number=4
choose_array      # (null)

# Thank you, Antonio Macchi, for pointing this out.
```

Variables picked by a number in their name are a list (or a dict) picked
by index. A missing array is a missing item, `None` from `.get` (and here
`""`, as bash prints nothing).

```tessh
# arr-choice.tsh

#  Passing arguments to a function to select
#+ one particular list out of a group.

arr = {
  0: [10 11 12 13 14 15]
  1: [20 21 22 23 24 25]
  2: [30 31 32 33 34 35]
}
#     0  1  2  3  4  5      Element number (zero-indexed)

def choose_array array_number element_number
  array_member = ($arr.get $array_number [])[$element_number:][:1]
  array_member = if $array_member then $array_member[0] else ""
  print "Element $element_number of array $array_number is $array_member"

choose_array 0 3    # 13
choose_array 2 4    # 34
choose_array 3 4    # (null)
```

```output
Element 3 of array 0 is 13
Element 4 of array 2 is 34
Element 4 of array 3 is 
```

### Example 15-13. Echoing the command-line parameters (`echo-params.sh`)

```bash no-run
#!/bin/bash
# echo-params.sh

# Call this script with a few command-line parameters.
# For example:
#     sh echo-params.sh first second third fourth fifth

params=$#              # Number of command-line parameters.
param=1                # Start at first command-line param.

while [ "$param" -le "$params" ]
do
  echo -n "Command-line parameter "
  echo -n \$$param     #  Gives only the *name* of variable.
#         ^^^          #  $1, $2, $3, etc.
                       #  Why?
                       #  \$ escapes the first "$"
                       #+ so it echoes literally,
                       #+ and $param dereferences "$param" . . .
                       #+ . . . as expected.
  echo -n " = "
  eval echo \$$param   #  Gives the *value* of variable.
# ^^^^      ^^^        #  The "eval" forces the *evaluation*
                       #+ of \$$
                       #+ as an indirect variable reference.

(( param ++ ))         # On to the next.
done

exit $?

# =================================================

$ sh echo-params.sh first second third fourth fifth
Command-line parameter $1 = first
Command-line parameter $2 = second
Command-line parameter $3 = third
Command-line parameter $4 = fourth
Command-line parameter $5 = fifth
```

`$argv` is a list, so the indirect `\$$param` is `$argv[$param]`.

```tessh
# echo-params.tsh

args = $argv[1:] or [first second third fourth fifth]   # (sample arguments, when none are given)

params = len $args     # Number of command-line parameters.
param = 1              # Start at first command-line param.

while $param <= $params
  print "Command-line parameter \$$param = ${$args[$param - 1]}"
  param += 1           # On to the next.

exit 0
```

```output
Command-line parameter $1 = first
Command-line parameter $2 = second
Command-line parameter $3 = third
Command-line parameter $4 = fourth
Command-line parameter $5 = fifth
```

### Example 15-14. Forcing a log-off (`ex44.sh`)

```bash no-run
#!/bin/bash
# Killing ppp to force a log-off.
# For dialup connection, of course.

# Script should be run as root user.

SERPORT=ttyS3
#  Depending on the hardware and even the kernel version,
#+ the modem port on your machine may be different --
#+ /dev/ttyS1 or /dev/ttyS2.


killppp="eval kill -9 `ps ax | awk '/ppp/ { print $1 }'`"
#                     -------- process ID of ppp -------  

$killppp                     # This variable is now a command.


# The following operations must be done as root user.

chmod 666 /dev/$SERPORT      # Restore r+w permissions, or else what?
#  Since doing a SIGKILL on ppp changed the permissions on the serial port,
#+ we restore permissions to previous state.

rm /var/lock/LCK..$SERPORT   # Remove the serial port lock file. Why?

exit $?

# Exercises:
# ---------
# 1) Have script check whether root user is invoking it.
# 2) Do a check on whether the process to be killed
#+   is actually running before attempting to kill it.   
# 3) Write an alternate version of this script based on 'fuser':
#+      if [ fuser -s /dev/modem ]; then . . .
```

This kills processes, changes `/dev` and deletes a lock file, so it can't run
here. The process IDs come from `ps` and `awk`, as a list: no `eval`, since
`kill` takes the list as its arguments.

```tessh no-run
import os
# Killing ppp to force a log-off.
# Script should be run as root user.

SERPORT = ttyS3

killppp = (ps ax | awk '/ppp/ { print $1 }').lines   # Process IDs of ppp.

kill -9 $killppp

# The following operations must be done as root user.

chmod 666 /dev/$SERPORT      # Restore r+w permissions, or else what?
rm /var/lock/LCK..$SERPORT   # Remove the serial port lock file. Why?

exit 0
```

### Example 15-15. A version of rot13 (`rot14.sh`)

```bash
#!/bin/bash
# A version of "rot13" using 'eval'.
# Compare to "rot13.sh" example.

setvar_rot_13()              # "rot13" scrambling
{
  local varname=$1 varvalue=$2
  eval $varname='$(echo "$varvalue" | tr a-z n-za-m)'
}


setvar_rot_13 var "foobar"   # Run "foobar" through rot13.
echo $var                    # sbbone

setvar_rot_13 var "$var"     # Run "sbbone" through rot13.
                             # Back to original variable.
echo $var                    # foobar

# This example by Stephane Chazelas.
# Modified by document author.

exit 0
```

The `eval` sets a variable whose name is in `$varname`. In tessh that's the
module's `globals` dict. (A function that returns the scrambled text,
`var = rot13 $var`, is the plainer way.)

```tessh
# A version of "rot13" setting a variable by name.

def setvar_rot_13 varname varvalue    # "rot13" scrambling
  (globals)[$varname] = echo $varvalue | tr a-z n-za-m

setvar_rot_13 var "foobar"   # Run "foobar" through rot13.
print $var                   # sbbone

setvar_rot_13 var $var       # Run "sbbone" through rot13.
                             # Back to original variable.
print $var                   # foobar

exit 0
```

```output
sbbone
foobar
```

**set** changes the shell's options (`set -e`, `set -x`…), and with
arguments sets the positional parameters `$1`, `$2`…, often to the words of
a command's output: `set $(uname -a)`. With no arguments it lists the
variables.

tessh's arguments are `$argv`, a list, and any list can be split from a
command's output: `fields = ((uname -a).split)`. There's no need to reuse the
positional parameters. `(globals)` holds the script's variables and `$env`
the environment.

### Example 15-16. Using set with positional parameters (`ex34.sh`)

```bash no-run
#!/bin/bash
# ex34.sh
# Script "set-test"

# Invoke this script with three command-line parameters,
# for example, "sh ex34.sh one two three".

echo
echo "Positional parameters before  set \`uname -a\` :"
echo "Command-line argument #1 = $1"
echo "Command-line argument #2 = $2"
echo "Command-line argument #3 = $3"


set `uname -a` # Sets the positional parameters to the output
               # of the command `uname -a`

echo
echo +++++
echo $_        # +++++
# Flags set in script.
echo $-        # hB
#                Anomalous behavior?
echo

echo "Positional parameters after  set \`uname -a\` :"
# $1, $2, $3, etc. reinitialized to result of `uname -a`
echo "Field #1 of 'uname -a' = $1"
echo "Field #2 of 'uname -a' = $2"
echo "Field #3 of 'uname -a' = $3"
echo \#\#\#
echo $_        # ###
echo

exit 0
```

```tessh
# ex34.tsh
# Script "set-test"

args = $argv[1:] or [one two three]   # (sample arguments, when none are given)

print
print "Arguments before setting them to the fields of `uname -a` :"
print "Command-line argument #1 = $args[0]"
print "Command-line argument #2 = $args[1]"
print "Command-line argument #3 = $args[2]"

args = ((uname -a).split)   # A list of the words of `uname -a`.

print

print "Arguments after setting them to the fields of `uname -a` :"
print "Field #1 of 'uname -a' = $args[0]"
print "Field #2 of 'uname -a' = $args[1]"
print "Field #3 of 'uname -a' = $args[2]"
print

exit 0
```

```text
(one run)

Arguments before setting them to the fields of `uname -a` :
Command-line argument #1 = one
Command-line argument #2 = two
Command-line argument #3 = three

Arguments after setting them to the fields of `uname -a` :
Field #1 of 'uname -a' = Linux
Field #2 of 'uname -a' = nixos
Field #3 of 'uname -a' = 7.3.5

```

**Gap:** no `$_` (the last argument of the command before) and no `$-` (the
shell's option flags).

### Example 15-17. Reversing the positional parameters (`revposparams.sh`)

```bash
#!/bin/bash
# revposparams.sh: Reverse positional parameters.
# Script by Dan Jacobson, with stylistic revisions by document author.


set a\ b c d\ e;
#     ^      ^     Spaces escaped 
#       ^ ^        Spaces not escaped
OIFS=$IFS; IFS=:;
#              ^   Saving old IFS and setting new one.

echo

until [ $# -eq 0 ]
do          #      Step through positional parameters.
  echo "### k0 = "$k""     # Before
  k=$1:$k;  #      Append each pos param to loop variable.
#     ^
  echo "### k = "$k""      # After
  echo
  shift;
done

set $k  #  Set new positional parameters.
echo -
echo $# #  Count of positional parameters.
echo -
echo

for i   #  Omitting the "in list" sets the variable -- i --
        #+ to the positional parameters.
do
  echo $i  # Display new positional parameters.
done

IFS=$OIFS  # Restore IFS.

#  Question:
#  Is it necessary to set an new IFS, internal field separator,
#+ in order for this script to work properly?
#  What happens if you don't? Try it.
#  And, why use the new IFS -- a colon -- in line 17,
#+ to append to the loop variable?
#  What is the purpose of this?

exit 0

$ ./revposparams.sh

### k0 = 
### k = a b

### k0 = a b
### k = c a b

### k0 = c a b
### k = d e c a b

-
3
-

d e
c
a b
```

```output bash

### k0 = 
### k = a b 

### k0 = a b 
### k = c a b 

### k0 = c a b 
### k = d e c a b 

-
3
-

d e
c
a b
```

`shift` drops the first argument; on a list, that's `.pop 0` or a slice.
Splitting `$k` at `:` is `.split ":"`, which keeps the empty field after the
last `:` that bash's word splitting drops, so the empty ones are filtered
out.

```tessh
# revposparams.tsh: Reverse positional parameters.

params = ["a b" c "d e"]

print

k = ""
while $params            #      Step through positional parameters.
  print "### k0 = $k"    # Before
  k = $params[0] + ":" + $k   # Append each pos param to loop variable.
  print "### k = $k"     # After
  print
  $params.pop 0          # (shift)

params = [$p for p in ($k.split ":") if $p]   #  Set new positional parameters.
print -
print (len $params)      #  Count of positional parameters.
print -
print

for i in $params
  print $i               # Display new positional parameters.

exit 0
```

```output

### k0 = 
### k = a b:

### k0 = a b:
### k = c:a b:

### k0 = c:a b:
### k = d e:c:a b:

-
3
-

d e
c
a b
```

The ABS Guide's output shows `k` as `a b` and `c a b`: `echo "### k = "$k""`
leaves `$k` unquoted, so with `IFS=:` its colons become spaces. tessh prints
the value as it is.

**set --** sets the positional parameters to a variable's words, or clears
them with nothing after it, and **shift** moves them along.

### Example 15-18. Reassigning the positional parameters (`set-pos.sh`)

```bash
#!/bin/bash

variable="one two three four five"

set -- $variable
# Sets positional parameters to the contents of "$variable".

first_param=$1
second_param=$2
shift; shift        # Shift past first two positional params.
# shift 2             also works.
remaining_params="$*"

echo
echo "first parameter = $first_param"             # one
echo "second parameter = $second_param"           # two
echo "remaining parameters = $remaining_params"   # three four five

echo; echo

# Again.
set -- $variable
first_param=$1
second_param=$2
echo "first parameter = $first_param"             # one
echo "second parameter = $second_param"           # two

# ======================================================

set --
# Unsets positional parameters if no variable specified.

first_param=$1
second_param=$2
echo "first parameter = $first_param"             # (null value)
echo "second parameter = $second_param"           # (null value)

exit 0
```

A list of the words, and slices of it: `$params[2:]` is what's left after
two `shift`s. A missing parameter is `""` in bash; in Python indexing past
the end is an error, so the empty case takes defaults.

```tessh
variable = "one two three four five"

params = ($variable.split)
# A list of the words of "$variable".

first_param = $params[0]
second_param = $params[1]
remaining_params = " ".join $params[2:]   # Past the first two (shift 2).

print
print "first parameter = $first_param"             # one
print "second parameter = $second_param"           # two
print "remaining parameters = $remaining_params"   # three four five

print ; print

# Again.
params = ($variable.split)
[first_param second_param *_] = $params
print "first parameter = $first_param"             # one
print "second parameter = $second_param"           # two

# ======================================================

params = []
# An empty list: no parameters.

[first_param second_param *_] = $params + ["" ""]   # ("" for the missing ones)
print "first parameter = $first_param"             # (null value)
print "second parameter = $second_param"           # (null value)

exit 0
```

```output

first parameter = one
second parameter = two
remaining parameters = three four five


first parameter = one
second parameter = two
first parameter = 
second parameter = 
```

**unset** deletes a shell variable. In tessh, `unset` is for environment
variables and `del` removes a tessh variable; reading it after that is a
`NameError`, not an empty string. `(globals).get "name" ""` reads one that
may not be there.

### Example 15-19. "Unsetting" a variable (`unset.sh`)

```bash
#!/bin/bash
# unset.sh: Unsetting a variable.

variable=hello                       #  Initialized.
echo "variable = $variable"

unset variable                       #  Unset.
                                     #  In this particular context,
                                     #+ same effect as:   variable=
echo "(unset) variable = $variable"  #  $variable is null.

if [ -z "$variable" ]                #  Try a string-length test.
then
  echo "\$variable has zero length."
fi

exit 0
```

```tessh
# unset.tsh: Deleting a variable.

variable = hello                     #  Initialized.
print "variable = $variable"

del variable                         #  Deleted.
value = (globals).get "variable" ""  #  ("" now that it's gone.)
print "(unset) variable = $value"

if not $value                        #  Empty?
  print "\$variable has zero length."

exit 0
```

```output
variable = hello
(unset) variable = 
$variable has zero length.
```

**export** puts a variable in the environment, for every program the script
runs from then on (never back to the shell that ran the script). tessh's
`export` is the same; `export name` alone exports the tessh variable's
value, and `$env.name = value` does it from Python.

### Example 15-20. Using export to pass a variable to an embedded awk script (`col-totaler3.sh`)

```bash no-run
#!/bin/bash

#  Yet another version of the "column totaler" script (col-totaler.sh)
#+ that adds up a specified column (of numbers) in the target file.
#  This uses the environment to pass a script variable to 'awk' . . .
#+ and places the awk script in a variable.


ARGS=2
E_WRONGARGS=85

if [ $# -ne "$ARGS" ] # Check for proper number of command-line args.
then
   echo "Usage: `basename $0` filename column-number"
   exit $E_WRONGARGS
fi

filename=$1
column_number=$2

#===== Same as original script, up to this point =====#

export column_number
# Export column number to environment, so it's available for retrieval.


# -----------------------------------------------
awkscript='{ total += $ENVIRON["column_number"] }
END { print total }'
# Yes, a variable can hold an awk script.
# -----------------------------------------------

# Now, run the awk script.
awk "$awkscript" "$filename"

# Thanks, Stephane Chazelas.

exit 0
```

`awk` reads the variable from its environment, as in bash. The awk script
is in single quotes, so tessh leaves its `$` alone.

```tessh
# (A made-up file of numbers to try it on.)
print "1 10 100\n2 20 200\n3 30 300" >numbers

ARGS = 2
E_WRONGARGS = 85

args = $argv[1:] or [numbers 2]   # (sample arguments, when none are given)

if (len $args) != $ARGS           # Check for proper number of command-line args.
  print "Usage: $(Path $argv[0]).name filename column-number"
  exit $E_WRONGARGS

[filename column_number] = $args

export column_number
# Export column number to environment, so it's available for retrieval.

# -----------------------------------------------
awkscript = '{ total += $ENVIRON["column_number"] }
END { print total }'
# Yes, a variable can hold an awk script.
# -----------------------------------------------

# Now, run the awk script.
awk $awkscript $filename

exit 0
```

```output
60
```

**declare** and **typeset** give a variable properties (integer, array,
read-only, exported), and **readonly** is `declare -r`. tessh's values
carry their types themselves (an `int` is an int, a list a list), `export`
covers exporting, and there's no read-only (see the gap under Example
15-2).

**getopts** reads a script's options one at a time: letters after a `-`,
several in one word (`-abc`), a letter followed by `:` in its list taking an
argument. `$OPTARG` holds the argument and `$OPTIND` the index of the next
word.

```bash no-run
while getopts ":abcde:fg" Option
# Initial declaration.
# a, b, c, d, e, f, and g are the options (flags) expected.
# The : after option 'e' shows it will have an argument passed with it.
do
  case $Option in
    a ) # Do something with variable 'a'.
    b ) # Do something with variable 'b'.
    ...
    e)  # Do something with 'e', and also with $OPTARG,
        # which is the associated argument passed with option 'e'.
    ...
    g ) # Do something with variable 'g'.
  esac
done
shift $(($OPTIND - 1))
# Move argument pointer to next.

# All this is not nearly as complicated as it looks <grin>.
```

Python's `getopt` module takes the same option string and gives back the
options found, each with its argument, and the arguments left after them
(so no `$OPTIND` and no `shift`). An unknown option is a `GetoptError`.
`argparse` does more: long options, `--help`, types and defaults.

### Example 15-21. Using getopts to read the options/arguments passed to a script (`ex33.sh`)

```bash no-run
#!/bin/bash
# ex33.sh: Exercising getopts and OPTIND
#          Script modified 10/09/03 at the suggestion of Bill Gradwohl.


# Here we observe how 'getopts' processes command-line arguments to script.
# The arguments are parsed as "options" (flags) and associated arguments.

# Try invoking this script with:
#   'scriptname -mn'
#   'scriptname -oq qOption' (qOption can be some arbitrary string.)
#   'scriptname -qXXX -r'
#
#   'scriptname -qr'
#+      - Unexpected result, takes "r" as the argument to option "q"
#   'scriptname -q -r' 
#+      - Unexpected result, same as above
#   'scriptname -mnop -mnop'  - Unexpected result
#   (OPTIND is unreliable at stating where an option came from.)
#
#  If an option expects an argument ("flag:"), then it will grab
#+ whatever is next on the command-line.

NO_ARGS=0 
E_OPTERROR=85

if [ $# -eq "$NO_ARGS" ]    # Script invoked with no command-line args?
then
  echo "Usage: `basename $0` options (-mnopqrs)"
  exit $E_OPTERROR          # Exit and explain usage.
                            # Usage: scriptname -options
                            # Note: dash (-) necessary
fi  


while getopts ":mnopq:rs" Option
do
  case $Option in
    m     ) echo "Scenario #1: option -m-   [OPTIND=${OPTIND}]";;
    n | o ) echo "Scenario #2: option -$Option-   [OPTIND=${OPTIND}]";;
    p     ) echo "Scenario #3: option -p-   [OPTIND=${OPTIND}]";;
    q     ) echo "Scenario #4: option -q-\
                  with argument \"$OPTARG\"   [OPTIND=${OPTIND}]";;
    #  Note that option 'q' must have an associated argument,
    #+ otherwise it falls through to the default.
    r | s ) echo "Scenario #5: option -$Option-";;
    *     ) echo "Unimplemented option chosen.";;   # Default.
  esac
done

shift $(($OPTIND - 1))
#  Decrements the argument pointer so it points to next argument.
#  $1 now references the first non-option item supplied on the command-line
#+ if one exists.

exit $?

#   As Bill Gradwohl states,
#  "The getopts mechanism allows one to specify:  scriptname -mnop -mnop
#+  but there is no reliable way to differentiate what came
#+ from where by using OPTIND."
#  There are, however, workarounds.
```

The script's body is a function here, run with the sample command lines
from the original's comments.

```tessh
import getopt
# ex33.tsh: Exercising getopt

NO_ARGS = 0
E_OPTERROR = 85

def main args
  if (len $args) == $NO_ARGS        # Script invoked with no command-line args?
    print "Usage: $(Path $argv[0]).name options (-mnopqrs)"
    return $E_OPTERROR              # Exit and explain usage.
  try
    [opts rest] = getopt.getopt $args "mnopq:rs"
  except getopt.GetoptError as e
    print "Unimplemented option chosen. ($e)"
    return $E_OPTERROR
  for [Option OPTARG] in $opts
    match $Option
      case "-m"
        print "Scenario #1: option -m-"
      case "-n" | "-o"
        print "Scenario #2: option $Option-"
      case "-p"
        print "Scenario #3: option -p-"
      case "-q"
        print "Scenario #4: option -q- with argument \"$OPTARG\""
      case "-r" | "-s"
        print "Scenario #5: option $Option-"
  # $rest holds the arguments after the options.
  0

for args in [[] [-mn] [-oq qOption] [-qXXX -r] [-qr] [-q -r] [-mnop -mnop] [-x]]   # (sample command lines)
  print "--- ex33" *$args
  main $args
```

```output
--- ex33
Usage: ex.tsh options (-mnopqrs)
--- ex33 -mn
Scenario #1: option -m-
Scenario #2: option -n-
--- ex33 -oq qOption
Scenario #2: option -o-
Scenario #4: option -q- with argument "qOption"
--- ex33 -qXXX -r
Scenario #4: option -q- with argument "XXX"
Scenario #5: option -r-
--- ex33 -qr
Scenario #4: option -q- with argument "r"
--- ex33 -q -r
Scenario #4: option -q- with argument "-r"
--- ex33 -mnop -mnop
Scenario #1: option -m-
Scenario #2: option -n-
Scenario #2: option -o-
Scenario #3: option -p-
Scenario #1: option -m-
Scenario #2: option -n-
Scenario #2: option -o-
Scenario #3: option -p-
--- ex33 -x
Unimplemented option chosen. (option -x not recognized)
```

As with `getopts`, `-qr` gives `r` to `-q` as its argument, and `-q -r`
gives it `-r`.

## Script behavior
<!-- help: abs-builtins-script source dot exit exec shopt caller LINENO -->

**source** (or **.**) runs a file in the current shell, as if its lines
were in the script: its variables and functions stay. tessh's `source`
does the same with a tessh file (`import` is the other way, keeping the
file's names in a module of their own).

### Example 15-22. "Including" a data file (`ex38.sh`, `data-file`)

```bash no-run
#!/bin/bash
#  Note that this example must be invoked with bash, i.e., bash ex38.sh
#+ not  sh ex38.sh !

. data-file    # Load a data file.
# Same effect as "source data-file", but more portable.

#  The file "data-file" must be present in current working directory,
#+ since it is referred to by its basename.

# Now, let's reference some data from that file.

echo "variable1 (from data-file) = $variable1"
echo "variable3 (from data-file) = $variable3"

let "sum = $variable2 + $variable4"
echo "Sum of variable2 + variable4 (from data-file) = $sum"
echo "message1 (from data-file) is \"$message1\""
#                                  Escaped quotes
echo "message2 (from data-file) is \"$message2\""

print_message This is the message-print function in the data-file.


exit $?
```

File `data-file` for the example above:

```bash no-run
# This is a data file loaded by a script.
# Files of this type may contain variables, functions, etc.
# It loads with a 'source' or '.' command from a shell script.

# Let's initialize some variables.

variable1=23
variable2=474
variable3=5
variable4=97

message1="Greetings from *** line $LINENO *** of the data file!"
message2="Enough for now. Goodbye."

print_message ()
{   # Echoes any message passed to it.

  if [ -z "$1" ]
  then
    return 1 # Error, if argument missing.
  fi

  echo

  until [ -z "$1" ]
  do             # Step through arguments passed to function.
    echo -n "$1" # Echo args one at a time, suppressing line feeds.
    echo -n " "  # Insert spaces between words.
    shift        # Next one.
  done  

  echo

  return 0
}
```

`$LINENO` is the line it's on; in tessh, ask Python for the line being run.

```tessh
# (The data file, in tessh.)
cat >data-file.tsh <<'END'
# This is a data file loaded by a script.
# Files of this type may contain variables, functions, etc.
# It loads with a 'source' command from a tessh script.
import inspect

# Let's initialize some variables.

variable1 = 23
variable2 = 474
variable3 = 5
variable4 = 97

message1 = "Greetings from *** line ${(inspect.currentframe).f_lineno} *** of the data file!"
message2 = "Enough for now. Goodbye."

def print_message *words
  # Prints any message passed to it.
  if not $words
    return 1                 # Error, if argument missing.
  print
  for word in $words         # Step through arguments passed to function.
    print "$word " end=""    # Print args one at a time, a space after each.
  print
  0
END

source data-file.tsh   # Load a data file.

print "variable1 (from data-file) = $variable1"
print "variable3 (from data-file) = $variable3"

sum = $variable2 + $variable4
print "Sum of variable2 + variable4 (from data-file) = $sum"
print "message1 (from data-file) is \"$message1\""
print "message2 (from data-file) is \"$message2\""

print_message This is the message-print function in the data-file.
```

```output
variable1 (from data-file) = 23
variable3 (from data-file) = 5
Sum of variable2 + variable4 (from data-file) = 571
message1 (from data-file) is "Greetings from *** line 13 *** of the data file!"
message2 (from data-file) is "Enough for now. Goodbye."

This is the message-print function in the data-file. 
```

**Gap:** no `$LINENO`: `(inspect.currentframe).f_lineno` gives it.

bash passes extra arguments of `source file arg…` to the file as its `$1`,
`$2`….

**Gap:** `source` takes only the file; it can't pass arguments to it.

### Example 15-23. A (useless) script that sources itself (`self-source.sh`)

```bash
#!/bin/bash
# self-source.sh: a script sourcing itself "recursively."
# From "Stupid Script Tricks," Volume II.

MAXPASSCNT=100    # Maximum number of execution passes.

echo -n  "$pass_count  "
#  At first execution pass, this just echoes two blank spaces,
#+ since $pass_count still uninitialized.

let "pass_count += 1"
#  Assumes the uninitialized variable $pass_count
#+ can be incremented the first time around.
#  This works with Bash and pdksh, but
#+ it relies on non-portable (and possibly dangerous) behavior.
#  Better would be to initialize $pass_count to 0 before incrementing.

while [ "$pass_count" -le $MAXPASSCNT ]
do
  . $0   # Script "sources" itself, rather than calling itself.
         # ./$0 (which would be true recursion) doesn't work here. Why?
done  

#  What occurs here is not actually recursion,
#+ since the script effectively "expands" itself, i.e.,
#+ generates a new section of code
#+ with each pass through the 'while' loop',
#  with each 'source' in line 20.
#
#  Of course, the script interprets each newly 'sourced' "#!" line
#+ as a comment, and not as the start of a new script.

echo

exit 0   # The net effect is counting from 1 to 100.
         # Very impressive.

# Exercise:
# --------
# Write a script that uses this trick to actually do something useful.
```

The same trick works in tessh: each `source` runs the whole file again, with
the same variables. `pass_count` isn't there the first time, so it's read
with `.get`.

```tessh
# self-source.tsh: a script sourcing itself "recursively."
# From "Stupid Script Tricks," Volume II.

MAXPASSCNT = 100    # Maximum number of execution passes.

pass_count = (globals).get "pass_count" ""
print "$pass_count  " end=""
#  At first execution pass, this just prints two blank spaces,
#+ since $pass_count isn't set yet.

pass_count = (int ($pass_count or 0)) + 1

while $pass_count <= $MAXPASSCNT
  source $argv[0]   # Script "sources" itself, rather than calling itself.

print

exit 0   # The net effect is counting from 1 to 100.
         # Very impressive.
```

```output
  1  2  3  4  5  6  7  8  9  10  11  12  13  14  15  16  17  18  19  20  21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40  41  42  43  44  45  46  47  48  49  50  51  52  53  54  55  56  57  58  59  60  61  62  63  64  65  66  67  68  69  70  71  72  73  74  75  76  77  78  79  80  81  82  83  84  85  86  87  88  89  90  91  92  93  94  95  96  97  98  99  100  
```

**exit** ends the script, with a status (0 for success). A bare `exit` in
bash passes on the last command's status; tessh's exits 0. `exit` raises
`SystemExit`, so a `finally` still runs.

**exec** replaces the shell with a program: the script ends when the
program does. (`exec <file` and the like, with only redirections, change the
shell's own file descriptors instead: see the I/O redirection chapter.)

### Example 15-24. Effects of exec (`ex54.sh`)

```bash no-run
#!/bin/bash

exec echo "Exiting \"$0\" at line $LINENO."   # Exit from script here.
# $LINENO is an internal Bash variable set to the line number it's on.

# ----------------------------------
# The following lines never execute.

echo "This echo fails to echo."

exit 99                       #  This script will not exit here.
                              #  Check exit value after script terminates
                              #+ with an 'echo $?'.
                              #  It will *not* be 99.
```

Python's `os.execvp` is `exec`. Python keeps what's printed in a buffer
when the output is a file or a pipe, and the new program won't write it out,
so flush it first.

```tessh
import os sys inspect

sys.stdout.flush
os.execvp echo [echo "Exiting \"$(Path $argv[0]).name\" at line ${(inspect.currentframe).f_lineno}."]   # Exit from script here.

# ----------------------------------
# The following lines never execute.

print "This print fails to print."

exit 99
```

```output
Exiting "ex.tsh" at line 4.
```

**Gap:** no `exec` builtin: `os.execvp` replaces the process (after a
`sys.stdout.flush`), and there's no `exec 3<file` to open a descriptor for
the rest of the script.

### Example 15-25. A script that exec's itself (`self-exec.sh`)

```bash no-run
#!/bin/bash
# self-exec.sh

# Note: Set permissions on this script to 555 or 755,
#       then call it with ./self-exec.sh or sh ./self-exec.sh.

echo

echo "This line appears ONCE in the script, yet it keeps echoing."
echo "The PID of this instance of the script is still $$."
#     Demonstrates that a subshell is not forked off.

echo "==================== Hit Ctl-C to exit ===================="

sleep 1

exec $0   #  Spawns another instance of this same script
          #+ that replaces the previous one.

echo "This line will never echo!"  # Why not?

exit 99                            # Will not exit here!
                                   # Exit code will not be 99!
```

The original goes on until Ctrl+C; this one stops after its third run. It
shows the process ID is the same each time by keeping the first one in the
environment, which `exec` hands on.

```tessh
import os sys
# self-exec.tsh

print

print "This line appears ONCE in the script, yet it keeps printing."
first = $env.get "FIRST_PID" (str (os.getpid))
print "The PID of this instance of the script is still the first one: $((str (os.getpid)) == $first)"
#     Demonstrates that a subprocess is not forked off.

print "==================== Hit Ctl-C to exit ===================="

sleep 0.1

runs = (int ($env.get "RUNS" "0")) + 1
if $runs == 3              # (Three runs here.)
  exit 0
export FIRST_PID=$first RUNS=$runs

sys.stdout.flush
os.execv (os.readlink /proc/self/exe) [tessh $argv[0]]   # Another instance of this same script
                                                         #+ that replaces the previous one.

print "This line will never print!"  # Why not?

exit 99
```

```output

This line appears ONCE in the script, yet it keeps printing.
The PID of this instance of the script is still the first one: True
==================== Hit Ctl-C to exit ====================

This line appears ONCE in the script, yet it keeps printing.
The PID of this instance of the script is still the first one: True
==================== Hit Ctl-C to exit ====================

This line appears ONCE in the script, yet it keeps printing.
The PID of this instance of the script is still the first one: True
==================== Hit Ctl-C to exit ====================
```

**shopt** sets bash's optional behaviors (`cdspell`, `extglob`,
`nullglob`…). tessh has no such switches.

**caller**, inside a function, tells where it was called from: the line,
the function and the file. Python's `inspect.stack` does that, and a
traceback shows the whole chain.

```bash no-run
#!/bin/bash

function1 ()
{
  # Inside function1 ().
  caller 0   # Tell me about it.
}

function1    # Line 9 of script.

# 9 main test.sh
# ^                 Line number that the function was called from.
#   ^^^^            Invoked from "main" part of script.
#        ^^^^^^^    Name of calling script.

caller 0     # Has no effect because it's not inside a function.
```

```tessh
import inspect

def caller
  # Where the function that called this one was called from.
  f = (inspect.stack)[2]
  print $f.lineno $f.function (Path $f.filename).name

def function1
  # Inside function1.
  caller     # Tell me about it.

function1    # Line 12 of script.
```

```output
12 <module> ex.tsh
```

## Commands
<!-- help: abs-builtins-commands true false type hash bind help -->

**true** and **false** are commands that only succeed or fail. tessh has
them as programs, and `True` and `False` as values: `while True` is the
endless loop, `if False` the branch never taken.

```bash
# Testing "false"
if false
then
  echo "false evaluates \"true\""
else
  echo "false evaluates \"false\""
fi
# false evaluates "false"
```

```tessh
if false             # (The program, as in bash.)
  print "false evaluates \"true\""
else
  print "false evaluates \"false\""
while False          # (the value)
  print "The following code will not execute."
```

```output
false evaluates "false"
```

A word alone after `if` or `while` is a command, as in bash: `if false`
runs `false`. `False` and `True` are Python's values.

**type** says what a name is: keyword, builtin, function, alias or program,
and with `-a` all of them. tessh's `which` finds programs, `help name`
describes builtins and functions, and Python's `type` gives a value's type.

**hash** remembers where programs are, so `$PATH` needn't be searched
again; **bind** shows and changes the keys of bash's line editor; **help**
describes a builtin. tessh has `help` (for its builtins, its topics, and any
Python value); the others have no counterpart.

**Gap:** no `type`: nothing says in one step whether a name is a tessh
builtin, a function or a program (and where).

## 15.1 Job Control Commands
<!-- help: abs-job-control-builtins jobs disown fg bg wait suspend logout times kill killall command builtin enable autoload -->

**jobs** lists the background jobs, **fg** brings one to the front and
**bg** lets a stopped one go on; tessh has all three. A job (`[1]`) isn't a
process (`1384`): `kill %1` and `kill 1384` both work in bash. In tessh a
trailing `&` gives a `Job` value, and `$job.wait` waits for it.

**disown** takes a job off the list; **suspend** stops the shell as
Ctrl+Z does; **logout** leaves a login shell; **times** prints the time the
shell has used (Python: `os.times`).

**wait** waits for the background jobs, or for one, and returns its status.
It keeps a script from going on before a job's work is done.

### Example 15-26. Waiting for a process to finish before proceeding (`ex39.sh`)

```bash no-run
#!/bin/bash

ROOT_UID=0   # Only users with $UID 0 have root privileges.
E_NOTROOT=65
E_NOPARAMS=66

if [ "$UID" -ne "$ROOT_UID" ]
then
  echo "Must be root to run this script."
  # "Run along kid, it's past your bedtime."
  exit $E_NOTROOT
fi  

if [ -z "$1" ]
then
  echo "Usage: `basename $0` find-string"
  exit $E_NOPARAMS
fi


echo "Updating 'locate' database..."
echo "This may take a while."
updatedb /usr &     # Must be run as root.

wait
# Don't run the rest of the script until 'updatedb' finished.
# You want the the database updated before looking up the file name.

locate $1

#  Without the 'wait' command, in the worse case scenario,
#+ the script would exit while 'updatedb' was still running,
#+ leaving it as an orphan process.

exit 0
```

`updatedb` needs root, so this can't run here. The job's `.wait` is bash's
`wait`.

```tessh no-run
import os

ROOT_UID = 0   # Only users with UID 0 have root privileges.
E_NOTROOT = 65
E_NOPARAMS = 66

if (os.getuid) != $ROOT_UID
  print "Must be root to run this script."
  exit $E_NOTROOT

if (len $argv) < 2
  print "Usage: $(Path $argv[0]).name find-string"
  exit $E_NOPARAMS

print "Updating 'locate' database..."
print "This may take a while."
job = updatedb /usr &      # Must be run as root.

$job.wait
# Don't run the rest of the script until 'updatedb' finished.

locate $argv[1]

exit 0
```

The same with a made-up job that takes a moment:

```tessh
print "Updating the database..."
job = sh -c "sleep 0.2; echo database updated >db" &
$job.wait
cat db
```

```output
Updating the database...
database updated
```

**Gap:** no `wait` for all the background jobs at once (wait on each `Job`),
no `$!`, and a `Job` doesn't give its process ID, so `kill` can't be sent to
it.

**kill** sends a process a signal: `kill -l` lists them, `-9` (KILL) can't
be caught, `-15` (TERM) is the default. **killall** kills processes by name.
In tessh `kill` is the program; from Python, `os.kill pid signal`.

### Example 15-27. A script that kills itself (`self-destruct.sh`)

```bash
#!/bin/bash
# self-destruct.sh

kill $$  # Script kills its own process here.
         # Recall that "$$" is the script's PID.

echo "This line will not echo."
# Instead, the shell sends a "Terminated" message to stdout.

exit 0   # Normal exit? No!

#  After this script terminates prematurely,
#+ what exit status does it return?
#
# sh self-destruct.sh
# echo $?
# 143
#
# 143 = 128 + 15
#             TERM signal
```

`$$`, the script's process ID, is `(os.getpid)`. A process killed by a
signal exits with 128 plus the signal's number: 143 for TERM.

```tessh
import os signal
# self-destruct.tsh

os.kill (os.getpid) $signal.SIGTERM   # Script kills its own process here.

print "This line will not print."

exit 0   # Normal exit? No!
```

```output
```

**command** runs a program even when a function or alias has its name:
tessh's `\name`. **builtin** runs the builtin of that name, and **enable**
turns builtins off and on (or loads new ones from a shared library);
**autoload** loads a function from a file when it's first called. In
tessh, `import` loads modules (tessh or Python) when the script asks.

A job can be named in several ways in bash:

| Notation | Meaning |
|---|---|
| `%N` | Job number [N] |
| `%S` | Invocation (command line) of job begins with string S |
| `%?S` | Invocation (command line) of job contains within it string S |
| `%%` | current job (last job stopped in foreground or started in background) |
| `%+` | current job (last job stopped in foreground or started in background) |
| `%-` | Last job |
| `$!` | Last background process |

tessh's `fg` and `bg` take the job's number, or none for the latest.
