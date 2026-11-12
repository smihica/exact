# `exact hatch` refuses every app `exact new` makes

**Status:** Closed
**Resolution:** `hatch` takes a platform from its crate or from `app.json` `host`, and writes the stubs a generated crate reads from `modules/`; the generated Linux crate runs no Rust hatches, which `issues/20261111-generated-linux-crate-runs-no-rust-hatches.md` holds.
**Systems:** scripts/exact.mjs (hatch), scripts/app-shells.mjs, docs/reference.md
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`hatch` (`scripts/exact.mjs:851`) decides which targets to write stubs for by looking for
crate directories in the app's tree: `apple/` (line 870), `web/` (962), `linux/` (1009) and
`android/` (1087). With none it throws (1140).

Since LLP 1153 D1 an app `exact new` makes has no such directories. Its platforms are the
keys of `app.json` `host`, and its crates are generated under `.exact/shells/` ("nothing
Rust enters the author's tree", `scripts/app-shells.mjs:1-7`). So the verb fails in every
new app, whatever platforms it names. LLP 1075.003.000.001 §13.11 says "an app `exact new`
made runs it as `bun exact.mjs hatch <word>`", `docs/reference.md:2481` gives that command,
`docs/contract-for-agents.md:56` sends authors to it, and `bun exact.mjs hatch --help`
shows it as the example.

## The failure

```sh
bun scripts/exact.mjs new /tmp/with-native --with ios,mac,linux,android
cd /tmp/with-native && bun exact.mjs hatch avatar
```

Observed (exit 1), with `host.web`, `host.ios`, `host.macos`, `host.linux` and
`host.android` all in `app.json`:

```
exact: hatch: with-native builds no target a hatch can be written for (apple/, web/, linux/ or android/)
```

`hatch --app` and `hatch --window` print the same line.

Expected: the stubs the builds read from the app's own tree, `modules/web/index.js`
(`host/web-js/build.mjs:168`), `modules/apple/*.swift` (`host/apple/assets.mjs:68`) and
`modules/android/Hatches.kt` (`scripts/agent-android.mjs:416`), and the word in `app.json`
`hatches`.

## Verification

Ran the two commands above in a new directory, then `hatch --app` and `hatch --window`: each
printed the refusal and exited 1, and nothing was written but `.exact/commands.jsonl`.

`bun test scripts/exact.test.mjs -t "exact hatch"` passes (1 pass). It passes because the
test creates `apple/` and `web/` by hand before the first call
(`scripts/exact.test.mjs:558`), then `linux/` (589) and `android/` (600): the layout of an
in-repository app or an authored workspace, never of what `createApp` writes.

Not run: a native build of a new app with hand-written stubs.

## Direction

When the app has no crate directories, read the targets from the manifest (`namesPlatform`,
`scripts/app-shells.mjs:46`; the web when `host.web` is named), and keep the directory test
for an authored workspace. Two of the stubs' "to do" lines name a `build.rs` the author of a
generated workspace cannot edit: the generated Android crate already includes the hatches
(`scripts/app-shells.mjs:218`), and the generated Linux crate (lines 170 to 188) includes
none, so say what applies there. Have the test start from what `createApp` writes.

## Related

No issue, QUEUE entry or `docs/agent-pitfalls.md` line names the verb. QUEUE's "LLP
1075.003.000.001, access hatches on every platform" lists presenter work.
