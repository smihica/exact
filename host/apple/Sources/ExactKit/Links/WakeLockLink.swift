// @ref LLP 1047.001 D10 — `wakelock="screen"` (LLP 1125 D11; WakeLock.swift)
// is a family (roster `wake_lock`, a `wakelock` that can be `screen`): the
// core reaches the process's lock only through this slot, which
// `ExactWakeLock.install()` sets. Without it no batch holds the lock, and
// the display dims as it otherwise would.

/// The slot is set before any session is made, and read on the main thread.
package enum WakeLockLink {
    /// A holder (a session's presenter) holds the process's lock or lets it go.
    package static var hold: ((AnyObject, Bool) -> Void)?
}

extension Presenter {
    /// The batch's word on the wake lock, when it says one.
    func applyWakeLock(_ batch: Batch) {
        if let held = batch.wakelock { WakeLockLink.hold?(self, held) }
    }
}
