import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// The input editor: a small multi-line editor docked under the terminal while
// the shell (or a REPL) waits for input. Enter runs the command (it goes to
// the shell as if typed, and scrolls up with its output); Shift+Enter starts
// a new line. Highlighting and syntax errors come from tree-sitter; the keys
// follow Emacs (readline), vi, or VS Code.
Rectangle {
    id: box
    property var view: null            // the TerminalView it types into
    property var controller: null
    property string lang: "bash"
    property string keys: "emacs"      // "emacs", "vi", "vscode"
    property var history: []           // oldest first (shared by the window)
    property var themeData: ({})       // the terminal's theme (ansi colors)
    signal submitted(string text)
    signal handBack()                  // Esc: type into the terminal instead

    readonly property alias text: edit.text
    readonly property bool editing: edit.activeFocus
    readonly property real lineHeight: Math.ceil(fm.height)
    readonly property int lines: Math.max(1, edit.lineCount)
    implicitHeight: Math.min(lines, 10) * lineHeight + 14

    color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.045 : 0.035)
    Rectangle { width: parent.width; height: 1; color: Theme.divider }

    FontMetrics { id: fm; font: edit.font }

    function ansi(i, fallback) {
        const a = box.themeData && box.themeData.ansi
        return a && a[i] ? a[i] : fallback
    }

    // ---- editing ----------------------------------------------------------
    property int histIndex: -1         // -1: the draft, else into history
    property string draft: ""
    property string killRing: ""
    property bool viNormal: false
    property string viPending: ""      // an operator waiting for its motion (d, c, y, g, r)

    function focusEditor() { edit.forceActiveFocus() }
    function insert(t) { edit.insert(edit.cursorPosition, t) }
    function setText(t) { edit.text = t; edit.cursorPosition = t.length }

    function submit() {
        const t = edit.text
        if (t.trim() !== "") {
            const h = box.history
            if (h.length === 0 || h[h.length - 1] !== t) h.push(t)
        }
        histIndex = -1
        draft = ""
        edit.text = ""
        viNormal = false
        box.submitted(t)
    }
    function historyStep(d) {
        const h = box.history
        if (h.length === 0) return
        if (histIndex === -1) {
            if (d > 0) return
            draft = edit.text
            histIndex = h.length - 1
        } else {
            histIndex += d
            if (histIndex >= h.length) { histIndex = -1; setText(draft); return }
            histIndex = Math.max(0, histIndex)
        }
        setText(h[histIndex])
    }

    // Positions within the text.
    function lineStart(p) { return edit.text.lastIndexOf("\n", p - 1) + 1 }
    function lineEnd(p) { const i = edit.text.indexOf("\n", p); return i < 0 ? edit.text.length : i }
    function isWord(c) { return /[A-Za-z0-9_]/.test(c) }
    function wordRight(p) {
        const t = edit.text
        while (p < t.length && !isWord(t[p])) p++
        while (p < t.length && isWord(t[p])) p++
        return p
    }
    function wordLeft(p) {
        const t = edit.text
        while (p > 0 && !isWord(t[p - 1])) p--
        while (p > 0 && isWord(t[p - 1])) p--
        return p
    }
    function wordEnd(p) {
        const t = edit.text
        p++
        while (p < t.length && !isWord(t[p])) p++
        while (p + 1 < t.length && isWord(t[p + 1])) p++
        return Math.min(p, t.length)
    }
    function kill(a, b, append) {
        if (b < a) { const x = a; a = b; b = x }
        const cut = edit.text.substring(a, b)
        killRing = append ? killRing + cut : cut
        edit.remove(a, b)
        edit.cursorPosition = a
    }

    // ---- VS Code-style editing: indentation, pairs -------------------------
    readonly property string indentUnit: lang === "python" ? "    " : "  "
    readonly property var pairs: ({ "(": ")", "[": "]", "{": "}", "\"": "\"", "'": "'", "`": "`" })
    function indentOf(p) {
        const a = lineStart(p)
        let i = a
        while (i < edit.text.length && (edit.text[i] === " " || edit.text[i] === "\t")) i++
        return edit.text.substring(a, i)
    }
    // A new line keeping the indentation, one deeper after an opener
    // (do, then, {, (, :, ...); between a bracket pair the closer moves down.
    function newline() {
        const p = edit.cursorPosition
        const before = edit.text.substring(lineStart(p), p).replace(/\s+$/, "")
        let ind = indentOf(p)
        const opens = lang === "python" ? /[:([{]$/.test(before)
                    : /(\b(do|then|else|in)|[{(\[]|\|\||&&|\||\\)$/.test(before)
        const deeper = opens ? ind + indentUnit : ind
        const next = edit.text[p]
        if (opens && next && ")]}".indexOf(next) >= 0 && before.slice(-1) === Object.keys(pairs).find(k => pairs[k] === next)) {
            insert("\n" + deeper + "\n" + ind)
            edit.cursorPosition = p + 1 + deeper.length
            return
        }
        insert("\n" + deeper)
    }
    // Typing a closing word or bracket as the first thing on a line moves the
    // line back out a level.
    function maybeOutdent() {
        if (lang === "python") return
        const p = edit.cursorPosition, a = lineStart(p)
        const line = edit.text.substring(a, p)
        if (!/^\s+(done|fi|esac|else|elif|\}|\))$/.test(line)) return
        const ind = indentOf(p)
        if (ind.length < indentUnit.length) return
        edit.remove(a, a + indentUnit.length)
    }
    function shiftLines(dir) {
        // Indent (1) or outdent (-1) the current line or every selected one.
        const s = edit.selectionStart, e = edit.selectionEnd
        let a = lineStart(s)
        const lines = edit.text.substring(a, lineEnd(e)).split("\n")
        const out = lines.map(l => dir > 0 ? indentUnit + l
                                           : l.replace(new RegExp("^( {1," + indentUnit.length + "}|\t)"), ""))
        const block = out.join("\n")
        const pos = edit.cursorPosition, delta = out[0].length - lines[0].length
        edit.remove(a, lineEnd(e))
        edit.insert(a, block)
        if (s !== e) edit.select(a, a + block.length)
        else edit.cursorPosition = Math.max(a, pos + delta)
    }
    function smartKey(e) {
        if (e.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier)) return false
        const p = edit.cursorPosition, t = edit.text
        if (e.key === Qt.Key_Backtab) { shiftLines(-1); return true }
        if (e.key === Qt.Key_Tab) {
            // In the indentation (or over several lines): indent.
            const lead = /^\s*$/.test(t.substring(lineStart(p), p))
            if (edit.selectedText.indexOf("\u2029") >= 0 || edit.selectedText.indexOf("\n") >= 0) { shiftLines(1); return true }
            if (lead) { insert(indentUnit); return true }
            return false   // (completion takes Tab elsewhere)
        }
        if (e.key === Qt.Key_Backspace && p > 0 && !edit.selectedText) {
            // Between a pair: remove both. In indentation: a whole level.
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
        // Over a closer that's already there: step past it.
        if (")]}\"'`".indexOf(ch) >= 0 && t[p] === ch && !edit.selectedText) {
            const quote = "\"'`".indexOf(ch) >= 0
            if (!quote || t[p - 1] !== "\\") { edit.cursorPosition = p + 1; return true }
        }
        if (pairs[ch]) {
            const sel = edit.selectedText
            if (sel) {
                // Wrap the selection.
                const a = edit.selectionStart, b = edit.selectionEnd
                edit.remove(a, b)
                edit.insert(a, ch + sel + pairs[ch])
                edit.select(a + 1, a + 1 + sel.length)
                return true
            }
            const quote = "\"'`".indexOf(ch) >= 0
            const prev = t[p - 1] || " ", next = t[p] || " "
            // Quotes pair only between word boundaries (not it's, not $'x).
            if (quote && (/[A-Za-z0-9_\\$]/.test(prev) || /[A-Za-z0-9_]/.test(next))) return false
            if (!quote && /[A-Za-z0-9_]/.test(next)) return false
            insert(ch + pairs[ch])
            edit.cursorPosition = p + 1
            return true
        }
        return false
    }

    // Keys shared by every mode.
    function commonKey(e) {
        const ctrl = e.modifiers & Qt.ControlModifier
        const shift = e.modifiers & Qt.ShiftModifier
        if ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter)) {
            if (shift || (e.modifiers & Qt.AltModifier)) { newline(); return true }
            submit()
            return true
        }
        if (ctrl && !shift && e.key === Qt.Key_C && !edit.selectedText) {
            // Clear the line; on an empty line, interrupt like ^C at a prompt.
            if (edit.text !== "") { edit.text = ""; histIndex = -1 } else if (box.view) box.view.sendText("\u0003")
            return true
        }
        if (ctrl && !shift && e.key === Qt.Key_D && edit.text === "") {
            if (box.view) box.view.sendText("\u0004")
            return true
        }
        if (shift && box.view) {
            // Scrolling the terminal, as when it has the keys.
            const page = Math.max(1, box.view.rows - 1)
            switch (e.key) {
            case Qt.Key_PageUp: box.view.scrollLines(-page); return true
            case Qt.Key_PageDown: box.view.scrollLines(page); return true
            }
            if (ctrl && e.key === Qt.Key_Home) { box.view.scrollToRow(0); return true }
            if (ctrl && e.key === Qt.Key_End) { box.view.scrollToBottom(); return true }
        }
        if (ctrl && !shift && e.key === Qt.Key_L) {
            if (box.view) box.view.sendText("\u000c")
            return true
        }
        const firstLine = edit.text.lastIndexOf("\n", edit.cursorPosition - 1) < 0
        const lastLine = edit.text.indexOf("\n", edit.cursorPosition) < 0
        if (e.key === Qt.Key_Up && !shift && firstLine && !(keys === "vi" && viNormal)) { historyStep(-1); return true }
        if (e.key === Qt.Key_Down && !shift && lastLine && !(keys === "vi" && viNormal)) { historyStep(1); return true }
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
            case Qt.Key_K: kill(p, lineEnd(p) === p ? Math.min(edit.length, p + 1) : lineEnd(p)); return true
            case Qt.Key_U: kill(lineStart(p), p); return true
            case Qt.Key_W: kill(wordLeft(p), p); return true
            case Qt.Key_Y: insert(killRing); return true
            case Qt.Key_H: if (p > 0) edit.remove(p - 1, p); return true
            case Qt.Key_P: historyStep(-1); return true
            case Qt.Key_N: historyStep(1); return true
            case Qt.Key_Underscore: case Qt.Key_Slash: edit.undo(); return true
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
            }
        }
        return false
    }

    function vscodeKey(e) {
        const ctrl = e.modifiers & Qt.ControlModifier
        const shift = e.modifiers & Qt.ShiftModifier
        const alt = e.modifiers & Qt.AltModifier
        const p = edit.cursorPosition
        if (ctrl && shift && e.key === Qt.Key_K) {
            const a = lineStart(p), b = lineEnd(p)
            kill(a, Math.min(edit.length, b + 1)); return true
        }
        if (alt && (e.key === Qt.Key_Up || e.key === Qt.Key_Down)) {
            // Move the line up or down.
            const t = edit.text, a = lineStart(p), b = lineEnd(p)
            const line = t.substring(a, b), col = p - a
            if (e.key === Qt.Key_Up && a > 0) {
                const pa = lineStart(a - 1)
                setText(t.substring(0, pa) + line + "\n" + t.substring(pa, a - 1) + t.substring(b))
                edit.cursorPosition = pa + col
            } else if (e.key === Qt.Key_Down && b < t.length) {
                const nb = lineEnd(b + 1)
                const next = t.substring(b + 1, nb)
                setText(t.substring(0, a) + next + "\n" + line + t.substring(nb))
                edit.cursorPosition = a + next.length + 1 + col
            }
            return true
        }
        if (ctrl && e.key === Qt.Key_Slash) {
            // Toggle a # comment on the line.
            const a = lineStart(p)
            if (edit.text.substring(a, a + 2) === "# ") edit.remove(a, a + 2)
            else edit.insert(a, "# ")
            return true
        }
        return false   // the rest (Ctrl+Z, Ctrl+A, Ctrl+arrows, ...) is TextArea's own
    }

    function viKey(e) {
        if (!viNormal) {
            if (e.key === Qt.Key_Escape) { viNormal = true; edit.cursorPosition = Math.max(lineStart(edit.cursorPosition), edit.cursorPosition - 1); return true }
            return emacsKey(e)   // insert mode keeps the readline keys
        }
        // Normal mode.
        const ch = e.text
        const p = edit.cursorPosition
        const t = edit.text
        if (e.key === Qt.Key_Escape) { viPending = ""; return true }
        if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_R) { edit.redo(); return true }
        if (viPending === "r") { if (ch && p < t.length) { edit.remove(p, p + 1); edit.insert(p, ch); edit.cursorPosition = p } viPending = ""; return true }
        if (viPending === "g") { viPending = ""; if (ch === "g") edit.cursorPosition = 0; return true }
        // Motions: where `ch` moves from p (or -1 when it isn't one).
        function motion(c) {
            switch (c) {
            case "h": return Math.max(lineStart(p), p - 1)
            case "l": return Math.min(lineEnd(p), p + 1)
            case "w": return wordRight(p)
            case "b": return wordLeft(p)
            case "e": return wordEnd(p)
            case "0": return lineStart(p)
            case "^": { let i = lineStart(p); while (i < t.length && t[i] === " ") i++; return i }
            case "$": return lineEnd(p)
            }
            return -1
        }
        if (viPending === "d" || viPending === "c" || viPending === "y") {
            const op = viPending
            viPending = ""
            let a, b
            if (ch === op) { a = lineStart(p); b = Math.min(t.length, lineEnd(p) + (op === "c" ? 0 : 1)) }
            else {
                const m = motion(ch === "w" && op === "c" ? "e" : ch)
                if (m < 0) return true
                a = Math.min(p, m); b = Math.max(p, m) + (ch === "e" || (ch === "w" && op === "c") ? 1 : 0)
            }
            if (op === "y") { killRing = t.substring(a, b); return true }
            kill(a, b)
            if (op === "c") viNormal = false
            return true
        }
        const m = motion(ch)
        if (m >= 0) { edit.cursorPosition = m; return true }
        switch (ch) {
        case "i": viNormal = false; return true
        case "a": edit.cursorPosition = Math.min(lineEnd(p), p + 1); viNormal = false; return true
        case "I": edit.cursorPosition = motion("^"); viNormal = false; return true
        case "A": edit.cursorPosition = lineEnd(p); viNormal = false; return true
        case "o": edit.cursorPosition = lineEnd(p); insert("\n"); viNormal = false; return true
        case "O": edit.cursorPosition = lineStart(p); insert("\n"); edit.cursorPosition = lineStart(p); viNormal = false; return true
        case "x": if (p < lineEnd(p)) kill(p, p + 1); return true
        case "X": if (p > lineStart(p)) kill(p - 1, p); return true
        case "D": kill(p, lineEnd(p)); return true
        case "C": kill(p, lineEnd(p)); viNormal = false; return true
        case "s": if (p < lineEnd(p)) kill(p, p + 1); viNormal = false; return true
        case "p": edit.insert(Math.min(t.length, p + 1), killRing); return true
        case "P": insert(killRing); return true
        case "u": edit.undo(); return true
        case "~": if (p < t.length) { const c = t[p]; edit.remove(p, p + 1); edit.insert(p, c === c.toUpperCase() ? c.toLowerCase() : c.toUpperCase()); edit.cursorPosition = p + 1 } return true
        case "G": edit.cursorPosition = t.length; return true
        case "j": historyStep(1); return true
        case "k": historyStep(-1); return true
        case "d": case "c": case "y": case "g": case "r": viPending = ch; return true
        }
        // Anything else in normal mode is swallowed (no stray typing).
        return ch !== "" && !(e.modifiers & Qt.ControlModifier)
    }

    // ---- view -------------------------------------------------------------
    Flickable {
        id: flick
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: modeTag.width + 18
        anchors.topMargin: 7
        anchors.bottomMargin: 7
        contentWidth: width
        contentHeight: edit.contentHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        function ensureVisible(r) {
            if (contentY >= r.y) contentY = r.y
            else if (contentY + height <= r.y + r.height) contentY = r.y + r.height - height
        }

        TextEdit {
            id: edit
            width: flick.width
            wrapMode: TextEdit.Wrap
            textFormat: TextEdit.PlainText
            renderType: Text.NativeRendering
            selectByMouse: true
            persistentSelection: false
            font.family: box.view ? box.view.fontFamily : "monospace"
            font.pointSize: box.view && box.view.fontPointSize > 0 ? box.view.fontPointSize : 11
            color: Theme.fg
            selectionColor: Theme.accent
            selectedTextColor: Theme.bg
            cursorDelegate: Rectangle {
                width: box.keys === "vi" && box.viNormal ? Math.max(2, fm.averageCharacterWidth) : 2
                color: Theme.accent
                opacity: box.keys === "vi" && box.viNormal ? 0.6 : 1
                visible: edit.activeFocus
            }
            onCursorRectangleChanged: flick.ensureVisible(cursorRectangle)
            // (Re-coloring also signals a change: react to real edits only.)
            property string seen: ""
            onTextChanged: {
                if (text === seen) return
                seen = text
                idle.restart()
                box.analyze(false)
            }
            Keys.priority: Keys.BeforeItem
            Keys.onPressed: (e) => {
                let done = box.commonKey(e)
                if (!done && !(box.keys === "vi" && box.viNormal)) done = box.smartKey(e)
                if (!done) {
                    if (box.keys === "vi") done = box.viKey(e)
                    else if (box.keys === "vscode") done = box.vscodeKey(e)
                    else done = box.emacsKey(e)
                }
                if (!done && e.key === Qt.Key_Escape) { box.handBack(); done = true }
                if (!done && e.key === Qt.Key_Tab) { box.insert(box.indentUnit); done = true }
                if (!done && e.text.length === 1 && !(box.keys === "vi" && box.viNormal)) Qt.callLater(box.maybeOutdent)
                e.accepted = done
            }
        }
    }

    Text {
        // Empty: a hint of what this is.
        anchors.left: flick.left
        anchors.verticalCenter: flick.verticalCenter
        anchors.verticalCenterOffset: 0.001
        visible: edit.text === "" && !(box.keys === "vi" && box.viNormal)
        text: "Type a command — Enter runs it, Shift+Enter adds a line"
        color: Theme.textDim
        opacity: 0.55
        font: edit.font
        renderType: Text.NativeRendering
    }

    Text {
        id: modeTag
        anchors.right: parent.right
        anchors.rightMargin: 10
        anchors.top: parent.top
        anchors.topMargin: 8
        text: box.problem !== "" ? "\u26a0 " + box.problem
              : box.keys === "vi" ? (box.viNormal ? "NORMAL" : "INSERT") : box.lang
        color: box.problem !== "" ? box.ansi(9, "#e05a5a") : Theme.textDim
        opacity: 0.7
        font.pixelSize: 10
        font.weight: Font.DemiBold
        font.letterSpacing: 0.5
    }

    // ---- syntax -----------------------------------------------------------
    // Colors from the theme's own palette; errors show once typing pauses
    // (half-typed text is always "wrong").
    property string problem: ""        // the first syntax error, shown in the corner
    Timer { id: idle; interval: 650; onTriggered: box.analyze(true) }
    function analyze(withErrors) {
        if (!controller) return
        const r = JSON.parse(controller.syntax(box.lang, edit.text))
        if (!withErrors) r.errors = []
        problem = r.errors.length > 0 ? r.errors[0][2] : ""
        hl.analysis = JSON.stringify(r)
    }
    SyntaxHighlighter {
        id: hl
        document: edit.textDocument
        errorColor: box.ansi(9, "#e05a5a")
        colors: ({
            keyword: box.ansi(5, "#c678dd"),
            string: box.ansi(2, "#98c379"),
            comment: Theme.textDim,
            number: box.ansi(3, "#d19a66"),
            constant: box.ansi(3, "#d19a66"),
            function: box.ansi(4, "#61afef"),
            operator: box.ansi(6, "#56b6c2"),
            property: box.ansi(6, "#56b6c2"),
            variable: box.ansi(1, "#e06c75"),
            embedded: box.ansi(13, "#c678dd"),
            type: box.ansi(3, "#e5c07b"),
            constructor: box.ansi(3, "#e5c07b"),
            punctuation: Theme.textDim,
            escape: box.ansi(6, "#56b6c2"),
        })
    }
    onLangChanged: analyze(false)
}
