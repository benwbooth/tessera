# Appendix C. A Sed and Awk Micro-Primer
<!-- help: abs-sedawk abs-c sed awk stream-editor text-processing -->

`sed` edits a stream of text line by line, and `awk` splits each line into
fields and runs a small C-like program on them. Both read standard input
(or files), write standard output, and use regular expressions, so they
fit in pipes. They're programs, so a tessh script runs them exactly as a
bash one does; the only thing to watch is quoting, which works the same
way (single quotes keep `$1` for awk, double quotes fill in tessh's
variables). For many jobs Python does the same in the script itself: `re.sub`
for `s///`, `.split` for awk's fields, a comprehension for `/pattern/p`.

## C.1 Sed
<!-- help: abs-sed sed s/// substitute delete print address -->

A sed command is an address (a line number, a range, a `/pattern/`) and an
operation: `p` print, `d` delete, `s/old/new/` substitute (`g` for every
match on the line), `y/abc/xyz/` translate characters, `i` insert. With no
address, every line.

| sed | effect | in Python |
|---|---|---|
| `8d` | delete line 8 | `[$l for [i l] in (enumerate $lines 1) if $i != 8]` |
| `/^$/d` | delete blank lines | `[$l for l in $lines if $l]` |
| `1,/^$/d` | delete up to the first blank line | `$lines[($lines.index "") + 1:]` |
| `-n /Jones/p` | print the lines with Jones | `[$l for l in $lines if "Jones" in $l]` |
| `s/Windows/Linux/` | replace the first Windows on each line | `$line.replace Windows Linux 1` |
| `s/BSOD/stability/g` | replace every one | `$line.replace BSOD stability` |
| `s/ *$//` | delete trailing spaces | `$line.rstrip " "` |
| `s/00*/0/g` | squeeze runs of zeros | `re.sub r"00*" "0" $line` |
| `/GUI/d`, `s/GUI//g` | delete lines with GUI, delete GUI | a filter, `.replace GUI ""` |
| `y/abc/xyz/` | translate characters | `$line.translate (str.maketrans abc xyz)` |
| `G` | double-space | `"\n\n".join $lines` |

As in bash, a sed command that uses a variable goes in double quotes:

```bash
printf "BEGIN here\nkeep this\n\nBEGIN again\nJones was here\n" > file1.txt
filename=file1.txt
pattern=BEGIN
sed "/^$pattern/d" "$filename"  # Works as specified.
sed -n '/Jones/p' $filename
echo "Working on it." | sed -e '1i How far are you along?'
```

```output bash
keep this

Jones was here
Jones was here
How far are you along?
Working on it.
```

```tessh
printf "BEGIN here\nkeep this\n\nBEGIN again\nJones was here\n" > file1.txt
filename = file1.txt
pattern = BEGIN
sed "/^$pattern/d" $filename    # Works as specified.
sed -n '/Jones/p' $filename
echo "Working on it." | sed -e '1i How far are you along?'
# The first two in Python:
lines = (cat $filename).lines
print ("\n".join [$l for l in $lines if not ($l.startswith $pattern)])
print ("\n".join [$l for l in $lines if "Jones" in $l])
```

```output
keep this

Jones was here
Jones was here
How far are you along?
Working on it.
keep this

Jones was here
Jones was here
```

## C.2 Awk
<!-- help: abs-awk awk fields $1 $0 BEGIN END -->

Awk splits each line into fields at whitespace: `$1` is the first, `$0`
the whole line. `{ … }` runs for every line, `BEGIN { … }` before the
first and `END { … }` after the last. In tessh, awk's program goes in single
quotes, so tessh leaves its `$1` alone.

```bash
echo one two | awk '{print $1}'
echo one two | awk '{print $2}'
echo one two | awk '{print $0}'
printf "a 1\nb 2\nc 3\n" | awk '{ total += $2 } END { print total }'
```

```output bash
one
two
one two
6
```

```tessh
echo one two | awk '{print $1}'
echo one two | awk '{print $2}'
echo one two | awk '{print $0}'
printf "a 1\nb 2\nc 3\n" | awk '{ total += $2 } END { print total }'
# The fields in Python:
print ("one two".split)[1]
print (sum [(int ($l.split)[1]) for l in ("a 1\nb 2\nc 3".splitlines)])
```

```output
one
two
one two
6
two
6
```

### Example C-1. Counting Letter Occurrences (`letter-count2.sh`)

```bash no-run
#! /bin/sh
# letter-count2.sh: Counting letter occurrences in a text file.
#
# Script by nyal [nyal@voila.fr].
# Used in ABS Guide with permission.
# Recommented and reformatted by ABS Guide author.
# Version 1.1: Modified to work with gawk 3.1.3.
#              (Will still work with earlier versions.)


INIT_TAB_AWK=""
# Parameter to initialize awk script.
count_case=0
FILE_PARSE=$1

E_PARAMERR=85

usage()
{
    echo "Usage: letter-count.sh file letters" 2>&1
    # For example:   ./letter-count2.sh filename.txt a b c
    exit $E_PARAMERR  # Too few arguments passed to script.
}

if [ ! -f "$1" ] ; then
    echo "$1: No such file." 2>&1
    usage                 # Print usage message and exit.
fi 

if [ -z "$2" ] ; then
    echo "$2: No letters specified." 2>&1
    usage
fi 

shift                      # Letters specified.
for letter in `echo $@`    # For each one . . .
  do
  INIT_TAB_AWK="$INIT_TAB_AWK tab_search[${count_case}] = \
  \"$letter\"; final_tab[${count_case}] = 0; " 
  # Pass as parameter to awk script below.
  count_case=`expr $count_case + 1`
done

# DEBUG:
# echo $INIT_TAB_AWK;

cat $FILE_PARSE |
# Pipe the target file to the following awk script.

# ---------------------------------------------------------------------
# Earlier version of script:
# awk -v tab_search=0 -v final_tab=0 -v tab=0 -v \
# nb_letter=0 -v chara=0 -v chara2=0 \

awk \
"BEGIN { $INIT_TAB_AWK } \
{ split(\$0, tab, \"\"); \
for (chara in tab) \
{ for (chara2 in tab_search) \
{ if (tab_search[chara2] == tab[chara]) { final_tab[chara2]++ } } } } \
END { for (chara in final_tab) \
{ print tab_search[chara] \" => \" final_tab[chara] } }"
# ---------------------------------------------------------------------
#  Nothing all that complicated, just . . .
#+ for-loops, if-tests, and a couple of specialized functions.

exit $?

# Compare this script to letter-count.sh.
```

The script builds awk's `BEGIN` block out of its arguments. In tessh the
awk program can stay as written, in single quotes, with the letters
handed over by `-v`; the counting is also shown in Python, with
`collections.Counter`. A made-up file and sample arguments stand in when
there are none.

```tessh
from collections import Counter
import sys
E_PARAMERR = 85

# (A made-up file to count in, and sample arguments.)
print "The quick brown fox jumps over the lazy dog.\nA bad cab." >sample.txt
args = $argv[1:] or [sample.txt a b c]

def usage
  print "Usage: letter-count.sh file letters" file=$sys.stderr
  exit $E_PARAMERR  # Too few arguments passed to script.

if not $args or not ((Path $args[0]).is_file)
  print "$(if $args then $args[0] else ''): No such file." file=$sys.stderr
  usage
[FILE_PARSE *letters] = $args
if not $letters
  print ": No letters specified." file=$sys.stderr
  usage

cat $FILE_PARSE | awk -v "letters=$(' '.join $letters)" '
BEGIN { n = split(letters, tab_search, " ") }
{ split($0, tab, "")
  for (chara in tab)
    for (chara2 = 1; chara2 <= n; chara2++)
      if (tab_search[chara2] == tab[chara]) final_tab[chara2]++ }
END { for (chara = 1; chara <= n; chara++)
        print tab_search[chara] " => " final_tab[chara] + 0 }'

# The same count, in Python:
counts = Counter (cat $FILE_PARSE)
for letter in $letters
  print "$letter => $counts[$letter]"
```

```output
a => 3
b => 3
c => 2
a => 3
b => 3
c => 2
```
