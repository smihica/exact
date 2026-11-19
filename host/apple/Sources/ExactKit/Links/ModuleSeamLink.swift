// @ref LLP 1047.001 D10 — the module seam's session-side families (cut list
// B2), each its own product: notifications (with push registration), host
// events and live activities. The core reaches them only through what this
// file declares; each family's `install()` sets its slots. Without one a
// session declares no notification categories (it is ready at once), holds
// no background action, registers for no push, hears no host event and
// sends no activity. Wakes, transfers and regions reach the core through
// none of these: the composition's launch hooks install them.
/// What a session asks of notifications (NotifyActions.swift).
package protocol NotificationHosting: AnyObject {
    /// Registration of the plan's categories, before data code runs; `ready` follows.
    func declareCategories(_ session: ExactSession, ready: @escaping () -> Void)
    /// The plan restarted: its background actions are replaced.
    func replaceBackgroundActions(_ session: ExactSession)
    /// Responses that arrived before the session could take them.
    func deliverResponses(_ session: ExactSession)
    /// A background action begins, unless it expired while queued.
    func beginBackgroundAction(_ session: ExactSession, _ id: String) -> Bool
    /// After work lands: a background action it ended lets the app go.
    func backgroundActionsApplied(_ session: ExactSession)
    /// The session ends: every background action it held lets the app go.
    func endBackgroundActions(_ session: ExactSession)
}

/// No notifications: categories are ready at once, and no background action begins.
package final class NoNotifications: NotificationHosting {
    package init() {}
    package func declareCategories(_ session: ExactSession, ready: @escaping () -> Void) { ready() }
    package func replaceBackgroundActions(_ session: ExactSession) {}
    package func deliverResponses(_ session: ExactSession) {}
    package func beginBackgroundAction(_ session: ExactSession, _ id: String) -> Bool { false }
    package func backgroundActionsApplied(_ session: ExactSession) {}
    package func endBackgroundActions(_ session: ExactSession) {}
}

/// What a session asks of its live activities (Activities.swift).
package protocol ActivityOutputting: AnyObject {
    /// An `activity` op from a batch.
    func enqueue(_ op: [String: Any])
    /// Once data has activated: what was queued is sent.
    func drain()
}

/// The slots are set before any session is made, and read on the main thread.
package enum ModuleSeamLink {
    /// Notification actions and responses.
    package static var notifications: (any NotificationHosting)?
    /// Push registration, listened for as a session is made.
    package static var listenPush: (() -> Void)?
    /// Host events, listened for as a session is made.
    package static var listenHostEvents: (() -> Void)?
    /// A session ended: the host events queued for it go to the first live
    /// session, which hears that they arrived; with none, they stay held.
    package static var sessionEnded: ((String) -> Void)?
    /// Whether the app ships the activity arm, and a session's activity output.
    package static var activitiesPackaged: (() -> Bool)?
    package static var makeActivities: ((ExactSession) -> any ActivityOutputting)?
}

extension ExactSession {
    /// The session's notifications, or none.
    var notificationHost: any NotificationHosting { ModuleSeamLink.notifications ?? NoNotifications() }
}
