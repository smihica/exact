# An empty date or time input on iOS opens at UTC's day and time, not the wall clock's

**Status:** Closed
**Resolution:** Fixed by c3eb81b7: an empty picker starts from the local wall clock, within its bounds (DateValue.initial). The issue's twelve Los Angeles and Tokyo cases pass; not run on a phone.
**Systems:** host/apple/Sources/ExactKit/IOS/DatesIOS.swift (makeDate, configureDate)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`makeDate` sets the `UIDatePicker`'s `timeZone` to UTC (`host/apple/Sources/ExactKit/IOS/DatesIOS.swift:14-17`), because HTML's values carry no zone. `configureDate` sets its `date` only from a bound value (lines 42-49). While the value is empty the picker keeps UIKit's default, "the date when the UIDatePicker object is created", and shows that instant in "the time zone reflected in the date displayed by the date picker" (Apple, `UIDatePicker.date`, `UIDatePicker.timeZone`). The calendar or wheels an empty input opens therefore start from UTC's clock.

The intent is the person's today (line 93: "Opening it starts at today"). The Mac's field starts from the wall clock's fields (`Mac/DatesMac.swift:112-117`), and `docs/agent-pitfalls.md` says these inputs "mean a local wall time, as on the web".

## The failure

```
input type="time" value=start change=setStart
input type="date" value=day change=setDay
```

with `start` and `day` empty.

- Time: in Los Angeles at 15:46 the wheels open at 11:46 PM, not 3:46 PM. The hour is wrong all day in every zone off UTC.
- Date: in Los Angeles from 16:00 to midnight (winter) the calendar opens with tomorrow selected. In Tokyo from midnight to 09:00 it opens with yesterday selected and today drawn as any other day.

## Verification

Run: UIKit, as a bare Mac Catalyst process (the macOS 27.2 SDK; no iOS simulator runs on this machine). A `UIDatePicker` with `makeDate`'s style, mode and UTC zone was read back at UTC, as `DateValue.format` reads it, under `TZ`:

| `TZ` and wall clock | The empty picker holds |
|---|---|
| America/Los_Angeles, 10 Nov 15:46 | time `23:46` |
| America/Los_Angeles, 10 Nov 16:00 | date `2026-11-11` |
| Asia/Tokyo, 11 Nov 08:46 | date `2026-11-10` |
| Asia/Tokyo, 11 Nov 09:00 | date `2026-11-11` |

In the inline style, which is the calendar the compact style opens, the same picker had UTC's day selected and no other day in a different ink. One left in UIKit's default zone had the wall clock's day selected.

Read, not run: the presenter, which does not build for the Mac test target.

The check that would settle it: on a simulator set to Tokyo's time zone, before 09:00, tap an empty `input type="date"` and see which day the calendar selects.

## Direction

While the value is empty, set the picker's date to the wall clock's fields read as UTC (year, month, day, hour and minute from a Gregorian calendar in the current time zone), when the picker is made and as it opens (`dateBegan`).

## Related

`issues/20261110-empty-date-cannot-take-the-day-the-picker-holds.md`: a tap on the day an empty picker holds sends no event. It notes in passing that the held day is UTC's; this issue is which day and time the person is shown, time inputs included.

`issues/20261110-date-field-now-follows-user-calendar.md`: the Mac's version of this correction misreads a non-Gregorian calendar, which is why the direction names the calendar.
