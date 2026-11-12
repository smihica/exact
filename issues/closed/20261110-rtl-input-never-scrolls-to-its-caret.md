# A right-to-left single-line input never scrolls, so text typed past its width and the caret are not shown

**Status:** Closed
**Resolution:** Painted RTL inputs reveal their caret and map wheels, scrollBy and scrollbar gestures to the requested physical direction.
**Systems:** host/linux/src/paint/field.rs, host/linux/src/text/field.rs (painted fields: Linux, and Windows where the field is painted)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A single-line field's value is shaped unwrapped and placed by `line_left`
(`host/linux/src/text/field.rs:47` to `:65`). When the value is wider than the box and the
direction is right-to-left it returns `width - w`, a negative offset: the text's start is at
the right edge and the rest runs off to the left. That placement is meant
(`host/linux/tests/it/field_align.rs:88`).

The field's scroll only ever moves the text further left. `PaintedField::set_scroll` clamps
the offset to `0..=scroll_max` and subtracts it from the origin
(`host/linux/src/paint/field.rs:136` to `:144`), and `reveal_cursor` computes the caret's
position from that same origin (`field.rs:146` to `:164`). For a caret at the end of an
overflowing right-to-left value the position is negative, the wanted offset is negative, and
the clamp returns 0. Nothing can bring the left part into view.

## The failure

```
input testId="editor" value="אבגדהוזחטי אבגדהוזחטי …" (12 times) width=200 height=64 padding=4
  appearance="none" outline="none" font-size=16 caret-color="#ff0000" direction="rtl"
```

Focus it (the caret goes to the end), type one more letter, then press Home.

```
rtl focused, caret at the end (132): scroll (0, 0), caret frame None, caret pixels in the field 0
rtl after typing one more character: scroll (0, 0), caret frame None, caret pixels in the field 0
rtl after Home (0):                  scroll (0.84, 0), caret frame Some((203, 24, 1, 24))
```

The caret and everything typed after the box filled are outside the box, clipped. The same
input with `direction="ltr"` and a Latin value scrolls to 729.7 and shows the caret at x 203.

Expected: the field scrolls to keep the caret in view. Chromium 156, `<input dir="rtl">`
with the same value, 200px wide: after typing at the end `scrollLeft` is -859 of a 1067px
scroll width, and 0 after Home.

## Verification

A temporary test in `exact-linux`'s library booted the Contract with the CPU painter, drove
`set_focus` and `field_key`, and printed the field's scroll, `caret_frame()` and the count
of caret-coloured pixels: the lines above. A default-styled input (no `appearance`) gave
the same `scroll (0, 0), caret frame None` under the Android look and under the neutral
look. The Chromium values are `scrollLeft` and `scrollWidth` read after `keyboard.type` and
`Home`. Not run: a Windows build, where a default-styled field may be drawn by
`d3d11::fields` instead (`paint/field.rs:420` to `:491`). Android hosts the platform's
`EditText` and is not affected.

## Direction

Give the single-line field one scroll range in both directions: for a right-to-left
overflowing value, place the origin from the scroll offset measured from the right edge
(the origin moves right as the offset grows), and compute `reveal_cursor` in that same
frame.

## Related

QUEUE's "Linux host" entry among the declared gaps lists "RTL" from LLP 1015 §7 ("RTL
untested", of the v1 host's text); it does not name a field's scroll.
`host/linux/src/presenter/field_scroll.rs` tests the left-to-right case only
(`single_line_caret_scrolls_horizontally_and_returns_home`).
`issues/20261110-textarea-arrows-wrap-at-the-border-box-width.md` is the multi-line field's
wrap width.
