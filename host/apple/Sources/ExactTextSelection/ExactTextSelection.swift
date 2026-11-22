// The text selection family (LLP 1047.001 D10, the roster's `text_selection`):
// iOS's selection of non-editable text (LLP 1115.000 D11.6), which only an
// authored `user-select: text` or `all` makes. An app's composition links
// this module when its plan has one, and installs it before any session is
// made; the core reaches it only through `TextSelectionLink`. Its body is
// iOS's: the Mac's selection is in the core (every Mac set names the
// capability), and tvOS selects no text.
import ExactKit

public enum ExactTextSelection {
    /// Set `TextSelectionLink`'s slot, before any session is made.
    public static func install() {
        #if os(iOS)
        TextSelectionLink.make = { TextSelectionHost($0) }
        #endif
    }
}
