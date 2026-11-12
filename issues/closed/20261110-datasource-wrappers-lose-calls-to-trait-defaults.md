# Every `DataSource` method has a default body, so a wrapper that omits a forward compiles and loses the call

**Status:** Closed
**Resolution:** Data-source wrappers implement every method through a required trait generated alongside the leaf defaults.
**Systems:** runner/src/runner/source.rs (`DataSource`), data/src/placed.rs, data/src/mixed.rs, data/host/src/lib.rs, logic/src/lib.rs, logic/src/configured.rs, host/render/src/source.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`DataSource` (`runner/src/runner/source.rs:312`, at `dd6f5dcd`) has 47 methods, and 46 have a default body that does nothing or answers `false`, `None` or an error. That suits a leaf: a Rust data crate implements `query` and little else. It also means a type that wraps another source and omits a method compiles, and the call stops at the wrapper with no error. The rule that a forwarder forwards is prose in the doc comments of eleven methods ("A source that forwards to another forwards this too").

Six types wrap a source and restate the list by hand: `Placed` (`data/src/placed.rs:639`), `Mixed` (`data/src/mixed.rs:585`), `Storage` (`data/host/src/lib.rs:155`), `Swappable` (`logic/src/lib.rs:229`), the `configured!` newtype (`logic/src/configured.rs:13`) and the render host's `Anonymous` (`host/render/src/source.rs:56`). An app's data is a stack of one to five of them: `Placed` alone for a TypeScript app on Apple, `configured!`, `Swappable`, `Storage`, `Mixed` and `Placed` for Fieldnotes on Linux. The methods at least one leaves to the default:

| Method | `Placed` | `Mixed` | `Storage` | `Swappable` | `configured!` | `Anonymous` |
|---|---|---|---|---|---|---|
| `note_storage_changed` | no | no | no | no | no | no |
| `module_landed` | no | yes | yes | yes | yes | yes |
| `module_delivery` | yes | yes | yes | no | no | no |
| `configure_secrets` | yes | yes | yes | no | no | by design |
| `keeps_sent` | yes | no | yes | no | no | no |
| `host_continuation` | yes | yes | yes | no | no | no |
| `set_document_owner`, `configure_storage`, `replacement` | yes | yes | yes | yes | yes | by design |
| `draw_2d` | no | no | no | no | no | no |

The other 37 are implemented by all six. `draw_2d` is harmless: each forwards `draw`, its only caller. `Anonymous` configures nothing on purpose. `keeps_sent` and `host_continuation` are asked only by a worker-placed `Placed` of the source it wraps, and no app places a wrapper on a worker, so those gaps are latent. The rest lose calls now:

- `issues/20261110-kept-reply-dropped-by-generated-entry.md`: `module_delivery` falls back to `module_landed`, which `Placed` drops. A render's source is `Anonymous` over `Placed` (`apps/expose/linux/src/bin/expose-render.rs:15`) and takes the same path.
- `issues/20261110-wrote-reports-no-storage-change.md`: `note_storage_changed`.
- `issues/20261110-swappable-drops-host-calls.md`: `configure_secrets`.

A wrapper that does forward can still miss the translation its other methods apply: `issues/20261110-storage-misroutes-delivery-continuation.md`.

## The failure

Each call made on a new wrapper over a leaf that records the methods it hears (a main-thread `Placed`, `Swappable::off`):

```
call                  leaf  Placed  Mixed  Storage  Swappable  configured!  Anonymous
note_storage_changed  yes   -       -      -        -          -            -
module_landed         yes   -       yes    yes      yes        yes          yes
configure_secrets     yes   yes     yes    yes      -          -            -
keeps_sent            yes   yes     -      yes      -          -            -
```

## Verification

Ran that matrix for 31 calls as a test beside `js/tests/it` (`cargo test -p exact-js --test it`). The rows above are its gaps, apart from the ones by design: `Anonymous`'s configuration, the calls `Mixed` routes to its TypeScript child, and `placement` and `when_preloaded`, which a composer answers itself. `module_delivery` reaches a leaf through each wrapper alone and is lost only in a stack, which the first issue shows by running. `host_continuation`, `replacement`, `draw_2d` and the render host's stack are read, not run.

## Direction

Make the forward mandatory where it is written: have a composer implement a trait whose methods have no default bodies, and give a leaf the defaults through a blanket implementation, so each row above and every method added later is a compile error. Separately, `configured!` and `Swappable::off` on Linux, Android and Windows exist only because those hosts construct `D: Default` (`js/bake/src/lib.rs:380`); taking the constructor, as Apple's `host!` does, removes both layers there.

## Related

The instance issues above. No issue, QUEUE entry or pitfall names the wrappers' forwarding.
