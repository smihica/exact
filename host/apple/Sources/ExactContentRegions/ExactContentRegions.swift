// Content regions (LLP 1047.001 D10, the roster's `content_regions`): long
// documents a worker lays out and rasters, installed into
// `ContentRegionLink` by an app whose entry registers one. The region
// controller is the Mac's; iOS presents none.
import ExactKit

public enum ExactContentRegions {
    /// Set `ContentRegionLink`'s slots, before any session is made.
    public static func install() {
        #if os(macOS)
        ContentRegionLink.make = { RegionController($0) }
        ContentRegionLink.isSurface = { $0 is RegionSurfaceMac }
        #endif
    }
}
