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
    // (A language with more than one guide: the one chosen, "" for its first.)
    property string guideId: ""
    readonly property string key: guideId !== "" ? guideId : lang
    readonly property var choices: backend ? JSON.parse(backend.guideChoices(lang) || "[]") : []
    signal chose(string id, string title)
    // (A hit in another guide, from All Guides: that guide, at the words.)
    signal openOther(string target, string query)
    property bool showingSearch: false
    function searchAll() {
        const q = findField.text.trim()
        if (q === "" || !backend) return
        showingSearch = true
        Async.call(backend, k => backend.guidesSearch(k, q), html => {
            web.loadHtml(shell("<nav><div class='title'>Search</div><a class='part' href='tessera-guide:back'>\u2190 Back to " + page.title.replace(/</g, "&lt;") + "</a></nav>" + html), "about:blank")
        })
    }
    property string fallbackTitle: lang === "tessh" ? "tessh guide" : lang + " guide"
    readonly property string title: {
        const c = choices.find(x => x[0] === key) || (guideId === "" && choices.length ? choices[0] : null)
        return c ? c[1] : fallbackTitle
    }
    color: Theme.bg
    function find(text) { findField.text = text; findField.forceActiveFocus() }
    function focusFind() { findField.forceActiveFocus() }
    // (Test hook: a touchpad swipe, ten 30 px moves 16 ms apart, then how
    // far it went by the end and after the glide.)
    function testWheel() {
        web.runJavaScript("(()=>{const m=document.querySelector('main');const y0=m.scrollTop;let n=0;const t=setInterval(()=>{m.dispatchEvent(new WheelEvent('wheel',{deltaY:30,deltaMode:0,bubbles:true,cancelable:true}));"
            + "if(++n===10){clearInterval(t);const y1=m.scrollTop;setTimeout(()=>console.log('tessera-wheel-test: swipe '+(y1-y0)+' px, after glide '+(m.scrollTop-y0)+' px'),1500)}},16)})()")
    }

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
            // (What's off screen isn't laid out: a long guide's tab or a find
            // relays out only what shows, not the whole page.)
            + "main>*{content-visibility:auto;contain-intrinsic-size:auto 80px}"
            + "pre.tessh{border-left:3px solid " + c(Theme.accent) + "}"
            // (The Advanced guide's examples: bash and tessh tabs.)
            + ".ex-tabs{margin:.9em 0;position:relative}.ex-tabs>input{position:absolute;opacity:0;pointer-events:none}"
            + ".ex-tabs>label{display:inline-block;padding:3px 14px;margin-right:3px;border-radius:7px 7px 0 0;font-size:12px;cursor:pointer;color:" + c(Theme.textDim) + "}"
            + ".ex-tabs>label:hover{color:" + c(Theme.text) + "}"
            + ".ex-tabs>input:checked+label{background:" + c(Theme.field) + ";color:" + c(Theme.fg) + ";font-weight:bold}"
            + ".ex-pane{display:none}.ex-r-bash:checked~.ex-bash,.ex-r-tessh:checked~.ex-tessh{display:block}"
            + ".ex-pane>pre:first-child{margin-top:0;border-top-left-radius:0}"
            + "pre.output{background:transparent;border:1px dashed " + c(Theme.divider) + ";color:" + c(Theme.textDim) + ";margin-top:-6px}"
            + "pre.output::before{content:'output';display:block;font-size:10px;letter-spacing:.08em;text-transform:uppercase;opacity:.7;margin-bottom:2px}"
            + ".t-keyword{color:" + col(5, "#c678dd") + "}.t-string{color:" + col(2, "#98c379") + "}"
            + ".t-number,.t-constant,.t-attribute{color:" + col(3, "#e5c07b") + "}.t-function{color:" + col(4, "#61afef") + "}"
            + ".t-comment{color:" + col(8, "#5c6370") + ";font-style:italic}.t-operator,.t-property{color:" + col(6, "#56b6c2") + "}"
            + ".t-variable{color:" + col(1, "#e06c75") + "}.t-punctuation{color:" + c(Theme.textDim) + "}"
            + ".t-embedded{color:" + col(13, "#c678dd") + ";font-weight:bold}.t-escape{color:" + col(6, "#56b6c2") + "}"
            + ".t-type,.t-constructor,.t-tag,.t-label,.t-namespace,.t-module{color:" + col(3, "#e5c07b") + "}"
            + "main li{margin:.5em 0}.hit{color:" + c(Theme.textDim) + ";font-size:13px}.note{color:" + c(Theme.textDim) + "}"
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
    // (While it's fetched: what it's doing, asked every 300 ms; a quick read.)
    property string progress: ""
    Timer {
        interval: 300; repeat: true; running: page.busy && page.lang !== "tessh"
        onTriggered: {
            const p = page.backend ? page.backend.guideProgress(page.key) : ""
            if (p === page.progress) return
            page.progress = p
            web.runJavaScript("(()=>{const e=document.getElementById('prog');if(e)e.textContent=" + JSON.stringify(p) + "})()")
        }
    }
    function head() {
        return "<style id='tessera-theme'>" + page.css() + "</style>"
            // (Links in the guide go to their heading; others open in the browser.)
            + "<script>document.addEventListener('click',e=>{const a=e.target.closest('a[href]');if(!a)return;const h=a.getAttribute('href');e.preventDefault();"
            + "if(h.startsWith('#')){const t=document.getElementById(decodeURIComponent(h.slice(1)));if(t)t.scrollIntoView({block:'start'})}else if(/^(https?|mailto|tessera-guide):/.test(h))window.open(h)})</script>"
            + "<script>" + momentum + "</script>"
    }
    // Momentum scrolling in the page, as MomentumWheel does it in QML: a
    // wheel notch eases 120 px to a set destination (quicker for longer
    // runs, a notch mid-glide extends it); a touchpad follows the fingers,
    // then glides on, slowing steadily, once they lift (no events for 60 ms).
    readonly property string momentum: "(()=>{const S=new Map(),DEC=2300,MAXV=9000;"
        + "function box(t){for(let e=t instanceof Element?t:t.parentElement;e;e=e.parentElement){const cs=getComputedStyle(e);if((cs.overflowY==='auto'||cs.overflowY==='scroll')&&e.scrollHeight>e.clientHeight+1)return e}return document.scrollingElement}"
        + "function st(e){let s=S.get(e);if(!s){s={pos:e.scrollTop,v:0,anim:false,target:0,start:0,v0:0,t:0,dur:.2,last:0,pv:0,raf:0,pad:0};S.set(e,s)}return s}"
        + "function hi(e){return Math.max(0,e.scrollHeight-e.clientHeight)}"
        + "function put(e,s,y){s.pos=Math.max(0,Math.min(hi(e),y));e.scrollTop=s.pos}"
        + "function stop(s){s.v=0;s.anim=false;cancelAnimationFrame(s.raf);s.raf=0;clearTimeout(s.pad);s.last=0;s.pv=0}"
        + "function run(e,s){if(s.raf)return;let prev=performance.now();const f=now=>{const dt=(now-prev)/1000;prev=now;s.raf=0;if(step(e,s,dt))s.raf=requestAnimationFrame(f)};s.raf=requestAnimationFrame(f)}"
        + "function step(e,s,dt){if(s.anim){s.t+=Math.max(0,dt);const p=Math.min(1,s.t/s.dur),t2=p*p,t3=t2*p,span=s.target-s.start,v0=s.v0*s.dur;"
        + "put(e,s,s.start+v0*(t3-2*t2+p)+span*(3*t2-2*t3));s.v=p>=1?0:(v0*(3*t2-4*p+1)+span*(6*p-6*t2))/s.dur;if(p>=1||s.pos<=0||s.pos>=hi(e)){stop(s);return false}return true}"
        + "dt=Math.min(.05,dt);if(dt<=0)return true;const want=s.pos+s.v*dt;put(e,s,want);const sp=Math.max(0,Math.abs(s.v)-DEC*dt);s.v=Math.sign(s.v)*sp;"
        + "if(sp<1||s.pos!==want){stop(s);return false}return true}"
        + "function notches(e,s,steps){const d=-steps*120,cur=s.pos,pend=s.anim?s.target-cur:0,origin=pend*d>0?s.target:cur;"
        + "s.target=Math.max(0,Math.min(hi(e),origin+d));clearTimeout(s.pad);s.last=0;s.pv=0;if(s.target===cur){stop(s);return}"
        + "const dist=Math.abs(s.target-cur);s.dur=Math.max(6,Math.min(12,14-dist/60))/60;s.v0=s.anim&&s.v*(s.target-cur)>0?s.v:0;s.start=cur;s.t=0;s.anim=true;run(e,s)}"
        + "function pixels(e,s,dy){const now=performance.now(),el=s.last>0?(now-s.last)/1000:0;cancelAnimationFrame(s.raf);s.raf=0;s.anim=false;s.v=0;put(e,s,s.pos+dy);"
        + "if(el>=.004&&el<=.08){const v=dy/el;s.pv=s.pv===0?v:s.pv*.65+v*.35}else s.pv=dy*60;s.pv=Math.max(-MAXV,Math.min(MAXV,s.pv));s.last=now;"
        + "clearTimeout(s.pad);s.pad=setTimeout(()=>{s.last=0;s.v=s.pv;s.pv=0;if(Math.abs(s.v)<40){stop(s);return}run(e,s)},60)}"
        + "addEventListener('wheel',ev=>{if(ev.ctrlKey||Math.abs(ev.deltaX)>Math.abs(ev.deltaY))return;const e=box(ev.target);if(!e||hi(e)<=0)return;ev.preventDefault();const s=st(e);"
        + "if(!s.anim&&!s.raf&&!s.last)s.pos=e.scrollTop;"
        + "const w=ev.wheelDeltaY||0;if(ev.deltaMode===0&&w!==0&&w%120===0)notches(e,s,w/120);else pixels(e,s,ev.deltaMode===1?ev.deltaY*40:ev.deltaY)},{passive:false});"
        + "addEventListener('keydown',()=>S.forEach(stop));addEventListener('mousedown',()=>S.forEach(s=>{stop(s)}))})();"
    // (The theme changed, or the profile's colours or font: the open page
    // takes the new style at once, nothing loaded again.)
    readonly property string liveCss: css()
    onLiveCssChanged: web.runJavaScript("(()=>{const s=document.getElementById('tessera-theme');if(s)s.textContent=" + JSON.stringify(liveCss) + "})()")
    function shell(body) { return "<!doctype html><html><head><meta charset='utf-8'>" + head() + "</head><body>" + body + "</body></html>" }
    // (Another guide chosen while one is fetched: that one when it's done.)
    property string loadingKey: ""
    onKeyChanged: if (asked) { asked = false; error = ""; if (!busy) load() }
    function load(refresh) {
        if (!backend || busy || (asked && !refresh)) return
        asked = true
        loadingKey = key
        busy = true
        error = ""
        if (lang !== "tessh")
            web.loadHtml(shell("<div class='msg'>Fetching <b>" + page.title
                + "</b> from its official site and converting it to Markdown…<br><br><span id='prog'></span><br><br>It's kept for next time (and offline).</div>"), "about:blank")
        // (Written to a file and loaded from it: loadHtml shows at most 2 MB.)
        Async.call(backend, k => backend.guidePage(k, page.key, !!refresh, head()), url => {
            busy = false
            progress = ""
            if (loadingKey !== key) { asked = false; load(); return }
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
                    text: page.key === "tessh-advanced" ? "The ABS Guide's examples, bash beside tessh; help advanced in tessh lists it" : page.lang === "tessh" ? "help TOPIC in tessh shows one part" : page.busy ? (page.progress || "Fetching…") : "From the language's own documentation; source and license at the top"
                    color: Theme.textDim; font.pixelSize: 12; renderType: Text.NativeRendering; visible: page.width >= 900
                }
                Item { Layout.fillWidth: true }
                // (More than one guide for the language: which one.)
                Repeater {
                    model: page.choices.length > 1 ? page.choices : []
                    Chip {
                        required property var modelData
                        required property int index
                        readonly property bool current: modelData[0] === page.key || (page.guideId === "" && index === 0)
                        base: current ? Theme.mix(Theme.panel, Theme.accent, 0.35) : Theme.panel
                        label: modelData[1]
                        tip: current ? "The guide shown" : "Show " + modelData[1] + " instead (kept apart: each is fetched once)"
                        onClicked: if (!current) page.chose(modelData[0], modelData[1])
                    }
                }
                Chip {
                    visible: page.lang !== "tessh"
                    base: Theme.panel
                    label: page.error !== "" ? "Retry" : "↻ Refresh"
                    tip: "Fetch the guide again from its site (it's kept, so it isn't fetched every time)"
                    onClicked: page.load(true)
                }
                Chip {
                    base: Theme.panel
                    label: "All Guides"
                    tip: "Look for the words in every kept guide, not just this one (Ctrl+Enter in the find box)"
                    onClicked: page.searchAll()
                }
                TextField {
                    id: findField
                    Layout.preferredWidth: 240
                    placeholderText: "Find in the guide (Enter: next)"
                    color: Theme.text
                    font.pixelSize: 12
                    background: Rectangle { radius: 6; color: Theme.field; border.color: findField.activeFocus ? Theme.accent : Theme.divider }
                    onTextChanged: web.findText(text)
                    Keys.onReturnPressed: (e) => {
                        if (e.modifiers & Qt.ControlModifier) page.searchAll()
                        else web.findText(text, (e.modifiers & Qt.ShiftModifier) ? WebEngineView.FindBackward : 0)
                    }
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
            onNewWindowRequested: (req) => {
                const u = req.requestedUrl.toString()
                if (u === "tessera-guide:back") { page.showingSearch = false; page.asked = false; page.load() }
                else if (u.startsWith("tessera-guide:")) {
                    const rest = u.slice(14), i = rest.indexOf("?q=")
                    const target = i < 0 ? rest : rest.slice(0, i), q = i < 0 ? "" : decodeURIComponent(rest.slice(i + 3))
                    // (This guide: back to it, at the words; another: its tab.)
                    const [l, id] = target.split("/")
                    const shown = page.guideId !== "" ? page.guideId : (page.choices.length ? page.choices[0][0] : page.lang)
                    if (l === page.lang && (id || shown) === shown) {
                        page.showingSearch = false; page.asked = false; page.load(); page.find(q)
                    } else page.openOther(target, q)
                } else Qt.openUrlExternally(req.requestedUrl)
            }
            // (The page's own test lines, to the log.)
            onJavaScriptConsoleMessage: (level, message) => { if (message.startsWith("tessera-")) console.log(message) }
            // (Nothing navigates the page away: links open in the browser.)
            onNavigationRequested: (req) => { if (req.navigationType === WebEngineNavigationRequest.LinkClickedNavigation) { req.reject(); Qt.openUrlExternally(req.url) } }
            onLoadingChanged: (r) => { if (r.status === WebEngineView.LoadSucceededStatus && findField.text !== "") findText(findField.text) }
        }
    }
}
