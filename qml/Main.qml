import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import QtQuick.Window
import Qt.labs.platform as P
import Tessera
import Tessera.Native
import GlobalMenu

ApplicationWindow {
    id: win
    // Shown again by the desktop (include/bootstrap.h): browser views redraw.
    signal reexposed()
    // (The size it was closed at, asked for before it's first shown: a
    // desktop that doesn't restore windows itself still gets it; one that
    // places windows itself, a tiling one, may not follow it.)
    readonly property var savedSize: {
        // (Not a New Window: that's the profile's columns × rows.)
        if (!controller.restoreAllowed()) return null
        let l = null
        try { l = JSON.parse(controller.lastLayout()) } catch (e) {}
        if (!l || !(l.width > 200) || !(l.height > 100)) return null
        const availW = Screen.desktopAvailableWidth > 0 ? Screen.desktopAvailableWidth : 1e9
        const availH = Screen.desktopAvailableHeight > 0 ? Screen.desktopAvailableHeight : 1e9
        return { w: Math.min(l.width, availW), h: Math.min(l.height, availH), max: !!l.maximized }
    }
    width: savedSize ? savedSize.w : 1100
    height: savedSize ? savedSize.h : 700
    property bool startMaximized: !!(savedSize && savedSize.max)
    // Shown from C++ once named, so the desktop can put it back where it was.
    visible: false
    objectName: "tessera-main"
    property bool showAtStart: true
    color: useTransparency && activeProfile.transparency > 0 ? "transparent" : Theme.bg
    // A unified title bar: the tab strip *is* the title bar. The window has
    // no system frame; the bar draws KDE Breeze buttons on the right and
    // moves the window.
    palette.toolTipBase: Theme.panel
    palette.toolTipText: Theme.text
    palette.highlight: Theme.mix(Theme.panel, Theme.accent, 0.35)
    palette.highlightedText: Theme.text
    palette.light: Theme.mix(Theme.panel, Theme.accent, 0.22)
    palette.midlight: Theme.mix(Theme.panel, Theme.accent, 0.35)
    flags: Qt.Window | Qt.FramelessWindowHint
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
    property var commandHistory: []    // (filled at startup)
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
    // Ctrl tapped on its own (include/bootstrap.h TesseraCtrlTap): over to
    // the active pane's terminal from its input editor, else to the editor
    // (when it's up: at a prompt).
    // A tap of Ctrl on its own (include/bootstrap.h): right Ctrl runs the
    // editor's command in the background (Ctrl+Enter's), and with nothing to
    // run does nothing; left Ctrl switches between the editor and the terminal.
    function ctrlTapped(right) {
        const p = activePane
        // (A program in the terminal reading lone modifier keys itself,
        // tessh say: the tap was its.)
        // (Left at a prompt, though, is always the switch: tessh's own
        // editor takes every key but leaves left Ctrl alone.)
        if (p && p.view.activeFocus && p.view.reportsAllKeys() && (right || !p.editorOn)) return
        if (right) { if (p && p.editorOn && p.compose.editing && p.compose.text.trim() !== "") p.compose.submitBackground(); return }
        toggleEditor()
    }
    function toggleEditor() {
        const p = activePane
        if (!p) return
        // (A command running: the editor called up to type the next one,
        // which waits for it, queued.)
        if (!p.editorOn && p.inputEditor && !p.view.exited && !p.takenOver) { p.editorCalled = true; Qt.callLater(backToEditor); return }
        if (!p.editorOn) return
        if (p.compose.editing) { p.compose.handBack(); return }
        backToEditor()
    }
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
            // (Kept across restarts with the layout.)
            onInputHiddenChanged: if (win.startupDone) layoutSaver.restart()
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
            notebookKernel: win.settings.notebook_kernel !== undefined ? win.settings.notebook_kernel : "tessh"
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
            // A page instead of panes ("guide:LANG": a language's guide;
            // "tessh-guide", the tessh guide, from before).
            property string page: ""
            readonly property var pageItem: pageLoader.item
            // The pane the title follows: the last focused, else the first.
            readonly property var titleView: { win.treeRev; if (lastView) return lastView; const ps = win.panesIn(rootSplit, []); return ps.length ? ps[0].view : null }
            readonly property var lastPane: titleView ? win.paneOf(titleView) : null
            readonly property var prof: win.profileFor(profile)
            readonly property string tabTitle: customTitle !== "" ? customTitle
                : prof.tab_title !== "" ? prof.tab_title
                // What's running in it (vim, htop, ssh...); at a prompt, the
                // shell's own title or the folder.
                : lastPane && lastPane.docPath ? lastPane.docPath.replace(/^.*\//, "") + (lastPane.docDirty ? " \u2022" : "")
                : lastPane && lastPane.job !== "" ? lastPane.job
                // (At a prompt, the folder, the same whichever shell: shells'
                // own titles differ, fish's shortens the path.)
                : (win.settings.tab_titles_per_tab && titleView && titleView.title && !(lastPane && lastPane.editorOn) ? titleView.title
                   : launchTitle !== "" ? launchTitle
                   : win.folderName(titleView && titleView.pwd ? titleView.pwd : (lastPane ? (lastPane.procCwd || lastPane.cwd) : ""))
                     || (lastPane && lastPane.shellName) || prof.name)
            PaneSplit {
                id: rootSplit
                anchors.fill: parent
                isRoot: true
                tab: tabItem
                visible: tabItem.page === ""
            }
            // (A tab in the background: its split doesn't lay out its panes
            // (SplitView skips what isn't visible), so they're sized here, to
            // the tab as it is now, sharing it as they were. Shown, it's
            // ready to draw at once, with nothing laid out again.)
            // (Just restored, shown or not: its panes keep their saved shares
            // of the tab as the window comes to its size.)
            onWidthChanged: if (!visible || restoring) hiddenFit.restart()
            onHeightChanged: if (!visible || restoring) hiddenFit.restart()
            property bool restoring: false
            Timer { id: hiddenFit; interval: 150; onTriggered: if (!tabItem.visible || tabItem.restoring) win.fitTree(rootSplit, tabItem.width, tabItem.height) }
            Timer { id: restored; interval: 3000; onTriggered: tabItem.restoring = false }
            function fitSoon() { restoring = true; restored.restart(); hiddenFit.restart() }
            Loader {
                id: pageLoader
                anchors.fill: parent
                active: tabItem.page === "tessh-guide" || tabItem.page.startsWith("guide:")
                sourceComponent: GuidePage {
                    backend: controller
                    // ("guide:LANG", or "guide:LANG/GUIDE" for one of a language's guides.)
                    lang: tabItem.page.startsWith("guide:") ? tabItem.page.slice(6).split("/")[0] : "tessh"
                    guideId: tabItem.page.startsWith("guide:") ? (tabItem.page.slice(6).split("/")[1] || "") : ""
                    fallbackTitle: win.guides[lang] || (lang + " guide")
                    onChose: (id, title) => { tabItem.page = "guide:" + lang + "/" + id; tabItem.customTitle = title }
                    onOpenOther: (target, query) => win.openGuide(target, query)
                    ansi: win.themeFor(tabItem.prof).ansi || []
                    fontFamily: tabItem.prof.font_family || "monospace"
                }
            }
        }
    }

    // The languages with a guide ({lang: title}): tessh's, built in, and
    // the others' own documentation, fetched the first time (guides.rs).
    readonly property var guides: JSON.parse(controller.guideList() || "{}")
    // The guide for an editor language ("" for none); the shell's own for
    // shell commands (zsh, fish...), when it has one.
    function guideFor(lang, shell) {
        if (lang === "bash" && shell && guides[shell]) return shell
        return guides[lang] ? lang : ""
    }
    // A language's guide, a tab of its own (the one there is, if it's open).
    // `query`: find it in the guide once it's there (Shift+F1, the chip).
    function openGuide(lang, query) {
        lang = lang || "tessh"
        // ("LANG/GUIDE": one of the language's guides.)
        const own = lang.indexOf("/") > 0 ? JSON.parse(controller.guideChoices(lang.split("/")[0]) || "[]").find(c => c[0] === lang.split("/")[1]) : null
        const page = "guide:" + lang
        for (let i = 0; i < tabs.length; ++i) if (tabs[i].page === page || tabs[i].page.startsWith(page + "/") || (lang === "tessh" && tabs[i].page === "tessh-guide")) {
            currentTab = i
            if (query && tabs[i].pageItem) tabs[i].pageItem.find(query)
            return
        }
        const t = tabComp.createObject(stack, { profile: profileFor(settings.default_profile).name, page: page, customTitle: own ? own[1] : guides[lang] || (lang + " guide") })
        tabs.splice(tabs.length, 0, t)
        tabs = tabs.slice()
        currentTab = tabs.length - 1
        treeRev++
        Qt.callLater(() => { if (t.pageItem) { if (query) t.pageItem.find(query); else t.pageItem.focusFind() } })
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
        showMenu(menu, item, x, y)
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
        // (A pane, not its notebook, whose view is the same terminal: the
        // terminal sits in a notebook card while a full-screen program runs.)
        while (it && !(it.view === view && it.compose !== undefined)) it = it.parent
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
        const props = { profileName: prof.name, cwd: cwdFor(prof, p.view), sshHost: p.sshHost, viewMode: p.viewMode, inputHidden: p.inputHidden }
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
        if (opts.layout && opts.layout.page) {
            // (A page's tab: the page again, no panes.)
            t.page = opts.layout.page
            t.customTitle = opts.layout.title || ""
        } else if (opts.layout) {
            buildSavedTree(t, opts.layout.root, opts.alive, !!opts.recreate)
            t.fitSoon()
            if (opts.layout.title) t.customTitle = opts.layout.title
            if (opts.layout.color) t.tabColor = opts.layout.color
            if (t.root.count === 0) { t.destroy(); return null }
        } else {
            // (At the tab's size from the start: no frame at a default size
            // before it's laid out.)
            const props = { profileName: prof.name, width: stack.width, height: stack.height }
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
            // (The input box put away in the tab it's opened from: away here too.)
            const src = active ? paneOf(active) : null
            if (src && src.inputHidden) props.inputHidden = true
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
        const busy = [], asks = []
        let always = false
        for (const p of panes) {
            if (p.view.exited) continue
            const prof = profileFor(p.profileName)
            if (prof.confirm_close === "always") { always = true; continue }
            if (prof.confirm_close !== "jobs") continue
            asks.push({ id: p.view.sessionId, ignored: prof.ignored_jobs.split(",").map(s => s.trim()).filter(s => s) })
        }
        const decide = () => {
            if (busy.length > 0)
                confirm("Close", "“" + busy.join("”, “") + "” " + (busy.length > 1 ? "are" : "is") + " still running. Close anyway?", doIt)
            else if (always) confirm("Close", "Close this session?", doIt)
            else doIt()
        }
        if (asks.length === 0) { decide(); return }
        // (Every pane's job asked at once; decided when the last answers.)
        let left = asks.length
        for (const a of asks)
            Async.call(controller, k => controller.sessionJob(k, a.id), job => {
                if (job !== "" && a.ignored.indexOf(job) < 0) busy.push(job)
                if (--left === 0) decide()
            })
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

    // The prompt bars' shared memory (PromptBar.cache()).
    property var promptCache: ({})

    // ---- Nix missing: what it is and how to install it -------------------------
    // (what: the program that needed it; host: where, "" for here.)
    function nixHelp(what, host) { if (!nixHelpBox.opened) nixHelpBox.show(what, host) }
    // (The same as text, for a terminal: an ssh host's shell, an editor's.)
    function noNixText(what) {
        return (what ? what + " needs Nix" : "This needs Nix") + ", which isn't installed here. Nix fetches the programs Tessera uses that you don't have yet, "
             + "kept apart from the system's own packages. To install it, run:\n\n  " + nixHelpBox.installCmd + "\n\nthen open a new tab. More: https://nixos.org/download/"
    }
    // (Test hook: TESSERA_TEST_CLOSETAB=<dir>: three tabs, the middle one
    // current, closed; each frame after it saved to <dir>/f-N.png.)
    property int closeTabFrames: -1
    Timer { interval: 3000; running: controller.testEnv("CLOSETAB") !== ""; onTriggered: { const v = controller.testEnv("CLOSETAB_VIEW"); newTab({ view: v }); win.later(1000, () => newTab({ view: v })); win.later(2000, () => focusTab(1)) } }
    Timer { interval: 7000; running: controller.testEnv("CLOSETAB") !== ""; onTriggered: { win.closeTabFrames = 0; rootLayout.grabToImage(r => r.saveToFile(controller.testEnv("CLOSETAB") + "/f-before.png")); console.log("closing at " + Date.now()); win.removeTab(win.tabs[1]) } }
    // (TESSERA_TEST_NEWTAB=<dir>: a new tab at 6 s, each frame after it saved.)
    Timer { interval: 6000; running: controller.testEnv("NEWTAB") !== ""; onTriggered: { win.closeTabFrames = 0; newTab({ view: controller.testEnv("CLOSETAB_VIEW") }) } }
    Connections { target: win; function onFrameSwapped() { if (win.closeTabFrames >= 0 && win.closeTabFrames < 20) { const n = win.closeTabFrames++; rootLayout.grabToImage(r => r.saveToFile((controller.testEnv("CLOSETAB") || controller.testEnv("NEWTAB")) + "/f-" + n + ".png")) } } }
    // (Test hook: TESSERA_TEST_HISTORY=<query>: the history list for it.)
    Timer { interval: 5000; running: controller.testEnv("HISTORY") !== ""; onTriggered: if (activePane) { activePane.compose.setText(controller.testEnv("HISTORY")); activePane.compose.openHistory() } }
    // (Test hook: TESSERA_TEST_NIXHELP=<program> shows it.)
    Timer { interval: 3000; running: controller.testEnv("NIXHELP") !== ""; onTriggered: win.nixHelp(controller.testEnv("NIXHELP"), "") }
    NixHelp {
        id: nixHelpBox
        onInstallRequested: (cmd, host) => newTab({ command: cmd + "; exec \"${SHELL:-sh}\" -l", host: host, view: "terminal", title: "Installing Nix" })
    }

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
        const path = dir + "/Tessera " + fileStamp() + ".png"
        // (Grabbed once the folder's there.)
        Async.call(controller, k => controller.fileAction(k, "mkdirs", dir, ""), () => {
            if (!item) return
            item.grabToImage(r => {
                if (!r.saveToFile(path)) { notice("Couldn't save the screenshot to " + dir, []); return }
                if (activeView) activeView.copyImageFile(path)
                notice("Screenshot saved and copied: " + path.replace(controller.homeDir(), "~"), [
                    { label: "Open", run: () => Async.call(controller, k => controller.fileAction(k, "open", path, ""), null) },
                    { label: "Show in folder", run: () => Async.call(controller, k => controller.fileAction(k, "reveal", path, ""), null) }])
            })
        })
    }
    function toggleRecording(view) {
        if (!view) return
        if (view.recording) {
            const path = view.stopRecording()
            if (path === "") return
            notice("Recording saved: " + path.replace(controller.homeDir(), "~"), [
                { label: "Make a GIF", run: () => { Async.call(controller, k => controller.fileAction(k, "gif", path, ""), e => { if (e.startsWith("needs-nix:")) nixHelp("agg", "") }); notice("Making the GIF (agg)… it opens when it's done.", []) } },
                { label: "Show in folder", run: () => Async.call(controller, k => controller.fileAction(k, "reveal", path, ""), null) }])
        } else {
            const dir = controller.homeDir() + "/Videos/Tessera"
            const file = dir + "/Tessera " + fileStamp() + ".cast"
            // (Started once the folder's there.)
            Async.call(controller, k => controller.fileAction(k, "mkdirs", dir, ""), () => {
                if (!view || view.recording) return
                const path = view.startRecording(file)
                notice(path !== "" ? "Recording this pane (the red dot); Session › Stop Recording ends it." : "Couldn't start recording in " + dir, [])
            })
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
    // (ms < 0: it stays until a button or × closes it.)
    function notice(text, actions, ms) { noticeBox.text = text; noticeBox.actions = actions; noticeBox.shown = true; noticeBox.sticky = (ms || 0) < 0; noticeTimer.interval = ms > 0 ? ms : 7000; if (noticeBox.sticky) noticeTimer.stop(); else noticeTimer.restart() }
    Timer { id: noticeTimer; interval: 7000; onTriggered: noticeBox.shown = false }
    Rectangle {
        id: noticeBox
        property string text: ""
        property var actions: []
        property bool shown: false
        property bool sticky: false
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
        HoverHandler { onHoveredChanged: if (hovered) noticeTimer.stop(); else if (!noticeBox.sticky) noticeTimer.restart() }
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
                     mode: n.viewMode, doc: n.docPath, id: withIds ? n.view.sessionId : "", host: n.sshHost, noInput: n.inputHidden ? true : undefined, w: n.width, h: n.height }
        const c = []
        for (let i = 0; i < n.count; ++i) c.push(serializeNode(n.itemAt(i), withIds))
        return { t: n.orientation === Qt.Horizontal ? "h" : "v", c: c, w: n.width, h: n.height }
    }

    function serializeTab(t, withIds) {
        return { title: t.customTitle, color: t.tabColor.a > 0 ? t.tabColor.toString() : "",
                 profile: t.profile, root: serializeNode(t.root, withIds), page: t.page }
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
        // (Each item's share of its split as saved, scaled to the room it has
        // now: a window smaller than the one it was saved from squeezes them
        // all alike, not the last to its minimum.)
        function shares(kids, horiz, room) {
            const want = kids.map(c => Math.max(1, (horiz ? c.w : c.h) || 1))
            const total = want.reduce((a, b) => a + b, 0)
            const free = room - (kids.length - 1)   // (the 1 px handles)
            return want.map(x => Math.max(1, Math.round(free * x / total)))
        }
        function build(n, host, W, H) {
            if (n.t === "pane") {
                const props = { profileName: profileFor(n.profile).name, cwd: n.cwd || "", commandOverride: n.cmd || "", viewMode: n.mode === "notebook" ? "notebook" : "terminal", docPath: n.doc || "", sshHost: n.host || "", inputHidden: !!n.noInput }
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
                p.prefW = W; p.prefH = H
                return p
            }
            const s = splitComp.createObject(null, { orientation: n.t === "h" ? Qt.Horizontal : Qt.Vertical })
            s.host = host
            s.prefW = W; s.prefH = H
            const horiz = n.t === "h"
            const sh = shares(n.c, horiz, horiz ? W : H)
            n.c.forEach((c, i) => {
                const it = build(c, s, horiz ? sh[i] : W, horiz ? H : sh[i])
                if (it) s.addItem(it)
            })
            if (s.count === 0) { s.destroy(); return null }
            if (s.count === 1) { const only = s.takeItem(0); only.host = host; s.destroy(); return only }
            return s
        }
        // (The size it was saved at; the tab's own once it has it: fitSoon.)
        const W = node.w || t.width || 800, H = node.h || t.height || 600
        if (node.t === "pane") {
            const it = build(node, t.root, W, H)
            if (it) t.root.addItem(it)
        } else {
            t.root.orientation = node.t === "h" ? Qt.Horizontal : Qt.Vertical
            const horiz = node.t === "h"
            const sh = shares(node.c, horiz, horiz ? W : H)
            node.c.forEach((c, i) => {
                const it = build(c, t.root, horiz ? sh[i] : W, horiz ? H : sh[i])
                if (it) { it.host = t.root; t.root.addItem(it) }
            })
        }
    }

    // A pane tree sized by hand to w × h, each split's items sharing it in
    // proportion to their preferred sizes (for a tab in the background,
    // whose SplitViews don't lay out).
    function fitTree(it, w, h) {
        if (!it || w <= 0 || h <= 0) return
        if (!it.isRoot) { it.width = w; it.height = h }   // (the root fills its tab already)
        if (it.orientation === undefined || it.itemAt === undefined) return
        const n = it.count
        if (n === 0) return
        const horiz = it.orientation === Qt.Horizontal
        const kids = []
        for (let i = 0; i < n; ++i) kids.push(it.itemAt(i))
        const want = kids.map(c => horiz ? (c.prefW > 0 ? c.prefW : c.width) : (c.prefH > 0 ? c.prefH : c.height))
        const total = want.reduce((a, b) => a + Math.max(1, b), 0)
        const room = (horiz ? w : h) - (n - 1)   // (the 1 px handles)
        let used = 0
        for (let i = 0; i < n; ++i) {
            const share = i === n - 1 ? room - used : Math.round(room * Math.max(1, want[i]) / total)
            used += share
            if (horiz) { if (n > 1) kids[i].prefW = share; fitTree(kids[i], share, h) }
            else { if (n > 1) kids[i].prefH = share; fitTree(kids[i], w, share) }
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
        // (The first free name: each one read in the background, then the next.)
        const tryName = (n) => {
            const path = n === 1 ? dir + "/Untitled.ipynb" : dir + "/Untitled-" + n + ".ipynb"
            if (allPanes().some(x => x.docPath === path)) { tryName(n + 1); return }
            Async.call(controller, k => controller.readTextFile(k, path), r => {
                let taken = false
                try { taken = JSON.parse(r).error === undefined } catch (e) {}
                if (taken) tryName(n + 1)
                else openNotebook(path)
            })
        }
        tryName(1)
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
    Shortcut { sequence: "Ctrl+Shift+H"; onActivated: win.toggleInputBox() }
    function toggleInputBox() { const p = activePane; if (p && p.inputEditor) p.setInputHidden(!p.inputHidden) }
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
    // (A tab is a terminal or a notebook from the start, for good.)
    Shortcut { sequence: "Ctrl+Shift+J"; autoRepeat: false; onActivated: newTab({ view: "notebook" }) }
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

    // Defined once: exported to the desktop's global menu bar (GlobalMenuBar:
    // hearth's top bar, Plasma's Global Menu) and shown by the ☰ button
    // (AppMenu). Shortcuts are only shown there: the Shortcut{}s above do the
    // work (a terminal keeps every other key for its shell). Ids are unique
    // across the whole model (the global menu updates items by id).
    readonly property bool hasSelection: activeView !== null && activeView.hasSelection
    readonly property var tabColors: [["Red", "#e5534b", "folder-red"], ["Orange", "#e8853d", "folder-orange"], ["Yellow", "#d9b43b", "folder-yellow"], ["Green", "#57ab5a", "folder-green"], ["Blue", "#4a90e2", "folder-blue"], ["Purple", "#986ee2", "folder-violet"], ["Gray", "#8b949e", "folder-grey"]]
    readonly property var menuModel: [
        { title: "Tessera", icon: "utilities-terminal", items: [
            { id: "about", icon: "help-about", text: "About Tessera", inWindow: false },
            { separator: true },
            { id: "prefs", icon: "configure", text: "Preferences…", shortcut: "Ctrl+," },
            { id: "restart-daemon", icon: "view-refresh", text: "Restart tesserad…" },
            { separator: true },
            { id: "quit", icon: "application-exit", text: "Quit Tessera", shortcut: "Ctrl+Shift+Q" } ] },
        { title: "Shell", icon: "utilities-terminal", items: [
            { id: "new-tab", icon: "tab-new", text: "New Tab", shortcut: "Ctrl+Shift+T" },
            { id: "new-notebook-tab", icon: "view-list-details", text: "New Notebook Tab", shortcut: "Ctrl+Shift+J" },
            { id: "new-notebook", icon: "document-new", text: "New Jupyter Notebook" },
            { id: "open-notebook", icon: "document-open", text: "Open Jupyter Notebook…", shortcut: "Ctrl+O" },
            { id: "new-tab-profile", icon: "tab-new", text: "New Tab with Current Profile" },
            { id: "new-window", icon: "window-new", text: "New Window", shortcut: "Ctrl+Shift+N" } ]
            .concat(Qt.platform.os === "windows" && !controller.isAdmin()
                ? [{ id: "new-admin-window", icon: "security-high", text: "New Administrator Window" }] : [])
            .concat([
            { id: "duplicate-tab", icon: "tab-duplicate", text: "Duplicate Tab", enabled: tabs.length > 0 },
            { separator: true },
            { id: "split-right-current", icon: "view-split-left-right", text: "Split Right with Current Profile", shortcut: "Ctrl+Shift+D" },
            { id: "split-down-current", icon: "view-split-top-bottom", text: "Split Down with Current Profile", shortcut: "Ctrl+Shift+E" },
            { id: "split-right", icon: "view-split-left-right", text: "Split Right" },
            { id: "split-down", icon: "view-split-top-bottom", text: "Split Down" },
            { separator: true },
            { id: "save-contents", icon: "document-save", text: "Save Contents…", enabled: activeView !== null },
            { id: "save-selection", icon: "document-save-as", text: "Save Selected Text…", enabled: win.hasSelection },
            { separator: true },
            { id: "undo-close", icon: "edit-undo", text: "Undo Close", shortcut: "Ctrl+Shift+Z", enabled: closedStack.length > 0 },
            { id: "close-tab", icon: "tab-close", text: "Close Tab", shortcut: "Ctrl+Shift+W", enabled: tabs.length > 0 },
            { id: "close-pane", icon: "view-close", text: "Close Pane", shortcut: "Ctrl+Alt+W", enabled: activePane !== null },
            { id: "close-all-panes", icon: "tab-close-other", text: "Close All Panes in Tab", enabled: tabs.length > 0 },
            { id: "close-window", icon: "window-close", text: "Close Window", shortcut: "Ctrl+Shift+Q" },
            { separator: true },
            { id: "broadcast", icon: "network-wireless-hotspot", text: "Broadcast Input", items: [
                { id: "broadcast-off", icon: "network-disconnect", text: "Send Input to Current Session Only", checkable: true, checked: broadcastMode === "off" },
                { id: "broadcast-all", icon: "view-grid", text: "Broadcast Input to All Panes in All Tabs", checkable: true, checked: broadcastMode === "all" },
                { id: "broadcast-tab", icon: "object-columns", text: "Broadcast Input to All Panes in Current Tab", checkable: true, checked: broadcastMode === "tab" } ] } ]) },
        { title: "Edit", icon: "edit-entry", items: [
            { id: "copy", icon: "edit-copy", text: "Copy", shortcut: "Ctrl+Shift+C", enabled: win.hasSelection },
            { id: "copy-styles", icon: "format-text-color", text: "Copy with Styles", enabled: win.hasSelection },
            { id: "copy-control", icon: "format-text-code", text: "Copy with Control Sequences", enabled: win.hasSelection },
            { id: "paste", icon: "edit-paste", text: "Paste", shortcut: "Ctrl+Shift+V", enabled: activeView !== null },
            { id: "paste-special", icon: "edit-paste-style", text: "Paste Special", enabled: activeView !== null, items: [
                { id: "paste-selection", icon: "edit-paste-in-place", text: "Paste Selection", shortcut: "Shift+Ins" },
                { id: "paste-slowly", icon: "chronometer", text: "Paste Slowly" },
                { separator: true },
                { id: "warn-multiline", icon: "dialog-warning", text: "Warn Before Multi-Line Paste", checkable: true, checked: !!settings.warn_multiline_paste } ] },
            { id: "paste-history", icon: "view-history", text: "Paste History", enabled: pasteHistory.length > 0 && activeView !== null,
              items: pasteHistory.length > 0 ? pasteHistory.map((s, i) => ({ id: "paste-history-" + i, icon: "text-plain", text: win.shortLabel(s) }))
                                             : [{ id: "paste-history-none", icon: "edit-none", text: "Nothing Pasted Yet", enabled: false }] },
            { id: "snippets", icon: "insert-text", text: "Snippets", items: (settings.snippets || []).map((s, i) => (
                { id: "snippet-" + i, icon: "text-x-script", text: s.title, enabled: activeView !== null })).concat([
                { separator: true },
                { id: "edit-snippets", icon: "configure", text: "Edit Snippets…" } ]) },
            { separator: true },
            { id: "select-all", icon: "edit-select-all", text: "Select All", shortcut: "Ctrl+Shift+A", enabled: activeView !== null },
            { id: "marks", icon: "bookmarks", text: "Marks", enabled: activeView !== null, items: [
                { id: "prev-prompt", icon: "go-up", text: "Previous Prompt", shortcut: "Ctrl+Shift+Up" },
                { id: "next-prompt", icon: "go-down", text: "Next Prompt", shortcut: "Ctrl+Shift+Down" },
                { separator: true },
                { id: "select-last-output", icon: "edit-select", text: "Select Output of Last Command", shortcut: "Ctrl+Shift+L" } ] },
            { separator: true },
            { id: "find-menu", icon: "edit-find", text: "Find", items: [
                { id: "find", icon: "edit-find", text: "Find…", shortcut: "Ctrl+Shift+F" },
                { id: "find-next", icon: "go-up", text: "Find Next", shortcut: "F3" },
                { id: "find-prev", icon: "go-down", text: "Find Previous", shortcut: "Shift+F3" },
                { separator: true },
                { id: "find-urls", icon: "link", text: "Find URLs" } ] },
            { separator: true },
            { id: "clear-buffer", icon: "edit-clear-history", text: "Clear Buffer", shortcut: "Ctrl+Shift+K", enabled: activeView !== null },
            { id: "clear-scrollback", icon: "edit-clear-history", text: "Clear Scrollback Buffer", enabled: activeView !== null } ] },
        { title: "View", icon: "view-visible", items: [
            { id: "fullscreen", icon: "view-fullscreen", text: "Full Screen", shortcut: "F11", checkable: true, checked: win.visibility === Window.FullScreen },
            { id: "assistant", icon: "help-hint", text: "Show Assistant", shortcut: "Ctrl+Shift+I", checkable: true, checked: win.assistantOpen },
            { id: "input-box", icon: "edit-entry", text: "Show Input Box", shortcut: "Ctrl+Shift+H", checkable: true,
              checked: activePane !== null && !activePane.inputHidden, enabled: activePane !== null && activePane.inputEditor },
            { separator: true },
            { id: "zoom-in", icon: "zoom-in", text: "Make Text Bigger", shortcut: "Ctrl+=" },
            { id: "zoom-out", icon: "zoom-out", text: "Make Text Smaller", shortcut: "Ctrl+-" },
            { id: "zoom-reset", icon: "zoom-original", text: "Make Text Normal Size", shortcut: "Ctrl+0", enabled: fontSize > 0 },
            { separator: true },
            { id: "maximize-pane", icon: "zoom-fit-best", text: "Maximize Active Pane", shortcut: "Ctrl+Shift+Return", checkable: true,
              checked: maximizedPane !== null, enabled: maximizedPane !== null || (activePane !== null && activePane.split) },
            { id: "transparency", icon: "edit-opacity", text: "Use Transparency", checkable: true, checked: useTransparency, enabled: activeProfile.transparency > 0 },
            { separator: true },
            { id: "find-cursor", icon: "crosshairs", text: "Find Cursor", shortcut: "Ctrl+Shift+/", enabled: activeView !== null },
            { id: "cursor-guide", icon: "draw-line", text: "Show Cursor Guide", checkable: true, checked: cursorGuide },
            { separator: true },
            { id: "tab-color", icon: "color-management", text: "Tab Color", enabled: tabs.length > 0, items: [
                { id: "tab-color-none", icon: "edit-clear", text: "No Color" },
                { separator: true } ].concat(win.tabColors.map((c, i) => ({ id: "tab-color-" + i, icon: c[2], text: c[0] }))) } ] },
        { title: "Session", icon: "system-run", items: [
            { id: "edit-session", icon: "document-edit", text: "Edit Session…" },
            { id: "restart-session", icon: "view-refresh", text: "Restart Session", enabled: activeView !== null },
            { id: "duplicate-session", icon: "tab-duplicate", text: "Duplicate Session", enabled: activeView !== null },
            { id: "reset", icon: "edit-clear", text: "Reset", enabled: activeView !== null },
            { separator: true },
            { id: "log", icon: "text-x-log", text: "Log", enabled: activeView !== null, items: [
                { id: "log-start", icon: "media-record", text: "Log to File…", enabled: activeView !== null && !activeView.logging },
                { id: "log-stop", icon: "media-playback-stop", text: "Stop Logging", enabled: activeView !== null && activeView.logging } ] },
            { id: "record", icon: activeView && activeView.recording ? "media-playback-stop" : "media-record",
              text: activeView && activeView.recording ? "Stop Recording" : "Record Pane (asciicast)", enabled: activeView !== null },
            { id: "shot-pane", icon: "camera-photo", text: "Screenshot of Pane", enabled: activeView !== null },
            { id: "shot-window", icon: "preferences-system-windows-effect-screenshot", text: "Screenshot of Window" } ] },
        { title: "Profiles", icon: "bookmarks", items: profileList.map((p, i) => (
            { id: "profile-" + i, icon: p.name === settings.default_profile ? "favorite" : "utilities-terminal", text: p.name + (p.name === settings.default_profile ? "  (default)" : "") })).concat([
            { separator: true },
            { id: "open-all-profiles", icon: "tab-new", text: "Open All" },
            { id: "edit-profiles", icon: "configure", text: "Edit Profiles…" } ]) },
        { title: "Window", icon: "window", items: [
            { id: "minimize", icon: "window-minimize", text: "Minimize" },
            { id: "zoom-window", icon: "window-maximize", text: "Zoom", checkable: true, checked: win.visibility === Window.Maximized },
            { separator: true },
            { id: "window-title", icon: "edit-rename", text: "Edit Window Title…" },
            { id: "tab-title", icon: "edit-rename", text: "Edit Tab Title…", enabled: tabs.length > 0 },
            { separator: true },
            { id: "next-tab", icon: "go-next", text: "Select Next Tab", shortcut: "Ctrl+PgDown", enabled: tabs.length > 1 },
            { id: "prev-tab", icon: "go-previous", text: "Select Previous Tab", shortcut: "Ctrl+PgUp", enabled: tabs.length > 1 },
            { id: "move-tab-left", icon: "arrow-left", text: "Move Tab Left", shortcut: "Ctrl+Shift+PgUp", enabled: currentTab > 0 },
            { id: "move-tab-right", icon: "arrow-right", text: "Move Tab Right", shortcut: "Ctrl+Shift+PgDown", enabled: currentTab < tabs.length - 1 },
            { id: "select-pane", icon: "view-split-left-right", text: "Select Split Pane", items: [
                { id: "pane-up", icon: "go-up", text: "Select Pane Above", shortcut: "Ctrl+Alt+Up" },
                { id: "pane-down", icon: "go-down", text: "Select Pane Below", shortcut: "Ctrl+Alt+Down" },
                { id: "pane-left", icon: "go-previous", text: "Select Pane Left", shortcut: "Ctrl+Alt+Left" },
                { id: "pane-right", icon: "go-next", text: "Select Pane Right", shortcut: "Ctrl+Alt+Right" },
                { separator: true },
                { id: "pane-next", icon: "go-next-view", text: "Next Pane", shortcut: "Ctrl+Shift+]" },
                { id: "pane-prev", icon: "go-previous-view", text: "Previous Pane", shortcut: "Ctrl+Shift+[" } ] },
            { separator: true },
            { id: "arrangements", icon: "view-group", text: "Arrangements", items: [
                { id: "save-arrangement", icon: "document-save", text: "Save Window Arrangement…" },
                { separator: true } ].concat(arrangementNames.map((n, i) => (
                { id: "arrangement-" + i, icon: "document-open", text: "Restore “" + n + "”" }))).concat([
                { separator: true },
                { id: "manage-arrangements", icon: "configure", text: "Manage Arrangements…" } ]) } ] },
        { title: "Help", icon: "help-contents", items: [
            { id: "help", icon: "help-contents", text: "Tessera Help" },
            { id: "about-help", icon: "help-about", text: "About Tessera" } ] }
    ]

    function menuAction(id) {
        const v = activeView
        const num = (prefix) => Number(id.slice(prefix.length))
        if (id.startsWith("paste-history-")) { const s = pasteHistory[num("paste-history-")]; if (s !== undefined) pasteInto(v, s); return }
        if (id.startsWith("snippet-")) { const s = (settings.snippets || [])[num("snippet-")]; if (s && v) v.sendText(win.unescapeText(s.text)); return }
        if (id.startsWith("tab-color-") && id !== "tab-color-none") { const c = tabColors[num("tab-color-")]; if (c && tabs[currentTab]) tabs[currentTab].tabColor = c[1]; return }
        if (id.startsWith("profile-")) { const p = profileList[num("profile-")]; if (p) newTab({ profile: p.name }); return }
        if (id.startsWith("arrangement-")) { const n = arrangementNames[num("arrangement-")]; if (n !== undefined) restoreArrangement(arrangementMap[n], null); return }
        switch (id) {
        case "about": case "about-help": prefs.showPage("about"); break
        case "prefs": prefs.showPage(""); break
        case "restart-daemon": win.askRestartDaemon(); break
        case "quit": case "close-window": win.close(); break
        case "new-tab": newTab({}); break
        case "new-notebook-tab": newTab({ view: "notebook" }); break
        case "new-notebook": newNotebook(); break
        case "open-notebook": notebookDialog.open(); break
        case "new-tab-profile": newTab({ profile: activeProfile.name }); break
        case "new-window": controller.newWindow(); break
        case "new-admin-window": controller.newAdminWindow(); break
        case "duplicate-tab": if (tabs.length > 0) newTab({ layout: serializeTab(tabs[currentTab], false), profile: tabs[currentTab].profile }); break
        case "split-right-current": splitPane(activePane, Qt.Horizontal); break
        case "split-down-current": splitPane(activePane, Qt.Vertical); break
        case "split-right": splitPane(activePane, Qt.Horizontal, settings.default_profile); break
        case "split-down": splitPane(activePane, Qt.Vertical, settings.default_profile); break
        case "save-contents": if (v) saveContentsTo(v.contentsText(), "Save Contents"); break
        case "save-selection": if (v && v.hasSelection) saveContentsTo(v.selectedText(), "Save Selected Text"); break
        case "undo-close": undoClose(); break
        case "close-tab": case "close-all-panes": if (tabs.length > 0) closeTab(tabs[currentTab]); break
        case "close-pane": closeActivePane(); break
        case "broadcast-off": broadcastMode = "off"; break
        case "broadcast-all": broadcastMode = "all"; break
        case "broadcast-tab": broadcastMode = "tab"; break
        case "copy": if (v) v.copy(); break
        case "copy-styles": if (v) v.copyWithStyles(); break
        case "copy-control": if (v) v.copyWithControlSequences(); break
        case "paste": pasteClipboard(v); break
        case "paste-selection": if (v) v.pasteSelection(); break
        case "paste-slowly": if (v) v.pasteSlowly(); break
        case "warn-multiline": setSetting("warn_multiline_paste", !settings.warn_multiline_paste); break
        case "edit-snippets": prefs.showPage("snippets"); break
        case "select-all": if (v) v.selectAll(); break
        case "prev-prompt": if (v) v.jumpToPrompt(-1); break
        case "next-prompt": if (v) v.jumpToPrompt(1); break
        case "select-last-output": if (v) v.selectLastOutput(); break
        case "find": openFind(false); break
        case "find-next": findStep(-1); break
        case "find-prev": findStep(1); break
        case "find-urls": openFind(true); break
        case "clear-buffer": if (v) v.clearScrollback(); break
        case "clear-scrollback": if (v) v.showMessage("\u001b[3J"); break
        case "fullscreen": toggleFullScreen(); break
        case "assistant": win.toggleAssistant(); break
        case "zoom-in": zoomBy(1); break
        case "zoom-out": zoomBy(-1); break
        case "zoom-reset": fontSize = 0; break
        case "maximize-pane": toggleMaximize(); break
        case "transparency": useTransparency = !useTransparency; break
        case "find-cursor": if (v) v.findCursor(); break
        case "cursor-guide": cursorGuide = !cursorGuide; break
        case "input-box": toggleInputBox(); break
        case "tab-color-none": if (tabs[currentTab]) tabs[currentTab].tabColor = "transparent"; break
        case "edit-session": case "edit-profiles": prefs.showProfile(activeProfile.name); break
        case "restart-session": if (v) confirmJobs([paneOf(v)], function() { v.restart() }); break
        case "duplicate-session": if (v) newTab({ profile: activeProfile.name, cwd: v.pwd }); break
        case "reset": if (v) v.resetTerminal(); break
        case "log-start": if (v && !v.logging) logDialog.open(); break
        case "log-stop": if (v && v.logging) v.stopLogging(); break
        case "record": if (v) win.toggleRecording(v); break
        case "shot-pane": if (v) win.screenshot(paneOf(v)); break
        case "shot-window": win.screenshot(rootLayout); break
        case "open-all-profiles": for (const p of profileList) newTab({ profile: p.name }); break
        case "minimize": win.showMinimized(); break
        case "zoom-window": win.visibility === Window.Maximized ? win.showNormal() : win.showMaximized(); break
        case "window-title": askText("Window Title", "Title (empty: the program's title)", customTitle, (t) => customTitle = t); break
        case "tab-title": if (tabs.length > 0) { const t = tabs[currentTab]; askText("Tab Title", "Title (empty: automatic)", t.customTitle, (s) => { t.customTitle = s; treeRev++ }) } break
        case "next-tab": focusTab((currentTab + 1) % tabs.length); break
        case "prev-tab": focusTab((currentTab + tabs.length - 1) % tabs.length); break
        case "move-tab-left": moveTab(-1); break
        case "move-tab-right": moveTab(1); break
        case "pane-up": moveFocus(0, -1); break
        case "pane-down": moveFocus(0, 1); break
        case "pane-left": moveFocus(-1, 0); break
        case "pane-right": moveFocus(1, 0); break
        case "pane-next": cyclePane(1); break
        case "pane-prev": cyclePane(-1); break
        case "save-arrangement": askText("Save Window Arrangement", "Name", "", (n) => { if (n.trim() !== "") controller.saveArrangement(n.trim(), JSON.stringify(serializeWindow(false))) }); break
        case "manage-arrangements": prefs.showPage("arrangements"); break
        case "help": Qt.openUrlExternally("https://github.com/benwbooth/tessera#readme"); break
        }
    }

    GlobalMenuBar {
        id: globalMenu
        window: win
        model: win.menuModel
        onTriggered: id => win.menuAction(id)
    }
    // The menu's address goes on the window's surface, which exists only
    // while the window is shown.
    onVisibleChanged: if (visible) Qt.callLater(globalMenu.attach)
    // The ☰ button's menu: the same model in the shared glass look (the
    // menus, then Restart tesserad…, Preferences…, Quit at the bottom;
    // About is in Help there). An in-window popup (Popup.Item): a separate
    // popup window gets a different scale from the compositor and loses
    // pixels.
    onActiveChanged: if (!active) appMenu.close()
    property real appMenuClosedAt: 0
    AppMenu {
        id: appMenu
        iconUrl: (n, px) => win.menuIconUrl(n, px)
        model: win.menuModel
        appMenuAtEnd: true
        accent: Theme.accent
        dark: Theme.dark
        popupType: Popup.Item
        onTriggered: id => win.menuAction(id)
        onAboutToShow: win.appMenuOpen = true
        onAboutToHide: win.appMenuOpen = false
        onClosed: win.appMenuClosedAt = Date.now()
    }

    // The icon theme to match: breeze-dark on a dark theme (rows already
    // built get their icons again).
    IconTheme { id: iconTheme; Component.onCompleted: win.applyIcons() }
    Connections { target: Theme; function onDarkChanged() { win.applyIcons() } }
    // (Menus' icons: drawn off the UI thread by IconTheme's provider, the
    // one-colour ones in the menu's text colour. A new iconRev: drawn again.)
    property int iconRev: 0
    function menuIconUrl(n, px) {
        return "image://icon/" + encodeURIComponent(n) + "?px=" + px + "&mono=" + (Theme.dark ? "f5f5f7" : "1d1d1f") + "&r=" + iconRev
    }
    function applyIcons() {
        if (iconTheme.setDark(Theme.dark)) iconRev++
    }

    // The menus below are written as platform menus (P.Menu, their items as
    // data) but shown in the app's own look, AppMenu, like the ☰: the
    // platform's were the desktop style's (light, whatever the theme). Each
    // row triggers its item, so they work as written.
    // One AppMenu per menu and set of rows shown (a link under the pointer
    // or not, a selection or not...), each built once: making the rows is
    // what's slow (~0.5 s for the terminal's), so opening one already made
    // only updates their texts and states.
    Component {
        id: bridgeComp
        AppMenu {
            iconUrl: (n, px) => win.menuIconUrl(n, px)
            flat: true
            accent: Theme.accent
            dark: Theme.dark
            popupType: Popup.Item
            property var src: null
            property string kind: ""
            property var acts: []
            onTriggered: id => { const it = acts[parseInt(id)]; if (it) it.triggered() }
            onClosed: if (src) src.aboutToHide()
        }
    }
    property var bridges: []
    function bridgeFor(menu, kind) {
        for (const b of bridges) if (b.src === menu && b.kind === kind) return b
        const b = bridgeComp.createObject(win, { src: menu, kind: kind })
        bridges.push(b)
        return b
    }
    // (The rows shown now; `shape` collects which, as the menu's kind.)
    function bridgeModel(menu, acts, shape) {
        const out = []
        for (let i = 0; i < menu.items.length; i++) {
            const it = menu.items[i]
            const on = !!it && it.visible && (!it.subMenu || it.subMenu.enabled)
            shape.push(on ? "1" : "0")
            if (!on) continue
            if (it.separator) { out.push({ separator: true }); continue }
            const sub = it.subMenu
            if (sub) {
                shape.push("(")
                out.push({ id: "sub-" + i, text: sub.title, icon: sub.icon.name, items: bridgeModel(sub, acts, shape) })
                shape.push(")")
                continue
            }
            const parts = it.text.split("\t")
            acts.push(it)
            out.push({ id: String(acts.length - 1), text: parts[0], shortcut: parts[1] || "", icon: it.icon.name,
                       enabled: it.enabled, checkable: it.checkable, checked: it.checked })
        }
        return out
    }
    // (Where: x, y in item's coordinates.)
    function showMenu(menu, item, x, y) {
        for (const o of bridges) if (o.visible) o.close()
        menu.aboutToShow()
        const acts = [], shape = []
        const model = [{ title: "", items: bridgeModel(menu, acts, shape) }]
        const b = bridgeFor(menu, shape.join(""))
        b.model = model
        b.acts = acts
        b.openAt(item, x, y)
    }

    // The + button's menu: profiles, the shells installed, ssh hosts.
    property var launchData: ({ shells: [], ssh_hosts: [] })
    property bool launchOpen: false
    property bool appMenuOpen: false
    P.Menu {
        id: launchMenu
        onAboutToShow: { win.launchData = JSON.parse(controller.launchers()); win.launchOpen = true }
        onAboutToHide: win.launchOpen = false
        // (Either view; a tab stays the one it was opened as.)
        P.MenuItem { icon.name: "view-list-details"; text: "New Notebook Tab\tCtrl+Shift+J"; onTriggered: newTab({ view: "notebook" }) }
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
    // (One line, for a menu row: a selection's line breaks and runs of
    // spaces as one space.)
    readonly property string contextSelShort: { const t = contextSelection.replace(/\s+/g, " ").trim(); return t.length > 30 ? t.slice(0, 28).trim() + "…" : t }

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
        property var infoAsking: ({})    // (session ids asked, not yet answered)
        onTriggered: {
            // (Every pane: titles, and each editor's prompt and REPL detection.)
            for (const p of allPanes()) {
                const v = p.view
                if (!v) continue
                const apply = (info) => {
                    if (p.job !== info.job) p.job = info.job
                    if (p.procCwd !== info.cwd) p.procCwd = info.cwd
                    if (p.shellName !== info.shell) p.shellName = info.shell
                    if (info.pid && p.shellPid !== info.pid) p.shellPid = info.pid
                }
                if (v.exited || v.sessionId === "") { apply({ job: "", cwd: "", shell: "" }); continue }
                // (One question in flight per pane; a stale answer, for a
                // session the pane no longer shows, is dropped.)
                const id = v.sessionId
                if (infoAsking[id]) continue
                infoAsking[id] = true
                Async.call(controller, k => controller.sessionInfo(k, id), r => {
                    delete infoAsking[id]
                    let info = { job: "", cwd: "", shell: "" }
                    try { info = JSON.parse(r) } catch (e) {}
                    try { if (p && p.view && p.view.sessionId === id) apply(info) } catch (e) {}
                })
            }
        }
    }
    // A notebook found tesserad (here or on a host) too old for what it
    // asked (moving a card to the background, a background job): asked
    // whether to update it now. (A newer build alone isn't asked about: the
    // devloop makes one every few minutes; it swaps itself in once no kernel
    // or job runs.) Once it's updated every notebook is told
    // (daemon.updated): their kernels start again.
    Connections {
        target: controller
        function onJupyterEvent(key, json) {
            if (key !== "tesserad" && !key.endsWith("//tesserad")) return
            let e
            try { e = JSON.parse(json) } catch (x) { return }
            if (e.type !== "daemon.old") return
            const host = key === "tesserad" ? "" : key.slice(0, -"//tesserad".length)
            sheet.ask({
                title: "Update tesserad?",
                message: "Tessera's background process (tesserad" + (host ? " on " + host : "") + ") is an older build, from before " + (e.feature || "this") + ". "
                       + "It updates itself to this build in place, keeping your shells, but only once no kernel or background job runs in it.\n\n"
                       + "Updating now stops them (kernels start again after, without their variables; background jobs end), then " + (e.retry || "carries on") + ".",
                okText: "Stop and Update",
                destructive: true,
                onAccept: () => Async.call(controller, k => controller.updateDaemon(k, host), json => {
                    let r = {}
                    try { r = JSON.parse(json) } catch (x) {}
                    controller.jupyterEvent(key, JSON.stringify({ type: "daemon.updated", ok: !!r.ok, error: r.error || "" }))
                })
            })
            // (Test hook: TESSERA_TEST_UPDATE_ACCEPT says yes.)
            if (controller.testEnv("UPDATE_ACCEPT") !== "") Qt.callLater(sheet.accept)
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
            onAccept: win.restartDaemon
        })
    }
    // A newer tesserad is waiting: the running one swaps it in (shells kept)
    // only once no notebook kernel or background job runs in it, so a window
    // with a notebook open can stay on the old one for good. Said once per
    // start, with the way to update now; it stays until answered (a devloop
    // window restarts often, and a note that went away by itself was missed).
    Timer {
        id: waitingTimer
        interval: 2500
        onTriggered: win.notice("A newer tesserad is waiting for the notebook kernels to stop", [
            { label: "Update now\u2026", run: win.askUpdateWaiting },
            { label: "Later", run: () => {} }], -1)
        // (Test hook: TESSERA_TEST_UPDATE_ACCEPT takes Update now and says yes.)
        onRunningChanged: if (!running && controller.testEnv("UPDATE_ACCEPT") !== "") { win.askUpdateWaiting(); Qt.callLater(sheet.accept) }
    }
    function askUpdateWaiting() {
        sheet.ask({
            title: "Update tesserad now?",
            message: "Tessera's background process (tesserad) is an older build. It updates itself to this one in place, keeping your shells "
                   + "and what runs in them, but only once no notebook kernel or background job runs in it.\n\n"
                   + "Updating now stops them: notebooks' kernels start again after, without their variables, and background jobs end.",
            okText: "Stop and Update",
            destructive: true,
            onAccept: () => Async.call(controller, k => controller.updateDaemon(k, ""), json => {
                let r = {}
                try { r = JSON.parse(json) } catch (x) {}
                controller.jupyterEvent("tesserad", JSON.stringify({ type: "daemon.updated", ok: !!r.ok, error: r.error || "" }))
                win.notice(r.ok ? "tesserad is up to date" : "tesserad didn't update: " + (r.error || "no answer"), [])
            })
        })
    }
    // tesserad, the shells' background process, started again (this build's):
    // new shells in every tab's place, each under its saved screen, then the
    // old tabs go. Programs running in them, and notebook kernels, end.
    function restartDaemon() {
        win.saveSnapshots()
        const layout = win.serializeWindow(true)
        const old = win.tabs.slice(), cur = win.currentTab
        // (Their shells end with the daemon: the old panes mustn't close
        // themselves, and the window with them, before the new ones are in.)
        const panes = win.allPanes()
        for (const p of panes) p.taken = true
        Async.call(controller, k => controller.stopDaemon(k), stopped => {
            if (stopped !== "true") {
                for (const p of panes) p.taken = false
                sheet.ask({ title: "Couldn't restart it", message: "The background process didn't stop. Logging out and back in restarts it.", infoOnly: true, okText: "OK" })
                return
            }
            win.restoreArrangement(layout, {}, true)
            for (const t of old) win.removeTab(t)
            win.focusTab(Math.min(cur, win.tabs.length - 1))
            win.saveLayoutNow()
        })
    }
    function askRestartDaemon() {
        sheet.ask({
            title: "Restart tesserad?",
            message: "tesserad is the background process your shells and notebook kernels run in. Restarting it starts this version's. "
                   + "Every tab comes back in its folder with its screen; programs running in them (and in other Tessera windows) end, "
                   + "and notebooks' kernels start again (their variables are lost).",
            okText: "Restart",
            destructive: true,
            onAccept: win.restartDaemon
        })
    }
    // (Test hook: TESSERA_TEST_GUIDE="text" opens the tessh guide (or
    // TESSERA_TEST_GUIDE_LANG's) at 3 s and finds the text in it.)
    Timer {
        interval: 3000; running: controller.testEnv("GUIDE") !== ""
        onTriggered: { win.openGuide(controller.testEnv("GUIDE_LANG")); guideFind.start() }
    }
    Timer {
        id: guideFind; interval: controller.testEnv("GUIDE_LANG") !== "" ? 20000 : 2500
        onTriggered: {
            const t = win.tabs[win.currentTab]
            if (!t || !t.pageItem) return
            if (controller.testEnv("GUIDE_WHEEL") !== "") t.pageItem.testWheel()
            // (TESSERA_TEST_GUIDE_SWITCH=GUIDE: its chip clicked, as a person would.)
            // (TESSERA_TEST_GUIDE_SEARCH=1: TESSERA_TEST_GUIDE's words looked for in every guide.)
            else if (controller.testEnv("GUIDE_SEARCH") !== "") { t.pageItem.find(controller.testEnv("GUIDE")); t.pageItem.searchAll() }
            // (TESSERA_TEST_GUIDE_RESTYLE=1: the profile's colours changed under an open guide.)
            else if (controller.testEnv("GUIDE_RESTYLE") !== "") { t.pageItem.ansi = Array(16).fill("#ff3030"); t.pageItem.find(controller.testEnv("GUIDE")) }
            else if (controller.testEnv("GUIDE_SWITCH") !== "") t.pageItem.chose(controller.testEnv("GUIDE_SWITCH"), controller.testEnv("GUIDE_SWITCH"))
            else t.pageItem.find(controller.testEnv("GUIDE"))
        }
    }
    // (Test hook: TESSERA_TEST_RESTART_DAEMON restarts tesserad at 6 s.)
    Timer { interval: 6000; running: controller.testEnv("RESTART_DAEMON") !== ""; onTriggered: win.restartDaemon() }
    // Test hook: TESSERA_TEST_KEYS="term|editor|away:key,key,..." (ca:x is Ctrl+Alt+x; tab, bs, enter,
    // lshift, rshift, or text), one key every 150 ms after the window settles.
    Timer {
        id: keysTestTimer
        interval: 4500
        property var keys: []
        onTriggered: {
            const spec = controller.testEnv("KEYS")
            const target = spec.slice(0, spec.indexOf(":")), list = spec.slice(spec.indexOf(":") + 1)
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
            if (k === "wait") { if (activePane) console.log("key test: wait ghost=" + JSON.stringify(activePane.compose.ghost) + " ready=" + activePane.compose.inlineReady + " editor=" + JSON.stringify(activePane.compose.text)); return }   // (a pause: let the AI answer)
            const named = { space: [Qt.Key_Space, 0, " ", 65], tab: [Qt.Key_Tab, 0, "\t", 23], bs: [Qt.Key_Backspace, 0, "\b", 22], enter: [Qt.Key_Return, 0, "\r", 36],
                            lshift: [Qt.Key_Shift, 0, "", 50], rshift: [Qt.Key_Shift, 0, "", 62], esc: [Qt.Key_Escape, 0, "\x1b", 9], ctrl: [Qt.Key_Control, 0, "", 37], rctrl: [Qt.Key_Control, 0, "", 105], stab: [Qt.Key_Backtab, Qt.ShiftModifier, "", 23],
                            up: [Qt.Key_Up, 0, "", 111], down: [Qt.Key_Down, 0, "", 116],
                            pgup: [Qt.Key_PageUp, 0, "", 112], pgdn: [Qt.Key_PageDown, 0, "", 117] }
            // (ca:x: Ctrl+Alt+x; ca:enter for Return. al:x: Alt+x. ct:x: Ctrl+x.)
            const ca = k.startsWith("ca:"), al = k.startsWith("al:"), ct = k.startsWith("ct:")
            const kk = ca || al || ct ? k.slice(3) : k
            const d0 = named[kk] || [kk.toUpperCase().charCodeAt(0), 0, kk, 0]
            const d = ca ? [d0[0], Qt.ControlModifier | Qt.AltModifier, "", d0[3]] : al ? [d0[0], Qt.AltModifier, "", d0[3]] : ct ? [d0[0], Qt.ControlModifier, "", d0[3]] : d0
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
    // Memory freed (a big listing's text, cards gone) handed back to the
    // system: glibc keeps it otherwise. Once things have settled after
    // starting, then every minute; on a thread of its own.
    Timer {
        interval: 15000; repeat: true; running: true
        onTriggered: { interval = 60000; controller.trimMemory() }
    }
    // (Test hook: TESSERA_TEST_CLICKS="x,y;x,y;..." moves to each and left-clicks
    // it, from 12 s, 1.5 s apart.)
    Timer {
        interval: 750; repeat: true
        running: controller.testEnv("CLICKS") !== ""
        property var pts: controller.testEnv("CLICKS").split(";").map(q => q.split(",").map(Number))
        property int n: -16
        onTriggered: {
            if (++n < 0) return
            const q = pts[n >> 1]
            if (!q) { stop(); return }
            if (n % 2 === 0) { controller.testMouse(3, q[0] - 1, q[1], 0, 0); controller.testMouse(3, q[0], q[1], 0, 0) }
            else { controller.testMouse(0, q[0], q[1], 1, 0); controller.testMouse(2, q[0], q[1], 1, 0) }
        }
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
    // (TESSERA_TEST_TAB_SWITCH=<n>: from 9 s, the next of the first n tabs
    // every 1.5 s, eight times, logging how long until the frame after each switch is shown.)
    property double switchAt: 0
    Connections { target: win; function onFrameSwapped() { if (win.switchAt > 0) { console.log("tab switch: " + (Date.now() - win.switchAt) + " ms"); win.switchAt = 0 } } }
    Timer { id: switchTest; interval: 1500; repeat: true; property int left: 8
            running: false
            onTriggered: { if (--left < 0) { stop(); return } const n = Math.min(win.tabs.length, Number(controller.testEnv("TAB_SWITCH")) || win.tabs.length); win.switchAt = Date.now(); win.focusTab((win.currentTab + 1) % n) } }
    Timer { interval: 9000; running: controller.testEnv("TAB_SWITCH") !== ""; onTriggered: switchTest.start() }
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
    Timer { id: composeTestTimer; interval: Number(controller.testEnv("COMPOSE_DELAY") || 4000); onTriggered: if (activePane) { activePane.compose.focusEditor(); if (controller.testEnv("COMPOSE_LANG") !== "") activePane.compose.pickLanguage(controller.testEnv("COMPOSE_LANG")); activePane.compose.insert(controller.testEnv("COMPOSE").replace(/\\n/g, "\n")); if (controller.testEnv("COMPOSE_TAB") !== "") activePane.compose.openComp(); if (controller.testEnv("COMPOSE_RUN") !== "") activePane.compose.submit(); if (controller.testEnv("COMPOSE_HISTORY") !== "") activePane.compose.openHistory(); if (controller.testEnv("COMPOSE_GUIDE") !== "") activePane.compose.openGuideAtWord() } }
    Timer { id: contextTestTimer; interval: 2500; onTriggered: { const c = controller.testEnv("CONTEXT"); if (c === "app") appMenu.openAt(menuButton, 0, menuButton.height + 4); else if (c === "launch") showMenu(launchMenu, win.contentItem, 300, 40); else if (c === "tab") { win.menuTab = tabs[0]; win.openMenuAt(tabMenu, win.contentItem, 120, 40) } else if (c === "zoom") showMenu(zoomMenu, win.contentItem, 300, 200); else if (c === "hideinput") toggleInputBox(); else if (c === "termtwice" && activePane) { activePane.contextMenu(activeView, 200, 150); later(1200, () => { for (const b of bridges) b.close(); later(1500, () => { activeView.selectAll(); activePane.contextMenu(activeView, 200, 150) }) }) } else if (activePane) activePane.contextMenu(activeView, 200, 150) } }
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
            visible: contextLink !== ""; onTriggered: { if (contextLink.startsWith("ssh://")) { Async.call(controller, k => controller.fileAction(k, "open", contextLink, ""), e => { if (e !== "") notice(e, []) }) } else Qt.openUrlExternally(contextLink) } }
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
            // (20 places, the history's most, each shown when it has an
            // entry: the menu keeps its shape as the history grows, so its
            // rows aren't made again.)
            Instantiator {
                model: 20
                delegate: P.MenuItem {
                    visible: index < win.pasteHistory.length
                    text: visible ? win.shortLabel(win.pasteHistory[index]) : ""
                    onTriggered: { const s = win.pasteHistory[index]; if (s !== undefined) pasteInto(contextView, s) }
                }
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
                onMenuClicked: showMenu(launchMenu, plus.menuAnchor, 0, plus.menuAnchor.height + 4)
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
        // A thousandth of a pixel keeps text off exact half pixels at 130%,
        // both ways: on one, each glyph rounds on its own (e/o drop a pixel,
        // letters squash together).
        anchors.topMargin: 0.001
        anchors.leftMargin: 0.001
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
            // Drag the bar to move the window, double-click to maximize,
            // right-click for the window menu (the shared ChromeTitleArea).
            ChromeTitleArea { anchors.fill: parent; window: win; menuDark: Theme.dark; menuAccent: Theme.accent }

            // ☰ The app menu.
            MacMenuButton {
                id: menuButton
                anchors.left: parent.left
                anchors.leftMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                dark: Theme.dark
                fg: Theme.fg
                menu: appMenu
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
                TapHandler { acceptedButtons: Qt.RightButton; onTapped: showMenu(zoomMenu, zoomChip, 0, zoomChip.height + 4) }
                ToolTip.visible: zoomHover.hovered && !win.zoomMenuOpen
                ToolTip.text: "Text size " + win.zoomPercent + "% — click for 100% (Ctrl+0)"
                ToolTip.delay: 500
            }

            // Without a tab row up here: the window title.
            Text {
                visible: !(showTabs && !tabsAtBottom)
                anchors.centerIn: parent
                anchors.verticalCenterOffset: 0.001
                anchors.horizontalCenterOffset: 0.001
                width: Math.min(implicitWidth, parent.width - 2 * (controls.width + 60))
                horizontalAlignment: Text.AlignHCenter
                renderType: Text.NativeRendering
                text: win.title
                color: win.active ? Theme.text : Theme.textDim
                font.pixelSize: 13
                font.weight: Font.DemiBold
                elide: Text.ElideMiddle
            }

            WindowControls {
                id: controls
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

    // Without the system frame, strips along the edges resize the window
    // (the shared ChromeResizeEdges, as every app).
    ChromeResizeEdges { id: edges; anchors.fill: parent; z: 10000; window: win }

    // Dim background windows (Appearance > Dimming).
    Rectangle {
        anchors.fill: parent
        visible: settings.dim_background_windows && !win.active
        color: Theme.dark ? "black" : "white"
        opacity: settings.dim_amount
    }

    // ---- startup -----------------------------------------------------------

    Component.onCompleted: {
        Async.call(controller, k => controller.historyAll(k, 5000), r => { try { win.commandHistory = JSON.parse(r) } catch (e) {} })
        const torn = controller.initialTab()
        if (torn !== "") {
            let restored = false
            try {
                const tl = JSON.parse(torn)
                newTab({ layout: tl, alive: sessionIds(tl.root, {}), profile: tl.profile })
                restored = tabs.length > 0
            } catch (e) {}
            startupRest(restored)
        } else if (controller.restoreAllowed() && settings.restore_sessions) {
            // (The daemon's live shells, asked in the background; the rest of
            // startup waits for them.)
            Async.call(controller, k => controller.restorableSessions(k, ""), r => {
                let alive = {}, ids = []
                try { ids = JSON.parse(r) } catch (e) {}
                for (const id of ids) alive[id] = true
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
                startupRest(tabs.length > 0)
            })
        } else startupRest(false)
    }
    // (The rest of startup, once the tabs are back.)
    function startupRest(restored) {
        if (!restored) {
            newTab({})
            if (!savedSize) sizeTimer.start()
            else if (startMaximized && win.visibility !== Window.Maximized) win.showMaximized()
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
        // (Test hook: the assistant's history dropdown, AssistantPanel.qml.)
        if (controller.testEnv("CHAT_HISTORY") !== "" || controller.testEnv("CHAT_SEARCH") !== "") win.toggleAssistant()
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
        else if (controller.daemonState() === "waiting") waitingTimer.start()
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
            // (TESSERA_TEST_SHOT_ALL: with popups too.)
            const target = controller.prefsPage() !== "" ? prefs.contentItem : controller.testEnv("SHOT_ALL") !== "" ? win.Overlay.overlay.parent : rootLayout
            const ok = target.grabToImage(function(r) {
                r.saveToFile(controller.screenshotPath())
                Qt.quit()
            })
            if (!ok) Qt.quit()
        }
    }
}
