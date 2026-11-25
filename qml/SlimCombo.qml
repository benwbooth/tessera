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
    // (The field's text cut off at its edge: its whole text in a tooltip.)
    readonly property bool clipped: contentItem.contentWidth > contentItem.width + 0.5
    ToolTip.visible: hovered && clipped && !popup.visible
    ToolTip.delay: 500
    ToolTip.text: displayText

    // (The list as wide as its longest entry, within the window.)
    TextMetrics { id: rowMetrics; font.pixelSize: 12; font.weight: Font.DemiBold }
    function widest() {
        let w = 0
        for (let i = 0; i < control.count; ++i) { rowMetrics.text = control.textAt(i); w = Math.max(w, rowMetrics.advanceWidth) }
        return Math.ceil(w) + 8 + 8 + 8 + 14   // (row padding, popup padding, scroll bar)
    }

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

    // The list: the shared macOS pop-up list (hearth/shared/app-menus), the
    // current choice over the field with its text lined up (an editable
    // one opens below).
    delegate: MacComboItem { combo: control }
    popup: MacComboPopup { combo: control; dark: Theme.dark; accent: Theme.accent }
}
