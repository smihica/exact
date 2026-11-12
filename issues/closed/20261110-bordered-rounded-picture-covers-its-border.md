# A rounded picture with a border paints over its own border on the painting host

**Status:** Closed
**Resolution:** The painter clips a picture and a 2D canvas's bitmap to the content box on the content edge's curve (`BoxGeometry::content_edge`), so they end at the border's inner curve.
**Systems:** host/linux/src/paint/rows.rs, host/linux/src/paint.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

CSS trims replaced content to the content edge's curve: each corner radius less the
border and padding beside it (CSS Backgrounds 3 §5.3). LLP 1011 §4 holds the Apple
host to the same: "clipped to the content edge's curve ... a bordered rounded picture
ends at the border's inner curve."

The painter that Linux, Windows, Android and `apple-painted` share clips a picture to
two shapes: the unrounded content rectangle and the border box's outer curve
(`host/linux/src/paint/rows.rs:427`, `&[Shape::rect(content), outer]`). Between the
inner and outer curves the content rectangle still covers the corner, so the picture,
drawn after the border, paints over it there. A 2D canvas takes the same pair
(`host/linux/src/paint.rs:1147` and `:1149`).

## The failure

A round avatar with a ring:

```
image "avatar.png" width=60 height=60 box-sizing="border-box" border-style="solid" border-width=6 border-color="#ff0000" border-radius="50%"
```

The border is the ring between 24 and 30 points from the centre. With a solid blue
picture:

```
                                   painting host   web host, Chrome 156   macOS
(30, 3)   r=26.5, vertical axis    (255, 0, 0)     (255, 0, 0)            (255, 0, 0)
(3, 30)   r=26.5, horizontal axis  (255, 0, 0)     (255, 0, 0)            (255, 0, 0)
(11, 11)  r=26.2, diagonal         (0, 0, 255)     (255, 0, 0)            (255, 0, 0)
(49, 11)  r=26.9, diagonal         (0, 0, 255)     (255, 0, 0)            (255, 0, 0)
(48, 48)  r=26.2, diagonal         (0, 0, 255)     (255, 0, 0)            (255, 0, 0)
(14, 14)  r=21.9, picture          (0, 0, 255)     (0, 0, 255)            (0, 0, 255)
```

The CPU, GPU and Ink painters agree. On the painting host the ring shows as four
short arcs, at the top, bottom, left and right; elsewhere the picture reaches the
outer edge.

## Verification

A temporary unit test in `host/linux/src/paint/` booted the Contract above, gave the
node a 48 by 48 blue bitmap through `Scene::images`, painted with `Raster`, the GPU
painter and Ink, and read the pixels. The same image as a `data:` source in
`apps/hello/app.contract` was built for the web and for macOS and screenshotted with
`scripts/agent.mjs web` and `macos`; an `<img>` with the same styles in Chromium 156
gave the web column's values. A 2D canvas was read, not run.

## Direction

Clip the picture (and a canvas's bitmap) to one shape: the content box with each
radius reduced by the border and padding beside it. `BoxPaint::background_shape`
already builds that shape for `background-clip: content-box` (`paint.rs:328`).

## Related

The painter clips a box's children at the border box where CSS uses the padding box:
the same mistake under a different rule and at a different line. Nothing in `issues/`,
`QUEUE.md` or `docs/agent-pitfalls.md` covers either.
