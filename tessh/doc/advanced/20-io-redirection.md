# 20. I/O Redirection
<!-- help: abs-io-redirection abs-20 redirection file-descriptor stdin stdout stderr -->

A program starts with three open files: standard input (0, the keyboard),
standard output (1, the screen) and standard error (2, the screen too, for
error messages). Each open file has a number, its file descriptor, and any
of them can be redirected: to a file, from a file, or to another
descriptor. Descriptors 3 to 9 are free for a script's own use.

tessh's redirections are bash's, written the same way, and done left to
right as in bash:

| redirection | does |
|---|---|
| `>f`, `1>f` | output to file `f`, emptied first |
| `>>f`, `1>>f` | output added to the end of `f` |
| `2>f`, `2>>f` | errors to `f` |
| `&>f`, `&>>f` | output and errors to `f` |
| `M>f`, `M>&N` | descriptor M to the file, or to where N goes |
| `2>&1`, `>&2` | errors where output goes, or output where errors go |
| `<f`, `0<f` | input from `f` |
| `M<>f` | `f` opened for reading and writing as M (0 without M) |
| `M>&-`, `M<&-` | descriptor M closed |
| `<<END`, `<<<word` | a heredoc, a here string (chapter 19) |

On a program they're as in bash. On a Python call (`print`, a function of
your own) descriptors 0, 1 and 2 are redirected: the call's `sys.stdin`,
`sys.stdout` and `sys.stderr`, and the programs it runs get them too. The
same goes for a function's `print`s and programs, which is how a block of
tessh code is redirected (below).

`: >filename` empties a file or makes an empty one, as in bash; a bare
`>filename` with no command doesn't work in tessh.

```bash
# Redirecting stdout, one line at a time.
LOGFILE=script.log

echo "This statement is sent to the log file, \"$LOGFILE\"." 1>$LOGFILE
echo "This statement is appended to \"$LOGFILE\"." 1>>$LOGFILE
echo "This statement is also appended to \"$LOGFILE\"." 1>>$LOGFILE
echo "This statement is echoed to stdout, and will not appear in \"$LOGFILE\"."
# These redirection commands automatically "reset" after each line.

# Redirecting stderr, one line at a time.
ERRORFILE=script.errors

bad_command1 2>$ERRORFILE       #  Error message sent to $ERRORFILE.
bad_command2 2>>$ERRORFILE      #  Error message appended to $ERRORFILE.
bad_command3                    #  Error message echoed to stderr,
                                #+ and does not appear in $ERRORFILE.
# These redirection commands also automatically "reset" after each line.
```

```output bash
This statement is echoed to stdout, and will not appear in "script.log".
```

The tessh version shows the two files at the end. For the bad commands it
runs `ls` on files that aren't there: tessh's own "command not found"
message doesn't follow a `2>` (a bug, logged).

```tessh
# Redirecting stdout, one line at a time.
LOGFILE = script.log

echo "This statement is sent to the log file, \"$LOGFILE\"." 1>$LOGFILE
echo "This statement is appended to \"$LOGFILE\"." 1>>$LOGFILE
print "This statement is also appended to \"$LOGFILE\"." 1>>$LOGFILE
echo "This statement is echoed to stdout, and will not appear in \"$LOGFILE\"."
# These redirection commands automatically "reset" after each line.

# Redirecting stderr, one line at a time.
ERRORFILE = script.errors

ls bad_file1 2>$ERRORFILE       #  Error message sent to $ERRORFILE.
ls bad_file2 2>>$ERRORFILE      #  Error message appended to $ERRORFILE.
ls bad_file3                    #  Error message echoed to stderr,
                                #+ and does not appear in $ERRORFILE.
cat $LOGFILE $ERRORFILE         # (To see them.)
```

```output
This statement is echoed to stdout, and will not appear in "script.log".
This statement is sent to the log file, "script.log".
This statement is appended to "script.log".
This statement is also appended to "script.log".
ls: cannot access 'bad_file1': No such file or directory
ls: cannot access 'bad_file2': No such file or directory
```

Where errors go when output is redirected too depends on the order, in
tessh as in bash. `2>&1` makes errors go where output goes *now*.

```bash
ls -yz >> command.log 2>&1
#  Capture result of illegal options "yz" in file "command.log."
#  Because stderr is redirected to the file,
#+ any error messages will also be there.

#  Note, however, that the following does *not* give the same result.
ls -yz 2>&1 >> command.log
#  Outputs an error message, but does not write to file.
#  More precisely, the command output (in this case, null)
#+ writes to the file, but the error message goes only to stdout.

#  If redirecting both stdout and stderr,
#+ the order of the commands makes a difference.
```

```tessh
ls -yz >> command.log 2>&1
#  Capture result of illegal options "yz" in file "command.log."

#  Note, however, that the following does *not* give the same result.
ls -yz 2>&1 >> command.log
#  Outputs an error message, but does not write to file.
```

```output
ls: invalid option -- 'y'
Try 'ls --help' for more information.
```

`2>&1 | command` sends errors down a pipe with the output (bash 4 also
writes it `|&`, which tessh doesn't have).

`<>` opens a file for reading and writing, which lets a script write at a
given place in it. bash does it with a descriptor kept open by `exec`; in
tessh, with Python's `os.open`, which gives a descriptor too. (`open File
r+` is the usual Python way, but its text-mode reads read ahead, so a write
after one lands at the end; seek first.)

```bash
echo 1234567890 > File    # Write string to "File".
exec 3<> File             # Open "File" and assign fd 3 to it.
read -n 4 <&3             # Read only 4 characters.
echo -n . >&3             # Write a decimal point there.
exec 3>&-                 # Close fd 3.
cat File                  # ==> 1234.67890
#  Random access, by golly.
```

```tessh
import os
echo 1234567890 > File      # Write string to "File".
fd = os.open File $os.O_RDWR   # Open "File" for reading and writing.
os.read $fd 4               # Read only 4 characters.
os.write $fd b"."           # Write a decimal point there.
os.close $fd                # Close it.
cat File                    # ==> 1234.67890
#  Random access, by golly.
```

```output
1234.67890
```

Several redirections and pipes go on one command, as in bash:
`command <input-file >output-file`, `cat *.txt | sort | uniq >result-file`.
(bash's `<input-file command` with the redirection first isn't tessh.)

## Closing file descriptors
<!-- help: abs-closing-fds close-fd >&- <&- -->

`n<&-` and `n>&-` close descriptor n for a command; `>&-` closes its
output. A program inherits its shell's open descriptors; closing one keeps
it from inheriting that one. This sends only the errors of `ls` down the
pipe, keeping its output on the screen through descriptor 3:

```bash
# Redirecting only stderr to a pipe.

exec 3>&1                              # Save current "value" of stdout.
ls -l 2>&1 >&3 3>&- | grep bad 3>&-    # Close fd 3 for 'grep' (but not 'ls').
#              ^^^^   ^^^^
exec 3>&-                              # Now close it for the remainder of the script.

# Thanks, S.C.
```

tessh has no `exec`, but the descriptors are the process's own, so
Python's `os.dup2` and `os.close` do what bash's `exec 3>&1` and
`exec 3>&-` do, and the programs tessh runs inherit them.

```tessh
import os
# Redirecting only stderr to a pipe.

os.dup2 1 3                            # Save current "value" of stdout.
ls -l 2>&1 >&3 3>&- | grep bad 3>&-    # Close fd 3 for 'grep' (but not 'ls').
os.close 3                             # Now close it for the remainder of the script.
```

```output
total 0
```

## 20.1 Using exec
<!-- help: abs-usingexec exec redirect-script sys.stdin sys.stdout dup2 -->

In bash, `exec <filename` makes a file the standard input of the rest of
the script, and `exec >filename` its standard output; `exec 6<&0` keeps
the old one in descriptor 6 so it can be put back. tessh has no `exec`.
What it has instead:

- for Python code (`print`, `input`, `sys.stdin.read`), set `$sys.stdin`
  or `$sys.stdout` to an open file, and back again after;
- for a stretch of code that runs programs too, put it in a function and
  redirect the call: `report >logfile`, `convert <in >out`. That's the
  block's redirection, and it ends with the call, so nothing needs putting
  back;
- for the descriptors themselves, `os.dup2`, as above.

**Gap:** `exec` itself, both for redirecting the rest of a script and for
replacing the shell with a program (`exec command`).

### Example 20-1. Redirecting stdin using exec (`redir1.sh`)

```bash no-run
#!/bin/bash
# Redirecting stdin using 'exec'.


exec 6<&0          # Link file descriptor #6 with stdin.
                   # Saves stdin.

exec < data-file   # stdin replaced by file "data-file"

read a1            # Reads first line of file "data-file".
read a2            # Reads second line of file "data-file."

echo
echo "Following lines read from file."
echo "-------------------------------"
echo $a1
echo $a2

echo; echo; echo

exec 0<&6 6<&-
#  Now restore stdin from fd #6, where it had been saved,
#+ and close fd #6 ( 6<&- ) to free it for other processes to use.
#
# <&6 6<&-    also works.

echo -n "Enter data  "
read b1  # Now "read" functions as expected, reading from normal stdin.
echo "Input read from stdin."
echo "----------------------"
echo "b1 = $b1"

echo

exit 0
```

Here `input` reads from a file put in `$sys.stdin`, as bash's `read` reads
from the file `exec` made its standard input. The last `read` is from the
keyboard; that's answered here, since these examples can't use one.

```tessh
import sys
# (A made-up data-file to try it on.)
printf "This is line 1 of data-file.\nThis is line 2.\nAnd line 3.\n" >data-file

saved = $sys.stdin                # Saves stdin.

$sys.stdin = (open data-file)     # stdin replaced by file "data-file"

a1 = (input)            # Reads first line of file "data-file".
a2 = (input)            # Reads second line of file "data-file."

print
print "Following lines read from file."
print "-------------------------------"
print $a1
print $a2

print ; print ; print

$sys.stdin.close
$sys.stdin = $saved
#  Now restore stdin from where it had been saved.

print "Enter data  " end=""
b1 = "xyzzy"            # (b1 = (input) reads from normal stdin: answered here)
print
print "Input read from stdin."
print "----------------------"
print "b1 = $b1"

print

exit 0
```

```output

Following lines read from file.
-------------------------------
This is line 1 of data-file.
This is line 2.



Enter data  
Input read from stdin.
----------------------
b1 = xyzzy

```

### Example 20-2. Redirecting stdout using exec (`reassign-stdout.sh`)

```bash no-run
#!/bin/bash
# reassign-stdout.sh

LOGFILE=logfile.txt

exec 6>&1           # Link file descriptor #6 with stdout.
                    # Saves stdout.

exec > $LOGFILE     # stdout replaced with file "logfile.txt".

# ----------------------------------------------------------- #
# All output from commands in this block sent to file $LOGFILE.

echo -n "Logfile: "
date
echo "-------------------------------------"
echo

echo "Output of \"ls -al\" command"
echo
ls -al
echo; echo
echo "Output of \"df\" command"
echo
df

# ----------------------------------------------------------- #

exec 1>&6 6>&-      # Restore stdout and close file descriptor #6.

echo
echo "== stdout now restored to default == "
echo
ls -al
echo

exit 0
```

The block that goes to the log file is a function, called with `>`: its
`print`s and the programs it runs (`date`, `ls`, `df`) all write to the
file. The end shows the file's headings, as its dates and sizes change from
run to run.

```tessh
LOGFILE = logfile.txt

def report
  # ----------------------------------------------------------- #
  # All output from commands in this block sent to file $LOGFILE.

  print "Logfile: " end=""
  date
  print "-------------------------------------"
  print

  print "Output of \"ls -al\" command"
  print
  ls -al
  print ; print
  print "Output of \"df\" command"
  print
  df
  # ----------------------------------------------------------- #

report >$LOGFILE      # stdout replaced with file "logfile.txt", for report.

print
print "== stdout now restored to default == "
print
ls
grep '^Output of' $LOGFILE     # (What went into it.)
print

exit 0
```

```output

== stdout now restored to default == 

logfile.txt
Output of "ls -al" command
Output of "df" command

```

### Example 20-3. Redirecting both stdin and stdout in the same script with exec (`upperconv.sh`)

```bash no-run
#!/bin/bash
# upperconv.sh
# Converts a specified input file to uppercase.

E_FILE_ACCESS=70
E_WRONG_ARGS=71

if [ ! -r "$1" ]     # Is specified input file readable?
then
  echo "Can't read from input file!"
  echo "Usage: $0 input-file output-file"
  exit $E_FILE_ACCESS
fi                   #  Will exit with same error
                     #+ even if input file ($1) not specified (why?).

if [ -z "$2" ]
then
  echo "Need to specify output file."
  echo "Usage: $0 input-file output-file"
  exit $E_WRONG_ARGS
fi


exec 4<&0
exec < $1            # Will read from input file.

exec 7>&1
exec > $2            # Will write to output file.
                     # Assumes output file writable (add check?).

# -----------------------------------------------
    cat - | tr a-z A-Z   # Uppercase conversion.
#   ^^^^^                # Reads from stdin.
#           ^^^^^^^^^^   # Writes to stdout.
# However, both stdin and stdout were redirected.
# Note that the 'cat' can be omitted.
# -----------------------------------------------

exec 1>&7 7>&-       # Restore stout.
exec 0<&4 4<&-       # Restore stdin.

# After restoration, the following line prints to stdout as expected.
echo "File \"$1\" written to \"$2\" as uppercase conversion."

exit 0
```

A function redirected both ways. (Just `tr a-z A-Z <$in >$out` would do,
of course.)

```tessh
import os
E_FILE_ACCESS = 70
E_WRONG_ARGS = 71
print "Some lower-case text.\nAnd more." >lower.txt   # (A made-up input file.)
args = $argv[1:] or [lower.txt upper.txt]   # (sample arguments, when none are given)

if not $args or not (os.access $args[0] $os.R_OK)   # Is input file readable?
  print "Can't read from input file!"
  print "Usage: $(Path $argv[0]).name input-file output-file"
  exit $E_FILE_ACCESS

if (len $args) < 2
  print "Need to specify output file."
  print "Usage: $(Path $argv[0]).name input-file output-file"
  exit $E_WRONG_ARGS

[infile outfile] = $args[:2]

def convert
  # -----------------------------------------------
  cat - | tr a-z A-Z   # Uppercase conversion.
  #   ^^^^^                # Reads from stdin.
  #           ^^^^^^^^^^   # Writes to stdout.
  # But both are redirected, for the call below.
  # -----------------------------------------------

convert <$infile >$outfile    # Will read from input file, write to output file.

# After the call, output goes to stdout as before.
print "File \"$infile\" written to \"$outfile\" as uppercase conversion."
cat $outfile                  # (To see it.)

exit 0
```

```output
File "lower.txt" written to "upper.txt" as uppercase conversion.
SOME LOWER-CASE TEXT.
AND MORE.
```

In bash, a loop at the end of a pipe runs in a subshell, so the variables
it changes are lost when it ends; reading through a descriptor opened by
`exec` avoids that. A tessh loop always runs in the script itself.

### Example 20-4. Avoiding a subshell (`avoid-subshell.sh`)

```bash no-run
#!/bin/bash
# avoid-subshell.sh
# Suggested by Matthew Walker.

Lines=0

echo

cat myfile.txt | while read line;
                 do {
                   echo $line
                   (( Lines++ ));  #  Incremented values of this variable
                                   #+ inaccessible outside loop.
                                   #  Subshell problem.
                 }
                 done

echo "Number of lines read = $Lines"     # 0
                                         # Wrong!

echo "------------------------"


exec 3<> myfile.txt
while read line <&3
do {
  echo "$line"
  (( Lines++ ));                   #  Incremented values of this variable
                                   #+ accessible outside loop.
                                   #  No subshell, no problem.
}
done
exec 3>&-

echo "Number of lines read = $Lines"     # 8

echo

exit 0

# Lines below not seen by script.

$ cat myfile.txt

Line 1.
Line 2.
Line 3.
Line 4.
Line 5.
Line 6.
Line 7.
Line 8.
```

The first loop goes over what `cat` printed, the second over the open
file; both count all 8 lines, as there's no subshell to lose the count in.

```tessh
# (The myfile.txt the original shows at its end.)
printf "Line %d.\n" 1 2 3 4 5 6 7 8 >myfile.txt

Lines = 0

print

for line in (cat myfile.txt).lines
  print $line
  Lines += 1          #  Incremented values of this variable
                      #+ still there outside the loop.

print "Number of lines read = $Lines"     # 8
                                          # Right!

print "------------------------"

Lines = 0
with (open myfile.txt) as f
  for line in $f
    print ($line.rstrip "\n")
    Lines += 1

print "Number of lines read = $Lines"     # 8

print

exit 0
```

```output

Line 1.
Line 2.
Line 3.
Line 4.
Line 5.
Line 6.
Line 7.
Line 8.
Number of lines read = 8
------------------------
Line 1.
Line 2.
Line 3.
Line 4.
Line 5.
Line 6.
Line 7.
Line 8.
Number of lines read = 8

```

## 20.2 Redirecting Code Blocks
<!-- help: abs-redircb redirect-block redirected-loop while-redirect function-redirect -->

In bash a whole `while`, `until` or `for` loop, an `if` block or a function
can take a redirection after its end: `done <file`, `fi <file`. tessh's
loops and `if`s can't; there are two ways instead. A loop over a file's
lines goes over the open file (`for line in (open file)`), which is the
usual way. Or the block goes in a function, and the call is redirected,
`block <file >out`; then `input` (bash's `read`) and the programs in it
read the file. The examples below use both.

**Gap:** no redirection after a loop or `if` block.

These examples read names.data, Example 20-11. Each makes it first, with
one `printf`.

### Example 20-5. Redirected while loop (`redir2.sh`)

```bash no-run
#!/bin/bash
# redir2.sh

if [ -z "$1" ]
then
  Filename=names.data       # Default, if no filename specified.
else
  Filename=$1
fi  
#+ Filename=${1:-names.data}
#  can replace the above test (parameter substitution).

count=0

echo

while [ "$name" != Smith ]  # Why is variable $name in quotes?
do
  read name                 # Reads from $Filename, rather than stdin.
  echo $name
  let "count += 1"
done <"$Filename"           # Redirects stdin to file $Filename. 
#    ^^^^^^^^^^^^

echo; echo "$count names read"; echo

exit 0

#  Note that in some older shell scripting languages,
#+ the redirected loop would run as a subshell.
#  Therefore, $count would return 0, the initialized value outside the loop.
#  Bash and ksh avoid starting a subshell *whenever possible*,
#+ so that this script, for example, runs correctly.
#  (Thanks to Heiner Steven for pointing this out.)

#  However . . .
#  Bash *can* sometimes start a subshell in a PIPED "while-read" loop,
#+ as distinct from a REDIRECTED "while" loop.

abc=hi
echo -e "1\n2\n3" | while read l
     do abc="$l"
        echo $abc
     done
echo $abc

#  Thanks, Bruno de Oliveira Schneider, for demonstrating this
#+ with the above snippet of code.
#  And, thanks, Brian Onn, for correcting an annotation error.
```

The loop is in a function, and its call takes the redirection that bash
puts after `done`. `count` is the function's own, given back at the end.

```tessh
# (names.data, Example 20-11, to read.)
printf "%s\n" Aristotle Arrhenius Belisarius Capablanca Dickens Euler Goethe \
  Hegel Jonah Laplace Maroczy Purcell Schmidt Schopenhauer Semmelweiss Smith \
  Steinmetz Tukhashevsky Turing Venn Warshawski Znosko-Borowski >names.data

Filename = if (len $argv) > 1 and $argv[1] then $argv[1] else names.data
#  (Default, if no filename specified.)

def read_names
  count = 0
  name = ""
  while $name != Smith
    name = (input)          # Reads from $Filename, rather than stdin.
    print $name
    count += 1
  $count

print
count = read_names <$Filename   # Redirects stdin to file $Filename.
#                  ^^^^^^^^^^
print ; print "$count names read" ; print

exit 0
```

```output

Aristotle
Arrhenius
Belisarius
Capablanca
Dickens
Euler
Goethe
Hegel
Jonah
Laplace
Maroczy
Purcell
Schmidt
Schopenhauer
Semmelweiss
Smith

16 names read

```

The original goes on, after its `exit`, to show the subshell problem: a
loop at the end of a pipe changes its variables in a subshell. bash prints
`hi` at the end; tessh's loop is in the script, so the variable keeps its
last value.

```bash
abc=hi
echo -e "1\n2\n3" | while read l
     do abc="$l"
        echo $abc
     done
echo $abc
```

```output bash
1
2
3
hi
```

```tessh
abc = hi
for l in (printf "1\n2\n3\n").lines
  abc = $l
  print $abc
print $abc
```

```output
1
2
3
3
```

### Example 20-6. Alternate form of redirected while loop (`redir2a.sh`)

```bash no-run
#!/bin/bash

# This is an alternate form of the preceding script.

#  Suggested by Heiner Steven
#+ as a workaround in those situations when a redirect loop
#+ runs as a subshell, and therefore variables inside the loop
# +do not keep their values upon loop termination.


if [ -z "$1" ]
then
  Filename=names.data     # Default, if no filename specified.
else
  Filename=$1
fi  


exec 3<&0                 # Save stdin to file descriptor 3.
exec 0<"$Filename"        # Redirect standard input.

count=0
echo


while [ "$name" != Smith ]
do
  read name               # Reads from redirected stdin ($Filename).
  echo $name
  let "count += 1"
done                      #  Loop reads from file $Filename
                          #+ because of line 20.

#  The original version of this script terminated the "while" loop with
#+      done <"$Filename" 
#  Exercise:
#  Why is this unnecessary?


exec 0<&3                 # Restore old stdin.
exec 3<&-                 # Close temporary fd 3.

echo; echo "$count names read"; echo

exit 0
```

This form, `exec` around a plain loop, is `$sys.stdin` set around it.

```tessh
import sys
# (names.data, Example 20-11, to read.)
printf "%s\n" Aristotle Arrhenius Belisarius Capablanca Dickens Euler Goethe \
  Hegel Jonah Laplace Maroczy Purcell Schmidt Schopenhauer Semmelweiss Smith \
  Steinmetz Tukhashevsky Turing Venn Warshawski Znosko-Borowski >names.data

Filename = if (len $argv) > 1 and $argv[1] then $argv[1] else names.data

saved = $sys.stdin                # Save stdin.
$sys.stdin = (open $Filename)     # Redirect standard input.

count = 0
name = ""
print

while $name != Smith
  name = (input)          # Reads from redirected stdin ($Filename).
  print $name
  count += 1

$sys.stdin.close
$sys.stdin = $saved       # Restore old stdin.

print ; print "$count names read" ; print

exit 0
```

```output

Aristotle
Arrhenius
Belisarius
Capablanca
Dickens
Euler
Goethe
Hegel
Jonah
Laplace
Maroczy
Purcell
Schmidt
Schopenhauer
Semmelweiss
Smith

16 names read

```

### Example 20-7. Redirected until loop (`redir3.sh`)

```bash no-run
#!/bin/bash
# Same as previous example, but with "until" loop.

if [ -z "$1" ]
then
  Filename=names.data         # Default, if no filename specified.
else
  Filename=$1
fi  

# while [ "$name" != Smith ]
until [ "$name" = Smith ]     # Change  !=  to =.
do
  read name                   # Reads from $Filename, rather than stdin.
  echo $name
done <"$Filename"             # Redirects stdin to file $Filename. 
#    ^^^^^^^^^^^^

# Same results as with "while" loop in previous example.

exit 0
```

tessh has no `until`: it's `while not`. Here the loop reads the open file
with `.readline`, as bash's `read` reads its redirected input.

```tessh
# (names.data, Example 20-11, to read.)
printf "%s\n" Aristotle Arrhenius Belisarius Capablanca Dickens Euler Goethe \
  Hegel Jonah Laplace Maroczy Purcell Schmidt Schopenhauer Semmelweiss Smith \
  Steinmetz Tukhashevsky Turing Venn Warshawski Znosko-Borowski >names.data

Filename = if (len $argv) > 1 and $argv[1] then $argv[1] else names.data

name = ""
with (open $Filename) as f
  # while $name != Smith
  while not ($name == Smith)        # (until, in the original)
    name = (.strip ($f.readline))   # Reads from $Filename.
    print $name

# Same results as with "while" loop in previous example.

exit 0
```

```output
Aristotle
Arrhenius
Belisarius
Capablanca
Dickens
Euler
Goethe
Hegel
Jonah
Laplace
Maroczy
Purcell
Schmidt
Schopenhauer
Semmelweiss
Smith
```

### Example 20-8. Redirected for loop (`redir4.sh`)

```bash no-run
#!/bin/bash

if [ -z "$1" ]
then
  Filename=names.data          # Default, if no filename specified.
else
  Filename=$1
fi  

line_count=`wc $Filename | awk '{ print $1 }'`
#           Number of lines in target file.
#
#  Very contrived and kludgy, nevertheless shows that
#+ it's possible to redirect stdin within a "for" loop...
#+ if you're clever enough.
#
# More concise is     line_count=$(wc -l < "$Filename")


for name in `seq $line_count`  # Recall that "seq" prints sequence of numbers.
# while [ "$name" != Smith ]   --   more complicated than a "while" loop   --
do
  read name                    # Reads from $Filename, rather than stdin.
  echo $name
  if [ "$name" = Smith ]       # Need all this extra baggage here.
  then
    break
  fi  
done <"$Filename"              # Redirects stdin to file $Filename. 
#    ^^^^^^^^^^^^

exit 0
```

The `for` loop over `seq` in a function, redirected, as the original
redirects the loop.

```tessh
# (names.data, Example 20-11, to read.)
printf "%s\n" Aristotle Arrhenius Belisarius Capablanca Dickens Euler Goethe \
  Hegel Jonah Laplace Maroczy Purcell Schmidt Schopenhauer Semmelweiss Smith \
  Steinmetz Tukhashevsky Turing Venn Warshawski Znosko-Borowski >names.data

Filename = if (len $argv) > 1 and $argv[1] then $argv[1] else names.data

line_count = wc $Filename | awk '{ print $1 }'
#           Number of lines in target file.
#
# More concise is     line_count = (len (open $Filename).readlines)

def names_loop
  for n in (seq $line_count).lines  # Recall that "seq" prints sequence of numbers.
    name = (input)                  # Reads from $Filename, rather than stdin.
    print $name
    if $name == Smith               # Need all this extra baggage here.
      break

names_loop <$Filename               # Redirects stdin to file $Filename.

exit 0
```

```output
Aristotle
Arrhenius
Belisarius
Capablanca
Dickens
Euler
Goethe
Hegel
Jonah
Laplace
Maroczy
Purcell
Schmidt
Schopenhauer
Semmelweiss
Smith
```

The loop's output can be redirected too.

### Example 20-9. Redirected for loop (both stdin and stdout redirected) (`redir4a.sh`)

```bash no-run
#!/bin/bash

if [ -z "$1" ]
then
  Filename=names.data          # Default, if no filename specified.
else
  Filename=$1
fi  

Savefile=$Filename.new         # Filename to save results in.
FinalName=Jonah                # Name to terminate "read" on.

line_count=`wc $Filename | awk '{ print $1 }'`  # Number of lines in target file.


for name in `seq $line_count`
do
  read name
  echo "$name"
  if [ "$name" = "$FinalName" ]
  then
    break
  fi  
done < "$Filename" > "$Savefile"     # Redirects stdin to file $Filename,
#    ^^^^^^^^^^^^^^^^^^^^^^^^^^^       and saves it to backup file.

exit 0
```

```tessh
# (names.data, Example 20-11, to read.)
printf "%s\n" Aristotle Arrhenius Belisarius Capablanca Dickens Euler Goethe \
  Hegel Jonah Laplace Maroczy Purcell Schmidt Schopenhauer Semmelweiss Smith \
  Steinmetz Tukhashevsky Turing Venn Warshawski Znosko-Borowski >names.data

Filename = if (len $argv) > 1 and $argv[1] then $argv[1] else names.data

Savefile = "${Filename}.new"     # Filename to save results in.
FinalName = Jonah                # Name to terminate "read" on.

line_count = wc $Filename | awk '{ print $1 }'  # Number of lines in target file.

def names_loop
  for n in (seq $line_count).lines
    name = (input)
    print $name
    if $name == $FinalName
      break

names_loop <$Filename >$Savefile  # Redirects stdin to file $Filename,
#          ^^^^^^^^^^^^^^^^^^^^^^   and saves it to backup file.
cat $Savefile                     # (To see it.)

exit 0
```

```output
Aristotle
Arrhenius
Belisarius
Capablanca
Dickens
Euler
Goethe
Hegel
Jonah
```

### Example 20-10. Redirected if/then test (`redir5.sh`)

```bash no-run
#!/bin/bash

if [ -z "$1" ]
then
  Filename=names.data   # Default, if no filename specified.
else
  Filename=$1
fi  

TRUE=1

if [ "$TRUE" ]          # if true    and   if :   also work.
then
 read name
 echo $name
fi <"$Filename"
#  ^^^^^^^^^^^^

# Reads only first line of file.
# An "if/then" test has no way of iterating unless embedded in a loop.

exit 0
```

```tessh
# (names.data, Example 20-11, to read.)
printf "%s\n" Aristotle Arrhenius Belisarius Capablanca Dickens Euler Goethe \
  Hegel Jonah Laplace Maroczy Purcell Schmidt Schopenhauer Semmelweiss Smith \
  Steinmetz Tukhashevsky Turing Venn Warshawski Znosko-Borowski >names.data

Filename = if (len $argv) > 1 and $argv[1] then $argv[1] else names.data

TRUE = 1

def first_name
  if $TRUE
    name = (input)
    print $name

first_name <$Filename

# Reads only first line of file.
# An "if" test has no way of iterating unless embedded in a loop.

exit 0
```

```output
Aristotle
```

### Example 20-11. Data file names.data for above examples (`names.data`)

```text
Aristotle
Arrhenius
Belisarius
Capablanca
Dickens
Euler
Goethe
Hegel
Jonah
Laplace
Maroczy
Purcell
Schmidt
Schopenhauer
Semmelweiss
Smith
Steinmetz
Tukhashevsky
Turing
Venn
Warshawski
Znosko-Borowski

#  This is a data file for
#+ "redir2.sh", "redir3.sh", "redir4.sh", "redir4a.sh", "redir5.sh".
```

Redirecting a block's output saves it to a file, as `report >$LOGFILE`
did in Example 20-2.

A heredoc is a redirected block too, so it can feed a loop. Here the
heredoc holds what a function printed:

```bash no-run
# This example by Albert Siersema
# Used with permission (thanks!).

function doesOutput()
 # Could be an external command too, of course.
 # Here we show you can use a function as well.
{
  ls -al *.jpg | awk '{print $5,$9}'
}

nr=0          #  We want the while loop to be able to manipulate these and
totalSize=0   #+ to be able to see the changes after the 'while' finished.

while read fileSize fileName ; do
  echo "$fileName is $fileSize bytes"
  let nr++
  totalSize=$((totalSize+fileSize))   # Or: "let totalSize+=fileSize"
done<<EOF
$(doesOutput)
EOF

echo "$nr files totaling $totalSize bytes"
```

In tessh the function gives back the pipeline's output (`return (…)`
captures it; a command alone on the last line would print it), and the loop
goes over its lines. No
heredoc is needed to get at it, and `nr` and `totalSize` are there after
the loop, as in bash.

```tessh
# (Made-up pictures to try it on.)
head -c 1500 /dev/zero >a.jpg ; head -c 2048 /dev/zero >b.jpg

def doesOutput
  # Could be an external command too, of course.
  # Here we show you can use a function as well.
  return (ls -al *.jpg | awk '{print $5,$9}')

nr = 0
totalSize = 0

for line in (doesOutput).lines
  [fileSize fileName] = ($line.split)
  print "$fileName is $fileSize bytes"
  nr += 1
  totalSize += (int $fileSize)

print "$nr files totaling $totalSize bytes"
```

```output
a.jpg is 1500 bytes
b.jpg is 2048 bytes
2 files totaling 3548 bytes
```

## 20.3 Applications
<!-- help: abs-redirapps logging log-levels debug-fd -->

Redirections can split a script's messages by kind: debug messages to
descriptors that go to the screen or nowhere, depending on a level, and a
log of events to a file.

### Example 20-12. Logging events (`logevents.sh`)

```bash no-run
#!/bin/bash
# logevents.sh
# Author: Stephane Chazelas.
# Used in ABS Guide with permission.

# Event logging to a file.
# Must be run as root (for write access in /var/log).

ROOT_UID=0     # Only users with $UID 0 have root privileges.
E_NOTROOT=67   # Non-root exit error.


if [ "$UID" -ne "$ROOT_UID" ]
then
  echo "Must be root to run this script."
  exit $E_NOTROOT
fi  


FD_DEBUG1=3
FD_DEBUG2=4
FD_DEBUG3=5

# === Uncomment one of the two lines below to activate script. ===
# LOG_EVENTS=1
# LOG_VARS=1


log()  # Writes time and date to log file.
{
echo "$(date)  $*" >&7     # This *appends* the date to the file.
#     ^^^^^^^  command substitution
                           # See below.
}



case $LOG_LEVEL in
 1) exec 3>&2         4> /dev/null 5> /dev/null;;
 2) exec 3>&2         4>&2         5> /dev/null;;
 3) exec 3>&2         4>&2         5>&2;;
 *) exec 3> /dev/null 4> /dev/null 5> /dev/null;;
esac

FD_LOGVARS=6
if [[ $LOG_VARS ]]
then exec 6>> /var/log/vars.log
else exec 6> /dev/null                     # Bury output.
fi

FD_LOGEVENTS=7
if [[ $LOG_EVENTS ]]
then
  # exec 7 >(exec gawk '{print strftime(), $0}' >> /var/log/event.log)
  # Above line fails in versions of Bash more recent than 2.04. Why?
  exec 7>> /var/log/event.log              # Append to "event.log".
  log                                      # Write time and date.
else exec 7> /dev/null                     # Bury output.
fi

echo "DEBUG3: beginning" >&${FD_DEBUG3}

ls -l >&5 2>&4                             # command1 >&5 2>&4

echo "Done"                                # command2 

echo "sending mail" >&${FD_LOGEVENTS}
# Writes "sending mail" to file descriptor #7.


exit 0
```

The descriptors are set up with `os.dup2`, as bash's `exec 3>&2` and
`exec 4>/dev/null` set them; the programs run after it inherit them. The
log files are in the current folder, not `/var/log`, so it runs without
root. `>&5` is written with the number: **bug:** tessh doesn't take a
variable there (`>&$FD_DEBUG3` is passed to `echo` as a word).

```tessh
import os

ROOT_UID = (os.getuid)    # (0, root, in the original: here, you)
E_NOTROOT = 67            # Non-root exit error.

if (os.getuid) != $ROOT_UID
  print "Must be root to run this script."
  exit $E_NOTROOT

FD_DEBUG1 = 3
FD_DEBUG2 = 4
FD_DEBUG3 = 5

# === Uncomment one of the two lines below to activate script. ===
# LOG_EVENTS = 1
# LOG_VARS = 1

def exec_to fd target mode=w    # bash's exec fd>target (mode a: fd>>target)
  if isinstance $target $int
    os.dup2 $target $fd           # exec fd>&target
  else
    with (open $target $mode) as f
      os.dup2 ($f.fileno) $fd

def log *words  # Writes time and date to log file.
  echo "$(date)  $(' '.join $words)" >&7     # This *appends* the date to the file.

match (os.environ.get LOG_LEVEL)
  case "1"
    exec_to 3 2 ; exec_to 4 /dev/null ; exec_to 5 /dev/null
  case "2"
    exec_to 3 2 ; exec_to 4 2 ; exec_to 5 /dev/null
  case "3"
    exec_to 3 2 ; exec_to 4 2 ; exec_to 5 2
  case _
    exec_to 3 /dev/null ; exec_to 4 /dev/null ; exec_to 5 /dev/null

FD_LOGVARS = 6
if (globals).get "LOG_VARS"
  exec_to 6 vars.log a                     # (/var/log/vars.log)
else
  exec_to 6 /dev/null                      # Bury output.

FD_LOGEVENTS = 7
if (globals).get "LOG_EVENTS"
  exec_to 7 event.log a                    # Append to "event.log".
  log                                      # Write time and date.
else
  exec_to 7 /dev/null                      # Bury output.

echo "DEBUG3: beginning" >&5               # (>&${FD_DEBUG3} in the original)

ls -l >&5 2>&4                             # command1 >&5 2>&4

echo "Done"                                # command2

echo "sending mail" >&7                    # (>&${FD_LOGEVENTS})
# Writes "sending mail" to file descriptor #7.

exit 0
```

```output
Done
```
