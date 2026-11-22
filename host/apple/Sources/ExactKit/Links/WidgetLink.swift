// @ref LLP 1047.001 D10 — the widgets family (LLP 1126 D8), its own product:
// the running app's publication of its entries' generation to the App Group,
// the timelines' reload, and the observer of the store counter an extension's
// writes advance. The core reaches it only through this slot, which the
// family's `install()` sets. Without it the app publishes nothing and
// observes nothing, as an app with no entry has nothing to publish.
import Foundation
import ExactCore

/// What the app asks of the widgets family (WidgetPublication.swift).
package protocol WidgetPublishing {
    /// After the first pixel: `assets`' generation, copied to the App Group
    /// off the main thread, then the timelines reloaded once.
    func publishAfterFirstPixel(_ assets: AssetResolver)
    /// The store counter's observer, which the app keeps alive; nil for an
    /// app with no widget. An explicit container is a test's or an embedder's.
    func observeStore(root: URL?, appID: String?, kinds: [String]?, foreground: [Notification.Name],
                      needsRefresh: @escaping () -> Bool, changed: @escaping (Bool, @escaping (Bool) -> Void) -> Void) -> AnyObject?
    /// After the sessions applied a store change: the timelines of `kinds` reload.
    @MainActor func reloadAfterStoreChange(kinds: [String]?)
    /// Why the last publication failed, which the dev menu shows.
    var publicationFailure: String? { get }
}

/// The slot is set before any session is made, and read on the main thread.
package enum WidgetLink {
    package static var publication: (any WidgetPublishing)?
}
