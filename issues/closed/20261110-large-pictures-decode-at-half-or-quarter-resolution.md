# On Apple a picture whose box is over about 1.5 million device pixels decodes at half or a quarter of the box's resolution

**Status:** Closed
**Resolution:** Apple decodes use the session budget and reduce resolution only to the largest size it holds, with an observable budget reason.
**Systems:** host/apple/Sources/ExactKit/RasterImage.swift, host/apple/Sources/ExactKit/RasterLoader.swift, host/raster/src/policy.rs
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`RasterDecodePlan.init` (`host/apple/Sources/ExactKit/RasterImage.swift:387-413`) reserves the output (4 bytes a pixel, rows rounded up to 64 bytes) plus an ImageIO staging allowance of 16 bytes a pixel with rows rounded up to 4096 bytes, and throws `tooLarge` when the sum passes a fixed 32 MiB. The shared core holds each request to the same constant, on purpose (`host/raster/src/policy.rs:1143`; `SESSION_BYTES`, `host/raster/src/types.rs:5`, "one decode's peak, always"). That admits 1.4 to 1.7 million pixels, by how the width rounds, whatever the session's budget is.

The session's budget is larger. `RasterLoader.viewportBudget` (`RasterLoader.swift:536-541`) gives eight RGBA bitmaps the size of the view's bounds (LLP 1008 §9), up to 192 MiB. One bitmap of that size can never be planned.

When no plan exists, `RasterLoader.reconcile` (`RasterLoader.swift:1063-1065`) halves the long side until one does. It logs nothing and sets no failure; the agent's state shows only `offeredPixel` beside `admittedPixel`.

## The failure

An `image` whose `object-fit` is `cover` or the default `fill`:

| box in device pixels | picture | decoded | of the box's pixels, each axis |
| --- | --- | --- | --- |
| 1179 x 1179 | 2048 x 2048 | 1179 x 1179 | 1.00 |
| 1179 x 1572 (393 x 524 pt at 3x: a phone's width) | 3024 x 4032 | 590 x 786 | 0.50 |
| 1080 x 1350 (540 x 675 pt at 2x) | 1080 x 1350 | 540 x 675 | 0.50 |
| 1320 x 2868 (440 x 956 pt at 3x: a large iPhone's screen) | 3024 x 4032 | 538 x 717 | 0.25 |
| 2064 x 2752 (1032 x 1376 pt at 2x: a 13-inch iPad's screen) | 3024 x 4032 | 516 x 688 | 0.25 |
| 2880 x 1800 (a 1440 x 900 Mac window at 2x) | 3024 x 4032 | 720 x 960 | 0.25 |

A full-width portrait photo in a phone's feed shows 1.5 pixels a point on a 3x screen. A full-screen photo on a 13-inch iPad shows one decoded pixel for every 16 device pixels. A 1080 x 1350 picture misses the bound by 34 KB and is halved though 1079 x 1348 fits.

Expected: the size the box paints (`requestedPixel`'s comment, `RasterLoader.swift:802-805`: "as a browser decodes for the size it paints"). LLP 1010 §6.3 lowers resolution only when "visible demand exceeds the budget", "with an observable reason", and LLP 1100 D7 makes it the last step of its fallback ladder. Here the budget has room, and no LLP accepts a reduced decode at screen size.

## Verification

A temporary XCTest on macOS (`bun host/apple/build.mjs --test --only <class>`) wrote a PNG of each picture size and loaded it into an `image` view with that box's device pixels, through `RasterLoader`, the shared core and ImageIO, with a 192 MiB session budget and nothing else loaded. It printed the table's sizes, for example:

```
decoded 538 x 717; offeredPixel 2868, admittedPixel 717, failure "", lastRefusal none
```

No picture of a device's screen was taken. The softness follows from the decoded size, which the view's layer stretches to the box.

## Direction

Hold one decode's peak to the session's budget, which the account already enforces (`host/raster/src/account.rs:58-63`), or reserve what ImageIO's thumbnail is seen to stage (`decode` already checks it, `RasterImage.swift:513-518`) in place of 16 bytes a pixel. Where a plan must still shrink, take the largest side that fits, not half, and log it.

## Related

QUEUE.md "An HDR edit of a 24-megapixel photo falls back to SDR on Apple" is the photo editor's own transform. `issues/closed/20261019-eager-image-rail-starves-raster-queue.md` is admission order. LLP 1100's fixture F17 records the ceiling for a 4096 x 4096 HDR plan only.
