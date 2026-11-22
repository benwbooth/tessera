import QtQuick
import QtQuick.Controls
import Tessera
import Tessera.Native

// A folder, browsable, in a notebook card (an `ls` or `cd` card). On the
// surface it reads as ls output: names with icons in columns going down
// then across (ls's own layout), `ls -l` lines, or tiles, in the
// terminal's font, one terminal row per line. Underneath: a click on a
// folder goes into it (back, forward and up, a path bar once away from
// where it started), a click on a file opens it, a right click has the
// file manager's actions (Dolphin, Finder), and a small bar (on hover)
// switches the view, the sort, folders first and hidden files. Those
// choices are kept (settings: file_browser) for the next card.
Item {
    id: fb
    property var controller: null
    property string startDir: ""
    property bool flagAll: false        // ls -a
    property bool flagLong: false       // ls -l
    property string onlyName: ""        // ls -d: just this entry of the folder (the folder itself)
    // ls's order: "date" (-t), "size" (-S), "type" (-X), "none" (-U), or "" (by
    // name); flagReverse: -r. Given, they're the card's, over the last choice.
    property string flagSort: ""
    property bool flagReverse: false
    property bool flagFoldersFirst: false   // ls --group-directories-first
    property string fontFamily: "monospace"
    property real pointSize: 10
    property real rowHeight: 18
    property var themeData: ({})
    property int refresh: 0             // bump to read the folder again
    signal commandRequested(string command)
    signal insertRequested(string text)
    signal viewRequested(string path)   // show it in Tessera (FilePreview)
    signal fileCardRequested(string path)  // show it in a card of its own (FileView)

    // ---- this card's view: its own (the notebook keeps it, by card), first
    // taken from the last one chosen (settings: file_browser). A change is
    // this card's, and the default for the next card. ----------------------
    readonly property var saved: { if (!controller) return {}; try { return JSON.parse(controller.settings()).file_browser || {} } catch (e) { return {} } }
    property var kept: null             // the notebook's copy of this card's choices
    signal keptChanged2(var choices)    // (to the notebook, to keep)
    property var mine: ({})
    Component.onCompleted: {
        mine = kept ? Object.assign({}, kept) : Object.assign({}, saved, { view: undefined })
        // (ls -t, -S, -r ...: the command's order, not the last one picked.)
        if (!kept && (flagSort !== "" || flagReverse || flagFoldersFirst)) mine = Object.assign({}, mine, { sort: undefined, desc: undefined, folders_first: undefined })
        list()
    }
    readonly property string view: mine.view ? mine.view : flagLong ? "details" : (saved.view || "list")
    readonly property string sortBy: mine.sort || flagSort || "name"
    // (As ls has it: by time and by size the newest and largest first, -r the other way.)
    readonly property bool desc: mine.sort ? !!mine.desc : (flagSort === "date" || flagSort === "size") !== flagReverse
    readonly property bool foldersFirst: mine.folders_first !== undefined ? !!mine.folders_first : flagFoldersFirst
    readonly property bool showHidden: flagAll || !!mine.hidden
    function save(k, v) {
        const m = Object.assign({}, mine)
        m[k] = v
        mine = m
        keptChanged2(m)
        const s = Object.assign({}, saved)
        s[k] = v
        if (controller) controller.setSetting("file_browser", JSON.stringify(s))
    }
    function setView(v) { save("view", v) }

    // ---- where it is ----------------------------------------------------
    property string dir: startDir
    property var back: []
    property var forward: []
    function go(d) {
        if (d === dir) return
        back = back.concat([dir]); forward = []
        dir = d
        selected = ""
    }
    function goBack() { if (!back.length) return; forward = [dir].concat(forward); dir = back[back.length - 1]; back = back.slice(0, -1) }
    function goForward() { if (!forward.length) return; back = back.concat([dir]); dir = forward[0]; forward = forward.slice(1) }
    // (Another host's folder: "ssh://<host>/<path>", its files read there.)
    readonly property string hostPrefix: (dir.match(/^ssh:\/\/[^/]*/) || [""])[0]
    readonly property string bareDir: dir.substring(hostPrefix.length) || "/"
    function up() { const p = bareDir.replace(/\/[^/]+\/?$/, ""); go(hostPrefix + (p === "" ? "/" : p)) }
    onStartDirChanged: { dir = startDir; back = []; forward = [] }

    // (Read in the background; the last listing stays up until the new one comes.)
    property var raw: []
    property int listSeq: 0
    property bool listed: false         // (an answer came: "(empty)" means it)
    function list() {
        const n = ++listSeq
        if (!controller || dir === "") { raw = []; return }
        Async.call(controller, k => controller.listDir(k, dir, showHidden), r => {
            if (n !== listSeq) return   // (another folder asked for since)
            try { raw = JSON.parse(r) } catch (e) { raw = [] }
            listed = true
        })
    }
    onRefreshChanged: list()
    onDirChanged: list()
    onShowHiddenChanged: list()
    onControllerChanged: list()
    readonly property var entries: {
        const a = onlyName === "" ? raw.slice() : raw.filter(x => x.name === onlyName)
        const name = (x) => x.name.toLowerCase().replace(/^\./, "")
        const cmp = {
            name: (x, y) => name(x).localeCompare(name(y), undefined, { numeric: true }),
            size: (x, y) => (x.dir ? x.items : x.size) - (y.dir ? y.items : y.size) || name(x).localeCompare(name(y)),
            date: (x, y) => x.mtime - y.mtime || name(x).localeCompare(name(y)),
            type: (x, y) => ext(x).localeCompare(ext(y)) || name(x).localeCompare(name(y)),
            none: (x, y) => 0,
        }[sortBy] || ((x, y) => 0)
        a.sort((x, y) => {
            if (foldersFirst && x.dir !== y.dir) return x.dir ? -1 : 1
            const r = cmp(x, y)
            return desc ? -r : r
        })
        return a
    }
    function ext(e) { return e.dir ? "" : (e.name.indexOf(".") > 0 ? e.name.replace(/^.*\./, "").toLowerCase() : "") }

    // ---- how an entry looks (lsd's way) ---------------------------------
    function ansi(n, d) { const a = themeData.ansi || []; return a[n] || d }
    function tint(e) {
        if (e.dir) return ansi(4, "#61afef")
        if (e.link) return ansi(6, "#56b6c2")
        if (e.exec) return ansi(2, "#98c379")
        return ({ image: ansi(5, "#c678dd"), video: ansi(5, "#c678dd"), audio: ansi(6, "#56b6c2"), archive: ansi(1, "#e06c75"), document: ansi(3, "#e5c07b") })[e.kind] || Theme.text
    }
    function icon(e) {
        if (e.dir) return e.link ? "" : ""
        if (e.link) return ""
        return ({ image: "", video: "", audio: "", archive: "", document: "", text: "",
                  config: "", code: "", executable: "" })[e.kind] || ""
    }
    function human(n) {
        if (n < 1024) return n + " B"
        const u = ["KB", "MB", "GB", "TB"]
        let i = -1
        do { n /= 1024; i++ } while (n >= 1024 && i < u.length - 1)
        return (n < 10 ? n.toFixed(1) : Math.round(n)) + " " + u[i]
    }
    function when(t) {
        const d = new Date(t * 1000)
        return Qt.formatDateTime(d, "ddd MMM d HH:mm:ss yyyy").replace(/ (\d) /, "  $1 ")
    }
    function esc(s) { return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;") }
    function label(e) {
        return "<span style=\"font-family:'Symbols Nerd Font Mono'\">" + icon(e) + "</span> " + esc(e.name) + (e.dir ? "/" : "")
    }
    FontMetrics { id: fm; font.family: fb.fontFamily; font.pointSize: fb.pointSize }
    readonly property real cw: fm.averageCharacterWidth > 0 ? fm.averageCharacterWidth : 8
    readonly property real row: Math.max(fm.height, rowHeight)

    // ---- ls's column layout: as many columns as fit, filled going down ----
    readonly property var grid: {
        const n = entries.length
        const w = entries.map(e => (e.name.length + 3 + (e.dir ? 1 : 0)) * cw)
        const avail = Math.max(cw * 10, width - 8)
        let best = { cols: 1, rows: n, widths: [Math.max(0, ...w)] }
        for (let cols = 2; cols <= n; cols++) {
            const rows = Math.ceil(n / cols)
            if (Math.ceil(n / rows) < cols) continue
            const widths = []
            for (let c = 0; c < cols; c++) widths.push(Math.max(0, ...w.slice(c * rows, (c + 1) * rows)) + 2 * cw)
            if (widths.reduce((a, b) => a + b, 0) > avail) break
            best = { cols: cols, rows: rows, widths: widths }
        }
        return best
    }
    readonly property var colX: { const x = [0]; for (const w of grid.widths) x.push(x[x.length - 1] + w); return x }
    // `ls -l`'s columns, padded to their widest.
    readonly property var longW: {
        const m = (f) => Math.max(1, ...entries.map(f))
        return { user: m(e => e.user.length), group: m(e => e.group.length), size: m(e => sizeText(e).length) }
    }
    function sizeText(e) { return e.dir ? (e.items >= 0 ? e.items + (e.items === 1 ? " item" : " items") : "") : human(e.size) }
    function pad(s, n, right) { s = String(s); while (s.length < n) s = right ? " " + s : s + " "; return s }

    readonly property real tileW: Math.max(96, cw * 14)
    readonly property int tileCols: Math.max(1, Math.floor((width - 8) / (tileW + 8)))
    readonly property real headerH: (back.length > 0 || dir !== startDir) ? row + 6 : 0
    readonly property real bodyH: entries.length === 0 ? row
                                : view === "details" ? entries.length * row
                                : view === "tiles" ? Math.ceil(entries.length / tileCols) * (tileW + 24)
                                : grid.rows * row
    implicitHeight: headerH + bodyH + 8

    property string selected: ""        // a path
    property double entryTapAt: 0       // an entry was just clicked (not the space around them)
    property var menuEntry: null        // what the right click was on (null: the folder)
    property string error: ""
    // (`then` gets whether it worked, once the answer comes.)
    function act(op, path, arg, then) {
        Async.call(controller, k => controller.fileAction(k, op, path, arg || ""), r => {
            // (A tool from Nix, with no Nix: what Nix is and how to get it.)
            if (r.startsWith("needs-nix:")) { error = ""; Window.window.nixHelp(r.substring(10), hostPrefix.replace(/^ssh:\/\//, "")); if (then) then(false); return }
            error = r
            if (r === "") refresh++
            if (then) then(r === "")
        })
    }
    // A click: a folder goes in; a text file that isn't a program shows in a
    // new card below, in Tessera's viewer (line numbers, highlighting);
    // anything else opens with its app.
    function openEntry(e) {
        if (e.dir) go(e.path)
        else if (e.text && !e.exec) fileCardRequested(e.path)
        else act("open", e.path)
    }
    // ---- drag and drop -----------------------------------------------------
    // Files dropped here (from this browser, another, or a file manager):
    // into `into`. From inside Tessera they move, from elsewhere they're
    // copied; Shift moves, Ctrl copies (the drop's proposed action).
    function pathOf(u) { let p = u.toString().replace(/^file:\/\/[^/]*/, ""); try { p = decodeURIComponent(p) } catch (e) {} return p }
    function dropInto(into, drop) {
        if (!drop.hasUrls) return
        const internal = drop.source !== null && drop.source !== undefined
        const move = drop.proposedAction === Qt.MoveAction || (internal && drop.proposedAction !== Qt.CopyAction)
        // (Each one reads the folder again when it's done.)
        for (const u of drop.urls) {
            const p = pathOf(u)
            if (p === "" || p === into || p.replace(/\/[^/]+$/, "") === into) continue
            act(move ? "move" : "copy", p, into)
        }
        drop.accept(move ? Qt.MoveAction : Qt.CopyAction)
    }
    property string dropOn: ""          // the folder a drag is over
    DropArea {
        anchors.fill: parent
        keys: ["text/uri-list"]
        onDropped: (drop) => fb.dropInto(fb.dir, drop)
    }
    function shq(s) { return /^[A-Za-z0-9_\/.~+:@-]+$/.test(s) ? s : "'" + s.replace(/'/g, "'\\''") + "'" }

    // ---- the path bar (once away from where it started) -----------------
    Row {
        id: pathBar
        visible: fb.headerH > 0
        y: 2
        x: 2
        height: fb.row
        spacing: 2
        component NavButton: Text {
            property bool on: true
            signal clicked()
            width: fb.cw * 2.5
            height: fb.row
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            color: on ? (nh.hovered ? Theme.text : Theme.textDim) : Theme.divider
            font.pointSize: fb.pointSize
            font.underline: on && nh.hovered
            HoverHandler { id: nh; cursorShape: Qt.PointingHandCursor }
            TapHandler { enabled: parent.on; onTapped: parent.clicked() }
        }
        NavButton { text: "←"; on: fb.back.length > 0; onClicked: fb.goBack() }
        NavButton { text: "→"; on: fb.forward.length > 0; onClicked: fb.goForward() }
        NavButton { text: "↑"; on: fb.dir !== "/"; onClicked: fb.up() }
        Repeater {
            // Each folder of the path, to go back up to.
            model: {
                const parts = fb.bareDir.split("/").filter(p => p !== "")
                const host = fb.hostPrefix !== "" ? decodeURIComponent(fb.hostPrefix.substring(6)) + ":/" : "/"
                const out = [{ name: host, path: fb.hostPrefix + "/" }]
                let acc = ""
                for (const p of parts) { acc += "/" + p; out.push({ name: p, path: fb.hostPrefix + acc }) }
                return out
            }
            delegate: Text {
                required property var modelData
                required property int index
                height: fb.row
                verticalAlignment: Text.AlignVCenter
                text: (index > 1 ? "/" : "") + modelData.name
                color: ph.hovered ? fb.ansi(4, "#61afef") : Theme.textDim
                font.family: fb.fontFamily
                font.pointSize: fb.pointSize
                font.underline: ph.hovered
                HoverHandler { id: ph; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: fb.go(modelData.path) }
            }
        }
        Text {
            height: fb.row
            verticalAlignment: Text.AlignVCenter
            text: "   cd here"
            color: ch.hovered ? fb.ansi(2, "#98c379") : Theme.textDim
            font.pointSize: fb.pointSize - 1
            font.underline: ch.hovered
            HoverHandler { id: ch; cursorShape: Qt.PointingHandCursor }
            TapHandler { onTapped: fb.commandRequested("cd " + fb.shq(fb.dir)) }
        }
    }

    // ---- the entries ----------------------------------------------------
    // (Scrolled in the room it's given: a card shorter than the folder's
    // list, or one dragged smaller, cut it off with no way to the rest.)
    Flickable {
        id: bodyScroll
        y: fb.headerH + 4
        width: parent.width
        height: Math.max(0, Math.min(fb.bodyH, fb.height - fb.headerH - 8))
        contentWidth: width
        contentHeight: fb.bodyH
        clip: contentHeight > height
        interactive: false   // (a drag is a file's: the wheel and the bar scroll)
        boundsBehavior: Flickable.StopAtBounds
        ScrollBar.vertical: ScrollBar { policy: bodyScroll.contentHeight > bodyScroll.height + 1 ? ScrollBar.AlwaysOn : ScrollBar.AlwaysOff }
        MomentumWheel { flick: bodyScroll; enabled: bodyScroll.contentHeight > bodyScroll.height + 1 }
    Item {
        id: body
        width: parent.width
        height: fb.bodyH
        // (A right click on nothing: the folder's own menu.)
        // (Only on nothing: a click on an entry reaches these too, after it.)
        TapHandler { acceptedButtons: Qt.RightButton; onTapped: (pt) => { const x = pt.position.x, y = pt.position.y; Qt.callLater(() => { if (Date.now() - fb.entryTapAt > 300) { fb.menuEntry = null; folderMenu.popup(parent, x, y) } }) } }
        TapHandler { onTapped: Qt.callLater(() => { if (Date.now() - fb.entryTapAt > 300) fb.selected = "" }) }
        Text {
            visible: fb.entries.length === 0
            text: fb.listed && fb.raw.length === 0 && fb.dir !== "" ? "(empty)" : ""
            color: Theme.textDim
            font.family: fb.fontFamily
            font.pointSize: fb.pointSize
        }
        Repeater {
            model: fb.entries
            delegate: Rectangle {
                id: ent
                required property var modelData
                required property int index
                readonly property var e: modelData
                readonly property bool sel: fb.selected === e.path
                readonly property int c: Math.floor(index / Math.max(1, fb.grid.rows))
                readonly property int r: index % Math.max(1, fb.grid.rows)
                x: fb.view === "details" ? 0 : fb.view === "tiles" ? (index % fb.tileCols) * (fb.tileW + 8) : (fb.colX[c] || 0)
                y: fb.view === "details" ? index * fb.row : fb.view === "tiles" ? Math.floor(index / fb.tileCols) * (fb.tileW + 24) : r * fb.row
                width: fb.view === "details" ? body.width : fb.view === "tiles" ? fb.tileW : Math.max(0, (fb.grid.widths[c] || 0) - fb.cw)
                height: fb.view === "tiles" ? fb.tileW + 20 : fb.row
                radius: 3
                color: sel ? Theme.mix(Theme.bg, Theme.accent, 0.3) : eh.hovered ? Theme.mix(Theme.bg, Theme.fg, 0.07) : "transparent"
                HoverHandler { id: eh; cursorShape: Qt.PointingHandCursor }
                // (Dragged out: as a file, to a file manager, an editor, the desktop.)
                Drag.dragType: Drag.Automatic
                Drag.supportedActions: Qt.CopyAction | Qt.MoveAction | Qt.LinkAction
                Drag.mimeData: ({ "text/uri-list": "file://" + encodeURI(ent.e.path) + "\r\n", "text/plain": ent.e.path })
                Drag.active: entDrag.active
                DragHandler { id: entDrag; target: null }
                // (A folder takes what's dropped on it.)
                DropArea {
                    anchors.fill: parent
                    enabled: ent.e.dir
                    keys: ["text/uri-list"]
                    onEntered: fb.dropOn = ent.e.path
                    onExited: if (fb.dropOn === ent.e.path) fb.dropOn = ""
                    onDropped: (drop) => { fb.dropOn = ""; fb.dropInto(ent.e.path, drop) }
                }
                border.color: fb.dropOn === ent.e.path ? Theme.accent : "transparent"
                border.width: 1
                TapHandler {
                    // (One click: a folder goes in, a file opens with its app.)
                    onTapped: { fb.entryTapAt = Date.now(); fb.selected = ent.e.path; fb.openEntry(ent.e) }
                }
                // (At the click: popup() with no place put it at the far left
                // in a notebook card.)
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: (pt) => { fb.entryTapAt = Date.now(); fb.selected = ent.e.path; fb.menuEntry = ent.e; itemMenu.popup(ent, pt.position.x, pt.position.y) } }
                ToolTip.visible: eh.hovered && fb.view !== "details"
                ToolTip.delay: 900
                ToolTip.text: ent.e.mode + "  " + ent.e.user + " " + ent.e.group + "  " + fb.sizeText(ent.e) + "\n" + fb.when(ent.e.mtime) + (ent.e.link ? "\n→ " + ent.e.target : "")
                // list: icon and name
                Text {
                    visible: fb.view === "list"
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    textFormat: Text.StyledText
                    text: fb.label(ent.e)
                    color: fb.tint(ent.e)
                    font.family: fb.fontFamily
                    font.pointSize: fb.pointSize
                    font.bold: ent.e.dir || ent.e.exec
                    font.underline: eh.hovered
                }
                // details: ls -l
                Text {
                    visible: fb.view === "details"
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    textFormat: Text.StyledText
                    // (Spaces kept: rich text would fold the columns' padding.)
                    // (StyledText colors by <font color>, not a span's style.)
                    text: "<font color='" + Theme.textDim + "'>" + (ent.e.mode + " " + fb.esc(fb.pad(ent.e.user, fb.longW.user)) + " " + fb.esc(fb.pad(ent.e.group, fb.longW.group))
                          + " " + fb.pad(fb.sizeText(ent.e), fb.longW.size, true) + " " + fb.when(ent.e.mtime)).replace(/ /g, "&nbsp;") + "</font> "
                          + "<font color='" + fb.tint(ent.e) + "'>" + (ent.e.dir || ent.e.exec ? "<b>" : "") + (eh.hovered ? "<u>" : "") + fb.label(ent.e) + (eh.hovered ? "</u>" : "") + (ent.e.dir || ent.e.exec ? "</b>" : "") + "</font>"
                          + (ent.e.link ? "<font color='" + Theme.textDim + "'> ⇒ " + fb.esc(ent.e.target) + "</font>" : "")
                    color: Theme.text
                    font.family: fb.fontFamily
                    font.pointSize: fb.pointSize
                }
                // tiles: a thumbnail or a big icon, the name under it
                Item {
                    visible: fb.view === "tiles"
                    anchors.fill: parent
                    Image {
                        id: thumb
                        visible: ent.e.kind === "image" && status === Image.Ready
                        x: 6; y: 4
                        width: parent.width - 12
                        height: fb.tileW - 8
                        source: fb.view === "tiles" && ent.e.kind === "image" ? "file://" + ent.e.path : ""
                        sourceSize.width: 192
                        sourceSize.height: 192
                        asynchronous: true
                        fillMode: Image.PreserveAspectFit
                    }
                    Text {
                        visible: !thumb.visible
                        anchors.horizontalCenter: parent.horizontalCenter
                        y: (fb.tileW - height) / 2 + 0.001
                        text: fb.icon(ent.e)
                        color: fb.tint(ent.e)
                        font.family: "Symbols Nerd Font Mono"
                        font.pixelSize: fb.tileW * 0.45
                    }
                    Text {
                        y: fb.tileW + 0.001
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        text: ent.e.name + (ent.e.dir ? "/" : "")
                        elide: Text.ElideMiddle
                        color: fb.tint(ent.e)
                        font.family: fb.fontFamily
                        font.pointSize: fb.pointSize - 1
                        font.bold: ent.e.dir
                        font.underline: eh.hovered
                    }
                }
            }
        }
    }
    }
    Text {
        visible: fb.error !== ""
        anchors.bottom: parent.bottom
        anchors.right: parent.right
        anchors.margins: 4
        text: fb.error
        color: fb.ansi(1, "#e06c75")
        font.pixelSize: 11
        TapHandler { onTapped: fb.error = "" }
    }

    // ---- the bar (on hover): view, sort, folders first, hidden ----------
    HoverHandler { id: fbHover }
    Rectangle {
        id: bar
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.topMargin: 2
        z: 5
        width: barRow.implicitWidth + 8
        height: 22
        radius: 6
        color: Theme.mix(Theme.bg, Theme.fg, 0.1)
        border.color: Theme.divider
        opacity: fbHover.hovered || itemMenu.opened || folderMenu.opened || sortMenu.opened ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 120 } }
        Row {
            id: barRow
            anchors.centerIn: parent
            spacing: 2
            component BarItem: Rectangle {
                property string label
                property string icon: ""        // (a Nerd Font glyph before the label, a gap after it)
                property bool on: false
                property string tip
                signal clicked()
                width: btRow.implicitWidth + 10
                height: 18
                radius: 4
                color: on ? Theme.mix(Theme.bg, Theme.accent, 0.4) : bh.hovered ? Theme.tabHover : "transparent"
                Row {
                    id: btRow
                    anchors.centerIn: parent
                    anchors.verticalCenterOffset: 0.001
                    spacing: 4
                    Text { visible: text !== ""; anchors.verticalCenter: parent.verticalCenter; text: btRow.parent.icon; color: Theme.text; font.family: "Symbols Nerd Font Mono"; font.pixelSize: 11; renderType: Text.NativeRendering }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: btRow.parent.label; color: Theme.text; font.pixelSize: 11; font.underline: bh.hovered; renderType: Text.NativeRendering }
                }
                HoverHandler { id: bh; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: parent.clicked() }
                ToolTip.visible: bh.hovered && tip !== ""
                ToolTip.delay: 600
                ToolTip.text: tip
            }
            BarItem { label: "☰"; tip: "List (as ls)"; on: fb.view === "list"; onClicked: fb.setView("list") }
            BarItem { label: "≡"; tip: "Details (as ls -l)"; on: fb.view === "details"; onClicked: fb.setView("details") }
            BarItem { label: "▦"; tip: "Tiles"; on: fb.view === "tiles"; onClicked: fb.setView("tiles") }
            BarItem { id: sortItem; label: "Sort: " + ({ name: "name", size: "size", date: "date", type: "type" })[fb.sortBy] + (fb.desc ? " ↓" : " ↑"); onClicked: sortMenu.popup(sortItem, 0, sortItem.height + 2) }
            BarItem { icon: ""; label: "first"; tip: "Folders first"; on: fb.foldersFirst; onClicked: fb.save("folders_first", !fb.foldersFirst) }
            BarItem { label: ".hidden"; tip: "Show hidden files"; on: fb.showHidden; onClicked: fb.save("hidden", !fb.mine.hidden) }
        }
    }
    SlimMenu {
        id: sortMenu
        Repeater {
            model: [["name", "Name"], ["size", "Size"], ["date", "Date modified"], ["type", "Type"]]
            delegate: SlimMenuItem {
                required property var modelData
                text: modelData[1]
                checkable: true
                checked: fb.sortBy === modelData[0]
                onTriggered: fb.save("sort", modelData[0])
            }
        }
        SlimMenuItem { text: "Descending"; checkable: true; checked: fb.desc; onTriggered: fb.save("desc", !fb.desc) }
    }

    // ---- right click: an entry's actions ----------------------------------
    TextEdit { id: clip; visible: false }
    function later(ms, f) { const t = Qt.createQmlObject("import QtQuick; Timer {}", fb); t.interval = ms; t.triggered.connect(() => { f(); t.destroy() }); t.start() }
    function copyText(t) { clip.text = t; clip.selectAll(); clip.copy() }
    component Sep: MenuSeparator {
        topPadding: 3; bottomPadding: 3
        contentItem: Rectangle { implicitWidth: 120; implicitHeight: 1; color: Theme.divider }
    }
    property var held: null             // {op: "copy"|"move", path}: for Paste (another host's, or copied here)
    // ---- the desktop's clipboard: files to and from file managers ------------
    FileClip {
        id: fileClip
        // (Something else copied since: that's what Paste takes now.)
        onChanged: { if (fb.putting) fb.putting = false; else fb.held = null }
    }
    property bool putting: false
    function local(path) { return !/^ssh:\/\//.test(path) }
    function copyFile(path, cut) {
        held = { op: cut ? "move" : "copy", path: path }
        if (local(path)) { putting = true; fileClip.put([path], cut) }
    }
    readonly property bool canPaste: held !== null || fileClip.hasFiles
    // (Into `dest`: what was copied or cut here, else the clipboard's files.)
    function pasteInto(dest) {
        if (held) {
            const h = held
            act(h.op, h.path, dest, ok => { if (ok && h.op === "move" && held === h) { held = null; if (local(h.path)) fileClip.clear() } })
            return
        }
        const paths = fileClip.files(), op = fileClip.cut ? "move" : "copy"
        const next = (i) => { if (i < paths.length) act(op, paths[i], dest, ok => { if (ok) next(i + 1) }); else if (op === "move") fileClip.clear() }
        next(0)
    }
    // (Its app list, read as the menu opens: [{id, name, icon, default}].)
    property var apps: []
    property string appsMime: ""
    function loadApps(path) {
        apps = []; appsMime = ""
        Async.call(controller, k => controller.fileApps(k, path), r => { try { const j = JSON.parse(r); apps = j.apps || []; appsMime = j.mime || "" } catch (x) {} })
    }
    readonly property bool isArchive: !!(menuEntry && !menuEntry.dir && /\.(tar(\.(gz|xz|bz2|zst))?|tgz|zip|7z|rar)$/i.test(menuEntry.name))
    // (Disk use, to look through and clean up: gdu, a TUI, in the card's
    // live terminal; from Nix when it isn't installed.)
    function diskUsage(path) {
        const q = shq(path)
        // (Another host's folder: its gdu, else Nix's there, decided by its shell.)
        if (hostPrefix !== "") {
            const b = shq(bareOf(path))
            commandRequested("if command -v gdu >/dev/null; then gdu " + b + "; elif command -v nix >/dev/null; then nix run nixpkgs#gdu -- " + b
                             + "; else printf '%s\\n' " + shq(Window.window.noNixText("gdu")) + "; fi")
            return
        }
        // (Neither: what Nix is and how to install it.)
        Async.call(controller, k => controller.findProgram(k, "gdu"), found => {
            if (found !== "") { commandRequested("gdu " + q); return }
            Async.call(controller, k => controller.findProgram(k, "nix"), nix => {
                if (nix !== "") commandRequested("nix run nixpkgs#gdu -- " + q)
                else Window.window.nixHelp("gdu", "")
            })
        })
    }
    function bareOf(p) { return p.replace(/^ssh:\/\/[^/]*/, "") || "/" }
    SlimMenu {
        id: itemMenu
        readonly property var e: fb.menuEntry || ({ dir: false, exec: false, path: "", name: "" })
        onAboutToShow: fb.loadApps(e.path)
        SlimMenuItem { text: "Open"; font.bold: true; onTriggered: fb.openEntry(itemMenu.e) }
        SlimMenu {
            title: "Open with ▸"
            enabled: fb.apps.length > 0
            topPadding: 4.001
            Repeater {
                model: fb.apps
                delegate: SlimMenuItem {
                    required property var modelData
                    text: modelData.name + (modelData.default ? "  (default)" : "")
                    onTriggered: fb.act("open_with", itemMenu.e.path, modelData.id)
                }
            }
        }
        SlimMenu {
            title: "Always open " + (itemMenu.e.dir ? "folders" : fb.ext(itemMenu.e) !== "" ? "." + fb.ext(itemMenu.e) + " files" : fb.appsMime !== "" ? fb.appsMime + " files" : "these") + " with ▸"
            enabled: fb.apps.length > 0
            topPadding: 4.001
            Repeater {
                model: fb.apps
                delegate: SlimMenuItem {
                    required property var modelData
                    text: modelData.name
                    checkable: true
                    checked: !!modelData.default
                    onTriggered: fb.act("set_default", itemMenu.e.path, modelData.id, ok => { if (ok) fb.loadApps(itemMenu.e.path) })
                }
            }
        }
        SlimMenuItem { text: "View in a card"; visible: !itemMenu.e.dir; height: visible ? implicitHeight : 0; onTriggered: fb.fileCardRequested(itemMenu.e.path) }
        SlimMenuItem { text: "View in Tessera's viewer"; visible: !itemMenu.e.dir; height: visible ? implicitHeight : 0; onTriggered: fb.viewRequested(itemMenu.e.path) }
        SlimMenuItem { text: itemMenu.e.dir ? "Open in the file manager" : "Show in the file manager"; onTriggered: fb.act(itemMenu.e.dir ? "open" : "reveal", itemMenu.e.path) }
        SlimMenuItem { text: itemMenu.e.dir ? "Go here in the shell (cd)" : "Run it"; visible: !!(itemMenu.e.dir || itemMenu.e.exec); height: visible ? implicitHeight : 0
            onTriggered: fb.commandRequested(itemMenu.e.dir ? "cd " + fb.shq(itemMenu.e.path) : fb.shq(itemMenu.e.path)) }
        Sep {}
        SlimMenuItem { text: "Cut"; onTriggered: fb.copyFile(itemMenu.e.path, true) }
        SlimMenuItem { text: "Copy"; onTriggered: fb.copyFile(itemMenu.e.path, false) }
        SlimMenuItem { text: "Paste into it"; visible: itemMenu.e.dir; height: visible ? implicitHeight : 0; enabled: fb.canPaste; onTriggered: fb.pasteInto(itemMenu.e.path) }
        SlimMenuItem { text: "Copy the path"; onTriggered: fb.copyText(itemMenu.e.path) }
        SlimMenuItem { text: "Copy the name"; onTriggered: fb.copyText(itemMenu.e.name) }
        SlimMenuItem { text: "Insert the path in the editor"; onTriggered: fb.insertRequested(fb.shq(itemMenu.e.path) + " ") }
        Sep {}
        SlimMenuItem { text: "Rename…"; onTriggered: fb.ask("rename", itemMenu.e) }
        SlimMenuItem { text: "Duplicate"; onTriggered: fb.act("duplicate", itemMenu.e.path) }
        SlimMenuItem { text: itemMenu.e.exec ? "Not a program (chmod -x)" : "Make it a program (chmod +x)"; visible: !itemMenu.e.dir; height: visible ? implicitHeight : 0; onTriggered: fb.act("chmod_x", itemMenu.e.path) }
        SlimMenu {
            title: "Compress ▸"
            topPadding: 4.001
            SlimMenuItem { text: "To " + itemMenu.e.name + ".tar.gz"; onTriggered: fb.act("compress", itemMenu.e.path, "tar.gz") }
            SlimMenuItem { text: "To " + itemMenu.e.name + ".zip"; onTriggered: fb.act("compress", itemMenu.e.path, "zip") }
        }
        SlimMenuItem { text: "Extract here"; visible: fb.isArchive; height: visible ? implicitHeight : 0; onTriggered: fb.act("extract", itemMenu.e.path) }
        SlimMenuItem { text: "Disk usage…"; visible: itemMenu.e.dir; height: visible ? implicitHeight : 0; onTriggered: fb.diskUsage(itemMenu.e.path) }
        Sep {}
        SlimMenuItem { text: "Move to the trash"; onTriggered: fb.act("trash", itemMenu.e.path) }
        SlimMenuItem { text: "Delete for good…"; onTriggered: fb.confirmDelete(itemMenu.e) }
        Sep {}
        SlimMenuItem { text: "Properties"; onTriggered: { fb.propsFor = itemMenu.e; props.open() } }
    }
    // ---- right click on nothing: the folder's actions ---------------------
    SlimMenu {
        id: folderMenu
        SlimMenuItem { text: "Open in the file manager"; onTriggered: fb.act("open", fb.dir) }
        SlimMenuItem { text: "Go here in the shell (cd)"; onTriggered: fb.commandRequested("cd " + fb.shq(fb.dir)) }
        Sep {}
        SlimMenuItem { text: "New folder…"; onTriggered: fb.ask("mkdir", null) }
        SlimMenuItem { text: "New file…"; onTriggered: fb.ask("touch", null) }
        SlimMenuItem { text: "Paste"; enabled: fb.canPaste; onTriggered: fb.pasteInto(fb.dir) }
        SlimMenuItem { text: "Copy the folder's path"; onTriggered: fb.copyText(fb.dir) }
        SlimMenuItem { text: "Disk usage…"; onTriggered: fb.diskUsage(fb.dir) }
        Sep {}
        SlimMenuItem { text: "List (as ls)"; checkable: true; checked: fb.view === "list"; onTriggered: fb.setView("list") }
        SlimMenuItem { text: "Details (as ls -l)"; checkable: true; checked: fb.view === "details"; onTriggered: fb.setView("details") }
        SlimMenuItem { text: "Tiles"; checkable: true; checked: fb.view === "tiles"; onTriggered: fb.setView("tiles") }
        SlimMenuItem { text: "Sort…"; onTriggered: sortMenu.popup() }
        SlimMenuItem { text: "Folders first"; checkable: true; checked: fb.foldersFirst; onTriggered: fb.save("folders_first", !fb.foldersFirst) }
        SlimMenuItem { text: "Show hidden files"; checkable: true; checked: fb.showHidden; onTriggered: fb.save("hidden", !fb.mine.hidden) }
        SlimMenuItem { text: "Refresh"; onTriggered: fb.refresh++ }
    }
    // ---- delete for good: asked first ---------------------------------------
    property var deleting: null
    function confirmDelete(e) { deleting = e; delPop.open() }
    Popup {
        id: delPop
        anchors.centerIn: parent
        width: 340
        padding: 12
        topPadding: 12.001   // (off the half pixel: 130% scale smears native text there)
        modal: true
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        contentItem: Column {
            spacing: 8
            Text { width: parent.width; wrapMode: Text.Wrap; color: Theme.text; font.pixelSize: 13; font.weight: Font.DemiBold
                   text: fb.deleting ? "Delete " + fb.deleting.name + (fb.deleting.dir ? " and everything in it" : "") + " for good?" : "" }
            Text { width: parent.width; wrapMode: Text.Wrap; color: Theme.textDim; font.pixelSize: 12; text: "It doesn't go to the trash: it can't be brought back." }
            Row {
                anchors.right: parent.right
                spacing: 6
                Chip { base: Theme.panel; label: "Cancel"; onClicked: delPop.close() }
                Chip { base: Theme.mix(Theme.panel, fb.ansi(1, "#e06c75"), 0.35); label: "Delete for good"; onClicked: { const e = fb.deleting; delPop.close(); if (e) fb.act("delete", e.path) } }
            }
        }
    }

    // ---- a name to ask for (rename, new folder, new file) ------------------
    property string asking: ""
    property var askFor: null
    function ask(op, e) {
        asking = op
        askFor = e
        nameField.text = op === "rename" ? e.name : op === "mkdir" ? "New Folder" : "New File.txt"
        namePop.open()
        // (The name without its extension selected, as file managers do.)
        Qt.callLater(() => { nameField.forceActiveFocus(); const dot = nameField.text.lastIndexOf("."); nameField.select(0, op !== "mkdir" && dot > 0 && !(e && e.dir) ? dot : nameField.text.length) })
    }
    Popup {
        id: namePop
        anchors.centerIn: parent
        width: 300
        padding: 10
        topPadding: 10.001   // (off the half pixel: 130% scale smears native text there)
        modal: true
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        contentItem: Column {
            spacing: 6
            Text { text: ({ rename: "Rename", mkdir: "New folder", touch: "New file" })[fb.asking] || ""; color: Theme.text; font.pixelSize: 13; font.weight: Font.DemiBold }
            TextField {
                id: nameField
                width: parent.width
                color: Theme.text
                font.pixelSize: 12
                background: Rectangle { radius: 5; color: Theme.bg; border.color: nameField.activeFocus ? Theme.accent : Theme.divider }
                Keys.onEscapePressed: namePop.close()
                onAccepted: {
                    const close = ok => { if (ok) namePop.close() }
                    if (fb.asking === "rename") fb.act("rename", fb.askFor.path, text, close)
                    else fb.act(fb.asking, fb.dir, text, close)
                }
            }
            Text { visible: fb.error !== ""; width: parent.width; wrapMode: Text.Wrap; text: fb.error; color: fb.ansi(1, "#e06c75"); font.pixelSize: 11 }
        }
    }
    // ---- properties ------------------------------------------------------
    property var propsFor: null
    Popup {
        id: props
        anchors.centerIn: parent
        width: 360
        padding: 12
        topPadding: 12.001   // (off the half pixel: 130% scale smears native text there)
        background: Rectangle { radius: 8; color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.04); border.color: Theme.divider }
        contentItem: Column {
            spacing: 4
            Text { text: fb.propsFor ? fb.label(fb.propsFor) : ""; textFormat: Text.StyledText; color: fb.propsFor ? fb.tint(fb.propsFor) : Theme.text; font.family: fb.fontFamily; font.pointSize: fb.pointSize + 1; font.bold: true }
            Repeater {
                model: fb.propsFor ? [["Where", fb.propsFor.path], ["Kind", fb.propsFor.kind + (fb.propsFor.link ? " (link to " + fb.propsFor.target + ")" : "")],
                                      ["Size", fb.sizeText(fb.propsFor) + (fb.propsFor.dir ? "" : " (" + fb.propsFor.size + " bytes)")],
                                      ["Modified", fb.when(fb.propsFor.mtime)], ["Permissions", fb.propsFor.mode], ["Owner", fb.propsFor.user + " : " + fb.propsFor.group]] : []
                delegate: Row {
                    required property var modelData
                    spacing: 8
                    Text { width: 80; text: modelData[0]; color: Theme.textDim; font.pixelSize: 11 }
                    Text { width: 250; text: modelData[1]; wrapMode: Text.WrapAnywhere; color: Theme.text; font.family: fb.fontFamily; font.pixelSize: 11 }
                }
            }
        }
    }
    // (Test hook: TESSERA_TEST_FB=go:<name> | menu:<name> | view:<name> | folder | tiles | details | bar | du.)
    // (…@<folder> only for the card of that folder; …#<ms> after that long.)
    Timer {
        readonly property string spec: fb.controller ? fb.controller.testEnv("FB") : ""
        interval: spec.indexOf("#") >= 0 ? parseInt(spec.split("#")[1]) : 2500
        running: spec !== "" && (spec.indexOf("@") < 0 || fb.startDir === spec.split("@")[1].split("#")[0])
        onTriggered: {
            const t = spec.split("@")[0].split("#")[0], arg = t.substring(t.indexOf(":") + 1)
            const e = fb.entries.find(x => x.name === arg)
            if (t.startsWith("go:") && e) fb.openEntry(e)
            else if (t.startsWith("menu:") && e) { fb.selected = e.path; fb.menuEntry = e; itemMenu.popup(fb, 40, 30) }
            else if (t.startsWith("view:") && e) fb.viewRequested(e.path)
            else if (t === "folder") folderMenu.popup(fb, 40, 30)
            else if (t === "tiles" || t === "details" || t === "list") fb.setView(t)
            else if (t === "du") fb.diskUsage(fb.dir)
            // (apps:<name> logs its apps; act:<op>:<name>[:<arg>] does op on it;
            // paste:<name> copies it to the desktop's clipboard, then pastes into <name>.d.)
            else if (t.startsWith("apps:") && e) { fb.loadApps(e.path); fb.later(1500, () => console.log("fb test apps: " + fb.appsMime + " " + JSON.stringify(fb.apps.map(a => a.name + (a.default ? "*" : ""))))) }
            else if (t.startsWith("act:")) {
                const [, op, nm, a2] = t.split(":")
                const x = fb.entries.find(y => y.name === nm)
                if (x) fb.act(op, x.path, a2 || "", ok => console.log("fb test act " + op + " " + nm + ": " + (ok ? "ok" : fb.error)))
            }
            else if (t.startsWith("paste:") && e) {
                fb.copyFile(e.path, false)
                fb.later(800, () => { fb.held = null; console.log("fb test clip: " + JSON.stringify(fileClip.files()) + " cut " + fileClip.cut); fb.pasteInto(fb.dir + "/" + arg + ".d") })
            }
        }
    }
}
