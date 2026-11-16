//! A terminal's whole state as VT sequences (screen, scrollback, colors,
//! cursor, modes), so a fresh terminal fed with it looks exactly the same.
//! The daemon keeps a terminal per session and sends this on attach.

use libghostty_vt::{
    fmt::{Format, Formatter, FormatterOptions},
    Terminal,
};

pub fn snapshot(term: &Terminal<'static, 'static>) -> Vec<u8> {
    // (No selection: the whole screen and scrollback. No tab stops: setting
    // them moves the cursor after it has been put back.)
    let opts = FormatterOptions::new()
        .with_format(Format::Vt)
        .with_unwrap(false)
        .with_trim(true)
        .with_style(true)
        .with_hyperlink(true)
        .with_palette(true)
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
}

