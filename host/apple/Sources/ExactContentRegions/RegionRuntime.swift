// A registered region's requests to the runtime (`exact_content_regions.h`),
// which only the region controller makes.
#if os(macOS)
import CExact.ContentRegions
import ExactCore
import ExactKit
import Foundation

extension Runtime {
    /// The region's current text request, its source copied when it is new.
    func regionRequest(_ id: UInt64, knownSource: UInt64) -> [String: Any]? {
        return on(busy: nil) {
            let length = exact_region_request(rt, id, knownSource)
            let data = Data(bytes: exact_out(rt), count: Int(length))
            return try? JSONSerialization.jsonObject(with: data) as? [String: Any]
        }
    }
    /// A region's artifact told to the runtime, and its fallback.
    func regionComplete(_ artifact: RegionArtifact) -> Batch {
        return on {
            let p = artifact.metadata
            let retained = Unmanaged.passRetained(artifact).toOpaque()
            return read(exact_region_complete(rt, artifact.id,
                ExactMetrics(width: Float(p.width), height: Float(p.height), baseline: Float(p.firstBaseline)),
                retained, { pointer in
                    if let pointer { Unmanaged<RegionArtifact>.fromOpaque(pointer).release() }
                }))
        }
    }
    func regionFallback(_ request: UInt64) -> Batch {
        on { read(exact_region_fallback(rt, request)) }
    }
}
#endif
