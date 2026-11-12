# The painting host leaves held work parked when it is woken with no reply, so a secret lease's continuation never runs

**Status:** Closed
**Resolution:** The painting host releases ready held continuations on a pump wake even when no reply or topic change is queued.
**Systems:** host/linux/src/presenter/pump.rs, host/linux/src/presenter.rs (`after_commit`)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A data source may answer a dispatch `Held` when the work waits on something off the runner's thread, and wake the host when it is ready. The host then asks `release_work()` and runs what was parked. The secret lease works this way: `js/src/secrets.rs` answers `Held` until the broker thread has the lease, then calls the host's wake.

On the painting host (`exact-linux`, which Linux, Android and Windows share) `release_work` is called in one place, `Presenter::after_commit` (`host/linux/src/presenter.rs:890`). `Presenter::pump` returns before `after_commit` when the wake brought no reply and nothing was announced (`host/linux/src/presenter/pump.rs:42-51`):

```rust
if outcomes.is_empty() && !self.host.has_announced() {
    …
    return region_error.or(refined).or(self.refresh_transform_geometry()).or(self.deliver_scroll_ends());
}
let e = self.host.fulfill_all(outcomes, now_ms);
let after = self.after_commit();
```

A wake whose only purpose is to release held work is exactly that case, so the parked request stays parked. The Apple bridge's `pump` always ends in `emit`, which runs the same release loop (`host/apple/src/abi/pump.rs:38-42`, `host/apple/src/abi.rs:303`).

## The failure

`apps/oauth-fixture` on the Linux host, with the generated entry's missing forwards supplied (`issues/20261110-swappable-drops-host-calls.md`): the sign-in's `signedIn` mutation stays pending on its continuation until the driver's `clock data` gives up at 20 s. The journal's last lines are `continuation 7 (signedIn): executor token 5` and `fulfil 6 (signedIn) [HTTP 302 …]`.

## Verification

The fixture's two Linux tests were run against four states of the tree:

| Entry forwards the host's calls | `pump` releases held work in its early return | Result |
|---|---|---|
| no | no | 0 of 2: "shared secrets are unavailable on this host" |
| yes | no | 0 of 2: `signedIn` stays pending on a continuation |
| no | yes | 0 of 2: the refusal again |
| yes | yes | 2 of 2 |

Not run: Android or Windows, which use the same presenter.

## Direction

Run the `release_work` loop in `pump`'s early return too, or move the loop to where every pump reaches it, as the Apple bridge has it.

## Related

`issues/20261110-swappable-drops-host-calls.md` is the other fix the fixture needs. No gate or lane runs `apps/oauth-fixture`'s Linux tests, which is how both stood.
