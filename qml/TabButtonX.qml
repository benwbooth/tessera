import QtQuick
import QtQuick.Shapes
import Tessera

// One tab. The current tab is cut from the terminal's own surface and runs
// into it (rounded corners away from the terminal, flared corners where it
// meets it); the others sit flat on the strip.
//
// Dragging: along the bar the tab follows the pointer and the others slide
// out of its way (the bar handles that from dragMoved); pulled out of the bar
// it becomes a real drag-and-drop, to another window's bar or, dropped
// anywhere else, a window of its own.
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
    property bool canTearOff: true
    property bool lifted: false          // being dragged along the bar
    property bool external: false        // being dragged out of the bar
    signal clicked()
    signal closeClicked()
    signal menuRequested()
    signal dragMoved(real dx, real dy)
    signal dragReleased()
    signal dragFinished(int action)

    readonly property real flare: 6
    readonly property bool hovered: hover.hovered
    readonly property color fill: current ? surface : (hovered ? Theme.tabHover : Theme.chrome)

    implicitHeight: 26
    implicitWidth: 170
    opacity: external ? 0.4 : 1

    HoverHandler { id: hover }
    TapHandler { onTapped: tab.clicked() }
    TapHandler { acceptedButtons: Qt.MiddleButton; onTapped: tab.closeClicked() }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: tab.menuRequested() }

    // ---- dragging -----------------------------------------------------------
    Drag.dragType: Drag.Automatic
    Drag.supportedActions: Qt.MoveAction | Qt.CopyAction
    Drag.proposedAction: Qt.MoveAction
    Drag.onDragFinished: (action) => {
        tab.Drag.active = false
        tab.external = false
        tab.dragFinished(action)
    }
    function startExternalDrag() {
        if (external) return
        external = true
        tab.grabToImage(function(r) {
            tab.Drag.imageSource = r.url
            tab.Drag.hotSpot.x = tab.width / 2
            tab.Drag.hotSpot.y = tab.height / 2
            tab.Drag.mimeData = { "application/x-tessera-tab": tab.payload ? tab.payload() : "" }
            tab.Drag.active = true
        })
    }
    DragHandler {
        id: dh
        target: null
        dragThreshold: 6
        onTranslationChanged: if (active && !tab.external) tab.dragMoved(activeTranslation.x, activeTranslation.y)
        onActiveChanged: if (!active && !tab.external) tab.dragReleased()
    }

    // ---- body -----------------------------------------------------------------
    Item {
        anchors.fill: parent
        transform: Scale { origin.y: tab.height / 2; yScale: tab.flipped ? -1 : 1 }

        Shape {
            anchors.fill: parent
            visible: tab.current || tab.hovered || tab.lifted
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                id: sp
                strokeWidth: tab.current || tab.lifted ? 1 : -1
                strokeColor: Qt.rgba(Theme.divider.r, Theme.divider.g, Theme.divider.b, Theme.dark ? 0.9 : 0.6)
                fillColor: tab.current || tab.lifted ? tab.surface : Theme.tabHover
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

        // Tab color: a short bar along the outer edge.
        Rectangle {
            visible: tab.tabColor.a > 0
            anchors.horizontalCenter: parent.horizontalCenter
            width: Math.min(48, tab.width - 4 * tab.flare)
            height: 3
            radius: 1.5
            y: 1
            color: tab.tabColor
        }
    }

    // (No separators between tabs: the hover fill and the current tab's
    // shape are enough, and quieter.)

    // ---- contents ---------------------------------------------------------
    Text {
        id: glyph
        visible: tab.number > 0 || tab.tmux || tab.tabColor.a > 0
        anchors.left: parent.left
        anchors.leftMargin: tab.flare + 7
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: tab.flipped ? -1 : 1
        width: 10
        horizontalAlignment: Text.AlignHCenter
        text: tab.number > 0 ? tab.number : (tab.tmux ? "\u2237" : "\u25cf")
        color: tab.tabColor.a > 0 ? tab.tabColor : (tab.tmux ? Theme.tmux : (tab.current ? Theme.accent : Theme.textDim))
        font.pixelSize: tab.number > 0 ? 10 : 9
        font.weight: Font.Bold
    }

    Text {
        id: title
        anchors.left: glyph.visible ? glyph.right : parent.left
        anchors.leftMargin: glyph.visible ? 5 : tab.flare + 10
        anchors.right: tail.left
        anchors.rightMargin: 2
        anchors.verticalCenter: parent.verticalCenter
        text: tab.text
        color: tab.current ? Theme.text : (tab.hovered ? Theme.text : Theme.textDim)
        font.pixelSize: 12
        font.weight: Font.Normal
        anchors.verticalCenterOffset: tab.flipped ? -1 : 1
        clip: true
        // Fade the end of a long title instead of an ellipsis.
        Rectangle {
            visible: title.implicitWidth > title.width
            anchors.right: parent.right
            width: 18
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
        anchors.rightMargin: tab.flare + 3
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: tab.flipped ? -1 : 1
        width: 16; height: 16
        readonly property bool showClose: tab.hovered || tab.closeAlways || (tab.current && !tab.activity && !tab.bell)

        Rectangle {
            anchors.centerIn: parent
            visible: !tail.showClose && (tab.activity || tab.bell)
            width: 6; height: 6; radius: 3
            color: tab.bell ? "#e8853d" : Theme.accent
        }
        Rectangle {
            anchors.fill: parent
            radius: 8
            visible: tail.showClose
            color: closeHover.hovered ? Theme.divider : "transparent"
            Text {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -1
                text: "×"
                color: closeHover.hovered ? Theme.text : Theme.textDim
                font.pixelSize: 13
            }
            HoverHandler { id: closeHover }
            TapHandler { onTapped: tab.closeClicked() }
        }
    }
}
