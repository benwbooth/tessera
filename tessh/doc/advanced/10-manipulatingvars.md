# 10. Manipulating Variables
<!-- help: abs-manipulatingvars abs-10 strings string-manipulation parameter-substitution -->

Bash's string operations are a mix of parameter substitution
(`${var#pat}`, `${var/a/b}`, `${var:-default}`…) and the `expr` program,
with a syntax for each. In tessh a string is a Python `str`: its length is
`len`, a piece of it is a slice, and its methods (`replace`, `find`,
`startswith`, `removeprefix`, `split`, `partition`…) and regexes (`re'…'`)
do the rest. A file name is better as a Path, whose `.name`, `.stem`,
`.suffix` and `.parent` are bash's `basename` and pattern tricks done
right. `expr` and `awk` are programs, so they work as in bash.

## 10.1 Manipulating Strings
<!-- help: abs-string-manipulation string-length substring slice expr-index expr-match replace -->

### String length

`${#string}` is `len $string`.

```bash
stringZ=abcABC123ABCabc

echo ${#stringZ}                 # 15
echo `expr length $stringZ`      # 15
echo `expr "$stringZ" : '.*'`    # 15
```

```tessh
stringZ = abcABC123ABCabc

print (len $stringZ)             # 15
print (expr length $stringZ)     # 15
print (expr $stringZ : '.*')     # 15
```

```output
15
15
15
```

### Example 10-1. Inserting a blank line between paragraphs in a text file (`paragraph-space.sh`)

```bash no-run
#!/bin/bash
# paragraph-space.sh
# Ver. 2.1, Reldate 29Jul12 [fixup]

# Inserts a blank line between paragraphs of a single-spaced text file.
# Usage: $0 <FILENAME

MINLEN=60        # Change this value? It's a judgment call.
#  Assume lines shorter than $MINLEN characters ending in a period
#+ terminate a paragraph. See exercises below.

while read line  # For as many lines as the input file has ...
do
  echo "$line"   # Output the line itself.

  len=${#line}
  if [[ "$len" -lt "$MINLEN" && "$line" =~ [*{\.}]$ ]]
# if [[ "$len" -lt "$MINLEN" && "$line" =~ \[*\.\] ]]
# An update to Bash broke the previous version of this script. Ouch!
# Thank you, Halim Srama, for pointing this out and suggesting a fix.
    then echo    #  Add a blank line immediately
  fi             #+ after a short line terminated by a period.
done

exit

# Exercises:
# ---------
#  1) The script usually inserts a blank line at the end
#+    of the target file. Fix this.
#  2) Line 17 only considers periods as sentence terminators.
#     Modify this to include other common end-of-sentence characters,
#+    such as ?, !, and ".
```

A `for` over the input gives its lines with their newlines, which
`rstrip` takes off. The regex says "a period at the end" plainly;
exercise 2 is `re'[.?!"]$'`.

```tessh
import sys
# (A made-up single-spaced text, standing in for the standard input.)
printf "%s\n" "It was a dark and stormy night; the rain fell in torrents," "except at occasional intervals." "Then the wind swept up the streets." "The end." > story.txt
$sys.stdin = open story.txt

MINLEN = 60      # Change this value? It's a judgment call.
#  Assume lines shorter than $MINLEN characters ending in a period
#+ terminate a paragraph.

for line in $sys.stdin    # For as many lines as the input has ...
  line = $line.rstrip "\n"
  print $line             # Output the line itself.
  if (len $line) < $MINLEN and $line =~ re'\.$'
    print                 #  Add a blank line immediately
                          #+ after a short line terminated by a period.
```

```output
It was a dark and stormy night; the rain fell in torrents,
except at occasional intervals.

Then the wind swept up the streets.

The end.

```

### Length of matching substring at beginning of string

`expr match "$string" 'regex'` (or `expr "$string" : 'regex'`) is how
many characters at the start match: the `end` of a regex's `match`.

```bash
stringZ=abcABC123ABCabc
#       |------|
#       12345678

echo `expr match "$stringZ" 'abc[A-Z]*.2'`   # 8
echo `expr "$stringZ" : 'abc[A-Z]*.2'`       # 8
```

```tessh
stringZ = abcABC123ABCabc

print (expr match $stringZ 'abc[A-Z]*.2')       # 8   (expr works as in bash)
print ((re'abc[A-Z]*.2'.match $stringZ).end)   # 8
```

```output
8
8
```

### Index

`expr index $string $chars` is the position (from 1) of the first
character of `$string` that is one of `$chars`. A regex character class
finds it; Python counts from 0. (`$string.find text` is where some
text starts, or -1.)

```bash
stringZ=abcABC123ABCabc
#       123456 ...
echo `expr index "$stringZ" C12`             # 6
                                             # C position.

echo `expr index "$stringZ" 1c`              # 3
# 'c' (in #3 position) matches before '1'.
```

```tessh
stringZ = abcABC123ABCabc

m = re'[C12]'.search $stringZ
print (($m.start) + 1)                          # 6
m = re'[1c]'.search $stringZ
print (($m.start) + 1)                          # 3
```

```output
6
3
```

### Substring extraction

`${string:position}` and `${string:position:length}` are slices,
`$string[position:]` and `$string[position:position + length]`, counting
from 0. A negative position counts from the end, without bash's trap
(`${stringZ:-4}` is a default value, not the last four characters).

```bash
stringZ=abcABC123ABCabc
#       0123456789.....
#       0-based indexing.

echo ${stringZ:0}                            # abcABC123ABCabc
echo ${stringZ:1}                            # bcABC123ABCabc
echo ${stringZ:7}                            # 23ABCabc

echo ${stringZ:7:3}                          # 23A
                                             # Three characters of substring.

# Is it possible to index from the right end of the string?

echo ${stringZ:-4}                           # abcABC123ABCabc
# Defaults to full string, as in ${parameter:-default}.
# However . . .

echo ${stringZ:(-4)}                         # Cabc
echo ${stringZ: -4}                          # Cabc
# Now, it works.
# Parentheses or added space "escape" the position parameter.

# Thank you, Dan Jacobson, for pointing this out.
```

```output bash
abcABC123ABCabc
bcABC123ABCabc
23ABCabc
23A
abcABC123ABCabc
Cabc
Cabc
```

```tessh
stringZ = abcABC123ABCabc
#         0123456789.....

print $stringZ[0:]           # abcABC123ABCabc
print $stringZ[1:]           # bcABC123ABCabc
print $stringZ[7:]           # 23ABCabc

print $stringZ[7:10]         # 23A
                             # Three characters of substring.

# Index from the right end of the string:
print $stringZ[-4:]          # Cabc
```

```output
abcABC123ABCabc
bcABC123ABCabc
23ABCabc
23A
Cabc
```

The position and length can be variables, as in the next example.

### Example 10-2. Generating an 8-character "random" string (`rand-string.sh`)

```bash no-run
#!/bin/bash
# rand-string.sh
# Generating an 8-character "random" string.

if [ -n "$1" ]  #  If command-line argument present,
then            #+ then set start-string to it.
  str0="$1"
else            #  Else use PID of script as start-string.
  str0="$$"
fi

POS=2  # Starting from position 2 in the string.
LEN=8  # Extract eight characters.

str1=$( echo "$str0" | md5sum | md5sum )
#  Doubly scramble     ^^^^^^   ^^^^^^
#+ by piping and repiping to md5sum.

randstring="${str1:$POS:$LEN}"
# Can parameterize ^^^^ ^^^^

echo "$randstring"

exit $?

# bozo$ ./rand-string.sh my-password
# 1bdd88c4

#  No, this is is not recommended
#+ as a method of generating hack-proof passwords.
```

The same pipeline, then a slice. Given `my-password` it prints what the
ABS Guide says. (For a string that's really random, `secrets.token_hex 4`.)

```tessh
import os

args = $argv[1:] or [my-password]    # (a sample argument, when none is given)

if $args          #  If command-line argument present,
  str0 = $args[0] #+ then set start-string to it.
else              #  Else use PID of script as start-string.
  str0 = str (os.getpid)

POS = 2  # Starting from position 2 in the string.
LEN = 8  # Extract eight characters.

str1 = echo $str0 | md5sum | md5sum
#  Doubly scramble   ^^^^^^   ^^^^^^

randstring = $str1[$POS:$POS + $LEN]

print $randstring
```

```output
1bdd88c4
```

`${*:2}` and `${@:2}` are the positional parameters from the second on, and
`${*:2:3}` three of them: slices of `$argv`.

```bash no-run
echo ${*:2}          # Echoes second and following positional parameters.
echo ${@:2}          # Same as above.

echo ${*:2:3}        # Echoes three positional parameters, starting at second.
```

```tessh
argv = $argv[:1] + [one two three four five]   # (sample arguments)

echo $argv[2:]       # Echoes second and following arguments.
echo $argv[2:5]      # Echoes three arguments, starting at second.
```

```output
two three four five
two three four
```

`expr substr $string $position $length` counts from 1:

```bash
stringZ=abcABC123ABCabc
#       123456789......
#       1-based indexing.

echo `expr substr $stringZ 1 2`              # ab
echo `expr substr $stringZ 4 3`              # ABC
```

```tessh
stringZ = abcABC123ABCabc
#         012345678......  (from 0)

print $stringZ[0:2]          # ab
print $stringZ[3:6]          # ABC
```

```output
ab
ABC
```

`expr match "$string" '\(regex\)'` gives the part at the start that a
regex matches; with `.*` in front, the part at the end. In Python that's a
group of a `match`.

```bash
stringZ=abcABC123ABCabc
#       =======

echo `expr match "$stringZ" '\(.[b-c]*[A-Z]..[0-9]\)'`   # abcABC1
echo `expr "$stringZ" : '\(.[b-c]*[A-Z]..[0-9]\)'`       # abcABC1
echo `expr "$stringZ" : '\(.......\)'`                   # abcABC1
# All of the above forms give an identical result.

stringZ=abcABC123ABCabc
#                ======

echo `expr match "$stringZ" '.*\([A-C][A-C][A-C][a-c]*\)'`    # ABCabc
echo `expr "$stringZ" : '.*\(......\)'`                       # ABCabc
```

```tessh
stringZ = abcABC123ABCabc

print (expr match $stringZ '\(.[b-c]*[A-Z]..[0-9]\)')     # abcABC1
print ((re'.[b-c]*[A-Z]..[0-9]'.match $stringZ).group)   # abcABC1
print ((re'.......'.match $stringZ).group)               # abcABC1

print ((re'.*([A-C][A-C][A-C][a-c]*)'.match $stringZ).group 1)   # ABCabc
print $stringZ[-6:]                                          # ABCabc
```

```output
abcABC1
abcABC1
abcABC1
ABCabc
ABCabc
```

### Substring removal

`${string#pattern}` takes the shortest match of a glob pattern off the
front, `##` the longest; `%` and `%%` do the same at the back. Python's
`removeprefix` and `removesuffix` take off fixed text, and `partition`
and `rpartition` cut at the first or last of something, which covers most
uses. For a real pattern, `fnmatch` matches globs, and the two functions
below are bash's operators. (Quote a pattern given to a function, so it's
the text, not a Glob of file names.)

**Gap:** no built-in for bash's `#`, `##`, `%` and `%%` with a pattern;
`strip_front` and `strip_back` below are a few lines each.

```bash
stringZ=abcABC123ABCabc
#       |----|          shortest
#       |----------|    longest

echo ${stringZ#a*C}      # 123ABCabc
# Strip out shortest match between 'a' and 'C'.

echo ${stringZ##a*C}     # abc
# Strip out longest match between 'a' and 'C'.

# You can parameterize the substrings.

X='a*C'

echo ${stringZ#$X}      # 123ABCabc
echo ${stringZ##$X}     # abc
                        # As above.

stringZ=abcABC123ABCabc
#                    ||     shortest
#        |------------|     longest

echo ${stringZ%b*c}      # abcABC123ABCa
# Strip out shortest match between 'b' and 'c', from back of $stringZ.

echo ${stringZ%%b*c}     # a
# Strip out longest match between 'b' and 'c', from back of $stringZ.
```

```tessh
from fnmatch import fnmatchcase

def strip_front s pat longest=False    # ${s#pat}, or ${s##pat} if longest
  ends = if $longest then (range (len $s) -1 -1) else (range ((len $s) + 1))
  for i in $ends
    if (fnmatchcase $s[:$i] $pat)
      return $s[$i:]
  $s

def strip_back s pat longest=False     # ${s%pat}, or ${s%%pat} if longest
  starts = if $longest then (range ((len $s) + 1)) else (range (len $s) -1 -1)
  for i in $starts
    if (fnmatchcase $s[$i:] $pat)
      return $s[:$i]
  $s

stringZ = abcABC123ABCabc

print (strip_front $stringZ 'a*C')               # 123ABCabc
print (strip_front $stringZ 'a*C' longest=True)  # abc

X = 'a*C'
print (strip_front $stringZ $X)                  # 123ABCabc
print (strip_front $stringZ $X longest=True)     # abc

print (strip_back $stringZ 'b*c')                # abcABC123ABCa
print (strip_back $stringZ 'b*c' longest=True)   # a
```

```output
123ABCabc
abc
123ABCabc
abc
abcABC123ABCa
a
```

Fixed text, and cutting at the first or last of something, need no
pattern:

```tessh
stringZ = abcABC123ABCabc

print ($stringZ.removeprefix abc) ($stringZ.removesuffix abc)  # ${stringZ#abc} ${stringZ%abc}
print ($stringZ.partition C)[2]                  # 123ABCabc   (${stringZ#*C})
print ($stringZ.rpartition C)[2]                 # abc         (${stringZ##*C})
```

```output
ABC123ABCabc abcABC123ABC
123ABCabc
abc
```

Renaming every `.TXT` file to `.txt` is a suffix change, which a Path
does itself:

```bash no-run
# Rename all filenames in $PWD with "TXT" suffix to a "txt" suffix.
# For example, "file1.TXT" becomes "file1.txt" . . .

SUFF=TXT
suff=txt

for i in $(ls *.$SUFF)
do
  mv -f $i ${i%.$SUFF}.$suff
  #  Leave unchanged everything *except* the shortest pattern match
  #+ starting from the right-hand-side of the variable $i . . .
done ### This could be condensed into a "one-liner" if desired.

# Thank you, Rory Winston.
```

```tessh
# (Some made-up files to try it on.)
touch file1.TXT file2.TXT notes.md

SUFF = TXT
suff = txt

for i in *.$SUFF                       # A glob's names are Paths.
  mv -f $i ($i.with_suffix ".$suff")   # file1.TXT -> file1.txt

ls
```

```output
file1.txt
file2.txt
notes.md
```

### Example 10-3. Converting graphic file formats, with filename change (`cvt.sh`)

```bash no-run
#!/bin/bash
#  cvt.sh:
#  Converts all the MacPaint image files in a directory to "pbm" format.

#  Uses the "macptopbm" binary from the "netpbm" package,
#+ which is maintained by Brian Henderson (bryanh@giraffe-data.com).
#  Netpbm is a standard part of most Linux distros.

OPERATION=macptopbm
SUFFIX=pbm          # New filename suffix. 

if [ -n "$1" ]
then
  directory=$1      # If directory name given as a script argument...
else
  directory=$PWD    # Otherwise use current working directory.
fi  
  
#  Assumes all files in the target directory are MacPaint image files,
#+ with a ".mac" filename suffix.

for file in $directory/*    # Filename globbing.
do
  filename=${file%.*c}      #  Strip ".mac" suffix off filename
                            #+ ('.*c' matches everything
			    #+ between '.' and 'c', inclusive).
  $OPERATION $file > "$filename.$SUFFIX"
                            # Redirect conversion to new filename.
  rm -f $file               # Delete original files after converting.   
  echo "$filename.$SUFFIX"  # Log what is happening to stdout.
done

exit 0

# Exercise:
# --------
#  As it stands, this script converts *all* the files in the current
#+ working directory.
#  Modify it to work *only* on files with a ".mac" suffix.



# *** And here's another way to do it. *** #

#!/bin/bash
# Batch convert into different graphic formats.
# Assumes imagemagick installed (standard in most Linux distros).

INFMT=png   # Can be tif, jpg, gif, etc.
OUTFMT=pdf  # Can be tif, jpg, gif, pdf, etc.

for pic in *"$INFMT"
do
  p2=$(ls "$pic" | sed -e s/\.$INFMT//)
  # echo $p2
    convert "$pic" $p2.$OUTFMT
    done

exit 0
```

The directory is a Path, its files `$directory.glob "*.mac"` (which also
does the exercise), and `with_suffix` makes the new name. The program to
run is a value, as in bash; `env` runs it (see the gap below). `macptopbm` isn't here, so this runs `cat`
(a copy) in its place on made-up files.

```tessh
# (Made-up "MacPaint" files to try it on.)
mkdir -p pics ; echo one >pics/a.mac ; echo two >pics/b.mac

OPERATION = cat     # (macptopbm in the original, which isn't here.)
SUFFIX = pbm        # New filename suffix.

directory = Path (if (len $argv) > 1 then $argv[1] else pics)   # (else $PWD in the original)

for file in (sorted ($directory.glob "*.mac"))     # Only the .mac files.
  newname = $file.with_suffix ".$SUFFIX"           # Strip ".mac", add ".pbm".
  env $OPERATION $file > $newname                  # Redirect conversion to new filename.
  rm -f $file                                      # Delete original files after converting.
  print $newname                                   # Log what is happening to stdout.
```

```output
pics/a.pbm
pics/b.pbm
```

`$OPERATION $file` works too, as in bash: text at the head of a command
runs the program it names. `env $OPERATION $file` says so outright.

The second way, with ImageMagick's `convert`, for `.png` files:

```tessh no-run
INFMT = png    # Can be tif, jpg, gif, etc.
OUTFMT = pdf   # Can be tif, jpg, gif, pdf, etc.

for pic in *.$INFMT
  convert $pic ($pic.with_suffix ".$OUTFMT")
```

### Example 10-4. Converting streaming audio files to ogg (`ra2ogg.sh`)

```bash no-run
#!/bin/bash
# ra2ogg.sh: Convert streaming audio files (*.ra) to ogg.

# Uses the "mplayer" media player program:
#      http://www.mplayerhq.hu/homepage
# Uses the "ogg" library and "oggenc":
#      http://www.xiph.org/
#
# This script may need appropriate codecs installed, such as sipr.so ...
# Possibly also the compat-libstdc++ package.


OFILEPREF=${1%%ra}      # Strip off the "ra" suffix.
OFILESUFF=wav           # Suffix for wav file.
OUTFILE="$OFILEPREF""$OFILESUFF"
E_NOARGS=85

if [ -z "$1" ]          # Must specify a filename to convert.
then
  echo "Usage: `basename $0` [filename]"
  exit $E_NOARGS
fi


##########################################################################
mplayer "$1" -ao pcm:file=$OUTFILE
oggenc "$OUTFILE"  # Correct file extension automatically added by oggenc.
##########################################################################

rm "$OUTFILE"      # Delete intermediate *.wav file.
                   # If you want to keep it, comment out above line.

exit $?

#  Note:
#  ----
#  On a Website, simply clicking on a *.ram streaming audio file
#+ usually only downloads the URL of the actual *.ra audio file.
#  You can then use "wget" or something similar
#+ to download the *.ra file itself.


#  Exercises:
#  ---------
#  As is, this script converts only *.ra filenames.
#  Add flexibility by permitting use of *.ram and other filenames.
#
#  If you're really ambitious, expand the script
#+ to do automatic downloads and conversions of streaming audio files.
#  Given a URL, batch download streaming audio files (using "wget")
#+ and convert them on the fly.
```

`with_suffix` does the exercise too: any suffix, `.ra` or `.ram`, is
replaced. It needs `mplayer`, `oggenc` and a file, so it doesn't run here.

```tessh no-run
E_NOARGS = 85

if (len $argv) < 2 or not $argv[1]    # Must specify a filename to convert.
  print "Usage: $(Path $argv[0]).name [filename]"
  exit $E_NOARGS

infile = Path $argv[1]
OUTFILE = $infile.with_suffix .wav    # Whatever the suffix was.

##########################################################################
mplayer $infile -ao "pcm:file=$OUTFILE"
oggenc $OUTFILE    # Correct file extension automatically added by oggenc.
##########################################################################

rm $OUTFILE        # Delete intermediate *.wav file.
                   # If you want to keep it, comment out above line.
```

### Example 10-5. Emulating getopt (`getopt-simple.sh`)

```bash no-run
#!/bin/bash
# getopt-simple.sh
# Author: Chris Morgan
# Used in the ABS Guide with permission.


getopt_simple()
{
    echo "getopt_simple()"
    echo "Parameters are '$*'"
    until [ -z "$1" ]
    do
      echo "Processing parameter of: '$1'"
      if [ ${1:0:1} = '/' ]
      then
          tmp=${1:1}               # Strip off leading '/' . . .
          parameter=${tmp%%=*}     # Extract name.
          value=${tmp##*=}         # Extract value.
          echo "Parameter: '$parameter', value: '$value'"
          eval $parameter=$value
      fi
      shift
    done
}

# Pass all options to getopt_simple().
getopt_simple $*

echo "test is '$test'"
echo "test2 is '$test2'"

exit 0  # See also, UseGetOpt.sh, a modified version of this script.

---

sh getopt_example.sh /test=value1 /test2=value2

Parameters are '/test=value1 /test2=value2'
Processing parameter of: '/test=value1'
Parameter: 'test', value: 'value1'
Processing parameter of: '/test2=value2'
Parameter: 'test2', value: 'value2'
test is 'value1'
test2 is 'value2'
```

Instead of `eval` making variables named by the input, the options go in
a dict, which can't clobber the script's own names. `partition` cuts at
the first `=` (bash's `%%=*` and `##*=` cut at the first and the last,
which differ when the value has an `=` in it).

```tessh
def getopt_simple *params
  print "getopt_simple()"
  print "Parameters are '$(" ".join $params)'"
  opts = {}
  for param in $params
    print "Processing parameter of: '$param'"
    if ($param.startswith "/")
      [parameter _ value] = $param[1:].partition "="   # Strip off leading '/',
      print "Parameter: '$parameter', value: '$value'" # split at the "=".
      $opts[$parameter] = $value
  $opts

args = $argv[1:] or ["/test=value1" "/test2=value2"]   # (sample arguments, when none are given)

# Pass all options to getopt_simple.
opts = getopt_simple *$args

print "test is '$($opts.get test "")'"
print "test2 is '$($opts.get test2 "")'"
```

```output
getopt_simple()
Parameters are '/test=value1 /test2=value2'
Processing parameter of: '/test=value1'
Parameter: 'test', value: 'value1'
Processing parameter of: '/test2=value2'
Parameter: 'test2', value: 'value2'
test is 'value1'
test2 is 'value2'
```

### Substring replacement

`${string/substring/replacement}` replaces the first match, `//` all of
them: `replace` with a count of 1, and without. Bash's substring there is
a pattern; `str.replace` takes text, and a regex's `sub` takes a pattern.
`/#` and `/%` replace only at the front or the back: a regex with `^` or
`$`.

```bash
stringZ=abcABC123ABCabc

echo ${stringZ/abc/xyz}       # xyzABC123ABCabc
                              # Replaces first match of 'abc' with 'xyz'.

echo ${stringZ//abc/xyz}      # xyzABC123ABCxyz
                              # Replaces all matches of 'abc' with # 'xyz'.

echo  ---------------
echo "$stringZ"               # abcABC123ABCabc
echo  ---------------
                              # The string itself is not altered!

# Can the match and replacement strings be parameterized?
match=abc
repl=000
echo ${stringZ/$match/$repl}  # 000ABC123ABCabc
#              ^      ^         ^^^
echo ${stringZ//$match/$repl} # 000ABC123ABC000
# Yes!          ^      ^        ^^^         ^^^

echo

# What happens if no $replacement string is supplied?
echo ${stringZ/abc}           # ABC123ABCabc
echo ${stringZ//abc}          # ABC123ABC
# A simple deletion takes place.

stringZ=abcABC123ABCabc

echo ${stringZ/#abc/XYZ}          # XYZABC123ABCabc
                                  # Replaces front-end match of 'abc' with 'XYZ'.

echo ${stringZ/%abc/XYZ}          # abcABC123ABCXYZ
                                  # Replaces back-end match of 'abc' with 'XYZ'.
```

```tessh
stringZ = abcABC123ABCabc

print ($stringZ.replace abc xyz 1)    # xyzABC123ABCabc
                                      # Replaces first match of 'abc' with 'xyz'.
print ($stringZ.replace abc xyz)      # xyzABC123ABCxyz
                                      # Replaces all matches of 'abc' with 'xyz'.

print "---------------"
print $stringZ                        # abcABC123ABCabc
print "---------------"
                                      # The string itself is not altered!

# The match and replacement strings can be variables.
match = abc
repl = "000"        # (Quoted: a bare 000 is the number 0.)
print ($stringZ.replace $match $repl 1)   # 000ABC123ABCabc
print ($stringZ.replace $match $repl)     # 000ABC123ABC000

print

# Replacing with nothing deletes.
print ($stringZ.replace abc "" 1)     # ABC123ABCabc
print ($stringZ.replace abc "")       # ABC123ABC

print (re'^abc'.sub XYZ $stringZ)     # XYZABC123ABCabc
                                      # Replaces front-end match of 'abc' with 'XYZ'.
print (re'abc$'.sub XYZ $stringZ)     # abcABC123ABCXYZ
                                      # Replaces back-end match of 'abc' with 'XYZ'.
```

```output
xyzABC123ABCabc
xyzABC123ABCxyz
---------------
abcABC123ABCabc
---------------
000ABC123ABCabc
000ABC123ABC000

ABC123ABCabc
ABC123ABC
XYZABC123ABCabc
abcABC123ABCXYZ
```

### Manipulating strings using awk

A script can hand string work to `awk` instead, which counts from 1.

### Example 10-6. Alternate ways of extracting and locating substrings (`substring-extraction.sh`)

```bash
#!/bin/bash
# substring-extraction.sh

String=23skidoo1
#      012345678    Bash
#      123456789    awk
# Note different string indexing system:
# Bash numbers first character of string as 0.
# Awk  numbers first character of string as 1.

echo ${String:2:4} # position 3 (0-1-2), 4 characters long
                                         # skid

# The awk equivalent of ${string:pos:length} is substr(string,pos,length).
echo | awk '
{ print substr("'"${String}"'",3,4)      # skid
}
'
#  Piping an empty "echo" to awk gives it dummy input,
#+ and thus makes it unnecessary to supply a filename.

echo "----"

# And likewise:

echo | awk '
{ print index("'"${String}"'", "skid")      # 3
}                                           # (skid starts at position 3)
'   # The awk equivalent of "expr index" ...

exit 0
```

`awk -v` passes the string in as an awk variable, which is cleaner than
splicing it into the program's quotes. `str.find` is the Python way,
counting from 0.

```tessh
String = 23skidoo1
#        012345678    tessh (Python)
#        123456789    awk

print $String[2:6]     # position 3 (0-1-2), 4 characters long
                       # skid

# The awk equivalent is substr(string,pos,length).
echo | awk -v "s=$String" '{ print substr(s, 3, 4) }'      # skid

print "----"

echo | awk -v "s=$String" '{ print index(s, "skid") }'     # 3
                                       # ($String.find skid is 2, from 0.)
```

```output
skid
skid
----
3
```

## 10.2 Parameter Substitution
<!-- help: abs-parameter-substitution ${var:-default} default-value ${var#pattern} ${var/pat/rep} -->

### ${parameter}

`${parameter}` is `$parameter`, with braces to mark where the name ends.
tessh's `${…}` in a string does that too (and takes any expression).

```bash no-run
your_id=${USER}-on-${HOSTNAME}
echo "$your_id"
#
echo "Old \$PATH = $PATH"
PATH=${PATH}:/opt/bin  # Add /opt/bin to $PATH for duration of script.
echo "New \$PATH = $PATH"
```

```tessh
USER = bozo ; HOSTNAME = darkstar      # (made-up values)
your_id = "${USER}-on-${HOSTNAME}"
print $your_id

$env.PATH = "$env.PATH:/opt/bin"       # Add /opt/bin to PATH for the script.
print ($env.PATH.endswith ":/opt/bin")
```

```output
bozo-on-darkstar
True
```

### Default values

`${parameter-default}` is the default when the parameter isn't set, and
`${parameter:-default}` also when it's empty. A tessh name is either
defined or an error, so "not set" is usually `None`, a missing key or a
missing argument:

- `$x or default` is `${x:-default}` (empty, `None`, 0 all take the default).
- `$x if $x is not None else default` is `${x-default}`.
- `$env.get NAME default` for an environment variable, `$d.get key default`
  for a dict.
- A function parameter's default (`def f name=default`) for an argument.

```bash
var1=1
var2=2
# var3 is unset.

echo ${var1-$var2}   # 1
echo ${var3-$var2}   # 2
#           ^          Note the $ prefix.
```

```tessh
var1 = 1
var2 = 2
var3 = None          # "unset"

print (if $var1 is not None then $var1 else $var2)   # 1
print (if $var3 is not None then $var3 else $var2)   # 2
```

```output
1
2
```

The difference between `-` and `:-` is a declared but empty variable:

```bash no-run
#!/bin/bash
# param-sub.sh

#  Whether a variable has been declared
#+ affects triggering of the default option
#+ even if the variable is null.

username0=
echo "username0 has been declared, but is set to null."
echo "username0 = ${username0-`whoami`}"
# Will not echo.

echo

echo username1 has not been declared.
echo "username1 = ${username1-`whoami`}"
# Will echo.

username2=
echo "username2 has been declared, but is set to null."
echo "username2 = ${username2:-`whoami`}"
#                            ^
# Will echo because of :- rather than just - in condition test.
# Compare to first instance, above.


#

# Once again:

variable=
# variable has been declared, but is set to null.

echo "${variable-0}"    # (no output)
echo "${variable:-1}"   # 1
#               ^

unset variable

echo "${variable-2}"    # 2
echo "${variable:-3}"   # 3

exit 0
```

`default` here is `${…-…}`; `or` is `${…:-…}`. (`whoami` is a stand-in
function, so the output doesn't depend on who runs it.)

```tessh
def whoami = "bozo"    # (In place of the whoami program.)

def default value fallback
  # ${value-fallback}: the fallback only if value is unset (None).
  if $value is None then $fallback else $value

username0 = ""
print "username0 has been declared, but is set to null."
print "username0 = $(default $username0 (whoami))"
# Will not echo.

print

print "username1 has not been declared."
username1 = None
print "username1 = $(default $username1 (whoami))"
# Will echo.

username2 = ""
print "username2 has been declared, but is set to null."
print "username2 = $($username2 or (whoami))"
# Will echo because of "or", which takes the empty string as false.

# Once again:

variable = ""
print (default $variable 0)    # (no output)
print ($variable or 1)         # 1

variable = None               # (unset variable)

print (default $variable 2)    # 2
print ($variable or 3)         # 3
```

```output
username0 has been declared, but is set to null.
username0 = 

username1 has not been declared.
username1 = bozo
username2 has been declared, but is set to null.
username2 = bozo

1
2
3
```

A default for a missing command-line argument:

```bash no-run
DEFAULT_FILENAME=generic.data
filename=${1:-$DEFAULT_FILENAME}
#  If not otherwise specified, the following command block operates
#+ on the file "generic.data".
```

```tessh
DEFAULT_FILENAME = generic.data
filename = if (len $argv) > 1 and $argv[1] then $argv[1] else $DEFAULT_FILENAME
print $filename
```

```output
generic.data
```

### Setting a default: ${parameter=default}

`${parameter=default}` sets the parameter to the default when it isn't
set. For a dict (or `$env`, or the script's `globals`) that's
`setdefault`.

```bash
echo ${var=abc}   # abc
echo ${var=xyz}   # abc
# $var had already been set to abc, so it did not change.
```

```tessh
settings = {}
print ($settings.setdefault var abc)   # abc
print ($settings.setdefault var xyz)   # abc
# var had already been set to abc, so it did not change.
```

```output
abc
abc
```

### Alternate values: ${parameter+alt_value}

`${parameter+alt_value}` is the alternate value if the parameter is set
(even to nothing), else nothing; `${parameter:+alt_value}` only if it's
set and not empty. In tessh, an `if … then … else` with `is not None` or
plain truth:

```bash
echo "###### \${parameter+alt_value} ########"
echo

a=${param1+xyz}
echo "a = $a"      # a =

param2=
a=${param2+xyz}
echo "a = $a"      # a = xyz

param3=123
a=${param3+xyz}
echo "a = $a"      # a = xyz

echo
echo "###### \${parameter:+alt_value} ########"
echo

a=${param4:+xyz}
echo "a = $a"      # a =

param5=
a=${param5:+xyz}
echo "a = $a"      # a =
# Different result from   a=${param5+xyz}

param6=123
a=${param6:+xyz}
echo "a = $a"      # a = xyz
```

```tessh
print "###### \${parameter+alt_value} ########"   # (is not None)
print

param1 = None
a = if $param1 is not None then xyz else ""
print "a = $a"      # a =

param2 = ""
a = if $param2 is not None then xyz else ""
print "a = $a"      # a = xyz

param3 = 123
a = if $param3 is not None then xyz else ""
print "a = $a"      # a = xyz

print
print "###### \${parameter:+alt_value} ########"  # (is true)
print

param4 = None
a = if $param4 then xyz else ""
print "a = $a"      # a =

param5 = ""
a = if $param5 then xyz else ""
print "a = $a"      # a =
# Different result from the "is not None" test.

param6 = 123
a = if $param6 then xyz else ""
print "a = $a"      # a = xyz
```

```output
###### ${parameter+alt_value} ########

a = 
a = xyz
a = xyz

###### ${parameter:+alt_value} ########

a = 
a = 
a = xyz
```

### Required values: ${parameter?err_msg}

`${parameter?err_msg}` uses the parameter if it's set, and otherwise
prints the message and ends the script with status 1. tessh does that on
its own for a script's own names: reading an undefined `$name` is an error
that stops the script. For environment variables `$env[NAME]` raises
`KeyError`. To print your own message, check first.

### Example 10-7. Using parameter substitution and error messages (`ex6.sh`)

```bash no-run
#!/bin/bash

#  Check some of the system's environmental variables.
#  This is good preventative maintenance.
#  If, for example, $USER, the name of the person at the console, is not set,
#+ the machine will not recognize you.

: ${HOSTNAME?} ${USER?} ${HOME?} ${MAIL?}
  echo
  echo "Name of the machine is $HOSTNAME."
  echo "You are $USER."
  echo "Your home directory is $HOME."
  echo "Your mail INBOX is located in $MAIL."
  echo
  echo "If you are reading this message,"
  echo "critical environmental variables have been set."
  echo
  echo

# ------------------------------------------------------

#  The ${variablename?} construction can also check
#+ for variables set within the script.

ThisVariable=Value-of-ThisVariable
#  Note, by the way, that string variables may be set
#+ to characters disallowed in their names.
: ${ThisVariable?}
echo "Value of ThisVariable is $ThisVariable".

echo; echo


: ${ZZXy23AB?"ZZXy23AB has not been set."}
#  Since ZZXy23AB has not been set,
#+ then the script terminates with an error message.

# You can specify the error message.
# : ${variablename?"ERROR MESSAGE"}


# Same result with:   dummy_variable=${ZZXy23AB?}
#                     dummy_variable=${ZZXy23AB?"ZXy23AB has not been set."}
#
#                     echo ${ZZXy23AB?} >/dev/null

#  Compare these methods of checking whether a variable has been set
#+ with "set -u" . . .



echo "You will not see this message, because script already terminated."

HERE=0
exit $HERE   # Will NOT exit here.

# In fact, this script will return an exit status (echo $?) of 1.
```

`required` checks names in the script and then in the environment, and
exits with bash's message. `$HOSTNAME` is a bash variable, not an
environment one, so it's `socket.gethostname` in tessh; here the
environment is made up so the output doesn't depend on the machine.

```tessh
import sys

# (A made-up environment, the same everywhere.)
export USER=bozo HOME=/home/bozo MAIL=/var/spool/mail/bozo
HOSTNAME = darkstar     # (socket.gethostname)

def required *names msg=None
  # bash's : ${name?msg}: stop the script if a name isn't set.
  for name in $names
    if $name not in (globals) and $name not in $env
      print "$name: $($msg or 'parameter null or not set')" file=$sys.stderr
      exit 1

required HOSTNAME USER HOME MAIL
print
print "Name of the machine is $HOSTNAME."
print "You are $env.USER."
print "Your home directory is $env.HOME."
print "Your mail INBOX is located in $env.MAIL."
print
print "If you are reading this message,"
print "critical environmental variables have been set."
print
print

# ------------------------------------------------------

#  This can also check for variables set within the script.

ThisVariable = Value-of-ThisVariable
required ThisVariable
print "Value of ThisVariable is $ThisVariable."

print ; print

required ZZXy23AB msg="ZZXy23AB has not been set."
#  Since ZZXy23AB has not been set,
#+ then the script terminates with an error message.

print "You will not see this message, because script already terminated."
```

```output

Name of the machine is darkstar.
You are bozo.
Your home directory is /home/bozo.
Your mail INBOX is located in /var/spool/mail/bozo.

If you are reading this message,
critical environmental variables have been set.


Value of ThisVariable is Value-of-ThisVariable.


```

### Example 10-8. Parameter substitution and usage messages (`usage-message.sh`)

```bash no-run
#!/bin/bash
# usage-message.sh

: ${1?"Usage: $0 ARGUMENT"}
#  Script exits here if command-line parameter absent,
#+ with following error message.
#    usage-message.sh: 1: Usage: usage-message.sh ARGUMENT

echo "These two lines echo only if command-line parameter given."
echo "command-line parameter = \"$1\""

exit 0  # Will exit here only if command-line parameter present.

# Check the exit status, both with and without command-line parameter.
# If command-line parameter present, then "$?" is 0.
# If not, then "$?" is 1.
```

```tessh
import sys

if (len $argv) < 2
  print "Usage: $argv[0] ARGUMENT" file=$sys.stderr
  exit 1
#  Script exits here if command-line parameter absent.

print "These two lines echo only if command-line parameter given."
print "command-line parameter = \"$argv[1]\""

exit 0  # Will exit here only if command-line parameter present.
```

Run without an argument, and with one:

```text
$ tessh usage-message.tsh ; echo "status $?"
Usage: usage-message.tsh ARGUMENT
status 1
$ tessh usage-message.tsh hello
These two lines echo only if command-line parameter given.
command-line parameter = "hello"
```

### Length and pattern removal

`${#var}` is the length; `${#*}` and `${#@}` the number of arguments,
which is `(len $argv) - 1`.

### Example 10-9. Length of a variable (`length.sh`)

```bash no-run
#!/bin/bash
# length.sh

E_NO_ARGS=65

if [ $# -eq 0 ]  # Must have command-line args to demo script.
then
  echo "Please invoke this script with one or more command-line arguments."
  exit $E_NO_ARGS
fi  

var01=abcdEFGH28ij
echo "var01 = ${var01}"
echo "Length of var01 = ${#var01}"
# Now, let's try embedding a space.
var02="abcd EFGH28ij"
echo "var02 = ${var02}"
echo "Length of var02 = ${#var02}"

echo "Number of command-line arguments passed to script = ${#@}"
echo "Number of command-line arguments passed to script = ${#*}"

exit 0
```

```tessh
E_NO_ARGS = 65

args = $argv[1:] or [one two three]   # (sample arguments, when none are given)

if not $args     # Must have command-line args to demo script.
  print "Please invoke this script with one or more command-line arguments."
  exit $E_NO_ARGS

var01 = abcdEFGH28ij
print "var01 = $var01"
print "Length of var01 = $(len $var01)"
# Now, let's try embedding a space.
var02 = "abcd EFGH28ij"
print "var02 = $var02"
print "Length of var02 = $(len $var02)"

print "Number of command-line arguments passed to script = $(len $args)"
```

```output
var01 = abcdEFGH28ij
Length of var01 = 12
var02 = abcd EFGH28ij
Length of var02 = 13
Number of command-line arguments passed to script = 3
```

`${var#Pattern}` and `${var##Pattern}` remove the shortest or longest
match at the front, as in 10.1. Two uses from the ABS Guide: stripping
leading zeros, and the base name of a path.

```bash no-run
strip_leading_zero () #  Strip possible leading zero(s)
{                     #+ from argument passed.
  return=${1#0}       #  The "1" refers to "$1" -- passed arg.
}                     #  The "0" is what to remove from "$1" -- strips zeros.

strip_leading_zero2 () # Strip possible leading zero(s), since otherwise
{                      # Bash will interpret such numbers as octal values.
  shopt -s extglob     # Turn on extended globbing.
  local val=${1##+(0)} # Use local variable, longest matching series of 0's.
  shopt -u extglob     # Turn off extended globbing.
  _strip_leading_zero2=${val:-0}
                       # If input was 0, return 0 instead of "".
}
```

`lstrip "0"` takes off all the zeros, and `or "0"` puts one back if
nothing is left. (Python's `int "08"` is 8: no octal surprise to guard
against.)

```tessh
def strip_leading_zero s = ($s.lstrip "0") or "0"

print (strip_leading_zero "0042") (strip_leading_zero "000") (int "08")
```

```output
42 0 8
```

```bash no-run
echo `basename $PWD`        # Basename of current working directory.
echo "${PWD##*/}"           # Basename of current working directory.
echo
echo `basename $0`          # Name of script.
echo $0                     # Name of script.
echo "${0##*/}"             # Name of script.
echo
filename=test.data
echo "${filename##*.}"      # data
                            # Extension of filename.
```

```tessh
mkdir -p project ; cd project          # (To have a folder with a known name.)

print (Path (pwd)).name                # Basename of current working directory.
print ((pwd).rpartition "/")[2]        # The same, cutting at the last "/".
print
print (Path $argv[0]).name             # Name of script.
print
filename = Path test.data
print $filename.suffix ($filename.suffix.lstrip ".")   # Extension of filename.
```

```output
project
project

ex.tsh

.data data
```

`${var%Pattern}` and `${var%%Pattern}` remove the shortest or longest
match at the back.

### Example 10-10. Pattern matching in parameter substitution (`patt-matching.sh`)

```bash
#!/bin/bash
# patt-matching.sh

# Pattern matching  using the # ## % %% parameter substitution operators.

var1=abcd12345abc6789
pattern1=a*c  # * (wild card) matches everything between a - c.

echo
echo "var1 = $var1"           # abcd12345abc6789
echo "var1 = ${var1}"         # abcd12345abc6789
                              # (alternate form)
echo "Number of characters in ${var1} = ${#var1}"
echo

echo "pattern1 = $pattern1"   # a*c  (everything between 'a' and 'c')
echo "--------------"
echo '${var1#$pattern1}  =' "${var1#$pattern1}"    #         d12345abc6789
# Shortest possible match, strips out first 3 characters  abcd12345abc6789
#                                     ^^^^^               |-|
echo '${var1##$pattern1} =' "${var1##$pattern1}"   #                  6789      
# Longest possible match, strips out first 12 characters  abcd12345abc6789
#                                    ^^^^^                |----------|

echo; echo; echo

pattern2=b*9            # everything between 'b' and '9'
echo "var1 = $var1"     # Still  abcd12345abc6789
echo
echo "pattern2 = $pattern2"
echo "--------------"
echo '${var1%pattern2}  =' "${var1%$pattern2}"     #     abcd12345a
# Shortest possible match, strips out last 6 characters  abcd12345abc6789
#                                     ^^^^                         |----|
echo '${var1%%pattern2} =' "${var1%%$pattern2}"    #     a
# Longest possible match, strips out last 12 characters  abcd12345abc6789
#                                    ^^^^                 |-------------|

# Remember, # and ## work from the left end (beginning) of string,
#           % and %% work from the right end.

echo

exit 0
```

With the two functions from 10.1:

```tessh
from fnmatch import fnmatchcase

def strip_front s pat longest=False    # ${s#pat}, or ${s##pat} if longest
  ends = if $longest then (range (len $s) -1 -1) else (range ((len $s) + 1))
  for i in $ends
    if (fnmatchcase $s[:$i] $pat)
      return $s[$i:]
  $s

def strip_back s pat longest=False     # ${s%pat}, or ${s%%pat} if longest
  starts = if $longest then (range ((len $s) + 1)) else (range (len $s) -1 -1)
  for i in $starts
    if (fnmatchcase $s[$i:] $pat)
      return $s[:$i]
  $s

var1 = abcd12345abc6789
pattern1 = 'a*c'  # * (wild card) matches everything between a - c.

print
print "var1 = $var1"           # abcd12345abc6789
print "var1 = ${var1}"         # abcd12345abc6789
                               # (alternate form)
print "Number of characters in $var1 = $(len $var1)"
print

print "pattern1 = $pattern1"   # a*c  (everything between 'a' and 'c')
print "--------------"
print '${var1#$pattern1}  =' (strip_front $var1 $pattern1)               #  d12345abc6789
# Shortest possible match, strips out first 3 characters
print '${var1##$pattern1} =' (strip_front $var1 $pattern1 longest=True)  #  6789
# Longest possible match, strips out first 12 characters

print ; print ; print

pattern2 = 'b*9'          # everything between 'b' and '9'
print "var1 = $var1"      # Still  abcd12345abc6789
print
print "pattern2 = $pattern2"
print "--------------"
print '${var1%pattern2}  =' (strip_back $var1 $pattern2)                 #  abcd12345a
# Shortest possible match, strips out last 6 characters
print '${var1%%pattern2} =' (strip_back $var1 $pattern2 longest=True)    #  a
# Longest possible match, strips out last 12 characters

print
```

```output

var1 = abcd12345abc6789
var1 = abcd12345abc6789
Number of characters in abcd12345abc6789 = 16

pattern1 = a*c
--------------
${var1#$pattern1}  = d12345abc6789
${var1##$pattern1} = 6789



var1 = abcd12345abc6789

pattern2 = b*9
--------------
${var1%pattern2}  = abcd12345a
${var1%%pattern2} = a

```

### Example 10-11. Renaming file extensions (`rfe.sh`)

```bash no-run
#!/bin/bash
# rfe.sh: Renaming file extensions.
#
#         rfe old_extension new_extension
#
# Example:
# To rename all *.gif files in working directory to *.jpg,
#          rfe gif jpg


E_BADARGS=65

case $# in
  0|1)             # The vertical bar means "or" in this context.
  echo "Usage: `basename $0` old_file_suffix new_file_suffix"
  exit $E_BADARGS  # If 0 or 1 arg, then bail out.
  ;;
esac


for filename in *.$1
# Traverse list of files ending with 1st argument.
do
  mv $filename ${filename%$1}$2
  #  Strip off part of filename matching 1st argument,
  #+ then append 2nd argument.
done

exit 0
```

```tessh
# (Made-up files to try it on.)
touch a.gif b.gif c.png

E_BADARGS = 65

args = $argv[1:] or [gif jpg]    # (sample arguments, when none are given)

match (len $args)
  case 0 | 1          # The vertical bar means "or" in this context.
    print "Usage: $(Path $argv[0]).name old_file_suffix new_file_suffix"
    exit $E_BADARGS   # If 0 or 1 arg, then bail out.

[old new] = $args[:2]
for filename in *.$old
  # Traverse list of files ending with 1st argument.
  mv $filename ($filename.with_suffix ".$new")

ls
```

```output
a.jpg
b.jpg
c.png
```

### Variable expansion / substring replacement

`${var:pos}` and `${var:pos:len}` are slices; `${var/Pattern/Replacement}`
and `${var//Pattern/Replacement}` are `replace` (or a regex's `sub`), as in
10.1.

### Example 10-12. Using pattern matching to parse arbitrary strings (`ex7.sh`)

```bash
#!/bin/bash

var1=abcd-1234-defg
echo "var1 = $var1"

t=${var1#*-*}
echo "var1 (with everything, up to and including first - stripped out) = $t"
#  t=${var1#*-}  works just the same,
#+ since # matches the shortest string,
#+ and * matches everything preceding, including an empty string.
# (Thanks, Stephane Chazelas, for pointing this out.)

t=${var1##*-*}
echo "If var1 contains a \"-\", returns empty string...   var1 = $t"


t=${var1%*-*}
echo "var1 (with everything from the last - on stripped out) = $t"

echo

# -------------------------------------------
path_name=/home/bozo/ideas/thoughts.for.today
# -------------------------------------------
echo "path_name = $path_name"
t=${path_name##/*/}
echo "path_name, stripped of prefixes = $t"
# Same effect as   t=`basename $path_name` in this particular case.
#  t=${path_name%/}; t=${t##*/}   is a more general solution,
#+ but still fails sometimes.
#  If $path_name ends with a newline, then `basename $path_name` will not work,
#+ but the above expression will.
# (Thanks, S.C.)

t=${path_name%/*.*}
# Same effect as   t=`dirname $path_name`
echo "path_name, stripped of suffixes = $t"
# These will fail in some cases, such as "../", "/foo////", # "foo/", "/".
#  Removing suffixes, especially when the basename has no suffix,
#+ but the dirname does, also complicates matters.
# (Thanks, S.C.)

echo

t=${path_name:11}
echo "$path_name, with first 11 chars stripped off = $t"
t=${path_name:11:5}
echo "$path_name, with first 11 chars stripped off, length 5 = $t"

echo

t=${path_name/bozo/clown}
echo "$path_name with \"bozo\" replaced  by \"clown\" = $t"
t=${path_name/today/}
echo "$path_name with \"today\" deleted = $t"
t=${path_name//o/O}
echo "$path_name with all o's capitalized = $t"
t=${path_name//o/}
echo "$path_name with all o's deleted = $t"

exit 0
```

`partition` and `rpartition` do the first part. For the path, a Path's
`.name` and `.parent` are `basename` and `dirname` without the cases the
comments warn of.

```tessh
var1 = abcd-1234-defg
print "var1 = $var1"

t = ($var1.partition "-")[2]
print "var1 (with everything, up to and including first - stripped out) = $t"

t = if "-" in $var1 then "" else $var1
print "If var1 contains a \"-\", returns empty string...   var1 = $t"

t = ($var1.rpartition "-")[0]
print "var1 (with everything from the last - on stripped out) = $t"

print

# -------------------------------------------
path_name = /home/bozo/ideas/thoughts.for.today
# -------------------------------------------
print "path_name = $path_name"
t = $path_name.name
print "path_name, stripped of prefixes = $t"

t = $path_name.parent
print "path_name, stripped of suffixes = $t"

print

p = str $path_name     # (Slicing and replacing are for text.)
t = $p[11:]
print "$path_name, with first 11 chars stripped off = $t"
t = $p[11:16]
print "$path_name, with first 11 chars stripped off, length 5 = $t"

print

t = $p.replace bozo clown 1
print "$path_name with \"bozo\" replaced  by \"clown\" = $t"
t = $p.replace today "" 1
print "$path_name with \"today\" deleted = $t"
t = $p.replace o O
print "$path_name with all o's capitalized = $t"
t = $p.replace o ""
print "$path_name with all o's deleted = $t"
```

```output
var1 = abcd-1234-defg
var1 (with everything, up to and including first - stripped out) = 1234-defg
If var1 contains a "-", returns empty string...   var1 = 
var1 (with everything from the last - on stripped out) = abcd-1234

path_name = /home/bozo/ideas/thoughts.for.today
path_name, stripped of prefixes = thoughts.for.today
path_name, stripped of suffixes = /home/bozo/ideas

/home/bozo/ideas/thoughts.for.today, with first 11 chars stripped off = ideas/thoughts.for.today
/home/bozo/ideas/thoughts.for.today, with first 11 chars stripped off, length 5 = ideas

/home/bozo/ideas/thoughts.for.today with "bozo" replaced  by "clown" = /home/clown/ideas/thoughts.for.today
/home/bozo/ideas/thoughts.for.today with "today" deleted = /home/bozo/ideas/thoughts.for.
/home/bozo/ideas/thoughts.for.today with all o's capitalized = /hOme/bOzO/ideas/thOughts.fOr.tOday
/home/bozo/ideas/thoughts.for.today with all o's deleted = /hme/bz/ideas/thughts.fr.tday
```

### Example 10-13. Matching patterns at prefix or suffix of string (`var-match.sh`)

```bash
#!/bin/bash
# var-match.sh:
# Demo of pattern replacement at prefix / suffix of string.

v0=abc1234zip1234abc    # Original variable.
echo "v0 = $v0"         # abc1234zip1234abc
echo

# Match at prefix (beginning) of string.
v1=${v0/#abc/ABCDEF}    # abc1234zip1234abc
                        # |-|
echo "v1 = $v1"         # ABCDEF1234zip1234abc
                        # |----|

# Match at suffix (end) of string.
v2=${v0/%abc/ABCDEF}    # abc1234zip123abc
                        #              |-|
echo "v2 = $v2"         # abc1234zip1234ABCDEF
                        #               |----|

echo

#  ----------------------------------------------------
#  Must match at beginning / end of string,
#+ otherwise no replacement results.
#  ----------------------------------------------------
v3=${v0/#123/000}       # Matches, but not at beginning.
echo "v3 = $v3"         # abc1234zip1234abc
                        # NO REPLACEMENT.
v4=${v0/%123/000}       # Matches, but not at end.
echo "v4 = $v4"         # abc1234zip1234abc
                        # NO REPLACEMENT.

exit 0
```

```tessh
v0 = abc1234zip1234abc     # Original variable.
print "v0 = $v0"           # abc1234zip1234abc
print

# Match at prefix (beginning) of string.
v1 = re'^abc'.sub ABCDEF $v0
print "v1 = $v1"           # ABCDEF1234zip1234abc

# Match at suffix (end) of string.
v2 = re'abc$'.sub ABCDEF $v0
print "v2 = $v2"           # abc1234zip1234ABCDEF

print

#  Must match at beginning / end of string,
#+ otherwise no replacement results.
v3 = re'^123'.sub "000" $v0     # Matches, but not at beginning.
print "v3 = $v3"                # abc1234zip1234abc
v4 = re'123$'.sub "000" $v0     # Matches, but not at end.
print "v4 = $v4"                # abc1234zip1234abc
```

```output
v0 = abc1234zip1234abc

v1 = ABCDEF1234zip1234abc
v2 = abc1234zip1234ABCDEF

v3 = abc1234zip1234abc
v4 = abc1234zip1234abc
```

### Variable names by prefix: ${!varprefix*}

`${!varprefix*}` is the names of the variables that start with a prefix,
and `${!name}` the value of the variable named by `$name`. A tessh
script's variables are in `(globals)`, a dict.

```bash
xyz23=whatever
xyz24=

a=${!xyz*}         #  Expands to *names* of declared variables
# ^ ^   ^           + beginning with "xyz".
echo "a = $a"      #  a = xyz23 xyz24
a=${!xyz@}         #  Same as above.
echo "a = $a"      #  a = xyz23 xyz24

echo "---"

abc23=something_else
b=${!abc*}
echo "b = $b"      #  b = abc23
c=${!b}            #  Now, the more familiar type of indirect reference.
echo $c            #  something_else
```

```output bash
a = xyz23 xyz24
a = xyz23 xyz24
---
b = abc23
something_else
```

```tessh
xyz23 = whatever
xyz24 = ""

a = [$n for n in (globals) if ($n.startswith xyz)]
print "a = $(' '.join $a)"     #  a = xyz23 xyz24

print "---"

abc23 = something_else
b = [$n for n in (globals) if ($n.startswith abc)][0]
print "b = $b"                 #  b = abc23
c = (globals)[$b]              #  The value of the variable named $b.
print $c                       #  something_else
```

```output
a = xyz23 xyz24
---
b = abc23
something_else
```

Usually, though, values you want to find by name belong in a dict of
their own.
