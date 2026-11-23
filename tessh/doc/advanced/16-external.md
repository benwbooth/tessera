# 16. External Filters, Programs and Commands
<!-- help: abs-external abs-16 external programs filters commands utilities -->

A shell script gets most of its power from the programs it runs: `ls`,
`find`, `sort`, `sed`, `awk`, `tar` and the rest. tessh runs them exactly as
bash does, so the commands in this chapter work as they stand. What changes
is the glue between them: where bash pipes text through `expr`, `cut` and
backquotes to get at a number or a file name, tessh has Python values,
Paths, Globs and `re'…'`, and a program's output is an `Output` string with
`.lines` and `.code`.

## 16.1 Basic Commands
<!-- help: abs-basic-commands ls cat tac rev cp mv rm rmdir mkdir chmod chattr ln man info -->

The commands a beginner learns first. `ls` lists files (`-R` recursive, `-S`
by size, `-t` by time, `-v` by version numbers in the names, `-i` with inode
numbers), and fails with status 2 on a name that isn't there. `cat` lists
and joins files (`-n` numbers lines, `-s` squeezes blank ones), `tac` lists
them last line first and `rev` mirrors each line. `cp` copies (`-a` a whole
tree, `-u` only newer files), `mv` moves or renames, `rm` removes (`-f`
without asking, `-r` a whole tree), `rmdir` removes an empty folder, `mkdir
-p` makes one with its parents, `chmod` sets permissions, `chattr` sets
ext2/3/4 attributes such as immutable (`+i`), and `ln -s` makes a symbolic
link. `man` and `info` show the manuals.

All of these are programs, and tessh runs them as bash does. Python has
most of them as well (`Path.mkdir`, `Path.rename`, `Path.unlink`,
`shutil.copytree`, `os.chmod`), which is handy when the names come from a
Python value, but nothing stops a tessh script from using the programs.

```bash
printf 'This is line 1.\nThis is line 2.\n' > file1.txt
cat file1.txt
tac file1.txt
rev file1.txt
```

```output bash
This is line 1.
This is line 2.
This is line 2.
This is line 1.
.1 enil si sihT
.2 enil si sihT
```

```tessh
printf 'This is line 1.\nThis is line 2.\n' > file1.txt
cat file1.txt
tac file1.txt
rev file1.txt
# The same from Python: the lines are a list.
lines = (cat file1.txt).lines
print ("\n".join (reversed $lines))
print ("\n".join [$l[::-1] for l in $lines])
```

```output
This is line 1.
This is line 2.
This is line 2.
This is line 1.
.1 enil si sihT
.2 enil si sihT
This is line 2.
This is line 1.
.1 enil si sihT
.2 enil si sihT
```

A file whose name starts with a dash looks like an option to `rm`. As in
bash, `--` ends the options, or `./` in front makes it a path. A Path
value never has that problem in Python: `$p.unlink` takes it as a name.

```bash
touch -- -badname
rm -badname 2>/dev/null || echo "rm took it for an option"
rm -- -badname && echo removed
```

```output bash
rm took it for an option
removed
```

```tessh
touch -- -badname
rm -badname 2>/dev/null || echo "rm took it for an option"
rm -- -badname && echo removed
touch ./-badname
(Path "-badname").unlink
print ((Path "-badname").exists)
```

```output
rm took it for an option
removed
False
```

### Example 16-1. Using `ls` to create a table of contents for burning a CDR disk (`ex40.sh`)

```bash no-run
#!/bin/bash
# ex40.sh (burn-cd.sh)
# Script to automate burning a CDR.


SPEED=10         # May use higher speed if your hardware supports it.
IMAGEFILE=cdimage.iso
CONTENTSFILE=contents
# DEVICE=/dev/cdrom     For older versions of cdrecord
DEVICE="1,0,0"
DEFAULTDIR=/opt  # This is the directory containing the data to be burned.
                 # Make sure it exists.
                 # Exercise: Add a test for this.

# Uses Joerg Schilling's "cdrecord" package:
# http://www.fokus.fhg.de/usr/schilling/cdrecord.html

#  If this script invoked as an ordinary user, may need to suid cdrecord
#+ chmod u+s /usr/bin/cdrecord, as root.
#  Of course, this creates a security hole, though a relatively minor one.

if [ -z "$1" ]
then
  IMAGE_DIRECTORY=$DEFAULTDIR
  # Default directory, if not specified on command-line.
else
    IMAGE_DIRECTORY=$1
fi

# Create a "table of contents" file.
ls -lRF $IMAGE_DIRECTORY > $IMAGE_DIRECTORY/$CONTENTSFILE
# The "l" option gives a "long" file listing.
# The "R" option makes the listing recursive.
# The "F" option marks the file types (directories get a trailing /).
echo "Creating table of contents."

# Create an image file preparatory to burning it onto the CDR.
mkisofs -r -o $IMAGEFILE $IMAGE_DIRECTORY
echo "Creating ISO9660 file system image ($IMAGEFILE)."

# Burn the CDR.
echo "Burning the disk."
echo "Please be patient, this will take a while."
wodim -v -isosize dev=$DEVICE $IMAGEFILE
#  In newer Linux distros, the "wodim" utility assumes the
#+ functionality of "cdrecord."
exitcode=$?
echo "Exit code = $exitcode"

exit $exitcode
```

It needs a CD burner, so it doesn't run here. The default directory is a
Path, the table of contents goes to `$IMAGE_DIRECTORY / $CONTENTSFILE`, and
the exercise (check that the folder exists) is one `if`.

```tessh no-run
#!/usr/bin/env tessh
# burn-cd: Script to automate burning a CDR.

SPEED = 10           # May use higher speed if your hardware supports it.
IMAGEFILE = cdimage.iso
CONTENTSFILE = contents
DEVICE = "1,0,0"
DEFAULTDIR = /opt    # The directory containing the data to be burned.

# The directory on the command line, else the default.
IMAGE_DIRECTORY = if $argv[1:] then Path $argv[1] else $DEFAULTDIR
if not ($IMAGE_DIRECTORY.is_dir)
  print "No such directory: $IMAGE_DIRECTORY"
  exit 1

# Create a "table of contents" file: long, recursive, types marked.
ls -lRF $IMAGE_DIRECTORY >($IMAGE_DIRECTORY / $CONTENTSFILE)
print "Creating table of contents."

# Create an image file preparatory to burning it onto the CDR.
mkisofs -r -o $IMAGEFILE $IMAGE_DIRECTORY
print "Creating ISO9660 file system image ($IMAGEFILE)."

# Burn the CDR.
print "Burning the disk."
print "Please be patient, this will take a while."
wodim -v -isosize dev=$DEVICE $IMAGEFILE
exitcode = $?
print "Exit code = $exitcode"

exit $exitcode
```

### Example 16-2. Hello or Good-bye (`hello.sh`)

```bash
#!/bin/bash
# hello.sh: Saying "hello" or "goodbye"
#+          depending on how script is invoked.

# Make a link in current working directory ($PWD) to this script:
#    ln -s hello.sh goodbye
# Now, try invoking this script both ways:
# ./hello.sh
# ./goodbye


HELLO_CALL=65
GOODBYE_CALL=66

if [ $0 = "./goodbye" ]
then
  echo "Good-bye!"
  # Some other goodbye-type commands, as appropriate.
  exit $GOODBYE_CALL
fi

echo "Hello!"
# Some other hello-type commands, as appropriate.
exit $HELLO_CALL
```

`$argv[0]` is bash's `$0`, the name the script was run by. Comparing its
`.name` rather than the whole text also works when the link is run from
another folder.

```tessh
#!/usr/bin/env tessh
# hello.tsh: Saying "hello" or "goodbye"
#            depending on how script is invoked.
#    ln -s hello.tsh goodbye

HELLO_CALL = 65
GOODBYE_CALL = 66

if (Path $argv[0]).name == "goodbye"
  print "Good-bye!"
  exit $GOODBYE_CALL

print "Hello!"
exit $HELLO_CALL
```

```output
Hello!
```

## 16.2 Complex Commands
<!-- help: abs-complex-commands find xargs expr -exec placeholder -->

`find` walks a folder tree and picks files by name (`-name '*.txt'`), type
(`-type f`), age (`-mtime -1`, `-atime +5`), permissions (`-perm +4000`),
inode (`-inum`) and more; `-exec COMMAND {} \;` runs a command on each, with
`{}` standing for the path. `xargs` turns its input into arguments for a
command: `-n N` at most N at a time, `-i`/`-I {}` with a placeholder, `-0`
for NUL-separated names (from `find -print0`), `-P N` in parallel. `expr`
does arithmetic, comparisons and string matching.

In tessh, `find` and `xargs` run as they are, `\;` included. Much of what
they're used for in bash is done with values, though: a Glob or
`Path.rglob` gives the files as Paths, and a `for` loop gives each to a
command as one argument, spaces and quotes and all, so there's no need for
`-print0 | xargs -0`. `expr` isn't needed at all: arithmetic and string
work is Python.

The ABS Guide's loop over the suid files that `find` finds:

```bash no-run
#!/bin/bash
#  Find suid root files.
#  A strange suid file might indicate a security hole,
#+ or even a system intrusion.

directory="/usr/sbin"
# Might also try /sbin, /bin, /usr/bin, /usr/local/bin, etc.
permissions="+4000"  # suid root (dangerous!)


for file in $( find "$directory" -perm "$permissions" )
do
  ls -ltF --author "$file"
done
```

`(find …).lines` is the list of paths, one per line, so a name with a
space in it stays whole (bash's `$( … )` would split it). What it finds
depends on the system, so this doesn't run here.

```tessh no-run
directory = /usr/sbin
permissions = "/4000"      # suid (GNU find spells "+4000" as "/4000" now)

for file in (find $directory -perm $permissions).lines
  ls -ltF --author $file
```

### Example 16-3. Badname, eliminate file names in current directory containing bad characters and whitespace (`ex57.sh`)

```bash no-run
#!/bin/bash
# badname.sh
# Delete filenames in current directory containing bad characters.

for filename in *
do
  badname=`echo "$filename" | sed -n /[\+\{\;\"\\\=\?~\(\)\<\>\&\*\|\$]/p`
# badname=`echo "$filename" | sed -n '/[+{;"\=?~()<>&*|$]/p'`  also works.
# Deletes files containing these nasties:     + { ; " \ = ? ~ ( ) < > & * | $
#
  rm $badname 2>/dev/null
#             ^^^^^^^^^^^ Error messages deep-sixed.
done

# Now, take care of files containing all manner of whitespace.
find . -name "* *" -exec rm -f {} \;
# The path name of the file that _find_ finds replaces the "{}".
# The '\' ensures that the ';' is interpreted literally, as end of command.

exit 0

#---------------------------------------------------------------------
# Commands below this line will not execute because of _exit_ command.

# An alternative to the above script:
find . -name '*[+{;"\\=?~()<>&*|$ ]*' -maxdepth 0 \
-exec rm -f '{}' \;
#  The "-maxdepth 0" option ensures that _find_ will not search
#+ subdirectories below $PWD.

# (Thanks, S.C.)
```

The name is tested with `=~` and a regex instead of being echoed through
`sed`, and `rm` gets it as one argument, so a bad name with a space in it
is removed too (bash's unquoted `$badname` splits it). `find … -exec rm -f
{} \;` is the same in tessh.

```tessh
# (Made-up files to try it on.)
touch good.txt plain 'a+b' 'semi;colon' 'what?' 'cost$5' 'two words' 'x<y'

for filename in *
  if $filename.name =~ re'[+{;"\\=?~()<>&*|$]'
    # Deletes files containing these nasties: + { ; " \ = ? ~ ( ) < > & * | $
    rm $filename

# Now, take care of files containing all manner of whitespace.
find . -name "* *" -exec rm -f {} \;
# The path name of the file that find finds replaces the "{}".

ls       # (What's left.)
```

```output
good.txt
plain
```

### Example 16-4. Deleting a file by its inode number (`idelete.sh`)

```bash no-run
#!/bin/bash
# idelete.sh: Deleting a file by its inode number.

#  This is useful when a filename starts with an illegal character,
#+ such as ? or -.

ARGCOUNT=1                      # Filename arg must be passed to script.
E_WRONGARGS=70
E_FILE_NOT_EXIST=71
E_CHANGED_MIND=72

if [ $# -ne "$ARGCOUNT" ]
then
  echo "Usage: `basename $0` filename"
  exit $E_WRONGARGS
fi  

if [ ! -e "$1" ]
then
  echo "File \""$1"\" does not exist."
  exit $E_FILE_NOT_EXIST
fi  

inum=`ls -i | grep "$1" | awk '{print $1}'`
# inum = inode (index node) number of file
# -----------------------------------------------------------------------
# Every file has an inode, a record that holds its physical address info.
# -----------------------------------------------------------------------

echo; echo -n "Are you absolutely sure you want to delete \"$1\" (y/n)? "
# The '-v' option to 'rm' also asks this.
read answer
case "$answer" in
[nN]) echo "Changed your mind, huh?"
      exit $E_CHANGED_MIND
      ;;
*)    echo "Deleting file \"$1\".";;
esac

find . -inum $inum -exec rm {} \;
#                           ^^
#        Curly brackets are placeholder
#+       for text output by "find."
echo "File "\"$1"\" deleted!"

exit 0
```

`os.stat` gives the inode number as an int, where bash cuts it out of `ls
-i` with `grep` and `awk` (and `grep` would also match other names that
contain this one). The answer is canned here, as stdin isn't usable.

```tessh
import os
# (A made-up file with a name that starts with a dash.)
touch -- -dashing.txt

ARGCOUNT = 1                    # Filename arg must be passed to script.
E_WRONGARGS = 70
E_FILE_NOT_EXIST = 71
E_CHANGED_MIND = 72

args = $argv[1:] or ["-dashing.txt"]   # (sample argument, when none is given)
if (len $args) != $ARGCOUNT
  print "Usage: $(Path $argv[0]).name filename"
  exit $E_WRONGARGS
name = $args[0]

if not (os.path.exists $name)
  print "File \"$name\" does not exist."
  exit $E_FILE_NOT_EXIST

inum = (os.stat $name).st_ino
# inum = inode (index node) number of file

print
print "Are you absolutely sure you want to delete \"$name\" (y/n)? " end=''
answer = y           # (canned; the original reads it from the terminal)
print $answer
match $answer
  case "n" | "N"
    print "Changed your mind, huh?"
    exit $E_CHANGED_MIND
  case _
    print "Deleting file \"$name\"."

find . -inum $inum -exec rm {} \;
print "File \"$name\" deleted!"
ls -A                # (Nothing left.)
exit 0
```

```output

Are you absolutely sure you want to delete "-dashing.txt" (y/n)? y
Deleting file "-dashing.txt".
File "-dashing.txt" deleted!
```

### Example 16-5. Logfile: Using `xargs` to monitor system log (`ex41.sh`)

```bash no-run
#!/bin/bash

# Generates a log file in current directory
# from the tail end of /var/log/messages.

# Note: /var/log/messages must be world readable
# if this script invoked by an ordinary user.
#         #root chmod 644 /var/log/messages

LINES=5

( date; uname -a ) >>logfile
# Time and machine name
echo ---------------------------------------------------------- >>logfile
tail -n $LINES /var/log/messages | xargs | fmt -s >>logfile
echo >>logfile
echo >>logfile

exit 0

#  Note:
#  ----
#  As Frank Wang points out,
#+ unmatched quotes (either single or double quotes) in the source file
#+ may give xargs indigestion.
#
#  He suggests the following substitution for line 15:
#  tail -n $LINES /var/log/messages | tr -d "\"'" | xargs | fmt -s >>logfile



#  Exercise:
#  --------
#  Modify this script to track changes in /var/log/messages at intervals
#+ of 20 minutes.
#  Hint: Use the "watch" command. 
```

Most systems log to the journal now, not `/var/log/messages`, so this one
reads a made-up `messages`. The pipeline is the same in tessh. bash's `(
date; uname -a ) >>logfile` runs both into one redirection; tessh has no
subshell group, so each line appends on its own (or open the file once
`with (open logfile a)`). At the end it shows the log without its first two
lines, which change from run to run.

```tessh
# (A made-up /var/log/messages to try it on.)
for i in (range 1 9)
  print "Nov 23 10:0$i host kernel: event $i" >>messages

LINES = 5

date >>logfile                # Time
uname -a >>logfile            # and machine name.
echo ---------------------------------------------------------- >>logfile
tail -n $LINES messages | xargs | fmt -s >>logfile
echo >>logfile
echo >>logfile

tail -n +3 logfile            # (The log, without the date and machine.)
exit 0
```

```output
----------------------------------------------------------
Nov 23 10:04 host kernel: event 4 Nov 23 10:05 host kernel: event 5 Nov
23 10:06 host kernel: event 6 Nov 23 10:07 host kernel: event 7 Nov 23
10:08 host kernel: event 8


```

### Example 16-6. Copying files in current directory to another (`ex42.sh`)

```bash no-run
#!/bin/bash
# copydir.sh

#  Copy (verbose) all files in current directory ($PWD)
#+ to directory specified on command-line.

E_NOARGS=85

if [ -z "$1" ]   # Exit if no argument given.
then
  echo "Usage: `basename $0` directory-to-copy-to"
  exit $E_NOARGS
fi  

ls . | xargs -i -t cp ./{} $1
#            ^^ ^^      ^^
#  -t is "verbose" (output command-line to stderr) option.
#  -i is "replace strings" option.
#  {} is a placeholder for output text.
#  This is similar to the use of a curly-bracket pair in "find."
#
#  List the files in current directory (ls .),
#+ pass the output of "ls" as arguments to "xargs" (-i -t options),
#+ then copy (cp) these arguments ({}) to new directory ($1).  
#
#  The net result is the exact equivalent of
#+   cp * $1
#+ unless any of the filenames has embedded "whitespace" characters.

exit 0
```

A `for` over the Glob `*` does what `ls . | xargs -i` does, and a name with
a space in it is still one argument. `cp -v` says what it copies, on
stdout, where `xargs -t` printed each command on stderr. Skipping folders
(the target among them) keeps `cp` from complaining about them.

```tessh
# (Made-up files, and the folder to copy them to.)
touch one.txt two.txt 'three with spaces.txt' ; mkdir backup

E_NOARGS = 85

args = $argv[1:] or [backup]     # (sample argument, when none is given)
if not $args or not $args[0]     # Exit if no argument given.
  print "Usage: $(Path $argv[0]).name directory-to-copy-to"
  exit $E_NOARGS
dest = $args[0]

for f in *
  if ($f.is_file)
    cp -v $f $dest
exit 0
```

```output
'one.txt' -> 'backup/one.txt'
'three with spaces.txt' -> 'backup/three with spaces.txt'
'two.txt' -> 'backup/two.txt'
```

### Example 16-7. Killing processes by name (`kill-byname.sh`)

```bash no-run
#!/bin/bash
# kill-byname.sh: Killing processes by name.
# Compare this script with kill-process.sh.

#  For instance,
#+ try "./kill-byname.sh xterm" --
#+ and watch all the xterms on your desktop disappear.

#  Warning:
#  -------
#  This is a fairly dangerous script.
#  Running it carelessly (especially as root)
#+ can cause data loss and other undesirable effects.

E_BADARGS=66

if test -z "$1"  # No command-line arg supplied?
then
  echo "Usage: `basename $0` Process(es)_to_kill"
  exit $E_BADARGS
fi


PROCESS_NAME="$1"
ps ax | grep "$PROCESS_NAME" | awk '{print $1}' | xargs -i kill {} 2&>/dev/null
#                                                       ^^      ^^

# ---------------------------------------------------------------
# Notes:
# -i is the "replace strings" option to xargs.
# The curly brackets are the placeholder for the replacement.
# 2&>/dev/null suppresses unwanted error messages.
#
# Can  grep "$PROCESS_NAME" be replaced by pidof "$PROCESS_NAME"?
# ---------------------------------------------------------------

exit $?

#  The "killall" command has the same effect as this script,
#+ but using it is not quite as educational.
```

Here it kills a `sleep` it started itself. The process list is read into
Python: `ps -eo pid=,args=` prints each process's number and command line,
and the loop splits off the number and compares the command. bash's
`grep "$PROCESS_NAME"` matches anywhere in the line, `grep`'s own process
included, which is what makes the original dangerous; comparing the whole
command line is exact.

```tessh
# (A process to kill.)
sleep 4242 &

E_BADARGS = 66

args = $argv[1:] or ["sleep 4242"]   # (sample argument, when none is given)
if not $args                         # No command-line arg supplied?
  print "Usage: $(Path $argv[0]).name Process(es)_to_kill"
  exit $E_BADARGS

PROCESS_NAME = $args[0]
for line in (ps -eo pid=,args=).lines
  [pid cmd] = ($line.strip).split " " 1
  if $cmd == $PROCESS_NAME
    kill $pid 2>/dev/null

sleep 0.2
print "still running:" (bool (pgrep -fx $PROCESS_NAME))
```

```output
still running: False
```

### Example 16-8. Word frequency analysis using `xargs` (`wf2.sh`)

```bash no-run
#!/bin/bash
# wf2.sh: Crude word frequency analysis on a text file.

# Uses 'xargs' to decompose lines of text into single words.
# Compare this example to the "wf.sh" script later on.


# Check for input file on command-line.
ARGS=1
E_BADARGS=85
E_NOFILE=86

if [ $# -ne "$ARGS" ]
# Correct number of arguments passed to script?
then
  echo "Usage: `basename $0` filename"
  exit $E_BADARGS
fi

if [ ! -f "$1" ]       # Does file exist?
then
  echo "File \"$1\" does not exist."
  exit $E_NOFILE
fi



#####################################################
cat "$1" | xargs -n1 | \
#  List the file, one word per line. 
tr A-Z a-z | \
#  Shift characters to lowercase.
sed -e 's/\.//g'  -e 's/\,//g' -e 's/ /\
/g' | \
#  Filter out periods and commas, and
#+ change space between words to linefeed,
sort | uniq -c | sort -nr
#  Finally remove duplicates, prefix occurrence count
#+ and sort numerically.
#####################################################

#  This does the same job as the "wf.sh" example,
#+ but a bit more ponderously, and it runs more slowly (why?).

exit $?
```

The pipeline runs as it is. A pipeline split over lines needs `\` at each
line's end in tessh too, but a comment can't sit between the pieces, so
the comments go above it. (Example 16-12 does the counting in Python.)

```tessh
# (A made-up text to try it on.)
cat >sample.txt <<'END'
The quick brown fox jumps over the lazy dog.
The dog sleeps, the fox runs.
A fox is quick; a dog is lazy.
END

ARGS = 1
E_BADARGS = 85
E_NOFILE = 86

args = $argv[1:] or [sample.txt]    # (sample argument, when none is given)
if (len $args) != $ARGS
  print "Usage: $(Path $argv[0]).name filename"
  exit $E_BADARGS
file = Path $args[0]
if not ($file.is_file)              # Does file exist?
  print "File \"$file\" does not exist."
  exit $E_NOFILE

# List the file one word per line, shift to lowercase, filter out periods
# and commas, then count each word and sort by the counts.
cat $file | xargs -n1 | tr A-Z a-z | sed -e 's/\.//g' -e 's/\,//g' -e 's/ /\n/g' | \
  sort | uniq -c | sort -nr
```

```output
      4 the
      3 fox
      3 dog
      2 lazy
      2 is
      2 a
      1 sleeps
      1 runs
      1 quick;
      1 quick
      1 over
      1 jumps
      1 brown
```

`expr` was the shell's calculator before `$(( ))`: `expr 3 + 5`, `expr 5
\* 3` (the `*` escaped from the shell), `y=`expr $y + 1``, `expr substr
$string $position $length`, and `expr match "$s" 'regex'` (or `expr "$s" :
'regex'`), which gives the length of the match at the start, or with
`\( … \)` the text it matched.

### Example 16-9. Using `expr` (`ex45.sh`)

```bash
#!/bin/bash

# Demonstrating some of the uses of 'expr'
# =======================================

echo

# Arithmetic Operators
# ---------- ---------

echo "Arithmetic Operators"
echo
a=`expr 5 + 3`
echo "5 + 3 = $a"

a=`expr $a + 1`
echo
echo "a + 1 = $a"
echo "(incrementing a variable)"

a=`expr 5 % 3`
# modulo
echo
echo "5 mod 3 = $a"

echo
echo

# Logical Operators
# ------- ---------

#  Returns 1 if true, 0 if false,
#+ opposite of normal Bash convention.

echo "Logical Operators"
echo

x=24
y=25
b=`expr $x = $y`         # Test equality.
echo "b = $b"            # 0  ( $x -ne $y )
echo

a=3
b=`expr $a \> 10`
echo 'b=`expr $a \> 10`, therefore...'
echo "If a > 10, b = 0 (false)"
echo "b = $b"            # 0  ( 3 ! -gt 10 )
echo

b=`expr $a \< 10`
echo "If a < 10, b = 1 (true)"
echo "b = $b"            # 1  ( 3 -lt 10 )
echo
# Note escaping of operators.

b=`expr $a \<= 3`
echo "If a <= 3, b = 1 (true)"
echo "b = $b"            # 1  ( 3 -le 3 )
# There is also a "\>=" operator (greater than or equal to).


echo
echo



# String Operators
# ------ ---------

echo "String Operators"
echo

a=1234zipper43231
echo "The string being operated upon is \"$a\"."

# length: length of string
b=`expr length $a`
echo "Length of \"$a\" is $b."

# index: position of first character in substring
#        that matches a character in string
b=`expr index $a 23`
echo "Numerical position of first \"2\" in \"$a\" is \"$b\"."

# substr: extract substring, starting position & length specified
b=`expr substr $a 2 6`
echo "Substring of \"$a\", starting at position 2,\
and 6 chars long is \"$b\"."


#  The default behavior of the 'match' operations is to
#+ search for the specified match at the BEGINNING of the string.
#
#       Using Regular Expressions ...
b=`expr match "$a" '[0-9]*'`               #  Numerical count.
echo Number of digits at the beginning of \"$a\" is $b.
b=`expr match "$a" '\([0-9]*\)'`           #  Note that escaped parentheses
#                   ==      ==             #+ trigger substring match.
echo "The digits at the beginning of \"$a\" are \"$b\"."

echo

exit 0
```

Each `expr` is a Python expression. A comparison is `True` or `False`, so
`int` turns it into `expr`'s 1 or 0. Strings count from 0 in Python and
from 1 in `expr`, hence the `+ 1` for `index` and the `[1:7]` slice for
`substr $a 2 6`. A regex match is `$a =~ re'…'`, with the length and the
text from the match object.

```tessh
# Demonstrating what replaces 'expr'
# ==================================

print

# Arithmetic Operators
print "Arithmetic Operators"
print
a = 5 + 3
print "5 + 3 = $a"

a += 1
print
print "a + 1 = $a"
print "(incrementing a variable)"

a = 5 % 3            # modulo
print
print "5 mod 3 = $a"

print
print

# Logical Operators: True and False; int makes them expr's 1 and 0.
print "Logical Operators"
print

x = 24
y = 25
b = int ($x == $y)       # Test equality.
print "b = $b"           # 0
print

a = 3
b = int ($a > 10)
print 'b=`expr $a \> 10`, therefore...'
print "If a > 10, b = 0 (false)"
print "b = $b"           # 0
print

b = int ($a < 10)
print "If a < 10, b = 1 (true)"
print "b = $b"           # 1
print

b = int ($a <= 3)
print "If a <= 3, b = 1 (true)"
print "b = $b"           # 1

print
print

# String Operators
print "String Operators"
print

a = "1234zipper43231"
print "The string being operated upon is \"$a\"."

# length: length of string
b = len $a
print "Length of \"$a\" is $b."

# index: position (from 1) of the first character that is in "23"
b = (min ($i for [i c] in (enumerate $a) if $c in "23")) + 1
print "Numerical position of first \"2\" in \"$a\" is \"$b\"."

# substr: 6 characters from position 2 (index 1 in Python)
b = $a[1:7]
# (The bash's quoted line break swallowed the space after the comma.)
print "Substring of \"$a\", starting at position 2,and 6 chars long is \"$b\"."

# Regular expressions: =~ with ^ to match at the beginning.
m = $a =~ re'^[0-9]*'
print "Number of digits at the beginning of \"$a\" is $(len ($m.group))."
print "The digits at the beginning of \"$a\" are \"$($m.group)\"."

print
exit 0
```

```output

Arithmetic Operators

5 + 3 = 8

a + 1 = 9
(incrementing a variable)

5 mod 3 = 2


Logical Operators

b = 0

b=`expr $a \> 10`, therefore...
If a > 10, b = 0 (false)
b = 0

If a < 10, b = 1 (true)
b = 1

If a <= 3, b = 1 (true)
b = 1


String Operators

The string being operated upon is "1234zipper43231".
Length of "1234zipper43231" is 15.
Numerical position of first "2" in "1234zipper43231" is "2".
Substring of "1234zipper43231", starting at position 2,and 6 chars long is "234zip".
Number of digits at the beginning of "1234zipper43231" is 4.
The digits at the beginning of "1234zipper43231" are "1234".

```

The ABS Guide follows it with the `expr "$string" : 'regex'` form, where
escaped parentheses pick out the part of the match to return:

```bash
#!/bin/bash

echo
echo "String operations using \"expr \$string : \" construct"
echo "==================================================="
echo

a=1234zipper5FLIPPER43231

echo "The string being operated upon is \"`expr "$a" : '\(.*\)'`\"."
#     Escaped parentheses grouping operator.            ==  ==

#       ***************************
#+          Escaped parentheses
#+           match a substring
#       ***************************


#  If no escaped parentheses ...
#+ then 'expr' converts the string operand to an integer.

echo "Length of \"$a\" is `expr "$a" : '.*'`."   # Length of string

echo "Number of digits at the beginning of \"$a\" is `expr "$a" : '[0-9]*'`."

# ------------------------------------------------------------------------- #

echo

echo "The digits at the beginning of \"$a\" are `expr "$a" : '\([0-9]*\)'`."
#                                                             ==      ==
echo "The first 7 characters of \"$a\" are `expr "$a" : '\(.......\)'`."
#         =====                                          ==       ==
# Again, escaped parentheses force a substring match.
#
echo "The last 7 characters of \"$a\" are `expr "$a" : '.*\(.......\)'`."
#         ====                  end of string operator  ^^
#  (In fact, means skip over one or more of any characters until specified
#+  substring found.)

echo

exit 0
```

```output bash

String operations using "expr $string : " construct
===================================================

The string being operated upon is "1234zipper5FLIPPER43231".
Length of "1234zipper5FLIPPER43231" is 23.
Number of digits at the beginning of "1234zipper5FLIPPER43231" is 4.

The digits at the beginning of "1234zipper5FLIPPER43231" are 1234.
The first 7 characters of "1234zipper5FLIPPER43231" are 1234zip.
The last 7 characters of "1234zipper5FLIPPER43231" are ER43231.

```

`expr`'s match is anchored at the start; Python's `re.match` (the
pattern's `.match` method) is too. `.group 1` is the parenthesized part, and
the ends of the match are the length.

```tessh
print
print "String operations using regexes instead of \"expr \$string : \""
print "==================================================="
print

a = "1234zipper5FLIPPER43231"

print "The string being operated upon is \"$((re'(.*)'.match $a).group 1)\"."
print "Length of \"$a\" is $((re'.*'.match $a).end)."
print "Number of digits at the beginning of \"$a\" is $((re'[0-9]*'.match $a).end)."

print

print "The digits at the beginning of \"$a\" are $((re'([0-9]*)'.match $a).group 1)."
print "The first 7 characters of \"$a\" are $((re'(.......)'.match $a).group 1)."
print "The last 7 characters of \"$a\" are $((re'.*(.......)'.match $a).group 1)."
# (Or simply $a[:7] and $a[-7:].)

print
exit 0
```

```output

String operations using regexes instead of "expr $string : "
===================================================

The string being operated upon is "1234zipper5FLIPPER43231".
Length of "1234zipper5FLIPPER43231" is 23.
Number of digits at the beginning of "1234zipper5FLIPPER43231" is 4.

The digits at the beginning of "1234zipper5FLIPPER43231" are 1234.
The first 7 characters of "1234zipper5FLIPPER43231" are 1234zip.
The last 7 characters of "1234zipper5FLIPPER43231" are ER43231.

```

Peter Knowles' trim, `LRFDATE=`expr "$LRFDATE" : '[[:space:]]*\(.*\)[[:space:]]*$'``,
is `LRFDATE = $LRFDATE.strip` in tessh (and `sed` or `awk` are better than
`expr` for string work in bash too, as the ABS Guide says).
