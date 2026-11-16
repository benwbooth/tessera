//! Installing the shell integration scripts (shell/tessera.*).

const BASH: &str = include_str!("../shell/tessera.bash");
const ZSH: &str = include_str!("../shell/tessera.zsh");
const FISH: &str = include_str!("../shell/tessera.fish");

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
