# Update store calls from different threads share one input and one output buffer, so a call can take another call's answer

**Status:** Closed
**Resolution:** `Updates` holds one lock from each C call's payload to its copied answer, so a call on one thread can no longer replace another's input or output.
**Systems:** host/apple/Sources/ExactUpdates/Updates.swift, host/apple-update/src/store.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, the update store's C entries share one process-wide input buffer and one output buffer (`host/apple-update/src/store.rs:165-166`). `input(len)` clears and resizes the input (`:181-186`); every answer replaces the output (`emit`, `:193-197`). Each buffer has its own mutex, but a call is three steps: write the payload, call, copy the answer out. Nothing holds across the three. The module comment describes these entries as main-thread calls (`:12-13`).

The Swift face takes the three steps with no lock, queue or actor: `Updates` is an `enum` of static functions (`host/apple/Sources/ExactUpdates/Updates.swift:17`; `read` and `write` at `:32-42`, `asset` at `:79-86`). It is called from several threads. A delivered generation's `AssetResolver` reads each asset through `Updates.asset` (`:175-177`), and its lock (`ExactCore/Assets.swift:14`, held across the read at `:65-74`) serializes the callers of that one resolver only:

- the two raster worker threads (`ExactKit/RasterLoader.swift:416`), through the live resolver (`:252`, `RasterInput.swift:102`);
- the widget publisher's queue, which copies every asset of a new generation (`ExactCore/WidgetContainer.swift:127`, `:199`), through the live resolver;
- an activation on the main thread, outside that lock: `Updates.prepare()` and the plan (`Updates.swift:59-69`, `:233`), the module pair (`:189-201`), then the candidate resolver's fonts and first pictures (`ExactKit/Session.swift:921`, `:939`), on the main thread or the owner thread it waits for.

## The failure

An app running a delivered generation activates a staged update (`deploy.activate: "immediate"`, or `deliveryActivate`) while a worker reads a picture for the first time or the publisher copies. The activation's payload replaces the name the worker wrote, or its answer replaces the worker's before the worker copies it.

Observed: a call returns another call's answer. The live resolver caches what it read under the picture's name (`Assets.swift:72`), or the read fails. On the activation's side, `prepare()` finds nothing staged or the candidate is refused.

Expected: each call's answer is its own.

## Verification

A temporary Mac XCTest (`bun host/apple/build.mjs --test --only <Class>`, Caltrain's archive, no store open) ran these for two seconds:

```swift
// a second thread, in a loop
Updates.asset(Updates.Selection(token: 999, entry: "x", seq: 0, plan: Data(), assets: []), name: "assets/photo.jpg")
// the main thread, in a loop
Updates.prepare()
```

Alone, every `asset` call is refused with `unknown update generation`. Together, 2,577 of 619,000 came back with something else, 1,618 of them `prepare`'s `{"error":"no store is open"}`. With one `NSLock` held across each call on both threads, 0 of 119,000 did.

Not run: an app activating a signed delivered generation. Which threads make the calls is by reading the lines cited.

## Direction

Hold one static lock in `Updates` across `write`, the call and `read` for every entry that uses a buffer. The Rust side needs no change.

## Related

QUEUE "An update store reopened while another thread spawns is refused" is about `Store::open`'s file lock. Closed `20260904-check-holds-store-lock-across-network` is about the store mutex during a fetch. Neither covers the buffers.
