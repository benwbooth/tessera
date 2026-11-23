import QtQuick
import QtTest
import "../../qml"

// vi through EditKeys, as a file card's editor has it: insert mode's keys
// (Emacs ones, pairs) still the field's, normal mode the vi engine's.
Item {
    width: 400; height: 300
    TextEdit { id: edit; width: 400; height: 300 }
    EditKeys { id: ek; edit: edit; keys: "vi" }

    TestCase {
        name: "EditKeysVi"
        when: windowShown
        function key(k, text, mods) {
            const ev = { key: k, text: text, modifiers: mods || 0 }
            if (!ek.handle(ev) && text.length === 1 && text >= " ") { const p = edit.cursorPosition; edit.insert(p, text); edit.cursorPosition = p + 1 }
        }
        function type(s) { for (const c of s) key(c.toUpperCase().charCodeAt(0), c) }
        function test_flow() {
            edit.text = ""
            ek.viNormal = false
            ek.vi.reset()
            type("foo(bar")
            compare(edit.text, "foo(bar)")              // (insert mode: pairs, the field's)
            key(Qt.Key_Escape, "\x1b")
            compare(ek.viNormal, true)
            compare(ek.vi.modeName, "NORMAL")
            type("ci(")
            compare(ek.viNormal, false)
            type("x")
            key(Qt.Key_Escape, "\x1b")
            compare(edit.text, "foo(x)")
            type("0ysiw\"")
            compare(edit.text, "\"foo\"(x)")
            type("u")
            compare(edit.text, "foo(x)")
            type("u")
            compare(edit.text, "foo(bar)")
            // (Ctrl+A in insert mode: Emacs' line start, not vi's add.)
            type("A")
            key(Qt.Key_A, "", Qt.ControlModifier)
            compare(edit.cursorPosition, 0)
            key(Qt.Key_Escape, "\x1b")
            let line = ""
            const f = (k, i) => { line = k + i }
            ek.commandLine.connect(f)
            type(":")
            ek.commandLine.disconnect(f)
            compare(line, ":")
            // (:normal runs vi keys on lines.)
            edit.text = "a b\nc d"
            ek.vi.reset()
            ek.ex.run("%normal dw")
            compare(edit.text, "b\nd")
        }
    }
}
