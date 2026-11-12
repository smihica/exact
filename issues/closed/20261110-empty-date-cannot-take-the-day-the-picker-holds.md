# An empty date input on iOS ignores a choice of today, or of the date it held before it was cleared

**Status:** Closed
**Resolution:** Opening an empty iOS date, time or datetime picker accepts the current local value within its bounds, including after a clear.
**Systems:** host/apple/Sources/ExactKit/IOS/DatesIOS.swift (BoundDatePicker, ControlHost.dateChanged, configureDate)
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

HTML's empty `input type="date|time|datetime-local"` is a state `BoundDatePicker` keeps beside the `UIDatePicker`'s own `date`, which UIKit always has (`host/apple/Sources/ExactKit/IOS/DatesIOS.swift:94-106`, at `dd6f5dcd`). For a person, only `ControlHost.dateChanged` ends that state and reports a value (lines 52-58), and it hears `.valueChanged` alone (line 18).

UIKit sends `.valueChanged` when the date changes, not when a day is tapped: "Date pickers use the target-action design pattern to notify your app when the user changes the selected date" (Apple, UIDatePicker). The calendar an empty picker opens has the picker's `date` selected. For a new picker that is the day it was made ("default is current date when picker created", `UIDatePicker.h`), read at UTC (line 17). After a value was cleared it is the day last chosen, since `configureDate` leaves `date` alone when the bound value becomes empty (lines 42-49). A tap on the selected day changes nothing, so UIKit sends nothing and the input stays empty.

## The failure

```
input type="date" value=due placeholder="Due" change=setDue
button press=add
  text "Add"
```

`add` saves the task and writes `due = ""`.

1. `due` is `""`. Tap the field, then today in the calendar. Observed: no `input`, no `change`; with the calendar dismissed the field still says "Due". Expected: today's date, as on the web.
2. Choose 12 November, press Add, choose 12 November for the next task. Observed: nothing; the second task has no date.

A `min` after today moves the held date to `min`, so an empty field cannot take its first allowed day. The way round is to choose another day, then the wanted one.

## Verification

UIKit's half was run as a bare Mac Catalyst process (the macOS 27.2 SDK's UIKit): a `UIDatePicker` in the inline style, which is the calendar the compact style opens, a closure action on every control event, and days chosen through its calendar's collection view delegate.

```
chose the day it holds (10): valueChanged fired 0 time(s); date unchanged: true; other events: []
chose another day (1): valueChanged fired 1 time(s) in all; date changed: true
```

A fresh picker given a `minimumDate` three days ahead held that date. The web: in the pinned Chromium, opening an empty `<input type="date">`'s calendar and pressing Enter on the day it opens on set `2026-11-10` and fired `input` then `change`; Safari on iOS writes today into an empty input as its picker opens (WebKit, `WKDateTimeInputControl.mm`, `setDateTimePickerToInitialValue`).

Exact's half was read: nothing else in `ExactKit/IOS` hears a date picker but `dateBegan` (`.editingDidBegin`, line 19), which reports no value. Not run: the compact picker's popover on a simulator (none runs on this machine).

## Direction

`UIDatePicker` cannot tell a tap on its selected day from no tap. Either report the held date when an empty picker opens, as Safari does (`dateBegan` hears that; the date should then be the wall clock's day, as the Mac's field makes it, `Mac/DatesMac.swift:112-117`), or show the empty state's calendar with `UICalendarView`, whose `UICalendarSelectionSingleDate` starts with no `selectedDate` and reports every tap.

## Related

QUEUE's "iOS sheets, type and control states, what is owed" lists "the calendar opening from an empty date" as not run and names no defect. The Mac's field has the same rule ("only a change commits it", `Mac/DatesMac.swift:75`) and was not checked.
