# A painted button's leading symbol stays on the left in a right-to-left direction

**Status:** Closed
**Resolution:** Painted buttons resolve leading and trailing symbol positions using the button's inherited direction.
**Systems:** host/linux/src/paint/button.rs (the painting host: Linux, Windows, Android)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The kernel names a semantic button's image placement by tree order: an image before the
title is `Leading`, after it `Trailing` (`kernel/src/arena/button.rs:179` to `:188`).
Leading is the inline start, which is the right in a right-to-left direction.

The painter reads the placement as a physical side. `face_layout` sets `leading` from the
placement alone (`host/linux/src/paint/button.rs:223` to `:226`), and `button_control` puts
a leading symbol at the content's left edge `bx` and the title after it (`button.rs:856`
to `:870`). The only use of the direction in the file is the title's `text-align`
(`button.rs:812` to `:815`). So in an Arabic or Hebrew app every button with a symbol and a
title has them in the left-to-right order, under the Android look and the neutral one.

## The failure

```
component App
  view
    column width=400 height=400 background-color="#ffffff"
      box direction="rtl" width=400
        button testId="rtl" width=200 height=44
          image "symbol:star" -exact-tint-color="#ff0000"
          text "Save" color="#0000ff"
```

The button is at x 200 to 400. Where the symbol's and the title's pixels are:

| | symbol | title |
| --- | --- | --- |
| Android look, `direction="rtl"` | 272 to 282 | 295 to 328 |
| the same button under `ltr`, at x 0 to 200 | 72 to 82 | 95 to 128 |
| neutral look, `direction="rtl"` | 273 to 284 | 293 to 327 |
| Chromium 156 | 376 to 392 | 346 to 376 |

Expected: the symbol right of the title. A row's first item sits at the inline start (CSS
Flexbox 1 §5.1 with CSS Writing Modes). The Chromium row is
`<div dir="rtl"><button style="display:flex;width:200px"><span id="img">…</span><span>Save</span></button></div>`.
UIKit's `.leading` placement, which the iOS presenter sets
(`host/apple/Sources/ExactKit/IOS/ButtonConfigurationIOS.swift:226` to `:231`), mirrors the
same way.

## Verification

A temporary test in `exact-linux`'s library booted the Contract with the CPU painter and
read the bounds of the red and blue pixels in the button's box, once under each look (the
neutral one alone in its process, since `app::show_neutral_look` is process-wide): the
first three rows. The fourth is `getBoundingClientRect` in Chromium. Not run: the web host
and the Apple hosts with this Contract; an Android device.

## Direction

In `button_control`, read the button's direction (as `title_style.direction` already is for
alignment) and swap the two x positions when it is right-to-left; `face_layout`'s sizes do
not change.

## Related

QUEUE's "Linux host" entry among the declared gaps lists "RTL" from LLP 1015 §7 ("RTL
untested", of the v1 host's text); it names no control, and the neutral controls beside the
button already mirror (`paint/neutral.rs`: the switch, the range, the select's caret, the
file field). `issues/20261110-titlebar-area-wrong-in-rtl.md` is the Mac title bar. QUEUE's
"An RTL grouped list swaps UIKit's unequal card margins" is UIKit's list.
