import QtQuick
import Tessera

// The prompt, drawn by Tessera above the input editor (like Starship, built
// in): segments for the folder, git, the last command's status and time, the
// user and host, and the clock. Which segments show, and in what order,
// comes from `segments` (Preferences); colors from the terminal's theme.
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

    implicitHeight: useStarship ? Math.max(1, starLines.length) * (Math.ceil(sfm.height) + 4) + 4 : 24
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
    function refresh() {
        if (!controller || !visible) return
        if (useStarship) {
            const cols = Math.max(20, Math.floor((bar.width - 16) / Math.max(1, sfm.averageCharacterWidth)))
            controller.requestStarship(key, bar.cwd, bar.view ? bar.view.lastStatus : 0,
                                       bar.view ? bar.view.lastDuration : 0, cols, bar.shellPid)
            return
        }
        try { info = JSON.parse(controller.promptInfo(bar.cwd, bar.shellPid)) } catch (e) { info = {} }
    }
    Connections {
        target: bar.controller
        function onStarshipReady(k, json) {
            if (k !== bar.key) return
            try { const r = JSON.parse(json); bar.starLines = r.lines; bar.starRight = r.right } catch (e) {}
        }
    }
    onWidthChanged: if (useStarship) resizeRefresh.restart()
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
    Timer { interval: 15000; running: bar.visible; repeat: true; onTriggered: { bar.now = new Date(); bar.refresh() } }

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

    readonly property var leftSegs: { bar.info; bar.now; const r = []; for (const s of segments) { if (s === "|") break; const x = segment(s); if (x) r.push(x) } return r }
    readonly property var rightSegs: { bar.info; bar.now; const i = segments.indexOf("|"); const r = []; if (i < 0) return r; for (const s of segments.slice(i + 1)) { const x = segment(s); if (x) r.push(x) } return r }

    component Seg: Item {
        id: seg
        property var s
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
            font.weight: seg.s.bg !== "" ? Font.DemiBold : Font.Normal
            renderType: Text.NativeRendering
        }
    }

    Row {
        visible: !bar.useStarship
        anchors.left: parent.left
        anchors.leftMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8
        Repeater { model: bar.leftSegs; delegate: Seg { s: modelData } }
    }
    // Starship's own prompt, run for run.
    // A Powerline / Nerd Font cap (a pill's rounded or pointed end) is drawn
    // as a shape the height of the run: the font's glyph doesn't line up with
    // the pill and leaves seams.
    component Run: Rectangle {
        id: run
        property var r
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
            renderType: Text.NativeRendering
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
    Column {
        visible: bar.useStarship
        anchors.left: parent.left
        anchors.leftMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        spacing: 2
        Repeater {
            model: bar.starLines
            delegate: Row {
                required property var modelData
                Repeater { model: modelData; delegate: Run { required property var modelData; r: modelData } }
            }
        }
    }
    Row {
        visible: bar.useStarship
        anchors.right: parent.right
        anchors.rightMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        Repeater { model: bar.starRight; delegate: Run { required property var modelData; r: modelData } }
    }
    Row {
        visible: !bar.useStarship
        anchors.right: parent.right
        anchors.rightMargin: 10
        anchors.verticalCenter: parent.verticalCenter
        spacing: 10
        Repeater { model: bar.rightSegs; delegate: Seg { s: modelData } }
    }
}
