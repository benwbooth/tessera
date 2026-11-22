import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtWebEngine
import Tessera

// A language's guide as a page, a tab of its own: contents on the left,
// find above, code highlighted. tessh's is built in (tessh/doc/guide.md);
// the others are the languages' own documentation, fetched the first time
// and kept (guides.rs).
Rectangle {
    id: page
    property var backend: null    // the Controller
    property var ansi: []
    property string fontFamily: "monospace"
    property string lang: "tessh"
    property string title: lang === "tessh" ? "tessh guide" : lang + " guide"
    color: Theme.bg
    function find(text) { findField.text = text; findField.forceActiveFocus() }
    function focusFind() { findField.forceActiveFocus() }

    function css() {
        const c = (x) => String(x), a = ansi
        const col = (n, d) => c(a[n] || d)
        return "body{background:" + c(Theme.bg) + ";color:" + c(Theme.text) + ";font:14px/1.6 'Noto Sans','Segoe UI',system-ui,sans-serif;margin:0;display:flex;height:100vh;overflow:hidden}"
            + "a{color:" + c(Theme.accent) + "}h1,h2{border-bottom:1px solid " + c(Theme.divider) + ";padding-bottom:.25em}"
            + "h1{font-size:1.8em}h2{font-size:1.4em}h1,h2,h3,h4{margin:1.1em 0 .5em;line-height:1.25}"
            + "code{font-family:" + fontFamily + ",monospace;font-size:.88em;background:" + c(Theme.field) + ";padding:.15em .35em;border-radius:5px}"
            + "pre{background:" + c(Theme.field) + ";padding:10px 12px;border-radius:7px;overflow:auto}pre code{background:none;padding:0}"
            + "table{border-collapse:collapse}th,td{border:1px solid " + c(Theme.divider) + ";padding:5px 10px}th{background:" + c(Theme.field) + "}"
            + "::-webkit-scrollbar{width:9px;height:9px}::-webkit-scrollbar-thumb{background:" + c(Theme.divider) + ";border-radius:5px}"
            + "nav{width:230px;flex:none;overflow:auto;padding:14px 10px 30px 14px;border-right:1px solid " + c(Theme.divider) + ";font-size:13px;line-height:1.35}"
            + "nav .title{font-weight:bold;font-size:15px;margin-bottom:10px;color:" + c(Theme.fg) + "}"
            + "nav a{display:block;color:" + c(Theme.text) + ";text-decoration:none;padding:2px 6px;border-radius:5px}"
            + "nav a:hover{background:" + c(Theme.field) + "}"
            + "nav a.part{font-weight:bold;margin-top:10px;color:" + c(Theme.fg) + "}"
            + "nav .topics a{padding-left:14px;color:" + c(Theme.textDim) + "}"
            + "main{flex:1;overflow:auto;padding:16px 28px 60px}main>*{max-width:860px}"
            + "pre.tessh{border-left:3px solid " + c(Theme.accent) + "}"
            + "pre.output{background:transparent;border:1px dashed " + c(Theme.divider) + ";color:" + c(Theme.textDim) + ";margin-top:-6px}"
            + "pre.output::before{content:'output';display:block;font-size:10px;letter-spacing:.08em;text-transform:uppercase;opacity:.7;margin-bottom:2px}"
            + ".t-keyword{color:" + col(5, "#c678dd") + "}.t-string{color:" + col(2, "#98c379") + "}"
            + ".t-number,.t-constant,.t-attribute{color:" + col(3, "#e5c07b") + "}.t-function{color:" + col(4, "#61afef") + "}"
            + ".t-comment{color:" + col(8, "#5c6370") + ";font-style:italic}.t-operator,.t-property{color:" + col(6, "#56b6c2") + "}"
            + ".t-variable{color:" + col(1, "#e06c75") + "}.t-punctuation{color:" + c(Theme.textDim) + "}"
            + ".t-embedded{color:" + col(13, "#c678dd") + ";font-weight:bold}.t-escape{color:" + col(6, "#56b6c2") + "}"
            + ".t-type,.t-constructor,.t-tag,.t-label,.t-namespace,.t-module{color:" + col(3, "#e5c07b") + "}"
            + "img{max-width:100%}blockquote{margin:.6em 0;padding:.2em 1em;border-left:3px solid " + c(Theme.divider) + ";color:" + c(Theme.textDim) + "}"
            + "main>blockquote:first-child{border-left-color:" + c(Theme.accent) + ";font-size:12.5px}"
            + ".msg{margin:40px auto;max-width:520px;color:" + c(Theme.textDim) + ";font-size:15px;text-align:center}"
            + "h1:target,h2:target,h3:target{color:" + c(Theme.accent) + "}"
    }
    // (The page is made in the background, then shown; a guide that isn't
    // kept yet is fetched first, which takes a few seconds.)
    property bool asked: false
    property bool busy: false
    property string error: ""
    function head() {
        return "<style>" + page.css() + "</style>"
            // (Links in the guide go to their heading; others open in the browser.)
            + "<script>document.addEventListener('click',e=>{const a=e.target.closest('a[href]');if(!a)return;const h=a.getAttribute('href');e.preventDefault();"
            + "if(h.startsWith('#')){const t=document.getElementById(decodeURIComponent(h.slice(1)));if(t)t.scrollIntoView({block:'start'})}else if(/^(https?|mailto):/.test(h))window.open(h)})</script>"
    }
    function shell(body) { return "<!doctype html><html><head><meta charset='utf-8'>" + head() + "</head><body>" + body + "</body></html>" }
    function load(refresh) {
        if (!backend || busy || (asked && !refresh)) return
        asked = true
        busy = true
        error = ""
        if (lang !== "tessh")
            web.loadHtml(shell("<div class='msg'>Fetching <b>" + page.title
                + "</b> from its official site and converting it to Markdown…<br><br>It's kept for next time (and offline).</div>"), "about:blank")
        // (Written to a file and loaded from it: loadHtml shows at most 2 MB.)
        Async.call(backend, k => backend.guidePage(k, lang, !!refresh, head()), url => {
            busy = false
            if (url.startsWith("error: ")) {
                error = url.slice(7)
                web.loadHtml(shell("<div class='msg'>The " + page.title + " couldn't be fetched:<br><br><code>"
                    + error.replace(/&/g, "&amp;").replace(/</g, "&lt;") + "</code><br><br>Check the connection, then Retry.</div>"), "about:blank")
                return
            }
            web.url = url + "?v=" + Date.now()
        })
    }
    onBackendChanged: Qt.callLater(load)
    Component.onCompleted: Qt.callLater(load)

    ColumnLayout {
        anchors.fill: parent
        spacing: 0
        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 40
            color: Theme.panel
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 8
                spacing: 10
                Text { topPadding: 0.001; text: page.title; color: Theme.fg; font.pixelSize: 14; font.bold: true; renderType: Text.NativeRendering; elide: Text.ElideRight; Layout.maximumWidth: page.width * 0.4 }
                Text {
                    topPadding: 0.001
                    text: page.lang === "tessh" ? "help TOPIC in tessh shows one part" : page.busy ? "Fetching…" : "From the language's own documentation; source and license at the top"
                    color: Theme.textDim; font.pixelSize: 12; renderType: Text.NativeRendering; visible: page.width >= 900
                }
                Item { Layout.fillWidth: true }
                Chip {
                    visible: page.lang !== "tessh"
                    base: Theme.panel
                    label: page.error !== "" ? "Retry" : "↻ Refresh"
                    tip: "Fetch the guide again from its site (it's kept, so it isn't fetched every time)"
                    onClicked: page.load(true)
                }
                TextField {
                    id: findField
                    Layout.preferredWidth: 240
                    placeholderText: "Find in the guide (Enter: next)"
                    color: Theme.text
                    font.pixelSize: 12
                    background: Rectangle { radius: 6; color: Theme.field; border.color: findField.activeFocus ? Theme.accent : Theme.divider }
                    onTextChanged: web.findText(text)
                    Keys.onReturnPressed: (e) => web.findText(text, (e.modifiers & Qt.ShiftModifier) ? WebEngineView.FindBackward : 0)
                    Keys.onEscapePressed: { text = ""; web.forceActiveFocus() }
                }
            }
            Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: Theme.divider }
        }
        WebEngineView {
            id: web
            Layout.fillWidth: true
            Layout.fillHeight: true
            backgroundColor: Theme.bg
            settings.pluginsEnabled: false
            settings.localContentCanAccessFileUrls: false
            // (The page is a file; its pictures are on the web.)
            settings.localContentCanAccessRemoteUrls: true
            onNewWindowRequested: (req) => Qt.openUrlExternally(req.requestedUrl)
            // (Nothing navigates the page away: links open in the browser.)
            onNavigationRequested: (req) => { if (req.navigationType === WebEngineNavigationRequest.LinkClickedNavigation) { req.reject(); Qt.openUrlExternally(req.url) } }
            onLoadingChanged: (r) => { if (r.status === WebEngineView.LoadSucceededStatus && findField.text !== "") findText(findField.text) }
        }
    }
}
