# Apple Canvas 2D keeps every font a list ever set, and a recorded canvas copies them all each frame

**Status:** Closed
**Resolution:** Canvas fonts live with replay state and kept lists, and the measurement cache holds at most 256 entries.
**Systems:** host/apple/Sources/ExactSurfaces/Canvas2DImage.swift, host/apple/Sources/ExactSurfaces/Canvas2DText.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd` two tables of the canvas host hold one `CTFont` for each distinct font a Canvas 2D list has set, and neither lets one go while the session lives:

- `Canvas2DHost.resolved` (`host/apple/Sources/ExactSurfaces/Canvas2DImage.swift:140`), written at `:215`. `forget` (`:391-408`) does not touch it, so the fonts outlive their canvas.
- `CanvasText.fonts` (`host/apple/Sources/ExactSurfaces/Canvas2DText.swift:22`, `:57`).

`resolved`'s key is the whole `Canvas2DFont`, its size, `letterSpacing` and `wordSpacing` included (`Canvas2D.swift:61-65`); `CanvasText.fonts`' is the face and the size (`Canvas2DText.swift:33`). The recorder sends the size unrounded (`canvas/src/context/text.rs:57-76`), so a draw that animates it sets a new font each frame.

Each job is handed all of `resolved` (`let fonts = resolved`, `:217`), since a replayer may still draw with a font an earlier list set. A recorded canvas's layer keeps that snapshot until the next frame shows (`Canvas2DRecord.swift:188-195`), so the next `apply`, which writes `resolved[f]` for each font its lists set, copies the whole dictionary on the main thread.

## The failure

```ts
export function draw(surface, args, ctx, frame) {
  ctx.fillStyle = '#fff'; ctx.fillRect(0, 0, frame.width, frame.height);
  ctx.font = `${24 + 6 * Math.sin(frame.time / 300)}px system-ui`;
  ctx.fillStyle = '#000'; ctx.fillText('Score', 10, 50);
  return true;
}
```

The footprint after a minute of it on macOS (3,600 frames), by what draws the canvas. An animating canvas is the GPU module's when the bundle has the module (`Canvas2DImage.swift:278-284`), else it is recorded:

| drawn by | one size | pulsing | pulsing, `TextEngine.fonts` emptied each second | pulsing, every font table emptied each second |
| --- | --- | --- | --- | --- |
| the GPU module | +0.7 MB | +185.5 MB | | +49.7 MB |
| a recording, never displayed | +0.7 MB | +30.9 MB | +30.5 MB | +17.3 MB |
| a recording, the draw also measuring the run | +0.8 MB | +192.0 MB | +191.4 MB | +52.6 MB |
| its bitmap (the op not animating) | +0.8 MB | +236.2 MB | +235.3 MB | +104.6 MB |

Both tables hold 3,600 fonts after the minute and after `forget` of the canvas. Emptying `TextEngine.fonts` alone frees nothing: these tables hold the same fonts. What still grows with every table emptied is outside them.

On the recorded canvas, `apply()` for a frame that sets a size already held takes 0.144 ms with 3,600 fonts held, against 0.029 ms with one; with 36,000 held a frame costs 1.08 ms. The copy is absent where the GPU module draws (0.023 ms against 0.022 ms) or the op is not animating (0.029 ms).

Expected: memory and the per-frame cost stay flat, as with one size.

## Verification

Temporary Mac XCTests (`bun host/apple/build.mjs --test --only <Classes>`), each case in a process of its own. Each frame handed `Canvas2DHost.apply` the list such a draw records (the cover, a `font`, a `fillText`) inside an autorelease pool; the measuring row also called `CanvasText.measure`. The counts are read through `Mirror`, the footprint from `task_info`. The GPU row ran with `EXACT_CANVAS_GPU_DYLIB` naming the built module; the recorded rows' layer was in no window, so nothing drew the recording. The last column replaced the `Canvas2DHost` and the engine's `canvasTextCache` and emptied `TextEngine.fonts` once a second; doing that with one size adds 0.5 MB. The draw was not run through a runner, and nothing ran on iOS, where the same files compile.

## Direction

Keep the resolved `CTFont` in the replayer's state, set at the `font` record and saved and restored with it, so a job carries only the fonts its own lists set and `resolved` needs no history. Bound `CanvasText.fonts`, which is a cache.

## Related

`issues/20261110-apple-font-cache-keeps-every-size.md` is `TextEngine.fonts`; the table's third column shows that bounding it alone leaves this. `QUEUE.md`, "LLP 1056 (Canvas 2D) stage 2 owes" (3), records that Apple's canvas image cache is never evicted; it does not name fonts. `issues/20261110-gpu-canvas-brush-cache-never-releases.md` is the Linux GPU painter's canvas snapshots.
