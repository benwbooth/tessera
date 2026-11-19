import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// A text file, read-only: line numbers, highlighted by its type, scrolling
// both ways (with momentum), its text selectable. A notebook's file card and
// the View overlay (FilePreview) both show files with it.
//
// Streamed, so a big file opens at once: its lines are counted in the
// background (files::line_index, the count growing as it goes, like less),
// and only the blocks of lines on screen (and near it) are read and
// highlighted, 500 lines a block.
Item {
    id: fv
    property var controller: null
    property string path: ""
    property string fontFamily: "monospace"
    property real pointSize: 10
    property var themeData: ({})
    // (How tall it would like to be: all its lines, up to `maxHeight`.)
    property real maxHeight: 460
    implicitHeight: inTerm ? maxHeight : editing ? Math.min(maxHeight, Math.max(exLine.kind !== "" || exLine.message.indexOf("\n") >= 0 ? 230 : 0, editBar.height + Math.max(3, editor.lineCount) * fm.lineSpacing + 14 + (exLine.visible ? exLine.height : 0)))
                  : lines === 0 ? 40 : Math.min(maxHeight, lines * fm.lineSpacing + 14)

    readonly property int block: 500
    // A scroll bar that shows while it scrolls by the wheel or keys too (they
    // move it themselves, not by a flick, which is all the plain one shows for).
    component ActiveBar: ScrollBar {
        id: sb
        property bool recent: false
        active: hovered || pressed || recent
        minimumSize: 0.05   // (a long file: still a handle to see and grab)
        onPositionChanged: { recent = true; sbHide.restart() }
        Timer { id: sbHide; interval: 1200; onTriggered: sb.recent = false }
    }
    function ansi(n, d) { const a = themeData.ansi || []; return a[n] || d }
    readonly property string name: path.replace(/^.*\//, "")
    readonly property string ext: name.indexOf(".") > 0 ? name.replace(/^.*\./, "").toLowerCase() : ""
    // The language to highlight it as: picked (langOverride; "-": none), else
    // the one its name says (tree-sitter's names).
    property string langOverride: ""
    readonly property string lang: langOverride === "-" ? "" : langOverride !== "" ? langOverride : detectedLang
    onLangChanged: { if (controller && lang !== "" && !controller.languageInstalled(lang)) controller.ensureLanguage(lang); rev++ }
    readonly property string detectedLang: ({
        rs: "rust", py: "python", js: "javascript", mjs: "javascript", cjs: "javascript", jsx: "javascript", ts: "typescript", tsx: "tsx",
        c: "c", h: "c", cpp: "cpp", cc: "cpp", cxx: "cpp", hpp: "cpp", hh: "cpp", go: "go", java: "java", kt: "kotlin", rb: "ruby", lua: "lua",
        sh: "bash", bash: "bash", zsh: "bash", fish: "fish", json: "json", yaml: "yaml", yml: "yaml", toml: "toml", html: "html", css: "css",
        md: "markdown", sql: "sql", nix: "nix", hs: "haskell", ml: "ocaml", scala: "scala", swift: "swift", zig: "zig", php: "php", pl: "perl",
        r: "r", jl: "julia", ex: "elixir", exs: "elixir", erl: "erlang", cs: "c-sharp", fs: "fsharp", dart: "dart", qml: "qmljs", xml: "xml",
        ini: "ini", cfg: "ini", conf: "ini", dockerfile: "dockerfile", make: "make", mk: "make", cmake: "cmake", tex: "latex", vim: "vim",
        nu: "nu", xsh: "python", ps1: "powershell", csv: "csv", diff: "diff", patch: "diff"
    })[ext] || ({ Makefile: "make", Dockerfile: "dockerfile", "CMakeLists.txt": "cmake", ".bashrc": "bash", ".zshrc": "bash", "flake.lock": "json" })[name] || ""

    // The lines counted so far, and whether that's all of them.
    property int lines: 0
    property bool counted: false
    property real size: 0
    property string problem: ""
    property int rev: 0                 // (read again: the card's ↻)
    readonly property bool languageReady: { fv.rev; return controller !== null && lang !== "" && controller.languageInstalled(lang) }
    onPathChanged: load()
    onControllerChanged: load()
    function load() {
        lines = 0
        counted = false
        size = 0
        problem = ""
        rev++
        if (!controller || path === "") return
        poll()
        if (lang !== "" && !controller.languageInstalled(lang)) controller.ensureLanguage(lang)
    }
    function reload() { if (editing) startEdit(); else load() }

    // ---- editing: the whole file in an editor, the input editor's keys ------
    property string editorKeys: "emacs"
    property bool editing: false
    readonly property bool viNormal: ek.viNormal
    property bool dirty: false
    property string savedText: ""       // (what's in the file: changed from it is unsaved)
    property bool crlf: false
    property bool savedCrlf: false
    property bool wrap: false
    function updateDirty() { dirty = editor.text !== savedText || crlf !== savedCrlf }
    onCrlfChanged: if (editing) updateDirty()
    property string editError: ""
    signal savedAs(string path)
    function startEdit() {
        editError = ""
        let r = {}
        try { r = JSON.parse(controller.fileReadForEdit(path)) } catch (e) {}
        if (r.error) { editError = r.error; return false }
        savedText = r.text || ""
        savedCrlf = !!r.crlf
        editor.text = savedText
        crlf = !!r.crlf
        dirty = false
        ek.viNormal = editorKeys === "vi"
        editing = true
        editor.cursorPosition = 0
        Qt.callLater(() => editor.forceActiveFocus())
        edHlTimer.restart()
        return true
    }
    function save() {
        if (!editing) return true
        const err = controller.fileSaveEdited(path, editor.text, crlf)
        editError = err
        if (err === "") { savedText = editor.text; savedCrlf = crlf; dirty = false; rev++ }
        return err === ""
    }
    function saveAs(p) {
        if (!editing || p === "") return false
        const err = controller.fileSaveEdited(p, editor.text, crlf)
        editError = err
        if (err !== "") return false
        savedText = editor.text
        savedCrlf = crlf
        dirty = false
        savedAs(p)
        return true
    }
    // ---- vi's command line: ":" commands, "/" and "?" searches ----------------
    function openCommandLine(kind, initial) { exLine.open(kind, initial) }
    function showExMessage(t, err) { exLine.show(t, err) }
    Connections {
        target: ek.ex
        function onWrite(p, force) {
            if (p === "") { if (fv.save()) fv.showExMessage("\"" + fv.name + "\" written", false) }
            else {
                const err = fv.controller.fileSaveEdited(p, editor.text, fv.crlf)
                fv.showExMessage(err === "" ? "\"" + p + "\" written" : err, err !== "")
            }
        }
        function onQuit(force) {
            if (fv.dirty && !force) { fv.showExMessage("E37: No write since last change (add ! to override)", true); return }
            fv.stopEdit()
        }
        function onEditFile(p, force) {
            if (fv.dirty && !force) { fv.showExMessage("E37: No write since last change (add ! to override)", true); return }
            if (p === "" || p === fv.path) { fv.reload(); return }
            fv.savedAs(p)                       // (the card shows that file now)
            Qt.callLater(fv.startEdit)
        }
        function onSetOption(name, value) {
            if (name === "wrap") fv.wrap = value === "toggle" ? !fv.wrap : value
            else if (name === "number" || name === "hlsearch" || name === "incsearch") {}
            else if (name === "list" || name === "readonly") fv.showExMessage("E518: Not here: " + name, true)
        }
    }
    Binding { target: ek.ex; property: "fileName"; value: fv.name }
    Binding { target: ek.ex; property: "folder"; value: fv.path.replace(/\/[^\/]*$/, "") }
    Component.onCompleted: ek.ex.readFile = (p) => {
        const r = JSON.parse(fv.controller.readTextFile(p === "" ? fv.path : p))
        if (r.error !== undefined) throw "E484: Can't open file " + (p || fv.path) + ": " + r.error
        return r.text
    }

    // ---- find, replace, go to a line ---------------------------------------
    // (Any case unless the search has a capital, as the terminal's find.)
    property var matches: []            // where the search is found (positions)
    property int matchAt: -1
    function findAll() {
        const q = findField.text
        if (q === "") { matches = []; matchAt = -1; return }
        const exact = q !== q.toLowerCase()
        const t = exact ? editor.text : editor.text.toLowerCase()
        const n = exact ? q : q.toLowerCase()
        const out = []
        for (let i = t.indexOf(n); i >= 0 && out.length < 100000; i = t.indexOf(n, i + Math.max(1, n.length))) out.push(i)
        matches = out
        matchAt = out.length ? Math.max(0, out.findIndex(x => x >= editor.cursorPosition - n.length)) : -1
        if (matchAt < 0 && out.length) matchAt = 0
    }
    function showMatch() {
        if (matchAt < 0 || matchAt >= matches.length) return
        const a = matches[matchAt]
        editor.select(a, a + findField.text.length)
        editFlick.ensureVisible(editor.positionToRectangle(a))
    }
    function findStep(dir) {
        if (matches.length === 0) findAll()
        if (matches.length === 0) return
        // (From the cursor: the next one after it, or the one before.)
        const p = dir > 0 ? editor.selectionEnd : editor.selectionStart
        let i = dir > 0 ? matches.findIndex(x => x >= p) : matches.length - 1 - matches.slice().reverse().findIndex(x => x < p)
        if (i < 0 || i >= matches.length) i = dir > 0 ? 0 : matches.length - 1
        matchAt = i
        showMatch()
    }
    function replaceOne() {
        const q = findField.text
        if (q === "") return
        const sel = editor.selectedText
        const same = q !== q.toLowerCase() ? sel === q : sel.toLowerCase() === q.toLowerCase()
        if (same) {
            const a = editor.selectionStart
            editor.remove(a, editor.selectionEnd)
            editor.insert(a, replaceField.text)
            editor.cursorPosition = a + replaceField.text.length
        }
        findAll()
        findStep(1)
    }
    function replaceAll() {
        const q = findField.text
        if (q === "") return
        findAll()
        // (From the end, so the positions before stay right.)
        for (let k = matches.length - 1; k >= 0; --k) {
            editor.remove(matches[k], matches[k] + q.length)
            editor.insert(matches[k], replaceField.text)
        }
        const n = matches.length
        findAll()
        editError = ""
        replaceNote = n + " replaced"
    }
    property string replaceNote: ""
    // The bar of these is hidden till wanted: Ctrl+F, H, G or the card's ⋯.
    property bool toolsShown: false
    readonly property bool canUndo: editor.canUndo
    readonly property bool canRedo: editor.canRedo
    function undo() { ek.ex.undo() }
    function redo() { ek.ex.redo() }
    function showFind() { toolsShown = true; findField.forceActiveFocus(); findField.selectAll() }
    function showReplace() { toolsShown = true; replaceField.forceActiveFocus(); replaceField.selectAll() }
    function showGoTo() { toolsShown = true; lineField.forceActiveFocus() }

    // ---- the file in a terminal editor, in a small terminal in the card ----
    // ("" is the shell's $VISUAL, else $EDITOR, else vi; the pick is kept.)
    property bool inTerm: false
    property string termCommand: ""
    // (Another host's file, "ssh://<host, URL-encoded>/<path>": the editor runs there.)
    readonly property var remote: path.match(/^ssh:\/\/([^\/]*)(\/.*)$/)
    readonly property string termHost: remote ? decodeURIComponent(remote[1]) : ""
    readonly property string termPath: remote ? remote[2] : path
    // (There, the terminal opens once the host is connected: until then not
    // ready, as a pane waits for its host.)
    property bool termHostUp: false
    property string termHostError: ""
    Connections {
        target: fv.controller
        enabled: fv.inTerm && fv.termHost !== "" && !fv.termHostUp
        function onHostConnected(h, err) {
            if (h !== fv.termHost) return
            if (err !== "") fv.termHostError = err
            else fv.termHostUp = true
        }
    }
    readonly property var termEditors: [["", "Your $EDITOR"], ["vim", "Vim"], ["nvim", "Neovim"], ["emacs -nw", "Emacs"], ["hx", "Helix"], ["micro", "micro"], ["nano", "nano"], ["kak", "Kakoune"]]
    property string termEditor: { if (!controller) return ""; controller.revision; try { return JSON.parse(controller.settings()).terminal_editor || "" } catch (e) { return "" } }
    function shq(x) { return /^[A-Za-z0-9_\/.~+:@-]+$/.test(x) ? x : "'" + x.replace(/'/g, "'\\''") + "'" }
    // Nixpkgs' package for each editor: one that isn't installed runs from
    // there (nix run nixpkgs#<it>), the first time fetched.
    readonly property var editorPackages: ({ vim: "vim", nvim: "neovim", emacs: "emacs-nox", hx: "helix", micro: "micro", nano: "nano", kak: "kakoune", vi: "vim" })
    // (A POSIX sh script, so it's the same whatever the user's shell is; the
    // file is its $1.)
    function editorScript(e) {
        if (e === "") return 'exec ${VISUAL:-${EDITOR:-vi}} "$1"'
        const words = e.split(" "), bin = words[0], args = words.slice(1).join(" ")
        const pkg = editorPackages[bin] || bin
        const a = args !== "" ? args + " " : ""
        return "if command -v " + bin + " >/dev/null 2>&1; then exec " + bin + " " + a + '"$1"; '
             + "elif command -v nix >/dev/null 2>&1; then echo '" + bin + " isn'\\''t installed: running it from nixpkgs (nix run nixpkgs#" + pkg + ")...'; "
             + "exec nix run nixpkgs#" + pkg + " -- " + a + '"$1"; '
             + "else echo '" + bin + " isn'\\''t installed, and there'\\''s no nix to run it with.'; printf 'Press Enter to close. '; read _; fi"
    }
    function openInTerminal(ed) {
        if (ed !== undefined && ed !== termEditor && controller) controller.setSetting("terminal_editor", JSON.stringify(ed))
        const e = ed !== undefined ? ed : termEditor
        if (editing && dirty) save()          // (what's typed here first, so it opens that)
        editing = false
        termHostUp = termHost === ""
        termHostError = ""
        if (!termHostUp && controller) controller.connectHost(termHost)
        termCommand = "exec sh -c " + shq(editorScript(e)) + " sh " + shq(termPath)
        inTerm = true
    }
    // (Quit the editor, or close it here: back to the file, as it is now.)
    function closeTerminal() {
        if (termLoader.item && !termLoader.item.exited) termLoader.item.killSession()
        inTerm = false
        load()
    }
    function goToLine(n) {
        const lines = editor.text.split("\n")
        n = Math.max(1, Math.min(lines.length, n))
        let p = 0
        for (let i = 0; i < n - 1; ++i) p += lines[i].length + 1
        editor.cursorPosition = p
        editor.forceActiveFocus()
    }
    Timer { id: findTimer; interval: 200; onTriggered: if (findField.text !== "") fv.findAll() }
    // The line numbers when lines wrap: each beside where its line starts.
    property string wrapNumbers: ""
    Timer {
        id: gutterTimer
        interval: 120
        onTriggered: {
            if (!fv.wrap || !fv.editing) return
            const lines = editor.text.split("\n")
            let s = "", p = 0, prevY = -1
            for (let i = 0; i < lines.length; ++i) {
                const y = editor.positionToRectangle(p).y
                if (prevY >= 0) { const extra = Math.round((y - prevY) / fm.lineSpacing) - 1; for (let k = 0; k < extra; ++k) s += "\n" }
                s += (i > 0 ? "\n" : "") + (i + 1)
                prevY = y
                p += lines[i].length + 1
            }
            fv.wrapNumbers = s
        }
    }
    onWrapChanged: gutterTimer.restart()

    // (Done: back to viewing, the file as it is now.)
    function stopEdit() { editing = false; editError = ""; load() }
    function poll() {
        let r = {}
        try { r = JSON.parse(controller.fileLineIndex(path)) } catch (e) {}
        if (r.error) { problem = r.error; counted = true; return }
        lines = r.lines || 0
        size = r.size || 0
        counted = !!r.done
        // (A NUL in the start: binary.)
        if (lines > 0 && problem === "" && controller.fileLines(path, 0, 50).indexOf("\u0000") >= 0) problem = "It's a binary file."
    }
    Timer { interval: 250; repeat: true; running: !fv.counted && fv.path !== ""; onTriggered: fv.poll() }
    Connections {
        target: fv.controller
        function onLanguageReady(name, error) { if (name === fv.lang && !error) fv.rev++ }
    }
    FontMetrics { id: fm; font.family: fv.fontFamily; font.pointSize: fv.pointSize }
    readonly property real gutterWidth: fm.advanceWidth("0") * String(Math.max(1, lines)).length + 16

    ListView {
        id: list
        visible: fv.problem === "" && !fv.editing && !fv.inTerm
        anchors.fill: parent
        clip: true
        model: Math.ceil(fv.lines / fv.block)
        cacheBuffer: 2000
        topMargin: 6
        bottomMargin: 6
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.HorizontalAndVerticalFlick
        // (Long lines: scrolled sideways, as wide as the widest block read.)
        property real widest: width
        contentWidth: widest
        ScrollBar.vertical: ActiveBar {}
        ScrollBar.horizontal: ActiveBar {}
        MomentumWheel { flick: list }
        // Paging: PgUp/PgDn a screen, Ctrl+Home/End the ends.
        Keys.onPressed: (e) => {
            const page = Math.max(fm.lineSpacing, list.height - fm.lineSpacing * 2)
            if (e.key === Qt.Key_PageDown) list.contentY = Math.min(list.contentY + page, list.contentHeight - list.height + list.originY)
            else if (e.key === Qt.Key_PageUp) list.contentY = Math.max(list.originY - list.topMargin, list.contentY - page)
            else if (e.key === Qt.Key_End && (e.modifiers & Qt.ControlModifier)) list.positionViewAtEnd()
            else if (e.key === Qt.Key_Home && (e.modifiers & Qt.ControlModifier)) list.positionViewAtBeginning()
            else return
            e.accepted = true
        }
        delegate: Item {
            id: blk
            required property int index
            readonly property int first: index * fv.block
            readonly property int count: Math.min(fv.block, fv.lines - first)
            // (Read when it comes near the screen; again when the file's read again
            // or the block fills out while the lines are still being counted.)
            readonly property string body: { fv.rev; return fv.controller ? fv.controller.fileLines(fv.path, first, count) : "" }
            width: Math.max(list.width, fv.gutterWidth + view.implicitWidth + 24)
            height: Math.max(count * fm.lineSpacing, view.contentHeight)
            onWidthChanged: if (width > list.widest) list.widest = width
            Rectangle {
                width: fv.gutterWidth
                height: parent.height
                color: Theme.mix(Theme.bg, Theme.fg, 0.03)
                Text {
                    x: 8.001
                    y: 0.001
                    width: fv.gutterWidth - 16
                    horizontalAlignment: Text.AlignRight
                    text: { let s = ""; for (let i = 0; i < blk.count; ++i) s += (i > 0 ? "\n" : "") + (blk.first + i + 1); return s }
                    color: Theme.textDim
                    font: view.font
                    renderType: Text.NativeRendering
                }
            }
            TextEdit {
                id: view
                x: fv.gutterWidth + 8.001
                y: 0.001
                readOnly: true
                selectByMouse: true
                text: blk.body
                textFormat: TextEdit.PlainText
                color: Theme.text
                selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
                selectedTextColor: Theme.text
                font.family: fv.fontFamily
                font.pointSize: fv.pointSize
                tabStopDistance: 4 * fm.advanceWidth(" ")
                renderType: Text.NativeRendering
                BlockSelect {}
                Keys.forwardTo: [list]
            }
            SyntaxHighlighter {
                id: hl
                document: view.textDocument
                errorColor: "transparent"
                colors: ({
                    keyword: fv.ansi(5, "#c678dd"), string: fv.ansi(2, "#98c379"), comment: Theme.textDim,
                    number: fv.ansi(3, "#d19a66"), constant: fv.ansi(3, "#d19a66"), function: fv.ansi(4, "#61afef"),
                    operator: fv.ansi(6, "#56b6c2"), property: fv.ansi(6, "#56b6c2"), variable: Theme.text,
                    embedded: fv.ansi(13, "#c678dd"), type: fv.ansi(3, "#e5c07b"), constructor: fv.ansi(3, "#e5c07b"),
                    punctuation: Theme.textDim, escape: fv.ansi(6, "#56b6c2"), attribute: fv.ansi(3, "#e5c07b"),
                    tag: fv.ansi(1, "#e06c75")
                })
            }
            // (Highlighted once read: a block on its own, so something spanning
            // two blocks may be colored as if it began at the second.)
            function highlight() {
                if (!fv.languageReady || body === "" || body.length > 500000) return
                try { const r = JSON.parse(fv.controller.syntax(fv.lang, body)); r.errors = []; hl.analysis = JSON.stringify(r) } catch (e) {}
            }
            onBodyChanged: Qt.callLater(highlight)
            Component.onCompleted: Qt.callLater(highlight)
            Connections { target: fv; function onLanguageReadyChanged() { Qt.callLater(blk.highlight) } }
        }
    }

    // ---- the editor's bar: undo, find and replace, go to a line, wrap, line ends ----
    Flow {
        id: editBar
        visible: fv.editing && fv.toolsShown && !fv.inTerm
        width: parent.width
        height: visible ? implicitHeight + 4 : 0
        spacing: 4
        topPadding: 2
        leftPadding: 4
        component BarField: TextField {
            height: 22
            font.pixelSize: 11
            color: Theme.text
            padding: 2
            leftPadding: 6
            background: Rectangle { radius: 4; color: Theme.bg; border.color: parent.activeFocus ? Theme.accent : Theme.divider }
            Keys.onEscapePressed: editor.forceActiveFocus()
        }
        Chip { base: Theme.panel; label: "↶"; tip: "Undo (Ctrl+Z)"; opacity: editor.canUndo ? 1 : 0.4; onClicked: fv.undo() }
        Chip { base: Theme.panel; label: "↷"; tip: "Redo (Ctrl+Shift+Z)"; opacity: editor.canRedo ? 1 : 0.4; onClicked: fv.redo() }
        BarField {
            id: findField
            width: 150
            placeholderText: "Find (Ctrl+F)"
            onTextChanged: { fv.replaceNote = ""; fv.findAll(); fv.showMatch() }
            onAccepted: fv.findStep(1)
            Keys.onPressed: (e) => { if (e.key === Qt.Key_F3 || ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter) && (e.modifiers & Qt.ShiftModifier))) { fv.findStep(e.modifiers & Qt.ShiftModifier ? -1 : 1); e.accepted = true } }
        }
        Chip { base: Theme.panel; label: "↑"; tip: "Previous (Shift+F3)"; onClicked: fv.findStep(-1) }
        Chip { base: Theme.panel; label: "↓"; tip: "Next (F3)"; onClicked: fv.findStep(1) }
        Text {
            visible: findField.text !== ""
            topPadding: 4.001
            text: fv.replaceNote !== "" ? fv.replaceNote : fv.matches.length === 0 ? "no matches" : (fv.matchAt + 1) + "/" + fv.matches.length
            color: fv.matches.length === 0 && fv.replaceNote === "" ? fv.ansi(1, "#e06c75") : Theme.textDim
            font.pixelSize: 11
            renderType: Text.NativeRendering
        }
        BarField { id: replaceField; width: 130; placeholderText: "Replace (Ctrl+H)"; onAccepted: fv.replaceOne() }
        Chip { base: Theme.panel; label: "Replace"; onClicked: fv.replaceOne() }
        Chip { base: Theme.panel; label: "All"; tip: "Replace every one"; onClicked: fv.replaceAll() }
        BarField {
            id: lineField
            width: 76
            placeholderText: "Line (Ctrl+G)"
            validator: IntValidator { bottom: 1 }
            onAccepted: { fv.goToLine(parseInt(text)); text = "" }
        }
        Chip { base: Theme.panel; label: fv.wrap ? "Wrap ✓" : "Wrap"; tip: "Wrap long lines"; onClicked: fv.wrap = !fv.wrap }
        Chip { base: Theme.panel; label: fv.crlf ? "CRLF" : "LF"; tip: "Line ends: " + (fv.crlf ? "Windows (CRLF)" : "Unix (LF)") + " — click to switch"; onClicked: fv.crlf = !fv.crlf }
        Chip { base: Theme.panel; label: "✕"; tip: "Hide these (they're in ⋯ too)"; onClicked: { fv.toolsShown = false; editor.forceActiveFocus() } }
    }

    Flickable {
        id: editFlick
        visible: fv.editing && !fv.inTerm
        anchors.fill: parent
        anchors.topMargin: editBar.height
        clip: true
        contentWidth: fv.wrap ? width : Math.max(width, fv.gutterWidth + editor.implicitWidth + 24)
        contentHeight: editor.contentHeight + 12
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ActiveBar {}
        ScrollBar.horizontal: ActiveBar {}
        MomentumWheel { flick: editFlick }
        function ensureVisible(r) {
            if (contentY > r.y) contentY = r.y
            else if (contentY + height < r.y + r.height + 12) contentY = r.y + r.height + 12 - height
            const x = r.x + fv.gutterWidth
            if (contentX > x - 8) contentX = Math.max(0, x - 8)
            else if (contentX + width < x + 16) contentX = x + 16 - width
        }
        Rectangle {
            width: fv.gutterWidth
            height: Math.max(editFlick.height, editFlick.contentHeight)
            color: Theme.mix(Theme.bg, Theme.fg, 0.03)
            Text {
                x: 8.001
                y: 6.001
                width: fv.gutterWidth - 16
                horizontalAlignment: Text.AlignRight
                // (By the file's lines, not the wrapped ones: wrapped, each
                // number beside where its line starts.)
                text: { if (fv.wrap) return fv.wrapNumbers; const n = editor.text.split("\n").length; let s = ""; for (let i = 1; i <= Math.max(1, n); ++i) s += (i > 1 ? "\n" : "") + i; return s }
                color: Theme.textDim
                font: editor.font
                renderType: Text.NativeRendering
            }
        }
        TextEdit {
            id: editor
            x: fv.gutterWidth + 8.001
            y: 6.001
            textFormat: TextEdit.PlainText
            wrapMode: fv.wrap ? TextEdit.Wrap : TextEdit.NoWrap
            width: fv.wrap ? editFlick.width - fv.gutterWidth - 16 : implicitWidth
            onWidthChanged: gutterTimer.restart()
            color: Theme.text
            selectByMouse: true
            persistentSelection: true
            selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
            selectedTextColor: Theme.text
            font.family: fv.fontFamily
            font.pointSize: fv.pointSize
            tabStopDistance: 4 * fm.advanceWidth(" ")
            renderType: Text.NativeRendering
            onTextChanged: if (fv.editing) { fv.updateDirty(); edHlTimer.restart(); findTimer.restart(); gutterTimer.restart() }
            onCursorRectangleChanged: editFlick.ensureVisible(cursorRectangle)
            // (Ctrl+S saves; the rest is the input editor's keys.)
            Keys.onPressed: (e) => {
                exLine.dismiss()       // (a key: what a command said goes)
                if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_S) { fv.save(); e.accepted = true; return }
                // (Undo and redo through the : commands' own, so one of theirs is one step.)
                if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_Z) { if (e.modifiers & Qt.ShiftModifier) ek.ex.redo(); else ek.ex.undo(); e.accepted = true; return }
                if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_Y) { ek.ex.redo(); e.accepted = true; return }
                // (Find, replace, a line: their fields in the bar; F3 the next match.)
                if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_F) { if (editor.selectedText !== "" && editor.selectedText.indexOf("\u2029") < 0) findField.text = editor.selectedText; fv.showFind(); e.accepted = true; return }
                if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_H) { fv.showReplace(); e.accepted = true; return }
                if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_G) { fv.showGoTo(); e.accepted = true; return }
                if (e.key === Qt.Key_F3) { fv.findStep(e.modifiers & Qt.ShiftModifier ? -1 : 1); e.accepted = true; return }
                // (A page of lines at a time; Shift selects as it goes.)
                if ((e.key === Qt.Key_PageDown || e.key === Qt.Key_PageUp) && !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier))) {
                    const n = Math.max(1, Math.floor(editFlick.height / fm.lineSpacing) - 1) * (e.key === Qt.Key_PageDown ? 1 : -1)
                    const r = editor.cursorRectangle
                    const y = Math.max(0, Math.min(editor.contentHeight - 1, r.y + r.height / 2 + n * fm.lineSpacing))
                    const to = editor.positionAt(r.x, y)
                    if (e.modifiers & Qt.ShiftModifier) editor.moveCursorSelection(to, TextEdit.SelectCharacters)
                    else editor.cursorPosition = to
                    editFlick.contentY = Math.max(0, Math.min(editFlick.contentHeight - editFlick.height, editFlick.contentY + n * fm.lineSpacing))
                    e.accepted = true
                    return
                }
                e.accepted = ek.handle(e)
            }
            // A block cursor, as the input editor's: the letter under it in
            // the background color (a hollow box when it isn't typing; a little
            // lighter in vi's insert mode).
            cursorDelegate: Rectangle {
                id: cur
                readonly property string under: { const c = editor.text[editor.cursorPosition]; return c === undefined || c === "\n" || c === "\u2029" ? "" : c }
                width: Math.max(2, fm.advanceWidth(under !== "" ? under : " "))
                height: fm.height
                color: editor.activeFocus ? Theme.text : "transparent"
                border.width: editor.activeFocus ? 0 : 1
                border.color: Theme.text
                opacity: ek.keys === "vi" && !ek.viNormal && editor.activeFocus ? 0.85 : 1
                Text {
                    anchors.fill: parent
                    text: cur.under
                    font: editor.font
                    color: Theme.bg
                    renderType: Text.NativeRendering
                    visible: editor.activeFocus
                }
            }
        }
    }
    EditKeys {
        id: ek
        edit: editor
        onCommandLine: (kind, initial) => fv.openCommandLine(kind, initial)
        onMessage: (t) => fv.showExMessage(t, true)
        keys: fv.editorKeys
        indentUnit: ["python", "rust", "c", "cpp", "java", "go", "haskell"].indexOf(fv.lang) >= 0 ? "    " : "  "
        commentPrefix: ({ rust: "// ", c: "// ", cpp: "// ", javascript: "// ", typescript: "// ", tsx: "// ", go: "// ", java: "// ", kotlin: "// ", qmljs: "// ", swift: "// ", zig: "// ", "c-sharp": "// ", dart: "// ", scala: "// ", php: "// ",
                         lua: "-- ", sql: "-- ", haskell: "-- ", vim: "\" ", latex: "% ", erlang: "% ", ini: "; " })[fv.lang] || "# "
    }
    SyntaxHighlighter {
        id: edHl
        document: editor.textDocument
        errorColor: "transparent"
        colors: ({
            keyword: fv.ansi(5, "#c678dd"), string: fv.ansi(2, "#98c379"), comment: Theme.textDim,
            number: fv.ansi(3, "#d19a66"), constant: fv.ansi(3, "#d19a66"), function: fv.ansi(4, "#61afef"),
            operator: fv.ansi(6, "#56b6c2"), property: fv.ansi(6, "#56b6c2"), variable: Theme.text,
            embedded: fv.ansi(13, "#c678dd"), type: fv.ansi(3, "#e5c07b"), constructor: fv.ansi(3, "#e5c07b"),
            punctuation: Theme.textDim, escape: fv.ansi(6, "#56b6c2"), attribute: fv.ansi(3, "#e5c07b"),
            tag: fv.ansi(1, "#e06c75")
        })
    }
    // (Highlighted again once typing pauses.)
    Timer {
        id: edHlTimer
        interval: 300
        onTriggered: {
            if (!fv.editing || !fv.languageReady || editor.text.length > 1000000) return
            try { const r = JSON.parse(fv.controller.syntax(fv.lang, editor.text)); r.errors = []; edHl.analysis = JSON.stringify(r) } catch (e) {}
        }
    }
    // vi's command line, at the bottom of the editor (ExLine.qml).
    ExLine {
        id: exLine
        visible: fv.editing && !fv.inTerm && (kind !== "" || message !== "")
        z: 6
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        ex: ek.ex
        font: editor.font
        errorColor: fv.ansi(1, "#e06c75")
        maxHeight: fv.height * 0.6
        onClosed: editor.forceActiveFocus()
    }

    // vi's mode, and what went wrong saving.
    Text {
        visible: fv.editing && fv.editError !== ""
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 6
        text: fv.editError !== "" ? fv.editError : ek.viNormal ? "NORMAL" : "INSERT"
        color: fv.editError !== "" ? fv.ansi(1, "#e06c75") : Theme.textDim
        font.pixelSize: 11
        font.bold: true
        renderType: Text.NativeRendering
    }

    // Nothing to show: why.
    Text {
        visible: !fv.inTerm && (fv.problem !== "" || (fv.counted && fv.lines === 0))
        anchors.centerIn: parent
        text: fv.problem !== "" ? fv.problem : "(empty)"
        color: Theme.textDim
        font.pixelSize: 12
        renderType: Text.NativeRendering
    }
    // (Still counting a big one: how far, at the bottom right.)
    Text {
        visible: !fv.inTerm && !fv.counted && fv.lines > 0
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 6
        text: fv.lines.toLocaleString(Qt.locale(), "f", 0) + " lines so far…"
        color: Theme.textDim
        font.pixelSize: 11
        renderType: Text.NativeRendering
    }

    Loader {
        id: termLoader
        active: fv.inTerm
        anchors.fill: parent
        sourceComponent: TerminalView {
            command: fv.termCommand
            host: fv.termHost
            ready: fv.termHostUp
            cwd: fv.termPath.replace(/\/[^\/]*$/, "") || "/"
            fontFamily: fv.fontFamily
            fontPointSize: fv.pointSize
            themeJson: JSON.stringify(fv.themeData)
            focus: true
            Component.onCompleted: forceActiveFocus()
            onExitedChanged: if (exited) Qt.callLater(fv.closeTerminal)
        }
    }
    Text {
        visible: fv.inTerm && !fv.termHostUp
        anchors.centerIn: parent
        text: fv.termHostError !== "" ? "Can't reach " + fv.termHost + ": " + fv.termHostError : "Connecting to " + fv.termHost + "\u2026"
        color: fv.termHostError !== "" ? fv.ansi(1, "#e06c75") : Theme.textDim
        font.pixelSize: 12
        topPadding: 0.001
        renderType: Text.NativeRendering
    }
    Component.onDestruction: if (termLoader.item && !termLoader.item.exited) termLoader.item.killSession()
    // (Test hooks, from the card showing: TESSERA_TEST_TERM_EDIT=<editor or "-"> opens it in one at
    // 2 s; TESSERA_TEST_EDIT_FIND=<text> finds it at 4.5 s, while editing.)
    Timer { interval: 2000; running: fv.controller !== null && fv.path !== "" && fv.controller.testEnv("TERM_EDIT") !== ""
            onTriggered: { const e = fv.controller.testEnv("TERM_EDIT"); fv.openInTerminal(e === "-" ? undefined : e) } }
    Timer { interval: 4500; running: fv.controller !== null && fv.path !== "" && fv.controller.testEnv("EDIT_FIND") !== ""
            onTriggered: { fv.showFind(); findField.text = fv.controller.testEnv("EDIT_FIND"); fv.findStep(1); console.log("find test: " + (fv.matchAt + 1) + "/" + fv.matches.length) } }
    // (Test hook: TESSERA_TEST_FILE_WHEEL: a wheel turn over the file at 3 s.)
    Timer { interval: 3000; running: fv.controller !== null && fv.path !== "" && fv.controller.testEnv("FILE_WHEEL") !== ""
            onTriggered: { const p = fv.mapToItem(null, fv.width / 2, fv.height / 2); for (let i = 0; i < 3; i++) fv.controller.testWheel(p.x, p.y, -120, 0, 0); wheelLog.start() } }
    Timer { id: wheelLog; interval: 300; onTriggered: { const b = list.ScrollBar.vertical; console.log("file wheel test: contentY " + list.contentY + " bar active " + b.active + " handle " + (b.contentItem ? b.contentItem.opacity : "-")) } }
    // (Test hook: TESSERA_TEST_EX="cmd;;cmd;;..." runs those : commands 5 s
    // after the card shows (while editing), logs what each said, then opens
    // the command line with TESSERA_TEST_EX_OPEN typed in it.)
    Timer { interval: 5000; running: fv.controller !== null && fv.path !== "" && fv.controller.testEnv("EX") !== ""
            onTriggered: {
                for (const c of fv.controller.testEnv("EX").split(";;")) { const out = ek.ex.run(c); console.log("ex test: :" + c + " -> " + JSON.stringify(out) + (ek.ex.error ? " (error)" : "") + "  now " + JSON.stringify(editor.text)) }
                console.log("ex test: text " + JSON.stringify(editor.text))
                if (fv.controller.testEnv("EX_OPEN") !== "") fv.openCommandLine(":", fv.controller.testEnv("EX_OPEN"))
            } }
}
