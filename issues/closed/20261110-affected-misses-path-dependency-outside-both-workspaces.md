# `affected.mjs` selects no suite for a change to `vendor/rapier3d`, which the game's physics crate builds

**Status:** Closed
**Resolution:** Resolved metadata for both workspaces makes Rapier changes select the physics tests and the async game lane.
**Systems:** scripts/affected.mjs, scripts/async.mjs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Repo` reads the root workspace with resolved `cargo metadata` and the game workspace with
`--no-deps` (`scripts/affected.mjs:220-225`). The comment says the game's packages are read "from
their manifests alone (`--no-deps`), their path dependencies with them", and `#addWorkspace` does
record each path dependency's name as an edge (`:249-251`). But `--no-deps` lists only the
workspace's members in `packages`, so a path dependency that is a member of neither workspace has
no package record, and its node expands to nothing (`:392-393`).

`game/physics/Cargo.toml:11` depends on `rapier3d = { path = "../../vendor/rapier3d" }`, which is
in neither workspace. It is the one dependency edge of 266 packages with no record. The 220 files
under `vendor/rapier3d` are therefore inputs of no Cargo suite. The directory holds Exact's own
patch to Rapier: `fe729216` changed `vendor/rapier3d/src/dynamics/rigid_body_components_more.rs`.

## The failure

```
echo '// x' >> vendor/rapier3d/src/lib.rs
bun scripts/affected.mjs
bun scripts/affected.mjs --why cargo:exact-game-physics
```

Observed:

```
1 changed files select 4 of 598 suites: Cargo packages 0 of 84, Bun files 4 of 431, …
cargo:exact-game-physics: not selected (no changed file is among its inputs)
```

The four Bun files are the ones that read the whole checkout. Expected:
`cargo:exact-game-physics` at least, since that crate compiles the file.

The async lane runs its game checks only when a game suite is selected
(`scripts/async.mjs:146`, `:198`), so a commit that changes only the patch runs no game test. The
lane runs every suite at least daily (`:540`), so a break is found by the next full run: up to a
day late, and without the commit that made it. `affected.mjs --run` on a branch runs nothing for
the change.

## Verification

Ran the three commands above in a worktree at `dd6f5dcd`; the output is as shown. The same edit to
`game/physics/src/lib.rs` selects `cargo test --manifest-path game/Cargo.toml --tests
--no-fail-fast -p exact-game-physics`. `laneSelection(['vendor/rapier3d/src/lib.rs'])`
(`scripts/async.mjs`) returns `game: false`, and `game: true` for `game/physics/src/lib.rs`.
`cargo metadata --no-deps --manifest-path game/Cargo.toml` lists seven packages, none of them
`rapier3d`, and names it only as `exact-game-physics`'s dependency with its `path`. The async lane
itself was not run.

## Direction

Give a path dependency that no workspace lists a package record from its `path` (its Rust and
manifest, as any dependency's), or read the game workspace resolved as the root's is.

## Related

`QUEUE.md`, "`affected.mjs --run` stops at the first failing command", is about running, not
selection. Nothing in `issues/` or `docs/agent-pitfalls.md` names `rapier3d` or this rule.
`docs/testing.md` says a read that nothing names escapes the selection; this one is named in a
manifest.
