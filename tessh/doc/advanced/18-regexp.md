# 18. Regular Expressions
<!-- help: abs-regexp abs-18 regular-expressions regex re grep sed awk -->

> . . . the intellectual activity associated with software development is
> largely one of gaining insight. (Stowe Boyd)

Regular expressions (REs) are the patterns of `grep`, `sed`, `awk`,
`expr` and Perl, and, since version 3, of bash's own `=~`. A tessh script
uses them in two ways: as arguments to those programs, exactly as in bash,
and in the script itself, where `re'…'` is a regular expression and `=~`
matches with it. tessh's REs are Python's, which are Perl's dialect: the
extended syntax without backslashes (`+`, `?`, `{3}`, `(…)`, `|`), with
`\d`, `\w`, `\s` and `\b`, and Perl's flags after the closing quote.

## 18.1 A Brief Introduction to Regular Expressions
<!-- help: abs-regex-intro metacharacters anchors character-set brackets dot star caret dollar backslash word-boundary -->

A regular expression is made of characters that mean themselves, *anchors*
(`^` and `$`) that tie the match to a place in the line, and *modifiers*
(`*`, brackets, the backslash) that widen or narrow what matches. It
matches a character or a string of them, a whole string or part of one.
The basic ones, as `grep` and `sed` read them, and in `re'…'`:

| RE | matches | in `re'…'` |
|---|---|---|
| `*` | any number (zero too) of what's before it: `1133*` is 113, 1133, 11333… | the same |
| `.` | any one character but a newline: `13.` is 13 and one more character | the same; with the `s` flag, a newline too |
| `^` | the start of the line (and in `[^…]`, "not") | the start of the text; with `m`, of every line |
| `$` | the end of the line: `XXX$`, and `^$` is a blank line | the end of the text; with `m`, of every line |
| `[xyz]`, `[c-n]`, `[B-Pk-y]`, `[^b-d]` | one of these characters, or a range; `^` first means "none of these" | the same |
| `\` | the next character as itself: `\$`, `\\` | the same |
| `\<the\>` | `the` as a whole word | `\bthe\b` |

`sed` and `awk` work a line at a time, so a `.` rarely meets a newline;
when the text has one in it, it matches it:

```bash
#!/bin/bash

sed -e 'N;s/.*/[&]/' << EOF   # Here Document
line1
line2
EOF
# OUTPUT:
# [line1
# line2]

echo

awk '{ $0=$1 "\n" $2; if (/line.1/) {print}}' << EOF
line 1
line 2
EOF
# OUTPUT:
# line
# 1

# Thanks, S.C.

exit 0
```

```output bash
[line1
line2]

line
1
```

The same programs and heredocs work in tessh. Python's `.` doesn't match a
newline unless the RE has the `s` flag:

```tessh
sed -e 'N;s/.*/[&]/' <<EOF
line1
line2
EOF

print

awk '{ $0=$1 "\n" $2; if (/line.1/) {print}}' <<EOF
line 1
line 2
EOF

text = "line1\nline2"
print (re'.*'.sub '[\g<0>]' $text count=1)    # Stops at the newline.
print (re'.*'s.sub '[\g<0>]' $text count=1)   # With s, . takes it in too.
```

```output
[line1
line2]

line
1
[line1]
line2
[line1
line2]
```

`[Yy][Ee][Ss]` matches yes, Yes, YES, yEs; `re'yes'i` does that with a
flag. `[0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9][0-9][0-9]` matches a US
Social Security number; with `\d`, counts and the `x` flag (which lets the
pattern have spaces and comments), it reads better:

```tessh
ssn = re'''
  ^ \d{3} - \d{2} - \d{4} $   # area, group, serial
'''x
for s in ["123-45-6789" "123-456-789" "YES" "yEs"]
  print $s (bool ($s =~ $ssn)) (bool ($s =~ re'^yes$'i))
```

```output
123-45-6789 True False
123-456-789 False False
YES False True
yEs False True
```

Word boundaries: `the` matches inside *there* and *other*, the whole word
doesn't.

```bash
cat >textfile <<'EOF'
This is line 1, of which there is only one instance.
This is the only instance of line 2.
This is line 3, another line.
This is line 4.
EOF

grep 'the' textfile
echo
grep '\<the\>' textfile
```

`grep` is the same in tessh; in the script, `=~` with `\b`:

```tessh
cat >textfile <<'EOF'
This is line 1, of which there is only one instance.
This is the only instance of line 2.
This is line 3, another line.
This is line 4.
EOF

grep 'the' textfile
print
for line in (cat textfile).lines
  if $line =~ re'\bthe\b'
    print $line
```

```output
This is line 1, of which there is only one instance.
This is the only instance of line 2.
This is line 3, another line.

This is the only instance of line 2.
```

The only way to be sure an RE does what you mean is to try it. The ABS
Guide's test file for `1133*`:

```bash
cat >tstfile <<'EOF'
Run   grep "1133*"  on this file.           # Match.
This line contains the number 113.          # Match.
This line contains the number 13.           # No match.
This line contains the number 133.          # No match.
This line contains the number 1133.         # Match.
This line contains the number 113312.       # Match.
This line contains the number 1112.         # No match.
This line contains the number 113312312.    # Match.
This line contains no numbers at all.       # No match.
EOF

grep "1133*" tstfile
```

In tessh, a list comprehension with `=~` keeps the lines that match, as
`grep` does:

```tessh
cat >tstfile <<'EOF'
Run   grep "1133*"  on this file.           # Match.
This line contains the number 113.          # Match.
This line contains the number 13.           # No match.
This line contains the number 133.          # No match.
This line contains the number 1133.         # Match.
This line contains the number 113312.       # Match.
This line contains the number 1112.         # No match.
This line contains the number 113312312.    # Match.
This line contains no numbers at all.       # No match.
EOF

matching = [$l for l in (cat tstfile).lines if $l =~ re'1133*']
print ("\n".join $matching)
```

```output
Run   grep "1133*"  on this file.           # Match.
This line contains the number 113.          # Match.
This line contains the number 1133.         # Match.
This line contains the number 113312.       # Match.
This line contains the number 113312312.    # Match.
```

### Extended REs

`egrep` (`grep -E`), `awk` and Perl add more:

| ERE | matches | in `re'…'` |
|---|---|---|
| `?` | zero or one of what's before | the same |
| `+` | one or more of what's before | the same |
| `\{5\}` (basic), `{5}` (extended) | exactly five of what's before: `[0-9]\{5\}` | `\d{5}`, `[0-9]{5}` |
| `( )` | a group | the same; `$m.group 1` is what it matched |
| `\|` | either side: `re(a\|e)d` | the same |

GNU `sed` and `grep` take `+` escaped, in their basic syntax:

```bash
# GNU versions of sed and awk can use "+",
# but it needs to be escaped.

echo a111b | sed -ne '/a1\+b/p'
echo a111b | grep 'a1\+b'
echo a111b | gawk '/a1+b/'
# All of above are equivalent.

echo 2222 | gawk --re-interval '/2{3}/'
```

```output bash
a111b
a111b
a111b
2222
```

```tessh
echo a111b | sed -ne '/a1\+b/p'
echo a111b | grep 'a1\+b'
echo a111b | gawk '/a1+b/'
# All of above are equivalent, and so is:
s = a111b
if $s =~ re'a1+b'
  print $s

echo 2222 | gawk --re-interval '/2{3}/'
print (if "2222" =~ re'2{3}' then "2222" else "")
```

```output
a111b
a111b
a111b
a111b
2222
2222
```

With a group and `|`:

```bash
cat >misc.txt <<'EOF'
People who read seem to be better informed than those who do not.
The clarinet produces sound by the vibration of its reed.
Reeds grow by the river.
EOF
egrep 're(a|e)d' misc.txt
```

```output bash
People who read seem to be better informed than those who do not.
The clarinet produces sound by the vibration of its reed.
```

`=~` with a regex gives the match: its `.group` is what matched, and
`.start` where.

```tessh
cat >misc.txt <<'EOF'
People who read seem to be better informed than those who do not.
The clarinet produces sound by the vibration of its reed.
Reeds grow by the river.
EOF
for line in (cat misc.txt).lines
  if m := $line =~ re're(a|e)d'
    print "${$m.group 0!r} at $($m.start): $line"
```

```output
'read' at 11: People who read seem to be better informed than those who do not.
'reed' at 52: The clarinet produces sound by the vibration of its reed.
```

(`Reeds` doesn't match: `R` isn't `r`. `re're(a|e)d'i` would take it.)

Groups do what `sed`'s `\(…\)` and `\1` do, in `.sub`, and `.findall` is
`grep -o`:

```bash
echo "John Smith" | sed -e 's/\([a-zA-Z]*\) \([a-zA-Z]*\)/\2, \1/'
echo "a1 b22 c333" | grep -oE '[0-9]+'
```

```tessh
print (re'(\w+) (\w+)'.sub r'\2, \1' "John Smith")
for n in (re'\d+'.findall "a1 b22 c333")
  print $n
```

```output
Smith, John
1
22
333
```

### POSIX character classes

`[:alnum:]`, `[:alpha:]`, `[:blank:]`, `[:cntrl:]`, `[:digit:]`,
`[:graph:]`, `[:lower:]`, `[:print:]`, `[:space:]`, `[:upper:]` and
`[:xdigit:]` name sets of characters, used inside brackets:
`[[:digit:]]`. Python's REs don't have them; they have `\d` (digits),
`\w` (letters, digits and `_`), `\s` (whitespace) and their opposites `\D`,
`\W`, `\S`, and ranges for the rest:

| POSIX | in `re'…'` |
|---|---|
| `[:alnum:]` | `[^\W_]` (or `[A-Za-z0-9]`) |
| `[:alpha:]` | `[^\W\d_]` (or `[A-Za-z]`) |
| `[:blank:]` | `[ \t]` |
| `[:cntrl:]` | `[\x00-\x1f\x7f]` |
| `[:digit:]` | `\d` (or `[0-9]`) |
| `[:graph:]` | `[!-~]` |
| `[:lower:]` | `[a-z]` |
| `[:print:]` | `[ -~]` |
| `[:space:]` | `\s` |
| `[:upper:]` | `[A-Z]` |
| `[:xdigit:]` | `[0-9A-Fa-f]` |

(`\d`, `\w` and the `[^\W…]` forms take in other scripts' letters and
digits; the `a` flag keeps them to ASCII.)

```bash
echo abc=723 > test.file
grep [[:digit:]] test.file
arow=5 acol=b
if [[ $arow =~ [[:digit:]] ]]   #  Numerical input?
then       #  POSIX char class
  if [[ $acol =~ [[:alpha:]] ]] # Number followed by a letter? Illegal!
  then
    echo "$arow$acol: illegal move"
  fi
fi
```

`grep` takes the POSIX classes in tessh too (quoted, so the brackets aren't
a glob); in the script, `\d` and `[^\W\d_]`:

```tessh
echo abc=723 > test.file
grep '[[:digit:]]' test.file
arow = 5 ; acol = b
if "$arow" =~ re'\d'             #  Numerical input?
  if $acol =~ re'[^\W\d_]'       # Number followed by a letter? Illegal!
    print "$arow$acol: illegal move"
```

```output
abc=723
5b: illegal move
```

**Gap:** `re'…'` doesn't know POSIX classes: `re'[[:digit:]]'` is read as
the set `[:digt` followed by `]`, with only a `FutureWarning`, and matches
the wrong things. The `regex` module (`need regex`) understands them.

`sed`, `awk` and Perl, as filters, take REs to sift and change text; the
ABS Guide recommends Friedl's *Mastering Regular Expressions* and
Dougherty and Robbins' *Sed & Awk*. Python's `re` module documentation
(`help re`) lists everything `re'…'` can do.

## 18.2 Globbing
<!-- help: abs-globbing globbing wildcards filename-expansion dotfiles nullglob -->

The shell itself doesn't use REs for file names: it *globs*, expanding
wild cards. `*` is any run of characters, `?` one character, `[…]` one of
a set (`[!…]` or `[^…]` one not in it). A `*` doesn't match a name that
starts with a dot; a pattern that starts with a literal `.` does. tessh
globs command words the same way, sorted, and a pattern that matches
nothing stays as written. Written as a value, a pattern is a Glob, which
can be looped over, counted, or tested against a name with `=~`.

```bash
touch a.1 b.1 c.1 t2.sh test1.txt
echo *
echo t*
echo t?.sh
ls [ab]*
ls [a-c]*
```

```tessh
touch a.1 b.1 c.1 t2.sh test1.txt
echo *
echo t*
echo t?.sh
ls [ab]*
ls [a-c]*
```

```output
a.1 b.1 c.1 t2.sh test1.txt
t2.sh test1.txt
t2.sh
a.1
b.1
a.1
b.1
c.1
```

(`ls` prints one name a line here because its output isn't a terminal.)

A negated set and the dot files:

```bash
touch a.1 b.1 c.1 t2.sh test1.txt .bashrc
echo [^ab]*
echo .ba?hrc .bashr* .[b]ashrc
echo ?bashrc
```

```output bash
c.1 t2.sh test1.txt
.bashrc .bashrc .bashrc
?bashrc
```

```tessh
touch a.1 b.1 c.1 t2.sh test1.txt .bashrc
names = [a.1 b.1 c.1 t2.sh test1.txt]
print [$n for n in $names if $n !~ [ab]*]   # Not starting with a or b.
print ((Path .bashrc) =~ .ba?hrc)           # A glob tested against a name.
echo ?bashrc
```

```output
['c.1' 't2.sh' 'test1.txt']
True
?bashrc
```

In a command the same globs work as in bash: `echo [!ab]*` and `echo
[^ab]*` list the names not starting with a or b, `.ba?hrc` and `.bashr*`
find the dot file, and `.*` lists the dot files without `.` and `..`.

```tessh
touch a.1 b.1 c.1 .bashrc
echo [!ab]* [^ab]*
echo .ba?hrc .bashr* .*
```

```output
c.1 c.1
.bashrc .bashrc .bashrc
```

bash's `{b*,c*,*est*}` (brace expansion, then globbing) has no tessh form;
`glob` takes several patterns:

```bash
touch a.1 b.1 c.1 t2.sh test1.txt
ls {b*,c*,*est*}
```

```tessh
touch a.1 b.1 c.1 t2.sh test1.txt
ls (glob b* c* *est*)
```

```output
b.1
c.1
test1.txt
```

POSIX classes work in bash's globs to an extent, `ls ?[[:digit:]][[:digit:]]?`
listing `a33b`. tessh's globs don't take them; `[0-9]` does the job:

```bash
touch a33b abcd
ls ?[[:digit:]][[:digit:]]?
```

```tessh
touch a33b abcd
ls ?[0-9][0-9]?
```

```output
a33b
```

**Gap:** no POSIX classes (`[[:digit:]]`) in globs, and no brace
expansion (see 3.6).

bash's `set -f` turns globbing off and `shopt` changes it (`nocaseglob`,
`nullglob`, `dotglob`). In tessh, quote a word to keep it as it is
(`'*'`), and use the Glob as a value when nothing matching should mean an
empty list (bash's `nullglob`): `(list *.none)` is `[]`.

File names with spaces are the classic trap of shell globbing, because
bash splits words again after expanding variables. David Wheeler's advice
for bash:

```bash no-run
IFS="$(printf '\n\t')"   # Remove space.

#  Correct glob use:
#  Always use for-loop, prefix glob, check if exists file.
for file in ./* ; do         # Use ./* ... NEVER bare *
  if [ -e "$file" ] ; then   # Check whether file exists.
     COMMAND ... "$file" ...
  fi
done

# This example taken from David Wheeler's site, with permission.
```

tessh doesn't split a variable's value into words, so a name with a space
in it stays one argument, and a glob gives `Path`s (`./` in front, so a
name starting with `-` can't pass for an option). The loop over no matches
doesn't run, so there's nothing to check:

```tessh
touch -- "my file.txt" -n plain.txt
for file in ./*
  wc -c $file
```

```output
0 ./-n
0 ./my file.txt
0 ./plain.txt
```
