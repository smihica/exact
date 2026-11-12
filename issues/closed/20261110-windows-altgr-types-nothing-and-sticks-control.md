# On Windows a character typed with AltGr is dropped and Control stays held afterwards

**Status:** Closed
**Resolution:** Windows uses winit's filtered modifiers, so AltGr enters text without leaving Control held.
**Systems:** host/windows/src/keyboard.rs, host/windows/src/window.rs, host/linux/src/surface_controls.rs, host/linux/src/presenter/field.rs
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`. On a layout whose right Alt is AltGr, Windows presses left Control with it: "the right-hand ALT key is converted internally into CTRL+ALT" ([VkKeyScanW](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-vkkeyscanw)). The Control arrives as its own key message before the Alt one, and [GetKeyState](https://learn.microsoft.com/windows/win32/api/winuser/nf-winuser-getkeystate)`(VK_LCONTROL)` reads down until AltGr is released. winit 0.30.13 hides this from an app: it drops the Control key event (`src/platform_impl/windows/keyboard.rs:711`), leaves Control and Alt out of `ModifiersChanged` while `VK_RMENU` is down (`keyboard_layout.rs:275`), and names the key by its AltGr character (`@`).

`sync` (`host/windows/src/keyboard.rs:6`) undoes that. It runs on every `ModifiersChanged` (`host/windows/src/window.rs:1160`), ignores the state winit sent and reads `GetKeyState(VK_LCONTROL)` itself, so it tells the presenter `ControlLeft` is held. winit sends `ModifiersChanged` at both of AltGr's key messages (`event_loop.rs:1128`), and Control reads down at both. The presenter treats a key pressed with Control held as a chord (`host/linux/src/surface_controls.rs:400`), and a field types nothing in a chord (`host/linux/src/presenter/field.rs:1020`, `:1071`).

When AltGr goes up, winit drops the Control release and sends no `ModifiersChanged`, because its own state is already empty. `ControlLeft` stays held until another modifier is pressed or the window regains focus, and each plain key until then is a Control chord.

No layout is special, so every AltGr character fails: German `@ € { [ ] } \ ~ |`, French `@ # { [ ] } \ €`, Spanish `@ # [ ] { }`, Polish `ą ć ę ł ń ó ś ź ż`.

**The failure.** German layout, a text field holding `name`: type AltGr+Q, then `example.com`.

- observed: the field still holds `name`, now selected, and the clipboard holds `name` (`a` selected all, `c` copied);
- expected: `name@example.com`, as in Notepad and Chrome.

**Verification.** The presenter's half was run on macOS: a temporary test in `host/linux/src/presenter/field_tests.rs` made the calls `sync` and `window.rs:1306` make, with a clipboard set as the Windows host sets one.

```rust
p.hold_modifier("ControlLeft", true);
p.hold_modifier("AltRight", true);
p.hardware_key("AltRight", "AltGraph", true, false);
p.hardware_key("KeyQ", "@", true, false);
p.hardware_key("KeyQ", "@", false, false);
p.hardware_key("AltRight", "AltGraph", false, false);
// then e, x, a, m, p, l, e, ., c, o, m, each down and up
```

It printed the field as `name` with the selection `0-4`, the clipboard as `name`, and `modifiers().ctrl == true` after AltGr's release. Without the first line, AltGr+Q then `x` typed `@x`. The Windows half is read from winit's source and Microsoft's documentation; no Windows machine was available. The check to make there: German layout, Shelf's search field, AltGr+Q then `x`.

**Direction.** Take the held modifiers from the state `ModifiersChanged` carries, and use `GetKeyState` only to tell left from right for a modifier that state names.

**Related.** No issue, QUEUE entry or pitfall names AltGr. A dead key's lost text is a separate defect: there the modifiers are right and the text is dropped.
