import QtQuick
import QtQuick.Controls
import Tessera

// A small, quiet button: the editor's corner controls and notebook cells.
Rectangle {
    id: chip
    property string label
    // (A Nerd Font symbol before the label: the font that has it.)
    property string icon: ""
    property string tip
    property color base: Theme.chrome
    signal clicked()
    // (Implicit, so a layout gives it its width instead of squeezing it.)
    implicitWidth: chipRow.implicitWidth + 12
    implicitHeight: 18
    radius: 5
    color: chipHover.hovered ? Theme.mix(base, Theme.fg, 0.12) : Theme.mix(base, Theme.fg, 0.05)
    Row {
        id: chipRow
        anchors.centerIn: parent
        spacing: 4
        Text { visible: chip.icon !== ""; anchors.verticalCenter: parent.verticalCenter; text: chip.icon; color: Theme.textDim; font.family: "Symbols Nerd Font Mono"; font.pixelSize: 10 }
        Text { id: chipText; anchors.verticalCenter: parent.verticalCenter; text: chip.label; color: Theme.textDim; font.pixelSize: 10; font.weight: Font.DemiBold; font.underline: chipHover.hovered }
    }
    HoverHandler { id: chipHover; cursorShape: Qt.PointingHandCursor }
    TapHandler { onTapped: chip.clicked() }
    ToolTip.visible: chipHover.hovered && tip !== ""
    ToolTip.text: tip
    ToolTip.delay: 600
}
