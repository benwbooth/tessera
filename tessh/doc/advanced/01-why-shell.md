# 1. Shell Programming!
<!-- help: abs-why-shell abs-1 why-shell -->

The ABS Guide opens by arguing for shell scripts: a boot runs them, they're
built from small pieces, and they're the quickest way to prototype a tool by
chaining programs together. It then lists when *not* to use one: heavy math
or floating point, speed-critical work, real data structures (linked lists,
trees, multi-dimensional arrays), sockets and other I/O beyond files,
libraries and legacy code, GUIs, and large structured programs. For those
it suggests Perl, Python, Ruby or C.

tessh is the shell half and the Python half in one language, so most of
that list stops applying. Commands, pipes, globs and redirections are
bash's; floats, big integers, lists, dicts, classes, sockets and every
Python library are there on the next line. A script can start as a list of
commands and grow types, functions and modules without being rewritten in
something else.

```bash
# bash: no floating point (awk or bc does it), and no array of arrays
awk 'BEGIN { printf "%.4f\n", 22/7 }'
```

```output bash
3.1429
```

```tessh
print (round (22 / 7) 4)
grid = [[1 2] [3 4]]
print $grid[1][0] (sum (map $sum $grid))
```

```output
3.1429
3 10
```

What the ABS Guide says about Bash — that it's the de facto standard
shell, that most of what it teaches works in ksh and other Bourne shells,
and that C shell scripting is best avoided — still holds. tessh doesn't try
to run bash scripts: it reads like bash where bash is good (running and
chaining programs) and like Python everywhere else.
