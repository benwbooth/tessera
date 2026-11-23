import QtQuick

// vi's block selection (Ctrl+V), drawn over a text field: a field selects
// only a run of text, so each line's part of the block is a rectangle here.
// A child of the TextEdit, so it scrolls with the text.
Item {
    id: vb
    property var vi: null
    property color color: "#5080c0"
    anchors.fill: parent
    Repeater {
        model: vb.vi ? vb.vi.blockRects : []
        Rectangle {
            required property var modelData
            x: modelData.x; y: modelData.y
            width: modelData.w; height: modelData.h
            color: vb.color
            opacity: 0.45
        }
    }
}
