# The compatibility id does not move when a data crate's path dependency changes, and target-specific code in `data/` is not refused

**Status:** Closed
**Resolution:** The data crate's identity hashes each path crate it is built from, less exact2's own, as `cargo metadata` lists them, so an edit to one moves the id; the `cfg(target_*)` refusal needs a ruling first and is issues/20261111-cfg-target-in-data-is-not-refused.md.
**Systems:** bake/src/compat.rs (data_crate), scripts/app.mjs (classifyArtifacts)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

LLP 1030 D3a puts the native data crate's identity in the compatibility id, so that a plan baked against newer native Rust never reaches a binary with the older code unannounced. The identity is the crate's source digest, which rests on a ruling (LLP 1030 §5 item 7, 2026-09-03): "bake refuses `cfg(target_*)` and native dependencies in `data/`", because "the crate already depends on `exact-runner` alone".

`data_crate` (`bake/src/compat.rs:713`) hashes the files under `<app>/data`, the workspace lockfile and the toolchain. Nothing in the bake refuses anything. Two consequences:

1. A path dependency outside `data/` is native data code the digest never reads. Five in-tree data crates have one: `apps/markdown/data`, `apps/llp/data`, `apps/harness/data` and `apps/markdown-stress/data` depend on `apps/markdown/parse`, and `apps/exact-live/data` on four other apps' data crates. An edit there changes what the binary answers and leaves the id, the stream and `dataCrate` as they were. LLP 1030 D3 says the one gap the classifier cannot see is named in its table as "same name, same shape, new native code"; `classifyArtifacts` keys that warning on `dataCrate` (`scripts/app.mjs:1383`), so it says nothing and the bundle is offered to the old cohort.
2. `cfg(target_os)` in `data/` is accepted, where the LLP says the bake refuses it.

## The failure

```
exact-bake compat apps/markdown --platform macos --json
#   id 21ca0bdb31601a00fb80e2f8144f58d9   dataCrate.tree f0afd99a6a0812cf…

# apps/markdown/parse/src/lib.rs:62, the name Contract switches on:
#   Kind::Heading => "heading",   becomes   Kind::Heading => "title",
exact-bake compat apps/markdown --platform macos --json
#   id 21ca0bdb31601a00fb80e2f8144f58d9   dataCrate.tree f0afd99a6a0812cf…   (unchanged)

# instead, appended to apps/markdown/data/src/welcome.rs:
#   #[cfg(target_os = "macos")]      pub fn only_on_mac() -> u32 { 7 }
#   #[cfg(not(target_os = "macos"))] pub fn only_on_mac() -> u32 { 8 }
exact-bake compat apps/markdown --platform macos --json
#   exit 0, id 27aa22b88cfe595f740a4cc95f493b40   (accepted)
```

Expected: the first edit moves the id, or the dependency is refused; the second is refused.

## Verification

Ran the three commands with a debug `exact-bake` (`cargo build -p exact-bake --bin exact-bake`), restoring each file after; the comments are their output. Not run: a deploy of a stored app with such a dependency. The in-tree apps that have one are all `deploy.store` `"0"` (the harness is a terminal app with no manifest), so no shipped stream is affected today.

## Direction

Hash what Cargo says the data crate is built from: its path-dependency closure from `cargo metadata`, less exact2's own crates, in place of the one directory. Implement the LLP's refusal of `cfg(target_*)` in `data/`, or amend the ruling.

## Related

QUEUE.md line 479 notes `dataCrate` is null whenever Rust is replaceable; that is the other branch. No issue names the data crate's dependencies.
