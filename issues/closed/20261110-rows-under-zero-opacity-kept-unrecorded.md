# A scroller's rows first walked under `opacity: 0` are kept as recorded though no backend recorded them, and stay blank on Android

**Status:** Closed
**Resolution:** Rows walked under zero opacity keep no unrecorded row and are recorded before replay when shown.
**Systems:** host/linux/src/paint.rs, host/linux/src/paint/rows.rs, host/linux/src/canvas.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A node whose presented opacity is 0 is walked with the painter's backend swapped for
`layer::Unpainted`, so its subtree's boxes are recorded and nothing is drawn
(`host/linux/src/paint.rs:1059`). Whether rows are kept this walk was decided earlier,
from the real backend (`paint/rows.rs:258`, `self.rows.active`). So a scroller inside
the transparent subtree still sends each child through `Painter::row`
(`rows.rs:529`), which asks `Unpainted` to record it: `row_begin` does nothing,
`row_end` returns the trait's default 0 (`paint/backend.rs:310`), and the row is
stored in `rows.kept` with id 0 (`rows.rs:690`).

When the opacity becomes positive the rows are not stale: the commit touches the faded
node, and `rows_dirty` marks only a row that contains a changed node (`rows.rs:206`),
not rows below it. Each is then replayed with `backend.row_draw(0, ..)`. The GPU and
Ink backends refuse an id they do not hold in `row_reusable` and record again. The
Canvas backend, Android's, does not override `row_reusable` (default `true`,
`backend.rs:301`) and writes `ROW_DRAW 0` (`canvas.rs:893`); its player draws nothing
for a row it has no node for
(`host/android/canvas/src/main/java/dev/exact/canvas/Player.kt:859`, `row(id)?.let`).

## The failure

```
component App
  state shown = false
  action show
    shown = true
  view
    column
      button "Show" press=show testId="show" height=40
      box testId="fade" opacity=(shown ? 1 : 0) width=300 height=300
        scroll width=200 height=200
          column
            box height=50 width=200 background-color="#2468ac"
            box height=50 width=200 background-color="#2468ac"
            box height=50 width=200 background-color="#2468ac"
```

The row ops in each stream the Canvas recorder finished:

```
first frame, opacity 0       ROW_BEGIN []          ROW_DRAW []
after the press, opacity 1   ROW_BEGIN []          ROW_DRAW [0, 0, 0]
the frame after              ROW_BEGIN []          ROW_DRAW [0, 0, 0]
```

With `transition="opacity 200ms linear"` on the box the three `ROW_DRAW 0` are in the
fading layer's drawing, and in every frame after the transition ends. With
`opacity=1` from the start the first frame holds `ROW_BEGIN [1, 2, 3]` and
`ROW_DRAW [1, 2, 3]`.

Expected after the press: three rows defined, then drawn by their own ids. No stream
ever defines them, so on Android the list is blank until something in a row changes,
the row is unmounted, or the viewport's width, the scale or the scheme changes.

## Verification

A temporary unit test in `host/linux/src/canvas/` booted the Contract above, replaced
the painter's backend with `canvas::Recorder`, painted, pressed the button with
`Presenter::tap`, painted again, and walked each finished stream with `stream::end`,
listing `ROW_BEGIN` and `ROW_DRAW` ids; the output above is what it printed, with
`EXACT_PAINTER=canvas` set (the reader plays the opacity, as on Android) and without.
The Kotlin player was read, not run: no Android device was driven.

## Direction

Keep no row while the backend in hand keeps none: in `Painter::row`, walk the child
with `self.node` when `!self.backend.rows()`, which is `Unpainted`'s answer.

## Related

`host/linux/src/presenter/kept_rows_tests.rs` covers a renewed row and a new reader,
not a walk that draws nothing. Nothing in `issues/`, `QUEUE.md` or
`docs/agent-pitfalls.md` covers it.
