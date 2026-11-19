import QtQuick
import Tessera

// One tab: flat and full height. The current tab is the terminal's own color
// (so it merges into it) with a crisp accent line along its outer edge; the
// others sit quietly on the bar with hairline dividers between them.
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
    property bool nextIsCurrent: false   // no divider next to the current tab
    property string view: ""             // "notebook" or "terminal": its icon
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
    signal menuRequested(real x, real y)
    signal dragMoved(real dx, real dy)
    signal dragReleased()
    signal dragFinished(int action)

    readonly property bool hovered: hover.hovered
    readonly property bool raised: current || lifted
    readonly property color fill: raised ? surface : (hovered ? Theme.tabHover : Theme.chrome)

    implicitHeight: 32
    implicitWidth: 170
    opacity: external ? 0.4 : 1

    HoverHandler { id: hover }
    TapHandler { onTapped: tab.clicked() }
    TapHandler { acceptedButtons: Qt.MiddleButton; onTapped: tab.closeClicked() }
    TapHandler { acceptedButtons: Qt.RightButton; onTapped: (ep) => tab.menuRequested(ep.position.x, ep.position.y) }

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
    Rectangle {
        anchors.fill: parent
        color: tab.fill
        Behavior on color { ColorAnimation { duration: 90 } }
    }

    // The accent line along the outer edge of the current tab (its own
    // color when it has one); a dimmer one on a colored background tab.
    Rectangle {
        visible: tab.raised || tab.tabColor.a > 0
        anchors.left: parent.left
        anchors.right: parent.right
        y: tab.flipped ? tab.height - height : 0
        height: 2
        color: tab.tabColor.a > 0 ? tab.tabColor : Theme.accent
        opacity: tab.raised ? 1 : 0.55
    }

    // Hairline dividers: on the right of every background tab (none touching
    // the current one) and on both sides of the current one, which has no
    // line along the terminal's side.
    Rectangle {
        anchors.right: parent.right
        width: 1
        height: parent.height
        color: Theme.divider
        visible: tab.raised || !tab.nextIsCurrent
        opacity: tab.raised ? 1 : 0.7
    }
    Rectangle {
        anchors.left: parent.left
        width: 1
        height: parent.height
        color: Theme.divider
        visible: tab.raised
    }
    Rectangle {
        // The bar's edge line under background tabs.
        visible: !tab.raised
        anchors.left: parent.left
        anchors.right: parent.right
        y: tab.flipped ? 0 : tab.height - 1
        height: 1
        color: Theme.divider
    }

    // ---- contents ---------------------------------------------------------
    Text {
        id: glyph
        visible: tab.number > 0
        anchors.left: parent.left
        anchors.leftMargin: 12
        anchors.verticalCenter: parent.verticalCenter
        width: 10
        horizontalAlignment: Text.AlignHCenter
        text: tab.number
        color: Theme.textDim
        font.pixelSize: tab.number > 0 ? 10 : 11
        font.weight: Font.Bold
    }

    // Notebook or terminal, at a glance.
    Text {
        id: viewIcon
        visible: tab.view !== ""
        anchors.left: glyph.visible ? glyph.right : parent.left
        anchors.leftMargin: glyph.visible ? 6 : 12
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: 0.001
        text: tab.view === "notebook" ? "\u25a4" : ">_"
        color: tab.view === "notebook" ? Theme.accent : (tab.raised || tab.hovered ? Theme.text : Theme.textDim)
        font.family: tab.view === "notebook" ? "" : "monospace"
        font.pixelSize: tab.view === "notebook" ? 12 : 10
        font.weight: Font.Bold
        renderType: Text.NativeRendering
    }

    Text {
        id: title
        anchors.left: viewIcon.visible ? viewIcon.right : glyph.visible ? glyph.right : parent.left
        anchors.leftMargin: viewIcon.visible || glyph.visible ? 6 : 13
        anchors.right: tail.left
        anchors.rightMargin: 4
        anchors.verticalCenter: parent.verticalCenter
        text: tab.text
        color: tab.raised ? Theme.text : (tab.hovered ? Theme.text : Theme.textDim)
        font.pixelSize: 12
        clip: true
        // A folder too long for the tab: cut from the left, so its own name
        // shows (tabs are a fixed width). Anything else fades at the end.
        readonly property bool isPath: /^[~\/]/.test(tab.text)
        elide: isPath ? Text.ElideLeft : Text.ElideNone
        renderType: Text.NativeRendering
        Rectangle {
            visible: !title.isPath && title.implicitWidth > title.width
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
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        width: 18; height: 18
        // Only while pointed at (or always, with Appearance > Tabs have close buttons).
        readonly property bool showClose: tab.hovered || tab.closeAlways

        Rectangle {
            anchors.centerIn: parent
            visible: !tail.showClose && (tab.activity || tab.bell)
            width: 6; height: 6; radius: 3
            color: tab.bell ? "#e8853d" : Theme.accent
        }
        Rectangle {
            anchors.fill: parent
            radius: 4
            visible: opacity > 0
            opacity: tail.showClose ? 1 : 0
            Behavior on opacity { NumberAnimation { duration: 110 } }
            color: closeHover.hovered ? Theme.mix(tab.fill, Theme.fg, 0.14) : "transparent"
            Canvas {
                anchors.centerIn: parent
                width: 10; height: 10
                property color ink: closeHover.hovered ? Theme.text : Theme.textDim
                onInkChanged: requestPaint()
                onPaint: {
                    const c = getContext("2d"); c.reset()
                    c.strokeStyle = ink; c.lineWidth = 1.3; c.lineCap = "round"
                    c.beginPath(); c.moveTo(2, 2); c.lineTo(8, 8); c.moveTo(8, 2); c.lineTo(2, 8); c.stroke()
                }
            }
            HoverHandler { id: closeHover }
            TapHandler { onTapped: tab.closeClicked() }
        }
    }
}
