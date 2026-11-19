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
    // Shown from C++ once named, so the desktop can put it back where it was.
    visible: false
    objectName: "tessera-main"
    property bool showAtStart: true
    color: useTransparency && activeProfile.transparency > 0 ? "transparent" : Theme.bg
    // A unified title bar: the tab strip *is* the title bar. macOS keeps its
    // own traffic lights inside it; elsewhere the window has no system frame
    // and the bar draws KDE (Breeze) or Windows buttons and moves the window.
    readonly property bool macOS: Qt.platform.os === "osx"
    palette.toolTipBase: Theme.panel
    palette.toolTipText: Theme.text
    palette.highlight: Theme.mix(Theme.panel, Theme.accent, 0.35)
    palette.highlightedText: Theme.text
    palette.light: Theme.mix(Theme.panel, Theme.accent, 0.22)
    palette.midlight: Theme.mix(Theme.panel, Theme.accent, 0.35)
    flags: macOS ? (Qt.Window | Qt.ExpandedClientAreaHint | Qt.NoTitleBarBackgroundHint)
                 : (Qt.Window | Qt.FramelessWindowHint)
    title: (controller.isAdmin() ? "Administrator: " : "")
           + (customTitle !== "" ? customTitle : (activeView && activeView.title ? activeView.title : "Tessera"))

    property var tabs: []
    property int currentTab: 0
    property var activeView: null
    property real fontSize: 0          // a zoom (Ctrl+=/-) over the profile's size; 0 = none
    // The input editor's history (every shell's, merged), shared by the window's panes.
    property bool assistantOpen: false
    property real assistantWidth: 400
    function toggleAssistant() {
        assistantOpen = !assistantOpen
        if (assistantOpen) Qt.callLater(assistant.focusInput)
        else if (activeView) activeView.forceActiveFocus()
    }
    property var commandHistory: { try { return JSON.parse(controller.historyAll(5000)) } catch (e) { return [] } }
    property real zoomBase: 11         // the profile's size when zooming started
    readonly property int zoomPercent: fontSize > 0 ? Math.round(100 * fontSize / zoomBase) : 100
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
    // Shift+Tab from anywhere that doesn't keep it (include/bootstrap.h
    // TesseraBacktab): the active pane's input editor, when it's up.
    function backToEditor() {
        const p = activePane
        if (!p || !p.editorOn) return false
        p.preferEditor = true
        p.compose.focusEditor()
        return true
    }
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
            zoomPercent: win.zoomPercent
            inputEditor: win.settings.input_editor !== false
            editorKeys: win.settings.editor_keys || "emacs"
            editorPrompt: win.settings.editor_prompt !== false
            aiSettings: win.settings.ai || ({})
            promptEngine: win.settings.prompt_engine || "auto"
            promptSegments: win.settings.prompt_segments || ["dir", "git", "venv", "nix", "status", "duration", "|", "kube", "aws", "gcloud", "host", "time"]
            commandHistory: win.commandHistory
            controller: controller
            onEditorKeysPicked: (k) => win.setSetting("editor_keys", k)
            markdownStyle: win.settings.markdown_style || "rich"
            onMarkdownStylePicked: (m) => win.setSetting("markdown_style", m)
            dimInactive: win.settings.dim_inactive
            dimAmount: win.settings.dim_amount
            showTitle: win.settings.pane_titles
            badge: prof.badge
            broadcasting: win.broadcastMode !== "off" && win.broadcastTargets(tp).length > 1
            onContextMenu: (v, x, y) => {
                win.contextView = v
                win.contextLink = v.linkAt(x, y)
                win.contextSelection = v.hasSelection ? v.selectedText() : ""
                win.openMenuAt(termMenu, v, x, y)
            }
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
            property string customTitle: "" // Window > Edit Tab Title
            property string launchTitle: "" // what the + menu started (a shell, ssh host)
            property string profile: ""
            property color tabColor: "transparent"
            property bool activity: false
            property bool bell: false
            property var lastView: null
            // The pane the title follows: the last focused, else the first.
            readonly property var titleView: { win.treeRev; if (lastView) return lastView; const ps = win.panesIn(rootSplit, []); return ps.length ? ps[0].view : null }
            readonly property var lastPane: titleView ? win.paneOf(titleView) : null
            readonly property var prof: win.profileFor(profile)
            readonly property string tabTitle: customTitle !== "" ? customTitle
                : prof.tab_title !== "" ? prof.tab_title
                // What's running in it (vim, htop, ssh...); at a prompt, the
                // shell's own title or the folder.
                : lastPane && lastPane.docPath !== "" ? lastPane.docPath.replace(/^.*\//, "") + (lastPane.docDirty ? " \u2022" : "")
                : lastPane && lastPane.job !== "" ? lastPane.job
                // (At a prompt, the folder, the same whichever shell: shells'
                // own titles differ, fish's shortens the path.)
                : (win.settings.tab_titles_per_tab && titleView && titleView.title && !(lastPane && lastPane.editorOn) ? titleView.title
                   : launchTitle !== "" ? launchTitle
                   : win.folderName(titleView && titleView.pwd ? titleView.pwd : (lastPane ? lastPane.procCwd : ""))
                     || (lastPane && lastPane.shellName) || prof.name)
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
        // (Terminal or notebook is kept with the layout.)
        p.viewModeChanged.connect(function() { if (startupDone) layoutSaver.restart() })
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
            if (!p.view.exited || p.taken) return
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
            if (prof.auto_log && p.view.sessionId !== "" && !p.view.logging) {
                const stamp = new Date().toISOString().replace(/[:T]/g, "-").slice(0, 19)
                const folder = (prof.log_folder || "~/tessera-logs").replace(/\/$/, "")
                p.view.startLogging(folder + "/" + prof.name.replace(/[^A-Za-z0-9_-]/g, "_") + "-" + stamp + ".log")
            }
        })
        p.view.sessionEvent.connect(function(ev) { onSessionEvent(p.view, ev) })
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

    // A native menu at a point in `item` (menus need an item to sit under:
    // without one they open wherever the platform guesses, or not at all on
    // Wayland).
    Item { id: menuSpot; width: 1; height: 1; parent: win.contentItem; z: -1 }
    function openMenuAt(menu, item, x, y) {
        const pt = item.mapToItem(win.contentItem, x, y)
        menuSpot.x = pt.x
        menuSpot.y = pt.y - 1
        menu.open(menuSpot)
    }

    // A folder's name for a title: "~" for home.
    function folderName(path) {
        path = (path || "").replace(/^file:\/\/[^/]*/, "")
        if (path === "") return ""
        const home = controller.homeDir ? controller.homeDir() : ""
        // (The whole path, ~ for home: never shortened.)
        path = path.replace(/(.)\/$/, "$1")
        if (home !== "" && (path === home || path.startsWith(home + "/"))) return "~" + path.substring(home.length)
        return path
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
        const prof = profileFor(profileName !== undefined ? profileName : p.profileName)
        // (A split of another host's pane opens there too.)
        const props = { profileName: prof.name, cwd: cwdFor(prof, p.view), sshHost: p.sshHost }
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
            if (!p.view.exited && settings.undo_close_seconds > 0 && p.view.sessionId !== "")
                retire({ title: "", color: "", profile: p.profileName, root: serializeNode(p, true) })
            else p.view.killSession()
        }
        if (p.docPath !== "" && p.docDirty) p.saveDoc()
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
        const prof = profileFor(opts.profile !== undefined ? opts.profile : settings.default_profile)
        const t = tabComp.createObject(stack, { profile: prof.name })
        if (opts.layout) {
            buildSavedTree(t, opts.layout.root, opts.alive, !!opts.recreate)
            if (opts.layout.title) t.customTitle = opts.layout.title
            if (opts.layout.color) t.tabColor = opts.layout.color
            if (t.root.count === 0) { t.destroy(); return null }
        } else {
            const props = { profileName: prof.name }
            if (opts.command) props.commandOverride = opts.command
            if (opts.title) t.launchTitle = opts.title
            if (opts.attachId) props.attachId = opts.attachId
            if (opts.host) props.sshHost = opts.host
            if (opts.doc) { props.docPath = opts.doc; props.viewMode = "notebook" }
            // (The view the tab it's opened from is in, unless one's asked for.)
            else {
                const from = active ? paneOf(active) : null
                const v = opts.view || (from ? from.viewMode : "")
                if (v === "notebook" || v === "terminal") props.viewMode = v
            }
            const cwd = opts.cwd !== undefined ? opts.cwd : cwdFor(prof, active)
            if (cwd) props.cwd = cwd
            const p = makePane(t.root, props)
            t.root.addItem(p)
            focusPane(p)
        }
        const at = tabs.length
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
        if (fontSize <= 0) zoomBase = activeView ? activeView.fontPointSize : 11
        const base = fontSize > 0 ? fontSize : zoomBase
        fontSize = Math.max(6, Math.min(48, base + d))
        if (Math.abs(fontSize - zoomBase) < 0.01) fontSize = 0
    }

    // ---- closing, with iTerm2's confirmations ------------------------------

    // Ask before closing panes whose profile says so (always, or when a job
    // other than the shell runs).
    function confirmJobs(panes, doIt) {
        const busy = []
        let always = false
        for (const p of panes) {
            if (p.view.exited) continue
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

    // ---- screenshots and recordings ------------------------------------------
    // A screenshot (of the pane, or the window) goes to ~/Pictures/Screenshots
    // and the clipboard; a recording (asciicast) of a pane's output to
    // ~/Videos/Tessera, and can be made a GIF (agg) when it stops.
    function fileStamp() {
        const d = new Date(), z = n => (n < 10 ? "0" : "") + n
        return d.getFullYear() + "-" + z(d.getMonth() + 1) + "-" + z(d.getDate()) + " " + z(d.getHours()) + "." + z(d.getMinutes()) + "." + z(d.getSeconds())
    }
    function screenshot(item) {
        if (!item) return
        const dir = controller.homeDir() + "/Pictures/Screenshots"
        controller.fileAction("mkdirs", dir, "")
        const path = dir + "/Tessera " + fileStamp() + ".png"
        item.grabToImage(r => {
            if (!r.saveToFile(path)) { notice("Couldn't save the screenshot to " + dir, []); return }
            if (activeView) activeView.copyImageFile(path)
            notice("Screenshot saved and copied: " + path.replace(controller.homeDir(), "~"), [
                { label: "Open", run: () => controller.fileAction("open", path, "") },
                { label: "Show in folder", run: () => controller.fileAction("reveal", path, "") }])
        })
    }
    function toggleRecording(view) {
        if (!view) return
        if (view.recording) {
            const path = view.stopRecording()
            if (path === "") return
            notice("Recording saved: " + path.replace(controller.homeDir(), "~"), [
                { label: "Make a GIF", run: () => { controller.fileAction("gif", path, ""); notice("Making the GIF (agg)… it opens when it's done.", []) } },
                { label: "Show in folder", run: () => controller.fileAction("reveal", path, "") }])
        } else {
            const dir = controller.homeDir() + "/Videos/Tessera"
            controller.fileAction("mkdirs", dir, "")
            const path = view.startRecording(dir + "/Tessera " + fileStamp() + ".cast")
            notice(path !== "" ? "Recording this pane (the red dot); Session › Stop Recording ends it." : "Couldn't start recording in " + dir, [])
        }
    }
    // (Test hook: TESSERA_TEST_RECORD=<ms>: record the pane from 5 s for that
    // long, with a window screenshot at the start.)
    Timer {
        interval: 5000
        running: controller.testEnv("RECORD") !== ""
        property bool on: false
        onTriggered: {
            if (!on) { win.toggleRecording(activeView); win.screenshot(rootLayout); on = true; interval = Number(controller.testEnv("RECORD")); start() }
            else win.toggleRecording(activeView)
        }
    }
    // (Test hook: TESSERA_TEST_SWITCH=<shell>,<ms>: the pane switches to that shell then.)
    Timer { interval: Number(controller.testEnv("SWITCH").split(",")[1] || 0); running: controller.testEnv("SWITCH") !== ""; onTriggered: if (activePane) activePane.switchShell(controller.testEnv("SWITCH").split(",")[0]) }
    // (Test hook: TESSERA_TEST_HOVER=x,y[,ms] moves the pointer there after 6 s (or ms).)
    Timer { interval: Number(controller.testEnv("HOVER").split(",")[2] || 6000); running: controller.testEnv("HOVER") !== ""; onTriggered: { const p = controller.testEnv("HOVER").split(",").map(Number); controller.testMouse(3, p[0] - 1, p[1], 0, 0); controller.testMouse(3, p[0], p[1], 0, 0) } }
    // A note at the bottom of the window for a few seconds, with buttons.
    function notice(text, actions) { noticeBox.text = text; noticeBox.actions = actions; noticeBox.shown = true; noticeTimer.restart() }
    Timer { id: noticeTimer; interval: 7000; onTriggered: noticeBox.shown = false }
    Rectangle {
        id: noticeBox
        property string text: ""
        property var actions: []
        property bool shown: false
        parent: win.contentItem
        z: 1000
        visible: shown
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 70
        width: Math.min(parent.width - 40, noticeRow.implicitWidth + 24)
        height: 34
        radius: 6
        color: Theme.panel
        border.color: Theme.divider
        HoverHandler { onHoveredChanged: if (hovered) noticeTimer.stop(); else noticeTimer.restart() }
        Row {
            id: noticeRow
            anchors.centerIn: parent
            spacing: 10
            Text { text: noticeBox.text; color: Theme.text; font.pixelSize: 12; elide: Text.ElideMiddle; width: Math.min(implicitWidth, noticeBox.parent.width - 300); anchors.verticalCenter: parent.verticalCenter; anchors.verticalCenterOffset: 0.001; renderType: Text.NativeRendering }
            Repeater {
                model: noticeBox.actions
                Chip { required property var modelData; base: Theme.panel; label: modelData.label; anchors.verticalCenter: parent.verticalCenter; onClicked: { noticeBox.shown = false; modelData.run() } }
            }
            Chip { base: Theme.panel; label: "×"; anchors.verticalCenter: parent.verticalCenter; onClicked: noticeBox.shown = false }
        }
    }

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
        for (const p of allPanes()) p.closingWithWindow = true
        for (const p of allPanes()) if (p.docPath !== "" && p.docDirty) p.saveDoc()
        // Closed sessions waiting for Undo end now.
        for (const e of closedStack) { if (e.timer) e.timer.stop(); for (const id of e.ids) controller.killSession(id) }
        closedStack = []
        if (settings.restore_sessions && tabs.length > 0) {
            saveLayoutNow()
            saveSnapshots()
        } else {
            controller.saveLastLayout("")
            for (const p of allPanes()) p.view.killSession()
        }
    }

    // ---- layouts: session restore and window arrangements ------------------

    function serializeNode(n, withIds) {
        if (n.view !== undefined)
            return { t: "pane", profile: n.profileName, cwd: n.view.pwd || n.view.cwd, cmd: n.commandOverride,
                     mode: n.viewMode, doc: n.docPath, id: withIds ? n.view.sessionId : "", host: n.sshHost, w: n.width, h: n.height }
        const c = []
        for (let i = 0; i < n.count; ++i) c.push(serializeNode(n.itemAt(i), withIds))
        return { t: n.orientation === Qt.Horizontal ? "h" : "v", c: c, w: n.width, h: n.height }
    }

    function serializeTab(t, withIds) {
        return { title: t.customTitle, color: t.tabColor.a > 0 ? t.tabColor.toString() : "",
                 profile: t.profile, root: serializeNode(t.root, withIds) }
    }

    function serializeWindow(withIds) {
        return { width: win.width, height: win.height, x: win.x, y: win.y,
                 maximized: win.visibility === Window.Maximized, current: currentTab,
                 tabs: tabs.map(t => serializeTab(t, withIds)) }
    }

    // `alive`: session ids that can be reattached (null: start new shells).
    // `recreate`: a pane whose shell is gone comes back as a new shell in its
    // folder, under its saved screen (startup after a reboot).
    function buildSavedTree(t, node, alive, recreate) {
        function build(n, host) {
            if (n.t === "pane") {
                const props = { profileName: profileFor(n.profile).name, cwd: n.cwd || "", commandOverride: n.cmd || "", viewMode: n.mode === "notebook" ? "notebook" : "terminal", docPath: n.doc || "", sshHost: n.host || "" }
                // (Another host's: whether it's still running there is known
                // once connected; the pane looks then.)
                if (alive && n.host) {
                    if (n.id) { props.attachId = n.id; props.restoreSnapshot = snapshotPath(n.id) }
                } else if (alive) {
                    if (n.id && alive[n.id]) {
                        props.attachId = n.id
                        alive[n.id] = false
                    } else if (recreate) {
                        if (n.id) props.preloadFile = snapshotPath(n.id)
                    } else {
                        return null
                    }
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

    function restoreArrangement(layout, alive, recreate) {
        for (const tl of layout.tabs) newTab({ layout: tl, alive: alive, profile: tl.profile, background: true, recreate: !!recreate })
    }

    // Saved screens, for panes whose shell doesn't survive (a reboot).
    readonly property string stateFolder: controller.stateFolder()
    function snapshotPath(id) { return stateFolder + "/snapshots/" + id + ".vt" }
    function saveSnapshots() {
        if (!settings.restore_sessions) return
        const keep = []
        for (const p of allPanes()) {
            if (p.view.sessionId === "") continue
            if (p.view.saveSnapshot(snapshotPath(p.view.sessionId))) keep.push(p.view.sessionId)
        }
        controller.pruneSnapshots(JSON.stringify(keep))
    }
    Timer { interval: 120000; repeat: true; running: startupDone; onTriggered: win.saveSnapshots() }
    // The desktop's id for this window, so it can put it back (Wayland).
    Timer { interval: 4000; running: startupDone; onTriggered: controller.saveWindowSession() }

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
    // ---- Jupyter notebooks: each in a tab of its own -----------------------------
    function openNotebook(path) {
        if (!path) return
        // (Already open: go to it.)
        for (let i = 0; i < tabs.length; ++i)
            for (const p of panesIn(tabs[i].root, [])) if (p.docPath === path) { currentTab = i; focusPane(p); return }
        newTab({ doc: path, cwd: path.replace(/\/[^/]*$/, "") || undefined })
    }
    function newNotebook() {
        const dir = (activeView && activeView.pwd) || controller.homeDir()
        let n = 1, path = dir + "/Untitled.ipynb"
        const taken = (p) => JSON.parse(controller.readTextFile(p)).error === undefined || allPanes().some(x => x.docPath === p)
        while (taken(path)) path = dir + "/Untitled-" + (++n) + ".ipynb"
        openNotebook(path)
    }
    function localPath(u) { return decodeURIComponent(String(u).replace(/^file:\/\//, "")) }
    FileDialog {
        id: notebookDialog
        title: "Open Jupyter Notebook"
        fileMode: FileDialog.OpenFile
        nameFilters: ["Jupyter notebooks (*.ipynb)", "All files (*)"]
        onAccepted: win.openNotebook(win.localPath(selectedFile))
    }
    Shortcut { sequence: "Ctrl+O"; autoRepeat: false; onActivated: notebookDialog.open() }
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
        return JSON.stringify(serializeTab(t, true))
    }

    function onTabDragFinished(t, action) {
        if (action === Qt.MoveAction) {
            // Another window took it (and reattaches its sessions).
            later(30, function() { removeTab(t) })
        } else if (action === Qt.IgnoreAction && tabs.length > 1) {
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

    // Events from a terminal for the window: a trigger's notification.
    function onSessionEvent(view, ev) {
        if (ev.type === "notify") controller.notify(ev.title, ev.body)
    }

    // ---- shortcuts (iTerm2's, with Ctrl+Shift where it has Cmd) -------------
    // The menus show these keys in their text; the keys themselves live here
    // (one place, and closing doesn't auto-repeat when the key is held).

    Shortcut { sequence: "Ctrl+Shift+T"; onActivated: newTab({}) }
    Shortcut { sequence: "Ctrl+Shift+N"; autoRepeat: false; onActivated: controller.newWindow() }
    // Ctrl+W stays the shell's (delete a word).
    Shortcut { sequence: "Ctrl+Alt+W"; autoRepeat: false; onActivated: closeActivePane() }
    Shortcut { sequence: "Ctrl+Shift+I"; onActivated: win.toggleAssistant() }
    Shortcut {
        sequence: "Ctrl+Shift+W"
        autoRepeat: false
        onActivated: if (currentTab >= 0 && currentTab < tabs.length) closeTab(tabs[currentTab])
    }
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
    // Terminal <-> notebook for the active pane.
    Shortcut { sequence: "Ctrl+Shift+J"; autoRepeat: false; onActivated: if (activePane) activePane.viewMode = activePane.viewMode === "notebook" ? "terminal" : "notebook" }
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
        onAboutToShow: win.appMenuOpen = true
        onAboutToHide: win.appMenuOpen = false

        P.Menu {
            title: "Shell"
            icon.name: "utilities-terminal"
            P.MenuItem { icon.name: "tab-new"; text: "New Tab\tCtrl+Shift+T"; onTriggered: newTab({}) }
            P.MenuItem { icon.name: "document-new"; text: "New Jupyter Notebook"; onTriggered: newNotebook() }
            P.MenuItem { icon.name: "document-open"; text: "Open Jupyter Notebook\u2026\tCtrl+O"; onTriggered: notebookDialog.open() }
            P.MenuItem { icon.name: "tab-new"; text: "New Tab with Current Profile"; onTriggered: newTab({ profile: activeProfile.name }) }
            P.MenuItem { icon.name: "window-new"; text: "New Window\tCtrl+Shift+N"; onTriggered: controller.newWindow() }
            P.MenuItem { icon.name: "security-high"; text: "New Administrator Window"; visible: Qt.platform.os === "windows" && !controller.isAdmin(); onTriggered: controller.newAdminWindow() }
            P.MenuItem { icon.name: "tab-duplicate"; text: "Duplicate Tab"; enabled: tabs.length > 0
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
            P.MenuItem { icon.name: "tab-close"; text: "Close Tab\tCtrl+Shift+W"; enabled: tabs.length > 0; onTriggered: closeTab(tabs[currentTab]) }
            P.MenuItem { icon.name: "view-close"; text: "Close Pane\tCtrl+Alt+W"; onTriggered: closeActivePane() }
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
            P.MenuItem { icon.name: "help-hint"; text: (win.assistantOpen ? "Hide" : "Show") + " Assistant\tCtrl+Shift+I"; onTriggered: win.toggleAssistant() }
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
            P.MenuItem { icon.name: "view-refresh"; text: "Restart Session"; enabled: activeView !== null
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
            P.MenuItem { icon.name: activeView && activeView.recording ? "media-playback-stop" : "media-record"; text: activeView && activeView.recording ? "Stop Recording" : "Record Pane (asciicast)"
                enabled: activeView !== null; onTriggered: win.toggleRecording(activeView) }
            P.MenuItem { icon.name: "camera-photo"; text: "Screenshot of Pane"; enabled: activeView !== null; onTriggered: win.screenshot(paneOf(activeView)) }
            P.MenuItem { icon.name: "camera-photo"; text: "Screenshot of Window"; onTriggered: win.screenshot(rootLayout) }
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
        P.MenuItem { icon.name: "configure"; text: "Preferences…\tCtrl+,"; onTriggered: prefs.showPage("") }
        P.MenuItem { icon.name: "application-exit"; text: "Quit Tessera\tCtrl+Shift+Q"; onTriggered: win.close() }
    }

    // The + button's menu: profiles, the shells installed, ssh hosts.
    property var launchData: ({ shells: [], ssh_hosts: [] })
    property bool launchOpen: false
    property bool appMenuOpen: false
    P.Menu {
        id: launchMenu
        onAboutToShow: { win.launchData = JSON.parse(controller.launchers()); win.launchOpen = true }
        onAboutToHide: win.launchOpen = false
        // (Either view, whichever the current tab is in.)
        P.MenuItem { icon.name: "view-list-details"; text: "New Notebook Tab"; onTriggered: newTab({ view: "notebook" }) }
        P.MenuItem { icon.name: "utilities-terminal"; text: "New Terminal Tab"; onTriggered: newTab({ view: "terminal" }) }
        P.MenuSeparator {}
        Instantiator {
            model: win.profileList
            delegate: P.MenuItem {
                icon.name: "utilities-terminal"
                text: modelData.name + (index < 9 ? "\tCtrl+Shift+" + (index + 1) : "")
                onTriggered: newTab({ profile: modelData.name })
            }
            onObjectAdded: (i, o) => launchMenu.insertItem(i + 3, o)
            onObjectRemoved: (i, o) => launchMenu.removeItem(o)
        }
        P.MenuSeparator {}
        P.Menu {
            id: shellsMenu
            title: "Shells"
            icon.name: "utilities-terminal"
            enabled: win.launchData.shells.length > 0
            Instantiator {
                model: win.launchData.shells
                delegate: P.MenuItem {
                    text: modelData.name + "\t" + modelData.path
                    onTriggered: newTab({ command: modelData.path, title: modelData.name })
                }
                onObjectAdded: (i, o) => shellsMenu.insertItem(i, o)
                onObjectRemoved: (i, o) => shellsMenu.removeItem(o)
            }
        }
        P.Menu {
            id: sshMenu
            title: "SSH"
            icon.name: "network-server"
            Instantiator {
                model: win.launchData.ssh_hosts
                delegate: P.MenuItem {
                    icon.name: "network-server"
                    text: modelData
                    // (Its shells and notebook kernels run there, in tesserad.)
                    onTriggered: newTab({ host: modelData, title: modelData })
                }
                onObjectAdded: (i, o) => sshMenu.insertItem(i, o)
                onObjectRemoved: (i, o) => sshMenu.removeItem(o)
            }
            P.MenuSeparator { visible: win.launchData.ssh_hosts.length > 0 }
            P.MenuItem {
                icon.name: "network-connect"
                text: "New Connection\u2026"
                onTriggered: askText("New SSH Connection", "user@host, and any ssh options", "", function(t) {
                    t = t.trim()
                    if (t !== "") newTab({ host: t, title: t.split(/\s+/).pop() })
                })
            }
        }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "configure"; text: "Edit Profiles\u2026"; onTriggered: prefs.showProfile(activeProfile.name) }
    }
    Repeater {
        model: 9
        delegate: Item {
            Shortcut {
                sequence: "Ctrl+Shift+" + (index + 1)
                onActivated: if (index < win.profileList.length) newTab({ profile: win.profileList[index].name })
            }
        }
    }

    // Right click on a tab.
    property var menuTab: null
    function renameTab(t) {
        askText("Tab Title", "Title (empty: automatic)", t.customTitle, function(s) { t.customTitle = s; treeRev++ })
    }
    function closeTabs(list) {
        list = list.filter(t => t)
        if (list.length === 0) return
        const count = list.reduce((n, t) => n + panesIn(t.root, []).length, 0)
        const doIt = function() {
            for (const t of list) {
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
        if (!t || tabs.length < 2) return
        const json = tabPayload(t)
        removeTab(t)
        controller.newWindowWithTab(json)
    }
    P.Menu {
        id: tabMenu
        readonly property var t: win.menuTab
        readonly property int at: tabs.indexOf(t)
        P.MenuItem { icon.name: "tab-new"; text: "New Tab\tCtrl+Shift+T"; onTriggered: newTab({}) }
        P.MenuItem { icon.name: "tab-duplicate"; text: "Duplicate Tab"; enabled: tabMenu.t !== null
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
        P.MenuItem { icon.name: "window-new"; text: "Move Tab to New Window"; enabled: tabMenu.t !== null && tabs.length > 1
            onTriggered: moveToNewWindow(tabMenu.t) }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "tab-close"; text: "Close Tab\tCtrl+Shift+W"; onTriggered: closeTab(tabMenu.t) }
        P.MenuItem { icon.name: "tab-close-other"; text: "Close Other Tabs"; enabled: tabs.length > 1
            onTriggered: closeTabs(tabs.filter(x => x !== tabMenu.t)) }
        P.MenuItem { icon.name: "tab-close"; text: "Close Tabs to the Right"; enabled: tabMenu.at >= 0 && tabMenu.at < tabs.length - 1
            onTriggered: closeTabs(tabs.slice(tabMenu.at + 1)) }
    }

    // Right click in a terminal (when the program isn't taking the mouse):
    // iTerm2's context menu, depending on what's under the pointer.
    property string contextLink: ""
    property string contextSelection: ""
    readonly property string contextSelShort: contextSelection.length > 30 ? contextSelection.slice(0, 28).trim() + "…" : contextSelection.trim()

    function movePaneToNewTab(p, window) {
        if (!p) return
        const layout = { title: "", color: "", profile: p.profileName, root: serializeNode(p, true) }
        const json = JSON.stringify(layout)
        closePane(p, false)   // detaches; the new tab or window reattaches it
        if (window) controller.newWindowWithTab(json)
        else later(150, function() { newTab({ layout: layout, alive: sessionIds(layout.root, {}), profile: layout.profile }) })
    }

    // Right-click on the zoom chip: preset sizes.
    property bool zoomMenuOpen: false
    function zoomTo(pct) {
        if (fontSize <= 0) zoomBase = activeView ? activeView.fontPointSize : 11
        fontSize = pct === 100 ? 0 : Math.max(6, Math.min(48, Math.round(zoomBase * pct / 10) / 10))
    }
    // Each tab's running program, for its title: asked of the daemon once a
    // second (the foreground process group of the pane's terminal).
    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            // (Every pane: titles, and each editor's prompt and REPL detection.)
            for (const p of allPanes()) {
                const v = p.view
                if (!v) continue
                let info = { job: "", cwd: "", shell: "" }
                if (!v.exited && v.sessionId !== "") {
                    try { info = JSON.parse(controller.sessionInfo(v.sessionId)) } catch (e) {}
                }
                if (p.job !== info.job) p.job = info.job
                if (p.procCwd !== info.cwd) p.procCwd = info.cwd
                if (p.shellName !== info.shell) p.shellName = info.shell
                if (info.pid && p.shellPid !== info.pid) p.shellPid = info.pid
            }
        }
    }
    // Shells in a daemon from before in-place upgrades: it can't take this
    // build's changes (shell integration, the input editor, ...) without a
    // restart. Asked once per start, after the window is up.
    Timer {
        id: daemonTimer
        interval: 1500
        onTriggered: sheet.ask({
            title: "Restart the shells' background process?",
            message: "Your shells run in Tessera's background process from an older version, which can't update itself. "
                   + "Restarting it gives them this version's features (prompt marks, the input editor, clean zoom). "
                   + "Every tab comes back in its folder with its screen; programs running in them (and in other Tessera windows) end.\n\n"
                   + "From now on it updates itself in place, keeping your shells.",
            okText: "Restart Shells",
            destructive: true,
            onAccept: function() {
                // New shells (from a new daemon) in every tab's place, each
                // under its saved screen; then the old tabs go.
                win.saveSnapshots()
                const layout = win.serializeWindow(true)
                const old = win.tabs.slice(), cur = win.currentTab
                if (!controller.stopDaemon()) {
                    sheet.ask({ title: "Couldn't restart it", message: "The background process didn't stop. Logging out and back in restarts it.", infoOnly: true, okText: "OK" })
                    return
                }
                win.restoreArrangement(layout, {}, true)
                for (const t of old) win.removeTab(t)
                win.focusTab(Math.min(cur, win.tabs.length - 1))
                win.saveLayoutNow()
            }
        })
    }
    // Test hook: TESSERA_TEST_KEYS="term|editor|away:key,key,..." (tab, bs, enter,
    // lshift, rshift, or text), one key every 150 ms after the window settles.
    Timer {
        id: keysTestTimer
        interval: 4500
        property var keys: []
        onTriggered: {
            const spec = controller.testEnv("KEYS")
            const [target, list] = spec.split(":")
            if (target === "away") rootLayout.forceActiveFocus()   // (somewhere that doesn't keep Shift+Tab)
            else if (target === "keep") {}                          // (wherever the keys are)
            else if (activePane) { if (target === "term") { activePane.preferEditor = false; activePane.view.forceActiveFocus() } else activePane.compose.focusEditor() }
            keys = list.split(",")
            keyStep.start()
        }
    }
    Timer {
        id: keyStep
        interval: 150; repeat: true
        onTriggered: {
            if (keysTestTimer.keys.length === 0) { stop(); return }
            const k = keysTestTimer.keys.shift()
            if (k === "wait") { if (activePane) console.log("key test: wait ghost=" + JSON.stringify(activePane.compose.ghost) + " ready=" + activePane.compose.inlineReady); return }   // (a pause: let the AI answer)
            const named = { space: [Qt.Key_Space, 0, " ", 65], tab: [Qt.Key_Tab, 0, "\t", 23], bs: [Qt.Key_Backspace, 0, "\b", 22], enter: [Qt.Key_Return, 0, "\r", 36],
                            lshift: [Qt.Key_Shift, 0, "", 50], rshift: [Qt.Key_Shift, 0, "", 62], esc: [Qt.Key_Escape, 0, "\x1b", 9], stab: [Qt.Key_Backtab, Qt.ShiftModifier, "", 23],
                            up: [Qt.Key_Up, 0, "", 111], down: [Qt.Key_Down, 0, "", 116],
                            pgup: [Qt.Key_PageUp, 0, "", 112], pgdn: [Qt.Key_PageDown, 0, "", 117] }
            const d = named[k] || [k.toUpperCase().charCodeAt(0), 0, k, 0]
            controller.testKey(d[0], d[1], d[2], d[3])
            if (activePane) console.log("key test: after " + k + " termFocus=" + activePane.view.activeFocus + " editorFocus=" + activePane.compose.editing + " focusItem=" + win.activeFocusItem + " editor=" + JSON.stringify(activePane.compose.text) + " ghost=" + JSON.stringify(activePane.compose.ghost) + " dropdown=" + activePane.compose.compOpen)
        }
    }
    // (Test hook: TESSERA_TEST_FLICKER=<dir>: a new tab, then back to the
    // first, each followed by 12 frames grabbed into <dir>.)
    property int flickFrames: -1
    property string flickTag: ""
    Timer {
        id: flickerTestTimer
        interval: 9000
        property int step: 0
        onTriggered: {
            // (With TESSERA_TEST_NOTEBOOK set, both tabs in notebook view.)
            const nbk = controller.testEnv("NOTEBOOK") !== ""
            if (step === 0) { if (nbk && activePane) activePane.viewMode = "notebook"; win.flickTag = "new"; win.flickFrames = 0; newTab({}) ; interval = 3000; step = 1; start() }
            else if (step === 1) { if (nbk && activePane) activePane.viewMode = "notebook"; win.flickTag = "back"; win.flickFrames = 0; currentTab = 0; interval = 3000; step = 2; start() }
            else if (step === 2) { win.flickTag = "fwd"; win.flickFrames = 0; currentTab = 1 }
        }
    }
    onFrameSwapped: {
        if (flickFrames < 0 || flickFrames >= 12) return
        const n = flickFrames++, tag = flickTag
        rootLayout.grabToImage(r => r.saveToFile(controller.testEnv("FLICKER") + "/" + tag + "-" + (n < 10 ? "0" : "") + n + ".png"))
    }
    // (Test hook: TESSERA_TEST_DRAG="x1,y1,x2,y2,mods[,right]" drags (or right-clicks) after 12 s.)
    Timer {
        id: dragTestTimer
        interval: 12000
        property var p: controller.testEnv("DRAG").split(",").map(Number)
        property int step: 0
        repeat: true
        onTriggered: {
            const [x1, y1, x2, y2, m, right] = p
            if (right) { controller.testMouse(0, x2, y2, 2, m); controller.testMouse(2, x2, y2, 2, m); stop(); return }
            if (step === 0) { controller.testMouse(0, x1, y1, 1, m); interval = 30 }
            else if (step <= 10) controller.testMouse(1, x1 + (x2 - x1) * step / 10, y1 + (y2 - y1) * step / 10, 1, m)
            else { controller.testMouse(2, x2, y2, 1, m); stop() }
            step++
        }
    }
    // (Test hook: TESSERA_TEST_TABS=<n> opens n more tabs at 3 s; with
    // TESSERA_TEST_TAB_MENU, the all-tabs dropdown opens at 8 s.)
    Timer { interval: 3000; running: controller.testEnv("TABS") !== ""; onTriggered: { for (let i = 0; i < Number(controller.testEnv("TABS")); ++i) newTab({}) } }
    Timer { interval: 8000; running: controller.testEnv("TAB_MENU") !== ""
            onTriggered: tabMenuTest.go() }
    QtObject {
        id: tabMenuTest
        function go() {
            // (The ⌄ is inside the tab strip's Loader: found by its object name.)
            const find = (it) => { if (!it) return null; if (it.objectName === "tabOverflow") return it; for (let i = 0; i < it.children.length; ++i) { const r = find(it.children[i]); if (r) return r } return null }
            const b = find(rootLayout)
            if (b) b.openMenu(); else console.log("tab menu test: no overflow button")
        }
    }
    Timer { id: notebookTestTimer; interval: 9000; onTriggered: if (activePane) activePane.viewMode = "notebook" }
    // (Test hook: TESSERA_TEST_TAB=<n> goes to that tab once the window is restored.)
    Timer { id: tabTestTimer; interval: 5000; onTriggered: { currentTab = parseInt(controller.testEnv("TAB")); if (activePane) focusPane(activePane) } }
    Timer { id: richTestTimer; interval: 6000; onTriggered: if (activePane) activePane.openRich() }
    Timer { id: assistantTestTimer; interval: 3000; onTriggered: { win.toggleAssistant(); assistant.send(controller.testEnv("ASSISTANT")) } }
    Timer { id: composeTestTimer; interval: Number(controller.testEnv("COMPOSE_DELAY") || 4000); onTriggered: if (activePane) { activePane.compose.focusEditor(); if (controller.testEnv("COMPOSE_LANG") !== "") activePane.compose.pickLanguage(controller.testEnv("COMPOSE_LANG")); activePane.compose.insert(controller.testEnv("COMPOSE").replace(/\\n/g, "\n")); if (controller.testEnv("COMPOSE_RUN") !== "") activePane.compose.submit(); if (controller.testEnv("COMPOSE_HISTORY") !== "") activePane.compose.openHistory() } }
    Timer { id: contextTestTimer; interval: 2500; onTriggered: { if (controller.testEnv("CONTEXT") === "app") appMenu.open(menuButton); else if (activePane) activePane.contextMenu(activeView, 200, 150) } }
    Timer { id: zoomTestTimer; interval: 2500; onTriggered: win.zoomTo(parseInt(controller.testEnv("ZOOM"))) }
    P.Menu {
        id: zoomMenu
        onAboutToShow: win.zoomMenuOpen = true
        onAboutToHide: win.zoomMenuOpen = false
        Instantiator {
            model: [75, 90, 100, 110, 125, 150, 175, 200]
            delegate: P.MenuItem {
                text: modelData + "%"
                checkable: true
                checked: win.zoomPercent === modelData
                onTriggered: win.zoomTo(modelData)
            }
            onObjectAdded: (i, o) => zoomMenu.insertItem(i, o)
            onObjectRemoved: (i, o) => zoomMenu.removeItem(o)
        }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "zoom-in"; text: "Bigger\tCtrl+="; onTriggered: zoomBy(1) }
        P.MenuItem { icon.name: "zoom-out"; text: "Smaller\tCtrl+-"; onTriggered: zoomBy(-1) }
    }

    P.Menu {
        id: termMenu
        // -- what's under the pointer
        // (A file under the pointer: shown in Tessera, FilePreview.)
        // (A path in another host's output is ssh://<host>/<path>: viewed
        // and opened through that host; its address is the path there.)
        readonly property string contextPath: contextLink.startsWith("ssh://") ? contextLink.replace(/^ssh:\/\/[^/]*/, "") : contextLink.replace(/^file:\/\//, "")
        P.MenuItem { icon.name: "document-preview"; text: "View “" + win.shortLabel(termMenu.contextPath) + "”"
            visible: contextLink.startsWith("file://") || contextLink.startsWith("ssh://")
            onTriggered: { const p = paneOf(contextView); let f = contextLink.startsWith("ssh://") ? contextLink : contextLink.replace(/^file:\/\/[^/]*/, ""); try { if (!f.startsWith("ssh://")) f = decodeURIComponent(f) } catch (e) {} if (p) p.viewFile(f) } }
        P.MenuItem { icon.name: "internet-services"; text: "Open “" + win.shortLabel(termMenu.contextPath) + "”"
            visible: contextLink !== ""; onTriggered: { if (contextLink.startsWith("ssh://")) { const e = controller.fileAction("open", contextLink, ""); if (e !== "") notice(e, []) } else Qt.openUrlExternally(contextLink) } }
        P.MenuItem { icon.name: "edit-copy"; text: "Copy Link Address"; visible: contextLink !== ""
            onTriggered: { clipHelper.text = termMenu.contextPath; clipHelper.selectAll(); clipHelper.copy() } }
        P.MenuSeparator { visible: contextLink !== "" }
        // -- the selection
        P.MenuItem { icon.name: "search"; text: "Search the Web for “" + contextSelShort + "”"; visible: contextSelection !== ""
            onTriggered: Qt.openUrlExternally("https://duckduckgo.com/?q=" + encodeURIComponent(contextSelection.trim())) }
        P.MenuItem { icon.name: "edit-find"; text: "Find “" + contextSelShort + "”"; visible: contextSelection !== ""
            onTriggered: { const p = paneOf(contextView); if (p) p.findBar.open(contextSelection.split("\n")[0]) } }
        P.MenuItem { icon.name: "go-jump"; text: "Send Selection"; visible: contextSelection !== ""
            onTriggered: contextView.pasteText(contextSelection) }
        P.MenuItem { icon.name: "insert-text"; text: "Save Selection as Snippet…"; visible: contextSelection !== ""
            onTriggered: { const sel = contextSelection, brief = contextSelShort
                askText("Save as Snippet", "Title", brief, function(t) {
                    setSetting("snippets", (settings.snippets || []).concat([{ title: t || brief, text: sel }]))
                }) } }
        P.MenuSeparator { visible: contextSelection !== "" }
        // -- copy and paste
        P.MenuItem { icon.name: "edit-copy"; text: "Copy\tCtrl+Shift+C"; enabled: contextSelection !== ""; onTriggered: contextView.copy() }
        P.MenuItem { icon.name: "edit-copy"; text: "Copy with Styles"; enabled: contextSelection !== ""; onTriggered: contextView.copyWithStyles() }
        P.MenuItem { icon.name: "edit-copy"; text: "Copy with Control Sequences"; enabled: contextSelection !== ""; onTriggered: contextView.copyWithControlSequences() }
        P.MenuItem { icon.name: "edit-paste"; text: "Paste\tCtrl+Shift+V"; onTriggered: pasteClipboard(contextView) }
        P.Menu {
            id: ctxPasteSpecial
            title: "Paste Special"
            icon.name: "edit-paste"
            P.MenuItem { text: "Paste Selection\tShift+Insert"; onTriggered: contextView.pasteSelection() }
            P.MenuItem { text: "Paste Slowly"; onTriggered: contextView.pasteSlowly() }
            P.MenuSeparator { visible: pasteHistory.length > 0 }
            Instantiator {
                model: win.pasteHistory
                delegate: P.MenuItem { text: win.shortLabel(modelData); onTriggered: pasteInto(contextView, modelData) }
                onObjectAdded: (i, o) => ctxPasteSpecial.insertItem(3 + i, o)
                onObjectRemoved: (i, o) => ctxPasteSpecial.removeItem(o)
            }
        }
        P.MenuItem { icon.name: "edit-select-all"; text: "Select All\tCtrl+Shift+A"; onTriggered: contextView.selectAll() }
        P.MenuItem { icon.name: "edit-select"; text: "Select Output of Last Command\tCtrl+Shift+L"; onTriggered: contextView.selectLastOutput() }
        P.MenuSeparator {}
        // -- panes
        P.MenuItem { icon.name: "view-split-left-right"; text: "Split Right\tCtrl+Shift+D"; onTriggered: splitPane(paneOf(contextView), Qt.Horizontal) }
        P.MenuItem { icon.name: "view-split-top-bottom"; text: "Split Down\tCtrl+Shift+E"; onTriggered: splitPane(paneOf(contextView), Qt.Vertical) }
        P.MenuItem { icon.name: "zoom-fit-best"; text: "Maximize Pane\tCtrl+Shift+Enter"; checkable: true; checked: maximizedPane !== null
            enabled: maximizedPane !== null || (contextView !== null && paneOf(contextView) !== null && paneOf(contextView).split); onTriggered: toggleMaximize() }
        P.MenuItem { icon.name: "tab-new"; text: "Move Pane to New Tab"
            enabled: contextView !== null && paneOf(contextView) !== null && paneOf(contextView).split
            onTriggered: movePaneToNewTab(paneOf(contextView), false) }
        P.MenuItem { icon.name: "window-new"; text: "Move Pane to New Window"
            enabled: contextView !== null && (tabs.length > 1 || (paneOf(contextView) !== null && paneOf(contextView).split))
            onTriggered: movePaneToNewTab(paneOf(contextView), true) }
        P.MenuSeparator {}
        // -- the session
        P.Menu {
            title: "Broadcast Input"
            icon.name: "network-transmit"
            P.MenuItemGroup { id: ctxBroadcastGroup }
            P.MenuItem { text: "Send Input to Current Session Only"; checkable: true; group: ctxBroadcastGroup; checked: broadcastMode === "off"; onTriggered: broadcastMode = "off" }
            P.MenuItem { text: "Broadcast to All Panes in Current Tab"; checkable: true; group: ctxBroadcastGroup; checked: broadcastMode === "tab"; onTriggered: broadcastMode = "tab" }
            P.MenuItem { text: "Broadcast to All Panes in All Tabs"; checkable: true; group: ctxBroadcastGroup; checked: broadcastMode === "all"; onTriggered: broadcastMode = "all" }
        }
        P.MenuItem { icon.name: "edit-find"; text: "Find…\tCtrl+Shift+F"; onTriggered: { const p = paneOf(contextView); if (p) p.findBar.open() } }
        P.MenuItem { icon.name: "link"; text: "Find URLs"; onTriggered: { const p = paneOf(contextView); if (p) p.findBar.findUrls() } }
        P.MenuItem { icon.name: "media-record"; text: contextView && contextView.logging ? "Stop Logging" : "Log to File…"
            onTriggered: contextView.logging ? contextView.stopLogging() : logDialog.open() }
        P.MenuItem { icon.name: contextView && contextView.recording ? "media-playback-stop" : "media-record"; text: contextView && contextView.recording ? "Stop Recording" : "Record Pane"
            onTriggered: win.toggleRecording(contextView) }
        P.MenuItem { icon.name: "camera-photo"; text: "Screenshot of Pane"; onTriggered: win.screenshot(paneOf(contextView)) }
        P.MenuItem { icon.name: "edit-clear-history"; text: "Clear Buffer\tCtrl+Shift+K"; onTriggered: contextView.clearScrollback() }
        P.MenuItem { icon.name: "edit-clear-history"; text: "Clear Scrollback Buffer"; onTriggered: contextView.showMessage("\u001b[3J") }
        P.MenuItem { icon.name: "edit-clear"; text: "Reset"; onTriggered: contextView.resetTerminal() }
        P.MenuItem { icon.name: "view-refresh"; text: "Restart Session"; enabled: contextView !== null
            onTriggered: { const v = contextView; confirmJobs([paneOf(v)], function() { v.restart() }) } }
        P.MenuSeparator {}
        P.Menu {
            icon.name: "zoom-original"
            title: "Text Size (" + win.zoomPercent + "%)"
            P.MenuItem { icon.name: "zoom-in"; text: "Bigger\tCtrl+="; onTriggered: zoomBy(1) }
            P.MenuItem { icon.name: "zoom-out"; text: "Smaller\tCtrl+-"; onTriggered: zoomBy(-1) }
            P.MenuItem { icon.name: "zoom-original"; text: "100%\tCtrl+0"; enabled: win.fontSize > 0; onTriggered: win.fontSize = 0 }
        }
        P.MenuItem { icon.name: "document-edit"; text: "Edit Session…"; onTriggered: prefs.showProfile(paneOf(contextView).profileName) }
        P.MenuItem { icon.name: "configure"; text: "Preferences…\tCtrl+,"; onTriggered: prefs.showPage("") }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "view-close"; text: "Close Pane\tCtrl+Alt+W"; onTriggered: requestClosePane(paneOf(contextView)) }
        P.MenuItem { icon.name: "tab-close"; text: "Close Tab\tCtrl+Shift+W"; onTriggered: { const t = tabOf(paneOf(contextView)); if (t) closeTab(t) } }
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
    readonly property bool showTabs: settings.always_show_tabs || tabs.length > 1
    readonly property real barHeight: 34

    // The tab row (with the + button and the drop target for dragged tabs),
    // used in the top bar or, with tabs at the bottom, below the terminal.
    Component {
        id: tabRowComp
        Item {
            id: box
            property bool flipped: false
            property real dropX: -1   // where a dragged tab would land

            // More tabs than fit at their narrowest: the strip scrolls (the
            // wheel, and to keep the current tab in sight), and a ⌄ after it
            // lists them all.
            readonly property bool overflow: tabs.length * 90 > box.width - 70
            Item {
                id: tabView
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: box.overflow ? Math.max(90, box.width - 70 - 30) : tabRow.width
                clip: box.overflow
                property real scroll: 0
                readonly property real maxScroll: Math.max(0, tabRow.width - width)
                function scrollTo(v) { scroll = Math.max(0, Math.min(maxScroll, v)) }
                // (The current tab in sight.)
                function showCurrent() {
                    const a = currentTab * tabRow.step, b = a + tabRow.tabWidth
                    if (a < scroll) scrollTo(a)
                    else if (b > scroll + width) scrollTo(b - width)
                }
                onMaxScrollChanged: scrollTo(scroll)
                onWidthChanged: Qt.callLater(showCurrent)
                Connections { target: win; function onCurrentTabChanged() { Qt.callLater(tabView.showCurrent) } }
                Behavior on scroll { NumberAnimation { duration: 140; easing.type: Easing.OutCubic } }
                WheelHandler {
                    enabled: box.overflow
                    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
                    onWheel: (ev) => tabView.scrollTo(tabView.scroll - (ev.angleDelta.y !== 0 ? ev.angleDelta.y : ev.angleDelta.x) * 0.8)
                }
            // Tabs are placed by slot (not a Row) so a dragged tab can follow
            // the pointer while the others slide out of its way.
            Item {
                id: tabRow
                x: -tabView.scroll
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: tabs.length > 0 ? (tabs.length - 1) * step + tabWidth : 0
                readonly property real overlap: 0
                readonly property real available: box.overflow ? tabView.width : box.width - 70
                readonly property real tabWidth: settings.stretch_tabs
                    ? Math.max(90, available / Math.max(1, tabs.length))
                    : Math.max(90, Math.min(200, available / Math.max(1, tabs.length)))
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
                        height: tabRow.height
                        y: 0
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
                        view: modelData.lastPane ? modelData.lastPane.viewMode : ""
                        activity: modelData.activity
                        bell: modelData.bell
                        tabColor: modelData.tabColor
                        closeAlways: settings.tab_close_buttons
                        flipped: box.flipped
                        surface: Theme.bg
                        tabItem: modelData
                        payload: function() { return win.tabPayload(modelData) }
                        onClicked: focusTab(index)
                        onCloseClicked: closeTab(modelData)
                        onMenuRequested: (x, y) => { win.menuTab = modelData; win.openMenuAt(tabMenu, tb, x, y) }
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
            }
            // Every tab, to go to one, when they don't all fit.
            Rectangle {
                id: overflowButton
                objectName: "tabOverflow"
                function openMenu() { overflowMenu.popup(overflowButton, 0, overflowButton.height + 2) }
                visible: box.overflow
                anchors.left: tabView.right
                anchors.leftMargin: 4
                anchors.verticalCenter: parent.verticalCenter
                width: visible ? 24 : 0
                height: 22
                radius: 5
                color: ovHover.hovered || overflowMenu.opened ? Theme.mix(Theme.chrome, Theme.fg, 0.12) : "transparent"
                Text {
                    anchors.centerIn: parent
                    anchors.verticalCenterOffset: 0.001
                    text: "\u2304"
                    color: ovHover.hovered ? Theme.text : Theme.textDim
                    font.pixelSize: 14
                    font.bold: true
                    renderType: Text.NativeRendering
                }
                HoverHandler { id: ovHover; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: overflowMenu.popup(overflowButton, 0, overflowButton.height + 2) }
                ToolTip.visible: ovHover.hovered && !overflowMenu.opened
                ToolTip.delay: 500
                ToolTip.text: "All " + tabs.length + " tabs"
            }
            SlimMenu {
                id: overflowMenu
                Repeater {
                    model: tabs
                    delegate: SlimMenuItem {
                        required property var modelData
                        required property int index
                        text: (index + 1) + "   " + ((modelData.lastPane && modelData.lastPane.viewMode === "notebook") ? "\u25a4  " : ">_  ") + modelData.tabTitle
                        checkable: true
                        checked: index === currentTab
                        onTriggered: focusTab(index)
                    }
                }
            }
            // New tab: + opens the default profile; the chevron offers the
            // other profiles, installed shells and ssh hosts.
            SplitButton {
                id: plus
                anchors.left: box.overflow ? overflowButton.right : tabView.right
                anchors.leftMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                mainTip: "New Tab (Ctrl+Shift+T)"
                menuTip: "Profiles, shells and SSH"
                menuOpen: win.launchOpen
                onMainClicked: newTab({})
                onMenuClicked: launchMenu.open(plus.menuAnchor)
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
                    return Math.max(0, Math.min(tabs.length, Math.round((x - tabView.x - tabRow.x) / tabRow.step)))
                }
                onEntered: (drag) => drag.accept(drag.source ? Qt.CopyAction : Qt.MoveAction)
                onPositionChanged: (drag) => { box.dropX = tabView.x + tabRow.x + slot(drag.x) * tabRow.step + 3 }
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

            // A hairline along the bottom; the current tab breaks through it.
            Rectangle {
                visible: showTabs && !tabsAtBottom
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: 1
                color: Theme.divider
                opacity: 0.8
            }
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
            BarButton {
                id: menuButton
                anchors.left: parent.left
                anchors.leftMargin: win.macOS ? 78 : 6
                anchors.verticalCenter: parent.verticalCenter
                icon: "menu"
                tip: "Menu"
                open: win.appMenuOpen
                onClicked: appMenu.open(menuButton)
            }

            Loader {
                active: showTabs && !tabsAtBottom
                anchors.left: menuButton.right
                anchors.leftMargin: 2
                anchors.right: zoomChip.visible ? zoomChip.left : aiButton.left
                anchors.rightMargin: 6
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                sourceComponent: tabRowComp
            }

            // The AI assistant.
            BarButton {
                id: aiButton
                anchors.right: controls.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                icon: "spark"
                tip: "Assistant (Ctrl+Shift+I)"
                open: win.assistantOpen
                onClicked: win.toggleAssistant()
            }

            // The zoom, while it isn't 100%: click to go back (Ctrl+0).
            Rectangle {
                id: zoomChip
                visible: win.fontSize > 0
                anchors.right: aiButton.left
                anchors.rightMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                width: zoomText.implicitWidth + 14
                height: 20
                radius: 5
                color: zoomHover.hovered ? Theme.mix(Theme.chrome, Theme.fg, 0.12) : Theme.mix(Theme.chrome, Theme.fg, 0.05)
                border.width: 1
                border.color: Theme.mix(Theme.chrome, Theme.fg, 0.14)
                Text {
                    id: zoomText
                    anchors.centerIn: parent
                    anchors.verticalCenterOffset: 0.001
                    text: win.zoomPercent + "%"
                    color: zoomHover.hovered ? Theme.text : Theme.textDim
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                    font.features: { "tnum": 1 }
                }
                HoverHandler { id: zoomHover }
                TapHandler { onTapped: win.fontSize = 0 }
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: zoomMenu.open(zoomChip) }
                ToolTip.visible: zoomHover.hovered && !win.zoomMenuOpen
                ToolTip.text: "Text size " + win.zoomPercent + "% — click for 100% (Ctrl+0)"
                ToolTip.delay: 500
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

        RowLayout {
            Layout.row: 1
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0
            Item {
                id: stack
                Layout.fillWidth: true
                Layout.fillHeight: true
            }
            // Drag to resize the assistant.
            Item {
                visible: win.assistantOpen
                Layout.fillHeight: true
                Layout.preferredWidth: 4
                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.SplitHCursor
                    property real startX: 0
                    property real startW: 0
                    onPressed: (m) => { startX = mapToItem(null, m.x, 0).x; startW = win.assistantWidth }
                    onPositionChanged: (m) => { if (pressed) win.assistantWidth = Math.max(280, Math.min(win.width * 0.7, startW - (mapToItem(null, m.x, 0).x - startX))) }
                }
            }
            AssistantPanel {
                id: assistant
                visible: win.assistantOpen
                Layout.fillHeight: true
                Layout.preferredWidth: win.assistantWidth
                controller: controller
                pane: win.activePane
                onCloseRequested: win.toggleAssistant()
            }
        }

        // Tabs below the terminal (Appearance > Tab bar location).
        Rectangle {
            Layout.row: 2
            Layout.fillWidth: true
            Layout.preferredHeight: showTabs && tabsAtBottom ? 34 : 0
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
            const ids = JSON.parse(controller.restorableSessions(""))
            for (const id of ids) alive[id] = true
            if (settings.restore_sessions) {
                let layout = null
                try { layout = JSON.parse(controller.lastLayout()) } catch (e) {}
                if (layout && layout.tabs) {
                    // Live shells reattach; gone ones come back with their saved screens.
                    restoreArrangement(layout, alive, true)
                    // (Kept within the screen's usable area: a window taller than
                    // the space above a panel hides its bottom, the editor, under it.)
                    if (layout.width > 200) {
                        const availW = Screen.desktopAvailableWidth > 0 ? Screen.desktopAvailableWidth : Screen.width
                        const availH = Screen.desktopAvailableHeight > 0 && Screen.desktopAvailableHeight < Screen.height ? Screen.desktopAvailableHeight : Screen.height - 56
                        win.width = Math.min(layout.width, availW)
                        win.height = Math.min(layout.height, availH)
                    }
                    // X11 lets windows place themselves (Wayland: the desktop does, above).
                    if (Qt.platform.pluginName === "xcb" && layout.x !== undefined) { win.x = layout.x; win.y = layout.y }
                    if (layout.maximized) Qt.callLater(win.showMaximized)
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
        if (controller.testEnv("ZOOM") !== "") zoomTestTimer.start()
        if (controller.testEnv("CONTEXT") !== "") contextTestTimer.start()
        if (controller.testEnv("COMPOSE") !== "") composeTestTimer.start()
        if (controller.testEnv("ASSISTANT") !== "") assistantTestTimer.start()
        if (controller.testEnv("RICH") !== "") richTestTimer.start()
        if (controller.testEnv("NOTEBOOK") !== "") notebookTestTimer.start()
        if (controller.testEnv("TAB") !== "") tabTestTimer.start()
        if (controller.testEnv("DRAG") !== "") dragTestTimer.start()
        if (controller.testEnv("FLICKER") !== "") flickerTestTimer.start()
        if (controller.openDoc() !== "") Qt.callLater(openNotebook, controller.openDoc())
        if (controller.testEnv("KEYS") !== "") keysTestTimer.start()
        // (Test hook: TESSERA_TEST_HOST=<ssh host> opens a tab there.)
        if (controller.testEnv("HOST") !== "") newTab({ host: controller.testEnv("HOST"), title: controller.testEnv("HOST") })
        if (controller.daemonState() === "outdated") daemonTimer.start()
        controller.aiAutodetect()
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
            win.saveLayoutNow()
            win.saveSnapshots()
            // (The window's whole content for Preferences, so open popups show too.)
            const target = controller.prefsPage() !== "" ? prefs.contentItem : rootLayout
            const ok = target.grabToImage(function(r) {
                r.saveToFile(controller.screenshotPath())
                Qt.quit()
            })
            if (!ok) Qt.quit()
        }
    }
}
