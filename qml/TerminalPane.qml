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
    // Where its shell runs: "" here, else an SSH host (its tesserad, set up
    // there when the pane opens; the shell waits for that).
    property alias sshHost: term.host
    // (Set once the pane knows where it is, not as a binding: the shell
    // mustn't open before an SSH host has been given.)
    property bool hostUp: false
    property string hostError: ""
    // (A saved remote pane's screen, shown when its shell there is gone.)
    property string restoreSnapshot: ""
    property bool closingWithWindow: false  // the window closes: its notebooks' kernels stay
    function connectHost() { hostError = ""; askInstall = ""; controller.connectHost(sshHost) }
    // The host has no tesserad and nobody has said to install it: asked here
    // ("<os> <arch>" while asking). Install: it's copied there and the pane
    // goes on. Plain SSH: the pane runs ssh to the host from here, as a
    // terminal does without Tessera there (no notebook kernels or files
    // there, nothing kept when the connection drops).
    property string askInstall: ""
    property bool rememberAnswer: true
    function hostChoice() { try { return (JSON.parse(controller.settings()).ssh_hosts || {})[sshHost] || "" } catch (e) { return "" } }
    function answerInstall(install) {
        if (rememberAnswer) {
            let m = {}
            try { m = JSON.parse(controller.settings()).ssh_hosts || {} } catch (e) {}
            m[sshHost] = install ? "tesserad" : "ssh"
            controller.setSetting("ssh_hosts", JSON.stringify(m))
        }
        if (install) { controller.allowTesseradInstall(sshHost); connectHost(); return }
        usePlainSsh()
    }
    function usePlainSsh() {
        const h = sshHost
        askInstall = ""
        commandOverride = "ssh " + h
        sshHost = ""
        hostUp = true
    }
    Component.onCompleted: {
        if (sshHost === "") hostUp = true
        else if (controller && hostChoice() === "ssh") usePlainSsh()
        else if (controller) connectHost()
    }
    onControllerChanged: if (sshHost !== "" && !hostUp && controller && hostError === "") connectHost()
    Connections {
        target: pane.controller
        enabled: pane.sshHost !== "" && !pane.hostUp
        function onHostConnected(h, err) {
            if (h !== pane.sshHost) return
            if (err.startsWith("tesserad-missing:")) {
                pane.askInstall = err.substring(17).trim()
                // (Test hook: TESSERA_TEST_TESSERAD_ANSWER=install|ssh.)
                const a = pane.controller.testEnv("TESSERAD_ANSWER")
                if (a !== "") Qt.callLater(pane.answerInstall, a === "install")
                return
            }
            if (err !== "") { pane.hostError = err; return }
            // (Reattaching: when the shell there has ended, a new one starts
            // under the saved screen.)
            const up = () => { pane.hostUp = true; notebook.listKernels() }
            if (term.attachId === "") { up(); return }
            Async.call(pane.controller, k => pane.controller.restorableSessions(k, h), r => {
                if (!term || h !== pane.sshHost || pane.hostUp) return
                let ids = []
                try { ids = JSON.parse(r) } catch (e) {}
                if (ids.indexOf(term.attachId) < 0) {
                    term.attachId = ""
                    if (pane.restoreSnapshot !== "") term.preloadFile = pane.restoreSnapshot
                }
                up()
            })
        }
    }
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
    property string notebookKernel: "tessh"  // what the notebook runs in until one is picked
    property bool preferEditor: true   // the keys go to the editor (not the terminal) when the pane gets them
    property bool inputEditor: false
    property string editorKeys: "emacs"
    property bool editorPrompt: true
    property var aiSettings: ({})
    property string promptEngine: "auto"
    property int shellPid: 0            // from the daemon (for the prompt's environment)
    property var promptSegments: ["dir", "git", "venv", "nix", "status", "duration", "|", "kube", "aws", "gcloud", "host", "time"]
    property var commandHistory: []
    property var historyLangs: ({})
    property var controller: null
    // REPLs the editor also serves (the shell needs its integration's prompt marks).
    readonly property var replPrograms: ["python", "python3", "ipython", "bpython", "node", "deno", "bun", "tessh"]
    readonly property string replLang: {
        const j = pane.job.replace(/^.*\//, "")
        return replPrograms.some(r => j === r || j.startsWith("python")) && controller ? controller.languageFor(j) : ""
    }
    onEditorOnChanged: {
        // (Back at the prompt after a command or a full-screen program like
        // vim: the editor takes the keys again if it had them before; keys
        // given to the terminal (a click there, a tap of Ctrl) stay there.)
        // (Not while a notebook card has the terminal live: its program reads the keys.)
        if (editorOn && focused && preferEditor && termSlot === null) Qt.callLater(compose.focusEditor)
        else if (!editorOn && compose.editing) term.forceActiveFocus()
    }
    // (A program that takes the screen: full-screen ones, or ones drawn in
    // place that read keys themselves. A REPL the editor serves (python's
    // own turns on bracketed paste) isn't one: its lines come from here.)
    // (Which program it is is asked the moment it takes over, and the
    // editor stays until the answer: a REPL then doesn't flash it away.)
    readonly property bool takenOver: term.fullScreen && replLang === "" && fsKnown
    property bool fsKnown: false
    readonly property bool fsNow: term.fullScreen
    onFsNowChanged: {
        if (!fsNow) { fsKnown = false; fsWait.stop(); return }
        fsWait.restart()
        const id = term.sessionId
        if (!controller || id === "") { fsKnown = true; return }
        Async.call(controller, k => controller.sessionJob(k, id), job => {
            if (!term || term.sessionId !== id || !pane.fsNow) return
            if (pane.job !== job) pane.job = job
            pane.fsKnown = true
        })
    }
    Timer { id: fsWait; interval: 400; onTriggered: pane.fsKnown = true }
    // (Put away by the user (its top edge, Ctrl+Shift+H, View ▸ Show Input
    // Box): a thin strip stays to bring it back.)
    property bool inputHidden: false
    // (A notebook starts with the input box out: its cells are run from it.)
    onViewModeChanged: if (viewMode === "notebook" && inputHidden) setInputHidden(false)
    readonly property bool stripShown: inputEditor && inputHidden && !term.exited && !takenOver
    function setInputHidden(h) {
        inputHidden = h
        if (h) term.forceActiveFocus()
        else { preferEditor = true; Qt.callLater(() => { if (pane.editorOn) compose.focusEditor() }) }
    }
    readonly property bool editorOn: inputEditor && !inputHidden && !term.exited && !takenOver && (promptHeld || editorCalled)
    // (Shown while a command runs too, the keys the program's: the pane
    // doesn't come and go, and a click or a tap of left Ctrl there types
    // the next command, queued. Only a full-screen program takes it away.)
    readonly property bool editorShown: inputEditor && !inputHidden && !term.exited && !takenOver
    // (At the prompt, or was a moment ago: a quick command (ls) leaves the
    // editor up rather than taking it away and back, which resizes
    // everything above it twice. One that's still going after 250 ms hides
    // it; a full-screen program does at once.)
    readonly property bool promptish: term.atPrompt || idleShell || replLang !== "" || awaitingPrompt
    property bool promptHeld: promptish
    onPromptishChanged: { if (promptish) { promptDrop.stop(); promptHeld = true; if (cmdQueue.length) queueTimer.restart() } else promptDrop.restart() }
    // Commands typed while one ran (Enter in the editor then): run in turn,
    // each at the next prompt.
    property var cmdQueue: []
    // (A moment after the prompt comes: sent the instant it starts, the
    // command was echoed above the prompt as the shell drew it.)
    Timer { id: queueTimer; interval: 200; onTriggered: pane.runQueued() }
    property bool flushing: false
    function runQueued() {
        if (!promptish || cmdQueue.length === 0) return
        const q = cmdQueue[0]
        cmdQueue = cmdQueue.slice(1)
        const L = compose.runLang, S = compose.runShellUsed
        compose.runLang = q.L; compose.runShellUsed = q.S; compose.submitMode = ""
        flushing = true
        compose.submitted(q.t)
        flushing = false
        compose.runLang = L; compose.runShellUsed = S
    }
    // (Called up while a command runs (a tap of left Ctrl): the editor's there
    // to type the next one, queued; it goes when the keys go back.)
    property bool editorCalled: false
    Connections { target: compose; function onEditingChanged() { if (!compose.editing) Qt.callLater(() => { if (!compose.editing) pane.editorCalled = false }) } }
    Timer { id: promptDrop; interval: 250; onTriggered: pane.promptHeld = pane.promptish }
    // (A new shell: the editor's room kept from the start, so the terminal
    // opens at the size it'll have at the prompt instead of shrinking when
    // the prompt comes. Not for a command of its own or a reattached shell;
    // given up if no prompt says so within a few seconds.)
    property bool awaitingPrompt: term.command === "" && term.attachId === ""
    Timer { running: pane.awaitingPrompt; interval: 4000; onTriggered: pane.awaitingPrompt = false }
    Connections { target: term; enabled: pane.awaitingPrompt; function onAtPromptChanged() { pane.awaitingPrompt = false } }
    // (Reattached after a restart, the shell may be at its prompt without
    // having said so since: nothing running in front of it counts as that,
    // until the shell integration says otherwise.)
    property bool idleShell: false
    // TEMPORARY: why the input editor is off (a notebook tab with no input box in Ben's window).
    Connections { target: pane; function onEditorOnChanged() { console.warn("editor " + (pane.editorOn ? "on" : "off") + ": tab view=" + pane.viewMode + " inputEditor=" + pane.inputEditor + " exited=" + term.exited + " fullScreen=" + term.fullScreen + " atPrompt=" + term.atPrompt + " idleShell=" + pane.idleShell + " replLang=" + pane.replLang + " session=" + term.sessionId) } }
    Timer { interval: 8000; running: true; onTriggered: if (!pane.editorOn) console.warn("editor off after start: tab view=" + pane.viewMode + " inputEditor=" + pane.inputEditor + " exited=" + term.exited + " fullScreen=" + term.fullScreen + " atPrompt=" + term.atPrompt + " idleShell=" + pane.idleShell + " runningLong=" + pane.runningLong + " session=" + term.sessionId) }
    // Switching the pane's shell (the editor's language list): to one it was
    // started from, the ones above it are left (exit), what they changed
    // handed down; to another, it's started in the current one. (Which shell
    // was started in which: from the processes, so it holds across restarts.)
    property int shellCmdPending: 0     // shell starts and exits sent, their cells still to come
    property var shellCmds: []          // those cells' ids
    function switchShell(sh) {
        if (!controller || term.exited || !(term.atPrompt || idleShell)) return
        const cur = pane.shellName !== "" ? pane.shellName : "bash"
        const pid = pane.shellPid, host = pane.sshHost
        // (The chain, then the hand back or the launch line: one ordered
        // worker, so they stay in order. A pane whose shell changed since
        // drops the answer.)
        Async.call(controller, k => controller.shellChain(k, pid, host), r => {
            if (!term || term.exited || pane.shellPid !== pid || pane.sshHost !== host) return
            let st = []
            try { st = JSON.parse(r) } catch (e) {}
            if (st.length === 0) st = [{ name: cur, pid: pid }]
            const below = st.slice(0, -1).map(x => x.name).lastIndexOf(sh)
            if (below >= 0) {
                controller.shellHandBack(st[st.length - 1].pid, st[below].pid, host)
                for (let k = st.length - 1; k > below; --k) { shellCmdPending++; term.sendText(" exit\r") }
                return
            }
            Async.call(controller, k => controller.shellLaunchLine(k, sh, pid, host), line => {
                if (!term || term.exited || pane.shellPid !== pid || line === "") return
                shellCmdPending++
                term.sendText(line + "\r")
            })
        })
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
        property bool asking: false     // (a tick's question still in flight: the next is skipped)
        onTriggered: {
            if (!pane.controller || asking) return
            asking = true
            const id = term.sessionId
            Async.call(pane.controller, k => pane.controller.sessionJob(k, id), job => {
                if (!term) return
                idleCheck.asking = false
                // (Stale: the session changed, or the prompt came back meanwhile.)
                if (term.sessionId !== id || !idleCheck.running) return
                if (job !== "") {
                    idleCheck.tries = 0
                    if (!pane.runningLong) { pane.runningLong = true; idleCheck.sawJob = true }
                } else if (++idleCheck.tries >= 2) {
                    if (idleCheck.sawJob) { pane.runningLong = false; idleCheck.sawJob = false }
                    pane.idleShell = true
                    idleCheck.tries = 0
                }
            })
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

    // Where the terminal sits in the pane. While a full-screen program runs
    // in notebook mode, the terminal itself moves into that command's card
    // (termSlot, the notebook's), so the program runs in the card at its size.
    property Item termSlot: notebook.liveSlot
    Item {
        id: termHome
        anchors.top: titleBar.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        // (Whether the editor should show, not whether it's on screen: a tab in
        // the background has it hidden, and the terminal mustn't resize then.)
        anchors.bottom: pane.editorShown ? compose.top : pane.stripShown ? inputStrip.top : parent.bottom
    }
    // The input box put away: a strip to bring it back.
    Rectangle {
        id: inputStrip
        z: 22
        visible: pane.stripShown
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 7
        color: stripHover.hovered ? Theme.mix(Theme.bg, Theme.accent, 0.3) : Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.045 : 0.035)
        Rectangle { width: parent.width; height: 1; color: Theme.divider }
        Rectangle {
            width: 36; height: 3; radius: 1.5
            x: Math.round((parent.width - width) / 2)
            y: 2
            color: Theme.textDim
            opacity: stripHover.hovered ? 1 : 0.6
        }
        HoverHandler { id: stripHover; cursorShape: Qt.PointingHandCursor }
        TapHandler { onTapped: pane.setInputHidden(false) }
        ToolTip.visible: stripHover.hovered
        ToolTip.delay: 600
        ToolTip.text: "Show the input box (Ctrl+Shift+H)"
    }
    TerminalView {
        id: term
        ready: pane.hostUp
        // (Ctrl+click on a path in another host's output: Tessera's viewer, read there.)
        onRemoteLinkActivated: (l) => pane.viewFile(l)
        // (Its place: here, or a notebook card's output while a full-screen
        // program runs there; see termSlot.)
        parent: pane.termSlot !== null ? pane.termSlot : termHome
        anchors.fill: parent
        focus: true
        onContextMenuRequested: (x, y) => pane.contextMenu(term, x, y)
        // At a prompt the keys go to the editor under it, even after a click
        // up here (Shift+Tab in the editor hands them to the terminal); the
        // pane remembers which, and goes back to it when it gets the keys
        // again (switching tabs or panes).
        onActiveFocusChanged: {
            // (Not while a notebook card has it live: its program reads the keys.)
            if (activeFocus && pane.editorOn && pane.preferEditor && pane.termSlot === null) Qt.callLater(compose.focusEditor)
            // (A notebook over it takes the keys, when the editor doesn't.)
            else if (activeFocus && notebook.visible && pane.termSlot === null && !pane.runningLong) Qt.callLater(() => notebook.forceActiveFocus())
        }
        // (A click anywhere in the terminal gives it the keys; a tap of Ctrl
        // gives them back to the editor.)
        editorBeside: pane.editorOn
        onTerminalClicked: pane.preferEditor = false
    }

    ComposeBox {
        id: compose
        // (Over the notebook and the views on the terminal: its dropdowns,
        // history and completions, open upward over them.)
        z: 22
        visible: pane.editorShown
        // (A command running with the keys: the editor waits, dimmed.)
        waiting: !pane.editorOn
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: implicitHeight
        view: term
        controller: pane.controller
        keys: pane.editorKeys
        history: pane.commandHistory
        historyLangs: pane.historyLangs
        // (A notebook with a Jupyter kernel: the editor writes its next cell.)
        readonly property bool forNotebook: notebook.visible && !notebook.shellKernel
        // (tessh as the pane's shell, not only started from another: its
        // completions, colors and checks, and what's typed goes to it.)
        autoLang: forNotebook ? notebook.language : pane.replLang !== "" ? pane.replLang : pane.shellName === "tessh" ? "tessh" : "bash"
        themeData: { try { return JSON.parse(pane.themeJson) } catch (e) { return {} } }
        cwd: term.pwd !== "" ? term.pwd : pane.procCwd
        showPrompt: pane.editorPrompt
        aiInline: pane.aiSettings.inline !== false
        aiCheck: pane.aiSettings.check !== false
        aiNL: pane.aiSettings.natural_language !== false
        shellName: pane.shellName !== "" ? pane.shellName : "bash"
        shellPid: pane.shellPid
        sshHost: pane.sshHost
        promptEngine: pane.promptEngine
        promptSegments: pane.promptSegments
        // (A shell picked: its kernel starts now, ready by the first Enter.)
        onRunShellChanged: if (runShell !== "" && notebook.hasKernelFor(runShell)) notebook.warmLang(runShell)
        queued: pane.cmdQueue.length
        onClearQueue: pane.cmdQueue = []
        onSubmitted: (t) => {
            // (In the background: a card run as a job, the shell's & for any
            // language; in the terminal it's in Jobs.)
            if (compose.submitMode === "bg" && !compose.commandRun) {
                notebook.submitBackground(t, compose.runShellUsed !== "" ? compose.runShellUsed : compose.runLang)
                // (Said, so it doesn't look like it vanished.)
                const line = t.split("\n")[0]
                Window.window.notice("Running in the background: " + (line.length > 60 ? line.substring(0, 60) + "\u2026" : line) + (t.indexOf("\n") >= 0 ? " \u2026" : ""),
                                     [{ label: "Show in Jobs", run: () => notebook.setJobsView(true) }])
                return
            }
            // (The terminal busy with a command: this one waits for its
            // prompt, then runs as it would have.)
            if (!forNotebook && !compose.commandRun && !pane.flushing && !pane.promptish) {
                pane.cmdQueue = pane.cmdQueue.concat([{ t: t, L: compose.runLang, S: compose.runShellUsed }])
                return
            }
            // (A notebook: its kernel's language, unless another is picked or
            // recognised in the editor: that one's kernel (or its program,
            // once); the pane's own shell picked: the terminal, a card of its own.)
            if (forNotebook) {
                const pickL = compose.runShellUsed !== "" ? compose.runShellUsed : compose.runLang
                if (compose.commandRun || (compose.runShellUsed === "" && compose.langOverride === "bash" && pickL === "bash")) {
                    if (t.indexOf("\n") < 0) term.sendText(t + "\r"); else { term.pasteText(t); term.sendText("\r") }
                    return
                }
                if (pickL !== "" && !notebook.sameLang(pickL, notebook.language)) { notebook.submitLang(t, pickL); return }
                notebook.submit(t)
                return
            }
            // (Code in another language than the shell's: its Jupyter kernel,
            // kept alive in the pane's notebook, if there's one; else run by
            // its interpreter, once.)
            const L = compose.runLang
            // (Another shell picked: its kernel, kept alive in the notebook,
            // else run by it once.)
            const S = compose.runShellUsed
            const cwd = compose.cwd, shell = compose.shellName, host = pane.sshHost
            // (The snippet's command comes back in the background; none: on
            // to the next way of running it.)
            const plain = () => {
                if (t.indexOf("\n") < 0) term.sendText(t + "\r")
                else { term.pasteText(t); term.sendText("\r") }
                term.scrollToBottom()
            }
            const snippet = (lang, otherwise) => Async.call(controller, k => controller.snippetCommand(k, lang, t, cwd, shell, host), cmd => {
                if (!term) return
                if (cmd !== "") { term.pasteText(cmd); term.sendText("\r"); term.scrollToBottom(); return }
                otherwise()
            })
            // (In the terminal, a shell's language — tessh, fish, nu, ... —
            // runs there, by that shell (tessh -c ...), with the terminal for
            // what it starts: claude, vim. Its notebook kernel is for the
            // notebook; going there took `claude` off to a kernel cell.)
            const inTerm = pane.viewMode !== "notebook"
            const shellLang = (l) => ["tessh", "fish", "nu", "powershell", "elvish", "xonsh"].indexOf(l) >= 0
            const asLang = () => {
                if (inTerm && L === pane.shellName) { plain(); return }
                if (inTerm && shellLang(L) && controller) { snippet(L, plain); return }
                if (!inTerm && pane.replLang === "" && L !== "" && L !== "bash" && controller && notebook.hasKernelFor(L)) {
                    notebook.submitLang(t, L)
                    return
                }
                if (pane.replLang === "" && L !== "" && L !== "bash" && controller) { snippet(L, plain); return }
                plain()
            }
            if (S !== "" && controller) {
                if (!inTerm && notebook.hasKernelFor(S)) { notebook.submitLang(t, S); return }
                snippet(S, asLang)
                return
            }
            asLang()
        }
        // (The terminal keeps the keys until the editor is clicked again.)
        onHandBack: { pane.preferEditor = false; term.forceActiveFocus() }
        onHideRequested: pane.setInputHidden(true)
        onKeysPicked: (k) => pane.editorKeysPicked(k)
        onOpenRich: pane.openRich()
        viewMode: pane.viewMode
        jobsRunning: notebook.jobsActive
        jobsAll: notebook.hostJobs.length
        // (The jobs list is the notebook's: in a terminal tab it shows over
        // the terminal while it's open, and the tab stays a terminal.)
        onJobsToggled: notebook.setJobsView(!notebook.jobsView)
        onEditingChanged: if (editing) { pane.preferEditor = true; if (!pane.editorOn) pane.editorCalled = true }
    }

    // Badge: large text behind the content, top right (iTerm2's badges).
    Text {
        visible: pane.badge !== ""
        anchors.top: termHome.top
        anchors.right: termHome.right
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
        anchors.centerIn: termHome
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
        anchors.top: termHome.top
        anchors.bottom: termHome.bottom
        anchors.right: termHome.right
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
    // (In notebook mode, a command that runs on, or a program in a card: the
    // keys go to it, as typed into the terminal; at the prompt again, back to
    // the editor.)
    onRunningLongChanged: if (runningLong && notebook.visible && !notebook.ownRun) term.forceActiveFocus()
    // (Its program done, the keys go back where they'd be: the input box,
    // else the notebook. Moving out of the card left them nowhere.)
    onTermSlotChanged: {
        if (termSlot !== null) { Qt.callLater(() => term.forceActiveFocus()); return }
        Qt.callLater(() => {
            const w = pane.Window.window
            const f = w ? w.activeFocusItem : null
            if (f && f !== term && f !== notebook && f !== w.contentItem) return
            if (pane.editorShown) compose.focusEditor(); else notebook.forceActiveFocus()
        })
    }
    Connections {
        target: term
        function onCellsChanged(id, done) {
            // (Starting a shell switched to, or leaving it: no card, and not
            // a long-running command (the notebook stays).)
            if (!done && pane.shellCmdPending > 0) {
                pane.shellCmdPending--
                pane.shellCmds = pane.shellCmds.concat([id])
                notebook.hideTerm(id)
            }
            if (pane.shellCmds.indexOf(id) >= 0) { longTimer.stop(); pane.runningLong = false; return }
            if (done) { longTimer.stop(); pane.runningLong = false }
            else longTimer.restart()
        }
    }
    // Another host's pane while SSH connects (and tesserad is started, or
    // copied there), or why it couldn't.
    Rectangle {
        z: 20
        anchors.fill: termHome
        visible: !pane.hostUp
        color: Theme.dark ? "#1e1f22" : "#f4f4f5"
        Column {
            anchors.centerIn: parent
            width: Math.min(parent.width - 48, 640)
            spacing: 12
            Text {
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                renderType: Text.NativeRendering
                color: Theme.dark ? "#e6e6e6" : "#222"
                font.pointSize: 11
                text: pane.askInstall !== "" ? "Install tesserad on " + pane.sshHost + "?"
                    : pane.hostError === "" ? "Connecting to " + pane.sshHost + "\u2026" : "Couldn't connect to " + pane.sshHost
            }
            Text {
                width: parent.width
                height: visible ? contentHeight : 0
                visible: pane.askInstall !== ""
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                renderType: Text.NativeRendering
                color: Theme.dark ? "#c8c8c8" : "#444"
                text: pane.sshHost + " (" + pane.askInstall + ") doesn't have tesserad, Tessera's backend. Installed there "
                    + (pane.askInstall.startsWith("Windows") ? "(in %LOCALAPPDATA%\\Tessera\\bin)" : "(in ~/.local/bin)")
                    + ", its shells, notebook kernels and files run there, and keep running when this window closes or the connection drops."
                    + " Without it, this pane is an ordinary ssh session from here."
            }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                visible: pane.askInstall !== ""
                spacing: 10
                Button { text: "Install tesserad"; onClicked: pane.answerInstall(true) }
                Button { text: "Use Plain SSH"; onClicked: pane.answerInstall(false) }
            }
            CheckBox {
                anchors.horizontalCenter: parent.horizontalCenter
                visible: pane.askInstall !== ""
                text: "Remember for " + pane.sshHost
                checked: pane.rememberAnswer
                onToggled: pane.rememberAnswer = checked
            }
            TextEdit {
                width: parent.width
                visible: pane.hostError !== "" && pane.askInstall === ""
                readOnly: true
                selectByMouse: true
                wrapMode: Text.Wrap
                renderType: Text.NativeRendering
                color: Theme.dark ? "#c8c8c8" : "#444"
                text: pane.hostError
            }
            Button {
                anchors.horizontalCenter: parent.horizontalCenter
                visible: pane.hostError !== ""
                text: "Try Again"
                onClicked: pane.connectHost()
            }
        }
    }
    NotebookView {
        id: notebook
        z: 15
        editorKeys: pane.editorKeys
        onEditorKeysPicked: (k) => pane.editorKeysPicked(k)
        shellName: pane.shellName
        anchors.fill: termHome
        // (It stays for a program that runs on: a full-screen one runs in its
        // card (termSlot), and keys go to the program while it runs.)
        shown: pane.viewMode === "notebook" || jobsView
        jobsOnly: pane.viewMode !== "notebook"
        // (A full-screen program with no card of its own, started before the
        // notebook showed: it has the pane, as in the terminal.)
        visible: shown && !(term.fullScreen && notebook.liveIndex < 0)
        enabled: visible
        view: term
        controller: pane.controller
        path: pane.docPath
        storeKey: term.sessionId
        sshHost: pane.sshHost
        keepKernels: pane.closingWithWindow
        shellPid: pane.shellPid
        fontFamily: term.fontFamily
        markdownStyle: pane.markdownStyle
        onMarkdownStylePicked: (m) => pane.markdownStylePicked(m)
        defaultKernel: pane.notebookKernel
        themeData: { try { return JSON.parse(pane.themeJson) } catch (e) { return {} } }
        onPathPicked: (p) => pane.docPath = p
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
            const local = drop.urls.map(u => { let p = u.toString().replace(/^file:\/\/[^/]*/, ""); try { p = decodeURIComponent(p) } catch (e) {} return p })
            drop.accept(Qt.CopyAction)
            // (Another host's shell: the files go there first, into its
            // folder, and their paths there are what's typed.)
            if (pane.sshHost !== "") {
                const dir = term.pwd !== "" ? term.pwd.replace(/^file:\/\/[^/]*/, "") : pane.procCwd
                pane.controller.uploadFiles("drop-" + term.sessionId, pane.sshHost, dir, JSON.stringify(local))
                return
            }
            pane.typePaths(local)
        }
    }
    function typePaths(list) {
        const q = (s) => /^[A-Za-z0-9_\/.~+:@-]+$/.test(s) ? s : "'" + s.replace(/'/g, "'\\''") + "'"
        const paths = list.map(q).join(" ") + " "
        if (pane.editorOn) { compose.focusEditor(); compose.insert(paths) }
        else term.sendText(paths)
    }
    Connections {
        target: pane.controller
        enabled: pane.sshHost !== ""
        function onFilesUploaded(key, paths, error) {
            if (key !== "drop-" + term.sessionId) return
            let list = []
            try { list = JSON.parse(paths) } catch (e) {}
            if (list.length > 0) pane.typePaths(list)
            if (error !== "") term.showMessage("\r\n\u001b[31m[tessera: " + error + "]\u001b[0m\r\n")
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
        anchors.fill: termHome
        fontFamily: term.fontFamily
        themeData: { try { return JSON.parse(pane.themeJson) } catch (e) { return {} } }
        onClosed: { visible = false; if (pane.editorOn) compose.focusEditor(); else term.forceActiveFocus() }
    }
    // A file, shown (View, from a file listing or the terminal's menu).
    function viewFile(p) { preview.path = ""; preview.path = fullPath(p); preview.visible = true; preview.forceActiveFocus() }
    // (A name from the terminal's text — IMG_1.jpeg, ~/a.txt — as the shell
    // means it: from its folder. Tessera's own folder isn't the shell's, and
    // Open with its app handed the bare name on, to be looked for there.)
    function fullPath(p) {
        if (p === "" || p.startsWith("/") || /^[a-z]+:/.test(p)) return p
        if ((p === "~" || p.startsWith("~/")) && controller) return controller.homeDir() + p.substring(1)
        let dir = compose.cwd
        if (dir.startsWith("file://")) { dir = dir.replace(/^file:\/\/[^/]*/, ""); try { dir = decodeURIComponent(dir) } catch (e) {} }
        return dir === "" ? p : dir.replace(/\/+$/, "") + "/" + p.replace(/^\.\//, "")
    }
    FilePreview {
        id: preview
        visible: false
        z: 21
        anchors.fill: termHome
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
        anchors.top: termHome.top
        anchors.right: termHome.right
        anchors.topMargin: 8
        anchors.rightMargin: 14
    }

    // Broadcast Input: the receiving panes get a red outline.
    Rectangle {
        anchors.fill: termHome
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
