# 28. Indirect References
<!-- help: abs-ivr abs-28 indirect-references indirect eval pointer globals -->

A variable whose value is the name of another variable is an indirect
reference. In bash, getting at "the value of the value" takes `eval` and an
escaped dollar sign, `eval newvar=\$$varname`, or the newer `${!varname}`
(chapter 37). In tessh a variable is a Python name, and the names a script
has made are in the dict `(globals)`: `(globals)[$name]` is the variable
called whatever `$name` holds. Often a dict of your own is better still: the
names you would build are its keys.

### Example 28-1. Indirect Variable References (`ind-ref.sh`)

```bash no-run
#!/bin/bash
# ind-ref.sh: Indirect variable referencing.
# Accessing the contents of the contents of a variable.

# First, let's fool around a little.

var=23

echo "\$var   = $var"           # $var   = 23
# So far, everything as expected. But ...

echo "\$\$var  = $$var"         # $$var  = 4570var
#  Not useful ...
#  \$\$ expanded to PID of the script
#  -- refer to the entry on the $$ variable --
#+ and "var" is echoed as plain text.
#  (Thank you, Jakob Bohm, for pointing this out.)

echo "\\\$\$var = \$$var"       # \$$var = $23
#  As expected. The first $ is escaped and pasted on to
#+ the value of var ($var = 23 ).
#  Meaningful, but still not useful. 

# Now, let's start over and do it the right way.

# ============================================== #


a=letter_of_alphabet   # Variable "a" holds the name of another variable.
letter_of_alphabet=z

echo

# Direct reference.
echo "a = $a"          # a = letter_of_alphabet

# Indirect reference.
  eval a=\$$a
# ^^^        Forcing an eval(uation), and ...
#        ^   Escaping the first $ ...
# ------------------------------------------------------------------------
# The 'eval' forces an update of $a, sets it to the updated value of \$$a.
# So, we see why 'eval' so often shows up in indirect reference notation.
# ------------------------------------------------------------------------
  echo "Now a = $a"    # Now a = z

echo


# Now, let's try changing the second-order reference.

t=table_cell_3
table_cell_3=24
echo "\"table_cell_3\" = ${table_cell_3}"            # "table_cell_3" = 24
echo -n "dereferenced \"t\" = "; eval echo \$$t    # dereferenced "t" = 24
# In this simple case, the following also works (why?).
#         eval t=\$$t; echo "\"t\" = $t"

echo

t=table_cell_3
NEW_VAL=387
table_cell_3=$NEW_VAL
echo "Changing value of \"table_cell_3\" to $NEW_VAL."
echo "\"table_cell_3\" now ${table_cell_3}"
echo -n "dereferenced \"t\" now "; eval echo \$$t
# "eval" takes the two arguments "echo" and "\$$t" (set equal to $table_cell_3)


echo

# (Thanks, Stephane Chazelas, for clearing up the above behavior.)


#   A more straightforward method is the ${!t} notation, discussed in the
#+ "Bash, version 2" section.
#   See also ex78.sh.

exit 0
```

(The bash version prints its own process ID, so it isn't run here.) tessh has
no `$$`: the process ID is `(os.getpid)`. There's no `eval` step either: the
lookup is an ordinary expression, and it reads the variable's value at the
moment it runs, so a changed `table_cell_3` shows through `t` at once.

```tessh
# ind-ref.sh: Indirect variable referencing.
# Accessing the contents of the contents of a variable.

var = 23

print "\$var   = $var"           # $var   = 23
# bash's "$$var" is the script's PID and then "var"; tessh has no $$
# ((os.getpid) is the PID).
print "\\\$\$var = \$$var"       # \$$var = $23

# Now, let's start over and do it the right way.

a = letter_of_alphabet   # Variable "a" holds the name of another variable.
letter_of_alphabet = z

print

# Direct reference.
print "a = $a"           # a = letter_of_alphabet

# Indirect reference: the variable named by $a, from the script's globals.
a = (globals)[$a]
print "Now a = $a"       # Now a = z

print

# Now, let's try changing the second-order reference.

t = table_cell_3
table_cell_3 = 24
print "\"table_cell_3\" = ${table_cell_3}"               # "table_cell_3" = 24
print "dereferenced \"t\" = $((globals)[$t])"          # dereferenced "t" = 24

print

NEW_VAL = 387
table_cell_3 = $NEW_VAL
print "Changing value of \"table_cell_3\" to $NEW_VAL."
print "\"table_cell_3\" now ${table_cell_3}"
print "dereferenced \"t\" now $((globals)[$t])"

print
```

```output
$var   = 23
\$$var = $23

a = letter_of_alphabet
Now a = z

"table_cell_3" = 24
dereferenced "t" = 24

Changing value of "table_cell_3" to 387.
"table_cell_3" now 387
dereferenced "t" now 387

```

Nils Radtke builds variable names on the fly, which helps when the values
come from a sourced configuration file.

```bash
#!/bin/bash
# ---------------------------------------------
# This could be "sourced" from a separate file.
isdnMyProviderRemoteNet=172.16.0.100
isdnYourProviderRemoteNet=10.0.0.10
isdnOnlineService="MyProvider"
# ---------------------------------------------
      

remoteNet=$(eval "echo \$$(echo isdn${isdnOnlineService}RemoteNet)")
remoteNet=$(eval "echo \$$(echo isdnMyProviderRemoteNet)")
remoteNet=$(eval "echo \$isdnMyProviderRemoteNet")
remoteNet=$(eval "echo $isdnMyProviderRemoteNet")

echo "$remoteNet"    # 172.16.0.100

# ================================================================

#  And, it gets even better.

#  Consider the following snippet given a variable named getSparc,
#+ but no such variable getIa64:

chkMirrorArchs () { 
  arch="$1";
  if [ "$(eval "echo \${$(echo get$(echo -ne $arch |
       sed 's/^\(.\).*/\1/g' | tr 'a-z' 'A-Z'; echo $arch |
       sed 's/^.\(.*\)/\1/g')):-false}")" = true ]
  then
     return 0;
  else
     return 1;
  fi;
}

getSparc="true"
unset getIa64
chkMirrorArchs sparc
echo $?        # 0
               # True

chkMirrorArchs Ia64
echo $?        # 1
               # False

# Notes:
# -----
# Even the to-be-substituted variable name part is built explicitly.
# The parameters to the chkMirrorArchs calls are all lower case.
# The variable name is composed of two parts: "get" and "Sparc" . . .
```

```output bash
172.16.0.100
0
1
```

The name is an f-string, the capitalizing is `$arch[0].upper`, and
`(globals).get name default` is bash's `${name:-false}`. The check gives
back `True` or `False` rather than an exit status.

```tessh
# ---------------------------------------------
# This could be "sourced" from a separate file.
isdnMyProviderRemoteNet = "172.16.0.100"
isdnYourProviderRemoteNet = "10.0.0.10"
isdnOnlineService = MyProvider
# ---------------------------------------------

remoteNet = (globals)["isdn${isdnOnlineService}RemoteNet"]
print $remoteNet     # 172.16.0.100

#  Given a variable named getSparc, but no such variable getIa64:
def chkMirrorArchs arch
  name = "get" + ($arch[0].upper) + $arch[1:]
  ((globals).get $name "false") == "true"

getSparc = "true"
print (chkMirrorArchs sparc)   # True
print (chkMirrorArchs ia64)    # False
```

```output
172.16.0.100
True
False
```

### Example 28-2. Passing an indirect reference to awk (`col-totaler2.sh`)

```bash no-run
#!/bin/bash

#  Another version of the "column totaler" script
#+ that adds up a specified column (of numbers) in the target file.
#  This one uses indirect references.

ARGS=2
E_WRONGARGS=85

if [ $# -ne "$ARGS" ] # Check for proper number of command-line args.
then
   echo "Usage: `basename $0` filename column-number"
   exit $E_WRONGARGS
fi

filename=$1         # Name of file to operate on.
column_number=$2    # Which column to total up.

#===== Same as original script, up to this point =====#


# A multi-line awk script is invoked by
#   awk "
#   ...
#   ...
#   ...
#   "


# Begin awk script.
# -------------------------------------------------
awk "

{ total += \$${column_number} # Indirect reference
}
END {
     print total
     }

     " "$filename"
# Note that awk doesn't need an eval preceding \$$.
# -------------------------------------------------
# End awk script.

#  Indirect variable reference avoids the hassles
#+ of referencing a shell variable within the embedded awk script.
#  Thanks, Stephane Chazelas.


exit $?
```

The awk program is a triple-quoted tessh string, so `\$` is awk's field sign
and `$column_number` is filled in. (`awk -v col=$column_number` and
`$col` inside, with the program in single quotes, is the tidier way: then
nothing is pasted into awk's code.)

```tessh
# (A made-up data file, and sample arguments when none are given.)
printf "a 1 10\nb 2 20\nc 3 30\n" >data.txt
args = $argv[1:] or [data.txt 3]

ARGS = 2
E_WRONGARGS = 85

if (len $args) != $ARGS      # Check for proper number of command-line args.
  print "Usage: $((Path $argv[0]).name) filename column-number"
  exit $E_WRONGARGS

[filename column_number] = $args

# Begin awk script.
awk """
{ total += \$$column_number  # Indirect reference
}
END {
     print total
     }
""" $filename
# End awk script.
```

```output
60
```

Bash has no pointer arithmetic, and indirect references are an
afterthought in it. In tessh every value is an object you can pass and
share, so the usual answer to "a variable that names a variable" is to pass
the value itself, or to keep related values in a dict keyed by name.
