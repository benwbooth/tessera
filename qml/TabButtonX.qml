import QtQuick
import QtQuick.Shapes
import Tessera

// One tab in the strip. The current tab is the terminal's own color and
// runs into it (rounded corners away from the terminal, flared corners where
// it meets it); the others sit flat on the strip with thin separators.
Item {
    id: tab
    property string text: ""
    property int number: 0               // shown when > 0 (Alt+number selects it)
    property bool current: false
    property bool nextIsCurrent: false   // hide the separator next to the current tab
    property bool tmux: false
    property bool activity: false        // new output while in the background
    property bool bell: false
    property color tabColor: "transparent"
    property bool closeAlways: false
    property bool flipped: false         // tabs below the terminal
    property color surface: Theme.bg     // the terminal background
    signal clicked()
    signal closeClicked()

    readonly property real flare: 8
    readonly property bool hovered: hover.hovered

    implicitHeight: 34
    implicitWidth: 220

    HoverHandler { id: hover }
    TapHandler { onTapped: tab.clicked() }
    TapHandler { acceptedButtons: Qt.MiddleButton; onTapped: tab.closeClicked() }

    Shape {
        anchors.fill: parent
        visible: tab.current || tab.hovered
        preferredRendererType: Shape.CurveRenderer
        transform: Scale { origin.y: tab.height / 2; yScale: tab.flipped ? -1 : 1 }
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

    // Tab color: a band along the outer edge.
    Rectangle {
        visible: tab.tabColor.a > 0
        x: tab.flare * 2
        width: tab.width - tab.flare * 4
        height: 3
        radius: 1.5
        y: tab.flipped ? tab.height - height - 1 : 1
        color: tab.tabColor
    }

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
        Text {
            id: num
            visible: tab.number > 0
            anchors.verticalCenter: parent.verticalCenter
            text: tab.number
            color: Theme.textDim
            font.pixelSize: 10
        }
        Rectangle {
            id: dot
            visible: tab.tmux || tab.activity
            anchors.verticalCenter: parent.verticalCenter
            width: 6; height: 6; radius: 3
            color: tab.activity ? Theme.accent : Theme.tmux
        }
        Text {
            id: bellIcon
            visible: tab.bell
            anchors.verticalCenter: parent.verticalCenter
            text: "\u{1F514}"
            font.pixelSize: 10
        }
        Text {
            width: parent.width - (num.visible ? num.width + 7 : 0) - (dot.visible ? 13 : 0) - (bellIcon.visible ? bellIcon.width + 7 : 0)
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
        opacity: tab.hovered || tab.current || tab.closeAlways ? 1 : 0
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
