import Foundation

/// Moves on every native layout, scroll, window move and presentation batch,
/// what may have moved a view, so a reader can keep a geometric answer until
/// it does (a canvas's on-screen test, `Canvases.shown`). It tells a
/// presenter's drag handles too, which report a transform handle's geometry
/// (`DragHandles.geometryChanged`).
package final class TransformGeometryHost {
    weak var presenter: Presenter?
    init(_ presenter: Presenter) { self.presenter = presenter }
    private(set) package var epoch = 0
    package func changed() {
        epoch &+= 1
        presenter?.drags.geometryChanged()
    }
}
