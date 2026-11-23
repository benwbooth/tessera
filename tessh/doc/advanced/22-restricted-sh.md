# 22. Restricted Shells
<!-- help: abs-restricted-sh abs-22 restricted-shell set-r rbash -->

In restricted mode (`bash -r`, or `set -r` partway through a script) bash
refuses things that would let a script, or its user, get out of the box it
was put in:

- changing folder with `cd`;
- changing `$PATH`, `$SHELL`, `$BASH_ENV` or `$ENV`, or reading or changing
  `$SHELLOPTS`;
- redirecting output;
- running a command whose name has a `/` in it;
- replacing the shell with `exec`;
- other ways around these, and leaving restricted mode.

**Gap:** tessh has no restricted mode. It couldn't have a meaningful one:
a tessh script is Python, and Python code can open files, change folders
and run programs (`open`, `os.chdir`, `subprocess`) whatever the shell
allows. To confine a script, confine the process: a separate user, a
container, or a sandbox such as `bwrap` or `systemd-run` with its
protection options.

### Example 22-1. Running a script in restricted mode (`restricted.sh`)

```bash no-run
#!/bin/bash

#  Starting the script with "#!/bin/bash -r"
#+ runs entire script in restricted mode.

echo

echo "Changing directory."
cd /usr/local
echo "Now in `pwd`"
echo "Coming back home."
cd
echo "Now in `pwd`"
echo

# Everything up to here in normal, unrestricted mode.

set -r
# set --restricted    has same effect.
echo "==> Now in restricted mode. <=="

echo
echo

echo "Attempting directory change in restricted mode."
cd ..
echo "Still in `pwd`"

echo
echo

echo "\$SHELL = $SHELL"
echo "Attempting to change shell in restricted mode."
SHELL="/bin/ash"
echo
echo "\$SHELL= $SHELL"

echo
echo

echo "Attempting to redirect output in restricted mode."
ls -l /usr/bin > bin.files
ls -l bin.files    # Try to list attempted file creation effort.

echo

exit 0
```

The tessh version runs in made-up folders rather than `/usr/local` and
your home, and has nothing to put where `set -r` was, so everything the
original's second half tries works: that's the gap, shown.

```tessh
#!/usr/bin/env tessh
mkdir -p usr/local           # (A made-up /usr/local.)
home = (pwd)                 # (And the current folder as home.)

print

print "Changing directory."
cd usr/local
print "Now in $(Path (pwd)).name"
print "Coming back home."
cd $home
print "Now in $(if (pwd) == $home then 'home' else (pwd))"
print

# Everything up to here in normal, unrestricted mode.

# set -r: tessh has no restricted mode, so none of the below is refused.
print "==> Not in restricted mode: there's none. <=="

print
print

print "Attempting directory change in restricted mode."
cd usr
print "Still in $(Path (pwd)).name"

print
print

print "Attempting to change shell in restricted mode."
$env.SHELL = /bin/ash
print
print "\$SHELL= $env.SHELL"

print
print

print "Attempting to redirect output in restricted mode."
ls -l .. > bin.files         # (ls -l /usr/bin in the original)
ls bin.files                 # Try to list attempted file creation effort.

print

exit 0
```

```output

Changing directory.
Now in local
Coming back home.
Now in home

==> Not in restricted mode: there's none. <==


Attempting directory change in restricted mode.
Still in usr


Attempting to change shell in restricted mode.

$SHELL= /bin/ash


Attempting to redirect output in restricted mode.
bin.files

```
