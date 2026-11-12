# A mouse press that leaves a swipe node inside the slop and lifts outside it leaves the swipe armed and its contact pending

**Status:** Closed
**Resolution:** Pending swipes and their deferred ancestors end on release or cancellation anywhere in the document; buttonless mouse moves cancel before capturing or beginning a hold.
**Systems:** host/web/motion-glue.js, host/web/input-glue.js (both web targets load them)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`attachSwipe` (`host/web/motion-glue.js:908`, at `dd6f5dcd`) takes no pointer capture at the down. It records
the contact, marks the pointer `pending` in `exact.contacts` (`:935-939`), and hears the contact's moves and
its end only on the node (`:973`, `:988`). A mouse is not captured implicitly. A press that leaves the node
before a move inside it passes the slop (4 px, `motion/src/gesture.rs:19`), and lifts elsewhere, is never
ended: a press within the slop of the edge it leaves by, or one quick move out. `drag` stays set and the
pointer stays `pending`.

`move` (`:940`) does not read `e.buttons`, and a mouse keeps one `pointerId`, so the stale record matches every
later mouse event:

1. Every recognizer that defers to a pending swipe waits on the stale mark, anywhere on the page: a pan
   (`host/web/input-glue.js:1150-1156`, `:1201`), a height drag and a transform drag (`motion-glue.js:120-128`).
2. When the mouse is next over the node, with no button down, `move` measures from the old press point, claims
   the swipe (`:951`) and the node follows the hovering mouse. A left click on the node cancels the hold; a
   right click's release reaches `finish`, which fires the swipe action past the knee (`:983-984`).

A touch is captured implicitly by its target and is not affected.

## The failure

A 300 by 44 px node at (100, 100) that hears `swiperight` and `swipeleft`, and a `pan` node below it. Chrome's
mouse, with the engine's slop, knee and resistance (4, 64, 0.2):

```
press 250,141   move to 250,171 in 2 px steps (button down)   release 250,170
```

Observed: the `pointerup` targets `<html>` and `exact.contacts.get(1)` is `"pending"`. A press at 250,122 that
leaves in one 24 px move does the same; one that leaves in 4 px steps leaves no contact. Then:

- A pan dragged 80 px with the button down dispatches no delta until the release, then `"80,0"` once. Without
  the stale press the same drag dispatches 19 deltas.
- Hovering 100 px right along the row with no button: the engine is asked `begin` and a `move` per step, and
  the row's computed `translate` is `69.6px`. A right click there asks the engine `action`.

Expected: the lift ends the contact wherever it happens, and no hold begins without a button down.
`input-glue.js` ends its own press and pan this way (`:918`, `:1173`).

## Verification

Run in the pinned headless Chromium (156) with CDP's `Input.dispatchMouseEvent`, against the real
`motionController` and `createInputHandlers` with a stand-in engine. It printed the results above. Not run: a
compiled app, Firefox, WebKit, a pen, a Control-click on a Mac.

## Direction

End a pending swipe when its pointer lifts or is cancelled anywhere (the document's capture phase, as the press
listens), and in `move` treat a mouse move with `buttons === 0` as the end. Both delete the `pending` mark.

## Related

None in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`. `issues/20261022-no-swipeleft.md` is about the
missing direction.
