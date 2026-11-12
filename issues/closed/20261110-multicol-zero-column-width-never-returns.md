# `column-width: 0` with `column-gap: 0` gives 4,294,967,295 columns, and layout does not return

**Status:** Closed
**Resolution:** The kernel clamps a used `column-width` to at least 1px, as CSS Multi-column Layout 1 §3.1 says, so the count is bounded by the box's width.
**Systems:** kernel/src/fragment.rs, kernel/src/fragment/cut.rs, vendor/taffy (patch 27)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`fragment::taffy_multicol` passes an authored `column-width` to the engine floored at
zero (`kernel/src/fragment.rs:200-203`). `Multicol::columns` then computes the count
as `floor((U + gap) / (width + gap))` cast to `u32`
(`vendor/taffy/src/style/block.rs:60-70`). With a width and a gap of zero the division
is infinite and the cast saturates: the used count is 4,294,967,295. A width or gap
just above zero gives a count in the millions.

The cut trusts that count. `Cutter::balance` runs `for _ in runs.len()..n`
(`kernel/src/fragment/cut.rs:313`), and `publish` pushes one 20-byte `Column` per
count (`cut.rs:580-592`). At the saturated count the first loop runs four billion
times on the layout thread, and the second would then ask for 86 GB.

CSS Multi-column Layout 1 §3.1 says of `column-width`: "Used values will be clamped to
a minimum of 1px." The kernel does not clamp.

## The failure

```contract
component App
  view
    view column-width=0 column-gap=0 width=400
      view height=20
```

This compiles with no diagnostic. Observed: layout does not return. `contract::bake`,
which an app's build script calls, lays the first frame out for its layout lint
(`contract/cli/src/lib.rs:1344`), so the build of an app whose first screen holds this
box does not finish; where the box mounts later, the running app's layout is what
stops. Expected, as Chrome for Testing 156.0.8078.4 measures it: the
container is 400 by 0.06 and the child's bounding box is 20 by 1 at the origin (it
spans twenty 1px columns). `column-width: 0.4px` measures the same.

## Verification

- `taffy::Multicol { count: None, width: Some(0.0), gap: 0.0, .. }.columns(400.0)`
  returns `(4294967295, 9.313226e-8)`.
- A temporary test compiled the Contract above, booted it in a `Runner` and called
  `compute_layout` on another thread (debug build): it had not returned after 20 s,
  when the process was ended. With `contract::bake` before the boot, the bake had not
  returned after 20 s either. `contract build` alone, which does not bake, accepts the
  file.
- `column-width=0 column-gap=0.0001` in the same box lays out with 4,000,001 columns
  in 144 ms (debug). Chrome makes 400: a 4,000px-tall child there is cut into 400
  pieces 10 tall.

Not run: the zero case to its end (the allocation after the loop would exhaust the
machine), and no host.

## Direction

Floor the used column width at 1px where the engine's multicol field is derived, as
the specification and Blink do, so the count is at most the content width in pixels.
The cut's loops then need no bound of their own.

## Related

`issues/closed/20261012-column-height-ignores-leaf-max-height.md` is about a flex
column's height, not multi-column layout. Nothing in `issues/`, `QUEUE.md` or
`docs/agent-pitfalls.md` names `column-width` or a column count limit, and LLP 1093
D10's declared deviations do not cover it.
