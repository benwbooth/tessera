# 24. Functions
<!-- help: abs-functions abs-24 function def subroutine -->

A function is a named block of code: write the steps once, call them by
name. bash has functions, though limited ones: they take arguments as `$1`,
`$2`…, give back only an exit status (0 to 255), and set variables for the
caller to read. A tessh function is a Python function written as tessh:
`def name params…`, an indented body, parameters with names and defaults,
and any value given back, the last form's or a `return`'s.

```bash no-run
function function_name {
command...
}

function_name () {
command...
}
```

```tessh no-run
def function_name
  command...

def function_name param1 param2=default *rest
  command...
```

A one-line bash function, `fun () { echo "This is a function"; echo; }`,
is a one-line `def` in tessh only when the body is one form
(`def sq x = $x * $x`); otherwise it takes the usual indented lines.

A function is called by its name, like a command.

### Example 24-1. Simple functions (`ex59.sh`)

```bash
#!/bin/bash
# ex59.sh: Exercising functions (simple).

JUST_A_SECOND=1

funky ()
{ # This is about as simple as functions get.
  echo "This is a funky function."
  echo "Now exiting funky function."
} # Function declaration must precede call.


fun ()
{ # A somewhat more complex function.
  i=0
  REPEATS=30

  echo
  echo "And now the fun really begins."
  echo

  sleep $JUST_A_SECOND    # Hey, wait a second!
  while [ $i -lt $REPEATS ]
  do
    echo "----------FUNCTIONS---------->"
    echo "<------------ARE-------------"
    echo "<------------FUN------------>"
    echo
    let "i+=1"
  done
}

  # Now, call the functions.

funky
fun

exit $?
```

The `while` with its counter is a `for` over a `range` in tessh. The
second's pause is shortened to a tenth here. A function's variables are its
own in tessh unless it says `global`, so `i` and `REPEATS` don't leak out
of `fun` as they do in bash.

```tessh
#!/usr/bin/env tessh
JUST_A_SECOND = 0.1     # (1 in the original)

def funky
  # This is about as simple as functions get.
  print "This is a funky function."
  print "Now exiting funky function."
  # Function declaration must precede call.


def fun
  # A somewhat more complex function.
  REPEATS = 30

  print
  print "And now the fun really begins."
  print

  sleep $JUST_A_SECOND    # Hey, wait a second!
  for i in (range $REPEATS)
    print "----------FUNCTIONS---------->"
    print "<------------ARE-------------"
    print "<------------FUN------------>"
    print

  # Now, call the functions.

funky
fun

exit $?
```

```output
This is a funky function.
Now exiting funky function.

And now the fun really begins.

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

----------FUNCTIONS---------->
<------------ARE-------------
<------------FUN------------>

```

The definition must come before the first call, in tessh as in bash: a
call of `f1` before its `def` is a `NameError`. But a function can call one
defined after it, as long as that's defined by the time of the call.

```bash
f1 ()
{
  echo "Calling function \"f2\" from within function \"f1\"."
  f2
}

f2 ()
{
  echo "Function \"f2\"."
}

f1  #  Function "f2" is not actually called until this point,
    #+ although it is referenced before its definition.
    #  This is permissible.
    
    # Thanks, S.C.
```

```tessh
def f1
  print "Calling function \"f2\" from within function \"f1\"."
  f2

def f2
  print "Function \"f2\"."

f1  #  Function "f2" is not actually called until this point,
    #+ although it is referenced before its definition.
```

```output
Calling function "f2" from within function "f1".
Function "f2".
```

A bash function can't be empty, not even with only comments in it; it
needs at least `:`. A tessh one needs at least `pass`. A function defined
inside another is, in bash, defined (for everyone) when the outer one runs;
in tessh it's the outer function's own local, never visible outside. And as
in bash, a function defined twice is the second one from then on.

```bash
func ()
{
  echo "First version of func ()."
}

func ()
{
  echo "Second version of func ()."
}

func   # Second version of func ().
```

```tessh
def func
  print "First version of func ()."

def func
  print "Second version of func ()."

func   # Second version of func ().
```

```output
Second version of func ().
```

bash allows definitions in odd places (in an `if`, in an and-list, even
replacing the `exit` builtin) and odd names (`_`, `:`). A tessh `def` can
go in an `if` too, and `_` is a fine name; builtins such as `exit` and
`cd` are tessh's own and stay so.

## 24.1 Complex Functions and Function Complexities
<!-- help: abs-complexfunct function-parameters return exit-status -->

A bash function gets its arguments by position, `$1`, `$2`…, and can't
tell how many it was meant to get. A tessh function names its parameters,
gives them defaults, and takes any number more with `*rest`; calling it
with too few or too many is an error that says so.

### Example 24-2. Function Taking Parameters (`ex60.sh`)

```bash
#!/bin/bash
# Functions and parameters

DEFAULT=default                             # Default param value.

func2 () {
   if [ -z "$1" ]                           # Is parameter #1 zero length?
   then
     echo "-Parameter #1 is zero length.-"  # Or no parameter passed.
   else
     echo "-Parameter #1 is \"$1\".-"
   fi

   variable=${1-$DEFAULT}                   #  What does
   echo "variable = $variable"              #+ parameter substitution show?
                                            #  ---------------------------
                                            #  It distinguishes between
                                            #+ no param and a null param.

   if [ "$2" ]
   then
     echo "-Parameter #2 is \"$2\".-"
   fi

   return 0
}

echo
   
echo "Nothing passed."   
func2                          # Called with no params
echo


echo "Zero-length parameter passed."
func2 ""                       # Called with zero-length param
echo

echo "Null parameter passed."
func2 "$uninitialized_param"   # Called with uninitialized param
echo

echo "One parameter passed."   
func2 first           # Called with one param
echo

echo "Two parameters passed."   
func2 first second    # Called with two params
echo

echo "\"\" \"second\" passed."
func2 "" second       # Called with zero-length first parameter
echo                  # and ASCII string as a second one.

exit 0
```

bash's `${1-$DEFAULT}` tells "no argument" from "an empty one". In tessh a
default of `None` does: an empty string is a value. (`$uninitialized_param`
would be a `NameError` in tessh, so an empty string stands in for it.)

```tessh
#!/usr/bin/env tessh
DEFAULT = default                             # Default param value.

def func2 p1=None p2=""
  if not $p1                                  # Is parameter #1 empty?
    print "-Parameter #1 is zero length.-"    # Or no parameter passed.
  else
    print "-Parameter #1 is \"$p1\".-"

  variable = if $p1 is None then $DEFAULT else $p1   #  What does
  print "variable = $variable"                       #+ this show?
                                              #  It distinguishes between
                                              #+ no param and a null param.

  if $p2
    print "-Parameter #2 is \"$p2\".-"

  return 0

print

print "Nothing passed."
func2                          # Called with no params
print


print "Zero-length parameter passed."
func2 ""                       # Called with zero-length param
print

print "Null parameter passed."
func2 ""                       # (Called with uninitialized param, in bash)
print

print "One parameter passed."
func2 first           # Called with one param
print

print "Two parameters passed."
func2 first second    # Called with two params
print

print "\"\" \"second\" passed."
func2 "" second       # Called with zero-length first parameter
print                 # and ASCII string as a second one.

exit 0
```

```output

Nothing passed.
-Parameter #1 is zero length.-
variable = default

Zero-length parameter passed.
-Parameter #1 is zero length.-
variable = 

Null parameter passed.
-Parameter #1 is zero length.-
variable = 

One parameter passed.
-Parameter #1 is "first".-
variable = first

Two parameters passed.
-Parameter #1 is "first".-
variable = first
-Parameter #2 is "second".-

"" "second" passed.
-Parameter #1 is zero length.-
variable = 
-Parameter #2 is "second".-

```

A function's `$1` is its own argument, not the script's: a function sees
the script's arguments only when they're passed to it. In tessh the
script's are in `$argv` everywhere, but a function's parameters are what
it's given.

### Example 24-3. Functions and command-line args passed to the script (`func-cmdlinearg.sh`)

```bash
#!/bin/bash
# func-cmdlinearg.sh
#  Call this script with a command-line argument,
#+ something like $0 arg1.


func ()

{
echo "$1"   # Echoes first arg passed to the function.
}           # Does a command-line arg qualify?

echo "First call to function: no arg passed."
echo "See if command-line arg is seen."
func
# No! Command-line arg not seen.

echo "============================================================"
echo
echo "Second call to function: command-line arg passed explicitly."
func $1
# Now it's seen!

exit 0
```

```output bash
First call to function: no arg passed.
See if command-line arg is seen.

============================================================

Second call to function: command-line arg passed explicitly.

```

```tessh
#!/usr/bin/env tessh
args = $argv[1:] or [arg1]    # (sample arguments, when none are given)

def func arg=""
  print $arg  # Prints first arg passed to the function.
              # Does a command-line arg qualify?

print "First call to function: no arg passed."
print "See if command-line arg is seen."
func
# No! Command-line arg not seen.

print "============================================================"
print
print "Second call to function: command-line arg passed explicitly."
func $args[0]
# Now it's seen!

exit 0
```

```output
First call to function: no arg passed.
See if command-line arg is seen.

============================================================

Second call to function: command-line arg passed explicitly.
arg1
```

bash passes a function only values: a variable's name passed to it is just
text. Indirect references (`${!name}`, chapter 28) let the function get at
the variable through its name. In tessh the script's variables are the
dict `(globals)`, so `(globals)[$name]` is the variable whose name is in
`name`. (Passing the value itself, or a list or dict to change, is the
usual way; see below.)

### Example 24-4. Passing an indirect reference to a function (`ind-func.sh`)

```bash
#!/bin/bash
# ind-func.sh: Passing an indirect reference to a function.

echo_var ()
{
echo "$1"
}

message=Hello
Hello=Goodbye

echo_var "$message"        # Hello
# Now, let's pass an indirect reference to the function.
echo_var "${!message}"     # Goodbye

echo "-------------"

# What happens if we change the contents of "hello" variable?
Hello="Hello, again!"
echo_var "$message"        # Hello
echo_var "${!message}"     # Hello, again!

exit 0
```

```tessh
#!/usr/bin/env tessh
def echo_var x
  print $x

message = Hello
Hello = Goodbye

echo_var $message                  # Hello
# Now, let's pass an indirect reference to the function.
echo_var (globals)[$message]       # Goodbye

print "-------------"

# What happens if we change the contents of "Hello" variable?
Hello = "Hello, again!"
echo_var $message                  # Hello
echo_var (globals)[$message]       # Hello, again!

exit 0
```

```output
Hello
Goodbye
-------------
Hello
Hello, again!
```

The function can change the variable it was given the name of, too: in
bash with `eval`, in tessh by assigning to `(globals)[$name]`.

### Example 24-5. Dereferencing a parameter passed to a function (`dereference.sh`)

```bash
#!/bin/bash
# dereference.sh
# Dereferencing parameter passed to a function.
# Script by Bruce W. Clare.

dereference ()
{
     y=\$"$1"   # Name of variable (not value!).
     echo $y    # $Junk

     x=`eval "expr \"$y\" "`
     echo $1=$x
     eval "$1=\"Some Different Text \""  # Assign new value.
}

Junk="Some Text"
echo $Junk "before"    # Some Text before

dereference Junk
echo $Junk "after"     # Some Different Text after

exit 0
```

```output bash
Some Text before
$Junk
Junk=Some Text
Some Different Text after
```

bash's unquoted `$Junk` loses the trailing space of the new value; tessh
passes a value as it is, so the space shows.

```tessh
#!/usr/bin/env tessh
def dereference name
  y = "\$$name"   # Name of variable (not value!).
  print $y        # $Junk

  x = (globals)[$name]
  print "$name=$x"
  (globals)[$name] = "Some Different Text "  # Assign new value.

Junk = "Some Text"
print $Junk "before"    # Some Text before

dereference Junk
print $Junk "after"     # Some Different Text after

exit 0
```

```output
Some Text before
$Junk
Junk=Some Text
Some Different Text  after
```

### Example 24-6. Again, dereferencing a parameter passed to a function (`ref-params.sh`)

```bash no-run
#!/bin/bash
# ref-params.sh: Dereferencing a parameter passed to a function.
#                (Complex Example)

ITERATIONS=3  # How many times to get input.
icount=1

my_read () {
  #  Called with my_read varname,
  #+ outputs the previous value between brackets as the default value,
  #+ then asks for a new value.

  local local_var

  echo -n "Enter a value "
  eval 'echo -n "[$'$1'] "'  #  Previous value.
# eval echo -n "[\$$1] "     #  Easier to understand,
                             #+ but loses trailing space in user prompt.
  read local_var
  [ -n "$local_var" ] && eval $1=\$local_var

  # "And-list": if "local_var" then set "$1" to its value.
}

echo

while [ "$icount" -le "$ITERATIONS" ]
do
  my_read var
  echo "Entry #$icount = $var"
  let "icount += 1"
  echo
done  


# Thanks to Stephane Chazelas for providing this instructive example.

exit 0
```

The answers come from a heredoc given to the loop's function (an empty
line keeps the old value), as there's no keyboard here. `input` prints its
prompt and reads a line, as `echo -n` and `read` do. The usual tessh way
gives the new value back, `var = (my_read $var)`; this one keeps the
original's passing of the name. (The variable is `entry`, not `var`:
`var` is one of tessh's own names.)

```tessh
#!/usr/bin/env tessh
ITERATIONS = 3  # How many times to get input.

def my_read name
  #  Called with my_read varname,
  #+ outputs the previous value between brackets as the default value,
  #+ then asks for a new value.
  g = (globals)
  local_var = input "Enter a value [$($g.get $name '')] "
  print $local_var                 # (What was typed, as the terminal shows it.)
  if $local_var
    $g[$name] = $local_var
  # If "local_var" then set the variable named in "name" to its value.

def ask_all
  for icount in (range 1 ($ITERATIONS + 1))
    my_read entry
    print "Entry #$icount = $entry"
    print

print
ask_all <<END
first

third
END
# (Three canned answers: the empty one keeps "first".)

exit 0
```

```output

Enter a value [] first
Entry #1 = first

Enter a value [first] 
Entry #2 = first

Enter a value [first] third
Entry #3 = third

```

### Exit and return

A bash function's result is its exit status: `return N`, or the status of
its last command, read from `$?`. That's a number from 0 to 255. A tessh
function gives back any value, with `return` or as its last form, and the
caller uses it like any other value. (A program's exit status is still a
number from 0 to 255: an `Output`'s `.code`, and `$?`.)

### Example 24-7. Maximum of two numbers (`max.sh`)

```bash
#!/bin/bash
# max.sh: Maximum of two integers.

E_PARAM_ERR=250    # If less than 2 params passed to function.
EQUAL=251          # Return value if both params equal.
#  Error values out of range of any
#+ params that might be fed to the function.

max2 ()             # Returns larger of two numbers.
{                   # Note: numbers compared must be less than 250.
if [ -z "$2" ]
then
  return $E_PARAM_ERR
fi

if [ "$1" -eq "$2" ]
then
  return $EQUAL
else
  if [ "$1" -gt "$2" ]
  then
    return $1
  else
    return $2
  fi
fi
}

max2 33 34
return_val=$?

if [ "$return_val" -eq $E_PARAM_ERR ]
then
  echo "Need to pass two parameters to the function."
elif [ "$return_val" -eq $EQUAL ]
  then
    echo "The two numbers are equal."
else
    echo "The larger of the two numbers is $return_val."
fi  

  
exit 0

#  Exercise (easy):
#  ---------------
#  Convert this to an interactive script,
#+ that is, have the script ask for input (two numbers).
```

The error values were chosen to be out of the range of the answers, which
had to fit in 0-255; in tessh any value will do, and `None` is the usual
"no answer". Here they're kept as they were.

```tessh
#!/usr/bin/env tessh
E_PARAM_ERR = 250    # If less than 2 params passed to function.
EQUAL = 251          # Return value if both params equal.

def max2 a=None b=None   # Returns larger of two numbers.
  if $b is None
    return $E_PARAM_ERR

  if $a == $b
    $EQUAL
  elif $a > $b
    $a
  else
    $b

return_val = max2 33 34

if $return_val == $E_PARAM_ERR
  print "Need to pass two parameters to the function."
elif $return_val == $EQUAL
  print "The two numbers are equal."
else
  print "The larger of the two numbers is $return_val."

exit 0
```

```output
The larger of the two numbers is 34.
```

To give back a string, a bash function sets a variable for it (`REPLY`
here) and returns a status saying whether it worked:

```bash no-run
count_lines_in_etc_passwd()
{
  [[ -r /etc/passwd ]] && REPLY=$(echo $(wc -l < /etc/passwd))
  #  If /etc/passwd is readable, set REPLY to line count.
  #  Returns both a parameter value and status information.
  #  The 'echo' seems unnecessary, but . . .
  #+ it removes excess whitespace from the output.
}

if count_lines_in_etc_passwd
then
  echo "There are $REPLY lines in /etc/passwd."
else
  echo "Cannot count lines in /etc/passwd."
fi  

# Thanks, S.C.
```

A tessh function gives back the count, or `None` when it can't. (Here on a
made-up password file.)

```tessh
import os
printf "root:x:0:0::/root:/bin/sh\nbozo:x:1000:100::/home/bozo:/bin/bash\n" >passwd

def count_lines file=/etc/passwd
  if os.access $file $os.R_OK
    len ((open $file).readlines)  # (wc -l <$file would do too.)

lines = count_lines passwd       # (/etc/passwd)
if $lines is not None
  print "There are $lines lines in passwd."
else
  print "Cannot count lines in passwd."
```

```output
There are 2 lines in passwd.
```

### Example 24-8. Converting numbers to Roman numerals (`ex61.sh`)

```bash no-run
#!/bin/bash

# Arabic number to Roman numeral conversion
# Range: 0 - 200
# It's crude, but it works.

# Extending the range and otherwise improving the script is left as an exercise.

# Usage: roman number-to-convert

LIMIT=200
E_ARG_ERR=65
E_OUT_OF_RANGE=66

if [ -z "$1" ]
then
  echo "Usage: `basename $0` number-to-convert"
  exit $E_ARG_ERR
fi  

num=$1
if [ "$num" -gt $LIMIT ]
then
  echo "Out of range!"
  exit $E_OUT_OF_RANGE
fi  

to_roman ()   # Must declare function before first call to it.
{
number=$1
factor=$2
rchar=$3
let "remainder = number - factor"
while [ "$remainder" -ge 0 ]
do
  echo -n $rchar
  let "number -= factor"
  let "remainder = number - factor"
done  

return $number
       # Exercises:
       # ---------
       # 1) Explain how this function works.
       #    Hint: division by successive subtraction.
       # 2) Extend to range of the function.
       #    Hint: use "echo" and command-substitution capture.
}
   

to_roman $num 100 C
num=$?
to_roman $num 90 LXXXX
num=$?
to_roman $num 50 L
num=$?
to_roman $num 40 XL
num=$?
to_roman $num 10 X
num=$?
to_roman $num 9 IX
num=$?
to_roman $num 5 V
num=$?
to_roman $num 4 IV
num=$?
to_roman $num 1 I
# Successive calls to conversion function!
# Is this really necessary??? Can it be simplified?

echo

exit
```

`to_roman` prints its letters and gives back what's left, as the original
returns it as its status; the calls pass it on. (The ABS Guide asks whether
nine calls are needed: a loop over the pairs is the answer in the
comment.)

```tessh
#!/usr/bin/env tessh
LIMIT = 200
E_ARG_ERR = 65
E_OUT_OF_RANGE = 66
args = $argv[1:] or ["189"]    # (sample arguments, when none are given)

if not $args
  print "Usage: $(Path $argv[0]).name number-to-convert"
  exit $E_ARG_ERR

num = int $args[0]
if $num > $LIMIT
  print "Out of range!"
  exit $E_OUT_OF_RANGE

def to_roman number factor rchar   # Must declare function before first call to it.
  remainder = $number - $factor
  while $remainder >= 0
    print $rchar end=""
    number -= $factor
    remainder = $number - $factor
  $number

num = to_roman $num 100 C
num = to_roman $num 90 LXXXX
num = to_roman $num 50 L
num = to_roman $num 40 XL
num = to_roman $num 10 X
num = to_roman $num 9 IX
num = to_roman $num 5 V
num = to_roman $num 4 IV
to_roman $num 1 I
# Successive calls to conversion function!
# Is this really necessary??? Can it be simplified?
#   for [factor rchar] in [[100 C] [90 LXXXX] [50 L] … [1 I]]
#     num = to_roman $num $factor $rchar

print

exit
```

```output
CLXXXIX
```

bash's 255 limit, and what happens past it:

### Example 24-9. Testing large return values in a function (`return-test.sh`)

```bash
#!/bin/bash
# return-test.sh

# The largest positive value a function can return is 255.

return_test ()         # Returns whatever passed to it.
{
  return $1
}

return_test 27         # o.k.
echo $?                # Returns 27.
  
return_test 255        # Still o.k.
echo $?                # Returns 255.

return_test 257        # Error!
echo $?                # Returns 1 (return code for miscellaneous error).

# =========================================================
return_test -151896    # Do large negative numbers work?
echo $?                # Will this return -151896?
                       # No! It returns 168.
#  Version of Bash before 2.05b permitted
#+ large negative integer return values.
#  It happened to be a useful feature.
#  Newer versions of Bash unfortunately plug this loophole.
#  This may break older scripts.
#  Caution!
# =========================================================

exit 0
```

```output bash
27
255
1
168
```

tessh has no such limit on a function's value; the limit is a program's
exit status, as the last lines show.

```tessh
#!/usr/bin/env tessh
def return_test n   # Returns whatever passed to it.
  return $n

print (return_test 27)        # 27
print (return_test 255)       # 255
print (return_test 257)       # 257
print (return_test -151896)   # -151896

# A program's exit status is still a byte:
r = sh -c "exit 257"
print $r.code                 # 1

exit 0
```

```output
27
255
257
-151896
1
```

The bash workaround for a larger value is a global variable to put it in:

```bash
Return_Val=   # Global variable to hold oversize return value of function.

alt_return_test ()
{
  fvar=$1
  Return_Val=$fvar
  return   # Returns 0 (success).
}

alt_return_test 1
echo $?                              # 0
echo "return value = $Return_Val"    # 1

alt_return_test 256
echo "return value = $Return_Val"    # 256

alt_return_test 257
echo "return value = $Return_Val"    # 257

alt_return_test 25701
echo "return value = $Return_Val"    #25701
```

```output bash
0
return value = 1
return value = 256
return value = 257
return value = 25701
```

In tessh it's not needed, but it works the same with `global`:

```tessh
Return_Val = None   # Global variable to hold the return value of function.

def alt_return_test fvar
  global Return_Val
  Return_Val = $fvar

alt_return_test 1
print "return value = $Return_Val"    # 1

alt_return_test 256
print "return value = $Return_Val"    # 256

alt_return_test 25701
print "return value = $Return_Val"    # 25701
```

```output
return value = 1
return value = 256
return value = 25701
```

The more elegant bash way is to have the function print its value and
catch it with command substitution, `$(func …)`.

### Example 24-10. Comparing two large integers (`max2.sh`)

```bash
#!/bin/bash
# max2.sh: Maximum of two LARGE integers.

#  This is the previous "max.sh" example,
#+ modified to permit comparing large integers.

EQUAL=0             # Return value if both params equal.
E_PARAM_ERR=-99999  # Not enough params passed to function.
#           ^^^^^^    Out of range of any params that might be passed.

max2 ()             # "Returns" larger of two numbers.
{
if [ -z "$2" ]
then
  echo $E_PARAM_ERR
  return
fi

if [ "$1" -eq "$2" ]
then
  echo $EQUAL
  return
else
  if [ "$1" -gt "$2" ]
  then
    retval=$1
  else
    retval=$2
  fi
fi

echo $retval        # Echoes (to stdout), rather than returning value.
                    # Why?
}


return_val=$(max2 33001 33997)
#            ^^^^             Function name
#                 ^^^^^ ^^^^^ Params passed
#  This is actually a form of command substitution:
#+ treating a function as if it were a command,
#+ and assigning the stdout of the function to the variable "return_val."


# ========================= OUTPUT ========================
if [ "$return_val" -eq "$E_PARAM_ERR" ]
  then
  echo "Error in parameters passed to comparison function!"
elif [ "$return_val" -eq "$EQUAL" ]
  then
    echo "The two numbers are equal."
else
    echo "The larger of the two numbers is $return_val."
fi
# =========================================================
  
exit 0

#  Exercises:
#  ---------
#  1) Find a more elegant way of testing
#+    the parameters passed to the function.
#  2) Simplify the if/then structure at "OUTPUT."
#  3) Rewrite the script to take input from command-line parameters.
```

In tessh the function returns the value: what bash does with `echo` and
`$(…)` is what a tessh function does anyway.

```tessh
#!/usr/bin/env tessh
EQUAL = 0             # Return value if both params equal.
E_PARAM_ERR = -99999  # Not enough params passed to function.

def max2 a=None b=None    # Returns larger of two numbers.
  if $b is None
    return $E_PARAM_ERR

  if $a == $b
    return $EQUAL
  if $a > $b then $a else $b


return_val = max2 33001 33997

# ========================= OUTPUT ========================
if $return_val == $E_PARAM_ERR
  print "Error in parameters passed to comparison function!"
elif $return_val == $EQUAL
  print "The two numbers are equal."
else
  print "The larger of the two numbers is $return_val."
# =========================================================

exit 0
```

```output
The larger of the two numbers is 33997.
```

Another function whose answer is caught from its output, with a parameter
passed on into an `awk` program:

```bash
month_length ()  # Takes month number as an argument.
{                # Returns number of days in month.
monthD="31 28 31 30 31 30 31 31 30 31 30 31"  # Declare as local?
echo "$monthD" | awk '{ print $'"${1}"' }'    # Tricky.
#                             ^^^^^^^^^
# Parameter passed to function  ($1 -- month number), then to awk.
# Awk sees this as "print $1 . . . print $12" (depending on month number)
# Template for passing a parameter to embedded awk script:
#                                 $'"${script_parameter}"'

#    Here's a slightly simpler awk construct:
#    echo $monthD | awk -v month=$1 '{print $(month)}'
#    Uses the -v awk option, which assigns a variable value
#+   prior to execution of the awk program block.
#    Thank you, Rich.

#  Needs error checking for correct parameter range (1-12)
#+ and for February in leap year.
}

# ----------------------------------------------
# Usage example:
month=4        # April, for example (4th month).
days_in=$(month_length $month)
echo $days_in  # 30
# ----------------------------------------------
```

```output bash
30
```

In tessh it's a list and an index, no `awk` needed. (Python's `calendar`
knows about leap years: `calendar.monthrange 2024 2` gives `(3, 29)`.)

```tessh
def month_length month   # Takes month number as an argument.
  monthD = [31 28 31 30 31 30 31 31 30 31 30 31]
  $monthD[$month - 1]    # Returns number of days in month.

# Usage example:
month = 4        # April, for example (4th month).
days_in = month_length $month
print $days_in   # 30

# The awk way works too, with -v:
"31 28 31 30 31 30 31 31 30 31 30 31" | awk -v month=$month '{print $(month)}'
```

```output
30
30
```

### Redirection

A function's standard input can be redirected: in bash, by putting the
redirection after the function's body, so it applies at every call. In
tessh the redirection goes on the call, and the function's `input`,
`sys.stdin` and the programs it runs read from it.

### Example 24-11. Real name from username (`realname.sh`)

```bash no-run
#!/bin/bash
# realname.sh
#
# From username, gets "real name" from /etc/passwd.


ARGCOUNT=1       # Expect one arg.
E_WRONGARGS=85

file=/etc/passwd
pattern=$1

if [ $# -ne "$ARGCOUNT" ]
then
  echo "Usage: `basename $0` USERNAME"
  exit $E_WRONGARGS
fi  

file_excerpt ()    #  Scan file for pattern,
{                  #+ then print relevant portion of line.
  while read line  # "while" does not necessarily need [ condition ]
  do
    echo "$line" | grep $1 | awk -F":" '{ print $5 }'
    # Have awk use ":" delimiter.
  done
} <$file  # Redirect into function's stdin.

file_excerpt $pattern

# Yes, this entire script could be reduced to
#       grep PATTERN /etc/passwd | awk -F":" '{ print $5 }'
# or
#       awk -F: '/PATTERN/ {print $5}'
# or
#       awk -F: '($1 == "username") { print $5 }' # real name from username
# However, it might not be as instructive.

exit 0
```

Here on a made-up password file. Each line goes through `grep` and `awk`,
as in the original, with the line written into the pipe as its first stage.

```tessh
#!/usr/bin/env tessh
import sys
# (A made-up /etc/passwd to try it on.)
cat <<END >passwd
root:x:0:0:Super User:/root:/bin/sh
bozo:x:1000:100:Bozo Bozeman:/home/bozo:/bin/bash
ann:x:1001:100:Ann Smith:/home/ann:/bin/bash
END

ARGCOUNT = 1       # Expect one arg.
E_WRONGARGS = 85

file = passwd      # (/etc/passwd)
args = $argv[1:] or [bozo]   # (sample arguments, when none are given)

if (len $args) != $ARGCOUNT
  print "Usage: $(Path $argv[0]).name USERNAME"
  exit $E_WRONGARGS

[pattern] = $args

def file_excerpt pattern    #  Scan file for pattern,
                            #+ then print relevant portion of line.
  for line in $sys.stdin
    $line | grep $pattern | awk -F":" '{ print $5 }'
    # Have awk use ":" delimiter.

file_excerpt $pattern <$file  # Redirect into function's stdin.

# Yes, this entire script could be reduced to
#       grep PATTERN passwd | awk -F":" '{ print $5 }'
# or, in tessh,
#       for line in (open passwd) ...  ($line.split ":")[4]

exit 0
```

```output
Bozo Bozeman
```

bash also allows a redirection on a block inside the function. A pipe from
a function needs that: `Function () { echo $*; } | tr a b` pipes the
definition, not the output. In tessh the pipe goes inside the function.

```bash no-run
# Instead of:
Function ()
{
 ...
 } < file

# Try this:
Function ()
{
  {
    ...
   } < file
}

# Similarly,

Function ()  # This works.
{
  {
   echo $*
  } | tr a b
}

Function ()  # This doesn't work.
{
  echo $*
} | tr a b   # A nested code block is mandatory here.

# Thanks, S.C.
```

```tessh
def Function *words
  echo *$words | tr a b

Function a cab bad
```

```output
b cbb bbd
```

## 24.2 Local Variables
<!-- help: abs-localvar local global scope -->

In bash a variable set in a function is global unless declared `local`. In
tessh, as in Python, it's the other way: a name assigned in a function is
the function's own, unless the function says `global name`. A function can
read the script's variables without saying anything.

### Example 24-12. Local variable visibility (`ex62.sh`)

```bash
#!/bin/bash
# ex62.sh: Global and local variables inside a function.

func ()
{
  local loc_var=23       # Declared as local variable.
  echo                   # Uses the 'local' builtin.
  echo "\"loc_var\" in function = $loc_var"
  global_var=999         # Not declared as local.
                         # Therefore, defaults to global. 
  echo "\"global_var\" in function = $global_var"
}  

func

# Now, to see if local variable "loc_var" exists outside the function.

echo
echo "\"loc_var\" outside function = $loc_var"
                                      # $loc_var outside function = 
                                      # No, $loc_var not visible globally.
echo "\"global_var\" outside function = $global_var"
                                      # $global_var outside function = 999
                                      # $global_var is visible globally.
echo				      

exit 0
#  In contrast to C, a Bash variable declared inside a function
#+ is local ONLY if declared as such.
```

Reading a name that isn't set is a `NameError` in tessh, so the outside
reads go through `(globals).get`, which gives a default.

```tessh
#!/usr/bin/env tessh
def func
  loc_var = 23           # Local: assigned in the function.
  print
  print "\"loc_var\" in function = $loc_var"
  global global_var      # Declared global.
  global_var = 999
  print "\"global_var\" in function = $global_var"

func

# Now, to see if local variable "loc_var" exists outside the function.

print
print "\"loc_var\" outside function = $((globals).get 'loc_var' '')"
                                      # No, loc_var not visible globally.
print "\"global_var\" outside function = $global_var"
                                      # global_var outside function = 999
print

exit 0
#  As in C, and unlike bash, a tessh variable set inside a function
#+ is local unless declared otherwise.
```

```output

"loc_var" in function = 23
"global_var" in function = 999

"loc_var" outside function = 
"global_var" outside function = 999

```

A bash `local` is seen by the functions its function calls: bash's scope
is dynamic. tessh's, Python's, is lexical: a function sees its own names
and the script's, not its caller's.

```bash
#!/bin/bash

function1 ()
{
  local func1var=20

  echo "Within function1, \$func1var = $func1var."

  function2
}

function2 ()
{
  echo "Within function2, \$func1var = $func1var."
}

function1

exit 0

# Output of the script:

# Within function1, $func1var = 20.
# Within function2, $func1var = 20.
```

```output bash
Within function1, $func1var = 20.
Within function2, $func1var = 20.
```

```tessh
#!/usr/bin/env tessh
def function1
  func1var = 20

  print "Within function1, \$func1var = $func1var."

  function2

def function2
  try
    print "Within function2, \$func1var = $func1var."
  except NameError
    print "Within function2, func1var isn't seen."

function1

exit 0
```

```output
Within function1, $func1var = 20.
Within function2, func1var isn't seen.
```

To give a function a value, pass it. (A function defined *inside* another
does see the outer one's names; that's lexical scope too.)

In bash, before a function has run, none of the variables it sets exist,
global or not. The same holds in tessh for a function's `global` names:

```bash
#!/bin/bash

func ()
{
global_var=37    #  Visible only within the function block
                 #+ before the function has been called. 
}                #  END OF FUNCTION

echo "global_var = $global_var"  # global_var =
                                 #  Function "func" has not yet been called,
                                 #+ so $global_var is not visible here.

func
echo "global_var = $global_var"  # global_var = 37
                                 # Has been set by function call.
```

```tessh
#!/usr/bin/env tessh
def func
  global global_var
  global_var = 37  #  Not there at all
                   #+ before the function has been called.

print "global_var = $((globals).get 'global_var' '')"  # global_var =
                                 #  Function "func" has not yet been called.

func
print "global_var = $global_var"  # global_var = 37
                                  # Has been set by function call.
```

```output
global_var = 
global_var = 37
```

The ABS Guide notes a bash oddity: `local t1=$(exit 1)` leaves `$?` at 0,
the status of `local`, not of the command. A tessh assignment from a
command keeps the status on the value: `t1 = sh -c "exit 1"`, then
`$t1.code` is 1 (and so is `$?`).

### 24.2.1 Local variables and recursion

A recursive function calls itself. Each call needs its own copy of the
variables that describe its part of the work; in bash that means `local`,
in tessh it's how every function's names are.

### Example 24-13. Demonstration of a simple recursive function (`recursion-demo.sh`)

```bash
#!/bin/bash
# recursion-demo.sh
# Demonstration of recursion.

RECURSIONS=9   # How many times to recurse.
r_count=0      # Must be global. Why?

recurse ()
{
  var="$1"

  while [ "$var" -ge 0 ]
  do
    echo "Recursion count = "$r_count"  +-+  \$var = "$var""
    (( var-- )); (( r_count++ ))
    recurse "$var"  #  Function calls itself (recurses)
  done              #+ until what condition is met?
}

recurse $RECURSIONS

exit $?
```

In the bash, `var` is global, shared by every level, so when the deepest
call takes it below 0, every loop above ends too. tessh's `var` would be
each call's own, and the loops would go on at every level, printing
hundreds of lines; to do what the original does, it's declared `global`.

```tessh
#!/usr/bin/env tessh
RECURSIONS = 9   # How many times to recurse.
r_count = 0      # Must be global. Why?

def recurse n
  global var r_count
  var = $n

  while $var >= 0
    print "Recursion count = $r_count  +-+  \$var = $var"
    var -= 1 ; r_count += 1
    recurse $var    #  Function calls itself (recurses)
                    #+ until what condition is met?

recurse $RECURSIONS

exit $?
```

```output
Recursion count = 0  +-+  $var = 9
Recursion count = 1  +-+  $var = 8
Recursion count = 2  +-+  $var = 7
Recursion count = 3  +-+  $var = 6
Recursion count = 4  +-+  $var = 5
Recursion count = 5  +-+  $var = 4
Recursion count = 6  +-+  $var = 3
Recursion count = 7  +-+  $var = 2
Recursion count = 8  +-+  $var = 1
Recursion count = 9  +-+  $var = 0
```

### Example 24-14. Another simple demonstration (`recursion-def.sh`)

```bash
#!/bin/bash
# recursion-def.sh
# A script that defines "recursion" in a rather graphic way.

RECURSIONS=10
r_count=0
sp=" "

define_recursion ()
{
  ((r_count++))
  sp="$sp"" "
  echo -n "$sp"
  echo "\"The act of recurring ... \""   # Per 1913 Webster's dictionary.

  while [ $r_count -le $RECURSIONS ]
  do
    define_recursion
  done
}

echo
echo "Recursion: "
define_recursion
echo

exit $?
```

```tessh
#!/usr/bin/env tessh
RECURSIONS = 10
r_count = 0
sp = " "

def define_recursion
  global r_count sp
  r_count += 1
  sp = $sp + " "
  print $sp end=""
  print "\"The act of recurring ... \""   # Per 1913 Webster's dictionary.

  while $r_count <= $RECURSIONS
    define_recursion

print
print "Recursion: "
define_recursion
print

exit $?
```

```output

Recursion: 
  "The act of recurring ... "
   "The act of recurring ... "
    "The act of recurring ... "
     "The act of recurring ... "
      "The act of recurring ... "
       "The act of recurring ... "
        "The act of recurring ... "
         "The act of recurring ... "
          "The act of recurring ... "
           "The act of recurring ... "
            "The act of recurring ... "

```

Recursion is slow in a shell, and a deep one can crash bash with a
segfault, as the ABS Guide's warning script shows (it recurses 50,000
levels). Python stops at its recursion limit (1000 calls, by default) with
a `RecursionError` you can catch.

```bash no-run
#!/bin/bash

#  Warning: Running this script could possibly lock up your system!
#  If you're lucky, it will segfault before using up all available memory.

recursive_function ()		   
{
echo "$1"     # Makes the function do something, and hastens the segfault.
(( $1 < $2 )) && recursive_function $(( $1 + 1 )) $2;
#  As long as 1st parameter is less than 2nd,
#+ increment 1st and recurse.
}

recursive_function 1 50000  # Recurse 50,000 levels!
#  Most likely segfaults (depending on stack size, set by ulimit -m).

#  Recursion this deep might cause even a C program to segfault,
#+ by using up all the memory allotted to the stack.

echo "This will probably not print."
exit 0  # This script will not exit normally.

#  Thanks, Stéphane Chazelas.
```

```tessh
#!/usr/bin/env tessh
def recursive_function a b
  if $a < $b
    recursive_function ($a + 1) $b

try
  recursive_function 1 50000  # Recurse 50,000 levels!
except RecursionError as e
  print "Stopped: $e"

print "This does print."
```

```output
Stopped: maximum recursion depth exceeded
This does print.
```

### Example 24-15. Recursion, using a local variable (`ex63.sh`)

```bash no-run
#!/bin/bash

#               factorial
#               ---------


# Does bash permit recursion?
# Well, yes, but...
# It's so slow that you gotta have rocks in your head to try it.


MAX_ARG=5
E_WRONG_ARGS=85
E_RANGE_ERR=86


if [ -z "$1" ]
then
  echo "Usage: `basename $0` number"
  exit $E_WRONG_ARGS
fi

if [ "$1" -gt $MAX_ARG ]
then
  echo "Out of range ($MAX_ARG is maximum)."
  #  Let's get real now.
  #  If you want greater range than this,
  #+ rewrite it in a Real Programming Language.
  exit $E_RANGE_ERR
fi  

fact ()
{
  local number=$1
  #  Variable "number" must be declared as local,
  #+ otherwise this doesn't work.
  if [ "$number" -eq 0 ]
  then
    factorial=1    # Factorial of 0 = 1.
  else
    let "decrnum = number - 1"
    fact $decrnum  # Recursive function call (the function calls itself).
    let "factorial = $number * $?"
  fi

  return $factorial
}

fact $1
echo "Factorial of $1 is $?."

exit 0
```

The bash factorial gives its answer back as a status, so it can't go past
5 (120; 6! is 720, over 255). A tessh function's value has no such limit;
the limit is kept, raised to 20, to keep the check.

```tessh
#!/usr/bin/env tessh
MAX_ARG = 20
E_WRONG_ARGS = 85
E_RANGE_ERR = 86
args = $argv[1:] or ["20"]   # (sample arguments, when none are given)

if not $args
  print "Usage: $(Path $argv[0]).name number"
  exit $E_WRONG_ARGS

if (int $args[0]) > $MAX_ARG
  print "Out of range ($MAX_ARG is maximum)."
  exit $E_RANGE_ERR

def fact number
  #  "number" is each call's own, as every function's names are.
  if $number == 0
    1    # Factorial of 0 = 1.
  else
    $number * (fact ($number - 1))   # Recursive function call.

print "Factorial of $args[0] is $(fact (int $args[0]))."

exit 0
```

```output
Factorial of 20 is 2432902008176640000.
```

## 24.3 Recursion Without Local Variables
<!-- help: abs-recurnolocvar recursion fibonacci hanoi -->

A function can recurse without local variables, passing what it needs as
arguments and giving back its answer.

### Example 24-16. The Fibonacci Sequence (`fibo.sh`)

```bash
#!/bin/bash
# fibo.sh : Fibonacci sequence (recursive)
# Author: M. Cooper
# License: GPL3

# ----------algorithm--------------
# Fibo(0) = 0
# Fibo(1) = 1
# else
#   Fibo(j) = Fibo(j-1) + Fibo(j-2)
# ---------------------------------

MAXTERM=15       # Number of terms (+1) to generate.
MINIDX=2         # If idx is less than 2, then Fibo(idx) = idx.

Fibonacci ()
{
  idx=$1   # Doesn't need to be local. Why not?
  if [ "$idx" -lt "$MINIDX" ]
  then
    echo "$idx"  # First two terms are 0 1 ... see above.
  else
    (( --idx ))  # j-1
    term1=$( Fibonacci $idx )   #  Fibo(j-1)

    (( --idx ))  # j-2
    term2=$( Fibonacci $idx )   #  Fibo(j-2)

    echo $(( term1 + term2 ))
  fi
  #  An ugly, ugly kludge.
  #  The more elegant implementation of recursive fibo in C
  #+ is a straightforward translation of the algorithm in lines 7 - 10.
}

for i in $(seq 0 $MAXTERM)
do  # Calculate $MAXTERM+1 terms.
  FIBO=$(Fibonacci $i)
  echo -n "$FIBO "
done
# 0 1 1 2 3 5 8 13 21 34 55 89 144 233 377 610
# Takes a while, doesn't it? Recursion in a script is slow.

echo

exit 0
```

The bash version catches each answer with `$(…)`, a subshell per call. The
tessh one returns it, and is quick. (`functools.cache` would make it
quicker still: see the guide's Functions section.)

```tessh
#!/usr/bin/env tessh
# ----------algorithm--------------
# Fibo(0) = 0
# Fibo(1) = 1
# else
#   Fibo(j) = Fibo(j-1) + Fibo(j-2)
# ---------------------------------

MAXTERM = 15       # Number of terms (+1) to generate.
MINIDX = 2         # If idx is less than 2, then Fibo(idx) = idx.

def Fibonacci idx
  if $idx < $MINIDX
    $idx  # First two terms are 0 1 ... see above.
  else
    (Fibonacci ($idx - 1)) + (Fibonacci ($idx - 2))

for i in (range ($MAXTERM + 1))
  # Calculate $MAXTERM+1 terms.
  FIBO = Fibonacci $i
  print $FIBO end=" "
# 0 1 1 2 3 5 8 13 21 34 55 89 144 233 377 610

print

exit 0
```

```output
0 1 1 2 3 5 8 13 21 34 55 89 144 233 377 610 
```

### Example 24-17. The Towers of Hanoi (`hanoi.bash`)

```bash no-run
#! /bin/bash
#
# The Towers Of Hanoi
# Bash script
# Copyright (C) 2000 Amit Singh. All Rights Reserved.
# http://hanoi.kernelthread.com
#
# Tested under Bash version 2.05b.0(13)-release.
# Also works under Bash version 3.x.
#
#  Used in "Advanced Bash Scripting Guide"
#+ with permission of script author.
#  Slightly modified and commented by ABS author.

#=================================================================#
#  The Tower of Hanoi is a mathematical puzzle attributed to
#+ Edouard Lucas, a nineteenth-century French mathematician.
#
#  There are three vertical posts set in a base.
#  The first post has a set of annular rings stacked on it.
#  These rings are disks with a hole drilled out of the center,
#+ so they can slip over the posts and rest flat.
#  The rings have different diameters, and they stack in ascending
#+ order, according to size.
#  The smallest ring is on top, and the largest on the bottom.
#
#  The task is to transfer the stack of rings
#+ to one of the other posts.
#  You can move only one ring at a time to another post.
#  You are permitted to move rings back to the original post.
#  You may place a smaller ring atop a larger one,
#+ but *not* vice versa.
#  Again, it is forbidden to place a larger ring atop a smaller one.
#
#  For a small number of rings, only a few moves are required.
#+ For each additional ring,
#+ the required number of moves approximately doubles,
#+ and the "strategy" becomes increasingly complicated.
#
#  For more information, see http://hanoi.kernelthread.com
#+ or pp. 186-92 of _The Armchair Universe_ by A.K. Dewdney.
#
#
#         ...                   ...                    ...
#         | |                   | |                    | |
#        _|_|_                  | |                    | |
#       |_____|                 | |                    | |
#      |_______|                | |                    | |
#     |_________|               | |                    | |
#    |___________|              | |                    | |
#   |             |             | |                    | |
# .--------------------------------------------------------------.
# |**************************************************************|
#          #1                   #2                      #3
#
#=================================================================#


E_NOPARAM=66  # No parameter passed to script.
E_BADPARAM=67 # Illegal number of disks passed to script.
Moves=        # Global variable holding number of moves.
              # Modification to original script.

dohanoi() {   # Recursive function.
    case $1 in
    0)
        ;;
    *)
        dohanoi "$(($1-1))" $2 $4 $3
        echo move $2 "-->" $3
        ((Moves++))          # Modification to original script.
        dohanoi "$(($1-1))" $4 $3 $2
        ;;
    esac
}

case $# in
    1) case $(($1>0)) in     # Must have at least one disk.
       1)  # Nested case statement.
           dohanoi $1 1 3 2
           echo "Total moves = $Moves"   # 2^n - 1, where n = # of disks.
           exit 0;
           ;;
       *)
           echo "$0: illegal value for number of disks";
           exit $E_BADPARAM;
           ;;
       esac
    ;;
    *)
       echo "usage: $0 N"
       echo "       Where \"N\" is the number of disks."
       exit $E_NOPARAM;
       ;;
esac

# Exercises:
# ---------
# 1) Would commands beyond this point ever be executed?
#    Why not? (Easy)
# 2) Explain the workings of the workings of the "dohanoi" function.
#    (Difficult -- see the Dewdney reference, above.)
```

`case` is `match`, the nested one included. `Moves` is the script's, so
`dohanoi` says `global`.

```tessh
#!/usr/bin/env tessh
#
# The Towers Of Hanoi
# Copyright (C) 2000 Amit Singh. All Rights Reserved.
# http://hanoi.kernelthread.com
#
#  (The long description and picture are in the bash version above.)

E_NOPARAM = 66  # No parameter passed to script.
E_BADPARAM = 67 # Illegal number of disks passed to script.
Moves = 0       # Global variable holding number of moves.
args = $argv[1:] or ["3"]    # (sample arguments, when none are given)

def dohanoi n src dst spare   # Recursive function.
  global Moves
  match $n
    case 0
      pass
    case _
      dohanoi ($n - 1) $src $spare $dst
      print move $src "-->" $dst
      Moves += 1
      dohanoi ($n - 1) $spare $dst $src

match $args
  case [$disks]
    match (int $disks) > 0      # Must have at least one disk.
      case True                 # Nested match statement.
        dohanoi (int $disks) 1 3 2
        print "Total moves = $Moves"   # 2^n - 1, where n = # of disks.
        exit 0
      case _
        print "$(Path $argv[0]).name: illegal value for number of disks"
        exit $E_BADPARAM
  case _
    print "usage: $(Path $argv[0]).name N"
    print "       Where \"N\" is the number of disks."
    exit $E_NOPARAM
```

```output
move 1 --> 3
move 1 --> 2
move 3 --> 2
move 1 --> 3
move 2 --> 1
move 2 --> 3
move 1 --> 3
Total moves = 7
```
