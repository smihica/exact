# A dash list of zeros strokes nothing on the vello and Linux replayers, a solid line everywhere else

**Status:** Closed
**Resolution:** All-zero dash lists produce solid strokes in the vello and Linux Canvas 2D replayers.
**Systems:** canvas/vello/src/replay.rs, host/linux/src/canvas2d/mod.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`setLineDash([0, 0])` is a legal dash list (only negative and non-finite entries are
refused), and the recorder passes it on as `LineDash 0 0`. Two replayers then return
no stroke at all for it:

- `canvas/vello/src/replay.rs:598-613`, `stroke_style`: "none when the dash list is
  all zeros (nothing is drawn)".
- `host/linux/src/canvas2d/mod.rs:198-201`, the same branch.

The other three draw a solid line: Chrome, the web host (which hands the list to the
browser, `host/web/canvas2d-glue.js:136`), and the Core Graphics replayer
(`host/apple/Sources/ExactSurfaces/Canvas2DPaint.swift:22` sets the list, and Core
Graphics strokes solid under it). The vello replayer says it has "the
reference replayer's semantics" (`replay.rs:1-2`), and the reference is the Core
Graphics one. So on Apple the same canvas strokes a solid line while it is still
(Core Graphics) and nothing once its draw asks for frames (vello, LLP 1056 §8.5), and
Linux differs from the web.

## The failure

```js
ctx.lineWidth = 4;
ctx.setLineDash([0, 0]);     // also setLineDash([0]), which doubles
ctx.beginPath(); ctx.moveTo(10, 50); ctx.lineTo(190, 50); ctx.stroke();
```

Expected: a solid line, 720 pixels on a 200 by 100 canvas, as Chrome paints it.
Observed: no pixel on the vello and Linux replayers.

## Verification

Run, pixels with ink for the stroke above:

| | no dash | `[0, 0]` | `[4, 4]` |
|---|---|---|---|
| Headless Chrome 156 | 720 | 720 | |
| Core Graphics (`CGContext.setLineDash`, a Swift script) | 720 | 720 | 368 |
| Linux replayer (a temporary test in `host/linux/src/canvas2d/tests.rs`) | 720 | 0 | 368 |

The Linux replayer also painted nothing for `[0]` and for `strokeRect` under
`[0, 0]`; Chrome painted both solid. The vello replayer, in a temporary test in
`canvas/vello/tests` with no GPU: `stroke_style()` is `Some` and the scene has one
path with no dash list, `None` and no path with `[0, 0]`.

## Direction

In both `stroke_style` functions, treat an all-zero list as no dash list instead of
returning `None`. A draw-in that scales its dash by a factor starting at zero then
begins solid, as it does in the browser.

## Related

`QUEUE.md`, "Canvas 2D on the GPU owes" and "LLP 1056 (Canvas 2D) stage 1 owes", list
parity gaps; neither names dashes. `canvas/tests/cases.txt` covers an odd list and a
refused one, not zeros.
