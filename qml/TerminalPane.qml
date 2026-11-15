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
