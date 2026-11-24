//! Markdown in, Markdown out: each ```tessh block becomes HTML coloured by
//! tessh's own highlighting (tree_sitter_tessh::highlight, as Tessera and
//! Grove colour it), for the documentation site (scripts/tessh-docs-site.sh).
//! Spans are `<span class="ts-KIND">`; the site's CSS gives the colours.

use std::io::{Read, Write};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// tessh code as `<pre>` HTML, a span a highlighted run.
fn html(code: &str) -> String {
    // (Later spans over earlier, as Tessera has them.)
    let mut kind: Vec<Option<&'static str>> = vec![None; code.len()];
    for (s, e, k) in tree_sitter_tessh::highlight::spans(code) {
        for b in &mut kind[s.min(code.len())..e.min(code.len())] {
            *b = Some(k);
        }
    }
    let mut out = String::from("<pre class=\"tessh\"><code class=\"language-tessh hljs\">");
    let mut s = 0;
    while s < code.len() {
        let k = kind[s];
        let mut e = s + 1;
        while e < code.len() && kind[e] == k {
            e += 1;
        }
        while !code.is_char_boundary(e) {
            e += 1;
        }
        match k {
            Some(k) => out.push_str(&format!("<span class=\"ts-{k}\">{}</span>", esc(&code[s..e]))),
            None => out.push_str(&esc(&code[s..e])),
        }
        s = e;
    }
    out.push_str("</code></pre>");
    out
}

fn main() {
    let mut md = String::new();
    std::io::stdin().read_to_string(&mut md).expect("markdown on stdin");
    let mut out = String::with_capacity(md.len() * 2);
    let mut lines = md.split_inclusive('\n');
    while let Some(line) = lines.next() {
        if line.trim_end() == "```tessh" {
            let mut code = String::new();
            for l in lines.by_ref() {
                if l.trim_end() == "```" {
                    break;
                }
                code.push_str(l);
            }
            // (Its own lines: a <pre> HTML block runs to </pre>, blank lines and all.)
            out.push_str(&html(code.strip_suffix('\n').unwrap_or(&code)));
            out.push('\n');
        } else {
            out.push_str(line);
        }
    }
    std::io::stdout().write_all(out.as_bytes()).expect("stdout");
}
