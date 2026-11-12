# `env(titlebar-area-*)` is outside the window and zero wide where the Mac's title bar is right-to-left

**Status:** Closed
**Resolution:** The area follows `NSWindow.windowTitlebarLayoutDirection`, and LLP 1039 D8 states both directions.
**Systems:** host/apple/Sources/ExactKit/Mac/TitleBarAreaMac.swift, viewport facts (LLP 1039 D8)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`TitleBarAreaMac.rect` (`host/apple/Sources/ExactKit/Mac/TitleBarAreaMac.swift:31-38`, at `dd6f5dcd`) assumes the traffic lights are at the left of the title bar. The area's left edge is the lights' far edge plus the margin before the first one, and its width is what is left to the band's right edge:

```swift
let end = NSRect(x: last + first - bar.bounds.minX, ...)
left = max(left, view.convert(end, from: bar).maxX)
...
return [left - bounds.minX, top, max(0, band.maxX - left), band.height]
```

AppKit puts the lights at the right when `NSWindow.windowTitlebarLayoutDirection` is right-to-left. Apple documents that it follows the primary system language, "even in applications that don't have a right to left language localization", and that an app which places controls under a transparent title bar should read it (https://developer.apple.com/documentation/appkit/nswindow/windowtitlebarlayoutdirection). So this reaches any app that reads these variables on a Mac set to Arabic, Hebrew, Persian or Urdu, whatever its own strings. `first` is then the leftmost light's x near the right edge, `last + first` is beyond the window, and the area starts past the window's right edge with no width.

## The failure

An 800-point window:

```
main viewport-fit="cover" width="100%" height="100%"
  row margin-left="env(titlebar-area-x, 5px)" width="env(titlebar-area-width, 50px)" height="env(titlebar-area-height, 9px)"
```

Left-to-right the area is `78 0 722×32`, and the row has that frame. Right-to-left the lights are at x 731 to 791, the area is `1522 0 0×32`, and the row is at x 1522 and zero wide: a title row laid out this way is gone. Expected: `0 0 722×32`, the band less the controls and the same margin, at its right. Window Controls Overlay's area leaves out what is under the window's controls (https://wicg.github.io/window-controls-overlay/), and LLP 1039 D8 adopts it. AppKit's own horizontally corner-adapted safe area ends at 722 in that window.

## Verification

A temporary XCTest booted that page in a session, in a full-size-content window: once an `NSWindow`, once a subclass whose `windowTitlebarLayoutDirection` is `.rightToLeft`, for which AppKit places the buttons at the right.

```
left-to-right   buttons x 9-23, 32-46, 55-69        titlebarArea [78, 0, 722, 32]   row (78, 0, 722, 32)   safe area x 78 to 784
right-to-left   buttons x 777-791, 754-768, 731-745 titlebarArea [1522, 0, 0, 32]   row (1522, 0, 0, 32)   safe area x 16 to 722
```

A bare AppKit window in a process launched with `-AppleTextDirection YES -NSForceRightToLeftWritingDirection YES` has its buttons at the same right-hand positions. Not run: a Mac whose system language is right-to-left. `TitleBarAreaMacTests` has no right-to-left case.

## Direction

Take the controls' side from where they are, or from `windowTitlebarLayoutDirection`: with the lights at the right, the area runs from the band's left edge to the leftmost light less the margin. LLP 1039 D8's "from the left of the band past the traffic lights" needs the same correction.

## Related

`issues/20261009-titlebar-env-display-mode.md` is open for a real full-screen transition and does not cover layout direction. `QUEUE.md`'s right-to-left entry is about UIKit's grouped list margins.
