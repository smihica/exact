# A reload or an activated update leaves the new runner with no module handler and no bound extensions

**Status:** Closed
**Resolution:** The Linux presenter's `replaced()` installs the module handler in the new runner's slot (waking the new executor), says the target and binds the extensions again, and points the package instances' notifiers at the new slot; Apple's `commit_plan` binds again the extensions the app's module bound.
**Systems:** host/linux/src/presenter/requests.rs, host/linux/src/presenter.rs, host/linux/src/presenter/delivery.rs, host/linux/src/canvas/requests.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The presenter gives a runner two things for module calls. `handle_modules`
(`host/linux/src/presenter/requests.rs:350`) installs the one handler in the runner's
native slot and records it in `requests.handling` (line 351). `serve_packages` (line 323)
binds the composed packages' extensions (line 339); Android's reader binds the APK's
(`host/linux/src/canvas/requests.rs:125`) from names it takes once (line 24).

A development reload (`reload_module`, `host/linux/src/presenter.rs:612`) and an activated
update (`activate_update`, `host/linux/src/presenter/delivery.rs:284`) replace the host,
its runner and the executor (`presenter.rs:671`, `delivery.rs:359`). The new runner has a
new slot (`runner/src/runner.rs:833`; a TypeScript source's `replacement`,
`js/src/source.rs:253`, makes a new one too) and no bound extension.
`Presenter::replaced` (`presenter.rs:1386`) restores neither, and `serve_packages` runs
only at boot (`host/linux/src/app.rs:534`). `requests.handling` stays true, so serving
again would return at the guard. From then on:

- a module name the plan declares is routed as unbound
  (`runner/src/runner/modules/admit.rs:592`): its declared substitute answers, never the
  package;
- any other long native call fails with "no native module here takes long calls"
  (`runner/src/runner/commit.rs:1033`).

The target the host said is lost as well. `host_target` falls back to the build's, the
same word on a device, but `owned_call` (`runner/src/runner/modules/pictures.rs:169`)
reads only a said one.

This is the Linux host's dev loop, a Linux release build whose app runs
`deliveryActivate()` (`host/linux/src/presenter/commands.rs:71`), and Android's dev loop.
Android's reader has no update store (`host/android/README.md:1256`), and Windows has
neither path.

## The failure

```rust
let mut p = boot();                      // packages_tests.rs's fixture
p.serve_packages(vec![Entry::of::<Probe>("probe")]);
assert!(p.host().runner().native_slot().handler().is_some());   // holds
let plan = crate::test_plan("component App\n  view\n    text \"Hi again\"\n");
p.reload(&plan, Nothing).unwrap();
assert!(p.host().runner().native_slot().handler().is_some());   // fails
```

Expected: a package answers after a reload or an update as it does after boot.

## Verification

Temporary tests in `host/linux/src/presenter/requests/packages_tests.rs` and
`delivery.rs`, run with `cargo test -p exact-linux --lib`:

- The test above failed at its second assertion. With `serve_requests()` in place of
  `serve_packages`, the handler was also gone, and a second `serve_requests()` did not
  bring it back.
- A plan that reads `geolocationStatus()` and sends `getCurrentPosition()` from
  `@exact/geolocation`, with a package composed for `geolocation`: before a reload
  `state.extensions.geolocation.bound` was true and the send returned the package's
  coordinates. After `reload` of the same plan it was false, the package heard nothing,
  the journal read `device permission 1 held (agent)` and the mutation stayed empty.
- `Runner::native_work` on a body naming no module: `{"error":"NotSupportedError"}` before
  the reload, `no native module here takes long calls` after it.
- An app's `deliveryActivate()` with a staged plan: the handler was present before the
  turn and absent after it.

Android's path was read, not run.

## Direction

Make what the presenter gives a runner part of `replaced()`: clear `requests.handling` and
install the handler in the new slot with the new executor's wake, bind the extensions and
say the target again (keeping the names Android's reader bound), and give package
instances already made the new slot (`requests/packages.rs:162`).

## Related

`QUEUE.md`, "A restart keeps per-node state that a new node with the same id then picks
up", is about Apple presenter state keyed by node id. "Android reader: a module source
that failed before the extensions were bound is not asked again" is about launch order.
Neither covers a replaced runner.
