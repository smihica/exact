# The update store deletes an unfinished download's files at the next launch, so an interrupted update starts again from nothing

**Status:** Closed
**Resolution:** Opening the store now removes only entries no role keeps; blobs no kept entry names are collected only as a check finishes, so an unfinished download's verified files and an old cohort's files are reused by digest at the next launch.
**Systems:** update/src/store.rs, update/src/store/disk.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A download puts each verified file in `blobs/<sha256>` before it builds the entry (`Store::obtain`, `update/src/store.rs:1363`), so a retry in the same process fetches only what is missing. But `Store::open` ends with `collect()` (`store.rs:611`), which removes every blob that no kept entry names (`store.rs:1404`, `update/src/store/disk.rs:212`). An entry that never finished names nothing, so the next launch deletes everything the last one fetched.

Collecting at open is the written rule (LLP 1030.000, the 2026-09-23 store hardening: "open and each finished check keep only the selected, pending, last-good and running entries and the head accepted at the floor, and the blobs those name"). Its effect on an unfinished download is not written, and it defeats LLP 1026 D11's "the store fetches what it lacks and reuses what it has". A client whose sessions are shorter than the download (a large asset set on a slow link) never finishes: each launch fetches the same leading files and loses them.

The same collection makes a comment in `open` false (`store.rs:537-539`): after a new binary changes the cohort, "the old cohort's entries stay on disk (their files are still reusable by digest)". Nothing keeps them, and they are gone before the first check.

## The failure

With `exact_update::Client` over a transport that logs each request. A head names a plan and three assets; the transport loses `assets/a3`.

```
launch 1, a3 lost:      refused: …/assets/a3: the connection was lost | fetched ["exact.json", "app.plan", "a1", "a2", "a3"]
launch 1, retry:        refused: …                                    | fetched ["exact.json", "a3"]
launch 2, network back: staged seq 1                                  | fetched ["exact.json", "app.plan", "a1", "a2", "a3"]
```

Expected at launch 2: `["exact.json", "a3"]`, as the retry in launch 1 did.

A new binary (compatibility id `def`) over a store whose old cohort (`abc`) staged the same plan and assets:

```
old binary:  staged seq 1 | fetched ["exact.json", "app.plan", "a1", "a2", "a3"]
new binary:  staged seq 1 | fetched ["exact.json", "app.plan", "a1", "a2", "a3"]
```

Expected for the new binary: `["exact.json"]`.

## Verification

Ran both as integration tests of `exact-update`, each launch a `Client::open_at` on one directory; the lines above are their output. Not run: a real transport or a host.

## Direction

Let a blob outlive one launch without a reference: collect at open only blobs that were already unreferenced at the previous open (the blob's modification time is enough), or collect blobs only where a check finishes, where the store knows what the head names. Then correct the comment at `store.rs:537` and the sentence in LLP 1030.000.

## Related

`issues/closed/20261102-current-head-leaves-no-rollback-witness.md` and `20260924-update-concurrent-store-rollback.md` concern the floor and the owner lock, not blobs. `update/tests/it/durability.rs` `an_unchanged_asset_is_stored_once_and_old_entries_are_collected` asserts collection of superseded entries, which this leaves alone.
