# A table's rows past the 10,000th are laid out on top of the 10,000th

**Status:** Closed
**Resolution:** A table's grid raises the engine's highest grid line to `i16::MAX` (Taffy patch 70), so each of its rows up to that bound has a track of its own, while an authored grid keeps the 10,000-track limit and its refusal.
**Systems:** kernel/src/layout/table.rs, vendor/taffy (grid placement)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A native host lays a table out as one grid (LLP 1153 D7.3). `table::place` gives each
row, row group and cell a grid row line counted from the table's first row
(`kernel/src/layout/table.rs:249-255`), bounded only by `i16::MAX`. The vendored
engine clamps a placement into a grid of 10,000 tracks (`clamp_span_to_limited_grid`,
`vendor/taffy/src/compute/grid/placement.rs:349-353`; `MAX_GRID_TRACKS`,
`vendor/taffy/src/compute/grid/types/coordinates.rs:13`). So every part whose row is
past the 10,000th is placed in the last track.

Neither a clamp nor a refusal is specified for a table. For an authored grid LLP 1001
chooses refusal: a line or span above 10,000 is a compile error, "refused rather than
clamped" (§1 and §4), because Chrome lays such a grid out and Taffy would lay it out
differently. A table's lines are computed by the kernel, so nothing refuses them.
LLP 1153 D7.3 and `kernel/src/style/table.rs` declare one deviation for tables, how
width is shared among columns, and no row limit.

## The failure

```contract
component App
  view
    table
      for i in range(10010) key = i
        tr testId=`r${i}`
          td
            view width=20 height=10
```

Rows are 12 tall at the UA sheet's 2px spacing and 1px cell padding.

| row (from 0) | kernel y | Chrome 156 y |
|---|---|---|
| 9,998 | 139,974 | 139,974 |
| 9,999 | 139,988 | 139,988 |
| 10,000 | 139,988 | 140,002 |
| 10,001 | 139,988 | 140,016 |
| 10,009 | 139,988 | 140,128 |

The kernel's table is 140,002 tall and Chrome's 140,142: the last ten rows are drawn
over row 9,999 and take no height. Nothing is journalled.

## Verification

A temporary test compiled the Contract above, booted it in a `Runner` on the
monospace kernel, laid it out and printed the kernel column of the table; the same
tree built through `Kernel::apply` gives the same frames. Chrome's column is headless
Chrome for Testing 156.0.8078.4 on a `table` of 10,010 `tr > td > div` rows, measured
with `getBoundingClientRect`. Not run: a native host showing the table. The web hosts
are unaffected, since the browser lays their tables out.

## Direction

Either state the limit where the table deviation is declared and journal a table that
passes it, once per table, as a multi-column refusal is journalled (LLP 1093 D10); or
lay a long table out as a stack of grids of at most 10,000 rows that share the column
widths. The first is the smaller change and matches how an authored grid past the
limit is handled.

## Related

The first layout of a table this long takes seconds for a separate reason, the engine
style interner's hash. `issues/closed/20261016-markdown-on-macos-table-columns-do.md`
is about Markdown table columns, not this grid.
