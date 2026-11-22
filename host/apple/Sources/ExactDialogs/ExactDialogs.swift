// The dialogs family (LLP 1047.001 D10, the roster's `Dialogs`): HTML's
// `dialog` shown by `showModal()` as UIKit's sheet, or as the author's own
// box in the top layer when its author paints it (iOS and tvOS). An app's
// composition links this module when its plan can have a `dialog`, and
// installs it before any session is made; the core reaches it only through
// `DialogLink`. The Mac has no such product: its core top-layer stack
// presents a `dialog` beside a sheet route.
import ExactKit

public enum ExactDialogs {
    public static func install() {
        #if os(iOS) || os(tvOS)
        DialogLink.make = { DialogSheets(presenter: $0) }
        #endif
    }
}
