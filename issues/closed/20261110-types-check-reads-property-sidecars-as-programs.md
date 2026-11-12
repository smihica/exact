# `difftest types` fails on the corpus's `.test.contract` sidecars, which are not programs

**Status:** Closed
**Resolution:** The shared program collector excludes .test.contract sidecars, so difftest types checks corpus programs without sidecar compiler refusals.
**Systems:** semantics/difftest/src/types.rs, scripts/async.mjs (the `difftest` check)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

`difftest types` takes every `.contract` file under `semantics/corpus` as a program (`semantics/difftest/src/types.rs:445`, `crate::collect`). Eight of them are property sidecars (`corpus/properties/*.test.contract`), which hold `always`, `after` and `never refuses` blocks beside their program and no component. The corpus loader skips them (`semantics/difftest/src/lib.rs:515`); the types check does not, so the compiler refuses each, the check counts it (`types.rs:477`) and returns failure (`:553`).

The run fails whatever the two checkers say: all 497 real programs and mutants agree. The async lane's `difftest` check runs `types --seed 1 --count 100` (`scripts/async.mjs:313`), and its parser reports each `REFUSED` line as a failure of the check.

## The failure

```
$ cargo run -q -p contract-difftest -- types --count 0
REFUSED corpus/properties/clock-until.test.contract: …:1:1 [analyze-no-component] a file needs a component
REFUSED corpus/properties/cone-clock.test.contract: …
… (cone-resource, false, holds, never-then, stays-false, stays)
difftest types: 497 programs (248 mutants, 188 refused by both): 497 agree, 0 refused by Lean alone,
  0 accepted by Lean alone, 8 refused by the compiler                       (exit 1)
```

Expected: `0 refused by the compiler`, exit 0.

## Verification

The command above was run and printed these eight `REFUSED` lines and this summary, exit 1.

## Direction

Skip names ending in `.test.contract` when `types.rs` collects the corpus, as `lib.rs::corpus` does. One collector for both would keep them from drifting again.

## Related

Nothing in `issues/` or `QUEUE.md` names the types check.
