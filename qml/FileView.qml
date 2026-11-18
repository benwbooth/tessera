import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// A text file, read-only: line numbers, highlighted by its type, scrolling
// both ways (with momentum), its text selectable. A notebook's file card and
// the View overlay (FilePreview) both show files with it.
Item {
    id: fv
    property var controller: null
    property string path: ""
    property string fontFamily: "monospace"
    property real pointSize: 10
    property var themeData: ({})
    // (How tall it would like to be: all its lines, up to `maxHeight`.)
    property real maxHeight: 460
    implicitHeight: problem !== "" && text === "" ? 40 : Math.min(maxHeight, view.contentHeight + 12)

    function ansi(n, d) { const a = themeData.ansi || []; return a[n] || d }
    readonly property string name: path.replace(/^.*\//, "")
    readonly property string ext: name.indexOf(".") > 0 ? name.replace(/^.*\./, "").toLowerCase() : ""
    // The language a file's name says (tree-sitter's names).
    readonly property string lang: ({
        rs: "rust", py: "python", js: "javascript", mjs: "javascript", cjs: "javascript", jsx: "javascript", ts: "typescript", tsx: "tsx",
        c: "c", h: "c", cpp: "cpp", cc: "cpp", cxx: "cpp", hpp: "cpp", hh: "cpp", go: "go", java: "java", kt: "kotlin", rb: "ruby", lua: "lua",
        sh: "bash", bash: "bash", zsh: "bash", fish: "fish", json: "json", yaml: "yaml", yml: "yaml", toml: "toml", html: "html", css: "css",
        md: "markdown", sql: "sql", nix: "nix", hs: "haskell", ml: "ocaml", scala: "scala", swift: "swift", zig: "zig", php: "php", pl: "perl",
        r: "r", jl: "julia", ex: "elixir", exs: "elixir", erl: "erlang", cs: "c-sharp", fs: "fsharp", dart: "dart", qml: "qmljs", xml: "xml",
        ini: "ini", cfg: "ini", conf: "ini", dockerfile: "dockerfile", make: "make", mk: "make", cmake: "cmake", tex: "latex", vim: "vim",
        nu: "nu", xsh: "python", ps1: "powershell", csv: "csv", diff: "diff", patch: "diff"
    })[ext] || ({ Makefile: "make", Dockerfile: "dockerfile", "CMakeLists.txt": "cmake", ".bashrc": "bash", ".zshrc": "bash", "flake.lock": "json" })[name] || ""

    // The text (the first 3 MB of a big file), or what's wrong.
    property string text: ""
    property string problem: ""
    property int lines: 0
    onPathChanged: load()
    onControllerChanged: load()
    function load() {
        text = ""
        problem = ""
        lines = 0
        if (!controller || path === "") return
        let r = {}
        try { r = JSON.parse(controller.readTextFile(path)) } catch (e) {}
        if (r.error) { problem = r.error; return }
        const t = r.text || ""
        // (A NUL byte, or much that isn't text: binary.)
        if (t.indexOf("\u0000") >= 0 || (t.match(/�/g) || []).length > t.length / 20) { problem = "It's a binary file."; return }
        text = t.length > 3000000 ? t.substring(0, 3000000) : t
        if (t.length > 3000000) problem = "Only the first 3 MB are shown."
        lines = text === "" ? 0 : text.split("\n").length - (text.endsWith("\n") ? 1 : 0)
        Qt.callLater(highlight)
    }
    // (Read again: the card's ↻.)
    function reload() { load() }

    Flickable {
        id: flick
        anchors.fill: parent
        clip: true
        contentWidth: Math.max(width, gutter.width + view.implicitWidth + 24)
        contentHeight: view.contentHeight + 12
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        ScrollBar.horizontal: ScrollBar {}
        MomentumWheel { flick: flick }
        // Line numbers, the same font and lines as the text.
        Rectangle {
            id: gutter
            width: numbers.implicitWidth + 16
            height: Math.max(flick.height, flick.contentHeight)
            color: Theme.mix(Theme.bg, Theme.fg, 0.03)
            Text {
                id: numbers
                x: 8.001
                y: 6.001
                text: { let s = ""; for (let i = 1; i <= Math.max(1, fv.lines); ++i) s += (i > 1 ? "\n" : "") + i; return s }
                horizontalAlignment: Text.AlignRight
                width: Math.max(implicitWidth, fm.advanceWidth("0") * String(Math.max(1, fv.lines)).length)
                color: Theme.textDim
                font: view.font
                renderType: Text.NativeRendering
            }
        }
        TextEdit {
            id: view
            x: gutter.width + 8.001
            y: 6.001
            readOnly: true
            selectByMouse: true
            text: fv.text
            textFormat: TextEdit.PlainText
            color: Theme.text
            selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
            selectedTextColor: Theme.text
            font.family: fv.fontFamily
            font.pointSize: fv.pointSize
            tabStopDistance: 4 * fm.advanceWidth(" ")
            renderType: Text.NativeRendering
            FontMetrics { id: fm; font: view.font }
            BlockSelect {}
            // Paging, once it's clicked into: PgUp/PgDn a screen, Home/End the ends.
            Keys.onPressed: (e) => {
                const page = Math.max(fm.height, flick.height - fm.height * 2)
                const top = flick.contentHeight - flick.height
                if (e.key === Qt.Key_PageDown) flick.contentY = Math.min(Math.max(0, top), flick.contentY + page)
                else if (e.key === Qt.Key_PageUp) flick.contentY = Math.max(0, flick.contentY - page)
                else if (e.key === Qt.Key_End && (e.modifiers & Qt.ControlModifier)) flick.contentY = Math.max(0, top)
                else if (e.key === Qt.Key_Home && (e.modifiers & Qt.ControlModifier)) flick.contentY = 0
                else return
                e.accepted = true
            }
        }
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
    function highlight() {
        if (!controller || lang === "" || text === "" || text.length > 500000) return
        if (!controller.languageInstalled(lang)) { controller.ensureLanguage(lang); return }
        try { const r = JSON.parse(controller.syntax(lang, text)); r.errors = []; hl.analysis = JSON.stringify(r) } catch (e) {}
    }
    Connections {
        target: fv.controller
        function onLanguageReady(name, error) { if (name === fv.lang && !error) fv.highlight() }
    }

    // Nothing to show: why.
    Text {
        visible: fv.text === "" && fv.path !== ""
        anchors.centerIn: parent
        text: fv.problem !== "" ? fv.problem : "(empty)"
        color: Theme.textDim
        font.pixelSize: 12
        renderType: Text.NativeRendering
    }
    // (Too big: only the start, said at the bottom right.)
    Text {
        visible: fv.problem !== "" && fv.text !== ""
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 6
        text: fv.problem
        color: fv.ansi(3, "#e5c07b")
        font.pixelSize: 11
        renderType: Text.NativeRendering
    }
}
