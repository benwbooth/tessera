# Appendix I. Important System Directories
<!-- help: abs-systemdirs abs-i directories /bin /etc /usr /var /tmp /proc /dev -->

A script that looks after a system needs to know where things are. These
are the same for a tessh script as for a bash one.

| folder | what's in it |
|---|---|
| `/bin`, `/usr/bin` | programs; the split is from when `/` was a small fast disk and `/usr` a big slow one |
| `/usr/local/bin` | programs local to this machine |
| `/sbin`, `/usr/sbin` | system administration programs (such as `fsck`) |
| `/etc` | system-wide configuration: `/etc/fstab` (the filesystems), `/etc/mtab` (the mounted ones), `/etc/inittab` |
| `/etc/rc.d` | boot scripts, on Red Hat and its relatives |
| `/usr/share/doc` | the documentation of installed packages |
| `/usr/man` (now `/usr/share/man`) | the manual pages |
| `/dev` | devices (chapter 29) |
| `/proc` | running processes and kernel settings (chapter 29) |
| `/sys` | devices and their names (Linux 2.6 and later) |
| `/mnt`, `/media` | where disks and removable devices are mounted |
| `/var` | files that change while the system runs; `/var/log` (logs), `/var/spool/mail` (mail) |
| `/lib`, `/usr/lib` | libraries |
| `/tmp` | temporary files |
| `/boot` | the kernel and boot loader: change with care |

On NixOS, where tessh is developed, most programs live in `/nix/store`
and are reached through `/run/current-system/sw/bin` and your profile, so
`/bin` and `/usr/bin` hold almost nothing (`/bin/sh` and `/usr/bin/env`
are kept). That's a reason to start scripts with `#!/usr/bin/env tessh`
and to find programs with `which` (or Python's `shutil.which`), not by a
fixed path. Python finds the temporary folder with `tempfile.gettempdir`,
which honours `$TMPDIR`.

```tessh
import shutil tempfile os
print ((shutil.which "sh") is not None) (os.path.isdir "/etc") ((len (tempfile.gettempdir)) > 0)
```

```output
True True True
```
