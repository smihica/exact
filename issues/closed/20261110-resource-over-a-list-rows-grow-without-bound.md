# A resource over a list adds a resource row for every element that joins and never frees one

**Status:** Closed
**Resolution:** An element that joins takes the row of a member whose element left in an earlier commit and has no request in flight, starting it empty, so the rows stay as many as the elements one settlement holds (those that stay, leave and join).
**Systems:** runner/src/runner/each.rs, runner/src/runner/settlement.rs, runner/src/runner/commit.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Each element of `resource r = source(x) for x in list` is a member: a copy of the resource's row pushed onto the runner's `plan.resources` (`runner/src/runner/each.rs:133-152`, at `dd6f5dcd`), with twelve per-resource tables grown to match (`fit_resources`, `each.rs:156-173`). An element that leaves marks its member `gone` (`each.rs:204-222`) and the row stays: "A member's row is never reused" (`each.rs:26`). `join` matches only live members (`each.rs:101-105`), so a key that leaves and returns gets a new row each time.

Every commit then pays for every row there has ever been: the checkpoint clones the per-resource tables (`commit.rs:88-101`), `settle_pass` clones `self.resources` and visits each row in each pass (`settlement.rs:240`, `:359`), `sync_pending_flags` allocates a flag for each row (`in_flight.rs:11`), and `join` scans all members for each element (`each.rs:101`).

LLP 1017.000 P4c (amended 2026-11-04) says a leaving element "lets only its own request go" and declares no limit on rows. The JS target drops a leaving element's resource (`host/web-js/each.js:29-37`).

## The failure

```
shape Got
  phase: string
component App
  state ids: string[] = []
  state n = 0
  resource got = look(u) for u in ids as shape Got
  action show(xs: string[])
    ids = xs
  action bump()
    n = n + 1
  view
    column
      text got.map(g => g.phase).join(",")
      text `${n}`
```

`show` a window of 25 ids, then move it a page at a time: 25 elements leave, 25 join, 25 stay live. Each page change adds 25 rows. Observed, in an optimized development build (`--profile host-dev`):

| page changes | resource rows | a page change's commit | an unrelated `bump` commit |
|---|---|---|---|
| 0 | 26 | | 10 µs |
| 10 | 276 | 46 µs | 22 µs |
| 40 | 1,026 | 107 µs | 58 µs |
| 160 | 4,026 | 242 µs | 215 µs |

Showing two earlier pages again adds 50 more rows. A list filtered from 100 elements to 10 and back adds 90 rows each time. Alternating two lists of two elements 6,000 times (four keys in all) leaves 12,001 rows and 484 µs for each unrelated commit.

Expected: rows bounded by the elements live at once (26 here), and `bump` at 10 µs however long the session runs. A list that only grows is bounded already: 100 elements make 101 rows.

## Verification

A temporary test booted the app with `Runner::boot` over a source that answers `look` at once, read `runner.plan().resources.len()` after each step, and timed `bump` as the median of five runs of 200 commits. It printed the figures above; a run of the two-list toggle printed 2,001 rows after 1,000 toggles and 12,001 after 6,000. The JS target was read, not run.

## Direction

Have `add_member` take a `gone` row of the same family before pushing a new one, clearing that row's tables, so rows are bounded by the most elements live at once. A gone member's request is already let go (`let_go_members`), so no ticket names its row.

## Related

No issue or QUEUE entry names the members or their rows. Two other defects come from members being rows of the plan, each with its own fix: `state` repeats a list's name for each member row, so in a development host `state.resourceStatus` grows with these rows (4,101 entries and 1 MB of `state` after 162 page changes); and a write record over a list never retires.
