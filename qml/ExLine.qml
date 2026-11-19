import QtQuick
import Tessera

// vi's command line under an editor: ":" for Ex commands (ExCommands.qml),
// "/" and "?" to search. While a command's name is typed, the commands it
// could be, each with what it does, show above it (↑/↓ pick one, Tab
// completes it); ↑/↓ otherwise go through the ones run before; Enter runs,
// Esc (or Backspace on nothing) closes. What a command says shows here
// afterwards, until the next key in the editor.
Rectangle {
    id: line
    property var ex: null                   // the ExCommands
    property font font
    property color errorColor: "#e06c75"
    property string kind: ""                // ":", "/", "?" while open
    property string message: ""
    property bool error: false
    property real maxHeight: 300
    property int historyAt: -1
    signal closed()                         // (the editor takes the keys again)

    function open(k, initial) {
        kind = k
        message = ""
        historyAt = -1
        field.text = initial
        field.forceActiveFocus()
        field.cursorPosition = initial.length
    }
    function close() { kind = ""; closed() }
    function show(t, err) { message = t; error = !!err; messageTimer.restart() }
    function run() {
        const k = kind, text = field.text
        close()
        if (k === "/" || k === "?") { const r = ex.search(text, k === "?"); if (r !== "") show(r, r.startsWith("E")); return }
        const out = ex.run(text)
        if (out !== "") show(out, ex.error)
    }
    // (A key in the editor: what a command said goes.)
    function dismiss() { if (kind === "") message = "" }

    visible: kind !== "" || message !== ""
    height: kind !== "" ? 24 : Math.min(maxHeight, msg.implicitHeight + 10)
    color: Theme.mix(Theme.bg, Theme.fg, 0.08)
    border.color: Theme.divider
    Timer { id: messageTimer; interval: 6000; onTriggered: if (line.message.indexOf("\n") < 0) line.message = "" }

    Text {
        id: prefix
        visible: line.kind !== ""
        x: 8.001
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: 0.001
        text: line.kind
        color: Theme.accent
        font: line.font
        renderType: Text.NativeRendering
    }
    TextInput {
        id: field
        visible: line.kind !== ""
        anchors.left: prefix.right
        anchors.leftMargin: 2
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: 0.001
        font: line.font
        color: Theme.text
        selectionColor: Theme.accent
        clip: true
        renderType: Text.NativeRendering
        property int pick: 0
        readonly property var choices: line.kind === ":" && line.ex ? line.ex.completions(text) : []
        onTextChanged: pick = 0
        function complete() {
            const c = choices[Math.min(pick, choices.length - 1)]
            if (!c) return
            text = c.head + c.name + (["<", ">", "=", "#"].indexOf(c.name) >= 0 ? "" : " ")
            cursorPosition = text.length
        }
        Keys.onPressed: (e) => {
            if (e.key === Qt.Key_Escape || (e.key === Qt.Key_Backspace && text === "")) { line.close(); e.accepted = true; return }
            if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { line.run(); e.accepted = true; return }
            if (e.key === Qt.Key_Tab && choices.length) { complete(); e.accepted = true; return }
            if ((e.key === Qt.Key_Down || e.key === Qt.Key_Up) && choices.length > 1 && line.historyAt < 0) {
                pick = (pick + (e.key === Qt.Key_Down ? 1 : choices.length - 1)) % choices.length
                e.accepted = true
                return
            }
            if (e.key === Qt.Key_Up || e.key === Qt.Key_Down) {
                const h = line.ex ? line.ex.history : []
                e.accepted = true
                if (h.length === 0) return
                let i = line.historyAt < 0 ? h.length : line.historyAt
                i = Math.max(0, Math.min(h.length, i + (e.key === Qt.Key_Up ? -1 : 1)))
                line.historyAt = i
                text = i < h.length ? h[i] : ""
            }
        }
    }
    Text {
        id: msg
        visible: line.kind === "" && line.message !== ""
        x: 8.001
        y: 5.001
        width: parent.width - 16
        text: line.message
        color: line.error ? line.errorColor : Theme.text
        font: line.font
        wrapMode: Text.WrapAnywhere
        renderType: Text.NativeRendering
        MouseArea { anchors.fill: parent; onClicked: { line.message = ""; line.closed() } }
    }

    // The commands it could be, above the line.
    Rectangle {
        visible: line.kind === ":" && field.choices.length > 0
        anchors.left: parent.left
        anchors.leftMargin: 4
        anchors.bottom: parent.top
        width: Math.min(parent.width - 8, 640)
        height: Math.min(list.contentHeight, 8 * 20) + 6
        radius: 6
        color: Theme.mix(Theme.bg, Theme.fg, 0.1)
        border.color: Theme.divider
        ListView {
            id: list
            anchors.fill: parent
            anchors.margins: 3
            clip: true
            model: field.choices
            currentIndex: field.pick
            onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
            boundsBehavior: Flickable.StopAtBounds
            delegate: Rectangle {
                required property var modelData
                required property int index
                width: ListView.view.width
                height: 20
                radius: 4
                color: index === field.pick ? Theme.mix(Theme.bg, Theme.accent, 0.35) : "transparent"
                Row {
                    x: 6.001
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    spacing: 10
                    Text { width: 170; text: modelData.name + (modelData.short !== modelData.name ? "  (" + modelData.short + ")" : ""); color: Theme.text; font.family: line.font.family; font.pointSize: line.font.pointSize; font.bold: true; elide: Text.ElideRight; renderType: Text.NativeRendering }
                    Text { text: modelData.help; color: Theme.textDim; font.pixelSize: 11; renderType: Text.NativeRendering }
                }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: { field.pick = index; field.complete(); field.forceActiveFocus() } }
            }
        }
    }
}
