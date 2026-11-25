# 5. Quoting
<!-- help: abs-quoting abs-5 quoting quotes escaping -->

Quoting protects special characters from the shell. Unquoted, `[Vv]*` is
a glob, the names that start with V or v; quoted, it's those four
characters, and `ls` looks for a file of that name.

```bash
touch VIEWDATA.BAT viewdata.sh notes.txt
ls -1 [Vv]*
ls -1 '[Vv]*'
```

```tessh
touch VIEWDATA.BAT viewdata.sh notes.txt
ls -1 [Vv]*
ls -1 '[Vv]*'
```

```output
VIEWDATA.BAT
viewdata.sh
```

(`ls` complains on standard error, which isn't shown.)

Quoting also hands a pattern to a program unexpanded, so the program can
use it: `grep '[Ff]irst' *.txt`. Unquoted, it works only until a file
called `first` turns up, in tessh as in bash.

```bash
printf "This is the first line of file1.txt.\n" > file1.txt
printf "This is the First line of file2.txt.\n" > file2.txt
grep '[Ff]irst' *.txt
```

```tessh
printf "This is the first line of file1.txt.\n" > file1.txt
printf "This is the First line of file2.txt.\n" > file2.txt
grep '[Ff]irst' *.txt
```

```output
file1.txt:This is the first line of file1.txt.
file2.txt:This is the First line of file2.txt.
```

In bash, quoting also keeps the newlines of a command's output: `echo
$(ls -l)` is one line, `echo "$(ls -l)"` several. In tessh a value is never
split, quoted or not, so both keep them; `.split` splits on purpose.

## 5.1 Quoting Variables
<!-- help: abs-quotingvar quoting-variables word-splitting double-quotes single-quotes -->

In bash a variable should nearly always be in double quotes. Unquoted,
its value is split into words at whitespace (by `$IFS`) and each word is
globbed; quoted, it's one word as it is. tessh never splits or globs a
variable's value: `$var` is one argument, whatever it holds, so the quotes
bash needs aren't needed. A list gives one argument per item, which is how
you pass several on purpose.

```bash
List="one two three"

for a in $List     # Splits the variable in parts at whitespace.
do
  echo "$a"
done
# one
# two
# three

echo "---"

for a in "$List"   # Preserves whitespace in a single variable.
do #     ^     ^
  echo "$a"
done
# one two three
```

```output bash
one
two
three
---
one two three
```

A `for` over a string goes through its characters, as Python's does; over
`.split`, its words. For a list of words, write a list.

```tessh
List = "one two three"

for a in ($List.split)   # Splits the string at whitespace.
  print $a

print "---"

for a in [$List]         # A list of one string.
  print $a

List = [one two three]   # Or make it a list to begin with.
print (len $List) $List[1]
```

```output
one
two
three
---
one two three
3 two
```

The ABS Guide counts a command's arguments with and without quotes. In
tessh `"$x"` and `$x` are the same one argument, and an empty string is an
argument too (bash drops an unquoted empty variable):

```bash
variable1="a variable containing five words"
printf "<%s>" This is $variable1 ; echo    # 7 arguments
printf "<%s>" "This is $variable1" ; echo  # 1 argument

variable2=""    # Empty.
printf "<%s>" $variable2 $variable2 $variable2 ; echo      # no arguments
printf "<%s>" "$variable2" "$variable2" "$variable2" ; echo  # 3 empty arguments
printf "<%s>" "$variable2 $variable2 $variable2" ; echo    # 1 argument (2 spaces)
```

```output bash
<This><is><a><variable><containing><five><words>
<This is a variable containing five words>
<>
<><><>
<  >
```

```tessh
variable1 = "a variable containing five words"
printf "<%s>" This 'is' $variable1 ; echo              # 3 arguments
printf "<%s>" This 'is' *(.split $variable1) ; echo     # 7 arguments
printf "<%s>" "This is $variable1" ; echo            # 1 argument

variable2 = ""    # Empty.
printf "<%s>" $variable2 $variable2 $variable2 ; echo   # 3 empty arguments
printf "<%s>" [$v for v in [$variable2] if $v] ; echo     # none: an empty list
printf "<%s>" "$variable2 $variable2 $variable2" ; echo # 1 argument (2 spaces)
```

```output
<This><is><a variable containing five words>
<This><is><a><variable><containing><five><words>
<This is a variable containing five words>
<><><>
<>
<  >
```

Bash's escapes in double quotes are a muddle: `"\a"` stays `\a`, `"\!"`
stays `\!`, and `echo -e` decides whether `\t` is a tab. tessh's double
quotes always understand the escapes (`\t`, `\n`, `\x41`, `\\`, `\"`, `\$`),
as Python's strings do, and single quotes never do. Outside quotes, a
backslash before a space or a special character takes it into the word;
before an ordinary letter it stays (`\z` is `\z`, bash's is `z`).

```bash
echo hello\!
echo "hello\!"
echo \a
echo "\a"
echo x\ty
echo "x\ty"
echo -e x\ty
echo -e "x\ty"
```

```output bash
hello!
hello\!
a
\a
xty
x\ty
xty
x	y
```

```tessh
# The tessh way to write each of bash's results:
echo hello!
echo 'hello\!'
echo a
echo '\a'
echo xty
echo 'x\ty'
echo xty
echo "x\ty"
```

```output
hello!
hello\!
a
\a
xty
x\ty
xty
x	y
```

### Example 5-1. Echoing Weird Variables (`weirdvars.sh`)

```bash no-run
#!/bin/bash
# weirdvars.sh: Echoing weird variables.

echo

var="'(]\\{}\$\""
echo $var        # '(]\{}$"
echo "$var"      # '(]\{}$"     Doesn't make a difference.

echo

IFS='\'
echo $var        # '(] {}$"     \ converted to space. Why?
echo "$var"      # '(]\{}$"

# Examples above supplied by Stephane Chazelas.

echo

var2="\\\\\""
echo $var2       #   "
echo "$var2"     # \\"
echo
# But ... var2="\\\\"" is illegal. Why?
var3='\\\\'
echo "$var3"     # \\\\
# Strong quoting works, though.


# ************************************************************ #
# As the first example above shows, nesting quotes is permitted.

echo "$(echo '"')"           # "
#    ^           ^


# At times this comes in useful.

var1="Two bits"
echo "\$var1 = "$var1""      # $var1 = Two bits
#    ^                ^

# Or, as Chris Hiestand points out ...

if [[ "$(du "$My_File1")" -gt "$(du "$My_File2")" ]]
#     ^     ^         ^ ^     ^     ^         ^ ^
then
  ...
fi
# ************************************************************ #
```

(`no-run`: the last lines are a fragment, `...` and all.) tessh has no
`$IFS`, so the `\`-turned-to-space line is a `.replace`, said outright.
Bash's `var2` comes out as ` "` unquoted because `IFS` is still `\`; in
tessh it's always `\\"`. A `$(…)` inside a double-quoted string is code, so
quotes nest in it as in bash. The closing comparison of two files' sizes
is a comparison of two numbers.

```tessh
#!/usr/bin/env tessh
import os
print

var = "'(]\\{}\$\""
print $var       # '(]\{}$"
print "$var"     # '(]\{}$"     Doesn't make a difference.

print

print ($var.replace "\\" " ")   # '(] {}$"   (bash's IFS='\' did this)
print $var                      # '(]\{}$"

print

var2 = "\\\\\""
print $var2      # \\"
print
var3 = '\\\\'
print "$var3"    # \\\\
# Strong quoting works, though.

# ************************************************************ #
# Nesting quotes is permitted.

print "$(echo '"')"          # "

var1 = "Two bits"
print "\$var1 = $var1"     # $var1 = Two bits

# Or, comparing two files' sizes (made-up files to try it on):
print aaaa >My_File1 ; print a >My_File2
if (os.path.getsize My_File1) > (os.path.getsize My_File2)
  print "My_File1 is bigger."
# ************************************************************ #
```

```output

'(]\{}$"
'(]\{}$"

'(] {}$"
'(]\{}$"

\\"

\\\\
"
$var1 = Two bits
My_File1 is bigger.
```

Single quotes keep everything as it is, a backslash included, so a single
quote can't go inside them. Bash's way around it is three strings glued
together; tessh's is to use double quotes, or a backslash in them.

```bash
echo "Why can't I write 's between single quotes"

echo

# The roundabout method.
echo 'Why can'\''t I write '"'"'s between single quotes'
#    |-------|  |----------|   |-----------------------|
# Three single-quoted strings, with escaped and quoted single quotes between.
# This example courtesy of Stéphane Chazelas.
```

```tessh
print "Why can't I write 's between single quotes"

print

# The roundabout method works too: glued strings make one word.
echo 'Why can'"'"'t I write '"'"'s between single quotes'
```

```output
Why can't I write 's between single quotes

Why can't I write 's between single quotes
```

## 5.2 Escaping
<!-- help: abs-escapingsection escaping escape backslash \n \t \x octal line-continuation -->

A backslash quotes the one character after it. To `echo -e`, `printf`,
`sed` and in bash's `$'…'`, an escaped letter instead turns a special
meaning on: `\n` newline, `\r` return, `\t` tab, `\v` vertical tab, `\b`
backspace, `\a` alert, `\0nn` an octal character code, `\xhh` a hex one.
tessh's double-quoted strings take them all, as Python's do: `"\042"` is
`"`, `"\033"` is escape, and `\x` (hex) and `\u` (Unicode) work too.

### Example 5-2. Escaped Characters (`escaped.sh`)

```bash no-run
#!/bin/bash
# escaped.sh: escaped characters

#############################################################
### First, let's show some basic escaped-character usage. ###
#############################################################

# Escaping a newline.
# ------------------

echo ""

echo "This will print
as two lines."
# This will print
# as two lines.

echo "This will print \
as one line."
# This will print as one line.

echo; echo

echo "============="


echo "\v\v\v\v"      # Prints \v\v\v\v literally.
# Use the -e option with 'echo' to print escaped characters.
echo "============="
echo "VERTICAL TABS"
echo -e "\v\v\v\v"   # Prints 4 vertical tabs.
echo "=============="

echo "QUOTATION MARK"
echo -e "\042"       # Prints " (quote, octal ASCII character 42).
echo "=============="



# The $'\X' construct makes the -e option unnecessary.

echo; echo "NEWLINE and (maybe) BEEP"
echo $'\n'           # Newline.
echo $'\a'           # Alert (beep).
                     # May only flash, not beep, depending on terminal.

# We have seen $'\nnn" string expansion, and now . . .

# =================================================================== #
# Version 2 of Bash introduced the $'\nnn' string expansion construct.
# =================================================================== #

echo "Introducing the \$\' ... \' string-expansion construct . . . "
echo ". . . featuring more quotation marks."

echo $'\t \042 \t'   # Quote (") framed by tabs.
# Note that  '\nnn' is an octal value.

# It also works with hexadecimal values, in an $'\xhhh' construct.
echo $'\t \x22 \t'  # Quote (") framed by tabs.
# Thank you, Greg Keraunen, for pointing this out.
# Earlier Bash versions allowed '\x022'.

echo


# Assigning ASCII characters to a variable.
# ----------------------------------------
quote=$'\042'        # " assigned to a variable.
echo "$quote Quoted string $quote and this lies outside the quotes."

echo

# Concatenating ASCII chars in a variable.
triple_underline=$'\137\137\137'  # 137 is octal ASCII code for '_'.
echo "$triple_underline UNDERLINE $triple_underline"

echo

ABC=$'\101\102\103\010'           # 101, 102, 103 are octal A, B, C.
echo $ABC

echo

escape=$'\033'                    # 033 is octal for escape.
echo "\"escape\" echoes as $escape"
#                                   no visible output.

echo

exit 0
```

(`no-run`: it prints control characters, a beep among them.) In tessh,
double quotes do what bash's `echo -e` and `$'…'` do, so neither is needed,
and single quotes are for the literal `\v`. The octal codes become hex
ones. Where the original prints control characters, the tessh version
sends them through `cat -v`, which shows them as `^K`, `^G` and so on.

```tessh
#!/usr/bin/env tessh
# Escaping a newline.
# ------------------

print ""

print "This will print
as two lines."

print "This will print \
as one line."

print; print

print "============="

print '\v\v\v\v'     # Prints \v\v\v\v literally: single quotes.
print "============="
print "VERTICAL TABS"
"\v\v\v\v\n" | cat -v   # 4 vertical tabs (shown by cat -v).
print "=============="

print "QUOTATION MARK"
print "\x22"         # Prints " (quote, hex ASCII character 22).
print "=============="

print; print "NEWLINE and (maybe) BEEP"
print "\n"           # Newline.
"\a\n" | cat -v      # Alert (beep), shown by cat -v.

print "Double quotes understand escapes . . . "
print ". . . featuring more quotation marks."

"\t \x22 \t\n" | cat -A   # Quote (") framed by tabs (^I to cat -A).

print

# Assigning ASCII characters to a variable.
# ----------------------------------------
quote = "\x22"       # " assigned to a variable.
print "$quote Quoted string $quote and this lies outside the quotes."

print

# Concatenating ASCII chars in a variable.
triple_underline = (chr 0o137) * 3   # 137 is octal ASCII code for '_'.
print "$triple_underline UNDERLINE $triple_underline"

print

ABC = "\x41\x42\x43\x08"             # 41, 42, 43 are hex A, B, C.
"$ABC\n" | cat -v

print

escape = "\x1b"                      # 1b is hex for escape.
"\"escape\" echoes as $escape\n" | cat -v

print

exit 0
```

```output

This will print
as two lines.
This will print as one line.


=============
\v\v\v\v
=============
VERTICAL TABS
^K^K^K^K
==============
QUOTATION MARK
"
==============

NEWLINE and (maybe) BEEP


^G
Double quotes understand escapes . . . 
. . . featuring more quotation marks.
^I " ^I$

" Quoted string " and this lies outside the quotes.

___ UNDERLINE ___

ABC^H

"escape" echoes as ^[

```

### Example 5-3. Detecting key-presses (`BashExtraKeys.sh`)

```bash no-run
#!/bin/bash
# Author: Sigurd Solaas, 20 Apr 2011
# Used in ABS Guide with permission.
# Requires version 4.2+ of Bash.

key="no value yet"
while true; do
  clear
  echo "Bash Extra Keys Demo. Keys to try:"
  echo
  echo "* Insert, Delete, Home, End, Page_Up and Page_Down"
  echo "* The four arrow keys"
  echo "* Tab, enter, escape, and space key"
  echo "* The letter and number keys, etc."
  echo
  echo "    d = show date/time"
  echo "    q = quit"
  echo "================================"
  echo

 # Convert the separate home-key to home-key_num_7:
 if [ "$key" = $'\x1b\x4f\x48' ]; then
  key=$'\x1b\x5b\x31\x7e'
  #   Quoted string-expansion construct. 
 fi

 # Convert the separate end-key to end-key_num_1.
 if [ "$key" = $'\x1b\x4f\x46' ]; then
  key=$'\x1b\x5b\x34\x7e'
 fi

 case "$key" in
  $'\x1b\x5b\x32\x7e')  # Insert
   echo Insert Key
  ;;
  $'\x1b\x5b\x33\x7e')  # Delete
   echo Delete Key
  ;;
  $'\x1b\x5b\x31\x7e')  # Home_key_num_7
   echo Home Key
  ;;
  $'\x1b\x5b\x34\x7e')  # End_key_num_1
   echo End Key
  ;;
  $'\x1b\x5b\x35\x7e')  # Page_Up
   echo Page_Up
  ;;
  $'\x1b\x5b\x36\x7e')  # Page_Down
   echo Page_Down
  ;;
  $'\x1b\x5b\x41')  # Up_arrow
   echo Up arrow
  ;;
  $'\x1b\x5b\x42')  # Down_arrow
   echo Down arrow
  ;;
  $'\x1b\x5b\x43')  # Right_arrow
   echo Right arrow
  ;;
  $'\x1b\x5b\x44')  # Left_arrow
   echo Left arrow
  ;;
  $'\x09')  # Tab
   echo Tab Key
  ;;
  $'\x0a')  # Enter
   echo Enter Key
  ;;
  $'\x1b')  # Escape
   echo Escape Key
  ;;
  $'\x20')  # Space
   echo Space Key
  ;;
  d)
   date
  ;;
  q)
  echo Time to quit...
  echo
  exit 0
  ;;
  *)
   echo You pressed: \'"$key"\'
  ;;
 esac

 echo
 echo "================================"

 unset K1 K2 K3
 read -s -N1 -p "Press a key: "
 K1="$REPLY"
 read -s -N2 -t 0.001
 K2="$REPLY"
 read -s -N1 -t 0.001
 K3="$REPLY"
 key="$K1$K2$K3"

done

exit $?
```

The key names are a dict from the escape sequences, which takes the place
of the long `case`. Reading a key without Enter is Python's `tty` and
`termios`: `read_key` puts the terminal in raw mode, reads one character
and then whatever follows it at once (the rest of an escape sequence). It
needs a terminal, so here a canned list of keys stands in for it, and the
menu is shown once rather than redrawn for each key.

```tessh
#!/usr/bin/env tessh
import sys os select termios tty

def read_key
  # One key press, as the characters it sends.
  fd = (sys.stdin.fileno)
  saved = termios.tcgetattr $fd
  try
    tty.setraw $fd
    key = os.read $fd 1
    while (select.select [$fd] [] [] 0.001)[0]   # The rest of a sequence.
      key += os.read $fd 1
  finally
    termios.tcsetattr $fd $termios.TCSADRAIN $saved
  $key.decode

NAMES = {
  "\x1b[2~": "Insert Key"   "\x1b[3~": "Delete Key"
  "\x1b[1~": "Home Key"     "\x1b[4~": "End Key"
  "\x1b[5~": "Page_Up"      "\x1b[6~": "Page_Down"
  "\x1b[A": "Up arrow"      "\x1b[B": "Down arrow"
  "\x1b[C": "Right arrow"   "\x1b[D": "Left arrow"
  "\t": "Tab Key"  "\r": "Enter Key"  "\x1b": "Escape Key"  " ": "Space Key"
}
# The separate Home and End keys, as their keypad twins.
SAME = {"\x1bOH": "\x1b[1~"  "\x1bOF": "\x1b[4~"}

def show_menu
  print "Bash Extra Keys Demo. Keys to try:"
  print
  print "* Insert, Delete, Home, End, Page_Up and Page_Down"
  print "* The four arrow keys"
  print "* Tab, enter, escape, and space key"
  print "* The letter and number keys, etc."
  print
  print "    d = show date/time"
  print "    q = quit"
  print "================================"
  print

def main next_key
  show_menu
  while True
    print "Press a key: "
    key = (next_key)
    key = $SAME.get $key $key
    match $key
      case d
        date
      case q
        print "Time to quit..."
        print
        return
      case _ if $key in $NAMES
        print $NAMES[$key]
      case _
        print "You pressed: '$key'"
    print
    print "================================"

# (Canned key presses: on a terminal, main $read_key.)
keys = (iter ["\x1b[2~" "\x1bOH" "\x1b[A" "\t" "x" " " "q"])
main (fn (next $keys))
```

```output
Bash Extra Keys Demo. Keys to try:

* Insert, Delete, Home, End, Page_Up and Page_Down
* The four arrow keys
* Tab, enter, escape, and space key
* The letter and number keys, etc.

    d = show date/time
    q = quit
================================

Press a key: 
Insert Key

================================
Press a key: 
Home Key

================================
Press a key: 
Up arrow

================================
Press a key: 
Tab Key

================================
Press a key: 
You pressed: 'x'

================================
Press a key: 
Space Key

================================
Press a key: 
Time to quit...

```

The other escapes the ABS Guide lists: `\"` and `\$` give a literal quote
and dollar sign inside double quotes, and `\\` a backslash, in both
shells. Bash's `echo "\"` waits for more input; in tessh it's an
unterminated string, an error `tessh --check` finds.

```bash
echo "Hello"                     # Hello
echo "\"Hello\" ... he said."    # "Hello" ... he said.
echo "\$variable01"           # $variable01
echo "The book cost \$7.98."  # The book cost $7.98.
echo "\\"  # Results in \
echo '\'   # Results in \
```

```tessh
print "Hello"                     # Hello
print "\"Hello\" ... he said."    # "Hello" ... he said.
print "\$variable01"              # $variable01
print "The book cost \$7.98."     # The book cost $7.98.
print "\\"                        # Results in \
print '\'                         # Results in \
```

```output
Hello
"Hello" ... he said.
$variable01
The book cost $7.98.
\
\
```

How many backslashes survive depends, in bash, on the quotes, on backquotes
and on here documents. Stéphane Chazelas's table, with tessh's answers
where tessh has the construct (it has no backquotes):

```bash
echo \z               #  z
echo \\z              # \z
echo '\z'             # \z
echo '\\z'            # \\z
echo "\z"             # \z
echo "\\z"            # \z
cat <<EOF
\z
EOF
cat <<EOF
\\z
EOF
```

```output bash
z
\z
\z
\\z
\z
\z
\z
\z
```

```tessh
echo \z               # \z   (bash: z)
echo '\z'             # \z
echo '\\z'            # \\z
echo "\z"             # \z
echo "\\z"            # \z
cat <<EOF
\z
EOF
cat <<EOF
\\z
EOF
```

```output
\z
\z
\\z
\z
\z
\z
\z
```

(Bare `\\z` is left out of the tessh version: tessh keeps it as `\\z`, and
a `\\` on its own reads as the program escape `\name`. Quote a backslash
to be sure of it.)

A variable can hold a lone backslash. In bash it has to be escaped
(`variable=\\`); in tessh it's a string, `variable = "\\"` or
`variable = '\'`.

```bash
variable=\
23skidoo
echo "$variable"        #  23skidoo
variable=\\
echo "$variable"        # \
variable=\\\\
echo "$variable"        # \\
```

```tessh
variable = \
  "23skidoo"
print $variable         # 23skidoo
variable = '\'
print $variable         # \
variable = '\\'
print $variable         # \\
```

```output
23skidoo
\
\\
```

Escaping a space joins two words into one argument, in both: `ls -l
/sbin/dump\ $file_list` makes `/sbin/dump /bin/cat` one name. In tessh,
since a variable isn't split, the way to give a command a list of files is
a list.

```bash no-run
file_list="/bin/cat /bin/gzip /bin/more /usr/bin/less /usr/bin/emacs-20.7"
# List of files as argument(s) to a command.

# Add two files to the list, and list all.
ls -l /usr/X11R6/bin/xsetroot /sbin/dump $file_list
```

```tessh no-run
file_list = [/bin/cat /bin/gzip /bin/more /usr/bin/less /usr/bin/emacs-20.7]
# List of files as argument(s) to a command.

# Add two files to the list, and list all.
ls -l /usr/X11R6/bin/xsetroot /sbin/dump $file_list
```

A backslash at the end of a line carries a command on to the next. tessh
does that too (with a space before it), and a line that ends in `|` or an
open bracket goes on by itself. In a double-quoted string, a backslash
before the newline leaves the newline out, in both; in single quotes it
stays.

```bash
echo "foo
bar"
#foo
#bar

echo

echo 'foo
bar'    # No difference yet.
#foo
#bar

echo

echo foo \
bar     # Newline escaped.
#foo bar

echo

echo "foo\
bar"     # Same here, as \ still interpreted as escape within weak quotes.
#foobar

echo

echo 'foo\
bar'     # Escape character \ taken literally because of strong quoting.
#foo\
#bar

# Examples suggested by Stéphane Chazelas.
```

```tessh
echo "foo
bar"

echo

echo 'foo
bar'    # No difference yet.

echo

echo foo \
bar     # Newline escaped.

echo

echo "foo\
bar"     # \ is an escape in double quotes.

echo

echo 'foo\
bar'     # Taken literally in single quotes.
```

```output
foo
bar

foo
bar

foo bar

foobar

foo\
bar
```

(The ABS Guide's `echo foo\` with no space before the backslash glues the
next line on, `foobar`; tessh doesn't continue that line for now, a bug.)
