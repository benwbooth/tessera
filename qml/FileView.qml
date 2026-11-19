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
    implicitHeight: editing ? Math.min(maxHeight, Math.max(3, editor.lineCount) * fm.lineSpacing + 14)
                  : lines === 0 ? 40 : Math.min(maxHeight, lines * fm.lineSpacing + 14)

    readonly property int block: 500
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
    property bool dirty: false
    property bool crlf: false
    property string editError: ""
    signal savedAs(string path)
    function startEdit() {
        editError = ""
        let r = {}
        try { r = JSON.parse(controller.fileReadForEdit(path)) } catch (e) {}
        if (r.error) { editError = r.error; return false }
        editor.text = r.text || ""
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
        if (err === "") { dirty = false; rev++ }
        return err === ""
    }
    function saveAs(p) {
        if (!editing || p === "") return false
        const err = controller.fileSaveEdited(p, editor.text, crlf)
        editError = err
        if (err !== "") return false
        dirty = false
        savedAs(p)
        return true
    }
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
        visible: fv.problem === "" && !fv.editing
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
        ScrollBar.vertical: ScrollBar {}
        ScrollBar.horizontal: ScrollBar {}
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

    Flickable {
        id: editFlick
        visible: fv.editing
        anchors.fill: parent
        clip: true
        contentWidth: Math.max(width, fv.gutterWidth + editor.implicitWidth + 24)
        contentHeight: editor.contentHeight + 12
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        ScrollBar.horizontal: ScrollBar {}
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
                text: { let s = ""; for (let i = 1; i <= Math.max(1, editor.lineCount); ++i) s += (i > 1 ? "\n" : "") + i; return s }
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
            color: Theme.text
            selectByMouse: true
            persistentSelection: true
            selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
            selectedTextColor: Theme.text
            font.family: fv.fontFamily
            font.pointSize: fv.pointSize
            tabStopDistance: 4 * fm.advanceWidth(" ")
            renderType: Text.NativeRendering
            onTextChanged: if (fv.editing) { fv.dirty = true; edHlTimer.restart() }
            onCursorRectangleChanged: editFlick.ensureVisible(cursorRectangle)
            // (Ctrl+S saves; the rest is the input editor's keys.)
            Keys.onPressed: (e) => {
                if ((e.modifiers & Qt.ControlModifier) && e.key === Qt.Key_S) { fv.save(); e.accepted = true; return }
                e.accepted = ek.handle(e)
            }
            // A block cursor in vi's normal mode, a bar otherwise.
            cursorDelegate: Rectangle {
                width: ek.keys === "vi" && ek.viNormal ? Math.max(2, fm.advanceWidth("m")) : 2
                height: fm.height
                color: Theme.accent
                opacity: ek.keys === "vi" && ek.viNormal ? 0.55 : 1
            }
        }
    }
    EditKeys {
        id: ek
        edit: editor
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
    // vi's mode, and what went wrong saving.
    Text {
        visible: fv.editing && (fv.editError !== "" || ek.keys === "vi")
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
        visible: fv.problem !== "" || (fv.counted && fv.lines === 0)
        anchors.centerIn: parent
        text: fv.problem !== "" ? fv.problem : "(empty)"
        color: Theme.textDim
        font.pixelSize: 12
        renderType: Text.NativeRendering
    }
    // (Still counting a big one: how far, at the bottom right.)
    Text {
        visible: !fv.counted && fv.lines > 0
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 6
        text: fv.lines.toLocaleString(Qt.locale(), "f", 0) + " lines so far…"
        color: Theme.textDim
        font.pixelSize: 11
        renderType: Text.NativeRendering
    }
}
