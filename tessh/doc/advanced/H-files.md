# Appendix H. Important Files
<!-- help: abs-files abs-h startup-files bashrc bash_profile rc.tsh passwd -->

**Startup files** set the environment, aliases and functions for an
interactive shell.

| bash | read by | tessh |
|---|---|---|
| `/etc/profile` | every Bourne-type login shell: system defaults, mostly the environment | read by your login shell, which hands its environment to tessh |
| `/etc/bashrc` | interactive bash: system functions and aliases | none |
| `~/.bash_profile` | your login shell: your environment | the same (tessh inherits it) |
| `~/.bashrc` | interactive bash: your functions, aliases, prompt | `~/.config/tessh/rc.tsh`, run before the REPL's first prompt |
| `~/.bash_logout` | a login shell, as it exits | `~/.config/tessh/logout.tsh` |
| `~/.bash_history` | the command history | `~/.local/share/tessh/history` |

`rc.tsh` is tessh code like any other: functions, `export`s, imports. A
script doesn't read it, so a script works the same for everyone. Appendix
M has a sample.

**Data files.** `/etc/passwd` lists the accounts: name, user and group
IDs, home folder and shell (the passwords are in `/etc/shadow`, encrypted).
Python's `pwd` and `grp` modules read them (and the network's accounts
too, which the file may not have):

```tessh
import pwd
root = pwd.getpwnam root
print $root.pw_uid $root.pw_dir
```

```output
0 /root
```

**System configuration files.** The ABS Guide's example is Red Hat's old
`/etc/sysconfig/hwconf`, a list of the hardware. Systems now have `lspci`,
`lsusb` and `/sys` for that, and tessh runs them as bash does.
