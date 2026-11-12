# On macOS a change of the window's occlusion stops a 2D canvas's frames until a later batch

**Status:** Closed
**Resolution:** Occlusion changes use frames.hasWork, preserving a 2D canvas's display link until its frame request clears.
**Systems:** host/apple/Sources/ExactKit/Agent.swift (`ExactSession.occlusionChanged`), host/apple/Sources/ExactKit/Frames.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`Frames.hasWork` (`host/apple/Sources/ExactKit/Frames.swift:32`, commit `dd6f5dcd`) says whether the session's display link should run:

```swift
var hasWork: Bool { motion || canvas2d || timerSoon || session?.canvases.wantsFrames == true }
```

The callers that reschedule frames pass it (`SessionClock.swift:17` and `:27`, `SessionActivation.swift:86` and `:108`, `ExactViewMac.swift:394`), except the one a Mac window's `windowDidChangeOcclusionState` reaches (`Agent.swift:1304`):

```swift
public func occlusionChanged() {
    #if os(macOS)
    canvases.occlusionChanged()
    #endif
    frames.run(frames.motion || frames.timerSoon || canvases.wantsFrames)
}
```

It leaves out `canvas2d`, which `apply` sets from `batch.canvas` when a 2D canvas's `draw` asked for another frame (LLP 1056 D5; `Session.swift:1151`). With a 2D canvas as the only thing animating, `run(false)` invalidates the link (`Frames.swift:188`), and the link's ticks are what make the canvas's next batch.

The session hears the same notification a second way: `PageFacts.observe` (`PageFacts.swift:355`) calls `tellPage` (`:396`), which applies three batches, each ending in `frames.run(frames.hasWork)`. ExactMac and the sample host make the session, which registers that observer, before they set the window's delegate (`ExactMac/main.swift:243` and `:288`), and in the test below the observer ran first, so the delegate's stop came last. The link starts again at the next applied batch: an input the runner hears, a timer, a response, or another page fact (the window's key status, the app's activation, a minimise or restore).

AppKit posts the notification when the window is covered and again when it shows ([`didChangeOcclusionStateNotification`](https://developer.apple.com/documentation/appkit/nswindow/didchangeocclusionstatenotification)). A window that is uncovered with no change of key window or active app, such as a background window another window passed over, therefore shows its canvas on its last frame.

## The failure

```swift
// A booted session in a window, then the delegate, as ExactMac orders them.
// The delegate: windowDidChangeOcclusionState → session.occlusionChanged()
session.frames.canvas2d = true                      // as apply sets it from batch.canvas
session.frames.run(session.frames.hasWork)          // the link runs
NotificationCenter.default.post(name: NSWindow.didChangeOcclusionStateNotification, object: window)
```

The stand-in plan has no canvas, so the delegate sets `canvas2d` again before its call, as a canvas that wants frames keeps `batch.canvas` true. Observed: three batches applied before the delegate ran, none after; then `hasWork` true and `frames.link` nil, still nil 0.3 s later. Expected: the link runs while `hasWork` is true.

With the delegate set before the session is made, the delegate ran first and the three batches after it; for a real canvas the last of them restarts the link.

## Verification

Run as temporary Mac XCTests (`bun host/apple/build.mjs --test --only <Class>`). The snippet printed `batches applied before the post 38, when the delegate ran 41, when the post returned 41; then canvas2d true, link nil`. A plainer test, the flag set and `session.occlusionChanged()` called, fails `XCTAssertNotNil(session.frames.link)`. Not run: a plan with an animating 2D canvas, and a window covered on a display (the test machine sent no occlusion change for `orderOut` and `orderFrontRegardless`), so which of AppKit's notifications comes last when a window returns from the Dock is not known.

## Direction

`frames.run(frames.hasWork)` in `occlusionChanged`.

## Related

No issue, QUEUE entry or pitfall names occlusion and the frame link. `issues/closed/20261013-macos-minimized-window-hidden.md` is about the page's `visibilityState`.
