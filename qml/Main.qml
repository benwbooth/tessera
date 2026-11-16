import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtQuick.Window
import Qt.labs.platform as P
import Tessera
import Tessera.Native

ApplicationWindow {
    id: win
    width: 1100
    height: 700
    visible: true
    color: useTransparency && activeProfile.transparency > 0 ? "transparent" : Theme.bg
    // A unified title bar: the tab strip *is* the title bar. macOS keeps its
    // own traffic lights inside it; elsewhere the window has no system frame
    // and the bar draws KDE (Breeze) or Windows buttons and moves the window.
    readonly property bool macOS: Qt.platform.os === "osx"
    flags: macOS ? (Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint)
                 : (Qt.Window | Qt.FramelessWindowHint)
    title: customTitle !== "" ? customTitle : (activeView && activeView.title ? activeView.title : "Tessera")

    property var tabs: []
    property int currentTab: 0
    property var activeView: null
    property real fontSize: 0          // a zoom (Ctrl+=/-) over the profile's size; 0 = none
    property var tmuxGateways: ({})    // gateway key -> gateway TerminalView
    property var contextView: null
    property string customTitle: ""    // Window > Edit Window Title
    property string broadcastMode: "off" // "off", "tab" or "all"
    property var maximizedPane: null   // View > Maximize Active Pane
    property bool useTransparency: true
    property bool cursorGuide: false   // View > Show Cursor Guide
    property int treeRev: 0            // bumps whenever panes or tabs change
    property bool allowClose: false

    Controller { id: controller }

    // ---- preferences, profiles and themes ---------------------------------

    readonly property int rev: controller.revision
    readonly property var settings: { rev; return JSON.parse(controller.settings()) }
    readonly property var profilesByName: {
        const m = {}
        for (const p of settings.profiles) m[p.name] = p
        return m
    }
    function profileFor(name) {
        return profilesByName[name] || profilesByName[settings.default_profile] || settings.profiles[0]
    }
    readonly property bool systemDark: Qt.styleHints.colorScheme !== Qt.ColorScheme.Light
    readonly property var themeCache: { rev; systemDark; return ({}) }
    function themeNameFor(p) {
        return p.theme_mode === "system" ? (systemDark ? p.theme_dark : p.theme_light) : p.theme
    }
    function themeFor(p) {
        const n = themeNameFor(p)
        if (!themeCache[n]) themeCache[n] = JSON.parse(controller.theme(n))
        return themeCache[n]
    }
    readonly property var activePane: activeView ? paneOf(activeView) : null
    readonly property var activeProfile: profileFor(activePane ? activePane.profileName : settings.default_profile)
    Binding { target: Theme; property: "term"; value: win.themeFor(win.activeProfile) }

    function setSetting(key, value) { controller.setSetting(key, JSON.stringify(value)) }
    function setProfile(name, key, value) { return controller.setProfile(name, key, JSON.stringify(value)) }

    function optionsFor(p) {
        return JSON.stringify({
            mouse_reporting: p.mouse_reporting, alternate_scroll: p.alternate_scroll,
            allow_title: p.allow_title, bracketed_paste: p.bracketed_paste, alt_mode: p.alt_mode,
            word_chars: settings.word_chars, triple_click_wrapped: settings.triple_click_wrapped,
            clipboard_access: settings.clipboard_access, copy_trailing_newline: settings.copy_trailing_newline,
            minimum_contrast: p.minimum_contrast, triggers: p.triggers || []
        })
    }

    Component {
        id: paneComp
        TerminalPane {
            id: tp
            readonly property var prof: win.profileFor(profileName)
            themeJson: JSON.stringify(win.themeFor(prof))
            fontFamily: prof.font_family
            fontPointSize: win.fontSize > 0 ? win.fontSize : prof.font_size
            lineHeight: prof.line_height
            paddingSize: prof.padding
            cursorShape: ({ block: 0, bar: 1, underline: 2 })[prof.cursor_style] || 0
            scrollback: prof.unlimited_scrollback ? 10000000 : prof.scrollback
            copyOnSelect: win.settings.copy_on_select
            dimInactive: win.settings.dim_inactive
            dimAmount: win.settings.dim_amount
            showTitle: win.settings.pane_titles
            badge: prof.badge
            broadcasting: win.broadcastMode !== "off" && win.broadcastTargets(tp).length > 1
            onContextMenu: (v) => { win.contextView = v; contextMenu.open() }
            Binding { target: tp.view; property: "command"; value: tp.commandOverride !== "" ? tp.commandOverride : (tp.prof.command_mode === "custom" ? tp.prof.command : "") }
            Binding { target: tp.view; property: "termName"; value: tp.prof.term }
            Binding { target: tp.view; property: "startupInput"; value: tp.prof.send_text }
            Binding { target: tp.view; property: "optionsJson"; value: win.optionsFor(tp.prof) }
            Binding { target: tp.view; property: "faintOpacity"; value: tp.prof.faint_opacity }
            Binding { target: tp.view; property: "charSpacing"; value: tp.prof.char_spacing }
            Binding { target: tp.view; property: "cursorBlink"; value: tp.prof.cursor_blink }
            Binding { target: tp.view; property: "cursorGuide"; value: tp.prof.cursor_guide || win.cursorGuide }
            Binding { target: tp.view; property: "boldFont"; value: tp.prof.bold_font }
            Binding { target: tp.view; property: "italicText"; value: tp.prof.italic }
            Binding { target: tp.view; property: "transparency"; value: win.useTransparency ? tp.prof.transparency : 0 }
            Binding { target: tp.view; property: "keepBackgroundsOpaque"; value: tp.prof.keep_backgrounds_opaque }
            Binding { target: tp.view; property: "visualBell"; value: tp.prof.visual_bell }
            Binding { target: tp.view; property: "silenceBell"; value: tp.prof.silence_bell }
            Binding { target: tp.view; property: "middleClickPastes"; value: win.settings.middle_click_pastes }
            Binding { target: tp.view; property: "rightClickPastes"; value: win.settings.right_click === "paste" }
            Binding { target: tp.view; property: "focusFollowsMouse"; value: win.settings.focus_follows_mouse }
            Binding { target: tp.view; property: "broadcastTo"; value: win.broadcastTargets(tp) }
            Binding { target: tp.view; property: "showMarks"; value: tp.prof.show_marks }
            Binding { target: tp.view; property: "markColor"; value: Theme.accent }
            Binding { target: tp.view; property: "ctrlClickOpens"; value: win.settings.ctrl_click_opens }
            Binding { target: tp.view; property: "keyMappingsJson"; value: JSON.stringify(tp.prof.key_mappings || []) }
        }
    }
    Component { id: splitComp; PaneSplit {} }
    Component {
        id: tabComp
        Item {
            id: tabItem
            anchors.fill: parent
            visible: win.tabs[win.currentTab] === tabItem
            property alias root: rootSplit
            property string gateway: ""     // tmux connection, for tmux windows
            property int tmuxWindow: -1
            property string fixedTitle: ""  // tmux window name
            property string customTitle: "" // Window > Edit Tab Title
            property string launchTitle: "" // what the + menu started (a shell, ssh host)
            property string profile: ""
            property color tabColor: "transparent"
            property bool activity: false
            property bool bell: false
            property var lastView: null
            readonly property var prof: win.profileFor(profile)
            readonly property string tabTitle: customTitle !== "" ? customTitle
                : fixedTitle !== "" ? fixedTitle
                : prof.tab_title !== "" ? prof.tab_title
                : (win.settings.tab_titles_per_tab && lastView && lastView.title ? lastView.title
                   : launchTitle !== "" ? launchTitle
                   : (lastView && lastView.pwd ? lastView.pwd.split("/").filter(x => x).pop() || "/" : prof.name))
            PaneSplit {
                id: rootSplit
                anchors.fill: parent
                isRoot: true
                tab: tabItem
            }
        }
    }

    // ---- pane tree -----------------------------------------------------

    function indexIn(split, item) {
        for (let i = 0; i < split.count; ++i)
            if (split.itemAt(i) === item) return i
        return -1
    }

    function tabOf(item) {
        let node = item
        while (node && !node.isRoot) node = node.host
        return node ? node.tab : null
    }

    function makePane(host, props) {
        if (props.profileName === undefined) props.profileName = settings.default_profile
        const p = paneComp.createObject(null, props)
        p.host = host
        p.view.activeFocusChanged.connect(function() {
            if (p.view.activeFocus) {
                activeView = p.view
                const t = tabOf(p)
                if (t) t.lastView = p.view
            }
        })
        p.view.takenOver.connect(function() {
            // Dragged into another window, which now shows this session.
            p.taken = true
            Qt.callLater(closePane, p, false)
        })
        p.view.exitedChanged.connect(function() {
            if (!p.view.exited || p.view.tmuxPane !== "" || p.taken) return
            const exitProf = profileFor(p.profileName)
            if (exitProf.notify_exit && (!win.active || tabOf(p) !== tabs[currentTab]))
                controller.notify("Session ended", (tabOf(p) ? tabOf(p).tabTitle : exitProf.name))
            const action = exitProf.on_exit
            if (action === "restart") Qt.callLater(function() { p.view.restart() })
            else if (action === "keep") p.view.showMessage("\n\u001b[2m[Process completed]\u001b[0m\n")
            else Qt.callLater(closePane, p, false)
        })
        p.view.activity.connect(function() {
            const t = tabOf(p)
            if (t && t !== tabs[currentTab] && settings.show_activity) t.activity = true
        })
        p.view.bell.connect(function() {
            const t = tabOf(p)
            const prof = profileFor(p.profileName)
            const away = t !== tabs[currentTab] || !win.active
            if (t && prof.bell_icon && away) t.bell = true
            if (prof.notify_bell && away) controller.notify("Bell in \u201c" + (t ? t.tabTitle : "Tessera") + "\u201d", p.view.title || "")
        })
        p.view.copied.connect(function(text) { addPasteHistory(text) })
        p.view.keyAction.connect(function(name) { runAction(name) })
        p.view.sessionIdChanged.connect(function() {
            const prof = profileFor(p.profileName)
            if (prof.auto_log && p.view.sessionId !== "" && !p.view.logging && p.view.tmuxPane === "") {
                const stamp = new Date().toISOString().replace(/[:T]/g, "-").slice(0, 19)
                const folder = (prof.log_folder || "~/tessera-logs").replace(/\/$/, "")
                p.view.startLogging(folder + "/" + prof.name.replace(/[^A-Za-z0-9_-]/g, "_") + "-" + stamp + ".log")
            }
        })
        p.view.tmuxEvent.connect(function(ev) { onTmuxEvent(p.view, ev) })
        p.view.openFailed.connect(function(msg) { console.warn("tessera: " + msg) })
        treeRev++
        return p
    }

    function equalize(split) {
        const n = split.count
        for (let i = 0; i < n; ++i) {
            const it = split.itemAt(i)
            if (split.orientation === Qt.Horizontal) it.prefW = split.width / n
            else it.prefH = split.height / n
        }
    }

    function focusPane(p) {
        if (p && p.view) Qt.callLater(function() { p.view.forceActiveFocus() })
    }

    function paneOf(view) {
        let it = view
        while (it && it.view !== view) it = it.parent
        return it
    }

    // Directory for a new session from `source`'s profile.
    function cwdFor(prof, sourceView) {
        if (prof.initial_dir_mode === "custom" && prof.directory !== "") return prof.directory
        if (prof.initial_dir_mode === "recycle" && sourceView && sourceView.pwd) return sourceView.pwd
        return ""
    }

    function splitPane(p, orientation, profileName) {
        if (!p) return
        if (maximizedPane) toggleMaximize()
        if (p.view.tmuxPaneId >= 0) {
            p.view.tmuxCommand((orientation === Qt.Horizontal ? "split-window -h -t %" : "split-window -v -t %") + p.view.tmuxPaneId)
            return
        }
        const prof = profileFor(profileName !== undefined ? profileName : p.profileName)
        const props = { profileName: prof.name, cwd: cwdFor(prof, p.view) }
        const h = p.host
        if (h.count === 1) h.orientation = orientation
        let np
        if (h.orientation === orientation) {
            np = makePane(h, props)
            h.insertItem(indexIn(h, p) + 1, np)
            equalize(h)
        } else {
            const idx = indexIn(h, p)
            const w = p.width, ht = p.height
            h.takeItem(idx)
            const s = splitComp.createObject(null, { orientation: orientation })
            s.host = h
            s.prefW = w; s.prefH = ht
            h.insertItem(idx, s)
            p.host = s
            s.addItem(p)
            np = makePane(s, props)
            s.addItem(np)
            Qt.callLater(equalize, s)
        }
        focusPane(np)
    }

    // Remove a pane from its split; `kill` also ends its shell.
    function closePane(p, kill) {
        if (!p || !p.host) return
        if (maximizedPane) toggleMaximize()
        if (kill) {
            if (p.view.tmuxPaneId >= 0) { p.view.tmuxCommand("kill-pane -t %" + p.view.tmuxPaneId); return }
            if (!p.view.exited && settings.undo_close_seconds > 0 && p.view.sessionId !== "")
                retire({ title: "", color: "", profile: p.profileName, root: serializeNode(p, true) })
            else p.view.killSession()
        }
        const h = p.host
        const idx = indexIn(h, p)
        if (idx >= 0) h.takeItem(idx)
        p.host = null
        if (activeView === p.view) activeView = null
        p.destroy()
        treeRev++
        collapse(h)
    }

    function collapse(h) {
        if (h.count === 0) {
            if (h.isRoot) removeTab(h.tab)
            else {
                const hh = h.host
                hh.takeItem(indexIn(hh, h))
                h.destroy()
                collapse(hh)
            }
            return
        }
        if (h.count === 1 && !h.isRoot) {
            const child = h.takeItem(0)
            const hh = h.host
            const idx = indexIn(hh, h)
            hh.takeItem(idx)
            hh.insertItem(idx, child)
            child.host = hh
            h.destroy()
            focusPane(firstPane(child))
            return
        }
        focusPane(firstPane(h))
    }

    function panesIn(node, out) {
        if (!node) return out
        if (node.view !== undefined) { out.push(node); return out }
        for (let i = 0; i < node.count; ++i) panesIn(node.itemAt(i), out)
        return out
    }

    function allPanes() {
        let out = []
        for (const t of tabs) panesIn(t.root, out)
        return out
    }

    function firstPane(node) {
        const all = panesIn(node, [])
        return all.length ? all[0] : null
    }

    // Move focus to the nearest pane in a direction (dx, dy).
    function moveFocus(dx, dy) {
        const t = tabs[currentTab]
        if (!t || !activeView) return
        const cur = paneOf(activeView)
        const c = cur.mapToItem(win.contentItem, cur.width / 2, cur.height / 2)
        let best = null, bestD = 1e9
        for (const p of panesIn(t.root, [])) {
            if (p === cur || !p.visible) continue
            const q = p.mapToItem(win.contentItem, p.width / 2, p.height / 2)
            const vx = q.x - c.x, vy = q.y - c.y
            if (vx * dx + vy * dy <= 0) continue
            const d = Math.abs(vx) + Math.abs(vy) + 2 * Math.abs(dx !== 0 ? vy : vx)
            if (d < bestD) { bestD = d; best = p }
        }
        focusPane(best)
    }

    function cyclePane(step) {
        const t = tabs[currentTab]
        if (!t) return
        const all = panesIn(t.root, [])
        const i = all.indexOf(activePane)
        if (all.length > 1) focusPane(all[(i + step + all.length) % all.length])
    }

    // View > Maximize Active Pane: hide every other pane of the tab.
    function toggleMaximize() {
        if (maximizedPane) {
            const t = tabOf(maximizedPane)
            function showAll(n) { n.visible = true; if (n.view === undefined) for (let i = 0; i < n.count; ++i) showAll(n.itemAt(i)) }
            if (t) showAll(t.root)
            maximizedPane = null
            return
        }
        const p = activePane
        if (!p || !p.split) return
        const t = tabOf(p)
        function contains(n) {
            if (n === p) return true
            if (n.view !== undefined) return false
            for (let i = 0; i < n.count; ++i) if (contains(n.itemAt(i))) return true
            return false
        }
        function hideOthers(n) {
            if (n.view !== undefined) { n.visible = n === p; return }
            for (let i = 0; i < n.count; ++i) {
                const c = n.itemAt(i)
                if (contains(c)) hideOthers(c)
                else c.visible = false
            }
        }
        hideOthers(t.root)
        maximizedPane = p
        focusPane(p)
    }

    // Broadcast Input: the views a pane's typing also goes to.
    function broadcastTargets(pane) {
        treeRev
        if (broadcastMode === "off") return []
        const panes = broadcastMode === "all" ? allPanes() : panesIn(tabOf(pane) ? tabOf(pane).root : null, [])
        return panes.map(p => p.view)
    }

    // ---- tabs ------------------------------------------------------------

    function newTab(opts) {
        opts = opts || {}
        const active = activeView
        if (!opts.tmux && active && active.tmuxPaneId >= 0 && !opts.attachId && !opts.layout) {
            active.tmuxCommand("new-window")
            return null
        }
        const prof = profileFor(opts.profile !== undefined ? opts.profile : settings.default_profile)
        const t = tabComp.createObject(stack, {
            gateway: opts.gateway || "",
            tmuxWindow: opts.window !== undefined ? opts.window : -1,
            profile: prof.name
        })
        if (opts.layout) {
            buildSavedTree(t, opts.layout.root, opts.alive)
            if (opts.layout.title) t.customTitle = opts.layout.title
            if (opts.layout.color) t.tabColor = opts.layout.color
            if (t.root.count === 0) { t.destroy(); return null }
        } else if (!opts.tmux) {
            const props = { profileName: prof.name }
            if (opts.command) props.commandOverride = opts.command
            if (opts.title) t.launchTitle = opts.title
            if (opts.attachId) props.attachId = opts.attachId
            const cwd = opts.cwd !== undefined ? opts.cwd : cwdFor(prof, active)
            if (cwd) props.cwd = cwd
            const p = makePane(t.root, props)
            t.root.addItem(p)
            focusPane(p)
        }
        // tmux windows keep tmux's order, right after their gateway's tab.
        let at = tabs.length
        if (opts.tmux) {
            const gwView = tmuxGateways[opts.gateway]
            let after = -1
            for (let i = 0; i < tabs.length; ++i) {
                const o = tabs[i]
                if (o.gateway === opts.gateway && o.tmuxWindow >= 0 && o.tmuxWindow < opts.window) after = i
                else if (after < 0 && gwView && panesIn(o.root, []).some(p => p.view === gwView)) after = i
            }
            if (after >= 0) at = after + 1
        }
        const cur = tabs[currentTab]
        tabs.splice(at, 0, t)
        tabs = tabs.slice()
        currentTab = opts.background && cur ? tabs.indexOf(cur) : at
        treeRev++
        if (!opts.background) focusTab(currentTab)
        return t
    }

    function removeTab(t) {
        const i = tabs.indexOf(t)
        if (i < 0) return
        if (maximizedPane && tabOf(maximizedPane) === t) maximizedPane = null
        tabs.splice(i, 1)
        tabs = tabs.slice()
        t.destroy()
        treeRev++
        if (tabs.length === 0) { allowClose = true; win.close(); return }
        currentTab = Math.min(currentTab, tabs.length - 1)
        focusTab(currentTab)
    }

    function closeTab(t) {
        if (t.tmuxWindow >= 0) {
            const gw = tmuxGateways[t.gateway]
            if (gw) gw.tmuxCommand("kill-window -t @" + t.tmuxWindow)
            return
        }
        const panes = panesIn(t.root, [])
        const doIt = function() {
            if (settings.undo_close_seconds > 0) retire(serializeTab(t, true))
            else for (const p of panes) p.view.killSession()
            removeTab(t)
        }
        if (panes.length > 1 && settings.confirm_close_multiple)
            confirm("Close Tab", "Close this tab and its " + panes.length + " sessions?", doIt)
        else confirmJobs(panes, doIt)
    }

    function focusTab(i) {
        if (i < 0 || i >= tabs.length) return
        currentTab = i
        const t = tabs[i]
        t.activity = false
        t.bell = false
        const p = t.lastView ? paneOf(t.lastView) : firstPane(t.root)
        focusPane(p)
    }

    function moveTab(step) {
        const i = currentTab, j = i + step
        if (j < 0 || j >= tabs.length) return
        const t = tabs[i]
        tabs.splice(i, 1)
        tabs.splice(j, 0, t)
        tabs = tabs.slice()
        currentTab = j
    }

    function zoomBy(d) {
        const base = fontSize > 0 ? fontSize : (activeView ? activeView.fontPointSize : 11)
        fontSize = Math.max(6, Math.min(48, base + d))
    }

    // ---- closing, with iTerm2's confirmations ------------------------------

    // Ask before closing panes whose profile says so (always, or when a job
    // other than the shell runs).
    function confirmJobs(panes, doIt) {
        const busy = []
        let always = false
        for (const p of panes) {
            if (p.view.tmuxPane !== "" || p.view.exited) continue
            const prof = profileFor(p.profileName)
            if (prof.confirm_close === "always") { always = true; continue }
            if (prof.confirm_close !== "jobs") continue
            const job = controller.sessionJob(p.view.sessionId)
            const ignored = prof.ignored_jobs.split(",").map(s => s.trim()).filter(s => s)
            if (job !== "" && ignored.indexOf(job) < 0) busy.push(job)
        }
        if (busy.length > 0)
            confirm("Close", "“" + busy.join("”, “") + "” " + (busy.length > 1 ? "are" : "is") + " still running. Close anyway?", doIt)
        else if (always) confirm("Close", "Close this session?", doIt)
        else doIt()
    }

    function requestClosePane(p) {
        if (!p) return
        if (p.view.tmuxPaneId >= 0) { closePane(p, true); return }
        confirmJobs([p], function() { closePane(p, true) })
    }

    function closeActivePane() { if (activePane) requestClosePane(activePane) }

    // Confirmations and prompts: a sheet in the theme's colors.
    function confirm(title, text, yes, okText) {
        sheet.ask({ title: title, message: text, okText: okText || title.split(" ")[0],
                    destructive: /^(Close|Quit|Restart)/.test(title), onAccept: function() { yes() } })
    }
    function askText(title, label, initial, done) {
        sheet.ask({ title: title, message: label, input: true, value: initial, okText: "OK", onAccept: done })
    }
    Sheet { id: sheet }

    onClosing: (close) => {
        if (allowClose) { finishClose(); return }
        const panes = allPanes()
        const ending = !settings.restore_sessions
        if (settings.confirm_quit || (ending && settings.confirm_close_multiple && panes.length > 1)) {
            close.accepted = false
            confirm("Quit Tessera", ending
                    ? "Close the window and end its " + panes.length + " sessions?"
                    : "Close the window? Its sessions keep running and come back next time.",
                    function() { allowClose = true; win.close() })
            return
        }
        finishClose()
    }

    // ---- Undo Close: closed sessions stay alive a few seconds -------------

    property var closedStack: []   // [{layout, ids, timer}]
    function retire(layout) {
        const ids = Object.keys(sessionIds(layout.root, {}))
        const entry = { layout: layout, ids: ids, timer: null }
        entry.timer = later(settings.undo_close_seconds * 1000, function() {
            const i = closedStack.indexOf(entry)
            if (i >= 0) { closedStack.splice(i, 1); closedStack = closedStack.slice() }
            for (const id of ids) controller.killSession(id)
        })
        closedStack.push(entry)
        closedStack = closedStack.slice()
    }
    function undoClose() {
        const entry = closedStack.pop()
        if (!entry) return
        closedStack = closedStack.slice()
        if (entry.timer) entry.timer.stop()
        newTab({ layout: entry.layout, alive: sessionIds(entry.layout.root, {}), profile: entry.layout.profile })
    }

    // ---- Paste History and Snippets ---------------------------------------

    property var pasteHistory: []
    function addPasteHistory(text) {
        if (!text || text.trim() === "") return
        const h = pasteHistory.filter(x => x !== text)
        h.unshift(text)
        pasteHistory = h.slice(0, 20)
    }
    function unescapeText(s) {
        return s.replace(/\\(n|r|t|e|\\)/g, (m, c) => ({ n: "\n", r: "\r", t: "\t", e: "\u001b", "\\": "\\" })[c])
    }
    function shortLabel(s) {
        const first = s.split("\n").filter(l => l.trim() !== "")[0] || s
        return (first.length > 60 ? first.slice(0, 57) + "\u2026" : first) + (s.indexOf("\n") >= 0 ? "  \u21b5" : "")
    }

    // Named actions, for key mappings ("action": name).
    function runAction(name) {
        const v = activeView
        switch (name) {
        case "new_tab": newTab({}); break
        case "new_window": controller.newWindow(); break
        case "close": closeActivePane(); break
        case "split_right": splitPane(activePane, Qt.Horizontal); break
        case "split_down": splitPane(activePane, Qt.Vertical); break
        case "next_tab": focusTab((currentTab + 1) % tabs.length); break
        case "previous_tab": focusTab((currentTab + tabs.length - 1) % tabs.length); break
        case "next_pane": cyclePane(1); break
        case "previous_pane": cyclePane(-1); break
        case "maximize_pane": toggleMaximize(); break
        case "find": openFind(false); break
        case "copy": if (v) v.copy(); break
        case "paste": pasteClipboard(v); break
        case "clear_buffer": if (v) v.clearScrollback(); break
        case "previous_prompt": if (v) v.jumpToPrompt(-1); break
        case "next_prompt": if (v) v.jumpToPrompt(1); break
        case "select_last_output": if (v) v.selectLastOutput(); break
        case "undo_close": undoClose(); break
        case "fullscreen": toggleFullScreen(); break
        case "preferences": prefs.showPage(""); break
        }
    }

    function finishClose() {
        // Closed sessions waiting for Undo end now.
        for (const e of closedStack) { if (e.timer) e.timer.stop(); for (const id of e.ids) controller.killSession(id) }
        closedStack = []
        if (settings.restore_sessions && tabs.length > 0) {
            saveLayoutNow()
        } else {
            controller.saveLastLayout("")
            for (const p of allPanes()) if (p.view.tmuxPane === "") p.view.killSession()
        }
    }

    // ---- layouts: session restore and window arrangements ------------------

    function serializeNode(n, withIds) {
        if (n.view !== undefined)
            return { t: "pane", profile: n.profileName, cwd: n.view.pwd || n.view.cwd, cmd: n.commandOverride,
                     id: withIds ? n.view.sessionId : "", w: n.width, h: n.height }
        const c = []
        for (let i = 0; i < n.count; ++i) c.push(serializeNode(n.itemAt(i), withIds))
        return { t: n.orientation === Qt.Horizontal ? "h" : "v", c: c, w: n.width, h: n.height }
    }

    function serializeTab(t, withIds) {
        return { title: t.customTitle, color: t.tabColor.a > 0 ? t.tabColor.toString() : "",
                 profile: t.profile, root: serializeNode(t.root, withIds) }
    }

    function serializeWindow(withIds) {
        return { width: win.width, height: win.height, current: currentTab,
                 tabs: tabs.filter(t => t.tmuxWindow < 0).map(t => serializeTab(t, withIds)) }
    }

    // `alive`: session ids that can be reattached (null: start new shells).
    function buildSavedTree(t, node, alive) {
        function build(n, host) {
            if (n.t === "pane") {
                const props = { profileName: profileFor(n.profile).name, cwd: n.cwd || "", commandOverride: n.cmd || "" }
                if (alive) {
                    if (!n.id || !alive[n.id]) return null
                    props.attachId = n.id
                    alive[n.id] = false
                }
                const p = makePane(host, props)
                p.prefW = n.w; p.prefH = n.h
                return p
            }
            const s = splitComp.createObject(null, { orientation: n.t === "h" ? Qt.Horizontal : Qt.Vertical })
            s.host = host
            s.prefW = n.w; s.prefH = n.h
            for (const c of n.c) {
                const it = build(c, s)
                if (it) s.addItem(it)
            }
            if (s.count === 0) { s.destroy(); return null }
            if (s.count === 1) { const only = s.takeItem(0); only.host = host; s.destroy(); return only }
            return s
        }
        if (node.t === "pane") {
            const it = build(node, t.root)
            if (it) t.root.addItem(it)
        } else {
            t.root.orientation = node.t === "h" ? Qt.Horizontal : Qt.Vertical
            for (const c of node.c) {
                const it = build(c, t.root)
                if (it) { it.host = t.root; t.root.addItem(it) }
            }
        }
    }

    function restoreArrangement(layout, alive) {
        for (const tl of layout.tabs) newTab({ layout: tl, alive: alive, profile: tl.profile, background: true })
    }

    function saveLayoutNow() {
        if (settings.restore_sessions && tabs.length > 0)
            controller.saveLastLayout(JSON.stringify(serializeWindow(true)))
    }
    // Saved as things change too: a rebuild (devloop) or crash never runs onClosing.
    Timer { id: layoutSaver; interval: 1500; onTriggered: win.saveLayoutNow() }
    onTreeRevChanged: if (startupDone) layoutSaver.restart()
    onCurrentTabChanged: if (startupDone) layoutSaver.restart()
    property bool startupDone: false

    function saveContentsTo(text, title) {
        saveDialog.pendingText = text
        saveDialog.title = title
        saveDialog.open()
    }
    FileDialog {
        id: saveDialog
        property string pendingText: ""
        fileMode: FileDialog.SaveFile
        nameFilters: ["Text files (*.txt)", "All files (*)"]
        onAccepted: controller.writeFile(selectedFile.toString(), pendingText)
    }
    FileDialog {
        id: logDialog
        title: "Log Session Output to File"
        fileMode: FileDialog.SaveFile
        nameFilters: ["Log files (*.log)", "All files (*)"]
        onAccepted: if (activeView) activeView.startLogging(selectedFile.toString())
    }

    // ---- paste, with the multi-line warning ---------------------------------

    function pasteInto(view, text) {
        if (!view || text === "") return
        addPasteHistory(text)
        const lines = text.replace(/\n$/, "").split("\n").length
        if (settings.warn_multiline_paste && lines > 1)
            confirm("Paste", "Paste " + lines + " lines?", function() { view.pasteText(text) })
        else view.pasteText(text)
    }
    function pasteClipboard(view) { clipHelper.text = ""; clipHelper.paste(); pasteInto(view, clipHelper.text) }
    // Reads the clipboard for pasteInto (QML has no direct clipboard API).
    TextEdit { id: clipHelper; visible: false }

    // ---- tab dragging: reorder, tear off, move between windows ---------------

    // Run fn a moment later, outside the drag-and-drop loop (the dragged tab's
    // button must not be destroyed while the drag is still finishing).
    function later(ms, fn) {
        const t = Qt.createQmlObject('import QtQuick; Timer {}', win)
        t.interval = ms
        t.triggered.connect(function() { fn(); t.destroy() })
        t.start()
        return t
    }

    function tabPayload(t) {
        return t.tmuxWindow >= 0 ? "" : JSON.stringify(serializeTab(t, true))
    }

    function onTabDragFinished(t, action) {
        if (action === Qt.MoveAction) {
            // Another window took it (and reattaches its sessions).
            later(30, function() { removeTab(t) })
        } else if (action === Qt.IgnoreAction && tabs.length > 1 && t.tmuxWindow < 0) {
            // Dropped away from any tab bar: it becomes a window of its own.
            const json = tabPayload(t)
            later(30, function() {
                removeTab(t)
                controller.newWindowWithTab(json)
            })
        }
    }

    function sessionIds(node, out) {
        if (node.t === "pane") { if (node.id) out[node.id] = true }
        else for (const c of node.c) sessionIds(c, out)
        return out
    }

    // A tab dropped in from another window: take over its sessions.
    function receiveTab(json, index) {
        let tl = null
        try { tl = JSON.parse(json) } catch (e) { return }
        if (!tl || !tl.root) return
        later(300, function() {
            const t = newTab({ layout: tl, alive: sessionIds(tl.root, {}), profile: tl.profile })
            if (t && index >= 0 && index < tabs.length - 1) moveTabTo(t, index)
        })
    }

    function moveTabToIndex(t, j) {
        const i = tabs.indexOf(t)
        if (i < 0 || j === i) return
        const cur = tabs[currentTab]
        tabs.splice(i, 1)
        tabs.splice(Math.max(0, Math.min(tabs.length, j)), 0, t)
        tabs = tabs.slice()
        currentTab = tabs.indexOf(cur)
        treeRev++
    }

    function moveTabTo(t, index) {
        const i = tabs.indexOf(t)
        if (i < 0) return
        let j = Math.max(0, Math.min(tabs.length - 1, index > i ? index - 1 : index))
        if (j === i) return
        const cur = tabs[currentTab]
        tabs.splice(i, 1)
        tabs.splice(j, 0, t)
        tabs = tabs.slice()
        currentTab = tabs.indexOf(cur)
        treeRev++
    }

    // ---- find ------------------------------------------------------------

    function openFind(urls) {
        const p = activePane
        if (!p) return
        if (urls) p.findBar.findUrls()
        else p.findBar.open()
    }
    function findStep(dir) {
        const p = activePane
        if (p && p.findBar.visible) p.findBar.step(dir)
        else openFind(false)
    }

    function toggleMaximized() {
        win.visibility === Window.Maximized ? win.showNormal() : win.showMaximized()
    }

    // ---- tmux control mode -----------------------------------------------

    function tmuxTab(gateway, window) {
        for (const t of tabs) if (t.gateway === gateway && t.tmuxWindow === window) return t
        return null
    }

    function activeGateway() {
        const t = tabs[currentTab]
        if (t && t.gateway !== "") return tmuxGateways[t.gateway]
        if (activeView && activeView.tmuxGateway !== "") return tmuxGateways[activeView.tmuxGateway]
        return null
    }

    function onTmuxEvent(view, ev) {
        switch (ev.type) {
        case "tmuxStart":
            tmuxGateways[ev.gateway] = view
            tmuxGateways = Object.assign({}, tmuxGateways)
            break
        case "window": {
            let t = tmuxTab(ev.gateway, ev.window)
            if (!t) t = newTab({ tmux: true, gateway: ev.gateway, window: ev.window, background: !ev.active })
            if (ev.name !== undefined) t.fixedTitle = ev.name
            buildTmuxLayout(t, ev.layout, view)
            if (ev.active) focusTab(tabs.indexOf(t))
            break
        }
        case "windowRename": {
            const t = tmuxTab(ev.gateway, ev.window)
            if (t) t.fixedTitle = ev.name
            break
        }
        case "windowClose": {
            const t = tmuxTab(ev.gateway, ev.window)
            if (t) removeTab(t)
            break
        }
        case "tmuxExit":
            for (const t of tabs.slice()) if (t.gateway === ev.gateway && t.tmuxWindow >= 0) removeTab(t)
            delete tmuxGateways[ev.gateway]
            tmuxGateways = Object.assign({}, tmuxGateways)
            break
        case "notify":
            controller.notify(ev.title, ev.body)
            break
        case "tmuxError":
            console.warn("tmux: " + ev.message)
            break
        }
    }

    // Rebuild a tmux window's splits from tmux's layout tree, keeping the
    // existing pane views (and their terminals) by pane id.
    function buildTmuxLayout(t, layout, gatewayView) {
        const existing = {}
        for (const p of panesIn(t.root, [])) existing[p.view.tmuxPaneId] = p
        const oldSplits = []
        function strip(split) {
            while (split.count > 0) {
                const it = split.takeItem(0)
                if (it.view === undefined) { strip(it); oldSplits.push(it) }
                else it.parent = holder
            }
        }
        strip(t.root)
        for (const s of oldSplits) s.destroy()

        const cw = gatewayView.cellWidth, ch = gatewayView.cellHeight, pad = 2 * gatewayView.padding
        const prof = profileFor(paneOf(gatewayView) ? paneOf(gatewayView).profileName : t.profile)
        function build(node, host) {
            if (node.t === "pane") {
                let p = existing[node.id]
                if (p) delete existing[node.id]
                else p = makePane(host, { tmuxPane: t.gateway + ":%" + node.id, profileName: prof.name })
                p.host = host
                p.prefW = node.w * cw + pad
                p.prefH = node.h * ch + pad
                return p
            }
            const s = splitComp.createObject(null, { orientation: node.t === "h" ? Qt.Horizontal : Qt.Vertical })
            s.host = host
            s.prefW = node.w * cw + pad
            s.prefH = node.h * ch + pad
            for (const c of node.c) s.addItem(build(c, s))
            return s
        }
        if (layout.t === "pane") {
            t.root.addItem(build(layout, t.root))
        } else {
            t.root.orientation = layout.t === "h" ? Qt.Horizontal : Qt.Vertical
            for (const c of layout.c) t.root.addItem(build(c, t.root))
        }
        for (const id in existing) existing[id].destroy()
        if (!t.lastView || panesIn(t.root, []).indexOf(paneOf(t.lastView)) < 0) {
            const f = firstPane(t.root)
            t.lastView = f ? f.view : null
        }
        treeRev++
    }

    Item { id: holder; visible: false }

    // ---- shortcuts (iTerm2's, with Ctrl+Shift where it has Cmd) -------------
    // The menus show these keys in their text; the keys themselves live here
    // (one place, and closing doesn't auto-repeat when the key is held).

    Shortcut { sequence: "Ctrl+Shift+T"; onActivated: newTab({}) }
    Shortcut { sequence: "Ctrl+Shift+N"; autoRepeat: false; onActivated: controller.newWindow() }
    Shortcut { sequence: "Ctrl+Shift+W"; autoRepeat: false; onActivated: closeActivePane() }
    Shortcut { sequence: "Ctrl+Shift+D"; autoRepeat: false; onActivated: splitPane(activePane, Qt.Horizontal) }
    Shortcut { sequence: "Ctrl+Shift+E"; autoRepeat: false; onActivated: splitPane(activePane, Qt.Vertical) }
    Shortcut { sequence: "Ctrl+Shift+C"; onActivated: if (activeView) activeView.copy() }
    Shortcut { sequence: "Ctrl+Shift+V"; onActivated: pasteClipboard(activeView) }
    Shortcut { sequence: "Shift+Insert"; onActivated: if (activeView) activeView.pasteSelection() }
    Shortcut { sequence: "Ctrl+Shift+A"; onActivated: if (activeView) activeView.selectAll() }
    Shortcut { sequence: "Ctrl+Shift+K"; autoRepeat: false; onActivated: if (activeView) activeView.clearScrollback() }
    Shortcut { sequence: "Ctrl+PgDown"; onActivated: focusTab((currentTab + 1) % tabs.length) }
    Shortcut { sequence: "Ctrl+PgUp"; onActivated: focusTab((currentTab + tabs.length - 1) % tabs.length) }
    Shortcut { sequence: "Ctrl+Tab"; onActivated: focusTab((currentTab + 1) % tabs.length) }
    Shortcut { sequence: "Ctrl+Shift+Tab"; onActivated: focusTab((currentTab + tabs.length - 1) % tabs.length) }
    Shortcut { sequence: "Ctrl+Shift+PgDown"; onActivated: moveTab(1) }
    Shortcut { sequence: "Ctrl+Shift+PgUp"; onActivated: moveTab(-1) }
    Shortcut { sequence: "Ctrl+Alt+Left"; onActivated: moveFocus(-1, 0) }
    Shortcut { sequence: "Ctrl+Alt+Right"; onActivated: moveFocus(1, 0) }
    Shortcut { sequence: "Ctrl+Alt+Up"; onActivated: moveFocus(0, -1) }
    Shortcut { sequence: "Ctrl+Alt+Down"; onActivated: moveFocus(0, 1) }
    Shortcut { sequence: "Ctrl+Shift+]"; onActivated: cyclePane(1) }
    Shortcut { sequence: "Ctrl+Shift+["; onActivated: cyclePane(-1) }
    Shortcut { sequence: "Ctrl+Shift+Return"; autoRepeat: false; onActivated: toggleMaximize() }
    Shortcut { sequence: "F11"; autoRepeat: false; onActivated: toggleFullScreen() }
    Shortcut { sequence: "Ctrl+Shift+/"; onActivated: if (activeView) activeView.findCursor() }
    Shortcut { sequences: ["Ctrl+=", "Ctrl++"]; onActivated: zoomBy(1) }
    Shortcut { sequence: "Ctrl+-"; onActivated: zoomBy(-1) }
    Shortcut { sequence: "Ctrl+0"; onActivated: fontSize = 0 }
    Shortcut { sequence: "Ctrl+,"; onActivated: prefs.showPage("") }
    Shortcut { sequence: "Ctrl+Shift+Q"; autoRepeat: false; onActivated: win.close() }
    Shortcut { sequence: "Ctrl+Shift+F"; onActivated: openFind(false) }
    Shortcut { sequence: "Ctrl+Shift+Up"; onActivated: if (activeView) activeView.jumpToPrompt(-1) }
    Shortcut { sequence: "Ctrl+Shift+Down"; onActivated: if (activeView) activeView.jumpToPrompt(1) }
    Shortcut { sequence: "Ctrl+Shift+L"; onActivated: if (activeView) activeView.selectLastOutput() }
    Shortcut { sequence: "Ctrl+Shift+Z"; autoRepeat: false; onActivated: undoClose() }
    Shortcut { sequence: "F3"; onActivated: findStep(-1) }
    Shortcut { sequence: "Shift+F3"; onActivated: findStep(1) }
    Repeater {
        model: 9
        delegate: Item {
            Shortcut { sequence: "Alt+" + (index + 1); onActivated: focusTab(index) }
        }
    }

    function toggleFullScreen() {
        win.visibility = win.visibility === Window.FullScreen ? Window.Windowed : Window.FullScreen
    }

    // ---- menus: iTerm2's menu bar, as the desktop's own menus ----------------

    readonly property var profileList: settings.profiles
    readonly property var arrangementMap: { rev; return JSON.parse(controller.arrangements()) }
    readonly property var arrangementNames: Object.keys(arrangementMap)

    P.Menu {
        id: appMenu

        P.Menu {
            title: "Shell"
            icon.name: "utilities-terminal"
            P.MenuItem { icon.name: "tab-new"; text: "New Tab\tCtrl+Shift+T"; onTriggered: newTab({}) }
            P.MenuItem { icon.name: "tab-new"; text: "New Tab with Current Profile"; onTriggered: newTab({ profile: activeProfile.name }) }
            P.MenuItem { icon.name: "window-new"; text: "New Window\tCtrl+Shift+N"; onTriggered: controller.newWindow() }
            P.MenuItem { icon.name: "tab-duplicate"; text: "Duplicate Tab"; enabled: tabs.length > 0 && tabs[currentTab].tmuxWindow < 0
                onTriggered: newTab({ layout: serializeTab(tabs[currentTab], false), profile: tabs[currentTab].profile }) }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "view-split-left-right"; text: "Split Right with Current Profile\tCtrl+Shift+D"; onTriggered: splitPane(activePane, Qt.Horizontal) }
            P.MenuItem { icon.name: "view-split-top-bottom"; text: "Split Down with Current Profile\tCtrl+Shift+E"; onTriggered: splitPane(activePane, Qt.Vertical) }
            P.MenuItem { icon.name: "view-split-left-right"; text: "Split Right"; onTriggered: splitPane(activePane, Qt.Horizontal, settings.default_profile) }
            P.MenuItem { icon.name: "view-split-top-bottom"; text: "Split Down"; onTriggered: splitPane(activePane, Qt.Vertical, settings.default_profile) }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "document-save"; text: "Save Contents…"; enabled: activeView !== null
                onTriggered: saveContentsTo(activeView.contentsText(), "Save Contents") }
            P.MenuItem { icon.name: "document-save-as"; text: "Save Selected Text…"; enabled: activeView !== null && activeView.hasSelection
                onTriggered: saveContentsTo(activeView.selectedText(), "Save Selected Text") }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "edit-undo"; text: "Undo Close\tCtrl+Shift+Z"; enabled: closedStack.length > 0; onTriggered: undoClose() }
            P.MenuItem { icon.name: "view-close"; text: "Close\tCtrl+Shift+W"; onTriggered: closeActivePane() }
            P.MenuItem { icon.name: "tab-close"; text: "Close All Panes in Tab"; enabled: tabs.length > 0; onTriggered: closeTab(tabs[currentTab]) }
            P.MenuItem { icon.name: "window-close"; text: "Close Window\tCtrl+Shift+Q"; onTriggered: win.close() }
            P.MenuSeparator {}
            P.Menu {
                title: "Broadcast Input"
                icon.name: "network-transmit"
                P.MenuItemGroup { id: broadcastGroup }
                P.MenuItem { text: "Send Input to Current Session Only"; checkable: true; group: broadcastGroup; checked: broadcastMode === "off"; onTriggered: broadcastMode = "off" }
                P.MenuItem { text: "Broadcast Input to All Panes in All Tabs"; checkable: true; group: broadcastGroup; checked: broadcastMode === "all"; onTriggered: broadcastMode = "all" }
                P.MenuItem { text: "Broadcast Input to All Panes in Current Tab"; checkable: true; group: broadcastGroup; checked: broadcastMode === "tab"; onTriggered: broadcastMode = "tab" }
            }
            P.Menu {
                title: "tmux"
                icon.name: "view-split-left-right"
                enabled: Object.keys(tmuxGateways).length > 0
                P.MenuItem { icon.name: "network-disconnect"; text: "Detach"; onTriggered: { const g = activeGateway(); if (g) g.tmuxCommand("detach-client") } }
                P.MenuItem { icon.name: "tab-new"; text: "New tmux Window"; onTriggered: { const g = activeGateway(); if (g) g.tmuxCommand("new-window") } }
            }
        }

        P.Menu {
            title: "Edit"
            icon.name: "edit-entry"
            P.MenuItem { icon.name: "edit-copy"; text: "Copy\tCtrl+Shift+C"; enabled: activeView !== null && activeView.hasSelection; onTriggered: activeView.copy() }
            P.MenuItem { icon.name: "edit-copy"; text: "Copy with Styles"; enabled: activeView !== null && activeView.hasSelection; onTriggered: activeView.copyWithStyles() }
            P.MenuItem { icon.name: "edit-copy"; text: "Copy with Control Sequences"; enabled: activeView !== null && activeView.hasSelection; onTriggered: activeView.copyWithControlSequences() }
            P.MenuItem { icon.name: "edit-paste"; text: "Paste\tCtrl+Shift+V"; onTriggered: pasteClipboard(activeView) }
            P.Menu {
                title: "Paste Special"
                icon.name: "edit-paste"
                P.MenuItem { text: "Paste Selection\tShift+Insert"; onTriggered: if (activeView) activeView.pasteSelection() }
                P.MenuItem { text: "Paste Slowly"; onTriggered: if (activeView) activeView.pasteSlowly() }
                P.MenuSeparator {}
                P.MenuItem { text: "Warn Before Multi-Line Paste"; checkable: true; checked: settings.warn_multiline_paste
                    onTriggered: setSetting("warn_multiline_paste", !settings.warn_multiline_paste) }
            }
            P.Menu {
                id: pasteHistoryMenu
                title: "Paste History"
                icon.name: "view-history"
                enabled: pasteHistory.length > 0
                Instantiator {
                    model: win.pasteHistory
                    delegate: P.MenuItem { text: win.shortLabel(modelData); onTriggered: pasteInto(activeView, modelData) }
                    onObjectAdded: (i, o) => pasteHistoryMenu.insertItem(i, o)
                    onObjectRemoved: (i, o) => pasteHistoryMenu.removeItem(o)
                }
            }
            P.Menu {
                id: snippetsMenu
                title: "Snippets"
                icon.name: "insert-text"
                Instantiator {
                    model: win.settings.snippets
                    delegate: P.MenuItem { text: modelData.title; onTriggered: if (activeView) activeView.sendText(win.unescapeText(modelData.text)) }
                    onObjectAdded: (i, o) => snippetsMenu.insertItem(i, o)
                    onObjectRemoved: (i, o) => snippetsMenu.removeItem(o)
                }
                P.MenuSeparator {}
                P.MenuItem { icon.name: "configure"; text: "Edit Snippets\u2026"; onTriggered: prefs.showPage("snippets") }
            }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "edit-select-all"; text: "Select All\tCtrl+Shift+A"; onTriggered: if (activeView) activeView.selectAll() }
            P.Menu {
                title: "Marks"
                icon.name: "bookmarks"
                P.MenuItem { icon.name: "go-up"; text: "Previous Prompt\tCtrl+Shift+Up"; onTriggered: if (activeView) activeView.jumpToPrompt(-1) }
                P.MenuItem { icon.name: "go-down"; text: "Next Prompt\tCtrl+Shift+Down"; onTriggered: if (activeView) activeView.jumpToPrompt(1) }
                P.MenuSeparator {}
                P.MenuItem { icon.name: "edit-select"; text: "Select Output of Last Command\tCtrl+Shift+L"; onTriggered: if (activeView) activeView.selectLastOutput() }
                P.MenuSeparator {}
                P.MenuItem { enabled: false; text: "These need shell integration (\u2630 \u203a Install Shell Integration)" }
            }
            P.MenuSeparator {}
            P.Menu {
                title: "Find"
                icon.name: "edit-find"
                P.MenuItem { icon.name: "edit-find"; text: "Find\u2026\tCtrl+Shift+F"; onTriggered: openFind(false) }
                P.MenuItem { icon.name: "go-up"; text: "Find Next\tF3"; onTriggered: findStep(-1) }
                P.MenuItem { icon.name: "go-down"; text: "Find Previous\tShift+F3"; onTriggered: findStep(1) }
                P.MenuSeparator {}
                P.MenuItem { icon.name: "link"; text: "Find URLs"; onTriggered: openFind(true) }
            }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "edit-clear-history"; text: "Clear Buffer\tCtrl+Shift+K"; onTriggered: if (activeView) activeView.clearScrollback() }
            P.MenuItem { icon.name: "edit-clear-history"; text: "Clear Scrollback Buffer"; onTriggered: if (activeView) activeView.showMessage("\u001b[3J") }
        }

        P.Menu {
            title: "View"
            icon.name: "view-visible"
            P.MenuItem { icon.name: "view-fullscreen"; text: "Toggle Full Screen\tF11"; onTriggered: toggleFullScreen() }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "zoom-in"; text: "Make Text Bigger\tCtrl+="; onTriggered: zoomBy(1) }
            P.MenuItem { icon.name: "zoom-out"; text: "Make Text Smaller\tCtrl+-"; onTriggered: zoomBy(-1) }
            P.MenuItem { icon.name: "zoom-original"; text: "Make Text Normal Size\tCtrl+0"; enabled: fontSize > 0; onTriggered: fontSize = 0 }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "zoom-fit-best"; text: "Maximize Active Pane\tCtrl+Shift+Enter"; checkable: true; checked: maximizedPane !== null
                enabled: maximizedPane !== null || (activePane !== null && activePane.split); onTriggered: toggleMaximize() }
            P.MenuItem { text: "Use Transparency"; checkable: true; checked: useTransparency; enabled: activeProfile.transparency > 0
                onTriggered: useTransparency = !useTransparency }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "edit-find"; text: "Find Cursor\tCtrl+Shift+/"; onTriggered: if (activeView) activeView.findCursor() }
            P.MenuItem { text: "Show Cursor Guide"; checkable: true; checked: cursorGuide; onTriggered: cursorGuide = !cursorGuide }
            P.MenuSeparator {}
            P.Menu {
                id: tabColorMenu
                title: "Tab Color"
                icon.name: "color-management"
                P.MenuItem { text: "No Color"; onTriggered: if (tabs[currentTab]) tabs[currentTab].tabColor = "transparent" }
                P.MenuSeparator {}
                Instantiator {
                    model: [["Red", "#e5534b"], ["Orange", "#e8853d"], ["Yellow", "#d9b43b"], ["Green", "#57ab5a"], ["Blue", "#4a90e2"], ["Purple", "#986ee2"], ["Gray", "#8b949e"]]
                    delegate: P.MenuItem {
                        text: modelData[0]
                        onTriggered: if (tabs[currentTab]) tabs[currentTab].tabColor = modelData[1]
                    }
                    onObjectAdded: (i, o) => tabColorMenu.insertItem(i + 2, o)
                    onObjectRemoved: (i, o) => tabColorMenu.removeItem(o)
                }
            }
        }

        P.Menu {
            title: "Session"
            icon.name: "system-run"
            P.MenuItem { icon.name: "document-edit"; text: "Edit Session…"; onTriggered: prefs.showProfile(activeProfile.name) }
            P.MenuItem { icon.name: "view-refresh"; text: "Restart Session"; enabled: activeView !== null && activeView.tmuxPane === ""
                onTriggered: { const v = activeView; confirmJobs([paneOf(v)], function() { v.restart() }) } }
            P.MenuItem { icon.name: "tab-duplicate"; text: "Duplicate Session"; enabled: activeView !== null
                onTriggered: newTab({ profile: activeProfile.name, cwd: activeView.pwd }) }
            P.MenuItem { icon.name: "edit-clear"; text: "Reset"; onTriggered: if (activeView) activeView.resetTerminal() }
            P.MenuSeparator {}
            P.Menu {
                title: "Log"
                icon.name: "text-x-log"
                P.MenuItem { icon.name: "media-record"; text: "Log to File…"; enabled: activeView !== null && !activeView.logging; onTriggered: logDialog.open() }
                P.MenuItem { icon.name: "media-playback-stop"; text: "Stop Logging"; enabled: activeView !== null && activeView.logging; onTriggered: activeView.stopLogging() }
            }
        }

        P.Menu {
            id: profilesMenu
            title: "Profiles"
            icon.name: "bookmarks"
            Instantiator {
                model: win.profileList
                delegate: P.MenuItem {
                    icon.name: "utilities-terminal"
                    text: modelData.name + (modelData.name === win.settings.default_profile ? "  (default)" : "")
                    onTriggered: newTab({ profile: modelData.name })
                }
                onObjectAdded: (i, o) => profilesMenu.insertItem(i, o)
                onObjectRemoved: (i, o) => profilesMenu.removeItem(o)
            }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "tab-new"; text: "Open All"; onTriggered: { for (const p of profileList) newTab({ profile: p.name }) } }
            P.MenuItem { icon.name: "configure"; text: "Edit Profiles…"; onTriggered: prefs.showProfile(activeProfile.name) }
        }

        P.Menu {
            title: "Window"
            icon.name: "window"
            P.MenuItem { icon.name: "window-minimize"; text: "Minimize"; onTriggered: win.showMinimized() }
            P.MenuItem { icon.name: "window-maximize"; text: "Zoom"; onTriggered: win.visibility === Window.Maximized ? win.showNormal() : win.showMaximized() }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "edit-rename"; text: "Edit Window Title…"; onTriggered: askText("Window Title", "Title (empty: the program's title)", customTitle, (t) => customTitle = t) }
            P.MenuItem { icon.name: "edit-rename"; text: "Edit Tab Title…"; enabled: tabs.length > 0
                onTriggered: { const t = tabs[currentTab]; askText("Tab Title", "Title (empty: automatic)", t.customTitle, (s) => { t.customTitle = s; treeRev++ }) } }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "go-next"; text: "Select Next Tab\tCtrl+PgDown"; enabled: tabs.length > 1; onTriggered: focusTab((currentTab + 1) % tabs.length) }
            P.MenuItem { icon.name: "go-previous"; text: "Select Previous Tab\tCtrl+PgUp"; enabled: tabs.length > 1; onTriggered: focusTab((currentTab + tabs.length - 1) % tabs.length) }
            P.MenuItem { icon.name: "arrow-left"; text: "Move Tab Left\tCtrl+Shift+PgUp"; enabled: currentTab > 0; onTriggered: moveTab(-1) }
            P.MenuItem { icon.name: "arrow-right"; text: "Move Tab Right\tCtrl+Shift+PgDown"; enabled: currentTab < tabs.length - 1; onTriggered: moveTab(1) }
            P.Menu {
                title: "Select Split Pane"
                icon.name: "view-split-left-right"
                P.MenuItem { icon.name: "go-up"; text: "Select Pane Above\tCtrl+Alt+Up"; onTriggered: moveFocus(0, -1) }
                P.MenuItem { icon.name: "go-down"; text: "Select Pane Below\tCtrl+Alt+Down"; onTriggered: moveFocus(0, 1) }
                P.MenuItem { icon.name: "go-previous"; text: "Select Pane Left\tCtrl+Alt+Left"; onTriggered: moveFocus(-1, 0) }
                P.MenuItem { icon.name: "go-next"; text: "Select Pane Right\tCtrl+Alt+Right"; onTriggered: moveFocus(1, 0) }
                P.MenuSeparator {}
                P.MenuItem { text: "Next Pane\tCtrl+Shift+]"; onTriggered: cyclePane(1) }
                P.MenuItem { text: "Previous Pane\tCtrl+Shift+["; onTriggered: cyclePane(-1) }
            }
            P.MenuSeparator {}
            P.Menu {
                id: arrangementsMenu
                title: "Arrangements"
                icon.name: "view-group"
                P.MenuItem { icon.name: "document-save"; text: "Save Window Arrangement…"
                    onTriggered: askText("Save Window Arrangement", "Name", "", (n) => { if (n.trim() !== "") controller.saveArrangement(n.trim(), JSON.stringify(serializeWindow(false))) }) }
                P.MenuSeparator {}
                Instantiator {
                    model: win.arrangementNames
                    delegate: P.MenuItem {
                        icon.name: "document-open"
                        text: "Restore “" + modelData + "”"
                        onTriggered: restoreArrangement(win.arrangementMap[modelData], null)
                    }
                    onObjectAdded: (i, o) => arrangementsMenu.insertItem(i + 2, o)
                    onObjectRemoved: (i, o) => arrangementsMenu.removeItem(o)
                }
                P.MenuSeparator {}
                P.MenuItem { icon.name: "configure"; text: "Manage Arrangements…"; onTriggered: prefs.showPage("arrangements") }
            }
        }

        P.Menu {
            title: "Help"
            icon.name: "help-contents"
            P.MenuItem { icon.name: "help-contents"; text: "Tessera Help"; onTriggered: Qt.openUrlExternally("https://github.com/benwbooth/tessera#readme") }
            P.MenuItem { icon.name: "help-about"; text: "About Tessera"; onTriggered: prefs.showPage("about") }
        }

        P.MenuSeparator {}
        P.Menu {
            id: shellIntegrationMenu
            title: "Install Shell Integration"
            icon.name: "run-install"
            Instantiator {
                model: ["bash", "zsh", "fish"]
                delegate: P.MenuItem {
                    text: modelData
                    onTriggered: sheet.ask({ title: "Shell Integration", message: controller.installShellIntegration(modelData), infoOnly: true, okText: "OK" })
                }
                onObjectAdded: (i, o) => shellIntegrationMenu.insertItem(i, o)
                onObjectRemoved: (i, o) => shellIntegrationMenu.removeItem(o)
            }
        }
        P.MenuItem { icon.name: "configure"; text: "Preferences…\tCtrl+,"; onTriggered: prefs.showPage("") }
        P.MenuItem { icon.name: "application-exit"; text: "Quit Tessera\tCtrl+Shift+Q"; onTriggered: win.close() }
    }

    // The + button's menu: profiles, the shells installed, ssh hosts.
    property var launchData: ({ shells: [], ssh_hosts: [] })
    P.Menu {
        id: launchMenu
        onAboutToShow: win.launchData = JSON.parse(controller.launchers())
        P.MenuItem { enabled: false; text: "Profiles" }
        Instantiator {
            model: win.profileList
            delegate: P.MenuItem {
                icon.name: "bookmarks"
                text: modelData.name + (modelData.name === win.settings.default_profile ? "  (default)" : "")
                onTriggered: newTab({ profile: modelData.name })
            }
            onObjectAdded: (i, o) => launchMenu.insertItem(1 + i, o)
            onObjectRemoved: (i, o) => launchMenu.removeItem(o)
        }
        P.MenuSeparator {}
        P.MenuItem { id: shellsHeader; enabled: false; text: "Shells" }
        Instantiator {
            model: win.launchData.shells
            delegate: P.MenuItem {
                icon.name: "utilities-terminal"
                text: modelData.name + "\t" + modelData.path
                onTriggered: newTab({ command: modelData.path, title: modelData.name })
            }
            onObjectAdded: (i, o) => launchMenu.insertItem(win.profileList.length + 3 + i, o)
            onObjectRemoved: (i, o) => launchMenu.removeItem(o)
        }
        P.MenuSeparator {}
        P.MenuItem { enabled: false; text: "SSH" }
        Instantiator {
            model: win.launchData.ssh_hosts
            delegate: P.MenuItem {
                icon.name: "network-server"
                text: modelData
                onTriggered: newTab({ command: "ssh " + modelData, title: modelData })
            }
            onObjectAdded: (i, o) => launchMenu.insertItem(win.profileList.length + win.launchData.shells.length + 5 + i, o)
            onObjectRemoved: (i, o) => launchMenu.removeItem(o)
        }
        P.MenuItem {
            icon.name: "network-connect"
            text: "New SSH Connection\u2026"
            onTriggered: askText("New SSH Connection", "user@host (and any ssh options)", "", function(t) {
                t = t.trim()
                if (t !== "") newTab({ command: "ssh " + t, title: t.split(/\s+/).pop() })
            })
        }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "configure"; text: "Edit Profiles\u2026"; onTriggered: prefs.showProfile(activeProfile.name) }
    }

    // Right click on a tab.
    property var menuTab: null
    function renameTab(t) {
        if (t.tmuxWindow >= 0) {
            askText("Rename tmux Window", "Name", t.fixedTitle, function(s) {
                const g = tmuxGateways[t.gateway]
                if (g && s.trim() !== "") g.tmuxCommand("rename-window -t @" + t.tmuxWindow + " '" + s.replace(/'/g, "'\\''") + "'")
            })
        } else {
            askText("Tab Title", "Title (empty: automatic)", t.customTitle, function(s) { t.customTitle = s; treeRev++ })
        }
    }
    function closeTabs(list) {
        list = list.filter(t => t)
        if (list.length === 0) return
        const local = list.filter(t => t.tmuxWindow < 0)
        const count = local.reduce((n, t) => n + panesIn(t.root, []).length, 0)
        const doIt = function() {
            for (const t of list) {
                if (t.tmuxWindow >= 0) { closeTab(t); continue }
                if (settings.undo_close_seconds > 0) retire(serializeTab(t, true))
                else for (const p of panesIn(t.root, [])) p.view.killSession()
                removeTab(t)
            }
        }
        if (settings.confirm_close_multiple && count > 1)
            confirm("Close Tabs", "Close " + list.length + (list.length === 1 ? " tab" : " tabs") + " and " + count + " sessions?", doIt)
        else doIt()
    }
    function moveToNewWindow(t) {
        if (!t || t.tmuxWindow >= 0 || tabs.length < 2) return
        const json = tabPayload(t)
        removeTab(t)
        controller.newWindowWithTab(json)
    }
    P.Menu {
        id: tabMenu
        readonly property var t: win.menuTab
        readonly property int at: tabs.indexOf(t)
        P.MenuItem { icon.name: "tab-new"; text: "New Tab\tCtrl+Shift+T"; onTriggered: newTab({}) }
        P.MenuItem { icon.name: "tab-duplicate"; text: "Duplicate Tab"; enabled: tabMenu.t !== null && tabMenu.t.tmuxWindow < 0
            onTriggered: newTab({ layout: serializeTab(tabMenu.t, false), profile: tabMenu.t.profile }) }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "edit-rename"; text: "Rename Tab\u2026"; onTriggered: renameTab(tabMenu.t) }
        P.Menu {
            id: tabMenuColor
            title: "Tab Color"
            icon.name: "color-management"
            P.MenuItem { text: "No Color"; onTriggered: tabMenu.t.tabColor = "transparent" }
            P.MenuSeparator {}
            Instantiator {
                model: [["Red", "#e5534b"], ["Orange", "#e8853d"], ["Yellow", "#d9b43b"], ["Green", "#57ab5a"], ["Blue", "#4a90e2"], ["Purple", "#986ee2"], ["Gray", "#8b949e"]]
                delegate: P.MenuItem { text: modelData[0]; onTriggered: { tabMenu.t.tabColor = modelData[1]; treeRev++ } }
                onObjectAdded: (i, o) => tabMenuColor.insertItem(i + 2, o)
                onObjectRemoved: (i, o) => tabMenuColor.removeItem(o)
            }
        }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "arrow-left"; text: "Move Tab Left"; enabled: tabMenu.at > 0; onTriggered: moveTabToIndex(tabMenu.t, tabMenu.at - 1) }
        P.MenuItem { icon.name: "arrow-right"; text: "Move Tab Right"; enabled: tabMenu.at >= 0 && tabMenu.at < tabs.length - 1; onTriggered: moveTabToIndex(tabMenu.t, tabMenu.at + 1) }
        P.MenuItem { icon.name: "window-new"; text: "Move Tab to New Window"; enabled: tabMenu.t !== null && tabMenu.t.tmuxWindow < 0 && tabs.length > 1
            onTriggered: moveToNewWindow(tabMenu.t) }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "tab-close"; text: "Close Tab"; onTriggered: closeTab(tabMenu.t) }
        P.MenuItem { icon.name: "tab-close-other"; text: "Close Other Tabs"; enabled: tabs.length > 1
            onTriggered: closeTabs(tabs.filter(x => x !== tabMenu.t)) }
        P.MenuItem { icon.name: "tab-close"; text: "Close Tabs to the Right"; enabled: tabMenu.at >= 0 && tabMenu.at < tabs.length - 1
            onTriggered: closeTabs(tabs.slice(tabMenu.at + 1)) }
    }

    // Right click in a terminal (when the program isn't taking the mouse).
    P.Menu {
        id: contextMenu
        P.MenuItem { icon.name: "edit-copy"; text: "Copy\tCtrl+Shift+C"; enabled: contextView !== null && contextView.hasSelection; onTriggered: contextView.copy() }
        P.MenuItem { icon.name: "edit-paste"; text: "Paste\tCtrl+Shift+V"; onTriggered: pasteClipboard(contextView) }
        P.MenuItem { icon.name: "edit-select-all"; text: "Select All\tCtrl+Shift+A"; onTriggered: contextView.selectAll() }
        P.MenuItem { icon.name: "edit-find"; text: "Find\u2026\tCtrl+Shift+F"; onTriggered: { const p = paneOf(contextView); if (p) p.findBar.open() } }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "view-split-left-right"; text: "Split Right\tCtrl+Shift+D"; onTriggered: splitPane(paneOf(contextView), Qt.Horizontal) }
        P.MenuItem { icon.name: "view-split-top-bottom"; text: "Split Down\tCtrl+Shift+E"; onTriggered: splitPane(paneOf(contextView), Qt.Vertical) }
        P.MenuItem { icon.name: "zoom-fit-best"; text: "Maximize Pane\tCtrl+Shift+Enter"; checkable: true; checked: maximizedPane !== null; onTriggered: toggleMaximize() }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "edit-clear-history"; text: "Clear Buffer\tCtrl+Shift+K"; onTriggered: contextView.clearScrollback() }
        P.MenuItem { icon.name: "edit-clear"; text: "Reset"; onTriggered: contextView.resetTerminal() }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "document-edit"; text: "Edit Session…"; onTriggered: prefs.showProfile(paneOf(contextView).profileName) }
        P.MenuItem { icon.name: "configure"; text: "Preferences…\tCtrl+,"; onTriggered: prefs.showPage("") }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "view-close"; text: "Close\tCtrl+Shift+W"; onTriggered: requestClosePane(paneOf(contextView)) }
    }

    Preferences {
        id: prefs
        controller: controller
        settings: win.settings
        systemDark: win.systemDark
        activeProfileName: win.activeProfile.name
        transientParent: win
        onChangeSetting: (k, v) => win.setSetting(k, v)
        onChangeProfile: (n, k, v) => win.setProfile(n, k, v)
        onRestoreArrangement: (name) => win.restoreArrangement(win.arrangementMap[name], null)
        onSaveArrangement: (name) => controller.saveArrangement(name, JSON.stringify(win.serializeWindow(false)))
    }

    // ---- layout ----------------------------------------------------------

    readonly property bool tabsAtBottom: settings.tab_position === "bottom"
    readonly property bool showTabs: settings.always_show_tabs || tabs.length > 1 || tabs.some(t => t.tmuxWindow >= 0)
    readonly property real barHeight: 32

    // The tab row (with the + button and the drop target for dragged tabs),
    // used in the top bar or, with tabs at the bottom, below the terminal.
    Component {
        id: tabRowComp
        Item {
            id: box
            property bool flipped: false
            property real dropX: -1   // where a dragged tab would land

            // Tabs are placed by slot (not a Row) so a dragged tab can follow
            // the pointer while the others slide out of its way.
            Item {
                id: tabRow
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: tabs.length > 0 ? (tabs.length - 1) * step + tabWidth : 0
                readonly property real overlap: 5
                readonly property real available: box.width - 44
                readonly property real tabWidth: settings.stretch_tabs
                    ? Math.max(72, (available + overlap * (tabs.length - 1)) / Math.max(1, tabs.length))
                    : Math.max(72, Math.min(180, (available + overlap * (tabs.length - 1)) / Math.max(1, tabs.length)))
                readonly property real step: tabWidth - overlap
                property int dragFrom: -1
                property int dragTo: -1
                property real dragX: 0
                property bool settling: false
                Behavior on dragX { enabled: tabRow.settling; NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }

                function finishDrag() {
                    const from = dragFrom, to = dragTo
                    if (from < 0) return
                    const t = tabs[from]
                    settling = true
                    dragX = to * step
                    win.later(130, function() {
                        tabRow.dragFrom = -1
                        tabRow.settling = false
                        if (to !== from) win.moveTabToIndex(t, to)
                    })
                }

                Repeater {
                    model: tabs
                    delegate: TabButtonX {
                        id: tb
                        width: tabRow.tabWidth
                        height: 26
                        y: box.flipped ? 0 : tabRow.height - height
                        readonly property bool isDragged: index === tabRow.dragFrom
                        readonly property int slot: {
                            const f = tabRow.dragFrom, t = tabRow.dragTo
                            if (f < 0 || isDragged) return index
                            if (f < t && index > f && index <= t) return index - 1
                            if (f > t && index >= t && index < f) return index + 1
                            return index
                        }
                        x: isDragged ? tabRow.dragX : slot * tabRow.step
                        Behavior on x { enabled: !tb.isDragged; NumberAnimation { duration: 140; easing.type: Easing.OutCubic } }
                        z: isDragged ? 10 : (index === currentTab ? 2 : (hovered ? 1 : 0))
                        lifted: isDragged
                        text: modelData.tabTitle
                        number: settings.show_tab_numbers && index < 9 ? index + 1 : 0
                        current: index === currentTab
                        nextIsCurrent: index + 1 === currentTab
                        tmux: modelData.tmuxWindow >= 0
                        activity: modelData.activity
                        bell: modelData.bell
                        tabColor: modelData.tabColor
                        closeAlways: settings.tab_close_buttons
                        flipped: box.flipped
                        surface: Theme.bg
                        tabItem: modelData
                        canTearOff: modelData.tmuxWindow < 0
                        payload: function() { return win.tabPayload(modelData) }
                        onClicked: focusTab(index)
                        onCloseClicked: closeTab(modelData)
                        onMenuRequested: { win.menuTab = modelData; tabMenu.open() }
                        onDragMoved: (dx, dy) => {
                            if (tabRow.settling) return
                            if (tabRow.dragFrom < 0) {
                                tabRow.dragFrom = index
                                tabRow.dragTo = index
                                focusTab(index)
                            }
                            const maxX = (tabs.length - 1) * tabRow.step
                            tabRow.dragX = Math.max(0, Math.min(maxX, index * tabRow.step + dx))
                            tabRow.dragTo = Math.max(0, Math.min(tabs.length - 1, Math.round(tabRow.dragX / tabRow.step)))
                            // Pulled out of the bar: drag it to another window, or off on its own.
                            if (Math.abs(dy) > 34 && tb.canTearOff) {
                                tabRow.dragFrom = -1
                                tb.startExternalDrag()
                            }
                        }
                        onDragReleased: if (tb.isDragged) tabRow.finishDrag()
                        onDragFinished: (action) => win.onTabDragFinished(modelData, action)
                    }
                }
            }
            Rectangle {
                id: plus
                anchors.left: tabRow.right
                anchors.leftMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                anchors.verticalCenterOffset: box.flipped ? -2 : 2
                width: 22; height: 22; radius: 6
                color: plusHover.hovered ? Theme.tabHover : "transparent"
                Text { anchors.centerIn: parent; anchors.verticalCenterOffset: -1; text: "+"; color: plusHover.hovered ? Theme.text : Theme.textDim; font.pixelSize: 15 }
                HoverHandler { id: plusHover }
                TapHandler { onTapped: newTab({}) }
            }
            // ⌄ More ways to start a tab: profiles, shells, ssh hosts.
            Rectangle {
                anchors.left: plus.right
                anchors.leftMargin: 1
                anchors.verticalCenter: plus.verticalCenter
                width: 16; height: 22; radius: 5
                color: launchHover.hovered ? Theme.tabHover : "transparent"
                Text { anchors.centerIn: parent; anchors.verticalCenterOffset: -2; text: "\u2304"; color: launchHover.hovered ? Theme.text : Theme.textDim; font.pixelSize: 13 }
                HoverHandler { id: launchHover }
                TapHandler { onTapped: launchMenu.open(parent) }
            }

            // Where a dragged tab would go.
            Rectangle {
                visible: box.dropX >= 0
                x: box.dropX - 1
                y: 6
                width: 2
                height: parent.height - 12
                radius: 1
                color: Theme.accent
            }

            DropArea {
                anchors.fill: parent
                keys: ["application/x-tessera-tab"]
                function slot(x) {
                    return Math.max(0, Math.min(tabs.length, Math.round((x - tabRow.x) / tabRow.step)))
                }
                onEntered: (drag) => drag.accept(drag.source ? Qt.CopyAction : Qt.MoveAction)
                onPositionChanged: (drag) => { box.dropX = tabRow.x + slot(drag.x) * tabRow.step + 3 }
                onExited: box.dropX = -1
                onDropped: (drop) => {
                    box.dropX = -1
                    const at = slot(drop.x)
                    if (drop.source && drop.source.tabItem) {
                        // One of this window's tabs: reorder.
                        const t = drop.source.tabItem
                        drop.accept(Qt.CopyAction)
                        win.later(30, function() { win.moveTabTo(t, at) })
                        return
                    }
                    const json = drop.getDataAsString("application/x-tessera-tab")
                    if (!json) return
                    drop.accept(Qt.MoveAction)
                    win.receiveTab(json, at)
                }
            }
        }
    }

    GridLayout {
        id: rootLayout
        anchors.fill: parent
        // A thousandth of a pixel keeps text off exact half pixels at 130%.
        anchors.topMargin: 0.001
        columns: 1
        rowSpacing: 0

        // ---- the top bar: title bar, menu, tabs and window buttons ----------
        Rectangle {
            id: topBar
            Layout.row: 0
            Layout.fillWidth: true
            Layout.preferredHeight: win.visibility === Window.FullScreen && !showTabs ? 0 : barHeight
            color: Theme.chrome
            clip: true

            // Drag the bar to move the window; double-click to maximize.
            DragHandler {
                target: null
                onActiveChanged: if (active) win.startSystemMove()
            }
            TapHandler {
                gesturePolicy: TapHandler.DragThreshold
                onDoubleTapped: win.toggleMaximized()
            }

            // ☰ The app menu.
            Rectangle {
                id: menuButton
                anchors.left: parent.left
                anchors.leftMargin: win.macOS ? 78 : 6
                anchors.verticalCenter: parent.verticalCenter
                anchors.verticalCenterOffset: (showTabs && !tabsAtBottom) ? 2 : 0
                width: 26; height: 22; radius: 6
                color: menuHover.hovered ? Theme.tabHover : "transparent"
                Item {
                    anchors.centerIn: parent
                    width: 14; height: 10
                    Repeater {
                        model: 3
                        Rectangle { y: index * 4; width: 14; height: 1.6; radius: 0.8; color: menuHover.hovered ? Theme.text : Theme.textDim }
                    }
                }
                HoverHandler { id: menuHover }
                TapHandler { onTapped: appMenu.open(menuButton) }
            }

            Loader {
                active: showTabs && !tabsAtBottom
                anchors.left: menuButton.right
                anchors.leftMargin: 2
                anchors.right: controls.left
                anchors.rightMargin: 12
                anchors.bottom: parent.bottom
                height: 28
                sourceComponent: tabRowComp
            }

            // Without a tab row up here: the window title.
            Text {
                visible: !(showTabs && !tabsAtBottom)
                anchors.centerIn: parent
                width: Math.min(implicitWidth, parent.width - 2 * (controls.width + 60))
                text: win.title
                color: win.active ? Theme.text : Theme.textDim
                font.pixelSize: 13
                font.weight: Font.DemiBold
                elide: Text.ElideMiddle
            }

            WindowControls {
                id: controls
                visible: !win.macOS
                width: win.macOS ? 0 : implicitWidth
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                window: win
            }
        }

        Item {
            id: stack
            Layout.row: 1
            Layout.fillWidth: true
            Layout.fillHeight: true
        }

        // Tabs below the terminal (Appearance > Tab bar location).
        Rectangle {
            Layout.row: 2
            Layout.fillWidth: true
            Layout.preferredHeight: showTabs && tabsAtBottom ? 30 : 0
            visible: showTabs && tabsAtBottom
            color: Theme.chrome
            Loader {
                active: parent.visible
                anchors.fill: parent
                anchors.leftMargin: 8
                anchors.rightMargin: 8
                sourceComponent: tabRowComp
                onLoaded: item.flipped = true
            }
        }
    }

    // Without the system frame, these strips along the edges resize the window.
    Item {
        id: edges
        anchors.fill: parent
        z: 10000
        visible: !win.macOS && win.visibility !== Window.Maximized && win.visibility !== Window.FullScreen
        readonly property int grip: 5
        component Edge: MouseArea {
            property int edgesToMove
            hoverEnabled: true
            onPressed: win.startSystemResize(edgesToMove)
        }
        Edge { x: edges.grip; width: parent.width - 2 * edges.grip; height: edges.grip; edgesToMove: Qt.TopEdge; cursorShape: Qt.SizeVerCursor }
        Edge { x: edges.grip; y: parent.height - edges.grip; width: parent.width - 2 * edges.grip; height: edges.grip; edgesToMove: Qt.BottomEdge; cursorShape: Qt.SizeVerCursor }
        Edge { y: edges.grip; width: edges.grip; height: parent.height - 2 * edges.grip; edgesToMove: Qt.LeftEdge; cursorShape: Qt.SizeHorCursor }
        Edge { x: parent.width - edges.grip; y: edges.grip; width: edges.grip; height: parent.height - 2 * edges.grip; edgesToMove: Qt.RightEdge; cursorShape: Qt.SizeHorCursor }
        Edge { width: edges.grip * 2; height: edges.grip * 2; edgesToMove: Qt.TopEdge | Qt.LeftEdge; cursorShape: Qt.SizeFDiagCursor }
        Edge { x: parent.width - edges.grip * 2; width: edges.grip * 2; height: edges.grip * 2; edgesToMove: Qt.TopEdge | Qt.RightEdge; cursorShape: Qt.SizeBDiagCursor }
        Edge { y: parent.height - edges.grip * 2; width: edges.grip * 2; height: edges.grip * 2; edgesToMove: Qt.BottomEdge | Qt.LeftEdge; cursorShape: Qt.SizeBDiagCursor }
        Edge { x: parent.width - edges.grip * 2; y: parent.height - edges.grip * 2; width: edges.grip * 2; height: edges.grip * 2; edgesToMove: Qt.BottomEdge | Qt.RightEdge; cursorShape: Qt.SizeFDiagCursor }
    }

    // Dim background windows (Appearance > Dimming).
    Rectangle {
        anchors.fill: parent
        visible: settings.dim_background_windows && !win.active
        color: Theme.dark ? "black" : "white"
        opacity: settings.dim_amount
    }

    // ---- startup -----------------------------------------------------------

    Component.onCompleted: {
        let alive = {}
        let restored = false
        const torn = controller.initialTab()
        if (torn !== "") {
            try {
                const tl = JSON.parse(torn)
                newTab({ layout: tl, alive: sessionIds(tl.root, {}), profile: tl.profile })
                restored = tabs.length > 0
            } catch (e) {}
        } else if (controller.restoreAllowed()) {
            const ids = JSON.parse(controller.restorableSessions())
            for (const id of ids) alive[id] = true
            if (settings.restore_sessions && ids.length > 0) {
                let layout = null
                try { layout = JSON.parse(controller.lastLayout()) } catch (e) {}
                if (layout && layout.tabs) {
                    restoreArrangement(layout, alive)
                    if (layout.width > 200) { win.width = layout.width; win.height = layout.height }
                    if (tabs.length > 0) focusTab(Math.min(layout.current || 0, tabs.length - 1))
                }
                // Shells the saved layout didn't know about get tabs of their own.
                for (const id of ids) if (alive[id]) newTab({ attachId: id })
                restored = tabs.length > 0
            }
        }
        if (!restored) {
            newTab({})
            sizeTimer.start()
        }
        startupDone = true
        layoutSaver.restart()
        const cmd = controller.startupCommand()
        if (cmd !== "") runTimer.start()
        if (controller.thenCommand() !== "") thenTimer.start()
        if (controller.prefsPage() !== "") prefs.showPage(controller.prefsPage())
        if (controller.findQuery() !== "") findTimer.start()
        if (controller.screenshotPath() !== "") shotTimer.start()
    }

    // A new window takes the profile's columns × rows.
    Timer {
        id: sizeTimer
        interval: 50
        onTriggered: {
            const v = activeView
            if (!v || v.cellWidth <= 0) return
            const p = activeProfile
            win.width = Math.round(p.columns * v.cellWidth + 2 * v.padding)
            win.height = Math.round(p.rows * v.cellHeight + 2 * v.padding + barHeight)
        }
    }

    Timer {
        id: runTimer
        interval: 600
        onTriggered: if (activeView) activeView.sendText(controller.startupCommand() + "\n")
    }

    Timer {
        id: findTimer
        interval: 2200
        onTriggered: if (activePane) activePane.findBar.open(controller.findQuery())
    }

    Timer {
        id: thenTimer
        interval: 3000
        onTriggered: if (activeView) activeView.sendText(controller.thenCommand() + "\n")
    }

    Timer {
        id: shotTimer
        interval: controller.screenshotDelay()
        onTriggered: {
            const target = controller.prefsPage() !== "" ? prefs.contentRoot : rootLayout
            const ok = target.grabToImage(function(r) {
                r.saveToFile(controller.screenshotPath())
                Qt.quit()
            })
            if (!ok) Qt.quit()
        }
    }
}
