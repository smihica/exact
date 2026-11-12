# Apple Canvas 2D draws a camera photo unrotated: its image decode ignores EXIF orientation

**Status:** Closed
**Resolution:** Canvas image handles decode at full size with EXIF orientation applied to pixels and reported dimensions.
**Systems:** host/apple/Sources/ExactSurfaces/Canvas2DImage.swift, Canvas 2D images (LLP 1056 D9)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `Canvas2DHost.load` decodes an image handle with `CGImageSourceCreateImageAtIndex` (`host/apple/Sources/ExactSurfaces/Canvas2DImage.swift:166-167`), which returns the stored pixels and does not apply the file's orientation tag. The session tells the runtime that image's stored width and height (`host/apple/Sources/ExactKit/Session.swift:528-531`), and `drawImage` draws the stored pixels (`Canvas2DImage.swift:24-40`).

LLP 1056 D9 makes a handle "the same URL or asset value an `image` node's `src` takes", and the LLP declares no limit on orientation. The Apple `image` node honours the tag: its natural size swaps the sides for orientations 5 to 8 (`host/apple/Sources/ExactKit/RasterImage.swift:80-84`) and it decodes with `kCGImageSourceCreateThumbnailWithTransform` (`RasterImage.swift:585-590`). The web host loads a handle into an `Image` and reports `naturalWidth` and `naturalHeight` (`host/web/canvas2d-glue.js:277-291`), which include the orientation (HTML, "The `img` element"; `image-orientation` is `from-image` by default, CSS Images 3 §6.2).

## The failure

A JPEG of 40 x 20 stored pixels, red in its left half and blue in its right, tagged orientation 6 (a phone camera's portrait): shown, it is 20 x 40 with red on top. A draw does `ctx.drawImage(photo, 0, 0)` on a 60 x 60 canvas.

| | Apple Canvas 2D | Chromium 156 |
| --- | --- | --- |
| the image's size | 40 x 20 | 20 x 40 |
| canvas pixel (10, 10) | red | red |
| canvas pixel (30, 10) | blue | clear |
| canvas pixel (10, 30) | clear | blue |

The Apple column is what the same file gives with no tag. The Apple `image` node's decode of the same bytes is 20 x 40, red at (5, 10) and blue at (15, 30). So a canvas draws a portrait photograph lying on its side, and anything the draw sizes from the image's width and height is transposed. A tag of 3 (upside down) is ignored the same way: the canvas draws red at the left, where the `image` node's decode has blue.

## Verification

A temporary Mac XCTest (`bun host/apple/build.mjs --test --only <Class>`) wrote the JPEG with ImageIO (`CGImageDestinationAddImage` with `kCGImagePropertyOrientation: 6`), loaded it through `Canvas2DHost.load`, replayed the `drawImage` record for the size the host reported, and read the pixels; for the `image` node it called `RasterMetadata.read` and `RasterImage.decode` on the same bytes. It printed the Apple figures. The Chromium column is a file written the same way, loaded in headless Chromium: `naturalWidth`, `naturalHeight`, and `getImageData` after `drawImage(img, 0, 0)`. Not run on iOS, where the same file compiles. The Linux and Android canvas decodes were not read.

## Direction

Decode with the orientation applied (`kCGImageSourceCreateThumbnailWithTransform` at the image's full size, as `RasterImage` does), so the size told to the runtime and the pixels drawn are the upright picture's.

## Related

`QUEUE.md`, "LLP 1056 (Canvas 2D) stage 2 owes" (3), is that this cache is never evicted and sits outside LLP 1010 §6's budget. `issues/20261110-large-pictures-decode-at-half-or-quarter-resolution.md` is the `image` node's decode plan, not the canvas's.
