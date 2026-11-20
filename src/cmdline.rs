//! A command line laid out to read, for a card's details (ⓘ): each program
//! on a line of its own (its folder dim, its name bold), then its arguments
//! one per line ("--flag value" kept together), each colored by what it is:
//! flags, values, paths (the last part brighter), numbers, URLs, Nix store
//! names and versions, quoted strings, redirections. Pipes and && / ; end a
//! line. (As treetop's process details, crates/core/src/command.rs there.)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// The program's folder.
    Dir,
    /// The program's name (bold).
    Prog,
    Flag,
    Eq,
    Value,
    Path,
    /// A path's last part.
    Leaf,
    Number,
    Url,
    /// A quoted string.
    Quoted,
    /// | && || ; & and redirections.
    Op,
    Plain,
    Space,
    /// The line continuations.
    Faint,
    StoreName,
    StoreVersion,
}

impl Kind {
    /// (color, alpha over the text color); None = the text color at that alpha.
    fn color(self, dark: bool) -> (Option<&'static str>, f64) {
        use Kind::*;
        match (self, dark) {
            (Dir | Eq, true) => (None, 0.45),
            (Dir | Eq, false) => (None, 0.5),
            (Faint, _) => (None, 0.25),
            (Prog, true) => (Some("#7fe08f"), 1.0),
            (Prog, false) => (Some("#17803a"), 1.0),
            (Flag, true) => (Some("#56b6e0"), 1.0),
            (Flag, false) => (Some("#16709a"), 1.0),
            (Value, true) => (Some("#e3c25a"), 1.0),
            (Value, false) => (Some("#8f6a00"), 1.0),
            (Quoted, true) => (Some("#98c379"), 1.0),
            (Quoted, false) => (Some("#3f7a1f"), 1.0),
            (Path, true) => (Some("#7f97c9"), 1.0),
            (Path, false) => (Some("#3f5d99"), 1.0),
            (Leaf, true) => (Some("#a9c1f2"), 1.0),
            (Leaf, false) => (Some("#203f80"), 1.0),
            (Number, true) => (Some("#c98ae0"), 1.0),
            (Number, false) => (Some("#86379f"), 1.0),
            (Url, true) => (Some("#e8995a"), 1.0),
            (Url, false) => (Some("#a8541a"), 1.0),
            (Op, true) => (Some("#e06c75"), 1.0),
            (Op, false) => (Some("#b0303a"), 1.0),
            (StoreName, true) => (Some("#93c2b0"), 1.0),
            (StoreName, false) => (Some("#2f7562"), 1.0),
            (StoreVersion, true) => (Some("#c4ab84"), 1.0),
            (StoreVersion, false) => (Some("#86663a"), 1.0),
            (Plain | Space, _) => (None, 1.0),
        }
    }
}

type Spans = Vec<(Kind, String)>;

fn push(out: &mut Spans, k: Kind, t: &str) {
    if !t.is_empty() {
        out.push((k, t.to_string()));
    }
}

fn nix32(c: u8) -> bool {
    c.is_ascii_digit() || (c.is_ascii_lowercase() && !matches!(c, b'e' | b'o' | b't' | b'u'))
}

/// Where each /nix/store entry's "name-version" is in `s` (byte ranges).
fn store_entries(s: &str) -> Vec<(usize, usize)> {
    const P: &str = "/nix/store/";
    let b = s.as_bytes();
    let mut found = Vec::new();
    let mut i = 0;
    while let Some(at) = s[i..].find(P) {
        let h = i + at + P.len();
        if h + 33 <= b.len() && b[h..h + 32].iter().all(|&c| nix32(c)) && b[h + 32] == b'-' {
            let start = h + 33;
            let end = s[start..].find(['/', ':', ' ']).map_or(s.len(), |e| start + e);
            if end > start {
                found.push((start, end));
            }
            i = end;
        } else {
            i = h;
        }
    }
    found
}

/// Spans of `kind`, with any store entry's name and version picked out
/// (the version from the first "-" followed by a digit, as Nix has it).
fn store(out: &mut Spans, kind: Kind, s: &str) {
    let mut last = 0;
    for (a, z) in store_entries(s) {
        push(out, kind, &s[last..a]);
        let e = &s[a..z];
        match e.as_bytes().windows(2).position(|w| w[0] == b'-' && w[1].is_ascii_digit()).filter(|&v| v > 0) {
            Some(v) => {
                push(out, Kind::StoreName, &e[..v]);
                push(out, kind, "-");
                push(out, Kind::StoreVersion, &e[v + 1..]);
            }
            None => push(out, Kind::StoreName, e),
        }
        last = z;
    }
    push(out, kind, &s[last..]);
}

fn path(out: &mut Spans, tok: &str) {
    let entry_last = store_entries(tok).last().is_some_and(|&(_, z)| z == tok.len());
    match tok.rfind('/') {
        Some(i) if i + 1 < tok.len() && !entry_last => {
            store(out, Kind::Path, &tok[..=i]);
            push(out, Kind::Leaf, &tok[i + 1..]);
        }
        _ => store(out, Kind::Path, tok),
    }
}

/// -?[0-9][0-9.,:<extra>]*
fn numeric(t: &str, extra: &str) -> bool {
    let t = t.strip_prefix('-').unwrap_or(t);
    let mut cs = t.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_digit()) && cs.all(|c| c.is_ascii_digit() || ".,:".contains(c) || extra.contains(c))
}

fn quoted(t: &str) -> bool {
    t.len() >= 2 && ((t.starts_with('\'') && t.ends_with('\'')) || (t.starts_with('"') && t.ends_with('"')) || t.starts_with("$'"))
}

fn value(out: &mut Spans, v: &str) {
    if quoted(v) {
        push(out, Kind::Quoted, v);
    } else if v.find("://").is_some_and(|i| i > 0) {
        push(out, Kind::Url, v);
    } else if v.contains('/') {
        path(out, v);
    } else if numeric(v, "xXabcdefABCDEFkKmMgG") {
        push(out, Kind::Number, v);
    } else {
        push(out, Kind::Value, v);
    }
}

fn is_flag(tok: &str) -> bool {
    tok.starts_with('-') && tok.len() > 1 && !tok[1..].starts_with(|c: char| c.is_ascii_digit())
}

fn ident(s: &str) -> bool {
    let mut cs = s.chars();
    matches!(cs.next(), Some(c) if c.is_ascii_alphabetic() || c == '_') && cs.all(|c| c.is_ascii_alphanumeric() || "_.-".contains(c))
}

/// One argument's spans.
fn arg(out: &mut Spans, tok: &str) {
    if is_flag(tok) {
        match tok.find('=') {
            Some(eq) if eq > 0 => {
                push(out, Kind::Flag, &tok[..eq]);
                push(out, Kind::Eq, "=");
                value(out, &tok[eq + 1..]);
            }
            _ => push(out, Kind::Flag, tok),
        }
        return;
    }
    if let Some(eq) = tok.find('=') {
        if eq > 0 && ident(&tok[..eq]) {
            push(out, Kind::Flag, &tok[..eq]);
            push(out, Kind::Eq, "=");
            value(out, &tok[eq + 1..]);
            return;
        }
    }
    if quoted(tok) {
        push(out, Kind::Quoted, tok);
    } else if tok.find("://").is_some_and(|i| i > 0) {
        push(out, Kind::Url, tok);
    } else if tok.contains('/') {
        path(out, tok);
    } else if numeric(tok, "") {
        push(out, Kind::Number, tok);
    } else {
        push(out, Kind::Plain, tok);
    }
}

#[derive(Debug, PartialEq)]
enum Tok {
    Word(String),
    /// | || && ; & ( )
    Sep(String),
    /// > >> < 2> &> 2>&1 ...
    Redir(String),
}

/// The line as the shell would split it, each word as written (quotes and
/// backslashes kept).
fn tokens(line: &str) -> Vec<Tok> {
    let cs: Vec<char> = line.chars().collect();
    let mut out = Vec::new();
    let mut w = String::new();
    let mut i = 0;
    let flush = |w: &mut String, out: &mut Vec<Tok>| {
        if !w.is_empty() {
            out.push(Tok::Word(std::mem::take(w)));
        }
    };
    while i < cs.len() {
        let c = cs[i];
        match c {
            '\'' | '"' => {
                // (To its closing quote; in "...", a backslash escapes.)
                w.push(c);
                i += 1;
                while i < cs.len() && cs[i] != c {
                    if c == '"' && cs[i] == '\\' && i + 1 < cs.len() {
                        w.push(cs[i]);
                        i += 1;
                    }
                    w.push(cs[i]);
                    i += 1;
                }
                if i < cs.len() {
                    w.push(c);
                    i += 1;
                }
            }
            '\\' if i + 1 < cs.len() => {
                w.push(c);
                w.push(cs[i + 1]);
                i += 2;
            }
            ' ' | '\t' | '\n' => {
                flush(&mut w, &mut out);
                i += 1;
            }
            '|' | ';' | '(' | ')' => {
                flush(&mut w, &mut out);
                let two = c == '|' && cs.get(i + 1) == Some(&'|');
                out.push(Tok::Sep(if two { "||".into() } else { c.to_string() }));
                i += if two { 2 } else { 1 };
            }
            '&' => {
                if cs.get(i + 1) == Some(&'&') {
                    flush(&mut w, &mut out);
                    out.push(Tok::Sep("&&".into()));
                    i += 2;
                } else if cs.get(i + 1) == Some(&'>') {
                    flush(&mut w, &mut out);
                    let n = if cs.get(i + 2) == Some(&'>') { 3 } else { 2 };
                    out.push(Tok::Redir(cs[i..i + n].iter().collect()));
                    i += n;
                } else {
                    flush(&mut w, &mut out);
                    out.push(Tok::Sep("&".into()));
                    i += 1;
                }
            }
            '>' | '<' => {
                // (With a number before it, as in 2>: that's part of it.)
                let mut r = String::new();
                if !w.is_empty() && w.chars().all(|d| d.is_ascii_digit()) {
                    r = std::mem::take(&mut w);
                } else {
                    flush(&mut w, &mut out);
                }
                r.push(c);
                i += 1;
                while i < cs.len() && (cs[i] == '>' || cs[i] == '<' || cs[i] == '&' || (cs[i].is_ascii_digit() && r.ends_with('&'))) {
                    r.push(cs[i]);
                    i += 1;
                }
                out.push(Tok::Redir(r));
            }
            _ => {
                w.push(c);
                i += 1;
            }
        }
    }
    flush(&mut w, &mut out);
    out
}

/// A word quoted for the shell, when it needs to be (argv shown as a line).
fn shell_quote(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_./:=@%+,".contains(c)) {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

/// The program word: its folder dim, its name bold.
fn program(out: &mut Spans, w: &str) {
    if quoted(w) {
        push(out, Kind::Prog, w);
        return;
    }
    let slash = w.rfind('/').map(|i| i + 1).unwrap_or(0);
    store(out, Kind::Dir, &w[..slash]);
    push(out, Kind::Prog, &w[slash..]);
}

/// The lines: [spans per line].
fn layout(toks: &[Tok]) -> Vec<Spans> {
    let mut lines: Vec<Spans> = Vec::new();
    let mut line = Spans::new();
    // (Each command: assignments, the program, then its arguments.)
    let mut at_start = true;
    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            Tok::Sep(s) => {
                // (Ends the line: the next command starts the next one.)
                if !line.is_empty() {
                    line.push((Kind::Space, " ".into()));
                }
                push(&mut line, Kind::Op, s);
                lines.push(std::mem::take(&mut line));
                at_start = true;
                i += 1;
            }
            Tok::Redir(r) => {
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                line.push((Kind::Space, "    ".into()));
                push(&mut line, Kind::Op, r);
                if let Some(Tok::Word(t)) = toks.get(i + 1) {
                    line.push((Kind::Space, " ".into()));
                    value(&mut line, t);
                    i += 1;
                }
                i += 1;
            }
            Tok::Word(w) => {
                if at_start {
                    // NAME=value before the program: on its line.
                    if w.find('=').is_some_and(|eq| eq > 0 && ident(&w[..eq])) && !quoted(w) {
                        if !line.is_empty() {
                            line.push((Kind::Space, " ".into()));
                        }
                        arg(&mut line, w);
                        i += 1;
                        continue;
                    }
                    if !line.is_empty() {
                        line.push((Kind::Space, " ".into()));
                    }
                    program(&mut line, w);
                    at_start = false;
                    i += 1;
                    continue;
                }
                lines.push(std::mem::take(&mut line));
                line.push((Kind::Space, "    ".into()));
                arg(&mut line, w);
                // "--flag value" together.
                if is_flag(w) && !w.contains('=') {
                    if let Some(Tok::Word(v)) = toks.get(i + 1) {
                        if !v.starts_with('-') {
                            line.push((Kind::Space, " ".into()));
                            value(&mut line, v);
                            i += 1;
                        }
                    }
                }
                i += 1;
            }
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    // (Line continuations, barely there, where a command goes on.)
    let n = lines.len();
    for l in lines.iter_mut().take(n.saturating_sub(1)) {
        // (Not after | && ; and the like: they say it goes on.)
        if l.last().is_some_and(|(k, t)| *k == Kind::Op && matches!(t.as_str(), "|" | "||" | "&&" | ";" | "&" | "(" | ")")) {
            continue;
        }
        l.push((Kind::Space, " ".into()));
        l.push((Kind::Faint, "\\".into()));
    }
    lines
}

/// The command line as Qt rich text over the text color `text`: laid out a
/// line a word when it's long, on one line (colored) when it's short.
/// `argv_json`: tessh's [[program, args...], ...] (a pipeline), else "".
pub fn pretty_html(line: &str, argv_json: &str, dark: bool, text: &str) -> String {
    let line = match serde_json::from_str::<Vec<Vec<String>>>(argv_json) {
        Ok(cmds) if !cmds.is_empty() => cmds.iter().map(|c| c.iter().map(|w| shell_quote(w)).collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join(" | "),
        _ => line.trim().to_string(),
    };
    let toks = tokens(&line);
    let words = toks.iter().filter(|t| matches!(t, Tok::Word(_))).count();
    let short = line.chars().count() <= 60 && words <= 4 && !line.contains('\n');
    let lines = if short {
        // (Short: one line, still colored.)
        let mut l = Spans::new();
        let mut at_start = true;
        for (n, t) in toks.iter().enumerate() {
            if n > 0 {
                l.push((Kind::Space, " ".into()));
            }
            match t {
                Tok::Word(w) if at_start && !(w.find('=').is_some_and(|eq| eq > 0 && ident(&w[..eq]))) => {
                    program(&mut l, w);
                    at_start = false;
                }
                Tok::Word(w) => arg(&mut l, w),
                Tok::Sep(s) => {
                    push(&mut l, Kind::Op, s);
                    at_start = true;
                }
                Tok::Redir(r) => push(&mut l, Kind::Op, r),
            }
        }
        vec![l]
    } else {
        layout(&toks)
    };
    let (r, g, b) = {
        let h = text.trim_start_matches('#');
        let p = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("cc"), 16).unwrap_or(0xcc);
        if h.len() >= 8 { (p(2), p(4), p(6)) } else { (p(0), p(2), p(4)) }
    };
    let esc = |t: &str| t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace(' ', "&nbsp;");
    lines
        .iter()
        .map(|l| {
            l.iter()
                .map(|(k, t)| {
                    if *k == Kind::Space {
                        return t.replace(' ', "&nbsp;");
                    }
                    let (c, a) = k.color(dark);
                    let color = match c {
                        Some(c) => c.to_string(),
                        None if a >= 1.0 => format!("#{r:02x}{g:02x}{b:02x}"),
                        None => format!("#{:02x}{r:02x}{g:02x}{b:02x}", (a * 255.0).round() as u8),
                    };
                    let body = if *k == Kind::Prog { format!("<b>{}</b>", esc(t)) } else { esc(t) };
                    format!("<font color='{color}'>{body}</font>")
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("<br>")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(l: &Spans) -> String {
        l.iter().map(|(_, t)| t.as_str()).collect()
    }

    #[test]
    fn splits_like_a_shell() {
        let t = tokens("FOO=1 grep -r 'a b' \"c\\\"d\" src 2>/dev/null | wc -l && echo done");
        let words: Vec<String> = t.iter().map(|x| match x { Tok::Word(w) | Tok::Sep(w) | Tok::Redir(w) => w.clone() }).collect();
        assert_eq!(words, ["FOO=1", "grep", "-r", "'a b'", "\"c\\\"d\"", "src", "2>", "/dev/null", "|", "wc", "-l", "&&", "echo", "done"]);
    }

    #[test]
    fn lays_out_a_line_a_word() {
        let l = layout(&tokens("/usr/bin/python3 -u --model m.gguf -v x.py | tee out.log"));
        let t: Vec<String> = l.iter().map(text).collect();
        assert_eq!(t, vec!["/usr/bin/python3 \\", "    -u \\", "    --model m.gguf \\", "    -v x.py |", "tee \\", "    out.log"]);
    }

    #[test]
    fn short_stays_one_line() {
        let h = pretty_html("ls -la", "", true, "#dddddd");
        assert!(!h.contains("<br>"), "{h}");
        assert!(h.contains("<b>ls</b>"), "{h}");
    }

    #[test]
    fn argv_is_quoted() {
        let h = pretty_html("", r#"[["echo","a b"],["wc","-w"]]"#, true, "#dddddd");
        assert!(h.contains("'a&nbsp;b'"), "{h}");
        assert!(h.contains("<b>wc</b>"), "{h}");
    }
}
