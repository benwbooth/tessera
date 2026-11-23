# Appendix O. Exercises
<!-- help: abs-exercises abs-o exercises practice problems -->

The ABS Guide ends with exercises: scripts to analyze, and scripts to
write. (In Hoboken, it says, a dusty brass plaque lists the few who have
mastered them all.) They work as well for tessh; here are the scripts to
analyze, each with a tessh version, and the writing exercises in brief.

## O.1 Analyzing Scripts
<!-- help: abs-scriptanalysis analyze-scripts -->

Run this, explain what it does, and rewrite it more compactly.

```bash
#!/bin/bash

MAX=10000


  for((nr=1; nr<$MAX; nr++))
  do

    let "t1 = nr % 5"
    if [ "$t1" -ne 3 ]
    then
      continue
    fi

    let "t2 = nr % 7"
    if [ "$t2" -ne 4 ]
    then
      continue
    fi

    let "t3 = nr % 9"
    if [ "$t3" -ne 5 ]
    then
      continue
    fi

  break   # What happens when you comment out this line? Why?

  done

  echo "Number = $nr"


exit 0
```

```output bash
Number = 158
```

The same loop in tessh, then the compact version the exercise asks for:
`next` takes the first number of a generator that meets all three
conditions.

```tessh
MAX = 10000

for nr in (range 1 $MAX)
  if $nr % 5 != 3
    continue
  if $nr % 7 != 4
    continue
  if $nr % 9 != 5
    continue
  break     # What happens when you comment out this line? Why?

print "Number = $nr"

# Compactly:
print "Number = $(next ($n for n in (range 1 $MAX) if ($n % 5, $n % 7, $n % 9) == (3, 4, 5)))"
```

```output
Number = 158
Number = 158
```

Explain this "parameterized command-line pipe":

```bash no-run
#!/bin/bash

DIRNAME=/usr/bin
FILETYPE="shell script"
LOGFILE=logfile

file "$DIRNAME"/* | fgrep "$FILETYPE" | tee $LOGFILE | wc -l

exit 0
```

(`no-run`: the count depends on the system.) In tessh it's the same pipe;
here a made-up folder stands in for `/usr/bin`.

```tessh
# (A made-up folder with two shell scripts and a text file.)
mkdir -p bin ; print "#!/bin/sh\necho a" >bin/a ; print "#!/bin/bash\necho b" >bin/b ; print text >bin/c

DIRNAME = ./bin              # (/usr/bin in the original)
FILETYPE = "shell script"
LOGFILE = logfile

file $DIRNAME/* | grep -F $FILETYPE | tee $LOGFILE | wc -l

exit 0
```

```output
2
```

Explain this one by Nathan Coulter (hints: `find` and `stat`):

```bash no-run
#!/bin/bash
# Author:  Nathan Coulter
# This code is released to the public domain.
# The author gave permission to use this code snippet in the ABS Guide.

find -maxdepth 1 -type f -printf '%f\000'  | {
   while read -d $'\000'; do
      mv "$REPLY" "$(date -d "$(stat -c '%y' "$REPLY") " '+%Y%m%d%H%M%S'
      )-$REPLY"
   done
}

# Warning: Test-drive this script in a "scratch" directory.
# It will somehow affect all the files there.
```

(`no-run`: it renames every file in the folder it runs in.) It puts each
file's modification time in front of its name. In tessh the time comes
from `os.path.getmtime`, and the names from a Glob; made-up files with
set times show it.

```tessh
import os datetime
# (A scratch folder with made-up files, their times set.)
touch -d "2014-03-10 12:00:00" notes.txt ; touch -d "2014-03-11 08:30:15" "my essay.txt"

for f in ./*
  if ($f.is_file)
    stamp = (datetime.datetime.fromtimestamp (os.path.getmtime $f)).strftime "%Y%m%d%H%M%S"
    mv $f "$stamp-$f.name"

ls -1
```

```output
20140310120000-notes.txt
20140311083015-my essay.txt
```

A reader wanted to follow the system log, but this hangs. Why? (Hint: a
pipe instead of redirecting the loop's input.)

```bash no-run
while read LINE
do
  echo $LINE
done < `tail -f /var/log/messages`
```

(`no-run`: `tail -f` never ends.) The backquotes wait for `tail -f` to
finish, which it never does, to use its output as a *file name*. With a
pipe, the loop reads the lines as they come. tessh reads a running
program's output line by line through Python's `subprocess`:

```tessh no-run
import subprocess
with (subprocess.Popen [tail -f /var/log/messages] stdout=$subprocess.PIPE text=True) as p
  for line in $p.stdout
    print ($line.rstrip "\n")
```

Analyze Rory Winston's one-liner:

```bash no-run
export SUM=0; for f in $(find src -name "*.java");
do export SUM=$(($SUM + $(wc -l $f | awk '{ print $1 }'))); done; echo $SUM
```

(`no-run`: it needs a `src` folder of Java files.) It adds up the lines of
every `.java` file under `src`. The `export`s are unnecessary, and
`$(find …)` breaks on names with spaces. In tessh the files are a Glob
(`**` goes into subfolders) and the count a sum:

```tessh
# (A made-up src folder.)
mkdir -p src/a/b ; printf "class A {\n}\n" >src/a/A.java ; printf "class B {\n\n}\n" >"src/a/b/B b.java"

print (sum [(len (.splitlines (.read_text $f))) for f in src/**/*.java])
```

```output
5
```

The last analysis exercise is to simplify and speed up the Game of Life
script (Appendix A), and make it read its first generation from any text
file, vowels being live cells.

## O.2 Writing Scripts
<!-- help: abs-writingscripts writing-exercises -->

Write a script for each task. tessh makes some of them much easier than
bash does: floating point, big numbers, dates, CSV and regular
expressions are all at hand.

**Easy.** A script that backs itself up (copies itself to `backup.sh`:
`$argv[0]`). A recursive listing of your home folder, compressed and
copied to a USB drive once `df` shows it mounted. The `for` loops of
Example 11-2 as `while` and `until` loops. Double-, triple- and
single-spacing a file. A script that prints itself backwards. Decompressing
a list of files by what `file` says they are. A 6-digit hex ID for your
computer from the `md5sum` of `/etc/passwd`. A tarball of the files in your
home folder changed in the last day. Checking, every so often, whether a
process is still running. The primes between 60000 and 63000, in columns.
Five different lottery numbers from 1 to 50, printed or saved with the
date.

**Intermediate.** A function telling an integer from a string. C's `atoi`
and `itoa`. Going through the files over 100K under your home folder,
offering to delete or compress each, with a log. Bash's old `banner`.
Removing accounts unused for 90 days. Enforcing disk quotas with `du` and
`mail`. The real names and last logins of the logged-in users. A safe
delete, `sdel`, that compresses files into `~/TRASH` and empties what's
older than 48 hours. Making change for $1.68 in the fewest coins. Solving
a quadratic equation to five decimals. A table of natural logarithms from
0.00 to 100.00. A Unicode table. The sum of the five-digit numbers with
exactly two digits from {4, 5, 6}. The "lucky numbers" (digits summing to
7, repeatedly) from 1000 to 10000. Craps. Tic-tac-toe that never loses.
Alphabetizing a string. `/etc/passwd` as a table. Logins from
`/var/log/messages`. A CSV file in labeled columns. Right-justifying text.
A mailing list. 8-character passwords with at least two digits. Logging a
user's activity for a week. Checking a web site for broken links.

**Difficult.** Testing passwords for strength (length, a digit, a symbol,
no dictionary words). A concordance of a file. Square roots by Newton's
method. Logging accesses to `/etc` for a day. Watching for processes that
spawn more than five children. Stripping a script's comments (but not
`#!`). Stripping HTML to text. XML to HTML and text. Tracing spam to its
sender's ISP. Making man pages from a text file. A hex dump, and then a
disassembler. A 64-bit shift register. Determinants by expanding minors.
A word-find puzzle, and its solver. Anagrams of four letters. Word
ladders. The Gunning fog index. Pi by Buffon's needle. The Playfair
cipher.

The ABS Guide gives the square root as pseudo-code, and warns that bash,
with no floating point, makes it messy. In tessh it's the pseudo-code:

```tessh
def newton_sqrt argument tolerance=0.000001
  #  (Isaac) Newton's Method for speedy extraction of square roots.
  guess = float $argument    # The first guess is the argument itself.
  oldguess = 0.0
  loopcnt = 0
  while (abs ($guess - $oldguess)) > $tolerance
    oldguess = $guess
    guess = ($oldguess + $argument / $oldguess) / 2.0
    loopcnt += 1
  [$guess $loopcnt]

for n in [2 144 1e6]
  [root loops] = newton_sqrt $n
  print "sqrt($n) = ${root:.6f} after $loops loops"
```

```output
sqrt(2) = 1.414214 after 5 loops
sqrt(144) = 12.000000 after 8 loops
sqrt(1000000.0) = 1000.000000 after 15 loops
```

Please don't send the ABS Guide's author your solutions, he asks: bug
fixes and suggestions for the book impress him more.
