import QtQuick
import QtQuick.Controls
import Tessera

// A small, quiet button: the editor's corner controls and notebook cells.
Rectangle {
    id: chip
    property string label
    property string tip
    property color base: Theme.chrome
    signal clicked()
    width: chipText.implicitWidth + 12
    height: 18
    radius: 5
    color: chipHover.hovered ? Theme.mix(base, Theme.fg, 0.12) : Theme.mix(base, Theme.fg, 0.05)
    Text { id: chipText; anchors.centerIn: parent; text: chip.label; color: Theme.textDim; font.pixelSize: 10; font.weight: Font.DemiBold }
    HoverHandler { id: chipHover }
    TapHandler { onTapped: chip.clicked() }
    ToolTip.visible: chipHover.hovered && tip !== ""
    ToolTip.text: tip
    ToolTip.delay: 600
}
