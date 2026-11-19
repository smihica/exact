// The notifications family (LLP 1047.001 D10, the roster's `notifications`):
// the one notification delegate, a notification's declared actions and
// background actions, and push registration. An app's composition links it
// when its grants name `device.notifications`, which push needs too.
import ExactKit

public enum ExactNotifications {
    /// Set `ModuleSeamLink`'s notification and push slots, before any session is made.
    public static func install() {
        ModuleSeamLink.notifications = BuiltInNotifications()
        ModuleSeamLink.listenPush = { PushRegistration.listen() }
    }
}
