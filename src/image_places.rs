//! Where the terminal's pictures go on screen and their pixels, for the
//! window's renderer (the Qt types; the rest of images.rs is shared with
//! tesserad).

use crate::images::*;
use libghostty_vt::kitty::graphics::{Compression, ImageFormat, PlacementIterator};
use libghostty_vt::Terminal;

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

/// Where the runs on each viewport row (`rows[y]`) draw their images.
pub fn placeholder_places(term: &Terminal<'static, 'static>, rows: &[Vec<PhRun>], cell_w: u32, cell_h: u32, out: &mut Vec<crate::bridge::ffi::ImagePlace>) {
    if rows.iter().all(|r| r.is_empty()) {
        return;
    }
    let Ok(g) = term.kitty_graphics() else { return };
    // The virtual placements: (image, placement) -> (columns, rows).
    let mut virt: Vec<(u32, u32, u32, u32)> = Vec::new();
    if let Ok(mut it) = PlacementIterator::new() {
        if let Ok(mut iter) = it.update(&g) {
            while let Some(p) = iter.next() {
                if p.is_virtual().unwrap_or(false) {
                    if let Ok(id) = p.image_id() {
                        virt.push((id, p.placement_id().unwrap_or(0), p.columns().unwrap_or(0), p.rows().unwrap_or(0)));
                    }
                }
            }
        }
    }
    let (cw, ch) = (cell_w as f64, cell_h as f64);
    for (y, runs) in rows.iter().enumerate() {
        for r in runs {
            let Some(&(_, _, cols, prows)) = virt.iter().find(|v| v.0 == r.image_id && (r.placement_id == 0 || v.1 == r.placement_id)) else { continue };
            let Some(img) = g.image(r.image_id) else { continue };
            let (Ok(iw), Ok(ih)) = (img.width(), img.height()) else { continue };
            let (iw, ih) = (iw.max(1) as f64, ih.max(1) as f64);
            let cols = if cols == 0 { (iw / cw).ceil() } else { cols as f64 };
            let prows = if prows == 0 { (ih / ch).ceil() } else { prows as f64 };
            // The image fitted into the placement's box, centred.
            let (bw, bh) = (cols * cw, prows * ch);
            let s = (bw / iw).min(bh / ih);
            let (ox, oy) = ((bw - iw * s) / 2.0, (bh - ih * s) / 2.0);
            // This run's part of the box, and of the image in it.
            let (rx, ry, rw, rh) = (r.col as f64 * cw, r.row as f64 * ch, r.width as f64 * cw, ch);
            let x0 = rx.max(ox);
            let y0 = ry.max(oy);
            let x1 = (rx + rw).min(ox + iw * s);
            let y1 = (ry + rh).min(oy + ih * s);
            if x1 <= x0 || y1 <= y0 {
                continue;
            }
            out.push(crate::bridge::ffi::ImagePlace {
                id: r.image_id,
                generation: img.generation().unwrap_or(0),
                x: (r.x as f64 * cw + (x0 - rx)).round() as i32,
                y: (y as f64 * ch + (y0 - ry)).round() as i32,
                width: (x1 - x0).round() as u32,
                height: (y1 - y0).round() as u32,
                src_x: ((x0 - ox) / s).round() as u32,
                src_y: ((y0 - oy) / s).round() as u32,
                src_w: ((x1 - x0) / s).round().max(1.0) as u32,
                src_h: ((y1 - y0) / s).round().max(1.0) as u32,
                z: 0,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::tests::png_1x1;

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
}
