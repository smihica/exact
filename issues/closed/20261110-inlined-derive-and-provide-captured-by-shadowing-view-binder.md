# A child's derive, or a provided value, read under a `for` or `match` binder that shadows the state it reads takes the binder's value

**Status:** Closed
**Resolution:** View binders are renamed apart from component declarations, so child derives and provided values keep their defining scope.
**Systems:** contract/syntax/src/inline.rs, contract/syntax/src/inline/use_node.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`. A view's `for` or `match` binder may shadow
a state. Inlining substitutes two kinds of component-scope expression into
view positions, and neither substitution avoids such a binder.

1. A child's derive is substituted at each read
   (`contract/syntax/src/inline/use_node.rs:248-261`). The child's states are
   renamed `name#N` (`:125-133`), and its view binders are renamed by the same
   function with the same `N` (`lifted`, `contract/syntax/src/inline.rs:49`,
   called at `:741` and `:766`). A state and a binder of one name both become
   `name#N`, so a derive that reads the state, read under the binder, reads the
   binder.
2. A `provide` binding is substituted into every injecting component under it
   (`use_node.rs:72-105`). The root's binders are not renamed, so a root binder
   spelled like a state the binding reads captures it.

## The failure

```
component App
  view
    column
      Child()

component Child
  state item = "state"
  state items = ["a", "b"]
  derive label = `${item}!`
  view
    column
      text label testId="outside"
      for item in items key=item
        text label testId=`in-${item}`
```

Observed: `outside` is `state!`, `in-a` is `a!`, `in-b` is `b!`. Expected
`state!` in all three, since `label` reads the state. With `Child`'s body as
the root component, where a derive is a plan derive, all three read `state!`.
A `match o` with `case some(v)` beside `state v` does the same (`bound!` where
`state!` is expected).

```
component App
  state item = "root"
  state items = ["a", "b"]
  provide
    current = item
  view
    column
      for item in items key=item
        Row(id=item)

component Row
  props
    id: string
  inject
    current: string
  view
    text current testId=`row-${id}`
```

Observed: `row-a` is `a`, `row-b` is `b`. Expected `root` in both: a `provide`
section binds for the whole view (`docs/contract-for-agents.md:290`), outside
the `for`.

## Verification

Each program compiles with no diagnostic. Booted each on the runner and read
the texts by test id: the values above, and `state!` three times for the root
form of the first. With the shadowed name a prop (`Child(item="prop")`) the
derive reads `prop!` everywhere, and a child action that reads the state under
the binder writes `state`: those two paths are correct.

## Direction

Spell a view binder apart from a lifted declaration (a suffix of its own in
`rename_nodes`), and rename a root binder that a provided expression names. Or
refuse a view binder that shadows a name in scope, as `type-let-shadow` refuses
a `let`.

## Related

`issues/closed/20260901-child-derive-name-capture.md` fixed a child's derive
capturing a parent's name. This is the component's own binder.
