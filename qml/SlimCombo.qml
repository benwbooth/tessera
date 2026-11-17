import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T

// A compact dropdown: a 26 px field with a chevron, and a tight popup list
// (24 px rows, scrolls with momentum) in the theme's colors. Works editable too.
T.ComboBox {
    id: control
    implicitWidth: 200
    implicitHeight: 26
    leftPadding: 9
    rightPadding: 24
    font.pixelSize: 12

    readonly property color tint: Theme.mix(Theme.panel, Theme.accent, 0.3)

    background: Rectangle {
        radius: 6
        color: control.hovered && !control.editable ? Theme.mix(Theme.field, Theme.text, 0.05) : Theme.field
        border.color: control.activeFocus || control.popup.visible ? Theme.accent : Theme.divider
    }

    indicator: Text {
        x: control.width - width - 8
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: 0.001
        text: "\u25be"
        color: Theme.textDim
        font.pixelSize: 11
        renderType: Text.NativeRendering
    }

    contentItem: T.TextField {
        leftPadding: 0
        rightPadding: 0
        verticalAlignment: Text.AlignVCenter
        text: control.editable ? control.editText : control.displayText
        enabled: control.editable
        readOnly: !control.editable
        autoScroll: control.editable
        inputMethodHints: control.inputMethodHints
        validator: control.validator
        selectByMouse: control.selectTextByMouse
        font: control.font
        color: Theme.text
        selectionColor: Theme.accent
        selectedTextColor: Theme.bg
        renderType: Text.NativeRendering
        topPadding: 0.001
    }

    delegate: T.ItemDelegate {
        id: row
        required property int index
        required property var modelData
        width: ListView.view ? ListView.view.width : control.width
        implicitHeight: 24
        leftPadding: 8
        rightPadding: 8
        highlighted: control.highlightedIndex === index
        hoverEnabled: true
        contentItem: Text {
            text: typeof row.modelData === "object" && row.modelData !== null && control.textRole ? row.modelData[control.textRole] : String(row.modelData)
            color: Theme.text
            font.pixelSize: 12
            font.weight: control.currentIndex === row.index ? Font.DemiBold : Font.Normal
            font.underline: row.hovered
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
            renderType: Text.NativeRendering
        }
        background: Rectangle {
            radius: 4
            color: row.highlighted || row.hovered ? control.tint : "transparent"
        }
    }

    popup: T.Popup {
        y: control.height + 2
        width: Math.max(control.width, 160)
        implicitHeight: Math.min(contentItem.implicitHeight + 8, 320)
        padding: 4
        topPadding: 4.001   // (off the half pixel: 130% scale smears native text there)
        contentItem: ListView {
            id: list
            clip: true
            implicitHeight: contentHeight
            model: control.delegateModel
            currentIndex: control.highlightedIndex
            boundsBehavior: Flickable.StopAtBounds
            highlightMoveDuration: 0
            ScrollBar.vertical: ScrollBar { policy: list.contentHeight > list.height ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff }
            MomentumWheel { flick: list }
        }
        background: Rectangle {
            radius: 7
            color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.03)
            border.color: Theme.divider
        }
    }
}
