//! Installing the shell integration scripts (shell/tessera.*).

const BASH: &str = include_str!("../shell/tessera.bash");
const ZSH: &str = include_str!("../shell/tessera.zsh");
const FISH: &str = include_str!("../shell/tessera.fish");
const NU: &str = include_str!("../shell/tessera.nu");
const XONSH: &str = include_str!("../shell/tessera.xsh");
const PWSH: &str = include_str!("../shell/tessera.ps1");

pub fn install(shell: &str) -> String {
    let (script, file, rc) = match shell {
        "bash" => (BASH, "tessera.bash", ".bashrc"),
        "zsh" => (ZSH, "tessera.zsh", ".zshrc"),
        "fish" => (FISH, "tessera.fish", ".config/fish/conf.d/tessera.fish"),
        _ => return format!("Shell integration isn't available for {shell}."),
    };
    let dir = crate::settings::config_dir().join("shell");
    if let Err(e) = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(dir.join(file), script)) {
        return format!("Couldn't write the script: {e}");
    }
    let script_path = dir.join(file);
    let home = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let rc_path = home.join(rc);
    let line = format!("[ -f \"{0}\" ] && source \"{0}\"", script_path.display());
    if shell == "fish" {
        // conf.d files run by themselves: link the script there.
        if let Some(d) = rc_path.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::remove_file(&rc_path);
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&script_path, &rc_path);
        #[cfg(not(unix))]
        let linked = std::fs::copy(&script_path, &rc_path).map(|_| ());
        return match linked {
            Ok(()) => format!("Installed for fish ({}). New fish sessions have it.", rc_path.display()),
            Err(e) => format!("Couldn't link {}: {e}", rc_path.display()),
        };
    }
    let current = std::fs::read_to_string(&rc_path).unwrap_or_default();
    if current.contains(&script_path.display().to_string()) {
        return format!("Already installed in ~/{rc}. New {shell} sessions have it.");
    }
    // Home Manager and similar make the startup file a read-only link into
    // the Nix store: then say what to add to the configuration instead.
    let managed = std::fs::symlink_metadata(&rc_path).map(|m| m.file_type().is_symlink()).unwrap_or(false);
    let appended = !managed
        && std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&rc_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, format!("\n# Tessera shell integration\n{line}\n").as_bytes()))
            .is_ok();
    if appended {
        format!("Installed: added a line to ~/{rc}. New {shell} sessions have it.")
    } else {
        format!("~/{rc} can't be changed here (managed by Nix?). Add this to your {shell} configuration:\n\n{line}")
    }
}

/// Loads the integration into new shells automatically, as Ghostty and
/// iTerm2 do, so prompts are marked without editing any startup file:
/// zsh through a ZDOTDIR whose .zshenv puts the user's ZDOTDIR back, runs
/// their .zshenv and then the script; fish through a vendor_conf.d folder on
/// XDG_DATA_DIRS. (The marks let the terminal clear a prompt that the shell
/// redraws on resize, instead of leaving its old lines behind.)
pub fn inject(cmd: &mut portable_pty::CommandBuilder) {
    if std::env::var_os("TESSERA_NO_SHELL_INTEGRATION").is_some() {
        return;
    }
    let Some(base) = write_auto_scripts() else { return };
    let zdot = base.join("zsh");
    if let Some(old) = std::env::var_os("ZDOTDIR") {
        cmd.env("TESSERA_ZDOTDIR", old);
    }
    cmd.env("ZDOTDIR", &zdot);
    let data = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    cmd.env("XDG_DATA_DIRS", format!("{}:{data}", base.display()));
    cmd.env("TESSERA_FISH_DATA_DIR", base.display().to_string());
}

const ZSHENV: &str = r#"# Tessera: put ZDOTDIR back, run the user's .zshenv, then load the
# shell integration (for interactive shells only).
if [[ -n "${TESSERA_ZDOTDIR+X}" ]]; then
    ZDOTDIR=$TESSERA_ZDOTDIR
    unset TESSERA_ZDOTDIR
else
    unset ZDOTDIR
fi
{
    [[ -r "${ZDOTDIR-$HOME}/.zshenv" ]] && source "${ZDOTDIR-$HOME}/.zshenv"
} always {
    [[ -o interactive ]] && source "${${(%):-%x}:A:h}/tessera.zsh"
}
"#;

fn write_auto_scripts() -> Option<std::path::PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/share")))?
        .join("tessera/shell-integration");
    let files = [
        ("zsh/.zshenv", ZSHENV),
        ("zsh/tessera.zsh", ZSH),
        ("fish/vendor_conf.d/tessera.fish", FISH),
        ("bash/tessera.bash", BASH),
        ("bash/bashrc", BASHRC),
        ("nu/tessera.nu", NU),
        ("xonsh/tessera.xsh", XONSH),
        ("powershell/tessera.ps1", PWSH),
    ];
    for (rel, text) in files {
        let p = base.join(rel);
        if std::fs::read_to_string(&p).ok().as_deref() != Some(text) {
            std::fs::create_dir_all(p.parent()?).ok()?;
            std::fs::write(&p, text).ok()?;
        }
    }
    Some(base)
}

// A nested bash: the user's .bashrc, then the integration.
const BASHRC: &str = "[ -f ~/.bashrc ] && . ~/.bashrc\n. \"$(dirname \"${BASH_SOURCE[0]}\")/tessera.bash\"\n";

/// The line that starts `shell` inside the pane's shell (parent: its pid),
/// with Tessera's integration loaded (zsh, bash, fish), the program from Nix
/// when it isn't installed. (A space first: kept out of the history.)
pub fn launch_line(shell: &str, parent: u32) -> String {
    #[allow(unused)]
    let (prog, pkg) = match shell {
        "bash" => ("bash", "bashInteractive"),
        "zsh" => ("zsh", "zsh"),
        "fish" => ("fish", "fish"),
        "nu" => ("nu", "nushell"),
        "xonsh" => ("xonsh", "xonsh"),
        "powershell" | "pwsh" => ("pwsh", "powershell"),
        "elvish" => ("elvish", "elvish"),
        "ksh" => ("ksh", "ksh"),
        "mksh" => ("mksh", "mksh"),
        "dash" => ("dash", "dash"),
        "tcsh" => ("tcsh", "tcsh"),
        "sh" => ("sh", "bashInteractive"),
        _ => return String::new(),
    };
    let on_path = std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(prog).is_file()));
    // (Not installed: from Nix: its kept path when it's been fetched before,
    // else through nix shell now, fetched and kept for next time meanwhile.)
    let run = if on_path {
        prog.to_string()
    } else {
        let argv: Vec<String> = ["nix", "shell", &format!("nixpkgs#{pkg}"), "-c", prog].iter().map(|s| s.to_string()).collect();
        match crate::jupyter::nix_kept(&argv) {
            Some(exe) => format!("'{}'", exe.replace('\'', "'\\''")),
            None => {
                std::thread::spawn(move || {
                    let _ = crate::jupyter::nix_cached(&argv, &|_| {});
                });
                format!("nix shell nixpkgs#{pkg} -c {prog}")
            }
        }
    };
    let q = |p: &std::path::Path| format!("'{}'", p.display().to_string().replace('\'', "'\\''"));
    let Some(base) = write_auto_scripts() else { return format!(" {run}") };
    match shell {
        "zsh" => {
            // (Through Tessera's ZDOTDIR, which puts the user's back.)
            let user = crate::prompt::shell_env(parent).and_then(|e| e.into_iter().find(|(k, _)| k == "ZDOTDIR").map(|(_, v)| v)).filter(|z| !z.contains("tessera/shell-integration"));
            let back = user.map(|z| format!(" TESSERA_ZDOTDIR='{}'", z.replace('\'', "'\\''"))).unwrap_or_default();
            format!(" env ZDOTDIR={}{back} {run}", q(&base.join("zsh")))
        }
        "bash" => format!(" {run} --rcfile {}", q(&base.join("bash/bashrc"))),
        "fish" => format!(" {run} --init-command 'source {}'", base.join("fish/vendor_conf.d/tessera.fish").display()),
        "nu" => format!(" {run} -e {}", q(&std::path::PathBuf::from(format!("source {}", base.join("nu/tessera.nu").display())))),
        "xonsh" => format!(" {run} -i --rc {}", q(&base.join("xonsh/tessera.xsh"))),
        "powershell" | "pwsh" => format!(" {run} -NoLogo -NoExit -Command {}", q(&std::path::PathBuf::from(format!(". '{}'", base.join("powershell/tessera.ps1").display())))),
        _ => format!(" {run}"),
    }
}

/// Leaving a nested shell (pid) for the one it was started from (parent):
/// its folder and what it exported go back down (applied there before its
/// next prompt), so switching shells carries them along.
pub fn hand_back(child: u32, parent: u32) {
    if child == 0 || parent == 0 || child == parent {
        return;
    }
    let (Some(now), Some(was)) = (crate::prompt::shell_env(child), crate::prompt::shell_env(parent)) else { return };
    let now: std::collections::HashMap<String, String> = now.into_iter().collect();
    let was: std::collections::HashMap<String, String> = was.into_iter().collect();
    // (Not the shells' own: their levels, prompts, integration.)
    let own = |k: &str| {
        matches!(k, "SHLVL" | "_" | "PWD" | "OLDPWD" | "PS1" | "PS2" | "PROMPT" | "RPROMPT" | "ZDOTDIR" | "TESSERA_ZDOTDIR" | "SHELL" | "TESSERA_SHELL_INTEGRATION" | "fish_pid" | "XONSH_VERSION")
            || k.starts_with("BASH")
            || k.starts_with("ZSH")
            || k.starts_with("__fish")
    };
    let set: Vec<(String, String)> = now.iter().filter(|(k, v)| !own(k) && was.get(*k) != Some(v)).map(|(k, v)| (k.clone(), v.clone())).collect();
    let unset: Vec<String> = was.keys().filter(|k| !own(k) && !now.contains_key(*k)).cloned().collect();
    let cwd = std::fs::read_link(format!("/proc/{child}/cwd")).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let pcwd = std::fs::read_link(format!("/proc/{parent}/cwd")).map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let cd = (!cwd.is_empty() && cwd != pcwd).then_some(cwd);
    if set.is_empty() && unset.is_empty() && cd.is_none() {
        return;
    }
    crate::jupyter::write_apply(parent, set, unset, cd);
}
