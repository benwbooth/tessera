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
    readonly property bool focused: term.activeFocus || compose.editing || notebook.activeFocus
    signal editorKeysPicked(string keys)
    signal markdownStylePicked(string style)
    property string markdownStyle: "rich"  // Markdown cells: "rich" or "text"
    property bool preferEditor: true   // the keys go to the editor (not the terminal) when the pane gets them
    property bool inputEditor: false
    property string editorKeys: "emacs"
    property bool editorPrompt: true
    property var aiSettings: ({})
    property string promptEngine: "auto"
    property int shellPid: 0            // from the daemon (for the prompt's environment)
    property var promptSegments: ["dir", "git", "venv", "nix", "status", "duration", "|", "kube", "aws", "gcloud", "host", "time"]
    property var commandHistory: []
    property var controller: null
    // REPLs the editor also serves (the shell needs its integration's prompt marks).
    readonly property var replPrograms: ["python", "python3", "ipython", "bpython", "node", "deno", "bun"]
    readonly property string replLang: {
        const j = pane.job.replace(/^.*\//, "")
        return replPrograms.some(r => j === r || j.startsWith("python")) && controller ? controller.languageFor(j) : ""
    }
    onEditorOnChanged: {
        // (Back at the prompt after a command or a full-screen program like
        // vim: the editor takes the keys again, even if the terminal had them.)
        if (editorOn && focused) { preferEditor = true; Qt.callLater(compose.focusEditor) }
        else if (!editorOn && compose.editing) term.forceActiveFocus()
    }
    readonly property bool editorOn: inputEditor && !term.exited && !term.fullScreen && (term.atPrompt || idleShell || replLang !== "")
    // (Reattached after a restart, the shell may be at its prompt without
    // having said so since: nothing running in front of it counts as that,
    // until the shell integration says otherwise.)
    property bool idleShell: false
    // Shells started in the pane's shell from the editor's language list, the
    // first (bottom) the pane's own: [{name, pid}]. Switching to one below
    // leaves the ones above (exit), what they changed handed down; to a new
    // one starts it on top.
    property var shellStack: []
    function switchShell(sh) {
        if (!controller || term.exited || !(term.atPrompt || idleShell)) return
        const cur = pane.shellName !== "" ? pane.shellName : "bash"
        let st = shellStack.length ? shellStack.slice() : [{ name: cur, pid: pane.shellPid }]
        const below = st.slice(0, -1).map(x => x.name).lastIndexOf(sh)
        if (below >= 0) {
            const top = st[st.length - 1]
            controller.shellHandBack(top.pid || pane.shellPid, st[below].pid)
            for (let k = st.length - 1; k > below; --k) term.sendText(" exit\r")
            shellStack = st.slice(0, below + 1)
            return
        }
        const line = controller.shellLaunchLine(sh, pane.shellPid)
        if (line === "") return
        st.push({ name: sh, pid: 0 })
        shellStack = st
        term.sendText(line + "\r")
    }
    // (Its pid once it's the shell in front; and a shell left by hand (exit)
    // comes off the list: the one in front is then one below. The daemon
    // reports the name first, then the pid: this goes by the pid.)
    onShellPidChanged: {
        if (shellStack.length < 2 || shellPid === 0) return
        const st = shellStack.slice()
        const top = st[st.length - 1]
        if (top.pid === 0) {
            if (shellName === top.name) { st[st.length - 1] = { name: top.name, pid: shellPid }; shellStack = st }
            return
        }
        const i = st.map(x => x.pid).lastIndexOf(shellPid)
        if (i >= 0 && i < st.length - 1) shellStack = st.slice(0, i + 1)
    }
    Timer {
        id: idleCheck
        interval: 1500
        repeat: true
        running: !term.atPrompt && !pane.idleShell && term.sessionId !== "" && !term.fullScreen && !term.exited
        property int tries: 0
        // (And the other way: a program in front of the shell that the pane
        // didn't see start (it was reattached) is shown, not hidden behind
        // the notebook.)
        property bool sawJob: false
        onTriggered: {
            if (!pane.controller) return
            const job = pane.controller.sessionJob(term.sessionId)
            if (job !== "") {
                tries = 0
                if (!pane.runningLong) { pane.runningLong = true; sawJob = true }
            } else if (++tries >= 2) {
                if (sawJob) { pane.runningLong = false; sawJob = false }
                pane.idleShell = true
                tries = 0
            }
        }
    }
    Connections { target: term; function onPromptChanged() { pane.idleShell = false; idleCheck.tries = 0; if (idleCheck.sawJob) { pane.runningLong = false; idleCheck.sawJob = false } } }      // the window's text zoom, shown with the size
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
        property bool keepsBacktab: true    // (it decides: the editor at a prompt, else the program)
        id: term
        anchors.top: titleBar.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        // (Whether the editor should show, not whether it's on screen: a tab in
        // the background has it hidden, and the terminal mustn't resize then.)
        anchors.bottom: pane.editorOn ? compose.top : parent.bottom
        focus: true
        onContextMenuRequested: (x, y) => pane.contextMenu(term, x, y)
        // At a prompt the keys go to the editor under it, even after a click
        // up here (Shift+Tab in the editor hands them to the terminal); the
        // pane remembers which, and goes back to it when it gets the keys
        // again (switching tabs or panes).
        onActiveFocusChanged: {
            if (activeFocus && pane.editorOn && pane.preferEditor) Qt.callLater(compose.focusEditor)
            // (A notebook over it takes the keys, when the editor doesn't.)
            else if (activeFocus && notebook.visible) Qt.callLater(() => notebook.forceActiveFocus())
        }
        // (At a prompt a click keeps the keys in the editor, unless it's on
        // the cursor's line: then the terminal takes them, as it's typed in.)
        focusOnCursorLineOnly: pane.editorOn
        onCursorLineClicked: pane.preferEditor = false
        onOffCursorLineClicked: { pane.preferEditor = true; compose.focusEditor() }
        // Shift+Tab at a prompt: back to the editor (elsewhere it goes to the program).
        Keys.onPressed: (e) => {
            if (e.key === Qt.Key_Backtab && pane.editorOn && !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier))) {
                pane.preferEditor = true
                compose.focusEditor()
                e.accepted = true
            }
        }
    }

    ComposeBox {
        id: compose
        // (Over the notebook and the views on the terminal: its dropdowns,
        // history and completions, open upward over them.)
        z: 22
        visible: pane.editorOn
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: implicitHeight
        view: term
        controller: pane.controller
        keys: pane.editorKeys
        history: pane.commandHistory
        // (A notebook with a Jupyter kernel: the editor writes its next cell.)
        readonly property bool forNotebook: notebook.visible && !notebook.shellKernel
        autoLang: forNotebook ? notebook.language : pane.replLang !== "" ? pane.replLang : "bash"
        themeData: { try { return JSON.parse(pane.themeJson) } catch (e) { return {} } }
        cwd: term.pwd !== "" ? term.pwd : pane.procCwd
        showPrompt: pane.editorPrompt
        aiInline: pane.aiSettings.inline !== false
        aiCheck: pane.aiSettings.check !== false
        aiNL: pane.aiSettings.natural_language !== false
        shellName: pane.shellName !== "" ? pane.shellName : "bash"
        shellPid: pane.shellPid
        promptEngine: pane.promptEngine
        promptSegments: pane.promptSegments
        onShellSwitchRequested: (sh) => pane.switchShell(sh)
        onSubmitted: (t) => {
            if (forNotebook) { notebook.submit(t); return }
            // (Code in another language than the shell's: its Jupyter kernel,
            // kept alive in the pane's notebook, if there's one; else run by
            // its interpreter, once.)
            const L = compose.runLang
            // (Another shell picked: its kernel, kept alive in the notebook,
            // else run by it once.)
            const S = compose.runShellUsed
            if (S !== "" && controller) {
                if (notebook.hasKernelFor(S)) { pane.viewMode = "notebook"; notebook.submitLang(t, S); return }
                const cmd = controller.snippetCommand(S, t, compose.cwd, compose.shellName)
                if (cmd !== "") { term.pasteText(cmd); term.sendText("\r"); term.scrollToBottom(); return }
            }
            if (pane.replLang === "" && L !== "" && L !== "bash" && controller && notebook.hasKernelFor(L)) {
                pane.viewMode = "notebook"
                notebook.submitLang(t, L)
                return
            }
            if (pane.replLang === "" && L !== "" && L !== "bash" && controller) {
                const cmd = controller.snippetCommand(L, t, compose.cwd, compose.shellName)
                if (cmd !== "") { term.pasteText(cmd); term.sendText("\r"); term.scrollToBottom(); return }
            }
            if (t.indexOf("\n") < 0) term.sendText(t + "\r")
            else { term.pasteText(t); term.sendText("\r") }
            term.scrollToBottom()
        }
        // (The terminal keeps the keys until the editor is clicked again.)
        onHandBack: { pane.preferEditor = false; term.forceActiveFocus() }
        onKeysPicked: (k) => pane.editorKeysPicked(k)
        onOpenRich: pane.openRich()
        viewMode: pane.viewMode
        onViewModePicked: (m) => pane.viewMode = m
        onEditingChanged: if (editing) pane.preferEditor = true
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
    // Terminal or notebook (each command a cell, and cells written there,
    // run by the shell or a Jupyter kernel). A notebook from a file (.ipynb)
    // is this pane's notebook too, its shell starting in the file's folder.
    // The notebook steps aside for a full-screen program, or a command typed
    // in the editor that runs for more than a moment (not one of its own
    // cells: those show their output as it comes).
    property string docPath: ""
    readonly property bool docDirty: docPath !== "" && notebook.dirty
    function saveDoc() { if (docPath !== "") notebook.save() }
    property string viewMode: "terminal"
    property bool runningLong: false
    Timer { id: longTimer; interval: 700; onTriggered: pane.runningLong = true }
    Connections {
        target: term
        function onCellsChanged(id, done) {
            if (done) { longTimer.stop(); pane.runningLong = false }
            else longTimer.restart()
        }
    }
    NotebookView {
        id: notebook
        z: 15
        shellName: pane.shellName
        anchors.fill: term
        visible: pane.viewMode === "notebook" && !term.fullScreen && !(pane.runningLong && !notebook.ownRun)
        enabled: visible
        view: term
        controller: pane.controller
        path: pane.docPath
        storeKey: term.sessionId
        shellPid: pane.shellPid
        fontFamily: term.fontFamily
        markdownStyle: pane.markdownStyle
        onMarkdownStylePicked: (m) => pane.markdownStylePicked(m)
        themeData: { try { return JSON.parse(pane.themeJson) } catch (e) { return {} } }
        onPathPicked: (p) => pane.docPath = p
        onTerminalRequested: { pane.viewMode = "terminal"; term.forceActiveFocus() }
        onCommandRequested: (c) => compose.runCommand(c)
        onInsertRequested: (t) => { compose.focusEditor(); compose.insert(t) }
        onViewRequested: (p) => pane.viewFile(p)
        onVisibleChanged: if (visible && !(pane.editorOn && pane.preferEditor) && pane.focused) Qt.callLater(forceActiveFocus)
    }
    // Files dropped on the pane: their paths, quoted, into the editor (or
    // typed into the terminal when the editor isn't up). The notebook's
    // file browsers take drops themselves.
    DropArea {
        anchors.fill: parent
        keys: ["text/uri-list"]
        z: -1
        onDropped: (drop) => {
            if (!drop.hasUrls) return
            const q = (s) => /^[A-Za-z0-9_\/.~+:@-]+$/.test(s) ? s : "'" + s.replace(/'/g, "'\\''") + "'"
            const paths = drop.urls.map(u => { let p = u.toString().replace(/^file:\/\/[^/]*/, ""); try { p = decodeURIComponent(p) } catch (e) {} return q(p) }).join(" ") + " "
            drop.accept(Qt.CopyAction)
            if (pane.editorOn) { compose.focusEditor(); compose.insert(paths) }
            else term.sendText(paths)
        }
    }
    // The last output's rich view, over the terminal.
    function openRich() {
        let o = {}
        try { o = JSON.parse(term.lastOutput()) } catch (e) {}
        if (!o.kind) return
        rich.kind = o.kind
        rich.text = o.text
        rich.visible = true
        rich.forceActiveFocus()
    }
    RichViewer {
        id: rich
        visible: false
        z: 20
        anchors.fill: term
        fontFamily: term.fontFamily
        themeData: { try { return JSON.parse(pane.themeJson) } catch (e) { return {} } }
        onClosed: { visible = false; if (pane.editorOn) compose.focusEditor(); else term.forceActiveFocus() }
    }
    // A file, shown (View, from a file listing or the terminal's menu).
    function viewFile(p) { preview.path = ""; preview.path = p; preview.visible = true; preview.forceActiveFocus() }
    FilePreview {
        id: preview
        visible: false
        z: 21
        anchors.fill: term
        controller: pane.controller
        fontFamily: term.fontFamily
        pointSize: term.fontPointSize
        themeData: { try { return JSON.parse(pane.themeJson) } catch (e) { return {} } }
        onClosed: { visible = false; path = ""; if (pane.editorOn) compose.focusEditor(); else notebook.forceActiveFocus() }
        onInsertRequested: (t) => { visible = false; compose.focusEditor(); compose.insert(t) }
    }
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

    // Recording (asciicast): a red dot in the corner while it lasts.
    Rectangle {
        visible: term.recording
        z: 30
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.margins: 8
        width: 12; height: 12; radius: 6
        color: "#e04848"
        SequentialAnimation on opacity { running: term.recording; loops: Animation.Infinite; NumberAnimation { to: 0.35; duration: 800 } NumberAnimation { to: 1; duration: 800 } }
        HoverHandler { id: recHover }
        ToolTip.visible: recHover.hovered
        ToolTip.text: "Recording this pane (Session › Stop Recording)"
    }

    // Dim panes without focus when the tab is split (like iTerm2).
    Rectangle {
        anchors.fill: parent
        color: Theme.dark ? "black" : "white"
        opacity: Theme.dark ? pane.dimAmount : pane.dimAmount * 1.5
        visible: pane.dimInactive && !pane.focused && pane.split
    }
}
