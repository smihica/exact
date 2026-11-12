# The CPU painter's kept clip mask ignores `corner-shape`, so a changed corner shape clips with the old outline

**Status:** Closed
**Resolution:** The CPU clip cache includes each corner's shape and rebuilds the mask when it changes.
**Systems:** host/linux/src/raster.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Raster` keeps one clip mask across frames, for the first clip a frame pushes that is
not the viewport, and reuses it when the next frame's first clip has the same key
(`host/linux/src/raster.rs:1035` to `1047`). The key, `ClipKey` (`raster.rs:46`,
built at `:192`), holds the frame's size and scale, the transform, and the shape's
rectangle and radii. It does not hold `Shape::corners`, the CSS `corner-shape` the
mask's path is built from (`raster/geometry.rs:13`, `border::shape_path`). Two shapes
that differ only in `corner-shape` have equal keys, and the second is clipped by the
first one's mask.

## The failure

```
component App
  state bevelled = false
  action cut
    bevelled = true
  view
    box width=100 height=100 testId="card" press=cut overflow="hidden" border-radius=40 corner-shape=(bevelled ? "bevel" : "round")
      box width=100 height=100 background-color="#ff0000"
```

Pixel (14, 14) is inside the round corner and outside the bevel, whose edge is the
line x + y = 40. Painted, pressed, painted again, in a 300 by 300 viewport at scale 1:

```
CPU   round (255, 0, 0)   after the press (255, 0, 0)       booted bevelled (255, 255, 255)
GPU   round (255, 0, 0)   after the press (255, 255, 255)   booted bevelled (255, 255, 255)
Ink   round (255, 0, 0)   after the press (255, 255, 255)   booted bevelled (255, 255, 255)
```

On the CPU painter the box keeps its round outline after the state changes, until its
size, radius, position or the frame's size changes.

A person sees it only where the CPU painter draws the window (no GPU adapter on Linux,
`EXACT_PAINTER=cpu`, a Windows game), the frame is at most 1 MiB of device pixels
(`CLIP_CACHE_BYTES`: 390 by 844 at scale 1 is, the same at scale 2 is not), and the
box whose `corner-shape` changes is the frame's first clip. The headless agent's
screenshots and the pinned pixel tests use this painter too.

## Verification

A temporary unit test in `host/linux/src/paint/` booted the Contract above with each
of `PainterChoice::Cpu`, `Gpu` and `Ink`, read the pixel, pressed the box with
`Presenter::tap`, and read it again; the table is its output. A second test drove
`Raster` alone (`begin`, `push_clip`, a red `fill`, `pop_clip`, `finish`) with a round
shape and then `with_corners(CornerShape::parse("bevel"))` and printed the same stale
pixel.

## Direction

Put the corner shape in `ClipKey` (the four corners' kind and parameter), or return
`None` from `clip_key` for a shape whose `corners` is set.

## Related

`issues/20261110-vello-clip-layers-grow-every-frame.md` is about a 2D canvas's clips
on the GPU replayer, not this cache. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md`
names the CPU clip cache.
