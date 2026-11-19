// MIDI ports and the MIDI queue's output (LLP 1047.001 D10, the roster's
// `midi`): the MIDI arm's Swift half (LLP 1125 D6–D8). An app's composition
// links this module when its plan asks for ports, hears an input or plays a
// sequence, and installs it before any session is made; the core reaches it
// only through `MediaLink`.
import ExactKit

public enum ExactMIDI {
    public static func install() {
        MediaLink.makeMIDI = { MIDIOutput($0) }
    }
}
