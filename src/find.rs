//! Find in scrollback: search every row (history and screen) of a terminal.
//!
//! Rows come from libghostty's plain-text formatter, one line per row
//! (soft wraps kept), so a match's position is (row from the top of the
//! scrollback, cell columns). Columns use libghostty's own character
//! widths, so wide characters line up with what's drawn.

use libghostty_vt::{
    fmt::{Format, Formatter, FormatterOptions},
    terminal::{Point, PointSpace},
    Terminal,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    /// Row counted from the top of the scrollback.
    pub row: usize,
    /// First cell column.
    pub start: u16,
    /// One past the last cell column.
    pub end: u16,
}

/// Row in screen space (0 = top of the scrollback) of a grid point.
pub fn screen_row(term: &Terminal<'static, 'static>, point: Point) -> Option<usize> {
    let r = term.grid_ref(point).ok()?;
    term.point_from_grid_ref(&r, PointSpace::Screen).ok().flatten().map(|p| p.y as usize)
}

/// Every row's text, oldest first, with where the text starts: the screen
/// row of the first line, and the column it starts at (the selection starts
/// at the first non-blank cell, so leading blank rows and the first line's
/// indent aren't in the text).
pub struct Rows {
    pub first: usize,
    pub first_col: u16,
    pub lines: Vec<String>,
}

pub fn rows(term: &Terminal<'static, 'static>) -> Rows {
    let empty = Rows { first: 0, first_col: 0, lines: Vec::new() };
    let Ok(Some(sel)) = term.select_all() else { return empty };
    let start = term.point_from_grid_ref(&sel.start(), PointSpace::Screen).ok().flatten();
    let first = start.map(|p| p.y as usize).unwrap_or(0);
    let first_col = start.map(|p| p.x).unwrap_or(0);
    let opts = FormatterOptions::new().with_format(Format::Plain).with_selection(&sel).with_unwrap(false).with_trim(true);
    let Ok(mut f) = Formatter::new(term, opts) else { return empty };
    let Ok(bytes) = f.format_alloc(None) else { return empty };
    Rows { first, first_col, lines: String::from_utf8_lossy(&bytes).split('\n').map(str::to_string).collect() }
}

/// Build the matcher: plain text unless `regex`; smart case (case-insensitive
/// unless the query has a capital letter).
pub fn compile(query: &str, regex: bool) -> Result<regex::Regex, String> {
    let pattern = if regex { query.to_string() } else { regex::escape(query) };
    let insensitive = !query.chars().any(char::is_uppercase);
    regex::RegexBuilder::new(&pattern).case_insensitive(insensitive).build().map_err(|e| e.to_string())
}

fn col_of(line: &str, byte: usize) -> u16 {
    line[..byte].chars().map(|c| libghostty_vt::unicode::codepoint_width(c) as u16).sum()
}

pub fn search(rows: &Rows, re: &regex::Regex) -> Vec<Match> {
    let mut out = Vec::new();
    for (i, line) in rows.lines.iter().enumerate() {
        let row = rows.first + i;
        let shift = if i == 0 { rows.first_col } else { 0 };
        for m in re.find_iter(line) {
            if m.start() == m.end() {
                continue;
            }
            let start = shift + col_of(line, m.start());
            let end = start + col_of(&line[m.start()..], m.end() - m.start());
            out.push(Match { row, start, end: end.max(start + 1) });
        }
    }
    out
}

/// What iTerm2's Find URLs looks for (the QML find bar uses the same pattern).
#[cfg(test)]
pub const URL_PATTERN: &str = r#"(?:https?|ftp|file|ssh)://[^\s"'<>()]+|www\.[^\s"'<>()]+"#;

#[cfg(test)]
mod tests {
    use super::*;
    use libghostty_vt::terminal::Options;

    fn term_with(text: &str) -> Terminal<'static, 'static> {
        let mut t = Terminal::new(Options { cols: 20, rows: 5, max_scrollback: 1000 }).unwrap();
        t.vt_write(text.as_bytes());
        t
    }

    #[test]
    fn one_line_per_row_from_the_top() {
        let mut s = String::new();
        for i in 0..12 {
            s.push_str(&format!("line {i}\r\n"));
        }
        let t = term_with(&s);
        let r = rows(&t);
        assert_eq!(r.first, 0);
        assert_eq!(r.lines[0], "line 0");
        assert_eq!(r.lines[11], "line 11");
        assert!(r.lines.len() <= t.total_rows().unwrap());
    }

    #[test]
    fn wrapped_rows_stay_separate() {
        // 30 characters on a 20-column terminal wrap onto a second row.
        let t = term_with("abcdefghijklmnopqrstUVWXYZ0123\r\nnext");
        let r = rows(&t);
        assert_eq!(r.lines[0], "abcdefghijklmnopqrst");
        assert_eq!(r.lines[1], "UVWXYZ0123");
        assert_eq!(r.lines[2], "next");
    }

    #[test]
    fn columns_count_wide_characters() {
        let t = term_with("日本 needle\r\n");
        let m = search(&rows(&t), &compile("needle", false).unwrap());
        assert_eq!(m, vec![Match { row: 0, start: 5, end: 11 }]);
    }

    #[test]
    fn leading_blank_rows_keep_true_row_numbers() {
        let t = term_with("\r\n\r\n  needle\r\n   needle\r\n");
        let m = search(&rows(&t), &compile("needle", false).unwrap());
        assert_eq!(m, vec![Match { row: 2, start: 2, end: 8 }, Match { row: 3, start: 3, end: 9 }]);
    }

    #[test]
    fn smart_case_and_regex() {
        let lines = vec!["Error error ERROR".to_string()];
        assert_eq!(search(&Rows { first: 0, first_col: 0, lines: lines.clone() }, &compile("error", false).unwrap()).len(), 3);
        assert_eq!(search(&Rows { first: 0, first_col: 0, lines: lines.clone() }, &compile("Error", false).unwrap()).len(), 1);
        assert_eq!(search(&Rows { first: 0, first_col: 0, lines: lines.clone() }, &compile("e.r", true).unwrap()).len(), 3);
        assert_eq!(search(&Rows { first: 0, first_col: 0, lines: lines.clone() }, &compile("^error", true).unwrap()).len(), 1);
        assert_eq!(search(&Rows { first: 0, first_col: 0, lines: lines.clone() }, &compile("e.r", false).unwrap()).len(), 0);
        assert!(compile("(", true).is_err());
        assert_eq!(search(&Rows { first: 0, first_col: 0, lines: lines.clone() }, &compile("(", false).unwrap()).len(), 0);
    }

    #[test]
    fn urls() {
        let lines = vec!["see https://example.com/a?b=1 and www.x.org.".to_string()];
        let re = compile(URL_PATTERN, true).unwrap();
        assert_eq!(search(&Rows { first: 0, first_col: 0, lines: lines.clone() }, &re).len(), 2);
    }
}

