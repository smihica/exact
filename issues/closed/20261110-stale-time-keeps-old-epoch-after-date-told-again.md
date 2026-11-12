# A stale time an answer named stays at the old epoch when the host tells the date again

**Status:** Closed
**Resolution:** Live sources retain Unix stale times and use the current epoch for refreshes and host wake deadlines.
**Systems:** `runner/src/runner/modules/stale.rs`, `runner/src/runner/time.rs`
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A live module source's answer may name the Unix time at which it goes stale, and the
runner "asks the live source's readers again when the session's wall clock reaches it"
(LLP 1133.015 D7). `take_stale` converts that time to session milliseconds once, with
the epoch the runner holds then, and stores the result
(`runner/src/runner/modules/stale.rs:34-40`). `set_time` replaces the epoch
(`runner/src/runner/time.rs:47-80`) and leaves the stored times alone. After the date
is told again, the ask comes early or late by as much as the epoch moved.

A host tells the date again when the zone's offset moves, with the epoch it derives at
that moment: the wall clock less the session clock
(`host/apple/Sources/ExactKit/SessionTime.swift:23-41`). On Apple the session clock is
`CACurrentMediaTime()` (`host/apple/Sources/ExactKit/Session.swift:50-51`, `593`),
which is `mach_absolute_time` and "does not increment while the system is asleep"
([Apple](https://developer.apple.com/documentation/kernel/1462446-mach_absolute_time)).
So the epoch moves by the time the machine has slept, and a subscription's end that
was named before is asked that much late.

## The failure

`@exact/purchases`, bound under the agent to an implementation whose `listPurchases`
answers

```json
{ "exact:answer": [ … ], "exact:stale": 1790000060000 }
```

The date is first told as epoch 1790000000000 (journal: `purchases.listPurchases:
stale at 60000 ms`). Then `set_time(1790000030000, 60)`: the session's wall clock now
reaches the stale time at 30 000 ms.

```
session clock 31000 ms, wall clock 1790000061000: listPurchases read 1 time
session clock 59000 ms, wall clock 1790000089000: listPurchases read 1 time
session clock 61000 ms, wall clock 1790000091000: listPurchases read 2 times
```

Expected: the source is asked again at the advance that reaches 30 000 ms.

## Verification

A temporary test beside `a_stale_time_asks_again` in
`contract/cli/tests/it/purchases.rs`, with that file's harness, called
`Runner::set_time` after the first answer and advanced the clock in the steps above.
It printed those lines, and the journal's `t=61000 purchases.listPurchases: stale,
asked again`.

The hosts' part was read, not run.

## Direction

Keep the Unix time each answer named, and compare it with `epoch_at_zero + now_ms`
where `stale_due_ms` and `ask_stale` read it.

## Related

No issue or `QUEUE.md` entry names a stale time.
