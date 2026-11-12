# The first update over the embedded bundle downloads every asset the binary already carries

**Status:** Closed
**Resolution:** Before fetching an asset the store lacks, `Store::obtain` reads the binary's embedded file of that name and keeps it in `blobs/` when its length and SHA-256 match the head's card.
**Systems:** update/src/store.rs, update/src/client.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

LLP 1026 D11, "Assets by digest": "the envelope lists each asset by name and digest; the store fetches what it lacks and reuses what it has, embedded assets included."

`Store::obtain` (`update/src/store.rs:1363`) skips a fetch only when `blobs.holds(card)` (`:1371`). What the store holds is `blobs/<sha256>` (LLP 1030.000: "Each file's bytes are kept once in `blobs/<sha256>` and hard-linked into entries"), and nothing puts an embedded asset there. The host's `embedded_asset` callback returns a digest, not bytes (`update/src/client.rs:106-110`, `:372`), and `admit_check` asks it only to decide whether entry zero is already the head (`store.rs:950-975`; the doc at `:759-761`: "it is asked only when the head's plan is the embedded one").

So while a binary runs its embedded bundle, `blobs/` is empty, and the first update fetches every asset the head names, including those the bundle holds under the same name and digest. A release followed by `bun scripts/push.mjs <app>` after a one-line Contract edit is this case: only `app.plan` changed, and each installed copy downloads the app's whole asset set (fonts, images, shaders) once. The second update reuses them, because the first filled `blobs/`.

## The failure

The bundle directory holds `assets/a1.png`, `assets/a2.png` and `assets/a3.png`. The head at seq 1 names a new plan and the same three assets by the same names, digests and sizes.

```rust
let mut client = Client::open_at(&base, &bundle, &compat, b"plan v1", Some(ORIGIN)).unwrap();
let outcome = client.check(&mut fetch); // `fetch` logs each URL's last segment
```

Observed: `staged seq 1 | fetched ["exact.json", "app.plan", "a1.png", "a2.png", "a3.png"]`.

Expected: `["exact.json", "app.plan"]`.

## Verification

Ran the case above as an integration test of `exact-update` (`cargo test -p exact-update --test <file> -- --nocapture --test-threads 1`, with `EXACT_AGENT` and `EXACT_UPDATE_DIR` unset). It printed the observed line. `Client::check` is `begin_check`, `CheckRequest::fetch` and `finish_check`, the three calls the Apple and Linux hosts make (`host/apple-update/src/store.rs:467-483`, `host/linux-update/src/store.rs:260-267`). No host was run.

## Direction

In `obtain`, before fetching, ask the host for the embedded file of that name and put it in the blob store when its length and digest match the card. The callback already finds the file under the asset root; it needs to hand back the bytes or a path as well as the digest.

## Related

`issues/20261110-update-store-collects-unfinished-download.md` is the neighbouring defect against the same sentence of D11: blobs fetched by an unfinished download are deleted at the next launch. It concerns files the store fetched; this one concerns files it never needed to fetch. `issues/closed/20260904-entry-zero-mistakes-an-asset-removal.md` fixed when entry zero counts as current and did not touch reuse.
