# The Apple presenter's sticky offset still cancels opposite insets on a box taller than its scrollport

**Status:** Closed
**Resolution:** Apple sticky placement reduces the end inset for oversized boxes and applies the start constraint to the adjusted position, matching the kernel
**Systems:** host/apple/Sources/ExactKit/Sticky.swift, kernel/src/kernel/sticky.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`StickyConstraint.offset` in `host/apple/Sources/ExactKit/Sticky.swift:38-53` is the Apple presenter's own copy of the kernel's rule, and its comment says so. `issues/closed/20261102-sticky-opposite-insets-cancel.md` fixed the kernel's copy (`kernel/src/kernel/sticky.rs:42-71`): the end inset is reduced until the sticky view rectangle holds the box, and the start push is computed from where the end pull left the box (`- d` in both terms). The Swift copy has neither change. It computes the pull and the push from the natural position and adds them.

The kernel sends the constraint's rectangles and insets, not an offset (the `sticky` op, `host/apple/src/batch.rs:803`). `StickyHost.place` (`Sticky.swift:94-97`) moves the view by the Swift value on iOS, tvOS and macOS, at each `sticky` op and each scroll of the box's scroller, the page or a dialog's sheet. The runner's geometry reads use the kernel's value (`runner/src/geometry/intersection.rs:194-203`), so on Apple they name a place the box is not drawn at.

## The failure

The closed issue's Contract:

```contract
component App
  view
    scroll width=200 height=100
      column height=500
        box height=20
        box position="sticky" top=0 bottom=0 height=150 width=50 testId="sticky"
        box height=330
```

The constraint is natural `[0,20,50,170]`, limit `[0,0,200,500]`, port `[0,0,200,100]`, top 0 and bottom 0. Chrome's measurements are in `kernel/tests/it/fixtures/browser_sticky.tsv:11`.

| scroll | Chrome and the kernel, y in the scrollport | `Sticky.swift`, y in the scrollport |
| --- | --- | --- |
| 0 | 0 | 0 |
| 20 | 0 | -20 |
| 50 | 0 | -20 |
| 100 | 0 | 0 |
| 400 | -50 | -50 |

Between offsets 0 and 70 the box's top is up to 20 points above the scrollport's top, where Chrome pins it. CSS Positioned Layout 3 §3.4 reduces the end inset so the start edge wins.

## Verification

A temporary XCTest on macOS (`bun host/apple/build.mjs --test --only <class>`) built a `StickyConstraint` from the payload above and called `offset` at the fixture's five offsets:

```
scroll 0: ExactKit y=0.0 (in the port 0.0); Chrome y=0.0 (in the port 0.0) ok
scroll 20: ExactKit y=0.0 (in the port -20.0); Chrome y=20.0 (in the port 0.0) WRONG
scroll 50: ExactKit y=30.0 (in the port -20.0); Chrome y=50.0 (in the port 0.0) WRONG
scroll 100: ExactKit y=100.0 (in the port 0.0); Chrome y=100.0 (in the port 0.0) ok
scroll 400: ExactKit y=350.0 (in the port -50.0); Chrome y=350.0 (in the port -50.0) ok
```

`cargo test -p exact-kernel --test it sticky_boxes_land_where_chrome_puts_them` passes on the same fixture line. No Apple app was scrolled: `StickyHost.place` applies the function's value with only pixel rounding between.

## Direction

Port the kernel's two changes into `axis` in `Sticky.swift`: reduce `end` by `port[i + 2] - port[i] - (start ?? 0) - (hi - lo)`, and subtract `d` in both terms of the push. Better, keep one rule: have the kernel answer the offset for a scroll position and delete the Swift copy.

## Related

`issues/closed/20261102-sticky-opposite-insets-cancel.md` names only `kernel/src/kernel/sticky.rs` and `host/linux/src/paint.rs`; its fix did not reach the Apple copy. `issues/closed/20261102-native-geometry-ignores-sticky-placement.md` concerns the runner's readers. No Swift test covers `StickyConstraint.offset`.
