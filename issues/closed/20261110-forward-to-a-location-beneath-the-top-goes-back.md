# A native Forward to a location the stack also holds beneath its top is recorded as a Back, so Forward never runs out

**Status:** Closed
**Resolution:** Native Forward consumes the nearest location ahead even when go pops to an earlier visit at that location.
**Systems:** runner/src/runner/history.rs, runner/src/runner/router.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A native host has no session history, so the runner keeps the locations Back left, the nearest last (`runner/src/runner/history.rs`, at `dd6f5dcd`). A host's Forward asks `Runner::location_ahead` and delivers that location to the navigation root's `navigate`, whose action calls `go` (LLP 1038 §3 gives it for "A browser Back or Forward past one step"), or to `Runner::host_go`, which calls it (`runner/src/runner/router.rs:1036`, `:1092-1109`). `go` pops to the nearest visit with that location on the selected stack before it considers pushing (`route/src/router.rs:253-262`). When the location Forward names is also beneath the top, the commit shortens the stack, and `history::step` reads any shortening as a Back (`history.rs:39-42`): it puts the dropped visits ahead and leaves the location Forward just took there too.

LLP 1115.000 D5.5: "Forward goes where Back left, as session history does". In a browser the same Forward lands on the last history entry, which takes the new top's id (LLP 1038 D7), and nothing is ahead. The stack `go` leaves is the same there; what is ahead is not. D5.5 states `step`'s rule ("a commit that drops visits from the top of the selected stack puts their locations ahead") without considering a Forward that drops visits.

## The failure

```
routes nav
  home "/"
    thread "/t/:thread"
component App
  action open(u: string)
    nav = push(nav, u)
  view
    main navigationKey=`${top(nav).id}` width="100%" height="100%"
      for e in stack(nav) key=e.id
        column navigationKey=`${e.id}` position="absolute" inset=0
          text e.url
          button press=open("/t/a") testId=`a${e.id}`
            text "Thread a"
          button press=open("/t/b") testId=`b${e.id}`
            text "Thread b"
```

On the Linux host, press Thread a, Thread b, Thread a, then the mouse's Back button once and its Forward button four times:

```
three links pressed   stack / /t/a /t/b /t/a   ahead none
mouse Back            stack / /t/a /t/b        ahead /t/a
mouse Forward         stack / /t/a             ahead /t/b
mouse Forward         stack / /t/a /t/b        ahead /t/a
mouse Forward         stack / /t/a             ahead /t/b
mouse Forward         stack / /t/a /t/b        ahead /t/a
```

Observed: Forward never runs out and alternates between two stacks, minting a new `/t/b` visit every second step. Expected: after the first Forward nothing is ahead, as with a stack that repeats no location (`/ /t/a /t/b /t/c`, Back, Forward: the four visits again, nothing ahead, a second Forward does nothing).

Apps reach such a stack: `push` is the verb their links use most (34 assignments in the contracts under `apps/`), and a post opened from a profile opened from that post repeats its location.

## Verification

A temporary test beside the Linux host's Back tests (`host/linux/src/presenter/back_tests.rs`) booted the app in a `Presenter`, pressed the links, and sent the mouse's Back and Forward buttons through `pointer_aux`, then Alt+Right twice. It printed the stacks above from the runner's router and `location_ahead`, and Alt+Right carried the alternation on. The result was the same with `navigate=follow` on the root (`action follow(url: string)` assigning `nav = go(nav, url)`) and with no handler.

A runner test making the same deliveries (`Event::Navigate` at the root, or `host_back` and `host_go`) printed the same stacks, and for `/ /t/a /t/b /t/c` what "Expected" says.

Not run: the Mac, whose Go ▸ Forward and swipe ask the same `locationAhead` and call the same `navigate` or `hostGo` (`host/apple/Sources/ExactKit/Mac/BackMac.swift:179`, `:194-198`).

## Direction

The host knows it is delivering a Forward. Let that reach `step` (a mark the delivery sets on the router for its commit): a commit so marked takes the nearest location from `ahead` and adds none, whatever `go` did to the stack. The top's location alone cannot tell it from a Back to an earlier visit with that location.

## Related

LLP 1038 D7's web table meets the same `go` by giving the browser's entry the new top's id, and the browser still owns what is ahead. No issue or QUEUE entry covers the native list.
