import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// A leaf of the pane tree: one terminal view.
Item {
    id: pane
    property var host: null
    property alias view: term
    property alias attachId: term.attachId
    property alias tmuxPane: term.tmuxPane
    property alias cwd: term.cwd
    property bool dimInactive: true
    property alias themeJson: term.themeJson
    property alias fontFamily: term.fontFamily
    property alias fontPointSize: term.fontPointSize
    property alias lineHeight: term.lineHeight
    property alias paddingSize: term.paddingSize
    property alias cursorShape: term.cursorShape
    property alias scrollback: term.scrollback
    property alias copyOnSelect: term.copyOnSelect
    property real prefW: -1
    property real prefH: -1
    signal contextMenu(var view)
    SplitView.preferredWidth: prefW
    SplitView.preferredHeight: prefH
    SplitView.minimumWidth: 60
    SplitView.minimumHeight: 40

    implicitWidth: 400
    implicitHeight: 300

    TerminalView {
        id: term
        anchors.fill: parent
        focus: true
        onContextMenuRequested: pane.contextMenu(term)
    }

    // Dim panes without focus when the tab is split (like iTerm2).
    Rectangle {
        anchors.fill: parent
        color: Theme.dark ? "black" : "white"
        opacity: Theme.dark ? 0.22 : 0.35
        visible: pane.dimInactive && !term.activeFocus && pane.host !== null && (!pane.host.isRoot || pane.host.count > 1)
    }
}
