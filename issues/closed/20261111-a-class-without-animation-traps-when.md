# A class without animation traps when its alternative sets animation longhands

**Status:** Closed
**Resolution:** Folding animation longhands reads a class choice's `none` arm as the longhand's initial value, so the side without an animation bakes as none.
**Systems:** contract/lower/src/svg.rs
**Severity:** P2
**Author:** Codex (GPT-6 Astra), from the fix lanes of the 2026-11-10 project review
**Date:** 2026-11-11

## What is wrong

At `09cb236d64007f51a668e38926fda6f64dfb6fb5`, animation longhand folding inserts computed class rows into a string template (`contract/lower/src/svg.rs:738`, `:742`). A class branch that does not supply a longhand can yield an absent value. The generated animation binding tries to format it as a string and traps during the bake.

## The failure

```contract
keyframes spin
  from rotate=0
  to rotate=360
style Idle
  opacity=1
style Active
  animation-name="spin"
  animation-duration="200ms"
component App
  state active = false
  view
    view class=(active ? Active : Idle)
```

Expected: the Idle branch bakes with no animation. Observed: compilation succeeds, but the bake fails with `bake-binding-trap`. Starting with `active = true` bakes successfully.

## Verification

Ran `CARGO_BUILD_JOBS=5 cargo test -p contract --test review_findings -- --nocapture`. The temporary test called `contract::compile` and `contract::bake(plan, ())` for both initial states. It printed:

```
active=false: Lint { id: "bake-binding-trap", message: "t=0 view 1 (site 0): animation trapped Trap(TypeMismatch { pc: 21, op: Call }); unset", site: Some(NodesId(0)) }
active=true: bake OK
```

The assertion that both bakes succeed failed. The temporary test was removed.

## Direction

Resolve missing class longhands to their CSS initial values before building the shorthand, or keep the absent branch outside the animation template. Test booting and switching between both branches.

## Related

`issues/closed/20261110-checked-part-traps-on-aria-string-token.md` also reported a bake binding trap, but its cause was a string condition in an ARIA part. This is an absent animation longhand in a class branch. No matching issue or QUEUE entry was found.
