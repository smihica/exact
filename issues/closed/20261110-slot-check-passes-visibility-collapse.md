# The slot check passes `visibility="collapse"`, which hides the fill on the terminal, Linux and web hosts

**Status:** Closed
**Resolution:** The shared slot checker refuses visibility: collapse as hiding the fill, alongside hidden.
**Systems:** contract/lower/src/anatomy/slot.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

LLP 1139 D4 refuses a row on a library component's slot chain that hides what fills the slot, and a value the check cannot judge is "refused, never assumed safe". `denied` (`contract/lower/src/anatomy/slot.rs:137-155`) refuses `visibility: hidden` and lists `collapse` beside `visible` as a value that shows the box (line 145).

`collapse` is a value of the kernel's `Visibility` enum (`kernel/tables/schema.json:2474-2481`), so an author can write it, and `visibility` is inherited, so the fill computes it too. What it does depends on the host:

- The terminal host and the Linux painter draw a node only when its computed `visibility` is `visible` (`host/terminal/src/paint.rs:197-198`, `host/linux/src/paint.rs:1472-1479`), so `collapse` hides it.
- A browser treats `collapse` on a box that is no table row or column as `hidden` (CSS 2 §11.2); Chrome does so for a flex item too.
- The Apple hosts hide only the string `hidden` (`host/apple/Sources/ExactKit/CssVisibility.swift:13`), so there the fill shows.

## The failure

A private library package `@acme/lib` with a design, whose entry is:

```contract
component Card
  slot
  view
    column padding="lib.pad" visibility="collapse"
      children
```

used by an app as `Card()` with a `text "fill"` child.

Observed: it compiles. With `visibility="hidden"` the same component is refused:

```
[lower-anatomy-slot] a `column` on the slot's chain in `Card`: `visibility: hidden` hides it, and the slot's fill would not show (LLP 1139 D4)
```

Expected: `collapse` is refused the same way.

## Verification

At `dd6f5dcd` a temporary test in `contract/cli/tests` wrote the package and the app to a temporary directory and compiled the app: `visible` compiled, `hidden` printed the refusal above, `collapse` compiled.

What `collapse` does was run on the terminal host. For

```contract
component App
  view
    column
      text "above"
      column visibility="collapse"
        text "FILL"
      text "below"
```

`exact-terminal <file> --size 20x5 print` printed `above`, an empty line and `below`, the same as with `hidden`; with `visible` the middle line was `FILL`. In headless Chrome 157 a `<div>` inside a flex column with `visibility: collapse` computes `collapse` and answers false to `checkVisibility({visibilityProperty: true})`, as under `hidden`. The Linux and Apple statements above are from reading the cited lines, not from a run.

## Direction

Refuse `collapse` in `denied` as `hidden` is refused; `visible` and the CSS-wide keywords stay the values that show. That the Apple hosts draw a `collapse` box the others hide is a separate disagreement, in `cssVisibilityHidden`.

## Related

`contract/cli/tests/it/anatomy_slot.rs` covers `hidden` only. No issue, `QUEUE.md` entry or `docs/agent-pitfalls.md` entry mentions `visibility: collapse`.
