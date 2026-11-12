# A zero-duration animation with a fill shows nothing on Apple when Core Animation plays it

**Status:** Closed
**Resolution:** `eligibility` samples a node with an animation of zero duration or zero iterations, so the engine presents its filled frame instead of Core Animation receiving a spec it cannot play.
**Systems:** host/apple/src/svg_lower.rs, host/apple/Sources/ExactKit/SvgScene.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

The Apple host hands some animations to Core Animation as specs, and the engine stops sampling them: a box's `opacity`, an SVG shape's `r`, `cx`, `cy`, `fill`, `stroke` and `stroke-dashoffset` (`host/apple/src/svg.rs:29`), and on iOS and tvOS a box's `translate`, `scale`, `rotate` and `background-color` (`svg.rs:46`). `eligibility` (`host/apple/src/svg_lower.rs:442`, commit `dd6f5dcd`) never looks at the animation's duration or iteration count, and `spec` (`svg_lower.rs:324`) writes them as they are.

Swift refuses to build an animation from such a spec (`host/apple/Sources/ExactKit/SvgScene.swift:161`):

```swift
guard duration > 0, repeatCount != 0, keyTimes.count == values.count, keyTimes.count >= 2 else { return nil }
```

The guard is right: `CALayer.add(_:forKey:)` changes a zero duration to 0.25 s. But nothing then plays, and because the node is lowered the engine presents nothing either, so the layer keeps its style row's value for good. LLP 1055 D7 makes the engine the source of truth, and the engine shows both cases below as CSS does when it samples them.

## The failure

```contract
keyframes dim
  from opacity=1
  to opacity=0.25
component Hello
  view
    row gap=8 padding=12
      box width=40 height=40 background-color="#ff0000" animation="dim 0ms linear forwards"
      box width=40 height=40 background-color="#ff0000" animation="dim 300ms linear forwards"
      box filter="blur(0.5px)"
        box width=40 height=40 background-color="#ff0000" animation="dim 0ms linear forwards"
```

Expected: all three squares at opacity 0.25. CSS Animations 1, `animation-duration`: at `0s` "the animation itself still occurs instantaneously", and "the last frame of the animation ... will be displayed if animation-fill-mode is set to forwards or both". Observed on macOS: the first square stays at opacity 1. The third, which the engine samples because it is under a filter, shows 0.25.

An iteration count of 0 with a fill fails the same way (the spec's `"n":0`), where CSS shows the first frame (`animation-fill-mode`). A duration that a state computes reaches the defect: `animation=(reduce ? "dim 0ms both" : "dim 300ms both")`.

## Verification

With the plan above as `apps/hello/app.contract`, `bun host/apple/build.mjs hello-apple`, then `bun scripts/agent.mjs macos --app hello "clock +1000" "screenshot out.png"`: the squares' centres read opacity 1.0, 0.25 and 0.25. With keyframes `rise` (opacity 0.25 to 1) and `animation="rise 600ms linear 0 forwards"`, a square alone read 1.0 and one under the filtered box 0.25.

A temporary test in `host/apple/src` (`cargo test -p exact-apple --lib <name> -- --nocapture`) printed what the Rust host sends for the `0ms` square, alone and under a filtered box:

```
{"op":"animations","specs":[{"d":0,"fill":1,"k":"opacity","n":1,"t":[0,1],"v":[1,0.25],…}]}   (no present)
{"op":"present","property":"opacity","x":0.25,"y":0} | {"op":"animations","specs":[]}
```

The computed `animation` above compiles and sends the same `"d":0` spec. With iOS's lowering switched on as `box_motion_tests.rs` does, `rotate`, `scale`, `translate` and `background-color` animations of `0ms linear forwards` go out as `"d":0` specs and the engine presents the start value (`rotate` 0; the Mac, where the engine plays `rotate`, presents 90). An SVG `circle` with the same `opacity` animation carries the `"d":0` spec in its scene. iOS itself was not run.

## Direction

In `eligibility`, sample a node that has an animation with a zero duration or a zero iteration count (`engine.set_node_sampled(node, true)`), so the engine presents its filled frame as it does on the other hosts.

## Related

No issue, QUEUE entry or pitfall mentions a zero-duration animation or a zero iteration count.
