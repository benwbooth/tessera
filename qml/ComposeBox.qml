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
            if (!box.controller || box.autoLang !== "bash" || box.langOverride !== "" || box.contentLang !== "" || t.trim() === "" || t.startsWith("#")) { box.aiLang = ""; return }
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
    // Background jobs on this pane's host (the notebook keeps count): the
    // badge shows how many run; a click shows or hides their list.
    property int jobsRunning: 0
    property int jobsAll: 0
    signal jobsToggled()
    signal handBack()                  // Ctrl tapped: type into the terminal instead

    readonly property alias text: edit.text
    readonly property bool editing: edit.activeFocus
    readonly property real lineHeight: Math.ceil(fm.height)
    readonly property int lines: Math.max(1, edit.lineCount)
    // (The editor's margins, and the prompt bar with the gap under it.)
    // (The text's real laid-out height, which some fonts make taller than
    // their metrics say, up to ten lines; then it scrolls.)
    // (The toolbar row (modeTag) on top, the prompt bar under it, then the
    // editor.)
    implicitHeight: Math.ceil(Math.min(Math.max(edit.contentHeight, lineHeight), 10 * lineHeight)) + flickTop + 9
    readonly property real flickTop: promptBar.visible ? promptBar.y + promptBar.height + 4 : modeTag.y + modeTag.height + 4

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

    // The editor's keys, for the hint's tooltip: a table, keys on the left.
    function keysHelp() {
        const groups = [
            ["Running", [["Enter", "Run the command \u2014 while one runs, it waits its turn"],
                         ["Right Ctrl", "Run it in the background, alongside (tap; or Ctrl+Enter)"],
                         ["Left Ctrl", "Switch between this editor and the terminal (tap)"]]],
            ["Editing", [["Right Shift", "New line (tap; or Shift+Enter)"],
                         ["Left Shift", "Take the grey suggestion (tap)"],
                         ["Tab", "Complete; in the indentation, indent"],
                         ["Shift+Tab", "Outdent"]]],
            ["Finding", [["Ctrl+R", "Search history"],
                         ["Ctrl+Alt+J", "Background jobs"]]]]
        if (box.aiNL && box.chatReady) groups.push(["AI", [["# \u2026", "Say what you want: the AI writes the command, for you to check"]]])
        let h = "<table cellspacing='0' cellpadding='2'>"
        for (const g of groups) {
            h += "<tr><td colspan='2' style='padding-top:4px'><span style='color:" + Theme.textDim + "'>" + g[0] + "</span></td></tr>"
            for (const r of g[1]) h += "<tr><td style='padding-right:14px'><b>" + r[0] + "</b></td><td>" + r[1] + "</td></tr>"
        }
        return h + "</table>"
    }
    // How the last submit wants it run: "" (now, or queued behind what's
    // running) or "bg" (in the background at once, alongside it: as a
    // shell's &, for any language). Ctrl+Enter or a tap of right Ctrl.
    property string submitMode: ""
    // A command's running with the keys (the editor's shown, waiting).
    property bool waiting: false
    // Ctrl held down now (its press seen, not its release).
    property bool ctrlDown: false
    // Commands waiting for the one running to finish (the pane's), and a
    // signal to drop them.
    property int queued: 0
    signal clearQueue()
    function submitBackground() { if (edit.text.trim() === "") return; submit("bg") }
    function submit(mode) {
        submitMode = mode || ""
        const t = edit.text
        // "# what I want": the AI writes the command, for you to check first.
        if (aiNL && /^#\s*\S/.test(t) && t.indexOf("\n") < 0 && chatReady) { askCommand(t.replace(/^#\s*/, "")); return }
        ghost = ""
        if (t.trim() !== "") {
            const h = box.history
            const old = h.indexOf(t)
            if (old >= 0) h.splice(old, 1)
            h.push(t)
            if (controller) controller.historyAdd(t, box.cwd, box.lang)
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
    // (commandRun: while it's submitted, so a notebook runs it in the terminal too.)
    property bool commandRun: false
    function runCommand(c) { runLang = "bash"; runShellUsed = ""; commandRun = true; box.submitted(c); commandRun = false }
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
    function kill(a, b, append) {
        if (b < a) { const x = a; a = b; b = x }
        const cut = edit.text.substring(a, b)
        killRing = append ? killRing + cut : cut
        edit.remove(a, b)
        edit.cursorPosition = a
    }

    // ---- VS Code-style editing: indentation, pairs -------------------------
    // (Preferences › Input editor: tabs or spaces, and how many columns a
    // level and a tab are.)
    readonly property var edPrefs: { const w = box.Window.window; return w && w.settings ? w.settings : ({}) }
    // Each language's usual indentation ([mode, size]); the indent chip's
    // choice for a language is kept per language (editor_indent_langs).
    readonly property var indentDefaults: ({
        python: ["spaces", 4], rust: ["spaces", 4], go: ["tabs", 4], c: ["spaces", 4], cpp: ["spaces", 4], java: ["spaces", 4],
        "c-sharp": ["spaces", 4], csharp: ["spaces", 4], kotlin: ["spaces", 4], swift: ["spaces", 4], php: ["spaces", 4], perl: ["spaces", 4],
        julia: ["spaces", 4], zig: ["spaces", 4], d: ["spaces", 4], groovy: ["spaces", 4], fsharp: ["spaces", 4], make: ["tabs", 8], makefile: ["tabs", 8],
        javascript: ["spaces", 2], typescript: ["spaces", 2], tsx: ["spaces", 2], json: ["spaces", 2], yaml: ["spaces", 2], html: ["spaces", 2], css: ["spaces", 2],
        ruby: ["spaces", 2], lua: ["spaces", 2], nix: ["spaces", 2], haskell: ["spaces", 2], ocaml: ["spaces", 2], elixir: ["spaces", 2], erlang: ["spaces", 4],
        scala: ["spaces", 2], dart: ["spaces", 2], r: ["spaces", 2], nim: ["spaces", 2], sql: ["spaces", 2], clojure: ["spaces", 2], racket: ["spaces", 2],
        lisp: ["spaces", 2], scheme: ["spaces", 2], fennel: ["spaces", 2], janet: ["spaces", 2], elisp: ["spaces", 2],
        tessh: ["spaces", 2], bash: ["spaces", 2], zsh: ["spaces", 2], fish: ["spaces", 4], powershell: ["spaces", 4], markdown: ["spaces", 2]
    })
    readonly property var indentPref: {
        const d = indentDefaults[lang] || ["spaces", 4]
        const m = (edPrefs.editor_indent_langs || {})[lang] || {}
        return { mode: ["spaces", "tabs", "mixed"].indexOf(m.mode) >= 0 ? m.mode : d[0], size: m.size > 0 ? m.size : d[1], defMode: d[0], defSize: d[1], own: m.mode !== undefined || m.size !== undefined }
    }
    // (Picked on the chip: that, not what's detected, until the editor's
    // emptied.)
    property bool indentPicked: false
    Connections { target: edit; function onTextChanged() { if (edit.text === "") box.indentPicked = false } }
    function setIndentPref(key, value) {
        if (!controller) return
        indentPicked = value !== null
        const all = JSON.parse(JSON.stringify(edPrefs.editor_indent_langs || {}))
        const m = all[lang] || {}
        if (value === null) delete m[key]; else m[key] = value
        if (Object.keys(m).length) all[lang] = m; else delete all[lang]
        controller.setSetting("editor_indent_langs", JSON.stringify(all))
    }
    // What the text itself is indented with, as VS Code detects it: tabs,
    // spaces (and how many a level: the commonest step in from one line to
    // the next), or both (mixed); null when it has no indented lines yet.
    // Followed over the language's setting unless detecting is off.
    readonly property var detectedIndent: {
        if (edPrefs.editor_detect_indent === false || indentPicked) return null
        const lines = edit.text.split("\n").filter(l => l.trim() !== "")
        let tabs = 0, spaces = 0
        const steps = {}
        let prev = 0
        for (const l of lines) {
            const ws = /^[ \t]*/.exec(l)[0]
            if (ws.indexOf("\t") >= 0) tabs++
            else if (ws.length > 0) spaces++
            const c = ws.indexOf("\t") >= 0 ? -1 : ws.length
            if (c >= 0 && c > prev && c - prev <= 8) steps[c - prev] = (steps[c - prev] || 0) + 1
            if (c >= 0) prev = c
        }
        if (tabs === 0 && spaces === 0) return null
        let size = 0, best = 0
        for (const k in steps) if (steps[k] > best || (steps[k] === best && Number(k) < size)) { best = steps[k]; size = Number(k) }
        if (tabs > 0 && spaces === 0) return { mode: "tabs", size: indentPref.size }
        if (size === 0) return null
        return { mode: tabs > 0 ? "mixed" : "spaces", size: size }
    }
    readonly property int tabSize: detectedIndent ? detectedIndent.size : indentPref.size
    // "spaces", "tabs", or "mixed" (levels of tabSize, every 8 columns a
    // tab, as Emacs and vi do it).
    readonly property string indentMode: detectedIndent ? detectedIndent.mode : indentPref.mode
    readonly property bool useTabs: indentMode === "tabs"
    // (A tab's width on screen: a level, but 8 when they're mixed.)
    readonly property int tabWidth: indentMode === "mixed" ? 8 : tabSize
    readonly property string indentUnit: useTabs ? "\t" : " ".repeat(tabSize)
    // Leading whitespace <-> its width in columns.
    function colsOf(ws) { let c = 0; for (const ch of ws) { if (ch === "\t") c += tabWidth - c % tabWidth; else if (ch === " ") c++; else break } return c }
    function indentFor(cols) {
        cols = Math.max(0, cols)
        if (indentMode === "spaces") return " ".repeat(cols)
        const w = indentMode === "mixed" ? 8 : tabSize
        return "\t".repeat(Math.floor(cols / w)) + " ".repeat(cols % w)
    }
    // Replace the leading whitespace of the line at `a` with `cols` columns'.
    function setIndent(a, cols) {
        const m = /^[ \t]*/.exec(edit.text.substring(a, lineEnd(a)))[0]
        const ws = indentFor(cols)
        if (ws === m) return 0
        edit.remove(a, a + m.length)
        edit.insert(a, ws)
        return ws.length - m.length
    }
    // Line numbers: each line's top (y), when there are several and they're on.
    readonly property var lineStarts: {
        if (edPrefs.editor_line_numbers === false) return []
        const text = edit.text
        edit.width; edit.contentHeight; edit.font.pointSize
        if (text.indexOf("\n") < 0) return []
        const out = []
        let p = 0
        for (const l of text.split("\n")) { out.push(edit.positionToRectangle(p).y); p += l.length + 1 }
        return out
    }
    readonly property int cursorLine: edit.text.substring(0, edit.cursorPosition).split("\n").length - 1
    readonly property int lineCount: edPrefs.editor_line_numbers === false ? 1 : edit.text.split("\n").length
    readonly property real gutterWidth: lineCount > 1 ? String(lineCount).length * fm.advanceWidth("0") + 10 : 0
    // Indent guides: [{x, y, h, on}], a line per indent level down the lines
    // indented past it (blank lines between them included), the one around
    // the cursor's block brighter, as VS Code draws them.
    readonly property var indentGuides: {
        if (edPrefs.editor_indent_guides === false) return []
        const text = edit.text, cw = fm.advanceWidth(" ")
        edit.width; edit.contentHeight; edit.font.pointSize
        if (text.indexOf("\n") < 0 && !/^[ \t]/.test(text)) return []
        const lines = text.split("\n"), n = lines.length
        const cols = l => box.colsOf(l)
        const blank = l => l.trim() === ""
        const ind = lines.map(l => blank(l) ? -1 : cols(l))
        // (A blank line: as deep as the shallower of the lines around it.)
        for (let i = 0; i < n; i++) if (ind[i] < 0) {
            let a = 0, b = 0
            for (let j = i - 1; j >= 0; j--) if (ind[j] >= 0 && !blank(lines[j])) { a = ind[j]; break }
            for (let j = i + 1; j < n; j++) if (!blank(lines[j])) { b = cols(lines[j]); break }
            ind[i] = Math.min(a, b)
        }
        const starts = []
        let p = 0
        for (let i = 0; i < n; i++) { starts.push(p); p += lines[i].length + 1 }
        const top = i => edit.positionToRectangle(starts[i]).y
        const bottom = i => i + 1 < n ? edit.positionToRectangle(starts[i + 1]).y : edit.contentHeight
        // (The cursor's block: its line's deepest level.)
        const cl = text.substring(0, edit.cursorPosition).split("\n").length - 1
        const cLevel = Math.floor(ind[cl] / tabSize) - 1
        const out = []
        const maxLevel = Math.max(0, ...ind.map(c => Math.ceil(c / tabSize)))
        for (let k = 0; k < maxLevel; k++) {
            let i = 0
            while (i < n) {
                if (ind[i] <= k * tabSize) { i++; continue }
                let j = i
                while (j + 1 < n && ind[j + 1] > k * tabSize) j++
                const on = k === cLevel && cl >= i && cl <= j
                out.push({ x: k * tabSize * cw, y: top(i), h: bottom(j) - top(i), on: on })
                i = j + 1
            }
        }
        return out
    }
    readonly property var pairs: ({ "(": ")", "[": "]", "{": "}", "\"": "\"", "'": "'", "`": "`" })
    // As VS Code does it: a pair is typed only before whitespace, the line's
    // end or one of these (autoCloseBefore), a quote not inside a string or
    // after a word; the closers it put in (and only those) are typed over and
    // go with their opener on Backspace, until the cursor leaves the pair.
    readonly property string closeBefore: ";:.,=}])> \t\n"
    property var autoPairs: []          // [{open, close}]: the pairs typed for you, by position
    function trackPairs(was, now) {
        if (autoPairs.length === 0) return
        // (What changed: the same start and end, the middle replaced.)
        let a = 0
        const n = Math.min(was.length, now.length)
        while (a < n && was[a] === now[a]) ++a
        let b = 0
        while (b < n - a && was[was.length - 1 - b] === now[now.length - 1 - b]) ++b
        const endWas = was.length - b, delta = now.length - was.length
        const out = []
        for (const q of autoPairs) {
            const o = q.open < a ? q.open : q.open >= endWas ? q.open + delta : -1
            const c = q.close < a ? q.close : q.close >= endWas ? q.close + delta : -1
            if (o >= 0 && c > o) out.push({ open: o, close: c })
        }
        autoPairs = out
    }
    function prunePairs() { const c = edit.cursorPosition; if (autoPairs.length) autoPairs = autoPairs.filter(q => c > q.open && c <= q.close) }
    function autoPairAt(close) { return autoPairs.findIndex(q => q.close === close) }
    // (Inside a string on this line, before p: a quote opened and not closed.)
    function inString(p) {
        const t = edit.text, a = lineStart(p)
        let q = ""
        for (let i = a; i < p; ++i) {
            const c = t[i]
            if (q !== "") { if (c === "\\") ++i; else if (c === q) q = "" }
            else if (c === "\"" || c === "'" || c === "`") q = c
            else if (c === "#" && lang !== "javascript" && lang !== "typescript") break   // (a comment)
        }
        return q !== ""
    }
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
        // (A new line isn't taking the suggestion: it goes.)
        ghost = ""
        lastGhostBase = ""
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
        const deeper = opens ? indentFor(colsOf(ind) + tabSize) : ind
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
        const c = colsOf(indentOf(p))
        if (c < tabSize) return
        const pos = edit.cursorPosition
        edit.cursorPosition = pos + setIndent(a, Math.ceil(c / tabSize) * tabSize - tabSize)
    }
    function shiftLines(dir) {
        // Indent (1) or outdent (-1) the current line or every selected one.
        const s = edit.selectionStart, e = edit.selectionEnd
        let a = lineStart(s)
        const lines = edit.text.substring(a, lineEnd(e)).split("\n")
        // (A level in or out, onto the levels: 3 columns at size 2 go to 4, or 2.)
        const out = lines.map(l => {
            const m = /^[ \t]*/.exec(l)[0], c = colsOf(m)
            if (l.trim() === "" && dir > 0) return l
            const to = dir > 0 ? Math.floor(c / tabSize) * tabSize + tabSize : Math.ceil(c / tabSize) * tabSize - tabSize
            return indentFor(to) + l.substring(m.length)
        })
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
            if (lead) {
                // (To the next level, written as the indent setting says.)
                const a = lineStart(p), c = colsOf(t.substring(a, p))
                setIndent(a, Math.floor(c / tabSize) * tabSize + tabSize)
                edit.cursorPosition = a + /^[ \t]*/.exec(edit.text.substring(a, lineEnd(a)))[0].length
                return true
            }
            return false   // (completion takes Tab elsewhere)
        }
        if (e.key === Qt.Key_Backspace && p > 0 && !edit.selectedText) {
            // Between a pair: remove both. In indentation: a whole level.
            const o = t[p - 1]
            const k = autoPairAt(p)
            if (pairs[o] && t[p] === pairs[o] && k >= 0 && autoPairs[k].open === p - 1) { edit.remove(p - 1, p + 1); return true }
            const lead = t.substring(lineStart(p), p)
            if (lead.length > 0 && /^[ \t]+$/.test(lead)) {
                // (Back a level, onto the levels.)
                const a = lineStart(p), c = colsOf(lead)
                const rest = /^[ \t]*/.exec(t.substring(a, lineEnd(a)))[0].length - lead.length
                setIndent(a, Math.ceil(c / tabSize) * tabSize - tabSize)
                edit.cursorPosition = a + /^[ \t]*/.exec(edit.text.substring(a, lineEnd(a)))[0].length - rest
                return true
            }
            return false
        }
        const ch = e.text
        if (ch.length !== 1) return false
        // Over a closer typed for you: step past it (one typed by hand isn't
        // skipped: the new one goes in).
        if (")]}\"'`".indexOf(ch) >= 0 && t[p] === ch && !edit.selectedText) {
            const k = autoPairAt(p)
            if (k >= 0) {
                autoPairs = autoPairs.filter((q, i) => i !== k)
                edit.cursorPosition = p + 1
                return true
            }
            if ("\"'`".indexOf(ch) >= 0) return false   // (a quote: typed, closing its string)
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
            const prev = t[p - 1] || " ", next = t[p] === undefined ? "\n" : t[p] === "\u2029" ? "\n" : t[p]
            // (Before something else: just the opener, as typed.)
            if (closeBefore.indexOf(next) < 0) return false
            // (A quote: not after a word (it's, $'x), and not closing a string.)
            if (quote && (/[A-Za-z0-9_\\$]/.test(prev) || inString(p))) return false
            insert(ch + pairs[ch])
            edit.cursorPosition = p + 1
            autoPairs = autoPairs.concat([{ open: p, close: p + 1 }])
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
    // A history entry's text colored as the editor colors it (its spans from
    // history::search), as StyledText.
    function coloredLine(text, spans) {
        const n = text.length
        if (!spans || spans.length === 0) return box.escapeStyled(text)
        const col = new Array(n).fill("")
        for (const s of spans) {
            const c = hl.colors[s[2]]
            if (!c) continue
            for (let i = s[0]; i < s[0] + s[1] && i < n; i++) col[i] = "" + c
        }
        let out = "", i = 0
        while (i < n) {
            let j = i
            while (j < n && col[j] === col[i]) j++
            const t = box.escapeStyled(text.substring(i, j))
            out += col[i] !== "" ? "<font color=\"" + col[i] + "\">" + t + "</font>" : t
            i = j
        }
        return out
    }
    function escapeStyled(t) { return t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/ /g, "&nbsp;") }
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
        // (tessh: its own names, programs and files, not the shell's.)
        const tessh = box.lang === "tessh" && box.sshHost === ""
        Async.call(controller, k => hist ? controller.historySearch(k, query, box.lang)
                                 : tessh ? controller.completeTessh(k, text, cursor, box.cwd)
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
    // ---- the chosen item's help, read in the background ----
    // (A command's examples and man page, an option's paragraph, a
    // subcommand's page: what the item doesn't carry itself. Kept by
    // command line context, item, and kind.)
    property var fetchedDoc: ({})
    // (A command with examples of its own: its man page goes under them.)
    // The word to look up in the guide: the selection (one line), else the
    // word the cursor is in or just after.
    function guideWord() {
        const sel = edit.selectedText.trim()
        if (sel !== "" && sel.length <= 60 && sel.indexOf("\n") < 0 && sel.indexOf("\u2029") < 0) return sel
        const t = edit.text, p = edit.cursorPosition
        let a = p, b = p
        while (a > 0 && /[\w$@]/.test(t[a - 1])) a--
        while (b < t.length && /\w/.test(t[b])) b++
        const w = t.slice(a, b)
        return /\w/.test(w) ? w : ""
    }
    function docFetchable(i) { return !!i && (i.kind === "command" && box.lang !== "tessh" ? true : (i.doc || "") === "") && ["command", "option", "opt", "subcommand"].indexOf(i.kind) >= 0 && i.detail !== "keyword" && i.detail !== "help topic" }
    function docKind(i) { return i.kind === "command" && (i.doc || "") !== "" ? "man" : i.kind }
    function docLine() { return edit.text.slice(0, comp.start) }
    function docKey(i) {
        // (An option's or subcommand's help depends on the command it's of.)
        const ctx = i.kind === "command" ? "" : (docLine().split(/[|;&]/).pop().trim().split(/\s+/).filter(w => w.indexOf("=") < 0)[0] || "")
        return docKind(i) + "\u0001" + ctx + "\u0001" + i.label
    }
    Timer {
        id: docFetch
        interval: 120
        onTriggered: {
            const i = box.comp.items[box.compIndex]
            if (!box.compOpen || !box.docFetchable(i) || !box.controller) return
            const k = box.docKey(i)
            if (box.fetchedDoc[k] !== undefined) return
            const line = box.docLine(), host = box.sshHost
            Async.call(box.controller, q => box.controller.itemDoc(q, host, box.docKind(i), i.label, line), doc => {
                const f = Object.assign({}, box.fetchedDoc); f[k] = doc; box.fetchedDoc = f
            })
        }
    }
    onCompIndexChanged: docFetch.restart()
    onCompChanged: docFetch.restart()
    // (The bar's chips, by key while this is the active pane's input; their
    // tooltips say them.)
    function openLanguages() { langPop.x = langChip.x + modeTag.x - 100; langPop.y = -langPop.height - 4; langPop.open() }
    readonly property bool keysHere: box.visible && Window.window !== null && Window.window.activePane !== null && Window.window.activePane !== undefined && Window.window.activePane.compose === box
    Shortcut { sequence: "Ctrl+Alt+I"; enabled: box.keysHere; context: Qt.WindowShortcut; onActivated: { box.focusEditor(); box.openLanguages() } }
    Shortcut { sequence: "Ctrl+Alt+E"; enabled: box.keysHere; context: Qt.WindowShortcut; onActivated: { const order = ["emacs", "vi", "vscode"]; box.keysPicked(order[(order.indexOf(box.keys) + 1) % order.length]) } }
    function compKey(e) {
        if (!compOpen || comp.items.length === 0) return false
        const n = comp.items.length
        // (Shift+PgUp/PgDn: the help over the list.)
        if ((e.modifiers & Qt.ShiftModifier) && (e.key === Qt.Key_PageDown || e.key === Qt.Key_PageUp)) { compPopup.scrollDoc(e.key === Qt.Key_PageDown ? 1 : -1); return true }
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
            // (Ctrl+Enter: in the background, alongside what's running. Ctrl
            // really held: keyd's Enter (overload(control, enter)) sends a
            // Ctrl tap just before a tapped Enter, and that Enter can come
            // still marked Ctrl.)
            if (ctrl && box.ctrlDown) { submitBackground(); return true }
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
        anchors.top: modeTag.bottom
        anchors.topMargin: 2
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
        anchors.rightMargin: 10
        anchors.topMargin: box.flickTop
        anchors.bottomMargin: 7
        contentWidth: width
        contentHeight: edit.contentHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        function ensureVisible(r) {
            if (contentY >= r.y) contentY = r.y
            else if (contentY + height <= r.y + r.height) contentY = r.y + r.height - height
        }

        // Line numbers, once there's more than one line (each line's number
        // at its first row: a wrapped line's other rows have none).
        Repeater {
            model: box.lineStarts
            Text {
                required property var modelData
                required property int index
                x: 0
                width: box.gutterWidth - 8
                y: modelData + 0.001
                horizontalAlignment: Text.AlignRight
                text: index + 1
                color: index === box.cursorLine ? Theme.text : Theme.textDim
                opacity: index === box.cursorLine ? 0.8 : 0.5
                font: edit.font
                renderType: Text.NativeRendering
            }
        }
        TextEdit {
            id: edit
            x: box.gutterWidth
            width: flick.width - box.gutterWidth
            wrapMode: TextEdit.Wrap
            textFormat: TextEdit.PlainText
            renderType: Text.NativeRendering
            selectByMouse: true
            persistentSelection: false
            font.family: box.view ? box.view.fontFamily : "monospace"
            font.pointSize: box.view && box.view.fontPointSize > 0 ? box.view.fontPointSize : 11
            tabStopDistance: box.tabWidth * fm.advanceWidth(" ")
            color: Theme.fg
            selectionColor: Theme.accent
            selectedTextColor: Theme.bg
            // A block cursor like the terminal's, the character under it in
            // the background color (a hollow box when the editor isn't typing).
            // The indent guides, under the text.
            Repeater {
                model: box.indentGuides
                Rectangle {
                    required property var modelData
                    z: -1
                    x: Math.round(modelData.x) + 0.5
                    y: modelData.y
                    width: 1
                    height: modelData.h
                    color: Theme.fg
                    opacity: modelData.on ? 0.32 : 0.12
                }
            }
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
            // (A pair typed for you is forgotten once the cursor leaves it.)
            // (After the edit that moved it is counted: an insert moves the cursor first.)
            onCursorPositionChanged: if (box.autoPairs.length) Qt.callLater(box.prunePairs)
            // (Re-coloring also signals a change: react to real edits only.)
            property string seen: ""
            onTextChanged: {
                if (text === seen) return
                box.trackPairs(seen, text)
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
                if (e.key === Qt.Key_Control) box.ctrlDown = false
                if (e.key === Qt.Key_Shift && shiftSide !== 0 && Date.now() - shiftAt < 450) {
                    if (shiftSide === 2) box.newline()
                    else if (!box.acceptGhost()) box.askGhostNow()
                    e.accepted = true
                }
                if (e.key === Qt.Key_Shift) shiftSide = 0
            }
            Keys.onPressed: (e) => {
                if (e.key === Qt.Key_Control) box.ctrlDown = true
                exLine.dismiss()
                if (e.key !== Qt.Key_Up && e.key !== Qt.Key_Down) box.fromHistory = false
                if (e.key === Qt.Key_Shift && !e.isAutoRepeat) { shiftSide = shiftSideOf(e); shiftAt = Date.now() }
                else shiftSide = 0
                let done = false
                const plain = !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.ShiftModifier))
                if (box.ghost !== "" && box.atSuggestEnd && plain
                        && (e.key === Qt.Key_Right || e.key === Qt.Key_End))
                    done = box.acceptGhost()
                if (!done && e.key === Qt.Key_Escape && box.ghost !== "") { box.ghost = ""; done = true }
                if (!done && e.key === Qt.Key_Escape && box.aiBusy !== "") { box.controller.aiCancel(box.aiKey + "-cmd"); box.aiBusy = ""; done = true }
                if (!done && e.key === Qt.Key_F1 && (e.modifiers & Qt.ShiftModifier) && guideChip.guide !== "") { Window.window.openGuide(guideChip.guide, box.guideWord()); done = true }
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
        // Empty: a hint of what this is, on the first line as typed text
        // and the prompt character are (centred in the box it sat lower).
        x: flick.x + 0.001
        y: flick.y + 0.001
        visible: edit.text === "" && !(box.keys === "vi" && box.viNormal)
        width: flick.width
        elide: Text.ElideRight
        text: box.waiting ? "Running \u2014 click or tap left Ctrl to queue the next command"
            : box.aiNL && box.chatReady ? "Type a command, or # and a request for the AI — hover for keys"
                                         : "Type a command — hover for keys"
        HoverHandler { id: hintHover }
        // (Its own tooltip: the default one draws text, not a table.)
        ToolTip {
            visible: hintHover.hovered
            delay: 300
            topPadding: 8.001
            bottomPadding: 8
            leftPadding: 10
            rightPadding: 10
            contentItem: Text {
                text: box.keysHelp()
                textFormat: Text.RichText
                color: Theme.text
                font.pixelSize: 12
                renderType: Text.NativeRendering
            }
            background: Rectangle { radius: 6; color: Theme.panel; border.color: Theme.divider }
        }
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

    // The toolbar, a row of its own at the top, over the prompt bar (the
    // editor has the whole width): a problem, or the language / vi mode;
    // the history button and the key switch (Emacs, vi, VS Code).
    Row {
        id: modeTag
        anchors.right: parent.right
        anchors.rightMargin: 8
        y: 5
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
            visible: box.queued > 0
            base: Theme.mix(box.color, Theme.accent, 0.3)
            label: "\u23f3 " + box.queued + " queued"
            tip: "Commands waiting for the running one to finish, run in turn. Click to drop them. (Ctrl+Enter or a tap of right Ctrl runs a command in the background instead.)"
            onClicked: box.clearQueue()
        }
        Chip { base: box.color; label: "\u21ba History"; tip: "Command history (Ctrl+R)"; onClicked: box.historyMode ? box.closeComp() : box.openHistory() }
        Chip {
            id: langChip
            base: box.color
            // (Shell code: the shell's own name, zsh, fish.)
            label: (box.runShell !== "" ? box.shellNames[box.runShell] : box.lang === "bash" ? (box.shellName || "bash") : box.lang) + (box.langOverride !== "" ? "" : " \u00b7 auto")
            tip: "Input language: highlighting, checks, completion (Ctrl+Alt+I)"
            onClicked: box.openLanguages()
        }
        // Indentation, as VS Code's status bar has it: spaces, tabs or mixed,
        // and how wide a level is; kept per language.
        Chip {
            id: indentChip
            base: box.color
            label: ({ spaces: "Spaces: ", tabs: "Tabs: ", mixed: "Mixed: " })[box.indentMode] + box.tabSize + (box.detectedIndent ? " \u00b7 detected" : "")
            tip: (box.detectedIndent ? "Detected from the text. " : "") + "Indentation for " + box.lang + ": spaces, tabs or mixed (a tab for every 8 columns), and the size of a level. Kept for each language."
            onClicked: indentMenu.popup(indentChip, 0, -indentMenu.implicitHeight - 4)
        }
        SlimMenu {
            id: indentMenu
            readonly property var p: box.indentPref
            SlimMenuItem { text: "Indent with spaces"; checkable: true; checked: indentMenu.p.mode === "spaces"; onTriggered: box.setIndentPref("mode", "spaces") }
            SlimMenuItem { text: "Indent with tabs"; checkable: true; checked: indentMenu.p.mode === "tabs"; onTriggered: box.setIndentPref("mode", "tabs") }
            SlimMenuItem { text: "Mixed (a tab for every 8 columns)"; checkable: true; checked: indentMenu.p.mode === "mixed"; onTriggered: box.setIndentPref("mode", "mixed") }
            SlimMenuItem { text: "Size"; enabled: false }
            Repeater {
                model: [1, 2, 4, 8]
                SlimMenuItem { required property int modelData; text: String(modelData); checkable: true; checked: indentMenu.p.size === modelData; onTriggered: box.setIndentPref("size", modelData) }
            }
            SlimMenuItem {
                text: "Custom…" + ([1, 2, 4, 8].indexOf(indentMenu.p.size) < 0 ? " (" + indentMenu.p.size + ")" : "")
                checkable: true; checked: [1, 2, 4, 8].indexOf(indentMenu.p.size) < 0
                onTriggered: Window.window.askText("Indent size", "Columns per level for " + box.lang + ":", String(indentMenu.p.size), v => { const n = parseInt(v); if (n > 0 && n <= 32) box.setIndentPref("size", n) })
            }
            SlimMenuItem {
                text: "Detect from the text"
                checkable: true; checked: box.edPrefs.editor_detect_indent !== false
                onTriggered: if (box.controller) box.controller.setSetting("editor_detect_indent", JSON.stringify(box.edPrefs.editor_detect_indent === false))
            }
            SlimMenuItem {
                text: "Use " + box.lang + "'s default (" + ({ spaces: "spaces", tabs: "tabs", mixed: "mixed" })[indentMenu.p.defMode] + ", " + indentMenu.p.defSize + ")"
                enabled: indentMenu.p.own
                onTriggered: { box.setIndentPref("mode", null); box.setIndentPref("size", null) }
            }
        }
        Chip {
            base: box.color
            id: guideChip
            readonly property string guide: Window.window && Window.window.guideFor ? Window.window.guideFor(box.lang, box.runShell || box.shellName) : ""
            visible: guide !== ""
            label: "Guide"
            tip: (guide === "tessh" ? "The tessh guide, in a tab: every feature, with examples"
                : "The " + Window.window.guides[guide] + ", in a tab: " + guide + "'s own documentation, fetched the first time and kept")
                + ". Opens at the selected word, or the word at the cursor (Shift+F1)"
            onClicked: Window.window.openGuide(guide, box.guideWord())
        }
        Chip {
            visible: box.jobsAll > 0
            base: box.jobsRunning > 0 ? Theme.mix(box.color, Theme.accent, 0.35) : box.color
            label: "\u29d7 Jobs " + box.jobsRunning
            tip: box.jobsRunning + " background job" + (box.jobsRunning === 1 ? "" : "s") + " running (" + box.jobsAll + " in all): click to show or hide them (Ctrl+Alt+J)"
            onClicked: box.jobsToggled()
        }
        Chip {
            id: keysChip
            base: box.color
            label: box.keys === "vi" ? "vi \u00b7 " + (box.viNormal ? "normal" : "insert")
                 : (({ emacs: "Emacs", vscode: "VS Code" })[box.keys] || box.keys)
            tip: "Editor keys: click for the next, Emacs, vi, VS Code (Ctrl+Alt+E)"
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
        onOpened: { langFilter.text = ""; sel = 0; langFilter.forceActiveFocus() }
        // (The one Enter picks: Up/Down move it, typing starts over at the top.)
        property int sel: 0
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
                placeholderText: "Search " + box.languages.length + " languages (\u2191\u2193, Enter)"
                color: Theme.text
                font.pixelSize: 12
                background: Rectangle { radius: 6; color: Theme.bg; border.color: langFilter.activeFocus ? Theme.accent : Theme.divider }
                onTextChanged: langPop.sel = 0
                Keys.onReturnPressed: if (langPop.shown.length > 0) { box.pickLanguage(langPop.shown[Math.min(langPop.sel, langPop.shown.length - 1)][0]); langPop.close() }
                Keys.onDownPressed: { langPop.sel = Math.min(langPop.shown.length - 1, langPop.sel + 1); langList.positionViewAtIndex(langPop.sel, ListView.Contain) }
                Keys.onUpPressed: { langPop.sel = Math.max(0, langPop.sel - 1); langList.positionViewAtIndex(langPop.sel, ListView.Contain) }
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
                    required property int index
                    width: langList.width
                    height: 24
                    radius: 5
                    color: index === langPop.sel ? Theme.mix(Theme.bg, Theme.accent, 0.35) : lh.hovered ? Theme.tabHover : (langPop.isPicked(modelData[0]) ? Theme.mix(Theme.bg, Theme.accent, 0.25) : "transparent")
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
        // (A new line just begun: nothing to finish on it yet.)
        if (/\n[ \t]*$/.test(edit.text.substring(0, edit.cursorPosition))) { ghost = ""; lastGhostBase = ""; return }
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

    // (History open: a click anywhere outside it closes it, and still
    // does what it was for there.)
    MouseArea {
        parent: box.Window.window ? box.Window.window.contentItem : box
        anchors.fill: parent
        z: 10000
        enabled: compPopup.visible && compPopup.hist
        visible: enabled
        acceptedButtons: Qt.AllButtons
        onPressed: (m) => {
            m.accepted = false
            const p = mapToItem(compPopup, m.x, m.y)
            if (p.x >= 0 && p.y >= 0 && p.x < compPopup.width && p.y < compPopup.height) return
            box.closeComp()
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
        height: (hist ? searchRow.height + 6 : 0) + rows * rowH + 8 + (docBox.visible ? docBox.height + 10 : 0)
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
            x: 4; y: compPopup.hist ? searchRow.height + 10 : docBox.visible ? docBox.height + 14 : 4
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
                    // (History: colored as the editor would color it.)
                    text: compPopup.hist && modelData.spans ? box.coloredLine(modelData.label, modelData.spans) : modelData.label
                    textFormat: compPopup.hist && modelData.spans ? Text.StyledText : Text.PlainText
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
        // The chosen item's full help, over the list: its signature and
        // docstring, or a command's examples and man page (read in the
        // background for whichever is chosen), scrolled with the wheel or
        // Shift+PgUp/PgDn. One height for the whole list, so moving through
        // it doesn't make the panel grow and shrink.
        Rectangle {
            id: docBox
            readonly property var item: box.comp.items[box.compIndex]
            readonly property bool anyDoc: box.comp.items.some(i => (i.doc || "") !== "" || box.docFetchable(i))
            readonly property bool anyLong: anyDoc || box.comp.items.some(i => (i.detail || "").length > 60)
            readonly property string fetched: item && box.docFetchable(item) ? (box.fetchedDoc[box.docKey(item)] || "") : ""
            readonly property string text: !item ? "" : (item.doc || "") !== "" ? item.doc + (fetched !== "" ? "\n\n" + fetched : "")
                                          : box.docFetchable(item) ? (box.fetchedDoc[box.docKey(item)] !== undefined ? box.fetchedDoc[box.docKey(item)] || item.detail || "" : (item.detail || "") + (item.detail ? "\n\n" : "") + "…")
                                          : item.detail || ""
            visible: !compPopup.hist && anyLong
            x: 6; y: 6
            width: parent.width - 12
            height: visible ? (anyDoc ? 13 : 5) * Math.ceil(docFm.height) + 8 : 0
            radius: 6
            color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.04 : 0.02)
            FontMetrics { id: docFm; font.pixelSize: 12 }
            onTextChanged: docFlick.contentY = 0
            Flickable {
                id: docFlick
                anchors.fill: parent
                anchors.margins: 4
                anchors.topMargin: 4.001
                anchors.leftMargin: 6.001
                clip: true
                contentHeight: docText.implicitHeight
                boundsBehavior: Flickable.StopAtBounds
                interactive: false
                ScrollBar.vertical: ScrollBar { policy: docFlick.contentHeight > docFlick.height ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff }
                MomentumWheel { flick: docFlick }
                TextEdit {
                    id: docText
                    width: docFlick.width - 12
                    text: docBox.text
                    readOnly: true
                    selectByMouse: true
                    wrapMode: Text.Wrap
                    color: Theme.text
                    selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
                    // (The code font: signatures, examples and man pages line up.)
                    font.family: edit.font.family
                    font.pixelSize: 12
                    textFormat: TextEdit.PlainText
                    renderType: Text.NativeRendering
                }
            }
            Rectangle { anchors.top: parent.bottom; anchors.topMargin: 4; width: parent.width; height: 1; color: Theme.divider }
        }
        function scrollDoc(pages) { docFlick.contentY = Math.max(0, Math.min(docFlick.contentHeight - docFlick.height, docFlick.contentY + pages * (docFlick.height - 20))) }
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
