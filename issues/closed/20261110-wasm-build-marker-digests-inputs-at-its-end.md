# A wasm-target web build whose source is edited while it runs is taken as current

**Status:** Closed
**Resolution:** Wasm web builds retain pre-bake authored digests and refresh only recorded bake outputs, including declared levels; paired handwritten assets remain checked.
**Systems:** host/web/build.mjs (the wasm target), scripts/agent-launch.mjs (`webChanges`)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

A web build's marker, `.exact-build.json`, records a content digest of every input, and `webChanges`
(`scripts/agent-launch.mjs:1018-1036`) compares them with the files as they are now. The driver refuses a drive
of a built `dist` on what it returns (`scripts/agent-playwright.mjs:75-76`), `scripts/evidence.mjs:57` calls a
proof stale by it, and conformance checks its wasm oracle with it (`host/web-js/conform.mjs:314`).

The JS target takes the digests, and the time, before it compiles anything (`host/web/build.mjs:108`), and says
why: "a source changed or gone while it compiles, or a build published over it meanwhile, is a change, never
taken as built". The wasm target (a game's web build, `--wasm`, `--bake`) takes them as its last step
(`build.mjs:575-582`), after the bake, `wasm-opt`, the documents and the GPU modules, and records no
`inputsAt`. The marker so describes the files as they are when the build ends, not as the build read them. A
source edited after the bake compiled the plan matches its digest, and the build that lacks the edit is fresh.

The game dev loop is not affected: `host/web/dev.mjs` trusts a build by `bake.json`, not by these digests.

## The failure

`apps/sparkline/app.contract` is edited (`text "Markets"` to `text "MarketsFirst"`), then
`bun host/web/build.mjs sparkline --wasm` is started, and the moment the bake's receipt
(`target/bake/…/web-wasm32-unknown-unknown.json`) is rewritten the source is edited again, to
`text "MarketsSecond"`. After the build exits 0:

```
wasm build exit 0 after 18580 ms; source edited again at 550 ms; stage directory made at 14485 ms
built app.plan holds MarketsFirst: true · MarketsSecond: false
webChanges(dist, app).all = []
```

Expected: `webChanges` names `apps/sparkline/app.contract` and the driver refuses with the rebuild command
(LLP 1012.001.000 D1). A JS-target build edited 215 ms after it started answers
`["apps/sparkline/app.contract"]`.

The window is the build from the bake to its end: 18 of 18.6 s here.

## Verification

Ran the sequence above after one warm build. A second run edited the source once the stage directory existed:
the same result, and `bun scripts/agent.mjs web --app sparkline tree` then exited 0 and printed
`Text#3 "Markets"` while the source said `MarketsEditedMidBuild`. Restoring the source afterwards made
`webChanges` name it, though the build then matched it. With nothing edited, the digests of all 969 inputs
taken before the build equal the marker's. Not run: a game's build, which takes the same path
(`build.mjs:89-91`, `:163` on).

## Direction

Take `webInputDigests(app, false)` and the time before `buildBake` (`build.mjs:194`) and write both into the
marker, as the JS path does at `:108` and `:128-129`. `webChanges` already reads `inputsAt`.

## Related

`issues/closed/20260904-dev-mjs-serves-a-stale-dist.md` is about `--app` switches.
`issues/20261110-serve-rebuild-misses-deep-sources.md` is `serve`'s own walk. Neither covers the marker's
digests.
