# An `action` prop given a conditional passes the use's check and is refused at the child's handler

**Status:** Closed
**Resolution:** Action bindings reject conditional choices at their supplier, preserve concrete child aliases, and keep independent child errors.
**Systems:** contract/types/src/uses.rs, contract/lower/src/handlers.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`. A use's argument is checked by type alone
(`contract/types/src/uses.rs:224-237`: `infer`, then `accepts_expr`). A
conditional between two actions has the type `action`, so `tap=flag ? a : b`
is accepted for a prop declared `tap: action`. An action prop is substituted
into the child's handler, where only a name or `name(args)` can stand
(`contract/lower/src/handlers.rs:40-49`). The refusal therefore comes from
lowering and points at the child's handler, which is written correctly.

## The failure

```
component App
  state n = 0
  state flag = true
  action a()
    n = 1
  action b()
    n = 2
  view
    column
      Child(tap=flag ? a : b)

component Child
  props
    tap: action
  view
    button press=tap testId="go"
      text "go"
```

Observed:

```
16:12 [lower-handler] a handler is an action name or `action(args)`
 16 |     button press=tap testId="go"
    |            ^^^^^
```

`press=tap` is an action name. The line to change is 10, which the message
does not mention. With `Child` in another file (`use Child from
"./child.contract"`), the refusal is reported in `child.contract` alone, a
file with nothing to fix.

Expected: a refusal at `tap=flag ? a : b` saying that an `action` prop takes
an action's name or `action(args)`. The root's own handler gets that where the
conditional is written (`press=(flag ? a : b)`):

```
9:12 [analyze-handler-shape] `press=` needs an action name or `action(args)`
```

Since `4f5cc4ab` that message goes on to name the remedy ("not a condition: write one
action that chooses"); the use's argument still reaches lowering unrefused.

## Verification

Ran `contract build` (debug) on the program above, on its two-file form and on
the root form: the three refusals as described.

## Direction

In `walk_use`, when the prop's type is `Ty::Action`, require the argument to
be an `Expr::Ident` or an `Expr::Call` (through `Expr::Typed`), and refuse
anything else there. Expansion already says this for a prop an action calls
(`syntax-call-target`, `contract/syntax/src/inline/use_node.rs:202-208`).

## Related

None in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
