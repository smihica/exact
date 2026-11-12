# On Android 12 and later a `backdrop-filter` box painted after a scroller records the scroller's rows again, displaced by the box's position

**Status:** Closed
**Resolution:** Android backdrop replay reuses recorded scroller groups and applies the backdrop offset only when drawing them.
**Systems:** host/android/canvas/src/main/java/dev/exact/canvas/Player.kt
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`. `Player.backdrop` (op 42, API 31 and later) draws what is behind a filtered box by replaying the stream from its start up to the op into a node of its own, placed at the blur's bounds (l, t). It tells the nested replay where that node is through `shiftX` and `shiftY` (`Player.kt:162` to `:167`):

```kotlin
shiftX = sx + l; shiftY = sy + t
val inner = node.beginRecording()
replay(inner, b, from, at)
```

A scroller's rows are not drawn into the canvas being replayed. Op 20 takes the scroller's one node (`ExactView.kt:104`) and records the rows into it, each moved by the shift (`Player.kt:887` to `:893`, `:862`):

```kotlin
val g = group!!(b.get(i + 1)); … c = g.beginRecording()
matrix.setTranslate(f(b, i + 2) - shiftX, f(b, i + 3) - shiftY)   // ROW_DRAW
```

So the nested replay records the scroller's node a second time, with every row moved by (-l, -t). The view's own drawing, recorded a moment earlier with no shift, draws that same node, and a display list holds a child node, not a copy of its drawing (AOSP `SkiaRecordingCanvas::drawRenderNode`; `endRecording` replaces the node's list, `RenderNode::setStagingDisplayList`). The page therefore draws the moved rows. Inside the blur the node is drawn under the shift a second time (`:902`), so the rows are displaced there too.

**The failure.** A scroller with a filtered box over its foot:

```
view width=200 height=200 position="relative"
  scroll width=200 height=200
    column
      view height=80 background-color="#ff0000"
      view height=80 background-color="#00ff00"
      view height=80 background-color="#0000ff"
  view position="absolute" left=0 right=0 bottom=0 height=40 backdrop-filter="blur(8px)" background-color="rgba(255,255,255,0.5)"
```

At one pixel a point the box is (0, 160, 200, 40) and the blur's pad is 3 × 8, so l = -24 and t = 136.

- observed (by reading): the rows are drawn 24 px to the right and 136 px up. The foot of the green row fills the top 24 px, the blue row follows, nothing is drawn below it, and the box blurs nothing;
- expected: red, green and the top of blue, with blue blurred under the box.

The Agents app's chat has this shape: a list (`apps/agents/app.contract:247`) under a composer with `blur(24px)` (`:299`). There the transcript is drawn 72 dp to the right and up by the composer's top less 72 dp, most of a screen.

It happens whenever the view's drawing is recorded: at the first frame, and at every stream the reader does not retain in place. A stream with a hosted text field in it is never retained (`Structure.kt:105`), and the composer has one.

**Verification.** Run: a temporary test in `host/linux/src/canvas/stream.rs` printed the Canvas recorder's stream for the page above: `MATRIX`, `CLIP_RRECT`, `GROUP_BEGIN`, three `ROW_BEGIN` and `ROW_DRAW`, `GROUP_END`, `RESTORE`, `MATRIX`, `BACKDROP` (blur 8 over 0, 160, 200, 40), `RRECT`. The group precedes the backdrop in one replay. A page shaped like Agents' chat and `scripts/fixtures/backdrop-nested.contract` give the same order. Read, not run: `Player.kt`, `ExactView.kt` and AOSP `android14-release`; no Android device was available. The device check: this page on Android 12 or later, beside the same page without `backdrop-filter`.

**Direction.** In a replay with a shift, skip a group's body to its `GROUP_END` and draw its node (or holder) under the shift: the outer replay has already recorded it.

**Related.** Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names this. `host/android/README.md` says only that there is no blur below Android 12.
