//! The forms the reader makes and macros take and return.

/// Where a form is: line from 1, columns in UTF-8 bytes from 0 (what
/// Python's AST counts).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Pos {
    pub line: u32,
    pub col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

#[derive(Clone, Debug)]
pub enum Part {
    Text(String),
    F(Form),
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// A bareword; where it means a name, `marks` are its hygiene marks (sorted).
    Sym { name: String, marks: Vec<u32> },
    /// $name (a Sym, or an Unquote inside a template).
    Var(Box<Form>),
    /// $?
    Status,
    Num(String),
    /// A string with nothing to interpolate: kind '"', '\'', 'r' or 'b'.
    Str { value: String, kind: char },
    /// A double-quoted string with interpolations.
    IStr(Vec<Part>),
    /// Glued pieces: text (globbed and ~-expanded in commands) and forms.
    Word(Vec<Part>),
    /// $name, $(form) or ${expr!conv:spec} inside a string or word.
    Interp { expr: Box<Form>, conv: Option<char>, spec: Option<String>, braced: bool },
    /// A spaced operator, or `name` (tick) for a function used infix.
    Op { name: String, tick: bool },
    Redir { op: String, target: Option<Box<Form>> },
    KwArg { name: String, value: Box<Form> },
    Splat { value: Box<Form>, n: u8 },
    Caret(Box<Form>),
    SymLit(Box<Form>),
    Quasi(Box<Form>),
    Unquote { form: Box<Form>, splice: bool },
    Attr { obj: Box<Form>, name: String },
    Index { obj: Box<Form>, items: Vec<Form> },
    Unary { op: String, operand: Box<Form> },
    Group { open: char, items: Vec<Form> },
    Comma,
    Semi,
    Line { items: Vec<Form>, children: Vec<Form> },
}

#[derive(Clone, Debug)]
pub struct Form {
    pub kind: Kind,
    pub pos: Option<Pos>,
}

impl Form {
    pub fn new(kind: Kind, pos: Option<Pos>) -> Form {
        Form { kind, pos }
    }

    pub fn sym(name: &str) -> Form {
        Form::new(Kind::Sym { name: name.to_string(), marks: vec![] }, None)
    }

    pub fn sym_at(name: &str, marks: &[u32], pos: Option<Pos>) -> Form {
        Form::new(Kind::Sym { name: name.to_string(), marks: marks.to_vec() }, pos)
    }

    pub fn line(items: Vec<Form>, children: Vec<Form>, pos: Option<Pos>) -> Form {
        Form::new(Kind::Line { items, children }, pos)
    }

    pub fn group(open: char, items: Vec<Form>, pos: Option<Pos>) -> Form {
        Form::new(Kind::Group { open, items }, pos)
    }

    /// The symbol's name, if it's a symbol.
    pub fn sym_name(&self) -> Option<&str> {
        match &self.kind {
            Kind::Sym { name, .. } => Some(name),
            _ => None,
        }
    }

    pub fn is_sym(&self, n: &str) -> bool {
        self.sym_name() == Some(n)
    }

    pub fn marks(&self) -> &[u32] {
        match &self.kind {
            Kind::Sym { marks, .. } => marks,
            _ => &[],
        }
    }

    pub fn op_name(&self) -> Option<&str> {
        match &self.kind {
            Kind::Op { name, tick: false } => Some(name),
            _ => None,
        }
    }

    pub fn is_op(&self, n: &str) -> bool {
        self.op_name() == Some(n)
    }

    pub fn is_comma(&self) -> bool {
        matches!(self.kind, Kind::Comma)
    }

    pub fn type_name(&self) -> &'static str {
        match &self.kind {
            Kind::Sym { .. } => "Sym",
            Kind::Var(_) => "Var",
            Kind::Status => "Status",
            Kind::Num(_) => "Num",
            Kind::Str { .. } => "Str",
            Kind::IStr(_) => "IStr",
            Kind::Word(_) => "Word",
            Kind::Interp { .. } => "Interp",
            Kind::Op { .. } => "Op",
            Kind::Redir { .. } => "Redir",
            Kind::KwArg { .. } => "KwArg",
            Kind::Splat { .. } => "Splat",
            Kind::Caret(_) => "Caret",
            Kind::SymLit(_) => "SymLit",
            Kind::Quasi(_) => "Quasi",
            Kind::Unquote { .. } => "Unquote",
            Kind::Attr { .. } => "Attr",
            Kind::Index { .. } => "Index",
            Kind::Unary { .. } => "Unary",
            Kind::Group { .. } => "Group",
            Kind::Comma => "Comma",
            Kind::Semi => "Semi",
            Kind::Line { .. } => "Line",
        }
    }

    /// Apply f to every form inside (children first), then to this one.
    pub fn map(self, f: &mut dyn FnMut(Form) -> Form) -> Form {
        let pos = self.pos;
        let mut m = |x: Box<Form>, f: &mut dyn FnMut(Form) -> Form| Box::new(x.map(f));
        let parts = |ps: Vec<Part>, f: &mut dyn FnMut(Form) -> Form| -> Vec<Part> {
            ps.into_iter().map(|p| match p {
                Part::F(x) => Part::F(x.map(f)),
                t => t,
            }).collect()
        };
        let list = |xs: Vec<Form>, f: &mut dyn FnMut(Form) -> Form| -> Vec<Form> { xs.into_iter().map(|x| x.map(f)).collect() };
        let kind = match self.kind {
            Kind::Var(x) => Kind::Var(m(x, f)),
            Kind::IStr(ps) => Kind::IStr(parts(ps, f)),
            Kind::Word(ps) => Kind::Word(parts(ps, f)),
            Kind::Interp { expr, conv, spec, braced } => Kind::Interp { expr: m(expr, f), conv, spec, braced },
            Kind::Redir { op, target } => Kind::Redir { op, target: target.map(|t| m(t, f)) },
            Kind::KwArg { name, value } => Kind::KwArg { name, value: m(value, f) },
            Kind::Splat { value, n } => Kind::Splat { value: m(value, f), n },
            Kind::Caret(x) => Kind::Caret(m(x, f)),
            Kind::SymLit(x) => Kind::SymLit(m(x, f)),
            Kind::Quasi(x) => Kind::Quasi(m(x, f)),
            Kind::Unquote { form, splice } => Kind::Unquote { form: m(form, f), splice },
            Kind::Attr { obj, name } => Kind::Attr { obj: m(obj, f), name },
            Kind::Index { obj, items } => Kind::Index { obj: m(obj, f), items: list(items, f) },
            Kind::Unary { op, operand } => Kind::Unary { op, operand: m(operand, f) },
            Kind::Group { open, items } => Kind::Group { open, items: list(items, f) },
            Kind::Line { items, children } => Kind::Line { items: list(items, f), children: list(children, f) },
            k => k,
        };
        f(Form { kind, pos })
    }

    /// Toggle a hygiene mark on every symbol.
    pub fn toggle_mark(self, mark: u32) -> Form {
        self.map(&mut |x| match x.kind {
            Kind::Sym { name, mut marks } => {
                if let Some(i) = marks.iter().position(|m| *m == mark) {
                    marks.remove(i);
                } else {
                    marks.push(mark);
                    marks.sort();
                }
                Form { kind: Kind::Sym { name, marks }, pos: x.pos }
            }
            k => Form { kind: k, pos: x.pos },
        })
    }

    /// Give forms without a position this one.
    pub fn positioned(self, pos: Option<Pos>) -> Form {
        if pos.is_none() {
            return self;
        }
        self.map(&mut |mut x| {
            if x.pos.is_none() {
                x.pos = pos;
            }
            x
        })
    }
}

// ---- showing forms as tessh ----------------------------------------------------

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('$', "\\$").replace('\n', "\\n")
}

pub fn show(f: &Form) -> String {
    show_indented(f, 0)
}

pub fn show_indented(f: &Form, indent: usize) -> String {
    match &f.kind {
        Kind::Line { items, children } => {
            let mut s = " ".repeat(indent) + &items.iter().map(show1).collect::<Vec<_>>().join(" ");
            for c in children {
                s += "\n";
                s += &show_indented(c, indent + 2);
            }
            s
        }
        _ => show1(f),
    }
}

fn show_parts(ps: &[Part], quoted: bool) -> String {
    ps.iter()
        .map(|p| match p {
            Part::Text(t) => if quoted { esc(t) } else { t.clone() },
            Part::F(x) => show1(x),
        })
        .collect()
}

pub fn show1(f: &Form) -> String {
    match &f.kind {
        Kind::Sym { name, .. } => name.clone(),
        Kind::Var(s) => format!("${}", show1(s)),
        Kind::Status => "$?".into(),
        Kind::Num(t) => t.clone(),
        Kind::Str { value, kind } => match kind {
            'b' => format!("b\"{}\"", esc(value)),
            '\'' | 'r' if !value.contains('\'') => format!("'{value}'"),
            _ => format!("\"{}\"", esc(value)),
        },
        Kind::IStr(ps) => format!("\"{}\"", show_parts(ps, true)),
        Kind::Word(ps) => show_parts(ps, false),
        Kind::Interp { expr, conv, spec, braced } => {
            let inner = match &expr.kind {
                Kind::Group { items, .. } => items.iter().map(show1).collect::<Vec<_>>().join(" "),
                _ => return show1(expr),
            };
            if !braced {
                format!("$({inner})")
            } else {
                format!("${{{inner}{}{}}}", conv.map(|c| format!("!{c}")).unwrap_or_default(), spec.as_ref().map(|s| format!(":{s}")).unwrap_or_default())
            }
        }
        Kind::Op { name, tick } => if *tick { format!("`{name}`") } else { name.clone() },
        Kind::Redir { op, target } => format!("{op}{}", target.as_ref().map(|t| show1(t)).unwrap_or_default()),
        Kind::KwArg { name, value } => format!("{name}={}", show1(value)),
        Kind::Splat { value, n } => format!("{}{}", "*".repeat(*n as usize), show1(value)),
        Kind::Caret(w) => format!("\\{}", show1(w)),
        Kind::SymLit(s) => format!(":{}", show1(s)),
        Kind::Quasi(x) => format!(":{}", show1(x)),
        Kind::Unquote { form, splice } => format!("{}{}", if *splice { "~@" } else { "~" }, show1(form)),
        Kind::Attr { obj, name } => format!("{}.{name}", show1(obj)),
        Kind::Index { obj, items } => format!("{}[{}]", show1(obj), show_items(items)),
        Kind::Unary { op, operand } => format!("{op}{}", show1(operand)),
        Kind::Group { open, items } => {
            let close = match open { '(' => ')', '[' => ']', _ => '}' };
            format!("{open}{}{close}", show_items(items))
        }
        Kind::Comma => ",".into(),
        Kind::Semi => ";".into(),
        Kind::Line { .. } => format!("({})", show(f)),
    }
}

fn show_items(items: &[Form]) -> String {
    let mut out = String::new();
    for i in items {
        match i.kind {
            Kind::Comma => out.push(','),
            Kind::Semi => out.push(';'),
            _ => {
                if !out.is_empty() {
                    out.push(' ');
                }
                out += &show1(i);
            }
        }
    }
    out
}
