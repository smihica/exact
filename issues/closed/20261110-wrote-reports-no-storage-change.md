# `ExactModuleContext.wrote` marks no storage change: no source wrapper forwards `note_storage_changed`

**Status:** Closed
**Resolution:** Every source wrapper forwards native storage-change notifications; worker placement records them in its shared flag.
**Systems:** data/src/placed.rs, data/src/mixed.rs, data/host/src/lib.rs, logic/src/configured.rs, logic/src/lib.rs, host/apple/src/abi.rs, widgets
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A native module that writes the app's files calls `context.wrote(path)`, which promises that "a widget's timeline is reloaded from the store as written" (`host/apple/modules/ExactNativeModule.swift:171`). The host turns it into `host.runner_mut().data().note_storage_changed()` (`host/apple/src/abi.rs:595`), and the next `persist` posts the widget change when `take_storage_changed()` then answers true (`host/apple/src/persistence.rs:8`).

`note_storage_changed` defaults to a no-op (`runner/src/runner/source.rs:338`) and only `exact_js::Module` implements it (`js/src/source.rs:46`). At `dd6f5dcd` no wrapper forwards it, though each forwards `take_storage_changed`: `Placed` (`data/src/placed.rs:653`), `Mixed` (`data/src/mixed.rs:595`), `Storage` (`data/host/src/lib.rs:163`), `Swappable` (`logic/src/lib.rs:241`) and `configured!` (`logic/src/configured.rs:21`).

The app's data is never a bare `Module`. The generated shell makes it `exact_js::Placed<exact_js::Module>` (`scripts/app-shells.mjs:115`), under `configured!` when the app has a Rust module. So the call stops at a wrapper in every app. The answers watching the path are still asked again, through `native_slot().changed(topic)`. The storage change is the half that is lost: a widget reloads only if the same commit also writes the store.

## The failure

With `module` an activated `exact_js::Module` (any fixture of `js/tests/it`):

```rust
let mut placed = module.placed(Placement::Main);
placed.note_storage_changed();
placed.take_storage_changed()
```

Observed: `false`. Expected: `true`, which the `Module` itself answers.

## Verification

Ran it in `js/tests/it` with the timers fixture's module (`cargo test -p exact-js --test it`). `note_storage_changed()` then `take_storage_changed()` printed

```
module true, Placed false, configured!(Swappable) false, Storage(Mixed) false
```

for the module, for `Placed` over it, for `configured!(…, || Swappable::off(…))` over that, and for `Storage` over a `Mixed` over it. No app with a widget and a native module was built. No test in the tree calls `app_changed` or `note_storage_changed`.

## Direction

Forward `note_storage_changed` in each wrapper to the source it wraps. For a worker-placed `Placed`, set the `storage_changed` flag its owner thread already shares (`data/src/placed.rs:256`).

## Related

`issues/closed/20261019-widget-refresh-native-hook-stale-layout-scale.md` added `wrote` and `note_storage_changed` and is closed as fixed; that fix does not take effect in a generated app, for the reason above. `issues/20261110-swappable-drops-host-calls.md` names the two `exact-logic` wrappers; forwarding there alone leaves `Placed`, which every app has. `issues/20261110-datasource-wrappers-lose-calls-to-trait-defaults.md` is the cause they share.
