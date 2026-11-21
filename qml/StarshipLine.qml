import QtQuick
import Tessera

// One Starship prompt as Starship draws it: lines of colored runs (from
// prompt::starship), Powerline caps as shapes.
Column {
    id: line
    property var lines: []
    property var rightRuns: []
    property var themeData: ({})
    property string fontFamily: "monospace"
    property real fontSize: 11
    spacing: 2
    FontMetrics { id: fm; font.family: line.fontFamily; font.pointSize: Math.max(8, line.fontSize - 1) }
    function color(c, fallback) {
        if (!c) return fallback
        if (c[0] !== "p") return c
        const a = themeData && themeData.ansi
        const i = parseInt(c.slice(1))
        return a && a[i] ? a[i] : fallback
    }
    component Run: Rectangle {
        id: run
        property var r
        readonly property string cap: r.t.length === 1 && "\ue0b0\ue0b2\ue0b4\ue0b6\ue0bc\ue0be\ue0b8\ue0ba".indexOf(r.t) >= 0 ? r.t : ""
        readonly property color ink: line.color(r.fg, Theme.fg)
        width: cap !== "" ? Math.ceil(height / 2) : Math.ceil(rt.implicitWidth)
        height: Math.ceil(fm.height) + 2
        color: r.bg ? line.color(r.bg, "transparent") : "transparent"
        Text {
            id: rt
            visible: run.cap === ""
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 0.001
            text: run.r.t
            color: run.ink
            font.family: line.fontFamily
            font.pointSize: Math.max(8, line.fontSize - 1)
            font.bold: run.r.b
            renderType: Text.NativeRendering
        }
        CapShape {
            visible: run.cap !== ""
            anchors.fill: parent
            cap: run.cap
            ink: run.ink
        }
    }
    Repeater {
        model: line.lines
        delegate: Row {
            required property var modelData
            Repeater { model: modelData; delegate: Run { required property var modelData; r: modelData } }
        }
    }
}
