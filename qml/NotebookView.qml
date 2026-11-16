import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
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

    color: Theme.bg

    // ---- cells ------------------------------------------------------------------
    property var cells: []
    property var kinds: ({})           // id -> chosen view (the dropdown)
    property var aiKinds: ({})         // id -> what the AI made of it
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
                                      ["html", "HTML"], ["markdown", "Markdown"], ["svg", "SVG"], ["diff", "Diff"], ["image", "Image"]]
    function shownKind(c) {
        const chosen = kinds[c.id]
        if (chosen && chosen !== "auto") return chosen
        if (c.kind !== "") return c.kind
        return aiKinds[c.id] || "terminal"
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
        spacing: 12
        model: nb.cells
        clip: true
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
                    text: cell.c.command !== "" ? cell.c.command : "(command)"
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
                            onTriggered: { const k = Object.assign({}, nb.kinds); k[cell.c.id] = modelData[0]; nb.kinds = k }
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
                height: loader.item ? Math.min(loader.item.implicitHeight, 460) : 0
                clip: true
                Loader {
                    id: loader
                    anchors.fill: parent
                    // (Re-read while the command runs.)
                    property int tick: nb.tick
                    sourceComponent: cell.kind === "html" ? htmlView
                                   : cell.kind === "image" ? imageView
                                   : ["json", "csv", "tsv", "markdown", "svg"].indexOf(cell.kind) >= 0 ? richView
                                   : cell.kind === "diff" ? diffView
                                   : cell.kind === "text" ? plainView : termView
                }
                Component {
                    id: termView
                    Flickable {
                        id: tf
                        implicitHeight: Math.max(20, tt.implicitHeight + 8)
                        contentWidth: Math.max(width, tt.implicitWidth)
                        contentHeight: tt.implicitHeight + 8
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        ScrollBar.vertical: ScrollBar { policy: tf.contentHeight > tf.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
                        ScrollBar.horizontal: ScrollBar {}
                        MomentumWheel { flick: tf }
                        Text {
                            id: tt
                            y: 4
                            textFormat: Text.RichText
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
                        implicitHeight: Math.max(20, pt.implicitHeight + 8)
                        contentHeight: pt.implicitHeight + 8
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        ScrollBar.vertical: ScrollBar { policy: pf.contentHeight > pf.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
                        MomentumWheel { flick: pf }
                        TextEdit {
                            id: pt
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
                        implicitHeight: Math.max(20, dt.implicitHeight + 8)
                        contentHeight: dt.implicitHeight + 8
                        clip: true
                        boundsBehavior: Flickable.StopAtBounds
                        ScrollBar.vertical: ScrollBar { policy: df.contentHeight > df.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
                        MomentumWheel { flick: df }
                        Text {
                            id: dt
                            y: 4
                            width: df.width - 12
                            textFormat: Text.StyledText
                            wrapMode: Text.WrapAnywhere
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
                        maxHeight: 460
                        kind: cell.kind
                        text: { loader.tick; return nb.view ? nb.view.cellOutput(cell.c.id, "text") : "" }
                        fontFamily: nb.fontFamily
                        themeData: nb.themeData
                    }
                }
                Component {
                    id: imageView
                    Item {
                        implicitHeight: Math.min(460, pic.sourceSize.height > 0 ? pic.sourceSize.height * Math.min(1, body.width / pic.sourceSize.width) : 100) + 8
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
                                runJavaScript("Math.ceil(document.body ? document.body.getBoundingClientRect().height + 24 : 0)", (h) => { if (h > 0) web.implicitHeight = Math.max(60, Math.min(460, h)) })
                        }
                        onNewWindowRequested: (req) => Qt.openUrlExternally(req.requestedUrl)
                    }
                }
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
}
