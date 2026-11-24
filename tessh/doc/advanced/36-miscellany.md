# 36. Miscellany
<!-- help: abs-miscellany abs-36 miscellany wrappers recursion colors optimization tips security portability -->

Odds and ends: interactive and non-interactive scripts, wrappers around
other programs, recursion, color, speed, assorted tips, security and
portability.

## 36.1 Interactive and non-interactive shells and scripts
<!-- help: abs-intandnonint interactive non-interactive tty isatty ps1 -->

An interactive shell reads commands typed at a terminal: it reads startup
files, shows a prompt and does job control. A script runs in a
non-interactive shell, though it can still use the terminal. In tessh,
`tessh` alone is the interactive REPL (it reads `~/.config/tessh/rc.tsh`
first), and `tessh script` runs a script.

Stéphane Chazelas's script that plays at being an interactive shell:

```bash no-run
#!/bin/bash
MY_PROMPT='$ '
while :
do
  echo -n "$MY_PROMPT"
  read line
  eval "$line"
  done

exit 0

# This example script, and much of the above explanation supplied by
# Stéphane Chazelas (thanks again).
```

tessh's `eval "$line"` runs a line of tessh text in the shell, so the
loop could keep its variables between lines; this one hands each line to a
new tessh (`tessh -c $line`), which forgets them. (`eval $line`, without
the quotes, is Python's `eval`.) (Python's `eval` and `exec` take Python, and `py`
takes Python written in the script.) The REPL itself is the real thing.

```tessh no-run
#!/usr/bin/env tessh
import os
TESSH = os.readlink /proc/self/exe   # (the tessh running this script)
MY_PROMPT = '$ '
while True
  line = input $MY_PROMPT
  $TESSH -c $line
```

An interactive script is one that asks the user things, usually with
`read` (in tessh, `read` or `input`). Init scripts and most maintenance scripts are
non-interactive, and must be: in the background an interactive script
hangs waiting for input that never comes. A here document, a file
redirected in, or `expect` can stand in for the user, and in tessh a value
passed in (`<<<`, a heredoc, a list of canned answers) does the same.

To find out whether it's interactive, a bash script looks at `$PS1` or for
`i` in `$-`; tessh scripts have neither, and the dependable test in both
is whether standard input is a terminal (`[ -t 0 ]`), or, as John Lange
points out for `ssh`, a pipe:

```bash
# Test for a terminal!

fd=0   # stdin

#  As we recall, the -t test option checks whether the stdin, [ -t 0 ],
#+ or stdout, [ -t 1 ], in a given script is running in a terminal.
if [ -t "$fd" ]
then
  echo interactive
else
  echo non-interactive
fi


#  But, as John points out:
#    if [ -t 0 ] works ... when you're logged in locally
#    but fails when you invoke the command remotely via ssh.
#    So for a true test you also have to test for a socket.

if [[ -t "$fd" || -p /dev/stdin ]]
then
  echo interactive
else
  echo non-interactive
fi
```

```tessh
import os stat sys

fd = 0   # stdin

#  os.isatty checks whether a file descriptor is a terminal,
#  as [ -t 0 ] does.
if os.isatty $fd
  print interactive
else
  print non-interactive

#  For ssh: also count a pipe (bash's -p /dev/stdin).
if (os.isatty $fd) or (stat.S_ISFIFO (os.fstat $fd).st_mode)
  print interactive
else
  print non-interactive
```

```output
non-interactive
non-interactive
```

(Here standard input is `/dev/null`, so both say non-interactive.)
`sys.stdin.isatty` and `sys.stdout.isatty` ask the same of Python's
streams; the second is how a script decides whether to use color.
`bash -i` forces interactive mode; tessh has no such switch.

## 36.2 Shell Wrappers
<!-- help: abs-wrapper wrapper wrappers sed awk perl python embedded -->

A wrapper is a script around a program, passing it a set of arguments, so
a long `sed` or `awk` command line becomes a name to call. Many system
commands are wrappers (`pdf2ps`, `batch`). tessh runs `sed` and `awk` as
bash does, and it can often do their job itself, in Python.

### Example 36-1. shell wrapper (`ex3.sh`)

```bash no-run
#!/bin/bash

# This simple script removes blank lines from a file.
# No argument checking.
#
# You might wish to add something like:
#
# E_NOARGS=85
# if [ -z "$1" ]
# then
#  echo "Usage: `basename $0` target-file"
#  exit $E_NOARGS
# fi



sed -e /^$/d "$1"
# Same as
#    sed -e '/^$/d' filename
# invoked from the command-line.

#  The '-e' means an "editing" command follows (optional here).
#  '^' indicates the beginning of line, '$' the end.
#  This matches lines with nothing between the beginning and the end --
#+ blank lines.
#  The 'd' is the delete command.

#  Quoting the command-line arg permits
#+ whitespace and special characters in the filename.

#  Note that this script doesn't actually change the target file.
#  If you need to do that, redirect its output.

exit
```

The `sed` program goes in single quotes, so tessh doesn't read `$` in it.
There's no quoting to remember for the file name: a value is one argument
whatever is in it. The argument check the comments suggest is included.

```tessh
#!/usr/bin/env tessh
# This simple script removes blank lines from a file.
# (A made-up file, and a sample argument when none is given.)
printf "one\n\ntwo\n\n\nthree\n" >"my file.txt"
args = $argv[1:] or ["my file.txt"]

E_NOARGS = 85
if not $args
  print "Usage: $((Path $argv[0]).name) target-file"
  exit $E_NOARGS

sed -e '/^$/d' $args[0]
#  '^' is the beginning of line, '$' the end: lines with nothing
#  between are blank lines. The 'd' is the delete command.
#  ([$l for l in (Path $args[0]).read_text.splitlines if $l] in Python.)
```

```output
one
two
three
```

### Example 36-2. A slightly more complex shell wrapper (`ex4.sh`)

```bash no-run
#!/bin/bash

#  subst.sh: a script that substitutes one pattern for
#+ another in a file,
#+ i.e., "sh subst.sh Smith Jones letter.txt".
#                     Jones replaces Smith.

ARGS=3         # Script requires 3 arguments.
E_BADARGS=85   # Wrong number of arguments passed to script.

if [ $# -ne "$ARGS" ]
then
  echo "Usage: `basename $0` old-pattern new-pattern filename"
  exit $E_BADARGS
fi

old_pattern=$1
new_pattern=$2

if [ -f "$3" ]
then
    file_name=$3
else
    echo "File \"$3\" does not exist."
    exit $E_BADARGS
fi


# -----------------------------------------------
#  Here is where the heavy work gets done.
sed -e "s/$old_pattern/$new_pattern/g" $file_name
# -----------------------------------------------

#  's' is, of course, the substitute command in sed,
#+ and /pattern/ invokes address matching.
#  The 'g,' or global flag causes substitution for EVERY
#+ occurence of $old_pattern on each line, not just the first.
#  Read the 'sed' docs for an in-depth explanation.

exit $?  # Redirect the output of this script to write to a file.
```

Unpacking the arguments into three names checks their number at the same
time (a `match` would do it too).

```tessh
#!/usr/bin/env tessh
#  subst: substitutes one pattern for another in a file,
#  i.e., "subst Smith Jones letter.txt". Jones replaces Smith.
# (A made-up letter, and sample arguments when none are given.)
print "Dear Mr. Smith,\nSmith & Sons thanks you." >letter.txt
args = $argv[1:] or [Smith Jones letter.txt]

ARGS = 3         # Script requires 3 arguments.
E_BADARGS = 85   # Wrong number of arguments passed to script.

if (len $args) != $ARGS
  print "Usage: $((Path $argv[0]).name) old-pattern new-pattern filename"
  exit $E_BADARGS

[old_pattern new_pattern file_name] = $args

if not ((Path $file_name).is_file)
  print "File \"$file_name\" does not exist."
  exit $E_BADARGS

# -----------------------------------------------
#  Here is where the heavy work gets done.
sed -e "s/$old_pattern/$new_pattern/g" $file_name
# -----------------------------------------------
#  (Python's re.sub $old_pattern $new_pattern $text does the same.)
```

```output
Dear Mr. Jones,
Jones & Sons thanks you.
```

### Example 36-3. A generic shell wrapper that writes to a logfile (`logging-wrapper.sh`)

```bash no-run
#!/bin/bash
#  logging-wrapper.sh
#  Generic shell wrapper that performs an operation
#+ and logs it.

DEFAULT_LOGFILE=logfile.txt

# Set the following two variables.
OPERATION=
#         Can be a complex chain of commands,
#+        for example an awk script or a pipe . . .

LOGFILE=
if [ -z "$LOGFILE" ]
then     # If not set, default to ...
  LOGFILE="$DEFAULT_LOGFILE"
fi

#         Command-line arguments, if any, for the operation.
OPTIONS="$@"


# Log it.
echo "`date` + `whoami` + $OPERATION "$@"" >> $LOGFILE
# Now, do it.
exec $OPERATION "$@"

# It's necessary to do the logging before the operation.
# Why?
```

The operation is a list, a program and its fixed arguments, so `[program
*fixed] = $OPERATION` takes it apart and `$program *$fixed *$OPTIONS` runs
it. tessh's `exec` replaces the shell with a program given as words
(`exec ls -l`), but given a value (`exec $program`) it's Python's `exec`,
which runs Python text: don't. So the script runs the program and exits
with its status instead. (The log's date and user are cut off when
it's shown here, so the output stays the same from run to run.)

```tessh
#!/usr/bin/env tessh
#  logging-wrapper: performs an operation and logs it.
import datetime getpass

DEFAULT_LOGFILE = logfile.txt

# Set the following two variables.
OPERATION = [wc -l]       # (Empty in the original.)
LOGFILE=
if not $LOGFILE           # If not set, default to ...
  LOGFILE = $DEFAULT_LOGFILE

#  Command-line arguments, if any, for the operation.
# (A made-up file, and a sample argument when none is given.)
seq 5 >numbers.txt
OPTIONS = $argv[1:] or [numbers.txt]

# Log it.
print "$((datetime.datetime.now).ctime) + $(getpass.getuser) + $(' '.join ($OPERATION + $OPTIONS))" >>$LOGFILE
# Now, do it.
[program *fixed] = $OPERATION
$program *$fixed *$OPTIONS
status = $?

cut -d+ -f3- $LOGFILE       # (What was logged, without date and user.)
exit $status
```

```output
5 numbers.txt
 wc -l numbers.txt
```

### Example 36-4. A shell wrapper around an awk script (`pr-asc.sh`)

```bash
#!/bin/bash
# pr-ascii.sh: Prints a table of ASCII characters.

START=33   # Range of printable ASCII characters (decimal).
END=127    # Will not work for unprintable characters (> 127).

echo " Decimal   Hex     Character"   # Header.
echo " -------   ---     ---------"

for ((i=START; i<=END; i++))
do
  echo $i | awk '{printf("  %3d       %2x         %c\n", $1, $1, $1)}'
# The Bash printf builtin will not work in this context:
#     printf "%c" "$i"
done

exit 0


#  Decimal   Hex     Character
#  -------   ---     ---------
#    33       21         !
#    34       22         "
#    35       23         #
#    36       24         $
#
#    . . .
#
#   122       7a         z
#   123       7b         {
#   124       7c         |
#   125       7d         }


#  Redirect the output of this script to a file
#+ or pipe it to "more":  sh pr-asc.sh | more
```

Python's format specs do what `awk`'s `printf` did (`:3d`, `:2x`), and
`chr` makes the character, so there's no `awk` to start 95 times. (The
last line is character 127, DEL, which shows as nothing.)

```tessh
#!/usr/bin/env tessh
# pr-ascii: prints a table of ASCII characters.

START = 33   # Range of printable ASCII characters (decimal).
END = 127

print " Decimal   Hex     Character"   # Header.
print " -------   ---     ---------"

for i in (range $START ($END + 1))
  print "  ${i:3d}       ${i:2x}         $(chr $i)"
```

```output
 Decimal   Hex     Character
 -------   ---     ---------
   33       21         !
   34       22         "
   35       23         #
   36       24         $
   37       25         %
   38       26         &
   39       27         '
   40       28         (
   41       29         )
   42       2a         *
   43       2b         +
   44       2c         ,
   45       2d         -
   46       2e         .
   47       2f         /
   48       30         0
   49       31         1
   50       32         2
   51       33         3
   52       34         4
   53       35         5
   54       36         6
   55       37         7
   56       38         8
   57       39         9
   58       3a         :
   59       3b         ;
   60       3c         <
   61       3d         =
   62       3e         >
   63       3f         ?
   64       40         @
   65       41         A
   66       42         B
   67       43         C
   68       44         D
   69       45         E
   70       46         F
   71       47         G
   72       48         H
   73       49         I
   74       4a         J
   75       4b         K
   76       4c         L
   77       4d         M
   78       4e         N
   79       4f         O
   80       50         P
   81       51         Q
   82       52         R
   83       53         S
   84       54         T
   85       55         U
   86       56         V
   87       57         W
   88       58         X
   89       59         Y
   90       5a         Z
   91       5b         [
   92       5c         \
   93       5d         ]
   94       5e         ^
   95       5f         _
   96       60         `
   97       61         a
   98       62         b
   99       63         c
  100       64         d
  101       65         e
  102       66         f
  103       67         g
  104       68         h
  105       69         i
  106       6a         j
  107       6b         k
  108       6c         l
  109       6d         m
  110       6e         n
  111       6f         o
  112       70         p
  113       71         q
  114       72         r
  115       73         s
  116       74         t
  117       75         u
  118       76         v
  119       77         w
  120       78         x
  121       79         y
  122       7a         z
  123       7b         {
  124       7c         |
  125       7d         }
  126       7e         ~
  127       7f         
```

### Example 36-5. A shell wrapper around another awk script (`col-totaler.sh`)

```bash no-run
#!/bin/bash

# Adds up a specified column (of numbers) in the target file.
# Floating-point (decimal) numbers okay, because awk can handle them.

ARGS=2
E_WRONGARGS=85

if [ $# -ne "$ARGS" ] # Check for proper number of command-line args.
then
   echo "Usage: `basename $0` filename column-number"
   exit $E_WRONGARGS
fi

filename=$1
column_number=$2

#  Passing shell variables to the awk part of the script is a bit tricky.
#  One method is to strong-quote the Bash-script variable
#+ within the awk script.
#     $'$BASH_SCRIPT_VAR'
#      ^                ^
#  This is done in the embedded awk script below.
#  See the awk documentation for more details.

# A multi-line awk script is here invoked by
#   awk '
#   ...
#   ...
#   ...
#   '


# Begin awk script.
# -----------------------------
awk '

{ total += $'"${column_number}"'
}
END {
     print total
}     

' "$filename"
# -----------------------------
# End awk script.


#   It may not be safe to pass shell variables to an embedded awk script,
#+  so Stephane Chazelas proposes the following alternative:
#   ---------------------------------------
#   awk -v column_number="$column_number" '
#   { total += $column_number
#   }
#   END {
#       print total
#   }' "$filename"
#   ---------------------------------------


exit 0
```

This uses Stéphane Chazelas's safer way: `awk -v` passes the column number
as an awk variable, and the awk program stays in single quotes, untouched.
(Chapter 28 has the version that pastes it into the program.)

```tessh
#!/usr/bin/env tessh
# Adds up a specified column (of numbers) in the target file.
# Floating-point (decimal) numbers okay, because awk can handle them.
# (A made-up data file, and sample arguments when none are given.)
printf "pens 3 1.25\nink 2 4.50\npaper 1 6.75\n" >sales.txt
args = $argv[1:] or [sales.txt 3]

ARGS = 2
E_WRONGARGS = 85

if (len $args) != $ARGS   # Check for proper number of command-line args.
  print "Usage: $((Path $argv[0]).name) filename column-number"
  exit $E_WRONGARGS

[filename column_number] = $args

# Begin awk script.
awk -v column_number=$column_number '
{ total += $column_number
}
END {
     print total
}' $filename
# End awk script.
```

```output
12.5
```

Perl does what `sed` and `awk` do and much more, and short Perl scripts
embed well in shell scripts.

### Example 36-6. Perl embedded in a Bash script (`ex56.sh`)

```bash no-run
#!/bin/bash

# Shell commands may precede the Perl script.
echo "This precedes the embedded Perl script within \"$0\"."
echo "==============================================================="

perl -e 'print "This line prints from an embedded Perl script.\n";'
# Like sed, Perl also uses the "-e" option.

echo "==============================================================="
echo "However, the script may also contain shell and system commands."

exit 0
```

(The bash version prints the path it was run by, so it isn't run here.)
`perl -e` works from tessh the same way.

```tessh
#!/usr/bin/env tessh
# Shell commands may precede the Perl script.
print "This precedes the embedded Perl script within \"$((Path $argv[0]).name)\"."
print "==============================================================="

perl -e 'print "This line prints from an embedded Perl script.\n";'
# Like sed, Perl also uses the "-e" option.

print "==============================================================="
print "However, the script may also contain shell and system commands."
```

```output
This precedes the embedded Perl script within "ex.tsh".
===============================================================
This line prints from an embedded Perl script.
===============================================================
However, the script may also contain shell and system commands.
```

### Example 36-7. Bash and Perl scripts combined (`bashandperl.sh`)

```bash no-run
#!/bin/bash
# bashandperl.sh

echo "Greetings from the Bash part of the script, $0."
# More Bash commands may follow here.

exit
# End of Bash part of the script.

# =======================================================

#!/usr/bin/perl
# This part of the script must be invoked with
#    perl -x bashandperl.sh

print "Greetings from the Perl part of the script, $0.\n";
#      Perl doesn't seem to like "echo" ...
# More Perl commands may follow here.

# End of Perl part of the script.
```

```text
bash$ bash bashandperl.sh
Greetings from the Bash part of the script.

bash$ perl -x bashandperl.sh
Greetings from the Perl part of the script.
```

The trick works with tessh too, with one change: tessh compiles the whole
file before running any of it, so the Perl part after `exit` has to be
valid tessh as well. Here it's in a string, which tessh ignores and `perl
-x` (which skips to the `#!…perl` line) reads as Perl. To show both, the
tessh part runs `perl -x` on its own file before it exits.

```tessh
# tessh-and-perl
import os

print "Greetings from the tessh part of the script, $((Path $argv[0]).name)."
# More tessh commands may follow here.

cd (Path $argv[0]).parent             # (To show the Perl part too.)
perl -x (Path $argv[0]).name
exit
# End of tessh part of the script.

"""
#!/usr/bin/perl
# This part of the script must be invoked with
#    perl -x thisscript

print "Greetings from the Perl part of the script, $0.\n";
__END__
"""
```

```output
Greetings from the tessh part of the script, ex.tsh.
Greetings from the Perl part of the script, ex.tsh.
```

Python is the language tessh is made of, so where bash embeds it with
`python -c`, tessh has it on every line, and `py "…"` takes Python
written as Python.

### Example 36-8. Python embedded in a Bash script (`ex56py.sh`)

```bash no-run
#!/bin/bash
# ex56py.sh

# Shell commands may precede the Python script.
echo "This precedes the embedded Python script within \"$0.\""
echo "==============================================================="

python -c 'print "This line prints from an embedded Python script.\n";'
# Unlike sed and perl, Python uses the "-c" option.
python -c 'k = raw_input( "Hit a key to exit to outer script. " )'

echo "==============================================================="
echo "However, the script may also contain shell and system commands."

exit 0
```

(This is Python 2, which is gone: `print` is a function and `raw_input`
is `input` now.) The key press is canned here.

```tessh
#!/usr/bin/env tessh
# Shell commands may precede the Python.
print "This precedes the embedded Python script within \"$((Path $argv[0]).name).\""
print "==============================================================="

py 'print("This line prints from embedded Python.")'
# No -c, no second process: it runs right here.
k = "x"   # (k = input "Hit a key to exit to outer script. ", interactively)

print "==============================================================="
print "However, the script may also contain shell and system commands."
```

```output
This precedes the embedded Python script within "ex.tsh."
===============================================================
This line prints from embedded Python.
===============================================================
However, the script may also contain shell and system commands.
```

### Example 36-9. A script that speaks (`speech.sh`)

```bash no-run
#!/bin/bash
#   Courtesy of:
#   http://elinux.org/RPi_Text_to_Speech_(Speech_Synthesis)

#  You must be on-line for this script to work,
#+ so you can access the Google translation server.
#  Of course, mplayer must be present on your computer.

speak()
  {
  local IFS=+
  # Invoke mplayer, then connect to Google translation server.
  /usr/bin/mplayer -ao alsa -really-quiet -noconsolecontrols \
 "http://translate.google.com/translate_tts?tl=en&q="$*""
  # Google translates, but can also speak.
  }

LINES=4

spk=$(tail -$LINES $0) # Tail end of same script!
speak "$spk"
exit
# Browns. Nice talking to you.
```

It needs the network and `mplayer`, so it isn't run here. `local IFS=+`
was a way to join the words with `+`; in tessh that's `urllib.parse.quote_plus`,
which also escapes what a URL can't hold. As in Example 36-7, the text
after `exit` must still be tessh, so the spoken lines are a comment.

```tessh no-run
#!/usr/bin/env tessh
import urllib.parse

def speak text
  # Invoke mplayer, then connect to Google translation server.
  url = "http://translate.google.com/translate_tts?tl=en&q=" + (urllib.parse.quote_plus $text)
  /usr/bin/mplayer -ao alsa -really-quiet -noconsolecontrols $url

LINES = 4
spk = tail -$LINES $argv[0]   # Tail end of same script!
speak $spk
exit
# Browns. Nice talking to you.
```

Bigger wrappers include Martin Matusiak's `undvd`, around `mencoder`, and
Itzchak Rehberg's Ext3Undel.

## 36.3 Tests and Comparisons: Alternatives
<!-- help: abs-testsandcomparisons test [[ (( comparison alternatives -->

Bash has several ways to write one test: `test`, `/bin/test`, `[ ]`, `[[
]]` and `(( ))`. tessh has one, a Python comparison, though `test` and `[`
still run as programs.

```bash
a=8

# All of the comparisons below are equivalent.
test "$a" -lt 16 && echo "yes, $a < 16"         # "and list"
/bin/test "$a" -lt 16 && echo "yes, $a < 16" 
[ "$a" -lt 16 ] && echo "yes, $a < 16" 
[[ $a -lt 16 ]] && echo "yes, $a < 16"          # Quoting variables within
(( a < 16 )) && echo "yes, $a < 16"             # [[ ]] and (( )) not necessary.

city="New York"
# Again, all of the comparisons below are equivalent.
test "$city" \< Paris && echo "Yes, Paris is greater than $city"
                                  # Greater ASCII order.
/bin/test "$city" \< Paris && echo "Yes, Paris is greater than $city" 
[ "$city" \< Paris ] && echo "Yes, Paris is greater than $city" 
[[ $city < Paris ]] && echo "Yes, Paris is greater than $city"
                                  # Need not quote $city.

# Thank you, S.C.
```

```output bash
yes, 8 < 16
yes, 8 < 16
yes, 8 < 16
yes, 8 < 16
Yes, Paris is greater than New York
Yes, Paris is greater than New York
Yes, Paris is greater than New York
```

(This system has no `/bin/test`, so those two lines fail in bash.) In
tessh, `&&` after a Python value goes by its truth, as after a program it
goes by the status.

```tessh
a = 8

test $a -lt 16 && echo "yes, $a < 16"     # The test program.
[ $a -lt 16 ] && echo "yes, $a < 16"      # [ is a program too.
($a < 16) && print "yes, $a < 16"         # A Python comparison.
if $a < 16                                # The usual way.
  print "yes, $a < 16"

city = "New York"
($city < Paris) && print "Yes, Paris is greater than $city"
                                          # Text compares in code-point order.
```

```output
yes, 8 < 16
yes, 8 < 16
yes, 8 < 16
yes, 8 < 16
Yes, Paris is greater than New York
```

## 36.4 Recursion: a script calling itself
<!-- help: abs-recursionsct recursion recursive self-calling script -->

A script can run itself. In bash that's `./$0` (or `"$0"`); in tessh it's
the tessh that's running, which `/proc/self/exe` names, and the script's
own path, `$argv[0]`. (**Gap:** tessh has no variable for its own
program; `sys.executable` is a Python.)

### Example 36-10. A (useless) script that recursively calls itself (`recurse.sh`)

```bash no-run
#!/bin/bash
# recurse.sh

#  Can a script recursively call itself?
#  Yes, but is this of any practical use?
#  (See the following.)

RANGE=10
MAXVAL=9

i=$RANDOM
let "i %= $RANGE"  # Generate a random number between 0 and $RANGE - 1.

if [ "$i" -lt "$MAXVAL" ]
then
  echo "i = $i"
  ./$0             #  Script recursively spawns a new instance of itself.
fi                 #  Each child script does the same, until
                   #+ a generated $i equals $MAXVAL.

#  Using a "while" loop instead of an "if/then" test causes problems.
#  Explain why.

exit 0

# Note:
# ----
# This script must have execute permission for it to work properly.
# This is the case even if it is invoked by an "sh" command.
# Explain why.
```

To make a run repeatable, each instance passes its depth to the next and
seeds `random` with it; leave the seed out for a different run each time.
Run by tessh, the script needs no execute permission.

```tessh script
#!/usr/bin/env tessh
# recurse: can a script recursively call itself?
import os random

TESSH = os.readlink /proc/self/exe   # The tessh running this script.
RANGE = 10
MAXVAL = 9

depth = int (if (len $argv) > 1 then $argv[1] else 0)
random.seed $depth                   # (Repeatable, for the example.)
i = random.randrange $RANGE          # A random number from 0 to RANGE - 1.

if $i < $MAXVAL
  print "i = $i"
  $TESSH $argv[0] ($depth + 1)       # Script spawns a new instance of itself,
                                     # until an i equals MAXVAL.
```

```output
i = 6
i = 2
i = 0
i = 3
i = 3
```

### Example 36-11. A (useful) script that recursively calls itself (`pb.sh`)

```bash no-run
#!/bin/bash
# pb.sh: phone book

# Written by Rick Boivie, and used with permission.
# Modifications by ABS Guide author.

MINARGS=1     #  Script needs at least one argument.
DATAFILE=./phonebook
              #  A data file in current working directory
              #+ named "phonebook" must exist.
PROGNAME=$0
E_NOARGS=70   #  No arguments error.

if [ $# -lt $MINARGS ]; then
      echo "Usage: "$PROGNAME" data-to-look-up"
      exit $E_NOARGS
fi      


if [ $# -eq $MINARGS ]; then
      grep $1 "$DATAFILE"
      # 'grep' prints an error message if $DATAFILE not present.
else
      ( shift; "$PROGNAME" $* ) | grep $1
      # Script recursively calls itself.
fi

exit 0        #  Script exits here.
              #  Therefore, it's o.k. to put
              #+ non-hashmarked comments and data after this point.

# ------------------------------------------------------------------------
Sample "phonebook" datafile:

John Doe        1555 Main St., Baltimore, MD 21228          (410) 222-3333
Mary Moe        9899 Jones Blvd., Warren, NH 03787          (603) 898-3232
Richard Roe     856 E. 7th St., New York, NY 10009          (212) 333-4567
Sam Roe         956 E. 8th St., New York, NY 10009          (212) 444-5678
Zoe Zenobia     4481 N. Baker St., San Francisco, SF 94338  (415) 501-1631
# ------------------------------------------------------------------------

$bash pb.sh Roe
Richard Roe     856 E. 7th St., New York, NY 10009          (212) 333-4567
Sam Roe         956 E. 8th St., New York, NY 10009          (212) 444-5678

$bash pb.sh Roe Sam
Sam Roe         956 E. 8th St., New York, NY 10009          (212) 444-5678

#  When more than one argument is passed to this script,
#+ it prints *only* the line(s) containing all the arguments.
```

The data can't follow `exit` as plain text in tessh (the whole file must
be tessh), so the script writes its sample phone book first. With more
than one word, it runs itself on all but the first and greps that output
for the first, as the original does.

```tessh script
#!/usr/bin/env tessh
# pb: phone book, by Rick Boivie.
import os

# (The sample "phonebook" data file.)
cat >phonebook <<'END'
John Doe        1555 Main St., Baltimore, MD 21228          (410) 222-3333
Mary Moe        9899 Jones Blvd., Warren, NH 03787          (603) 898-3232
Richard Roe     856 E. 7th St., New York, NY 10009          (212) 333-4567
Sam Roe         956 E. 8th St., New York, NY 10009          (212) 444-5678
Zoe Zenobia     4481 N. Baker St., San Francisco, SF 94338  (415) 501-1631
END

MINARGS = 1       #  Script needs at least one argument.
DATAFILE = ./phonebook
TESSH = os.readlink /proc/self/exe
PROGNAME = $argv[0]
E_NOARGS = 70     #  No arguments error.

args = $argv[1:] or [Roe Sam]   # (sample arguments, when none are given)

if (len $args) < $MINARGS
  print "Usage: $PROGNAME data-to-look-up"
  exit $E_NOARGS

if (len $args) == $MINARGS
  grep $args[0] $DATAFILE
else
  rest = $TESSH $PROGNAME *$args[1:]    # Script recursively calls itself,
  $rest | grep $args[0]                  # and greps what it found.
```

```output
Sam Roe         956 E. 8th St., New York, NY 10009          (212) 444-5678
```

### Example 36-12. Another (useful) script that recursively calls itself (`usrmnt.sh`)

```bash no-run
#!/bin/bash
# usrmnt.sh, written by Anthony Richardson
# Used in ABS Guide with permission.

# usage:       usrmnt.sh
# description: mount device, invoking user must be listed in the
#              MNTUSERS group in the /etc/sudoers file.

# ----------------------------------------------------------
#  This is a usermount script that reruns itself using sudo.
#  A user with the proper permissions only has to type

#   usermount /dev/fd0 /mnt/floppy

# instead of

#   sudo usermount /dev/fd0 /mnt/floppy

#  I use this same technique for all of my
#+ sudo scripts, because I find it convenient.
# ----------------------------------------------------------

#  If SUDO_COMMAND variable is not set we are not being run through
#+ sudo, so rerun ourselves. Pass the user's real and group id . . .

if [ -z "$SUDO_COMMAND" ]
then
   mntusr=$(id -u) grpusr=$(id -g) sudo $0 $*
   exit 0
fi

# We will only get here if we are being run by sudo.
/bin/mount $* -o uid=$mntusr,gid=$grpusr

exit 0

# Additional notes (from the author of this script): 
# -------------------------------------------------

# 1) Linux allows the "users" option in the /etc/fstab
#    file so that any user can mount removable media.
#    But, on a server, I like to allow only a few
#    individuals access to removable media.
#    I find using sudo gives me more control.

# 2) I also find sudo to be more convenient than
#    accomplishing this task through groups.

# 3) This method gives anyone with proper permissions
#    root access to the mount command, so be careful
#    about who you allow access.
#    You can get finer control over which access can be mounted
#    by using this same technique in separate mntfloppy, mntcdrom,
#    and mntsamba scripts.
```

It needs `sudo` and a device, so it isn't run here. The `mntusr=…
grpusr=… sudo $0 $*` prefix carries over (sudo passes them on only if its
policy allows). The user's ids come from `os.getuid` and `os.getgid`.

```tessh no-run
#!/usr/bin/env tessh
# usrmnt, by Anthony Richardson: mount a device, rerunning itself with sudo.
import os

#  If SUDO_COMMAND is not set we are not being run through sudo,
#+ so rerun ourselves, passing the user's real and group id.
if "SUDO_COMMAND" not in $env
  TESSH = os.readlink /proc/self/exe
  mntusr=(os.getuid) grpusr=(os.getgid) sudo $TESSH $argv[0] *$argv[1:]
  exit $?

# We will only get here if we are being run by sudo.
/bin/mount *$argv[1:] -o "uid=$env.mntusr,gid=$env.grpusr"
```

Too deep a recursion runs out of stack. For a tessh function that calls
itself, Python stops it with a `RecursionError` at about a thousand
levels; a script that runs itself uses a process per level.

## 36.5 "Colorizing" Scripts
<!-- help: abs-colorizing color colour ansi escape-sequences bold tput -->

ANSI escape sequences set the terminal's colors and attributes. `\e[1m`
turns on bold and `\e[0m` turns everything off: `\e` (written `\033` in
bash) is the escape character, `[` starts the sequence, the numbers are
attributes separated by `;`, and `m` ends it. tessh strings take `\e`, as
`echo -e` does; Python's own spelling is `\x1b`. (`"\033"` in a tessh
string is not the escape character. **Gap:** tessh doesn't read octal
escapes as Python does.)

```bash no-run
bash$ echo -e "\033[1mThis is bold text.\033[0m"
bash$ echo -e "\033[4mThis is underlined text.\033[0m"
bash$ echo -e '\E[34;47mThis prints in blue.'; tput sgr0
bash$ echo -e '\E[33;44m'"yellow text on blue background"; tput sgr0
bash$ echo -e '\E[1;33;44m'"BOLD yellow text on blue background"; tput sgr0
```

```tessh no-run
print "\e[1mThis is bold text.\e[0m"
print "\e[4mThis is underlined text.\e[0m"
print "\e[34;47mThis prints in blue.\e[0m"
print "\e[33;44myellow text on blue background\e[0m"
print "\e[1;33;44mBOLD yellow text on blue background\e[0m"
```

`tput sgr0` (or `\e[0m`, which works where `tput` doesn't) puts the
terminal back to normal; leave it out and everything after stays blue.
Bold helps light colors. The template is `\e[FOREGROUND;BACKGROUNDm`:

| color | foreground | background |
| --- | --- | --- |
| black | 30 | 40 |
| red | 31 | 41 |
| green | 32 | 42 |
| yellow | 33 | 43 |
| blue | 34 | 44 |
| magenta | 35 | 45 |
| cyan | 36 | 46 |
| white | 37 | 47 |

A script should only send these to a terminal: in a file or a pipe they're
garbage. The examples below check `sys.stdout.isatty` and honor `NO_COLOR`,
the convention for turning color off, so here, in a pipe, they print
plain text.

### Example 36-13. A colorized address database (`ex30a.sh`)

```bash no-run
#!/bin/bash
# ex30a.sh: "Colorized" version of ex30.sh.
#            Crude address database


clear                                   # Clear the screen.

echo -n "          "
echo -e '\E[37;44m'"\033[1mContact List\033[0m"
                                        # White on blue background
echo; echo
echo -e "\033[1mChoose one of the following persons:\033[0m"
                                        # Bold
tput sgr0                               # Reset attributes.
echo "(Enter only the first letter of name.)"
echo
echo -en '\E[47;34m'"\033[1mE\033[0m"   # Blue
tput sgr0                               # Reset colors to "normal."
echo "vans, Roland"                     # "[E]vans, Roland"
echo -en '\E[47;35m'"\033[1mJ\033[0m"   # Magenta
tput sgr0
echo "ambalaya, Mildred"
echo -en '\E[47;32m'"\033[1mS\033[0m"   # Green
tput sgr0
echo "mith, Julie"
echo -en '\E[47;31m'"\033[1mZ\033[0m"   # Red
tput sgr0
echo "ane, Morris"
echo

read person

case "$person" in
# Note variable is quoted.

  "E" | "e" )
  # Accept upper or lowercase input.
  echo
  echo "Roland Evans"
  echo "4321 Flash Dr."
  echo "Hardscrabble, CO 80753"
  echo "(303) 734-9874"
  echo "(303) 734-9892 fax"
  echo "revans@zzy.net"
  echo "Business partner & old friend"
  ;;

  "J" | "j" )
  echo
  echo "Mildred Jambalaya"
  echo "249 E. 7th St., Apt. 19"
  echo "New York, NY 10009"
  echo "(212) 533-2814"
  echo "(212) 533-9972 fax"
  echo "milliej@loisaida.com"
  echo "Girlfriend"
  echo "Birthday: Feb. 11"
  ;;

# Add info for Smith & Zane later.

          * )
   # Default option.	  
   # Empty input (hitting RETURN) fits here, too.
   echo
   echo "Not yet in database."
  ;;

esac

tput sgr0                               # Reset colors to "normal."

echo

exit 0
```

A function `color` wraps text in a sequence, or leaves it alone when color
is off. The `case` is a `match`, with `.lower` so one pattern takes both
cases. The answer is canned, as the script can't read the terminal here.

```tessh
#!/usr/bin/env tessh
# ex30a: "colorized" crude address database.
import os sys

USE_COLOR = (sys.stdout.isatty) and "NO_COLOR" not in $env
def color codes text
  if $USE_COLOR then "\e[${codes}m$text\e[0m" else $text

if $USE_COLOR
  clear                                 # Clear the screen.

print "          " (color "37;44;1" "Contact List") sep=""
                                        # White on blue background
print ; print
print (color 1 "Choose one of the following persons:")   # Bold
print "(Enter only the first letter of name.)"
print
print (color "47;34;1" E) "vans, Roland" sep=""         # Blue
print (color "47;35;1" J) "ambalaya, Mildred" sep=""    # Magenta
print (color "47;32;1" S) "mith, Julie" sep=""          # Green
print (color "47;31;1" Z) "ane, Morris" sep=""          # Red
print

person = "j"     # (person = input "", interactively)

match ($person.lower)
  case "e"
    print
    print "Roland Evans"
    print "4321 Flash Dr."
    print "Hardscrabble, CO 80753"
    print "(303) 734-9874"
    print "(303) 734-9892 fax"
    print "revans@zzy.net"
    print "Business partner & old friend"
  case "j"
    print
    print "Mildred Jambalaya"
    print "249 E. 7th St., Apt. 19"
    print "New York, NY 10009"
    print "(212) 533-2814"
    print "(212) 533-9972 fax"
    print "milliej@loisaida.com"
    print "Girlfriend"
    print "Birthday: Feb. 11"
  # Add info for Smith & Zane later.
  case _
    # Default option. Empty input (hitting RETURN) fits here, too.
    print
    print "Not yet in database."

print
```

```output
          Contact List


Choose one of the following persons:
(Enter only the first letter of name.)

Evans, Roland
Jambalaya, Mildred
Smith, Julie
Zane, Morris


Mildred Jambalaya
249 E. 7th St., Apt. 19
New York, NY 10009
(212) 533-2814
(212) 533-9972 fax
milliej@loisaida.com
Girlfriend
Birthday: Feb. 11

```

### Example 36-14. Drawing a box (`Draw-box.sh`)

```bash no-run
#!/bin/bash
# Draw-box.sh: Drawing a box using ASCII characters.

# Script by Stefano Palmeri, with minor editing by document author.
# Minor edits suggested by Jim Angstadt.
# Used in the ABS Guide with permission.


######################################################################
###  draw_box function doc  ###

#  The "draw_box" function lets the user
#+ draw a box in a terminal.       
#
#  Usage: draw_box ROW COLUMN HEIGHT WIDTH [COLOR] 
#  ROW and COLUMN represent the position        
#+ of the upper left angle of the box you're going to draw.
#  ROW and COLUMN must be greater than 0
#+ and less than current terminal dimension.
#  HEIGHT is the number of rows of the box, and must be > 0. 
#  HEIGHT + ROW must be <= than current terminal height. 
#  WIDTH is the number of columns of the box and must be > 0.
#  WIDTH + COLUMN must be <= than current terminal width.
#
# E.g.: If your terminal dimension is 20x80,
#  draw_box 2 3 10 45 is good
#  draw_box 2 3 19 45 has bad HEIGHT value (19+2 > 20)
#  draw_box 2 3 18 78 has bad WIDTH value (78+3 > 80)
#
#  COLOR is the color of the box frame.
#  This is the 5th argument and is optional.
#  0=black 1=red 2=green 3=tan 4=blue 5=purple 6=cyan 7=white.
#  If you pass the function bad arguments,
#+ it will just exit with code 65,
#+ and no messages will be printed on stderr.
#
#  Clear the terminal before you start to draw a box.
#  The clear command is not contained within the function.
#  This allows the user to draw multiple boxes, even overlapping ones.

###  end of draw_box function doc  ### 
######################################################################

draw_box(){

#=============#
HORZ="-"
VERT="|"
CORNER_CHAR="+"

MINARGS=4
E_BADARGS=65
#=============#


if [ $# -lt "$MINARGS" ]; then          # If args are less than 4, exit.
    exit $E_BADARGS
fi

# Looking for non digit chars in arguments.
# Probably it could be done better (exercise for the reader?).
if echo $@ | tr -d [:blank:] | tr -d [:digit:] | grep . &> /dev/null; then
   exit $E_BADARGS
fi

BOX_HEIGHT=`expr $3 - 1`   #  -1 correction needed because angle char "+"
BOX_WIDTH=`expr $4 - 1`    #+ is a part of both box height and width.
T_ROWS=`tput lines`        #  Define current terminal dimension 
T_COLS=`tput cols`         #+ in rows and columns.
         
if [ $1 -lt 1 ] || [ $1 -gt $T_ROWS ]; then    #  Start checking if arguments
   exit $E_BADARGS                             #+ are correct.
fi
if [ $2 -lt 1 ] || [ $2 -gt $T_COLS ]; then
   exit $E_BADARGS
fi
if [ `expr $1 + $BOX_HEIGHT + 1` -gt $T_ROWS ]; then
   exit $E_BADARGS
fi
if [ `expr $2 + $BOX_WIDTH + 1` -gt $T_COLS ]; then
   exit $E_BADARGS
fi
if [ $3 -lt 1 ] || [ $4 -lt 1 ]; then
   exit $E_BADARGS
fi                                 # End checking arguments.

plot_char(){                       # Function within a function.
   echo -e "\E[${1};${2}H"$3
}

echo -ne "\E[3${5}m"               # Set box frame color, if defined.

# start drawing the box

count=1                                         #  Draw vertical lines using
for (( r=$1; count<=$BOX_HEIGHT; r++)); do      #+ plot_char function.
  plot_char $r $2 $VERT
  let count=count+1
done 

count=1
c=`expr $2 + $BOX_WIDTH`
for (( r=$1; count<=$BOX_HEIGHT; r++)); do
  plot_char $r $c $VERT
  let count=count+1
done 

count=1                                        #  Draw horizontal lines using
for (( c=$2; count<=$BOX_WIDTH; c++)); do      #+ plot_char function.
  plot_char $1 $c $HORZ
  let count=count+1
done 

count=1
r=`expr $1 + $BOX_HEIGHT`
for (( c=$2; count<=$BOX_WIDTH; c++)); do
  plot_char $r $c $HORZ
  let count=count+1
done 

plot_char $1 $2 $CORNER_CHAR                   # Draw box angles.
plot_char $1 `expr $2 + $BOX_WIDTH` $CORNER_CHAR
plot_char `expr $1 + $BOX_HEIGHT` $2 $CORNER_CHAR
plot_char `expr $1 + $BOX_HEIGHT` `expr $2 + $BOX_WIDTH` $CORNER_CHAR

echo -ne "\E[0m"             #  Restore old colors.

P_ROWS=`expr $T_ROWS - 1`    #  Put the prompt at bottom of the terminal.

echo -e "\E[${P_ROWS};1H"
}      


# Now, let's try drawing a box.
clear                       # Clear the terminal.
R=2      # Row
C=3      # Column
H=10     # Height
W=45     # Width 
col=1    # Color (red)
draw_box $R $C $H $W $col   # Draw the box.

exit 0

# Exercise:
# --------
# Add the option of printing text within the drawn box.
```

The same checks and the same four lines, but `plot_char` puts each
character on a screen of its own, a list of rows, instead of moving the
terminal's cursor; the screen is printed at the end. That works in a pipe
and a file too, and colors only on a terminal. The checks raise
`ValueError` with a reason rather than exiting with a bare 65. The
exercise, text in the box, is the optional last argument.

```tessh
#!/usr/bin/env tessh
# Draw-box: drawing a box using ASCII characters, after Stefano Palmeri's.
import shutil sys

HORZ = "-"
VERT = "|"
CORNER_CHAR = "+"

def draw_box screen row col height width color=None text=""
  # Usage: draw_box SCREEN ROW COLUMN HEIGHT WIDTH [COLOR] [TEXT]
  # ROW and COLUMN (from 1) are the box's upper left corner. COLOR is
  # 0=black 1=red 2=green 3=tan 4=blue 5=purple 6=cyan 7=white.
  t_rows = len $screen
  t_cols = len $screen[0]
  if $height < 1 or $width < 1
    raise (ValueError "height and width must be > 0")
  if not (1 <= $row and $row + $height - 1 <= $t_rows)
    raise (ValueError "the box doesn't fit in $t_rows rows")
  if not (1 <= $col and $col + $width - 1 <= $t_cols)
    raise (ValueError "the box doesn't fit in $t_cols columns")

  def plot_char r c ch               # Function within a function.
    $screen[$r - 1][$c - 1] = $ch

  bottom = $row + $height - 1
  right = $col + $width - 1
  for r in (range $row $bottom)      # Vertical lines.
    plot_char $r $col $VERT
    plot_char $r $right $VERT
  for c in (range $col $right)       # Horizontal lines.
    plot_char $row $c $HORZ
    plot_char $bottom $c $HORZ
  for [r c] in [[$row $col] [$row $right] [$bottom $col] [$bottom $right]]
    plot_char $r $c $CORNER_CHAR     # Draw box angles.
  for [i ch] in (enumerate $text[:($width - 4)])
    plot_char ($row + 1) ($col + 2 + $i) $ch
  if $color is not None and (sys.stdout.isatty)
    for r in (range $row ($bottom + 1))   # Color the frame's rows.
      line = $screen[$r - 1]
      $line[$col - 1] = "\e[3${color}m" + $line[$col - 1]
      $line[$right - 1] = $line[$right - 1] + "\e[0m"

# Now, let's try drawing a box, on a screen the size of the terminal
# (80 by 24 when there's none).
[T_COLS T_ROWS] = shutil.get_terminal_size (80, 24)
T_ROWS = min $T_ROWS 12                # (Only the top of it, to show.)
screen = [[" " for c in (range $T_COLS)] for r in (range $T_ROWS)]

R = 2      # Row
C = 3      # Column
H = 10     # Height
W = 45     # Width
col = 1    # Color (red)
draw_box $screen $R $C $H $W $col "Hello from inside the box."

for line in $screen
  print (.rstrip ("".join $line))
```

```output

  +-------------------------------------------+
  | Hello from inside the box.                |
  |                                           |
  |                                           |
  |                                           |
  |                                           |
  |                                           |
  |                                           |
  |                                           |
  +-------------------------------------------+

```

### Example 36-15. Echoing colored text (`color-echo.sh`)

```bash no-run
#!/bin/bash
# color-echo.sh: Echoing text messages in color.

# Modify this script for your own purposes.
# It's easier than hand-coding color.

black='\E[30;47m'
red='\E[31;47m'
green='\E[32;47m'
yellow='\E[33;47m'
blue='\E[34;47m'
magenta='\E[35;47m'
cyan='\E[36;47m'
white='\E[37;47m'


alias Reset="tput sgr0"      #  Reset text attributes to normal
                             #+ without clearing screen.


cecho ()                     # Color-echo.
                             # Argument $1 = message
                             # Argument $2 = color
{
local default_msg="No message passed."
                             # Doesn't really need to be a local variable.

message=${1:-$default_msg}   # Defaults to default message.
color=${2:-$black}           # Defaults to black, if not specified.

  echo -e "$color"
  echo "$message"
  Reset                      # Reset to normal.

  return
}  


# Now, let's try it out.
# ----------------------------------------------------
cecho "Feeling blue..." $blue
cecho "Magenta looks more like purple." $magenta
cecho "Green with envy." $green
cecho "Seeing red?" $red
cecho "Cyan, more familiarly known as aqua." $cyan
cecho "No color passed (defaults to black)."
       # Missing $color argument.
cecho "\"Empty\" color passed (defaults to black)." ""
       # Empty $color argument.
cecho
       # Missing $message and $color arguments.
cecho "" ""
       # Empty $message and $color arguments.
# ----------------------------------------------------

echo

exit 0

# Exercises:
# ---------
# 1) Add the "bold" attribute to the 'cecho ()' function.
# 2) Add options for colored backgrounds.
```

The colors go in a dict, and `cecho`'s defaults are its parameters'
defaults; `or` gives the default for an empty argument too, as `${1:-…}`
does. Both exercises are done: `bold=True` and `background=`.

```tessh
#!/usr/bin/env tessh
# color-echo: echoing text messages in color.
import sys

COLORS = {black: 30 red: 31 green: 32 yellow: 33 blue: 34 magenta: 35 cyan: 36 white: 37}
USE_COLOR = (sys.stdout.isatty) and "NO_COLOR" not in $env

def cecho message=None color=None bold=False background="white"
  message = $message or "No message passed."    # Defaults to default message.
  color = $color or "black"                     # Defaults to black.
  if not $USE_COLOR
    print $message
    return
  codes = [$COLORS[$color] ($COLORS[$background] + 10)] + (if $bold then [1] else [])
  print "\e[$(';'.join (map $str $codes))m$message\e[0m"   # Reset to normal.

# Now, let's try it out.
# ----------------------------------------------------
cecho "Feeling blue..." "blue"
cecho "Magenta looks more like purple." "magenta"
cecho "Green with envy." "green" bold=True
cecho "Seeing red?" "red"
cecho "Cyan, more familiarly known as aqua." "cyan"
cecho "No color passed (defaults to black)."
       # Missing color argument.
cecho "\"Empty\" color passed (defaults to black)." ""
       # Empty color argument.
cecho
       # Missing message and color arguments.
cecho "" ""
       # Empty message and color arguments.
# ----------------------------------------------------

print
```

```output
Feeling blue...
Magenta looks more like purple.
Green with envy.
Seeing red?
Cyan, more familiarly known as aqua.
No color passed (defaults to black).
"Empty" color passed (defaults to black).
No message passed.
No message passed.

```

### Example 36-16. A horserace game (`horserace.sh`)

```bash no-run
#!/bin/bash
# horserace.sh: Very simple horserace simulation.
# Author: Stefano Palmeri
# Used with permission.

################################################################
#  Goals of the script:
#  playing with escape sequences and terminal colors.
#
#  Exercise:
#  Edit the script to make it run less randomly,
#+ set up a fake betting shop . . .     
#  Um . . . um . . . it's starting to remind me of a movie . . .
#
#  The script gives each horse a random handicap.
#  The odds are calculated upon horse handicap
#+ and are expressed in European(?) style.
#  E.g., odds=3.75 means that if you bet $1 and win,
#+ you receive $3.75.
# 
#  The script has been tested with a GNU/Linux OS,
#+ using xterm and rxvt, and konsole.
#  On a machine with an AMD 900 MHz processor,
#+ the average race time is 75 seconds.    
#  On faster computers the race time would be lower.
#  So, if you want more suspense, reset the USLEEP_ARG variable.
#
#  Script by Stefano Palmeri.
################################################################

E_RUNERR=65

# Check if md5sum and bc are installed. 
if ! which bc &> /dev/null; then
   echo bc is not installed.  
   echo "Can\'t run . . . "
   exit $E_RUNERR
fi
if ! which md5sum &> /dev/null; then
   echo md5sum is not installed.  
   echo "Can\'t run . . . "
   exit $E_RUNERR
fi

#  Set the following variable to slow down script execution.
#  It will be passed as the argument for usleep (man usleep)  
#+ and is expressed in microseconds (500000 = half a second).
USLEEP_ARG=0  

#  Clean up the temp directory, restore terminal cursor and 
#+ terminal colors -- if script interrupted by Ctl-C.
trap 'echo -en "\E[?25h"; echo -en "\E[0m"; stty echo;\
tput cup 20 0; rm -fr  $HORSE_RACE_TMP_DIR'  TERM EXIT
#  See the chapter on debugging for an explanation of 'trap.'

# Set a unique (paranoid) name for the temp directory the script needs.
HORSE_RACE_TMP_DIR=$HOME/.horserace-`date +%s`-`head -c10 /dev/urandom \
| md5sum | head -c30`

# Create the temp directory and move right in.
mkdir $HORSE_RACE_TMP_DIR
cd $HORSE_RACE_TMP_DIR


#  This function moves the cursor to line $1 column $2 and then prints $3.
#  E.g.: "move_and_echo 5 10 linux" is equivalent to
#+ "tput cup 4 9; echo linux", but with one command instead of two.
#  Note: "tput cup" defines 0 0 the upper left angle of the terminal,
#+ echo defines 1 1 the upper left angle of the terminal.
move_and_echo() {
          echo -ne "\E[${1};${2}H""$3" 
}

# Function to generate a pseudo-random number between 1 and 9. 
random_1_9 ()
{
    head -c10 /dev/urandom | md5sum | tr -d [a-z] | tr -d 0 | cut -c1 
}

#  Two functions that simulate "movement," when drawing the horses. 
draw_horse_one() {
               echo -n " "//$MOVE_HORSE//
}
draw_horse_two(){
              echo -n " "\\\\$MOVE_HORSE\\\\ 
}   


# Define current terminal dimension.
N_COLS=`tput cols`
N_LINES=`tput lines`

# Need at least a 20-LINES X 80-COLUMNS terminal. Check it.
if [ $N_COLS -lt 80 ] || [ $N_LINES -lt 20 ]; then
   echo "`basename $0` needs a 80-cols X 20-lines terminal."
   echo "Your terminal is ${N_COLS}-cols X ${N_LINES}-lines."
   exit $E_RUNERR
fi


# Start drawing the race field.

# Need a string of 80 chars. See below.
BLANK80=`seq -s "" 100 | head -c80`

clear

# Set foreground and background colors to white.
echo -ne '\E[37;47m'

# Move the cursor on the upper left angle of the terminal.
tput cup 0 0 

# Draw six white lines.
for n in `seq 5`; do
      echo $BLANK80   # Use the 80 chars string to colorize the terminal.
done

# Sets foreground color to black. 
echo -ne '\E[30m'

move_and_echo 3 1 "START  1"            
move_and_echo 3 75 FINISH
move_and_echo 1 5 "|"
move_and_echo 1 80 "|"
move_and_echo 2 5 "|"
move_and_echo 2 80 "|"
move_and_echo 4 5 "|  2"
move_and_echo 4 80 "|"
move_and_echo 5 5 "V  3"
move_and_echo 5 80 "V"

# Set foreground color to red. 
echo -ne '\E[31m'

# Some ASCII art.
move_and_echo 1 8 "..@@@..@@@@@...@@@@@.@...@..@@@@..."
move_and_echo 2 8 ".@...@...@.......@...@...@.@......."
move_and_echo 3 8 ".@@@@@...@.......@...@@@@@.@@@@...."
move_and_echo 4 8 ".@...@...@.......@...@...@.@......."
move_and_echo 5 8 ".@...@...@.......@...@...@..@@@@..."
move_and_echo 1 43 "@@@@...@@@...@@@@..@@@@..@@@@."
move_and_echo 2 43 "@...@.@...@.@.....@.....@....."
move_and_echo 3 43 "@@@@..@@@@@.@.....@@@@...@@@.."
move_and_echo 4 43 "@..@..@...@.@.....@.........@."
move_and_echo 5 43 "@...@.@...@..@@@@..@@@@.@@@@.."


# Set foreground and background colors to green.
echo -ne '\E[32;42m'

# Draw  eleven green lines.
tput cup 5 0
for n in `seq 11`; do
      echo $BLANK80
done

# Set foreground color to black. 
echo -ne '\E[30m'
tput cup 5 0

# Draw the fences. 
echo "++++++++++++++++++++++++++++++++++++++\
++++++++++++++++++++++++++++++++++++++++++"

tput cup 15 0
echo "++++++++++++++++++++++++++++++++++++++\
++++++++++++++++++++++++++++++++++++++++++"

# Set foreground and background colors to white.
echo -ne '\E[37;47m'

# Draw three white lines.
for n in `seq 3`; do
      echo $BLANK80
done

# Set foreground color to black.
echo -ne '\E[30m'

# Create 9 files to stores handicaps.
for n in `seq 10 7 68`; do
      touch $n
done  

# Set the first type of "horse" the script will draw.
HORSE_TYPE=2

#  Create position-file and odds-file for every "horse".
#+ In these files, store the current position of the horse,
#+ the type and the odds.
for HN in `seq 9`; do
      touch horse_${HN}_position
      touch odds_${HN}
      echo \-1 > horse_${HN}_position
      echo $HORSE_TYPE >>  horse_${HN}_position
      # Define a random handicap for horse.
       HANDICAP=`random_1_9`
      # Check if the random_1_9 function returned a good value.
      while ! echo $HANDICAP | grep [1-9] &> /dev/null; do
                HANDICAP=`random_1_9`
      done
      # Define last handicap position for horse. 
      LHP=`expr $HANDICAP \* 7 + 3`
      for FILE in `seq 10 7 $LHP`; do
            echo $HN >> $FILE
      done   
     
      # Calculate odds.
      case $HANDICAP in 
              1) ODDS=`echo $HANDICAP \* 0.25 + 1.25 | bc`
                                 echo $ODDS > odds_${HN}
              ;;
              2 | 3) ODDS=`echo $HANDICAP \* 0.40 + 1.25 | bc`
                                       echo $ODDS > odds_${HN}
              ;;
              4 | 5 | 6) ODDS=`echo $HANDICAP \* 0.55 + 1.25 | bc`
                                             echo $ODDS > odds_${HN}
              ;; 
              7 | 8) ODDS=`echo $HANDICAP \* 0.75 + 1.25 | bc`
                                       echo $ODDS > odds_${HN}
              ;; 
              9) ODDS=`echo $HANDICAP \* 0.90 + 1.25 | bc`
                                  echo $ODDS > odds_${HN}
      esac


done


# Print odds.
print_odds() {
tput cup 6 0
echo -ne '\E[30;42m'
for HN in `seq 9`; do
      echo "#$HN odds->" `cat odds_${HN}`
done
}

# Draw the horses at starting line.
draw_horses() {
tput cup 6 0
echo -ne '\E[30;42m'
for HN in `seq 9`; do
      echo /\\$HN/\\"                               "
done
}

print_odds

echo -ne '\E[47m'
# Wait for a enter key press to start the race.
# The escape sequence '\E[?25l' disables the cursor.
tput cup 17 0
echo -e '\E[?25l'Press [enter] key to start the race...
read -s

#  Disable normal echoing in the terminal.
#  This avoids key presses that might "contaminate" the screen
#+ during the race.  
stty -echo

# --------------------------------------------------------
# Start the race.

draw_horses
echo -ne '\E[37;47m'
move_and_echo 18 1 $BLANK80
echo -ne '\E[30m'
move_and_echo 18 1 Starting...
sleep 1

# Set the column of the finish line.
WINNING_POS=74

# Define the time the race started.
START_TIME=`date +%s`

# COL variable needed by following "while" construct.
COL=0    

while [ $COL -lt $WINNING_POS ]; do
                   
          MOVE_HORSE=0     
          
          # Check if the random_1_9 function has returned a good value.
          while ! echo $MOVE_HORSE | grep [1-9] &> /dev/null; do
                MOVE_HORSE=`random_1_9`
          done
          
          # Define old type and position of the "randomized horse".
          HORSE_TYPE=`cat  horse_${MOVE_HORSE}_position | tail -n 1`
          COL=$(expr `cat  horse_${MOVE_HORSE}_position | head -n 1`)
          
          ADD_POS=1
          # Check if the current position is an handicap position. 
          if seq 10 7 68 | grep -w $COL &> /dev/null; then
                if grep -w $MOVE_HORSE $COL &> /dev/null; then
                      ADD_POS=0
                      grep -v -w  $MOVE_HORSE $COL > ${COL}_new
                      rm -f $COL
                      mv -f ${COL}_new $COL
                      else ADD_POS=1
                fi 
          else ADD_POS=1
          fi
          COL=`expr $COL + $ADD_POS`
          echo $COL >  horse_${MOVE_HORSE}_position  # Store new position.
                            
         # Choose the type of horse to draw.         
          case $HORSE_TYPE in 
                1) HORSE_TYPE=2; DRAW_HORSE=draw_horse_two
                ;;
                2) HORSE_TYPE=1; DRAW_HORSE=draw_horse_one 
          esac       
          echo $HORSE_TYPE >>  horse_${MOVE_HORSE}_position
          # Store current type.
         
          # Set foreground color to black and background to green.
          echo -ne '\E[30;42m'
          
          # Move the cursor to new horse position.
          tput cup `expr $MOVE_HORSE + 5` \
	  `cat  horse_${MOVE_HORSE}_position | head -n 1` 
          
          # Draw the horse.
          $DRAW_HORSE
           usleep $USLEEP_ARG
          
           # When all horses have gone beyond field line 15, reprint odds.
           touch fieldline15
           if [ $COL = 15 ]; then
             echo $MOVE_HORSE >> fieldline15  
           fi
           if [ `wc -l fieldline15 | cut -f1 -d " "` = 9 ]; then
               print_odds
               : > fieldline15
           fi           
          
          # Define the leading horse.
          HIGHEST_POS=`cat *position | sort -n | tail -1`          
          
          # Set background color to white.
          echo -ne '\E[47m'
          tput cup 17 0
          echo -n Current leader: `grep -w $HIGHEST_POS *position | cut -c7`\
	  "                              "

done  

# Define the time the race finished.
FINISH_TIME=`date +%s`

# Set background color to green and enable blinking text.
echo -ne '\E[30;42m'
echo -en '\E[5m'

# Make the winning horse blink.
tput cup `expr $MOVE_HORSE + 5` \
`cat  horse_${MOVE_HORSE}_position | head -n 1`
$DRAW_HORSE

# Disable blinking text.
echo -en '\E[25m'

# Set foreground and background color to white.
echo -ne '\E[37;47m'
move_and_echo 18 1 $BLANK80

# Set foreground color to black.
echo -ne '\E[30m'

# Make winner blink.
tput cup 17 0
echo -e "\E[5mWINNER: $MOVE_HORSE\E[25m""  Odds: `cat odds_${MOVE_HORSE}`"\
"  Race time: `expr $FINISH_TIME - $START_TIME` secs"

# Restore cursor and old colors.
echo -en "\E[?25h"
echo -en "\E[0m"

# Restore echoing.
stty echo

# Remove race temp directory.
rm -rf $HORSE_RACE_TMP_DIR

tput cup 19 0

exit 0
```

The bash script keeps each horse's position, type and odds in files in a
temp folder, and makes random digits with `md5sum`; tessh keeps them in a
list of dicts and asks `random` (seeded here, so the race is the same each
time it's shown). `bc` isn't needed for the odds, and a `try`/`finally`
puts the terminal back however the race ends, as the `trap` did. The
drawing happens only on a terminal, 80 by 20 at least; anywhere else the
script prints the odds and the result.

```tessh
#!/usr/bin/env tessh
# horserace: very simple horserace simulation, after Stefano Palmeri's.
import random shutil sys time

random.seed 2     # (The same race each time, for the example.)
E_RUNERR = 65
USLEEP = 0.0      # Seconds between moves; raise it for more suspense.
DRAW = (sys.stdout.isatty)    # Draw the race only on a terminal.

def move_and_echo row col text   # Cursor to row, col (from 1), then text.
  if $DRAW
    print "\e[${row};${col}H$text" end="" flush=True

def paint codes                  # Set colors, on a terminal.
  if $DRAW
    print "\e[${codes}m" end=""

def random_1_9 = random.randint 1 9

if $DRAW
  [N_COLS N_LINES] = (shutil.get_terminal_size)
  if $N_COLS < 80 or $N_LINES < 20
    print "$((Path $argv[0]).name) needs a 80-cols X 20-lines terminal."
    print "Your terminal is ${N_COLS}-cols X ${N_LINES}-lines."
    exit $E_RUNERR

BLANK80 = " " * 80
HANDICAP_COLS = range 10 69 7     # Columns 10, 17, ... 66.

def draw_field
  clear
  paint "37;47"
  for n in (range 5)
    move_and_echo ($n + 1) 1 $BLANK80
  paint 30
  for [r c t] in [[3 1 "START  1"] [3 75 FINISH] [1 5 "|"] [1 80 "|"] [2 5 "|"]
                  [2 80 "|"] [4 5 "|  2"] [4 80 "|"] [5 5 "V  3"] [5 80 "V"]]
    move_and_echo $r $c $t
  paint 31                       # Some ASCII art.
  art = ["..@@@..@@@@@...@@@@@.@...@..@@@@...   @@@@...@@@...@@@@..@@@@..@@@@."
         ".@...@...@.......@...@...@.@.......   @...@.@...@.@.....@.....@....."
         ".@@@@@...@.......@...@@@@@.@@@@....   @@@@..@@@@@.@.....@@@@...@@@.."
         ".@...@...@.......@...@...@.@.......   @..@..@...@.@.....@.........@."
         ".@...@...@.......@...@...@..@@@@...   @...@.@...@..@@@@..@@@@.@@@@.."]
  for [i line] in (enumerate $art 1)
    move_and_echo $i 8 $line
  paint "32;42"
  for n in (range 11)
    move_and_echo ($n + 6) 1 $BLANK80
  paint 30
  move_and_echo 6 1 ("+" * 80)   # Draw the fences.
  move_and_echo 16 1 ("+" * 80)
  paint "37;47"
  for n in (range 3)
    move_and_echo ($n + 17) 1 $BLANK80
  paint 30

def odds_for handicap            # European style: 3.75 pays $3.75 for $1.
  factor = match $handicap
    case 1
      0.25
    case 2 | 3
      0.40
    case 4 | 5 | 6
      0.55
    case 7 | 8
      0.75
    case 9
      0.90
  $handicap * $factor + 1.25

# Each horse: position, type of drawing, the handicap columns it must stop
# at once each, and its odds.
horses = {}
for hn in (range 1 10)
  handicap = (random_1_9)
  $horses[$hn] = {pos: -1 type: 2 odds: (odds_for $handicap)
                 stops: (set (range 10 ($handicap * 7 + 4) 7))}

def print_odds
  if $DRAW
    paint "30;42"
  for hn in (range 1 10)
    move_and_echo ($hn + 6) 1 ""
    odds = $horses[$hn][odds]
    print "#$hn odds-> ${odds:.2f}"

def draw_horse hn
  h = $horses[$hn]
  look = if $h[type] == 1 then " //$hn//" else " \\\\$hn\\\\"
  move_and_echo ($hn + 6) ($h[pos] + 1) $look

try
  if $DRAW
    draw_field
    print "\e[?25l" end=""       # Hide the cursor.
    stty -echo                   # Keys pressed don't spoil the screen.
  print_odds
  START_TIME = (time.time)
  WINNING_POS = 74
  col = 0
  while $col < $WINNING_POS
    move_horse = (random_1_9)
    h = $horses[$move_horse]
    col = $h[pos]
    if $col in $h[stops]         # At a handicap column: wait a turn.
      $h[stops].remove $col
    else
      col += 1
    $h[pos] = $col
    $h[type] = 3 - $h[type]      # Switch between the two drawings.
    paint "30;42"
    draw_horse $move_horse
    time.sleep $USLEEP
    leader = max $horses key=(fn [n] $horses[$n][pos])
    paint 47
    move_and_echo 18 1 "Current leader: $leader                    "

  FINISH_TIME = (time.time)
  paint "30;42;5"                # The winning horse blinks.
  draw_horse $move_horse
  paint "25;37;47"
  move_and_echo 18 1 $BLANK80
  paint 30
  move_and_echo 18 1 ""
  odds = $horses[$move_horse][odds]
  result = "WINNER: $move_horse  Odds: ${odds:.2f}"
  if $DRAW
    result += "  Race time: $(round ($FINISH_TIME - $START_TIME)) secs"
  print $result
finally
  if $DRAW                       # Restore cursor, colors and echoing.
    print "\e[?25h\e[0m" end=""
    stty echo
    tput cup 20 0
```

```output
#1 odds-> 1.50
#2 odds-> 2.05
#3 odds-> 2.05
#4 odds-> 4.55
#5 odds-> 2.45
#6 odds-> 4.00
#7 odds-> 4.00
#8 odds-> 3.45
#9 odds-> 1.50
WINNER: 8  Odds: 3.45
```

## 36.6 Optimizations
<!-- help: abs-optimizations optimization speed performance profile timeit -->

Most scripts are quick fixes where speed doesn't matter. When one does
matter, the ABS Guide's advice holds, adjusted for tessh:

- Look at the loops first: move what doesn't change out of them.
- Prefer what runs in the script to what starts a process. In bash that
  means builtins over programs; in tessh it means Python over programs for
  small jobs. Each program run is a new process, so `expr $n + 1` in a loop
  is slow in bash, and `$n + 1` in tessh costs next to nothing. For
  big data, the opposite can hold: `sort` or `grep` on a large file beat
  Python line loops.
- Avoid unnecessary commands, especially in a pipe: `grep $word $file`, not
  `cat $file | grep $word`. In tessh, a value can start a pipe itself:
  `$text | grep $word`.
- `export LC_ALL=C` speeds up `grep`, `sort` and other programs on large
  text by turning off Unicode handling, in tessh as in bash (for the
  programs; Python's own strings are Unicode either way).
- Bash's `expr` is far slower than `$(( ))`. tessh has no such trap:
  arithmetic is Python's.
- `case` beat `if [ ]` chains in bash. In tessh a dict lookup beats both
  `if`/`elif` and `match` for a long table.
- Erik Brandsberg found bash's associative arrays faster than numeric ones
  for rewriting entries. Python's lists and dicts are both fast; use the
  one that fits.
- Profile: `time` works on programs, and Python's `cProfile` and
  `timeit` time Python: `timeit.timeit (fn ($n + 1)) number=100000`.
- Keep file I/O down; read a file once into a value rather than with a
  program per line.
- Write modular code, in functions, so it can be reorganized and tightened
  later.

```tessh
import timeit
n = 41
t = timeit.timeit (fn ($n + 1)) number=100000
print (if $t < 1 then "fast" else "slow")
```

```output
fast
```

## 36.7 Assorted Tips
<!-- help: abs-assortedtips tips tricks pseudo-code library return-values progress-bar -->

### Ideas for more powerful scripts

**Pseudo-code.** When you don't know how to write a part, write it in
words and come back to it. A tessh script with pseudo-code in it doesn't
compile, which keeps you honest: `tessh --check` lists what's left. In
bash:

```bash no-run
#!/bin/bash

ARGCOUNT=1                     # Need name as argument.
E_WRONGARGS=65

if [ number-of-arguments is-not-equal-to "$ARGCOUNT" ]
#    ^^^^^^^^^^^^^^^^^^^ ^^^^^^^^^^^^^^^
#  Can't figure out how to code this . . .
#+ . . . so write it in pseudo-code.

then
  echo "Usage: name-of-script name"
  #            ^^^^^^^^^^^^^^     More pseudo-code.
  exit $E_WRONGARGS
fi 

. . .

exit 0


# Later on, substitute working code for the pseudo-code.

# Line 6 becomes:
if [ $# -ne "$ARGCOUNT" ]

# Line 12 becomes:
  echo "Usage: `basename $0` name"
```

In tessh line 6 becomes `if (len $argv) - 1 != $ARGCOUNT`, and line 12
`print "Usage: $((Path $argv[0]).name) name"`.

**Keeping a record of the scripts run**, appended to a file named in the
environment:

```bash no-run
# Append (>>) following to end of each script tracked.

whoami>> $SAVE_FILE    # User invoking the script.
echo $0>> $SAVE_FILE   # Script name.
date>> $SAVE_FILE      # Date and time.
echo>> $SAVE_FILE      # Blank line as separator.

#  Of course, SAVE_FILE defined and exported as environmental variable in ~/.bashrc
#+ (something like ~/.scripts-run)
```

In tessh it's one line in the same spirit: `print (getpass.getuser)
$argv[0] (time.ctime) "" sep="\n" >>$env.SAVE_FILE`.

**Prepending a line** to a file: `>>` appends, but to put a line first:

```bash no-run
file=data.txt
title="***This is the title line of data text file***"

echo $title | cat - $file >$file.new
# "cat -" concatenates stdout to $file.
#  End result is
#+ to write a new file with $title appended at *beginning*.
```

```tessh
# (A made-up data file.)
printf "line 1\nline 2\n" >data.txt

file = ./data.txt
title = "***This is the title line of data text file***"

echo $title | cat - $file >"${file}.new"  # The same, in tessh.
cat "${file}.new"

# Or in Python, rewriting the file in place:
$file.write_text ($title + "\n" + ($file.read_text))
head -1 $file
```

```output
***This is the title line of data text file***
line 1
line 2
***This is the title line of data text file***
```

**Scripts inside other things.** A script can be called from another
script, a Makefile, or a C program's `system()`; in Python,
`subprocess.run [tessh script.tsh]`.

**Embedded sed and awk scripts in variables** make the wrapper around them
easier to read: `awk_prog = '{ total += $1 } END { print total }'` then
`awk $awk_prog file`.

**Libraries.** Keep your favorite definitions and functions in a file and
pull it in where needed. Bash `source`s a file of functions; a tessh file
is a module, imported by name, its functions used with the module's name
in front (or `from lib import …`). The ABS Guide's library:

```bash no-run
# SCRIPT LIBRARY
# ------ -------

# Note:
# No "#!" here.
# No "live code" either.


# Useful variable definitions

ROOT_UID=0             # Root has $UID 0.
E_NOTROOT=101          # Not root user error. 
MAXRETVAL=255          # Maximum (positive) return value of a function.
SUCCESS=0
FAILURE=-1



# Functions

Usage ()               # "Usage:" message.
{
  if [ -z "$1" ]       # No arg passed.
  then
    msg=filename
  else
    msg=$@
  fi

  echo "Usage: `basename $0` "$msg""
}  


Check_if_root ()       # Check if root running script.
{                      # From "ex39.sh" example.
  if [ "$UID" -ne "$ROOT_UID" ]
  then
    echo "Must be root to run this script."
    exit $E_NOTROOT
  fi
}  


CreateTempfileName ()  # Creates a "unique" temp filename.
{                      # From "ex51.sh" example.
  prefix=temp
  suffix=`eval date +%s`
  Tempfilename=$prefix.$suffix
}


isalpha2 ()            # Tests whether *entire string* is alphabetic.
{                      # From "isalpha.sh" example.
  [ $# -eq 1 ] || return $FAILURE

  case $1 in
  *[!a-zA-Z]*|"") return $FAILURE;;
  *) return $SUCCESS;;
  esac                 # Thanks, S.C.
}


abs ()                           # Absolute value.
{                                # Caution: Max return value = 255.
  E_ARGERR=-999999

  if [ -z "$1" ]                 # Need arg passed.
  then
    return $E_ARGERR             # Obvious error value returned.
  fi

  if [ "$1" -ge 0 ]              # If non-negative,
  then                           #
    absval=$1                    # stays as-is.
  else                           # Otherwise,
    let "absval = (( 0 - $1 ))"  # change sign.
  fi  

  return $absval
}


tolower ()             #  Converts string(s) passed as argument(s)
{                      #+ to lowercase.

  if [ -z "$1" ]       #  If no argument(s) passed,
  then                 #+ send error message
    echo "(null)"      #+ (C-style void-pointer error message)
    return             #+ and return from function.
  fi  

  echo "$@" | tr A-Z a-z
  # Translate all passed arguments ($@).

  return

# Use command substitution to set a variable to function output.
# For example:
#    oldvar="A seT of miXed-caSe LEtTerS"
#    newvar=`tolower "$oldvar"`
#    echo "$newvar"    # a set of mixed-case letters
#
# Exercise: Rewrite this function to change lowercase passed argument(s)
#           to uppercase ... toupper()  [easy].
}
```

The tessh library is shorter, since Python has several of these already
(`abs`, `str.isalpha`, `str.lower`, `tempfile`), and its functions return
real values with no 255 limit. Here the script writes the library, then
imports it.

```tessh
cat >scriptlib.tsh <<'END'
# SCRIPT LIBRARY: no "#!" here, and no live code either.
import os tempfile

ROOT_UID = 0           # Root has uid 0.
E_NOTROOT = 101        # Not root user error.

def usage script msg="filename"      # "Usage:" message.
  print "Usage: $((Path $script).name) $msg"

def check_if_root                    # Check if root running script.
  if (os.getuid) != $ROOT_UID
    print "Must be root to run this script."
    exit $E_NOTROOT

def create_tempfile_name             # Creates a unique temp file name.
  (tempfile.mkstemp prefix="temp.")[1]

def isalpha2 s                       # Whether the entire string is letters.
  ($s.isascii) and ($s.isalpha)

def tolower *words                   # The words passed, in lowercase.
  if not $words then "(null)" else (.lower (" ".join $words))
END

import scriptlib
scriptlib.usage $argv[0]
print (scriptlib.isalpha2 abc) (scriptlib.isalpha2 a1c) (abs -7)
oldvar = "A seT of miXed-caSe LEtTerS"
print (scriptlib.tolower $oldvar)     # a set of mixed-case letters
```

```output
Usage: ex.tsh filename
True False 7
a set of mixed-case letters
```

**Special comment headers** make a script easier to read, in tessh as in
bash:

```bash no-run
## Caution.
rm -rf *.zzy   ##  The "-rf" options to "rm" are very dangerous,
               ##+ especially with wild cards.

#+ Line continuation.
#  This is line 1
#+ of a multi-line comment,
#+ and this is the final line.

#* Note.

#o List item.

#> Another point of view.
while [ "$var1" != "end" ]    #> while test "$var1" != "end"
```

**A progress bar**, from Dotan Barak:

### Example 36-17. A Progress Bar (`progress-bar.sh`)

```bash no-run
#!/bin/bash
# progress-bar.sh

# Author: Dotan Barak (very minor revisions by ABS Guide author).
# Used in ABS Guide with permission (thanks!).


BAR_WIDTH=50
BAR_CHAR_START="["
BAR_CHAR_END="]"
BAR_CHAR_EMPTY="."
BAR_CHAR_FULL="="
BRACKET_CHARS=2
LIMIT=100

print_progress_bar()
{
        # Calculate how many characters will be full.
        let "full_limit = ((($1 - $BRACKET_CHARS) * $2) / $LIMIT)"

        # Calculate how many characters will be empty.
        let "empty_limit = ($1 - $BRACKET_CHARS) - ${full_limit}"

        # Prepare the bar.
        bar_line="${BAR_CHAR_START}"
        for ((j=0; j<full_limit; j++)); do
                bar_line="${bar_line}${BAR_CHAR_FULL}"
        done

        for ((j=0; j<empty_limit; j++)); do
                bar_line="${bar_line}${BAR_CHAR_EMPTY}"
        done

        bar_line="${bar_line}${BAR_CHAR_END}"

        printf "%3d%% %s" $2 ${bar_line}
}

# Here is a sample of code that uses it.
MAX_PERCENT=100
for ((i=0; i<=MAX_PERCENT; i++)); do
        #
        usleep 10000
        # ... Or run some other commands ...
        #
        print_progress_bar ${BAR_WIDTH} ${i}
        echo -en "\r"
done

echo ""

exit
```

(`usleep` is gone from most systems, so the bash version isn't run here.)
Strings multiply: `"=" * n` is n of them, so the bar is one expression.
`\r` takes the cursor back to the start of the line to draw over it; that
only makes sense on a terminal, so elsewhere the script prints every
twenty-fifth bar on a line of its own.

```tessh
#!/usr/bin/env tessh
# progress-bar, by Dotan Barak.
import sys time

BAR_WIDTH = 50
BAR_CHAR_START = "["
BAR_CHAR_END = "]"
BAR_CHAR_EMPTY = "."
BAR_CHAR_FULL = "="
BRACKET_CHARS = 2
LIMIT = 100

def progress_bar width percent
  # How many characters will be full, and how many empty.
  full_limit = ($width - $BRACKET_CHARS) * $percent // $LIMIT
  empty_limit = $width - $BRACKET_CHARS - $full_limit
  bar_line = $BAR_CHAR_START + $BAR_CHAR_FULL * $full_limit + $BAR_CHAR_EMPTY * $empty_limit + $BAR_CHAR_END
  "${percent:3d}% $bar_line"

# Here is a sample of code that uses it.
on_terminal = (sys.stdout.isatty)
MAX_PERCENT = 100
for i in (range ($MAX_PERCENT + 1))
  time.sleep 0.01
  # ... Or run some other commands ...
  if $on_terminal
    print (progress_bar $BAR_WIDTH $i) end="\r" flush=True
  elif $i % 25 == 0
    print (progress_bar $BAR_WIDTH $i)
if $on_terminal
  print
```

```output
  0% [................................................]
 25% [============....................................]
 50% [========================........................]
 75% [====================================............]
100% [================================================]
```

**Comment blocks with `if`.** A bash trick: an `if` whose test is false
skips a block of text that's never parsed.

```bash
#!/bin/bash

COMMENT_BLOCK=
#  Try setting the above variable to some value
#+ for an unpleasant surprise.

if [ $COMMENT_BLOCK ]; then

Comment block --
=================================
This is a comment line.
This is another comment line.
This is yet another comment line.
=================================

echo "This will not echo."

Comment blocks are error-free! Whee!

fi

echo "No more comments, please."

exit 0
```

A tessh file is compiled whole, so text in a skipped block is still an
error. A string that nothing uses is the tessh way (or `#_`, which drops
the indented block under it):

```tessh
#!/usr/bin/env tessh
"""
Comment block --
=================================
This is a comment line.
This is another comment line.
This is yet another comment line.
=================================
print "This will not echo."
"""

#_ print "This will not echo either."
  print "nor this."

print "No more comments, please."
```

```output
No more comments, please.
```

**Testing for an integer** with the exit status of `test`:

```bash no-run
#!/bin/bash

SUCCESS=0
E_BADINPUT=85

test "$1" -ne 0 -o "$1" -eq 0 2>/dev/null
# An integer is either equal to 0 or not equal to 0.
# 2>/dev/null suppresses error message.

if [ $? -ne "$SUCCESS" ]
then
  echo "Usage: `basename $0` integer-input"
  exit $E_BADINPUT
fi

let "sum = $1 + 25"             # Would give error if $1 not integer.
echo "Sum = $sum"

# Any variable, not just a command-line parameter, can be tested this way.

exit 0
```

In tessh, `int` either converts the text or raises `ValueError`, and the
`try` is the test:

```tessh
#!/usr/bin/env tessh
E_BADINPUT = 85
arg = if (len $argv) > 1 then $argv[1] else "17"   # (sample argument)

n = try
  int $arg
except ValueError
  print "Usage: $((Path $argv[0]).name) integer-input"
  exit $E_BADINPUT

print "Sum = $($n + 25)"
```

```output
Sum = 42
```

**Return values.** A bash function's return status only goes from 0 to
255, so bash functions hand back results by echoing them, to be captured
with command substitution. A tessh function returns a value, any value,
so none of the following tricks is needed; they're here to show the
straightforward version.

### Example 36-18. Return value trickery (`multiplication.sh`)

```bash
#!/bin/bash
# multiplication.sh

multiply ()                     # Multiplies params passed.
{                               # Will accept a variable number of args.

  local product=1

  until [ -z "$1" ]             # Until uses up arguments passed...
  do
    let "product *= $1"
    shift
  done

  echo $product                 #  Will not echo to stdout,
}                               #+ since this will be assigned to a variable.

mult1=15383; mult2=25211
val1=`multiply $mult1 $mult2`
# Assigns stdout (echo) of function to the variable val1.
echo "$mult1 X $mult2 = $val1"                   # 387820813

mult1=25; mult2=5; mult3=20
val2=`multiply $mult1 $mult2 $mult3`
echo "$mult1 X $mult2 X $mult3 = $val2"          # 2500

mult1=188; mult2=37; mult3=25; mult4=47
val3=`multiply $mult1 $mult2 $mult3 $mult4`
echo "$mult1 X $mult2 X $mult3 X $mult4 = $val3" # 8173300

exit 0
```

```tessh
#!/usr/bin/env tessh
# multiplication

def multiply *factors           # Multiplies params passed,
  product = 1                   # any number of them.
  for f in $factors
    product *= $f
  $product                      # Returned (math.prod does all this).

mult1 = 15383 ; mult2 = 25211
val1 = multiply $mult1 $mult2
print "$mult1 X $mult2 = $val1"                   # 387820813

mult1 = 25 ; mult2 = 5 ; mult3 = 20
val2 = multiply $mult1 $mult2 $mult3
print "$mult1 X $mult2 X $mult3 = $val2"          # 2500

mult1 = 188 ; mult2 = 37 ; mult3 = 25 ; mult4 = 47
val3 = multiply $mult1 $mult2 $mult3 $mult4
print "$mult1 X $mult2 X $mult3 X $mult4 = $val3" # 8173300
```

```output
15383 X 25211 = 387820813
25 X 5 X 20 = 2500
188 X 37 X 25 X 47 = 8173300
```

The same goes for text: a function can return a string.

```bash
capitalize_ichar ()          #  Capitalizes initial character
{                            #+ of argument string(s) passed.

  string0="$@"               # Accepts multiple arguments.

  firstchar=${string0:0:1}   # First character.
  string1=${string0:1}       # Rest of string(s).

  FirstChar=`echo "$firstchar" | tr a-z A-Z`
                             # Capitalize first character.

  echo "$FirstChar$string1"  # Output to stdout.

}  

newstring=`capitalize_ichar "every sentence should start with a capital letter."`
echo "$newstring"          # Every sentence should start with a capital letter.
```

```tessh
def capitalize_ichar *words  #  Capitalizes initial character
  string0 = " ".join $words  #+ of argument string(s) passed.
  ($string0[:1].upper) + $string0[1:]

newstring = capitalize_ichar "every sentence should start with a capital letter."
print $newstring           # Every sentence should start with a capital letter.
```

```output
Every sentence should start with a capital letter.
```

### Example 36-19. Even more return value trickery (`sum-product.sh`)

```bash no-run
#!/bin/bash
# sum-product.sh
# A function may "return" more than one value.

sum_and_product ()   # Calculates both sum and product of passed args.
{
  echo $(( $1 + $2 )) $(( $1 * $2 ))
# Echoes to stdout each calculated value, separated by space.
}

echo
echo "Enter first number "
read first

echo
echo "Enter second number "
read second
echo

retval=`sum_and_product $first $second`      # Assigns output of function.
sum=`echo "$retval" | awk '{print $1}'`      # Assigns first field.
product=`echo "$retval" | awk '{print $2}'`  # Assigns second field.

echo "$first + $second = $sum"
echo "$first * $second = $product"
echo

exit 0
```

Several values come back as a tuple and unpack into names, with no `awk`
to pick the fields apart, and an extra `print` in the function can't spoil
them (the bash version breaks if the function echoes anything else). The
two numbers are canned here.

```tessh
#!/usr/bin/env tessh
# sum-product: a function may return more than one value.

def sum_and_product a b   # Calculates both sum and product of passed args.
  ($a + $b), ($a * $b)

answers = iter ["6" "7"]  # (input "", interactively)
print
print "Enter first number "
first = int (next $answers)
print
print "Enter second number "
second = int (next $answers)
print

[sum product] = sum_and_product $first $second

print "$first + $second = $sum"
print "$first * $second = $product"
print
```

```output

Enter first number 

Enter second number 

6 + 7 = 13
6 * 7 = 42

```

### Example 36-20. Passing and returning arrays (`array-function.sh`)

```bash
#!/bin/bash
# array-function.sh: Passing an array to a function and ...
#                   "returning" an array from a function


Pass_Array ()
{
  local passed_array   # Local variable!
  passed_array=( `echo "$1"` )
  echo "${passed_array[@]}"
  #  List all the elements of the new array
  #+ declared and set within the function.
}


original_array=( element1 element2 element3 element4 element5 )

echo
echo "original_array = ${original_array[@]}"
#                      List all elements of original array.


# This is the trick that permits passing an array to a function.
# **********************************
argument=`echo ${original_array[@]}`
# **********************************
#  Pack a variable
#+ with all the space-separated elements of the original array.
#
# Attempting to just pass the array itself will not work.


# This is the trick that allows grabbing an array as a "return value".
# *****************************************
returned_array=( `Pass_Array "$argument"` )
# *****************************************
# Assign 'echoed' output of function to array variable.

echo "returned_array = ${returned_array[@]}"

echo "============================================================="

#  Now, try it again,
#+ attempting to access (list) the array from outside the function.
Pass_Array "$argument"

# The function itself lists the array, but ...
#+ accessing the array from outside the function is forbidden.
echo "Passed array (within function) = ${passed_array[@]}"
# NULL VALUE since the array is a variable local to the function.

echo

############################################

# And here is an even more explicit example:

ret_array ()
{
  for element in {11..20}
  do
    echo "$element "   #  Echo individual elements
  done                 #+ of what will be assembled into an array.
}

arr=( $(ret_array) )   #  Assemble into array.

echo "Capturing array \"arr\" from function ret_array () ..."
echo "Third element of array \"arr\" is ${arr[2]}."   # 13  (zero-indexed)
echo -n "Entire array is: "
echo ${arr[@]}                # 11 12 13 14 15 16 17 18 19 20

echo

exit 0

#  Nathan Coulter points out that passing arrays with elements containing
#+ whitespace breaks this example.
```

A list is a value like any other: it goes into a function and comes back
out as itself, elements with spaces and all (which, as Nathan Coulter
points out, break the bash version). `*$list` in a `print` lists its
elements as `${array[@]}` does.

```tessh
#!/usr/bin/env tessh
# array-function: passing a list to a function and returning one.

def pass_array passed_array      # A local name, as in bash.
  list $passed_array             # Returns a copy of it.

original_array = [element1 element2 element3 element4 element5]

print
print "original_array =" *$original_array

returned_array = pass_array $original_array
print "returned_array =" *$returned_array

print "============================================================="

#  Now, try it again,
#+ attempting to access the list from outside the function.
print *(pass_array $original_array)
print "Passed array (within function) = $((globals).get 'passed_array' '')"
# Nothing: the name is local to the function.

print

# And here is an even more explicit example:

def ret_array
  list (range 11 21)             # [11 12 ... 20]

arr = (ret_array)
print "Capturing array \"arr\" from function ret_array () ..."
print "Third element of array \"arr\" is $arr[2]."   # 13  (zero-indexed)
print "Entire array is:" *$arr   # 11 12 13 14 15 16 17 18 19 20
print
```

```output

original_array = element1 element2 element3 element4 element5
returned_array = element1 element2 element3 element4 element5
=============================================================
element1 element2 element3 element4 element5
Passed array (within function) = 

Capturing array "arr" from function ret_array () ...
Third element of array "arr" is 13.
Entire array is: 11 12 13 14 15 16 17 18 19 20

```

**More tips.** The C-style `(( ))` loops of bash are `for i in (range …)`
and `i += 1` in tessh. Setting `PATH` and `umask` at the top makes a
script safer on a machine whose user has a strange setup (tessh's
`umask 022` works too):

```bash no-run
#!/bin/bash
PATH=/bin:/usr/bin:/usr/local/bin ; export PATH
umask 022   # Files that the script creates will have 755 permission.

# Thanks to Ian D. Allen, for this tip.
```

```tessh no-run
#!/usr/bin/env tessh
import os
$env.PATH = "/bin:/usr/bin:/usr/local/bin"
os.umask 0o022   # Files the script creates will have 755 permission.
```

Feeding a filter's output back into the same filter with other options
is as good a technique in tessh as in bash (from `wstrings.sh`):

```bash no-run
wlist=`strings "$1" | tr A-Z a-z | tr '[:space:]' Z | \
tr -cs '[:alpha:]' Z | tr -s '\173-\377' Z | tr Z ' '`
```

### Example 36-21. Fun with anagrams (`agram.sh`)

```bash no-run
#!/bin/bash
# agram.sh: Playing games with anagrams.

# Find anagrams of...
LETTERSET=etaoinshrdlu
FILTER='.......'       # How many letters minimum?
#       1234567

anagram "$LETTERSET" | # Find all anagrams of the letterset...
grep "$FILTER" |       # With at least 7 letters,
grep '^is' |           # starting with 'is'
grep -v 's$' |         # no plurals
grep -v 'ed$'          # no past tense verbs
# Possible to add many combinations of conditions and filters.

#  Uses "anagram" utility
#+ that is part of the author's "yawl" word list package.
#  http://ibiblio.org/pub/Linux/libs/yawl-0.3.2.tar.gz
#  http://bash.deta.in/yawl-0.3.2.tar.gz

exit 0                 # End of code.


bash$ sh agram.sh
islander
isolate
isolead
isotheral



#  Exercises:
#  ---------
#  Modify this script to take the LETTERSET as a command-line parameter.
#  Parameterize the filters in lines 11 - 13 (as with $FILTER),
#+ so that they can be specified by passing arguments to a function.

#  For a slightly different approach to anagramming,
#+ see the agram2.sh script.
```

The `anagram` program from the yawl package is rarely installed, so here
it's a tessh function over a small made-up word list: a word fits when
`Counter` says it needs no letter more often than the set has it. Its
output starts the same pipe of `grep`s. The letter set comes from the
command line, as the first exercise asks.

```tessh
#!/usr/bin/env tessh
# agram: playing games with anagrams.
from collections import Counter

# (A small made-up word list, in place of yawl's.)
WORDS = [hailstone island islander isolate isolated isolead isohel
         isotheral isotherm outlines sardine islands]

def anagram letterset          # The words made of the set's letters.
  have = Counter $letterset
  [$w for w in $WORDS if not ((Counter $w) - $have)]

# Find anagrams of...
LETTERSET = if (len $argv) > 1 then $argv[1] else etaoinshrdlu
FILTER = '.......'             # How many letters minimum?

anagram $LETTERSET |
  grep $FILTER |
  grep '^is' |
  grep -v 's$' |
  grep -v 'ed$'
# Find all anagrams of the letterset, with at least 7 letters,
# starting with 'is', no plurals, no past tense verbs.
```

```output
islander
isolate
isolead
isotheral
```

Anonymous here documents can comment out blocks of code (in tessh, an
unused string). And before relying on a program that may not be
installed, check for it; `which` in tessh gives back its path, or a false
value when there's none:

```bash no-run
CMD=command1                 # First choice.
PlanB=command2               # Fallback option.

command_test=$(whatis "$CMD" | grep 'nothing appropriate')
#  If 'command1' not found on system , 'whatis' will return
#+ "command1: nothing appropriate."
#
#  A safer alternative is:
#     command_test=$(whereis "$CMD" | grep \/)
#  But then the sense of the following test would have to be reversed,
#+ since the $command_test variable holds content only if
#+ the $CMD exists on the system.
#     (Thanks, bojster.)


if [[ -z "$command_test" ]]  # Check whether command present.
then
  $CMD option1 option2       #  Run command1 with options.
else                         #  Otherwise,
  $PlanB                     #+ run command2. 
fi
```

```tessh
import shutil
CMD = command1-not-installed    # First choice.
PlanB = echo                    # Fallback option.

if shutil.which $CMD            # Check whether command present.
  $CMD option1 option2          #  Run command1 with options.
else                            #  Otherwise,
  $PlanB "plan B"               #+ run command2.
```

```output
plan B
```

An `if`-`grep` test can miss text that went to stderr; send it into the
pipe with `2>&1`:

```bash
if ls -l nonexistent_filename | grep -q 'No such file or directory'
  then echo "File \"nonexistent_filename\" does not exist."
fi

if ls -l nonexistent_filename 2>&1 | grep -q 'No such file or directory'
#                             ^^^^
  then echo "File \"nonexistent_filename\" does not exist."
fi

# Thanks, Chris Martin, for pointing this out.
```

```tessh
if ls -l nonexistent_filename 2>/dev/null | grep -q 'No such file or directory'
  print "File \"nonexistent_filename\" does not exist."     # (Never: it went to stderr.)

if ls -l nonexistent_filename 2>&1 | grep -q 'No such file or directory'
  print "File \"nonexistent_filename\" does not exist."
# (Or in Python: if not ((Path nonexistent_filename).exists) …)
```

```output
File "nonexistent_filename" does not exist.
```

Getting a variable out of a bash subshell takes a temp file. tessh has no
subshells, so there's nothing to get out.

```bash no-run
TMPFILE=tmpfile                  # Create a temp file to store the variable.

(   # Inside the subshell ...
inner_variable=Inner
echo $inner_variable
echo $inner_variable >>$TMPFILE  # Append to temp file.
 )

    # Outside the subshell ...

echo; echo "-----"; echo
echo $inner_variable             # Null, as expected.
echo "-----"; echo

# Now ...
read inner_variable <$TMPFILE    # Read back shell variable.
rm -f "$TMPFILE"                 # Get rid of temp file.
echo "$inner_variable"           # It's an ugly kludge, but it works.
```

`run-parts` runs a folder of scripts in order (handy with `cron`), and a
revision control system keeps a script's history; today that's `git`
rather than `rcs`.

### Widgets

Dialog boxes from a script: the `dialog` family (`dialog` in a text
console; `gdialog`, `Xdialog`, `kdialog` and `zenity` with windows),
`xmessage` for a simple message, and Tk, PerlTk, PyQt and the like for
more. Python comes with Tk, as `tkinter`.

### Example 36-22. Widgets invoked from a shell script (`dialog.sh`)

```bash no-run
#!/bin/bash
# dialog.sh: Using 'gdialog' widgets.

# Must have 'gdialog' installed on your system to run this script.
# Or, you can replace all instance of 'gdialog' below with 'kdialog' ...
# Version 1.1 (corrected 04/05/05)

# This script was inspired by the following article.
#     "Scripting for X Productivity," by Marco Fioretti,
#      LINUX JOURNAL, Issue 113, September 2003, pp. 86-9.
# Thank you, all you good people at LJ.


# Input error in dialog box.
E_INPUT=85
# Dimensions of display, input widgets.
HEIGHT=50
WIDTH=60

# Output file name (constructed out of script name).
OUTFILE=$0.output

# Display this script in a text widget.
gdialog --title "Displaying: $0" --textbox $0 $HEIGHT $WIDTH



# Now, we'll try saving input in a file.
echo -n "VARIABLE=" > $OUTFILE
gdialog --title "User Input" --inputbox "Enter variable, please:" \
$HEIGHT $WIDTH 2>> $OUTFILE


if [ "$?" -eq 0 ]
# It's good practice to check exit status.
then
  echo "Executed \"dialog box\" without errors."
else
  echo "Error(s) in \"dialog box\" execution."
        # Or, clicked on "Cancel", instead of "OK" button.
  rm $OUTFILE
  exit $E_INPUT
fi



# Now, we'll retrieve and display the saved variable.
. $OUTFILE   # 'Source' the saved file.
echo "The variable input in the \"input box\" was: "$VARIABLE""


rm $OUTFILE  # Clean up by removing the temp file.
             # Some applications may need to retain this file.

exit $?

# Exercise: Rewrite this script using the 'zenity' widget set.
```

It needs a display and a person, so it isn't run here. The input box
writes the answer on stderr; capturing that directly (`2>&1` with the box's
own screen output thrown away) gives the value without the temp file and
`source`. This is the exercise's `zenity`, which writes the answer to
stdout instead, so it's simpler still.

```tessh no-run
#!/usr/bin/env tessh
# dialog: using zenity widgets.

E_INPUT = 85

# Display this script in a text widget.
zenity --text-info --title "Displaying: $argv[0]" --filename $argv[0]

# Now, we'll ask for input.
answer = zenity --entry --title "User Input" --text "Enter variable, please:"

if $answer.ok                  # It's good practice to check exit status.
  print "Executed \"dialog box\" without errors."
else
  print "Error(s) in \"dialog box\" execution."   # Or, clicked on Cancel.
  exit $E_INPUT

print "The variable input in the \"input box\" was: $answer"
```

`xmessage Fatal error in script! -button exit` pops up a message, and
`zenity --entry` a question whose answer it prints:

```bash no-run
get_info ()
{
  zenity --entry       #  Pops up query window . . .
                       #+ and prints user entry to stdout.

                       #  Also try the --calendar and --scale options.
}

answer=$( get_info )   #  Capture stdout in $answer variable.

echo "User entered: "$answer""
```

In tessh, `answer = zenity --entry` and `print "User entered: $answer"`.

## 36.8 Security Issues
<!-- help: abs-securityissues security secure-scripts malware secrets path -->

**Infected scripts.** A script can carry a worm, a trojan or a virus, and
it's easy to write one. Never run a script as root, or let it into the
startup scripts, unless it comes from a source you trust or you've read
it and know what it does. Being able to read scripts is one more reason
to learn to write them. The same holds for tessh scripts, which can do
anything Python can.

**Hiding the source.** `shc` compiles a shell script into a binary, though
the source can sometimes be recovered. For tessh, `tessh --py` shows the
Python a script becomes, which hides nothing either. A script you can run
is a script someone can read.

**Writing secure scripts**, after Dan Stromberg:

- Don't put secret data in environment variables.
- Don't pass secret data in a program's arguments, where `ps` shows them
  to everyone; pass it on standard input (`<<<$secret`, a pipe) instead.
- Set `PATH` yourself, especially in a script run as root, and think about
  what a caller could do by setting any variable you take from the
  environment (`HOME=/etc`, say).

tessh helps in one way: a value is always one argument, never split or
globbed, so text from outside can't turn into extra arguments or file
names. It can still turn into code if you hand it to Python's `eval` or
`exec`, or to `sh -c`; don't.

## 36.9 Portability Issues
<!-- help: abs-portabilityissues portability posix sh bash-features -->

Larry Wall: it is easier to port a shell than a shell script.

Shells have been moving toward POSIX 1003.2, and `bash --posix`, `set -o
posix` or a `#!/bin/sh` header keeps a bash script close to it. Most bash
scripts run under `ksh` and the other way round. GNU tools have mostly
replaced the commercial UNIX ones, so GNU options are less of a problem
than they were.

Bash has features the Bourne shell lacks: extended options, `$( )`, brace
expansion, arrays and associative arrays, `[[ ]]`, `(( ))`, string
operations, process substitution, `=~`, its own builtins and coprocesses.

A tessh script is portable wherever tessh runs. What it does in Python is
the same on every system; the programs it runs are as portable as they
are in bash.

### A Test Suite

The ABS Guide tests a shell for some of bash's features. The same test
suite for tessh, with brace expansion added (which fails: see chapter 33):

### Example 36-23. Test Suite (`test-suite.sh`)

```bash
#!/bin/bash
# test-suite.sh
# A partial Bash compatibility test suite.
# Run this on your version of Bash, or some other shell.

default_option=FAIL         # Tests below will fail unless . . .

echo
echo -n "Testing "
sleep 1; echo -n ". "
sleep 1; echo -n ". "
sleep 1; echo ". "
echo

# Double brackets
String="Double brackets supported?"
echo -n "Double brackets test: "
if [[ "$String" = "Double brackets supported?" ]]
then
  echo "PASS"
else
  echo "FAIL"
fi


# Double brackets and regex matching
String="Regex matching supported?"
echo -n "Regex matching: "
if [[ "$String" =~ R.....matching* ]]
then
  echo "PASS"
else
  echo "FAIL"
fi


# Arrays
test_arr=$default_option     # FAIL
Array=( If supports arrays will print PASS )
test_arr=${Array[5]}
echo "Array test: $test_arr"


# Command Substitution
csub_test ()
{
  echo "PASS"
}

test_csub=$default_option    # FAIL
test_csub=$(csub_test)
echo "Command substitution test: $test_csub"

echo

#  Completing this script is an exercise for the reader.
#  Add to the above similar tests for double parentheses,
#+ brace expansion, process substitution, etc.

exit $?
```

```output bash

Testing . . . 

Double brackets test: PASS
Regex matching: PASS
Array test: PASS
Command substitution test: PASS

```

```tessh
#!/usr/bin/env tessh
# test-suite: a partial tessh test suite, after the ABS Guide's.

default_option = FAIL       # Tests below will fail unless . . .

print
print "Testing " end=""
for n in (range 3)
  sleep 0.1                 # (1 second in the original)
  print ". " end="" flush=True
print ; print

# Comparison
String = "Comparison supported?"
print "Comparison test:" (if $String == "Comparison supported?" then PASS else FAIL)

# Regex matching
String = "Regex matching supported?"
print "Regex matching:" (if $String =~ re'R.....matching*' then PASS else FAIL)

# Lists
Array = [If supports arrays will print PASS]
test_arr = (if (len $Array) > 5 then $Array[5] else $default_option)
print "Array test: $test_arr"

# Command substitution: a program's output, captured.
test_csub = echo PASS
print "Command substitution test: $test_csub"

# Process substitution
test_psub = cat <(echo PASS)
print "Process substitution test: $test_psub"

# Brace expansion
test_brace = echo {P,}ASS
print "Brace expansion test:" (if $test_brace == "PASS ASS" then PASS else FAIL)

print
```

```output

Testing . . . 

Comparison test: PASS
Regex matching: PASS
Array test: PASS
Command substitution test: PASS
Process substitution test: PASS
Brace expansion test: FAIL

```

## 36.10 Shell Scripting Under Windows
<!-- help: abs-winscript windows cygwin wsl powershell -->

Windows users can run UNIX-style scripts with Cygwin, the MKS tools, or
David Korn's UWIN, and Windows PowerShell has some bash-like scripting of
its own. Today the usual way is the Windows Subsystem for Linux (WSL),
which runs a real Linux, bash and tessh included, as on any Linux system.
