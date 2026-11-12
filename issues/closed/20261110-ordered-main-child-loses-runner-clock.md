# A main-thread child ordered behind a worker runs its turns with the clock at zero, so its timers fire at once

**Status:** Closed
**Resolution:** Ordered main-child turns and kept replies preserve the runner clock in their snapshot stores.
**Systems:** data/src/mixed.rs, ordered sets (LLP 1027.002 D3), answer timers (LLP 1027.000 D1)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

The runner stamps its clock on the store before it dispatches work (`runner/src/runner/commit.rs:997`), and a source reads it as `Store::clock_ms`. The TypeScript executor arms `setTimeout` and `AbortSignal.timeout` against it: a timer is due at `clock + delay` (`js/src/door.rs:241`, `js/src/binary.js:362`). All lines are at `dd6f5dcd`.

`Placed` carries the clock to its owner thread (`data/src/placed.rs:1012`, `:540`). `Mixed` does not. A child on the main thread that shares a `secret.keep` name with a worker child is a member of the worker's ordered set, and `Mixed::start` runs its turn against a new store built from a snapshot (`data/src/mixed.rs:398`), whose clock is zero. So a timer armed in such a turn is due at `delay` on the runner's clock, which has usually passed.

Only the TypeScript executor reads the clock, so the shape that shows it is a TypeScript module on the main thread ordered behind a worker-placed Rust source. No app in `apps/` has it: Fieldnotes, the one app with a worker, places its TypeScript there.

## The failure

`js/tests/it/timers.rs`'s fixture, its grants given `secret.keep shared`, on the main thread beside a worker-placed Rust child that keeps `shared` too, with the runner's clock at 10 000:

```rust
let mixed = Mixed::new(module().placed(Placement::Main),
    Placed::new(Keeper, Placement::Worker), &["slept", "log", "spin"], &[]).unwrap();
// boot under Storage, advance the clock to 10 000
r.act("sleep", vec![Value::str("a"), Value::Number(450.)]).unwrap();   // await sleep(450)
// pump, without moving the clock
```

Observed: `result` is already `a after 450` and `data_timer_due_ms()` is `None`. Expected: `result` unset and the timer due at 10 450, which the same two children give when both are on the main thread.

## Verification

Ran both with `cargo test -p exact-js --test it`. With the Rust child on the main thread the drive prints `data timer due: Some(10450.0), now 10000` and the result arrives at 10 450. With it on a worker the drive prints `data timer due: None, now 10000` and the result is set before the clock moves. With stand-in sources in `data/src` (`cargo test -p exact-data --lib`), an ordered main child reads `clock_ms()` as `0.0` where the runner set `5000.0`; an unordered child and a worker-placed one read `5000.0`.

## Direction

Copy `store.clock_ms()` onto the store `Mixed::start` builds, as `Placed` does for its jobs. `Mixed::start_delivery` (`data/src/mixed.rs:430`) and `Placed`'s kept-reply turn (`data/src/placed.rs:492`) build their stores the same way and can take it too.

## Related

`issues/20261110-mixed-worker-child-timer-wait-fails.md` is the worker child's timer; this is the main child's clock. `issues/closed/20261009-typescript-failure-parity.md` added answer timers. No issue or QUEUE entry names `clock_ms`.
