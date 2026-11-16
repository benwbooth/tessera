import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// A leaf of the pane tree: one terminal view, with its profile.
Item {
    id: pane
    property var host: null
    property alias view: term
    property alias attachId: term.attachId
    property alias tmuxPane: term.tmuxPane
    property alias cwd: term.cwd
    property string profileName: ""
    property bool taken: false         // its session moved to another window
    property string commandOverride: "" // a shell or ssh picked from the + menu
    property alias preloadFile: term.preloadFile
    property alias themeJson: term.themeJson
    property alias fontFamily: term.fontFamily
    property alias fontPointSize: term.fontPointSize
    property alias lineHeight: term.lineHeight
    property alias paddingSize: term.paddingSize
    property alias cursorShape: term.cursorShape
    property alias scrollback: term.scrollback
    property alias copyOnSelect: term.copyOnSelect
    property bool dimInactive: true
    property real dimAmount: 0.22
    property bool showTitle: false     // per-pane title bar (Appearance > Panes)
    property bool broadcasting: false  // receives broadcast input: red-stripe indicator
    property string badge: ""
    property real prefW: -1
    property real prefH: -1
    property string job: ""            // the program running in it (empty at a prompt)
    property string procCwd: ""        // its folder, read from the process
    property string shellName: ""      // the shell at the prompt
    property int zoomPercent: 100
    // The input editor (ComposeBox) and what it needs from the window.
    // Has the keys (the terminal or its editor).
    readonly property bool focused: term.activeFocus || compose.editing
    property bool inputEditor: false
    property string editorKeys: "emacs"
    property var commandHistory: []
    property var controller: null
    // REPLs the editor also serves (the shell needs its integration's prompt marks).
    readonly property var replPrograms: ["python", "python3", "ipython", "bpython", "node", "deno", "bun"]
    readonly property string replLang: {
        const j = pane.job.replace(/^.*\//, "")
        return replPrograms.some(r => j === r || j.startsWith("python")) && controller ? controller.languageFor(j) : ""
    }
    readonly property bool editorOn: inputEditor && !term.exited && !term.fullScreen && (term.atPrompt || replLang !== "")      // the window's text zoom, shown with the size
    signal contextMenu(var view, real x, real y)
    SplitView.preferredWidth: prefW
    SplitView.preferredHeight: prefH
    SplitView.minimumWidth: 60
    SplitView.minimumHeight: 40

    implicitWidth: 400
    implicitHeight: 300

    readonly property bool split: pane.host !== null && (!pane.host.isRoot || pane.host.count > 1)

    Rectangle {
        id: titleBar
        visible: pane.showTitle && pane.split
        anchors.left: parent.left
        anchors.right: parent.right
        height: visible ? 22 : 0
        color: pane.focused ? Theme.tabHover : Theme.chrome
        Text {
            anchors.fill: parent
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            verticalAlignment: Text.AlignVCenter
            text: term.title || (term.pwd ? term.pwd : pane.profileName)
            color: pane.focused ? Theme.text : Theme.textDim
            elide: Text.ElideMiddle
            font.pixelSize: 11
        }
    }

    TerminalView {
        id: term
        anchors.top: titleBar.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: compose.visible ? compose.top : parent.bottom
        focus: true
        onContextMenuRequested: (x, y) => pane.contextMenu(term, x, y)
        // The pane taking the keys (switching to it, clicking it) gives them
        // to the editor while it's up.
        onActiveFocusChanged: if (activeFocus && compose.visible) Qt.callLater(compose.focusEditor)
        // Typing into the terminal while the editor is up goes to the editor.
        Keys.onPressed: (e) => {
            if (compose.visible && e.text !== "" && e.text.charCodeAt(0) >= 32
                    && !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))) {
                compose.focusEditor()
                compose.insert(e.text)
                e.accepted = true
            }
        }
    }

    ComposeBox {
        id: compose
        visible: pane.editorOn
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: implicitHeight
        view: term
        controller: pane.controller
        keys: pane.editorKeys
        history: pane.commandHistory
        lang: pane.replLang !== "" ? pane.replLang : "bash"
        themeData: { try { return JSON.parse(pane.themeJson) } catch (e) { return {} } }
        cwd: term.pwd !== "" ? term.pwd : pane.procCwd
        onSubmitted: (t) => {
            if (t.indexOf("\n") < 0) term.sendText(t + "\r")
            else { term.pasteText(t); term.sendText("\r") }
            term.scrollToBottom()
        }
        onHandBack: term.forceActiveFocus()
        onVisibleChanged: {
            if (visible && term.activeFocus) focusEditor()
            else if (!visible && editing) term.forceActiveFocus()
        }
    }

    // Badge: large text behind the content, top right (iTerm2's badges).
    Text {
        visible: pane.badge !== ""
        anchors.top: term.top
        anchors.right: term.right
        anchors.margins: 14
        text: pane.badge
            .replace("{title}", term.title).replace("{cwd}", term.pwd)
            .replace("{profile}", pane.profileName)
        color: Theme.fg
        opacity: 0.18
        font.pixelSize: Math.max(18, Math.min(64, term.height / 6))
        font.weight: Font.Bold
    }

    // The grid size (and zoom), briefly, whenever it changes: while zooming
    // or resizing the window, as iTerm2 shows it.
    Rectangle {
        id: sizeTip
        z: 6
        anchors.centerIn: term
        width: sizeText.implicitWidth + 24
        height: sizeText.implicitHeight + 12
        radius: 8
        color: Theme.mix(Theme.bg, Theme.fg, 0.10)
        border.width: 1
        border.color: Theme.mix(Theme.bg, Theme.fg, 0.2)
        opacity: 0
        visible: opacity > 0
        Behavior on opacity { NumberAnimation { duration: 140 } }
        property bool armed: false     // not for the first size, when it opens
        Text {
            id: sizeText
            anchors.centerIn: parent
            anchors.verticalCenterOffset: 0.001
            text: term.columns + " × " + term.rows + (pane.zoomPercent !== 100 ? "   " + pane.zoomPercent + "%" : "")
            color: Theme.text
            font.pixelSize: 14
            font.weight: Font.DemiBold
            font.features: { "tnum": 1 }
        }
        Timer { id: sizeHide; interval: 1100; onTriggered: sizeTip.opacity = 0 }
        Timer { interval: 1500; running: true; onTriggered: sizeTip.armed = true }
        function flash() {
            if (!armed || !pane.visible || term.columns <= 0) return
            opacity = 0.96
            sizeHide.restart()
        }
        Connections {
            target: term
            function onGridChanged() { sizeTip.flash() }
        }
    }
    onZoomPercentChanged: sizeTip.flash()

    // Overlay scrollbar: shows while scrolling or near the right edge.
    Item {
        id: sbar
        z: 4
        anchors.top: term.top
        anchors.bottom: term.bottom
        anchors.right: term.right
        anchors.topMargin: 2
        anchors.bottomMargin: 2
        width: 14
        readonly property real total: term.scrollTotal
        readonly property real len: term.scrollLength
        readonly property bool scrollable: total > len + 0.5
        readonly property bool hot: barHover.hovered || thumbDrag.active
        property bool recent: false
        visible: scrollable
        opacity: hot || recent ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 220 } }

        Connections {
            target: term
            function onScrollChanged() { sbar.recent = true; hideTimer.restart() }
        }
        Timer { id: hideTimer; interval: 1100; onTriggered: sbar.recent = false }
        HoverHandler { id: barHover }

        // The track (only while hovered), click to page.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 2
            radius: width / 2
            color: Theme.fg
            opacity: sbar.hot ? 0.06 : 0
            Behavior on opacity { NumberAnimation { duration: 150 } }
            TapHandler {
                onTapped: (pt) => {
                    const page = Math.max(1, sbar.len - 1)
                    const up = pt.position.y < thumb.y
                    term.scrollToRow(Math.max(0, term.scrollOffset + (up ? -page : page)))
                }
            }
        }
        Rectangle {
            id: thumb
            readonly property real span: sbar.height - height
            width: sbar.hot ? 8 : 5
            anchors.right: parent.right
            anchors.rightMargin: 3
            radius: width / 2
            height: Math.max(28, sbar.height * sbar.len / Math.max(1, sbar.total))
            y: sbar.total > sbar.len ? span * term.scrollOffset / (sbar.total - sbar.len) : 0
            color: Theme.fg
            opacity: thumbDrag.active ? 0.6 : (sbar.hot ? 0.45 : 0.32)
            Behavior on width { NumberAnimation { duration: 120 } }
            property real startOffset: 0
            DragHandler {
                id: thumbDrag
                target: null
                yAxis.enabled: true
                xAxis.enabled: false
                onActiveChanged: if (active) thumb.startOffset = term.scrollOffset
                onTranslationChanged: if (active && thumb.span > 0) {
                    const rows = activeTranslation.y / thumb.span * (sbar.total - sbar.len)
                    term.scrollToRow(Math.round(Math.max(0, Math.min(sbar.total - sbar.len, thumb.startOffset + rows))))
                }
            }
        }
    }

    property alias findBar: findBar
    property alias compose: compose
    FindBar {
        id: findBar
        view: term
        z: 5
        anchors.top: term.top
        anchors.right: term.right
        anchors.topMargin: 8
        anchors.rightMargin: 14
    }

    // Broadcast Input: the receiving panes get a red outline.
    Rectangle {
        anchors.fill: term
        visible: pane.broadcasting
        color: "transparent"
        border.width: 2
        border.color: "#e04848"
        opacity: 0.8
    }

    // Dim panes without focus when the tab is split (like iTerm2).
    Rectangle {
        anchors.fill: parent
        color: Theme.dark ? "black" : "white"
        opacity: Theme.dark ? pane.dimAmount : pane.dimAmount * 1.5
        visible: pane.dimInactive && !pane.focused && pane.split
    }
}
