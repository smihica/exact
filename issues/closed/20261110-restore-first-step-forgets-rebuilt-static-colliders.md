# The first step after a restore forgets which static colliders the save rebuilt, so the next save or hash verifies them all again

**Status:** Closed
**Resolution:** The first step after restore retains verified rebuilds for unchanged static colliders and invalidates them when sync touches the collider.
**Systems:** game/physics/src/step.rs, game/physics/src/state.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

An EXPHYS v3 snapshot writes each static collider that its entry rebuilds bit-exactly as a hole. `Saved::decode` records every hole it fills in `Live::elidable` and says why (`game/physics/src/state.rs:133-135` at `dd6f5dcd`):

```rust
// A filled hole is its entry's rebuild by construction: elidable without
// re-verifying, so a restore does not re-serialize every static collider.
```

The first `physics::step` in a restored world is a full sync (`live.synced` is `None`), which visits every row, and `sync` drops the mark of every row it visits before it looks at whether anything changed (`game/physics/src/step.rs:160-163`):

```rust
for &e in &pending.rows {
    if let Some(h) = live.entries.get(&e).and_then(|entry| entry.collider_handle) {
        live.elidable.remove(&h);
    }
```

So after one step `elidable` is empty, and the next `refresh` runs `verify` over every static collider: one rebuild and two bincode encodings each (`state.rs:281-296`). Only a world wholly at rest keeps its marks, because `unchanged_asleep` skips `sync` (`step.rs:372`, `:436`).

## The failure

A world of static offset cylinders (the Forest's trunks, as in `physics/tests/scale.rs`) over a box floor with one walking `CapsuleController`. Save, load into a fresh world, tick once, then time `Physics::refresh_snapshot`:

| trees | steady snapshot | first snapshot after restore and one step | the one after |
|---|---|---|---|
| 20,000 | 1.5 ms | 9.1 ms | 1.7 ms |
| 100,000 | 8.3 ms | 48 ms | 8.5 ms |

Expected: the first snapshot after a restore costs what a steady one does, as the comment in `decode` states. The two worlds stay byte-identical; only the time differs. For scale, the load itself takes 34 and 167 ms.

A save or a hash refreshes the snapshot. A world is restored when a save is opened, when a dev-loop gameplay reload carries the running world (`game/README.md:80`), and in a proof's Save and FreshGame modes, which restore at every tick they observe.

## Verification

Ran it as a temporary test in `game/physics/tests/`, three times on an arm64 Mac (from `game/`, `cargo test -p exact-game-physics --release --test <name> -- --nocapture`). The table gives the medians; the first snapshot after a restore ranged from 8.9 to 9.6 ms and from 47 to 49 ms. With the change below tried in the worktree, that snapshot cost the steady figure (3.1 and 18.7 ms in the test profile, against 24.7 and 127.7 ms before) and every test of `exact-game-physics` passed, pins included.

## Direction

Drop a row's `elidable` mark in `sync` only where the Rapier collider is touched: when the row has or had a Body, when `collider_changed`, and when the pose changed.

## Related

`game/physics/README.md` documents the hole format and its steady cost, not the cost after a restore. No issue or `QUEUE.md` entry names `elidable` or EXPHYS.
