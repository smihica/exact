# A Mac window keeps the old sidebar after a restart into a plan that wants none

**Status:** Closed
**Resolution:** The Mac window is notified after a restart changes whether the new plan needs a sidebar, so removing or opting out of root tabs removes the old sidebar.
**Systems:** host/apple/Sources/ExactKit/Mac/SidebarMac.swift, host/apple/Sources/ExactMac/main.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The window's owner composes the sidebar only when `SidebarHost` says `wanted` changed: `sync` reads `wanted` before and after it reads the tree and calls `onWanted` when they differ (`host/apple/Sources/ExactKit/Mac/SidebarMac.swift:121-124`), and `ExactMac` recomposes the window from nowhere else (`host/apple/Sources/ExactMac/main.swift:366`, `:382-425`).

A restart resets the presenter first (`presentCommitted` and `finishBoot` call `resetHeld`; `Mac/PresenterMac.swift:631`), and `SidebarHost.reset` sets `shown = nil` and tells nobody (`SidebarMac.swift:110-113`). The new plan's first batch then finds `wanted` false before it looks. If the new plan has root tabs, false to true tells the owner. If it has none, or has a tab bar the rule keeps as authored (`appearance="none"`, a placement of the app's own), false to false tells nobody, and the window keeps the `ExactSidebarController` and its rows.

A restart is the dev loop's reload after an edit, Develop ▸ Reload, Develop ▸ Open Project… and an update applied while the app is open. An author who removes the root tab bar, or writes `appearance="none"` on it, sees the sidebar stay beside the page until the app is relaunched. Its rows press nothing (`press(row:)` needs `shown`).

## The failure

Boot the tabs recipe (a root `tablist` with tabs "Today" and "History") in a window composed as `ExactMac` composes it, then replace the session's plan:

```swift
view.onWindowSidebar = { told += 1 }
session.apply(sameTabsPlan, label: "same")       // control
session.apply(tabsNonePlan, label: "none")       // the same app, appearance="none" on its tablist
session.apply(plainPlan, label: "plain")         // an app with no tabs
```

Observed after the second and after the third: `told` unchanged, `view.wantsWindowSidebar` false, and the split view's sidebar still 144 points wide, listing "Today" and "History". After the second the authored tab bar is drawn in the page as well. Expected: the owner is told once, and the window's content becomes the view.

## Verification

Run as a temporary Mac XCTest (`bun host/apple/build.mjs --test --only <class>`) over three compiled Contract pages, with an `ExactSidebarController` in an `NSWindow`. It printed:

```
booted with root tabs: wanted true, installed true, rows ["Today", "History"], sidebar width 144.0
restarted into the same app: told 1, wanted true, rows ["Today", "History"]
restarted into the app with appearance="none": told 0, wanted false, installed false, sidebar rows ["Today", "History"], collapsed false, width 144.0; the authored tablist hidden false, its frame (0.0, 424.0, 556.0, 24.0)
a press on the first row: false
restarted into an app with no tabs: told 0, wanted false, sidebar rows ["Today", "History"], collapsed false, width 144.0
```

The test drove `ExactSession.apply(_:label:)`, which reaches the same `presentCommitted` as the app's plan swap (`Session.swift:281`, `:1013`). The dev loop and `ExactMac`'s `compose` were read, not run.

## Direction

Compare with what the owner was last told, not with `shown` after the reset: keep that bit across `reset()`, or have `reset()` queue `onWanted` when a sidebar was shown.

## Related

`SplitHost.reset` (`Mac/SplitMac.swift:257-264`) tells its controller to stop splitting, so a list's detail pane does not outlive a restart. QUEUE's "A restart keeps per-node state that a new node with the same id then picks up" is host state keyed by node id that `Presenter.reset` does not clear; here the reset clears the state, and that hides the change from the owner. QUEUE's "The Mac's sidebar does not follow a state-driven change of the tab bar's placement while it shows" involves no restart.
