# On Apple the date an app reads falls behind by the time the system slept, and a system clock change is never told

**Status:** Closed
**Resolution:** The Apple session tells the date again whenever it has moved by a second or more from the one last told, at each timer's and frame's check and on Foundation's clock and zone change, AppKit's wake and UIKit's activation and significant time change notices.
**Systems:** host/apple/Sources/ExactKit/SessionTime.swift, host/apple/Sources/ExactKit/Session.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

An app's date is `time.epochAtZero + performanceNow()` (LLP 1027.000.000 D1; `docs/contract-grammar.md:1013`). On Apple the session's clock is `ExactEnv.wall()`, `(CACurrentMediaTime() - t0) * 1000` (`host/apple/Sources/ExactKit/Session.swift:50-51`, `:593`). `epochAtZero` is told at each boot (`primeTime`, `tellTime`) and afterwards only by `followOffset` (`SessionTime.swift:23-35`), which sends the date only when the zone's offset moved:

```swift
let date = dateNow()
guard date.offset != toldOffset else { return }
```

`CACurrentMediaTime()` is "the result of calling mach_absolute_time () and converting the units to seconds" (`QuartzCore/CABase.h`), and that clock stops while the system is asleep (`mach/mach_time.h`: `mach_continuous_time` is "like mach_absolute_time, but advances during sleep"). `Date()` does not stop. After a sleep of length T the session's clock is T short, so the sum is T behind the real date, and nothing tells the runner a new `epochAtZero`.

A clock change is not told either. LLP 1027.000.000 D2 says hosts call `set_time` "on a system clock or zone change", for Apple on `NSSystemClockDidChange` and `NSSystemTimeZoneDidChange`. Nothing under `host/apple` observes `NSSystemClockDidChange`, a wake from sleep or a day change, and `setTime(` has two callers: `followOffset` and the agent's `tellAgentOffset` (`SessionTime.swift:34`, `:51`).

## The failure

Any app that declares `resource time = exactTime() as shape T` and computes the date as the grammar says. Ten apps under `apps/` read `exactTime`, Messages, Shelf and Weatherlight among them; `apps/auth-fixture/app.contract:81` is the shortest example:

```contract
send prepared = prepareSignIn(time.epochAtZero + performanceNow())
```

Leave the Mac app open, let the Mac sleep for an hour and wake it: the sum is an hour short, and stays so until the app is relaunched or the zone's offset changes. The same holds for an iPhone app that stays in memory while the phone sleeps, and on either platform when the person or the network sets the clock. Every relative time, countdown and "today" computed from the date is off by that amount.

## Verification

By reading; no sleep or clock change was driven. This Mac has not slept since it booted (`sysctl kern.sleeptime` is zero), so the two clocks could not be shown apart: a Swift script printed `CACurrentMediaTime()`, `mach_absolute_time()` and `mach_continuous_time()` within 8 µs of one another. The runner would take the correction: `set_time` accepts any epoch that differs by a second or more (`runner/src/runner/time.rs:60-65`).

## Direction

In `followOffset`, tell the runner when `dateNow().epoch` differs from the epoch last told by a second or more, as well as when the offset moved, and make that comparison outside the `offsetHeld` hour. It already runs before each advance and frame. Observing `NSSystemClockDidChange` and the app's activation covers an app with no timer running.

## Related

`issues/closed/20260904-data-sources-read-ambient-time.md` is about data sources reading the ambient clock. QUEUE.md has no entry on sleep or a clock change. The Linux presenter's `follow_local_offset` (`host/linux/src/presenter/preferences.rs:68-75`) also retells only a moved offset, though D2 says "Linux: at resume"; it was not examined further here.
