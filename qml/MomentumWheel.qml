import QtQuick

// Momentum scrolling for a Flickable (list, grid, scroll view), like the
// terminal's: a wheel notch flicks the content so it glides to a stop, and
// notches in quick succession build speed; a touchpad follows the fingers
// and keeps going when they lift. Put it inside the Flickable:
//     MomentumWheel { flick: myGrid }
WheelHandler {
    id: wh
    property Flickable flick: null
    target: null
    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
    property real boost: 1
    property double lastWheel: 0
    property real padVelocity: 0        // px/s, the touchpad's latest speed
    property double lastPad: 0

    function clampY(y) {
        return Math.max(flick.originY - flick.topMargin,
                        Math.min(y, flick.originY + flick.contentHeight + flick.bottomMargin - flick.height))
    }

    onWheel: (ev) => {
        if (!flick || flick.contentHeight <= flick.height) { ev.accepted = false; return }
        ev.accepted = true
        const now = Date.now()
        // Touchpad fingers lifted (an event with no movement): keep going at
        // the speed they had, if they were still moving.
        if (ev.phase === Qt.ScrollEnd) {
            if (now - lastPad < 150 && Math.abs(padVelocity) > 120)
                flick.flick(0, Math.max(-12000, Math.min(12000, padVelocity * 1.15)))
            padVelocity = 0
            return
        }
        if (ev.pixelDelta.y === 0 && ev.angleDelta.y === 0) return
        // A touchpad: pixel movement, or a scroll with phases.
        if (ev.pixelDelta.y !== 0 || ev.phase === Qt.ScrollBegin || ev.phase === Qt.ScrollUpdate) {
            // Move with the fingers.
            const dy = ev.pixelDelta.y !== 0 ? ev.pixelDelta.y : ev.angleDelta.y / 3
            flick.cancelFlick()
            flick.contentY = clampY(flick.contentY - dy)
            const dt = Math.max(4, now - lastPad)
            const v = dy * 1000 / dt
            padVelocity = dt > 80 ? v : padVelocity * 0.5 + v * 0.5
            lastPad = now
            return
        }
        // Wheel: each notch adds speed; a quick run of notches multiplies it.
        boost = now - lastWheel < 260 ? Math.min(boost * 1.3, 8) : 1
        lastWheel = now
        const notches = ev.angleDelta.y / 120
        let v = notches * 1100 * boost
        // flick() takes the speed of a drag (positive scrolls up); the view's
        // own verticalVelocity has the other sign.
        const current = -flick.verticalVelocity
        if (current * v > 0) v += current
        flick.flick(0, v)
    }
}
