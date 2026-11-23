# 17. System and Administrative Commands
<!-- help: abs-system abs-17 sysadmin administrative root system-commands -->

These are the commands of system administration, most of them run by root
from the start-up and shut-down scripts. They're programs, so tessh runs
them exactly as bash does; what changes is the script around them. The
examples that need root, a terminal, the network or hardware can't run
here and are marked so; where it's safe, they run on made-up files instead.
Use these commands with care: several can damage a system.

## Users and Groups
<!-- help: abs-sys-users users groups chown chgrp useradd userdel usermod id who whoami logname su sudo passwd last -->

**users** and **who** show who's logged in, **groups** and **id** the
groups and user IDs (bash's `$GROUPS`, `$UID` and `$EUID`), **whoami** and
**logname** the user's name. In tessh these values come from Python's `os`
module: `(os.getuid)`, `(os.geteuid)`, `(os.getgroups)`, `(os.getlogin)`,
and `pwd.getpwuid` for the name of a user ID. **chown** and **chgrp**
change who owns a file (`os.chown` and `shutil.chown` from Python);
**useradd**, **userdel**, **usermod** and **groupmod** manage accounts;
**su** and **sudo** run commands as someone else; **ac** and **last** read
the log of logins; **newgrp** changes the current group.

**passwd** sets a password. It can be used in a script, but probably
shouldn't be.

### Example 17-1. Setting a new password (`setnew-passwd.sh`)

```bash no-run
#!/bin/bash
#  setnew-password.sh: For demonstration purposes only.
#                      Not a good idea to actually run this script.
#  This script must be run as root.

ROOT_UID=0         # Root has $UID 0.
E_WRONG_USER=65    # Not root?

E_NOSUCHUSER=70
SUCCESS=0


if [ "$UID" -ne "$ROOT_UID" ]
then
  echo; echo "Only root can run this script."; echo
  exit $E_WRONG_USER
else
  echo
  echo "You should know better than to run this script, root."
  echo "Even root users get the blues... "
  echo
fi  


username=bozo
NEWPASSWORD=security_violation

# Check if bozo lives here.
grep -q "$username" /etc/passwd
if [ $? -ne $SUCCESS ]
then
  echo "User $username does not exist."
  echo "No password changed."
  exit $E_NOSUCHUSER
fi  

echo "$NEWPASSWORD" | passwd --stdin "$username"
#  The '--stdin' option to 'passwd' permits
#+ getting a new password from stdin (or a pipe).

echo; echo "User $username's password changed!"

# Using the 'passwd' command in a script is dangerous.

exit 0
```

This runs on a made-up `passwd` file, with "root" being you (as in
chapter 2), and a tessh function standing in for the real `passwd`: a
function as a pipe stage gets what's piped in as its last argument.

```tessh
#!/usr/bin/env tessh
import os
# (A made-up /etc/passwd, and a stand-in for the passwd program, to try it on.)
print "root:x:0:0:root:/root:/bin/bash\nbozo:x:1000:100:Bozo:/home/bozo:/bin/bash" >passwd
def passwd *args
  # (The real passwd --stdin would set the user's password to its input.)
  print "passwd: (stand-in) new password for ${$args[-2]} is \"${$args[-1]}\""

#  setnew-password.tsh: For demonstration purposes only.
#                       Not a good idea to actually run this script.
#  This script must be run as root.

ROOT_UID = (os.getuid)   # (0, root, in the original: here, you)
E_WRONG_USER = 65        # Not root?

E_NOSUCHUSER = 70

if (os.getuid) != $ROOT_UID
  print ; print "Only root can run this script." ; print
  exit $E_WRONG_USER
else
  print
  print "You should know better than to run this script, root."
  print "Even root users get the blues... "
  print

username = bozo
NEWPASSWORD = security_violation

# Check if bozo lives here.
if not (grep -q $username passwd)      # (/etc/passwd in the original)
  print "User $username does not exist."
  print "No password changed."
  exit $E_NOSUCHUSER

$NEWPASSWORD | passwd --stdin $username
#  The '--stdin' option to 'passwd' permits
#+ getting a new password from stdin (or a pipe).

print ; print "User $username's password changed!"

exit 0
```

```output

You should know better than to run this script, root.
Even root users get the blues... 

passwd: (stand-in) new password for bozo is "security_violation"

User bozo's password changed!
```

The `-l`, `-u` and `-d` options to `passwd` lock, unlock and delete a
password (root only).

## Terminals
<!-- help: abs-sys-terminals tty stty setterm tset setserial getty mesg wall canonical raw -->

**tty** names the terminal, and **stty** shows and changes its settings:
which keys erase and interrupt, whether what's typed is echoed, whether
input comes a line at a time. A script uses it to read keys or passwords.
These need a real terminal, so none of the three examples can run here.
tessh runs `stty` as bash does; Python's `termios`, `tty` and `getpass`
modules do the same from inside the script.

### Example 17-2. Setting an erase character (`erase.sh`)

```bash no-run
#!/bin/bash
# erase.sh: Using "stty" to set an erase character when reading input.

echo -n "What is your name? "
read name                      #  Try to backspace
                               #+ to erase characters of input.
                               #  Problems?
echo "Your name is $name."

stty erase '#'                 #  Set "hashmark" (#) as erase character.
echo -n "What is your name? "
read name                      #  Use # to erase last character typed.
echo "Your name is $name."

exit 0

# Even after the script exits, the new key value remains set.
# Exercise: How would you reset the erase character to the default value?
```

```tessh no-run
#!/usr/bin/env tessh
# erase.tsh: Using "stty" to set an erase character when reading input.

name = input "What is your name? "   #  Try to backspace
                                     #+ to erase characters of input.
print "Your name is $name."

stty erase '#'                       #  Set "hashmark" (#) as erase character.
name = input "What is your name? "   #  Use # to erase last character typed.
print "Your name is $name."

exit 0

# Even after the script exits, the new key value remains set.
```

### Example 17-3. secret password: Turning off terminal echoing (`secret-pw.sh`)

```bash no-run
#!/bin/bash
# secret-pw.sh: secret password

echo
echo -n "Enter password "
read passwd
echo "password is $passwd"
echo -n "If someone had been looking over your shoulder, "
echo "your password would have been compromised."

echo && echo  # Two line-feeds in an "and list."


stty -echo    # Turns off screen echo.
#   May also be done with
#   read -sp passwd
#   A big Thank You to Leigh James for pointing this out.

echo -n "Enter password again "
read passwd
echo
echo "password is $passwd"
echo

stty echo     # Restores screen echo.

exit 0

# Do an 'info stty' for more on this useful-but-tricky command.
```

`stty -echo` works as in bash; `getpass.getpass "Enter password "` is the
Python way (bash's `read -sp`), and puts the echo back by itself.

```tessh no-run
#!/usr/bin/env tessh
# secret-pw.tsh: secret password

print
passwd = input "Enter password "
print "password is $passwd"
print "If someone had been looking over your shoulder, " end=""
print "your password would have been compromised."

print ; print

stty -echo    # Turns off screen echo.
#   May also be done with
#   passwd = getpass.getpass "Enter password again "

try
  passwd = input "Enter password again "
  print
  print "password is $passwd"
  print
finally
  stty echo   # Restores screen echo, even after Ctrl+C.

exit 0
```

`stty -icanon` hands the script each key as it's typed, without waiting for
Enter.

### Example 17-4. Keypress detection (`keypress.sh`)

```bash no-run
#!/bin/bash
# keypress.sh: Detect a user keypress ("hot keys").

echo

old_tty_settings=$(stty -g)   # Save old settings (why?).
stty -icanon
Keypress=$(head -c1)          # or $(dd bs=1 count=1 2> /dev/null)
                              # on non-GNU systems

echo
echo "Key pressed was \""$Keypress"\"."
echo

stty "$old_tty_settings"      # Restore old settings.

# Thanks, Stephane Chazelas.

exit 0
```

The same commands; `finally` puts the settings back however the script
ends.

```tessh no-run
#!/usr/bin/env tessh
# keypress.tsh: Detect a user keypress ("hot keys").

print

old_tty_settings = stty -g     # Save old settings (why?).
try
  stty -icanon
  Keypress = head -c1          # or (dd bs=1 count=1 2>/dev/null)
finally
  stty $old_tty_settings       # Restore old settings.

print
print "Key pressed was \"$Keypress\"."
print

exit 0
```

A terminal normally works in *canonical* mode: it keeps what's typed in a
line buffer, with its own small line editor (erase, kill a word, kill the
line), and hands the program the line on Enter. In *raw* (non-canonical)
mode every key goes to the program at once. bash's prompt, like tessh's
REPL, turns off canonical mode and echoing and does the editing itself.

**setterm** and **tset** change and show terminal attributes
(`setterm -cursor off`, `setterm -bold on`), **setserial** sets up serial
ports, **getty** and **agetty** set up a terminal for logins, **mesg**
allows or stops other users writing to your terminal, and **wall** writes
to all of them.

## Information and Statistics
<!-- help: abs-sys-info uname arch lsof strace nc free du df dmesg stat vmstat uptime hostname -->

**uname** and **arch** describe the system, **lastcomm** and **lastlog**
show past commands and logins, **lsof** lists open files, **strace** and
**ltrace** trace a program's system and library calls, **nc** (netcat)
connects to network ports, **free**, **vmstat** and **procinfo** show
memory, **du** and **df** disk use, **dmesg** the kernel's messages,
**uptime** the load, **hostname** the host's name (`socket.gethostname` in
Python, bash's `$HOSTNAME`), **sar** activity, and **readelf** and **size**
describe binaries. Their output is text for `grep` and `awk`, in tessh as
in bash; `free | grep Mem | awk '{ print $4 }'` is the same line in both.

### Example 17-5. Checking a remote server for identd (`iscan.sh`)

```bash no-run
#! /bin/sh
## Duplicate DaveG's ident-scan thingie using netcat. Oooh, he'll be p*ssed.
## Args: target port [port port port ...]
## Hose stdout _and_ stderr together.
##
##  Advantages: runs slower than ident-scan, giving remote inetd less cause
##+ for alarm, and only hits the few known daemon ports you specify.
##  Disadvantages: requires numeric-only port args, the output sleazitude,
##+ and won't work for r-services when coming from high source ports.
# Script author: Hobbit <hobbit@avian.org>
# Used in ABS Guide with permission.

# ---------------------------------------------------
E_BADARGS=65       # Need at least two args.
TWO_WINKS=2        # How long to sleep.
THREE_WINKS=3
IDPORT=113         # Authentication "tap ident" port.
RAND1=999
RAND2=31337
TIMEOUT0=9
TIMEOUT1=8
TIMEOUT2=4
# ---------------------------------------------------

case "${2}" in
  "" ) echo "Need HOST and at least one PORT." ; exit $E_BADARGS ;;
esac

# Ping 'em once and see if they *are* running identd.
nc -z -w $TIMEOUT0 "$1" $IDPORT || \
{ echo "Oops, $1 isn't running identd." ; exit 0 ; }
#  -z scans for listening daemons.
#     -w $TIMEOUT = How long to try to connect.

# Generate a randomish base port.
RP=`expr $$ % $RAND1 + $RAND2`

TRG="$1"
shift

while test "$1" ; do
  nc -v -w $TIMEOUT1 -p ${RP} "$TRG" ${1} < /dev/null > /dev/null &
  PROC=$!
  sleep $THREE_WINKS
  echo "${1},${RP}" | nc -w $TIMEOUT2 -r "$TRG" $IDPORT 2>&1
  sleep $TWO_WINKS

# Does this look like a lamer script or what . . . ?
# ABS Guide author comments: "Ain't really all that bad . . .
#+                            kinda clever, actually."

  kill -HUP $PROC
  RP=`expr ${RP} + 1`
  shift
done

exit $?

#  Notes:
#  -----

#  Try commenting out line 30 and running this script
#+ with "localhost.localdomain 25" as arguments.

#  For more of Hobbit's 'nc' example scripts,
#+ look in the documentation:
#+ the /usr/share/doc/nc-X.XX/scripts directory.
```

This needs the network, and a host to scan. The background `nc` has to be
killed later, so it's started with Python's `subprocess.Popen`, whose
`.send_signal` does `kill -HUP $PROC` (a tessh `&` job doesn't give its
process ID: see chapter 15's gap).

```tessh no-run
#!/usr/bin/env tessh
import os subprocess signal
## Duplicate DaveG's ident-scan thingie using netcat. Oooh, he'll be p*ssed.
## Args: target port [port port port ...]
## Hose stdout _and_ stderr together.
# Script author: Hobbit <hobbit@avian.org>

# ---------------------------------------------------
E_BADARGS = 65       # Need at least two args.
TWO_WINKS = 2        # How long to sleep.
THREE_WINKS = 3
IDPORT = 113         # Authentication "tap ident" port.
RAND1 = 999
RAND2 = 31337
TIMEOUT0 = 9
TIMEOUT1 = 8
TIMEOUT2 = 4
# ---------------------------------------------------

if (len $argv) < 3
  print "Need HOST and at least one PORT."
  exit $E_BADARGS
[TRG *ports] = $argv[1:]

# Ping 'em once and see if they *are* running identd.
if not (nc -z -w $TIMEOUT0 $TRG $IDPORT)
  print "Oops, $TRG isn't running identd."
  exit 0
#  -z scans for listening daemons.
#     -w $TIMEOUT = How long to try to connect.

# Generate a randomish base port.
RP = (os.getpid) % $RAND1 + $RAND2

for port in $ports
  PROC = subprocess.Popen [nc -v -w "$TIMEOUT1" -p "$RP" $TRG $port] stdin=$subprocess.DEVNULL stdout=$subprocess.DEVNULL
  sleep $THREE_WINKS
  "$port,$RP" | nc -w $TIMEOUT2 -r $TRG $IDPORT 2>&1
  sleep $TWO_WINKS
  $PROC.send_signal $signal.SIGHUP
  RP += 1

exit 0
```

Dr. Andrew Tridgell's one-line script from the BitKeeper affair is the same
line in tessh: `echo clone | nc thunk.org 5000 > e2fsprogs.dat`.

**stat** gives the details of a file, and with `-c` just the ones asked
for; a script can take them into variables. Python's `Path.stat` gives
them all at once as numbers, `stat.filemode` makes the `-rw-r--r--` form,
and `Path.owner` the owner's name.

```bash no-run
#!/bin/bash
# fileinfo2.sh

# Per suggestion of Joël Bourquard and . . .
# http://www.linuxquestions.org/questions/showthread.php?t=410766

FILENAME=testfile.txt
file_name=$(stat -c%n "$FILENAME")   # Same as "$FILENAME" of course.
file_owner=$(stat -c%U "$FILENAME")
file_size=$(stat -c%s "$FILENAME")
#  Certainly easier than using "ls -l $FILENAME"
#+ and then parsing with sed.
file_inode=$(stat -c%i "$FILENAME")
file_type=$(stat -c%F "$FILENAME")
file_access_rights=$(stat -c%A "$FILENAME")

echo "File name:          $file_name"
echo "File owner:         $file_owner"
echo "File size:          $file_size"
echo "File inode:         $file_inode"
echo "File type:          $file_type"
echo "File access rights: $file_access_rights"

exit 0
```

```tessh
#!/usr/bin/env tessh
import stat
# fileinfo2.tsh
FILENAME = ./testfile.txt
print "Some text to look at." >$FILENAME   # (A made-up file.)
chmod 664 $FILENAME

info = ($FILENAME.stat)                   # All the numbers at once.
file_name = $FILENAME.name
file_owner = ($FILENAME.owner)
file_size = $info.st_size
file_inode = $info.st_ino
file_type = if (stat.S_ISREG $info.st_mode) then "regular file" else "something else"
file_access_rights = stat.filemode $info.st_mode

print "File name:          $file_name"
print "File owner:         $file_owner"
print "File size:          $file_size"
print "File inode:         $file_inode"
print "File type:          $file_type"
print "File access rights: $file_access_rights"

exit 0
```

```text
(one run)
File name:          testfile.txt
File owner:         ben
File size:          22
File inode:         44481544
File type:          regular file
File access rights: -rw-rw-r--
```

## System Logs
<!-- help: abs-sys-logs logger logrotate syslog -->

**logger** adds a line to the system log; **logrotate** keeps the logs
from growing for ever. A script can log its progress with `logger`; bash's
`$LINENO` is Python's frame line in tessh. (Not run here: it writes to the
real system log.)

```bash no-run
logger -t $0 -i Logging at line "$LINENO".
# The "-t" option specifies the tag for the logger entry.
# The "-i" option records the process ID.
```

```tessh no-run
import inspect
logger -t (Path $argv[0]).name -i Logging at line ((inspect.currentframe).f_lineno).
```

(Python's `syslog` module writes to the log without a program.)

## Job Control
<!-- help: abs-sys-jobs ps pgrep pkill pstree top nice nohup pidof fuser cron -->

**ps** lists processes, **pgrep** and **pkill** find and signal them by
name, **pstree** and **top** show them as a tree and by CPU use, **nice**
runs a command at a lower priority, **nohup** keeps it running after you log
out, **pidof** gives a running program's process IDs, **fuser** the
processes using a file or port, and **cron** runs commands on a schedule.

### Example 17-6. pidof helps kill a process (`kill-process.sh`)

```bash
#!/bin/bash
# kill-process.sh

NOPROCESS=2

process=xxxyyyzzz  # Use nonexistent process.
# For demo purposes only...
# ... don't want to actually kill any actual process with this script.
#
# If, for example, you wanted to use this script to logoff the Internet,
#     process=pppd

t=`pidof $process`       # Find pid (process id) of $process.
# The pid is needed by 'kill' (can't 'kill' by program name).

if [ -z "$t" ]           # If process not present, 'pidof' returns null.
then
  echo "Process $process was not running."
  echo "Nothing killed."
  exit $NOPROCESS
fi  

kill $t                  # May need 'kill -9' for stubborn process.

# Need a check here to see if process allowed itself to be killed.
# Perhaps another " t=`pidof $process` " or ...


# This entire script could be replaced by
#        kill $(pidof -x process_name)
# or
#        killall process_name
# but it would not be as instructive.

exit 0
```

`pidof` as a value is its output, and false when it found nothing. `kill`
takes the list of IDs.

```tessh
#!/usr/bin/env tessh
# kill-process.tsh

NOPROCESS = 2

process = xxxyyyzzz   # Use nonexistent process.
# For demo purposes only...
# ... don't want to actually kill any actual process with this script.

t = pidof $process    # Find pid (process id) of $process.
# The pid is needed by 'kill' (can't 'kill' by program name).

if not $t             # If process not present, 'pidof' fails and prints nothing.
  print "Process $process was not running."
  print "Nothing killed."
  exit $NOPROCESS

kill ($t.split)       # May need 'kill -9' for stubborn process.

# This entire script could be replaced by
#        kill (pidof -x process_name)
# or
#        killall process_name
# but it would not be as instructive.

exit 0
```

```output
Process xxxyyyzzz was not running.
Nothing killed.
```

`fuser -um /dev/device` answers "device is busy" when unmounting: it names
the processes still using it, so they can be killed.

## Process Control and Booting
<!-- help: abs-sys-boot init telinit runlevel halt shutdown reboot service -->

**init** is the parent of all processes; **telinit** changes the runlevel
and **runlevel** shows it; **halt**, **shutdown** and **reboot** stop the
system; **service** starts and stops system services. (On a systemd system
these are all `systemctl`.) Nothing for a script to do differently in
tessh.

## Network
<!-- help: abs-sys-network nmap ifconfig netstat iwconfig ip route iptables chkconfig tcpdump -->

**nmap** scans ports, **ifconfig** and **ip** set up interfaces and
routes, **netstat** shows connections, **iwconfig** wireless,
**route** the routing table, **iptables** the firewall, **chkconfig**
boot-time services and **tcpdump** the packets themselves. A port check
with `nmap`:

```bash no-run
#!/bin/bash

SERVER=$HOST                           # localhost.localdomain (127.0.0.1).
PORT_NUMBER=25                         # SMTP port.

nmap $SERVER | grep -w "$PORT_NUMBER"  # Is that particular port open?
#              grep -w matches whole words only,
#+             so this wouldn't match port 1025, for example.

exit 0

# 25/tcp     open        smtp
```

```tessh no-run
#!/usr/bin/env tessh
SERVER = $env.get "HOST" localhost     # localhost.localdomain (127.0.0.1).
PORT_NUMBER = 25                       # SMTP port.

nmap $SERVER | grep -w $PORT_NUMBER    # Is that particular port open?

exit 0
```

(Python's `socket.create_connection` tests one port without `nmap`.) The
network start-up script's loop over the interfaces is plain shell, the
same commands in tessh:

```bash no-run
for i in $interfaces ; do
  if ifconfig $i 2>/dev/null | grep -q "UP" >/dev/null 2>&1 ; then
    action "Shutting down interface $i: " ./ifdown $i boot
  fi
done

echo "Currently active devices:"
echo `/sbin/ifconfig | grep ^[a-z] | awk '{print $1}'`
```

```tessh no-run
for i in $interfaces
  if ifconfig $i 2>/dev/null | grep -q "UP"
    action "Shutting down interface $i: " ./ifdown $i boot

print "Currently active devices:"
print (" ".join (/sbin/ifconfig | grep '^[a-z]' | awk '{print $1}').lines)
```

The `^[a-z]` is quoted: unquoted it's a glob, in tessh as in bash.

A GRE tunnel set up with `ip` (by Juan Nicolas Ruiz) is a list of `ip`
commands, the same in tessh:

```bash no-run
#!/bin/bash
# Script by Juan Nicolas Ruiz
# Used with his kind permission.

# Setting up (and stopping) a GRE tunnel.


# --- start-tunnel.sh ---

LOCAL_IP="192.168.1.17"
REMOTE_IP="10.0.5.33"
OTHER_IFACE="192.168.0.100"
REMOTE_NET="192.168.3.0/24"

/sbin/ip tunnel add netb mode gre remote $REMOTE_IP \
  local $LOCAL_IP ttl 255
/sbin/ip addr add $OTHER_IFACE dev netb
/sbin/ip link set netb up
/sbin/ip route add $REMOTE_NET dev netb

exit 0  #############################################

# --- stop-tunnel.sh ---

REMOTE_NET="192.168.3.0/24"

/sbin/ip route del $REMOTE_NET dev netb
/sbin/ip link set netb down
/sbin/ip tunnel del netb

exit 0
```

## Filesystem
<!-- help: abs-sys-filesystem mount umount sync losetup mkswap swapon mke2fs fsck badblocks lsusb lspci chroot lockfile flock mknod tmpwatch -->

**mount** and **umount** attach and detach filesystems (`/etc/fstab`
lists them, `/etc/mtab` the mounted ones); `mount -o loop` mounts a file,
such as a CD image, as if it were a device.

### Example 17-7. Checking a CD image

```bash no-run
# As root...

mkdir /mnt/cdtest  # Prepare a mount point, if not already there.

mount -r -t iso9660 -o loop cd-image.iso /mnt/cdtest   # Mount the image.
#                  "-o loop" option equivalent to "losetup /dev/loop0"
cd /mnt/cdtest     # Now, check the image.
ls -alR            # List the files in the directory tree there.
                   # And so forth.
```

It needs root, and a CD image. The tessh is the same commands.

```tessh no-run
# As root...

mkdir -p /mnt/cdtest   # Prepare a mount point, if not already there.

mount -r -t iso9660 -o loop cd-image.iso /mnt/cdtest   # Mount the image.
cd /mnt/cdtest         # Now, check the image.
ls -alR                # List the files in the directory tree there.
```

**sync** writes out what's still in memory to the disks. **losetup** sets
up loop devices, which make a file act as a disk:

### Example 17-8. Creating a filesystem in a file

```bash no-run
SIZE=1000000  # 1 meg

head -c $SIZE < /dev/zero > file  # Set up file of designated size.
losetup /dev/loop0 file           # Set it up as loopback device.
mke2fs /dev/loop0                 # Create filesystem.
mount -o loop /dev/loop0 /mnt     # Mount it.

# Thanks, S.C.
```

Root again, for `losetup` and `mount`.

```tessh no-run
SIZE = 1000000   # 1 meg

head -c $SIZE </dev/zero >file    # Set up file of designated size.
losetup /dev/loop0 file           # Set it up as loopback device.
mke2fs /dev/loop0                 # Create filesystem.
mount -o loop /dev/loop0 /mnt     # Mount it.
```

**mkswap**, **swapon** and **swapoff** make and use swap space;
**mke2fs** makes an ext2 filesystem, **tune2fs**, **dumpe2fs** and
**hdparm** adjust and show filesystems and disks, **fdisk** partitions a
disk, **fsck**, **e2fsck** and **debugfs** check and repair, and
**badblocks** looks for flaws. All need root, and all can destroy data.

### Example 17-9. Adding a new hard drive (`add-drive.sh`)

```bash no-run
#!/bin/bash

# Adding a second hard drive to system.
# Software configuration. Assumes hardware already mounted.
# From an article by the author of the ABS Guide.
# In issue #38 of _Linux Gazette_, http://www.linuxgazette.com.

ROOT_UID=0     # This script must be run as root.
E_NOTROOT=67   # Non-root exit error.

if [ "$UID" -ne "$ROOT_UID" ]
then
  echo "Must be root to run this script."
  exit $E_NOTROOT
fi  

# Use with extreme caution!
# If something goes wrong, you may wipe out your current filesystem.


NEWDISK=/dev/hdb         # Assumes /dev/hdb vacant. Check!
MOUNTPOINT=/mnt/newdisk  # Or choose another mount point.


fdisk $NEWDISK
mke2fs -cv $NEWDISK1   # Check for bad blocks (verbose output).
#  Note:           ^     /dev/hdb1, *not* /dev/hdb!
mkdir $MOUNTPOINT
chmod 777 $MOUNTPOINT  # Makes new drive accessible to all users.


# Now, test ...
# mount -t ext2 /dev/hdb1 /mnt/newdisk
# Try creating a directory.
# If it works, umount it, and proceed.

# Final step:
# Add the following line to /etc/fstab.
# /dev/hdb1  /mnt/newdisk  ext2  defaults  1 1

exit
```

Not to be run anywhere but on a new disk, as root. Note the original's
`$NEWDISK1`: bash reads that as a variable named `NEWDISK1`, which is empty,
not `/dev/hdb1`. `${NEWDISK}1` (in bash or tessh) is what it means.

```tessh no-run
#!/usr/bin/env tessh
import os

# Adding a second hard drive to system.
# Software configuration. Assumes hardware already mounted.

ROOT_UID = 0     # This script must be run as root.
E_NOTROOT = 67   # Non-root exit error.

if (os.getuid) != $ROOT_UID
  print "Must be root to run this script."
  exit $E_NOTROOT

# Use with extreme caution!
# If something goes wrong, you may wipe out your current filesystem.

NEWDISK = /dev/hdb         # Assumes /dev/hdb vacant. Check!
MOUNTPOINT = /mnt/newdisk  # Or choose another mount point.

fdisk $NEWDISK
mke2fs -cv ${NEWDISK}1     # Check for bad blocks (verbose output).
mkdir $MOUNTPOINT
chmod 777 $MOUNTPOINT      # Makes new drive accessible to all users.

# Now, test ...
# mount -t ext2 /dev/hdb1 /mnt/newdisk
# Try creating a directory.
# If it works, umount it, and proceed.

# Final step:
# Add the following line to /etc/fstab.
# /dev/hdb1  /mnt/newdisk  ext2  defaults  1 1

exit
```

**mkdosfs** makes a FAT filesystem; **lsusb**, **usbmodules** and
**lspci** list devices; **mkbootdisk** and **mkisofs** make boot disks and
CD images; **chroot** runs a command with another folder as `/`.

**lockfile** makes a lock file, a flag that a file or device is in use;
others check for it before going on. A test for one:

```bash
appname=xyzip
# Application "xyzip" created lock file "/var/lock/xyzip.lock".

if [ -e "/var/lock/$appname.lock" ]
then   #+ Prevent other programs & scripts
       #  from accessing files/resources used by xyzip.
  echo "$appname is running."
fi
```

```tessh
appname = xyzip
# Application "xyzip" created lock file "/var/lock/xyzip.lock".

if test -e /var/lock/${appname}.lock   # (or ((Path "/var/lock/${appname}.lock").exists))
  # Prevent other programs & scripts
  # from accessing files/resources used by xyzip.
  print "$appname is running."
```

```output
```

(The ABS Guide's `...` in the `then` branch is an `echo` here, so it runs.
`"$appname.lock"` in tessh would read the attribute `.lock` of `$appname`:
`${appname}.lock` is the name followed by `.lock`.)

**flock** runs a command while holding a lock on a file: `flock $0 cat $0 >
lockfile__$0`. Python's `fcntl.flock` takes the lock from inside the
script, in a `with` block. **mknod** and **MAKEDEV** make device files, and
**tmpwatch** deletes files nobody has used for a while.

## Backup
<!-- help: abs-sys-backup dump restore fdformat -->

**dump** and **restore** back up and restore whole filesystems;
**fdformat** formats a floppy. Most people use `tar` or `rsync` instead.

## System Resources
<!-- help: abs-sys-resources ulimit quota setquota umask rdev fork-bomb -->

**ulimit** sets limits on what a process may use: file size (`-f`), core
dumps (`-c`), number of processes (`-u`). A limit on processes stops a
fork bomb, a script that starts copies of itself until the system freezes
(the ABS Guide shows one; it isn't repeated here). tessh has no `ulimit`;
Python's `resource` module does the same for the script and the programs
it runs.

```tessh no-run
import resource
resource.setrlimit $resource.RLIMIT_NPROC (200, 200)   # ulimit -u 200
resource.setrlimit $resource.RLIMIT_CORE (0, 0)        # ulimit -c 0: no core dumps
```

**quota** and **setquota** show and set disk quotas. **rdev** sets up the
root device.

**umask** sets which permissions new files *don't* get: with `umask 022`
they have at most `755`. In tessh it's `os.umask` (and the programs the
script runs inherit it).

### Example 17-10. Using umask to hide an output file from prying eyes (`rot13a.sh`)

```bash no-run
#!/bin/bash
# rot13a.sh: Same as "rot13.sh" script, but writes output to "secure" file.

# Usage: ./rot13a.sh filename
# or     ./rot13a.sh <filename
# or     ./rot13a.sh and supply keyboard input (stdin)

umask 177               #  File creation mask.
                        #  Files created by this script
                        #+ will have 600 permissions.

OUTFILE=decrypted.txt   #  Results output to file "decrypted.txt"
                        #+ which can only be read/written
                        #  by invoker of script (or root).

cat "$@" | tr 'a-zA-Z' 'n-za-mN-ZA-M' > $OUTFILE 
#    ^^ Input from stdin or a file.   ^^^^^^^^^^ Output redirected to file. 

exit 0
```

```tessh
#!/usr/bin/env tessh
import os
# (A made-up file to scramble.)
print "Meet me at the old oak tree." >secret.txt

# rot13a.tsh: rot13, with the output in a "secure" file.

# Usage: rot13a.tsh filename …

os.umask 0o177          #  File creation mask.
                        #  Files created by this script
                        #+ will have 600 permissions.

OUTFILE = decrypted.txt #  Results output to file "decrypted.txt"
                        #+ which can only be read/written
                        #  by invoker of script (or root).

args = $argv[1:] or [secret.txt]   # (sample arguments, when none are given)

cat $args | tr 'a-zA-Z' 'n-za-mN-ZA-M' >$OUTFILE

stat -c "%A %n" $OUTFILE   # (To see what it did.)
cat $OUTFILE

exit 0
```

```output
-rw------- decrypted.txt
Zrrg zr ng gur byq bnx gerr.
```

## Modules
<!-- help: abs-sys-modules lsmod insmod rmmod modprobe depmod modinfo -->

**lsmod** lists the kernel's modules (as `/proc/modules` does),
**insmod**, **rmmod** and **modprobe** load and unload them, **depmod**
works out their dependencies and **modinfo** describes one.

## Miscellaneous
<!-- help: abs-sys-misc env ldd watch strip nm xrandr rdist -->

**env** runs a program with variables set or changed, and with none lists
the environment. `name=value command` does the same for one command, in
tessh as in bash; and `#!/usr/bin/env tessh` finds tessh wherever it is.

```bash
var1=value1 var2=value2 sh -c 'echo $var1 $var2'
# $var1 and $var2 set in the environment of this command only.
echo "[$var1]"
```

```tessh
var1=value1 var2=value2 sh -c 'echo $var1 $var2'
# $var1 and $var2 set in the environment of this command only.
print "[$($env.get "var1" "")]"
```

```output
value1 value2
[]
```

**ldd** lists a program's shared libraries; **watch** runs a command every
few seconds (`watch -n 5 tail /var/log/messages`); **strip** and **nm**
remove and list a binary's symbols; **rdist** copies files to other
machines; **xrandr** sets up the screen, its brightness included.

### Example 17-11. Backlight: changes the brightness of the (laptop) screen backlight (`backlight.sh`)

```bash no-run
#!/bin/bash
# backlight.sh
# reldate 02dec2011

#  A bug in Fedora Core 16/17 messes up the keyboard backlight controls.
#  This script is a quick-n-dirty workaround, essentially a shell wrapper
#+ for xrandr. It gives more control than on-screen sliders and widgets.

OUTPUT=$(xrandr | grep LV | awk '{print $1}')   # Get display name!
INCR=.05      # For finer-grained control, set INCR to .03 or .02.

old_brightness=$(xrandr --verbose | grep rightness | awk '{ print $2 }')


if [ -z "$1" ]
then
  bright=1    # If no command-line arg, set brightness to 1.0 (default).

  else
    if [ "$1" = "+" ]
    then
      bright=$(echo "scale=2; $old_brightness + $INCR" | bc)   # +.05

  else
    if [ "$1" = "-" ]
    then
      bright=$(echo "scale=2; $old_brightness - $INCR" | bc)   # -.05

  else
    if [ "$1" = "#" ]   # Echoes current brightness; does not change it.
    then
      bright=$old_brightness

  else
    if [[ "$1" = "h" || "$1" = "H" ]]
    then
      echo
      echo "Usage:"
      echo "$0 [No args]    Sets/resets brightness to default (1.0)."
      echo "$0 +            Increments brightness by 0.5."
      echo "$0 -            Decrements brightness by 0.5."
      echo "$0 #            Echoes current brightness without changing it."
      echo "$0 N (number)   Sets brightness to N (useful range .7 - 1.2)."
      echo "$0 h [H]        Echoes this help message."
      echo "$0 any-other    Gives xrandr usage message."

      bright=$old_brightness

  else
    bright="$1"

      fi
     fi
    fi
  fi
fi


xrandr --output "$OUTPUT" --brightness "$bright"   # See xrandr manpage.
                                                   # As root!
E_CHANGE0=$?
echo "Current brightness = $bright"

exit $E_CHANGE0


# =========== Or, alternately . . . ==================== #

#!/bin/bash
# backlight2.sh
# reldate 20jun2012

#  A bug in Fedora Core 16/17 messes up the keyboard backlight controls.
#  This is a quick-n-dirty workaround, an alternate to backlight.sh.

target_dir=\
/sys/devices/pci0000:00/0000:00:01.0/0000:01:00.0/backlight/acpi_video0
# Hardware directory.

actual_brightness=$(cat $target_dir/actual_brightness)
max_brightness=$(cat $target_dir/max_brightness)
Brightness=$target_dir/brightness

let "req_brightness = actual_brightness"   # Requested brightness.

if [ "$1" = "-" ]
then     # Decrement brightness 1 notch.
  let "req_brightness = $actual_brightness - 1"
else
  if [ "$1" = "+" ]
  then   # Increment brightness 1 notch.
    let "req_brightness = $actual_brightness + 1"
   fi
fi

if [ $req_brightness -gt $max_brightness ]
then
  req_brightness=$max_brightness
fi   # Do not exceed max. hardware design brightness.

echo

echo "Old brightness = $actual_brightness"
echo "Max brightness = $max_brightness"
echo "Requested brightness = $req_brightness"
echo

# =====================================
echo $req_brightness > $Brightness
# Must be root for this to take effect.
E_CHANGE1=$?   # Successful?
# =====================================

if [ "$?" -eq 0 ]
then
  echo "Changed brightness!"
else
  echo "Failed to change brightness!"
fi

act_brightness=$(cat $Brightness)
echo "Actual brightness = $act_brightness"

scale0=2
sf=100 # Scale factor.
pct=$(echo "scale=$scale0; $act_brightness / $max_brightness * $sf" | bc)
echo "Percentage brightness = $pct%"

exit $E_CHANGE1
```

The first script needs X and a laptop screen. Its nest of `if`s is a
`match` in tessh, and `bc`'s arithmetic is Python's.

```tessh no-run
#!/usr/bin/env tessh
# backlight.tsh

OUTPUT = xrandr | grep LV | awk '{print $1}'   # Get display name!
INCR = 0.05      # For finer-grained control, set INCR to .03 or .02.

old_brightness = float (xrandr --verbose | grep rightness | awk '{ print $2 }')
me = $argv[0]

match $argv[1:]
  case []
    bright = 1   # If no command-line arg, set brightness to 1.0 (default).
  case ["+"]
    bright = round ($old_brightness + $INCR) 2   # +.05
  case ["-"]
    bright = round ($old_brightness - $INCR) 2   # -.05
  case ["#"]     # Prints current brightness; does not change it.
    bright = $old_brightness
  case ["h"] | ["H"]
    print
    print "Usage:"
    print "$me [No args]    Sets/resets brightness to default (1.0)."
    print "$me +            Increments brightness by 0.5."
    print "$me -            Decrements brightness by 0.5."
    print "$me #            Prints current brightness without changing it."
    print "$me N (number)   Sets brightness to N (useful range .7 - 1.2)."
    print "$me h [H]        Prints this help message."
    print "$me any-other    Gives xrandr usage message."
    bright = $old_brightness
  case [$arg *_]
    bright = $arg

xrandr --output $OUTPUT --brightness $bright   # See xrandr manpage.
                                               # As root!
E_CHANGE0 = $?
print "Current brightness = $bright"

exit $E_CHANGE0
```

The second (`backlight2.sh`, in the same file) writes to the backlight's
folder in `/sys`. It runs here on a made-up folder with the same three
files.

```tessh
# (A made-up backlight folder to try it on.)
mkdir acpi_video0
print 7 >acpi_video0/actual_brightness
print 10 >acpi_video0/max_brightness
print 7 >acpi_video0/brightness

# backlight2.tsh

target_dir = ./acpi_video0   # (/sys/devices/pci0000:00/…/backlight/acpi_video0 in the original)
# Hardware directory.

actual_brightness = int (cat $target_dir/actual_brightness)
max_brightness = int (cat $target_dir/max_brightness)
Brightness = $target_dir / brightness

args = $argv[1:] or ["+"]   # (sample arguments, when none are given)

req_brightness = $actual_brightness   # Requested brightness.

if $args[0] == "-"        # Decrement brightness 1 notch.
  req_brightness = $actual_brightness - 1
elif $args[0] == "+"      # Increment brightness 1 notch.
  req_brightness = $actual_brightness + 1

req_brightness = min $req_brightness $max_brightness
# Do not exceed max. hardware design brightness.

print

print "Old brightness = $actual_brightness"
print "Max brightness = $max_brightness"
print "Requested brightness = $req_brightness"
print

# =====================================
E_CHANGE1 = try
  print $req_brightness >$Brightness
  # Must be root for this to take effect.
  print "Changed brightness!"
  0
except OSError
  print "Failed to change brightness!"
  1
# =====================================

act_brightness = int (cat $Brightness)
print "Actual brightness = $act_brightness"

pct = $act_brightness / $max_brightness * 100
print "Percentage brightness = ${pct:.2f}%"

exit $E_CHANGE1
```

```output

Old brightness = 7
Max brightness = 10
Requested brightness = 8

Changed brightness!
Actual brightness = 8
Percentage brightness = 80.00%
```

## 17.1 Analyzing a System Script
<!-- help: abs-sysscripts rc-scripts init.d killall-script analyzing -->

The `killall` script from `/etc/rc.d/init.d` (not the `killall` program)
stops the services still running at shut-down: each one has a lock file in
`/var/lock/subsys`, named after its start-up script.

### Example 17-12. killall, from /etc/rc.d/init.d (`ex55.sh`)

```bash no-run
#!/bin/sh

# --> Comments added by the author of this document marked by "# -->".

# --> This is part of the 'rc' script package
# --> by Miquel van Smoorenburg, <miquels@drinkel.nl.mugnet.org>.

# --> This particular script seems to be Red Hat / FC specific
# --> (may not be present in other distributions).

#  Bring down all unneeded services that are still running
#+ (there shouldn't be any, so this is just a sanity check)

for i in /var/lock/subsys/*; do
        # --> Standard for/in loop, but since "do" is on same line,
        # --> it is necessary to add ";".
        # Check if the script is there.
        [ ! -f $i ] && continue
        # --> This is a clever use of an "and list", equivalent to:
        # --> if [ ! -f "$i" ]; then continue

        # Get the subsystem name.
        subsys=${i#/var/lock/subsys/}
        # --> Match variable name, which, in this case, is the file name.
        # --> This is the exact equivalent of subsys=`basename $i`.
	
        # -->  It gets it from the lock file name
        # -->+ (if there is a lock file,
        # -->+ that's proof the process has been running).
        # -->  See the "lockfile" entry, above.


        # Bring the subsystem down.
        if [ -f /etc/rc.d/init.d/$subsys.init ]; then
           /etc/rc.d/init.d/$subsys.init stop
        else
           /etc/rc.d/init.d/$subsys stop
        # -->  Suspend running jobs and daemons.
        # -->  Note that "stop" is a positional parameter,
        # -->+ not a shell builtin.
        fi
done
```

Here the lock files and start-up scripts are made up, under the current
folder. A glob is a list of `Path`s, so `.name` gives the subsystem's name
(bash's `${i#/var/lock/subsys/}`), and `test -f` is bash's `[ -f … ]`. A
word with a `/` in it, like `./etc/rc.d/init.d/$subsys`, runs that program.

```tessh
#!/usr/bin/env tessh
# (Made-up lock files and start-up scripts to try it on.)
mkdir -p var/lock/subsys/olddir etc/rc.d/init.d
touch var/lock/subsys/atd var/lock/subsys/network var/lock/subsys/sshd
for s in [atd network sshd.init]
  cat >etc/rc.d/init.d/$s <<'END'
#!/bin/sh
echo "${0##*/}: $1"
END
  chmod +x etc/rc.d/init.d/$s

#  Bring down all unneeded services that are still running
#+ (there shouldn't be any, so this is just a sanity check)

for i in var/lock/subsys/*      # (/var/lock/subsys/* in the original)
  # Check if the script is there.
  if not ($i.is_file)
    continue

  # Get the subsystem name.
  subsys = $i.name
  # --> It gets it from the lock file name
  # -->+ (if there is a lock file,
  # -->+ that's proof the process has been running).

  # Bring the subsystem down.
  if test -f etc/rc.d/init.d/${subsys}.init
    ./etc/rc.d/init.d/${subsys}.init stop
  else
    ./etc/rc.d/init.d/$subsys stop
  # -->  Note that "stop" is a positional parameter,
  # -->+ not a shell builtin.
```

```output
atd: stop
network: stop
sshd.init: stop
```

Exercises: analyze `/etc/rc.d/init.d/halt` the same way (a copy, with its
commands turned into `echo`s, run as an ordinary user, or checked with
`sh -vn`); then some of the longer scripts there. A tessh port can be
checked without running it, with `tessh --check`.
