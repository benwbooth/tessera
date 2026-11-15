import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtQuick.Window
import Tessera

// Preferences: every change applies at once (like iTerm2) and is saved.
Window {
    id: prefs
    property var controller
    property var settings: ({})
    property string themeName: ""
    property bool systemDark: true
    property string page: "appearance"
    signal pick(var theme)
    signal change(string key, var value)

    title: "Tessera Preferences"
    width: 860
    height: 640
    minimumWidth: 700
    minimumHeight: 480
    color: Theme.panel
    flags: Qt.Dialog

    readonly property var themes: { controller.revision; return JSON.parse(controller.themes()) }
    readonly property var fonts: JSON.parse(controller.monospaceFamilies())

    function showPage(p) {
        if (p !== "") page = p
        show()
        raise()
        requestActivate()
    }

    palette.window: Theme.panel
    palette.windowText: Theme.text
    palette.base: Theme.field
    palette.text: Theme.text
    palette.button: Theme.field
    palette.buttonText: Theme.text
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.bg
    palette.mid: Theme.divider
    palette.dark: Theme.divider
    palette.light: Theme.field
    palette.placeholderText: Theme.textDim

    // ---- small controls -------------------------------------------------

    component SectionTitle: Text {
        color: Theme.text
        font.pixelSize: 15
        font.weight: Font.DemiBold
        Layout.topMargin: 6
    }

    component Hint: Text {
        color: Theme.textDim
        font.pixelSize: 12
        wrapMode: Text.WordWrap
        Layout.fillWidth: true
    }

    // A labelled row: label on the left, the control on the right.
    component Row2: RowLayout {
        property alias label: lbl.text
        property alias hint: hintText.text
        Layout.fillWidth: true
        spacing: 16
        ColumnLayout {
            Layout.preferredWidth: 220
            Layout.alignment: Qt.AlignTop
            spacing: 2
            Text { id: lbl; color: Theme.text; font.pixelSize: 13 }
            Text { id: hintText; visible: text !== ""; color: Theme.textDim; font.pixelSize: 11; wrapMode: Text.WordWrap; Layout.preferredWidth: 220 }
        }
    }

    component Toggle: Rectangle {
        id: tg
        property bool checked: false
        signal toggled(bool on)
        implicitWidth: 38
        implicitHeight: 22
        radius: 11
        color: checked ? Theme.accent : Theme.divider
        Behavior on color { ColorAnimation { duration: 120 } }
        Rectangle {
            width: 16; height: 16; radius: 8
            y: 3
            x: tg.checked ? tg.width - width - 3 : 3
            color: "white"
            Behavior on x { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
        }
        TapHandler { onTapped: tg.toggled(!tg.checked) }
    }

    // Segmented control: one of several labels.
    component Segmented: Rectangle {
        id: seg
        property var options: []      // [{label, value}]
        property var value
        signal picked(var value)
        implicitHeight: 30
        implicitWidth: segRow.implicitWidth + 6
        radius: 8
        color: Theme.field
        border.color: Theme.divider
        Row {
            id: segRow
            anchors.centerIn: parent
            spacing: 2
            Repeater {
                model: seg.options
                delegate: Rectangle {
                    readonly property bool on: modelData.value === seg.value
                    width: Math.max(74, t.implicitWidth + 24)
                    height: 24
                    radius: 6
                    color: on ? Theme.accent : (h.hovered ? Theme.tabHover : "transparent")
                    Text {
                        id: t
                        anchors.centerIn: parent
                        text: modelData.label
                        color: parent.on ? Theme.bg : Theme.text
                        font.pixelSize: 12
                    }
                    HoverHandler { id: h }
                    TapHandler { onTapped: seg.picked(modelData.value) }
                }
            }
        }
    }

    // − value + stepper.
    component Stepper: Rectangle {
        id: st
        property real value: 0
        property real step: 1
        property real from: 0
        property real to: 100
        property string display: String(value)
        signal moved(real value)
        implicitWidth: 150
        implicitHeight: 30
        radius: 8
        color: Theme.field
        border.color: Theme.divider
        RowLayout {
            anchors.fill: parent
            anchors.margins: 3
            spacing: 0
            Rectangle {
                Layout.preferredWidth: 26; Layout.fillHeight: true; radius: 6
                color: mh.hovered ? Theme.tabHover : "transparent"
                Text { anchors.centerIn: parent; text: "−"; color: Theme.text; font.pixelSize: 15 }
                HoverHandler { id: mh }
                TapHandler { onTapped: st.moved(Math.max(st.from, st.value - st.step)) }
            }
            Text { Layout.fillWidth: true; horizontalAlignment: Text.AlignHCenter; text: st.display; color: Theme.text; font.pixelSize: 13 }
            Rectangle {
                Layout.preferredWidth: 26; Layout.fillHeight: true; radius: 6
                color: ph.hovered ? Theme.tabHover : "transparent"
                Text { anchors.centerIn: parent; text: "+"; color: Theme.text; font.pixelSize: 15 }
                HoverHandler { id: ph }
                TapHandler { onTapped: st.moved(Math.min(st.to, st.value + st.step)) }
            }
        }
    }

    component PlainButton: Rectangle {
        id: pb
        property alias text: pbText.text
        signal clicked()
        implicitWidth: pbText.implicitWidth + 28
        implicitHeight: 30
        radius: 8
        color: pbh.hovered ? Theme.tabHover : Theme.field
        border.color: Theme.divider
        Text { id: pbText; anchors.centerIn: parent; color: Theme.text; font.pixelSize: 12 }
        HoverHandler { id: pbh }
        TapHandler { onTapped: pb.clicked() }
    }

    // A theme as a little terminal: prompt, a command, the ANSI colors.
    component ThemeCard: Item {
        id: card
        property var theme
        property bool selected: false
        property string badge: ""
        signal clicked()
        signal removeClicked()
        width: 188
        height: 132
        Rectangle {
            id: swatch
            width: parent.width
            height: 100
            radius: 10
            color: card.theme.background
            border.width: card.selected ? 2 : 1
            border.color: card.selected ? Theme.accent : Theme.divider
            Column {
                x: 12; y: 10
                spacing: 3
                Row {
                    spacing: 0
                    Text { text: "~ "; color: card.theme.ansi[4]; font.family: "monospace"; font.pixelSize: 11 }
                    Text { text: "❯ "; color: card.theme.ansi[2]; font.family: "monospace"; font.pixelSize: 11 }
                    Text { text: "ls --color"; color: card.theme.foreground; font.family: "monospace"; font.pixelSize: 11 }
                }
                Row {
                    spacing: 6
                    Text { text: "src"; color: card.theme.ansi[4]; font.family: "monospace"; font.pixelSize: 11; font.bold: true }
                    Text { text: "run.sh"; color: card.theme.ansi[2]; font.family: "monospace"; font.pixelSize: 11 }
                    Text { text: "notes"; color: card.theme.foreground; font.family: "monospace"; font.pixelSize: 11 }
                }
                Row {
                    spacing: 0
                    Text { text: "error"; color: card.theme.ansi[1]; font.family: "monospace"; font.pixelSize: 11 }
                    Text { text: ": "; color: card.theme.foreground; font.family: "monospace"; font.pixelSize: 11 }
                    Text { text: "warn"; color: card.theme.ansi[3]; font.family: "monospace"; font.pixelSize: 11 }
                    Rectangle { width: 7; height: 13; color: card.theme.cursor }
                }
            }
            Row {
                x: 12
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 10
                spacing: 4
                Repeater {
                    model: 8
                    Rectangle { width: 12; height: 12; radius: 3; color: card.theme.ansi[[1, 2, 3, 4, 5, 6, 9, 12][index]] }
                }
            }
            Rectangle {
                visible: !card.theme.builtin && cardHover.hovered
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 6
                width: 20; height: 20; radius: 10
                color: Qt.rgba(0, 0, 0, 0.45)
                Text { anchors.centerIn: parent; text: "×"; color: "white"; font.pixelSize: 13 }
                TapHandler { onTapped: card.removeClicked() }
            }
        }
        RowLayout {
            anchors.top: swatch.bottom
            anchors.topMargin: 6
            width: parent.width
            spacing: 6
            Text {
                Layout.fillWidth: true
                text: card.theme.name
                color: Theme.text
                font.pixelSize: 12
                elide: Text.ElideRight
            }
            Rectangle {
                visible: card.badge !== ""
                radius: 4
                color: Theme.accent
                implicitWidth: badgeText.implicitWidth + 10
                implicitHeight: 16
                Text { id: badgeText; anchors.centerIn: parent; text: card.badge; color: Theme.bg; font.pixelSize: 10; font.weight: Font.DemiBold }
            }
        }
        HoverHandler { id: cardHover; cursorShape: Qt.PointingHandCursor }
        TapHandler { onTapped: card.clicked() }
    }

    // ---- layout ------------------------------------------------------------

    property alias contentRoot: prefsBg
    Rectangle { id: prefsBg; anchors.fill: parent; color: Theme.panel }
    RowLayout {
        id: prefsRoot
        anchors.fill: parent
        // A thousandth of a pixel keeps text off exact half pixels at 130%.
        anchors.topMargin: 0.001
        spacing: 0

        // Sidebar.
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: 196
            color: Theme.chrome
            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                anchors.topMargin: 16
                spacing: 2
                Repeater {
                    model: [
                        { id: "appearance", label: "Appearance", glyph: "◐" },
                        { id: "text", label: "Text", glyph: "Aa" },
                        { id: "terminal", label: "Terminal", glyph: "❯_" },
                        { id: "window", label: "Window & Tabs", glyph: "▤" },
                        { id: "about", label: "About", glyph: "i" }
                    ]
                    delegate: Rectangle {
                        Layout.fillWidth: true
                        implicitHeight: 34
                        radius: 8
                        readonly property bool on: prefs.page === modelData.id
                        color: on ? Theme.tabHover : (sh.hovered ? Qt.rgba(Theme.tabHover.r, Theme.tabHover.g, Theme.tabHover.b, 0.6) : "transparent")
                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            spacing: 10
                            Text {
                                Layout.preferredWidth: 22
                                horizontalAlignment: Text.AlignHCenter
                                text: modelData.glyph
                                color: parent.parent.on ? Theme.accent : Theme.textDim
                                font.pixelSize: 13
                                font.family: "monospace"
                            }
                            Text {
                                Layout.fillWidth: true
                                text: modelData.label
                                color: Theme.text
                                font.pixelSize: 13
                                font.weight: parent.parent.on ? Font.DemiBold : Font.Normal
                            }
                        }
                        HoverHandler { id: sh }
                        TapHandler { onTapped: prefs.page = modelData.id }
                    }
                }
                Item { Layout.fillHeight: true }
            }
        }

        // Pages.
        ScrollView {
            id: scroller
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: availableWidth
            clip: true

            ColumnLayout {
                width: scroller.availableWidth
                spacing: 0

                // -- Appearance
                ColumnLayout {
                    visible: prefs.page === "appearance"
                    Layout.fillWidth: true
                    Layout.margins: 24
                    spacing: 14

                    SectionTitle { text: "Color theme" }
                    Row2 {
                        label: "Theme"
                        hint: prefs.settings.theme_mode === "system"
                              ? "Light and dark themes switch with the desktop. Pick one of each below."
                              : "One theme all the time."
                        Segmented {
                            options: [{ label: "Follow system", value: "system" }, { label: "Always the same", value: "fixed" }]
                            value: prefs.settings.theme_mode
                            onPicked: (v) => prefs.change("theme_mode", v)
                        }
                        Item { Layout.fillWidth: true }
                    }

                    Flow {
                        Layout.fillWidth: true
                        Layout.topMargin: 4
                        spacing: 14
                        Repeater {
                            model: prefs.themes
                            delegate: ThemeCard {
                                theme: modelData
                                readonly property bool isLight: prefs.settings.theme_light === modelData.name
                                readonly property bool isDark: prefs.settings.theme_dark === modelData.name
                                selected: prefs.settings.theme_mode === "system" ? (isLight || isDark) : prefs.settings.theme === modelData.name
                                badge: prefs.settings.theme_mode === "system" ? (isDark ? "DARK" : (isLight ? "LIGHT" : "")) : ""
                                onClicked: prefs.pick(modelData)
                                onRemoveClicked: prefs.controller.deleteTheme(modelData.name)
                            }
                        }
                    }

                    RowLayout {
                        spacing: 10
                        PlainButton { text: "Import .itermcolors…"; onClicked: importDialog.open() }
                        Hint { id: importStatus; text: "" }
                    }
                }

                // -- Text
                ColumnLayout {
                    visible: prefs.page === "text"
                    Layout.fillWidth: true
                    Layout.margins: 24
                    spacing: 16

                    SectionTitle { text: "Font" }
                    Row2 {
                        label: "Family"
                        hint: "Fixed-width fonts installed on this computer."
                        ComboBox {
                            id: fontBox
                            Layout.preferredWidth: 280
                            model: ["System monospace"].concat(prefs.fonts)
                            currentIndex: prefs.settings.font_family ? Math.max(0, prefs.fonts.indexOf(prefs.settings.font_family) + 1) : 0
                            onActivated: (i) => prefs.change("font_family", i === 0 ? "" : prefs.fonts[i - 1])
                        }
                        Item { Layout.fillWidth: true }
                    }
                    Row2 {
                        label: "Size"
                        Stepper {
                            value: prefs.settings.font_size
                            step: prefs.settings.font_size > 0 ? 0.5 : 11
                            from: 0
                            to: 48
                            display: prefs.settings.font_size > 0 ? prefs.settings.font_size + " pt" : "System"
                            onMoved: (v) => prefs.change("font_size", v)
                        }
                        Item { Layout.fillWidth: true }
                    }
                    Row2 {
                        label: "Line height"
                        hint: "Space between rows, as a multiple of the font's height."
                        Slider {
                            Layout.preferredWidth: 220
                            from: 0.9; to: 1.6; stepSize: 0.05
                            value: prefs.settings.line_height
                            onMoved: prefs.change("line_height", Math.round(value * 100) / 100)
                        }
                        Text { text: Number(prefs.settings.line_height).toFixed(2) + "×"; color: Theme.textDim; font.pixelSize: 12 }
                        Item { Layout.fillWidth: true }
                    }

                    // Preview in the current theme.
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.topMargin: 6
                        implicitHeight: previewCol.implicitHeight + 28
                        radius: 10
                        color: Theme.bg
                        border.color: Theme.divider
                        Column {
                            id: previewCol
                            x: 14; y: 14
                            spacing: (prefs.settings.line_height - 1) * 14
                            Repeater {
                                model: ["The quick brown fox jumps over the lazy dog", "0O 1lI| {}[]() => != <= === ~/src $ cargo build", "┌──┐ ✓ → λ 日本語"]
                                Text {
                                    text: modelData
                                    color: index === 1 ? Theme.term.ansi[2] : Theme.fg
                                    font.family: prefs.settings.font_family || "monospace"
                                    font.pointSize: prefs.settings.font_size > 0 ? prefs.settings.font_size : 11
                                }
                            }
                        }
                    }
                }

                // -- Terminal
                ColumnLayout {
                    visible: prefs.page === "terminal"
                    Layout.fillWidth: true
                    Layout.margins: 24
                    spacing: 16

                    SectionTitle { text: "Cursor" }
                    Row2 {
                        label: "Shape"
                        hint: "Programs can still change it (vim's insert mode, for example)."
                        Segmented {
                            options: [{ label: "Block", value: "block" }, { label: "Bar", value: "bar" }, { label: "Underline", value: "underline" }]
                            value: prefs.settings.cursor_style
                            onPicked: (v) => prefs.change("cursor_style", v)
                        }
                        Item { Layout.fillWidth: true }
                    }

                    SectionTitle { text: "Scrollback" }
                    Row2 {
                        label: "Lines kept"
                        hint: "For new tabs and panes."
                        Stepper {
                            value: prefs.settings.scrollback
                            step: prefs.settings.scrollback >= 10000 ? 10000 : 1000
                            from: 1000
                            to: 1000000
                            display: Number(prefs.settings.scrollback).toLocaleString(Qt.locale(), "f", 0)
                            onMoved: (v) => prefs.change("scrollback", Math.round(v))
                        }
                        Item { Layout.fillWidth: true }
                    }

                    SectionTitle { text: "Selection" }
                    Row2 {
                        label: "Copy on select"
                        hint: "Selected text goes straight to the clipboard. (The middle-click selection always gets it.)"
                        Toggle { checked: prefs.settings.copy_on_select; onToggled: (on) => prefs.change("copy_on_select", on) }
                        Item { Layout.fillWidth: true }
                    }
                }

                // -- Window
                ColumnLayout {
                    visible: prefs.page === "window"
                    Layout.fillWidth: true
                    Layout.margins: 24
                    spacing: 16

                    SectionTitle { text: "Tabs" }
                    Row2 {
                        label: "Always show the tab bar"
                        hint: "Off: it appears once there's a second tab (the menu is then in the right-click menu)."
                        Toggle { checked: prefs.settings.always_show_tabs; onToggled: (on) => prefs.change("always_show_tabs", on) }
                        Item { Layout.fillWidth: true }
                    }

                    SectionTitle { text: "Panes" }
                    Row2 {
                        label: "Dim inactive panes"
                        Toggle { checked: prefs.settings.dim_inactive; onToggled: (on) => prefs.change("dim_inactive", on) }
                        Item { Layout.fillWidth: true }
                    }
                    Row2 {
                        label: "Padding"
                        hint: "Space around the text."
                        Slider {
                            Layout.preferredWidth: 220
                            from: 0; to: 24; stepSize: 1
                            value: prefs.settings.padding
                            onMoved: prefs.change("padding", value)
                        }
                        Text { text: Math.round(prefs.settings.padding) + " px"; color: Theme.textDim; font.pixelSize: 12 }
                        Item { Layout.fillWidth: true }
                    }
                }

                // -- About
                ColumnLayout {
                    visible: prefs.page === "about"
                    Layout.fillWidth: true
                    Layout.margins: 24
                    spacing: 10
                    Text { text: "Tessera"; color: Theme.text; font.pixelSize: 24; font.weight: Font.DemiBold }
                    Hint { text: "A terminal in the spirit of iTerm2. Terminal emulation by libghostty-vt; interface in Qt Quick via cxx-qt." }
                    Hint { text: "Settings and imported themes: " + prefs.controller.configFolder() }
                }
            }
        }
    }

    FileDialog {
        id: importDialog
        title: "Import an iTerm2 color theme"
        nameFilters: ["iTerm2 colors (*.itermcolors)", "All files (*)"]
        onAccepted: {
            const r = prefs.controller.importTheme(selectedFile.toString())
            importStatus.text = r.startsWith("error:") ? r : ("Imported “" + r + "”")
        }
    }
}
