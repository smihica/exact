# `lower-width-padding` never names the view's root, the box every starter writes `width="100%"` on

**Status:** Closed
**Resolution:** lower-width-padding now diagnoses full-width view roots with horizontal padding or a border under default content-box sizing.
**Systems:** contract/lower/src/warnings/width.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`lower-width-padding` names a container written `width="100%"` with horizontal padding or a
border under the default `content-box`: it is wider than its parent by the padding, "and past
the phone's right side when the parent is the screen" (`contract/lower/src/warnings/width.rs:1-9`).
The walk judges a box only when its parent is known to give the percentage its own content
width (`Parent::Fills`), and it starts the view's root with `Parent::Other` (`width.rs:200`):

```rust
walk(view, Parent::Other, false, styles, &mut seen, out);
```

So the root, whose parent is the window, is never named. `lower-scroll-grows` starts the same
kind of walk with the window as a definite parent (`warnings/scroll_bound.rs:87-99`).

The root is where the width is most often written. The app `exact new` makes
(`game/new.mjs:656`), all six patterns under `docs/patterns/` and nine recipes write the root
with `width="100%" height="100%"`, so padding or a border added to that line overflows with no
warning.

## The failure

```
component App
  view
    main testId="root" width="100%" height="100%" padding=16
      text "Hello"
```

Observed: no warning. Laid out in a 390 by 800 window the root's frame is 422 by 832. `exact
new`'s own root with `padding=16` added is the same 422 by 832.

The same box one level down is named:

```
component App
  view
    main width="100%" height="100%"
      column testId="root" width="100%" padding=16
        text "Hello"
```

`4:7 [lower-width-padding] `width="100%"` with padding is wider than its parent by the padding …`,
and its frame is 422 wide.

`apps/motion-gallery/app.contract:57` has this root (`main … width="100%" height="100%" …
padding=12`): its frame is 414 by 824 in the same window, and `contract build` prints no
`lower-width-padding` for it.

Expected: the root is named. The overflow itself is CSS's: under `content-box` a percentage
`width` is the content width (CSS 2 §10.2), and the padding is added to it.

## Verification

A temporary test compiled each source above with `contract::compile_warned` and printed the
warnings, then booted each plan in the runner, laid it out with `Offer::definite(390.0,
800.0)` and printed the root's frame. The frames and the warning above are its output. With
`box-sizing="border-box"` the root is 390 wide.

## Direction

Start the walk with `Parent::Fills`: the window gives the root its width as a block gives a
child.

## Related

`docs/agent-pitfalls.md` ("A `width="100%"` box with padding runs past its parent") says the
compiler diagnoses this and does not except the root. Nothing in `issues/` or `QUEUE.md` names
the warning's root.
