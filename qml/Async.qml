pragma Singleton
import QtQuick

// Calls that run in the background (never on the UI thread) and answer
// later: Async.call(controller, k => controller.fileList(k, path), r => ...).
// The start function gets a key to pass on; `done` gets the answer: the
// string the call used to return.
QtObject {
    property int seq: 0
    property var waiting: ({})
    property var hooked: []

    function call(controller, start, done) {
        if (!controller) return
        if (hooked.indexOf(controller) < 0) {
            hooked.push(controller)
            controller.asyncReply.connect(reply)
        }
        const k = "a" + (++seq)
        if (done) waiting[k] = done
        start(k)
    }
    function reply(k, json) {
        const done = waiting[k]
        if (!done) return
        delete waiting[k]
        done(json)
    }
}
