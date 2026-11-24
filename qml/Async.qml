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
        // (Test hook: TESSERA_TEST_ASYNC_LOG logs each call and its answer.)
        if (logging < 0) logging = controller.testEnv("ASYNC_LOG") !== "" ? 1 : 0
        if (logging) console.log("async call " + k + ": " + start.toString().slice(0, 90))
        start(k)
    }
    property int logging: -1
    function reply(k, json) {
        if (logging > 0) console.log("async reply " + k + ": " + json.length + " chars " + json.slice(0, 120))
        const done = waiting[k]
        if (!done) return
        delete waiting[k]
        done(json)
    }
}
