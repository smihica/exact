# macOS: an empty date input starts at the wrong year on a Mac whose calendar is not Gregorian

**Status:** Closed
**Resolution:** Fixed by c3eb81b7: an empty field's value comes from DateValue.initial, whose formatter is en_US_POSIX and ISO 8601; run under th_TH, ar_SA, fa_IR and a Japanese calendar, each gives the Gregorian date.
**Systems:** host/apple/Sources/ExactKit/Mac/DatesMac.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`DateField.becomeFirstResponder` (`host/apple/Sources/ExactKit/Mac/DatesMac.swift:112-117`) starts an empty `input type="date"`, `"time"` or `"datetime-local"` at now. It writes the wall clock as text with a `DateFormatter` that sets only `dateFormat`, then parses the text with `DateValue.parse`, whose formatter is `en_US_POSIX`, ISO 8601 and UTC (`ControlKinds.swift:89-104`):

```swift
let local = DateFormatter()
local.dateFormat = "yyyy-MM-dd'T'HH:mm"
dateValue = DateValue.parse("datetime-local", local.string(from: Date())) ?? dateValue
```

A `DateFormatter` with no locale uses the person's region and calendar, so `yyyy-MM-dd` is that calendar's year, month and day, which the ISO parser reads as Gregorian ones. Apple's rule for a fixed format is to set `en_US_POSIX` (`DateFormatter`, "Working With Fixed Format Date Representations"; Technical Q&A QA1480). HTML's date value is always a proleptic Gregorian `yyyy-mm-dd` (HTML, "Dates").

## The failure

On a Mac whose region is Thailand (its calendar is Buddhist), focus an empty `input type="date" change=setDue` and press Up Arrow. On 2026-11-10 the app's `change` carries `2569-11-11`, where a browser and a Gregorian Mac give a date in 2026. A `datetime-local` carries `2569-11-10T14:11`. A `time` is right: only its hour and minute are written. Saudi Arabia's region (the Umm al-Qura calendar) gives `1448-05-30`, Iran's `1405-08-19` and a Japanese calendar `0008-11-10`.

iOS is not affected: `IOS/DatesIOS.swift` builds no date from formatted text.

## Verification

A temporary XCTest made the three inputs with empty values in a `Presenter`, focused each and read what `change` carried. Run as the Mac tests run, then with `-AppleLocale th_TH` passed to `xcrun xctest`:

```
Locale.current en_US, Calendar.current gregorian
empty date focused, then changed -> ["2026-11-10"]
empty date focused, Up Arrow -> ["2026-12-10"]

Locale.current th_TH, Calendar.current buddhist
empty date focused, then changed -> ["2569-11-10"]
empty datetime-local focused, then changed -> ["2569-11-10T14:11"]
empty time focused, then changed -> ["14:11"]
empty date focused, Up Arrow -> ["2569-11-11"]
```

The three lines quoted above with `DateValue`'s formatter, as a standalone tool run with `-AppleLocale ar_SA`, `fa_IR` and `ja_JP@calendar=japanese`, printed the other three values.

## Direction

Read the wall clock's fields with a Gregorian calendar in the current time zone and build the UTC date from them, or give this formatter `en_US_POSIX` and the ISO 8601 calendar as `DateValue.formatter` does.

## Related

`issues/closed/20261102-apple-pay-amount-follows-the-locale.md` is the same class of error in another file. The pitfall "A date input reads `10/05/2026` beside a label the app wrote in UTC" (`docs/agent-pitfalls.md`) is about what the field shows, not its value.
