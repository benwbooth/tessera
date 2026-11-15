import QtQuick
import Tessera

// Minimize, maximize/restore and close, drawn the way the desktop draws them:
// KDE Plasma's Breeze on Linux (round buttons, red close), Windows' wide
// square buttons on Windows. macOS keeps its own traffic lights (the window
// asks for them in the title bar) and doesn't use this.
Row {
    id: wc
    required property var window
    readonly property bool maximized: window.visibility === Window.Maximized
    readonly property string look: Qt.platform.os === "windows" ? "windows" : "breeze"
    spacing: look === "windows" ? 0 : 6
    rightPadding: look === "windows" ? 0 : 8
    leftPadding: look === "windows" ? 0 : 6

    component WinButton: Item {
        id: b
        property string glyph
        property bool danger: false
        signal hit()
        width: wc.look === "windows" ? 46 : 24
        height: wc.look === "windows" ? wc.height : 24
        anchors.verticalCenter: parent.verticalCenter
        readonly property bool hot: area.containsMouse
        // Windows: a full-height block that lights up (red for close).
        Rectangle {
            visible: wc.look === "windows"
            anchors.fill: parent
            color: area.pressed ? (b.danger ? "#b71c1c" : Theme.divider) : (b.hot ? (b.danger ? "#e81123" : Theme.tabHover) : "transparent")
        }
        // Breeze: a circle that fills on hover (red for close).
        Rectangle {
            visible: wc.look === "breeze"
            anchors.fill: parent
            radius: width / 2
            color: b.hot ? (b.danger ? "#da4453" : Theme.text) : "transparent"
            opacity: area.pressed ? 0.8 : 1
        }
        Canvas {
            id: cv
            anchors.centerIn: parent
            width: 16; height: 16
            readonly property color ink: wc.look === "breeze"
                ? (b.hot ? (b.danger ? "white" : Theme.chrome) : Theme.text)
                : (b.hot && b.danger ? "white" : Theme.text)
            onInkChanged: requestPaint()
            Connections { target: b; function onGlyphChanged() { cv.requestPaint() } }
            onPaint: {
                const c = getContext("2d")
                c.reset()
                c.strokeStyle = ink
                c.lineWidth = wc.look === "windows" ? 1 : 1.5
                c.lineCap = "round"
                c.lineJoin = "round"
                c.beginPath()
                switch (b.glyph) {
                case "chevronDown": c.moveTo(4, 6.5); c.lineTo(8, 10.5); c.lineTo(12, 6.5); break
                case "chevronUp": c.moveTo(4, 10); c.lineTo(8, 6); c.lineTo(12, 10); break
                case "diamond": c.moveTo(8, 3.5); c.lineTo(12.5, 8); c.lineTo(8, 12.5); c.lineTo(3.5, 8); c.closePath(); break
                case "close": c.moveTo(4.5, 4.5); c.lineTo(11.5, 11.5); c.moveTo(11.5, 4.5); c.lineTo(4.5, 11.5); break
                case "minimize": c.moveTo(3, 8.5); c.lineTo(13, 8.5); break
                case "maximize": c.rect(3.5, 3.5, 9, 9); break
                case "restore": c.rect(3.5, 5.5, 7, 7); c.moveTo(5.5, 5.5); c.lineTo(5.5, 3.5); c.lineTo(12.5, 3.5); c.lineTo(12.5, 10.5); c.lineTo(10.5, 10.5); break
                }
                c.stroke()
            }
        }
        MouseArea { id: area; anchors.fill: parent; hoverEnabled: true; onClicked: b.hit() }
    }

    // Breeze: ⌄ minimize, ⌃ maximize, ◇ restore, ✕ close. Windows: — □ ❐ ✕.
    WinButton {
        glyph: wc.look === "breeze" ? "chevronDown" : "minimize"
        onHit: wc.window.showMinimized()
    }
    WinButton {
        glyph: wc.look === "breeze" ? (wc.maximized ? "diamond" : "chevronUp") : (wc.maximized ? "restore" : "maximize")
        onHit: wc.maximized ? wc.window.showNormal() : wc.window.showMaximized()
    }
    WinButton { glyph: "close"; danger: true; onHit: wc.window.close() }
}
