//! Options from a command's man page (or, without one, its GNU info page):
//! (flag, description) pairs, short and long. Tessera's completer and
//! tessh's (included there by #[path]) both read them. Only std.

/// Options from a rendered man page: lines starting with an option (indented
/// a little), and the description on the same line after a gap or on the
/// lines indented below it.
pub fn parse_man(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let indent = |l: &str| l.len() - l.trim_start().len();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let t = line.trim_start();
        let ind = indent(line);
        if t.starts_with('-') && t.len() > 1 && ind > 0 && ind <= 14 && !t.starts_with("--- ") && !t.starts_with("-- ") {
            // The option part: up to two spaces (or the end).
            let (head, rest) = match t.find("  ") {
                Some(p) => (&t[..p], t[p..].trim()),
                None => (t, ""),
            };
            let mut desc = rest.to_string();
            if desc.is_empty() {
                let mut j = i + 1;
                while j < lines.len() && !lines[j].trim().is_empty() && indent(lines[j]) > ind {
                    if !desc.is_empty() {
                        desc.push(' ');
                    }
                    desc.push_str(lines[j].trim());
                    j += 1;
                    if desc.len() > 300 {
                        break;
                    }
                }
            }
            for flag in head.split(", ").flat_map(|p| p.split(" | ")) {
                let flag = flag.trim();
                // "-o, --output=FILE" -> "--output" (the argument shows in the detail).
                let name: String = flag.chars().take_while(|c| !matches!(c, '=' | '[' | ' ' | '<')).collect();
                if name.len() < 2 || !name.starts_with('-') || name.chars().nth(1).is_some_and(|c| c.is_ascii_digit() && name.len() > 3) {
                    continue;
                }
                let arg = flag[name.len()..].trim();
                let detail = if arg.is_empty() { desc.clone() } else { format!("{arg}  {desc}") };
                if !out.iter().any(|(n, _)| *n == name) {
                    out.push((name, detail));
                }
            }
        }
        i += 1;
    }
    for (_, d) in out.iter_mut() {
        // (Justified text has runs of spaces.)
        *d = d.split_whitespace().collect::<Vec<_>>().join(" ");
        if d.len() > 240 {
            let cut = (0..=240).rev().find(|&k| d.is_char_boundary(k)).unwrap_or(0);
            d.truncate(cut);
            d.push('…');
        }
    }
    out
}

/// GNU info pages (for commands documented there rather than in man):
/// options as "‘-a’" / "‘--all’" lines with the description indented below.
pub fn parse_info(text: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<(String, String)> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let mut flags = Vec::new();
        while i < lines.len() {
            let t = lines[i].trim();
            let q = t.strip_prefix('\u{2018}').and_then(|r| r.strip_suffix('\u{2019}')).or_else(|| t.strip_prefix('`').and_then(|r| r.strip_suffix('\'')));
            match q {
                Some(f) if f.starts_with('-') => {
                    flags.push(f.to_string());
                    i += 1;
                }
                _ => break,
            }
        }
        if flags.is_empty() {
            i += 1;
            continue;
        }
        let mut desc = String::new();
        while i < lines.len() && !lines[i].trim().is_empty() && lines[i].starts_with("     ") {
            if !desc.is_empty() {
                desc.push(' ');
            }
            desc.push_str(lines[i].trim());
            i += 1;
            if desc.len() > 240 {
                break;
            }
        }
        let desc = desc.split_whitespace().collect::<Vec<_>>().join(" ");
        for f in flags {
            let name: String = f.chars().take_while(|c| !matches!(c, '=' | '[' | ' ')).collect();
            if name.len() >= 2 && !out.iter().any(|(n, _)| *n == name) {
                out.push((name, desc.clone()));
            }
        }
    }
    out
}

pub fn info_options(page: &str) -> Vec<(String, String)> {
    let Ok(o) = std::process::Command::new("info").args(["--output=-", page]).stderr(std::process::Stdio::null()).output() else { return Vec::new() };
    if !o.status.success() {
        return Vec::new();
    }
    parse_info(&String::from_utf8_lossy(&o.stdout))
}

pub fn man_options(page: &str) -> Vec<(String, String)> {
    if cfg!(windows) {
        return Vec::new();
    }
    let out = std::process::Command::new("man")
        .args(["-P", "cat", page])
        .env("MANWIDTH", "400")
        .env("MAN_KEEP_FORMATTING", "0")
        .env("GROFF_NO_SGR", "1")
        .stderr(std::process::Stdio::null())
        .output();
    let Ok(o) = out else { return Vec::new() };
    // Strip overstrike (x\bx bold, _\bx underline).
    let raw = String::from_utf8_lossy(&o.stdout);
    let mut text = String::with_capacity(raw.len());
    for c in raw.chars() {
        if c == '\u{8}' {
            text.pop();
        } else {
            text.push(c);
        }
    }
    let opts = parse_man(&text);
    if opts.is_empty() {
        return info_options(page);
    }
    opts
}


/// A bundle of short options being typed (-l, -la): it with each other
/// short option after it, as `ls -la` takes them, each with what the added
/// one does. Nothing for a long option or one not all short flags.
pub fn bundled(typed: &str, opts: &[(String, String)]) -> Vec<(String, String)> {
    let Some(letters) = typed.strip_prefix('-').filter(|l| !l.is_empty() && !l.starts_with('-') && l.chars().all(|c| c.is_ascii_alphanumeric())) else { return vec![] };
    let short = |f: &str| f.len() == 2 && f.starts_with('-') && f.chars().nth(1).is_some_and(|c| c.is_ascii_alphanumeric());
    let have = |c: char| opts.iter().any(|(f, _)| short(f) && f.ends_with(c));
    if !letters.chars().all(have) {
        return vec![];
    }
    opts.iter()
        .filter(|(f, _)| short(f) && !letters.contains(f.chars().nth(1).unwrap()))
        .map(|(f, d)| (format!("{typed}{}", &f[1..]), d.clone()))
        .collect()
}

/// What a bundle of short options being typed means (-la: -l's and -a's
/// descriptions): (one line, a line per letter), or None when it isn't one
/// or a letter isn't an option of the command's.
pub fn bundle_meaning(typed: &str, opts: &[(String, String)]) -> Option<(String, String)> {
    let letters = typed.strip_prefix('-').filter(|l| !l.is_empty() && !l.starts_with('-') && l.chars().all(|c| c.is_ascii_alphanumeric()))?;
    let mut line = Vec::new();
    let mut doc = Vec::new();
    for c in letters.chars() {
        let flag = format!("-{c}");
        let d = opts.iter().find(|(f, _)| *f == flag).map(|(_, d)| d.clone())?;
        // (Its first sentence: "starting with ." keeps its dot.)
        let first = d.split("; ").next().unwrap_or("").split(". ").next().unwrap_or("");
        let short: String = first.chars().take(60).collect();
        line.push(format!("{flag} {}", short.trim()));
        doc.push(format!("{flag}  {d}"));
    }
    Some((line.join(" · "), doc.join("\n")))
}

/// Shell builtins' options, which no man page or carapace has: cd's and
/// the folder stack's (as tessh's and bash's take them). `oldpwd`: the
/// folder `cd -` goes to, when known.
pub fn builtin_options(cmd: &str, oldpwd: Option<&str>) -> Option<Vec<(String, String)>> {
    let back = match oldpwd.filter(|p| !p.is_empty()) {
        Some(p) => format!("back to the folder you were in: {p}"),
        None => "back to the folder you were in before".to_string(),
    };
    let o = |f: &str, d: &str| (f.to_string(), d.to_string());
    Some(match cmd {
        "cd" => vec![
            ("-".to_string(), back),
            o("-L", "keep the path as written, symlinks and all (the default)"),
            o("-P", "go by the real folders, symlinks resolved"),
            o("-e", "with -P: fail if the folder it ends in can't be worked out"),
            o("--", "the end of the options: a folder named with a - next"),
        ],
        "pushd" => vec![
            o("-n", "change the stack only, not the folder"),
            o("+1", "turn the stack so its 2nd folder (from the left of dirs) is on top"),
            o("-0", "turn the stack so its last folder (from the right of dirs) is on top"),
        ],
        "popd" => vec![
            o("-n", "change the stack only, not the folder"),
            o("+1", "drop the 2nd folder of the stack (from the left of dirs)"),
            o("-0", "drop the last folder of the stack (from the right of dirs)"),
        ],
        "dirs" => vec![
            o("-c", "clear the stack"),
            o("-l", "spell out ~ as the full path"),
            o("-p", "one folder a line"),
            o("-v", "one folder a line, numbered"),
        ],
        _ => return None,
    })
}
