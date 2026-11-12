# `exact share` refuses an app that has an update store but no signing keys

**Status:** Closed
**Resolution:** `share` checks for verification keys before it builds and, without them, refuses at once naming the two ways on: `deploy.store.ios: "0"`, or a key in `deploy.signing.keys`.
**Systems:** scripts/share.mjs, host/apple/build.mjs, bake/src/compat.rs
**Severity:** P2
**Author:** Claude (Opus 5.5), from sharing the Softwear template
**Date:** 2026-11-10

## What is wrong

`bun exact.mjs share` is how a person puts an iPhone app on someone's phone.
It builds with `host/apple/build.mjs --device --archive <ipa> --unsigned`,
and `--archive` makes the build a release with production update trust. For
an app whose `deploy.store.ios` is not `"0"`, which is every app that
says nothing about `deploy`, `share` sets `EXACT_UPDATE_GENESIS=1` so that
the build starts its own stream (`scripts/share.mjs`).

The bake then refuses it (`bake/src/compat.rs`), because production trust
with an update store needs a verification key:

```
production updater requires deploy.signing.keys with at least one verification key; use EXACT_UPDATE_TRUST=development only for a development artifact
```

So `share` fails for an app made with `exact new` that has never been set up
to deploy, which is the app `share` is meant for. `build.mjs` also ignores
`EXACT_UPDATE_TRUST=development` for an archive, so the message's advice
does not apply here.

## The failure

Seen with the Softwear template in `expo/exact-templates` on `1cbce180d4`,
and with the same `build.mjs` call on `4756891bc2`. The build succeeds once
`app.json` has `"deploy": { "store": { "ios": "0" } }`, which makes a
binary that takes no over-the-air updates. That is what was shared, by
editing `app.json` for the build and reverting it afterward.

## Fix

`share` could build a tester's binary that needs no keys: one that takes no
updates (as `deploy.store.ios: "0"` does) unless the app has signing keys.
Failing that, `share` should check for the keys before it builds and say
what to add to `app.json`: either `deploy.signing.keys` or
`deploy.store.ios: "0"`.
