# 30. Network Programming
<!-- help: abs-networkprogramming abs-30 network networking cgi lsof ip-addresses -->

Linux has plenty of tools for looking at and managing network connections,
and a script can put them together. tessh runs them as bash does, and adds
Python's own networking: `socket`, `urllib.request`, `http.server`,
`ipaddress`, all without installing anything.

### Example 30-1. Print the server environment (`test-cgi.sh`)

```bash no-run
#!/bin/bash
# test-cgi.sh
# by Michael Zick
# Used with permission

# May have to change the location for your site.
# (At the ISP's servers, Bash may not be in the usual place.)
# Other places: /usr/bin or /usr/local/bin
# Might even try it without any path in sha-bang.

# Disable filename globbing.
set -f

# Header tells browser what to expect.
echo Content-type: text/plain
echo

echo CGI/1.0 test script report:
echo

echo environment settings:
set
echo

echo whereis bash?
whereis bash
echo


echo who are we?
echo ${BASH_VERSINFO[*]}
echo

echo argc is $#. argv is "$*".
echo

# CGI/1.0 expected environment variables.

echo SERVER_SOFTWARE = $SERVER_SOFTWARE
echo SERVER_NAME = $SERVER_NAME
echo GATEWAY_INTERFACE = $GATEWAY_INTERFACE
echo SERVER_PROTOCOL = $SERVER_PROTOCOL
echo SERVER_PORT = $SERVER_PORT
echo REQUEST_METHOD = $REQUEST_METHOD
echo HTTP_ACCEPT = "$HTTP_ACCEPT"
echo PATH_INFO = "$PATH_INFO"
echo PATH_TRANSLATED = "$PATH_TRANSLATED"
echo SCRIPT_NAME = "$SCRIPT_NAME"
echo QUERY_STRING = "$QUERY_STRING"
echo REMOTE_HOST = $REMOTE_HOST
echo REMOTE_ADDR = $REMOTE_ADDR
echo REMOTE_USER = $REMOTE_USER
echo AUTH_TYPE = $AUTH_TYPE
echo CONTENT_TYPE = $CONTENT_TYPE
echo CONTENT_LENGTH = $CONTENT_LENGTH

exit 0

# Here document to give short instructions.
:<<-'_test_CGI_'

1) Drop this in your http://domain.name/cgi-bin directory.
2) Then, open http://domain.name/cgi-bin/test-cgi.sh.

_test_CGI_
```

A CGI script gets the request in environment variables, so in tessh they
are `$env.get name ""` (a missing one is an empty string, as bash's unset
variable is). The seventeen `echo` lines become a loop over the names.
There's no `set -f` to turn off: a tessh variable's value is never globbed.
Bash's `set` would list every variable, which is more than a web page
should show; this lists the request's. To try it, the script first sets up a
made-up request as a web server would.

```tessh
import sys

# (A made-up request, as the web server would set it up, to try it on.)
export SERVER_SOFTWARE=Apache/2.4.58 SERVER_NAME=example.org
export GATEWAY_INTERFACE=CGI/1.1 SERVER_PROTOCOL=HTTP/1.1 SERVER_PORT=80
export REQUEST_METHOD=GET HTTP_ACCEPT='*/*' SCRIPT_NAME=/cgi-bin/test-cgi.tsh
export QUERY_STRING='a=1&b=2' REMOTE_ADDR=192.0.2.7

# Header tells browser what to expect.
print Content-type: text/plain
print

print CGI/1.0 test script report:
print

print environment settings:
# (bash's set lists every variable; these are the request's.)
for [name value] in (sorted ($env.items))
  if $name =~ re'^(SERVER|GATEWAY|REQUEST|HTTP|SCRIPT|QUERY|REMOTE)_'
    print "$name=$value"
print

print who are we?
print "tessh, on Python $sys.version_info.major"
print

args = $argv[1:]
print "argc is $(len $args). argv is $(' '.join $args)."
print

# CGI/1.0 expected environment variables.
CGI_VARS = [SERVER_SOFTWARE SERVER_NAME GATEWAY_INTERFACE SERVER_PROTOCOL
            SERVER_PORT REQUEST_METHOD HTTP_ACCEPT PATH_INFO PATH_TRANSLATED
            SCRIPT_NAME QUERY_STRING REMOTE_HOST REMOTE_ADDR REMOTE_USER
            AUTH_TYPE CONTENT_TYPE CONTENT_LENGTH]
for name in $CGI_VARS
  print $name = ($env.get $name "")

exit 0

# 1) Drop this in your http://domain.name/cgi-bin directory.
# 2) Then, open http://domain.name/cgi-bin/test-cgi.tsh.
```

```output
Content-type: text/plain

CGI/1.0 test script report:

environment settings:
GATEWAY_INTERFACE=CGI/1.1
HTTP_ACCEPT=*/*
QUERY_STRING=a=1&b=2
REMOTE_ADDR=192.0.2.7
REQUEST_METHOD=GET
SCRIPT_NAME=/cgi-bin/test-cgi.tsh
SERVER_NAME=example.org
SERVER_PORT=80
SERVER_PROTOCOL=HTTP/1.1
SERVER_SOFTWARE=Apache/2.4.58

who are we?
tessh, on Python 3

argc is 0. argv is .

SERVER_SOFTWARE = Apache/2.4.58
SERVER_NAME = example.org
GATEWAY_INTERFACE = CGI/1.1
SERVER_PROTOCOL = HTTP/1.1
SERVER_PORT = 80
REQUEST_METHOD = GET
HTTP_ACCEPT = */*
PATH_INFO = 
PATH_TRANSLATED = 
SCRIPT_NAME = /cgi-bin/test-cgi.tsh
QUERY_STRING = a=1&b=2
REMOTE_HOST = 
REMOTE_ADDR = 192.0.2.7
REMOTE_USER = 
AUTH_TYPE = 
CONTENT_TYPE = 
CONTENT_LENGTH = 
```

The `whereis bash` step is left out (`which tessh` shows where tessh is).
Python's `http.server` module can run such a script for a quick test,
`python3 -m http.server --cgi`, and `urllib.parse.parse_qs $env.QUERY_STRING`
takes the query string apart.

### Example 30-2. IP addresses (`ip-addresses.sh`)

```bash no-run
#!/bin/bash
# ip-addresses.sh
# List the IP addresses your computer is connected to.

#  Inspired by Greg Bledsoe's ddos.sh script,
#  Linux Journal, 09 March 2011.
#    URL:
#  http://www.linuxjournal.com/content/back-dead-simple-bash-complex-ddos
#  Greg licensed his script under the GPL2,
#+ and as a derivative, this script is likewise GPL2.

connection_type=TCP      # Also try UDP.
field=2           # Which field of the output we're interested in.
no_match=LISTEN   # Filter out records containing this. Why?
lsof_args=-ni     # -i lists Internet-associated files.
                  # -n preserves numerical IP addresses.
		  # What happens without the -n option? Try it.
router="[0-9][0-9][0-9][0-9][0-9]->"
#       Delete the router info.

lsof "$lsof_args" | grep $connection_type | grep -v "$no_match" |
      awk '{print $9}' | cut -d : -f $field | sort | uniq |
      sed s/"^$router"//

#  Bledsoe's script assigns the output of a filtered IP list,
#  (similar to lines 19-22, above) to a variable.
#  He checks for multiple connections to a single IP address,
#  then uses:
#
#    iptables -I INPUT -s $ip -p tcp -j REJECT --reject-with tcp-reset
#
#  ... within a 60-second delay loop to bounce packets from DDOS attacks.


#  Exercise:
#  --------
#  Use the 'iptables' command to extend this script
#+ to reject connection attempts from well-known spammer IP domains.
```

The pipeline is the same in tessh. It's fed made-up `lsof -ni` output here,
since the real list is different on every machine (and nobody's business).
A pipe at the end of a line doesn't carry on to the next in tessh, as it
does in bash: end the line with `| \`.

```tessh
#  ip-addresses: list the IP addresses your computer is connected to.
#  Inspired by Greg Bledsoe's ddos.sh script (GPL2, as this is).

connection_type = TCP    # Also try UDP.
field = 2                # Which field of the output we're interested in.
no_match = LISTEN        # Filter out records containing this. Why?
lsof_args = -ni          # -i lists Internet-associated files.
                         # -n preserves numerical IP addresses.
router = "[0-9][0-9][0-9][0-9][0-9]->"
#         Delete the router info.

# (Made-up output of  lsof $lsof_args  to try it on.)
lsof_output = """\
COMMAND    PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
sshd       812 root    3u  IPv4  20211      0t0  TCP *:22 (LISTEN)
firefox   2345 ann    51u  IPv4  81234      0t0  TCP 192.168.1.5:52654->203.0.113.80:443 (ESTABLISHED)
firefox   2345 ann    63u  IPv4  81240      0t0  TCP 192.168.1.5:52660->198.51.100.9:443 (ESTABLISHED)
firefox   2345 ann    70u  IPv4  81302      0t0  TCP 192.168.1.5:52671->203.0.113.80:443 (ESTABLISHED)
thunderbi 2410 ann    44u  IPv4  80777      0t0  TCP 192.168.1.5:41022->192.0.2.25:993 (ESTABLISHED)
avahi-dae  640 avahi  12u  IPv4  18843      0t0  UDP *:5353
"""

$lsof_output | grep $connection_type | grep -v $no_match | \
      awk '{print $9}' | cut -d : -f $field | sort | uniq | \
      sed "s/^$router//"
```

```output
192.0.2.25
203.0.113.80
198.51.100.9
203.0.113.80
```

As in bash, an address can show twice: `uniq` runs while the local port
is still on the front, and the `sed` only strips five-digit ports. A regex says what's wanted
directly, and a `Counter` counts the connections to each address, which is
what Bledsoe's script went on to do:

```tessh
from collections import Counter
# (The same made-up lsof output.)
lsof_output = """\
firefox   2345 ann    51u  IPv4  81234      0t0  TCP 192.168.1.5:52654->203.0.113.80:443 (ESTABLISHED)
firefox   2345 ann    63u  IPv4  81240      0t0  TCP 192.168.1.5:52660->198.51.100.9:443 (ESTABLISHED)
firefox   2345 ann    70u  IPv4  81302      0t0  TCP 192.168.1.5:52671->203.0.113.80:443 (ESTABLISHED)
thunderbi 2410 ann    44u  IPv4  80777      0t0  TCP 192.168.1.5:41022->192.0.2.25:993 (ESTABLISHED)
"""
peers = Counter (re'->([^:]+):'.findall $lsof_output)
for [ip n] in ($peers.most_common)
  print $ip $n
```

```output
203.0.113.80 2
198.51.100.9 1
192.0.2.25 1
```

More network examples elsewhere in the guide: the time from nist.gov and
downloading a URL (chapter 29), a GRE tunnel, and checking whether an
Internet server is up. See also the networking commands in chapter 17 and
the communications commands in chapter 16.
