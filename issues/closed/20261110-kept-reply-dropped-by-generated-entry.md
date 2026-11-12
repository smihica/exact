# A kept request's reply never reaches a TypeScript module on Linux, Android or Windows

**Status:** Closed
**Resolution:** Generated entries forward kept replies and delivery continuations; inline Placed sources forward module_landed.
**Systems:** data/src/placed.rs, logic/src/configured.rs, logic/src/lib.rs, js/bake/src/lib.rs, runner module work (LLP 1146 D2)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A request with effects that the runner lets go runs to its end, and its reply reaches the let-go answer's continuation in a module turn, whose store writes stand (LLP 1146 D2). The runner delivers the reply with `DataSource::module_delivery` (`runner/src/runner/module_work.rs:236`). Its default body calls `module_landed` (`runner/src/runner/source.rs:426`), whose default does nothing (`:500`). At `dd6f5dcd`:

- `exact_logic::configured!` and `Swappable` forward `module_landed` (`logic/src/configured.rs:152`, `logic/src/lib.rs:399`) and not `module_delivery`.
- `Placed` forwards `module_delivery` (`data/src/placed.rs:1194`) and not `module_landed`, though the trait asks a source that forwards `background` to forward it (`source.rs:469`).

Stacked, the reply is dropped: `configured!` falls back to `module_landed`, `Swappable` passes it to `Placed`, and `Placed` answers `Ok(())` without calling the module. The bake writes that stack for every TypeScript app on Linux, Android and Windows (`js/bake/src/lib.rs:385`), and for one with a Rust module wherever its Rust mode is not `off` (`contract/cli/src/logic.rs:45`). `cargo check -p lights-linux` generates

```
exact_logic::configured!(AppData, ExactEmbeddedData, || exact_logic::Swappable::off(embedded_data()));
```

where `lights-apple` gets `type AppData = ExactEmbeddedData;`.

There the continuation after a let-go POST never runs: a renewed token is not stored, and an answer awaiting the same promise stays pending. A fetch that loses a race with its answer's own timer (`timer_before_reply`) is lost the same way.

## The failure

In `js/tests/it/timers.rs`, with `exact-logic` as a dev-dependency. The fixture's `postTimeout` logs `posted <n>` after its POST.

```rust
exact_logic::configured!(Generated, exact_data::Placed<Module>, || {
    exact_logic::Swappable::off(module().placed(exact_runner::Placement::Main))
});

let mut source = Generated::default();
source.activate().unwrap();
let mut r = Runner::boot(plan(), source, Kernel::with_monospace(), Default::default(), "/").unwrap();
let post = |r: &mut Runner<Generated>, n: &str| {
    r.act("post", vec![Value::str(n), Value::str("direct")]).unwrap();
    pump(r).remove(0)
};
let (first, second) = (post(&mut r, "1"), post(&mut r, "2")); // the first is let go and kept
r.fulfill(first.ticket, ok()).unwrap();
pump(&mut r);
r.fulfill(second.ticket, ok()).unwrap();
pump(&mut r);
r.act("readLog", vec![]).unwrap();
pump(&mut r);
result(&r, "result")
```

Observed: `posted 2`. Expected: `posted 1,posted 2`, which `module().placed(Placement::Main)` alone gives. Both journals say `keep request 1 (waited)` and `module work 1: HTTP 200, 0 bytes`.

## Verification

Ran that test both ways with `cargo test -p exact-js --test it`: the generated entry fails as above and the bare `Placed` passes. The same pair for `raceFetch` (a fetch raced against `sleep(100)`): `fetch landed` is logged behind `Placed` and never behind the generated entry. `a_renewal_outlives_the_answers_that_asked_for_it` (`js/tests/it/outlive.rs`, the Apple Bridge) booted on the generated entry ends with `token` showing `none` and `read` pending. No Linux, Android or Windows app was driven.

Seven TypeScript apps in `apps/` with a Linux or Android crate send POSTs from their data module (`agents`, `auth-fixture`, `expose`, `lights`, `messages`, `oauth-fixture`, `photo-editor`). LLP 1146 §9 records that `apps/oauth-fixture`'s renewal case fails on the Linux host when no let-go request is kept.

## Direction

Give `Placed` a `module_landed` that forwards inline, and forward `module_delivery` in `configured!` and `Swappable`: a worker-placed module's delivery takes two turns and cannot be expressed as `module_landed`.

## Related

`issues/20261110-swappable-drops-host-calls.md` names `module_delivery` among the calls the two `exact-logic` wrappers drop; the consequence it shows is the host's secrets. This issue is the kept reply, and `Placed`'s half of it. `issues/20261110-datasource-wrappers-lose-calls-to-trait-defaults.md` is the cause they share. `issues/closed/20261102-kept-replies-lose-tool-origin.md` is about the origin a kept reply runs under.
