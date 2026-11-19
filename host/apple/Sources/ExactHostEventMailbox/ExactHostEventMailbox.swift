// The host events family (LLP 1047.001 D10, the roster's `host_events`;
// LLP 1133.001.000 D2): system calls on the app's or the scene's delegate
// that a package takes. An app's composition links it when its grants name
// one a listed event needs signed (`device.cloudkit`).
import ExactKit

public enum ExactHostEventMailbox {
    /// Set `ModuleSeamLink`'s host event slots, before any session is made.
    public static func install() {
        ModuleSeamLink.listenHostEvents = { HostEvents.listen() }
        ModuleSeamLink.sessionEnded = { HostEvents.ended($0) }
    }
}
