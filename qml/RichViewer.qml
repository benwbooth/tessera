import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Tessera

// A command's output shown for what it is: JSON as a highlighted, folded
// tree, CSV/TSV as a table, HTML and Markdown formatted, SVG drawn. Over the
// terminal it came from; the Text button (or Esc) goes back to the text.
Rectangle {
    id: viewer
    property string kind: ""           // json, csv, tsv, html, markdown, svg
    property string text: ""
    property string fontFamily: "monospace"
    property var themeData: ({})
    signal closed()

    color: Theme.bg
    focus: visible
    Keys.onEscapePressed: closed()

    function ansi(i, fallback) {
        const a = themeData && themeData.ansi
        return a && a[i] ? a[i] : fallback
    }
    function esc(s) { return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") }

    // ---- JSON: a tree of rows {depth, key, value text, kind, open, path} ----
    property var json: null
    property var folded: ({})
    function parseJson() {
        const t = text.trim()
        try { return JSON.parse(t) } catch (e) {}
        // JSON lines: an array of them.
        try { return t.split("\n").filter(l => l.trim() !== "").map(l => JSON.parse(l)) } catch (e) { return null }
    }
    readonly property var jsonRows: {
        if (kind !== "json" || json === null) return []
        const rows = []
        const walk = (v, key, depth, path, last) => {
            const isObj = v !== null && typeof v === "object"
            if (!isObj) { rows.push({ depth: depth, key: key, value: v, path: path, kind: v === null ? "null" : typeof v, last: last }); return }
            const arr = Array.isArray(v)
            const keys = arr ? v.map((_, i) => i) : Object.keys(v)
            const open = !folded[path]
            rows.push({ depth: depth, key: key, path: path, kind: arr ? "array" : "object", count: keys.length, open: open, last: last })
            if (!open || rows.length > 20000) return
            keys.forEach((k, i) => walk(v[k], arr ? null : k, depth + 1, path + "/" + k, i === keys.length - 1))
            rows.push({ depth: depth, kind: arr ? "endarray" : "endobject", path: path + "/$", last: last })
        }
        walk(json, null, 0, "", true)
        return rows
    }
    function toggle(path) { const f = Object.assign({}, folded); f[path] = !f[path]; folded = f }

    // ---- CSV / TSV: rows of cells (quotes understood) ----
    readonly property var table: {
        if (kind !== "csv" && kind !== "tsv") return []
        const sep = kind === "tsv" ? "\t" : ","
        const out = []
        for (const line of text.split("\n")) {
            if (line.trim() === "") continue
            const cells = []
            let cur = "", q = false
            for (let i = 0; i < line.length; i++) {
                const c = line[i]
                if (q) { if (c === '"' && line[i + 1] === '"') { cur += '"'; i++ } else if (c === '"') q = false; else cur += c }
                else if (c === '"') q = true
                else if (c === sep) { cells.push(cur); cur = "" }
                else cur += c
            }
            cells.push(cur)
            out.push(cells)
            if (out.length > 5000) break
        }
        return out
    }
    property int sortCol: -1
    property bool sortDesc: false
    readonly property var sortedBody: {
        const body = table.slice(1)
        if (sortCol < 0) return body
        const num = body.every(r => r[sortCol] === undefined || r[sortCol] === "" || !isNaN(Number(r[sortCol])))
        body.sort((a, b) => {
            const x = a[sortCol] || "", y = b[sortCol] || ""
            const d = num ? Number(x) - Number(y) : x.localeCompare(y)
            return sortDesc ? -d : d
        })
        return body
    }
    readonly property var colWidths: {
        if (table.length === 0) return []
        const n = Math.max(...table.slice(0, 200).map(r => r.length))
        const w = []
        for (let c = 0; c < n; c++) w.push(Math.min(360, Math.max(60, ...table.slice(0, 200).map(r => (r[c] || "").length * 8 + 24))))
        return w
    }

    onTextChanged: { folded = {}; sortCol = -1; json = kind === "json" ? parseJson() : null }
    onKindChanged: json = kind === "json" ? parseJson() : null

    // ---- header -----------------------------------------------------------------
    Rectangle {
        id: head
        width: parent.width
        height: 34
        color: Theme.mix(Theme.bg, Theme.fg, 0.05)
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.divider }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 8
            spacing: 10
            Text {
                text: ({ json: "JSON", csv: "CSV table", tsv: "TSV table", html: "HTML", markdown: "Markdown", svg: "SVG image" })[viewer.kind] || viewer.kind
                color: Theme.text
                font.pixelSize: 13
                font.weight: Font.DemiBold
            }
            Text {
                text: viewer.kind === "csv" || viewer.kind === "tsv" ? Math.max(0, viewer.table.length - 1) + " rows" + (viewer.table.length > 5000 ? " (first 5000)" : "")
                      : (viewer.text.length / 1024).toFixed(1) + " KB"
                color: Theme.textDim
                font.pixelSize: 11
            }
            Item { Layout.fillWidth: true }
            Button { text: "Copy"; flat: true; onClicked: { clip.text = viewer.text; clip.selectAll(); clip.copy() } }
            Button { text: "Text  (Esc)"; flat: true; onClicked: viewer.closed() }
        }
    }
    TextEdit { id: clip; visible: false }

    // ---- views ------------------------------------------------------------------
    Item {
        anchors.top: head.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        clip: true

        // JSON
        ListView {
            id: jsonList
            anchors.fill: parent
            anchors.margins: 8
            visible: viewer.kind === "json"
            model: viewer.jsonRows
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}
            MomentumWheel { flick: jsonList }
            delegate: Text {
                required property var modelData
                readonly property var r: modelData
                width: jsonList.width
                textFormat: Text.StyledText
                font.family: viewer.fontFamily
                font.pointSize: 10
                elide: Text.ElideRight
                renderType: Text.NativeRendering
                color: Theme.text
                text: {
                    const pad = "&nbsp;".repeat(r.depth * 2)
                    const key = r.key !== null && r.key !== undefined && r.kind !== "endobject" && r.kind !== "endarray"
                              ? "<font color='" + viewer.ansi(4, "#61afef") + "'>\"" + viewer.esc(r.key) + "\"</font>: " : ""
                    const comma = r.last ? "" : ","
                    switch (r.kind) {
                    case "object": return pad + "<font color='" + Theme.textDim + "'>" + (r.open ? "▾ " : "▸ ") + "</font>" + key + (r.open ? "{" : "{ <font color='" + Theme.textDim + "'>" + r.count + " keys</font> }" + comma)
                    case "array": return pad + "<font color='" + Theme.textDim + "'>" + (r.open ? "▾ " : "▸ ") + "</font>" + key + (r.open ? "[" : "[ <font color='" + Theme.textDim + "'>" + r.count + " items</font> ]" + comma)
                    case "endobject": return pad + "&nbsp;&nbsp;}" + comma
                    case "endarray": return pad + "&nbsp;&nbsp;]" + comma
                    case "string": return pad + "&nbsp;&nbsp;" + key + "<font color='" + viewer.ansi(2, "#98c379") + "'>\"" + viewer.esc(r.value) + "\"</font>" + comma
                    case "number": return pad + "&nbsp;&nbsp;" + key + "<font color='" + viewer.ansi(3, "#d19a66") + "'>" + r.value + "</font>" + comma
                    case "boolean": return pad + "&nbsp;&nbsp;" + key + "<font color='" + viewer.ansi(5, "#c678dd") + "'>" + r.value + "</font>" + comma
                    default: return pad + "&nbsp;&nbsp;" + key + "<font color='" + viewer.ansi(5, "#c678dd") + "'>null</font>" + comma
                    }
                }
                TapHandler { onTapped: if (r.kind === "object" || r.kind === "array") viewer.toggle(r.path) }
            }
        }
        Text {
            visible: viewer.kind === "json" && viewer.json === null
            anchors.centerIn: parent
            text: "Couldn't read this as JSON."
            color: Theme.textDim
        }

        // CSV / TSV
        Flickable {
            id: tableFlick
            anchors.fill: parent
            visible: viewer.kind === "csv" || viewer.kind === "tsv"
            contentWidth: viewer.colWidths.reduce((a, b) => a + b, 0) + 16
            contentHeight: tableCol.height + 16
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}
            ScrollBar.horizontal: ScrollBar {}
            MomentumWheel { flick: tableFlick }
            Column {
                id: tableCol
                x: 8; y: 8
                // Header (click to sort)
                Row {
                    Repeater {
                        model: viewer.table.length ? viewer.table[0] : []
                        delegate: Rectangle {
                            required property var modelData
                            required property int index
                            width: viewer.colWidths[index] || 80
                            height: 26
                            color: Theme.mix(Theme.bg, Theme.fg, 0.08)
                            Text {
                                anchors.verticalCenter: parent.verticalCenter
                                x: 8; width: parent.width - 16
                                text: modelData + (viewer.sortCol === index ? (viewer.sortDesc ? "  ▾" : "  ▴") : "")
                                font.weight: Font.DemiBold
                                font.pixelSize: 12
                                color: Theme.text
                                elide: Text.ElideRight
                            }
                            TapHandler { onTapped: { if (viewer.sortCol === index) viewer.sortDesc = !viewer.sortDesc; else { viewer.sortCol = index; viewer.sortDesc = false } } }
                        }
                    }
                }
                Repeater {
                    model: viewer.sortedBody
                    delegate: Row {
                        required property var modelData
                        required property int index
                        readonly property int rowIndex: index
                        Repeater {
                            model: viewer.colWidths.length
                            delegate: Rectangle {
                                required property int index
                                width: viewer.colWidths[index]
                                height: 24
                                color: rowIndex % 2 ? Theme.mix(Theme.bg, Theme.fg, 0.03) : "transparent"
                                Text {
                                    anchors.verticalCenter: parent.verticalCenter
                                    x: 8; width: parent.width - 16
                                    readonly property string v: modelData[index] !== undefined ? modelData[index] : ""
                                    text: v
                                    horizontalAlignment: v !== "" && !isNaN(Number(v)) ? Text.AlignRight : Text.AlignLeft
                                    font.family: viewer.fontFamily
                                    font.pointSize: 10
                                    color: Theme.text
                                    elide: Text.ElideRight
                                    renderType: Text.NativeRendering
                                }
                            }
                        }
                    }
                }
            }
        }

        // HTML / Markdown
        Flickable {
            id: docFlick
            anchors.fill: parent
            anchors.margins: 16
            visible: viewer.kind === "html" || viewer.kind === "markdown"
            contentHeight: docText.implicitHeight
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}
            MomentumWheel { flick: docFlick }
            Text {
                id: docText
                width: docFlick.width - 12
                text: docFlick.visible ? viewer.text : ""
                textFormat: viewer.kind === "html" ? Text.RichText : Text.MarkdownText
                wrapMode: Text.Wrap
                color: Theme.text
                linkColor: Theme.accent
                font.pixelSize: 14
                onLinkActivated: (l) => Qt.openUrlExternally(l)
            }
        }

        // SVG
        Image {
            anchors.fill: parent
            anchors.margins: 16
            visible: viewer.kind === "svg"
            fillMode: Image.PreserveAspectFit
            source: visible ? "data:image/svg+xml;utf8," + encodeURIComponent(viewer.text) : ""
            sourceSize.width: width * 2
            sourceSize.height: height * 2
        }
    }
}
