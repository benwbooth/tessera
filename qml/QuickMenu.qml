import QtQuick
import QtQuick.Controls

// A small black menu, made once and shown at once (the input box's ▾):
// compact rows of a label and its shortcut, a ✓ for what's on, headers and
// separators. Its rows are a model of plain objects:
//     { label, key, checked, enabled, act }   a row (act: called on a pick)
//     { header: "Editor keys", key }          a heading (with a shortcut)
//     { sep: true }                           a line
// Opened over `item` (or under it when there's no room above): openAt(item).
// Keys: Up/Down, Enter, Escape; a click outside closes it.
Popup {
    id: qm
    property var rows: []
    property int sel: -1
    // (Whether it went up, over the button: the ▴/▾ shows it.)
    property bool up: true
    signal picked()
    padding: 4
    topPadding: 4.001   // (off the half pixel: 130% scale smears native text there)
    modal: false
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutsideParent
    width: Math.min(360, Math.max(200, col.implicitWidth + leftPadding + rightPadding))
    readonly property color ink: "#e6e6e6"
    readonly property color faint: "#8a8a8a"
    background: Rectangle {
        radius: 6
        color: "#0e0e0e"
        border.color: "#2c2c2c"
        border.width: 1
    }
    enter: null
    exit: null

    function selectable(i) { const r = rows[i]; return !!r && !r.sep && r.header === undefined && r.enabled !== false }
    function move(d) {
        const n = rows.length
        for (let k = 1; k <= n; k++) {
            const i = ((sel < 0 ? (d > 0 ? -1 : 0) : sel) + d * k + n * 4) % n
            if (selectable(i)) { sel = i; return }
        }
    }
    function pick(i) {
        if (!selectable(i)) return
        const r = rows[i]
        close()
        picked()
        if (r.act) Qt.callLater(r.act)
    }
    // Over `item` (its left edge), or under it when the window has no room
    // above.
    function openAt(item) {
        sel = -1
        parent = item
        // (From the rows: the Column isn't laid out for them yet.)
        let h = topPadding + bottomPadding
        for (const r of rows) h += r.sep ? 7 : 22
        const p = item.mapToItem(null, 0, 0)
        up = p.y - h - 4 >= 0
        x = 0
        y = up ? -h - 4 : item.height + 4
        open()
        forceActiveFocus()
    }

    contentItem: Column {
        id: col
        focus: true
        Keys.onPressed: (e) => {
            if (e.key === Qt.Key_Down) { qm.move(1); e.accepted = true }
            else if (e.key === Qt.Key_Up) { qm.move(-1); e.accepted = true }
            else if ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter) && qm.sel >= 0) { qm.pick(qm.sel); e.accepted = true }
        }
        Repeater {
            model: qm.rows
            delegate: Item {
                id: row
                required property var modelData
                required property int index
                readonly property bool isSep: !!modelData.sep
                readonly property bool isHead: modelData.header !== undefined
                readonly property bool lit: !isSep && !isHead && modelData.enabled !== false && (qm.sel === index || rowHover.hovered)
                width: Math.max(implicitWidth, col.width)
                implicitWidth: isSep ? 0 : 22 + label.implicitWidth + 24 + keyText.implicitWidth + 8
                height: isSep ? 7 : 22
                Rectangle {
                    visible: row.isSep
                    x: 6; width: parent.width - 12; y: 3; height: 1
                    color: "#2a2a2a"
                }
                Rectangle {
                    anchors.fill: parent
                    radius: 4
                    visible: row.lit
                    color: Theme.accent
                }
                Text {
                    x: 7
                    y: (parent.height - height) / 2 + 0.001
                    visible: !!row.modelData.checked
                    text: "✓"
                    color: row.lit ? "white" : qm.ink
                    font.pixelSize: 11
                    renderType: Text.NativeRendering
                }
                Text {
                    id: label
                    visible: !row.isSep
                    x: row.isHead ? 8 : 22
                    y: (parent.height - height) / 2 + 0.001
                    text: row.isHead ? row.modelData.header : (row.modelData.label || "")
                    color: row.lit ? "white" : row.isHead || row.modelData.enabled === false ? qm.faint : qm.ink
                    font.pixelSize: row.isHead ? 10 : 12
                    font.weight: row.isHead ? Font.DemiBold : Font.Normal
                    font.letterSpacing: row.isHead ? 0.4 : 0
                    renderType: Text.NativeRendering
                }
                Text {
                    id: keyText
                    visible: !row.isSep
                    anchors.right: parent.right
                    anchors.rightMargin: 8
                    y: (parent.height - height) / 2 + 0.001
                    text: row.modelData.key || ""
                    color: row.lit ? "white" : qm.faint
                    font.pixelSize: 11
                    renderType: Text.NativeRendering
                }
                HoverHandler { id: rowHover; enabled: !row.isSep && !row.isHead }
                TapHandler { enabled: !row.isSep && !row.isHead; onTapped: qm.pick(row.index) }
            }
        }
    }
}
