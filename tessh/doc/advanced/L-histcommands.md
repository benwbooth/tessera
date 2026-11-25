# Appendix L. History Commands
<!-- help: abs-histcommands abs-l history !! fc HISTFILE -->

Bash keeps the commands you type, and lets you list them (`history`),
edit and rerun them (`fc`), and recall them with `!` expansions: `!!` the
last command, `!$` its last argument, `!N` command number N, `!-N` the Nth
back, `!STRING` the last that starts with STRING, `!?STRING?` the last
that contains it, `^old^new^` the last with old replaced by new. Variables
tune it: `$HISTFILE`, `$HISTSIZE`, `$HISTFILESIZE`, `$HISTCONTROL`,
`$HISTIGNORE`, `$HISTTIMEFORMAT`, `$HISTCMD`.

tessh's REPL keeps its history in `~/.local/share/tessh/history`. ↑ and ↓
go through it (PgUp and PgDn a whole multi-line entry at a time), Ctrl+R
searches it, and the grey suggestion after the cursor comes from it (→ or
End takes it). `history [N]` lists the entries (the last N), and
`history -c`, `history -d N` and `history -s text` clear the list, delete
an entry and add one; `fc -l [-nr] [first [last]]` lists them as bash's
does. Like any builtin, `history` used as a value gives what it prints,
and in a pipe its output goes down the pipe: `history | grep ssh`.
At the prompt, `!!`, `!$`, `!^`, `!*`, `!n`, `!-n`, `!prefix`,
`!?text?` and `!#` (the line so far) bring back a past command or its
words, as bash's do, and the line is shown as it runs. Word designators
pick words (`:0`, `:n`, `:x-y`, `:x-`, `:$`, `:^`, `:*`, `:%`), and the
modifiers `:h`, `:t`, `:r`, `:e`, `:p`, `:q`, `:x`, `:s/old/new/`,
`:gs/old/new/`, `:&` and `:g&` change them. `^old^new^` reruns the last
command with old replaced by new. `fc` edits past commands in
`$FCEDIT`, `$EDITOR` or vi and runs what's saved, and `fc -s old=new`
runs one again at once. `$HISTSIZE`, `$HISTFILESIZE`, `$HISTCONTROL`
(`ignorespace`, `ignoredups`, `ignoreboth`, `erasedups`) and
`$HISTIGNORE` work as bash's, set as tessh variables (in `rc.tsh`) or in
the environment. `$HISTCMD` is the running command's number. By design,
`$HISTFILE` is always tessh's own file, so bash's history doesn't get
tessh's lines, and with no times kept in it there's no `$HISTTIMEFORMAT`.

```tessh no-run
cp notes.txt /tmp/backup/
^notes^todo^                       # cp todo.txt /tmp/backup/
ls !$:h                            # ls /tmp/backup
```

The ABS Guide notes that history is no use in a script anyway: it's
turned off there.

```bash
#!/bin/bash
# history.sh
# A (vain) attempt to use the 'history' command in a script.

history                      # No output.

var=$(history); echo "$var"  # $var is empty.

#  History commands are, by default, disabled within a script.
#  However, as dhw points out,
#+ set -o history
#+ enables the history mechanism.

set -o history
var=$(history); echo "$var"   # 1  var=$(history)
```

```output bash

    1  var=$(history); echo "$var"   # 1  var=$(history)
```

A tessh script has no history of its own; the REPL's file can be read
like any other,
an entry to a line (with a multi-line entry's lines joined by an escape),
but a script that depends on it depends on what you typed, so it's for
your own use at the REPL:

```tessh no-run
#!/usr/bin/env tessh
hist = (Path ~/.local/share/tessh/history).read_text
print *($hist.splitlines)[-5:] sep="\n"    # The last five entries.
```

The ABS Guide points to "Advancing in the Bash Shell"
(<http://samrowe.com/wordpress/advancing-in-the-bash-shell/>) for more on
bash's history.
