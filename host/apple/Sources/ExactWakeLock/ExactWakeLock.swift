// The wake lock family (LLP 1047.001 D10, the roster's `wake_lock`): the
// process's lock that `wakelock="screen"` holds (LLP 1125 D11). An app's
// composition links this module when its plan has a `wakelock` that can be
// `screen`, and installs it before any session is made; the core reaches it
// only through `WakeLockLink`.
import ExactKit

public enum ExactWakeLock {
    /// Set `WakeLockLink`'s slot, before any session is made.
    public static func install() {
        WakeLockLink.hold = { WakeLock.hold($0, $1) }
    }
}
