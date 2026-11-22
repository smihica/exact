// Sharing (LLP 1047.001 D10, the roster's `share`): `share(…)`'s sheet, a
// link's context-menu Share item and a received share's claim. An app's
// composition links this module when its plan shares, has a link with the
// host's own context menu, or its manifest declares `share_target`, and
// installs it before any session is made; the core reaches it only through
// `ShareLink`.
import ExactKit

public enum ExactSharing {
    public static func install() {
        ShareLink.installed = BuiltInShare()
    }
}
