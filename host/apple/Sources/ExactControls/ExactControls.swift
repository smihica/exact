// The optional form controls (LLP 1047.001 D10, the roster's `controls`):
// switches, checkboxes, radios, ranges, selects, dates and progress, and a
// presenter's segmented tablists and radio groups, installed into
// `ControlLink` by an app whose composition links them.
import ExactKit

public enum ExactControls {
    /// Set `ControlLink`'s slots, before any session is made.
    public static func install() {
        ControlLink.make = { BuiltInControlKinds($0) }
        ControlLink.makeSegments = { SegmentHost($0) }
        #if EXACT_AGENT
        ControlLink.drawnToggle = BuiltInControlKinds.drawnToggle
        #endif
    }
}
