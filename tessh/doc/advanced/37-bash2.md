# 37. Bash, versions 2, 3, and 4
<!-- help: abs-bash2 abs-37 bash-versions bash2 bash3 bash4 new-features -->

Each new bash added features the ABS Guide records here: arrays, `$'…'`
strings and `${!name}` in version 2; `{a..z}`, `=~` and `+=` in 3;
associative arrays, `coproc`, `mapfile`, case conversion and `**` in 4.
Many of them are things Python always had (dicts, negative indexes,
`str.upper`, regexes), so in tessh they are simply there; a few have no
tessh counterpart and are marked as gaps. `$BASH_VERSION`'s counterparts
are `$sys.version_info` for the Python and `tessh --help` for tessh.

## 37.1 Bash, version 2
<!-- help: abs-bashver2 bash-2 string-expansion indirect arrays -->

Version 2 added arrays, string and parameter expansion, and a better way
to write indirect references.

### Example 37-1. String expansion (`ex77.sh`)

```bash no-run
#!/bin/bash

# String expansion.
# Introduced with version 2 of Bash.

#  Strings of the form $'xxx'
#+ have the standard escaped characters interpreted. 

echo $'Ringing bell 3 times \a \a \a'
     # May only ring once with certain terminals.
     # Or ...
     # May not ring at all, depending on terminal settings.
echo $'Three form feeds \f \f \f'
echo $'10 newlines \n\n\n\n\n\n\n\n\n\n'
echo $'\102\141\163\150'
     #   B   a   s   h
     # Octal equivalent of characters.

exit
```

(It rings the bell, so it isn't run here.) Every double-quoted tessh string
reads escapes, as `$'…'` does. Piping through `cat -v` shows the bell and
form feeds as `^G` and `^L`. Octal escapes are the exception: **Gap:**
`"\102"` isn't `B` in tessh (it is in Python), so the last line spells the
octal numbers out with `0o`, Python's octal prefix.

```tessh
#!/usr/bin/env tessh
# String expansion: escapes in double-quoted strings.

echo "Ringing bell 3 times \a \a \a" | cat -v
     # May only ring once with certain terminals.
echo "Three form feeds \f \f \f" | cat -v
print "10 newlines \n\n\n\n\n\n\n\n\n\n"
print ("".join (map $chr [0o102 0o141 0o163 0o150]))
     #                       B     a     s     h
     # Octal equivalent of characters.
```

```output
Ringing bell 3 times ^G ^G ^G
Three form feeds ^L ^L ^L
10 newlines 










Bash
```

### Example 37-2. Indirect variable references - the new way (`ex78.sh`)

```bash
#!/bin/bash

# Indirect variable referencing.
# This has a few of the attributes of references in C++.


a=letter_of_alphabet
letter_of_alphabet=z

echo "a = $a"           # Direct reference.

echo "Now a = ${!a}"    # Indirect reference.
#  The ${!variable} notation is more intuitive than the old
#+ eval var1=\$$var2

echo

t=table_cell_3
table_cell_3=24
echo "t = ${!t}"                      # t = 24
table_cell_3=387
echo "Value of t changed to ${!t}"    # 387
# No 'eval' necessary.

#  This is useful for referencing members of an array or table,
#+ or for simulating a multi-dimensional array.
#  An indexing option (analogous to pointer arithmetic)
#+ would have been nice. Sigh.

exit 0

# See also, ind-ref.sh example.
```

`${!a}` is `(globals)[$a]` in tessh, as in chapter 28.

```tessh
#!/usr/bin/env tessh
# Indirect variable referencing.

a = letter_of_alphabet
letter_of_alphabet = z

print "a = $a"                       # Direct reference.

print "Now a = $((globals)[$a])"     # Indirect reference.

print

t = table_cell_3
table_cell_3 = 24
print "t = $((globals)[$t])"                   # t = 24
table_cell_3 = 387
print "Value of t changed to $((globals)[$t])" # 387
#  For a table, or a multi-dimensional array, a dict (of lists, of dicts)
#+ does it directly, and has no need of names made up on the fly.
```

```output
a = letter_of_alphabet
Now a = z

t = 24
Value of t changed to 387
```

### Example 37-3. Simple database application, using indirect variable referencing (`resistor-inventory.sh`)

```bash no-run
#!/bin/bash
# resistor-inventory.sh
# Simple database / table-lookup application.

# ============================================================== #
# Data

B1723_value=470                                   # Ohms
B1723_powerdissip=.25                             # Watts
B1723_colorcode="yellow-violet-brown"             # Color bands
B1723_loc=173                                     # Where they are
B1723_inventory=78                                # How many

B1724_value=1000
B1724_powerdissip=.25
B1724_colorcode="brown-black-red"
B1724_loc=24N
B1724_inventory=243

B1725_value=10000
B1725_powerdissip=.125
B1725_colorcode="brown-black-orange"
B1725_loc=24N
B1725_inventory=89

# ============================================================== #


echo

PS3='Enter catalog number: '

echo

select catalog_number in "B1723" "B1724" "B1725"
do
  Inv=${catalog_number}_inventory
  Val=${catalog_number}_value
  Pdissip=${catalog_number}_powerdissip
  Loc=${catalog_number}_loc
  Ccode=${catalog_number}_colorcode

  echo
  echo "Catalog number $catalog_number:"
  # Now, retrieve value, using indirect referencing.
  echo "There are ${!Inv} of  [${!Val} ohm / ${!Pdissip} watt]\
  resistors in stock."  #        ^             ^
  # As of Bash 4.2, you can replace "ohm" with \u2126 (using echo -e).
  echo "These are located in bin # ${!Loc}."
  echo "Their color code is \"${!Ccode}\"."

  break
done

echo; echo

# Exercises:
# ---------
# 1) Rewrite this script to read its data from an external file.
# 2) Rewrite this script to use arrays,
#+   rather than indirect variable referencing.
#    Which method is more straightforward and intuitive?
#    Which method is easier to code?


# Notes:
# -----
#  Shell scripts are inappropriate for anything except the most simple
#+ database applications, and even then it involves workarounds and kludges.
#  Much better is to use a language with native support for data structures,
#+ such as C++ or Java (or even Perl).

exit 0
```

It reads a menu choice from the terminal, so it isn't run here. **Gap:**
tessh's `select` works as a loop (`for item in (select …)`); here the menu
is a short loop and the choice is canned, so it runs without a terminal.
The lookups are the same indirect references as above, with names made
with `"${catalog_number}_value"`. The second exercise asks for arrays
instead: in tessh that would be a dict of dicts,
`{B1723: {value: 470 …} …}`, and `$parts[$n][value]`. And the 4.2 note
holds: `\u2126` is Ω.

```tessh
#!/usr/bin/env tessh
# resistor-inventory: simple database / table-lookup application.

# ============================================================== #
# Data

B1723_value = 470                                 # Ohms
B1723_powerdissip = ".25"                         # Watts
B1723_colorcode = "yellow-violet-brown"           # Color bands
B1723_loc = 173                                   # Where they are
B1723_inventory = 78                              # How many

B1724_value = 1000
B1724_powerdissip = ".25"
B1724_colorcode = "brown-black-red"
B1724_loc = 24N
B1724_inventory = 243

B1725_value = 10000
B1725_powerdissip = ".125"
B1725_colorcode = "brown-black-orange"
B1725_loc = 24N
B1725_inventory = 89

# ============================================================== #

def lookup catalog_number field    # The variable named NUMBER_FIELD.
  (globals)["${catalog_number}_$field"]

print
choices = [B1723 B1724 B1725]
for [i c] in (enumerate $choices 1)
  print "$i) $c"
print "Enter catalog number: 2"    # (input "Enter catalog number: ")
catalog_number = $choices[2 - 1]

print
print "Catalog number $catalog_number:"
# Now, retrieve the values, using indirect referencing.
print "There are $(lookup $catalog_number inventory) of  [$(lookup $catalog_number value) \u2126 / $(lookup $catalog_number powerdissip) watt]  resistors in stock."
print "These are located in bin # $(lookup $catalog_number loc)."
print "Their color code is \"$(lookup $catalog_number colorcode)\"."
print ; print
```

```output

1) B1723
2) B1724
3) B1725
Enter catalog number: 2

Catalog number B1724:
There are 243 of  [1000 Ω / .25 watt]  resistors in stock.
These are located in bin # 24N.
Their color code is "brown-black-red".


```

### Example 37-4. Using arrays and other miscellaneous trickery to deal four random hands from a deck of cards (`cards.sh`)

```bash no-run
#!/bin/bash
# cards.sh

# Deals four random hands from a deck of cards.

UNPICKED=0
PICKED=1

DUPE_CARD=99

LOWER_LIMIT=0
UPPER_LIMIT=51
CARDS_IN_SUIT=13
CARDS=52

declare -a Deck
declare -a Suits
declare -a Cards
#  It would have been easier to implement and more intuitive
#+ with a single, 3-dimensional array.
#  Perhaps a future version of Bash will support multidimensional arrays.


initialize_Deck ()
{
i=$LOWER_LIMIT
until [ "$i" -gt $UPPER_LIMIT ]
do
  Deck[i]=$UNPICKED   # Set each card of "Deck" as unpicked.
  let "i += 1"
done
echo
}

initialize_Suits ()
{
Suits[0]=C #Clubs
Suits[1]=D #Diamonds
Suits[2]=H #Hearts
Suits[3]=S #Spades
}

initialize_Cards ()
{
Cards=(2 3 4 5 6 7 8 9 10 J Q K A)
# Alternate method of initializing an array.
}

pick_a_card ()
{
card_number=$RANDOM
let "card_number %= $CARDS" # Restrict range to 0 - 51, i.e., 52 cards.
if [ "${Deck[card_number]}" -eq $UNPICKED ]
then
  Deck[card_number]=$PICKED
  return $card_number
else  
  return $DUPE_CARD
fi
}

parse_card ()
{
number=$1
let "suit_number = number / CARDS_IN_SUIT"
suit=${Suits[suit_number]}
echo -n "$suit-"
let "card_no = number % CARDS_IN_SUIT"
Card=${Cards[card_no]}
printf %-4s $Card
# Print cards in neat columns.
}

seed_random ()  # Seed random number generator.
{               # What happens if you don't do this?
seed=`eval date +%s`
let "seed %= 32766"
RANDOM=$seed
} # Consider other methods of seeding the random number generator.

deal_cards ()
{
echo

cards_picked=0
while [ "$cards_picked" -le $UPPER_LIMIT ]
do
  pick_a_card
  t=$?

  if [ "$t" -ne $DUPE_CARD ]
  then
    parse_card $t

    u=$cards_picked+1
    # Change back to 1-based indexing, temporarily. Why?
    let "u %= $CARDS_IN_SUIT"
    if [ "$u" -eq 0 ]   # Nested if/then condition test.
    then
     echo
     echo
    fi                  # Each hand set apart with a blank line.

    let "cards_picked += 1"
  fi  
done  

echo

return 0
}


# Structured programming:
# Entire program logic modularized in functions.

#===============
seed_random
initialize_Deck
initialize_Suits
initialize_Cards
deal_cards
#===============

exit



# Exercise 1:
# Add comments to thoroughly document this script.

# Exercise 2:
# Add a routine (function) to print out each hand sorted in suits.
# You may add other bells and whistles if you like.

# Exercise 3:
# Simplify and streamline the logic of the script.
```

(Its hands are random each run, so it isn't run here; the tessh version
seeds `random` with a fixed number so its deal is the same each time.)
The structure is kept, function for function, but `pick_a_card` gives back
`None` for a card already dealt instead of the magic 99, which a return
status forced on bash. Exercise 3, streamlined: `random.shuffle` on a list
of 52 cards and four slices of 13 would do the whole job, and exercise 2's
sorting is `sorted` with a `key`.

```tessh
#!/usr/bin/env tessh
# cards: deals four random hands from a deck of cards.
import random

UNPICKED = 0
PICKED = 1

LOWER_LIMIT = 0
UPPER_LIMIT = 51
CARDS_IN_SUIT = 13
CARDS = 52

def initialize_deck              # Every card unpicked.
  print
  [$UNPICKED for i in (range $LOWER_LIMIT ($UPPER_LIMIT + 1))]

Suits = [C D H S]                # Clubs Diamonds Hearts Spades
Cards = [2 3 4 5 6 7 8 9 10 J Q K A]

def pick_a_card deck
  card_number = random.randrange $CARDS   # 0 - 51, i.e., 52 cards.
  if $deck[$card_number] == $UNPICKED
    $deck[$card_number] = $PICKED
    $card_number
  else
    None                         # Already dealt.

def parse_card number
  suit = $Suits[$number // $CARDS_IN_SUIT]
  card = $Cards[$number % $CARDS_IN_SUIT]
  print "$suit-${card:<4}" end=""         # Print cards in neat columns.

def deal_cards deck
  print
  cards_picked = 0
  while $cards_picked <= $UPPER_LIMIT
    t = pick_a_card $deck
    if $t is not None
      parse_card $t
      if ($cards_picked + 1) % $CARDS_IN_SUIT == 0
        print ; print            # Each hand set apart with a blank line.
      cards_picked += 1
  print

#===============
random.seed 4     # (A fixed seed, so the deal is the same each time.)
Deck = (initialize_deck)
deal_cards $Deck
#===============
```

```output


D-4   D-8   C-8   S-9   D-A   H-6   C-J   C-7   C-6   C-3   H-J   D-7   S-A   

S-J   C-5   D-3   H-9   H-10  D-Q   D-6   S-Q   C-K   D-5   D-2   S-4   C-A   

C-Q   S-3   H-A   D-10  S-5   D-K   H-8   C-2   H-Q   S-8   H-2   H-3   H-4   

C-4   H-5   S-7   S-6   D-J   S-10  H-K   D-9   S-2   C-9   C-10  S-K   H-7   


```

## 37.2 Bash, version 3
<!-- help: abs-bashver3 bash-3 brace-expansion =~ pipefail += -->

Bash 3 (2004) added `{a..z}` brace expansion, `${!array[@]}`, the `=~`
regex operator and `set -o pipefail`, and broke some old scripts on the
way.

**Brace expansion.** **Gap:** tessh has none (chapter 33). Ranges and
lists do its jobs, and unlike bash's they take variables:

```bash
#!/bin/bash

for i in {1..10}
#  Simpler and more straightforward than
#+ for i in $(seq 10)
do
  echo -n "$i "
done

echo
# 1 2 3 4 5 6 7 8 9 10



# Or just . . .

echo {a..z}    #  a b c d e f g h i j k l m n o p q r s t u v w x y z
echo {e..m}    #  e f g h i j k l m
echo {z..a}    #  z y x w v u t s r q p o n m l k j i h g f e d c b a
               #  Works backwards, too.
echo {25..30}  #  25 26 27 28 29 30
echo {3..-2}   #  3 2 1 0 -1 -2
echo {X..d}    #  X Y Z [  ] ^ _ ` a b c d
               #  Shows (some of) the ASCII characters between Z and a,
               #+ but don't rely on this type of behavior because . . .
echo {]..a}    #  {]..a}
               #  Why?


# You can tack on prefixes and suffixes.
echo "Number #"{1..4}, "..."
     # Number #1, Number #2, Number #3, Number #4, ...


# You can concatenate brace-expansion sets.
echo {1..3}{x..z}" +" "..."
     # 1x + 1y + 1z + 2x + 2y + 2z + 3x + 3y + 3z + ...
     # Generates an algebraic expression.
     # This could be used to find permutations.


# You can nest brace-expansion sets.
echo {{a..c},{1..3}}
     # a b c 1 2 3
     # The "comma operator" splices together strings.

# ########## ######### ############ ########### ######### ###############

# Unfortunately, brace expansion does not lend itself to parameterization.
var1=1
var2=5
echo {$var1..$var2}   # {1..5}


# Yet, as Emiliano G. points out, using "eval" overcomes this limitation.

start=0
end=10
for index in $(eval echo {$start..$end})
do
  echo -n "$index "   # 0 1 2 3 4 5 6 7 8 9 10 
done

echo
```

```output bash
1 2 3 4 5 6 7 8 9 10 
a b c d e f g h i j k l m n o p q r s t u v w x y z
e f g h i j k l m
z y x w v u t s r q p o n m l k j i h g f e d c b a
25 26 27 28 29 30
3 2 1 0 -1 -2
X Y Z [  ] ^ _ ` a b c d
{]..a}
Number #1, Number #2, Number #3, Number #4, ...
1x + 1y + 1z + 2x + 2y + 2z + 3x + 3y + 3z + ...
a b c 1 2 3
{1..5}
0 1 2 3 4 5 6 7 8 9 10 
```

`string.ascii_lowercase` is a to z, a slice with step `-1` runs
backwards, `chr` and `ord` give the in-between characters, and
`itertools.product` concatenates sets.

```tessh
#!/usr/bin/env tessh
import string itertools

for i in (range 1 11)
  print $i end=" "
print
# 1 2 3 4 5 6 7 8 9 10

def letters a b                     # Like {a..b}, either way round.
  step = if (ord $a) <= (ord $b) then 1 else -1
  [(chr $c) for c in (range (ord $a) ((ord $b) + $step) $step)]

print *$string.ascii_lowercase      #  a b c ... z
print *(letters e m)                #  e f g h i j k l m
print *(letters z a)                #  z y x ... a: backwards, too.
print *(range 25 31)                #  25 26 27 28 29 30
print *(range 3 -3 -1)              #  3 2 1 0 -1 -2
print *(letters X d)                #  X Y Z [ \ ] ^ _ ` a b c d

# Prefixes and suffixes.
numbers = ["Number #$n," for n in (range 1 5)]
print *$numbers "..."

# Concatenating sets.
terms = ["$a$b +" for [a b] in (itertools.product [1 2 3] (letters x z))]
print *$terms "..."

# Nesting.
print *((letters a c) + (list (range 1 4)))

# Variables work: no eval needed.
var1 = 1
var2 = 5
print *(range $var1 ($var2 + 1))    # 1 2 3 4 5

start = 0
end = 10
for index in (range $start ($end + 1))
  print $index end=" "              # 0 1 2 3 4 5 6 7 8 9 10
print
```

```output
1 2 3 4 5 6 7 8 9 10 
a b c d e f g h i j k l m n o p q r s t u v w x y z
e f g h i j k l m
z y x w v u t s r q p o n m l k j i h g f e d c b a
25 26 27 28 29 30
3 2 1 0 -1 -2
X Y Z [ \ ] ^ _ ` a b c d
Number #1, Number #2, Number #3, Number #4, ...
1x + 1y + 1z + 2x + 2y + 2z + 3x + 3y + 3z + ...
a b c 1 2 3
1 2 3 4 5
0 1 2 3 4 5 6 7 8 9 10 
```

**All the indices of an array.** `${!Array[@]}` is `range (len $Array)`,
and `enumerate` gives index and element together:

```bash
#!/bin/bash

Array=(element-zero element-one element-two element-three)

echo ${Array[0]}   # element-zero
                   # First element of array.

echo ${!Array[@]}  # 0 1 2 3
                   # All the indices of Array.

for i in ${!Array[@]}
do
  echo ${Array[i]} # element-zero
                   # element-one
                   # element-two
                   # element-three
                   #
                   # All the elements in Array.
done
```

```tessh
#!/usr/bin/env tessh
Array = [element-zero element-one element-two element-three]

print $Array[0]                     # element-zero

print *(range (len $Array))         # 0 1 2 3: all the indices.

for [i element] in (enumerate $Array)
  print $element                    # (and $i is its index)
```

```output
element-zero
0 1 2 3
element-zero
element-one
element-two
element-three
```

**The `=~` operator.** tessh has it too, with Python's regexes written
`re'…'`, and it gives back the match (bash's `$BASH_REMATCH`) or `None`:

```bash
#!/bin/bash

variable="This is a fine mess."

echo "$variable"

# Regex matching with =~ operator within [[ double brackets ]].
if [[ "$variable" =~ T.........fin*es* ]]
# NOTE: As of version 3.2 of Bash, expression to match no longer quoted.
then
  echo "match found"
      # match found
fi
```

```tessh
#!/usr/bin/env tessh
variable = "This is a fine mess."

print $variable

if $variable =~ re'T.........fin*es*'
  print "match found"
```

```output
This is a fine mess.
match found
```

And more usefully, checking the form of an input:

```bash no-run
#!/bin/bash
input=$1


if [[ "$input" =~ "[0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9][0-9][0-9]" ]]
#                 ^ NOTE: Quoting not necessary, as of version 3.2 of Bash.

# NNN-NN-NNNN (where each N is a digit).
then
  echo "Social Security number."
  # Process SSN.
else
  echo "Not a Social Security number!"
  # Or, ask for corrected input.
fi
```

A quoted pattern with `=~` in tessh is text to look for as it is, which is
the same trap bash 3.2 fell into; the `re` makes it a regex. `^…$` makes
the whole input fit (bash's version also accepts "x123-45-6789y").

```tessh
#!/usr/bin/env tessh
for input in ["123-45-6789" "12-345-6789"]   # (sample inputs)
  if $input =~ re'^\d{3}-\d{2}-\d{4}$'
    print "$input: Social Security number."
  else
    print "$input: Not a Social Security number!"
```

```output
123-45-6789: Social Security number.
12-345-6789: Not a Social Security number!
```

**`set -o pipefail`** makes a pipe's status that of the last program in
it that failed. **Gap:** tessh has no `pipefail` (chapter 33).

### Bash, version 3.1

`+=` appends to a variable, where only `=` was allowed before. In bash it
appends text, except inside `let`, where it adds:

```bash
a=1
echo $a        # 1

a+=5           # Won't work under versions of Bash earlier than 3.1.
echo $a        # 15

a+=Hello
echo $a        # 15Hello
```

```output bash
1
15
15Hello
```

In tessh, `+=` is Python's: it adds numbers, appends text and lists, and a
number plus text is an error, not a guess:

```tessh
a = 1
print $a        # 1

a += 5          # A number: addition, as bash's let a+=5.
print $a        # 6

s = "1"
s += "5"        # Text: concatenation, as bash's a+=5.
print $s        # 15

try
  a += Hello
except TypeError as e
  print $e
```

```output
1
6
15
unsupported operand type(s) for +=: 'int' and 'str'
```

Jeffrey Haemer appends a folder to `$PATH` with it (`PATH+=:/opt/bin`); in
tessh, `$env.PATH += ":/opt/bin"`.

### Bash, version 3.2

Mostly bug fixes: a global substitution's pattern no longer anchors at the
start; `--wordexp` turns off process substitution; and `=~`'s pattern must
not be quoted any more (quoting makes it literal text, as in tessh;
`shopt -s compat31` brings back the old way).

## 37.3 Bash, version 4
<!-- help: abs-bashver4 bash-4 associative-arrays coproc mapfile case-modification globstar -->

Bash 4 (2009) added associative arrays, new `case` terminators,
coprocesses, `mapfile`, case conversion, more brace expansion, `**`, and
more.

**Associative arrays**: a set of keys, each with its value. In tessh
that's a dict, with all of Python's support for it.

### Example 37-5. A simple address database (`fetch_address.sh`)

```bash
#!/bin/bash4
# fetch_address.sh

declare -A address
#       -A option declares associative array.

address[Charles]="414 W. 10th Ave., Baltimore, MD 21236"
address[John]="202 E. 3rd St., New York, NY 10009"
address[Wilma]="1854 Vermont Ave, Los Angeles, CA 90023"


echo "Charles's address is ${address[Charles]}."
# Charles's address is 414 W. 10th Ave., Baltimore, MD 21236.
echo "Wilma's address is ${address[Wilma]}."
# Wilma's address is 1854 Vermont Ave, Los Angeles, CA 90023.
echo "John's address is ${address[John]}."
# John's address is 202 E. 3rd St., New York, NY 10009.

echo

echo "${!address[*]}"   # The array indices ...
# Charles John Wilma
```

```output bash
Charles's address is 414 W. 10th Ave., Baltimore, MD 21236.
Wilma's address is 1854 Vermont Ave, Los Angeles, CA 90023.
John's address is 202 E. 3rd St., New York, NY 10009.

Wilma Charles John
```

A dict keeps its keys in the order they went in; bash lists them in
whatever order its hash table has them.

```tessh
#!/usr/bin/env tessh
# fetch_address

address = {}

$address[Charles] = "414 W. 10th Ave., Baltimore, MD 21236"
$address[John] = "202 E. 3rd St., New York, NY 10009"
$address[Wilma] = "1854 Vermont Ave, Los Angeles, CA 90023"

print "Charles's address is $address[Charles]."
print "Wilma's address is $address[Wilma]."
print "John's address is $address[John]."

print

print *$address          # The keys ...
# Charles John Wilma
```

```output
Charles's address is 414 W. 10th Ave., Baltimore, MD 21236.
Wilma's address is 1854 Vermont Ave, Los Angeles, CA 90023.
John's address is 202 E. 3rd St., New York, NY 10009.

Charles John Wilma
```

### Example 37-6. A somewhat more elaborate address database (`fetch_address-2.sh`)

```bash
#!/bin/bash4
# fetch_address-2.sh
# A more elaborate version of fetch_address.sh.

SUCCESS=0
E_DB=99    # Error code for missing entry.

declare -A address
#       -A option declares associative array.


store_address ()
{
  address[$1]="$2"
  return $?
}


fetch_address ()
{
  if [[ -z "${address[$1]}" ]]
  then
    echo "$1's address is not in database."
    return $E_DB
  fi

  echo "$1's address is ${address[$1]}."
  return $?
}


store_address "Lucas Fayne" "414 W. 13th Ave., Baltimore, MD 21236"
store_address "Arvid Boyce" "202 E. 3rd St., New York, NY 10009"
store_address "Velma Winston" "1854 Vermont Ave, Los Angeles, CA 90023"
#  Exercise:
#  Rewrite the above store_address calls to read data from a file,
#+ then assign field 1 to name, field 2 to address in the array.
#  Each line in the file would have a format corresponding to the above.
#  Use a while-read loop to read from file, sed or awk to parse the fields.

fetch_address "Lucas Fayne"
# Lucas Fayne's address is 414 W. 13th Ave., Baltimore, MD 21236.
fetch_address "Velma Winston"
# Velma Winston's address is 1854 Vermont Ave, Los Angeles, CA 90023.
fetch_address "Arvid Boyce"
# Arvid Boyce's address is 202 E. 3rd St., New York, NY 10009.
fetch_address "Bozo Bozeman"
# Bozo Bozeman's address is not in database.

exit $?   # In this case, exit code = 99, since that is function return.
```

The exercise is done too: the addresses come from a file, one per line,
name and address separated by a tab, split with `.split "\t" 1`.

```tessh
#!/usr/bin/env tessh
# fetch_address-2: a more elaborate version of fetch_address.

E_DB = 99    # Error code for missing entry.

address = {}

def store_address name addr
  $address[$name] = $addr

def fetch_address name
  if $name not in $address
    print "$name's address is not in database."
    return $E_DB
  print "$name's address is $address[$name]."
  0

# (A made-up data file: name, a tab, the address.)
printf "Lucas Fayne\t414 W. 13th Ave., Baltimore, MD 21236\nArvid Boyce\t202 E. 3rd St., New York, NY 10009\nVelma Winston\t1854 Vermont Ave, Los Angeles, CA 90023\n" >addresses.txt

text = ((Path addresses.txt).read_text)
for line in ($text.splitlines)
  [name addr] = $line.split "\t" 1
  store_address $name $addr

fetch_address "Lucas Fayne"
fetch_address "Velma Winston"
fetch_address "Arvid Boyce"
status = fetch_address "Bozo Bozeman"

exit $status   # In this case, exit code = 99.
```

```output
Lucas Fayne's address is 414 W. 13th Ave., Baltimore, MD 21236.
Velma Winston's address is 1854 Vermont Ave, Los Angeles, CA 90023.
Arvid Boyce's address is 202 E. 3rd St., New York, NY 10009.
Bozo Bozeman's address is not in database.
```

A key may hold spaces, even at its ends, but bash refuses a key that is
only spaces (`address[   ]="Blank"`); a dict takes any key, `""` and `"
"` included.

**`case` terminators.** `;;&` goes on to test the next pattern, and `;&`
runs the next branch without testing it.

### Example 37-7. Testing characters (`case4.sh`)

```bash
#!/bin/bash4

test_char ()
{
  case "$1" in
    [[:print:]] )  echo "$1 is a printable character.";;&       # |
    # The ;;& terminator continues to the next pattern test.      |
    [[:alnum:]] )  echo "$1 is an alpha/numeric character.";;&  # v
    [[:alpha:]] )  echo "$1 is an alphabetic character.";;&     # v
    [[:lower:]] )  echo "$1 is a lowercase alphabetic character.";;&
    [[:digit:]] )  echo "$1 is an numeric character.";&         # |
    # The ;& terminator executes the next statement ...         # |
    %%%@@@@@    )  echo "********************************";;    # v
#   ^^^^^^^^  ... even with a dummy pattern.
  esac
}

echo

test_char 3
# 3 is a printable character.
# 3 is an alpha/numeric character.
# 3 is an numeric character.
# ********************************
echo

test_char m
# m is a printable character.
# m is an alpha/numeric character.
# m is an alphabetic character.
# m is a lowercase alphabetic character.
echo

test_char /
# / is a printable character.

echo

# The ;;& terminator can save complex if/then conditions.
# The ;& is somewhat less useful.
```

A `match` takes the first case that fits and stops, like `;;`; there's no
going on to the next. Tests that should each be tried are a row of `if`s,
and Python's string methods are the character classes. The `;&`
fall-through is just the line after.

```tessh
#!/usr/bin/env tessh
def test_char c
  if ($c.isprintable)
    print "$c is a printable character."
  if ($c.isalnum)
    print "$c is an alpha/numeric character."
  if ($c.isalpha)
    print "$c is an alphabetic character."
  if ($c.islower)
    print "$c is a lowercase alphabetic character."
  if ($c.isdigit)
    print "$c is an numeric character."
    print "********************************"   # (bash's ;& fall-through)

print

test_char "3"
print

test_char m
print

test_char /

print
```

```output

3 is a printable character.
3 is an alpha/numeric character.
3 is an numeric character.
********************************

m is a printable character.
m is an alpha/numeric character.
m is an alphabetic character.
m is a lowercase alphabetic character.

/ is a printable character.

```

**Coprocesses.** `coproc` runs a command in the background with pipes to
and from it. **Gap:** tessh has no `coproc`; `subprocess.Popen` with
`stdin=PIPE` and `stdout=PIPE` is Python's coprocess, and its `stdout` is
read like a file.

```bash no-run
#!/bin/bash4
# A coprocess communicates with a while-read loop.


coproc { cat mx_data.txt; sleep 2; }
#                         ^^^^^^^
# Try running this without "sleep 2" and see what happens.

while read -u ${COPROC[0]} line    #  ${COPROC[0]} is the
do                                 #+ file descriptor of the coprocess.
  echo "$line" | sed -e 's/line/NOT-ORIGINAL-TEXT/'
done

kill $COPROC_PID                   #  No longer need the coprocess,
                                   #+ so kill its PID.
```

```tessh
#!/usr/bin/env tessh
import subprocess
# (A made-up data file.)
printf "line one\nline two\n" >mx_data.txt

coproc = subprocess.Popen [sh -c "cat mx_data.txt; sleep 2"] stdout=$subprocess.PIPE text=True

for line in $coproc.stdout          # Read from the coprocess, line by line.
  $line | sed -e 's/line/NOT-ORIGINAL-TEXT/'

$coproc.kill                        # No longer need the coprocess.
```

```output
NOT-ORIGINAL-TEXT one
NOT-ORIGINAL-TEXT two
```

The ABS Guide warns that a `while read` loop on a coprocess runs in a
subshell, so the variables it reads are gone after it. A Python loop runs
in the script, so they're kept:

```bash no-run
#!/bin/bash4

echo; echo
a=aaa
b=bbb
c=ccc

coproc echo "one two three"
while read -u ${COPROC[0]} a b c;  #  Note that this loop
do                                 #+ runs in a subshell.
  echo "Inside while-read loop: ";
  echo "a = $a"; echo "b = $b"; echo "c = $c"
  echo "coproc file descriptor: ${COPROC[0]}"
done 

# a = one
# b = two
# c = three
# So far, so good, but ...
echo "-----------------"
echo "Outside while-read loop: "
echo "a = $a"  # a =
echo "b = $b"  # b =
echo "c = $c"  # c =
echo "coproc file descriptor: ${COPROC[0]}"
echo
#  The coproc is still running, but ...
#+ it still doesn't enable the parent process
#+ to "inherit" variables from the child process, the while-read loop.

#  Compare this to the "badread.sh" script.
```

```tessh
#!/usr/bin/env tessh
import subprocess

a = aaa ; b = bbb ; c = ccc

coproc = subprocess.Popen [echo "one two three"] stdout=$subprocess.PIPE text=True
for line in $coproc.stdout
  [a b c] = ($line.split)
  print "Inside the loop: a = $a, b = $b, c = $c"
$coproc.wait
print "Outside the loop: a = $a, b = $b, c = $c"   # Kept.
```

```output
Inside the loop: a = one, b = two, c = three
Outside the loop: a = one, b = two, c = three
```

A coprocess is asynchronous and may end before the other side is done
talking to it; with `Popen`, `.communicate` writes and reads both ends
and waits, which avoids the deadlocks the ABS Guide's named-coprocess
example runs into:

```bash no-run
#!/bin/bash4

coproc cpname { for i in {0..10}; do echo "index = $i"; done; }
#      ^^^^^^ This is a *named* coprocess.
read -u ${cpname[0]}
echo $REPLY         #  index = 0
echo ${COPROC[0]}   #+ No output ... the coprocess timed out
#  after the first loop iteration.



# However, George Dimitriu has a partial fix.

coproc cpname { for i in {0..10}; do echo "index = $i"; done; sleep 1;
echo hi > myo; cat - >> myo; }
#       ^^^^^ This is a *named* coprocess.

echo "I am main"$'\04' >&${cpname[1]}
myfd=${cpname[0]}
echo myfd=$myfd

### while read -u $myfd
### do
###   echo $REPLY;
### done

echo $cpname_PID

#  Run this with and without the commented-out while-loop, and it is
#+ apparent that each process, the executing shell and the coprocess,
#+ waits for the other to finish writing in its own write-enabled pipe.
```

```tessh
#!/usr/bin/env tessh
import subprocess
script = "for i in 0 1 2; do echo \"index = \$i\"; done; echo hi >myo; cat - >>myo"
cpname = subprocess.Popen [sh -c $script] stdin=$subprocess.PIPE stdout=$subprocess.PIPE text=True
[out err] = $cpname.communicate "I am main\n"   # Write, read, and wait.
print $out end=""
cat myo
```

```output
index = 0
index = 1
index = 2
hi
I am main
```

**`mapfile`** loads a file's lines into an array. In tessh, `.splitlines`
on the text, or `.lines` on a program's output; `read -a` reads one line's
words, which is `.split` on that line (tessh's `read -a Arr2 <file` works
too).

```bash no-run
#!/bin/bash4

mapfile Arr1 < $0
# Same result as     Arr1=( $(cat $0) )
echo "${Arr1[@]}"  # Copies this entire script out to stdout.

echo "--"; echo

# But, not the same as   read -a   !!!
read -a Arr2 < $0
echo "${Arr2[@]}"  # Reads only first line of script into the array.

exit
```

```tessh
#!/usr/bin/env tessh
Arr1 = (((Path $argv[0]).read_text).splitlines)
print "$(len $Arr1) lines; the first: $Arr1[0]"

print "--" ; print

Arr2 = ($Arr1[0].split)    # read -a: the words of the first line.
print $Arr2
```

```output
8 lines; the first: #!/usr/bin/env tessh
--

['#!/usr/bin/env' 'tessh']
```

`read -t` takes fractions of a second and `read -i` fills in the edit
buffer. tessh's `read` has `-t secs` (status 142 when time runs out) but
no `-i`; Python's `input` has no timeout without `select` or a thread.

**Case conversion** in parameter substitution, and `declare -l`/`-c`:

```bash
#!/bin/bash4

var=veryMixedUpVariable
echo ${var}            # veryMixedUpVariable
echo ${var^}           # VeryMixedUpVariable
#         *              First char --> uppercase.
echo ${var^^}          # VERYMIXEDUPVARIABLE
#         **             All chars  --> uppercase.
echo ${var,}           # veryMixedUpVariable
#         *              First char --> lowercase.
echo ${var,,}          # verymixedupvariable
#         **             All chars  --> lowercase.
```

```tessh
#!/usr/bin/env tessh
var = veryMixedUpVariable
print $var                                  # veryMixedUpVariable
print (($var[:1].upper) + $var[1:])         # VeryMixedUpVariable
print ($var.upper)                          # VERYMIXEDUPVARIABLE
print (($var[:1].lower) + $var[1:])         # veryMixedUpVariable
print ($var.lower)                          # verymixedupvariable
```

```output
veryMixedUpVariable
VeryMixedUpVariable
VERYMIXEDUPVARIABLE
veryMixedUpVariable
verymixedupvariable
```

`declare -l var1` lowercases whatever is assigned to `var1` from then on,
and `declare -c` capitalizes it. tessh variables have no such attributes;
convert the value when you assign it (`var1 = (.lower
MixedCaseVARIABLE)`), or `.capitalize`, which also lowercases the rest.

**More brace expansion**: a step, `{40..60..2}`, and zero-padding,
`{000..10}`. In tessh, `range`'s third argument is the step, and a format
spec pads:

```bash
echo {40..60..2}
echo {60..40..2}
echo {000..10}
```

```tessh
print *(range 40 61 2)
print *(range 60 39 -2)
padded = ["${n:03d}" for n in (range 11)]
print *$padded
```

```output
40 42 44 46 48 50 52 54 56 58 60
60 58 56 54 52 50 48 46 44 42 40
000 001 002 003 004 005 006 007 008 009 010
```

**`$0` in substring extraction** of the positional parameters:
`${@:0}` now starts with the script's name. `$argv` always did:
`$argv[0]` is the script and `$argv[1:]` its arguments.

**`**`** matches files and folders recursively once `shopt -s globstar`
is on. In tessh it's always on.

```bash no-run
#!/bin/bash4
# filelist.bash4

shopt -s globstar  # Must enable globstar, otherwise ** doesn't work.
                   # The globstar shell option is new to version 4 of Bash.

echo "Using *"; echo
for filename in *
do
  echo "$filename"
done   # Lists only files in current directory ($PWD).

echo; echo "--------------"; echo

echo "Using **"
for filename in **
do
  echo "$filename"
done   # Lists complete file tree, recursively.

exit
```

```tessh
#!/usr/bin/env tessh
# filelist
# (A made-up file tree.)
mkdir -p allmyfiles/my_music allmyfiles/my_pictures
touch allmyfiles/file.index.txt allmyfiles/my_music/piano-lesson.1.ogg
touch allmyfiles/my_pictures/picnic.png filelist

print "Using *" ; print
for filename in *
  print $filename      # Lists only files in the current folder.

print ; print "--------------" ; print

print "Using **"
for filename in **
  print $filename      # Lists the complete file tree, recursively.
```

```output
Using *

allmyfiles
filelist

--------------

Using **
allmyfiles
allmyfiles/file.index.txt
allmyfiles/my_music
allmyfiles/my_music/piano-lesson.1.ogg
allmyfiles/my_pictures
allmyfiles/my_pictures/picnic.png
filelist
```

**`$BASHPID`** is the process ID of the current bash, subshell or not;
tessh's is `(os.getpid)`.

**`command_not_found_handle`** is a function bash calls for a command it
can't find. **Gap:** tessh has no such hook: it prints `tessh: command not
found: NAME` and sets `$?` to 127, which a script can test.

```bash
#!/bin/bash4

command_not_found_handle ()
{ # Accepts implicit parameters.
  echo "The following command is not valid: \""$1\"""
  echo "With the following argument(s): \""$2\"" \""$3\"""   # $4, $5 ...
} # $1, $2, etc. are not explicitly passed to the function.


bad_command arg1 arg2

# The following command is not valid: "bad_command"
# With the following argument(s): "arg1" "arg2"
```

```output bash
The following command is not valid: "bad_command"
With the following argument(s): "arg1" "arg2"
```

```tessh
#!/usr/bin/env tessh
import shutil

def run_or_complain cmd *args       # A wrapper doing the handler's job.
  if not (shutil.which $cmd)
    print "The following command is not valid: \"$cmd\""
    quoted = ["\"$a\"" for a in $args]
    print "With the following argument(s):" *$quoted
    return 127
  $cmd *$args

run_or_complain bad_command arg1 arg2
```

```output
The following command is not valid: "bad_command"
With the following argument(s): "arg1" "arg2"
```

The ABS Guide's editorial wonders whether bash is suffering feature creep
and asks for what it still lacked: for-each loops, multi-dimensional
arrays, and a fix for the piped `read`. Python has had all three from the
start: `for x in $xs`, lists of lists, and no subshells.

### Bash, version 4.1

`printf -v` can set array elements; `<` and `>` in `[[ ]]` follow the
locale (tessh's compare code points; `locale.strcoll` follows the locale);
`read -N n` reads exactly n characters; and a here document inside `$( )`
may end with `)` on the limit string's line.

### Example 37-8. Reading N characters (`read-N.sh`)

```bash
#!/bin/bash
# Requires Bash version -ge 4.1 ...

num_chars=61

read -N $num_chars var < $0   # Read first 61 characters of script!
echo "$var"
exit

####### Output of Script #######

#!/bin/bash
# Requires Bash version -ge 4.1 ...

num_chars=61
```

```output bash
#!/bin/bash
# Requires Bash version -ge 4.1 ...

num_chars=61
```

A file's `.read n` reads n characters; tessh's `read -N n` does too, from
its input.

```tessh
#!/usr/bin/env tessh
# Reads the first 64 characters of this script.

num_chars = 64

with (open $argv[0]) as f
  var = $f.read $num_chars   # Read first 64 characters of script!
print $var
```

```output
#!/usr/bin/env tessh
# Reads the first 64 characters of this scr
```

### Example 37-9. Using a here document to set a variable (`here-commsub.sh`)

```bash
#!/bin/bash
# here-commsub.sh
# Requires Bash version -ge 4.1 ...

multi_line_var=$( cat <<ENDxxx
------------------------------
This is line 1 of the variable
This is line 2 of the variable
This is line 3 of the variable
------------------------------
ENDxxx)

#  Rather than what Bash 4.0 requires:
#+ that the terminating limit string and
#+ the terminating close-parenthesis be on separate lines.

# ENDxxx
# )


echo "$multi_line_var"

#  Bash still emits a warning, though.
#  warning: here-document at line 10 delimited
#+ by end-of-file (wanted `ENDxxx')
```

A heredoc on a captured command needs no parentheses at all: the
assignment takes the command's output. (A triple-quoted string is the
other way to write several lines into a variable.)

```tessh
#!/usr/bin/env tessh
multi_line_var = cat <<ENDxxx
------------------------------
This is line 1 of the variable
This is line 2 of the variable
This is line 3 of the variable
------------------------------
ENDxxx

print $multi_line_var
```

```output
------------------------------
This is line 1 of the variable
This is line 2 of the variable
This is line 3 of the variable
------------------------------
```

### Bash, version 4.2

`\u` and `\U` Unicode escapes, in `$'…'` and `echo -e`. tessh strings
take them, as Python's do:

```bash
echo -e '\u2630'   # Horizontal triple bar character.
# Equivalent to the more roundabout:
echo -e "\xE2\x98\xB0"
                   # Recognized by earlier Bash versions.

echo -e '\u220F'   # PI (Greek letter and mathematical symbol)
echo -e '\u0416'   # Capital "ZHE" (Cyrillic letter)
echo -e '\u2708'   # Airplane (Dingbat font) symbol
echo -e '\u2622'   # Radioactivity trefoil

echo -e "The amplifier circuit requires a 100 \u2126 pull-up resistor."


unicode_var='\u2640'
echo -e $unicode_var      # Female symbol
printf "$unicode_var \n"  # Female symbol, with newline


#  And for something a bit more elaborate . . .

#  We can store Unicode symbols in an associative array,
#+ then retrieve them by name.
#  Run this in a gnome-terminal or a terminal with a large, bold font
#+ for better legibility.

declare -A symbol  # Associative array.

symbol[script_E]='\u2130'
symbol[script_F]='\u2131'
symbol[script_J]='\u2110'
symbol[script_M]='\u2133'
symbol[Rx]='\u211E'
symbol[TEL]='\u2121'
symbol[FAX]='\u213B'
symbol[care_of]='\u2105'
symbol[account]='\u2100'
symbol[trademark]='\u2122'


echo -ne "${symbol[script_E]}   "
echo -ne "${symbol[script_F]}   "
echo -ne "${symbol[script_J]}   "
echo -ne "${symbol[script_M]}   "
echo -ne "${symbol[Rx]}   "
echo -ne "${symbol[TEL]}   "
echo -ne "${symbol[FAX]}   "
echo -ne "${symbol[care_of]}   "
echo -ne "${symbol[account]}   "
echo -ne "${symbol[trademark]}   "
echo
```

```output bash
☰
☰
∏
Ж
✈
☢
The amplifier circuit requires a 100 Ω pull-up resistor.
♀
♀ 
ℰ   ℱ   ℐ   ℳ   ℞   ℡   ℻   ℅   ℀   ™   
```

The UTF-8 bytes are `.encode`'s business, and `unicodedata.name` says
what a character is called.

```tessh
import unicodedata

print "\u2630"     # Horizontal triple bar character.
print (b"\xE2\x98\xB0".decode)       # The same, from its UTF-8 bytes.

print "\u220F"     # PI (Greek letter and mathematical symbol)
print "\u0416"     # Capital "ZHE" (Cyrillic letter)
print "\u2708"     # Airplane (Dingbat font) symbol
print "\u2622"     # Radioactivity trefoil

print "The amplifier circuit requires a 100 \u2126 pull-up resistor."

unicode_var = "\u2640"
print $unicode_var (unicodedata.name $unicode_var)

#  Unicode symbols in a dict, retrieved by name.
symbol = {script_E: "\u2130" script_F: "\u2131" script_J: "\u2110"
          script_M: "\u2133" Rx: "\u211E" TEL: "\u2121" FAX: "\u213B"
          care_of: "\u2105" account: "\u2100" trademark: "\u2122"}

print *($symbol.values) sep="   "
```

```output
☰
☰
∏
Ж
✈
☢
The amplifier circuit requires a 100 Ω pull-up resistor.
♀ FEMALE SIGN
ℰ   ℱ   ℐ   ℳ   ℞   ℡   ℻   ℅   ℀   ™
```

**`shopt -s lastpipe`** runs the last command of a pipe in the shell
itself, not a subshell, so `… | read line` finally works.

### Example 37-10. Piping input to a read (`lastpipe-option.sh`)

```bash
#!/bin/bash
# lastpipe-option.sh

line=''                   # Null value.
echo "\$line = "$line""   # $line =

echo

shopt -s lastpipe         # Error on Bash version -lt 4.2.
echo "Exit status of attempting to set \"lastpipe\" option is $?"
#     1 if Bash version -lt 4.2, 0 otherwise.

echo

head -1 $0 | read line    # Pipe the first line of the script to read.
#            ^^^^^^^^^      Not in a subshell!!!

echo "\$line = "$line""
# Older Bash releases       $line =
# Bash version 4.2          $line = #!/bin/bash
```

```output bash
$line = 

Exit status of attempting to set "lastpipe" option is 0

$line = #!/bin/bash
```

tessh has no subshell to lose the value in: its `read` runs in the shell,
so `head -1 $argv[0] | read line` sets `line`, `lastpipe` or not. Or the pipe's
output is the value, assigned like any other, as here.

```tessh
#!/usr/bin/env tessh
# lastpipe-option

line = ""                   # Empty.
print "\$line = $line"      # $line =

print

line = head -1 $argv[0]     # The first line of the script.

print "\$line = $line"      # $line = #!/usr/bin/env tessh
```

```output
$line = 

$line = #!/usr/bin/env tessh
```

**Negative array indices** count back from the end. Python always had
them.

### Example 37-11. Negative array indices (`neg-array.sh`)

```bash
#!/bin/bash
# neg-array.sh
# Requires Bash, version -ge 4.2.

array=( zero one two three four five )   # Six-element array.
#         0    1   2    3    4    5
#        -6   -5  -4   -3   -2   -1

# Negative array indices now permitted.
echo ${array[-1]}   # five
echo ${array[-2]}   # four
# ...
echo ${array[-6]}   # zero
# Negative array indices count backward from the last element+1.

# But, you cannot index past the beginning of the array.
echo ${array[-7]}   # array: bad array subscript


# So, what is this new feature good for?

echo "The last element in the array is "${array[-1]}""
# Which is quite a bit more straightforward than:
echo "The last element in the array is "${array[${#array[*]}-1]}""
echo

# And ...

index=0
let "neg_element_count = 0 - ${#array[*]}"
# Number of elements, converted to a negative number.

while [ $index -gt $neg_element_count ]; do
  ((index--)); echo -n "${array[index]} "
done  # Lists the elements in the array, backwards.
      # We have just simulated the "tac" command on this array.

echo

# See also neg-offset.sh.
```

```output bash
five
four
zero

The last element in the array is five
The last element in the array is five

five four three two one zero 
```

Indexing past the start is an `IndexError`, caught here. The backwards
loop is kept, though `$array[::-1]` (or `reversed`) is the list
backwards in one step.

```tessh
#!/usr/bin/env tessh
# neg-array

array = [zero one two three four five]   # Six-element list.
#          0    1   2    3    4    5
#         -6   -5  -4   -3   -2   -1

print $array[-1]   # five
print $array[-2]   # four
print $array[-6]   # zero

try
  print $array[-7]
except IndexError as e
  print $e         # list index out of range

print "The last element in the array is $array[-1]"
print "The last element in the array is $array[(len $array) - 1]"
print

index = 0
neg_element_count = - (len $array)
while $index > $neg_element_count
  index -= 1
  print $array[$index] end=" "
print              # Lists the elements backwards, as tac would.
print *$array[::-1]
```

```output
five
four
zero
list index out of range
The last element in the array is five
The last element in the array is five

five four three two one zero 
five four three two one zero
```

**Negative lengths** in substring extraction count from the end. Python's
slices do the same with a negative end.

### Example 37-12. Negative parameter in string-extraction construct (`neg-offset.sh`)

```bash
#!/bin/bash
# Bash, version -ge 4.2
# Negative length-index in substring extraction.
# Important: It changes the interpretation of this construct!

stringZ=abcABC123ABCabc

echo ${stringZ}                              # abcABC123ABCabc
#                   Position within string:    0123456789.....
echo ${stringZ:2:3}                          #   cAB
#  Count 2 chars forward from string beginning, and extract 3 chars.
#  ${string:position:length}

#  So far, nothing new, but now ...

                                             # abcABC123ABCabc
#                   Position within string:    0123....6543210
echo ${stringZ:3:-6}                         #    ABC123
#                ^
#  Index 3 chars forward from beginning and 6 chars backward from end,
#+ and extract everything in between.
#  ${string:offset-from-front:offset-from-end}
#  When the "length" parameter is negative, 
#+ it serves as an offset-from-end parameter.

#  See also neg-array.sh.
```

A slice is start and end, not start and length: `${stringZ:2:3}` is
`$stringZ[2:5]`, and `${stringZ:3:-6}` is `$stringZ[3:-6]`.

```tessh
#!/usr/bin/env tessh
stringZ = abcABC123ABCabc

print $stringZ                  # abcABC123ABCabc
print $stringZ[2:5]             #   cAB: from 2, 3 characters.
print $stringZ[3:-6]            #    ABC123: from 3 to 6 from the end.
```

```output
abcABC123ABCabc
cAB
ABC123
```
