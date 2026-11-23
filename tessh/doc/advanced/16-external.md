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

## 16.3 Time / Date Commands
<!-- help: abs-time-date date zdump time touch at batch cal sleep usleep hwclock datetime -->

`date` prints the date and time, in any format with `+FORMAT` (`%j` day
of the year, `%s` seconds since 1970, `%N` nanoseconds), in UTC with
`-u`, and for another time with `-d`/`--date` (`'6 days ago'`,
`'2007-12-25 12:30'`). `TZ=zone` in front picks a time zone, `zdump`
shows a zone's time, and `time` times a command. `touch` sets a file's
times (or makes an empty file), `at` and `batch` run commands later, `cal`
prints a calendar, `sleep` waits (`usleep` in microseconds), and
`hwclock` reads or sets the hardware clock.

The programs run the same in tessh. For calculating with dates, Python's
`datetime` is better than `date` and arithmetic on seconds: a
`datetime` minus another is a `timedelta` with `.days` and `.seconds`,
one plus a `timedelta` is a new date, and a format spec in a string
(`"${d:%d %B}"`) formats it the way `date +…` does.

### Example 16-10. Using `date` (`ex51.sh`)

```bash
#!/bin/bash
# Exercising the 'date' command

echo "The number of days since the year's beginning is `date +%j`."
# Needs a leading '+' to invoke formatting.
# %j gives day of year.

echo "The number of seconds elapsed since 01/01/1970 is `date +%s`."
#  %s yields number of seconds since "UNIX epoch" began,
#+ but how is this useful?

prefix=temp
suffix=$(date +%s)  # The "+%s" option to 'date' is GNU-specific.
filename=$prefix.$suffix
echo "Temporary filename = $filename"
#  It's great for creating "unique and random" temp filenames,
#+ even better than using $$.

# Read the 'date' man page for more formatting options.

exit 0
```

`datetime.datetime.now` is the time as a value; `strftime` takes `date`'s
formats, and `.timestamp` is the seconds since 1970. For a temporary file
name, Python's `tempfile` makes one that's really unique (and creates it
safely), which a time in seconds isn't.

```tessh
import datetime
now = (datetime.datetime.now)

print "The number of days since the year's beginning is ${now:%j}."
# %j gives day of year.

print "The number of seconds elapsed since 01/01/1970 is $(int ($now.timestamp))."

prefix = temp
suffix = int ($now.timestamp)
filename = "$prefix.$suffix"
print "Temporary filename = $filename"
# (tempfile.mkstemp makes a temporary file with a name no other has.)

exit 0
```

```text
(one run)
The number of days since the year's beginning is 327.
The number of seconds elapsed since 01/01/1970 is 1795458756.
Temporary filename = temp.1795458756
```

The other `date` tricks in the text are `datetime` too:

```bash no-run
date +%N | sed -e 's/000$//' -e 's/^0//'   # nanoseconds, as a random number
date +%j                                    # day of the year
TZ=EST date                                 # the time in another zone
SixDaysAgo=$(date --date='6 days ago')
OneYearAgo=$(date --date='1 year ago')
```

```tessh
import datetime
from zoneinfo import ZoneInfo
now = (datetime.datetime.now)
print $now.microsecond                      # (random enough: use random)
print "${now:%j}"                           # day of the year
print (datetime.datetime.now (ZoneInfo EST)) # the time in another zone
TZ=EST date                                  # (or as in bash)
SixDaysAgo = $now - (datetime.timedelta days=6)
OneYearAgo = $now.replace year=($now.year - 1)
print $SixDaysAgo $OneYearAgo
```

```text
(one run)
948829
327
2026-11-23 13:32:36.952673-05:00
Mon Nov 23 01:32:36 PM EST 2026
2026-11-17 10:32:36.948829 2025-11-23 10:32:36.948829
```

### Example 16-11. Date calculations (`date-calc.sh`)

```bash
#!/bin/bash
# date-calc.sh
# Author: Nathan Coulter
# Used in ABS Guide with permission (thanks!).

MPHR=60    # Minutes per hour.
HPD=24     # Hours per day.

diff () {
        printf '%s' $(( $(date -u -d"$TARGET" +%s) -
                        $(date -u -d"$CURRENT" +%s)))
#                       %d = day of month.
}


CURRENT=$(date -u -d '2007-09-01 17:30:24' '+%F %T.%N %Z')
TARGET=$(date -u -d'2007-12-25 12:30:00' '+%F %T.%N %Z')
# %F = full date, %T = %H:%M:%S, %N = nanoseconds, %Z = time zone.

printf '\nIn 2007, %s ' \
       "$(date -d"$CURRENT +
        $(( $(diff) /$MPHR /$MPHR /$HPD / 2 )) days" '+%d %B')" 
#       %B = name of month                ^ halfway
printf 'was halfway between %s ' "$(date -d"$CURRENT" '+%d %B')"
printf 'and %s\n' "$(date -d"$TARGET" '+%d %B')"

printf '\nOn %s at %s, there were\n' \
        $(date -u -d"$CURRENT" +%F) $(date -u -d"$CURRENT" +%T)
DAYS=$(( $(diff) / $MPHR / $MPHR / $HPD ))
CURRENT=$(date -d"$CURRENT +$DAYS days" '+%F %T.%N %Z')
HOURS=$(( $(diff) / $MPHR / $MPHR ))
CURRENT=$(date -d"$CURRENT +$HOURS hours" '+%F %T.%N %Z')
MINUTES=$(( $(diff) / $MPHR ))
CURRENT=$(date -d"$CURRENT +$MINUTES minutes" '+%F %T.%N %Z')
printf '%s days, %s hours, ' "$DAYS" "$HOURS"
printf '%s minutes, and %s seconds ' "$MINUTES" "$(diff)"
printf 'until Christmas Dinner!\n\n'

#  Exercise:
#  --------
#  Rewrite the diff () function to accept passed parameters,
#+ rather than using global variables.
```

Nathan Coulter's script converts each date to seconds with `date +%s`,
subtracts, and steps the current date forward a day, an hour and a minute
at a time with `date -d"… +N days"`. With `datetime` the difference is
one subtraction, a `timedelta`, and halving it is `/ 2`. The exercise
(`diff` with parameters instead of globals) is done too.

```tessh
# date-calc.tsh
# After Nathan Coulter's date-calc.sh.
from datetime import datetime timedelta

def diff current target = $target - $current     # a timedelta

CURRENT = datetime 2007 9 1 17 30 24
TARGET = datetime 2007 12 25 12 30 0

# Halfway, in whole days as the original counts them.
half = $CURRENT + (timedelta days=((diff $CURRENT $TARGET).days // 2))
print "\nIn 2007, ${half:%d %B} was halfway between ${CURRENT:%d %B} and ${TARGET:%d %B}"

print "\nOn ${CURRENT:%F} at ${CURRENT:%T}, there were"
left = diff $CURRENT $TARGET
DAYS = $left.days
[HOURS rest] = divmod $left.seconds 3600
[MINUTES SECONDS] = divmod $rest 60
print "$DAYS days, $HOURS hours, $MINUTES minutes, and $SECONDS seconds until Christmas Dinner!\n"
```

```output

In 2007, 28 October was halfway between 01 September and 25 December

On 2007-09-01 at 17:30:24, there were
114 days, 18 hours, 59 minutes, and 36 seconds until Christmas Dinner!

```

`time` is not a bash-style keyword in tessh: `time cmd` runs the `time`
program, which prints its own report (`0.00user 0.00system 0:00.10elapsed
…`) on stderr. To time Python code, take `time.perf_counter` before and
after.

**Gap:** tessh has no `time` keyword: `time` runs `/usr/bin/time` when it's
installed, so it can't time a pipeline as a whole, a Python call, or a
block, and its report isn't bash's `real/user/sys`.

`touch`, `at`, `batch`, `cal`, `sleep` and `hwclock` are programs and run
the same; `sleep` takes fractions (`sleep 0.2`), which makes `usleep`
unnecessary, and `time.sleep` does it from Python.

## 16.4 Text Processing Commands
<!-- help: abs-text-processing sort tsort uniq expand cut paste join head tail grep egrep fgrep look sed awk wc tr fold fmt col column colrm nl pr iconv groff -->

The filters: `sort` (`-n` numeric, `-r` reversed, `-k` by field, `-m`
merging sorted files), `tsort` (a topological sort of pairs), `uniq` (drops
repeated lines; `-c` counts them), `expand`/`unexpand` (tabs to spaces and
back), `cut` (fields with `-d` and `-f`, characters with `-c`), `paste`
(files side by side), `join` (lines of two files with the same key), `head`
and `tail` (the first or last lines; `tail -f` follows a growing file),
`grep` (lines matching a regex: `-i`, `-w`, `-l`, `-r`, `-n`, `-v`, `-c`,
`-o`, `-q`; `grep -E` is `egrep`, `grep -F` is `fgrep`), `look` (words
starting with a prefix in a sorted list), `sed` and `awk`, `wc` (lines,
words, characters), `tr` (translates or deletes characters), `fold` and
`fmt` (wrap lines), `column` (lines into a table), `colrm` (removes
columns), `nl` (numbers lines), `pr` (paginates), `iconv` and `recode`
(change encodings), and the typesetters `groff`, TeX and friends.

They all run in tessh as in bash, in pipes as filters. What tessh adds is
that the text can become values at either end: `(cmd).lines` is a list,
a Python stage in a pipe gets the text, a list or string can feed a pipe,
and `re'…'`, `str` methods and `collections.Counter` do much of what
`grep`, `cut`, `tr` and `uniq -c` are called for in a script.

```bash
cat > testfile <<'END'
This line occurs only once.
This line occurs twice.
This line occurs twice.
This line occurs three times.
This line occurs three times.
This line occurs three times.
END
uniq -c testfile
sort testfile | uniq -c | sort -nr
```

```output bash
      1 This line occurs only once.
      2 This line occurs twice.
      3 This line occurs three times.
      3 This line occurs three times.
      2 This line occurs twice.
      1 This line occurs only once.
```

```tessh
cat > testfile <<'END'
This line occurs only once.
This line occurs twice.
This line occurs twice.
This line occurs three times.
This line occurs three times.
This line occurs three times.
END
uniq -c testfile
sort testfile | uniq -c | sort -nr
# The counting as a value:
from collections import Counter
print ((Counter (cat testfile).lines).most_common 1)
```

```output
      1 This line occurs only once.
      2 This line occurs twice.
      3 This line occurs three times.
      3 This line occurs three times.
      2 This line occurs twice.
      1 This line occurs only once.
[('This line occurs three times.', 3)]
```

`cut` takes fields out of lines. Looping over its output is the ABS
Guide's way to list the users in `/etc/passwd`; in tessh, `.lines` gives
the list (here of a made-up passwd file), or `str.split` takes the field
without `cut`.

```bash no-run
# List all the users in /etc/passwd.

FILENAME=/etc/passwd

for user in $(cut -d: -f1 $FILENAME)
do
  echo $user
done

# Thanks, Oleg Philon for suggesting this.
```

```tessh
# (A made-up /etc/passwd.)
cat >passwd <<'END'
root:x:0:0:root:/root:/bin/bash
bozo:x:1000:100:Bozo:/home/bozo:/bin/bash
nobody:x:65534:65534:nobody:/:/sbin/nologin
END
FILENAME = ./passwd               # (/etc/passwd in the original)

for user in (cut -d: -f1 $FILENAME).lines
  print $user
# Or without cut:
print [($line.split ":")[0] for line in (cat $FILENAME).lines]
```

```output
root
bozo
nobody
['root' 'bozo' 'nobody']
```

`paste` puts files side by side and `join` joins lines with the same key:

```bash
printf 'alphabet blocks\nbuilding blocks\ncables\n' > items
printf '$1.00/dozen\n$2.50 ea.\n$3.75\n' > prices
paste items prices
printf '100 Shoes\n200 Laces\n300 Socks\n' > 1.data
printf '100 $40.00\n200 $1.00\n300 $2.00\n' > 2.data
join 1.data 2.data
```

```output bash
alphabet blocks	$1.00/dozen
building blocks	$2.50 ea.
cables	$3.75
100 Shoes $40.00
200 Laces $1.00
300 Socks $2.00
```

In Python, `zip` pairs the lines and a dict does the join.

```tessh
printf 'alphabet blocks\nbuilding blocks\ncables\n' > items
printf '$1.00/dozen\n$2.50 ea.\n$3.75\n' > prices
paste items prices
printf '100 Shoes\n200 Laces\n300 Socks\n' > 1.data
printf '100 $40.00\n200 $1.00\n300 $2.00\n' > 2.data
join 1.data 2.data
# The same with values:
for [item price] in (zip (cat items).lines (cat prices).lines)
  print "$item\t$price"
prices = dict [($l.split " " 1) for l in (cat 2.data).lines]
for [key what] in [($l.split " " 1) for l in (cat 1.data).lines]
  print $key $what $prices[$key]
```

```output
alphabet blocks	$1.00/dozen
building blocks	$2.50 ea.
cables	$3.75
100 Shoes $40.00
200 Laces $1.00
300 Socks $2.00
alphabet blocks	$1.00/dozen
building blocks	$2.50 ea.
cables	$3.75
100 Shoes $40.00
200 Laces $1.00
300 Socks $2.00
```

### Example 16-12. Word Frequency Analysis (`wf.sh`)

```bash no-run
#!/bin/bash
# wf.sh: Crude word frequency analysis on a text file.
# This is a more efficient version of the "wf2.sh" script.


# Check for input file on command-line.
ARGS=1
E_BADARGS=85
E_NOFILE=86

if [ $# -ne "$ARGS" ]  # Correct number of arguments passed to script?
then
  echo "Usage: `basename $0` filename"
  exit $E_BADARGS
fi

if [ ! -f "$1" ]       # Check if file exists.
then
  echo "File \"$1\" does not exist."
  exit $E_NOFILE
fi



########################################################
# main ()
sed -e 's/\.//g'  -e 's/\,//g' -e 's/ /\
/g' "$1" | tr 'A-Z' 'a-z' | sort | uniq -c | sort -nr
#                           =========================
#                            Frequency of occurrence

#  Filter out periods and commas, and
#+ change space between words to linefeed,
#+ then shift characters to lowercase, and
#+ finally prefix occurrence count and sort numerically.

#  Arun Giridhar suggests modifying the above to:
#  . . . | sort | uniq -c | sort +1 [-f] | sort +0 -nr
#  This adds a secondary sort key, so instances of
#+ equal occurrence are sorted alphabetically.
#  As he explains it:
#  "This is effectively a radix sort, first on the
#+ least significant column
#+ (word or string, optionally case-insensitive)
#+ and last on the most significant column (frequency)."
#
#  As Frank Wang explains, the above is equivalent to
#+       . . . | sort | uniq -c | sort +0 -nr
#+ and the following also works:
#+       . . . | sort | uniq -c | sort -k1nr -k
########################################################

exit 0

# Exercises:
# ---------
# 1) Add 'sed' commands to filter out other punctuation,
#+   such as semicolons.
# 2) Modify the script to also filter out multiple spaces and
#+   other whitespace.
```

The counting is a `Counter`: the text, without periods and commas, made
lowercase and split into words. Sorting by the count and then the word,
both reversed, gives `sort -nr`'s order, and `${n:7d}` lays the numbers out
as `uniq -c` does. (Arun Giridhar's alphabetical order for ties is `key=(fn
[kv] (- $kv[1], $kv[0]))` without `reverse`.) This runs on the ABS
Guide's `testfile`.

```tessh
from collections import Counter
# (The ABS Guide's testfile.)
printf "%s\n" "This line occurs only once." "This line occurs twice." \
  "This line occurs twice." "This line occurs three times." \
  "This line occurs three times." "This line occurs three times." > testfile

ARGS = 1
E_BADARGS = 85
E_NOFILE = 86

args = $argv[1:] or [testfile]   # (sample argument, when none is given)
if (len $args) != $ARGS          # Correct number of arguments passed to script?
  print "Usage: $(Path $argv[0]).name filename"
  exit $E_BADARGS
file = Path $args[0]
if not ($file.is_file)           # Check if file exists.
  print "File \"$file\" does not exist."
  exit $E_NOFILE

# main
# Filter out periods and commas, shift to lowercase, split into words,
# count them, and sort by frequency of occurrence.
text = re'[.,]'.sub "" ($file.read_text)
counts = Counter (($text.lower).split)
for [word n] in (sorted ($counts.items) key=(fn [kv] ($kv[1], $kv[0])) reverse=True)
  print "${n:7d} $word"

exit 0
```

```output
      6 this
      6 occurs
      6 line
      3 times
      3 three
      2 twice
      1 only
      1 once
```

`head` lists the start of a file, `-c N` its first N bytes:

### Example 16-13. Which files are scripts? (`script-detector.sh`)

```bash no-run
#!/bin/bash
# script-detector.sh: Detects scripts within a directory.

TESTCHARS=2    # Test first 2 characters.
SHABANG='#!'   # Scripts begin with a "sha-bang."

for file in *  # Traverse all the files in current directory.
do
  if [[ `head -c$TESTCHARS "$file"` = "$SHABANG" ]]
  #      head -c2                      #!
  #  The '-c' option to "head" outputs a specified
  #+ number of characters, rather than lines (the default).
  then
    echo "File \"$file\" is a script."
  else
    echo "File \"$file\" is *not* a script."
  fi
done
  
exit 0

#  Exercises:
#  ---------
#  1) Modify this script to take as an optional argument
#+    the directory to scan for scripts
#+    (rather than just the current working directory).
#
#  2) As it stands, this script gives "false positives" for
#+    Perl, awk, and other scripting language scripts.
#     Correct this.
```

`head -c2` works the same; here the first two bytes are read in Python
instead, from a file opened in binary mode, so a binary file doesn't
trouble it. A folder is skipped (`head` on one is an error), and it runs on
a few made-up files.

```tessh
# (Made-up files to try it on.)
printf '#!/bin/bash\necho hi\n' > hello.sh
printf '#!/usr/bin/env tessh\nprint hi\n' > hi.tsh
printf 'Just some notes.\n' > notes.txt
printf '\x7fELF\x02\x01' > program

TESTCHARS = 2     # Test first 2 characters.
SHABANG = b"#!"   # Scripts begin with a "sha-bang."

for file in *     # Traverse all the files in current directory.
  if not ($file.is_file)
    continue
  with (open $file rb) as f
    start = $f.read $TESTCHARS
  if $start == $SHABANG
    print "File \"$file\" is a script."
  else
    print "File \"$file\" is *not* a script."

exit 0
```

```output
File "hello.sh" is a script.
File "hi.tsh" is a script.
File "notes.txt" is *not* a script.
File "program" is *not* a script.
```

### Example 16-14. Generating 10-digit random numbers (`rnd.sh`)

```bash
#!/bin/bash
# rnd.sh: Outputs a 10-digit random number

# Script by Stephane Chazelas.

head -c4 /dev/urandom | od -N4 -tu4 | sed -ne '1s/.* //p'


# =================================================================== #

# Analysis
# --------

# head:
# -c4 option takes first 4 bytes.

# od:
# -N4 option limits output to 4 bytes.
# -tu4 option selects unsigned decimal format for output.

# sed: 
# -n option, in combination with "p" flag to the "s" command,
# outputs only matched lines.



# The author of this script explains the action of 'sed', as follows.

# head -c4 /dev/urandom | od -N4 -tu4 | sed -ne '1s/.* //p'
# ----------------------------------> |

# Assume output up to "sed" --------> |
# is 0000000 1198195154\n

#  sed begins reading characters: 0000000 1198195154\n.
#  Here it finds a newline character,
#+ so it is ready to process the first line (0000000 1198195154).
#  It looks at its <range><action>s. The first and only one is

#   range     action
#   1         s/.* //p

#  The line number is in the range, so it executes the action:
#+ tries to substitute the longest string ending with a space in the line
#  ("0000000 ") with nothing (//), and if it succeeds, prints the result
#  ("p" is a flag to the "s" command here, this is different
#+ from the "p" command).

#  sed is now ready to continue reading its input. (Note that before
#+ continuing, if -n option had not been passed, sed would have printed
#+ the line once again).

#  Now, sed reads the remainder of the characters, and finds the
#+ end of the file.
#  It is now ready to process its 2nd line (which is also numbered '$' as
#+ it's the last one).
#  It sees it is not matched by any <range>, so its job is done.

#  In few word this sed commmand means:
#  "On the first line only, remove any character up to the right-most space,
#+ then print it."

# A better way to do this would have been:
#           sed -e 's/.* //;q'

# Here, two <range><action>s (could have been written
#           sed -e 's/.* //' -e q):

#   range                    action
#   nothing (matches line)   s/.* //
#   nothing (matches line)   q (quit)

#  Here, sed only reads its first line of input.
#  It performs both actions, and prints the line (substituted) before
#+ quitting (because of the "q" action) since the "-n" option is not passed.

# =================================================================== #

# An even simpler altenative to the above one-line script would be:
#           head -c4 /dev/urandom| od -An -tu4

exit
```

Stéphane Chazelas' pipeline runs as is in tessh. The Python way reads the
same four bytes and makes them a number, with no text to cut up.

```tessh
# rnd.tsh: Outputs a 10-digit random number
head -c4 /dev/urandom | od -N4 -tu4 | sed -ne '1s/.* //p'

import os
print (int.from_bytes (os.urandom 4))   # (or random.getrandbits 32)
```

```text
(one run)
3244878020
1079381601
```

`tail` lists the end of a file, and `tail -f` keeps printing what's
appended to it:

### Example 16-15. Using `tail` to monitor the system log (`ex12.sh`)

```bash no-run
#!/bin/bash

filename=sys.log

cat /dev/null > $filename; echo "Creating / cleaning out file."
#  Creates the file if it does not already exist,
#+ and truncates it to zero length if it does.
#  : > filename   and   > filename also work.

tail /var/log/messages > $filename  
# /var/log/messages must have world read permission for this to work.

echo "$filename contains tail end of system log."

exit 0
```

It reads a made-up `messages` here (there's no `/var/log/messages` on
systems that log to the journal; `journalctl -n 10` is the modern `tail`
of the log).

```tessh
# (A made-up /var/log/messages.)
for i in (range 1 13)
  print "Nov 23 10:${i:02d} host kernel: event $i" >>messages

filename = sys.log

cat /dev/null > $filename ; print "Creating / cleaning out file."
#  Creates the file if it does not already exist,
#+ and truncates it to zero length if it does.

tail messages > $filename      # (/var/log/messages in the original)

print "$filename contains tail end of system log."
head -n 2 $filename            # (To see it.)
exit 0
```

```output
Creating / cleaning out file.
sys.log contains tail end of system log.
Nov 23 10:03 host kernel: event 3
Nov 23 10:04 host kernel: event 4
```

`head -n 8 file | tail -n 1` is the 8th line, `(cat file).lines[7]` in
tessh. `grep` searches files for a regex. `-q` makes it quiet, for its exit
status alone, and a tessh `if` takes a command as its condition, so the
ABS Guide's `grep -q` test needs no `$?`:

```bash
SUCCESS=0                      # if grep lookup succeeds
word=Linux
filename=data.file
printf 'All about Linux.\n' > $filename    # (a made-up data.file)

grep -q "$word" "$filename"    #  The "-q" option
                               #+ causes nothing to echo to stdout.
if [ $? -eq $SUCCESS ]
# if grep -q "$word" "$filename"   can replace lines 5 - 7.
then
  echo "$word found in $filename"
else
  echo "$word not found in $filename"
fi
```

```output bash
Linux found in data.file
```

```tessh
word = Linux
filename = data.file
printf 'All about Linux.\n' > $filename    # (a made-up data.file)

if grep -q $word $filename
  print "$word found in $filename"
else
  print "$word not found in $filename"
# Or in Python:
print ($word in ((Path $filename).read_text))
```

```output
Linux found in data.file
True
```

### Example 16-16. Printing out the *From* lines in stored e-mail messages (`from.sh`)

```bash no-run
#!/bin/bash
# from.sh

#  Emulates the useful 'from' utility in Solaris, BSD, etc.
#  Echoes the "From" header line in all messages
#+ in your e-mail directory.


MAILDIR=~/mail/*               #  No quoting of variable. Why?
# Maybe check if-exists $MAILDIR:   if [ -d $MAILDIR ] . . .
GREP_OPTS="-H -A 5 --color"    #  Show file, plus extra context lines
                               #+ and display "From" in color.
TARGETSTR="^From"              # "From" at beginning of line.

for file in $MAILDIR           #  No quoting of variable.
do
  grep $GREP_OPTS "$TARGETSTR" "$file"
  #    ^^^^^^^^^^              #  Again, do not quote this variable.
  echo
done

exit $?

#  You might wish to pipe the output of this script to 'more'
#+ or redirect it to a file . . .
```

The ABS Guide leaves `$MAILDIR` and `$GREP_OPTS` unquoted on purpose: one
must glob, the other must split into words. In tessh they're values that
say so: `./mail/*` written bare is a Glob, read again each time it's
looped over, and the options are a list, one argument per item. It runs on
a made-up `mail` folder (`~/mail/*` in the original).

```tessh
# (A made-up mail folder.)
mkdir mail
cat >mail/inbox <<'END'
From bozo@example.com Mon Nov 23 10:00:00 2026
Subject: lunch?
To: you

Tacos at noon.
END
cat >mail/saved <<'END'
From ann@example.org Sun Nov 22 09:30:00 2026
Subject: the report
To: you

Attached.
END

MAILDIR = ./mail/*             # A Glob (~/mail/* in the original).
GREP_OPTS = [-H -A 2]          # Show file, plus extra context lines.
TARGETSTR = "^From"            # "From" at beginning of line.

for file in $MAILDIR
  grep *$GREP_OPTS $TARGETSTR $file
  print

exit $?
```

```output
./mail/inbox:From bozo@example.com Mon Nov 23 10:00:00 2026
./mail/inbox-Subject: lunch?
./mail/inbox-To: you

./mail/saved:From ann@example.org Sun Nov 22 09:30:00 2026
./mail/saved-Subject: the report
./mail/saved-To: you

```

### Example 16-17. Emulating `grep` in a script (`grp.sh`)

```bash no-run
#!/bin/bash
# grp.sh: Rudimentary reimplementation of grep.

E_BADARGS=85

if [ -z "$1" ]    # Check for argument to script.
then
  echo "Usage: `basename $0` pattern"
  exit $E_BADARGS
fi  

echo

for file in *     # Traverse all files in $PWD.
do
  output=$(sed -n /"$1"/p $file)  # Command substitution.

  if [ ! -z "$output" ]           # What happens if "$output" is not quoted?
  then
    echo -n "$file: "
    echo "$output"
  fi              #  sed -ne "/$1/s|^|${file}: |p"  is equivalent to above.

  echo
done  

echo

exit 0

# Exercises:
# ---------
# 1) Add newlines to output, if more than one match in any given file.
# 2) Add features.
```

A grep in Python: the pattern compiled with `re.compile`, each file's lines
kept when it matches. bash's `sed -n /"$1"/p` breaks on a pattern with a `/`
in it; `re.compile` doesn't care.

```tessh
import re
# (Made-up files to search.)
printf 'This is a file containing information about Linux.\nNothing here.\n' > osinfo.txt
printf 'The Linux operating system is steadily gaining in popularity.\n' > misc.txt
printf 'No penguins.\n' > other.txt

E_BADARGS = 85

args = $argv[1:] or [Linux]      # (sample argument, when none is given)
if not $args or not $args[0]     # Check for argument to script.
  print "Usage: $(Path $argv[0]).name pattern"
  exit $E_BADARGS
pattern = re.compile $args[0]

print

for file in *                    # Traverse all files in the folder.
  output = [$l for l in (($file.read_text).splitlines) if ($pattern.search $l)]
  if $output
    print "$file: " end=""
    print ("\n".join $output)
  print

print
exit 0
```

```output

misc.txt: The Linux operating system is steadily gaining in popularity.

osinfo.txt: This is a file containing information about Linux.



```

To find the lines with *both* of two patterns, pipe one `grep` into the
other, as in bash (`grep file tstfile | grep text`), or test both with
`and` in Python.
