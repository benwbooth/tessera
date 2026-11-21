//! A process's name and working folder, for tab titles.

/// The program's short name ("vim", "zsh").
pub fn name(pid: u32) -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string(format!("/proc/{pid}/comm")).map(|c| c.trim().to_string()).unwrap_or_default()
    }
    #[cfg(target_os = "macos")]
    {
        ps(pid, "comm=").rsplit('/').next().unwrap_or_default().trim_start_matches('-').to_string()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        String::new()
    }
}

/// The folder it's in.
pub fn cwd(pid: u32) -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_link(format!("/proc/{pid}/cwd")).map(|p| p.display().to_string()).unwrap_or_default()
    }
    #[cfg(target_os = "macos")]
    {
        // lsof -Fn prints "p<pid>", "fcwd", then "n<path>".
        std::process::Command::new("lsof")
            .args(["-a", "-d", "cwd", "-p", &pid.to_string(), "-Fn"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).lines().find_map(|l| l.strip_prefix('n').map(str::to_string)))
            .unwrap_or_default()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        String::new()
    }
}

#[cfg(target_os = "macos")]
fn ps(pid: u32, field: &str) -> String {
    std::process::Command::new("ps")
        .args(["-o", field, "-p", &pid.to_string()])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

/// A shell's name from its process: its program, unwrapped (NixOS's
/// .xonsh-wrapped, a Python running xonsh).
pub fn shell_name(pid: u32) -> String {
    let comm = name(pid);
    let comm = comm.strip_prefix('.').and_then(|c| c.strip_suffix("-wrapped")).map(String::from).unwrap_or(comm);
    if comm.starts_with("python") || comm == "xonsh" {
        let cmd = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        if String::from_utf8_lossy(&cmd).contains("xonsh") {
            return "xonsh".into();
        }
    }
    comm
}

pub fn is_shell(name: &str) -> bool {
    matches!(name, "bash" | "zsh" | "fish" | "nu" | "xonsh" | "pwsh" | "sh" | "dash" | "ksh" | "mksh" | "tcsh" | "csh" | "elvish" | "tessh")
}

fn parent(pid: u32) -> u32 {
    // (The field after the name, which may have spaces: after its last ')'.)
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    stat.rsplit_once(')').and_then(|(_, r)| r.split_whitespace().nth(1)).and_then(|p| p.parse().ok()).unwrap_or(0)
}

/// The shells a pane's shell (pid, the one in front) was started in, the
/// first the pane's own: [{name, pid}]. From the processes, so it holds
/// after Tessera restarts. (A nix shell or env between two is passed over.)
pub fn shell_chain(pid: u32) -> String {
    let mut chain = Vec::new();
    let mut p = pid;
    while p > 1 && is_shell(&shell_name(p)) {
        chain.push(serde_json::json!({"name": shell_name(p), "pid": p}));
        let mut q = parent(p);
        while q > 1 && matches!(name(q).as_str(), "nix" | "env" | "sudo" | "doas") {
            q = parent(q);
        }
        p = q;
    }
    chain.reverse();
    serde_json::Value::Array(chain).to_string()
}

/// A signal by name ("INT", "TERM", "KILL", ...), for a card's Signal menu.
#[cfg(unix)]
pub fn signal_number(name: &str) -> Option<i32> {
    Some(match name.trim_start_matches("SIG") {
        "HUP" => libc::SIGHUP,
        "INT" => libc::SIGINT,
        "QUIT" => libc::SIGQUIT,
        "KILL" => libc::SIGKILL,
        "USR1" => libc::SIGUSR1,
        "USR2" => libc::SIGUSR2,
        "TERM" => libc::SIGTERM,
        "CONT" => libc::SIGCONT,
        "STOP" => libc::SIGSTOP,
        "TSTP" => libc::SIGTSTP,
        "WINCH" => libc::SIGWINCH,
        _ => return None,
    })
}

/// The program in front in process `pid`'s terminal (the terminal's
/// foreground process group), sent signal `name`; `pid` is any process on
/// it (the shell, or what it runs). False when the shell (the terminal's
/// session leader) is in front, or the signal isn't known.
pub fn signal_foreground(pid: u32, name: &str) -> bool {
    #[cfg(unix)]
    {
        let Some(sig) = signal_number(name) else { return false };
        let Some((leader, tpgid)) = groups(pid) else { return false };
        if tpgid <= 1 || tpgid == leader {
            return false;
        }
        unsafe { libc::kill(-tpgid, sig) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, name);
        false
    }
}

/// Process `pid`'s session (its leader: the terminal's shell) and its
/// terminal's foreground group.
#[cfg(unix)]
fn groups(pid: u32) -> Option<(i32, i32)> {
    #[cfg(target_os = "linux")]
    {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // (After the name in parentheses: state ppid pgrp session tty_nr tpgid.)
        let f: Vec<&str> = stat[stat.rfind(')')? + 1..].split_whitespace().collect();
        Some((f.get(3)?.parse().ok()?, f.get(5)?.parse().ok()?))
    }
    #[cfg(target_os = "macos")]
    {
        let out = ps(pid, "sess=,tpgid=");
        let mut it = out.split_whitespace().map(|v| v.parse::<i32>().ok());
        Some((it.next()??, it.next()??))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

#[cfg(test)]
mod tests {
    /// TESSERA_CHAIN_PID=<pid>: prints its chain (--ignored --nocapture).
    #[test]
    #[ignore]
    fn chain_of() {
        let pid: u32 = std::env::var("TESSERA_CHAIN_PID").ok().and_then(|p| p.parse().ok()).unwrap_or(std::process::id());
        eprintln!("chain: {}", super::shell_chain(pid));
    }
}
