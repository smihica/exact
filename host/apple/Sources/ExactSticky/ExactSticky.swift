// `position: sticky` (LLP 1083 D4; LLP 1047.001 D10, the roster's `sticky`),
// installed into `StickyLink` by an app whose composition links it.
import ExactKit

public enum ExactSticky {
    /// Set `StickyLink`'s slot, before any session is made.
    public static func install() {
        StickyLink.make = { StickyHost($0) }
    }
}
