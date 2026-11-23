# 8. Operations and Related Topics
<!-- help: abs-operations abs-8 operations operators arithmetic -->

Bash's arithmetic lives in special places: `let`, `$(( … ))`, `(( … ))`,
`expr`. Outside them, `1+2` is just text. tessh's arithmetic is Python's
and works anywhere a value does, on integers of any size, floats, and the
rest of Python's numbers.

## 8.1 Operators
<!-- help: abs-ops operators assignment arithmetic bitwise logical comma += ** % -->

**Assignment.** `=` assigns, to any kind of value. In bash `=` also tests
string equality inside `[ ]`; tessh's test is `==`, so the two never mix.

```bash
var=27
category=minerals  # No spaces allowed after the "=".
string1=rock ; string2=rock
if [ "$string1" = "$string2" ]
then
   echo "same"
fi
```

```tessh
var = 27
category = minerals  # Spaces are fine.
string1 = rock ; string2 = rock
if $string1 == $string2
  print "same"
```

```output
same
```

**Arithmetic.** `+`, `-`, `*`, `/`, `**` (power) and `%` (remainder), in
both, plus `//` in tessh. Bash's `/` drops the fraction; Python's `/` keeps
it and `//` drops it (rounding down, so `-7 // 2` is -4 where bash's `-7 /
2` is -3). `%` takes the sign of the divisor in Python and of the dividend
in bash, so the two differ for negative numbers too.

```bash
let "z=5**3"    # 5 * 5 * 5
echo "z = $z"   # z = 125
expr 5 % 3
echo $(( 7 / 2 )) $(( -7 / 2 )) $(( -7 % 3 ))
```

```output bash
z = 125
2
3 -3 -1
```

```tessh
z = 5 ** 3      # 5 * 5 * 5
print "z = $z"  # z = 125
print (5 % 3)
print (7 / 2) (7 // 2) (-7 // 2) (-7 % 3)
```

```output
z = 125
2
3.5 3 -4 2
```

The remainder turns up in random ranges, formatting and primes, and in
Euclid's algorithm.

### Example 8-1. Greatest common divisor (`gcd.sh`)

```bash no-run
#!/bin/bash
# gcd.sh: greatest common divisor
#         Uses Euclid's algorithm

#  The "greatest common divisor" (gcd) of two integers
#+ is the largest integer that will divide both, leaving no remainder.

#  Euclid's algorithm uses successive division.
#    In each pass,
#+      dividend <---  divisor
#+      divisor  <---  remainder
#+   until remainder = 0.
#    The gcd = dividend, on the final pass.
#
#  For an excellent discussion of Euclid's algorithm, see
#+ Jim Loy's site, http://www.jimloy.com/number/euclids.htm.


# ------------------------------------------------------
# Argument check
ARGS=2
E_BADARGS=85

if [ $# -ne "$ARGS" ]
then
  echo "Usage: `basename $0` first-number second-number"
  exit $E_BADARGS
fi
# ------------------------------------------------------


gcd ()
{

  dividend=$1             #  Arbitrary assignment.
  divisor=$2              #! It doesn't matter which of the two is larger.
                          #  Why not?

  remainder=1             #  If an uninitialized variable is used inside
                          #+ test brackets, an error message results.

  until [ "$remainder" -eq 0 ]
  do    #  ^^^^^^^^^^  Must be previously initialized!
    let "remainder = $dividend % $divisor"
    dividend=$divisor     # Now repeat with 2 smallest numbers.
    divisor=$remainder
  done                    # Euclid's algorithm

}                         # Last $dividend is the gcd.


gcd $1 $2

echo; echo "GCD of $1 and $2 = $dividend"; echo


# Exercises :
# ---------
# 1) Check command-line arguments to make sure they are integers,
#+   and exit the script with an appropriate error message if not.
# 2) Rewrite the gcd () function to use local variables.

exit 0
```

The function's variables are local, as the second exercise asks, and it
returns the gcd rather than leaving it in a global. Two values swap in
one assignment, so the loop is one line. `int` does the first exercise:
an argument that isn't a whole number is a `ValueError`. (`math.gcd`
does the whole job.) Sample arguments stand in when there are none.

```tessh
#!/usr/bin/env tessh
import math
ARGS = 2
E_BADARGS = 85

args = $argv[1:] or ["42" "56"]        # (sample arguments)
if (len $args) != $ARGS
  print "Usage: $(Path $argv[0]).name first-number second-number"
  exit $E_BADARGS
try
  [first second] = [(int $a) for a in $args]
except ValueError
  print "Both arguments must be whole numbers."
  exit $E_BADARGS

def gcd dividend divisor
  # It doesn't matter which of the two is larger. Why not?
  while $divisor != 0
    dividend, divisor = $divisor, $dividend % $divisor   # Euclid's algorithm
  $dividend                    # The last dividend is the gcd.

print; print "GCD of $first and $second = $(gcd $first $second)"; print
print "(math.gcd agrees: $(math.gcd $first $second))"

exit 0
```

```output

GCD of 42 and 56 = 14

(math.gcd agrees: 14)
```

**Assignment operators.** `+=`, `-=`, `*=`, `/=` and `%=` change a
variable in place, in both; tessh adds `//=`, `**=` and the bitwise ones.
`+=` also joins strings (and lists), which is handy for adding to `$PATH`.

```bash
var=10
let "var += 5" ; echo $var
let "var *= 4" ; echo $var
p=/usr/bin
p+=:/opt/bin ; echo $p
```

```tessh
var = 10
var += 5 ; print $var
var *= 4 ; print $var
p = "/usr/bin"
p += ":/opt/bin" ; print $p
```

```output
15
60
/usr/bin:/opt/bin
```

### Example 8-2. Using Arithmetic Operations (`arith-ops.sh`)

```bash
#!/bin/bash
# Counting to 11 in 10 different ways.

n=1; echo -n "$n "

let "n = $n + 1"   # let "n = n + 1"  also works.
echo -n "$n "


: $((n = $n + 1))
#  ":" necessary because otherwise Bash attempts
#+ to interpret "$((n = $n + 1))" as a command.
echo -n "$n "

(( n = n + 1 ))
#  A simpler alternative to the method above.
#  Thanks, David Lombard, for pointing this out.
echo -n "$n "

n=$(($n + 1))
echo -n "$n "

: $[ n = $n + 1 ]
#  ":" necessary because otherwise Bash attempts
#+ to interpret "$[ n = $n + 1 ]" as a command.
#  Works even if "n" was initialized as a string.
echo -n "$n "

n=$[ $n + 1 ]
#  Works even if "n" was initialized as a string.
#* Avoid this type of construct, since it is obsolete and nonportable.
#  Thanks, Stephane Chazelas.
echo -n "$n "

# Now for C-style increment operators.
# Thanks, Frank Wang, for pointing this out.

let "n++"          # let "++n"  also works.
echo -n "$n "

(( n++ ))          # (( ++n ))  also works.
echo -n "$n "

: $(( n++ ))       # : $(( ++n )) also works.
echo -n "$n "

: $[ n++ ]         # : $[ ++n ] also works
echo -n "$n "

echo

exit 0
```

tessh has one obvious way, `n += 1`, and no `++`. The other ten ways here
are the ones Python and tessh offer, for the fun of it, as in the original.

```tessh
#!/usr/bin/env tessh
import operator
# Counting to 11 in 11 different ways.

n = 1; print "$n " end=""

n = $n + 1                 # Plain arithmetic.
print "$n " end=""

n += 1                     # Augmented assignment: the usual way.
print "$n " end=""

n = (+ $n 1)               # An operator at the head of parentheses.
print "$n " end=""

n = inc $n                 # Clojure's inc.
print "$n " end=""

n = operator.add $n 1      # Python's operator module.
print "$n " end=""

n = int (expr $n + 1)      # The expr program, its output read as a number.
print "$n " end=""

n = py "n + 1"             # Inline Python.
print "$n " end=""

n = sum [$n 1]             # A sum of a list.
print "$n " end=""

print "${(n := $n + 1)} " end=""   # The walrus, inside an expression.

n = $n.__add__ 1           # The method under it all.
print "$n " end=""

print

exit 0
```

```output
1 2 3 4 5 6 7 8 9 10 11 
```

Bash's integers were 32 bits once, and are 64 bits now; past the limit
they wrap around to negative numbers. Python's integers have no limit.

```bash
a=9223372036854775806
echo "a = $a"
let "a+=1"
echo "a = $a"
let "a+=1"           # increment "a" again, past the limit.
echo "a = $a"
```

```output bash
a = 9223372036854775806
a = 9223372036854775807
a = -9223372036854775808
```

```tessh
a = 9223372036854775806
print "a = $a"
a += 1
print "a = $a"
a += 1               # increment "a" again: no limit.
print "a = $a"
```

```output
a = 9223372036854775806
a = 9223372036854775807
a = 9223372036854775808
```

Bash has no floating point at all (`let "b = 1.5 + 1.3"` is a syntax
error); scripts call `bc`. Python's floats work as they are, and
`decimal` and `fractions` do exact arithmetic.

```tessh
from fractions import Fraction
a = 1.5
b = $a + 1.3
print "b = $b"
print (0.1 + 0.2) ((Fraction 1 10) + (Fraction 2 10))
```

```output
b = 2.8
0.30000000000000004 3/10
```

**Bitwise operators**: `<<`, `<<=`, `>>`, `>>=`, `&`, `&=`, `|`, `|=`,
`^`, `^=`, and `~` (not). tessh has them all, except that `~` is the home
folder, so bitwise not is `operator.invert`. A `|` with a program or a
function on its right is a pipe; with a number there, it's "or".

```bash
var=1
let "var <<= 2" ; echo $var
echo $(( 12 & 10 )) $(( 12 | 10 )) $(( 12 ^ 10 )) $(( ~5 )) $(( 256 >> 4 ))
```

```tessh
import operator
var = 1
var <<= 2 ; print $var
print (12 & 10) (12 | 10) (12 ^ 10) (operator.invert 5) (256 >> 4)
```

```output
4
8 14 6 -6 16
```

**Logical operators**: `!`, `&&` and `||` join tests and commands. tessh
has `not`, `and` and `or` (and `!`, `&&`, `||`, the same). Between commands
they go by exit status; between values, by truth.

```bash
condition1=yes ; condition2=yes
if [ $condition1 ] && [ $condition2 ]
then echo "both"
fi
if [[ $condition1 && $condition2 ]]    # Also works.
then echo "both again"
fi
```

```tessh
condition1 = yes ; condition2 = yes
if $condition1 and $condition2
  print "both"
if $condition1 && $condition2       # Also works.
  print "both again"
```

```output
both
both again
```

### Example 8-3. Compound Condition Tests Using && and || (`and-or.sh`)

```bash
#!/bin/bash

a=24
b=47

if [ "$a" -eq 24 ] && [ "$b" -eq 47 ]
then
  echo "Test #1 succeeds."
else
  echo "Test #1 fails."
fi

# ERROR:   if [ "$a" -eq 24 && "$b" -eq 47 ]
#+         attempts to execute  ' [ "$a" -eq 24 '
#+         and fails to finding matching ']'.
#
#  Note:  if [[ $a -eq 24 && $b -eq 24 ]]  works.
#  The double-bracket if-test is more flexible
#+ than the single-bracket version.       
#    (The "&&" has a different meaning in line 17 than in line 6.)
#    Thanks, Stephane Chazelas, for pointing this out.


if [ "$a" -eq 98 ] || [ "$b" -eq 47 ]
then
  echo "Test #2 succeeds."
else
  echo "Test #2 fails."
fi


#  The -a and -o options provide
#+ an alternative compound condition test.
#  Thanks to Patrick Callahan for pointing this out.


if [ "$a" -eq 24 -a "$b" -eq 47 ]
then
  echo "Test #3 succeeds."
else
  echo "Test #3 fails."
fi


if [ "$a" -eq 98 -o "$b" -eq 47 ]
then
  echo "Test #4 succeeds."
else
  echo "Test #4 fails."
fi


a=rhino
b=crocodile
if [ "$a" = rhino ] && [ "$b" = crocodile ]
then
  echo "Test #5 succeeds."
else
  echo "Test #5 fails."
fi

exit 0
```

Bash's two ways (`&&` between tests, `-a` inside one) are one in tessh:
`and` and `or` (spelled `&&` and `||` if you like), between any tests.

```tessh
#!/usr/bin/env tessh
a = 24
b = 47

if $a == 24 and $b == 47
  print "Test #1 succeeds."
else
  print "Test #1 fails."

if $a == 98 or $b == 47
  print "Test #2 succeeds."
else
  print "Test #2 fails."

if $a == 24 && $b == 47
  print "Test #3 succeeds."
else
  print "Test #3 fails."

if $a == 98 || $b == 47
  print "Test #4 succeeds."
else
  print "Test #4 fails."

a = rhino
b = crocodile
if $a == "rhino" and $b == "crocodile"   # (quoted: a bug, for now)
  print "Test #5 succeeds."
else
  print "Test #5 fails."

exit 0
```

```output
Test #1 succeeds.
Test #2 succeeds.
Test #3 succeeds.
Test #4 succeeds.
Test #5 succeeds.
```

In bash's arithmetic, `&&` and `||` give 1 or 0. Python's `and` and `or`
give one of the two values (the one that decided), which is often handier;
`bool` makes it `True` or `False`, and `int` of that 1 or 0.

```bash
echo $(( 1 && 2 )) $((3 && 0)) $((4 || 0)) $((0 || 0))
```

```output bash
1 0 1 0
```

```tessh
print (1 && 2) (3 && 0) (4 || 0) (0 || 0)
print (int (bool (1 and 2))) (int (bool (3 and 0))) (int (bool (4 or 0))) (int (bool (0 or 0)))
```

```output
2 0 4 0
1 0 1 0
```

**The comma operator** does several calculations and keeps the last. A
`do` block is tessh's nearest thing: its value is its last form's.

```bash
let "t1 = ((5 + 3, 7 - 1, 15 - 4))"
echo "t1 = $t1"           # t1 = 11
# Here t1 is set to the result of the last operation. Why?

let "t2 = ((a = 9, 15 / 3))"      # Set "a" and calculate "t2".
echo "t2 = $t2    a = $a"         # t2 = 5    a = 9
```

```tessh
t1 = do
  5 + 3
  7 - 1
  15 - 4
print "t1 = $t1"

t2 = do
  a = 9
  15 // 3
print "t2 = $t2    a = $a"
```

```output
t1 = 11
t2 = 5    a = 9
```

## 8.2 Numerical Constants
<!-- help: abs-numerical-constants numbers octal hex binary base int -->

A bash number is decimal, octal with a leading `0`, hex with `0x`, or in
any base from 2 to 64 as `BASE#NUMBER`. tessh's are Python's: `0o32`
(octal: a plain leading zero is an error, not octal), `0x32`, `0b101`, and
`int text base` reads text in any base from 2 to 36. Bash's bases above
36 (with `@` and `_` as digits) need a few lines of code.

### Example 8-4. Representation of numerical constants (`numbers.sh`)

```bash
#!/bin/bash
# numbers.sh: Representation of numbers in different bases.

# Decimal: the default
let "dec = 32"
echo "decimal number = $dec"             # 32
# Nothing out of the ordinary here.


# Octal: numbers preceded by '0' (zero)
let "oct = 032"
echo "octal number = $oct"               # 26
# Expresses result in decimal.
# --------- ------ -- -------


# Hexadecimal: numbers preceded by '0x' or '0X'
let "hex = 0x32"
echo "hexadecimal number = $hex"         # 50

echo $((0x9abc))                         # 39612
#     ^^      ^^   double-parentheses arithmetic expansion/evaluation
# Expresses result in decimal.



# Other bases: BASE#NUMBER
# BASE between 2 and 64.
# NUMBER must use symbols within the BASE range, see below.


let "bin = 2#111100111001101"
echo "binary number = $bin"              # 31181

let "b32 = 32#77"
echo "base-32 number = $b32"             # 231

let "b64 = 64#@_"
echo "base-64 number = $b64"             # 4031
# This notation only works for a limited range (2 - 64) of ASCII characters.
# 10 digits + 26 lowercase characters + 26 uppercase characters + @ + _


echo

echo $((36#zz)) $((2#10101010)) $((16#AF16)) $((53#1aA))
                                         # 1295 170 44822 3375


#  Important note:
#  --------------
#  Using a digit out of range of the specified base notation
#+ gives an error message.

let "bad_oct = 081"
# (Partial) error message output:
#  bad_oct = 081: value too great for base (error token is "081")
#              Octal numbers use only digits in the range 0 - 7.

exit $?   # Exit value = 1 (error)

# Thanks, Rich Bartell and Stephane Chazelas, for clarification.
```

```output bash
decimal number = 32
octal number = 26
hexadecimal number = 50
39612
binary number = 31181
base-32 number = 231
base-64 number = 4031

1295 170 44822 3375
```

`from_base` reads bash's `BASE#NUMBER` for every base bash takes, with
bash's digits (for bases up to 36 a letter's case doesn't matter, so `int`
does it). A digit out of range is a `ValueError`, caught to show it.

```tessh
#!/usr/bin/env tessh
import string
# Representation of numbers in different bases.

# Decimal: the default
dec = 32
print "decimal number = $dec"            # 32

# Octal: numbers preceded by '0o'
oct = 0o32
print "octal number = $oct"              # 26
# Expresses result in decimal.

# Hexadecimal: numbers preceded by '0x' or '0X'
hex = 0x32
print "hexadecimal number = $hex"        # 50

print 0x9abc                             # 39612

# Other bases: int reads text in base 2 to 36, and from_base
# in bash's 2 to 64 (10 digits, 26 lowercase, 26 uppercase, @, _).
DIGITS = $string.digits + $string.ascii_lowercase + $string.ascii_uppercase + "@_"

def from_base number base
  if $base <= 36
    return (int $number $base)
  n = 0
  for c in $number
    d = $DIGITS.index $c
    if $d >= $base
      raise (ValueError "value too great for base: $number")
    n = $n * $base + $d
  $n

bin = from_base "111100111001101" 2
print "binary number = $bin"             # 31181

b32 = from_base "77" 32
print "base-32 number = $b32"            # 231

b64 = from_base "@_" 64
print "base-64 number = $b64"            # 4031

print

print (from_base zz 36) 0b10101010 (from_base AF16 16) (from_base 1aA 53)
                                         # 1295 170 44822 3375

#  Using a digit out of range of the base is an error.
try
  bad_oct = int "081" 8
except ValueError as e
  print $e
  exit 1
```

```output
decimal number = 32
octal number = 26
hexadecimal number = 50
39612
binary number = 31181
base-32 number = 231
base-64 number = 4031

1295 170 44822 3375
invalid literal for int() with base 8: '081'
```

## 8.3 The Double-Parentheses Construct
<!-- help: abs-dblparens double-parentheses (( )) c-style ++ -- ternary -->

`(( … ))` lets bash do arithmetic C's way: `a = 5 + 3` with spaces, `a++`,
`--a`, `a < 45 ? 7 : 11`. tessh's arithmetic is that way everywhere,
without the parentheses, except that there's no `++` or `--` (`+= 1`, `-=
1`) and the conditional is `if … then … else …`.

### Example 8-5. C-style manipulation of variables (`c-vars.sh`)

```bash
#!/bin/bash
# c-vars.sh
# Manipulating a variable, C-style, using the (( ... )) construct.


echo

(( a = 23 ))  #  Setting a value, C-style,
              #+ with spaces on both sides of the "=".
echo "a (initial value) = $a"   # 23

(( a++ ))     #  Post-increment 'a', C-style.
echo "a (after a++) = $a"       # 24

(( a-- ))     #  Post-decrement 'a', C-style.
echo "a (after a--) = $a"       # 23


(( ++a ))     #  Pre-increment 'a', C-style.
echo "a (after ++a) = $a"       # 24

(( --a ))     #  Pre-decrement 'a', C-style.
echo "a (after --a) = $a"       # 23

echo

########################################################
#  Note that, as in C, pre- and post-decrement operators
#+ have different side-effects.

n=1; let --n && echo "True" || echo "False"  # False
n=1; let n-- && echo "True" || echo "False"  # True

#  Thanks, Jeroen Domburg.
########################################################

echo

(( t = a<45?7:11 ))   # C-style trinary operator.
#       ^  ^ ^
echo "If a < 45, then t = 7, else t = 11."  # a = 23
echo "t = $t "                              # t = 7

echo


# -----------------
# Easter Egg alert!
# -----------------
#  Chet Ramey seems to have snuck a bunch of undocumented C-style
#+ constructs into Bash (actually adapted from ksh, pretty much).
#  In the Bash docs, Ramey calls (( ... )) shell arithmetic,
#+ but it goes far beyond that.
#  Sorry, Chet, the secret is out.

# See also "for" and "while" loops using the (( ... )) construct.

# These work only with version 2.04 or later of Bash.

exit
```

```output bash

a (initial value) = 23
a (after a++) = 24
a (after a--) = 23
a (after ++a) = 24
a (after --a) = 23

False
True

If a < 45, then t = 7, else t = 11.
t = 7 

```

The pre- and post-decrement difference (which value the expression has,
the old or the new) shows with the walrus `:=`, whose value is the new
one; the old one is simply read before. bash's `test && echo True || echo
False` is the conditional expression in tessh: `print` gives `None`, which
is false, so `x && print a || print b` would print both.

```tessh
#!/usr/bin/env tessh
print

a = 23        # Setting a value: spaces are fine.
print "a (initial value) = $a"  # 23

a += 1        # There's no a++ : add 1.
print "a (after a += 1) = $a"   # 24

a -= 1
print "a (after a -= 1) = $a"   # 23

print

########################################################
#  Bash's --n is the new value; n-- is the old one.

n = 1; print (if (n := $n - 1) then "True" else "False")   # False: the new value, 0
n = 1; old = $n ; n -= 1 ; print (if $old then "True" else "False")  # True: the old one
########################################################

print

t = if $a < 45 then 7 else 11   # The conditional expression.
print "If a < 45, then t = 7, else t = 11."  # a = 23
print "t = $t "                              # t = 7

print

exit
```

```output

a (initial value) = 23
a (after a += 1) = 24
a (after a -= 1) = 23

False
True

If a < 45, then t = 7, else t = 11.
t = 7 

```

## 8.4 Operator Precedence
<!-- help: abs-opprecedence precedence priority order-of-operations -->

Operators with higher precedence are done first. Bash's order, from the
tightest, against tessh's (which is Python's, with `|` moved for pipes):

| bash | meaning | tessh |
|---|---|---|
| `var++ var--`, `++var --var` | increment, decrement | none: `+= 1`, `-= 1` |
| `! ~` | not, bitwise not | `not` is loose (below the comparisons); `operator.invert` |
| `**` | power | `**` |
| `* / %` | multiply, divide, remainder | `* / // % @` |
| `+ -` | add, subtract | `+ -` |
| `<< >>` | shift | `<< >>` |
| `-z -n`, `-e -f …` | string and file tests | methods and functions |
| `< -lt > -gt <= -le >= -ge` | comparisons | `< > <= >=`, with `== != in is` |
| `-nt -ot -ef` | file comparisons | functions |
| `== -eq != -ne` | equality | `== !=` (all the comparisons are one level, and chain) |
| `&`, `^`, `\|` | bitwise and, xor, or | `&`, `^` above the comparisons; `\|` below them |
| `&& -a` | and | `and &&` |
| `\|\| -o` | or | `or \|\|` |
| `?:` | conditional | `if … then … else …` |
| `= *= /= …` | assignment | statements, not operators (`:=` inside parentheses) |
| `,` | comma | none (a `do` block) |

All you need to remember, the ABS Guide says, is "My Dear Aunt Sally"
(multiply, divide, add, subtract), that the logical operators are loose,
and that equal ones go left to right. That holds for tessh, with two
things to add: applying a function binds tighter than any operator (`len
$x + 1` is `(len $x) + 1`), and a `|` is looser than a comparison, since
it's mostly a pipe (in Python `1 | 2 == 3` is `(1 | 2) == 3`; in tessh it's
`1 | (2 == 3)`).

```tessh
print (1 + 2 * 3) (2 ** 3 ** 2) ((len [1 2]) + 1)
print (1 | 2 == 3) ((1 | 2) == 3)
```

```output
7 512 3
1 True
```

The ABS Guide takes apart two tests from a Fedora init script. Both are
"this and that", with `-a` the loosest operator in them; in tessh `and` is
loose the same way, and the tests read as they mean:

```bash no-run
while [ -n "$remaining" -a "$retry" -gt 0 ]; do

if [ -f /etc/sysconfig/i18n -a -z "${NOLOCALE:-}" ] ; then
```

```tessh no-run
import os
while $remaining and $retry > 0
  pass

if (os.path.isfile /etc/sysconfig/i18n) and not ($env.get NOLOCALE)
  pass
```

When a test gets long, group it. The ABS Guide's advice for bash, to use
separate `[[ ]]`s, is parentheses in tessh:

```bash no-run
if [ "$v1" -gt "$v2"  -o  "$v1" -lt "$v2"  -a  -e "$filename" ]
# Unclear what's going on here...

if [[ "$v1" -gt "$v2" ]] || [[ "$v1" -lt "$v2" ]] && [[ -e "$filename" ]]
# Much better -- the condition tests are grouped in logical sections.
```

```tessh no-run
if ($v1 > $v2) or ($v1 < $v2 and (os.path.exists $filename))
  pass
```
