//! Pictures in the terminal: the kitty graphics protocol, sixel or iTerm2's
//! inline images, whichever the terminal answers to (asked once), else
//! half-block characters in 24-bit color. Decoded as Tessera's
//! src/images.rs does (the `image` crate; kitty gets PNG on the wire).

use base64::Engine;
use std::io::Write;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Proto {
    Kitty,
    Sixel,
    Iterm,
    Blocks,
}

impl Proto {
    pub fn name(self) -> &'static str {
        match self {
            Proto::Kitty => "kitty graphics",
            Proto::Sixel => "sixel",
            Proto::Iterm => "iTerm2 images",
            Proto::Blocks => "blocks",
        }
    }
}

/// Read what the terminal sends back, until `done` says it's all there or
/// `ms` pass.
fn read_reply(ms: u64, done: impl Fn(&[u8]) -> bool) -> Vec<u8> {
    let mut got = vec![];
    let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
    loop {
        let left = end.saturating_duration_since(std::time::Instant::now()).as_millis() as i32;
        if left <= 0 {
            break;
        }
        let mut pfd = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
        if unsafe { libc::poll(&mut pfd, 1, left) } <= 0 {
            break;
        }
        let mut buf = [0u8; 512];
        let n = unsafe { libc::read(0, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
        if n <= 0 {
            break;
        }
        got.extend_from_slice(&buf[..n as usize]);
        if done(&got) {
            break;
        }
    }
    got
}

/// Which protocol to use: $TESSH_GRAPHICS when set (kitty, sixel, iterm,
/// blocks), else asked of the terminal (raw mode already on): a kitty query
/// and the primary device attributes (sixel is attribute 4). iTerm2's own
/// images go by what the terminal says it is. Asked once per run.
pub fn detect() -> Proto {
    static P: std::sync::OnceLock<Proto> = std::sync::OnceLock::new();
    *P.get_or_init(|| {
        match std::env::var("TESSH_GRAPHICS").unwrap_or_default().as_str() {
            "kitty" => return Proto::Kitty,
            "sixel" => return Proto::Sixel,
            "iterm" | "iterm2" => return Proto::Iterm,
            "blocks" | "none" => return Proto::Blocks,
            _ => {}
        }
        if unsafe { libc::isatty(0) } != 1 || unsafe { libc::isatty(1) } != 1 {
            return Proto::Blocks;
        }
        // (In tmux: tmux answers for itself, but a picture shows only if the
        // terminal it's drawn on takes it too. Sixel when that one does
        // (tmux passes it on); kitty's and iTerm2's don't get through.)
        if std::env::var_os("TMUX").is_some() {
            let feats = std::process::Command::new("tmux").args(["display-message", "-p", "#{client_termfeatures}"]).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
            return if feats.split(',').any(|f| f.trim() == "sixel") { Proto::Sixel } else { Proto::Blocks };
        }
        let raw = crossterm::terminal::is_raw_mode_enabled().unwrap_or(false);
        if !raw {
            let _ = crossterm::terminal::enable_raw_mode();
        }
        let mut o = std::io::stdout();
        let _ = write!(o, "\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c");
        let _ = o.flush();
        // (The attributes' reply, ESC [ ? … c, comes last: every terminal answers it.)
        let reply = read_reply(800, |b| {
            let s = String::from_utf8_lossy(b);
            s.find("\x1b[?").is_some_and(|i| s[i..].contains('c'))
        });
        if !raw {
            let _ = crossterm::terminal::disable_raw_mode();
        }
        let s = String::from_utf8_lossy(&reply);
        let iterm = matches!(std::env::var("TERM_PROGRAM").as_deref(), Ok("iTerm.app") | Ok("WezTerm")) || std::env::var("LC_TERMINAL").as_deref() == Ok("iTerm2");
        if s.contains("_Gi=31;OK") {
            Proto::Kitty
        } else if iterm && std::env::var("TMUX").is_err() {
            Proto::Iterm
        } else if s.find("\x1b[?").map(|i| &s[i + 3..]).and_then(|r| r.split('c').next()).is_some_and(|a| a.split(';').any(|x| x == "4")) {
            Proto::Sixel
        } else {
            Proto::Blocks
        }
    })
}

/// A cell's size in pixels (the tty's, else a guess).
pub fn cell_px() -> (u32, u32) {
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    if unsafe { libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) } == 0 && ws.ws_xpixel > 0 && ws.ws_ypixel > 0 && ws.ws_col > 0 && ws.ws_row > 0 {
        return ((ws.ws_xpixel / ws.ws_col) as u32, (ws.ws_ypixel / ws.ws_row) as u32);
    }
    (8, 16)
}

/// The cells (columns, rows) a picture of `w`×`h` pixels takes fitted into
/// `cols`×`rows`, never larger than its own size.
pub fn fit(w: u32, h: u32, cols: u32, rows: u32) -> (u32, u32) {
    let (cw, ch) = cell_px();
    let (nat_c, nat_r) = (w.div_ceil(cw).max(1), h.div_ceil(ch).max(1));
    if nat_c <= cols && nat_r <= rows {
        return (nat_c, nat_r);
    }
    // (Scaled by the tighter side, in pixels so the aspect stays.)
    let s = (cols as f64 * cw as f64 / w as f64).min(rows as f64 * ch as f64 / h as f64);
    (((w as f64 * s) / cw as f64).floor().max(1.0) as u32, ((h as f64 * s) / ch as f64).floor().max(1.0) as u32)
}

/// The escapes that draw `img` at the cursor in `cols`×`rows` cells (the
/// size `fit` gave), by `proto`. Blocks: one line per row, each starting at
/// column `x` (1-based) of row `y` onwards.
pub fn render(img: &image::DynamicImage, proto: Proto, cols: u32, rows: u32, x: u32, y: u32) -> String {
    let mut out = format!("\x1b[{y};{x}H");
    match proto {
        Proto::Kitty => {
            let mut png = Vec::new();
            let _ = img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png);
            let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
            let chunks: Vec<&[u8]> = b64.as_bytes().chunks(4096).collect();
            for (i, c) in chunks.iter().enumerate() {
                let more = if i + 1 < chunks.len() { 1 } else { 0 };
                if i == 0 {
                    out.push_str(&format!("\x1b_Ga=T,f=100,q=2,C=1,c={cols},r={rows},m={more};"));
                } else {
                    out.push_str(&format!("\x1b_Gm={more};"));
                }
                out.push_str(std::str::from_utf8(c).unwrap_or(""));
                out.push_str("\x1b\\");
            }
        }
        Proto::Iterm => {
            let mut png = Vec::new();
            let _ = img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png);
            let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
            out.push_str(&format!("\x1b]1337;File=inline=1;size={};width={cols};height={rows};preserveAspectRatio=1:{b64}\x07", png.len()));
        }
        Proto::Sixel => {
            let (cw, ch) = cell_px();
            let small = img.resize_exact(cols * cw, rows * ch, image::imageops::FilterType::Triangle).to_rgba8();
            if let Ok(s) = icy_sixel::SixelImage::from_rgba(small.into_raw(), (cols * cw) as usize, (rows * ch) as usize).encode() {
                out.push_str(&s);
            }
        }
        Proto::Blocks => {
            out.clear();
            for (r, line) in blocks(img, cols, rows).into_iter().enumerate() {
                out.push_str(&format!("\x1b[{};{x}H{line}", y + r as u32));
            }
        }
    }
    out
}

/// The picture as half blocks: each cell two pixels high (▀ in the top
/// one's color over the bottom one's), `cols`×`rows` cells, a line each.
pub fn blocks(img: &image::DynamicImage, cols: u32, rows: u32) -> Vec<String> {
    let small = img.resize_exact(cols.max(1), (rows * 2).max(1), image::imageops::FilterType::Triangle).to_rgba8();
    let mut lines = vec![];
    for r in 0..rows {
        let mut s = String::new();
        for c in 0..cols {
            let t = small.get_pixel(c, r * 2);
            let b = small.get_pixel(c, (r * 2 + 1).min(small.height() - 1));
            match (t[3] >= 128, b[3] >= 128) {
                (true, true) => s.push_str(&format!("\x1b[38;2;{};{};{};48;2;{};{};{}m▀", t[0], t[1], t[2], b[0], b[1], b[2])),
                (true, false) => s.push_str(&format!("\x1b[49;38;2;{};{};{}m▀", t[0], t[1], t[2])),
                (false, true) => s.push_str(&format!("\x1b[49;38;2;{};{};{}m▄", b[0], b[1], b[2])),
                (false, false) => s.push_str("\x1b[0m "),
            }
        }
        s.push_str("\x1b[0m");
        lines.push(s);
    }
    lines
}

/// Remove what kitty graphics drew (on leaving, or before drawing again).
pub fn clear(proto: Proto) -> &'static str {
    if proto == Proto::Kitty {
        "\x1b_Ga=d,d=A,q=2\x1b\\"
    } else {
        ""
    }
}

/// Print a picture where the cursor is, inline (cat at a tty): as wide as
/// it is, or the terminal (and no taller than the screen).
pub fn print_inline(path: &std::path::Path) -> Result<(), String> {
    let img = image::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (tc, tr) = crossterm::terminal::size().unwrap_or((80, 24));
    let proto = detect();
    let (c, r) = fit(img.width(), img.height(), tc as u32, (tr as u32).saturating_sub(2).max(4));
    let mut o = std::io::stdout().lock();
    if proto == Proto::Blocks {
        for l in blocks(&img, c, r) {
            let _ = writeln!(o, "{l}");
        }
    } else {
        // (Room first: scrolled up so the picture fits below the cursor, then
        // drawn there, the cursor left under it.)
        let _ = write!(o, "{}\x1b[{r}A\r", "\n".repeat(r as usize));
        let s = render(&img, proto, c, r, 1, 1);
        // (render positions absolutely; inline it's at the cursor instead.)
        let s = s.split_once('H').map(|(_, rest)| rest.to_string()).unwrap_or(s);
        let _ = write!(o, "\x1b7{s}\x1b8\x1b[{r}B\r");
    }
    let _ = o.flush();
    Ok(())
}

/// A picture full screen on the other screen: fitted, its name, size and
/// format at the foot; q or Esc back, o opens it in its own app, y copies
/// its path, + and - zoom (0 fits it again), the arrows move a zoomed one.
pub fn show(path: &std::path::Path) -> Result<(), String> {
    let img = image::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut o = std::io::stdout();
    let _ = crossterm::terminal::enable_raw_mode();
    let proto = detect();
    let _ = write!(o, "\x1b[?1049h\x1b[?1002h\x1b[?1006h\x1b[?25l");
    let fmt = image::ImageFormat::from_path(path).map(|f| format!("{f:?}")).unwrap_or_else(|_| "image".into());
    let size = std::fs::metadata(path).map(|m| super::human_size(m.len())).unwrap_or_default();
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let mut zoom: f64 = 1.0;
    let (mut px, mut py) = (0.5f64, 0.5f64);
    let mut last = (0u16, 0u16, 0.0f64, 0.0f64, 0.0f64);
    let mut note = String::new();
    loop {
        let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
        if (cols, rows, zoom, px, py) != last {
            last = (cols, rows, zoom, px, py);
            let body_r = (rows as u32).saturating_sub(2).max(1);
            let (fc, fr) = fit(img.width(), img.height(), cols as u32, body_r);
            // (Zoomed: a part of the picture, centered on (px, py), filling the screen.)
            let (view, c, r) = if zoom > 1.0 {
                let (w, h) = ((img.width() as f64 / zoom).max(1.0), (img.height() as f64 / zoom).max(1.0));
                let x0 = ((img.width() as f64 * px - w / 2.0).max(0.0)).min(img.width() as f64 - w);
                let y0 = ((img.height() as f64 * py - h / 2.0).max(0.0)).min(img.height() as f64 - h);
                let part = img.crop_imm(x0 as u32, y0 as u32, w as u32, h as u32);
                let (cw, ch) = cell_px();
                let s = (cols as f64 * cw as f64 / w).min(body_r as f64 * ch as f64 / h);
                let (c, r) = (((w * s) / cw as f64).max(1.0) as u32, ((h * s) / ch as f64).max(1.0) as u32);
                (part, c.min(cols as u32), r.min(body_r))
            } else {
                (img.clone(), fc, fr)
            };
            let x = (cols as u32).saturating_sub(c) / 2 + 1;
            let y = body_r.saturating_sub(r) / 2 + 2;
            let mut s = String::from("\x1b[?2026h");
            s.push_str(clear(proto));
            s.push_str("\x1b[0m\x1b[H\x1b[2J");
            let head = format!(" {name}  \x1b[2m{}×{} {fmt} · {size} · {}{}\x1b[22m", img.width(), img.height(), proto.name(), if zoom > 1.0 { format!(" · {:.0}%", zoom * 100.0) } else { String::new() });
            s.push_str(&format!("\x1b[1;1H\x1b[1m{}\x1b[0m", super::super::editor::pager::cut(&head, cols as usize)));
            s.push_str(&render(&view, proto, c, r, x, y));
            let hint = if note.is_empty() { " q back   o open in its app   + - zoom   0 fit   ← → ↑ ↓ move (zoomed)   y copy path   (wheel zooms, click opens)".to_string() } else { format!(" {note}") };
            s.push_str(&format!("\x1b[{rows};1H\x1b[0m\x1b[2m{}\x1b[0m", super::super::editor::pager::cut(&hint, cols as usize)));
            s.push_str("\x1b[?2026l");
            let _ = write!(o, "{s}");
            let _ = o.flush();
        }
        let ev = match crossterm::event::read() {
            Ok(e) => e,
            Err(_) => break,
        };
        use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};
        note.clear();
        let step = 0.25 / zoom;
        match ev {
            Event::Key(k) if k.kind != crossterm::event::KeyEventKind::Release => match k.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('o') | KeyCode::Enter => {
                    super::open_outside(path);
                    note = "opened in its app".into();
                    last.0 = 0;
                }
                KeyCode::Char('y') => {
                    crate::editor::term::Term::copy(&path.to_string_lossy());
                    note = "copied the path".into();
                    last.0 = 0;
                }
                KeyCode::Char('+') | KeyCode::Char('=') => zoom = (zoom * 1.5).min(32.0),
                KeyCode::Char('-') => zoom = (zoom / 1.5).max(1.0),
                KeyCode::Char('0') => {
                    zoom = 1.0;
                    (px, py) = (0.5, 0.5);
                }
                KeyCode::Left | KeyCode::Char('h') => px = (px - step).max(0.0),
                KeyCode::Right | KeyCode::Char('l') => px = (px + step).min(1.0),
                KeyCode::Up | KeyCode::Char('k') => py = (py - step).max(0.0),
                KeyCode::Down | KeyCode::Char('j') => py = (py + step).min(1.0),
                _ => {}
            },
            Event::Mouse(m) => match m.kind {
                MouseEventKind::ScrollUp => zoom = (zoom * 1.25).min(32.0),
                MouseEventKind::ScrollDown => zoom = (zoom / 1.25).max(1.0),
                MouseEventKind::Down(MouseButton::Left) => {
                    super::open_outside(path);
                    note = "opened in its app".into();
                    last.0 = 0;
                }
                _ => {}
            },
            Event::Resize(..) => last.0 = 0,
            _ => {}
        }
    }
    let _ = write!(o, "{}\x1b[?1006l\x1b[?1002l\x1b[?1049l\x1b[?25h", clear(proto));
    let _ = o.flush();
    let _ = crossterm::terminal::disable_raw_mode();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_are_two_pixels_a_cell() {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_fn(4, 4, |_, y| if y < 2 { image::Rgba([255, 0, 0, 255]) } else { image::Rgba([0, 0, 255, 255]) }));
        let l = blocks(&img, 4, 2);
        assert_eq!(l.len(), 2);
        assert!(l[0].contains("38;2;255;0;0;48;2;255;0;0m▀"), "{:?}", l[0]);
        assert!(l[1].contains("38;2;0;0;255"), "{:?}", l[1]);
    }

    #[test]
    fn fitting_keeps_the_shape_and_never_enlarges() {
        let (c, r) = fit(16, 16, 80, 24);
        assert!(c <= 2 && r <= 1);
        let (c, r) = fit(8000, 4000, 80, 40);
        assert!(c <= 80 && r <= 40);
    }
}
