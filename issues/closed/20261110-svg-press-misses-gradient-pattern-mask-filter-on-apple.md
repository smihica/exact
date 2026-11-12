# An SVG element with a gradient or pattern paint, a mask or a filter takes no press on Apple

**Status:** Closed
**Resolution:** Apple SVG presses use fill and stroke geometry for paint servers and filtered content, honor clips, and ignore mask coverage and filter regions.
**Systems:** host/apple/Sources/ExactKit/SvgScene.swift, host/apple/Sources/ExactKit/SvgPaint.swift, host/apple/Sources/ExactKit/SvgIsland.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At commit `dd6f5dcd`, Apple finds the SVG element under a press by walking its Core Animation layers (`SvgScene.hit`, `host/apple/Sources/ExactKit/SvgScene.swift:378-393`). A layer counts only when it is a `CAShapeLayer` whose own `fillColor` or `strokeColor` paints the point (`paints`, `:397-408`), and a layer's mask must hold shapes that cover it (`covers`, `:412-418`). Three things are drawn another way, and each ends the walk with no element:

- **A gradient or pattern paint.** `attach` clears the shape's colours and hands the paint to part layers (`:552-555`). A server's part is a plain `CALayer` holding a bitmap, clipped by bounds or by a shape mask (`SvgPaint.swift:52-71`, `:186-191`). The comment at `SvgScene.swift:386` says a gradient's part hits as its element; only a solid part does.
- **A mask.** It is an island, a `CALayer` with a bitmap or a gradient layer (`SvgIsland.swift:189-210`), set as the layer's mask (`SvgScene.swift:568-581`). `covers` finds no shape in it, so `hit` returns at `:381`.
- **A filter other than a lone drop shadow.** The element arrives as a group whose content is only in the filter's picture (an empty `Kind::Group`, `host/apple/src/svg.rs:561-587`), so there is no shape layer to test.

The press goes to what is beneath, or nowhere. LLP 1055.000 D17 has masks and filters not change hits, and the kernel's rule, which Linux uses, counts any fill that is not `none` (`kernel/src/svg/scene/hit.rs:77-96`), as SVG 2's `visiblePainted` does.

## The failure

Nine pictures, each `svg width=120 height=60 viewBox="0 0 120 60"` with the `defs` it names and one shape with `press=hit("<name>")`:

```
solid      rect x=10 y=5 width=100 height=50 fill="#ef4444"
pattern    the same rect, fill="url(#dots)"
gradient   the same rect, fill="url(#sunrise)" (a linearGradient)
disc       circle cx=60 cy=30 r=25 fill="url(#glow)" (a radialGradient)
stroke     rect x=20 y=15 width=80 height=30 fill="none" stroke="url(#ink)" stroke-width=12
masked     the solid rect, mask="url(#all)" (a white rect over the whole picture)
blurred    the solid rect, filter="url(#soft)" (feGaussianBlur stdDeviation=1)
clipped    the solid rect, clip-path="url(#plot)"
shadowed   the solid rect, filter="url(#lift)" (feDropShadow)
```

A press inside each one's paint should count once. Observed on the Mac: `{"solid":1,"pattern":0,"stroke":0,"gradient":0,"disc":0,"masked":0,"blurred":0,"clipped":1,"shadowed":1}`.

## Verification

Run: the pictures as `apps/hello/app.contract`, `bun host/apple/build.mjs hello-apple`, then `bun scripts/agent.mjs macos --app hello "tap solid at 60 30" "tap pattern at 60 30" "tap gradient at 60 30" "tap disc at 60 30" "tap stroke at 20 30" "tap masked at 60 30" "tap blurred at 60 30" "tap clipped at 60 30" "tap shadowed at 60 30" state "screenshot out.png"`. Every tap reported `delivery platform`, the state was the line above, and the screenshot shows all nine drawn. In a Mac XCTest of the same page, `SvgHost.target` at those points answered an element for `solid`, `clipped` and `shadowed` and nil for the other six. iOS was not run (no simulator here); it calls the same `SvgScene.target` (`IOS/PointerIOS.swift:415`, `:542`).

## Direction

Hit by what the element is, not by which layer carries its pixels: remember at `attach` that a shape's fill or stroke is a server and test its path and outline, let a mask island cover every point, and test a filtered element's content from its scene. Or ask the kernel's `Scene::hit`, as Linux does.

## Related

LLP 1055.000's "Stage 6 as built" and QUEUE's "LLP 1055 / 1055.000 (SVG, completed in stages) owes" list what Apple's hit leaves out (text, the other `pointer-events` values, hover); none of these paints is among them. `issues/20261110-svg-gradient-repeat-past-256-periods-draws-one.md` is about drawing.
