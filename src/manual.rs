//! A guide's Markdown as a page (the tessh guide's and the languages'
//! guides', guides.rs): contents, headings with ids, code highlighted.

/// Fence names as the editor's languages ("" for none to draw).
pub fn code_lang(info: &str, default: &str) -> String {
    let l = info.split([' ', ',', '{']).next().unwrap_or("").trim().to_lowercase();
    // (Sites' class names: "language-rust", "mw-highlight-lang-cpp",
    // "highlight-python3", "source-cpp": the name after the last prefix.)
    let l = ["language-", "lang-", "highlight-", "source-"].iter().fold(l.as_str(), |l, p| l.rsplit_once(p).map(|(_, x)| x).unwrap_or(l)).to_string();
    let l = l.as_str();
    let l = match l {
        "" => default,
        "py" | "python3" | "py3" | "pycon" | "ipython" | "ipython3" | "pytb" => "python",
        "sh" | "shell" | "console" | "shell-session" | "shellsession" | "zsh" | "bash-session" | "shell-script" => "bash",
        "js" | "node" | "mjs" | "javascript-nolint" => "javascript",
        "ts" => "typescript",
        "rs" => "rust",
        "c++" | "cxx" | "cc" => "cpp",
        "golang" => "go",
        "rb" => "ruby",
        "yml" => "yaml",
        "jsonc" | "json5" => "json",
        "md" => "markdown",
        "ps1" | "pwsh" | "posh" => "powershell",
        "cs" | "csharp" => "c-sharp",
        "fs" | "f#" => "fsharp",
        "kt" | "kts" => "kotlin",
        "ex" | "exs" | "iex" => "elixir",
        "erl" => "erlang",
        "hs" => "haskell",
        "ml" => "ocaml",
        "clj" | "cljs" => "clojure",
        "lisp" | "common-lisp" | "cl" => "common-lisp",
        "jl" | "julia-repl" | "jldoctest" => "julia",
        // (Helix's names for these grammars; Racket's code drawn as Scheme.)
        "janet" => "janet-simple",
        "octave" => "matlab",
        "racket" | "rkt" => "scheme",
        "pl" | "perl5" => "perl",
        "text" | "txt" | "plain" | "plaintext" | "output" | "none" | "nohighlight" | "default" | "raw" => "",
        l => l,
    };
    // (Only languages the editor can draw: anything else would be drawn as
    // a shell command.)
    if crate::syntax::is_builtin(l) || crate::grammars::is_known(l) { l.to_string() } else { String::new() }
}

/// Raw HTML without what could run: scripts, frames, objects, event
/// handlers and javascript: links.
fn safe_html(h: &str) -> String {
    use std::sync::OnceLock;
    static RES: OnceLock<[regex::Regex; 4]> = OnceLock::new();
    let [bodies, tags, attrs, js] = RES.get_or_init(|| [
        regex::Regex::new(r"(?is)<script\b.*?</script\s*>|<style\b.*?</style\s*>").unwrap(),
        regex::Regex::new(r"(?i)</?(script|iframe|object|embed|frame|frameset|style|link|meta|base|form)\b[^>]*>").unwrap(),
        regex::Regex::new(r#"(?i)\son[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#).unwrap(),
        regex::Regex::new(r#"(?i)(href|src|action)\s*=\s*(["']?)\s*javascript:"#).unwrap(),
    ]);
    let h = bodies.replace_all(h, "");
    let h = tags.replace_all(&h, "");
    let h = attrs.replace_all(&h, "");
    js.replace_all(&h, "$1=$2#").into_owned()
}

/// GitHub's heading ids: lower case, spaces as dashes, punctuation gone.
fn slug(t: &str) -> String {
    t.trim().to_lowercase().chars().filter_map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { Some(c) } else if c == ' ' { Some('-') } else { None }).collect()
}

/// A guide's Markdown as a page: a contents list (`<nav>`, the parts at
/// the level the document is divided at, their topics under them) and the
/// document (`<main>`), headings with ids, code highlighted (tessh examples
/// as `pre.tessh`, their output as `pre.output`).
pub fn manual_html(md: &str, default_code: &str) -> String {
    use pulldown_cmark::{html, CodeBlockKind, CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    // (Heading texts first, for ids and the contents.)
    let mut heads: Vec<(HeadingLevel, String)> = vec![];
    let mut cur: Option<(HeadingLevel, String)> = None;
    for ev in Parser::new_ext(md, opts) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => cur = Some((level, String::new())),
            Event::Text(t) | Event::Code(t) => {
                if let Some((_, s)) = &mut cur {
                    s.push_str(&t);
                }
            }
            Event::End(TagEnd::Heading(_)) => heads.extend(cur.take()),
            _ => {}
        }
    }
    let mut ids: Vec<String> = vec![];
    let mut seen = std::collections::HashSet::new();
    for (k, (_, t)) in heads.iter().enumerate() {
        let mut id = slug(t);
        if id.is_empty() || !seen.insert(id.clone()) {
            id = format!("{}-{k}", if id.is_empty() { "h" } else { &id });
            seen.insert(id.clone());
        }
        ids.push(id);
    }
    // (The parts: the shallowest level there's more than one of.)
    let count = |l: HeadingLevel| heads.iter().filter(|(x, _)| *x == l).count();
    let levels = [HeadingLevel::H1, HeadingLevel::H2, HeadingLevel::H3, HeadingLevel::H4];
    let top = levels.iter().copied().find(|&l| count(l) >= 2).unwrap_or(HeadingLevel::H1);
    let topic = levels.iter().copied().find(|&l| l > top).unwrap_or(HeadingLevel::H6);
    let esc = |t: &str| t.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let mut nav = String::from("<nav><div class='title'>Contents</div>");
    let mut open = false;
    let mut intro = false;
    for (k, (level, t)) in heads.iter().enumerate() {
        let id = &ids[k];
        if *level < top || (*level == top && k == 0 && heads.len() > 1 && heads[0].0 == HeadingLevel::H1 && count(HeadingLevel::H1) == 1) {
            if !intro {
                nav.push_str(&format!("<a class='part' href='#{id}'>Introduction</a>"));
                intro = true;
            }
        } else if *level == top {
            if open {
                nav.push_str("</div>");
            }
            nav.push_str(&format!("<a class='part' href='#{id}'>{}</a><div class='topics'>", esc(t)));
            open = true;
        } else if *level == topic {
            nav.push_str(&format!("<a href='#{id}'>{}</a>", esc(t)));
        }
    }
    if open {
        nav.push_str("</div>");
    }
    nav.push_str("</nav>");
    // The document, with ids on headings and its code drawn.
    let mut events: Vec<Event> = vec![];
    let mut code: Option<(String, String)> = None; // (fence info, text)
    let mut h = 0;
    for ev in Parser::new_ext(md, opts) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                let id = ids.get(h).cloned().unwrap_or_default();
                h += 1;
                events.push(Event::Html(CowStr::from(format!("<{level} id='{id}'>"))));
            }
            Event::End(TagEnd::Heading(level)) => events.push(Event::Html(CowStr::from(format!("</{level}>")))),
            Event::Start(Tag::CodeBlock(kind)) => {
                let info = match kind {
                    CodeBlockKind::Fenced(i) => i.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                code = Some((info, String::new()));
            }
            Event::Text(t) if code.is_some() => code.as_mut().unwrap().1.push_str(&t),
            Event::End(TagEnd::CodeBlock) => {
                let (info, text) = code.take().unwrap_or_default();
                let text = text.trim_end_matches('\n');
                let first = info.split_whitespace().next().unwrap_or("");
                let block = match first {
                    "tessh" => format!("<pre class='tessh'><code>{}</code></pre>", crate::syntax::highlight_html("tessh", text)),
                    "output" => format!("<pre class='output'><code>{}</code></pre>", esc(text)),
                    _ => match code_lang(first, default_code) {
                        l if l.is_empty() => format!("<pre><code>{}</code></pre>", esc(text)),
                        l => format!("<pre class='code'><code>{}</code></pre>", crate::syntax::highlight_html(&l, text)),
                    },
                };
                events.push(Event::Html(CowStr::from(block)));
            }
            // (HTML in the Markdown, from the documents' own sources: no
            // scripts or frames, no handlers.)
            Event::Html(h) => events.push(Event::Html(CowStr::from(safe_html(&h)))),
            Event::InlineHtml(h) => events.push(Event::InlineHtml(CowStr::from(safe_html(&h)))),
            ev => events.push(ev),
        }
    }
    let mut body = String::new();
    html::push_html(&mut body, events.into_iter());
    format!("{nav}<main>{body}</main>")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_class_names() {
        assert_eq!(code_lang("mw-highlight-lang-cpp", ""), "cpp");
        assert_eq!(code_lang("language-rust", ""), "rust");
        assert_eq!(code_lang("highlight-python3", ""), "python");
        assert_eq!(code_lang("", "lua"), "lua");
        assert_eq!(code_lang("text", "lua"), "");
    }

    #[test]
    fn raw_html_made_safe() {
        let m = manual_html("<p onclick=\"x()\">a</p>\n\n<script>alert(1)</script>\n\nb <a href=\"javascript:x()\">c</a> <img src=x onerror=y>\n", "");
        assert!(!m.contains("script") && !m.contains("onclick") && !m.contains("onerror") && !m.contains("javascript:"), "{m}");
    }

    #[test]
    fn page_shape() {
        let m = manual_html("# Title\n\nIntro\n\n## One\n\n### a\n\n```python\nx = 1\n```\n\n## Two\n\n```\nprint(2)\n```\n", "python");
        assert!(m.starts_with("<nav>") && m.contains("<a class='part' href='#one'>One</a>") && m.contains("href='#a'"), "{m}");
        assert!(m.contains("<h2 id='two'>") && m.matches("<span class=\"t-").count() >= 2, "{m}");
    }

}

