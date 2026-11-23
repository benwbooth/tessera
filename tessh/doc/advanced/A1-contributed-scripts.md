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
