import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtQuick.Window
import Tessera

// Preferences, laid out like iTerm2's: General, Appearance, Profiles (with
// its tabs), Arrangements, Pointer. Every change applies at once and is saved.
Window {
    id: prefs
    property var controller
    property var settings: ({ profiles: [] })
    property bool systemDark: true
    property string activeProfileName: ""
    property string page: "general"
    property string editingProfile: ""
    property string profileTab: "general"
    signal changeSetting(string key, var value)
    signal changeProfile(string name, string key, var value)
    signal restoreArrangement(string name)
    signal saveArrangement(string name)

    title: "Tessera Preferences"
    width: 1200
    height: 820
    minimumWidth: 820
    minimumHeight: 560
    color: Theme.panel
    flags: Qt.Dialog

    readonly property var themes: { controller.revision; return JSON.parse(controller.themes()) }
    // The theme galleries' search and filter; long lists show a page at a time.
    property string themeQuery: ""
    property string themeKind: "all"     // "all", "dark", "light"
    function themeMatches(t) {
        if (themeKind === "dark" && !t.dark) return false
        if (themeKind === "light" && t.dark) return false
        const q = themeQuery.trim().toLowerCase()
        return q === "" || t.name.toLowerCase().indexOf(q) >= 0
    }
    readonly property var fonts: JSON.parse(controller.monospaceFamilies())
    readonly property var arrangements: { controller.revision; return JSON.parse(controller.arrangements()) }
    readonly property var prof: {
        for (const p of settings.profiles) if (p.name === editingProfile) return p
        return settings.profiles.length ? settings.profiles[0] : ({})
    }

    function showPage(p) {
        // "profiles/colors": a page and one of its tabs.
        if (p.indexOf("/") > 0) { profileTab = p.split("/")[1]; p = p.split("/")[0] }
        if (p !== "") page = p
        if (editingProfile === "" || !settings.profiles.some(x => x.name === editingProfile)) editingProfile = activeProfileName
        show()
        raise()
        requestActivate()
    }
    function showProfile(name) {
        editingProfile = name
        profileTab = "general"
        showPage("profiles")
    }
    function setP(key, value) { prefs.changeProfile(prof.name, key, value) }
    function setG(key, value) { prefs.changeSetting(key, value) }
    // ---- AI ---------------------------------------------------------------
    readonly property var ai: settings.ai || ({ providers: [] })
    function setAi(key, value) { const a = JSON.parse(JSON.stringify(ai)); a[key] = value; setG("ai", a) }
    property var aiModels: ({})        // provider id -> [model] or "error: ..."
    readonly property var aiPresets: { try { return JSON.parse(controller.aiPresets()) } catch (e) { return [] } }
    function refreshModels() {
        for (const p of (ai.providers || [])) controller.aiListModels(JSON.stringify(p))
    }
    readonly property var aiModelRefs: {
        const out = []
        for (const p of (ai.providers || [])) {
            const m = aiModels[p.id]
            if (Array.isArray(m)) for (const x of m) out.push(p.id + "/" + x)
        }
        return out
    }
    Connections {
        target: prefs.controller
        function onAiModelsReady(provider, json) {
            const m = Object.assign({}, prefs.aiModels)
            m[provider] = json.startsWith("error:") ? json : JSON.parse(json)
            prefs.aiModels = m
        }
    }
    onPageChanged: { if (page === "ai") refreshModels(); if (page === "prompt") loadPrompt() }
    // ---- the prompt builder -------------------------------------------------
    property string promptToml: ""
    property var promptPreviews: ({})     // key -> {lines, right}
    readonly property var promptPresets: { try { return JSON.parse(controller.promptPresets()) } catch (e) { return [] } }
    readonly property var promptTheme: themes.find(t => t.name === prof.theme) || ({})
    function loadPrompt() {
        promptToml = controller.promptTesseraConfig()
        previewPrompt("current", promptToml)
        for (const p of promptPresets) previewPrompt("preset-" + p, controller.promptPreset(p))
    }
    function previewPrompt(key, toml) { controller.requestPromptPreview("pp-" + key, toml, "", 96) }
    function setPromptToml(t) { promptToml = t; promptPreview.restart() }
    Timer { id: promptPreview; interval: 300; onTriggered: prefs.previewPrompt("current", prefs.promptToml) }
    Connections {
        target: prefs.controller
        function onStarshipReady(key, json) {
            if (!key.startsWith("pp-")) return
            const m = Object.assign({}, prefs.promptPreviews)
            try { m[key.slice(3)] = JSON.parse(json) } catch (e) {}
            prefs.promptPreviews = m
        }
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
        Layout.topMargin: 10
    }

    component Hint: Text {
        color: Theme.textDim
        font.pixelSize: 12
        wrapMode: Text.WordWrap
        Layout.fillWidth: true
    }

    // A labelled row: label (and hint) on the left, controls on the right.
    component Row2: RowLayout {
        property alias label: lbl.text
        property alias hint: hintText.text
        Layout.fillWidth: true
        spacing: 16
        ColumnLayout {
            Layout.preferredWidth: 200
            Layout.maximumWidth: 200
            Layout.alignment: Qt.AlignTop
            spacing: 2
            Text { id: lbl; color: Theme.text; font.pixelSize: 13; wrapMode: Text.WordWrap; Layout.preferredWidth: 200 }
            Text { id: hintText; visible: text !== ""; color: Theme.textDim; font.pixelSize: 11; wrapMode: Text.WordWrap; Layout.preferredWidth: 200 }
        }
    }

    // A toggle row: the switch on the left, label and hint beside it.
    component ToggleRow: RowLayout {
        id: tr
        property alias label: tlbl.text
        property alias hint: thint.text
        property bool checked: false
        signal toggled(bool on)
        Layout.fillWidth: true
        spacing: 12
        Toggle { Layout.alignment: Qt.AlignTop; checked: tr.checked; onToggled: (on) => tr.toggled(on) }
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Text { id: tlbl; color: Theme.text; font.pixelSize: 13; Layout.fillWidth: true; wrapMode: Text.WordWrap }
            Text { id: thint; visible: text !== ""; color: Theme.textDim; font.pixelSize: 11; Layout.fillWidth: true; wrapMode: Text.WordWrap }
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
                    width: Math.max(64, t.implicitWidth + 22)
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

    component SliderRow: Row2 {
        id: sr
        property real from: 0
        property real to: 1
        property real stepSize: 0.05
        property real value: 0
        property string suffix: ""
        property int decimals: 2
        signal moved(real value)
        Slider {
            Layout.preferredWidth: 220
            from: sr.from; to: sr.to; stepSize: sr.stepSize
            value: sr.value
            onMoved: sr.moved(Math.round(value / sr.stepSize) * sr.stepSize)
        }
        Text { text: Number(sr.value).toFixed(sr.decimals) + sr.suffix; color: Theme.textDim; font.pixelSize: 12 }
        Item { Layout.fillWidth: true }
    }

    // A text field that saves when editing finishes.
    component Field: TextField {
        id: fld
        property string saved: ""
        signal commit(string text)
        text: saved
        Layout.fillWidth: true
        Layout.maximumWidth: 360
        Layout.minimumWidth: 140
        selectByMouse: true
        onEditingFinished: if (text !== saved) fld.commit(text)
        background: Rectangle { radius: 7; color: Theme.field; border.color: fld.activeFocus ? Theme.accent : Theme.divider }
        color: Theme.text
    }

    component PlainButton: Rectangle {
        id: pb
        property alias text: pbText.text
        property bool enabled2: true
        signal clicked()
        implicitWidth: pbText.implicitWidth + 28
        implicitHeight: 30
        radius: 8
        opacity: enabled2 ? 1 : 0.45
        color: pbh.hovered && enabled2 ? Theme.tabHover : Theme.field
        border.color: Theme.divider
        Text { id: pbText; anchors.centerIn: parent; color: Theme.text; font.pixelSize: 12 }
        HoverHandler { id: pbh }
        TapHandler { onTapped: if (pb.enabled2) pb.clicked() }
    }

    // Lists in profiles/settings: replace the whole array.
    function editItem(list, i, field, value) {
        const copy = JSON.parse(JSON.stringify(list || []))
        copy[i][field] = value
        return copy
    }
    function removeItem(list, i) {
        const copy = JSON.parse(JSON.stringify(list || []))
        copy.splice(i, 1)
        return copy
    }

    // Records a key combination: click it, then press the keys.
    component KeyField: Rectangle {
        id: kf
        property string value: ""
        property bool recording: false
        signal recorded(string seq)
        implicitWidth: 150
        implicitHeight: 30
        radius: 7
        color: Theme.field
        border.color: recording || activeFocus ? Theme.accent : Theme.divider
        Text {
            anchors.centerIn: parent
            text: kf.recording ? "Press keys…" : (kf.value || "Click to record")
            color: kf.value && !kf.recording ? Theme.text : Theme.textDim
            font.pixelSize: 12
        }
        TapHandler { onTapped: { kf.recording = true; kf.forceActiveFocus() } }
        function keyName(e) {
            const special = {}
            special[Qt.Key_Up] = "Up"; special[Qt.Key_Down] = "Down"; special[Qt.Key_Left] = "Left"; special[Qt.Key_Right] = "Right"
            special[Qt.Key_Home] = "Home"; special[Qt.Key_End] = "End"; special[Qt.Key_PageUp] = "PgUp"; special[Qt.Key_PageDown] = "PgDown"
            special[Qt.Key_Tab] = "Tab"; special[Qt.Key_Backtab] = "Tab"; special[Qt.Key_Return] = "Return"; special[Qt.Key_Enter] = "Enter"
            special[Qt.Key_Escape] = "Esc"; special[Qt.Key_Backspace] = "Backspace"; special[Qt.Key_Delete] = "Del"
            special[Qt.Key_Insert] = "Ins"; special[Qt.Key_Space] = "Space"
            if (special[e.key]) return special[e.key]
            if (e.key >= Qt.Key_F1 && e.key <= Qt.Key_F24) return "F" + (e.key - Qt.Key_F1 + 1)
            if (e.key >= 0x20 && e.key < 0x7f) return String.fromCharCode(e.key).toUpperCase()
            return ""
        }
        Keys.onPressed: (e) => {
            if (!kf.recording) return
            e.accepted = true
            if ([Qt.Key_Control, Qt.Key_Shift, Qt.Key_Alt, Qt.Key_Meta].indexOf(e.key) >= 0) return
            const k = keyName(e)
            if (k === "") return
            let mods = ""
            if (e.modifiers & Qt.ControlModifier) mods += "Ctrl+"
            if (e.modifiers & Qt.AltModifier) mods += "Alt+"
            if (e.modifiers & Qt.ShiftModifier) mods += "Shift+"
            if (e.modifiers & Qt.MetaModifier) mods += "Meta+"
            kf.recording = false
            kf.recorded(mods + k)
        }
        onActiveFocusChanged: if (!activeFocus) recording = false
    }

    component SmallButton: Rectangle {
        id: smb
        property string glyph: "×"
        signal hit()
        implicitWidth: 26; implicitHeight: 26; radius: 6
        color: smh.hovered ? Theme.tabHover : "transparent"
        Text { anchors.centerIn: parent; text: smb.glyph; color: Theme.textDim; font.pixelSize: 14 }
        HoverHandler { id: smh }
        TapHandler { onTapped: smb.hit() }
    }

    component ChoiceBox: ComboBox {
        property var values: []
        property string value: ""
        signal chosen(string v)
        implicitWidth: 170
        model: values.map(v => v[1])
        currentIndex: Math.max(0, values.findIndex(v => v[0] === value))
        onActivated: (i) => chosen(values[i][0])
    }
    // A theme as a little terminal: prompt, a command, the ANSI colors.
    component ThemeCard: Item {
        id: card
        property var theme
        property bool selected: false
        property string badge: ""
        signal clicked()
        signal removeClicked()
        width: 176
        height: 124
        Rectangle {
            id: swatch
            width: parent.width
            height: 94
            radius: 10
            color: card.theme.background
            border.width: card.selected ? 2 : 1
            border.color: card.selected ? Theme.accent : Theme.divider
            Column {
                x: 11; y: 9
                spacing: 3
                Row {
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
                    Text { text: "error"; color: card.theme.ansi[1]; font.family: "monospace"; font.pixelSize: 11 }
                    Text { text: ": "; color: card.theme.foreground; font.family: "monospace"; font.pixelSize: 11 }
                    Text { text: "warn"; color: card.theme.ansi[3]; font.family: "monospace"; font.pixelSize: 11 }
                    Rectangle { width: 7; height: 13; color: card.theme.cursor }
                }
            }
            Row {
                x: 11
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 9
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
            Text { Layout.fillWidth: true; text: card.theme.name; color: Theme.text; font.pixelSize: 12; elide: Text.ElideRight }
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
    Rectangle {
        id: prefsBg
        anchors.fill: parent
        color: Theme.panel

        RowLayout {
            anchors.fill: parent
            // A thousandth of a pixel keeps text off exact half pixels at 130%.
            anchors.topMargin: 0.001
            spacing: 0

            // Panels.
            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: 190
                color: Theme.chrome
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    anchors.topMargin: 16
                    spacing: 2
                    Repeater {
                        model: [
                            { id: "general", label: "General", glyph: "⚙" },
                            { id: "appearance", label: "Appearance", glyph: "◐" },
                            { id: "profiles", label: "Profiles", glyph: "❯_" },
                            { id: "arrangements", label: "Arrangements", glyph: "\u25a4" },
                            { id: "snippets", label: "Snippets", glyph: "\u2702" },
                            { id: "prompt", label: "Prompt", glyph: "\u276f" },
                            { id: "ai", label: "AI", glyph: "\u2726" },
                            { id: "pointer", label: "Pointer", glyph: "↖" },
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

            // Profile list (Profiles panel only).
            Rectangle {
                visible: prefs.page === "profiles"
                Layout.fillHeight: true
                Layout.preferredWidth: 200
                color: Qt.rgba(Theme.chrome.r, Theme.chrome.g, Theme.chrome.b, 0.5)
                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: 10
                    anchors.topMargin: 16
                    spacing: 2
                    Text { text: "Profiles"; color: Theme.textDim; font.pixelSize: 11; Layout.leftMargin: 6; Layout.bottomMargin: 4 }
                    Repeater {
                        model: prefs.settings.profiles
                        delegate: Rectangle {
                            Layout.fillWidth: true
                            implicitHeight: 32
                            radius: 7
                            readonly property bool on: modelData.name === prefs.prof.name
                            color: on ? Theme.accent : (ph2.hovered ? Theme.tabHover : "transparent")
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 10
                                anchors.rightMargin: 8
                                Text { Layout.fillWidth: true; text: modelData.name; color: parent.parent.on ? Theme.bg : Theme.text; font.pixelSize: 13; elide: Text.ElideRight }
                                Text { visible: modelData.name === prefs.settings.default_profile; text: "★"; color: parent.parent.on ? Theme.bg : Theme.accent; font.pixelSize: 12 }
                            }
                            HoverHandler { id: ph2 }
                            TapHandler { onTapped: prefs.editingProfile = modelData.name }
                        }
                    }
                    Item { Layout.fillHeight: true }
                    RowLayout {
                        spacing: 4
                        PlainButton { text: "+"; implicitWidth: 32; onClicked: prefs.editingProfile = prefs.controller.addProfile("") }
                        PlainButton { text: "−"; implicitWidth: 32; enabled2: prefs.settings.profiles.length > 1; onClicked: prefs.controller.removeProfile(prefs.prof.name) }
                        PlainButton { text: "Duplicate"; onClicked: prefs.editingProfile = prefs.controller.addProfile(prefs.prof.name) }
                    }
                    PlainButton {
                        Layout.fillWidth: true
                        text: "★  Set as Default"
                        enabled2: prefs.prof.name !== prefs.settings.default_profile
                        onClicked: prefs.setG("default_profile", prefs.prof.name)
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                spacing: 0

                // Profile tabs.
                Item {
                    visible: prefs.page === "profiles"
                    Layout.fillWidth: true
                    Layout.preferredHeight: 46
                    Segmented {
                        anchors.left: parent.left
                        anchors.leftMargin: 24
                        anchors.bottom: parent.bottom
                        options: [
                            { label: "General", value: "general" }, { label: "Colors", value: "colors" },
                            { label: "Text", value: "text" }, { label: "Window", value: "window" },
                            { label: "Terminal", value: "terminal" }, { label: "Session", value: "session" },
                            { label: "Keys", value: "keys" }, { label: "Advanced", value: "advanced" }
                        ]
                        value: prefs.profileTab
                        onPicked: (v) => prefs.profileTab = v
                    }
                }

                ScrollView {
                    id: scroller
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    contentWidth: availableWidth
                    clip: true
                    Component.onCompleted: {
                        const f = scroller.contentItem
                        f.maximumFlickVelocity = 12000
                        f.flickDeceleration = 1800
                    }
                    MomentumWheel { flick: scroller.contentItem }

                    ColumnLayout {
                        width: scroller.availableWidth
                        spacing: 0

                        // ================= General =================
                        ColumnLayout {
                            visible: prefs.page === "general"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Startup" }
                            ToggleRow {
                                label: "Restore sessions"
                                hint: "Closing the window keeps its shells running; they come back, with their tabs and splits, the next time Tessera opens. Off: closing the window ends them."
                                checked: prefs.settings.restore_sessions
                                onToggled: (on) => prefs.setG("restore_sessions", on)
                            }

                            SectionTitle { text: "Closing" }
                            ToggleRow { label: "Confirm closing multiple sessions"; checked: prefs.settings.confirm_close_multiple; onToggled: (on) => prefs.setG("confirm_close_multiple", on) }
                            ToggleRow { label: "Confirm closing the window"; checked: prefs.settings.confirm_quit; onToggled: (on) => prefs.setG("confirm_quit", on) }
                            SectionTitle { text: "Input editor" }
                            ToggleRow {
                                label: "Edit commands in an editor under the terminal"
                                hint: "At a shell prompt (with shell integration) and in Python or Node: Enter runs, Shift+Enter adds a line, with highlighting and syntax checking."
                                checked: prefs.settings.input_editor !== false
                                onToggled: (on) => prefs.setG("input_editor", on)
                            }
                            Row2 {
                                label: "Editor keys"
                                hint: "Emacs: the shell's own (readline) keys. vi: Esc for normal mode. VS Code: Ctrl+Z, Alt+Up/Down moves lines, Ctrl+/ comments."
                                ChoiceBox {
                                    values: [["emacs", "Emacs (readline)"], ["vi", "vi"], ["vscode", "VS Code"]]
                                    value: prefs.settings.editor_keys || "emacs"
                                    onChosen: (v) => prefs.setG("editor_keys", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            ToggleRow {
                                label: "Prompt above the editor"
                                hint: "Folder, git, status and more, drawn by Tessera above the editor."
                                checked: prefs.settings.editor_prompt !== false
                                onToggled: (on) => prefs.setG("editor_prompt", on)
                            }
                            Row2 {
                                label: "Prompt from"
                                hint: prefs.controller.hasStarship() ? "Starship uses your starship.toml (every module, your colors)." : "Starship isn't installed: the built-in segments below."
                                Segmented {
                                    options: [{ label: "Auto", value: "auto" }, { label: "Starship", value: "starship" }, { label: "Built-in", value: "builtin" }]
                                    value: prefs.settings.prompt_engine || "auto"
                                    onPicked: (v) => prefs.setG("prompt_engine", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            // The built-in prompt's segments, in order; "right side" moves the rest right.
                            Row2 {
                                label: "Built-in segments"
                                hint: "In order, left to right. Everything after \u201cright side\u201d is on the right."
                                Flow {
                                    Layout.fillWidth: true
                                    Layout.maximumWidth: 560
                                    spacing: 6
                                    readonly property var names: ({ dir: "Folder", git: "Git", venv: "Python venv", nix: "Nix shell", status: "Exit status", duration: "Duration", kube: "Kubernetes", aws: "AWS", gcloud: "gcloud", host: "user@host (SSH)", user: "User", time: "Time", "|": "\u2192 right side" })
                                    readonly property var segs: prefs.settings.prompt_segments || []
                                    Repeater {
                                        model: parent.segs
                                        delegate: Rectangle {
                                            required property string modelData
                                            required property int index
                                            readonly property var segs: parent.segs
                                            height: 26
                                            width: segRow.implicitWidth + 12
                                            radius: 13
                                            color: modelData === "|" ? "transparent" : Theme.field
                                            border.color: Theme.divider
                                            Row {
                                                id: segRow
                                                anchors.centerIn: parent
                                                spacing: 4
                                                Text { text: "\u2039"; color: Theme.textDim; font.pixelSize: 14; visible: index > 0
                                                       TapHandler { onTapped: { const a = segs.slice(); a.splice(index - 1, 0, a.splice(index, 1)[0]); prefs.setG("prompt_segments", a) } } }
                                                Text { text: parent.parent.parent.names[modelData] || modelData; color: Theme.text; font.pixelSize: 12; anchors.verticalCenter: parent.verticalCenter }
                                                Text { text: "\u203a"; color: Theme.textDim; font.pixelSize: 14; visible: index < segs.length - 1
                                                       TapHandler { onTapped: { const a = segs.slice(); a.splice(index + 1, 0, a.splice(index, 1)[0]); prefs.setG("prompt_segments", a) } } }
                                                Text { text: "\u00d7"; color: Theme.textDim; font.pixelSize: 13; visible: modelData !== "|"
                                                       TapHandler { onTapped: { const a = segs.slice(); a.splice(index, 1); prefs.setG("prompt_segments", a) } } }
                                            }
                                        }
                                    }
                                    ChoiceBox {
                                        implicitWidth: 150
                                        values: [["", "Add\u2026"]].concat(Object.keys(parent.names).filter(k => parent.segs.indexOf(k) < 0).map(k => [k, parent.names[k]]))
                                        value: ""
                                        onChosen: (v) => { if (v !== "") { prefs.setG("prompt_segments", parent.segs.concat([v])); currentIndex = 0 } }
                                    }
                                }
                            }
                            Row2 {
                                label: "Languages"
                                hint: prefs.controller.helpStatus.indexOf("grammars") >= 0 || prefs.controller.helpStatus.indexOf("languages") >= 0 ? prefs.controller.helpStatus
                                      : (18 + prefs.controller.grammarCount()) + " languages for highlighting and checks: 18 built in, the rest from Helix's tree-sitter grammars, each fetched the first time it's used (Helix's archive, about 16 MB, once). Download All gets them all for working offline (about 185 MB)."
                                PlainButton {
                                    text: "Download All"
                                    enabled2: !prefs.controller.helpStatus.startsWith("Downloading")
                                    onClicked: prefs.controller.downloadGrammars()
                                }
                                Item { Layout.fillWidth: true }
                            }
                            Row2 {
                                label: "Command help"
                                hint: prefs.controller.helpStatus !== "" ? prefs.controller.helpStatus
                                      : "Completions show help from man and info pages, tldr-pages (examples, fetched once) and fish-shell's completions (subcommands and options for thousands of commands)."
                                PlainButton {
                                    text: "Download / Update"
                                    enabled2: !prefs.controller.helpStatus.startsWith("Downloading")
                                    onClicked: prefs.controller.downloadCommandHelp()
                                }
                                Item { Layout.fillWidth: true }
                            }
                            Row2 {
                                label: "Undo Close"
                                hint: "A closed tab or pane can be brought back (Ctrl+Shift+Z) for this long; its shell keeps running meanwhile."
                                Stepper {
                                    value: prefs.settings.undo_close_seconds
                                    step: 5; from: 0; to: 600
                                    display: prefs.settings.undo_close_seconds > 0 ? prefs.settings.undo_close_seconds + " s" : "Off"
                                    onMoved: (v) => prefs.setG("undo_close_seconds", Math.round(v))
                                }
                                Item { Layout.fillWidth: true }
                            }

                            SectionTitle { text: "Selection" }
                            ToggleRow { label: "Copy to clipboard on selection"; hint: "The middle-click selection always gets it."; checked: prefs.settings.copy_on_select; onToggled: (on) => prefs.setG("copy_on_select", on) }
                            ToggleRow { label: "Copied text includes trailing newline"; checked: prefs.settings.copy_trailing_newline; onToggled: (on) => prefs.setG("copy_trailing_newline", on) }
                            ToggleRow {
                                label: "Applications in terminal may access clipboard"
                                hint: "Programs may set the clipboard with an escape sequence (OSC 52). Reading it is never allowed."
                                checked: prefs.settings.clipboard_access
                                onToggled: (on) => prefs.setG("clipboard_access", on)
                            }
                            Row2 {
                                label: "Characters considered part of a word"
                                hint: "For double-click selection, besides letters and digits."
                                Field { saved: prefs.settings.word_chars; onCommit: (t) => prefs.setG("word_chars", t); Layout.preferredWidth: 160; font.family: "monospace" }
                                Item { Layout.fillWidth: true }
                            }

                            SectionTitle { text: "Window" }
                            ToggleRow { label: "Each tab may have its own title"; hint: "A program's title change renames only its own tab."; checked: prefs.settings.tab_titles_per_tab; onToggled: (on) => prefs.setG("tab_titles_per_tab", on) }

                            SectionTitle { text: "Paste" }
                            ToggleRow { label: "Warn before multi-line paste"; checked: prefs.settings.warn_multiline_paste; onToggled: (on) => prefs.setG("warn_multiline_paste", on) }
                        }

                        // ================= Appearance =================
                        ColumnLayout {
                            visible: prefs.page === "appearance"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Tabs" }
                            Row2 {
                                label: "Tab bar location"
                                Segmented {
                                    options: [{ label: "Top", value: "top" }, { label: "Bottom", value: "bottom" }]
                                    value: prefs.settings.tab_position
                                    onPicked: (v) => prefs.setG("tab_position", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            ToggleRow {
                                label: "Show tab bar even when there is only one tab"
                                hint: "It holds the ☰ menu. Hidden, the menu is still in the right-click menu and Ctrl+,."
                                checked: prefs.settings.always_show_tabs
                                onToggled: (on) => prefs.setG("always_show_tabs", on)
                            }
                            ToggleRow { label: "Show tab numbers"; hint: "Alt+number selects a tab."; checked: prefs.settings.show_tab_numbers; onToggled: (on) => prefs.setG("show_tab_numbers", on) }
                            ToggleRow { label: "Tabs have close buttons"; hint: "Off: they show on hover."; checked: prefs.settings.tab_close_buttons; onToggled: (on) => prefs.setG("tab_close_buttons", on) }
                            ToggleRow { label: "Show activity indicator"; hint: "A dot on background tabs with new output."; checked: prefs.settings.show_activity; onToggled: (on) => prefs.setG("show_activity", on) }
                            ToggleRow { label: "Stretch tabs to fill bar"; checked: prefs.settings.stretch_tabs; onToggled: (on) => prefs.setG("stretch_tabs", on) }

                            SectionTitle { text: "Panes" }
                            ToggleRow { label: "Show per-pane title bar with split panes"; checked: prefs.settings.pane_titles; onToggled: (on) => prefs.setG("pane_titles", on) }

                            SectionTitle { text: "Dimming" }
                            ToggleRow { label: "Dim inactive split panes"; checked: prefs.settings.dim_inactive; onToggled: (on) => prefs.setG("dim_inactive", on) }
                            ToggleRow { label: "Dim background windows"; checked: prefs.settings.dim_background_windows; onToggled: (on) => prefs.setG("dim_background_windows", on) }
                            SliderRow {
                                label: "Dimming amount"
                                from: 0.05; to: 0.6; stepSize: 0.01
                                value: prefs.settings.dim_amount
                                onMoved: (v) => prefs.setG("dim_amount", v)
                            }
                        }

                        // ================= Profiles: General =================
                        ColumnLayout {
                            visible: prefs.page === "profiles" && prefs.profileTab === "general"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            Row2 {
                                label: "Name"
                                Field { saved: prefs.prof.name || ""; onCommit: (t) => { if (prefs.controller.setProfile(prefs.prof.name, "name", JSON.stringify(t))) prefs.editingProfile = t } }
                                Item { Layout.fillWidth: true }
                            }
                            SectionTitle { text: "Command" }
                            Row2 {
                                label: "Run"
                                hint: "A command runs through your login shell."
                                ColumnLayout {
                                    spacing: 8
                                    Segmented {
                                        options: [{ label: "Login shell", value: "login" }, { label: "Command", value: "custom" }]
                                        value: prefs.prof.command_mode
                                        onPicked: (v) => prefs.setP("command_mode", v)
                                    }
                                    Field {
                                        visible: prefs.prof.command_mode === "custom"
                                        saved: prefs.prof.command || ""
                                        placeholderText: "e.g. ssh server, htop, nvim"
                                        onCommit: (t) => prefs.setP("command", t)
                                    }
                                }
                                Item { Layout.fillWidth: true }
                            }
                            Row2 {
                                label: "Send text at start"
                                hint: "Typed into new sessions (a newline is added)."
                                Field { saved: prefs.prof.send_text || ""; onCommit: (t) => prefs.setP("send_text", t) }
                                Item { Layout.fillWidth: true }
                            }
                            SectionTitle { text: "Working Directory" }
                            Row2 {
                                label: "Start in"
                                ColumnLayout {
                                    spacing: 8
                                    Segmented {
                                        options: [{ label: "Home", value: "home" }, { label: "Reuse previous", value: "recycle" }, { label: "Directory", value: "custom" }]
                                        value: prefs.prof.initial_dir_mode
                                        onPicked: (v) => prefs.setP("initial_dir_mode", v)
                                    }
                                    Field {
                                        visible: prefs.prof.initial_dir_mode === "custom"
                                        saved: prefs.prof.directory || ""
                                        placeholderText: "/path/to/folder"
                                        onCommit: (t) => prefs.setP("directory", t)
                                    }
                                }
                                Item { Layout.fillWidth: true }
                            }
                            SectionTitle { text: "Title and Badge" }
                            Row2 {
                                label: "Tab title"
                                hint: "Empty: the program's title, or the folder name."
                                Field { saved: prefs.prof.tab_title || ""; onCommit: (t) => prefs.setP("tab_title", t) }
                                Item { Layout.fillWidth: true }
                            }
                            Row2 {
                                label: "Badge"
                                hint: "Large text behind the terminal. {title}, {cwd} and {profile} are filled in."
                                Field { saved: prefs.prof.badge || ""; onCommit: (t) => prefs.setP("badge", t) }
                                Item { Layout.fillWidth: true }
                            }
                        }

                        // ================= Profiles: Colors =================
                        ColumnLayout {
                            visible: prefs.page === "profiles" && prefs.profileTab === "colors"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            ToggleRow {
                                label: "Use separate themes for light and dark mode"
                                hint: prefs.prof.theme_mode === "system"
                                      ? "Switches with the desktop, which is " + (prefs.systemDark ? "dark" : "light") + " now."
                                      : "Off: one theme all the time."
                                checked: prefs.prof.theme_mode === "system"
                                onToggled: (on) => prefs.setP("theme_mode", on ? "system" : "fixed")
                            }
                            RowLayout {
                                Layout.fillWidth: true
                                spacing: 10
                                TextField {
                                    id: themeSearch
                                    Layout.preferredWidth: 240
                                    placeholderText: "Search " + prefs.themes.length + " themes"
                                    placeholderTextColor: Theme.textDim
                                    selectByMouse: true
                                    color: Theme.text
                                    onTextChanged: prefs.themeQuery = text
                                    background: Rectangle { radius: 7; color: Theme.field; border.color: themeSearch.activeFocus ? Theme.accent : Theme.divider }
                                }
                                Segmented {
                                    options: [{ label: "All", value: "all" }, { label: "Dark", value: "dark" }, { label: "Light", value: "light" }]
                                    value: prefs.themeKind
                                    onPicked: (v) => prefs.themeKind = v
                                }
                                Item { Layout.fillWidth: true }
                            }
                            Repeater {
                                // One gallery, or one each for dark and light mode.
                                model: prefs.prof.theme_mode === "system"
                                       ? [{ key: "theme_dark", label: "When the desktop is dark", dark: true },
                                          { key: "theme_light", label: "When the desktop is light", dark: false }]
                                       : [{ key: "theme", label: "", dark: null }]
                                delegate: ColumnLayout {
                                    id: gallery
                                    Layout.fillWidth: true
                                    spacing: 8
                                    readonly property var slot: modelData
                                    Text {
                                        visible: gallery.slot.label !== ""
                                        text: gallery.slot.label + ((gallery.slot.dark === prefs.systemDark) ? "  \u2022  in use now" : "")
                                        color: Theme.text
                                        font.pixelSize: 13
                                        font.weight: Font.DemiBold
                                        Layout.topMargin: 8
                                    }
                                    // Themes of the matching kind first; every theme is
                                    // allowed. The chosen one always shows.
                                    readonly property var all: {
                                        const l = (gallery.slot.dark === null ? prefs.themes
                                                   : prefs.themes.filter(t => t.dark === gallery.slot.dark).concat(prefs.themes.filter(t => t.dark !== gallery.slot.dark)))
                                                  .filter(t => prefs.themeMatches(t))
                                        return l
                                    }
                                    Hint {
                                        visible: gallery.all.length === 0
                                        text: "No themes match."
                                    }
                                    // Every theme in one scrolling grid (only the visible
                                    // cards exist at a time).
                                    GridView {
                                        id: grid
                                        Layout.fillWidth: true
                                        Layout.preferredHeight: prefs.prof.theme_mode === "system" ? 340 : 540
                                        visible: gallery.all.length > 0
                                        clip: true
                                        cellWidth: 188
                                        cellHeight: 136
                                        boundsBehavior: Flickable.StopAtBounds
                                        maximumFlickVelocity: 12000
                                        flickDeceleration: 1800
                                        MomentumWheel { flick: grid }
                                        model: gallery.all
                                        reuseItems: true
                                        delegate: ThemeCard {
                                            theme: modelData
                                            selected: prefs.prof[gallery.slot.key] === modelData.name
                                            onClicked: prefs.setP(gallery.slot.key, modelData.name)
                                            onRemoveClicked: prefs.controller.deleteTheme(modelData.name)
                                        }
                                        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded; minimumSize: 0.06 }
                                        // Opening the page shows the chosen theme.
                                        function showCurrent() {
                                            const i = gallery.all.findIndex(t => t.name === prefs.prof[gallery.slot.key])
                                            if (i >= 0) positionViewAtIndex(i, GridView.Contain)
                                        }
                                        Component.onCompleted: Qt.callLater(showCurrent)
                                        onVisibleChanged: if (visible) Qt.callLater(showCurrent)
                                    }
                                    Hint {
                                        text: gallery.all.length + (gallery.all.length === 1 ? " theme" : " themes")
                                              + (prefs.themeQuery !== "" || prefs.themeKind !== "all" ? " match" : "")
                                    }
                                }
                            }
                            RowLayout {
                                spacing: 10
                                PlainButton { text: "Import .itermcolors…"; onClicked: importDialog.open() }
                                PlainButton {
                                    text: "Get All iTerm2-Color-Schemes"
                                    enabled2: !prefs.controller.packStatus.startsWith("Downloading")
                                    onClicked: prefs.controller.downloadThemePack()
                                }
                                Hint { id: importStatus; text: prefs.controller.packStatus }
                            }
                            SectionTitle { text: "Text" }
                            SliderRow {
                                label: "Minimum contrast"
                                hint: "Text too close to its background is moved toward black or white until it's readable. 0 leaves every color as the theme has it."
                                from: 0; to: 1; stepSize: 0.05
                                value: prefs.prof.minimum_contrast
                                onMoved: (v) => prefs.setP("minimum_contrast", v)
                            }
                            SliderRow {
                                label: "Faint text opacity"
                                from: 0.1; to: 1; stepSize: 0.05
                                value: prefs.prof.faint_opacity
                                onMoved: (v) => prefs.setP("faint_opacity", v)
                            }
                            ToggleRow { label: "Cursor guide"; hint: "A faint band along the cursor's row."; checked: prefs.prof.cursor_guide; onToggled: (on) => prefs.setP("cursor_guide", on) }
                        }

                        // ================= Profiles: Text =================
                        ColumnLayout {
                            visible: prefs.page === "profiles" && prefs.profileTab === "text"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Cursor" }
                            Row2 {
                                label: "Shape"
                                hint: "Programs can still change it (vim's insert mode, for example)."
                                Segmented {
                                    options: [{ label: "Block", value: "block" }, { label: "Bar", value: "bar" }, { label: "Underline", value: "underline" }]
                                    value: prefs.prof.cursor_style
                                    onPicked: (v) => prefs.setP("cursor_style", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            ToggleRow { label: "Blinking cursor"; checked: prefs.prof.cursor_blink; onToggled: (on) => prefs.setP("cursor_blink", on) }

                            SectionTitle { text: "Font" }
                            Row2 {
                                label: "Family"
                                hint: "Fixed-width fonts installed on this computer."
                                ComboBox {
                                    Layout.preferredWidth: 280
                                    model: ["System monospace"].concat(prefs.fonts)
                                    currentIndex: prefs.prof.font_family ? Math.max(0, prefs.fonts.indexOf(prefs.prof.font_family) + 1) : 0
                                    onActivated: (i) => prefs.setP("font_family", i === 0 ? "" : prefs.fonts[i - 1])
                                }
                                Item { Layout.fillWidth: true }
                            }
                            Row2 {
                                label: "Size"
                                Stepper {
                                    value: prefs.prof.font_size
                                    step: prefs.prof.font_size > 0 ? 0.5 : 11
                                    from: 0; to: 48
                                    display: prefs.prof.font_size > 0 ? prefs.prof.font_size + " pt" : "System"
                                    onMoved: (v) => prefs.setP("font_size", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            SliderRow {
                                label: "Line spacing"
                                hint: "Row height as a multiple of the font's."
                                from: 0.9; to: 1.6; stepSize: 0.05
                                value: prefs.prof.line_height; suffix: "×"
                                onMoved: (v) => prefs.setP("line_height", v)
                            }
                            SliderRow {
                                label: "Character spacing"
                                from: 0.85; to: 1.4; stepSize: 0.05
                                value: prefs.prof.char_spacing; suffix: "×"
                                onMoved: (v) => prefs.setP("char_spacing", v)
                            }
                            ToggleRow { label: "Draw bold text in bold font"; checked: prefs.prof.bold_font; onToggled: (on) => prefs.setP("bold_font", on) }
                            ToggleRow { label: "Italic text"; hint: "Off: italic text is drawn upright."; checked: prefs.prof.italic; onToggled: (on) => prefs.setP("italic", on) }

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
                                    spacing: (prefs.prof.line_height - 1) * 14
                                    Repeater {
                                        model: ["The quick brown fox jumps over the lazy dog", "0O 1lI| {}[]() => != <= === ~/src $ cargo build", "┌──┐ ✓ → λ 日本語"]
                                        Text {
                                            text: modelData
                                            color: index === 1 ? Theme.term.ansi[2] : Theme.fg
                                            font.family: prefs.prof.font_family || "monospace"
                                            font.pointSize: prefs.prof.font_size > 0 ? prefs.prof.font_size : 11
                                            font.letterSpacing: (prefs.prof.char_spacing - 1) * 8
                                        }
                                    }
                                }
                            }
                        }

                        // ================= Profiles: Window =================
                        ColumnLayout {
                            visible: prefs.page === "profiles" && prefs.profileTab === "window"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Transparency" }
                            SliderRow {
                                label: "Transparency"
                                hint: "Needs a compositing desktop. View › Use Transparency turns it off for a while."
                                from: 0; to: 0.9; stepSize: 0.05
                                value: prefs.prof.transparency
                                onMoved: (v) => prefs.setP("transparency", v)
                            }
                            ToggleRow { label: "Keep background colors opaque"; checked: prefs.prof.keep_backgrounds_opaque; onToggled: (on) => prefs.setP("keep_backgrounds_opaque", on) }

                            SectionTitle { text: "Size" }
                            Row2 {
                                label: "Columns"
                                hint: "Size of new windows."
                                Stepper { value: prefs.prof.columns; step: 5; from: 20; to: 500; onMoved: (v) => prefs.setP("columns", Math.round(v)) }
                                Item { Layout.fillWidth: true }
                            }
                            Row2 {
                                label: "Rows"
                                Stepper { value: prefs.prof.rows; step: 2; from: 5; to: 200; onMoved: (v) => prefs.setP("rows", Math.round(v)) }
                                Item { Layout.fillWidth: true }
                            }
                            SliderRow {
                                label: "Margins"
                                hint: "Space around the text."
                                from: 0; to: 24; stepSize: 1; decimals: 0; suffix: " px"
                                value: prefs.prof.padding
                                onMoved: (v) => prefs.setP("padding", v)
                            }
                        }

                        // ================= Profiles: Terminal =================
                        ColumnLayout {
                            visible: prefs.page === "profiles" && prefs.profileTab === "terminal"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Scrollback" }
                            Row2 {
                                label: "Scrollback lines"
                                hint: "For new sessions."
                                Stepper {
                                    value: prefs.prof.scrollback
                                    step: prefs.prof.scrollback >= 10000 ? 10000 : 1000
                                    from: 1000; to: 1000000
                                    display: Number(prefs.prof.scrollback).toLocaleString(Qt.locale(), "f", 0)
                                    onMoved: (v) => prefs.setP("scrollback", Math.round(v))
                                }
                                Item { Layout.fillWidth: true }
                            }
                            ToggleRow { label: "Unlimited scrollback"; checked: prefs.prof.unlimited_scrollback; onToggled: (on) => prefs.setP("unlimited_scrollback", on) }

                            SectionTitle { text: "Terminal Emulation" }
                            Row2 {
                                label: "Report terminal type"
                                hint: "TERM for new sessions."
                                ComboBox {
                                    Layout.preferredWidth: 220
                                    editable: true
                                    model: ["xterm-256color", "xterm", "xterm-ghostty", "screen-256color", "tmux-256color", "linux", "vt100"]
                                    Component.onCompleted: editText = prefs.prof.term
                                    onActivated: prefs.setP("term", currentText)
                                    onAccepted: prefs.setP("term", editText)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            ToggleRow { label: "Terminal may change the title"; checked: prefs.prof.allow_title; onToggled: (on) => prefs.setP("allow_title", on) }
                            ToggleRow { label: "Terminal may enable paste bracketing"; checked: prefs.prof.bracketed_paste; onToggled: (on) => prefs.setP("bracketed_paste", on) }

                            SectionTitle { text: "Mouse" }
                            ToggleRow { label: "Enable mouse reporting"; hint: "Programs like vim, htop and tmux get clicks and scrolling. Hold Shift to select text anyway."; checked: prefs.prof.mouse_reporting; onToggled: (on) => prefs.setP("mouse_reporting", on) }
                            ToggleRow { label: "Alternate mouse scroll"; hint: "Scrolling in full-screen programs sends arrow keys."; checked: prefs.prof.alternate_scroll; onToggled: (on) => prefs.setP("alternate_scroll", on) }

                            SectionTitle { text: "Bell" }
                            ToggleRow { label: "Silence bell"; checked: prefs.prof.silence_bell; onToggled: (on) => prefs.setP("silence_bell", on) }
                            ToggleRow { label: "Flash visual bell"; checked: prefs.prof.visual_bell; onToggled: (on) => prefs.setP("visual_bell", on) }
                            ToggleRow { label: "Show bell icon in tabs"; checked: prefs.prof.bell_icon; onToggled: (on) => prefs.setP("bell_icon", on) }

                            SectionTitle { text: "Notifications" }
                            ToggleRow { label: "Notify on bell"; hint: "A desktop notification when a background tab, or this window while you're elsewhere, rings the bell."; checked: prefs.prof.notify_bell; onToggled: (on) => prefs.setP("notify_bell", on) }
                            ToggleRow { label: "Notify when a session ends"; hint: "When it isn't the tab you're looking at."; checked: prefs.prof.notify_exit; onToggled: (on) => prefs.setP("notify_exit", on) }

                            SectionTitle { text: "Shell Integration" }
                            ToggleRow { label: "Show mark indicators"; hint: "A tick in the left margin at each prompt. Needs shell integration (\u2630 \u203a Install Shell Integration)."; checked: prefs.prof.show_marks; onToggled: (on) => prefs.setP("show_marks", on) }
                        }

                        // ================= Profiles: Session =================
                        ColumnLayout {
                            visible: prefs.page === "profiles" && prefs.profileTab === "session"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            Row2 {
                                label: "After a session ends"
                                Segmented {
                                    options: [{ label: "Close", value: "close" }, { label: "Restart", value: "restart" }, { label: "Keep open", value: "keep" }]
                                    value: prefs.prof.on_exit
                                    onPicked: (v) => prefs.setP("on_exit", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            Row2 {
                                label: "Prompt before closing"
                                ColumnLayout {
                                    spacing: 8
                                    Segmented {
                                        options: [{ label: "Never", value: "never" }, { label: "Always", value: "always" }, { label: "If jobs are running", value: "jobs" }]
                                        value: prefs.prof.confirm_close
                                        onPicked: (v) => prefs.setP("confirm_close", v)
                                    }
                                    Hint { visible: prefs.prof.confirm_close === "jobs"; text: "Jobs that don't count:" }
                                    Field {
                                        visible: prefs.prof.confirm_close === "jobs"
                                        saved: prefs.prof.ignored_jobs || ""
                                        Layout.preferredWidth: 380
                                        onCommit: (t) => prefs.setP("ignored_jobs", t)
                                    }
                                }
                                Item { Layout.fillWidth: true }
                            }
                            SectionTitle { text: "Logging" }
                            ToggleRow { label: "Automatically log session output"; hint: "Every new session of this profile writes its output (raw, with escape sequences) to a file."; checked: prefs.prof.auto_log; onToggled: (on) => prefs.setP("auto_log", on) }
                            Row2 {
                                visible: prefs.prof.auto_log
                                label: "Folder"
                                Field { saved: prefs.prof.log_folder || ""; onCommit: (t) => prefs.setP("log_folder", t) }
                                Item { Layout.fillWidth: true }
                            }
                            Hint { text: "Logging one session by hand: Session \u203a Log \u203a Log to File. With \u201cRestore sessions\u201d on (General), sessions outlive the window." }
                        }

                        // ================= Profiles: Keys =================
                        ColumnLayout {
                            visible: prefs.page === "profiles" && prefs.profileTab === "keys"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            Row2 {
                                label: "Alt key acts as"
                                hint: "+Esc: Alt+key sends Esc then the key (what shells and editors expect). Normal: the key alone."
                                Segmented {
                                    options: [{ label: "+Esc", value: "esc" }, { label: "Normal", value: "normal" }]
                                    value: prefs.prof.alt_mode
                                    onPicked: (v) => prefs.setP("alt_mode", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            Hint { text: "Programs can turn on the Kitty keyboard protocol or application keypad mode themselves; both are supported." }

                            SectionTitle { text: "Key Mappings" }
                            Hint { text: "A key here does something else instead of reaching the program. In text, \\n is a newline, \\t a tab and \\e escape." }
                            Repeater {
                                model: prefs.prof.key_mappings || []
                                delegate: RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    KeyField { value: modelData.key; onRecorded: (k) => prefs.setP("key_mappings", prefs.editItem(prefs.prof.key_mappings, index, "key", k)) }
                                    ChoiceBox {
                                        values: [["send_text", "Send text"], ["send_escape", "Send Esc +"], ["send_hex", "Send hex codes"], ["action", "Run action"], ["ignore", "Ignore"]]
                                        value: modelData.action
                                        onChosen: (v) => prefs.setP("key_mappings", prefs.editItem(prefs.prof.key_mappings, index, "action", v))
                                    }
                                    ChoiceBox {
                                        visible: modelData.action === "action"
                                        values: [["new_tab", "New tab"], ["new_window", "New window"], ["close", "Close pane"], ["split_right", "Split right"], ["split_down", "Split down"],
                                                 ["next_tab", "Next tab"], ["previous_tab", "Previous tab"], ["next_pane", "Next pane"], ["previous_pane", "Previous pane"],
                                                 ["maximize_pane", "Maximize pane"], ["find", "Find"], ["copy", "Copy"], ["paste", "Paste"], ["clear_buffer", "Clear buffer"],
                                                 ["previous_prompt", "Previous prompt"], ["next_prompt", "Next prompt"], ["select_last_output", "Select last output"],
                                                 ["undo_close", "Undo close"], ["fullscreen", "Full screen"], ["preferences", "Preferences"]]
                                        value: modelData.value
                                        onChosen: (v) => prefs.setP("key_mappings", prefs.editItem(prefs.prof.key_mappings, index, "value", v))
                                    }
                                    Field {
                                        visible: modelData.action !== "action" && modelData.action !== "ignore"
                                        saved: modelData.value
                                        placeholderText: modelData.action === "send_hex" ? "0x1b 0x5b 0x41" : "text"
                                        onCommit: (t) => prefs.setP("key_mappings", prefs.editItem(prefs.prof.key_mappings, index, "value", t))
                                    }
                                    Item { Layout.fillWidth: true }
                                    SmallButton { onHit: prefs.setP("key_mappings", prefs.removeItem(prefs.prof.key_mappings, index)) }
                                }
                            }
                            PlainButton { text: "Add Key Mapping"; onClicked: prefs.setP("key_mappings", (prefs.prof.key_mappings || []).concat([{ key: "", action: "send_text", value: "" }])) }
                        }

                        // ================= Profiles: Advanced =================
                        ColumnLayout {
                            visible: prefs.page === "profiles" && prefs.profileTab === "advanced"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Triggers" }
                            Hint { text: "When a line of output matches a regular expression: highlight the match, post a notification, ring the bell, or send text back (\\n is Enter)." }
                            Repeater {
                                model: prefs.prof.triggers || []
                                delegate: RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    Toggle { checked: modelData.enabled; onToggled: (on) => prefs.setP("triggers", prefs.editItem(prefs.prof.triggers, index, "enabled", on)) }
                                    Field {
                                        saved: modelData.regex
                                        placeholderText: "regular expression"
                                        font.family: "monospace"
                                        onCommit: (t) => prefs.setP("triggers", prefs.editItem(prefs.prof.triggers, index, "regex", t))
                                    }
                                    ChoiceBox {
                                        values: [["highlight", "Highlight"], ["notify", "Notify"], ["bell", "Ring bell"], ["send", "Send text"]]
                                        value: modelData.action
                                        onChosen: (v) => prefs.setP("triggers", prefs.editItem(prefs.prof.triggers, index, "action", v))
                                    }
                                    Field {
                                        visible: modelData.action !== "bell"
                                        Layout.maximumWidth: 160
                                        saved: modelData.param
                                        placeholderText: modelData.action === "highlight" ? "#d9b43b" : (modelData.action === "notify" ? "title" : "text")
                                        onCommit: (t) => prefs.setP("triggers", prefs.editItem(prefs.prof.triggers, index, "param", t))
                                    }
                                    SmallButton { onHit: prefs.setP("triggers", prefs.removeItem(prefs.prof.triggers, index)) }
                                }
                            }
                            PlainButton { text: "Add Trigger"; onClicked: prefs.setP("triggers", (prefs.prof.triggers || []).concat([{ regex: "", action: "highlight", param: "#d9b43b", enabled: true }])) }
                        }

                        // ================= Snippets =================
                        ColumnLayout {
                            visible: prefs.page === "snippets"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Snippets" }
                            Hint { text: "Text sent to the terminal as if typed (Edit › Snippets). \\n is Enter, \\t Tab, \\e Escape." }
                            Repeater {
                                model: prefs.settings.snippets || []
                                delegate: RowLayout {
                                    Layout.fillWidth: true
                                    spacing: 8
                                    Field {
                                        Layout.maximumWidth: 180
                                        saved: modelData.title
                                        placeholderText: "Title"
                                        onCommit: (t) => prefs.setG("snippets", prefs.editItem(prefs.settings.snippets, index, "title", t))
                                    }
                                    Field {
                                        Layout.maximumWidth: 1000
                                        saved: modelData.text
                                        placeholderText: "Text"
                                        font.family: "monospace"
                                        onCommit: (t) => prefs.setG("snippets", prefs.editItem(prefs.settings.snippets, index, "text", t))
                                    }
                                    SmallButton { onHit: prefs.setG("snippets", prefs.removeItem(prefs.settings.snippets, index)) }
                                }
                            }
                            PlainButton { text: "Add Snippet"; onClicked: prefs.setG("snippets", (prefs.settings.snippets || []).concat([{ title: "New Snippet", text: "" }])) }
                        }
                        // ================= Arrangements =================
                        ColumnLayout {
                            visible: prefs.page === "arrangements"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Window Arrangements" }
                            Hint { text: "An arrangement remembers tabs, splits, profiles and folders. Restoring one opens fresh sessions laid out the same way." }
                            Repeater {
                                model: Object.keys(prefs.arrangements)
                                delegate: Rectangle {
                                    Layout.fillWidth: true
                                    implicitHeight: 44
                                    radius: 8
                                    color: Theme.field
                                    border.color: Theme.divider
                                    RowLayout {
                                        anchors.fill: parent
                                        anchors.leftMargin: 14
                                        anchors.rightMargin: 8
                                        spacing: 8
                                        Text { Layout.fillWidth: true; text: modelData; color: Theme.text; font.pixelSize: 13 }
                                        Text { text: (prefs.arrangements[modelData].tabs || []).length + " tabs"; color: Theme.textDim; font.pixelSize: 12 }
                                        PlainButton { text: "Restore"; onClicked: prefs.restoreArrangement(modelData) }
                                        PlainButton { text: "Delete"; onClicked: prefs.controller.deleteArrangement(modelData) }
                                    }
                                }
                            }
                            Hint { visible: Object.keys(prefs.arrangements).length === 0; text: "None saved yet." }
                            RowLayout {
                                spacing: 8
                                Field { id: arrName; placeholderText: "Name"; Layout.preferredWidth: 220 }
                                PlainButton { text: "Save Current Window"; enabled2: arrName.text.trim() !== ""; onClicked: { prefs.saveArrangement(arrName.text.trim()); arrName.text = "" } }
                            }
                        }

                        // ================= Prompt =================
                        ColumnLayout {
                            visible: prefs.page === "prompt"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            Row2 {
                                label: "Prompt from"
                                hint: (prefs.settings.prompt_config || "own") === "tessera" ? "The prompt built on this page (Tessera keeps its own starship.toml; yours is left alone)."
                                                                                     : "Your own starship.toml. Build one here and choose Use This Prompt to switch."
                                Segmented {
                                    options: [{ label: "Your starship.toml", value: "own" }, { label: "Built here", value: "tessera" }]
                                    value: prefs.settings.prompt_config || "own"
                                    onPicked: (v) => prefs.setG("prompt_config", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            component PromptCard: Rectangle {
                                id: pcard
                                property string key
                                property string title
                                property bool chosen: false
                                signal picked()
                                Layout.fillWidth: true
                                implicitHeight: pcol.implicitHeight + 16
                                radius: 8
                                color: prefs.promptTheme.background || Theme.bg
                                border.width: chosen ? 2 : 1
                                border.color: chosen ? Theme.accent : (cardHover.hovered ? Theme.textDim : Theme.divider)
                                Column {
                                    id: pcol
                                    x: 10; y: 8
                                    width: parent.width - 20
                                    spacing: 4
                                    Text { text: pcard.title; color: prefs.promptTheme.foreground || Theme.text; opacity: 0.7; font.pixelSize: 11 }
                                    StarshipLine {
                                        width: parent.width
                                        clip: true
                                        lines: (prefs.promptPreviews[pcard.key] || {}).lines || []
                                        themeData: prefs.promptTheme
                                        fontFamily: prefs.prof.font_family || "monospace"
                                        fontSize: 11
                                    }
                                }
                                HoverHandler { id: cardHover }
                                TapHandler { onTapped: pcard.picked() }
                            }
                            SectionTitle { text: "Your prompt" }
                            PromptCard { key: "current"; title: "Preview (your home folder; the last command failed after 3.4 s)"; chosen: true }
                            RowLayout {
                                spacing: 10
                                PlainButton {
                                    text: "Use This Prompt"
                                    onClicked: {
                                        const err = prefs.controller.savePromptConfig(prefs.promptToml)
                                        if (err === "") prefs.setG("prompt_config", "tessera")
                                    }
                                }
                                PlainButton { text: "Start From My starship.toml"; onClicked: prefs.setPromptToml(prefs.controller.promptUserConfig()) }
                                Hint { text: (prefs.settings.prompt_config || "own") === "tessera" ? "In use." : "" }
                            }

                            SectionTitle { text: "Modules" }
                            Flow {
                                Layout.fillWidth: true
                                spacing: 6
                                readonly property var states: { try { return JSON.parse(prefs.controller.promptModules(prefs.promptToml)) } catch (e) { return {} } }
                                Repeater {
                                    model: Object.keys(parent.states)
                                    delegate: Rectangle {
                                        required property string modelData
                                        readonly property var st: parent.states[modelData]
                                        height: 26
                                        width: modText.implicitWidth + 30
                                        radius: 13
                                        color: st.on ? Theme.mix(Theme.panel, Theme.accent, 0.3) : Theme.field
                                        border.color: st.on ? Theme.accent : Theme.divider
                                        Text {
                                            id: modText
                                            anchors.centerIn: parent
                                            text: (parent.st.on ? "\u2713 " : "") + parent.st.label
                                            color: Theme.text
                                            font.pixelSize: 12
                                        }
                                        TapHandler { onTapped: prefs.setPromptToml(prefs.controller.promptSetModule(prefs.promptToml, modelData, !parent.st.on)) }
                                    }
                                }
                            }

                            SectionTitle { text: "Presets" }
                            Hint { text: "Starship's own designs (they use a Nerd Font). Click one to start from it." }
                            Repeater {
                                model: prefs.promptPresets
                                delegate: PromptCard {
                                    required property string modelData
                                    key: "preset-" + modelData
                                    title: modelData
                                    onPicked: prefs.setPromptToml(prefs.controller.promptPreset(modelData))
                                }
                            }

                            SectionTitle { text: "Settings (starship.toml)" }
                            Hint { text: "Every Starship option: modules, formats, colors, symbols (starship.rs/config). The preview follows as you type." }
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 320
                                radius: 8
                                color: Theme.field
                                border.color: tomlEdit.activeFocus ? Theme.accent : Theme.divider
                                ScrollView {
                                    anchors.fill: parent
                                    anchors.margins: 6
                                    TextArea {
                                        id: tomlEdit
                                        text: prefs.promptToml
                                        font.family: prefs.prof.font_family || "monospace"
                                        font.pointSize: 10
                                        color: Theme.text
                                        selectByMouse: true
                                        background: null
                                        onTextChanged: if (activeFocus && text !== prefs.promptToml) prefs.setPromptToml(text)
                                    }
                                }
                            }
                        }

                        // ================= AI =================
                        ColumnLayout {
                            visible: prefs.page === "ai"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "Providers" }
                            Hint {
                                Layout.fillWidth: true
                                wrapMode: Text.Wrap
                                text: "Local models (Ollama, llama.cpp, LM Studio, vLLM, LocalAI) need no key. Claude, OpenAI/Codex, Gemini, OpenRouter, Groq, Mistral, DeepSeek and xAI need an API key: paste it, or write env:NAME to read it from an environment variable. Keys are kept in secrets.json in the config folder, readable only by you."
                            }
                            Repeater {
                                model: prefs.ai.providers || []
                                delegate: Rectangle {
                                    id: provRow
                                    required property var modelData
                                    required property int index
                                    Layout.fillWidth: true
                                    Layout.maximumWidth: 760
                                    implicitHeight: provCol.implicitHeight + 20
                                    radius: 8
                                    color: Theme.field
                                    border.color: Theme.divider
                                    ColumnLayout {
                                        id: provCol
                                        anchors.fill: parent
                                        anchors.margins: 10
                                        spacing: 6
                                        RowLayout {
                                            Text { text: provRow.modelData.name || provRow.modelData.id; color: Theme.text; font.pixelSize: 13; font.weight: Font.DemiBold }
                                            Text {
                                                readonly property var m: prefs.aiModels[provRow.modelData.id]
                                                readonly property bool keyless: !provRow.modelData.local && !(prefs.controller.revision >= 0 && prefs.controller.aiHasKey(provRow.modelData.id))
                                                text: keyless ? "needs an API key" : m === undefined ? "checking\u2026" : (typeof m === "string" ? m.replace(/^error: /, "\u26a0 ") : m.length + " models")
                                                color: typeof m === "string" ? "#e06c75" : Theme.textDim
                                                font.pixelSize: 11
                                                elide: Text.ElideRight
                                                Layout.fillWidth: true
                                            }
                                            PlainButton { implicitHeight: 24; text: "Refresh"; onClicked: prefs.controller.aiListModels(JSON.stringify(provRow.modelData)) }
                                            PlainButton {
                                                implicitHeight: 24
                                                text: "Remove"
                                                onClicked: prefs.setAi("providers", prefs.ai.providers.filter((_, i) => i !== provRow.index))
                                            }
                                        }
                                        RowLayout {
                                            Text { text: "Address"; color: Theme.textDim; font.pixelSize: 12; Layout.preferredWidth: 60 }
                                            Field {
                                                saved: provRow.modelData.base_url
                                                Layout.maximumWidth: 9999
                                                onCommit: (t) => {
                                                    const ps = JSON.parse(JSON.stringify(prefs.ai.providers))
                                                    ps[provRow.index].base_url = t.trim()
                                                    prefs.setAi("providers", ps)
                                                }
                                            }
                                        }
                                        RowLayout {
                                            visible: !provRow.modelData.local
                                            Text { text: "API key"; color: Theme.textDim; font.pixelSize: 12; Layout.preferredWidth: 60 }
                                            TextField {
                                                id: keyField
                                                Layout.fillWidth: true
                                                echoMode: TextInput.Password
                                                color: Theme.text
                                                placeholderText: prefs.controller.revision >= 0 && prefs.controller.aiHasKey(provRow.modelData.id) ? "\u2022\u2022\u2022\u2022 set (type to replace)" : "Paste the key, or env:VARIABLE"
                                                placeholderTextColor: Theme.textDim
                                                background: Rectangle { radius: 7; color: Theme.bg; border.color: keyField.activeFocus ? Theme.accent : Theme.divider }
                                                onEditingFinished: if (text !== "") {
                                                    prefs.controller.aiSetKey(provRow.modelData.id, text)
                                                    text = ""
                                                    prefs.controller.aiListModels(JSON.stringify(provRow.modelData))
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            RowLayout {
                                spacing: 10
                                ChoiceBox {
                                    id: addProv
                                    implicitWidth: 240
                                    values: [["", "Add a provider\u2026"]].concat(prefs.aiPresets.filter(p => !(prefs.ai.providers || []).some(x => x.id === p.id)).map(p => [p.id, p.name]))
                                    value: ""
                                    onChosen: (v) => {
                                        if (v === "") return
                                        const p = prefs.aiPresets.find(x => x.id === v)
                                        prefs.setAi("providers", (prefs.ai.providers || []).concat([p]))
                                        prefs.controller.aiListModels(JSON.stringify(p))
                                        currentIndex = 0
                                    }
                                }
                                PlainButton {
                                    text: "Look for Ollama"
                                    onClicked: { prefs.setAi("autodetected", false); prefs.controller.aiAutodetect() }
                                }
                            }

                            SectionTitle { text: "Models" }
                            component ModelRow: Row2 {
                                id: mr
                                property string key
                                property string fallbackHint: ""
                                ComboBox {
                                    id: mc
                                    editable: true
                                    implicitWidth: 340
                                    model: prefs.aiModelRefs
                                    // (A new list resets the text: put the saved model back.)
                                    function showSaved() { const v = prefs.ai[mr.key] || ""; currentIndex = prefs.aiModelRefs.indexOf(v); editText = v }
                                    Component.onCompleted: showSaved()
                                    onModelChanged: Qt.callLater(showSaved)
                                    Connections { target: prefs; function onAiChanged() { if (!mc.activeFocus) mc.showSaved() } }
                                    onAccepted: prefs.setAi(mr.key, editText.trim())
                                    onActivated: (i) => prefs.setAi(mr.key, prefs.aiModelRefs[i])
                                }
                                Item { Layout.fillWidth: true }
                            }
                            ModelRow { key: "inline_model"; label: "Suggestions as you type"; hint: "Small and fast is best (e.g. ollama/qwen3.5:4b). Also checks commands unless the next one is set." }
                            ModelRow { key: "chat_model"; label: "Assistant and commands"; hint: "The sidebar, # descriptions turned into commands, and fixing failed commands (e.g. ollama/qwen3.5:9b, anthropic/claude-sonnet-4-5, openai/gpt-5)." }
                            ModelRow { key: "check_model"; label: "Checking commands"; hint: "Empty: the suggestions model." }

                            SectionTitle { text: "In the editor" }
                            ToggleRow { label: "Suggest as you type"; hint: "Grey text after the cursor: Tab or \u2192 takes it."; checked: prefs.ai.inline !== false; onToggled: (on) => prefs.setAi("inline", on) }
                            ToggleRow { label: "Check commands as you type"; hint: "A short warning in the editor's corner when a command looks wrong or risky."; checked: prefs.ai.check !== false; onToggled: (on) => prefs.setAi("check", on) }
                            ToggleRow { label: "Turn descriptions into commands"; hint: "Start a line with # and press Enter: the command replaces it, for you to check and run."; checked: prefs.ai.natural_language !== false; onToggled: (on) => prefs.setAi("natural_language", on) }
                        }

                        // ================= Pointer =================
                        ColumnLayout {
                            visible: prefs.page === "pointer"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 12

                            SectionTitle { text: "General" }
                            ToggleRow { label: "Middle click pastes the selection"; checked: prefs.settings.middle_click_pastes; onToggled: (on) => prefs.setG("middle_click_pastes", on) }
                            ToggleRow { label: "Ctrl+click opens URLs and files"; hint: "Links (including OSC 8 hyperlinks), and paths of files that exist, such as file:line from compiler errors."; checked: prefs.settings.ctrl_click_opens; onToggled: (on) => prefs.setG("ctrl_click_opens", on) }
                            Row2 {
                                label: "Right click"
                                Segmented {
                                    options: [{ label: "Opens menu", value: "menu" }, { label: "Pastes", value: "paste" }]
                                    value: prefs.settings.right_click
                                    onPicked: (v) => prefs.setG("right_click", v)
                                }
                                Item { Layout.fillWidth: true }
                            }
                            ToggleRow { label: "Focus follows mouse"; hint: "Pointing at a pane gives it the keyboard."; checked: prefs.settings.focus_follows_mouse; onToggled: (on) => prefs.setG("focus_follows_mouse", on) }
                            Hint { text: "When a program asks for the mouse, clicks go to it; hold Shift to select or open the menu anyway." }
                        }

                        // ================= About =================
                        ColumnLayout {
                            visible: prefs.page === "about"
                            Layout.fillWidth: true
                            Layout.margins: 24
                            spacing: 10
                            Text { text: "Tessera"; color: Theme.text; font.pixelSize: 24; font.weight: Font.DemiBold }
                            Hint { text: "A terminal in the spirit of iTerm2. Terminal emulation by libghostty-vt; interface in Qt Quick via cxx-qt." }
                            Hint { text: "Settings, profiles and imported themes: " + prefs.controller.configFolder() }
                        }
                    }
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
