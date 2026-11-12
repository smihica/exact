# Every commit resolves every `rem`, `em`, text-style and hairline row again

**Status:** Closed
**Resolution:** Relative rows resolve only after style, ancestry, inherited font or host metric changes, in parent order.
**Systems:** kernel/src/txn/relative.rs, kernel/src/arena.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`relative::resolve` runs at the end of every batch (`kernel/src/txn.rs:1012`). After the
three host facts it handles (root font size, display scale, Bold Text), it visits every
slot in `arena.relative_slots` (`kernel/src/txn/relative.rs:119-185`): it walks to the
root for the slot's depth, sorts the slots, clones each slot's whole `StyleProps`
(`relative.rs:150`), resolves each relative row and only then finds that nothing changed
(`relative.rs:176`).

`relative_slots` is every node whose style writes a relative row
(`kernel/src/arena.rs:1066-1070`), not the nodes that need resolving. A relative row is a
`rem` or `em` length, a platform text style on `font-size` (`-exact-body`), a hairline or
a length ramp (`kernel/src/style/relative.rs:34-54`). The guide tells authors to size
with `rem` (`docs/contract-for-agents.md:3244-3247`), and the compiler writes text styles
and hairlines for grouped lists and menus. So a commit's cost grows with the tree,
whatever the commit changed, against LLP 1001 §5 ("Layout proportional to change"). LLP
1069.000 D3 describes the pass ("each commit resolves them, parents first") and does not
ask for that cost.

## The failure

A column of boxes, some with `padding-left` and `margin-bottom` in `rem`; then commits
that each set `testId` on the root. Release build, an M-series Mac:

| Nodes | With a `rem` row | Depth | One commit |
|---|---|---|---|
| 3,001 | 0 | 2 | 0.2 µs |
| 101 | 100 | 2 | 13 µs |
| 1,001 | 1,000 | 2 | 128 µs |
| 3,001 | 3,000 | 2 | 383 µs |
| 1,001 | 125 | 8 | 28 µs |
| 10,001 | 1,250 | 8 | 282 µs |
| 16,001 | 16,000 | 2 | 2.2 ms |

That is 0.13 µs for each node with a relative row at depth 2 and 0.23 µs at depth 8.
Shelf's library screen, as the Linux host builds it, holds 10 such nodes of 85: about
2 µs a commit. A page pays 0.2 ms a commit from about 1,000 of them, which a page sized
wholly in `rem` reaches.

Expected: near the first row. No relative row's inputs changed in any of these commits.

## Verification

The table is the output of a temporary kernel test run with
`cargo test --release -p exact-kernel --test it`. With a temporary early return in place
of the loop at `relative.rs:119`, the `rem` trees cost what the px trees do (0.3 µs a
commit at 1,000 to 64,000 boxes). Shelf's count is from a temporary test over
`apps/shelf` in `host/linux`. Only `rem` was timed; the other units take the same loop.

## Direction

Resolve a slot only when one of its inputs moved: every slot when the root font size or
the display scale changed (the blocks above the loop already know), otherwise the slots
this batch created, restyled or moved, and the relative slots under a node whose
`font-size` it changed. The batch already records the nodes it created and restyled
(`created`, `touched`), and `propagate_inherited` already walks the subtree of a changed
`font-size`.

## Related

`issues/20261110-engine-style-hash-omits-insets-and-grid-placement.md` is another cost
that grows with the tree, in the style interner. Nothing in `QUEUE.md` names this pass.
