import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// A leaf of the pane tree: one terminal view, with its profile.
Item {
    id: pane
    property var host: null
    property alias view: term
    property alias attachId: term.attachId
    property alias tmuxPane: term.tmuxPane
    property alias cwd: term.cwd
    property string profileName: ""
    property bool taken: false         // its session moved to another window
    property string commandOverride: "" // a shell or ssh picked from the + menu
    property alias themeJson: term.themeJson
    property alias fontFamily: term.fontFamily
    property alias fontPointSize: term.fontPointSize
    property alias lineHeight: term.lineHeight
    property alias paddingSize: term.paddingSize
    property alias cursorShape: term.cursorShape
    property alias scrollback: term.scrollback
    property alias copyOnSelect: term.copyOnSelect
    property bool dimInactive: true
    property real dimAmount: 0.22
    property bool showTitle: false     // per-pane title bar (Appearance > Panes)
    property bool broadcasting: false  // receives broadcast input: red-stripe indicator
    property string badge: ""
    property real prefW: -1
    property real prefH: -1
    signal contextMenu(var view)
    SplitView.preferredWidth: prefW
    SplitView.preferredHeight: prefH
    SplitView.minimumWidth: 60
    SplitView.minimumHeight: 40

    implicitWidth: 400
    implicitHeight: 300

    readonly property bool split: pane.host !== null && (!pane.host.isRoot || pane.host.count > 1)

    Rectangle {
        id: titleBar
        visible: pane.showTitle && pane.split
        anchors.left: parent.left
        anchors.right: parent.right
        height: visible ? 22 : 0
        color: term.activeFocus ? Theme.tabHover : Theme.chrome
        Text {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            verticalAlignment: Text.AlignVCenter
            text: term.title || (term.pwd ? term.pwd : pane.profileName)
            color: term.activeFocus ? Theme.text : Theme.textDim
            elide: Text.ElideMiddle
            font.pixelSize: 11
        }
    }

    TerminalView {
        id: term
        anchors.top: titleBar.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        focus: true
        onContextMenuRequested: pane.contextMenu(term)
    }

    // Badge: large text behind the content, top right (iTerm2's badges).
    Text {
        visible: pane.badge !== ""
        anchors.top: term.top
        anchors.right: term.right
        anchors.margins: 14
        text: pane.badge
            .replace("{title}", term.title).replace("{cwd}", term.pwd)
            .replace("{profile}", pane.profileName)
        color: Theme.fg
        opacity: 0.18
        font.pixelSize: Math.max(18, Math.min(64, term.height / 6))
        font.weight: Font.Bold
    }

    // Overlay scrollbar: shows while scrolling or near the right edge.
    Item {
        id: sbar
        z: 4
        anchors.top: term.top
        anchors.bottom: term.bottom
        anchors.right: term.right
        anchors.topMargin: 2
        anchors.bottomMargin: 2
        width: 14
        readonly property real total: term.scrollTotal
        readonly property real len: term.scrollLength
        readonly property bool scrollable: total > len + 0.5
        readonly property bool hot: barHover.hovered || thumbDrag.active
        property bool recent: false
        visible: scrollable
        opacity: hot || recent ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 220 } }

        Connections {
            target: term
            function onScrollChanged() { sbar.recent = true; hideTimer.restart() }
        }
        Timer { id: hideTimer; interval: 1100; onTriggered: sbar.recent = false }
        HoverHandler { id: barHover }

        // The track (only while hovered), click to page.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 2
            radius: width / 2
            color: Theme.fg
            opacity: sbar.hot ? 0.06 : 0
            Behavior on opacity { NumberAnimation { duration: 150 } }
            TapHandler {
                onTapped: (pt) => {
                    const page = Math.max(1, sbar.len - 1)
                    const up = pt.position.y < thumb.y
                    term.scrollToRow(Math.max(0, term.scrollOffset + (up ? -page : page)))
                }
            }
        }
        Rectangle {
            id: thumb
            readonly property real span: sbar.height - height
            width: sbar.hot ? 8 : 5
            anchors.right: parent.right
            anchors.rightMargin: 3
            radius: width / 2
            height: Math.max(28, sbar.height * sbar.len / Math.max(1, sbar.total))
            y: sbar.total > sbar.len ? span * term.scrollOffset / (sbar.total - sbar.len) : 0
            color: Theme.fg
            opacity: thumbDrag.active ? 0.6 : (sbar.hot ? 0.45 : 0.32)
            Behavior on width { NumberAnimation { duration: 120 } }
            property real startOffset: 0
            DragHandler {
                id: thumbDrag
                target: null
                yAxis.enabled: true
                xAxis.enabled: false
                onActiveChanged: if (active) thumb.startOffset = term.scrollOffset
                onTranslationChanged: if (active && thumb.span > 0) {
                    const rows = activeTranslation.y / thumb.span * (sbar.total - sbar.len)
                    term.scrollToRow(Math.round(Math.max(0, Math.min(sbar.total - sbar.len, thumb.startOffset + rows))))
                }
            }
        }
    }

    property alias findBar: findBar
    FindBar {
        id: findBar
        view: term
        z: 5
        anchors.top: term.top
        anchors.right: term.right
        anchors.topMargin: 8
        anchors.rightMargin: 14
    }

    // Broadcast Input: the receiving panes get a red outline.
    Rectangle {
        anchors.fill: term
        visible: pane.broadcasting
        color: "transparent"
        border.width: 2
        border.color: "#e04848"
        opacity: 0.8
    }

    // Dim panes without focus when the tab is split (like iTerm2).
    Rectangle {
        anchors.fill: parent
        color: Theme.dark ? "black" : "white"
        opacity: Theme.dark ? pane.dimAmount : pane.dimAmount * 1.5
        visible: pane.dimInactive && !term.activeFocus && pane.split
    }
}
