// @ref LLP 1047.001 D10 — the binding core (LLP 1121 D12; ExactFormaBinding):
// a design token resolved for a place, for code outside the plan. A native
// module's library asks it over the hatches' host table, so the native module
// family links and installs it; a production app with no native module links
// none of it. The session tells it after each batch, only through this slot;
// without it nothing is asked again after a batch, and no observer hears a
// move.

/// The slot is set before any session is made, and read on the main thread.
package enum FormaLink {
    /// After a batch: each place whose inputs moved is asked again, redrawn,
    /// and its observers told.
    package static var moved: ((ExactSession) -> Void)?
}
