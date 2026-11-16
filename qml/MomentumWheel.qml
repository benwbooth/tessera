import QtQuick

// Momentum scrolling for a Flickable (list, grid, scroll view), the way Kog
// does it: a wheel notch eases 120 px to a set destination (Chrome's curve,
// quicker for longer runs; a notch mid-glide extends the destination), and a
// touchpad follows the fingers, then glides on with a steady slowdown once
// they lift. Touching the pad again stops a glide. Put it inside the Flickable:
//     MomentumWheel { flick: myGrid }
WheelHandler {
    id: wh
    property Flickable flick: null
    target: null
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad

    property real velocity: 0              // px/s of content movement
    property real maximumVelocity: 9000
    property real deceleration: 2300       // px/s² for a touchpad glide
    property real wheelStep: 120
    property bool wheelAnimating: false
    property real wheelTarget: 0
    property real wheelStart: 0
    property real wheelStartVelocity: 0
    property real wheelElapsed: 0
    property real wheelDuration: 0.2
    property double lastPixelAt: 0
    property real pixelVelocity: 0

    function lo() { return flick.originY - flick.topMargin }
    function hi() { return Math.max(lo(), flick.originY + flick.contentHeight - flick.height + flick.bottomMargin) }
    function moveTo(y) { flick.contentY = Math.max(lo(), Math.min(hi(), y)) }

    function stop() {
        velocity = 0
        wheelAnimating = false
        frames.stop()
        padEnd.stop()
        lastPixelAt = 0
        pixelVelocity = 0
    }

    // Wheel notches (positive scrolls up).
    function notches(steps) {
        const delta = -steps * wheelStep
        const current = flick.contentY
        const pending = wheelAnimating ? wheelTarget - current : 0
        // (A reversal starts from here, not from the old destination.)
        const origin = pending * delta > 0 ? wheelTarget : current
        wheelTarget = Math.max(lo(), Math.min(hi(), origin + delta))
        padEnd.stop(); lastPixelAt = 0; pixelVelocity = 0
        if (wheelTarget === current) { stop(); return false }
        // 12 frames at 60 Hz up to 120 px, 6 frames from 480 px.
        const distance = Math.abs(wheelTarget - current)
        wheelDuration = Math.max(6, Math.min(12, 14 - distance / 60)) / 60
        wheelStartVelocity = wheelAnimating && velocity * (wheelTarget - current) > 0 ? velocity : 0
        wheelStart = current
        wheelElapsed = 0
        wheelAnimating = true
        frames.start()
        return true
    }

    // From the start position and speed to the destination at rest.
    function wheelCurve(t) {
        const span = wheelTarget - wheelStart
        const v0 = wheelStartVelocity * wheelDuration
        const t2 = t * t, t3 = t2 * t
        return {
            position: wheelStart + v0 * (t3 - 2 * t2 + t) + span * (3 * t2 - 2 * t3),
            velocity: (v0 * (3 * t2 - 4 * t + 1) + span * (6 * t - 6 * t2)) / wheelDuration
        }
    }

    // Touchpad (or precision wheel) pixels: follow them; a touchpad glides on.
    function pixels(dy, glide) {
        const now = Date.now()
        const move = -dy
        const elapsed = lastPixelAt > 0 ? (now - lastPixelAt) / 1000 : 0
        frames.stop()
        wheelAnimating = false
        velocity = 0
        moveTo(flick.contentY + move)
        if (!glide) { padEnd.stop(); lastPixelAt = 0; pixelVelocity = 0; return }
        if (elapsed >= 0.004 && elapsed <= 0.08) {
            const v = move / elapsed
            pixelVelocity = pixelVelocity === 0 ? v : pixelVelocity * 0.65 + v * 0.35
        } else {
            pixelVelocity = move * 60
        }
        pixelVelocity = Math.max(-maximumVelocity, Math.min(maximumVelocity, pixelVelocity))
        lastPixelAt = now
        padEnd.restart()
    }

    function fingersLifted() {
        lastPixelAt = 0
        velocity = pixelVelocity
        pixelVelocity = 0
        if (Math.abs(velocity) < 40) { stop(); return }
        frames.start()
    }

    function advance(frameTime) {
        if (wheelAnimating) {
            wheelElapsed += Math.max(0, frameTime)
            const progress = Math.min(1, wheelElapsed / wheelDuration)
            const p = wheelCurve(progress)
            moveTo(p.position)
            velocity = progress === 1 ? 0 : p.velocity
            if (progress === 1 || flick.contentY === lo() || flick.contentY === hi()) stop()
            return
        }
        const dt = Math.min(0.05, frameTime)
        if (dt <= 0) return
        const next = Math.max(lo(), Math.min(hi(), flick.contentY + velocity * dt))
        const atEdge = next === flick.contentY && ((velocity < 0 && next <= lo()) || (velocity > 0 && next >= hi()))
        moveTo(next)
        const speed = Math.max(0, Math.abs(velocity) - deceleration * dt)
        velocity = Math.sign(velocity) * speed
        if (atEdge || speed < 1) stop()
    }

    onWheel: (ev) => {
        if (!flick || flick.contentHeight <= flick.height) { ev.accepted = false; return }
        // Fingers back on the pad: a glide stops where it is.
        if (ev.phase === Qt.ScrollBegin) { stop(); ev.accepted = true; return }
        if (ev.phase === Qt.ScrollEnd) { ev.accepted = true; return }   // (the pause timer decides the glide)
        if (ev.pixelDelta.y !== 0) {
            pixels(ev.pixelDelta.y, ev.device.type === PointerDevice.TouchPad && ev.phase !== Qt.ScrollMomentum)
            ev.accepted = true
            return
        }
        const steps = ev.angleDelta.y / 120
        ev.accepted = steps !== 0 && notches(steps)
    }

    property FrameAnimation frames: FrameAnimation { onTriggered: wh.advance(frameTime) }
    // The fingers have stopped sending movement: they lifted (or paused).
    property Timer padEnd: Timer { interval: 45; onTriggered: wh.fingersLifted() }
    // A drag or the scroll bar takes over.
    property Connections drags: Connections {
        target: wh.flick
        function onDraggingChanged() { if (wh.flick.dragging) wh.stop() }
    }
}
