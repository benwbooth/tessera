# Appendix N. Converting DOS Batch Files to Shell Scripts
<!-- help: abs-dosbatch abs-n dos batch bat windows cmd -->

Many people learned scripting with DOS batch files, and now and then one
needs converting. Batch's operators are a small subset of the shell's, so
it's mostly a matter of looking things up. Here is the ABS Guide's table
with a tessh column.

| batch | bash | tessh | meaning |
|---|---|---|---|
| `%1` | `$1` | `$argv[1]` | command-line parameter |
| `/` | `-` | `-` | option flag |
| `\` | `/` | `/` | path separator |
| `==` | `=` | `==` | string equality |
| `!==!` | `!=` | `!=` | string inequality |
| `\|` | `\|` | `\|` | pipe |
| `@` | `set +v` | (nothing is echoed) | don't echo this command |
| `*` | `*` | `*` | wildcard |
| `>`, `>>`, `<` | `>`, `>>`, `<` | `>`, `>>`, `<` | redirection |
| `%VAR%` | `$VAR` | `$VAR`, `$env.VAR` | environment variable |
| `REM` | `#` | `#` | comment |
| `NOT` | `!` | `not` | negate a test |
| `NUL` | `/dev/null` | `/dev/null` | the black hole |
| `ECHO`, `ECHO.` | `echo`, `echo` | `print`, `print` | print, print a blank line |
| `ECHO OFF` | `set +v` | (nothing is echoed) | don't echo commands |
| `FOR %%VAR IN (LIST) DO` | `for var in list; do` | `for var in [list]` | for loop |
| `:LABEL`, `GOTO` | none (use a function) | none (use a function) | jump |
| `PAUSE` | `sleep` | `sleep`, `time.sleep` | wait |
| `CHOICE` | `case`, `select` | `match`, `input` | menu choice |
| `IF`, `IF EXIST FILENAME` | `if`, `if [ -e filename ]` | `if`, `if (os.path.exists filename)` | test |
| `IF !%N==!` | `if [ -z "$N" ]` | `if (len $argv) <= N` | parameter N not given |
| `CALL`, `COMMAND /C` | `source`, `.` | `source`, `import` | include another script |
| `SET` | `export` | `export`, `$env.VAR = …` | set an environment variable |
| `SHIFT` | `shift` | `$args.pop 0` | shift the arguments |
| `SGN` | `-lt`, `-gt` | `<`, `>` | sign |
| `ERRORLEVEL` | `$?` | `$?`, `.code` | exit status |
| `CON`, `PRN`, `LPT1`, `COM1` | stdin, `/dev/lp0`, `/dev/lp0`, `/dev/ttyS0` | the same | devices |

The DOS commands become Unix programs, which tessh runs as bash does:

| DOS | Unix | | DOS | Unix |
|---|---|---|---|---|
| `ASSIGN` | `ln` | | `FIND` | `grep` |
| `ATTRIB` | `chmod` | | `MD`, `MKDIR` | `mkdir` |
| `CD`, `CHDIR` | `cd` | | `MORE` | `more` |
| `CLS` | `clear` | | `MOVE`, `REN`, `RENAME` | `mv` |
| `COMP`, `FC` | `diff`, `comm`, `cmp` | | `PATH` | `$PATH` |
| `COPY`, `XCOPY` | `cp` | | `RD`, `RMDIR` | `rmdir` |
| `Ctl-C`, `Ctl-Z` | `Ctl-C`, `Ctl-D` | | `SORT` | `sort` |
| `DEL`, `ERASE` | `rm` | | `TIME` | `date` |
| `DELTREE` | `rm -rf` | | `TYPE` | `cat` |
| `DIR` | `ls -l` | | `EXIT` | `exit` |

DOS's wildcards are only `*` and `?`; its auxiliary programs (`ask.com`)
stand in for what the shell has built in.

### Example N-1. VIEWDATA.BAT: DOS Batch File (`VIEWDATA.BAT`)

```bat
REM VIEWDATA

REM INSPIRED BY AN EXAMPLE IN "DOS POWERTOOLS"
REM                           BY PAUL SOMERSON


@ECHO OFF

IF !%1==! GOTO VIEWDATA
REM  IF NO COMMAND-LINE ARG...
FIND "%1" C:\BOZO\BOOKLIST.TXT
GOTO EXIT0
REM  PRINT LINE WITH STRING MATCH, THEN EXIT.

:VIEWDATA
TYPE C:\BOZO\BOOKLIST.TXT | MORE
REM  SHOW ENTIRE FILE, 1 PAGE AT A TIME.

:EXIT0
```

Without an argument it shows the whole book list a page at a time;
with one it prints the lines that have it. The conversion follows.

### Example N-2. viewdata.sh: Shell Script Conversion of VIEWDATA.BAT (`viewdata.sh`)

```bash no-run
#!/bin/bash
# viewdata.sh
# Conversion of VIEWDATA.BAT to shell script.

DATAFILE=/home/bozo/datafiles/book-collection.data
ARGNO=1

# @ECHO OFF                 Command unnecessary here.

if [ $# -lt "$ARGNO" ]    # IF !%1==! GOTO VIEWDATA
then
  less $DATAFILE          # TYPE C:\MYDIR\BOOKLIST.TXT | MORE
else
  grep "$1" $DATAFILE     # FIND "%1" C:\MYDIR\BOOKLIST.TXT
fi  

exit 0                    # :EXIT0

#  GOTOs, labels, smoke-and-mirrors, and flimflam unnecessary.
#  The converted script is short, sweet, and clean,
#+ which is more than can be said for the original.
```

In tessh the same, with the argument test on `$argv`. A made-up book list
stands in for the data file; `less` copies it when its output isn't a
terminal, which is what happens here, with no argument.

```tessh
#!/usr/bin/env tessh
# Conversion of VIEWDATA.BAT to tessh.

# (A made-up book list to try it on.)
printf "Moby Dick, Melville\nUlysses, Joyce\nDubliners, Joyce\n" >book-collection.data

DATAFILE = ./book-collection.data   # (/home/bozo/datafiles/... in the original)
ARGNO = 1

if (len $argv) - 1 < $ARGNO         # IF !%1==! GOTO VIEWDATA
  less $DATAFILE                    # TYPE C:\MYDIR\BOOKLIST.TXT | MORE
else
  grep $argv[1] $DATAFILE           # FIND "%1" C:\MYDIR\BOOKLIST.TXT

exit 0                              # :EXIT0
```

```output
Moby Dick, Melville
Ulysses, Joyce
Dubliners, Joyce
```

The ABS Guide's author adds that prettifying the batch file is wasted
effort, since a shell script can work on a DOS or even an NTFS file
system directly (the latter with `ntfs-3g`).
