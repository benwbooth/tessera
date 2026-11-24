# 11. Loops and Branches
<!-- help: abs-loops abs-11 loops for while until case select break continue -->

> What needs this iteration, woman? --Shakespeare, *Othello*

Bash has `for`, `while` and `until` loops, `break` and `continue`, and
`case` and `select` for branching. tessh has Python's `for` and `while`
(with `else`), `break` and `continue`, and `match` for `case`. The big
difference is the list a `for` goes over: a real list (or any iterable:
a range, a file's lines, a Glob's names) rather than words split out of a
string, so nothing needs quoting to stay whole.

## 11.1 Loops
<!-- help: abs-loops1 for-loop while-loop until-loop range iteration -->

### for loops

`for arg in [list]` runs its block once for each item. In tessh the list
is a list: `[a b c]`, a variable holding one, `(range 1 11)`, a Glob, or
anything else Python can iterate over.

### Example 11-1. Simple for loops (`ex22.sh`)

```bash
#!/bin/bash
# Listing the planets.

for planet in Mercury Venus Earth Mars Jupiter Saturn Uranus Neptune Pluto
do
  echo $planet  # Each planet on a separate line.
done

echo; echo

for planet in "Mercury Venus Earth Mars Jupiter Saturn Uranus Neptune Pluto"
    # All planets on same line.
    # Entire 'list' enclosed in quotes creates a single variable.
    # Why? Whitespace incorporated into the variable.
do
  echo $planet
done

echo; echo "Whoops! Pluto is no longer a planet!"

exit 0
```

```tessh
#!/usr/bin/env tessh
# Listing the planets.

for planet in [Mercury Venus Earth Mars Jupiter Saturn Uranus Neptune Pluto]
  print $planet  # Each planet on a separate line.

print ; print

for planet in ["Mercury Venus Earth Mars Jupiter Saturn Uranus Neptune Pluto"]
  # All planets on same line: one quoted string is one item.
  print $planet

print ; print "Whoops! Pluto is no longer a planet!"
```

```output
Mercury
Venus
Earth
Mars
Jupiter
Saturn
Uranus
Neptune
Pluto


Mercury Venus Earth Mars Jupiter Saturn Uranus Neptune Pluto

Whoops! Pluto is no longer a planet!
```

Each item can hold several values. Bash splits a string with `set --`; in
tessh the items are pairs, and `for [name distance] in …` unpacks each.

### Example 11-2. for loop with two parameters in each [list] element (`ex22a.sh`)

```bash
#!/bin/bash
# Planets revisited.

# Associate the name of each planet with its distance from the sun.

for planet in "Mercury 36" "Venus 67" "Earth 93"  "Mars 142" "Jupiter 483"
do
  set -- $planet  #  Parses variable "planet"
                  #+ and sets positional parameters.
  #  The "--" prevents nasty surprises if $planet is null or
  #+ begins with a dash.

  #  May need to save original positional parameters,
  #+ since they get overwritten.
  #  One way of doing this is to use an array,
  #         original_params=("$@")

  echo "$1		$2,000,000 miles from the sun"
  #-------two  tabs---concatenate zeroes onto parameter $2
done

# (Thanks, S.C., for additional clarification.)

exit 0
```

```tessh
#!/usr/bin/env tessh
# Planets revisited.

# Associate the name of each planet with its distance from the sun.

for [planet distance] in [[Mercury 36] [Venus 67] [Earth 93] [Mars 142] [Jupiter 483]]
  # (From "Mercury 36" it would be: [planet distance] = $item.split)
  print "$planet\t\t$distance,000,000 miles from the sun"
  #-------two tabs---------concatenate zeroes onto the distance
```

```output
Mercury		36,000,000 miles from the sun
Venus		67,000,000 miles from the sun
Earth		93,000,000 miles from the sun
Mars		142,000,000 miles from the sun
Jupiter		483,000,000 miles from the sun
```

A variable may hold the list.

### Example 11-3. Fileinfo: operating on a file list contained in a variable (`file-info.sh`)

```bash no-run
#!/bin/bash
# fileinfo.sh

FILES="/usr/sbin/accept
/usr/sbin/pwck
/usr/sbin/chroot
/usr/bin/fakefile
/sbin/badblocks
/sbin/ypbind"     # List of files you are curious about.
                  # Threw in a dummy file, /usr/bin/fakefile.

echo

for file in $FILES
do

  if [ ! -e "$file" ]       # Check if file exists.
  then
    echo "$file does not exist."; echo
    continue                # On to next.
   fi

  ls -l $file | awk '{ print $8 "         file size: " $5 }'  # Print 2 fields.
  whatis `basename $file`   # File info.
  # Note that the whatis database needs to have been set up for this to work.
  # To do this, as root run /usr/bin/makewhatis.
  echo
done  

exit 0
```

A Path knows whether it exists and its size (`stat`), so there's no
`ls | awk` to pick fields from `ls`'s output. The files here are made up,
and `file` (what's in it) stands in for `whatis` (what a program is for),
which depends on what's installed.

```tessh
#!/usr/bin/env tessh
# (Made-up files to be curious about.)
mkdir -p sbin ; printf "#!/bin/sh\necho hi\n" >sbin/accept ; echo text >sbin/pwck

FILES = [./sbin/accept ./sbin/pwck ./sbin/fakefile]   # List of files you are curious about.
                                                     # Threw in a dummy file, fakefile.
print

for file in $FILES
  if not ($file.exists)       # Check if file exists.
    print "$file does not exist." ; print
    continue                  # On to next.

  print "$file         file size: $(($file.stat).st_size)"
  \file -b $file              # File info. (\file: the program, not the variable.)
  print
```

```output

./sbin/accept         file size: 18
POSIX shell script, ASCII text executable

./sbin/pwck         file size: 5
ASCII text

./sbin/fakefile does not exist.

```

### Example 11-4. Operating on a parameterized file list (`file-info01.sh`)

```bash no-run
#!/bin/bash

filename="*txt"

for file in $filename
do
 echo "Contents of $file"
 echo "---"
 cat "$file"
 echo
done
```

Unquoted, `*txt` is a Glob: a value that stands for the matching files,
read again each time it's looped over. (Quoted, `'*txt'` would be text.)

```tessh
#!/usr/bin/env tessh
# (Made-up files.)
echo "first file" >a.txt ; echo "second file" >b.txt

filename = *txt

for file in $filename
  print "Contents of $file"
  print "---"
  cat $file
  print
```

```output
Contents of a.txt
---
first file

Contents of b.txt
---
second file

```

### Example 11-5. Operating on files with a for loop (`list-glob.sh`)

```bash no-run
#!/bin/bash
# list-glob.sh: Generating [list] in a for-loop, using "globbing" ...
# Globbing = filename expansion.

echo

for file in *
#           ^  Bash performs filename expansion
#+             on expressions that globbing recognizes.
do
  ls -l "$file"  # Lists all files in $PWD (current directory).
  #  Recall that the wild card character "*" matches every filename,
  #+ however, in "globbing," it doesn't match dot-files.

  #  If the pattern matches no file, it is expanded to itself.
  #  To prevent this, set the nullglob option
  #+   (shopt -s nullglob).
  #  Thanks, S.C.
done

echo; echo

for file in [jx]*
do
  rm -f $file    # Removes only files beginning with "j" or "x" in $PWD.
  echo "Removed file \"$file\"".
done

echo

exit 0
```

In a `for`, a Glob gives the files it matches and nothing when there are
none, which is bash's `nullglob`: the loop doesn't run with the pattern
itself. `ls -s` shows sizes here, where `ls -l` would show dates and an
owner that change from run to run.

```tessh
#!/usr/bin/env tessh
# (Made-up files.)
touch apple jam xylophone .hidden ; echo data >kiwi

print

for file in *
  ls -s $file      # Lists all files in the current folder (not dot-files).

print ; print

for file in [jx]*
  rm -f $file      # Removes only files beginning with "j" or "x".
  print "Removed file \"$file\"."

for file in *.nothing
  print "never printed: no match, no loop"

print
```

```output

0 apple
0 jam
4 kiwi
0 xylophone


Removed file "jam".
Removed file "xylophone".

```

Bash's `for a` with no `in [list]` goes over the script's arguments. In
tessh that's `for a in $argv[1:]`.

### Example 11-6. Missing in [list] in a for loop (`ex23.sh`)

```bash
#!/bin/bash

#  Invoke this script both with and without arguments,
#+ and see what happens.

for a
do
 echo -n "$a "
done

#  The 'in list' missing, therefore the loop operates on '$@'
#+ (command-line argument list, including whitespace).

echo

exit 0
```

```output bash

```

With the loop in a function, it's tried both ways.

```tessh
#!/usr/bin/env tessh
def show_args args
  for a in $args
    print "$a " end=""
  print

show_args $argv[1:]                 # The script's arguments (none here).
show_args ["one" "two three" four]  # And some, one with a space in it.
```

```output

one two three four 
```

The list can come from a command's output.

### Example 11-7. Generating the [list] in a for loop with command substitution (`for-loopcmd.sh`)

```bash
#!/bin/bash
#  for-loopcmd.sh: for-loop with [list]
#+ generated by command substitution.

NUMBERS="9 7 3 8 37.53"

for number in `echo $NUMBERS`  # for number in 9 7 3 8 37.53
do
  echo -n "$number "
done

echo 
exit 0
```

A command's output is one string; `.split` makes the list, where bash
splits it without being asked.

```tessh
#!/usr/bin/env tessh
NUMBERS = "9 7 3 8 37.53"

for number in ((echo $NUMBERS).split)   # (or just $NUMBERS.split)
  print "$number " end=""

print
```

```output
9 7 3 8 37.53 
```

### Example 11-8. A grep replacement for binary files (`bin-grep.sh`)

```bash no-run
#!/bin/bash
# bin-grep.sh: Locates matching strings in a binary file.

# A "grep" replacement for binary files.
# Similar effect to "grep -a"

E_BADARGS=65
E_NOFILE=66

if [ $# -ne 2 ]
then
  echo "Usage: `basename $0` search_string filename"
  exit $E_BADARGS
fi

if [ ! -f "$2" ]
then
  echo "File \"$2\" does not exist."
  exit $E_NOFILE
fi  


IFS=$'\012'       # Per suggestion of Anton Filippov.
                  # was:  IFS="\n"
for word in $( strings "$2" | grep "$1" )
# The "strings" command lists strings in binary files.
# Output then piped to "grep", which tests for desired string.
do
  echo $word
done

# As S.C. points out, lines 23 - 30 could be replaced with the simpler
#    strings "$2" | grep "$1" | tr -s "$IFS" '[\n*]'


#  Try something like  "./bin-grep.sh mem /bin/ls"
#+ to exercise this script.

exit 0
```

Bash sets `$IFS` to a newline to get one word per line; `.lines` says so
directly.

```tessh
#!/usr/bin/env tessh
# (A made-up binary file.)
printf '\000\001memory map\000\002\003junk\000\377memo pad\000' >data.bin

E_BADARGS = 65
E_NOFILE = 66

args = $argv[1:] or [mem data.bin]   # (sample arguments, when none are given)

if (len $args) != 2
  print "Usage: $(Path $argv[0]).name search_string filename"
  exit $E_BADARGS

[search filename] = $args
if not ((Path $filename).is_file)
  print "File \"$filename\" does not exist."
  exit $E_NOFILE

for word in (strings $filename | grep $search).lines
  # "strings" lists the strings in a binary file; grep picks some.
  print $word
```

```output
memory map
memo pad
```

### Example 11-9. Listing all users on the system (`userlist.sh`)

```bash no-run
#!/bin/bash
# userlist.sh

PASSWORD_FILE=/etc/passwd
n=1           # User number

for name in $(awk 'BEGIN{FS=":"}{print $1}' < "$PASSWORD_FILE" )
# Field separator = :    ^^^^^^
# Print first field              ^^^^^^^^
# Get input from password file  /etc/passwd  ^^^^^^^^^^^^^^^^^
do
  echo "USER #$n = $name"
  let "n += 1"
done  


# USER #1 = root
# USER #2 = bin
# USER #3 = daemon
# ...
# USER #33 = bozo

exit $?

#  Discussion:
#  ----------
#  How is it that an ordinary user, or a script run by same,
#+ can read /etc/passwd? (Hint: Check the /etc/passwd file permissions.)
#  Is this a security hole? Why or why not?
```

The same `awk`, on a made-up password file, with `enumerate` counting.
(On the real system, Python's `pwd.getpwall` lists the users without
reading the file yourself.)

```tessh
#!/usr/bin/env tessh
# (A made-up /etc/passwd.)
printf "%s\n" "root:x:0:0:root:/root:/bin/bash" "bin:x:1:1:bin:/bin:/sbin/nologin" "bozo:x:1000:100:Bozo:/home/bozo:/bin/bash" >passwd

PASSWORD_FILE = ./passwd   # (/etc/passwd in the original)

names = awk 'BEGIN{FS=":"}{print $1}' <$PASSWORD_FILE
for [n name] in (enumerate $names.lines 1)
  print "USER #$n = $name"
```

```output
USER #1 = root
USER #2 = bin
USER #3 = bozo
```

### Example 11-10. Checking all the binaries in a directory for authorship (`findstring.sh`)

```bash no-run
#!/bin/bash
# findstring.sh:
# Find a particular string in the binaries in a specified directory.

directory=/usr/bin/
fstring="Free Software Foundation"  # See which files come from the FSF.

for file in $( find $directory -type f -name '*' | sort )
do
  strings -f $file | grep "$fstring" | sed -e "s%$directory%%"
  #  In the "sed" expression,
  #+ it is necessary to substitute for the normal "/" delimiter
  #+ because "/" happens to be one of the characters filtered out.
  #  Failure to do so gives an error message. (Try it.)
done  

exit $?

#  Exercise (easy):
#  ---------------
#  Convert this script to take command-line parameters
#+ for $directory and $fstring.
```

`find … | sort` gives the list; the rest is the same pipeline. Here it
searches a made-up folder rather than all of `/usr/bin`, and takes the
folder and the string as arguments (the exercise).

```tessh
#!/usr/bin/env tessh
# (A made-up bin folder.)
mkdir -p bin
printf '\177ELF\000Copyright Free Software Foundation, Inc.\000' >bin/cat
printf '\177ELF\000Copyright Bozo Software\000' >bin/bozo-tool

args = $argv[1:] or ["bin/" "Free Software Foundation"]   # (sample arguments)
[directory fstring] = $args

for file in (find $directory -type f | sort).lines
  strings -f $file | grep $fstring | sed -e "s%$directory%%"
```

```output
cat: Copyright Free Software Foundation, Inc.
```

The list can come from a function, too:

```bash
generate_list ()
{
  echo "one two three"
}

for word in $(generate_list)  # Let "word" grab output of function.
do
  echo "$word"
done

# one
# two
# three
```

```tessh
def generate_list = [one two three]   # A function returns the list itself.

for word in (generate_list)
  print $word
```

```output
one
two
three
```

A `for` loop's output can be piped into a command, or redirected to a
file. tessh can redirect a function's output (`f >file`), but not yet pipe
it (`f | sort`), so the next two collect the names and sort them in Python.

**Gap:** a loop (or a function) can't be a pipeline stage, as bash's
`for … done | sort` is.

### Example 11-11. Listing the symbolic links in a directory (`symlinks.sh`)

```bash no-run
#!/bin/bash
# symlinks.sh: Lists symbolic links in a directory.


directory=${1-`pwd`}
#  Defaults to current working directory,
#+ if not otherwise specified.
#  Equivalent to code block below.
# ----------------------------------------------------------
# ARGS=1                 # Expect one command-line argument.
#
# if [ $# -ne "$ARGS" ]  # If not 1 arg...
# then
#   directory=`pwd`      # current working directory
# else
#   directory=$1
# fi
# ----------------------------------------------------------

echo "symbolic links in directory \"$directory\""

for file in "$( find $directory -type l )"   # -type l = symbolic links
do
  echo "$file"
done | sort                                  # Otherwise file list is unsorted.
#  Strictly speaking, a loop isn't really necessary here,
#+ since the output of the "find" command is expanded into a single word.
#  However, it's easy to understand and illustrative this way.

#  As Dominik 'Aeneas' Schnitzer points out,
#+ failing to quote  $( find $directory -type l )
#+ will choke on filenames with embedded whitespace.
#  containing whitespace. 

exit 0


# --------------------------------------------------------
# Jean Helou proposes the following alternative:

echo "symbolic links in directory \"$directory\""
# Backup of the current IFS. One can never be too cautious.
OLDIFS=$IFS
IFS=:

for file in $(find $directory -type l -printf "%p$IFS")
do     #                              ^^^^^^^^^^^^^^^^
       echo "$file"
       done|sort

# And, James "Mike" Conley suggests modifying Helou's code thusly:

OLDIFS=$IFS
IFS='' # Null IFS means no word breaks
for file in $( find $directory -type l )
do
  echo $file
  done | sort

#  This works in the "pathological" case of a directory name having
#+ an embedded colon.
#  "This also fixes the pathological case of the directory name having
#+  a colon (or space in earlier example) as well."
```

`rglob` walks the folder as `find` does, and `is_symlink` is `-type l`. A
Path is never split, so names with spaces or colons need no `$IFS` tricks.

```tessh
#!/usr/bin/env tessh
# (A made-up folder with links in it, one with a space in its name.)
mkdir -p stuff/sub ; touch stuff/file
ln -s file stuff/link ; ln -s ../file "stuff/sub/other link" ; ln -s /nowhere stuff/dangling

args = $argv[1:] or [stuff]          # (sample argument; (pwd) in the original)
directory = Path $args[0]

print "symbolic links in directory \"$directory\""

for file in (sorted [$f for f in ($directory.rglob "*") if ($f.is_symlink)])
  print $file
```

```output
symbolic links in directory "stuff"
stuff/dangling
stuff/link
stuff/sub/other link
```

### Example 11-12. Symbolic links in a directory, saved to a file (`symlinks2.sh`)

```bash no-run
#!/bin/bash
# symlinks.sh: Lists symbolic links in a directory.

OUTFILE=symlinks.list                         # save-file

directory=${1-`pwd`}
#  Defaults to current working directory,
#+ if not otherwise specified.


echo "symbolic links in directory \"$directory\"" > "$OUTFILE"
echo "---------------------------" >> "$OUTFILE"

for file in "$( find $directory -type l )"    # -type l = symbolic links
do
  echo "$file"
done | sort >> "$OUTFILE"                     # stdout of loop
#           ^^^^^^^^^^^^^                       redirected to save file.

# echo "Output file = $OUTFILE"

exit $?
```

Redirecting a function call sends what it prints to the file.

```tessh
#!/usr/bin/env tessh
mkdir -p stuff ; touch stuff/file ; ln -s file stuff/link ; ln -s /nowhere stuff/dangling

OUTFILE = symlinks.list            # save-file

args = $argv[1:] or [stuff]        # (sample argument)
directory = Path $args[0]

def list_links
  for file in (sorted [$f for f in ($directory.rglob "*") if ($f.is_symlink)])
    print $file

print "symbolic links in directory \"$directory\"" >$OUTFILE
print "---------------------------" >>$OUTFILE
list_links >>$OUTFILE              # stdout of the loop, to the save file.

cat $OUTFILE                       # (To see it.)
```

```output
symbolic links in directory "stuff"
---------------------------
stuff/dangling
stuff/link
```

Bash also has a C-style `for ((a=1; a <= LIMIT; a++))`. Python's is
`for a in (range 1 ($LIMIT + 1))`; two variables stepping together are
`zip`.

### Example 11-13. A C-style for loop (`for-loopc.sh`)

```bash
#!/bin/bash
# Multiple ways to count up to 10.

echo

# Standard syntax.
for a in 1 2 3 4 5 6 7 8 9 10
do
  echo -n "$a "
done  

echo; echo

# +==========================================+

# Using "seq" ...
for a in `seq 10`
do
  echo -n "$a "
done  

echo; echo

# +==========================================+

# Using brace expansion ...
# Bash, version 3+.
for a in {1..10}
do
  echo -n "$a "
done  

echo; echo

# +==========================================+

# Now, let's do the same, using C-like syntax.

LIMIT=10

for ((a=1; a <= LIMIT ; a++))  # Double parentheses, and naked "LIMIT"
do
  echo -n "$a "
done                           # A construct borrowed from ksh93.

echo; echo

# +=========================================================================+

# Let's use the C "comma operator" to increment two variables simultaneously.

for ((a=1, b=1; a <= LIMIT ; a++, b++))
do  # The comma concatenates operations.
  echo -n "$a-$b "
done

echo; echo

exit 0
```

```tessh
#!/usr/bin/env tessh
# Multiple ways to count up to 10.

print

# A list.
for a in [1 2 3 4 5 6 7 8 9 10]
  print "$a " end=""

print ; print

# Using "seq" ...
for a in (seq 10).lines
  print "$a " end=""

print ; print

# Using range (bash's {1..10}).
for a in (range 1 11)
  print "$a " end=""

print ; print

# What C's for (a=1; a <= LIMIT; a++) does:
LIMIT = 10

for a in (range 1 ($LIMIT + 1))
  print "$a " end=""

print ; print

# Two variables stepping together.
for [a b] in (zip (range 1 ($LIMIT + 1)) (range 1 ($LIMIT + 1)))
  print "$a-$b " end=""

print ; print
```

```output

1 2 3 4 5 6 7 8 9 10 

1 2 3 4 5 6 7 8 9 10 

1 2 3 4 5 6 7 8 9 10 

1 2 3 4 5 6 7 8 9 10 

1-1 2-2 3-3 4-4 5-5 6-6 7-7 8-8 9-9 10-10 

```

### Example 11-14. Using efax in batch mode (`ex24.sh`)

```bash no-run
#!/bin/bash
# Faxing (must have 'efax' package installed).

EXPECTED_ARGS=2
E_BADARGS=85
MODEM_PORT="/dev/ttyS2"   # May be different on your machine.
#                ^^^^^      PCMCIA modem card default port.

if [ $# -ne $EXPECTED_ARGS ]
# Check for proper number of command-line args.
then
   echo "Usage: `basename $0` phone# text-file"
   exit $E_BADARGS
fi


if [ ! -f "$2" ]
then
  echo "File $2 is not a text file."
  #     File is not a regular file, or does not exist.
  exit $E_BADARGS
fi
  

fax make $2              #  Create fax-formatted files from text files.

for file in $(ls $2.0*)  #  Concatenate the converted files.
                         #  Uses wild card (filename "globbing")
			 #+ in variable list.
do
  fil="$fil $file"
done  

efax -d "$MODEM_PORT"  -t "T$1" $fil   # Finally, do the work.
# Trying adding  -o1  if above line fails.


#  As S.C. points out, the for-loop can be eliminated with
#     efax -d /dev/ttyS2 -o1 -t "T$1" $2.0*
#+ but it's not quite as instructive [grin].

exit $?   # Also, efax sends diagnostic messages to stdout.
```

The converted files are a Glob; given to `efax` it becomes their names,
so the loop that strings them together isn't needed. It needs `efax` and a
modem, so it doesn't run here.

```tessh no-run
#!/usr/bin/env tessh
EXPECTED_ARGS = 2
E_BADARGS = 85
MODEM_PORT = /dev/ttyS2   # May be different on your machine.

if (len $argv) - 1 != $EXPECTED_ARGS
  print "Usage: $(Path $argv[0]).name phone# text-file"
  exit $E_BADARGS

[phone textfile] = $argv[1:]
if not ((Path $textfile).is_file)
  print "File $textfile is not a text file."
  exit $E_BADARGS

fax make $textfile        # Create fax-formatted files from text files.

fil = sorted ((Path ".").glob "$textfile.0*")   # The converted files.

efax -d $MODEM_PORT -t "T$phone" $fil   # Finally, do the work.
```

### while

`while` tests its condition at the top of the loop and goes on while it's
true. In tessh the condition is any value (a comparison, a call); a
command's exit status works too, as in `if`.

### Example 11-15. Simple while loop (`ex25.sh`)

```bash
#!/bin/bash

var0=0
LIMIT=10

while [ "$var0" -lt "$LIMIT" ]
#      ^                    ^
# Spaces, because these are "test-brackets" . . .
do
  echo -n "$var0 "        # -n suppresses newline.
  #             ^           Space, to separate printed out numbers.

  var0=`expr $var0 + 1`   # var0=$(($var0+1))  also works.
                          # var0=$((var0 + 1)) also works.
                          # let "var0 += 1"    also works.
done                      # Various other methods also work.

echo

exit 0
```

```tessh
#!/usr/bin/env tessh
var0 = 0
LIMIT = 10

while $var0 < $LIMIT
  print "$var0 " end=""   # end="" leaves out the newline.
  var0 += 1

print
```

```output
0 1 2 3 4 5 6 7 8 9 
```

### Example 11-16. Another while loop (`ex26.sh`)

```bash no-run
#!/bin/bash

echo
                               # Equivalent to:
while [ "$var1" != "end" ]     # while test "$var1" != "end"
do
  echo "Input variable #1 (end to exit) "
  read var1                    # Not 'read $var1' (why?).
  echo "variable #1 = $var1"   # Need quotes because of "#" . . .
  # If input is 'end', echoes it here.
  # Does not test for termination condition until top of loop.
  echo
done  

exit 0
```

The answers are canned here (no one is typing), and `input` reads them.
`var1` needs a value before the first test, where bash's unset variable
is just empty.

```tessh
#!/usr/bin/env tessh
import io sys
$sys.stdin = io.StringIO "hello\nworld\nend\n"   # (What someone types.)

print
var1 = ""
while $var1 != end
  print "Input variable #1 (end to exit) "
  var1 = (input)
  print "variable #1 = $var1"
  # If input is 'end', prints it here.
  # Does not test for termination condition until top of loop.
  print
```

```output

Input variable #1 (end to exit) 
variable #1 = hello

Input variable #1 (end to exit) 
variable #1 = world

Input variable #1 (end to exit) 
variable #1 = end

```

A bash `while` can have several commands before `do`; the last one's
status decides. In tessh the condition is one expression, so the
commands go in a function whose value is the test.

### Example 11-17. while loop with multiple conditions (`ex26a.sh`)

```bash no-run
#!/bin/bash

var1=unset
previous=$var1

while echo "previous-variable = $previous"
      echo
      previous=$var1
      [ "$var1" != end ] # Keeps track of what $var1 was previously.
      # Four conditions on *while*, but only the final one controls loop.
      # The *last* exit status is the one that counts.
do
echo "Input variable #1 (end to exit) "
  read var1
  echo "variable #1 = $var1"
done  

# Try to figure out how this all works.
# It's a wee bit tricky.

exit 0
```

```tessh
#!/usr/bin/env tessh
import io sys
$sys.stdin = io.StringIO "first\nsecond\nend\n"   # (What someone types.)

var1 = unset
previous = $var1

def keep_going
  # Three things to do, then the test that controls the loop.
  global previous
  print "previous-variable = $previous"
  print
  previous = $var1
  $var1 != end      # Keeps track of what var1 was previously.

while (keep_going)
  print "Input variable #1 (end to exit) "
  var1 = (input)
  print "variable #1 = $var1"
```

```output
previous-variable = unset

Input variable #1 (end to exit) 
variable #1 = first
previous-variable = unset

Input variable #1 (end to exit) 
variable #1 = second
previous-variable = first

Input variable #1 (end to exit) 
variable #1 = end
previous-variable = second

```

### Example 11-18. C-style syntax in a while loop (`wh-loopc.sh`)

```bash
#!/bin/bash
# wh-loopc.sh: Count to 10 in a "while" loop.

LIMIT=10                 # 10 iterations.
a=1

while [ "$a" -le $LIMIT ]
do
  echo -n "$a "
  let "a+=1"
done                     # No surprises, so far.

echo; echo

# +=================================================================+

# Now, we'll repeat with C-like syntax.

((a = 1))      # a=1
# Double parentheses permit space when setting a variable, as in C.

while (( a <= LIMIT ))   #  Double parentheses,
do                       #+ and no "$" preceding variables.
  echo -n "$a "
  ((a += 1))             # let "a+=1"
  # Yes, indeed.
  # Double parentheses permit incrementing a variable with C-like syntax.
done

echo

# C and Java programmers can feel right at home in Bash.

exit 0
```

tessh's arithmetic is always the C-like kind, so the two halves are the
same loop.

```tessh
#!/usr/bin/env tessh
LIMIT = 10                 # 10 iterations.
a = 1

while $a <= $LIMIT
  print "$a " end=""
  a += 1

print ; print

a = 1
while $a <= $LIMIT         # The same: no other syntax to switch to.
  print "$a " end=""
  a += 1

print
```

```output
1 2 3 4 5 6 7 8 9 10 

1 2 3 4 5 6 7 8 9 10 
```

A `while` can call a function in its test:

```bash
t=0

condition ()
{
  ((t++))

  if [ $t -lt 5 ]
  then
    return 0  # true
  else
    return 1  # false
  fi
}

while condition
#     ^^^^^^^^^
#     Function call -- four loop iterations.
do
  echo "Still going: t = $t"
done

# Still going: t = 1
# Still going: t = 2
# Still going: t = 3
# Still going: t = 4
```

```tessh
t = 0

def condition
  global t
  t += 1
  $t < 5          # True or False, rather than an exit status.

while (condition)
  print "Still going: t = $t"
```

```output
Still going: t = 1
Still going: t = 2
Still going: t = 3
Still going: t = 4
```

`while read line` reads a file line by line. tessh has `read`, and it
fails at the end of the input, so `while read line` loops over the
script's own standard input (`cmd | tessh script.tsh`). `while read line
<file` reads the file once, line after line, as bash's `while read line;
do …; done <file` does: the same `read` goes on where it left off until
the end of the file. A `for` over the open file is the Python way, shown
below, and the file is closed when the `with` ends.

```tessh no-run
while read line <data.txt          # One line per pass, then the loop ends.
  print $line
```

```bash no-run
cat $filename |   # Supply input from a file.
while read line   # As long as there is another line to read ...
do
  ...
done
```

```tessh
printf "3\n4\n8\n" >data.txt       # (A made-up data file.)

with (open data.txt) as f
  values = [(float $line) for line in $f]   # One data point at a time.

print "mean:" ((sum $values) / (len $values))
```

```output
mean: 5.0
```

### until

`until` loops while its condition is false. tessh writes it `while not`.

### Example 11-19. until loop (`ex27.sh`)

```bash no-run
#!/bin/bash

END_CONDITION=end

until [ "$var1" = "$END_CONDITION" ]
# Tests condition here, at top of loop.
do
  echo "Input variable #1 "
  echo "($END_CONDITION to exit)"
  read var1
  echo "variable #1 = $var1"
  echo
done  

#                     ---                        #

#  As with "for" and "while" loops,
#+ an "until" loop permits C-like test constructs.

LIMIT=10
var=0

until (( var > LIMIT ))
do  # ^^ ^     ^     ^^   No brackets, no $ prefixing variables.
  echo -n "$var "
  (( var++ ))
done    # 0 1 2 3 4 5 6 7 8 9 10 


exit 0
```

```tessh
#!/usr/bin/env tessh
import io sys
$sys.stdin = io.StringIO "carrots\nend\n"   # (What someone types.)

END_CONDITION = end

var1 = ""
while not ($var1 == $END_CONDITION)
  # Tests condition here, at top of loop.
  print "Input variable #1 "
  print "($END_CONDITION to exit)"
  var1 = (input)
  print "variable #1 = $var1"
  print

LIMIT = 10
var = 0

while not ($var > $LIMIT)
  print "$var " end=""
  var += 1
print     # 0 1 2 3 4 5 6 7 8 9 10
```

```output
Input variable #1 
(end to exit)
variable #1 = carrots

Input variable #1 
(end to exit)
variable #1 = end

0 1 2 3 4 5 6 7 8 9 10 
```

## 11.2 Nested Loops
<!-- help: abs-nestedloops nested-loops inner-loop outer-loop -->

A loop in a loop: each pass of the outer one runs the inner one through.

### Example 11-20. Nested Loop (`nested-loop.sh`)

```bash
#!/bin/bash
# nested-loop.sh: Nested "for" loops.

outer=1             # Set outer loop counter.

# Beginning of outer loop.
for a in 1 2 3 4 5
do
  echo "Pass $outer in outer loop."
  echo "---------------------"
  inner=1           # Reset inner loop counter.

  # ===============================================
  # Beginning of inner loop.
  for b in 1 2 3 4 5
  do
    echo "Pass $inner in inner loop."
    let "inner+=1"  # Increment inner loop counter.
  done
  # End of inner loop.
  # ===============================================

  let "outer+=1"    # Increment outer loop counter. 
  echo              # Space between output blocks in pass of outer loop.
done               
# End of outer loop.

exit 0
```

The loop variables are the counters here, so no separate ones are kept.

```tessh
#!/usr/bin/env tessh
# Nested "for" loops.

for outer in (range 1 6)          # Beginning of outer loop.
  print "Pass $outer in outer loop."
  print "---------------------"

  for inner in (range 1 6)        # Beginning of inner loop.
    print "Pass $inner in inner loop."

  print                           # Space between output blocks.
```

```output
Pass 1 in outer loop.
---------------------
Pass 1 in inner loop.
Pass 2 in inner loop.
Pass 3 in inner loop.
Pass 4 in inner loop.
Pass 5 in inner loop.

Pass 2 in outer loop.
---------------------
Pass 1 in inner loop.
Pass 2 in inner loop.
Pass 3 in inner loop.
Pass 4 in inner loop.
Pass 5 in inner loop.

Pass 3 in outer loop.
---------------------
Pass 1 in inner loop.
Pass 2 in inner loop.
Pass 3 in inner loop.
Pass 4 in inner loop.
Pass 5 in inner loop.

Pass 4 in outer loop.
---------------------
Pass 1 in inner loop.
Pass 2 in inner loop.
Pass 3 in inner loop.
Pass 4 in inner loop.
Pass 5 in inner loop.

Pass 5 in outer loop.
---------------------
Pass 1 in inner loop.
Pass 2 in inner loop.
Pass 3 in inner loop.
Pass 4 in inner loop.
Pass 5 in inner loop.

```

## 11.3 Loop Control
<!-- help: abs-loopcontrol break continue break-n continue-n for-else -->

> Tournez cent tours, tournez mille tours, Tournez souvent et tournez
> toujours . . . --Verlaine, *Chevaux de bois*

`break` leaves the loop and `continue` goes on to its next pass, in tessh
as in bash.

### Example 11-21. Effects of break and continue in a loop (`ex28.sh`)

```bash
#!/bin/bash

LIMIT=19  # Upper limit

echo
echo "Printing Numbers 1 through 20 (but not 3 and 11)."

a=0

while [ $a -le "$LIMIT" ]
do
 a=$(($a+1))

 if [ "$a" -eq 3 ] || [ "$a" -eq 11 ]  # Excludes 3 and 11.
 then
   continue      # Skip rest of this particular loop iteration.
 fi

 echo -n "$a "   # This will not execute for 3 and 11.
done 

# Exercise:
# Why does the loop print up to 20?

echo; echo

echo Printing Numbers 1 through 20, but something happens after 2.

##################################################################

# Same loop, but substituting 'break' for 'continue'.

a=0

while [ "$a" -le "$LIMIT" ]
do
 a=$(($a+1))

 if [ "$a" -gt 2 ]
 then
   break  # Skip entire rest of loop.
 fi

 echo -n "$a "
done

echo; echo; echo

exit 0
```

```tessh
#!/usr/bin/env tessh
LIMIT = 19  # Upper limit

print
print "Printing Numbers 1 through 20 (but not 3 and 11)."

a = 0

while $a <= $LIMIT
  a += 1
  if $a in [3 11]   # Excludes 3 and 11.
    continue        # Skip rest of this particular loop iteration.
  print "$a " end=""

print ; print

print "Printing Numbers 1 through 20, but something happens after 2."

# Same loop, but substituting 'break' for 'continue'.

a = 0

while $a <= $LIMIT
  a += 1
  if $a > 2
    break           # Skip entire rest of loop.
  print "$a " end=""

print ; print ; print
```

```output

Printing Numbers 1 through 20 (but not 3 and 11).
1 2 4 5 6 7 8 9 10 12 13 14 15 16 17 18 19 20 

Printing Numbers 1 through 20, but something happens after 2.
1 2 


```

`break N` leaves N levels of loops at once, and `continue N` goes on with
the loop N levels up. Python's `break` and `continue` only act on the
innermost loop.

**Gap:** no `break N` or `continue N`. The ways around it: put the loops
in a function and `return`; use a loop's `else` (run when it ends without
`break`); or, for `continue 2` at the end of an outer loop's body, a plain
`break` of the inner loop.

### Example 11-22. Breaking out of multiple loop levels (`break-levels.sh`)

```bash
#!/bin/bash
# break-levels.sh: Breaking out of loops.

# "break N" breaks out of N level loops.

for outerloop in 1 2 3 4 5
do
  echo -n "Group $outerloop:   "

  # --------------------------------------------------------
  for innerloop in 1 2 3 4 5
  do
    echo -n "$innerloop "

    if [ "$innerloop" -eq 3 ]
    then
      break  # Try   break 2   to see what happens.
             # ("Breaks" out of both inner and outer loops.)
    fi
  done
  # --------------------------------------------------------

  echo
done  

echo

exit 0
```

```tessh
#!/usr/bin/env tessh
def groups
  for outerloop in (range 1 6)
    print "Group $outerloop:   " end=""

    # --------------------------------------------------------
    for innerloop in (range 1 6)
      print "$innerloop " end=""
      if $innerloop == 3
        break    # (return here would leave both loops: bash's break 2.)
    # --------------------------------------------------------

    print

groups
print
```

```output
Group 1:   1 2 3 
Group 2:   1 2 3 
Group 3:   1 2 3 
Group 4:   1 2 3 
Group 5:   1 2 3 

```

### Example 11-23. Continuing at a higher loop level (`continue-nlevel.sh`)

```bash
#!/bin/bash
# The "continue N" command, continuing at the Nth level loop.

for outer in I II III IV V           # outer loop
do
  echo; echo -n "Group $outer: "

  # --------------------------------------------------------------------
  for inner in 1 2 3 4 5 6 7 8 9 10  # inner loop
  do

    if [[ "$inner" -eq 7 && "$outer" = "III" ]]
    then
      continue 2  # Continue at loop on 2nd level, that is "outer loop".
                  # Replace above line with a simple "continue"
                  # to see normal loop behavior.
    fi  

    echo -n "$inner "  # 7 8 9 10 will not echo on "Group III."
  done  
  # --------------------------------------------------------------------

done

echo; echo

# Exercise:
# Come up with a meaningful use for "continue N" in a script.

exit 0
```

Nothing follows the inner loop in the outer one's body, so leaving the
inner loop is going on with the outer one.

```tessh
#!/usr/bin/env tessh
for outer in [I II III IV V]            # outer loop
  print ; print "Group $outer: " end=""

  # --------------------------------------------------------------------
  for inner in (range 1 11)             # inner loop
    if $inner == 7 and $outer == III
      break       # On to the outer loop's next pass (bash's continue 2).
    print "$inner " end=""              # 7 8 9 10 will not print on "Group III."
  # --------------------------------------------------------------------

print ; print
```

```output

Group I: 1 2 3 4 5 6 7 8 9 10 
Group II: 1 2 3 4 5 6 7 8 9 10 
Group III: 1 2 3 4 5 6 
Group IV: 1 2 3 4 5 6 7 8 9 10 
Group V: 1 2 3 4 5 6 7 8 9 10 

```

### Example 11-24. Using continue N in an actual task (`continue-n.example`)

```bash no-run
# Albert Reiner gives an example of how to use "continue N":
# ---------------------------------------------------------

#  Suppose I have a large number of jobs that need to be run, with
#+ any data that is to be treated in files of a given name pattern
#+ in a directory. There are several machines that access
#+ this directory, and I want to distribute the work over these
#+ different boxen.
#  Then I usually nohup something like the following on every box:

while true
do
  for n in .iso.*
  do
    [ "$n" = ".iso.opts" ] && continue
    beta=${n#.iso.}
    [ -r .Iso.$beta ] && continue
    [ -r .lock.$beta ] && sleep 10 && continue
    lockfile -r0 .lock.$beta || continue
    echo -n "$beta: " `date`
    run-isotherm $beta
    date
    ls -alF .Iso.$beta
    [ -r .Iso.$beta ] && rm -f .lock.$beta
    continue 2
  done
  break
done

exit 0

#  The details, in particular the sleep N, are particular to my
#+ application, but the general pattern is:

while true
do
  for job in {pattern}
  do
    {job already done or running} && continue
    {mark job as running, do job, mark job as done}
    continue 2
  done
  break        # Or something like `sleep 600' to avoid termination.
done

#  This way the script will stop only when there are no more jobs to do
#+ (including jobs that were added during runtime). Through the use
#+ of appropriate lockfiles it can be run on several machines
#+ concurrently without duplication of calculations [which run a couple
#+ of hours in my case, so I really want to avoid this]. Also, as search
#+ always starts again from the beginning, one can encode priorities in
#+ the file names. Of course, one could also do this without `continue 2',
#+ but then one would have to actually check whether or not some job
#+ was done (so that we should immediately look for the next job) or not
#+ (in which case we terminate or sleep for a long time before checking
#+ for a new job).
```

Python's `for … else` fits this pattern exactly: `break` out of the `for`
after a job starts the search again (bash's `continue 2`), and the `else`,
which runs only when the `for` found nothing to do, ends the `while`.
Opening the lock file with `x` mode fails if it exists, which is
`lockfile -r0`. The real job, `run-isotherm`, isn't here, so this doesn't
run.

```tessh no-run
import os time

while True
  for n in .iso.*
    if $n.name == .iso.opts
      continue
    beta = $n.name.removeprefix .iso.
    if (Path ".Iso.$beta").exists      # Done already.
      continue
    if (Path ".lock.$beta").exists     # Someone's on it.
      time.sleep 10
      continue
    try
      (open ".lock.$beta" x).close     # Take the lock, or go on.
    except FileExistsError
      continue
    print "$beta: " (date)
    run-isotherm $beta
    date
    ls -alF ".Iso.$beta"
    if (Path ".Iso.$beta").exists
      rm -f ".lock.$beta"
    break        # Start the search again (continue 2).
  else
    break        # No job left: stop.
```

## 11.4 Testing and Branching
<!-- help: abs-testbranch case esac match select menus -->

`case` picks a block by matching a value against patterns. tessh's
`match` is Python's: a `case` takes literal values (`"E" | "e"`), list and
dict shapes that also take them apart, and `if` guards. A bash glob
pattern is a guard, `case _ if $x =~ [a-z]*`, as `=~` with a glob tests
the whole string; for character classes, `str`'s `isalpha`, `isdigit`,
`islower` and `isupper` say it in words.

`$x =~ [0-9]` is a one-character glob, as a `case` pattern is.
**Gap:** globs have no POSIX classes such as `[[:lower:]]`.

### Example 11-25. Using case (`ex29.sh`)

```bash no-run
#!/bin/bash
# Testing ranges of characters.

echo; echo "Hit a key, then hit return."
read Keypress

case "$Keypress" in
  [[:lower:]]   ) echo "Lowercase letter";;
  [[:upper:]]   ) echo "Uppercase letter";;
  [0-9]         ) echo "Digit";;
  *             ) echo "Punctuation, whitespace, or other";;
esac      #  Allows ranges of characters in [square brackets],
          #+ or POSIX ranges in [[double square brackets.

#  In the first version of this example,
#+ the tests for lowercase and uppercase characters were
#+ [a-z] and [A-Z].
#  This no longer works in certain locales and/or Linux distros.
#  POSIX is more portable.
#  Thanks to Frank Wang for pointing this out.

#  Exercise:
#  --------
#  As the script stands, it accepts a single keystroke, then terminates.
#  Change the script so it accepts repeated input,
#+ reports on each keystroke, and terminates only when "X" is hit.
#  Hint: enclose everything in a "while" loop.

exit 0
```

With canned keys, and the exercise done: it goes on until `X`.

```tessh
#!/usr/bin/env tessh
import io sys
$sys.stdin = io.StringIO "q\nQ\n7\n!\nX\n"   # (The keys someone hits.)

while True
  print ; print "Hit a key, then hit return."
  Keypress = (input)
  match $Keypress
    case X
      break
    case _ if ($Keypress.islower)
      print "Lowercase letter"
    case _ if ($Keypress.isupper)
      print "Uppercase letter"
    case _ if ($Keypress.isdigit)
      print "Digit"
    case _
      print "Punctuation, whitespace, or other"
```

```output

Hit a key, then hit return.
Lowercase letter

Hit a key, then hit return.
Uppercase letter

Hit a key, then hit return.
Digit

Hit a key, then hit return.
Punctuation, whitespace, or other

Hit a key, then hit return.
```

### Example 11-26. Creating menus using case (`ex30.sh`)

```bash no-run
#!/bin/bash

# Crude address database

clear # Clear the screen.

echo "          Contact List"
echo "          ------- ----"
echo "Choose one of the following persons:" 
echo
echo "[E]vans, Roland"
echo "[J]ones, Mildred"
echo "[S]mith, Julie"
echo "[Z]ane, Morris"
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
# Note double semicolon to terminate each option.

  "J" | "j" )
  echo
  echo "Mildred Jones"
  echo "249 E. 7th St., Apt. 19"
  echo "New York, NY 10009"
  echo "(212) 533-2814"
  echo "(212) 533-9972 fax"
  echo "milliej@loisaida.com"
  echo "Ex-girlfriend"
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

echo

#  Exercise:
#  --------
#  Change the script so it accepts multiple inputs,
#+ instead of terminating after displaying just one address.

exit 0
```

Each `case` ends at the next one: no `;;`, and no falling through.

```tessh
#!/usr/bin/env tessh
import io sys
$sys.stdin = io.StringIO "j\n"     # (The answer someone types.)

# Crude address database

# clear    (Clear the screen: not here.)

print "          Contact List"
print "          ------- ----"
print "Choose one of the following persons:"
print
print "[E]vans, Roland"
print "[J]ones, Mildred"
print "[S]mith, Julie"
print "[Z]ane, Morris"
print

person = (input)

match $person
  case "E" | "e"
    # Accept upper or lowercase input.
    print
    print "Roland Evans"
    print "4321 Flash Dr."
    print "Hardscrabble, CO 80753"
    print "(303) 734-9874"
    print "(303) 734-9892 fax"
    print "revans@zzy.net"
    print "Business partner & old friend"
  case "J" | "j"
    print
    print "Mildred Jones"
    print "249 E. 7th St., Apt. 19"
    print "New York, NY 10009"
    print "(212) 533-2814"
    print "(212) 533-9972 fax"
    print "milliej@loisaida.com"
    print "Ex-girlfriend"
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
          ------- ----
Choose one of the following persons:

[E]vans, Roland
[J]ones, Mildred
[S]mith, Julie
[Z]ane, Morris


Mildred Jones
249 E. 7th St., Apt. 19
New York, NY 10009
(212) 533-2814
(212) 533-9972 fax
milliej@loisaida.com
Ex-girlfriend
Birthday: Feb. 11

```

A `case` on a command-line argument, to put `./` in front of a file name
that starts with a dash:

```bash no-run
case "$1" in
  "") echo "Usage: ${0##*/} <filename>"; exit $E_PARAM;;
                      # No command-line parameters,
                      # or first parameter empty.
# Note that ${0##*/} is ${var##pattern} param substitution.
                      # Net result is $0.

  -*) FILENAME=./$1;;   #  If filename passed as argument ($1)
                      #+ starts with a dash,
                      #+ replace it with ./$1
                      #+ so further commands don't interpret it
                      #+ as an option.

  * ) FILENAME=$1;;     # Otherwise, $1.
esac
```

```tessh
def filename_for arg
  match $arg
    case ""
      print "Usage: $(Path $argv[0]).name <filename>"
      None
    case _ if ($arg.startswith "-")
      "./$arg"      # So further commands don't take it for an option.
    case _
      $arg

print (filename_for "") (filename_for "-weird") (filename_for notes.txt)
```

```output
Usage: ex.tsh <filename>
None ./-weird notes.txt
```

And a loop over the options. tessh has `shift`, but here a `match` on the
argument list takes an option and its value off the front in one pattern,
so no `shift` is needed.

```bash no-run
while [ $# -gt 0 ]; do    # Until you run out of parameters . . .
  case "$1" in
    -d|--debug)
              # "-d" or "--debug" parameter?
              DEBUG=1
              ;;
    -c|--conf)
              CONFFILE="$2"
              shift
              if [ ! -f $CONFFILE ]; then
                echo "Error: Supplied file doesn't exist!"
                exit $E_CONFFILE     # File not found error.
              fi
              ;;
  esac
  shift       # Check next set of parameters.
done

#  From Stefano Falsetto's "Log2Rot" script,
#+ part of his "rottlog" package.
#  Used with permission.
```

```tessh
E_CONFFILE = 1
echo "rotate weekly" >my.conf      # (A made-up config file.)
args = $argv[1:] or [-d --conf my.conf extra]   # (sample arguments)
DEBUG = 0
CONFFILE = None

while $args                        # Until you run out of parameters . . .
  match $args
    case [("-d" | "--debug") *$rest]
      DEBUG = 1
    case [("-c" | "--conf") $CONFFILE *$rest]
      if not ((Path $CONFFILE).is_file)
        print "Supplied file doesn't exist!"
        exit $E_CONFFILE           # File not found.
    case [_ *$rest]
      pass                         # Anything else: skip it.
  args = $rest                     # Check next set of parameters.

print "DEBUG=$DEBUG CONFFILE=$CONFFILE"
```

```output
DEBUG=1 CONFFILE=my.conf
```

`argparse` does all of this, with a usage message, for a script that takes
real options.

### Example 11-27. Using command substitution to generate the case variable (`case-cmd.sh`)

```bash
#!/bin/bash
# case-cmd.sh: Using command substitution to generate a "case" variable.

case $( arch ) in   # $( arch ) returns machine architecture.
                    # Equivalent to 'uname -m' ...
  i386 ) echo "80386-based machine";;
  i486 ) echo "80486-based machine";;
  i586 ) echo "Pentium-based machine";;
  i686 ) echo "Pentium2+-based machine";;
  *    ) echo "Other type of machine";;
esac

exit 0
```

```tessh
#!/usr/bin/env tessh
match (uname -m)    # The machine's architecture (arch isn't everywhere).
  case i386
    print "80386-based machine"
  case i486
    print "80486-based machine"
  case i586
    print "Pentium-based machine"
  case i686
    print "Pentium2+-based machine"
  case _
    print "Other type of machine"
```

```output
Other type of machine
```

### Example 11-28. Simple string matching (`match-string.sh`)

```bash
#!/bin/bash
# match-string.sh: Simple string matching
#                  using a 'case' construct.

match_string ()
{ # Exact string match.
  MATCH=0
  E_NOMATCH=90
  PARAMS=2     # Function requires 2 arguments.
  E_BAD_PARAMS=91

  [ $# -eq $PARAMS ] || return $E_BAD_PARAMS

  case "$1" in
  "$2") return $MATCH;;
  *   ) return $E_NOMATCH;;
  esac

}  


a=one
b=two
c=three
d=two


match_string $a     # wrong number of parameters
echo $?             # 91

match_string $a $b  # no match
echo $?             # 90

match_string $b $d  # match
echo $?             # 0


exit 0		    
```

A tessh function returns a value rather than an exit status, so the codes
are what it returns. In a `match`, a `$name` captures, so comparing with a
variable is a guard (or plainly `==`).

```tessh
#!/usr/bin/env tessh
def match_string *args
  # Exact string match.
  MATCH = 0
  E_NOMATCH = 90
  PARAMS = 2         # Function requires 2 arguments.
  E_BAD_PARAMS = 91

  if (len $args) != $PARAMS
    return $E_BAD_PARAMS

  match $args
    case [$x $y] if $x == $y
      $MATCH
    case _
      $E_NOMATCH

a = one
b = two
c = three
d = two

print (match_string $a)       # wrong number of parameters: 91
print (match_string $a $b)    # no match: 90
print (match_string $b $d)    # match: 0
```

```output
91
90
0
```

### Example 11-29. Checking for alphabetic input (`isalpha.sh`)

```bash
#!/bin/bash
# isalpha.sh: Using a "case" structure to filter a string.

SUCCESS=0
FAILURE=1   #  Was FAILURE=-1,
            #+ but Bash no longer allows negative return value.

isalpha ()  # Tests whether *first character* of input string is alphabetic.
{
if [ -z "$1" ]                # No argument passed?
then
  return $FAILURE
fi

case "$1" in
  [a-zA-Z]*) return $SUCCESS;;  # Begins with a letter?
  *        ) return $FAILURE;;
esac
}             # Compare this with "isalpha ()" function in C.


isalpha2 ()   # Tests whether *entire string* is alphabetic.
{
  [ $# -eq 1 ] || return $FAILURE

  case $1 in
  *[!a-zA-Z]*|"") return $FAILURE;;
               *) return $SUCCESS;;
  esac
}

isdigit ()    # Tests whether *entire string* is numerical.
{             # In other words, tests for integer variable.
  [ $# -eq 1 ] || return $FAILURE

  case $1 in
    *[!0-9]*|"") return $FAILURE;;
              *) return $SUCCESS;;
  esac
}



check_var ()  # Front-end to isalpha ().
{
if isalpha "$@"
then
  echo "\"$*\" begins with an alpha character."
  if isalpha2 "$@"
  then        # No point in testing if first char is non-alpha.
    echo "\"$*\" contains only alpha characters."
  else
    echo "\"$*\" contains at least one non-alpha character."
  fi  
else
  echo "\"$*\" begins with a non-alpha character."
              # Also "non-alpha" if no argument passed.
fi

echo

}

digit_check ()  # Front-end to isdigit ().
{
if isdigit "$@"
then
  echo "\"$*\" contains only digits [0 - 9]."
else
  echo "\"$*\" has at least one non-digit character."
fi

echo

}

a=23skidoo
b=H3llo
c=-What?
d=What?
e=$(echo $b)   # Command substitution.
f=AbcDef
g=27234
h=27a34
i=27.34

check_var $a
check_var $b
check_var $c
check_var $d
check_var $e
check_var $f
check_var     # No argument passed, so what happens?
#
digit_check $g
digit_check $h
digit_check $i


exit 0        # Script improved by S.C.

# Exercise:
# --------
#  Write an 'isfloat ()' function that tests for floating point numbers.
#  Hint: The function duplicates 'isdigit ()',
#+ but adds a test for a mandatory decimal point.
```

The same glob patterns, with `=~`, in functions that return `True` or
`False`. (`str.isalpha` would also take letters outside `a-z`, such as
`é`.) An argument that may be missing gets a default of `""`.

```tessh
#!/usr/bin/env tessh
def isalpha s=""        # Tests whether *first character* is alphabetic.
  $s =~ [a-zA-Z]*       # Begins with a letter? ("" doesn't.)

def isalpha2 s=""       # Tests whether *entire string* is alphabetic.
  $s != "" and $s !~ *[!a-zA-Z]*

def isdigit s=""        # Tests whether *entire string* is numerical.
  $s != "" and $s !~ *[!0-9]*

def check_var s=""      # Front-end to isalpha.
  if (isalpha $s)
    print "\"$s\" begins with an alpha character."
    if (isalpha2 $s)    # No point in testing if first char is non-alpha.
      print "\"$s\" contains only alpha characters."
    else
      print "\"$s\" contains at least one non-alpha character."
  else
    print "\"$s\" begins with a non-alpha character."
                        # Also "non-alpha" if no argument passed.
  print

def digit_check s=""    # Front-end to isdigit.
  if (isdigit $s)
    print "\"$s\" contains only digits [0 - 9]."
  else
    print "\"$s\" has at least one non-digit character."
  print

a = 23skidoo
b = H3llo
c = "-What?"
d = "What?"
e = echo $b      # Command substitution.
f = AbcDef
g = "27234"      # (Quoted: bare, 27234 would be a number.)
h = 27a34
i = "27.34"

check_var $a
check_var $b
check_var $c
check_var $d
check_var $e
check_var $f
check_var        # No argument passed, so what happens?
#
digit_check $g
digit_check $h
digit_check $i
```

```output
"23skidoo" begins with a non-alpha character.

"H3llo" begins with an alpha character.
"H3llo" contains at least one non-alpha character.

"-What?" begins with a non-alpha character.

"What?" begins with an alpha character.
"What?" contains at least one non-alpha character.

"H3llo" begins with an alpha character.
"H3llo" contains at least one non-alpha character.

"AbcDef" begins with an alpha character.
"AbcDef" contains only alpha characters.

"" begins with a non-alpha character.

"27234" contains only digits [0 - 9].

"27a34" has at least one non-digit character.

"27.34" has at least one non-digit character.

```

### select

`select` prints a numbered menu of its list, reads a choice with the `$PS3`
prompt, and runs its body with the variable set to the item chosen.

tessh's `select` is a loop over the choices: `for color in (select red
green blue)` prints the menu, asks `$PS3`, and gives the item picked each
time, until the input ends. The port below does it by hand, with `input`,
so its answers can be canned.

### Example 11-30. Creating menus using select (`ex31.sh`)

```bash no-run
#!/bin/bash

PS3='Choose your favorite vegetable: ' # Sets the prompt string.
                                       # Otherwise it defaults to #? .

echo

select vegetable in "beans" "carrots" "potatoes" "onions" "rutabagas"
do
  echo
  echo "Your favorite veggie is $vegetable."
  echo "Yuck!"
  echo
  break  # What happens if there is no 'break' here?
done

exit

# Exercise:
# --------
#  Fix this script to accept user input not specified in
#+ the "select" statement.
#  For example, if the user inputs "peas,"
#+ the script would respond "Sorry. That is not on the menu."
```

`select` here asks until it gets a number on the menu, as bash's does, and
says so when it doesn't (the exercise). The answers are canned.

```tessh
#!/usr/bin/env tessh
import io sys
$sys.stdin = io.StringIO "peas\n2\n"     # (What someone types.)

def select prompt items
  # Bash's select: a numbered menu, asked until a choice is on it.
  for [n item] in (enumerate $items 1)
    print "$n) $item"
  while True
    reply = input $prompt
    print           # (The Enter someone types.)
    if ($reply.isdigit) and 1 <= (int $reply) <= (len $items)
      return $items[(int $reply) - 1]
    print "Sorry. That is not on the menu."

PS3 = 'Choose your favorite vegetable: '   # The prompt string.

print

vegetable = select $PS3 [beans carrots potatoes onions rutabagas]
print
print "Your favorite veggie is $vegetable."
print "Yuck!"
print
```

```output

1) beans
2) carrots
3) potatoes
4) onions
5) rutabagas
Choose your favorite vegetable: 
Sorry. That is not on the menu.
Choose your favorite vegetable: 

Your favorite veggie is carrots.
Yuck!

```

Without `in list`, `select` uses the arguments of the script or function
it's in.

### Example 11-31. Creating menus using select in a function (`ex32.sh`)

```bash no-run
#!/bin/bash

PS3='Choose your favorite vegetable: '

echo

choice_of()
{
select vegetable
# [in list] omitted, so 'select' uses arguments passed to function.
do
  echo
  echo "Your favorite veggie is $vegetable."
  echo "Yuck!"
  echo
  break
done
}

choice_of beans rice carrots radishes rutabaga spinach
#         $1    $2   $3      $4       $5       $6
#         passed to choice_of() function

exit 0
```

```tessh
#!/usr/bin/env tessh
import io sys
$sys.stdin = io.StringIO "6\n"           # (What someone types.)

def select prompt items
  for [n item] in (enumerate $items 1)
    print "$n) $item"
  while True
    reply = input $prompt
    print
    if ($reply.isdigit) and 1 <= (int $reply) <= (len $items)
      return $items[(int $reply) - 1]

PS3 = 'Choose your favorite vegetable: '

print

def choice_of *vegetables
  vegetable = select $PS3 $vegetables    # The function's arguments.
  print
  print "Your favorite veggie is $vegetable."
  print "Yuck!"
  print

choice_of beans rice carrots radishes rutabaga spinach
```

```output

1) beans
2) rice
3) carrots
4) radishes
5) rutabaga
6) spinach
Choose your favorite vegetable: 

Your favorite veggie is spinach.
Yuck!

```
