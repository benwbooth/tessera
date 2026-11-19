/**
 * tessh, for highlighting and syntax errors: lines of items (words,
 * variables, strings, numbers, operators, groups that may span lines).
 * Indentation isn't structure here: the reader (tessh/src/reader.rs) has
 * that; this is the same language, made flat.
 */
module.exports = grammar({
  name: 'tessh',

  extras: $ => [/[ \t]/, /\\\r?\n/],

  word: $ => $.bareword,

  rules: {
    source_file: $ => seq(optional($._line_or_comment), repeat(seq($._newline, optional($._line_or_comment)))),

    _line_or_comment: $ => choice($.line, $.comment),

    line: $ => seq(repeat1(choice($._item, ';', ',')), optional($.comment)),

    _newline: _ => /\r?\n/,

    comment: _ => token(seq('#', /[^\n]*/)),

    _item: $ => choice(
      $.string,
      $.variable,
      $.status,
      $.interpolation,
      $.number,
      $.operator,
      $.infix_function,
      $.symbol,
      $.unquote,
      $.quasi,
      $.paren,
      $.bracket,
      $.brace,
      $.bareword,
    ),

    _group_items: $ => repeat1(choice($._item, $._newline, $.comment, ',', ';')),

    paren: $ => seq('(', optional($._group_items), ')'),
    bracket: $ => seq('[', optional($._group_items), ']'),
    brace: $ => seq('{', optional($._group_items), '}'),

    interpolation: $ => choice(
      seq('$(', optional($._group_items), ')'),
      seq('${', optional($._group_items), '}'),
    ),

    quasi: $ => seq(':(', optional($._group_items), ')'),

    unquote: $ => choice(
      token(seq('~', optional('@'), /[A-Za-z_][A-Za-z0-9_]*/)),
      seq(token(seq('~', optional('@'), '(')), optional($._group_items), ')'),
    ),

    variable: _ => token(seq('$', optional('~'), /[A-Za-z_][A-Za-z0-9_]*/, repeat(seq('.', /[A-Za-z_][A-Za-z0-9_]*/)))),

    status: _ => '$?',

    string: _ => token(choice(
      seq(optional(/[bBrR]{1,2}/), '"""', repeat(choice(/[^"\\]/, /\\(.|\n)/, /"[^"]/, /""[^"]/)), '"""'),
      seq(optional(/[bBrR]{1,2}/), "'''", repeat(choice(/[^']/, /'[^']/, /''[^']/)), "'''"),
      seq(optional(/[bBrR]{1,2}/), '"', repeat(choice(/[^"\\]/, /\\(.|\n)/)), '"'),
      seq(optional(/[bBrR]{1,2}/), "'", /[^']*/, "'"),
    )),

    number: _ => token(seq(
      optional(/[-+]/),
      choice(
        /0[xX][0-9a-fA-F_]+/,
        /0[oO][0-7_]+/,
        /0[bB][01_]+/,
        seq(/[0-9][0-9_]*/, optional(seq('.', optional(/[0-9][0-9_]*/))), optional(/[eE][-+]?[0-9]+/), optional(/[jJ]/)),
        seq('.', /[0-9][0-9_]*/, optional(/[eE][-+]?[0-9]+/), optional(/[jJ]/)),
      ),
    )),

    operator: _ => token(/[+\-*\/<>=!&|%^@?:][+\-*\/<>=!&|%^~@?:.]*/),

    infix_function: _ => token(seq('`', /[^`\s]+/, '`')),

    symbol: _ => token(seq(':', /[A-Za-z_][A-Za-z0-9_]*/)),

    bareword: _ => token(/[^\s()\[\]{}"'`;$,#][^\s()\[\]{}"'`;,]*/),
  },
});
