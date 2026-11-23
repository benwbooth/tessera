//! tessh's line editor for terminals: Tessera's input editor
//! (qml/ComposeBox.qml, qml/PromptBar.qml) drawn with text, so it looks and
//! behaves the same in any terminal: its toolbar of buttons, the prompt bar
//! (Starship or Tessera's widgets), highlighting, line numbers, indent guides,
//! the syntax check, completion with each item's help, history and folder
//! lists, suggestions from history and the AI, the Emacs, vi and VS Code keys,
//! and the mouse (click, drag to select, the wheel, hover for tooltips).
//! Drawn inline at the bottom like any shell's prompt: no alternate screen,
//! the terminal's scrollback stays as it is.

pub mod core;
pub mod run;
pub mod screen;
pub mod term;
pub mod text;
pub mod view;
pub mod promptbar;

/// A color: one of the terminal's 16 (its theme's), or RGB.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Col {
    Ansi(u8),
    Rgb(u8, u8, u8),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Sty {
    pub fg: Option<Col>,
    pub bg: Option<Col>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    /// A wavy underline in this color (an error or warning).
    pub curl: Option<Col>,
}

impl Sty {
    pub fn fg(c: Col) -> Sty {
        Sty { fg: Some(c), ..Default::default() }
    }
    pub fn with_bg(mut self, c: Option<Col>) -> Sty {
        if self.bg.is_none() {
            self.bg = c;
        }
        self
    }
    pub fn bold(mut self) -> Sty {
        self.bold = true;
        self
    }
    pub fn dim(mut self) -> Sty {
        self.dim = true;
        self
    }
}

/// What a cell is, for the mouse: a click, a drag, hovering.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum Hit {
    #[default]
    None,
    /// A position in the text (a byte offset).
    Text(usize),
    /// A toolbar button, by name ("history", "folders", "lang", "guide",
    /// "indent", "jobs", "keys", "fix", "queued", "hint").
    Chip(&'static str),
    /// A popup's row (completion item, history entry, choice, menu item).
    Row(usize),
    /// A popup's help panel (the wheel scrolls it).
    Doc,
    /// A folder in the prompt bar's path: a click goes there.
    Path(String),
    /// A prompt bar widget ("dir", "git", "starship", "star:<module>", …).
    Widget(String),
}

/// A run of text with one look and one meaning for the mouse.
#[derive(Clone, Debug, Default)]
pub struct Span {
    pub text: String,
    pub sty: Sty,
    pub hit: Hit,
}

impl Span {
    pub fn new(text: impl Into<String>, sty: Sty) -> Span {
        Span { text: text.into(), sty, hit: Hit::None }
    }
    pub fn hit(mut self, h: Hit) -> Span {
        self.hit = h;
        self
    }
}

pub type Line = Vec<Span>;

/// The display width of a line.
pub fn width(l: &[Span]) -> usize {
    l.iter().map(|s| unicode_width::UnicodeWidthStr::width(s.text.as_str())).sum()
}
