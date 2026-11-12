# A virtualized list stays empty when the text row that made its rows zero tall is cleared

**Status:** Closed
**Resolution:** Clearing inherited typography invalidates virtualized row measurements and remounts rows measured at zero, with or without a kernel.
**Systems:** runner/src/instance.rs, runner/src/instance/collection
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`Tree::update` invalidates every virtualized list's row measurements when an update changes typography on a view that already existed (`runner/src/instance.rs:1336`, at `dd6f5dcd`). It counts only an `Op::SetStyle` whose mask intersects `StyleMask::TEXT`. A text row that becomes unset is an `Op::ClearStyle` (`instance.rs:1063-1077` for `none` or a CSS-wide keyword, `1129-1143` for a refused value) and is not counted. The branch for a runner with no kernel has the same gap: it skips a value that is `none` (`instance.rs:1017`).

A conditional class reaches this: a row is `none` whenever the chosen style does not set it. So `font-size` going from unset to a value invalidates, and going back to unset does not.

Invalidation is what gives rows measured at zero their estimate again and mounts a window (`invalidate_height_estimates`, `runner/src/instance/collection/mod.rs:296`). Without it a list whose rows all measured zero has no extent and no mounted row, so no host can measure it again.

## The failure

```
style Shut
  font-size=0
style Plain
  opacity=1
component App
  state shut = false
  action flip
    shut = not shut
  view
    column
      button press=flip testId="flip"
        text "flip"
      list class=(shut ? Shut : Plain) virtualized=true estimated-item-height=20 height=100
        for x in range(200) key=x
          text "row"
```

Press `flip` twice. At boot and after each press, report every mounted row's laid-out height until the list stops changing. Observed, from `Runner::collections()`:

```
open:        total_extent 3991.200008392334, 11 rows mounted
shut:        total_extent 0, 0 rows mounted
open again:  total_extent 0, 0 rows mounted
```

Expected on the last line: the first line again. With `font-size=16` in `Plain` in place of `opacity=1`, the last line is the first line again. `line-height=0` in `Shut` gives the same two results.

Rows taller than zero recover without the invalidation. With `font-size=24` in place of `font-size=0`, rows reported at 30 keep that height when the row is cleared, and the next report of 20 at the unchanged epoch is accepted (`set_measured_height_at`, `collection/index.rs:521`).

## Verification

A temporary test booted the app above with `Runner::boot`, pressed `flip` through `Runner::dispatch`, and after each commit ran the kernel's layout and sent each mounted row's frame height through `Runner::collection_feedback`. It printed the lines above. With `Op::ClearStyle` counted at `instance.rs:1336`, the same test printed 11 rows mounted for both styles and the other tests under `instance::collection` passed.

Not run: a host, and a runner with no kernel. That the Linux and Apple hosts report a mounted row's changed height at an unchanged epoch is from reading `host/linux/src/presenter/collection.rs:1219` and `host/apple/Sources/ExactKit/Collection.swift:274`.

## Direction

In `Tree::update`, count an `Op::ClearStyle` on an existing view whose mask intersects `StyleMask::TEXT`. In the branch with no kernel, count a text row whose value changed to `none`.

## Related

`typography_change_remounts_a_fully_zero_measured_collection` (`runner/src/instance/collection/tests.rs:746`) states the requirement for a row that is set. No issue or QUEUE entry mentions list measurements and a cleared style.
