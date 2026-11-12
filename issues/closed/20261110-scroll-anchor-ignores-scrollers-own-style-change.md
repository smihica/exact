# Scroll anchoring on the painted hosts adjusts when the scroller's own padding, margin or height changes, where CSS suppresses it

**Status:** Closed
**Resolution:** Changes to the scroller's own layout properties suppress scroll anchoring, using the same property comparison as the anchor path.
**Systems:** host/linux/src/scroll_anchor.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

CSS Scroll Anchoring suppresses the adjustment on "any change to the computed value of
any of the following properties, on any element in the path from the anchor node to
the scrollable element (or document), inclusive of both": the insets, margin, padding,
the width and height properties, `position` and `transform`
(§2.2.2, https://drafts.csswg.org/css-scroll-anchoring/#suppression-triggers).

`host/linux/src/scroll_anchor.rs` snapshots that path without the scroller. `capture`
(lines 269-274) ends the chain below it (`at.filter(|&n| n != id)`), and
`Anchor::shift` (lines 290-312) compares the `LAYOUT` rows of the chain's nodes only.
A change to the scroller's own rows is never seen, so the host adjusts the offset where
a browser leaves it.

## The failure

```
component App
  state pad = 0
  state top = 0
  state tall = 300
  state head = 0
  state inner = 0
  action headed
    head = 50
  action inset
    inner = 50
  action padded
    pad = 50
  action margined
    top = 50
    head = 50
  action heightened
    tall = 350
    head = 50
  view
    column width=400 height=600
      scroll testId="port" width=400 height=tall padding-top=pad margin-top=top
        box height=head
        column padding-top=inner
          box testId="row" height=1000
      button "head" testId="headed" press=headed
      button "inner" testId="inset" press=inset
      button "pad" testId="padded" press=padded
      button "top" testId="margined" press=margined
      button "tall" testId="heightened" press=heightened
```

Scroll `port` to 400, then press one button:

| button | what changes | painted host | Chrome 156 |
|---|---|---|---|
| `headed` | a box above the anchor grows by 50 | 450 | 450 |
| `inset` | the anchor's parent's `padding-top`, 0 to 50 | 400 | 400 |
| `padded` | the scroller's own `padding-top`, 0 to 50 | 450 | 400 |
| `margined` | the scroller's own `margin-top`, 0 to 50, and the box above | 450 | 400 |
| `heightened` | the scroller's own `height`, 300 to 350, and the box above | 450 | 400 |

In the last three Chrome suppresses the adjustment and the content moves. On the
painted host the offset follows the content, so padding the author added to the
scroller is scrolled out of view.

## Verification

A temporary test in `host/linux/src/presenter/collection/anchoring.rs`
(`cargo test -p exact-linux --lib`) booted this source at 400 by 600 with the CPU
painter once for each button, scrolled, dispatched the press and read the offset after
the next frame. It printed the painted host's column. Chrome's column is from the
scroller and its three boxes as HTML (`overflow: auto`, the default `content-box`),
driven with the repository's `playwright-core` and Chromium (156.0.8078.4): `scrollTop`
set to 400, two animation frames, the same style changes, two more frames. Not run on
a display, on Windows or on Android.

## Direction

In `capture`, when the scroller is a node, keep its own `LAYOUT` rows beside the
chain, and return `None` from `shift` when one of them changed. The page's chain
already runs to the root.

## Related

QUEUE's "`overflow-anchor` and priority candidates" is the opt-out, and "Linux scroll
anchoring: input inside the frame that carries an anchor correction" is input during a
correction. Neither is suppression on the scroller.
