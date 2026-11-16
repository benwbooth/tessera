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
