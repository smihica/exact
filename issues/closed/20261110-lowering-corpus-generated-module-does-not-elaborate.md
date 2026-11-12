# `difftest lowering-corpus` checks nothing: its Lean module fails to elaborate on `.land`

**Status:** Closed
**Resolution:** Lowering checks accept script landings and validate bodies after mutation continuations and queued sends complete.
**Systems:** semantics/difftest/src/lowering.rs, semantics/Contract/LowerCheck.lean, scripts/async.mjs (the `difftest` check)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

A corpus script follows every `tap` and `type` with a landing (`Event::InputLand`, `semantics/difftest/src/script.rs:235`), and `clock data` is `Event::Land`. Both print as the Lean term `(.land)` (`script.rs:143`), a constructor of `Observe.ScriptEvent` (`semantics/Contract/Observe.lean:287`). The lowering check writes a case's events as `List Observe.Event` (`semantics/difftest/src/lowering.rs:182`), the type `LowerCheck.check` takes (`semantics/Contract/LowerCheck.lean:201`), and `Observe.Event` (`Observe.lean:225`) has no `land`. The generated module of any batch that holds a corpus case with an input does not compile, and the command stops before it compares a single body. `difftest expansion` drops the two events before printing (`expansion.rs:93`); the lowering check does not.

`lowering` and `quick` are unaffected: generated scripts hold only taps, types and clocks (`quick`'s lowering part passed in this review). What is lost is the translation validation of the corpus programs, which the README lists (`semantics/README.md:663`) and the async lane runs (`scripts/async.mjs:312`). The lane's failure parser has no pattern for this error (`async.mjs:416`-`:422`), so it shows only as `difftest: exit 1 (see log)`, and only when no other part of the check failed.

## The failure

```
$ cargo run -q -p contract-difftest -- lowering-corpus semantics/corpus/counter.contract
difftest: …/target/difftest/lowering-corpus-0-37143.lean: lean failed:
…lowering-corpus-0-37143.lean:33:54: error(lean.unknownIdentifier): Unknown constant `Contract.Observe.Event.land`
Note: Inferred this name from the expected resulting type of `.land`:
  Observe.Event
…                                                          (exit 2)
```

Expected: the summary line `difftest lowering: N cases checked … 0 diverge`, exit 0.

## Verification

The command above and the whole `lowering-corpus` were run: both exit 2, the whole run with 40 such errors from its first batch and no summary line. A batch is one Lean module of 16 cases, so one case with an input fails the 15 beside it.

With the `(.land)` terms left out of the generated event list (a temporary edit, reverted), the whole run exits 0: `587 cases checked (0 not compiled, 16 boot refused): runs 32277 agree, 188 both fail, 0 diverge; bodies 22416 identical to Contract.Lower, 0 differ, 31 refused by it, 0 undecoded`. So the landing is the only obstacle.

## Direction

Have `LowerCheck.check` take `List Observe.ScriptEvent` and step a landing with `Observe.land`, so the configurations after a `then` or a queued send are checked too. Dropping the two events, as `expansion.rs` does, is the smaller fix and checks fewer configurations.

## Related

Nothing in `issues/` or `QUEUE.md` names `lowering-corpus`.
