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
110
458
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
