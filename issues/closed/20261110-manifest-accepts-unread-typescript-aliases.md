# `Manifest::placement` still accepts `typescript.aliases`, a retired key the scripts refuse and nothing reads

**Status:** Closed
**Resolution:** Manifest::placement rejects the retired typescript.aliases key as unknown, matching the script manifest validator.
**Systems:** contract/cli/src/manifest.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`issues/closed/20260927-manifest-keys-follow-the-web.md` retired `typescript.aliases`: import
aliases come from the app's `tsconfig.json` `paths`, the key left `scripts/app.schema.json`,
and the bake's alias translation was removed. The compiler's own check of the `typescript`
object was not changed. `validate_placement_policy`
(`contract/cli/src/manifest.rs:490-542`) still admits the key, with a comment that is no
longer true (`manifest.rs:527-532`):

```rust
// Mounted directories and import aliases: the bake resolves and checks them (`js/bake`).
"aliases" => {
    value
        .as_object()
        .ok_or("typescript.aliases must be an object of specifier to directory")?;
}
```

`js/bake` reads `typescript.sources` and no `aliases`. So the two validators of one manifest
disagree on the key, and the one that accepts it then ignores it: a bake that reads the
manifest without the scripts (`bake/src/compat.rs:557`, under a plain `cargo build` of an
app's crate) gives no diagnostic and no alias.

## The failure

`app.json` with `"typescript": { "aliases": { "@core": "../core" } }`:

| reader | result |
|---|---|
| `Manifest::placement("typescript", "ios")` | `Ok("main")` |
| `readManifest` (`scripts/app.mjs`) | `app.json does not conform to scripts/app.schema.json: typescript.aliases: not a known key` |

For comparison, `{"typescript": {"elsewhere": 1}}` gives `Err("invalid typescript policy key:
elsewhere")` from `Manifest::placement`.

## Verification

A temporary unit test beside `typescript_sources_sit_beside_the_placement`
(`manifest.rs:557-571`) printed the two `Manifest::placement` results above. `readManifest`
was called on a directory holding that `app.json` and printed the refusal quoted;
`scripts/app-manifest.test.mjs:200` asserts it. A search of `js/bake`, `bake`, `scripts`,
`host/web` and `host/web-js` for `aliases` finds no reader of the key.

## Direction

Delete the `aliases` arm, so the key is refused as any unknown key is, and shorten the comment
to `sources`.

## Related

`issues/closed/20260927-manifest-keys-follow-the-web.md`, whose change this arm was left out
of.
