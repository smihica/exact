# `feTurbulence` with `stitchTiles="stitch"` does not stitch, and a fractional `seed` picks another pattern than Chrome's

**Status:** Closed
**Resolution:** feTurbulence wraps lattice coordinates before masking and truncates fractional seeds toward zero.
**Systems:** host/svg-raster/src/filter/noise.rs (Apple's SVG island module, the Linux host)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `host/svg-raster/src/filter/noise.rs` says it is "Filter Effects 1 §15.23's reference algorithm, as the spec gives it in C", and LLP 1055.000 stage 9b lists `feTurbulence` as "the spec's reference algorithm, stitching included". The specification calls that code "the exact algorithm used for this filter effect" ([Filter Effects 1, feTurbulence](https://www.w3.org/TR/filter-effects-1/#feTurbulenceElement)). The file differs from it in two places.

**The lattice is masked before the stitch test.** `noise2` (`noise.rs:79-87`) computes `bx0 = (t as i64) & BM` and `bx1 = (bx0 + 1) & BM`, and the same for `y`, so each is in 0..=255. The stitch test that follows (`noise.rs:88-101`) compares them with `wrap_x`, which is 4096 or more for a tile at a non-negative origin. It is never true, so a stitched tile is never wrapped. The specification's `noise2` takes `bx0 = (int)t; bx1 = bx0+1;`, tests against `nWrapX`, and masks afterwards (`bx0 &= BM`).

**The seed is rounded.** `Lattice::new` (`noise.rs:40-42`) takes `seed.round()`, and its comment says Chrome rounds half away from zero. The specification says the seed "must first be truncated, i.e. rounded to the closest integer value towards zero", and Chrome 157 truncates.

The web hands the filter to the browser, so an app differs from its web build on the hosts that run this raster.

## The failure

```
<filter id="f" filterUnits="userSpaceOnUse" x="0" y="0" width="100" height="100">
  <feTurbulence x="0" y="0" width="50" height="50" baseFrequency="0.033"
                numOctaves="2" seed="3" stitchTiles="stitch" result="n"/>
  <feTile in="n"/>
</filter>
```

The four 50 px tiles should join without an edge. Mean channel step between neighbouring columns of the result (0 to 255):

| | across the seam (x 49, 50) | inside a tile (x 24, 25) |
| --- | --- | --- |
| Chrome 157 | 5.57 | 4.27 |
| the raster | 14.72 | 3.80 |
| the raster with the mask moved after the test | 5.52 | 4.12 |

(The raster's rows sample pixel centres, as the comparison with Chrome does; as shipped, at pixel corners, its seam is 16.13 against 3.63.)

`seed="1.5"`: Chrome draws exactly its `seed="1"` picture (maximum difference 0), the raster its `seed="2"` picture. `seed="-1.5"`: Chrome draws its `seed="2"` picture (truncated to -1, which the algorithm maps to 2), the raster another pattern.

## Verification

Run. A test in `host/svg-raster` built each chain as an `exact_svg_filter::Filter` and ran `exact_svg_raster::filter::run` at 100 by 100 over a red square; a page drew the same filters in headless Chrome 157.0.8103.50 into a canvas and the pixels were compared. Without `stitchTiles` and with integer seeds the two agree (mean difference 2 of 255 when the raster samples pixel centres), so the comparison is like with like. With the mask taken off the four lattice lines and `trunc()` for the seed, the stitched cases fall from a mean difference of 12.1 and 34.9 to 2.7 and 1.3, and the fractional seeds match Chrome's.

## Direction

Take `bx0`, `bx1`, `by0`, `by1` unmasked, as the specification does (the mask after the stitch test is already there, `noise.rs:102-107`), and truncate the seed. Correct the comment on the seed.

## Related

`apps/svg-gallery` uses `feTurbulence` with integer seeds and no `stitchTiles`, so it is unaffected. None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` for `feTurbulence`, `stitch` or `seed`.
