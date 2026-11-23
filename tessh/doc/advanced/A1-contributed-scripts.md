# Appendix A. Contributed Scripts
<!-- help: abs-contributed-scripts abs-a appendix-a contributed scripts programs examples -->

The ABS Guide ends its main text with a collection of whole programs sent in
by its readers and by Mendel Cooper himself: a mail formatter, a game of
life, a soundex coder, a hash library and more. They don't fit any one
chapter, but they show what scripts look like when they grow up. Each one
here is the original bash next to a tessh port that does the same job in
tessh's own way: Python values and functions where bash juggles strings,
programs where programs are the right tool.

## mailformat: Formatting an e-mail message
<!-- help: abs-mail-format mailformat mail-format sed fold e-mail -->

A filter for quoted mail: it strips the `>` carets and the leading blanks
and tabs, then folds long lines. The sed script lives in a variable.

### Example A-1. mailformat: Formatting an e-mail message (`mail-format.sh`)

```bash no-run
#!/bin/bash
# mail-format.sh (ver. 1.1): Format e-mail messages.

# Gets rid of carets, tabs, and also folds excessively long lines.

# =================================================================
#                 Standard Check for Script Argument(s)
ARGS=1
E_BADARGS=85
E_NOFILE=86

if [ $# -ne $ARGS ]  # Correct number of arguments passed to script?
then
  echo "Usage: `basename $0` filename"
  exit $E_BADARGS
fi

if [ -f "$1" ]       # Check if file exists.
then
    file_name=$1
else
    echo "File \"$1\" does not exist."
    exit $E_NOFILE
fi
# -----------------------------------------------------------------

MAXWIDTH=70          # Width to fold excessively long lines to.

# =================================
# A variable can hold a sed script.
# It's a useful technique.
sedscript='s/^>//
s/^  *>//
s/^  *//
s/		*//'
# =================================

#  Delete carets and tabs at beginning of lines,
#+ then fold lines to $MAXWIDTH characters.
sed "$sedscript" $1 | fold -s --width=$MAXWIDTH
                        #  -s option to "fold"
                        #+ breaks lines at whitespace, if possible.


#  This script was inspired by an article in a well-known trade journal
#+ extolling a 164K MS Windows utility with similar functionality.
#
#  An nice set of text processing utilities and an efficient
#+ scripting language provide an alternative to the bloated executables
#+ of a clunky operating system.

exit $?
```

The tessh version keeps the sed script in a variable too: a quoted string
is one argument, so `sed $sedscript $file_name` needs no extra quotes. The
argument checks are Python: `len $args`, and `Path.is_file` for `[ -f ]`.

```tessh
# (A made-up message to try it on.)
print ">> Original text, quoted twice.\n  > An indented quote.\n\t\tTabbed in, and so long that it goes well past seventy characters, which is where fold cuts it.\nThe end." >message.txt

# mail-format.tsh (ver. 1.1): Format e-mail messages.
# Gets rid of carets, tabs, and also folds excessively long lines.

ARGS = 1
E_BADARGS = 85
E_NOFILE = 86

args = $argv[1:] or [message.txt]     # (sample arguments, when none are given)

if (len $args) != $ARGS      # Correct number of arguments passed to script?
  print "Usage: $((Path $argv[0]).name) filename"
  exit $E_BADARGS

file_name = Path $args[0]
if not ($file_name.is_file)  # Check if file exists.
  print "File \"$file_name\" does not exist."
  exit $E_NOFILE

MAXWIDTH = 70                # Width to fold excessively long lines to.

# A variable can hold a sed script.
sedscript = 's/^>//
s/^  *>//
s/^  *//
s/\t\t*//'

#  Delete carets and tabs at beginning of lines,
#+ then fold lines to $MAXWIDTH characters.
sed $sedscript $file_name | fold -s --width=$MAXWIDTH
                             # -s: break lines at whitespace, if possible.
exit $?
```

```output
> Original text, quoted twice.
An indented quote.
Tabbed in, and so long that it goes well past seventy characters, 
which is where fold cuts it.
The end.
```

## rn: A simple-minded file renaming utility
<!-- help: abs-rn rn rename-files mv pattern -->

`rn gif jpg` renames every file with `gif` in its name, putting `jpg` in
its place. It's a variation on `lowercase.sh` from chapter 16.

### Example A-2. rn: A simple-minded file renaming utility (`rn.sh`)

```bash no-run
#! /bin/bash
# rn.sh

# Very simpleminded filename "rename" utility (based on "lowercase.sh").
#
#  The "ren" utility, by Vladimir Lanin (lanin@csd2.nyu.edu),
#+ does a much better job of this.


ARGS=2
E_BADARGS=85
ONE=1                     # For getting singular/plural right (see below).

if [ $# -ne "$ARGS" ]
then
  echo "Usage: `basename $0` old-pattern new-pattern"
  # As in "rn gif jpg", which renames all gif files in working directory to jpg.
  exit $E_BADARGS
fi

number=0                  # Keeps track of how many files actually renamed.


for filename in *$1*      #Traverse all matching files in directory.
do
   if [ -f "$filename" ]  # If finds match...
   then
     fname=`basename $filename`            # Strip off path.
     n=`echo $fname | sed -e "s/$1/$2/"`   # Substitute new for old in filename.
     mv $fname $n                          # Rename.
     let "number += 1"
   fi
done   

if [ "$number" -eq "$ONE" ]                # For correct grammar.
then
 echo "$number file renamed."
else 
 echo "$number files renamed."
fi 

exit $?


# Exercises:
# ---------
# What types of files will this not work on?
# How can this be fixed?
```

The script's exercise asks what files it won't work on: names with blanks,
since bash splits `$filename` into words. In tessh a value is always one
argument, so `holiday snap.gif` renames like any other. The glob
`*$old*` is written bare, so it's a glob with the variable in it, and its
items are Paths. `str.replace` with a count of 1 replaces the first `old`,
as `sed s/…/…/` did, but takes it literally: in the bash `rn . _` would
match any character.

```tessh
# (Some made-up files to try it on.)
touch photo1.gif photo2.gif "holiday snap.gif" notes.txt

# rn.tsh: very simpleminded filename "rename" utility (based on "lowercase.sh").

ARGS = 2
E_BADARGS = 85
ONE = 1                      # For getting singular/plural right (see below).

args = $argv[1:] or [gif jpg]         # (sample arguments, when none are given)
if (len $args) != $ARGS
  print "Usage: $((Path $argv[0]).name) old-pattern new-pattern"
  # As in "rn gif jpg", which renames all gif files in working directory to jpg.
  exit $E_BADARGS
[old new] = $args

number = 0                   # Keeps track of how many files actually renamed.

for filename in *$old*       # Traverse all matching files in folder.
  if ($filename.is_file)
    n = $filename.name.replace $old $new 1    # Substitute new for old in name.
    mv $filename $n                            # Rename.
    number += 1

if $number == $ONE           # For correct grammar.
  print "$number file renamed."
else
  print "$number files renamed."

echo *                       # (To see what it did.)
```

```output
3 files renamed.
holiday snap.jpg notes.txt photo1.jpg photo2.jpg
```

## blank-rename: Renames filenames containing blanks
<!-- help: abs-blank-rename blank-rename blanks spaces underscores filenames -->

An even simpler renamer: blanks in file names become underscores.

### Example A-3. blank-rename: Renames filenames containing blanks (`blank-rename.sh`)

```bash no-run
#! /bin/bash
# blank-rename.sh
#
# Substitutes underscores for blanks in all the filenames in a directory.

ONE=1                     # For getting singular/plural right (see below).
number=0                  # Keeps track of how many files actually renamed.
FOUND=0                   # Successful return value.

for filename in *         #Traverse all files in directory.
do
     echo "$filename" | grep -q " "         #  Check whether filename
     if [ $? -eq $FOUND ]                   #+ contains space(s).
     then
       fname=$filename                      # Yes, this filename needs work.
       n=`echo $fname | sed -e "s/ /_/g"`   # Substitute underscore for blank.
       mv "$fname" "$n"                     # Do the actual renaming.
       let "number += 1"
     fi
done   

if [ "$number" -eq "$ONE" ]                 # For correct grammar.
then
 echo "$number file renamed."
else 
 echo "$number files renamed."
fi 

exit 0
```

The bash pipes each name through `grep -q " "` and tests `$?`; in tessh
that's `" " in $filename.name`. There's one difference in the result: bash's
unquoted `echo $fname` squeezes runs of blanks into one, so `two  blanks`
became `two_blanks`. tessh keeps every blank, and so every underscore.

```tessh
# (Some made-up files to try it on.)
touch "my file.txt" "two  blanks here" plain.txt

# blank-rename.tsh
# Substitutes underscores for blanks in all the filenames in a folder.

ONE = 1                      # For getting singular/plural right (see below).
number = 0                   # Keeps track of how many files actually renamed.

for filename in *            # Traverse all files in folder.
  if " " in $filename.name   # Does the name contain blank(s)?
    n = $filename.name.replace " " _   # Substitute underscore for blank.
    mv $filename $n                    # Do the actual renaming.
    number += 1

if $number == $ONE           # For correct grammar.
  print "$number file renamed."
else
  print "$number files renamed."

echo *                       # (To see what it did.)
exit 0
```

```output
2 files renamed.
my_file.txt plain.txt two__blanks_here
```

## encryptedpw: Uploading to an ftp site, using a locally encrypted password
<!-- help: abs-encryptedpw encryptedpw ftp password upload heredoc -->

The `ftp` upload of chapter 19 (`ex72.sh`), with the password kept
encrypted on disk and decrypted only when the script runs.

### Example A-4. encryptedpw: Uploading to an ftp site, using a locally encrypted password (`encryptedpw.sh`)

```bash no-run
#!/bin/bash

# Example "ex72.sh" modified to use encrypted password.

#  Note that this is still rather insecure,
#+ since the decrypted password is sent in the clear.
#  Use something like "ssh" if this is a concern.

E_BADARGS=85

if [ -z "$1" ]
then
  echo "Usage: `basename $0` filename"
  exit $E_BADARGS
fi  

Username=bozo           # Change to suit.
pword=/home/bozo/secret/password_encrypted.file
# File containing encrypted password.

Filename=`basename $1`  # Strips pathname out of file name.

Server="XXX"
Directory="YYY"         # Change above to actual server name & directory.


Password=`cruft <$pword`          # Decrypt password.
#  Uses the author's own "cruft" file encryption package,
#+ based on the classic "onetime pad" algorithm,
#+ and obtainable from:
#+ Primary-site:   ftp://ibiblio.org/pub/Linux/utils/file
#+                 cruft-0.2.tar.gz [16k]


ftp -n $Server <<End-Of-Session
user $Username $Password
binary
bell
cd $Directory
put $Filename
bye
End-Of-Session
# -n option to "ftp" disables auto-logon.
# Note that "bell" rings 'bell' after each file transfer.

exit 0
```

There's no ftp server here, and no `cruft`, so the tessh version shows what
it would send: `rot13` (with `tr`) stands in for the decryption, and a
function named `ftp` stands in for the program and prints the session it's
given. A Python function of a program's name is called instead of it; take
the function away (or write `\ftp`) and the real ftp runs. The heredoc's
word is `EndOfSession` here: tessh stops a heredoc word at a `-`.

```tessh
import sys
# (Made up: a file to send, and the password "secret", "encrypted" with rot13.)
echo "the report" >report.txt
echo frperg >password_encrypted.file
def ftp *args                # (A stand-in for ftp: it shows what it's sent.)
  print "[ftp $(' '.join $args)]"
  print (sys.stdin.read) end=""

#  Note that this is still rather insecure,
#+ since the decrypted password is sent in the clear.
#  Use something like "ssh" if this is a concern.

E_BADARGS = 85

args = $argv[1:] or [./report.txt]    # (sample arguments, when none are given)
if not $args or not $args[0]
  print "Usage: $((Path $argv[0]).name) filename"
  exit $E_BADARGS

Username = bozo              # Change to suit.
pword = ./password_encrypted.file    # (/home/bozo/secret/… in the original)
# File containing encrypted password.

Filename = (Path $args[0]).name      # Strips pathname out of file name.

Server = XXX
Directory = YYY              # Change above to actual server name & folder.

Password = tr a-zA-Z n-za-mN-ZA-M <$pword   # Decrypt password.
# (The original uses the author's own "cruft" one-time-pad package.)

ftp -n $Server <<EndOfSession
user $Username $Password
binary
bell
cd $Directory
put $Filename
bye
EndOfSession
# -n option to "ftp" disables auto-logon.
# Note that "bell" rings 'bell' after each file transfer.

exit 0
```

```output
[ftp -n XXX]
user bozo secret
binary
bell
cd YYY
put report.txt
bye
```

## copy-cd: Copying a data CD
<!-- help: abs-copy-cd copy-cd cdrom dd wodim cdrecord burn -->

Copies a data CD to an image file with `dd`, then burns the image to a
blank with `wodim` (once `cdrecord`), asking the user to swap discs between
the two.

### Example A-5. copy-cd: Copying a data CD (`copy-cd.sh`)

```bash no-run
#!/bin/bash
# copy-cd.sh: copying a data CD

CDROM=/dev/cdrom                           # CD ROM device
OF=/home/bozo/projects/cdimage.iso         # output file
#       /xxxx/xxxxxxxx/                      Change to suit your system.
BLOCKSIZE=2048
# SPEED=10                                 # If unspecified, uses max spd.
# DEVICE=/dev/cdrom                          older version.
DEVICE="1,0,0"

echo; echo "Insert source CD, but do *not* mount it."
echo "Press ENTER when ready. "
read ready                                 # Wait for input, $ready not used.

echo; echo "Copying the source CD to $OF."
echo "This may take a while. Please be patient."

dd if=$CDROM of=$OF bs=$BLOCKSIZE          # Raw device copy.


echo; echo "Remove data CD."
echo "Insert blank CDR."
echo "Press ENTER when ready. "
read ready                                 # Wait for input, $ready not used.

echo "Copying $OF to CDR."

# cdrecord -v -isosize speed=$SPEED dev=$DEVICE $OF   # Old version.
wodim -v -isosize dev=$DEVICE $OF
# Uses Joerg Schilling's "cdrecord" package (see its docs).
# http://www.fokus.gmd.de/nthp/employees/schilling/cdrecord.html
# Newer Linux distros may use "wodim" rather than "cdrecord" ...


echo; echo "Done copying $OF to CDR on device $CDROM."

echo "Do you want to erase the image file (y/n)? "  # Probably a huge file.
read answer

case "$answer" in
[yY]) rm -f $OF
      echo "$OF erased."
      ;;
*)    echo "$OF not erased.";;
esac

echo

# Exercise:
# Change the above "case" statement to also accept "yes" and "Yes" as input.

exit 0
```

This one needs a CD drive and someone at the keyboard, so the tessh version
doesn't run here. `input` is bash's `read`, and the `case` on the answer is
a `match`; the exercise's "also accept yes and Yes" is one more `|` in the
pattern (and `.lower` would take `YES` too).

```tessh no-run
# copy-cd.tsh: copying a data CD

CDROM = /dev/cdrom                       # CD ROM device
OF = /home/bozo/projects/cdimage.iso     # output file
#     /xxxx/xxxxxxxx/                      Change to suit your system.
BLOCKSIZE = 2048
# SPEED = 10                             # If unspecified, uses max spd.
DEVICE = "1,0,0"

print ; print "Insert source CD, but do *not* mount it."
input "Press ENTER when ready. "        # Wait for input.

print ; print "Copying the source CD to $OF."
print "This may take a while. Please be patient."

dd if=$CDROM of=$OF bs=$BLOCKSIZE        # Raw device copy.

print ; print "Remove data CD."
print "Insert blank CDR."
input "Press ENTER when ready. "

print "Copying $OF to CDR."

wodim -v -isosize dev=$DEVICE $OF
# Uses Joerg Schilling's "cdrecord" package (see its docs).
# Newer Linux distros may use "wodim" rather than "cdrecord" ...

print ; print "Done copying $OF to CDR on device $CDROM."

answer = input "Do you want to erase the image file (y/n)? "  # Probably a huge file.

match $answer
  case y | Y | yes | Yes
    rm -f $OF
    print "$OF erased."
  case _
    print "$OF not erased."

print
exit 0
```
