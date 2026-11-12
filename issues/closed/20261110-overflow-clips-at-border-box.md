# The painting host clips `overflow` at the border box, so clipped content paints over the box's own border

**Status:** Closed
**Resolution:** The painter clips a box's children at its padding box with the padding box's radii (`overflow_clip` in `host/linux/src/paint/field.rs`), so the border paints over them; LLP 1001 says so (llp 1fa1e1d4).
**Systems:** host/linux/src/paint.rs, LLP 1001
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

CSS clips a box whose `overflow` is not `visible` at its padding edge, with the
padding box's (inner) corner radii (CSS Overflow 3 §3.1 and §3.2, "overflow clip
edge"; CSS Backgrounds 3 §5.3). The painter that Linux, Windows, Android and
`apple-painted` share pushes the clip for a box's children at the border box:
`host/linux/src/paint.rs:1251` is `self.backend.push_clip(&surface.outer, ts)`, and
`surface.outer` is the border box with the outer radii. The border is painted before
the children (`paint.rs:1119`), so whatever a child draws between the border edge and
the padding edge covers the border.

The other hosts keep the border. The web host emits CSS. The Apple host makes a
clipping box's border the layer's own, "which Core Animation paints over the
sublayers" (`host/apple/Sources/ExactKit/IOS/BoxLayerIOS.swift:267`). The runner's
own hit walk clips at the padding box (`runner/src/geometry.rs:1312` to `1320`), so
the painter and the runner disagree about where a clipped child ends.

LLP 1001 says `overflow: hidden` "clips its children to the rounded border box on
every host". The sentence is not among its declared deviations, and it is false of the
web.

## The failure

Three boxes, each 100 by 100, `box-sizing="border-box"`, with a red border and a blue
child:

```
A  box overflow="hidden" border-style="solid" border-width=10 border-color="#ff0000"
     box width=200 height=200 margin-left=-10 margin-top=-10 background-color="#0000ff"
B  box overflow="hidden" border-style="solid" border-width=6 border-radius=20 border-color="#ff0000"
     box width=88 height=88 background-color="#0000ff"
C  scroll border-style="solid" border-width=10 border-color="#ff0000"
     box width=80 height=300 background-color="#0000ff"
```

```
                               painting host     web host, Chrome 156   macOS
A  (50, 5)   top border        (0, 0, 255)       (255, 0, 0)            (255, 0, 0)
B  (8, 8)    inner corner      (0, 0, 255)       (255, 0, 0)            (255, 0, 0)
C  (50, 95)  bottom border     (0, 0, 255)       (255, 0, 0)            (255, 0, 0)
```

The CPU, GPU and Ink painters agree on each. C needs no scrolling: a bordered
scroller with more content than fits has it painted over its bottom border, and over
the top one once scrolled. `apps/unsaid/app.contract:285` and
`apps/ui-gallery/text-styles.contract:101` are such scrollers.

## Verification

Temporary unit tests in `host/linux/src/paint/` booted each Contract with
`PainterChoice::Cpu`, `Gpu` and `Ink` and read the frame's pixels. The same Contracts
as `apps/hello/app.contract` were built for the web and for macOS and screenshotted
with `scripts/agent.mjs web` and `macos`. Bare `<div>`s with the same styles in
Chromium 156 gave the web column's values. UIKit was read, not run.

## Direction

Push the children's clip at the padding box: the outer shape inset by the border
widths with radii reduced by them, as `BoxPaint::background_shape` builds for
`background-clip: padding-box` (`paint.rs:328`), and record that rectangle as the
painted box's clip (`paint.rs:1252`). Correct LLP 1001's sentence to say the padding
box.

## Related

`issues/closed/20261102-hit-testing-uses-the-authored-border-width.md` fixed the width
the runner's hit clip is inset by; it did not touch the painter. A rounded picture
with a border covers its border by the same mistake at another line and under another
rule (the content edge). Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` covers the
clip edge.
