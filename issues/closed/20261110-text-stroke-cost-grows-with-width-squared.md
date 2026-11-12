# `-webkit-text-stroke` is recomputed on the CPU at every paint, on every painter, at a cost that grows with the square of its width and about the fourth power of the scale

**Status:** Closed
**Resolution:** Paragraphs reuse bounded stroke bands while preserving glyph phases and pixel rows; disc morphology uses linear row passes, and painters without island images skip the work.
**Systems:** host/linux/src/paint/text_stroke.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The painting host draws a text stroke as a band: the paragraph painted into a CPU
island, dilated by half the width, less the same eroded
(`host/linux/src/paint/text_stroke.rs:15`, `stroke_pass`). `morphology`
(`text_stroke.rs:61`) visits, for every pixel of the island, every pixel within the
radius in device pixels, twice (once to dilate, once to erode), and takes a square
root at each visit. The island is the whole paragraph plus a margin, at device scale.
The work is island area times radius squared: with the scale `s` and the width `w`,
`s² · (w·s)²`.

Nothing keeps the band. Every paint that walks the paragraph runs both passes again,
on the CPU, whichever painter draws the frame: the GPU and Ink painters take the band
as an image (`island_image`). The Canvas recorder, Android's, has no `island_image`,
so there the band is computed and dropped. Only a paragraph inside a scroller's kept
row is spared, while the row stands.

## The failure

One line in a 390 by 300 page:

```
text "Exact paints a headline" font-size=32 font-weight=800 color="#fdd835" -webkit-text-stroke="2px #000000"
```

Median of seven paints of the unchanged page, `host-dev` (optimized) profile, Apple
Silicon, CPU painter:

```
            no stroke   1px       2px       4px       8px
scale 1     0.0 ms      0.6 ms    0.6 ms    1.4 ms    4.5 ms
scale 2     0.1 ms      2.8 ms    6.1 ms    17.5 ms   70.7 ms
scale 3     0.2 ms      12.7 ms   20.7 ms   70.9 ms   313.6 ms
```

The same page on each painter:

```
                 CPU       GPU       Ink
scale 2, none    0.1 ms    2.0 ms    1.1 ms
scale 2, 2px     4.7 ms    8.1 ms    7.5 ms
scale 3, none    0.2 ms    2.0 ms    1.2 ms
scale 3, 2px     18.7 ms   23.2 ms   22.4 ms
scale 3, 4px     70.2 ms   72.7 ms   69.1 ms
```

The seventh paint of the unchanged page costs as much as the median. At scale 3 a 2
pixel stroke on one
headline costs more than a 60 Hz frame, and it is paid at each frame of a scroll or of
an animation the engine samples. Doubling the width from 4 to 8 multiplies the time by
4.4; going from scale 1 to 3 at 4 pixels multiplies it by 51.

## Verification

Temporary unit tests in `host/linux/src/paint/` booted the Contract above at each
scale and width and timed `Painter::paint` over `Raster`, `Gpu` and `Ink`
(`cargo test --profile host-dev -p exact-linux --lib`); the tables are their output.
Other builds were running on the machine, so the absolute times are approximate. A
test in `host/linux/src/canvas/` painted the page through `canvas::Recorder`: the
stream for the stroked paragraph holds the same three ops as the unstroked one, and
no image. No phone was run.

## Direction

Keep the band with the paragraph for the (width, scale, colours) it was made for, as
the paragraph's `ink` cache keeps its glyph index (`host/linux/src/text.rs:304`), so a
paint that changes nothing reuses it. To
make the first one cheap, compute the disc's weights once per radius instead of a
square root per visit, or replace the two disc passes with a distance transform, which
is linear in the island's area.

## Related

LLP 1001 declares the band's look (no inner contours) and LLP 1077's stage 3 notes
describe it; neither gives a cost. No app under `apps/` uses the property today.
Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names `text-stroke`
performance.
