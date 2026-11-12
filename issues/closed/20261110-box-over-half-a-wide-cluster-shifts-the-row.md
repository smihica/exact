# A box painted over half of a wide cluster shifts the rest of the row by a column

**Status:** Closed
**Resolution:** Painting over any part of a wide cluster clears the whole cluster, preserves uncovered backgrounds, and keeps subsequent cells aligned.
**Systems:** host/terminal (grid.rs, paint.rs)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A wide cluster occupies two grid cells: its lead holds the text and the next cell is an empty continuation (`host/terminal/src/grid.rs:42`, `:170-186`, at `dd6f5dcd`). `Grid::fill` (`grid.rs:153-166`) and `Grid::put` write the cells they are given and never look at what was there. When a later box covers only one of the two cells, the other half survives:

- the continuation is overwritten and the lead stays: the terminal still draws the cluster two columns wide, so everything after it in the run lands one column to the right;
- the lead is overwritten and the continuation stays: the writer emits nothing for that cell, so everything after it lands one column to the left.

The writers emit a run's cells one after another (`grid.rs:263-289`, `:315-355`, `:360-406`), so the grid and the screen disagree from that cell to the end of the run. Anything painted over earlier content can do it: an open dialog or popover over the page (`host/terminal/src/paint.rs:89-101`), an absolutely positioned box. CJK text and emoji are the wide clusters.

## The failure

Full screen, 24 columns. The nine-column dialog is centred, so its left border is column 7, the second cell of a character:

```contract
component App
  view
    column width="100%" height="100%"
      text "日本語日本語日本語日本語"
      text "日本語日本語日本語日本語"
      text "日本語日本語日本語日本語"
      button commandfor="d" command="show-modal" testId="open"
        text "open"
      dialog id="d" border="thin solid" width="7ch" background-color="#202020"
        text "hello"
```

Rows 1 and 2 after pressing `open`, cell by cell (`·` is a wide cluster's second cell):

```
grid  |日·本·語·日┌───────┐語·日·本·語·|
screen|日·本·語·日·┌───────┐語·日·本· |
grid  |日·本·語·日│hello  │語·日·本·語·|
screen|日·本·語·日·│hello  │語·日·本· |
```

On the screen the dialog's top two rows start at column 8 and its bottom border at column 7, and the row's last character no longer fits. Expected: the screen shows the grid, with the half-covered character blanked, as a terminal does when one cell of a wide character is overwritten. Closing the dialog repaints the rows correctly.

The other half: in 28 columns, with the row `abcdefghijklmnopq🚀 shipped` under the same dialog's middle row, the right border (column 17) covers the emoji's lead, and ` shipped` is drawn from column 18 where the grid has it from column 19.

## Verification

A temporary test compiled each source with `contract::compile_terminal`, booted it full screen, wrote each frame's `grid.diff` against the frame before into a `vt100` parser (the writer's oracle, LLP 1101 §5) and printed both, cell by cell: the rows above. A `column position="absolute" left="3ch" top="0" width="4ch" height="1lh" background-color="#202020"` over a row of the same text differed from the screen at four columns in the first frame. No real terminal was run.

## Direction

In `Grid::put` and `Grid::fill`, before writing a cell, repair its neighbour: writing over a continuation turns its lead into a space, and writing over a lead turns the continuation after it into a space, each keeping its background.

## Related

`a_wide_cluster_takes_two_cells_and_the_diff_redraws_from_its_lead` (`grid.rs:656`) covers the diff for an intact pair only. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
