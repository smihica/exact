# In the terminal a box scrolled out of view takes with it the children that reach outside its rows

**Status:** Closed
**Resolution:** The terminal paints overflowing descendants when their parent is outside the visible rows, retaining ancestor clipping on each axis according to its computed overflow.
**Systems:** host/terminal/src/paint.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. The painter returns early for a box whose own rows are all above or
below the clip (`host/terminal/src/paint.rs:192-196`):

```rust
// A box wholly above or below the rows asked for shows nothing, nor do
// its flow children: a long transcript paints only the rows in view.
if rect.h > 0 && (rect.y + rect.h <= clip.y || rect.y >= clip.y + clip.h) {
    return;
}
```

The return skips the whole subtree, and a child's cells are not always inside its
parent's rows. An absolutely positioned child is placed from its containing
block's origin: one with `top` past the parent's height sits below it. A flow
child of a box with an authored `height` overflows below it. Either can be in
view when the parent is not, and CSS paints it
([CSS 2 §11.1.1](https://www.w3.org/TR/CSS2/visufx.html#overflow): content is
clipped by `overflow` on an ancestor, not by its parent's box). LLP 1101 §4 admits
`position: absolute` in the terminal, "painted in order over the grid".

## The failure

A menu hung under the first row of a scroller, 20 by 6 screen:

```
column
  scroll height="4lh"
    column position="relative" height="1lh"
      text "anchor"
      text "menu" position="absolute" top="1lh" left="10ch"
    text "two"
    text "three"
    text "four"
    text "five"
    text "six"
  text "after"
```

At rest the second row reads `two       menu`. After the wheel scrolls one row,
the anchor is out of view and the menu's row is the scroller's first.

Observed: the first row reads `two`; the menu is gone. Expected: `two       menu`.

The same holds without a scroller, and for flow content. With
`column height="1lh"` holding `text "first"` and `text "second"`, one row above
an `overflow="hidden"` clip, `second` belongs on the clip's first row and nothing
is drawn.

## Verification

A temporary test in `host/terminal/tests/it/screen.rs` booted each app in
`Mode::Fullscreen` at 20 by 6, rendered into the `Vt` emulator, called
`host.wheel(2, 2, 1)` for the scroller and printed the text
(`cargo test -p exact-terminal --test it`). It printed the rows described, and
for a box moved one row above its clip with `margin-top="-1lh"` three empty rows
where the positioned child or the overflowing line belongs.

## Direction

Keep the early exit for a subtree that cannot reach the clip: test the extent of
the box's descendants, not its own rows, or do not return for a box that has a
positioned descendant or content taller than itself.

## Related

No issue or QUEUE entry names the terminal painter's clip.
