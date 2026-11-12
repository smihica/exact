# Each time the camera crosses a water surface with shafts, the frame's attachments and bloom chain are recreated

**Status:** Closed
**Resolution:** Once water shafts draw, depth stays retained and surface crossings reuse the frame attachments and bloom chain.
**Systems:** game/render/src/renderer/draw.rs, game/render/src/water.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. `prepare_targets` replaces the HDR attachments whenever
`retained` (the forward depth is kept for a later pass) differs from what the
current attachments were made with, and drops the bloom and SSAO targets with
them (`game/render/src/renderer/draw.rs:612-640`). One of `retained`'s inputs
is this frame's `shafts` (`draw.rs:181-191`), and for a perspective camera
`water::shafts` is true only while the camera is under the surface
(`game/render/src/water.rs:49-59`: `frame.camera_position.y < w.height`).

So `retained` flips every time the camera crosses the surface, and every flip
makes new colour, depth and resolve attachments and a new bloom chain. Soft
particles have the same dependency and latch it for exactly this reason
(`draw.rs:167-172`: "Depth stays retained once they have drawn, so toggling
them never recreates the targets (and drops bloom and SSAO)"). Shafts have no
latch.

A frame with ambient occlusion, soft particles that have drawn, or the depth
prepass already retains the depth and does not flip.

## The failure

A renderer with the default environment (bloom on) and
`Water { height: 0., shafts: Some(Shafts { intensity: 1., length: 0.5 }), .. }`,
drawn six times at 256 by 256 with the eye alternately 0.2 m under and over the
surface, against the same frames with `shafts: None`. Cumulative
`Stats::texture_creations` after each frame:

```
no shafts: [16, 16, 16, 16, 16, 16]
shafts on: [16, 27, 38, 49, 60, 71]
```

Eleven textures per crossing: three frame attachments and the eight of
`BloomTargets`. Expected: none after the depth is first retained, as with soft
particles. At 1920 by 1080 with four samples those eleven are about 130 MB by
their formats.

## Verification

Ran that as a test in `game/render` on a Mac (Metal); it printed the two rows
above. Timing 60 such frames of sky and water at 1920 by 1080, from `draw` to
the GPU finishing the frame (medians):

| Frame | Recreating | Steady |
|---|---|---|
| goes under the surface | 4.2 to 4.7 ms | 2.6 ms |
| comes out | 2.3 ms | 1.2 ms |

So a crossing costs one to two milliseconds on this machine. No phone was
measured.

## Direction

Latch it as `soft_retained` does: keep the depth retained once shafts have
drawn, or whenever the frame's water has shafts that are allowed, whichever
side the camera is on.

## Related

QUEUE's "Water's shafts pass under a lid" is about what the shafts show, not
their attachments. "Exact's game holds 160 MB of graphics memory on a Pixel 5"
concerns steady memory.
