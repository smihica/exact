// @ref LLP 1056 D9 (as amended for expo/exact#188) — a canvas capture's
// presenter half: the runtime names the canvas (`mode: "canvas"` surface
// work), and the Surfaces product encodes its bitmap (`CanvasRuntime.swift`)
// and completes the work through `completeSurface` (kind 8).
import Foundation
import ExactCore

extension ExactSession {
    /// Replay finishes before the snapshot on the presenter's serial queue.
    func drainCanvasCaptures(_ batch: Batch) {
        let work = pendingSurfaceWork
        pendingSurfaceWork = []
        for (op, owner) in work {
            guard op["mode"] as? String == "canvas" else { pendingSurfaceWork.append((op, owner)); continue }
            let ticket = (op["ticket"] as? NSNumber)?.uint64Value ?? 0
            guard generation == owner, runtime.requestActive(ticket) else { continue }
            // Without the Surfaces product no canvas holds a bitmap: the
            // capture completes empty, and the runtime refuses it.
            guard let surfaces = SurfacesLink.installed else {
                DispatchQueue.main.async { [weak self] in self?.completeSurface(ticket, generation: owner, kind: 8) }
                continue
            }
            surfaces.capture(self, op)
        }
    }

    package func completeSurface(_ ticket: UInt64, generation owner: Int, kind: UInt32, body: Data = Data()) {
        guard state != .destroyed, generation == owner, runtime.requestActive(ticket) else { return }
        apply(runtime.fulfillSurface(ticket, kind: kind, body: body, now: now()))
    }
}
