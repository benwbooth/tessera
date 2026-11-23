# Appendix E. Exit Codes With Special Meanings
<!-- help: abs-exitcodes abs-e exit-codes reserved-exit-status 126 127 128 130 255 -->

Some exit statuses mean something already, so a script shouldn't use them
for its own errors. The ABS Guide's table, with what tessh does:

| code | bash's meaning | bash example | in tessh |
|---|---|---|---|
| 1 | general errors (divide by zero and the like) | `let "var1 = 1/0"` | an uncaught exception (`var1 = 1 / 0`) prints a traceback and exits 1 |
| 2 | misuse of shell builtins | `empty_function() {}` | a syntax error is found before anything runs, and exits 1 |
| 126 | the command can't run | `/dev/null` | the same: `permission denied`, status 126 |
| 127 | command not found | `illegal_command` | the same: `command not found`, status 127 |
| 128 | invalid argument to `exit` | `exit 3.14159` | `exit` with anything but a whole number exits 1 |
| 128+n | killed by signal n | `kill -9 $PPID` | the same for programs and for tessh itself (137 for `kill -9`) |
| 130 | stopped by Ctrl-C (128 + 2) | Ctl-C | the same: Ctrl+C is `KeyboardInterrupt`, and the script exits 130 |
| 255 | exit status out of range | `exit -1` | `exit -1` is 255 too |

An exit status bigger than 255 is taken modulo 256 (`exit 3809` exits
225), in both.

```bash
bash -c 'exit 3809' ; echo $?
bash -c 'exit -1' ; echo $?
bash -c 'illegal_command' 2>/dev/null ; echo $?
bash -c '/dev/null' 2>/dev/null ; echo $?
bash -c 'kill -9 $$' ; echo $?
```

```output bash
225
255
127
126
137
```

```tessh
tessh -c 'exit 3809' ; print $?
tessh -c 'exit -1' ; print $?
tessh -c 'illegal_command' 2>/dev/null ; print $?
tessh -c '/dev/null' 2>/dev/null ; print $?
sh -c 'kill -9 $$' ; print $?
tessh -c 'var1 = 1 / 0' 2>/dev/null ; print $?
```

```output
225
255
127
126
137
1
```

The author proposes that scripts keep their own exit codes to 64–113, as
`/usr/include/sysexits.h` does for C (its codes, 64 to 78, are in Python
as `os.EX_USAGE`, `os.EX_DATAERR` and the rest). The examples in this
guide follow him: `E_NOARGS = 85` and so on.

In tessh, a script usually doesn't pick a status for each error at all:
it lets the exception stop it (status 1, with a traceback that says what
went wrong and where), or catches it and calls `exit` with a code of its
own.
