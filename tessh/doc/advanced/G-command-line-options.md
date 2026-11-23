# Appendix G. Command-Line Options
<!-- help: abs-command-line-options abs-g options flags --help argparse -->

Programs take options that change what they do: `command -o`. tessh
passes them to a program as words, as bash does, so everything here
applies to programs run from tessh unchanged.

## G.1 Standard Command-Line Options
<!-- help: abs-standard-options -h --help -v --version -q -r -f -->

Over time a loose standard has grown up for what the option letters mean.
Traditionally an option is a dash and a letter; GNU added long options, a
double dash and a word.

| option | meaning |
|---|---|
| `-h`, `--help` | print a usage message and exit |
| `-v`, `--version` | print the version and exit |
| `-a`, `--all` | all the information, or all the arguments |
| `-l`, `--list` | list files or arguments, without doing anything else |
| `-o` | the output file name |
| `-q`, `--quiet` | print nothing on standard output |
| `-r`, `-R`, `--recursive` | go down the folder tree |
| `-v`, `--verbose` | say more |
| `-z`, `--compress` | compress (usually with gzip) |
| `-f`, `--file` | in `tar` and `gawk`: a file name follows |
| `-f`, `--force` | in `cp`, `mv` and `rm`: overwrite without asking |

Many programs don't follow it, so check the manual page. The GNU coding
standards (<http://www.gnu.org/prep/standards/>) have the full table.

A tessh script that takes options gets the standard ones, a usage message
and error checking from Python's `argparse` (bash's is `getopts`, chapter
15):

```tessh
import argparse
parser = argparse.ArgumentParser prog=backup description="Back up some files."
parser.add_argument "-v" "--verbose" action="store_true" help="say what's done"
parser.add_argument "-o" metavar="FILE" default="backup.tar" help="the archive"
parser.add_argument "files" nargs="*"
# (Sample options: a script passes $argv[1:].)
args = parser.parse_args ["-v" "-o" "out.tar" "a.txt" "b.txt"]
print $args.verbose $args.o $args.files
parser.print_help
```

```output
True out.tar ['a.txt' 'b.txt']
usage: backup [-h] [-v] [-o FILE] [files ...]

Back up some files.

positional arguments:
  files

options:
  -h, --help     show this help message and exit
  -v, --verbose  say what's done
  -o FILE        the archive
```

## G.2 Bash and tessh Command-Line Options
<!-- help: abs-bash-options tessh-options -c --check --py --with -->

Bash's own options, and tessh's:

| bash | meaning | tessh |
|---|---|---|
| `bash script args…` | run a script | `tessh script args…` |
| `-c string args…` | run the string, the rest its arguments | `tessh -c code args…` (`$argv[0]` is `-c`) |
| `-r`, `--restricted` | restricted mode | none |
| `--posix` | POSIX mode | none |
| `--version` | print the version | `tessh --version` |
| `--` | the end of the options | the script's name ends tessh's options |
| `-n` | read the script without running it | `tessh --check script` (finds much more) |
| | print the Python a script compiles to | `tessh --py script` |
| | PyPI packages to have first | `tessh --with pkg,pkg script` |
| | the guide, in the terminal | `tessh --guide [topic]` |
| | the Jupyter kernel | `tessh --kernel -f conn` |

```bash
bash -c 'set a b c d; IFS="+-;"; echo "$*"'
```

```tessh
tessh -c 'print *$argv[1:] sep="+"' a b c d
```

```output
a+b+c+d
```
