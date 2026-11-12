# `time`'s refusals and its `vocab` entry tell the author to write `date("…")`, a function Contract does not have

**Status:** Closed
**Resolution:** Time refusals and vocabulary describe numeric instants from data sources or exactTime(), with examples that compile.
**Systems:** contract/lower (live_time.rs, values.rs, vocab.rs)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A `time` element's `datetime` is an instant in epoch milliseconds. Three places tell an author
who has a date string to convert it with `date(…)`:

- `contract/lower/src/live_time.rs:26`, for a string literal: "`datetime` is an instant in
  epoch milliseconds; write `datetime=date("2026-10-17T12:00:00Z")` to parse an ISO date"
- `contract/lower/src/values.rs:1205-1211`, for a bound string: "… parse a string with
  `date(…)`"
- `contract/lower/src/vocab.rs:173`, printed by `contract vocab time`: "`datetime` is an instant
  in epoch milliseconds, as `date(...)` returns"

The roster has no `date`. `contract vocab --json` lists 68 functions; none is `date` and none
parses a date string.

## The failure

```contract
component App
  view
    time datetime="2026-01-01"
```

`contract build` refuses it:

```
app.contract:3:10 [lower-time-instant] `datetime` is an instant in epoch milliseconds; write `datetime=date("2026-10-17T12:00:00Z")` to parse an ISO date
```

Writing what it says, `time datetime=date("2026-10-17T12:00:00Z")`, is refused again:

```
app.contract:3:19 [type-unknown-function] `date` is not in the stdlib roster and is not an action; data comes from a `resource`
```

With `state s = "2026-01-01"` and `time datetime=s` the refusal is the second text:

```
app.contract:4:10 [lower-time-instant] `datetime` is an instant in epoch milliseconds; parse a string with `date(…)`
```

Expected: each refusal names a repair that compiles. The instant comes from data (a source's
number field) or from the reserved source's epoch, as `contract vocab date` and
`docs/recipes/dates-and-days.md` show:

```contract
shape Time
  epochAtZero: number
  utcOffset: number
component App
  resource wall = exactTime() as shape Time
  view
    time datetime=(wall.epochAtZero + performanceNow()) format="relative"
```

## Verification

Ran `contract build` on the four programs above (the first three refused as quoted, the last
built) and on `derive d = date("2026-10-17T12:00:00Z")` (the same `type-unknown-function`);
ran `contract vocab time` and listed the functions of `contract vocab --json`.

## Direction

Reword the two refusals and the `vocab` row to say where the number comes from (a source's
field in epoch milliseconds, or `exactTime()`'s `epochAtZero + performanceNow()`), and drop
`date(…)`.

## Related

No entry in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names `lower-time-instant` or
`date(`. `issues/20261110-time-bounds-compared-as-strings.md` is about `input type="time"`, not
the `time` element.
