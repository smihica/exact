# On macOS, a modifier pair fires on one key once its monitor has missed a single key event

**Status:** Closed
**Resolution:** macOS modifier pairs use each event's side-specific flags and fire only when both keys are down, even after missed events.
**Systems:** packages/global-shortcuts/apple/GlobalShortcuts.swift
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A pair such as `Meta+Meta` (both Command keys) is heard by a `.flagsChanged` monitor. At `dd6f5dcd`, `flags` keeps the keys it believes are down by toggling: each event for a key code flips that key's membership of `down` (`packages/global-shortcuts/apple/GlobalShortcuts.swift:222` to `227`). It never reads which way the key went. The comment says "the device flags say which side", but the event's `modifierFlags` are not consulted.

Toggling is right only if the monitor hears every event of a key from a state where it is up. `down` starts empty when the first pair is read (`installMonitors`, line 209), whatever is held then. If one of the pair's keys is held at that moment, its release is the first event the monitor hears and is taken as a press. From then on that key's state is inverted until the monitors are removed: it reads down while up, so a press of the other key alone satisfies `down.contains(left), down.contains(right)` (line 229) and delivers the chord, and a real press of both delivers nothing until the inverted key is released. Any other event the monitors miss inverts a key the same way.

## The failure

`resource palette = globalShortcut(id="palette", keys="Meta+Meta")` on a screen the person opens with a Command shortcut, so the reader mounts while left Command is held. They release it, and later press right Command alone.

Observed: `globalshortcut` is raised for `palette` at that single key press, and again at every later press of right Command, so each shortcut typed with that key also runs the app's action. Expected: the pair fires only while both keys are down (LLP 1133.025 D3: "both of that modifier's keys held at once").

## Verification

Compiled the module's file as written on a Mac, with a stand-in for `ExactPackage`, `private` dropped from `HotKeys` and `flags`, and `AXIsProcessTrusted()` answered true because the test process has no Accessibility grant. After `globalShortcut(id="palette", keys="Meta+Meta")` answered, it handed `flags` the `flagsChanged` events a monitor would hear, made with `NSEvent.keyEvent(with: .flagsChanged, …)`: key code 55 with no flags (left Command released), then key code 54 with `.command`, with none, and with `.command` again. It printed:

```
left Command released (the monitor's first event of that key):
right Command (54) pressed alone:
  EMIT: globalshortcut palette
right Command released:
right Command pressed alone again:
  EMIT: globalshortcut palette
```

A second run pressed left and then right after the same first event: nothing was raised until left Command was released. Not run with real key presses, which need the Accessibility grant and a person. Which events a global monitor misses in practice was not measured.

## Direction

Set each key's state from the event instead of toggling it: the side-specific bits of `modifierFlags` (`NX_DEVICELCMDKEYMASK` and the others), or `CGEventSource.keyState` for the key code.

## Related

QUEUE "The desktop modules, what is owed" lists the pair's Windows hook as unbuilt and the origin of a macOS press; neither is this.
