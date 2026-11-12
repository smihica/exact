# `scroll-margin` accepts a percentage because its schema rows misspell `refusesPercent`

**Status:** Closed
**Resolution:** All four scroll-margin sides refuse percentages, including in calculations, during compilation, dynamic updates and wire decoding.
**Systems:** kernel/tables/schema.json, kernel/build.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The four `scroll_margin_*` rows mark themselves `"refuses_percent": true`
(`kernel/tables/schema.json:4400`, `4408`, `4416`, `4424`). The generator reads the
key `refusesPercent` (`kernel/build.rs:71`), which is how `column_width`,
`baseline_shift` and the `hit_slop_*` rows spell it. `StyleRow` does not deny unknown
fields, so the snake-case key is dropped silently and
`StyleId::ScrollMarginTop.refuses_percent()` is `false`. The refusal at
`kernel/src/style.rs:518` and the wire's `PercentNotAdmitted`
(`kernel/build/validate.rs:237`) never run for these rows.

CSS gives `scroll-margin` no percentage: its grammar is `<length>{1,4}` (CSS Scroll
Snap 1 §5.1).

## The failure

```
text "a" scroll-margin-top="10%"
```

compiles with no diagnostic. In the kernel:

```rust
let mut s = StyleProps::default();
s.set_dynamic(StyleId::ScrollMarginTop, &StyleValue::Text("10%".into()))
```

Observed: `Ok`. The row holds `Dimension::Percent(10.0)`, and `calc(10% + 4px)` is
held as `Calc(10.0, 4.0)`. Expected: the refusal `column-width` and the hit slop rows
give, "a length or auto: CSS takes no percentage here, nor one inside calc(), min(),
max() or clamp()".

The native readers resolve the row against a basis of zero
(`runner/src/runner/collection.rs:489`, `host/linux/src/presenter/reveal.rs:426`), so
the percentage counts for nothing, and a browser drops the declaration. The author
gets no margin and no diagnostic.

## Verification

A temporary kernel test printed, for a percentage number, the text `10%` and
`calc(10% + 4px)`:

```
ScrollMarginTop: refuses_percent()=false  Ok(Percent(10.0))  Ok(Percent(10.0))  Ok(Calc(10.0, 4.0))
ColumnWidth:     refuses_percent()=true   Err  Err  Err
HitSlopTop:      refuses_percent()=true   Err  Err  Err
BaselineShift:   refuses_percent()=true   Err  Err  Err
ScrollMarginTop Percent(10.0).at_basis(env, 0.0) = 0
```

`contract build` on the Contract line above, and on `scroll-margin="calc(10% + 4px)"`,
reported no error or warning. In headless Chrome 156,
`CSS.supports('scroll-margin-top', '10%')` and the same with `calc(10% + 4px)` are
`false`; with `10px` it is `true`.

## Direction

Rename the four keys to `refusesPercent`. `#[serde(deny_unknown_fields)]` on
`StyleRow`, with the keys other readers own declared (`snapshot`, `watch`, `terminal`,
`_comment`), would make the next misspelled key fail the build.

## Related

`issues/closed/20261009-scroll-padding-settlement-restore.md` added the rows and says
nothing about percentages. No open issue or QUEUE entry names them.
