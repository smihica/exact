# A cleared `flex` is not CSS's initial `0 1 auto`: a conditional class's absent arm gives `0 1 0%`, and `flex="unset"` gives `1 1 auto`

**Status:** Closed
**Resolution:** Cleared flex arms and flex="unset" restore CSS's initial 0 1 auto and intrinsic sizing.
**Systems:** contract/lower/src/values.rs (`flex_component`), contract/lower/src/class.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`flex_component` (`contract/lower/src/values.rs:1286`) projects `flex` into `flex-grow`, `flex-shrink` and `flex-basis`. Two cleared forms project to the wrong longhands.

1. A conditional class writes `none` for a row only one of its styles sets (`contract/lower/src/class.rs:61-66`), "an explicit unset the runner clears to the kernel's default". `shorthands::component` keeps that `none` for every longhand (`contract/lower/src/shorthands.rs:53-54`). `flex_component` has no arm for it, so it falls to the arms for a computed value (`values.rs:1364-1366`): grow stays `none`, shrink becomes `1` and basis becomes `"0%"`. The arm that sets no `flex` gets `flex: 0 1 0%`.

2. `flex="unset"` is read as a lone basis word (`values.rs:1304-1356`), so grow and shrink default to 1: `flex: 1 1 auto`. `unset` on a property that does not inherit is its initial value (CSS Cascade 4 §7.3), and `docs/contract-for-agents.md:4994` says "`unset` clears any row". `"initial"` is handled on the line above (`values.rs:1303`).

## The failure

```
style Grow
  flex=1
style Plain
  color="red"
component A
  state on = false
  view
    row width=300
      box testId="a" class=(on ? Grow : Plain)
        text "aaa bbb ccc ddd"
```

Observed: the box has `flex-grow` 0, `flex-shrink` 1, `flex-basis` 0% and is 28.8 wide, its min-content width, with one word on each line. `flex=(on ? 1 : none)` on the node gives the same longhands.

Expected: `0 1 auto` and 144 wide, which `class=Plain` alone gives.

`box flex="unset"` in the same row has grow 1, shrink 1, basis auto and is 300 wide. Expected `0 1 auto` and 144 wide, as `flex="initial"` gives.

## Verification

At `dd6f5dcd`, a temporary test in `contract/cli/tests` compiled, baked and booted each source, printed the box's three longhands and laid it out with the monospace kernel. It printed the longhands and widths above, and `grow=0.0 shrink=1.0 basis=Auto` for `flex-grow="unset" flex-shrink="unset" flex-basis="unset"`.

Chrome 157 lays out the same row of `<div>`s 144.5 wide for no `flex`, `flex: unset` and `flex: initial` (each computing to `0 1 auto`), and 28.9 wide for `flex: 0 1 0%`.

The lint (`contract/lower/src/lint.rs:617`) and the design server's rows (`contract/lower/src/theme/rows.rs:426`) call the same function and were not exercised.

## Direction

In `flex_component`, return `none` for every index when the value is `none`, as `shorthands::component` does, and give a literal `unset` the longhands `initial` gets.

## Related

`QUEUE.md`'s "CSS intrinsic flex-basis keywords" is about `content` and `min-content` values, not cleared forms. No issue or `docs/agent-pitfalls.md` entry names `flex` with `unset` or a conditional class. The other shorthands clear to their initial values in the absent arm: `border`, `gap`, `margin`, `text-decoration`, `columns`, `overflow` and `border-radius` were run the same way.
