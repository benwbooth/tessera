import QtQuick

// vi's ":" command line for a text field (a file card's editor, the input
// editor): the Ex commands VSCodeVim has that mean something for one text,
// plus :g and :v. `run(line)` does one and returns what to show under it
// ("" for nothing; `error` says whether it's a complaint). What belongs to
// the field's owner (writing, quitting, reading a file, an option) goes out
// as a signal.
//
// Ranges: N, ., $, %, 'x (a mark; '< '> the selection), /pat/ and ?pat?,
// each with +N/-N, joined by , or ;. Patterns are vim's (magic: \( \) \|
// \+ \? \= \{n,m} \< \>; \v very magic; \c and \C), smart case by default.
//
// Commands: :N (go to line), :s[ubstitute] (flags g c-less i I n &, and :&
// :s alone to repeat), :g[lobal] / :v[global] / :g!, :d[elete] [x], :y[ank],
// :pu[t][!], :m[ove], :t / :co[py], :j[oin][!], :> :<, :norm[al][!],
// :sor[t][!] (i n u x r /pat/), :ret[ab][!] [n], :le[ft] :ri[ght] :ce[nter],
// :ma[rk] / :k, :delm[arks][!], :marks, :reg[isters] / :di[splay],
// :p[rint] :nu[mber] :# :l[ist] :=, :f[ile], :u[ndo] :red[o],
// :noh[lsearch], :se[t] (wrap, ignorecase, smartcase, shiftwidth, tabstop,
// expandtab, number, hlsearch: some go to the owner), :ec[ho],
// :w[rite] / :up[date] / :sav[eas] / :w {file}, :wq, :x[it], :exi[t],
// :q[uit][!], :qa[ll], :wa[ll], :xa[ll], :wqa[ll], :e[dit][!] {file},
// :r[ead] {file}, :pw[d].
QtObject {
    id: ex
    property var edit: null                 // the TextEdit / TextArea
    property string indentUnit: "    "
    property bool ignoreCase: true
    property bool smartCase: true
    property int tabstop: 8
    property string killRing: ""            // the unnamed register (shared with the keys)
    property var registers: ({})            // named ones: a-z
    property var marks: ({})                // a-z: a line number
    property var history: []
    property string lastSearch: ""
    property var lastSub: null              // {pat, rep, flags}
    property string fileName: ""
    property string folder: ""
    // (How to read a file, for :r: set by the owner; returns its text or throws.)
    property var readFile: null
    // (The keys, for :normal: a function (key event) -> handled, and its
    // vi normal mode flag.)
    property var keyHandler: null
    property var viKeys: null

    signal write(string path, bool force)   // "" is the file itself
    signal quit(bool force)
    signal editFile(string path, bool force)  // "" with force: reload
    signal setOption(string name, var value)

    // ---- lines -------------------------------------------------------------
    // The text as line objects, so :g and :m keep track of lines as they move.
    property var ln: []
    // (A text ending in a newline has no empty line after it, as in vim: the
    // newline is the last line's end, put back when stored.)
    property bool endsWithNewline: false
    function load() {
        let t = edit.text
        endsWithNewline = t.endsWith("\n")
        if (endsWithNewline) t = t.substring(0, t.length - 1)
        ln = t.split("\n").map(s => ({ s: s }))
    }
    function store(cursorLine, col) {
        const t = ln.map(o => o.s).join("\n") + (endsWithNewline ? "\n" : "")
        if (t !== edit.text) {
            // (Only what changed, between the same start and end: a remove and
            // an insert, which undo takes as one step: see undo().)
            const o = edit.text
            let a = 0
            while (a < o.length && a < t.length && o[a] === t[a]) a++
            let b = 0
            while (b < o.length - a && b < t.length - a && o[o.length - 1 - b] === t[t.length - 1 - b]) b++
            const gone = o.length - a - b, put = t.substring(a, t.length - b)
            storing = true
            if (gone > 0) edit.remove(a, a + gone)
            if (put !== "") edit.insert(a, put)
            storing = false
            undoSteps = undoSteps.concat([gone > 0 && put !== "" ? 2 : 1])
            redoSteps = []
        }
        if (cursorLine !== undefined) {
            const n = Math.max(0, Math.min(ln.length - 1, cursorLine))
            let p = 0
            for (let i = 0; i < n; ++i) p += ln[i].s.length + 1
            const line = ln[n] ? ln[n].s : ""
            if (col === undefined) { col = 0; while (col < line.length && (line[col] === " " || line[col] === "\t")) col++ }
            edit.cursorPosition = Math.min(edit.length, p + Math.min(col, line.length))
        }
    }
    // ---- undo: a command's change is one step -----------------------------------
    // (The field's own undo has a remove and an insert for it: two steps. Each
    // of ours is remembered until something else changes the text.)
    property bool storing: false
    property var undoSteps: []
    property var redoSteps: []
    property var watch: Connections {
        target: ex.edit
        function onTextChanged() { if (!ex.storing && !ex.undoing) { ex.undoSteps = []; ex.redoSteps = [] } }
    }
    property bool undoing: false
    function undo() {
        undoing = true
        if (undoSteps.length) {
            const n = undoSteps[undoSteps.length - 1]
            undoSteps = undoSteps.slice(0, -1)
            for (let i = 0; i < n; ++i) edit.undo()
            redoSteps = redoSteps.concat([n])
        } else { edit.undo(); redoSteps = [] }
        undoing = false
    }
    function redo() {
        undoing = true
        if (redoSteps.length) {
            const n = redoSteps[redoSteps.length - 1]
            redoSteps = redoSteps.slice(0, -1)
            for (let i = 0; i < n; ++i) edit.redo()
            undoSteps = undoSteps.concat([n])
        } else edit.redo()
        undoing = false
    }
    function cursorLine() { return edit.text.substring(0, edit.cursorPosition).split("\n").length - 1 }
    function lineOfPos(p) { return edit.text.substring(0, p).split("\n").length - 1 }

    // ---- patterns ----------------------------------------------------------
    // vim's regex (magic) as a JS one.
    function regex(pat, flags) {
        let ic = ignoreCase
        if (pat.indexOf("\\c") >= 0) { ic = true; pat = pat.replace(/\\c/g, "") }
        else if (pat.indexOf("\\C") >= 0) { ic = false; pat = pat.replace(/\\C/g, "") }
        else if (ic && smartCase && /[A-Z]/.test(pat.replace(/\\./g, ""))) ic = false
        if (flags && flags.indexOf("i") >= 0) ic = true
        if (flags && flags.indexOf("I") >= 0) ic = false
        let js = ""
        if (pat.startsWith("\\v")) js = pat.substring(2).replace(/</g, "\\b").replace(/>/g, "\\b").replace(/=/g, "?")
        else {
            for (let i = 0; i < pat.length; ++i) {
                const c = pat[i]
                if (c === "\\" && i + 1 < pat.length) {
                    const d = pat[++i]
                    if ("(){}|+?".indexOf(d) >= 0) js += d
                    else if (d === "=") js += "?"
                    else if (d === "<" || d === ">") js += "\\b"
                    else if (d === "n") js += "\\n"
                    else if (d === "t") js += "\\t"
                    else js += "\\" + d                      // \s \d \w \. \/ ...
                } else if ("(){}|+?".indexOf(c) >= 0) js += "\\" + c
                else if (c === "~") js += "\\~"
                else js += c
            }
        }
        return new RegExp(js, ic ? "i" : "")
    }
    // (A String.replace callback's arguments: the match and its groups only,
    // not the offset and string after them.)
    function groups(args) {
        const named = typeof args[args.length - 1] === "object" && args[args.length - 1] !== null
        return args.slice(0, args.length - (named ? 3 : 2))
    }
    // vim's replacement as text for a match (& \0-\9 \n \r \t \u \U \l \ln \E ~).
    function replacement(rep, m) {
        let out = "", mode = "", once = ""
        const put = (s) => {
            for (const ch of s) {
                let c = ch
                if (once === "u") { c = c.toUpperCase(); once = "" }
                else if (once === "l") { c = c.toLowerCase(); once = "" }
                else if (mode === "U") c = c.toUpperCase()
                else if (mode === "L") c = c.toLowerCase()
                out += c
            }
        }
        for (let i = 0; i < rep.length; ++i) {
            const c = rep[i]
            if (c === "&") put(m[0])
            else if (c === "\\" && i + 1 < rep.length) {
                const d = rep[++i]
                if (d >= "0" && d <= "9") put(String(m[Number(d)] === undefined ? "" : m[Number(d)]))
                else if (d === "n" || d === "r") out += "\n"
                else if (d === "t") out += "\t"
                else if (d === "u" || d === "l") once = d
                else if (d === "U" || d === "L") mode = d
                else if (d === "E" || d === "e") mode = ""
                else put(d)
            } else put(c)
        }
        return out
    }
    // The text between delimiters from `i` (escaped ones kept): [text, index after].
    function delimited(s, i, d) {
        let out = ""
        for (; i < s.length; ++i) {
            if (s[i] === "\\" && s[i + 1] === d) { out += d === "/" || d === "?" ? "\\" + d : d; i++; continue }
            if (s[i] === "\\" && i + 1 < s.length) { out += s[i] + s[i + 1]; i++; continue }
            if (s[i] === d) return [out, i + 1]
            out += s[i]
        }
        return [out, i]
    }

    // ---- ranges ------------------------------------------------------------
    // One address from `i`: [line (0-based) or null, index after].
    function address(s, i, cur) {
        let line = null
        const c = s[i]
        if (c === ".") { line = cur; i++ }
        else if (c === "$") { line = ln.length - 1; i++ }
        else if (c >= "0" && c <= "9") { let n = ""; while (s[i] >= "0" && s[i] <= "9") n += s[i++]; line = Number(n) - 1 }
        else if (c === "'" && i + 1 < s.length) {
            const k = s[i + 1]; i += 2
            line = markLine(k)
            if (line < 0) throw "E20: Mark not set"
        } else if (c === "/" || c === "?") {
            const [p, j] = delimited(s, i + 1, c)
            i = j
            const re = regex(p !== "" ? p : lastSearch)
            if (p !== "") lastSearch = p
            line = -1
            for (let k = 1; k <= ln.length; ++k) {
                const n = c === "/" ? (cur + k) % ln.length : (cur - k + ln.length * 2) % ln.length
                if (re.test(ln[n].s)) { line = n; break }
            }
            if (line < 0) throw "E486: Pattern not found: " + p
        }
        // (+N, -N, a bare + or -; a number after an address adds to it.)
        while (i < s.length && (s[i] === "+" || s[i] === "-")) {
            const sign = s[i] === "+" ? 1 : -1; i++
            let n = ""; while (s[i] >= "0" && s[i] <= "9") n += s[i++]
            line = (line === null ? cur : line) + sign * (n === "" ? 1 : Number(n))
        }
        return [line, i]
    }
    function markLine(k) {
        if (k === "<" || k === ">") {
            if (edit.selectionStart === edit.selectionEnd) return -1
            return lineOfPos(k === "<" ? edit.selectionStart : Math.max(edit.selectionStart, edit.selectionEnd - 1))
        }
        const m = marks[k]
        if (m === undefined || m >= ln.length) return -1
        return m
    }
    // The range at the start of `s`: {a, b, given, rest}.
    function range(s, cur) {
        s = s.replace(/^[\s:]+/, "")
        if (s.startsWith("%")) return { a: 0, b: ln.length - 1, given: true, rest: s.substring(1) }
        if (s.startsWith("*")) { const a = markLine("<"), b = markLine(">"); if (a < 0) throw "E20: Mark not set"; return { a: a, b: b, given: true, rest: s.substring(1) } }
        let [a, i] = address(s, 0, cur)
        let b = a
        while (s[i] === "," || s[i] === ";") {
            const semi = s[i] === ";"
            if (semi && a !== null) cur = a
            const r = address(s, i + 1, cur)
            if (a === null) a = cur
            b = r[0] === null ? cur : r[0]
            i = r[1]
        }
        const given = a !== null
        if (a === null) a = b = cur
        if (b < a) { const x = a; a = b; b = x }
        return { a: a, b: b, given: given, rest: s.substring(i) }
    }
    function checkRange(r) {
        if (r.a < 0 || r.b >= ln.length) throw "E16: Invalid range"
    }

    // ---- "/" and "?": the next match from the cursor, wrapping ---------------
    property bool searchBack: false
    function search(pat, back) {
        if (pat === "") pat = lastSearch
        if (pat === "") return "E35: No previous regular expression"
        lastSearch = pat
        searchBack = back
        return searchNext(false)
    }
    function searchNext(reverse) {
        if (lastSearch === "") return "E35: No previous regular expression"
        let re
        try { re = regex(lastSearch) } catch (x) { return "E383: Invalid search string" }
        const g = new RegExp(re.source, re.flags + "g")
        const t = edit.text, p = edit.cursorPosition
        const all = []
        let m
        while ((m = g.exec(t)) !== null && all.length < 200000) { all.push(m.index); if (m[0] === "") g.lastIndex++ }
        if (all.length === 0) return "E486: Pattern not found: " + lastSearch
        const back = searchBack !== reverse
        let i = back ? all.length - 1 - all.slice().reverse().findIndex(x => x < p) : all.findIndex(x => x > p)
        let wrapped = false
        if (i < 0 || i >= all.length) { i = back ? all.length - 1 : 0; wrapped = true }
        edit.cursorPosition = all[i]
        return wrapped ? (back ? "search hit TOP, continuing at BOTTOM" : "search hit BOTTOM, continuing at TOP") : ""
    }

    // ---- messages -----------------------------------------------------------
    property bool error: false
    function lines(n) { return n + (n === 1 ? " line" : " lines") }
    function show(a, b, nums, list) {
        let out = []
        for (let i = a; i <= b && out.length < 200; ++i) {
            let s = list ? ln[i].s.replace(/\t/g, "^I") + "$" : ln[i].s
            out.push(nums ? String(i + 1).padStart(3) + " " + s : s)
        }
        return out.join("\n")
    }

    // ---- running one ---------------------------------------------------------
    function run(line) {
        error = false
        if (!edit) return ""
        line = line.replace(/^\s*:*/, "")
        if (line.trim() === "") return ""
        if (history[history.length - 1] !== line) history = history.concat([line]).slice(-100)
        load()
        try {
            const r = command(line, cursorLine(), false)
            return r === undefined ? "" : r
        } catch (e) {
            error = true
            if (e instanceof Error) console.warn("ex: " + e + "\n" + e.stack)
            return String(e)
        }
    }
    // A command line on the lines; inG: inside :g (no cursor moves, no history).
    function command(line, cur, inG) {
        const r = range(line, cur)
        let rest = r.rest.replace(/^\s+/, "")
        // (A range alone: go to its last line.)
        if (rest === "") {
            if (!r.given) return ""
            const n = Math.max(0, Math.min(ln.length - 1, r.b))
            if (!inG) store(n)
            return ""
        }
        // The name, then a !, then its arguments.
        let m = rest.match(/^([a-zA-Z]+|[&<>=#!~]+)/)
        let name = m ? m[1] : ""
        let after = rest.substring(name.length)
        // (:s and :g take their delimiter right after the name: :s/a/b/, :g#x#d.)
        if (/^(s|su|sub|subs|subst|substi|substit|substitu|substitut|substitute)$/.test(name) || name.match(/^s[gciIn&]*$/) && after.match(/^[^\w\s]/)) {
            if (name.match(/^s[gciIn&]+$/)) { after = "//~/" + name.substring(1) + after; name = "s" }
            return substitute(r, after.replace(/^\s+/, ""), inG)
        }
        if (name === "&" || name === "&&") return substitute(r, "&" + (name === "&&" ? "&" : "") + after.trim(), inG)
        if (name === "~") return substitute(r, "//~/" + after.trim(), inG)
        if (/^(g|gl|glo|glob|globa|global|v|vg|vgl|vglo|vglob|vgloba|vglobal)$/.test(name)) {
            let invert = name.startsWith("v")
            if (after.startsWith("!")) { invert = true; after = after.substring(1) }
            return global(r, after.replace(/^\s+/, ""), invert, inG)
        }
        const bang = after.startsWith("!")
        if (bang) after = after.substring(1)
        const arg = after.trim()
        // (Not "short"/"long": reserved words to Qt's QML compiler on Windows.)
        const is = (abbrev, full) => name.length >= abbrev.length && full.startsWith(name) && name.startsWith(abbrev)
        const cl = r.b
        const need = () => checkRange(r)

        if (name === ">" || name.startsWith(">") && /^>+$/.test(name) || name === "<" || /^<+$/.test(name)) {
            need()
            const dir = name[0] === ">" ? 1 : -1, times = name.length
            let [a, b] = [r.a, r.b]
            if (/^\d+$/.test(arg)) { a = r.b; b = Math.min(ln.length - 1, r.b + Number(arg) - 1) }
            for (let i = a; i <= b; ++i)
                for (let k = 0; k < times; ++k)
                    ln[i].s = dir > 0 ? (ln[i].s === "" ? "" : indentUnit + ln[i].s) : ln[i].s.replace(new RegExp("^( {1," + indentUnit.length + "}|\\t)"), "")
            if (!inG) store(b)
            return ""
        }
        if (name === "=") return String((r.given ? r.b : ln.length - 1) + 1)
        if (name === "#" || is("nu", "number")) { need(); return show(r.a, r.b, true, false) }
        if (is("p", "print") || is("P", "Print")) { need(); return show(r.a, r.b, false, false) }
        if (is("l", "list")) { need(); return show(r.a, r.b, false, true) }
        if (is("d", "delete")) {
            need()
            let [a, b] = [r.a, r.b], reg = ""
            const am = arg.match(/^([a-zA-Z"])?\s*(\d+)?$/)
            if (am) { reg = am[1] || ""; if (am[2]) { a = r.b; b = Math.min(ln.length - 1, r.b + Number(am[2]) - 1) } }
            const cut = ln.slice(a, b + 1).map(o => o.s).join("\n") + "\n"
            putRegister(reg, cut)
            ln.splice(a, b - a + 1)
            if (ln.length === 0) ln = [{ s: "" }]
            if (!inG) store(Math.min(a, ln.length - 1))
            return b - a + 1 > 2 && !inG ? lines(b - a + 1) + " fewer" : ""
        }
        if (is("y", "yank")) {
            need()
            let [a, b] = [r.a, r.b], reg = ""
            const am = arg.match(/^([a-zA-Z"])?\s*(\d+)?$/)
            if (am) { reg = am[1] || ""; if (am[2]) { a = r.b; b = Math.min(ln.length - 1, r.b + Number(am[2]) - 1) } }
            putRegister(reg, ln.slice(a, b + 1).map(o => o.s).join("\n") + "\n")
            return b - a + 1 > 2 ? lines(b - a + 1) + " yanked" : ""
        }
        if (is("pu", "put")) {
            const reg = arg.match(/^[a-zA-Z"]$/) ? arg : ""
            let text = reg && reg !== "\"" ? (registers[reg.toLowerCase()] || "") : killRing
            const add = text.replace(/\n$/, "").split("\n").map(s => ({ s: s }))
            const at = bang ? r.b : r.b + 1
            ln.splice(at, 0, ...add)
            if (!inG) store(at + add.length - 1)
            return ""
        }
        if (is("m", "move") || is("t", "t") || is("co", "copy")) {
            need()
            const copy = name !== "m" && !"move".startsWith(name) || name === "t"
            const dest = arg === "0" ? -1 : range(arg, cur).b
            if (dest >= ln.length) throw "E16: Invalid range"
            const block = ln.slice(r.a, r.b + 1)
            if (copy) {
                const add = block.map(o => ({ s: o.s }))
                ln.splice(dest + 1, 0, ...add)
                if (!inG) store(dest + add.length)
            } else {
                if (dest >= r.a && dest < r.b) throw "E134: Cannot move a range of lines into itself"
                ln.splice(r.a, block.length)
                const at = dest >= r.b ? dest - block.length + 1 : dest + 1
                ln.splice(at, 0, ...block)
                if (!inG) store(at + block.length - 1)
            }
            return ""
        }
        if (is("j", "join")) {
            need()
            let [a, b] = [r.a, r.given && r.a !== r.b ? r.b : r.b + 1]
            if (/^\d+$/.test(arg)) b = r.b + Number(arg) - 1
            b = Math.min(ln.length - 1, b)
            if (b <= a) return ""
            let s = ln[a].s
            for (let i = a + 1; i <= b; ++i) {
                const next = bang ? ln[i].s : ln[i].s.replace(/^\s+/, "")
                if (bang || s === "" || next === "" || /\s$/.test(s)) s += next
                else s += (next.startsWith(")") ? "" : /[.!?]$/.test(s) ? " " : " ") + next
            }
            ln[a].s = s
            ln.splice(a + 1, b - a)
            if (!inG) store(a, s.length)
            return ""
        }
        if (is("norm", "normal")) {
            need()
            if (!keyHandler) throw "Not here: :normal"
            if (arg === "") throw "E471: Argument required"
            normal(r.a, r.b, after.replace(/^ /, ""))
            return ""
        }
        if (is("sor", "sort")) {
            const [a, b] = r.given ? [r.a, r.b] : [0, ln.length - 1]
            let opts = arg, pat = null
            const pm = opts.match(/\/((?:\\.|[^\/])*)\//)
            if (pm) { pat = regex(pm[1] !== "" ? pm[1] : lastSearch); opts = opts.replace(pm[0], "") }
            const ic = opts.indexOf("i") >= 0, num = /[nfxob]/.test(opts), uniq = opts.indexOf("u") >= 0, useMatch = opts.indexOf("r") >= 0
            const key = (s) => {
                if (pat) { const mm = s.match(pat); if (mm) s = useMatch ? mm[0] : s.substring(mm.index + mm[0].length); else s = useMatch ? "" : s }
                return ic ? s.toLowerCase() : s
            }
            const numOf = (s) => {
                const mm = opts.indexOf("x") >= 0 ? s.match(/-?(0[xX])?[0-9a-fA-F]+/) : s.match(/-?\d+(\.\d+)?/)
                return mm ? (opts.indexOf("x") >= 0 ? parseInt(mm[0], 16) : Number(mm[0])) : -Infinity
            }
            let part = ln.slice(a, b + 1)
            part.sort((x, y) => {
                const kx = key(x.s), ky = key(y.s)
                if (num) return numOf(kx) - numOf(ky)
                return kx < ky ? -1 : kx > ky ? 1 : 0
            })
            if (bang) part.reverse()
            if (uniq) part = part.filter((o, i) => i === 0 || key(o.s) !== key(part[i - 1].s))
            ln.splice(a, b - a + 1, ...part)
            if (!inG) store(a)
            return ""
        }
        if (is("ret", "retab")) {
            const [a, b] = r.given ? [r.a, r.b] : [0, ln.length - 1]
            const ts = /^\d+$/.test(arg) ? Number(arg) : tabstop
            for (let i = a; i <= b; ++i) {
                let out = "", col = 0
                for (const c of ln[i].s) {
                    if (c === "\t") { const n = ts - (col % ts); out += " ".repeat(n); col += n }
                    else { out += c; col++ }
                }
                ln[i].s = out
            }
            if (arg !== "" && /^\d+$/.test(arg)) tabstop = ts
            if (!inG) store(cl)
            return ""
        }
        if (is("le", "left") || is("ri", "right") || is("ce", "center")) {
            need()
            const which = name.startsWith("l") ? "l" : name.startsWith("r") ? "r" : "c"
            const w = /^\d+$/.test(arg) ? Number(arg) : which === "l" ? 0 : 80
            for (let i = r.a; i <= r.b; ++i) {
                const s = ln[i].s.trim()
                ln[i].s = which === "l" ? " ".repeat(w) + s
                       : which === "r" ? " ".repeat(Math.max(0, w - s.length)) + s
                       : " ".repeat(Math.max(0, Math.floor((w - s.length) / 2))) + s
            }
            if (!inG) store(r.b)
            return ""
        }
        if (is("ma", "mark") || name === "k" || name.match(/^k[a-z]$/)) {
            const k = name.length === 2 && name[0] === "k" ? name[1] : arg
            if (!/^[a-zA-Z'`]$/.test(k)) throw "E191: Argument must be a letter"
            const mm = Object.assign({}, marks); mm[k] = r.b; marks = mm
            return ""
        }
        if (is("delm", "delmarks")) {
            if (bang) { marks = ({}); return "" }
            const mm = Object.assign({}, marks)
            for (const part of arg.split(/\s+/)) {
                const rg = part.match(/^([a-z])-([a-z])$/)
                if (rg) for (let c = rg[1].charCodeAt(0); c <= rg[2].charCodeAt(0); ++c) delete mm[String.fromCharCode(c)]
                else for (const c of part) delete mm[c]
            }
            marks = mm
            return ""
        }
        if (name === "marks") {
            const out = ["mark line  col text"]
            for (const k of Object.keys(marks).sort()) {
                const n = markLine(k)
                if (n >= 0) out.push(" " + k + "  " + String(n + 1).padStart(5) + "    0 " + ln[n].s.trim())
            }
            return out.join("\n")
        }
        if (is("reg", "registers") || is("di", "display")) {
            const out = ["Type Name Content"]
            const fmt = (s) => s.replace(/\n/g, "^J").substring(0, 70)
            if (killRing !== "") out.push("  l  \"\"   " + fmt(killRing))
            for (const k of Object.keys(registers).sort()) if (arg === "" || arg.indexOf(k) >= 0) out.push("  l  \"" + k + "   " + fmt(registers[k]))
            return out.join("\n")
        }
        if (is("f", "file")) {
            const n = cursorLine()
            return "\"" + (fileName || "[No Name]") + "\"" + (edit.text !== "" ? " " + lines(ln.length) : "") + " --" + Math.round((n + 1) * 100 / Math.max(1, ln.length)) + "%--"
        }
        if (is("pw", "pwd")) return folder
        if (is("ec", "echo")) return arg.replace(/^(["'])(.*)\1$/, "$2")
        if (is("u", "undo")) { undo(); return "" }
        if (is("red", "redo")) { redo(); return "" }
        if (is("noh", "nohlsearch")) { setOption("hlsearch-off", true); return "" }
        if (is("se", "set")) return setOptions(arg)
        if (is("sav", "saveas")) { if (arg === "") throw "E471: Argument required"; write(path(arg), bang); return "" }
        if (is("w", "write") || is("up", "update")) {
            if (arg.startsWith("!")) throw "Not here: :w !{cmd}"
            write(arg === "" ? "" : path(arg), bang)
            return ""
        }
        if (name === "wq" || is("x", "xit") || is("exi", "exit") || name === "wqa" || is("wqa", "wqall") || is("xa", "xall")) {
            write(arg === "" ? "" : path(arg), bang)
            quit(bang)
            return ""
        }
        if (is("wa", "wall")) { write("", bang); return "" }
        if (is("q", "quit") || is("qa", "qall") || is("quita", "quitall") || is("clo", "close")) { quit(bang); return "" }
        if (is("e", "edit") || name === "ex") { editFile(arg === "" ? "" : path(arg), bang); return "" }
        if (is("ene", "enew")) throw "Not here: :enew (a file card shows one file)"
        if (is("r", "read")) {
            if (arg.startsWith("!")) throw "Not here: :r !{cmd}"
            if (!readFile) throw "Not here: :read"
            const text = readFile(arg === "" ? "" : path(arg))
            const add = String(text).replace(/\r\n/g, "\n").replace(/\n$/, "").split("\n").map(s => ({ s: s }))
            const at = r.given ? r.b + 1 : cur + 1
            ln.splice(at, 0, ...add)
            if (!inG) store(at)
            return ""
        }
        if (name === "!" || bang && name === "") throw "Not here: :!{cmd} (run it in the terminal)"
        if (/^(sp|split|vs|vsplit|new|vne|vnew|tabe|tabedit|tabnew|tabc|tabclose|tabn|tabnext|tabp|tabprevious|b|buffer|bn|bnext|bp|bprevious|bd|bdelete|on|only|te|terminal|sh|shell)$/.test(name))
            throw "Not here: :" + name + " (a file card shows one file)"
        throw "E492: Not an editor command: " + line.trim()
    }
    function path(p) {
        p = p.replace(/^~(?=\/|$)/, folder.replace(/^(\/home\/[^\/]+|\/Users\/[^\/]+|\/root).*$/, "$1"))
        return p.startsWith("/") || folder === "" ? p : folder.replace(/\/$/, "") + "/" + p
    }
    function putRegister(reg, text) {
        if (reg === "_") return
        if (reg !== "" && reg !== "\"") {
            const rr = Object.assign({}, registers)
            const k = reg.toLowerCase()
            rr[k] = reg !== k ? (rr[k] || "") + text : text       // (A-Z appends.)
            registers = rr
        }
        killRing = text
    }

    // :s/pat/rep/flags [count], the range's lines.
    function substitute(r, args, inG) {
        let pat, rep, flags = "", count = ""
        if (args.startsWith("&")) {
            if (!lastSub) throw "E35: No previous regular expression"
            pat = lastSub.pat; rep = lastSub.rep
            flags = (args.substring(1).startsWith("&") ? lastSub.flags : "") + args.replace(/^&+/, "")
        } else if (args === "" || /^[gciIn&\d\s]+$/.test(args)) {
            if (!lastSub) throw "E35: No previous regular expression"
            pat = lastSub.pat; rep = lastSub.rep; flags = args
        } else {
            const d = args[0]
            const [p, i] = delimited(args, 1, d)
            const [rp, j] = delimited(args, i, d)
            pat = p !== "" ? p : (lastSearch || (lastSub ? lastSub.pat : ""))
            rep = rp === "~" && lastSub ? lastSub.rep : rp.replace(/(^|[^\\])~/g, (mm, pre) => pre + (lastSub ? lastSub.rep : ""))
            flags = args.substring(j)
        }
        if (pat === "") throw "E35: No previous regular expression"
        const fm = flags.match(/^\s*([&gciIn]*)\s*(\d*)\s*$/)
        if (fm) { flags = fm[1]; count = fm[2] }
        lastSub = { pat: pat, rep: rep, flags: flags.replace(/&/g, "") }
        lastSearch = pat
        let [a, b] = [r.a, r.b]
        if (count !== "") { a = r.b; b = Math.min(ln.length - 1, r.b + Number(count) - 1) }
        checkRange({ a: a, b: b })
        const re = regex(pat, flags)
        const all = flags.indexOf("g") >= 0
        let n = 0, linesHit = 0, last = a
        // (A pattern with \n works on the joined lines.)
        if (pat.indexOf("\\n") >= 0) {
            const block = ln.slice(a, b + 1).map(o => o.s).join("\n")
            const g = new RegExp(re.source, re.flags + "g")
            let hits = 0
            const out = block.replace(g, (...m) => { if (!all && hits > 0) return m[0]; hits++; return replacement(rep, groups(m)) })
            n = hits; linesHit = hits ? 1 : 0
            if (flags.indexOf("n") < 0 && hits) ln.splice(a, b - a + 1, ...out.split("\n").map(s => ({ s: s })))
        } else {
            for (let i = a; i <= b; ++i) {
                const s = ln[i].s
                const g = new RegExp(re.source, re.flags + (all ? "g" : ""))
                let hits = 0
                const out = s.replace(g, (...m) => { hits++; return replacement(rep, groups(m)) })
                if (hits) {
                    n += hits; linesHit++; last = i
                    if (flags.indexOf("n") < 0) {
                        const parts = out.split("\n")
                        ln[i].s = parts[0]
                        if (parts.length > 1) { ln.splice(i + 1, 0, ...parts.slice(1).map(x => ({ s: x }))); b += parts.length - 1; i += parts.length - 1 }
                    }
                }
            }
        }
        if (n === 0) { if (inG) return ""; throw "E486: Pattern not found: " + pat }
        if (flags.indexOf("n") >= 0) return n + (n === 1 ? " match" : " matches") + " on " + lines(linesHit)
        if (!inG) store(last)
        return linesHit > 1 && !inG ? n + " substitution" + (n === 1 ? "" : "s") + " on " + lines(linesHit) : ""
    }

    // :g/pat/cmd (each matching line, cmd :p if none), :v and :g! the others.
    function global(r, args, invert, inG) {
        if (inG) throw "E147: Cannot do :global recursive"
        const d = args[0]
        if (!d) throw "E476: Invalid command"
        const [p, i] = delimited(args, 1, d)
        const pat = p !== "" ? p : lastSearch
        if (pat !== "") lastSearch = pat
        let cmd = args.substring(i).trim() || "p"
        const re = regex(pat)
        const [a, b] = r.given ? [r.a, r.b] : [0, ln.length - 1]
        const hit = ln.slice(a, b + 1).filter(o => re.test(o.s) !== invert)
        if (hit.length === 0) throw "E486: Pattern not found: " + pat
        let shown = []
        let lastLine = -1
        for (const o of hit) {
            const n = ln.indexOf(o)
            if (n < 0) continue                    // (deleted by a command on an earlier one)
            // (Commands run one after another: :g/x/s/a/b/|s/c/d/)
            for (const one of splitBar(cmd)) {
                const out = command(one, n, true)
                if (out) shown.push(out)
            }
            lastLine = ln.indexOf(o) >= 0 ? ln.indexOf(o) : Math.min(n, ln.length - 1)
        }
        store(Math.max(0, lastLine))
        return shown.join("\n")
    }

    // (Commands after :g joined by |, a \| kept as a |.)
    function splitBar(c) {
        const out = []
        let cur = ""
        for (let i = 0; i < c.length; ++i) {
            if (c[i] === "\\" && c[i + 1] === "|") { cur += "|"; i++ }
            else if (c[i] === "|") { out.push(cur); cur = "" }
            else cur += c[i]
        }
        out.push(cur)
        return out
    }

    // ---- completion: the commands, with what each does ------------------------
    // [name as typed in full, the shortest it can be, what it does]
    readonly property var commandList: [
        ["substitute", "s", "s/pat/rep/[g i I n] — replace in the range (default: this line)"],
        ["global", "g", "g/pat/cmd — run cmd on each line matching (default cmd: print)"],
        ["vglobal", "v", "v/pat/cmd — run cmd on each line NOT matching"],
        ["delete", "d", "delete the range's lines [into register x] [count]"],
        ["yank", "y", "copy the range's lines [into register x]"],
        ["put", "pu", "put the register's lines after the line (put! before)"],
        ["move", "m", "move the range below line {address} (0: to the top)"],
        ["copy", "co", "copy the range below line {address} (same as :t)"],
        ["t", "t", "copy the range below line {address}"],
        ["join", "j", "join the range's lines (join! without spaces)"],
        [">", ">", "indent the range (>> twice)"],
        ["<", "<", "unindent the range"],
        ["normal", "norm", "normal {keys} — run vi keys on each line of the range"],
        ["sort", "sor", "sort the range (all lines): ! reverse, i any case, n numbers, u unique, /pat/"],
        ["retab", "ret", "tabs to spaces [tabstop]"],
        ["left", "le", "align the range left [indent]"],
        ["right", "ri", "align the range right [width]"],
        ["center", "ce", "center the range [width]"],
        ["mark", "ma", "mark {a-z} — set a mark on the line (use as 'a)"],
        ["k", "k", "k{a-z} — set a mark on the line"],
        ["delmarks", "delm", "delete marks (delm! all)"],
        ["marks", "marks", "list the marks"],
        ["registers", "reg", "list the registers"],
        ["display", "di", "list the registers"],
        ["print", "p", "show the range's lines"],
        ["number", "nu", "show the range's lines with numbers"],
        ["#", "#", "show the range's lines with numbers"],
        ["list", "l", "show the range's lines with tabs and ends"],
        ["=", "=", "the last line's number (or the range's)"],
        ["file", "f", "the file's name, lines and where you are"],
        ["undo", "u", "undo"],
        ["redo", "red", "redo"],
        ["nohlsearch", "noh", "clear the search highlight"],
        ["set", "se", "set ic scs sw=N ts=N et wrap nu (no…, …!, …?)"],
        ["echo", "ec", "show the text"],
        ["write", "w", "save (w {file}: save a copy there; w!: even if unchanged)"],
        ["update", "up", "save if changed"],
        ["saveas", "sav", "save as {file}, and edit that"],
        ["wq", "wq", "save and close the editor"],
        ["xit", "x", "save if changed and close the editor"],
        ["exit", "exi", "save if changed and close the editor"],
        ["wall", "wa", "save"],
        ["wqall", "wqa", "save and close"],
        ["xall", "xa", "save and close"],
        ["quit", "q", "close the editor (q! drops the changes)"],
        ["qall", "qa", "close the editor"],
        ["close", "clo", "close the editor"],
        ["edit", "e", "e! drops the changes and reads the file again; e {file} opens one"],
        ["read", "r", "r {file} — put a file's lines after the line"],
        ["pwd", "pw", "the file's folder"]
    ]
    // (An owner's own meanings: name -> its help, or null to leave it out.)
    property var helpFor: ({})
    // The ones the command line's word could be: [{name, short, help}], best first.
    function completions(line) {
        // (Only while typing the command's name, after any range: past it,
        // its arguments, there's nothing to offer.)
        const m = line.match(/^((?:[\s\d.$%,;+\-:]|'[a-zA-Z<>])*)([a-zA-Z]*|[<>=#&~]*)$/)
        if (!m) return []
        const head = m[1], word = m[2]
        const w = word.toLowerCase()
        const out = []
        for (const c0 of commandList) {
            const o = helpFor[c0[0]]
            if (o === null) continue
            const c = o !== undefined ? [c0[0], c0[1], o] : c0
            if (c[0].startsWith(w) || (w.length >= c[1].length && c[0].startsWith(w)) || (w !== "" && c[1].startsWith(w)))
                out.push({ name: c[0], short: c[1], help: c[2], head: head })
        }
        // (Exact abbreviations first: s is substitute, d delete.)
        out.sort((x, y) => (y.short === w) - (x.short === w) || (y.name.startsWith(w)) - (x.name.startsWith(w)))
        return out
    }

    // :normal {keys} on each line of the range, the keys in vi's normal mode.
    function normal(a, b, keys) {
        store()
        const objs = ln.slice(a, b + 1)
        for (const o of objs) {
            load()
            const n = ln.findIndex(x => x.s === o.s)
            if (n < 0) continue
            store(n, 0)
            if (viKeys) { viKeys.viNormal = true; viKeys.viPending = "" }
            for (const c of keys) {
                const ev = { key: c === "\x1b" ? Qt.Key_Escape : c.toUpperCase().charCodeAt(0), text: c, modifiers: 0, accepted: false }
                const done = keyHandler(ev)
                if (!done && c !== "\x1b") { edit.insert(edit.cursorPosition, c) }
            }
            if (viKeys) { viKeys.viNormal = true; viKeys.viPending = "" }
        }
        load()
    }

    // :set name, noname, name!, name?, name=value (and :set alone: what's set).
    function setOptions(arg) {
        if (arg === "" || arg === "all") return "ignorecase=" + ignoreCase + " smartcase=" + smartCase + " shiftwidth=" + indentUnit.length + " tabstop=" + tabstop + " expandtab=" + (indentUnit !== "\t")
        const out = []
        for (const part of arg.split(/\s+/)) {
            const m = part.match(/^(no|inv)?([a-z]+)(!|\?|[=:](.*))?$/)
            if (!m) throw "E518: Unknown option: " + part
            const neg = m[1] === "no", inv = m[1] === "inv" || m[3] === "!", ask = m[3] === "?"
            const name = ({ ic: "ignorecase", scs: "smartcase", sw: "shiftwidth", ts: "tabstop", et: "expandtab", sts: "softtabstop", nu: "number", hls: "hlsearch", is: "incsearch" })[m[2]] || m[2]
            const val = m[4]
            const flag = (cur) => inv ? !cur : !neg
            switch (name) {
            case "ignorecase": if (ask) out.push((ignoreCase ? "" : "no") + name); else ignoreCase = flag(ignoreCase); break
            case "smartcase": if (ask) out.push((smartCase ? "" : "no") + name); else smartCase = flag(smartCase); break
            case "shiftwidth": case "softtabstop":
                if (ask || val === undefined) out.push(name + "=" + indentUnit.length)
                else { const n = Number(val); if (!(n > 0)) throw "E487: Argument must be positive"; indentUnit = indentUnit === "\t" ? "\t" : " ".repeat(n); setOption("shiftwidth", n) }
                break
            case "tabstop":
                if (ask || val === undefined) out.push("tabstop=" + tabstop)
                else { const n = Number(val); if (!(n > 0)) throw "E487: Argument must be positive"; tabstop = n }
                break
            case "expandtab":
                if (ask) out.push((indentUnit !== "\t" ? "" : "no") + name)
                else { const on = flag(indentUnit !== "\t"); indentUnit = on ? " ".repeat(Math.max(1, indentUnit === "\t" ? tabstop : indentUnit.length)) : "\t"; setOption("expandtab", on) }
                break
            case "wrap": case "number": case "hlsearch": case "incsearch": case "list": case "readonly":
                // (The owner's: it says whether it has it.)
                if (ask) out.push(name)
                else setOption(name, inv ? "toggle" : !neg)
                break
            default: throw "E518: Unknown option: " + m[2]
            }
        }
        return out.join("  ")
    }
}
