import QtQuick
import QtQuick.Controls
import Tessera

// The prompt, drawn by Tessera above the input editor: a row of widgets
// (Tessera's own: the folder, git, the last command's status and time, the
// user and host, the clock, cloud contexts; any Starship module; or
// Starship's whole prompt), left and right of a divider.
//
// It's arranged on the bar itself: drag a widget along it (past the middle
// for the right side, off the bar to remove it), + adds one, a right click
// restyles it. A click acts on it (folders to go to, branches to switch to,
// contexts and profiles): commands, run as if typed (runRequested).
Item {
    id: bar
    property var view: null
    property var controller: null
    property string cwd: ""
    property var themeData: ({})
    property var segments: ["dir", "git", "venv", "nix", "status", "duration", "|", "kube", "aws", "gcloud", "host", "time"]
    property string fontFamily: "monospace"
    // Nerd Font symbols (Powerline, git, language icons) in Symbols Nerd Font:
    // otherwise whatever font the system picks for a private-use character
    // draws it, often something else entirely.
    function glyphs(t) {
        const esc = String(t).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")
        if (!/[\ue000-\uf8ff\udb80-\udbff]/.test(esc)) return "<span style='white-space:pre'>" + esc + "</span>"
        return "<span style='white-space:pre'>" + esc.replace(/([\ue000-\uf8ff]|[\udb80-\udbff][\udc00-\udfff])+/g,
            (m) => "<span style=\"font-family:'Symbols Nerd Font Mono'\">" + m + "</span>") + "</span>"
    }
    property real fontSize: 11
    property int shellPid: 0
    property string engine: "auto"     // "auto", "starship", "builtin"
    readonly property bool useStarship: controller !== null && (engine === "starship" || (engine === "auto" && controller.hasStarship()))
    signal runRequested(string command)
    signal fixRequested()

    // ---- the arrangement (settings: prompt_layout, prompt_styles) -----------
    readonly property var settings: { if (!controller) return {}; controller.revision; try { return JSON.parse(controller.settings()) } catch (e) { return {} } }
    readonly property var layout: {
        const l = settings.prompt_layout || []
        if (l.length > 0) return l
        return useStarship ? ["starship", "|", "starship_right"] : segments
    }
    readonly property var styles: settings.prompt_styles || ({})
    function saveLayout(l) { if (controller) controller.setSetting("prompt_layout", JSON.stringify(l)) }
    function saveStyle(name, st) {
        const all = JSON.parse(JSON.stringify(styles))
        if (st === null) delete all[name]; else all[name] = st
        if (controller) controller.setSetting("prompt_styles", JSON.stringify(all))
    }
    function withDivider(l) { return l.indexOf("|") >= 0 ? l.slice() : l.concat(["|"]) }
    function addWidget(name) {
        const l = withDivider(layout)
        if (name === "|") return
        const at = l.indexOf("|")
        l.splice(at, 0, name)
        saveLayout(l)
    }
    function removeWidget(i) { const l = layout.slice(); l.splice(i, 1); saveLayout(l) }
    function moveWidget(from, to) {
        const l = layout.slice()
        const x = l.splice(from, 1)[0]
        l.splice(to > from ? to - 1 : to, 0, x)
        saveLayout(l)
    }
    function otherSide(i) {
        const l = withDivider(layout)
        const d = l.indexOf("|")
        const x = l.splice(i, 1)[0]
        const nd = l.indexOf("|")
        if (i < d) l.push(x); else l.splice(nd, 0, x)
        saveLayout(l)
    }
    // Starship's whole prompt, split into its modules (from its format).
    function splitStarship() {
        let mods = []
        try { mods = JSON.parse(controller.promptFormatModules()) } catch (e) {}
        if (mods.length === 0) return
        const l = []
        for (const w of layout) {
            if (w === "starship") { for (const m of mods) { if (m === "|") break; l.push(m) } }
            else if (w === "starship_right") { const i = mods.indexOf("|"); if (i >= 0) for (const m of mods.slice(i + 1)) l.push(m) }
            else l.push(w)
        }
        saveLayout(l)
    }
    readonly property var builtins: [
        ["dir", "Folder", "The current folder; click for folders to go to"],
        ["git", "Git", "Branch and changes; click to switch branches"],
        ["status", "Exit status", "The last command's, when it failed; click to fix"],
        ["duration", "Command time", "How long the last command took (over 2 s)"],
        ["venv", "Python virtualenv", ""], ["nix", "Nix shell", ""],
        ["kube", "Kubernetes context", "Click to switch"], ["aws", "AWS profile", "Click to switch"],
        ["gcloud", "gcloud account", "Click to switch configuration"],
        ["host", "user@host (over SSH)", ""], ["user", "User", ""], ["time", "Clock", ""],
        ["starship", "Starship's whole prompt", "As your starship.toml draws it"],
        ["starship_right", "Starship's right prompt", ""]]
    function title(name) {
        if (name.startsWith("star:")) return name.substring(5).replace(/_/g, " ")
        const b = builtins.find(x => x[0] === name)
        return b ? b[1] : name
    }

    implicitHeight: Math.max(24, Math.max(leftRow.implicitHeight, rightRow.implicitHeight) + 6)
    FontMetrics { id: sfm; font.family: bar.fontFamily; font.pointSize: Math.max(8, bar.fontSize - 1) }

    function ansi(i, fallback) {
        const a = bar.themeData && bar.themeData.ansi
        return a && a[i] ? a[i] : fallback
    }

    // ---- facts ------------------------------------------------------------
    property var info: ({})
    // Starship's answer: lines of colored runs (prompt::starship).
    property var starLines: []
    property var starRight: []
    readonly property string key: "prompt-" + Math.random().toString(36).slice(2)
    property var starMods: ({})        // Starship modules drawn one by one: {name: runs}
    // How wide Starship's whole prompt may be (its $fill pads to it): the room
    // the other widgets leave it on the left. On the right it gets no fill:
    // its width would change the room it's given, over and over.
    function fillCols() {
        const d = layout.indexOf("|"), i = layout.indexOf("starship")
        if (i < 0 || (d >= 0 && i > d)) return 1
        let others = 0
        for (let k = 0; k < leftRow.children.length; k++) {
            const c = leftRow.children[k]
            if (c.wd !== undefined && c.wd.name !== "starship" && c.visible) others += c.width + leftRow.spacing
        }
        const room = bar.width - 8 - others - addButton.width - 12 - (rightRow.width > 0 ? rightRow.width + 16 : 0)
        return Math.max(1, Math.floor(room / Math.max(1, sfm.averageCharacterWidth)))
    }
    // (Nothing redrawn while a widget is dragged: that would drop it.)
    property int lastCols: 0
    // (The others changed size: Starship's room did too. Its own size doesn't count.)
    function roomChanged() { if (layout.indexOf("starship") >= 0 && dragFrom < 0 && fillCols() !== lastCols) resizeRefresh.restart() }
    property bool refreshHeld: false
    property var heldStar: null
    property var heldMods: null
    function refresh() {
        if (!controller || !visible) return
        if (dragFrom >= 0) { refreshHeld = true; return }
        const st = bar.view ? bar.view.lastStatus : 0, du = bar.view ? bar.view.lastDuration : 0
        if (layout.indexOf("starship") >= 0 || layout.indexOf("starship_right") >= 0) {
            lastCols = fillCols()
            controller.requestStarship(key, bar.cwd, st, du, lastCols, bar.shellPid)
        }
        const mods = layout.filter(w => w.startsWith("star:")).map(w => w.substring(5))
        if (mods.length > 0) controller.requestPromptModules(key, JSON.stringify(mods), bar.cwd, st, du, bar.shellPid)
        try { info = JSON.parse(controller.promptInfo(bar.cwd, bar.shellPid)) } catch (e) { info = {} }
    }
    onLayoutChanged: refresh()
    Connections {
        target: bar.controller
        function onStarshipReady(k, json) {
            if (k !== bar.key) return
            if (bar.dragFrom >= 0) { bar.heldStar = json; return }
            try { const r = JSON.parse(json); bar.starLines = r.lines; bar.starRight = r.right } catch (e) {}
        }
        function onPromptModulesReady(k, json) {
            if (k !== bar.key) return
            if (bar.dragFrom >= 0) { bar.heldMods = json; return }
            try { bar.starMods = JSON.parse(json) } catch (e) {}
        }
    }
    onWidthChanged: roomChanged()
    Timer { id: resizeRefresh; interval: 200; onTriggered: bar.refresh() }
    onShellPidChanged: refresh()
    // (The prompt's config changed in Preferences.)
    Connections { target: bar.controller; function onRevisionChanged() { bar.refresh() } }
    function color(c, fallback) {
        if (!c) return fallback
        return c[0] === "p" ? ansi(parseInt(c.slice(1)), fallback) : c
    }
    onCwdChanged: refresh()
    onVisibleChanged: refresh()
    Connections {
        target: bar.view
        function onCommandFinished() { bar.refresh() }
    }
    property date now: new Date()
    Timer { interval: 15000; running: bar.visible; repeat: true; onTriggered: { if (bar.dragFrom >= 0) return; bar.now = new Date(); bar.refresh() } }

    function path() {
        let p = bar.cwd.replace(/^file:\/\/[^/]*/, "")
        try { p = decodeURIComponent(p) } catch (e) {}
        const home = controller ? controller.homeDir() : ""
        if (home !== "" && (p === home || p.startsWith(home + "/"))) p = "~" + p.substring(home.length)
        const parts = p.split("/")
        // Long paths keep their last three folders.
        return parts.length > 4 && p.startsWith("~") === false ? "…/" + parts.slice(-3).join("/")
             : parts.length > 4 ? "~/…/" + parts.slice(-3).join("/") : p
    }
    function seconds(ms) {
        if (ms < 60000) return (ms / 1000).toFixed(ms < 10000 ? 1 : 0) + "s"
        const m = Math.floor(ms / 60000), s = Math.round((ms % 60000) / 1000)
        return m < 60 ? m + "m " + s + "s" : Math.floor(m / 60) + "h " + (m % 60) + "m"
    }

    // ---- segments -----------------------------------------------------------
    // Each: {text, fg, bg (empty: plain text)}, or none.
    function segment(name) {
        const i = bar.info
        switch (name) {
        case "dir":
            return { text: "  " + path(), fg: Theme.bg, bg: ansi(5, "#c678dd") }
        case "git": {
            if (!i.branch) return null
            let t = (i.detached ? "➦ " : "") + i.branch
            const marks = []
            if (i.conflicts) marks.push("=" + i.conflicts)
            if (i.staged) marks.push("+" + i.staged)
            if (i.modified) marks.push("!" + i.modified)
            if (i.untracked) marks.push("?" + i.untracked)
            if (i.ahead) marks.push("⇡" + i.ahead)
            if (i.behind) marks.push("⇣" + i.behind)
            if (marks.length) t += "  " + marks.join(" ")
            const dirty = i.staged || i.modified || i.untracked || i.conflicts
            return { text: t, fg: dirty ? ansi(3, "#e5c07b") : ansi(2, "#98c379"), bg: Theme.mix(Theme.bg, Theme.fg, 0.12), icon: i.detached ? "" : "branch" }
        }
        case "status":
            if (!bar.view || bar.view.lastStatus === 0) return null
            return { text: "✘ " + bar.view.lastStatus, fg: ansi(1, "#e06c75"), bg: "" }
        case "duration":
            if (!bar.view || bar.view.lastDuration < 2000) return null
            return { text: "took " + seconds(bar.view.lastDuration), fg: ansi(3, "#e5c07b"), bg: "" }
        case "host":
            // (Only over SSH, like Starship.)
            return i.ssh ? { text: i.user + "@" + i.host, fg: ansi(6, "#56b6c2"), bg: "" } : null
        case "gcloud": {
            const g = i.cloud && i.cloud.gcloud
            if (!g) return null
            const who = (g.account || "").replace(/@.*/, "")
            return { text: "\u2601 " + who + (g.project ? "(" + g.project + ")" : ""), fg: ansi(6, "#89dceb"), bg: Theme.mix(Theme.bg, Theme.fg, 0.12) }
        }
        case "aws": {
            const a = i.cloud && i.cloud.aws
            return a ? { text: "\uf270 " + a.profile + (a.region ? " (" + a.region + ")" : ""), fg: ansi(3, "#fab387"), bg: "" } : null
        }
        case "kube":
            return i.cloud && i.cloud.kube ? { text: "\u2388 " + i.cloud.kube, fg: ansi(4, "#89b4fa"), bg: "" } : null
        case "venv":
            return i.cloud && i.cloud.venv ? { text: "\ue73c " + i.cloud.venv, fg: ansi(3, "#f9e2af"), bg: "" } : null
        case "nix":
            return i.cloud && i.cloud.nix ? { text: "\uf313 " + i.cloud.nix, fg: ansi(4, "#89b4fa"), bg: "" } : null
        case "user":
            return i.user ? { text: i.user, fg: ansi(6, "#56b6c2"), bg: "" } : null
        case "time":
            return { text: Qt.formatTime(bar.now, "hh:mm"), fg: Theme.textDim, bg: "" }
        }
        return null
    }

    // ---- the path's folders, each a click away -----------------------------
    // The folders of a path shown in `t` (Tessera's or Starship's directory:
    // "~/src/tessera", "/usr/bin", "…/a/b"), matched against the shell's
    // folder: [{start, end, path, last}] (character positions in t), or [].
    function pathParts(t) {
        if (!controller) return []
        let cwd = bar.cwd.replace(/^file:\/\/[^/]*/, "")
        try { cwd = decodeURIComponent(cwd) } catch (e) {}
        if (cwd === "") return []
        const home = controller.homeDir()
        const re = /\S+/g
        let m
        while ((m = re.exec(t)) !== null) {
            const tok = m[0]
            if (tok.indexOf("/") < 0 && tok !== "~" && tok !== cwd.replace(/.*\//, "")) continue
            const comps = tok.split("/")
            // The shell's folder and its parents, from the end.
            let dir = cwd.replace(/\/+$/, "") || "/"
            const parts = []
            let pos = m.index + tok.length
            let ok = true
            for (let j = comps.length - 1; j >= 0; j--) {
                const c = comps[j]
                const start = pos - c.length
                if (c === "" && j === 0 && dir === "/") { parts.unshift({ start: m.index, end: m.index + 1, path: "/" }); break }
                if (c === "~") { if (dir !== home) ok = false; else parts.unshift({ start: start, end: pos, path: home }); break }
                if (c === "…" || c === "...") break
                if (c !== dir.replace(/.*\//, "")) { ok = false; break }
                if (c !== "") parts.unshift({ start: start, end: pos, path: dir })
                dir = dir.substring(0, dir.lastIndexOf("/")) || "/"
                pos = start - 1
            }
            if (!ok || parts.length === 0) continue
            parts[parts.length - 1].last = true
            return parts
        }
        return []
    }
    property string hoverPath: ""       // the parent folder of the path under the pointer
    // Over a path's text: a click area per folder (a parent: cd there).
    component PathHits: Item {
        id: ph
        property string t: ""
        property var font
        property color ink
        readonly property var parts: { bar.cwd; return bar.pathParts(t) }
        Repeater {
            model: ph.parts
            delegate: Item {
                required property var modelData
                Text { topPadding: 0.001; id: m0; visible: false; text: bar.glyphs(ph.t.substring(0, modelData.start)); textFormat: Text.RichText; font: ph.font }
                Text { topPadding: 0.001; id: m1; visible: false; text: bar.glyphs(ph.t.substring(0, modelData.end)); textFormat: Text.RichText; font: ph.font }
                x: m0.implicitWidth
                width: m1.implicitWidth - m0.implicitWidth
                height: ph.height
                Rectangle {
                    visible: hh.hovered
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 1
                    width: parent.width
                    height: 1
                    color: ph.ink
                }
                // (A click on the widget goes there while the pointer is over it.)
                HoverHandler {
                    id: hh
                    cursorShape: Qt.PointingHandCursor
                    // (The folder itself: its menu, recent places first; a parent: cd there.)
                    onHoveredChanged: {
                        const v = modelData.last ? "@here" : modelData.path
                        bar.hoverPath = hovered ? v : (bar.hoverPath === v ? "" : bar.hoverPath)
                    }
                }
                ToolTip.visible: hh.hovered
                ToolTip.delay: 600
                ToolTip.text: modelData.last ? "Recent and frequent folders, and this one's" : "cd " + modelData.path
            }
        }
    }

    component Seg: Item {
        id: seg
        property var s
        property bool hot: false
        implicitWidth: label.implicitWidth + (icon.visible ? icon.width + 4 : 0) + (s.bg !== "" ? 18 : 2)
        height: 18
        anchors.verticalCenter: parent.verticalCenter
        Rectangle {
            anchors.fill: parent
            visible: seg.s.bg !== ""
            radius: height / 2
            color: seg.s.bg
        }
        // The branch mark, drawn (a font's Powerline glyph varies, or is missing).
        Canvas {
            id: icon
            visible: seg.s.icon === "branch"
            width: 8
            height: 12
            anchors.verticalCenter: parent.verticalCenter
            x: (seg.width - label.implicitWidth - width - 4) / 2
            property color ink: seg.s.fg
            onInkChanged: requestPaint()
            onPaint: {
                const c = getContext("2d")
                c.reset()
                c.strokeStyle = ink
                c.fillStyle = ink
                c.lineWidth = 1.3
                c.beginPath(); c.moveTo(2, 2.5); c.lineTo(2, 9.5); c.stroke()
                c.beginPath(); c.moveTo(6.2, 4); c.bezierCurveTo(6.2, 7, 2, 6.5, 2, 9); c.stroke()
                for (const p of [[2, 2], [2, 10], [6.2, 3.2]]) { c.beginPath(); c.arc(p[0], p[1], 1.6, 0, Math.PI * 2); c.fill() }
            }
        }
        Text {
            id: label
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: icon.visible ? (icon.width + 4) / 2 : 0
            anchors.verticalCenterOffset: 0.001
            text: bar.glyphs(seg.s.text)
            textFormat: Text.RichText
            color: seg.s.fg
            font.family: bar.fontFamily
            font.pointSize: Math.max(8, bar.fontSize - 1)
            font.weight: seg.s.bold !== undefined ? (seg.s.bold ? Font.Bold : Font.Normal) : seg.s.bg !== "" ? Font.DemiBold : Font.Normal
            font.underline: seg.hot
            renderType: Text.NativeRendering
            PathHits { anchors.fill: parent; t: seg.s.text; font: label.font; ink: label.color }
        }
    }

    // Starship's own prompt, run for run.
    // A Powerline / Nerd Font cap (a pill's rounded or pointed end) is drawn
    // as a shape the height of the run: the font's glyph doesn't line up with
    // the pill and leaves seams.
    component Run: Rectangle {
        id: run
        property var r
        property bool hot: false
        readonly property string cap: r.t.length === 1 && "\ue0b0\ue0b2\ue0b4\ue0b6\ue0bc\ue0be\ue0b8\ue0ba".indexOf(r.t) >= 0 ? r.t : ""
        readonly property color ink: bar.color(r.fg, Theme.fg)
        width: cap !== "" ? Math.ceil(height / 2) : Math.ceil(rt.implicitWidth)
        height: Math.ceil(sfm.height) + 2
        color: r.bg ? bar.color(r.bg, "transparent") : "transparent"
        Text {
            id: rt
            visible: run.cap === ""
            anchors.verticalCenter: parent.verticalCenter
            anchors.verticalCenterOffset: 0.001
            text: bar.glyphs(run.r.t)
            textFormat: Text.RichText
            color: run.ink
            font.family: bar.fontFamily
            font.pointSize: Math.max(8, bar.fontSize - 1)
            font.bold: run.r.b
            font.underline: run.hot && run.r.t.trim() !== ""
            renderType: Text.NativeRendering
            PathHits { anchors.fill: parent; t: run.cap === "" ? run.r.t : ""; font: rt.font; ink: rt.color }
        }
        Canvas {
            visible: run.cap !== ""
            anchors.fill: parent
            property color ink: run.ink
            property string cap: run.cap
            onInkChanged: requestPaint()
            onCapChanged: requestPaint()
            onWidthChanged: requestPaint()
            onHeightChanged: requestPaint()
            onPaint: {
                const c = getContext("2d")
                c.reset()
                c.fillStyle = ink
                const w = width, h = height
                c.beginPath()
                switch (cap) {
                case "\ue0b6": // left half circle
                    c.moveTo(w, 0); c.arc(w, h / 2, h / 2, -Math.PI / 2, Math.PI / 2, true); c.lineTo(w, h); break
                case "\ue0b4": // right half circle
                    c.moveTo(0, 0); c.arc(0, h / 2, h / 2, -Math.PI / 2, Math.PI / 2, false); c.lineTo(0, h); break
                case "\ue0b0": case "\ue0b8": // right-pointing triangle
                    c.moveTo(0, 0); c.lineTo(w, h / 2); c.lineTo(0, h); break
                case "\ue0b2": case "\ue0ba": // left-pointing triangle
                    c.moveTo(w, 0); c.lineTo(0, h / 2); c.lineTo(w, h); break
                case "\ue0bc": // upper-left triangle
                    c.moveTo(0, 0); c.lineTo(w, 0); c.lineTo(0, h); break
                case "\ue0be": // upper-right triangle
                    c.moveTo(0, 0); c.lineTo(w, 0); c.lineTo(w, h); break
                }
                c.closePath()
                c.fill()
            }
        }
    }

    // A widget's look, with its style (prompt_styles) laid over.
    function styled(name, x) {
        const st = styles[name]
        if (!x || !st) return x
        const y = Object.assign({}, x)
        if (st.fg) y.fg = color(st.fg, y.fg)
        if (st.bg === "none") y.bg = ""
        else if (st.bg) y.bg = color(st.bg, y.bg)
        if (st.prefix) y.text = st.prefix + y.text
        if (st.suffix) y.text = y.text + st.suffix
        if (st.bold !== undefined) y.bold = st.bold
        return y
    }
    function styledRuns(name, runs) {
        const st = styles[name]
        if (!st) return runs
        let r = runs.map(x => {
            const y = Object.assign({}, x)
            if (st.fg) y.fg = st.fg
            if (st.bg === "none") y.bg = ""
            else if (st.bg) y.bg = st.bg
            if (st.bold !== undefined) y.b = st.bold
            return y
        })
        const edge = r.length ? r[0] : {}
        if (st.prefix) r = [{ t: st.prefix, fg: st.fg || edge.fg || "", bg: st.bg && st.bg !== "none" ? st.bg : "", b: !!st.bold }].concat(r)
        if (st.suffix) r = r.concat([{ t: st.suffix, fg: st.fg || edge.fg || "", bg: st.bg && st.bg !== "none" ? st.bg : "", b: !!st.bold }])
        return r
    }
    // The widgets, by side: [{name, i (in layout), kind, seg | runs | lines}].
    function widgets(right) {
        bar.info; bar.now; bar.starMods; bar.starLines; bar.starRight; bar.styles
        const out = []
        const d = layout.indexOf("|")
        layout.forEach((w, i) => {
            if (w === "|" || (right ? (d < 0 || i < d) : (d >= 0 && i > d))) return
            if (w === "starship") { if (bar.starLines.length) out.push({ name: w, i: i, kind: "lines", lines: bar.starLines.map(l => styledRuns(w, l)) }) }
            else if (w === "starship_right") { if (bar.starRight.length) out.push({ name: w, i: i, kind: "runs", runs: styledRuns(w, bar.starRight) }) }
            else if (w.startsWith("star:")) { const r = bar.starMods[w.substring(5)]; if (r && r.length) out.push({ name: w, i: i, kind: "runs", runs: styledRuns(w, r) }) }
            else { const x = styled(w, segment(w)); if (x) out.push({ name: w, i: i, kind: "seg", seg: x }) }
        })
        return out
    }
    readonly property var leftWidgets: widgets(false)
    readonly property var rightWidgets: widgets(true)

    // ---- dragging -----------------------------------------------------------
    property int dragFrom: -1           // the layout index being dragged
    property real dragX: 0
    property real dragY: 0
    // (Well off the bar, up or down: removed. A drag that drifts a little stays.)
    readonly property bool dragOff: dragFrom >= 0 && Math.abs(dragY - height / 2) > height / 2 + 56
    property var dropAt: -1             // the layout index it would go before
    property real dropX: 0
    // Where a dragged widget would go: the left side or the right (the bar's
    // halves), before the first widget there whose middle is past the pointer.
    function dropTarget() {
        const right = bar.dragX > bar.width / 2
        const row = right ? rightRow : leftRow
        const marks = []
        for (let k = 0; k < row.children.length; k++) {
            const c = row.children[k]
            if (c.wd === undefined || !c.visible) continue
            const p = c.mapToItem(bar, 0, 0)
            marks.push({ i: c.wi, x: p.x + c.width / 2, left: p.x - 4, right: p.x + c.width + 4 })
        }
        marks.sort((a, b) => a.x - b.x)
        for (const m of marks)
            if (bar.dragX < m.x) { bar.dropX = m.left; return m.i }
        const d = layout.indexOf("|")
        if (marks.length) bar.dropX = marks[marks.length - 1].right
        else bar.dropX = right ? rightRow.x + rightRow.width - 2 : leftRow.x
        // (The end of the side: before the divider, or the very end.)
        return right ? layout.length : (d >= 0 ? d : layout.length)
    }
    function releaseHeld() {
        if (heldStar !== null) { try { const r = JSON.parse(heldStar); starLines = r.lines; starRight = r.right } catch (e) {} heldStar = null }
        if (heldMods !== null) { try { starMods = JSON.parse(heldMods) } catch (e) {} heldMods = null }
        if (refreshHeld) { refreshHeld = false; Qt.callLater(refresh) }
    }
    function finishDrag() {
        if (dragFrom < 0) return
        const from = dragFrom
        dragFrom = -1
        Qt.callLater(releaseHeld)
        if (dragOff) { removeWidget(from); return }
        const right = dragX > width / 2
        const l = withDivider(layout)
        let to = dropAt
        // (Dropped at the right side's end with no divider yet: after it.)
        if (layout.indexOf("|") < 0 && right) to = l.length
        if (to === from || to === from + 1) return
        const x = l.splice(from, 1)[0]
        l.splice(to > from ? to - 1 : to, 0, x)
        saveLayout(l)
    }

    // ---- a widget on the bar -------------------------------------------------
    component Widget: Item {
        id: w
        required property var modelData
        readonly property var wd: modelData
        readonly property int wi: wd.i
        implicitWidth: body.implicitWidth
        implicitHeight: body.implicitHeight
        width: implicitWidth
        height: implicitHeight
        anchors.verticalCenter: parent ? parent.verticalCenter : undefined
        opacity: bar.dragFrom === wi ? 0.35 : 1
        // (Hovered, and not on a folder of its path: underlined, it's clickable.)
        readonly property bool hot: hover.hovered && bar.hoverPath === "" && bar.dragFrom < 0
        Loader {
            id: body
            sourceComponent: w.wd.kind === "seg" ? segBody : w.wd.kind === "lines" ? linesBody : runsBody
            property var wd: w.wd
            property bool hot: w.hot
        }
        Rectangle {
            anchors.fill: parent
            anchors.margins: -2
            radius: 6
            color: "transparent"
            border.color: Theme.accent
            border.width: 1
            opacity: hover.hovered && bar.dragFrom < 0 && (w.choicesFor !== "") ? 0.6 : 0
        }
        readonly property string choicesFor: w.wd.name === "status" ? "fix" : w.wd.name
        HoverHandler { id: hover; cursorShape: Qt.PointingHandCursor }
        TapHandler {
            acceptedButtons: Qt.LeftButton
            // (On a parent folder of its path: cd there.)
            onTapped: {
                const p = bar.hoverPath
                if (p === "@here") bar.openChoices("dir", w)
                else if (p !== "") bar.runRequested("cd " + (/^[A-Za-z0-9_\/.~+:@-]+$/.test(p) ? p : "'" + p.replace(/'/g, "'\\''") + "'"))
                else bar.openChoices(w.wd.name, w)
            }
        }
        TapHandler {
            acceptedButtons: Qt.RightButton
            onTapped: (pt) => bar.openWidgetMenu(w.wi, w.wd.name, w)
        }
        DragHandler {
            id: drag
            target: null
            dragThreshold: 6
            onActiveChanged: {
                if (active) { bar.dragFrom = w.wi; ghost.text = bar.title(w.wd.name) }
                else bar.finishDrag()
            }
            onCentroidChanged: {
                if (!active) return
                const p = w.mapToItem(bar, centroid.position.x, centroid.position.y)
                bar.dragX = p.x
                bar.dragY = p.y
                bar.dropAt = bar.dropTarget()
            }
        }
        ToolTip.visible: hover.hovered && bar.hoverPath === "" && bar.dragFrom < 0 && !choicesPop.opened && !widgetMenu.opened
        ToolTip.delay: 900
        ToolTip.text: bar.title(w.wd.name) + " — click for choices, drag to move, right-click to style"
    }
    Component { id: segBody; Seg { s: parent.wd.seg; hot: parent ? parent.hot : false } }
    Component {
        id: runsBody
        Row {
            id: rr
            readonly property var wd: parent ? parent.wd : null
            Repeater { model: rr.wd ? rr.wd.runs : []; delegate: Run { required property var modelData; r: modelData; hot: rr.parent ? rr.parent.hot : false } }
        }
    }
    Component {
        id: linesBody
        Column {
            id: lc
            readonly property var wd: parent ? parent.wd : null
            spacing: 2
            Repeater {
                model: lc.wd ? lc.wd.lines : []
                delegate: Row {
                    required property var modelData
                    Repeater { model: modelData; delegate: Run { required property var modelData; r: modelData; hot: lc.parent ? lc.parent.hot : false } }
                }
            }
        }
    }

    Row {
        id: leftRow
        onImplicitWidthChanged: bar.roomChanged()
        anchors.left: parent.left
        anchors.leftMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8
        Repeater { model: bar.leftWidgets; delegate: Widget {} }
    }
    Row {
        id: rightRow
        onImplicitWidthChanged: bar.roomChanged()
        anchors.right: addButton.left
        anchors.rightMargin: 6
        anchors.verticalCenter: parent.verticalCenter
        spacing: 10
        Repeater { model: bar.rightWidgets; delegate: Widget {} }
    }
    // Where a dragged widget would land, and its name under the pointer.
    Rectangle {
        visible: bar.dragFrom >= 0 && !bar.dragOff
        x: bar.dropX - 1
        width: 2
        height: bar.height - 4
        y: 2
        radius: 1
        color: Theme.accent
    }
    Rectangle {
        visible: bar.dragFrom >= 0
        x: bar.dragX - width / 2
        y: bar.dragY - height / 2
        z: 10
        width: ghost.implicitWidth + 16
        height: 20
        radius: 10
        color: bar.dragOff ? ansi(1, "#e06c75") : Theme.accent
        opacity: 0.9
        Text {
            id: ghost
            anchors.centerIn: parent
            anchors.verticalCenterOffset: 0.001
            text: ""
            color: Theme.bg
            font.pixelSize: 11
            font.weight: Font.DemiBold
            renderType: Text.NativeRendering
        }
        Text { topPadding: 0.001;
            visible: bar.dragOff
            anchors.top: parent.bottom
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Let go to remove"
            color: ansi(1, "#e06c75")
            font.pixelSize: 10
            renderType: Text.NativeRendering
        }
    }

    // ---- + -------------------------------------------------------------------
    HoverHandler { id: barHover }
    Rectangle {
        id: addButton
        anchors.right: parent.right
        anchors.rightMargin: 6
        anchors.verticalCenter: parent.verticalCenter
        width: 18
        height: 18
        radius: 9
        color: addHover.hovered ? Theme.mix(Theme.bg, Theme.accent, 0.35) : Theme.mix(Theme.bg, Theme.fg, 0.08)
        opacity: barHover.hovered || addPop.opened || bar.layout.length === 0 ? 1 : 0.35
        Text {
            anchors.centerIn: parent
            anchors.verticalCenterOffset: 0.001
            text: "+"
            color: Theme.text
            font.pixelSize: 13
            renderType: Text.NativeRendering
        }
        HoverHandler { id: addHover; cursorShape: Qt.PointingHandCursor }
        TapHandler { onTapped: { addPop.x = Math.max(0, bar.width - addPop.width - 4); addPop.open() } }
        ToolTip.visible: addHover.hovered && !addPop.opened
        ToolTip.delay: 500
        ToolTip.text: "Add a widget to the prompt"
    }
    Popup {
        id: addPop
        y: -height - 4
        width: 300
        height: Math.min(420, addList.contentHeight + addFilter.height + 22)
        padding: 6
        topPadding: 6.001   // (off the half pixel: 130% scale smears native text there)
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        property var star: []
        onOpened: { addFilter.text = ""; addFilter.forceActiveFocus(); try { star = JSON.parse(bar.controller.promptStarModules()) } catch (e) { star = [] } }
        readonly property var options: {
            const o = [{ head: "Tessera" }]
            for (const b of bar.builtins) o.push({ name: b[0], label: b[1], note: b[2] })
            o.push({ head: "Starship modules" + (star.length ? " (" + star.length + ")" : "") })
            if (star.length === 0) o.push({ name: "", label: "Getting Starship from nixpkgs…", note: "Open this again in a moment" })
            for (const m of star) o.push({ name: "star:" + m, label: m.replace(/_/g, " "), note: "" })
            const f = addFilter.text.toLowerCase()
            return f === "" ? o : o.filter(x => !x.head && (x.label + " " + x.name + " " + x.note).toLowerCase().indexOf(f) >= 0)
        }
        contentItem: Column {
            spacing: 4
            TextField {
                id: addFilter
                width: parent.width
                placeholderText: "Search widgets"
                color: Theme.text
                font.pixelSize: 12
                background: Rectangle { radius: 6; color: Theme.bg; border.color: addFilter.activeFocus ? Theme.accent : Theme.divider }
                Keys.onReturnPressed: { const x = addPop.options.find(o => !o.head && o.name); if (x) { bar.addWidget(x.name); addPop.close() } }
                Keys.onEscapePressed: addPop.close()
            }
            ListView {
                id: addList
                width: parent.width
                height: Math.min(370, contentHeight)
                clip: true
                model: addPop.options
                ScrollBar.vertical: ScrollBar {}
                MomentumWheel { flick: addList }
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool on: !modelData.head && bar.layout.indexOf(modelData.name) >= 0
                    width: addList.width
                    height: modelData.head ? 22 : (modelData.note ? 34 : 24)
                    radius: 4
                    color: !modelData.head && ah.hovered ? Theme.tabHover : "transparent"
                    Column {
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.left: parent.left
                        anchors.leftMargin: 8
                        anchors.right: parent.right
                        Text { topPadding: 0.001;
                            text: modelData.head ? modelData.head : (parent.parent.on ? "✓ " : "") + modelData.label
                            color: modelData.head ? Theme.textDim : Theme.text
                            font.pixelSize: modelData.head ? 10 : 12
                            font.underline: !modelData.head && ah.hovered
                            font.weight: modelData.head ? Font.DemiBold : Font.Normal
                            renderType: Text.NativeRendering
                        }
                        Text { topPadding: 0.001;
                            visible: !!modelData.note
                            width: parent.width
                            text: modelData.note || ""
                            color: Theme.textDim
                            font.pixelSize: 10
                            elide: Text.ElideRight
                            renderType: Text.NativeRendering
                        }
                    }
                    HoverHandler { id: ah }
                    TapHandler { enabled: !modelData.head && modelData.name !== ""; onTapped: { bar.addWidget(modelData.name); addPop.close() } }
                }
            }
        }
    }

    // ---- a click: what the widget offers ---------------------------------------
    property var choices: []
    function openChoices(name, item) {
        if (name === "status") { bar.fixRequested(); return }
        let c = []
        try { c = JSON.parse(controller.promptChoices(name, bar.cwd, bar.shellPid)) } catch (e) {}
        if (c.length === 0) { openWidgetMenu(layout.indexOf(name), name, item); return }
        choices = c
        choiceFilter.text = ""
        const p = item.mapToItem(bar, 0, 0)
        choicesPop.x = Math.min(Math.max(0, p.x), bar.width - choicesPop.width)
        choicesPop.open()
    }
    // (Long lists, the folder's: a search field filters them.)
    readonly property var shownChoices: {
        const f = choiceFilter.text.toLowerCase()
        if (f === "") return choices
        return choices.filter(c => !c.section && c.label.toLowerCase().indexOf(f) >= 0)
    }
    Popup {
        id: choicesPop
        y: -height - 4
        onOpened: if (bar.choices.length > 8) choiceFilter.forceActiveFocus()
        width: 280
        height: Math.min(420, choiceList.contentHeight + 12 + (choiceFilter.visible ? choiceFilter.height + 6 : 0))
        padding: 6
        topPadding: 6.001   // (off the half pixel: 130% scale smears native text there)
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        contentItem: Column {
          spacing: 6
          TextField {
            id: choiceFilter
            visible: bar.choices.length > 8
            width: parent.width
            height: visible ? implicitHeight : 0
            placeholderText: "Search"
            color: Theme.text
            font.pixelSize: 12
            background: Rectangle { radius: 6; color: Theme.bg; border.color: choiceFilter.activeFocus ? Theme.accent : Theme.divider }
            Keys.onReturnPressed: { const c = bar.shownChoices.find(x => !x.section); if (c) { choicesPop.close(); bar.runRequested(c.cmd) } }
            Keys.onEscapePressed: choicesPop.close()
          }
          ListView {
            id: choiceList
            width: parent.width
            height: Math.min(380, contentHeight)
            clip: true
            model: bar.shownChoices
            ScrollBar.vertical: ScrollBar {}
            MomentumWheel { flick: choiceList }
            delegate: Rectangle {
                required property var modelData
                width: choiceList.width
                height: modelData.section ? 22 : 24
                radius: 4
                color: !modelData.section && ch.hovered ? Theme.tabHover : "transparent"
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    x: 8
                    width: parent.width - 16
                    text: (modelData.current ? "✓ " : "") + modelData.label
                    color: modelData.section ? Theme.textDim : Theme.text
                    font.pixelSize: modelData.section ? 10 : 12
                    font.weight: modelData.section || modelData.current ? Font.DemiBold : Font.Normal
                    font.family: modelData.section ? Qt.application.font.family : bar.fontFamily
                    font.underline: !modelData.section && ch.hovered
                    elide: Text.ElideMiddle
                    renderType: Text.NativeRendering
                }
                HoverHandler { id: ch; cursorShape: modelData.section ? Qt.ArrowCursor : Qt.PointingHandCursor }
                TapHandler { enabled: !modelData.section; onTapped: { choicesPop.close(); bar.runRequested(modelData.cmd) } }
            }
        }
  }
    }

    // ---- a right click: the widget's own menu, and its style ----------------
    property int menuFor: -1
    property string menuName: ""
    function openWidgetMenu(i, name, item) {
        menuFor = i
        menuName = name
        const p = item.mapToItem(bar, 0, 0)
        widgetMenu.x = Math.min(Math.max(0, p.x), bar.width - widgetMenu.width)
        widgetMenu.open()
    }
    SlimMenu {
        id: widgetMenu
        y: -height - 4
        SlimMenuItem { text: "Style…"; onTriggered: bar.openStyle(bar.menuName) }
        SlimMenuItem {
            text: bar.layout.indexOf("|") >= 0 && bar.menuFor > bar.layout.indexOf("|") ? "Move to the left" : "Move to the right"
            onTriggered: bar.otherSide(bar.menuFor)
        }
        SlimMenuItem { text: "Split into Starship modules"; visible: bar.menuName === "starship"; height: visible ? implicitHeight : 0; onTriggered: bar.splitStarship() }
        SlimMenuItem { text: "Add a widget…"; onTriggered: { addPop.x = Math.max(0, bar.width - addPop.width - 4); addPop.open() } }
        SlimMenuItem { text: "Remove"; onTriggered: bar.removeWidget(bar.menuFor) }
        SlimMenuItem { text: "Reset the prompt"; onTriggered: { bar.saveLayout([]); if (bar.controller) bar.controller.setSetting("prompt_styles", "{}") } }
    }
    property var styleDraft: ({})
    function openStyle(name) {
        styleDraft = Object.assign({}, styles[name] || {})
        stylePop.name = name
        stylePop.x = Math.max(0, Math.min(bar.width - stylePop.width, 40))
        stylePop.open()
    }
    function setStyle(k, v) {
        const d = Object.assign({}, styleDraft)
        if (v === undefined || v === "") delete d[k]; else d[k] = v
        styleDraft = d
        saveStyle(stylePop.name, Object.keys(d).length ? d : null)
    }
    Popup {
        id: stylePop
        y: -height - 4
        property string name: ""
        width: 300
        padding: 10
        topPadding: 10.001   // (off the half pixel: 130% scale smears native text there)
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        component Swatches: Flow {
            id: sw
            property string key
            width: parent.width
            spacing: 4
            Repeater {
                model: ["", "none"].concat([1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 0, 8].map(n => "p" + n))
                delegate: Rectangle {
                    required property var modelData
                    readonly property bool picked: (bar.styleDraft[sw.key] || "") === modelData
                    visible: modelData !== "none" || sw.key === "bg"
                    width: 18; height: 18; radius: 9
                    color: modelData === "" || modelData === "none" ? "transparent" : bar.color(modelData, "transparent")
                    border.color: picked ? Theme.text : Theme.divider
                    border.width: picked ? 2 : 1
                    Text { topPadding: 0.001;
                        anchors.centerIn: parent
                        visible: modelData === "" || modelData === "none"
                        text: modelData === "" ? "A" : "∅"
                        color: Theme.textDim
                        font.pixelSize: 9
                        renderType: Text.NativeRendering
                    }
                    TapHandler { onTapped: bar.setStyle(sw.key, modelData) }
                    HoverHandler { cursorShape: Qt.PointingHandCursor }
                }
            }
        }
        contentItem: Column {
            spacing: 6
            Text { topPadding: 0.001; text: bar.title(stylePop.name); color: Theme.text; font.pixelSize: 13; font.weight: Font.DemiBold; renderType: Text.NativeRendering }
            Text { topPadding: 0.001; text: "Text color"; color: Theme.textDim; font.pixelSize: 10; renderType: Text.NativeRendering }
            Swatches { key: "fg" }
            Text { topPadding: 0.001; text: "Background (∅ plain, A its own)"; color: Theme.textDim; font.pixelSize: 10; renderType: Text.NativeRendering }
            Swatches { key: "bg" }
            Row {
                spacing: 6
                width: parent.width
                TextField {
                    width: (parent.width - 6) / 2
                    placeholderText: "Before (icon, text)"
                    text: bar.styleDraft.prefix || ""
                    color: Theme.text
                    font.pixelSize: 11
                    background: Rectangle { radius: 5; color: Theme.bg; border.color: parent.activeFocus ? Theme.accent : Theme.divider }
                    onEditingFinished: bar.setStyle("prefix", text)
                }
                TextField {
                    width: (parent.width - 6) / 2
                    placeholderText: "After"
                    text: bar.styleDraft.suffix || ""
                    color: Theme.text
                    font.pixelSize: 11
                    background: Rectangle { radius: 5; color: Theme.bg; border.color: parent.activeFocus ? Theme.accent : Theme.divider }
                    onEditingFinished: bar.setStyle("suffix", text)
                }
            }
            Row {
                spacing: 12
                component Toggle: Rectangle {
                    id: tg
                    property string label
                    property bool on: false
                    signal clicked()
                    width: tl.implicitWidth + 16
                    height: 22
                    radius: 11
                    color: on ? Theme.mix(Theme.bg, Theme.accent, 0.45) : th.hovered ? Theme.tabHover : Theme.mix(Theme.bg, Theme.fg, 0.06)
                    border.color: Theme.divider
                    Text { id: tl; anchors.centerIn: parent; anchors.verticalCenterOffset: 0.001; text: tg.label; color: Theme.text; font.pixelSize: 11; font.underline: th.hovered; renderType: Text.NativeRendering }
                    HoverHandler { id: th; cursorShape: Qt.PointingHandCursor }
                    TapHandler { onTapped: tg.clicked() }
                }
                Toggle { label: "Bold"; on: !!bar.styleDraft.bold; onClicked: bar.setStyle("bold", on ? undefined : true) }
                Toggle { label: "Default look"; onClicked: { bar.styleDraft = {}; bar.saveStyle(stylePop.name, null) } }
            }
            Text { topPadding: 0.001;
                width: parent.width
                visible: stylePop.name.startsWith("star") 
                text: "Starship draws this one: colors here go over its own. Its format is in your starship.toml (or Preferences › Prompt)."
                wrapMode: Text.Wrap
                color: Theme.textDim
                font.pixelSize: 10
                renderType: Text.NativeRendering
            }
        }
    }
    Timer { id: t2; property string cmd; interval: 2500; onTriggered: { for (let k = 0; k < leftRow.children.length; k++) { const c = leftRow.children[k]; if (c.wd && c.wd.name === cmd.substring(cmd.indexOf(":") + 1)) { if (cmd.startsWith("choices:")) bar.openChoices(c.wd.name, c); else bar.openStyle(c.wd.name) } } } }
    // (Test hook: TESSERA_TEST_PROMPT=add | choices:<widget> | style:<widget> | split[,choices:… | ,style:…] | run:<a folder choice's command>.)
    Timer {
        interval: 5000
        running: bar.controller !== null && bar.visible && bar.controller.testEnv("PROMPT") !== ""
        onTriggered: {
            const t = bar.controller.testEnv("PROMPT")
            if (t === "add") { addPop.x = Math.max(0, bar.width - addPop.width - 4); addPop.open() }
            else if (t === "split") bar.splitStarship()
            else if (t.startsWith("run:")) { let c = []; try { c = JSON.parse(controller.promptChoices("dir", "/usr", 0)) } catch (e) {} const x = c.find(o => o.cmd === t.substring(4)); if (x) bar.runRequested(x.cmd) }
            else if (t.startsWith("split,")) { bar.splitStarship(); t2.cmd = t.substring(6); t2.start() }
            else for (const row of [leftRow, rightRow]) for (let k = 0; k < row.children.length; k++) {
                const c = row.children[k]
                if (c.wd === undefined || c.wd.name !== t.substring(t.indexOf(":") + 1)) continue
                if (t.startsWith("choices:")) bar.openChoices(c.wd.name, c)
                else if (t.startsWith("style:")) bar.openStyle(c.wd.name)
            }
        }
    }
}
