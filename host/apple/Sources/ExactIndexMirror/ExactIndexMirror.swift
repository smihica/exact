// The content index (LLP 1047.001 D10, the roster's `content_index`; LLP
// 1133.023 D4, D9): the mirror `@exact/search-index` writes, its Spotlight
// index, and the entries a widget's or an intent's entity parameter offers.
// The composition's launch hook installs it before the app finishes
// launching, so a search result or the reindex delegate finds it.
import ExactKit

public enum ExactIndexMirror {
    /// Nothing to set: the index is installed at launch
    /// (`ExactApp.installContentIndex`), and the core reaches it through no slot.
    public static func install() {}
}

extension ExactApp {
    /// Hear `@exact/search-index`'s changes and the generated intents'
    /// queries, and answer Spotlight's reindex delegate. Install at launch.
    public func installContentIndex() {
        ExactContentIndex.install()
    }
}
