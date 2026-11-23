# 35. Scripting With Style
<!-- help: abs-scrstyle abs-35 style stylesheet naming comments docstrings -->

Even a script written on the back of an envelope is better for a few
minutes' planning. These are the ABS Guide's suggestions, which hold for
tessh with a few changes, since a tessh script is also a Python program
and Python has its own long-settled style (PEP 8): lowercase names with
underscores for variables and functions, CapWords for classes, capitals
for constants.

## 35.1 Unofficial Shell Scripting Stylesheet
<!-- help: abs-unofficialst stylesheet style-guide magic-numbers exit-codes flags -->

**Comment your code.** It makes perfect sense when you write it and is a
mystery a year later:

```bash no-run
PASS="$PASS${MATRIX:$(($RANDOM%${#MATRIX})):1}"
#  It made perfect sense when you wrote it last year,
#+ but now it's a complete mystery.
#  (From Antek Sawicki's "pw.sh" script.)
```

tessh helps by letting you say it plainly, `password += (random.choice
$MATRIX)`, but the comment saying why is still yours to write.

**Add descriptive headers to scripts and functions.** The same comment
blocks serve in tessh. (In Python a string as a function's first line is
its docstring, which `help` shows. **Gap:** tessh drops such a string, so
the header stays a comment for now.)

```bash no-run
#!/bin/bash
#************************************************#
#                   xyz.sh                       #
#           written by Bozo Bozeman              #
#                July 05, 2001                   #
#                                                #
#           Clean up project files.              #
#************************************************#

E_BADDIR=85                       # No such directory.
projectdir=/home/bozo/projects    # Directory to clean up.

# --------------------------------------------------------- #
# cleanup_pfiles ()                                         #
# Removes all files in designated directory.                #
# Parameter: $target_directory                              #
# Returns: 0 on success, $E_BADDIR if something went wrong. #
# --------------------------------------------------------- #
cleanup_pfiles ()
{
  if [ ! -d "$1" ]  # Test if target directory exists.
  then
    echo "$1 is not a directory."
    return $E_BADDIR
  fi

  rm -f "$1"/*
  return 0   # Success.
}  

cleanup_pfiles $projectdir

exit $?
```

```tessh
#!/usr/bin/env tessh
#************************************************#
#                   xyz.tsh                      #
#           written by Bozo Bozeman              #
#                July 05, 2001                   #
#                                                #
#           Clean up project files.              #
#************************************************#

E_BADDIR = 85                     # No such directory.
projectdir = ./projects           # Directory to clean up.
                                  # (/home/bozo/projects in the original.)
# (A made-up project folder to try it on.)
mkdir -p $projectdir ; touch $projectdir/a.o $projectdir/b.o

# --------------------------------------------------------- #
# cleanup_pfiles                                            #
# Removes all files in designated directory.                #
# Parameter: target_directory                               #
# Returns: 0 on success, E_BADDIR if something went wrong.  #
# --------------------------------------------------------- #
def cleanup_pfiles target_directory
  if not ($target_directory.is_dir)
    print "$target_directory is not a directory."
    return $E_BADDIR
  rm -f $target_directory/*
  0   # Success.

status = cleanup_pfiles $projectdir
print (len (list ($projectdir.iterdir))) files left
exit $status
```

```output
0 files left
```

**Avoid magic numbers**: literal constants wired into the code. Give them
names, so a change is one line:

```bash no-run
if [ -f /var/log/messages ]
then
  ...
fi
#  A year later, you decide to change the script to check /var/log/syslog.
#  It is now necessary to manually change the script, instance by instance,
#+ and hope nothing breaks.

# A better way:
LOGFILE=/var/log/messages  # Only line that needs to be changed.
if [ -f "$LOGFILE" ]
then
  ...
fi
```

In tessh: `LOGFILE = /var/log/messages` (a `Path`), then `if
($LOGFILE.is_file)`.

**Choose descriptive names.** `file_listing = ls -al $dirname`, not `fl`.
The ABS Guide's conventions carry over: capitals for constants (`MAXVAL =
10`), `E_` and capitals for exit codes (`E_NOTFOUND = 95`), capitals for
environment variables (`export MAIL_DIRECTORY=…`). For functions, Python's
habit is lowercase with underscores (`get_answer`) rather than the ABS
Guide's `GetAnswer`; CapWords is for classes. A name starting with `_`
means "private" in Python, so leave it for internals, as the Guide says.

```bash no-run
GetAnswer ()                         #  Mixed case works well for a
{                                    #+ function name, especially
  prompt=$1                          #+ when it improves legibility.
  echo -n $prompt
  read answer
  return $answer
}  

GetAnswer "What is your favorite number? "
favorite_number=$?
echo $favorite_number
```

A tessh function returns the number itself, not through the exit status
(which can't go above 255). The answer is canned here; `input $prompt` is
the interactive way.

```tessh
def get_answer prompt answers
  print $prompt end=""
  answer = next $answers           # (input $prompt, interactively)
  print $answer
  int $answer

favorite_number = get_answer "What is your favorite number? " (iter ["42"])
print $favorite_number
```

```output
What is your favorite number? 42
42
```

**Use exit codes systematically**, with names: `E_WRONG_ARGS = 95` and
`exit $E_WRONG_ARGS`. Ender suggests the codes in
`/usr/include/sysexits.h`; Python has them as `os.EX_USAGE`,
`os.EX_DATAERR`, `os.EX_NOINPUT` and so on.

**Use standard flags.** Ender proposes `-a` all, `-b` brief, `-c` copy,
`-d` daily, `-e` extended, `-h` help, `-l` log, `-m` manual, `-n` numbers,
`-r` recursive, `-s` setup, `-u` usage, `-v` verbose, `-V` version.
Python's `argparse` builds `-h`/`--help` and the usage message for you, and
reports bad flags itself.

**Break complex scripts into modules.** Functions, and in tessh whole
files: a `name.tsh` next to the script is a module it can `import`.

**Don't use a complex construct where a simple one will do.**

```bash no-run
COMMAND
if [ $? -eq 0 ]
...
# Redundant and non-intuitive.

if COMMAND
...
# More concise (if perhaps not quite as legible).
```

The same in tessh: `if grep -q x file` tests the program directly. And
where Python offers a direct way, prefer it to a clever one: a
comprehension over a loop of appends, `with open` over hand-closed files,
an exception over a status checked by hand.

Landon Noll, after reading the Bourne shell's source: simple algorithms
can be made cryptic, and so useless, by a poor choice of code style.
