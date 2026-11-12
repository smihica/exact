# An animation running when an ancestor gains a CSS `filter` stays with Core Animation and freezes in the filtered picture

**Status:** Closed
**Resolution:** After each receipt, `svg_lower::resample` switches every lowered animated node now under a filtered box or in an SVG picture to sampling, as it did for opacity in a glass group, and withdraws its Core Animation specs.
**Systems:** host/apple/src/svg_lower.rs, host/apple/src/host.rs, host/apple/Sources/ExactKit/BoxFilter.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

On Apple a filtered box is shown as a picture of its subtree (a lone `drop-shadow()` aside), drawn with `box.render(in: ctx)` (`host/apple/Sources/ExactKit/BoxFilter.swift:148`, commit `dd6f5dcd`). `CALayer.render(in:)` draws the layer tree "ignoring any animations added to the render tree", and the picture is drawn again when a batch touches the box or something in it, which a lowered animation never does. So an animation under a filtered box must be sampled by the engine, as the file's header says: "An animation under a filtered box is sampled instead of lowered (`svg_lower::eligibility`)."

`eligibility` decides this only for the nodes a commit's motion sync names (`host/apple/src/svg_lower.rs:449`, `for (node, animations) in &sync.animations`): the nodes the commit created or touched. A commit that sets `filter` on an ancestor does not touch the animated descendant, so `under_box_filter` (`svg_lower.rs:1023`) is not asked again and the descendant stays lowered. Transitions are rechecked every batch for exactly this (`hand_over_transitions`, `svg_lower.rs:648`: "an ancestor gained a filter"), and opacity under a glass group is too (`glass_sampling`, `svg_lower.rs:990`). Animations under a filter are not. `in_picture` (`svg_lower.rs:1048`) is asked at the same point, so an SVG shape whose group gains a filter stays lowered too.

## The failure

```contract
keyframes pulse
  from opacity=1
  to opacity=0.2
component Hello
  state blurred = false
  action toggle
    blurred = not blurred
  view
    column gap=8 padding=12
      button "Blur" testId="blur" press=toggle
      row gap=8
        box filter=(blurred ? "blur(0.5px)" : "none")
          box width=40 height=40 background-color="#ff0000" animation="pulse 1000ms linear infinite alternate"
        box filter="blur(0.5px)"
          box width=40 height=40 background-color="#ff0000" animation="pulse 1000ms linear infinite alternate"
```

Press Blur. Expected: both squares keep pulsing together. Observed on macOS: the first square jumps to its row's opacity, 1, and stays there until the filter goes; the second, filtered from boot and so sampled, keeps pulsing. On iOS and tvOS a box's lowered `translate`, `scale`, `rotate` and `background-color` animations take the same path, so a turning indicator would stop too.

## Verification

With the plan above as `apps/hello/app.contract`, `bun host/apple/build.mjs hello-apple`, then `bun scripts/agent.mjs macos --app hello "clock +500" "screenshot a.png" "tap blur" "clock +250" "screenshot b.png"`. The squares' centres read opacity 0.6 and 0.6 in `a.png`, and 1.0 and 0.4 in `b.png`. Pressing Blur again brings the first square back to the second's value. An `svg` `rect` under a `g` with the same toggled `filter` read 1.0 beside 0.4 the same way.

A temporary test in `host/apple/src` (`cargo test -p exact-apple --lib <name> -- --nocapture`) printed the engine's answer and the ops for the animated square:

```
blurred at boot=false: sampled=false ops={"op":"animations","specs":[{…"k":"opacity"…}]}
  after toggle (blurred=true): sampled=false ops=
  tick 600ms: ops=
blurred at boot=true: sampled=true ops={"op":"animations","specs":[]}
  after toggle (blurred=false): sampled=true ops={"op":"present","property":"opacity","x":0.92,…}
```

iOS and tvOS are by reading: they draw the picture with the same `BoxFilter.render`.

## Direction

After each receipt, sample every lowered animated node that is now under a box filter or in an SVG picture, as `glass_sampling` does for a glass group (one walk over `engine.animated_nodes()`, withdrawing the specs of the views it switches).

## Related

QUEUE "CSS `filter` on a box, Linux and what Apple still owes" lists what changes a box without a batch (an image decode, a text raster, a video frame); a lowered animation is not among them and has a different cause. Nothing in `issues/` or `docs/agent-pitfalls.md` covers it.
