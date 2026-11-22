import ExactCore
import Foundation

extension ExactSession {
    package func surfaceRecord(_ name: String, _ json: String?) {
        publishSurfaceRecord { $0.surfaceRecord(name, json) }
    }

    /// A surface's record for the runtime, now or, while a batch is being
    /// applied, once it is in.
    package func publishSurfaceRecord(_ record: @escaping (Runtime) -> Batch) {
        guard state != .destroyed else { return }
        if applying { pendingSurfaceRecords.append(record); return }
        apply(record(runtime))
    }
}
