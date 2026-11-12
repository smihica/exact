# Package suites build fixtures that `exact new` makes web-only

**Status:** Closed
**Resolution:** Each fixture asks `exact new --with` for the hosts its suites build and writes the generated workspace before the first build; the Android suites use its `android` crate instead of writing one.
**Systems:** packages/*/test/fixture.mjs, packages/*/test/{apple,android,transfer,seams,drive}.mjs
**Severity:** P2
**Author:** Claude (Opus 5.5)
**Date:** 2026-11-11

## What is wrong

Line references are at `d991e8dd`. `exact new` makes an app whose platforms are its
manifest's `host.<platform>` keys, named by `--with` or by a native verb of the app's own
`exact.mjs` the first time it runs (LLP 1153 D1; `game/new.mjs:138-173`). A bare
`exact new` names none. The package suites under `packages/*/test/` make their fixture with
a bare `bun scripts/exact.mjs new <dir>` and then call a host's build directly
(`host/apple/build.mjs`, `scripts/build-linux.mjs`, `scripts/agent-android.mjs build`),
which names no platform. The generated workspace then has only the web crate, and the
build stops:

```
$ bun packages/camera/test/apple.mjs macos
Cargo has no camerafixture-apple target
      at run (packages/camera/test/fixture.mjs:14:33)
      at packages/camera/test/apple.mjs:72:3
```

The message is `scripts/app.mjs:1002` (`buildGraph`). Every suite that builds a native
host this way fails before its first check.

The Android suites fail earlier. Each writes Weatherlight's Canvas library into the app as
`android/` and edits the app's `Cargo.toml` to add it, but an app `exact new` makes has no
`Cargo.toml`: its workspace is the generated one under `.exact/shells/`, which holds an `android`
crate once the manifest names Android (`scripts/app-shells.mjs`, `crateFiles`). The
shared Android fixture (`packages/maps/test/android.mjs`, `androidFixture`) also reads the
app id from an `export const appId` that the starter `app.ts` no longer has, and throws
before it builds.

`grep -rln "exact.mjs'), 'new'" packages/*/test/*.mjs | xargs grep -L "'--with'"`
lists 29 files. `packages/dock/test/desktop.mjs` and
`packages/background-transfer/test/apple.mjs` already pass `--with`.

## Direction

Pass `--with` for the hosts each fixture's suites build, in the shared `fixture.mjs`
where a package has one, and write the generated workspace (`ensureShells`) after
`bun install`, before the first build, as `packages/dock/test/desktop.mjs` does: a
workspace the build wrote is newer than the build, and the driver refuses the build as
stale. The Android suites drop their hand-written crate and `Cargo.toml` edit.

## Related

`packages/device/test/linux.mjs --cross` reads the app's `linux/Cargo.toml` and
`Cargo.toml`, which are under `.exact/shells/` now and whose dependency paths are relative; that
mode is not covered here.
