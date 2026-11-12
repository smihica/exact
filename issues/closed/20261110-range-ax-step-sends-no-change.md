# A Mac range stepped by an assistive app sends no `change` unless the app's last event was a mouse-up or a key-down

**Status:** Closed
**Resolution:** Mac range actions commit outside mouse-down and dragging, so accessibility steps send input followed by change.
**Systems:** host/apple/Sources/ExactKit/Mac/RangeMac.swift, ControlHost
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`ControlHost.rangeChanged` (`host/apple/Sources/ExactKit/Mac/RangeMac.swift:33-49`, at `dd6f5dcd`) decides whether a move of the slider is committed from the event the application last took from its queue:

```swift
let released = NSApp.currentEvent.map { $0.type == .leftMouseUp || $0.type == .keyDown } ?? true
```

A slider's action also runs with no event of its own. An accessibility increment or decrement (VoiceOver, Voice Control, Switch Control) changes the `NSSlider` and sends its action from the accessibility request. `NSApp.currentEvent` is then whichever event the app handled last, so `released` follows something unrelated: after a mouse move, a key-up or a modifier change the app hears `input` and no `change`, and `lastRange[id]` keeps the value; after a mouse-up it hears both.

## The failure

```
input type="range" min=0 max=100 value=volume change=setVolume aria-label="Volume"
```

With VoiceOver on, step the slider. Observed: the knob moves and `setVolume` does not run. `apps/lights/app.contract:61` binds a light's brightness to `change` alone, so a stepped brightness is never sent. Expected: `input` then `change`, as an arrow key sends. LLP 1069.001 says `change` "fires when the value is committed", HTML's `change`; a step has no release to wait for.

## Verification

A temporary XCTest booted that page in a session with `input` and `change` handlers that log, put events through the application's queue to set `currentEvent`, and called `accessibilityPerformIncrement()` on the range's slider. What the app heard at each step:

```
after a mouse move (currentEvent: an AppKit-defined event)   input:55
the same on the slider's cell                                input:60
Right Arrow (`moveRight:` while its key-down is current)     input:65 change:65
after that key's key-up                                      input:70
after an unrelated mouse-up                                  input:75 change:75
```

Chromium 156: ArrowRight on a focused `<input type="range">` fires `input` then `change`. Not run: VoiceOver itself, or an `AXIncrement` from another thread through the AX API (the test process is not trusted for accessibility).

## Direction

Treat a move as uncommitted only while the mouse is tracking the knob (the current event is a left mouse-down or drag); every other action commits. The agent's `typeRange` already sends both.

## Related

`issues/closed/20261009-aria-range-accessibility-values.md` is about the values a range reports to assistive technology, not the events its steps send. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` covers it.
