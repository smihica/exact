# The native pedometers use the session clock where they need the time since the epoch

**Status:** Closed
**Resolution:** Native pedometers use epoch time for boundaries and timestamps; Android refresh intervals use the session clock.
**Systems:** packages/pedometer/apple/Pedometer.swift, packages/pedometer/android/Pedometer.kt
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`pedometer`'s `start`, `since` and `timestamp` are milliseconds since the epoch: the declared
substitute answers them on the epoch (`runner/src/runner/modules/samples.rs:376-405` adds
`epoch_at_zero` to the session clock), and Fieldnotes passes
`walkStart = time.epochAtZero + performanceNow()` (`apps/fieldnotes/app.contract:248`,
`app.ts:216`). A package's `context.now()` is the session clock, which counts from the process's
start (`host/apple/Sources/ExactKit/Session.swift:51`, `:593`;
`host/linux/src/canvas/jni_extensions.rs:38`). At `dd6f5dcd` both native implementations use it
as epoch time.

**Apple** (`packages/pedometer/apple/Pedometer.swift:134-135`):

```swift
let asked = context.now()
group = Group(boundary: Date(timeIntervalSince1970: max(start ?? asked, asked - Self.kept) / 1000))
```

With no `start` the boundary is a few seconds into 1 January 1970, so `startUpdates(from:)`
answers everything Core Motion keeps, the past seven days, where the count should begin at the
first ask (LLP 1133.026 D14). `asked - kept` is negative, so the seven-day clamp never applies.
A reader that passes an epoch `start` is answered correctly.

**Android** (`packages/pedometer/android/Pedometer.kt:114-123`). `now` is the group's `start`
when none is given, the subscription's time (`:101`), and the end of the Recording read, whose
range the API takes in seconds since the epoch.

- With an epoch `start`, as Fieldnotes passes, `from` is later than `now`.
  `LocalDataReadRequest.Builder.build()` requires the end to be after the start and throws; the
  host's catch answers `NotSupportedError`
  (`host/android/canvas/src/main/java/dev/exact/canvas/Extensions.kt:526-528`). The walk shows no
  steps.
- With no `start`, every read covers seconds of 1970 and totals 0, and `Count.recorded` restarts
  the count from that total. The count falls to 0 every 60 s (`READ_EVERY_MS`) and never includes
  steps taken while the app was closed.
- On the Recording and the sensor path alike, `timestamp` (`:134-136`) reads as 1970, and so
  does `since` wherever the module sets it instead of repeating `start`.

## Verification

Read, not run on a phone: a simulator and an emulator have no step counter, and under the agent
the substitute answers. On a Mac:

- Apple's boundary expression, `asked` taken as `ExactEnv.wall()` takes it 1.5 s after start:
  no `start` gives `1970-01-01T00:00:01Z`; a `start` 30 days back stays 30 days back.
- A line-for-line JavaScript port of `Count.kt` and of `read`'s range arithmetic, fed two steps a
  second: `pedometer()` reads 118 steps at 62 s, 0 at 63 s, 118 at 122 s and 0 at 123 s, with
  `since` and `timestamp` in 1970; with an epoch `start` the request fails "Invalid end time".
  Given an epoch clock the same port reads 120, 122, 240 and 242.
- `build()`'s checks, read from the bytecode of play-services-fitness 21.3.0: start above 0, end
  above start.

The device check: on an iPhone that counted steps this week, the first answer of
`resource steps = pedometer()` after a fresh launch should be 0; on an Android phone with current
Play services, Fieldnotes' Start walk should show steps.

## Direction

Take the wall clock's epoch time (`Date()`, `System.currentTimeMillis()`) for boundaries, ranges
and stamps, and keep the session clock only for the intervals inside `Count`.

## Related

`packages/pedometer/test/checklist.md` item 7 owes the device check of `startDate`. No issue or
QUEUE entry names the clock.
