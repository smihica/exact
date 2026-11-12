# On Linux, Android and Windows the date an app reads falls behind after a sleep, and a clock change is never told

**Status:** Closed
**Resolution:** `Presenter::follow_local_offset` retells the date at the live clock's zero when it moved by a second or more, run before each advance and frame on Linux, at each Android loop turn and canvas reader frame, and at each Windows turn, which a Linux `CLOCK_REALTIME` cancel-on-set timer and Windows `WM_TIMECHANGE` and `PBT_APMRESUMEAUTOMATIC` wake.
**Systems:** host/linux/src/app.rs, host/linux/src/presenter/preferences.rs (`follow_local_offset`), host/linux/src/android.rs, host/windows/src/window.rs
**Severity:** P2
**Author:** Claude (Opus 5.5), from fixing the Apple host's (issues/closed/20261110-apple-date-lags-after-sleep-or-a-clock-change.md)
**Date:** 2026-11-11

## What is wrong

Each host derives the date an app reads (`epochAtZero`) from `SystemTime` less a
`std::time::Instant` taken at launch (`host/linux/src/app.rs`), as Apple did before its fix.
`Instant` reads `CLOCK_MONOTONIC` on Linux and Android, which stops while the system is
suspended, so after a sleep the date is behind by the sleep's length. A clock set by hand or by
NTP is never told on any of the three:

- Linux retells the date only when the zone's offset moves (`follow_local_offset`, run in the
  display loop alone).
- Android's loop (`android.rs`) never retells after launch.
- Windows's window (`window.rs`) never retells after launch. Whether its `Instant` stops in sleep
  was not established.

## To see it

Read from the code, by the agent that fixed the Apple host; not run on these hosts.

## What a fix is

As on Apple: retell the date when it has moved by a second or more from the one last told, at
each check of the loop, and on the platform's clock-change and resume notices
(`CLOCK_REALTIME` compared against the last told value; Windows `WM_TIMECHANGE` and
`WM_POWERBROADCAST`/`PBT_APMRESUMEAUTOMATIC`, https://learn.microsoft.com/windows/win32/sysinfo/wm-timechange).
