# `Storage` hands out a kept reply's continuation under a token it cannot dispatch

**Status:** Closed
**Resolution:** `Storage` gives a delivery's continuation a token of its own, which `dispatch`, `release`, `discard`, `continuation` and `host_continuation` map back to the child's and `forgotten` keeps, since the runner holds it as module work rather than in flight.
**Systems:** data/host/src/lib.rs (`exact_data_host::Storage`), kept replies (LLP 1146 D2), ordered sets (LLP 1027.002 D3)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`Storage` renumbers every continuation its source hands out: `step` and `timer_before_reply` store the child's token in `pending` and give the runner one of `Storage`'s own (`data/host/src/lib.rs:140`, `:432`), and `dispatch` and `release` look tokens up in `pending` (`:341`, `:364`). All lines are at `dd6f5dcd`.

`module_delivery` does not (`:421`). It returns the child's request as it is, so the continuation carries the child's token. A worker-placed `Placed` and a `Mixed` whose TypeScript child is in an ordered set both answer a kept reply with such a continuation (`data/src/placed.rs:1229`, `data/src/mixed.rs:1121`), and the runner sends it out and dispatches it on the app's data (`runner/src/runner/module_work.rs:237`).

`Storage::dispatch` then finds no entry and answers `Missing`, so the delivery never runs and the reply is lost. Or it finds another request's entry under the same number (both count from 1) and runs that request's work as the delivery: the other request's value is discarded and its own dispatch is then `Missing`.

No app in `apps/` reaches it today. Fieldnotes is the one app whose data is `Storage` over a `Mixed` with an ordered TypeScript child, and it sends no request with effects.

## The failure

`Leaf` is a source that counts `module_delivery`; `javascript.placement()` is `Worker`, which orders it.

```rust
let mut storage = Storage::new(Mixed::new(javascript, Leaf::default(), &["a"], &["b"]).unwrap());
storage.activate().unwrap();
let request = storage.module_delivery(&mut store, 7, reply()).unwrap().unwrap();
storage.dispatch(request.continuation.unwrap(), &store)
```

Observed: `Dispatch::Missing`. Expected: `Dispatch::Run`, and the leaf's count at 1.

## Verification

Ran that in `data/host/src` (`cargo test -p exact-data-host --lib`): `Missing`. With one ordinary answer waiting on a child continuation 40 (`Storage`'s token 1), a delivery that yields the child's token 1 dispatched child token 40.

Ran it through a `Runner` and the Hermes module too: `js/tests/it/timers.rs`'s fixture given `secret.keep shared`, on the main thread beside a worker-placed Rust child that keeps `shared`, two `post` sends, the first let go and kept. The test's pump fails a `Missing` continuation as the Apple executor does (`host/apple/src/executor_core.rs:1349`).

- The `Mixed` as the runner's data: the module's log reads `posted 1,posted 2`, whether each reply is flushed on its own or both arrive before one flush.
- `Storage` over it, each reply flushed on its own: the log reads `posted 2`, and the journal says `module work 6: unsupported: missing or consumed native continuation`.
- `Storage` over it, both replies before one flush: the log reads `posted 2` and `waited` stays unset. The journal says `module work 6: HTTP 200, 81 bytes` (the second POST's turn, run as the delivery), then `request 7 (waited) failed and is no longer pending: … missing or consumed native continuation`.

## Direction

Renumber the continuation in `Storage::module_delivery` as `timer_before_reply` does, so `dispatch` and `release` find it.

## Related

`issues/20261110-kept-reply-dropped-by-generated-entry.md` is the same reply lost earlier, in `Placed` and `exact-logic`; this one remains once that is fixed. `issues/closed/20261102-mixed-disjoint-workers-serialized.md` is about which sets `Mixed` makes.
