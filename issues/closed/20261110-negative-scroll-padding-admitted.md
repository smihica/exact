# A negative `scroll-padding` compiles; the browser drops it and the native hosts use it

**Status:** Closed
**Resolution:** Scroll padding and SVG radii refuse negative literals in every length form; folded negative scroll-padding calculations clamp to zero.
**Systems:** kernel/src/style (row admission), contract/lower, host/web/src/css.rs, runner/src/runner/collection.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

CSS gives `scroll-padding-*` only `auto` or a non-negative
`<length-percentage>` (CSS Scroll Snap 1 §4.2). The compiler and the kernel
admit a negative length or percentage on the four rows. The web host writes
it through its default arm (`host/web/src/css.rs:438-447`), the browser
discards the declaration, and the native hosts read the number.

## The failure

Both compile:

```
component App
  view
    column scroll-padding-top="-10px"
      text "a"
```

and the same with `scroll-padding-top="-5%"`. The web host writes
`scroll-padding-top:-10px;` and `scroll-padding-top:-5%;` into the element's
style.

Chromium answers `CSS.supports("scroll-padding-top", "-10px")` false, and an
element with `style="scroll-padding-top:-10px"` computes `auto`.

Natively `scroll_padding` (`runner/src/runner/collection.rs:436-452`) returns
a resolved `Points(n)` whatever its sign, so a keyed jump lands 10 points
further than on the web. (A percentage resolves to 0 there, as `auto` does.)

Expected: refused at compile time, as `font-size="-1px"` is
(`lower-attr-value`, "expected a nonnegative length").

## Verification

A temporary test in `host/web/tests/it` compiled a `column` with each row and
printed `host.document().root`, which held the two declarations above; beside
them `font-size="-1px"` printed the refusal quoted. The pinned Chromium (156)
printed `false` for `CSS.supports` of both values and a computed
`scroll-padding-top` of `auto`, against `10px` for `scroll-padding-top:10px`.

The same test admitted `circle r="-10px"` and `rect rx="-10px"`, written
`r:-10px;` and `rx:-10px;`, which Chromium also refuses (`CSS.supports`
false; SVG 2 makes a negative radius invalid).

Not run: a native host's scroll with a negative padding.

## Direction

Add the four `scroll_padding_*` rows, and `r`, `rx`, `ry` for a literal, to
the kernel's rows that take no negative value, so the compiler names it.

## Related

The kernel keeps three lists of rows that refuse a negative value
(`relative::nonnegative`, `kernel/src/style/relative.rs:290`;
`calc::nonnegative_row`, `kernel/src/style/calc/parse.rs:720`;
`calc::no_negative_lengths`, `kernel/src/style/calc/mod.rs:481`), and
`scroll-padding` is on none of them. Making them one list fixes this only if
the list gains these rows. `cbc40bcb` refused the colours CSS refuses the same
way.

`issues/20261110-negative-number-accepted-where-negative-text-is-refused.md` covers the three lists themselves.
