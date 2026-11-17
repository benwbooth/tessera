import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import QtQuick.Dialogs
import Tessera

// The pane as a notebook (like Jupyter): each command a cell, its output in
// a pane of its own, shown for what it is — terminal text with its colors,
// JSON, a table, HTML (a real browser view), Markdown, SVG, a picture. The
// kind is recognised (rules, then the AI for the rest) and can be changed
// per cell from its dropdown.
Rectangle {
    id: nb
    property var view: null            // the TerminalView the cells come from
    property var controller: null
    property var themeData: ({})
    property string fontFamily: "monospace"
    signal rerun(string command)
    signal openNotebook(string path)
    property string markdownStyle: "rich"  // Markdown cells by default: "rich" or "text"
    signal markdownStylePicked(string style)

    color: Theme.bg

    // ---- cells ------------------------------------------------------------------
    property var cells: []
    property var kinds: ({})           // id -> chosen view (the dropdown)
    property var aiKinds: ({})         // id -> what the AI made of it
    property var heights: ({})         // id -> output height the user dragged to
    property int expandedId: -1        // the cell filling the window, if any
    function setHeight(id, h) { const m = Object.assign({}, heights); if (h > 0) m[id] = h; else delete m[id]; heights = m }
    function reload() {
        if (!view) return
        try { cells = JSON.parse(view.cells()) } catch (e) { cells = [] }
        Qt.callLater(list.positionViewAtEnd)
    }
    Connections {
        target: nb.view
        function onCellsChanged(id, done) {
            nb.reload()
            if (done) nb.classify(id)
        }
    }
    Component.onCompleted: reload()
    onVisibleChanged: if (visible) reload()
    // While a command runs, its cell's output is read again now and then.
    property int tick: 0
    Timer { interval: 400; repeat: true; running: nb.visible && nb.cells.length > 0 && !nb.cells[nb.cells.length - 1].done; onTriggered: nb.tick++ }

    // The rules found nothing: ask the AI (the suggestions model) what it is.
    readonly property string aiKey: "nb-" + Math.random().toString(36).slice(2)
    function classify(id) {
        const c = cells.find(x => x.id === id)
        if (!c || c.kind !== "" || c.size < 80 || !controller || !controller.aiReady("inline")) return
        const sample = view.cellOutput(id, "text").slice(0, 2000)
        controller.aiRun(aiKey + "-" + id, "classify", JSON.stringify([{ role: "user", content: sample }]), JSON.stringify({ cwd: c.cwd }))
    }
    Connections {
        target: nb.controller
        function onAiDone(key, reply, error) {
            if (!key.startsWith(nb.aiKey + "-") || error) return
            const id = parseInt(key.slice(nb.aiKey.length + 1))
            const k = reply.trim().toLowerCase().replace(/[^a-z]/g, "")
            if (["json", "csv", "tsv", "html", "markdown", "svg", "diff", "yaml", "xml"].indexOf(k) < 0) return
            const a = Object.assign({}, nb.aiKinds); a[id] = k; nb.aiKinds = a
        }
    }

    readonly property var viewNames: [["auto", "Auto"], ["terminal", "Terminal"], ["text", "Plain text"], ["json", "JSON"], ["csv", "Table (CSV)"], ["tsv", "Table (TSV)"],
                                      ["html", "HTML"], ["markdown", "Markdown (rich)"], ["mdtext", "Markdown (text)"], ["svg", "SVG"], ["diff", "Diff"], ["image", "Image"]]
    function shownKind(c) {
        const chosen = kinds[c.id]
        if (chosen && chosen !== "auto") return chosen
        const k = c.kind !== "" ? c.kind : aiKinds[c.id] || "terminal"
        return k === "markdown" && markdownStyle === "text" ? "mdtext" : k
    }
    // The terminal's palette colors in a cell's HTML (var(--vt-palette-N)).
    function paletteColor(n) {
        const a = themeData.ansi || []
        if (n < 16) return a[n] || ["#000000", "#cd3131", "#0dbc79", "#e5e510", "#2472c8", "#bc3fbc", "#11a8cd", "#e5e5e5"][n % 8]
        if (n < 232) {
            const v = (x) => x === 0 ? 0 : 55 + x * 40, i = n - 16
            return Qt.rgba(v(Math.floor(i / 36)) / 255, v(Math.floor(i / 6) % 6) / 255, v(i % 6) / 255, 1).toString()
        }
        const g = (8 + (n - 232) * 10) / 255
        return Qt.rgba(g, g, g, 1).toString()
    }
    function palette(html) { return html.replace(/var\(--vt-palette-(\d+)\)/g, (m, n) => paletteColor(parseInt(n))) }
    function shortDir(p) {
        const home = controller ? controller.homeDir() : ""
        return home !== "" && p.startsWith(home) ? "~" + p.slice(home.length) : p
    }
    function seconds(ms) { return ms < 1000 ? ms + " ms" : ms < 60000 ? (ms / 1000).toFixed(1) + " s" : Math.floor(ms / 60000) + " m " + Math.round((ms % 60000) / 1000) + " s" }

    ListView {
        id: list
        anchors.fill: parent
        anchors.margins: 10
        anchors.topMargin: nb.cells.length > 0 ? 34 : 10
        spacing: 12
        model: nb.cells
        clip: true
        interactive: false   // (a drag selects text; the wheel, touchpad and bar scroll)
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        MomentumWheel { flick: list }

        delegate: Rectangle {
            id: cell
            required property var modelData
            required property int index
            readonly property var c: modelData
            readonly property string kind: nb.shownKind(c)
            property bool collapsed: false
            width: list.width - 12
            readonly property bool expanded: nb.expandedId === c.id
            height: header.height + (collapsed ? 0 : body.height + 12)
            radius: 8
            color: Theme.mix(Theme.bg, Theme.fg, 0.035)
            border.color: Theme.divider

            // ---- header: status, command, folder, time, view ----
            RowLayout {
                id: header
                x: 10
                width: parent.width - 20
                height: 26
                spacing: 6
                Text {
                    text: !cell.c.done ? "◌" : cell.c.status === 0 ? "✓" : "✘"
                    color: !cell.c.done ? Theme.accent : cell.c.status === 0 ? nb.themeData.ansi ? nb.themeData.ansi[2] : "#98c379" : nb.themeData.ansi ? nb.themeData.ansi[1] : "#e06c75"
                    font.pixelSize: 13
                    font.weight: Font.Bold
                    RotationAnimation on rotation { running: !cell.c.done; from: 0; to: 360; duration: 1200; loops: Animation.Infinite }
                }
                Text {
                    Layout.fillWidth: true
                    text: cell.c.command !== "" ? cell.c.command.trim().replace(/\s*\n\s*/g, "  \u23ce  ") : "(command)"
                    maximumLineCount: 1
                    wrapMode: Text.NoWrap
                    elide: Text.ElideRight
                    color: Theme.text
                    font.family: nb.fontFamily
                    font.pointSize: 9
                    font.weight: Font.DemiBold
                    renderType: Text.NativeRendering
                    TapHandler { onTapped: cell.collapsed = !cell.collapsed }
                }
                Text { text: nb.shortDir(cell.c.cwd); color: Theme.textDim; font.pixelSize: 10; elide: Text.ElideMiddle; Layout.maximumWidth: 200 }
                Text {
                    text: (cell.c.done ? nb.seconds(cell.c.duration) + "  ·  " : "") + Qt.formatTime(new Date(cell.c.started), "hh:mm")
                          + (cell.c.done && cell.c.status !== 0 ? "  ·  exit " + cell.c.status : "")
                    color: Theme.textDim
                    font.pixelSize: 10
                }
                Chip {
                    id: kindChip
                    base: cell.color
                    label: (nb.viewNames.find(v => v[0] === cell.kind) || ["", cell.kind])[1] + ((nb.kinds[cell.c.id] || "auto") === "auto" ? " \u00b7 auto" : "") + " \u25be"
                    tip: "Show the output as\u2026"
                    onClicked: kindMenu.popup(kindChip, 0, kindChip.height + 2)
                }
                Chip { base: cell.color; label: "Copy"; tip: "Copy the output"
                       onClicked: { clip.text = nb.view.cellOutput(cell.c.id, "text"); clip.selectAll(); clip.copy() } }
                Chip { base: cell.color; label: "\u21bb"; tip: "Put the command back in the editor"; onClicked: nb.rerun(cell.c.command) }
                Chip {
                    base: cell.color
                    label: cell.expanded ? "\u2921 Restore" : "\u2922 Expand"
                    tip: cell.expanded ? "Back to its own size" : "Fill the window with this output"
                    onClicked: {
                        nb.expandedId = cell.expanded ? -1 : cell.c.id
                        cell.collapsed = false
                        const i = cell.index
                        Qt.callLater(() => list.positionViewAtIndex(i, ListView.Beginning))
                    }
                }
                // (Test hook: TESSERA_TEST_EXPAND_CELL fills the window with the first cell.)
                Timer { interval: 2500; running: cell.index === 0 && nb.controller !== null && nb.controller.testEnv("EXPAND_CELL") !== ""; onTriggered: nb.expandedId = cell.c.id }
                // (Test hook: TESSERA_TEST_OPEN_KIND_MENU opens the first cell's menu.)
                Timer { interval: 1500; running: cell.index === 0 && nb.controller !== null && nb.controller.testEnv("OPEN_KIND_MENU") !== ""; onTriggered: kindMenu.popup(kindChip, 0, kindChip.height + 2) }
                SlimMenu {
                    id: kindMenu
                    Repeater {
                        model: nb.viewNames
                        delegate: SlimMenuItem {
                            required property var modelData
                            text: modelData[1]
                            checkable: true
                            checked: (nb.kinds[cell.c.id] || "auto") === modelData[0]
                            onTriggered: {
                                const k = Object.assign({}, nb.kinds); k[cell.c.id] = modelData[0]; nb.kinds = k
                                // (Rich or text Markdown becomes the default too.)
                                if (modelData[0] === "markdown" || modelData[0] === "mdtext") nb.markdownStylePicked(modelData[0] === "mdtext" ? "text" : "rich")
                            }
                        }
                    }
                }
            }

            // ---- body: the output, in its own scrolling pane ----
            Item {
                id: body
                visible: !cell.collapsed
                anchors.top: header.bottom
                x: 10
                width: parent.width - 20
                // Its natural size up to 460 px, or what it was dragged to, or the
                // whole window when expanded.
                readonly property real natural: loader.item ? loader.item.implicitHeight : 0
                height: cell.expanded ? Math.max(80, list.height - header.height - 12)
                      : nb.heights[cell.c.id] > 0 ? nb.heights[cell.c.id] : Math.min(natural, 460)
                clip: true
                Loader {
                    id: loader
                    anchors.fill: parent
                    // (Re-read while the command runs.)
                    property int tick: nb.tick
                    sourceComponent: cell.kind === "html" ? htmlView
                                   : cell.kind === "markdown" || cell.kind === "mdtext" ? markdownView
                                   : cell.kind === "image" ? imageView
                                   : ["json", "csv", "tsv", "svg"].indexOf(cell.kind) >= 0 ? richView
                                   : cell.kind === "diff" ? diffView
                                   : cell.kind === "text" ? plainView : termView
                }
                Component {
                    id: termView
                    Flickable {
                        id: tf
                        interactive: false   // (a drag selects text; the wheel and bar scroll)
                        implicitHeight: Math.max(20, tt.implicitHeight + 8)
                        contentWidth: Math.max(width, tt.implicitWidth)
                        contentHeight: tt.implicitHeight + 8
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        ScrollBar.vertical: ScrollBar { policy: tf.contentHeight > tf.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
                        ScrollBar.horizontal: ScrollBar {}
                        MomentumWheel { flick: tf }
                        TextEdit {
                            id: tt
                            BlockSelect { }
                            y: 4
                            readOnly: true
                            selectByMouse: true
                            selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
                            selectedTextColor: Theme.text
                            textFormat: TextEdit.RichText
                            text: { loader.tick; return nb.view ? nb.palette(nb.view.cellOutput(cell.c.id, "html")) : "" }
                            color: Theme.text
                            font.family: nb.fontFamily
                            font.pointSize: 10
                            renderType: Text.NativeRendering
                        }
                    }
                }
                Component {
                    id: plainView
                    Flickable {
                        id: pf
                        interactive: false   // (a drag selects text; the wheel and bar scroll)
                        implicitHeight: Math.max(20, pt.implicitHeight + 8)
                        contentHeight: pt.implicitHeight + 8
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        ScrollBar.vertical: ScrollBar { policy: pf.contentHeight > pf.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
                        MomentumWheel { flick: pf }
                        TextEdit {
                            id: pt
                            BlockSelect { }
                            y: 4
                            width: pf.width - 12
                            readOnly: true
                            selectByMouse: true
                            wrapMode: TextEdit.Wrap
                            text: { loader.tick; return nb.view ? nb.view.cellOutput(cell.c.id, "text") : "" }
                            color: Theme.text
                            font.family: nb.fontFamily
                            font.pointSize: 10
                        }
                    }
                }
                Component {
                    id: diffView
                    Flickable {
                        id: df
                        interactive: false   // (a drag selects text; the wheel and bar scroll)
                        implicitHeight: Math.max(20, dt.implicitHeight + 8)
                        contentHeight: dt.implicitHeight + 8
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        ScrollBar.vertical: ScrollBar { policy: df.contentHeight > df.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
                        MomentumWheel { flick: df }
                        TextEdit {
                            id: dt
                            BlockSelect { }
                            y: 4
                            width: df.width - 12
                            readOnly: true
                            selectByMouse: true
                            selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
                            selectedTextColor: Theme.text
                            textFormat: TextEdit.RichText
                            wrapMode: TextEdit.WrapAnywhere
                            font.family: nb.fontFamily
                            font.pointSize: 10
                            color: Theme.text
                            text: {
                                loader.tick
                                const a = nb.themeData.ansi || []
                                const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/ /g, "&nbsp;")
                                return (nb.view ? nb.view.cellOutput(cell.c.id, "text") : "").split("\n").map(l =>
                                    l.startsWith("+") && !l.startsWith("+++") ? "<font color='" + (a[2] || "#98c379") + "'>" + esc(l) + "</font>"
                                    : l.startsWith("-") && !l.startsWith("---") ? "<font color='" + (a[1] || "#e06c75") + "'>" + esc(l) + "</font>"
                                    : l.startsWith("@@") ? "<font color='" + (a[6] || "#56b6c2") + "'>" + esc(l) + "</font>" : esc(l)).join("<br>")
                            }
                        }
                    }
                }
                Component {
                    id: richView
                    RichViewer {
                        embedded: true
                        maxHeight: 100000
                        kind: cell.kind
                        text: { loader.tick; return nb.view ? nb.view.cellOutput(cell.c.id, "text") : "" }
                        fontFamily: nb.fontFamily
                        themeData: nb.themeData
                    }
                }
                Component {
                    id: imageView
                    Item {
                        implicitHeight: (pic.sourceSize.height > 0 ? pic.sourceSize.height * Math.min(1, body.width / pic.sourceSize.width) : 100) + 8
                        Image {
                            id: pic
                            anchors.fill: parent
                            anchors.topMargin: 4
                            anchors.bottomMargin: 4
                            fillMode: Image.PreserveAspectFit
                            horizontalAlignment: Image.AlignLeft
                            source: { const b = nb.view ? nb.view.cellOutput(cell.c.id, "image") : ""; return b !== "" ? "data:image/png;base64," + b : "" }
                        }
                    }
                }
                Component {
                    id: markdownView
                    // Markdown drawn as a page: headings, lists, tables, code and
                    // pictures (relative ones from the command's folder), in the
                    // theme's colors. Its HTML may show but never run scripts.
                    WebEngineView {
                        id: md
                        implicitHeight: 360
                        backgroundColor: Theme.bg
                        settings.javascriptEnabled: false
                        settings.pluginsEnabled: false
                        settings.localContentCanAccessFileUrls: true
                        settings.localContentCanAccessRemoteUrls: true
                        settings.showScrollBars: true
                        readonly property bool asText: cell.kind === "mdtext"
                        onAsTextChanged: show()
                        // Monospace, the way a terminal Markdown viewer draws it: headings
                        // keep their #s in color, bullets, a bar for quotes.
                        function textCss() {
                            const c = (x) => String(x), a = nb.themeData.ansi || []
                            const col = (n, d) => c(a[n] || d)
                            return "body{background:" + c(Theme.bg) + ";color:" + c(Theme.text) + ";font:13px/1.45 '" + nb.fontFamily + "',monospace;margin:8px 10px}"
                                + "h1,h2,h3,h4,h5,h6{font-size:1em;font-weight:bold;margin:1em 0 .4em}"
                                + "h1{color:" + col(5, "#c678dd") + "}h2{color:" + col(4, "#61afef") + "}h3{color:" + col(6, "#56b6c2") + "}h4,h5,h6{color:" + col(2, "#98c379") + "}"
                                + "h1::before{content:'# '}h2::before{content:'## '}h3::before{content:'### '}h4::before{content:'#### '}h5::before{content:'##### '}h6::before{content:'###### '}"
                                + "h1{text-decoration:underline}p{margin:.5em 0}"
                                + "a{color:" + col(4, "#61afef") + "}strong{color:" + c(Theme.fg) + "}em{color:" + col(3, "#e5c07b") + "}"
                                + "code{font:inherit;color:" + col(3, "#e5c07b") + "}"
                                + "pre{background:" + c(Theme.field) + ";padding:6px 10px;margin:.5em 0;overflow:auto}pre code{color:" + c(Theme.text) + "}"
                                + "ul{list-style:none;padding-left:2ch}ul>li::before{content:'\\2022';color:" + col(4, "#61afef") + ";display:inline-block;width:2ch;margin-left:-2ch}"
                                + "ol{padding-left:3ch}li{margin:.1em 0}"
                                + "blockquote{margin:.5em 0;padding-left:1ch;color:" + c(Theme.textDim) + ";border-left:2px solid " + col(8, "#5c6370") + "}"
                                + "table{border-collapse:collapse}th,td{border:1px solid " + c(Theme.divider) + ";padding:0 1ch}th{color:" + c(Theme.fg) + "}"
                                + "img{max-width:100%}hr{border:0;border-top:1px dashed " + c(Theme.divider) + "}"
                                + "::-webkit-scrollbar{width:9px;height:9px}::-webkit-scrollbar-thumb{background:" + c(Theme.divider) + ";border-radius:5px}"
                        }
                        function css() {
                            if (asText) return textCss()
                            const c = (x) => String(x)
                            return "body{background:" + c(Theme.bg) + ";color:" + c(Theme.text) + ";font:14px/1.6 'Noto Sans','Segoe UI',system-ui,sans-serif;margin:12px 18px;max-width:900px}"
                                + "a{color:" + c(Theme.accent) + "}h1,h2{border-bottom:1px solid " + c(Theme.divider) + ";padding-bottom:.25em}"
                                + "h1{font-size:1.8em}h2{font-size:1.4em}h1,h2,h3,h4{margin:1.1em 0 .5em;line-height:1.25}"
                                + "code{font-family:" + nb.fontFamily + ",monospace;font-size:.88em;background:" + c(Theme.field) + ";padding:.15em .35em;border-radius:5px}"
                                + "pre{background:" + c(Theme.field) + ";padding:10px 12px;border-radius:7px;overflow:auto}pre code{background:none;padding:0}"
                                + "blockquote{margin:0;padding:0 1em;color:" + c(Theme.textDim) + ";border-left:3px solid " + c(Theme.divider) + "}"
                                + "table{border-collapse:collapse}th,td{border:1px solid " + c(Theme.divider) + ";padding:5px 10px}th{background:" + c(Theme.field) + "}"
                                + "img{max-width:100%}hr{border:0;border-top:1px solid " + c(Theme.divider) + "}"
                                + "::-webkit-scrollbar{width:9px;height:9px}::-webkit-scrollbar-thumb{background:" + c(Theme.divider) + ";border-radius:5px}"
                        }
                        // Relative pictures are found next to the file shown (cat dir/README.md).
                        function baseDir() {
                            const cwd = cell.c.cwd || "/"
                            const f = (cell.c.command || "").trim().split(/\s+/).pop()
                            if (!/\.(md|markdown|mdx)$/i.test(f) || f.indexOf("/") < 0) return cwd
                            let d = f.replace(/\/[^/]*$/, "")
                            if (d.startsWith("~")) d = (nb.controller ? nb.controller.homeDir() : "") + d.slice(1)
                            return d.startsWith("/") ? d : cwd + "/" + d
                        }
                        function show() {
                            const body = nb.view ? nb.view.cellOutput(cell.c.id, "markdown") : ""
                            loadHtml("<!doctype html><html><head><meta charset='utf-8'><style>" + css() + "</style></head><body>" + body + "</body></html>",
                                     "file://" + baseDir() + "/")
                        }
                        Component.onCompleted: show()
                        Connections { target: loader; function onTickChanged() { if (!cell.c.done) md.show() } }
                        onLoadingChanged: (r) => {
                            if (r.status === WebEngineView.LoadSucceededStatus)
                                runJavaScript("Math.ceil(document.documentElement.scrollHeight)", (h) => { if (h > 0) md.implicitHeight = Math.max(60, h) })
                        }
                        onNewWindowRequested: (req) => Qt.openUrlExternally(req.requestedUrl)
                        onNavigationRequested: (req) => {
                            // (Links open in the browser, not in the cell.)
                            if (req.navigationType === WebEngineNavigationRequest.LinkClickedNavigation) {
                                req.reject(); Qt.openUrlExternally(req.url)
                            }
                        }
                    }
                }
                Component {
                    id: htmlView
                    WebEngineView {
                        id: web
                        implicitHeight: 360
                        backgroundColor: "white"
                        // Pages may not reach local files or run plugins.
                        settings.localContentCanAccessFileUrls: false
                        settings.pluginsEnabled: false
                        Component.onCompleted: loadHtml(nb.view ? nb.view.cellOutput(cell.c.id, "text") : "", "about:blank")
                        onLoadingChanged: (r) => {
                            if (r.status === WebEngineView.LoadSucceededStatus)
                                runJavaScript("Math.ceil(document.body ? document.body.getBoundingClientRect().height + 24 : 0)", (h) => { if (h > 0) web.implicitHeight = Math.max(60, h) })
                        }
                        onNewWindowRequested: (req) => Qt.openUrlExternally(req.requestedUrl)
                    }
                }
            }

            // ---- the bottom edge: drag to resize, double-click for its own size ----
            Item {
                id: grip
                visible: !cell.collapsed
                anchors.bottom: parent.bottom
                width: parent.width
                height: 10
                property real startHeight: 0
                Rectangle {
                    anchors.centerIn: parent
                    width: 36; height: 3; radius: 1.5
                    color: gripDrag.active ? Theme.accent : Theme.divider
                    opacity: gripHover.hovered || gripDrag.active ? 1 : 0.5
                }
                HoverHandler { id: gripHover; cursorShape: Qt.SizeVerCursor }
                DragHandler {
                    id: gripDrag
                    target: null
                    xAxis.enabled: false
                    cursorShape: Qt.SizeVerCursor
                    onActiveChanged: if (active) { grip.startHeight = body.height; if (cell.expanded) nb.expandedId = -1 }
                    onTranslationChanged: if (active) nb.setHeight(cell.c.id, Math.max(48, Math.round(grip.startHeight + translation.y)))
                }
                TapHandler { onDoubleTapped: { nb.setHeight(cell.c.id, 0); if (cell.expanded) nb.expandedId = -1 } }
            }
        }

        Text {
            anchors.centerIn: parent
            width: parent.width - 60
            visible: nb.cells.length === 0
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.Wrap
            text: "Notebook: each command you run becomes a cell, its output shown for what it is (a table, JSON, HTML, a picture, ...). Switch back to the terminal from the editor's corner."
            color: Theme.textDim
            font.pixelSize: 13
        }
    }
    TextEdit { id: clip; visible: false }

    // ---- as a Jupyter notebook: each command a code cell (Bash), its output kept --
    function exportIpynb(path) {
        const lines = (s) => { const o = []; let at = 0; while (at < s.length) { const n = s.indexOf("\n", at); if (n < 0) { o.push(s.slice(at)); break } o.push(s.slice(at, n + 1)); at = n + 1 } return o }
        const out = []
        let n = 0
        for (const c of cells) {
            if (!c.done || c.command.trim() === "") continue
            const outs = []
            const text = view.cellOutput(c.id, "text")
            if (text.trim() !== "") outs.push({ name: "stdout", output_type: "stream", text: lines(text.endsWith("\n") ? text : text + "\n") })
            const img = c.kind === "image" ? view.cellOutput(c.id, "image") : ""
            if (img !== "") outs.push({ data: { "image/png": img }, metadata: {}, output_type: "display_data" })
            out.push({ cell_type: "code", execution_count: ++n, id: "cell-" + c.id, metadata: { tessera: { cwd: c.cwd, status: c.status, duration_ms: c.duration } },
                       outputs: outs, source: lines(c.command) })
        }
        const nb = { cells: out, metadata: { kernelspec: { display_name: "Bash", language: "bash", name: "bash" }, language_info: { name: "bash" } }, nbformat: 4, nbformat_minor: 5 }
        const err = controller.writeTextFile(path, JSON.stringify(nb, null, 1) + "\n")
        if (err === "") openNotebook(path)
        else exportError = err
    }
    property string exportError: ""
    // (Test hook: TESSERA_TEST_NB_EXPORT=<path> saves the cells there and opens it.)
    Timer { interval: 3000; running: nb.visible && nb.controller !== null && nb.controller.testEnv("NB_EXPORT") !== ""; onTriggered: nb.exportIpynb(nb.controller.testEnv("NB_EXPORT")) }
    Row {
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.topMargin: 6
        anchors.rightMargin: 16
        z: 5
        spacing: 6
        visible: nb.cells.length > 0
        Text { visible: nb.exportError !== ""; text: nb.exportError; color: Theme.textDim; font.pixelSize: 11; anchors.verticalCenter: parent.verticalCenter }
        Chip {
            base: Theme.bg
            label: "Save as Jupyter Notebook\u2026"
            tip: "These commands and their outputs as a .ipynb, opened to run again or share"
            onClicked: exportDialog.open()
        }
    }
    FileDialog {
        id: exportDialog
        title: "Save as Jupyter Notebook"
        fileMode: FileDialog.SaveFile
        defaultSuffix: "ipynb"
        nameFilters: ["Jupyter notebooks (*.ipynb)"]
        currentFolder: nb.cells.length ? "file://" + nb.cells[nb.cells.length - 1].cwd : ""
        onAccepted: nb.exportIpynb(decodeURIComponent(String(selectedFile).replace(/^file:\/\//, "")))
    }
}
