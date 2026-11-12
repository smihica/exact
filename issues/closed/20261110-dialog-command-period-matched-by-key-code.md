# macOS: a dialog reads Command-period by key code, so ⌘V on a Dvorak layout dismisses it

**Status:** Closed
**Resolution:** macOS dialogs and modal routes match Command-period by its character, so keyboard layouts can move the shortcut without turning other commands into dismissal.
**Systems:** host/apple/Sources/ExactKit/Mac/DialogsMac.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`DialogHost.key` (`host/apple/Sources/ExactKit/Mac/DialogsMac.swift:336-342`) gives an open dialog AppKit's Command-period, a sheet's Cancel, by the event's key code:

```swift
let period = event.type == .keyDown && event.keyCode == 47
    && event.modifierFlags.intersection([.command, .control, .option, .shift]) == .command
if event.keyCode == 53 || period {
    if event.type == .keyDown, !event.isARepeat, dismissalOwner(of: dialog).props["closedby"] != "none" { dismiss(dialog) }
    return true
}
```

Key code 47 is a physical key (`kVK_ANSI_Period`), and the keyboard layout decides what it types. `Presenter.routeKey` (`KeyEvents.swift:219-236`) calls this from the session's event monitor (`ExactViewMac.swift:389`), which runs before the menu bar's key equivalents and drops an event the route took.

An app's own `aria-keyshortcuts="Meta+."` is matched by character (`Shortcut.matches`, `ShortcutsMac.swift:50-73`). The two rules disagree on every layout that does not type `.` at that key. The shortcut matcher also reads `charactersIgnoringModifiers`, which loses Command's layout change under Dvorak - QWERTY ⌘.

## The failure

With the Dvorak layout, open a `dialog` that holds an `input` (and is not `closedby="none"`) and press ⌘V in the field. Key code 47 types `v` there, so the dialog closes, its `cancel` handler runs and nothing is pasted; a modal route's sheet goes back the same way (`DialogsMac.swift:383-392`). Expected: Paste. Dvorak's own ⌘. is key code 14, which the dialog does not hear. On French AZERTY key code 47 types `:`, and on Turkish Q it types `ç` and `.` is key code 44.

## Verification

A temporary XCTest opened a dialog in a booted session, put the caret in its field and gave `Presenter.routeKey` a Command key-down with each key code and character. The result was the same in AppKit's sheet and painted in the page:

```
ANSI Command-V (key code 9, "v"): taken false; dialog open after true
ANSI Command-period (key code 47, "."): taken true; dialog open after false
Dvorak Command-V (key code 47, "v"): taken true; dialog open after false
Dvorak Command-period (key code 14, "."): taken false; dialog open after true
French Command-colon (key code 47, ":"): taken true; dialog open after false
Turkish Q Command-period (key code 44, "."): taken false; dialog open after true
```

What key code 47 types, read from each installed layout's own data with `UCKeyTranslate`:

```
US "."   Dvorak "v"   Dvorak - QWERTY ⌘ "v" (with Command ".")   French ":"   Turkish Q "ç"
```

Not run: a person's key under a Dvorak input source, which would change this Mac's.

## Direction

Match Command alone and `.` in `characters`, preserving Command's effect on the layout in both dialog dismissal and `Shortcut.matches`. Dvorak - QWERTY ⌘ produces `characters == "."` and `charactersIgnoringModifiers == "v"` at key code 47; its ⌘E at key code 14 produces `"e"` and `"."`. Escape (53) and Tab (48) are the same key on every layout and can stay.

## Related

No issue, QUEUE entry or pitfall names Command-period or a keyboard layout. The Mac presenter's other key code tests are Escape (53) and Tab (48), which no layout moves.
