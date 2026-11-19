// What the launch's boot cost, part by part (`exact.boot.parts` in the launch
// report): the runner's paragraphs measured by the host, then each op kind
// and each pass of the first batch, timed only while the boot runs, so no
// later batch pays for it.
import Foundation
import QuartzCore

package enum ApplyProfile {
    /// On only while the boot applies its first batch (`Session.finishBoot`).
    nonisolated(unsafe) static var on = false
    /// Milliseconds by part: `op.create`, `op.style`, …, `pass.navigation`, ….
    nonisolated(unsafe) private(set) static var parts: [String: Double] = [:]

    static func begin() { parts = [:]; on = true }
    static func end() -> [String: Double] { on = false; defer { parts = [:] }; return parts }

    /// Adds the time since `since` (a `CACurrentMediaTime`) to `part`.
    @inline(__always) static func add(_ part: String, since: Double) {
        parts[part, default: 0] += (CACurrentMediaTime() - since) * 1000
    }

    /// One more of `part`, counted rather than timed (shown as a count).
    @inline(__always) static func count(_ part: String) { parts["#" + part, default: 0] += 1 }

    /// `body`, timed as `part` while on.
    @inline(__always) package static func time<T>(_ part: String, _ body: () -> T) -> T {
        guard on else { return body() }
        let t = CACurrentMediaTime()
        defer { add(part, since: t) }
        return body()
    }
}
