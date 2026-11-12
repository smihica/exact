# iOS: a batch applied from another batch's closing work sends a scrolled-up `scrollFollowEnd` scroller to its end

**Status:** Closed
**Resolution:** UIKit restores captured scroll positions after navigation layout and collection insets settle, before deferred callbacks or navigation retirement can apply another batch.
**Systems:** host/apple/Sources/ExactKit/IOS/PresenterIOS.swift, host/apple/Sources/ExactKit/IOS/ScrollViewIOS.swift, host/apple/Sources/ExactKit/IOS/NavigationCoversIOS.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

References are at `dd6f5dcd`.

`Presenter.apply` captures each plain scroller's position at the start of a batch (`PresenterIOS.swift:804`) and restores it in the closure it queues with `afterBatch` (`PresenterIOS.swift:1092-1106`). The queue runs once `applying` is false again (`PresenterIOS.swift:825-833`), and the closure calls `navigation.batchApplied` before its restore loop.

`batchApplied` runs `sync`, which covers a header a navigation bar now stands for (`coverLifted`, `NavigationBarIOS.swift:589-599`). The cover is delivered through `afterBatch` (`NavigationCoversIOS.swift:196`), which runs at once when no batch is applying (`PresenterIOS.swift:747`), and `onCovers` applies the runtime's answer on the spot (`Session.swift:771`, `1052`). So a second batch is applied inside the first one's closure. It captures every scroller again, after the first batch's ops, and its own closure restores them and clears the capture (`ScrollViewIOS.swift:350`).

The first batch's restore loop then finds nothing captured, which a `scrollFollowEnd` scroller reads as "it was at its end" (`ScrollViewIOS.swift:370`), and writes the offset to the end:

```swift
let prior = followedScroll ?? (top: maximum, end: true, start: false)
```

A route's block (`OwnersIOS.swift:289-340`, from `sync` at `NavigationIOS.swift:865`) and a cells list's margins (`RowCellsIOS.swift:261-277`) reach the runtime the same way. The Mac presenter restores inside the batch (`PresenterMac.swift:1234-1239`) and is not affected.

## The failure

A plain follow-end scroller (a virtualized `list` is the collection's and skips this path) in a route that pushes one with a `header`:

```
for e in stack(nav) key=e.id
  column navigationKey=`${e.id}`
    if e.name == "thread"
      scroll scrollFollowEnd=true flex=1 min-height=0
        for m in messages key=m.id
          text m.body
      button press=openInfo
        text "Info"
    if e.name == "info"
      header
        text "Info" role="heading" aria-level=2
```

The reader scrolls the transcript up 500 points and taps Info. Observed, by the path above: the transcript's offset is set to its end, so Back shows the newest message. Expected: it stays where the reader left it (`docs/recipes/chat-thread.md:76`: "`scrollFollowEnd` follows only from there").

Other plain scrollers lose the first batch's scroll anchor the same way: the capture that is restored was taken after its rows moved.

## Verification

By reading; not run, because this machine cannot run the iOS simulator. No test covers it: `FollowEndBounceIOSTests` tests `followedTop` alone, and the tests that apply batches to a follow-end scroller set `onCovers` to nil (`NativeListNavigationIOSTests`) or use a presenter with no session.

A simulator run should scroll the transcript up, push the header route under `--chrome platform` (the agent's own chrome lifts no header, `NavigationBarIOS.swift:350-352`) and read the transcript's `contentOffset.y`. It should then push a route with no header, as `apps/messages` pushes its reader over its transcript: that case rests on the route's block, told only once UIKit has loaded the pushed route's view.

## Direction

Restore the scrollers before the closing work can apply another batch, as the Mac presenter does.

## Related

`issues/closed/20261009-scroll-padding-settlement-restore.md` and QUEUE's "A followed end resolved by the host" concern where the end is, not this order.
