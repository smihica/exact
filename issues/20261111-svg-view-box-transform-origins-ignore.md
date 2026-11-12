# SVG view-box transform origins ignore the viewBox’s minimum coordinates

**Status:** Open
**Systems:** kernel/src/svg/scene.rs
**Severity:** P3
**Author:** Codex (GPT-6 Astra), from the fix lanes of the 2026-11-10 project review
**Date:** 2026-11-11

## What is wrong

At `09cb236d64007f51a668e38926fda6f64dfb6fb5`, `Resolver::transform` uses `(0, 0, vp.width, vp.height)` as the reference rectangle for `transform-box="view-box"` (`kernel/src/svg/scene.rs:568`). The viewport retains the viewBox's dimensions but not its minimum coordinates. A percentage or keyword transform origin is displaced when the viewBox starts away from zero.

## The failure

```xml
<svg width="100" height="100" viewBox="1000 1000 100 100">
  <rect x="1000" y="1000" width="10" height="10"
        style="transform-box:view-box;transform-origin:center;rotate:90deg"/>
</svg>
```

Expected origin in the rect's user space: `(1050,1050)`. The resolved scene gives `(50,50)`, so rotation is around a point 1,000 units away in both axes.

## Verification

Ran `CARGO_BUILD_JOBS=5 cargo test -p exact-kernel --test review_findings -- --nocapture`. A temporary test constructed the equivalent nodes with `Kernel::apply`, laid them out and called `svg::scene::resolve`. It printed:

```
Transform { origin: (50.0, 50.0), translate: (0.0, 0.0), rotate: 90.0, scale: 1.0, matrix: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0] }
```

The origin assertion failed against `(1050.0, 1050.0)`. The temporary test was removed. No native host was driven.

## Direction

Carry the viewBox's minimum coordinates alongside its dimensions and use the full reference rectangle for transform origins, including nested SVG and symbol viewports. Percentage lengths still use the dimensions.

## Related

`issues/20261110-svg-use-and-text-have-no-bounding-box.md` concerns fill/stroke bounding boxes for different element kinds. This case uses a rect and the view-box reference. No matching issue or QUEUE entry was found.
