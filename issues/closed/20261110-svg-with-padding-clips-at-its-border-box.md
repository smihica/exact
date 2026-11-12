# An `svg` with padding clips its content at the border box, so it draws over its own padding

**Status:** Closed
**Resolution:** SVG scenes clip to the content box and its inner corner curves, leaving padding and borders clear.
**Systems:** host/linux/src/paint/svg.rs (the painting host: Linux, Windows, Android)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Painter::paint_svg` maps the scene's view box into the content box but clips it to `rect`,
the border box (`host/linux/src/paint/svg.rs:379` to `:404`; its doc says so: "clipped to
the border box `rect` when `clips`"). The caller passes both rectangles
(`host/linux/src/paint.rs:1185`).

An `svg` whose `overflow` is not `visible` is clipped to its viewport: "a clip, the exact
size of the SVG viewport is applied" (SVG 2 §3.11), and the user agent sheet sets
`overflow: hidden` on an `svg` that is not a document's root. The viewport is where the
view box is mapped, the content box. So whatever the scene draws outside its viewport (a
shape past the view box, a stroke centred on its edge) shows in the padding and under the
border on this host.

## The failure

```
component App
  view
    box width=400 height=400 background-color="#ffffff"
      svg width=100 height=100 padding=20 viewBox="0 0 100 100"
        rect x=-50 y=-50 width=200 height=200 fill="#ff0000"
```

The border box is 140 by 140 and the content box is (20, 20) to (120, 120).

| pixel | CPU, GPU and Ink painters | Chromium 156, the same `<svg style="padding:20px">` |
| --- | --- | --- |
| (10, 10), padding | red | white |
| (25, 25), content | red | red |
| (130, 130), padding | red | white |
| (150, 150), outside | white | white |

## Verification

A temporary test in `exact-linux`'s library booted the Contract with `PainterChoice::Cpu`,
`Gpu` and `Ink` and read the four pixels of the frame; the three painters agree. The
Chromium column is a screenshot of
`<svg style="display:block;padding:20px" width="100" height="100" viewBox="0 0 100 100">`
holding the same `rect`. Read, not run: the Apple host, which clips to the content box
(`host/apple/Sources/ExactKit/SvgScene.swift:242`). Not run: the web host.

## Direction

Clip to `content` in `paint_svg` (with the box's inner radii where it has them).
`svg_picture` passes one rectangle for both and is unchanged.

## Related

`issues/20261110-overflow-clips-at-border-box.md` is a box's children, clipped at the
border edge where CSS says the padding edge (`paint.rs:1251`). Its fix does not reach this:
the scene has its own `push_clip` (`paint/svg.rs:394`) and its edge is the content box.
`issues/20261110-bordered-rounded-picture-covers-its-border.md` is an image's clip
(`paint/rows.rs:427`).
