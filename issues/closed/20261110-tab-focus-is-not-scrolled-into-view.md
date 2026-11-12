# Tab moves the focus to a control out of view and scrolls nothing

**Status:** Closed
**Resolution:** Tab's default action on the painted hosts scrolls each scroller above the new focus, then the page, only as far as it must to show it (`keep_in_view`).
**Systems:** host/linux/src/presenter/typing.rs, host/linux/src/presenter/events.rs, host/linux/src/presenter/reveal.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

Tab's default action ends in `self.set_focus(Some(order[next].2), now_ms)`
(`host/linux/src/presenter/typing.rs:579`). `set_focus`
(`host/linux/src/presenter/events.rs:386`) moves the focus and its ring and sets
`field_reveal`, which scrolls only a text field's own text to its caret
(`field_scroll.rs`). Nothing scrolls the page or a scroll container to the focused node.
`keep_in_view` (`host/linux/src/presenter/reveal.rs:386`) does that, and has three
callers: a TV remote's arrow (`remote.rs:350`), Android's Next and Previous on a hosted
field (`hosted.rs:647`) and the keyboard's rise (`window.rs:192`, `:208`). Tab on the
Linux desktop and on Windows is not among them, so a keyboard user's focus goes where they
cannot see it.

The platforms scroll it into view. Chrome does on Tab (measured below), and HTML's
`focus()` scrolls its target into view unless `preventScroll` is set (HTML, "Focus
management APIs"). WinUI's `ScrollViewer.BringIntoViewOnFocusChange` is true by default
("a behavior that brings focused items into view", Microsoft Learn). WCAG 2.2 SC 2.4.7
asks that the focus indicator be visible.

## The failure

A 400 by 300 window:

```
component App
  state n = 0
  action bump
    n = n + 1
  view
    column width=400
      button press=bump testId="top" height=30
        text "Top"
      column width=300 height=100 overflow-y="auto" testId="list"
        box height=400 flex-shrink=0
          text "Inner"
        button press=bump testId="inner" height=30 flex-shrink=0
          text "Inner button"
      box height=2000 flex-shrink=0
        text "Tall"
      button press=bump testId="bottom" height=30 flex-shrink=0
        text "Bottom"
```

Press Tab until `inner` has the focus, then until `bottom` has it.

| | scroller | page | the focused button |
|---|---|---|---|
| expected (Chrome 156) at `inner` | 330 | 0 | y 100 to 130, inside the scroller |
| observed at `inner` | 0 | 0 | y 430, the scroller shows 30 to 130 |
| expected (Chrome 156) at `bottom` | | 1860 | y 270 to 300 |
| observed at `bottom` | | 0 | y 2130 in a 300 px window |

## Verification

A temporary test in `host/linux/src/presenter/fragment_tests.rs` booted the plan, pressed
Tab with `key_down("Tab", …)` and printed the offsets and boxes in the "observed" rows. The
"expected" rows are the same boxes as bare HTML in Chrome 156, with Tab pressed through
Playwright. The path has no `cfg`, so the Windows build takes it; not run on Windows.

## Direction

After Tab moves the focus, call `keep_in_view(next, Cause::FocusMove)`, as
`move_focus_toward` does for a remote. Decide separately whether an app's `focus()` does
the same, as the web's does.

## Related

`QUEUE.md`, "Finish Windows text interaction and accessibility", lists a click that leaves
the focus unset, not a focus that leaves the view. Through two scrollers on one axis
`keep_in_view` over-scrolls the outer one while a display is attached (the issue "With a
display attached, scrolling a node into view moves each outer scroller as if the inner one
had not moved").
