import QtQuick

// A rectangular (column) selection over a read-only TextEdit, like the
// terminal's: Alt+drag (or Ctrl+Alt+drag) marks a block, and its text (each
// line's part inside the block) is copied when the mouse is let go. A plain
// drag goes on to the TextEdit's own selection. Put it inside the TextEdit:
//     BlockSelect { edit: myText }
MouseArea {
    id: bs
    property var edit: parent
    property string copied: ""          // the last block (for tests)
    anchors.fill: parent
    acceptedButtons: Qt.LeftButton | Qt.RightButton
    preventStealing: true
    property point from: Qt.point(0, 0)
    property point to: Qt.point(0, 0)
    property bool on: false             // a block is shown

    onPressed: (m) => {
        if (m.button === Qt.RightButton) {
            menu.popup(bs, m.x, m.y)
            return
        }
        if (!(m.modifiers & Qt.AltModifier)) {
            // (Not a block: the text's own selection, and the old block goes.)
            on = false
            m.accepted = false
            return
        }
        edit.deselect()
        from = Qt.point(m.x, m.y)
        to = from
        on = true
    }
    onPositionChanged: (m) => { if (on && pressed) to = Qt.point(m.x, m.y) }
    onReleased: (m) => {
        if (m.button === Qt.RightButton || !on) return
        to = Qt.point(m.x, m.y)
        const t = blockText()
        if (t === "") { on = false; return }
        copied = t
        copyText(t)
    }

    // The block, line by line: [{y, height, a, b}] (a, b text positions).
    function rows() {
        const left = Math.min(from.x, to.x), right = Math.max(from.x, to.x)
        const top = Math.min(from.y, to.y), bottom = Math.max(from.y, to.y)
        const out = []
        let y = top
        let guard = 0
        while (y <= bottom + 0.5 && guard++ < 10000) {
            const p = edit.positionAt(left, y)
            const r = edit.positionToRectangle(p)
            const mid = r.y + r.height / 2
            const a = edit.positionAt(left, mid)
            const b = edit.positionAt(right, mid)
            if (out.length === 0 || out[out.length - 1].y !== r.y) out.push({ y: r.y, height: r.height, a: a, b: Math.max(a, b) })
            const next = r.y + Math.max(1, r.height)
            if (next <= y) break
            y = next + 0.5
        }
        return out
    }
    function blockText() {
        return rows().map(r => edit.getText(r.a, r.b).replace(/[\u2028\u2029]/g, "")).join("\n")
    }
    readonly property var shown: { from; to; on; return on ? rows() : [] }

    // The block drawn: a band per line, from the left edge's column to the right's.
    Repeater {
        model: bs.shown
        delegate: Rectangle {
            required property var modelData
            readonly property rect ra: bs.edit.positionToRectangle(modelData.a)
            readonly property real fromX: Math.min(bs.from.x, bs.to.x)
            readonly property real toX: Math.max(bs.from.x, bs.to.x)
            x: modelData.a === modelData.b ? fromX : ra.x
            y: modelData.y
            width: Math.max(2, toX - x)
            height: modelData.height
            color: Theme.accent
            opacity: 0.35
        }
    }
    TextEdit { id: clip; visible: false }
    function copyText(t) { clip.text = t; clip.selectAll(); clip.copy() }
    function allText() { return edit.getText(0, edit.length).replace(/[\u2028\u2029]/g, "\n") }
    // Right click: copy what's selected (or the block), everything, or select it all.
    SlimMenu {
        id: menu
        SlimMenuItem {
            text: "Copy"
            enabled: bs.edit.selectedText !== "" || bs.on
            onTriggered: bs.copyText(bs.edit.selectedText !== "" ? bs.edit.selectedText.replace(/[\u2028\u2029]/g, "\n") : bs.blockText())
        }
        SlimMenuItem { text: "Copy All"; onTriggered: bs.copyText(bs.allText()) }
        SlimMenuItem { text: "Select All"; onTriggered: { bs.on = false; bs.edit.selectAll() } }
        SlimMenuItem { text: "Block select: Alt+drag"; enabled: false }
    }
}
