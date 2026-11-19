// The wakes family (LLP 1047.001 D10, the roster's `wakes`; LLP 1133.017,
// LLP 1133.021 D14): packages' wake requests, the background transfer
// carrier and the region carrier, each of which can launch the app with no
// scene. An app's composition links it when its grants name a background
// line, or its plan uses a background transfer's or a geofence's names; the
// composition's launch hooks install it before launch completes.
// The core reaches it through no slot.
import ExactKit

public enum ExactWakeCarriers {
    /// Nothing to set: its hooks are the app delegate's
    /// (`ExactComposition.installWakes`, `handleTransferEvents`).
    public static func install() {}
}
