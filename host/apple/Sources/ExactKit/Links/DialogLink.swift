// @ref LLP 1047.001 D10 — the `dialog` element as a sheet (LLP 1115.000
// D11.5), as the core and the menus reach it: a dialog that is not a
// confirmation by its rows, presented over the page with its content laid
// out by the kernel. Which dialogs are sheets and which are confirmations
// is decided outside it, from their shape (`DialogShape`).
//
// Without the family the presenter holds `NoDialogs`, which answers what a
// plan with no sheet observes: nothing is presented, owned or retiring. A
// `dialog` that would be a sheet is not presented, and the journal says so.
#if os(iOS) || os(tvOS)
import UIKit

/// What the core and the menus ask of a presenter's dialog sheets.
package protocol DialogHosting: AnyObject {
    // Lifecycle, in the menus' pass (`MenuHost.sync`, `reset`) and the presenter's op loop.
    func sync()
    func applied(_ op: BatchOp)
    func framed(_ node: NodeView)
    func children(_ parent: UIView, _ wanted: [NodeView])
    func reset()

    /// Present `dialog` over `owner`; false when it cannot be now.
    func show(_ dialog: NodeView, from source: NodeView?, over owner: UIViewController) -> Bool
    func close(_ dialog: NodeView, animated: Bool)
    /// Shown, or still on screen while its sheet leaves.
    func owns(_ node: NodeView) -> Bool
    func isOpen(_ node: NodeView) -> Bool
    var top: UIViewController? { get }
    var topDialog: NodeView? { get }
    var inTransition: Bool { get }

    /// A dialog's authored parent while it is in its sheet; nil for any other node.
    func authoredParent(of node: NodeView) -> UIView?
    func shows(_ view: UIView) -> Bool
    func controllers(holding view: UIView) -> [UIViewController]
    func pressLifetime(of view: UIView) -> (() -> Bool)?
    func preservesFocus(_ id: UInt32) -> Bool
    var containingBlocks: [(UInt32, HostBlock)] { get }
    var presented: [TopLayerBox] { get }
    func surfaces(includingRetiring: Bool) -> [SessionSurface]
    /// Retire the sheets over `controller` as the route `route` goes; the returned work ends them.
    func retire(over controller: UIViewController, removing route: NodeView) -> () -> Void
    /// Present again a sheet a route's dismissal took down; false while it waits.
    func resumeAfterRouteDismissal() -> Bool

    var observation: [String: Any]? { get }
    func inspectionOwns(_ view: UIView) -> Bool
    #if EXACT_AGENT
    /// The agent's `back` on the top sheet: nil with none, else why it was refused ("" when it went).
    func agentCancel() -> String?
    #endif
}

extension DialogHosting {
    /// Closed as its own close button closes it: animated unless the agent froze motion.
    package func close(_ dialog: NodeView) { close(dialog, animated: !ExactEnv.agentFreezes) }
}

/// The slot is set before any session is made, and read on the main thread.
package enum DialogLink {
    package static var make: ((Presenter) -> any DialogHosting)? = { DialogSheets(presenter: $0) }
}

/// Dialog sheets where the composition links none.
package final class NoDialogs: DialogHosting {
    private unowned let presenter: Presenter
    package init(_ presenter: Presenter) { self.presenter = presenter }
    package func sync() {}
    package func applied(_ op: BatchOp) {}
    package func framed(_ node: NodeView) {}
    package func children(_ parent: UIView, _ wanted: [NodeView]) {}
    package func reset() {}
    package func show(_ dialog: NodeView, from source: NodeView?, over owner: UIViewController) -> Bool { false }
    package func close(_ dialog: NodeView, animated: Bool) {}
    package func owns(_ node: NodeView) -> Bool { false }
    package func isOpen(_ node: NodeView) -> Bool { false }
    package var top: UIViewController? { nil }
    package var topDialog: NodeView? { nil }
    package var inTransition: Bool { false }
    package func authoredParent(of node: NodeView) -> UIView? { nil }
    /// A view in a popover's surface is this session's too, as `DialogSheets.shows` answers it.
    package func shows(_ view: UIView) -> Bool {
        guard let surface = presenter.surface(containing: view) else { return false }
        return surface.kind == .dialog || surface.kind == .popover
    }
    package func controllers(holding view: UIView) -> [UIViewController] { [] }
    package func pressLifetime(of view: UIView) -> (() -> Bool)? { nil }
    package func preservesFocus(_ id: UInt32) -> Bool { false }
    package var containingBlocks: [(UInt32, HostBlock)] { [] }
    package var presented: [TopLayerBox] { [] }
    package func surfaces(includingRetiring: Bool) -> [SessionSurface] { [] }
    package func retire(over controller: UIViewController, removing route: NodeView) -> () -> Void { {} }
    /// Nothing to present again, once no route or modal is moving.
    package func resumeAfterRouteDismissal() -> Bool { !presenter.modals.inTransition && !presenter.navigation.inTransition }
    package var observation: [String: Any]? { nil }
    package func inspectionOwns(_ view: UIView) -> Bool { false }
    #if EXACT_AGENT
    package func agentCancel() -> String? { nil }
    #endif
}

/// What makes a `dialog` a sheet, a confirmation or the author's own box: its
/// shape, read the same whether the dialogs family is linked or not.
package enum DialogShape {
    /// Whether `dialog`'s author painted its surface, which makes it the
    /// author's box in the top layer rather than the platform's sheet or
    /// alert (LLP 1035.001 D11).
    package static func paintsOwnSurface(_ dialog: NodeView) -> Bool {
        dialog.wrote("background_color") || dialog.wrote("background_image")
    }
}
#endif
