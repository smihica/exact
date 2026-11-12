# A count button's measurement shortcut answers a wrapped button's height for a count that fits

**Status:** Closed
**Resolution:** Count-button shortcuts use an unconstrained reference, so wrapped counts cannot supply a fitting count's width or height.
**Systems:** host/apple/Sources/ExactKit/IOS/ButtonMeasureIOS.swift (ButtonMeasureCache)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `ButtonMeasureCache.answer` (`host/apple/Sources/ExactKit/IOS/ButtonMeasureIOS.swift:45`) keeps one measured button per face whose title is a count, keyed by the face without its title, the kind of offer, its width and the traits (`numerals`, lines 49 and 64). It answers every other count of that key from it: the stored size, wider or narrower by the difference of the two titles' ink (`Numeral.answer`, lines 92-101). Its comment says the title "is digits, on one line it cannot break" (line 72).

UIKit does break it. A button's title wraps "at word boundaries, unless the word doesn't fit on a single line" (Apple, `NSLineBreakMode.byWordWrapping`), and `ButtonConfigurationIOS.apply` changes that only for an authored `white-space: nowrap` or `line-clamp` (`ButtonConfigurationIOS.swift:295-296`, `:7-13`). So a definite offer narrower than the count wraps it inside the number and `sizeThatFits` answers a taller button. Line 64 stores that measurement as the key's reference without asking whether it fit. `Numeral.answer` checks only that the new count's computed width is within the offer (line 99), so a shorter count gets the wrapped reference's height and a width derived from the wrapped width.

## The failure

A native button whose title is a count, at a width the layout gives it:

```
row
  button press=like flex=1
    image "symbol:heart"
    text `${likes}`
```

With each button offered 100 points:

1. One post's count is `1,234,567`. It does not fit and is measured: 98 by 54.5, two lines. This becomes the reference.
2. The next post's count is `5`. The shortcut answers 30.5 by 54.5.

Expected for `5`: 61 by 34.5, one line, which is what UIKit measures. The kernel takes the height at a definite offer (`host/apple/src/control_text.rs:147`, "Height at the offered width"), so every count of that face not yet measured at that width is then laid out two lines tall. A count that fails line 99 is wider than the reference, wraps too, and replaces it with another wrapped one, so the reference does not return to one line.

## Verification

`Numeral`, `numeral(_:)` and the control flow of `answer` were copied into a bare Mac Catalyst process (the macOS 27.2 SDK's UIKit) and measured with the UIKit calls `measure` makes for a definite offer: a configured `UIButton` with a title and a symbol, `layoutIfNeeded`, `sizeThatFits`, the title label's attributed text. It printed:

```
1,234,567 offered 100.0: 98.0 x 54.5  (measured)
5         offered 100.0: 30.5 x 54.5  (numeral shortcut)
5         offered 100.0: 61.0 x 34.5  (UIKit, measured directly)
```

Not run: the real `ButtonMeasureCache` on an iOS simulator (none runs on this machine). The sizes are Catalyst's; an iPhone's differ, the wrap does not.

## Direction

Keep a measurement as a key's reference only when its title is on one line: for a definite offer, when the title's one-line width fits the offer, or measure the reference unconstrained. A count that wraps is then measured each time, as line 98's comment already intends.

## Related

No issue or QUEUE entry names the numeral shortcut, and `docs/agent-pitfalls.md` has nothing on count buttons.
