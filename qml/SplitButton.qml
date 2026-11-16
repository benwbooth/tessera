import QtQuick
import QtQuick.Controls
import Tessera

// One control with two panes: the main action on the left (+, a new tab)
// and a menu on the right (the chevron). A framed, rounded pill split by a
// hairline; each pane lights up on its own when pointed at or pressed.
Item {
    id: sb
    property string mainTip: ""
    property string menuTip: ""
    property bool menuOpen: false
    signal mainClicked()
    signal menuClicked()
    readonly property Item menuAnchor: menuPane

    readonly property real r: 7
    implicitWidth: 52
    implicitHeight: 24

    readonly property color frame: Qt.rgba(Theme.fg.r, Theme.fg.g, Theme.fg.b, Theme.dark ? 0.13 : 0.16)
    readonly property color base: Qt.rgba(Theme.fg.r, Theme.fg.g, Theme.fg.b, Theme.dark ? 0.045 : 0.035)
    readonly property color lit: Qt.rgba(Theme.fg.r, Theme.fg.g, Theme.fg.b, Theme.dark ? 0.11 : 0.08)
    readonly property color down: Qt.rgba(Theme.fg.r, Theme.fg.g, Theme.fg.b, Theme.dark ? 0.18 : 0.14)

    // The pill.
    Rectangle {
        anchors.fill: parent
        radius: sb.r
        color: sb.base
        border.width: 1
        border.color: sb.frame
    }

    // A pane's fill: rounded on the outside, square where it meets the other.
    component PaneFill: Item {
        id: pf
        property bool leftSide: true
        property color fill: "transparent"
        anchors.fill: parent
        anchors.margins: 1
        Rectangle { anchors.fill: parent; radius: sb.r - 1; color: pf.fill }
        Rectangle {
            width: sb.r; height: parent.height
            x: pf.leftSide ? parent.width - width : 0
            color: pf.fill
        }
    }

    component Glyph: Canvas {
        id: g
        property string kind: "plus"
        property color ink: Theme.textDim
        width: 14; height: 14
        renderStrategy: Canvas.Immediate
        onInkChanged: requestPaint()
        onPaint: {
            const c = getContext("2d")
            c.reset()
            c.strokeStyle = ink
            c.lineWidth = 1.5
            c.lineCap = "round"
            c.lineJoin = "round"
            c.beginPath()
            if (kind === "plus") {
                c.moveTo(7, 2.5); c.lineTo(7, 11.5)
                c.moveTo(2.5, 7); c.lineTo(11.5, 7)
            } else {
                c.moveTo(4, 5.5); c.lineTo(7, 8.5); c.lineTo(10, 5.5)
            }
            c.stroke()
        }
    }

    Item {
        id: mainPane
        x: 0; y: 0
        width: sb.width - menuPane.width
        height: sb.height
        PaneFill {
            leftSide: true
            fill: mainTap.pressed ? sb.down : (mainHover.hovered ? sb.lit : "transparent")
        }
        Glyph {
            anchors.centerIn: parent
            kind: "plus"
            ink: mainHover.hovered ? Theme.text : Theme.textDim
        }
        HoverHandler { id: mainHover }
        TapHandler { id: mainTap; onTapped: sb.mainClicked() }
        ToolTip.visible: mainHover.hovered && sb.mainTip !== ""
        ToolTip.text: sb.mainTip
        ToolTip.delay: 700
    }

    // The hairline between the panes.
    Rectangle {
        x: menuPane.x - 0.5
        y: 5
        width: 1
        height: sb.height - 10
        color: sb.frame
    }

    Item {
        id: menuPane
        x: sb.width - width; y: 0
        width: 20
        height: sb.height
        PaneFill {
            leftSide: false
            fill: menuTap.pressed || sb.menuOpen ? sb.down : (menuHover.hovered ? sb.lit : "transparent")
        }
        Glyph {
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: -0.5
            kind: "chevron"
            ink: menuHover.hovered || sb.menuOpen ? Theme.text : Theme.textDim
        }
        HoverHandler { id: menuHover }
        TapHandler { id: menuTap; onTapped: sb.menuClicked() }
        ToolTip.visible: menuHover.hovered && sb.menuTip !== "" && !sb.menuOpen
        ToolTip.text: sb.menuTip
        ToolTip.delay: 700
    }
}
