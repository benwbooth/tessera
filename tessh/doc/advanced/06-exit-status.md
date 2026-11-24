# 6. Exit and Exit Status
<!-- help: abs-exit-status abs-6 exit exit-status $? return-status -->

*... there are dark corners in the Bourne shell, and people use all of
them.* --Chet Ramey

`exit` ends a script, and can hand a number, its exit status, to whoever
ran it. Every command has one: 0 for success, 1 to 255 for some failure.
In bash a function's status is that of its last command, and a script's
too, which is why a bare `exit` (or `exit $?`, or none at all) passes on
the last command's status.

tessh keeps exit statuses for commands: `$?` is the last command's, and a
command's value (an `Output`) has it as `.code`, with `.ok` for "it was 0"
and a truth value to match. A script that ends without `exit` ends with
the last program's status, as in bash, but a bare `exit` is `exit 0`, so
write `exit $?` to pass a failure on. tessh's functions return values, not
statuses: `return 5` gives 5 to the caller, and leaves `$?` alone.

Builtins give statuses too: `cd dir || exit 1`, `if cd dir` and `$?` work
as in bash. Where bash goes on after a builtin fails on a line of its own,
tessh stops the script, as it does for a Python error (a missing file, a
bad conversion). `(expr)?` turns a Python error into a false value, so
`(int $s)? || 0` falls back the way `cmd || other` does, and `a ||? b` says
the same without the parentheses. The other way round, `(cmd)!` turns a
failed command into an exception, `CommandFailed`, with the `.status`, and
strict mode (`strict`, `set -e`, `tessh -e`) does that for every program
that fails on a line of its own (chapter 33).

### Example 6-1. exit / exit status (`ex5.sh`)

```bash
#!/bin/bash

echo hello
echo $?    # Exit status 0 returned because command executed successfully.

lskdf      # Unrecognized command.
echo $?    # Non-zero exit status returned -- command failed to execute.

echo

exit 113   # Will return 113 to shell.
           # To verify this, type "echo $?" after script terminates.

#  By convention, an 'exit 0' indicates success,
#+ while a non-zero exit value means an error or anomalous condition.
#  See the "Exit Codes With Special Meanings" appendix.
```

```output bash
hello
0
127

```

A program that isn't there is status 127 in tessh too (with a message on
standard error), and the script goes on. Capturing a command gives its
status as `.code`, which is often easier than `$?`.

```tessh
#!/usr/bin/env tessh
echo hello
print $?    # Exit status 0 returned because command executed successfully.

lskdf       # Unrecognized command.
print $?    # Non-zero exit status returned -- command failed to execute.

r = ls /no/such/folder 2>/dev/null
print $r.code $r.ok (bool $r)   # The status, kept with the output.

print

exit 113    # Will return 113 to the shell.
```

```output
hello
0
127
2 False False

```

A tessh function gives back a value; the status of a program it runs is in
`$?` after it, as in bash:

```bash
f () { false; }
f
echo $?
g () { return 5; }
g
echo $?
```

```tessh
def f
  false
f
print $?          # The last program's status: false's.
def g
  return 5
print (g)         # A function's value is what it returns.
```

```output
1
5
```

## Negating a condition with `!`
<!-- help: abs-negcond ! negate not invert-status -->

`!` before a command inverts its status for `&&`, `||` and `if`, in both
shells. In tessh `not` inverts any value, a command's result included.

### Example 6-2. Negating a condition using `!`

```bash
true    # The "true" builtin.
echo "exit status of \"true\" = $?"     # 0

! true
echo "exit status of \"! true\" = $?"   # 1
# Note that the "!" needs a space between it and the command.
#    !true   leads to a "command not found" error
#
# The '!' operator prefixing a command invokes the Bash history mechanism.

true
!true
# No error this time, but no negation either.
# It just repeats the previous command (true).

# =========================================================== #
# Preceding a _pipe_ with ! inverts the exit status returned.
ls | bogus_command     # bash: bogus_command: command not found
echo $?                # 127

! ls | bogus_command   # bash: bogus_command: command not found
echo $?                # 0
# Note that the ! does not change the execution of the pipe.
# Only the exit status changes.
# =========================================================== #

# Thanks, Stéphane Chazelas and Kristopher Newsome.
```

tessh's history expansion (at the prompt: `!!`, `!$`, `!n`, `!prefix`…)
works on `!true` there too, as bash's does; in a script it's only the name
of a program that isn't there. The rest is the same. (`not` inverts any value, a
command's result included: `not (true)` is `False`.)

```tessh
true
print "exit status of \"true\" = $?"       # 0

! true
print "exit status of \"! true\" = $?"     # 1

# =========================================================== #
# Preceding a _pipe_ with ! inverts the exit status returned.
ls | bogus_command 2>/dev/null
print $?                                   # 127

! ls | bogus_command 2>/dev/null
print $?                                   # 0
# Note that the ! does not change the execution of the pipe.
# Only the exit status changes.
# =========================================================== #
```

```output
exit status of "true" = 0
exit status of "! true" = 1
127
0
```

Some exit statuses have reserved meanings (126, 127, 128 + a signal's
number, 255): Appendix E lists them. Don't use them for your own errors.
