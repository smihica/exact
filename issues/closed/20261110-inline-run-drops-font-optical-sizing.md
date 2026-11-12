# An inline run drops `font-optical-sizing: none` on Apple, so its text is painted at a width it was not measured at

**Status:** Closed
**Resolution:** Apple inline runs preserve font-optical-sizing in either JSON key order, so painted text uses the measured font axes.
**Systems:** host/apple/Sources/ExactCore/BatchShapes.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, a paragraph op's inline runs are decoded by one of two readers. When the op's first key is `"op"`, as Rust writes it, `BatchReader.inlineStyle` reads each run's style and hands `InlineStyle.set` only the rows in a fixed list (`host/apple/Sources/ExactCore/BatchShapes.swift:219-222`); it skips every other row. For any other key order, `BatchFields.inline` hands it every row (`:281`). `InlineStyle.set` handles `font_optical_sizing` (`:324`), the one row it handles that the list does not name.

The row does arrive. An inline run's style is `style_json_inherited` (`host/apple/src/paragraph.rs:266`), which sends the inherited text rows, and `font-optical-sizing` is inherited. The kernel measures the run with `optical_sizing_none` set (`host/apple/src/measure.rs:400`), so the run is measured at the face's own `opsz` and painted with `opsz` at the font size. A paragraph with no nested runs is unaffected: its own style goes through `FontAxis.of(style)` (`FontAxes.swift:38-40`). The headless session takes its runs from the same decode (`HeadlessSession.swift:569`), so widgets and the watch are affected too.

## The failure

```
component App
  view
    column
      text font-size=12 font-optical-sizing="none"
        text "The quick "
        text "brown fox" color="red"
        text " jumps over the lazy dog"
```

Observed: the three runs decode without the `none` marker. Their fonts set the line 252.4 pt wide; the fonts it was measured with set it 227.3 pt, so the line overflows its box or wraps.

Expected: painted equals measured. Under `none` the optical size is left alone (CSS Fonts 4, `font-optical-sizing`).

The same sentence in the system font, by size:

| size | painted (`auto`) | measured (`none`) |
|---|---|---|
| 12 pt | 252.363 | 227.338 |
| 17 pt | 338.954 | 322.062 |
| 28 pt | 530.455 | 530.455 |
| 48 pt | 896.250 | 909.352 |

## Verification

A temporary test in `host/apple/tests/it` booted the Contract above with `Host::boot` and printed its paragraph op: the op's first key is `"op"`, and each run's style carries `"font_optical_sizing":"none"`.

That op, passed through `Batch.decodeWire` (ExactCore's sources compiled with a `main.swift` as one module, macOS 27), printed `ownOpticalSize=false` for all three runs and `painted width 252.363  measured width 227.338`. One run decoded from an op whose first key is `"id"` kept the marker.

Not run: a whole app.

## Direction

Add `"font_optical_sizing"` to the list in `BatchReader.inlineStyle`, or drop the list and let `InlineStyle.set` decide, as `BatchFields.inline` does, so the two readers cannot differ again.

## Related

Closed `20260925-apple-inline-background-never-paints` was this reader dropping `background_color`. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names optical sizing.
