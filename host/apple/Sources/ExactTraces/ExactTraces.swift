// The traces family (LLP 1047.001 D10, the roster's `traces`): the recording
// an app starts and stops (`startTrace_EXPERIMENTAL`,
// `stopTrace_EXPERIMENTAL`), its report and its compact copy, and the public
// `ExactSession.startTrace()` and `stopTrace(maxBytes:)`. An app's composition
// links this module when its plan calls the trace commands or its manifest
// names `traces` in `link`; an embedder that calls the public API links it
// and calls `install()`. Frame health's sampler, in the core, records the
// frames; the core reaches the family only through `TraceLink`.
import ExactKit

public enum ExactTraces {
    /// Set `TraceLink`'s slot, before any session is made.
    public static func install() {
        TraceLink.command = { $0.traceCommand($1, $2) }
    }
}
