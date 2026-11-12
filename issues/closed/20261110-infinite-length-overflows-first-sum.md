# `border-radius: calc(infinity * 1px)` draws square corners on the painted hosts: an infinite length is held as `f32::MAX`, and two of them sum to infinity

**Status:** Closed
**Resolution:** A censored length is Chrome's largest, 2^25 px, so sums of such lengths stay finite, and the painters' radius reduction sums in `f64`, so a pill written as `calc(infinity * 1px)` or `3e38px` draws as one.
**Systems:** kernel/src/style/calc/mod.rs, host/linux/src/paint/border.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`calc::narrow` (`kernel/src/style/calc/mod.rs:226`) censors a top-level calculation as
CSS Values 4 §10.9.2 says, to the largest value the context allows, and takes that to
be `f32::MAX` (`kernel/tests/it/compare.rs:674` pins it). Every reader then works in
`f32`, so the first sum of two such lengths is `+inf`.

The painter that the Linux, Windows, Android and `apple-painted` hosts share reduces
radii by CSS's corner-overlap factor, `edge / (r1 + r2)`
(`host/linux/src/paint/border.rs:85`, `reduced`). With both radii `f32::MAX` the sum
is infinite, the factor is 0 and every radius becomes 0.

`calc(infinity * 1px)` is how current CSS writes a pill: the web host writes it for a
capsule control (`host/web/src/css.rs:307`), and it is Tailwind 4's `rounded-full`.

## The failure

```
view width=200 height=150 background-color="#ffffff"
  view width=100 height=40 border-radius="calc(infinity * 1px)" background-color="#2468ac"
```

Observed on the Linux host: a square box. `border-radius=9999` on the same box draws a
pill. `border-radius="3e38px"` draws a square box too. Expected: a pill for all three.
Chrome clamps the length to 2^25 px and the overlap rule (CSS Backgrounds 3 §5.5)
scales every radius to 20.

Layout overflows the same way. A box whose `width` and `padding-left` are both
`calc(infinity * 1px)` gets a frame `inf` wide; in Chrome it is 33,554,432 px. The
kernel refuses a non-finite number in a style patch to keep one out of a frame
(LLP 1001, `ApplyError::NonFiniteStyle`); here two finite numbers make one.

## Verification

A temporary test beside `percentage_radius_paints_an_ellipse_in_a_nonsquare_box` in
`host/linux/tests/pinned/paint.rs` compiled the Contract above and read the frame's
pixels. The CPU, GPU and Ink painters agree:

```
border-radius="calc(infinity * 1px)": (2,2) (36, 104, 172)   the box's colour
border-radius=9999:                   (2,2) (255, 255, 255)  the page's
border-radius="3e38px":               (2,2) (36, 104, 172)
```

A kernel test printed the row, `Points(3.4028235e38)`, and the `inf` frame. In
headless Chrome 156 the same box with either value has a computed radius of
`3.35544e+07px`, and `document.elementFromPoint` three pixels inside its corner is
the body. Not run: Windows, Android, and the Apple hosts, whose `BorderPaint.reduced`
sums in `CGFloat`, a double.

## Direction

Sum the radii in `f64` in `reduced`. Or make the largest length `narrow` returns one
whose sums stay finite, as Chrome's is, which keeps the frame finite as well; a
literal such as `3e38px` then wants the same ceiling where a row is set.

## Related

`issues/closed/20261002-border-radius-percentages.md` admitted percentages on the
radius rows and does not cover infinite lengths. Nothing in `QUEUE.md` or
`docs/agent-pitfalls.md` names `infinity`.
