import QtQuick
import QtQuick.Controls

// A scroll bar in the theme's colors, easy to see: an 8 px handle on a faint
// track, brighter under the pointer or while dragged. (Its width never
// changes: one that grew and shrank as it was used jumped about.)
ScrollBar {
    id: bar
    readonly property bool lit: hovered || pressed
    padding: 2
    contentItem: Rectangle {
        implicitWidth: 8
        implicitHeight: 8
        radius: width / 2
        color: Theme.mix(Theme.textDim, Theme.text, bar.lit ? 0.6 : 0.2)
        opacity: bar.policy === ScrollBar.AlwaysOff ? 0 : bar.lit ? 0.95 : 0.7
    }
    background: Rectangle {
        color: Theme.mix(Theme.bg, Theme.fg, 0.06)
        radius: 4
        visible: bar.policy === ScrollBar.AlwaysOn
    }
}
