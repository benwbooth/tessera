//! tessh as a login shell (chsh, sshd, login, a terminal's "run as a login
//! shell"): the usual shell flags, the profile's environment, and sh for the
//! commands other programs send over ssh.
//!
//! - `-l`/`--login`, or a `-` before argv[0] (how login and sshd start one):
//!   the environment /etc/profile, /etc/profile.d, ~/.profile (and NixOS's
//!   set-environment, which /etc/profile reads) give, imported by running
//!   `sh -l` once; then ~/.config/tessh/login.tsh.
//! - `-i`: accepted (the REPL needs a terminal: without one, stdin is run as
//!   a script). `-s` (or stdin not a terminal, with no file): the script
//!   read from stdin, arguments after.
//! - `-c CMD` started by sshd (`ssh host cmd`, scp, rsync, VS Code's remote
//!   server): CMD goes to /bin/sh, which those programs expect. A `-c` typed
//!   anywhere else stays tessh. TESSH_SH_C=1 or 0 decides it either way.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Set while `sh -l` runs for the environment: a profile that starts tessh
/// (`exec tessh` in ~/.profile) gets the environment printed, not a REPL.
const PROBE: &str = "TESSH_PROFILE_PROBE";

#[derive(Default, Debug, PartialEq)]
pub struct Opts {
    pub login: bool,
    pub interactive: bool,
    pub stdin: bool,
}

/// The shell flags before the rest (`-l`, `--login`, `-i`, `-s`, together as
/// in `-li`; `-lc CMD` is `-l -c CMD`), and a `-` before argv[0].
pub fn parse(argv0: &str, mut args: Vec<String>) -> (Opts, Vec<String>) {
    let mut o = Opts { login: std::path::Path::new(argv0).file_name().map_or(argv0, |n| n.to_str().unwrap_or("")).starts_with('-'), ..Default::default() };
    loop {
        let Some(a) = args.first() else { break };
        if a == "--login" {
            o.login = true;
            args.remove(0);
            continue;
        }
        let Some(flags) = a.strip_prefix('-') else { break };
        if flags.is_empty() || flags.starts_with('-') || !flags.chars().all(|c| "lisc".contains(c)) || flags == "c" {
            break;
        }
        // (c only last: -lc CMD; -cl isn't a cluster of these.)
        if flags[..flags.len() - 1].contains('c') {
            break;
        }
        for c in flags.chars() {
            match c {
                'l' => o.login = true,
                'i' => o.interactive = true,
                's' => o.stdin = true,
                _ => {}
            }
        }
        if flags.ends_with('c') {
            args[0] = "-c".into();
            break;
        }
        args.remove(0);
    }
    (o, args)
}

/// Started by a profile while tessh reads it: print the environment as
/// `env -0` would and stop, so `exec tessh` in ~/.profile doesn't loop.
pub fn probe_guard() {
    if std::env::var_os(PROBE).is_some() {
        let mut out = Vec::new();
        for (k, v) in std::env::vars_os() {
            if k == PROBE {
                continue;
            }
            out.extend_from_slice(k.as_encoded_bytes());
            out.push(b'=');
            out.extend_from_slice(v.as_encoded_bytes());
            out.push(0);
        }
        use std::io::Write;
        let _ = std::io::stdout().write_all(&out);
        std::process::exit(0);
    }
}

/// The login environment: `sh -l -c 'env -0'` (no terminal, 10 s at most),
/// taken into tessh's own. Its messages go to stderr, as a login's would.
pub fn import_profile() {
    let Ok(mut child) = Command::new("/bin/sh").args(["-l", "-c", "env -0"]).env(PROBE, "1").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn() else {
        return;
    };
    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = stdout.read_to_end(&mut b);
        b
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < Duration::from_secs(10) => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                eprintln!("tessh: the profile took over 10 s; going on without its environment");
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
        }
    }
    let Ok(bytes) = reader.join() else { return };
    for (k, v) in parse_env(&bytes) {
        unsafe { std::env::set_var(k, v) };
    }
}

/// `env -0`'s output, less what belongs to that sh (its folder, level, `_`).
fn parse_env(bytes: &[u8]) -> Vec<(String, String)> {
    bytes
        .split(|b| *b == 0)
        .filter_map(|e| {
            let s = std::str::from_utf8(e).ok()?;
            let (k, v) = s.split_once('=')?;
            (!k.is_empty() && !matches!(k, "PWD" | "OLDPWD" | "SHLVL" | "_" | PROBE)).then(|| (k.to_string(), v.to_string()))
        })
        .collect()
}

/// Whether a `-c` command goes to /bin/sh: sshd started this tessh (`ssh
/// host cmd`, scp, rsync), or TESSH_SH_C=1.
pub fn sh_for_c() -> bool {
    match std::env::var("TESSH_SH_C").as_deref() {
        Ok("1") => return true,
        Ok("0") => return false,
        _ => {}
    }
    parent_is_sshd()
}

#[cfg(target_os = "linux")]
fn parent_is_sshd() -> bool {
    let ppid = unsafe { libc::getppid() };
    std::fs::read_to_string(format!("/proc/{ppid}/comm")).is_ok_and(|c| c.trim().starts_with("sshd"))
}

#[cfg(not(target_os = "linux"))]
fn parent_is_sshd() -> bool {
    // (macOS: no /proc; sshd's own variable, and no terminal.)
    std::env::var_os("SSH_CONNECTION").is_some() && unsafe { libc::isatty(0) } == 0
}

/// Run `-c CMD [NAME ARGS…]` in /bin/sh, as that shell would: its status.
pub fn run_sh_c(args: &[String]) -> i32 {
    use std::os::unix::process::CommandExt;
    let err = Command::new("/bin/sh").args(args).exec();
    eprintln!("tessh: /bin/sh: {err}");
    127
}

/// ~/.config/tessh/login.tsh, for a login shell (after the profile, before rc.tsh).
pub fn login_file() -> std::path::PathBuf {
    let config = std::env::var("XDG_CONFIG_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config"));
    config.join("tessh/login.tsh")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(argv0: &str, a: &[&str]) -> (Opts, Vec<String>) {
        parse(argv0, a.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn flags() {
        assert_eq!(p("tessh", &[]), (Opts::default(), vec![]));
        assert!(p("-tessh", &[]).0.login);
        assert_eq!(p("tessh", &["-l"]).0, Opts { login: true, ..Default::default() });
        assert_eq!(p("tessh", &["--login", "-i"]).0, Opts { login: true, interactive: true, ..Default::default() });
        assert_eq!(p("tessh", &["-li", "x.tsh"]), (Opts { login: true, interactive: true, ..Default::default() }, vec!["x.tsh".into()]));
        assert_eq!(p("tessh", &["-lc", "ls"]), (Opts { login: true, ..Default::default() }, vec!["-c".into(), "ls".into()]));
        assert_eq!(p("tessh", &["-c", "ls"]).1, vec!["-c".to_string(), "ls".into()]);
        assert_eq!(p("tessh", &["-s", "a", "b"]), (Opts { stdin: true, ..Default::default() }, vec!["a".into(), "b".into()]));
        assert_eq!(p("tessh", &["--check"]).1, vec!["--check".to_string()]);
        assert_eq!(p("tessh", &["-cl"]).1, vec!["-cl".to_string()]);
    }

    #[test]
    fn env_parse() {
        let v = parse_env(b"PATH=/a:/b\0PWD=/x\0SHLVL=1\0EMPTY=\0WEIRD=a=b\0\0");
        assert_eq!(v, vec![("PATH".into(), "/a:/b".into()), ("EMPTY".into(), "".into()), ("WEIRD".into(), "a=b".into())]);
    }
}
