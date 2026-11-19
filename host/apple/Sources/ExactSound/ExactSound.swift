// The voice table's output (LLP 1047.001 D10, the roster's `sound`): the
// sound arm's Swift half (LLP 1096). An app's composition links this module
// when its plan declares a sound or a sequence or runs a voice-table command,
// and installs it before any session is made; the core reaches it only
// through `MediaLink`.
import ExactKit

public enum ExactSound {
    public static func install() {
        MediaLink.makeSound = { SoundOutput($0) }
        #if os(iOS) || os(tvOS)
        MediaLink.claimsAudioSession()
        #endif
    }
}
