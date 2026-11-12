# A value prop named like a function the component calls replaces that call's head

**Status:** Closed
**Resolution:** Value props and injects replace value reads only, preserving standard and app function calls in views and actions.
**Systems:** contract/syntax/src/inline/use_node.rs, contract/syntax/src/inline/subst.rs, contract/types/src/component.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`. Inlining substitutes a prop into a call's
head as well as into value reads (`contract/syntax/src/inline/subst.rs:215-228`).
Names that can only be values are exempt through `value_only`, but `inline_use`
puts only the child's states, derives and device facts there
(`contract/syntax/src/inline/use_node.rs:141-152`). A prop or inject with a
value type (`min: number`) is not in it, so a call the child makes to a
function of that name has its head replaced by the name the use passes, after
the child's own type check resolved the call to the function.

`type-prop-hides-fn` refuses this clash for an app `fn` only (`hides_fn`,
`contract/types/src/component.rs:533-535`), and `expression_call` (`:566`)
does not look in action bodies. The standard functions (`min`, `max`, `first`,
`round`, …) and every action body are uncovered. No document reserves these
names: `docs/contract-grammar.md:72` reserves sixteen words, none of them a
function, and `docs/contract-for-agents.md:653` describes the refusal for an
app `fn`.

## The failure

A wrong value:

```
component App
  state max = 3
  view
    column
      Child(min=max)

component Child
  props
    min: number
  view
    text `${min(4, 9)} ${min}` testId="a"
```

It compiles and the text reads `9 3`: `min(4, 9)` became `max(4, 9)`. Expected
`4 3`, which `Child(min=3)` gives, and which the same line gives in the root
beside `state min = 3`.

More often the result is a refusal that names the parent's state. A clamp:

```
component App
  state lo = 0
  state hi = 10
  state v = 5
  view
    column
      Meter(min=lo, max=hi, value=v)

component Meter
  props
    min: number
    max: number
    value: number
  view
    text `${max(min, min(max, value))}`
```

```
15:13 [type-unknown-function] `hi` is not in the stdlib roster and is not an action; data comes from a `resource`
```

With `Meter(min=0, max=10, value=v)` the component compiles, so the first use
that passes a name breaks it.

In an action the prop is a hidden parameter, so every use fails, an app `fn`
included. In a child used as `Stepper(min=0, max=10)`:

```
  action up()
    value = min(max, value + 1)
```

```
13:13 [type-unknown-function] `@capture:1:0` is not in the stdlib roster and is not an action; data comes from a `resource`
```

## Verification

Ran `contract build` (debug) on each program above: the two refusals are as
printed. Booted the first program on the runner and read the text: `9 3`;
`4 3` with `Child(min=3)` and with the root form. A child with prop
`label: string`, an app `fn label` and `shown = label(3)` in its action gets
the `@capture:1:0` message, not `type-prop-hides-fn`.

## Direction

Add every prop and inject whose declared type is not `action` to `value_names`
in `inline_use`: such a name can never be called, so its replacement belongs to
value reads only, in views and action bodies alike. `hides_fn` then has only
the `action` prop to refuse.

## Related

`issues/closed/20260901-child-action-parameter-capture.md` is the reverse
capture (a prop replacing an action's parameter).
