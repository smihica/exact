# On touch, a height or transform drag that waited for an inner swipe cancels itself when it takes the capture

**Status:** Closed
**Resolution:** Height and transform drags ignore a child's capture loss and retain the touch until their own release or cancellation.
**Systems:** host/web/motion-glue.js (both web targets load it)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A touch is captured implicitly by the element it lands on. When a recognizer on an ancestor later calls
`setPointerCapture`, the browser fires `lostpointercapture` at that first element, and the event bubbles to the
ancestor. Four recognizers hear `lostpointercapture` on their own node. Three ignore one whose target is another
node (lines at `dd6f5dcd`):

- the swipe: `host/web/motion-glue.js:976-977`
- the pan: `host/web/input-glue.js:1186-1187`
- the reorder grip: `host/web/motion-glue.js:1143-1146`

The height drag and the transform drag do not. Their `finish` runs for every `lostpointercapture` that reaches
the handle and stops the drag (`motion-glue.js:888-890` with `:905`; `:789-791` with `:802`).

Both take the capture in `pointerdown` unless a swipe further in is still deciding (`deferred`, `:852-855`,
`:766-769`). In `pointerdown` the landing element has not yet received its capture, so nothing is lost. In the
deferred case they take it in a later move (`deferred()`, `:120-128`), after the landing element has it. Its loss
then bubbles to the handle and cancels the drag that was just recognized.

LLP 1057.001 §1 (rule 3 and the web's row of the mechanism table) has an ancestor's pan, transform or height
drag wait for the swipe's verdict and then take the contact.

## The failure

A handle whose child hears `swiperight`. A finger lands on the child and drags vertically 100 px in 20 px
steps, so the swipe refuses and the handle's drag begins.

| Handle | Child hears `swiperight` | Engine requests |
|---|---|---|
| height drag | no | `height-begin`, a `move` per step, `height-action` at the lift |
| height drag | yes | `height-begin`, `move`, `cancel`; nothing at the lift |
| transform drag | no | `transform-begin`, a `transform-move` per step, `transform-action` at the lift |
| transform drag | yes | `transform-begin`, `cancel`, `cancel`; nothing at the lift |

In the two failing rows the handle hears `lostpointercapture` from the child and then its own, and the target
stays where it was (height 300 px; `translate: none`) while the finger moves on.

Expected: the rows with the swipe node match the rows without it. A mouse has no implicit capture: the same
drags by mouse follow the pointer and send their action with or without the swipe node.

## Verification

Run in the pinned headless Chromium (156) against the real `motionController` (`attachSwipe`,
`attachHeightDrag`, `attachTransformDrag`) with a stand-in engine that accepts every request, touches from CDP's
`Input.dispatchTouchEvent` and the mouse from `Input.dispatchMouseEvent`. It printed the table above and, for the
mouse, the passing sequences. Not run: a compiled app, Firefox, WebKit. No app in this repository puts a swipe
node inside a height or transform handle.

## Direction

In both `finish` functions, ignore a `lostpointercapture` whose target is not the handle, as the swipe does.

## Related

None in `issues/` or `QUEUE.md`. `QUEUE.md`, "The web's touch press under a person's scroll, not yet seen in
Firefox for Android", is about press feedback, not capture.
