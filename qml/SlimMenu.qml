import QtQuick

// tessera's popup menus: the shared macOS 26 menu (MacMenu, from
// hearth/shared/app-menus) in tessera's theme. Fill it with SlimMenuItem
// (or MacMenuItem) and MacMenuSeparator.
MacMenu {
    dark: Theme.dark
    accent: Theme.accent
    // (A submenu that doesn't apply, disabled, isn't shown at all: menus
    // show only what applies.)
    delegate: SlimMenuItem { visible: !subMenu || subMenu.enabled }
}
