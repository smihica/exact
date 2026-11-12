# Layout panics after `Kernel::trim` when a destroyed `-exact-fit: shrink` text has not been laid out since

**Status:** Closed
**Resolution:** Destroyed fitted text leaves the fitting set, and layout checks slot liveness before reading arena columns after trim.
**Systems:** kernel/src/layout/fit.rs, kernel/src/layout.rs, kernel/src/arena.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The engine tree keeps the slots of the fitted paragraphs it has measured in
`LayoutTree::fitting` (`kernel/src/layout.rs:187-188`, filled at `:1347`).
`LayoutTree::remove` drops a removed node's slot from `flowing`, `displacements` and
the table records (`layout.rs:505-527`) but not from `fitting`. The set is cleaned
only by `fit::settle`, at the end of the next published layout, which starts by
reading `arena.taffy(slot)` for every slot in it and tests liveness after
(`kernel/src/layout/fit.rs:126-130`).

`NodeArena::taffy` indexes the column directly (`kernel/src/arena.rs:481-483`), and
`NodeArena::trim` truncates the columns to the last live slot (`arena.rs:327-361`).
So when a fitted text is destroyed and the kernel is trimmed before the next layout,
that layout indexes past the column's end. `Kernel::trim` says "nothing a reader sees
changes" (`kernel/src/kernel/trim.rs:7-10`), and LLP 1001 §5 says an engine fault is
never a panic.

This is a hazard of the kernel's API, not a path a host takes today. `Kernel::trim`
has two callers, iOS when scrolling has rested
(`host/apple/Sources/ExactKit/SessionRest.swift:9`) and the terminal host after a
commit (`host/terminal/src/host.rs:391`), and both lay out in the call that commits,
which empties the set first. A layout that fails inside the engine rebuilds the engine
tree (`kernel/src/kernel.rs:685-703`), which empties it too. The order needs a layout
that never reached the engine: on Apple, one refused by a height sync
(`host/apple/src/host.rs:1363-1365`, `host/apple/src/layout.rs:152-154`), with the
scroll coming to rest before the next commit.

## The failure

```rust
// 1: a 100px-wide root; 2..=299: views; 300: a text with
// `-exact-fit: shrink; white-space: nowrap` and a line wider than 100px.
k.compute_layout(1, offer).unwrap();           // fit scale 0.5, 300 slots
k.apply(0, 2, &destroy_2_to_300).unwrap();
k.trim();                                      // 1 slot
k.compute_layout(1, offer);
```

Observed: `panicked at kernel/src/arena.rs:482:19: index out of bounds: the len is 1
but the index is 299`. Expected: the layout succeeds, as it does without the `trim`
or with a layout between the destroy and the trim.

## Verification

A temporary kernel test ran the three orders. Without a trim and with a layout before
the trim, `compute_layout` returned `Ok`; with the trim first it printed the panic
above. The callers of `trim` and the hosts' layout calls were traced by reading; no
host was driven into the order.

## Direction

Remove the slot from `fitting` in `LayoutTree::remove`, beside `flowing`, and have
`fit::settle` test liveness before it reads a column.

## Related

`issues/closed/20261102-kernel-reset-trim-retains-dirty-slots.md` fixed the same
class of read past a trimmed column for `layout_dirty` after a reset, also with no
production caller established. No issue or QUEUE entry mentions `fitting`.
