import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import QtQuick.Dialogs
import Tessera
import Tessera.Native

// The pane as a notebook (like Jupyter): one card per cell. A command run in
// the pane (typed in the editor, or in the terminal) becomes a cell, its
// output shown for what it is: terminal text with its colors, JSON, a table,
// HTML (a real browser view), Markdown, SVG, a picture. The kind is
// recognised (rules, then the AI for the rest) and can be changed per cell.
//
// It's also a Jupyter notebook: cells can be written and run here, with this
// pane's shell or any Jupyter kernel (Python from Nix, an installed one), and
// Markdown cells sit between them. It opens and saves .ipynb files; one that
// isn't a file yet is kept with the pane's session.
//
// Keys, while editing a cell (as in the pane's editor): Enter runs it and goes
// on, a tap of right Shift (or Shift+Enter) is a new line, Ctrl+Enter runs
// it, Alt+Enter runs it and adds one below, Tab completes (from the kernel),
// Esc leaves the cell. Between cells: Up/Down (or k/j) move, Enter edits, A/B
// add above/below, D D deletes (Z brings it back), M/Y make it Markdown/code,
// O hides its output, Ctrl+S saves.
FocusScope {
    id: nb
    property var view: null             // the pane's TerminalView (its shell runs cells)
    property var controller: null
    property var themeData: ({})
    property string fontFamily: "monospace"
    property string path: ""            // the .ipynb, or "" for the pane's own notebook
    property string storeKey: ""        // the session an unsaved notebook is kept with
    property string markdownStyle: "rich"  // Markdown by default: "rich" or "text"
    signal markdownStylePicked(string style)
    signal pathPicked(string path)      // saved as a file
    signal terminalRequested()          // "show me the terminal"
    signal commandRequested(string command)   // run in the pane's shell, as if typed
    signal insertRequested(string text)       // into the pane's editor
    signal viewRequested(string path)         // a file, shown over the pane

    readonly property string key: "nb-" + Math.random().toString(36).slice(2)
    readonly property string fileName: path.replace(/^.*\//, "")
    readonly property string folder: path.indexOf("/") >= 0 ? path.replace(/\/[^/]*$/, "") : (view ? view.pwd : "")
    property bool dirty: false
    property int shellPid: 0            // the pane's shell (kernels start with its environment)
    property string loadError: ""
    property string saveError: ""
    property var metadata: ({})
    property bool loaded: false

    // ---- kernel -------------------------------------------------------------------
    property var kernels: []            // [{name, display, language}]
    property string kernelName: "tessera-shell"
    property string kernelState: ""     // "", starting, idle, busy, dead
    property string kernelMessage: ""
    property string language: "bash"
    property string languageVersion: ""
    readonly property var kernel: kernels.find(k => k.name === kernelName) || null
    readonly property bool shellKernel: kernelName === "tessera-shell"
    readonly property bool kernelUp: shellKernel || kernelState === "idle" || kernelState === "busy"
    readonly property string codeLang: shellKernel ? "bash" : language
    property var queue: []              // cells waiting for the kernel to come up

    function listKernels() {
        let ks = []
        if (view) ks.push({ name: "tessera-shell", display: "This pane's shell", language: "bash" })
        try { ks = ks.concat(JSON.parse(controller.jupyterKernels())) } catch (e) {}
        kernels = ks
    }
    function pickKernel(spec) {
        // The notebook's own kernel, else one for its language; a shell
        // notebook (or none said) runs in the pane's shell.
        const want = spec && spec.name ? spec.name : ""
        const lang = (spec && spec.language) || (metadata.language_info && metadata.language_info.name) || ""
        if (want === "" && lang === "" || want === "bash" || lang === "bash" || want === "tessera-shell") {
            kernelName = view ? "tessera-shell" : (kernels.length ? kernels[0].name : "")
            language = "bash"
            return
        }
        const k = kernels.find(x => x.name === want)
            || kernels.find(x => x.language === lang && x.name === "python3")
            || kernels.find(x => x.language === lang && x.name !== "tessera-nix-python")
            || kernels.find(x => x.language === lang)
        kernelName = k ? k.name : (kernels.length ? kernels[0].name : "")
        language = k ? k.language : lang
    }
    function startKernel() {
        if (!controller || kernelName === "" || shellKernel) return
        kernelState = "starting"
        kernelMessage = ""
        controller.jupyterStart(key, kernelName, folder, shellPid)
    }
    function stopRunning() {
        for (let i = 0; i < cells.count; ++i) if (cells.get(i).running && cells.get(i).tid <= 0) cells.setProperty(i, "running", false)
        queue = []
    }
    function restartKernel() {
        if (shellKernel) return
        controller.jupyterStop(key)
        stopRunning()
        kernelState = ""
        startKernel()
    }
    function changeKernel(name) {
        if (name === kernelName) return
        controller.jupyterStop(key)
        stopRunning()
        kernelState = ""
        kernelName = name
        language = kernel ? kernel.language : language
        touch()
    }
    Connections {
        target: nb.controller
        function onJupyterEvent(key, json) {
            if (key.startsWith(nb.key + "~")) {
                let le
                try { le = JSON.parse(json) } catch (x) { return }
                nb.langEvent(key.substring(nb.key.length + 1), le)
                return
            }
            if (key !== nb.key) return
            let e
            try { e = JSON.parse(json) } catch (x) { return }
            if (e.type !== "complete") nb.kernelEvent(e)
        }
    }
    function indexOf(cid) {
        for (let i = 0; i < cells.count; ++i) if (cells.get(i).cid === cid) return i
        return -1
    }
    // Kernels asking for input (input(), readline): cid -> {prompt, password, key}.
    property var asking: ({})
    function setAsking(cid, a) { const m = Object.assign({}, asking); if (a) m[cid] = a; else delete m[cid]; asking = m }
    function answer(cid, text) {
        const a = asking[cid]
        if (!a) return
        controller.jupyterInput(a.key, text)
        // (What was asked and answered stays in the output, as Jupyter shows it.)
        const i = indexOf(cid)
        if (i >= 0) addOutput(i, { output_type: "stream", name: "stdout", text: (a.prompt || "") + (a.password ? "\u2022\u2022\u2022\u2022" : text) + "\n" })
        setAsking(cid, null)
    }
    function kernelEvent(e, fromKey) {
        if (e.type === "input") { setAsking(e.cell, { prompt: e.prompt || "", password: !!e.password, key: fromKey || key }); return }
        if (e.type === "status") {
            kernelState = e.state
            if (e.message !== undefined) kernelMessage = e.message
            if (e.state === "idle" && queue.length > 0) { const q = queue; queue = []; for (const c of q) execute(c) }
            if (e.state === "dead") stopRunning()
        } else if (e.type === "info") {
            if (e.language) language = e.language
            languageVersion = e.version || ""
        } else {
            const i = indexOf(e.cell)
            if (i < 0) return
            if (e.type === "count") cells.setProperty(i, "count", e.count)
            else if (e.type === "clear") {
                if (e.wait) cells.setProperty(i, "clearPending", true)
                else cells.setProperty(i, "outputs", "[]")
            } else if (e.type === "output") addOutput(i, e.output)
            else if (e.type === "done") {
                if (asking[e.cell]) setAsking(e.cell, null)
                cells.setProperty(i, "running", false)
                if (e.count !== null && e.count !== undefined) cells.setProperty(i, "count", e.count)
                cells.setProperty(i, "ended", Date.now())
                cells.setProperty(i, "failed", e.status !== "ok")
                // (Stopped at an error: the cells waiting after it don't run.)
                if (e.status === "error") for (let j = 0; j < cells.count; ++j) if (cells.get(j).running && cells.get(j).tid <= 0 && cells.get(j).started > cells.get(i).started) cells.setProperty(j, "running", false)
                touch()
            }
        }
    }
    function addOutput(i, out) {
        let outs = []
        try { outs = JSON.parse(cells.get(i).outputs) } catch (x) {}
        if (cells.get(i).clearPending) { outs = []; cells.setProperty(i, "clearPending", false) }
        // (Stream text goes on the last stream output of the same name.)
        const last = outs[outs.length - 1]
        if (out.output_type === "stream" && last && last.output_type === "stream" && last.name === out.name) last.text += out.text
        else outs.push(out)
        cells.setProperty(i, "outputs", JSON.stringify(outs))
        touch()
    }

    // ---- the cells ----------------------------------------------------------------
    // tid: the terminal's cell (a command the shell ran), its output read from
    // the terminal; otherwise the outputs are Jupyter's (JSON).
    ListModel { id: cells }
    property int current: 0
    property int nextId: 1
    property var kinds: ({})            // cid -> chosen view (the dropdown)
    property var fileViews: ({})        // cid -> an ls card's own view, sort, ... (FileBrowser)
    property var aiKinds: ({})          // cid -> what the AI made of it
    property var heights: ({})          // cid -> output height the user dragged to
    property string expandedId: ""      // the cell filling the window, if any
    property int execCount: 0
    function setHeight(cid, h) { const m = Object.assign({}, heights); if (h > 0) m[cid] = h; else delete m[cid]; heights = m; touch() }
    function newCell(type, source) {
        return { cid: "c" + (nextId++) + "-" + Math.random().toString(36).slice(2, 8), nbid: Math.random().toString(36).slice(2, 10),
                 type: type, source: source || "", tid: 0, mine: false, outputs: "[]", count: -1, running: false, failed: false,
                 started: 0, ended: 0, cwd: "", status: 0, tkind: "", tsize: 0,
                 editing: false, meta: "{}", clearPending: false, hideOutput: false, lang: "" }
    }
    function insertCell(at, type, edit) {
        at = Math.max(0, Math.min(cells.count, at))
        const c = newCell(type, "")
        c.editing = true
        cells.insert(at, c)
        current = at
        touch()
        if (edit !== false) Qt.callLater(focusCell, at, true)
    }
    function deleteCell(i) {
        if (i < 0 || i >= cells.count) return
        const c = cells.get(i)
        undoCell = { at: i, cell: JSON.parse(JSON.stringify(c)) }
        if (c.tid > 0) retire(c.tid)
        cells.remove(i)
        current = Math.max(0, Math.min(i, cells.count - 1))
        touch()
        Qt.callLater(focusCell, current, false)
    }
    property var undoCell: null
    function undoDelete() {
        if (!undoCell) return
        cells.insert(Math.min(undoCell.at, cells.count), undoCell.cell)
        if (undoCell.cell.tid > 0) { retired = retired.filter(t => t !== undoCell.cell.tid); link(undoCell.cell.tid, undoCell.cell.cid) }
        current = undoCell.at
        undoCell = null
        touch()
    }
    function moveCell(i, d) {
        const j = i + d
        if (j < 0 || j >= cells.count) return
        cells.move(i, j, 1)
        current = j
        touch()
    }
    function setType(i, type) {
        if (cells.get(i).type === type) return
        if (cells.get(i).tid > 0) retire(cells.get(i).tid)
        cells.setProperty(i, "type", type)
        cells.setProperty(i, "tid", 0)
        cells.setProperty(i, "outputs", "[]")
        cells.setProperty(i, "count", -1)
        cells.setProperty(i, "ended", 0)
        cells.setProperty(i, "editing", true)
        touch()
    }
    // A new card from a command: the view goes to the end and stays there as
    // the card is laid out and its output grows, until it's scrolled up.
    property bool sticking: false
    property bool wasAtEnd: true        // (shown again: back to the end, if it was there)
    // (At once, before the frame is drawn, so a tab switched to shows its
    // end straight away; and again once it's laid out.)
    onVisibleChanged: if (visible && wasAtEnd) { sticking = true; pin(); Qt.callLater(pin) }
    property bool pinning: false
    function follow() { sticking = true; Qt.callLater(pin) }
    function pin() {
        if (!sticking) return
        pinning = true
        list.positionViewAtEnd()
        pinning = false
    }
    function focusCell(i, edit) {
        if (i < 0 || i >= cells.count) return
        current = i
        sticking = false
        list.positionViewAtIndex(i, ListView.Contain)
        if (edit) {
            cells.setProperty(i, "editing", true)
            Qt.callLater(() => { const x = list.itemAtIndex(i); if (x) x.focusEditor() })
        } else {
            list.forceActiveFocus()
        }
    }
    function stopEditing(i) {
        const c = cells.get(i)
        if (!c) return
        // (Code that was run, or Markdown with something in it, goes back to its card.)
        if (c.type === "markdown" ? c.source.trim() !== "" : c.ended > 0 || c.tid > 0 || c.running) cells.setProperty(i, "editing", false)
        list.forceActiveFocus()
    }

    // ---- running ------------------------------------------------------------------
    function run(i) {
        const c = cells.get(i)
        if (!c) return
        if (c.type === "markdown") { if (c.source.trim() !== "") cells.setProperty(i, "editing", false); return }
        if (c.type !== "code" || c.source.trim() === "") return
        if (c.tid > 0) retire(c.tid)
        cells.setProperty(i, "tid", 0)
        cells.setProperty(i, "outputs", "[]")
        cells.setProperty(i, "running", true)
        cells.setProperty(i, "failed", false)
        cells.setProperty(i, "status", 0)
        cells.setProperty(i, "started", Date.now())
        cells.setProperty(i, "ended", 0)
        cells.setProperty(i, "mine", true)
        cells.setProperty(i, "editing", false)
        touch()
        // (A cell in another language: that language's kernel.)
        if (c.lang !== "" && c.lang !== codeLang) { runLang(c.cid, c.lang); return }
        if (shellKernel) { shellRun(c.cid, c.source); return }
        if (kernelUp) execute(c.cid)
        else {
            queue = queue.concat([c.cid])
            if (kernelState === "" || kernelState === "dead") startKernel()
        }
    }
    // The shell's folder now (kernels follow it, and its environment).
    readonly property string shellDir: { const p = view ? view.pwd : ""; let x = (p || "").replace(/^file:\/\/[^/]*/, ""); try { x = decodeURIComponent(x) } catch (e) {} return x !== "" ? x : folder }
    function execute(cid) {
        const i = indexOf(cid)
        if (i < 0) return
        if (path === "") controller.jupyterSync(key, shellPid, shellDir)
        if (!controller.jupyterExecute(key, cid, cells.get(i).source)) {
            cells.setProperty(i, "running", false)
            addOutput(i, { output_type: "error", ename: "Not sent", evalue: "The kernel isn't reachable.", traceback: ["The kernel isn't reachable: restart it."] })
        }
    }
    function runAll() { for (let i = 0; i < cells.count; ++i) if (cells.get(i).type === "code") run(i) }
    function runAbove(n) { for (let i = 0; i < n; ++i) if (cells.get(i).type === "code") run(i) }
    function interrupt() {
        if (shellKernel) { if (view) view.sendText("\x03"); return }
        controller.jupyterInterrupt(key)
    }
    function clearOutputs() {
        for (let i = 0; i < cells.count; ++i) {
            const c = cells.get(i)
            if (c.running) continue
            if (c.tid > 0) retire(c.tid)
            cells.setProperty(i, "tid", 0)
            cells.setProperty(i, "outputs", "[]")
            cells.setProperty(i, "count", -1)
            cells.setProperty(i, "ended", 0)
        }
        touch()
    }
    // ---- the kernels running, every notebook's (the bar's Kernels list) ----
    property var runningKernels: []
    function refreshKernels() { if (controller) try { runningKernels = JSON.parse(controller.jupyterRunning()) } catch (e) { runningKernels = [] } }
    Timer { interval: kernelsPop.opened ? 1500 : 4000; running: nb.visible && nb.controller !== null; repeat: true; triggeredOnStart: true; onTriggered: nb.refreshKernels() }
    // (Test hook: TESSERA_TEST_NB_KERNELS=<ms>: the list opened after that long.)
    Timer { interval: nb.controller ? parseInt(nb.controller.testEnv("NB_KERNELS") || "0") : 0; running: interval > 0 && nb.visible; onTriggered: { nb.refreshKernels(); kernelsPop.open() } }
    function stopKernel(k) {
        controller.jupyterStop(k.key)
        // (This notebook's own: forgotten, so the next run starts it again.)
        if (k.key === key) { kernelState = ""; stopRunning() }
        else if (k.key.startsWith(key + "~")) { const m = Object.assign({}, langKernels); delete m[k.key.substring(key.length + 1)]; langKernels = m }
        Qt.callLater(refreshKernels)
    }
    function restartKernelOf(k) {
        if (k.key === key) { restartKernel(); return }
        if (k.key.startsWith(key + "~")) {
            const l = k.key.substring(key.length + 1)
            stopKernel(k)
            setLang(l, { name: k.name, display: k.display, state: "starting", message: "", queue: [] })
            controller.jupyterStart(langKey(l), k.name, shellDir, shellPid)
            return
        }
        stopKernel(k)   // (another notebook's: it starts again when that one runs)
    }
    Popup {
        id: kernelsPop
        parent: nb
        x: Math.max(8, nb.width - width - 8)
        y: 36
        width: Math.min(nb.width - 16, 560)
        padding: 8
        topPadding: 8.001   // (off the half pixel: 130% scale smears native text there)
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        contentItem: Column {
            spacing: 4
            Row {
                width: parent.width
                Text { text: "Kernels running"; color: Theme.text; font.pixelSize: 13; font.weight: Font.DemiBold; width: parent.width - stopAll.width }
                Chip { id: stopAll; base: Theme.panel; label: "Stop all"; onClicked: { for (const k of nb.runningKernels) nb.stopKernel(k) } }
            }
            Text { visible: nb.runningKernels.length === 0; text: "None."; color: Theme.textDim; font.pixelSize: 12 }
            Repeater {
                model: nb.runningKernels
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool mine: modelData.key === nb.key || modelData.key.startsWith(nb.key + "~")
                    width: parent.width
                    height: 30
                    radius: 5
                    color: kh.hovered ? Theme.tabHover : "transparent"
                    HoverHandler { id: kh }
                    Rectangle {
                        x: 6
                        anchors.verticalCenter: parent.verticalCenter
                        width: 8; height: 8; radius: 4
                        color: modelData.state === "busy" ? nb.ansi(3, "#e5c07b") : modelData.state === "starting" ? nb.ansi(4, "#61afef") : nb.ansi(2, "#98c379")
                    }
                    Column {
                        x: 22
                        anchors.verticalCenter: parent.verticalCenter
                        width: parent.width - x - kbtns.width - 8
                        Text { width: parent.width; elide: Text.ElideRight; text: modelData.display + (parent.parent.mine ? "" : "  (another notebook)"); color: Theme.text; font.pixelSize: 12 }
                        Text {
                            text: modelData.state + " \u00b7 " + (modelData.rss_kb > 0 ? (modelData.rss_kb / 1024).toFixed(0) + " MB" : "") + " \u00b7 "
                                  + (modelData.seconds < 3600 ? Math.round(modelData.seconds / 60) + " min" : (modelData.seconds / 3600).toFixed(1) + " h")
                            color: Theme.textDim
                            font.pixelSize: 10
                        }
                    }
                    Row {
                        id: kbtns
                        anchors.right: parent.right
                        anchors.rightMargin: 4
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 4
                        Chip { base: Theme.panel; label: "Restart"; tip: "Its variables are lost"; onClicked: nb.restartKernelOf(modelData) }
                        Chip { base: Theme.panel; label: "Stop"; tip: "It starts again when that language runs"; onClicked: nb.stopKernel(modelData) }
                    }
                }
            }
        }
    }

    // ---- a kernel per language (cells in another language than the notebook's) ----
    property var langKernels: ({})      // lang -> {name, display, state, message, queue: [cid]}
    function langKey(l) { return key + "~" + l }
    // (Names the editor and kernels give a language.)
    function sameLang(a, b) {
        const n = (x) => { x = (x || "").toLowerCase(); return ({ "c++": "cpp", "cxx": "cpp", "js": "javascript", "ts": "typescript", "sh": "bash", "shell": "bash", "pwsh": "powershell", "r": "r" })[x] || x }
        const x = n(a), y = n(b)
        return x === y || (x === "javascript" && y === "typescript") || (x === "typescript" && y === "javascript")
    }
    // The kernel for a language: one installed, else one Tessera fetches.
    function langSpec(l) {
        const ks = kernels.filter(k => k.name !== "tessera-shell" && sameLang(k.language, l))
        return ks.find(k => !k.name.startsWith("tessera-")) || ks.find(k => k.name === "tessera-" + l) || ks[0] || null
    }
    function hasKernelFor(l) { if (kernels.length === 0) listKernels(); return l !== "" && langSpec(l) !== null }
    function setLang(l, patch) { const m = Object.assign({}, langKernels); m[l] = Object.assign({}, m[l] || {}, patch); langKernels = m }
    function runLang(cid, l) {
        if (kernels.length === 0) listKernels()
        const e = langKernels[l]
        if (e && (e.state === "idle" || e.state === "busy")) { execLang(cid, l); return }
        setLang(l, { queue: ((e && e.queue) || []).concat([cid]) })
        if (e && e.state === "starting") return
        const spec = langSpec(l)
        if (!spec) {
            const i = indexOf(cid)
            cells.setProperty(i, "running", false)
            addOutput(i, { output_type: "error", ename: "No kernel", evalue: "No Jupyter kernel for " + l, traceback: ["No Jupyter kernel for " + l + " (installed or from Nix)."] })
            setLang(l, { queue: [] })
            return
        }
        setLang(l, { name: spec.name, display: spec.display, state: "starting", message: "" })
        controller.jupyterStart(langKey(l), spec.name, shellDir, shellPid)
    }
    function execLang(cid, l) {
        const i = indexOf(cid)
        if (i < 0) return
        controller.jupyterSync(langKey(l), shellPid, shellDir)
        if (!controller.jupyterExecute(langKey(l), cid, cells.get(i).source)) {
            cells.setProperty(i, "running", false)
            addOutput(i, { output_type: "error", ename: "Not sent", evalue: "The kernel isn't reachable.", traceback: ["The " + l + " kernel isn't reachable: run it again to restart it."] })
            setLang(l, { state: "dead" })
        }
    }
    function langEvent(l, e) {
        if (e.type === "status") {
            const patch = { state: e.state }
            if (e.message !== undefined) patch.message = e.message
            setLang(l, patch)
            const q = (langKernels[l] && langKernels[l].queue) || []
            if (e.state === "idle" && q.length > 0) { setLang(l, { queue: [] }); for (const c of q) execLang(c, l) }
            if (e.state === "dead") {
                for (const c of q.concat()) {
                    const i = indexOf(c)
                    if (i < 0) continue
                    cells.setProperty(i, "running", false)
                    addOutput(i, { output_type: "error", ename: "Kernel stopped", evalue: e.message || "", traceback: ["The " + l + " kernel stopped" + (e.message ? ": " + e.message : ".")] })
                }
                for (let i = 0; i < cells.count; ++i) if (cells.get(i).running && cells.get(i).lang === l && cells.get(i).tid <= 0) cells.setProperty(i, "running", false)
                setLang(l, { queue: [] })
            }
        } else if (e.type !== "info" && e.type !== "complete") kernelEvent(e, langKey(l))
    }
    function stopLangKernels() { for (const l in langKernels) controller.jupyterStop(langKey(l)); langKernels = ({}) }
    // Code in another language from the pane's editor: a cell in that
    // language at the end, run in its kernel.
    function submitLang(code, l) {
        const c = newCell("code", code)
        c.lang = sameLang(l, codeLang) ? "" : l
        cells.append(c)
        current = cells.count - 1
        run(cells.count - 1)
        follow()
    }
    // Code from the pane's editor, for a kernel that isn't the shell: a new
    // cell at the end, run.
    function submit(code) {
        const c = newCell("code", code)
        cells.append(c)
        current = cells.count - 1
        run(cells.count - 1)
        follow()
    }
    property bool ownRun: false         // a cell run from here is running in the shell
    function updateOwnRun() {
        let on = false
        for (let i = 0; i < cells.count && !on; ++i) { const c = cells.get(i); on = c.running && c.mine && (c.tid > 0 || shellWaiting.indexOf(c.cid) >= 0) }
        ownRun = on
    }

    // ---- the shell: cells the terminal made ----------------------------------------
    // Each command the shell runs (shell integration's marks) is a terminal
    // cell. One sent from here fills the cell that sent it; any other (typed
    // in the editor or the terminal) is added at the end.
    property var shellWaiting: []       // our cells sent to the shell, in order
    property var linked: ({})           // terminal cell id -> our cell
    property var retired: []            // terminal cells not shown (deleted, run again)
    property int fileStart: 0            // a file's notebook: the terminal's cells before it opened aren't its
    function link(tid, cid) { const m = Object.assign({}, linked); m[tid] = cid; linked = m }
    function retire(tid) {
        if (retired.indexOf(tid) < 0) retired = retired.concat([tid])
        const m = Object.assign({}, linked); delete m[tid]; linked = m
    }
    function shellRun(cid, code) {
        if (!view) return
        shellWaiting = shellWaiting.concat([cid])
        updateOwnRun()
        if (code.indexOf("\n") < 0) view.sendText(code + "\r")
        else { view.pasteText(code); view.sendText("\r") }
    }
    function termCells() { try { return JSON.parse(view.cells()) } catch (e) { return [] } }
    function fromTerm(i, t) {
        const c = cells.get(i)
        if (c.source === "" && t.command !== "") cells.setProperty(i, "source", t.command.replace(/\n+$/, ""))
        cells.setProperty(i, "cwd", t.cwd)
        cells.setProperty(i, "started", t.started)
        cells.setProperty(i, "running", !t.done)
        cells.setProperty(i, "status", t.status)
        cells.setProperty(i, "failed", t.done && t.status !== 0)
        cells.setProperty(i, "ended", t.done ? t.started + t.duration : 0)
        cells.setProperty(i, "tkind", t.kind)
        cells.setProperty(i, "tsize", t.size)
        if (t.done && c.mine && c.count < 0) cells.setProperty(i, "count", ++execCount)
    }
    function syncTerm() {
        if (!view || !loaded) return
        const ts = termCells()
        const byId = {}
        for (const t of ts) byId[t.id] = t
        // (Cells the terminal no longer keeps go, unless they're a file's.)
        const oldest = ts.length ? ts[0].id : 0
        for (let i = cells.count - 1; i >= 0; --i) {
            const c = cells.get(i)
            if (c.tid <= 0) continue
            if (byId[c.tid]) fromTerm(i, byId[c.tid])
            else if (path === "" && oldest > 0 && c.tid < oldest) cells.remove(i)
        }
        let added = false
        for (const t of ts) {
            if (linked[t.id] !== undefined || retired.indexOf(t.id) >= 0 || t.id <= fileStart) continue
            let at = -1
            if (shellWaiting.length > 0) {
                at = indexOf(shellWaiting[0])
                shellWaiting = shellWaiting.slice(1)
            }
            if (at < 0) {
                const c = newCell("code", "")
                cells.append(c)
                at = cells.count - 1
                added = true
            }
            cells.setProperty(at, "tid", t.id)
            link(t.id, cells.get(at).cid)
            fromTerm(at, t)
            touch()
        }
        updateOwnRun()
        if (added) follow()
    }
    Connections {
        target: nb.view
        function onCellsChanged(id, done) {
            // (0: the terminal's kept cells came back after a restart.)
            if (id === 0 && nb.path === "" && !nb.storeFound) nb.loadStore()
            if (id === 0) nb.termRev++
            nb.syncTerm()
            if (done && id > 0) nb.classify(id)
        }
    }
    // While a command runs, its cell's output is read again now and then.
    property int tick: 0
    property int termRev: 0             // the terminal's cells came back: read outputs again
    Timer { interval: 400; repeat: true; running: nb.visible && nb.ownRunOrAny; onTriggered: nb.tick++ }
    property bool ownRunOrAny: false
    Timer { interval: 300; repeat: true; running: nb.visible; onTriggered: { let r = false; for (let i = 0; i < cells.count && !r; ++i) r = cells.get(i).running && cells.get(i).tid > 0; nb.ownRunOrAny = r } }

    // The rules found nothing: ask the AI (the suggestions model) what it is.
    readonly property string aiKey: "nbk-" + Math.random().toString(36).slice(2)
    function classify(tid) {
        const cid = linked[tid]
        const i = cid !== undefined ? indexOf(cid) : -1
        if (i < 0) return
        const c = cells.get(i)
        if (c.tkind !== "" || c.tsize < 80 || !controller || !controller.aiReady("inline")) return
        const sample = view.cellOutput(tid, "text").slice(0, 2000)
        controller.aiRun(aiKey + "-" + cid, "classify", JSON.stringify([{ role: "user", content: sample }]), JSON.stringify({ cwd: c.cwd }))
    }
    Connections {
        target: nb.controller
        function onAiDone(key, reply, error) {
            if (!key.startsWith(nb.aiKey + "-") || error) return
            const cid = key.slice(nb.aiKey.length + 1)
            const k = reply.trim().toLowerCase().replace(/[^a-z]/g, "")
            if (["json", "csv", "tsv", "html", "markdown", "svg", "diff", "yaml", "xml"].indexOf(k) < 0) return
            // (A guess of JSON has to read as JSON, or JSON lines.)
            if (k === "json") {
                const i = nb.indexOf(cid)
                const t = i >= 0 && nb.view ? nb.view.cellOutput(cells.get(i).tid, "text").trim() : ""
                const parses = (x) => { try { JSON.parse(x); return true } catch (e) { return false } }
                if (!parses(t) && !t.split("\n").every(l => l.trim() === "" || parses(l))) return
            }
            const a = Object.assign({}, nb.aiKinds); a[cid] = k; nb.aiKinds = a
        }
    }
    readonly property var viewNames: [["auto", "Auto"], ["terminal", "Terminal"], ["text", "Plain text"], ["json", "JSON"], ["csv", "Table (CSV)"], ["tsv", "Table (TSV)"],
                                      ["html", "HTML"], ["markdown", "Markdown (rich)"], ["mdtext", "Markdown (text)"], ["svg", "SVG"], ["diff", "Diff"], ["image", "Image"],
                                      ["files", "Files"], ["env", "Variables"]]
    // ls (lsd, eza, ...) and env: Tessera's own views, to click and edit.
    function builtinKind(command) {
        const c = (command || "").trim()
        if (c === "" || /[|;&<>`$(]/.test(c)) return ""
        const w = c.split(/\s+/)
        let first = w[0]
        try { const al = JSON.parse(view ? view.aliases : "{}"); if (al[first]) first = al[first].trim().split(/\s+/)[0] } catch (e) {}
        if (["ls", "ll", "la", "l", "lsd", "eza", "exa", "dir"].indexOf(first) >= 0 && w.slice(1).filter(a => !a.startsWith("-")).length <= 1) return "files"
        // (cd, then the folder it went to.)
        if (["cd", "pushd", "z", "zi"].indexOf(w[0]) >= 0 && w.length <= 2) return "files"
        if ((first === "env" || first === "printenv") && w.length === 1) return "env"
        if (c === "export" || c === "export -p" || c === "declare -x" || c === "typeset -x") return "env"
        return ""
    }
    function shownKind(cid, tkind, command) {
        const chosen = kinds[cid]
        if (chosen && chosen !== "auto") return chosen
        const b = builtinKind(command)
        if (b !== "") return b
        const k = tkind !== "" ? tkind : aiKinds[cid] || "terminal"
        return k === "markdown" && markdownStyle === "text" ? "mdtext" : k
    }

    // ---- reading and writing .ipynb -------------------------------------------------
    function joined(v) { return Array.isArray(v) ? v.join("") : (v === undefined || v === null ? "" : String(v)) }
    function lines(s) {
        // Jupyter's way: a list of lines, each keeping its newline.
        const out = []
        let at = 0
        while (at < s.length) {
            const n = s.indexOf("\n", at)
            if (n < 0) { out.push(s.slice(at)); break }
            out.push(s.slice(at, n + 1))
            at = n + 1
        }
        return out
    }
    function normalOutput(o) {
        const r = Object.assign({}, o)
        if (r.text !== undefined) r.text = joined(r.text)
        if (r.data) {
            const d = {}
            for (const k in r.data) d[k] = typeof r.data[k] === "string" || Array.isArray(r.data[k]) ? (k.startsWith("text/") || k === "image/svg+xml" ? joined(r.data[k]) : joined(r.data[k]).replace(/\n/g, "")) : r.data[k]
            r.data = d
        }
        return r
    }
    // Cells from a notebook's JSON (a file, or the pane's kept one).
    function fill(nbj, keepTids) {
        cells.clear()
        metadata = nbj.metadata || {}
        const t = metadata.tessera || {}
        let maxCount = 0
        for (const c of nbj.cells || []) {
            const x = newCell(c.cell_type === "markdown" ? "markdown" : c.cell_type === "raw" ? "raw" : "code", joined(c.source))
            if (c.id) x.nbid = c.id
            const m = Object.assign({}, c.metadata || {})
            const tm = m.tessera || {}
            delete m.tessera
            x.meta = JSON.stringify(m)
            if (keepTids && tm.tid > 0) { x.tid = tm.tid; x.mine = !!tm.mine; if (tm.cid) x.cid = tm.cid }
            if (tm.cwd) x.cwd = tm.cwd
            if (tm.lang) x.lang = tm.lang
            if (x.type === "code") {
                x.outputs = JSON.stringify((c.outputs || []).map(normalOutput))
                x.count = c.execution_count === null || c.execution_count === undefined ? -1 : c.execution_count
                maxCount = Math.max(maxCount, x.count)
                x.hideOutput = !!(c.metadata && c.metadata.jupyter && c.metadata.jupyter.outputs_hidden)
                if (tm.status !== undefined) { x.status = tm.status; x.failed = tm.status !== 0 }
                if (tm.duration_ms !== undefined) { x.started = tm.started || 1; x.ended = x.started + tm.duration_ms }
                x.editing = x.source === "" && x.tid <= 0
            } else {
                x.editing = x.source.trim() === ""
                if (c.attachments) x.meta = JSON.stringify(Object.assign({}, m, { "tessera.attachments": c.attachments }))
            }
            cells.append(x)
        }
        execCount = maxCount
        const l = {}
        for (let i = 0; i < cells.count; ++i) if (cells.get(i).tid > 0) l[cells.get(i).tid] = cells.get(i).cid
        linked = l
        retired = keepTids ? (t.retired || []) : []
        kinds = t.kinds || {}
        fileViews = t.fileViews || {}
        heights = t.heights || {}
        pickKernel(metadata.kernelspec)
    }
    function load() {
        loaded = false
        loadError = ""
        saveError = ""
        controller.jupyterStop(key)
        kernelState = ""
        shellWaiting = []
        listKernels()
        if (path === "") {
            // The pane's own notebook: what was kept with its session, then the
            // terminal's cells.
            cells.clear(); linked = ({}); retired = []; kinds = ({}); fileViews = ({}); heights = ({}); metadata = ({})
            fileStart = 0
            pickKernel(null)
            loadStore()
        } else {
            const r = JSON.parse(controller.readTextFile(path))
            cells.clear(); metadata = ({})
            if (r.error !== undefined) {
                // A new notebook (nothing there yet), or one that can't be read.
                if (!/No such file/i.test(r.error)) loadError = r.error
                pickKernel(null)
                cells.append(Object.assign(newCell("code", ""), { editing: true }))
            } else {
                try { fill(JSON.parse(r.text), false) }
                catch (e) { loadError = "Not a notebook: " + e; pickKernel(null) }
                if (cells.count === 0) cells.append(Object.assign(newCell("code", ""), { editing: true }))
            }
            // (The terminal's cells from before it opened aren't the file's.)
            const ts = view ? termCells() : []
            fileStart = ts.length ? ts[ts.length - 1].id : 0
        }
        dirty = false
        current = 0
        loaded = true
        syncTerm()
        // (A notebook opens at its end, the latest cells, and stays there
        // while its cards lay out.)
        follow()
    }
    // ---- the pane's own notebook, kept with its session ----
    property bool storeFound: false
    function storePath() { return controller && storeKey !== "" ? controller.stateFolder() + "/notebooks/" + storeKey + ".ipynb" : "" }
    function loadStore() {
        const p = storePath()
        if (p === "") return
        const r = JSON.parse(controller.readTextFile(p))
        if (r.error !== undefined) return
        try { fill(JSON.parse(r.text), true); storeFound = true } catch (e) {}
        dirty = false
    }
    onStoreKeyChanged: if (loaded && path === "" && !storeFound) { loadStore(); syncTerm() }
    function touch() {
        dirty = true
        if (path === "" && loaded) storeTimer.restart()
    }
    Timer { id: storeTimer; interval: 1500; onTriggered: nb.keep() }
    function keep() {
        const p = storePath()
        if (p === "" || path !== "") return
        controller.writeTextFile(p, serialize(true))
        storeFound = true
        dirty = false
    }
    // Keys sorted, as Jupyter writes them (smaller diffs).
    function sorted(v) {
        if (Array.isArray(v)) return v.map(sorted)
        if (v && typeof v === "object") { const o = {}; for (const k of Object.keys(v).sort()) o[k] = sorted(v[k]); return o }
        return v
    }
    // The terminal's output of a cell, as Jupyter outputs.
    function termOutputs(tid) {
        const outs = []
        if (!view) return outs
        const text = view.cellOutput(tid, "text")
        if (text.trim() !== "") outs.push({ name: "stdout", output_type: "stream", text: text.endsWith("\n") ? text : text + "\n" })
        const img = view.cellOutput(tid, "image")
        if (img !== "") outs.push({ data: { "image/png": img }, metadata: {}, output_type: "display_data" })
        return outs
    }
    // kept: the pane's own copy (terminal cells by reference, not their output).
    function serialize(kept) {
        const out = []
        let n = 0
        for (let i = 0; i < cells.count; ++i) {
            const c = cells.get(i)
            let meta = {}
            try { meta = JSON.parse(c.meta) } catch (e) {}
            const att = meta["tessera.attachments"]
            delete meta["tessera.attachments"]
            const cell = { cell_type: c.type, id: c.nbid, metadata: meta, source: lines(c.source) }
            if (c.type === "code") {
                let outs = []
                if (c.tid > 0) {
                    outs = kept ? [] : termOutputs(c.tid)
                    meta.tessera = { cwd: c.cwd, status: c.status, duration_ms: Math.max(0, c.ended - c.started), started: c.started }
                    if (kept) Object.assign(meta.tessera, { tid: c.tid, mine: c.mine, cid: c.cid })
                } else try { outs = JSON.parse(c.outputs) } catch (e) {}
                if (c.lang !== "") meta.tessera = Object.assign({}, meta.tessera || {}, { lang: c.lang })
                cell.execution_count = c.count >= 0 ? c.count : (c.tid > 0 && !kept ? ++n : null)
                if (c.hideOutput) meta.jupyter = Object.assign({}, meta.jupyter || {}, { outputs_hidden: true })
                cell.outputs = outs.map(o => {
                    const r = Object.assign({}, o)
                    if (typeof r.text === "string") r.text = lines(r.text)
                    if (r.data) {
                        const d = {}
                        for (const k in r.data) d[k] = typeof r.data[k] === "string" && (k.startsWith("text/") || k === "image/svg+xml") ? lines(r.data[k]) : r.data[k]
                        r.data = d
                    }
                    if ((r.output_type === "display_data" || r.output_type === "execute_result") && !r.metadata) r.metadata = {}
                    return r
                })
            } else if (att) cell.attachments = att
            out.push(cell)
        }
        const md = Object.assign({}, metadata)
        delete md.tessera
        if (shellKernel) md.kernelspec = { name: "bash", display_name: "Bash", language: "bash" }
        else if (kernel) md.kernelspec = { name: kernel.name, display_name: kernel.display, language: kernel.language }
        md.language_info = Object.assign({}, md.language_info || {}, { name: codeLang })
        if (languageVersion !== "" && !shellKernel) md.language_info.version = languageVersion
        const t = {}
        if (Object.keys(kinds).length) t.kinds = kinds
        if (Object.keys(fileViews).length) t.fileViews = fileViews
        if (Object.keys(heights).length) t.heights = heights
        if (kept && retired.length) t.retired = retired.slice(-500)
        if (Object.keys(t).length) md.tessera = t
        return JSON.stringify(sorted({ cells: out, metadata: md, nbformat: 4, nbformat_minor: 5 }), null, 1) + "\n"
    }
    property string written: ""         // the path just saved to (not read back)
    function save() {
        if (path === "") { saveDialog.open(); return }
        writeTo(path)
    }
    function writeTo(p) {
        const err = controller.writeTextFile(p, serialize(false))
        if (err !== "") { saveError = err; return false }
        saveError = ""
        dirty = false
        if (p !== path) {
            // (The pane's notebook is now this file's: the shell's cells so far are in it.)
            written = p
            pathPicked(p)
        }
        return true
    }
    onPathChanged: {
        if (!controller) return
        if (path !== "" && path === written) { written = ""; return }
        load()
    }
    Component.onCompleted: if (controller) load()
    onControllerChanged: if (controller && !loaded) load()
    Component.onDestruction: { if (controller) { if (path === "" && dirty) keep(); controller.jupyterStop(key); stopLangKernels() } }
    FileDialog {
        id: saveDialog
        title: "Save as Jupyter Notebook"
        fileMode: FileDialog.SaveFile
        defaultSuffix: "ipynb"
        nameFilters: ["Jupyter notebooks (*.ipynb)"]
        currentFolder: nb.folder !== "" ? "file://" + nb.folder : ""
        onAccepted: nb.writeTo(decodeURIComponent(String(selectedFile).replace(/^file:\/\//, "")))
    }
    Shortcut { sequence: "Ctrl+S"; enabled: nb.visible && nb.activeFocus; context: Qt.WindowShortcut; onActivated: nb.save() }

    // (Test hooks: TESSERA_TEST_NB_RUN_ALL runs every cell once it's open,
    // NB_SAVE saves, NB_SCROLL_END scrolls down, NB_EXPORT=<path> saves there.)
    Timer { interval: 1500; running: nb.controller !== null && nb.visible && nb.controller.testEnv("NB_RUN_ALL") !== ""; onTriggered: nb.runAll() }
    Timer { interval: 20000; running: nb.controller !== null && nb.visible && nb.controller.testEnv("NB_SAVE") !== ""; onTriggered: nb.save() }
    Timer { interval: 16000; running: nb.controller !== null && nb.visible && nb.controller.testEnv("NB_SCROLL_END") !== ""; onTriggered: list.positionViewAtEnd() }
    Timer { interval: 3000; running: nb.visible && nb.controller !== null && nb.controller.testEnv("NB_EXPORT") !== ""; onTriggered: nb.writeTo(nb.controller.testEnv("NB_EXPORT")) }

    // ---- colors -----------------------------------------------------------------------
    function ansi(n, d) { const a = themeData.ansi || []; return a[n] || d }
    function paletteColor(n) {
        const a = themeData.ansi || []
        if (n < 16) return a[n] || ["#000000", "#cd3131", "#0dbc79", "#e5e510", "#2472c8", "#bc3fbc", "#11a8cd", "#e5e5e5"][n % 8]
        if (n < 232) {
            const v = (x) => x === 0 ? 0 : 55 + x * 40, i = n - 16
            return Qt.rgba(v(Math.floor(i / 36)) / 255, v(Math.floor(i / 6) % 6) / 255, v(i % 6) / 255, 1).toString()
        }
        const g = (8 + (n - 232) * 10) / 255
        return Qt.rgba(g, g, g, 1).toString()
    }
    // Output lines closer together than the font's own spacing (the
    // terminal's rows are this tight).
    // (Each line one terminal row high, at the terminal's font size: output
    // reads as it did in the terminal.)
    readonly property real outputRow: view && view.cellHeight > 0 ? view.cellHeight : 0
    readonly property real outputPointSize: view && view.fontPointSize > 0 ? view.fontPointSize : 10
    function tight(html) { return "<div style=\"line-height:" + (outputRow > 0 ? Math.round(outputRow) + "px" : "100%") + "\">" + html + "</div>" }
    function tightText(t) { return tight("<span style=\"white-space:pre-wrap\">" + t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") + "</span>") }
    function palette(html) { return html.replace(/var\(--vt-palette-(\d+)\)/g, (m, n) => paletteColor(parseInt(n))) }
    function colored(text) { return controller ? tight(palette(controller.ansiHtml(text))) : "" }
    function shortDir(p) {
        const home = controller ? controller.homeDir() : ""
        return home !== "" && p.startsWith(home) ? "~" + p.slice(home.length) : p
    }
    function seconds(ms) { ms = Math.max(0, ms); return ms < 1000 ? ms + " ms" : ms < 60000 ? (ms / 1000).toFixed(1) + " s" : Math.floor(ms / 60000) + " m " + Math.round((ms % 60000) / 1000) + " s" }
    function htmlPage(body) {
        const c = (x) => String(x)
        return "<!doctype html><html><head><meta charset='utf-8'><style>"
            + "html,body{margin:0;padding:0}body{background:" + c(Theme.bg) + ";color:" + c(Theme.text) + ";font:13px/1.5 'Noto Sans','Segoe UI',system-ui,sans-serif;padding:4px 2px;overflow-x:auto}"
            + "a{color:" + c(Theme.accent) + "}table{border-collapse:collapse;font-size:12px}th,td{border:1px solid " + c(Theme.divider) + ";padding:3px 8px;text-align:right}"
            + "th{background:" + c(Theme.field) + "}tr:nth-child(even) td{background:" + c(Theme.mix(Theme.bg, Theme.fg, 0.03)) + "}"
            + "code,pre{font-family:'" + fontFamily + "',monospace}pre{background:" + c(Theme.field) + ";padding:8px;border-radius:6px;overflow:auto}"
            + "img,svg{max-width:100%}"
            + "</style></head><body>" + body + "</body></html>"
    }
    readonly property var mimeOrder: ["text/html", "image/svg+xml", "image/png", "image/jpeg", "image/gif", "text/markdown", "text/latex", "application/json", "text/plain"]
    function bestMime(data) {
        for (const m of mimeOrder) if (data[m] !== undefined) return m
        const ks = Object.keys(data)
        return ks.length ? ks[0] : ""
    }
    // Markdown as a page: rich (headings, tables, a proportional font) or as
    // a terminal Markdown viewer draws it (monospace, #s kept in color).
    function markdownCss(asText) {
        const c = (x) => String(x), a = themeData.ansi || []
        const col = (n, d) => c(a[n] || d)
        if (asText) return "body{background:" + c(Theme.bg) + ";color:" + c(Theme.text) + ";font:13px/1.45 '" + fontFamily + "',monospace;margin:8px 10px}"
            + "h1,h2,h3,h4,h5,h6{font-size:1em;font-weight:bold;margin:1em 0 .4em}"
            + "h1{color:" + col(5, "#c678dd") + "}h2{color:" + col(4, "#61afef") + "}h3{color:" + col(6, "#56b6c2") + "}h4,h5,h6{color:" + col(2, "#98c379") + "}"
            + "h1::before{content:'# '}h2::before{content:'## '}h3::before{content:'### '}h4::before{content:'#### '}h5::before{content:'##### '}h6::before{content:'###### '}"
            + "h1{text-decoration:underline}p{margin:.5em 0}"
            + "a{color:" + col(4, "#61afef") + "}strong{color:" + c(Theme.fg) + "}em{color:" + col(3, "#e5c07b") + "}"
            + "code{font:inherit;color:" + col(3, "#e5c07b") + "}"
            + "pre{background:" + c(Theme.field) + ";padding:6px 10px;margin:.5em 0;overflow:auto}pre code{color:" + c(Theme.text) + "}"
            + "ul{list-style:none;padding-left:2ch}ul>li::before{content:'\\2022';color:" + col(4, "#61afef") + ";display:inline-block;width:2ch;margin-left:-2ch}"
            + "ol{padding-left:3ch}li{margin:.1em 0}"
            + "blockquote{margin:.5em 0;padding-left:1ch;color:" + c(Theme.textDim) + ";border-left:2px solid " + col(8, "#5c6370") + "}"
            + "table{border-collapse:collapse}th,td{border:1px solid " + c(Theme.divider) + ";padding:0 1ch}th{color:" + c(Theme.fg) + "}"
            + "img{max-width:100%}hr{border:0;border-top:1px dashed " + c(Theme.divider) + "}"
            + "::-webkit-scrollbar{width:9px;height:9px}::-webkit-scrollbar-thumb{background:" + c(Theme.divider) + ";border-radius:5px}"
        return "body{background:" + c(Theme.bg) + ";color:" + c(Theme.text) + ";font:14px/1.6 'Noto Sans','Segoe UI',system-ui,sans-serif;margin:12px 18px;max-width:900px}"
            + "a{color:" + c(Theme.accent) + "}h1,h2{border-bottom:1px solid " + c(Theme.divider) + ";padding-bottom:.25em}"
            + "h1{font-size:1.8em}h2{font-size:1.4em}h1,h2,h3,h4{margin:1.1em 0 .5em;line-height:1.25}"
            + "body>:first-child{margin-top:0}"
            + "code{font-family:" + fontFamily + ",monospace;font-size:.88em;background:" + c(Theme.field) + ";padding:.15em .35em;border-radius:5px}"
            + "pre{background:" + c(Theme.field) + ";padding:10px 12px;border-radius:7px;overflow:auto}pre code{background:none;padding:0}"
            + "blockquote{margin:0;padding:0 1em;color:" + c(Theme.textDim) + ";border-left:3px solid " + c(Theme.divider) + "}"
            + "table{border-collapse:collapse}th,td{border:1px solid " + c(Theme.divider) + ";padding:5px 10px}th{background:" + c(Theme.field) + "}"
            + "img{max-width:100%}hr{border:0;border-top:1px solid " + c(Theme.divider) + "}"
            + "::-webkit-scrollbar{width:9px;height:9px}::-webkit-scrollbar-thumb{background:" + c(Theme.divider) + ";border-radius:5px}"
    }

    // ---- the strip: running, the kernel, saving -------------------------------------------
    component Tool: Chip {
        base: Theme.panel
        Layout.minimumWidth: implicitWidth
    }
    readonly property bool narrow: width < 900
    readonly property bool tiny: width < 560
    Rectangle {
        id: bar
        width: parent.width
        height: 32
        color: Theme.panel
        z: 2
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.divider }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 8
            spacing: 6
            clip: true
            Text { topPadding: 0.001;
                visible: !nb.tiny
                text: nb.path !== "" ? nb.fileName + (nb.dirty ? " •" : "") : "Notebook"
                color: Theme.text
                font.pixelSize: 12
                font.weight: Font.DemiBold
                elide: Text.ElideMiddle
                Layout.maximumWidth: nb.narrow ? 120 : 240
                renderType: Text.NativeRendering
                ToolTip.visible: nameHover.hovered
                ToolTip.text: nb.path === "" ? "This pane's notebook: kept with its session. Save it as a .ipynb to keep it as a file." : nb.path
                ToolTip.delay: 400
                HoverHandler { id: nameHover }
            }
            Item { width: 2; visible: !nb.tiny }
            Tool { label: nb.narrow ? "▶▶" : "▶▶ Run All"; tip: "Run every cell, top to bottom"; onClicked: nb.runAll() }
            Tool { label: "■"; tip: "Interrupt"; onClicked: nb.interrupt() }
            Tool { visible: !nb.shellKernel; label: nb.narrow ? "↻" : "↻ Restart"; tip: "Restart the kernel (its variables are lost)"; onClicked: nb.restartKernel() }
            Tool { label: nb.narrow ? "⌫" : "Clear"; tip: "Clear every cell's output"; onClicked: nb.clearOutputs() }
            Tool {
                id: kernelsTool
                visible: nb.runningKernels.length > 0
                label: (nb.narrow ? "\u2699 " : "Kernels ") + nb.runningKernels.length
                tip: "The kernels running (every notebook's): see, restart or stop them"
                onClicked: { nb.refreshKernels(); kernelsPop.open() }
            }
            Rectangle { width: 1; height: 14; color: Theme.divider }
            Tool { label: nb.narrow ? "+" : "+ Code"; tip: "Add a code cell below (B)"; onClicked: nb.insertCell(nb.current + 1, "code") }
            Tool { label: nb.narrow ? "+M↓" : "+ Markdown"; tip: "Add a Markdown cell below"; onClicked: nb.insertCell(nb.current + 1, "markdown") }
            Item { Layout.fillWidth: true }
            // The kernel: which, and how it's doing.
            Rectangle {
                width: 8; height: 8; radius: 4
                color: nb.shellKernel || nb.kernelState === "idle" ? nb.ansi(2, "#98c379")
                     : nb.kernelState === "busy" ? nb.ansi(3, "#e5c07b")
                     : nb.kernelState === "starting" ? Theme.accent
                     : nb.kernelState === "dead" ? nb.ansi(1, "#e06c75") : Theme.textDim
                SequentialAnimation on opacity {
                    running: nb.kernelState === "starting" || nb.kernelState === "busy"
                    loops: Animation.Infinite
                    NumberAnimation { to: 0.3; duration: 500 }
                    NumberAnimation { to: 1; duration: 500 }
                    onRunningChanged: if (!running) parent.opacity = 1
                }
                ToolTip.visible: dotHover.hovered
                ToolTip.text: nb.shellKernel ? "ready" : nb.kernelState === "" ? "not started (starts with the first run)" : nb.kernelState
                HoverHandler { id: dotHover }
            }
            SlimCombo {
                id: kernelBox
                implicitWidth: nb.narrow ? 140 : 190
                Layout.minimumWidth: 90
                Layout.preferredWidth: implicitWidth
                model: nb.kernels.length ? nb.kernels.map(k => k.display) : ["No kernels found"]
                currentIndex: Math.max(0, nb.kernels.findIndex(k => k.name === nb.kernelName))
                onActivated: (i) => { if (nb.kernels[i]) nb.changeKernel(nb.kernels[i].name) }
                onPressedChanged: if (pressed) nb.listKernels()
                ToolTip.visible: hovered && !popup.visible
                ToolTip.delay: 500
                ToolTip.text: "What runs the code cells: this pane's shell, or a Jupyter kernel"
            }
            Tool {
                label: nb.path === "" ? (nb.narrow ? "Save…" : "Save as .ipynb…") : nb.dirty ? "Save" : "Saved"
                tip: nb.path === "" ? "Save as a Jupyter notebook (a file to run again or share)" : "Save the notebook (Ctrl+S)"
                onClicked: nb.save()
            }
            Tool { visible: nb.view !== null; label: ">_"; tip: "Show the terminal (Ctrl+Shift+J)"; onClicked: nb.terminalRequested() }
        }
    }
    // Starting, failing: said under the strip.
    Rectangle {
        id: banner
        anchors.top: bar.bottom
        width: parent.width
        visible: bannerText.text !== ""
        height: visible ? bannerText.implicitHeight + 14 : 0
        z: 2
        color: nb.kernelState === "dead" || nb.loadError !== "" || nb.saveError !== "" ? Theme.mix(Theme.panel, nb.ansi(1, "#e06c75"), 0.18) : Theme.mix(Theme.panel, Theme.accent, 0.12)
        Text { topPadding: 0.001;
            id: bannerText
            x: 12
            width: parent.width - 24 - (bannerActions.visible ? bannerActions.width + 8 : 0)
            anchors.verticalCenter: parent.verticalCenter
            wrapMode: Text.Wrap
            maximumLineCount: 8
            elide: Text.ElideRight
            color: Theme.text
            font.pixelSize: 12
            text: nb.loadError !== "" ? "Couldn't read " + nb.fileName + ": " + nb.loadError
                : nb.saveError !== "" ? "Couldn't save: " + nb.saveError
                : nb.shellKernel ? ""
                : nb.kernelState === "dead" ? (nb.kernelMessage || "The kernel stopped.")
                : nb.kernelState === "starting" ? (nb.kernelName === "tessera-nix-python"
                      ? "Starting Python from Nix… the first time, Nix fetches it (a minute or two)."
                      : nb.kernelMessage || "Starting the kernel…")
                : nb.kernels.length <= 1 && nb.kernelName === "" ? "No Jupyter kernel found. Install one (pip install ipykernel), or pick This pane's shell."
                : ""
        }
        Row {
            id: bannerActions
            visible: nb.kernelState === "dead"
            anchors.right: parent.right
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            Chip { base: banner.color; label: "Restart"; onClicked: nb.restartKernel() }
        }
    }

    // ---- the cards -----------------------------------------------------------------------
    Rectangle { anchors.fill: parent; color: Theme.bg; z: -1 }
    property bool dPending: false
    Timer { id: dTimer; interval: 700; onTriggered: nb.dPending = false }
    ListView {
        id: list
        anchors.top: banner.bottom
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.leftMargin: 10
        anchors.rightMargin: 10
        // (A pane's notebook fills up from the bottom, by the input editor,
        // like a terminal; a document starts at the top.)
        topMargin: nb.path === "" ? Math.max(10, height - contentHeight - bottomMargin) : 10
        bottomMargin: 10
        spacing: 12
        model: cells
        clip: true
        focus: true
        cacheBuffer: 1500
        interactive: false   // (a drag selects text; the wheel, touchpad and bar scroll)
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar {}
        MomentumWheel { flick: list }
        property real lastY: 0
        property real lastH: 0
        onContentHeightChanged: if (nb.sticking) { nb.pin(); Qt.callLater(nb.pin) }
        onHeightChanged: if (nb.sticking) { nb.pin(); Qt.callLater(nb.pin) }
        onContentYChanged: {
            // (Scrolled up by hand lets go; cards being laid out don't.)
            if (!nb.pinning && contentY < lastY - 2 && contentHeight === lastH) nb.sticking = false
            lastH = contentHeight
            if (nb.visible && height > 0) nb.wasAtEnd = atYEnd || nb.sticking
            lastY = contentY
        }
        Keys.onPressed: (e) => {
            const i = nb.current
            const plain = !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))
            const enter = e.key === Qt.Key_Return || e.key === Qt.Key_Enter
            if (enter && (e.modifiers & Qt.ShiftModifier)) { nb.run(i); nb.focusCell(Math.min(cells.count - 1, i + 1), false); e.accepted = true }
            else if (enter && (e.modifiers & Qt.ControlModifier)) { nb.run(i); e.accepted = true }
            else if (enter) { nb.focusCell(i, true); e.accepted = true }
            else if ((e.key === Qt.Key_Up || e.key === Qt.Key_K) && plain) { nb.focusCell(Math.max(0, i - 1), false); e.accepted = true }
            else if ((e.key === Qt.Key_Down || e.key === Qt.Key_J) && plain) { nb.focusCell(Math.min(cells.count - 1, i + 1), false); e.accepted = true }
            else if (e.key === Qt.Key_A && plain) { nb.insertCell(i, "code"); e.accepted = true }
            else if (e.key === Qt.Key_B && plain) { nb.insertCell(i + 1, "code"); e.accepted = true }
            else if (e.key === Qt.Key_M && plain) { nb.setType(i, "markdown"); e.accepted = true }
            else if (e.key === Qt.Key_Y && plain) { nb.setType(i, "code"); e.accepted = true }
            else if (e.key === Qt.Key_Z && plain) { nb.undoDelete(); e.accepted = true }
            else if (e.key === Qt.Key_O && plain && cells.get(i)) { cells.setProperty(i, "hideOutput", !cells.get(i).hideOutput); nb.touch(); e.accepted = true }
            else if (e.key === Qt.Key_D && plain) {
                if (nb.dPending) { nb.dPending = false; nb.deleteCell(i) } else { nb.dPending = true; dTimer.restart() }
                e.accepted = true
            } else if (e.key === Qt.Key_Delete && plain) { nb.deleteCell(i); e.accepted = true }
        }

        delegate: Rectangle {
            id: cell
            required property int index
            required property string cid
            required property string type
            required property string source
            required property int tid
            required property bool mine
            required property string outputs
            required property int count
            required property bool running
            required property bool failed
            required property double started
            required property double ended
            required property string cwd
            required property int status
            required property string tkind
            required property bool editing
            required property bool hideOutput
            required property string lang
            readonly property var ask: nb.asking[cid] || null
            readonly property var langKernel: lang !== "" ? nb.langKernels[lang] || null : null
            readonly property bool selected: nb.current === index
            readonly property bool code: type !== "markdown"
            readonly property bool fromTerm: tid > 0
            readonly property var outs: { try { return JSON.parse(outputs) } catch (e) { return [] } }
            readonly property string kind: fromTerm ? nb.shownKind(cid, tkind, source) : ""
            readonly property bool hasOutput: code && (fromTerm || outs.length > 0)
            readonly property bool expanded: nb.expandedId === cid
            readonly property bool plainMd: !code && !editing
            width: list.width - 12
            height: (plainMd ? mdBody.height + 8 : header.height + (editing ? editorBox.height + 8 : 0)
                     + (hasOutput ? (hideOutput ? 22 : body.height + 12) : 4) + (ask ? askBox.height + 6 : 0))
            radius: 8
            color: plainMd ? "transparent" : Theme.mix(Theme.bg, Theme.fg, 0.035)
            border.color: selected && list.activeFocus ? Theme.mix(Theme.divider, Theme.accent, 0.55)
                        : plainMd ? (cellHover.hovered || selected ? Theme.divider : "transparent") : Theme.divider
            function focusEditor() { if (editorLoader.item) editorLoader.item.focusIn() }
            HoverHandler { id: cellHover }
            TapHandler { onTapped: { nb.current = cell.index; if (!(editorLoader.item && editorLoader.item.hasFocus)) list.forceActiveFocus() } }

            // ---- header: status, the code, folder, time, view ----
            RowLayout {
                id: header
                visible: !cell.plainMd
                x: 10
                width: parent.width - 20
                height: 26
                spacing: 6
                // Status; a click runs it (or stops it).
                Text { topPadding: 0.001;
                    text: cell.running ? "◌" : cell.ended > 0 || cell.fromTerm ? (cell.failed ? "✘" : "✓") : cell.code ? "▶" : "M↓"
                    color: cell.running ? Theme.accent : cell.ended > 0 || cell.fromTerm ? (cell.failed ? nb.ansi(1, "#e06c75") : nb.ansi(2, "#98c379"))
                         : statusHover.hovered ? Theme.accent : Theme.textDim
                    font.pixelSize: 13
                    font.weight: Font.Bold
                    Layout.minimumWidth: 14
                    horizontalAlignment: Text.AlignHCenter
                    RotationAnimation on rotation { running: cell.running; from: 0; to: 360; duration: 1200; loops: Animation.Infinite; onRunningChanged: if (!running) parent.rotation = 0 }
                    HoverHandler { id: statusHover; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: { nb.current = cell.index; cell.running ? nb.interrupt() : nb.run(cell.index) } }
                    ToolTip.visible: statusHover.hovered
                    ToolTip.delay: 500
                    ToolTip.text: cell.running ? "Interrupt" : cell.code ? "Run (Ctrl+Enter)" : "Show it (Ctrl+Enter)"
                }
                // (A cell in another language: its language, and its kernel coming up.)
                Text { topPadding: 0.001;
                    visible: cell.lang !== ""
                    text: cell.lang + (cell.running && cell.langKernel && cell.langKernel.state === "starting" ? " \u00b7 " + (cell.langKernel.message || ("starting " + (cell.langKernel.display || "its kernel") + "\u2026")) : "")
                    color: nb.ansi(5, "#c678dd")
                    font.pixelSize: 10
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                    Layout.maximumWidth: 360
                }
                Text { topPadding: 0.001;
                    visible: cell.count >= 0 && cell.code
                    text: "[" + cell.count + "]"
                    color: Theme.textDim
                    font.family: nb.fontFamily
                    font.pixelSize: 10
                }
                Text { topPadding: 0.001;
                    Layout.fillWidth: true
                    text: cell.editing ? (cell.code ? "" : "Markdown") : cell.source.trim() !== "" ? cell.source.trim().replace(/\s*\n\s*/g, "  ⏎  ") : "(empty)"
                    maximumLineCount: 1
                    wrapMode: Text.NoWrap
                    elide: Text.ElideRight
                    color: cell.editing ? Theme.textDim : Theme.text
                    font.family: nb.fontFamily
                    font.pointSize: 9
                    font.weight: Font.DemiBold
                    font.underline: srcHover.hovered && !cell.editing
                    renderType: Text.NativeRendering
                    HoverHandler { id: srcHover; cursorShape: Qt.IBeamCursor }
                    TapHandler { onTapped: nb.focusCell(cell.index, true) }
                }
                Text { topPadding: 0.001; visible: cell.cwd !== "" && !nb.narrow; text: nb.shortDir(cell.cwd); color: Theme.textDim; font.pixelSize: 10; elide: Text.ElideMiddle; Layout.maximumWidth: 200 }
                Text { topPadding: 0.001;
                    visible: cell.started > 1
                    text: (cell.ended > 0 ? nb.seconds(cell.ended - cell.started) + "  ·  " : "") + Qt.formatTime(new Date(cell.started), "hh:mm")
                          + (cell.ended > 0 && cell.fromTerm && cell.status !== 0 ? "  ·  exit " + cell.status : "")
                    color: Theme.textDim
                    font.pixelSize: 10
                }
                Chip {
                    id: kindChip
                    visible: cell.fromTerm
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: (nb.viewNames.find(v => v[0] === cell.kind) || ["", cell.kind])[1] + ((nb.kinds[cell.cid] || "auto") === "auto" ? " · auto" : "") + " ▾"
                    tip: "Show the output as…"
                    onClicked: kindMenu.popup(kindChip, 0, kindChip.height + 2)
                }
                Chip { visible: cell.hasOutput; base: cell.color; label: "Copy"; tip: "Copy the output"; Layout.minimumWidth: implicitWidth
                       onClicked: nb.copyText(nb.outputText(cell.index)) }
                Chip { visible: cell.code && !cell.running && cell.source.trim() !== ""; base: cell.color; label: "↻"; tip: "Run it again"; Layout.minimumWidth: implicitWidth
                       onClicked: { nb.current = cell.index; nb.run(cell.index) } }
                Chip {
                    visible: cell.hasOutput
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: cell.expanded ? "⤡ Restore" : "⤢ Expand"
                    tip: cell.expanded ? "Back to its own size" : "Fill the window with this output"
                    onClicked: {
                        nb.expandedId = cell.expanded ? "" : cell.cid
                        cells.setProperty(cell.index, "hideOutput", false)
                        const i = cell.index
                        Qt.callLater(() => list.positionViewAtIndex(i, ListView.Beginning))
                    }
                }
                Chip { id: moreChip; base: cell.color; label: "⋯"; tip: "More"; Layout.minimumWidth: implicitWidth
                       onClicked: moreMenu.popup(moreChip, 0, moreChip.height + 2) }
                // (Test hook: TESSERA_TEST_EXPAND_CELL fills the window with the first cell.)
                Timer { interval: 2500; running: cell.index === 0 && nb.controller !== null && nb.controller.testEnv("EXPAND_CELL") !== ""; onTriggered: nb.expandedId = cell.cid }
                // (Test hook: TESSERA_TEST_OPEN_KIND_MENU opens the first cell's menu.)
                Timer { interval: 1500; running: cell.index === 0 && cell.fromTerm && nb.controller !== null && nb.controller.testEnv("OPEN_KIND_MENU") !== ""; onTriggered: kindMenu.popup(kindChip, 0, kindChip.height + 2) }
            }
            SlimMenu {
                id: kindMenu
                Repeater {
                    model: nb.viewNames
                    delegate: SlimMenuItem {
                        required property var modelData
                        text: modelData[1]
                        checkable: true
                        checked: (nb.kinds[cell.cid] || "auto") === modelData[0]
                        onTriggered: {
                            const k = Object.assign({}, nb.kinds); k[cell.cid] = modelData[0]; nb.kinds = k
                            nb.touch()
                            // (Rich or text Markdown becomes the default too.)
                            if (modelData[0] === "markdown" || modelData[0] === "mdtext") nb.markdownStylePicked(modelData[0] === "mdtext" ? "text" : "rich")
                        }
                    }
                }
            }
            SlimMenu {
                id: moreMenu
                SlimMenuItem { text: cell.code ? "Edit" : "Edit the Markdown"; onTriggered: nb.focusCell(cell.index, true) }
                SlimMenuItem { text: "Run the cells above"; visible: cell.code; onTriggered: nb.runAbove(cell.index) }
                SlimMenuItem { text: cell.code ? "Make it Markdown" : "Make it code"; onTriggered: nb.setType(cell.index, cell.code ? "markdown" : "code") }
                SlimMenuItem { text: cell.hideOutput ? "Show the output" : "Hide the output"; visible: cell.hasOutput; onTriggered: { cells.setProperty(cell.index, "hideOutput", !cell.hideOutput); nb.touch() } }
                SlimMenuItem { text: "Move up"; enabled: cell.index > 0; onTriggered: nb.moveCell(cell.index, -1) }
                SlimMenuItem { text: "Move down"; enabled: cell.index < cells.count - 1; onTriggered: nb.moveCell(cell.index, 1) }
                SlimMenuItem { text: "Add code below"; onTriggered: nb.insertCell(cell.index + 1, "code") }
                SlimMenuItem { text: "Add Markdown below"; onTriggered: nb.insertCell(cell.index + 1, "markdown") }
                SlimMenuItem { text: "Delete"; onTriggered: nb.deleteCell(cell.index) }
            }

            // ---- the code (or Markdown) being written ----
            Rectangle {
                id: editorBox
                visible: cell.editing
                anchors.top: header.bottom
                x: 10
                width: parent.width - 20
                height: visible ? editorLoader.height + 2 : 0
                radius: 6
                color: Theme.field
                border.color: editorLoader.item && editorLoader.item.hasFocus ? Theme.accent : Theme.divider
                Loader {
                    id: editorLoader
                    active: cell.editing
                    width: parent.width - 2
                    x: 1; y: 1
                    sourceComponent: cellEditor
                    onLoaded: { item.cellIndex = Qt.binding(() => cell.index); item.lang = Qt.binding(() => cell.type === "markdown" ? "markdown" : cell.type === "raw" ? "" : cell.lang !== "" ? cell.lang : nb.codeLang) }
                }
            }

            // ---- a Markdown cell, shown ----
            Item {
                id: mdBody
                visible: cell.plainMd
                x: 4
                y: 4
                width: parent.width - 8
                height: visible ? Math.max(24, mdView.implicitHeight) : 0
                Loader {
                    id: mdView
                    active: cell.plainMd
                    width: parent.width
                    height: item ? item.implicitHeight : 0
                    sourceComponent: markdownPage
                    onLoaded: { item.md = Qt.binding(() => cell.source); item.baseDir = Qt.binding(() => nb.folder); item.asText = Qt.binding(() => nb.markdownStyle === "text") }
                }
                // (The page takes the clicks: double-click to write, a chip for more.)
                Row {
                    anchors.right: parent.right
                    anchors.rightMargin: 6
                    y: 4
                    z: 3
                    spacing: 4
                    visible: cellHover.hovered || cell.selected
                    Chip { base: Theme.bg; label: "Edit"; tip: "Write the Markdown"; onClicked: nb.focusCell(cell.index, true) }
                    Chip { id: mdMore; base: Theme.bg; label: "⋯"; tip: "More"; onClicked: moreMenu.popup(mdMore, 0, mdMore.height + 2) }
                }
            }

            // ---- the output, in its own scrolling pane ----
            Text { topPadding: 0.001;
                visible: cell.hasOutput && cell.hideOutput
                anchors.top: editorBox.visible ? editorBox.bottom : header.bottom
                x: 12
                text: "… output hidden (O shows it)"
                color: Theme.textDim
                font.pixelSize: 11
                font.underline: hiddenHover.hovered
                HoverHandler { id: hiddenHover; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: { cells.setProperty(cell.index, "hideOutput", false); nb.touch() } }
            }
            Item {
                id: body
                visible: cell.hasOutput && !cell.hideOutput
                anchors.top: editorBox.visible ? editorBox.bottom : header.bottom
                anchors.topMargin: editorBox.visible ? 6 : 0
                x: 10
                width: parent.width - 20
                // Its natural size up to 460 px, or what it was dragged to, or the
                // whole window when expanded.
                readonly property real natural: loader.item ? loader.item.implicitHeight : 0
                height: !visible ? 0 : cell.expanded ? Math.max(80, list.height - header.height - 12)
                      : nb.heights[cell.cid] > 0 ? nb.heights[cell.cid] : Math.min(Math.max(natural, 16), 460)
                clip: true
                Loader {
                    id: loader
                    anchors.fill: parent
                    // (Re-read while the command runs.)
                    // (and once more when it's done)
                    // (and when the terminal's kept cells come back after a restart)
                    property var tick: [cell.running ? nb.tick : cell.ended, nb.termRev]
                    property int tid: cell.tid
                    property var outs: cell.outs
                    property string cwd: cell.cwd
                    property string command: cell.source
                    property string cid: cell.cid
                    property bool running: cell.running
                    readonly property string kind: cell.kind
                    sourceComponent: !cell.fromTerm ? jupyterOutputs
                                   : cell.kind === "html" ? htmlView
                                   : cell.kind === "markdown" || cell.kind === "mdtext" ? markdownView
                                   : cell.kind === "image" ? imageView
                                   : ["json", "csv", "tsv", "svg"].indexOf(cell.kind) >= 0 ? richView
                                   : cell.kind === "diff" ? diffView
                                   : cell.kind === "files" ? filesView
                                   : cell.kind === "env" ? envView
                                   : cell.kind === "text" ? plainView : termView
                }
            }

            // ---- the kernel asking for input: its prompt, a field, Enter sends ----
            Row {
                id: askBox
                visible: cell.ask !== null
                anchors.top: body.visible ? body.bottom : header.bottom
                anchors.topMargin: 4
                x: 12
                width: parent.width - 24
                height: visible ? 26 : 0
                spacing: 6
                Text {
                    id: askPrompt
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    text: cell.ask ? (cell.ask.prompt || (cell.ask.password ? "Password:" : "Input:")) : ""
                    color: Theme.text
                    font.family: nb.fontFamily
                    font.pointSize: nb.outputPointSize
                }
                TextField {
                    id: askField
                    width: Math.max(160, parent.width - askPrompt.width - 12)
                    height: 24
                    echoMode: cell.ask && cell.ask.password ? TextInput.Password : TextInput.Normal
                    color: Theme.text
                    font.family: nb.fontFamily
                    font.pointSize: nb.outputPointSize
                    placeholderText: "Enter sends it"
                    background: Rectangle { radius: 4; color: Theme.bg; border.color: askField.activeFocus ? Theme.accent : Theme.divider }
                    onVisibleChanged: if (visible) { text = ""; Qt.callLater(forceActiveFocus) }
                    onAccepted: nb.answer(cell.cid, text)
                }
            }

            // ---- the bottom edge: drag to resize, double-click for its own size ----
            Item {
                id: grip
                visible: body.visible
                anchors.bottom: parent.bottom
                width: parent.width
                height: 10
                property real startHeight: 0
                Rectangle {
                    anchors.centerIn: parent
                    width: 36; height: 3; radius: 1.5
                    color: gripDrag.active ? Theme.accent : Theme.divider
                    opacity: gripHover.hovered || gripDrag.active ? 1 : 0.5
                }
                HoverHandler { id: gripHover; cursorShape: Qt.SizeVerCursor }
                DragHandler {
                    id: gripDrag
                    target: null
                    xAxis.enabled: false
                    cursorShape: Qt.SizeVerCursor
                    onActiveChanged: if (active) { grip.startHeight = body.height; if (cell.expanded) nb.expandedId = "" }
                    onTranslationChanged: if (active) nb.setHeight(cell.cid, Math.max(48, Math.round(grip.startHeight + translation.y)))
                }
                TapHandler { onDoubleTapped: { nb.setHeight(cell.cid, 0); if (cell.expanded) nb.expandedId = "" } }
            }
        }
        footer: Item {
            width: list.width
            height: cells.count > 0 ? 40 : 0
            Row {
                anchors.centerIn: parent
                spacing: 8
                visible: cells.count > 0
                Chip { base: Theme.bg; label: "+ Code"; tip: "Add a code cell at the end"; onClicked: nb.insertCell(cells.count, "code") }
                Chip { base: Theme.bg; label: "+ Markdown"; tip: "Add a Markdown cell at the end"; onClicked: nb.insertCell(cells.count, "markdown") }
            }
        }
        Column {
            anchors.centerIn: parent
            width: parent.width - 60
            spacing: 12
            visible: cells.count === 0
            Text { topPadding: 0.001;
                width: parent.width
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                text: "Notebook: each command you run becomes a cell, its output shown for what it is (a table, JSON, HTML, a picture, ...). "
                    + "Write cells here too, run by this pane's shell or a Jupyter kernel, and save it all as a .ipynb."
                color: Theme.textDim
                font.pixelSize: 13
            }
            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                spacing: 8
                Chip { base: Theme.bg; label: "+ Code"; onClicked: nb.insertCell(0, "code") }
                Chip { base: Theme.bg; label: "+ Markdown"; onClicked: nb.insertCell(0, "markdown") }
            }
        }
    }
    TextEdit { id: clip; visible: false }
    function copyText(t) { clip.text = t; clip.selectAll(); clip.copy() }
    function outputText(i) {
        const c = cells.get(i)
        if (!c) return ""
        if (c.tid > 0) return view ? view.cellOutput(c.tid, "text") : ""
        let outs = []
        try { outs = JSON.parse(c.outputs) } catch (e) {}
        return outs.map(o => o.output_type === "stream" ? o.text
                           : o.output_type === "error" ? o.ename + ": " + o.evalue + "\n"
                           : o.data && o.data["text/plain"] !== undefined ? o.data["text/plain"] : "").join("")
    }

    // ---- a cell's editor ------------------------------------------------------------------
    Component {
        id: cellEditor
        Item {
            id: ed
            property int cellIndex: -1
            property string lang: "bash"
            readonly property bool hasFocus: edit.activeFocus
            implicitHeight: edit.implicitHeight + 14
            height: implicitHeight
            function focusIn() { edit.forceActiveFocus(); if (edit.cursorPosition === 0 && edit.length > 0) edit.cursorPosition = edit.length }
            TextEdit {
                id: edit
                property bool keepsBacktab: true   // (Shift+Tab outdents here)
                x: 10; y: 7
                width: parent.width - 20
                text: cells.get(ed.cellIndex) ? cells.get(ed.cellIndex).source : ""
                font.family: nb.fontFamily
                font.pointSize: 10
                color: Theme.text
                selectionColor: Theme.mix(Theme.field, Theme.accent, 0.45)
                selectedTextColor: Theme.text
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                persistentSelection: false
                renderType: Text.NativeRendering
                textFormat: TextEdit.PlainText
                tabStopDistance: 4 * fm.advanceWidth(" ")
                FontMetrics { id: fm; font: edit.font }
                onTextChanged: {
                    if (ed.cellIndex < 0 || ed.cellIndex >= cells.count || cells.get(ed.cellIndex).source === text) return
                    cells.setProperty(ed.cellIndex, "source", text)
                    nb.touch()
                    hlTimer.restart()
                    comp.visible = false
                }
                onActiveFocusChanged: if (activeFocus) nb.current = ed.cellIndex
                Keys.onPressed: (e) => ed.key(e)
                // (As in the pane's editor: a tap of right Shift is a new line.)
                Keys.onReleased: (e) => {
                    if (e.key === Qt.Key_Shift && ed.shiftRight && Date.now() - ed.shiftAt < 450) { ed.newline(); e.accepted = true }
                    if (e.key === Qt.Key_Shift) ed.shiftRight = false
                }
                Text { topPadding: 0.001;
                    visible: edit.text === ""
                    text: cells.get(ed.cellIndex) && cells.get(ed.cellIndex).type === "markdown" ? "Markdown — Enter shows it, right Shift for a new line"
                        : nb.shellKernel ? "A command — Enter runs it in this pane's shell, right Shift for a new line" : "Code — Enter runs it, right Shift for a new line"
                    color: Theme.textDim
                    font: edit.font
                }
            }
            SyntaxHighlighter {
                id: hl
                document: edit.textDocument
                errorColor: "transparent"
                colors: ({
                    keyword: nb.ansi(5, "#c678dd"), string: nb.ansi(2, "#98c379"), comment: Theme.textDim,
                    number: nb.ansi(3, "#d19a66"), constant: nb.ansi(3, "#d19a66"), function: nb.ansi(4, "#61afef"),
                    operator: nb.ansi(6, "#56b6c2"), property: nb.ansi(6, "#56b6c2"), variable: Theme.text,
                    embedded: nb.ansi(13, "#c678dd"), type: nb.ansi(3, "#e5c07b"), constructor: nb.ansi(3, "#e5c07b"),
                    punctuation: Theme.textDim, escape: nb.ansi(6, "#56b6c2"), attribute: nb.ansi(3, "#e5c07b"),
                    tag: nb.ansi(1, "#e06c75")
                })
            }
            Timer { id: hlTimer; interval: 120; onTriggered: ed.highlight() }
            Component.onCompleted: highlight()
            onLangChanged: highlight()
            function highlight() {
                if (!nb.controller || lang === "") return
                try {
                    const r = JSON.parse(nb.controller.syntax(lang, edit.text))
                    r.errors = []
                    hl.analysis = JSON.stringify(r)
                } catch (e) {}
            }
            function indentOf(line) { const m = line.match(/^[ \t]*/); return m ? m[0] : "" }
            // Run, then go on: the next cell, or a new one at the end.
            function runAndAdvance() {
                const i = cellIndex
                nb.run(i)
                if (i + 1 >= cells.count) nb.insertCell(cells.count, "code", true)
                else nb.focusCell(i + 1, cells.get(i + 1).editing)
            }
            property bool shiftRight: false
            property double shiftAt: 0
            // A new line, indented like this one (more after a ":").
            function newline() {
                const before = edit.text.slice(0, edit.cursorPosition)
                const line = before.slice(before.lastIndexOf("\n") + 1)
                let ind = indentOf(line)
                if (lang === "python" && /:\s*$/.test(line)) ind += "    "
                edit.remove(edit.selectionStart, edit.selectionEnd)
                edit.insert(edit.cursorPosition, "\n" + ind)
            }
            function key(e) {
                // (X11/Wayland keycode 62, Windows 54, macOS 60: right Shift.)
                if (e.key === Qt.Key_Shift && !e.isAutoRepeat) {
                    shiftRight = e.nativeScanCode === 62 || e.nativeScanCode === 54 || e.nativeVirtualKey === 60 || e.nativeVirtualKey === 0xffe2
                    shiftAt = Date.now()
                } else shiftRight = false
                const ctrl = e.modifiers & Qt.ControlModifier, shift = e.modifiers & Qt.ShiftModifier, alt = e.modifiers & Qt.AltModifier
                const enter = e.key === Qt.Key_Return || e.key === Qt.Key_Enter
                if (comp.visible) {
                    if (e.key === Qt.Key_Down) { comp.pick = Math.min(comp.items.length - 1, comp.pick + 1); e.accepted = true; return }
                    if (e.key === Qt.Key_Up) { comp.pick = Math.max(0, comp.pick - 1); e.accepted = true; return }
                    if (enter || e.key === Qt.Key_Tab) { comp.accept(); e.accepted = true; return }
                    if (e.key === Qt.Key_Escape) { comp.visible = false; e.accepted = true; return }
                }
                // As in the pane's editor: Enter runs, Shift+Enter (or a tap of
                // right Shift) is a new line.
                if (enter && shift) { newline(); e.accepted = true }
                else if (enter && ctrl) { nb.run(cellIndex); list.forceActiveFocus(); e.accepted = true }
                else if (enter && alt) { nb.run(cellIndex); nb.insertCell(cellIndex + 1, "code"); e.accepted = true }
                else if (enter) { runAndAdvance(); e.accepted = true } else if (e.key === Qt.Key_Escape) { nb.current = cellIndex; nb.stopEditing(cellIndex); e.accepted = true }
                else if (e.key === Qt.Key_Tab && !ctrl) {
                    const before = edit.text.slice(0, edit.cursorPosition)
                    if (edit.selectedText === "" && /[\w.\]\)'"]$/.test(before) && !nb.shellKernel && nb.kernelUp) comp.ask()
                    else edit.insert(edit.cursorPosition, "    ")
                    e.accepted = true
                } else if (e.key === Qt.Key_Backtab) {
                    // Dedent the line.
                    const p = edit.cursorPosition, start = edit.text.lastIndexOf("\n", p - 1) + 1
                    const n = edit.text.slice(start, start + 4).match(/^ {1,4}/)
                    if (n) edit.remove(start, start + n[0].length)
                    e.accepted = true
                } else if (e.key === Qt.Key_Up && !shift && edit.text.lastIndexOf("\n", edit.cursorPosition - 1) < 0 && cellIndex > 0) {
                    nb.focusCell(cellIndex - 1, cells.get(cellIndex - 1).editing); e.accepted = true
                } else if (e.key === Qt.Key_Down && !shift && edit.text.indexOf("\n", edit.cursorPosition) < 0 && cellIndex + 1 < cells.count) {
                    nb.focusCell(cellIndex + 1, cells.get(cellIndex + 1).editing); e.accepted = true
                }
            }
            // Completion from the kernel.
            Rectangle {
                id: comp
                visible: false
                z: 10
                property var items: []
                property int pick: 0
                property int start: 0
                property int end: 0
                property string req: ""
                width: Math.min(360, Math.max(160, compList.contentItem.childrenRect.width + 16))
                height: Math.min(8, items.length) * 22 + 8
                radius: 6
                color: Theme.mix(Theme.bg, Theme.fg, 0.08)
                border.color: Theme.divider
                x: Math.min(parent.width - width, edit.x + edit.cursorRectangle.x)
                y: edit.y + edit.cursorRectangle.y + edit.cursorRectangle.height + 2
                function ask() {
                    req = nb.key + "-comp-" + Math.random().toString(36).slice(2)
                    nb.controller.jupyterComplete(nb.key, req, edit.text, edit.cursorPosition)
                }
                function accept() {
                    const m = items[pick]
                    visible = false
                    if (!m) return
                    edit.remove(start, Math.max(start, edit.cursorPosition))
                    edit.insert(start, m)
                }
                Connections {
                    target: nb.controller
                    function onJupyterEvent(key, json) {
                        if (key !== nb.key || json.indexOf(comp.req) < 0) return
                        const e = JSON.parse(json)
                        if (e.type !== "complete" || e.request !== comp.req) return
                        comp.items = (e.matches || []).slice(0, 200)
                        comp.start = e.start; comp.end = e.end; comp.pick = 0
                        if (comp.items.length === 1) { comp.accept(); return }
                        comp.visible = comp.items.length > 0
                    }
                }
                ListView {
                    id: compList
                    anchors.fill: parent
                    anchors.margins: 4
                    clip: true
                    model: comp.items
                    currentIndex: comp.pick
                    delegate: Rectangle {
                        required property string modelData
                        required property int index
                        width: compList.width
                        height: 22
                        radius: 4
                        color: index === comp.pick ? Theme.mix(Theme.panel, Theme.accent, 0.35) : "transparent"
                        Text { topPadding: 0.001;
                            x: 6
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData
                            color: Theme.text
                            font.family: nb.fontFamily
                            font.pointSize: 9
                        }
                        TapHandler { onTapped: { comp.pick = index; comp.accept() } }
                    }
                }
            }
        }
    }

    // ---- a terminal cell's output, by kind ---------------------------------------------------
    // (Each reads its Loader: tid, tick, kind, cwd, command.)
    component OutText: TextEdit {
        BlockSelect { }
        readOnly: true
        selectByMouse: true
        selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
        selectedTextColor: Theme.text
        color: Theme.text
        font.family: nb.fontFamily
        font.pointSize: nb.outputPointSize
        renderType: Text.NativeRendering
    }
    Component {
        id: termView
        Flickable {
            id: tf
            readonly property var src: parent
            interactive: false   // (a drag selects text; the wheel and bar scroll)
            implicitHeight: Math.max(16, tt.implicitHeight + 8)
            contentWidth: Math.max(width, tt.implicitWidth)
            contentHeight: tt.implicitHeight + 8
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: tf.contentHeight > tf.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
            ScrollBar.horizontal: ScrollBar {}
            MomentumWheel { flick: tf }
            OutText {
                id: tt
                y: 4
                textFormat: TextEdit.RichText
                text: { tf.src.tick; return nb.view ? nb.tight(nb.palette(nb.view.cellOutput(tf.src.tid, "html"))) : "" }
            }
        }
    }
    // ---- ls and cd: the folder, to browse (FileBrowser.qml) ----
    function localPath(p) { let x = (p || "").replace(/^file:\/\/[^/]*/, ""); try { x = decodeURIComponent(x) } catch (e) {} return x }
    function shq(s) { return /^[A-Za-z0-9_\/.~+:@-]+$/.test(s) ? s : "'" + s.replace(/'/g, "'\\''") + "'" }
    Component {
        id: filesView
        FileBrowser {
            readonly property var src: parent
            readonly property var words: (src.command || "").trim().split(/\s+/)
            readonly property bool isCd: ["cd", "pushd", "z", "zi"].indexOf(words[0]) >= 0
            readonly property var args: words.slice(1)
            controller: nb.controller
            themeData: nb.themeData
            fontFamily: nb.fontFamily
            pointSize: nb.outputPointSize
            rowHeight: nb.outputRow > 0 ? nb.outputRow : 18
            flagAll: !isCd && args.some(a => /^-[a-zA-Z]*[aA]/.test(a) || a === "--all" || a === "--almost-all")
            flagLong: !isCd && (args.some(a => /^-[a-zA-Z]*l/.test(a)) || /^(ll|la)$/.test(words[0]))
            startDir: {
                const base = nb.localPath(src.cwd)
                const home = nb.controller ? nb.controller.homeDir() : "~"
                const a = args.find(x => !x.startsWith("-"))
                // (cd: where it went. With no folder, home; "cd -" and z: wherever the shell is now.)
                if (isCd && (!a || a === "-" || words[0] === "z" || words[0] === "zi")) return a || words[0] !== "cd" ? nb.shellDir : home
                if (!a) return base
                const p = a.replace(/^['"]|['"]$/g, "").replace(/^~(?=\/|$)/, home)
                return p.startsWith("/") ? p : base + "/" + p
            }
            refresh: src.tick ? src.tick.length : 0
            kept: nb.fileViews[src.cid] || null
            onKeptChanged2: (c) => { const m = Object.assign({}, nb.fileViews); m[src.cid] = c; nb.fileViews = m; nb.touch() }
            onCommandRequested: (c) => nb.commandRequested(c)
            onInsertRequested: (t) => nb.insertRequested(t)
            onViewRequested: (p) => nb.viewRequested(p)
        }
    }

    // ---- env: the variables, to edit (each change runs export/unset) ----
    Component {
        id: envView
        Column {
            id: ev
            readonly property var src: parent
            property string filter: ""
            readonly property var vars: {
                src.tick
                const out = []
                const text = nb.view ? nb.view.cellOutput(src.tid, "text") : ""
                for (let l of text.split("\n")) {
                    l = l.replace(/^(declare -x|typeset -x|export)\s+/, "")
                    const i = l.indexOf("=")
                    if (i <= 0 || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(l.substring(0, i))) continue
                    let v = l.substring(i + 1)
                    if (/^".*"$/.test(v) || /^'.*'$/.test(v)) v = v.slice(1, -1)
                    out.push({ k: l.substring(0, i), v: v })
                }
                out.sort((a, b) => a.k.localeCompare(b.k))
                const f = filter.toLowerCase()
                return f === "" ? out : out.filter(x => (x.k + "=" + x.v).toLowerCase().indexOf(f) >= 0)
            }
            spacing: 4
            topPadding: 4
            Row {
                spacing: 6
                width: parent.width
                TextField {
                    width: Math.min(260, parent.width / 3)
                    placeholderText: "Filter"
                    color: Theme.text
                    font.pixelSize: 12
                    background: Rectangle { radius: 5; color: Theme.bg; border.color: parent.activeFocus ? Theme.accent : Theme.divider }
                    onTextChanged: ev.filter = text
                }
                TextField {
                    id: newVar
                    width: Math.min(360, parent.width / 2)
                    placeholderText: "NEW_NAME=value, Enter to set"
                    color: Theme.text
                    font.family: nb.fontFamily
                    font.pixelSize: 12
                    background: Rectangle { radius: 5; color: Theme.bg; border.color: parent.activeFocus ? Theme.accent : Theme.divider }
                    onAccepted: {
                        const i = text.indexOf("=")
                        if (i > 0) { nb.commandRequested("export " + text.substring(0, i) + "=" + nb.shq(text.substring(i + 1))); text = "" }
                    }
                }
            }
            Repeater {
                model: ev.vars
                delegate: Row {
                    required property var modelData
                    width: ev.width
                    spacing: 8
                    Text { topPadding: 0.001;
                        width: Math.min(240, ev.width * 0.3)
                        anchors.verticalCenter: parent.verticalCenter
                        text: modelData.k
                        elide: Text.ElideRight
                        color: nb.ansi(4, "#61afef")
                        font.family: nb.fontFamily
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                        renderType: Text.NativeRendering
                    }
                    TextField {
                        id: vf
                        width: ev.width - Math.min(240, ev.width * 0.3) - 40
                        height: 22
                        text: modelData.v
                        color: Theme.text
                        font.family: nb.fontFamily
                        font.pixelSize: 12
                        padding: 3
                        background: Rectangle { radius: 4; color: vf.activeFocus ? Theme.bg : "transparent"; border.color: vf.activeFocus ? Theme.accent : "transparent" }
                        Component.onCompleted: cursorPosition = 0
                        onActiveFocusChanged: if (!activeFocus) cursorPosition = 0
                        // (Enter: export it with the new value.)
                        onAccepted: if (text !== modelData.v) nb.commandRequested("export " + modelData.k + "=" + nb.shq(text))
                    }
                    Text { topPadding: 0.001;
                        anchors.verticalCenter: parent.verticalCenter
                        text: "×"
                        color: xh.hovered ? nb.ansi(1, "#e06c75") : Theme.textDim
                        font.pixelSize: 14
                        renderType: Text.NativeRendering
                        HoverHandler { id: xh; cursorShape: Qt.PointingHandCursor }
                        TapHandler { onTapped: nb.commandRequested("unset " + modelData.k) }
                        ToolTip.visible: xh.hovered
                        ToolTip.text: "unset " + modelData.k
                    }
                }
            }
        }
    }
    Component {
        id: plainView
        Flickable {
            id: pf
            readonly property var src: parent
            interactive: false
            implicitHeight: Math.max(16, pt.implicitHeight + 8)
            contentHeight: pt.implicitHeight + 8
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: pf.contentHeight > pf.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
            MomentumWheel { flick: pf }
            OutText {
                id: pt
                y: 4
                width: pf.width - 12
                wrapMode: TextEdit.Wrap
                textFormat: TextEdit.RichText
                text: { pf.src.tick; return nb.view ? nb.tightText(nb.view.cellOutput(pf.src.tid, "text")) : "" }
            }
        }
    }
    Component {
        id: diffView
        Flickable {
            id: df
            readonly property var src: parent
            interactive: false
            implicitHeight: Math.max(16, dt.implicitHeight + 8)
            contentHeight: dt.implicitHeight + 8
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: df.contentHeight > df.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
            MomentumWheel { flick: df }
            OutText {
                id: dt
                y: 4
                width: df.width - 12
                textFormat: TextEdit.RichText
                wrapMode: TextEdit.WrapAnywhere
                text: {
                    df.src.tick
                    const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/ /g, "&nbsp;")
                    return (nb.view ? nb.view.cellOutput(df.src.tid, "text") : "").split("\n").map(l =>
                        l.startsWith("+") && !l.startsWith("+++") ? "<font color='" + nb.ansi(2, "#98c379") + "'>" + esc(l) + "</font>"
                        : l.startsWith("-") && !l.startsWith("---") ? "<font color='" + nb.ansi(1, "#e06c75") + "'>" + esc(l) + "</font>"
                        : l.startsWith("@@") ? "<font color='" + nb.ansi(6, "#56b6c2") + "'>" + esc(l) + "</font>" : esc(l)).join("<br>")
                }
            }
        }
    }
    Component {
        id: richView
        RichViewer {
            readonly property var src: parent
            embedded: true
            maxHeight: 100000
            kind: src.kind
            text: { src.tick; return nb.view ? nb.view.cellOutput(src.tid, "text") : "" }
            fontFamily: nb.fontFamily
            themeData: nb.themeData
        }
    }
    Component {
        id: imageView
        Item {
            readonly property var src: parent
            implicitHeight: (pic.sourceSize.height > 0 ? pic.sourceSize.height * Math.min(1, width / pic.sourceSize.width) : 100) + 8
            Image {
                id: pic
                anchors.fill: parent
                anchors.topMargin: 4
                anchors.bottomMargin: 4
                fillMode: Image.PreserveAspectFit
                horizontalAlignment: Image.AlignLeft
                source: { const b = nb.view ? nb.view.cellOutput(parent.src.tid, "image") : ""; return b !== "" ? "data:image/png;base64," + b : "" }
            }
        }
    }
    Component {
        id: markdownView
        // A command's Markdown (cat README.md): relative pictures from the
        // file's folder.
        Item {
            id: mv
            readonly property var src: parent
            implicitHeight: page.implicitHeight
            function baseDir() {
                const cwd = src.cwd || "/"
                const f = (src.command || "").trim().split(/\s+/).pop()
                if (!/\.(md|markdown|mdx)$/i.test(f) || f.indexOf("/") < 0) return cwd
                let d = f.replace(/\/[^/]*$/, "")
                if (d.startsWith("~")) d = (nb.controller ? nb.controller.homeDir() : "") + d.slice(1)
                return d.startsWith("/") ? d : cwd + "/" + d
            }
            Loader {
                id: page
                anchors.fill: parent
                sourceComponent: markdownPage
                onLoaded: {
                    item.html = Qt.binding(() => { mv.src.tick; return nb.view ? nb.view.cellOutput(mv.src.tid, "markdown") : "" })
                    item.baseDir = Qt.binding(() => mv.baseDir())
                    item.asText = Qt.binding(() => mv.src.kind === "mdtext")
                    item.scrolls = true
                }
            }
        }
    }
    Component {
        id: htmlView
        WebEngineView {
            id: web
            readonly property var src: parent
            implicitHeight: 360
            backgroundColor: "white"
            // Pages may not reach local files or run plugins.
            settings.localContentCanAccessFileUrls: false
            settings.pluginsEnabled: false
            Component.onCompleted: loadHtml(nb.view ? nb.view.cellOutput(src.tid, "text") : "", "about:blank")
            onLoadingChanged: (r) => {
                if (r.status === WebEngineView.LoadSucceededStatus)
                    runJavaScript("Math.ceil(document.body ? document.body.getBoundingClientRect().height + 24 : 0)", (h) => { if (h > 0) web.implicitHeight = Math.max(60, h) })
            }
            onNewWindowRequested: (req) => Qt.openUrlExternally(req.requestedUrl)
        }
    }
    // Markdown drawn as a page, in the theme's colors (scripts never run):
    // `md` (Markdown) or `html` (already made), pictures from baseDir.
    Component {
        id: markdownPage
        WebEngineView {
            id: mp
            property string md: ""
            property string html: ""
            property string baseDir: ""
            property bool asText: false
            property bool scrolls: false    // its own scrollbar (an output), or as tall as it is (a cell)
            implicitHeight: 40
            backgroundColor: "transparent"
            settings.javascriptEnabled: false
            settings.pluginsEnabled: false
            settings.localContentCanAccessFileUrls: true
            settings.localContentCanAccessRemoteUrls: true
            settings.showScrollBars: scrolls
            function show() {
                if (!nb.controller) return
                const body = md !== "" ? nb.controller.markdownHtml(md) : html
                loadHtml("<!doctype html><html><head><meta charset='utf-8'><style>" + nb.markdownCss(asText) + (scrolls ? "" : "body{margin:4px 10px;overflow:hidden}") + "</style></head><body>" + body + "</body></html>",
                         "file://" + (baseDir || "/") + "/")
            }
            Timer { id: again; interval: 60; onTriggered: mp.show() }
            onMdChanged: again.restart()
            onHtmlChanged: again.restart()
            onAsTextChanged: again.restart()
            onBaseDirChanged: again.restart()
            Component.onCompleted: again.restart()
            // (Its height: measured a few times as it lays out, fonts and pictures coming in.)
            onLoadingChanged: (r) => { if (r.status === WebEngineView.LoadSucceededStatus) { measure.n = 0; measure.start() } }
            onContentsSizeChanged: if (contentsSize.height > 0) mp.implicitHeight = Math.max(24, Math.ceil(contentsSize.height))
            Timer {
                id: measure
                interval: 80
                repeat: true
                property int n: 0
                onTriggered: {
                    mp.runJavaScript("Math.ceil(document.body.getBoundingClientRect().bottom + parseFloat(getComputedStyle(document.body).marginBottom || 0))", (h) => { if (h > 0) mp.implicitHeight = Math.max(24, h) })
                    if (++n > 12) stop()
                }
            }
            onNewWindowRequested: (req) => Qt.openUrlExternally(req.requestedUrl)
            onNavigationRequested: (req) => {
                // (Links open in the browser, not in the cell.)
                if (req.navigationType === WebEngineNavigationRequest.LinkClickedNavigation) { req.reject(); Qt.openUrlExternally(req.url) }
            }
        }
    }

    // ---- a Jupyter cell's outputs ----------------------------------------------------------
    Component {
        id: jupyterOutputs
        Flickable {
            id: jf
            readonly property var src: parent
            interactive: false
            implicitHeight: oc.height + 4
            contentHeight: oc.height + 4
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: ScrollBar { policy: jf.contentHeight > jf.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
            MomentumWheel { flick: jf }
            Column {
                id: oc
                y: 2
                width: jf.width - 10
                spacing: 4
                Repeater {
                    model: jf.src.outs
                    delegate: Loader {
                        required property var modelData
                        width: oc.width
                        property var out: modelData
                        sourceComponent: {
                            const o = modelData
                            if (o.output_type === "stream" || o.output_type === "error") return textOut
                            const m = nb.bestMime(o.data || {})
                            if (m === "image/png" || m === "image/jpeg" || m === "image/gif") return imageOut
                            if (m === "text/html" || m === "image/svg+xml") return htmlOut
                            if (m === "text/markdown") return markdownOut
                            return textOut
                        }
                    }
                }
                Text { topPadding: 0.001;
                    visible: jf.src.outs.length === 0
                    text: jf.src.running ? "" : "(no output)"
                    color: Theme.textDim
                    font.pixelSize: 11
                }
            }
        }
    }
    Component {
        id: textOut
        // stdout/stderr, a traceback, a value's text: with its colors.
        Rectangle {
            id: tbox
            readonly property var o: parent ? parent.out : ({})
            readonly property bool bad: o.output_type === "error" || (o.output_type === "stream" && o.name === "stderr")
            implicitHeight: tv.implicitHeight + 8
            height: implicitHeight
            color: bad ? Theme.mix(Theme.bg, nb.ansi(1, "#e06c75"), 0.07) : "transparent"
            radius: 4
            Flickable {
                id: tf
                interactive: false
                anchors.fill: parent
                anchors.margins: 4
                contentWidth: Math.max(width, tv.implicitWidth)
                contentHeight: tv.implicitHeight
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.horizontal: ScrollBar {}
                OutText {
                    id: tv
                    textFormat: TextEdit.RichText
                    text: {
                        const o = tbox.o
                        if (!o) return ""
                        if (o.output_type === "stream") return nb.colored(o.text || "")
                        if (o.output_type === "error") return nb.colored((o.traceback || []).join("\n") || (o.ename + ": " + o.evalue))
                        const d = o.data || {}
                        const t = d["text/plain"] !== undefined ? d["text/plain"]
                                : d["application/json"] !== undefined ? JSON.stringify(d["application/json"], null, 2)
                                : d["text/latex"] !== undefined ? d["text/latex"]
                                : d["application/vnd.jupyter.widget-view+json"] !== undefined ? "(an interactive widget: not shown here)"
                                : "(" + Object.keys(d).join(", ") + ")"
                        return nb.colored(t)
                    }
                }
            }
        }
    }
    Component {
        id: imageOut
        Item {
            readonly property var o: parent ? parent.out : ({})
            readonly property string mime: nb.bestMime(o.data || {})
            implicitHeight: pic.status === Image.Ready ? Math.min(pic.sourceSize.height, pic.sourceSize.height * width / Math.max(1, pic.sourceSize.width)) + 4 : 40
            height: implicitHeight
            Image {
                id: pic
                width: Math.min(parent.width, sourceSize.width)
                height: parent.height - 4
                fillMode: Image.PreserveAspectFit
                horizontalAlignment: Image.AlignLeft
                smooth: true
                mipmap: true
                source: o.data ? "data:" + mime + ";base64," + String(o.data[mime]).replace(/\s/g, "") : ""
            }
        }
    }
    Component {
        id: htmlOut
        // HTML (tables, plots) and SVG in a browser view; links open outside.
        WebEngineView {
            id: web
            readonly property var o: parent ? parent.out : ({})
            implicitHeight: 60
            height: implicitHeight
            backgroundColor: Theme.bg
            settings.localContentCanAccessFileUrls: false
            settings.pluginsEnabled: false
            settings.showScrollBars: true
            Component.onCompleted: {
                const d = o.data || {}
                loadHtml(nb.htmlPage(d["text/html"] !== undefined ? d["text/html"] : d["image/svg+xml"]), "about:blank")
            }
            onLoadingChanged: (r) => { if (r.status === WebEngineView.LoadSucceededStatus) measure.start() }
            // (Plots draw after loading: measured again shortly.)
            Timer {
                id: measure
                interval: 60
                repeat: true
                property int n: 0
                onTriggered: {
                    web.runJavaScript("Math.ceil(document.documentElement.scrollHeight)", (h) => { if (h > 0) web.implicitHeight = Math.min(900, Math.max(24, h)) })
                    if (++n > 20) { stop(); n = 0 }
                }
            }
            onNewWindowRequested: (req) => Qt.openUrlExternally(req.requestedUrl)
            onNavigationRequested: (req) => {
                if (req.navigationType === WebEngineNavigationRequest.LinkClickedNavigation) { req.reject(); Qt.openUrlExternally(req.url) }
            }
        }
    }
    Component {
        id: markdownOut
        Loader {
            readonly property var o: parent ? parent.out : ({})
            height: item ? item.implicitHeight : 40
            sourceComponent: markdownPage
            onLoaded: { item.md = o.data ? nb.joined(o.data["text/markdown"]) : ""; item.baseDir = nb.folder; item.asText = nb.markdownStyle === "text" }
        }
    }
}
