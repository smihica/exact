// A presenter's reorders (LLP 1041 §8.5, LLP 1094), installed into
// `ReorderLink`: the one Arrange contact and the one grouped session, the
// handles' gestures and keys, and the grips' custom actions.
import ExactCore
import ExactKit
#if os(macOS)
import AppKit
#else
import UIKit
#endif

final class ReorderHost: ReorderHosting {
    private weak var presenter: Presenter?
    /// The one Arrange contact, until its source settles.
    var lift: ReorderHold?
    /// A grouped session, until its ghost lands.
    var group: ReorderGroupHold?
    /// A test's runtime calls in place of the session's.
    var calls: ReorderCalls?
    var groupCalls: ReorderGroupCalls?
    #if os(macOS)
    private(set) lazy var mouse: MouseDrag = MouseReorder(presenter!)
    #endif

    init(_ presenter: Presenter) { self.presenter = presenter }

    var lifting: Bool { lift != nil }
    func lifts(_ view: UInt32) -> Bool { lift?.lifts(view) == true }
    var groupActive: Bool { group?.active == true }
    func observe(_ ops: [BatchOp]) {
        for op in ops {
            lift?.observe(ReorderState(op.payload))
            group?.observe(ReorderGroupState(op.payload))
        }
        lift?.raiseLifted()
    }
    func abandon() {
        lift?.abandon()
        group?.abandon()
    }
    func takesKeys(_ view: NodeView) -> Bool { view.reorderKeys }
    func key(_ view: NodeView, _ name: String) -> Bool { view.reorderKey(name) }
    #if os(macOS)
    func actions(_ view: NodeView) -> [NSAccessibilityCustomAction]? {
        guard view.reorderGroupList != nil else { return nil }
        return ReorderGroupStep.actions.map { name, step in
            NSAccessibilityCustomAction(name: name) { [weak view] in view?.reorderAction(step) ?? false }
        }
    }
    #else
    func actions(_ view: NodeView) -> [UIAccessibilityCustomAction]? {
        guard view.reorderGroupList != nil else { return nil }
        return ReorderGroupStep.actions.map { name, step in
            UIAccessibilityCustomAction(name: name) { [weak view] _ in view?.reorderAction(step) ?? false }
        }
    }
    func updateGesture(_ view: NodeView) { view.updateReorderGesture() }
    func shouldBegin(_ view: NodeView, _ gesture: UIGestureRecognizer) -> Bool? { view.reorderShouldBegin(gesture) }
    #endif
}

#if os(macOS)
extension MouseReorder: MouseDrag {
    var armed: NodeView? { candidate }
    var engaged: Bool { hold != nil || grouped }
    func arm(_ node: NodeView, event: NSEvent) { down(node, event: event) }
}
#endif

extension Presenter {
    /// This presenter's reorders, as `ExactCollections` installed them.
    var reorderHost: ReorderHost? { reorders as? ReorderHost }
}
