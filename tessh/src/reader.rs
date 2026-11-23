//! The tessh reader, made with chumsky (a parser-combinator library).
//!
//! The chumsky grammar reads logical lines: each line's indentation and its
//! items (terms, groups that may span lines, strings with interpolation).
//! Then the lines become blocks by their indentation. `read(text)` gives the
//! Line forms.

use crate::forms::{Form, Kind, Part, Pos};
use chumsky::prelude::*;

type Ex<'a> = extra::Err<Rich<'a, char>>;

#[derive(Debug, Clone)]
pub struct ReadError {
    pub msg: String,
    pub line: u32,
    /// Columns in characters from 0 (for people and editors).
    pub col: u32,
    pub end_col: u32,
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col + 1, self.msg)
    }
}

const DELIMS: &str = " \t\r\n()[]{}\"'`;$,";
/// What a backslash makes literal in a bare word: 7th\ Heaven, \$5, \*.
const ESCAPABLE: &str = " \t()[]{}\"'`;,$#*?&|<>";
const CLOSERS: &str = ")]}";
pub const OPCHARS: &str = "+-*/<>=!&|%^~@?:.";

fn sp(s: SimpleSpan) -> Option<Pos> {
    Some(Pos { line: 0, col: s.start as u32, end_line: 0, end_col: s.end as u32 })
}

fn span_of(a: &Option<Pos>, b: &Option<Pos>) -> Option<Pos> {
    match (a, b) {
        (Some(a), Some(b)) => Some(Pos { line: 0, col: a.col, end_line: 0, end_col: b.end_col }),
        _ => *a,
    }
}

#[derive(Clone, Debug)]
enum Piece {
    Text(String),
    F(Form),
}

#[derive(Clone, Debug)]
pub struct RawLine {
    pub indent: usize,
    pub items: Vec<Form>,
    pub start: usize,
}

fn escape_char(c: char) -> Option<char> {
    Some(match c {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '0' => '\0',
        'a' => '\x07',
        'b' => '\x08',
        'f' => '\x0c',
        'v' => '\x0b',
        'e' => '\x1b',
        '\\' | '"' | '\'' | '$' | '`' => c,
        _ => return None,
    })
}

/// The grammar of a whole text: its logical lines.
fn lines_parser<'a>() -> impl Parser<'a, &'a str, Vec<RawLine>, Ex<'a>> {
    let ws = choice((one_of(" \t").ignored(), just('\\').then(just('\r').or_not()).then(just('\n')).ignored()))
        .repeated()
        .at_least(1)
        .labelled("space");
    // (`#_` isn't a comment: it drops the next form.)
    let discard = just("#_").then(just('_').not()).ignored();
    let comment = discard.clone().not().ignore_then(just('#')).then(none_of("\n").repeated()).ignored();
    let gws = choice((one_of(" \t\r\n").ignored(), just('\\').then(just('\n')).ignored(), comment))
        .repeated()
        .at_least(1);
    // (A name: a letter or _, then letters, digits and _, as in Python and
    // bash. chumsky's text::ascii::ident stops at a digit: $a1 was $a then 1.)
    let ident = any()
        .filter(|c: &char| c.is_ascii_alphabetic() || *c == '_')
        .then(any().filter(|c: &char| c.is_ascii_alphanumeric() || *c == '_').repeated())
        .to_slice()
        .boxed();

    let mut group_items = Recursive::declare();

    let group = |open: char, close: char, items: Recursive<_>| {
        just(open)
            .ignore_then(items)
            .then_ignore(just(close).labelled(match close { ')' => "')'", ']' => "']'", _ => "'}'" }))
            .map_with(move |items: Vec<Form>, e| Form::group(open, items, sp(e.span())))
    };
    let paren = group('(', ')', group_items.clone()).boxed();
    let bracket = group('[', ']', group_items.clone()).boxed();
    let brace = group('{', '}', group_items.clone()).boxed();

    // Glued postfixes: .name and [items].
    let postfix = choice((
        just('.').ignore_then(ident.clone()).map_with(|n: &str, e| (Some(n.to_string()), None, sp(e.span()))),
        bracket.clone().map_with(|g: Form, e| (None, Some(g), sp(e.span()))),
    ));
    let with_postfix = move |base: chumsky::Boxed<'a, 'a, &'a str, Form, Ex<'a>>| {
        base.foldl(postfix.clone().repeated(), |acc: Form, (name, idx, end)| {
            let pos = span_of(&acc.pos, &end);
            match (name, idx) {
                (Some(n), _) => Form::new(Kind::Attr { obj: Box::new(acc), name: n }, pos),
                (_, Some(Form { kind: Kind::Group { items, .. }, .. })) => Form::new(Kind::Index { obj: Box::new(acc), items }, pos),
                _ => acc,
            }
        })
        .boxed()
    };

    // ${…}: the inside, read later (the format spec needs the raw text).
    let braced = just("${")
        .ignore_then(custom(|inp| {
            let before = inp.cursor();
            let mut depth = 0i32;
            let mut quote: Option<char> = None;
            loop {
                match inp.peek() {
                    None => return Err(Rich::custom(inp.span_since(&before), "missing '}' to close the '${'")),
                    Some(c) => {
                        if let Some(q) = quote {
                            if c == '\\' {
                                inp.skip();
                            } else if c == q {
                                quote = None;
                            }
                        } else if c == '"' || c == '\'' {
                            quote = Some(c);
                        } else if "([{".contains(c) {
                            depth += 1;
                        } else if c == '}' && depth == 0 {
                            break;
                        } else if ")]}".contains(c) {
                            depth -= 1;
                        }
                        inp.skip();
                    }
                }
            }
            let s: &str = inp.slice_since(&before..);
            let span: SimpleSpan = inp.span_since(&before);
            Ok((s.to_string(), span.start))
        }))
        .then_ignore(just('}'))
        .map_with(|(body, start): (String, usize), e| {
            let raw = Form::new(Kind::Str { value: body, kind: '{' }, Some(Pos { line: 0, col: start as u32, end_line: 0, end_col: start as u32 }));
            Form::new(Kind::Interp { expr: Box::new(raw), conv: None, spec: None, braced: true }, sp(e.span()))
        })
        .boxed();

    let unquote_inner = choice((
        paren.clone(),
        just('$').ignore_then(ident.clone()).map_with(|n: &str, e| Form::new(Kind::Var(Box::new(Form::sym_at(n, &[], sp(e.span())))), sp(e.span()))).boxed(),
        ident.clone().map_with(|n: &str, e| Form::sym_at(n, &[], sp(e.span()))).boxed(),
    ));
    let unquote = just('~')
        .ignore_then(just('@').or_not())
        .then(unquote_inner)
        .map_with(|(splice, f), e| Form::new(Kind::Unquote { form: Box::new(f), splice: splice.is_some() }, sp(e.span())))
        .boxed();

    // What follows a $.
    let dollar = just('$')
        .ignore_then(choice((
            just('?').map_with(|_, e| Form::new(Kind::Status, sp(e.span()))).boxed(),
            // ($%, $%1…$%9, $%&: a short fn's arguments.)
            just('%').then(one_of("123456789&").or_not()).to_slice().map_with(|n: &str, e| Form::new(Kind::Var(Box::new(Form::sym_at(n, &[], sp(e.span())))), sp(e.span()))).boxed(),
            paren.clone().map_with(|g, e| Form::new(Kind::Interp { expr: Box::new(g), conv: None, spec: None, braced: false }, sp(e.span()))).boxed(),
            unquote.clone().map_with(|u, e| Form::new(Kind::Var(Box::new(u)), sp(e.span()))).boxed(),
            ident.clone().map_with(|n: &str, e| Form::sym_at(n, &[], sp(e.span()))).map_with(|s, e| Form::new(Kind::Var(Box::new(s)), sp(e.span()))).boxed(),
        )))
        .map_with(|f, e| {
            let mut f = f;
            if let Some(p) = &mut f.pos {
                p.col = e.span().start as u32;
            }
            f
        })
        .boxed();
    let dollar_piece = choice((braced.clone(), with_postfix(dollar.clone())))
        .map(Piece::F)
        .or(just('$').to(Piece::Text("$".into())))
        .boxed();

    // Strings.
    let hexn = |n: usize| any().filter(|c: &char| c.is_ascii_hexdigit()).repeated().exactly(n).to_slice().map(|h: &str| char::from_u32(u32::from_str_radix(h, 16).unwrap()).unwrap_or('\u{fffd}'));
    let escape = just('\\')
        .ignore_then(choice((
            just('\n').to(None),
            just('x').ignore_then(hexn(2)).map(Some),
            just('u').ignore_then(hexn(4)).map(Some),
            just('U').ignore_then(hexn(8)).map(Some),
            any().map(|c: char| Some(escape_char(c).map(|x| x.to_string()).unwrap_or_else(|| format!("\\{c}")).chars().collect::<String>())).map(|s| s.and_then(|s| if s.chars().count() == 1 { s.chars().next() } else { None }).or(None)),
        )))
        .boxed();
    // (Escapes that aren't escapes keep their backslash.)
    let escape_text = just('\\').ignore_then(any()).try_map(|c: char, _| if escape_char(c).is_none() && !"xuU\n".contains(c) { Ok(format!("\\{c}")) } else { Err(Rich::custom(SimpleSpan::from(0..0), "")) });
    let interp_in_string = choice((braced.clone(), with_postfix(dollar.clone()))).boxed();
    let dq = |triple: bool, raw: bool, interp: bool| {
        let q = if triple { "\"\"\"" } else { "\"" };
        let stop = if triple { "\\$" } else { "\"\\$" };
        let text_char = if triple {
            just("\"\"\"").not().ignore_then(none_of(stop)).boxed()
        } else {
            none_of(stop).boxed()
        };
        let mut parts = text_char.repeated().at_least(1).collect::<String>().map(|s| Piece::Text(s)).boxed();
        if raw {
            parts = choice((parts, just('\\').then(any()).to_slice().map(|s: &str| Piece::Text(s.to_string())), just('$').to(Piece::Text("$".into())))).boxed();
        } else {
            parts = choice((
                parts,
                escape_text.clone().map(Piece::Text).boxed(),
                escape.clone().map(|c| Piece::Text(c.map(|c| c.to_string()).unwrap_or_default())).boxed(),
            ))
            .boxed();
            if interp {
                parts = choice((parts, interp_in_string.clone().map(Piece::F), just('$').to(Piece::Text("$".into())))).boxed();
            } else {
                parts = choice((parts, just('$').to(Piece::Text("$".into())))).boxed();
            }
        }
        just(q)
            .ignore_then(parts.repeated().collect::<Vec<Piece>>())
            .then_ignore(just(q).labelled(if triple { "'\"\"\"' to close the string" } else { "'\"' to close the string" }))
            .boxed()
    };
    let sq = |triple: bool| {
        let q = if triple { "'''" } else { "'" };
        let body = if triple { just("'''").not().ignore_then(any()).repeated().to_slice().boxed() } else { none_of("'").repeated().to_slice().boxed() };
        just(q).ignore_then(body).then_ignore(just(q).labelled(if triple { "\"'''\" to close the string" } else { "\"'\" to close the string" })).map(|s: &str| s.to_string()).boxed()
    };
    let pieces_to_string = |ps: Vec<Piece>, kind: char, pos| -> Form {
        if ps.iter().all(|p| matches!(p, Piece::Text(_))) {
            let s: String = ps.into_iter().map(|p| if let Piece::Text(t) = p { t } else { String::new() }).collect();
            Form::new(Kind::Str { value: s, kind }, pos)
        } else {
            let mut parts: Vec<Part> = vec![];
            for p in ps {
                match p {
                    Piece::Text(t) => match parts.last_mut() {
                        Some(Part::Text(prev)) => prev.push_str(&t),
                        _ => parts.push(Part::Text(t)),
                    },
                    Piece::F(f) => {
                        let f = if matches!(f.kind, Kind::Interp { .. }) { f } else {
                            let pos = f.pos;
                            Form::new(Kind::Interp { expr: Box::new(f), conv: None, spec: None, braced: false }, pos)
                        };
                        parts.push(Part::F(f))
                    }
                }
            }
            Form::new(Kind::IStr(parts), pos)
        }
    };
    let string = choice((
        // (re'…', re"…": a regex, raw — re'^\d+$' — with Perl's flags after
        // it: re'abc'i ignores case; i m s x a, as (?ims) at its start.)
        just("re")
            .ignore_then(choice((sq(true), sq(false))))
            .then(one_of("imsxa").repeated().collect::<String>())
            .map_with(|(s, fl), e| Form::new(Kind::Str { value: regex_flags(s, &fl), kind: 'x' }, sp(e.span())))
            .boxed(),
        just("re")
            .ignore_then(choice((dq(true, true, false), dq(false, true, false))))
            .then(one_of("imsxa").repeated().collect::<String>())
            .map_with(move |(ps, fl), e| {
                let mut f = pieces_to_string(ps, 'x', sp(e.span()));
                if let Kind::Str { value, .. } = &mut f.kind {
                    *value = regex_flags(std::mem::take(value), &fl);
                }
                f
            })
            .boxed(),
        one_of("bB").then(one_of("rR")).ignore_then(choice((dq(true, true, false), dq(false, true, false)))).map_with(move |ps, e| { let f = pieces_to_string(ps, 'b', sp(e.span())); f }).boxed(),
        one_of("rR").then(one_of("bB")).ignore_then(choice((dq(true, true, false), dq(false, true, false)))).map_with(move |ps, e| pieces_to_string(ps, 'b', sp(e.span()))).boxed(),
        one_of("bB").ignore_then(choice((dq(true, false, false), dq(false, false, false)))).map_with(move |ps, e| pieces_to_string(ps, 'b', sp(e.span()))).boxed(),
        one_of("rR").ignore_then(choice((dq(true, true, false), dq(false, true, false)))).map_with(move |ps, e| pieces_to_string(ps, 'r', sp(e.span()))).boxed(),
        one_of("rRbB").or_not().ignore_then(choice((sq(true), sq(false)))).map_with(|s, e| Form::new(Kind::Str { value: s, kind: '\'' }, sp(e.span()))).boxed(),
        choice((dq(true, false, true), dq(false, false, true))).map_with(move |ps, e| pieces_to_string(ps, '"', sp(e.span()))).boxed(),
    ))
    .boxed();
    let string_piece = with_postfix(string.clone()).map(Piece::F).boxed();

    let raw_text = choice((
        just('\\').then(one_of(ESCAPABLE)).not().ignore_then(none_of(DELIMS)).ignored(),
        just(',').then_ignore(none_of(" \t\r\n)]}").rewind()).ignored(),
    ))
    .repeated()
    .at_least(1)
    .to_slice()
    .map(|s: &str| Piece::Text(s.to_string()))
    .boxed();

    let balanced = |open: char, close: char| {
        just(open).rewind().ignore_then(custom(move |inp| {
            let before = inp.cursor();
            if inp.peek() != Some(open) {
                return Err(Rich::custom(inp.span_since(&before), format!("'{open}'")));
            }
            let mut depth = 0;
            loop {
                match inp.next() {
                    None => return Err(Rich::custom(inp.span_since(&before), format!("'{close}'"))),
                    Some(c) if c == open => depth += 1,
                    Some(c) if c == close => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    Some(c) if " \t\r\n".contains(c) => return Err(Rich::custom(inp.span_since(&before), format!("'{close}'"))),
                    _ => {}
                }
            }
            let s: &str = inp.slice_since(&before..);
            Ok(Piece::Text(s.to_string()))
        }))
    };

    // (\ and a space or special character: that character, as if quoted —
    // 7th\ Heaven/, a\(b, \$5, \*. A \ before a letter is \ls: the program.)
    let escaped = just('\\')
        .ignore_then(one_of(ESCAPABLE))
        .map_with(|c: char, e| Piece::F(Form::new(Kind::Str { value: c.to_string(), kind: '\'' }, sp(e.span()))))
        .boxed();
    let first_piece = choice((
        escaped.clone(),
        string_piece.clone(),
        dollar_piece.clone(),
        with_postfix(paren.clone()).map(Piece::F).boxed(),
        with_postfix(bracket.clone()).map(Piece::F).boxed(),
        with_postfix(brace.clone()).map(Piece::F).boxed(),
        just('#').not().ignore_then(raw_text.clone()).boxed(),
    ))
    .boxed();
    let more_piece = choice((
        escaped.clone(),
        string_piece.clone(),
        dollar_piece.clone(),
        with_postfix(paren.clone()).map(Piece::F).boxed(),
        balanced('[', ']').boxed(),
        balanced('{', '}').boxed(),
        just('[').to(Piece::Text("[".into())).boxed(),
        just('{').to(Piece::Text("{".into())).boxed(),
        just('`').to(Piece::Text("`".into())).boxed(),
        raw_text.clone(),
    ))
    .boxed();

    let delim_ahead = choice((one_of(" \t\r\n;,)]}").ignored(), end())).rewind();
    let tick_op = just('`')
        .ignore_then(none_of("` \t\r\n").repeated().at_least(1).to_slice())
        .then_ignore(just('`').labelled("'`' to close the operator"))
        .map_with(|n: &str, e| Form::new(Kind::Op { name: n.to_string(), tick: true }, sp(e.span())))
        .boxed();
    let term = first_piece
        .then(more_piece.repeated().collect::<Vec<Piece>>())
        .map_with(|(first, rest), e| {
            let mut ps = vec![first];
            ps.extend(rest);
            classify(merge(ps), sp(e.span()))
        })
        .boxed();
    let item = choice((
        just(',').map_with(|_, e| Form::new(Kind::Comma, sp(e.span()))).boxed(),
        just(';').map_with(|_, e| Form::new(Kind::Semi, sp(e.span()))).boxed(),
        discard.clone().map_with(|_, e| Form::sym_at("#_", &[], sp(e.span()))).boxed(),
        tick_op,
        unquote.clone().then_ignore(delim_ahead.clone()).boxed(),
        term,
    ))
    .boxed();

    group_items.define(gws.clone().or_not().ignore_then(item.clone().then_ignore(gws.clone().or_not()).repeated().collect::<Vec<Form>>()));

    let line_items = item.clone().then_ignore(ws.clone().or_not()).repeated().collect::<Vec<Form>>();
    let indent = one_of(" \t").repeated().to_slice().validate(|s: &str, e, emitter| {
        if s.contains('\t') {
            emitter.emit(Rich::custom(e.span(), "tabs can't indent (use spaces)"));
        }
        s.len()
    });
    let body = indent
        .then(line_items)
        .then_ignore(ws.clone().or_not())
        .then_ignore(comment.or_not())
        .map_with(|(indent, items), e| RawLine { indent, items, start: e.span().start });
    let newline = just('\r').or_not().then(just('\n')).ignored();
    body.clone()
        .then_ignore(newline)
        .repeated()
        .collect::<Vec<RawLine>>()
        .then(body.or_not())
        .then_ignore(end())
        .map(|(mut ls, last)| {
            ls.extend(last);
            ls
        })
}

fn merge(ps: Vec<Piece>) -> Vec<Piece> {
    let mut out: Vec<Piece> = vec![];
    for p in ps {
        match (out.last_mut(), p) {
            (Some(Piece::Text(prev)), Piece::Text(t)) => prev.push_str(&t),
            (_, p) => out.push(p),
        }
    }
    out
}

pub fn is_number(t: &str) -> bool {
    let s = t.strip_prefix(['-', '+']).unwrap_or(t);
    if s.is_empty() {
        return false;
    }
    let s = s.strip_suffix(['j', 'J']).unwrap_or(s);
    let ok_digits = |d: &str, radix: u32| !d.is_empty() && !d.starts_with('_') && !d.ends_with('_') && !d.contains("__") && d.chars().all(|c| c == '_' || c.is_digit(radix));
    if let Some(r) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return ok_digits(r, 16);
    }
    if let Some(r) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        return ok_digits(r, 8);
    }
    if let Some(r) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        return ok_digits(r, 2);
    }
    let (mant, exp) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    if let Some(e) = exp {
        let e = e.strip_prefix(['-', '+']).unwrap_or(e);
        if !ok_digits(e, 10) {
            return false;
        }
    }
    match mant.split_once('.') {
        Some((a, b)) => (a.is_empty() || ok_digits(a, 10)) && (b.is_empty() && !a.is_empty() || ok_digits(b, 10)) && !(a.is_empty() && b.is_empty()),
        None => ok_digits(mant, 10),
    }
}

pub fn is_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(ch) if ch.is_ascii_alphabetic() || ch == '_') && c.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// A redirection operator at the start of t: its length.
/// A redirection's operator at the start of t (its length; the target is
/// what follows): `>f`, `2>>f`, `3<f`, `<>f`, `>|f`, `<<<word`, `&>f`,
/// `>&f` (both, as `&>f`). Not `>=`, `<<` (a shift) and the like.
pub fn redir_op(t: &str) -> Option<usize> {
    for op in ["&>>", "&>"] {
        if let Some(rest) = t.strip_prefix(op) {
            return (!rest.starts_with(['<', '>', '=', '&'])).then_some(op.len());
        }
    }
    let d = t.bytes().take_while(|b| b.is_ascii_digit()).count();
    let r = &t[d..];
    for op in ["<<<", ">>", ">|", "<>", ">&", ">", "<"] {
        if let Some(rest) = r.strip_prefix(op) {
            if op == ">&" {
                // (>&2, >&-: whole ones; >&file: both to it.)
                return (d == 0 && !rest.is_empty() && !rest.starts_with(|c: char| c.is_ascii_digit() || c == '-')).then_some(2);
            }
            if rest.starts_with(['<', '>', '=', '&']) {
                return None;
            }
            return Some(d + op.len());
        }
    }
    None
}

/// A redirection with no file: `2>&1`, `>&2`, `3>&1`, `0<&3`, `2>&-`, `>&-`.
fn is_whole_redir(t: &str) -> bool {
    if t == "&>&1" {
        return true;
    }
    let d = t.bytes().take_while(|b| b.is_ascii_digit()).count();
    let r = &t[d..];
    let Some(m) = r.strip_prefix(">&").or_else(|| r.strip_prefix("<&")) else { return false };
    m == "-" || (!m.is_empty() && m.bytes().all(|b| b.is_ascii_digit()))
}

fn word(parts: Vec<Piece>, pos: Option<Pos>) -> Form {
    Form::new(Kind::Word(parts.into_iter().map(|p| match p { Piece::Text(t) => Part::Text(t), Piece::F(f) => Part::F(f) }).collect()), pos)
}

fn shift(pos: Option<Pos>, by: usize) -> Option<Pos> {
    pos.map(|p| Pos { col: p.col + by as u32, ..p })
}

fn valueish(f: &Form) -> bool {
    matches!(f.kind, Kind::Var(_) | Kind::Group { .. } | Kind::Attr { .. } | Kind::Index { .. })
}

fn classify_text(t: &str, pos: Option<Pos>) -> Form {
    classify(vec![Piece::Text(t.to_string())], pos)
}

fn classify(mut ps: Vec<Piece>, pos: Option<Pos>) -> Form {
    if ps.len() == 1 {
        if let Piece::F(_) = &ps[0] {
            if let Some(Piece::F(f)) = ps.pop() {
                return f;
            }
        }
    }
    let first = match &ps[0] {
        Piece::Text(t) => t.clone(),
        Piece::F(_) => return word(ps, pos),
    };
    let rest: Vec<Piece> = ps[1..].to_vec();
    if rest.is_empty() {
        let t = first.as_str();
        if is_number(t) {
            return Form::new(Kind::Num(t.to_string()), pos);
        }
        if is_whole_redir(t) {
            return Form::new(Kind::Redir { op: t.to_string(), target: None }, pos);
        }
        if t.chars().all(|c| OPCHARS.contains(c)) && !matches!(t, "~" | "?" | ":" | "@@") && !t.chars().all(|c| c == '.') {
            return Form::new(Kind::Op { name: t.to_string(), tick: false }, pos);
        }
        if let Some(n) = redir_op(t) {
            if n < t.len() {
                return Form::new(Kind::Redir { op: t[..n].to_string(), target: Some(Box::new(classify_text(&t[n..], shift(pos, n)))) }, pos);
            }
        }
        if let Some(r) = t.strip_prefix('\\') {
            if !r.is_empty() {
                return Form::new(Kind::Caret(Box::new(classify_text(r, shift(pos, 1)))), pos);
            }
        }
        if let Some(r) = t.strip_prefix(':') {
            if is_ident(r) {
                return Form::new(Kind::SymLit(Box::new(Form::sym_at(r, &[], shift(pos, 1)))), pos);
            }
        }
        if let Some(eq) = t.find('=') {
            let (n, v) = (&t[..eq], &t[eq + 1..]);
            if is_ident(n) && !v.is_empty() && !v.starts_with('=') {
                return Form::new(Kind::KwArg { name: n.to_string(), value: Box::new(classify_text(v, shift(pos, eq + 1))) }, pos);
            }
            // (name= with nothing after: the empty string, as bash's a= is.)
            if is_ident(n) && v.is_empty() {
                return Form::new(Kind::KwArg { name: n.to_string(), value: Box::new(Form::new(Kind::Str { value: String::new(), kind: '\'' }, shift(pos, eq + 1))) }, pos);
            }
        }
        return Form::new(Kind::Sym { name: t.to_string(), marks: vec![] }, pos);
    }
    // Prefixes glued to a form.
    if rest.len() == 1 {
        if let Piece::F(r) = &rest[0] {
            let r = r.clone();
            match first.as_str() {
                "-" | "+" if valueish(&r) => return Form::new(Kind::Unary { op: first, operand: Box::new(r) }, pos),
                "~" if matches!(r.kind, Kind::Var(_) | Kind::Attr { .. } | Kind::Index { .. }) => return Form::new(Kind::Unary { op: first, operand: Box::new(r) }, pos),
                "*" | "**" if valueish(&r) => return Form::new(Kind::Splat { value: Box::new(r), n: first.len() as u8 }, pos),
                ":" if matches!(&r.kind, Kind::Group { open: '(', .. }) => return Form::new(Kind::Quasi(Box::new(r)), pos),
                "\\" => return Form::new(Kind::Caret(Box::new(r)), pos),
                _ => {}
            }
        }
    }
    if let Some(n) = first.strip_suffix('=') {
        if is_ident(n) {
            let value = if rest.len() == 1 && matches!(rest[0], Piece::F(_)) {
                match &rest[0] { Piece::F(f) => f.clone(), _ => unreachable!() }
            } else {
                word(rest, pos)
            };
            return Form::new(Kind::KwArg { name: n.to_string(), value: Box::new(value) }, pos);
        }
    }
    if let Some(eq) = first.find('=') {
        let (n, v) = (&first[..eq], &first[eq + 1..]);
        if is_ident(n) && !v.is_empty() && !v.starts_with('=') {
            let mut ps = vec![Piece::Text(v.to_string())];
            ps.extend(rest);
            return Form::new(Kind::KwArg { name: n.to_string(), value: Box::new(word(ps, pos)) }, pos);
        }
    }
    // (A heredoc, as read_at has it: `<<` glued to the triple-quoted text.)
    if let Some(d) = first.strip_suffix("<<").filter(|d| d.bytes().all(|b| b.is_ascii_digit())) {
        if let [Piece::F(f)] = rest.as_slice() {
            if matches!(f.kind, Kind::Str { .. } | Kind::IStr(_)) {
                return Form::new(Kind::Redir { op: format!("{d}<<"), target: Some(Box::new(f.clone())) }, pos);
            }
        }
    }
    if let Some(n) = redir_op(&first) {
        {
            let mut tgt = vec![];
            if n < first.len() {
                tgt.push(Piece::Text(first[n..].to_string()));
            }
            tgt.extend(rest);
            let t = if tgt.len() == 1 && matches!(tgt[0], Piece::F(_)) { match tgt.pop() { Some(Piece::F(f)) => f, _ => unreachable!() } } else { word(tgt, pos) };
            return Form::new(Kind::Redir { op: first[..n].to_string(), target: Some(Box::new(t)) }, pos);
        }
    }
    if let Some(r) = first.strip_prefix('\\') {
        if !r.is_empty() {
            let mut ps = vec![Piece::Text(r.to_string())];
            ps.extend(rest);
            return Form::new(Kind::Caret(Box::new(word(ps, pos))), pos);
        }
    }
    word(ps, pos)
}

// ---- positions ------------------------------------------------------------------

struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    fn new(text: &'a str) -> Self {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        LineIndex { text, starts }
    }

    /// (line from 1, byte column) of a byte offset.
    fn at(&self, i: usize) -> (u32, u32) {
        let n = match self.starts.binary_search(&i) {
            Ok(n) => n,
            Err(n) => n - 1,
        };
        ((n + 1) as u32, (i - self.starts[n]) as u32)
    }

    fn char_col(&self, i: usize) -> u32 {
        let (line, _) = self.at(i);
        let start = self.starts[line as usize - 1];
        self.text.get(start..i.min(self.text.len())).map(|s| s.chars().count() as u32).unwrap_or(0)
    }
}

fn to_line_cols(f: Form, idx: &LineIndex, first_line: u32, base: usize) -> Form {
    f.map(&mut |mut x| {
        if let Some(p) = x.pos {
            if p.line == 0 {
                let (l1, c1) = idx.at(p.col as usize);
                let (l2, c2) = idx.at(p.end_col as usize);
                let _ = base;
                x.pos = Some(Pos { line: l1 + first_line - 1, col: c1, end_line: l2 + first_line - 1, end_col: c2 });
            }
        }
        x
    })
}

/// Read the ${…} bodies (kept as text so far).
fn read_braced(f: Form, text: &str) -> Result<Form, ReadError> {
    let mut err: Option<ReadError> = None;
    let f = f.map(&mut |x| {
        if let Kind::Interp { expr, conv: None, spec: None, braced: true } = &x.kind {
            if let Kind::Str { value, kind: '{' } = &expr.kind {
                let start = expr.pos.map(|p| p.col as usize).unwrap_or(0);
                let (body, conv, spec) = split_format(value);
                let inner = format!("({body})");
                let parsed = lines_parser().parse(inner.as_str()).into_result().map_err(|_| ());
                match parsed {
                    Ok(ls) => {
                        let items: Vec<Form> = ls.into_iter().flat_map(|l| l.items).collect();
                        let items = match items.into_iter().next() {
                            Some(Form { kind: Kind::Group { items, .. }, .. }) => items,
                            _ => vec![],
                        };
                        let items: Vec<Form> = items.into_iter().map(|i| shift_all(i, start as isize - 1)).collect();
                        let g = Form::group('(', items, Some(Pos { line: 0, col: start as u32, end_line: 0, end_col: (start + body.len()) as u32 }));
                        return Form::new(Kind::Interp { expr: Box::new(g), conv, spec, braced: true }, x.pos);
                    }
                    Err(_) => {
                        if err.is_none() {
                            let idx = LineIndex::new(text);
                            let (line, _) = idx.at(start);
                            err = Some(ReadError { msg: "can't read the inside of ${…}".into(), line, col: idx.char_col(start), end_col: idx.char_col(start) + 1 });
                        }
                    }
                }
            }
        }
        x
    });
    match err {
        Some(e) => Err(e),
        None => Ok(f),
    }
}

fn shift_all(f: Form, by: isize) -> Form {
    f.map(&mut |mut x| {
        if let Some(p) = &mut x.pos {
            if p.line == 0 {
                p.col = (p.col as isize + by) as u32;
                p.end_col = (p.end_col as isize + by) as u32;
            }
        }
        x
    })
}

/// "expr!r:>10" -> ("expr", Some('r'), Some(">10")), at the top level.
fn split_format(body: &str) -> (String, Option<char>, Option<String>) {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut cut = body.len();
    let mut spec = None;
    let chars: Vec<(usize, char)> = body.char_indices().collect();
    for (k, &(i, c)) in chars.iter().enumerate() {
        if let Some(q) = quote {
            if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ':' if depth == 0 && chars.get(k + 1).map(|x| x.1) != Some('=') => {
                spec = Some(body[i + 1..].to_string());
                cut = i;
                break;
            }
            _ => {}
        }
    }
    let mut expr = body[..cut].trim_end().to_string();
    let mut conv = None;
    if expr.len() >= 2 && expr.as_bytes()[expr.len() - 2] == b'!' && "rsa".contains(expr.chars().last().unwrap()) {
        conv = expr.chars().last();
        expr.truncate(expr.len() - 2);
    }
    (expr, conv, spec)
}

/// Where the bracket or quote that's never closed opens (a byte offset).
fn unclosed(text: &str) -> Option<usize> {
    let b = text.as_bytes();
    let mut stack = vec![];
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == b'#' && (i == 0 || b" \t\n([{".contains(&b[i - 1])) && !(b.get(i + 1) == Some(&b'_') && b.get(i + 2) != Some(&b'_')) {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == b'"' || c == b'\'' {
            let triple = text[i..].starts_with(if c == b'"' { "\"\"\"" } else { "'''" });
            let q = if triple { 3 } else { 1 };
            let start = i;
            i += q;
            loop {
                if i >= b.len() {
                    return Some(start);
                }
                if b[i] == b'\\' && c == b'"' {
                    i += 2;
                    continue;
                }
                if b[i] == c && (!triple || text[i..].starts_with(&text[start..start + 3])) {
                    i += q;
                    break;
                }
                i += 1;
            }
            continue;
        }
        if b"([{".contains(&c) {
            stack.push(i);
        } else if b")]}".contains(&c) {
            stack.pop();
        }
        i += 1;
    }
    stack.pop()
}

fn message(text: &str, e: &Rich<char>) -> (String, usize) {
    let at = e.span().start;
    let expected: Vec<String> = e.expected().map(|x| x.to_string()).collect();
    if let chumsky::error::RichReason::Custom(m) = e.reason() {
        if !m.is_empty() {
            return (m.to_string(), at);
        }
    }
    if let Some(close) = expected.iter().find(|x| x.contains("to close") || *x == "')'" || *x == "']'" || *x == "'}'") {
        if e.found().is_none() || text[at..].starts_with('\n') && !close.contains("string") {
            if let Some(o) = unclosed(text) {
                let what = if close.contains("to close") { close.clone() } else { format!("{close} to close the '{}'", &text[o..o + 1]) };
                return (format!("missing {what}"), o);
            }
        }
    }
    if let Some(c) = e.found() {
        if CLOSERS.contains(*c) {
            return (format!("unmatched '{c}'"), at);
        }
        return (format!("unexpected '{}'", c.escape_default()), at);
    }
    if let Some(o) = unclosed(text) {
        return (format!("missing a closing '{}'", match &text[o..o + 1] { "(" => ")", "[" => "]", "{" => "}", q => q }), o);
    }
    ("unexpected end".into(), at)
}

// ---- reading ---------------------------------------------------------------------

/// The lines of a tessh text, as Line forms with their blocks.
pub fn read(text: &str) -> Result<Vec<Form>, ReadError> {
    read_at(text, 1)
}

/// Heredocs (`cat <<EOF` … `EOF`, `<<-EOF` with tabs taken off, `<<'EOF'`
/// as it is): written as `<<"""…"""` (or `<<'''…'''`) for the reader,
/// the body's lines where they were, the rest of the line after the end
/// word. The text's first newline goes at run time.
fn heredocs(text: &str) -> Result<std::borrow::Cow<'_, str>, ReadError> {
    if !text.contains("<<") {
        return Ok(std::borrow::Cow::Borrowed(text));
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out: Vec<String> = vec![];
    let mut in_triple: Option<&str> = None;
    let mut i = 0;
    let mut changed = false;
    while i < lines.len() {
        let line = lines[i];
        // (Inside a triple-quoted string from a line before: as it is.)
        if let Some(q) = in_triple {
            if line.matches(q).count() % 2 == 1 {
                in_triple = None;
            }
            out.push(line.to_string());
            i += 1;
            continue;
        }
        let toks = heredoc_tokens(line);
        if toks.is_empty() {
            for q in ["\"\"\"", "'''"] {
                if line.matches(q).count() % 2 == 1 {
                    in_triple = Some(q);
                }
            }
            out.push(line.to_string());
            i += 1;
            continue;
        }
        changed = true;
        let mut cur = String::new();
        let mut at = 0;
        let mut j = i + 1;
        for (start, end, fd, strip, quoted, delim) in toks {
            cur.push_str(&line[at..start]);
            let q = if quoted { "'''" } else { "\"\"\"" };
            cur.push_str(&format!("{fd}<<{q}"));
            out.push(std::mem::take(&mut cur));
            loop {
                let Some(l) = lines.get(j) else {
                    return Err(ReadError { msg: format!("the heredoc never ends: no line {delim}"), line: i as u32 + 1, col: start as u32, end_col: end as u32 });
                };
                let l = l.strip_suffix('\r').unwrap_or(l);
                let body = if strip { l.trim_start_matches('\t') } else { l };
                j += 1;
                if body == delim {
                    break;
                }
                if quoted && body.contains("'''") {
                    return Err(ReadError { msg: "a quoted heredoc can't hold '''".into(), line: j as u32, col: 0, end_col: 1 });
                }
                out.push(if quoted { body.to_string() } else { heredoc_escape(body) });
            }
            cur.push_str(q);
            at = end;
        }
        cur.push_str(&line[at..]);
        out.push(cur);
        i = j;
    }
    Ok(if changed { std::borrow::Cow::Owned(out.join("\n")) } else { std::borrow::Cow::Borrowed(text) })
}

/// A heredoc body line for `"""…"""`: as bash has it, `$` interpolates and
/// a backslash is only special before `$`, `` ` `` and `\\`.
fn heredoc_escape(l: &str) -> String {
    let mut o = String::with_capacity(l.len());
    let cs: Vec<char> = l.chars().collect();
    let mut k = 0;
    while k < cs.len() {
        let c = cs[k];
        if c == '\\' {
            if matches!(cs.get(k + 1), Some('$' | '`' | '\\')) {
                o.push(c);
                o.push(cs[k + 1]);
                k += 2;
                continue;
            }
            o.push_str("\\\\");
        } else if c == '"' {
            o.push_str("\\\"");
        } else {
            o.push(c);
        }
        k += 1;
    }
    o
}

/// The heredocs a line starts, outside quotes: (start, end, fd, strip
/// tabs, quoted, end word).
fn heredoc_tokens(line: &str) -> Vec<(usize, usize, String, bool, bool, String)> {
    let b = line.as_bytes();
    let mut out = vec![];
    let mut quote: Option<u8> = None;
    let mut k = 0;
    while k < b.len() {
        let c = b[k];
        if let Some(q) = quote {
            if c == b'\\' && q == b'"' {
                k += 1;
            } else if c == q {
                quote = None;
            }
            k += 1;
            continue;
        }
        if c == b'#' && (k == 0 || b[k - 1] == b' ' || b[k - 1] == b'\t') {
            break;
        }
        if c == b'"' || c == b'\'' {
            quote = Some(c);
            k += 1;
            continue;
        }
        if c == b'<' && b.get(k + 1) == Some(&b'<') && b.get(k + 2) != Some(&b'<') && (k == 0 || b[k - 1] != b'<') {
            // (Where its word starts: after a space, with the fd's digits.)
            let mut s = k;
            while s > 0 && b[s - 1].is_ascii_digit() {
                s -= 1;
            }
            if s == 0 || b[s - 1] == b' ' || b[s - 1] == b'\t' {
                let mut p = k + 2;
                let strip = b.get(p) == Some(&b'-');
                if strip {
                    p += 1;
                }
                let (quoted, delim, end) = match b.get(p) {
                    Some(&q) if q == b'\'' || q == b'"' => match line[p + 1..].find(q as char) {
                        Some(e) if e > 0 => (true, line[p + 1..p + 1 + e].to_string(), p + 2 + e),
                        _ => (false, String::new(), p),
                    },
                    // (The word up to a space or one of ;|&<>(): bash's, so
                    // End-Of-Session and EOF.txt are delimiters too.)
                    Some(c) if c.is_ascii_alphabetic() || *c == b'_' => {
                        let e = line[p..].find(|ch: char| ch.is_whitespace() || ";|&<>()'\"".contains(ch)).map(|e| p + e).unwrap_or(line.len());
                        (false, line[p..e].to_string(), e)
                    }
                    _ => (false, String::new(), p),
                };
                if !delim.is_empty() {
                    out.push((s, end, line[s..k].to_string(), strip, quoted, delim));
                    k = end;
                    continue;
                }
            }
        }
        k += 1;
    }
    out
}

pub fn read_at(text: &str, first_line: u32) -> Result<Vec<Form>, ReadError> {
    let hd = heredocs(text).map_err(|e| ReadError { line: e.line + first_line - 1, ..e })?;
    let text: &str = &hd;
    let idx = LineIndex::new(text);
    let raw = match lines_parser().parse(text).into_result() {
        Ok(r) => r,
        Err(errs) => {
            let e = &errs[0];
            let (msg, at) = message(text, e);
            let (line, _) = idx.at(at);
            let col = idx.char_col(at);
            return Err(ReadError { msg, line: line + first_line - 1, col, end_col: col + 1 });
        }
    };
    // ; separates forms on a line (an indented block goes with the last).
    let mut lines: Vec<RawLine> = vec![];
    for l in raw {
        if l.items.is_empty() {
            continue;
        }
        let mut cur = vec![];
        for it in l.items {
            if matches!(it.kind, Kind::Semi) {
                if !cur.is_empty() {
                    lines.push(RawLine { indent: l.indent, items: std::mem::take(&mut cur), start: l.start });
                }
            } else {
                cur.push(it);
            }
        }
        if !cur.is_empty() {
            lines.push(RawLine { indent: l.indent, items: cur, start: l.start });
        }
    }
    let err_at = |l: &RawLine, msg: &str| {
        let (line, _) = idx.at(l.start);
        ReadError { msg: msg.into(), line: line + first_line - 1, col: l.indent as u32, end_col: l.indent as u32 + 1 }
    };
    if let Some(l) = lines.first() {
        if l.indent != 0 {
            return Err(err_at(l, "unexpected indentation"));
        }
    }
    fn block(lines: &[RawLine], i: &mut usize, level: usize, err: &dyn Fn(&RawLine, &str) -> ReadError) -> Result<Vec<Form>, ReadError> {
        let mut out = vec![];
        while *i < lines.len() && lines[*i].indent == level {
            let l = &lines[*i];
            *i += 1;
            let mut kids = vec![];
            if *i < lines.len() && lines[*i].indent > level {
                let child_level = lines[*i].indent;
                kids = block(lines, i, child_level, err)?;
                if *i < lines.len() && lines[*i].indent > level && lines[*i].indent < child_level {
                    return Err(err(&lines[*i], "this line's indentation doesn't match any line above it"));
                }
            }
            // (`#_` first on a line drops the line and its block.)
            if l.items[0].is_sym("#_") {
                continue;
            }
            let items = discard(l.items.clone());
            if items.is_empty() {
                continue;
            }
            let pos = span_of(&items[0].pos, &items[items.len() - 1].pos);
            out.push(Form::line(items, kids, pos));
        }
        Ok(out)
    }
    let mut i = 0;
    let forms = block(&lines, &mut i, 0, &err_at)?;
    if i < lines.len() {
        return Err(err_at(&lines[i], "this line's indentation doesn't match any line above it"));
    }
    let mut out = vec![];
    for f in forms {
        let f = read_braced(f, text)?;
        out.push(to_line_cols(f, &idx, first_line, 0));
    }
    Ok(out)
}

/// Drops each `#_` and the form after it (in brackets too).
fn discard(items: Vec<Form>) -> Vec<Form> {
    let mut out = vec![];
    let mut it = items.into_iter();
    while let Some(f) = it.next() {
        if f.is_sym("#_") {
            it.next();
            continue;
        }
        out.push(match f.kind {
            Kind::Group { open, items } => Form::new(Kind::Group { open, items: discard(items) }, f.pos),
            k => Form::new(k, f.pos),
        });
    }
    out
}

/// Does the text stop in the middle (an open bracket or string, or a trailing
/// backslash)? For the REPL.
pub fn incomplete(text: &str) -> bool {
    let t = text.trim_end_matches([' ', '\t']);
    if t.ends_with('\\') {
        return true;
    }
    match lines_parser().parse(text).into_result() {
        Ok(_) => false,
        Err(errs) => {
            let (msg, _) = message(text, &errs[0]);
            msg.starts_with("missing") && unclosed(text).is_some()
        }
    }
}

/// Character columns of a position in a text (for error reports).
pub fn char_cols(text: &str, line: u32, byte_col: u32) -> u32 {
    text.lines().nth(line.saturating_sub(1) as usize).map(|l| l.get(..(byte_col as usize).min(l.len())).map(|s| s.chars().count()).unwrap_or(byte_col as usize) as u32).unwrap_or(byte_col)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forms::show;

    fn items(src: &str) -> Vec<Form> {
        let ls = read(src).unwrap();
        assert_eq!(ls.len(), 1, "{src}");
        match &ls[0].kind {
            Kind::Line { items, .. } => items.clone(),
            _ => unreachable!(),
        }
    }

    fn one(src: &str) -> Form {
        items(src).remove(0)
    }

    #[test]
    fn debug_errors() {
        for src in ["x = [1\n  2\n     3]", "x = (1"] {
            let r = lines_parser().parse(src).into_result();
            if let Err(es) = r {
                for e in es {
                    eprintln!("{src:?}: {:?} reason={:?} expected={:?} found={:?}", e.span(), e.reason(), e.expected().collect::<Vec<_>>(), e.found());
                }
            }
        }
    }

    #[test]
    fn barewords_flags_and_paths() {
        for i in items("ls -la --all --color=auto *.py . .. ./run.sh ../x ~/src g++ c++ /etc") {
            assert_eq!(i.type_name(), "Sym", "{}", show(&i));
        }
        assert_eq!(one("/").type_name(), "Op");
    }

    #[test]
    fn numbers() {
        for t in ["1", "-7", "0x1f", "1_000", "2.5e3", "3j", ".5", "1."] {
            assert_eq!(one(t).type_name(), "Num", "{t}");
        }
        assert_eq!(one("1.2.3").type_name(), "Sym");
        assert_eq!(one("-la").type_name(), "Sym");
    }

    #[test]
    fn operators_only_when_spaced() {
        let its = items("$a + $b a+b -$x");
        assert!(its[1].is_op("+"));
        assert!(its[3].is_sym("a+b"));
        assert_eq!(its[4].type_name(), "Unary");
    }

    #[test]
    fn variables_and_postfix() {
        let v = one("$x.a[0].b");
        assert!(matches!(&v.kind, Kind::Attr { name, obj } if name == "b" && obj.type_name() == "Index"));
        assert_eq!(one("$?").type_name(), "Status");
        assert_eq!(one("$HOME/src").type_name(), "Word");
        assert_eq!(one("${name}.txt").type_name(), "Word");
        assert_eq!(one("$").type_name(), "Sym");
    }

    #[test]
    fn strings() {
        assert!(matches!(&one("\"a\\tb\"").kind, Kind::Str { value, .. } if value == "a\tb"));
        assert!(matches!(&one("'raw $x \\n'").kind, Kind::Str { value, .. } if value == "raw $x \\n"));
        let s = one("\"hi $name, ${n:.2f} $(pwd)\"");
        match &s.kind {
            Kind::IStr(ps) => {
                assert_eq!(ps.len(), 6);
                assert!(matches!(&ps[3], Part::F(Form { kind: Kind::Interp { spec: Some(sp), braced: true, .. }, .. }) if sp == ".2f"));
            }
            k => panic!("{k:?}"),
        }
        assert!(matches!(&one("b\"x\"").kind, Kind::Str { kind: 'b', .. }));
        assert!(matches!(&one("\"\"\"a\nb\"\"\"").kind, Kind::Str { value, .. } if value == "a\nb"));
        assert!(matches!(&one("\"cost \\$5\"").kind, Kind::Str { value, .. } if value == "cost $5"));
        assert!(matches!(&one("\"a\\qb\"").kind, Kind::Str { value, .. } if value == "a\\qb"));
        assert_eq!(one("' '.join").type_name(), "Attr");
    }

    #[test]
    fn kwargs_splats_and_prefixes() {
        let its = items("f key=val *$a **$k \\\\ls :sym `max` x==y");
        let names: Vec<_> = its.iter().map(|i| i.type_name()).collect();
        assert_eq!(names, ["Sym", "KwArg", "Splat", "Splat", "Caret", "SymLit", "Op", "Sym"]);
    }

    #[test]
    fn redirections() {
        let its = items("cmd >out.txt 2>&1 >>log <in 2>err > f");
        let names: Vec<_> = its.iter().map(|i| i.type_name()).collect();
        assert_eq!(names, ["Sym", "Redir", "Redir", "Redir", "Redir", "Redir", "Op", "Sym"]);
    }

    #[test]
    fn commas() {
        let its = items("cut -f1,3 [1, 2]");
        assert!(its[1].is_sym("-f1,3"));
        assert!(matches!(&its[2].kind, Kind::Group { items, .. } if items[1].is_comma()));
    }

    #[test]
    fn quasi_and_unquote() {
        let q = one(":(if ~x ~@ys)");
        match &q.kind {
            Kind::Quasi(g) => match &g.kind {
                Kind::Group { items, .. } => {
                    assert!(matches!(items[1].kind, Kind::Unquote { splice: false, .. }));
                    assert!(matches!(items[2].kind, Kind::Unquote { splice: true, .. }));
                }
                _ => panic!(),
            },
            _ => panic!(),
        }
        assert_eq!(one("$~a").type_name(), "Var");
        assert!(one("~/x").is_sym("~/x"));
        assert!(one("~").is_sym("~"));
    }

    #[test]
    fn indentation_blocks() {
        let ls = read("def f x\n  if $x\n    a\n  else\n    b\nf 1\n").unwrap();
        assert_eq!(ls.len(), 2);
        match &ls[0].kind {
            Kind::Line { children, .. } => {
                assert_eq!(children.len(), 2);
                assert_eq!(show(&ls[0]), "def f x\n  if $x\n    a\n  else\n    b");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn brackets_span_lines_and_continuations() {
        let ls = read("x = [1\n  2\n     3]\ny = 1").unwrap();
        assert_eq!(ls.len(), 2);
        let ls = read("echo a \\\n  b; echo c").unwrap();
        assert_eq!(ls.len(), 2);
        assert_eq!(show(&ls[0]), "echo a b");
    }

    #[test]
    fn comments() {
        let ls = read("# hi\nls # list\n  # inner\nx#y").unwrap();
        let shown: Vec<_> = ls.iter().map(show).collect();
        assert_eq!(shown, ["ls", "x#y"]);
    }

    #[test]
    fn positions() {
        let ls = read("a\n  bé $c").unwrap();
        match &ls[0].kind {
            Kind::Line { children, .. } => {
                let p = children[0].pos.unwrap();
                assert_eq!((p.line, p.col), (2, 2));
                match &children[0].kind {
                    Kind::Line { items, .. } => assert_eq!(items[1].pos.unwrap().col, 6),
                    _ => panic!(),
                }
            }
            _ => panic!(),
        }
    }

    #[test]
    fn errors_have_positions() {
        let e = read("x = (1 2\ny").unwrap_err();
        assert!(e.msg.starts_with("missing"), "{}", e.msg);
        assert_eq!((e.line, e.col), (1, 4));
        let e = read("a\n    b\n  c\n").unwrap_err();
        assert_eq!(e.line, 3);
        let e = read("\tx").unwrap_err();
        assert!(e.msg.contains("tab"), "{}", e.msg);
        let e = read("x = \"abc").unwrap_err();
        assert_eq!((e.line, e.col), (1, 4), "{}", e.msg);
        let e = read("x = 1)").unwrap_err();
        assert!(e.msg.contains("unmatched"), "{}", e.msg);
    }

    #[test]
    fn incomplete_input() {
        assert!(incomplete("x = (1"));
        assert!(incomplete("x = \"a"));
        assert!(incomplete("echo \\"));
        assert!(!incomplete("echo hi"));
        assert!(!incomplete("x = 1)"));
    }
}

/// A regex with Perl's flags (re'abc'i): Python's inline flags at its start.
fn regex_flags(pattern: String, flags: &str) -> String {
    let mut fl: Vec<char> = flags.chars().collect();
    fl.sort();
    fl.dedup();
    if fl.is_empty() {
        pattern
    } else {
        format!("(?{}){pattern}", fl.into_iter().collect::<String>())
    }
}
