import QtQuick
import QtQuick.Controls
import QtQuick.Templates as T

// A compact popup menu (like SlimCombo's list): 24 px rows, a check mark for
// the chosen one. Fill it with SlimMenuItem.
T.Menu {
    id: menu
    implicitWidth: Math.max(140, contentItem.implicitWidth + leftPadding + rightPadding)
    implicitHeight: contentItem.implicitHeight + topPadding + bottomPadding
    padding: 4
    topPadding: 4.001   // (off the half pixel: 130% scale smears native text there)
    // (Where it opens, on whole pixels plus the nudge too: a chip it opens
    // from can sit at a fraction of one.)
    onAboutToShow: {
        if (!parent) return
        const p = parent.mapToItem(null, x, y)
        x += Math.round(p.x) - p.x + 0.001
        y += Math.round(p.y) - p.y + 0.001
    }
    margins: 4
    delegate: SlimMenuItem {}
    contentItem: ListView {
        implicitHeight: contentHeight
        implicitWidth: {
            let w = 0
            for (let i = 0; i < count; i++) {
                const it = itemAtIndex(i)
                if (it) w = Math.max(w, it.implicitWidth)
            }
            return w
        }
        model: menu.contentModel
        interactive: Window.window ? contentHeight + menu.topPadding + menu.bottomPadding > menu.height : false
        clip: true
        currentIndex: menu.currentIndex
    }
    background: Rectangle {
        radius: 7
        color: Theme.mix(Theme.bg, Theme.fg, Theme.dark ? 0.08 : 0.03)
        border.color: Theme.divider
    }
}
