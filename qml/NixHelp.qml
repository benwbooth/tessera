import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Tessera

// What Nix is and how to install it: shown when something Tessera fetches
// with Nix (a kernel, gdu, an editor, an archive tool) can't run because Nix
// isn't installed. On Windows, Nix runs inside WSL 2, so it's the steps for
// that instead.
Popup {
    id: help
    // (The program that needed it, for the first line; "" for none.)
    property string what: ""
    property bool windows: Qt.platform.os === "windows"
    // (The host it's missing on: "" for this computer.)
    property string host: ""
    signal installRequested(string command, string host)

    readonly property string installCmd: "curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix | sh -s -- install"

    function show(what_, host_) {
        what = what_ || ""
        host = host_ || ""
        windows = Qt.platform.os === "windows" && host === ""
        open()
    }
    function copy(t) { clip.text = t; clip.selectAll(); clip.copy(); copied.restart() }
    TextEdit { id: clip; visible: false }
    Timer { id: copied; interval: 1500 }

    parent: Overlay.overlay
    anchors.centerIn: parent
    width: Math.min(520, parent ? parent.width - 40 : 520)
    modal: true
    focus: true
    padding: 0
    topPadding: 0.001   // (off the half pixel: 130% scale smears native text there)
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside
    Overlay.modal: Rectangle { color: Qt.rgba(0, 0, 0, Theme.dark ? 0.45 : 0.25) }
    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: 120 }
        NumberAnimation { property: "scale"; from: 0.96; to: 1; duration: 140; easing.type: Easing.OutCubic }
    }
    exit: Transition { NumberAnimation { property: "opacity"; to: 0; duration: 90 } }
    background: Rectangle {
        radius: 12
        color: Theme.panel
        border.color: Theme.divider
        Rectangle { z: -1; anchors.fill: parent; anchors.margins: -1; anchors.topMargin: 3; radius: 13; color: "black"; opacity: 0.3 }
    }

    component Para: Text {
        Layout.fillWidth: true
        Layout.leftMargin: 20
        Layout.rightMargin: 20
        Layout.bottomMargin: 10
        textFormat: Text.PlainText
        color: Theme.textDim
        font.pixelSize: 13
        wrapMode: Text.WordWrap
        renderType: Text.NativeRendering
    }
    // (A command to copy: monospace, with its Copy button.)
    component Code: Rectangle {
        id: code
        property string cmd: ""
        Layout.fillWidth: true
        Layout.leftMargin: 20
        Layout.rightMargin: 20
        Layout.bottomMargin: 12
        implicitHeight: codeText.implicitHeight + 16
        radius: 7
        color: Theme.field
        border.color: Theme.divider
        TextEdit {
            id: codeText
            x: 10; y: 8.001
            width: parent.width - 20 - copyChip.width - 8
            text: code.cmd
            readOnly: true
            selectByMouse: true
            wrapMode: TextEdit.WrapAnywhere
            color: Theme.text
            selectionColor: Theme.accent
            font.family: "monospace"
            font.pixelSize: 12
            renderType: Text.NativeRendering
        }
        Chip {
            id: copyChip
            anchors.right: parent.right
            anchors.rightMargin: 6
            anchors.verticalCenter: parent.verticalCenter
            base: Theme.field
            label: copied.running && help.lastCopied === code.cmd ? "Copied" : "Copy"
            onClicked: { help.lastCopied = code.cmd; help.copy(code.cmd) }
        }
    }
    property string lastCopied: ""

    contentItem: ColumnLayout {
        spacing: 0
        Text {
            Layout.fillWidth: true
            Layout.margins: 20
            Layout.bottomMargin: 8
            text: (help.what !== "" ? help.what + " needs Nix, which isn't installed" : "Nix isn't installed") + (help.host !== "" ? " on " + help.host : "")
            color: Theme.text
            font.pixelSize: 15
            font.weight: Font.DemiBold
            wrapMode: Text.WordWrap
            renderType: Text.NativeRendering
        }
        Para {
            text: "Nix is a package manager. Tessera uses it to fetch programs you don't have yet (language kernels, gdu, editors, archive tools) the first time you use them. "
                + "They're kept apart from your system's own packages, so nothing else changes, and Nix can be removed again later."
        }
        // ---- Linux and macOS ----
        Para { visible: !help.windows; text: "To install it, run this in a terminal. It asks for your password, shows what it will do, and takes a minute or two:" }
        Code { visible: !help.windows; cmd: help.installCmd }
        Para { visible: !help.windows; text: "Then open a new tab, so the shell finds nix." }
        // ---- Windows: in WSL 2 ----
        Para { visible: help.windows; text: "Nix doesn't run on Windows itself: it runs in WSL 2 (Linux inside Windows). Once it's installed there, Tessera's WSL tabs fetch programs with it.\n\n1. Install WSL in PowerShell as administrator, then restart Windows:" }
        Code { visible: help.windows; cmd: "wsl --install" }
        Para { visible: help.windows; text: "2. Open a WSL tab (the + menu lists each distribution) and install Nix there:" }
        Code { visible: help.windows; cmd: help.installCmd }

        Rectangle { Layout.fillWidth: true; Layout.topMargin: 4; height: 1; color: Theme.divider; opacity: 0.6 }
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: 12
            spacing: 8
            component Btn: Rectangle {
                id: b
                property alias text: bt.text
                property bool primary: false
                property string tip: ""
                signal hit()
                implicitWidth: Math.max(84, bt.implicitWidth + 28)
                implicitHeight: 30
                radius: 8
                color: primary ? (bh.hovered ? Qt.lighter(Theme.accent, 1.1) : Theme.accent) : (bh.hovered ? Theme.tabHover : Theme.field)
                border.color: primary ? "transparent" : Theme.divider
                Text { id: bt; anchors.centerIn: parent; anchors.verticalCenterOffset: 0.001; color: b.primary ? "white" : Theme.text; font.pixelSize: 13; font.weight: b.primary ? Font.DemiBold : Font.Normal; renderType: Text.NativeRendering }
                HoverHandler { id: bh; cursorShape: Qt.PointingHandCursor }
                TapHandler { onTapped: b.hit() }
                ToolTip.visible: tip !== "" && bh.hovered
                ToolTip.delay: 500
                ToolTip.text: tip
            }
            Btn { text: "Learn more"; tip: "nixos.org: what Nix is, and other ways to install it"; onHit: Qt.openUrlExternally("https://nixos.org/download/") }
            Item { Layout.fillWidth: true }
            Btn { text: "Close"; onHit: help.close() }
            Btn {
                visible: !help.windows
                primary: true
                text: "Install in a new tab"
                tip: "Runs the installer above in a new terminal tab, where it asks before changing anything"
                onHit: { help.close(); help.installRequested(help.installCmd, help.host) }
            }
        }
    }
}
