# On the CPU painter a collapsed `overflow: hidden` box inside another clip shows all of its content

**Status:** Closed
**Resolution:** CPU clip intersections have empty coverage when any box has no area, including inside an existing clip.
**Systems:** host/linux/src/raster/mask.rs, host/linux/src/raster.rs (the CPU painter)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A clip whose box has no area (`height: 0; overflow: hidden`, an accordion's closed body)
must show nothing. `Raster::push_clip` gets that only when no clip is above it: `mask_with`
returns `None` and the `None` arm pushes an empty mask ("Nothing can show inside an empty
box", `host/linux/src/raster.rs:1065` to `:1071`).

Under any other clip, `mask_with` drops the empty shape instead. `rounded_rect` returns
`None` for a shape with no width or height (`host/linux/src/raster/geometry.rs:6`), and
every arm of `mask_with` reads its shapes through `filter_map(rounded_rect)`
(`host/linux/src/raster/mask.rs:106`, `:133`, `:151`, `:169`). With an active parent mask
the function falls through to `(**c).clone()` (`mask.rs:145`) and returns the parent's mask
unchanged; with an unmasked parent it returns a full mask (`mask.rs:149` to `:154`). The
collapsed box then clips nothing.

The GPU and Ink painters handle the case (`host/linux/src/gpu.rs:916`,
`host/linux/src/ink/clips.rs:119`). The CPU painter is the pixel oracle of the tests and
paints when Linux finds no GPU (`host/linux/src/presenter/painter.rs:241`), under
`EXACT_PAINTER=cpu`, for a Windows game, and for the first frames on Windows and under the
Mac carrier (`host/linux/src/app.rs:435`, `:471`).

## The failure

```
component App
  view
    box width=400 height=400 background-color="#ffffff"
      scroll width=300 height=300
        text "Section" font-size=16
        box testId="body" height=0 overflow="hidden"
          box width=100 height=100 flex-shrink=0 background-color="#ff0000"
        text "Next section" font-size=16
```

The body's box is (0, 19, 300, 0). Red pixels in the frame:

| painter | red pixels |
| --- | --- |
| CPU | 9,598, spanning (0, 19) to (99, 118), over "Next section" |
| GPU | 0 |
| Ink | 0 |

Expected: 0 on each.

## Verification

Temporary tests in `exact-linux`'s library booted the Contract above with
`PainterChoice::Cpu`, `Gpu` and `Ink` and counted red pixels: the table. On the CPU painter
the same collapsed box in a plain `overflow="hidden"` parent painted 10,000 red pixels with
a height of 0 and with a width of 0, 9,964 in a rounded parent, 100 (one row) with a height
of 1, and 0 with no clipping ancestor. Directly on `Raster`: `push_clip` of the shape
(10, 10, 20, 0) alone leaves a mask covering 0 pixels; under a rounded parent covering 1,560
pixels the mask covers 1,560, and under a square one covering 1,600 it covers 1,600. Not
run: the Android recorder; an image's or a canvas's clip list, which goes through the same
`mask_with`.

## Direction

In `mask_with`, a shape with no area makes the result an empty mask (`self.new_clip_mask()`)
before any arm reads the shapes. The test
`bounded_child_intersection_preserves_parent_coverage_and_order` (`mask.rs:369`) builds its
reference with the same `filter_map` (`:391`), so its case with a zero-size shape (`:387`)
asserts the defect and passes today; it needs the corrected expectation.

## Related

`issues/20261110-overflow-clips-at-border-box.md`,
`issues/20261110-overflow-x-clip-clips-both-axes.md` and
`issues/20261110-cpu-clip-cache-ignores-corner-shape.md` are other rules of the same clip;
none covers an empty box. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` does.
