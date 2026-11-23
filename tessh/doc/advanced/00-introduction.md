# Advanced tessh Scripting Guide
<!-- help: advanced abs advanced-guide abs-guide bash-to-tessh porting -->

This is Mendel Cooper's *Advanced Bash-Scripting Guide* (the ABS Guide,
version 10, 10 March 2014), ported to tessh. Every chapter follows the
original, and every example script is shown twice: the original bash, then
a tessh version that does the same thing, with what it printed. Read them
side by side to see how a bash habit is written in tessh, and where tessh
does it another way.

`help advanced` shows this page in tessh, and `help abs-loops` (or any
chapter's `abs-` name) one chapter. In Tessera it's the second guide on the
tessh Guide page.

## Licence
<!-- help: abs-licence abs-license public-domain -->

The ABS Guide is in the public domain. Its title page says it "is herewith
granted to the Public Domain. No copyright!", and its copyright appendix
(Appendix S, <https://tldp.org/LDP/abs/html/copyright.html>) that "There
can be NO restrictions" on copying and distributing it or any derivative.
The original text and example scripts used here come from that release (the
`abs-guide_10` source archive). This port keeps the author's name and
credits, as a courtesy: the scripts' own credit lines are kept in the bash
versions, and examples contributed by others say so. The author asked, not
as a condition, that the book not be used in lawsuits against the open
source community.

## How to read it
<!-- help: abs-conventions abs-how-to-read -->

Each example is three blocks:

- **bash**: the original script, as the ABS Guide has it (its file name is
  in the heading).
- **tessh**: the same script in tessh. It runs: the guide's test runs every
  one as a script, `ex.tsh`, in a scratch folder and checks what it prints.
  (A few run tessh itself again, through `/proc/self/exe`; they're marked
  `tessh script` and checked with the tessh program instead.)
- **output**: what the tessh version printed. Where bash printed something
  else, an `output bash` block after the bash version shows bash's.

Scripts that need root, delete files, read a terminal or reach the network
are shown working on a scratch folder or on made-up input instead, and say
so. What tessh can't do yet is marked **Gap** where it comes up, and the
last chapter lists them all.

tessh is Python underneath, so a lot of what the ABS Guide teaches as
tricks (arrays, string slicing, arithmetic, `printf -v`, `eval`) is plain
Python in tessh, and its programs (`grep`, `sed`, `awk`, `find`) run as
they do in bash. The tessh guide (`help`) explains the language; this guide
assumes you've skimmed it.

The prose here is a short summary of the ABS Guide's, with tessh's side of
it; the original is at <https://tldp.org/LDP/abs/html/>.
