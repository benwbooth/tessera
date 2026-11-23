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

## Collatz series
<!-- help: abs-collatz collatz hailstone series iterate 3n+1 -->

The "hailstone" series: halve an even number, triple an odd one and add 1,
and go on. Every seed anyone has tried ends up cycling through 4, 2, 1.

### Example A-6. Collatz series (`collatz.sh`)

```bash no-run
#!/bin/bash
# collatz.sh

#  The notorious "hailstone" or Collatz series.
#  -------------------------------------------
#  1) Get the integer "seed" from the command-line.
#  2) NUMBER <-- seed
#  3) Print NUMBER.
#  4)  If NUMBER is even, divide by 2, or
#  5)+ if odd, multiply by 3 and add 1.
#  6) NUMBER <-- result 
#  7) Loop back to step 3 (for specified number of iterations).
#
#  The theory is that every such sequence,
#+ no matter how large the initial value,
#+ eventually settles down to repeating "4,2,1..." cycles,
#+ even after fluctuating through a wide range of values.
#
#  This is an instance of an "iterate,"
#+ an operation that feeds its output back into its input.
#  Sometimes the result is a "chaotic" series.


MAX_ITERATIONS=200
# For large seed numbers (>32000), try increasing MAX_ITERATIONS.

h=${1:-$$}                      #  Seed.
                                #  Use $PID as seed,
                                #+ if not specified as command-line arg.

echo
echo "C($h) -*- $MAX_ITERATIONS Iterations"
echo

for ((i=1; i<=MAX_ITERATIONS; i++))
do

# echo -n "$h	"
#            ^^^ 
#            tab
# printf does it better ...
COLWIDTH=%7d
printf $COLWIDTH $h

  let "remainder = h % 2"
  if [ "$remainder" -eq 0 ]   # Even?
  then
    let "h /= 2"              # Divide by 2.
  else
    let "h = h*3 + 1"         # Multiply by 3 and add 1.
  fi


COLUMNS=10                    # Output 10 values per line.
let "line_break = i % $COLUMNS"
if [ "$line_break" -eq 0 ]
then
  echo
fi  

done

echo

#  For more information on this strange mathematical function,
#+ see _Computers, Pattern, Chaos, and Beauty_, by Pickover, p. 185 ff.,
#+ as listed in the bibliography.

exit 0
```

In tessh the series is a generator, an endless one, as the mathematics
says; `take` stops it after `MAX_ITERATIONS` values, and `partition_all`
cuts those into rows of ten. `"${n:7d}"` is the `printf %7d`. The seed is
the first argument or, here, 27 (the bash takes its own PID, which would
make the output different every run).

```tessh
# collatz.tsh

#  The notorious "hailstone" or Collatz series.
#  1) Get the integer "seed" from the command-line.
#  2) NUMBER <-- seed
#  3) Print NUMBER.
#  4)  If NUMBER is even, divide by 2, or
#  5)+ if odd, multiply by 3 and add 1.
#  6) NUMBER <-- result
#  7) Loop back to step 3 (for specified number of iterations).

MAX_ITERATIONS = 200
COLUMNS = 10                 # Output 10 values per line.

h = int (if (len $argv) > 1 then $argv[1] else 27)   # Seed.
                             # (The original falls back on its PID.)

def hailstone n
  while True
    yield $n
    n = if $n % 2 == 0 then $n // 2 else $n * 3 + 1

print
print "C($h) -*- $MAX_ITERATIONS Iterations"
print

for row in (partition_all $COLUMNS (take $MAX_ITERATIONS (hailstone $h)))
  print (''.join ["${n:7d}" for n in $row])

print

#  For more information on this strange mathematical function,
#+ see _Computers, Pattern, Chaos, and Beauty_, by Pickover, p. 185 ff.
```

```output

C(27) -*- 200 Iterations

     27     82     41    124     62     31     94     47    142     71
    214    107    322    161    484    242    121    364    182     91
    274    137    412    206    103    310    155    466    233    700
    350    175    526    263    790    395   1186    593   1780    890
    445   1336    668    334    167    502    251    754    377   1132
    566    283    850    425   1276    638    319    958    479   1438
    719   2158   1079   3238   1619   4858   2429   7288   3644   1822
    911   2734   1367   4102   2051   6154   3077   9232   4616   2308
   1154    577   1732    866    433   1300    650    325    976    488
    244    122     61    184     92     46     23     70     35    106
     53    160     80     40     20     10      5     16      8      4
      2      1      4      2      1      4      2      1      4      2
      1      4      2      1      4      2      1      4      2      1
      4      2      1      4      2      1      4      2      1      4
      2      1      4      2      1      4      2      1      4      2
      1      4      2      1      4      2      1      4      2      1
      4      2      1      4      2      1      4      2      1      4
      2      1      4      2      1      4      2      1      4      2
      1      4      2      1      4      2      1      4      2      1
      4      2      1      4      2      1      4      2      1      4

```

## days-between: Days between two dates
<!-- help: abs-days-between days-between dates gauss date-difference datetime -->

The number of days between two dates, by Gauss' formula for the days since
March 1, 1600.

### Example A-7. days-between: Days between two dates (`days-between.sh`)

```bash no-run
#!/bin/bash
# days-between.sh:    Number of days between two dates.
# Usage: ./days-between.sh [M]M/[D]D/YYYY [M]M/[D]D/YYYY
#
# Note: Script modified to account for changes in Bash, v. 2.05b +,
#+      that closed the loophole permitting large negative
#+      integer return values.

ARGS=2                # Two command-line parameters expected.
E_PARAM_ERR=85        # Param error.

REFYR=1600            # Reference year.
CENTURY=100
DIY=365
ADJ_DIY=367           # Adjusted for leap year + fraction.
MIY=12
DIM=31
LEAPCYCLE=4

MAXRETVAL=255         #  Largest permissible
                      #+ positive return value from a function.

diff=                 # Declare global variable for date difference.
value=                # Declare global variable for absolute value.
day=                  # Declare globals for day, month, year.
month=
year=


Param_Error ()        # Command-line parameters wrong.
{
  echo "Usage: `basename $0` [M]M/[D]D/YYYY [M]M/[D]D/YYYY"
  echo "       (date must be after 1/3/1600)"
  exit $E_PARAM_ERR
}  


Parse_Date ()                 # Parse date from command-line params.
{
  month=${1%%/**}
  dm=${1%/**}                 # Day and month.
  day=${dm#*/}
  let "year = `basename $1`"  # Not a filename, but works just the same.
}  


check_date ()                 # Checks for invalid date(s) passed.
{
  [ "$day" -gt "$DIM" ] || [ "$month" -gt "$MIY" ] ||
  [ "$year" -lt "$REFYR" ] && Param_Error
  # Exit script on bad value(s).
  # Uses or-list / and-list.
  #
  # Exercise: Implement more rigorous date checking.
}


strip_leading_zero () #  Better to strip possible leading zero(s)
{                     #+ from day and/or month
  return ${1#0}       #+ since otherwise Bash will interpret them
}                     #+ as octal values (POSIX.2, sect 2.9.2.1).


day_index ()          # Gauss' Formula:
{                     # Days from March 1, 1600 to date passed as param.
                      #           ^^^^^^^^^^^^^
  day=$1
  month=$2
  year=$3

  let "month = $month - 2"
  if [ "$month" -le 0 ]
  then
    let "month += 12"
    let "year -= 1"
  fi  

  let "year -= $REFYR"
  let "indexyr = $year / $CENTURY"


  let "Days = $DIY*$year + $year/$LEAPCYCLE - $indexyr \
              + $indexyr/$LEAPCYCLE + $ADJ_DIY*$month/$MIY + $day - $DIM"
  #  For an in-depth explanation of this algorithm, see
  #+   http://weblogs.asp.net/pgreborio/archive/2005/01/06/347968.aspx


  echo $Days

}  


calculate_difference ()            # Difference between two day indices.
{
  let "diff = $1 - $2"             # Global variable.
}  


abs ()                             #  Absolute value
{                                  #  Uses global "value" variable.
  if [ "$1" -lt 0 ]                #  If negative
  then                             #+ then
    let "value = 0 - $1"           #+ change sign,
  else                             #+ else
    let "value = $1"               #+ leave it alone.
  fi
}



if [ $# -ne "$ARGS" ]              # Require two command-line params.
then
  Param_Error
fi  

Parse_Date $1
check_date $day $month $year       #  See if valid date.

strip_leading_zero $day            #  Remove any leading zeroes
day=$?                             #+ on day and/or month.
strip_leading_zero $month
month=$?

let "date1 = `day_index $day $month $year`"


Parse_Date $2
check_date $day $month $year

strip_leading_zero $day
day=$?
strip_leading_zero $month
month=$?

date2=$(day_index $day $month $year) # Command substitution.


calculate_difference $date1 $date2

abs $diff                            # Make sure it's positive.
diff=$value

echo $diff

exit 0

#  Exercise:
#  --------
#  If given only one command-line parameter, have the script
#+ use today's date as the second.


#  Compare this script with
#+ the implementation of Gauss' Formula in a C program at
#+    http://buschencrew.hypermart.net/software/datedif
```

A bash function can only give back a number from 0 to 255 as its status,
so the bash hands results back in global variables, in `$?` (to strip a
leading zero) and by echoing them. Python functions return any value, so in
tessh each function returns what it computes. `int "08"` is 8: there's no
octal trap to strip zeros for. Python's `datetime` knows the calendar, so
the last line checks Gauss' answer against it.

```tessh
import datetime
# days-between.tsh:    Number of days between two dates.
# Usage: days-between.tsh [M]M/[D]D/YYYY [M]M/[D]D/YYYY

ARGS = 2                     # Two command-line parameters expected.
E_PARAM_ERR = 85             # Param error.

REFYR = 1600                 # Reference year.
CENTURY = 100
DIY = 365
ADJ_DIY = 367                # Adjusted for leap year + fraction.
MIY = 12
DIM = 31
LEAPCYCLE = 4

def Param_Error              # Command-line parameters wrong.
  print "Usage: $((Path $argv[0]).name) [M]M/[D]D/YYYY [M]M/[D]D/YYYY"
  print "       (date must be after 1/3/1600)"
  exit $E_PARAM_ERR

def Parse_Date text          # Parse date from command-line params.
  [month day year] = [int $part for part in ($text.split "/")]
  [$day $month $year]

def check_date day month year     # Checks for invalid date(s) passed.
  if $day > $DIM or $month > $MIY or $year < $REFYR
    Param_Error              # Exit script on bad value(s).

def day_index day month year # Gauss' Formula:
                             # Days from March 1, 1600 to date passed.
  month -= 2
  if $month <= 0
    month += 12
    year -= 1
  year -= $REFYR
  indexyr = $year // $CENTURY
  ($DIY * $year + $year // $LEAPCYCLE - $indexyr + $indexyr // $LEAPCYCLE
   + $ADJ_DIY * $month // $MIY + $day - $DIM)

args = $argv[1:] or ["7/4/1776" "11/23/2026"]   # (sample arguments, when none are given)

if (len $args) != $ARGS      # Require two command-line params.
  Param_Error

[day month year] = Parse_Date $args[0]
check_date $day $month $year # See if valid date.
first_index = day_index $day $month $year

[day month year] = Parse_Date $args[1]
check_date $day $month $year
second_index = day_index $day $month $year

diff = abs ($first_index - $second_index)   # Make sure it's positive.
print $diff

# (Python's datetime agrees.)
[a b] = [(datetime.date $y $m $d) for [d m y] in [(Parse_Date $s) for s in $args]]
print (abs ($b - $a).days)

exit 0
```

```output
91452
91452
```

## Making a dictionary
<!-- help: abs-makedict makedict dictionary words tr sort uniq word-list -->

A filter after Alec Muffett's `mkdict` from the Crack package: every word
of the files given, lowercased, sorted, once each.

### Example A-8. Making a dictionary (`makedict.sh`)

```bash no-run
#!/bin/bash
# makedict.sh  [make dictionary]

# Modification of /usr/sbin/mkdict (/usr/sbin/cracklib-forman) script.
# Original script copyright 1993, by Alec Muffett.
#
#  This modified script included in this document in a manner
#+ consistent with the "LICENSE" document of the "Crack" package
#+ that the original script is a part of.

#  This script processes text files to produce a sorted list
#+ of words found in the files.
#  This may be useful for compiling dictionaries
#+ and for other lexicographic purposes.


E_BADARGS=85

if [ ! -r "$1" ]                    #  Need at least one
then                                #+ valid file argument.
  echo "Usage: $0 files-to-process"
  exit $E_BADARGS
fi  


# SORT="sort"                       #  No longer necessary to define
                                    #+ options to sort. Changed from
                                    #+ original script.

cat $* |                            #  Dump specified files to stdout.
        tr A-Z a-z |                #  Convert to lowercase.
        tr ' ' '\012' |             #  New: change spaces to newlines.
#       tr -cd '\012[a-z][0-9]' |   #  Get rid of everything
                                    #+ non-alphanumeric (in orig. script).
        tr -c '\012a-z'  '\012' |   #  Rather than deleting non-alpha
                                    #+ chars, change them to newlines.
        sort |                      #  $SORT options unnecessary now.
        uniq |                      #  Remove duplicates.
        grep -v '^#' |              #  Delete lines starting with #.
        grep -v '^$'                #  Delete blank lines.

exit $?
```

The pipeline is the same in tessh, programs and all. A line can't end in
`|` in tessh, so each stage after the first starts a line continued with
`\`, and the comments go above. `*$files` puts each file in as its own
argument, as bash's `$*` did.

```tessh
# (A made-up text to try it on.)
print "The Cat sat on the mat.\nThe cat's hat, 2 times; the end!" >text.txt

import os
# makedict.tsh  [make dictionary]
# Modification of /usr/sbin/mkdict (/usr/sbin/cracklib-forman) script.
# Original script copyright 1993, by Alec Muffett.

E_BADARGS = 85

files = $argv[1:] or [text.txt]       # (sample arguments, when none are given)

if not $files or not (os.access $files[0] $os.R_OK)   # Need at least one
  print "Usage: $((Path $argv[0]).name) files-to-process"  # valid file argument.
  exit $E_BADARGS

# Dump the files, lowercase them, put each word on a line of its own,
# change every other character to a newline, then sort out the words.
cat *$files \
  | tr A-Z a-z \
  | tr ' ' '\012' \
  | tr -c '\012a-z' '\012' \
  | sort \
  | uniq \
  | grep -v '^#' \
  | grep -v '^$'

exit $?
```

```output
cat
end
hat
mat
on
s
sat
the
times
```

The same in Python, for comparison: `re.findall` picks out the runs of
letters, and a set takes the duplicates out.

```tessh
import re
print "The Cat sat on the mat.\nThe cat's hat, 2 times; the end!" >text.txt

text = ((Path text.txt).read_text)
print (' '.join (sorted (set (re.findall '[a-z]+' ($text.lower)))))
```

```output
cat end hat mat on s sat the times
```

## Soundex conversion
<!-- help: abs-soundex soundex names sound-alike tr translate genealogy -->

Mendel Cooper's soundex coder. Soundex files names by how they sound: the
first letter, then three digits for the consonants after it, so Smith and
Smythe both come out as S530.

### Example A-9. Soundex conversion (`soundex.sh`)

```bash no-run
#!/bin/bash
# soundex.sh: Calculate "soundex" code for names

# =======================================================
#        Soundex script
#              by
#         Mendel Cooper
#     thegrendel.abs@gmail.com
#     reldate: 23 January, 2002
#
#   Placed in the Public Domain.
#
# A slightly different version of this script appeared in
#+ Ed Schaefer's July, 2002 "Shell Corner" column
#+ in "Unix Review" on-line,
#+ http://www.unixreview.com/documents/uni1026336632258/
# =======================================================


ARGCOUNT=1                     # Need name as argument.
E_WRONGARGS=90

if [ $# -ne "$ARGCOUNT" ]
then
  echo "Usage: `basename $0` name"
  exit $E_WRONGARGS
fi  


assign_value ()                #  Assigns numerical value
{                              #+ to letters of name.

  val1=bfpv                    # 'b,f,p,v' = 1
  val2=cgjkqsxz                # 'c,g,j,k,q,s,x,z' = 2
  val3=dt                      #  etc.
  val4=l
  val5=mn
  val6=r

# Exceptionally clever use of 'tr' follows.
# Try to figure out what is going on here.

value=$( echo "$1" \
| tr -d wh \
| tr $val1 1 | tr $val2 2 | tr $val3 3 \
| tr $val4 4 | tr $val5 5 | tr $val6 6 \
| tr -s 123456 \
| tr -d aeiouy )

# Assign letter values.
# Remove duplicate numbers, except when separated by vowels.
# Ignore vowels, except as separators, so delete them last.
# Ignore 'w' and 'h', even as separators, so delete them first.
#
# The above command substitution lays more pipe than a plumber <g>.

}  


input_name="$1"
echo
echo "Name = $input_name"


# Change all characters of name input to lowercase.
# ------------------------------------------------
name=$( echo $input_name | tr A-Z a-z )
# ------------------------------------------------
# Just in case argument to script is mixed case.


# Prefix of soundex code: first letter of name.
# --------------------------------------------


char_pos=0                     # Initialize character position. 
prefix0=${name:$char_pos:1}
prefix=`echo $prefix0 | tr a-z A-Z`
                               # Uppercase 1st letter of soundex.

let "char_pos += 1"            # Bump character position to 2nd letter of name.
name1=${name:$char_pos}


# ++++++++++++++++++++++++++ Exception Patch ++++++++++++++++++++++++++++++
#  Now, we run both the input name and the name shifted one char
#+ to the right through the value-assigning function.
#  If we get the same value out, that means that the first two characters
#+ of the name have the same value assigned, and that one should cancel.
#  However, we also need to test whether the first letter of the name
#+ is a vowel or 'w' or 'h', because otherwise this would bollix things up.

char1=`echo $prefix | tr A-Z a-z`    # First letter of name, lowercased.

assign_value $name
s1=$value
assign_value $name1
s2=$value
assign_value $char1
s3=$value
s3=9$s3                              #  If first letter of name is a vowel
                                     #+ or 'w' or 'h',
                                     #+ then its "value" will be null (unset).
				     #+ Therefore, set it to 9, an otherwise
				     #+ unused value, which can be tested for.


if [[ "$s1" -ne "$s2" || "$s3" -eq 9 ]]
then
  suffix=$s2
else  
  suffix=${s2:$char_pos}
fi  
# ++++++++++++++++++++++ end Exception Patch ++++++++++++++++++++++++++++++


padding=000                    # Use at most 3 zeroes to pad.


soun=$prefix$suffix$padding    # Pad with zeroes.

MAXLEN=4                       # Truncate to maximum of 4 chars.
soundex=${soun:0:$MAXLEN}

echo "Soundex = $soundex"

echo

#  The soundex code is a method of indexing and classifying names
#+ by grouping together the ones that sound alike.
#  The soundex code for a given name is the first letter of the name,
#+ followed by a calculated three-number code.
#  Similar sounding names should have almost the same soundex codes.

#   Examples:
#   Smith and Smythe both have a "S-530" soundex.
#   Harrison = H-625
#   Hargison = H-622
#   Harriman = H-655

#  This works out fairly well in practice, but there are numerous anomalies.
#
#
#  The U.S. Census and certain other governmental agencies use soundex,
#  as do genealogical researchers.
#
#  For more information,
#+ see the "National Archives and Records Administration home page",
#+ http://www.nara.gov/genealogy/soundex/soundex.html



# Exercise:
# --------
# Simplify the "Exception Patch" section of this script.

exit 0
```

The bash's "exceptionally clever" `tr` pipeline is three Python steps on a
string in tessh: `str.translate` with a table built from the letter groups,
a regex with a back-reference for `tr -s` (squeeze repeated digits), and
another to delete the vowels. Each function returns its code, where bash
leaves it in the global `value`. The script takes one name; here it's
called on the names from its own comments.

```tessh
# soundex.tsh: Calculate "soundex" code for names

# =======================================================
#        Soundex script
#              by
#         Mendel Cooper
#     thegrendel.abs@gmail.com
#     reldate: 23 January, 2002
#
#   Placed in the Public Domain.
# =======================================================

ARGCOUNT = 1                 # Need name as argument.
E_WRONGARGS = 90

# 'b,f,p,v' = 1, 'c,g,j,k,q,s,x,z' = 2, etc.
groups = [bfpv cgjkqsxz dt l mn r]
VALUES = str.maketrans {$c: (str $n) for [n letters] in (enumerate $groups 1) for c in $letters}
DROP_WH = str.maketrans "" "" wh

def assign_value name        #  Assigns numerical value
                             #+ to letters of name.
  value = $name.translate $DROP_WH         # Ignore 'w' and 'h', even as
                                           # separators: delete them first.
  value = $value.translate $VALUES         # Assign letter values.
  value = re'([1-6])\1+'.sub '\1' $value   # Remove duplicate numbers,
                                           # except when separated by vowels.
  re'[aeiouy]'.sub '' $value               # Ignore vowels, except as
                                           # separators: delete them last.

def soundex input_name
  name = ($input_name.lower) # Just in case argument is mixed case.

  # Prefix of soundex code: first letter of name.
  prefix = ($name[0].upper)
  name1 = $name[1:]

  # Exception patch: run both the name and the name shifted one letter
  # through the value-assigning function. If we get the same value out,
  # the first two letters have the same value, and one should cancel,
  # unless the first letter is a vowel or 'w' or 'h' (no value: "9").
  char1 = ($prefix.lower)
  s1 = assign_value $name
  s2 = assign_value $name1
  s3 = "9" + (assign_value $char1)

  suffix = if $s1 != $s2 or $s3 == "9" then $s2 else $s2[1:]

  padding = "000"            # Use at most 3 zeroes to pad.
  soun = $prefix + $suffix + $padding

  MAXLEN = 4                 # Truncate to maximum of 4 chars.
  $soun[:$MAXLEN]

def main args
  if (len $args) != $ARGCOUNT
    print "Usage: $((Path $argv[0]).name) name"
    exit $E_WRONGARGS
  input_name = $args[0]
  print
  print "Name = $input_name"
  print "Soundex = $(soundex $input_name)"

if (len $argv) > 1
  main $argv[1:]
else                         # (sample names, when none are given)
  for name in [Smith Smythe Harrison Hargison Harriman Ashcraft]
    main [$name]
print

#   Examples:
#   Smith and Smythe both have a "S-530" soundex.
#   Harrison = H-625
#   Hargison = H-622
#   Harriman = H-655
```

```output

Name = Smith
Soundex = S530

Name = Smythe
Soundex = S530

Name = Harrison
Soundex = H625

Name = Hargison
Soundex = H622

Name = Harriman
Soundex = H655

Name = Ashcraft
Soundex = A261

```

The script's exercise is to simplify the exception patch. Run the whole
name through `assign_value`: when the first letter has a value, the code
starts with it (and has already swallowed a repeat of it), so drop that
digit; when it hasn't, there's nothing to drop.

```tessh
groups = [bfpv cgjkqsxz dt l mn r]
VALUES = str.maketrans {$c: (str $n) for [n letters] in (enumerate $groups 1) for c in $letters}
DROP_WH = str.maketrans "" "" wh

def soundex name
  name = ($name.lower)
  code = -> $name (.translate $DROP_WH) (.translate $VALUES)
  code = ->> $code (re'(\d)\1+'.sub '\1') (re'[aeiouy]'.sub '')
  code = if $name[0] in "aeiouywh" then $code else $code[1:]
  (($name[0].upper) + $code + "000")[:4]

print (' '.join [(soundex $n) for n in [Smith Smythe Harrison Hargison Harriman Ashcraft Pfister Lloyd]])
```

```output
S530 S530 H625 H622 H655 A261 P236 L300
```

## Game of Life
<!-- help: abs-life life game-of-life conway cellular-automata gen0 -->

Mendel Cooper's "Life in the Slow Lane", with patches by Daniel Albers:
John Conway's Game of Life on a 10 × 10 grid. A live cell with two or three
live neighbors lives on, a dead cell with three comes alive, and every
other cell is dead in the next generation. The starting grid is read from
the file `gen0` (Example A-11).

### Example A-10. Game of Life (`life.sh`)

```bash no-run
#!/bin/bash
# life.sh: "Life in the Slow Lane"
# Author: Mendel Cooper
# License: GPL3

# Version 0.2:   Patched by Daniel Albers
#+               to allow non-square grids as input.
# Version 0.2.1: Added 2-second delay between generations.

# ##################################################################### #
# This is the Bash script version of John Conway's "Game of Life".      #
# "Life" is a simple implementation of cellular automata.               #
# --------------------------------------------------------------------- #
# On a rectangular grid, let each "cell" be either "living" or "dead."  #
# Designate a living cell with a dot, and a dead one with a blank space.#
#      Begin with an arbitrarily drawn dot-and-blank grid,              #
#+     and let this be the starting generation: generation 0.           #
# Determine each successive generation by the following rules:          #
#   1) Each cell has 8 neighbors, the adjoining cells                   #
#+     left, right, top, bottom, and the 4 diagonals.                   #
#                                                                       #
#                       123                                             #
#                       4*5     The * is the cell under consideration.  #
#                       678                                             #
#                                                                       #
# 2) A living cell with either 2 or 3 living neighbors remains alive.   #
SURVIVE=2                                                               #
# 3) A dead cell with 3 living neighbors comes alive, a "birth."        #
BIRTH=3                                                                 #
# 4) All other cases result in a dead cell for the next generation.     #
# ##################################################################### #


startfile=gen0   # Read the starting generation from the file "gen0" ...
                 # Default, if no other file specified when invoking script.
                 #
if [ -n "$1" ]   # Specify another "generation 0" file.
then
    startfile="$1"
fi  

############################################
#  Abort script if "startfile" not specified
#+ and
#+ default file "gen0" not present.

E_NOSTARTFILE=86

if [ ! -e "$startfile" ]
then
  echo "Startfile \""$startfile"\" missing!"
  exit $E_NOSTARTFILE
fi
############################################


ALIVE1=.
DEAD1=_
                 # Represent living and dead cells in the start-up file.

#  -----------------------------------------------------#
#  This script uses a 10 x 10 grid (may be increased,
#+ but a large grid will slow down execution).
ROWS=10
COLS=10
#  Change above two variables to match desired grid size.
#  -----------------------------------------------------#

GENERATIONS=10          #  How many generations to cycle through.
                        #  Adjust this upwards
                        #+ if you have time on your hands.

NONE_ALIVE=85           #  Exit status on premature bailout,
                        #+ if no cells left alive.
DELAY=2                 #  Pause between generations.
TRUE=0
FALSE=1
ALIVE=0
DEAD=1

avar=                   # Global; holds current generation.
generation=0            # Initialize generation count.

# =================================================================

let "cells = $ROWS * $COLS"   # How many cells.

# Arrays containing "cells."
declare -a initial
declare -a current

display ()
{

alive=0                 # How many cells alive at any given time.
                        # Initially zero.

declare -a arr
arr=( `echo "$1"` )     # Convert passed arg to array.

element_count=${#arr[*]}

local i
local rowcheck

for ((i=0; i<$element_count; i++))
do

  # Insert newline at end of each row.
  let "rowcheck = $i % COLS"
  if [ "$rowcheck" -eq 0 ]
  then
    echo                # Newline.
    echo -n "      "    # Indent.
  fi  

  cell=${arr[i]}

  if [ "$cell" = . ]
  then
    let "alive += 1"
  fi  

  echo -n "$cell" | sed -e 's/_/ /g'
  # Print out array, changing underscores to spaces.
done  

return

}

IsValid ()                            # Test if cell coordinate valid.
{

  if [ -z "$1"  -o -z "$2" ]          # Mandatory arguments missing?
  then
    return $FALSE
  fi

local row
local lower_limit=0                   # Disallow negative coordinate.
local upper_limit
local left
local right

let "upper_limit = $ROWS * $COLS - 1" # Total number of cells.


if [ "$1" -lt "$lower_limit" -o "$1" -gt "$upper_limit" ]
then
  return $FALSE                       # Out of array bounds.
fi  

row=$2
let "left = $row * $COLS"             # Left limit.
let "right = $left + $COLS - 1"       # Right limit.

if [ "$1" -lt "$left" -o "$1" -gt "$right" ]
then
  return $FALSE                       # Beyond row boundary.
fi  

return $TRUE                          # Valid coordinate.

}  


IsAlive ()              #  Test whether cell is alive.
                        #  Takes array, cell number, and
{                       #+ state of cell as arguments.
  GetCount "$1" $2      #  Get alive cell count in neighborhood.
  local nhbd=$?

  if [ "$nhbd" -eq "$BIRTH" ]  # Alive in any case.
  then
    return $ALIVE
  fi

  if [ "$3" = "." -a "$nhbd" -eq "$SURVIVE" ]
  then                  # Alive only if previously alive.
    return $ALIVE
  fi  

  return $DEAD          # Defaults to dead.

}  


GetCount ()             # Count live cells in passed cell's neighborhood.
                        # Two arguments needed:
			# $1) variable holding array
			# $2) cell number
{
  local cell_number=$2
  local array
  local top
  local center
  local bottom
  local r
  local row
  local i
  local t_top
  local t_cen
  local t_bot
  local count=0
  local ROW_NHBD=3

  array=( `echo "$1"` )

  let "top = $cell_number - $COLS - 1"    # Set up cell neighborhood.
  let "center = $cell_number - 1"
  let "bottom = $cell_number + $COLS - 1"
  let "r = $cell_number / $COLS"

  for ((i=0; i<$ROW_NHBD; i++))           # Traverse from left to right. 
  do
    let "t_top = $top + $i"
    let "t_cen = $center + $i"
    let "t_bot = $bottom + $i"


    let "row = $r"                        # Count center row.
    IsValid $t_cen $row                   # Valid cell position?
    if [ $? -eq "$TRUE" ]
    then
      if [ ${array[$t_cen]} = "$ALIVE1" ] # Is it alive?
      then                                # If yes, then ...
        let "count += 1"                  # Increment count.
      fi	
    fi  

    let "row = $r - 1"                    # Count top row.          
    IsValid $t_top $row
    if [ $? -eq "$TRUE" ]
    then
      if [ ${array[$t_top]} = "$ALIVE1" ] # Redundancy here.
      then                                # Can it be optimized?
        let "count += 1"
      fi	
    fi  

    let "row = $r + 1"                    # Count bottom row.
    IsValid $t_bot $row
    if [ $? -eq "$TRUE" ]
    then
      if [ ${array[$t_bot]} = "$ALIVE1" ] 
      then
        let "count += 1"
      fi	
    fi  

  done  


  if [ ${array[$cell_number]} = "$ALIVE1" ]
  then
    let "count -= 1"        #  Make sure value of tested cell itself
  fi                        #+ is not counted.


  return $count
  
}

next_gen ()               # Update generation array.
{

local array
local i=0

array=( `echo "$1"` )     # Convert passed arg to array.

while [ "$i" -lt "$cells" ]
do
  IsAlive "$1" $i ${array[$i]}   # Is the cell alive?
  if [ $? -eq "$ALIVE" ]
  then                           #  If alive, then
    array[$i]=.                  #+ represent the cell as a period.
  else  
    array[$i]="_"                #  Otherwise underscore
   fi                            #+ (will later be converted to space).
  let "i += 1" 
done   


#    let "generation += 1"       # Increment generation count.
###  Why was the above line commented out?


# Set variable to pass as parameter to "display" function.
avar=`echo ${array[@]}`   # Convert array back to string variable.
display "$avar"           # Display it.
echo; echo
echo "Generation $generation  -  $alive alive"

if [ "$alive" -eq 0 ]
then
  echo
  echo "Premature exit: no more cells alive!"
  exit $NONE_ALIVE        #  No point in continuing
fi                        #+ if no live cells.

}


# =========================================================

# main ()
# {

# Load initial array with contents of startup file.
initial=( `cat "$startfile" | sed -e '/#/d' | tr -d '\n' |\
# Delete lines containing '#' comment character.
           sed -e 's/\./\. /g' -e 's/_/_ /g'` )
# Remove linefeeds and insert space between elements.

clear          # Clear screen.

echo #         Title
setterm -reverse on
echo "======================="
setterm -reverse off
echo "    $GENERATIONS generations"
echo "           of"
echo "\"Life in the Slow Lane\""
setterm -reverse on
echo "======================="
setterm -reverse off

sleep $DELAY   # Display "splash screen" for 2 seconds.


# -------- Display first generation. --------
Gen0=`echo ${initial[@]}`
display "$Gen0"           # Display only.
echo; echo
echo "Generation $generation  -  $alive alive"
sleep $DELAY
# -------------------------------------------


let "generation += 1"     # Bump generation count.
echo

# ------- Display second generation. -------
Cur=`echo ${initial[@]}`
next_gen "$Cur"          # Update & display.
sleep $DELAY
# ------------------------------------------

let "generation += 1"     # Increment generation count.

# ------ Main loop for displaying subsequent generations ------
while [ "$generation" -le "$GENERATIONS" ]
do
  Cur="$avar"
  next_gen "$Cur"
  let "generation += 1"
  sleep $DELAY
done
# ==============================================================

echo
# }

exit 0   # CEOF:EOF



# The grid in this script has a "boundary problem."
# The the top, bottom, and sides border on a void of dead cells.
# Exercise: Change the script to have the grid wrap around,
# +         so that the left and right sides will "touch,"      
# +         as will the top and bottom.
#
# Exercise: Create a new "gen0" file to seed this script.
#           Use a 12 x 16 grid, instead of the original 10 x 10 one.
#           Make the necessary changes to the script,
#+          so it will run with the altered file.
#
# Exercise: Modify this script so that it can determine the grid size
#+          from the "gen0" file, and set any variables necessary
#+          for the script to run.
#           This would make unnecessary any changes to variables
#+          in the script for an altered grid size.
#
# Exercise: Optimize this script.
#           It has redundant code.
```

Bash has no two-dimensional arrays (nor arrays it can pass to a function),
so the bash keeps the grid as one string of 100 words, turns it back into
an array in every function, and returns neighbor counts as exit statuses.
In tessh the grid is a list of rows of booleans, and the rules are a line
each. The grid's size comes from the file, which is the script's third
exercise. It clears the screen, shows the title in reverse video and waits
two seconds between generations only when it's on a terminal; here it
isn't, so it doesn't. The rows are printed without trailing blanks.

```tessh
import sys time
# (The gen0 file of Example A-11, to start from.)
cat <<'END' >gen0
# gen0
#
# This is an example "generation 0" start-up file for "life.sh".
# Lines beginning with a '#' are comments, and the script ignores them.
__.__..___
__.._.____
____.___..
_._______.
____._____
..__...___
____._____
___...____
__.._..___
_..___..__
END

# life.tsh: "Life in the Slow Lane"
# Author: Mendel Cooper
# License: GPL3

# Version 0.2:   Patched by Daniel Albers
#+               to allow non-square grids as input.
# Version 0.2.1: Added 2-second delay between generations.

# ##################################################################### #
# This is the tessh version of John Conway's "Game of Life".            #
# On a rectangular grid, let each "cell" be either "living" or "dead."  #
# Designate a living cell with a dot, and a dead one with a blank space.#
#   1) Each cell has 8 neighbors, the adjoining cells                   #
#+     left, right, top, bottom, and the 4 diagonals.                   #
#                       123                                             #
#                       4*5     The * is the cell under consideration.  #
#                       678                                             #
# 2) A living cell with either 2 or 3 living neighbors remains alive.   #
SURVIVE = 2                                                             #
# 3) A dead cell with 3 living neighbors comes alive, a "birth."        #
BIRTH = 3                                                               #
# 4) All other cases result in a dead cell for the next generation.     #
# ##################################################################### #

# Read the starting generation from the file "gen0", or the one given.
startfile = Path (if (len $argv) > 1 and $argv[1] then $argv[1] else gen0)

E_NOSTARTFILE = 86
if not ($startfile.exists)
  print "Startfile \"$startfile\" missing!"
  exit $E_NOSTARTFILE

ALIVE1 = "."                 # Represent living cells in the start-up file
                             # (and dead ones with "_").
GENERATIONS = 10             # How many generations to cycle through.
NONE_ALIVE = 85              # Exit status on premature bailout,
                             # if no cells left alive.
on_terminal = (sys.stdout.isatty)
DELAY = if $on_terminal then 2 else 0     # Pause between generations.

# Load the grid from the start-up file, without the lines with a '#':
# a list of rows, each a list of True (alive) and False (dead).
grid = [[($cell == $ALIVE1) for cell in $line] for line in (cat $startfile).lines if "#" not in $line]
ROWS = len $grid             # The grid's size is the file's.
COLS = len $grid[0]

def is_valid row col         # Test if cell coordinate valid.
  0 <= $row < $ROWS and 0 <= $col < $COLS

def get_count grid row col   # Count live cells in the cell's neighborhood
  sum (1 for r in [($row - 1) $row ($row + 1)]       # (not the cell itself).
         for c in [($col - 1) $col ($col + 1)]
         if [$r $c] != [$row $col] and (is_valid $r $c) and $grid[$r][$c])

def is_alive grid row col    # Is the cell alive in the next generation?
  nhbd = get_count $grid $row $col
  $nhbd == $BIRTH or ($grid[$row][$col] and $nhbd == $SURVIVE)

def next_gen grid            # The next generation.
  [[(is_alive $grid $r $c) for c in (range $COLS)] for r in (range $ROWS)]

def display grid generation
  print
  for row in $grid
    line = ''.join [(if $alive then $ALIVE1 else " ") for alive in $row]
    print ("      $line".rstrip)        # Indent.
  alive = sum [(sum $row) for row in $grid]
  print
  print "Generation $generation  -  $alive alive"
  $alive

def reverse_video state      # (Only on a terminal.)
  if $on_terminal
    setterm -reverse $state

# =========================================================

if $on_terminal
  clear                      # Clear screen.

print                        # Title
reverse_video on
print "======================="
reverse_video off
print "    $GENERATIONS generations"
print "           of"
print "\"Life in the Slow Lane\""
reverse_video on
print "======================="
reverse_video off

time.sleep $DELAY            # Display "splash screen" for 2 seconds.

generation = 0
display $grid $generation    # Display first generation.
time.sleep $DELAY
print

for generation in (range 1 ($GENERATIONS + 1))
  grid = next_gen $grid      # Update & display.
  if (display $grid $generation) == 0
    print
    print "Premature exit: no more cells alive!"
    exit $NONE_ALIVE         #  No point in continuing
  time.sleep $DELAY          #+ if no live cells.

print
exit 0

# The grid in this script has a "boundary problem."
# The top, bottom, and sides border on a void of dead cells.
# Exercise: Change the script to have the grid wrap around.
```

```output

=======================
    10 generations
           of
"Life in the Slow Lane"
=======================

        .  ..
        .. .
          .   ..
       .       .
          .
      ..  ...
          .
         ...
        .. ..
       ..   ..

Generation 0  -  29 alive


        .....
        .. ..
        ...   ..
              ..
      ..  .
         ..
            .
        .   .
       .     .
       ... ...

Generation 1  -  32 alive

        .   .
       .    ..
        . .. ...
       .. .   ..
         ..
         ...
         . .
            ..
       . . . .
       ..   ..

Generation 2  -  33 alive

            ..
       ...
        . ..   .
       ..    . .

        .  .
         . .
        .  . .
       .   .  .
       ..   ..

Generation 3  -  27 alive

        .
       ......
          .   .
       ...    .
       ..
          .
        .. .
        .  .
       .   .  .
       ..   ..

Generation 4  -  28 alive

       .. ..
       .. ..
             .
       . .
       .
       .  .
        .. .
       ... ..
       .   . .
       ..   ..

Generation 5  -  29 alive

       .. ..
       .. ...
       . ..
        .
      ..
       . ..
           ..
       . . .
      .  ... .
       ..   ..

Generation 6  -  32 alive

       .. . .
      .     .
       .  .
      . ..
      .. .
      ... ..
         . ..
        ..
      .  . . .
       .......

Generation 7  -  35 alive

       .   .
      . ..
      ....
      .  ..

      .    ..
           ..
        .. .
           . .
       ... . .

Generation 8  -  27 alive

       ..
      .  ..
      .
      .  ..
          ..
           ..

           .
       .   .
        . .

Generation 9  -  18 alive

       ...
      . ..
      ..
         ...
         .  .
          ...
           ..

          ..


Generation 10  -  20 alive

```

### Example A-11. Data file for Game of Life (`gen0`)

The tessh version reads the same file, made at the top of its example.

```bash no-run
# gen0
#
# This is an example "generation 0" start-up file for "life.sh".
# --------------------------------------------------------------
#  The "gen0" file is a 10 x 10 grid using a period (.) for live cells,
#+ and an underscore (_) for dead ones. We cannot simply use spaces
#+ for dead cells in this file because of a peculiarity in Bash arrays.
#  [Exercise for the reader: explain this.]
#
# Lines beginning with a '#' are comments, and the script ignores them.
__.__..___
__.._.____
____.___..
_._______.
____._____
..__...___
____._____
___...____
__.._..___
_..___..__
```
