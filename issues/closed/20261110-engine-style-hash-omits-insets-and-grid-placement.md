# Styling `n` boxes that differ only by an inset, a maximum size or a grid placement takes `n²` style comparisons

**Status:** Closed
**Resolution:** Engine style hashes include insets, maximum sizes, grid placements, all margins and paddings, and both gaps, avoiding quadratic comparisons when these values differ.
**Systems:** kernel/src/shared_style.rs, kernel/src/layout.rs, kernel/src/layout/table.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

Every engine style is looked up in an interner before it is kept (`LayoutTree::share`,
`kernel/src/layout.rs:431-437`). The lookup hashes the style with `engine_hash`, which
reads a fixed set of rows: size, minimum size, flex basis, the left and top margin and
padding, the column gap, flex grow and shrink, display, position, flex direction and
box sizing (`kernel/src/shared_style.rs:425-449`). Styles equal in those rows share
one bucket, and `Interner::get` compares the new style with each entry of the bucket
in turn, with `taffy::Style`'s full equality (`shared_style.rs:53-56`).

The four insets, the maximum size and the grid row and column are not hashed. So `n`
boxes that differ only in one of them fill one bucket, and deriving their styles takes
`n²/2` comparisons. Three trees do this: boxes of one size placed by `top` or `left`
(the marks of a chart), grid cells placed by `grid-row` and `grid-column`, and a
table, whose every row and cell gets its own grid line
(`kernel/src/layout/table.rs:249-266`).

## The failure

```contract
component App
  view
    view position="relative" width=400 height=400
      for i in range(10000) key = i
        view position="absolute" left=0 width=10 height=10 top=i
```

In a release build on an M-series Mac, `Runner::boot` of this takes 286 ms, and 23 ms
with `top=0 width=i` in its place. A `display="grid"` box of 5,000 cells that each say
their own `grid-row` boots in 186 ms, and in 8 ms when the cells differ by `width`.
At the kernel, one batch that mounts `n` absolutely positioned boxes:

| differing row | 2,500 | 5,000 | 10,000 | 20,000 |
|---|---|---|---|---|
| `width` (hashed) | 2.8 ms | 6.2 ms | 14.2 ms | 32.8 ms |
| `top` (not hashed) | 17.9 ms | 69.9 ms | 287 ms | 1.31 s |
| `max-width` (not hashed) | 18.2 ms | 72.6 ms | 290 ms | 1.23 s |

Writing a new `top` on each of 2,500 mounted boxes in one batch takes 15.5 ms where a
new `width` takes 1.4 ms. A table of one-cell rows lays out the first time in 35 ms at
1,250 rows, 142 ms at 2,500, 626 ms at 5,000 and 3.24 s at 10,000. Expected: each
linear in `n`.

## Verification

Temporary tests built with `--release` timed `Kernel::apply` of the batches in the
table, the first `compute_layout` of a `table > tr > td > div` tree, and
`Runner::boot` of the compiled Contract above and of its grid counterpart. With the
insets, the maximum size and the grid row and column fed to `engine_hash` (a temporary
edit, reverted), the same runs are linear: 20,000 boxes with distinct `top` mount in
27 ms, restyling 2,500 takes 1.0 ms, and the 10,000-row table lays out in 54 ms.
Not run: a host.

## Direction

Feed the unhashed rows that boxes commonly differ by into `engine_hash`: the insets,
the grid row and column, the maximum size and the other two margins and paddings.
Equality still decides, so a wider hash changes no result.

## Related

`issues/closed/20261017-memo-recorder-repeats-quadratic.md` fixed a different
quadratic pass, the row memo's recorder. No issue or QUEUE entry names the interner.
