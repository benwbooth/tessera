# Appendix F. A Detailed Introduction to I/O and I/O Redirection
<!-- help: abs-ioredirintro abs-f file-descriptors fd stdin stdout stderr redirection-intro -->

*Written by Stéphane Chazelas, and revised by the ABS Guide's author.*

Every program starts with three open file descriptors: 0, standard input,
for reading; 1, standard output, and 2, standard error, for writing. A
terminal program (`xterm`, Tessera) opens the terminal three times and
starts the shell with them, and the shell's programs inherit them unless
they're redirected. A redirection points one of them somewhere else (a
file, a pipe, `/dev/null`) for one command, a group of commands, or, with
bash's `exec`, for the rest of the script. A program whose descriptor is
closed complains:

```bash
cat /etc/hostname >&-
echo $?
```

```tessh
cat /etc/hostname >&-
print $?
```

```output
1
```

(`cat`'s complaint goes to standard error, not shown here.)

tessh starts its programs the same way, with the same three descriptors
and bash's redirections: `>f`, `2>&1`, `3>&1`, `>&-`, `<>f`, and the rest
(chapter 20). Where the descriptors point can be seen in `/proc/self/fd`:

```bash
sh -c 'readlink /proc/self/fd/1 /proc/self/fd/2' 2>&1 | sed 's/:\[[0-9]*\]//'
sh -c 'readlink /proc/self/fd/1' >/dev/null 2>&1 ; sh -c 'readlink /proc/self/fd/1' 2>/dev/null </dev/null >&2 2>&1 | cat
sh -c 'readlink /proc/self/fd/0' </dev/null
```

```tessh
sh -c 'readlink /proc/self/fd/1 /proc/self/fd/2' 2>&1 | sed 's/:\[[0-9]*\]//'
sh -c 'readlink /proc/self/fd/1' >/dev/null 2>&1 ; sh -c 'readlink /proc/self/fd/1' 2>/dev/null </dev/null >&2 2>&1 | cat
sh -c 'readlink /proc/self/fd/0' </dev/null
```

```output
pipe
pipe
/dev/null
```

The first line shows both of `sh`'s outputs going into the pipe (`2>&1`
after the pipe is set up). The second's two commands print nothing to the
pipe: the first sends everything to `/dev/null`, and the second sends its
output to where errors were going (`/dev/null`) before pointing errors
back at it. Order matters, left to right, in tessh as in bash.

Python's own printing goes through `sys.stdout` and `sys.stderr`, which
are descriptors 1 and 2 too. A redirection on a Python call (`f >file`)
points them, and the descriptors of the programs the call runs, at the
file for the length of the call.

bash's `exec 2>/dev/null` (or `exec 3>file`), which redirects a
descriptor for the rest of the script, works the same in tessh. Any other
redirection lasts one command or call; Python's `os.dup2` does it by
hand.

The appendix ends with an exercise: a script that opens two FIFOs and
descriptors 3 to 8, nests three subshells each with its own redirections,
and sends nine lines through them, for the reader to work out which line
ends up where.

```bash no-run
#! /usr/bin/env bash

mkfifo /tmp/fifo1 /tmp/fifo2
while read a; do echo "FIFO1: $a"; done < /tmp/fifo1 & exec 7> /tmp/fifo1
exec 8> >(while read a; do echo "FD8: $a, to fd7"; done >&7)

exec 3>&1
(
 (
  (
   while read a; do echo "FIFO2: $a"; done < /tmp/fifo2 | tee /dev/stderr \
   | tee /dev/fd/4 | tee /dev/fd/5 | tee /dev/fd/6 >&7 & exec 3> /tmp/fifo2

   echo 1st, to stdout
   sleep 1
   echo 2nd, to stderr >&2
   sleep 1
   echo 3rd, to fd 3 >&3
   sleep 1
   echo 4th, to fd 4 >&4
   sleep 1
   echo 5th, to fd 5 >&5
   sleep 1
   echo 6th, through a pipe | sed 's/.*/PIPE: &, to fd 5/' >&5
   sleep 1
   echo 7th, to fd 6 >&6
   sleep 1
   echo 8th, to fd 7 >&7
   sleep 1
   echo 9th, to fd 8 >&8

  ) 4>&1 >&3 3>&- | while read a; do echo "FD4: $a"; done 1>&3 5>&- 6>&-
 ) 5>&1 >&3 | while read a; do echo "FD5: $a"; done 1>&3 6>&-
) 6>&1 >&3 | while read a; do echo "FD6: $a"; done 3>&-

rm -f /tmp/fifo1 /tmp/fifo2


# For each command and subshell, figure out which fd points to what.
# Good luck!

exit 0
```

The exercise is about bash's own plumbing (`exec` on descriptors 3, 7 and
8 for the rest of the script, and subshells with their own descriptors),
which tessh doesn't have, so it isn't ported. In tessh the same plumbing
would be explicit: `os.mkfifo`, `os.pipe`, and `subprocess` with `pass_fds`.
