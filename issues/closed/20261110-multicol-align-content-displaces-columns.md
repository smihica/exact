# `align-content` on a multi-column container moves its boxes apart and out of the box

**Status:** Closed
**Resolution:** The engine lays a multi-column container's flow thread out without its `align-content`, and the cut moves every column by the free space as Chrome aligns them, the columns of `column-fill: auto` as tall as their content while it fits one.
**Systems:** kernel/src/fragment.rs, kernel/src/fragment/build.rs, kernel/src/style.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A multi-column container's content is laid out once as one tall column, the flow
thread, and then cut into columns (LLP 1093 D2 to D5). The container's `align-content`
still reaches the engine with the rest of its block style
(`kernel/src/style.rs:1350`), so the engine aligns the flow thread inside the
container's height: it shifts every child by the container's content height less the
thread's whole height. The cut reads those shifted positions
(`kernel/src/fragment/build.rs:165-167`) and breaks at column ends measured from the
unshifted content top (`kernel/src/fragment/cut.rs:211`), so the shift lands in the
first column only, or is negative when the thread is taller than the columns.

It also makes the used height an input to the children, which `fragment::settle`
assumes it is not (`kernel/src/fragment.rs:332-336`). A container that is cut once and
skipped on later passes keeps placements from before the shift, and the incremental
layout differs from a rebuild.

## The failure

```contract
component App
  view
    view width=320 height=100 column-count=2 column-gap=0 align-content="flex-end"
      view height=20 testId="a"
      view height=20 testId="b"
```

| case | kernel `a`, `b` (x, y) | Chrome 156 `a`, `b` |
|---|---|---|
| `flex-end`, height 100 | (0, 60), (160, 0) | (0, 80), (160, 80) |
| `center`, height 100 | (0, 30), (160, 0) | (0, 40), (160, 40) |
| `flex-end`, automatic height | (0, -20), (0, 0) | (0, 0), (160, 0) |

In the last row the container is 20 tall, `a` is drawn above it and `b` stays in the
first column. With `align-content: center` and an automatic height, growing `b` to 30
leaves it at y 5 where a rehydrated kernel puts it at 0. With no `align-content` the
100px-tall case matches Chrome: (0, 0) and (160, 0).

## Verification

A temporary kernel test built each case through `Kernel::apply`, laid it out, changed
`b`'s height, laid out again and compared with a rehydrated kernel: it printed the
kernel frames above and, for `center` with an automatic height,
`incremental 4=[160.0, 5.0, 160.0, 30.0]` beside `rehydrated 4=[160.0, 0.0, 160.0, 30.0]`.
The Contract above compiles with no diagnostic and, booted in a `Runner`, gives the
first row's frames. Chrome's column was measured in headless Chrome for Testing
156.0.8078.4 with `getBoundingClientRect` on `div`s carrying the same declarations.
Not run: a host.

## Direction

Do not pass `align-content` to the engine for a multi-column container, so the flow
thread starts at the content top and the used height is again no input to it. Then
either shift every column by the free space when the cut is published, as Chrome
aligns the columns as a whole, or declare the row unsupported there in LLP 1093 D10.

## Related

LLP 1093 D10 lists the declared multi-column deviations and does not name
`align-content`; nor does LLP 1001. No issue or QUEUE entry mentions it.
