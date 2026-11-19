// The heavy leaves' hold (LLP 1068 §5.1), linked by the products that make
// heavy leaves (ExactVideo, ExactWebViews, ExactNativeViews), each of whose
// `install()` installs it.
import ExactKit

package enum ExactHeavyLeaves {
    package static func install() {
        LeavesLink.make = { HeavyLeaves($0) }
    }
}
