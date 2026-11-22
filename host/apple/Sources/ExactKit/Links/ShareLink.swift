// @ref LLP 1047.001 D10 — sharing, the `ExactSharing` product (roster
// `share`): `share(…)`, the system share sheet (LLP 1069.003), a link's
// context-menu Share item, and a received share's claim from the share
// extension's inbox (LLP 1133.024). The core reaches all three only through
// what this file declares. The product is linked by a plan's `share`
// command, by a link that gets the host's own context menu, or by the
// manifest's `share_target`. Without it a `share` is refused into the
// journal, which is all its outcome ever is (D2), a link's menu offers no
// Share item, and no share is received.
#if canImport(UIKit)
import UIKit
#endif

/// What a session asks of sharing; one object serves every session.
package protocol ShareHosting: AnyObject {
    /// `share(title, text, url, files)`, from the input on `source`.
    func share(_ session: ExactSession, _ args: [Any], source: UInt32?)
    /// Before the boot: a received share becomes the launch location.
    func claimCold(_ session: ExactSession)
    /// Once storage is configured: the cold share's files land, and the
    /// inbox is claimed from now on.
    func land(_ session: ExactSession)
    #if os(iOS)
    /// A link's context-menu Share item: the sheet with `url`, off `view`'s
    /// `rect`; `log` gets the journal line.
    func shareLink(_ url: URL, from view: UIView, at rect: CGRect, log: @escaping (String) -> Void)
    #endif
    #if EXACT_AGENT
    // What the agent asks: development only, so production strips them.
    /// The agent answered a share's hold: its real sheet closes.
    func answered(_ session: ExactSession, _ reply: [String: Any])
    /// The driver's `share:` form, delivered at the navigation root.
    func receive(_ session: ExactSession, _ request: [String: Any]) -> [String: Any]
    #if os(iOS)
    /// `back` with a real sheet up: the newest closes; false with none.
    func agentBack(_ session: ExactSession) -> Bool
    /// Whether a real sheet has yet to finish arriving.
    func arriving(_ session: ExactSession) -> Bool
    #endif
    #endif
}

/// No sharing: a `share` is refused, and nothing is received.
package final class NoShare: ShareHosting {
    package init() {}
    package func share(_ session: ExactSession, _ args: [Any], source: UInt32?) {
        session.log("share: refused: this build links no share sheet")
    }
    package func claimCold(_ session: ExactSession) {}
    package func land(_ session: ExactSession) {}
    #if os(iOS)
    package func shareLink(_ url: URL, from view: UIView, at rect: CGRect, log: @escaping (String) -> Void) {}
    #endif
    #if EXACT_AGENT
    package func answered(_ session: ExactSession, _ reply: [String: Any]) {}
    package func receive(_ session: ExactSession, _ request: [String: Any]) -> [String: Any] {
        ["error": "share: this build links no share target"]
    }
    #if os(iOS)
    package func agentBack(_ session: ExactSession) -> Bool { false }
    package func arriving(_ session: ExactSession) -> Bool { false }
    #endif
    #endif
}

/// The slot `ExactSharing.install()` sets before any session is made, read
/// on the main thread.
package enum ShareLink {
    package static var installed: (any ShareHosting)?
}

extension ExactSession {
    /// Sharing, or none.
    var shareHost: any ShareHosting { ShareLink.installed ?? NoShare() }
}
