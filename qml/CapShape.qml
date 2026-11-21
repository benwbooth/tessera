import QtQuick
import QtQuick.Shapes

// A Powerline / Nerd Font cap (a pill's rounded or pointed end) as a shape
// the height of its run. A Shape, not a Canvas: it's drawn in the same frame
// as the run (a Canvas paints a frame later, so a new prompt flashed square
// ends first).
Shape {
    id: cs
    property string cap: ""
    property color ink: "white"
    // (A pixel more on the flat side, under the next run: no seam.)
    property bool seam: false
    preferredRendererType: Shape.CurveRenderer
    readonly property bool flatRight: ["\ue0b6", "\ue0b2", "\ue0ba", "\ue0be"].indexOf(cap) >= 0
    // (W: the shape's width; x: where it starts.)
    function path(cap, W, h, x) {
        const r = h / 2, R = x + W
        switch (cap) {
        case "\ue0b6": return `M ${R} 0 A ${r} ${r} 0 0 0 ${R} ${h} Z`                 // left half circle
        case "\ue0b4": return `M ${x} 0 A ${r} ${r} 0 0 1 ${x} ${h} Z`                 // right half circle
        case "\ue0b0": case "\ue0b8": return `M ${x} 0 L ${R} ${r} L ${x} ${h} Z`      // right-pointing triangle
        case "\ue0b2": case "\ue0ba": return `M ${R} 0 L ${x} ${r} L ${R} ${h} Z`      // left-pointing triangle
        case "\ue0bc": return `M ${x} 0 L ${R} 0 L ${x} ${h} Z`                        // upper-left triangle
        case "\ue0be": return `M ${x} 0 L ${R} 0 L ${R} ${h} Z`                        // upper-right triangle
        }
        return ""
    }
    ShapePath {
        strokeWidth: -1
        fillColor: cs.ink
        PathSvg {
            path: {
                const h = cs.height
                if (!cs.seam) return cs.path(cs.cap, cs.width, h, 0)
                // (A pixel in on the round side, with a 1.5 px strip on the
                // flat edge, under the neighbouring run.)
                const W = cs.width - 1
                return cs.flatRight
                    ? `M ${W - 0.5} 0 L ${W + 1} 0 L ${W + 1} ${h} L ${W - 0.5} ${h} Z ` + cs.path(cs.cap, W, h, 0)
                    : `M 0 0 L 1.5 0 L 1.5 ${h} L 0 ${h} Z ` + cs.path(cs.cap, W, h, 1)
            }
        }
    }
}
