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
        return match std::os::unix::fs::symlink(&script_path, &rc_path) {
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
