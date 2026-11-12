# `difftest expansion` reports every case that commits as a divergence: only one side prints `ran` lines

**Status:** Closed
**Resolution:** Expansion checks compare outcomes and configurations without commit traces, and typing prefers input over change.
**Systems:** semantics/difftest/src/expansion.rs, semantics/Contract/CompSem.lean, scripts/async.mjs (the `difftest` check)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

`difftest expansion` compares the flat semantics of Rust's expansion with the component-level semantics of the unexpanded file, line by line (`semantics/Contract/ExpandCheck.lean:141`). The flat side is `Observe.run`, which prints a `ran <kind> <action> <outcome>` line for each attempted commit (`semantics/Contract/Observe.lean:338`, `runScript`, through `checkedLines`). The component side is `CompSem.crun`, which calls `Observe.lines` without the commits (`semantics/Contract/CompSem.lean:844`) and prints none. `comparable` (`semantics/difftest/src/expansion.rs:136`) drops `#` lines and lifted slots and keeps `ran` lines, so the first step that runs a handler, a task or a `then` differs.

So half (b) of the check, the comparison of meanings, cannot pass on a script that commits, and a real disagreement after the first `ran` line is never reached. The README names this check as the only cover for what the expansion proofs leave out (`semantics/README.md:777`). The async lane runs `expansion semantics/corpus --seed 1 --count 200` (`scripts/async.mjs:314`). The JS comparison strips `ran` lines from both sides for the same reason (`semantics/difftest/src/js.rs:278`).

## The failure

```
$ cargo run -q -p contract-difftest -- expansion semantics/corpus/counter.contract
DIVERGE corpus/counter.contract: increments at line 11:
  flat: ran handler add ok
  comp: slot count n4018000000000000
difftest expansion: 1 cases: 0 agree, 0 expand differently, 1 mean differently, …   (exit 1)

$ cargo run -q -p contract-difftest -- expansion --seed 1 --count 3
DIVERGE gen-1 at line 31:  flat: ran handler a0 ok  comp: slot s0 …
DIVERGE gen-2 at line 40:  flat: ran handler a1 ok  comp: slot s0 …
DIVERGE gen-3 at line 44:  flat: ran handler a0 ok  comp: slot nav …
difftest expansion: 3 cases: 0 agree, … 3 mean differently, …                      (exit 1)
```

Expected: `1 agree` and `3 agree`, exit 0.

## Verification

Both commands were run and printed the above. `expansion semantics/corpus/actions semantics/corpus/views` gave 47 cases: 4 agree, 43 mean differently, each first differing at a `flat: ran …` line.

With `ran ` lines also dropped in `comparable` (a temporary edit, reverted), `expansion semantics/corpus --seed 1 --count 60` gave 636 cases: 634 agree, 2 mean differently. The two (`corpus/edge/input-handler-not-change.contract`, `corpus/properties/holds.contract`) are a second mismatch of the same kind: `Observe.runScript` sends a `type` to the element's `input` handler when it has one (`Observe.lean:349`), and `CompSem.cstep` always dispatches `change` (`CompSem.lean:828`), so the component side refuses or runs the other handler.

## Direction

Drop `ran ` lines in `comparable`, as `js.rs` does, so the configurations are compared again, and choose `input` over `change` in `cstep` as `runScript` does. Printing the commits from `CompSem.crun` would compare attribution too, and needs a traced `cstep`.

## Related

Nothing in `issues/` or `QUEUE.md` names the expansion check. `difftest lowering-corpus` fails from the same change to the script vocabulary (its own issue).
