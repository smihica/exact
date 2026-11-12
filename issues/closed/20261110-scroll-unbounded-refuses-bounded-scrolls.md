# `lower-scroll-unbounded` refuses a `scroll` that a flex row, a grid, its own insets or its flex longhands bound

**Status:** Closed
**Resolution:** `lower-scroll-unbounded` reads the parent's literal `display` and `flex-direction` (a flex row, a grid or a computed value does not stack), and counts the scroll's flex longhands, both vertical insets when absolutely positioned, an `aspect-ratio` beside a width, and an unwritten `min-height` in a bounded column as bounds.
**Systems:** contract/lower/src/nodes.rs (the `lower-scroll-unbounded` check), contract/lower/src/values.rs (`flex_bounds`, `shrinking_scroll`)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

LLP 1017.000 (P1c) and the comment at `contract/lower/src/nodes.rs:49-51` make this refusal conservative: "Only the case nothing on the path can bound is refused; the parent that could have bounded it is bake's." The check (`nodes.rs:320-339`) decides that the parent stacks the scroll from the parent's tag alone:

```rust
let parent_stacks = !matches!(parent_tag, Some("row") | Some("canvas"));
```

and on the `scroll` reads only `height`, `max-height`, the `flex` shorthand, and `min-height=0` with a shrink. The LLP words the parent test the same way ("anything but a `row` or a `canvas`"), so the code follows its letter and breaks its stated intent. A `view display="flex"` is the flex row that `row` is (`contract/lower/src/tags.rs:77-80`), a grid stretches its item to the cell, an absolutely positioned box is sized by its insets, and `flex-grow` and `flex-basis` are what `flex` sets. Each is refused, with a message that is false for it.

## The failure

```
component A
  view
    view display="flex" height=300
      scroll testId="s" width=200
        view height=2000
```

Observed: ``4:7 [lower-scroll-unbounded] `scroll` has no `height`, `max-height`, or `flex`, and its parent stacks it top to bottom, so it will grow with its content and never scroll``.

Expected: it compiles, as it does with `row height=300` as the parent, and as the same box written `view overflow-y="scroll"` does.

Each form below is refused the same way. The height is what the kernel gives the scroll once `max-height=100000` is added to pass the check, and what Chrome 157 gives the same `<div>`s.

| Refused `scroll`, holding `view height=2000` | Height |
| --- | --- |
| `scroll width=200` in `view display="flex" height=300` | 300 |
| `scroll` in `view display="grid" grid-template-columns="200px 1fr" height=300` | 300 |
| `scroll position="absolute" inset=0` in `view position="relative" height=300` | 300 |
| `scroll position="absolute" top=0 bottom=0 width=200` in the same | 300 |
| `scroll flex-grow=1 flex-basis=0` in `column height=300` (`flex=1` compiles) | 300 |
| `scroll flex-basis=100` in `column height=300` | 100 |
| `scroll width=100 aspect-ratio="1"` in `column height=300` | 100 |

The compiler itself tells an author with a computed basis to write the flex longhands (`contract/lower/src/lib.rs:1309`).

## Verification

At `dd6f5dcd`, a temporary test in `contract/cli/tests` compiled each form (the refusal above), then compiled, baked, booted and laid out the same source with `max-height=100000` on the scroll, and with `view overflow-y="scroll"` in its place: both gave the heights in the table. A `scroll` in a plain `view height=300` came out 2000 tall, the case the refusal is for. Headless Chrome 157 laid out the same boxes as `<div>`s with `overflow-y: auto` and reported the same heights, each with `scrollHeight` 2000.

## Direction

Pass the check the parent's literal `display` and `flex-direction`, as `parent_bounded_column` is passed (`values::bounded_column` reads them), and treat a flex row or a grid as not stacking. On the scroll, read `flex-grow` and `flex-basis` as `flex_bounds` reads `flex`, and count `position="absolute"` with both vertical insets, and `aspect-ratio` beside a width, as bounds. Leave a computed `display` or position to the bake's measured `bake-scroll-unbounded`, and reword LLP 1017.000's sentence to match.

A `scroll` with no attributes in a `column` with a literal height compiles too (Claude, 2026-11-11; three models asked separately each chose it; no person has ruled). The kernel and Chrome both give it the column's height, because a scroll container's automatic minimum size is zero (CSS Flexbox 1 §4.5) and a flex item shrinks by default, so the refusal's message is false of it. The refusal stays for a column with no bound the compiler can see. `shrinking_scroll` takes a `scroll` that writes no `min-height` as it takes `min-height=0` (one that does not shrink still fails it), and a test of the bare form goes beside `a_zero_minimum_scroller_can_shrink_under_a_bounded_column` (`contract/cli/tests/it/styles.rs`).

## Related

- The sibling refusal `lower-zero-size` already counts an absolutely positioned box's insets (`contract/lower/src/controls.rs:1126-1136`).
- `lower-collection-unbounded` (`contract/lower/src/collection.rs:150-158`) reads the same spellings: `list virtualized=true flex-grow=1 flex-basis=0` in a `column height=400` is refused where `flex=1` compiles.
- `contract/cli/tests/it/collection_bounds.rs:217,241` and `contract/cli/tests/it/styles.rs:1243` assert the refusal only where nothing bounds the scroll. No issue, `QUEUE.md` entry or `docs/agent-pitfalls.md` entry names these forms.
