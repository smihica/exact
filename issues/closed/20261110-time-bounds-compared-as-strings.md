# A time or datetime-local control's `min` and `max` are compared as strings, so a value at its bound written with other precision is refused

**Status:** Closed
**Resolution:** Date and time bounds compare parsed milliseconds, accept equivalent precision, and honor time ranges that cross midnight.
**Systems:** runner/src/runner/control.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`control_payload` holds a `date`, `time` or `datetime-local` control's `change` to its `min` and `max` by comparing the strings (`runner/src/runner/control.rs:99-110`, at `dd6f5dcd`: `text.as_str() < *m`, `text.as_str() > *m`). HTML compares the numbers they convert to. For `type=time` the conversion is "the number of milliseconds elapsed from midnight to the parsed time", and a value underflows when "the number obtained from that algorithm is less than the minimum" (HTML, "Time state (`type=time`)" and "The `min` and `max` attributes"). String order differs in two ways:

1. A valid time string is `hh:mm`, `hh:mm:ss` or `hh:mm:ss.sss` (the kernel's `valid_time`, `kernel/src/control.rs:198-214`, accepts all three), and `09:00` sorts before `09:00:00` though they are the same time. A value equal to its bound is refused when the two are written with different precision. The Apple host reports `HH:mm` (`host/apple/Sources/ExactKit/ControlKinds.swift:97`, `:111`) and gives its picker 09:00 as the minimum for `min="09:00:00"` (`:103`), so the earliest time the picker offers is refused.
2. A time control "has a periodic domain", and one whose maximum is less than its minimum has a reversed range, where only a value "more than the maximum and less than the minimum" is out of range (HTML, same sections). The runner refuses every value of such a control.

A refused `change` leaves the app's state behind what the control shows.

## The failure

```
input type="time" value=alarm min="09:00:00" max="17:00" change=setAlarm
input type="datetime-local" value=at min="2026-01-01T09:00:00" max="2026-12-31T17:00" change=setAt
input type="time" value=night min="22:00" max="06:00" change=setNight
```

Dispatch `Event::Change` with each value. Observed:

```
alarm  09:00                InvalidValue: "09:00" is before min "09:00:00"
alarm  17:00:00             InvalidValue: "17:00:00" is after max "17:00"
at     2026-01-01T09:00     InvalidValue: "2026-01-01T09:00" is before min "2026-01-01T09:00:00"
at     2026-12-31T17:00:00  InvalidValue: "2026-12-31T17:00:00" is after max "2026-12-31T17:00"
night  23:00                InvalidValue: "23:00" is after max "06:00"
night  05:00                InvalidValue: "05:00" is before min "22:00"
```

Expected: all six accepted. `alarm` at `08:59` or `17:01` and `night` at `12:00` are refused, as they should be.

## Verification

A temporary test booted an app with the three controls through `Runner::boot` and called `Runner::dispatch` with each value. It printed the refusals above, accepted `09:00:00` and `17:00` for `alarm`, and refused `22:00` and `06:00` for `night` too. The Apple host's picker was read, not run.

## Direction

Compare what the strings mean: parse the value and each bound to milliseconds, as HTML's string-to-number algorithm does, and for a time whose `max` is below its `min` refuse only a value between them.

## Related

`contract/cli/tests/it/controls.rs`'s `a_date_carries_htmls_value_format_within_min_and_max` covers dates of one precision only. The same comparison misorders a date whose year has five digits, filed separately; parsing fixes both. No issue or QUEUE entry.
