# A hatch's `after` does not fire while a scroll, a menu or a drag is tracked

**Status:** Closed
**Resolution:** Hatch after timers use the session clock's common run-loop modes and fire while input is tracked.
**Systems:** host/apple/Sources/ExactKit/HatchClock.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

LLP 1075.003.000.001 §2.4 makes `context.after(ms)` logical time, "for behaviour that must keep step with the app", and §13.9 records it as built: "An `after` is a timer on the session clock." On the wall `HatchClock.rearm` (`host/apple/Sources/ExactKit/HatchClock.swift:135`, commit `dd6f5dcd`) arms it with

```swift
timer = Timer.scheduledTimer(withTimeInterval: max(0, (next - now) / 1000), repeats: false) { … }
```

`scheduledTimer` puts the timer on the run loop in the default mode only ([`Timer.scheduledTimer(withTimeInterval:repeats:block:)`](https://developer.apple.com/documentation/foundation/timer/scheduledtimer(withtimeinterval:repeats:block:))). While UIKit tracks a touch in a scroll view the main run loop runs in its tracking mode ([`RunLoop.Mode.tracking`](https://developer.apple.com/documentation/foundation/runloop/mode/tracking)); AppKit uses [`.eventTracking`](https://developer.apple.com/documentation/foundation/runloop/mode/eventtracking) for a menu, a live resize and a control's drag. A default-mode timer waits until that ends.

The session's own timers do not wait: `SessionClockTimer.schedule` adds its timer for `.common` (`SessionClockTimer.swift:17`), and `issues/20261027-timers-cannot-wait-for-a-frame.md` records that an app's timer fires while a scroll view tracks or decelerates. So the app's timers fire during a scroll, and a package's `context.after` fires only once the tracking ends, however long that is. `packages/gamepad/apple/Gamepad.swift:403-404` starts and ends an effect with it, and `packages/purchases/apple/Purchases.swift:191` uses it for a timeout.

## The failure

```swift
let clock = session.natives.hatchClock
clock.natives = session.natives                                  // as NativeViews.module wires it
let timer = SessionClockTimer.schedule(after: 0.02) { _ in … }   // the session's own kind of timer
clock.after(7, ms: 20)
// Turn RunLoop.main in .eventTracking for 500 ms, then in .default,
// noting when the timer fires and when clock.nextAfter becomes nil.
```

Observed: the session clock's timer fired at 26 ms, inside the tracking; the hatch's `after` fired at 507 ms, a millisecond after the loop returned to the default mode at 506 ms. Expected: both fire at about 20 ms.

## Verification

Run as a temporary Mac XCTest on a booted session (`bun host/apple/build.mjs --test --only <Class>`), which printed `the session clock's timer fired at 26 ms; tracking ended at 506 ms; the hatch's after fired at 507 ms`. The test turned the run loop in `.eventTracking` itself; no menu was opened, and UIKit's tracking mode during a real scroll was not driven (no simulator on the test machine). It is the same rule for a default-mode timer.

## Direction

Arm it as the session's timers are: `SessionClockTimer.schedule(after:)`, or a `Timer` added to `RunLoop.main` for `.common`.

## Related

`issues/20261027-timers-cannot-wait-for-a-frame.md` asks for an app timer that may wait for a scroll's end as an explicit choice; this is a hatch timer that waits without one. `ScrollPumpIOS.swift:288` also uses `scheduledTimer`, for a rest timer that is meant to wait for the scroll to end.
