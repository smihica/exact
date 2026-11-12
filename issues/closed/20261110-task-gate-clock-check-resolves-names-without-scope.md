# The task-gate clock check misses a clock read through a derive named like its `fn`, and refuses a parameter spelled like a clock derive

**Status:** Closed
**Resolution:** Task and sequence clock checks distinguish derives from functions and honor lexical bindings.
**Systems:** contract/analyze/src/gates.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`. LLP 1092 D9 refuses "a gate or key that
reads `now()`, directly, through a derive, or through a `fn` body"
(`analyze-task-gate-clock`). `clock` (`contract/analyze/src/gates.rs:79-139`)
decides it for a task's and a sequence's gate, and resolves names by spelling
alone, in two ways that give a wrong verdict.

1. One `seen` set holds derive names and `fn` names (`:86`, `:104`). A derive
   may share its `fn`'s name (`docs/contract-for-agents.md:660`: "Beside `fn
   pick`, `derive pick = pick(1)`"). Visiting the derive inserts the name, so
   the call inside it finds the name already seen and never opens the `fn`'s
   body.
2. `Expr::Ident` is looked up among the root's derives (`:110`, `:89`) with no
   record of the binders in force. An arrow parameter, a `match` binder or a
   `fn`'s parameter spelled like a derive is read as that derive.

## The failure

Missed. This builds, though the gate reads the clock:

```
fn elapsed(start: number): number = performanceNow() - start

component App
  state start = 0
  state toast = "x"
  derive elapsed = elapsed(start)
  action clear()
    toast = ""
  task hide when elapsed > 5000
    after(1000, clear)
  view
    text toast
```

With the derive named `since` the same file is refused: "`task hide`'s gate
reads `performanceNow()` through `since`".

Refused wrongly:

```
fn late(elapsed: number): bool = elapsed > 5000

component App
  state start = 0
  state shownFor = 0
  state toast = "x"
  derive elapsed = performanceNow() - start
  action clear()
    toast = ""
  task hide when late(shownFor)
    after(1000, clear)
  view
    text `${toast} ${elapsed}`
```

```
10:18 [analyze-task-gate-clock] `task hide`'s gate reads `performanceNow()` through `fn late`. …
```

The gate reads the state `shownFor`; `elapsed` in `late` is its parameter.
With the parameter named `ms` the file builds. `task hide when xs.filter(t =>
t > 1).length > 0` beside `derive t = performanceNow() - start` is refused the
same way, "through `t`".

## Verification

Ran `contract build` (debug) on the two programs, on each one's renamed twin
and on the arrow form: the five verdicts above. Not run: the plan of the first
program, to watch the missed gate fire late.

## Direction

Carry the bound names down the walk as `properties.rs`'s `Cone` does
(`contract/analyze/src/properties.rs:74-158`: a `bound` set, and `seen` keyed
by `("derive" | "fn", name)`), or share that walk.

## Related

`issues/20261110-resource-clock-warnings-cubic-in-fns.md` is the cost of the
sibling pass in `lib.rs`, not this verdict.
