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
    property real prefW: -1
    property real prefH: -1
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
    }

    // Dim panes without focus when the tab is split (like iTerm2).
    Rectangle {
        anchors.fill: parent
        color: "black"
        opacity: 0.22
        visible: !term.activeFocus && pane.host !== null && (!pane.host.isRoot || pane.host.count > 1)
    }
}
