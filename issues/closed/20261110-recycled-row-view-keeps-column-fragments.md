# iOS: a list row's view parked with column fragments keeps them, so the next row's text is painted and hit through the old row's cut

**Status:** Closed
**Resolution:** Recycling a row clears its column fragments and column-rule layers before another row takes its views.
**Systems:** host/apple/Sources/ExactKit/IOS/NodePoolIOS.swift, host/apple/Sources/ExactKit/Columns.swift, host/apple/src/layout.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

References are at `dd6f5dcd`.

A box cut across columns keeps its pieces in `columnRecord`, which only a `fragments` op writes (`Columns.swift:56-60`). The host sends that op when a node's record changes and clears it only for an id that is still live (`host/apple/src/layout.rs:246-261`): a destroyed node gets no clearing op, and a new node that is whole in one column gets no op at all.

A virtualized list's retired row parks its views for the next row of the same shape. Neither `recycle` (`NodePoolIOS.swift:530-564`) nor `rebind` (`586-615`) clears `columnRecord`, and `recyclable` (`342-358`) does not refuse a view that holds one, as it refuses `flowShapes`, the other record a layout op writes. So a text that was cut in the retired row leaves its fragments to the text that takes its view.

A paragraph with a record is painted once per fragment, clipped to it (`eachFragment`, `Columns.swift:130-141`, from `NodeViewIOS.swift:1487`), is hit only inside a fragment (`NodeViewIOS.swift:906`) and never uses the text raster (`TextRasterIOS.swift:433`).

## The failure

```
list virtualized=true estimated-item-height=140 width=320 height=400
  for row in rows key=row
    view column-count=2 column-gap=20 width=320
      text "<a sentence of 164 characters>"
      text (row % 2 == 0 ? "A short note." : "<the sentence, three times>")
```

In an even row the first text is cut across both columns; in an odd row it is whole in the first. Scrolling retires even rows and builds odd ones.

Observed, by the path above: an odd row that takes an even row's views paints its first text through the even row's two fragments. The first clips it to that fragment's height and the second lies outside its box, so its last lines are not drawn, and a tap on them misses the text. Expected: the whole text, as a row built on new views shows.

## Verification

The host's half was run: a temporary test in `host/apple/tests/it` booted this list with the monospace measurer, scrolled it 6,000 points and read both batches (`cargo test -p exact-apple --test it`). It printed, for each kind of row:

```
boot: row 0 first text id 4: box 288x153.6, cut into 2 fragments, the first 134x153.6 holding lines 0..8, fragments op in this batch: true
boot: row 1 first text id 8: box 134x268.80002, whole in one column, fragments op in this batch: false
the scroll's batch: row 0 first text id 4: destroyed true, a fragments op clears it false
after the scroll: row 41 first text id 25: box 134x268.80002, whole in one column, fragments op in this batch: false
```

So a 268.8-point text in a view parked by row 0 would be clipped at 153.6 points: 8 of its 14 lines.

The pool's half is by reading: this machine cannot run the iOS simulator, and `NodePoolIOSTests` parks no fragmented view. A simulator run should scroll such a list and capture an odd row built after an even one retired.

## Direction

Clear `columnRecord` in `recycle`, where the paragraph and its layout caches are dropped.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names column fragments with the row pool.
