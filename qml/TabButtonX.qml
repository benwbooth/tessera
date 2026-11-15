import QtQuick
import QtQuick.Shapes
import Tessera

// One tab in the strip. The current tab is the terminal's own color and
// runs into it (rounded top corners, flared bottom corners); the others sit
// flat on the strip with thin separators between them.
Item {
    id: tab
    property string text: ""
    property bool current: false
    property bool nextIsCurrent: false   // hide the separator next to the current tab
    property bool tmux: false
    property color surface: Theme.bg    // the terminal background
    signal clicked()
    signal closeClicked()

    readonly property real flare: 8
    readonly property bool hovered: hover.hovered

    implicitHeight: 34
    implicitWidth: 220

    HoverHandler { id: hover }
    TapHandler { onTapped: tab.clicked() }
    TapHandler { acceptedButtons: Qt.MiddleButton; onTapped: tab.closeClicked() }

    // Tab body: rounded top, flared bottom (drawn for the current tab, and
    // faintly on hover).
    Shape {
        anchors.fill: parent
        visible: tab.current || tab.hovered
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            id: sp
            strokeWidth: -1
            fillColor: tab.current ? tab.surface : Theme.tabHover
            readonly property real w: tab.width
            readonly property real h: tab.height
            readonly property real r: tab.flare
            startX: 0; startY: sp.h
            PathQuad { x: sp.r; y: sp.h - sp.r; controlX: sp.r; controlY: sp.h }
            PathLine { x: sp.r; y: sp.r }
            PathQuad { x: 2 * sp.r; y: 0; controlX: sp.r; controlY: 0 }
            PathLine { x: sp.w - 2 * sp.r; y: 0 }
            PathQuad { x: sp.w - sp.r; y: sp.r; controlX: sp.w - sp.r; controlY: 0 }
            PathLine { x: sp.w - sp.r; y: sp.h - sp.r }
            PathQuad { x: sp.w; y: sp.h; controlX: sp.w - sp.r; controlY: sp.h }
            PathLine { x: 0; y: sp.h }
        }
    }

    // Separator at the right edge, between two plain tabs.
    Rectangle {
        anchors.right: parent.right
        anchors.rightMargin: tab.flare / 2
        anchors.verticalCenter: parent.verticalCenter
        width: 1
        height: 16
        color: Theme.divider
        visible: !tab.current && !tab.hovered && !tab.nextIsCurrent
    }

    Row {
        anchors.left: parent.left
        anchors.leftMargin: tab.flare + 12
        anchors.right: close.left
        anchors.rightMargin: 6
        anchors.verticalCenter: parent.verticalCenter
        spacing: 7
        Rectangle {
            visible: tab.tmux
            anchors.verticalCenter: parent.verticalCenter
            width: 6; height: 6; radius: 3
            color: Theme.tmux
        }
        Text {
            width: parent.width - (tab.tmux ? 13 : 0)
            text: tab.text
            color: tab.current ? Theme.text : Theme.textDim
            elide: Text.ElideRight
            font.pixelSize: 12
        }
    }

    Rectangle {
        id: close
        anchors.right: parent.right
        anchors.rightMargin: tab.flare + 6
        anchors.verticalCenter: parent.verticalCenter
        width: 18; height: 18; radius: 9
        color: closeHover.hovered ? Theme.divider : "transparent"
        opacity: tab.hovered || tab.current ? 1 : 0
        Text {
            anchors.centerIn: parent
            anchors.verticalCenterOffset: -1
            text: "×"
            color: closeHover.hovered ? Theme.text : Theme.textDim
            font.pixelSize: 14
        }
        HoverHandler { id: closeHover }
        TapHandler { onTapped: tab.closeClicked() }
    }
}
