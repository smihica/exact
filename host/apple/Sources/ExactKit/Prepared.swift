// What a batch's ops need that is a plain value, built on the owner thread
// as the batch is decoded (LLP 1072 stage 4): the SVG scene's paths, CSS
// animations' readings (`AnimationLink.read`), and a flat leaf's paint. Main
// applies them to layers; it no longer parses them from the batch's numbers.
import CoreGraphics
import Foundation
import QuartzCore

/// A path built off main, carried in an SVG element's payload under "p",
/// with the digest of the numbers it was built from: what the scene hashes
/// a shape's spec by (`SvgScene.drew`) in their place.
package final class PreparedPath {
    package let path: CGPath
    let digest: Int
    package init(_ source: Any) {
        path = SvgGeometry.path(source)
        var h = Hasher()
        CssAnimations.digest(source, into: &h)
        digest = h.finalize()
    }
}

extension FlatPaint {
    init?(_ style: NodeStyle) { self.init(style, colorChannels: { $0.channels(dark: $1) }) }
}

extension Batch {
    /// Build this batch's plain values; on the owner, as it is decoded.
    mutating func prepare() {
        for i in ops.indices {
            switch ops[i].op {
            case .create, .style:
                ops[i].flat = FlatPaint(ops[i].style)
            case .svg:
                if let prepare = SvgLink.prepare, var scene = ops[i].payload["scene"] as? [String: Any] {
                    prepare(&scene)
                    ops[i].payload["scene"] = scene
                }
            case .animations:
                if let read = AnimationLink.read, let specs = ops[i].payload["specs"] as? [[String: Any]] {
                    ops[i].payload["specs"] = read(specs)
                }
            default: break
            }
        }
    }
}
