# On Linux and Windows a real Escape key takes the focus from a text field or a button, and `preventDefault()` does not stop it

**Status:** Closed
**Resolution:** Escape leaves the focus where it is unless a canvas in the app takes keys, which the blur exists to hand them back to; there a `preventDefault()` still does not stop it.
**Systems:** host/linux/src/surface_controls.rs (the painted presenter: the Linux display and VNC, the Windows window)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Presenter::hardware_key`, which a device's keys reach, ends every Escape key-down that
no open surface took by dropping the focus when it is on a text field, or on any node
outside a canvas that takes input (`host/linux/src/surface_controls.rs:506-517`):

```rust
if code == "Escape"
    && !surface_was_open
    && self.focus.is_some_and(|id| {
        self.host.kernel().node(id).is_some_and(|n| n.node_type == NodeType::TextInput)
            || self.input_surface(id).is_none()
    })
{
    self.blur();
}
```

The blur is not the key's default action. It runs after the `key` handlers whatever
they did, so `preventDefault()` does not stop it. The agent's `type … key Escape`
calls `type_key` (`host/linux/src/agent.rs:557-577`) and never reaches it, so a drive
shows the field keeping the focus.

No LLP rules the blur. LLP 1015: "A key down, from the display or the agent's
`type … key`, is heard first by the nearest `key` handler at or above the focus, by the
web's name, then does its default", and the defaults it lists do not include leaving
the field.
`surface_controls_tests.rs:778-779` pins it for an editor beside canvases that take
keys, where it hands the keys back to the game, but the condition applies it to every
app. A browser leaves the focus where it is, and the same symptom on the Mac was a
defect (`issues/closed/20261016-escape-in-recorder-blurs-on-macos.md`: "blurs it, where
the web keeps it").

## The failure

```
component App
  state note = ""
  state heard = ""
  action typed(value: string)
    note = value
  action key(e: string)
    heard = e
  action left
    heard = `${heard} then blur`
  view
    column width=400 height=400
      input testId="field" value=note input=typed key=key blur=left height=30
      button "Save" testId="save" height=40
      text heard testId="heard"
```

Focus the field, press Escape, type `a`:

| | painted host, a device's key | Chrome 156 |
|---|---|---|
| focus after Escape, from the field | none | the field |
| focus after Escape, from the button | none | the button |
| the field's value after `a` | `""` | `"a"` |

The field also hears a `blur` the web does not send, so a field that commits or closes
on `blur` does so on Escape.

## Verification

A temporary test in `host/linux/src/presenter/arrive_tests.rs` drove
`hardware_key("Escape", …)` over this source (`cargo test -p exact-linux --lib`) and
printed `focus on field; after Escape: nothing`, `focus on save; after Escape: nothing`,
`Escape then a in the field: value = Some("")` and
`the field's handlers heard: Some("Escape then blur")`. A second, whose `key` handler
calls `preventDefault()` on Escape, printed:

```
handler prevents the default only; agent key: focus on field; the field heard blur Some("0") times
handler prevents the default only; device key: focus on nothing; the field heard blur Some("1") times
```

Chrome: the same page in HTML, driven with the repository's `playwright-core` and
Chromium (156.0.8078.4), kept the focus and gave `"a"`. Not run on Windows or with a
real keyboard. Android does not send Escape to this path
(`host/android/canvas/src/main/java/dev/exact/canvas/Keys.kt:55-79`).

## Direction

Keep the blur only where a canvas takes keys, or remove it. If it stays anywhere, make
it the key's default action, so `preventDefault()` stops it and the agent's key meets
it too. The test at `surface_controls_tests.rs:778-779` changes with it.

## Related

`issues/closed/20261016-escape-in-recorder-blurs-on-macos.md` fixed the Mac. QUEUE's
"Establish native keyboard behavior after system-menu Escape" is about the Windows
system menu.
