import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// The input editor: a small multi-line editor docked under the terminal while
// the shell (or a REPL) waits for input. Enter runs the command (it goes to
// the shell as if typed, and scrolls up with its output); a right Shift tap (or Shift+Enter) starts
// a new line. Highlighting and syntax errors come from tree-sitter; the keys
// follow Emacs (readline), vi, or VS Code.
Rectangle {
    id: box
    property var view: null            // the TerminalView it types into
    property var controller: null
    // The input language: picked from the corner (langOverride), or what the
    // program at the prompt implies (autoLang).
    property string autoLang: "bash"
    property string langOverride: ""
    // (Pasted code in another language is recognised when it's clear.)
    property string contentLang: ""
    // (A line that isn't a shell command: the AI says what it looks like, with
    // the languages used lately ranked up. It runs with that language's
    // interpreter; see TerminalPane's onSubmitted.)
    property string aiLang: ""
    readonly property string lang: langOverride !== "" ? langOverride : contentLang !== "" ? contentLang : aiLang !== "" ? aiLang : autoLang
    property string runLang: ""        // the language of what was just run
    // Another shell picked to run the input ("": the pane's own), and the one
    // what was just run went to.
    property string runShell: ""
    property string runShellUsed: ""
    // (By the names their programs go by, as the daemon reports the shell.)
    readonly property var shellNames: ({ zsh: "zsh", bash: "bash", fish: "fish", nu: "nushell", xonsh: "xonsh", pwsh: "PowerShell", elvish: "elvish", ksh: "ksh", dash: "dash", tcsh: "tcsh", sh: "sh" })
    // (The grammar each one is highlighted with.)
    function shellGrammar(s) { return ({ fish: "fish", nu: "nu" })[s] || "bash" }
    // (The last language picked is kept, for every pane and the next start:
    // the state folder's input-language.json.)
    property string savedPick: ""
    property bool pickRestored: false
    function restorePick() {
        if (pickRestored || !controller) return
        pickRestored = true
        // (Read in the background; a pick made meanwhile wins.)
        Async.call(controller, k => controller.readTextFile(k, controller.stateFolder() + "/input-language.json"), r => {
            if (!box || box.savedPick !== "") return
            let id = ""
            try { id = JSON.parse(JSON.parse(r).text || "{}").id || "" } catch (e) {}
            box.savedPick = id
            if (id !== "") box.pickLanguage(id, false)
        })
        loadLangRecent()
    }
    onControllerChanged: { restorePick(); refreshLsp() }
    Component.onCompleted: { restorePick(); refreshLsp() }
    // (A shell picked before the pane's own shell is known: decided again once it is.)
    onShellNameChanged: if (savedPick.startsWith("shell:") && langOverride === "bash") pickLanguage(savedPick, false)
    function pickLanguage(id, remember) {
        if (remember !== false && controller) {
            savedPick = id
            Async.call(controller, k => controller.writeTextFile(k, controller.stateFolder() + "/input-language.json", JSON.stringify({ id: id })), null)
        }
        if (id.startsWith("shell:")) {
            const sh = id.substring(6)
            // (Chosen, not guessed. Another shell than the pane's: the input
            // runs in its kernel, a card each time, the folder and variables
            // shared with the pane's shell.)
            runShell = sh === (shellName || "bash") ? "" : sh
            langOverride = "bash"
        } else {
            runShell = ""
            langOverride = id
        }
    }
    property var langRecent: []
    // (Read in the background; languages used meanwhile stay in front.)
    function loadLangRecent() {
        Async.call(controller, k => controller.readTextFile(k, controller.stateFolder() + "/languages.json"), r => {
            if (!box) return
            let l = []
            try { l = JSON.parse(JSON.parse(r).text || "[]") } catch (e) {}
            if (!Array.isArray(l)) return
            box.langRecent = box.langRecent.concat(l.filter(x => box.langRecent.indexOf(x) < 0)).slice(0, 8)
        })
    }
    function useLanguage(l) {
        if (!controller || l === "" || l === "bash") return
        const r = [l].concat(langRecent.filter(x => x !== l)).slice(0, 8)
        langRecent = r
        Async.call(controller, k => controller.writeTextFile(k, controller.stateFolder() + "/languages.json", JSON.stringify(r)), null)
    }
    property string langAsked: ""
    Timer {
        id: langTimer
        interval: 450
        onTriggered: box.detectLang()
    }
    function detectLang() {
        {
            const t = edit.text
            if (box.autoLang !== "bash" || box.langOverride !== "" || box.contentLang !== "" || t.trim() === "" || t.startsWith("#")) { box.aiLang = ""; return }
            if (!box.controller.shellDoubt(t, box.view ? box.view.aliases : "{}")) { box.aiLang = ""; box.langAsked = ""; return }
            if (t === box.langAsked) return
            // (The plainest marks at once; the AI's answer, when it comes, decides.)
            box.aiLang = box.controller.detectLanguage(t)
            // (A sure mark, println!( or console.log(, isn't second-guessed.)
            if (box.aiLang !== "" || !box.inlineReady) return
            box.langAsked = t
            box.controller.aiRun(box.aiKey + "-lang", "language", JSON.stringify([{ role: "user", content: t }]), box.aiContext())
        }
    }
    readonly property var languages: { try { return JSON.parse(controller.syntaxLanguages()) } catch (e) { return [["bash", "Shell"]] } }
    property string keys: "emacs"      // "emacs", "vi", "vscode"
    property var history: []           // oldest first (shared by the window)
    property var themeData: ({})       // the terminal's theme (ansi colors)
    property string cwd: ""            // the shell's folder (for path completion)
    property bool showPrompt: true     // Tessera's prompt bar above the editor
    property int shellPid: 0
    property string sshHost: ""         // the pane's host ("" here): completions are worked out there
    property string promptEngine: "auto"
    property var promptSegments: ["dir", "git", "venv", "nix", "status", "duration", "|", "kube", "aws", "gcloud", "host", "time"]
    signal submitted(string text)
    signal keysPicked(string keys)     // the corner switch: "emacs", "vi", "vscode"
    signal openRich()                  // show the last output's rich view
    property string viewMode: "terminal"
    signal viewModePicked(string mode) // "terminal" or "notebook"
    signal handBack()                  // Shift+Tab: type into the terminal instead

    readonly property alias text: edit.text
    readonly property bool editing: edit.activeFocus
    readonly property real lineHeight: Math.ceil(fm.height)
    readonly property int lines: Math.max(1, edit.lineCount)
    // (The editor's margins, and the prompt bar with the gap under it.)
    // (The text's real laid-out height, which some fonts make taller than
    // their metrics say, up to ten lines; then it scrolls.)
    implicitHeight: Math.ceil(Math.min(Math.max(edit.contentHeight, lineHeight), 10 * lineHeight)) + 16
                    + (promptBar.visible ? promptBar.height + 4 + 4 : 0)

    color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.045 : 0.035)
    Rectangle { width: parent.width; height: 1; color: Theme.divider }

    FontMetrics { id: fm; font: edit.font }

    readonly property color cursorColor: themeData && themeData.cursor ? themeData.cursor : Theme.fg
    function ansi(i, fallback) {
        const a = box.themeData && box.themeData.ansi
        return a && a[i] ? a[i] : fallback
    }

    // ---- editing ----------------------------------------------------------
    property int histIndex: -1         // -1: the draft, else into history
    property string draft: ""
    // vi's ":" commands on what's being written (ExCommands.qml), sharing the
    // yanked text. :w / :x / :wq run it, :q clears it.
    readonly property ExCommands ex: ExCommands {
        id: cex
        edit: edit
        indentUnit: box.indentUnit
        keyHandler: (e) => box.keys === "vi" ? box.viKey(e) : false
        viKeys: box
        onWrite: (p, force) => { if (p !== "") exLine.show("Not here: :w {file} (this runs the command)", true); else Qt.callLater(box.submit) }
        onQuit: (force) => { edit.text = ""; box.viNormal = false }
        onEditFile: (p, force) => exLine.show("Not here: :e (this is the command being written)", true)
        onSetOption: (name, value) => {}
        helpFor: ({ write: "run the command", update: "run the command", wq: "run the command", xit: "run the command", exit: "run the command",
                    wall: "run the command", wqall: "run the command", xall: "run the command",
                    quit: "clear the editor", qall: "clear the editor", close: "clear the editor",
                    saveas: null, edit: null, read: null, file: null, pwd: null })
    }
    property alias killRing: cex.killRing
    property bool viNormal: false
    property string viPending: ""      // an operator waiting for its motion (d, c, y, g, r)

    function focusEditor() { edit.forceActiveFocus() }
    function insert(t) { edit.insert(edit.cursorPosition, t) }
    function setText(t) { edit.text = t; edit.cursorPosition = t.length }

    function submit() {
        const t = edit.text
        // "# what I want": the AI writes the command, for you to check first.
        if (aiNL && /^#\s*\S/.test(t) && t.indexOf("\n") < 0 && chatReady) { askCommand(t.replace(/^#\s*/, "")); return }
        ghost = ""
        if (t.trim() !== "") {
            const h = box.history
            const old = h.indexOf(t)
            if (old >= 0) h.splice(old, 1)
            h.push(t)
            if (controller) controller.historyAdd(t, box.cwd)
        }
        histIndex = -1
        draft = ""
        // (Enter before the pause that detects the language: detect it now,
        // the quick local way, so code doesn't go to the shell.)
        if (langTimer.running) { langTimer.stop(); detectLang() }
        runLang = lang
        runShellUsed = runShell
        useLanguage(runLang)
        edit.text = ""
        viNormal = false
        box.submitted(t)
    }
    // A command from Tessera itself (a prompt widget's choice, a file or
    // variable clicked in the notebook): run as if typed, in the shell.
    function runCommand(c) { runLang = "bash"; runShellUsed = ""; box.submitted(c) }
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
        // (A line from history: the completion list stays shut, or it would
        // take the next Up/Down.)
        fromHistory = true
        closeComp()
    }
    property bool fromHistory: false

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
    // tessh: the lines that take an indented block (def, if, for, a block
    // after x = if …), and where an else, elif, except or finally goes: back
    // to its if or try.
    readonly property var tesshBlocks: ["def", "class", "if", "elif", "else", "while", "for", "try", "except", "except*", "finally", "with", "match", "case", "defmacro", "quasi", "do", "async", "fn", "lambda"]
    readonly property var tesshOpeners: ({ "if": ["elif", "else"], "elif": ["elif", "else"], "for": ["else"], "while": ["else"], "try": ["except", "finally"], "except": ["except", "else", "finally"], "else": ["finally"] })
    function tesshHead(line) {
        const w = line.trim().split(/\s+/)
        let h = w[0] || ""
        if (w.length > 2 && /^(=|\+=|-=)$/.test(w[1])) h = w[2]
        if (h === "async" && w.length > 1) h = w[1]
        return h
    }
    function tesshOpens(line) {
        const t = line.trim(), w = t.split(/\s+/), h = tesshHead(line)
        if (t === "" || t[0] === "#") return false
        if (/[([{]$/.test(t)) return true
        if ((h === "def" || h === "class") && w.indexOf("=") > 0) return false
        if ((h === "fn" || h === "lambda") && w.length > 2) return false
        if ((h === "quasi" || h === "do") && w.length > 1) return false
        if (h === "if" && w.indexOf("then") > 0) return false
        return tesshBlocks.indexOf(h) >= 0
    }
    // The indentation an else (elif, except, finally) at `ind` belongs at,
    // from the lines above (or ind when there's no if or try for it).
    function tesshAlign(above, word, ind) {
        const lines = above.split("\n")
        for (let i = lines.length - 1; i >= 0; i--) {
            const l = lines[i]
            if (l.trim() === "") continue
            const li = l.length - l.replace(/^ +/, "").length
            if (li > ind) continue
            const ok = tesshOpeners[tesshHead(l)]
            if (ok && ok.indexOf(word) >= 0) return li
            if (li < ind && li === 0) break
        }
        return ind
    }
    // A new line keeping the indentation, one deeper after an opener
    // (do, then, {, (, :, ...); between a bracket pair the closer moves down.
    function newline() {
        let p = edit.cursorPosition
        if (lang === "tessh") {
            tesshOutdent()
            p = edit.cursorPosition
        }
        const before = edit.text.substring(lineStart(p), p).replace(/\s+$/, "")
        let ind = indentOf(p)
        const opens = lang === "python" ? /[:([{]$/.test(before)
                    : lang === "tessh" ? tesshOpens(before)
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
    // tessh: an else (elif, except, finally) first on its line goes back to
    // its if or try.
    function tesshOutdent() {
        const p = edit.cursorPosition, a = lineStart(p)
        const m = /^( +)(else|elif|except\*?|finally)\b/.exec(edit.text.substring(a, lineEnd(p)))
        if (!m) return
        const want = tesshAlign(edit.text.substring(0, Math.max(0, a - 1)), m[2], m[1].length)
        if (want < m[1].length) edit.remove(a, a + m[1].length - want)
    }
    // Typing a closing word or bracket as the first thing on a line moves the
    // line back out a level.
    function maybeOutdent() {
        if (lang === "python") return
        if (lang === "tessh") {
            const p = edit.cursorPosition
            if (/^ +(else|elif|except|finally)$/.test(edit.text.substring(lineStart(p), p))) tesshOutdent()
            return
        }
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

    // ---- completion --------------------------------------------------------
    // The dropdown: commands, subcommands, options with their help, paths,
    // branches, hosts. Opens on Tab, and by itself while typing a word; Tab
    // takes the choice (Enter too once you've moved through the list).
    property var comp: ({ start: 0, end: 0, items: [], pending: false })
    property bool compOpen: false
    property bool typing: false        // keys coming: the list waits for a pause
    property bool compChosen: false    // Up/Down used: Enter takes the item
    property int compIndex: 0
    property bool historyMode: false   // the dropdown lists history (Ctrl+R, the clock)
    property string historyQuery: ""
    function openHistory() {
        historyMode = true; compOpen = true; compChosen = true; compIndex = 0
        historyQuery = edit.text.trim()
        historySearch.text = historyQuery
        requestComplete()
        historySearch.forceActiveFocus()
        historySearch.selectAll()
    }
    // (Answered in the background: a reply is dropped when a newer ask went
    // out, or the text or cursor moved since.)
    property int compSeq: 0
    function requestComplete() {
        if (!controller) return
        const seq = ++compSeq, hist = historyMode, query = historyQuery
        const text = edit.text, cursor = edit.cursorPosition
        Async.call(controller, k => hist ? controller.historySearch(k, query)
                                         : controller.complete(k, text, cursor, box.cwd, box.view ? box.view.aliases : "{}", box.shellPid, box.sshHost), json => {
            if (!box || seq !== box.compSeq || !box.compOpen || box.historyMode !== hist) return
            if (hist ? box.historyQuery !== query : (edit.text !== text || edit.cursorPosition !== cursor)) return
            let r
            try { r = JSON.parse(json) } catch (e) { return }
            box.comp = r
            if (box.compIndex >= r.items.length) box.compIndex = 0
            if (r.items.length === 0 && !r.pending) box.closeComp()
            if (r.pending) compRetry.restart()
            // (The language server's items are added to these when they come.)
            if (!hist && box.lspServer !== "")
                box.controller.lspAsk(box.aiKey + "-comp", box.lang, box.cwd, text, "complete", cursor, box.sshHost)
        })
    }
    function openComp() { compOpen = true; compChosen = false; compIndex = 0; typing = false; requestComplete() }
    function closeComp() { compOpen = false; compChosen = false; historyMode = false }
    function acceptComp(i) {
        const it = comp.items[i === undefined ? compIndex : i]
        if (!it) return
        if (historyMode) { setText(it.insert); closeComp(); edit.forceActiveFocus(); return }
        edit.remove(comp.start, comp.end)
        let ins = it.insert
        // A finished word gets its space; a folder or "--opt=" keeps going.
        const more = it.kind === "dir" || ins.endsWith("=") || ins.endsWith("/")
        if (!more && edit.text[comp.start] !== " ") ins += " "
        edit.insert(comp.start, ins)
        edit.cursorPosition = comp.start + ins.length
        if (more) { compIndex = 0; requestComplete() } else closeComp()
    }
    Timer { id: compRetry; interval: 250; onTriggered: if (box.compOpen) box.requestComplete() }
    Timer {
        id: compAuto
        // (Once typing pauses: the list is hidden while keys come.)
        interval: 300
        onTriggered: {
            box.typing = false
            // Open by itself on a word being typed (not after a space, not in
            // the middle of a word), and keep an open list up to date.
            const p = edit.cursorPosition, t = edit.text
            const prev = t[p - 1] || " ", next = t[p] || " "
            const inWord = !/\s/.test(prev) && /\s/.test(next)
            // (Right after "cd ", "ls ", "export ", "unset ": the folders or
            // variables at once, to click through.)
            const builtinArg = p === t.length && /^\s*(cd|pushd|ls|ll|la|export|unset)\s+$/.test(t)
            if (builtinArg && !(box.keys === "vi" && box.viNormal)) { box.compOpen = true; box.compIndex = 0; box.requestComplete(); return }
            if (box.compOpen && box.historyMode) return
            if (box.fromHistory) return
            else if (box.compOpen) { if (inWord || prev === "/") box.requestComplete(); else box.closeComp() }
            else if (inWord && !(box.keys === "vi" && box.viNormal)) { box.compOpen = true; box.compIndex = 0; box.requestComplete() }
        }
    }
    function compKey(e) {
        if (!compOpen || comp.items.length === 0) return false
        const n = comp.items.length
        switch (e.key) {
        case Qt.Key_Down: compIndex = (compIndex + 1) % n; compChosen = true; return true
        case Qt.Key_Up: compIndex = (compIndex - 1 + n) % n; compChosen = true; return true
        case Qt.Key_PageDown: compIndex = Math.min(n - 1, compIndex + 8); compChosen = true; return true
        case Qt.Key_PageUp: compIndex = Math.max(0, compIndex - 8); compChosen = true; return true
        case Qt.Key_Tab: acceptComp(); return true
        case Qt.Key_Escape: closeComp(); return true
        case Qt.Key_Return: case Qt.Key_Enter:
            if (compChosen && !(e.modifiers & Qt.ShiftModifier)) { acceptComp(); return true }
            closeComp(); return false
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
        // (PgUp/PgDn: history a whole entry at a time, wherever the cursor is.)
        if (e.key === Qt.Key_PageUp && !shift) { historyStep(-1); return true }
        if (e.key === Qt.Key_PageDown && !shift) { historyStep(1); return true }
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
        if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_R) { cex.redo(); return true }
        if (viPending === "r") { if (ch && p < t.length) { edit.remove(p, p + 1); edit.insert(p, ch); edit.cursorPosition = p } viPending = ""; return true }
        if (viPending === "g") { viPending = ""; if (ch === "g") edit.cursorPosition = 0; return true }
        // (: opens the command line, / and ? search, n and N go on.)
        if (ch === ":" && viPending === "") { exLine.open(":", edit.selectedText !== "" ? "'<,'>" : ""); return true }
        if ((ch === "/" || ch === "?") && viPending === "") { exLine.open(ch, ""); return true }
        if ((ch === "n" || ch === "N") && viPending === "") { const r = cex.searchNext(ch === "N"); if (r !== "") exLine.show(r, r.startsWith("E")); return true }
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
        case "u": cex.undo(); return true
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
    PromptBar {
        id: promptBar
        visible: box.showPrompt
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.topMargin: 4
        height: visible ? implicitHeight : 0
        view: box.view
        controller: box.controller
        cwd: box.cwd
        themeData: box.themeData
        segments: box.promptSegments
        shellPid: box.shellPid
        sshHost: box.sshHost
        engine: box.promptEngine
        fontFamily: edit.font.family
        fontSize: edit.font.pointSize
        // (A widget's choice runs as if typed: a card, history.)
        onRunRequested: (c) => { box.runCommand(c); edit.forceActiveFocus() }
        onFixRequested: box.askFix()
    }

    // The prompt character, as the shell shows it: green, red after a failure.
    Text {
        id: promptChar
        x: 10
        y: flick.y - flick.contentY + 0.001
        visible: y >= flick.y - 1
        text: "\u276F"
        font: edit.font
        color: box.view && box.view.lastStatus !== 0 ? box.ansi(1, "#e06c75") : box.ansi(2, "#98c379")
        renderType: Text.NativeRendering
    }

    Flickable {
        id: flick
        anchors.fill: parent
        anchors.leftMargin: 10 + promptChar.implicitWidth + fm.advanceWidth(" ")
        anchors.rightMargin: modeTag.width + 18
        anchors.topMargin: 7 + (promptBar.visible ? promptBar.height + 4 : 0)
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
            property bool keepsBacktab: true   // (its own Shift+Tab: to the terminal, or outdent)
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
            // A block cursor like the terminal's, the character under it in
            // the background color (a hollow box when the editor isn't typing).
            cursorDelegate: Rectangle {
                id: cur
                readonly property string under: {
                    const c = edit.text[edit.cursorPosition]
                    // (At the end, the suggestion's first letter is under it.)
                    if ((c === undefined || box.closers !== "") && box.ghost !== "" && edit.activeFocus) return box.ghost[0]
                    return c === undefined || c === "\n" || c === "\u2029" ? "" : c
                }
                width: Math.max(2, fm.advanceWidth(under !== "" ? under : " "))
                height: fm.height
                color: edit.activeFocus ? box.cursorColor : "transparent"
                border.width: edit.activeFocus ? 0 : 1
                border.color: box.cursorColor
                opacity: box.keys === "vi" && !box.viNormal && edit.activeFocus ? 0.85 : 1
                Text {
                    anchors.fill: parent
                    text: cur.under
                    font: edit.font
                    color: Theme.bg
                    renderType: Text.NativeRendering
                    visible: edit.activeFocus
                }
            }
            onCursorRectangleChanged: flick.ensureVisible(cursorRectangle)
            // Pointing at a word (half a second) shows the language server's help for it.
            HoverHandler {
                id: wordHover
                onPointChanged: { box.hoverDoc = ""; hoverTimer.restart() }
                onHoveredChanged: if (!hovered) { hoverTimer.stop(); box.hoverDoc = "" }
            }
            Timer {
                id: hoverTimer
                interval: 550
                onTriggered: if (wordHover.hovered) box.hoverAtPosition(edit.positionAt(wordHover.point.position.x, wordHover.point.position.y))
            }
            // (Re-coloring also signals a change: react to real edits only.)
            property string seen: ""
            onTextChanged: {
                if (text === seen) return
                seen = text
                idle.restart()
                box.contentLang = box.controller && (text.indexOf("\n") >= 0 || /^\s*[{\[]|^#!/.test(text)) ? box.controller.detectLanguage(text) : ""
                if (text.trim() === "") { box.aiLang = ""; box.langAsked = "" }
                langTimer.restart()
                box.analyze(false)
                if (!box.historyMode) box.typing = true
                compAuto.restart()
                // A "# request": the command it'd be, from the AI, as the
                // suggestion (taken, it replaces the line).
                if (/^#\s*\S/.test(text) && text.indexOf("\n") < 0) {
                    box.nlCommand = ""
                    // (The words finished as they're typed; the command once they're done.)
                    const full = box.lastGhostBase + box.ghost
                    if (box.ghost !== "" && !box.ghost.startsWith("  \u2192") && full.startsWith(text) && full.length > text.length) box.ghost = full.substring(text.length)
                    else box.ghost = ""
                    box.lastGhostBase = text
                    if (box.ghost === "") reqTimer.restart()
                    nlTimer.restart()
                    if (text.trim() !== box.checkedText) box.aiNote = ""
                    return
                }
                box.nlCommand = ""
                // (Once the cursor has moved too: a pair typed puts it between.)
                Qt.callLater(box.refreshGhost)
                if (text.trim() !== box.checkedText) box.aiNote = ""
                checkTimer.restart()
            }
            Keys.priority: Keys.BeforeItem
            // A Shift tapped on its own: right Shift is a new line (Shift+Enter),
            // left Shift takes the AI's suggestion (or asks for one).
            property int shiftSide: 0          // 1 left, 2 right, 0 none / spoiled
            property double shiftAt: 0
            function shiftSideOf(e) {
                // X11/Wayland keycodes 50/62, Windows scan codes 42/54, macOS key codes 56/60.
                const n = e.nativeScanCode, v = e.nativeVirtualKey
                if (n === 50 || n === 42 || v === 56 || v === 0xffe1) return 1
                if (n === 62 || n === 54 || v === 60 || v === 0xffe2) return 2
                return 0
            }
            Keys.onReleased: (e) => {
                if (e.key === Qt.Key_Shift && shiftSide !== 0 && Date.now() - shiftAt < 450) {
                    if (shiftSide === 2) box.newline()
                    else if (!box.acceptGhost()) box.askGhostNow()
                    e.accepted = true
                }
                if (e.key === Qt.Key_Shift) shiftSide = 0
            }
            Keys.onPressed: (e) => {
                exLine.dismiss()
                if (e.key !== Qt.Key_Up && e.key !== Qt.Key_Down) box.fromHistory = false
                if (e.key === Qt.Key_Shift && !e.isAutoRepeat) { shiftSide = shiftSideOf(e); shiftAt = Date.now() }
                else shiftSide = 0
                let done = false
                // Shift+Tab: over to the terminal (several lines selected: outdent them).
                if (e.key === Qt.Key_Backtab && !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier))
                        && selectedText.indexOf("\u2029") < 0 && selectedText.indexOf("\n") < 0) {
                    box.closeComp(); box.ghost = ""; box.handBack(); e.accepted = true; return
                }
                const plain = !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.ShiftModifier))
                if (box.ghost !== "" && box.atSuggestEnd && plain
                        && (e.key === Qt.Key_Right || e.key === Qt.Key_End))
                    done = box.acceptGhost()
                if (!done && e.key === Qt.Key_Escape && box.ghost !== "") { box.ghost = ""; done = true }
                if (!done && e.key === Qt.Key_Escape && box.aiBusy !== "") { box.controller.aiCancel(box.aiKey + "-cmd"); box.aiBusy = ""; done = true }
                if (!done && e.key === Qt.Key_F1) { box.hoverAtPosition(cursorPosition); done = true }
                if (!done) done = box.compKey(e)
                if (!done && e.key === Qt.Key_Tab && !(e.modifiers & (Qt.ShiftModifier | Qt.ControlModifier))
                        && !/^\s*$/.test(text.substring(box.lineStart(cursorPosition), cursorPosition))) {
                    box.openComp(); done = true
                }
                if (!done && e.key === Qt.Key_Space && (e.modifiers & Qt.ControlModifier)) { box.openComp(); done = true }
                if (!done && e.key === Qt.Key_R && (e.modifiers & Qt.ControlModifier) && !(box.keys === "vi" && box.viNormal)) {
                    if (box.historyMode) box.closeComp(); else box.openHistory()
                    done = true
                }
                if (!done) done = box.commonKey(e)
                if (!done && !(box.keys === "vi" && box.viNormal)) done = box.smartKey(e)
                if (!done) {
                    if (box.keys === "vi") done = box.viKey(e)
                    else if (box.keys === "vscode") done = box.vscodeKey(e)
                    else done = box.emacsKey(e)
                }
                if (!done && e.key === Qt.Key_Tab) { box.insert(box.indentUnit); done = true }
                if (!done && e.text.length === 1 && !(box.keys === "vi" && box.viNormal)) Qt.callLater(box.maybeOutdent)
                e.accepted = done
            }
        }
    }

    // (Test hook: TESSERA_TEST_COMPOSE_EX="cmd;;cmd" runs those : commands on
    // the editor's text at 6 s, then opens the line with TESSERA_TEST_EX_OPEN.)
    Timer { interval: 6000; running: box.controller !== null && box.controller.testEnv("COMPOSE_EX") !== ""
            onTriggered: {
                for (const c of box.controller.testEnv("COMPOSE_EX").split(";;")) { const out = cex.run(c); console.log("compose ex test: :" + c + " -> " + JSON.stringify(out) + "  now " + JSON.stringify(edit.text)) }
                if (box.controller.testEnv("EX_OPEN") !== "") { edit.forceActiveFocus(); exLine.open(":", box.controller.testEnv("EX_OPEN")) }
            } }
    // vi's command line, just above the editor (its choices above it).
    ExLine {
        id: exLine
        z: 30
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.top
        ex: cex
        font: edit.font
        errorColor: box.themeData && box.themeData.ansi && box.themeData.ansi[1] ? box.themeData.ansi[1] : "#e06c75"
        onClosed: edit.forceActiveFocus()
    }

    // The AI's suggestion: the rest of the command, greyed, after the cursor.
    // (Before closers: its first letter in the cursor, the rest drawn over
    // the closers, which follow it when it doesn't close them itself.)
    readonly property real ghostSkip: closers !== "" && ghost !== "" ? fm.advanceWidth(ghost[0]) : 0
    Rectangle {
        parent: flick.contentItem
        readonly property rect at: edit.positionToRectangle(edit.cursorPosition)
        visible: ghostText.visible && box.closers !== ""
        z: 1
        x: at.x + box.ghostSkip
        y: at.y
        width: ghostRow.width
        height: at.height
        color: box.color
    }
    Row {
        id: ghostRow
        parent: flick.contentItem
        z: 2
        readonly property rect at: edit.positionToRectangle(box.closers !== "" ? edit.cursorPosition : edit.length)
        x: at.x + box.ghostSkip
        y: at.y
        Text {
            id: ghostText
            visible: box.ghost !== "" && edit.activeFocus && box.atSuggestEnd
            text: box.closers !== "" ? box.ghost.substring(1) : box.ghost
            font: edit.font
            color: Theme.mix(Theme.text, Theme.bg, 0.45)
            renderType: Text.NativeRendering
        }
        Text {
            visible: ghostText.visible && box.closers !== "" && !box.ghostTakesClosers()
            text: box.closers
            font: edit.font
            color: Theme.text
            renderType: Text.NativeRendering
        }
    }

    Text {
        // Empty: a hint of what this is.
        anchors.left: flick.left
        anchors.verticalCenter: flick.verticalCenter
        anchors.verticalCenterOffset: 0.001
        visible: edit.text === "" && !(box.keys === "vi" && box.viNormal)
        width: flick.width
        elide: Text.ElideRight
        text: box.aiNL && box.chatReady ? "Type a command, or # and a request for the AI — hover for keys"
                                         : "Type a command — hover for keys"
        HoverHandler { id: hintHover }
        ToolTip.visible: hintHover.hovered
        ToolTip.delay: 300
        ToolTip.text: "Enter runs the command\n"
                      + "Tap right Shift (or Shift+Enter) for a new line\n"
                      + "Tap left Shift to take the grey suggestion (from history or the AI)\n"
                      + "Tab completes; Ctrl+R searches history\nShift+Tab switches between this editor and the terminal"
                      + (box.aiNL && box.chatReady ? "\n# then what you want, in words: the AI writes the command for you to check and run" : "")
        color: Theme.textDim
        opacity: 0.55
        font: edit.font
        renderType: Text.NativeRendering
    }

    // Working: a short bar sweeping along the editor's top edge.
    Item {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        height: 2
        clip: true
        visible: spinner.on
        Rectangle {
            id: sweep
            width: parent.width * 0.3
            height: parent.height
            radius: 1
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: "transparent" }
                GradientStop { position: 0.5; color: Theme.accent }
                GradientStop { position: 1; color: "transparent" }
            }
            NumberAnimation on x { from: -sweep.width; to: sweep.parent.width; duration: 1100; loops: Animation.Infinite; running: spinner.on; easing.type: Easing.InOutQuad }
        }
    }

    // The corner: a problem, or the language / vi mode; the history button
    // and the key switch (Emacs, vi, VS Code).
    Row {
        id: modeTag
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.top: flick.top
        anchors.topMargin: -2
        spacing: 6
        // Working: a turning arc.
        Canvas {
            id: spinner
            readonly property bool on: box.aiBusy !== "" || box.fetchingLang !== ""
            visible: on
            anchors.verticalCenter: parent.verticalCenter
            width: 12; height: 12
            onPaint: {
                const g = getContext("2d")
                g.reset()
                g.lineWidth = 1.8
                g.lineCap = "round"
                g.strokeStyle = Theme.accent
                g.beginPath()
                g.arc(6, 6, 4.6, 0, Math.PI * 1.4)
                g.stroke()
            }
            RotationAnimator on rotation { from: 0; to: 360; duration: 800; loops: Animation.Infinite; running: spinner.on }
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: box.problem !== "" ? "\u26a0 " + box.problem
                  : box.fetchingLang !== "" ? "getting " + box.fetchingLang + "\u2026"
                  : box.aiBusy !== "" ? (box.aiBusy === "fix" ? "fixing the command\u2026" : "writing the command\u2026") + "  esc cancels"
                  : box.aiNote !== "" ? "\u2726 " + box.aiNote
                  : box.lspNote !== "" ? "\u26a0 " + box.lspNote
                  : ""
            color: box.problem !== "" ? box.ansi(9, "#e05a5a") : spinner.on ? Theme.accent : (box.aiNote !== "" || box.lspNote !== "") ? box.ansi(3, "#e5c07b") : Theme.textDim
            width: Math.min(implicitWidth, box.width * 0.45)
            elide: Text.ElideRight
            HoverHandler { id: noteHover }
            ToolTip.visible: noteHover.hovered && (box.aiNote !== "" || box.lspNote !== "" || box.problem !== "")
            ToolTip.text: box.problem !== "" ? box.problem : box.aiNote !== "" ? box.aiNote : box.lspNote
            opacity: 0.75
            font.pixelSize: 10
            font.weight: Font.DemiBold
            font.letterSpacing: 0.5
        }
        Chip {
            base: box.color
            visible: box.view !== null && box.view.lastStatus !== 0 && edit.text === "" && box.chatReady && box.aiBusy === ""
            label: "\u2726 Fix"
            tip: "Ask the AI to correct the command that just failed"
            onClicked: box.askFix()
        }
        Chip {
            base: box.color
            visible: box.view !== null && box.view.richKind !== ""
            label: "\u25eb " + (({ json: "JSON", csv: "Table", tsv: "Table", html: "HTML", markdown: "Markdown", svg: "SVG" })[box.view ? box.view.richKind : ""] || "View")
            tip: "Show the last command's output formatted"
            onClicked: box.openRich()
        }
        Chip {
            id: langChip
            base: box.color
            // (Shell code: the shell's own name, zsh, fish.)
            label: (box.runShell !== "" ? box.shellNames[box.runShell] : box.lang === "bash" ? (box.shellName || "bash") : box.lang) + (box.langOverride !== "" ? "" : " \u00b7 auto")
            tip: "Input language (highlighting, checks, completion)"
            onClicked: { langPop.x = langChip.x + modeTag.x - 100; langPop.y = -langPop.height - 4; langPop.open() }
        }
        Chip {
            id: modeChip
            base: box.color
            label: box.viewMode === "notebook" ? "\u25a4 Notebook" : "\u25ad Terminal"
            tip: "Click (or Ctrl+Shift+J) to switch between terminal and notebook (each command a cell)"
            onClicked: box.viewModePicked(box.viewMode === "notebook" ? "terminal" : "notebook")
        }
        Chip { base: box.color; label: "\u21ba History"; tip: "Command history (Ctrl+R)"; onClicked: box.historyMode ? box.closeComp() : box.openHistory() }
        Chip {
            id: keysChip
            base: box.color
            label: box.keys === "vi" ? "vi \u00b7 " + (box.viNormal ? "normal" : "insert")
                 : (({ emacs: "Emacs", vscode: "VS Code" })[box.keys] || box.keys)
            tip: "Editor keys: click for the next (Emacs, vi, VS Code)"
            onClicked: {
                const order = ["emacs", "vi", "vscode"]
                box.keysPicked(order[(order.indexOf(box.keys) + 1) % order.length])
                edit.forceActiveFocus()
            }
        }
    }
    // The input language: a searchable list (hundreds with Helix's grammars).
    Popup {
        id: langPop
        width: 260
        height: Math.min(360, langList.contentHeight + langFilter.height + 24)
        padding: 6
        topPadding: 6.001   // (off the half pixel: 130% scale smears native text there)
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        onOpened: { langFilter.text = ""; langFilter.forceActiveFocus() }
        onClosed: edit.forceActiveFocus()
        // (The shells by name, this pane's first: picking another runs the
        // input in it, in the pane's folder and environment.)
        readonly property var shellOptions: {
            const mine = box.shellName || "bash"
            return [["shell:" + mine, (box.shellNames[mine] || mine) + " (this pane's shell)"]]
                .concat(Object.keys(box.shellNames).filter(k => k !== mine).map(k => ["shell:" + k, box.shellNames[k]]))
        }
        // (The shells first, then Automatic, then the rest; the one in use
        // isn't in the list: switching is to something else.)
        readonly property var options: shellOptions.concat([["", "Automatic (" + (box.contentLang !== "" ? box.contentLang : box.autoLang) + ")"]], box.languages.filter(l => l[0] !== "bash"))
            .filter(o => !(o[0] === "" ? box.langOverride === "" : isPicked(o[0])))
        function isPicked(id) { return id.startsWith("shell:") ? box.langOverride !== "" && (box.runShell !== "" ? id === "shell:" + box.runShell : id === "shell:" + (box.shellName || "bash") && box.langOverride === "bash") : box.runShell === "" && box.langOverride === id }
        readonly property var shown: options.filter(o => langFilter.text === "" || (o[1] + " " + o[0]).toLowerCase().indexOf(langFilter.text.toLowerCase()) >= 0)
        contentItem: Column {
            spacing: 4
            TextField {
                id: langFilter
                width: parent.width
                placeholderText: "Search " + box.languages.length + " languages"
                color: Theme.text
                font.pixelSize: 12
                background: Rectangle { radius: 6; color: Theme.bg; border.color: langFilter.activeFocus ? Theme.accent : Theme.divider }
                Keys.onReturnPressed: if (langPop.shown.length > 0) { box.pickLanguage(langPop.shown[0][0]); langPop.close() }
                Keys.onEscapePressed: langPop.close()
            }
            ListView {
                id: langList
                width: parent.width
                height: Math.min(300, contentHeight)
                clip: true
                model: langPop.shown
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}
                MomentumWheel { flick: langList }
                delegate: Rectangle {
                    required property var modelData
                    width: langList.width
                    height: 24
                    radius: 5
                    color: lh.hovered ? Theme.tabHover : (langPop.isPicked(modelData[0]) ? Theme.mix(Theme.bg, Theme.accent, 0.25) : "transparent")
                    Text { x: 8; anchors.verticalCenter: parent.verticalCenter; text: modelData[1]; color: Theme.text; font.pixelSize: 12 }
                    HoverHandler { id: lh }
                    TapHandler { onTapped: { box.pickLanguage(modelData[0]); langPop.close() } }
                }
            }
        }
    }

    // ---- syntax -----------------------------------------------------------
    // Colors from the theme's own palette; errors show once typing pauses
    // (half-typed text is always "wrong").
    // ---- AI ------------------------------------------------------------------
    property bool aiInline: true
    property bool aiCheck: true
    property bool aiNL: true
    property string shellName: "bash"
    readonly property string aiKey: "ed-" + Math.random().toString(36).slice(2)
    readonly property bool inlineReady: controller !== null && controller.revision >= 0 && controller.aiReady("inline")
    readonly property bool chatReady: controller !== null && controller.revision >= 0 && controller.aiReady("chat")
    readonly property bool checkReady: controller !== null && controller.revision >= 0 && controller.aiReady("check")
    property string ghost: ""          // the suggestion's rest, after the cursor
    property string lastGhostBase: ""  // the text the suggestion was made for
    property string aiBusy: ""         // "command" or "fix" while the AI works on one
    property string aiNote: ""         // the check's warning
    property string checkedText: ""
    function aiContext() {
        let cwd = box.cwd.replace(/^file:\/\/[^/]*/, "")
        try { cwd = decodeURIComponent(cwd) } catch (e) {}
        // The aliases the command uses (ls -> lsd), so the AI knows what runs.
        let al = {}
        try { al = JSON.parse(box.view ? box.view.aliases : "{}") } catch (e) {}
        const used = {}
        for (const w of edit.text.split(/[\s;|&()]+/)) if (al[w] !== undefined) used[w] = al[w]
        return JSON.stringify({ cwd: cwd, shell: box.shellName || "bash", aliases: used })
    }
    // The newest command run before that starts with what's typed: the rest of it.
    // Suggestions work at the end of the line, or before closers only (the
    // quote or bracket typed in pairs): `closers` is what follows the cursor
    // then, and the suggestion is for what's before it.
    readonly property string closers: { const t = edit.text.substring(edit.cursorPosition); return t !== "" && /^[)\]}"'`]+$/.test(t) ? t : "" }
    readonly property bool atSuggestEnd: edit.cursorPosition === edit.length || closers !== ""
    function suggestBase() { return closers !== "" ? edit.text.substring(0, edit.cursorPosition) : edit.text }
    // (What a suggestion adds: when it closes what the closers close, it
    // takes their place; else they follow it.)
    function ghostTakesClosers() { return closers !== "" && ghost.indexOf(closers) >= 0 }
    function historyGhost(t) {
        if (t.trim().length < 2 || t.indexOf("\n") >= 0 || !atSuggestEnd) return ""
        for (let i = box.history.length - 1; i >= 0; i--) {
            const h = box.history[i]
            if (h.length > t.length && h.startsWith(t) && h.indexOf("\n") < 0) return h.substring(t.length)
        }
        return ""
    }
    function recent(n) { return box.history.slice(-n).join("\n") }
    function askCommand(what) {
        aiBusy = "command"
        controller.aiRun(aiKey + "-cmd", "command", JSON.stringify([{ role: "user", content: "Recent commands:\n" + recent(10) + "\n\nWhat I want: " + what }]), aiContext())
    }
    function askFix() {
        if (!box.view) return
        const lines = box.view.contentsText().split("\n")
        const tail = lines.slice(-60).join("\n")
        const last = box.history.length ? box.history[box.history.length - 1] : ""
        aiBusy = "fix"
        controller.aiRun(aiKey + "-cmd", "fix", JSON.stringify([{ role: "user", content: "Command: " + last + "\nExit status: " + box.view.lastStatus + "\nThe terminal's last lines:\n" + tail }]), aiContext())
    }
    property string ghostAsked: ""      // the text an AI suggestion was asked for
    property string nlCommand: ""       // the command a "# request" would be
    property string nlAsked: ""
    Timer {
        id: reqTimer
        interval: 250
        onTriggered: {
            const t = edit.text
            if (!box.aiNL || !box.inlineReady || !/^#\s*\S/.test(t) || t.indexOf("\n") >= 0 || edit.cursorPosition !== t.length) return
            box.reqAsked = t
            box.controller.aiRun(box.aiKey + "-req", "request", JSON.stringify([{ role: "user", content: "Recent commands:\n" + box.recent(8) + "\n\nComplete this request: " + t.replace(/^#\s*/, "") }]), box.aiContext())
        }
    }
    property string reqAsked: ""
    Timer {
        id: nlTimer
        interval: 1400
        onTriggered: {
            const t = edit.text
            // (Only when the words are done: nothing left to finish them with.)
            if (box.ghost !== "" && !box.ghost.startsWith("  \u2192")) return
            if (!box.aiNL || !box.chatReady || !/^#\s*\S/.test(t) || t.indexOf("\n") >= 0 || t === box.nlAsked) return
            box.nlAsked = t
            box.controller.aiRun(box.aiKey + "-nl", "command", JSON.stringify([{ role: "user", content: "Recent commands:\n" + box.recent(10) + "\n\nWhat I want: " + t.replace(/^#\s*/, "") }]), box.aiContext())
        }
    }
    // A suggestion stays while what's typed still leads into it; else what
    // was run before comes up at once (like fish), and the AI is asked when
    // history has nothing.
    function refreshGhost() {
        if (/^#\s*\S/.test(edit.text)) return
        const t = suggestBase()
        const full = lastGhostBase + ghost
        if (ghost !== "" && t.length > lastGhostBase.length && full.startsWith(t) && full.length > t.length) {
            ghost = full.substring(t.length)
            lastGhostBase = t
        } else {
            ghost = historyGhost(t)
            lastGhostBase = t
            if (ghost === "") ghostTimer.restart()
        }
    }
    function askGhostNow() { ghostTimer.stop(); ghostTimer.triggered() }
    function acceptGhost() {
        // (A "# request"'s suggestion: the command, in place of the line.)
        if (nlCommand !== "" && /^#/.test(edit.text)) {
            setText(nlCommand)
            nlCommand = ""
            ghost = ""
            return true
        }
        if (ghost === "") return false
        if (closers !== "") {
            const at = edit.cursorPosition
            if (ghostTakesClosers()) edit.remove(at, at + closers.length)
            edit.insert(at, ghost)
            edit.cursorPosition = edit.length
        } else {
            edit.insert(edit.length, ghost)
            edit.cursorPosition = edit.length
        }
        ghost = ""
        return true
    }
    Timer {
        id: ghostTimer
        interval: 220
        onTriggered: {
            const t = box.suggestBase()
            if (!box.aiInline || !box.inlineReady || box.lang !== box.autoLang || t.trim().length < 2 || t.startsWith("#") || box.historyMode
                    || !box.atSuggestEnd || t.indexOf("\n") >= 0) return
            box.ghostAsked = t
            box.controller.aiRun(box.aiKey + "-inline", "inline", JSON.stringify([{ role: "user", content: "Recent commands:\n" + box.recent(8) + "\n\nComplete this command line: " + t }]), box.aiContext())
        }
    }
    Timer {
        id: checkTimer
        interval: 1800
        onTriggered: {
            const t = edit.text.trim()
            // (Not while it's still being written: a suggestion says so, and so
            // does a syntax error.)
            if (!box.aiCheck || !box.checkReady || box.lang !== box.autoLang || t.length < 4 || t.startsWith("#") || t === box.checkedText
                    || box.ghost !== "" || box.problem !== "") return
            box.checkedText = t
            box.controller.aiRun(box.aiKey + "-check", "check", JSON.stringify([{ role: "user", content: t }]), box.aiContext())
        }
    }
    Connections {
        target: box.controller
        function onAiDone(key, reply, error) {
            if (key === box.aiKey + "-inline") {
                const t = box.suggestBase()
                // (Only for what it was asked about, or that typed on further:
                // a late reply after the line was run or cleared is dropped.)
                const fits = box.ghostAsked !== "" && t.trim().length >= 2 && t.startsWith(box.ghostAsked) && box.atSuggestEnd
                box.ghost = fits && !error && reply.startsWith(t) && reply.length > t.length ? reply.substring(t.length).split("\n")[0] : ""
                box.lastGhostBase = t
                // (Still being written, then: a check's complaint doesn't apply.)
                if (box.ghost !== "") { box.aiNote = ""; box.checkedText = "" }
            } else if (key === box.aiKey + "-req") {
                const t = edit.text
                if (error || t !== box.reqAsked || edit.cursorPosition !== t.length) return
                // (The reply is the whole request; what follows what's typed is the suggestion.)
                const body = t.replace(/^#\s*/, ""), r = reply.trim().replace(/^#\s*/, "").split("\n")[0]
                if (r.toLowerCase().startsWith(body.toLowerCase()) && r.length > body.length) {
                    box.ghost = r.substring(body.length)
                    box.lastGhostBase = t
                }
            } else if (key === box.aiKey + "-nl") {
                if (error || edit.text !== box.nlAsked) return
                const cmd = reply.trim().replace(/^```\w*\n?|```$/g, "").trim().split("\n")[0]
                if (cmd === "") return
                box.nlCommand = cmd
                box.ghost = "  \u2192 " + cmd
                box.lastGhostBase = edit.text
            } else if (key === box.aiKey + "-lang") {
                if (edit.text !== box.langAsked) return
                const best = error ? "" : box.controller.rankLanguages(reply, JSON.stringify(box.langRecent))
                if (best !== "" || (!error && /bash|shell/i.test(reply))) box.aiLang = best
            } else if (key === box.aiKey + "-check") {
                const r = reply.trim()
                if (box.ghost !== "" || edit.text.trim() !== box.checkedText) return
                box.aiNote = error || /^ok\.?$/i.test(r) || r === "" ? "" : r
            } else if (key === box.aiKey + "-cmd") {
                if (box.aiBusy === "") return   // (cancelled)
                box.aiBusy = ""
                if (error) box.aiNote = error
                else if (reply.trim() !== "") { box.setText(reply.trim()); box.aiNote = "" }
            }
        }
    }

    property string problem: ""        // the first syntax error, shown in the corner
    Timer { id: idle; interval: 650; onTriggered: box.analyze(true) }
    // ---- language server --------------------------------------------------
    // (Asked in the background when the language or host changes; the last
    // answer holds until the new one comes.)
    property string lspServer: ""
    function refreshLsp() {
        if (!controller) return
        const lang = box.lang, host = box.sshHost
        Async.call(controller, k => controller.lspAvailable(k, lang, host), r => {
            if (!box || box.lang !== lang || box.sshHost !== host) return
            box.lspServer = r
        })
    }
    onSshHostChanged: refreshLsp()
    property var lspDiags: []          // [[start, length, message, severity]] for lspText
    property string lspText: ""
    // (Diagnostics are for a language: bash's shellcheck says nothing about C++.)
    property string lspLang: ""
    property string lspAskedLang: ""
    property string lspNote: ""        // the first warning, for the corner
    function analyze(withErrors) {
        if (!controller) return
        const r = JSON.parse(controller.syntax(box.lang, edit.text))
        // (Shell: the shell's own check, zsh -n and the like, when typing
        // pauses; tree-sitter's bash grammar and shellcheck are too strict
        // for a command line, and zsh isn't bash.)
        if (box.lang === "bash") {
            r.errors = []
            if (withErrors && edit.text.trim() !== "") { try { r.errors = JSON.parse(controller.shellSyntax(box.shellName, edit.text)) } catch (e) {} }
        }
        // (tessh: its own check too, `tessh --check`, when typing pauses:
        // indentation, a misplaced else, macros.)
        if (box.lang === "tessh" && withErrors && edit.text.trim() !== "") {
            try { r.errors = r.errors.concat(JSON.parse(controller.shellSyntax("tessh", edit.text))) } catch (e) {}
        }
        if (!withErrors) r.errors = []
        problem = r.errors.length > 0 ? r.errors[0][2] : ""
        lspNote = ""
        if (withErrors && box.lspText === edit.text && box.lspLang === box.lang && box.lspDiags.length > 0) {
            r.errors = r.errors.concat(box.lspDiags)
            const err = box.lspDiags.find(d => d[3] <= 1), warn = box.lspDiags.find(d => d[3] >= 2)
            if (problem === "" && err) problem = err[2]
            if (warn) lspNote = warn[2]
        }
        hl.analysis = JSON.stringify(r)
        if (withErrors && box.lspServer !== "" && box.lang !== "bash" && (box.lspText !== edit.text || box.lspLang !== box.lang) && edit.text.trim() !== "") {
            box.lspAskedLang = box.lang
            controller.lspAsk(aiKey + "-diag", box.lang, box.cwd, edit.text, "diagnostics", 0, box.sshHost)
        }
    }
    property string hoverDoc: ""
    property rect hoverAt
    function hoverAtPosition(pos) {
        if (box.lspServer === "" || edit.text.trim() === "") return
        hoverAt = edit.positionToRectangle(pos)
        controller.lspAsk(aiKey + "-hover", box.lang, box.cwd, edit.text, "hover", pos, box.sshHost)
    }
    Connections {
        target: box.controller
        function onLspReady(key, what, json) {
            if (key === box.aiKey + "-diag") {
                try { box.lspDiags = JSON.parse(json || "[]") } catch (e) { box.lspDiags = [] }
                box.lspText = edit.text
                box.lspLang = box.lspAskedLang
                box.analyze(true)
            } else if (key === box.aiKey + "-hover") {
                box.hoverDoc = (json || "").trim()
            } else if (key === box.aiKey + "-comp" && box.compOpen && !box.historyMode) {
                let extra = []
                try { extra = JSON.parse(json || "[]") } catch (e) {}
                const have = new Set(box.comp.items.map(i => i.label))
                extra = extra.filter(i => i.label !== "" && !have.has(i.label))
                if (extra.length > 0) box.comp = Object.assign({}, box.comp, { items: box.comp.items.concat(extra) })
            }
        }
    }
    SyntaxHighlighter {
        id: hl
        document: edit.textDocument
        errorColor: box.ansi(9, "#e05a5a")
        warningColor: box.ansi(3, "#e5c07b")
        infoColor: box.ansi(4, "#61afef")
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
            attribute: box.ansi(3, "#e5c07b"),
            tag: box.ansi(1, "#e06c75"),
            namespace: box.ansi(3, "#e5c07b"),
            module: box.ansi(3, "#e5c07b"),
            label: box.ansi(5, "#c678dd"),
            special: box.ansi(6, "#56b6c2"),
            boolean: box.ansi(3, "#d19a66"),
            character: box.ansi(2, "#98c379"),
            markup: box.ansi(4, "#61afef"),
        })
    }
    // A language that isn't here yet is fetched (once) in the background.
    property string fetchingLang: ""
    onLangChanged: {
        refreshLsp()
        // (Another language: the last one's diagnostics go, and it's checked again.)
        lspDiags = []; lspText = ""; lspNote = ""
        idle.restart()
        analyze(false)
        if (controller && !controller.languageInstalled(lang)) {
            fetchingLang = lang
            controller.ensureLanguage(lang)
        }
    }
    Connections {
        target: box.controller
        function onLanguageReady(name, error) {
            if (name !== box.fetchingLang) return
            box.fetchingLang = ""
            if (error !== "") box.aiNote = error
            else box.analyze(true)
        }
    }

    // The dropdown, above the editor at the word being completed.
    Rectangle {
        id: compPopup
        // History: the editor's full width, with its own search field.
        readonly property bool hist: box.historyMode
        visible: box.compOpen && (hist || (box.comp.items.length > 0 && !box.typing)) && (edit.activeFocus || historySearch.activeFocus)
        z: 50
        readonly property real rowH: Math.ceil(fm.height) + 6
        readonly property int rows: Math.max(hist ? 1 : 0, Math.min(hist ? 14 : 10, box.comp.items.length))
        readonly property var anchorRect: edit.positionToRectangle(box.comp.start)
        width: hist ? box.width - 16 : Math.min(box.width - 16, 640)
        height: (hist ? searchRow.height + 6 : 0) + rows * rowH + 8 + (docText.visible ? docText.height + 12 : 0)
        x: hist ? 8 : Math.max(8, Math.min(flick.x + anchorRect.x - 28, box.width - width - 8))
        y: -height - 4
        radius: 8
        color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04)
        border.color: Theme.divider
        Rectangle {
            id: searchRow
            visible: compPopup.hist
            x: 6; y: 6
            width: parent.width - 12
            height: visible ? 30 : 0
            radius: 6
            color: Theme.mix(Theme.bg, Theme.fg, 0.04)
            border.color: historySearch.activeFocus ? Theme.accent : Theme.divider
            Text {
                id: searchIcon
                anchors.left: parent.left
                anchors.leftMargin: 9
                anchors.verticalCenter: parent.verticalCenter
                text: "\u2315"
                color: Theme.textDim
                font.pixelSize: 15
            }
            TextInput {
                id: historySearch
                anchors.left: searchIcon.right
                anchors.leftMargin: 8
                anchors.right: countText.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                color: Theme.text
                selectionColor: Theme.accent
                selectedTextColor: Theme.bg
                font.family: edit.font.family
                font.pointSize: edit.font.pointSize
                renderType: Text.NativeRendering
                clip: true
                onTextChanged: if (box.historyMode) { box.historyQuery = text; box.compIndex = 0; box.requestComplete() }
                Keys.onPressed: (e) => {
                    if (e.key === Qt.Key_Escape) { box.closeComp(); edit.forceActiveFocus(); e.accepted = true; return }
                    if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { box.acceptComp(); e.accepted = true; return }
                    if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_R) { box.compIndex = Math.min(box.comp.items.length - 1, box.compIndex + 1); e.accepted = true; return }
                    e.accepted = box.compKey(e)
                }
                Text {
                    visible: parent.text === ""
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Search history (every word, any order)"
                    color: Theme.textDim
                    opacity: 0.6
                    font: parent.font
                }
            }
            Text {
                id: countText
                anchors.right: parent.right
                anchors.rightMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                text: box.comp.items.length >= 300 ? "300+" : box.comp.items.length + (box.comp.items.length === 1 ? " command" : " commands")
                color: Theme.textDim
                font.pixelSize: 11
            }
        }
        Text {
            visible: compPopup.hist && box.comp.items.length === 0
            anchors.top: searchRow.bottom
            anchors.topMargin: 8
            anchors.horizontalCenter: parent.horizontalCenter
            text: "No commands match."
            color: Theme.textDim
            font.pixelSize: 12
        }
        ListView {
            id: compList
            x: 4; y: compPopup.hist ? searchRow.height + 10 : 4
            width: parent.width - 8
            height: compPopup.rows * compPopup.rowH
            clip: true
            model: box.comp.items
            currentIndex: box.compIndex
            highlightMoveDuration: 0
            boundsBehavior: Flickable.StopAtBounds
            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
            ScrollBar.vertical: ScrollBar { policy: compList.contentHeight > compList.height ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff }
            // (Test hook: three wheel notches down, then the position over time.)
            Timer {
                running: compPopup.visible && box.controller && box.controller.testEnv("WHEEL") !== ""
                interval: 1500
                property int step: 0
                onTriggered: {
                    // "pad": a touchpad swipe (begin, eight moves 16 ms apart, lift).
                    const p = compList.mapToItem(null, compList.width / 2, compList.height / 2)
                    if (box.controller.testEnv("WHEEL") === "pad") { padSwipe.p = p; padSwipe.n = 0; padSwipe.start(); return }
                    for (let i = 0; i < 3; i++) box.controller.testWheel(p.x, p.y, -120, 0, 0)
                    wheelLog.n = 0; wheelLog.start()
                }
            }
            Timer {
                id: padSwipe
                property var p
                property int n: 0
                interval: 16; repeat: true
                onTriggered: {
                    if (n === 0) box.controller.testWheel(p.x, p.y, 0, 0, 1)
                    if (n < 8) box.controller.testWheel(p.x, p.y, -60, -25, 2)
                    else { box.controller.testWheel(p.x, p.y, 0, 0, 3); stop(); console.log("wheel test: lifted at contentY=" + Math.round(compList.contentY)); wheelLog.n = 0; wheelLog.start() }
                    n++
                }
            }
            Timer {
                id: wheelLog
                property int n: 0
                interval: 100; repeat: true
                onTriggered: { console.log("wheel test: t=" + (n * 100) + "ms contentY=" + Math.round(compList.contentY)); if (++n > 15) stop() }
            }
            MomentumWheel { flick: compList }
            delegate: Rectangle {
                width: compList.width
                height: compPopup.rowH
                radius: 5
                color: index === box.compIndex ? Theme.mix(Theme.bg, Theme.accent, 0.35) : (rowHover.hovered ? Theme.tabHover : "transparent")
                Text {
                    id: kindTag
                    x: 6
                    anchors.verticalCenter: parent.verticalCenter
                    width: 14
                    text: ({ history: "\u21ba", command: "\u276f", subcommand: "\u25b8", option: "\u2013", dir: "\u25a2", file: "\u25ab", branch: "\u2442", host: "@", variable: "$", value: "\u2022" })[modelData.kind] || "\u2022"
                    color: ({ command: box.ansi(4, "#61afef"), subcommand: box.ansi(5, "#c678dd"), option: box.ansi(6, "#56b6c2"), dir: box.ansi(3, "#d19a66"), branch: box.ansi(2, "#98c379") })[modelData.kind] || Theme.textDim
                    font.pixelSize: 11
                    horizontalAlignment: Text.AlignHCenter
                }
                Text {
                    id: lbl
                    anchors.left: kindTag.right
                    anchors.leftMargin: 6
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    // (History: the whole command, with the time and folder to its right.)
                    width: compPopup.hist ? compList.width - kindTag.width - 32 - Math.min(histDetail.implicitWidth, 260)
                                          : Math.min(implicitWidth, compList.width * 0.45)
                    elide: Text.ElideRight
                    text: modelData.label
                    color: Theme.text
                    font.family: edit.font.family
                    font.pointSize: edit.font.pointSize
                    renderType: Text.NativeRendering
                }
                Text {
                    id: histDetail
                    anchors.left: compPopup.hist ? undefined : lbl.right
                    anchors.leftMargin: 14
                    anchors.right: parent.right
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    text: modelData.detail
                    elide: Text.ElideRight
                    color: Theme.textDim
                    font.pixelSize: 12
                    renderType: Text.NativeRendering
                }
                HoverHandler { id: rowHover }
                TapHandler { onTapped: { box.compIndex = index; box.acceptComp(index); edit.forceActiveFocus() } }
                ToolTip.visible: compPopup.hist && rowHover.hovered && lbl.truncated
                ToolTip.text: modelData.insert
                ToolTip.delay: 500
            }
        }
        // The chosen item's full help, under the list.
        Text {
            id: docText
            readonly property var item: box.comp.items[box.compIndex]
            // The panel keeps one height for the whole list (moving through
            // it mustn't make the dropdown grow and shrink): shown when any
            // item has more to say than fits its row, sized for the longest kind.
            readonly property bool anyDoc: box.comp.items.some(i => (i.doc || "") !== "")
            readonly property bool anyLong: anyDoc || box.comp.items.some(i => (i.detail || "").length > 60)
            visible: !compPopup.hist && anyLong
            height: visible ? (anyDoc ? 14 : 5) * Math.ceil(docFm.height) : 0
            FontMetrics { id: docFm; font.pixelSize: 12 }
            anchors.top: compList.bottom
            anchors.topMargin: 6
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.margins: 10
            text: item ? (item.doc || item.detail) : ""
            wrapMode: Text.Wrap
            maximumLineCount: anyDoc ? 14 : 5
            verticalAlignment: Text.AlignTop
            elide: Text.ElideRight
            color: Theme.text
            opacity: 0.85
            font.pixelSize: 12
            renderType: Text.NativeRendering
            Rectangle { anchors.bottom: parent.top; anchors.bottomMargin: 3; width: parent.width; height: 1; color: Theme.divider }
        }
    }

    // The language server's help for a word (pointed at, or F1 at the cursor).
    Rectangle {
        id: hoverPopup
        visible: box.hoverDoc !== "" && !compPopup.visible
        z: 49
        width: Math.min(box.width - 16, 620)
        height: Math.min(hoverText.implicitHeight + 16, 260)
        x: Math.max(8, Math.min(flick.x + box.hoverAt.x - 20, box.width - width - 8))
        y: -height - 4
        radius: 8
        color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04)
        border.color: Theme.divider
        Flickable {
            id: hoverFlick
            anchors.fill: parent
            anchors.margins: 8
            contentHeight: hoverText.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}
            MomentumWheel { flick: hoverFlick }
            Text {
                id: hoverText
                width: hoverFlick.width - 8
                text: box.hoverDoc
                textFormat: Text.MarkdownText
                wrapMode: Text.Wrap
                color: Theme.text
                font.pixelSize: 12
                renderType: Text.NativeRendering
            }
        }
    }
}
