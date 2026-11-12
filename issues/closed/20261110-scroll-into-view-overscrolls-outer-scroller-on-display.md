# With a display attached, scrolling a node into view moves each outer scroller as if the inner one had not moved

**Status:** Closed
**Resolution:** Reveal uses each box snapshot's page offset and projects clamped, pending scrolls through affine and projective transforms and sticky positioning before aligning outer scrollers.
**Systems:** host/linux/src/presenter/reveal.rs, host/linux/src/presenter.rs, host/linux/src/presenter/display_frame.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Presenter::scroll_into` (`host/linux/src/presenter/reveal.rs:403`) walks the scroll
containers above a node, innermost first, then the page. At each step it reads the node's
painted box again (`self.box_of(id)`, lines 466 and 508) and moves that scroller by what
the box asks. That is right only when the box shows the scrollers already moved.
`Presenter::boxes` (`host/linux/src/presenter.rs:1081`) paints again on demand only while
no display is attached. Once `display_frame` has run
(`host/linux/src/presenter/display_frame.rs:199`; its callers are the Linux display loop,
`host/linux/src/display.rs:425`, and Android's reader, `host/linux/src/android.rs:572` and
`host/linux/src/canvas.rs:1274`), the boxes are the acknowledged picture's until the next
flip. So every scroller after the first is moved from where the node was before any of
them moved, and two scrollers on one axis move the node about twice as far as it needs.
One scroller alone is right. Headless runs and Windows paint on demand and are right.

Every caller takes this walk: an app's `scrollIntoView` (`scroll_element_into_view`), a
fragment (`scroll_to_fragment`), the field kept above a keyboard and a remote's focus move
(`keep_in_view`), and the agent's `reveal`.

## The failure

A row in a scroller, on a page that scrolls, in a 400 by 300 window:

```
component App
  action aim
    scrollIntoView("row-30", block="start")
  view
    column width=400
      button press=aim testId="aim" height=30 flex-shrink=0
        text "Aim"
      box height=200 flex-shrink=0
      scroll testId="port" height=100 flex-shrink=0
        column
          box height=50 id="row-0"
          … (40 rows, `row-0` to `row-39`)
      box height=2000 flex-shrink=0
```

Tap `aim`, run the commands, paint. Expected: the scroller at 1500, the page at 230 and
the row at the window's top (CSSOM View, "scroll an element into view": each scrolling box
in turn, from where the element then is).

| | scroller | page | the row's y |
|---|---|---|---|
| no display | 1500 | 230 | 0 |
| display attached | 1500 | 1730 | -1500 |

With `block="nearest"` the page goes to 1480 where the headless run gives 30. The agent's
`reveal` of the row moves the page to 1605 where the headless run leaves it at 0, and it
answers `"scrolled":false`.

## Verification

A temporary test in `host/linux/src/presenter/fragment_tests.rs` booted the plan twice,
painting once with `frame()` and once with `display_frame()` then `display_complete()`,
the calls and the order of Android's loop, and printed the numbers above. Running the
commands between `display_frame` and `display_complete`, as the Linux display loop can,
gave 1730 too. A scroller on a page that does not scroll gave the same offsets both ways.
Chrome 156 gave 1500 and 230 for the same boxes in HTML. Not run on an Android device or a
Linux display.

## Direction

In `scroll_into`, keep how far each scroller moved and subtract it from the target's rect
in the later steps, instead of reading `box_of` again.

## Related

`QUEUE.md`, "Linux scroll anchoring: input inside the frame that carries an anchor
correction", is about a correction waiting for its picture, not this walk. `reveal.rs:43`
allows for the stale boxes in the sticky check only.
