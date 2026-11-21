import QtQuick
import QtQuick.Controls
import Tessera

// A value in a notebook card, folded: a list, dict, set, tuple or record
// (tessh's tree, application/vnd.tessh.tree+json) with a ▸/▾ to open or
// close each part; Shift+click opens or closes everything under it. Texts
// arrive colored (ANSI); colored() turns them into rich text.
Column {
    id: vt
    property var node: ({})
    property string keyText: ""      // ("a: ", "x=": before the value)
    property bool wrapped: false     // (a record field's [list]: in parentheses)
    property string sep: ""          // ("," after it, in a tuple)
    property var colored: function (t) { return t }
    property string fontFamily: "monospace"
    property real pointSize: 10
    readonly property bool folds: node && node.o !== undefined
    property bool open: folds && !!node.e
    readonly property real toggleW: tm.advanceWidth("m") * 2

    FontMetrics { id: tm; font.family: vt.fontFamily; font.pointSize: vt.pointSize }

    // Shift+click: this and everything under it.
    function setAll(v) {
        if (!folds) return
        open = v
        for (let i = 0; i < kids.count; ++i) {
            const l = kids.itemAt(i)
            if (l && l.item) l.item.setAll(v)
        }
    }

    component Line: Text {
        topPadding: 0.001
        textFormat: Text.RichText
        color: Theme.text
        font.family: vt.fontFamily
        font.pointSize: vt.pointSize
        renderType: Text.NativeRendering
    }

    Item {
        width: hrow.width
        height: hrow.height
        Row {
            id: hrow
            Item {
                width: vt.toggleW
                height: head.height
                Text {
                    visible: vt.folds
                    anchors.verticalCenter: parent.verticalCenter
                    topPadding: 0.001
                    text: vt.open ? "▾" : "▸"
                    color: arrowMouse.containsMouse ? Theme.accent : Theme.textDim
                    font.family: vt.fontFamily
                    font.pointSize: vt.pointSize
                    renderType: Text.NativeRendering
                }
            }
            Line {
                id: head
                text: vt.colored(vt.keyText + (vt.wrapped ? "(" : "")
                                 + (!vt.folds ? vt.node.s + vt.sep
                                    : vt.open ? vt.node.o
                                    : vt.node.f + (vt.wrapped ? ")" : "") + vt.sep))
            }
        }
        // (The whole first line opens and closes it.)
        MouseArea {
            id: arrowMouse
            enabled: vt.folds
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: vt.folds ? Qt.PointingHandCursor : Qt.ArrowCursor
            onClicked: (e) => { if (e.modifiers & Qt.ShiftModifier) vt.setAll(!vt.open); else vt.open = !vt.open }
            ToolTip.visible: vt.folds && containsMouse
            ToolTip.delay: 700
            ToolTip.text: (vt.open ? "Fold" : "Unfold") + " (Shift+click: everything inside too)"
        }
    }
    // Its parts, indented: plain values packed in rows, others folding on their own.
    Column {
        visible: vt.folds && vt.open
        x: vt.toggleW
        Repeater {
            model: vt.folds && vt.open && vt.node.rows ? vt.node.rows : []
            delegate: Line {
                required property var modelData
                text: vt.colored(modelData)
            }
        }
        Repeater {
            id: kids
            model: vt.folds && vt.open && vt.node.items ? vt.node.items : []
            delegate: Loader {
                required property var modelData
                source: "ValueTree.qml"
                onLoaded: {
                    item.colored = vt.colored
                    item.fontFamily = vt.fontFamily
                    item.pointSize = vt.pointSize
                    item.keyText = modelData.k
                    item.wrapped = modelData.w
                    item.sep = vt.node.t ? "," : ""
                    item.node = modelData.v
                    // (A part opens as print lays it out: when it doesn't fit.)
                    item.open = item.folds && !!modelData.v.e
                }
            }
        }
    }
    Line {
        visible: vt.folds && vt.open
        x: vt.toggleW
        text: vt.colored(vt.node.c + (vt.wrapped ? ")" : "") + vt.sep)
    }
}
