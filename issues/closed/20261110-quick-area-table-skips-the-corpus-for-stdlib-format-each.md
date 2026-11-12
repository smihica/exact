# `difftest quick` skips the corpus that tests `runner/src/stdlib.rs`, `format.rs` and `each.rs`

**Status:** Closed
**Resolution:** Quick checks the whole corpus for stdlib changes and includes formatting and resource cases for format and each changes.
**Systems:** semantics/difftest/src/quick.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

`quick` picks corpus directories from words in each changed file's name (`AREAS`, `semantics/difftest/src/quick.rs:25`), and runs the whole corpus only when some changed file matches no entry (`corpus_files`, `:183`). Three entries point a file away from the programs that test it:

- `stdlib` maps to `fns/` (`:45`). `runner/src/stdlib.rs` is the roster (`trim`, `parseNumber`, `calendarDiff`, `max`, `at`, `t`). `corpus/fns/` holds three programs about authored `fn`s; the roster's tests are in `text/`, `numbers/`, `lists/`, `localized/` and `typescript/`.
- `format` maps to `numbers/` (`:37`). `runner/src/format.rs` and `semantics/Contract/Format.lean` are tested by `corpus/format/` (eight programs), which no entry names.
- `each` maps to `rows/` and `lists/` (`:28`). `runner/src/runner/each.rs` is a resource over a list, tested by `corpus/resources/each.contract`.

AGENTS.md says to run `quick` before landing a change under `runner/` and that it "checks the semantics on what you changed".

## The failure

In `runner/src/stdlib.rs:669` (`calendar_diff`, years) change `<` to `<=`, so an age is one short on the birthday, and run quick:

```
$ cargo run -q -p contract-difftest -- quick
quick: corpus, 2 file(s), 6 case(s) (0.5s)
quick: random --seed 18207971833852344912 --count 40 (2.5s)
quick: lowering --seed 18207971833852344912 --count 8 (1.2s)
quick: agreed in 4.6s                                  (exit 0)

$ cargo run -q -p contract-difftest -- corpus semantics/corpus/text/calendar-diff.contract
EXPECT corpus/text/calendar-diff.contract: values: … Text("leap", "0 1 4 11") does not hold …
DIVERGE corpus/text/calendar-diff.contract: values at line 4:
  rust: derive leap "0 1 3 11"
  lean: derive leap "0 1 4 11"                         (exit 1)
```

Expected: quick runs `corpus/text/` for a change to the roster and fails.

## Verification

Both commands were run with the one-character edit, which was then reverted. The two files quick ran were `corpus/fns/` and `counter.contract`.

With only a comment appended to `runner/src/format.rs`, quick printed `corpus, 2 file(s), 30 case(s)`: `numbers/` and `counter.contract`, none of `format/`'s 12 cases. With one appended to `runner/src/runner/each.rs`, it printed `corpus, 3 file(s), 35 case(s)`: `rows/`, `lists/` and `counter.contract`, not `resources/each.contract`. Both files were restored.

## Direction

Drop `stdlib` from the `fns` entry (an unmatched name runs the whole corpus: 587 cases, 8 s here, 3 s with Lean's observations cached), add `format` to the `format` entry's directories, and `resources` to `each`'s.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names quick's selection.
