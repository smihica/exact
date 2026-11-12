# The Apple hosts draw a `visibility="collapse"` box that Linux, the terminal and the web hide

**Status:** Closed
**Resolution:** Apple hides collapse boxes and inline runs like hidden, including input, focus and accessibility.
**Systems:** host/apple/Sources/ExactKit/CssVisibility.swift, host/apple/Sources/ExactKit/InlineText.swift, host/apple/Sources/ExactKit/NodeText.swift, host/apple/Sources/ExactCore/BatchShapes.swift, host/apple/src/paragraph.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`visibility` has three values (`kernel/tables/schema.json:2474`: `visible`, `hidden`, `collapse`), and the Apple host sends the row as its CSS spelling (`host/apple/src/style.rs:372`). Every Apple reader hides only the literal `hidden` (commit `dd6f5dcd`):

- `host/apple/Sources/ExactKit/CssVisibility.swift:13`: `var cssVisibilityHidden: Bool { style["visibility"]?.string == "hidden" }`, which box paint, hit testing, focus and accessibility read on macOS, iOS and tvOS.
- `host/apple/Sources/ExactKit/InlineText.swift:41`, `NodeText.swift:153` and `host/apple/Sources/ExactCore/BatchShapes.swift:328`: a text run is hidden when its `visibility` `== "hidden"`.
- `host/apple/src/paragraph.rs:246`: an image in a paragraph's text is marked `inlineBoxHidden` only when its computed visibility `== Visibility::Hidden`.

The other hosts hide everything that is not `visible`: `host/linux/src/paint.rs:1477` and `:1494`, `host/terminal/src/paint.rs:198`, the kernel's SVG scene (`kernel/src/svg/scene.rs:713`) and the Apple host's own `host/apple/src/svg_lower.rs:774`. So does a browser: CSS 2.1 §11.2, "If used on elements other than rows, row groups, columns, or column groups, 'collapse' has the same meaning as 'hidden'."

## The failure

```contract
component Hello
  state pressed = "none"
  action hid
    pressed = "hidden"
  action col
    pressed = "collapse"
  view
    column gap=8 padding=12
      row gap=8
        box width=40 height=40 background-color="#0000ff" visibility="hidden"
        box width=40 height=40 background-color="#ff0000" visibility="collapse"
      text
        text "hidden ["
        image "symbol:star" width=16 height=16 visibility="hidden"
        text "] collapse ["
        image "symbol:star" width=16 height=16 visibility="collapse"
        text "]"
      button "Hidden" testId="hid" press=hid visibility="hidden"
      button "Collapse" testId="col" press=col visibility="collapse"
```

Expected, and what the web build shows: neither square, neither star, neither button, and no press reaches a button. On macOS the red square, the second star and the Collapse button draw, and the button is in the accessibility tree and takes the press.

A `tr` with `visibility="collapse"` is drawn too. There a browser removes the row and its space (CSS 2.1 §17.5.5), which the kernel's layout does not do: LLP 1153 D7 lays a table out as a grid, and LLP 1074 §7 does not propose `collapse`'s effect on layout. This issue is the painting only.

## Verification

With the plan above as `apps/hello/app.contract`:

- macOS (`bun host/apple/build.mjs hello-apple`, then `bun scripts/agent.mjs macos --app hello "tree --ax" "screenshot out.png" "tap hid" "state pressed" "tap col" "state pressed"`): the screenshot shows the red square, the second star and the Collapse button and nothing of the hidden three; `tree --ax` lists `button "Collapse"` and no `Hidden`; `state pressed` prints `"none"`, then `"collapse"`.
- Web (`bun host/web/build.mjs hello`, the same drive with `web`, Chrome 156): none of the six is drawn, neither button is listed, and `tap hid` answers `view 11 is hidden or inert`.
- A `table` of four rows, the second `collapse` and the third `hidden`: macOS draws the second row and keeps both rows' space; Chrome draws rows one and four with one row's gap between them.

iOS and tvOS are by reading: the comparisons above are theirs too. Linux and the terminal are by reading the cited lines.

## Direction

Treat every value but `visible` as hidden in the four Swift readers and in `paragraph.rs:246` (compare with `Visible`, as `svg_lower.rs` does). `kernel/src/kernel.rs:292` (`read_runs`, an inline image's accessible text) has the same `== Hidden` test.

## Related

`issues/20261110-slot-check-passes-visibility-collapse.md` is the compiler's slot check for the same value and names this disagreement between hosts. No QUEUE entry or pitfall mentions `collapse` for `visibility`.
