import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Tessera
import Tessera.Native

// A Jupyter notebook (.ipynb) in a pane, the way VS Code shows one: code and
// Markdown cells, each code cell run by a kernel (any installed Jupyter
// kernel, Python from Nix, or this pane's own shell), its outputs below it —
// text with its colors, tables and HTML, pictures, Markdown.
//
// Keys, while editing a cell: Shift+Enter runs it and goes to the next,
// Ctrl+Enter runs it, Alt+Enter runs it and adds one below, Tab completes
// (from the kernel), Esc leaves the cell. Between cells: ↑/↓ (or k/j) move,
// Enter edits, A/B add above/below, D D deletes, M/Y make it Markdown/code,
// Ctrl+S saves.
FocusScope {
    id: jv
    property string path: ""
    property var controller: null
    property var term: null             // the pane's terminal (the shell kernel)
    property var themeData: ({})
    property string fontFamily: "monospace"
    signal closeRequested()

    readonly property string key: "jn-" + Math.random().toString(36).slice(2)
    readonly property string fileName: path.replace(/^.*\//, "") || "Untitled.ipynb"
    readonly property string folder: path.indexOf("/") >= 0 ? path.replace(/\/[^/]*$/, "") : ""
    property bool dirty: false
    property string loadError: ""
    property var metadata: ({})

    // ---- kernel -------------------------------------------------------------
    property var kernels: []            // [{name, display, language}]
    property string kernelName: ""
    property string kernelState: ""     // "", starting, idle, busy, dead
    property string kernelMessage: ""
    property string language: "python"
    property string languageVersion: ""
    readonly property var kernel: kernels.find(k => k.name === kernelName) || null
    readonly property bool shellKernel: kernelName === "tessera-shell"
    readonly property bool kernelUp: shellKernel || kernelState === "idle" || kernelState === "busy"
    property var queue: []              // cells waiting for the kernel to come up

    function listKernels() {
        let ks = []
        try { ks = JSON.parse(controller.jupyterKernels()) } catch (e) {}
        if (term) ks.push({ name: "tessera-shell", display: "This pane's shell", language: "bash" })
        kernels = ks
    }
    function pickKernel(spec) {
        // The notebook's own kernel, else one for its language.
        const want = spec && spec.name ? spec.name : ""
        const lang = (spec && spec.language) || (metadata.language_info && metadata.language_info.name) || "python"
        let k = kernels.find(x => x.name === want)
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
        controller.jupyterStart(key, kernelName, folder)
    }
    function restartKernel() {
        controller.jupyterStop(key)
        for (let i = 0; i < cells.count; ++i) cells.setProperty(i, "running", false)
        queue = []
        kernelState = ""
        if (!shellKernel) startKernel()
    }
    function changeKernel(name) {
        if (name === kernelName) return
        controller.jupyterStop(key)
        kernelState = ""
        kernelName = name
        language = kernel ? kernel.language : language
        dirty = true
        for (let i = 0; i < cells.count; ++i) cells.setProperty(i, "running", false)
    }
    Connections {
        target: jv.controller
        function onJupyterEvent(key, json) {
            if (key !== jv.key) return
            let e
            try { e = JSON.parse(json) } catch (x) { return }
            jv.kernelEvent(e)
        }
    }
    function indexOf(cid) {
        for (let i = 0; i < cells.count; ++i) if (cells.get(i).cid === cid) return i
        return -1
    }
    function kernelEvent(e) {
        if (e.type === "status") {
            kernelState = e.state
            if (e.message !== undefined) kernelMessage = e.message
            if (e.state === "idle" && queue.length > 0) { const q = queue; queue = []; for (const c of q) execute(c) }
            if (e.state === "dead") { for (let i = 0; i < cells.count; ++i) cells.setProperty(i, "running", false); queue = [] }
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
                cells.setProperty(i, "running", false)
                if (e.count !== null && e.count !== undefined) cells.setProperty(i, "count", e.count)
                cells.setProperty(i, "ended", Date.now())
                cells.setProperty(i, "failed", e.status !== "ok")
                // (Stopped at an error: the cells waiting after it don't run.)
                if (e.status === "error") for (let j = 0; j < cells.count; ++j) if (cells.get(j).running && cells.get(j).started > cells.get(i).started) cells.setProperty(j, "running", false)
                dirty = true
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
        dirty = true
    }

    // ---- running cells --------------------------------------------------------
    function run(i) {
        const c = cells.get(i)
        if (!c) return
        if (c.type === "markdown") { cells.setProperty(i, "editing", false); return }
        if (c.type !== "code" || c.source.trim() === "") return
        cells.setProperty(i, "outputs", "[]")
        cells.setProperty(i, "running", true)
        cells.setProperty(i, "failed", false)
        cells.setProperty(i, "started", Date.now())
        cells.setProperty(i, "ended", 0)
        if (shellKernel) { shellRun(c.cid, c.source); return }
        if (kernelUp) execute(c.cid)
        else {
            queue = queue.concat([c.cid])
            if (kernelState === "" || kernelState === "dead") startKernel()
        }
    }
    function execute(cid) {
        const i = indexOf(cid)
        if (i < 0) return
        if (!controller.jupyterExecute(key, cid, cells.get(i).source)) {
            cells.setProperty(i, "running", false)
            addOutput(i, { output_type: "error", ename: "Not sent", evalue: "The kernel isn't reachable.", traceback: ["The kernel isn't reachable: restart it."] })
        }
    }
    function runAll() { for (let i = 0; i < cells.count; ++i) run(i) }
    function runAbove(n) { for (let i = 0; i < n; ++i) run(i) }
    function interrupt() {
        if (shellKernel) { if (term) term.sendText("\x03"); return }
        controller.jupyterInterrupt(key)
    }
    function clearOutputs() {
        for (let i = 0; i < cells.count; ++i) { cells.setProperty(i, "outputs", "[]"); cells.setProperty(i, "count", -1) }
        dirty = true
    }

    // This pane's shell as the kernel: each cell is typed into it, and its
    // output comes back from shell integration's marks (a cell of the pane's
    // own notebook).
    property var shellWaiting: []       // our cells, in the order sent
    property var shellCells: ({})       // the terminal's cell id -> ours
    property int shellCount: 0
    function shellRun(cid, code) {
        if (!term) return
        shellWaiting = shellWaiting.concat([cid])
        if (code.indexOf("\n") < 0) term.sendText(code + "\r")
        else { term.pasteText(code); term.sendText("\r") }
    }
    Connections {
        target: jv.shellKernel ? jv.term : null
        function onCellsChanged(id, done) {
            if (id === 0) return
            let mine = jv.shellCells[id]
            if (mine === undefined && !done && jv.shellWaiting.length > 0) {
                mine = jv.shellWaiting[0]
                jv.shellWaiting = jv.shellWaiting.slice(1)
                const m = Object.assign({}, jv.shellCells); m[id] = mine; jv.shellCells = m
            }
            if (mine === undefined || !done) return
            const i = jv.indexOf(mine)
            if (i < 0) return
            const text = jv.term.cellOutput(id, "text")
            let cs = []
            try { cs = JSON.parse(jv.term.cells()) } catch (e) {}
            const tc = cs.find(x => x.id === id)
            const outs = []
            if (text.trim() !== "") outs.push({ output_type: "stream", name: "stdout", text: text.endsWith("\n") ? text : text + "\n" })
            const img = jv.term.cellOutput(id, "image")
            if (img !== "") outs.push({ output_type: "display_data", data: { "image/png": img }, metadata: {} })
            cells.setProperty(i, "outputs", JSON.stringify(outs))
            cells.setProperty(i, "running", false)
            cells.setProperty(i, "count", ++jv.shellCount)
            cells.setProperty(i, "ended", Date.now())
            cells.setProperty(i, "failed", tc ? tc.status !== 0 : false)
            jv.dirty = true
        }
    }

    // ---- cells ------------------------------------------------------------------
    ListModel { id: cells }
    property int current: 0
    property int nextId: 1
    function newCell(type, source) {
        return { cid: "c" + (nextId++) + "-" + Math.random().toString(36).slice(2, 8), nbid: Math.random().toString(36).slice(2, 10),
                 type: type, source: source || "", outputs: "[]", count: -1, running: false, failed: false,
                 started: 0, ended: 0, editing: type !== "markdown" || (source || "") === "", meta: "{}", clearPending: false, hideOutput: false }
    }
    function insertCell(at, type, edit) {
        at = Math.max(0, Math.min(cells.count, at))
        cells.insert(at, newCell(type, ""))
        current = at
        dirty = true
        if (edit !== false) Qt.callLater(focusCell, at, true)
    }
    function deleteCell(i) {
        if (i < 0 || i >= cells.count) return
        undoCell = { at: i, cell: cells.get(i) && JSON.parse(JSON.stringify(cells.get(i))) }
        cells.remove(i)
        if (cells.count === 0) cells.append(newCell("code", ""))
        current = Math.min(i, cells.count - 1)
        dirty = true
        Qt.callLater(focusCell, current, false)
    }
    property var undoCell: null
    function undoDelete() {
        if (!undoCell) return
        cells.insert(undoCell.at, undoCell.cell)
        current = undoCell.at
        undoCell = null
        dirty = true
    }
    function moveCell(i, d) {
        const j = i + d
        if (j < 0 || j >= cells.count) return
        cells.move(i, j, 1)
        current = j
        dirty = true
    }
    function setType(i, type) {
        if (cells.get(i).type === type) return
        cells.setProperty(i, "type", type)
        cells.setProperty(i, "outputs", "[]")
        cells.setProperty(i, "count", -1)
        cells.setProperty(i, "editing", type !== "markdown")
        dirty = true
    }
    function focusCell(i, edit) {
        if (i < 0 || i >= cells.count) return
        current = i
        list.positionViewAtIndex(i, ListView.Contain)
        const it = list.itemAtIndex(i)
        if (edit) {
            if (cells.get(i).type === "markdown") cells.setProperty(i, "editing", true)
            if (it) Qt.callLater(() => { const x = list.itemAtIndex(i); if (x) x.focusEditor() })
        } else {
            list.forceActiveFocus()
        }
    }
    // Run, then go on (Shift+Enter): the next cell, or a new one at the end.
    function runAndAdvance(i) {
        run(i)
        if (i + 1 >= cells.count) insertCell(cells.count, "code", true)
        else focusCell(i + 1, cells.get(i + 1).type === "code")
    }

    // ---- reading and writing .ipynb ---------------------------------------------
    function joined(v) { return Array.isArray(v) ? v.join("") : (v === undefined || v === null ? "" : String(v)) }
    function lines(s) {
        // Jupyter's way: a list of lines, each keeping its "\n".
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
    function load() {
        cells.clear()
        loadError = ""
        listKernels()
        if (path === "") { cells.append(newCell("code", "")); pickKernel(null); dirty = false; return }
        const r = JSON.parse(controller.readTextFile(path))
        if (r.error !== undefined) {
            // A new notebook (nothing there yet), or one that can't be read.
            if (!/No such file/i.test(r.error)) loadError = r.error
            cells.append(newCell("code", ""))
            metadata = {}
            pickKernel(null)
            dirty = false
            return
        }
        let nb
        try { nb = JSON.parse(r.text) } catch (e) { loadError = "Not a notebook: " + e; cells.append(newCell("code", "")); return }
        metadata = nb.metadata || {}
        let maxCount = 0
        for (const c of nb.cells || []) {
            const x = newCell(c.cell_type === "markdown" ? "markdown" : c.cell_type === "raw" ? "raw" : "code", joined(c.source))
            if (c.id) x.nbid = c.id
            x.meta = JSON.stringify(c.metadata || {})
            x.editing = x.type !== "markdown"
            if (x.type === "code") {
                x.outputs = JSON.stringify((c.outputs || []).map(normalOutput))
                x.count = c.execution_count === null || c.execution_count === undefined ? -1 : c.execution_count
                maxCount = Math.max(maxCount, x.count)
                x.hideOutput = !!(c.metadata && c.metadata.jupyter && c.metadata.jupyter.outputs_hidden)
            } else if (c.attachments) {
                x.meta = JSON.stringify(Object.assign({}, c.metadata || {}, { "tessera.attachments": c.attachments }))
            }
            cells.append(x)
        }
        if (cells.count === 0) cells.append(newCell("code", ""))
        shellCount = maxCount
        pickKernel(metadata.kernelspec)
        dirty = false
        current = 0
    }
    // Keys sorted, as Jupyter writes them (smaller diffs).
    function sorted(v) {
        if (Array.isArray(v)) return v.map(sorted)
        if (v && typeof v === "object") { const o = {}; for (const k of Object.keys(v).sort()) o[k] = sorted(v[k]); return o }
        return v
    }
    function serialize() {
        const out = []
        for (let i = 0; i < cells.count; ++i) {
            const c = cells.get(i)
            let meta = {}
            try { meta = JSON.parse(c.meta) } catch (e) {}
            const att = meta["tessera.attachments"]
            delete meta["tessera.attachments"]
            const cell = { cell_type: c.type, id: c.nbid, metadata: meta, source: lines(c.source) }
            if (c.type === "code") {
                let outs = []
                try { outs = JSON.parse(c.outputs) } catch (e) {}
                cell.execution_count = c.count >= 0 ? c.count : null
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
        if (kernel && !shellKernel) md.kernelspec = { name: kernel.name, display_name: kernel.display, language: kernel.language }
        else if (shellKernel) md.kernelspec = { name: "bash", display_name: "Bash", language: "bash" }
        md.language_info = Object.assign({}, md.language_info || {}, { name: language })
        if (languageVersion !== "") md.language_info.version = languageVersion
        return JSON.stringify(sorted({ cells: out, metadata: md, nbformat: 4, nbformat_minor: 5 }), null, 1) + "\n"
    }
    function save() {
        if (path === "") return
        const err = controller.writeTextFile(path, serialize())
        if (err !== "") { saveError = err; return }
        saveError = ""
        dirty = false
    }
    property string saveError: ""
    Component.onCompleted: if (controller) load()
    // (Test hook: TESSERA_TEST_NB_RUN_ALL runs every cell once it's open.)
    Timer { interval: 1500; running: jv.controller !== null && jv.visible && jv.controller.testEnv("NB_RUN_ALL") !== ""; onTriggered: jv.runAll() }
    Timer { interval: 20000; running: jv.controller !== null && jv.visible && jv.controller.testEnv("NB_SAVE") !== ""; onTriggered: jv.save() }
    Timer { interval: 16000; running: jv.controller !== null && jv.visible && jv.controller.testEnv("NB_SCROLL_END") !== ""; onTriggered: list.positionViewAtEnd() }
    onPathChanged: if (controller) load()
    Component.onDestruction: if (controller) controller.jupyterStop(key)

    // ---- the colors outputs use ---------------------------------------------------
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
    function colored(text) {
        if (!controller) return ""
        return controller.ansiHtml(text).replace(/var\(--vt-palette-(\d+)\)/g, (m, n) => paletteColor(parseInt(n)))
    }
    function htmlPage(body, scripts) {
        const c = (x) => String(x)
        return "<!doctype html><html><head><meta charset='utf-8'><style>"
            + "html,body{margin:0;padding:0}body{background:" + c(Theme.bg) + ";color:" + c(Theme.text) + ";font:13px/1.5 'Noto Sans','Segoe UI',system-ui,sans-serif;padding:4px 2px;overflow-x:auto}"
            + "a{color:" + c(Theme.accent) + "}table{border-collapse:collapse;font-size:12px}th,td{border:1px solid " + c(Theme.divider) + ";padding:3px 8px;text-align:right}"
            + "th{background:" + c(Theme.field) + "}tr:nth-child(even) td{background:" + c(Theme.mix(Theme.bg, Theme.fg, 0.03)) + "}"
            + "code,pre{font-family:'" + fontFamily + "',monospace}pre{background:" + c(Theme.field) + ";padding:8px;border-radius:6px;overflow:auto}"
            + "img,svg{max-width:100%}"
            + "</style></head><body>" + body + "</body></html>"
    }
    readonly property var mimeOrder: ["application/vnd.jupyter.widget-view+json", "text/html", "image/svg+xml", "image/png", "image/jpeg", "image/gif", "text/markdown", "text/latex", "application/json", "text/plain"]
    function bestMime(data) {
        for (const m of mimeOrder) if (data[m] !== undefined && m !== "application/vnd.jupyter.widget-view+json") return m
        const ks = Object.keys(data)
        return ks.length ? ks[0] : ""
    }

    // ---- keys between cells (command mode) ---------------------------------------
    property bool dPending: false
    Timer { id: dTimer; interval: 700; onTriggered: jv.dPending = false }
    Shortcut { sequence: StandardKey.Save; enabled: jv.visible; context: Qt.WindowShortcut; onActivated: jv.save() }

    // ---- toolbar ------------------------------------------------------------------
    component Tool: Chip {
        base: Theme.panel
    }
    // Narrow: the toolbar says less.
    readonly property bool narrow: width < 980
    Rectangle {
        id: bar
        width: parent.width
        height: 34
        color: Theme.panel
        z: 2
        Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.divider }
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 10
            anchors.rightMargin: 8
            spacing: 6
            Text {
                text: "▣"
                color: Theme.accent
                font.pixelSize: 13
            }
            Text {
                text: jv.fileName + (jv.dirty ? " •" : "")
                color: Theme.text
                font.pixelSize: 12
                font.weight: Font.DemiBold
                elide: Text.ElideMiddle
                Layout.maximumWidth: jv.narrow ? 140 : 260
                renderType: Text.NativeRendering
                ToolTip.visible: nameHover.hovered
                ToolTip.text: jv.path === "" ? "Not saved yet" : jv.path
                ToolTip.delay: 400
                HoverHandler { id: nameHover }
            }
            Item { width: 6 }
            Tool { label: jv.narrow ? "▶▶" : "▶▶ Run All"; tip: "Run every cell, top to bottom"; onClicked: jv.runAll() }
            Tool { label: "■"; tip: "Interrupt the kernel"; enabled: jv.kernelState === "busy" || jv.shellKernel; onClicked: jv.interrupt() }
            Tool { label: jv.narrow ? "↻" : "↻ Restart"; tip: "Restart the kernel (its variables are lost)"; onClicked: jv.restartKernel() }
            Tool { label: jv.narrow ? "⌫" : "Clear Outputs"; tip: "Clear every cell's output"; onClicked: jv.clearOutputs() }
            Rectangle { width: 1; height: 16; color: Theme.divider }
            Tool { label: jv.narrow ? "+" : "+ Code"; tip: "Add a code cell below (B)"; onClicked: jv.insertCell(jv.current + 1, "code") }
            Tool { label: jv.narrow ? "+M↓" : "+ Markdown"; tip: "Add a Markdown cell below"; onClicked: jv.insertCell(jv.current + 1, "markdown") }
            Item { Layout.fillWidth: true }
            // Kernel: which, and how it's doing.
            Rectangle {
                width: 8; height: 8; radius: 4
                color: jv.shellKernel ? jv.ansi(2, "#98c379")
                     : jv.kernelState === "idle" ? jv.ansi(2, "#98c379")
                     : jv.kernelState === "busy" ? jv.ansi(3, "#e5c07b")
                     : jv.kernelState === "starting" ? Theme.accent
                     : jv.kernelState === "dead" ? jv.ansi(1, "#e06c75") : Theme.textDim
                SequentialAnimation on opacity {
                    running: jv.kernelState === "starting" || jv.kernelState === "busy"
                    loops: Animation.Infinite
                    NumberAnimation { to: 0.3; duration: 500 }
                    NumberAnimation { to: 1; duration: 500 }
                    onRunningChanged: if (!running) parent.opacity = 1
                }
            }
            Text {
                visible: !jv.narrow
                text: jv.shellKernel ? "ready" : jv.kernelState === "" ? "not started" : jv.kernelState
                color: Theme.textDim
                font.pixelSize: 11
            }
            SlimCombo {
                id: kernelBox
                implicitWidth: jv.narrow ? 150 : 210
                model: jv.kernels.length ? jv.kernels.map(k => k.display) : ["No kernels found"]
                currentIndex: Math.max(0, jv.kernels.findIndex(k => k.name === jv.kernelName))
                onActivated: (i) => { if (jv.kernels[i]) jv.changeKernel(jv.kernels[i].name) }
                onPressedChanged: if (pressed) jv.listKernels()
                ToolTip.visible: hovered && !popup.visible
                ToolTip.delay: 500
                ToolTip.text: "The kernel that runs this notebook's code"
            }
            Tool {
                label: jv.dirty ? "Save" : "Saved"
                tip: "Save the notebook (Ctrl+S)"
                enabled: jv.path !== ""
                onClicked: jv.save()
            }
        }
    }
    // Starting, failing: said under the toolbar.
    Rectangle {
        id: banner
        anchors.top: bar.bottom
        width: parent.width
        visible: text.text !== ""
        height: visible ? text.implicitHeight + 14 : 0
        z: 2
        color: jv.kernelState === "dead" || jv.loadError !== "" || jv.saveError !== "" ? Theme.mix(Theme.panel, jv.ansi(1, "#e06c75"), 0.18) : Theme.mix(Theme.panel, Theme.accent, 0.12)
        Text {
            id: text
            x: 12
            width: parent.width - 24 - (banner.actions.visible ? banner.actions.width + 8 : 0)
            anchors.verticalCenter: parent.verticalCenter
            wrapMode: Text.Wrap
            maximumLineCount: 8
            elide: Text.ElideRight
            color: Theme.text
            font.pixelSize: 12
            text: jv.loadError !== "" ? "Couldn't read " + jv.fileName + ": " + jv.loadError
                : jv.saveError !== "" ? "Couldn't save: " + jv.saveError
                : jv.kernelState === "dead" ? (jv.kernelMessage || "The kernel stopped.")
                : jv.kernelState === "starting" ? (jv.kernelName === "tessera-nix-python"
                      ? "Starting Python from Nix… the first time, Nix fetches it (a minute or two)."
                      : jv.kernelMessage || "Starting the kernel…")
                : jv.kernels.length === 0 ? "No Jupyter kernel found. Install one (pip install ipykernel), or pick This pane's shell."
                : ""
        }
        property alias actions: bannerActions
        Row {
            id: bannerActions
            visible: jv.kernelState === "dead"
            anchors.right: parent.right
            anchors.rightMargin: 10
            anchors.verticalCenter: parent.verticalCenter
            Chip { base: banner.color; label: "Restart"; onClicked: jv.restartKernel() }
        }
    }

    // ---- the cells -------------------------------------------------------------
    Rectangle { anchors.fill: parent; color: Theme.bg; z: -1 }
    ListView {
        id: list
        anchors.top: banner.bottom
        anchors.bottom: parent.bottom
        width: parent.width
        clip: true
        focus: true
        model: cells
        spacing: 2
        cacheBuffer: 2000
        interactive: false   // (a drag selects text; the wheel, touchpad and bar scroll)
        boundsBehavior: Flickable.StopAtBounds
        topMargin: 10
        bottomMargin: 80
        ScrollBar.vertical: ScrollBar {}
        MomentumWheel { flick: list }
        Keys.onPressed: (e) => {
            const i = jv.current
            const plain = !(e.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))
            if ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter) && (e.modifiers & Qt.ShiftModifier)) { jv.runAndAdvance(i); e.accepted = true }
            else if ((e.key === Qt.Key_Return || e.key === Qt.Key_Enter) && (e.modifiers & Qt.ControlModifier)) { jv.run(i); e.accepted = true }
            else if (e.key === Qt.Key_Return || e.key === Qt.Key_Enter) { jv.focusCell(i, true); e.accepted = true }
            else if ((e.key === Qt.Key_Up || e.key === Qt.Key_K) && plain) { jv.focusCell(Math.max(0, i - 1), false); e.accepted = true }
            else if ((e.key === Qt.Key_Down || e.key === Qt.Key_J) && plain) { jv.focusCell(Math.min(cells.count - 1, i + 1), false); e.accepted = true }
            else if (e.key === Qt.Key_A && plain) { jv.insertCell(i, "code"); e.accepted = true }
            else if (e.key === Qt.Key_B && plain) { jv.insertCell(i + 1, "code"); e.accepted = true }
            else if (e.key === Qt.Key_M && plain) { jv.setType(i, "markdown"); e.accepted = true }
            else if (e.key === Qt.Key_Y && plain) { jv.setType(i, "code"); e.accepted = true }
            else if (e.key === Qt.Key_Z && plain) { jv.undoDelete(); e.accepted = true }
            else if (e.key === Qt.Key_O && plain) { cells.setProperty(i, "hideOutput", !cells.get(i).hideOutput); e.accepted = true }
            else if (e.key === Qt.Key_D && plain) {
                if (jv.dPending) { jv.dPending = false; jv.deleteCell(i) } else { jv.dPending = true; dTimer.restart() }
                e.accepted = true
            } else if (e.key === Qt.Key_Delete && plain) { jv.deleteCell(i); e.accepted = true }
        }

        delegate: Item {
            id: cell
            required property int index
            required property string cid
            required property string type
            required property string source
            required property string outputs
            required property int count
            required property bool running
            required property bool failed
            required property double started
            required property double ended
            required property bool editing
            required property bool hideOutput
            readonly property bool selected: jv.current === index
            readonly property var outs: { try { return JSON.parse(outputs) } catch (e) { return [] } }
            width: list.width
            height: col.height + 8
            function focusEditor() { if (editorLoader.item) editorLoader.item.focusIn() }

            HoverHandler { id: cellHover }
            TapHandler { onTapped: { jv.current = cell.index; if (!(editorLoader.item && editorLoader.item.hasFocus)) list.forceActiveFocus() } }

            // The selected cell's mark, on the left.
            Rectangle {
                x: 6
                y: 4
                width: 3
                height: col.height
                radius: 1.5
                visible: cell.selected
                color: list.activeFocus ? Theme.accent : Theme.mix(Theme.accent, Theme.bg, 0.4)
            }
            Column {
                id: col
                x: 14
                y: 4
                width: parent.width - 28
                spacing: 4
                Row {
                    width: parent.width
                    spacing: 6
                    // Gutter: run button and [count].
                    Item {
                        width: 40
                        height: Math.max(28, editorBox.height)
                        Rectangle {
                            id: runBtn
                            visible: cell.type === "code"
                            width: 24; height: 24; radius: 12
                            x: 8
                            y: 2
                            color: runTap.containsMouse ? Theme.mix(Theme.panel, Theme.accent, 0.3) : "transparent"
                            Text {
                                anchors.centerIn: parent
                                visible: !cell.running
                                text: "▶"
                                color: cell.selected || cellHover.hovered ? Theme.accent : Theme.textDim
                                font.pixelSize: 11
                            }
                            // Running: a turning arc.
                            Canvas {
                                anchors.fill: parent
                                anchors.margins: 5
                                visible: cell.running
                                property color ink: Theme.accent
                                onPaint: {
                                    const c = getContext("2d"); c.reset()
                                    c.strokeStyle = ink; c.lineWidth = 2; c.lineCap = "round"
                                    c.beginPath(); c.arc(width / 2, height / 2, width / 2 - 1, 0, Math.PI * 1.4); c.stroke()
                                }
                                RotationAnimator on rotation { from: 0; to: 360; duration: 900; loops: Animation.Infinite; running: cell.running }
                            }
                            MouseArea {
                                id: runTap
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: { jv.current = cell.index; cell.running ? jv.interrupt() : jv.run(cell.index) }
                                ToolTip.visible: containsMouse
                                ToolTip.delay: 500
                                ToolTip.text: cell.running ? "Interrupt" : "Run (Ctrl+Enter)"
                            }
                        }
                        Text {
                            anchors.horizontalCenter: runBtn.horizontalCenter
                            y: 28
                            visible: cell.type === "code"
                            text: cell.running ? "[*]" : cell.count >= 0 ? "[" + cell.count + "]" : "[ ]"
                            color: Theme.textDim
                            font.family: jv.fontFamily
                            font.pixelSize: 10
                        }
                    }
                    // The source: an editor (code, or Markdown being written) or
                    // the Markdown as text.
                    Rectangle {
                        id: editorBox
                        width: parent.width - 46
                        height: cell.type === "markdown" && !cell.editing ? mdText.implicitHeight + 12 : editorLoader.height + 2
                        radius: 6
                        color: cell.type === "markdown" && !cell.editing ? "transparent" : Theme.field
                        border.color: editorLoader.item && editorLoader.item.hasFocus ? Theme.accent
                                    : cell.selected ? Theme.mix(Theme.divider, Theme.accent, 0.4)
                                    : cell.type === "markdown" && !cell.editing ? "transparent" : Theme.divider
                        Loader {
                            id: editorLoader
                            active: cell.type !== "markdown" || cell.editing
                            width: parent.width - 2
                            x: 1; y: 1
                            sourceComponent: cellEditor
                            onLoaded: { item.cellIndex = Qt.binding(() => cell.index); item.lang = Qt.binding(() => cell.type === "markdown" ? "markdown" : cell.type === "raw" ? "" : jv.language) }
                        }
                        Text {
                            id: mdText
                            visible: cell.type === "markdown" && !cell.editing
                            x: 8; y: 6
                            width: parent.width - 16
                            textFormat: Text.MarkdownText
                            text: cell.source.trim() === "" ? "*Empty Markdown cell (double-click to write)*" : cell.source
                            baseUrl: jv.folder !== "" ? "file://" + jv.folder + "/" : ""
                            wrapMode: Text.Wrap
                            color: Theme.text
                            linkColor: Theme.accent
                            font.pixelSize: 14
                            renderType: Text.NativeRendering
                            onLinkActivated: (l) => Qt.openUrlExternally(l)
                            TapHandler { onDoubleTapped: jv.focusCell(cell.index, true) }
                            HoverHandler { cursorShape: mdText.hoveredLink !== "" ? Qt.PointingHandCursor : Qt.ArrowCursor }
                        }
                        // On hover: what can be done to the cell.
                        Row {
                            anchors.right: parent.right
                            anchors.rightMargin: 4
                            y: -10
                            z: 3
                            spacing: 2
                            visible: cellHover.hovered || cell.selected
                            opacity: cellHover.hovered ? 1 : 0.0
                            Behavior on opacity { NumberAnimation { duration: 120 } }
                            Chip { base: Theme.panel; label: cell.type === "markdown" ? "M↓" : "{ }"; tip: cell.type === "markdown" ? "Make it a code cell (Y)" : "Make it Markdown (M)"
                                   onClicked: jv.setType(cell.index, cell.type === "markdown" ? "code" : "markdown") }
                            Chip { base: Theme.panel; label: "▲"; tip: "Move up"; onClicked: jv.moveCell(cell.index, -1) }
                            Chip { base: Theme.panel; label: "▼"; tip: "Move down"; onClicked: jv.moveCell(cell.index, 1) }
                            Chip { base: Theme.panel; label: "▶↑"; tip: "Run every cell above this one"; visible: cell.type === "code"; onClicked: jv.runAbove(cell.index) }
                            Chip { base: Theme.panel; label: "✕"; tip: "Delete (D D; Z brings it back)"; onClicked: jv.deleteCell(cell.index) }
                        }
                    }
                }
                // Outputs.
                Item {
                    id: outArea
                    visible: cell.type === "code" && (cell.outs.length > 0 || cell.ended > 0)
                    x: 46
                    width: parent.width - 46
                    height: visible ? (cell.hideOutput ? 22 : outCol.height) : 0
                    Text {
                        visible: cell.hideOutput
                        text: "… output hidden (O shows it)"
                        color: Theme.textDim
                        font.pixelSize: 11
                        TapHandler { onTapped: cells.setProperty(cell.index, "hideOutput", false) }
                    }
                    Column {
                        id: outCol
                        visible: !cell.hideOutput
                        width: parent.width
                        spacing: 4
                        Repeater {
                            model: cell.outs
                            delegate: Loader {
                                required property var modelData
                                width: outCol.width
                                property var out: modelData
                                sourceComponent: {
                                    const o = modelData
                                    if (o.output_type === "stream" || o.output_type === "error") return textOut
                                    const m = jv.bestMime(o.data || {})
                                    if (m === "image/png" || m === "image/jpeg" || m === "image/gif") return imageOut
                                    if (m === "text/html" || m === "image/svg+xml") return htmlOut
                                    if (m === "text/markdown") return markdownOut
                                    return textOut
                                }
                            }
                        }
                        // How long it took, and how it went.
                        Text {
                            visible: cell.ended > 0
                            text: (cell.failed ? "✘ " : "✓ ") + jv.seconds(cell.ended - cell.started)
                            color: cell.failed ? jv.ansi(1, "#e06c75") : Theme.textDim
                            font.pixelSize: 10
                        }
                    }
                    // Hide/show by clicking the strip on its left.
                    Rectangle {
                        x: -10
                        width: 4
                        height: parent.height
                        radius: 2
                        color: stripHover.hovered ? Theme.accent : "transparent"
                        HoverHandler { id: stripHover; cursorShape: Qt.PointingHandCursor }
                        TapHandler { onTapped: cells.setProperty(cell.index, "hideOutput", !cell.hideOutput) }
                    }
                }
            }
            // Between cells, on hover: add one.
            Row {
                anchors.horizontalCenter: parent.horizontalCenter
                y: parent.height - 9
                z: 4
                spacing: 6
                visible: addHover.hovered
                Chip { base: Theme.bg; label: "+ Code"; onClicked: jv.insertCell(cell.index + 1, "code") }
                Chip { base: Theme.bg; label: "+ Markdown"; onClicked: jv.insertCell(cell.index + 1, "markdown") }
            }
            Item {
                width: parent.width
                height: 14
                y: parent.height - 12
                HoverHandler { id: addHover }
            }
        }
        footer: Item {
            width: list.width
            height: 44
            Row {
                anchors.centerIn: parent
                spacing: 8
                Chip { base: Theme.bg; label: "+ Code"; tip: "Add a code cell at the end"; onClicked: jv.insertCell(cells.count, "code") }
                Chip { base: Theme.bg; label: "+ Markdown"; tip: "Add a Markdown cell at the end"; onClicked: jv.insertCell(cells.count, "markdown") }
            }
        }
    }
    function seconds(ms) { return ms < 1000 ? Math.max(0, ms) + " ms" : ms < 60000 ? (ms / 1000).toFixed(1) + " s" : Math.floor(ms / 60000) + " m " + Math.round((ms % 60000) / 1000) + " s" }

    // ---- a cell's editor ---------------------------------------------------------
    Component {
        id: cellEditor
        Item {
            id: ed
            property int cellIndex: -1
            property string lang: "python"
            readonly property bool hasFocus: edit.activeFocus
            implicitHeight: edit.implicitHeight + 14
            height: implicitHeight
            function focusIn() { edit.forceActiveFocus(); if (edit.cursorPosition === 0 && edit.length > 0 && cells.get(cellIndex).type === "markdown") edit.cursorPosition = edit.length }
            TextEdit {
                id: edit
                x: 10; y: 7
                width: parent.width - 20
                text: cells.get(ed.cellIndex) ? cells.get(ed.cellIndex).source : ""
                font.family: jv.fontFamily
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
                    jv.dirty = true
                    hlTimer.restart()
                    comp.visible = false
                }
                onActiveFocusChanged: if (activeFocus) jv.current = ed.cellIndex
                Keys.onPressed: (e) => ed.key(e)
                Text {
                    visible: edit.text === "" && edit.activeFocus
                    text: cells.get(ed.cellIndex) && cells.get(ed.cellIndex).type === "markdown" ? "Markdown — Shift+Enter shows it" : "Code — Shift+Enter runs it"
                    color: Theme.textDim
                    font: edit.font
                }
            }
            SyntaxHighlighter {
                id: hl
                document: edit.textDocument
                errorColor: "transparent"
                colors: ({
                    keyword: jv.ansi(5, "#c678dd"), string: jv.ansi(2, "#98c379"), comment: Theme.textDim,
                    number: jv.ansi(3, "#d19a66"), constant: jv.ansi(3, "#d19a66"), function: jv.ansi(4, "#61afef"),
                    operator: jv.ansi(6, "#56b6c2"), property: jv.ansi(6, "#56b6c2"), variable: Theme.text,
                    embedded: jv.ansi(13, "#c678dd"), type: jv.ansi(3, "#e5c07b"), constructor: jv.ansi(3, "#e5c07b"),
                    punctuation: Theme.textDim, escape: jv.ansi(6, "#56b6c2"), attribute: jv.ansi(3, "#e5c07b"),
                    tag: jv.ansi(1, "#e06c75")
                })
            }
            Timer { id: hlTimer; interval: 120; onTriggered: ed.highlight() }
            Component.onCompleted: highlight()
            onLangChanged: highlight()
            function highlight() {
                if (!jv.controller || lang === "") return
                try {
                    const r = JSON.parse(jv.controller.syntax(lang, edit.text))
                    r.errors = []
                    hl.analysis = JSON.stringify(r)
                } catch (e) {}
            }
            function indentOf(line) { const m = line.match(/^[ \t]*/); return m ? m[0] : "" }
            function key(e) {
                const ctrl = e.modifiers & Qt.ControlModifier, shift = e.modifiers & Qt.ShiftModifier, alt = e.modifiers & Qt.AltModifier
                const enter = e.key === Qt.Key_Return || e.key === Qt.Key_Enter
                if (comp.visible) {
                    if (e.key === Qt.Key_Down) { comp.pick = Math.min(comp.items.length - 1, comp.pick + 1); e.accepted = true; return }
                    if (e.key === Qt.Key_Up) { comp.pick = Math.max(0, comp.pick - 1); e.accepted = true; return }
                    if (enter || e.key === Qt.Key_Tab) { comp.accept(); e.accepted = true; return }
                    if (e.key === Qt.Key_Escape) { comp.visible = false; e.accepted = true; return }
                }
                if (enter && shift) { jv.runAndAdvance(cellIndex); e.accepted = true }
                else if (enter && ctrl) { jv.run(cellIndex); if (cells.get(cellIndex).type === "markdown") list.forceActiveFocus(); e.accepted = true }
                else if (enter && alt) { jv.run(cellIndex); jv.insertCell(cellIndex + 1, "code"); e.accepted = true }
                else if (enter) {
                    // A new line, indented like this one (more after a ":").
                    const before = edit.text.slice(0, edit.cursorPosition)
                    const line = before.slice(before.lastIndexOf("\n") + 1)
                    let ind = indentOf(line)
                    if (lang === "python" && /:\s*$/.test(line)) ind += "    "
                    edit.remove(edit.selectionStart, edit.selectionEnd)
                    edit.insert(edit.cursorPosition, "\n" + ind)
                    e.accepted = true
                } else if (e.key === Qt.Key_Escape) { jv.current = cellIndex; if (cells.get(cellIndex).type === "markdown") cells.setProperty(cellIndex, "editing", false); list.forceActiveFocus(); e.accepted = true }
                else if (e.key === Qt.Key_Tab && !ctrl) {
                    const before = edit.text.slice(0, edit.cursorPosition)
                    if (edit.selectedText === "" && /[\w.\]\)'"]$/.test(before) && !jv.shellKernel && jv.kernelUp) {
                        comp.ask()
                    } else {
                        edit.insert(edit.cursorPosition, "    ")
                    }
                    e.accepted = true
                } else if (e.key === Qt.Key_Backtab) {
                    // Dedent the line.
                    const p = edit.cursorPosition, start = edit.text.lastIndexOf("\n", p - 1) + 1
                    const n = edit.text.slice(start, start + 4).match(/^ {1,4}/)
                    if (n) edit.remove(start, start + n[0].length)
                    e.accepted = true
                } else if (e.key === Qt.Key_Up && !shift && edit.text.lastIndexOf("\n", edit.cursorPosition - 1) < 0 && cellIndex > 0) {
                    jv.focusCell(cellIndex - 1, true); e.accepted = true
                } else if (e.key === Qt.Key_Down && !shift && edit.text.indexOf("\n", edit.cursorPosition) < 0 && cellIndex + 1 < cells.count) {
                    jv.focusCell(cellIndex + 1, true); e.accepted = true
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
                    req = jv.key + "-comp-" + Math.random().toString(36).slice(2)
                    jv.controller.jupyterComplete(jv.key, req, edit.text, edit.cursorPosition)
                }
                function accept() {
                    const m = items[pick]
                    visible = false
                    if (!m) return
                    // (Kernel positions count code points; close enough for completions.)
                    edit.remove(start, Math.max(start, edit.cursorPosition))
                    edit.insert(start, m)
                }
                Connections {
                    target: jv.controller
                    function onJupyterEvent(key, json) {
                        if (key !== jv.key || json.indexOf(comp.req) < 0) return
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
                        Text {
                            x: 6
                            anchors.verticalCenter: parent.verticalCenter
                            text: modelData
                            color: Theme.text
                            font.family: jv.fontFamily
                            font.pointSize: 9
                        }
                        TapHandler { onTapped: { comp.pick = index; comp.accept() } }
                    }
                }
            }
        }
    }

    // ---- outputs -----------------------------------------------------------------
    Component {
        id: textOut
        // stdout/stderr, a traceback, a value's text: with its colors.
        Rectangle {
            id: tbox
            readonly property var o: parent ? parent.out : ({})
            readonly property bool bad: o.output_type === "error" || (o.output_type === "stream" && o.name === "stderr")
            implicitHeight: Math.min(tv.implicitHeight + 8, 600)
            height: implicitHeight
            color: bad ? Theme.mix(Theme.bg, jv.ansi(1, "#e06c75"), 0.07) : "transparent"
            radius: 4
            Flickable {
                id: tf
                interactive: false   // (a drag selects text)
                anchors.fill: parent
                anchors.margins: 4
                contentWidth: Math.max(width, tv.implicitWidth)
                contentHeight: tv.implicitHeight
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar { policy: tf.contentHeight > tf.height ? ScrollBar.AlwaysOn : ScrollBar.AsNeeded }
                ScrollBar.horizontal: ScrollBar {}
                MomentumWheel { flick: tf }
                TextEdit {
                    id: tv
                    BlockSelect { }
                    readOnly: true
                    selectByMouse: true
                    selectionColor: Theme.mix(Theme.bg, Theme.accent, 0.45)
                    selectedTextColor: Theme.text
                    textFormat: TextEdit.RichText
                    color: Theme.text
                    font.family: jv.fontFamily
                    font.pointSize: 10
                    text: {
                        const o = tbox.o
                        if (!o) return ""
                        if (o.output_type === "stream") return jv.colored(o.text || "")
                        if (o.output_type === "error") return jv.colored((o.traceback || []).join("\n") || (o.ename + ": " + o.evalue))
                        const d = o.data || {}
                        const t = d["text/plain"] !== undefined ? d["text/plain"]
                                : d["application/json"] !== undefined ? JSON.stringify(d["application/json"], null, 2)
                                : d["text/latex"] !== undefined ? d["text/latex"]
                                : d["application/vnd.jupyter.widget-view+json"] !== undefined ? "(an interactive widget: not shown here)"
                                : "(" + Object.keys(d).join(", ") + ")"
                        return jv.colored(t)
                    }
                }
            }
        }
    }
    Component {
        id: imageOut
        Item {
            readonly property var o: parent ? parent.out : ({})
            readonly property string mime: jv.bestMime(o.data || {})
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
                const body = d["text/html"] !== undefined ? d["text/html"] : d["image/svg+xml"]
                loadHtml(jv.htmlPage(body), "about:blank")
            }
            onLoadingChanged: (r) => {
                if (r.status === WebEngineView.LoadSucceededStatus) measure.start()
            }
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
        Text {
            readonly property var o: parent ? parent.out : ({})
            textFormat: Text.MarkdownText
            text: o.data ? o.data["text/markdown"] : ""
            wrapMode: Text.Wrap
            color: Theme.text
            linkColor: Theme.accent
            font.pixelSize: 13
            renderType: Text.NativeRendering
            onLinkActivated: (l) => Qt.openUrlExternally(l)
        }
    }
}
