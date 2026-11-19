// The Live Activities family (LLP 1047.001 D10, the roster's `activities`;
// LLP 1126.001 WP9): a session's activity output and the arm that loads on
// demand. An app's composition links it when its plan starts, updates or
// ends an activity, or reads them.
import ExactKit

public enum ExactActivities {
    /// Set `ModuleSeamLink`'s activity slots, before any session is made.
    public static func install() {
        ModuleSeamLink.activitiesPackaged = { ActivityLibrary.packaged }
        ModuleSeamLink.makeActivities = { ActivityOutput($0) }
    }
}
