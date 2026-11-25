## 16.5 File and Archiving Commands
<!-- help: abs-file-archiving tar shar ar rpm cpio pax gzip bzip2 xz zip file which whereis readlink strings diff patch cmp comm basename dirname split csplit md5sum sha1sum uuencode openssl shred mktemp make install -->

**Archiving.** `tar` packs files into one (`-c` create, `-x` extract, `-t`
list, `-r` append, `-u` update, `-z`/`-j`/`-J` through gzip, bzip2 or xz),
`shar` makes a shell-script archive, `ar` a library of object files, `rpm`
installs and queries Red Hat packages, `cpio` copies files in and out of
archives (and whole trees), `rpm2cpio` turns an rpm into a cpio archive,
and `pax` is POSIX's would-be replacement for both. **Compression:**
`gzip`/`gunzip`/`zcat`, `bzip2`, `compress`, `zip`/`unzip`, `lzma`, `xz`.
**File information:** `file` names a file's type from its contents,
`which` and `whereis` find a program, `whatis` says what it is, `vdir` is `ls
-lb`, `locate` searches a database of names, `getfacl`/`setfacl` read and
set access control lists, `readlink` shows where a link points, and
`strings` digs the text out of a binary. **Comparison:** `diff` (and
`patch`, which applies its output), `diff3`, `merge`, `sdiff`, `cmp` and
`comm`. **Utilities:** `basename`, `dirname`, `split`, `csplit`.
**Encoding:** `sum`, `cksum`, `md5sum`, `sha1sum`/`sha256sum`, `uuencode`,
`mimencode`, `openssl`, `shred`. **Others:** `mktemp`, `make`, `install`,
`dos2unix`, `ptx`, `more` and `less`.

All of them run in tessh as in bash. Python has its own modules for much of
this (`tarfile`, `zipfile`, `gzip`, `hashlib`, `filecmp`, `difflib`,
`tempfile`, `shutil`), and Paths do `basename` and `dirname` as `.name` and
`.parent`; a script uses whichever is shorter.

```bash
mkdir -p project/src ; echo 'int x;' > project/src/x.c ; echo notes > project/README
tar czf project.tar.gz project
tar tzf project.tar.gz
```

```output bash
project/
project/src/
project/src/x.c
project/README
```

```tessh
mkdir -p project/src ; echo 'int x;' > project/src/x.c ; echo notes > project/README
tar czf project.tar.gz project
tar tzf project.tar.gz
# Python's tarfile reads it as a value:
import tarfile
with (tarfile.open project.tar.gz) as t
  print [$m.name for m in ($t.getmembers) if ($m.isfile)]
```

```output
project/
project/src/
project/src/x.c
project/README
['project/src/x.c' 'project/README']
```

### Example 16-30. Using `cpio` to move a directory tree (`ex48.sh`)

```bash no-run
#!/bin/bash

# Copying a directory tree using cpio.

# Advantages of using 'cpio':
#   Speed of copying. It's faster than 'tar' with pipes.
#   Well suited for copying special files (named pipes, etc.)
#+  that 'cp' may choke on.

ARGS=2
E_BADARGS=65

if [ $# -ne "$ARGS" ]
then
  echo "Usage: `basename $0` source destination"
  exit $E_BADARGS
fi  

source="$1"
destination="$2"

###################################################################
find "$source" -depth | cpio -admvp "$destination"
#               ^^^^^         ^^^^^
#  Read the 'find' and 'cpio' info pages to decipher these options.
#  The above works only relative to $PWD (current directory) . . .
#+ full pathnames are specified.
###################################################################


# Exercise:
# --------

#  Add code to check the exit status ($?) of the 'find | cpio' pipe
#+ and output appropriate error messages if anything went wrong.

exit $?
```

The pipeline is the same. The exercise asks for the exit status of the
`find | cpio` pipe: `$?` after a pipe is the last program's, as in bash
without `pipefail`, and `cpio` is the one that fails when anything does.
`cpio -v` lists the files on stderr; here the copy is listed after.

```tessh
#!/usr/bin/env tessh
# (A made-up tree to copy.)
mkdir -p notes/2026/nov ; echo a > notes/todo ; echo b > notes/2026/nov/23

ARGS = 2
E_BADARGS = 65
E_CPIO = 66

args = $argv[1:] or [notes copy]   # (sample arguments, when none are given)
if (len $args) != $ARGS
  print "Usage: $(Path $argv[0]).name source destination"
  exit $E_BADARGS
[source destination] = $args

###################################################################
find $source -depth | cpio -admp $destination 2>/dev/null
###################################################################
if $? != 0                         # (The exercise.)
  print "cpio failed to copy $source to $destination."
  exit $E_CPIO

find $destination | sort
exit 0
```

```output
copy
copy/notes
copy/notes/2026
copy/notes/2026/nov
copy/notes/2026/nov/23
copy/notes/todo
```

### Example 16-31. Unpacking an *rpm* archive (`de-rpm.sh`)

```bash no-run
#!/bin/bash
# de-rpm.sh: Unpack an 'rpm' archive

: ${1?"Usage: `basename $0` target-file"}
# Must specify 'rpm' archive name as an argument.


TEMPFILE=$$.cpio                         #  Tempfile with "unique" name.
                                         #  $$ is process ID of script.

rpm2cpio < $1 > $TEMPFILE                #  Converts rpm archive into
                                         #+ cpio archive.
cpio --make-directories -F $TEMPFILE -i  #  Unpacks cpio archive.
rm -f $TEMPFILE                          #  Deletes cpio archive.

exit 0

#  Exercise:
#  Add check for whether 1) "target-file" exists and
#+                       2) it is an rpm archive.
#  Hint:                    Parse output of 'file' command.
```

`${1?…}` is a usage check that stops the script; in tessh it's an `if`.
`tempfile` makes the temporary file (bash's `$$.cpio` is only unique
among running processes), and the exercise checks the file with `file`.
It needs an rpm and `rpm2cpio`, so it doesn't run here.

```tessh no-run
#!/usr/bin/env tessh
# de-rpm.tsh: Unpack an 'rpm' archive
import tempfile

if not $argv[1:]
  print "Usage: $(Path $argv[0]).name target-file"
  exit 1
rpm = Path $argv[1]
if not ($rpm.is_file)                       # Exercise, 1)
  print "No such file: $rpm"
  exit 1
if (file -b $rpm) !~ 'RPM'                  # Exercise, 2)
  print "Not an rpm archive: $rpm"
  exit 1

with (tempfile.NamedTemporaryFile suffix=.cpio) as tmp
  rpm2cpio <$rpm >$tmp.name                 # rpm archive to cpio archive.
  cpio --make-directories -F $tmp.name -i   # Unpacks cpio archive.
# (The temporary file goes away at the end of the with.)
exit 0
```

`file` names a file's type. The ABS Guide finds the shell scripts in a
folder with `file $DIRECTORY/* | fgrep Bourne`; that's the same in tessh,
with a Glob for the files.

### Example 16-32. Stripping comments from C program files (`strip-comments.sh`)

```bash no-run
#!/bin/bash
# strip-comment.sh: Strips out the comments (/* COMMENT */) in a C program.

E_NOARGS=0
E_ARGERROR=66
E_WRONG_FILE_TYPE=67

if [ $# -eq "$E_NOARGS" ]
then
  echo "Usage: `basename $0` C-program-file" >&2 # Error message to stderr.
  exit $E_ARGERROR
fi  

# Test for correct file type.
type=`file $1 | awk '{ print $2, $3, $4, $5 }'`
# "file $1" echoes file type . . .
# Then awk removes the first field, the filename . . .
# Then the result is fed into the variable "type."
correct_type="ASCII C program text"

if [ "$type" != "$correct_type" ]
then
  echo
  echo "This script works on C program files only."
  echo
  exit $E_WRONG_FILE_TYPE
fi  


# Rather cryptic sed script:
#--------
sed '
/^\/\*/d
/.*\*\//d
' $1
#--------
# Easy to understand if you take several hours to learn sed fundamentals.


#  Need to add one more line to the sed script to deal with
#+ case where line of code has a comment following it on same line.
#  This is left as a non-trivial exercise.

#  Also, the above code deletes non-comment lines with a "*/" . . .
#+ not a desirable result.

exit 0


# ----------------------------------------------------------------
# Code below this line will not execute because of 'exit 0' above.

# Stephane Chazelas suggests the following alternative:

usage() {
  echo "Usage: `basename $0` C-program-file" >&2
  exit 1
}

WEIRD=`echo -n -e '\377'`   # or WEIRD=$'\377'
[[ $# -eq 1 ]] || usage
case `file "$1"` in
  *"C program text"*) sed -e "s%/\*%${WEIRD}%g;s%\*/%${WEIRD}%g" "$1" \
     | tr '\377\n' '\n\377' \
     | sed -ne 'p;n' \
     | tr -d '\n' | tr '\377' '\n';;
  *) usage;;
esac

#  This is still fooled by things like:
#  printf("/*");
#  or
#  /*  /* buggy embedded comment */
#
#  To handle all special cases (comments in strings, comments in string
#+ where there is a \", \\" ...),
#+ the only way is to write a C parser (using lex or yacc perhaps?).

exit 0
```

`file` calls a C file "C source, ASCII text" nowadays, so the type test
looks for `C source` in `file -b`'s answer with `=~`. The `sed` script is
the same, and so are its faults (it deletes a line with code before a
comment, and a comment's middle lines survive). The exercise, comments
anywhere, is easy with a Python regex: `s` lets `.` match newlines, and
`*?` stops at the first `*/`, and `m` lets `^` match at each line.

```tessh
#!/usr/bin/env tessh
import sys
cat >hello.c <<'END'
/* hello.c: says hello */
#include <stdio.h>

int main(void)
{
  printf("hello\n");   /* greet */
  return 0;
}
END

E_ARGERROR = 66
E_WRONG_FILE_TYPE = 67

args = $argv[1:] or [hello.c]     # (sample argument, when none is given)
if not $args
  print "Usage: $(Path $argv[0]).name C-program-file" file=$sys.stderr
  exit $E_ARGERROR
cfile = $args[0]   # (not "file": that name would hide the program)

# Test for correct file type.
if (file -b $cfile) !~ 'C source'
  print
  print "This script works on C program files only."
  print
  exit $E_WRONG_FILE_TYPE

# Rather cryptic sed script:
sed '
/^\/\*/d
/.*\*\//d
' $cfile

print "-----"
# The exercise: every comment, wherever it is.
# (A comment alone on its line goes with its line end.)
print (re'^[ \t]*/\*.*?\*/\n|[ \t]*/\*.*?\*/'ms.sub "" ((Path $cfile).read_text)) end=""
exit 0
```

```output
#include <stdio.h>

int main(void)
{
  return 0;
}
-----
#include <stdio.h>

int main(void)
{
  printf("hello\n");
  return 0;
}
```

`which` finds a program (tessh's `which` is a builtin whose value is the
path), `whereis` its manual page too, and `whatis` its one-line
description.

### Example 16-33. Exploring `/usr/X11R6/bin` (`what.sh`)

```bash no-run
#!/bin/bash

# What are all those mysterious binaries in /usr/X11R6/bin?

DIRECTORY="/usr/X11R6/bin"
# Try also "/bin", "/usr/bin", "/usr/local/bin", etc.

for file in $DIRECTORY/*
do
  whatis `basename $file`   # Echoes info about the binary.
done

exit 0

#  Note: For this to work, you must create a "whatis" database
#+ with /usr/sbin/makewhatis.
#  You may wish to redirect output of this script, like so:
#    ./what.sh >>whatis.db
#  or view it a page at a time on stdout,
#    ./what.sh | less
```

The folder is a Path and the loop a Glob over it; each file's `.name` is
what `whatis` wants. It needs a `whatis` database (there's none here) and
the folder, so it doesn't run.

```tessh no-run
#!/usr/bin/env tessh
DIRECTORY = /usr/X11R6/bin
# Try also /bin, /usr/bin, /usr/local/bin, etc.

for file in ($DIRECTORY.glob "*")
  whatis $file.name       # Echoes info about the binary.

exit 0
```

`vdir`, `locate`, `getfacl`/`setfacl` and `readlink` run as they are
(`Path.readlink` is `readlink` in Python). `strings` lists the runs of
printable characters in a file.

### Example 16-34. An *improved* `strings` command (`wstrings.sh`)

```bash no-run
#!/bin/bash
# wstrings.sh: "word-strings" (enhanced "strings" command)
#
#  This script filters the output of "strings" by checking it
#+ against a standard word list file.
#  This effectively eliminates gibberish and noise,
#+ and outputs only recognized words.

# ===========================================================
#                 Standard Check for Script Argument(s)
ARGS=1
E_BADARGS=85
E_NOFILE=86

if [ $# -ne $ARGS ]
then
  echo "Usage: `basename $0` filename"
  exit $E_BADARGS
fi

if [ ! -f "$1" ]                      # Check if file exists.
then
    echo "File \"$1\" does not exist."
    exit $E_NOFILE
fi
# ===========================================================


MINSTRLEN=3                           #  Minimum string length.
WORDFILE=/usr/share/dict/linux.words  #  Dictionary file.
#  May specify a different word list file
#+ of one-word-per-line format.
#  For example, the "yawl" word-list package,
#  http://bash.deta.in/yawl-0.3.2.tar.gz


wlist=`strings "$1" | tr A-Z a-z | tr '[:space:]' Z | \
       tr -cs '[:alpha:]' Z | tr -s '\173-\377' Z | tr Z ' '`

# Translate output of 'strings' command with multiple passes of 'tr'.
#  "tr A-Z a-z"  converts to lowercase.
#  "tr '[:space:]'"  converts whitespace characters to Z's.
#  "tr -cs '[:alpha:]' Z"  converts non-alphabetic characters to Z's,
#+ and squeezes multiple consecutive Z's.
#  "tr -s '\173-\377' Z"  converts all characters past 'z' to Z's
#+ and squeezes multiple consecutive Z's,
#+ which gets rid of all the weird characters that the previous
#+ translation failed to deal with.
#  Finally, "tr Z ' '" converts all those Z's to whitespace,
#+ which will be seen as word separators in the loop below.

#  ***********************************************************************
#  Note the technique of feeding/piping the output of 'tr' back to itself,
#+ but with different arguments and/or options on each successive pass.
#  ***********************************************************************


for word in $wlist                    #  Important:
                                      #  $wlist must not be quoted here.
                                      # "$wlist" does not work.
                                      #  Why not?
do
  strlen=${#word}                     #  String length.
  if [ "$strlen" -lt "$MINSTRLEN" ]   #  Skip over short strings.
  then
    continue
  fi

  grep -Fw $word "$WORDFILE"          #   Match whole words only.
#      ^^^                            #  "Fixed strings" and
                                      #+ "whole words" options. 
done  

exit $?
```

Five `tr` passes turn `strings`' output into words; in tessh one regex
finds the runs of letters, `if (len $word) < $MINSTRLEN` skips the short
ones, and the word list is read once into a set instead of `grep`ped once
per word. It runs on a made-up binary file and word list.

```tessh
#!/usr/bin/env tessh
# (A made-up binary file and word list.)
printf '\x00\x01ELF\x02hello world\x00\xff\xfeqzx the\x03PRINTER_status\x00ok\n' > data.bin
printf "%s\n" hello printer status the world > words

ARGS = 1
E_BADARGS = 85
E_NOFILE = 86

args = $argv[1:] or [data.bin]   # (sample argument, when none is given)
if (len $args) != $ARGS
  print "Usage: $(Path $argv[0]).name filename"
  exit $E_BADARGS
if not ((Path $args[0]).is_file)   # Check if file exists.
  print "File \"$args[0]\" does not exist."
  exit $E_NOFILE

MINSTRLEN = 3                    # Minimum string length.
WORDFILE = ./words               # Dictionary file (/usr/share/dict/linux.words).

dictionary = set (cat $WORDFILE).lines
wlist = re'[a-z]+'.findall ((strings $args[0]).lower)

for word in $wlist
  if (len $word) < $MINSTRLEN    # Skip over short strings.
    continue
  if $word in $dictionary        # Whole words only.
    print $word

exit 0
```

```output
hello
world
the
printer
status
```

`diff` shows how two files differ, `patch` applies a `diff` to the old
file, and `cmp` says only where they first differ. Both exit 0 when the
files are the same and 1 when not, so they work as `if` conditions.

### Example 16-35. Using `cmp` to compare two files within a script. (`file-comparison.sh`)

```bash no-run
#!/bin/bash
# file-comparison.sh

ARGS=2  # Two args to script expected.
E_BADARGS=85
E_UNREADABLE=86

if [ $# -ne "$ARGS" ]
then
  echo "Usage: `basename $0` file1 file2"
  exit $E_BADARGS
fi

if [[ ! -r "$1" || ! -r "$2" ]]
then
  echo "Both files to be compared must exist and be readable."
  exit $E_UNREADABLE
fi

cmp $1 $2 &> /dev/null
#   Redirection to /dev/null buries the output of the "cmp" command.
#   cmp -s $1 $2  has same result ("-s" silent flag to "cmp")
#   Thank you  Anders Gustavsson for pointing this out.
#
#  Also works with 'diff', i.e.,
#+ diff $1 $2 &> /dev/null

if [ $? -eq 0 ]         # Test exit status of "cmp" command.
then
  echo "File \"$1\" is identical to file \"$2\"."
else  
  echo "File \"$1\" differs from file \"$2\"."
fi

exit 0
```

`if cmp -s …` tests the status directly, and `os.access` is bash's `-r`
test. Python's `filecmp.cmp f1 f2 shallow=False` does the same comparison.

```tessh
#!/usr/bin/env tessh
import os
printf 'one\ntwo\n' > a.txt ; printf 'one\ntwo\n' > b.txt ; printf 'one\n2\n' > c.txt

ARGS = 2  # Two args to script expected.
E_BADARGS = 85
E_UNREADABLE = 86

def compare args
  if (len $args) != $ARGS
    print "Usage: $(Path $argv[0]).name file1 file2"
    exit $E_BADARGS
  [f1 f2] = $args
  if not ((os.access $f1 $os.R_OK) and (os.access $f2 $os.R_OK))
    print "Both files to be compared must exist and be readable."
    exit $E_UNREADABLE

  if cmp -s $f1 $f2          # ("-s" silent flag to "cmp")
    print "File \"$f1\" is identical to file \"$f2\"."
  else
    print "File \"$f1\" differs from file \"$f2\"."

# (sample arguments, when none are given)
if $argv[1:]
  compare $argv[1:]
else
  compare [a.txt b.txt]
  compare [a.txt c.txt]
exit 0
```

```output
File "a.txt" is identical to file "b.txt".
File "a.txt" differs from file "c.txt".
```

`comm` puts the lines of two sorted files in three columns: only in the
first, only in the second, in both (`-1`, `-2`, `-3` drop a column). For
word lists, Python's sets do it: `$a - $b`, `$b - $a`, `$a & $b`.

`basename` and `dirname` take a path apart. They work on any string, not
only on files that exist.

### Example 16-36. `basename` and `dirname` (`ex35.sh`)

```bash no-run
#!/bin/bash

address=/home/bozo/daily-journal.txt

echo "Basename of /home/bozo/daily-journal.txt = `basename $address`"
echo "Dirname of /home/bozo/daily-journal.txt = `dirname $address`"
echo
echo "My own home is `basename ~/`."         # `basename ~` also works.
echo "The home of my home is `dirname ~/`."  # `dirname ~`  also works.

exit 0
```

A path written bare is a Path, so `.name` and `.parent` are `basename` and
`dirname`. (The bash prints your own home, so it isn't run here; the tessh
version sets `HOME` to bozo's first, as `~` follows `$env.HOME`.)

```tessh
#!/usr/bin/env tessh
$env.HOME = /home/bozo          # (So the output doesn't show yours.)

address = /home/bozo/daily-journal.txt

print "Basename of /home/bozo/daily-journal.txt = $address.name"
print "Dirname of /home/bozo/daily-journal.txt = $address.parent"
print
home = Path ~/
print "My own home is $home.name."
print "The home of my home is $home.parent."
exit 0
```

```output
Basename of /home/bozo/daily-journal.txt = daily-journal.txt
Dirname of /home/bozo/daily-journal.txt = /home/bozo

My own home is bozo.
The home of my home is /home.
```

`split` cuts a file into pieces of a size, and `csplit` at lines or
patterns.

### Example 16-37. A script that copies itself in sections (`splitcopy.sh`)

```bash no-run
#!/bin/bash
# splitcopy.sh

#  A script that splits itself into chunks,
#+ then reassembles the chunks into an exact copy
#+ of the original script.

CHUNKSIZE=4    #  Size of first chunk of split files.
OUTPREFIX=xx   #  csplit prefixes, by default,
               #+ files with "xx" ...

csplit "$0" "$CHUNKSIZE"

# Some comment lines for padding . . .
# Line 15
# Line 16
# Line 17
# Line 18
# Line 19
# Line 20

cat "$OUTPREFIX"* > "$0.copy"  # Concatenate the chunks.
rm "$OUTPREFIX"*               # Get rid of the chunks.

exit $?
```

The copy goes in the current folder (the bash writes it next to the
script). `$OUTPREFIX*` glued together is a Glob, so `cat` gets the pieces
in order, and `rm` too. `csplit` prints the size of each piece, and `cmp`
shows the copy is exact.

```tessh
#!/usr/bin/env tessh
# splitcopy.tsh
#  A script that splits itself into chunks,
#+ then reassembles the chunks into an exact copy
#+ of the original script.

CHUNKSIZE = 4    #  Size of first chunk of split files.
OUTPREFIX = xx   #  csplit prefixes, by default, files with "xx" ...

csplit $argv[0] $CHUNKSIZE

# Some comment lines for padding . . .
# Line 15
# Line 16

copy = (Path $argv[0]).name + .copy
cat $OUTPREFIX* > $copy          # Concatenate the chunks.
rm $OUTPREFIX*                   # Get rid of the chunks.
cmp $argv[0] $copy && print "$copy is an exact copy."

exit $?
```

```output
81
508
ex.tsh.copy is an exact copy.
```

`md5sum` and `sha1sum` (better, `sha256sum`) print a checksum of each
file's contents; a list of them shows later whether any file changed.
`cksum` adds the size, and a dash stands for stdin.

### Example 16-38. Checking file integrity (`file-integrity.sh`)

```bash no-run
#!/bin/bash
# file-integrity.sh: Checking whether files in a given directory
#                    have been tampered with.

E_DIR_NOMATCH=80
E_BAD_DBFILE=81

dbfile=File_record.md5
# Filename for storing records (database file).


set_up_database ()
{
  echo ""$directory"" > "$dbfile"
  # Write directory name to first line of file.
  md5sum "$directory"/* >> "$dbfile"
  # Append md5 checksums and filenames.
}

check_database ()
{
  local n=0
  local filename
  local checksum

  # ------------------------------------------- #
  #  This file check should be unnecessary,
  #+ but better safe than sorry.

  if [ ! -r "$dbfile" ]
  then
    echo "Unable to read checksum database file!"
    exit $E_BAD_DBFILE
  fi
  # ------------------------------------------- #

  while read record[n]
  do

    directory_checked="${record[0]}"
    if [ "$directory_checked" != "$directory" ]
    then
      echo "Directories do not match up!"
      # Tried to use file for a different directory.
      exit $E_DIR_NOMATCH
    fi

    if [ "$n" -gt 0 ]   # Not directory name.
    then
      filename[n]=$( echo ${record[$n]} | awk '{ print $2 }' )
      #  md5sum writes records backwards,
      #+ checksum first, then filename.
      checksum[n]=$( md5sum "${filename[n]}" )


      if [ "${record[n]}" = "${checksum[n]}" ]
      then
        echo "${filename[n]} unchanged."

        elif [ "`basename ${filename[n]}`" != "$dbfile" ]
               #  Skip over checksum database file,
               #+ as it will change with each invocation of script.
               #  ---
               #  This unfortunately means that when running
               #+ this script on $PWD, tampering with the
               #+ checksum database file will not be detected.
               #  Exercise: Fix this.
        then
          echo "${filename[n]} : CHECKSUM ERROR!"
        # File has been changed since last checked.
        fi

      fi



    let "n+=1"
  done <"$dbfile"       # Read from checksum database file. 

}  

# =================================================== #
# main ()

if [ -z  "$1" ]
then
  directory="$PWD"      #  If not specified,
else                    #+ use current working directory.
  directory="$1"
fi  

clear                   # Clear screen.
echo " Running file integrity check on $directory"
echo

# ------------------------------------------------------------------ #
  if [ ! -r "$dbfile" ] # Need to create database file?
  then
    echo "Setting up database file, \""$directory"/"$dbfile"\"."; echo
    set_up_database
  fi  
# ------------------------------------------------------------------ #

check_database          # Do the actual work.

echo 

#  You may wish to redirect the stdout of this script to a file,
#+ especially if the directory checked has many files in it.

exit 0

#  For a much more thorough file integrity check,
#+ consider the "Tripwire" package,
#+ http://sourceforge.net/projects/tripwire/.
```

The record is the same file: the folder's name on the first line, then
`md5sum`'s lines. Reading it is `(cat $dbfile).lines`, each record split
into checksum and name with `.split`, where bash reads it into an array
with `read record[n]` and cuts the name out with `awk`. `clear` is left out
(stdin and stdout aren't a terminal here). It runs twice, the second time
after one file has been changed.

```tessh
#!/usr/bin/env tessh
# (A made-up folder to watch.)
mkdir data ; echo alpha > data/a ; echo beta > data/b ; echo gamma > data/c

E_DIR_NOMATCH = 80
E_BAD_DBFILE = 81

dbfile = File_record.md5
# Filename for storing records (database file).

def set_up_database directory
  print $directory >$dbfile        # Directory name on the first line,
  md5sum $directory/* >>$dbfile    # then checksums and filenames.

def check_database directory
  if not ((Path $dbfile).is_file)
    print "Unable to read checksum database file!"
    exit $E_BAD_DBFILE

  [directory_checked *records] = (cat $dbfile).lines
  if $directory_checked != $directory
    print "Directories do not match up!"
    exit $E_DIR_NOMATCH

  for record in $records
    [checksum filename] = $record.split "  " 1
    #  md5sum writes records backwards, checksum first, then filename.
    if (md5sum $filename) == $record
      print "$filename unchanged."
    elif (Path $filename).name != $dbfile
      print "$filename : CHECKSUM ERROR!"
      # File has been changed since last checked.

# main
def main args
  directory = if $args then $args[0] else "."   # Default: the current folder.
  print " Running file integrity check on $directory"
  print
  if not ((Path $dbfile).is_file)   # Need to create database file?
    print "Setting up database file, \"$directory/$dbfile\"."
    print
    set_up_database $directory
  check_database $directory
  print

main ($argv[1:] or [data])     # (sample argument, when none is given)
echo tampered > data/b
main ($argv[1:] or [data])
exit 0
```

```output
 Running file integrity check on data

Setting up database file, "data/File_record.md5".

data/a unchanged.
data/b unchanged.
data/c unchanged.

 Running file integrity check on data

data/a unchanged.
data/b : CHECKSUM ERROR!
data/c unchanged.

```

`uuencode` turns a binary file into lines of text for mail and news, and
`uudecode` turns it back.

### Example 16-39. Uudecoding encoded files (`ex52.sh`)

```bash no-run
#!/bin/bash
# Uudecodes all uuencoded files in current working directory.

lines=35        # Allow 35 lines for the header (very generous).

for File in *   # Test all the files in $PWD.
do
  search1=`head -n $lines $File | grep begin | wc -w`
  search2=`tail -n $lines $File | grep end | wc -w`
  #  Uuencoded files have a "begin" near the beginning,
  #+ and an "end" near the end.
  if [ "$search1" -gt 0 ]
  then
    if [ "$search2" -gt 0 ]
    then
      echo "uudecoding - $File -"
      uudecode $File
    fi  
  fi
done  

#  Note that running this script upon itself fools it
#+ into thinking it is a uuencoded file,
#+ because it contains both "begin" and "end".

#  Exercise:
#  --------
#  Modify this script to check each file for a newsgroup header,
#+ and skip to next if not found.

exit 0
```

`uudecode` isn't installed here (and Python 3.13 dropped its `uu`
module), but `binascii.a2b_uu` decodes a line, so a few lines of Python do
the job, and make the made-up encoded file to try it on. The test for
`begin` and `end` is a regex on the first and last lines rather than
counting `grep`'s words.

```tessh
#!/usr/bin/env tessh
import binascii
# (A made-up uuencoded file, made with binascii.b2a_uu, and a plain one.)
data = b"Hello from a uuencoded file.\n"
with (open hello.uue w) as f
  print "begin 644 hello.txt" file=$f
  print (($binascii.b2a_uu $data).decode) end="" file=$f
  print "`\nend" file=$f
print "Just text." >plain.txt

def uudecode path
  # Decode a uuencoded file into the file its "begin" line names.
  lines = (cat $path).lines
  [_ _ name] = $lines[0].split " " 2
  body = [($binascii.a2b_uu $l) for l in $lines[1:] if $l not in ["`" end]]
  (Path $name).write_bytes (b"".join $body)

lines = 35        # Allow 35 lines for the header (very generous).

for File in *     # Test all the files in the folder.
  text = (cat $File).lines
  #  Uuencoded files have a "begin" near the beginning,
  #+ and an "end" near the end.
  if (any ($l =~ re'^begin ' for l in $text[:$lines])) and \
     (any ($l == end for l in $text[- $lines:]))
    print "uudecoding - $File -"
    uudecode $File

cat hello.txt
exit 0
```

```output
uudecoding - hello.uue -
Hello from a uuencoded file.
```

`mimencode` handles MIME attachments; Python's `email` and `base64`
modules do it now. `crypt` is long gone; `openssl` encrypts (`openssl
enc -aes-256-cbc -pbkdf2 -in file -out file.enc`), and piping `tar` into it
encrypts a whole tree. `shred` overwrites a file before deleting it.

`mktemp` makes a temporary file with a unique name, and its value in tessh
is the name, so there's no backquote. Python's `tempfile` does the same
and can remove the file when done.

```bash no-run
PREFIX=filename
tempfile=`mktemp $PREFIX.XXXXXX`
#                        ^^^^^^ Need at least 6 placeholders
#+                              in the filename template.
#   If no filename template supplied,
#+ "tmp.XXXXXXXXXX" is the default.

echo "tempfile name = $tempfile"
# tempfile name = filename.QA2ZpY
#                 or something similar...

#  Creates a file of that name in the current working directory
#+ with 600 file permissions.
#  A "umask 177" is therefore unnecessary,
#+ but it's good programming practice nevertheless.
```

```tessh
from tempfile import mkstemp
PREFIX = filename
tempfile = mktemp ${PREFIX}.XXXXXX
print "tempfile name = $tempfile"
[fd name] = mkstemp prefix="$PREFIX." dir="."
print "and from Python: $((Path $name).name)"
```

```text
(one run)
tempfile name = filename.JfOgtz
and from Python: filename.p2i7vemn
```

`make` builds what a Makefile says from what changed, `install` copies with
permissions, `dos2unix` converts line ends, `ptx` makes a permuted index,
and `more` and `less` page through text. The ABS Guide's trick of piping
commands to `more` to look at them before piping them to `sh`:

```bash no-run
ls /home/bozo | awk '{print "rm -rf " $1}' | more
#                                            ^^^^
# Testing the effect of the following (disastrous) command-line:
#      ls /home/bozo | awk '{print "rm -rf " $1}' | sh
#      Hand off to the shell to execute . . .       ^^
```

In tessh the commands are a list first, to look at, and then to run: no
quoting goes wrong on the way to `sh`.

```tessh
mkdir junk ; touch junk/old1 junk/old2   # (made up)
commands = [[rm -rf $f] for f in junk/*]
for c in $commands
  print *$c                              # Look first . . .
# for c in $commands                     # . . . then run them:
#   command *$c
```

```output
rm -rf junk/old1
rm -rf junk/old2
```

## 16.6 Communications Commands
<!-- help: abs-communications host ipcalc nslookup dig traceroute ping whois finger ftp uucp telnet wget lynx rsync ssh scp write mail -->

**Information:** `host`, `ipcalc -h`, `nslookup` and `dig` look names and
addresses up in DNS (`dig -x` the other way), `traceroute` shows the way
packets go, `ping` checks that a host answers, `whois` asks who owns a
domain, `finger` and `chfn` show and change user information, `vrfy` checks
a mail address. **Remote hosts:** `sx`/`rx` and `sz`/`rz` (xmodem and
zmodem), `ftp`, `uucp`/`uux`/`cu`, `telnet`, `wget` and `lynx -dump` (fetch
web pages), `rlogin`, `rsh`, `rcp`, `rsync`, and `ssh`/`scp`, which replace
the insecure ones. **Local network:** `write`, `netconfig`. **Mail:**
`mail`, `mailto`, `mailstats`, `vacation`.

These need a network (or a mail system), so this section's examples don't
run here: their tessh versions are checked by `tessh --check` but not run.
What tessh changes is, again, the glue: a program's output is text to take
apart with `.split`, `.lines` and regexes, an exit status is an `if`
condition, and options built up in a list stay one argument each, with no
`eval`. Python's `socket`, `urllib.request`, `smtplib` and `subprocess`
can stand in for several of the programs.

`ping` exits 0 when the host answers. The ABS Guide tests whether its
output is empty; testing its exit status says the same more directly:

```bash no-run
HNAME=news-15.net  # Notorious spammer.
# HNAME=$HOST     # Debug: test for localhost.
  count=2  # Send only two pings.

if [[ `ping -c $count "$HNAME"` ]]
then
  echo ""$HNAME" still up and broadcasting spam your way."
else
  echo ""$HNAME" seems to be down. Pity."
fi
```

```tessh no-run
HNAME = news-15.net  # Notorious spammer.
count = 2            # Send only two pings.

if ping -c $count $HNAME >/dev/null
  print "$HNAME still up and broadcasting spam your way."
else
  print "$HNAME seems to be down. Pity."
```

### Example 16-40. Finding out where to report a spammer (`spam-lookup.sh`)

```bash no-run
#!/bin/bash
# spam-lookup.sh: Look up abuse contact to report a spammer.
# Thanks, Michael Zick.

# Check for command-line arg.
ARGCOUNT=1
E_WRONGARGS=85
if [ $# -ne "$ARGCOUNT" ]
then
  echo "Usage: `basename $0` domain-name"
  exit $E_WRONGARGS
fi


dig +short $1.contacts.abuse.net -c in -t txt
# Also try:
#     dig +nssearch $1
#     Tries to find "authoritative name servers" and display SOA records.

# The following also works:
#     whois -h whois.abuse.net $1
#           ^^ ^^^^^^^^^^^^^^^  Specify host.  
#     Can even lookup multiple spammers with this, i.e."
#     whois -h whois.abuse.net $spamdomain1 $spamdomain2 . . .


#  Exercise:
#  --------
#  Expand the functionality of this script
#+ so that it automatically e-mails a notification
#+ to the responsible ISP's contact address(es).
#  Hint: use the "mail" command.

exit $?

# spam-lookup.sh chinatietong.com
#                A known spam domain.

# "crnet_mgr@chinatietong.com"
# "crnet_tec@chinatietong.com"
# "postmaster@chinatietong.com"


#  For a more elaborate version of this script,
#+ see the SpamViz home page, http://www.spamviz.net/index.html.
```

It asks DNS, so it doesn't run here. `dig +short` prints one quoted address
per line; captured, `.lines` is the list, and the quotes come off with
`.strip`.

```tessh no-run
#!/usr/bin/env tessh
# spam-lookup.tsh: Look up abuse contact to report a spammer.

ARGCOUNT = 1
E_WRONGARGS = 85
if (len $argv) - 1 != $ARGCOUNT
  print "Usage: $(Path $argv[0]).name domain-name"
  exit $E_WRONGARGS
domain = $argv[1]

contacts = dig +short "$domain.contacts.abuse.net" -c 'in' -t txt
for line in $contacts.lines
  print ($line.strip '"')
# Also try:
#     whois -h whois.abuse.net $domain

exit $contacts.code
```

### Example 16-41. Analyzing a spam domain (`is-spammer.sh`)

```bash no-run
#! /bin/bash
# is-spammer.sh: Identifying spam domains

# $Id: is-spammer, v 1.4 2004/09/01 19:37:52 mszick Exp $
# Above line is RCS ID info.
#
#  This is a simplified version of the "is_spammer.bash
#+ script in the Contributed Scripts appendix.

# is-spammer <domain.name>

# Uses an external program: 'dig'
# Tested with version: 9.2.4rc5

# Uses functions.
# Uses IFS to parse strings by assignment into arrays.
# And even does something useful: checks e-mail blacklists.

# Use the domain.name(s) from the text body:
# http://www.good_stuff.spammer.biz/just_ignore_everything_else
#                       ^^^^^^^^^^^
# Or the domain.name(s) from any e-mail address:
# Really_Good_Offer@spammer.biz
#
# as the only argument to this script.
#(PS: have your Inet connection running)
#
# So, to invoke this script in the above two instances:
#       is-spammer.sh spammer.biz


# Whitespace == :Space:Tab:Line Feed:Carriage Return:
WSP_IFS=$'\x20'$'\x09'$'\x0A'$'\x0D'

# No Whitespace == Line Feed:Carriage Return
No_WSP=$'\x0A'$'\x0D'

# Field separator for dotted decimal ip addresses
ADR_IFS=${No_WSP}'.'

# Get the dns text resource record.
# get_txt <error_code> <list_query>
get_txt() {

    # Parse $1 by assignment at the dots.
    local -a dns
    IFS=$ADR_IFS
    dns=( $1 )
    IFS=$WSP_IFS
    if [ "${dns[0]}" == '127' ]
    then
        # See if there is a reason.
        echo $(dig +short $2 -t txt)
    fi
}

# Get the dns address resource record.
# chk_adr <rev_dns> <list_server>
chk_adr() {
    local reply
    local server
    local reason

    server=${1}${2}
    reply=$( dig +short ${server} )

    # If reply might be an error code . . .
    if [ ${#reply} -gt 6 ]
    then
        reason=$(get_txt ${reply} ${server} )
        reason=${reason:-${reply}}
    fi
    echo ${reason:-' not blacklisted.'}
}

# Need to get the IP address from the name.
echo 'Get address of: '$1
ip_adr=$(dig +short $1)
dns_reply=${ip_adr:-' no answer '}
echo ' Found address: '${dns_reply}

# A valid reply is at least 4 digits plus 3 dots.
if [ ${#ip_adr} -gt 6 ]
then
    echo
    declare query

    # Parse by assignment at the dots.
    declare -a dns
    IFS=$ADR_IFS
    dns=( ${ip_adr} )
    IFS=$WSP_IFS

    # Reorder octets into dns query order.
    rev_dns="${dns[3]}"'.'"${dns[2]}"'.'"${dns[1]}"'.'"${dns[0]}"'.'

# See: http://www.spamhaus.org (Conservative, well maintained)
    echo -n 'spamhaus.org says: '
    echo $(chk_adr ${rev_dns} 'sbl-xbl.spamhaus.org')

# See: http://ordb.org (Open mail relays)
    echo -n '   ordb.org  says: '
    echo $(chk_adr ${rev_dns} 'relays.ordb.org')

# See: http://www.spamcop.net/ (You can report spammers here)
    echo -n ' spamcop.net says: '
    echo $(chk_adr ${rev_dns} 'bl.spamcop.net')

# # # other blacklist operations # # #

# See: http://cbl.abuseat.org.
    echo -n ' abuseat.org says: '
    echo $(chk_adr ${rev_dns} 'cbl.abuseat.org')

# See: http://dsbl.org/usage (Various mail relays)
    echo
    echo 'Distributed Server Listings'
    echo -n '       list.dsbl.org says: '
    echo $(chk_adr ${rev_dns} 'list.dsbl.org')

    echo -n '   multihop.dsbl.org says: '
    echo $(chk_adr ${rev_dns} 'multihop.dsbl.org')

    echo -n 'unconfirmed.dsbl.org says: '
    echo $(chk_adr ${rev_dns} 'unconfirmed.dsbl.org')

else
    echo
    echo 'Could not use that address.'
fi

exit 0

# Exercises:
# --------

# 1) Check arguments to script,
#    and exit with appropriate error message if necessary.

# 2) Check if on-line at invocation of script,
#    and exit with appropriate error message if necessary.

# 3) Substitute generic variables for "hard-coded" BHL domains.

# 4) Set a time-out for the script using the "+time=" option
     to the 'dig' command.
```

Michael Zick's script parses addresses by setting `IFS` to a dot and
assigning to an array. In tessh an address is split with `.split "."`, and
reversing the octets for the blacklist query is `reversed`. The blacklists
are a list (exercise 3), the `dig` time-out is an option (exercise 4), and
the argument is checked (exercise 1). It needs DNS, so it doesn't run here.

```tessh no-run
#!/usr/bin/env tessh
# is-spammer.tsh: Identifying spam domains
# After Michael Zick's is-spammer.sh.
# Uses an external program: 'dig'

E_WRONGARGS = 85
DIG_OPTS = [+short "+time=5"]       # (Exercise 4: a time-out.)

# Exercise 3: the blacklists as data.
BLACKLISTS = [
  ["spamhaus.org" "sbl-xbl.spamhaus.org"]
  ["ordb.org" "relays.ordb.org"]
  ["spamcop.net" "bl.spamcop.net"]
  ["abuseat.org" "cbl.abuseat.org"]
  ["list.dsbl.org" "list.dsbl.org"]
  ["multihop.dsbl.org" "multihop.dsbl.org"]
  ["unconfirmed.dsbl.org" "unconfirmed.dsbl.org"]
]

# The dns text resource record: a reason, if the reply is an error code.
def get_txt error_code list_query
  if ($error_code.split ".")[0] == "127"
    " ".join (dig *$DIG_OPTS $list_query -t txt).lines

# Look the reversed address up in one blacklist.
def chk_adr rev_dns list_server
  server = "$rev_dns$list_server"
  reply = dig *$DIG_OPTS $server
  reason = None
  if (len $reply) > 6                # If reply might be an error code . . .
    reason = (get_txt $reply $server) or $reply
  $reason or " not blacklisted."

if (len $argv) != 2                  # (Exercise 1.)
  print "Usage: $(Path $argv[0]).name domain.name"
  exit $E_WRONGARGS

# Need to get the IP address from the name.
print "Get address of: $argv[1]"
ip_adr = dig *$DIG_OPTS $argv[1]
print " Found address: $(if $ip_adr then $ip_adr else ' no answer ')"

# A valid reply is at least 4 digits plus 3 dots.
if (len $ip_adr) > 6
  print
  # Reorder octets into dns query order.
  rev_dns = ".".join (reversed ($ip_adr.split ".")) + "."
  for [name server] in $BLACKLISTS
    print "${name:>20} says: $(chk_adr $rev_dns $server)"
else
  print
  print "Could not use that address."

exit 0
```

`wget` and `lynx -dump` fetch web pages from a script; `wget -c` resumes
an interrupted download.

### Example 16-42. Getting a stock quote (`quote-fetch.sh`)

```bash no-run
#!/bin/bash
# quote-fetch.sh: Download a stock quote.


E_NOPARAMS=86

if [ -z "$1" ]  # Must specify a stock (symbol) to fetch.
  then echo "Usage: `basename $0` stock-symbol"
  exit $E_NOPARAMS
fi

stock_symbol=$1

file_suffix=.html
# Fetches an HTML file, so name it appropriately.
URL='http://finance.yahoo.com/q?s='
# Yahoo finance board, with stock query suffix.

# -----------------------------------------------------------
wget -O ${stock_symbol}${file_suffix} "${URL}${stock_symbol}"
# -----------------------------------------------------------


# To look up stuff on http://search.yahoo.com:
# -----------------------------------------------------------
# URL="http://search.yahoo.com/search?fr=ush-news&p=${query}"
# wget -O "$savefilename" "${URL}"
# -----------------------------------------------------------
# Saves a list of relevant URLs.

exit $?

# Exercises:
# ---------
#
# 1) Add a test to ensure the user running the script is on-line.
#    (Hint: parse the output of 'ps -ax' for "ppp" or "connect."
#
# 2) Modify this script to fetch the local weather report,
#+   taking the user's zip code as an argument.
```

The same `wget`, with the URL and file name built in f-string fashion (and
Yahoo's old URL long gone). `urllib.request.urlretrieve` would do it without
`wget`. It needs the network.

```tessh no-run
#!/usr/bin/env tessh
# quote-fetch.tsh: Download a stock quote.

E_NOPARAMS = 86

if not $argv[1:]       # Must specify a stock (symbol) to fetch.
  print "Usage: $(Path $argv[0]).name stock-symbol"
  exit $E_NOPARAMS

stock_symbol = $argv[1]
file_suffix = .html    # Fetches an HTML file, so name it appropriately.
URL = 'http://finance.yahoo.com/q?s='

# -----------------------------------------------------------
wget -O "$stock_symbol$file_suffix" "$URL$stock_symbol"
# -----------------------------------------------------------

exit $?
```

`rsync` brings a copy of a tree up to date, sending only what changed.

### Example 16-43. Updating FC4 (`fc4upd.sh`)

```bash no-run
#!/bin/bash
# fc4upd.sh

# Script author: Frank Wang.
# Slight stylistic modifications by ABS Guide author.
# Used in ABS Guide with permission.


#  Download Fedora Core 4 update from mirror site using rsync. 
#  Should also work for newer Fedora Cores -- 5, 6, . . .
#  Only download latest package if multiple versions exist,
#+ to save space.

URL=rsync://distro.ibiblio.org/fedora-linux-core/updates/
# URL=rsync://ftp.kddilabs.jp/fedora/core/updates/
# URL=rsync://rsync.planetmirror.com/fedora-linux-core/updates/

DEST=${1:-/var/www/html/fedora/updates/}
LOG=/tmp/repo-update-$(/bin/date +%Y-%m-%d).txt
PID_FILE=/var/run/${0##*/}.pid

E_RETURN=85        # Something unexpected happened.


# General rsync options
# -r: recursive download
# -t: reserve time
# -v: verbose

OPTS="-rtv --delete-excluded --delete-after --partial"

# rsync include pattern
# Leading slash causes absolute path name match.
INCLUDE=(
    "/4/i386/kde-i18n-Chinese*" 
#   ^                         ^
# Quoting is necessary to prevent globbing.
) 


# rsync exclude pattern
# Temporarily comment out unwanted pkgs using "#" . . .
EXCLUDE=(
    /1
    /2
    /3
    /testing
    /4/SRPMS
    /4/ppc
    /4/x86_64
    /4/i386/debug
   "/4/i386/kde-i18n-*"
   "/4/i386/openoffice.org-langpack-*"
   "/4/i386/*i586.rpm"
   "/4/i386/GFS-*"
   "/4/i386/cman-*"
   "/4/i386/dlm-*"
   "/4/i386/gnbd-*"
   "/4/i386/kernel-smp*"
#  "/4/i386/kernel-xen*" 
#  "/4/i386/xen-*" 
)


init () {
    # Let pipe command return possible rsync error, e.g., stalled network.
    set -o pipefail                  # Newly introduced in Bash, version 3.

    TMP=${TMPDIR:-/tmp}/${0##*/}.$$  # Store refined download list.
    trap "{
        rm -f $TMP 2>/dev/null
    }" EXIT                          # Clear temporary file on exit.
}


check_pid () {
# Check if process exists.
    if [ -s "$PID_FILE" ]; then
        echo "PID file exists. Checking ..."
        PID=$(/bin/egrep -o "^[[:digit:]]+" $PID_FILE)
        if /bin/ps --pid $PID &>/dev/null; then
            echo "Process $PID found. ${0##*/} seems to be running!"
           /usr/bin/logger -t ${0##*/} \
                 "Process $PID found. ${0##*/} seems to be running!"
            exit $E_RETURN
        fi
        echo "Process $PID not found. Start new process . . ."
    fi
}


#  Set overall file update range starting from root or $URL,
#+ according to above patterns.
set_range () {
    include=
    exclude=
    for p in "${INCLUDE[@]}"; do
        include="$include --include \"$p\""
    done

    for p in "${EXCLUDE[@]}"; do
        exclude="$exclude --exclude \"$p\""
    done
}


# Retrieve and refine rsync update list.
get_list () {
    echo $$ > $PID_FILE || {
        echo "Can't write to pid file $PID_FILE"
        exit $E_RETURN
    }

    echo -n "Retrieving and refining update list . . ."

    # Retrieve list -- 'eval' is needed to run rsync as a single command.
    # $3 and $4 is the date and time of file creation.
    # $5 is the full package name.
    previous=
    pre_file=
    pre_date=0
    eval /bin/nice /usr/bin/rsync \
        -r $include $exclude $URL | \
        egrep '^dr.x|^-r' | \
        awk '{print $3, $4, $5}' | \
        sort -k3 | \
        { while read line; do
            # Get seconds since epoch, to filter out obsolete pkgs.
            cur_date=$(date -d "$(echo $line | awk '{print $1, $2}')" +%s)
            #  echo $cur_date

            # Get file name.
            cur_file=$(echo $line | awk '{print $3}')
            #  echo $cur_file

            # Get rpm pkg name from file name, if possible.
            if [[ $cur_file == *rpm ]]; then
                pkg_name=$(echo $cur_file | sed -r -e \
                    's/(^([^_-]+[_-])+)[[:digit:]]+\..*[_-].*$/\1/')
            else
                pkg_name=
            fi
            # echo $pkg_name

            if [ -z "$pkg_name" ]; then   #  If not a rpm file,
                echo $cur_file >> $TMP    #+ then append to download list.
            elif [ "$pkg_name" != "$previous" ]; then   # A new pkg found.
                echo $pre_file >> $TMP                  # Output latest file.
                previous=$pkg_name                      # Save current.
                pre_date=$cur_date
                pre_file=$cur_file
            elif [ "$cur_date" -gt "$pre_date" ]; then
                                                #  If same pkg, but newer,
                pre_date=$cur_date              #+ then update latest pointer.
                pre_file=$cur_file
            fi
            done
            echo $pre_file >> $TMP              #  TMP contains ALL
                                                #+ of refined list now.
            # echo "subshell=$BASH_SUBSHELL"

    }       # Bracket required here to let final "echo $pre_file >> $TMP" 
            # Remained in the same subshell ( 1 ) with the entire loop.

    RET=$?  # Get return code of the pipe command.

    [ "$RET" -ne 0 ] && {
        echo "List retrieving failed with code $RET"
        exit $E_RETURN
    }

    echo "done"; echo
}

# Real rsync download part.
get_file () {

    echo "Downloading..."
    /bin/nice /usr/bin/rsync \
        $OPTS \
        --filter "merge,+/ $TMP" \
        --exclude '*'  \
        $URL $DEST     \
        | /usr/bin/tee $LOG

    RET=$?

   #  --filter merge,+/ is crucial for the intention. 
   #  + modifier means include and / means absolute path.
   #  Then sorted list in $TMP will contain ascending dir name and 
   #+ prevent the following --exclude '*' from "shortcutting the circuit." 

    echo "Done"

    rm -f $PID_FILE 2>/dev/null

    return $RET
}

# -------
# Main
init
check_pid
set_range
get_list
get_file
RET=$?
# -------

if [ "$RET" -eq 0 ]; then
    /usr/bin/logger -t ${0##*/} "Fedora update mirrored successfully."
else
    /usr/bin/logger -t ${0##*/} \
    "Fedora update mirrored with failure code: $RET"
fi

exit $RET
```

Frank Wang's script builds its `--include`/`--exclude` options as one
string with embedded quotes, which then needs `eval`; in tessh they're a
list, one argument per item, and the patterns are never globbed because
they're quoted text. The listing is captured, its exit status checked (no
`pipefail` needed), and refined in Python: `.split` instead of `awk`, a
regex instead of `sed`, `datetime.strptime` instead of `date -d`, and a
dict of the newest file per package instead of a sorted pass with
`previous` and `pre_date`. `try`/`finally` removes the temporary and PID
files, as bash's `trap … EXIT` does. It needs the mirror, so it doesn't run
here.

```tessh no-run
#!/usr/bin/env tessh
# fc4upd.tsh
# After Frank Wang's fc4upd.sh.
#  Download Fedora Core 4 update from mirror site using rsync.
#  Only download latest package if multiple versions exist, to save space.
import os tempfile
from datetime import datetime

URL = rsync://distro.ibiblio.org/fedora-linux-core/updates/
ME = (Path $argv[0]).name
DEST = if $argv[1:] then $argv[1] else /var/www/html/fedora/updates/
LOG = Path "/tmp/repo-update-$(((datetime.now).date)).txt"
PID_FILE = Path "/var/run/$ME.pid"

E_RETURN = 85        # Something unexpected happened.

# General rsync options: recursive, keep times, verbose . . .
OPTS = [-rtv --delete-excluded --delete-after --partial]

# rsync include and exclude patterns (a leading slash anchors the path).
INCLUDE = ["/4/i386/kde-i18n-Chinese*"]
EXCLUDE = [
  /1 /2 /3 /testing /4/SRPMS /4/ppc /4/x86_64 /4/i386/debug
  "/4/i386/kde-i18n-*"
  "/4/i386/openoffice.org-langpack-*"
  "/4/i386/*i586.rpm"
  "/4/i386/GFS-*" "/4/i386/cman-*" "/4/i386/dlm-*" "/4/i386/gnbd-*"
  "/4/i386/kernel-smp*"
]

# Check if process exists.
def check_pid
  if ($PID_FILE.exists) and ($PID_FILE.stat).st_size > 0
    print "PID file exists. Checking ..."
    PID = (($PID_FILE.read_text).split)[0]
    if ps --pid $PID &>/dev/null
      print "Process $PID found. $ME seems to be running!"
      logger -t $ME "Process $PID found. $ME seems to be running!"
      exit $E_RETURN
    print "Process $PID not found. Start new process . . ."

# Set overall file update range, according to above patterns:
# the rsync options, one argument each (no quotes inside, no eval).
def set_range
  opts = []
  for p in $INCLUDE
    $opts.extend [--include $p]
  for p in $EXCLUDE
    $opts.extend [--exclude $p]
  $opts

# Retrieve and refine rsync update list: the newest file of each package.
def get_list tmp
  try
    $PID_FILE.write_text "$(os.getpid)\n"
  except OSError
    print "Can't write to pid file $PID_FILE"
    exit $E_RETURN

  print "Retrieving and refining update list . . ." end=""
  listing = nice rsync -r *(set_range) $URL
  if $listing.code != 0
    print "List retrieving failed with code $listing.code"
    exit $E_RETURN

  newest = {}      # package name -> [date file]
  others = []      # files that aren't rpms
  for line in $listing.lines
    if $line !~ re'^dr.x|^-r'
      continue
    [_ _ day time cur_file] = ($line.split)
    cur_date = datetime.strptime "$day $time" "%Y/%m/%d %H:%M:%S"
    # Get rpm pkg name from file name, if possible.
    m = $cur_file =~ re'^((?:[^_-]+[_-])+)\d+\..*[_-].*\.rpm$'
    if not $m
      $others.append $cur_file
    elif ($m.group 1) not in $newest or $cur_date > $newest[($m.group 1)][0]
      $newest[($m.group 1)] = [$cur_date $cur_file]
  files = sorted ($others + [$f for [_ f] in ($newest.values)])
  (Path $tmp).write_text (("\n".join $files) + "\n")
  print "done" ; print

# Real rsync download part.
def get_file tmp
  print "Downloading..."
  nice rsync *$OPTS --filter "merge,+/ $tmp" --exclude '*' $URL $DEST | tee $LOG
  RET = $?
  print "Done"
  $PID_FILE.unlink missing_ok=True
  $RET

# -------
# Main
[fd tmp] = tempfile.mkstemp prefix="$ME."
try
  check_pid
  get_list $tmp
  RET = get_file $tmp
finally
  os.unlink $tmp        # Clear temporary file on exit.
# -------

if $RET == 0
  logger -t $ME "Fedora update mirrored successfully."
else
  logger -t $ME "Fedora update mirrored with failure code: $RET"

exit $RET
```

`ssh` runs a command on another machine, `scp` copies files to and from it.
In a loop, `ssh` reads the loop's stdin unless given `-n`.

### Example 16-44. Using `ssh` (`remote.bash`)

```bash no-run
#!/bin/bash
# remote.bash: Using ssh.

# This example by Michael Zick.
# Used with permission.


#   Presumptions:
#   ------------
#   fd-2 isn't being captured ( '2>/dev/null' ).
#   ssh/sshd presumes stderr ('2') will display to user.
#
#   sshd is running on your machine.
#   For any 'standard' distribution, it probably is,
#+  and without any funky ssh-keygen having been done.

# Try ssh to your machine from the command-line:
#
# $ ssh $HOSTNAME
# Without extra set-up you'll be asked for your password.
#   enter password
#   when done,  $ exit
#
# Did that work? If so, you're ready for more fun.

# Try ssh to your machine as 'root':
#
#   $  ssh -l root $HOSTNAME
#   When asked for password, enter root's, not yours.
#          Last login: Tue Aug 10 20:25:49 2004 from localhost.localdomain
#   Enter 'exit' when done.

#  The above gives you an interactive shell.
#  It is possible for sshd to be set up in a 'single command' mode,
#+ but that is beyond the scope of this example.
#  The only thing to note is that the following will work in
#+ 'single command' mode.


# A basic, write stdout (local) command.

ls -l

# Now the same basic command on a remote machine.
# Pass a different 'USERNAME' 'HOSTNAME' if desired:
USER=${USERNAME:-$(whoami)}
HOST=${HOSTNAME:-$(hostname)}

#  Now excute the above command-line on the remote host,
#+ with all transmissions encrypted.

ssh -l ${USER} ${HOST} " ls -l "

#  The expected result is a listing of your username's home
#+ directory on the remote machine.
#  To see any difference, run this script from somewhere
#+ other than your home directory.

#  In other words, the Bash command is passed as a quoted line
#+ to the remote shell, which executes it on the remote machine.
#  In this case, sshd does  ' bash -c "ls -l" '   on your behalf.

#  For information on topics such as not having to enter a
#+ password/passphrase for every command-line, see
#+    man ssh
#+    man ssh-keygen
#+    man sshd_config.

exit 0
```

`${USERNAME:-$(whoami)}` is "the environment variable, else the command's
output": `$env.get` with a default does it. It needs an `ssh` server, so it
doesn't run here.

```tessh no-run
#!/usr/bin/env tessh
# remote.tsh: Using ssh.
# After Michael Zick's remote.bash.
import getpass socket

# A basic, write stdout (local) command.
ls -l

# Now the same basic command on a remote machine.
# Pass a different USERNAME or HOSTNAME in the environment if desired:
USER = $env.get USERNAME (getpass.getuser)
HOST = $env.get HOSTNAME (socket.gethostname)

#  Now execute the above command-line on the remote host,
#+ with all transmissions encrypted.
ssh -l $USER $HOST " ls -l "

exit 0
```

`write` sends lines to another user's terminal; `mail` sends mail from a
script.

### Example 16-45. A script that mails itself (`self-mailer.sh`)

```bash no-run
#!/bin/sh
# self-mailer.sh: Self-mailing script

adr=${1:-`whoami`}     # Default to current user, if not specified.
#  Typing 'self-mailer.sh wiseguy@superdupergenius.com'
#+ sends this script to that addressee.
#  Just 'self-mailer.sh' (no argument) sends the script
#+ to the person invoking it, for example, bozo@localhost.localdomain.
#
#  For more on the ${parameter:-default} construct,
#+ see the "Parameter Substitution" section
#+ of the "Variables Revisited" chapter.

# ============================================================================
  cat $0 | mail -s "Script \"`basename $0`\" has mailed itself to you." "$adr"
# ============================================================================

# --------------------------------------------
#  Greetings from the self-mailing script.
#  A mischievous person has run this script,
#+ which has caused it to mail itself to you.
#  Apparently, some people have nothing better
#+ to do with their time.
# --------------------------------------------

echo "At `date`, script \"`basename $0`\" mailed to "$adr"."

exit 0

#  Note that the "mailx" command (in "send" mode) may be substituted
#+ for "mail" ... but with somewhat different options.
```

It sends mail, so it doesn't run here. The script reads itself from
`$argv[0]` and pipes it to `mail`; the user name comes from `getpass`
rather than a `whoami` process.

```tessh no-run
#!/usr/bin/env tessh
# self-mailer.tsh: Self-mailing script
import getpass datetime

adr = if $argv[1:] then $argv[1] else (getpass.getuser)  # Default to current user.
name = (Path $argv[0]).name

# ============================================================================
cat $argv[0] | mail -s "Script \"$name\" has mailed itself to you." $adr
# ============================================================================

# --------------------------------------------
#  Greetings from the self-mailing script.
#  A mischievous person has run this script,
#+ which has caused it to mail itself to you.
# --------------------------------------------

print "At $(((datetime.datetime.now).ctime)), script \"$name\" mailed to $adr."
exit 0
```

`mailto` sends MIME mail, `mailstats` shows the mail server's statistics,
and `vacation` answers mail while you're away. Python's `smtplib` and
`email` modules send mail directly from a script.

## 16.7 Terminal Control Commands
<!-- help: abs-terminal-control tput infocmp reset clear resize script terminfo -->

`tput` reads the terminal's capabilities from terminfo and sends them:
`tput clear`, `tput cup ROW COL` (move the cursor), `tput bold`, `smul`
(underline), `smso` (reverse), `sgr0` (back to normal), `tput longname`.
`infocmp` prints the whole terminfo entry, `reset` and `clear` reset and
clear the screen, `resize` prints the commands that set `COLUMNS` and
`LINES`, and `script` records a terminal session to a file. (`stty`, in
the previous chapter's company, sets the terminal's modes.)

They're programs, and run the same from tessh. tput's output is a string
like any command's, so it can be kept in a variable and put in a string;
Python's `shutil.get_terminal_size` gives the columns and lines without a
program. They need a terminal, so this doesn't run here.

```tessh no-run
import shutil
bold = tput bold
normal = tput sgr0
clear
tput cup 2 10
print "${bold}Hello${normal}, at row 2, column 10."
{columns lines} = shutil.get_terminal_size
print "The terminal is $columns by $lines."
```

## 16.8 Math Commands
<!-- help: abs-math-commands factor bc dc awk floating-point decimal math arithmetic -->

`factor` prints a number's prime factors. `bc` is an arbitrary-precision
calculator with a C-like language, used in bash scripts for floating
point (`variable=$(echo "scale=9; …" | bc)`, or with a here document for
longer programs); `-l` loads its math library (`s` sine, `c` cosine, `l`
log, `e` exp, `sqrt`). `dc` is a stack calculator in reverse Polish
notation. `awk`'s arithmetic is floating point too.

tessh doesn't need any of them for arithmetic: it's Python, so `/` is
floating-point division, `**` raises to a power, `math` has the functions,
`decimal` does exact decimal arithmetic to any precision (what `bc` is
for), and `fractions` exact fractions. A format spec (`${x:.2f}`) sets the
places shown. (`bc` and `dc` aren't even installed here, so the bash
versions of their examples don't run.) Note that `bc` *cuts* a result to
`scale` places, where Python's formatting rounds.

### Example 16-46. Generating prime numbers (`primes2.sh`)

```bash no-run
#!/bin/bash
# primes2.sh

#  Generating prime numbers the quick-and-easy way,
#+ without resorting to fancy algorithms.

CEILING=10000   # 1 to 10000
PRIME=0
E_NOTPRIME=

is_prime ()
{
  local factors
  factors=( $(factor $1) )  # Load output of `factor` into array.

if [ -z "${factors[2]}" ]
#  Third element of "factors" array:
#+ ${factors[2]} is 2nd factor of argument.
#  If it is blank, then there is no 2nd factor,
#+ and the argument is therefore prime.
then
  return $PRIME             # 0
else
  return $E_NOTPRIME        # null
fi
}

echo
for n in $(seq $CEILING)
do
  if is_prime $n
  then
    printf %5d $n
  fi   #    ^  Five positions per number suffices.
done   #       For a higher $CEILING, adjust upward, as necessary.

echo

exit
```

bash runs `factor` once for each number. `factor` takes any number of
arguments, so tessh runs it once for them all and reads its lines
(`n: f1 f2 …`) into Python: a number is prime when its only factor is
itself. That also leaves out 1, which the original counts as prime. The
ceiling is 100 to keep the output short.

```tessh
#!/usr/bin/env tessh
CEILING = 100       # 1 to 100 (10000 in the original)

# One factor run for all the numbers: "n: factor factor ...".
numbers = list (range 1 ($CEILING + 1))
primes = []
for line in (factor *$numbers).lines
  [n factors] = $line.split ":"
  if ($factors.split) == [$n]   # Its only factor is itself.
    $primes.append (int $n)

print
for n in $primes
  print "${n:5d}" end=""        # Five positions per number suffices.
print
exit 0
```

```output

    2    3    5    7   11   13   17   19   23   29   31   37   41   43   47   53   59   61   67   71   73   79   83   89   97
```

### Example 16-47. Monthly Payment on a Mortgage (`monthlypmt.sh`)

```bash no-run
#!/bin/bash
# monthlypmt.sh: Calculates monthly payment on a mortgage.


#  This is a modification of code in the
#+ "mcalc" (mortgage calculator) package,
#+ by Jeff Schmidt
#+ and
#+ Mendel Cooper (yours truly, the ABS Guide author).
#   http://www.ibiblio.org/pub/Linux/apps/financial/mcalc-1.6.tar.gz

echo
echo "Given the principal, interest rate, and term of a mortgage,"
echo "calculate the monthly payment."

bottom=1.0

echo
echo -n "Enter principal (no commas) "
read principal
echo -n "Enter interest rate (percent) "  # If 12%, enter "12", not ".12".
read interest_r
echo -n "Enter term (months) "
read term


 interest_r=$(echo "scale=9; $interest_r/100.0" | bc) # Convert to decimal.
                 #           ^^^^^^^^^^^^^^^^^  Divide by 100. 
                 # "scale" determines how many decimal places.

 interest_rate=$(echo "scale=9; $interest_r/12 + 1.0" | bc)
 

 top=$(echo "scale=9; $principal*$interest_rate^$term" | bc)
          #           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
          #           Standard formula for figuring interest.

 echo; echo "Please be patient. This may take a while."

 let "months = $term - 1"
# ==================================================================== 
 for ((x=$months; x > 0; x--))
 do
   bot=$(echo "scale=9; $interest_rate^$x" | bc)
   bottom=$(echo "scale=9; $bottom+$bot" | bc)
#  bottom = $(($bottom + $bot"))
 done
# ==================================================================== 

# -------------------------------------------------------------------- 
#  Rick Boivie pointed out a more efficient implementation
#+ of the above loop, which decreases computation time by 2/3.

# for ((x=1; x <= $months; x++))
# do
#   bottom=$(echo "scale=9; $bottom * $interest_rate + 1" | bc)
# done


#  And then he came up with an even more efficient alternative,
#+ one that cuts down the run time by about 95%!

# bottom=`{
#     echo "scale=9; bottom=$bottom; interest_rate=$interest_rate"
#     for ((x=1; x <= $months; x++))
#     do
#          echo 'bottom = bottom * interest_rate + 1'
#     done
#     echo 'bottom'
#     } | bc`       # Embeds a 'for loop' within command substitution.
# --------------------------------------------------------------------------
#  On the other hand, Frank Wang suggests:
#  bottom=$(echo "scale=9; ($interest_rate^$term-1)/($interest_rate-1)" | bc)

#  Because . . .
#  The algorithm behind the loop
#+ is actually a sum of geometric proportion series.
#  The sum formula is e0(1-q^n)/(1-q),
#+ where e0 is the first element and q=e(n+1)/e(n)
#+ and n is the number of elements.
# --------------------------------------------------------------------------


 # let "payment = $top/$bottom"
 payment=$(echo "scale=2; $top/$bottom" | bc)
 # Use two decimal places for dollars and cents.
 
 echo
 echo "monthly payment = \$$payment"  # Echo a dollar sign in front of amount.
 echo


 exit 0


 # Exercises:
 #   1) Filter input to permit commas in principal amount.
 #   2) Filter input to permit interest to be entered as percent or decimal.
 #   3) If you are really ambitious,
 #+     expand this script to print complete amortization tables.
```

The three answers are canned (stdin isn't usable here). Each `bc` call is
a Python float expression; the loop is Rick Boivie's faster one, and Frank
Wang's closed form gives the same sum. Exercise 1, commas in the principal,
is a `.replace`.

```tessh
#!/usr/bin/env tessh
print
print "Given the principal, interest rate, and term of a mortgage,"
print "calculate the monthly payment."

# (Canned answers; the original reads them from the terminal.)
answers = ["150,000" "6.5" "360"]
def ask prompt
  answer = $answers.pop 0
  print "$prompt $answer"
  $answer

print
principal = float ((ask "Enter principal (no commas)").replace "," "")  # (Exercise 1.)
interest_r = (float (ask "Enter interest rate (percent)")) / 100.0  # Convert to decimal.
term = int (ask "Enter term (months)")

interest_rate = $interest_r / 12 + 1.0
top = $principal * $interest_rate ** $term   # Standard formula for figuring interest.

bottom = 1.0
for x in (range 1 $term)                     # (Rick Boivie's loop.)
  bottom = $bottom * $interest_rate + 1
# Frank Wang's sum of the geometric series gives the same:
# bottom = ($interest_rate ** $term - 1) / ($interest_rate - 1)

payment = $top / $bottom
print
print "monthly payment = \$${payment:.2f}"   # Two decimal places for dollars and cents.
print
exit 0
```

```output

Given the principal, interest rate, and term of a mortgage,
calculate the monthly payment.

Enter principal (no commas) 150,000
Enter interest rate (percent) 6.5
Enter term (months) 360

monthly payment = $948.10

```

### Example 16-48. Base Conversion (`base.sh`)

```bash no-run
#!/bin/bash
###########################################################################
# Shellscript:	base.sh - print number to different bases (Bourne Shell)
# Author     :	Heiner Steven (heiner.steven@odn.de)
# Date       :	07-03-95
# Category   :	Desktop
# $Id: base.sh,v 1.2 2000/02/06 19:55:35 heiner Exp $
# ==> Above line is RCS ID info.
###########################################################################
# Description
#
# Changes
# 21-03-95 stv	fixed error occuring with 0xb as input (0.2)
###########################################################################

# ==> Used in ABS Guide with the script author's permission.
# ==> Comments added by ABS Guide author.

NOARGS=85
PN=`basename "$0"`			       # Program name
VER=`echo '$Revision: 1.2 $' | cut -d' ' -f2`  # ==> VER=1.2

Usage () {
    echo "$PN - print number to different bases, $VER (stv '95)
usage: $PN [number ...]

If no number is given, the numbers are read from standard input.
A number may be
    binary (base 2)		starting with 0b (i.e. 0b1100)
    octal (base 8)		starting with 0  (i.e. 014)
    hexadecimal (base 16)	starting with 0x (i.e. 0xc)
    decimal			otherwise (i.e. 12)" >&2
    exit $NOARGS 
}   # ==> Prints usage message.

Msg () {
    for i   # ==> in [list] missing. Why?
    do echo "$PN: $i" >&2
    done
}

Fatal () { Msg "$@"; exit 66; }

PrintBases () {
    # Determine base of the number
    for i      # ==> in [list] missing...
    do         # ==> so operates on command-line arg(s).
	case "$i" in
	    0b*)		ibase=2;;	# binary
	    0x*|[a-f]*|[A-F]*)	ibase=16;;	# hexadecimal
	    0*)			ibase=8;;	# octal
	    [1-9]*)		ibase=10;;	# decimal
	    *)
		Msg "illegal number $i - ignored"
		continue;;
	esac

	# Remove prefix, convert hex digits to uppercase (bc needs this).
	number=`echo "$i" | sed -e 's:^0[bBxX]::' | tr '[a-f]' '[A-F]'`
	# ==> Uses ":" as sed separator, rather than "/".

	# Convert number to decimal
	dec=`echo "ibase=$ibase; $number" | bc`  # ==> 'bc' is calculator utility.
	case "$dec" in
	    [0-9]*)	;;			 # number ok
	    *)		continue;;		 # error: ignore
	esac

	# Print all conversions in one line.
	# ==> 'here document' feeds command list to 'bc'.
	echo `bc <<!
	    obase=16; "hex="; $dec
	    obase=10; "dec="; $dec
	    obase=8;  "oct="; $dec
	    obase=2;  "bin="; $dec
!
    ` | sed -e 's: :	:g'

    done
}

while [ $# -gt 0 ]
# ==>  Is a "while loop" really necessary here,
# ==>+ since all the cases either break out of the loop
# ==>+ or terminate the script.
# ==> (Above comment by Paulo Marcel Coelho Aragao.)
do
    case "$1" in
	--)     shift; break;;
	-h)     Usage;;                 # ==> Help message.
	-*)     Usage;;
         *)     break;;                 # First number
    esac   # ==> Error checking for illegal input might be appropriate.
    shift
done

if [ $# -gt 0 ]
then
    PrintBases "$@"
else					# Read from stdin.
    while read line
    do
	PrintBases $line
    done
fi


exit
```

Heiner Steven's script converts with `bc` in both directions; Python's
`int text base` reads a number in any base and format specs (`X`, `o`,
`b`) write it in hex, octal and binary. The `case` on the prefix is a
`match` with patterns and guards, the option loop is a `match` on the
argument list, and the numbers come from sample arguments rather than
stdin.

```tessh
#!/usr/bin/env tessh
import sys

NOARGS = 85
PN = (Path $argv[0]).name            # Program name
VER = ('$Revision: 1.2 $'.split " ")[1]

def Usage
  print """$PN - print number to different bases, $VER (stv '95)
usage: $PN [number ...]

If no number is given, the numbers are read from standard input.
A number may be
    binary (base 2)		starting with 0b (i.e. 0b1100)
    octal (base 8)		starting with 0  (i.e. 014)
    hexadecimal (base 16)	starting with 0x (i.e. 0xc)
    decimal			otherwise (i.e. 12)""" file=$sys.stderr
  exit $NOARGS

def Msg *msgs
  for i in $msgs
    print "$PN: $i" file=$sys.stderr

def PrintBases numbers
  for i in $numbers
    # Determine base of the number, and remove its prefix.
    [ibase number] = match $i
      case _ if $i =~ re'^0[bB]'
        [2 $i[2:]]                     # binary
      case _ if $i =~ re'^0[xX]|^[a-fA-F]'
        [16 (re'^0[xX]'.sub "" $i)]    # hexadecimal
      case _ if $i =~ re'^0'
        [8 $i]                         # octal
      case _ if $i =~ re'^[1-9]'
        [10 $i]                        # decimal
      case _
        Msg "illegal number $i - ignored"
        continue
    dec = try
      int $number $ibase               # Convert number to decimal.
    except ValueError
      continue                         # error: ignore
    # Print all conversions in one line.
    print "hex=${dec:X}\tdec=$dec\toct=${dec:o}\tbin=${dec:b}"

args = $argv[1:] or ["0b1100" "014" "0xc" "12" "ff" "9z"]  # (sample arguments)
while $args
  match $args[0]
    case "--"
      args = $args[1:]
      break
    case _ if ($args[0].startswith "-")
      Usage                            # Help message.
    case _
      break                            # First number

PrintBases $args
exit 0
```

```output
hex=C	dec=12	oct=14	bin=1100
hex=C	dec=12	oct=14	bin=1100
hex=C	dec=12	oct=14	bin=1100
hex=C	dec=12	oct=14	bin=1100
hex=FF	dec=255	oct=377	bin=11111111
```

`bc` with a here document takes a whole program, with variables, `scale`
and the math library.

### Example 16-49. Invoking `bc` using a *here document* (`alt-bc.sh`)

```bash no-run
#!/bin/bash
# Invoking 'bc' using command substitution
# in combination with a 'here document'.


var1=`bc << EOF
18.33 * 19.78
EOF
`
echo $var1       # 362.56


#  $( ... ) notation also works.
v1=23.53
v2=17.881
v3=83.501
v4=171.63

var2=$(bc << EOF
scale = 4
a = ( $v1 + $v2 )
b = ( $v3 * $v4 )
a * b + 15.35
EOF
)
echo $var2       # 593487.8452


var3=$(bc -l << EOF
scale = 9
s ( 1.7 )
EOF
)
# Returns the sine of 1.7 radians.
# The "-l" option calls the 'bc' math library.
echo $var3       # .991664810


# Now, try it in a function...
hypotenuse ()    # Calculate hypotenuse of a right triangle.
{                # c = sqrt( a^2 + b^2 )
hyp=$(bc -l << EOF
scale = 9
sqrt ( $1 * $1 + $2 * $2 )
EOF
)
# Can't directly return floating point values from a Bash function.
# But, can echo-and-capture:
echo "$hyp"
}

hyp=$(hypotenuse 3.68 7.31)
echo "hypotenuse = $hyp"    # 8.184039344


exit 0
```

`decimal` is exact decimal arithmetic, as `bc` is, but it doesn't cut each
result to a `scale`, so the second answer has all its digits (`bc` says
593487.8452). `math.sin` and `math.hypot` are the math library, and a
function returns its float, where bash has to echo it and capture the text.

```tessh
#!/usr/bin/env tessh
import math
from decimal import Decimal

var1 = (Decimal "18.33") * (Decimal "19.78")
print $var1       # 362.5674 (bc: 362.56)

v1 = Decimal "23.53"
v2 = Decimal "17.881"
v3 = Decimal "83.501"
v4 = Decimal "171.63"

a = $v1 + $v2
b = $v3 * $v4
var2 = $a * $b + (Decimal "15.35")
print $var2

var3 = math.sin 1.7           # The sine of 1.7 radians.
print "${var3:.9f}"

# Now, try it in a function...
def hypotenuse a b            # c = sqrt( a^2 + b^2 )
  math.sqrt ($a * $a + $b * $b)   # (or math.hypot $a $b)

hyp = hypotenuse 3.68 7.31
print "hypotenuse = ${hyp:.9f}"

exit 0
```

```output
362.5674
593487.84652493
0.991664810
hypotenuse = 8.184039345
```

### Example 16-50. Calculating PI (`cannon.sh`)

```bash no-run
#!/bin/bash
# cannon.sh: Approximating PI by firing cannonballs.

# Author: Mendel Cooper
# License: Public Domain
# Version 2.2, reldate 13oct08.

# This is a very simple instance of a "Monte Carlo" simulation:
#+ a mathematical model of a real-life event,
#+ using pseudorandom numbers to emulate random chance.

#  Consider a perfectly square plot of land, 10000 units on a side.
#  This land has a perfectly circular lake in its center,
#+ with a diameter of 10000 units.
#  The plot is actually mostly water, except for land in the four corners.
#  (Think of it as a square with an inscribed circle.)
#
#  We will fire iron cannonballs from an old-style cannon
#+ at the square.
#  All the shots impact somewhere on the square,
#+ either in the lake or on the dry corners.
#  Since the lake takes up most of the area,
#+ most of the shots will SPLASH! into the water.
#  Just a few shots will THUD! into solid ground
#+ in the four corners of the square.
#
#  If we take enough random, unaimed shots at the square,
#+ Then the ratio of SPLASHES to total shots will approximate
#+ the value of PI/4.
#
#  The simplified explanation is that the cannon is actually
#+ shooting only at the upper right-hand quadrant of the square,
#+ i.e., Quadrant I of the Cartesian coordinate plane.
#
#
#  Theoretically, the more shots taken, the better the fit.
#  However, a shell script, as opposed to a compiled language
#+ with floating-point math built in, requires some compromises.
#  This decreases the accuracy of the simulation.


DIMENSION=10000  # Length of each side of the plot.
                 # Also sets ceiling for random integers generated.

MAXSHOTS=1000    # Fire this many shots.
                 # 10000 or more would be better, but would take too long.
PMULTIPLIER=4.0  # Scaling factor.

declare -r M_PI=3.141592654
                 # Actual 9-place value of PI, for comparison purposes.

get_random ()
{
SEED=$(head -n 1 /dev/urandom | od -N 1 | awk '{ print $2 }')
RANDOM=$SEED                                  #  From "seeding-random.sh"
                                              #+ example script.
let "rnum = $RANDOM % $DIMENSION"             #  Range less than 10000.
echo $rnum
}

distance=        # Declare global variable.
hypotenuse ()    # Calculate hypotenuse of a right triangle.
{                # From "alt-bc.sh" example.
distance=$(bc -l << EOF
scale = 0
sqrt ( $1 * $1 + $2 * $2 )
EOF
)
#  Setting "scale" to zero rounds down result to integer value,
#+ a necessary compromise in this script.
#  It decreases the accuracy of this simulation.
}


# ==========================================================
# main() {
# "Main" code block, mimicking a C-language main() function.

# Initialize variables.
shots=0
splashes=0
thuds=0
Pi=0
error=0

while [ "$shots" -lt  "$MAXSHOTS" ]           # Main loop.
do

  xCoord=$(get_random)                        # Get random X and Y coords.
  yCoord=$(get_random)
  hypotenuse $xCoord $yCoord                  #  Hypotenuse of
                                              #+ right-triangle = distance.
  ((shots++))

  printf "#%4d   " $shots
  printf "Xc = %4d  " $xCoord
  printf "Yc = %4d  " $yCoord
  printf "Distance = %5d  " $distance         #   Distance from
                                              #+  center of lake
                                              #+  -- the "origin" --
                                              #+  coordinate (0,0).

  if [ "$distance" -le "$DIMENSION" ]
  then
    echo -n "SPLASH!  "
    ((splashes++))
  else
    echo -n "THUD!    "
    ((thuds++))
  fi

  Pi=$(echo "scale=9; $PMULTIPLIER*$splashes/$shots" | bc)
  # Multiply ratio by 4.0.
  echo -n "PI ~ $Pi"
  echo

done

echo
echo "After $shots shots, PI looks like approximately   $Pi"
#  Tends to run a bit high,
#+ possibly due to round-off error and imperfect randomness of $RANDOM.
#  But still usually within plus-or-minus 5% . . .
#+ a pretty fair rough approximation.
error=$(echo "scale=9; $Pi - $M_PI" | bc)
pct_error=$(echo "scale=2; 100.0 * $error / $M_PI" | bc)
echo -n "Deviation from mathematical value of PI =        $error"
echo " ($pct_error% error)"
echo

# End of "main" code block.
# }
# ==========================================================

exit 0

#  One might well wonder whether a shell script is appropriate for
#+ an application as complex and computation-intensive as a simulation.
#
#  There are at least two justifications.
#  1) As a proof of concept: to show it can be done.
#  2) To prototype and test the algorithms before rewriting
#+    it in a compiled high-level language.
```

The random numbers come from Python's `random`, seeded so the run is the
same each time (the original reseeds `$RANDOM` from `/dev/urandom` for
every shot), and it fires 20 shots rather than 1000. `math.isqrt` is the
integer square root that `bc` with `scale = 0` gives; the rest of the
arithmetic is floating point, so no `bc` at all.

```tessh
#!/usr/bin/env tessh
# cannon.tsh: Approximating PI by firing cannonballs.
# After Mendel Cooper's cannon.sh: a very simple "Monte Carlo" simulation.
import math random

DIMENSION = 10000  # Length of each side of the plot.
MAXSHOTS = 20      # Fire this many shots (1000 in the original).
PMULTIPLIER = 4.0  # Scaling factor.
M_PI = $math.pi    # For comparison purposes.

random.seed 2      # (So each run is the same.)

def get_random = random.randrange $DIMENSION     # Range less than 10000.

def hypotenuse a b = math.isqrt ($a * $a + $b * $b)  # Rounded down, as bc's scale=0.

# main
shots = 0
splashes = 0
thuds = 0
Pi = 0

while $shots < $MAXSHOTS                    # Main loop.
  xCoord = (get_random)                     # Get random X and Y coords.
  yCoord = (get_random)
  distance = hypotenuse $xCoord $yCoord     # Distance from the center of the lake.
  shots += 1

  print "#${shots:4d}   Xc = ${xCoord:4d}  Yc = ${yCoord:4d}  Distance = ${distance:5d}  " end=""

  if $distance <= $DIMENSION
    print "SPLASH!  " end=""
    splashes += 1
  else
    print "THUD!    " end=""
    thuds += 1

  Pi = $PMULTIPLIER * $splashes / $shots    # Multiply ratio by 4.0.
  print "PI ~ ${Pi:.9f}"

print
print "After $shots shots, PI looks like approximately   ${Pi:.9f}"
error = $Pi - $M_PI
pct_error = 100.0 * $error / $M_PI
print "Deviation from mathematical value of PI =        ${error:.9f} (${pct_error:.2f}% error)"
print
exit 0
```

```output
#   1   Xc =  926  Yc = 1500  Distance =  1762  SPLASH!  PI ~ 4.000000000
#   2   Xc = 1390  Yc = 5915  Distance =  6076  SPLASH!  PI ~ 4.000000000
#   3   Xc = 2770  Yc = 5048  Distance =  5758  SPLASH!  PI ~ 4.000000000
#   4   Xc = 4121  Yc = 9927  Distance = 10748  THUD!    PI ~ 3.000000000
#   5   Xc = 3476  Yc = 9941  Distance = 10531  THUD!    PI ~ 2.400000000
#   6   Xc =  585  Yc = 9522  Distance =  9539  SPLASH!  PI ~ 2.666666667
#   7   Xc = 2594  Yc = 7056  Distance =  7517  SPLASH!  PI ~ 2.857142857
#   8   Xc = 6447  Yc = 8340  Distance = 10541  THUD!    PI ~ 2.500000000
#   9   Xc = 6095  Yc = 8915  Distance = 10799  THUD!    PI ~ 2.222222222
#  10   Xc = 7288  Yc = 8225  Distance = 10989  THUD!    PI ~ 2.000000000
#  11   Xc = 4394  Yc =  588  Distance =  4433  SPLASH!  PI ~ 2.181818182
#  12   Xc =  449  Yc = 5964  Distance =  5980  SPLASH!  PI ~ 2.333333333
#  13   Xc = 7616  Yc = 5217  Distance =  9231  SPLASH!  PI ~ 2.461538462
#  14   Xc = 6226  Yc = 6940  Distance =  9323  SPLASH!  PI ~ 2.571428571
#  15   Xc = 8613  Yc = 2694  Distance =  9024  SPLASH!  PI ~ 2.666666667
#  16   Xc = 9183  Yc = 2907  Distance =  9632  SPLASH!  PI ~ 2.750000000
#  17   Xc = 3868  Yc = 3778  Distance =  5406  SPLASH!  PI ~ 2.823529412
#  18   Xc =  390  Yc = 2895  Distance =  2921  SPLASH!  PI ~ 2.888888889
#  19   Xc = 5327  Yc = 2844  Distance =  6038  SPLASH!  PI ~ 2.947368421
#  20   Xc = 2239  Yc = 8358  Distance =  8652  SPLASH!  PI ~ 3.000000000

After 20 shots, PI looks like approximately   3.000000000
Deviation from mathematical value of PI =        -0.141592654 (-4.51% error)

```

`dc` takes its program as text too: `echo "7 8 * p" | dc` prints 56, and
`dc <<< 10k5v1+2/p` the golden ratio to ten places. In tessh they're
expressions:

```tessh
print (7 * 8)
import math
print "${(1 + (math.sqrt 5)) / 2:.10f}"
```

```output
56
1.6180339887
```

### Example 16-51. Converting a decimal number to hexadecimal (`hexconvert.sh`)

```bash no-run
#!/bin/bash
# hexconvert.sh: Convert a decimal number to hexadecimal.

E_NOARGS=85 # Command-line arg missing.
BASE=16     # Hexadecimal.

if [ -z "$1" ]
then        # Need a command-line argument.
  echo "Usage: $0 number"
  exit $E_NOARGS
fi          # Exercise: add argument validity checking.


hexcvt ()
{
if [ -z "$1" ]
then
  echo 0
  return    # "Return" 0 if no arg passed to function.
fi

echo ""$1" "$BASE" o p" | dc
#                  o    sets radix (numerical base) of output.
#                    p  prints the top of stack.
# For other options: 'man dc' ...
return
}

hexcvt "$1"

exit
```

`${n:X}` is hex in uppercase, as `dc` prints it. The exercise, checking the
argument, is `int` in a `try`.

```tessh
#!/usr/bin/env tessh
E_NOARGS = 85    # Command-line arg missing.
E_BADARG = 86

def hexcvt n=None
  if $n is None
    return 0     # "Return" 0 if no arg passed to function.
  "${n:X}"

args = $argv[1:] or ["255"]      # (sample argument, when none is given)
if not $args or not $args[0]     # Need a command-line argument.
  print "Usage: $argv[0] number"
  exit $E_NOARGS
n = try                          # (The exercise: argument validity checking.)
  int $args[0]
except ValueError
  print "Not a number: $args[0]"
  exit $E_BADARG

print (hexcvt $n)
exit 0
```

```output
FF
```

### Example 16-52. Factoring (`factr.sh`)

```bash no-run
#!/bin/bash
# factr.sh: Factor a number

MIN=2       # Will not work for number smaller than this.
E_NOARGS=85
E_TOOSMALL=86

if [ -z $1 ]
then
  echo "Usage: $0 number"
  exit $E_NOARGS
fi

if [ "$1" -lt "$MIN" ]
then
  echo "Number to factor must be $MIN or greater."
  exit $E_TOOSMALL
fi  

# Exercise: Add type checking (to reject non-integer arg).

echo "Factors of $1:"
# -------------------------------------------------------
echo  "$1[p]s2[lip/dli%0=1dvsr]s12sid2%0=13sidvsr[dli%0=\
1lrli2+dsi!>.]ds.xd1<2" | dc
# -------------------------------------------------------
#  Above code written by Michel Charpentier <charpov@cs.unh.edu>
#  (as a one-liner, here broken into two lines for display purposes).
#  Used in ABS Guide with permission (thanks!).

 exit

 # $ sh factr.sh 270138
 # 2
 # 3
 # 11
 # 4093
```

Michel Charpentier's `dc` one-liner divides by 2 and then by odd numbers up
to the square root. The same trial division in Python is a short loop (and
the `factor` program does it in one call, as in Example 16-46). The
exercise, rejecting a non-integer, is `str.isdigit`.

```tessh
#!/usr/bin/env tessh
MIN = 2       # Will not work for number smaller than this.
E_NOARGS = 85
E_TOOSMALL = 86
E_NOTINT = 87

def factors n
  # Trial division: by 2, then by odd numbers up to the square root.
  d = 2
  while $d * $d <= $n
    while $n % $d == 0
      yield $d
      n //= $d
    d += if $d == 2 then 1 else 2
  if $n > 1
    yield $n

args = $argv[1:] or ["270138"]   # (sample argument, when none is given)
if not $args or not $args[0]
  print "Usage: $argv[0] number"
  exit $E_NOARGS
if not ($args[0].isdigit)        # (The exercise: type checking.)
  print "Number to factor must be an integer."
  exit $E_NOTINT
number = int $args[0]
if $number < $MIN
  print "Number to factor must be $MIN or greater."
  exit $E_TOOSMALL

print "Factors of $number:"
for f in (factors $number)
  print $f
factor $number                   # (The factor program agrees.)
exit 0
```

```output
Factors of 270138:
2
3
11
4093
270138: 2 3 11 4093
```

### Example 16-53. Calculating the hypotenuse of a triangle (`hypotenuse.sh`)

```bash no-run
#!/bin/bash
# hypotenuse.sh: Returns the "hypotenuse" of a right triangle.
#                (square root of sum of squares of the "legs")

ARGS=2                # Script needs sides of triangle passed.
E_BADARGS=85          # Wrong number of arguments.

if [ $# -ne "$ARGS" ] # Test number of arguments to script.
then
  echo "Usage: `basename $0` side_1 side_2"
  exit $E_BADARGS
fi


AWKSCRIPT=' { printf( "%3.7f\n", sqrt($1*$1 + $2*$2) ) } '
#             command(s) / parameters passed to awk


# Now, pipe the parameters to awk.
    echo -n "Hypotenuse of $1 and $2 = "
    echo $1 $2 | awk "$AWKSCRIPT"
#   ^^^^^^^^^^^^
# An echo-and-pipe is an easy way of passing shell parameters to awk.

exit

# Exercise: Rewrite this script using 'bc' rather than awk.
#           Which method is more intuitive?
```

The `awk` program runs as it is, with the numbers piped in. The exercise
asks for `bc`; in tessh it's Python, with `%3.7f` as a format spec.

```tessh
#!/usr/bin/env tessh
import math
ARGS = 2              # Script needs sides of triangle passed.
E_BADARGS = 85        # Wrong number of arguments.

args = $argv[1:] or ["3" "4.5"]  # (sample arguments, when none are given)
if (len $args) != $ARGS          # Test number of arguments to script.
  print "Usage: $(Path $argv[0]).name side_1 side_2"
  exit $E_BADARGS
[a b] = $args

AWKSCRIPT = ' { printf( "%3.7f\n", sqrt($1*$1 + $2*$2) ) } '

# Now, pipe the parameters to awk.
print "Hypotenuse of $a and $b = " end=""
echo $a $b | awk $AWKSCRIPT

# The exercise, without awk or bc:
print "Hypotenuse of $a and $b = ${math.hypot (float $a) (float $b):3.7f}"
exit
```

```output
Hypotenuse of 3 and 4.5 = 5.4083269
Hypotenuse of 3 and 4.5 = 5.4083269
```

## 16.9 Miscellaneous Commands
<!-- help: abs-misc-commands seq jot getopt run-parts yes banner printenv lp tee mkfifo pathchk dd od hexdump objdump mcookie units m4 xmessage zenity doexec dialog sox -->

`seq` (and BSD's `jot`) prints a sequence of numbers (`seq 5`, `seq -s :
5`, `seq FIRST STEP LAST`), for `for` loops; `getopt` parses options, long
ones too; `run-parts` runs every script in a folder; `yes` prints `y` (or
its arguments) forever, to answer a program's questions; `banner` prints
big letters; `printenv` the environment; `lp`/`lpr` print on paper; `tee`
copies a pipe to a file on the way through; `mkfifo` makes a named pipe;
`pathchk` checks a file name; `dd` copies with conversions, block sizes,
skips and counts (disk images, too); `od`, `hexdump` and `objdump` dump
binary files; `mcookie` makes a random 128-bit hex number; `units`
converts between units; `m4` is a macro processor; `xmessage`, `zenity` and
`dialog` show dialogs; `doexec` runs a program under another `argv[0]`;
`sox` converts sound files.

In tessh, `seq` in a loop is `range` (no process, no splitting of text), and
`yes`, `mcookie` and `units` have small Python stand-ins, but the programs
all run as they are.

### Example 16-54. Using `seq` to generate loop arguments (`ex53.sh`)

```bash
#!/bin/bash
# Using "seq"

echo

for a in `seq 80`  # or   for a in $( seq 80 )
# Same as   for a in 1 2 3 4 5 ... 80   (saves much typing!).
# May also use 'jot' (if present on system).
do
  echo -n "$a "
done      # 1 2 3 4 5 ... 80
# Example of using the output of a command to generate 
# the [list] in a "for" loop.

echo; echo


COUNT=80  # Yes, 'seq' also accepts a replaceable parameter.

for a in `seq $COUNT`  # or   for a in $( seq $COUNT )
do
  echo -n "$a "
done      # 1 2 3 4 5 ... 80

echo; echo

BEGIN=75
END=80

for a in `seq $BEGIN $END`
#  Giving "seq" two arguments starts the count at the first one,
#+ and continues until it reaches the second.
do
  echo -n "$a "
done      # 75 76 77 78 79 80

echo; echo

BEGIN=45
INTERVAL=5
END=80

for a in `seq $BEGIN $INTERVAL $END`
#  Giving "seq" three arguments starts the count at the first one,
#+ uses the second for a step interval,
#+ and continues until it reaches the third.
do
  echo -n "$a "
done      # 45 50 55 60 65 70 75 80

echo; echo

exit 0
```

`range` takes the same one, two or three numbers as `seq`, except that it
stops *before* its end and its step comes last. `(seq 80).lines` would
work too, as text.

```tessh
#!/usr/bin/env tessh
# Using "range" (where bash uses "seq")

print

for a in (range 1 81)        # or   for a in (seq 80).lines
  print "$a " end=""
print ; print

COUNT = 80
for a in (range 1 ($COUNT + 1))
  print "$a " end=""
print ; print

BEGIN = 75
END = 80
for a in (range $BEGIN ($END + 1))
  print "$a " end=""         # 75 76 77 78 79 80
print ; print

BEGIN = 45
INTERVAL = 5
END = 80
for a in (range $BEGIN ($END + 1) $INTERVAL)
  print "$a " end=""         # 45 50 55 60 65 70 75 80
print ; print

exit 0
```

```output

1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 

1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 

75 76 77 78 79 80 

45 50 55 60 65 70 75 80 

```

The ABS Guide's simpler example makes ten files named `file.1` to
`file.10`:

```bash
COUNT=10
PREFIX=file

for filename in `seq $COUNT`
do
  touch $PREFIX.$filename
  #  Or, can do other operations,
  #+ such as rm, grep, etc.
done
ls
```

```tessh
COUNT = 10
PREFIX = file

for n in (range 1 ($COUNT + 1))
  touch "$PREFIX.$n"
ls
```

```output
file.1
file.10
file.2
file.3
file.4
file.5
file.6
file.7
file.8
file.9
```

### Example 16-55. Letter Count (`letter-count.sh`)

```bash no-run
#!/bin/bash
# letter-count.sh: Counting letter occurrences in a text file.
# Written by Stefano Palmeri.
# Used in ABS Guide with permission.
# Slightly modified by document author.

MINARGS=2          # Script requires at least two arguments.
E_BADARGS=65
FILE=$1

let LETTERS=$#-1   # How many letters specified (as command-line args).
                   # (Subtract 1 from number of command-line args.)


show_help(){
	   echo
           echo Usage: `basename $0` file letters  
           echo Note: `basename $0` arguments are case sensitive.
           echo Example: `basename $0` foobar.txt G n U L i N U x.
	   echo
}

# Checks number of arguments.
if [ $# -lt $MINARGS ]; then
   echo
   echo "Not enough arguments."
   echo
   show_help
   exit $E_BADARGS
fi  


# Checks if file exists.
if [ ! -f $FILE ]; then
    echo "File \"$FILE\" does not exist."
    exit $E_BADARGS
fi



# Counts letter occurrences .
for n in `seq $LETTERS`; do
      shift
      if [[ `echo -n "$1" | wc -c` -eq 1 ]]; then             #  Checks arg.
             echo "$1" -\> `cat $FILE | tr -cd  "$1" | wc -c` #  Counting.
      else
             echo "$1 is not a  single char."
      fi  
done

exit $?

#  This script has exactly the same functionality as letter-count2.sh,
#+ but executes faster.
#  Why?
```

Stefano Palmeri's script runs `cat | tr -cd | wc -c` for every letter;
Python's `str.count` counts in the text read once. The arguments are the
file and then the letters, so `[FILE *letters] = $args` takes them apart
where bash `shift`s through them.

```tessh
#!/usr/bin/env tessh
printf 'GNU is Not Unix, and Linux is not GNU.\n' > foobar.txt   # (made up)

MINARGS = 2          # Script requires at least two arguments.
E_BADARGS = 65
ME = (Path $argv[0]).name

def show_help
  print
  print "Usage: $ME file letters"
  print "Note: $ME arguments are case sensitive."
  print "Example: $ME foobar.txt G n U L i N U x."
  print

# (sample arguments, when none are given)
args = $argv[1:] or [foobar.txt G n U L i N U x xy]

# Checks number of arguments.
if (len $args) < $MINARGS
  print
  print "Not enough arguments."
  print
  show_help
  exit $E_BADARGS
[FILE *letters] = $args

# Checks if file exists.
if not ((Path $FILE).is_file)
  print "File \"$FILE\" does not exist."
  exit $E_BADARGS

text = ((Path $FILE).read_text)
# Counts letter occurrences.
for letter in $letters
  if (len $letter) == 1                  # Checks arg.
    print "$letter -> $($text.count $letter)"
  else
    print "$letter is not a  single char."

exit 0
```

```output
G -> 2
n -> 4
U -> 3
L -> 1
i -> 4
N -> 3
U -> 3
x -> 2
xy is not a  single char.
```

`getopt` sorts the options out and prints them back, one per word, with
`--` before the other arguments. bash `set`s its output as the new
arguments; tessh splits it into a list.

### Example 16-56. Using `getopt` to parse command-line options (`ex33a.sh`)

```bash no-run
#!/bin/bash
# Using getopt

# Try the following when invoking this script:
#   sh ex33a.sh -a
#   sh ex33a.sh -abc
#   sh ex33a.sh -a -b -c
#   sh ex33a.sh -d
#   sh ex33a.sh -dXYZ
#   sh ex33a.sh -d XYZ
#   sh ex33a.sh -abcd
#   sh ex33a.sh -abcdZ
#   sh ex33a.sh -z
#   sh ex33a.sh a
# Explain the results of each of the above.

E_OPTERR=65

if [ "$#" -eq 0 ]
then   # Script needs at least one command-line argument.
  echo "Usage $0 -[options a,b,c]"
  exit $E_OPTERR
fi  

set -- `getopt "abcd:" "$@"`
# Sets positional parameters to command-line arguments.
# What happens if you use "$*" instead of "$@"?

while [ ! -z "$1" ]
do
  case "$1" in
    -a) echo "Option \"a\"";;
    -b) echo "Option \"b\"";;
    -c) echo "Option \"c\"";;
    -d) echo "Option \"d\" $2";;
     *) break;;
  esac

  shift
done

#  It is usually better to use the 'getopts' builtin in a script.
#  See "ex33.sh."

exit 0
```

Here the script's body is a function, run with each of the argument lists
its comments suggest. `getopt "abcd:"` (the old-style call) prints its
answer without quotes, so `.split` takes it apart; `match` on a list
pattern takes `-d` and its value together. `getopt`'s own complaints go to
stderr. (Python's `argparse` is the tessh way to take options; chapter 15
compares it with `getopts`.)

```tessh
#!/usr/bin/env tessh
E_OPTERR = 65

def main args
  print "--- $(' '.join $args)"
  if not $args                     # Script needs at least one argument.
    print "Usage $argv[0] -[options a,b,c]"
    return $E_OPTERR

  words = ((getopt "abcd:" *$args 2>/dev/null).split)
  while $words
    match $words
      case ["-a" *$rest]
        print "Option \"a\""
      case ["-b" *$rest]
        print "Option \"b\""
      case ["-c" *$rest]
        print "Option \"c\""
      case ["-d" $value *$rest]
        print "Option \"d\" $value"
        words = $words[1:]
      case _
        break
    words = $words[1:]

# (The argument lists the original suggests, when none are given.)
if $argv[1:]
  main $argv[1:]
else
  for args in [[-a] [-abc] [-a -b -c] [-d] [-dXYZ] [-d XYZ] [-abcd] [-abcdZ] [-z] [a]]
    main $args
exit 0
```

```output
--- -a
Option "a"
--- -abc
Option "a"
Option "b"
Option "c"
--- -a -b -c
Option "a"
Option "b"
Option "c"
--- -d
--- -dXYZ
Option "d" XYZ
--- -d XYZ
Option "d" XYZ
--- -abcd
Option "a"
Option "b"
Option "c"
--- -abcdZ
Option "a"
Option "b"
Option "c"
Option "d" Z
--- -z
--- a
```

`run-parts` runs a folder's scripts in order, as cron does with
`/etc/cron.daily`. `yes` repeats a line forever, to answer a program that
asks questions (`yes | fsck …`, with care). The ABS Guide's bash function
that emulates it never ends; a tessh generator yields lines only as they're
asked for, so `take` can take a few:

```bash no-run
yes ()
{ # Trivial emulation of "yes" ...
  local DEFAULT_TEXT="y"
  while [ true ]   # Endless loop.
  do
    if [ -z "$1" ]
    then
      echo "$DEFAULT_TEXT"
    else           # If argument ...
      echo "$1"    # ... expand and echo it.
    fi
  done             #  The only things missing are the
}                  #+ --help and --version options.
```

```tessh
def yes text="y"
  while True
    yield $text

print (take 3 (yes))
print (take 2 (yes "I agree"))
\yes | head -n 2                 # (The program: \yes, as yes is now ours.)
```

```output
['y' 'y' 'y']
['I agree' 'I agree']
y
y
```

`banner` is gone from most systems, `printenv` prints the environment
(`$env` in tessh), and `lp` and `lpr` send files to the printer. `tee`
saves what goes through a pipe without stopping it:

```bash
printf 'b\na\nb\n' > listfile1 ; printf 'c\na\n' > listfile2
cat listfile* | sort | tee check.file | uniq > result.file
cat check.file
echo ---
cat result.file
```

```tessh
printf 'b\na\nb\n' > listfile1 ; printf 'c\na\n' > listfile2
cat listfile* | sort | tee check.file | uniq > result.file
cat check.file
echo ---
cat result.file
```

```output
a
a
b
b
c
---
a
b
c
```

`mkfifo` makes a named pipe: one process writes into it, another reads
from it. Omair Eshkenazi's example sends `ls -l`'s owner column through
one into a background job; it's the same in tessh, with the background
job's two programs piped together and `&` at the end (the listing has
your user name in it, hence no output here):

```bash no-run
#!/bin/bash
# This short script by Omair Eshkenazi.
# Used in ABS Guide with permission (thanks!).

mkfifo pipe1   # Yes, pipes can be given names.
mkfifo pipe2   # Hence the designation "named pipe."

(cut -d' ' -f1 | tr "a-z" "A-Z") >pipe2 <pipe1 &
ls -l | tr -s ' ' | cut -d' ' -f3,9- | tee pipe1 |
cut -d' ' -f2 | paste - pipe2

rm -f pipe1
rm -f pipe2

# No need to kill background processes when script terminates (why not?).

exit $?
```

```tessh no-run
#!/usr/bin/env tessh
mkfifo pipe1 pipe2

cut -d' ' -f1 <pipe1 | tr "a-z" "A-Z" >pipe2 &
ls -l | tr -s ' ' | cut -d' ' -f3,9- | tee pipe1 | cut -d' ' -f2 | paste - pipe2

rm -f pipe1 pipe2
```

`pathchk` checks that a name is a valid path. `dd` copies with conversions
(`conv=ucase`, `lcase`, `swab`, `ebcdic`), from `if=` to `of=`, in blocks of
`bs=` bytes, skipping `skip=` blocks of the input, `seek=` blocks of the
output, and copying `count=` blocks.

### Example 16-57. A script that copies itself (`self-copy.sh`)

```bash no-run
#!/bin/bash
# self-copy.sh

# This script copies itself.

file_subscript=copy

dd if=$0 of=$0.$file_subscript 2>/dev/null
# Suppress messages from dd:   ^^^^^^^^^^^

exit $?

#  A program whose only output is its own source code
#+ is called a "quine" per Willard Quine.
#  Does this script qualify as a quine?
```

The copy goes in the current folder (the bash writes it next to the
script), and `cmp` shows it's the same.

```tessh
#!/usr/bin/env tessh
# self-copy.tsh
# This script copies itself.

file_subscript = copy
copy = "$((Path $argv[0]).name).$file_subscript"

dd if=$argv[0] of=$copy 2>/dev/null
# Suppress messages from dd:  ^^^^^^^^^^^

cmp $argv[0] $copy && print "$copy is the same as the script."
exit $?
```

```output
ex.tsh.copy is the same as the script.
```

### Example 16-58. Exercising `dd` (`exercising-dd.sh`)

```bash
#!/bin/bash
# exercising-dd.sh

# Script by Stephane Chazelas.
# Somewhat modified by ABS Guide author.

infile=$0           # This script.
outfile=log.txt     # Output file left behind.
n=8
p=11

dd if=$infile of=$outfile bs=1 skip=$((n-1)) count=$((p-n+1)) 2> /dev/null
# Extracts characters n to p (8 to 11) from this script ("bash").

# ----------------------------------------------------------------

echo -n "hello vertical world" | dd cbs=1 conv=unblock 2> /dev/null
# Echoes "hello vertical world" vertically downward.
# Why? A newline follows each character dd emits.

exit $?
```

```output bash
h
e
l
l
o

v
e
r
t
i
c
a
l

w
o
r
l
d
```

The script's own first line is `#!/usr/bin/env tessh`, so characters 16 to
20 pick out "tessh" as 8 to 11 picked "bash", and the tessh version shows
what it left in `log.txt`. Python slices do the first part
(`(Path $infile).read_text)[15:20]`).

```tessh
#!/usr/bin/env tessh
# exercising-dd.tsh
# After Stephane Chazelas' exercising-dd.sh.

infile = $argv[0]   # This script.
outfile = log.txt   # Output file left behind.
n = 16
p = 20

dd if=$infile of=$outfile bs=1 skip=($n - 1) count=($p - $n + 1) 2>/dev/null
# Extracts characters n to p (16 to 20) from this script ("tessh").
print (cat $outfile)

# ----------------------------------------------------------------

echo -n "hello vertical world" | dd cbs=1 conv=unblock 2>/dev/null
# Echoes "hello vertical world" vertically downward.
# Why? A newline follows each character dd emits.

exit $?
```

```output
tessh
h
e
l
l
o

v
e
r
t
i
c
a
l

w
o
r
l
d
```

`dd` reads keys one at a time once `stty` has turned off line editing:

### Example 16-59. Capturing Keystrokes (`dd-keypress.sh`)

```bash no-run
#!/bin/bash
# dd-keypress.sh: Capture keystrokes without needing to press ENTER.


keypresses=4                      # Number of keypresses to capture.


old_tty_setting=$(stty -g)        # Save old terminal settings.

echo "Press $keypresses keys."
stty -icanon -echo                # Disable canonical mode.
                                  # Disable local echo.
keys=$(dd bs=1 count=$keypresses 2> /dev/null)
# 'dd' uses stdin, if "if" (input file) not specified.

stty "$old_tty_setting"           # Restore old terminal settings.

echo "You pressed the \"$keys\" keys."

# Thanks, Stephane Chazelas, for showing the way.
exit 0
```

The same in tessh, with `try`/`finally` so the terminal's settings come
back even when the script is interrupted. It needs a terminal to type in.

```tessh no-run
#!/usr/bin/env tessh
# dd-keypress.tsh: Capture keystrokes without needing to press ENTER.

keypresses = 4                    # Number of keypresses to capture.

old_tty_setting = stty -g         # Save old terminal settings.

print "Press $keypresses keys."
stty -icanon -echo                # Disable canonical mode and local echo.
keys = try
  dd bs=1 count=$keypresses 2>/dev/null   # dd reads stdin: the keys.
finally
  stty $old_tty_setting           # Restore old terminal settings.

print "You pressed the \"$keys\" keys."
exit 0
```

`dd` also does random access: `seek=` writes at an offset in a file, and
`conv=notrunc` keeps the rest of it. Python opens the file for reading and
writing and `seek`s:

```bash
printf 'abcdefgh' > file
echo -n . | dd bs=1 seek=4 of=file conv=notrunc 2>/dev/null
cat file ; echo
```

```output bash
abcd.fgh
```

```tessh
printf 'abcdefgh' > file
echo -n . | dd bs=1 seek=4 of=file conv=notrunc 2>/dev/null
cat file ; echo
with (open file "r+b") as f
  $f.seek 6
  $f.write b"!"
cat file ; echo
```

```output
abcd.fgh
abcd.f!h
```

`dd` copies disk images to and from devices: boot floppies once, SD cards
and USB sticks now (`dd if=image.iso of=/dev/sdb`).

### Example 16-60. Preparing a bootable SD card for the *Raspberry Pi* (`rp.sdcard.sh`)

```bash no-run
#!/bin/bash
# rp.sdcard.sh
# Preparing an SD card with a bootable image for the Raspberry Pi.

# $1 = imagefile name
# $2 = sdcard (device file)
# Otherwise defaults to the defaults, see below.

DEFAULTbs=4M                                 # Block size, 4 mb default.
DEFAULTif="2013-07-26-wheezy-raspbian.img"   # Commonly used distro.
DEFAULTsdcard="/dev/mmcblk0"                 # May be different. Check!
ROOTUSER_NAME=root                           # Must run as root!
E_NOTROOT=81
E_NOIMAGE=82

username=$(id -nu)                           # Who is running this script?
if [ "$username" != "$ROOTUSER_NAME" ]
then
  echo "This script must run as root or with root privileges."
  exit $E_NOTROOT
fi

if [ -n "$1" ]
then
  imagefile="$1"
else
  imagefile="$DEFAULTif"
fi

if [ -n "$2" ]
then
  sdcard="$2"
else
  sdcard="$DEFAULTsdcard"
fi

if [ ! -e $imagefile ]
then
  echo "Image file \"$imagefile\" not found!"
  exit $E_NOIMAGE
fi

echo "Last chance to change your mind!"; echo
read -s -n1 -p "Hit a key to write $imagefile to $sdcard [Ctl-c to exit]."
echo; echo

echo "Writing $imagefile to $sdcard ..."
dd bs=$DEFAULTbs if=$imagefile of=$sdcard

exit $?

# Exercises:
# ---------
# 1) Provide additional error checking.
# 2) Have script autodetect device file for SD card (difficult!).
# 3) Have script sutodetect image file (*img) in $PWD.
```

It must run as root and writes to a device, so it doesn't run here.
`os.getuid` checks for root without `id`, exercise 3 (find the image in the
folder) is a Glob, and the key press is `input` (which waits for Enter).

```tessh no-run
#!/usr/bin/env tessh
# rp.sdcard.tsh
# Preparing an SD card with a bootable image for the Raspberry Pi.
import os

DEFAULTbs = 4M                                  # Block size, 4 mb default.
DEFAULTsdcard = /dev/mmcblk0                    # May be different. Check!
E_NOTROOT = 81
E_NOIMAGE = 82

if (os.getuid) != 0                             # Who is running this script?
  print "This script must run as root or with root privileges."
  exit $E_NOTROOT

# Exercise 3: the image file is the first *.img here, unless one is given.
images = *.img
imagefile = if $argv[1:] then Path $argv[1] elif $images then $images[0] else None
sdcard = if $argv[2:] then $argv[2] else $DEFAULTsdcard

if $imagefile is None or not ($imagefile.exists)
  print "Image file \"$imagefile\" not found!"
  exit $E_NOIMAGE

print "Last chance to change your mind!" ; print
input "Hit Enter to write $imagefile to $sdcard [Ctl-c to exit]."
print ; print

print "Writing $imagefile to $sdcard ..."
dd bs=$DEFAULTbs if=$imagefile of=$sdcard

exit $?
```

### Example 16-61. Securely deleting a file (`blot-out.sh`)

```bash no-run
#!/bin/bash
# blot-out.sh: Erase "all" traces of a file.

#  This script overwrites a target file alternately
#+ with random bytes, then zeros before finally deleting it.
#  After that, even examining the raw disk sectors by conventional methods
#+ will not reveal the original file data.

PASSES=7         #  Number of file-shredding passes.
                 #  Increasing this slows script execution,
                 #+ especially on large target files.
BLOCKSIZE=1      #  I/O with /dev/urandom requires unit block size,
                 #+ otherwise you get weird results.
E_BADARGS=70     #  Various error exit codes.
E_NOT_FOUND=71
E_CHANGED_MIND=72

if [ -z "$1" ]   # No filename specified.
then
  echo "Usage: `basename $0` filename"
  exit $E_BADARGS
fi

file=$1

if [ ! -e "$file" ]
then
  echo "File \"$file\" not found."
  exit $E_NOT_FOUND
fi  

echo; echo -n "Are you absolutely sure you want to blot out \"$file\" (y/n)? "
read answer
case "$answer" in
[nN]) echo "Changed your mind, huh?"
      exit $E_CHANGED_MIND
      ;;
*)    echo "Blotting out file \"$file\".";;
esac


flength=$(ls -l "$file" | awk '{print $5}')  # Field 5 is file length.
pass_count=1

chmod u+w "$file"   # Allow overwriting/deleting the file.

echo

while [ "$pass_count" -le "$PASSES" ]
do
  echo "Pass #$pass_count"
  sync         # Flush buffers.
  dd if=/dev/urandom of=$file bs=$BLOCKSIZE count=$flength
               # Fill with random bytes.
  sync         # Flush buffers again.
  dd if=/dev/zero of=$file bs=$BLOCKSIZE count=$flength
               # Fill with zeros.
  sync         # Flush buffers yet again.
  let "pass_count += 1"
  echo
done  


rm -f $file    # Finally, delete scrambled and shredded file.
sync           # Flush buffers a final time.

echo "File \"$file\" blotted out and deleted."; echo


exit 0

#  This is a fairly secure, if inefficient and slow method
#+ of thoroughly "shredding" a file.
#  The "shred" command, part of the GNU "fileutils" package,
#+ does the same thing, although more efficiently.

#  The file cannot not be "undeleted" or retrieved by normal methods.
#  However . . .
#+ this simple method would *not* likely withstand
#+ sophisticated forensic analysis.

#  This script may not play well with a journaled file system.
#  Exercise (difficult): Fix it so it does.



#  Tom Vier's "wipe" file-deletion package does a much more thorough job
#+ of file shredding than this simple script.
#     http://www.ibiblio.org/pub/Linux/utils/file/wipe-2.0.0.tar.bz2

#  For an in-depth analysis on the topic of file deletion and security,
#+ see Peter Gutmann's paper,
#+     "Secure Deletion of Data From Magnetic and Solid-State Memory".
#       http://www.cs.auckland.ac.nz/~pgut001/pubs/secure_del.html
```

It runs on a made-up file here, with the answer canned. The length comes
from `stat` rather than cutting field 5 out of `ls -l`, and `sync` is given
the file, so only it is flushed. `dd` prints its record counts on stderr,
so the passes show only their numbers. (`shred -u file` does all this in
one command.)

```tessh
#!/usr/bin/env tessh
import os
print "Top secret. Burn after reading." >secret.txt   # (made up)

PASSES = 7         #  Number of file-shredding passes.
BLOCKSIZE = 1      #  I/O with /dev/urandom requires unit block size.
E_BADARGS = 70
E_NOT_FOUND = 71
E_CHANGED_MIND = 72

args = $argv[1:] or [secret.txt]   # (sample argument, when none is given)
if not $args or not $args[0]       # No filename specified.
  print "Usage: $(Path $argv[0]).name filename"
  exit $E_BADARGS
file = Path $args[0]

if not ($file.exists)
  print "File \"$file\" not found."
  exit $E_NOT_FOUND

print ; print "Are you absolutely sure you want to blot out \"$file\" (y/n)? " end=""
answer = y         # (canned; the original reads it from the terminal)
print $answer
match $answer
  case "n" | "N"
    print "Changed your mind, huh?"
    exit $E_CHANGED_MIND
  case _
    print "Blotting out file \"$file\"."

flength = ($file.stat).st_size
chmod u+w $file    # Allow overwriting/deleting the file.
print

for pass_count in (range 1 ($PASSES + 1))
  print "Pass #$pass_count"
  sync $file       # Flush buffers.
  dd if=/dev/urandom of=$file bs=$BLOCKSIZE count=$flength 2>/dev/null
  sync $file       # Fill with random bytes, flush again.
  dd if=/dev/zero of=$file bs=$BLOCKSIZE count=$flength 2>/dev/null
  sync $file       # Fill with zeros, flush yet again.
  print

rm -f $file        # Finally, delete scrambled and shredded file.
print "File \"$file\" blotted out and deleted." ; print
exit 0
```

```output

Are you absolutely sure you want to blot out "secret.txt" (y/n)? y
Blotting out file "secret.txt".

Pass #1

Pass #2

Pass #3

Pass #4

Pass #5

Pass #6

Pass #7

File "secret.txt" blotted out and deleted.

```

`od` dumps a file in octal, decimal or hex (`od -tu4` in the random number
example), as do `hexdump -C` and `xxd`; `objdump -d` disassembles a
program. `mcookie` prints 32 random hex digits, and a checksum of the
script itself is another "random" number: `random001 = ((md5sum
$argv[0]).split)[0]`.

### Example 16-62. Filename generator (`tempfile-name.sh`)

```bash
#!/bin/bash
# tempfile-name.sh:  temp filename generator

BASE_STR=`mcookie`   # 32-character magic cookie.
POS=11               # Arbitrary position in magic cookie string.
LEN=5                # Get $LEN consecutive characters.

prefix=temp          #  This is, after all, a "temp" file.
                     #  For more "uniqueness," generate the
                     #+ filename prefix using the same method
                     #+ as the suffix, below.

suffix=${BASE_STR:POS:LEN}
                     #  Extract a 5-character string,
                     #+ starting at position 11.

temp_filename=$prefix.$suffix
                     # Construct the filename.

echo "Temp filename = "$temp_filename""

# sh tempfile-name.sh
# Temp filename = temp.e19ea

#  Compare this method of generating "unique" filenames
#+ with the 'date' method in ex51.sh.

exit 0
```

bash's `${BASE_STR:POS:LEN}` is a slice, `$BASE_STR[$POS : $POS + $LEN]`.
Python's `secrets.token_hex 16` makes the same kind of cookie without a
program (and `tempfile`, in 16.5, makes the file itself).

```tessh
#!/usr/bin/env tessh
import secrets

BASE_STR = (mcookie)  # 32-character magic cookie.
POS = 11              # Arbitrary position in magic cookie string.
LEN = 5               # Get $LEN consecutive characters.

prefix = temp         #  This is, after all, a "temp" file.

suffix = $BASE_STR[$POS : $POS + $LEN]
temp_filename = "$prefix.$suffix"
print "Temp filename = $temp_filename"
print "From Python:   = $prefix.$((secrets.token_hex 16)[$POS : $POS + $LEN])"
exit 0
```

```text
(one run)
Temp filename = temp.950c8
From Python:   = temp.87b39
```

`units` converts between units of measure, interactively or in a script.

### Example 16-63. Converting meters to miles (`unit-conversion.sh`)

```bash no-run
#!/bin/bash
# unit-conversion.sh
# Must have 'units' utility installed.


convert_units ()  # Takes as arguments the units to convert.
{
  cf=$(units "$1" "$2" | sed --silent -e '1p' | awk '{print $2}')
  # Strip off everything except the actual conversion factor.
  echo "$cf"
}  

Unit1=miles
Unit2=meters
cfactor=`convert_units $Unit1 $Unit2`
quantity=3.73

result=$(echo $quantity*$cfactor | bc)

echo "There are $result $Unit2 in $quantity $Unit1."

#  What happens if you pass incompatible units,
#+ such as "acres" and "miles" to the function?

exit 0

# Exercise: Edit this script to accept command-line parameters,
#           with appropriate error checking, of course.
```

`units` isn't installed here, so the conversion factors are a small dict
of meters per unit (with `units` it would be `cf = ((units $a $b).split)[1]`,
the second word of its first line). The exercise is done too: units and
quantity from the command line, and a clear error for a unit it doesn't
know.

```tessh
#!/usr/bin/env tessh
METERS_PER = {meters: 1 miles: 1609.344 feet: 0.3048 kilometers: 1000}

def convert_units unit1 unit2   # Takes as arguments the units to convert.
  $METERS_PER[$unit1] / $METERS_PER[$unit2]

# (The exercise: Unit1, Unit2 and quantity from the command line.)
args = $argv[1:] or [miles meters "3.73"]   # (sample arguments)
if (len $args) != 3
  print "Usage: $(Path $argv[0]).name from-unit to-unit quantity"
  exit 1
[Unit1 Unit2 quantity] = $args
cfactor = try
  convert_units $Unit1 $Unit2
except KeyError as e
  print "Unknown unit: $e"
  exit 1

result = (float $quantity) * $cfactor
print "There are $result $Unit2 in $quantity $Unit1."
exit 0
```

```output
There are 6002.85312 meters in 3.73 miles.
```

`m4` is a macro processor, a language of its own with string, regex and
arithmetic built-ins.

### Example 16-64. Using `m4` (`m4.sh`)

```bash no-run
#!/bin/bash
# m4.sh: Using the m4 macro processor

# Strings
string=abcdA01
echo "len($string)" | m4                            #   7
echo "substr($string,4)" | m4                       # A01
echo "regexp($string,[0-1][0-1],\&Z)" | m4      # 01Z

# Arithmetic
var=99
echo "incr($var)" | m4                              #  100
echo "eval($var / 3)" | m4                          #   33

exit
```

`m4` isn't installed here. Each of its built-ins used here is a Python
expression in tessh: `len`, a slice for `substr`, `re.sub` for `regexp`
(its `\&` is the whole match, `\g<0>` in Python), `+ 1` and `//`. (tessh's
own macros, `defmacro`, work on tessh code as it compiles, not on text.)

```tessh
#!/usr/bin/env tessh
import re
# Strings
string = abcdA01
print (len $string)                                    #   7
print $string[4:]                                      # A01
m = re.search '[0-1][0-1]' $string
print ($m.expand '\g<0>Z')                             # 01Z

# Arithmetic
var = 99
print ($var + 1)                                       #  100
print ($var // 3)                                      #   33
exit
```

```output
7
A01
01Z
100
33
```

`xmessage` and `zenity` pop up dialogs on the desktop, and `dialog` (with
`gdialog`, `Xdialog` and `kdialog`) in a terminal. `doexec` runs a program
under another name, which it can `match` on, as Example 16-2 does with
`$argv[0]`. `sox` converts and plays sound files, a job for a script's
loop over a Glob of them:

```tessh no-run
for f in *.wav
  sox $f ($f.with_suffix .au)
```
