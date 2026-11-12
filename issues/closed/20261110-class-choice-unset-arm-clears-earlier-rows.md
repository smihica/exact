# The absent arm of `class=(cond ? A : B)` clears a row the tag or the chosen style already set

**Status:** Closed
**Resolution:** A class choice pairs attributes that write one row under different names row by row, and a side that sets no row takes the value the node has unwritten (its tag's fixed row, a link's colour) instead of `none`.
**Systems:** contract/lower/src/class.rs, contract/lower/src/nodes.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`class=(cond ? A : B)` pairs the two styles' rows by attribute name (`contract/lower/src/class.rs:57-85`). A name only one style writes becomes `cond ? value : none`, and `none` unsets the row to the kernel's default (`class.rs:18-22`). The node's bindings are made in order and the last binding of a style row wins (`contract/lower/src/nodes.rs:793-820`), so the `none` arm replaces whatever bound that row earlier on the node. Two things the author did not clear are cleared:

1. **A row the tag fixes.** `row` and `column` bind `display` and `flex-direction` first (`nodes.rs:432-434`, `contract/lower/src/tags.rs:70-80`).
2. **A row the chosen style writes under another name.** A shorthand and its longhand are two names for one row, so each gets a `none` arm and the later one wins.

In both, the choice disagrees with the plain `class=` of the style it selects.

## The failures

```contract
style Wide
  flex-direction="row"
style Narrow
  gap=0
component App
  state wide = false
  view
    column width=400 height=800
      column class=(wide ? Wide : Narrow)
        view width=50 height=20
        view width=50 height=20
```

The second child is at (50, 0): the column is a row while `wide` is false, because `flex-direction = wide ? "row" : none` unsets the tag's `column`. With `class=Narrow` it is at (0, 20). In the same way `row class=(hidden ? Hidden : Shown)`, where `Hidden` writes `display="none"`, is a block while shown: its second child is at (0, 20), and at (50, 0) with `class=Shown`.

With `style A` writing `padding=10`, `style B` writing `padding-top=4` and `on = true`, the resolved padding of the node (left, top, right, bottom) is:

| class | observed | expected |
| --- | --- | --- |
| `A` | 10, 10, 10, 10 | |
| `B` | 0, 4, 0, 0 | |
| `(on ? A : B)` | 10, **0**, 10, 10 | 10, 10, 10, 10 |
| `(on ? B : A)` | 0, **0**, 0, 0 | 0, 4, 0, 0 |

The chosen style's value is lost the same way for `margin` and `margin-top`, `inset` and `top`, `gap` and `row-gap`, `border` and `border-color`, `border-radius` and `border-top-left-radius`, `overflow` and `overflow-y`, and `flex` and `flex-grow`.

## Verification

At `dd6f5dcd` a temporary test in `contract/cli/tests` compiled and booted each plan, laid it out at 400 by 800 and read the children's frames, `Kernel::resolved_padding` and the node's style. The results are the ones above; each pair in the last paragraph was run.

## Direction

An absent arm should leave the row as the node has it without that attribute, not unset it. Pair the two styles by the rows they write, with shorthands expanded (`anatomy::surviving`, `contract/lower/src/anatomy.rs:108-123`, already compares by longhand), and where a style writes nothing for a row use the tag's fixed value (`implied`, `nodes.rs:380-393`, gathers these for a state part's base), `none` only where there is none.

## Related

- `contract/corpus/styles.contract` pins the kernel's default for `padding` on a `button`: one name, and a row the tag does not fix. That stays right.
- `QUEUE.md`, "A popover's default surface: two cases it leaves as today", case (2): the same `none` arm counts as authored and blocks a default surface.
- An absent arm through `flex` alone ends as `flex-basis: 0%` (`class=B` gives `auto`). That is `values::flex_component` projecting `none`, a separate fix.
