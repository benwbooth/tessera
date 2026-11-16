import QtQuick
import QtQuick.Controls
import Tessera

// A quiet icon button for the top bar (menu, new tab, more): no frame until
// pointed at, then a soft rounded fill; darker while pressed. Icons are drawn
// as vectors (not font glyphs) so they're crisp and consistent at any scale.
Item {
    id: btn
    property string icon: "plus"       // "plus", "chevron", "menu"
    property string tip: ""
    property bool open: false           // its menu is showing
    signal clicked()

    implicitWidth: 26
    implicitHeight: 26

    readonly property bool hot: hover.hovered || open
    readonly property color ink: hot ? Theme.text : Theme.textDim

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: tap.pressed ? Theme.divider : Theme.tabHover
        opacity: btn.hot || tap.pressed ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 90 } }
    }

    Canvas {
        id: cv
        anchors.centerIn: parent
        width: 14; height: 14
        renderStrategy: Canvas.Immediate
        property color ink: btn.ink
        onInkChanged: requestPaint()
        Connections { target: btn; function onIconChanged() { cv.requestPaint() } }
        onPaint: {
            const c = getContext("2d")
            c.reset()
            c.strokeStyle = ink
            c.fillStyle = ink
            c.lineWidth = 1.5
            c.lineCap = "round"
            c.lineJoin = "round"
            c.beginPath()
            switch (btn.icon) {
            case "plus":
                c.moveTo(7, 2.5); c.lineTo(7, 11.5)
                c.moveTo(2.5, 7); c.lineTo(11.5, 7)
                break
            case "chevron":
                c.moveTo(3.5, 5.5); c.lineTo(7, 9); c.lineTo(10.5, 5.5)
                break
            case "menu":
                c.moveTo(2.5, 3.5); c.lineTo(11.5, 3.5)
                c.moveTo(2.5, 7); c.lineTo(11.5, 7)
                c.moveTo(2.5, 10.5); c.lineTo(11.5, 10.5)
                break
            }
            c.stroke()
        }
    }

    HoverHandler { id: hover }
    TapHandler { id: tap; onTapped: btn.clicked() }

    ToolTip.visible: hover.hovered && tip !== "" && !open
    ToolTip.text: tip
    ToolTip.delay: 700
}
