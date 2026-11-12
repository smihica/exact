# A `:checked` part on a node whose `aria-checked` or `aria-selected` is a quoted `"true"` or `"false"` compiles to a binding that traps

**Status:** Closed
**Resolution:** :checked parts normalize quoted aria-checked and aria-selected boolean tokens through the property-binding conversion and bake without trapping.
**Systems:** contract/lower/src/states.rs (`fold_states`), contract/lower/src/values.rs (`aria_bool_literal`)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The compiler admits a bool ARIA state written as its WAI-ARIA token: `aria-checked="true"` is bound as `aria-checked=true` (`values::aria_bool_literal`, `contract/lower/src/values.rs:985-999`, applied at `contract/lower/src/lib.rs:1387`; `contract/cli/tests/it/aria.rs:113` asserts it for `aria-hidden` and `aria-modal`).

`fold_states` builds `:checked`'s condition from what the node writes for `checked`, `aria-checked`, `aria-selected` and `aria-pressed` (`contract/lower/src/states.rs:484-509`). It converts only `aria-pressed`'s string to a bool (lines 488-502). For `aria-checked` and `aria-selected` it takes the expression as written (line 503), so the string literal becomes the condition of each folded row: `"true" ? checked : base`. Nothing type-checks that condition, and the runner traps on it.

## The failure

```
style T
  background-color = "#ff0000"
  :checked
    background-color = "#00ff00"

component App
  view
    column
      button role="checkbox" aria-checked="true" class=T testId="a"
        text "x"
```

Observed: `contract::compile` succeeds and `contract build` exits 0. `contract::bake`, which each app's build script runs, fails with the lint `bake-binding-trap`:

```
t=0 view 2 (site 1): background-color trapped Trap(TypeMismatch { pc: 5, op: JumpIfFalse }); unset
```

which names the node and the bytecode, not the `aria-checked` value. Booted without the bake, the row is left unset: the background is transparent, neither the green nor the red. `aria-checked="false"`, `aria-selected="true"` and `aria-selected="false"` fail the same way, with the part in a `style` or under the node.

Expected: green for `"true"` and red for `"false"`, as `aria-checked=true` and `aria-checked=false` give, and as `aria-pressed="true"` and `aria-pressed="false"` already give.

## Verification

At `dd6f5dcd`, a temporary test in `contract/cli/tests` compiled, baked and booted the source above with each attribute in turn and printed the node's computed `background-color`. It printed `Lint { id: "bake-binding-trap", … }` with the message above for the four quoted forms; green for `aria-checked=true`, `aria-selected=true` and `aria-pressed="true"`; red for `aria-checked=false` and `aria-pressed="false"`. Booting the unbaked plan printed a transparent background and the journal line `t=0 view 2 (site 1): background-color trapped Trap(TypeMismatch { pc: 5, op: JumpIfFalse }); unset`. `contract build` on the file exited 0.

## Direction

In `fold_states`, convert a literal `"true"` or `"false"` on `aria-checked` and `aria-selected` to the bool, as the `aria-pressed` arm does and as `values::aria_bool_literal` does where the prop is bound.

## Related

No issue, `QUEUE.md` entry or `docs/agent-pitfalls.md` entry covers a `:checked` part over a quoted ARIA token. Only the literal reaches this: a computed string (`aria-checked=(on ? "true" : "false")`) is refused as `lower-attr-type`.
