import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Tessera

// The AI assistant, beside the terminal: ask questions, have output
// explained, get commands. Each command it suggests has Run (it goes to the
// terminal you're in; you approve each one) and Insert (into its editor to
// change first). After a command it ran finishes, its output goes back to
// the assistant, which carries on.
Rectangle {
    id: panel
    property var controller: null
    property var pane: null            // the active TerminalPane
    signal closeRequested()

    color: Theme.mix(Theme.bg, Theme.chrome, 0.5)

    // [{role: "user"|"assistant"|"output", content}]
    property var messages: []
    property bool busy: false
    property bool testRan: false
    property bool includeTerminal: true
    property bool sendBack: true
    readonly property string key: "chat-" + Math.random().toString(36).slice(2)
    // The conversation shown, kept under the state folder (src/chats.rs) and
    // listed in the header's history dropdown.
    property string chatId: newId()
    property string chatTitle: ""
    function newId() { return Date.now().toString(36) + "-" + Math.random().toString(36).slice(2, 8) }
    readonly property bool ready: controller !== null && controller.revision >= 0 && controller.aiReady("chat")
    readonly property string modelName: {
        controller ? controller.revision : 0
        try { return JSON.parse(controller.settings()).ai.chat_model || "" } catch (e) { return "" }
    }

    function context() {
        const v = pane ? pane.view : null
        let cwd = v ? (v.pwd !== "" ? v.pwd : pane.procCwd) : ""
        cwd = cwd.replace(/^file:\/\/[^/]*/, "")
        try { cwd = decodeURIComponent(cwd) } catch (e) {}
        return { cwd: cwd, shell: pane && pane.shellName ? pane.shellName : "bash" }
    }
    function terminalTail(n) {
        if (!pane || !pane.view) return ""
        const lines = pane.view.contentsText().split("\n")
        while (lines.length && lines[lines.length - 1].trim() === "") lines.pop()
        return lines.slice(-n).join("\n")
    }
    function push(m) { const a = messages.slice(); a.push(m); messages = a; Qt.callLater(list.positionViewAtEnd) }
    function setLast(content) { const a = messages.slice(); a[a.length - 1] = Object.assign({}, a[a.length - 1], { content: content }); messages = a }

    function send(text) {
        if (text.trim() === "" || !ready) return
        let content = text
        if (includeTerminal && pane) {
            const tail = terminalTail(40)
            if (tail !== "") content += "\n\n(The terminal's last lines:)\n```\n" + tail + "\n```"
        }
        push({ role: "user", content: content, shown: text })
        ask()
    }
    function ask() {
        // The conversation as the model sees it ("output" turns are the
        // user's messages too).
        const msgs = messages.filter(m => m.role !== "error").map(m => ({ role: m.role === "assistant" ? "assistant" : "user", content: m.content }))
        push({ role: "assistant", content: "" })
        busy = true
        controller.aiRun(key, "chat", JSON.stringify(msgs), JSON.stringify(context()))
    }
    function stop() { controller.aiCancel(key); busy = false }
    function clear() { stop(); saveNow(); openSeq++; messages = []; chatId = newId(); chatTitle = "" }

    // ---- history -------------------------------------------------------------
    // Saved a moment after each change (and right away before switching).
    Timer { id: saveTimer; interval: 400; onTriggered: panel.saveNow() }
    onMessagesChanged: saveTimer.restart()
    function saveNow() {
        saveTimer.stop()
        if (!controller) return
        const keep = messages.filter(m => !(m.role === "assistant" && m.content === ""))
        controller.chatSave(chatId, JSON.stringify(keep))
        if (keep.length > 0 && chatTitle === "") {
            const first = keep.find(m => m.role === "user")
            if (first) chatTitle = (first.shown || first.content).split("\n")[0].trim()
        }
    }
    function open(id, title) {
        if (id === chatId) return
        stop(); saveNow()
        const s = ++openSeq
        Async.call(controller, k => controller.chatLoad(k, id), r => {
            if (s !== openSeq) return   // (another chat opened, or cleared, since)
            let msgs = []
            try { msgs = JSON.parse(r) } catch (e) {}
            chatId = id
            chatTitle = title || ""
            messages = msgs
            saveTimer.stop()
            Qt.callLater(list.positionViewAtEnd)
        })
    }
    property int openSeq: 0
    function remove(id) {
        controller.chatDelete(id)
        if (id === chatId) { stop(); openSeq++; messages = []; chatId = newId(); chatTitle = ""; saveTimer.stop() }
    }
    // "Today 14:05", "Yesterday", "Mon", "12 Nov", "12 Nov 2025"
    function when(secs) {
        const d = new Date(secs * 1000), now = new Date()
        const day = (x) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime()
        const days = Math.round((day(now) - day(d)) / 86400000)
        if (days === 0) return d.toLocaleTimeString(Qt.locale(), "HH:mm")
        if (days === 1) return "Yesterday"
        if (days < 7) return d.toLocaleDateString(Qt.locale(), "ddd")
        if (d.getFullYear() === now.getFullYear()) return d.toLocaleDateString(Qt.locale(), "d MMM")
        return d.toLocaleDateString(Qt.locale(), "d MMM yyyy")
    }
    // The last conversation comes back with the window.
    Component.onCompleted: Qt.callLater(() => {
        if (!controller) return
        Async.call(controller, k => controller.chatsList(k), r => {
            let all = []
            try { all = JSON.parse(r) } catch (e) {}
            // (Not if a chat was started or opened meanwhile.)
            if (all.length > 0 && openSeq === 0 && messages.length === 0) open(all[0].id, all[0].title)
        })
        // (Test hooks: TESSERA_TEST_CHAT_HISTORY opens the dropdown;
        // TESSERA_TEST_CHAT_SEARCH=<text> also types that in its search.)
        if (controller.testEnv("CHAT_HISTORY") !== "" || controller.testEnv("CHAT_SEARCH") !== "") historyTest.start()
    })
    Timer {
        id: historyTest
        interval: 6000
        onTriggered: {
            history.openIt()
            const q = panel.controller.testEnv("CHAT_SEARCH")
            if (q !== "") search.text = q
            // (TESSERA_TEST_CHAT_CHOOSE: and open the first match.)
            if (panel.controller.testEnv("CHAT_CHOOSE") !== "") history.refresh(() => history.choose(0))
        }
    }

    Connections {
        target: panel.controller
        function onAiDelta(k, text) {
            if (k !== panel.key) return
            panel.setLast(panel.messages[panel.messages.length - 1].content + text)
            Qt.callLater(list.positionViewAtEnd)
        }
        function onAiDone(k, reply, error) {
            if (k !== panel.key) return
            panel.busy = false
            if (error) { panel.setLast(""); const a = panel.messages.slice(0, -1); a.push({ role: "error", content: error }); panel.messages = a }
            else if (reply !== "") panel.setLast(reply)
            // (Test hook: run the first command it suggests.)
            if (!error && panel.controller.testEnv("ASSISTANT_RUN") !== "" && !panel.testRan) {
                const c = panel.parts(reply).find(p => p.code)
                if (c) { panel.testRan = true; panel.run(c.text) }
            }
        }
    }

    // ---- running a command it suggested ------------------------------------
    property string waitingFor: ""     // the command it's running, until it finishes
    property var waitingView: null
    function run(cmd) {
        if (!pane || !pane.view) return
        const v = pane.view
        waitingFor = cmd
        waitingView = v
        startLine = v.contentsText().split("\n").length
        if (cmd.indexOf("\n") < 0) v.sendText(cmd + "\r")
        else { v.pasteText(cmd); v.sendText("\r") }
        v.scrollToBottom()
        if (pane.controller) pane.controller.historyAdd(cmd, context().cwd)
    }
    property int startLine: 0
    function insert(cmd) {
        if (!pane) return
        pane.compose.setText(cmd)
        pane.compose.focusEditor()
    }
    Connections {
        target: panel.waitingView
        function onCommandFinished() {
            if (panel.waitingFor === "") return
            const v = panel.waitingView
            const all = v.contentsText().split("\n")
            while (all.length && all[all.length - 1].trim() === "") all.pop()
            // What came out since it was sent (the screen may have scrolled:
            // at most the last 120 lines).
            const out = all.slice(Math.max(panel.startLine - 2, all.length - 120)).join("\n")
            const cmd = panel.waitingFor
            panel.waitingFor = ""
            panel.push({ role: "output", content: "I ran `" + cmd + "` (exit status " + v.lastStatus + "). Output:\n```\n" + out.slice(-6000) + "\n```", shown: cmd, status: v.lastStatus })
            if (panel.sendBack && panel.ready) panel.ask()
        }
    }

    // A reply split into text and code blocks.
    function parts(text) {
        const out = []
        const re = /```([^\n`]*)\n([\s\S]*?)(```|$)/g
        let last = 0, m
        while ((m = re.exec(text)) !== null) {
            if (m.index > last) out.push({ code: false, text: text.substring(last, m.index) })
            out.push({ code: true, lang: m[1].trim(), text: m[2].replace(/\n$/, "") })
            last = re.lastIndex
            if (m[3] === "") break
        }
        if (last < text.length) out.push({ code: false, text: text.substring(last) })
        return out
    }

    // ---- layout --------------------------------------------------------------
    Rectangle { width: 1; height: parent.height; color: Theme.divider }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: 1
        spacing: 0

        // Header
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 10
            spacing: 8
            Text { text: "✦"; color: Theme.accent; font.pixelSize: 15 }
            // The conversation's title: opens the history dropdown.
            Rectangle {
                id: titleButton
                Layout.fillWidth: true
                Layout.preferredHeight: titleCol.implicitHeight + 6
                radius: 6
                color: titleHover.hovered || history.opened ? Theme.mix(Theme.bg, Theme.fg, 0.07) : "transparent"
                ColumnLayout {
                    id: titleCol
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: 4
                    anchors.rightMargin: 4
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 0
                    RowLayout {
                        spacing: 4
                        Layout.fillWidth: true
                        Text {
                            text: panel.chatTitle !== "" ? panel.chatTitle : "Assistant"
                            color: Theme.text; font.pixelSize: 13; font.weight: Font.DemiBold
                            elide: Text.ElideRight
                            Layout.maximumWidth: titleCol.width - 20
                        }
                        Text { text: "\u2304"; color: Theme.textDim; font.pixelSize: 13; font.bold: true; Layout.alignment: Qt.AlignTop }
                        Item { Layout.fillWidth: true }
                    }
                    Text {
                        text: panel.ready ? panel.modelName : "No model set up — Preferences › AI"
                        color: Theme.textDim; font.pixelSize: 11; elide: Text.ElideRight; Layout.fillWidth: true
                    }
                }
                HoverHandler { id: titleHover; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: history.opened ? history.close() : history.openIt() }
                ToolTip.visible: titleHover.hovered && !history.opened
                ToolTip.delay: 600
                ToolTip.text: "Past conversations"
            }
            BarButton { icon: "plus"; tip: "New conversation"; onClicked: panel.clear() }
            BarButton { icon: "close"; tip: "Close (Ctrl+Shift+I)"; onClicked: panel.closeRequested() }
        }
        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            spacing: 10
            topMargin: 10
            bottomMargin: 10
            model: panel.messages
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar {}
            MomentumWheel { flick: list }
            delegate: Item {
                id: msg
                required property var modelData
                required property int index
                width: list.width
                height: body.implicitHeight
                readonly property bool mine: modelData.role === "user"
                Column {
                    id: body
                    x: 12
                    width: msg.width - 24
                    spacing: 6
                    // You
                    Rectangle {
                        visible: msg.mine
                        anchors.right: parent.right
                        width: Math.min(parent.width * 0.9, youText.implicitWidth + 20)
                        height: youText.implicitHeight + 14
                        radius: 10
                        color: Theme.mix(Theme.bg, Theme.accent, 0.25)
                        Text {
                            id: youText
                            anchors.centerIn: parent
                            width: Math.min(body.width * 0.9 - 20, implicitWidth)
                            text: msg.modelData.shown || msg.modelData.content
                            wrapMode: Text.Wrap
                            color: Theme.text
                            font.pixelSize: 13
                            textFormat: Text.PlainText
                        }
                    }
                    // A command's result
                    Text {
                        visible: msg.modelData.role === "output"
                        width: parent.width
                        text: (msg.modelData.status === 0 ? "✓ " : "✘ ") + "ran " + (msg.modelData.shown || "") + (msg.modelData.status === 0 ? "" : "  (exit " + msg.modelData.status + ")")
                        color: msg.modelData.status === 0 ? Theme.textDim : "#e06c75"
                        font.pixelSize: 11
                        elide: Text.ElideRight
                    }
                    Text {
                        visible: msg.modelData.role === "error"
                        width: parent.width
                        text: "⚠ " + msg.modelData.content
                        color: "#e06c75"
                        wrapMode: Text.Wrap
                        font.pixelSize: 12
                    }
                    // The assistant: text and code blocks
                    Repeater {
                        model: msg.modelData.role === "assistant" ? panel.parts(msg.modelData.content) : []
                        delegate: Item {
                            id: part
                            required property var modelData
                            width: body.width
                            height: modelData.code ? codeBox.height : mdText.implicitHeight
                            Text {
                                id: mdText
                                visible: !part.modelData.code
                                width: parent.width
                                text: part.modelData.text.trim()
                                textFormat: Text.MarkdownText
                                wrapMode: Text.Wrap
                                color: Theme.text
                                linkColor: Theme.accent
                                font.pixelSize: 13
                                onLinkActivated: (l) => Qt.openUrlExternally(l)
                            }
                            Rectangle {
                                id: codeBox
                                visible: part.modelData.code
                                width: parent.width
                                height: code.implicitHeight + 16 + (actions.visible ? actions.height + 6 : 0)
                                radius: 8
                                color: Theme.bg
                                border.color: Theme.divider
                                TextEdit {
                                    id: code
                                    x: 10; y: 8
                                    width: parent.width - 20
                                    text: part.modelData.text
                                    readOnly: true
                                    selectByMouse: true
                                    wrapMode: TextEdit.Wrap
                                    color: Theme.text
                                    font.family: panel.pane && panel.pane.view ? panel.pane.view.fontFamily : "monospace"
                                    font.pointSize: 10
                                    renderType: Text.NativeRendering
                                }
                                Row {
                                    id: actions
                                    readonly property bool shell: ["", "bash", "sh", "zsh", "fish", "shell", "console", "powershell", "pwsh", "cmd"].indexOf((part.modelData.lang || "").toLowerCase()) >= 0
                                    visible: shell
                                    anchors.right: parent.right
                                    anchors.rightMargin: 8
                                    anchors.bottom: parent.bottom
                                    anchors.bottomMargin: 6
                                    spacing: 6
                                    component Act: Rectangle {
                                        id: act
                                        property string label
                                        property bool primary: false
                                        property bool enabledAct: true
                                        signal clicked()
                                        width: actText.implicitWidth + 16
                                        height: 22
                                        radius: 6
                                        opacity: enabledAct ? 1 : 0.4
                                        color: primary ? (actHover.hovered ? Qt.lighter(Theme.accent, 1.1) : Theme.accent)
                                                       : (actHover.hovered ? Theme.tabHover : Theme.mix(Theme.bg, Theme.fg, 0.06))
                                        Text { id: actText; anchors.centerIn: parent; text: act.label; color: act.primary ? Theme.bg : Theme.text; font.pixelSize: 11; font.weight: Font.DemiBold }
                                        HoverHandler { id: actHover }
                                        TapHandler { onTapped: if (act.enabledAct) act.clicked() }
                                    }
                                    Act { label: "Copy"; onClicked: { clip.text = part.modelData.text; clip.selectAll(); clip.copy() } }
                                    Act { label: "Insert"; enabledAct: panel.pane !== null; onClicked: panel.insert(part.modelData.text) }
                                    Act {
                                        label: panel.waitingFor !== "" ? "Running…" : "▶ Run"
                                        primary: true
                                        enabledAct: panel.pane !== null && panel.waitingFor === "" && panel.pane.view.atPrompt
                                        onClicked: panel.run(part.modelData.text)
                                    }
                                }
                            }
                        }
                    }
                    Text {
                        visible: msg.modelData.role === "assistant" && msg.modelData.content === "" && panel.busy
                        text: "thinking…"
                        color: Theme.textDim
                        font.pixelSize: 12
                        font.italic: true
                    }
                }
            }
            Text {
                anchors.centerIn: parent
                width: parent.width - 40
                visible: panel.messages.length === 0
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                text: panel.ready
                      ? "Ask anything about your terminal. The assistant sees the folder you're in and the terminal's last lines; commands it suggests run when you press Run, and their output comes back to it."
                      : "Set up a model in Preferences › AI (a local one with Ollama, or Claude, OpenAI, Gemini, ...)."
                color: Theme.textDim
                font.pixelSize: 12
            }
        }
        TextEdit { id: clip; visible: false }

        Rectangle { Layout.fillWidth: true; height: 1; color: Theme.divider }
        // Options
        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 10
            Layout.rightMargin: 10
            Layout.topMargin: 6
            spacing: 12
            component Opt: Row {
                id: opt
                property string label
                property bool on
                signal toggled()
                spacing: 6
                Rectangle {
                    width: 26; height: 14; radius: 7
                    anchors.verticalCenter: parent.verticalCenter
                    color: opt.on ? Theme.accent : Theme.mix(Theme.bg, Theme.fg, 0.18)
                    Rectangle { width: 10; height: 10; radius: 5; y: 2; x: opt.on ? 14 : 2; color: Theme.bg; Behavior on x { NumberAnimation { duration: 90 } } }
                }
                Text { text: opt.label; color: Theme.textDim; font.pixelSize: 11; anchors.verticalCenter: parent.verticalCenter }
                TapHandler { onTapped: opt.toggled() }
            }
            Opt { label: "Include terminal"; on: panel.includeTerminal; onToggled: panel.includeTerminal = !panel.includeTerminal }
            Opt { label: "Send results back"; on: panel.sendBack; onToggled: panel.sendBack = !panel.sendBack }
            Item { Layout.fillWidth: true }
        }
        // Input
        Rectangle {
            Layout.fillWidth: true
            Layout.margins: 10
            Layout.preferredHeight: Math.min(160, Math.max(38, input.contentHeight + 16))
            radius: 8
            color: Theme.bg
            border.color: input.activeFocus ? Theme.accent : Theme.divider
            Flickable {
                id: inFlick
                anchors.fill: parent
                anchors.margins: 8
                anchors.rightMargin: 40
                contentHeight: input.contentHeight
                clip: true
                TextEdit {
                    id: input
                    width: inFlick.width
                    wrapMode: TextEdit.Wrap
                    color: Theme.text
                    selectionColor: Theme.accent
                    selectedTextColor: Theme.bg
                    font.pixelSize: 13
                    Keys.onPressed: (e) => {
                        if ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter) && !(e.modifiers & Qt.ShiftModifier)) {
                            if (!panel.busy) { panel.send(text); text = "" }
                            e.accepted = true
                        } else if (e.key === Qt.Key_Escape && panel.busy) { panel.stop(); e.accepted = true }
                    }
                }
                Text {
                    visible: input.text === ""
                    text: panel.busy ? "Esc stops it" : "Ask, or describe what you want to do…"
                    color: Theme.textDim
                    opacity: 0.7
                    font.pixelSize: 13
                }
            }
            Rectangle {
                anchors.right: parent.right
                anchors.rightMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                width: 28; height: 28; radius: 6
                color: sendHover.hovered ? Theme.tabHover : "transparent"
                Text { anchors.centerIn: parent; text: panel.busy ? "■" : "↑"; color: panel.busy ? "#e06c75" : Theme.accent; font.pixelSize: 15; font.weight: Font.Bold }
                HoverHandler { id: sendHover }
                TapHandler { onTapped: { if (panel.busy) panel.stop(); else { panel.send(input.text); input.text = "" } } }
            }
        }
    }
    // ---- the history dropdown: past conversations, newest first, and a search
    // through their text -----------------------------------------------------
    // (An item over the panel rather than a Popup: it always fits inside the
    // panel, and window screenshots then show it.)
    MouseArea {
        anchors.fill: parent
        visible: history.opened
        z: 9
        onPressed: history.close()
        onWheel: (w) => w.accepted = true
    }
    Rectangle {
        id: history
        property bool opened: false
        function open() { opened = true }
        function close() { opened = false }
        visible: opened
        z: 10
        property var rows: []
        property int current: 0
        property int seq: 0
        function refresh(then) {
            panel.saveNow()
            const s = ++seq, q = search.text
            Async.call(panel.controller, k => q.trim() === "" ? panel.controller.chatsList(k) : panel.controller.chatsSearch(k, q), r => {
                if (s !== seq) return   // (the search changed since)
                try { rows = JSON.parse(r) } catch (e) { rows = [] }
                current = 0
                if (then) then()
            })
        }
        function openIt() {
            // (On whole pixels in the window, plus the nudge: native text at
            // 130% smears on half pixels.)
            const g = titleButton.mapToItem(null, 0, titleButton.height + 4), o = panel.mapToItem(null, 0, 0)
            x = Math.round(o.x + Math.max(8, Math.min(g.x - o.x, panel.width - width - 8))) - o.x + 0.001
            y = Math.round(g.y) - o.y + 0.001
            search.text = ""
            refresh()
            open()
            search.forceActiveFocus()
        }
        function choose(i) {
            if (i < 0 || i >= rows.length) return
            panel.open(rows[i].id, rows[i].title)
            close()
            panel.focusInput()
        }
        width: Math.min(panel.width - 16, 420)
        height: Math.min(panel.height - y - 10, header.implicitHeight + Math.max(rowsView.contentHeight, 40) + 16)
        radius: 8
        color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.03)
        border.color: Theme.divider
        MouseArea { anchors.fill: parent; onWheel: (w) => w.accepted = true }
        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 6
            spacing: 4
            ColumnLayout {
                id: header
                spacing: 4
                Layout.fillWidth: true
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 30
                    radius: 6
                    color: Theme.bg
                    border.color: search.activeFocus ? Theme.accent : Theme.divider
                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 8
                        anchors.rightMargin: 4
                        spacing: 4
                        Text { text: "⌕"; color: Theme.textDim; font.pixelSize: 14 }
                        TextField {
                            id: search
                            Layout.fillWidth: true
                            placeholderText: "Search conversations"
                            placeholderTextColor: Theme.textDim
                            color: Theme.text
                            font.pixelSize: 12
                            selectByMouse: true
                            background: Item {}
                            onTextChanged: if (history.opened) history.refresh()
                            Keys.onDownPressed: history.current = Math.min(history.rows.length - 1, history.current + 1)
                            Keys.onUpPressed: history.current = Math.max(0, history.current - 1)
                            Keys.onReturnPressed: history.choose(history.current)
                            Keys.onEnterPressed: history.choose(history.current)
                            Keys.onEscapePressed: { history.close(); panel.focusInput() }
                        }
                    }
                }
                // A new one, at the top.
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 26
                    visible: search.text === ""
                    radius: 5
                    color: newHover.hovered ? Theme.tabHover : "transparent"
                    Text { x: 8; anchors.verticalCenter: parent.verticalCenter; text: "+  New conversation"; color: Theme.accent; font.pixelSize: 12 }
                    HoverHandler { id: newHover; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: { panel.clear(); history.close(); panel.focusInput() } }
                }
            }
            ListView {
                id: rowsView
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: history.rows
                boundsBehavior: Flickable.StopAtBounds
                currentIndex: history.current
                onCurrentIndexChanged: positionViewAtIndex(currentIndex, ListView.Contain)
                ScrollBar.vertical: ScrollBar {}
                delegate: Rectangle {
                    id: row
                    required property var modelData
                    required property int index
                    width: rowsView.width
                    height: rowCol.implicitHeight + 10
                    radius: 5
                    readonly property bool here: modelData.id === panel.chatId
                    color: index === history.current || rowHover.hovered ? Theme.tabHover : "transparent"
                    ColumnLayout {
                        id: rowCol
                        x: 8
                        y: 5
                        width: row.width - 16 - (rowHover.hovered ? 22 : 0)
                        spacing: 1
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8
                            Text {
                                Layout.fillWidth: true
                                text: (row.here ? "✓ " : "") + (row.modelData.title || "New conversation")
                                color: Theme.text
                                font.pixelSize: 12
                                font.weight: row.here ? Font.DemiBold : Font.Normal
                                elide: Text.ElideRight
                            }
                            Text { text: panel.when(row.modelData.updated || 0); color: Theme.textDim; font.pixelSize: 11 }
                        }
                        Text {
                            Layout.fillWidth: true
                            visible: text !== ""
                            text: row.modelData.snippet || ""
                            color: Theme.textDim
                            font.pixelSize: 11
                            elide: Text.ElideRight
                            textFormat: Text.PlainText
                        }
                    }
                    HoverHandler { id: rowHover; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: history.choose(row.index) }
                    // Forget it.
                    Rectangle {
                        visible: rowHover.hovered
                        anchors.right: parent.right
                        anchors.rightMargin: 4
                        anchors.verticalCenter: parent.verticalCenter
                        width: 20; height: 20; radius: 4
                        color: delHover.hovered ? Theme.mix(Theme.bg, Theme.fg, 0.15) : "transparent"
                        Text { anchors.centerIn: parent; text: "✕"; color: delHover.hovered ? "#e06c75" : Theme.textDim; font.pixelSize: 11 }
                        HoverHandler { id: delHover }
                        TapHandler { onTapped: { panel.remove(row.modelData.id); history.refresh() } }
                        ToolTip.visible: delHover.hovered
                        ToolTip.delay: 600
                        ToolTip.text: "Delete this conversation"
                    }
                }
                Text {
                    anchors.centerIn: parent
                    visible: history.rows.length === 0
                    text: search.text === "" ? "No past conversations" : "Nothing matches"
                    color: Theme.textDim
                    font.pixelSize: 12
                }
            }
        }
    }
    function focusInput() { input.forceActiveFocus() }
}
