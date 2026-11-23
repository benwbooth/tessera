# 31. Of Zeros and Nulls
<!-- help: abs-zeros abs-31 /dev/null /dev/zero null zero black-hole swapfile ramdisk -->

`/dev/null` and `/dev/zero` are pseudo-devices: files that exist only in
software. tessh's redirections are bash's, so everything below works the
same; Python adds `subprocess.DEVNULL` and `os.devnull` for the same black
hole, and `bytes n` for n zero bytes without reading a device.

**Uses of /dev/null.** Think of it as a black hole, a write-only file:
everything written to it disappears, and reading it gives nothing at once.
It silences output, errors or both, and empties a file while keeping the
file and its permissions.

```bash
filename=notes.txt ; badname=nope.txt
echo "some text" > $filename

cat $filename >/dev/null
# Contents of the file will not list to stdout.

rm $badname 2>/dev/null
#           So error messages [stderr] deep-sixed.

cat $filename 2>/dev/null >/dev/null
#  This can be useful in situations where the return code from a command
#+ needs to be tested, but no output is desired.
if cat $badname &>/dev/null; then echo "$badname is readable"; fi

cat /dev/null > $filename
#  : > $filename   has same effect, but does not spawn a new process.
wc -c < $filename
```

A failing program doesn't stop a tessh script (its status is `$?` and the
`Output`'s truth), so these lines behave as bash's. A Python call that
fails raises instead; there, `try` is what keeps quiet, not `2>/dev/null`.

```tessh
filename = notes.txt ; badname = nope.txt
print "some text" >$filename

cat $filename >/dev/null
# Contents of the file will not list to stdout.

rm $badname 2>/dev/null
#           So error messages [stderr] deep-sixed.

cat $filename 2>/dev/null >/dev/null
#  Useful when the return code from a command needs to be tested,
#+ but no output is desired.
if cat $badname &>/dev/null
  print "$badname is readable"

cat /dev/null >$filename
#  : >$filename   has the same effect (in Python: (open $filename w).close).
wc -c <$filename
```

```output
0
```

### Example 31-1. Hiding the cookie jar

```bash no-run
# Obsolete Netscape browser.
# Same principle applies to newer browsers.

if [ -f ~/.netscape/cookies ]  # Remove, if exists.
then
  rm -f ~/.netscape/cookies
fi

ln -s /dev/null ~/.netscape/cookies
# All cookies now get sent to a black hole, rather than saved to disk.
```

The same, with `Path`. It works on a made-up home folder, not `~`.

```tessh
# (A made-up home folder, with a cookie jar in it, to try it on.)
home = ./home
mkdir -p $home/.netscape ; print "a cookie" >$home/.netscape/cookies

# Obsolete Netscape browser.
# Same principle applies to newer browsers.
jar = $home / .netscape/cookies

if ($jar.is_file)              # Remove, if exists.
  $jar.unlink

$jar.symlink_to /dev/null
# All cookies now get sent to a black hole, rather than saved to disk.
print $jar "->" ($jar.readlink)
```

```output
home/.netscape/cookies -> /dev/null
```

**Uses of /dev/zero.** It's a stream of null bytes (binary zeros, not the
character `0`). Its main use is a file of a set size filled with zeros: a
swap file, a filesystem image for a loopback device, or overwriting a file
before deleting it. Looking at the zeros takes `od` or a hex editor; in
Python, `bytes n` is n of them.

```tessh
head -c 8 /dev/zero | od -An -tx1
print (bytes 8)
```

```output
 00 00 00 00 00 00 00 00
b'\x00\x00\x00\x00\x00\x00\x00\x00'
```

### Example 31-2. Setting up a swapfile using /dev/zero (`ex73.sh`)

```bash no-run
#!/bin/bash
# Creating a swap file.

#  A swap file provides a temporary storage cache
#+ which helps speed up certain filesystem operations.

ROOT_UID=0         # Root has $UID 0.
E_WRONG_USER=85    # Not root?

FILE=/swap
BLOCKSIZE=1024
MINBLOCKS=40
SUCCESS=0


# This script must be run as root.
if [ "$UID" -ne "$ROOT_UID" ]
then
  echo; echo "You must be root to run this script."; echo
  exit $E_WRONG_USER
fi  
  

blocks=${1:-$MINBLOCKS}          #  Set to default of 40 blocks,
                                 #+ if nothing specified on command-line.
# This is the equivalent of the command block below.
# --------------------------------------------------
# if [ -n "$1" ]
# then
#   blocks=$1
# else
#   blocks=$MINBLOCKS
# fi
# --------------------------------------------------


if [ "$blocks" -lt $MINBLOCKS ]
then
  blocks=$MINBLOCKS              # Must be at least 40 blocks long.
fi  


######################################################################
echo "Creating swap file of size $blocks blocks (KB)."
dd if=/dev/zero of=$FILE bs=$BLOCKSIZE count=$blocks  # Zero out file.
mkswap $FILE $blocks             # Designate it a swap file.
swapon $FILE                     # Activate swap file.
retcode=$?                       # Everything worked?
#  Note that if one or more of these commands fails,
#+ then it could cause nasty problems.
######################################################################

#  Exercise:
#  Rewrite the above block of code so that if it does not execute
#+ successfully, then:
#    1) an error message is echoed to stderr,
#    2) all temporary files are cleaned up, and
#    3) the script exits in an orderly fashion with an
#+      appropriate error code.

echo "Swap file created and activated."

exit $retcode
```

It needs root, so it isn't run here. `${1:-$MINBLOCKS}` is `or`, and the
"at least 40" test is `max`. The tessh version also does the exercise:
each program's result is checked with `.ok`, a failure raises, and the
`except` prints to stderr, removes the half-made file and exits with an
error code. (Writing 40 KB of zeros is `bytes`, no `dd` needed.)

```tessh no-run
# Creating a swap file.
import os sys

ROOT_UID = 0         # Root has uid 0.
E_WRONG_USER = 85    # Not root?
E_SWAP_FAILED = 86

FILE = /swap
BLOCKSIZE = 1024
MINBLOCKS = 40

# This script must be run as root.
if (os.getuid) != $ROOT_UID
  print ; print "You must be root to run this script." ; print
  exit $E_WRONG_USER

#  Default of 40 blocks if nothing is given on the command line,
#+ and at least 40 in any case.
blocks = max (int (if (len $argv) > 1 then $argv[1] else $MINBLOCKS)) $MINBLOCKS

print "Creating swap file of size $blocks blocks (KB)."
try
  $FILE.write_bytes (bytes ($blocks * $BLOCKSIZE))   # Zero out file.
  for step in [[mkswap $FILE] [swapon $FILE]]     # Designate it a swap file,
    r = command *$step                            # and activate it.
    if not $r.ok
      raise (OSError "$step[0] failed with status $r.code")
except OSError as e
  print "Swap file not set up: $e" file=$sys.stderr
  $FILE.unlink missing_ok=True
  exit $E_SWAP_FAILED

print "Swap file created and activated."
```

### Example 31-3. Creating a ramdisk (`ramdisk.sh`)

```bash no-run
#!/bin/bash
# ramdisk.sh

#  A "ramdisk" is a segment of system RAM memory
#+ which acts as if it were a filesystem.
#  Its advantage is very fast access (read/write time).
#  Disadvantages: volatility, loss of data on reboot or powerdown,
#+                less RAM available to system.
#
#  Of what use is a ramdisk?
#  Keeping a large dataset, such as a table or dictionary on ramdisk,
#+ speeds up data lookup, since memory access is much faster than disk access.


E_NON_ROOT_USER=70             # Must run as root.
ROOTUSER_NAME=root

MOUNTPT=/mnt/ramdisk           # Create with mkdir /mnt/ramdisk.
SIZE=2000                      # 2K blocks (change as appropriate)
BLOCKSIZE=1024                 # 1K (1024 byte) block size
DEVICE=/dev/ram0               # First ram device

username=`id -nu`
if [ "$username" != "$ROOTUSER_NAME" ]
then
  echo "Must be root to run \"`basename $0`\"."
  exit $E_NON_ROOT_USER
fi

if [ ! -d "$MOUNTPT" ]         #  Test whether mount point already there,
then                           #+ so no error if this script is run
  mkdir $MOUNTPT               #+ multiple times.
fi

##############################################################################
dd if=/dev/zero of=$DEVICE count=$SIZE bs=$BLOCKSIZE  # Zero out RAM device.
                                                      # Why is this necessary?
mke2fs $DEVICE                 # Create an ext2 filesystem on it.
mount $DEVICE $MOUNTPT         # Mount it.
chmod 777 $MOUNTPT             # Enables ordinary user to access ramdisk.
                               # However, must be root to unmount it.
##############################################################################
# Need to test whether above commands succeed. Could cause problems otherwise.
# Exercise: modify this script to make it safer.

echo "\"$MOUNTPT\" now available for use."
# The ramdisk is now accessible for storing files, even by an ordinary user.

#  Caution, the ramdisk is volatile, and its contents will disappear
#+ on reboot or power loss.
#  Copy anything you want saved to a regular directory.

# After reboot, run this script to again set up ramdisk.
# Remounting /mnt/ramdisk without the other steps will not work.

#  Suitably modified, this script can by invoked in /etc/rc.d/rc.local,
#+ to set up ramdisk automatically at bootup.
#  That may be appropriate on, for example, a database server.

exit 0
```

Root again, so not run here. `mkdir -p` (or `$MOUNTPT.mkdir
exist_ok=True`) makes the "already there?" test unnecessary. As the
exercise asks, the tessh version stops at the first step that fails rather
than mounting a half-made disk: `&&` chains the steps. (Today `mount -t
tmpfs -o size=2m tmpfs /mnt/ramdisk` does all of this in one step, and
`/dev/shm` is usually a ready-made ramdisk.)

```tessh no-run
# ramdisk: RAM that acts as if it were a filesystem.
import getpass

E_NON_ROOT_USER = 70           # Must run as root.
E_FAILED = 71
ROOTUSER_NAME = root

MOUNTPT = /mnt/ramdisk
SIZE = 2000                    # 2K blocks (change as appropriate)
BLOCKSIZE = 1024               # 1K (1024 byte) block size
DEVICE = /dev/ram0             # First ram device

if (getpass.getuser) != $ROOTUSER_NAME
  print "Must be root to run \"$((Path $argv[0]).name)\"."
  exit $E_NON_ROOT_USER

mkdir -p $MOUNTPT              # No error if this script is run twice.

# Zero out the RAM device, make an ext2 filesystem on it, and mount it;
# chmod lets ordinary users in (only root can unmount it).
dd if=/dev/zero of=$DEVICE count=$SIZE bs=$BLOCKSIZE && mke2fs -q $DEVICE && mount $DEVICE $MOUNTPT && chmod 777 $MOUNTPT
if $? != 0
  print "Setting up the ramdisk failed."
  exit $E_FAILED

print "\"$MOUNTPT\" now available for use."
#  Caution, the ramdisk is volatile: its contents disappear on reboot.
```

Besides all this, `/dev/zero` is used by the loader of ELF binaries.
