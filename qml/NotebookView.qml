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

    // The notebook's kernels, in its host's tesserad ("host//nb-..." for
    // another host's). A pane's notebook keeps its key with it, so its
    // kernels (run on while the window was closed) are found again.
    property string sshHost: ""
    property string keyBase: "nb-" + Math.random().toString(36).slice(2)
    // A notebook file's kernels are known by the file (they run on while the
    // window is closed, and are found when it opens again).
    function fileKey(p) { let h = 5381; for (let i = 0; i < p.length; ++i) h = ((h * 33) ^ p.charCodeAt(i)) >>> 0; return "nb-f-" + h.toString(36) }
    property bool quitting: false       // the app is quitting: kernels stay
    property bool keepKernels: false    // its window is closing: kernels stay
    readonly property string key: (sshHost !== "" ? sshHost + "//" : "") + keyBase
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
    // tessh's indentation (as in the pane's editor): the lines that take a
    // block, and where an else, elif, except or finally goes.
    readonly property var tesshBlocks: ["def", "class", "if", "elif", "else", "while", "for", "try", "except", "except*", "finally", "with", "match", "case", "defmacro", "quasi", "do", "async", "fn", "lambda"]
    readonly property var tesshOpeners: ({ "if": ["elif", "else"], "elif": ["elif", "else"], "for": ["else"], "while": ["else"], "try": ["except", "finally"], "except": ["except", "else", "finally"], "else": ["finally"] })
    function tesshHead(line) {
        const w = line.trim().split(/\s+/)
        let h = w[0] || ""
        if (w.length > 2 && /^(=|\+=|-=)$/.test(w[1])) h = w[2]
        if (h === "async" && w.length > 1) h = w[1]
        return h
    }
    function tesshOpens(line) {
        const t = line.trim(), w = t.split(/\s+/), h = tesshHead(line)
        if (t === "" || t[0] === "#") return false
        if (/[([{]$/.test(t)) return true
        if ((h === "def" || h === "class") && w.indexOf("=") > 0) return false
        if ((h === "fn" || h === "lambda") && w.length > 2) return false
        if ((h === "quasi" || h === "do") && w.length > 1) return false
        if (h === "if" && w.indexOf("then") > 0) return false
        return tesshBlocks.indexOf(h) >= 0
    }
    function tesshAlign(above, word, ind) {
        const lines = above.split("\n")
        for (let i = lines.length - 1; i >= 0; i--) {
            const l = lines[i]
            if (l.trim() === "") continue
            const li = l.length - l.replace(/^ +/, "").length
            if (li > ind) continue
            const ok = tesshOpeners[tesshHead(l)]
            if (ok && ok.indexOf(word) >= 0) return li
            if (li < ind && li === 0) break
        }
        return ind
    }
    property string kernelName: "tessera-shell"
    // What a pane's notebook (and a new notebook) runs cells in until one is
    // picked in its own list (Preferences › Notebook kernel; settings'
    // notebook_kernel): a kernel's name ("tessh", "python3",
    // "tessera-sh-zsh"), a language ("python", "fish") or "tessera-shell",
    // the pane's own shell. One it can't find: the pane's shell.
    property string defaultKernel: "tessh"
    // (Picked in this notebook's list: kept, whatever the default.)
    property bool kernelPicked: false
    function defaultKernelName() {
        const v = defaultKernel.trim()
        const shell = view ? "tessera-shell" : (kernels.length ? kernels[0].name : "")
        if (v === "" || v === "tessera-shell" || v === "shell") return shell
        const k = kernels.find(x => x.name === v) || langSpec(v)
        return k ? k.name : shell
    }
    onDefaultKernelChanged: {
        if (!loaded || kernelPicked) return
        const n = defaultKernelName()
        if (n !== kernelName) { setKernel(n); touch() }
    }
    // (The default kernel only in the list later (tesserad just started,
    // another host's kernels found): a notebook nothing has run in yet takes it.)
    function takeDefault() {
        if (!loaded || kernelPicked || path !== "" || kernelState !== "") return
        const n = defaultKernelName()
        if (n !== "" && n !== kernelName) setKernel(n)
    }
    onKernelsChanged: takeDefault()
    onViewChanged: takeDefault()
    Timer { interval: 2000; repeat: true; running: nb.visible && nb.controller !== null && nb.kernels.length === 0; onTriggered: nb.listKernels() }
    property string kernelState: ""     // "", starting, idle, busy, dead
    property string kernelMessage: ""
    property string language: "bash"
    property string languageVersion: ""
    readonly property var kernel: kernels.find(k => k.name === kernelName) || null
    readonly property bool shellKernel: kernelName === "tessera-shell"
    // (A language with no Jupyter kernel: each cell runs once, as a program,
    // in the pane's shell, as the input editor runs it.)
    readonly property bool oneShot: kernelName.startsWith("tessera-run:")
    readonly property bool viaShell: shellKernel || oneShot
    readonly property bool kernelUp: viaShell || kernelState === "idle" || kernelState === "busy"
    readonly property string codeLang: shellKernel ? "bash" : language
    // (What runs a cell with no language of its own: the pane's shell by its
    // name, so bash from a zsh pane goes to bash's kernel, not to zsh.)
    readonly property string runsIn: shellKernel ? (shellName || "bash") : language
    property var queue: []              // cells waiting for the kernel to come up

    // (Asked in the background; `done` runs once the list is in.)
    property int kernelsSeq: 0
    property int kernelsApplied: 0
    function listKernels(done) {
        const s = ++kernelsSeq, h = sshHost
        const apply = (json) => {
            // (An older answer than one shown, or another host's: not shown.)
            if (s > kernelsApplied && h === sshHost) {
                kernelsApplied = s
                let ks = []
                if (view) ks.push({ name: "tessera-shell", display: "This pane's shell", language: "bash" })
                try { ks = ks.concat(JSON.parse(json)) } catch (e) {}
                // (Then every language Tessera can run code in without a kernel.)
                let run = []
                try { run = JSON.parse(controller.runnableLanguages()) } catch (e) {}
                const more = run.filter(r => !ks.some(k => k.name !== "tessera-shell" && sameLang(k.language, r[0])))
                    .map(r => ({ name: "tessera-run:" + r[0], display: r[1] + " (each cell runs once)", language: r[0], oneShot: true }))
                    .sort((a, b) => a.display.toLowerCase() < b.display.toLowerCase() ? -1 : 1)
                kernels = ks.concat(more)
            }
            if (done) done()
        }
        if (!controller) { apply(""); return }
        Async.call(controller, k => controller.jupyterKernels(k, h), apply)
    }
    function pickKernel(spec) {
        // The notebook's own kernel, else one for its language; a shell
        // notebook (or none said) runs in the pane's shell.
        const want = spec && spec.name ? spec.name : ""
        const lang = (spec && spec.language) || (metadata.language_info && metadata.language_info.name) || ""
        if (want === "" && lang === "") {
            kernelName = defaultKernelName()
            language = kernel ? kernel.language : "bash"
            return
        }
        if (want === "bash" || lang === "bash" || want === "tessera-shell") {
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
        if (!controller || kernelName === "" || viaShell) return
        kernelState = "starting"
        kernelMessage = ""
        controller.jupyterStart(key, kernelName, folder, shellPid, storeKey)
    }
    function stopRunning() {
        // (Background jobs go on: they're not the kernel's.)
        for (let i = 0; i < cells.count; ++i) if (cells.get(i).running && cells.get(i).tid <= 0 && cells.get(i).job <= 0) cells.setProperty(i, "running", false)
        queue = []
    }
    function restartKernel() {
        if (viaShell) return
        controller.jupyterStop(key)
        stopRunning()
        kernelState = ""
        startKernel()
    }
    function changeKernel(name) {
        kernelPicked = true
        setKernel(name)
    }
    function setKernel(name) {
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
        function onKernelsReady(host) {
            if (host !== nb.sshHost) return
            nb.listKernels()
        }
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
        // (A tessh program to run in the terminal: the event's tty, or, through
        // an older tesserad that passes only the prompt on, in the prompt.)
        if (e.type === "input") {
            let tty = e.tty || null
            const M = "\x1b]tessera-tty;", p = e.prompt || ""
            if (!tty && p.startsWith(M)) { try { tty = JSON.parse(p.substring(M.length)) } catch (x) {} }
            if (tty) { runTty(e.cell, tty, fromKey || key); return }
        }
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
            else if (e.type === "exec") {
                // (A program a tessh cell runs, and what with: its details.)
                let xs = []
                try { xs = JSON.parse(cells.get(i).execs) } catch (x) {}
                cells.setProperty(i, "execs", JSON.stringify(xs.concat([e.exec]).slice(-50)))
            }
            else if (e.type === "done") {
                if (asking[e.cell]) setAsking(e.cell, null)
                cells.setProperty(i, "running", false)
                if (e.count !== null && e.count !== undefined) cells.setProperty(i, "count", e.count)
                cells.setProperty(i, "ended", Date.now())
                cells.setProperty(i, "failed", e.status !== "ok")
                if (e.cwd) cells.setProperty(i, "cwd", e.cwd)
                // (A shell's kernel: its folder and exports go back to the
                // pane's shell, so the next shell carries on from there.)
                const L = cells.get(i).lang
                if (L !== "" && ["bash", "zsh", "xonsh", "powershell", "sh"].indexOf(L) >= 0) controller.jupyterExport(langKey(L), shellPid)
                // (Stopped at an error: the cells waiting after it don't run.)
                if (e.status === "error") for (let j = 0; j < cells.count; ++j) if (cells.get(j).running && cells.get(j).tid <= 0 && cells.get(j).job <= 0 && cells.get(j).started > cells.get(i).started) cells.setProperty(j, "running", false)
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
    ListModel { id: cells; onDataChanged: Qt.callLater(nb.updateJobsRunning); onCountChanged: Qt.callLater(nb.updateJobsRunning) }
    property int current: 0
    property int nextId: 1
    property var kinds: ({})            // cid -> chosen view (the dropdown)
    property var streamViews: ({})      // cid -> "all", "stdout" or "stderr" (a card's output)
    function setStreamView(cid, v) { const m = Object.assign({}, streamViews); m[cid] = v; streamViews = m }
    property var fileViews: ({})        // cid -> an ls card's own view, sort, ... (FileBrowser)
    property var aiKinds: ({})          // cid -> what the AI made of it
    property var heights: ({})          // cid -> output height the user dragged to
    // cid -> the output's last natural height, changed in place (no
    // notifications): a card made again (scrolled back into the list's
    // buffer) starts at it while its output loads in the background,
    // instead of small and then growing. (Small-then-growing in a narrow
    // pane pushed other cards out of the buffer, which were then made
    // again small, and so on without end.)
    property var seenH: ({})
    // The tessh guide, in a tab of its own (Main's openGuide).
    function openGuide() { Window.window.openGuide() }
    property string expandedId: ""      // the cell filling the window, if any
    Connections { target: cells; function onCountChanged() { if (nb.expandedId !== "" && nb.indexOf(nb.expandedId) < 0) nb.expandedId = "" } }
    property int execCount: 0
    function setHeight(cid, h) { const m = Object.assign({}, heights); if (h > 0) m[cid] = h; else delete m[cid]; heights = m; touch() }
    function newCell(type, source) {
        return { cid: "c" + (nextId++) + "-" + Math.random().toString(36).slice(2, 8), nbid: Math.random().toString(36).slice(2, 10),
                 type: type, source: source || "", tid: 0, mine: false, outputs: "[]", count: -1, running: false, failed: false,
                 started: 0, ended: 0, cwd: "", status: 0, tkind: "", tsize: 0,
                 editing: false, meta: "{}", clearPending: false, hideOutput: false, lang: "", ranIn: "", viewPath: "", viewLang: "", execs: "[]", job: 0, jobAt: 0 }
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
    function run(i, bg) {
        const c = cells.get(i)
        if (!c) return
        if (c.type === "markdown") { if (c.source.trim() !== "") cells.setProperty(i, "editing", false); return }
        if (c.type !== "code" || c.source.trim() === "") return
        // (A file card: read the file again.)
        if (c.viewPath !== "") { fileRev++; return }
        if (c.tid > 0) retire(c.tid)
        cells.setProperty(i, "tid", 0)
        cells.setProperty(i, "outputs", "[]")
        cells.setProperty(i, "execs", "[]")
        cells.setProperty(i, "job", 0)
        cells.setProperty(i, "jobAt", 0)
        cells.setProperty(i, "running", true)
        cells.setProperty(i, "failed", false)
        cells.setProperty(i, "status", 0)
        cells.setProperty(i, "started", Date.now())
        cells.setProperty(i, "ended", 0)
        cells.setProperty(i, "mine", true)
        cells.setProperty(i, "editing", false)
        // (Where it runs: the pane's folder, which the kernel is brought to.)
        if (c.tid <= 0) cells.setProperty(i, "cwd", shellDir)
        // (What runs it, kept with it: the shell or kernel then, not now.)
        cells.setProperty(i, "ranIn", c.lang !== "" && !sameLang(c.lang, runsIn) ? ((langKernels[c.lang] && langKernels[c.lang].display) || (langSpec(c.lang) || {}).display || c.lang)
                                     : shellKernel ? (shellName || "shell") : (kernel ? kernel.display : kernelName))
        touch()
        // (In the background: a line ending in &, or asked for. It runs on its
        // own, in tesserad, and the notebook carries on.)
        const bl = bgLang(c)
        if (bl !== "" && (bg || isBgLine(c.source))) { runJob(i, c.source.trim().replace(/\s*&$/, ""), bl); return }
        // (A cell in another language: that language's kernel.)
        if (c.lang !== "" && !sameLang(c.lang, runsIn)) { runLang(c.cid, c.lang); return }
        if (shellKernel) { shellRun(c.cid, c.source); return }
        if (oneShot) { runOnce(c.cid, language, c.source); return }
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
        Async.call(controller, k => controller.jupyterExecute(k, key, cid, cells.get(i).source), ok => {
            const j = indexOf(cid)
            if (ok === "true" || j < 0) return
            cells.setProperty(j, "running", false)
            addOutput(j, { output_type: "error", ename: "Not sent", evalue: "The kernel isn't reachable.", traceback: ["The kernel isn't reachable: restart it."] })
        })
    }
    // ---- background jobs (src/jobs.rs, in tesserad) ----
    // A cell run in the background: its card shows the job's output as it
    // comes, its pid and status, and Stop; the Jobs list has them all.
    // What runs it: "shell" (the pane's shell), "tessh", or "" (can't).
    function bgLang(c) {
        const L = c.lang !== "" ? c.lang : shellKernel ? "shell" : language
        if (L === "tessh") return "tessh"
        if (L === "shell" || ["bash", "zsh", "sh", "fish"].indexOf(L) >= 0) return view || L !== "shell" ? "shell" : ""
        return ""
    }
    function isBgLine(src) { const t = src.trim(); return t.indexOf("\n") < 0 && /[^&]&$/.test(t) }
    function runJob(i, command, lang) {
        const cid = cells.get(i).cid
        cells.setProperty(i, "ranIn", (lang === "tessh" ? "tessh" : (shellName || "shell")) + " \u00b7 bg")
        Async.call(controller, k => controller.jobCall(k, sshHost, "job.start", JSON.stringify({ command: command, lang: lang, cwd: shellDir, pid: shellPid, owner: key, cell: cid })), json => {
            const j = indexOf(cid)
            if (j < 0) return
            let r = {}
            try { r = JSON.parse(json) } catch (e) {}
            if (!r.id) {
                // (A tesserad from before jobs: it swaps in the new build once its kernels are done.)
                if (/no request/.test(r.error || "")) r.error = "tesserad here is an older build, from before background jobs. It upgrades itself once no kernels are running: stop them (Kernels \u2192 Stop all), then run this again."
                cells.setProperty(j, "running", false)
                cells.setProperty(j, "failed", true)
                cells.setProperty(j, "ended", Date.now())
                addOutput(j, { output_type: "error", ename: "Not started", evalue: r.error || "tesserad didn't answer", traceback: ["Couldn't start it in the background: " + (r.error || "tesserad didn't answer")] })
                return
            }
            cells.setProperty(j, "job", r.id)
            cells.setProperty(j, "execs", JSON.stringify([r.exec]))
            touch()
            refreshJobs()
        })
    }
    function stopJob(i) {
        const c = cells.get(i)
        if (c.job > 0) controller.jobCall("", sshHost, "job.stop", JSON.stringify({ id: c.job }))
    }
    // (Each running job's new output, every half second; one round at a time.)
    property bool jobPolling: false
    property bool jobsRunning: false
    Timer {
        interval: 500; repeat: true
        running: nb.jobsRunning && !nb.jobPolling
        onTriggered: nb.pollJobs()
    }
    function updateJobsRunning() {
        let r = false
        for (let i = 0; i < cells.count && !r; ++i) r = cells.get(i).job > 0 && cells.get(i).running
        jobsRunning = r
    }
    function pollJobs() {
        const want = []
        for (let i = 0; i < cells.count; ++i) { const c = cells.get(i); if (c.job > 0 && c.running) want.push({ cid: c.cid, id: c.job, from: c.jobAt }) }
        if (want.length === 0) { jobsRunning = false; return }
        jobPolling = true
        let left = want.length
        for (const w of want) {
            Async.call(controller, k => controller.jobCall(k, sshHost, "job.output", JSON.stringify({ id: w.id, from: w.from })), json => {
                if (--left === 0) jobPolling = false
                const i = indexOf(w.cid)
                if (i < 0 || cells.get(i).job !== w.id) return
                let r = {}
                try { r = JSON.parse(json) } catch (e) { return }
                if (r.error) return
                if (r.gone) { cells.setProperty(i, "running", false); cells.setProperty(i, "ended", Date.now()); updateJobsRunning(); touch(); return }
                for (const o of r.out || []) addOutput(i, { output_type: "stream", name: o[0] === 2 ? "stderr" : "stdout", text: o[1] })
                if (r.next !== undefined) cells.setProperty(i, "jobAt", r.next)
                if (!r.running) {
                    cells.setProperty(i, "running", false)
                    cells.setProperty(i, "status", r.status || 0)
                    cells.setProperty(i, "failed", (r.status || 0) !== 0)
                    cells.setProperty(i, "ended", r.ended || Date.now())
                    if (cells.get(i).count < 0) cells.setProperty(i, "count", ++execCount)
                    updateJobsRunning()
                    refreshJobs()
                    touch()
                }
            })
        }
    }
    // Every job on this host (the Jobs list): [{id, owner, cell, command, pid, started, ended, running, status}].
    property var hostJobs: []
    readonly property int jobsActive: hostJobs.filter(j => j.running).length
    function refreshJobs() {
        Async.call(controller, k => controller.jobCall(k, sshHost, "job.list", "{}"), json => {
            let r = []
            try { r = JSON.parse(json) } catch (e) {}
            if (Array.isArray(r)) hostJobs = r
        })
    }
    Timer { interval: 2000; repeat: true; running: nb.visible && (nb.jobsActive > 0 || jobsPop.opened); onTriggered: nb.refreshJobs() }
    function showJobCell(cid) {
        const i = indexOf(cid)
        if (i < 0) return
        current = i
        list.positionViewAtIndex(i, ListView.Beginning)
    }

    function runAll() { for (let i = 0; i < cells.count; ++i) if (cells.get(i).type === "code") run(i) }
    function runAbove(n) { for (let i = 0; i < n; ++i) if (cells.get(i).type === "code") run(i) }
    function interrupt() {
        if (viaShell) { if (view) view.sendText("\x03"); return }
        if (ttyRun && view) view.sendText("\x03")
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
    property int runningSeq: 0
    property int runningApplied: 0
    function refreshKernels() {
        if (!controller) return
        const s = ++runningSeq
        Async.call(controller, k => controller.jupyterRunning(k), r => {
            if (s < runningApplied) return   // (a later answer is shown already)
            runningApplied = s
            try { runningKernels = JSON.parse(r) } catch (e) { runningKernels = [] }
        })
    }
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
                        Text { width: parent.width; elide: Text.ElideRight; text: modelData.display + (modelData.host ? " on " + modelData.host : "") + (parent.parent.mine ? "" : "  (another notebook)"); color: Theme.text; font.pixelSize: 12 }
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
    property string shellName: ""       // the pane's shell (zsh, bash, fish, ...)
    // A file shown in a card (a text file clicked in a listing): its lines,
    // highlighted, in Tessera's viewer.
    property int fileRev: 0
    function addFileCard(path) {
        const c = newCell("code", path)
        c.viewPath = path
        c.ranIn = "file"
        c.cwd = path.replace(/\/[^\/]*$/, "") || "/"
        c.started = Date.now()
        c.ended = c.started
        cells.append(c)
        current = cells.count - 1
        touch()
        follow()
    }
    // (Cards made before the shell's name was known get it now.)
    onShellNameChanged: if (shellName !== "") for (let i = 0; i < cells.count; ++i) if (cells.get(i).tid > 0 && cells.get(i).ranIn === "") cells.setProperty(i, "ranIn", shellName)
    // What a cell's exit status means: the shell's codes or the command's man
    // page, else (asked when hovered) the AI's reading of it.
    property var exitNotes: ({})        // cid -> the AI's meaning ("…" while asked)
    readonly property string exitKey: "nb-exit-" + Math.random().toString(36).slice(2) + "-"
    function exitTip(c) {
        const st = c.fromTerm ? c.status : (c.failed ? 1 : 0)
        if (!c.fromTerm) return c.failed ? "Failed (an error)" : "Done"
        const m = exitMeaning(c.source, st)
        const ai = exitNotes[c.cid] || ""
        return "Exit " + st + (m !== "" ? ": " + m : ai === "…" ? ": asking the AI…" : ai !== "" ? ": " + ai + " (AI)" : "")
    }
    // (The shell's or man page's meaning, asked in the background and kept
    // by host, status and command: "" until it's in, then the tip shows it.)
    property var exitMeanings: ({})
    property var exitAsking: ({})       // (changed in place: no binding depends on it)
    function exitMeaning(src, st, done) {
        const k = sshHost + "\n" + st + "\n" + src
        if (exitMeanings[k] !== undefined) { if (done) done(exitMeanings[k]); return exitMeanings[k] }
        if (exitAsking[k]) { if (done) exitAsking[k].push(done); return "" }
        exitAsking[k] = done ? [done] : []
        const h = sshHost
        // (Later, not inside the tip's binding.)
        Qt.callLater(() => {
            const answer = (r) => {
                const m = Object.assign({}, exitMeanings); m[k] = r; exitMeanings = m
                const w = exitAsking[k] || []
                delete exitAsking[k]
                for (const f of w) f(r)
            }
            if (!controller) { answer(""); return }
            Async.call(controller, q => controller.exitMeaning(q, src, st, h), answer)
        })
        return ""
    }
    function askExit(c, i) {
        if (!controller || !(c.tid > 0) || c.status === 0 || exitNotes[c.cid] !== undefined) return
        const cid = c.cid, src = c.source, st = c.status, cwd = c.cwd
        exitMeaning(src, st, m => {
            if (m !== "" || !controller || !controller.aiReady("inline") || exitNotes[cid] !== undefined) return
            // (The cell again by its id: it may have moved or gone.)
            const j = indexOf(cid)
            if (j < 0) return
            const n = Object.assign({}, exitNotes); n[cid] = "…"; exitNotes = n
            const out = outputText(j).split("\n").slice(-30).join("\n")
            controller.aiRun(exitKey + cid, "exit", JSON.stringify([{ role: "user", content: "Command: " + src + "\nExit status: " + st + "\nIts output (the end):\n" + out }]), JSON.stringify({ cwd: cwd, shell: shellName }))
        })
    }
    // (Test hook: TESSERA_TEST_EXIT_AI: the last cell's status asked about at
    // 12 s, its tip logged at 30 s.)
    Timer { interval: 12000; running: nb.controller !== null && nb.controller.testEnv("EXIT_AI") !== ""; onTriggered: { const i = cells.count - 1; if (i >= 0) nb.askExit(cells.get(i), i) } }
    Timer { interval: 30000; running: nb.controller !== null && nb.controller.testEnv("EXIT_AI") !== ""; onTriggered: { const i = cells.count - 1; if (i >= 0) console.log("exit tip: " + nb.exitTip(Object.assign({ fromTerm: cells.get(i).tid > 0 }, cells.get(i)))) } }
    Connections {
        target: nb.controller
        function onAiDone(key, reply, error) {
            if (!key.startsWith(nb.exitKey)) return
            const r = (reply || "").replace(/<think>[\s\S]*?<\/think>/g, "").trim().split("\n")[0].replace(/^["']|["'.]$/g, "")
            const n = Object.assign({}, nb.exitNotes); n[key.substring(nb.exitKey.length)] = error || r === "" || /^unknown$/i.test(r) ? "" : r; nb.exitNotes = n
        }
    }
    // What ran a cell: the pane's shell, its language's kernel, or the
    // notebook's kernel.
    function ranBy(c) {
        if (c.ranIn !== "") return c.ranIn
        if (c.lang !== "") return (c.langKernel && c.langKernel.display) || c.lang
        if (c.fromTerm || shellKernel) return shellName !== "" ? shellName : "shell"
        return kernel ? kernel.display : kernelName
    }
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
        // (A shell: Tessera's own shell kernel first.)
        return ks.find(k => k.name.startsWith("tessera-sh-")) || ks.find(k => !k.name.startsWith("tessera-")) || ks.find(k => k.name === "tessera-" + l) || ks[0] || null
    }
    function hasKernelFor(l) { if (kernels.length === 0) listKernels(); return l !== "" && langSpec(l) !== null }
    function setLang(l, patch) { const m = Object.assign({}, langKernels); m[l] = Object.assign({}, m[l] || {}, patch); langKernels = m }
    function runLang(cid, l, listed) {
        // (No list yet: run once it's in.)
        if (kernels.length === 0 && !listed) { listKernels(() => runLang(cid, l, true)); return }
        const e = langKernels[l]
        if (e && (e.state === "idle" || e.state === "busy")) { execLang(cid, l); return }
        setLang(l, { queue: ((e && e.queue) || []).concat([cid]) })
        if (e && e.state === "starting") return
        const spec = langSpec(l)
        if (!spec) {
            setLang(l, { queue: [] })
            const i = indexOf(cid)
            const none = () => {
                const j = indexOf(cid)
                if (j < 0) return
                cells.setProperty(j, "running", false)
                addOutput(j, { output_type: "error", ename: "No kernel", evalue: "No Jupyter kernel for " + l, traceback: ["No Jupyter kernel for " + l + " (installed or from Nix)."] })
                setLang(l, { queue: [] })
            }
            // (No kernel for it: run once, as a program, in the pane's shell.)
            if (i >= 0) runOnce(cid, l, cells.get(i).source, none)
            else none()
            return
        }
        setLang(l, { name: spec.name, display: spec.display, state: "starting", message: "" })
        controller.jupyterStart(langKey(l), spec.name, shellDir, shellPid, storeKey)
    }
    // Start a language's kernel ahead of its first cell (a shell picked in
    // the editor), so the run doesn't wait for it to come up.
    function warmLang(l, listed) {
        if (kernels.length === 0 && !listed) { listKernels(() => warmLang(l, true)); return }
        const e = langKernels[l]
        if (e && e.state !== "dead") return
        const spec = langSpec(l)
        if (!spec) return
        setLang(l, { name: spec.name, display: spec.display, state: "starting", message: "", queue: [] })
        controller.jupyterStart(langKey(l), spec.name, shellDir, shellPid, storeKey)
    }
    function execLang(cid, l) {
        const i = indexOf(cid)
        if (i < 0) return
        controller.jupyterSync(langKey(l), shellPid, shellDir)
        Async.call(controller, k => controller.jupyterExecute(k, langKey(l), cid, cells.get(i).source), ok => {
            const j = indexOf(cid)
            if (ok === "true" || j < 0) return
            cells.setProperty(j, "running", false)
            addOutput(j, { output_type: "error", ename: "Not sent", evalue: "The kernel isn't reachable.", traceback: ["The " + l + " kernel isn't reachable: run it again to restart it."] })
            setLang(l, { state: "dead" })
        })
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
                for (let i = 0; i < cells.count; ++i) if (cells.get(i).running && cells.get(i).lang === l && cells.get(i).tid <= 0 && cells.get(i).job <= 0) cells.setProperty(i, "running", false)
                setLang(l, { queue: [] })
            }
        } else if (e.type !== "info" && e.type !== "complete") kernelEvent(e, langKey(l))
    }
    function stopLangKernels() { for (const l in langKernels) controller.jupyterStop(langKey(l)); langKernels = ({}) }
    // Code in another language from the pane's editor: a cell in that
    // language at the end, run in its kernel.
    function submitLang(code, l) {
        const c = newCell("code", code)
        c.lang = sameLang(l, runsIn) ? "" : l
        cells.append(c)
        current = cells.count - 1
        run(cells.count - 1)
        follow()
    }
    // A cell as a program run once in the pane's shell (snippets.rs's runner
    // for its language); `failed` runs if there's none. (The command is
    // worked out in the background; runs go to the shell in the order asked.)
    property var onceQueue: []          // (changed in place)
    function runOnce(cid, l, code, failed) {
        const e = { cid: cid, l: l, failed: failed, cmd: "", done: false }
        onceQueue.push(e)
        const answer = (cmd) => {
            e.cmd = cmd; e.done = true
            while (onceQueue.length > 0 && onceQueue[0].done) onceRun(onceQueue.shift())
        }
        if (!controller) { answer(""); return }
        Async.call(controller, k => controller.snippetCommand(k, l, code, shellDir, shellName || "bash", sshHost), answer)
    }
    function onceRun(e) {
        const i = indexOf(e.cid)
        if (i < 0) return   // (the cell's gone)
        if (e.cmd === "" || !view) {
            cells.setProperty(i, "running", false)
            addOutput(i, { output_type: "error", ename: "Can't run", evalue: e.l, traceback: ["Tessera has no way to run " + e.l + " here."] })
            if (e.failed) e.failed()
            return
        }
        if (!cells.get(i).running) return   // (stopped meanwhile)
        shellRun(e.cid, e.cmd)
    }
    // Code from the pane's editor, for a kernel that isn't the shell: a new
    // cell at the end, run.
    function submit(code) {
        expandedId = ""         // (a new card shows: nothing hides it)
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
        ownRun = on || ttyRun !== null
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
    // A terminal command that's no card: starting or leaving a shell switched
    // to from the editor.
    function hideTerm(tid) {
        retire(tid)
        for (let i = cells.count - 1; i >= 0; --i) if (cells.get(i).tid === tid && cells.get(i).cid !== undefined) { cells.remove(i); touch() }
    }
    // (No command and nothing to read in its output: a shell starting up.)
    function blankTerm(t) {
        if (!t.done || (t.command || "").trim() !== "") return false
        if (!(t.size > 0)) return true
        return view.cellOutput(t.id, "text").replace(/[\s\u23ce%]/g, "") === ""
    }
    function retire(tid) {
        if (retired.indexOf(tid) < 0) retired = retired.concat([tid])
        const m = Object.assign({}, linked); delete m[tid]; linked = m
    }
    // A program a tessh cell runs: the kernel hands it here and it runs in
    // this pane's terminal, as a shell cell's does. Its output goes in the
    // cell's card as it comes; a terminal app (one on the alternate screen)
    // gets the live terminal in the card instead, and leaves no text. Its
    // exit status goes back. {cid, key, tid (its terminal cell), base (the
    // card's outputs before it)}
    property var ttyRun: null
    function runTty(cid, tty, k) {
        if (!view) { controller.jupyterInput(k, "127"); return }
        const q = s => "'" + String(s).replace(/'/g, "'\\''") + "'"
        const cmd = (tty.cwd && tty.cwd !== shellDir ? "cd " + q(tty.cwd) + " && " : "") + tty.command
        // (since: the terminal's newest command now; the one this starts comes
        // after it, and reads as cmd.)
        const ts = termCells()
        ttyRun = { cid: cid, key: k, tid: 0, base: -1, before: "", cmd: cmd, since: ts.length ? ts[ts.length - 1].id : 0 }
        updateOwnRun()
        touch()
        view.sendText(cmd + "\r")
        findLive()
        Qt.callLater(followLive)
    }
    // (Its output so far, after what the card had before it.)
    function ttyText(t) {
        const r = ttyRun
        const i = indexOf(r.cid)
        if (i < 0) return
        let outs = []
        try { outs = JSON.parse(cells.get(i).outputs) } catch (x) {}
        // (It goes on the text before it, as a stream's next text would.)
        if (r.base < 0) {
            const last = outs[outs.length - 1]
            const joins = last && last.output_type === "stream" && last.name === "stdout"
            ttyRun = Object.assign({}, r, { base: joins ? outs.length - 1 : outs.length, before: joins ? last.text : "" })
            return ttyText(t)
        }
        outs = outs.slice(0, r.base)
        let text = t.screen ? "" : view.cellOutput(t.id, "text")
        text = text.trim() === "" ? "" : text.replace(/\n*$/, "\n")
        if (r.before + text !== "") outs.push({ output_type: "stream", name: "stdout", text: r.before + text })
        // (Only when it changed: a card set again redraws, and can take the
        // keys from a terminal app running in it.)
        const j = JSON.stringify(outs)
        if (j !== cells.get(i).outputs) cells.setProperty(i, "outputs", j)
    }
    Timer {
        interval: 300; repeat: true
        running: nb.ttyRun !== null && nb.ttyRun.tid > 0
        // (Its end too: the terminal's word that it finished can come while
        // the notebook isn't listening, and the cell would spin on.)
        onTriggered: {
            const t = nb.termCells().find(x => x.id === nb.ttyRun.tid)
            if (t) { if (t.done) nb.finishTty(t); else nb.ttyText(t) }
            else nb.askKept()
        }
    }
    // (A window that came back while it ran never saw it start, and won't see
    // it end: tesserad keeps it once it's done, so that's asked, now and then.)
    property bool keptAsking: false
    property real keptAsked: 0
    function askKept() {
        if (keptAsking || !controller || storeKey === "" || Date.now() - keptAsked < 1000) return
        keptAsking = true
        keptAsked = Date.now()
        const r = ttyRun, h = sshHost, sk = storeKey
        Async.call(controller, k => controller.keptCell(k, h, sk, r.tid), text => {
            keptAsking = false
            if (text === "" || !ttyRun || ttyRun.key !== r.key || ttyRun.tid !== r.tid) return
            let c = null
            try { c = JSON.parse(text) } catch (e) { return }
            finishTty({ id: c.id, done: true, status: c.status, screen: true })
        })
    }
    function finishTty(t) {
        ttyText(t)
        const r = ttyRun
        ttyRun = null
        updateOwnRun()
        touch()
        controller.jupyterInput(r.key, String(t.status))
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
        // (What it ran with, from shell integration: kept with the card.)
        if (c.execs === "[]") {
            let x = null
            try { x = JSON.parse(view.cellOutput(t.id, "exec") || "null") } catch (e) {}
            if (x && (x.user || (x.env && x.env.length))) cells.setProperty(i, "execs", JSON.stringify([x]))
        }
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
            const bt = byId[c.tid]
            if (bt && nb.blankTerm(bt)) { cells.remove(i); retire(c.tid); continue }
            if (bt) fromTerm(i, bt)
            else if (path === "" && oldest > 0 && c.tid < oldest) cells.remove(i)
        }
        // (A tessh cell's program: its terminal command found by what it ran,
        // newer than when it was sent; shown in that cell, no card of its own.
        // Not by the ids kept from before: a new shell numbers from 1 again.)
        if (ttyRun && ttyRun.tid === 0) {
            const want = (ttyRun.cmd || "").trim()
            const t = ts.find(x => x.id > (ttyRun.since || 0) && (want === "" || (x.command || "").trim() === want))
            if (t) { ttyRun = Object.assign({}, ttyRun, { tid: t.id }); retire(t.id); delete linked[t.id]; touch() }
        }
        let added = false
        for (const t of ts) {
            if (ttyRun && t.id === ttyRun.tid) continue
            if (linked[t.id] !== undefined || retired.indexOf(t.id) >= 0 || t.id <= fileStart) continue
            // (Nothing typed and nothing shown: a shell starting up, no card.)
            if (nb.blankTerm(t)) { retire(t.id); continue }
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
            if (cells.get(at).ranIn === "" && shellName !== "") cells.setProperty(at, "ranIn", shellName)
            link(t.id, cells.get(at).cid)
            fromTerm(at, t)
            touch()
        }
        if (ttyRun && ttyRun.tid > 0 && byId[ttyRun.tid]) {
            if (byId[ttyRun.tid].done) finishTty(byId[ttyRun.tid])
            else ttyText(byId[ttyRun.tid])
        }
        updateOwnRun()
        if (added) follow()
    }
    Connections {
        target: nb.view
        function onCellsChanged(id, done) {
            // (0: the terminal's kept cells came back after a restart.)
            // (The kept notebook first, then the terminal's cells.)
            if (id === 0 && nb.path === "" && !nb.storeFound) { nb.loadStore(() => { nb.termRev++; nb.syncTerm() }); return }
            if (id === 0) nb.termRev++
            nb.syncTerm()
            if (done && id > 0) nb.classify(id)
        }
    }
    // While a command runs, its cell's output is read again now and then.
    property int tick: 0
    // ---- a full-screen program in its card ----------------------------------
    // The running command's card holds the pane's live terminal while a
    // full-screen program (vim, htop, mdfried, ...) runs: the terminal moves
    // into liveHost (TerminalPane's termSlot), which sits over that card's
    // output, at its size; Expand makes it the window's.
    property bool shown: true               // (the pane shows the notebook)
    // A terminal app's card: most of the notebook, at least 300 px.
    Timer { id: restoreFollow; interval: 120; onTriggered: nb.followLive() }
    // (A terminal app's card: a regular card, 25 rows; less if the notebook
    // is smaller. Expand for more.)
    readonly property int liveHeight: Math.min(Math.round((outputRow > 0 ? outputRow : 18) * 25 + 12), Math.max(200, list.height - 120))
    readonly property bool liveWanted: view !== null && view.fullScreen && shown
    property int liveIndex: -1
    property Item liveBox: null            // the card's place for it (set by the card)
    readonly property Item liveSlot: liveWanted && liveIndex >= 0 ? liveHost : null
    function findLive() {
        if (ttyRun) { liveIndex = indexOf(ttyRun.cid); return }
        let n = -1
        for (let i = cells.count - 1; i >= 0; --i) if (cells.get(i).running) { n = i; break }
        liveIndex = n
    }
    // (The terminal app ended with its card expanded: the card goes back to its
    // own size.)
    onLiveWantedChanged: {
        const was = liveIndex >= 0 && liveIndex < cells.count ? cells.get(liveIndex).cid : ""
        findLive()
        if (liveWanted) Qt.callLater(followLive)
        else if (was !== "" && expandedId === was) toggleExpand(indexOf(was))
    }
    Timer { interval: 150; repeat: true; running: nb.liveWanted; onTriggered: { nb.findLive(); nb.placeLive() } }
    function placeLive() {
        if (!liveBox || !liveBox.visible) { liveHost.visible = false; return }
        const p = liveBox.mapToItem(liveClip, 0, 0)
        liveHost.x = Math.round(p.x); liveHost.y = Math.round(p.y)
        liveHost.width = liveBox.width; liveHost.height = liveBox.height
        liveHost.visible = true
    }
    // (Test hook, with EXPAND_CYCLE: each frame's time after a toggle.)
    property real expandT: 0
    property int expandFrames: -1
    property real expandLast: 0
    Connections {
        target: nb.Window.window
        enabled: nb.expandFrames >= 0
        function onFrameSwapped() {
            const t = Date.now() - nb.expandT
            if (t > 2500) { nb.expandFrames = -1; return }
            if (t - nb.expandLast > 40 || nb.expandFrames === 0) console.log("expand: frame " + nb.expandFrames + " at +" + t + " ms (gap " + (t - nb.expandLast) + ")")
            nb.expandFrames++; nb.expandLast = t
        }
    }
    // Expand (fill the notebook with card i) or Restore.
    function toggleExpand(i) {
        if (controller && controller.testEnv("EXPAND_CYCLE") !== "") { expandT = Date.now(); expandFrames = 0; expandLast = 0 }
        const cid = cells.get(i).cid
        expandedId = expandedId === cid ? "" : cid
        cells.setProperty(i, "hideOutput", false)
        // (Back from expanded, a terminal app's card stays in view.)
        Qt.callLater(() => { list.positionViewAtIndex(i, expandedId === "" && i === liveIndex ? ListView.Contain : ListView.Beginning); placeLive() })
        // (The other cards come back and lay out first: then again.)
        if (expandedId === "" && i === liveIndex) restoreFollow.restart()
        // (A terminal app's card: the keys go back to it, not to the button
        // clicked; a :q typed then went nowhere and it ran on.)
        if (i === liveIndex && liveWanted && view) Qt.callLater(() => view.forceActiveFocus())
    }
    function isMarkdownFile(p) { return /\.(md|markdown|mdx)$/i.test(p || "") }
    function followLive() { if (liveIndex >= 0) { list.positionViewAtIndex(liveIndex, ListView.Contain); Qt.callLater(placeLive) } }
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
    // A tessh cell whose code is an ls (or cd) alone, its view not changed by hand.
    function tesshFiles(cid, lang, source) {
        const tessh = lang !== "" ? lang === "tessh" : (kernelName === "tessh" || kernelName.endsWith("-tessh") || language === "tessh")
        return tessh && (!kinds[cid] || kinds[cid] === "auto") && builtinKind(source) === "files"
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
        if (keepTids && t.kernel_key) keyBase = t.kernel_key
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
            if (!(tm.tid > 0) && tm.started > 1) { x.started = tm.started; x.ended = tm.started + (tm.duration_ms || 0); x.failed = !!tm.failed }
            if (keepTids && !(tm.tid > 0) && tm.cid) { x.cid = tm.cid; if (tm.running) { x.running = true; x.ended = 0 } }
            if (keepTids && tm.job > 0) { x.job = tm.job; x.jobAt = tm.job_at || 0 }
            if (tm.lang) x.lang = tm.lang
            if (tm.ran_in) x.ranIn = tm.ran_in
            if (tm.view) x.viewPath = tm.view
            if (tm.view_lang) x.viewLang = tm.view_lang
            if (Array.isArray(tm.exec) && tm.exec.length) x.execs = JSON.stringify(tm.exec)
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
        // (A pane's notebook runs in the default kernel unless one was picked
        // in it; one kept from before there was a default, as a shell
        // notebook, does too.)
        kernelPicked = !!t.kernel_picked
        if (path === "" && t.tty_run && t.tty_run.cid && t.tty_run.key) ttyRun = t.tty_run
        const ks = metadata.kernelspec
        pickKernel(path === "" && !kernelPicked && (!ks || ks.name === "bash") ? null : ks)
    }
    // (Each step answers in the background; a later load() drops an
    // earlier one's answers.)
    property int loadSeq: 0
    function load() {
        const s = ++loadSeq
        loaded = false
        shadowed = false
        loadError = ""
        saveError = ""
        // (Kernels are kept, a pane's notebook's and a file's: reattach()
        // finds them.)
        if (path !== "") keyBase = fileKey(path)
        kernelState = ""
        shellWaiting = []
        listKernels(() => {
            if (s !== loadSeq) return
            if (path === "") loadOwn(s)
            else loadFile(s)
        })
    }
    function loadOwn(s) {
        // The pane's own notebook: what was kept with its session, then the
        // terminal's cells.
        cells.clear(); linked = ({}); retired = []; kinds = ({}); fileViews = ({}); heights = ({}); metadata = ({})
        fileStart = 0
        pickKernel(null)
        loadStore(() => {
            if (s !== loadSeq) return
            reattach(() => { if (s === loadSeq) finishLoad() })
        })
    }
    function loadFile(s) {
        const p = path, h = sshHost, kb = keyBase
        Async.call(controller, k => controller.readTextFile(k, p), text => {
            if (s !== loadSeq) return
            const r = JSON.parse(text)
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
            // (Kept when the app quit with its kernels running: the cells as
            // they were then, their ids and outputs, so what the kernels
            // printed since lands in them. Unsaved changes, as they were.)
            Async.call(controller, k => controller.notebookRead(k, h, kb), kept => {
                if (s !== loadSeq) return
                let shadow = null
                try { shadow = JSON.parse(kept) } catch (e) {}
                if (shadow && shadow.text) {
                    try { fill(JSON.parse(shadow.text), true); shadowed = true } catch (e) {}
                }
                // (The terminal's cells from before it opened aren't the file's.)
                const ts = view ? termCells() : []
                fileStart = ts.length ? ts[ts.length - 1].id : 0
                finishLoad()
            })
        })
    }
    function finishLoad() {
        dirty = shadowed
        shadowed = false
        current = 0
        loaded = true
        syncTerm()
        if (path !== "") reattach()
        // (A notebook opens at its end, the latest cells, and stays there
        // while its cards lay out.)
        follow()
    }
    property bool shadowed: false
    // ---- the pane's own notebook, kept with its session ----
    property bool storeFound: false
    function storePath() { return controller && storeKey !== "" ? controller.stateFolder() + "/notebooks/" + storeKey + ".ipynb" : "" }
    // (Read in the background, one read at a time: a call while one is out
    // waits for it, and `done` runs once it's filled in.)
    property bool storeLoading: false
    property var storeWaiting: []
    function loadStore(done) {
        if (done) storeWaiting = storeWaiting.concat([done])
        if (!controller || storeKey === "") {
            if (storeLoading) return
            const w = storeWaiting; storeWaiting = []
            for (const f of w) f()
            return
        }
        if (storeLoading) return
        storeLoading = true
        const h = sshHost, sk = storeKey
        Async.call(controller, k => controller.notebookRead(k, h, sk), text => {
            storeLoading = false
            // (Another session's since: that one's read instead.)
            if (h !== sshHost || sk !== storeKey) { loadStore(); return }
            const w = storeWaiting; storeWaiting = []
            const r = JSON.parse(text)
            if (r.error === undefined) {
                try { fill(JSON.parse(r.text), true); storeFound = true } catch (e) {}
                dirty = false
            }
            for (const f of w) f()
        })
    }
    // (The pane's shell started again (exit, restart): a new session, and the
    // notebook carries on in it, its cards as they were.)
    property string prevStoreKey: ""
    onStoreKeyChanged: {
        const was = prevStoreKey
        prevStoreKey = storeKey
        if (loaded && path === "" && was !== "" && storeKey !== "" && cells.count > 0) {
            const doc = JSON.parse(serialize(false))
            fill(doc, false)
            retired = []
            linked = ({})
            fileStart = 0
            storeFound = true
            touch()
            return
        }
        if (loaded && path === "" && !storeFound) loadStore(() => reattach(() => syncTerm()))
    }
    // The pane's notebook's kernels that ran on in tesserad (the window was
    // closed, or rebuilt): taken back, with what they printed meanwhile.
    function reattach(done) {
        if (!controller) { if (done) done(); return }
        const ak = key
        Async.call(controller, q => controller.jupyterAttach(q, ak, storeKey), text => {
            // (Another notebook's kernels by now: not these.)
            if (ak === key) attached(text)
            if (done) done()
        })
    }
    function attached(text) {
        let r = null
        try { r = JSON.parse(text) } catch (e) { return }
        const up = (s) => s === "busy" ? "busy" : s === "starting" ? "starting" : "idle"
        for (const k of (r && r.kernels) || []) {
            if (k.key === key) {
                if (k.name !== kernelName && k.name !== "") { kernelName = k.name; language = k.language || language }
                kernelState = up(k.state)
            } else if (k.key.startsWith(key + "~")) {
                setLang(k.key.substring(key.length + 1), { name: k.name, display: k.display, state: up(k.state), message: "", queue: [] })
            }
        }
        // (A cell left running whose kernel is gone: it ended unseen.)
        const live = ((r && r.kernels) || []).map(k => k.key)
        for (let i = 0; i < cells.count; ++i) {
            const c = cells.get(i)
            if (!c.running || c.tid > 0 || c.job > 0) continue
            if (live.indexOf(c.lang !== "" ? langKey(c.lang) : key) < 0) cells.setProperty(i, "running", false)
        }
    }
    function touch() {
        dirty = true
        if (path === "" && loaded) storeTimer.restart()
    }
    Timer { id: storeTimer; interval: 1500; onTriggered: nb.keep() }
    // (Quitting: what came since the last save is kept now. Kernels' output
    // after this is kept by tesserad for the next start.)
    Connections {
        target: Qt.application
        function onAboutToQuit() {
            nb.quitting = true
            if (nb.path === "" && nb.dirty) { storeTimer.stop(); nb.keep() }
            // (A file's notebook with kernels running: kept as it is now, for
            // the next start, without touching the file.)
            if (nb.path !== "" && nb.kernelsRunning()) nb.controller.notebookWrite(nb.sshHost, nb.keyBase, nb.serialize(true))
        }
    }
    function keep() {
        // (Not while loading: the cells aren't all in.)
        if (!controller || storeKey === "" || path !== "" || !loaded) return
        controller.notebookWrite(sshHost, storeKey, serialize(true))
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
                } else {
                    try { outs = JSON.parse(c.outputs) } catch (e) {}
                    // (Where, when and how it ran, kept with it.)
                    if (c.started > 1) meta.tessera = Object.assign({}, meta.tessera || {}, { cwd: c.cwd, started: c.started, duration_ms: Math.max(0, c.ended - c.started), failed: !!c.failed })
                    // (Kept with the pane: its id, for the kernel's output that
                    // comes after a restart, and whether it was still running.)
                    if (kept) meta.tessera = Object.assign({}, meta.tessera || {}, { cid: c.cid, running: !!c.running })
                    // (A background job: which, and how much of its output is in.)
                    if (kept && c.job > 0) meta.tessera = Object.assign({}, meta.tessera || {}, { job: c.job, job_at: c.jobAt })
                }
                if (c.lang !== "") meta.tessera = Object.assign({}, meta.tessera || {}, { lang: c.lang })
                if (c.ranIn !== "") meta.tessera = Object.assign({}, meta.tessera || {}, { ran_in: c.ranIn })
                // (What each program ran with: command line, folder, user, environment.)
                if (c.execs !== "[]") { try { meta.tessera = Object.assign({}, meta.tessera || {}, { exec: JSON.parse(c.execs) }) } catch (e) {} }
                if (c.viewPath !== "") { meta.tessera = Object.assign({}, meta.tessera || {}, c.viewLang !== "" ? { view: c.viewPath, view_lang: c.viewLang } : { view: c.viewPath }); outs = [] }
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
        if (kept) t.kernel_key = keyBase
        if (kernelPicked) t.kernel_picked = true
        // (A tessh program running in the terminal for a cell: still waited on
        // by the kernel when the window comes back.)
        if (kept && ttyRun) t.tty_run = ttyRun
        if (Object.keys(t).length) md.tessera = t
        return JSON.stringify(sorted({ cells: out, metadata: md, nbformat: 4, nbformat_minor: 5 }), null, 1) + "\n"
    }
    property string written: ""         // the path just saved to (not read back)
    function save() {
        if (path === "") { saveDialog.open(); return }
        writeTo(path)
    }
    function writeTo(p) {
        const was = path, h = sshHost, kb = keyBase
        Async.call(controller, k => controller.writeTextFile(k, p, serialize(false)), err => {
            // (Saved: the copy kept for after a restart isn't needed.)
            if (err === "" && p === was) controller.notebookWrite(h, kb, "")
            // (Another notebook here since: nothing more of this one's.)
            if (path !== was) return
            if (err !== "") { saveError = err; return }
            saveError = ""
            dirty = false
            if (p !== path) {
                // (The pane's notebook is now this file's: the shell's cells so far are in it.)
                written = p
                pathPicked(p)
            }
        })
    }
    onPathChanged: {
        if (!controller) return
        if (path !== "" && path === written) { written = ""; return }
        load()
    }
    Component.onCompleted: if (controller) { load(); refreshJobs() }
    onControllerChanged: if (controller && !loaded) load()
    // (A pane's notebook's kernels stay, in tesserad, as long as its session:
    // the window closing doesn't end them. A file's go with it.)
    function kernelsRunning() {
        if (!viaShell && (kernelState === "idle" || kernelState === "busy" || kernelState === "starting")) return true
        for (const l in langKernels) if (langKernels[l].state && langKernels[l].state !== "dead") return true
        return false
    }
    // (A pane's notebook's kernels stay, in tesserad, as long as its session.
    // A file's go when its pane is closed; quitting or closing the window
    // keeps them, like the shells.)
    Component.onDestruction: {
        if (!controller) return
        if (path === "" && dirty) keep()
        if (path !== "" && !quitting && !keepKernels) { controller.jupyterStop(key); stopLangKernels(); controller.notebookWrite(sshHost, keyBase, "") }
        else if (path !== "" && keepKernels && kernelsRunning()) controller.notebookWrite(sshHost, keyBase, serialize(true))
    }
    property string editorKeys: "emacs"   // the input editor's (Emacs, VS Code, vi): file cards edit with them
    signal editorKeysPicked(string keys)  // (picked on a file card: for the input editor too)
    property int saveAsCell: -1
    FileDialog {
        id: fileSaveDialog
        title: "Save the file as"
        fileMode: FileDialog.SaveFile
        onAccepted: {
            const p = decodeURIComponent(String(selectedFile).replace(/^file:\/\//, ""))
            const i = nb.saveAsCell
            const it = list.itemAtIndex(i)
            // (The card's viewer does it: it has the text.)
            if (it) for (let k = 0; k < it.children.length; ++k) { const c = it.children[k]; if (c.objectName === "cardBody" && c.viewer) c.viewer.saveAs(p) }
        }
    }
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
    // The toolbar's, while this notebook is the window's active pane's (its
    // tooltips say them). Ctrl+Alt: free in shells and editors; not L or T,
    // which the desktop takes.
    readonly property bool keysHere: nb.visible && view !== null && Window.window !== null && Window.window.activePane !== null && Window.window.activePane !== undefined && Window.window.activePane.view === view
    Shortcut { sequence: "Ctrl+Alt+Return"; enabled: nb.keysHere; context: Qt.WindowShortcut; onActivated: nb.runAll() }
    Shortcut { sequence: "Ctrl+Alt+C"; enabled: nb.keysHere; context: Qt.WindowShortcut; onActivated: nb.interrupt() }
    Shortcut { sequence: "Ctrl+Alt+R"; enabled: nb.keysHere && !nb.viaShell; context: Qt.WindowShortcut; onActivated: nb.restartKernel() }
    Shortcut { sequence: "Ctrl+Alt+X"; enabled: nb.keysHere; context: Qt.WindowShortcut; onActivated: nb.clearOutputs() }
    Shortcut { sequence: "Ctrl+Alt+K"; enabled: nb.keysHere; context: Qt.WindowShortcut; onActivated: { nb.refreshKernels(); kernelsPop.open() } }
    Shortcut { sequence: "Ctrl+Alt+J"; enabled: nb.keysHere; context: Qt.WindowShortcut; onActivated: { nb.refreshJobs(); jobsPop.open() } }
    Shortcut { sequence: "Ctrl+Alt+B"; enabled: nb.keysHere; context: Qt.WindowShortcut; onActivated: nb.insertCell(nb.current + 1, "code") }
    Shortcut { sequence: "Ctrl+Alt+M"; enabled: nb.keysHere; context: Qt.WindowShortcut; onActivated: nb.insertCell(nb.current + 1, "markdown") }
    Shortcut { sequence: "Ctrl+Alt+P"; enabled: nb.keysHere; context: Qt.WindowShortcut; onActivated: { nb.listKernels(); kernelBox.popup.open() } }
    // (Expand or Restore: the expanded card, else the chosen one, else a
    // running terminal app's, else the last.)
    Shortcut { sequence: "Ctrl+Alt+F"; enabled: nb.keysHere && cells.count > 0; context: Qt.WindowShortcut
               onActivated: nb.toggleExpand(nb.expandedId !== "" ? nb.indexOf(nb.expandedId) : nb.current >= 0 && nb.current < cells.count ? nb.current : nb.liveIndex >= 0 ? nb.liveIndex : cells.count - 1) }

    // (Test hook: TESSERA_TEST_OPEN_GUIDE opens the guide's tab at 8 s.)
    Timer { interval: 8000; running: nb.controller !== null && nb.path === "" && nb.controller.testEnv("OPEN_GUIDE") !== ""; onTriggered: nb.openGuide() }
    Shortcut { sequence: "F1"; enabled: nb.visible && (nb.kernelName === "tessh" || nb.kernelName.endsWith("-tessh")); context: Qt.WindowShortcut; onActivated: nb.openGuide() }


    // (Test hooks: TESSERA_TEST_NB_RUN_ALL runs every cell once it's open,
    // NB_SAVE saves, NB_SCROLL_END scrolls down, NB_EXPORT=<path> saves there.)
    Timer { interval: 1500; running: nb.controller !== null && nb.visible && nb.controller.testEnv("NB_RUN_ALL") !== ""; onTriggered: nb.runAll() }
    Timer { interval: 20000; running: nb.controller !== null && nb.visible && nb.controller.testEnv("NB_SAVE") !== ""; onTriggered: nb.save() }
    Timer { interval: 16000; running: nb.controller !== null && nb.visible && nb.controller.testEnv("NB_SCROLL_END") !== ""; onTriggered: list.positionViewAtEnd() }
    // (Test hook: TESSERA_TEST_NB_KERNEL=<name> picks that kernel at 9.5 s;
    // TESSERA_TEST_NB_CELL=<code> then runs a cell of it at 11 s.)
    Timer { interval: 9500; running: nb.controller !== null && nb.controller.testEnv("NB_KERNEL") !== ""
            onTriggered: nb.listKernels(() => { nb.changeKernel(nb.controller.testEnv("NB_KERNEL")); console.log("nb kernel test: " + nb.kernels.length + " in the list, picked " + nb.kernelName + " oneShot " + nb.oneShot) }) }
    Timer { interval: 11000; running: nb.controller !== null && nb.controller.testEnv("NB_CELL") !== ""; onTriggered: nb.submit(nb.controller.testEnv("NB_CELL").replace(/\\n/g, "\n")) }
    // (Test hook: TESSERA_TEST_EXPAND_LAST expands the last card at 16 s, as
    // its Expand button does, focus taken first as a click takes it.)
    Timer { interval: 16000; running: nb.controller !== null && nb.controller.testEnv("EXPAND_LAST") !== ""; onTriggered: if (cells.count > 0) { list.forceActiveFocus(); nb.toggleExpand(cells.count - 1) } }
    // (TESSERA_TEST_EXPAND_CYCLE=n: Expand/Restore the last card n times, every 1.5 s from 12 s.)
    Timer {
        property int left: nb.controller !== null ? 2 * Number(nb.controller.testEnv("EXPAND_CYCLE") || 0) : 0
        interval: 12000; repeat: true; running: left > 0
        onTriggered: { interval = 1500; list.forceActiveFocus(); nb.toggleExpand(cells.count - 1); --left }
    }
    // (TESSERA_TEST_EXPAND_LAST=restore: and back to its own size at 19 s.)
    Timer { interval: 19000; running: nb.controller !== null && nb.controller.testEnv("EXPAND_LAST") === "restore"; onTriggered: if (nb.expandedId !== "") { list.forceActiveFocus(); nb.toggleExpand(nb.indexOf(nb.expandedId)) } }
    // (Test hook: TESSERA_TEST_FILE_CARD=<path>: a file card for it at 10 s.)
    Timer { interval: 10000; running: nb.controller !== null && nb.controller.testEnv("FILE_CARD") !== ""; onTriggered: nb.addFileCard(nb.controller.testEnv("FILE_CARD")) }
    // (Test hook: TESSERA_TEST_EDIT_FILE: the last card edited at 12 s, saved at 20 s.)
    function lastViewer() { const it = list.itemAtIndex(cells.count - 1); if (it) for (let k = 0; k < it.children.length; ++k) { const c = it.children[k]; if (c.objectName === "cardBody") return c.viewer } return null }
    Timer { interval: 12000; running: nb.controller !== null && nb.controller.testEnv("EDIT_FILE") !== ""; onTriggered: { const v = nb.lastViewer(); if (v) v.startEdit() } }
    Timer { interval: 20000; running: nb.controller !== null && nb.controller.testEnv("EDIT_FILE") !== ""; onTriggered: { const v = nb.lastViewer(); if (v) console.log("edit test: saved " + v.save() + " dirty " + v.dirty) } }
    // (Test hook: TESSERA_TEST_TABLE_SQL=<sql>: run on the last card's table at 14 s, saved at 18 s.)
    Timer { interval: 14000; running: nb.controller !== null && nb.controller.testEnv("TABLE_SQL") !== ""; onTriggered: { const v = nb.lastViewer(); if (v && v.runSql) v.runSql(nb.controller.testEnv("TABLE_SQL")) } }
    Timer { interval: 18000; running: nb.controller !== null && nb.controller.testEnv("TABLE_SQL") !== ""; onTriggered: { const v = nb.lastViewer(); if (v && v.saveTo) { v.saveTo(""); console.log("table test: " + v.note + " " + v.error) } } }
    // (Test hook: TESSERA_TEST_NB_SCROLL_TOP=<ms>: to the top then.)
    Timer { interval: Number(nb.controller ? nb.controller.testEnv("NB_SCROLL_TOP") || 0 : 0); running: nb.controller !== null && nb.visible && nb.controller.testEnv("NB_SCROLL_TOP") !== ""; onTriggered: { nb.sticking = false; list.positionViewAtBeginning() } }
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
    // (A terminal app's card: whole rows, no strip under its last one.)
    function liveFit(h) {
        if (!view || outputRow <= 0) return h
        const pad = 2 * view.padding
        return Math.floor((h - pad) / outputRow) * outputRow + pad + 1
    }
    readonly property real outputRow: view && view.cellHeight > 0 ? view.cellHeight : 0
    readonly property real outputPointSize: view && view.fontPointSize > 0 ? view.fontPointSize : 10
    function tight(html) { return "<div style=\"line-height:" + (outputRow > 0 ? Math.round(outputRow) + "px" : "100%") + "\">" + html + "</div>" }
    function tightText(t) { return tight("<span style=\"white-space:pre-wrap\">" + t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") + "</span>") }
    function palette(html) { return html.replace(/var\(--vt-palette-(\d+)\)/g, (m, n) => paletteColor(parseInt(n))) }
    function colored(text) { return controller ? tight(palette(controller.ansiHtml(text))) : "" }
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
            Tool { label: nb.narrow ? "▶▶" : "▶▶ Run All"; tip: "Run every cell, top to bottom (Ctrl+Alt+Enter)"; onClicked: nb.runAll() }
            Tool { label: "■"; tip: "Interrupt (Ctrl+Alt+C)"; onClicked: nb.interrupt() }
            Tool { visible: !nb.viaShell; label: nb.narrow ? "↻" : "↻ Restart"; tip: "Restart the kernel: its variables are lost (Ctrl+Alt+R)"; onClicked: nb.restartKernel() }
            Tool { label: nb.narrow ? "⌫" : "Clear"; tip: "Clear every cell's output (Ctrl+Alt+X)"; onClicked: nb.clearOutputs() }
            Tool {
                id: kernelsTool
                visible: nb.runningKernels.length > 0
                label: (nb.narrow ? "\u2699 " : "Kernels ") + nb.runningKernels.length
                tip: "The kernels running (every notebook's): see, restart or stop them (Ctrl+Alt+K)"
                onClicked: { nb.refreshKernels(); kernelsPop.open() }
            }
            Tool {
                visible: nb.hostJobs.length > 0
                label: (nb.narrow ? "\u29d7 " : "Jobs ") + nb.jobsActive + (nb.hostJobs.length > nb.jobsActive ? "/" + nb.hostJobs.length : "")
                tip: "Background jobs (running/all): their output, status, Stop (Ctrl+Alt+J)"
                onClicked: { nb.refreshJobs(); jobsPop.open() }
            }
            Rectangle { width: 1; height: 14; color: Theme.divider }
            Tool { label: nb.narrow ? "+" : "+ Code"; tip: "Add a code cell below (Ctrl+Alt+B)"; onClicked: nb.insertCell(nb.current + 1, "code") }
            Tool { label: nb.narrow ? "+M↓" : "+ Markdown"; tip: "Add a Markdown cell below (Ctrl+Alt+M)"; onClicked: nb.insertCell(nb.current + 1, "markdown") }
            Item { Layout.fillWidth: true }
            // The kernel: which, and how it's doing.
            Rectangle {
                width: 8; height: 8; radius: 4
                color: nb.viaShell || nb.kernelState === "idle" ? nb.ansi(2, "#98c379")
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
                ToolTip.text: nb.viaShell ? "ready" : nb.kernelState === "" ? "not started (starts with the first run)" : nb.kernelState
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
                ToolTip.text: "What runs the code cells: this pane's shell, or a Jupyter kernel (Ctrl+Alt+P)"
            }
            Tool {
                visible: nb.kernelName === "tessh" || nb.kernelName.endsWith("-tessh")
                label: nb.narrow ? "?" : "Guide"
                tip: "The tessh guide, in a tab: every feature, with examples (F1)"
                onClicked: nb.openGuide()
            }
            Tool {
                label: nb.path === "" ? (nb.narrow ? "Save…" : "Save as .ipynb…") : nb.dirty ? "Save" : "Saved"
                tip: (nb.path === "" ? "Save as a Jupyter notebook (a file to run again or share)" : "Save the notebook") + " (Ctrl+S in the notebook)"
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
        // (Not `visible ?`: that's false in a tab in the background too, and
        // the notebook mustn't change height there.)
        readonly property bool wanted: bannerText.text !== ""
        visible: wanted
        height: wanted ? bannerText.implicitHeight + 14 : 0
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
    // (Over the list, clipped to it: where the live terminal goes; it follows
    // its card as the list scrolls.)
    Item {
        id: liveClip
        z: 50
        anchors.fill: list
        clip: true
        visible: nb.liveSlot !== null
        Item { id: liveHost; visible: false }
    }
    Connections { target: list; function onContentYChanged() { if (nb.liveWanted) nb.placeLive() } }
    // (An expanded card stays where it fills the list as the others change.)
    Connections {
        target: list
        enabled: nb.expandedId !== ""
        function onContentHeightChanged() { const i = nb.indexOf(nb.expandedId); if (i >= 0) Qt.callLater(() => list.positionViewAtIndex(i, ListView.Beginning)) }
    }
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
        topMargin: nb.expandedId !== "" ? 4 : nb.path === "" ? Math.max(10, height - contentHeight - bottomMargin) : 10
        bottomMargin: nb.expandedId !== "" ? 4 : 10
        spacing: 12
        model: cells
        clip: true
        focus: true
        cacheBuffer: 1500
        interactive: false   // (a drag selects text; the wheel, touchpad and bar scroll)
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { id: listBar; visible: nb.expandedId === ""; onPressedChanged: if (pressed) nb.sticking = false }
        // (A wheel or touchpad scroll up lets go of the end; cards laying out don't.)
        MomentumWheel { id: listWheel; flick: list; enabled: nb.expandedId === "" }
        Connections { target: listWheel; function onWheel(w) { if (w.angleDelta.y > 0 || w.pixelDelta.y > 0) nb.sticking = false } }
        property real lastY: 0
        property real lastH: 0
        onContentHeightChanged: if (nb.sticking) { nb.pin(); Qt.callLater(nb.pin) }
        onHeightChanged: if (nb.sticking) { nb.pin(); Qt.callLater(nb.pin) }
        onContentYChanged: {
            // (Scrolled up by hand lets go; cards being laid out don't.)
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
            required property string ranIn     // what ran it, as it was then (a shell's name, a kernel's)
            required property string viewPath  // a file card: the file it shows
            required property string viewLang  // and the language picked for it ("": by its name, "-": none)
            required property string execs     // what its programs ran with: [{command, line, argv, cwd, user, env}]
            required property int job          // a background job running it (tesserad's id), or 0
            readonly property var ask: nb.asking[cid] || null
            readonly property var langKernel: lang !== "" ? nb.langKernels[lang] || null : null
            readonly property bool selected: nb.current === index
            readonly property bool code: type !== "markdown"
            readonly property bool fromTerm: tid > 0
            readonly property var outs: { try { return JSON.parse(outputs) } catch (e) { return [] } }
            // (A tessh cell that's an ls or cd: Tessera's own folder view, as a shell cell's.)
            readonly property string kind: fromTerm ? nb.shownKind(cid, tkind, source) : nb.tesshFiles(cid, lang, source) ? "files" : ""
            // (A terminal app running in it counts: it needs the card's room.)
            readonly property bool hasOutput: code && (fromTerm || outs.length > 0 || viewPath !== "" || kind === "files" || (nb.liveWanted && index === nb.liveIndex))
            readonly property bool expanded: nb.expandedId === cid
            readonly property bool plainMd: !code && !editing
            width: list.width - 12
            // (Its own say, not `visible`, which a tab in the background turns
            // off too: its cards mustn't fold to nothing there, or every one is
            // laid out again and the list rebuilt when the tab comes back.
            // One card expanded fills the list, held there; the others stay as
            // they are, out of sight: folding them all to nothing brought every
            // card into the list at once, and Expand took a second.)
            readonly property bool wanted: true
            visible: wanted
            height: !wanted ? 0 : (plainMd ? mdBody.height + 8 : header.height + (editing ? editorBox.height + 8 : 0)
                     + (hasOutput ? (hideOutput ? 22 : body.height + (body.live ? 14 : 12)) : 4) + (ask ? askBox.height + 6 : 0))
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
                // (Taller when a path in it needs two lines: paths are shown whole.)
                height: Math.max(26, implicitHeight + 6)
                spacing: 6
                // Status; a click runs it (or stops it). (The spinner turns on its
                // own, so the mark after it is never left at an angle.)
                Text { topPadding: 0.001;
                    text: cell.viewPath !== "" ? "\u2261" : cell.running ? "" : cell.ended > 0 || cell.fromTerm ? (cell.failed ? "✘" : "✓") : cell.code ? "▶" : "M↓"
                    color: cell.running ? Theme.accent : cell.ended > 0 || cell.fromTerm ? (cell.failed ? nb.ansi(1, "#e06c75") : nb.ansi(2, "#98c379"))
                         : statusHover.hovered ? Theme.accent : Theme.textDim
                    font.pixelSize: 13
                    font.weight: Font.Bold
                    Layout.minimumWidth: 14
                    horizontalAlignment: Text.AlignHCenter
                    Text {
                        anchors.centerIn: parent
                        visible: cell.running
                        text: "◌"
                        color: Theme.accent
                        font.pixelSize: 13
                        font.weight: Font.Bold
                        RotationAnimation on rotation { running: cell.running; from: 0; to: 360; duration: 1200; loops: Animation.Infinite }
                    }
                    HoverHandler { id: statusHover; cursorShape: Qt.PointingHandCursor; onHoveredChanged: if (hovered && cell.failed) nb.askExit(cells.get(cell.index), cell.index) }
                    TapHandler { onTapped: { nb.current = cell.index; cell.running ? (cell.job > 0 ? nb.stopJob(cell.index) : nb.interrupt()) : nb.run(cell.index) } }
                    ToolTip.visible: statusHover.hovered
                    ToolTip.delay: 400
                    // (Done: its exit status, and what it means for the command.)
                    ToolTip.text: cell.viewPath !== "" ? "A file: click to read it again"
                        : cell.running ? "Interrupt"
                        : (cell.ended > 0 || cell.fromTerm) && cell.code ? nb.exitTip(cell) + "\nClick to run it again"
                        : cell.code ? "Run (Ctrl+Enter)" : "Show it (Ctrl+Enter)"
                }
                // (What runs it: the shell, a language's kernel (and it coming
                // up), the notebook's kernel.)
                Text { topPadding: 0.001;
                    visible: cell.code && text !== ""
                    text: nb.ranBy(cell) + (cell.running && cell.langKernel && cell.langKernel.state === "starting" ? " \u00b7 " + (cell.langKernel.message || ("starting " + (cell.langKernel.display || "its kernel") + "\u2026")) : "")
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
                    // (A file card: its whole path, wrapped if need be, never cut short.)
                    // (Another host's: its name as typed, not URL-encoded.)
                    text: cell.viewPath !== "" ? cell.viewPath.replace(/^ssh:\/\/([^\/]*)/, (m, h) => { try { return "ssh://" + decodeURIComponent(h) } catch (e) { return m } })
                        : cell.editing ? (cell.code ? "" : "Markdown") : cell.source.trim() !== "" ? cell.source.replace(/^\s*\n|\s+$/g, "") : "(empty)"
                    // (A command wraps too, so a path in it shows whole; code of
                    // several lines shows as its lines, indented as written,
                    // the rest in the cell past forty.)
                    maximumLineCount: cell.source.trim().indexOf("\n") >= 0 ? 40 : 6
                    wrapMode: cell.viewPath !== "" ? Text.WrapAnywhere : Text.WrapAtWordBoundaryOrAnywhere
                    elide: cell.viewPath !== "" ? Text.ElideNone : Text.ElideRight
                    color: cell.editing ? Theme.textDim : Theme.text
                    font.family: nb.fontFamily
                    font.pointSize: 9
                    font.weight: Font.DemiBold
                    font.underline: srcHover.hovered && !cell.editing
                    renderType: Text.NativeRendering
                    HoverHandler { id: srcHover; cursorShape: Qt.IBeamCursor }
                    TapHandler { onTapped: if (cell.viewPath === "") nb.focusCell(cell.index, true) }
                }
                // (The folder it ran in, whole: wrapped when there's no room on
                // one line. A file card's path has its folder already.)
                Text { topPadding: 0.001; visible: cell.cwd !== "" && cell.viewPath === ""; text: cell.cwd; color: Theme.textDim; font.pixelSize: 10
                       wrapMode: Text.WrapAnywhere; Layout.maximumWidth: Math.max(120, header.width * 0.4) }
                Text { topPadding: 0.001;
                    visible: cell.started > 1
                    text: (cell.ended > 0 ? nb.seconds(cell.ended - cell.started) + "  ·  " : "") + Qt.formatTime(new Date(cell.started), "hh:mm")
                          + (cell.ended > 0 && cell.fromTerm && cell.status !== 0 ? "  ·  exit " + cell.status : "")
                    color: Theme.textDim
                    font.pixelSize: 10
                }
                // (A file card, editing: save, save as, read again, done; ● unsaved.)
                readonly property var fileView: cell.viewPath !== "" && loader.item && loader.item.editing !== undefined ? loader.item : null
                // (Edit; while editing, Save (written, back to viewing) and
                // Discard (the changes dropped, back to viewing). Ctrl+S saves and
                // stays. The keys button comes next.)
                Chip {
                    visible: header.fileView !== null && !header.fileView.editing && !header.fileView.inTerm
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: "✎ Edit"
                    tip: "Edit the file (in a terminal editor: ⋯)"
                    onClicked: header.fileView.startEdit()
                }
                // (The file open in a terminal editor in the card: quit it there, or here.)
                Chip {
                    visible: header.fileView !== null && header.fileView.inTerm
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: "✕ Close the editor"
                    tip: "Close the terminal editor (anything it hasn't saved is lost)"
                    onClicked: header.fileView.closeTerminal()
                }
                Chip {
                    visible: header.fileView !== null && header.fileView.editing
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: "✓ Save"
                    tip: "Save the file and go back to viewing (Ctrl+S saves and stays)"
                    onClicked: { const v = header.fileView; if (v.save()) v.stopEdit() }
                }
                Chip {
                    visible: header.fileView !== null && header.fileView.editing
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: "✕ Discard"
                    tip: header.fileView && header.fileView.dirty ? "Drop the changes and go back to viewing" : "Back to viewing"
                    onClicked: header.fileView.stopEdit()
                }
                // (Editing: the keys it edits with, in sight (vi's mode too), ●
                // when there are changes; a click picks other keys. Saving and
                // the rest are in the ⋯ menu.)
                Chip {
                    id: keysChip
                    visible: header.fileView !== null && header.fileView.editing
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: (header.fileView && header.fileView.dirty ? "● " : "") + ({ emacs: "Emacs", vscode: "VS Code", vi: "vi" })[nb.editorKeys]
                           + (nb.editorKeys === "vi" && header.fileView ? (header.fileView.viNormal ? " · NORMAL" : " · INSERT") : "") + " ▾"
                    tip: "Editing keys (" + (header.fileView && header.fileView.dirty ? "unsaved changes; " : "") + "save from ⋯)"
                    onClicked: keysMenu.popup(keysChip, 0, keysChip.height + 2)
                }
                SlimMenu {
                    id: keysMenu
                    SlimMenuItem { text: "Emacs"; checkable: true; checked: nb.editorKeys === "emacs"; onTriggered: nb.editorKeysPicked("emacs") }
                    SlimMenuItem { text: "VS Code"; checkable: true; checked: nb.editorKeys === "vscode"; onTriggered: nb.editorKeysPicked("vscode") }
                    SlimMenuItem { text: "vi"; checkable: true; checked: nb.editorKeys === "vi"; onTriggered: nb.editorKeysPicked("vi") }
                }
                // (A file card: the language it's highlighted as, to change.)
                Chip {
                    id: typeChip
                    visible: cell.viewPath !== ""
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: (nb.isTableFile(cell.viewPath) && cell.viewLang === "" ? "Table" : cell.viewLang === "-" ? "Plain text" : loader.item && loader.item.lang ? loader.item.lang : "Plain text") + (cell.viewLang === "" ? " · auto" : "") + " ▾"
                    tip: "Highlight it as…"
                    onClicked: nb.pickFileType(cell.index, typeChip)
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
                // (A background job: its pid, and Stop while it runs.)
                Chip { visible: cell.job > 0; base: cell.color
                       label: "bg" + (cell.running ? " \u00b7 pid " + (((nb.parseList(cell.execs)[0] || {}).pid) || "") : "")
                       tip: "Running in the background (it goes on when the window closes): see every job"
                       Layout.minimumWidth: implicitWidth; onClicked: { nb.refreshJobs(); jobsPop.open() } }
                Chip { visible: cell.job > 0 && cell.running; base: cell.color; label: "\u25a0 Stop"; tip: "Stop it (TERM, then KILL after 3 s)"
                       Layout.minimumWidth: implicitWidth; onClicked: nb.stopJob(cell.index) }
                // (What it ran with: command line, folder, user, environment.)
                Chip { visible: cell.code && cell.viewPath === "" && (cell.fromTerm || cell.ended > 0 || cell.running || cell.execs !== "[]")
                       base: cell.color; label: "\u24d8"; tip: "Details: the command line as run, folder, user and environment"
                       Layout.minimumWidth: implicitWidth; onClicked: nb.showExec(cell.index) }
                Chip { visible: cell.hasOutput; base: cell.color; label: "Copy"; tip: "Copy the output"; Layout.minimumWidth: implicitWidth
                       onClicked: { const c = cells.get(cell.index)
                                    // (A file card's text: read now, then copied.)
                                    if (c && c.tid <= 0 && c.viewPath !== "") nb.fileText(c.viewPath, t => nb.copyText(t))
                                    else nb.copyText(nb.outputText(cell.index)) } }
                Chip { visible: cell.code && !cell.running && cell.source.trim() !== ""; base: cell.color; label: "↻"; tip: "Run it again"; Layout.minimumWidth: implicitWidth
                       onClicked: { nb.current = cell.index; nb.run(cell.index) } }
                Chip {
                    // (A terminal app running in it counts as output.)
                    visible: cell.hasOutput || cell.expanded || (nb.liveWanted && cell.index === nb.liveIndex)
                    base: cell.color
                    Layout.minimumWidth: implicitWidth
                    label: cell.expanded ? "⤡ Restore" : "⤢ Expand"
                    tip: (cell.expanded ? "Back to its own size" : "Fill the window with this output") + " (Ctrl+Alt+F)"
                    onClicked: nb.toggleExpand(cell.index)
                }
                Chip { id: moreChip; base: cell.color; label: "⋯"; tip: "More"; Layout.minimumWidth: implicitWidth
                       onClicked: moreMenu.popup(moreChip, 0, moreChip.height + 2) }
                // (Test hook: TESSERA_TEST_EXPAND_CELL fills the window with the first cell.)
                Timer { interval: 2500; running: cell.index === 0 && nb.controller !== null && nb.controller.testEnv("EXPAND_CELL") !== ""; onTriggered: nb.expandedId = cell.cid }
                // (Test hook: TESSERA_TEST_OPEN_MORE_MENU opens the first card's ⋯ menu.)
                Timer { interval: 13000; running: cell.index === 0 && nb.controller !== null && nb.controller.testEnv("OPEN_MORE_MENU") !== ""; onTriggered: moreMenu.popup(moreChip, 0, moreChip.height + 2) }
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
                // (A file card: the file's own; a cell: its code.)
                SlimMenuItem { text: "Edit the file"; visible: header.fileView !== null && !header.fileView.editing && !header.fileView.inTerm; onTriggered: header.fileView.startEdit() }
                // (A Markdown file: as a page, or its text.)
                SlimMenuItem { text: nb.kinds[cell.cid] === "mdtext" ? "Show it rendered" : "Show the source"; visible: header.fileView !== null && !header.fileView.editing && nb.isMarkdownFile(cell.viewPath); height: visible ? implicitHeight : 0
                               onTriggered: { const k = Object.assign({}, nb.kinds); if (k[cell.cid] === "mdtext") delete k[cell.cid]; else k[cell.cid] = "mdtext"; nb.kinds = k; nb.touch() } }
                SlimMenuItem { text: "Edit in " + (header.fileView ? nb.termEditorName(header.fileView.termEditor) : ""); visible: header.fileView !== null && !header.fileView.inTerm; onTriggered: header.fileView.openInTerminal() }
                SlimMenu {
                    title: "Edit in another terminal editor ▸"
                    enabled: header.fileView !== null && !header.fileView.inTerm
                    topPadding: 4.001
                    Repeater {
                        model: header.fileView ? header.fileView.termEditors : []
                        delegate: SlimMenuItem {
                            required property var modelData
                            text: modelData[1]
                            checkable: true
                            checked: header.fileView !== null && header.fileView.termEditor === modelData[0]
                            onTriggered: header.fileView.openInTerminal(modelData[0])
                        }
                    }
                }
                SlimMenuItem { text: "Close the terminal editor"; visible: header.fileView !== null && header.fileView.inTerm; onTriggered: header.fileView.closeTerminal() }
                SlimMenuItem { text: "Find and replace…\tCtrl+F"; visible: header.fileView !== null && header.fileView.editing; onTriggered: Qt.callLater(header.fileView.showFind) }
                SlimMenuItem { text: "Go to line…\tCtrl+G"; visible: header.fileView !== null && header.fileView.editing; onTriggered: Qt.callLater(header.fileView.showGoTo) }
                SlimMenuItem { text: "Undo\tCtrl+Z"; visible: header.fileView !== null && header.fileView.editing; enabled: header.fileView !== null && header.fileView.canUndo; onTriggered: header.fileView.undo() }
                SlimMenuItem { text: "Redo\tCtrl+Shift+Z"; visible: header.fileView !== null && header.fileView.editing; enabled: header.fileView !== null && header.fileView.canRedo; onTriggered: header.fileView.redo() }
                SlimMenuItem { text: "Wrap long lines"; checkable: true; checked: header.fileView !== null && header.fileView.wrap; visible: header.fileView !== null && header.fileView.editing; onTriggered: header.fileView.wrap = !header.fileView.wrap }
                SlimMenuItem { text: "Windows line ends (CRLF)"; checkable: true; checked: header.fileView !== null && header.fileView.crlf; visible: header.fileView !== null && header.fileView.editing; onTriggered: header.fileView.crlf = !header.fileView.crlf }
                SlimMenuItem { text: header.fileView && header.fileView.toolsShown ? "Hide the edit bar" : "Show the edit bar"; visible: header.fileView !== null && header.fileView.editing; onTriggered: header.fileView.toolsShown = !header.fileView.toolsShown }
                SlimMenuItem { text: "Save\tCtrl+S"; visible: header.fileView !== null && header.fileView.editing; onTriggered: header.fileView.save() }
                SlimMenuItem { text: "Save as…"; visible: header.fileView !== null && header.fileView.editing
                               onTriggered: { nb.saveAsCell = cell.index; fileSaveDialog.currentFolder = nb.saveFolder(cell.viewPath); fileSaveDialog.open() } }
                SlimMenuItem { text: "Discard changes"; visible: header.fileView !== null && header.fileView.editing; enabled: header.fileView !== null && header.fileView.dirty; onTriggered: header.fileView.reload() }
                SlimMenuItem { text: header.fileView && header.fileView.dirty ? "Done, discarding the changes" : "Done editing"; visible: header.fileView !== null && header.fileView.editing; onTriggered: header.fileView.stopEdit() }
                SlimMenuItem { text: "\u24d8 Details (command line, environment)"; visible: cell.code && cell.viewPath === ""; onTriggered: nb.showExec(cell.index) }
                SlimMenuItem { text: cell.code ? "Edit" : "Edit the Markdown"; visible: cell.viewPath === ""; onTriggered: nb.focusCell(cell.index, true) }
                SlimMenuItem { text: "Run in the background"; visible: cell.code && cell.viewPath === "" && !cell.running && nb.bgLang(cells.get(cell.index)) !== ""; onTriggered: nb.run(cell.index, true) }
                SlimMenuItem { text: "Stop the background job"; visible: cell.job > 0 && cell.running; onTriggered: nb.stopJob(cell.index) }
                SlimMenuItem { text: "Run the cells above"; visible: cell.code && cell.viewPath === ""; onTriggered: nb.runAbove(cell.index) }
                SlimMenuItem { text: cell.code ? "Make it Markdown" : "Make it code"; visible: cell.viewPath === ""; onTriggered: nb.setType(cell.index, cell.code ? "markdown" : "code") }
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
                height: cell.editing ? editorLoader.height + 2 : 0
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
                height: cell.plainMd ? Math.max(24, mdView.implicitHeight) : 0
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
                anchors.top: cell.editing ? editorBox.bottom : header.bottom
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
                objectName: "cardBody"
                readonly property var viewer: loader.item
                // (The card a full-screen program runs in: its terminal here.)
                readonly property bool live: nb.liveWanted && cell.index === nb.liveIndex
                readonly property bool wanted: (cell.hasOutput || live) && !cell.hideOutput
                visible: wanted
                anchors.top: cell.editing ? editorBox.bottom : header.bottom
                // (A terminal app: as much room above it as under it, where
                // the grip is.)
                anchors.topMargin: cell.editing || live ? 6 : 0
                x: 10
                width: parent.width - 20
                // Its natural size up to 460 px, or what it was dragged to, or the
                // whole window when expanded.
                readonly property real itemH: loader.item ? loader.item.implicitHeight : 0
                // (Its first moments: no smaller than it last was, while what
                // it shows loads; after that, whatever it is.)
                property bool fresh: true
                Timer { running: body.fresh; interval: 1500; onTriggered: body.fresh = false }
                readonly property real natural: fresh ? Math.max(itemH, nb.seenH[cell.cid] || 0) : itemH
                onItemHChanged: if (!fresh || itemH >= (nb.seenH[cell.cid] || 0)) nb.seenH[cell.cid] = itemH
                height: !wanted ? 0 : cell.expanded ? Math.max(80, list.height - list.topMargin - list.bottomMargin - header.height - 12 - (cell.editing ? editorBox.height + 8 : 0))
                      // (A terminal app running in it: room for it, whatever
                      // size the card was dragged to before.)
                      : live ? nb.liveFit(Math.max(nb.heights[cell.cid] || 0, nb.liveHeight))
                      : nb.heights[cell.cid] > 0 ? nb.heights[cell.cid]
                      : Math.min(Math.max(natural, 16), 460)
                clip: true
                Item {
                    id: liveBox
                    anchors.fill: parent
                    visible: body.live
                    z: 4
                    onVisibleChanged: { if (visible) { nb.liveBox = liveBox; Qt.callLater(nb.followLive) } else if (nb.liveBox === liveBox) nb.liveBox = null }
                    onWidthChanged: if (visible) Qt.callLater(nb.placeLive)
                    onHeightChanged: if (visible) Qt.callLater(nb.placeLive)
                    Component.onDestruction: if (nb.liveBox === liveBox) nb.liveBox = null
                    // (A card made again (the list drops and remakes cards as
                    // Expand/Restore hide and show the others) that shows it
                    // from the start: no visible change comes, and the
                    // terminal would stay hidden, black.)
                    Component.onCompleted: if (visible) { nb.liveBox = liveBox; Qt.callLater(nb.followLive) }
                }
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
                    property string viewPath: cell.viewPath
                    property string viewLang: cell.viewLang
                    readonly property string kind: cell.kind
                    sourceComponent: cell.viewPath !== "" ? (nb.isTableFile(cell.viewPath) && cell.viewLang === "" ? tableCardView : fileCardView)
                                   : !cell.fromTerm ? (cell.kind === "files" ? filesView : jupyterOutputs)
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
                anchors.top: body.wanted ? body.bottom : header.bottom
                anchors.topMargin: 4
                x: 12
                width: parent.width - 24
                height: cell.ask !== null ? 26 : 0
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
        // (Not while a card is expanded: it fills the notebook, and this under
        // it pushed its header, with Restore, off the top.)
        footer: Item {
            width: list.width
            height: cells.count > 0 && nb.expandedId === "" ? 40 : 0
            Row {
                anchors.centerIn: parent
                spacing: 8
                visible: cells.count > 0 && nb.expandedId === ""
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
    // A file card's text, read in the background and kept by path: what was
    // read last until the new read is in (`done` gets the new one).
    property var fileTexts: ({})
    property var fileAsking: ({})       // (changed in place: no binding depends on it)
    function fileText(p, done) {
        if (fileAsking[p]) { if (done) fileAsking[p].push(done) }
        else {
            fileAsking[p] = done ? [done] : []
            Qt.callLater(() => {
                const answer = (text) => {
                    let t = ""
                    try { t = JSON.parse(text).text || "" } catch (e) {}
                    const m = Object.assign({}, fileTexts); m[p] = t; fileTexts = m
                    const w = fileAsking[p] || []
                    delete fileAsking[p]
                    for (const f of w) f(t)
                }
                if (!controller) { answer(""); return }
                Async.call(controller, k => controller.readTextFile(k, p), answer)
            })
        }
        return fileTexts[p] || ""
    }
    function copyText(t) { clip.text = t; clip.selectAll(); clip.copy() }
    function outputText(i) {
        const c = cells.get(i)
        if (!c) return ""
        if (c.tid > 0) return view ? view.cellOutput(c.tid, "text") : ""
        // (A file card: its text, the first 3 MB at most of a big one.)
        if (c.viewPath !== "") return fileText(c.viewPath)
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
            // A new line, indented like this one (more after a ":"; in tessh,
            // after a block's head, and an else goes back to its if).
            function newline() {
                if (lang === "tessh") {
                    const s0 = edit.text.lastIndexOf("\n", edit.cursorPosition - 1) + 1
                    const m = /^( +)(else|elif|except\*?|finally)\b/.exec(edit.text.slice(s0, edit.cursorPosition))
                    if (m) {
                        const want = nb.tesshAlign(edit.text.slice(0, Math.max(0, s0 - 1)), m[2], m[1].length)
                        if (want < m[1].length) edit.remove(s0, s0 + m[1].length - want)
                    }
                }
                const before = edit.text.slice(0, edit.cursorPosition)
                const line = before.slice(before.lastIndexOf("\n") + 1)
                let ind = indentOf(line)
                if (lang === "python" && /:\s*$/.test(line)) ind += "    "
                if (lang === "tessh" && nb.tesshOpens(line)) ind += "  "
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
                    if (edit.selectedText === "" && /[\w.\]\)'"]$/.test(before) && !nb.viaShell && nb.kernelUp) comp.ask()
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
    // A path in the pane's shell's world: on another host, "ssh://<host>/...",
    // which the controller reads there.
    // Where Save as… starts for a card's file: its folder, or home for
    // another host's file (the dialog shows this computer's folders; the
    // copy is saved here).
    // Another host's home folder, asked in the background and kept by host
    // ("" until it's in; one not connected yet is asked again next time).
    property var hostHomes: ({})
    property var homeAsking: ({})       // (changed in place: no binding depends on it)
    function homeOn(h) {
        if (!homeAsking[h] && !hostHomes[h]) {
            homeAsking[h] = true
            Qt.callLater(() => {
                if (!controller) { delete homeAsking[h]; return }
                Async.call(controller, k => controller.hostHome(k, h), home => {
                    delete homeAsking[h]
                    if (home === "") return
                    const m = Object.assign({}, hostHomes); m[h] = home; hostHomes = m
                })
            })
        }
        return hostHomes[h] || ""
    }
    function saveFolder(p) { return p.startsWith("ssh://") ? "file://" + (controller ? controller.homeDir() : "") : "file://" + p.replace(/\/[^\/]*$/, "") }
    function onHost(p) { return sshHost === "" || p.startsWith("ssh://") ? p : "ssh://" + encodeURIComponent(sshHost) + p }
    function localPath(p) { let x = (p || "").replace(/^file:\/\/[^/]*/, ""); try { x = decodeURIComponent(x) } catch (e) {} return x }
    function termEditorName(e) { return e === "" ? "your $EDITOR" : ({ vim: "Vim", nvim: "Neovim", "emacs -nw": "Emacs", hx: "Helix", kak: "Kakoune" })[e] || e }
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
                const base = nb.localPath(src.cwd) || nb.shellDir
                const home = nb.controller ? (nb.sshHost !== "" ? nb.homeOn(nb.sshHost) : nb.controller.homeDir()) : "~"
                const a = args.find(x => !x.startsWith("-"))
                // (cd: where it went. With no folder, home; "cd -" and z: wherever the shell is now.)
                if (isCd && (!a || a === "-" || words[0] === "z" || words[0] === "zi")) return nb.onHost(a || words[0] !== "cd" ? nb.shellDir : home)
                if (!a) return nb.onHost(base)
                const p = a.replace(/^['"]|['"]$/g, "").replace(/^~(?=\/|$)/, home)
                return nb.onHost(p.startsWith("/") ? p : base + "/" + p)
            }
            refresh: src.tick ? src.tick.length : 0
            kept: nb.fileViews[src.cid] || null
            onKeptChanged2: (c) => { const m = Object.assign({}, nb.fileViews); m[src.cid] = c; nb.fileViews = m; nb.touch() }
            onCommandRequested: (c) => nb.commandRequested(c)
            onInsertRequested: (t) => nb.insertRequested(t)
            onViewRequested: (p) => nb.viewRequested(p)
            onFileCardRequested: (p) => nb.addFileCard(p)
        }
    }

    // A file card's language: a searchable list (Helix's hundreds), with
    // Automatic (by its name) and Plain text first.
    readonly property var syntaxLanguages: { try { return JSON.parse(controller.syntaxLanguages()) } catch (e) { return [] } }
    property int typeFor: -1
    function pickFileType(i, chip) {
        typeFor = i
        const p = chip.mapToItem(nb, 0, chip.height + 2)
        typePop.x = Math.max(4, Math.min(p.x, nb.width - typePop.width - 4))
        typePop.y = Math.min(p.y, nb.height - typePop.height - 4)
        typePop.open()
    }
    // (Test hook: TESSERA_TEST_VIEWLANG=<lang>: the last card's picked at 12 s.)
    Timer { interval: 12000; running: nb.controller !== null && nb.controller.testEnv("VIEWLANG") !== ""; onTriggered: { nb.typeFor = cells.count - 1; typePop.pick(nb.controller.testEnv("VIEWLANG")) } }
    Popup {
        id: typePop
        width: 260
        height: Math.min(380, typeList.contentHeight + typeFilter.height + 24)
        padding: 6
        topPadding: 6.001   // (off the half pixel: 130% scale smears native text there)
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        onOpened: { typeFilter.text = ""; typeFilter.forceActiveFocus() }
        readonly property var options: [["", nb.typeFor >= 0 && nb.typeFor < cells.count && nb.isTableFile(cells.get(nb.typeFor).viewPath) ? "Table (CSV/TSV)" : "Automatic (by its name)"], ["-", "Plain text"]].concat(nb.syntaxLanguages.filter(l => l[0] !== ""))
        readonly property var shown: options.filter(o => typeFilter.text === "" || (o[1] + " " + o[0]).toLowerCase().indexOf(typeFilter.text.toLowerCase()) >= 0)
        function pick(id) {
            if (nb.typeFor >= 0 && nb.typeFor < cells.count) { cells.setProperty(nb.typeFor, "viewLang", id); nb.touch() }
            typePop.close()
        }
        contentItem: Column {
            spacing: 4
            TextField {
                id: typeFilter
                width: parent.width
                placeholderText: "Search " + nb.syntaxLanguages.length + " languages"
                color: Theme.text
                font.pixelSize: 12
                background: Rectangle { radius: 6; color: Theme.bg; border.color: typeFilter.activeFocus ? Theme.accent : Theme.divider }
                Keys.onReturnPressed: if (typePop.shown.length > 0) typePop.pick(typePop.shown[0][0])
                Keys.onEscapePressed: typePop.close()
            }
            ListView {
                id: typeList
                width: parent.width
                height: Math.min(320, contentHeight)
                clip: true
                model: typePop.shown
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}
                MomentumWheel { flick: typeList }
                delegate: Rectangle {
                    required property var modelData
                    width: typeList.width
                    height: 24
                    radius: 5
                    readonly property bool picked: nb.typeFor >= 0 && nb.typeFor < cells.count && cells.get(nb.typeFor).viewLang === modelData[0]
                    color: th.hovered ? Theme.tabHover : picked ? Theme.mix(Theme.bg, Theme.accent, 0.25) : "transparent"
                    Text { x: 8; anchors.verticalCenter: parent.verticalCenter; anchors.verticalCenterOffset: 0.001; text: modelData[1]; color: Theme.text; font.pixelSize: 12; renderType: Text.NativeRendering }
                    HoverHandler { id: th; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: typePop.pick(modelData[0]) }
                }
            }
        }
    }

    function parseList(s) { try { return JSON.parse(s) } catch (e) { return [] } }
    function ago(ms) {
        const s = Math.max(0, Math.round(ms / 1000))
        return s < 60 ? s + " s" : s < 3600 ? Math.floor(s / 60) + " min " + (s % 60) + " s" : Math.floor(s / 3600) + " h " + Math.floor(s % 3600 / 60) + " min"
    }
    // ---- every background job on this host ----
    Popup {
        id: jobsPop
        parent: nb
        x: Math.max(8, nb.width - width - 8)
        y: 36
        width: Math.min(nb.width - 16, 640)
        height: Math.min(nb.height - 50, implicitHeight)
        padding: 8
        topPadding: 8.001   // (off the half pixel: 130% scale smears native text there)
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        property real now: Date.now()
        Timer { interval: 1000; repeat: true; running: jobsPop.opened; onTriggered: jobsPop.now = Date.now() }
        contentItem: ColumnLayout {
            spacing: 4
            RowLayout {
                Layout.fillWidth: true
                Text { text: "Background jobs" + (nb.sshHost !== "" ? " on " + nb.sshHost : ""); color: Theme.text; font.pixelSize: 13; font.weight: Font.DemiBold; Layout.fillWidth: true }
                Chip { base: Theme.panel; label: "Clear finished"; visible: nb.hostJobs.some(j => !j.running)
                       onClicked: { for (const j of nb.hostJobs) if (!j.running) nb.controller.jobCall("", nb.sshHost, "job.forget", JSON.stringify({ id: j.id })); Qt.callLater(nb.refreshJobs) } }
            }
            Text {
                visible: nb.hostJobs.length === 0
                text: "None. End a shell or tessh line with & (or \u22ef \u2192 Run in the background) to run it on its own."
                color: Theme.textDim; font.pixelSize: 12; wrapMode: Text.Wrap; Layout.fillWidth: true
            }
            ListView {
                id: jobsList
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(contentHeight, 360)
                clip: true
                model: nb.hostJobs.slice().reverse()
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}
                delegate: RowLayout {
                    required property var modelData
                    readonly property bool mine: modelData.owner === nb.key && nb.indexOf(modelData.cell) >= 0
                    width: jobsList.width - 10
                    height: 28
                    spacing: 6
                    Text {
                        text: modelData.running ? "\u25cf" : modelData.status === 0 ? "\u2713" : "\u2718"
                        color: modelData.running ? Theme.accent : modelData.status === 0 ? nb.ansi(2, "#98c379") : nb.ansi(1, "#e06c75")
                        font.pixelSize: 12; font.weight: Font.Bold; topPadding: 0.001
                    }
                    Text {
                        text: modelData.command
                        color: Theme.text; font.family: nb.fontFamily; font.pixelSize: 12; topPadding: 0.001
                        elide: Text.ElideRight; Layout.fillWidth: true
                    }
                    Text {
                        text: "pid " + modelData.pid + " \u00b7 " + (modelData.running ? nb.ago(jobsPop.now - modelData.started) : "exit " + modelData.status + " \u00b7 " + nb.ago(modelData.ended - modelData.started))
                        color: Theme.textDim; font.pixelSize: 11; topPadding: 0.001
                    }
                    Chip { base: Theme.panel; label: "Show"; visible: mine; tip: "Go to its card"; onClicked: { nb.showJobCell(modelData.cell); jobsPop.close() } }
                    Chip { base: Theme.panel; label: "\u24d8"; tip: "What it ran with"
                           onClicked: Async.call(nb.controller, k => nb.controller.jobCall(k, nb.sshHost, "job.output", JSON.stringify({ id: modelData.id, from: 1e15, exec: true })), json => {
                               let r = {}
                               try { r = JSON.parse(json) } catch (e) {}
                               if (!r.exec) return
                               execPop.records = [r.exec]; execPop.sel = 0; execPop.note = ""; execPop.open()
                           }) }
                    Chip { base: Theme.panel; label: "\u25a0 Stop"; visible: modelData.running; onClicked: { nb.controller.jobCall("", nb.sshHost, "job.stop", JSON.stringify({ id: modelData.id })); Qt.callLater(nb.refreshJobs) } }
                }
            }
        }
    }
    // ---- a card's details: what its programs ran with ----
    // Command line (typed, and as run), folder, user and environment: a shell
    // card's from shell integration, a tessh card's from its kernel, one per
    // program it ran.
    function execRecords(i) {
        const c = cells.get(i)
        let xs = []
        try { xs = JSON.parse(c.execs) } catch (e) {}
        if (xs.length === 0 && c.tid > 0 && view) {
            try { const x = JSON.parse(view.cellOutput(c.tid, "exec") || "null"); if (x) xs = [x] } catch (e) {}
        }
        // (Not reported: what the card itself knows.)
        if (xs.length === 0) xs = [{ command: c.source, cwd: c.cwd, missing: c.tid > 0 ? "shell" : "kernel" }]
        return xs
    }
    // (Test hook: TESSERA_TEST_NB_JOBS=<ms>: the Jobs list opened then.)
    Timer { interval: nb.controller ? +nb.controller.testEnv("NB_JOBS") || 1 : 1; running: nb.controller !== null && nb.controller.testEnv("NB_JOBS") !== ""; onTriggered: { nb.refreshJobs(); jobsPop.open() } }
    // (Test hook: TESSERA_TEST_NB_DETAILS=<n>[,ms]: the nth card's details (1 the first, -1 the last) at 14 s (or ms).)
    Timer {
        readonly property var arg: nb.controller ? nb.controller.testEnv("NB_DETAILS").split(",") : [""]
        interval: arg.length > 1 ? +arg[1] : 14000
        running: arg[0] !== ""
        onTriggered: { const n = +arg[0]; nb.showExec(n > 0 ? n - 1 : cells.count + n) }
    }
    function showExec(i) {
        if (i < 0 || i >= cells.count) return
        execPop.records = execRecords(i)
        execPop.sel = Math.max(0, execPop.records.length - 1)
        execPop.note = ""
        execPop.open()
        // (A shell that didn't report it, here on this computer: its process's
        // user and environment, as it started.)
        const r = execPop.records[0]
        if (r.missing === "shell" || (cells.get(i).tid > 0 && !r.env)) {
            if (sshHost === "" && shellPid > 0) {
                const want = execPop.records
                Async.call(controller, k => controller.processExec(k, shellPid), json => {
                    let x = null
                    try { x = JSON.parse(json || "null") } catch (e) {}
                    if (!x || execPop.records !== want) return
                    execPop.records = [Object.assign({}, r, { user: x.user, env: x.env })]
                    execPop.note = "This shell's integration is from before it reported each command's: this is the shell's own environment as it started (open a new tab or run exec $SHELL for each command's own)."
                })
            } else execPop.note = "This shell's integration is from before it reported each command's environment: open a new tab (or run exec $SHELL) to get it."
        } else if (r.missing === "kernel") execPop.note = "This kernel didn't report the programs it ran (it's from before it did): restart the kernel."
        if (controller && controller.testEnv("NB_DETAILS") !== "") console.log("nb details: card " + i + " " + JSON.stringify(execPop.rec).slice(0, 200))
    }
    Popup {
        id: execPop
        parent: nb
        property var records: []
        property int sel: 0
        property string note: ""
        readonly property var rec: records[sel] || ({})
        // (The words as run: the shell's expansion, or tessh's argv.)
        readonly property string ran: rec.argv ? rec.command : (rec.line || rec.command || "")
        readonly property var envs: {
            const f = envFilter.text.toLowerCase()
            const all = (rec.env || []).map(e => { const k = e.indexOf("="); return { k: e.substring(0, k), v: e.substring(k + 1) } })
            return f === "" ? all : all.filter(x => (x.k + "=" + x.v).toLowerCase().indexOf(f) >= 0)
        }
        x: Math.round((nb.width - width) / 2)
        y: 40
        width: Math.min(nb.width - 32, 820)
        height: Math.min(nb.height - 60, implicitHeight)
        padding: 10
        topPadding: 10.001   // (off the half pixel: 130% scale smears native text there)
        modal: false
        focus: true
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        onOpened: envFilter.text = ""
        contentItem: ColumnLayout {
            spacing: 6
            RowLayout {
                Layout.fillWidth: true
                Text { text: "Details"; color: Theme.text; font.pixelSize: 13; font.weight: Font.DemiBold; Layout.fillWidth: true }
                Chip { base: Theme.panel; label: "Close"; onClicked: execPop.close() }
            }
            // (A tessh cell that ran several programs: one at a time.)
            Flow {
                Layout.fillWidth: true
                visible: execPop.records.length > 1
                spacing: 4
                Repeater {
                    model: execPop.records
                    delegate: Chip {
                        required property var modelData
                        required property int index
                        base: index === execPop.sel ? Theme.mix(Theme.bg, Theme.accent, 0.3) : Theme.panel
                        label: (index + 1) + ". " + (modelData.cell ? "the cell" : (modelData.argv && modelData.argv[0] && modelData.argv[0][0]) || (modelData.command || "").split(" ")[0])
                        tip: modelData.command || ""
                        onClicked: execPop.sel = index
                    }
                }
            }
            Text {
                visible: execPop.note !== ""
                Layout.fillWidth: true
                text: execPop.note
                wrapMode: Text.Wrap
                color: Theme.textDim
                font.pixelSize: 12
                font.italic: true
            }
            GridLayout {
                Layout.fillWidth: true
                columns: 3
                columnSpacing: 8
                rowSpacing: 4
                Text { text: execPop.rec.cell ? "Code (no programs)" : "Command line"; color: Theme.textDim; font.pixelSize: 12; Layout.alignment: Qt.AlignTop; topPadding: 3.001 }
                TextEdit {
                    objectName: "execLine"; text: execPop.ran
                    readOnly: true; selectByMouse: true; wrapMode: TextEdit.WrapAnywhere; textFormat: TextEdit.PlainText
                    color: Theme.text; selectionColor: Theme.accent; font.family: nb.fontFamily; font.pixelSize: 12
                    topPadding: 3.001; Layout.fillWidth: true
                }
                Chip { base: Theme.panel; label: "Copy"; Layout.alignment: Qt.AlignTop; onClicked: nb.copyText(execPop.ran) }
                Text { text: "Typed"; color: Theme.textDim; font.pixelSize: 12; Layout.alignment: Qt.AlignTop; topPadding: 3.001; visible: typed.visible }
                TextEdit {
                    id: typed; visible: !execPop.rec.argv && !!execPop.rec.line && execPop.rec.line !== execPop.rec.command; text: execPop.rec.command || ""
                    readOnly: true; selectByMouse: true; wrapMode: TextEdit.WrapAnywhere; textFormat: TextEdit.PlainText
                    color: Theme.text; selectionColor: Theme.accent; font.family: nb.fontFamily; font.pixelSize: 12
                    topPadding: 3.001; Layout.fillWidth: true
                }
                Item { visible: typed.visible; width: 1; height: 1 }
                Text { text: "Folder"; color: Theme.textDim; font.pixelSize: 12; Layout.alignment: Qt.AlignTop; topPadding: 3.001 }
                TextEdit {
                    text: execPop.rec.cwd || "—"
                    readOnly: true; selectByMouse: true; wrapMode: TextEdit.WrapAnywhere; textFormat: TextEdit.PlainText
                    color: Theme.text; selectionColor: Theme.accent; font.family: nb.fontFamily; font.pixelSize: 12
                    topPadding: 3.001; Layout.fillWidth: true
                }
                Chip { base: Theme.panel; label: "Copy"; Layout.alignment: Qt.AlignTop; onClicked: nb.copyText(execPop.rec.cwd || "") }
                Text { text: "User"; color: Theme.textDim; font.pixelSize: 12; Layout.alignment: Qt.AlignTop; topPadding: 3.001 }
                TextEdit {
                    text: execPop.rec.user || "—"
                    readOnly: true; selectByMouse: true; wrapMode: TextEdit.WrapAnywhere; textFormat: TextEdit.PlainText
                    color: Theme.text; selectionColor: Theme.accent; font.family: nb.fontFamily; font.pixelSize: 12
                    topPadding: 3.001; Layout.fillWidth: true
                }
                Item { width: 1; height: 1 }
            }
            RowLayout {
                Layout.fillWidth: true
                spacing: 6
                Text { text: "Environment" + (execPop.rec.env ? " (" + execPop.rec.env.length + ")" : ": not reported"); color: Theme.textDim; font.pixelSize: 12 }
                TextField {
                    id: envFilter
                    visible: !!execPop.rec.env
                    Layout.preferredWidth: 240
                    placeholderText: "Filter"
                    color: Theme.text
                    font.pixelSize: 12
                    background: Rectangle { radius: 5; color: Theme.bg; border.color: envFilter.activeFocus ? Theme.accent : Theme.divider }
                    Keys.onEscapePressed: execPop.close()
                }
                Item { Layout.fillWidth: true }
                Chip { base: Theme.panel; label: "Copy all"; visible: !!execPop.rec.env; onClicked: nb.copyText(execPop.envs.map(x => x.k + "=" + x.v).join("\n") + "\n") }
            }
            ListView {
                id: envList
                Layout.fillWidth: true
                Layout.preferredHeight: Math.min(contentHeight, Math.max(120, nb.height - 340))
                visible: !!execPop.rec.env
                clip: true
                model: execPop.envs
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: ScrollBar {}
                MomentumWheel { flick: envList }
                delegate: Row {
                    required property var modelData
                    width: envList.width - 12
                    spacing: 8
                    Text {
                        topPadding: 2.001
                        width: Math.min(240, envList.width * 0.3)
                        text: modelData.k
                        elide: Text.ElideRight
                        color: nb.ansi(4, "#61afef")
                        font.family: nb.fontFamily
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                    }
                    TextEdit {
                        topPadding: 2.001
                        width: parent.width - Math.min(240, envList.width * 0.3) - 8
                        text: modelData.v
                        readOnly: true; selectByMouse: true; wrapMode: TextEdit.WrapAnywhere
                        textFormat: TextEdit.PlainText
                        color: Theme.text; selectionColor: Theme.accent
                        font.family: nb.fontFamily
                        font.pixelSize: 12
                    }
                }
            }
        }
    }

    function isTableFile(p) { return /\.(csv|tsv|tab)$/i.test(p) }
    // ---- a CSV or TSV file card: a table ----
    Component {
        id: tableCardView
        CsvTable {
            readonly property var src: parent
            anchors.fill: parent
            onSaveAsRequested: { nb.saveAsCell = nb.indexOf(src.cid); fileSaveDialog.currentFolder = nb.saveFolder(src.viewPath); fileSaveDialog.open() }
            controller: nb.controller
            path: src.viewPath
            fontFamily: nb.fontFamily
            pointSize: nb.outputPointSize
            themeData: nb.themeData
        }
    }

    // ---- a file card: the file in Tessera's viewer ----
    Component {
        id: fileCardView
        FileView {
            id: fcv
            readonly property var src: parent
            anchors.fill: parent
            // A Markdown file: drawn as a page over it (⋯ → Show the source:
            // its text, highlighted; Edit shows the text too).
            readonly property bool mdRendered: nb.isMarkdownFile(path) && (langOverride === "" || langOverride === "markdown") && !editing && !inTerm && nb.kinds[src.cid] !== "mdtext"
            property string mdText: ""
            function readMd() {
                if (!mdRendered || !nb.controller) return
                const p = path
                Async.call(nb.controller, k => nb.controller.readTextFile(k, p), r => { try { const o = JSON.parse(r); if (p === fcv.path) fcv.mdText = o.text || "" } catch (e) {} })
            }
            onMdRenderedChanged: readMd()
            onPathChanged: readMd()
            Component.onCompleted: readMd()
            Loader {
                anchors.fill: parent
                z: 20
                active: fcv.mdRendered
                sourceComponent: markdownPage
                onLoaded: {
                    item.md = Qt.binding(() => fcv.mdText)
                    item.baseDir = Qt.binding(() => fcv.path.replace(/\/[^/]*$/, ""))
                    item.scrolls = true
                }
            }
            controller: nb.controller
            path: src.viewPath
            langOverride: src.viewLang
            editorKeys: nb.editorKeys
            // (Saved under another name: the card shows that file now.)
            onSavedAs: (p) => { const i = nb.indexOf(src.cid); if (i >= 0) { cells.setProperty(i, "viewPath", p); cells.setProperty(i, "source", p); nb.touch() } }
            fontFamily: nb.fontFamily
            pointSize: nb.outputPointSize
            themeData: nb.themeData
            property int rev: nb.fileRev
            onRevChanged: { reload(); readMd() }
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
            // stdout and stderr together (in order, marked in the margin), or
            // one at a time: the card's choice (nb.streamViews).
            readonly property bool both: jf.src.outs.some(o => o.output_type === "stream" && o.name === "stdout")
                                         && jf.src.outs.some(o => (o.output_type === "stream" && o.name === "stderr") || o.output_type === "error")
            // (TESSERA_TEST_STREAM_VIEW: the tab cards start on, for tests.)
            readonly property string pick: both ? (nb.streamViews[jf.src.cid] || (nb.controller ? nb.controller.testEnv("STREAM_VIEW") : "") || "all") : "all"
            readonly property var shown: pick === "all" ? jf.src.outs
                : jf.src.outs.filter(o => pick === "stderr" ? (o.output_type === "error" || (o.output_type === "stream" && o.name === "stderr"))
                                                            : !(o.output_type === "error" || (o.output_type === "stream" && o.name === "stderr")))
            Column {
                id: oc
                y: 2
                width: jf.width - 10
                spacing: 4
                Row {
                    visible: jf.both
                    spacing: 4
                    Repeater {
                        model: [["all", "All"], ["stdout", "stdout"], ["stderr", "stderr"]]
                        delegate: Chip {
                            required property var modelData
                            base: jf.pick === modelData[0] ? Theme.mix(Theme.panel, Theme.accent, 0.35) : Theme.panel
                            label: modelData[1]
                            tip: modelData[0] === "all" ? "Both, in the order they came (stderr marked red)" : "Only " + modelData[1]
                            onClicked: nb.setStreamView(jf.src.cid, modelData[0])
                        }
                    }
                }
                Repeater {
                    model: jf.shown
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
            readonly property bool stream: o.output_type === "stream" || o.output_type === "error"
            implicitHeight: tv.implicitHeight + 8
            height: implicitHeight
            color: "transparent"
            radius: 4
            // (Where it came from, in the margin: stderr and errors red, stdout plain.)
            Rectangle {
                visible: tbox.stream
                width: 3
                height: parent.height
                radius: 1.5
                color: tbox.bad ? nb.ansi(1, "#e06c75") : Theme.divider
            }
            Flickable {
                id: tf
                interactive: false
                anchors.fill: parent
                anchors.margins: 4
                anchors.leftMargin: tbox.stream ? 10 : 4
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
                        if (o.output_type === "stream") return nb.colored((o.text || "").replace(/\n$/, ""))
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
