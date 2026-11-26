//! carapace (carapace-bin), when it's installed: completions for some 700
//! commands, their subcommands, options short and long (bundles like -la
//! too) and values (git's branches, systemd's units...), each with what it
//! is. Tessera's completer and tessh's (included there by #[path]) ask it
//! first; it answers nothing for a command it doesn't know, and its files
//! and folders are left to them (their quoting, cd's folders only).
//! Only std and serde_json.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct Value {
    pub value: String,
    pub description: String,
    /// "option", "subcommand" or "value".
    pub kind: &'static str,
    /// No space after it: more goes on (-l, then a for -la; a key=).
    pub nospace: bool,
}

/// The carapace on PATH (TESSERA_NO_CARAPACE=1: not used).
pub fn find() -> Option<PathBuf> {
    if std::env::var_os("TESSERA_NO_CARAPACE").is_some_and(|v| v != "0") {
        return None;
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|d| d.join("carapace")).find(|p| p.is_file())
}

/// The words of a line's last simple command (after | ; & ( ), quotes
/// taken off.
pub fn line_words(line: &str) -> Vec<String> {
    let (mut out, mut cur, mut quote, mut esc, mut any) = (Vec::new(), String::new(), None::<char>, false, false);
    for c in line.chars() {
        if esc {
            cur.push(c);
            esc = false;
            continue;
        }
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some('"'), '\\') => esc = true,
            (Some(_), c) => cur.push(c),
            (None, '\\') => {
                esc = true;
                any = true;
            }
            (None, '\'' | '"') => {
                quote = Some(c);
                any = true;
            }
            (None, '|' | ';' | '&' | '(' | '\n') => {
                out.clear();
                cur.clear();
                any = false;
            }
            (None, c) if c.is_whitespace() => {
                if any || !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                any = false;
            }
            (None, c) => cur.push(c),
        }
    }
    if any || !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// What carapace offers for the last of `args` (the command first, the
/// word being typed last, "" for a new one), run in `cwd`: None when it
/// isn't installed, doesn't know the command, or has only files and
/// folders. At most a second (a value list can ask git, docker...).
pub fn complete(args: &[String], cwd: &str) -> Option<Vec<Value>> {
    let cmd = args.first().filter(|c| !c.is_empty() && !c.contains('/'))?;
    let exe = find()?;
    let mut c = Command::new(exe);
    c.arg(cmd).arg("export").args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    if !cwd.is_empty() && std::path::Path::new(cwd).is_dir() {
        c.current_dir(cwd);
    }
    let mut child = c.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut b = Vec::new();
        let _ = stdout.read_to_end(&mut b);
        b
    });
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < Duration::from_secs(1) => std::thread::sleep(Duration::from_millis(2)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
    let out = reader.join().ok()?;
    parse(&String::from_utf8_lossy(&out))
}

/// `carapace CMD export ...`'s JSON: {nospace, values: [{value,
/// description, tag}]}.
pub fn parse(json: &str) -> Option<Vec<Value>> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let nospace = v["nospace"].as_str().unwrap_or("");
    let mut out = Vec::new();
    for x in v["values"].as_array()? {
        let value = x["value"].as_str().unwrap_or("").to_string();
        let tag = x["tag"].as_str().unwrap_or("");
        if value.is_empty() || matches!(tag, "files" | "directories" | "folders") {
            continue;
        }
        let kind = if tag.contains("flag") {
            "option"
        } else if tag.contains("command") {
            "subcommand"
        } else {
            "value"
        };
        let no_space = nospace == "*" || value.chars().last().is_some_and(|c| nospace.contains(c));
        out.push(Value { value, description: x["description"].as_str().unwrap_or("").to_string(), kind, nospace: no_space });
    }
    (!out.is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn words_and_parse() {
        assert_eq!(line_words("echo a | git commit -m 'a b' "), ["git", "commit", "-m", "a b"]);
        assert_eq!(line_words("ls -l"), ["ls", "-l"]);
        let v = parse(r#"{"nospace":"abl","values":[{"value":"-la","description":"all","tag":"shorthand flags"},{"value":"Documents/","tag":"files"},{"value":"main","tag":"local branches"}]}"#).unwrap();
        assert_eq!(v.len(), 2);
        assert!(v[0].nospace && v[0].kind == "option");
        assert!(!v[1].nospace && v[1].kind == "value");
        assert!(parse(r#"{"values":[{"value":"x/","tag":"files"}]}"#).is_none());
    }
}

/// The program's short options and what they do, as carapace has them
/// (for a bundle's meaning): `args` without the word being typed.
pub fn short_options(args: &[String], cwd: &str) -> Vec<(String, String)> {
    let mut a = args.to_vec();
    a.push("-".into());
    complete(&a, cwd)
        .unwrap_or_default()
        .into_iter()
        .filter(|v| v.kind == "option" && v.value.len() == 2 && !v.value.starts_with("--"))
        .map(|v| (v.value, v.description))
        .collect()
}
