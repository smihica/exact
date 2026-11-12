# Five bench apps cannot be built: their patch list lacks accesskit_windows

**Status:** Closed
**Resolution:** The five bench workspaces patch `accesskit_windows` as the root does, so `outsideWorkspaceProblems` admits them.
**Systems:** bench/list, bench/crypto-list, bench/extra-heavy, bench/dioxus, Cargo.toml, scripts/app.mjs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd` the root `Cargo.toml:44` patches `accesskit_windows` to `vendor/accesskit-windows`, and `outsideWorkspaceProblems` (`scripts/app.mjs:197-203`) refuses an app workspace whose `[patch.crates-io]` does not name every crate the root patches. Five bench apps, each a workspace of its own, carry a four-entry list without it:

- `bench/list/exact-listbench/Cargo.toml:18-22`
- `bench/crypto-list/exact-crypto-gpu/Cargo.toml:19-23`
- `bench/crypto-list/exact-crypto-svg/Cargo.toml:18-22`
- `bench/extra-heavy/exact-xheavy/Cargo.toml:19-23`
- `bench/dioxus/exact-bounded/Cargo.toml:16-20`

`bench/heavy-list/exact-heavylist/Cargo.toml:17-22` has all five entries.

The apps are in use. `bench/board/builds.sh:26,38` builds `exact-xheavy` and `exact-crypto-svg` at HEAD for the device scoreboard, and `QUEUE.md:876` and `:936` record work of 2026-10-22 and 2026-10-23 measured on `bench/extra-heavy` and on a built `exact-xheavy`.

## The failure

From `bench/list/README.md:76-77`:

```sh
APP=$PWD/bench/list/exact-listbench
EXACT_APP_DIR=$APP bun host/apple/build.mjs --ios exact-listbench-apple
```

Observed: exit 1 before Cargo runs.

```
…/bench/list/exact-listbench/Cargo.toml: [patch.crates-io] must name exact2's vendored accesskit_windows. Use (or run `bun exact.mjs update` in an app `exact new` made):
[patch.crates-io]
accesskit_windows = { path = "../../../vendor/accesskit-windows" }
taffy = { path = "../../../vendor/taffy" }
parley = { path = "../../../vendor/parley" }
fontique = { path = "../../../vendor/fontique" }
wgpu-hal = { path = "../../../vendor/wgpu-hal" }
```

Expected: the build starts. Every builder of these apps reaches the same check through `resolveApp` (`scripts/app.mjs:536-537`): `bench/crypto-list/build-exact.sh`, `bench/extra-heavy/ios/build-exact.sh`, `mac/build-exact.sh` and `web/build.sh`, `bench/dioxus/web/build.sh`, `mac/build.sh` and `device/build.sh`.

The async lane is not affected. `host/web-js/lower-equal.test.mjs:278-282` builds `exact-bounded` with `host/web-js/build.mjs` alone, which does not reach the check for this app.

## Verification

Ran the build command each README gives. The same refusal, exit 1, came from `bench/list/README.md:77`, `bench/crypto-list/exact-crypto-svg/README.md:32`, `bench/crypto-list/exact-crypto-gpu/README.md:27`, `PART=bounded bench/dioxus/web/build.sh` (`bounded: BUILD FAILED`) and `EXACT_APP_DIR=$PWD/bench/dioxus/exact-bounded bun host/apple/build.mjs exact-bounded-apple`. The command of `bench/extra-heavy/README.md:78-79` is refused first for `icon.png`, which `prepare.sh` writes; with a placeholder icon in place it prints the same patch refusal. `EXACT_APP_DIR=$PWD/bench/dioxus/exact-bounded bun host/web-js/build.mjs exact-bounded --out <dir> --render none`, with an empty `data/messages.json`, exits 0. Not run: a full app build, and the device scripts (they need signing identities).

## Direction

Add `accesskit_windows = { path = "../../../vendor/accesskit-windows" }` to the five lists. Five of the six manifests (all but `exact-listbench`) also carry `[profile.*]` tables under a comment about `[profile.apple-dev]`, a profile that no longer exists. `injectedProfiles` (`scripts/app.mjs:158`) passes the root's profiles to every outside build and outranks a manifest, so those tables and the comment can be deleted.

## Related

No entry in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names these workspaces or the patch list.
