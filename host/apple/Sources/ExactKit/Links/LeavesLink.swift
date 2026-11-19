// @ref LLP 1068 §5.1 — the heavy leaves' hold, `ExactHeavyLeaves`: a video,
// web view or native module view whose creation costs more than a frame
// waits while its list moves. The three products that make those leaves,
// ExactVideo, ExactWebViews and ExactNativeViews, each link it and install
// it; an app with none of them links none of it, and the presenter has no
// hold (`Presenter.leaves` is nil).
#if os(macOS)
import AppKit
#else
import UIKit
#endif

/// What the presenter asks of the hold.
package protocol HeavyLeafHolding: AnyObject {
    /// A new video's, module's or iframe's node: its platform view is made
    /// now, or held.
    func embed(_ node: NodeView)
    /// A batch applied; `moved` when rows may have come or moved.
    func batchApplied(moved: Bool)
    /// The list moved.
    func scrolled()
    /// After a video node's create ops.
    func created(_ node: NodeView)
    /// Whether `node`'s platform view waits.
    func isPending(_ node: NodeView) -> Bool
    /// Every waiting leaf is made now (the agent's settle).
    func settle()
    /// Every waiting leaf is dropped (the presenter's reset).
    func reset()
    #if os(macOS)
    /// A held leaf's box was clicked.
    func pressed(_ node: NodeView)
    #endif
    /// `state`'s part of the pool section.
    var observation: [String: Any] { get }
}

/// The hold, installed by the products that make heavy leaves.
package enum LeavesLink {
    package static var make: ((Presenter) -> HeavyLeafHolding)?
}
