# Every commit decodes and validates the whole router value three times, whether or not it navigated

**Status:** Closed
**Resolution:** Commits that retain the committed router object without fragment navigation skip decoding and validation, preserving pending changes and Forward history.
**Systems:** runner/src/runner/router.rs, runner/src/runner/apply.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`Runner::apply` ends every commit with `commit_router` (`runner/src/runner/apply.rs:61`, at `dd6f5dcd`), which calls `RouterContext::commit` (`runner/src/runner/router.rs:717-781`). When the router slot holds the value already committed, `change()` has returned `None` on an identity check (`router.rs:655-661`), but `commit` still calls `self.router(…)` three times:

- `router.rs:722`, to look for a navigation to the fragment the top already shows, though no verb ran;
- `router.rs:740`, the value after, for `history::step`;
- `router.rs:741`, the value before, for the same call.

Each call decodes every tab and visit and checks every visit against the route table (`router.rs:279-328`: canonical URL, a pattern match, parameter equality). So a timer tick, a keystroke, a scroll report or a reply pays for every visit on every tab's stack, three times over, to learn that nothing navigated.

## The failure

Microseconds for each commit in an optimized development build (`--profile host-dev`), as built and with the three reads skipped when nothing navigated. The first two rows are the builder plan of `runner/tests/it/support/router.rs`, which has a router and no view, running `encode`, an action that writes one unrelated string slot. The others are a compiled app with routes `/` and `/t/:thread` whose visits are columns under `for e in stack(nav)`: an action that changes a counter each visit shows, and one that changes a string 60 rows on the home visit show.

| commit | stack depth 1 | depth 4 | depth 16 |
|---|---|---|---|
| no view, 2 routes | 3.0, skipped 1.4 | 9.9, skipped 1.4 | 36.7, skipped 1.5 |
| no view, 10 routes | 3.7, skipped 1.4 | 11.0, skipped 1.3 | 41.1, skipped 1.3 |
| one text changes in each visit | 2.9, skipped 1.5 | 9.7, skipped 2.3 | |
| a keystroke changes the text of 60 rows | 16.8, skipped 15.1 | 23.1, skipped 15.4 | |

The reads cost about 2 µs for each visit on a stack. At depth four that is three quarters of a small commit and a third of one that changes 60 rows. Expected: a commit that leaves the router value as it was costs the same at any depth.

No budget is at stake: `rules/RULES.md` and `scripts/metrics.mjs` budget milliseconds and seconds.

## Verification

Temporary counters in `RouterContext::router` and `RouterContext::commit` showed one router commit and three decodes for an action that leaves the router alone, in both the builder plan (100 such actions: 100 and 300) and the compiled app; a push made six decodes. The timings are medians over runs of the same action with and without this guard at the top of `RouterContext::commit`, switched at run time. With the guard on, a push and a `back` still moved the stack and what Forward offers.

```rust
if change.is_none()
    && self.navigations.borrow().is_empty()
    && (self.committed.as_ref()).is_some_and(|c| crate::compare::same(c, &value))
{
    return;
}
```

The router's 20 integration tests (`runner/tests/it/router.rs`) pass with the guard on.

## Direction

Return early from `commit` as above: with the same value and no verb run there is no change to publish, no activation, and `history::step` would find the same visits. Where the value did change, `change()` has already read the new router; pass it in instead of reading it twice more.

## Related

No issue or QUEUE entry. `QUEUE.md` has no router performance entry.
