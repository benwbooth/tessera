# 12. Command Substitution
<!-- help: abs-commandsub abs-12 command-substitution backticks $(…) captured-output -->

Command substitution, `` `command` `` or `$(command)`, puts a command's
output in place of the command: into a variable, an argument, a `for`
list. In tessh a command used as a value is already that: `x = date +%F`,
or `(date)` in the middle of a line (a lone word, `x = date`, is just the
text `date`), gives its output as text (an `Output`,
a `str` that also has the exit status). Backticks are tessh's infix
operators, so only the `(…)` form exists. And `"$(…)"` in a string
interpolates any value, a command included.

The output is one string: a command's output is never split into words
or globbed. `.split` or `.lines` makes a list of it when you want one.

```bash
script_name=`basename $0`
echo "The name of this script is $script_name."
```

```output bash
The name of this script is ex.sh.
```

```tessh
script_name = basename $argv[0]          # (Path $argv[0]).name says it in Python.
print "The name of this script is $script_name."
```

```output
The name of this script is ex.tsh.
```

The output can be the arguments of another command, or a list for a loop.
Bash splits it at whitespace (and globs it); tessh splits only when told,
at lines with `.lines` (names with spaces in them survive) or at
whitespace with `.split`. For files, a Glob is better still than `ls`.

```bash no-run
rm `cat filename`   # filename contains a list of files to delete.
#
# S. C. points out that "arg list too long" error might result.
# Better is              xargs rm -- < filename
# ( -- covers those cases where filename begins with a - )

textfile_listing=`ls *.txt`
# Variable contains names of all *.txt files in current working directory.
echo $textfile_listing

textfile_listing2=$(ls *.txt)   # The alternative form of command substitution.
echo $textfile_listing2
# Same result.

# A possible problem with putting a list of files into a single string
# is that a newline may creep in.
#
# A safer way to assign a list of files to a parameter is with an array.
#      shopt -s nullglob    # If no match, filename expands to nothing.
#      textfile_listing=( *.txt )
#
# Thanks, S.C.
```

```tessh
# (Made-up files.)
touch a.txt "b c.txt" old1 old2
printf "old1\nold2\n" >filename

rm (cat filename).lines     # One name per line in filename; a list of them.

textfile_listing = ls *.txt
print $textfile_listing     # Lines kept as they were.

textfile_listing = *.txt    # Better: a Glob, the names themselves.
print (list $textfile_listing)
ls
```

```output
a.txt
b c.txt
[(Path 'a.txt') (Path 'b c.txt')]
a.txt
b c.txt
filename
```

Bash splits an unquoted substitution into words, and drops an empty one;
quoted, it's one argument. In tessh it's always one argument:

```bash
printf "<%s>" `echo a b` ; echo      # 2 args: a and b
printf "<%s>" "`echo a b`" ; echo    # 1 arg: "a b"
printf "<%s>" `echo` ; echo          # no arg
printf "<%s>" "`echo`" ; echo        # one empty arg
```

```output bash
<a><b>
<a b>
<>
<>
```

```tessh
printf "<%s>" ((echo a b).split) ; echo   # 2 args: a and b
printf "<%s>" (echo a b) ; echo           # 1 arg: "a b"
printf "<%s>" ((echo).split) ; echo       # no arg
printf "<%s>" (echo) ; echo               # one empty arg
```

```output
<a><b>
<a b>
<>
<>
```

(`printf` with no argument at all prints its format once, so the third
line looks like the fourth in both.)

Command substitution drops the newlines at the end of the output. Bash
drops all of them, tessh only the last one, the newline that ends the
output's last line. tessh's `pwd` is its own, and gives the folder as it
is, so the ABS Guide's trap of a folder whose name ends with a newline
doesn't happen:

```bash no-run
# cd "`pwd`"  # This should always work.
# However...

mkdir 'dir with trailing newline
'

cd 'dir with trailing newline
'

cd "`pwd`"  # Error message:
# bash: cd: /tmp/file with trailing newline: No such file or directory

cd "$PWD"   # Works fine.
```

```tessh
mkdir "dir with trailing newline\n"
cd "dir with trailing newline\n"

cd (pwd)                   # Works: (pwd) keeps the name whole.
print ((pwd).endswith "newline\n")

x = printf "a\n\n\n"
print (repr $x)            # Only the last newline goes.
```

```output
True
'a\n\n'
```

The ABS Guide's `stty`/`dd` snippet, reading one key, shows the other side
of this: a key that is Return comes out empty. It needs a terminal; in
tessh, `sys.stdin.read 1` after `tty.setcbreak` reads a key, newline
and all.

`echo $listing` unquoted runs the lines of `ls -l` together, since bash
splits the value into words and `echo` joins them with spaces. In tessh a
value is never split, so `print $listing` shows the lines as they were.

```bash no-run
dir_listing=`ls -l`
echo $dir_listing     # unquoted

# Expecting a nicely ordered directory listing.

# However, what you get is:
# total 3 -rw-rw-r-- 1 bozo bozo 30 May 13 17:15 1.txt -rw-rw-r-- 1 bozo
# bozo 51 May 15 20:57 t2.sh -rwxr-xr-x 1 bozo bozo 217 Mar 5 21:13 wi.sh

# The newlines disappeared.

echo "$dir_listing"   # quoted
# -rw-rw-r--    1 bozo       30 May 13 17:15 1.txt
# -rw-rw-r--    1 bozo       51 May 15 20:57 t2.sh
# -rwxr-xr-x    1 bozo      217 Mar  5 21:13 wi.sh
```

```tessh
echo "some text" >1.txt ; echo "echo hi" >t2.sh       # (Made-up files.)

dir_listing = ls -1        # (ls -l's dates would change from run to run.)
print $dir_listing         # The newlines are still there.
```

```output
1.txt
t2.sh
```

A variable can hold a file's contents. Bash's `` `<file1` `` reads it
without starting `cat`; in tessh that's Python's file reading, and the
contents can be bytes as well as text.

```bash no-run
variable1=`<file1`      #  Set "variable1" to contents of "file1".
variable2=`cat file2`   #  Set "variable2" to contents of "file2".
                        #  This, however, forks a new process,
                        #+ so the line of code executes slower than the above version.

#  Note that the variables may contain embedded whitespace,
#+ or even (horrors), control characters.

#  It is not necessary to explicitly assign a variable.
echo "` <$0`"           # Echoes the script itself to stdout.
```

```tessh
echo "contents of file1" >file1 ; echo "contents of file2" >file2   # (Made-up files.)

variable1 = ((Path file1).read_text) # Set "variable1" to contents of "file1".
variable2 = cat file2                # Set "variable2" to contents of "file2".
print $variable1 end=""
print $variable2

print ((Path $argv[0]).read_text) end=""   # Prints the script itself.
```

```output
contents of file1
contents of file2
echo "contents of file1" >file1 ; echo "contents of file2" >file2   # (Made-up files.)

variable1 = ((Path file1).read_text) # Set "variable1" to contents of "file1".
variable2 = cat file2                # Set "variable2" to contents of "file2".
print $variable1 end=""
print $variable2

print ((Path $argv[0]).read_text) end=""   # Prints the script itself.
```

Red Hat's old `/etc/rc.d/rc.sysinit` did this everywhere:
`` fsckoptions=`cat /fsckoptions` ``, `` ktag="`cat /proc/version`" ``.
Not, though, with a long file, or a binary one:

### Example 12-1. Stupid script tricks (`stupid-script-tricks.sh`)

```bash
#!/bin/bash
# stupid-script-tricks.sh: Don't try this at home, folks.
# From "Stupid Script Tricks," Volume I.

exit 99  ### Comment out this line if you dare.

dangerous_variable=`cat /boot/vmlinuz`   # The compressed Linux kernel itself.

echo "string-length of \$dangerous_variable = ${#dangerous_variable}"
# string-length of $dangerous_variable = 794151
# (Newer kernels are bigger.)
# Does not give same count as 'wc -c /boot/vmlinuz'.

# echo "$dangerous_variable"
# Don't try this! It would hang the script.


#  The document author is aware of no useful applications for
#+ setting a variable to the contents of a binary file.

exit 0
```

In tessh the trick is harmless: a binary file read as `bytes` keeps every
byte, so its length is `wc -c`'s. (A command's captured output is text,
so `cat` of a binary file would be decoded, and is the wrong tool.)

```tessh
exit 99  ### Comment out this line if you dare.

dangerous_variable = (Path /boot/vmlinuz).read_bytes   # The kernel, as bytes.

print "string-length of dangerous_variable = $(len $dangerous_variable)"
# The same count as 'wc -c /boot/vmlinuz'.
```

```output
```

The output of a loop can go into a variable too. In bash the loop
`echo`s and the substitution catches it; in tessh the loop makes the
values and `join` puts them together.

### Example 12-2. Generating a variable from a loop (`csubloop.sh`)

```bash
#!/bin/bash
# csubloop.sh: Setting a variable to the output of a loop.

variable1=`for i in 1 2 3 4 5
do
  echo -n "$i"                 #  The 'echo' command is critical
done`                          #+ to command substitution here.

echo "variable1 = $variable1"  # variable1 = 12345


i=0
variable2=`while [ "$i" -lt 10 ]
do
  echo -n "$i"                 # Again, the necessary 'echo'.
  let "i += 1"                 # Increment.
done`

echo "variable2 = $variable2"  # variable2 = 0123456789

#  Demonstrates that it's possible to embed a loop
#+ inside a variable declaration.

exit 0
```

```tessh
variable1 = "".join [(str $i) for i in [1 2 3 4 5]]
print "variable1 = $variable1"  # variable1 = 12345

def digits                      # A while loop, as a generator.
  i = 0
  while $i < 10
    yield (str $i)
    i += 1

variable2 = "".join (digits)
print "variable2 = $variable2"  # variable2 = 0123456789
```

```output
variable1 = 12345
variable2 = 0123456789
```

**Gap:** what a tessh block prints can't be captured as a value the way
bash's `` `for …` `` does; build the value instead, or redirect a
function to a file.

Any program that writes to its standard output extends the shell this
way, a compiled one included:

```bash no-run
#!/bin/bash
# hello.sh

greeting=`./hello`
echo $greeting
```

```tessh no-run
greeting = ./hello     # (A C program that prints "Hello, world.")
print $greeting
```

`$(…)` nests, where backticks need escaping:

```bash no-run
word_count=` wc -w \`echo * | awk '{print $8}'\` `
word_count=$( wc -w $(echo * | awk '{print $8}') )
```

```tessh
echo one two >a ; echo three >b       # (Made-up files.)
word_count = wc -w (echo * | awk '{print $1}')    # (The first file, where the ABS Guide takes the 8th.)
print $word_count
```

```output
2 a
```

### Example 12-3. Finding anagrams (`agram2.sh`)

```bash no-run
#!/bin/bash
# agram2.sh
# Example of nested command substitution.

#  Uses "anagram" utility
#+ that is part of the author's "yawl" word list package.
#  http://ibiblio.org/pub/Linux/libs/yawl-0.3.2.tar.gz
#  http://bash.deta.in/yawl-0.3.2.tar.gz

E_NOARGS=86
E_BADARG=87
MINLEN=7

if [ -z "$1" ]
then
  echo "Usage $0 LETTERSET"
  exit $E_NOARGS         # Script needs a command-line argument.
elif [ ${#1} -lt $MINLEN ]
then
  echo "Argument must have at least $MINLEN letters."
  exit $E_BADARG
fi



FILTER='.......'         # Must have at least 7 letters.
#       1234567
Anagrams=( $(echo $(anagram $1 | grep $FILTER) ) )
#          $(     $(  nested command sub.    ) )
#        (              array assignment         )

echo
echo "${#Anagrams[*]}  7+ letter anagrams found"
echo
echo ${Anagrams[0]}      # First anagram.
echo ${Anagrams[1]}      # Second anagram.
                         # Etc.

# echo "${Anagrams[*]}"  # To list all the anagrams in a single line . . .

#  Look ahead to the Arrays chapter for enlightenment on
#+ what's going on here.

# See also the agram.sh script for an exercise in anagram finding.

exit $?
```

The `anagram` program (from the author's yawl package) isn't here, so a
function stands in for it: the words of a word list that can be spelled
with the given letters, printed one per line as `anagram` does. The
nesting is the same; `.lines` makes the list that bash's `( … )` makes.

```tessh
from collections import Counter

# (A made-up word list.)
printf "%s\n" teacher cheater hectare reteach rachet each tea >words

@command
def anagram letters
  # Stand-in for yawl's anagram: words spelled from the letters given.
  have = Counter $letters
  for word in ((cat words).lines)
    if (Counter $word) <= $have
      print $word

E_NOARGS = 86
E_BADARG = 87
MINLEN = 7

args = $argv[1:] or [hectare]   # (a sample argument, when none is given)

if not $args or not $args[0]
  print "Usage $argv[0] LETTERSET"
  exit $E_NOARGS                # Script needs a command-line argument.
elif (len $args[0]) < $MINLEN
  print "Argument must have at least $MINLEN letters."
  exit $E_BADARG

FILTER = '.......'              # Must have at least 7 letters.
anagram $args[0] >found         # (A function can't be piped yet: see chapter 11.)
Anagrams = (grep $FILTER found).lines

print
print "$(len $Anagrams)  7+ letter anagrams found"
print
print $Anagrams[0]              # First anagram.
print $Anagrams[1]              # Second anagram.
```

```output

4  7+ letter anagrams found

teacher
cheater
```
