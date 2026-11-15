import QtQuick
import QtQuick.Shapes
import Tessera

// One tab. The current tab is cut from the terminal's own surface and runs
// into it (rounded corners away from the terminal, flared corners where it
// meets it, a hairline outline and a soft shadow); the others sit flat on the
// strip. Tabs can be dragged: along the strip to reorder, onto another
// window's strip to move there, or anywhere else to become a window of their own.
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
    property var tabItem: null           // the tab this button shows (for drops)
    property var payload: null           // function() -> JSON for dragging it out
    property bool dragging: false
    signal clicked()
    signal closeClicked()
    signal dragFinished(int action)

    readonly property real flare: 9
    readonly property bool hovered: hover.hovered
    readonly property color fill: current ? surface : (hovered ? Theme.tabHover : Theme.chrome)

    implicitHeight: 36
    implicitWidth: 220
    opacity: dragging ? 0.45 : 1

    HoverHandler { id: hover }
    TapHandler { onTapped: tab.clicked() }
    TapHandler { acceptedButtons: Qt.MiddleButton; onTapped: tab.closeClicked() }

    // ---- dragging -----------------------------------------------------------
    Drag.dragType: Drag.Automatic
    Drag.supportedActions: Qt.MoveAction | Qt.CopyAction
    Drag.proposedAction: Qt.MoveAction
    Drag.onDragFinished: (action) => {
        tab.Drag.active = false
        tab.dragging = false
        tab.dragFinished(action)
    }
    DragHandler {
        id: dh
        target: null
        dragThreshold: 8
        onActiveChanged: if (active && !tab.dragging) {
            tab.dragging = true
            tab.Drag.hotSpot.x = centroid.pressPosition.x
            tab.Drag.hotSpot.y = centroid.pressPosition.y
            tab.grabToImage(function(r) {
                tab.Drag.imageSource = r.url
                tab.Drag.mimeData = { "application/x-tessera-tab": tab.payload ? tab.payload() : "" }
                tab.Drag.active = true
            })
        }
    }

    // ---- body -----------------------------------------------------------------
    Item {
        anchors.fill: parent
        transform: Scale { origin.y: tab.height / 2; yScale: tab.flipped ? -1 : 1 }

        // Soft shadow around the current tab.
        Shape {
            anchors.fill: parent
            visible: tab.current
            preferredRendererType: Shape.CurveRenderer
            opacity: Theme.dark ? 0.35 : 0.12
            ShapePath {
                strokeWidth: 4
                strokeColor: "black"
                fillColor: "transparent"
                joinStyle: ShapePath.RoundJoin
                startX: sh.r; startY: sh.h
                PathLine { x: sh.r; y: sh.r + 1 }
                PathQuad { x: 2 * sh.r; y: 1; controlX: sh.r; controlY: 1 }
                PathLine { x: sh.w - 2 * sh.r; y: 1 }
                PathQuad { x: sh.w - sh.r; y: sh.r + 1; controlX: sh.w - sh.r; controlY: 1 }
                PathLine { x: sh.w - sh.r; y: sh.h }
            }
            QtObject { id: sh; readonly property real w: tab.width; readonly property real h: tab.height; readonly property real r: tab.flare }
        }

        Shape {
            anchors.fill: parent
            visible: tab.current || tab.hovered
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                id: sp
                strokeWidth: tab.current ? 1 : -1
                strokeColor: Qt.rgba(Theme.divider.r, Theme.divider.g, Theme.divider.b, 0.7)
                fillColor: tab.current ? tab.surface : Theme.tabHover
                readonly property real w: tab.width
                readonly property real h: tab.height + 1   // past the edge: no outline along the join
                readonly property real r: tab.flare
                startX: 0; startY: sp.h
                PathQuad { x: sp.r; y: sp.h - sp.r; controlX: sp.r; controlY: sp.h }
                PathLine { x: sp.r; y: sp.r }
                PathQuad { x: 2 * sp.r; y: 0.5; controlX: sp.r; controlY: 0.5 }
                PathLine { x: sp.w - 2 * sp.r; y: 0.5 }
                PathQuad { x: sp.w - sp.r; y: sp.r; controlX: sp.w - sp.r; controlY: 0.5 }
                PathLine { x: sp.w - sp.r; y: sp.h - sp.r }
                PathQuad { x: sp.w; y: sp.h; controlX: sp.w - sp.r; controlY: sp.h }
            }
        }

        // Tab color: a band along the outer edge.
        Rectangle {
            visible: tab.tabColor.a > 0
            x: tab.flare * 2
            width: tab.width - tab.flare * 4
            height: 3
            radius: 1.5
            y: 2
            color: tab.tabColor
        }
    }

    Rectangle {
        anchors.right: parent.right
        anchors.rightMargin: tab.flare / 2
        anchors.verticalCenter: parent.verticalCenter
        width: 1
        height: 14
        color: Theme.divider
        opacity: 0.7
        visible: !tab.current && !tab.hovered && !tab.nextIsCurrent
    }

    // ---- contents ---------------------------------------------------------
    Item {
        id: badge
        anchors.left: parent.left
        anchors.leftMargin: tab.flare + 11
        anchors.verticalCenter: parent.verticalCenter
        width: 18; height: 18
        Rectangle {
            anchors.fill: parent
            radius: 5
            color: tab.tabColor.a > 0 ? tab.tabColor : (tab.tmux ? Theme.tmux : Theme.accent)
            opacity: tab.current ? 0.95 : 0.55
        }
        Text {
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: tab.number > 0 ? 0 : 0.5
            text: tab.number > 0 ? tab.number : (tab.tmux ? "∷" : "❯")
            color: Theme.dark ? Theme.chrome : "white"
            font.pixelSize: tab.number > 0 ? 11 : 10
            font.weight: Font.Bold
        }
    }

    Text {
        id: title
        anchors.left: badge.right
        anchors.leftMargin: 8
        anchors.right: tail.left
        anchors.rightMargin: 4
        anchors.verticalCenter: parent.verticalCenter
        text: tab.text
        color: tab.current ? Theme.text : Theme.textDim
        font.pixelSize: 13
        font.weight: tab.current ? Font.DemiBold : Font.Normal
        clip: true
        // Fade the end of a long title instead of an ellipsis.
        Rectangle {
            visible: title.implicitWidth > title.width
            anchors.right: parent.right
            width: 22
            height: parent.height
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: Qt.rgba(tab.fill.r, tab.fill.g, tab.fill.b, 0) }
                GradientStop { position: 1; color: tab.fill }
            }
        }
    }

    // Close button, sharing its place with the activity / bell markers.
    Item {
        id: tail
        anchors.right: parent.right
        anchors.rightMargin: tab.flare + 6
        anchors.verticalCenter: parent.verticalCenter
        width: 20; height: 20
        readonly property bool showClose: tab.hovered || tab.closeAlways || (tab.current && !tab.activity && !tab.bell)

        Rectangle {
            anchors.centerIn: parent
            visible: !tail.showClose && (tab.activity || tab.bell)
            width: 7; height: 7; radius: 3.5
            color: tab.bell ? "#e8853d" : Theme.accent
        }
        Rectangle {
            anchors.fill: parent
            radius: 10
            visible: tail.showClose
            color: closeHover.hovered ? Theme.divider : "transparent"
            Text {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -1
                text: "×"
                color: closeHover.hovered ? Theme.text : Theme.textDim
                font.pixelSize: 15
            }
            HoverHandler { id: closeHover }
            TapHandler { onTapped: tab.closeClicked() }
        }
    }
}
