# macOS: a grouped reorder's drop never releases the pointer hold its press began

**Status:** Closed
**Resolution:** macOS grouped drops release the pointer hold and deliver pointerup while the reorder may keep its ghost held.
**Systems:** host/apple/Sources/ExactDrag/MouseReorderMac.swift, host/apple/Sources/ExactKit/Mac/MouseChainMac.swift, host/apple/tests/ExactKitTests/ReorderGroupMacTests.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A left press begins a pointer hold when the pressed node or an authored ancestor hears `pointerdown`, `pointerup` or `pointermove`: `pointerPressed` sets `presenter.pointerHeld`, `pointerSource` and `pointerPath` (`MouseChainMac.swift:341-354`). Only `pointerReleased` clears them (`MouseChainMac.swift:413-440`), and for a left button `NodeView.mouseUp` calls it (`MouseEventsMac.swift:294-298`).

When the drag lifts a grouped grip (LLP 1094 D6), `MouseReorder.drag` installs a local monitor that takes the rest of the contact (`MouseReorderMac.swift:68-72`):

```swift
contact = NSEvent.addLocalMonitorForEvents(matching: [.leftMouseDragged, .leftMouseUp]) { [weak self] event in
    guard let self, grouped else { return event }
    if event.type == .leftMouseUp { _ = up(event) } else { presenter.reorderGroup?.move(event.locationInWindow) }
    return nil
}
```

A monitor that returns nil stops the event's dispatch, so no view's `mouseUp` runs, and `MouseReorder.up` (`MouseReorderMac.swift:93-100`) does not release the hold. `ExactView`'s monitor releases one only when its source has left the window (`ExactViewMac.swift:347-350`).

## The failure

Two lists with `reorderGroup="cards"` inside a box with `pointerdown`, `pointermove` and `pointerup` handlers. Drag a card to the other list and drop it. The box hears no `pointerup`. Until the next click, `trackPointer` returns at its guard (`MouseChainMac.swift:749`), so `:hover` looks and free `pointermove` stop. At the next click `pointerPressed` returns at its guard (line 341): the box hears no `pointerdown` for it, and its up delivers the dropped contact's `pointerup`. Expected: the drop ends the hold with `pointerup`, as the release of any other press does.

## Verification

A temporary XCTest used `ReorderGroupMacTests`' fixture with those three handlers on the lists' parent (node 9). The down and the lifting drag went to the pressed view. The rest went either to the view, as that test sends its up (line 94), or through `NSApp.sendEvent`, as a person's arrives:

```
up handed to the view:  pointerHeld nil;          heard ["down 9", "move 9", "move 9", "up 9"]
up through NSApp:       pointerHeld Optional(9);  heard ["down 9", "move 9"]
  a move over the other list before the next click heard []
  the next click: its down heard [], its up heard ["up 9"]
  the click after that heard ["down 9", "up 9"]
```

The existing test passes because it hands the up to the view, which a real up never reaches.

## Direction

In the grouped branch of `MouseReorder.up`, release the hold (`presenter.pointerSource?.pointerReleased(event)`), and send the test's up through `NSApp.sendEvent` so it takes the monitor's path.

## Related

`issues/closed/20261017-macos-hover-stuck-after-context-menu.md` fixed the same stale hold where `NSMenu` took the up. `issues/closed/20261010-pointer-events-reach-ancestors.md` made holds bubble.
