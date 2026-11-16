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
    property var segments: ["dir", "git", "status", "duration", "|", "host", "time"]
    property string fontFamily: "monospace"
    property real fontSize: 11

    implicitHeight: 24

    function ansi(i, fallback) {
        const a = bar.themeData && bar.themeData.ansi
        return a && a[i] ? a[i] : fallback
    }

    // ---- facts ------------------------------------------------------------
    property var info: ({})
    function refresh() {
        if (!controller || !visible) return
        try { info = JSON.parse(controller.promptInfo(bar.cwd)) } catch (e) { info = {} }
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
            let t = (i.detached ? "➦ " : " ") + i.branch
            const marks = []
            if (i.conflicts) marks.push("=" + i.conflicts)
            if (i.staged) marks.push("+" + i.staged)
            if (i.modified) marks.push("!" + i.modified)
            if (i.untracked) marks.push("?" + i.untracked)
            if (i.ahead) marks.push("⇡" + i.ahead)
            if (i.behind) marks.push("⇣" + i.behind)
            if (marks.length) t += "  " + marks.join(" ")
            const dirty = i.staged || i.modified || i.untracked || i.conflicts
            return { text: t, fg: dirty ? ansi(3, "#e5c07b") : ansi(2, "#98c379"), bg: Theme.mix(Theme.bg, Theme.fg, 0.12) }
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
        implicitWidth: label.implicitWidth + (s.bg !== "" ? 18 : 2)
        height: 18
        anchors.verticalCenter: parent.verticalCenter
        Rectangle {
            anchors.fill: parent
            visible: seg.s.bg !== ""
            radius: height / 2
            color: seg.s.bg
        }
        Text {
            id: label
            anchors.centerIn: parent
            anchors.verticalCenterOffset: 0.001
            text: seg.s.text
            color: seg.s.fg
            font.family: bar.fontFamily
            font.pointSize: Math.max(8, bar.fontSize - 1)
            font.weight: seg.s.bg !== "" ? Font.DemiBold : Font.Normal
            renderType: Text.NativeRendering
        }
    }

    Row {
        anchors.left: parent.left
        anchors.leftMargin: 8
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8
        Repeater { model: bar.leftSegs; delegate: Seg { s: modelData } }
    }
    Row {
        anchors.right: parent.right
        anchors.rightMargin: 10
        anchors.verticalCenter: parent.verticalCenter
        spacing: 10
        Repeater { model: bar.rightSegs; delegate: Seg { s: modelData } }
    }
}
