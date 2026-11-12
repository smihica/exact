# The Apple bridge inserts presenter ops at byte 8 of every reply, which corrupts a transform drag's reply

**Status:** Closed
**Resolution:** Presenter work is added to transform replies' nested batches; mode and live replies keep the batch prefix and replies without a batch leave work pending.
**Systems:** host/apple/src/abi.rs (`emit`), host/apple/src/batch.rs (`prepend_to`), host/apple/src/transform_drag.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. Every reply the bridge returns goes through `emit`
(`host/apple/src/abi.rs:234-318`). It takes the requests the call left on the
runner, turns the ones the presenter runs into ops (a Live Activity send, an auth
session, a surface capture or restore, a presence check: `:263-293`, `:312`), and
adds them to the reply with `Batch::prepend_to` (`host/apple/src/batch.rs:584-595`):

```rust
let at = "{\"ops\":[".len();
debug_assert!(finished.starts_with("{\"ops\":["));
…
finished.insert_str(at, &text);
```

Development and release builds compile the assertion out (`host-dev` inherits
`release`, `Cargo.toml:149-150`). A transform drag's replies wrap their batch and
run app actions: a release answers
`{"accepted":true,"dispatched":true,"committed":…,"velocity":[…],"batch":{…}}`
after the `transformrelease` action (`host/apple/src/transform_drag.rs:285-298`),
and a geometry report answers `{"accepted":true,"committed":…,"batch":{…}}` after
`transformgeometry` (`:380-394`, `:566-574`). Both go through `emit`
(`abi.rs:1075-1087`). When a commit of that call asks the presenter for work, the
ops land inside the first key. `placement_box` writes its output without `emit`
for this reason (`abi.rs:1262-1272`).

## The failure

A `transformrelease` action that starts a Live Activity:

```
mutation sent as shape ID
action finish(px: number, py: number, s: number, vx: number, vy: number, vs: number)
  send sent = startActivity("train", State(title="Train"))
```

The release's reply:

```
{"accept{"op":"activity","ticket":1,"request":{"args":["train",{"title":"Train"}],"command":"startActivity"}},ed":true,"dispatched":true,"committed":true,…
```

Swift cannot decode it: `TransformDragReply(_:)` returns nil
(`host/apple/Sources/ExactKit/TransformDragBridge.swift:40-41`), so the release
applies no batch (`host/apple/Sources/ExactDrag/TransformDragHold.swift:127-132`).
The action ran in the runner, the presenter never gets its ops, and the activity
request is never run, so its mutation stays pending.

Expected: a reply that decodes, with the activity op in its batch.

## Verification

Temporary tests in `host/apple/src/transform_drag_tests.rs` drove a `Bridge` with
activities linked through a geometry report, a begin and a release
(`cargo test -p exact-apple --lib`). In a debug build the release stops at
`prepend_to`'s assertion. With the crate's debug assertions off
(`--config 'profile.dev.package.exact-apple.debug-assertions=false'`) it printed
the reply above, which `serde_json` refuses; a release whose action sends nothing
parsed. A timer due by the release whose action sends corrupts the reply the same
way under the runner's own timers; a live session's ordinary timers are native
and an input does not pull them in (`runner/src/runner/native_timers.rs:60-61`),
so there the path is the handler's own send.

Not run: Swift's decoder on the reply, an app on a device, and the agent's
`prefer mode` reply `{"modes":…,"ops":[…]}` (`host/apple/src/colors.rs:60`), which
takes the same insertion.

## Direction

Have the callers that wrap a batch hand `emit` the batch and the wrapper apart, so
the ops go into the batch wherever it sits, and refuse, not splice, a reply that
holds no batch.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names `prepend_to`.
