# The generated app entry does not pass the host's shared secrets to the data source, so `@exact/oauth` cannot sign in on Linux, Android or Windows

**Status:** Closed
**Resolution:** Swappable and configured forward shared-secret configuration, host continuations and kept-fetch queries; the separate Linux pump issue still blocks the OAuth fixture.
**Systems:** logic/src/lib.rs (Swappable), logic/src/configured.rs, js/bake/src/lib.rs, host/linux/src/host.rs, packages/oauth
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Swappable<D>` (`logic/src/lib.rs:229`) and `configured!` (`logic/src/configured.rs:13`) implement `DataSource` by forwarding each method to the embedded source by hand. The trait has 47 methods and both forward 41. `configure_secrets` is one of the six left to the trait's default, which does nothing (`runner/src/runner/source.rs:358`).

The bake puts every TypeScript app on Linux, Android and Windows behind both wrappers (`js/bake/src/lib.rs:386`), and every app that names a Rust module behind them on every platform (`contract/cli/src/logic.rs:46`). An Apple app with no Rust module is not wrapped.

The Linux host, which the Android and Windows entries also boot through, gives the source its shared secrets with this call and nowhere else (`host/linux/src/host.rs:277`; Apple: `host/apple/src/host.rs:463`). The call stops at the wrapper, so the TypeScript module has no backend and refuses `store.exclusive` (`js/src/secrets.rs:230`). `@exact/oauth` runs every code exchange, refresh and sign-out inside `store.exclusive` (`packages/oauth/index.ts:264`); no other package or app calls it.

On these three hosts a system-browser sign-in already answers 501 outside the agent (LLP 1069.006 D5). What this breaks is a sign-in under `exact test` or the agent, and a sign-in through the app's own screens (LLP 1146 D8).

Of the other five: `keeps_sent` and `host_continuation` are asked only by `Placed` and `Mixed` of their own children, so nothing calls the wrapper's copy today; `draw` covers `draw_2d`; `module_delivery` and `note_storage_changed` are dropped as well, each with an effect of its own (a kept request's reply, `context.wrote`).

## The failure

```
cargo build --profile host-dev -p oauth-fixture-linux
bun scripts/agent.mjs linux --app oauth-fixture --test apps/oauth-fixture/app.test.contract
```

Observed: `0 passed, 2 failed`, each with `mutation signedIn's reply was refused: shared secrets are unavailable on this host`. Expected: both pass, as the app's diary records for the Linux host on 2026-10-29.

## Verification

The two commands, run against these states of the tree:

| Wrappers | Linux `pump` | Result |
|---|---|---|
| unchanged | unchanged | 0 of 2: "shared secrets are unavailable on this host" |
| forward the five calls | unchanged | 0 of 2: no refusal; `signedIn` stays pending on a continuation |
| unchanged | releases held work when woken with no reply | 0 of 2: the refusal again |
| forward the five calls, or `configure_secrets` alone | releases held work when woken with no reply | 2 of 2 |

The fixture therefore needs a second fix. The secret lease's continuation is dispatched `Held` until the lease is ready, and the lease then wakes the host. The Linux `pump` returns before `after_commit` when a wake brings no reply (`host/linux/src/presenter/pump.rs:42`), and `release_work` is called only in `after_commit` (`host/linux/src/presenter.rs:890`). Apple's `pump` always ends in `emit`, which releases (`host/apple/src/abi/pump.rs:42`, `host/apple/src/abi.rs:303`).

Not run: Android, Windows, or an Apple app with a Rust module.

## Direction

Forward `configure_secrets` and the other dropped calls in both wrappers, and release held work in the Linux `pump`'s early return. One delegation shared by every wrapper would stop the next defaulted method from being dropped.

## Related

`issues/20261110-datasource-wrappers-lose-calls-to-trait-defaults.md` gives the forwarding table for every wrapper, and `issues/20261110-linux-pump-leaves-held-work-parked-when-woken-with-no-reply.md` is the `pump` half. No QUEUE entry or pitfall names `configure_secrets`. `issues/closed/20261102-oauth-refresh-lock-does-not-span-realms.md` added `store.exclusive`; its verification lists no drive of a built Linux app.
