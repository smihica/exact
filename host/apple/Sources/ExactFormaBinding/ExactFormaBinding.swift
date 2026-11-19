// The binding core (LLP 1121 D12): a design token resolved for a place, for
// code outside the plan, in the process (`ExactForma`) and for a native
// module's library over the hatches' host table. It has no roster row: the
// native module family, which answers a module's bindings through it, links
// and installs it, and an embedder that calls `ExactForma` links that family.
import ExactKit

public enum ExactFormaBinding {
    /// Set `FormaLink`'s slot, before any session is made.
    public static func install() {
        FormaLink.moved = { Formas.moved($0) }
    }
}
