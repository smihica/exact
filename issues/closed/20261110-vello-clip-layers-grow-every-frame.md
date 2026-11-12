# On the GPU replayer a canvas that clips each frame without `save` gains a clip layer a frame, about 1 MiB a frame on a phone's full screen

**Status:** Closed
**Resolution:** The vello replayer skips a clip whose path is already in force and folds a rectangle into the rectangle its save level opened last, so a draw that clips every frame without `save` keeps one clip layer.
**Systems:** canvas/vello/src/replay.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A canvas's clip persists between draws, as on the web (LLP 1056 D3). HTML's `clip()`
replaces the clipping region with its intersection with the path, and only `restore`
or `reset` widens it. So a draw may call `clip()` every frame with no `save` around
it, and its region stays the same rectangle.

The vello replayer keeps that region as a list of every clip path in force (`open`,
`canvas/vello/src/replay.rs:130-132`). `Op::Clip` pushes one (`replay.rs:404-415`);
only `restore` and `reset` pop (`replay.rs:265-279`). Each replay pushes every open
clip as a vello layer (`begin`, `replay.rs:190-194`) and pops them at the end (`end`,
`replay.rs:197-201`). Frame N is therefore rendered under N nested clip layers, and
the module's GPU buffers grow with them.

The picture stays right; the cost grows without bound. The canvases that reach this
replayer are the ones whose draw asks for the next frame (LLP 1056 §8.5), so the list
grows 60 or 120 times a second. Core Graphics intersects the clip in its graphics
state, so the same canvas costs the same every frame while it is still.

## The failure

```js
export function draw(surface, args, ctx, frame) {
  ctx.clearRect(0, 0, frame.width, frame.height);
  ctx.beginPath(); ctx.rect(10, 10, 100, 50); ctx.clip();   // no save or restore
  ctx.fillRect(20 + (frame.time / 16) % 60, 20, 30, 30);
  return true;
}
```

Observed, the same calls replayed through the module's ABI (`ecg_canvas_replay`,
`ecg_canvas_stats`, `ecg_memory`) on an M5 Ultra:

| Canvas, clip | Frame | Replay | Module memory |
|---|---|---|---|
| 200 × 100 px, clip 100 × 50 | 60 | 2.1 ms | 2.2 MiB |
| | 600 | 3.8 ms | 18 MiB |
| | 3,600 | 13.4 ms | 80 MiB |
| | 7,200 | 24.9 ms | 160 MiB |
| 1179 × 2556 px (393 × 852 at 3×), clip 373 × 832 | 60 | 3.5 ms | 96 MiB |
| | 600 | 10.0 ms | 485 MiB |
| | 1,800 | 17.9 ms | 1,875 MiB |

With `save` and `restore` around the clip the small canvas takes about 2.5 ms and
0.6 MiB at every frame.

Expected: a cost that stays near the first frame's. Chrome's grows too, far more
slowly: the same draw in headless Chrome 156, each frame flushed by a one-pixel
`getImageData`, took 0.005 ms a frame at 100 frames, 0.33 ms at 10,000 and 1.75 ms at
50,000.

## Verification

Run: temporary tests in `canvas/vello/tests`. One recorded the draw with one
`Context2d` and replayed each frame's lists into two IOSurfaces through the ABI,
printing the table (a debug build; at frame 7,200 the GPU's share is 23.4 of the
24.9 ms). The square was painted correctly at every frame printed. Another replayed
through `Replayer::apply` with no GPU and printed `scene.encoding().n_clips`: 4 at
frame 1, 122 at frame 60, 1,202 at frame 600. No iPhone or iPad was measured.

## Direction

A clip equal to one already open at the same save level leaves the region unchanged:
do not push it again. That bounds the common case. Clips that differ every frame
still accumulate; fold rectangles into their intersection, and the rest into one mask
once their count passes a small bound.

## Related

`QUEUE.md`, "Canvas 2D on the GPU owes", lists memory (the second IOSurface), the
main thread, the bundle and text; not this. LLP 1056 D4 drops the clip-depth limit
because no browser has one, which is about depth the author asks for.
