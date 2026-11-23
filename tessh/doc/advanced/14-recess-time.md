# 14. Recess Time
<!-- help: abs-recess-time abs-14 recess intermission chain-letter -->

The ABS Guide pauses here for a joke: a chain letter. Send a 100-line Bash
script to the first person on the list and pass the letter on to ten
friends, or else. Those who broke the chain found their job changed to
"COBOL programmer", or their terminal on fire and themselves writing
documentation for MS Windows; one who kept it got enough hardware for a
100-node Beowulf cluster playing Tuxracer.

The tessh edition asks less: send a 100-line tessh script instead. It
will probably be 60 lines.

```tessh
friends = [Wilfred Howard Amelia]
for [n name] in (enumerate $friends 1)
  print "$n. $name: don't break the chain!"
```

```output
1. Wilfred: don't break the chain!
2. Howard: don't break the chain!
3. Amelia: don't break the chain!
```
