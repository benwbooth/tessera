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
    // (Which of the folder's entries an ls shows, when not all: names given
    // (ls a b, ls -d x), patterns (ls *.txt, as regexes), ls -I/--hide/-B's
    // patterns to leave out, and folders left out (they get panes of their
    // own: ls a dir).)
    property var onlyNames: []
    property var namePatterns: []
    property var ignorePatterns: []
    property bool hideDirs: false
    property bool collapseEmpty: false  // (nothing left: no room taken, not even "(empty)")
    // ls's ways of showing them (from the command; all optional):
    // inode (-i), blocks (-s), numeric (-n), noOwner (-g), noGroup (-o, -G),
    // time "mtime"/"ctime"/"atime" (-c, -u, --time), fullTime (--full-time),
    // bytes (no -h: sizes exact), classify "F"/"p"/"slash" (-F, -p), quote
    // "c"/"shell"/"escape"/"literal" (-Q, --quoting-style, -b, -N), commas (-m).
    property var lsOpts: ({})
    function picked(x) {
        if (onlyNames.length === 0 && namePatterns.length === 0) return true
        if (onlyNames.indexOf(x.name) >= 0) return true
        return namePatterns.some(p => new RegExp(p).test(x.name))
    }
    function shown(x) {
        return picked(x) && !ignorePatterns.some(p => new RegExp(p).test(x.name))
    }
    // The folders an ls names here (ls a dir, ls *): each gets a pane.
    readonly property var matchedDirs: raw.filter(x => x.dir && (onlyNames.length > 0 || namePatterns.length > 0) && shown(x)).map(x => x.path)
    // (ls -R: the folders in it, each a pane after it.)
    readonly property var subDirs: raw.filter(x => x.dir && shown(x) && (showHidden || !x.name.startsWith("."))).map(x => x.path)
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
    // (Another pane of the card changed them: these follow.)
    onKeptChanged: if (kept && JSON.stringify(kept) !== JSON.stringify(mine)) mine = Object.assign({}, kept)
    signal keptChanged2(var choices)    // (to the notebook, to keep)
    property var mine: ({})
    Component.onCompleted: {
        mine = kept ? Object.assign({}, kept) : Object.assign({}, saved, { view: undefined })
        // (ls -t, -S, -r ...: the command's order, not the last one picked.)
        if (!kept && (flagSort !== "" || flagReverse || flagFoldersFirst)) mine = Object.assign({}, mine, { sort: undefined, desc: undefined, folders_first: undefined })
        list()
    }
    readonly property string view: lsOpts.commas ? "commas" : mine.view ? mine.view : flagLong ? "details" : (saved.view || "list")
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
    // (Off: not read until it's wanted — an ls card's pane far below the
    // screen, read as it's scrolled to.)
    property bool wanted: true
    onWantedChanged: if (wanted && !listed) list()
    function list() {
        if (!wanted) return
        const n = ++listSeq
        if (!controller || dir === "") { raw = []; return }
        Async.call(controller, k => controller.listDir(k, dir, showHidden), r => {
            if (n !== listSeq) return   // (another folder asked for since)
            try { raw = JSON.parse(r) } catch (e) { raw = [] }
            listed = true
        })
    }
    onRefreshChanged: { kids = ({}); asked = ({}); sortedKids = ({}); stats = ({}); list() }   // (and the folders open in it read again)
    onDirChanged: list()
    onShowHiddenChanged: { kids = ({}); asked = ({}); sortedKids = ({}); list() }
    onControllerChanged: list()
    readonly property var entries: arrange(raw.filter(x => shown(x) && !(hideDirs && x.dir)))
    // A table's order: [{key, desc}] by priority (a header click; Shift+click
    // adds the next); none: the ls flags' order, as before.
    readonly property var sortKeys: mine.sortKeys || []
    // A header click: sorted by it, then the other way, then not (back to
    // the command's order); Shift+click does the same to it as one more key,
    // the others kept.
    function sortOn(key, add) {
        let ks = sortKeys.slice()
        const j = ks.findIndex(k => k.key === key)
        // (Newest and largest first to begin with, as ls has them.)
        const first = key === "date" || key === "size"
        const next = j < 0 ? { key: key, desc: first } : ks[j].desc === first ? { key: key, desc: !first } : null
        if (add) {
            if (j < 0) ks.push(next)
            else if (next) ks[j] = next
            else ks.splice(j, 1)
        } else {
            const only = ks.length === 1 && j === 0
            ks = next && (only || j < 0) ? [next] : j >= 0 && !only ? [{ key: key, desc: first }] : []
        }
        save("sortKeys", ks)
    }
    function arrange(a) {
        const name = (x) => x.name.toLowerCase().replace(/^\./, "")
        const byName = (x, y) => name(x).localeCompare(name(y), undefined, { numeric: true })
        const keyCmp = {
            name: byName,
            size: (x, y) => (x.dir ? x.items : x.size) - (y.dir ? y.items : y.size),
            date: (x, y) => timeOf(x) - timeOf(y),
            type: (x, y) => ext(x).localeCompare(ext(y)),
            none: (x, y) => 0,
            mode: (x, y) => String(x.mode).localeCompare(String(y.mode)),
            user: (x, y) => owner(x).localeCompare(owner(y)),
            group: (x, y) => groupOf(x).localeCompare(groupOf(y)),
            ino: (x, y) => (x.ino || 0) - (y.ino || 0),
            blocks: (x, y) => (x.blocks || 0) - (y.blocks || 0),
            nlink: (x, y) => (x.nlink || 0) - (y.nlink || 0),
        }
        let cmp
        if (sortKeys.length > 0) cmp = (x, y) => {
            for (const k of sortKeys) { const r = (keyCmp[k.key] || keyCmp.none)(x, y); if (r) return k.desc ? -r : r }
            return byName(x, y)
        }
        else {
            const one = keyCmp[sortBy] || keyCmp.none
            cmp = (x, y) => { const r = one(x, y) || (sortBy === "name" || sortBy === "none" ? 0 : byName(x, y)); return desc ? -r : r }
        }
        a.sort((x, y) => (foldersFirst && x.dir !== y.dir) ? (x.dir ? -1 : 1) : cmp(x, y))
        return a
    }
    function ext(e) { return e.dir ? "" : (e.name.indexOf(".") > 0 ? e.name.replace(/^.*\./, "").toLowerCase() : "") }

    // ---- folders opened in place (▸/▾): their entries under them, indented ----
    property var openDirs: ({})         // path -> true (open) or false (folded)
    property var kids: ({})             // path -> its listing
    property var asked: ({})            // (listings on their way)
    // ls -R: a tree with every folder open, each read as its row comes near
    // the view (nearView, from the card; viewTick when that may have changed).
    property bool expandAll: false
    function isOpen(path) { const o = openDirs[path]; return o === undefined ? expandAll : o }
    function readKids(path) {
        if (kids[path] || asked[path] || !controller) return
        asked[path] = true
        Async.call(controller, k => controller.listDir(k, path, showHidden), r => {
            let l = []
            try { l = JSON.parse(r) } catch (x) {}
            // (Several arriving together: the tree redone once for them all.)
            pendingKids[path] = l
            if (!kidsFlush.running) kidsFlush.start()
        })
    }
    property var pendingKids: ({})
    Timer {
        id: kidsFlush
        interval: 40
        onTriggered: { const m = Object.assign({}, fb.kids, fb.pendingKids); fb.pendingKids = {}; fb.kids = m }
    }
    function toggleOpen(e) {
        const o = Object.assign({}, openDirs)
        o[e.path] = !isOpen(e.path)
        openDirs = o
        if (o[e.path]) readKids(e.path)
    }
    // (Each open folder's listing sorted once, when it comes or the order
    // changes, not on every redraw of the tree.)
    property var sortedKids: ({})
    readonly property string orderKey: sortBy + desc + foldersFirst + JSON.stringify(ignorePatterns) + JSON.stringify(sortKeys)
    onOrderKeyChanged: sortedKids = ({})
    function kidsOf(path) {
        let k = sortedKids[path]
        if (!k && kids[path]) { k = arrange(kids[path].filter(x => !ignorePatterns.some(p => new RegExp(p).test(x.name)))); sortedKids[path] = k }
        return k
    }
    // What's drawn: the entries, an open folder's after it a level in.
    readonly property var rows: {
        const out = []
        const sk = sortedKids, kd = kids   // (again when they change)
        const add = (list, depth) => {
            for (const e of list) {
                out.push({ e: e, depth: depth })
                if (out.length >= 200000) return
                if (e.dir && isOpen(e.path) && kd[e.path]) add(kidsOf(e.path), depth + 1)
            }
        }
        add(entries, 0)
        return out
    }
    // ---- what's on screen: only those rows are made (a big ls -R, a
    // folder of thousands, scroll without making them all) ----
    // viewTop/viewBottom: the part of this view that shows, in its own
    // coordinates (the card sets them as it scrolls); all of it otherwise.
    property real viewTop: -1e9
    property real viewBottom: 1e9
    // The lines drawn: a fixed set of slots (160), moved over the rows as
    // it scrolls, refilled, never remade. Without a view (not in a card): all.
    readonly property bool windowed: viewTop > -1e8
    readonly property int totalLines: lined ? rows.length : view === "list" ? grid.rows : 0
    readonly property int firstLine: windowed ? Math.max(0, Math.floor((viewTop - headerH - 4) / row) - 40) : 0
    readonly property int slotLines: windowed ? 160 : Math.max(1, totalLines)
    readonly property var blank: ({ name: "", path: "", dir: false, mode: "", user: "", group: "", size: 0, mtime: 0, items: -1 })
    // (Open folders in the window, not read yet: read now — ls -R's tree
    // fills in as it's scrolled.)
    onFirstLineChanged: Qt.callLater(readShown)
    onRowsChanged: Qt.callLater(readShown)
    function readShown() {
        if (!lined) return
        const end = Math.min(rows.length, firstLine + slotLines)
        for (let i = firstLine; i < end; ++i) { const e = rows[i].e; if (e.dir && isOpen(e.path) && !kids[e.path]) readKids(e.path) }
    }
    // ---- the table (the details view): columns, each sortable ----
    // Its columns, as the user left them (kept with the card and as the
    // next card's, like the view): which show (right-click the header),
    // their order (drag a header) and widths (drag an edge; double-click it:
    // fit again). Otherwise as ls -l, with what the ls asked for.
    readonly property var colShown: mine.colShown || ({})
    readonly property var colOrder: mine.colOrder || []
    readonly property var colW: mine.colW || ({})
    readonly property var allCols: ["name", "size", "date", "mode", "nlink", "user", "group", "ino", "blocks"]
    function colTitle(k) {
        return ({ name: "Name", size: "Size", mode: "Permissions", nlink: "Links", user: "Owner", group: "Group", ino: "Inode", blocks: "Blocks",
                  date: lsOpts.time === "ctime" ? "Changed" : lsOpts.time === "atime" ? "Accessed" : "Modified" })[k] || k
    }
    function colOn(k) {
        if (k === "name") return true
        if (colShown[k] !== undefined) return colShown[k]
        const o = lsOpts
        return k === "size" || k === "date" || k === "mode" || (k === "user" && !o.noOwner) || (k === "group" && !o.noGroup) || (k === "ino" && !!o.inode) || (k === "blocks" && !!o.blocks)
    }
    function setColOn(k, on) { const m = Object.assign({}, colShown); m[k] = on; save("colShown", m) }
    function setColW(k, w) { const m = Object.assign({}, colW); if (w > 0) m[k] = w; else delete m[k]; save("colW", m) }
    // (A column dragged to before another.)
    function moveCol(k, before) {
        let o = colOrder.length ? colOrder.slice() : allCols.slice()
        for (const c of allCols) if (o.indexOf(c) < 0) o.push(c)
        o = o.filter(c => c !== k)
        const j = before === "" ? o.length : o.indexOf(before)
        o.splice(j < 0 ? o.length : j, 0, k)
        save("colOrder", o)
    }
    // (While an edge is dragged: that column's width, before it's kept.)
    property string sizingCol: ""
    property real sizingW: 0
    // (Each folder's widest cells, measured once when it comes: the table's
    // widths from those, not from every row on every change.)
    property var stats: ({})
    readonly property string statsKey: JSON.stringify(lsOpts) + JSON.stringify(colShown)
    onStatsKeyChanged: stats = ({})
    onEntriesChanged: delete stats[""]
    function statsOf(key, list) {
        let st = stats[key]
        if (st) return st
        st = { name: 0, size: 0, user: 0, group: 0, nlink: 0, ino: 0, blocks: 0 }
        const ks = ["size", "user", "group", "nlink", "ino", "blocks"].filter(k => colOn(k))
        for (const e of list) {
            const nl = plainName(e).length
            if (nl > st.name) st.name = nl
            for (const k of ks) { const v = cell(e, k).length; if (v > st[k]) st[k] = v }
        }
        stats[key] = st
        return st
    }
    // The open folders that have come, and how deep: [[path, depth]].
    readonly property var openFolders: {
        const out = [], kd = kids
        const walk = (list, depth) => {
            for (const e of list) if (e.dir && isOpen(e.path) && kd[e.path]) { out.push([e.path, depth + 1]); walk(kidsOf(e.path), depth + 1) }
        }
        if (view === "details") walk(entries, 0)
        return out
    }
    readonly property var tableCols: {
        if (view !== "details") return []
        // Each column's natural width (in characters): its widest cell.
        const nat = { name: 8, size: 4, date: lsOpts.fullTime ? 19 : 24, mode: 11, nlink: 5, user: 5, group: 5, ino: 5, blocks: 6 }
        const add = (st, depth) => {
            nat.name = Math.max(nat.name, depth * 2 + 5 + st.name)
            for (const k of ["size", "user", "group", "nlink", "ino", "blocks"]) if (st[k] > nat[k]) nat[k] = st[k]
        }
        add(statsOf("", entries), 0)
        for (const [path, depth] of openFolders) add(statsOf(path, kidsOf(path)), depth)
        let order = colOrder.filter(k => allCols.indexOf(k) >= 0)
        for (const k of allCols) if (order.indexOf(k) < 0) order.push(k)
        order = order.filter(k => colOn(k))
        const cols = order.map(k => ({ key: k, title: colTitle(k), right: ["size", "nlink", "ino", "blocks"].indexOf(k) >= 0,
                                       w: k === sizingCol ? sizingW : (colW[k] || Math.max(nat[k], colTitle(k).length + 2)) }))
        // (Name not sized by hand: as wide as the widest, or what's left.)
        const ni = cols.findIndex(c => c.key === "name")
        if (ni >= 0 && !colW.name && sizingCol !== "name") {
            let rest = 0
            for (const c of cols) if (c.key !== "name") rest += c.w + 2
            cols[ni].w = Math.max(16, Math.min(cols[ni].w, Math.floor((width - 8) / cw) - rest - 2))
        }
        let x = 0
        for (const c of cols) { c.x = x * cw; c.px = c.w * cw; x += c.w + 2 }
        return cols
    }
    readonly property var nameCol: tableCols.find(c => c.key === "name") || null
    // (A header being dragged: where it is, to drop before which column.)
    property string draggingCol: ""
    property real dragAt: 0
    function dropBefore(px) {
        for (const c of tableCols) if (c.key !== draggingCol && px < arrowW + c.x + c.px / 2) return c.key
        return ""
    }
    function cell(e, key) {
        switch (key) {
        case "size": return sizeText(e)
        case "date": return e.path === "" ? "" : stamp(timeOf(e))
        case "mode": return e.mode
        case "user": return owner(e)
        case "group": return groupOf(e)
        case "nlink": return e.nlink !== undefined ? String(e.nlink) : ""
        case "ino": return e.ino !== undefined ? String(e.ino) : ""
        case "blocks": return e.blocks !== undefined ? String(e.blocks) : ""
        }
        return ""
    }
    function sortMark(key) {
        const j = sortKeys.findIndex(k => k.key === key)
        if (j < 0) return ""
        return (sortKeys[j].desc ? " ▼" : " ▲") + (sortKeys.length > 1 ? (j + 1) : "")
    }
    // (A folder open in the columns: one a line, as a tree, from then on.)
    // (Folders open in place in the table only: the columns fill down then
    // across, so there's nowhere for a folder's entries to go.)
    readonly property bool tree: false
    readonly property bool lined: view === "details"
    readonly property real arrowW: cw * 2

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
    // A name as ls -Q / --quoting-style would print it.
    function quoted(n) {
        const q = lsOpts.quote || ""
        if (q === "c") return '"' + n.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\n/g, "\\n").replace(/\t/g, "\\t") + '"'
        if (q === "escape") return n.replace(/\\/g, "\\\\").replace(/ /g, "\\ ").replace(/\n/g, "\\n").replace(/\t/g, "\\t")
        if (q === "shell" || q === "shell-always") return q === "shell" && /^[A-Za-z0-9_.\/+,:@%=-]+$/.test(n) ? n : "'" + n.replace(/'/g, "'\\''") + "'"
        return n
    }
    // ls -F's mark after a name (dir /, program *, link @, fifo |, socket =); -p: / only.
    function mark(e) {
        const c = lsOpts.classify || "slash"
        if (e.dir) return "/"
        if (c !== "F") return ""
        return e.link ? "@" : e.special === "fifo" ? "|" : e.special === "socket" ? "=" : e.exec ? "*" : ""
    }
    // -i and -s: the inode and block count before the name.
    function prefix(e) {
        let p = ""
        if (lsOpts.inode) p += pad(e.ino !== undefined ? e.ino : "?", longW.ino, true) + " "
        if (lsOpts.blocks) p += pad(e.blocks !== undefined ? e.blocks : "?", longW.blocks, true) + " "
        return p
    }
    function plainName(e) { return quoted(e.name) + mark(e) }
    function label(e) {
        return "<span style=\"font-family:'Symbols Nerd Font Mono'\">" + icon(e) + "</span> " + esc(plainName(e))
    }
    FontMetrics { id: fm; font.family: fb.fontFamily; font.pointSize: fb.pointSize }
    readonly property real cw: fm.averageCharacterWidth > 0 ? fm.averageCharacterWidth : 8
    readonly property real row: Math.max(fm.height, rowHeight)

    // ---- ls's column layout: as many columns as fit, filled going down ----
    readonly property var grid: {
        const n = entries.length
        if (n === 0) return { cols: 1, rows: 0, widths: [0] }
        const w = new Array(n)
        let minw = 1e9, maxw = 0
        for (let i = 0; i < n; ++i) {
            const x = (plainName(entries[i]).length + prefix(entries[i]).length + 3) * cw
            w[i] = x
            if (x < minw) minw = x
            if (x > maxw) maxw = x
        }
        const avail = Math.max(cw * 10, width - 8)
        let best = { cols: 1, rows: n, widths: [maxw + 2 * cw] }
        // (As ls: the most columns that fit, trying them all — loops, not
        // spreads: a folder of 100,000 is too many arguments for Math.max.)
        const most = Math.min(n, Math.floor(avail / Math.max(1, minw + 2 * cw)))
        for (let cols = 2; cols <= most; ++cols) {
            const rows = Math.ceil(n / cols)
            if (Math.ceil(n / rows) < cols) continue
            const widths = []
            let total = 0
            for (let c = 0; c < cols && total <= avail; ++c) {
                let m = 0
                const end = Math.min(n, (c + 1) * rows)
                for (let i = c * rows; i < end; ++i) if (w[i] > m) m = w[i]
                widths.push(m + 2 * cw)
                total += m + 2 * cw
            }
            if (total > avail || widths.length < cols) continue
            best = { cols: cols, rows: rows, widths: widths }
        }
        return best
    }
    readonly property var colX: { const x = [0]; for (const w of grid.widths) x.push(x[x.length - 1] + w); return x }
    // `ls -l`'s columns, padded to their widest.
    readonly property var longW: {
        const m = (f) => { let x = 1; for (const e of entries) { const v = f(e); if (v > x) x = v } return x }
        return { user: m(e => owner(e).length), group: m(e => groupOf(e).length), size: m(e => sizeText(e).length),
                 ino: m(e => String(e.ino !== undefined ? e.ino : "?").length), blocks: m(e => String(e.blocks !== undefined ? e.blocks : "?").length),
                 nlink: m(e => String(e.nlink !== undefined ? e.nlink : 1).length) }
    }
    function sizeText(e) {
        if (lsOpts.bytes) return String(e.size)
        return e.dir ? (e.items >= 0 ? e.items + (e.items === 1 ? " item" : " items") : "") : human(e.size)
    }
    function owner(e) { return lsOpts.numeric && e.uid !== undefined ? String(e.uid) : e.user }
    function groupOf(e) { return lsOpts.numeric && e.gid !== undefined ? String(e.gid) : e.group }
    function timeOf(e) { const t = lsOpts.time === "ctime" ? e.ctime : lsOpts.time === "atime" ? e.atime : e.mtime; return t !== undefined ? t : e.mtime }
    function stamp(t) { return lsOpts.fullTime ? Qt.formatDateTime(new Date(t * 1000), "yyyy-MM-dd HH:mm:ss") : when(t) }
    // ls -l's line before the name: [inode] [blocks] mode links owner group size time.
    function longLine(e) {
        let l = prefix(e) + e.mode + " " + pad(e.nlink !== undefined ? e.nlink : 1, longW.nlink, true)
        if (!lsOpts.noOwner) l += " " + pad(owner(e), longW.user)
        if (!lsOpts.noGroup) l += " " + pad(groupOf(e), longW.group)
        return l + " " + pad(sizeText(e), longW.size, true) + " " + stamp(timeOf(e))
    }
    function pad(s, n, right) { s = String(s); while (s.length < n) s = right ? " " + s : s + " "; return s }

    readonly property real tileW: Math.max(96, cw * 14)
    readonly property int tileCols: Math.max(1, Math.floor((width - 8) / (tileW + 8)))
    // (Its toolbar, a row at the top; the path bar under it once it's been
    // gone somewhere.)
    property bool showBar: true
    readonly property real barH: showBar ? 26 : 0
    readonly property bool navigated: back.length > 0 || dir !== startDir
    readonly property real headerH: barH + (navigated ? row + 6 : 0)
    readonly property real bodyH: entries.length === 0 ? (view === "details" ? 2 * row : row)
                                : view === "commas" ? commasText.implicitHeight
                                : lined ? (rows.length + (view === "details" ? 1 : 0)) * row
                                : view === "tiles" ? Math.ceil(entries.length / tileCols) * (tileW + 24)
                                : grid.rows * row
    implicitHeight: collapseEmpty && listed && entries.length === 0 ? 0 : headerH + bodyH + 8
    visible: !(collapseEmpty && listed && entries.length === 0)

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
        visible: fb.navigated
        y: fb.barH + 2
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
            y: fb.view === "details" ? fb.row : 0   // (under the table's header)
            text: fb.listed && fb.raw.length === 0 && fb.dir !== "" ? "(empty)" : ""
            color: Theme.textDim
            font.family: fb.fontFamily
            font.pointSize: fb.pointSize
        }
        // ls -m: the names in a line, commas between, wrapped.
        Text {
            id: commasText
            visible: fb.view === "commas"
            width: parent.width
            topPadding: 0.001
            wrapMode: Text.Wrap
            textFormat: Text.StyledText
            text: fb.view !== "commas" ? "" : fb.entries.map(e => "<font color='" + fb.tint(e) + "'>" + fb.esc(fb.prefix(e) + fb.plainName(e)) + "</font>").join(", ")
            color: Theme.text
            font.family: fb.fontFamily
            font.pointSize: fb.pointSize
        }
        // ls -l as a table: its header, each a sort (Shift+click: the next one).
        Item {
            visible: fb.view === "details"
            width: parent.width
            height: fb.row
            Repeater {
                model: fb.view === "details" ? fb.tableCols : []
                delegate: Item {
                    id: hcell
                    required property var modelData
                    readonly property bool dragged: fb.draggingCol === modelData.key
                    x: fb.arrowW + modelData.x + (dragged ? fb.dragAt - hm.downAt : 0)
                    z: dragged ? 3 : 1
                    width: modelData.px
                    height: fb.row
                    Rectangle { anchors.fill: parent; visible: hcell.dragged; color: Theme.mix(Theme.bg, Theme.accent, 0.25); radius: 3 }
                    Text {
                        anchors.fill: parent
                        verticalAlignment: Text.AlignVCenter
                        horizontalAlignment: hcell.modelData.right ? Text.AlignRight : Text.AlignLeft
                        topPadding: 0.001
                        text: hcell.modelData.title + fb.sortMark(hcell.modelData.key)
                        color: hm.containsMouse ? Theme.accent : Theme.textDim
                        font.family: fb.fontFamily
                        font.pointSize: fb.pointSize
                        font.bold: true
                        elide: Text.ElideRight
                        renderType: Text.NativeRendering
                    }
                    // A click sorts by it (Shift: then by it), a drag moves it,
                    // a right click picks the columns.
                    MouseArea {
                        id: hm
                        anchors.fill: parent
                        anchors.rightMargin: 5
                        hoverEnabled: true
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        cursorShape: fb.draggingCol !== "" ? Qt.ClosedHandCursor : Qt.PointingHandCursor
                        property real downAt: 0
                        property bool moved: false
                        onPressed: (m) => { fb.entryTapAt = Date.now(); downAt = mapToItem(body, m.x, 0).x; moved = false }
                        onPositionChanged: (m) => {
                            if (!pressed || !(pressedButtons & Qt.LeftButton)) return
                            const x = mapToItem(body, m.x, 0).x
                            if (!moved && Math.abs(x - downAt) > 8) { moved = true; fb.draggingCol = hcell.modelData.key }
                            if (moved) fb.dragAt = x
                        }
                        onReleased: (m) => {
                            fb.entryTapAt = Date.now()
                            if (moved) { const k = fb.draggingCol, to = fb.dropBefore(fb.dragAt); fb.draggingCol = ""; moved = false; if (to !== k) fb.moveCol(k, to); return }
                            if (m.button === Qt.RightButton) { colMenu.popup(hcell, m.x, m.y); return }
                            fb.sortOn(hcell.modelData.key, !!(m.modifiers & Qt.ShiftModifier))
                        }
                        onCanceled: { fb.draggingCol = ""; moved = false }
                        ToolTip.visible: containsMouse && !pressed
                        ToolTip.delay: 800
                        ToolTip.text: "Sort by " + hcell.modelData.title.toLowerCase() + " (again: the other way; Shift+click: then by it). Drag to move it; right-click for the columns."
                    }
                    // Its right edge: drag to size it, double-click to fit it again.
                    MouseArea {
                        x: parent.width - 3
                        width: fb.cw * 2 - 2
                        height: parent.height
                        z: 2
                        cursorShape: Qt.SizeHorCursor
                        property real startX: 0
                        property real startW: 0
                        onPressed: (m) => { fb.entryTapAt = Date.now(); startX = mapToItem(body, m.x, 0).x; startW = hcell.modelData.w; fb.sizingW = startW; fb.sizingCol = hcell.modelData.key }
                        onPositionChanged: (m) => { if (pressed) fb.sizingW = Math.max(3, Math.round(startW + (mapToItem(body, m.x, 0).x - startX) / fb.cw)) }
                        onReleased: { const k = fb.sizingCol, w = fb.sizingW; fb.sizingCol = ""; fb.setColW(k, w) }
                        onDoubleClicked: { fb.entryTapAt = Date.now(); fb.sizingCol = ""; fb.setColW(hcell.modelData.key, 0) }
                    }
                }
            }
            // (Where a dragged header would go.)
            Rectangle {
                visible: fb.draggingCol !== ""
                readonly property var to: fb.draggingCol !== "" ? fb.tableCols.find(c => c.key === fb.dropBefore(fb.dragAt)) : null
                x: fb.arrowW + (to ? to.x : (fb.tableCols.length ? fb.tableCols[fb.tableCols.length - 1].x + fb.tableCols[fb.tableCols.length - 1].px : 0)) - 3
                width: 2
                height: fb.row
                color: Theme.accent
            }
        }
        // The rows: a fixed set of slots over the lines in view (in the
        // columns, one a column per line), refilled as it scrolls.
        Repeater {
            model: fb.view === "tiles" || fb.view === "commas" ? 0 : fb.slotLines * (fb.lined ? 1 : Math.max(1, fb.grid.cols))
            delegate: Rectangle {
                id: ent
                required property int index
                readonly property int col: fb.lined ? 0 : Math.floor(index / fb.slotLines)
                // (Each slot keeps its line till that line leaves the window:
                // a line's scroll refills one slot, not all of them.)
                readonly property int k: fb.lined ? index : index % fb.slotLines
                readonly property int line: fb.firstLine + (((k - fb.firstLine) % fb.slotLines) + fb.slotLines) % fb.slotLines
                readonly property int i: fb.lined ? line : col * Math.max(1, fb.grid.rows) + line
                readonly property bool has: fb.lined ? line < fb.rows.length : line < fb.grid.rows && i < fb.entries.length
                readonly property var e: !has ? fb.blank : fb.lined ? fb.rows[line].e : fb.entries[i]
                readonly property int depth: has && fb.lined ? fb.rows[line].depth : 0
                readonly property real indent: depth * fb.arrowW + (fb.view === "details" ? fb.arrowW : 0)
                readonly property real nameX: fb.view === "details" && fb.nameCol ? fb.nameCol.x : 0
                readonly property bool sel: has && fb.selected === e.path
                visible: has
                x: fb.lined ? 0 : (fb.colX[col] || 0)
                y: (line + (fb.view === "details" ? 1 : 0)) * fb.row
                width: fb.lined ? body.width : Math.max(0, (fb.grid.widths[col] || 0) - fb.cw)
                height: fb.row
                radius: 3
                color: sel ? Theme.mix(Theme.bg, Theme.accent, 0.3) : eh.hovered ? Theme.mix(Theme.bg, Theme.fg, 0.07) : "transparent"
                HoverHandler { id: eh; enabled: ent.has; cursorShape: Qt.PointingHandCursor }
                // ▸/▾: a folder opened in place (its entries under it).
                Text {
                    visible: ent.has && ent.e.dir && fb.view === "details"
                    x: ent.nameX + ent.indent - fb.arrowW
                    width: fb.arrowW
                    height: parent.height
                    verticalAlignment: Text.AlignVCenter
                    topPadding: 0.001
                    text: fb.isOpen(ent.e.path) ? "▾" : "▸"
                    color: arrowHover.hovered ? Theme.accent : Theme.textDim
                    font.family: fb.fontFamily
                    font.pointSize: fb.pointSize
                    renderType: Text.NativeRendering
                    z: 2
                    HoverHandler { id: arrowHover; cursorShape: Qt.PointingHandCursor }
                    // (Its own click: the folder isn't gone into as well.)
                    TapHandler { gesturePolicy: TapHandler.WithinBounds; onTapped: { fb.entryTapAt = Date.now(); fb.toggleOpen(ent.e) } }
                    ToolTip.visible: arrowHover.hovered
                    ToolTip.delay: 600
                    ToolTip.text: fb.isOpen(ent.e.path) ? "Fold it" : "Show what's in it, here"
                }
                // (Dragged out: as a file, to a file manager, an editor, the desktop.)
                Drag.dragType: Drag.Automatic
                Drag.supportedActions: Qt.CopyAction | Qt.MoveAction | Qt.LinkAction
                Drag.mimeData: ({ "text/uri-list": "file://" + encodeURI(ent.e.path) + "\r\n", "text/plain": ent.e.path })
                Drag.active: entDrag.active
                DragHandler { id: entDrag; enabled: ent.has; target: null }
                // (A folder takes what's dropped on it.)
                DropArea {
                    anchors.fill: parent
                    enabled: ent.has && ent.e.dir
                    keys: ["text/uri-list"]
                    onEntered: fb.dropOn = ent.e.path
                    onExited: if (fb.dropOn === ent.e.path) fb.dropOn = ""
                    onDropped: (drop) => { fb.dropOn = ""; fb.dropInto(ent.e.path, drop) }
                }
                border.color: ent.has && fb.dropOn === ent.e.path ? Theme.accent : "transparent"
                border.width: 1
                TapHandler {
                    enabled: ent.has
                    // (One click: a folder goes in, a file opens with its app.)
                    onTapped: { fb.entryTapAt = Date.now(); fb.selected = ent.e.path; fb.openEntry(ent.e) }
                }
                TapHandler { enabled: ent.has; acceptedButtons: Qt.RightButton; onTapped: (pt) => { fb.entryTapAt = Date.now(); fb.selected = ent.e.path; fb.menuEntry = ent.e; itemMenu.popup(ent, pt.position.x, pt.position.y) } }
                ToolTip.visible: ent.has && eh.hovered && fb.view !== "details"
                ToolTip.delay: 900
                ToolTip.text: ent.e.mode + "  " + ent.e.user + " " + ent.e.group + "  " + fb.sizeText(ent.e) + "\n" + fb.when(ent.e.mtime) + (ent.e.link ? "\n→ " + ent.e.target : "")
                // the name: icon and name (the list; the table's first column)
                Text {
                    visible: ent.has
                    x: ent.nameX + ent.indent
                    width: fb.view === "details" && fb.nameCol ? Math.max(0, fb.nameCol.px - ent.indent + fb.arrowW) : implicitWidth
                    anchors.verticalCenter: parent.verticalCenter
                    anchors.verticalCenterOffset: 0.001
                    textFormat: Text.StyledText
                    text: (fb.view !== "details" && (fb.lsOpts.inode || fb.lsOpts.blocks) ? "<font color='" + Theme.textDim + "'>" + fb.prefix(ent.e).replace(/ /g, "&nbsp;") + "</font>" : "") + fb.label(ent.e)
                          + (ent.e.link && fb.view === "details" ? "<font color='" + Theme.textDim + "'> ⇒ " + fb.esc(ent.e.target) + "</font>" : "")
                    color: fb.tint(ent.e)
                    elide: Text.ElideRight
                    font.family: fb.fontFamily
                    font.pointSize: fb.pointSize
                    font.bold: ent.e.dir || ent.e.exec
                    font.underline: eh.hovered
                }
                // the table's other columns
                Repeater {
                    model: fb.view === "details" ? fb.tableCols.filter(c => c.key !== "name") : []
                    delegate: Text {
                        required property var modelData
                        x: fb.arrowW + modelData.x
                        width: modelData.px
                        height: fb.row
                        verticalAlignment: Text.AlignVCenter
                        horizontalAlignment: modelData.right ? Text.AlignRight : Text.AlignLeft
                        topPadding: 0.001
                        text: ent.has ? fb.cell(ent.e, modelData.key) : ""
                        color: Theme.textDim
                        elide: Text.ElideRight
                        font.family: fb.fontFamily
                        font.pointSize: fb.pointSize
                        renderType: Text.NativeRendering
                    }
                }
            }
        }
        // Tiles: a thumbnail or a big icon each, the name under it.
        Repeater {
            model: fb.view === "tiles" ? fb.entries : []
            delegate: Rectangle {
                id: tile
                required property var modelData
                required property int index
                readonly property var e: modelData
                x: (index % fb.tileCols) * (fb.tileW + 8)
                y: Math.floor(index / fb.tileCols) * (fb.tileW + 24)
                width: fb.tileW
                height: fb.tileW + 20
                radius: 3
                color: fb.selected === e.path ? Theme.mix(Theme.bg, Theme.accent, 0.3) : th.hovered ? Theme.mix(Theme.bg, Theme.fg, 0.07) : "transparent"
                HoverHandler { id: th; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: { fb.entryTapAt = Date.now(); fb.selected = tile.e.path; fb.openEntry(tile.e) } }
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: (pt) => { fb.entryTapAt = Date.now(); fb.selected = tile.e.path; fb.menuEntry = tile.e; itemMenu.popup(tile, pt.position.x, pt.position.y) } }
                ToolTip.visible: th.hovered
                ToolTip.delay: 900
                ToolTip.text: tile.e.mode + "  " + tile.e.user + " " + tile.e.group + "  " + fb.sizeText(tile.e) + "\n" + fb.when(tile.e.mtime)
                // tiles: a thumbnail or a big icon, the name under it
                Item {
                    visible: fb.view === "tiles"
                    anchors.fill: parent
                    Image {
                        id: thumb
                        visible: tile.e.kind === "image" && status === Image.Ready
                        x: 6; y: 4
                        width: parent.width - 12
                        height: fb.tileW - 8
                        source: fb.view === "tiles" && tile.e.kind === "image" ? "file://" + tile.e.path : ""
                        sourceSize.width: 192
                        sourceSize.height: 192
                        asynchronous: true
                        fillMode: Image.PreserveAspectFit
                    }
                    Text {
                        visible: !thumb.visible
                        anchors.horizontalCenter: parent.horizontalCenter
                        y: (fb.tileW - height) / 2 + 0.001
                        text: fb.icon(tile.e)
                        color: fb.tint(tile.e)
                        font.family: "Symbols Nerd Font Mono"
                        font.pixelSize: fb.tileW * 0.45
                    }
                    Text {
                        y: fb.tileW + 0.001
                        width: parent.width
                        horizontalAlignment: Text.AlignHCenter
                        text: tile.e.name + (tile.e.dir ? "/" : "")
                        elide: Text.ElideMiddle
                        color: fb.tint(tile.e)
                        font.family: fb.fontFamily
                        font.pointSize: fb.pointSize - 1
                        font.bold: tile.e.dir
                        font.underline: th.hovered
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

    // ---- the toolbar, above it: view, columns, folders first, hidden ------
    // (Sorting is the table's headers.)
    Rectangle {
        id: bar
        visible: fb.showBar
        x: 0
        y: 2
        z: 5
        width: barRow.implicitWidth + 8
        height: 22
        radius: 6
        color: Theme.mix(Theme.bg, Theme.fg, 0.06)
        border.color: Theme.divider
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
            BarItem { id: colsItem; visible: fb.view === "details"; label: "Columns ▾"; tip: "Which columns show (or right-click the header)"; onClicked: colMenu.popup(colsItem, 0, colsItem.height + 2) }
            BarItem { icon: ""; label: "first"; tip: "Folders first"; on: fb.foldersFirst; onClicked: fb.save("folders_first", !fb.foldersFirst) }
            BarItem { label: ".hidden"; tip: "Show hidden files"; on: fb.showHidden; onClicked: fb.save("hidden", !fb.mine.hidden) }
        }
    }
    // The table's columns: which show; their widths and order back as they were.
    SlimMenu {
        id: colMenu
        Repeater {
            model: fb.allCols.filter(k => k !== "name")
            delegate: SlimMenuItem {
                required property var modelData
                text: fb.colTitle(modelData)
                checkable: true
                checked: fb.colOn(modelData)
                onTriggered: fb.setColOn(modelData, !fb.colOn(modelData))
            }
        }
        Sep {}
        SlimMenuItem { text: "Fit every column"; onTriggered: fb.save("colW", ({})) }
        SlimMenuItem { text: "Columns as ls -l has them"; onTriggered: { fb.save("colShown", ({})); fb.save("colOrder", []); fb.save("colW", ({})) } }
        SlimMenuItem { text: "Sort as the command did"; enabled: fb.sortKeys.length > 0; onTriggered: fb.save("sortKeys", []) }
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
