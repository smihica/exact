# An answer that awaits `setTimeout` fails in the worker-placed child of a `Mixed` source

**Status:** Closed
**Resolution:** Mixed dispatches a worker child's timer continuation as its next ordered turn and parses its reply once.
**Systems:** data/src/mixed.rs, data/src/placed.rs, ordered sets (LLP 1027.002 D3), answer timers (LLP 1027.000 D1)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

An answer that awaits its own timer yields a continuation with `due_ms` and is resumed when the runner's clock reaches it (`js/src/lib.rs:207`). A worker-placed `Placed` handles this itself: on the turn's reply it records the resume at once and returns that recorded turn's token with the due time (`data/src/placed.rs:408`). The next reply under that token is again a turn's envelope. All lines are at `dd6f5dcd`.

`Mixed` treats every `Later` from an ordered child's turn as a request the host runs and stages the call as `Yielded` (`data/src/mixed.rs:545`). So when the resumed turn's envelope comes back, `Mixed` records it as a host outcome (`:552`) and, at dispatch, hands it to the child's `parse` inside `start` (`:352`). `Placed` applies it there and returns the value. `start` wraps the value in a second envelope (`:392`), and on that reply `Mixed` calls the worker child's `parse` again (`:533`). `Placed` has no stage left for the call and refuses it.

This is Fieldnotes' shape (`apps/fieldnotes/app.json:18` places its TypeScript on a worker beside a Rust source). Fieldnotes' TypeScript awaits no timer, and no other app places a source on a worker, so no app in `apps/` fails today.

## The failure

`js/tests/it/timers.rs`'s fixture, whose `slept(label, ms)` awaits a `setTimeout`, with the module on a worker under the composers a native host stacks:

```rust
let mixed = Mixed::new(module().placed(Placement::Worker),
    Placed::new(Quiet, Placement::Main), &["slept", "log", "spin"], &[]).unwrap();
let mut source = Storage::new(mixed);
source.activate().unwrap();
let mut r = Runner::boot(plan(), source, Kernel::with_monospace(), Default::default(), "/").unwrap();
r.act("sleep", vec![Value::str("a"), Value::Number(450.)]).unwrap();
// pump, advance the clock to 450, pump
```

Observed: `result` stays unset and the journal ends

```
t=450 fulfil 4 (result) [HTTP 200, 89 bytes] refused: Data { resource: "result",
  error: Unavailable("`slept`: a reply for an answer not in flight") }
t=450 request 4 (result) failed and is no longer pending: it is not retried: …
```

Expected: `a after 450`.

## Verification

Ran three drives with `cargo test -p exact-js --test it`: the worker-placed module alone and the main-thread module under `Storage` and `Mixed` both give `a after 450`; the worker-placed module under `Storage` and `Mixed` fails as above. With stand-in sources in `data/src` (`cargo test -p exact-data --lib`), a `Mixed` alone fails the same way, and a timer followed by a fetch fails before the fetch is sent.

## Direction

A worker child's `Later` that carries `due_ms` is that child's own recorded turn, not a request the host runs: `Mixed` should pass the reply under it to the child as a turn's reply, reserving the set again when it is dispatched, instead of recording a resume.

## Related

`issues/closed/20261009-typescript-failure-parity.md` added answer timers. `a_timer_wins_its_race_behind_the_composers` (`js/tests/it/timers.rs:446`) covers the composers with the module on the main thread only. `issues/20261110-ordered-main-child-loses-runner-clock.md` is the main child's clock in the same sets.
