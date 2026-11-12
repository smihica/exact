# The CPU painter's damage repaint clears to white instead of the page's colour when the frame's size does not round-trip through the scale

**Status:** Closed
**Resolution:** CPU damage repaints clear to the page colour even when the frame size does not round-trip through its scale.
**Systems:** host/linux/src/raster.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Raster::damage` (`host/linux/src/raster.rs:380`) seeds a partial repaint from the
last frame and clears the damaged rectangles before the walk paints into them. It has
two ways to clear, chosen by `direct_clear` (`raster.rs:391`), which among other
things requires `(width / scale) * scale == width` in `f32` for the device width, and
the same for the height.

- When it holds, each rectangle is filled with the page's colour
  (`raster.rs:403`, `solid(self.page.unwrap_or([255; 4]))`).
- When it does not, the whole frame is filled through the damage mask with opaque
  white (`raster.rs:446` to `455`, `self.fill(.., [255; 4], ..)`).

`Backend::page` exists so that a frame is cleared to the platform's surface for the
scheme "and never white under a dark one" (`host/linux/src/paint/backend.rs:135`).
The second branch ignores it. Where the app paints no background of its own under the
damage, the rectangle turns white on a dark page.

The round trip holds at every width from 300 to 2,000 points at scales 1, 1.25, 1.5,
2, 2.25, 2.5 and 3. It fails for 4% of those widths at 1.75, 5% at 2.625 and 14% at
2.75.

## The failure

Text flowing around an exclusion, in the dark scheme, on the CPU painter; a key
moves the exclusion, which is the flow damage of LLP 1043.000 (`paint/damage.rs`):

```
component App
  state offset = 100
  action move(k: string)
    offset = offset + 5
  view
    column width=400 height=360 padding=20 gap=10 align-items="flex-start"
      button "After" id="after" testId="after" key=move
      view width=300 height=200 overflow="hidden"
        text "one two three four five six seven eight nine ten one two three four five six seven eight nine ten" width=300 height=200
        view position="absolute" left=offset top=0 width=80 height=60 wrap-flow="both" testId="hole"
```

The pixel at the middle of the empty exclusion box, before and after the key, in a
viewport 400 points tall:

```
440 points wide, scale 1     (440 px, round-trips)   (0, 0, 0)   (0, 0, 0)
440 points wide, scale 1.75  (770 px, round-trips)   (0, 0, 0)   (0, 0, 0)
513 points wide, scale 1.75  (898 px, does not)      (0, 0, 0)   (255, 255, 255)
```

Each repaint reported five damage rectangles. Expected: black in all three.

A person sees it on the CPU painter (no GPU adapter on Linux, `EXACT_PAINTER=cpu`, a
Windows game) at a fractional scale and one of the sizes that fail, in the dark
scheme, when exclusion-flow or split-pane text changes over the page's own colour.

## Verification

A temporary unit test in `host/linux/src/paint/` booted the Contract above with
`PainterChoice::Cpu`, called `set_system_scheme(true)`, painted, sent the key with
`type_key`, painted again and read the pixel; the table is its output. A second test
drove `Raster` alone (`page([18, 18, 18, 255])`, `begin`, `finish`, `begin_damage`
with one rectangle, `finish`) and read white inside the rectangle at 300 points and
scale 2.625, and the page's colour at scale 1. The share of failing widths comes from
evaluating the `f32` expression for each width.
`host/linux/tests/pinned/paint.rs` (`damage_clearing_matches_the_full_masked_viewport`) compares
the two branches with no page colour told, where both are white.

## Direction

Fill with `self.page.unwrap_or([255; 4])` in the second branch, as the first does.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names the damage clear.
