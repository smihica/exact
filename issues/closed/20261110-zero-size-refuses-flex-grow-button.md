# `lower-zero-size` refuses an empty button sized by `flex-grow` or `flex-basis`

**Status:** Closed
**Resolution:** Empty buttons and links accept flex-grow and flex-basis as sizing attributes; bake still refuses zero area.
**Systems:** contract/lower/src/controls.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`check_zero_size` (`contract/lower/src/controls.rs:1108-1149`) refuses a `button` or `link` with no children unless one of a fixed list of attributes is present (`SIZES`, 1114-1125) or it is absolutely positioned with insets. The list has `flex` but not its longhands `flex-grow` and `flex-basis`, so a childless button that takes its size from either is refused with a message that says it has zero area. It does not: `flex-grow=1` takes the share of the main axis that `flex=1` takes, and `flex-basis=40` is a 40-point main size.

## The failure

```contract
component App
  action go
  view
    row height=44
      button press=go aria-label="Close" flex-grow=1
```

Observed:

```
5:7 [lower-zero-size] `button` has no children and no size, so it has zero area and nothing to press: give it children or a size
```

Expected: it compiles, as the same button with `flex=1` does. In the 44-point row the button is stretched to the row's height (CSS Flexbox 1 §8.3, `align-items: stretch`) and grows to its width (§7.2), so it has area. `flex-basis=40` is refused the same way.

## Verification

At `dd6f5dcd`, `contract build` on the source above printed the refusal, the same for `flex-basis=40`, and compiled it with `flex=1`. A temporary test in `contract/cli/tests` then added `padding=0` (in `SIZES`, and no change to layout) to step past the check, baked each plan and laid it out in a 300-point-wide column:

| Button | Frame | `contract::bake` |
| --- | --- | --- |
| `flex=1` | 300 × 44 | passes |
| `flex-grow=1` | 300 × 44 | passes |
| `flex-basis=40` | 40 × 44 | passes |

## Direction

Add `flex-grow` and `flex-basis` to `SIZES`. The check is meant to be conservative ("only the case nothing on the path can bound is refused", `contract/lower/src/nodes.rs:49-51`), and a size the bake's `bake-zero-size` finds missing is still refused there.

## Related

No issue or `QUEUE.md` entry names `lower-zero-size`. `docs/agent-pitfalls.md` has one entry for it, about a `link` whose title is positional, which is a different input.
