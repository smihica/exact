// Staged batches on UIKit: not yet; every batch applies in its own turn
// (LLP 1072 T13; QUEUE.md).
#if os(iOS) || os(tvOS)
import UIKit
import ExactCore

extension Presenter {
    var revealsStage: Bool { false }
    var revealing: StagePlan? { get { nil } set {} }
    func stagePlan(_ batch: Batch) -> StagePlan? { nil }
    func prepare(_ plan: StagePlan, until deadline: CFTimeInterval) {}
    func abandon(_ plan: StagePlan?) {}
}

extension NodeView {
    var offstageAlpha: CGFloat? { nil }
    var held: Bool { false }
}
#endif
