# A wheel event between mouse down and up cancels a press that has begun no gesture, so the click is lost

**Status:** Closed
**Resolution:** A wheel cancels recognized UI gestures and scrollbar drags; a pending mouse press clicks if its button remains under the pointer.
**Systems:** host/linux/src/presenter/wheel.rs (the painting host with a mouse: Linux, Windows)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Presenter::wheel_at` cancels the pointer contact before it knows what the contact is or
whether anything scrolls (`host/linux/src/presenter/wheel.rs:129` to `:135`):

```rust
// A UI wheel can take over a UI gesture, but does not release a game's
// captured pointer. Browser wheel events do not end pointer capture.
if self.contact_canvas().is_none() {
    if let Err(error) = self.pointer_cancel(self.pointer_now()) {
```

The takeover is meant for a gesture. LLP 1015.002 item 3 keeps "the existing wheel takeover
for UI gestures"; LLP 1057.001 §6 lists it among the cancels of "a contact cancelled after
it began" and says "A contact that never began releases nothing: it is a tap, and rule 4's
`press` on the same node fires instead"; the test that holds it,
`display_cancel_and_wheel_win_without_dispatching_reply`
(`host/linux/src/presenter/swipe_tests.rs:119`), wheels during a recognized swipe.

A press that has not moved is no gesture, but the cancel takes it too, and the release that
follows presses nothing. A browser delivers that click, so the web host and the painted
hosts split on this input. A mouse sends wheel events while a button is held more often
than it seems: a free-spinning wheel keeps turning after a flick, and a click lands inside
that run.

## The failure

```
component App
  state n = 0
  action go
    n = n + 1
  view
    column width=400
      button press=go testId="b" width=200 height=200
        text "Go"
```

Mouse down at (100, 150), one wheel there, mouse up there. For the last two rows a
`box height=2000 flex-shrink=0` follows the button, so the page scrolls.

| the wheel | painted presenter | Chromium 156, the same boxes |
| --- | --- | --- |
| none | `n` = 1 | 1 click |
| 40 down, the page cannot scroll | `n` = 0 | 1 click |
| 40 up, at the top of a page that scrolls | `n` = 0 | 1 click |
| 40 down, the page scrolls 40 and the button stays under the pointer | `n` = 0 | 1 click |

Expected: `n` = 1 in every row. Chromium drops the click only when the wheel carries the
button out from under the pointer (a wheel of 300: 0 clicks).

## Verification

A temporary test in `exact-linux`'s library ran `pointer_down`, `wheel_at(x, y, 0, ±40)`
and `pointer_up` for each row and printed the page's offset and the slot `n`. The Chromium
column is bare HTML with a counting `onclick`, driven by `mouse.down`, `mouse.wheel` and
`mouse.up`; a `wheel` listener counted one event in each wheel row. Not run: Windows, or a
real mouse on a Linux display.

## Direction

Cancel the contact only when it holds a gesture that has begun (a pan, a swipe, a drag). A
press that has begun none keeps its release, and the release presses only if the pressed
node is still under the pointer.

## Related

`issues/20261110-horizontal-wheel-scrolls-vertically.md` and
`issues/20261110-evdev-wheel-detent-scrolls-twice.md` are about the wheel's delta.
`issues/20261110-swipe-press-lifted-outside-stays-pending.md` is the swipe's release.
Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` names a wheel during a mouse press.
