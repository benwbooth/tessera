import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import Tessera

// Minimize, maximize/restore and close, drawn like KDE Plasma's Breeze
// window decoration: thin stroked icons (⌄ minimize, ⌃ maximize, ◇ restore,
// ✕ close) with a round highlight behind the one under the pointer, red with
// a white ✕ for close. Colours follow the app's theme.
Row {
    id: wc
    required property var window
    readonly property bool maximized: window.visibility === Window.Maximized
    spacing: 6
    leftPadding: 6
    rightPadding: 8

    component WinButton: Item {
        id: b
        property string glyph
        property bool danger: false
        property string tip
        signal hit()
        width: 20
        height: 20
        anchors.verticalCenter: parent ? parent.verticalCenter : undefined
        readonly property bool hot: area.containsMouse
        readonly property bool down: area.pressed

        // The round highlight: the title text's colour (red for close).
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            visible: b.hot || b.down
            color: b.danger ? (b.down ? "#bf1010" : "#ed1515")
                            : (b.down ? Theme.mix(Theme.text, Theme.chrome, 0.3) : Theme.text)
        }

        // The icon, on a 16×16 grid centred in the button.
        Shape {
            anchors.centerIn: parent
            width: 16; height: 16
            preferredRendererType: Shape.CurveRenderer
            ShapePath {
                strokeColor: (b.hot || b.down) ? (b.danger ? "white" : Theme.chrome) : Theme.text
                strokeWidth: 1.5
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                joinStyle: ShapePath.RoundJoin
                PathSvg {
                    path: b.glyph === "minimize" ? "M 4.5 6.5 L 8 10 L 11.5 6.5"
                        : b.glyph === "maximize" ? "M 4.5 10 L 8 6.5 L 11.5 10"
                        : b.glyph === "restore" ? "M 8 4 L 12 8 L 8 12 L 4 8 Z"
                        : "M 4.75 4.75 L 11.25 11.25 M 11.25 4.75 L 4.75 11.25"
                }
            }
        }

        MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; onClicked: b.hit() }
        ToolTip.visible: b.hot && b.tip !== ""
        ToolTip.text: b.tip
        ToolTip.delay: 700
    }

    WinButton {
        glyph: "minimize"
        tip: "Minimize"
        onHit: wc.window.showMinimized()
    }
    WinButton {
        glyph: wc.maximized ? "restore" : "maximize"
        tip: wc.maximized ? "Restore" : "Maximize"
        onHit: wc.maximized ? wc.window.showNormal() : wc.window.showMaximized()
    }
    WinButton {
        glyph: "close"
        danger: true
        tip: "Close"
        onHit: wc.window.close()
    }
}
