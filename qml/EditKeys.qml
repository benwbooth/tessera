import QtQuick

// Editing keys for a text field, Emacs, VS Code or vi (`keys`), the same as
// the input editor's: line and word motions, a kill ring, brackets and
// quotes in pairs, indenting a line or a selection, vi's normal mode. A
// file card's editor uses it; `handle(e)` from the field's Keys.onPressed
// takes a key or leaves it for the field's own (TextEdit) handling.
QtObject {
    id: ek
    property var edit: null             // the TextEdit
    property string keys: "emacs"       // "emacs", "vscode", "vi"
    // vi's normal and visual modes (ViKeys.qml).
    readonly property ViKeys vi: ViKeys {
        id: vik
        edit: ek.edit
        ex: exc
        indentUnit: ek.indentUnit
        onCommandLine: (kind, initial) => ek.commandLine(kind, initial)
        onMessage: (text, error) => ek.message(text)
    }
    property alias viNormal: vik.viNormal
    property alias viPending: vik.viPending
    // vi's ":" commands (ExCommands.qml), sharing the yanked text.
    readonly property ExCommands ex: ExCommands { id: exc; edit: ek.edit; indentUnit: ek.indentUnit; keyHandler: (e) => ek.handle(e); viKeys: ek }
    property alias killRing: exc.killRing
    // (vi's command line wanted: ":" (with '<,'> over a selection), or "/" and
    // "?" to search. The owner shows the field and calls ex.run or search.)
    signal commandLine(string kind, string initial)
    property string indentUnit: "    "
    property string commentPrefix: "# " // (Ctrl+/ in VS Code keys)
    readonly property var pairs: ({ "(": ")", "[": "]", "{": "}", "\"": "\"", "'": "'", "`": "`" })

    function isWord(c) { return /[A-Za-z0-9_]/.test(c) }
    function lineStart(p) { return edit.text.lastIndexOf("\n", p - 1) + 1 }
    function lineEnd(p) { const i = edit.text.indexOf("\n", p); return i < 0 ? edit.text.length : i }
    // (As VS Code moves by word: past spaces, then one run, either word
    // characters or punctuation ("${ is one), never over a line's end. Bash's
    // readline skips all punctuation, so print "${ went in one go.)
    function isSpace(c) { return c === " " || c === "\t" }
    function isBreak(c) { return c === "\n" || c === "\u2029" }
    function wordRight(p) {
        const t = edit.text, s = p
        while (p < t.length && isSpace(t[p])) p++
        if (p < t.length && isBreak(t[p])) return p > s ? p : p + 1
        if (p < t.length && isWord(t[p])) { while (p < t.length && isWord(t[p])) p++ }
        else { while (p < t.length && !isWord(t[p]) && !isSpace(t[p]) && !isBreak(t[p])) p++ }
        return p
    }
    function wordLeft(p) {
        const t = edit.text, s = p
        while (p > 0 && isSpace(t[p - 1])) p--
        if (p > 0 && isBreak(t[p - 1])) return p < s ? p : p - 1
        if (p > 0 && isWord(t[p - 1])) { while (p > 0 && isWord(t[p - 1])) p-- }
        else { while (p > 0 && !isWord(t[p - 1]) && !isSpace(t[p - 1]) && !isBreak(t[p - 1])) p-- }
        return p
    }
    function wordEnd(p) {
        const t = edit.text
        p++
        while (p < t.length && !isWord(t[p])) p++
        while (p + 1 < t.length && isWord(t[p + 1])) p++
        return Math.min(p, t.length)
    }
    function insert(s) {
        if (edit.selectedText !== "") edit.remove(edit.selectionStart, edit.selectionEnd)
        const p = edit.cursorPosition
        edit.insert(p, s)
        edit.cursorPosition = p + s.length
    }
    function kill(a, b, append) {
        if (b < a) { const x = a; a = b; b = x }
        const cut = edit.text.substring(a, b)
        killRing = append ? killRing + cut : cut
        edit.remove(a, b)
        edit.cursorPosition = a
    }
    function indentOf(p) {
        const a = lineStart(p)
        let i = a
        while (i < edit.text.length && (edit.text[i] === " " || edit.text[i] === "\t")) i++
        return edit.text.substring(a, i)
    }
    // A new line keeping the indentation, one deeper after an opener; between
    // a bracket pair the closer moves down.
    function newline() {
        const p = edit.cursorPosition
        const before = edit.text.substring(lineStart(p), p).replace(/\s+$/, "")
        const ind = indentOf(p)
        const opens = /[{(\[:]$/.test(before)
        const deeper = opens ? ind + indentUnit : ind
        const next = edit.text[p]
        if (opens && next && ")]}".indexOf(next) >= 0 && before.slice(-1) === Object.keys(pairs).find(k => pairs[k] === next)) {
            insert("\n" + deeper + "\n" + ind)
            edit.cursorPosition = p + 1 + deeper.length
            return
        }
        insert("\n" + deeper)
    }
    function shiftLines(dir) {
        const s = edit.selectionStart, e = edit.selectionEnd
        const a = lineStart(s)
        const lines = edit.text.substring(a, lineEnd(e)).split("\n")
        const out = lines.map(l => dir > 0 ? indentUnit + l : l.replace(new RegExp("^( {1," + indentUnit.length + "}|\t)"), ""))
        const block = out.join("\n")
        const pos = edit.cursorPosition, delta = out[0].length - lines[0].length
        edit.remove(a, lineEnd(e))
        edit.insert(a, block)
        if (s !== e) edit.select(a, a + block.length)
        else edit.cursorPosition = Math.max(a, pos + delta)
    }
    function moveLine(dir) {
        const t = edit.text, p = edit.cursorPosition, a = lineStart(p), b = lineEnd(p)
        const line = t.substring(a, b), col = p - a
        if (dir < 0 && a > 0) {
            const pa = lineStart(a - 1)
            edit.remove(pa, b)
            edit.insert(pa, line + "\n" + t.substring(pa, a - 1))
            edit.cursorPosition = pa + col
        } else if (dir > 0 && b < t.length) {
            const nb = lineEnd(b + 1), next = t.substring(b + 1, nb)
            edit.remove(a, nb)
            edit.insert(a, next + "\n" + line)
            edit.cursorPosition = a + next.length + 1 + col
        }
    }

    // Brackets and quotes in pairs, Tab/Shift+Tab indenting, Backspace in a pair.
    function smartKey(e) {
        if (e.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return false
        const p = edit.cursorPosition, t = edit.text
        if (e.key === Qt.Key_Backtab) { shiftLines(-1); return true }
        if (e.key === Qt.Key_Tab) {
            if (edit.selectedText.indexOf(" ") >= 0 || edit.selectedText.indexOf("\n") >= 0) { shiftLines(1); return true }
            insert(indentUnit)
            return true
        }
        if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { newline(); return true }
        if (e.key === Qt.Key_Backspace && p > 0 && !edit.selectedText) {
            const o = t[p - 1]
            if (pairs[o] && t[p] === pairs[o]) { edit.remove(p - 1, p + 1); return true }
            const lead = t.substring(lineStart(p), p)
            if (lead.length > 0 && /^ +$/.test(lead)) {
                const n = ((lead.length - 1) % indentUnit.length) + 1
                edit.remove(p - n, p)
                return true
            }
            return false
        }
        const ch = e.text
        if (ch.length !== 1) return false
        if (")]}\"'`".indexOf(ch) >= 0 && t[p] === ch && !edit.selectedText) {
            const quote = "\"'`".indexOf(ch) >= 0
            if (!quote || t[p - 1] !== "\\") { edit.cursorPosition = p + 1; return true }
        }
        if (pairs[ch]) {
            const sel = edit.selectedText
            if (sel) {
                const a = edit.selectionStart, b = edit.selectionEnd
                edit.remove(a, b)
                edit.insert(a, ch + sel + pairs[ch])
                edit.select(a + 1, a + 1 + sel.length)
                return true
            }
            const quote = "\"'`".indexOf(ch) >= 0
            const prev = t[p - 1] || " ", next = t[p] || " "
            if (quote && (isWord(prev) || prev === "\\" || isWord(next))) return false
            if (!quote && isWord(next)) return false
            insert(ch + pairs[ch])
            edit.cursorPosition = p + 1
            return true
        }
        return false
    }

    function emacsKey(e) {
        const ctrl = (e.modifiers & Qt.ControlModifier) && !(e.modifiers & Qt.ShiftModifier)
        const alt = e.modifiers & Qt.AltModifier
        const p = edit.cursorPosition
        if (ctrl) {
            switch (e.key) {
            case Qt.Key_A: edit.cursorPosition = lineStart(p); return true
            case Qt.Key_E: edit.cursorPosition = lineEnd(p); return true
            case Qt.Key_B: edit.cursorPosition = Math.max(0, p - 1); return true
            case Qt.Key_F: edit.cursorPosition = Math.min(edit.length, p + 1); return true
            case Qt.Key_P: { const a = lineStart(p), col = p - a; if (a > 0) { const pa = lineStart(a - 1); edit.cursorPosition = Math.min(pa + col, a - 1) } return true }
            case Qt.Key_N: { const a = lineStart(p), col = p - a, b = lineEnd(p); if (b < edit.length) { const nb = lineEnd(b + 1); edit.cursorPosition = Math.min(b + 1 + col, nb) } return true }
            case Qt.Key_K: kill(p, lineEnd(p) === p ? Math.min(edit.length, p + 1) : lineEnd(p)); return true
            case Qt.Key_U: kill(lineStart(p), p); return true
            case Qt.Key_W: kill(wordLeft(p), p); return true
            case Qt.Key_Y: insert(killRing); return true
            case Qt.Key_H: if (p > 0) edit.remove(p - 1, p); return true
            case Qt.Key_D: if (p < edit.length) edit.remove(p, p + 1); return true
            case Qt.Key_Underscore: case Qt.Key_Slash: exc.undo(); return true
            case Qt.Key_T:
                if (p > 0 && p < edit.length) {
                    const t = edit.text
                    edit.remove(p - 1, p + 1); edit.insert(p - 1, t[p] + t[p - 1]); edit.cursorPosition = p + 1
                }
                return true
            }
        }
        if (alt && !(e.modifiers & Qt.ControlModifier)) {
            switch (e.key) {
            case Qt.Key_F: edit.cursorPosition = wordRight(p); return true
            case Qt.Key_B: edit.cursorPosition = wordLeft(p); return true
            case Qt.Key_D: kill(p, wordRight(p)); return true
            case Qt.Key_Backspace: kill(wordLeft(p), p); return true
            case Qt.Key_Less: edit.cursorPosition = 0; return true
            case Qt.Key_Greater: edit.cursorPosition = edit.length; return true
            }
        }
        return false
    }

    function vscodeKey(e) {
        const ctrl = e.modifiers & Qt.ControlModifier
        const shift = e.modifiers & Qt.ShiftModifier
        const alt = e.modifiers & Qt.AltModifier
        const p = edit.cursorPosition
        if (ctrl && shift && e.key === Qt.Key_K) { kill(lineStart(p), Math.min(edit.length, lineEnd(p) + 1)); return true }
        if (alt && e.key === Qt.Key_Up) { moveLine(-1); return true }
        if (alt && e.key === Qt.Key_Down) { moveLine(1); return true }
        if (ctrl && e.key === Qt.Key_Slash) {
            const a = lineStart(p)
            if (edit.text.substring(a, a + commentPrefix.length) === commentPrefix) edit.remove(a, a + commentPrefix.length)
            else edit.insert(a, commentPrefix)
            return true
        }
        if (ctrl && e.key === Qt.Key_D && !shift) {
            // (Select the word, as a start for the next.)
            edit.select(wordLeft(p), wordRight(p))
            return true
        }
        return false
    }

    // "/" or "?" searching (ExCommands').
    function search(pat, back) { return exc.search(pat, back) }
    function searchNext(reverse) { return exc.searchNext(reverse) }
    signal message(string text)

    // A key: its editing meaning in the keys chosen, or false for the field.
    function handle(e) {
        if (!edit) return false
        if (keys === "vi") {
            if (vik.handle(e)) return true
            if (viNormal) return false
        }
        let done = keys === "vscode" ? vscodeKey(e) : emacsKey(e)
        if (!done) done = smartKey(e)
        return done
    }
}
