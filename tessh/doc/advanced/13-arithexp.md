# 13. Arithmetic Expansion
<!-- help: abs-arithexp abs-13 arithmetic-expansion $((…)) let expr -->

Bash does integer arithmetic in `$((…))`, `((…))` and `let`, and before
those with the `expr` program. tessh's expressions are Python's, so
arithmetic needs no special form: `$z + 3` is a sum wherever a value goes,
`z += 3` changes `z`, and the numbers can be as big as you like, or
floats (`7 / 2` is 3.5; `7 // 2` is bash's 3). In a string, `"$($z * 2)"`
is bash's `"$((z * 2))"`.

```bash
z=5
z=`expr $z + 3`          # The 'expr' command performs the expansion.
echo "z = $z"            # z = 8

z=$(($z+3))
z=$((z+3))                                  #  Also correct.
                                            #  Within double parentheses,
                                            #+ parameter dereferencing
                                            #+ is optional.
echo "z = $z"            # z = 14

# $((EXPRESSION)) is arithmetic expansion.  #  Not to be confused with
                                            #+ command substitution.

# You may also use operations within double parentheses without assignment.

  n=0
  echo "n = $n"                             # n = 0

  (( n += 1 ))                              # Increment.
# (( $n += 1 )) is incorrect!
  echo "n = $n"                             # n = 1

let z=z+3
let "z += 3"  #  Quotes permit the use of spaces in variable assignment.
              #  The 'let' operator actually performs arithmetic evaluation,
              #+ rather than expansion.
echo "z = $z"            # z = 20
```

```tessh
z = 5
z = int (expr $z + 3)    # expr still works; its output is text, hence int.
print "z = $z"           # z = 8

z = $z + 3
z += 3
print "z = $z"           # z = 14

n = 0
print "n = $n"           # n = 0
n += 1                   # Increment.
print "n = $n"           # n = 1

z = $z + 3
z += 3
print "z = $z"           # z = 20

```

```output
z = 8
z = 14
n = 0
n = 1
z = 20
```

The one thing to watch: values read from commands, files or arguments are
text. `"3" + 3` is an error, not 6, so convert with `int` or `float`
first.
