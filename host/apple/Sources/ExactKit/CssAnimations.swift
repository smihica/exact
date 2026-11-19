// @ref LLP 1055 D7 — the colour and path readings CSS animations' keyframes
// share with an SVG scene, and how a spec's values hash. Lowering and playing
// the animations are the animations family's (`ExactAnimations`).
import CoreGraphics
import Foundation
import QuartzCore

private func num(_ v: Any?) -> Double { (v as? NSNumber)?.doubleValue ?? 0 }
package func nums(_ v: Any?) -> [Double] { (v as? [Any])?.map(num) ?? [] }

/// A colour the host sent: `[r,g,b,a]` (0–255), a `light-dark()` pair, or a
/// colour in its own space, `{"cs": [{"s", "v"}…]}` (LLP 1100 D2).
package func paintColor(_ v: Any?, dark: Bool, tint: PlatformColor? = nil) -> CGColor? {
    // Restore the typed color, including the ICC handle kept by the payload
    // adapter, and use the same conversion/intent as every other paint path.
    func value(_ v: Any?) -> BatchValue {
        switch v {
        case let p as ProfileSpaces.Handle: return .profile(p)
        case let n as NSNumber: return .number(n.doubleValue)
        case let s as String: return .string(s)
        case let a as [Any]: return .array(a.map { value($0) })
        case let o as [String: Any]: return .object(o.mapValues { value($0) })
        default: return .null
        }
    }
    return value(v).cgColor(dark: dark, tint: tint)
}

/// `[0,x,y, 1,x,y, 2,x1,y1,x2,y2,x,y, 3]`: move, line, cubic, close.
package func svgPath(_ v: Any?) -> CGPath {
    if let prepared = v as? PreparedPath { return prepared.path }
    let n = nums(v), p = CGMutablePath()
    var i = 0
    while i < n.count {
        switch Int(n[i]) {
        case 0 where i + 2 < n.count: p.move(to: CGPoint(x: n[i + 1], y: n[i + 2])); i += 3
        case 1 where i + 2 < n.count: p.addLine(to: CGPoint(x: n[i + 1], y: n[i + 2])); i += 3
        case 2 where i + 6 < n.count:
            p.addCurve(to: CGPoint(x: n[i + 5], y: n[i + 6]), control1: CGPoint(x: n[i + 1], y: n[i + 2]), control2: CGPoint(x: n[i + 3], y: n[i + 4])); i += 7
        case 3: p.closeSubpath(); i += 1
        default: return p
        }
    }
    return p
}

/// The scene's path parser, for the owner's preparation (`SvgPrepare`).
enum SvgGeometry {
    static func path(_ v: Any?) -> CGPath { svgPath(v) }
}

/// A circle about the origin from (r, 0), toward positive y (SVG 2 §10.3),
/// the same four cubics the kernel draws, so a radius animation interpolates
/// between circles.
package func svgCircle(_ r: Double) -> CGPath {
    let k = r * 0.5522848, p = CGMutablePath()
    p.move(to: CGPoint(x: r, y: 0))
    p.addCurve(to: CGPoint(x: 0, y: r), control1: CGPoint(x: r, y: k), control2: CGPoint(x: k, y: r))
    p.addCurve(to: CGPoint(x: -r, y: 0), control1: CGPoint(x: -k, y: r), control2: CGPoint(x: -r, y: k))
    p.addCurve(to: CGPoint(x: 0, y: -r), control1: CGPoint(x: -r, y: -k), control2: CGPoint(x: -k, y: -r))
    p.addCurve(to: CGPoint(x: r, y: 0), control1: CGPoint(x: k, y: -r), control2: CGPoint(x: r, y: -k))
    p.closeSubpath()
    return p
}

/// CSS animations (LLP 1055 D7). The core keeps how a spec's values hash,
/// which a prepared path shares; lowering and playing them are the
/// animations family's (`ExactAnimations`).
package enum CssAnimations {
    /// A spec's values, hashed: describing them as text was most of a
    /// scene's cost on a list's rows.
    /// A batch's numbers arrive as `NSNumber`: that case goes first, as a
    /// class cast, because `as Double` bridges and was most of a list
    /// row's animation cost.
    package static func digest(_ v: Any?, into h: inout Hasher) {
        switch v {
        case let n as NSNumber: h.combine(n.doubleValue.bitPattern)
        case let n as Double: h.combine(n.bitPattern)
        case let s as String: h.combine(s)
        case let p as ProfileSpaces.Handle: h.combine(p.key)
        case let a as [Any]: h.combine(a.count); for x in a { digest(x, into: &h) }
        // A prepared path hashes as the numbers it was built from.
        case let p as PreparedPath: h.combine(p.digest)
        case let d as [String: Any]:
            // A spec's prepared parts ("#") are made from the rest.
            let keys = d.keys.filter { $0 != "#" }.sorted()
            h.combine(keys.count); for k in keys { h.combine(k); digest(d[k], into: &h) }
        case .none: h.combine(0 as UInt8)
        case let .some(other): h.combine(String(describing: other))
        }
    }
}
