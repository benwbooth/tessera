# 19. Here Documents
<!-- help: abs-here-docs abs-19 here-document heredoc limit-string << -->

A here document is a block of lines given to a command as its standard
input: `COMMAND <<LimitString`, then the lines, then a line that is just
the limit string. tessh reads them as bash does: `$name` and `$(…)` in the
lines are filled in, `<<'END'` (or `<<"END"`) keeps them as written, and
`<<-END` takes the tabs off the front of each line. What's filled in is
tessh's own: `$name` is a Python variable (or the environment's), `$(…)`
any form, `${…}` any expression.

A heredoc can feed any program, even one that isn't usually scripted, such
as `wall`.

### Example 19-1. broadcast: Sends message to everyone logged in (`ex70.sh`)

```bash no-run
#!/bin/bash

wall <<zzz23EndOfMessagezzz23
E-mail your noontime orders for pizza to the system administrator.
    (Add an extra dollar for anchovy or mushroom topping.)
# Additional message text goes here.
# Note: 'wall' prints comment lines.
zzz23EndOfMessagezzz23

# Could have been done more efficiently by
#         wall <message-file
#  However, embedding the message template in a script
#+ is a quick-and-dirty one-off solution.

exit
```

The same heredoc in tessh, with `cat` standing in for `wall` so that the
message doesn't go to every terminal on the machine. A bare `exit` exits 0.

```tessh
#!/usr/bin/env tessh
cat <<zzz23EndOfMessagezzz23     # (wall in the original)
E-mail your noontime orders for pizza to the system administrator.
    (Add an extra dollar for anchovy or mushroom topping.)
# Additional message text goes here.
# Note: 'wall' prints comment lines.
zzz23EndOfMessagezzz23

#  Could have been done more efficiently by
#         wall <message-file

exit
```

```output
E-mail your noontime orders for pizza to the system administrator.
    (Add an extra dollar for anchovy or mushroom topping.)
# Additional message text goes here.
# Note: 'wall' prints comment lines.
```

### Example 19-2. dummyfile: Creates a 2-line dummy file (`ex69.sh`)

```bash no-run
#!/bin/bash

# Noninteractive use of 'vi' to edit a file.
# Emulates 'sed'.

E_BADARGS=85

if [ -z "$1" ]
then
  echo "Usage: `basename $0` filename"
  exit $E_BADARGS
fi

TARGETFILE=$1

# Insert 2 lines in file, then save.
#--------Begin here document-----------#
vi $TARGETFILE <<x23LimitStringx23
i
This is line 1 of the example file.
This is line 2 of the example file.
^[
ZZ
x23LimitStringx23
#----------End here document-----------#

#  Note that ^[ above is a literal escape
#+ typed by Control-V <Esc>.

#  Bram Moolenaar points out that this may not work with 'vim'
#+ because of possible problems with terminal interaction.

exit
```

Driving `vi` this way needs a terminal, so neither version runs here. The
literal escape (`^[`) is `\x1b` in a tessh string; in a heredoc, which is
taken as written, it has to be typed as the character itself, as in bash.

```tessh no-run
#!/usr/bin/env tessh
E_BADARGS = 85

if (len $argv) < 2 or not $argv[1]
  print "Usage: $(Path $argv[0]).name filename"
  exit $E_BADARGS

TARGETFILE = $argv[1]

# Insert 2 lines in file, then save.
#--------Begin here document-----------#
vi $TARGETFILE <<x23LimitStringx23
i
This is line 1 of the example file.
This is line 2 of the example file.
^[
ZZ
x23LimitStringx23
#----------End here document-----------#

#  Note that ^[ above is a literal escape
#+ typed by Control-V <Esc>.
```

`ex`, `vi`'s line mode, takes a heredoc of commands without a terminal: an
"ex script". This one replaces "Smith" with "Jones" in every `.txt` file
that has it. (Vim's `ex` wants `-s` to run quietly from a script.)

```bash no-run
#!/bin/bash
#  Replace all instances of "Smith" with "Jones"
#+ in files with a ".txt" filename suffix. 

ORIGINAL=Smith
REPLACEMENT=Jones

for word in $(fgrep -l $ORIGINAL *.txt)
do
  # -------------------------------------
  ex $word <<EOF
  :%s/$ORIGINAL/$REPLACEMENT/g
  :wq
EOF
  # :%s is the "ex" substitution command.
  # :wq is write-and-quit.
  # -------------------------------------
done
```

```tessh
#!/usr/bin/env tessh
# (Made-up files to try it on.)
print "Mr. Smith wrote to Ms. Smith." >a.txt
print "Nobody here." >b.txt
print "Smith & Sons, est. 1890" >c.txt

ORIGINAL = Smith
REPLACEMENT = Jones

for word in (grep -lF $ORIGINAL *.txt).lines
  # -------------------------------------
  ex -s $word <<EOF
  :%s/$ORIGINAL/$REPLACEMENT/g
  :wq
EOF
  # :%s is the "ex" substitution command.
  # :wq is write-and-quit.
  # -------------------------------------
cat *.txt
```

```output
Mr. Jones wrote to Ms. Jones.
Nobody here.
Jones & Sons, est. 1890
```

The ex script only changes files, so it prints nothing; the `cat` at the
end shows what it did. (`grep -F` is what `fgrep` is now called.)

A `cat` script is the same thing for a block of text to print.

### Example 19-3. Multi-line message using cat (`ex71.sh`)

```bash
#!/bin/bash

#  'echo' is fine for printing single line messages,
#+  but somewhat problematic for for message blocks.
#   A 'cat' here document overcomes this limitation.

cat <<End-of-message
-------------------------------------
This is line 1 of the message.
This is line 2 of the message.
This is line 3 of the message.
This is line 4 of the message.
This is the last line of the message.
-------------------------------------
End-of-message

#  Replacing line 7, above, with
#+   cat > $Newfile <<End-of-message
#+       ^^^^^^^^^^
#+ writes the output to the file $Newfile, rather than to stdout.

exit 0


#--------------------------------------------
# Code below disabled, due to "exit 0" above.

# S.C. points out that the following also works.
echo "-------------------------------------
This is line 1 of the message.
This is line 2 of the message.
This is line 3 of the message.
This is line 4 of the message.
This is the last line of the message.
-------------------------------------"
# However, text may not include double quotes unless they are escaped.
```

Below `exit` the original has the other way, a multi-line string. In tessh
that's a triple-quoted string, which can hold `"` as it is. Code after
`exit` must still be correct tessh, since the whole file is compiled first.

```tessh
#!/usr/bin/env tessh
cat <<End-of-message
-------------------------------------
This is line 1 of the message.
This is line 2 of the message.
This is line 3 of the message.
This is line 4 of the message.
This is the last line of the message.
-------------------------------------
End-of-message

#  Replacing the cat line above with
#+   cat > $Newfile <<End-of-message
#+ writes the output to the file $Newfile, rather than to stdout.

exit 0


#--------------------------------------------
# Code below disabled, due to "exit 0" above.

print """-------------------------------------
This is line 1 of the message.
This is line 2 of the message.
This is line 3 of the message.
This is line 4 of the message.
This is the last line of the message.
-------------------------------------"""
```

```output
-------------------------------------
This is line 1 of the message.
This is line 2 of the message.
This is line 3 of the message.
This is line 4 of the message.
This is the last line of the message.
-------------------------------------
```

### Example 19-4. Multi-line message, with tabs suppressed (`ex71a.sh`)

```bash
#!/bin/bash
# Same as previous example, but...

#  The - option to a here document <<-
#+ suppresses leading tabs in the body of the document,
#+ but *not* spaces.

cat <<-ENDOFMESSAGE
	This is line 1 of the message.
	This is line 2 of the message.
	This is line 3 of the message.
	This is line 4 of the message.
	This is the last line of the message.
ENDOFMESSAGE
# The output of the script will be flush left.
# Leading tab in each line will not show.

# Above 5 lines of "message" prefaced by a tab, not spaces.
# Spaces not affected by   <<-  .

# Note that this option has no effect on *embedded* tabs.

exit 0
```

tessh's `<<-` takes off leading tabs, not spaces, as bash's does. The
lines of a heredoc are text, so the tabs in them are allowed even though
tessh's own indentation must be spaces.

```tessh
#!/usr/bin/env tessh
#  The - option to a here document <<-
#+ suppresses leading tabs in the body of the document,
#+ but *not* spaces.

cat <<-ENDOFMESSAGE
	This is line 1 of the message.
	This is line 2 of the message.
	This is line 3 of the message.
	This is line 4 of the message.
	This is the last line of the message.
ENDOFMESSAGE
# The output of the script will be flush left.
# Leading tab in each line will not show.

exit 0
```

```output
This is line 1 of the message.
This is line 2 of the message.
This is line 3 of the message.
This is line 4 of the message.
This is the last line of the message.
```

Since the heredoc is filled in when it runs, different values make different
text.

### Example 19-5. Here document with replaceable parameters (`ex71b.sh`)

```bash
#!/bin/bash
# Another 'cat' here document, using parameter substitution.

# Try it with no command-line parameters,   ./scriptname
# Try it with one command-line parameter,   ./scriptname Mortimer
# Try it with one two-word quoted command-line parameter,
#                           ./scriptname "Mortimer Jones"

CMDLINEPARAM=1     #  Expect at least command-line parameter.

if [ $# -ge $CMDLINEPARAM ]
then
  NAME=$1          #  If more than one command-line param,
                   #+ then just take the first.
else
  NAME="John Doe"  #  Default, if no command-line parameter.
fi  

RESPONDENT="the author of this fine script"  
  

cat <<Endofmessage

Hello, there, $NAME.
Greetings to you, $NAME, from $RESPONDENT.

# This comment shows up in the output (why?).

Endofmessage

# Note that the blank lines show up in the output.
# So does the comment.

exit
```

`$NAME` and `$RESPONDENT` are the script's variables, and the heredoc's
lines, comment and blank lines included, are text.

```tessh
#!/usr/bin/env tessh
# Try it with no command-line parameters,   tessh scriptname
# Try it with one command-line parameter,   tessh scriptname Mortimer
# Try it with one two-word quoted command-line parameter,
#                           tessh scriptname "Mortimer Jones"

CMDLINEPARAM = 1     #  Expect at least one command-line parameter.

if (len $argv) - 1 >= $CMDLINEPARAM
  NAME = $argv[1]    #  If more than one command-line param,
                     #+ then just take the first.
else
  NAME = "John Doe"  #  Default, if no command-line parameter.

RESPONDENT = "the author of this fine script"

cat <<Endofmessage

Hello, there, $NAME.
Greetings to you, $NAME, from $RESPONDENT.

# This comment shows up in the output (why?).

Endofmessage

# Note that the blank lines show up in the output.
# So does the comment.

exit
```

```output

Hello, there, John Doe.
Greetings to you, John Doe, from the author of this fine script.

# This comment shows up in the output (why?).

```

### Example 19-6. Upload a file pair to Sunsite incoming directory (`ex72.sh`)

```bash no-run
#!/bin/bash
# upload.sh

#  Upload file pair (Filename.lsm, Filename.tar.gz)
#+ to incoming directory at Sunsite/UNC (ibiblio.org).
#  Filename.tar.gz is the tarball itself.
#  Filename.lsm is the descriptor file.
#  Sunsite requires "lsm" file, otherwise will bounce contributions.


E_ARGERROR=85

if [ -z "$1" ]
then
  echo "Usage: `basename $0` Filename-to-upload"
  exit $E_ARGERROR
fi  


Filename=`basename $1`           # Strips pathname out of file name.

Server="ibiblio.org"
Directory="/incoming/Linux"
#  These need not be hard-coded into script,
#+ but may instead be changed to command-line argument.

Password="your.e-mail.address"   # Change above to suit.

ftp -n $Server <<End-Of-Session
# -n option disables auto-logon

user anonymous "$Password"       #  If this doesn't work, then try:
                                 #  quote user anonymous "$Password"
binary
bell                             # Ring 'bell' after each file transfer.
cd $Directory
put "$Filename.lsm"
put "$Filename.tar.gz"
bye
End-Of-Session

exit 0
```

This one needs the network (and an `ftp` program), so it doesn't run here.
Note `${Filename}.lsm`: in tessh `$Filename.lsm` would be the `lsm`
attribute of `$Filename`, in a heredoc as in a string. (Python's `ftplib`
does the same job without a heredoc.)

```tessh no-run
#!/usr/bin/env tessh
E_ARGERROR = 85

if (len $argv) < 2 or not $argv[1]
  print "Usage: $(Path $argv[0]).name Filename-to-upload"
  exit $E_ARGERROR

Filename = (Path $argv[1]).name    # Strips pathname out of file name.

Server = "ibiblio.org"
Directory = "/incoming/Linux"

Password = "your.e-mail.address"   # Change above to suit.

ftp -n $Server <<End-Of-Session
# -n option disables auto-logon

user anonymous "$Password"       #  If this doesn't work, then try:
                                 #  quote user anonymous "$Password"
binary
bell                             # Ring 'bell' after each file transfer.
cd $Directory
put "${Filename}.lsm"
put "${Filename}.tar.gz"
bye
End-Of-Session

exit 0
```

Quoting the limit string turns the filling-in off: the lines come out as
they are. Bash's `$`, `` ` `` and `\` are then plain characters, and so are
tessh's `$name`, `$(…)` and `${…}`.

### Example 19-7. Parameter substitution turned off (`ex71c.sh`)

```bash
#!/bin/bash
#  A 'cat' here-document, but with parameter substitution disabled.

NAME="John Doe"
RESPONDENT="the author of this fine script"  

cat <<'Endofmessage'

Hello, there, $NAME.
Greetings to you, $NAME, from $RESPONDENT.

Endofmessage

#   No parameter substitution when the "limit string" is quoted or escaped.
#   Either of the following at the head of the here document would have
#+  the same effect.
#   cat <<"Endofmessage"
#   cat <<\Endofmessage



#   And, likewise:

cat <<"SpecialCharTest"

Directory listing would follow
if limit string were not quoted.
`ls -l`

Arithmetic expansion would take place
if limit string were not quoted.
$((5 + 3))

A a single backslash would echo
if limit string were not quoted.
\\

SpecialCharTest


exit
```

```output bash

Hello, there, $NAME.
Greetings to you, $NAME, from $RESPONDENT.


Directory listing would follow
if limit string were not quoted.
`ls -l`

Arithmetic expansion would take place
if limit string were not quoted.
$((5 + 3))

A a single backslash would echo
if limit string were not quoted.
\\

```

The second heredoc is written with tessh's substitutions, `$(ls -l)` and
`$(5 + 3)`, in place of bash's. **Gap:** bash's third way of quoting the
limit string, `<<\Endofmessage`, isn't understood.

```tessh
#!/usr/bin/env tessh
NAME = "John Doe"
RESPONDENT = "the author of this fine script"

cat <<'Endofmessage'

Hello, there, $NAME.
Greetings to you, $NAME, from $RESPONDENT.

Endofmessage

#   No parameter substitution when the "limit string" is quoted.
#   This would have the same effect:
#   cat <<"Endofmessage"

#   And, likewise:

cat <<"SpecialCharTest"

Directory listing would follow
if limit string were not quoted.
$(ls -l)

Arithmetic expansion would take place
if limit string were not quoted.
$(5 + 3)

A a single backslash would echo
if limit string were not quoted.
\\

SpecialCharTest


exit
```

```output

Hello, there, $NAME.
Greetings to you, $NAME, from $RESPONDENT.


Directory listing would follow
if limit string were not quoted.
$(ls -l)

Arithmetic expansion would take place
if limit string were not quoted.
$(5 + 3)

A a single backslash would echo
if limit string were not quoted.
\\

```

That makes it easy to write code, a script for instance, from a script.

### Example 19-8. A script that generates another script (`generate-script.sh`)

```bash
#!/bin/bash
# generate-script.sh
# Based on an idea by Albert Reiner.

OUTFILE=generated.sh         # Name of the file to generate.


# -----------------------------------------------------------
# 'Here document containing the body of the generated script.
(
cat <<'EOF'
#!/bin/bash

echo "This is a generated shell script."
#  Note that since we are inside a subshell,
#+ we can't access variables in the "outside" script.

echo "Generated file will be named: $OUTFILE"
#  Above line will not work as normally expected
#+ because parameter expansion has been disabled.
#  Instead, the result is literal output.

a=7
b=3

let "c = $a * $b"
echo "c = $c"

exit 0
EOF
) > $OUTFILE
# -----------------------------------------------------------

#  Quoting the 'limit string' prevents variable expansion
#+ within the body of the above 'here document.'
#  This permits outputting literal strings in the output file.

if [ -f "$OUTFILE" ]
then
  chmod 755 $OUTFILE
  # Make the generated file executable.
else
  echo "Problem in creating file: \"$OUTFILE\""
fi

#  This method also works for generating
#+ C programs, Perl programs, Python programs, Makefiles,
#+ and the like.

exit 0
```

```output bash
```

The bash version writes the file and stops; the tessh one also runs what
it wrote. bash's `( cat <<'EOF' … ) > $OUTFILE` needs no subshell in tessh:
the heredoc and the `>` go on the same command.

```tessh
#!/usr/bin/env tessh
OUTFILE = generated.sh         # Name of the file to generate.

# -----------------------------------------------------------
# 'Here document containing the body of the generated script.
cat <<'EOF' >$OUTFILE
#!/bin/bash

echo "This is a generated shell script."
#  Note that since we are inside a subshell,
#+ we can't access variables in the "outside" script.

echo "Generated file will be named: $OUTFILE"
#  Above line will not work as normally expected
#+ because parameter expansion has been disabled.
#  Instead, the result is literal output.

a=7
b=3

let "c = $a * $b"
echo "c = $c"

exit 0
EOF
# -----------------------------------------------------------

#  Quoting the 'limit string' prevents variable expansion
#+ within the body of the above 'here document.'
#  This permits outputting literal strings in the output file.

if (Path $OUTFILE).is_file
  chmod 755 $OUTFILE
  # Make the generated file executable.
else
  print "Problem in creating file: \"$OUTFILE\""

./generated.sh                 # (Run it, to see.)

#  This method also works for generating
#+ C programs, Perl programs, Python programs, Makefiles,
#+ and the like.

exit 0
```

```output
This is a generated shell script.
Generated file will be named: 
c = 21
```

A heredoc can be the value of a variable: in bash through command
substitution, in tessh as the captured output of the `cat` it feeds.

```bash
variable=$(cat <<SETVAR
This variable
runs over multiple lines.
SETVAR
)

echo "$variable"
```

```tessh
variable = cat <<SETVAR
This variable
runs over multiple lines.
SETVAR

print $variable
```

```output
This variable
runs over multiple lines.
```

(A triple-quoted string, `"""…"""`, is the plainer way to put lines in a
variable in tessh.)

A heredoc can also be the input of a function in the same script.

### Example 19-9. Here documents and functions (`here-function.sh`)

```bash
#!/bin/bash
# here-function.sh

GetPersonalData ()
{
  read firstname
  read lastname
  read address
  read city 
  read state 
  read zipcode
} # This certainly appears to be an interactive function, but . . .


# Supply input to the above function.
GetPersonalData <<RECORD001
Bozo
Bozeman
2726 Nondescript Dr.
Bozeman
MT
21226
RECORD001


echo
echo "$firstname $lastname"
echo "$address"
echo "$city, $state $zipcode"
echo

exit 0
```

A redirection after a Python call, a heredoc included, is that call's
`sys.stdin`, so `input` in the function reads the heredoc's lines one by
one, as `read` does. The bash function sets global variables; the tessh one
gives back the six values, and the caller unpacks them.

```tessh
#!/usr/bin/env tessh
def GetPersonalData
  [(input) for _ in (range 6)]
# This certainly appears to be an interactive function, but . . .


# Supply input to the above function.
[firstname lastname address city state zipcode] = GetPersonalData <<RECORD001
Bozo
Bozeman
2726 Nondescript Dr.
Bozeman
MT
21226
RECORD001


print
print "$firstname $lastname"
print $address
print "$city, $state $zipcode"
print

exit 0
```

```output

Bozo Bozeman
2726 Nondescript Dr.
Bozeman, MT 21226

```

## Anonymous here documents and comment blocks
<!-- help: abs-anonheredoc colon-heredoc comment-block self-document : -->

`:` is bash's do-nothing command, and tessh has it too. Given a heredoc it
does nothing with the text, but the text is still filled in first, so this
checks that variables are set: bash's `${HOSTNAME?}` is an error when
`HOSTNAME` isn't, and so is any `$name` in tessh that is neither a variable
nor in the environment.

### Example 19-10. Anonymous Here Document

```bash
#!/bin/bash

: <<TESTVARIABLES
${HOSTNAME?}${USER?}${MAIL?}  # Print error message if one of the variables not set.
TESTVARIABLES

exit $?
```

```output bash
```

The error is a `NameError`, which the tessh version catches to say which
one. (`HOSTNAME` is a bash variable, not an environment one, so it isn't
there in tessh; `HOME` and `PATH` are.)

```tessh
#!/usr/bin/env tessh
try
  : <<TESTVARIABLES
$HOME$PATH$NO_SUCH_VARIABLE  # Error if one of the variables is not set.
TESTVARIABLES
except NameError as e
  print $e
  exit 1

exit $?
```

```output
name 'NO_SUCH_VARIABLE' is not defined (and there's no environment variable NO_SUCH_VARIABLE)
```

The same trick comments out a block of lines. Quote the limit string so
that nothing in the block is filled in (an unquoted `$(rm -rf …)` would run,
in tessh as in bash).

### Example 19-11. Commenting out a block of code (`commentblock.sh`)

```bash
#!/bin/bash
# commentblock.sh

: <<COMMENTBLOCK
echo "This line will not echo."
This is a comment line missing the "#" prefix.
This is another comment line missing the "#" prefix.

&*@!!++=
The above line will cause no error message,
because the Bash interpreter will ignore it.
COMMENTBLOCK

echo "Exit value of above \"COMMENTBLOCK\" is $?."   # 0
# No error shown.
echo


#  The above technique also comes in useful for commenting out
#+ a block of working code for debugging purposes.
#  This saves having to put a "#" at the beginning of each line,
#+ then having to go back and delete each "#" later.
#  Note that the use of of colon, above, is optional.

echo "Just before commented-out code block."
#  The lines of code between the double-dashed lines will not execute.
#  ===================================================================
: <<DEBUGXXX
for file in *
do
 cat "$file"
done
DEBUGXXX
#  ===================================================================
echo "Just after commented-out code block."

exit 0



######################################################################
#  Note, however, that if a bracketed variable is contained within
#+ the commented-out code block,
#+ then this could cause problems.
#  for example:


#/!/bin/bash

  : <<COMMENTBLOCK
  echo "This line will not echo."
  &*@!!++=
  ${foo_bar_bazz?}
  $(rm -rf /tmp/foobar/)
  $(touch my_build_directory/cups/Makefile)
COMMENTBLOCK


$ sh commented-bad.sh
commented-bad.sh: line 3: foo_bar_bazz: parameter null or not set

# The remedy for this is to strong-quote the 'COMMENTBLOCK' in line 49, above.

  : <<'COMMENTBLOCK'

# Thank you, Kurt Pfeifle, for pointing this out.
```

```output bash
Exit value of above "COMMENTBLOCK" is 0.

Just before commented-out code block.
Just after commented-out code block.
```

Everything below the original's `exit 0` is notes, not bash; tessh compiles
the whole file before running it, so they're left out here. tessh has its
own way to comment out code, `#_`: placed first on a line, it drops that
line and the block indented under it, and what's dropped must still be
correct tessh.

```tessh
#!/usr/bin/env tessh
: <<'COMMENTBLOCK'
echo "This line will not echo."
This is a comment line missing the "#" prefix.
This is another comment line missing the "#" prefix.

&*@!!++=
The above line will cause no error message,
because the tessh interpreter will ignore it.
COMMENTBLOCK

print "Exit value of above \"COMMENTBLOCK\" is $?."   # 0
# No error shown.
print


#  The above technique also comes in useful for commenting out
#+ a block of working code for debugging purposes.

print "Just before commented-out code block."
#  The lines of code between the double-dashed lines will not execute.
#  ===================================================================
: <<'DEBUGXXX'
for file in *
do
 cat "$file"
done
DEBUGXXX
#  ===================================================================
print "Just after commented-out code block."

#  tessh's own way: #_ drops the form after it, here the whole loop.
#_ for file in *
  cat $file
print "Just after the #_ block."

exit 0
```

```output
Exit value of above "COMMENTBLOCK" is 0.

Just before commented-out code block.
Just after commented-out code block.
Just after the #_ block.
```

Another twist: the help text of a script, kept in a comment block, that the
script prints itself.

### Example 19-12. A self-documenting script (`self-document.sh`)

```bash
#!/bin/bash
# self-document.sh: self-documenting script
# Modification of "colm.sh".

DOC_REQUEST=70

if [ "$1" = "-h"  -o "$1" = "--help" ]     # Request help.
then
  echo; echo "Usage: $0 [directory-name]"; echo
  sed --silent -e '/DOCUMENTATIONXX$/,/^DOCUMENTATIONXX$/p' "$0" |
  sed -e '/DOCUMENTATIONXX$/d'; exit $DOC_REQUEST; fi


: <<DOCUMENTATIONXX
List the statistics of a specified directory in tabular format.
---------------------------------------------------------------
The command-line parameter gives the directory to be listed.
If no directory specified or directory specified cannot be read,
then list the current working directory.

DOCUMENTATIONXX

if [ -z "$1" -o ! -r "$1" ]
then
  directory=.
else
  directory="$1"
fi  

echo "Listing of "$directory":"; echo
(printf "PERMISSIONS LINKS OWNER GROUP SIZE MONTH DAY HH:MM PROG-NAME\n" \
; ls -l "$directory" | sed 1d) | column -t

exit 0
```

```output bash
Listing of .:

PERMISSIONS  LINKS  OWNER  GROUP  SIZE  MONTH  DAY  HH:MM  PROG-NAME
```

The tessh version is run here with `-h`, to show its help. (It runs `sed`
on its own file, `$argv[0]`, as the original does on `$0`.) A line can't
end with `|` to go on to the next in tessh: a `\` carries it on. bash's
`(printf …; ls …) | column -t` pipes two commands' output together; tessh
has no such group, so the heading and the listing are put together as text,
and a value first in a pipe is written to the next stage.

```tessh
#!/usr/bin/env tessh
DOC_REQUEST = 70
args = $argv[1:] or ["-h"]          # (sample arguments, when none are given)

if $args[0] in ["-h" "--help"]      # Request help.
  print ; print "Usage: $(Path $argv[0]).name [directory-name]" ; print
  sed --silent -e '/DOCUMENTATIONXX$/,/^DOCUMENTATIONXX$/p' $argv[0] \
    | sed -e '/DOCUMENTATIONXX$/d'
  exit $DOC_REQUEST


: <<DOCUMENTATIONXX
List the statistics of a specified directory in tabular format.
---------------------------------------------------------------
The command-line parameter gives the directory to be listed.
If no directory specified or directory specified cannot be read,
then list the current working directory.

DOCUMENTATIONXX

import os
directory = if not $args or not (os.access $args[0] $os.R_OK) then "." else $args[0]

print "Listing of $directory:" ; print
listing = ls -l $directory | sed 1d
"PERMISSIONS LINKS OWNER GROUP SIZE MONTH DAY HH:MM PROG-NAME\n$listing" | column -t

exit 0
```

```output

Usage: ex.tsh [directory-name]

List the statistics of a specified directory in tabular format.
---------------------------------------------------------------
The command-line parameter gives the directory to be listed.
If no directory specified or directory specified cannot be read,
then list the current working directory.

```

A `cat` script does the same more simply.

```bash no-run
DOC_REQUEST=70

if [ "$1" = "-h"  -o "$1" = "--help" ]     # Request help.
then                                       # Use a "cat script" . . .
  cat <<DOCUMENTATIONXX
List the statistics of a specified directory in tabular format.
---------------------------------------------------------------
The command-line parameter gives the directory to be listed.
If no directory specified or directory specified cannot be read,
then list the current working directory.

DOCUMENTATIONXX
exit $DOC_REQUEST
fi
```

```tessh
DOC_REQUEST = 70
args = $argv[1:] or ["--help"]      # (sample arguments, when none are given)

if $args[0] in ["-h" "--help"]      # Request help.
  cat <<DOCUMENTATIONXX             # Use a "cat script" . . .
List the statistics of a specified directory in tabular format.
---------------------------------------------------------------
The command-line parameter gives the directory to be listed.
If no directory specified or directory specified cannot be read,
then list the current working directory.

DOCUMENTATIONXX
  exit $DOC_REQUEST
```

```output
List the statistics of a specified directory in tabular format.
---------------------------------------------------------------
The command-line parameter gives the directory to be listed.
If no directory specified or directory specified cannot be read,
then list the current working directory.

```

Bash keeps a heredoc in a temporary file that's deleted as soon as it's
opened; tessh hands the text to the program through a pipe. Either way the
program sees only its standard input.

The closing limit string must be alone on its line, at the start: indented
(other than with tabs, after `<<-`) or with spaces after it, it isn't seen,
and the heredoc runs on to the end of the file. The same goes in tessh, even
when the command with the heredoc is in an indented block: the heredoc's
lines are text, and the limit string starts its line.

```tessh
if True
  cat <<END
  indented lines stay indented
END
  print "and the block goes on"
```

```output
  indented lines stay indented
and the block goes on
```

**Gap:** bash takes a single `!` as a limit string (`cat <<!`); tessh
doesn't. A word is safer anyway, as the ABS Guide says.

For input too complex for a heredoc, the `expect` program (or Python's
`pexpect`) talks to interactive programs.

## 19.1 Here Strings
<!-- help: abs-herestrings here-string <<< -->

A here string, `COMMAND <<< $WORD`, gives the command one word (and a
newline) as its input. tessh takes `<<<` with any value: a variable, a
quoted string or a form.

```bash no-run
# Instead of:
if echo "$VAR" | grep -q txt   # if [[ $VAR = *txt* ]]
# etc.

# Try:
if grep -q "txt" <<< "$VAR"
then   #         ^^^
   echo "$VAR contains the substring sequence \"txt\""
fi
# Thank you, Sebastian Kaminski, for the suggestion.
```

```tessh
VAR = "notes.txt.bak"           # (made up)
if grep -q "txt" <<< $VAR
  print "$VAR contains the substring sequence \"txt\""
# Without a program: if "txt" in $VAR, or $VAR =~ 'txt'.
```

```output
notes.txt.bak contains the substring sequence "txt"
```

With `read -a`, a here string splits a string into an array. In tessh
that's `.split`, which gives a list.

```bash
String="This is a string of words."

read -r -a Words <<< "$String"
#  The -a option to "read"
#+ assigns the resulting values to successive members of an array.

echo "First word in String is:    ${Words[0]}"   # This
echo "Second word in String is:   ${Words[1]}"   # is
echo "Third word in String is:    ${Words[2]}"   # a
echo "Fourth word in String is:   ${Words[3]}"   # string
echo "Fifth word in String is:    ${Words[4]}"   # of
echo "Sixth word in String is:    ${Words[5]}"   # words.
echo "Seventh word in String is:  ${Words[6]}"   # (null)
                                                 # Past end of $String.

# Thank you, Francisco Lobo, for the suggestion.
```

Past the end, a list index is an `IndexError`, not an empty string;
`get_in` with a default gives the empty string instead.

```tessh
String = "This is a string of words."

Words = ($String.split)

print "First word in String is:    $Words[0]"   # This
print "Second word in String is:   $Words[1]"   # is
print "Third word in String is:    $Words[2]"   # a
print "Fourth word in String is:   $Words[3]"   # string
print "Fifth word in String is:    $Words[4]"   # of
print "Sixth word in String is:    $Words[5]"   # words.
print "Seventh word in String is:  $(get_in $Words [6] '')"   # (empty)
                                                 # Past end of $String.
```

```output
First word in String is:    This
Second word in String is:   is
Third word in String is:    a
Fourth word in String is:   string
Fifth word in String is:    of
Sixth word in String is:    words.
Seventh word in String is:  
```

A here string can feed a loop. In bash it's the loop's input; in tessh a
loop goes over a value, so the list itself is what it goes over.

```bash
# As Seamus points out . . .

ArrayVar=( element0 element1 element2 {A..D} )

while read element ; do
  echo "$element" 1>&2
done <<< $(echo ${ArrayVar[*]})

# element0 element1 element2 A B C D
```

```output bash
```

The bash loop runs once, on the one line the here string makes, and writes
to standard error; the tessh one prints each element to standard output.

```tessh
ArrayVar = [element0 element1 element2] + (list "ABCD")

for element in $ArrayVar
  print $element
```

```output
element0
element1
element2
A
B
C
D
```

### Example 19-13. Prepending a line to a file (`prepend.sh`)

```bash no-run
#!/bin/bash
# prepend.sh: Add text at beginning of file.
#
#  Example contributed by Kenny Stauffer,
#+ and slightly modified by document author.


E_NOSUCHFILE=85

read -p "File: " file   # -p arg to 'read' displays prompt.
if [ ! -e "$file" ]
then   # Bail out if no such file.
  echo "File $file not found."
  exit $E_NOSUCHFILE
fi

read -p "Title: " title
cat - $file <<<$title > $file.new

echo "Modified file is $file.new"

exit  # Ends script execution.

  from 'man bash':
  Here Strings
  	A variant of here documents, the format is:
  
  		<<<word
  
  	The word is expanded and supplied to the command on its standard input.


  Of course, the following also works:
   sed -e '1i\
   Title: ' $file
```

The two `read -p` questions are answered here by values (the file is made
up). `cat -` reads the here string, then the file. Note `${file}.new`: a
glued `.new` after `$file` would be an attribute.

```tessh
#!/usr/bin/env tessh
E_NOSUCHFILE = 85
print "The first line of the file." >notes.txt   # (A made-up file.)

file = notes.txt          # (read -p "File: " file, answered)
if not (Path $file).exists
  # Bail out if no such file.
  print "File $file not found."
  exit $E_NOSUCHFILE

title = "Title: Notes"    # (read -p "Title: " title, answered)
cat - $file <<<$title >${file}.new

print "Modified file is ${file}.new"
cat ${file}.new           # (To see it.)

exit  # Ends script execution.
```

```output
Modified file is notes.txt.new
Title: Notes
The first line of the file.
```

### Example 19-14. Parsing a mailbox (`mailbox_grep.sh`)

```bash no-run
#!/bin/bash
#  Script by Francisco Lobo,
#+ and slightly modified and commented by ABS Guide author.
#  Used in ABS Guide with permission. (Thank you!)

# This script will not run under Bash versions -lt 3.0.


E_MISSING_ARG=87
if [ -z "$1" ]
then
  echo "Usage: $0 mailbox-file"
  exit $E_MISSING_ARG
fi

mbox_grep()  # Parse mailbox file.
{
    declare -i body=0 match=0
    declare -a date sender
    declare mail header value


    while IFS= read -r mail
#         ^^^^                 Reset $IFS.
#  Otherwise "read" will strip leading & trailing space from its input.

   do
       if [[ $mail =~ ^From  ]]   # Match "From" field in message.
       then
          (( body  = 0 ))           # "Zero out" variables.
          (( match = 0 ))
          unset date

       elif (( body ))
       then
            (( match ))
            #  echo "$mail"
            #  Uncomment above line if you want entire body
            #+ of message to display.

   elif [[ $mail ]]; then
      IFS=: read -r header value <<< "$mail"
      #                          ^^^  "here string"

      case "$header" in
      [Ff][Rr][Oo][Mm] ) [[ $value =~ "$2" ]] && (( match++ )) ;;
      # Match "From" line.
      [Dd][Aa][Tt][Ee] ) read -r -a date <<< "$value" ;;
      #                                  ^^^
      # Match "Date" line.
      [Rr][Ee][Cc][Ee][Ii][Vv][Ee][Dd] ) read -r -a sender <<< "$value" ;;
      #                                                    ^^^
      # Match IP Address (may be spoofed).
      esac

       else
          (( body++ ))
          (( match  )) &&
          echo "MESSAGE ${date:+of: ${date[*]} }"
       #    Entire $date array             ^
          echo "IP address of sender: ${sender[1]}"
       #    Second field of "Received" line    ^

       fi


    done < "$1" # Redirect stdout of file into loop.
}


mbox_grep "$1"  # Send mailbox file to function.

exit $?

# Exercises:
# ---------
# 1) Break the single function, above, into multiple functions,
#+   for the sake of readability.
# 2) Add additional parsing to the script, checking for various keywords.



$ mailbox_grep.sh scam_mail
  MESSAGE of Thu, 5 Jan 2006 08:00:56 -0500 (EST) 
  IP address of sender: 196.3.62.4
```

In tessh the function goes over the file's lines with `for`, takes a header
apart with `.partition`, and matches its name with `match`. Two things the
bash gets wrong are put right: its `^From` matches the `From:` header as
well as the line that starts a message (`^From ` with the space is meant),
and the function's `$2` is the function's own, always empty, so the search
pattern is now passed to it. Also `match` is a word of the language in
tessh, so the counter is `matched`.

```tessh
#!/usr/bin/env tessh
E_MISSING_ARG = 87
# (A made-up mailbox to try it on.)
cat <<'END' >scam_mail
From mike@example.com Thu Jan  5 08:00:56 2006
Received: from 196.3.62.4 by mail.example.com
From: "Dr. Mike" <mike@example.com>
Date: Thu, 5 Jan 2006 08:00:56 -0500 (EST)
Subject: Urgent business proposal

Dear friend, I have 25 million dollars for you.
END
args = $argv[1:] or [scam_mail Mike]   # (sample arguments, when none are given)

if not $args
  print "Usage: $(Path $argv[0]).name mailbox-file [pattern]"
  exit $E_MISSING_ARG

def mbox_grep mailbox pattern=""    # Parse mailbox file.
  body = 0 ; matched = 0
  date = [] ; sender = []

  for mail in (open $mailbox)
    mail = ($mail.removesuffix "\n")
    if $mail =~ re'^From '          # The line that starts a message.
      body = 0 ; matched = 0        # "Zero out" variables.
      date = []
    elif $body
      pass
      #  print $mail
      #  Uncomment above line if you want entire body
      #+ of message to display.
    elif $mail
      [header _ value] = ($mail.partition ":")
      match ($header.lower)
        case from                   # Match "From" line.
          if $value =~ $pattern
            matched += 1
        case date                   # Match "Date" line.
          date = ($value.split)
        case received               # Match IP Address (may be spoofed).
          sender = ($value.split)
    else
      body += 1
      if $matched
        print "MESSAGE" (if $date then "of: $(' '.join $date) " else "")
      print "IP address of sender: $(get_in $sender [1] '')"
      #    Second field of "Received" line    ^

mbox_grep *$args  # Send mailbox file to function.
```

```output
MESSAGE of: Thu, 5 Jan 2006 08:00:56 -0500 (EST) 
IP address of sender: 196.3.62.4
```

As an exercise the ABS Guide suggests using here strings in other places,
such as to give `dc` its input.
