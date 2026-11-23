import QtQuick
import QtTest
import "../../qml"

// vi keys (qml/ViKeys.qml) on a bare TextEdit. Run:
//   qmltestrunner -platform offscreen -input tests/qml
// Each case: the text with | at the cursor, the keys (vim notation: <esc>,
// <cr>, <bs>, <C-a>), the text and cursor after. Keys in insert mode are
// typed into the field, as the field itself would.
Item {
    width: 400; height: 300
    TextEdit { id: edit; width: 400; height: 300; font.family: "monospace" }
    TextEdit { id: other; visible: false }   // (another app, for the clipboard)
    ExCommands { id: exc; edit: edit; keyHandler: (e) => vik.handle(e); viKeys: vik }
    ViKeys { id: vik; edit: edit; ex: exc }

    TestCase {
        name: "ViKeys"
        when: windowShown

        function tokens(s) {
            const out = []
            for (let i = 0; i < s.length; ++i) {
                if (s[i] === "<") {
                    const j = s.indexOf(">", i)
                    const name = j > i ? s.substring(i, j + 1) : ""
                    if (/^<(esc|cr|bs|C-[a-z\[])>$/.test(name)) { out.push(name); i = j; continue }
                }
                out.push(s[i])
            }
            return out
        }
        function press(tk) {
            let ev
            if (tk === "<esc>") ev = { key: Qt.Key_Escape, text: "\x1b", modifiers: 0 }
            else if (tk === "<cr>") ev = { key: Qt.Key_Return, text: "\r", modifiers: 0 }
            else if (tk === "<bs>") ev = { key: Qt.Key_Backspace, text: "\b", modifiers: 0 }
            else if (tk.startsWith("<C-")) ev = { key: tk[3] === "[" ? Qt.Key_BracketLeft : tk.toUpperCase().charCodeAt(3), text: "", modifiers: Qt.ControlModifier }
            else ev = { key: tk.toUpperCase().charCodeAt(0), text: tk, modifiers: 0 }
            if (vik.handle(ev)) return
            // (Insert mode: what the field does with it.)
            const p = edit.cursorPosition
            if (tk === "<cr>") { edit.insert(p, "\n"); edit.cursorPosition = p + 1 }
            else if (tk === "<bs>") { if (p > 0) edit.remove(p - 1, p) }
            else if (tk.length === 1) { edit.insert(p, tk); edit.cursorPosition = p + 1 }
        }
        function start(text) {
            const p = text.indexOf("|")
            edit.text = text.replace("|", "")
            exc.killRing = ""; exc.registers = ({}); exc.marks = ({})
            vik.reset()
            vik.lastChange = null; vik.lastFind = null; vik.regKind = ({})
            vik.viNormal = true
            edit.cursorPosition = p
        }
        function shown() {
            const t = edit.text, p = edit.cursorPosition
            return t.substring(0, p) + "|" + t.substring(p)
        }
        function check(before, keys, after, mode) {
            start(before)
            for (const tk of tokens(keys)) press(tk)
            compare(shown(), after, JSON.stringify(before) + " " + keys)
            if (mode !== undefined) compare(vik.modeName, mode, keys + " mode")
        }

        function test_motions_data() {
            return [
                { tag: "w", b: "|foo bar.baz", k: "w", a: "foo |bar.baz" },
                { tag: "3w", b: "|foo bar.baz", k: "3w", a: "foo bar.|baz" },
                { tag: "W", b: "|foo bar.baz qux", k: "W", a: "foo |bar.baz qux" },
                { tag: "2W", b: "|foo bar.baz qux", k: "2W", a: "foo bar.baz |qux" },
                { tag: "e", b: "|foo bar", k: "e", a: "fo|o bar" },
                { tag: "b", b: "foo ba|r", k: "b", a: "foo |bar" },
                { tag: "ge", b: "foo ba|r", k: "ge", a: "fo|o bar" },
                { tag: "w over lines", b: "fo|o\n  bar", k: "w", a: "foo\n  |bar" },
                { tag: "w to empty line", b: "fo|o\n\nbar", k: "w", a: "foo\n|\nbar" },
                { tag: "$", b: "|foo bar", k: "$", a: "foo ba|r" },
                { tag: "0", b: "  foo b|ar", k: "0", a: "|  foo bar" },
                { tag: "^", b: "  foo b|ar", k: "^", a: "  |foo bar" },
                { tag: "f", b: "|a,b,c", k: "f,", a: "a|,b,c" },
                { tag: "2f", b: "|a,b,c", k: "2f,", a: "a,b|,c" },
                { tag: "f;", b: "|a,b,c", k: "f,;", a: "a,b|,c" },
                { tag: "f;,", b: "|a,b,c", k: "f,;,", a: "a|,b,c" },
                { tag: "t", b: "|a,b,c", k: "t,", a: "|a,b,c" },
                { tag: "t;", b: "|a,b,c", k: "t,;", a: "a,|b,c" },
                { tag: "F", b: "a,b,|c", k: "F,", a: "a,b|,c" },
                { tag: "T", b: "a,b,|c", k: "T,", a: "a,b,|c" },
                { tag: "%", b: "|f(a, (b))", k: "%", a: "f(a, (b)|)" },
                { tag: "% back", b: "f(a, (b)|)", k: "%", a: "f|(a, (b))" },
                { tag: "j col", b: "ab|cd\nx\nabcdef", k: "jj", a: "abcd\nx\nab|cdef" },
                { tag: "k", b: "abc\nab|c", k: "k", a: "ab|c\nabc" },
                { tag: "G", b: "|a\nb\n  c", k: "G", a: "a\nb\n  |c" },
                { tag: "2G", b: "|a\n  b\nc", k: "2G", a: "a\n  |b\nc" },
                { tag: "gg", b: "a\nb\n|c", k: "gg", a: "|a\nb\nc" },
                { tag: "}", b: "|a\nb\n\nc", k: "}", a: "a\nb\n|\nc" },
                { tag: "{", b: "a\n\nb\n|c", k: "{", a: "a\n|\nb\nc" },
                { tag: ")", b: "|One two. Three.", k: ")", a: "One two. |Three." },
                { tag: "(", b: "One two. Thr|ee.", k: "(", a: "One two. |Three." },
                { tag: "+", b: "|a\n  b", k: "+", a: "a\n  |b" },
                { tag: "|", b: "|abcdef", k: "4|", a: "abc|def" },
                { tag: "mark", b: "a|bc\ndef", k: "majl`a", a: "a|bc\ndef" },
                { tag: "mark line", b: "  a|bc\ndef", k: "majl'a", a: "  |abc\ndef" },
            ]
        }
        function test_motions(d) { check(d.b, d.k, d.a) }

        function test_operators_data() {
            return [
                { tag: "dw", b: "|foo bar", k: "dw", a: "|bar" },
                { tag: "d2w", b: "|a b c", k: "d2w", a: "|c" },
                { tag: "2dw", b: "|a b c", k: "2dw", a: "|c" },
                { tag: "dw at line end", b: "foo |bar\nbaz", k: "dw", a: "foo| \nbaz" },
                { tag: "de", b: "|foo bar", k: "de", a: "| bar" },
                { tag: "d$", b: "fo|o bar\nx", k: "d$", a: "f|o\nx" },
                { tag: "D", b: "fo|o bar", k: "D", a: "f|o" },
                { tag: "dd", b: "a\n|b\nc", k: "dd", a: "a\n|c" },
                { tag: "2dd", b: "|a\nb\nc", k: "2dd", a: "|c" },
                { tag: "dd last", b: "a\n|b", k: "dd", a: "|a" },
                { tag: "dj", b: "|a\nb\nc", k: "dj", a: "|c" },
                { tag: "dk", b: "a\nb\n|c", k: "dk", a: "|a" },
                { tag: "dG", b: "a\n|b\nc", k: "dG", a: "|a" },
                { tag: "dt", b: "|abc,def", k: "dt,", a: "|,def" },
                { tag: "df", b: "|abc,def", k: "df,", a: "|def" },
                { tag: "d%", b: "x|(a(b))y", k: "d%", a: "x|y" },
                { tag: "d}", b: "|a\nb\n\nc", k: "d}", a: "|\n\nc" },
                { tag: "cw", b: "|foo bar", k: "cwxy<esc>", a: "x|y bar" },
                { tag: "cw one char", b: "|a bar", k: "cwx<esc>", a: "|x bar" },
                { tag: "cc", b: "a\n  |bc\nd", k: "ccx<esc>", a: "a\n  |x\nd" },
                { tag: "C", b: "fo|o bar", k: "Cx<esc>", a: "fo|x" },
                { tag: "s", b: "|abc", k: "2sx<esc>", a: "|xc" },
                { tag: "S", b: "  a|bc", k: "Sx<esc>", a: "  |x" },
                { tag: "x", b: "|abc", k: "x", a: "|bc" },
                { tag: "3x", b: "|abcd", k: "3x", a: "|d" },
                { tag: "X", b: "ab|c", k: "X", a: "a|c" },
                { tag: "yw p", b: "|foo bar", k: "ywP", a: "foo| foo bar" },
                { tag: "yy p", b: "|a\nb", k: "yyp", a: "a\n|a\nb" },
                { tag: "yy P", b: "a\n|b", k: "yyP", a: "a\n|b\nb" },
                { tag: "yy p last", b: "a\n|b", k: "yyp", a: "a\nb\n|b" },
                { tag: "3p", b: "|ab", k: "yl3p", a: "aaa|ab" },
                { tag: "ddp", b: "|a\nb", k: "ddp", a: "b\n|a" },
                { tag: "xp", b: "|ab", k: "xp", a: "b|a" },
                { tag: "register", b: "|foo bar", k: "\"ayiwwdiw\"ap", a: "foo fo|o" },
                { tag: "black hole", b: "|foo bar", k: "yiww\"_diwp", a: "foo fo|o" },
                { tag: ">>", b: "|a\nb", k: ">>", a: "    |a\nb" },
                { tag: "2>>", b: "|a\nb", k: "2>>", a: "    |a\n    b" },
                { tag: ">j", b: "|a\nb\nc", k: ">j", a: "    |a\n    b\nc" },
                { tag: "<<", b: "    |a", k: "<<", a: "|a" },
                { tag: "gUiw", b: "f|oo bar", k: "gUiw", a: "|FOO bar" },
                { tag: "guu", b: "F|OO BAR", k: "guu", a: "f|oo bar" },
                { tag: "gUU", b: "f|oo", k: "gUU", a: "F|OO" },
                { tag: "g~w", b: "|aBc d", k: "g~w", a: "|AbC d" },
                { tag: "g??", b: "|abc", k: "g??", a: "|nop" },
                { tag: "~", b: "|abc", k: "~~", a: "AB|c" },
                { tag: "3~", b: "|abcd", k: "3~", a: "ABC|d" },
                { tag: "r", b: "|abc", k: "rx", a: "|xbc" },
                { tag: "3r", b: "|abcd", k: "3rx", a: "xx|xd" },
                { tag: "r enter", b: "a|bc", k: "r<cr>", a: "a\n|c" },
                { tag: "J", b: "|a\n  b", k: "J", a: "a| b" },
                { tag: "3J", b: "|a\nb\nc", k: "3J", a: "a b| c" },
                { tag: "gJ", b: "|a\n  b", k: "gJ", a: "a|  b" },
                { tag: "R", b: "|abcd", k: "Rxy<esc>", a: "x|ycd" },
                { tag: "C-a", b: "x |9 y", k: "<C-a>", a: "x 1|0 y" },
                { tag: "C-a after", b: "|x 41", k: "<C-a>", a: "x 4|2" },
                { tag: "5C-x", b: "|3", k: "5<C-x>", a: "-|2" },
                { tag: "C-a hex", b: "|0xff", k: "<C-a>", a: "0x10|0" },
                { tag: "o", b: "  |a\nb", k: "ox<esc>", a: "  a\n  |x\nb" },
                { tag: "O", b: "a\n|b", k: "Ox<esc>", a: "a\n|x\nb" },
                { tag: "3i", b: "|z", k: "3iab<esc>", a: "ababa|bz" },
                { tag: "A", b: "|ab", k: "Ac<esc>", a: "ab|c" },
                { tag: "I", b: "  a|b", k: "Ic<esc>", a: "  |cab" },
            ]
        }
        function test_operators(d) { check(d.b, d.k, d.a) }

        function test_objects_data() {
            return [
                { tag: "diw", b: "foo b|ar baz", k: "diw", a: "foo | baz" },
                { tag: "daw", b: "foo b|ar baz", k: "daw", a: "foo |baz" },
                { tag: "daw last", b: "foo b|ar", k: "daw", a: "fo|o" },
                { tag: "ciw", b: "foo b|ar baz", k: "ciwX<esc>", a: "foo |X baz" },
                { tag: "yiw", b: "foo b|ar", k: "yiw$p", a: "foo barba|r" },
                { tag: "diW", b: "x a.b|-c y", k: "diW", a: "x | y" },
                { tag: "daW", b: "x a.b|-c y", k: "daW", a: "x |y" },
                { tag: "di\"", b: "say \"he|llo\" now", k: "di\"", a: "say \"|\" now" },
                { tag: "da\"", b: "say \"he|llo\" now", k: "da\"", a: "say |now" },
                { tag: "ci\" ahead", b: "|x = \"a\"", k: "ci\"b<esc>", a: "x = \"|b\"" },
                { tag: "di'", b: "a 'b|c' d", k: "di'", a: "a '|' d" },
                { tag: "di`", b: "a `b|c` d", k: "di`", a: "a `|` d" },
                { tag: "di(", b: "f(a, |b)", k: "di(", a: "f(|)" },
                { tag: "dib", b: "f(a, |b)", k: "dib", a: "f(|)" },
                { tag: "da(", b: "f(a, |b)", k: "da(", a: "|f" },
                { tag: "di) nested", b: "f(a, (|b))", k: "di)", a: "f(a, (|))" },
                { tag: "d2i(", b: "f(a, (|b))", k: "d2i(", a: "f(|)" },
                { tag: "ci( ahead", b: "|f(a)", k: "ci(x<esc>", a: "f(|x)" },
                { tag: "di[", b: "[1, |2]", k: "di[", a: "[|]" },
                { tag: "di{", b: "{ a |}", k: "di{", a: "{|}" },
                { tag: "diB lines", b: "f {\n  |a\n  b\n}", k: "diB", a: "f {\n|}" },
                { tag: "ci{ lines", b: "f {\n  |a\n  b\n}", k: "ci{x<esc>", a: "f {\n  |x\n}" },
                { tag: "da{", b: "x{ |a }y", k: "da{", a: "x|y" },
                { tag: "di<", b: "Vec<St|ring>", k: "di<", a: "Vec<|>" },
                { tag: "dit", b: "<a><b>x|y</b></a>", k: "dit", a: "<a><b>|</b></a>" },
                { tag: "dat", b: "<a><b>x|y</b></a>", k: "dat", a: "<a>|</a>" },
                { tag: "d2it", b: "<a><b>x|y</b></a>", k: "d2it", a: "<a>|</a>" },
                { tag: "dit attrs", b: "<p class=\"x\">a|b</p>", k: "dit", a: "<p class=\"x\">|</p>" },
                { tag: "dip", b: "a\n|b\n\nc", k: "dip", a: "|\nc" },
                { tag: "dap", b: "a\n|b\n\nc", k: "dap", a: "|c" },
                { tag: "yip", b: "|a\nb\n\nc", k: "yipGp", a: "a\nb\n\nc\n|a\nb" },
                { tag: "dis", b: "One. Tw|o three. Four.", k: "dis", a: "One. | Four." },
                { tag: "das", b: "One. Tw|o three. Four.", k: "das", a: "One. |Four." },
                { tag: ">ip", b: "|a\nb\n\nc", k: ">ip", a: "    |a\n    b\n\nc" },
                { tag: "gUi\"", b: "\"a|b\"", k: "gUi\"", a: "\"|AB\"" },
            ]
        }
        function test_objects(d) { check(d.b, d.k, d.a) }

        function test_surround_data() {
            return [
                { tag: "ysiw)", b: "f|oo bar", k: "ysiw)", a: "|(foo) bar" },
                { tag: "ysiw(", b: "f|oo bar", k: "ysiw(", a: "|( foo ) bar" },
                { tag: "ysiw\"", b: "f|oo bar", k: "ysiw\"", a: "|\"foo\" bar" },
                { tag: "ysaw] trims", b: "f|oo bar", k: "ysaw]", a: "|[foo] bar" },
                { tag: "ys2w", b: "|a b c", k: "ys2w'", a: "|'a b' c" },
                { tag: "yse", b: "|foo bar", k: "yseb", a: "|(foo) bar" },
                { tag: "ysiw tag", b: "f|oo", k: "ysiw<em>", a: "|<em>foo</em>" },
                { tag: "ysiwt", b: "f|oo", k: "ysiwtb<cr>", a: "|<b>foo</b>" },
                { tag: "yss", b: "  a |b  ", k: "yss)", a: "  |(a b)  " },
                { tag: "ysip linewise", b: "|a\nb", k: "ysip{", a: "|{\n    a\n    b\n}" },
                { tag: "ds\"", b: "x \"a|b\" y", k: "ds\"", a: "x |ab y" },
                { tag: "ds(", b: "f( a|b )", k: "ds(", a: "f|ab" },
                { tag: "ds)", b: "f( a|b )", k: "ds)", a: "f| ab " },
                { tag: "dsb", b: "f(a|b)", k: "dsb", a: "f|ab" },
                { tag: "dst", b: "<b>a|b</b>", k: "dst", a: "|ab" },
                { tag: "cs\"'", b: "x \"a|b\" y", k: "cs\"'", a: "x |'ab' y" },
                { tag: "cs)]", b: "f(a|b)", k: "cs)]", a: "f|[ab]" },
                { tag: "cs)[", b: "f(a|b)", k: "cs)[", a: "f|[ ab ]" },
                { tag: "cs]{", b: "[a|b]", k: "cs]}", a: "|{ab}" },
                { tag: "cst\"", b: "<p>a|b</p>", k: "cst\"", a: "|\"ab\"" },
                { tag: "cs\"t", b: "\"a|b\"", k: "cs\"<i>", a: "|<i>ab</i>" },
                { tag: "visual S", b: "a |bc d", k: "vlS]", a: "a |[bc] d" },
                { tag: "visual S tag", b: "a |bc d", k: "vlS<b>", a: "a |<b>bc</b> d" },
                { tag: "V S", b: "|a\nb", k: "VjS(", a: "|(\n    a\n    b\n)" },
                { tag: "dot ysiw", b: "|a b", k: "ysiw)W.", a: "(a) |(b)" },
            ]
        }
        function test_surround(d) { check(d.b, d.k, d.a) }

        function test_visual_data() {
            return [
                { tag: "vd", b: "|abcd", k: "vld", a: "|cd" },
                { tag: "v$d", b: "a|bc\nd", k: "vey", a: "a|bc\nd" },
                { tag: "viw y", b: "foo b|ar", k: "viwy$p", a: "foo barba|r" },
                { tag: "vi( c", b: "f(a|b)", k: "vi(cx<esc>", a: "f(|x)" },
                { tag: "vi( i( grows", b: "((a|b))", k: "vi(i(d", a: "(|)" },
                { tag: "Vd", b: "a\n|b\nc", k: "Vd", a: "a\n|c" },
                { tag: "Vjd", b: "|a\nb\nc", k: "Vjd", a: "|c" },
                { tag: "V>", b: "|a\nb", k: "Vj>", a: "    |a\n    b" },
                { tag: "vU", b: "|abc", k: "vlU", a: "|ABc" },
                { tag: "v~", b: "|aBc", k: "v$~", a: "|AbC" },
                { tag: "vr", b: "|abc", k: "vlrx", a: "|xxc" },
                { tag: "vJ", b: "|a\nb\nc", k: "VjjJ", a: "a b| c" },
                { tag: "vo", b: "a|bcd", k: "vlohd", a: "|d" },
                { tag: "vp", b: "|foo bar", k: "yiwwviwp", a: "foo |foo" },
                { tag: "esc", b: "|abc", k: "vl<esc>x", a: "a|c" },
                { tag: "gv", b: "|abc", k: "vl<esc>gvd", a: "|c" },
                { tag: "block d", b: "|abc\nabc\nabc", k: "<C-v>jjld", a: "|c\nc\nc" },
                { tag: "block I", b: "|ab\nab", k: "<C-v>jI# <esc>", a: "|# ab\n# ab" },
                { tag: "block A", b: "|ab\nab", k: "<C-v>jlA;<esc>", a: "ab|;\nab;" },
                { tag: "block c", b: "a|bc\nabc", k: "<C-v>jcX<esc>", a: "a|Xc\naXc" },
                { tag: "block y p", b: "|ab\ncd", k: "<C-v>jy$p", a: "ab|a\ncdc" },
                { tag: "block r", b: "|ab\nab", k: "<C-v>jrx", a: "|xb\nxb" },
                { tag: "block $A", b: "|a\nabc", k: "<C-v>j$A;<esc>", a: "a|;\nabc;" },
            ]
        }
        function test_visual(d) { check(d.b, d.k, d.a) }

        function test_dot_undo_data() {
            return [
                { tag: "dw.", b: "|a b c", k: "dw.", a: "|c" },
                { tag: "x.", b: "|abc", k: "x..", a: "|" },
                { tag: "ciw.", b: "|a b", k: "ciwX<esc>w.", a: "X |X" },
                { tag: "3.", b: "|abcdef", k: "x3.", a: "|ef" },
                { tag: "A.", b: "|a\nb", k: "A;<esc>j.", a: "a;\nb|;" },
                { tag: "dd.", b: "|a\nb\nc", k: "dd.", a: "|c" },
                { tag: "u", b: "|a b c", k: "dwdwu", a: "|b c" },
                { tag: "uu", b: "|a b c", k: "dwdwuu", a: "|a b c" },
                { tag: "u insert", b: "|a", k: "ciwhello<esc>u", a: "|a" },
                { tag: "C-r", b: "|a b c", k: "dwdwuu<C-r>", a: "|b c" },
                { tag: "u J", b: "|a\nb", k: "Ju", a: "|a\nb" },
                { tag: "u surround", b: "f|oo", k: "ysiw)u", a: "|foo" },
                { tag: "x x u", b: "|abc", k: "xxu", a: "|bc" },
            ]
        }
        function test_dot_undo(d) { check(d.b, d.k, d.a) }

        function test_modes() {
            check("|abc", "v", "a|bc", "VISUAL")
            check("|abc", "V", "abc|", "V-LINE")
            check("|abc", "<C-v>", "|abc", "V-BLOCK")
            check("|abc", "R", "|abc", "REPLACE")
            check("|abc", "i", "|abc", "INSERT")
            check("|abc", "i<C-[>", "|abc", "NORMAL")
            check("|abc", "d", "|abc", "NORMAL")
            compare(vik.viPending, "d")
            check("|abc", "d<esc>x", "|bc", "NORMAL")
        }
        function test_misc() {
            check("|foo bar foo", "*", "foo bar |foo")
            check("foo bar |foo", "#", "|foo bar foo")
            check("|foo bar foo", "*n", "|foo bar foo")
            // (& repeats the last :s on this line, g& on every line.)
            start("|a a\na a\na a")
            exc.run("s/a/b/")
            for (const tk of tokens("j&")) press(tk)
            compare(edit.text, "b a\nb a\na a")
            for (const tk of tokens("g&")) press(tk)
            compare(edit.text, "b b\nb b\nb a")
            // (A selection made with the mouse is a visual one.)
            start("|foo bar")
            edit.select(0, 4)
            for (const tk of tokens("d")) press(tk)
            compare(shown(), "|bar")
            // (k on the first line: the owner's history.)
            start("a|b\ncd")
            let went = 0
            vik.edgeLine = (d) => { went = d }
            for (const tk of tokens("jk")) press(tk)
            compare(went, 0)
            for (const tk of tokens("k")) press(tk)
            compare(went, -1)
            vik.edgeLine = null
            // (: with a count: those lines.)
            let line = ""
            start("|a\nb")
            const conn = (k, i) => { line = k + i }
            vik.commandLine.connect(conn)
            for (const tk of tokens("3:")) press(tk)
            vik.commandLine.disconnect(conn)
            compare(line, ":.,.+2")
            // (Visual : gives '<,'> and leaves the selection for it.)
            start("|a\nb")
            vik.commandLine.connect(conn)
            for (const tk of tokens("Vj:")) press(tk)
            vik.commandLine.disconnect(conn)
            compare(line, ":'<,'>")
            compare(vik.modeName, "NORMAL")
            compare(exc.markLine(">"), 1)
        }
        function test_clipboard() {
            // ("+ and "* are the system clipboard.)
            check("|foo bar", "\"+yiw", "|foo bar")
            other.text = ""; other.paste()
            compare(other.text, "foo")
            check("|foo bar", "\"+yiw$\"*p", "foo barfo|o")
            // (Text another app copied.)
            other.text = "pasted"; other.selectAll(); other.copy()
            check("|ab", "\"+p", "apaste|db")
            check("|ab", "v\"+p", "|pastedb")
            // (Lines yanked into it paste as lines; from elsewhere, a trailing newline makes them lines.)
            check("|a\nb", "\"+yyj\"+p", "a\nb\n|a")
            other.text = "x\ny\n"; other.selectAll(); other.copy()
            check("|a", "\"+P", "|x\ny\na")
            // (Deleting into it.)
            check("|foo bar", "\"+dw", "|bar")
            other.text = ""; other.paste()
            compare(other.text, "foo ")
            // (The unnamed register is untouched by "+ only when another register was named; vim sets it too.)
            check("|foo bar", "\"+yiwwP", "foo fo|obar")
        }
        function test_block_rects() {
            check("|abc\nabc", "<C-v>jl", "a|bc\nabc".replace("a|bc\nabc", "abc\na|bc"))
            compare(vik.blockRects.length, 2)
        }
    }
}
