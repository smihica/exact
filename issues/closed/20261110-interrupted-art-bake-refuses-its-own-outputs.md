# An art bake that stops between its outputs and its manifest is refused by every later bake as an authored-asset collision

**Status:** Closed
**Resolution:** Art bakes record current and pending output digests before writing and serialize concurrent bakes, so interrupted bakes can resume without adopting authored assets.
**Systems:** game/bake/src/lib.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`bake_art` tells its own files in `assets/` from an author's by `.baked-assets.json`, the digest of each output it last wrote. It reads the manifest, encodes the art, and then refuses any existing file whose digest is not the recorded one (`game/bake/src/lib.rs:648-663` at `dd6f5dcd`):

```rust
Ok(current) if previous.get(name) == Some(&digest(&current)) => {}
Ok(_) => return Err(format!("generated asset `{name}` collides with an authored asset")),
```

It writes every output (`:664-668`) and the manifest last (`:681-684`). A bake that stops in between (a build interrupted or timed out, a failed write) leaves outputs the manifest does not describe. Every later bake refuses them, naming one file and blaming an authored asset that does not exist, until the named files are deleted by hand.

Two bakes of one game at once fail the same way, for one build: the bake that read the manifest before the other finished finds the other's new outputs. Each host build runs the bake from its GPU crate's build script. `game/prove.mjs:177-182` runs its hosts' builds one after another; builds started separately are not serialized.

The refusal itself is intended: `content_equal_without_ownership_never_adopts_authored_output` (`game/bake/tests/outputs.rs:283`) pins that a file is never adopted because its bytes equal the output. The defect is that the bake puts its own files in that state.

## The failure

A game whose `art/` holds eight 1023 by 1023 PNGs, baked once with `exact-game-bake --art <game>`. Change the PNGs, start the bake again and kill it when its first output lands:

```
outputs rewritten before the kill: 1 of 24; manifest unchanged
next bake: generated asset `s00.astc.tex` collides with an authored asset
```

Every later bake prints that line until `assets/s00.astc.tex` is deleted. A first bake killed the same way leaves one output and no manifest, and is refused the same way. Expected: the bake that wrote those files can finish its own work.

## Verification

Ran the bake's binary on a game made outside the repository and sent it SIGKILL as soon as its first output changed, for an update and for a first bake. Both printed the lines above; deleting the named file let the update finish. Then started two bakes of freshly changed art 0 to 190 ms apart (one bake takes 0.3 s): in 16 of 20 rounds the second exited 1 with the same message, the first always exited 0, and a bake alone afterwards passed.

## Direction

Record ownership before the first write: write the manifest with, for each name, the digest it had and the digest it is about to have, read it again just before the check, accept either digest, and write the final manifest after the last output. The pinned test still passes: a manifest that is missing or edited by hand records no pending digest.

## Related

No entry in `issues/`, `QUEUE.md` or `docs/game-pitfalls.md` names `.baked-assets.json` or this message.
