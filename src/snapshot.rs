//! A terminal's whole state as VT sequences (screen, scrollback, colors,
//! cursor, modes), so a fresh terminal fed with it looks exactly the same.
//! The daemon keeps a terminal per session and sends this on attach.

use libghostty_vt::{
    fmt::{Format, Formatter, FormatterOptions},
    Terminal,
};

pub fn snapshot(term: &Terminal<'static, 'static>) -> Vec<u8> {
    // (No selection: the whole screen and scrollback. No tab stops: setting
    // them moves the cursor after it has been put back. No palette: the
    // window's theme decides the colors.)
    let opts = FormatterOptions::new()
        .with_format(Format::Vt)
        .with_unwrap(false)
        .with_trim(true)
        .with_style(true)
        .with_hyperlink(true)
        .with_modes(true)
        .with_cursor(true)
        .with_scrolling_region(true)
        .with_pwd(true)
        .with_keyboard(true)
        .with_charsets(true)
        .with_kitty_keyboard(true)
        .with_protection(true);
    let Ok(mut f) = Formatter::new(term, opts) else { return Vec::new() };
    f.format_alloc(None).map(|b| b.to_vec()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use libghostty_vt::terminal::Options;

    fn plain(t: &Terminal<'static, 'static>) -> String {
        let r = crate::find::rows(t);
        r.lines.join("\n")
    }

    #[test]
    fn round_trip() {
        let mut a = Terminal::new(Options { cols: 40, rows: 10, max_scrollback: 1 << 20 }).unwrap();
        for i in 0..30 {
            a.vt_write(format!("\x1b[3{}mline {i}\x1b[0m plain\r\n", i % 7 + 1).as_bytes());
        }
        a.vt_write(b"\x1b[1mbold\x1b[0m prompt $ ");
        let snap = snapshot(&a);
        assert!(!snap.is_empty());
        let mut b = Terminal::new(Options { cols: 40, rows: 10, max_scrollback: 1 << 20 }).unwrap();
        b.vt_write(&snap);
        assert_eq!(plain(&a), plain(&b));
        assert_eq!((a.cursor_x().unwrap(), a.cursor_y().unwrap()), (b.cursor_x().unwrap(), b.cursor_y().unwrap()));
        assert_eq!(a.scrollback_rows().unwrap(), b.scrollback_rows().unwrap());
    }

    // A real interactive bash, resized through zoom steps the way the window
    // does: the screen afterwards should hold one prompt, not a stack of them.
    #[test]
    #[ignore]
    fn zoom_bash() {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize};
        use std::io::Read;
        let sz = |r: u16, c: u16| PtySize { rows: r, cols: c, pixel_width: 0, pixel_height: 0 };
        let pair = native_pty_system().openpty(sz(24, 80)).unwrap();
        let mut cmd = CommandBuilder::new(std::env::var("ZOOM_SHELL").unwrap_or("bash".into()));
        cmd.arg("-il");
        cmd.env("TERM", "xterm-256color");
        if std::env::var_os("ZOOM_INJECT").is_some() { crate::shell_integration::inject(&mut cmd) }
        let mut child = pair.slave.spawn_command(cmd).unwrap();
        let mut rd = pair.master.try_clone_reader().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut b = [0u8; 65536];
            while let Ok(n) = rd.read(&mut b) {
                if n == 0 { break }
                if tx.send(b[..n].to_vec()).is_err() { break }
            }
        });
        let mut t = Terminal::new(Options { cols: 80, rows: 24, max_scrollback: 1 << 22 }).unwrap();
        t.resize(80, 24, 8, 16).unwrap();
        let pump = |t: &mut Terminal<'static, 'static>, ms: u64| {
            let end = std::time::Instant::now() + std::time::Duration::from_millis(ms);
            while let Some(left) = end.checked_duration_since(std::time::Instant::now()) {
                if let Ok(d) = rx.recv_timeout(left) { t.vt_write(&d) }
            }
        };
        pump(&mut t, 2500);
        println!("== start\n{}", plain(&t));
        let start_y = t.cursor_y().unwrap();
        for (c, r) in [(72u16, 21u16), (65, 19), (58, 17), (65, 19), (72, 21), (80, 24), (88, 26), (80, 24)] {
            t.resize(c, r, 8, 16).unwrap();
            pair.master.resize(sz(r, c)).unwrap();
            pump(&mut t, 600);
            println!("== {c}x{r} cursor y {:?}\n{}", t.cursor_y(), plain(&t));
        }
        println!("----\n{}\n----", plain(&t));
        if std::env::var_os("ZOOM_INJECT").is_some() {
            assert_eq!(t.cursor_y().unwrap(), start_y, "the prompt moved");
        }
        let _ = child.kill();
    }
}

/// libghostty limits scrollback in bytes, not lines: a row takes about
/// 9.3 bytes per column (measured). Budget for 300 columns so at least
/// `lines` rows are kept at any usual width (more when narrower).
pub fn scrollback_bytes(lines: u32) -> usize {
    if lines >= 10_000_000 {
        return usize::MAX;
    }
    (lines as usize).saturating_mul(2_900) + (1 << 20)
}
