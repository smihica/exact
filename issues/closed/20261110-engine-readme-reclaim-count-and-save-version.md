# The engine README says presentation pages are reclaimed after two empty presents (it is 60), and save.rs calls the save v5 (it is v6)

**Status:** Closed
**Resolution:** The engine README states the 60-present reclamation threshold, and save.rs identifies EXGAME v6.
**Systems:** game/engine/README.md, game/engine/src/world/save.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Two sentences at commit `dd6f5dcd` disagree with the code they describe.

1. `game/engine/README.md:478-479` says "Presentation payload pages are reused, then
   reclaimed after two consecutive empty presents." The storage reclaims a page after 60
   (`game/engine/src/storage/raw.rs:402-414`, `if self.empty_presents[i] >= 60`). The other
   statements of the rule say 60: the doc comment on `World::presentation_bytes`
   (`game/engine/src/world/presentation.rs:9`), `game/render/README.md:1095`, LLP 1046.013
   ("still empty after 60 consecutive presents") and the test
   `retained_pages_reclaim_after_sixty_empty_presents` (`game/engine/tests/node_offsets.rs:76`).

2. `game/engine/src/world/save.rs:1` opens "The EXGAME v5 save". The format written and
   read is v6 (`MAGIC`, `game/engine/src/world.rs:259`), and `saved_payload` refuses a v5
   file by name (`world.rs:1171-1174`). The same README says v6 (`README.md:519`).

## The failure

An author sizing memory from the README expects `World::presentation_bytes()` to fall two
presents after a crowd leaves the screen; it falls after 60 (one second of presents at
60 Hz). A reader of `save.rs` looking for the v5 layout finds the v6 one.

## Verification

`cargo test -p exact-game --test node_offsets retained_pages` (in `game/`) passes: it holds
`presentation_bytes()` unchanged through 60 empty presents and sees it fall on the next.
A temporary test printed a world save's first eight bytes, `EXGAME\0\x06`, and loading the
same bytes with the last one set to 5 returned `restore refused: EXGAME v5 predates v6
tableInputs`.

## Direction

Change "two" to "60" in the README and "v5" to "v6" in the module comment.

## Related

None in `issues/`, QUEUE or the pitfalls files.
