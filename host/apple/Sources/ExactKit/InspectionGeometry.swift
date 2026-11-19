import Foundation
#if os(macOS)
import AppKit
#else
import UIKit
#endif

package enum InspectionValue {
    package static func r2(_ value: CGFloat) -> Double { (Double(value) * 100).rounded() / 100 }
}

package enum InspectionGeometry {
    package static func rect(_ rect: CGRect) -> [String: Any] {
        ["x": InspectionValue.r2(rect.minX), "y": InspectionValue.r2(rect.minY),
         "w": InspectionValue.r2(rect.width), "h": InspectionValue.r2(rect.height)]
    }

    #if os(macOS)
    package static func box(_ bounds: CGRect, of view: NSView, viewport: NSScrollView) -> CGRect {
        if (view as? NodeView)?.placedAncestor?.placementHidden == true { return .zero }
        let clip = viewport.contentView
        if let node = view as? NodeView, node.placedAncestor?.placement != nil || node.drawnOffFrame {
            let rect = node.drawnRect(bounds, in: clip)
            return CGRect(x: rect.origin.x - clip.bounds.origin.x, y: rect.origin.y - clip.bounds.origin.y,
                          width: rect.width, height: rect.height)
        }
        let rect = clip.convertAcross(bounds.applying(view.layer?.affineTransform() ?? .identity), from: view)
        return CGRect(x: rect.origin.x - clip.bounds.origin.x, y: rect.origin.y - clip.bounds.origin.y,
                      width: rect.width, height: rect.height)
    }
    #else
    package static func box(_ bounds: CGRect, of view: UIView, viewport: UIScrollView) -> CGRect {
        if (view as? NodeView)?.placedAncestor?.placementHidden == true { return .zero }
        let origin = viewport.pageOrigin
        if let node = view as? NodeView, node.placedAncestor?.placement != nil {
            let rect = node.drawnRect(bounds, in: viewport)
            return CGRect(x: rect.origin.x - origin.x, y: rect.origin.y - origin.y,
                          width: rect.width, height: rect.height)
        }
        let rect = view.convert(bounds, to: viewport)
        return CGRect(x: rect.origin.x - origin.x, y: rect.origin.y - origin.y,
                      width: rect.width, height: rect.height)
    }
    #endif
}

#if os(iOS) || os(tvOS)
/// A node as a contact found it, including the incarnation that prevents a
/// pooled view reused for another node from becoming the original target.
struct HeldNode {
    weak var node: NodeView?
    let id: UInt32, incarnation: UInt64
    init(_ node: NodeView) { self.node = node; id = node.id; incarnation = node.incarnation }
    func live(in presenter: Presenter) -> NodeView? {
        guard let node, presenter.views[id] === node, node.id == id, node.incarnation == incarnation else { return nil }
        return node
    }
}

/// A header button's bar item that the bar shows no control for; `overflowed`,
/// that UIKit moved it into the bar's overflow menu.
package struct BarItemMissing: Error {
    package var overflowed = false
    package init(overflowed: Bool = false) { self.overflowed = overflowed }
}


extension UIScrollView {
    /// The committed offset along the scrolling axis and whether a finger,
    /// deceleration, or overscroll currently moves it.
    var scrollSample: (Double, String) {
        let inset = adjustedContentInset
        let sideways = contentSize.width + inset.left + inset.right > bounds.width + 0.5
            && contentSize.height + inset.top + inset.bottom <= bounds.height + 0.5
        let at = sideways ? contentOffset.x : contentOffset.y
        let low = sideways ? -inset.left : -inset.top
        let high = max(low, sideways ? contentSize.width + inset.right - bounds.width : contentSize.height + inset.bottom - bounds.height)
        return (Double(at), at < low - 0.5 || at > high + 0.5 ? "b" : isTracking && isDragging ? "t" : isDecelerating ? "d" : "-")
    }
}
#endif
