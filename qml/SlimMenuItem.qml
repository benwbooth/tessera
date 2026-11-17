import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T

// A row of a SlimMenu.
T.MenuItem {
    id: item
    implicitWidth: 22 + label.implicitWidth + 16
    implicitHeight: 24
    hoverEnabled: true
    contentItem: Item {
        Text {
            x: -2
            width: 14
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 0.001
            text: item.checked ? "✓" : ""
            color: Theme.accent
            font.pixelSize: 11
            horizontalAlignment: Text.AlignHCenter
            renderType: Text.NativeRendering
        }
        Text {
            id: label
            x: 16
            width: parent.width - x
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 0.001
            text: item.text
            color: item.enabled ? Theme.text : Theme.textDim
            font.pixelSize: 12
            font.weight: item.checked ? Font.DemiBold : Font.Normal
            font.underline: item.hovered && item.enabled
            elide: Text.ElideRight
            renderType: Text.NativeRendering
        }
    }
    leftPadding: 8
    rightPadding: 8
    background: Rectangle {
        radius: 4
        color: item.highlighted || item.hovered ? Theme.mix(Theme.panel, Theme.accent, 0.3) : "transparent"
    }
}
