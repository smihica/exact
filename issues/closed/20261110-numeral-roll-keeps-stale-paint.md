# A rolling number on iOS keeps its old colour and font until its digits change

**Status:** Closed
**Resolution:** Rolling numerals apply changed text attributes immediately and roll only when their characters change.
**Systems:** host/apple/Sources/ExactNumerals/Numerals.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, a paragraph under `-exact-content-transition: numeric` is drawn on iOS and tvOS by SwiftUI's `Text` in the Numerals capability. Its node drops its own raster (`ExactKit/IOS/NodeViewIOS.swift:1471-1474`), so that `Text` is the paragraph's only paint. Each draw hands the capability the paragraph as an attributed string, and `NumeralRoll.show` returns before taking it when the characters are unchanged (`host/apple/Sources/ExactNumerals/Numerals.swift:79-89`):

```swift
let plain = attributed.string
guard plain != model.plain || model.version == 0 else { return }
...
model.text = (try? AttributedString(attributed, including: \.uiKit)) ?? AttributedString(plain)
model.version += 1
```

The guard keeps an unchanged number from rolling. It also discards every change that is only in the attributes: colour, font, size, letter spacing, a decoration. `NodeView.draw` is the only caller, and nothing else restyles the roll.

The colours in that string are resolved, not dynamic: `paragraphSpec()` picks the ink for the node's appearance (`ExactKit/NodeText.swift:139-142`) and `TextEngine.color` makes a plain `UIColor` (`ExactKit/Text.swift:124-133`). So SwiftUI does not correct them.

## The failure

```
component App
  state count = 41
  state bold = false
  action flip
    bold = not bold
  view
    column
      text "${count}" font-weight=(bold ? 700 : 400) -exact-content-transition="numeric"
      button "Flip" press=flip
```

Switch the device from light to dark. `traitCollectionDidChange` invalidates the text and redraws (`NodeViewIOS.swift:663-672`), the node calls `show` with the dark appearance's ink, and `show` returns at the guard: the number keeps the light appearance's ink until `count` changes. Pressing Flip leaves the weight as it was, and a text size change leaves the old size in a box laid out for the new one.

Expected: the paragraph shows its current style, as the raster path and the Mac do, and rolls only when its characters change.

A capture does not show this. `rollsNumerals` is false while capturing (`ExactKit/IOS/TextRasterIOS.swift:440-443`), so the node draws the paragraph itself and removes the roll, and the next draw makes a new one.

## Verification

`Numerals.swift` was compiled for Mac Catalyst (`-target arm64-apple-ios17.0-macabi`) against a stub of the ExactKit declarations it names, with its three private types made internal. A `main.swift` called `NumeralRoll.show` four times and drew `NumeralBody` with `ImageRenderer` over mid grey after each:

| `show` | `model.text` | `version` | drawn (ink, its height at 2x) |
|---|---|---|---|
| "41", black, 17 pt | black, 17 pt | 1 | black, 25 px |
| "41", white, 17 pt | black, 17 pt | 1 | black, 25 px |
| "41", white, 28 pt | black, 17 pt | 1 | black, 25 px |
| "42", white, 28 pt | white, 28 pt | 2 | white, 41 px |

Not run: the iOS simulator, which this machine cannot run. The path from a trait or style change to `show` is by reading the lines cited.

## Direction

Keep the last attributed string beside `plain`. When the characters are equal and the attributes are not, set `model.text` without advancing `version`, so the new paint shows with no roll.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names the numerals capability.
