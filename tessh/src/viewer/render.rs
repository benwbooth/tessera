//! Each kind of file laid out for the pager: its views (the first shown,
//! Tab for the next), its lines, what the foot says about it.

use super::{highlight, human_size, Kind};
use crate::editor::pager::{Content, Guide, Lines};
use std::io::Read;
use std::path::Path;

/// The most of a text that's read (the rest isn't shown, and the foot says so).
pub const TEXT_MAX: usize = 32 << 20;
/// The most that's highlighted (bigger: plain, quickly).
pub const HIGHLIGHT_MAX: usize = 4 << 20;
/// The most of a binary that's shown as hex.
pub const HEX_MAX: usize = 1 << 20;

/// CSV's delimiter, as Tessera's tables.rs picks it: a .tsv's tab, else
/// what the first line has most of.
pub fn delimiter(name: &str, first: &[u8]) -> u8 {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".tsv") || lower.ends_with(".tab") {
        return b'\t';
    }
    let line = first.split(|&b| b == b'\n').next().unwrap_or(&[]);
    let count = |c: u8| line.iter().filter(|&&b| b == c).count();
    [b',', b'\t', b';', b'|'].into_iter().max_by_key(|&c| count(c)).filter(|&c| count(c) > 0).unwrap_or(b',')
}

/// Up to `max` bytes of a file, and whether there was more.
fn read_some(p: &Path, max: usize) -> Result<(Vec<u8>, bool), String> {
    let f = std::fs::File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut buf = vec![];
    f.take(max as u64 + 1).read_to_end(&mut buf).map_err(|e| format!("{}: {e}", p.display()))?;
    let more = buf.len() > max;
    buf.truncate(max);
    Ok((buf, more))
}

/// Bytes as text: UTF-16 by its mark, else UTF-8 (a bad byte: �).
pub fn decode(b: &[u8]) -> String {
    let utf16 = |be: bool| {
        let units: Vec<u16> = b[2..].chunks_exact(2).map(|c| if be { u16::from_be_bytes([c[0], c[1]]) } else { u16::from_le_bytes([c[0], c[1]]) }).collect();
        String::from_utf16_lossy(&units)
    };
    if b.starts_with(&[0xff, 0xfe]) {
        utf16(false)
    } else if b.starts_with(&[0xfe, 0xff]) {
        utf16(true)
    } else {
        String::from_utf8_lossy(b.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(b)).into_owned()
    }
}

/// A text file's text (up to TEXT_MAX).
pub fn read_text(p: &Path) -> Result<String, String> {
    Ok(decode(&read_some(p, TEXT_MAX)?.0))
}

/// Text made safe to show: tabs to the next 4 columns, CRLF's CR dropped,
/// any other control char as its picture (␛, ␇…), but SGR colors a file
/// has (ESC [ … m, a colored log) kept: they're only colors.
pub fn sanitize(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut col = 0;
    let mut it = t.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\n' => {
                out.push('\n');
                col = 0;
            }
            '\t' => {
                let n = 4 - col % 4;
                out.push_str(&" ".repeat(n));
                col += n;
            }
            '\r' if it.peek() == Some(&'\n') => {}
            '\x1b' if it.peek() == Some(&'[') => {
                // (SGR kept, any other sequence shown as text.)
                let rest: String = it.clone().skip(1).take_while(|x| x.is_ascii_digit() || *x == ';' || *x == ':').collect();
                if it.clone().nth(1 + rest.len()) == Some('m') {
                    out.push_str("\x1b[");
                    out.push_str(&rest);
                    out.push('m');
                    for _ in 0..rest.len() + 2 {
                        it.next();
                    }
                } else {
                    out.push('␛');
                    col += 1;
                }
            }
            c if (c as u32) < 0x20 => {
                out.push(char::from_u32(0x2400 + c as u32).unwrap_or('?'));
                col += 1;
            }
            '\x7f' => {
                out.push('␡');
                col += 1;
            }
            c if ('\u{80}'..='\u{9f}').contains(&c) => {
                out.push('�');
                col += 1;
            }
            c => {
                out.push(c);
                col += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            }
        }
    }
    out
}

/// Lines with a line-number gutter ("  12 │ ").
fn numbered(lines: Vec<String>, wrap: bool) -> Lines {
    let mut lines = lines;
    // (A final newline isn't a line of its own.)
    if lines.len() > 1 && lines.last().is_some_and(|l| crate::editor::pager::strip(l).is_empty()) {
        lines.pop();
    }
    let w = lines.len().to_string().len().max(3);
    let lines = lines.into_iter().enumerate().map(|(i, l)| format!("\x1b[2m{:>w$} │\x1b[22m {l}\x1b[0m", i + 1)).collect();
    Lines { lines, gutter: w + 3, wrap, parts: vec![] }
}

/// Text highlighted for `path`'s language (or as `as_name` says), numbered.
fn code(path: &Path, text: &str, as_name: Option<&str>) -> (Lines, String) {
    let clean = sanitize(text);
    let colored_already = clean.contains('\x1b');
    let (lines, lang) = if clean.len() > HIGHLIGHT_MAX || colored_already {
        (clean.split('\n').map(String::from).collect(), String::new())
    } else {
        let p = as_name.map(|n| path.with_file_name(n)).unwrap_or_else(|| path.to_path_buf());
        highlight::highlight_file(&p, &clean)
    };
    (numbered(lines, true), lang)
}

/// "name · kind · 120 lines · 4.2 KB", what the foot starts with.
fn info(p: &Path, kind: &str, extra: &str) -> String {
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.display().to_string());
    let size = std::fs::metadata(p).map(|m| human_size(m.len())).unwrap_or_default();
    let mut s = format!("\x1b[1m{name}\x1b[22m");
    for part in [kind, extra, &size] {
        if !part.is_empty() {
            s.push_str(" · ");
            s.push_str(part);
        }
    }
    s
}

fn guide(p: &Path, views: Vec<(&str, Content)>, info: String) -> Guide {
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Guide::views(views.into_iter().map(|(t, c)| (t.to_string(), c)).collect(), 0, &name, &info).with_file(p)
}

/// The pager for a file of `kind`.
pub fn render(p: &Path, kind: Kind) -> Result<Guide, String> {
    match kind {
        Kind::Text => {
            let (b, more) = read_some(p, TEXT_MAX)?;
            let t = decode(&b);
            let (l, lang) = code(p, &t, None);
            let n = format!("{} lines{}", l.lines.len(), if more { " (the first 32 MB)" } else { "" });
            Ok(guide(p, vec![("text", Content::Lines(l))], info(p, if lang.is_empty() { "text" } else { &lang }, &n)))
        }
        Kind::Markdown => {
            let t = read_text(p)?;
            let (src, _) = code(p, &t, None);
            let n = format!("{} lines", src.lines.len());
            Ok(guide(p, vec![("rendered", Content::Markdown(sanitize(&t))), ("source", Content::Lines(src))], info(p, "Markdown", &n)))
        }
        Kind::Json => {
            let t = read_text(p)?;
            let (src, _) = code(p, &t, Some("x.json"));
            match serde_json::from_str::<serde::de::IgnoredAny>(&t) {
                Ok(_) => {
                    let pretty = pretty_json(&t);
                    let (pl, _) = code(p, &pretty, Some("x.json"));
                    let n = format!("{} lines", pl.lines.len());
                    Ok(guide(p, vec![("formatted", Content::Lines(pl)), ("as written", Content::Lines(src))], info(p, "JSON", &n)))
                }
                Err(e) => {
                    // (JSON lines, or not quite JSON: as it is, and why.)
                    let lines_ok = t.lines().filter(|l| !l.trim().is_empty()).all(|l| serde_json::from_str::<serde::de::IgnoredAny>(l).is_ok());
                    let note = if lines_ok { "JSON lines".to_string() } else { format!("JSON, \x1b[33m⚠ {e}\x1b[39m") };
                    Ok(guide(p, vec![("as written", Content::Lines(src))], info(p, &note, "")))
                }
            }
        }
        Kind::Yaml | Kind::Toml => {
            let t = read_text(p)?;
            let (yaml, ext) = (kind == Kind::Yaml, if kind == Kind::Yaml { "x.yaml" } else { "x.toml" });
            let (src, _) = code(p, &t, Some(ext));
            let formatted: Result<String, String> = if yaml {
                // (Every document in the file, each formatted.)
                let mut out = vec![];
                let mut err = None;
                for d in serde_yaml::Deserializer::from_str(&t) {
                    match <serde_yaml::Value as serde::Deserialize>::deserialize(d).map_err(|e| e.to_string()) {
                        Ok(v) => out.push(serde_yaml::to_string(&v).unwrap_or_default().trim_start_matches("---\n").to_string()),
                        Err(e) => {
                            err = Some(e);
                            break;
                        }
                    }
                }
                match err {
                    Some(e) => Err(e),
                    None => Ok(out.join("---\n")),
                }
            } else {
                t.parse::<toml::Table>().map_err(|e| e.to_string().lines().next().unwrap_or("").to_string()).map(|v| toml::to_string_pretty(&v).unwrap_or_default())
            };
            let label = if yaml { "YAML" } else { "TOML" };
            let n = format!("{} lines", src.lines.len());
            match formatted {
                Ok(f) => {
                    let (fl, _) = code(p, &f, Some(ext));
                    Ok(guide(p, vec![("as written", Content::Lines(src)), ("formatted", Content::Lines(fl))], info(p, label, &n)))
                }
                Err(e) => Ok(guide(p, vec![("as written", Content::Lines(src))], info(p, &format!("{label}, \x1b[33m⚠ {e}\x1b[39m"), &n))),
            }
        }
        Kind::Table(d) => {
            let t = read_text(p)?;
            let (src, _) = code(p, &t, Some(if d == b'\t' { "x.tsv" } else { "x.csv" }));
            let (table, rows, cols) = table(&t, d);
            let label = if d == b'\t' { "TSV" } else { "CSV" };
            Ok(guide(p, vec![("table", Content::Lines(table)), ("as written", Content::Lines(src))], info(p, label, &format!("{rows} rows × {cols} columns"))))
        }
        Kind::Pdf => {
            let (text, pages) = pdf_text(p)?;
            Ok(guide(p, vec![("text", Content::Lines(text))], info(p, "PDF", &format!("{pages} pages, its text"))))
        }
        Kind::Archive => {
            let (l, n) = archive(p)?;
            Ok(guide(p, vec![("contents", Content::Lines(l))], info(p, "archive", &n)))
        }
        Kind::Compressed => {
            // (What's inside, shown as what its name without .gz says it is.)
            let inner = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let b = decompress(p, TEXT_MAX)?;
            if super::is_text(&b[..b.len().min(8192)]) {
                let t = decode(&b);
                let (l, lang) = code(p, &t, Some(&inner));
                let n = format!("{} lines, uncompressed", l.lines.len());
                Ok(guide(p, vec![("text", Content::Lines(l))], info(p, if lang.is_empty() { "text" } else { &lang }, &n)))
            } else {
                let l = hex(&b[..b.len().min(HEX_MAX)]);
                Ok(guide(p, vec![("hex", Content::Lines(l))], info(p, "compressed", &format!("{} uncompressed{}", human_size(b.len() as u64), if b.len() > HEX_MAX { ", the first 1 MB as hex" } else { "" }))))
            }
        }
        Kind::Outside | Kind::Binary | Kind::Image => {
            let (b, more) = read_some(p, HEX_MAX)?;
            let mut l = hex(&b);
            let what = if kind == Kind::Outside { "its app shows it (o)" } else { "binary" };
            if more {
                l.lines.push(format!("{}\x1b[2m… the first 1 MB of {}\x1b[22m", " ".repeat(l.gutter), human_size(std::fs::metadata(p).map(|m| m.len()).unwrap_or(0))));
            }
            Ok(guide(p, vec![("hex", Content::Lines(l))], info(p, what, "")))
        }
        Kind::Dir => Err(format!("{}: a folder", p.display())),
    }
}

/// JSON laid out with two-space indents, keys in their order, numbers as
/// written (reindented token by token, not parsed and printed again).
pub fn pretty_json(t: &str) -> String {
    let mut out = String::with_capacity(t.len() * 2);
    let mut depth = 0usize;
    let mut chars = t.chars().peekable();
    let nl = |out: &mut String, d: usize| {
        out.push('\n');
        out.push_str(&"  ".repeat(d));
    };
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push('"');
                while let Some(x) = chars.next() {
                    out.push(x);
                    if x == '\\' {
                        if let Some(y) = chars.next() {
                            out.push(y);
                        }
                    } else if x == '"' {
                        break;
                    }
                }
            }
            '{' | '[' => {
                // (Empty: {} and [] as they are.)
                while chars.peek().is_some_and(|x| x.is_whitespace()) {
                    chars.next();
                }
                let close = if c == '{' { '}' } else { ']' };
                if chars.peek() == Some(&close) {
                    chars.next();
                    out.push(c);
                    out.push(close);
                } else {
                    out.push(c);
                    depth += 1;
                    nl(&mut out, depth);
                }
            }
            '}' | ']' => {
                depth = depth.saturating_sub(1);
                nl(&mut out, depth);
                out.push(c);
            }
            ',' => {
                out.push(',');
                nl(&mut out, depth);
            }
            ':' => out.push_str(": "),
            c if c.is_whitespace() => {}
            c => out.push(c),
        }
    }
    out.push('\n');
    out
}

/// A CSV as a table: columns as wide as their widest cell (up to 40, more
/// cut with …), numbers to the right, the header bold, a row number gutter.
/// The lines, the rows and the columns.
fn table(t: &str, d: u8) -> (Lines, usize, usize) {
    let mut rd = csv::ReaderBuilder::new().delimiter(d).has_headers(false).flexible(true).from_reader(t.as_bytes());
    let rows: Vec<Vec<String>> = rd.records().filter_map(|r| r.ok()).take(500_000).map(|r| r.iter().map(|c| sanitize(c).replace('\n', "⏎")).collect()).collect();
    let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let width = |s: &str| unicode_width::UnicodeWidthStr::width(s);
    let mut widths = vec![0usize; ncols];
    let mut numeric = vec![true; ncols];
    for (i, r) in rows.iter().enumerate() {
        for (c, cell) in r.iter().enumerate() {
            widths[c] = widths[c].max(width(cell).min(40));
            if i > 0 && !cell.is_empty() && cell.trim().parse::<f64>().is_err() {
                numeric[c] = false;
            }
        }
    }
    let gw = rows.len().to_string().len().max(3);
    let mut lines = vec![];
    for (i, r) in rows.iter().enumerate() {
        let mut s = if i == 0 { format!("\x1b[2m{:>gw$} │\x1b[22m ", "") } else { format!("\x1b[2m{i:>gw$} │\x1b[22m ") };
        for c in 0..ncols {
            let cell = r.get(c).map(String::as_str).unwrap_or("");
            let shown = if width(cell) > 40 { format!("{}…", crate::editor::pager::cut(cell, 39)) } else { cell.to_string() };
            let pad = " ".repeat(widths[c].saturating_sub(width(&shown)));
            let body = if numeric[c] && i > 0 { format!("{pad}{shown}") } else { format!("{shown}{pad}") };
            if i == 0 {
                s.push_str(&format!("\x1b[1;4m{body}\x1b[0m"));
            } else {
                s.push_str(&body);
            }
            if c + 1 < ncols {
                s.push_str(" \x1b[2m│\x1b[22m ");
            }
        }
        lines.push(s);
    }
    (Lines { lines, gutter: gw + 3, wrap: false, parts: vec![] }, rows.len().saturating_sub(1), ncols)
}

/// A PDF's text: poppler's pdftotext (its layout kept) when it's installed,
/// else read here (pdf-extract). A part for each page.
fn pdf_text(p: &Path) -> Result<(Lines, usize), String> {
    let via_tool = std::process::Command::new("pdftotext").arg("-layout").arg("-enc").arg("UTF-8").arg(p).arg("-").output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
    let text = match via_tool {
        Some(t) => t,
        None => {
            let bytes = std::fs::read(p).map_err(|e| format!("{}: {e}", p.display()))?;
            // (A PDF it can't read can panic it: caught, it's an error.)
            let hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(|_| {}));
            let r = std::panic::catch_unwind(|| pdf_extract::extract_text_from_mem_by_pages(&bytes));
            std::panic::set_hook(hook);
            match r {
                Ok(Ok(pages)) => pages.join("\x0c"),
                Ok(Err(e)) => return Err(format!("{}: {e}", p.display())),
                Err(_) => return Err(format!("{}: this PDF couldn't be read", p.display())),
            }
        }
    };
    let pages: Vec<&str> = text.trim_end_matches('\x0c').split('\x0c').collect();
    let mut all = vec![];
    let mut parts = vec![];
    for (i, page) in pages.iter().enumerate() {
        parts.push((1, format!("Page {}", i + 1), all.len()));
        all.push(format!("\x1b[2m── page {} ──\x1b[22m", i + 1));
        let clean = sanitize(page);
        all.extend(clean.trim_end().split('\n').map(String::from));
        all.push(String::new());
    }
    Ok((Lines { lines: all, gutter: 0, wrap: true, parts }, pages.len()))
}

/// A date (seconds since 1970, UTC) as 2026-11-23 14:05.
fn date(secs: i64) -> String {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // (Howard Hinnant's days-to-civil.)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rem / 3600, rem % 3600 / 60)
}

/// The decompressing reader for a gzip, bzip2 or zstd file (xz: None, it's
/// read whole).
fn decoder(p: &Path) -> Result<Option<Box<dyn Read>>, String> {
    let f = std::fs::File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
    let b = super::head(p);
    let f = std::io::BufReader::new(f);
    Ok(if b.starts_with(&[0x1f, 0x8b]) {
        Some(Box::new(flate2::bufread::MultiGzDecoder::new(f)))
    } else if b.starts_with(b"BZh") {
        Some(Box::new(bzip2::bufread::MultiBzDecoder::new(f)))
    } else if b.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        Some(Box::new(ruzstd::decoding::StreamingDecoder::new(f).map_err(|e| format!("{}: {e}", p.display()))?))
    } else if b.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0]) {
        None
    } else {
        Some(Box::new(f))
    })
}

/// Up to `max` bytes of what a compressed file holds.
fn decompress(p: &Path, max: usize) -> Result<Vec<u8>, String> {
    match decoder(p)? {
        Some(r) => {
            let mut out = vec![];
            r.take(max as u64).read_to_end(&mut out).map_err(|e| format!("{}: {e}", p.display()))?;
            Ok(out)
        }
        None => {
            // (xz: lzma-rs writes it all; stopped at `max` by a writer that says "full".)
            struct Cap(Vec<u8>, usize);
            impl std::io::Write for Cap {
                fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                    if self.0.len() >= self.1 {
                        return Err(std::io::Error::other("full"));
                    }
                    let n = b.len().min(self.1 - self.0.len());
                    self.0.extend_from_slice(&b[..n]);
                    Ok(n)
                }
                fn flush(&mut self) -> std::io::Result<()> {
                    Ok(())
                }
            }
            let f = std::fs::File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let mut cap = Cap(vec![], max);
            let r = lzma_rs::xz_decompress(&mut std::io::BufReader::new(f), &mut cap);
            if cap.0.is_empty() {
                r.map_err(|e| format!("{}: {e:?}", p.display()))?;
            }
            Ok(cap.0)
        }
    }
}

/// The permission string ls shows (drwxr-xr-x).
fn mode_string(mode: u32, dir: bool, link: bool) -> String {
    let mut s = String::from(if dir { "d" } else if link { "l" } else { "-" });
    for shift in [6, 3, 0] {
        let m = (mode >> shift) & 7;
        s.push(if m & 4 != 0 { 'r' } else { '-' });
        s.push(if m & 2 != 0 { 'w' } else { '-' });
        s.push(if m & 1 != 0 { 'x' } else { '-' });
    }
    s
}

/// An archive's list: one line per entry (permissions, size, date, name;
/// folders blue as ls has them), and a summary for the foot. zip and tar
/// (gzip, bzip2, xz, zstd) read here; others listed by bsdtar or 7z.
fn archive(p: &Path) -> Result<(Lines, String), String> {
    struct E {
        name: String,
        size: u64,
        time: Option<String>,
        mode: Option<u32>,
        dir: bool,
        link: bool,
    }
    let b = super::head(p);
    let mut entries: Vec<E> = vec![];
    let mut kind = "";
    if b.starts_with(b"PK") {
        kind = "zip";
        let f = std::fs::File::open(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let mut z = zip::ZipArchive::new(f).map_err(|e| format!("{}: {e}", p.display()))?;
        for i in 0..z.len() {
            let Ok(e) = z.by_index_raw(i) else { continue };
            let time = e.last_modified().map(|t| format!("{:04}-{:02}-{:02} {:02}:{:02}", t.year(), t.month(), t.day(), t.hour(), t.minute()));
            entries.push(E { name: e.name().map(|n| n.to_string()).unwrap_or_default(), size: e.size(), time, mode: e.unix_mode(), dir: e.is_dir(), link: e.is_symlink() });
        }
    } else if b.get(257..262) == Some(b"ustar") || [&[0x1f, 0x8b][..], b"BZh", &[0x28, 0xb5, 0x2f, 0xfd], &[0xfd, b'7', b'z', b'X', b'Z', 0]].iter().any(|m| b.starts_with(m)) || super::ext_of(p) == "tar" {
        kind = "tar";
        let r: Box<dyn Read> = match decoder(p)? {
            Some(r) => r,
            None => Box::new(std::io::Cursor::new(decompress(p, 1 << 30)?)),
        };
        let mut a = tar::Archive::new(r);
        let it = a.entries().map_err(|e| format!("{}: {e}", p.display()))?;
        for e in it {
            let Ok(e) = e else { break };
            let h = e.header();
            let t = h.entry_type();
            // (pax and GNU long-name records describe the entries, they aren't any.)
            if t.is_pax_global_extensions() || t.is_pax_local_extensions() || t.is_gnu_longname() || t.is_gnu_longlink() {
                continue;
            }
            entries.push(E {
                name: e.path().map(|x| x.to_string_lossy().to_string()).unwrap_or_default(),
                size: h.size().unwrap_or(0),
                time: h.mtime().ok().map(|s| date(s as i64)),
                mode: h.mode().ok(),
                dir: t.is_dir(),
                link: t.is_symlink(),
            });
        }
    }
    if kind.is_empty() {
        // (7z, rar, iso, deb, cab…: a tool that knows them, its list as it gives it.)
        for (prog, args) in [("bsdtar", vec!["-tvf"]), ("7z", vec!["l"]), ("7zz", vec!["l"]), ("unrar", vec!["l"])] {
            if let Ok(o) = std::process::Command::new(prog).args(&args).arg(p).output() {
                if o.status.success() {
                    let t = sanitize(&String::from_utf8_lossy(&o.stdout));
                    let lines: Vec<String> = t.trim_end().split('\n').map(String::from).collect();
                    let n = format!("listed by {prog}");
                    return Ok((Lines { lines, gutter: 0, wrap: false, parts: vec![] }, n));
                }
            }
        }
        return Err(format!("{}: an archive tessh can't list (install bsdtar or 7z)", p.display()));
    }
    let total: u64 = entries.iter().map(|e| e.size).sum();
    let sw = entries.iter().map(|e| e.size.to_string().len()).max().unwrap_or(1);
    let mut lines = vec![];
    let mut parts = vec![];
    for e in &entries {
        let mode = mode_string(e.mode.unwrap_or(if e.dir { 0o755 } else { 0o644 }), e.dir, e.link);
        let name = sanitize(&e.name);
        let top = !name.trim_end_matches('/').contains('/');
        if e.dir && top {
            parts.push((1, name.trim_end_matches('/').to_string(), lines.len()));
        }
        let shown = if e.dir { format!("\x1b[1;34m{}\x1b[0m", if name.ends_with('/') { name.clone() } else { format!("{name}/") }) } else if e.link { format!("\x1b[36m{name}\x1b[0m") } else { name };
        lines.push(format!("\x1b[2m{mode}\x1b[22m  {:>sw$}  \x1b[2m{}\x1b[22m  {shown}", e.size, e.time.clone().unwrap_or_else(|| " ".repeat(16))));
    }
    let files = entries.iter().filter(|e| !e.dir).count();
    Ok((Lines { lines, gutter: 0, wrap: false, parts }, format!("{kind}, {files} files, {} uncompressed", human_size(total))))
}

/// Bytes as a hex dump, 16 a line: the offset (the gutter), the bytes in
/// two groups of eight, the characters. Colored as hexyl colors them: NUL
/// dim, printable ASCII cyan, whitespace green, other ASCII magenta, the
/// rest yellow.
fn hex(b: &[u8]) -> Lines {
    let color = |x: u8| match x {
        0 => "\x1b[2m",
        b' ' | b'\t' | b'\n' | b'\r' => "\x1b[32m",
        0x21..=0x7e => "\x1b[36m",
        0x01..=0x1f | 0x7f => "\x1b[35m",
        _ => "\x1b[33m",
    };
    let ch = |x: u8| match x {
        0 => '⋄',
        0x21..=0x7e => x as char,
        b' ' => ' ',
        b'\t' | b'\n' | b'\r' => '_',
        0x01..=0x1f | 0x7f => '•',
        _ => '×',
    };
    let mut lines = Vec::with_capacity(b.len() / 16 + 1);
    for (i, row) in b.chunks(16).enumerate() {
        let mut s = format!("\x1b[2m{:08x} │\x1b[22m ", i * 16);
        for k in 0..16 {
            if k == 8 {
                s.push(' ');
            }
            match row.get(k) {
                Some(&x) => s.push_str(&format!("{}{x:02x}\x1b[0m ", color(x))),
                None => s.push_str("   "),
            }
        }
        s.push_str("\x1b[2m│\x1b[22m ");
        for &x in row {
            s.push_str(color(x));
            s.push(ch(x));
            s.push_str("\x1b[0m");
        }
        lines.push(s);
    }
    if lines.is_empty() {
        lines.push("\x1b[2m(empty)\x1b[22m".into());
    }
    Lines { lines, gutter: 11, wrap: false, parts: vec![] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::pager::strip;

    #[test]
    fn json_is_reindented_in_its_order() {
        assert_eq!(pretty_json(r#"{"b":1,"a":[1,2],"e":{},"s":"x,{y}"}"#), "{\n  \"b\": 1,\n  \"a\": [\n    1,\n    2\n  ],\n  \"e\": {},\n  \"s\": \"x,{y}\"\n}\n");
    }

    #[test]
    fn control_chars_are_shown_not_sent() {
        assert_eq!(sanitize("a\tb\x1b]0;x\x07\r\n"), "a   b␛]0;x␇\n");
        assert_eq!(sanitize("\x1b[31mred\x1b[0m"), "\x1b[31mred\x1b[0m");
    }

    #[test]
    fn a_csv_is_a_table() {
        let (l, rows, cols) = table("name,n\nab,3\nc,10\n", b',');
        assert_eq!((rows, cols), (2, 2));
        let plain: Vec<String> = l.lines.iter().map(|x| strip(x)).collect();
        assert_eq!(plain[1], "  1 │ ab   │  3");
        assert_eq!(plain[2], "  2 │ c    │ 10");
    }

    #[test]
    fn a_hex_dump_has_offsets_bytes_and_chars() {
        let l = hex(b"AB\0");
        let p = strip(&l.lines[0]);
        assert!(p.starts_with("00000000 │ 41 42 00"), "{p}");
        assert!(p.ends_with("AB⋄"), "{p}");
    }

    #[test]
    fn dates_read_right() {
        assert_eq!(date(0), "1970-01-01 00:00");
        assert_eq!(date(1_795_000_000), "2026-11-18 11:06");
    }

    #[test]
    fn a_tar_gz_and_a_zip_list_their_files() {
        let d = std::env::temp_dir().join(format!("tessh-arc-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let tgz = d.join("a.tar.gz");
        {
            let f = std::fs::File::create(&tgz).unwrap();
            let gz = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
            let mut t = tar::Builder::new(gz);
            let mut h = tar::Header::new_gnu();
            h.set_size(5);
            h.set_mode(0o644);
            h.set_cksum();
            t.append_data(&mut h, "dir/hello.txt", &b"hello"[..]).unwrap();
            t.into_inner().unwrap().finish().unwrap();
        }
        let (l, n) = archive(&tgz).unwrap();
        assert!(strip(&l.lines[0]).contains("dir/hello.txt"), "{:?}", l.lines);
        assert!(n.starts_with("tar, 1 files"), "{n}");
    }
}
