// The app-level hooks of the families the composition links (LLP 1047.001
// D10): what the host executables and an embedder's app delegate call, each
// a no-op when its family is not linked, so the executables build the same
// for every composition. A wake, a notification's background action or a
// tool call can launch the app with no scene, so the app delegate installs
// these before `application(_:didFinishLaunchingWithOptions:)` returns; each
// installs once.
import ExactKit
import Foundation
#if !os(tvOS)
import UserNotifications
#endif

extension ExactComposition {
    /// The delegate methods of the host events the build takes, added
    /// before any scene connects (LLP 1133.001.000 D2).
    public static func installHostEvents(on app: ExactApp, applicationDelegate: AnyClass?, sceneDelegate: AnyClass?,
                                         session: @escaping () -> ExactSession? = { nil }) {
        #if EXACT_LINK_HOST_EVENTS
        app.installHostEvents(applicationDelegate: applicationDelegate, sceneDelegate: sceneDelegate, session: session)
        #endif
    }

    /// The host event a scene's connection options carry, when one launched the app.
    public static func hostEvents(on app: ExactApp, connectionOptions: NSObject) {
        #if EXACT_LINK_HOST_EVENTS
        app.hostEvents(connectionOptions: connectionOptions)
        #endif
    }

    /// The notification delegate: `boot` takes a session a background
    /// action launched with no scene, `activate` brings one forward.
    public static func installNotificationClicks(on app: ExactApp, boot: ((ExactSession) -> Void)? = nil,
                                                 activate: ((ExactSession) -> Void)? = nil) {
        #if !os(tvOS) && EXACT_LINK_NOTIFICATIONS
        app.installNotificationClicks(boot: boot, activate: activate)
        #endif
    }

    #if !os(tvOS)
    /// A notification click that launched the app.
    public static func openNotification(on app: ExactApp, _ response: UNNotificationResponse) {
        #if EXACT_LINK_NOTIFICATIONS
        app.openNotification(response)
        #endif
    }
    #endif

    /// The system's answer to a push registration, for a delegate that answers it itself.
    public static func registeredForRemoteNotifications(on app: ExactApp, deviceToken: Data) {
        #if EXACT_LINK_NOTIFICATIONS
        app.registeredForRemoteNotifications(deviceToken: deviceToken)
        #endif
    }

    public static func failedToRegisterForRemoteNotifications(on app: ExactApp, error: Error) {
        #if EXACT_LINK_NOTIFICATIONS
        app.failedToRegisterForRemoteNotifications(error: error)
        #endif
    }

    /// The wake identifiers and the transfer and region carriers; `boot`
    /// takes a wake that launched the app with no scene (LLP 1133.017 D7).
    public static func installWakes(on app: ExactApp, boot: (() -> Void)? = nil) {
        #if EXACT_LINK_WAKES
        app.installWakes(boot: boot)
        #endif
    }

    /// A background transfer session's events; without the wakes family
    /// there is no such session, and the system is told at once.
    public static func handleTransferEvents(on app: ExactApp, _ identifier: String, completion: @escaping () -> Void) {
        #if EXACT_LINK_WAKES
        app.handleTransferEvents(identifier, completion: completion)
        #else
        completion()
        #endif
    }
}
