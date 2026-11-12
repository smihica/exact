# The runner journals a persisted state over its size limit again at every commit

**Status:** Closed
**Resolution:** Unchanged oversized persisted values are reported once per slot while the store retains its last accepted value.
**Systems:** `runner/src/runner/persist.rs`
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`persist_slots` runs after every commit that stands. For each persisted slot it encodes
the value and skips the slot when the store already holds that text
(`runner/src/runner/persist.rs:61-67`). A value over `PERSIST_LIMIT` (16 KiB) is never
stored, so that comparison never succeeds, and the "not kept" line (`persist.rs:68-76`)
is written again at every later commit, whatever that commit changed.

LLP 1116 D5 has the runner act "after each commit that changes a persisted state". The
web JS target does: it remembers the oversized text as the last one seen and says so
once for each value (`host/web-js/persist.js:60`).

The journal is a ring of 4096 lines (`JOURNAL_RING`, `runner/src/runner.rs:575`). An app
that holds one oversized persisted value spends about every second line of `logs` on
it, so the lines a person or an agent reads to debug are pushed out twice as fast.

## The failure

```
component App
  state big = "" persist
  state n = 0
  action set(v: string)
    big = v
  action bump
    n = n + 1
  view
    text `${n}`
```

`act set` with a string of 17,000 characters, then `act bump` 50 times.

Observed on the runner: 51 of the journal's 104 lines read

```
persisted state big: 17002 bytes is over the 16384 a persisted value may hold: not kept (the store keeps its last value)
```

Expected: one line, when the value first goes over the limit.

## Verification

- The runner: a temporary test in `runner/tests/it/` booted the app above with
  `Runner::boot_stored`, ran the two actions and counted the journal lines that hold
  `persisted state big`. It printed 1 line after `set` and "51 line(s) of 104 in the
  journal" after the 50 commits.
- The web JS target: an app with the same two states, whose `big` an action doubles to
  16,384 characters, built with `bun host/web-js/build.mjs` and driven in headless
  Chrome through `scripts/agent.mjs` (ten taps to grow `big`, then 50 taps of `bump`).
  `logs` held 1 such line of 12, before and after the 50 commits.

## Direction

Remember for each slot the oversized text last reported, and skip the slot while its
value is unchanged, as `persist.js` does.

## Related

`QUEUE.md`, "A prerendered web page shows a persisted state's declared value until its
module boots" is about first paint. `docs/contract-for-agents.md:579` documents the
limit and that `logs` says so; it does not say how often.
