# A mutation that `refreshes` a resource over a list leaves a write record for every send, and its overlay is never asked

**Status:** Closed
**Resolution:** List members receive overlays and retire landed writes independently after refresh or removal in Rust and JS.
**Systems:** runner/src/runner/writes.rs, runner/src/runner/settlement.rs, runner/src/runner/each.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A send to a mutation that declares `refreshes` makes a write record (`runner/src/runner/writes.rs:88-105`, at `dd6f5dcd`). When its reply lands, the record keeps the resources it still shows in (`land_write`, `writes.rs:145-168`), and `Runner::shown` retires it from each once that resource has an answer asked after the landing (`writes.rs:328-342`).

A record names the rows the mutation declares, so for a resource over a list it names the list's row. That row takes its value from its members and never calls `shown` (`settlement.rs:383-415`). A member does call it (`settlement.rs:825`, `:1050`), with its own index, and `writes_for` finds no record under that index (`writes.rs:221-230`). So for `mutation m … refreshes r` with `r` over a list:

1. Each landed send leaves a record in `self.writes.records` for the rest of the session. Every later checkpoint clones them (`commit.rs:98`) and `state.writes` lists them all.
2. The source's `overlay` is never called for `r` or its elements, so a write never shows before its reply. `docs/contract-for-agents.md:903` gives `refreshes` with `overlay` for that, and LLP 1017.000 P4c (amended 2026-11-04) says a member's "request, reply, failure, refresh and overlay are an ordinary resource's".

Asking every element again when the reply lands does work (`docs/contract-grammar.md:540`).

## The failure

```
shape Got
  phase: string
shape Done
  ok: bool
component App
  state urls: string[] = ["a", "b"]
  resource got = look(u) for u in urls as shape Got
  resource one = look("z") as shape Got
  mutation saved as shape Done refreshes got, one
  action save
    send saved = save()
  view
    text got.map(g => g.phase).join(",")
```

`look` answers at once and `save` later. Run `save` and land its reply three times. Observed after the third reply:

```
"writes":[{"id":4,"mutation":"saved","landed":true},{"id":10,"mutation":"saved","landed":true},{"id":16,"mutation":"saved","landed":true}]
```

and across each send the overlay was asked twice for `one` (argument `z`) and never for `a` or `b`.

Expected: `writes` is empty once the answers asked after a reply are in, and the overlay is asked for each element while a write shows.

## Verification

A temporary test booted the app with `Runner::boot` over a source that counts its `overlay` calls by argument, and printed `Runner::writes()` and the agent's `state` after each reply: the lines above. A second run, 50 sends to a mutation that refreshes only the list, beside the same app with a plain `resource got = look("a")`, printed:

```
plain resource: 0 write records after 50 landed sends (50 re-asks, 100 overlay calls)
resource over a list: 50 write records after 50 landed sends (100 re-asks, 0 overlay calls)
```

The JS target was read, not run: a mutation's `refreshes` there hold the list's own object (`host/web-js/rt.js:555`, `each.js:47`) and `overlay.js:42` looks for the element's, so it has the same mismatch.

## Direction

Match a record that names a list against that list's members in `writes_for`, so each element's answer is overlaid and retires the write as an ordinary resource's does, and drop the record once no live member shows it. If an overlay over a list is not meant, leave the list out of `showing` in `land_write` and say so in `docs/contract-grammar.md`.

## Related

No issue or QUEUE entry covers writes over a resource over a list. The unbounded member rows and `state` naming a list once for each member are separate defects of the same feature; fixing them does not retire these records.
