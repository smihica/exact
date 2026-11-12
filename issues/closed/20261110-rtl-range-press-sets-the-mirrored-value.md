# A press on a right-to-left range sets the value of the mirrored point, so the thumb jumps away from the pointer

**Status:** Closed
**Resolution:** The painter and the press read one thumb geometry (`paint::control::range_thumb`, `range_mirrored`), so a right-to-left range has its minimum at the right under both looks and a press sets the value under the pointer.
**Systems:** host/linux/src/presenter/control.rs, host/linux/src/paint/neutral.rs (the neutral look: Linux desktop, Windows)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The neutral look paints a range's thumb from the inline start: under `direction: rtl` the
minimum is at the right (`host/linux/src/paint/neutral.rs:179`,
`let along = if rtl(node) { 1.0 - fraction } else { fraction };`). That is what a browser
draws.

The press that sets the value does not know the direction. `press_range` takes the fraction
from the left edge whatever the box's direction
(`host/linux/src/presenter/control.rs:342` to `:347`):

```rust
let t = if bw > 16.0 { ((x - bx - 8.0) / (bw - 16.0)).clamp(0.0, 1.0) as f64 } else { 0.5 };
let value = range.min + t * (range.max - range.min);
```

So the paint and the press disagree. In a right-to-left app a press four fifths of the way
across sets four fifths of the range, and the thumb is then painted one fifth of the way
across. Every press on a slider lands at its mirror image.

## The failure

```
box direction="rtl" width=400
  input type="range" min=0 max=100 testId="r" width=200 accent-color="#ff0000"
```

under the neutral look (`app::show_neutral_look`), a press and release at 80% of the
track's width:

| direction | range box x | pressed at x | value | thumb painted at x |
| --- | --- | --- | --- | --- |
| ltr | 0 to 200 | 160 | 82.6 | 152 to 166 |
| rtl | 200 to 400 | 360 | 82.6 | 233 to 247 |

Expected under `rtl`: the thumb under the pointer (about 352 to 366) and a value near 17.
Chromium 156, `<div dir="rtl"><input type="range" min="0" max="100" style="width:200px">`,
a click at x 360 of a box from 200 to 400: value 17 (83 for the same click under `ltr`).

## Verification

A temporary test in `exact-linux`'s library set the neutral look (alone in its process: the
look is process-wide), booted the Contract with the CPU painter, pressed and released at the
point, and printed the chosen value and the bounds of the accent-coloured pixels: the table.
The Chromium values are the input's `value` after `mouse.click`. Under the Android look the
same press gives 82.6 in both directions and the fill is painted from the box's left edge to
the pointer in both (`host/linux/src/paint/control.rs:334` to `:350`), so paint and press
agree there. Not run: Windows.

## Direction

In `press_range`, mirror `t` when the range's computed direction is right-to-left, as
`neutral::range` mirrors `fraction`. The assistive step (`presenter/access.rs`) works on the
value and is unaffected.

## Related

`issues/20261110-range-takes-no-drag-or-keys-on-painted-hosts.md` is the missing drag and
keys; a drag added for it needs the same mirror. QUEUE's "Linux host" entry among the
declared gaps lists "RTL" from LLP 1015 §7 ("RTL untested", of the v1 host's text); the
range's paint already handles the direction, and this is the press disagreeing with it.
