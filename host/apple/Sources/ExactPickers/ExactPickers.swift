// The pickers family (LLP 1047.001 D10, the roster's `picker`): `showPicker`'s
// file picker (LLP 1069.002), and `saveFile` and the File System Access
// pickers (LLP 1069.010), which one object presents. An app's composition
// links this module when its linked set names `picker`: a file input,
// `showPicker`, a document-picker command, or a manifest that links
// `documents` or `media_picker` ahead. It installs it before any session is
// made; the core reaches it only through `PickerLink`.
import ExactKit

public enum ExactPickers {
    /// Set `PickerLink`'s slot, before any session is made.
    public static func install() {
        PickerLink.make = { Picker(session: $0) }
    }
}
