# Appendix J. An Introduction to Programmable Completion
<!-- help: abs-tabexpansion abs-j tab-completion completion complete compgen -->

Pressing Tab at bash's prompt completes a partly typed command, variable
or file name, and lists the choices when there are several. That's the
command line's business, not a script's. The tessh REPL does the same:
Tab completes Python names, programs, paths, `$` variables, an object's
attributes after a `.` (`os.pa` Tab, `$s.up` Tab) and `help` topics, and
shows each choice's help beside it.

Bash's `complete` and `compgen` builtins teach Tab a program's own
arguments: `complete -f -X '!*.txt' sample_command` offers only `.txt`
files after `sample_command`, and a function (by convention named for its
command, with an underscore) can offer the options a script takes.

### Example J-1. Completion script for UseGetOpt.sh (`UseGetOpt-2`)

```bash no-run
# file: UseGetOpt-2
# UseGetOpt-2.sh parameter-completion

_UseGetOpt-2 ()   #  By convention, the function name
{                 #+ starts with an underscore.
  local cur
  # Pointer to current completion word.
  # By convention, it's named "cur" but this isn't strictly necessary.

  COMPREPLY=()   # Array variable storing the possible completions.
  cur=${COMP_WORDS[COMP_CWORD]}

  case "$cur" in
    -*)
    COMPREPLY=( $( compgen -W '-a -d -f -l -t -h --aoption --debug \
                               --file --log --test --help --' -- $cur ) );;
#   Generate the completion matches and load them into $COMPREPLY array.
#   xx) May add more cases here.
#   yy)
#   zz)
  esac

  return 0
}

complete -F _UseGetOpt-2 -o filenames ./UseGetOpt-2.sh
#        ^^ ^^^^^^^^^^^^  Invokes the function _UseGetOpt-2.
```

tessh has `complete` and `compgen`: `complete -W "words" cmd` (or `-F
func`, `-f`, `-d`, `-c`) sets what Tab offers after a command. It also
takes bash's `-C command`, `-G glob`, `-X filter` (`!` keeps the
matches), `-P prefix`, `-S suffix`, `-A action` (file, directory, command,
builtin, keyword, variable, function, export, user, hostname, job,
signal) and `-o default`, `bashdefault`, `dirnames`, `plusdirs` and
`nosort`; `-o filenames` and `-o nospace` are accepted and do nothing.
`compgen` takes the same. There's no `compopt`.

A `-F` function comes in two forms. tessh's has two parameters: it's
given the word being typed and the line's words so far, and returns the
matches as a list. Any other is bash's: it's called with the command,
the word and the word before it, with `$COMP_WORDS`, `$COMP_CWORD`,
`$COMP_LINE` and `$COMP_POINT` set, and it fills `$COMPREPLY` (a global,
so `global COMPREPLY` in a `def`) or returns a list.

```tessh no-run
def _UseGetOpt_2 cmd cur prev      # bash's form: fills $COMPREPLY.
  global COMPREPLY
  COMPREPLY = [$o for o in $OPTIONS if ($o.startswith $cur)]

complete -F _UseGetOpt_2 ./UseGetOpt-2.sh
```

What the function works out is a one-line filter in tessh:

```tessh
OPTIONS = [-a -d -f -l -t -h --aoption --debug --file --log --test --help --]

def _UseGetOpt_2 cur
  # The possible completions of the word being typed.
  if $cur.startswith "-"
    [$o for o in $OPTIONS if ($o.startswith $cur)]
  else
    []

print (_UseGetOpt_2 "-")
print (_UseGetOpt_2 "--")
print (_UseGetOpt_2 "--d")
```

```output
['-a' '-d' '-f' '-l' '-t' '-h' '--aoption' '--debug' '--file' '--log' '--test' '--help' '--']
['--aoption' '--debug' '--file' '--log' '--test' '--help' '--']
['--debug']
```

As the ABS Guide says, programmers will put in long hours to save ten
minutes of work; this is known as optimization.
