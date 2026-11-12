# A second instance of an app boots the embedded bundle while the first runs the update

**Status:** Closed
**Resolution:** A process that finds the store owned opens it as a reader that boots the record's selection, holds that entry with a shared lock the owner's collection respects, and writes nothing.
**Systems:** update/src/store.rs, host/linux-update/src/lib.rs, host/apple-update/src/store.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Store::open` takes an exclusive lock on `owner.lock` and fails when another process holds it (`update/src/store.rs:482-494`; LLP 1026 D11: "A second owner is refused before it can read a stale record or sweep another owner's temporary files"). The lock protects the record. It also takes the selection away from the second process, because both hosts treat a failed open as "no store":

- Linux: `Err(error) => eprintln!("exact update: {error}")`, then `run_config` with the baked plan (`host/linux-update/src/lib.rs:61-65`).
- Apple: `open` answers the refusal and the app "runs without one and answers the embedded facts" (`host/apple-update/src/store.rs:204-228`).

So when an app is opened twice (two launches of a Linux desktop app, `open -n` on a Mac), the first process runs the selected update and the second runs entry zero: an older plan and older assets, over the same app storage. The second never checks for updates. Neither LLP 1026 nor LLP 1030 says what a refused process boots.

## The failure

Caltrain on the Linux host (`cargo build --profile host-dev -p caltrain-linux`), with `EXACT_UPDATE_DIR` naming one store and `EXACT_UPDATE_ORIGIN` a local origin whose head (seq 1) names a plan that differs from the baked one in the brand text. One launch stages the update. Then two processes, the second started while the first runs, each asked `tree brand` and `state delivery`:

```
first:   Text#10 [brand] "Caltrain v2" · stream "prod/0662de1b11c1408c48823779a035906a" · seq 1
second:  Text#10 [brand] "Caltrain"    · stream "embedded" · seq 0 · staged false
  exact update: update store …/store is unavailable (exclusive owner): lock acquisition failed because the operation would block
```

Expected: the second instance boots seq 1 as well. Selection is "a stat and a read" (LLP 1026 D9) and needs no write.

## Verification

Ran the above with `bun scripts/agent.mjs linux --app caltrain`; the lines are its output. The Apple host's fallback was read, not run.

## Direction

When the lock is held, open read-only instead of failing: read the record and the selected entry and write nothing (no sweep, no collection, no boot marks, no check), as `frozen` already does for a foreign record (`store.rs:513-518`). The owner's collection must then keep an entry a reader holds; a shared lock on the entry, or leaving collection to the next exclusive open, would do. LLP 1026 D11's sentence refuses a second reader as written, so it wants amending with the change.

## Related

`issues/closed/20260924-update-concurrent-store-rollback.md` added the lock; it does not discuss what the refused process boots. QUEUE.md's "An update store reopened while another thread spawns is refused" is a spurious refusal within one process.
