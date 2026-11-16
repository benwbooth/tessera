//! Pictures in the terminal: the kitty graphics protocol (libghostty keeps
//! the images and where they're placed; Tessera draws them), iTerm2's
//! inline images (imgcat), turned into kitty ones, and raw image files a
//! command prints (cat photo.png), shown as the picture instead of noise.

use base64::Engine;
use libghostty_vt::alloc::{Allocator, Bytes};
use libghostty_vt::kitty::graphics::{self, Compression, DecodePng, DecodedImage, ImageFormat, PlacementIterator};
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

/// Where each visible image goes, in device pixels from the grid's origin.
pub fn placements(term: &Terminal<'static, 'static>, cell_w: u32, cell_h: u32, out: &mut Vec<crate::bridge::ffi::ImagePlace>) {
    out.clear();
    let Ok(g) = term.kitty_graphics() else { return };
    let Ok(mut it) = PlacementIterator::new() else { return };
    let Ok(mut iter) = it.update(&g) else { return };
    while let Some(p) = iter.next() {
        let Ok(id) = p.image_id() else { continue };
        let Some(img) = g.image(id) else { continue };
        let Ok(Some(pos)) = p.viewport_pos(&img, term) else { continue };
        let Ok(size) = p.pixel_size(&img, term) else { continue };
        let Ok(src) = p.source_rect(&img) else { continue };
        out.push(crate::bridge::ffi::ImagePlace {
            id,
            generation: img.generation().unwrap_or(0),
            x: pos.col * cell_w as i32 + p.x_offset().unwrap_or(0) as i32,
            y: pos.row * cell_h as i32 + p.y_offset().unwrap_or(0) as i32,
            width: size.width,
            height: size.height,
            src_x: src.x,
            src_y: src.y,
            src_w: src.width,
            src_h: src.height,
            z: p.z().unwrap_or(0),
        });
    }
}

/// An image's pixels as RGBA (empty when it can't be read).
pub fn rgba(term: &Terminal<'static, 'static>, id: u32) -> crate::bridge::ffi::ImagePixels {
    let none = crate::bridge::ffi::ImagePixels { width: 0, height: 0, rgba: Vec::new() };
    let Ok(g) = term.kitty_graphics() else { return none };
    let Some(img) = g.image(id) else { return none };
    let (Ok(w), Ok(h), Ok(fmt), Ok(data)) = (img.width(), img.height(), img.format(), img.data()) else { return none };
    let data: Vec<u8> = match img.compression() {
        Ok(Compression::ZlibDeflate) => match miniz_oxide::inflate::decompress_to_vec_zlib(data) {
            Ok(d) => d,
            Err(_) => return none,
        },
        _ => data.to_vec(),
    };
    let n = (w as usize) * (h as usize);
    let rgba = match fmt {
        ImageFormat::Rgba if data.len() >= n * 4 => data[..n * 4].to_vec(),
        ImageFormat::Rgb if data.len() >= n * 3 => data[..n * 3].chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        ImageFormat::Gray if data.len() >= n => data[..n].iter().flat_map(|&g| [g, g, g, 255]).collect(),
        ImageFormat::GrayAlpha if data.len() >= n * 2 => data[..n * 2].chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        ImageFormat::Png => match image::load_from_memory(&data) {
            Ok(i) => i.to_rgba8().into_raw(),
            Err(_) => return none,
        },
        _ => return none,
    };
    crate::bridge::ffi::ImagePixels { width: w, height: h, rgba }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn png_1x1() -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    #[test]
    fn kitty_image_round_trip() {
        let mut t = Terminal::new(libghostty_vt::terminal::Options { cols: 40, rows: 10, max_scrollback: 0 }).unwrap();
        t.resize(40, 10, 8, 16).unwrap();
        enable(&mut t);
        let png = png_1x1();
        assert!(is_image(&png));
        t.vt_write(&kitty_display(&png, 0, 8).unwrap());
        let mut places = Vec::new();
        placements(&t, 8, 16, &mut places);
        assert_eq!(places.len(), 1, "one placement");
        let px = rgba(&t, places[0].id);
        assert_eq!((px.width, px.height), (2, 2));
        assert_eq!(&px.rgba[..4], &[255, 0, 0, 255]);
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
    if lower.starts_with("<!doctype html") || lower.starts_with("<html") || (lower.starts_with('<') && (lower.contains("<body") || lower.contains("<div") || lower.contains("<table") || lower.contains("<p>"))) {
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
    if lines.len() >= 3 && md_marks * 3 >= lines.len() && lines.iter().any(|l| l.starts_with('#')) {
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
        assert_eq!(detect_text("total 12\ndrwxr-xr-x 2 ben users 4096 src\n-rw-r--r-- 1 ben users 3 a, b\n"), "");
        assert_eq!(plain_text(b"\x1b[31mred\x1b[0m\r\nnext"), "red\nnext");
    }
}
