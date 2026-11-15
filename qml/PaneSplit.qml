import QtQuick
import QtQuick.Controls
import Tessera

// A split container in a tab's pane tree. Its items are TerminalPanes or
// further PaneSplits. The tab's root split may hold a single pane.
SplitView {
    id: split
    property var host: null       // parent PaneSplit (null at the tab root)
    property bool isRoot: false
    property var tab: null        // the tab item (set on the root)
    property real prefW: -1
    property real prefH: -1
    SplitView.preferredWidth: prefW
    SplitView.preferredHeight: prefH

    handle: Rectangle {
        id: grip
        implicitWidth: 1
        implicitHeight: 1
        color: SplitHandle.pressed || SplitHandle.hovered ? Theme.accent : Theme.divider
        // A wider invisible grab area.
        containmentMask: Item {
            x: split.orientation === Qt.Horizontal ? -4 : 0
            y: split.orientation === Qt.Horizontal ? 0 : -4
            width: split.orientation === Qt.Horizontal ? 9 : grip.width
            height: split.orientation === Qt.Horizontal ? grip.height : 9
        }
    }
}
