import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Tessera

// A small sheet over the window for confirmations and one-line prompts,
// drawn in the terminal theme's colors (not the toolkit's default dialog).
Popup {
    id: sheet
    property string title: ""
    property string message: ""
    property string okText: "OK"
    property bool destructive: false
    property bool input: false
    property alias value: field.text
    property string placeholder: ""
    property var onAccept: null

    function ask(opts) {
        title = opts.title || ""
        message = opts.message || ""
        okText = opts.okText || "OK"
        destructive = !!opts.destructive
        input = !!opts.input
        placeholder = opts.placeholder || ""
        field.text = opts.value || ""
        onAccept = opts.onAccept || null
        open()
    }
    function accept() {
        const f = onAccept
        onAccept = null
        close()
        if (f) f(field.text)
    }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(420, parent ? parent.width - 40 : 420)
    modal: true
    focus: true
    padding: 0
    closePolicy: Popup.CloseOnEscape

    onOpened: {
        if (input) { field.forceActiveFocus(); field.selectAll() }
        else okButton.forceActiveFocus()
    }

    Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, Theme.dark ? 0.45 : 0.25) }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: 120 }
        NumberAnimation { property: "scale"; from: 0.96; to: 1; duration: 140; easing.type: Easing.OutCubic }
    }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: 90 } }

    background: Rectangle {
        radius: 12
        color: Theme.panel
        border.color: Theme.divider
        // A soft shadow.
        Rectangle {
            z: -1
            anchors.fill: parent
            anchors.margins: -1
            anchors.topMargin: 3
            radius: 13
            color: "black"
            opacity: 0.3
        }
    }

    contentItem: ColumnLayout {
        spacing: 0
        Keys.onReturnPressed: sheet.accept()
        Keys.onEnterPressed: sheet.accept()

        Text {
            Layout.fillWidth: true
            Layout.margins: 20
            Layout.bottomMargin: sheet.message !== "" || sheet.input ? 6 : 16
            text: sheet.title
            color: Theme.text
            font.pixelSize: 15
            font.weight: Font.DemiBold
            wrapMode: Text.WordWrap
        }
        Text {
            visible: sheet.message !== ""
            Layout.fillWidth: true
            Layout.leftMargin: 20
            Layout.rightMargin: 20
            Layout.bottomMargin: sheet.input ? 10 : 18
            text: sheet.message
            color: Theme.textDim
            font.pixelSize: 13
            wrapMode: Text.WordWrap
        }
        TextField {
            id: field
            visible: sheet.input
            Layout.fillWidth: true
            Layout.leftMargin: 20
            Layout.rightMargin: 20
            Layout.bottomMargin: 18
            placeholderText: sheet.placeholder
            placeholderTextColor: Theme.textDim
            color: Theme.text
            selectByMouse: true
            font.pixelSize: 13
            leftPadding: 10
            background: Rectangle { implicitHeight: 32; radius: 7; color: Theme.field; border.color: field.activeFocus ? Theme.accent : Theme.divider }
            onAccepted: sheet.accept()
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider; opacity: 0.6 }
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 12
            spacing: 8
            Item { Layout.fillWidth: true }
            component SheetButton: Rectangle {
                id: sb
                property alias text: sbt.text
                property bool primary: false
                property bool danger: false
                signal hit()
                implicitWidth: Math.max(84, sbt.implicitWidth + 28)
                implicitHeight: 30
                radius: 8
                color: primary ? (danger ? (sbh.hovered ? "#e0606a" : "#d9474f") : (sbh.hovered ? Qt.lighter(Theme.accent, 1.1) : Theme.accent))
                               : (sbh.hovered ? Theme.tabHover : Theme.field)
                border.color: primary ? "transparent" : Theme.divider
                border.width: activeFocus && !primary ? 2 : 1
                Text { id: sbt; anchors.centerIn: parent; color: sb.primary ? "white" : Theme.text; font.pixelSize: 13; font.weight: sb.primary ? Font.DemiBold : Font.Normal }
                HoverHandler { id: sbh }
                TapHandler { onTapped: sb.hit() }
                Keys.onSpacePressed: sb.hit()
            }
            SheetButton { text: "Cancel"; onHit: sheet.close() }
            SheetButton { id: okButton; text: sheet.okText; primary: true; danger: sheet.destructive; onHit: sheet.accept() }
        }
    }
}
