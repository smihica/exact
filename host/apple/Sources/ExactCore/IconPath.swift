// An icon token's path on Apple (LLP 1121 D10; stage 9 A): an icon with no
// `sf` source and a template SVG arrives as absolute `M`/`L`/`C`/`Z` commands,
// `;` between its SVG's paths, and its viewBox (host/apple/src/icon_path.rs,
// the kernel's parse). Each path is filled on its own, as Linux and the web
// fill them, nonzero unless the icon says even-odd (`symbolFillRule`, stage 14 C2), under the viewBox kept in its aspect (`xMidYMid meet`,
// as Linux's `fitted` and the web's mask). The page's image views and the
// widget snapshot both draw it from here.
import CoreGraphics
#if canImport(UIKit)
import UIKit
package typealias IconImage = UIImage
#else
import AppKit
package typealias IconImage = NSImage
#endif

package enum IconPath {
    /// Each path the commands draw, in viewBox units; none for no valid one.
    package static func paths(_ commands: String) -> [CGPath] {
        commands.split(separator: ";").compactMap { path(String($0)) }
    }

    /// One path's commands as a path; nil for none or a verb it doesn't send.
    package static func path(_ commands: String) -> CGPath? {
        let words = commands.split(separator: " ")
        let path = CGMutablePath()
        var i = 0
        func n() -> CGFloat? { guard i < words.count, let v = Double(words[i]) else { return nil }; i += 1; return CGFloat(v) }
        while i < words.count {
            let verb = words[i]; i += 1
            switch verb {
            case "M": guard let x = n(), let y = n() else { return nil }; path.move(to: CGPoint(x: x, y: y))
            case "L": guard let x = n(), let y = n() else { return nil }; path.addLine(to: CGPoint(x: x, y: y))
            case "C":
                guard let a = n(), let b = n(), let c = n(), let d = n(), let x = n(), let y = n() else { return nil }
                path.addCurve(to: CGPoint(x: x, y: y), control1: CGPoint(x: a, y: b), control2: CGPoint(x: c, y: d))
            case "Z": path.closeSubpath()
            default: return nil
            }
        }
        return path.isEmpty ? nil : path
    }

    /// An icon's fill rule from its `symbolFillRule`: even-odd for `evenodd`, else nonzero.
    package static func rule(_ text: String?) -> CGPathFillRule { text == "evenodd" ? .evenOdd : .winding }

    /// `min-x min-y width height`, a 24-unit square when it says none.
    package static func viewBox(_ text: String) -> CGRect {
        let v = text.split(whereSeparator: { $0.isWhitespace || $0 == "," }).map { Double($0) }
        guard v.count == 4, let x = v[0], let y = v[1], let w = v[2], let h = v[3],
              [x, y, w, h].allSatisfy({ $0.isFinite }), w > 0, h > 0 else { return CGRect(x: 0, y: 0, width: 24, height: 24) }
        return CGRect(x: x, y: y, width: w, height: h)
    }

    /// The viewBox fitted into `rect`, its aspect kept and centred.
    package static func fitted(_ box: CGRect, in rect: CGRect) -> CGAffineTransform {
        let scale = min(rect.width / box.width, rect.height / box.height)
        let dx = rect.minX + (rect.width - box.width * scale) / 2 - box.minX * scale
        let dy = rect.minY + (rect.height - box.height * scale) / 2 - box.minY * scale
        return CGAffineTransform(a: scale, b: 0, c: 0, d: scale, tx: dx, ty: dy)
    }

    /// The natural glyph box: its viewBox's aspect, its longer side at one em.
    package static func natural(_ box: CGRect, em: CGFloat) -> CGSize {
        let scale = em / max(box.width, box.height)
        return CGSize(width: box.width * scale, height: box.height * scale)
    }

    /// The CSS object viewport. Under `fill` it fills the content box;
    /// SVG's default `xMidYMid meet` centres the viewBox inside it.
    package static func placed(viewBox: CGRect, em: CGFloat, fit: String, position: [BatchValue]? = nil, in box: CGRect) -> CGRect {
        placed(natural: natural(viewBox, em: em), fit: fit, position: position, in: box)
    }

    /// Place a native symbol's measured box or a template's natural viewport.
    package static func placed(natural size: CGSize, fit: String, position: [BatchValue]? = nil, in box: CGRect) -> CGRect {
        guard size.width > 0, size.height > 0, box.width > 0, box.height > 0 else { return .zero }
        let contained = min(box.width / size.width, box.height / size.height)
        let scale: CGFloat
        switch fit {
        case "cover": scale = max(box.width / size.width, box.height / size.height)
        case "none": scale = 1
        case "scale-down": scale = min(1, contained)
        default: scale = contained
        }
        let width = fit == "fill" ? box.width : size.width * scale
        let height = fit == "fill" ? box.height : size.height * scale
        let dx = box.width - width, dy = box.height - height
        if let position, position.count == 2,
           let x = position[0].length(basis: dx), let y = position[1].length(basis: dy), x.isFinite, y.isFinite {
            return CGRect(x: box.minX + x, y: box.minY + y, width: width, height: height)
        }
        return CGRect(x: box.minX + dx / 2, y: box.minY + dy / 2, width: width, height: height)
    }

    /// Fill each path by `rule`, placed into `rect` (a top-left origin), in `ctx`'s fill colour.
    package static func fill(_ paths: [CGPath], viewBox text: String, in rect: CGRect, rule: CGPathFillRule = .winding, _ ctx: CGContext) {
        var transform = fitted(viewBox(text), in: rect)
        for p in paths { if let placed = p.copy(using: &transform) { ctx.addPath(placed); ctx.fillPath(using: rule) } }
    }

    /// Fit the viewBox to `size` and draw the crop in those coordinates.
    package static func image(_ commands: String, viewBox text: String, size: CGSize, crop: CGRect? = nil,
                              rule: CGPathFillRule = .winding) -> IconImage? {
        let paths = paths(commands)
        guard size.width > 0, size.height > 0, !paths.isEmpty else { return nil }
        let visible = crop ?? CGRect(origin: .zero, size: size)
        guard !visible.isEmpty else { return nil }
        let viewport = CGRect(x: -visible.minX, y: -visible.minY, width: size.width, height: size.height)
        #if os(watchOS)
        return nil // (no image renderer on watchOS; no watch host draws the path yet)
        #elseif canImport(UIKit)
        let format = UIGraphicsImageRendererFormat.preferred(); format.opaque = false
        let image = UIGraphicsImageRenderer(size: visible.size, format: format).image { ctx in
            ctx.cgContext.setFillColor(UIColor.black.cgColor); fill(paths, viewBox: text, in: viewport, rule: rule, ctx.cgContext)
        }
        return image.withRenderingMode(.alwaysTemplate)
        #else
        // A vector image: AppKit draws it again at the size and scale it shows at.
        let image = NSImage(size: visible.size, flipped: true) { drawn in
            guard let cg = NSGraphicsContext.current?.cgContext else { return false }
            let rect = viewport.applying(CGAffineTransform(scaleX: drawn.width / visible.width, y: drawn.height / visible.height))
                .offsetBy(dx: drawn.minX, dy: drawn.minY)
            cg.setFillColor(NSColor.black.cgColor); fill(paths, viewBox: text, in: rect, rule: rule, cg)
            return true
        }
        image.isTemplate = true
        return image
        #endif
    }
}
