import QtQuick
import QtQuick.Controls
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
    color: Theme.bg
    title: activeView && activeView.title ? activeView.title : "Tessera"

    property var tabs: []
    property int currentTab: 0
    property var activeView: null
    property real fontSize: 0      // a zoom (Ctrl+=/-) over the preference; 0 = none
    property var tmuxGateways: ({}) // gateway key -> gateway TerminalView

    Controller { id: controller }

    // ---- preferences and theme -------------------------------------------

    readonly property int rev: controller.revision
    readonly property var settings: { rev; return JSON.parse(controller.settings()) }
    readonly property bool systemDark: Qt.styleHints.colorScheme !== Qt.ColorScheme.Light
    readonly property string themeName: settings.theme_mode === "system"
        ? (systemDark ? settings.theme_dark : settings.theme_light) : settings.theme
    readonly property var theme: { rev; return JSON.parse(controller.theme(themeName)) }
    readonly property string themeJson: JSON.stringify(theme)
    readonly property int cursorShape: ({ block: 0, bar: 1, underline: 2 })[settings.cursor_style] || 0
    Binding { target: Theme; property: "term"; value: win.theme }

    function setSetting(key, value) { controller.setSetting(key, JSON.stringify(value)) }

    Component {
        id: paneComp
        TerminalPane {
            themeJson: win.themeJson
            fontFamily: win.settings.font_family
            fontPointSize: win.fontSize > 0 ? win.fontSize : win.settings.font_size
            lineHeight: win.settings.line_height
            paddingSize: win.settings.padding
            cursorShape: win.cursorShape
            scrollback: win.settings.scrollback
            copyOnSelect: win.settings.copy_on_select
            dimInactive: win.settings.dim_inactive
            onContextMenu: (v) => { win.contextView = v; contextMenu.open() }
        }
    }
    property var contextView: null
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
            property string fixedTitle: ""
            property var lastView: null
            readonly property string tabTitle: fixedTitle !== "" ? fixedTitle
                : (lastView && lastView.title ? lastView.title
                   : (lastView && lastView.pwd ? lastView.pwd.split("/").filter(x => x).pop() || "/" : "Shell"))
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
        let h = item.host !== undefined ? item.host : null
        let node = item
        while (node && !node.isRoot) node = node.host
        return node ? node.tab : null
    }

    function makePane(host, props) {
        const p = paneComp.createObject(null, props)
        p.host = host
        p.view.activeFocusChanged.connect(function() {
            if (p.view.activeFocus) {
                activeView = p.view
                const t = tabOf(p)
                if (t) t.lastView = p.view
            }
        })
        p.view.exitedChanged.connect(function() {
            if (p.view.exited && p.view.tmuxPane === "") Qt.callLater(closePane, p, false)
        })
        p.view.tmuxEvent.connect(function(ev) { onTmuxEvent(p.view, ev) })
        p.view.openFailed.connect(function(msg) { console.warn("tessera: " + msg) })
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

    function splitPane(p, orientation) {
        if (!p) return
        if (p.view.tmuxPaneId >= 0) {
            p.view.tmuxCommand((orientation === Qt.Horizontal ? "split-window -h -t %" : "split-window -v -t %") + p.view.tmuxPaneId)
            return
        }
        const h = p.host
        if (h.count === 1) h.orientation = orientation
        const np = (h.orientation === orientation)
            ? (function() {
                const n = makePane(h, { cwd: p.view.pwd })
                h.insertItem(indexIn(h, p) + 1, n)
                equalize(h)
                return n
            })()
            : (function() {
                const idx = indexIn(h, p)
                const w = p.width, ht = p.height
                h.takeItem(idx)
                const s = splitComp.createObject(null, { orientation: orientation })
                s.host = h
                s.prefW = w; s.prefH = ht
                h.insertItem(idx, s)
                p.host = s
                s.addItem(p)
                const n = makePane(s, { cwd: p.view.pwd })
                s.addItem(n)
                Qt.callLater(equalize, s)
                return n
            })()
        focusPane(np)
    }

    // Remove a pane from its split; `kill` also ends its shell.
    function closePane(p, kill) {
        if (!p || !p.host) return
        if (kill) {
            if (p.view.tmuxPaneId >= 0) { p.view.tmuxCommand("kill-pane -t %" + p.view.tmuxPaneId); return }
            p.view.killSession()
        }
        const h = p.host
        const idx = indexIn(h, p)
        if (idx >= 0) h.takeItem(idx)
        p.host = null
        p.destroy()
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
        if (node.view !== undefined) { out.push(node); return out }
        for (let i = 0; i < node.count; ++i) panesIn(node.itemAt(i), out)
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
            if (p === cur) continue
            const q = p.mapToItem(win.contentItem, p.width / 2, p.height / 2)
            const vx = q.x - c.x, vy = q.y - c.y
            if (vx * dx + vy * dy <= 0) continue
            const d = Math.abs(vx) + Math.abs(vy) + 2 * Math.abs(dx !== 0 ? vy : vx)
            if (d < bestD) { bestD = d; best = p }
        }
        focusPane(best)
    }

    // ---- tabs ------------------------------------------------------------

    function newTab(opts) {
        opts = opts || {}
        const active = activeView
        if (!opts.tmux && active && active.tmuxPaneId >= 0 && !opts.attachId) {
            active.tmuxCommand("new-window")
            return null
        }
        const t = tabComp.createObject(stack, { gateway: opts.gateway || "", tmuxWindow: opts.window !== undefined ? opts.window : -1 })
        if (!opts.tmux) {
            const props = {}
            if (opts.attachId) props.attachId = opts.attachId
            if (opts.cwd) props.cwd = opts.cwd
            else if (active && active.pwd) props.cwd = active.pwd
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
        return t
    }

    function removeTab(t) {
        const i = tabs.indexOf(t)
        if (i < 0) return
        tabs.splice(i, 1)
        tabs = tabs.slice()
        t.destroy()
        if (tabs.length === 0) { Qt.quit(); return }
        currentTab = Math.min(currentTab, tabs.length - 1)
        focusTab(currentTab)
    }

    function closeTab(t) {
        if (t.tmuxWindow >= 0) {
            const gw = tmuxGateways[t.gateway]
            if (gw) gw.tmuxCommand("kill-window -t @" + t.tmuxWindow)
            return
        }
        for (const p of panesIn(t.root, [])) p.view.killSession()
        removeTab(t)
    }

    function focusTab(i) {
        if (i < 0 || i >= tabs.length) return
        currentTab = i
        const t = tabs[i]
        const p = t.lastView ? paneOf(t.lastView) : firstPane(t.root)
        focusPane(p)
    }

    function zoomBy(d) {
        const base = fontSize > 0 ? fontSize : (activeView ? activeView.fontPointSize : 11)
        fontSize = Math.max(6, Math.min(48, base + d))
    }

    function closeActivePane() { if (activeView) closePane(paneOf(activeView), true) }
    function splitActive(o) { if (activeView) splitPane(paneOf(activeView), o) }

    // ---- tmux control mode -----------------------------------------------

    function tmuxTab(gateway, window) {
        for (const t of tabs) if (t.gateway === gateway && t.tmuxWindow === window) return t
        return null
    }

    function onTmuxEvent(view, ev) {
        if (controller.screenshotPath() !== "") console.log("tmux event " + JSON.stringify(ev))
        switch (ev.type) {
        case "tmuxStart":
            tmuxGateways[ev.gateway] = view
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
        function build(node, host) {
            if (node.t === "pane") {
                let p = existing[node.id]
                if (p) delete existing[node.id]
                else p = makePane(host, { tmuxPane: t.gateway + ":%" + node.id })
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
    }

    Item { id: holder; visible: false }

    // ---- menus (the desktop's own: Breeze icons and look on KDE) ---------

    P.Menu {
        id: appMenu
        P.MenuItem { icon.name: "tab-new"; text: "New Tab\tCtrl+Shift+T"; onTriggered: newTab({}) }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "view-split-left-right"; text: "Split Right\tCtrl+Shift+D"; onTriggered: splitActive(Qt.Horizontal) }
        P.MenuItem { icon.name: "view-split-top-bottom"; text: "Split Down\tCtrl+Shift+E"; onTriggered: splitActive(Qt.Vertical) }
        P.MenuItem { icon.name: "view-close"; text: "Close Pane\tCtrl+Shift+W"; onTriggered: closeActivePane() }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "edit-copy"; text: "Copy\tCtrl+Shift+C"; enabled: activeView !== null && activeView.hasSelection; onTriggered: activeView.copy() }
        P.MenuItem { icon.name: "edit-paste"; text: "Paste\tCtrl+Shift+V"; onTriggered: if (activeView) activeView.paste() }
        P.MenuItem { icon.name: "edit-select-all"; text: "Select All\tCtrl+Shift+A"; onTriggered: if (activeView) activeView.selectAll() }
        P.MenuItem { icon.name: "edit-clear-history"; text: "Clear Buffer\tCtrl+Shift+K"; onTriggered: if (activeView) activeView.clearScrollback() }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "zoom-in"; text: "Bigger Text\tCtrl+="; onTriggered: zoomBy(1) }
        P.MenuItem { icon.name: "zoom-out"; text: "Smaller Text\tCtrl+-"; onTriggered: zoomBy(-1) }
        P.MenuItem { icon.name: "zoom-original"; text: "Actual Size\tCtrl+0"; enabled: fontSize > 0; onTriggered: fontSize = 0 }
        P.Menu {
            id: themeMenu
            title: "Color Theme"
            icon.name: "color-management"
            P.MenuItemGroup { id: themeGroup }
            Instantiator {
                model: { win.rev; return JSON.parse(controller.themes()) }
                delegate: P.MenuItem {
                    text: modelData.name
                    checkable: true
                    checked: modelData.name === win.themeName
                    group: themeGroup
                    onTriggered: pickTheme(modelData)
                }
                onObjectAdded: (i, o) => themeMenu.insertItem(i, o)
                onObjectRemoved: (i, o) => themeMenu.removeItem(o)
            }
            P.MenuSeparator {}
            P.MenuItem { icon.name: "configure"; text: "More Themes…"; onTriggered: prefs.showPage("appearance") }
        }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "go-next"; text: "Next Tab\tCtrl+PgDown"; enabled: tabs.length > 1; onTriggered: focusTab((currentTab + 1) % tabs.length) }
        P.MenuItem { icon.name: "go-previous"; text: "Previous Tab\tCtrl+PgUp"; enabled: tabs.length > 1; onTriggered: focusTab((currentTab + tabs.length - 1) % tabs.length) }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "configure"; text: "Preferences…\tCtrl+,"; onTriggered: prefs.showPage("") }
        P.MenuItem { icon.name: "help-about"; text: "About Tessera"; onTriggered: prefs.showPage("about") }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "application-exit"; text: "Quit\tCtrl+Shift+Q"; onTriggered: Qt.quit() }
    }

    // Right click in a terminal (when the program isn't taking the mouse).
    P.Menu {
        id: contextMenu
        P.MenuItem { icon.name: "edit-copy"; text: "Copy\tCtrl+Shift+C"; enabled: contextView !== null && contextView.hasSelection; onTriggered: contextView.copy() }
        P.MenuItem { icon.name: "edit-paste"; text: "Paste\tCtrl+Shift+V"; onTriggered: contextView.paste() }
        P.MenuItem { icon.name: "edit-select-all"; text: "Select All\tCtrl+Shift+A"; onTriggered: contextView.selectAll() }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "view-split-left-right"; text: "Split Right\tCtrl+Shift+D"; onTriggered: splitPane(paneOf(contextView), Qt.Horizontal) }
        P.MenuItem { icon.name: "view-split-top-bottom"; text: "Split Down\tCtrl+Shift+E"; onTriggered: splitPane(paneOf(contextView), Qt.Vertical) }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "edit-clear-history"; text: "Clear Buffer\tCtrl+Shift+K"; onTriggered: contextView.clearScrollback() }
        P.MenuItem { icon.name: "configure"; text: "Preferences…\tCtrl+,"; onTriggered: prefs.showPage("") }
        P.MenuSeparator {}
        P.MenuItem { icon.name: "view-close"; text: "Close Pane\tCtrl+Shift+W"; onTriggered: closePane(paneOf(contextView), true) }
    }

    // Picking a theme: with "follow the system" it becomes the light or dark
    // theme (by its colors); otherwise the one theme.
    function pickTheme(t) {
        if (settings.theme_mode === "system") setSetting(t.dark ? "theme_dark" : "theme_light", t.name)
        else setSetting("theme", t.name)
    }

    Preferences {
        id: prefs
        controller: controller
        settings: win.settings
        themeName: win.themeName
        systemDark: win.systemDark
        transientParent: win
        onPick: (t) => win.pickTheme(t)
        onChange: (k, v) => win.setSetting(k, v)
    }

    // ---- shortcuts (iTerm2's, with Ctrl+Shift for Cmd) --------------------

    // The menus show these keys in their text; the keys themselves live here
    // (one place, and Close Pane doesn't auto-repeat when held).
    Shortcut { sequence: "Ctrl+Shift+T"; onActivated: newTab({}) }
    Shortcut { sequence: "Ctrl+Shift+W"; autoRepeat: false; onActivated: closeActivePane() }
    Shortcut { sequence: "Ctrl+Shift+D"; autoRepeat: false; onActivated: splitActive(Qt.Horizontal) }
    Shortcut { sequence: "Ctrl+Shift+E"; autoRepeat: false; onActivated: splitActive(Qt.Vertical) }
    Shortcut { sequence: "Ctrl+Shift+C"; onActivated: if (activeView) activeView.copy() }
    Shortcut { sequence: "Ctrl+Shift+V"; onActivated: if (activeView) activeView.paste() }
    Shortcut { sequence: "Shift+Insert"; onActivated: if (activeView) activeView.paste() }
    Shortcut { sequence: "Ctrl+Shift+A"; onActivated: if (activeView) activeView.selectAll() }
    Shortcut { sequence: "Ctrl+Shift+K"; autoRepeat: false; onActivated: if (activeView) activeView.clearScrollback() }
    Shortcut { sequence: "Ctrl+PgDown"; onActivated: focusTab((currentTab + 1) % tabs.length) }
    Shortcut { sequence: "Ctrl+PgUp"; onActivated: focusTab((currentTab + tabs.length - 1) % tabs.length) }
    Shortcut { sequence: "Ctrl+Tab"; onActivated: focusTab((currentTab + 1) % tabs.length) }
    Shortcut { sequence: "Ctrl+Shift+Tab"; onActivated: focusTab((currentTab + tabs.length - 1) % tabs.length) }
    Shortcut { sequence: "Ctrl+Alt+Left"; onActivated: moveFocus(-1, 0) }
    Shortcut { sequence: "Ctrl+Alt+Right"; onActivated: moveFocus(1, 0) }
    Shortcut { sequence: "Ctrl+Alt+Up"; onActivated: moveFocus(0, -1) }
    Shortcut { sequence: "Ctrl+Alt+Down"; onActivated: moveFocus(0, 1) }
    Shortcut { sequences: ["Ctrl+=", "Ctrl++"]; onActivated: zoomBy(1) }
    Shortcut { sequence: "Ctrl+-"; onActivated: zoomBy(-1) }
    Shortcut { sequence: "Ctrl+0"; onActivated: fontSize = 0 }
    Shortcut { sequence: "Ctrl+,"; onActivated: prefs.showPage("") }
    Shortcut { sequence: "Ctrl+Shift+Q"; autoRepeat: false; onActivated: Qt.quit() }
    Repeater {
        model: 9
        delegate: Item {
            Shortcut { sequence: "Alt+" + (index + 1); onActivated: focusTab(index) }
        }
    }

    // ---- layout ----------------------------------------------------------

    ColumnLayout {
        id: rootLayout
        anchors.fill: parent
        // A thousandth of a pixel keeps text off exact half pixels at 130%.
        anchors.topMargin: 0.001
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            color: Theme.chrome
            visible: settings.always_show_tabs || tabs.length > 1 || tabs.some(t => t.tmuxWindow >= 0)

            // ☰ The app menu.
            Rectangle {
                id: menuButton
                anchors.left: parent.left
                anchors.leftMargin: 6
                anchors.verticalCenter: tabRow.verticalCenter
                width: 30; height: 28; radius: 6
                color: menuHover.hovered ? Theme.tabHover : "transparent"
                Item {
                    anchors.centerIn: parent
                    width: 16; height: 12
                    Repeater {
                        model: 3
                        Rectangle { y: index * 5; width: 16; height: 2; radius: 1; color: Theme.textDim }
                    }
                }
                HoverHandler { id: menuHover }
                TapHandler { onTapped: appMenu.open(menuButton) }
            }

            // Tabs sit on the bottom edge so the current one joins the
            // terminal; neighbours overlap by their flares.
            Row {
                id: tabRow
                anchors.left: menuButton.right
                anchors.leftMargin: 2
                anchors.bottom: parent.bottom
                spacing: -8
                readonly property real tabWidth: Math.max(90, Math.min(240,
                    (parent.width - 100 + 8 * tabs.length) / Math.max(1, tabs.length)))
                Repeater {
                    model: tabs
                    delegate: TabButtonX {
                        width: tabRow.tabWidth
                        z: index === currentTab ? 2 : (hovered ? 1 : 0)
                        text: modelData.tabTitle
                        current: index === currentTab
                        nextIsCurrent: index + 1 === currentTab
                        tmux: modelData.tmuxWindow >= 0
                        surface: Theme.bg
                        onClicked: focusTab(index)
                        onCloseClicked: closeTab(modelData)
                    }
                }
            }
            Rectangle {
                anchors.left: tabRow.right
                anchors.leftMargin: 4
                anchors.verticalCenter: tabRow.verticalCenter
                width: 26; height: 26; radius: 13
                color: plusHover.hovered ? Theme.tabHover : "transparent"
                Text { anchors.centerIn: parent; anchors.verticalCenterOffset: -1; text: "+"; color: Theme.textDim; font.pixelSize: 17 }
                HoverHandler { id: plusHover }
                TapHandler { onTapped: newTab({}) }
            }
        }

        Item {
            id: stack
            Layout.fillWidth: true
            Layout.fillHeight: true
        }
    }

    // ---- startup -----------------------------------------------------------

    Component.onCompleted: {
        let restore = []
        try { restore = JSON.parse(controller.restorableSessions()) } catch (e) {}
        if (restore.length > 0) {
            for (const id of restore) newTab({ attachId: id })
        } else {
            newTab({})
        }
        const cmd = controller.startupCommand()
        if (cmd !== "") runTimer.start()
        if (controller.thenCommand() !== "") thenTimer.start()
        if (controller.prefsPage() !== "") prefs.showPage(controller.prefsPage())
        if (controller.screenshotPath() !== "") shotTimer.start()
    }

    Timer {
        id: runTimer
        interval: 600
        onTriggered: if (activeView) activeView.sendText(controller.startupCommand() + "\n")
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
