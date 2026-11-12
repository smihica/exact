# An empty date input on the Mac shows today while focused and stays empty when the person types today's date

**Status:** Closed
**Resolution:** Typing, an arrow or Return accepts an empty Mac date field's displayed value once; focus and Tab leave it empty.
**Systems:** host/apple/Sources/ExactKit/Mac/DatesMac.swift (DateField, ControlHost.dateChanged)
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

HTML's empty `input type="date|time|datetime-local"` is a flag `DateField` keeps beside the `NSDatePicker`'s own `dateValue`, which AppKit always has (`host/apple/Sources/ExactKit/Mac/DatesMac.swift:101`). An empty field that takes the focus is given the wall clock's date and time and drawn in the text colour (`:110-118`), so the person sees today's date in it. Only `ControlHost.dateChanged` ends the empty state and reports a value (`:44-50`). It is the picker's action, which AppKit sends when the date changes, and typing the date the field already shows changes nothing. When the field loses the focus it is drawn empty again (`:106-109`, `:122-127`).

So a person cannot enter the date the field opens on, which is today, by typing it or by pressing Return. The comment at `:74-75` states the rule: "only a change commits it". A `time` and a `datetime-local` cannot take the value they open on either.

## The failure

```
component App
  state due = ""
  state changes = 0
  action setDue(v: string)
    due = v
    changes = changes + 1
  view
    column padding=16 gap=12
      text `due "${due}" changes ${changes}`
      input type="date" value=due change=setDue placeholder="Due" testId="due"
      input type="text" testId="other" placeholder="Other"
```

On 2026-11-10, focus the date field (it shows 11/10/2026), type `11`, Right Arrow, `10`, Right Arrow, `2026`, then leave it. Observed: `due` is `""`, `changes` is 0, and the field shows "Due" again. Expected: `2026-11-10` after `input` and `change`, as a browser reports a date typed into an empty date input. The way round is to step the date away and back.

## Verification

Run as a temporary Mac XCTest (`bun host/apple/build.mjs --test --only <class>`): the page above booted in a session in a window, the field made first responder, and each key sent as the agent's `type due key <k>` sends it. What the presenter reported and what the app held after each step:

```
focused                                picker holds 2026-11-10T16:17, drawn in the text colour
typed 1 1 Right 1 0 Right 2 0 2 6      nothing reported; due "", changes 0
left                                   placeholder drawn; due "", changes 0
focused, Return, left                  nothing reported; due "", changes 0
focused, Up Arrow                      2027-11-10 input change; changes 1
Down Arrow                             2026-11-10 input change; changes 2
```

Control: `0 3` typed into an empty field's month reported `2026-03-10` at once. An empty `time` given the hour it showed and Return, and an empty `datetime-local` given its month and Return, reported nothing and were empty again when left.

Chromium 156, the same digits typed into an empty `<input type="date">`: `value` is `2026-11-10`, with `input` and `change` fired.

## Direction

When a person edits an empty field and the picker sends no action, report the date it holds: a key the picker takes (a digit, an arrow, Return) ends the empty state as `dateChanged` does. Tabbing through the field leaves it empty.

## Related

`issues/20261110-empty-date-cannot-take-the-day-the-picker-holds.md` is the same rule in iOS's calendar and says the Mac's field "was not checked". `issues/20261110-date-field-now-follows-user-calendar.md` is about the calendar in which lines `:114-116` read the wall clock.
