# `overflow-x: clip` beside a visible y axis clips both axes on the native hosts

**Status:** Closed
**Resolution:** `clip` beside `visible` now clips its own axis only, square, in the Linux painters (the paint walk and retained content regions), the runner's hit walk and the Apple hosts (a clip box reaching 65,536 points past the node on the visible axis).
**Systems:** host/linux/src/paint.rs, runner/src/geometry.rs, host/apple/Sources/ExactKit
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

CSS lets `clip` on one axis stand beside `visible` on the other: the box clips only
the axis that says `clip` (CSS Overflow 3 §3.1: `visible` computes to `auto` only
beside a value that is neither `visible` nor `clip`). The kernel computes this
correctly: `Overflow::computed` (`kernel/src/style.rs:1126`) leaves `(clip, visible)`
as it is, and the web host hands it to the browser.

The native hosts then ask one question for both axes.

- The painting host, `host/linux/src/paint.rs:1248`: `clips = ox != Visible || oy !=
  Visible`, and the whole box is pushed as the clip (`paint.rs:1251`).
- The runner's hit walk makes the same test (`runner/src/geometry.rs:1312`).
- The Apple host's `overflowClips` is true when either axis is `hidden` or `clip`
  (`Mac/NodeViewMac.swift:1102`, `IOS/NodeViewIOS.swift:1285`) and sets
  `clipsToBounds`.

So what overflows on the visible axis is not painted.

## The failure

```
box width=100 height=100 overflow-x="clip" background-color="#eeeeee"
  box width=200 height=200 background-color="#0000ff"
```

The child overflows the box to the right and below.

```
                                  painting host     macOS             web host, Chrome 156
(50, 50)   inside the box         (0, 0, 255)       (0, 0, 255)       (0, 0, 255)
(150, 50)  right of it, clipped   (255, 255, 255)   (255, 255, 255)   (255, 255, 255)
(50, 150)  below it, visible      (255, 255, 255)   (255, 255, 255)   (0, 0, 255)
```

The CPU, GPU and Ink painters agree. `effective_overflow` for the box is
`(Clip, Visible)`, and Chrome's computed style is `clip` and `visible`. Chrome shows
the child below the box to its full 200 pixel height, cut at x = 100.

`overflow-x: clip` is what a page writes to stop sideways overflow without making a
scroll container, so that a tall child, a popover or a sticky descendant still shows
below.

## Verification

A temporary unit test in `host/linux/src/paint/` booted the Contract above with
`PainterChoice::Cpu`, `Gpu` and `Ink` and read the frame's pixels and the effective
overflow. The same Contract as `apps/hello/app.contract` was built for the web and
for macOS and screenshotted with `scripts/agent.mjs web` and `macos`; bare `<div>`s in
Chromium 156 gave the web column's values and the computed style. The runner's hit
walk and UIKit were read, not run.

## Direction

When exactly one axis is `clip` and the other `visible`, clip a rectangle that spans
the box on the clipped axis and is unbounded on the other (CSS does not round it:
Overflow 3 §3.1), and test the hit walk on that axis only. On Apple that is a clip
box or mask in place of `clipsToBounds`. If a host cannot, declare it in LLP 1001.

## Related

The painting host also clips at the border box where CSS uses the padding box, on the
same line: a separate defect about which edge. `docs/contract-for-agents.md:4235`
says `clip` "clips without a scroll container" and nothing of one axis; LLP 1001
declares no deviation for it. Nothing in `issues/`, `QUEUE.md` or
`docs/agent-pitfalls.md` covers one axis.
