pragma Singleton
import QtQuick

// UI colors, derived from the terminal's color theme so the window chrome
// always matches the terminal (dark or light).
QtObject {
    id: theme
    property var term: ({
        background: "#1b1d20", foreground: "#d7dade",
        ansi: ["#2a2d32", "#ef6b73", "#8fcf7a", "#e8c46b", "#62a8f0", "#c792ea", "#5cc8c8", "#c9ccd1",
               "#5c636e", "#ff8a92", "#a9e392", "#f5d68a", "#86bdf7", "#d8acf2", "#7fdede", "#f2f4f7"]
    })

    function mix(a, b, t) {
        a = Qt.color(a); b = Qt.color(b)
        return Qt.rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1)
    }
    function luma(c) { c = Qt.color(c); return 0.299 * c.r + 0.587 * c.g + 0.114 * c.b }

    readonly property color bg: term.background
    readonly property color fg: term.foreground
    readonly property bool dark: luma(bg) < 0.5
    // The tab strip: a step darker than the terminal, or a step lighter when
    // the terminal is already (nearly) black.
    // Pure black (OLED themes) stays black, so the bar is black too.
    readonly property color chrome: luma(bg) < 0.002 ? bg
                                  : luma(bg) < 0.06 ? mix(bg, fg, 0.07)
                                  : dark ? mix(bg, "#000000", 0.2) : mix(bg, "#000000", 0.05)
    readonly property color tabHover: mix(chrome, fg, dark ? 0.06 : 0.045)
    readonly property color text: fg
    readonly property color textDim: mix(fg, chrome, 0.42)
    readonly property color divider: mix(chrome, fg, dark ? 0.12 : 0.13)
    readonly property color accent: term.ansi ? term.ansi[4] : "#5aa0ff"
    readonly property color tmux: term.ansi ? term.ansi[2] : "#57c78a"
    // Preferences window surfaces.
    readonly property color panel: dark ? mix(bg, fg, 0.05) : mix(bg, "#000000", 0.03)
    readonly property color field: dark ? mix(bg, fg, 0.10) : "#ffffff"
}
