import QtQuick

// vi for a text field: normal, visual (v, V, Ctrl+V) and replace modes;
// counts; operators (d c y > < gu gU g~ g?) over motions and text objects
// (iw aw iW aW is as ip ap, quotes, brackets, tags); surround.vim (ys, yss,
// ds, cs, S in visual); registers, marks, dot repeat, and undo by whole
// changes, as vim's. The input editor (ComposeBox) and file cards' editor
// (EditKeys) both use it: `handle(e)` from the field's key handler takes a
// key (true) or leaves it (false: insert mode's own keys, or the field's).
QtObject {
    id: vi
    property var edit: null             // the TextEdit / TextArea
    property var ex: null               // ExCommands: registers, marks, searching
    property string indentUnit: "    "
    property bool viNormal: false
    property string viPending: ""       // the keys of a command not finished yet
    property string visual: ""          // "", "v", "V", "b" (Ctrl+V: a block)
    property bool replacing: false      // R
    property int vAnchor: 0
    property int vHead: 0
    // A block selection's rectangle on each line (ViBlock draws them).
    property var blockRects: []
    // (j or k past the first or last line: the input editor goes through its history.)
    property var edgeLine: null
    signal commandLine(string kind, string initial)
    signal message(string text, bool error)
    readonly property string modeName: !viNormal ? (replacing ? "REPLACE" : "INSERT")
        : visual === "v" ? "VISUAL" : visual === "V" ? "V-LINE" : visual === "b" ? "V-BLOCK" : "NORMAL"

    // The keys, for a help table: [group, [[keys, what]]].
    readonly property var help: [
        ["vi: moving (a count first repeats: 3w, 5j)", [
            ["h j k l  w b e ge  W B E gE", "Letter, line; word start, back, end (W: words with punctuation)"],
            ["0 ^ $ g_ |  gg G", "Line start, first letter, end, last letter, column; first, last line (5G: line 5)"],
            ["f F t T {c}  ; ,", "To a letter on the line (t: just before); again, back"],
            ["% { } ( )", "The matching bracket; paragraph; sentence"],
            ["* # / ? n N", "Search the word, a pattern; next, previous"],
            ["m{a-z}  `{a-z} '{a-z}  ``", "Set a mark; go to it, its line; back where you jumped from"]]],
        ["vi: changing", [
            ["d c y > < gu gU g~ {motion}", "Delete, change, copy, indent, lower, upper, swap case: dw, c$, y2j; doubled for lines: dd cc yy >> guu"],
            ["iw aw  iW aW  is as  ip ap", "Text objects, after an operator or v: word, WORD, sentence, paragraph (i: inside, a: with spaces)"],
            ["i\" i' i`  i( ib  i[  i{ iB  i<  it", "Inside quotes, brackets, a tag (a\u2026: with them): ci\", da(, yit"],
            ["ys{motion}{c}  yss{c}", "Surround with c: ysiw) \u2192 (word); ( [ { add spaces inside; t or < a tag"],
            ["ds{c}  cs{old}{new}", "Delete or change the surrounding pair: ds\", cs'\", cst<b>"],
            ["x X s S C D Y J gJ", "Letter; before; change letter, line; change, delete to the end; copy line; join lines"],
            ["r{c} R ~ Ctrl+A Ctrl+X", "Replace a letter; type over; swap case; add to, subtract from the number"],
            ["i a I A o O gi", "Insert: before, after, line start, end, new line below, above, where you last did"],
            ["p P  \"{a-z} \"+", "Paste after, before; into or from a register: \"ayiw, \"ap; \"+ (or \"*) the system clipboard: \"+yy, \"+p"],
            ["u  Ctrl+R  .", "Undo, redo, do the last change again"],
            ["v V Ctrl+V  gv  o", "Select letters, lines, a block; the last selection; its other end"],
            ["S{c} (selecting)  I A (block)", "Surround the selection; type on every line of the block"],
            [": & g&", "Commands (:s/a/b/g, :g, :normal\u2026); the last :s again, on every line"]]]
    ]

    property var keyBuf: []
    property real wantCol: -1           // j and k keep to this column ($: the end)
    property var lastFind: null         // {c: f F t T, ch}
    property var markCols: ({})
    property int lastJump: -1
    property var lastVisual: null
    property int lastInsert: -1
    property var regKind: ({})          // register -> "char", "line", "block"
    property var lastChange: null       // {cmd, text, replace}: for "."
    property bool replaying: false
    property var ins: null              // the insert being typed: {start, len, count, cmd, prefix, block, typed}
    property var selSnap: null
    // Undo: each command, and each insert with the command that began it, is one step.
    property var undoStack: []
    property var redoStack: []
    property var lastKnown: null
    property var changeBase: null

    onEditChanged: lastKnown = edit ? edit.text : null
    onViPendingChanged: if (viPending === "" && keyBuf.length) keyBuf = []
    onViNormalChanged: {
        if (viNormal) { endChange(); syncExternal() }
        else { if (visual !== "") { visual = ""; blockRects = [] } }
    }
    // (A new text, not an edit of the old: nothing to undo.)
    function reset() {
        undoStack = []; redoStack = []; changeBase = null; keyBuf = []; viPending = ""
        visual = ""; blockRects = []; ins = null; replacing = false
        lastKnown = edit ? edit.text : null
    }

    // ---- text ----------------------------------------------------------------
    function txt() { return edit.text }
    function isWord(c) { return c !== undefined && /[A-Za-z0-9_À-￿]/.test(c) }
    function isBlank(c) { return c === " " || c === "\t" }
    function lineStart(p) { return p <= 0 ? 0 : edit.text.lastIndexOf("\n", p - 1) + 1 }
    function lineEnd(p) { const i = edit.text.indexOf("\n", p); return i < 0 ? edit.text.length : i }
    function lineOf(p) { const t = edit.text; let n = 0, i = -1; while ((i = t.indexOf("\n", i + 1)) >= 0 && i < p) n++; return n }
    function lineCount() { const t = edit.text; let n = 1, i = -1; while ((i = t.indexOf("\n", i + 1)) >= 0) n++; return n }
    function posOfLine(n) { const t = edit.text; let p = 0; for (let k = 0; k < n; ++k) { const i = t.indexOf("\n", p); if (i < 0) return p; p = i + 1 } return p }
    function firstNonBlank(p) { const t = edit.text; let i = lineStart(p); const e = lineEnd(p); while (i < e && isBlank(t[i])) i++; return i }
    function indentOf(p) { return edit.text.substring(lineStart(p), firstNonBlank(p)) }
    function linesRange(l1, l2) {
        const t = edit.text, a = posOfLine(l1), e = lineEnd(posOfLine(l2))
        return { a: a, b: e < t.length ? e + 1 : e, linewise: true }
    }
    function replace(a, b, s) {
        if (b > a) edit.remove(a, b)
        if (s !== "") edit.insert(a, s)
    }
    // Lines l1..l2 (whole, without their last newline) become `lines`.
    function replaceLines(l1, l2, lines) {
        const a = posOfLine(l1), b = lineEnd(posOfLine(l2))
        replace(a, b, lines.join("\n"))
    }
    function setCursor(p) { edit.cursorPosition = Math.max(0, Math.min(edit.length, p)) }
    function clampCursor() {
        if (visual !== "" || !viNormal || edit.selectionStart !== edit.selectionEnd) return
        const p = edit.cursorPosition
        if (p > 0 && p === lineEnd(p) && p > lineStart(p)) edit.cursorPosition = p - 1
    }

    // ---- keys ----------------------------------------------------------------
    function token(e) {
        const mods = e.modifiers || 0
        const ctrl = (mods & Qt.ControlModifier) && !(mods & (Qt.AltModifier | Qt.MetaModifier))
        if (e.key === Qt.Key_Escape || (ctrl && e.key === Qt.Key_BracketLeft)) return "<esc>"
        if (ctrl) return e.key >= Qt.Key_A && e.key <= Qt.Key_Z ? "<C-" + String.fromCharCode(e.key + 32) + ">" : ""
        if (mods & (Qt.AltModifier | Qt.MetaModifier)) return ""
        switch (e.key) {
        case Qt.Key_Return: case Qt.Key_Enter: return "<cr>"
        case Qt.Key_Backspace: return "<bs>"
        case Qt.Key_Delete: return "<del>"
        case Qt.Key_Left: return "h"
        case Qt.Key_Right: return "l"
        case Qt.Key_Up: return "k"
        case Qt.Key_Down: return "j"
        case Qt.Key_Home: return "0"
        case Qt.Key_End: return "$"
        case Qt.Key_Tab: case Qt.Key_Backtab: return ""
        }
        const t = e.text || ""
        return t.length === 1 && t >= " " ? t : ""
    }

    function handle(e) {
        if (!edit) return false
        if (!viNormal) return insertKey(e)
        syncSelection()
        const tk = token(e)
        if (tk === "") return (e.text || "") !== "" && !((e.modifiers || 0) & Qt.ControlModifier)
        if (tk.startsWith("<C-") && keyBuf.length === 0 && ["<C-r>", "<C-a>", "<C-x>", "<C-v>", "<C-q>"].indexOf(tk) < 0) return false
        if (tk === "<esc>") {
            const had = keyBuf.length > 0
            keyBuf = []; viPending = ""
            if (!had && visual !== "") endVisual()
            return true
        }
        keyBuf = keyBuf.concat([tk])
        const r = visual !== "" ? parseVisual(keyBuf) : parse(keyBuf)
        if (r === null) { viPending = keyBuf.join(""); return true }
        keyBuf = []; viPending = ""
        if (r !== false) run(r)
        return true
    }

    // Insert mode: Esc (or Ctrl+[) back to normal, R's typing over; the rest is the field's.
    function insertKey(e) {
        const tk = token(e)
        if (tk === "<esc>") { leaveInsert(); return true }
        if (replacing) {
            const ch = e.text || "", p = edit.cursorPosition
            if (ch.length === 1 && ch >= " " && !((e.modifiers || 0) & (Qt.ControlModifier | Qt.AltModifier))) {
                overwrite(ch)
                if (ins) ins.typed = (ins.typed || "") + ch
                return true
            }
            if (e.key === Qt.Key_Backspace) { edit.cursorPosition = Math.max(lineStart(p), p - 1); return true }
        }
        return false
    }
    function overwrite(s) {
        for (const ch of s) {
            const p = edit.cursorPosition
            if (p < lineEnd(p) && ch !== "\n") edit.remove(p, p + 1)
            edit.insert(p, ch)
            edit.cursorPosition = p + 1
        }
    }
    function enterInsert(cmd, count, extra) {
        ins = Object.assign({ start: edit.cursorPosition, len: edit.text.length, count: Math.max(1, count || 1), cmd: cmd }, extra || {})
        viNormal = false
    }
    function leaveInsert() {
        let p = edit.cursorPosition
        if (ins) {
            const t = edit.text
            let typed = ins.typed !== undefined ? ins.typed
                : (p >= ins.start && t.length - ins.len === p - ins.start ? t.substring(ins.start, p) : "")
            if (typed !== "" && ins.count > 1 && !replacing) {
                const more = ((ins.prefix || "") + typed).repeat(ins.count - 1)
                edit.insert(p, more)
                p += more.length
                edit.cursorPosition = p
            }
            if (ins.block && typed !== "" && typed.indexOf("\n") < 0) {
                const b = ins.block, lines = edit.text.split("\n")
                for (let l = b.l1 + 1; l <= b.l2 && l < lines.length; ++l) {
                    let line = lines[l]
                    const col = b.append && b.dollar ? line.length : b.col
                    if (line.length < col) { if (!b.append) continue; line = line + " ".repeat(col - line.length) }
                    lines[l] = line.substring(0, col) + typed + line.substring(col)
                }
                if (b.l2 > b.l1) {
                    const keep = posOfLine(b.l1) + b.col
                    replaceLines(b.l1 + 1, Math.min(b.l2, lines.length - 1), lines.slice(b.l1 + 1, b.l2 + 1))
                    edit.cursorPosition = keep + 1
                    p = keep + 1
                }
            }
            if (ins.cmd && !replaying) lastChange = { cmd: ins.cmd, text: typed, replace: replacing }
        }
        lastInsert = p
        ins = null
        replacing = false
        edit.cursorPosition = Math.max(lineStart(p), p - 1)
        viNormal = true
    }

    // ---- parsing: [count]["x][count] command ---------------------------------------
    function isDigit(s) { return s.length === 1 && s >= "0" && s <= "9" }
    function one(s) { return s !== undefined && s.length === 1 }
    readonly property string simpleMotions: "hjklwbeWBE0^$G|;,%{}()+-_ nN"
    function parseCount(k, i) {
        let c = ""
        while (i < k.length && isDigit(k[i]) && (k[i] !== "0" || c !== "")) c += k[i++]
        return [c === "" ? 0 : parseInt(c), i]
    }
    function parseMotion(k, j) {
        const c = k[j]
        if (c === undefined) return null
        if (c === "<cr>" || c === "<bs>") return { m: c, next: j + 1 }
        if (!one(c)) return false
        if (simpleMotions.indexOf(c) >= 0) return { m: c, next: j + 1 }
        if ("fFtT`'".indexOf(c) >= 0) {
            if (j + 1 >= k.length) return null
            if (!one(k[j + 1])) return false
            return { m: c, ch: k[j + 1], next: j + 2 }
        }
        if (c === "g") {
            if (j + 1 >= k.length) return null
            if (["g", "e", "E", "_"].indexOf(k[j + 1]) >= 0) return { m: "g" + k[j + 1], next: j + 2 }
            return false
        }
        return false
    }
    function parseObject(k, j) {
        const c = k[j]
        if (c !== "i" && c !== "a") return false
        if (j + 1 >= k.length) return null
        if (!one(k[j + 1]) || "wWsp\"'`()b[]{}B<>t".indexOf(k[j + 1]) < 0) return false
        return { obj: c + k[j + 1], next: j + 2 }
    }
    // A surround target: one character, or a tag (< or t, its name, then > or Enter).
    function parseTarget(k, j) {
        if (j >= k.length) return null
        const c = k[j]
        if (c === "<" || c === "t") {
            let s = ""
            for (let i = j + 1; i < k.length; ++i) {
                if (k[i] === ">" || k[i] === "<cr>") return { target: "<" + s.trim() + ">", next: i + 1 }
                if (k[i] === "<bs>") { s = s.slice(0, -1); continue }
                if (!one(k[i])) return false
                s += k[i]
            }
            return null
        }
        if (!one(c)) return false
        return { target: c, next: j + 1 }
    }
    function parsePrefix(k) {
        let i = 0, reg = ""
        if (k[0] === "\"") {
            if (k.length < 2) return null
            reg = k[1]
            if (!/^[a-zA-Z0-9"_+*-]$/.test(reg)) return false
            i = 2
        }
        const [count, j] = parseCount(k, i)
        if (j >= k.length) return null
        return { reg: reg, count: count, i: j }
    }
    function parse(k) {
        const pre = parsePrefix(k)
        if (!pre) return pre
        let i = pre.i
        const r = { reg: pre.reg, count: pre.count }
        const c = k[i], n1 = k[i + 1]
        // ds{c}, cs{old}{new}
        if ((c === "d" || c === "c") && n1 === "s") {
            if (i + 2 >= k.length) return null
            if (!one(k[i + 2])) return false
            r.name = c + "s"; r.ch = k[i + 2]
            if (c === "d") return r
            const tg = parseTarget(k, i + 3)
            if (!tg) return tg
            r.target = tg.target
            return r
        }
        let op = "", j = i + 1
        if (one(c) && "dcy<>".indexOf(c) >= 0) op = c
        if (c === "g") {
            if (n1 === undefined) return null
            if ("uU~?".indexOf(n1) >= 0) { op = "g" + n1; j = i + 2 }
        }
        if (op === "y" && n1 === "s") { op = "ys"; j = i + 2 }
        if (op !== "") {
            r.op = op
            const [c2, jj] = parseCount(k, j)
            r.count2 = c2; j = jj
            if (j >= k.length) return null
            const last = op === "ys" ? "s" : op[op.length - 1]
            let rest = -1
            if (k[j] === last) { r.line = true; rest = j + 1 }
            else if (op.length === 2 && op[0] === "g" && op !== "ys" && k[j] === "g") {
                if (j + 1 >= k.length) return null
                if (k[j + 1] === last) { r.line = true; rest = j + 2 }
            }
            if (rest < 0) {
                const m = parseMotion(k, j)
                if (m === null) return null
                if (m) { r.motion = m; rest = m.next }
                else {
                    const o = parseObject(k, j)
                    if (!o) return o
                    r.obj = o.obj; rest = o.next
                }
            }
            if (op === "ys") {
                const tg = parseTarget(k, rest)
                if (!tg) return tg
                r.target = tg.target
            }
            return r
        }
        const m = parseMotion(k, i)
        if (m === null) return null
        if (m) { r.motion = m; return r }
        if (c === "r" || c === "m") {
            if (n1 === undefined) return null
            if (!one(n1) && !(c === "r" && n1 === "<cr>")) return false
            r.name = c; r.ch = n1
            return r
        }
        if (c === "g") {
            if (["J", "v", "i", "I", "p", "P", "&"].indexOf(n1) >= 0) { r.name = "g" + n1; return r }
            return false
        }
        if (["x", "X", "s", "S", "C", "D", "Y", "p", "P", "J", "~", "i", "a", "I", "A", "o", "O", "R", "u", "<C-r>", ".",
             "v", "V", "<C-v>", "<C-q>", "*", "#", ":", "/", "?", "&", "<C-a>", "<C-x>", "<del>"].indexOf(c) >= 0) { r.name = c; return r }
        return false
    }
    function parseVisual(k) {
        const pre = parsePrefix(k)
        if (!pre) return pre
        const i = pre.i, c = k[i], n1 = k[i + 1]
        const r = { reg: pre.reg, count: pre.count }
        const m = parseMotion(k, i)
        if (m === null) return null
        if (m) { r.motion = m; return r }
        const o = parseObject(k, i)
        if (o === null) return null
        if (o) { r.obj = o.obj; return r }
        if (c === "r") {
            if (n1 === undefined) return null
            if (!one(n1)) return false
            r.name = "r"; r.ch = n1
            return r
        }
        if (c === "S") {
            const tg = parseTarget(k, i + 1)
            if (!tg) return tg
            r.name = "S"; r.target = tg.target
            return r
        }
        if (c === "g") {
            if (n1 === undefined) return null
            if (["u", "U", "~", "?", "J", "v"].indexOf(n1) >= 0) { r.name = "g" + n1; return r }
            return false
        }
        if (["d", "x", "X", "D", "c", "s", "C", "R", "y", "Y", ">", "<", "~", "u", "U", "J", "p", "P", "I", "A", "o", "O",
             "v", "V", "<C-v>", "<C-q>", ":", "<C-a>", "<C-x>", "<del>"].indexOf(c) >= 0) { r.name = c; return r }
        return false
    }

    // ---- running -----------------------------------------------------------------
    function run(r) {
        if (r.name === "u" || r.name === "<C-r>") {
            syncExternal()
            if (r.name === "u") undo(Math.max(1, r.count)); else redo(Math.max(1, r.count))
            clampCursor()
            return
        }
        syncExternal()
        beginChange()
        try {
            if (visual !== "") runVisual(r)
            else runNormal(r)
        } catch (x) {
            console.warn("vi: " + x + (x && x.stack ? "\n" + x.stack : ""))
        }
        if (viNormal) endChange()
        clampCursor()
        updateVisual()
    }
    function isChange(r) {
        if (r.op) return r.op !== "y"
        return ["x", "X", "s", "S", "C", "D", "p", "P", "J", "~", "r", "R", "i", "a", "I", "A", "o", "O", "gI", "gi",
                "<C-a>", "<C-x>", "gJ", "gp", "gP", "ds", "cs", "&", "g&", "<del>"].indexOf(r.name) >= 0
    }

    function runNormal(r) {
        const p = edit.cursorPosition, t = edit.text
        const n = Math.max(1, r.count)
        if (!r.motion) wantCol = -1
        if (r.name !== undefined || r.op) {
            if (!replaying && isChange(r) && r.name !== "." ) lastChange = { cmd: r, text: "" }
        }
        if (r.op) { operate(r); return }
        if (r.motion) {
            const m = motionTarget(r.motion, p, r.count, false)
            if (!m) return
            moveTo(m, r.motion)
            return
        }
        const ls = lineStart(p), le = lineEnd(p), L = lineOf(p)
        switch (r.name) {
        case "x": case "<del>": if (p < le) deleteRange(r.reg, { a: p, b: Math.min(le, p + n) }); return
        case "X": if (p > ls) deleteRange(r.reg, { a: Math.max(ls, p - n), b: p }); return
        case "s": change(r.reg, { a: p, b: Math.min(le, p + n) }, r); return
        case "S": change(r.reg, linesRange(L, Math.min(lineCount() - 1, L + n - 1)), r); return
        case "C": change(r.reg, { a: p, b: lineEnd(posOfLine(Math.min(lineCount() - 1, L + n - 1))) }, r); return
        case "D": deleteRange(r.reg, { a: p, b: lineEnd(posOfLine(Math.min(lineCount() - 1, L + n - 1))) }); return
        case "Y": yank(r.reg, linesRange(L, Math.min(lineCount() - 1, L + n - 1))); return
        case "p": put(r.reg, true, n, false); return
        case "P": put(r.reg, false, n, false); return
        case "gp": put(r.reg, true, n, true); return
        case "gP": put(r.reg, false, n, true); return
        case "J": join(L, Math.max(2, n), true); return
        case "gJ": join(L, Math.max(2, n), false); return
        case "~": {
            const b = Math.min(le, p + n)
            if (b > p) { replace(p, b, toggleCase(t.substring(p, b))); setCursor(b) }
            return
        }
        case "r": {
            if (p + n > le) return
            if (r.ch === "<cr>") { replace(p, p + n, "\n" + indentOf(p)); setCursor(p + 1 + indentOf(p).length); return }
            replace(p, p + n, r.ch.repeat(n))
            setCursor(p + n - 1)
            return
        }
        case "R": replacing = true; enterInsert(r, n, { typed: "" }); return
        case "i": enterInsert(r, n); return
        case "a": setCursor(Math.min(le, p + 1)); enterInsert(r, n); return
        case "I": setCursor(firstNonBlank(p)); enterInsert(r, n); return
        case "gI": setCursor(ls); enterInsert(r, n); return
        case "A": setCursor(le); enterInsert(r, n); return
        case "gi": setCursor(lastInsert >= 0 ? Math.min(lastInsert, t.length) : p); enterInsert(r, n); return
        case "o": {
            const ind = indentOf(p)
            edit.insert(le, "\n" + ind)
            setCursor(le + 1 + ind.length)
            enterInsert(r, n, { prefix: "\n" + ind })
            return
        }
        case "O": {
            const ind = indentOf(p)
            edit.insert(ls, ind + "\n")
            setCursor(ls + ind.length)
            enterInsert(r, n, { prefix: "\n" + ind })
            return
        }
        case ".": repeatChange(r); return
        case "v": case "V": case "<C-v>": case "<C-q>": startVisual(r.name === "v" ? "v" : r.name === "V" ? "V" : "b"); return
        case "gv": {
            if (!lastVisual) return
            visual = lastVisual.mode
            vAnchor = Math.min(lastVisual.a, t.length); vHead = Math.min(lastVisual.h, t.length)
            return
        }
        case "*": case "#": {
            let s = p
            while (s < le && !isWord(t[s])) s++
            if (s >= le) return
            let a = s, e = s
            while (a > ls && isWord(t[a - 1])) a--
            while (e < le && isWord(t[e])) e++
            const w = t.substring(a, e)
            lastJump = p
            let msg = ex.search("\\<" + w + "\\>", r.name === "#")
            for (let k = 1; k < n && !msg.startsWith("E"); ++k) msg = ex.searchNext(false)
            if (msg !== "") message(msg, msg.startsWith("E"))
            return
        }
        case ":": commandLine(":", r.count > 1 ? ".,.+" + (r.count - 1) : r.count === 1 ? "." : ""); return
        case "/": case "?": commandLine(r.name, ""); return
        case "&": case "g&": {
            const msg = ex.run(r.name === "&" ? "s" : "%s//~/&")
            if (msg !== "") message(msg, ex.error)
            return
        }
        case "m": {
            if (!/^[a-zA-Z]$/.test(r.ch)) return
            ex.load()
            const mm = Object.assign({}, ex.marks); mm[r.ch] = L; ex.marks = mm
            const mc = Object.assign({}, markCols); mc[r.ch] = p - ls; markCols = mc
            return
        }
        case "ds": deleteSurround(r.ch, n); return
        case "cs": changeSurround(r.ch, r.target, n); return
        case "<C-a>": increment(p, n); return
        case "<C-x>": increment(p, -n); return
        }
    }

    function moveTo(m, mo) {
        if (m.edge) { if (edgeLine) edgeLine(m.edge); return }
        const p = edit.cursorPosition
        if (m.jump) lastJump = p
        setCursor(m.pos)
        if (m.col !== undefined) wantCol = m.col
        else wantCol = mo.m === "$" ? 1e9 : -1
        if (m.msg) message(m.msg, m.msg.startsWith("E"))
    }

    // ---- motions: where one goes from p ---------------------------------------
    // {pos, linewise, inclusive, jump, col (j/k's column), msg} or null.
    function cls(c, big) {
        if (c === undefined) return -1
        if (c === " " || c === "\t" || c === "\n") return 0
        return big ? 1 : isWord(c) ? 2 : 1
    }
    function wordFwd(p, big) {
        const t = edit.text, n = t.length
        if (p >= n) return n
        const c0 = cls(t[p], big)
        if (c0 !== 0) while (p < n && cls(t[p], big) === c0) p++
        while (p < n) {
            if (t[p] === "\n") { p++; if (p < n && t[p] === "\n") return p; continue }
            if (isBlank(t[p])) { p++; continue }
            break
        }
        return p
    }
    function wordEndFwd(p, big) {
        const t = edit.text, n = t.length
        p++
        while (p < n && cls(t[p], big) === 0) p++
        if (p >= n) return Math.max(0, n - 1)
        const c = cls(t[p], big)
        while (p + 1 < n && cls(t[p + 1], big) === c) p++
        return p
    }
    function wordBack(p, big) {
        const t = edit.text
        if (p <= 0) return 0
        p--
        while (p > 0 && cls(t[p], big) === 0) { if (t[p] === "\n" && t[p - 1] === "\n") return p; p-- }
        if (cls(t[p], big) === 0) return p
        const c = cls(t[p], big)
        while (p > 0 && cls(t[p - 1], big) === c) p--
        return p
    }
    function wordEndBack(p, big) {
        const t = edit.text
        const c = cls(t[p], big)
        if (c > 0) while (p > 0 && cls(t[p - 1], big) === c) p--
        p--
        while (p > 0 && cls(t[p], big) === 0) { if (t[p] === "\n" && t[p - 1] === "\n") return p; p-- }
        return Math.max(0, p)
    }
    function findChar(c, ch, n, p, again) {
        const t = edit.text, a = lineStart(p), e = lineEnd(p)
        let q = p
        if (c === "f" || c === "t") {
            let from = p + 1 + (c === "t" && again ? 1 : 0)
            for (let k = 0; k < n; ++k) {
                q = t.indexOf(ch, k === 0 ? from : q + 1)
                if (q < 0 || q >= e) return null
            }
            return { pos: c === "t" ? q - 1 : q, inclusive: true }
        }
        let from = p - 1 - (c === "T" && again ? 1 : 0)
        for (let k = 0; k < n; ++k) {
            const s = k === 0 ? from : q - 1
            if (s < a) return null
            q = t.lastIndexOf(ch, s)
            if (q < a) return null
        }
        return { pos: c === "T" ? q + 1 : q }
    }
    readonly property var bracketPairs: ({ "(": ")", "[": "]", "{": "}", ")": "(", "]": "[", "}": "{" })
    function matchBracket(p) {
        const t = edit.text, e = lineEnd(p)
        let i = p
        while (i < e && "()[]{}".indexOf(t[i]) < 0) i++
        if (i >= e) return -1
        const c = t[i], o = bracketPairs[c]
        let depth = 0
        if ("([{".indexOf(c) >= 0) {
            for (let j = i; j < t.length; ++j) { if (t[j] === c) depth++; else if (t[j] === o && --depth === 0) return j }
        } else {
            for (let j = i; j >= 0; --j) { if (t[j] === c) depth++; else if (t[j] === o && --depth === 0) return j }
        }
        return -1
    }
    function emptyLine(a) { return a < edit.text.length ? edit.text[a] === "\n" : a === lineStart(a) }
    function paraFwd(p) {
        const t = edit.text
        let a = lineStart(p)
        while (a < t.length && t[a] === "\n") a++
        while (a < t.length && t[a] !== "\n") { const e = lineEnd(a); if (e >= t.length) return t.length; a = e + 1 }
        return a
    }
    function paraBack(p) {
        const t = edit.text
        let a = lineStart(p)
        while (a > 0 && t[a] === "\n") a = lineStart(a - 1)
        while (a > 0 && t[a] !== "\n") a = lineStart(a - 1)
        return a
    }
    function sentenceStarts() {
        const t = edit.text, out = []
        let k = 0
        while (k < t.length && /\s/.test(t[k])) k++
        out.push(k)
        for (let i = 0; i < t.length; ++i) {
            const c = t[i]
            if (c === "\n" && t[i + 1] === "\n") {
                out.push(i + 1)
                let j = i + 1
                while (j < t.length && /\s/.test(t[j])) j++
                if (j < t.length) out.push(j)
            } else if (".!?".indexOf(c) >= 0) {
                let j = i + 1
                while (j < t.length && ")]\"'".indexOf(t[j]) >= 0) j++
                if (j < t.length && /\s/.test(t[j])) {
                    while (j < t.length && /\s/.test(t[j]) && !(t[j] === "\n" && t[j + 1] === "\n")) j++
                    if (j < t.length && t[j] !== "\n") out.push(j)
                }
            }
        }
        return out.filter((x, i, a) => x <= t.length && a.indexOf(x) === i).sort((x, y) => x - y)
    }
    function motionTarget(mo, p, count, inOp) {
        const t = edit.text, n = Math.max(1, count)
        const L = lineOf(p)
        switch (mo.m) {
        case "h": case "<bs>": return p > lineStart(p) ? { pos: Math.max(lineStart(p), p - n) } : null
        case "l": case " ": {
            const e = lineEnd(p), max = inOp ? e : Math.max(lineStart(p), e - 1)
            return p >= max ? null : { pos: Math.min(max, p + n) }
        }
        case "j": case "k": {
            const total = lineCount()
            const dest = L + (mo.m === "j" ? n : -n)
            if (dest < 0 || dest >= total) {
                if (edgeLine && !inOp && visual === "") return { edge: mo.m === "j" ? 1 : -1 }
                if ((mo.m === "j" && L === total - 1) || (mo.m === "k" && L === 0)) return null
            }
            const d = Math.max(0, Math.min(total - 1, dest))
            const col = wantCol >= 0 ? wantCol : p - lineStart(p)
            const a = posOfLine(d), e = lineEnd(a)
            return { pos: Math.min(a + col, e), linewise: true, col: col }
        }
        case "+": case "<cr>": case "-": case "_": {
            const total = lineCount()
            const dest = mo.m === "-" ? L - n : mo.m === "_" ? L + n - 1 : L + n
            if (dest < 0 || dest >= total) return null
            return { pos: firstNonBlank(posOfLine(dest)), linewise: true }
        }
        case "G": case "gg": {
            const total = lineCount()
            const dest = count > 0 ? Math.min(total, count) - 1 : mo.m === "G" ? total - 1 : 0
            return { pos: firstNonBlank(posOfLine(dest)), linewise: true, jump: true }
        }
        case "0": return { pos: lineStart(p) }
        case "^": return { pos: firstNonBlank(p) }
        case "$": return { pos: lineEnd(posOfLine(Math.min(lineCount() - 1, L + n - 1))), col: 1e9 }
        case "g_": {
            const a = posOfLine(Math.min(lineCount() - 1, L + n - 1))
            let e = lineEnd(a)
            while (e > a && isBlank(t[e - 1])) e--
            return { pos: Math.max(a, e - 1), inclusive: true }
        }
        case "|": return { pos: Math.min(lineEnd(p), lineStart(p) + n - 1) }
        case "w": case "W": { let q = p; for (let k = 0; k < n; ++k) q = wordFwd(q, mo.m === "W"); return q === p ? null : { pos: q } }
        case "b": case "B": { let q = p; for (let k = 0; k < n; ++k) q = wordBack(q, mo.m === "B"); return q === p ? null : { pos: q } }
        case "e": case "E": { let q = p; for (let k = 0; k < n; ++k) q = wordEndFwd(q, mo.m === "E"); return { pos: q, inclusive: true } }
        case "ge": case "gE": { let q = p; for (let k = 0; k < n; ++k) q = wordEndBack(q, mo.m === "gE"); return { pos: q, inclusive: true } }
        case "f": case "F": case "t": case "T": {
            const f = findChar(mo.m, mo.ch, n, p, false)
            lastFind = { c: mo.m, ch: mo.ch }
            return f
        }
        case ";": case ",": {
            if (!lastFind) return null
            let c = lastFind.c
            if (mo.m === ",") c = ({ f: "F", F: "f", t: "T", T: "t" })[c]
            return findChar(c, lastFind.ch, n, p, true)
        }
        case "%": {
            if (count > 0) {
                const dest = Math.min(lineCount(), Math.floor((count * lineCount() + 99) / 100)) - 1
                return { pos: firstNonBlank(posOfLine(Math.max(0, dest))), linewise: true, jump: true }
            }
            const q = matchBracket(p)
            if (q < 0) return null
            // (From before the bracket, an operator covers from the cursor.)
            return { pos: q, inclusive: true, jump: true, from: Math.min(p, q) }
        }
        case "}": { let q = p; for (let k = 0; k < n; ++k) q = paraFwd(q); return { pos: q, jump: true } }
        case "{": { let q = p; for (let k = 0; k < n; ++k) q = paraBack(q); return { pos: q, jump: true } }
        case ")": case "(": {
            const s = sentenceStarts()
            let q = p
            for (let k = 0; k < n; ++k) {
                if (mo.m === ")") { const x = s.find(v => v > q); q = x === undefined ? t.length : x }
                else { let x = -1; for (const v of s) if (v < q) x = v; q = x < 0 ? 0 : x }
            }
            return { pos: q }
        }
        case "`": case "'": {
            let pos = -1
            if (mo.ch === "`" || mo.ch === "'") pos = lastJump
            else if (mo.ch === "<" || mo.ch === ">") {
                if (!lastVisual) return null
                const a = Math.min(lastVisual.a, lastVisual.h), b = Math.max(lastVisual.a, lastVisual.h)
                pos = mo.ch === "<" ? a : b
            } else {
                ex.load()
                const ln = ex.markLine(mo.ch)
                if (ln >= 0) { const a = posOfLine(ln); pos = Math.min(lineEnd(a), a + (markCols[mo.ch] || 0)) }
            }
            if (pos < 0) { message("E20: Mark not set", true); return null }
            pos = Math.min(pos, t.length)
            return mo.m === "'" ? { pos: firstNonBlank(pos), linewise: true, jump: true } : { pos: pos, jump: true }
        }
        case "n": case "N": {
            const keep = edit.cursorPosition
            edit.cursorPosition = p
            let msg = ""
            for (let k = 0; k < n; ++k) { msg = ex.searchNext(mo.m === "N"); if (msg.startsWith("E")) break }
            const q = edit.cursorPosition
            edit.cursorPosition = keep
            if (msg.startsWith("E")) { message(msg, true); return null }
            return { pos: q, jump: true, msg: msg }
        }
        }
        return null
    }

    // ---- text objects: {a, b (past the end), linewise} or null ------------------
    function objectRange(o, p, n) {
        const around = o[0] === "a", k = o[1]
        switch (k) {
        case "w": case "W": return wordObject(p, k === "W", around, n)
        case "s": return sentenceObject(p, around)
        case "p": return paraObject(p, around, n)
        case "\"": case "'": case "`": return quoteObject(k, p, around)
        case "(": case ")": case "b": return bracketObject("(", ")", p, around, n)
        case "[": case "]": return bracketObject("[", "]", p, around, n)
        case "{": case "}": case "B": return bracketObject("{", "}", p, around, n)
        case "<": case ">": return bracketObject("<", ">", p, around, n)
        case "t": return tagObject(p, around, n)
        }
        return null
    }
    function wordObject(p, big, around, n) {
        const t = edit.text, ls = lineStart(p), le = lineEnd(p)
        if (le === ls) return null
        p = Math.min(p, le - 1)
        const kind = (i) => isBlank(t[i]) ? 0 : big ? 1 : isWord(t[i]) ? 2 : 1
        let a = p, b = p
        const c = kind(p)
        while (a > ls && kind(a - 1) === c) a--
        while (b < le && kind(b) === c) b++
        if (!around) {
            for (let k = 1; k < n && b < le; ++k) { const c2 = kind(b); while (b < le && kind(b) === c2) b++ }
            return { a: a, b: b }
        }
        if (c === 0) {
            if (b < le) { const c2 = kind(b); while (b < le && kind(b) === c2) b++ }
        } else {
            let b2 = b
            while (b2 < le && kind(b2) === 0) b2++
            if (b2 > b) b = b2
            else { let a2 = a; while (a2 > ls && kind(a2 - 1) === 0) a2--; if (a2 > ls) a = a2 }
        }
        for (let k = 1; k < n && b < le; ++k) {
            const c2 = kind(b); while (b < le && kind(b) === c2) b++
            while (b < le && kind(b) === 0) b++
        }
        return { a: a, b: b }
    }
    function sentenceObject(p, around) {
        const t = edit.text, s = sentenceStarts()
        let a = 0
        for (const v of s) if (v <= p) a = v
        const nx = s.find(v => v > p)
        let b = nx === undefined ? t.length : nx
        if (!around) while (b > a && /\s/.test(t[b - 1])) b--
        return { a: a, b: b }
    }
    function paraObject(p, around, n) {
        const lines = edit.text.split("\n"), N = lines.length
        const blank = (l) => lines[l].trim() === ""
        const L = lineOf(p)
        let s = L, e = L
        const b = blank(L)
        while (s > 0 && blank(s - 1) === b) s--
        while (e + 1 < N && blank(e + 1) === b) e++
        for (let k = 1; k < n && e + 1 < N; ++k) { const b2 = blank(e + 1); e++; while (e + 1 < N && blank(e + 1) === b2) e++ }
        if (around) {
            if (!b) {
                let e2 = e
                while (e2 + 1 < N && blank(e2 + 1)) e2++
                if (e2 > e) e = e2
                else while (s > 0 && blank(s - 1)) s--
            } else while (e + 1 < N && !blank(e + 1)) e++
        }
        return linesRange(s, e)
    }
    function quoteObject(q, p, around) {
        const t = edit.text, ls = lineStart(p), le = lineEnd(p)
        const qs = []
        for (let i = ls; i < le; ++i) if (t[i] === q && !(i > ls && t[i - 1] === "\\")) qs.push(i)
        let o = -1, c = -1
        for (let k = 0; k + 1 < qs.length; k += 2) if (qs[k] <= p && p <= qs[k + 1]) { o = qs[k]; c = qs[k + 1]; break }
        if (o < 0) for (let k = 0; k + 1 < qs.length; ++k) if (qs[k] > p) { o = qs[k]; c = qs[k + 1]; break }
        if (o < 0) return null
        if (!around) return { a: o + 1, b: c }
        let a = o, b = c + 1
        if (b < le && isBlank(t[b])) { while (b < le && isBlank(t[b])) b++ }
        else { let a2 = a; while (a2 > ls && isBlank(t[a2 - 1])) a2--; if (a2 > ls) a = a2 }
        return { a: a, b: b }
    }
    function enclosingOpen(open, close, from) {
        const t = edit.text
        if (from < 0) return -1
        if (t[from] === open) return from
        let depth = 0
        for (let i = from - 1; i >= 0; --i) {
            if (t[i] === close) depth++
            else if (t[i] === open) { if (depth === 0) return i; depth-- }
        }
        return -1
    }
    function closeOf(open, close, o) {
        const t = edit.text
        let depth = 0
        for (let i = o; i < t.length; ++i) {
            if (t[i] === open) depth++
            else if (t[i] === close && --depth === 0) return i
        }
        return -1
    }
    // The pair's {o, c} around p (n levels out); not in one: the next on the line.
    function bracketPair(open, close, p, n) {
        const t = edit.text
        let o = -1, from = p
        for (let k = 0; k < n; ++k) {
            const x = enclosingOpen(open, close, from)
            if (x < 0) { if (k === 0) o = -1; break }
            o = x; from = x - 1
            if (closeOf(open, close, o) < 0) { o = -1; break }
        }
        if (o < 0) {
            const i = t.indexOf(open, p)
            if (i < 0 || i >= lineEnd(p)) return null
            o = i
        }
        const c = closeOf(open, close, o)
        return c < 0 ? null : { o: o, c: c }
    }
    function bracketObject(open, close, p, around, n) {
        const pr = bracketPair(open, close, p, n)
        if (!pr) return null
        if (around) return { a: pr.o, b: pr.c + 1 }
        const t = edit.text
        let a = pr.o + 1, b = pr.c
        // (Over lines, "{" ending its line and "}" starting its own: the lines between.)
        if (t[a] === "\n" && /^[ \t]*$/.test(t.substring(lineStart(b), b)) && lineStart(b) > a + 1)
            return { a: a + 1, b: lineStart(b), linewise: true }
        return { a: a, b: b }
    }
    function tagPairs() {
        const t = edit.text, re = /<(\/?)([A-Za-z][\w:.-]*)(?:\s[^<>]*?)?(\/?)>/g
        const stack = [], pairs = []
        let m
        while ((m = re.exec(t)) !== null) {
            if (m[3] === "/") continue
            if (m[1] === "") stack.push({ name: m[2], s: m.index, e: re.lastIndex })
            else for (let k = stack.length - 1; k >= 0; --k) if (stack[k].name === m[2]) {
                const o = stack[k]
                stack.length = k
                pairs.push({ os: o.s, oe: o.e, cs: m.index, ce: re.lastIndex })
                break
            }
        }
        return pairs
    }
    function tagObject(p, around, n) {
        const pr = tagPairs().filter(x => x.os <= p && p < x.ce).sort((x, y) => (x.ce - x.os) - (y.ce - y.os))[n - 1]
        if (!pr) return null
        return around ? { a: pr.os, b: pr.ce } : { a: pr.oe, b: pr.cs }
    }

    // ---- operators ---------------------------------------------------------------
    function operate(r) {
        const t = edit.text, p = edit.cursorPosition
        const counted = r.count > 0 || r.count2 > 0
        const n = Math.max(1, r.count) * Math.max(1, r.count2)
        let rg
        if (r.line) {
            const L = lineOf(p)
            const L2 = Math.min(lineCount() - 1, L + n - 1)
            if (r.op === "ys") {
                let a = firstNonBlank(p), b = lineEnd(posOfLine(L2))
                while (b > a && isBlank(t[b - 1])) b--
                rg = { a: a, b: b }
            } else rg = linesRange(L, L2)
        } else if (r.obj) {
            rg = objectRange(r.obj, p, n)
            if (!rg) return
        } else {
            let mo = r.motion
            if (r.op === "c" && (mo.m === "w" || mo.m === "W") && t[p] !== undefined && cls(t[p]) !== 0) {
                // (cw: to the word's end, as ce, the word under the cursor counting as the first.)
                const big = mo.m === "W"
                let q = p
                for (let k = 0; k < n; ++k) {
                    if (k === 0 && cls(t[q + 1], big) !== cls(t[q], big)) continue
                    q = wordEndFwd(q, big)
                }
                rg = { a: p, b: Math.min(t.length, q + 1) }
            } else {
                const m = motionTarget(mo, p, counted ? n : 0, true)
                if (!m || m.edge) return
                let a = Math.min(p, m.pos), b = Math.max(p, m.pos)
                if (m.from !== undefined) a = Math.min(a, m.from)
                if (m.linewise) rg = linesRange(lineOf(a), lineOf(b))
                else {
                    if (m.inclusive) b = Math.min(t.length, b + 1)
                    else if (m.pos > p && lineOf(m.pos) > lineOf(p)) {
                        // (An exclusive motion onto a later line stops at the line before's end.)
                        if (mo.m === "w" || mo.m === "W") { const e = lineStart(m.pos) - 1; if (e > a) b = e }
                        else if (m.pos === lineStart(m.pos)) b = m.pos - 1
                    }
                    rg = { a: a, b: b }
                }
            }
        }
        applyOp(r.op, rg, r, n)
    }
    function applyOp(op, rg, r, n) {
        const p = edit.cursorPosition
        switch (op) {
        case "y":
            yank(r.reg, rg)
            if (!rg.linewise) setCursor(rg.a)
            else if (rg.a < lineStart(p)) setCursor(Math.min(rg.a + (p - lineStart(p)), lineEnd(rg.a)))
            return
        case "d": deleteRange(r.reg, rg); return
        case "c": change(r.reg, rg, r); return
        case ">": case "<": shiftLines(lineOf(rg.a), lineOf(Math.max(rg.a, rg.b - 1)), op === ">" ? 1 : -1, 1); return
        case "gu": case "gU": case "g~": case "g?": {
            const s = edit.text.substring(rg.a, rg.b)
            replace(rg.a, rg.b, caseOp(op, s))
            setCursor(rg.linewise ? Math.max(rg.a, Math.min(p, lineEnd(rg.a))) : rg.a)
            return
        }
        case "ys": surroundRange(rg, r.target); return
        }
    }
    function toggleCase(s) { let o = ""; for (const c of s) o += c === c.toUpperCase() ? c.toLowerCase() : c.toUpperCase(); return o }
    function caseOp(op, s) {
        if (op === "gu" || op === "u") return s.toLowerCase()
        if (op === "gU" || op === "U") return s.toUpperCase()
        if (op === "g?") return s.replace(/[a-zA-Z]/g, c => String.fromCharCode((c <= "Z" ? 90 : 122) >= (c = c.charCodeAt(0) + 13) ? c : c - 26))
        return toggleCase(s)
    }

    // ---- registers ---------------------------------------------------------------
    // "+ and "* are the system clipboard (through a hidden field's copy and
    // paste, as elsewhere here); the rest live in ExCommands.
    function isClip(reg) { return reg === "+" || reg === "*" }
    readonly property TextEdit clip: TextEdit { visible: false; textFormat: TextEdit.PlainText }
    property string clipLast: ""        // (what we put there, so a block or lines paste back as one)
    function clipSet(t) { clip.text = t; clip.selectAll(); clip.copy(); clip.text = ""; clipLast = t }
    function clipGet() {
        clip.text = ""
        clip.paste()
        const t = clip.text.replace(/\u2029/g, "\n")
        clip.text = ""
        return t
    }
    function regName(reg) { return reg === "" || reg === "\"" ? "\"" : isClip(reg) ? "+" : reg.toLowerCase() }
    function putReg(reg, text, kind, isYank) {
        if (reg === "_") return
        if (isClip(reg)) clipSet(text)
        ex.putRegister(reg === "\"" || isClip(reg) ? "" : reg, text)
        const rk = Object.assign({}, regKind)
        rk["\""] = kind
        if (reg !== "") rk[regName(reg)] = kind
        if (isYank) {
            const rr = Object.assign({}, ex.registers); rr["0"] = text; ex.registers = rr
            rk["0"] = kind
        }
        regKind = rk
    }
    function getReg(reg) {
        const name = regName(reg)
        if (name === "+") {
            const t = clipGet()
            return { text: t, kind: t === clipLast && regKind["+"] ? regKind["+"] : (t.endsWith("\n") ? "line" : "char") }
        }
        const text = name === "\"" ? ex.killRing : (ex.registers[name] || "")
        return { text: text, kind: regKind[name] || (text.endsWith("\n") ? "line" : "char") }
    }
    function rangeText(rg) {
        let s = edit.text.substring(rg.a, rg.b)
        if (rg.linewise && !s.endsWith("\n")) s += "\n"
        return s
    }
    function yank(reg, rg) { putReg(reg, rangeText(rg), rg.linewise ? "line" : "char", true) }
    function deleteRange(reg, rg) {
        const t = edit.text
        putReg(reg, rangeText(rg), rg.linewise ? "line" : "char", false)
        let a = rg.a
        if (rg.linewise && rg.b === t.length && t[rg.b - 1] !== "\n" && a > 0) a--
        replace(a, rg.b, "")
        setCursor(rg.linewise ? firstNonBlank(Math.min(a === rg.a ? a : lineStart(a), edit.length)) : a)
    }
    function change(reg, rg, cmd) {
        const t = edit.text
        putReg(reg, rangeText(rg), rg.linewise ? "line" : "char", false)
        if (rg.linewise) {
            const ind = indentOf(rg.a)
            const nl = rg.b > rg.a && t[rg.b - 1] === "\n" ? "\n" : ""
            replace(rg.a, rg.b, ind + nl)
            setCursor(rg.a + ind.length)
        } else {
            replace(rg.a, rg.b, "")
            setCursor(rg.a)
        }
        enterInsert(cmd, 1)
    }
    function put(reg, after, n, gMode) {
        const R = getReg(reg)
        if (R.text === "") return
        const p = edit.cursorPosition, t = edit.text
        if (R.kind === "line") {
            const body = (R.text.endsWith("\n") ? R.text : R.text + "\n").repeat(n)
            if (after) {
                const e = lineEnd(p)
                if (e >= t.length) {
                    edit.insert(e, "\n" + body.slice(0, -1))
                    setCursor(gMode ? edit.length : firstNonBlank(e + 1))
                } else {
                    edit.insert(e + 1, body)
                    setCursor(gMode ? e + 1 + body.length : firstNonBlank(e + 1))
                }
            } else {
                const a = lineStart(p)
                edit.insert(a, body)
                setCursor(gMode ? a + body.length : firstNonBlank(a))
            }
            return
        }
        if (R.kind === "block") { putBlock(R.text, after, n); return }
        const body = R.text.repeat(n)
        const at = after && p < lineEnd(p) ? p + 1 : p
        edit.insert(at, body)
        setCursor(gMode ? at + body.length : body.indexOf("\n") >= 0 ? at : at + body.length - 1)
    }
    function putBlock(text, after, n) {
        const p = edit.cursorPosition
        const parts = text.split("\n")
        const w = Math.max.apply(null, parts.map(s => s.length))
        const L = lineOf(p)
        const col = (after && p < lineEnd(p) ? p + 1 : p) - lineStart(p)
        const lines = edit.text.split("\n")
        const last = L + parts.length - 1
        while (lines.length <= last) lines.push("")
        for (let i = 0; i < parts.length; ++i) {
            let line = lines[L + i]
            if (line.length < col) line += " ".repeat(col - line.length)
            const piece = parts[i].repeat(1)
            let ins = ""
            for (let k = 0; k < n; ++k) ins += piece + (k < n - 1 || line.length > col ? " ".repeat(w - piece.length) : "")
            lines[L + i] = line.substring(0, col) + ins + line.substring(col)
        }
        const oldLast = Math.min(last, lineCount() - 1)
        const extra = lines.slice(L, last + 1)
        const a = posOfLine(L), b = lineEnd(posOfLine(oldLast))
        replace(a, b, extra.join("\n"))
        setCursor(a + col)
    }

    // ---- other changes ---------------------------------------------------------------
    function shiftLines(l1, l2, dir, times) {
        const lines = edit.text.split("\n")
        const out = []
        for (let l = l1; l <= l2; ++l) {
            let s = lines[l]
            for (let k = 0; k < times; ++k) {
                if (dir > 0) { if (s !== "") s = indentUnit + s }
                else s = s.replace(new RegExp("^( {1," + Math.max(1, indentUnit.length) + "}|\t)"), "")
            }
            out.push(s)
        }
        replaceLines(l1, l2, out)
        setCursor(firstNonBlank(posOfLine(l1)))
    }
    function join(L, count, spaces) {
        const lines = edit.text.split("\n")
        const last = Math.min(lines.length - 1, L + count - 1)
        if (last === L) return
        let s = lines[L], at = 0
        for (let l = L + 1; l <= last; ++l) {
            let nx = lines[l]
            at = s.length
            if (spaces) {
                nx = nx.replace(/^\s+/, "")
                if (nx !== "" && s !== "" && !/[ \t]$/.test(s) && nx[0] !== ")") nx = " " + nx
            }
            s += nx
        }
        replaceLines(L, last, [s])
        setCursor(posOfLine(L) + at)
    }
    function increment(p, delta) {
        const t = edit.text, ls = lineStart(p), le = lineEnd(p), line = t.substring(ls, le)
        const re = /0[xX][0-9a-fA-F]+|-?\d+/g
        let m, found = null
        while ((m = re.exec(line)) !== null) if (ls + m.index + m[0].length > p) { found = m; break }
        if (!found) return
        let s = found[0], idx = found.index
        if (s[0] === "-" && idx > 0 && isWord(line[idx - 1])) { s = s.substring(1); idx++ }
        let out
        if (/^0[xX]/.test(s)) {
            const digits = s.substring(2)
            let v = (parseInt(digits, 16) + delta) % 0x100000000
            if (v < 0) v += 0x100000000
            out = v.toString(16)
            if (/[A-F]/.test(digits) && !/[a-f]/.test(digits)) out = out.toUpperCase()
            out = s.substring(0, 2) + out.padStart(digits.length, "0")
        } else {
            const neg = s[0] === "-", digits = neg ? s.substring(1) : s
            const v = parseInt(s, 10) + delta
            out = String(v)
            if (digits.length > 1 && digits[0] === "0") out = (v < 0 ? "-" : "") + String(Math.abs(v)).padStart(digits.length, "0")
        }
        replace(ls + idx, ls + idx + s.length, out)
        setCursor(ls + idx + out.length - 1)
    }

    // ---- surround.vim ------------------------------------------------------------------
    // The opening and closing text for a target: "(" "[" "{" with spaces
    // inside, ")" "]" "}" b B r a ">" without, a tag, or the character twice.
    function surroundPair(target) {
        if (target.length > 1 && target[0] === "<") {
            const inner = target.slice(1, -1)
            const name = inner.split(/\s/)[0]
            return [target, "</" + name + ">"]
        }
        switch (target) {
        case "(": return ["( ", " )"]
        case ")": case "b": return ["(", ")"]
        case "[": return ["[ ", " ]"]
        case "]": case "r": return ["[", "]"]
        case "{": return ["{ ", " }"]
        case "}": case "B": return ["{", "}"]
        case ">": case "a": return ["<", ">"]
        }
        return [target, target]
    }
    function surroundRange(rg, target) {
        const t = edit.text
        const [o, c] = surroundPair(target)
        if (rg.linewise) {
            const l1 = lineOf(rg.a), l2 = lineOf(Math.max(rg.a, rg.b - 1))
            const ind = indentOf(rg.a)
            const lines = t.split("\n").slice(l1, l2 + 1).map(s => s === "" ? s : indentUnit + s)
            replaceLines(l1, l2, [ind + o.trim()].concat(lines, [ind + c.trim()]))
            setCursor(firstNonBlank(posOfLine(l1)))
            return
        }
        let a = rg.a, b = rg.b
        while (b > a && /\s/.test(t[b - 1])) b--
        while (a < b && /\s/.test(t[a])) a++
        replace(a, b, o + t.substring(a, b) + c)
        setCursor(a)
    }
    // The pair `ch` names around the cursor: {a, b, ia, ib} (outer and inner).
    function surroundFind(ch, p, n) {
        const t = edit.text
        const open = ({ "(": "(", ")": "(", b: "(", "[": "[", "]": "[", r: "[", "{": "{", "}": "{", B: "{", "<": "<", ">": "<", a: "<" })[ch]
        if (open) {
            const pr = bracketPair(open, bracketPairs[open] || ">", p, n)
            if (!pr) return null
            let ia = pr.o + 1, ib = pr.c
            if ("([{".indexOf(ch) >= 0) { while (ia < ib && isBlank(t[ia])) ia++; while (ib > ia && isBlank(t[ib - 1])) ib-- }
            return { a: pr.o, b: pr.c + 1, ia: ia, ib: ib }
        }
        if (ch === "t") {
            const x = tagObject(p, true, n), y = tagObject(p, false, n)
            return x && y ? { a: x.a, b: x.b, ia: y.a, ib: y.b } : null
        }
        const q = quoteObject(ch, p, false)
        return q ? { a: q.a - 1, b: q.b + 1, ia: q.a, ib: q.b } : null
    }
    function deleteSurround(ch, n) {
        const s = surroundFind(ch, edit.cursorPosition, n)
        if (!s) return
        replace(s.a, s.b, edit.text.substring(s.ia, s.ib))
        setCursor(s.a)
    }
    function changeSurround(ch, target, n) {
        const s = surroundFind(ch, edit.cursorPosition, n)
        if (!s) return
        const [o, c] = surroundPair(target)
        replace(s.a, s.b, o + edit.text.substring(s.ia, s.ib) + c)
        setCursor(s.a)
    }

    // ---- dot ------------------------------------------------------------------------
    function repeatChange(r) {
        const lc = lastChange
        if (!lc) return
        const c = Object.assign({}, lc.cmd)
        if (r.count > 0) { c.count = r.count; c.count2 = 0 }
        if (r.reg !== "") c.reg = r.reg
        replaying = true
        try {
            runNormal(c)
            if (!viNormal) {
                if (lc.replace) overwrite(lc.text)
                else if (lc.text !== "") {
                    const p = edit.cursorPosition
                    edit.insert(p, lc.text)
                    edit.cursorPosition = p + lc.text.length
                }
                if (ins) ins.typed = lc.text
                leaveInsert()
            }
        } finally { replaying = false }
    }

    // ---- visual ---------------------------------------------------------------------
    function startVisual(mode) {
        visual = mode
        vAnchor = vHead = edit.cursorPosition
    }
    function endVisual() {
        lastVisual = { mode: visual, a: vAnchor, h: vHead }
        const h = Math.min(vHead, edit.length)
        visual = ""
        blockRects = []
        edit.deselect()
        setCursor(h)
        selSnap = null
        clampCursor()
    }
    // (An operator done: the selection is gone, the cursor where the operator left it.)
    function dropVisual() {
        lastVisual = { mode: visual, a: vAnchor, h: vHead }
        visual = ""
        blockRects = []
        selSnap = null
    }
    function visualRange() {
        const t = edit.text
        const a = Math.min(vAnchor, vHead), h = Math.max(vAnchor, vHead)
        if (visual === "v") return { a: a, b: Math.min(t.length, h + 1) }
        if (visual === "V") return linesRange(lineOf(a), lineOf(h))
        const ca = vAnchor - lineStart(vAnchor), ch = vHead - lineStart(vHead)
        return { block: true, l1: lineOf(a), l2: lineOf(h), c1: Math.min(ca, ch), c2: Math.max(ca, ch), dollar: wantCol >= 1e9 }
    }
    function updateVisual() {
        if (visual === "") { if (blockRects.length) blockRects = []; return }
        const t = edit.text
        vHead = Math.max(0, Math.min(t.length, vHead)); vAnchor = Math.max(0, Math.min(t.length, vAnchor))
        if (visual === "b") {
            edit.deselect()
            edit.cursorPosition = vHead
            const r = visualRange(), out = []
            for (let l = r.l1; l <= r.l2; ++l) {
                const ls = posOfLine(l), len = lineEnd(ls) - ls
                const c1 = Math.min(r.c1, len), c2 = r.dollar ? len : Math.min(r.c2 + 1, len)
                const x1 = edit.positionToRectangle(ls + c1)
                const x2 = edit.positionToRectangle(ls + c2)
                if (c2 <= c1 && len < r.c1) continue
                out.push({ x: x1.x, y: x1.y, w: Math.max(2, x2.x - x1.x), h: x1.height })
            }
            blockRects = out
        } else {
            blockRects = []
            const rg = visualRange(), b = rg.b
            if (vHead >= vAnchor) { edit.cursorPosition = rg.a; edit.moveCursorSelection(b, TextEdit.SelectCharacters) }
            else { edit.cursorPosition = b; edit.moveCursorSelection(rg.a, TextEdit.SelectCharacters) }
        }
        selSnap = { s: edit.selectionStart, e: edit.selectionEnd, c: edit.cursorPosition }
    }
    // A selection made otherwise (the mouse, Shift+arrows) is a visual one; a click ends it.
    function syncSelection() {
        const s = edit.selectionStart, e = edit.selectionEnd, c = edit.cursorPosition
        if (visual !== "" && selSnap && selSnap.s === s && selSnap.e === e && selSnap.c === c) return
        if (visual === "b" && selSnap && s === e && c === selSnap.c) return
        if (s === e) {
            if (visual !== "") { visual = ""; blockRects = []; selSnap = null }
            return
        }
        visual = "v"
        if (c === e) { vAnchor = s; vHead = Math.max(s, e - 1) } else { vAnchor = Math.max(s, e - 1); vHead = s }
        selSnap = { s: s, e: e, c: c }
    }
    function runVisual(r) {
        const n = Math.max(1, r.count)
        if (r.motion) {
            const m = motionTarget(r.motion, vHead, r.count, false)
            if (!m || m.edge) return
            if (m.jump) lastJump = vHead
            vHead = m.pos
            if (m.col !== undefined) wantCol = m.col; else wantCol = r.motion.m === "$" ? 1e9 : -1
            if (m.msg) message(m.msg, m.msg.startsWith("E"))
            return
        }
        if (r.obj) {
            let o = objectRange(r.obj, vHead, n)
            const cur = visualRange()
            // (The same object selected already: the one around it.)
            if (o && vAnchor !== vHead && o.a === cur.a && o.b === cur.b) o = objectRange(r.obj, vHead, n + 1)
            if (!o) return
            if (o.linewise && visual === "v") visual = "V"
            if (vAnchor === vHead || (o.a <= Math.min(vAnchor, vHead) && o.b > Math.max(vAnchor, vHead))) { vAnchor = o.a; vHead = Math.max(o.a, o.b - 1) }
            else if (vHead > vAnchor) vHead = Math.max(vHead, o.b - 1)
            else vHead = Math.min(vHead, o.a)
            return
        }
        const rg = visualRange()
        const t = edit.text
        const mode = visual
        switch (r.name) {
        case "o": { const x = vAnchor; vAnchor = vHead; vHead = x; return }
        case "O": {
            if (mode !== "b") { const x = vAnchor; vAnchor = vHead; vHead = x; return }
            const ca = vAnchor - lineStart(vAnchor), ch = vHead - lineStart(vHead)
            vAnchor = Math.min(lineEnd(vAnchor), lineStart(vAnchor) + ch)
            vHead = Math.min(lineEnd(vHead), lineStart(vHead) + ca)
            return
        }
        case "v": case "V": case "<C-v>": case "<C-q>": {
            const m = r.name === "v" ? "v" : r.name === "V" ? "V" : "b"
            if (m === visual) endVisual(); else visual = m
            return
        }
        case "gv": {
            if (!lastVisual) return
            const cur = { mode: visual, a: vAnchor, h: vHead }
            visual = lastVisual.mode; vAnchor = lastVisual.a; vHead = lastVisual.h
            lastVisual = cur
            return
        }
        case ":": {
            dropVisual()
            if (rg.block) edit.select(posOfLine(rg.l1), lineEnd(posOfLine(rg.l2)))
            commandLine(":", "'<,'>")
            return
        }
        }
        dropVisual()
        edit.deselect()
        if (rg.block) { blockOp(r, rg, n); return }
        const lines = linesRange(lineOf(rg.a), lineOf(Math.max(rg.a, rg.b - 1)))
        switch (r.name) {
        case "d": case "x": case "<del>": deleteRange(r.reg, rg); return
        case "X": case "D": deleteRange(r.reg, lines); return
        case "c": case "s": change(r.reg, rg, null); return
        case "C": case "R": change(r.reg, lines, null); return
        case "y": yank(r.reg, rg); setCursor(rg.a); return
        case "Y": yank(r.reg, lines); setCursor(rg.a); return
        case ">": case "<": shiftLines(lineOf(rg.a), lineOf(Math.max(rg.a, rg.b - 1)), r.name === ">" ? 1 : -1, n); return
        case "~": case "u": case "U": case "g~": case "gu": case "gU": case "g?":
            replace(rg.a, rg.b, caseOp(r.name.length === 1 ? (r.name === "~" ? "g~" : r.name) : r.name, t.substring(rg.a, rg.b)))
            setCursor(rg.a)
            return
        case "J": case "gJ": {
            const l1 = lineOf(rg.a), l2 = lineOf(Math.max(rg.a, rg.b - 1))
            join(l1, Math.max(2, l2 - l1 + 1), r.name === "J")
            return
        }
        case "r": {
            const s = t.substring(rg.a, rg.b).replace(/[^\n]/g, r.ch)
            replace(rg.a, rg.b, s)
            setCursor(rg.a)
            return
        }
        case "p": case "P": {
            const R = getReg(r.reg)
            if (r.name === "p") putReg("", rangeText(rg), rg.linewise ? "line" : "char", false)
            let body = R.text
            if (R.kind === "line" && !rg.linewise) body = "\n" + body.replace(/\n$/, "") + "\n"
            else if (R.kind !== "line" && rg.linewise) body = body + "\n"
            replace(rg.a, rg.b, body)
            setCursor(rg.a)
            return
        }
        case "I": setCursor(rg.linewise ? firstNonBlank(rg.a) : rg.a); enterInsert(null, 1); return
        case "A": setCursor(rg.linewise ? lineEnd(Math.max(rg.a, rg.b - 1)) : rg.b); enterInsert(null, 1); return
        case "S": surroundRange(rg, r.target); return
        case "<C-a>": case "<C-x>": {
            const l1 = lineOf(rg.a), l2 = lineOf(Math.max(rg.a, rg.b - 1))
            for (let l = l1; l <= l2; ++l) {
                const a = posOfLine(l)
                increment(l === l1 ? rg.a : a, r.name === "<C-a>" ? n : -n)
            }
            setCursor(rg.a)
            return
        }
        }
    }
    // A block's operators: each line's part of it.
    function blockOp(r, rg, n) {
        const lines = edit.text.split("\n")
        const parts = []
        const seg = (l) => {
            const s = lines[l], a = Math.min(rg.c1, s.length), b = rg.dollar ? s.length : Math.min(rg.c2 + 1, s.length)
            return [a, b]
        }
        for (let l = rg.l1; l <= rg.l2; ++l) { const [a, b] = seg(l); parts.push(lines[l].substring(a, b)) }
        const at = posOfLine(rg.l1) + Math.min(rg.c1, lines[rg.l1].length)
        const each = (f) => {
            const out = []
            for (let l = rg.l1; l <= rg.l2; ++l) { const [a, b] = seg(l); out.push(f(lines[l], a, b)) }
            replaceLines(rg.l1, rg.l2, out)
        }
        switch (r.name) {
        case "y": case "Y": putReg(r.reg, parts.join("\n"), "block", true); setCursor(at); return
        case "d": case "x": case "<del>":
            putReg(r.reg, parts.join("\n"), "block", false)
            each((s, a, b) => s.substring(0, a) + s.substring(b))
            setCursor(at)
            return
        case "D": case "X":
            putReg(r.reg, parts.join("\n"), "block", false)
            each((s, a, b) => s.substring(0, a))
            setCursor(at)
            return
        case "c": case "s":
            putReg(r.reg, parts.join("\n"), "block", false)
            each((s, a, b) => s.substring(0, a) + s.substring(b))
            setCursor(at)
            enterInsert(null, 1, { block: { l1: rg.l1, l2: rg.l2, col: rg.c1 } })
            return
        case "C":
            each((s, a, b) => s.substring(0, a))
            setCursor(at)
            enterInsert(null, 1, { block: { l1: rg.l1, l2: rg.l2, col: rg.c1 } })
            return
        case "I":
            setCursor(at)
            enterInsert(null, 1, { block: { l1: rg.l1, l2: rg.l2, col: rg.c1 } })
            return
        case "A": {
            const col = rg.dollar ? lines[rg.l1].length : rg.c2 + 1
            let s = lines[rg.l1]
            if (s.length < col) { replaceLines(rg.l1, rg.l1, [s + " ".repeat(col - s.length)]) }
            setCursor(posOfLine(rg.l1) + col)
            enterInsert(null, 1, { block: { l1: rg.l1, l2: rg.l2, col: col, append: true, dollar: rg.dollar } })
            return
        }
        case "r": each((s, a, b) => s.substring(0, a) + s.substring(a, b).replace(/./g, r.ch) + s.substring(b)); setCursor(at); return
        case "~": case "u": case "U": case "g~": case "gu": case "gU": case "g?": {
            const op = r.name.length === 1 ? (r.name === "~" ? "g~" : r.name) : r.name
            each((s, a, b) => s.substring(0, a) + caseOp(op, s.substring(a, b)) + s.substring(b))
            setCursor(at)
            return
        }
        case ">": case "<": shiftLines(rg.l1, rg.l2, r.name === ">" ? 1 : -1, n); return
        case "J": case "gJ": join(rg.l1, Math.max(2, rg.l2 - rg.l1 + 1), r.name === "J"); return
        case "S": {
            const [o, c] = surroundPair(r.target)
            each((s, a, b) => b > a ? s.substring(0, a) + o + s.substring(a, b) + c + s.substring(b) : s)
            setCursor(at)
            return
        }
        case "p": case "P": {
            const R = getReg(r.reg)
            each((s, a, b) => s.substring(0, a) + s.substring(b))
            setCursor(at)
            if (R.kind === "block") putBlock(R.text, false, 1)
            else edit.insert(at, R.text)
            return
        }
        }
    }

    // ---- undo -----------------------------------------------------------------------
    function diffOf(o, n) {
        let a = 0
        while (a < o.length && a < n.length && o[a] === n[a]) a++
        let b = 0
        while (b < o.length - a && b < n.length - a && o[o.length - 1 - b] === n[n.length - 1 - b]) b++
        return { a: a, del: o.substring(a, o.length - b), ins: n.substring(a, n.length - b) }
    }
    function pushUndo(o, n, cur) {
        if (o === n) return
        undoStack = undoStack.concat([{ d: diffOf(o, n), cur: cur }]).slice(-500)
        redoStack = []
    }
    function beginChange() { if (changeBase === null) changeBase = { text: edit.text, cur: edit.cursorPosition } }
    function endChange() {
        if (changeBase === null || !edit) return
        pushUndo(changeBase.text, edit.text, changeBase.cur)
        changeBase = null
        lastKnown = edit.text
    }
    // (A change from elsewhere, a : command, the history, typing begun in insert mode: one step.)
    function syncExternal() {
        if (!edit) return
        if (lastKnown === null) { lastKnown = edit.text; return }
        if (changeBase === null && edit.text !== lastKnown) { pushUndo(lastKnown, edit.text, edit.cursorPosition); lastKnown = edit.text }
    }
    function undo(n) {
        if (visual !== "") endVisual()
        let done = 0
        for (; done < n && undoStack.length; ++done) {
            const u = undoStack[undoStack.length - 1]
            undoStack = undoStack.slice(0, -1)
            const d = u.d
            replace(d.a, d.a + d.ins.length, d.del)
            setCursor(d.a)
            redoStack = redoStack.concat([u])
        }
        lastKnown = edit.text
        if (done === 0) message("Already at oldest change", false)
    }
    function redo(n) {
        if (visual !== "") endVisual()
        let done = 0
        for (; done < n && redoStack.length; ++done) {
            const u = redoStack[redoStack.length - 1]
            redoStack = redoStack.slice(0, -1)
            const d = u.d
            replace(d.a, d.a + d.del.length, d.ins)
            setCursor(d.a)
            undoStack = undoStack.concat([u])
        }
        lastKnown = edit.text
        if (done === 0) message("Already at newest change", false)
    }
}
