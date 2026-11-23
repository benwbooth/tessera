# 25. Aliases
<!-- help: abs-aliases abs-25 alias unalias shortcut -->

A bash alias is a shortcut typed at the start of a command: with
`alias lm="ls -l | more"` in `~/.bashrc`, typing `lm` runs `ls -l | more`.
In a script aliases are of little use. They're off unless the script turns
them on, they don't take arguments in the middle of their text, they
aren't expanded inside `if`s, loops and functions defined at the same time,
and they don't expand recursively. A function does all an alias does, and
more.

**Gap:** tessh has no aliases; a function is the way. A plain `def` gets
its arguments as values. With `@command` before it, it gets them as the
words a program would, globs and all, so it reads like a command and
passes its arguments on: `def ll *words` then `ls -l *$words`. Unlike an
alias, a function works anywhere once defined, in an `if` or a loop too.
In the REPL, functions in `~/.config/tessh/rc.tsh` are there in every
session, as aliases in `~/.bashrc` are.

### Example 25-1. Aliases within a script (`alias.sh`)

```bash no-run
#!/bin/bash
# alias.sh

shopt -s expand_aliases
# Must set this option, else script will not expand aliases.


# First, some fun.
alias Jesse_James='echo "\"Alias Jesse James\" was a 1959 comedy starring Bob Hope."'
Jesse_James

echo; echo; echo;

alias ll="ls -l"
# May use either single (') or double (") quotes to define an alias.

echo "Trying aliased \"ll\":"
ll /usr/X11R6/bin/mk*   #* Alias works.

echo

directory=/usr/X11R6/bin/
prefix=mk*  # See if wild card causes problems.
echo "Variables \"directory\" + \"prefix\" = $directory$prefix"
echo

alias lll="ls -l $directory$prefix"

echo "Trying aliased \"lll\":"
lll         # Long listing of all files in /usr/X11R6/bin stating with mk.
# An alias can handle concatenated variables -- including wild card -- o.k.




TRUE=1

echo

if [ TRUE ]
then
  alias rr="ls -l"
  echo "Trying aliased \"rr\" within if/then statement:"
  rr /usr/X11R6/bin/mk*   #* Error message results!
  # Aliases not expanded within compound statements.
  echo "However, previously expanded alias still recognized:"
  ll /usr/X11R6/bin/mk*
fi  

echo

count=0
while [ $count -lt 3 ]
do
  alias rrr="ls -l"
  echo "Trying aliased \"rrr\" within \"while\" loop:"
  rrr /usr/X11R6/bin/mk*   #* Alias will not expand here either.
                           #  alias.sh: line 57: rrr: command not found
  let count+=1
done 

echo; echo

alias xyz='cat $0'   # Script lists itself.
                     # Note strong quotes.
xyz
#  This seems to work,
#+ although the Bash documentation suggests that it shouldn't.
#
#  However, as Steve Jacobson points out,
#+ the "$0" parameter expands immediately upon declaration of the alias.

exit 0
```

The tessh version lists made-up files in a `bin` folder, with `ls -1`
rather than `ls -l`, whose owners and dates change from run to run. Every
"alias" works, even the ones the original says won't. A pattern in a
variable is text; `Glob` makes it a pattern to match. `xyz` shows the first
lines of the script, `$argv[0]`.

```tessh
#!/usr/bin/env tessh
# (A made-up /usr/X11R6/bin to list.)
mkdir -p bin ; touch bin/mkdirhier bin/mkfontdir bin/xterm

# First, some fun.
def Jesse_James
  print "\"Alias Jesse James\" was a 1959 comedy starring Bob Hope."
Jesse_James

print ; print ; print

@command
def ll *words
  ls -1 *$words      # (ls -l in the original)

print "Trying \"ll\":"
ll bin/mk*

print

directory = bin/
prefix = 'mk*'  # See if wild card causes problems.
print "Variables \"directory\" + \"prefix\" = $directory$prefix"
print

def lll
  ls -1 (Glob "$directory$prefix")

print "Trying \"lll\":"
lll         # Listing of all files in bin starting with mk.

TRUE = 1

print

if $TRUE
  @command
  def rr *words
    ls -1 *$words
  print "Trying \"rr\" within if/then statement:"
  rr bin/mk*   # A function works here, as an alias wouldn't.
  print "And the one defined before:"
  ll bin/mk*

print

for count in (range 3)
  @command
  def rrr *words
    ls -1 *$words
  print "Trying \"rrr\" within \"for\" loop:"
  rrr bin/mk*

print ; print

def xyz
  head -n 3 $argv[0]   # Script lists (the start of) itself.
xyz

exit 0
```

```output
"Alias Jesse James" was a 1959 comedy starring Bob Hope.



Trying "ll":
bin/mkdirhier
bin/mkfontdir

Variables "directory" + "prefix" = bin/mk*

Trying "lll":
bin/mkdirhier
bin/mkfontdir

Trying "rr" within if/then statement:
bin/mkdirhier
bin/mkfontdir
And the one defined before:
bin/mkdirhier
bin/mkfontdir

Trying "rrr" within "for" loop:
bin/mkdirhier
bin/mkfontdir
Trying "rrr" within "for" loop:
bin/mkdirhier
bin/mkfontdir
Trying "rrr" within "for" loop:
bin/mkdirhier
bin/mkfontdir


#!/usr/bin/env tessh
# (A made-up /usr/X11R6/bin to list.)
mkdir -p bin ; touch bin/mkdirhier bin/mkfontdir bin/xterm
```

`unalias` removes an alias; `del` removes a function (any name, in fact).
Calling it after that is a `NameError`, which stops the script unless it's
caught.

### Example 25-2. unalias: Setting and unsetting an alias (`unalias.sh`)

```bash no-run
#!/bin/bash
# unalias.sh

shopt -s expand_aliases  # Enables alias expansion.

alias llm='ls -al | more'
llm

echo

unalias llm              # Unset alias.
llm
# Error message results, since 'llm' no longer recognized.

exit 0
```

```tessh
#!/usr/bin/env tessh
touch unalias.tsh        # (Something for ls to show.)

def llm
  ls -a | more

llm

print

del llm                  # Unset the function.
try
  llm
except NameError as e
  print "llm: $e"
# Error message results, since 'llm' no longer recognized.

exit 0
```

```output
.
..
unalias.tsh

llm: name 'llm' is not defined
```
