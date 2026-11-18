import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// A file, shown over the pane (View, from a file listing's right click):
// an image fitted (a click: its own size), Markdown, JSON, tables, HTML and
// SVG in the rich viewer, code and text highlighted, read-only. Esc (or ×)
// closes it.
Rectangle {
    id: pv
    property var controller: null
    property string path: ""
    property string fontFamily: "monospace"
    property real pointSize: 10
    property var themeData: ({})
    signal closed()
    signal insertRequested(string text)
    color: Theme.bg
    focus: visible
    Keys.onEscapePressed: closed()

    function ansi(n, d) { const a = themeData.ansi || []; return a[n] || d }
    readonly property string name: path.replace(/^.*\//, "")
    readonly property string ext: name.indexOf(".") > 0 ? name.replace(/^.*\./, "").toLowerCase() : ""
    readonly property bool isImage: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tif", "tiff", "avif"].indexOf(ext) >= 0
    readonly property string richKind: ({ md: "markdown", markdown: "markdown", json: "json", csv: "csv", tsv: "tsv", html: "html", htm: "html", svg: "svg" })[ext] || ""
    // The language a file's name says (tree-sitter's names).
    readonly property string lang: ({
        rs: "rust", py: "python", js: "javascript", mjs: "javascript", cjs: "javascript", jsx: "javascript", ts: "typescript", tsx: "tsx",
        c: "c", h: "c", cpp: "cpp", cc: "cpp", cxx: "cpp", hpp: "cpp", hh: "cpp", go: "go", java: "java", kt: "kotlin", rb: "ruby", lua: "lua",
        sh: "bash", bash: "bash", zsh: "bash", fish: "fish", json: "json", yaml: "yaml", yml: "yaml", toml: "toml", html: "html", css: "css",
        md: "markdown", sql: "sql", nix: "nix", hs: "haskell", ml: "ocaml", scala: "scala", swift: "swift", zig: "zig", php: "php", pl: "perl",
        r: "r", jl: "julia", ex: "elixir", exs: "elixir", erl: "erlang", cs: "c-sharp", fs: "fsharp", dart: "dart", qml: "qmljs", xml: "xml",
        ini: "ini", cfg: "ini", conf: "ini", dockerfile: "dockerfile", make: "make", mk: "make", cmake: "cmake", tex: "latex", vim: "vim"
    })[ext] || ({ Makefile: "make", Dockerfile: "dockerfile", "CMakeLists.txt": "cmake", ".bashrc": "bash", ".zshrc": "bash" })[name] || ""

    // The text, unless it's an image or too big or binary.
    property string text: ""
    property string problem: ""
    onPathChanged: load()
    function load() {
        text = ""
        problem = ""
        if (!controller || path === "" || isImage) return
        let r = {}
        try { r = JSON.parse(controller.readTextFile(path)) } catch (e) {}
        if (r.error) { problem = r.error; return }
        const t = r.text || ""
        // (A NUL byte, or much that isn't text: binary.)
        if (t.indexOf("\u0000") >= 0 || (t.match(/�/g) || []).length > t.length / 20) { problem = "It's a binary file."; return }
        text = t.length > 3000000 ? t.substring(0, 3000000) : t
        if (t.length > 3000000) problem = "Only the first 3 MB are shown."
        Qt.callLater(highlight)
    }

    // ---- the bar: what it is, and what to do with it ----------------------
    Rectangle {
        id: head
        width: parent.width
        height: 30
        color: Theme.panel
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.divider }
        Row {
            anchors.left: parent.left
            anchors.leftMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8
            Text { text: pv.name; color: Theme.text; font.pixelSize: 12; font.weight: Font.DemiBold; anchors.verticalCenter: parent.verticalCenter; anchors.verticalCenterOffset: 0.001 }
            Text { text: pv.path.replace(/\/[^/]*$/, ""); color: Theme.textDim; font.pixelSize: 11; elide: Text.ElideMiddle; width: Math.min(implicitWidth, pv.width * 0.4); anchors.verticalCenter: parent.verticalCenter; anchors.verticalCenterOffset: 0.001 }
            Text { visible: pv.problem !== "" && pv.text !== ""; text: pv.problem; color: pv.ansi(3, "#e5c07b"); font.pixelSize: 11; anchors.verticalCenter: parent.verticalCenter; anchors.verticalCenterOffset: 0.001 }
        }
        Row {
            anchors.right: parent.right
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6
            Chip { base: Theme.panel; label: "Open with its app"; onClicked: pv.controller.fileAction("open", pv.path, "") }
            Chip { base: Theme.panel; label: "Insert the path"; onClicked: pv.insertRequested((/^[A-Za-z0-9_\/.~+:@-]+$/.test(pv.path) ? pv.path : "'" + pv.path.replace(/'/g, "'\\''") + "'") + " ") }
            Chip { base: Theme.panel; label: "×"; tip: "Close (Esc)"; onClicked: pv.closed() }
        }
    }

    // ---- an image ------------------------------------------------------------
    Flickable {
        id: imgFlick
        visible: pv.isImage
        anchors.fill: parent
        anchors.topMargin: head.height
        clip: true
        contentWidth: Math.max(width, img.width)
        contentHeight: Math.max(height, img.height)
        property bool actual: false
        ScrollBar.vertical: ScrollBar {}
        ScrollBar.horizontal: ScrollBar {}
        MomentumWheel { flick: imgFlick }
        Image {
            id: img
            source: pv.isImage && pv.path !== "" ? "file://" + pv.path : ""
            asynchronous: true
            fillMode: Image.PreserveAspectFit
            readonly property real fit: Math.min(1, Math.min(imgFlick.width / Math.max(1, implicitWidth), imgFlick.height / Math.max(1, implicitHeight)))
            width: implicitWidth * (imgFlick.actual ? 1 : fit)
            height: implicitHeight * (imgFlick.actual ? 1 : fit)
            x: Math.max(0, (imgFlick.width - width) / 2)
            y: Math.max(0, (imgFlick.height - height) / 2)
            TapHandler { onTapped: imgFlick.actual = !imgFlick.actual }
            HoverHandler { cursorShape: Qt.PointingHandCursor }
        }
        Text {
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            anchors.margins: 8
            text: img.status === Image.Ready ? img.implicitWidth + " × " + img.implicitHeight + (imgFlick.actual ? "  (actual size)" : "  (fitted; click for actual size)") : img.status === Image.Error ? "Couldn't read the image." : ""
            color: Theme.textDim
            font.pixelSize: 11
        }
    }

    // ---- Markdown, JSON, tables, HTML, SVG --------------------------------
    RichViewer {
        visible: !pv.isImage && pv.richKind !== "" && pv.text !== ""
        anchors.fill: parent
        anchors.topMargin: head.height
        kind: pv.richKind
        text: pv.text
        fontFamily: pv.fontFamily
        themeData: pv.themeData
        onClosed: pv.closed()
    }

    // ---- code and text: highlighted, read-only ----------------------------
    Flickable {
        id: textFlick
        visible: !pv.isImage && pv.richKind === "" && pv.text !== ""
        anchors.fill: parent
        anchors.topMargin: head.height
        anchors.leftMargin: 10
        clip: true
        contentWidth: Math.max(width, view.implicitWidth + 20)
        contentHeight: view.implicitHeight + 16
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        ScrollBar.horizontal: ScrollBar {}
        MomentumWheel { flick: textFlick }
        TextEdit {
            id: view
            y: 8
            readOnly: true
            selectByMouse: true
            text: pv.text
            textFormat: TextEdit.PlainText
            color: Theme.text
            selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
            selectedTextColor: Theme.text
            font.family: pv.fontFamily
            font.pointSize: pv.pointSize
            tabStopDistance: 4 * fm.advanceWidth(" ")
            renderType: Text.NativeRendering
            FontMetrics { id: fm; font: view.font }
            BlockSelect {}
        }
    }
    SyntaxHighlighter {
        id: hl
        document: view.textDocument
        errorColor: "transparent"
        colors: ({
            keyword: pv.ansi(5, "#c678dd"), string: pv.ansi(2, "#98c379"), comment: Theme.textDim,
            number: pv.ansi(3, "#d19a66"), constant: pv.ansi(3, "#d19a66"), function: pv.ansi(4, "#61afef"),
            operator: pv.ansi(6, "#56b6c2"), property: pv.ansi(6, "#56b6c2"), variable: Theme.text,
            embedded: pv.ansi(13, "#c678dd"), type: pv.ansi(3, "#e5c07b"), constructor: pv.ansi(3, "#e5c07b"),
            punctuation: Theme.textDim, escape: pv.ansi(6, "#56b6c2"), attribute: pv.ansi(3, "#e5c07b"),
            tag: pv.ansi(1, "#e06c75")
        })
    }
    function highlight() {
        if (!controller || lang === "" || text === "" || text.length > 500000) return
        if (!controller.languageInstalled(lang)) { controller.ensureLanguage(lang); return }
        try { const r = JSON.parse(controller.syntax(lang, text)); r.errors = []; hl.analysis = JSON.stringify(r) } catch (e) {}
    }
    Connections {
        target: pv.controller
        function onLanguageReady(name, error) { if (name === pv.lang && !error) pv.highlight() }
    }

    // ---- nothing to show ------------------------------------------------------
    Column {
        visible: !pv.isImage && pv.text === "" && pv.path !== ""
        anchors.centerIn: parent
        spacing: 10
        Text { anchors.horizontalCenter: parent.horizontalCenter; text: pv.problem !== "" ? pv.problem : "(empty)"; color: Theme.textDim; font.pixelSize: 13 }
        Chip { anchors.horizontalCenter: parent.horizontalCenter; visible: pv.problem !== ""; base: Theme.bg; label: "Open with its app"; onClicked: pv.controller.fileAction("open", pv.path, "") }
    }
}
