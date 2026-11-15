import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Tessera

// Find in scrollback, floating at the top right of a pane (like iTerm2's).
// Enter goes to the previous (older) match, Shift+Enter to the next one.
Rectangle {
    id: bar
    property var view
    property bool regex: false
    property int count: 0
    property int index: -1
    property bool bad: false
    signal closed()

    readonly property string urlPattern: "(?:https?|ftp|file|ssh)://[^\\s\"'<>()]+|www\\.[^\\s\"'<>()]+"

    width: 380
    height: 40
    radius: 10
    color: Theme.panel
    border.color: Theme.divider
    visible: false

    function open(initial) {
        visible = true
        if (initial !== undefined) { field.text = initial }
        field.forceActiveFocus()
        field.selectAll()
        run()
    }
    function findUrls() {
        regex = true
        open(urlPattern)
    }
    function close() {
        visible = false
        if (view) { view.endFind(); view.forceActiveFocus() }
        closed()
    }
    function run() {
        if (!view) return
        const n = view.find(field.text, regex)
        bad = n < 0
        count = Math.max(0, n)
        index = view.findIndex()
    }
    function step(dir) {
        if (!view || count === 0) return
        index = view.findStep(dir)
    }

    // Keep the matches current as output arrives.
    Connections {
        target: bar.visible ? bar.view : null
        function onActivity() { refresh.restart() }
    }
    Timer {
        id: refresh
        interval: 400
        onTriggered: if (bar.visible && bar.view && !bar.bad && bar.count >= 0) {
            bar.count = Math.max(0, bar.view.findRefresh())
            bar.index = bar.view.findIndex()
        }
    }

    // A soft shadow.
    Rectangle {
        z: -1
        anchors.fill: parent
        anchors.margins: -1
        anchors.topMargin: 1
        radius: parent.radius + 1
        color: "black"
        opacity: 0.25
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 10
        anchors.rightMargin: 6
        spacing: 4

        Text { text: "⌕"; color: Theme.textDim; font.pixelSize: 15 }
        TextField {
            id: field
            Layout.fillWidth: true
            placeholderText: bar.regex ? "Regular expression" : "Find"
            color: bar.bad ? "#e5534b" : Theme.text
            placeholderTextColor: Theme.textDim
            selectByMouse: true
            background: Item {}
            onTextChanged: bar.run()
            Keys.onReturnPressed: (e) => bar.step((e.modifiers & Qt.ShiftModifier) ? 1 : -1)
            Keys.onEnterPressed: (e) => bar.step((e.modifiers & Qt.ShiftModifier) ? 1 : -1)
            Keys.onEscapePressed: bar.close()
            Keys.onPressed: (e) => {
                if (e.key === Qt.Key_F3) { bar.step((e.modifiers & Qt.ShiftModifier) ? 1 : -1); e.accepted = true }
            }
        }
        Text {
            text: bar.bad ? "bad regex" : (field.text === "" ? "" : (bar.count === 0 ? "no matches" : (bar.index + 1) + " of " + bar.count))
            color: bar.bad || (bar.count === 0 && field.text !== "") ? "#e5534b" : Theme.textDim
            font.pixelSize: 11
        }
        component BarButton: Rectangle {
            id: bb
            property string glyph
            property bool on: false
            property string tip
            signal hit()
            implicitWidth: 26; implicitHeight: 26; radius: 6
            color: on ? Theme.accent : (h.hovered ? Theme.tabHover : "transparent")
            Text { anchors.centerIn: parent; text: bb.glyph; color: bb.on ? Theme.bg : Theme.text; font.pixelSize: 13; font.family: bb.glyph === ".*" ? "monospace" : "" }
            HoverHandler { id: h }
            TapHandler { onTapped: bb.hit() }
            ToolTip.visible: h.hovered && tip !== ""
            ToolTip.text: tip
            ToolTip.delay: 600
        }
        BarButton { glyph: ".*"; on: bar.regex; tip: "Regular expression"; onHit: { bar.regex = !bar.regex; bar.run() } }
        BarButton { glyph: "↑"; tip: "Previous (older) match — Enter, F3"; onHit: bar.step(-1) }
        BarButton { glyph: "↓"; tip: "Next match — Shift+Enter, Shift+F3"; onHit: bar.step(1) }
        BarButton { glyph: "×"; tip: "Close — Esc"; onHit: bar.close() }
    }
}
