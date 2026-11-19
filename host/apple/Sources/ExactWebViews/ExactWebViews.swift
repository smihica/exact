// Iframes' web views (LLP 1047.001 D10, the roster's `web_views`): the web
// arm's Swift half (LLP 1020). An app's composition links this module when
// its plan has an `iframe`, and installs it before any session is made; the
// core reaches it only through `MediaLink`.
import ExactKit
import ExactHeavyLeaves

public enum ExactWebViews {
    public static func install() {
        ExactHeavyLeaves.install()
        MediaLink.makeWeb = { WebViews() }
    }
}
