// Shared-element flights (LLP 1013.000; LLP 1047.001 D10, the roster's
// `flights`), installed into `FlightLink` by an app whose composition links
// them.
import ExactKit

public enum ExactFlights {
    /// Set `FlightLink`'s slot, before any session is made.
    public static func install() {
        FlightLink.make = { BuiltInFlights($0) }
    }
}
