# 27. Arrays
<!-- help: abs-arrays abs-27 array arrays list dict sparse -->

bash has one-dimensional arrays: `name[i]=value` sets an element,
`name=( a b c )` sets them all, and `${name[i]}` reads one. The indexes
needn't be consecutive: an array can have holes ("sparse" arrays). tessh
uses Python's collections instead. A list, `[a b c]`, is a packed array:
`$xs[0]`, `len $xs`, `$xs.append x`, slices `$xs[1:3]`. A dict,
`{11: 23 13: 37}`, is a sparse one (or one indexed by names, as bash 4's
associative arrays are). They hold any values, lists and dicts included,
so the tricks bash needs for nested or two-dimensional arrays aren't
needed.

| bash | tessh |
|---|---|
| `a=( x y z )` | `a = [x y z]` |
| `a[5]=v` (sparse) | `$a[5] = v` on a dict, or `.append` on a list |
| `${a[i]}` | `$a[i]` (`$a.get i ""` on a dict, for a default) |
| `${a[@]}`, `${a[*]}` | `$a` as a value; `*$a` spreads it into arguments |
| `${#a[@]}` | `len $a` |
| `${#a[0]}` | `len $a[0]` |
| `${a[@]:1:2}` | `$a[1:3]` |
| `${a[@]/x/y}` | `[($e.replace x y 1) for e in $a]` |
| `unset a[1]` | `del $a[1]` (a list closes the gap; a dict doesn't have one) |
| `a=( "${a[@]}" new )` | `$a.append new`, or `a = $a + [new]` |

### Example 27-1. Simple array usage (`ex66.sh`)

```bash
#!/bin/bash


area[11]=23
area[13]=37
area[51]=UFOs

#  Array members need not be consecutive or contiguous.

#  Some members of the array can be left uninitialized.
#  Gaps in the array are okay.
#  In fact, arrays with sparse data ("sparse arrays")
#+ are useful in spreadsheet-processing software.


echo -n "area[11] = "
echo ${area[11]}    #  {curly brackets} needed.

echo -n "area[13] = "
echo ${area[13]}

echo "Contents of area[51] are ${area[51]}."

# Contents of uninitialized array variable print blank (null variable).
echo -n "area[43] = "
echo ${area[43]}
echo "(area[43] unassigned)"

echo

# Sum of two array variables assigned to third
area[5]=`expr ${area[11]} + ${area[13]}`
echo "area[5] = area[11] + area[13]"
echo -n "area[5] = "
echo ${area[5]}

area[6]=`expr ${area[11]} + ${area[51]}`
echo "area[6] = area[11] + area[51]"
echo -n "area[6] = "
echo ${area[6]}
# This fails because adding an integer to a string is not permitted.

echo; echo; echo

# -----------------------------------------------------------------
# Another array, "area2".
# Another way of assigning array variables...
# array_name=( XXX YYY ZZZ ... )

area2=( zero one two three four )

echo -n "area2[0] = "
echo ${area2[0]}
# Aha, zero-based indexing (first element of array is [0], not [1]).

echo -n "area2[1] = "
echo ${area2[1]}    # [1] is second element of array.
# -----------------------------------------------------------------

echo; echo; echo

# -----------------------------------------------
# Yet another array, "area3".
# Yet another way of assigning array variables...
# array_name=([xx]=XXX [yy]=YYY ...)

area3=([17]=seventeen [24]=twenty-four)

echo -n "area3[17] = "
echo ${area3[17]}

echo -n "area3[24] = "
echo ${area3[24]}
# -----------------------------------------------

exit 0
```

`area` has holes, so it's a dict. An element that isn't there is a
`KeyError`; `.get` gives a default. Adding a number to a string is a
`TypeError`, which takes the place of `expr`'s complaint.

```tessh
area = {}
$area[11] = 23
$area[13] = 37
$area[51] = UFOs

#  Array members need not be consecutive or contiguous:
#+ a dict holds just the ones that are set.

print "area[11] = $area[11]"
print "area[13] = $area[13]"
print "Contents of area[51] are $area[51]."

# An element that isn't there: .get gives a default.
print "area[43] = $($area.get 43 '')"
print "(area[43] unassigned)"

print

# Sum of two array variables assigned to third
$area[5] = $area[11] + $area[13]
print "area[5] = area[11] + area[13]"
print "area[5] = $area[5]"

$area[6] = try
  $area[11] + $area[51]
except TypeError
  ""
print "area[6] = area[11] + area[51]"
print "area[6] = $area[6]"
# This fails because adding an integer to a string is not permitted.

print ; print ; print

# -----------------------------------------------------------------
# Another array, "area2": a list.

area2 = [zero one two three four]

print "area2[0] = $area2[0]"
# Aha, zero-based indexing (first element of array is [0], not [1]).

print "area2[1] = $area2[1]"    # [1] is second element of array.
# -----------------------------------------------------------------

print ; print ; print

# -----------------------------------------------
# Yet another array, "area3": a dict written out whole.

area3 = {17: seventeen 24: twenty-four}

print "area3[17] = $area3[17]"
print "area3[24] = $area3[24]"
# -----------------------------------------------

exit 0
```

```output
area[11] = 23
area[13] = 37
Contents of area[51] are UFOs.
area[43] = 
(area[43] unassigned)

area[5] = area[11] + area[13]
area[5] = 60
area[6] = area[11] + area[51]
area[6] = 



area2[0] = zero
area2[1] = one



area3[17] = seventeen
area3[24] = twenty-four
```

bash fills an array from a brace expansion, `base64_charset=( {A..Z}
{a..z} {0..9} + / = )`; in tessh, `(list string.ascii_letters)` and the
like, or `list` of any string. And bash lets a plain variable be used as an
array of one element (`${string[0]}` is the string, `${#string[@]}` is 1);
in tessh a string is a sequence of characters, so `$string[0]` is its first
character.

### Example 27-2. Formatting a poem (`poem.sh`)

```bash no-run
#!/bin/bash
# poem.sh: Pretty-prints one of the ABS Guide author's favorite poems.

# Lines of the poem (single stanza).
Line[1]="I do not know which to prefer,"
Line[2]="The beauty of inflections"
Line[3]="Or the beauty of innuendoes,"
Line[4]="The blackbird whistling"
Line[5]="Or just after."
# Note that quoting permits embedding whitespace.

# Attribution.
Attrib[1]=" Wallace Stevens"
Attrib[2]="\"Thirteen Ways of Looking at a Blackbird\""
# This poem is in the Public Domain (copyright expired).

echo

tput bold   # Bold print.

for index in 1 2 3 4 5    # Five lines.
do
  printf "     %s\n" "${Line[index]}"
done

for index in 1 2          # Two attribution lines.
do
  printf "          %s\n" "${Attrib[index]}"
done

tput sgr0   # Reset terminal.
            # See 'tput' docs.

echo

exit 0

# Exercise:
# --------
# Modify this script to pretty-print a poem from a text data file.
```

The lines are a list, gone through with `for`. The bold (`tput bold`) is
only for a terminal, so the tessh version asks whether output goes to one.

```tessh
import sys
# Lines of the poem (single stanza).
Line = [
  "I do not know which to prefer,"
  "The beauty of inflections"
  "Or the beauty of innuendoes,"
  "The blackbird whistling"
  "Or just after."
]

# Attribution.
Attrib = [" Wallace Stevens" "\"Thirteen Ways of Looking at a Blackbird\""]
# This poem is in the Public Domain (copyright expired).

print

bold = ($sys.stdout.isatty)
if $bold
  tput bold   # Bold print.

for line in $Line           # Five lines.
  printf "     %s\n" $line

for line in $Attrib         # Two attribution lines.
  printf "          %s\n" $line

if $bold
  tput sgr0   # Reset terminal.

print

exit 0
```

```output

     I do not know which to prefer,
     The beauty of inflections
     Or the beauty of innuendoes,
     The blackbird whistling
     Or just after.
           Wallace Stevens
          "Thirteen Ways of Looking at a Blackbird"

```

### Example 27-3. Various array operations (`array-ops.sh`)

```bash
#!/bin/bash
# array-ops.sh: More fun with arrays.


array=( zero one two three four five )
# Element 0   1   2    3     4    5

echo ${array[0]}       #  zero
echo ${array:0}        #  zero
                       #  Parameter expansion of first element,
                       #+ starting at position # 0 (1st character).
echo ${array:1}        #  ero
                       #  Parameter expansion of first element,
                       #+ starting at position # 1 (2nd character).

echo "--------------"

echo ${#array[0]}      #  4
                       #  Length of first element of array.
echo ${#array}         #  4
                       #  Length of first element of array.
                       #  (Alternate notation)

echo ${#array[1]}      #  3
                       #  Length of second element of array.
                       #  Arrays in Bash have zero-based indexing.

echo ${#array[*]}      #  6
                       #  Number of elements in array.
echo ${#array[@]}      #  6
                       #  Number of elements in array.

echo "--------------"

array2=( [0]="first element" [1]="second element" [3]="fourth element" )
#            ^     ^       ^     ^      ^       ^     ^      ^       ^
# Quoting permits embedding whitespace within individual array elements.

echo ${array2[0]}      # first element
echo ${array2[1]}      # second element
echo ${array2[2]}      #
                       # Skipped in initialization, and therefore null.
echo ${array2[3]}      # fourth element
echo ${#array2[0]}     # 13    (length of first element)
echo ${#array2[*]}     # 3     (number of elements in array)

exit
```

```output bash
zero
zero
ero
--------------
4
4
3
6
6
--------------
first element
second element

fourth element
13
3
```

In bash an array's name alone is its first element (`${array:1}`,
`${#array}`); in tessh it's the whole list, and its first element is
`$array[0]`. `array2` skips index 2, so it's a dict.

```tessh
array = [zero one two three four five]
# Element 0   1   2    3     4    5

print $array[0]        #  zero
print $array[0][0:]    #  zero
                       #  The first element, from position 0.
print $array[0][1:]    #  ero
                       #  The first element, from position 1.

print "--------------"

print (len $array[0])  #  4
                       #  Length of first element of array.
print (len $array[1])  #  3
                       #  Length of second element of array.
print (len $array)     #  6
                       #  Number of elements in array.

print "--------------"

array2 = {0: "first element" 1: "second element" 3: "fourth element"}

print $array2[0]           # first element
print $array2[1]           # second element
print ($array2.get 2 "")   #
                           # Skipped in initialization, and therefore missing.
print $array2[3]           # fourth element
print (len $array2[0])     # 13    (length of first element)
print (len $array2)        # 3     (number of elements in array)

exit
```

```output
zero
zero
ero
--------------
4
3
6
--------------
first element
second element

fourth element
13
3
```

### Example 27-4. String operations on arrays (`array-strops.sh`)

```bash
#!/bin/bash
# array-strops.sh: String operations on arrays.

# Script by Michael Zick.
# Used in ABS Guide with permission.
# Fixups: 05 May 08, 04 Aug 08.

#  In general, any string operation using the ${name ... } notation
#+ can be applied to all string elements in an array,
#+ with the ${name[@] ... } or ${name[*] ...} notation.


arrayZ=( one two three four five five )

echo

# Trailing Substring Extraction
echo ${arrayZ[@]:0}     # one two three four five five
#                ^        All elements.

echo ${arrayZ[@]:1}     # two three four five five
#                ^        All elements following element[0].

echo ${arrayZ[@]:1:2}   # two three
#                  ^      Only the two elements after element[0].

echo "---------"


# Substring Removal

# Removes shortest match from front of string(s).

echo ${arrayZ[@]#f*r}   # one two three five five
#               ^       # Applied to all elements of the array.
                        # Matches "four" and removes it.

# Longest match from front of string(s)
echo ${arrayZ[@]##t*e}  # one two four five five
#               ^^      # Applied to all elements of the array.
                        # Matches "three" and removes it.

# Shortest match from back of string(s)
echo ${arrayZ[@]%h*e}   # one two t four five five
#               ^       # Applied to all elements of the array.
                        # Matches "hree" and removes it.

# Longest match from back of string(s)
echo ${arrayZ[@]%%t*e}  # one two four five five
#               ^^      # Applied to all elements of the array.
                        # Matches "three" and removes it.

echo "----------------------"


# Substring Replacement

# Replace first occurrence of substring with replacement.
echo ${arrayZ[@]/fiv/XYZ}   # one two three four XYZe XYZe
#               ^           # Applied to all elements of the array.

# Replace all occurrences of substring.
echo ${arrayZ[@]//iv/YY}    # one two three four fYYe fYYe
                            # Applied to all elements of the array.

# Delete all occurrences of substring.
# Not specifing a replacement defaults to 'delete' ...
echo ${arrayZ[@]//fi/}      # one two three four ve ve
#               ^^          # Applied to all elements of the array.

# Replace front-end occurrences of substring.
echo ${arrayZ[@]/#fi/XY}    # one two three four XYve XYve
#                ^          # Applied to all elements of the array.

# Replace back-end occurrences of substring.
echo ${arrayZ[@]/%ve/ZZ}    # one two three four fiZZ fiZZ
#                ^          # Applied to all elements of the array.

echo ${arrayZ[@]/%o/XX}     # one twXX three four five five
#                ^          # Why?

echo "-----------------------------"


replacement() {
    echo -n "!!!"
}

echo ${arrayZ[@]/%e/$(replacement)}
#                ^  ^^^^^^^^^^^^^^
# on!!! two thre!!! four fiv!!! fiv!!!
# The stdout of replacement() is the replacement string.
# Q.E.D: The replacement action is, in effect, an 'assignment.'

echo "------------------------------------"

#  Accessing the "for-each":
echo ${arrayZ[@]//*/$(replacement optional_arguments)}
#                ^^ ^^^^^^^^^^^^^
# !!! !!! !!! !!! !!! !!!

#  Now, if Bash would only pass the matched string
#+ to the function being called . . .

echo

exit 0

#  Before reaching for a Big Hammer -- Perl, Python, or all the rest --
#  recall:
#    $( ... ) is command substitution.
#    A function runs as a sub-process.
#    A function writes its output (if echo-ed) to stdout.
#    Assignment, in conjunction with "echo" and command substitution,
#+   can read a function's stdout.
#    The name[@] notation specifies (the equivalent of) a "for-each"
#+   operation.
#  Bash is more powerful than you think!
```

```output bash

one two three four five five
two three four five five
two three
---------
one two three five five
one two four five five
one two t four five five
one two four five five
----------------------
one two three four XYZe XYZe
one two three four fYYe fYYe
one two three four ve ve
one two three four XYve XYve
one two three four fiZZ fiZZ
one twXX three four five five
-----------------------------
on!!! two thre!!! four fiv!!! fiv!!!
------------------------------------
!!! !!! !!! !!! !!! !!!

```

bash's `${name[@]…}` does a string operation on every element. tessh does
it with a comprehension, `[… for e in $arrayZ]`, and the operation is
Python's: a slice, `.replace`, or a regex's `.sub` for bash's patterns
(`#f*r`, the shortest match at the front, is `re'^f.*?r'`; `##t*e`, the
longest, is `re'^t.*e'`; and so on). `show` prints a list as bash's
unquoted `echo ${…}` does, leaving out the elements that came out empty.
And since a tessh function gives back a value, the replacement can be
computed from what was matched, which is the ABS Guide's wish at the end.

```tessh
arrayZ = [one two three four five five]

def show xs
  kept = [$x for x in $xs if $x]
  print *$kept

print

# Trailing Substring Extraction
show $arrayZ[0:]      # one two three four five five
                      # All elements.

show $arrayZ[1:]      # two three four five five
                      # All elements following element[0].

show $arrayZ[1:3]     # two three
                      # Only the two elements after element[0].

print "---------"


# Substring Removal

# Removes shortest match from front of string(s).
show [(re'^f.*?r'.sub "" $e) for e in $arrayZ]    # one two three five five
                        # Applied to all elements of the array.
                        # Matches "four" and removes it.

# Longest match from front of string(s)
show [(re'^t.*e'.sub "" $e) for e in $arrayZ]     # one two four five five
                        # Matches "three" and removes it.

# Shortest match from back of string(s)
show [(re'h[^h]*e$'.sub "" $e) for e in $arrayZ]  # one two t four five five
                        # Matches "hree" and removes it.

# Longest match from back of string(s)
show [(re't.*e$'.sub "" $e) for e in $arrayZ]     # one two four five five
                        # Matches "three" and removes it.

print "----------------------"


# Substring Replacement

# Replace first occurrence of substring with replacement.
show [($e.replace fiv XYZ 1) for e in $arrayZ]    # one two three four XYZe XYZe

# Replace all occurrences of substring.
show [($e.replace iv YY) for e in $arrayZ]        # one two three four fYYe fYYe

# Delete all occurrences of substring.
show [($e.replace fi "") for e in $arrayZ]        # one two three four ve ve

# Replace front-end occurrences of substring.
show [(re'^fi'.sub XY $e) for e in $arrayZ]       # one two three four XYve XYve

# Replace back-end occurrences of substring.
show [(re've$'.sub ZZ $e) for e in $arrayZ]       # one two three four fiZZ fiZZ

show [(re'o$'.sub XX $e) for e in $arrayZ]        # one twXX three four five five
#                                                   Why?

print "-----------------------------"


def replacement *args
  "!!!"

show [(re'e$'.sub (replacement) $e) for e in $arrayZ]
# on!!! two thre!!! four fiv!!! fiv!!!
# What replacement() gives back is the replacement string.

print "------------------------------------"

#  Accessing the "for-each":
show [(replacement optional_arguments) for e in $arrayZ]
# !!! !!! !!! !!! !!! !!!

#  Now, if Bash would only pass the matched string
#+ to the function being called . . .
#  tessh does: .sub calls a function with each match.
show [(re'[aeiou]'.sub (fn (.upper ($%.group 0))) $e) for e in $arrayZ]

print

exit 0
```

```output

one two three four five five
two three four five five
two three
---------
one two three five five
one two four five five
one two t four five five
one two four five five
----------------------
one two three four XYZe XYZe
one two three four fYYe fYYe
one two three four ve ve
one two three four XYve XYve
one two three four fiZZ fiZZ
one twXX three four five five
-----------------------------
on!!! two thre!!! four fiv!!! fiv!!!
------------------------------------
!!! !!! !!! !!! !!! !!!
OnE twO thrEE fOUr fIvE fIvE

```

### Example 27-5. Loading the contents of a script into an array (`script-array.sh`)

```bash
#!/bin/bash
# script-array.sh: Loads this script into an array.
# Inspired by an e-mail from Chris Martin (thanks!).

script_contents=( $(cat "$0") )  #  Stores contents of this script ($0)
                                 #+ in an array.

for element in $(seq 0 $((${#script_contents[@]} - 1)))
  do                #  ${#script_contents[@]}
                    #+ gives number of elements in the array.
                    #
                    #  Question:
                    #  Why is  seq 0  necessary?
                    #  Try changing it to seq 1.
  echo -n "${script_contents[$element]}"
                    # List each field of this script on a single line.
# echo -n "${script_contents[element]}" also works because of ${ ... }.
  echo -n " -- "    # Use " -- " as a field separator.
done

echo

exit 0

# Exercise:
# --------
#  Modify this script so it lists itself
#+ in its original format,
#+ complete with whitespace, line breaks, etc.
```

```output bash
#!/bin/bash -- # -- script-array.sh: -- Loads -- this -- script -- into -- an -- array. -- # -- Inspired -- by -- an -- e-mail -- from -- Chris -- Martin -- (thanks!). -- script_contents=( -- $(cat -- "$0") -- ) -- # -- Stores -- contents -- of -- this -- script -- ($0) -- #+ -- in -- an -- array. -- for -- element -- in -- $(seq -- 0 -- $((${#script_contents[@]} -- - -- 1))) -- do -- # -- ${#script_contents[@]} -- #+ -- gives -- number -- of -- elements -- in -- the -- array. -- # -- # -- Question: -- # -- Why -- is -- seq -- 0 -- necessary? -- # -- Try -- changing -- it -- to -- seq -- 1. -- echo --  -- "${script_contents[$element]}" -- # -- List -- each -- field -- of -- this -- script -- on -- a -- single -- line. -- # -- echo --  -- "${script_contents[element]}" -- also -- works -- because -- of -- ${ -- ... -- }. -- echo --  -- " -- -- -- " -- # -- Use -- " -- -- -- " -- as -- a -- field -- separator. -- done -- echo -- exit -- 0 -- # -- Exercise: -- # -- -------- -- # -- Modify -- this -- script -- so -- it -- lists -- itself -- #+ -- in -- its -- original -- format, -- #+ -- complete -- with -- whitespace, -- line -- breaks, -- etc. -- 
```

`.split` makes the list of words. The loop goes over the words themselves;
the index (`range`, for bash's `seq 0 …`) isn't needed.

```tessh
script_contents = ((cat $argv[0]).split)   #  Stores contents of this script
                                         #+ ($argv[0]) in a list of words.

for element in $script_contents
  print $element end=""      # List each field of this script on a single line.
  print " -- " end=""        # Use " -- " as a field separator.

print

exit 0
```

```output
script_contents -- = -- ((cat -- $argv[0]).split) -- # -- Stores -- contents -- of -- this -- script -- #+ -- ($argv[0]) -- in -- a -- list -- of -- words. -- for -- element -- in -- $script_contents -- print -- $element -- end="" -- # -- List -- each -- field -- of -- this -- script -- on -- a -- single -- line. -- print -- " -- -- -- " -- end="" -- # -- Use -- " -- -- -- " -- as -- a -- field -- separator. -- print -- exit -- 0 -- 
```

`unset` deletes an element of an array, or the whole array; in tessh `del`
does both.

### Example 27-6. Some special properties of arrays (`ex67.sh`)

```bash no-run
#!/bin/bash

declare -a colors
#  All subsequent commands in this script will treat
#+ the variable "colors" as an array.

echo "Enter your favorite colors (separated from each other by a space)."

read -a colors    # Enter at least 3 colors to demonstrate features below.
#  Special option to 'read' command,
#+ allowing assignment of elements in an array.

echo

element_count=${#colors[@]}
# Special syntax to extract number of elements in array.
#     element_count=${#colors[*]} works also.
#
#  The "@" variable allows word splitting within quotes
#+ (extracts variables separated by whitespace).
#
#  This corresponds to the behavior of "$@" and "$*"
#+ in positional parameters. 

index=0

while [ "$index" -lt "$element_count" ]
do    # List all the elements in the array.
  echo ${colors[$index]}
  #    ${colors[index]} also works because it's within ${ ... } brackets.
  let "index = $index + 1"
  # Or:
  #    ((index++))
done
# Each array element listed on a separate line.
# If this is not desired, use  echo -n "${colors[$index]} "
#
# Doing it with a "for" loop instead:
#   for i in "${colors[@]}"
#   do
#     echo "$i"
#   done
# (Thanks, S.C.)

echo

# Again, list all the elements in the array, but using a more elegant method.
  echo ${colors[@]}          # echo ${colors[*]} also works.

echo

# The "unset" command deletes elements of an array, or entire array.
unset colors[1]              # Remove 2nd element of array.
                             # Same effect as   colors[1]=
echo  ${colors[@]}           # List array again, missing 2nd element.

unset colors                 # Delete entire array.
                             #  unset colors[*] and
                             #+ unset colors[@] also work.
echo; echo -n "Colors gone."			   
echo ${colors[@]}            # List array again, now empty.

exit 0
```

`read -a` splits what's typed into an array; here the typing is a canned
string, split with `.split`. A list's `del` closes the gap, where bash
leaves a hole.

```tessh
print "Enter your favorite colors (separated from each other by a space)."

colors = ("red green blue yellow".split)  # (read -a colors: answered here)

print

element_count = len $colors
# The number of elements in the list.

index = 0

while $index < $element_count
  # List all the elements in the array.
  print $colors[$index]
  index += 1
# Each array element listed on a separate line.
#
# Doing it with a "for" loop instead:
#   for i in $colors
#     print $i

print

# Again, list all the elements in the array, but using a more elegant method.
print *$colors

print

# del deletes elements of a list, or the entire list.
del $colors[1]               # Remove 2nd element of array.
print *$colors               # List array again, missing 2nd element.

del colors                   # Delete entire list (the name too).
print ; print "Colors gone." ("colors" in (globals))

exit 0
```

```output
Enter your favorite colors (separated from each other by a space).

red
green
blue
yellow

red green blue yellow

red blue yellow

Colors gone. False
```

An empty array is not the same as an array with an empty element in it.

### Example 27-7. Of empty arrays and empty elements (`empty-array.sh`)

```bash
#!/bin/bash
# empty-array.sh

#  Thanks to Stephane Chazelas for the original example,
#+ and to Michael Zick and Omair Eshkenazi, for extending it.
#  And to Nathan Coulter for clarifications and corrections.


# An empty array is not the same as an array with empty elements.

  array0=( first second third )
  array1=( '' )   # "array1" consists of one empty element.
  array2=( )      # No elements . . . "array2" is empty.
  array3=(   )    # What about this array?


echo
ListArray()
{
echo
echo "Elements in array0:  ${array0[@]}"
echo "Elements in array1:  ${array1[@]}"
echo "Elements in array2:  ${array2[@]}"
echo "Elements in array3:  ${array3[@]}"
echo
echo "Length of first element in array0 = ${#array0}"
echo "Length of first element in array1 = ${#array1}"
echo "Length of first element in array2 = ${#array2}"
echo "Length of first element in array3 = ${#array3}"
echo
echo "Number of elements in array0 = ${#array0[*]}"  # 3
echo "Number of elements in array1 = ${#array1[*]}"  # 1  (Surprise!)
echo "Number of elements in array2 = ${#array2[*]}"  # 0
echo "Number of elements in array3 = ${#array3[*]}"  # 0
}

# ===================================================================

ListArray

# Try extending those arrays.

# Adding an element to an array.
array0=( "${array0[@]}" "new1" )
array1=( "${array1[@]}" "new1" )
array2=( "${array2[@]}" "new1" )
array3=( "${array3[@]}" "new1" )

ListArray

# or
array0[${#array0[*]}]="new2"
array1[${#array1[*]}]="new2"
array2[${#array2[*]}]="new2"
array3[${#array3[*]}]="new2"

ListArray

# When extended as above, arrays are 'stacks' ...
# Above is the 'push' ...
# The stack 'height' is:
height=${#array2[@]}
echo
echo "Stack height for array2 = $height"

# The 'pop' is:
unset array2[${#array2[@]}-1]   #  Arrays are zero-based,
height=${#array2[@]}            #+ which means first element has index 0.
echo
echo "POP"
echo "New stack height for array2 = $height"

ListArray

# List only 2nd and 3rd elements of array0.
from=1		    # Zero-based numbering.
to=2
array3=( ${array0[@]:1:2} )
echo
echo "Elements in array3:  ${array3[@]}"

# Works like a string (array of characters).
# Try some other "string" forms.

# Replacement:
array4=( ${array0[@]/second/2nd} )
echo
echo "Elements in array4:  ${array4[@]}"

# Replace all matching wildcarded string.
array5=( ${array0[@]//new?/old} )
echo
echo "Elements in array5:  ${array5[@]}"

# Just when you are getting the feel for this . . .
array6=( ${array0[@]#*new} )
echo # This one might surprise you.
echo "Elements in array6:  ${array6[@]}"

array7=( ${array0[@]#new1} )
echo # After array6 this should not be a surprise.
echo "Elements in array7:  ${array7[@]}"

# Which looks a lot like . . .
array8=( ${array0[@]/new1/} )
echo
echo "Elements in array8:  ${array8[@]}"

#  So what can one say about this?

#  The string operations are performed on
#+ each of the elements in var[@] in succession.
#  Therefore : Bash supports string vector operations.
#  If the result is a zero length string,
#+ that element disappears in the resulting assignment.
#  However, if the expansion is in quotes, the null elements remain.

#  Michael Zick:    Question, are those strings hard or soft quotes?
#  Nathan Coulter:  There is no such thing as "soft quotes."
#!    What's really happening is that
#!+   the pattern matching happens after
#!+   all the other expansions of [word]
#!+   in cases like ${parameter#word}.


zap='new*'
array9=( ${array0[@]/$zap/} )
echo
echo "Number of elements in array9:  ${#array9[@]}"
array9=( "${array0[@]/$zap/}" )
echo "Elements in array9:  ${array9[@]}"
# This time the null elements remain.
echo "Number of elements in array9:  ${#array9[@]}"


# Just when you thought you were still in Kansas . . .
array10=( ${array0[@]#$zap} )
echo
echo "Elements in array10:  ${array10[@]}"
# But, the asterisk in zap won't be interpreted if quoted.
array10=( ${array0[@]#"$zap"} )
echo
echo "Elements in array10:  ${array10[@]}"
# Well, maybe we _are_ still in Kansas . . .
# (Revisions to above code block by Nathan Coulter.)


#  Compare array7 with array10.
#  Compare array8 with array9.

#  Reiterating: No such thing as soft quotes!
#  Nathan Coulter explains:
#  Pattern matching of 'word' in ${parameter#word} is done after
#+ parameter expansion and *before* quote removal.
#  In the normal case, pattern matching is done *after* quote removal.
 
exit
```

In tessh `['']` has one element and `[]` none, as in bash. The string
operations are comprehensions; bash drops the elements that came out empty
when the expansion isn't quoted, so the tessh version filters them out
with `if $e` where bash would, and keeps them where bash's quotes do.
bash's `#` and `/` take shell patterns (`new?`, `*new`, `new*`); here
they're regexes or plain text, as each one means.

```tessh
import re
# An empty array is not the same as an array with empty elements.

array0 = [first second third]
array1 = ['']   # "array1" consists of one empty element.
array2 = []     # No elements . . . "array2" is empty.
array3 = [  ]   # What about this array?

def first_len a      # bash's ${#a}: the length of the first element.
  if $a then (len $a[0]) else 0

print
def ListArray
  print
  print "Elements in array0:  $(' '.join $array0)"
  print "Elements in array1:  $(' '.join $array1)"
  print "Elements in array2:  $(' '.join $array2)"
  print "Elements in array3:  $(' '.join $array3)"
  print
  print "Length of first element in array0 = $(first_len $array0)"
  print "Length of first element in array1 = $(first_len $array1)"
  print "Length of first element in array2 = $(first_len $array2)"
  print "Length of first element in array3 = $(first_len $array3)"
  print
  print "Number of elements in array0 = $(len $array0)"  # 3
  print "Number of elements in array1 = $(len $array1)"  # 1  (Surprise!)
  print "Number of elements in array2 = $(len $array2)"  # 0
  print "Number of elements in array3 = $(len $array3)"  # 0

# ===================================================================

ListArray

# Try extending those arrays.

# Adding an element to an array.
array0 = $array0 + [new1]
array1 = $array1 + [new1]
array2 = $array2 + [new1]
array3 = $array3 + [new1]

ListArray

# or
$array0.append new2
$array1.append new2
$array2.append new2
$array3.append new2

ListArray

# When extended as above, arrays are 'stacks' ...
# Above is the 'push' ...
# The stack 'height' is:
height = len $array2
print
print "Stack height for array2 = $height"

# The 'pop' is:
$array2.pop
height = len $array2
print
print "POP"
print "New stack height for array2 = $height"

ListArray

# List only 2nd and 3rd elements of array0.
array3 = $array0[1:3]
print
print "Elements in array3:  $(' '.join $array3)"

# Replacement (the first "second" in each):
array4 = [($e.replace second 2nd 1) for e in $array0]
print
print "Elements in array4:  $(' '.join $array4)"

# Replace all matching wildcarded string (new? is new.).
array5 = [(re'new.'.sub old $e) for e in $array0]
print
print "Elements in array5:  $(' '.join $array5)"

# Shortest match of *new at the front.
array6 = [(re'^.*?new'.sub "" $e) for e in $array0]
print # This one might surprise you.
print "Elements in array6:  $(' '.join $array6)"

array7 = [$e for e in [($e.removeprefix new1) for e in $array0] if $e]
print # After array6 this should not be a surprise.
print "Elements in array7:  $(' '.join $array7)"

# Which looks a lot like . . .
array8 = [$e for e in [($e.replace new1 "" 1) for e in $array0] if $e]
print
print "Elements in array8:  $(' '.join $array8)"

#  The string operations are performed on each of the elements in turn.
#  bash drops the elements that come out empty, unless quoted;
#+ tessh keeps them unless told not to.

zap = re'new.*'
array9 = [$e for e in [($zap.sub "" $e 1) for e in $array0] if $e]
print
print "Number of elements in array9:  $(len $array9)"
array9 = [($zap.sub "" $e 1) for e in $array0]
print "Elements in array9:  $(' '.join $array9)"
# This time the null elements remain.
print "Number of elements in array9:  $(len $array9)"


# Shortest match of new* at the front (that's "new").
array10 = [(re'^new'.sub "" $e) for e in $array0]
print
print "Elements in array10:  $(' '.join $array10)"
# The text 'new*', taken as it is, matches nothing.
array10 = [($e.removeprefix 'new*') for e in $array0]
print
print "Elements in array10:  $(' '.join $array10)"

exit
```

```output


Elements in array0:  first second third
Elements in array1:  
Elements in array2:  
Elements in array3:  

Length of first element in array0 = 5
Length of first element in array1 = 0
Length of first element in array2 = 0
Length of first element in array3 = 0

Number of elements in array0 = 3
Number of elements in array1 = 1
Number of elements in array2 = 0
Number of elements in array3 = 0

Elements in array0:  first second third new1
Elements in array1:   new1
Elements in array2:  new1
Elements in array3:  new1

Length of first element in array0 = 5
Length of first element in array1 = 0
Length of first element in array2 = 4
Length of first element in array3 = 4

Number of elements in array0 = 4
Number of elements in array1 = 2
Number of elements in array2 = 1
Number of elements in array3 = 1

Elements in array0:  first second third new1 new2
Elements in array1:   new1 new2
Elements in array2:  new1 new2
Elements in array3:  new1 new2

Length of first element in array0 = 5
Length of first element in array1 = 0
Length of first element in array2 = 4
Length of first element in array3 = 4

Number of elements in array0 = 5
Number of elements in array1 = 3
Number of elements in array2 = 2
Number of elements in array3 = 2

Stack height for array2 = 2

POP
New stack height for array2 = 1

Elements in array0:  first second third new1 new2
Elements in array1:   new1 new2
Elements in array2:  new1
Elements in array3:  new1 new2

Length of first element in array0 = 5
Length of first element in array1 = 0
Length of first element in array2 = 4
Length of first element in array3 = 4

Number of elements in array0 = 5
Number of elements in array1 = 3
Number of elements in array2 = 1
Number of elements in array3 = 2

Elements in array3:  second third

Elements in array4:  first 2nd third new1 new2

Elements in array5:  first second third old old

Elements in array6:  first second third 1 2

Elements in array7:  first second third new2

Elements in array8:  first second third new2

Number of elements in array9:  3
Elements in array9:  first second third  
Number of elements in array9:  5

Elements in array10:  first second third 1 2

Elements in array10:  first second third new1 new2
```

Copying an array copies its elements; bash's `array2=( "${array1[@]}" )`
loses the indexes of a sparse one. In tessh, `(list $a)` or `$a[:]` copies
a list and `($d.copy)` a dict, indexes and all; a plain `b = $a` makes a
second name for the *same* list. Adding an element is `.append`.

```bash
# Copying an array.
array2=( "${array1[@]}" )
# or
array2="${array1[@]}"
#
#  However, this fails with "sparse" arrays,
#+ arrays with holes (missing elements) in them,
#+ as Jochen DeSmet points out.
# ------------------------------------------
  array1[0]=0
# array1[1] not assigned
  array1[2]=2
  array2=( "${array1[@]}" )       # Copy it?

echo ${array2[0]}      # 0
echo ${array2[2]}      # (null), should be 2
# ------------------------------------------

# Adding an element to an array.
array=( "${array[@]}" "new element" )
# or
array[${#array[*]}]="new element"

# Thanks, S.C.
```

```output bash
0

```

```tessh
array1 = {0: 0 2: 2}         # Sparse: a dict.
array2 = ($array1.copy)      # Copy it.

print $array2[0]      # 0
print $array2[2]      # 2

same = $array1        # Not a copy: the same dict, by another name.
$same[3] = 3
print $array1         # {0: 0 2: 2 3: 3}

# Adding an element to a list.
array = [old]
$array.append "new element"
print $array
```

```output
0
2
{0: 0 2: 2 3: 3}
['old' 'new element']
```

Command substitution fills an array from a file, a word per element:

```bash no-run
#!/bin/bash

filename=sample_file

#            cat sample_file
#
#            1 a b c
#            2 d e fg

declare -a array1

array1=( `cat "$filename"`)                #  Loads contents
#         List file to stdout              #+ of $filename into array1.
#
#  array1=( `cat "$filename" | tr '\n' ' '`)
#                            change linefeeds in file to spaces. 
#  Not necessary because Bash does word splitting,
#+ changing linefeeds to spaces.

echo ${array1[@]}            # List the array.
#                              1 a b c 2 d e fg
#
#  Each whitespace-separated "word" in the file
#+ has been assigned to an element of the array.

element_count=${#array1[*]}
echo $element_count          # 8
```

```tessh
printf "1 a b c\n2 d e fg\n" >sample_file    # (The sample_file.)
filename = sample_file

array1 = ((cat $filename).split)     # Loads the words of $filename into array1.
# (Or without cat: (open $filename).read.split. And .lines for lines.)

print *$array1               # List the array.
#                              1 a b c 2 d e fg

element_count = len $array1
print $element_count         # 8
```

```output
1 a b c 2 d e fg
8
```

### Example 27-8. Initializing arrays (`array-assign.bash`)

```bash no-run
#! /bin/bash
# array-assign.bash

#  Array operations are Bash-specific,
#+ hence the ".bash" in the script name.

# Copyright (c) Michael S. Zick, 2003, All rights reserved.
# License: Unrestricted reuse in any form, for any purpose.
# Version: $ID$
#
# Clarification and additional comments by William Park.

#  Based on an example provided by Stephane Chazelas
#+ which appeared in an earlier version of the
#+ Advanced Bash Scripting Guide.

# Output format of the 'times' command:
# User CPU <space> System CPU
# User CPU of dead children <space> System CPU of dead children

#  Bash has two versions of assigning all elements of an array
#+ to a new array variable.
#  Both drop 'null reference' elements
#+ in Bash versions 2.04 and later.
#  An additional array assignment that maintains the relationship of
#+ [subscript]=value for arrays may be added to newer versions.

#  Constructs a large array using an internal command,
#+ but anything creating an array of several thousand elements
#+ will do just fine.

declare -a bigOne=( /dev/* )  # All the files in /dev . . .
echo
echo 'Conditions: Unquoted, default IFS, All-Elements-Of'
echo "Number of elements in array is ${#bigOne[@]}"

# set -vx



echo
echo '- - testing: =( ${array[@]} ) - -'
times
declare -a bigTwo=( ${bigOne[@]} )
# Note parens:    ^              ^
times


echo
echo '- - testing: =${array[@]} - -'
times
declare -a bigThree=${bigOne[@]}
# No parentheses this time.
times

#  Comparing the numbers shows that the second form, pointed out
#+ by Stephane Chazelas, is faster.
#
#  As William Park explains:
#+ The bigTwo array assigned element by element (because of parentheses),
#+ whereas bigThree assigned as a single string.
#  So, in essence, you have:
#                   bigTwo=( [0]="..." [1]="..." [2]="..." ... )
#                   bigThree=( [0]="... ... ..." )
#
#  Verify this by:  echo ${bigTwo[0]}
#                   echo ${bigThree[0]}


#  I will continue to use the first form in my example descriptions
#+ because I think it is a better illustration of what is happening.

#  The reusable portions of my examples will actual contain
#+ the second form where appropriate because of the speedup.

# MSZ: Sorry about that earlier oversight folks.


#  Note:
#  ----
#  The "declare -a" statements in lines 32 and 44
#+ are not strictly necessary, since it is implicit
#+ in the  Array=( ... )  assignment form.
#  However, eliminating these declarations slows down
#+ the execution of the following sections of the script.
#  Try it, and see.

exit 0
```

This one times two ways of copying a big array, with `times`; its numbers
change from run to run, as do the names in `/dev`. In tessh the ways are
copying the list and joining it into one string (what bash's
`bigThree=${bigOne[@]}` makes), timed with `time.perf_counter`. Both take
microseconds.

```tessh
import time
bigOne = (list /dev/*)  # All the files in /dev . . .
print
print "Number of elements in array is $(len $bigOne)"

def timed label f
  t = (time.perf_counter)
  r = (f)
  print "$label: ${((time.perf_counter) - $t) * 1e6:.0f} µs"
  $r

print
print '- - testing: copy the list - -'
bigTwo = timed "list" (fn (list $bigOne))

print
print '- - testing: join into a string - -'
bigThree = timed "join" (fn (' '.join (map $str $bigOne)))

#  bigTwo has an element per file, bigThree is one string:
print (len $bigTwo) (type $bigThree).__name__

exit 0
```

```text
(one run)

Number of elements in array is 240

- - testing: copy the list - -
list: 2 µs

- - testing: join into a string - -
join: 35 µs
240 str
```

### Example 27-9. Copying and concatenating arrays (`CopyArray.sh`)

```bash
#! /bin/bash
# CopyArray.sh
#
# This script written by Michael Zick.
# Used here with permission.

#  How-To "Pass by Name & Return by Name"
#+ or "Building your own assignment statement".


CpArray_Mac() {

# Assignment Command Statement Builder

    echo -n 'eval '
    echo -n "$2"                    # Destination name
    echo -n '=( ${'
    echo -n "$1"                    # Source name
    echo -n '[@]} )'

# That could all be a single command.
# Matter of style only.
}

declare -f CopyArray                # Function "Pointer"
CopyArray=CpArray_Mac               # Statement Builder

Hype()
{

# Hype the array named $1.
# (Splice it together with array containing "Really Rocks".)
# Return in array named $2.

    local -a TMP
    local -a hype=( Really Rocks )

    $($CopyArray $1 TMP)
    TMP=( ${TMP[@]} ${hype[@]} )
    $($CopyArray TMP $2)
}

declare -a before=( Advanced Bash Scripting )
declare -a after

echo "Array Before = ${before[@]}"

Hype before after

echo "Array After = ${after[@]}"

# Too much hype?

echo "What ${after[@]:3:2}?"

declare -a modest=( ${after[@]:2:1} ${after[@]:3:2} )
#                    ---- substring extraction ----

echo "Array Modest = ${modest[@]}"

# What happened to 'before' ?

echo "Array Before = ${before[@]}"

exit 0
```

The bash builds an `eval` statement to copy an array by name. tessh has
the names in `(globals)`, so a function can copy by name directly. (The
usual tessh is simpler still: `after = $before + [Really Rocks]`.)

```tessh
#  How-To "Pass by Name & Return by Name"

def CopyArray src dst     # Copy the list named src to the one named dst.
  g = (globals)
  $g[$dst] = (list $g[$src])

def Hype src dst
  # Hype the array named src.
  # (Splice it together with array containing "Really Rocks".)
  # Return in array named dst.
  hype = [Really Rocks]
  CopyArray $src TMP
  (globals)[TMP] = $TMP + $hype
  CopyArray TMP $dst

before = [Advanced Bash Scripting]

print "Array Before = $(' '.join $before)"

Hype before after

print "Array After = $(' '.join $after)"

# Too much hype?

print "What $(' '.join $after[3:5])?"

modest = $after[2:3] + $after[3:5]
#        ---- slices ----

print "Array Modest = $(' '.join $modest)"

# What happened to 'before' ?

print "Array Before = $(' '.join $before)"

exit 0
```

```output
Array Before = Advanced Bash Scripting
Array After = Advanced Bash Scripting Really Rocks
What Really Rocks?
Array Modest = Scripting Really Rocks
Array Before = Advanced Bash Scripting
```

### Example 27-10. More on concatenating arrays (`array-append.bash`)

```bash
#! /bin/bash
# array-append.bash

# Copyright (c) Michael S. Zick, 2003, All rights reserved.
# License: Unrestricted reuse in any form, for any purpose.
# Version: $ID$
#
# Slightly modified in formatting by M.C.


# Array operations are Bash-specific.
# Legacy UNIX /bin/sh lacks equivalents.


#  Pipe the output of this script to 'more'
#+ so it doesn't scroll off the terminal.
#  Or, redirect output to a file.


declare -a array1=( zero1 one1 two1 )
# Subscript packed.
declare -a array2=( [0]=zero2 [2]=two2 [3]=three2 )
# Subscript sparse -- [1] is not defined.

echo
echo '- Confirm that the array is really subscript sparse. -'
echo "Number of elements: 4"        # Hard-coded for illustration.
for (( i = 0 ; i < 4 ; i++ ))
do
    echo "Element [$i]: ${array2[$i]}"
done
# See also the more general code example in basics-reviewed.bash.


declare -a dest

# Combine (append) two arrays into a third array.
echo
echo 'Conditions: Unquoted, default IFS, All-Elements-Of operator'
echo '- Undefined elements not present, subscripts not maintained. -'
# # The undefined elements do not exist; they are not being dropped.

dest=( ${array1[@]} ${array2[@]} )
# dest=${array1[@]}${array2[@]}     # Strange results, possibly a bug.

# Now, list the result.
echo
echo '- - Testing Array Append - -'
cnt=${#dest[@]}

echo "Number of elements: $cnt"
for (( i = 0 ; i < cnt ; i++ ))
do
    echo "Element [$i]: ${dest[$i]}"
done

# Assign an array to a single array element (twice).
dest[0]=${array1[@]}
dest[1]=${array2[@]}

# List the result.
echo
echo '- - Testing modified array - -'
cnt=${#dest[@]}

echo "Number of elements: $cnt"
for (( i = 0 ; i < cnt ; i++ ))
do
    echo "Element [$i]: ${dest[$i]}"
done

# Examine the modified second element.
echo
echo '- - Reassign and list second element - -'

declare -a subArray=${dest[1]}
cnt=${#subArray[@]}

echo "Number of elements: $cnt"
for (( i = 0 ; i < cnt ; i++ ))
do
    echo "Element [$i]: ${subArray[$i]}"
done

#  The assignment of an entire array to a single element
#+ of another array using the '=${ ... }' array assignment
#+ has converted the array being assigned into a string,
#+ with the elements separated by a space (the first character of IFS).

# If the original elements didn't contain whitespace . . .
# If the original array isn't subscript sparse . . .
# Then we could get the original array structure back again.

# Restore from the modified second element.
echo
echo '- - Listing restored element - -'

declare -a subArray=( ${dest[1]} )
cnt=${#subArray[@]}

echo "Number of elements: $cnt"
for (( i = 0 ; i < cnt ; i++ ))
do
    echo "Element [$i]: ${subArray[$i]}"
done
echo '- - Do not depend on this behavior. - -'
echo '- - This behavior is subject to change - -'
echo '- - in versions of Bash newer than version 2.05b - -'

# MSZ: Sorry about any earlier confusion folks.

exit 0
```

```output bash

- Confirm that the array is really subscript sparse. -
Number of elements: 4
Element [0]: zero2
Element [1]: 
Element [2]: two2
Element [3]: three2

Conditions: Unquoted, default IFS, All-Elements-Of operator
- Undefined elements not present, subscripts not maintained. -

- - Testing Array Append - -
Number of elements: 6
Element [0]: zero1
Element [1]: one1
Element [2]: two1
Element [3]: zero2
Element [4]: two2
Element [5]: three2

- - Testing modified array - -
Number of elements: 6
Element [0]: zero1 one1 two1
Element [1]: zero2 two2 three2
Element [2]: two1
Element [3]: zero2
Element [4]: two2
Element [5]: three2

- - Reassign and list second element - -
Number of elements: 1
Element [0]: zero2 two2 three2

- - Listing restored element - -
Number of elements: 3
Element [0]: zero2
Element [1]: two2
Element [2]: three2
- - Do not depend on this behavior. - -
- - This behavior is subject to change - -
- - in versions of Bash newer than version 2.05b - -
```

`array2` is sparse, so it's a dict, and joining it to the list takes its
values. Putting a whole array in one element makes the element a string in
bash; in tessh it could hold the list itself, but the string is kept here
to follow the original (`.split` gets the list back).

```tessh
array1 = [zero1 one1 two1]
# Subscript packed: a list.
array2 = {0: zero2 2: two2 3: three2}
# Subscript sparse -- [1] is not defined: a dict.

print
print '- Confirm that the array is really subscript sparse. -'
print "Number of elements: 4"        # Hard-coded for illustration.
for i in (range 4)
  print "Element [$i]: $($array2.get $i '')"

# Combine (append) two arrays into a third array.
print
print '- Undefined elements not present, subscripts not maintained. -'

dest = $array1 + (list ($array2.values))

# Now, list the result.
print
print '- - Testing Array Append - -'
cnt = len $dest

print "Number of elements: $cnt"
for i in (range $cnt)
  print "Element [$i]: $dest[$i]"

# Assign an array to a single array element (twice), as a string.
$dest[0] = ' '.join $array1
$dest[1] = ' '.join ($array2.values)

# List the result.
print
print '- - Testing modified array - -'
cnt = len $dest

print "Number of elements: $cnt"
for i in (range $cnt)
  print "Element [$i]: $dest[$i]"

# Examine the modified second element.
print
print '- - Reassign and list second element - -'

subArray = [$dest[1]]
cnt = len $subArray

print "Number of elements: $cnt"
for i in (range $cnt)
  print "Element [$i]: $subArray[$i]"

#  The element is one string, the elements separated by a space.

# Restore from the modified second element.
print
print '- - Listing restored element - -'

subArray = ($dest[1].split)
cnt = len $subArray

print "Number of elements: $cnt"
for i in (range $cnt)
  print "Element [$i]: $subArray[$i]"
print '- - Do not depend on this behavior. - -'
print '- - It breaks when an element has a space in it - -'
print '- - (keep the list itself in the element instead). - -'

exit 0
```

```output

- Confirm that the array is really subscript sparse. -
Number of elements: 4
Element [0]: zero2
Element [1]: 
Element [2]: two2
Element [3]: three2

- Undefined elements not present, subscripts not maintained. -

- - Testing Array Append - -
Number of elements: 6
Element [0]: zero1
Element [1]: one1
Element [2]: two1
Element [3]: zero2
Element [4]: two2
Element [5]: three2

- - Testing modified array - -
Number of elements: 6
Element [0]: zero1 one1 two1
Element [1]: zero2 two2 three2
Element [2]: two1
Element [3]: zero2
Element [4]: two2
Element [5]: three2

- - Reassign and list second element - -
Number of elements: 1
Element [0]: zero2 two2 three2

- - Listing restored element - -
Number of elements: 3
Element [0]: zero2
Element [1]: two2
Element [2]: three2
- - Do not depend on this behavior. - -
- - It breaks when an element has a space in it - -
- - (keep the list itself in the element instead). - -
```

Old algorithms work with arrays, in tessh as in bash.

### Example 27-11. The Bubble Sort (`bubble.sh`)

```bash no-run
#!/bin/bash
# bubble.sh: Bubble sort, of sorts.

# Recall the algorithm for a bubble sort. In this particular version...

#  With each successive pass through the array to be sorted,
#+ compare two adjacent elements, and swap them if out of order.
#  At the end of the first pass, the "heaviest" element has sunk to bottom.
#  At the end of the second pass, the next "heaviest" one has sunk next to bottom.
#  And so forth.
#  This means that each successive pass needs to traverse less of the array.
#  You will therefore notice a speeding up in the printing of the later passes.


exchange()
{
  # Swaps two members of the array.
  local temp=${Countries[$1]} #  Temporary storage
                              #+ for element getting swapped out.
  Countries[$1]=${Countries[$2]}
  Countries[$2]=$temp
  
  return
}  

declare -a Countries  #  Declare array,
                      #+ optional here since it's initialized below.

#  Is it permissable to split an array variable over multiple lines
#+ using an escape (\)?
#  Yes.

Countries=(Netherlands Ukraine Zaire Turkey Russia Yemen Syria \
Brazil Argentina Nicaragua Japan Mexico Venezuela Greece England \
Israel Peru Canada Oman Denmark Wales France Kenya \
Xanadu Qatar Liechtenstein Hungary)

# "Xanadu" is the mythical place where, according to Coleridge,
#+ Kubla Khan did a pleasure dome decree.


clear                      # Clear the screen to start with. 

echo "0: ${Countries[*]}"  # List entire array at pass 0.

number_of_elements=${#Countries[@]}
let "comparisons = $number_of_elements - 1"

count=1 # Pass number.

while [ "$comparisons" -gt 0 ]          # Beginning of outer loop
do

  index=0  # Reset index to start of array after each pass.

  while [ "$index" -lt "$comparisons" ] # Beginning of inner loop
  do
    if [ ${Countries[$index]} \> ${Countries[`expr $index + 1`]} ]
    #  If out of order...
    #  Recalling that \> is ASCII comparison operator
    #+ within single brackets.

    #  if [[ ${Countries[$index]} > ${Countries[`expr $index + 1`]} ]]
    #+ also works.
    then
      exchange $index `expr $index + 1`  # Swap.
    fi  
    let "index += 1"  # Or,   index+=1   on Bash, ver. 3.1 or newer.
  done # End of inner loop

# ----------------------------------------------------------------------
# Paulo Marcel Coelho Aragao suggests for-loops as a simpler altenative.
#
# for (( last = $number_of_elements - 1 ; last > 0 ; last-- ))
##                     Fix by C.Y. Hunt          ^   (Thanks!)
# do
#     for (( i = 0 ; i < last ; i++ ))
#     do
#         [[ "${Countries[$i]}" > "${Countries[$((i+1))]}" ]] \
#             && exchange $i $((i+1))
#     done
# done
# ----------------------------------------------------------------------
  

let "comparisons -= 1" #  Since "heaviest" element bubbles to bottom,
                       #+ we need do one less comparison each pass.

echo
echo "$count: ${Countries[@]}"  # Print resultant array at end of each pass.
echo
let "count += 1"                # Increment pass count.

done                            # End of outer loop
                                # All done.

exit 0
```

The swap changes the list in place, so `exchange` needs no `global`. (The
original starts with `clear`, left out here; and `sorted $Countries` is
the one-word way.)

```tessh
def exchange i j
  # Swaps two members of the array.
  [$Countries[$i] $Countries[$j]] = [$Countries[$j] $Countries[$i]]

Countries = [Netherlands Ukraine Zaire Turkey Russia Yemen Syria
  Brazil Argentina Nicaragua Japan Mexico Venezuela Greece England
  Israel Peru Canada Oman Denmark Wales France Kenya
  Xanadu Qatar Liechtenstein Hungary]

# "Xanadu" is the mythical place where, according to Coleridge,
#+ Kubla Khan did a pleasure dome decree.

print "0: $(' '.join $Countries)"  # List entire array at pass 0.

number_of_elements = len $Countries
comparisons = $number_of_elements - 1

count = 1 # Pass number.

while $comparisons > 0          # Beginning of outer loop
  for index in (range $comparisons)    # Inner loop
    if $Countries[$index] > $Countries[$index + 1]
      #  If out of order...
      exchange $index ($index + 1)  # Swap.

  comparisons -= 1  #  Since "heaviest" element bubbles to bottom,
                    #+ we need do one less comparison each pass.

  print
  print "$count: $(' '.join $Countries)"  # Print resultant array at end of each pass.
  print
  count += 1                      # Increment pass count.

exit 0
```

```output
0: Netherlands Ukraine Zaire Turkey Russia Yemen Syria Brazil Argentina Nicaragua Japan Mexico Venezuela Greece England Israel Peru Canada Oman Denmark Wales France Kenya Xanadu Qatar Liechtenstein Hungary

1: Netherlands Ukraine Turkey Russia Yemen Syria Brazil Argentina Nicaragua Japan Mexico Venezuela Greece England Israel Peru Canada Oman Denmark Wales France Kenya Xanadu Qatar Liechtenstein Hungary Zaire


2: Netherlands Turkey Russia Ukraine Syria Brazil Argentina Nicaragua Japan Mexico Venezuela Greece England Israel Peru Canada Oman Denmark Wales France Kenya Xanadu Qatar Liechtenstein Hungary Yemen Zaire


3: Netherlands Russia Turkey Syria Brazil Argentina Nicaragua Japan Mexico Ukraine Greece England Israel Peru Canada Oman Denmark Venezuela France Kenya Wales Qatar Liechtenstein Hungary Xanadu Yemen Zaire


4: Netherlands Russia Syria Brazil Argentina Nicaragua Japan Mexico Turkey Greece England Israel Peru Canada Oman Denmark Ukraine France Kenya Venezuela Qatar Liechtenstein Hungary Wales Xanadu Yemen Zaire


5: Netherlands Russia Brazil Argentina Nicaragua Japan Mexico Syria Greece England Israel Peru Canada Oman Denmark Turkey France Kenya Ukraine Qatar Liechtenstein Hungary Venezuela Wales Xanadu Yemen Zaire


6: Netherlands Brazil Argentina Nicaragua Japan Mexico Russia Greece England Israel Peru Canada Oman Denmark Syria France Kenya Turkey Qatar Liechtenstein Hungary Ukraine Venezuela Wales Xanadu Yemen Zaire


7: Brazil Argentina Netherlands Japan Mexico Nicaragua Greece England Israel Peru Canada Oman Denmark Russia France Kenya Syria Qatar Liechtenstein Hungary Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


8: Argentina Brazil Japan Mexico Netherlands Greece England Israel Nicaragua Canada Oman Denmark Peru France Kenya Russia Qatar Liechtenstein Hungary Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


9: Argentina Brazil Japan Mexico Greece England Israel Netherlands Canada Nicaragua Denmark Oman France Kenya Peru Qatar Liechtenstein Hungary Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


10: Argentina Brazil Japan Greece England Israel Mexico Canada Netherlands Denmark Nicaragua France Kenya Oman Peru Liechtenstein Hungary Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


11: Argentina Brazil Greece England Israel Japan Canada Mexico Denmark Netherlands France Kenya Nicaragua Oman Liechtenstein Hungary Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


12: Argentina Brazil England Greece Israel Canada Japan Denmark Mexico France Kenya Netherlands Nicaragua Liechtenstein Hungary Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


13: Argentina Brazil England Greece Canada Israel Denmark Japan France Kenya Mexico Netherlands Liechtenstein Hungary Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


14: Argentina Brazil England Canada Greece Denmark Israel France Japan Kenya Mexico Liechtenstein Hungary Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


15: Argentina Brazil Canada England Denmark Greece France Israel Japan Kenya Liechtenstein Hungary Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


16: Argentina Brazil Canada Denmark England France Greece Israel Japan Kenya Hungary Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


17: Argentina Brazil Canada Denmark England France Greece Israel Japan Hungary Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


18: Argentina Brazil Canada Denmark England France Greece Israel Hungary Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


19: Argentina Brazil Canada Denmark England France Greece Hungary Israel Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


20: Argentina Brazil Canada Denmark England France Greece Hungary Israel Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


21: Argentina Brazil Canada Denmark England France Greece Hungary Israel Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


22: Argentina Brazil Canada Denmark England France Greece Hungary Israel Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


23: Argentina Brazil Canada Denmark England France Greece Hungary Israel Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


24: Argentina Brazil Canada Denmark England France Greece Hungary Israel Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


25: Argentina Brazil Canada Denmark England France Greece Hungary Israel Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire


26: Argentina Brazil Canada Denmark England France Greece Hungary Israel Japan Kenya Liechtenstein Mexico Netherlands Nicaragua Oman Peru Qatar Russia Syria Turkey Ukraine Venezuela Wales Xanadu Yemen Zaire

```

bash arrays hold only strings, so nesting one in another means storing its
text. Michael Zick's example puts the words of an `ls -l` line in an array
and picks some out into another:

```bash no-run
#!/bin/bash
# "Nested" array.

#  Michael Zick provided this example,
#+ with corrections and clarifications by William Park.

AnArray=( $(ls --inode --ignore-backups --almost-all \
	--directory --full-time --color=none --time=status \
	--sort=time -l ${PWD} ) )  # Commands and options.

# Spaces are significant . . . and don't quote anything in the above.

SubArray=( ${AnArray[@]:11:1}  ${AnArray[@]:6:5} )
#  This array has six elements:
#+     SubArray=( [0]=${AnArray[11]} [1]=${AnArray[6]} [2]=${AnArray[7]}
#      [3]=${AnArray[8]} [4]=${AnArray[9]} [5]=${AnArray[10]} )
#
#  Arrays in Bash are (circularly) linked lists
#+ of type string (char *).
#  So, this isn't actually a nested array,
#+ but it's functionally similar.

echo "Current directory and date of last status change:"
echo "${SubArray[@]}"

exit 0
```

Its output is a date, so the tessh version doesn't run here. A tessh list
holds lists, so a real nested one is just written: `[[a b] [c d]]`.

```tessh no-run
AnArray = (ls --inode --ignore-backups --almost-all --directory --full-time \
  --color=none --time=status --sort=time -l (pwd)).split

SubArray = $AnArray[11:12] + $AnArray[6:11]

print "Current directory and date of last status change:"
print *$SubArray
```

### Example 27-12. Embedded arrays and indirect references (`embedded-arrays.sh`)

```bash
#!/bin/bash
# embedded-arrays.sh
# Embedded arrays and indirect references.

# This script by Dennis Leeuw.
# Used with permission.
# Modified by document author.


ARRAY1=(
        VAR1_1=value11
        VAR1_2=value12
        VAR1_3=value13
)

ARRAY2=(
        VARIABLE="test"
        STRING="VAR1=value1 VAR2=value2 VAR3=value3"
        ARRAY21=${ARRAY1[*]}
)       # Embed ARRAY1 within this second array.

function print () {
        OLD_IFS="$IFS"
        IFS=$'\n'       #  To print each array element
                        #+ on a separate line.
        TEST1="ARRAY2[*]"
        local ${!TEST1} # See what happens if you delete this line.
        #  Indirect reference.
	#  This makes the components of $TEST1
	#+ accessible to this function.


        #  Let's see what we've got so far.
        echo
        echo "\$TEST1 = $TEST1"       #  Just the name of the variable.
        echo; echo
        echo "{\$TEST1} = ${!TEST1}"  #  Contents of the variable.
                                      #  That's what an indirect
                                      #+ reference does.
        echo
        echo "-------------------------------------------"; echo
        echo


        # Print variable
        echo "Variable VARIABLE: $VARIABLE"
	
        # Print a string element
        IFS="$OLD_IFS"
        TEST2="STRING[*]"
        local ${!TEST2}      # Indirect reference (as above).
        echo "String element VAR2: $VAR2 from STRING"

        # Print an array element
        TEST2="ARRAY21[*]"
        local ${!TEST2}      # Indirect reference (as above).
        echo "Array element VAR1_1: $VAR1_1 from ARRAY21"
}

print
echo

exit 0

#   As the author of the script notes,
#+ "you can easily expand it to create named-hashes in bash."
#   (Difficult) exercise for the reader: implement this.
```

```output bash

$TEST1 = ARRAY2[*]


{$TEST1} = VARIABLE=test
STRING=VAR1=value1 VAR2=value2 VAR3=value3
ARRAY21=VAR1_1=value11
VAR1_2=value12
VAR1_3=value13

-------------------------------------------


Variable VARIABLE: test
String element VAR2: value2 from STRING
Array element VAR1_1: value11 from ARRAY21

```

The bash keeps `name=value` strings in arrays and makes variables of them
with `local ${!TEST1}`. In tessh those are dicts, one inside the other,
and the "named hashes" the ABS Guide leaves as an exercise. The indirect
reference is `(globals)[$TEST1]`. The function is `print_array2`, since
`print` is tessh's.

```tessh
ARRAY1 = {VAR1_1: value11 VAR1_2: value12 VAR1_3: value13}

ARRAY2 = {
  VARIABLE: test
  STRING: "VAR1=value1 VAR2=value2 VAR3=value3"
  ARRAY21: $ARRAY1
}       # Embed ARRAY1 within this second array.

def print_array2
  TEST1 = ARRAY2
  array = (globals)[$TEST1]  # Indirect reference.

  #  Let's see what we've got so far.
  print
  print "\$TEST1 = $TEST1"     #  Just the name of the variable.
  print ; print
  print "{\$TEST1} ="          #  Contents of the variable.
  for [k v] in ($array.items)
    print "$k=$v"
  print
  print "-------------------------------------------" ; print
  print

  # Print variable
  print "Variable VARIABLE: $array['VARIABLE']"

  # Print a string element: its name=value words, made a dict.
  STRING = dict [($w.split "=") for w in ($array['STRING'].split)]
  print "String element VAR2: $STRING['VAR2'] from STRING"

  # Print an array element
  print "Array element VAR1_1: $array['ARRAY21']['VAR1_1'] from ARRAY21"

print_array2
print

exit 0
```

```output

$TEST1 = ARRAY2


{$TEST1} =
VARIABLE=test
STRING=VAR1=value1 VAR2=value2 VAR3=value3
ARRAY21={'VAR1_1': 'value11', 'VAR1_2': 'value12', 'VAR1_3': 'value13'}

-------------------------------------------


Variable VARIABLE: test
String element VAR2: value2 from STRING
Array element VAR1_1: value11 from ARRAY21

```

Arrays make the Sieve of Eratosthenes possible in a script, if slowly.

### Example 27-13. The Sieve of Eratosthenes (`ex68.sh`)

```bash
#!/bin/bash
# sieve.sh (ex68.sh)

# Sieve of Eratosthenes
# Ancient algorithm for finding prime numbers.

#  This runs a couple of orders of magnitude slower
#+ than the equivalent program written in C.

LOWER_LIMIT=1       # Starting with 1.
UPPER_LIMIT=1000    # Up to 1000.
# (You may set this higher . . . if you have time on your hands.)

PRIME=1
NON_PRIME=0

let SPLIT=UPPER_LIMIT/2
# Optimization:
# Need to test numbers only halfway to upper limit. Why?


declare -a Primes
# Primes[] is an array.


initialize ()
{
# Initialize the array.

i=$LOWER_LIMIT
until [ "$i" -gt "$UPPER_LIMIT" ]
do
  Primes[i]=$PRIME
  let "i += 1"
done
#  Assume all array members guilty (prime)
#+ until proven innocent.
}

print_primes ()
{
# Print out the members of the Primes[] array tagged as prime.

i=$LOWER_LIMIT

until [ "$i" -gt "$UPPER_LIMIT" ]
do

  if [ "${Primes[i]}" -eq "$PRIME" ]
  then
    printf "%8d" $i
    # 8 spaces per number gives nice, even columns.
  fi
  
  let "i += 1"
  
done

}

sift () # Sift out the non-primes.
{

let i=$LOWER_LIMIT+1
# Let's start with 2.

until [ "$i" -gt "$UPPER_LIMIT" ]
do

if [ "${Primes[i]}" -eq "$PRIME" ]
# Don't bother sieving numbers already sieved (tagged as non-prime).
then

  t=$i

  while [ "$t" -le "$UPPER_LIMIT" ]
  do
    let "t += $i "
    Primes[t]=$NON_PRIME
    # Tag as non-prime all multiples.
  done

fi  

  let "i += 1"
done  


}


# ==============================================
# main ()
# Invoke the functions sequentially.
initialize
sift
print_primes
# This is what they call structured programming.
# ==============================================

echo

exit 0



# -------------------------------------------------------- #
# Code below line will not execute, because of 'exit.'

#  This improved version of the Sieve, by Stephane Chazelas,
#+ executes somewhat faster.

# Must invoke with command-line argument (limit of primes).

UPPER_LIMIT=$1                  # From command-line.
let SPLIT=UPPER_LIMIT/2         # Halfway to max number.

Primes=( '' $(seq $UPPER_LIMIT) )

i=1
until (( ( i += 1 ) > SPLIT ))  # Need check only halfway.
do
  if [[ -n ${Primes[i]} ]]
  then
    t=$i
    until (( ( t += i ) > UPPER_LIMIT ))
    do
      Primes[t]=
    done
  fi  
done  
echo ${Primes[*]}

exit $?
```

`Primes` is a list of `True`/`False`, indexed by the number. The code
below the original's `exit` is a faster version; it's after this example,
in tessh.

```tessh
# Sieve of Eratosthenes
# Ancient algorithm for finding prime numbers.

LOWER_LIMIT = 1       # Starting with 1.
UPPER_LIMIT = 1000    # Up to 1000.

PRIME = True
NON_PRIME = False

Primes = []

def initialize
  # Assume all array members guilty (prime)
  #+ until proven innocent.
  global Primes
  Primes = [$PRIME] * ($UPPER_LIMIT + 1)

def print_primes
  # Print out the members of the Primes list tagged as prime.
  for i in (range $LOWER_LIMIT ($UPPER_LIMIT + 1))
    if $Primes[$i] == $PRIME
      printf "%8d" $i
      # 8 spaces per number gives nice, even columns.

def sift # Sift out the non-primes.
  for i in (range ($LOWER_LIMIT + 1) ($UPPER_LIMIT + 1))   # Let's start with 2.
    if $Primes[$i] == $PRIME
      # Don't bother sieving numbers already sieved (tagged as non-prime).
      for t in (range (2 * $i) ($UPPER_LIMIT + 1) $i)
        $Primes[$t] = $NON_PRIME
        # Tag as non-prime all multiples.

# ==============================================
# main ()
# Invoke the functions sequentially.
initialize
sift
print_primes
# This is what they call structured programming.
# ==============================================

print

exit 0
```

```output
       1       2       3       5       7      11      13      17      19      23      29      31      37      41      43      47      53      59      61      67      71      73      79      83      89      97     101     103     107     109     113     127     131     137     139     149     151     157     163     167     173     179     181     191     193     197     199     211     223     227     229     233     239     241     251     257     263     269     271     277     281     283     293     307     311     313     317     331     337     347     349     353     359     367     373     379     383     389     397     401     409     419     421     431     433     439     443     449     457     461     463     467     479     487     491     499     503     509     521     523     541     547     557     563     569     571     577     587     593     599     601     607     613     617     619     631     641     643     647     653     659     661     673     677     683     691     701     709     719     727     733     739     743     751     757     761     769     773     787     797     809     811     821     823     827     829     839     853     857     859     863     877     881     883     887     907     911     919     929     937     941     947     953     967     971     977     983     991     997
```

The faster version below the original's `exit`, by Stéphane Chazelas,
needs the limit as an argument:

```bash no-run
#  This improved version of the Sieve, by Stephane Chazelas,
#+ executes somewhat faster.

# Must invoke with command-line argument (limit of primes).

UPPER_LIMIT=$1                  # From command-line.
let SPLIT=UPPER_LIMIT/2         # Halfway to max number.

Primes=( '' $(seq $UPPER_LIMIT) )

i=1
until (( ( i += 1 ) > SPLIT ))  # Need check only halfway.
do
  if [[ -n ${Primes[i]} ]]
  then
    t=$i
    until (( ( t += i ) > UPPER_LIMIT ))
    do
      Primes[t]=
    done
  fi  
done  
echo ${Primes[*]}

exit $?
```

```tessh
args = $argv[1:] or ["100"]     # (sample arguments, when none are given)
UPPER_LIMIT = int $args[0]      # From command-line.
SPLIT = $UPPER_LIMIT // 2       # Halfway to max number.

Primes = [""] + (list (range 1 ($UPPER_LIMIT + 1)))

for i in (range 2 ($SPLIT + 1))   # Need check only halfway.
  if $Primes[$i]
    for t in (range (2 * $i) ($UPPER_LIMIT + 1) $i)
      $Primes[$t] = ""
found = [$p for p in $Primes if $p]
print *$found

exit $?
```

```output
1 2 3 5 7 11 13 17 19 23 29 31 37 41 43 47 53 59 61 67 71 73 79 83 89 97
```

### Example 27-14. The Sieve of Eratosthenes, Optimized (`ex68a.sh`)

```bash no-run
#!/bin/bash
# Optimized Sieve of Eratosthenes
# Script by Jared Martin, with very minor changes by ABS Guide author.
# Used in ABS Guide with permission (thanks!).

# Based on script in Advanced Bash Scripting Guide.
# http://tldp.org/LDP/abs/html/arrays.html#PRIMES0 (ex68.sh).

# http://www.cs.hmc.edu/~oneill/papers/Sieve-JFP.pdf (reference)
# Check results against http://primes.utm.edu/lists/small/1000.txt

# Necessary but not sufficient would be, e.g.,
#     (($(sieve 7919 | wc -w) == 1000)) && echo "7919 is the 1000th prime"

UPPER_LIMIT=${1:?"Need an upper limit of primes to search."}

Primes=( '' $(seq ${UPPER_LIMIT}) )

typeset -i i t
Primes[i=1]='' # 1 is not a prime.
until (( ( i += 1 ) > (${UPPER_LIMIT}/i) ))  # Need check only ith-way.
  do                                         # Why?
    if ((${Primes[t=i*(i-1), i]}))
    # Obscure, but instructive, use of arithmetic expansion in subscript.
    then
      until (( ( t += i ) > ${UPPER_LIMIT} ))
        do Primes[t]=; done
    fi
  done

# echo ${Primes[*]}
echo   # Change to original script for pretty-printing (80-col. display).
printf "%8d" ${Primes[*]}
echo; echo

exit $?
```

Sieving starts at `i * i` and stops at the square root of the limit; the
crossing out is a slice assignment. (Run here with 1000, the limit the
original's comment checks against.)

```tessh
import sys
args = $argv[1:] or ["1000"]    # (sample arguments, when none are given)
if not $args
  print "Need an upper limit of primes to search." file=$sys.stderr
  exit 1
UPPER_LIMIT = int $args[0]

Primes = [""] + (list (range 1 ($UPPER_LIMIT + 1)))

$Primes[1] = "" # 1 is not a prime.
i = 2
while $i * $i <= $UPPER_LIMIT   # Need check only up to the square root.
  if $Primes[$i]                 # Why?
    for t in (range ($i * $i) ($UPPER_LIMIT + 1) $i)
      $Primes[$t] = ""
  i += 1

print
printf "%8d" [$p for p in $Primes if $p]
print ; print

exit $?
```

```output

       2       3       5       7      11      13      17      19      23      29      31      37      41      43      47      53      59      61      67      71      73      79      83      89      97     101     103     107     109     113     127     131     137     139     149     151     157     163     167     173     179     181     191     193     197     199     211     223     227     229     233     239     241     251     257     263     269     271     277     281     283     293     307     311     313     317     331     337     347     349     353     359     367     373     379     383     389     397     401     409     419     421     431     433     439     443     449     457     461     463     467     479     487     491     499     503     509     521     523     541     547     557     563     569     571     577     587     593     599     601     607     613     617     619     631     641     643     647     653     659     661     673     677     683     691     701     709     719     727     733     739     743     751     757     761     769     773     787     797     809     811     821     823     827     829     839     853     857     859     863     877     881     883     887     907     911     919     929     937     941     947     953     967     971     977     983     991     997

```

Arrays can stand in for data structures bash doesn't have, such as a
stack.

### Example 27-15. Emulating a push-down stack (`stack.sh`)

```bash
#!/bin/bash
# stack.sh: push-down stack simulation

#  Similar to the CPU stack, a push-down stack stores data items
#+ sequentially, but releases them in reverse order, last-in first-out.


BP=100            #  Base Pointer of stack array.
                  #  Begin at element 100.

SP=$BP            #  Stack Pointer.
                  #  Initialize it to "base" (bottom) of stack.

Data=             #  Contents of stack location.  
                  #  Must use global variable,
                  #+ because of limitation on function return range.


                  # 100     Base pointer       <-- Base Pointer
                  #  99     First data item
                  #  98     Second data item
                  # ...     More data
                  #         Last data item     <-- Stack pointer


declare -a stack


push()            # Push item on stack.
{
if [ -z "$1" ]    # Nothing to push?
then
  return
fi

let "SP -= 1"     # Bump stack pointer.
stack[$SP]=$1

return
}

pop()                    # Pop item off stack.
{
Data=                    # Empty out data item.

if [ "$SP" -eq "$BP" ]   # Stack empty?
then
  return
fi                       #  This also keeps SP from getting past 100,
                         #+ i.e., prevents a runaway stack.

Data=${stack[$SP]}
let "SP += 1"            # Bump stack pointer.
return
}

status_report()          # Find out what's happening.
{
echo "-------------------------------------"
echo "REPORT"
echo "Stack Pointer = $SP"
echo "Just popped \""$Data"\" off the stack."
echo "-------------------------------------"
echo
}


# =======================================================
# Now, for some fun.

echo

# See if you can pop anything off empty stack.
pop
status_report

echo

push garbage
pop
status_report     # Garbage in, garbage out.      

value1=23;        push $value1
value2=skidoo;    push $value2
value3=LAST;      push $value3

pop               # LAST
status_report
pop               # skidoo
status_report
pop               # 23
status_report     # Last-in, first-out!

#  Notice how the stack pointer decrements with each push,
#+ and increments with each pop.

echo

exit 0

# =======================================================


# Exercises:
# ---------

# 1)  Modify the "push()" function to permit pushing
#   + multiple element on the stack with a single function call.

# 2)  Modify the "pop()" function to permit popping
#   + multiple element from the stack with a single function call.

# 3)  Add error checking to the critical functions.
#     That is, return an error code, depending on
#   + successful or unsuccessful completion of the operation,
#   + and take appropriate action.

# 4)  Using this script as a starting point,
#   + write a stack-based 4-function calculator.
```

The stack is a dict indexed by the stack pointer, as the original's array
is, and `pop` gives back what it popped rather than setting a global (the
ABS Guide's "limitation on function return range" doesn't apply). A list
is a stack by itself: `.append` pushes and `.pop` pops.

```tessh
BP = 100            #  Base Pointer of stack array.
                    #  Begin at element 100.

SP = $BP            #  Stack Pointer.
                    #  Initialize it to "base" (bottom) of stack.

Data = ""           #  Contents of stack location.

stack = {}

def push item=""    # Push item on stack.
  global SP
  if not $item      # Nothing to push?
    return
  SP -= 1           # Bump stack pointer.
  $stack[$SP] = $item

def pop             # Pop item off stack.
  global SP
  if $SP == $BP     # Stack empty?
    return ""       #  This also keeps SP from getting past 100,
                    #+ i.e., prevents a runaway stack.
  item = $stack[$SP]
  SP += 1           # Bump stack pointer.
  $item

def status_report   # Find out what's happening.
  print "-------------------------------------"
  print "REPORT"
  print "Stack Pointer = $SP"
  print "Just popped \"$Data\" off the stack."
  print "-------------------------------------"
  print


# =======================================================
# Now, for some fun.

print

# See if you can pop anything off empty stack.
Data = (pop)
status_report

print

push garbage
Data = (pop)
status_report     # Garbage in, garbage out.

value1 = 23 ;       push $value1
value2 = skidoo ;   push $value2
value3 = LAST ;     push $value3

Data = (pop)      # LAST
status_report
Data = (pop)      # skidoo
status_report
Data = (pop)      # 23
status_report     # Last-in, first-out!

#  Notice how the stack pointer decrements with each push,
#+ and increments with each pop.

print

exit 0
```

```output

-------------------------------------
REPORT
Stack Pointer = 100
Just popped "" off the stack.
-------------------------------------


-------------------------------------
REPORT
Stack Pointer = 100
Just popped "garbage" off the stack.
-------------------------------------

-------------------------------------
REPORT
Stack Pointer = 98
Just popped "LAST" off the stack.
-------------------------------------

-------------------------------------
REPORT
Stack Pointer = 99
Just popped "skidoo" off the stack.
-------------------------------------

-------------------------------------
REPORT
Stack Pointer = 100
Just popped "23" off the stack.
-------------------------------------


```

Arithmetic on array indexes needs care, and in bash intermediate
variables.

### Example 27-16. Complex array application: Exploring a weird mathematical series (`q-function.sh`)

```bash
#!/bin/bash

# Douglas Hofstadter's notorious "Q-series":

# Q(1) = Q(2) = 1
# Q(n) = Q(n - Q(n-1)) + Q(n - Q(n-2)), for n>2

#  This is a "chaotic" integer series with strange
#+ and unpredictable behavior.
#  The first 20 terms of the series are:
#  1 1 2 3 3 4 5 5 6 6 6 8 8 8 10 9 10 11 11 12 

#  See Hofstadter's book, _Goedel, Escher, Bach: An Eternal Golden Braid_,
#+ p. 137, ff.


LIMIT=100     # Number of terms to calculate.
LINEWIDTH=20  # Number of terms printed per line.

Q[1]=1        # First two terms of series are 1.
Q[2]=1

echo
echo "Q-series [$LIMIT terms]:"
echo -n "${Q[1]} "             # Output first two terms.
echo -n "${Q[2]} "

for ((n=3; n <= $LIMIT; n++))  # C-like loop expression.
do   # Q[n] = Q[n - Q[n-1]] + Q[n - Q[n-2]]  for n>2
#    Need to break the expression into intermediate terms,
#+   since Bash doesn't handle complex array arithmetic very well.

  let "n1 = $n - 1"        # n-1
  let "n2 = $n - 2"        # n-2
  
  t0=`expr $n - ${Q[n1]}`  # n - Q[n-1]
  t1=`expr $n - ${Q[n2]}`  # n - Q[n-2]
  
  T0=${Q[t0]}              # Q[n - Q[n-1]]
  T1=${Q[t1]}              # Q[n - Q[n-2]]

Q[n]=`expr $T0 + $T1`      # Q[n - Q[n-1]] + Q[n - Q[n-2]]
echo -n "${Q[n]} "

if [ `expr $n % $LINEWIDTH` -eq 0 ]    # Format output.
then   #      ^ modulo
  echo # Break lines into neat chunks.
fi

done

echo

exit 0

#  This is an iterative implementation of the Q-series.
#  The more intuitive recursive implementation is left as an exercise.
#  Warning: calculating this series recursively takes a VERY long time
#+ via a script. C/C++ would be orders of magnitude faster.
```

In tessh the formula goes in as it's written. `Q` is a list with a dummy
element at 0, so the indexes are the terms' numbers.

```tessh
# Douglas Hofstadter's notorious "Q-series":

# Q(1) = Q(2) = 1
# Q(n) = Q(n - Q(n-1)) + Q(n - Q(n-2)), for n>2

LIMIT = 100     # Number of terms to calculate.
LINEWIDTH = 20  # Number of terms printed per line.

Q = [None 1 1]  # First two terms of series are 1.

print
print "Q-series [$LIMIT terms]:"
print $Q[1] end=" "             # Output first two terms.
print $Q[2] end=" "

for n in (range 3 ($LIMIT + 1))
  # Q[n] = Q[n - Q[n-1]] + Q[n - Q[n-2]]  for n>2
  $Q.append ($Q[$n - $Q[$n - 1]] + $Q[$n - $Q[$n - 2]])
  print $Q[$n] end=" "

  if $n % $LINEWIDTH == 0    # Format output.
    print # Break lines into neat chunks.

print

exit 0
```

```output

Q-series [100 terms]:
1 1 2 3 3 4 5 5 6 6 6 8 8 8 10 9 10 11 11 12 
12 12 12 16 14 14 16 16 16 16 20 17 17 20 21 19 20 22 21 22 
23 23 24 24 24 24 24 32 24 25 30 28 26 30 30 28 32 30 32 32 
32 32 40 33 31 38 35 33 39 40 37 38 40 39 40 39 42 40 41 43 
44 43 43 46 44 45 47 47 46 48 48 48 48 48 48 64 41 52 54 56 

```

bash arrays have one dimension; a two-dimensional one is simulated with an
index of `row * columns + column`.

### Example 27-17. Simulating a two-dimensional array, then tilting it (`twodim.sh`)

```bash
#!/bin/bash
# twodim.sh: Simulating a two-dimensional array.

# A one-dimensional array consists of a single row.
# A two-dimensional array stores rows sequentially.

Rows=5
Columns=5
# 5 X 5 Array.

declare -a alpha     # char alpha [Rows] [Columns];
                     # Unnecessary declaration. Why?

load_alpha ()
{
local rc=0
local index

for i in A B C D E F G H I J K L M N O P Q R S T U V W X Y
do     # Use different symbols if you like.
  local row=`expr $rc / $Columns`
  local column=`expr $rc % $Rows`
  let "index = $row * $Rows + $column"
  alpha[$index]=$i
# alpha[$row][$column]
  let "rc += 1"
done  

#  Simpler would be
#+   declare -a alpha=( A B C D E F G H I J K L M N O P Q R S T U V W X Y )
#+ but this somehow lacks the "flavor" of a two-dimensional array.
}

print_alpha ()
{
local row=0
local index

echo

while [ "$row" -lt "$Rows" ]   #  Print out in "row major" order:
do                             #+ columns vary,
                               #+ while row (outer loop) remains the same.
  local column=0

  echo -n "       "            #  Lines up "square" array with rotated one.
  
  while [ "$column" -lt "$Columns" ]
  do
    let "index = $row * $Rows + $column"
    echo -n "${alpha[index]} "  # alpha[$row][$column]
    let "column += 1"
  done

  let "row += 1"
  echo

done  

# The simpler equivalent is
#     echo ${alpha[*]} | xargs -n $Columns

echo
}

filter ()     # Filter out negative array indices.
{

echo -n "  "  # Provides the tilt.
              # Explain how.

if [[ "$1" -ge 0 &&  "$1" -lt "$Rows" && "$2" -ge 0 && "$2" -lt "$Columns" ]]
then
    let "index = $1 * $Rows + $2"
    # Now, print it rotated.
    echo -n " ${alpha[index]}"
    #           alpha[$row][$column]
fi    

}
  



rotate ()  #  Rotate the array 45 degrees --
{          #+ "balance" it on its lower lefthand corner.
local row
local column

for (( row = Rows; row > -Rows; row-- ))
  do       # Step through the array backwards. Why?

  for (( column = 0; column < Columns; column++ ))
  do

    if [ "$row" -ge 0 ]
    then
      let "t1 = $column - $row"
      let "t2 = $column"
    else
      let "t1 = $column"
      let "t2 = $column + $row"
    fi  

    filter $t1 $t2   # Filter out negative array indices.
                     # What happens if you don't do this?
  done

  echo; echo

done 

#  Array rotation inspired by examples (pp. 143-146) in
#+ "Advanced C Programming on the IBM PC," by Herbert Mayer
#+ (see bibliography).
#  This just goes to show that much of what can be done in C
#+ can also be done in shell scripting.

}


#--------------- Now, let the show begin. ------------#
load_alpha     # Load the array.
print_alpha    # Print it out.  
rotate         # Rotate it 45 degrees counterclockwise.
#-----------------------------------------------------#

exit 0

# This is a rather contrived, not to mention inelegant simulation.

# Exercises:
# ---------
# 1)  Rewrite the array loading and printing functions
#     in a more intuitive and less kludgy fashion.
#
# 2)  Figure out how the array rotation functions work.
#     Hint: think about the implications of backwards-indexing an array.
#
# 3)  Rewrite this script to handle a non-square array,
#     such as a 6 X 4 one.
#     Try to minimize "distortion" when the array is rotated.
```

The tessh version keeps the original's flat list and its index arithmetic,
to tilt it the same way. A list of rows, `[[A B C D E] [F G H I J] …]`, is
the natural tessh two-dimensional array, read as `$alpha[row][column]`.

```tessh
Rows = 5
Columns = 5
# 5 X 5 Array.

alpha = [None] * ($Rows * $Columns)

def load_alpha
  rc = 0
  for i in (list "ABCDEFGHIJKLMNOPQRSTUVWXY")
    # Use different symbols if you like.
    row = $rc // $Columns
    column = $rc % $Rows
    $alpha[$row * $Rows + $column] = $i
    # alpha[$row][$column]
    rc += 1

def print_alpha
  print

  for row in (range $Rows)   #  Print out in "row major" order:
                             #+ columns vary,
                             #+ while row (outer loop) remains the same.
    print "       " end=""   #  Lines up "square" array with rotated one.

    for column in (range $Columns)
      print $alpha[$row * $Rows + $column] end=" "  # alpha[$row][$column]

    print

  print

def filter_print r c   # Filter out negative array indices.
  print "  " end=""    # Provides the tilt.

  if 0 <= $r < $Rows and 0 <= $c < $Columns
    # Now, print it rotated.
    print " $alpha[$r * $Rows + $c]" end=""
    #           alpha[$row][$column]

def rotate  #  Rotate the array 45 degrees --
            #+ "balance" it on its lower lefthand corner.
  for row in (range $Rows (- $Rows) -1)
    # Step through the array backwards. Why?

    for column in (range $Columns)
      if $row >= 0
        t1 = $column - $row
        t2 = $column
      else
        t1 = $column
        t2 = $column + $row

      filter_print $t1 $t2   # Filter out negative array indices.
                             # What happens if you don't do this?

    print ; print


#--------------- Now, let the show begin. ------------#
load_alpha     # Load the array.
print_alpha    # Print it out.
rotate         # Rotate it 45 degrees counterclockwise.
#-----------------------------------------------------#

exit 0
```

```output

       A B C D E 
       F G H I J 
       K L M N O 
       P Q R S T 
       U V W X Y 

          

           E

         D   J

       C   I   O

     B   H   N   T

   A   G   M   S   Y

     F   L   R   X

       K   Q   W

         P   V

           U

```
