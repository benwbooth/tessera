# 29. /dev and /proc
<!-- help: abs-devproc abs-29 /dev /proc device-files pseudo-filesystem -->

A Linux system has two special folders: `/dev`, whose entries stand for
devices, and `/proc`, a window into the running kernel and its processes.
They are files as far as any program is concerned, so tessh reads and writes
them as bash does: with programs and redirections, or with Python's `open`
and `Path`.

## 29.1 /dev
<!-- help: abs-devref /dev device loopback dev-tcp socket -->

`/dev` holds an entry for each physical device that may be present (the
disk partitions `df` lists, `/dev/sda1`…), loopback devices (`/dev/loop0`,
which let an ordinary file be mounted as a filesystem), and virtual devices
that exist only in software: `/dev/null`, `/dev/zero`, `/dev/urandom`
(chapter 31). A block device reads and writes in chunks (disks, flash
drives), a character device a character at a time (keyboards, sound cards).
Mounting a USB stick, or checking for a disc in the drive with
`head -1 /dev/hdc`, is the same command in tessh as in bash.

Bash also has `/dev/tcp/HOST/PORT` and `/dev/udp/HOST/PORT`: not real files,
but names that bash itself turns into a network connection when you
redirect to them.

```bash no-run
bash$ cat </dev/tcp/time.nist.gov/13
53082 04-03-18 04:26:54 68 0 0 502.3 UTC(NIST) *
```

```bash no-run
#!/bin/bash
# This script must run with root permissions.

URL="time.nist.gov/13"

Time=$(cat </dev/tcp/"$URL")
UTC=$(echo "$Time" | awk '{print$3}')   # Third field is UTC (GMT) time.
# Exercise: modify this for different time zones.

echo "UTC Time = "$UTC""
```

**Gap:** tessh has no `/dev/tcp` or `/dev/udp`: a redirection to one is
an ordinary file that isn't there. Python's `socket` module does the same
job, and says more about what went wrong when it fails.

```tessh no-run
#!/usr/bin/env tessh
import socket

with (socket.create_connection ("time.nist.gov", 13) timeout=10) as s
  Time = (($s.makefile).read)
UTC = ($Time.split)[2]                  # Third field is UTC (GMT) time.

print "UTC Time = $UTC"
```

Downloading a page by hand over `/dev/tcp` (`exec 5<>/dev/tcp/www.net.cn/80`,
then writing the request to `&5` and reading it back) is
`urllib.request.urlopen` in tessh, or `curl` as a program.

### Example 29-1. Using /dev/tcp for troubleshooting (`dev-tcp.sh`)

```bash no-run
#!/bin/bash
# dev-tcp.sh: /dev/tcp redirection to check Internet connection.

# Script by Troy Engel.
# Used with permission.
 
TCP_HOST=news-15.net       # A known spam-friendly ISP.
TCP_PORT=80                # Port 80 is http.
  
# Try to connect. (Somewhat similar to a 'ping' . . .) 
echo "HEAD / HTTP/1.0" >/dev/tcp/${TCP_HOST}/${TCP_PORT}
MYEXIT=$?

: <<EXPLANATION
If bash was compiled with --enable-net-redirections, it has the capability of
using a special character device for both TCP and UDP redirections. These
redirections are used identically as STDIN/STDOUT/STDERR. The device entries
are 30,36 for /dev/tcp:

  mknod /dev/tcp c 30 36

>From the bash reference:
/dev/tcp/host/port
    If host is a valid hostname or Internet address, and port is an integer
port number or service name, Bash attempts to open a TCP connection to the
corresponding socket.
EXPLANATION

   
if [ "X$MYEXIT" = "X0" ]; then
  echo "Connection successful. Exit code: $MYEXIT"
else
  echo "Connection unsuccessful. Exit code: $MYEXIT"
fi

exit $MYEXIT
```

With `socket.create_connection` in a `try`, a failed connection is an
`OSError` whose message says why (refused, timed out, no such host), not
just a status of 1. So it can run here, this version first opens a listener
of its own on this machine and connects to that.

```tessh
#!/usr/bin/env tessh
import socket

# (A listener of our own on this machine to try it on; the original
#  connects to news-15.net, port 80.)
server = socket.create_server ("127.0.0.1", 0)

TCP_HOST = "127.0.0.1"                    # news-15.net in the original.
TCP_PORT = ($server.getsockname)[1]       # 80, http, in the original.

# Try to connect. (Somewhat similar to a 'ping' . . .)
MYEXIT = try
  with (socket.create_connection ($TCP_HOST, $TCP_PORT) timeout=10) as s
    $s.sendall b"HEAD / HTTP/1.0\r\n\r\n"
  0
except OSError as e
  print "($e)"
  1

if $MYEXIT == 0
  print "Connection successful. Exit code: $MYEXIT"
else
  print "Connection unsuccessful. Exit code: $MYEXIT"

exit $MYEXIT
```

```output
Connection successful. Exit code: 0
```

### Example 29-2. Playing music (`music.sh`)

```bash no-run
#!/bin/bash
# music.sh

# Music without external files

# Author: Antonio Macchi
# Used in ABS Guide with permission.


#  /dev/dsp default = 8000 frames per second, 8 bits per frame (1 byte),
#+ 1 channel (mono)

duration=2000       # If 8000 bytes = 1 second, then 2000 = 1/4 second.
volume=$'\xc0'      # Max volume = \xff (or \x00).
mute=$'\x80'        # No volume = \x80 (the middle).

function mknote ()  # $1=Note Hz in bytes (e.g. A = 440Hz ::
{                   #+ 8000 fps / 440 = 16 :: A = 16 bytes per second)
  for t in `seq 0 $duration`
  do
    test $(( $t % $1 )) = 0 && echo -n $volume || echo -n $mute
  done
}

e=`mknote 49`
g=`mknote 41`
a=`mknote 36`
b=`mknote 32`
c=`mknote 30`
cis=`mknote 29`
d=`mknote 27`
e2=`mknote 24`
n=`mknote 32767`
# European notation.

echo -n "$g$e2$d$c$d$c$a$g$n$g$e$n$g$e2$d$c$c$b$c$cis$n$cis$d \
$n$g$e2$d$c$d$c$a$g$n$g$e$n$g$a$d$c$b$a$b$c" > /dev/dsp
# dsp = Digital Signal Processor

exit      # A "bonny" example of an elegant shell script!
```

The samples are `bytes`, which Python builds and joins directly, with no
detour through `echo -n` and command substitution. A note is a list of
byte values made by a comprehension, and the tune is a list of notes. The
old OSS `/dev/dsp` is gone from most systems, so this writes the raw sound
to a file; `aplay -r 8000 -f U8 music.raw` (or `pacat --format=u8
--rate=8000 --channels=1`) plays it.

```tessh
#!/usr/bin/env tessh
# music: music without external files.
# Author: Antonio Macchi

#  8000 frames per second, 8 bits per frame (1 byte), 1 channel (mono)

duration = 2000     # If 8000 bytes = 1 second, then 2000 = 1/4 second.
volume = 0xc0       # Max volume = 0xff (or 0x00).
mute = 0x80         # No volume = 0x80 (the middle).

def mknote hz       # hz = the note in bytes (A = 440Hz ::
                    #  8000 fps / 440 = 16 :: A = 16 bytes per second)
  bytes [(if $t % $hz == 0 then $volume else $mute) for t in (range ($duration + 1))]

e = mknote 49
g = mknote 41
a = mknote 36
b = mknote 32
c = mknote 30
cis = mknote 29
d = mknote 27
e2 = mknote 24
n = mknote 32767
# European notation.

tune = [$g $e2 $d $c $d $c $a $g $n $g $e $n $g $e2 $d $c $c $b $c $cis $n $cis $d
        $n $g $e2 $d $c $d $c $a $g $n $g $e $n $g $a $d $c $b $a $b $c]

with (open music.raw wb) as dsp    # (/dev/dsp in the original)
  $dsp.write (b"".join $tune)

print (len $tune) notes
wc -c music.raw
```

```output
44 notes
88044 music.raw
```

## 29.2 /proc
<!-- help: abs-procref /proc proc pseudo-filesystem cpuinfo meminfo cmdline pid -->

`/proc` is a pseudo-filesystem: its files describe the running system and
its processes, made up by the kernel when you read them. `cat
/proc/devices`, `/proc/interrupts`, `/proc/partitions`, `/proc/loadavg`,
`fgrep Mem /proc/meminfo` and the rest work the same from tessh. Programs
such as `free`, `vmstat` and `uptime` read them too.

Scripts pick values out of them. The ABS Guide's snippets:

```bash no-run
FS=iso                       # ISO filesystem support in kernel?
grep $FS /proc/filesystems   # iso9660

kernel_version=$( awk '{ print $3 }' /proc/version )

CPU=$( awk '/model name/ {print $5}' < /proc/cpuinfo )

cpu_speed=$( fgrep "cpu MHz" /proc/cpuinfo | awk '{print $4}' )
#  Current operating speed (in MHz) of the cpu on your machine.
```

In tessh the same programs work, or the text can be taken apart in Python:
`.split` for fields, a regex for a line.

```tessh
FS = iso                     # ISO filesystem support in kernel?
grep $FS /proc/filesystems   # iso9660

kernel_version = awk '{ print $3 }' /proc/version

cpuinfo = ((Path /proc/cpuinfo).read_text)
CPU = ($cpuinfo =~ re'^model name\s*:\s*(.*)$'m).group 1
cpu_speed = ($cpuinfo =~ re'^cpu MHz\s*:\s*(\S+)'m).group 1
#  Current operating speed (in MHz) of the cpu on your machine.

print kernel $kernel_version
print cpu $CPU at $cpu_speed MHz
```

```text
(one run, with no iso9660 module loaded)
kernel 7.3.5
cpu AMD Ryzen 9 7900X3D 12-Core Processor at 3599.192 MHz
```

Each running process has a folder named by its process ID: `status` and
`stat` keep its statistics, `cmdline` holds its arguments separated by NUL
bytes, and `exe` is a link to the program it runs.

```bash no-run
#!/bin/bash
# get-commandline.sh
# Get the command-line parameters of a process.

OPTION=cmdline

# Identify PID.
pid=$( echo $(pidof "$1") | awk '{ print $1 }' )
# Get only first            ^^^^^^^^^^^^^^^^^^ of multiple instances.

echo
echo "Process ID of (first instance of) "$1" = $pid"
echo -n "Command-line arguments: "
cat /proc/"$pid"/"$OPTION" | xargs -0 echo
#   Formats output:        ^^^^^^^^^^^^^^^
#   (Thanks, Han Holl, for the fixup!)

echo; echo


# For example:
# sh get-commandline.sh xterm
```

Python splits the NUL-separated arguments itself, which is what `xargs -0`
was for. Here it looks at a small script it starts in the background.

```tessh
#!/usr/bin/env tessh
# get-commandline: get the command-line parameters of a process.
OPTION = cmdline

# (Something to look at: a little script, run in the background.)
print "#!/bin/sh\nsleep 2" >napper ; chmod +x napper
./napper one two &
sleep 0.3

name = if (len $argv) > 1 then $argv[1] else napper

# Identify PID: only the first of several instances.
# (-x finds scripts too, by their own name.)
pid = ((pidof -x $name).split)[0]

print
print "Process ID of (first instance of) $name = (a number)"
args = ((Path "/proc/$pid/$OPTION").read_bytes).split b"\0"
words = [($a.decode) for a in $args if $a]
print "Command-line arguments:" *$words
print
```

```output

Process ID of (first instance of) napper = (a number)
Command-line arguments: /bin/sh ./napper one two

```

Another reads the USB bus speed:

```bash no-run
devfile="/proc/bus/usb/devices"
text="Spd"
USB1="Spd=12"
USB2="Spd=480"


bus_speed=$(fgrep -m 1 "$text" $devfile | awk '{print $9}')
#                 ^^^^ Stop after first match.

if [ "$bus_speed" = "$USB1" ]
then
  echo "USB 1.1 port found."
  # Do something appropriate for USB 1.1.
fi
```

which is the same `fgrep … | awk …` captured in tessh, compared with `==`.
Some hardware can be driven by writing to `/proc` (as root, `echo on
>/proc/acpi/ibm/light` turns on a ThinkPad's light), but writing there can
crash the machine, so take care.

### Example 29-3. Finding the process associated with a PID (`pid-identifier.sh`)

```bash no-run
#!/bin/bash
# pid-identifier.sh:
# Gives complete path name to process associated with pid.

ARGNO=1  # Number of arguments the script expects.
E_WRONGARGS=65
E_BADPID=66
E_NOSUCHPROCESS=67
E_NOPERMISSION=68
PROCFILE=exe

if [ $# -ne $ARGNO ]
then
  echo "Usage: `basename $0` PID-number" >&2  # Error message >stderr.
  exit $E_WRONGARGS
fi  

pidno=$( ps ax | grep $1 | awk '{ print $1 }' | grep $1 )
# Checks for pid in "ps" listing, field #1.
# Then makes sure it is the actual process, not the process invoked by this script.
# The last "grep $1" filters out this possibility.
#
#    pidno=$( ps ax | awk '{ print $1 }' | grep $1 )
#    also works, as Teemu Huovila, points out.

if [ -z "$pidno" ]  #  If, after all the filtering, the result is a zero-length string,
then                #+ no running process corresponds to the pid given.
  echo "No such process running."
  exit $E_NOSUCHPROCESS
fi  

# Alternatively:
#   if ! ps $1 > /dev/null 2>&1
#   then                # no running process corresponds to the pid given.
#     echo "No such process running."
#     exit $E_NOSUCHPROCESS
#    fi

# To simplify the entire process, use "pidof".


if [ ! -r "/proc/$1/$PROCFILE" ]  # Check for read permission.
then
  echo "Process $1 running, but..."
  echo "Can't get read permission on /proc/$1/$PROCFILE."
  exit $E_NOPERMISSION  # Ordinary user can't access some files in /proc.
fi  

# The last two tests may be replaced by:
#    if ! kill -0 $1 > /dev/null 2>&1 # '0' is not a signal, but
                                      # this will test whether it is possible
                                      # to send a signal to the process.
#    then echo "PID doesn't exist or you're not its owner" >&2
#    exit $E_BADPID
#    fi



exe_file=$( ls -l /proc/$1 | grep "exe" | awk '{ print $11 }' )
# Or       exe_file=$( ls -l /proc/$1/exe | awk '{print $11}' )
#
#  /proc/pid-number/exe is a symbolic link
#+ to the complete path name of the invoking process.

if [ -e "$exe_file" ]  #  If /proc/pid-number/exe exists,
then                   #+ then the corresponding process exists.
  echo "Process #$1 invoked by $exe_file."
else
  echo "No such process running."
fi  


#  This elaborate script can *almost* be replaced by
#       ps ax | grep $1 | awk '{ print $5 }'
#  However, this will not work...
#+ because the fifth field of 'ps' is argv[0] of the process,
#+ not the executable file path.
#
# However, either of the following would work.
#       find /proc/$1/exe -printf '%l\n'
#       lsof -aFn -p $1 -d txt | sed -ne 's/^n//p'

# Additional commentary by Stephane Chazelas.

exit 0
```

A process is running when its `/proc` folder is there, so the `ps | grep |
awk | grep` filtering becomes `.is_dir`. Reading the `exe` link is
`os.readlink`, and a link you may not read raises `PermissionError`, which
takes the place of the `-r` test. With no argument it looks at process 1,
whose `exe` only root may read.

```tessh
#!/usr/bin/env tessh
# pid-identifier: gives complete path name to process associated with pid.
import os sys

ARGNO = 1  # Number of arguments the script expects.
E_WRONGARGS = 65
E_NOSUCHPROCESS = 67
E_NOPERMISSION = 68
PROCFILE = exe

args = $argv[1:] or ["1"]   # (sample argument: process 1, init)

if (len $args) != $ARGNO
  print "Usage: $((Path $argv[0]).name) PID-number" file=$sys.stderr
  exit $E_WRONGARGS

pid = $args[0]
proc = Path "/proc/$pid"

if not ($proc.is_dir)         # No running process has this pid.
  print "No such process running."
  exit $E_NOSUCHPROCESS

#  /proc/pid-number/exe is a symbolic link
#+ to the complete path name of the invoking process.
exe_file = try
  os.readlink ($proc / $PROCFILE)
except PermissionError
  print "Process $pid running, but..."
  print "Can't get read permission on $proc/$PROCFILE."
  exit $E_NOPERMISSION   # Ordinary user can't access some files in /proc.

if ((Path $exe_file).exists)
  print "Process #$pid invoked by $exe_file."
else
  print "No such process running."
```

```output
Process 1 running, but...
Can't get read permission on /proc/1/exe.
```

### Example 29-4. On-line connect status (`connect-stat.sh`)

```bash no-run
#!/bin/bash
# connect-stat.sh
#  Note that this script may need modification
#+ to work with a wireless connection.

PROCNAME=pppd        # ppp daemon
PROCFILENAME=status  # Where to look.
NOTCONNECTED=85
INTERVAL=2           # Update every 2 seconds.

pidno=$( ps ax | grep -v "ps ax" | grep -v grep | grep $PROCNAME |
awk '{ print $1 }' )

# Finding the process number of 'pppd', the 'ppp daemon'.
# Have to filter out the process lines generated by the search itself.
#
#  However, as Oleg Philon points out,
#+ this could have been considerably simplified by using "pidof".
#  pidno=$( pidof $PROCNAME )
#
#  Moral of the story:
#+ When a command sequence gets too complex, look for a shortcut.


if [ -z "$pidno" ]   # If no pid, then process is not running.
then
  echo "Not connected."
# exit $NOTCONNECTED
else
  echo "Connected."; echo
fi

while [ true ]       # Endless loop, script can be improved here.
do

  if [ ! -e "/proc/$pidno/$PROCFILENAME" ]
  # While process running, then "status" file exists.
  then
    echo "Disconnected."
#   exit $NOTCONNECTED
  fi

netstat -s | grep "packets received"  # Get some connect statistics.
netstat -s | grep "packets delivered"


  sleep $INTERVAL
  echo; echo

done

exit 0

# As it stands, this script must be terminated with a Control-C.

#    Exercises:
#    ---------
#    Improve the script so it exits on a "q" keystroke.
#    Make the script more user-friendly in other ways.
#    Fix the script to work with wireless/DSL connections.
```

This takes Oleg Philon's shortcut, `pidof`. `netstat` is often not
installed now; the counts it showed come from `/proc/net/snmp`, whose `Ip:`
lines are a row of names and a row of numbers, zipped here into a dict. The
endless loop is cut to two rounds so it ends.

```tessh
#!/usr/bin/env tessh
# connect-stat: on-line connect status.

PROCNAME = pppd        # ppp daemon
PROCFILENAME = status  # Where to look.
INTERVAL = 0.2         # Update every 0.2 seconds (2 in the original).

pidno = pidof $PROCNAME

if not $pidno          # If no pid, then process is not running.
  print "Not connected."
else
  print "Connected." ; print

def ip_stats
  text = ((Path /proc/net/snmp).read_text)
  [names values] = [($l.split)[1:] for l in ($text.splitlines) if ($l.startswith "Ip:")]
  dict (zip $names $values)

for round in (range 2)   # (Endless in the original: Control-C ended it.)
  if not ((Path "/proc/$pidno/$PROCFILENAME").exists)
  # While process running, then "status" file exists.
    print "Disconnected."
  stats = (ip_stats)     # Get some connect statistics.
  print "    $stats['InReceives'] total packets received"
  print "    $stats['InDelivers'] incoming packets delivered"
  sleep $INTERVAL
  print ; print
```

```text
(one run)
Not connected.
Disconnected.
    86472760 total packets received
    31892782 incoming packets delivered


Disconnected.
    86472912 total packets received
    31892818 incoming packets delivered


```
