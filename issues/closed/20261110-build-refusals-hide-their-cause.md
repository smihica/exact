# Three `contract build` refusals hide their cause: a missing Bun, a mistyped widget `preview`, a stale `.shells/app.json`

**Status:** Closed
**Resolution:** Build refusals name missing Bun without blaming a source file, identify preview fields with Contract types, and explain stale shell manifests for native modules.
**Systems:** contract/cli/src/ambient.rs, contract/cli/src/snapshot/entry.rs, contract/cli/src/native.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

Line references are at commit `dd6f5dcd`. Each refusal below was printed by
`target/debug/contract build` on the input shown and exited 1.

## 1. A missing Bun is reported as a missing file on `app.ts`

For an app with an `app.ts`, the build runs the bake's ambient-API analysis with Bun, found
as `$BUN`, then `~/.bun-1.4.2/bin/bun`, then `bun` on `PATH`
(`contract/cli/src/ambient.rs:16-27`). A spawn failure becomes the diagnostic as it is
(`ambient.rs:38-41`), attached to `app.ts`. With Bun at `~/.bun/bin` only and `BUN` unset:

```
…/app.ts:0:0 [analyze-ambient-api] ambient API check: No such file or directory (os error 2)
```

With `BUN=$HOME/.bun/bin/bun` the same app builds. `issues/closed/20260930-bake-missing-bun-unnamed.md`
fixed this message at the bake's two spawn sites with `exact_bake::bun_error`; this site has
its own lookup and spawn in a crate the bake depends on, so it cannot call that. Say the same
sentence here ("bun: not found; … install it, put ~/.bun/bin on PATH, or name it with BUN"),
on no source file, and read the pinned version as the bake does rather than spell `1.4.2`.

## 2. A mistyped `preview` prints the type's Rust `Debug` text and no field name

`preview` (`contract/cli/src/snapshot/entry.rs:493-560`) formats the expected type with
`{ty:?}` (`entry.rs:498`). With `shape CardProps` of `count: number` and `tags: string[]`, and
a widget whose `app.json` entry has `"preview": { "count": "7", "tags": [] }`:

```
app.contract:0:0 [snapshot-entry] entry card preview: "7" does not match Named("number", Span { line: 6, col: 10, end_col: 16, source_id: 0 }); supply preview props of the declared shape
```

A preview that leaves a field out gets the same sentence for the whole object
(`{"at":1,"kind":"a"} does not match Named("CardProps", Span { … })`), because the object's
size is compared (`entry.rs:510-513`) before the per-field refusal that would name it
(`entry.rs:518-520`). `viewport_shape` formats `{shape:?}` the same way (`entry.rs:163`, read,
not run). Print the type with `modules::written` (`contract/cli/src/modules.rs:756-770`), name
the field, and check each field before the size.

## 3. With a stale `.shells/app.json`, `bake-unknown-module` says `app.json` declares no modules

`Manifest::read` prefers a game's generated `.shells/app.json`
(`contract/cli/src/manifest.rs:146-152`). The refusal of an undeclared `hatch` word says when
that file is older than the authored one (`contract/cli/src/native.rs:604-609`); the refusal
of a module tag, from the same read (`native.rs:637-683`), does not. With `"modules":
["my-view"], "hatches": ["avatar"]` in `app.json`, neither in an older `.shells/app.json`, and
a view placing `my-view` and `view hatch="avatar"`:

```
app.contract:4:7 [bake-unknown-module] `my-view` is not a native module of this app: the app declares no native modules (app.json `modules`) and depends on no package that serves a view
app.contract:5:12 [bake-undeclared-hatch] `hatch="avatar"` is not declared: the app declares no hatch words (app.json `hatches`); .shells/app.json is stale, generated from an earlier app.json: `bun exact.mjs shells` regenerates it, as the web and test builds do
```

`Manifest::generated_stale` has that one caller. Append the same sentence in `check_modules`.

## Related

`issues/closed/20261006-tests-spawn-path-bun.md` is about which Bun tests spawn. Nothing in
`issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names the other two.
