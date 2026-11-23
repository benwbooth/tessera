# 23. Process Substitution
<!-- help: abs-process-sub abs-23 process-substitution <( >( /dev/fd -->

A pipe feeds one command's output to another's input. Process substitution
does it where a command wants a *file*: `<(command)` is a file name, a
`/dev/fd/N`, whose contents are what the command prints, and `>(command)`
is one that the command reads what's written to. So a program that takes
file names can take the output of several commands at once. There's no
space between the `<` or `>` and the parenthesis.

tessh has bash's `<(…)` and `>(…)`. Inside can be a program, a pipeline
(with its own redirections), or a call of a function that prints.

```bash
echo >(true)
echo <(true)
echo >(true) <(true)
```

```output bash
/dev/fd/63
/dev/fd/63
/dev/fd/63 /dev/fd/62
```

```tessh
echo >(true)
echo <(true)
echo >(true) <(true)
```

```output
/dev/fd/4
/dev/fd/3
/dev/fd/4 /dev/fd/3
```

(The numbers differ: tessh takes the lowest free ones.) bash makes a pipe,
connects the command in the parentheses to one end, and hands the other
end's `/dev/fd` name to the command outside; tessh does the same.

`wc <(cat /usr/share/dict/linux.words)` counts the lines of a command's
output, with the file name `/dev/fd/63` in place of a real one. Comparing
two commands' output, or one command's with different options, is a common
use: `comm <(ls -l) <(ls -al)`, or `diff` on two folders' listings to see
which names are in one but not the other.

```bash no-run
diff <(ls $first_directory) <(ls $second_directory)
```

```tessh
# (Two made-up folders to compare.)
mkdir -p first second ; touch first/a first/b second/b second/c
first_directory = first ; second_directory = second

diff <(ls $first_directory) <(ls $second_directory)
```

```output
1d0
< a
2a2
> c
```

More uses: `>(…)` with `tee`, to take checksums of a stream on its way
through. This one reads from the network; in the tessh version a `printf`
stands in for `netcat`.

```bash no-run
PORT=6881   # bittorrent

# Scan the port to make sure nothing nefarious is going on.
netcat -l $PORT | tee>(md5sum ->mydata-orig.md5) |
gzip | tee>(md5sum - | sed 's/-$/mydata.lz2/'>mydata-gz.md5)>mydata.gz

# Check the decompression:
  gzip -d<mydata.gz | md5sum -c mydata-orig.md5)
# The MD5sum of the original checks stdin and detects compression issues.

#  Bill Davidsen contributed this example
#+ (with light edits by the ABS Guide author).
```

The `>(…)` commands run on their own, and bash doesn't wait for them; the
tessh version waits a moment before it checks. (The original has a stray
`)` on its check line, and a pipe that goes on to the next line, which in
tessh is written with `\`.)

```tessh
import time
printf "the data\n" | tee >(md5sum - >mydata-orig.md5) \
  | gzip | tee >(md5sum - | sed 's/-$/mydata.gz/' >mydata-gz.md5) >mydata.gz
time.sleep 0.2         # (Let the md5sums finish.)

# Check the decompression:
gzip -d <mydata.gz | md5sum -c mydata-orig.md5
# The MD5sum of the original checks stdin and detects compression issues.
cat mydata-gz.md5
```

```output
-: OK
a2b16e1baa1ef475d286bf743f7cd817  mydata.gz
```

And others:

```bash no-run
cat <(ls -l)
# Same as     ls -l | cat

sort -k 9 <(ls -l /bin) <(ls -l /usr/bin) <(ls -l /usr/X11R6/bin)
# Lists all the files in the 3 main 'bin' directories, and sorts by filename.
# Note that three (count 'em) distinct commands are fed to 'sort'.

 
diff <(command1) <(command2)    # Gives difference in command output.

tar cf >(bzip2 -c > file.tar.bz2) $directory_name
# Calls "tar cf /dev/fd/?? $directory_name", and "bzip2 -c > file.tar.bz2".
#
# Because of the /dev/fd/<n> system feature,
# the pipe between both commands does not need to be named.
```

```tessh
import time
mkdir -p project ; echo hi >project/notes     # (A made-up folder.)
directory_name = project

sort <(printf "pear\n") <(printf "apple\n") <(printf "fig\n")
# Three (count 'em) distinct commands are fed to 'sort'.

tar cf >(bzip2 -c > file.tar.bz2) $directory_name
# Calls "tar cf /dev/fd/?? $directory_name", and "bzip2 -c > file.tar.bz2".
time.sleep 0.2         # (Let bzip2 finish.)
tar tjf file.tar.bz2
```

```output
apple
fig
pear
project/
project/notes
```

The ABS Guide also shows how this can be done without `/dev/fd`, with a
named pipe (`mkfifo`) or with descriptors juggled by `exec`; tessh does that
with `os.mkfifo` and `os.dup2`, as in chapter 20.

A loop at the end of a pipe runs in a subshell in bash, so what it sets is
lost; a loop redirected from a process substitution doesn't, so it's kept.
In tessh no loop runs in a subshell, and a loop goes over a value: the
command's output, or the open process substitution.

### Example 23-1. Code block redirection without forking (`wr-ps.bash`)

```bash
#!/bin/bash
# wr-ps.bash: while-read loop with process substitution.

# This example contributed by Tomas Pospisek.
# (Heavily edited by the ABS Guide author.)

echo

echo "random input" | while read i
do
  global=3D": Not available outside the loop."
  # ... because it runs in a subshell.
done

echo "\$global (from outside the subprocess) = $global"
# $global (from outside the subprocess) =

echo; echo "--"; echo

while read i
do
  echo $i
  global=3D": Available outside the loop."
  # ... because it does NOT run in a subshell.
done < <( echo "random input" )
#    ^ ^

echo "\$global (using process substitution) = $global"
# Random input
# $global (using process substitution) = 3D: Available outside the loop.


echo; echo "##########"; echo



# And likewise . . .

declare -a inloop
index=0
cat $0 | while read line
do
  inloop[$index]="$line"
  ((index++))
  # It runs in a subshell, so ...
done
echo "OUTPUT = "
echo ${inloop[*]}           # ... nothing echoes.


echo; echo "--"; echo


declare -a outloop
index=0
while read line
do
  outloop[$index]="$line"
  ((index++))
  # It does NOT run in a subshell, so ...
done < <( cat $0 )
echo "OUTPUT = "
echo ${outloop[*]}          # ... the entire script echoes.

exit $?
```

```output bash

$global (from outside the subprocess) = 

--

random input
$global (using process substitution) = 3D: Available outside the loop.

##########

OUTPUT = 


--

OUTPUT = 
#!/bin/bash # wr-ps.bash: while-read loop with process substitution. # This example contributed by Tomas Pospisek. # (Heavily edited by the ABS Guide author.) echo echo "random input" | while read i do global=3D": Not available outside the loop." # ... because it runs in a subshell. done echo "$global (from outside the subprocess) = $global" # $global (from outside the subprocess) = echo; echo "--"; echo while read i do echo $i global=3D": Available outside the loop." # ... because it does NOT run in a subshell. done < <( echo "random input" ) # ^ ^ echo "$global (using process substitution) = $global" # Random input # $global (using process substitution) = 3D: Available outside the loop. echo; echo "##########"; echo # And likewise . . . declare -a inloop index=0 cat $0 | while read line do inloop[$index]="$line" ((index++)) # It runs in a subshell, so ... done echo "OUTPUT = " echo ${inloop[*]} # ... nothing echoes. echo; echo "--"; echo declare -a outloop index=0 while read line do outloop[$index]="$line" ((index++)) # It does NOT run in a subshell, so ... done < <( cat $0 ) echo "OUTPUT = " echo ${outloop[*]} # ... the entire script echoes. exit $?
```

Both halves of each pair work in tessh, so the "not available" lines are
available too. (`global` is a word of the language in Python, so the
variable is `global_var`; the arrays are lists.)

```tessh
#!/usr/bin/env tessh
print

for i in (echo "random input").lines
  global_var = "3D: Available outside the loop too."
  # ... because no loop runs in a subshell in tessh.

print "\$global_var (from the pipe's output) = $global_var"

print ; print "--" ; print

for i in (open <(echo "random input"))
  print ($i.strip)
  global_var = "3D: Available outside the loop."

print "\$global_var (using process substitution) = $global_var"

print ; print "##########" ; print

# And likewise . . .

inloop = []
for line in (cat $argv[0]).lines
  $inloop.append ($line.strip)
print "OUTPUT = "
print (len $inloop) lines              # ... all of them.

print ; print "--" ; print

outloop = []
for line in (open <(cat $argv[0]))
  $outloop.append ($line.strip)
print "OUTPUT = "
print *$outloop[:3]                    # (The first three: the whole script is there.)

exit $?
```

```output

$global_var (from the pipe's output) = 3D: Available outside the loop too.

--

random input
$global_var (using process substitution) = 3D: Available outside the loop.

##########

OUTPUT = 
36 lines

--

OUTPUT = 
#!/usr/bin/env tessh print 
```

### Example 23-2. Redirecting the output of process substitution into a loop. (`psub.bash`)

```bash
#!/bin/bash
# psub.bash

# As inspired by Diego Molina (thanks!).

declare -a array0
while read
do
  array0[${#array0[@]}]="$REPLY"
done < <( sed -e 's/bash/CRASH-BANG!/' $0 | grep bin | awk '{print $1}' )
#  Sets the default 'read' variable, $REPLY, by process substitution,
#+ then copies it into an array.

echo "${array0[@]}"

exit $?

# ====================================== #

bash psub.bash

#!/bin/CRASH-BANG! done #!/bin/CRASH-BANG!
```

```output bash
#!/bin/CRASH-BANG! done #!/bin/CRASH-BANG!
```

The tessh version reads its own file the same way; its lines with `bin`
are the `#!` line and the loop's own.

```tessh
#!/usr/bin/env tessh
# psub.tsh

array0 = []
for REPLY in (open <(sed -e 's/tessh/CRASH-BANG!/' $argv[0] | grep bin | awk '{print $1}'))
  $array0.append ($REPLY.removesuffix "\n")
#  Each line of the process substitution's output,
#+ then appended to the list.

print *$array0

exit $?
```

```output
#!/usr/bin/env for
```

A loop over a process substitution, from SuSE's scripts, that goes over
the routing table. Its output depends on the machine, so it doesn't run
here.

```bash no-run
# Script fragment taken from SuSE distribution:

# --------------------------------------------------------------#
while read  des what mask iface; do
# Some commands ...
done < <(route -n)  
#    ^ ^  First < is redirection, second is process substitution.

# To test it, let's make it do something.
while read  des what mask iface; do
  echo $des $what $mask $iface
done < <(route -n)  

# Output:
# Kernel IP routing table
# Destination Gateway Genmask Flags Metric Ref Use Iface
# 127.0.0.0 0.0.0.0 255.0.0.0 U 0 0 0 lo
# --------------------------------------------------------------#

#  As Stéphane Chazelas points out,
#+ an easier-to-understand equivalent is:
route -n |
  while read des what mask iface; do   # Variables set from output of pipe.
    echo $des $what $mask $iface
  done  #  This yields the same output as above.
        #  However, as Ulrich Gayer points out . . .
        #+ this simplified equivalent uses a subshell for the while loop,
        #+ and therefore the variables disappear when the pipe terminates.
	
# --------------------------------------------------------------#
	
#  However, Filip Moritz comments that there is a subtle difference
#+ between the above two examples, as the following shows.

(
route -n | while read x; do ((y++)); done
echo $y # $y is still unset

while read x; do ((y++)); done < <(route -n)
echo $y # $y has the number of lines of output of route -n
)
```

`read des what mask iface` splits a line into four names, the last taking
the rest; in tessh that's unpacking `.split` with a `*rest`. Whether the
loop goes over the pipe's output or the process substitution, `y` keeps
its count.

```tessh no-run
for line in (open <(route -n))
  [des what mask iface *rest] = ($line.split) + ["" "" "" ""]
  print $des $what $mask $iface

y = 0
for x in (route -n).lines
  y += 1
print $y    # the number of lines of output of route -n

y = 0
for x in (open <(route -n))
  y += 1
print $y    # the same
```
