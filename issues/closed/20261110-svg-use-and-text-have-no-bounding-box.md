# An SVG `use`, a `text` and a group of them have no object bounding box, so a `filter` or a `mask` on one draws nothing on native hosts

**Status:** Closed
**Resolution:** A `use` (and a group of one) takes the object bounding box of its target, transformed and moved by its `x`/`y`, so a filter, mask, clip or `fill-box` on it measures what it draws; a `filter` or `mask` on a `text` takes a region that holds its glyphs until the resolver has advances.
**Systems:** kernel/src/svg/scene.rs, kernel/src/svg/scene/filter.rs, native SVG painting (Apple, Linux)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Resolver::bbox_of` (`kernel/src/svg/scene.rs:618-655`) knows three cases: a group (the
union of its children), a nested `svg`, and everything else through
`Resolver::geometry`, which ends in `svg::geometry` (`kernel/src/svg/shape.rs:55`) and
answers `None` for a `use` and for a `text`. So a `use`, a `text` and a `g` holding only
those have no box. SVG 2 §8.10 gives a `use` the box of what it instances and a `text`
the box of its glyph cells.

Every reader of the box then takes its "no box" branch:

- A `filter` element or a `mask` at its default units (`objectBoundingBox`), and an
  `objectBoundingBox` `clipPath`, hide the element (`scene/filter.rs:428`,
  `scene/island.rs:97`, `scene/clip.rs:72-80`).
- A CSS filter function grows its region from the box. With none the filter is empty
  and nothing is drawn (`scene/filter.rs:339-386`).
- `transform-box: fill-box` puts the origin at `0 0` (`scene.rs:572-574`), where LLP
  1055.000 D5 says it "spins an icon about its own middle".

## The failure

```
svg width=100 height=100 viewBox="0 0 100 100"
  defs
    filter id="f"
      feGaussianBlur stdDeviation=1
    rect id="r" x=10 y=10 width=20 height=10 fill="#dc2626"
  use href="#r" filter="url(#f)"
```

| On the element | Chrome 156 | macOS app |
|---|---|---|
| `use`, `filter="url(#f)"` | 284 pixels painted | none |
| `text`, `filter="url(#f)"` | 121 | none |
| `use`, `mask="url(#m)"` (the mask holds a white 100×100 `rect`) | 200 | none |
| `use`, `filter="blur(1px)"` | 324 | none |
| `text`, `filter="blur(1px)"` | 121 | none |
| `use`, `transform-box="fill-box" transform-origin="center" rotate=90` | at `15,5`, 10×20 | none: outside the `svg`, at `-20,10` |

Each row on the `rect` itself is painted by both. A `g` holding only the `use` behaves
as the `use` does.

## Verification

- Each row and its control in a temporary page in `apps/hello`
  (`bun host/apple/build.mjs hello-apple`, then
  `bun scripts/agent.mjs macos --app hello "screenshot out.png"`): the pixels of each
  `svg` that differ from its background, counted in the screenshot.
- A kernel test over `scene::resolve` printed, for the `use`: origin `(0.0, 0.0)` and
  the target painted at `(-20.0, 10.0, 10.0, 20.0)`; a `blur(1px)` filter of region
  `(0.0, 0.0, 0.0, 0.0)` and no primitives (the `rect`: `(7.0, 7.0, 26.0, 16.0)` and
  one); an `objectBoundingBox` clip of no shapes; a mask and a `filter` element of empty
  region.
- Headless Chrome 156: `getBBox()` is `10,10,20,10` for the `use` and for a `g` holding
  it; the counts are of the same documents drawn to a canvas.
- The Linux host was not run. It returns before drawing a filter with no primitives
  (`host/linux/src/paint/island.rs:116`).

## Direction

Give `bbox_of` a `use` arm: its target's box through the instance's placement. A `text`
arm needs each chunk's advance, which the resolver does not have yet; text hit testing
waits on the same (`scene/hit.rs:70`). Until it has one, a filter or a mask on a `text`
should fall back to a region that draws, not to nothing.

## Related

The `QUEUE.md` entry "LLP 1055 / 1055.000 (SVG, completed in stages) owes" lists text hit
testing and does not list a box for `use` or `text`.
