// The tool door (LLP 1047.001 D10, the roster's `tools`; LLP 1133.023 D7,
// D15 item 4): a system's call of one of the plan's tools, handed to the
// session's runner. An app's composition links it when its plan has a tool,
// the same fact that links the runner's tool seam; the composition's launch
// hook installs it before the app finishes launching.
import ExactKit

public enum ExactToolDoor {
    /// Nothing to set: the door is installed at launch
    /// (`ExactApp.installToolDoor`), and the core reaches it through no slot.
    public static func install() {}
}

extension ExactApp {
    /// Hear the system's calls of the plan's tools. Install at launch, before
    /// the app finishes launching. `boot` boots a session with no view for a
    /// call that launched the app with none.
    public func installToolDoor(boot: (() -> Void)? = nil) {
        ExactTools.boot = boot
        ExactTools.install()
    }
}
