# Appendix D. Parsing and Managing Pathnames
<!-- help: abs-pathmanagement abs-d pathnames PATH filenames spaces -->

Emmanuel Rouat contributed functions that clean up a `PATH`-like
variable: drop empty entries, extra slashes and duplicates, and remove,
append or prepend a folder. They pass the variable's *name* around and
`eval` the result back into it, and lean on `awk`, `grep` and `paste`.

```bash
#!/usr/bin/env bash
#-----------------------------------------------------------
# Management of PATH, LD_LIBRARY_PATH, MANPATH variables...
# By Emmanuel Rouat <no-email>
# (Inspired by the bash documentation 'pathfuncs' and on
# discussions found on stackoverflow:
# http://stackoverflow.com/questions/370047/
# http://stackoverflow.com/questions/273909/#346860 )
# Last modified: Sat Sep 22 12:01:55 CEST 2012
#
# The following functions handle spaces correctly.
# These functions belong in .bash_profile rather than in
# .bashrc, I guess.
#
# The modular aspect of these functions should make it easy
# to expand them to handle path substitutions instead
# of path removal etc....
#
# See http://www.catonmat.net/blog/awk-one-liners-explained-part-two/
# (item 43) for an explanation of the 'duplicate-entries' removal
# (it's a nice trick!)
#-----------------------------------------------------------

# Show $@ (usually PATH) as list.
function p_show() { local p="$@" && for p; do [[ ${!p} ]] &&
echo -e ${!p//:/\\n}; done }

# Filter out empty lines, multiple/trailing slashes, and duplicate entries.
function p_filter()
{ awk '/^[ \t]*$/ {next} {sub(/\/+$/, "");gsub(/\/+/, "/")}!x[$0]++' ;}

# Rebuild list of items into ':' separated word (PATH-like).
function p_build() { paste -sd: ;}

# Clean $1 (typically PATH) and rebuild it
function p_clean()
{ local p=${1} && eval ${p}='$(p_show ${p} | p_filter | p_build)' ;}

# Remove $1 from $2 (found on stackoverflow, with modifications).
function p_rm()
{ local d=$(echo $1 | p_filter) p=${2} &&
  eval ${p}='$(p_show ${p} | p_filter | grep -xv "${d}" | p_build)' ;}

#  Same as previous, but filters on a pattern (dangerous...
#+ don't use 'bin' or '/' as pattern!).
function p_rmpat()
{ local d=$(echo $1 | p_filter) p=${2} && eval ${p}='$(p_show ${p} |
  p_filter | grep -v "${d}" | p_build)' ;}

# Delete $1 from $2 and append it cleanly.
function p_append()
{ local d=$(echo $1 | p_filter) p=${2} && p_rm "${d}" ${p} &&
  eval ${p}='$(p_show ${p} d | p_build)' ;}

# Delete $1 from $2 and prepend it cleanly.
function p_prepend()
{ local d=$(echo $1 | p_filter) p=${2} && p_rm "${d}" ${p} &&
  eval ${p}='$(p_show d ${p} | p_build)' ;}

# Some tests:
echo
MYPATH="/bin:/usr/bin/:/bin://bin/"
p_append "/project//my project/bin" MYPATH
echo "Append '/project//my project/bin' to '/bin:/usr/bin/:/bin://bin/'"
echo "(result should be: /bin:/usr/bin:/project/my project/bin)"
echo $MYPATH

echo
MYOTHERPATH="/bin:/usr/bin/:/bin:/project//my project/bin"
p_prepend "/project//my project/bin" MYOTHERPATH
echo "Prepend '/project//my project/bin' \
to '/bin:/usr/bin/:/bin:/project//my project/bin/'"
echo "(result should be: /project/my project/bin:/bin:/usr/bin)"
echo $MYOTHERPATH

echo
p_prepend "/project//my project/bin" FOOPATH  # FOOPATH doesn't exist.
echo "Prepend '/project//my project/bin' to an unset variable"
echo "(result should be: /project/my project/bin)"
echo $FOOPATH

echo
BARPATH="/a:/b/://b c://a:/my local pub"
p_clean BARPATH
echo "Clean BARPATH='/a:/b/://b c://a:/my local pub'"
echo "(result should be: /a:/b:/b c:/my local pub)"
echo $BARPATH
```

In tessh the functions take a path string and give the new one back, so
there's no `eval` and no passing of names: `MYPATH = p_append dir $MYPATH`,
or `$env.PATH = p_prepend ~/bin $env.PATH` for the real `PATH`. Between
splitting and joining, a path is a list, and `p_filter` cleans a list.

```tessh
import re

# Filter out empty items, multiple/trailing slashes, and duplicates.
def p_filter items
  out = []
  for item in $items
    if not ($item.strip)
      continue
    item = re.sub r"/+" "/" ($item.rstrip "/")
    if $item not in $out
      $out.append $item
  $out

# A PATH-like string as a clean list, and a list back as a string.
def p_show path = p_filter (($path or "").split ":")
def p_build items = ":".join $items

# Clean a path.
def p_clean path = p_build (p_show $path)

# Remove dir from path.
def p_rm dir path
  d = (p_filter [$dir])[0]
  p_build [$i for i in (p_show $path) if $i != $d]

# Same as previous, but filters on a pattern (dangerous...
# don't use 'bin' or '/' as pattern!).
def p_rmpat pat path
  d = (p_filter [$pat])[0]
  p_build [$i for i in (p_show $path) if $d not in $i]

# Delete dir from path and append it cleanly.
def p_append dir path = p_build ((p_show (p_rm $dir $path)) + (p_filter [$dir]))

# Delete dir from path and prepend it cleanly.
def p_prepend dir path = p_build ((p_filter [$dir]) + (p_show (p_rm $dir $path)))

# Some tests:
print
MYPATH = "/bin:/usr/bin/:/bin://bin/"
MYPATH = p_append "/project//my project/bin" $MYPATH
print "Append '/project//my project/bin' to '/bin:/usr/bin/:/bin://bin/'"
print "(result should be: /bin:/usr/bin:/project/my project/bin)"
print $MYPATH

print
MYOTHERPATH = "/bin:/usr/bin/:/bin:/project//my project/bin"
MYOTHERPATH = p_prepend "/project//my project/bin" $MYOTHERPATH
print "Prepend '/project//my project/bin' \
to '/bin:/usr/bin/:/bin:/project//my project/bin/'"
print "(result should be: /project/my project/bin:/bin:/usr/bin)"
print $MYOTHERPATH

print
FOOPATH = p_prepend "/project//my project/bin" ($env.get FOOPATH)  # Not set.
print "Prepend '/project//my project/bin' to an unset variable"
print "(result should be: /project/my project/bin)"
print $FOOPATH

print
BARPATH = p_clean "/a:/b/://b c://a:/my local pub"
print "Clean BARPATH='/a:/b/://b c://a:/my local pub'"
print "(result should be: /a:/b:/b c:/my local pub)"
print $BARPATH
```

```output

Append '/project//my project/bin' to '/bin:/usr/bin/:/bin://bin/'
(result should be: /bin:/usr/bin:/project/my project/bin)
/bin:/usr/bin:/project/my project/bin

Prepend '/project//my project/bin' to '/bin:/usr/bin/:/bin:/project//my project/bin/'
(result should be: /project/my project/bin:/bin:/usr/bin)
/project/my project/bin:/bin:/usr/bin

Prepend '/project//my project/bin' to an unset variable
(result should be: /project/my project/bin)
/project/my project/bin

Clean BARPATH='/a:/b/://b c://a:/my local pub'
(result should be: /a:/b:/b c:/my local pub)
/a:/b:/b c:/my local pub
```

## Doing it correctly: a quick summary
<!-- help: abs-filenames-correctly filenames-with-spaces wheeler globs find print0 -->

David Wheeler's summary of handling file names safely in bash: quote
every `"$variable"`, set `IFS` to newline and tab, prefix globs with `./`
so no name starts with `-`, check that a glob matched, and use `find
-exec`, `find -print0 | xargs -0` or `read -d ""` for names that may hold
anything, newlines included.

```bash no-run
 IFS="$(printf '\n\t')"
 # Remove SPACE, so filenames with spaces work well.

 #  Correct glob use:
 #+ always use "for" loop, prefix glob, check for existence:
 for file in ./* ; do          # Use "./*" ... NEVER bare "*" ...
   if [ -e "$file" ] ; then    # Make sure it isn't an empty match.
     COMMAND ... "$file" ...
   fi
 done

 # Correct glob use, but requires nonstandard bash extension.
 shopt -s nullglob  #  Bash extension,
                    #+ so that empty glob matches will work.
 for file in ./* ; do        # Use "./*", NEVER bare "*"
   COMMAND ... "$file" ...
 done

 #  These handle all filenames correctly;
 #+ can be unwieldy if COMMAND is large:
 find ... -exec COMMAND... {} \;
 find ... -exec COMMAND... {} \+ # If multiple files are okay for COMMAND.

 # Requires nonstandard but common extensions in find and xargs:
 find . -print0 | xargs -0 COMMAND

 # Requires nonstandard extensions to find and to shell (bash works).
 # variables might not stay set once the loop ends:
 find . -print0 | while IFS="" read -r -d "" file ; do ...
   COMMAND "$file" # Use quoted "$file", not $file, everywhere.
 done
```

Most of this is about bash splitting and globbing variables, which tessh
never does: a file name in a variable is one argument, whatever is in it.
What's left:

- Iterating a Glob gives the names that exist, and nothing when nothing
  matches, so the `-e` check and `nullglob` aren't needed.
- A name that starts with `-` can still look like an option to a program.
  `./*` (names with `./` in front) or `--` before the names guard against
  it, as in bash.
- `find … -print0` split at `"\0"`, or Python's `os.walk`, gives every
  name exactly, newlines and all.

```tessh
import os
# (Made-up files with awkward names: a space, a dash, a newline.)
touch "a file" ./-dash ; touch "new
line"

for file in ./*                      # Paths, with ./ in front.
  print [(str $file)]

for file in ((find . -type f -print0).split "\0")
  if $file                           # (The last item is empty.)
    print [$file]

for [root dirs files] in (os.walk ".")
  for f in (sorted $files)
    print [(os.path.join $root $f)]

for file in ./*.nothing              # No match: no iterations.
  print "never"
```

```text
(one run)
['./-dash']
['./a file']
["./new\nline"]
['./a file']
['./-dash']
["./new\nline"]
['./-dash']
['./a file']
["./new\nline"]
```

(`find`'s order is the folder's, not sorted, so it can change.)
