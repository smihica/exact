# `filter: drop-shadow(… currentcolor)` is refused, though `currentcolor` is the colour the function already holds by default

**Status:** Closed
**Resolution:** drop-shadow() accepts currentcolor in any ASCII case as the omitted colour while still rejecting duplicate colours.
**Systems:** kernel/src/svg/filter.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`FilterFn::DropShadow` carries `Option<ColorValue>`, and `None` is `currentcolor`
(`kernel/src/svg/filter.rs:28`, "`currentcolor` when none is given"). The grammar
reads a colour word with
`ColorValue::parse_light_dark(word).or_else(|| Color::parse(word))` (`filter.rs:153`),
which knows no `currentcolor`, so the keyword fails the whole list.
`TextShadow::check` (`kernel/src/style/shadow.rs:211`) and `stroke::parse`
(`kernel/src/style/stroke.rs:41`) read the same keyword as `None` before they try a
colour.

CSS takes it: `drop-shadow( [ <color>? && <length>{2,3} ] )` (Filter Effects 1 §7),
and `currentcolor` is a `<color>`.

## The failure

```
column filter="drop-shadow(0 2px 4px currentcolor)"
```

Observed: `lower-attr-value`, "is not a valid `filter`: `filter` is `none`, or
`url(#id)` naming a `filter` and the filter functions (blur, …, drop-shadow, …), in
order". The message names `drop-shadow` as admitted and does not say what is wrong.
Expected: the same value as `drop-shadow(0 2px 4px)`, which compiles and holds
`DropShadow(0.0, 2.0, 4.0, None)`.

## Verification

`contract build` on five lines: `filter="drop-shadow(0 2px 4px currentcolor)"` and
`filter="drop-shadow(currentColor 0 2px 4px)"` were refused with the message above;
`drop-shadow(0 2px 4px #000000)`, `drop-shadow(0 2px 4px)` and
`text-shadow="0 2px 4px currentcolor"` compiled. In headless Chrome 156
`CSS.supports('filter', 'drop-shadow(0 2px 4px currentcolor)')` is `true`, with the
colour first as well.

## Direction

In the `drop-shadow` arm, read `currentcolor` (any ASCII case) as no colour before
parsing one, as `TextShadow::check` does.

## Related

`BoxShadows::check` refuses a missing colour on purpose and says so (LLP 1064 D1);
this row already holds the default and only its keyword is missing. No issue or QUEUE
entry names `drop-shadow`.
