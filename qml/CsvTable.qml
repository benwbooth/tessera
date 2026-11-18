import QtQuick
import QtQuick.Controls
import Tessera

// A CSV or TSV file as a table, however big (tables.rs reads it into SQLite
// in the background; only the rows on screen are asked for, 200 at a time).
// Click a column to sort by it (again: the other way; Shift: by it too),
// type to filter (every word somewhere in the row), or write SQL: a
// condition (age > 30) or a whole query (SELECT city, count(*) FROM t GROUP
// BY city). Columns: dragged to reorder, hidden and shown again from the
// Columns list or a right click.
Item {
    id: tv
    property var controller: null
    property string path: ""
    property string fontFamily: "monospace"
    property real pointSize: 10
    property var themeData: ({})
    property real maxHeight: 460

    readonly property int pageSize: 200
    readonly property real rowH: Math.ceil(fm.height) + 6
    implicitHeight: Math.min(maxHeight, bar.height + head.height + Math.max(1, total) * rowH + 12)

    // What the file has, and how it's shown.
    property var columns: []            // all of them, as in the file
    property var numeric: []
    property int rows: 0
    property bool done: false
    property string error: ""
    property var order: []              // the columns shown, in order
    property var sort: []               // [[name, desc]]
    property string filter: ""
    property string query: ""
    // A page of rows each, as asked for (a change of view starts again).
    property var pages: ({})
    property int total: 0
    property var shownCols: []          // what the rows are (a SELECT's own columns)
    property var widths: ({})
    property int gen: 0
    readonly property bool wholeQuery: /^\s*(select|with)\b/i.test(query)

    FontMetrics { id: fm; font.family: tv.fontFamily; font.pointSize: tv.pointSize }
    function ansi(n, d) { const a = themeData.ansi || []; return a[n] || d }

    onPathChanged: start()
    onControllerChanged: start()
    Component.onDestruction: if (controller && path !== "") controller.tableClose(path)
    function start() {
        columns = []; order = []; sort = []; pages = ({}); total = 0; rows = 0; done = false; error = ""; widths = ({})
        if (!controller || path === "") return
        poll()
    }
    function poll() {
        let r = {}
        try { r = JSON.parse(controller.tableOpen(path)) } catch (e) {}
        if (r.error) { error = r.error; done = true; return }
        const fresh = columns.length === 0 && (r.columns || []).length > 0
        columns = r.columns || []
        numeric = r.numeric || []
        rows = r.rows || 0
        done = !!r.done
        if (fresh) order = columns.slice()
        refresh()
    }
    Timer { interval: 400; repeat: true; running: !tv.done && tv.path !== ""; onTriggered: tv.poll() }

    // Asked again from the top (the view changed, or more rows came in).
    function refresh() { pages = ({}); gen++; fetch(0) }
    function fetch(page) {
        if (!controller || pages[page] !== undefined) return
        const spec = { columns: order, sort: sort, filter: filter, query: query, offset: page * pageSize, limit: pageSize }
        let r = {}
        try { r = JSON.parse(controller.tableQuery(path, JSON.stringify(spec))) } catch (e) {}
        const p = Object.assign({}, pages)
        p[page] = r.rows || []
        pages = p
        total = r.total >= 0 ? r.total : total
        error = r.error || ""
        if (r.columns && JSON.stringify(r.columns) !== JSON.stringify(shownCols)) shownCols = r.columns
        // (Column widths from what's in them, the first time a column shows.)
        const w = Object.assign({}, widths)
        let changed = false
        for (let i = 0; i < shownCols.length; ++i) {
            const c = shownCols[i]
            if (w[c] !== undefined) continue
            let n = c.length
            for (const row of p[page].slice(0, 100)) n = Math.max(n, String(row[i] || "").length)
            w[c] = Math.min(40, Math.max(4, n)) * fm.advanceWidth("0") + 18
            changed = true
        }
        if (changed) widths = w
    }
    function cell(row, col) {
        const pg = Math.floor(row / pageSize)
        const p = pages[pg]
        if (p === undefined) { Qt.callLater(fetch, pg); return "" }
        const r = p[row - pg * pageSize]
        return r ? (r[col] === undefined ? "" : r[col]) : ""
    }
    function sortBy(c, add) {
        const at = sort.findIndex(s => s[0] === c)
        let s = add ? sort.slice() : []
        if (at >= 0 && sort[at][1]) s = s.filter(x => x[0] !== c)       // (third click: unsorted)
        else if (at >= 0) { s = add ? s.map(x => x[0] === c ? [c, true] : x) : [[c, true]] }
        else s.push([c, false])
        sort = s
        refresh()
    }
    function hide(c) { order = order.filter(x => x !== c); refresh() }
    function show(c) { if (order.indexOf(c) < 0) { const o = order.slice(); o.push(c); order = o; refresh() } }
    function move(c, to) {
        const o = order.filter(x => x !== c)
        o.splice(Math.max(0, Math.min(o.length, to)), 0, c)
        order = o
        refresh()
    }
    readonly property real tableWidth: { let w = 0; for (const c of shownCols) w += widths[c] || 80; return w + 50 }

    // ---- the bar: filter, SQL, columns, how many --------------------------------
    Row {
        id: bar
        width: parent.width
        height: 28
        spacing: 6
        TextField {
            id: filterField
            width: Math.min(220, tv.width * 0.28)
            height: 24
            placeholderText: "Filter"
            font.pixelSize: 12
            color: Theme.text
            background: Rectangle { radius: 5; color: Theme.bg; border.color: filterField.activeFocus ? Theme.accent : Theme.divider }
            onTextChanged: filterTimer.restart()
            Timer { id: filterTimer; interval: 250; onTriggered: { tv.filter = filterField.text; tv.refresh() } }
        }
        TextField {
            id: queryField
            width: Math.min(420, tv.width * 0.45)
            height: 24
            placeholderText: "SQL: age > 30, or SELECT … FROM t"
            font.family: tv.fontFamily
            font.pixelSize: 12
            color: Theme.text
            background: Rectangle { radius: 5; color: Theme.bg; border.color: tv.error !== "" ? tv.ansi(1, "#e06c75") : queryField.activeFocus ? Theme.accent : Theme.divider }
            onAccepted: { tv.query = text; tv.refresh() }
            ToolTip.visible: hovered && tv.error !== ""
            ToolTip.text: tv.error
        }
        Chip {
            id: colsChip
            anchors.verticalCenter: parent.verticalCenter
            base: Theme.bg
            label: "Columns ▾"
            tip: "Show, hide and order the columns"
            onClicked: { colsPop.x = colsChip.x; colsPop.y = bar.height; colsPop.open() }
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 0.001
            text: tv.error !== "" ? tv.error
                : (tv.total.toLocaleString(Qt.locale(), "f", 0) + (tv.filter !== "" || (tv.query !== "" && !tv.wholeQuery) ? " of " + tv.rows.toLocaleString(Qt.locale(), "f", 0) : "") + " rows")
                  + (tv.done ? "" : " (reading…)")
            color: tv.error !== "" ? tv.ansi(1, "#e06c75") : Theme.textDim
            font.pixelSize: 11
            elide: Text.ElideRight
            width: Math.max(60, tv.width - x - 4)
            renderType: Text.NativeRendering
        }
    }

    // ---- the header: click sorts, drag reorders, right click for more ---------------
    Item {
        id: head
        anchors.top: bar.bottom
        anchors.topMargin: 2
        width: parent.width
        height: tv.rowH + 2
        clip: true
        Rectangle { anchors.fill: parent; color: Theme.mix(Theme.bg, Theme.fg, 0.06) }
        Row {
            x: -list.contentX + 50
            Repeater {
                model: tv.shownCols
                delegate: Rectangle {
                    id: hc
                    required property var modelData
                    required property int index
                    readonly property var s: tv.sort.find(x => x[0] === modelData)
                    width: tv.widths[modelData] || 80
                    height: head.height
                    color: hh.hovered ? Theme.tabHover : "transparent"
                    Rectangle { anchors.right: parent.right; width: 1; height: parent.height; color: Theme.divider }
                    Text {
                        x: 8.001
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.verticalCenterOffset: 0.001
                        width: parent.width - 14
                        elide: Text.ElideRight
                        text: hc.modelData + (hc.s ? (hc.s[1] ? " ▼" : " ▲") : "")
                        color: Theme.text
                        font.family: tv.fontFamily
                        font.pointSize: tv.pointSize
                        font.bold: true
                        font.underline: hh.hovered
                        renderType: Text.NativeRendering
                    }
                    HoverHandler { id: hh; cursorShape: tv.wholeQuery ? Qt.ArrowCursor : Qt.PointingHandCursor }
                    TapHandler { enabled: !tv.wholeQuery; onTapped: (ep, b) => tv.sortBy(hc.modelData, ep.modifiers & Qt.ShiftModifier) }
                    TapHandler { enabled: !tv.wholeQuery; acceptedButtons: Qt.RightButton; onTapped: { colMenu.col = hc.modelData; colMenu.popup() } }
                    // (Dragged along the header: put where it's let go.)
                    DragHandler {
                        enabled: !tv.wholeQuery
                        target: null
                        xAxis.enabled: true
                        yAxis.enabled: false
                        onActiveChanged: if (!active) {
                            const to = hc.x + hc.width / 2 + centroid.position.x - centroid.pressPosition.x
                            let acc = 0, at = 0
                            for (const c of tv.order) { if (acc + (tv.widths[c] || 80) / 2 > to) break; acc += tv.widths[c] || 80; at++ }
                            if (at !== hc.index) tv.move(hc.modelData, at > hc.index ? at - 1 : at)
                        }
                    }
                    // (Its right edge dragged: wider or narrower.)
                    Rectangle {
                        anchors.right: parent.right
                        width: 6
                        height: parent.height
                        color: "transparent"
                        HoverHandler { cursorShape: Qt.SplitHCursor }
                        DragHandler {
                            target: null
                            yAxis.enabled: false
                            property real start: 0
                            onActiveChanged: if (active) start = tv.widths[hc.modelData] || 80
                            onTranslationChanged: if (active) { const w = Object.assign({}, tv.widths); w[hc.modelData] = Math.max(30, start + translation.x); tv.widths = w }
                        }
                    }
                }
            }
        }
    }

    // ---- the rows ----------------------------------------------------------------------
    ListView {
        id: list
        anchors.top: head.bottom
        anchors.bottom: parent.bottom
        width: parent.width
        clip: true
        model: tv.total
        cacheBuffer: 400
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.HorizontalAndVerticalFlick
        contentWidth: tv.tableWidth
        ScrollBar.vertical: ScrollBar {}
        ScrollBar.horizontal: ScrollBar {}
        MomentumWheel { flick: list }
        delegate: Rectangle {
            id: rowItem
            required property int index
            width: tv.tableWidth
            height: tv.rowH
            color: index % 2 ? Theme.mix(Theme.bg, Theme.fg, 0.025) : "transparent"
            Text {
                width: 44
                horizontalAlignment: Text.AlignRight
                anchors.verticalCenter: parent.verticalCenter
                anchors.verticalCenterOffset: 0.001
                text: rowItem.index + 1
                color: Theme.textDim
                font.family: tv.fontFamily
                font.pointSize: tv.pointSize - 1
                renderType: Text.NativeRendering
            }
            Row {
                x: 50
                height: parent.height
                Repeater {
                    model: tv.shownCols
                    delegate: Item {
                        required property var modelData
                        required property int index
                        width: tv.widths[modelData] || 80
                        height: parent.height
                        readonly property bool num: !tv.wholeQuery && tv.numeric[tv.columns.indexOf(modelData)] === true
                        Text {
                            x: 8.001
                            width: parent.width - 14
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.verticalCenterOffset: 0.001
                            horizontalAlignment: parent.num ? Text.AlignRight : Text.AlignLeft
                            elide: Text.ElideRight
                            text: { tv.gen; tv.pages; return tv.cell(rowItem.index, index) }
                            color: parent.num ? tv.ansi(3, "#d19a66") : Theme.text
                            font.family: tv.fontFamily
                            font.pointSize: tv.pointSize
                            renderType: Text.NativeRendering
                        }
                    }
                }
            }
        }
    }

    // ---- columns: shown or hidden, in order --------------------------------------------
    Popup {
        id: colsPop
        width: 260
        height: Math.min(380, colsList.contentHeight + 16)
        padding: 6
        topPadding: 6.001
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        ListView {
            id: colsList
            anchors.fill: parent
            clip: true
            // (The shown ones in their order, then the hidden.)
            model: tv.order.concat(tv.columns.filter(c => tv.order.indexOf(c) < 0))
            boundsBehavior: Flickable.StopAtBounds
            MomentumWheel { flick: colsList }
            delegate: Row {
                required property var modelData
                readonly property bool on: tv.order.indexOf(modelData) >= 0
                height: 24
                spacing: 4
                CheckBox {
                    checked: parent.on
                    onToggled: checked ? tv.show(parent.modelData) : tv.hide(parent.modelData)
                    height: 22
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    width: 150
                    elide: Text.ElideRight
                    text: parent.modelData
                    color: parent.on ? Theme.text : Theme.textDim
                    font.pixelSize: 12
                    renderType: Text.NativeRendering
                }
                Chip { visible: parent.on; base: Theme.panel; label: "↑"; tip: "Earlier"; onClicked: tv.move(parent.modelData, tv.order.indexOf(parent.modelData) - 1) }
                Chip { visible: parent.on; base: Theme.panel; label: "↓"; tip: "Later"; onClicked: tv.move(parent.modelData, tv.order.indexOf(parent.modelData) + 1) }
            }
        }
    }
    SlimMenu {
        id: colMenu
        property string col: ""
        SlimMenuItem { text: "Sort ascending"; onTriggered: { tv.sort = [[colMenu.col, false]]; tv.refresh() } }
        SlimMenuItem { text: "Sort descending"; onTriggered: { tv.sort = [[colMenu.col, true]]; tv.refresh() } }
        SlimMenuItem { text: "Hide this column"; onTriggered: tv.hide(colMenu.col) }
        SlimMenuItem { text: "Move left"; onTriggered: tv.move(colMenu.col, tv.order.indexOf(colMenu.col) - 1) }
        SlimMenuItem { text: "Move right"; onTriggered: tv.move(colMenu.col, tv.order.indexOf(colMenu.col) + 1) }
        SlimMenuItem { text: "Show all columns"; onTriggered: { tv.order = tv.columns.slice(); tv.refresh() } }
    }
}
