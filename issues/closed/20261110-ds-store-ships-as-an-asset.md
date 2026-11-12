# Finder's `.DS_Store` in an app's `assets/` becomes an embedded asset, and `push` publishes it

**Status:** Closed
**Resolution:** Static asset inventories and copies omit Finder's .DS_Store files at every depth; push uses the same owned filesystem reader.
**Systems:** filesystem/src/directory.rs (visit_files), bake/src/receipt.rs (asset_cards), scripts/push.mjs (carriedAssets), host/web/serve.mjs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The owned filesystem's `tree` and `names` list every regular file under a root (`filesystem/src/main.rs:20-37`, `filesystem/src/directory.rs:55-77`). Their callers take the list whole: the bake makes an asset card for each name under `assets/` and `deck/` (`bake/src/receipt.rs:219-224`), and the web server and web build capture the same trees (`host/web/serve.mjs:253`, `:1002`). `push` lists the two directories itself, with no filter either (`scripts/push.mjs:168-184`). The asset name rule admits a leading dot (`update/src/envelope.rs:479-498` refuses only `.` and `..` as a segment).

macOS's Finder writes a `.DS_Store` into every folder it displays, and the repository's `.gitignore` hides it from `git status`. Once a developer has looked at `assets/` in Finder, `assets/.DS_Store` is in the embedded roster of every build made from that tree, in the signed update `push` publishes, and served from the web root. It records the folder's file names and view settings.

`deploy` does not publish it silently: it captures ignored files too and refuses a snapshot that has any, naming each, unless `--dirty` is passed (`scripts/deploy.mjs:339`, `:422`).

Xcode leaves the file out of a bundle (its resource copy runs `builtin-copy -exclude .DS_Store`), so a hand-built app never ships one.

## The failure

```
printf Bud1 > apps/caltrain/assets/.DS_Store
cargo build --profile host-dev -p caltrain-linux --bin caltrain-linux
```

Observed, in `target/host-dev/build/caltrain-linux-*/out/compat.json` (`embedded.assets`) and in `artifacts.json` beside it:

```
… 'activities/service-alert.schema.json', 'assets/.DS_Store', 'assets/caltrain.png', 'deck/deck-chunk.mjs', …
```

Expected: no `assets/.DS_Store` in either. The compatibility id is the same with and without the file.

The listing alone, for a folder holding `mark.png`, `icons/a.png` and a `.DS_Store` in each directory:

```
printf '{"op":"names","root":"%s"}\n' "$PWD/app/assets" | target/debug/exact-filesystem
{"value":{".DS_Store":null,"icons/.DS_Store":null,"icons/a.png":null,"mark.png":null}}
```

## Verification

Ran both. That `push` and the web server take the file was read in the lines named above, not run; `deploy`'s refusal was read, not run.

## Direction

Skip `.DS_Store` where the tree is visited (`Directory::visit_files`) and have `push` list through the same reader, so the bake, the web build, the dev server and `push` agree. Refusing every dot file would be simpler, if no app is meant to ship one.

## Related

No issue, QUEUE entry or pitfall names `.DS_Store`. `game/bakeoff/garden/run.mjs:55` and `scripts/async-inputs.mjs:96` each filter it by hand for their own walks.
