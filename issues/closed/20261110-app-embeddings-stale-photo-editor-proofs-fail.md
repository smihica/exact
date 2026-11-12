# Three app embeddings under semantics/Apps are stale, and Photo Editor's `keeps_ready` fails on the app as it is

**Status:** Closed
**Resolution:** All four app embeddings match their sources, and Photo Editor's readiness proof covers every current action.
**Systems:** semantics/Apps, semantics/Apps/Proofs/PhotoEditor.lean, semantics/README.md, scripts/async.mjs (the `difftest` check)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

`semantics/README.md:524` says "An app is verified against its embedding, so the proofs are about the source as it is", and `:533` that `difftest apps` "fails when the source moves and the embedding did not". Three of the four embeddings are not what `contract lean` makes of their apps now (`semantics/difftest/src/main.rs:497`, `run_apps`):

- `Apps/TypeTour.lean` and `Apps/VideoPlayer.lean` differ only inside `view`, in presentation attributes (colours are now `-exact-label` and the like; `color-scheme` was added). Their states, actions and tasks are unchanged, and their proofs still check after regeneration.
- `Apps/PhotoEditor.lean` embeds a program of 5 states and 5 actions with no routes. The app (`apps/photo-editor/app.contract`, 324 lines) now has a router, a locale slot, mutations, derives and 26 actions. The three Photo Editor rows of the README's table (`:594`-`:596`: `ready_stays`, `rotate_only_turns`, `reset_spec`) are theorems about a program that is no longer the app. Over the regenerated embedding `keeps_ready` fails, and `ready_stays` rests on it; the other two elaborate as written.

The async lane's `difftest` check starts with `apps` (`scripts/async.mjs:312`), so that check fails at this commit wherever it runs.

## The failure

```
$ cargo run -q -p contract-difftest -- apps
STALE …/semantics/Apps/TypeTour.lean: not what `contract lean apps/typetour/app.contract --name typeTour` makes; …
STALE …/semantics/Apps/PhotoEditor.lean: …
STALE …/semantics/Apps/VideoPlayer.lean: …
difftest: 4 app embeddings, 3 stale        (exit 1)

$ cargo run -q -p contract-difftest -- apps --write && (cd semantics && lake build Apps)
✖ Building Apps.Proofs.PhotoEditor
error: Apps/Proofs/PhotoEditor.lean:20:41: Tactic `subst` failed: invalid equality proof …
error: build failed
```

Expected: 0 stale, and `lake build` green on the regenerated embeddings.

## Verification

Both commands were run and printed the above; `semantics/Apps` was restored afterwards. The error at `PhotoEditor.lean:20:41` is the build's only one: `Apps.Proofs.TypeTour` and `Apps.Proofs.VideoPlayer` built over their regenerated embeddings, and `Apps.Proofs.UpdateLab` was not stale. Before regeneration `lake build` is green (2 min 12 s cold). Not run: the async lane.

## Direction

Commit the regenerated `TypeTour.lean` and `VideoPlayer.lean`. For Photo Editor, re-prove `keeps_ready` over the 26 actions (its `rcases` names five; only `loaded` writes `ready`, `apps/photo-editor/app.contract:128`) and commit the regenerated embedding, or drop the app from `APPS` and from the README's table.

## Related

`QUEUE.md`, "Photo Editor: camera and photo-library sends are outside the differential oracle", is about `contract verify` on the app: its module sends are answered differently by the runner and the semantics. It bears on the choice above, since the regenerated embedding holds those sends. No issue or queue entry names the stale embeddings.
