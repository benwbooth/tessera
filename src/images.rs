//! Pictures in the terminal: the kitty graphics protocol (libghostty keeps
//! the images and where they're placed; Tessera draws them), iTerm2's
//! inline images (imgcat), turned into kitty ones, and raw image files a
//! command prints (cat photo.png), shown as the picture instead of noise.

use base64::Engine;
use libghostty_vt::alloc::{Allocator, Bytes};
use libghostty_vt::kitty::graphics::{self, DecodePng, DecodedImage};
use libghostty_vt::Terminal;

/// Decodes PNG (and, for the conversions below, other formats) to RGBA.
struct Decoder;

impl DecodePng for Decoder {
    fn decode_png<'alloc>(&mut self, alloc: &'alloc Allocator<'_>, data: &[u8]) -> Option<DecodedImage<'alloc>> {
        let img = image::load_from_memory(data).ok()?.to_rgba8();
        let (w, h) = img.dimensions();
        let raw = img.into_raw();
        let mut bytes = Bytes::new_with_alloc(alloc, raw.len()).ok()?;
        bytes.copy_from_slice(&raw);
        Some(DecodedImage { width: w, height: h, data: bytes })
    }
}

/// Turn image support on for a terminal (and its thread: the decoder is
/// per thread).
pub fn enable(term: &mut Terminal<'static, 'static>) {
    let _ = graphics::set_png_decoder(Some(Box::new(Decoder)));
    let _ = term.set_kitty_image_storage_limit(320 * 1024 * 1024);
    // Files and shared memory stay off: only images sent through the terminal.
    let _ = term.set_kitty_image_from_file_allowed(false);
    let _ = term.set_kitty_image_from_temp_file_allowed(false);
    let _ = term.set_kitty_image_from_shared_mem_allowed(false);
}

/// A picture (any format `image` reads) as the kitty commands that show it
/// at the cursor, then a newline: its own size, or `max_cols` wide when it's
/// wider than that (0: no limit; `cell_w`: a cell's width in pixels).
pub fn kitty_display(data: &[u8], max_cols: u32, cell_w: u32) -> Option<Vec<u8>> {
    let img = image::load_from_memory(data).ok()?;
    let natural = if cell_w > 0 { img.width().div_ceil(cell_w) } else { 0 };
    let cols = if max_cols > 0 && (natural == 0 || natural > max_cols) { max_cols } else { 0 };
    // PNG on the wire (the terminal decodes it).
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).ok()?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
    let mut out = Vec::new();
    let chunks: Vec<&[u8]> = b64.as_bytes().chunks(4096).collect();
    for (i, c) in chunks.iter().enumerate() {
        let more = if i + 1 < chunks.len() { 1 } else { 0 };
        if i == 0 {
            let size = if cols > 0 { format!(",c={cols}") } else { String::new() };
            out.extend_from_slice(format!("\x1b_Ga=T,f=100,q=2,C=0{size},m={more};").as_bytes());
        } else {
            out.extend_from_slice(format!("\x1b_Gm={more};").as_bytes());
        }
        out.extend_from_slice(c);
        out.extend_from_slice(b"\x1b\\");
    }
    out.extend_from_slice(b"\r\n");
    Some(out)
}

/// iTerm2's inline image ("OSC 1337 ; File=[args] : base64 BEL"): the
/// picture, if the arguments say inline=1.
pub fn iterm2_file(payload: &[u8]) -> Option<Vec<u8>> {
    let s = std::str::from_utf8(payload).ok()?;
    let rest = s.strip_prefix("File=").unwrap_or(s);
    let (args, data) = rest.split_once(':')?;
    let inline = args.split(';').any(|a| a.trim() == "inline=1");
    if !inline {
        return None;
    }
    base64::engine::general_purpose::STANDARD.decode(data.trim()).ok()
}

/// Undo the tty's output processing (ONLCR turns each "\n" a program writes
/// into "\r\n"; nothing else becomes "\r\n"), giving back a file's bytes.
pub fn un_onlcr(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut i = 0;
    while i < data.len() {
        if data[i] == b'\r' && data.get(i + 1) == Some(&b'\n') {
            out.push(b'\n');
            i += 2;
        } else {
            out.push(data[i]);
            i += 1;
        }
    }
    out
}

/// Whether output starts like an image file (as the tty passes it on).
pub fn output_is_image(data: &[u8]) -> bool {
    is_image(&un_onlcr(&data[..data.len().min(32)]))
}

/// Whether bytes start like an image file.
pub fn is_image(data: &[u8]) -> bool {
    data.starts_with(b"\x89PNG\r\n\x1a\n")
        || data.starts_with(&[0xff, 0xd8, 0xff])
        || data.starts_with(b"GIF87a")
        || data.starts_with(b"GIF89a")
        || (data.len() > 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP")
        || data.starts_with(b"BM")
}

// ---- kitty's Unicode placeholders ---------------------------------------
// An image placed virtually (U=1) shows where the text has U+10EEEE cells:
// the cell's foreground color is the image's id, its underline color the
// placement's, and combining marks after it the image's row, column and the
// id's high byte (kitty's table of diacritics; the n-th means n). A run of
// such cells shows a strip of the image, fitted into the placement's rows
// and columns. (Mdfried's headings, among others.)

pub const PLACEHOLDER: char = '\u{10EEEE}';

const DIACRITICS: [u32; 297] = [
    0x0305, 0x030D, 0x030E, 0x0310, 0x0312, 0x033D, 0x033E, 0x033F, 0x0346, 0x034A, 0x034B, 0x034C, 0x0350,
    0x0351, 0x0352, 0x0357, 0x035B, 0x0363, 0x0364, 0x0365, 0x0366, 0x0367, 0x0368, 0x0369, 0x036A, 0x036B,
    0x036C, 0x036D, 0x036E, 0x036F, 0x0483, 0x0484, 0x0485, 0x0486, 0x0487, 0x0592, 0x0593, 0x0594, 0x0595,
    0x0597, 0x0598, 0x0599, 0x059C, 0x059D, 0x059E, 0x059F, 0x05A0, 0x05A1, 0x05A8, 0x05A9, 0x05AB, 0x05AC,
    0x05AF, 0x05C4, 0x0610, 0x0611, 0x0612, 0x0613, 0x0614, 0x0615, 0x0616, 0x0617, 0x0657, 0x0658, 0x0659,
    0x065A, 0x065B, 0x065D, 0x065E, 0x06D6, 0x06D7, 0x06D8, 0x06D9, 0x06DA, 0x06DB, 0x06DC, 0x06DF, 0x06E0,
    0x06E1, 0x06E2, 0x06E4, 0x06E7, 0x06E8, 0x06EB, 0x06EC, 0x0730, 0x0732, 0x0733, 0x0735, 0x0736, 0x073A,
    0x073D, 0x073F, 0x0740, 0x0741, 0x0743, 0x0745, 0x0747, 0x0749, 0x074A, 0x07EB, 0x07EC, 0x07ED, 0x07EE,
    0x07EF, 0x07F0, 0x07F1, 0x07F3, 0x0816, 0x0817, 0x0818, 0x0819, 0x081B, 0x081C, 0x081D, 0x081E, 0x081F,
    0x0820, 0x0821, 0x0822, 0x0823, 0x0825, 0x0826, 0x0827, 0x0829, 0x082A, 0x082B, 0x082C, 0x082D, 0x0951,
    0x0953, 0x0954, 0x0F82, 0x0F83, 0x0F86, 0x0F87, 0x135D, 0x135E, 0x135F, 0x17DD, 0x193A, 0x1A17, 0x1A75,
    0x1A76, 0x1A77, 0x1A78, 0x1A79, 0x1A7A, 0x1A7B, 0x1A7C, 0x1B6B, 0x1B6D, 0x1B6E, 0x1B6F, 0x1B70, 0x1B71,
    0x1B72, 0x1B73, 0x1CD0, 0x1CD1, 0x1CD2, 0x1CDA, 0x1CDB, 0x1CE0, 0x1DC0, 0x1DC1, 0x1DC3, 0x1DC4, 0x1DC5,
    0x1DC6, 0x1DC7, 0x1DC8, 0x1DC9, 0x1DCB, 0x1DCC, 0x1DD1, 0x1DD2, 0x1DD3, 0x1DD4, 0x1DD5, 0x1DD6, 0x1DD7,
    0x1DD8, 0x1DD9, 0x1DDA, 0x1DDB, 0x1DDC, 0x1DDD, 0x1DDE, 0x1DDF, 0x1DE0, 0x1DE1, 0x1DE2, 0x1DE3, 0x1DE4,
    0x1DE5, 0x1DE6, 0x1DFE, 0x20D0, 0x20D1, 0x20D4, 0x20D5, 0x20D6, 0x20D7, 0x20DB, 0x20DC, 0x20E1, 0x20E7,
    0x20E9, 0x20F0, 0x2CEF, 0x2CF0, 0x2CF1, 0x2DE0, 0x2DE1, 0x2DE2, 0x2DE3, 0x2DE4, 0x2DE5, 0x2DE6, 0x2DE7,
    0x2DE8, 0x2DE9, 0x2DEA, 0x2DEB, 0x2DEC, 0x2DED, 0x2DEE, 0x2DEF, 0x2DF0, 0x2DF1, 0x2DF2, 0x2DF3, 0x2DF4,
    0x2DF5, 0x2DF6, 0x2DF7, 0x2DF8, 0x2DF9, 0x2DFA, 0x2DFB, 0x2DFC, 0x2DFD, 0x2DFE, 0x2DFF, 0xA66F, 0xA67C,
    0xA67D, 0xA6F0, 0xA6F1, 0xA8E0, 0xA8E1, 0xA8E2, 0xA8E3, 0xA8E4, 0xA8E5, 0xA8E6, 0xA8E7, 0xA8E8, 0xA8E9,
    0xA8EA, 0xA8EB, 0xA8EC, 0xA8ED, 0xA8EE, 0xA8EF, 0xA8F0, 0xA8F1, 0xAAB0, 0xAAB2, 0xAAB3, 0xAAB7, 0xAAB8,
    0xAABE, 0xAABF, 0xAAC1, 0xFE20, 0xFE21, 0xFE22, 0xFE23, 0xFE24, 0xFE25, 0xFE26, 0x10A0F, 0x10A38, 0x1D185,
    0x1D186, 0x1D187, 0x1D188, 0x1D189, 0x1D1AA, 0x1D1AB, 0x1D1AC, 0x1D1AD, 0x1D242, 0x1D243, 0x1D244
];

fn diacritic(c: char) -> Option<u32> {
    DIACRITICS.binary_search(&(c as u32)).ok().map(|i| i as u32)
}

fn color_id(c: libghostty_vt::style::StyleColor) -> u32 {
    use libghostty_vt::style::StyleColor;
    match c {
        StyleColor::None => 0,
        StyleColor::Palette(p) => p.0 as u32,
        StyleColor::Rgb(c) => (c.r as u32) << 16 | (c.g as u32) << 8 | c.b as u32,
    }
}

/// A run of placeholder cells on one row: from viewport column `x`, `width`
/// cells showing the image's cells from (`row`, `col`).
#[derive(Clone, Debug, PartialEq)]
pub struct PhRun {
    pub x: u16,
    pub image_id: u32,
    pub placement_id: u32,
    pub row: u32,
    pub col: u32,
    pub width: u32,
    high: Option<u32>,
}

/// A placeholder cell at column `x` (its text: U+10EEEE and the marks),
/// added to the row's runs; false when it isn't one.
pub fn placeholder_cell(runs: &mut Vec<PhRun>, x: u16, text: &str, fg: libghostty_vt::style::StyleColor, underline: libghostty_vt::style::StyleColor) -> bool {
    let mut cs = text.chars();
    if cs.next() != Some(PLACEHOLDER) {
        return false;
    }
    let low = color_id(fg);
    let pid = color_id(underline);
    let marks: Vec<Option<u32>> = cs.take(3).map(diacritic).collect();
    let row = marks.first().copied().flatten();
    let col = marks.get(1).copied().flatten();
    let high = marks.get(2).copied().flatten().filter(|h| *h < 256);
    if let Some(p) = runs.last_mut() {
        // (Kitty's rule: the same image and placement, the same row, the next
        // column, the cell right after.)
        if p.x as u32 + p.width == x as u32
            && p.image_id & 0xffffff == low
            && p.placement_id == pid
            && row.is_none_or(|r| r == p.row)
            && col.is_none_or(|c| c == p.col + p.width)
            && high.is_none_or(|h| Some(h) == p.high)
        {
            p.width += 1;
            return true;
        }
    }
    runs.push(PhRun { x, image_id: low | high.unwrap_or(0) << 24, placement_id: pid, row: row.unwrap_or(0), col: col.unwrap_or(0), width: 1, high });
    true
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn png_1x1() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }


    #[test]
    fn through_a_tty() {
        let png = png_1x1();
        let mut tty = Vec::new();
        for &b in &png {
            if b == b'\n' { tty.extend_from_slice(b"\r\n") } else { tty.push(b) }
        }
        assert!(output_is_image(&tty));
        assert_eq!(un_onlcr(&tty), png);
    }

    #[test]
    fn iterm2_inline() {
        let png = png_1x1();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
        let p = format!("File=name=eA==;size={};inline=1:{b64}", png.len());
        assert_eq!(iterm2_file(p.as_bytes()).unwrap(), png);
        assert!(iterm2_file(b"File=inline=0:AAAA").is_none());
    }
}

/// What a file shown by a viewer command (cat README.md, bat data.json,
/// head -50 x.csv) is, from its name: "" when it isn't one of those.
pub fn kind_from_command(cmd: &str) -> &'static str {
    // (The last command of a list: cd somewhere; cat it.md)
    let last = cmd.rsplit(|c| c == ';' || c == '&' || c == '|').find(|p| !p.trim().is_empty()).unwrap_or(cmd);
    let words = shell_words::split(last).unwrap_or_default();
    let Some(first) = words.first() else { return "" };
    let prog = first.rsplit('/').next().unwrap_or(first);
    if !matches!(prog, "cat" | "bat" | "batcat" | "head" | "tail" | "less" | "more" | "glow" | "mdcat" | "jq" | "type" | "Get-Content") {
        return "";
    }
    let Some(file) = words.iter().skip(1).rev().find(|w| !w.starts_with('-')) else { return "" };
    let ext = file.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "md" | "markdown" | "mdx" => "markdown",
        "json" | "jsonl" | "ndjson" | "geojson" => "json",
        "csv" => "csv",
        "tsv" => "tsv",
        "html" | "htm" => "html",
        "svg" => "svg",
        "diff" | "patch" => "diff",
        _ => "",
    }
}

/// What a command's text output is, for a rich view: "json", "csv", "tsv",
/// "html", "svg", "markdown", or "" (plain text). Clear-cut cases only.
pub fn detect_text(text: &str) -> &'static str {
    let t = text.trim();
    if t.len() < 2 {
        return "";
    }
    let lower: String = t.chars().take(256).collect::<String>().to_lowercase();
    if lower.starts_with("<svg") || (lower.starts_with("<?xml") && lower.contains("<svg")) {
        return "svg";
    }
    // (Markdown that opens with some HTML, as many READMEs do, is still Markdown.)
    let md_headings = t.lines().filter(|l| l.starts_with("# ") || l.starts_with("## ") || l.starts_with("### ")).count();
    if lower.starts_with("<!doctype html") || lower.starts_with("<html")
        || (lower.starts_with('<') && md_headings < 2 && (lower.contains("<body") || lower.contains("<div") || lower.contains("<table") || lower.contains("<p>")))
    {
        return "html";
    }
    if (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']')) {
        if serde_json::from_str::<serde_json::Value>(t).is_ok() && t.len() > 20 {
            return "json";
        }
        // JSON lines.
        if t.lines().count() > 1 && t.lines().all(|l| l.trim().is_empty() || serde_json::from_str::<serde_json::Value>(l).is_ok()) {
            return "json";
        }
    }
    let lines: Vec<&str> = t.lines().filter(|l| !l.trim().is_empty()).take(200).collect();
    if lines.len() >= 3 {
        for (sep, kind) in [('\t', "tsv"), (',', "csv")] {
            let counts: Vec<usize> = lines.iter().map(|l| l.matches(sep).count()).collect();
            if counts[0] >= 1 && counts.iter().all(|&c| c == counts[0]) {
                return kind;
            }
        }
    }
    let md_marks = lines.iter().filter(|l| l.starts_with("# ") || l.starts_with("## ") || l.starts_with("- ") || l.starts_with("* ") || l.starts_with("```") || l.starts_with("| ") || l.starts_with("> ")).count();
    // (A script's comments aren't headings.)
    if !t.starts_with("#!")
        && ((lines.len() >= 3 && md_marks * 3 >= lines.len() && lines.iter().any(|l| l.starts_with('#')))
            || (md_headings >= 2 && (t.contains("](") || t.contains("```"))))
    {
        return "markdown";
    }
    ""
}

/// Terminal output as plain text: escape sequences and carriage returns out.
pub fn plain_text(raw: &[u8]) -> String {
    let s = String::from_utf8_lossy(&un_onlcr(raw)).into_owned();
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.peek() {
                Some('[') => {
                    chars.next();
                    for x in chars.by_ref() {
                        if ('@'..='~').contains(&x) {
                            break;
                        }
                    }
                }
                Some(']') | Some('P') | Some('_') => {
                    // OSC / DCS / APC: through BEL or ST.
                    while let Some(x) = chars.next() {
                        if x == '\u{7}' {
                            break;
                        }
                        if x == '\u{1b}' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {
                    chars.next();
                }
            },
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod detect_tests {
    use super::*;

    #[test]
    fn kinds() {
        assert_eq!(detect_text("{\"name\": \"tessera\", \"stars\": 3}"), "json");
        assert_eq!(detect_text("a,b,c\n1,2,3\n4,5,6\n"), "csv");
        assert_eq!(detect_text("<!DOCTYPE html><html><body>x</body></html>"), "html");
        assert_eq!(detect_text("<svg xmlns='http://www.w3.org/2000/svg'></svg>"), "svg");
        assert_eq!(detect_text("# Title\n\n- one\n- two\n"), "markdown");
        assert_eq!(kind_from_command("cat README.md"), "markdown");
        assert_eq!(kind_from_command("bat -p data.json"), "json");
        assert_eq!(kind_from_command("git add README.md"), "");
        assert_eq!(kind_from_command("cd ~/src/Kog; cat README.md"), "markdown");
        assert_eq!(detect_text("<p align=center>x</p>\n\n# One\n\n[a](b)\n\n## Two\n\nmore text here\n"), "markdown");
        assert_eq!(detect_text("#!/bin/sh\n# build it\nmake\n# then test\nmake test\nexit 0\n"), "");
        assert_eq!(detect_text("total 12\ndrwxr-xr-x 2 ben users 4096 src\n-rw-r--r-- 1 ben users 3 a, b\n"), "");
        assert_eq!(plain_text(b"\x1b[31mred\x1b[0m\r\nnext"), "red\nnext");
    }
}
