# After the device's time zone changes, `toLocaleTimeString` and `Intl.DateTimeFormat` keep formatting in the old zone until the app restarts

**Status:** Closed
**Resolution:** Only explicit time zones are cached; implicit-zone date formatting reads the system zone on each call while existing formatters retain their original zone.
**Systems:** js/src/intl.js (the native executor's date formatter cache)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `js/src/intl.js:50-113` keeps one engine `Intl.DateTimeFormat` for each distinct `(locales, options)` a module formats with (`madeFor`, `intl.js:56-65`), and answers every later `new Intl.DateTimeFormat(…)`, `toLocaleString`, `toLocaleDateString` and `toLocaleTimeString` with the same arguments from it. The key is the arguments alone (`formatKey`, `intl.js:27-40`). A formatter made without a `timeZone` option takes the system's zone when it is made, so the cached one holds the zone of its first use. Nothing clears the map but its 64-entry bound.

ECMA-402 reads the zone at each construction: CreateDateTimeFormat takes `timeZone` with the fallback `SystemTimeZoneIdentifier()`, and `Date.prototype.toLocaleTimeString` creates a formatter at each call. The engine's own formatter, made at the call, follows the zone.

A module's runtime lives as long as its session, and iOS keeps a suspended app for days. Someone who flies with the app in memory sees the module's times in the zone they left, while the date the host tells the runner follows the zone (`host/apple/Sources/ExactKit/SessionTime.swift:23-35`).

## The failure

A module that answers `new Date(t).toLocaleTimeString("en-US")` and `new Intl.DateTimeFormat("en-US", { hour: "numeric", minute: "numeric" }).format(new Date(t))` for `t` = 2026-11-10T20:00:00Z, asked once in `America/Los_Angeles` and again after the process's zone becomes `Asia/Tokyo`:

| | first | after the change |
| --- | --- | --- |
| `toLocaleTimeString` | `12:00:00 PM` | `12:00:00 PM` |
| `DateTimeFormat(…).format` | `12:00 PM` | `12:00 PM` |
| its `resolvedOptions().timeZone` | `America/Los_Angeles` | `America/Los_Angeles` |
| the same options as a class instance, which `formatKey` does not key | `12:00 PM` | `5:00 AM` |
| that one's `resolvedOptions().timeZone` | `America/Los_Angeles` | `Asia/Tokyo` |

Expected: the second column in Tokyo's time, as the last two rows give, where the engine makes the formatter at the call.

## Verification

Run on macOS with the Hermes executor: a fixture module under `js/tests/fixtures` answering the calls above, and a test under `js/tests/it` that asks it, changes the zone, and asks again; it printed the table. The zone change was simulated in the process: `TZ` set, then `tzset()` and `CFTimeZoneResetSystem()`. Not run: a zone change on a device or through System Settings. That a real one reaches a new formatter is read from upstream sources: Hermes's Apple Intl takes `NSTimeZone.defaultTimeZone` when it makes one (`PlatformIntlApple.mm`, `getDefaultTimeZone`), and Foundation resets its cached zone on the system's notice (swift-foundation, `TimeZone_Cache.swift`: "On Darwin we listen for certain distributed notifications to reset the current TimeZone"). `Date.prototype.getHours()` also kept the old zone in this run; that is the engine's own offset cache, apart from this one.

## Direction

Drop the cached formatters when the system zone changes: the host hears it (`NSSystemTimeZoneDidChange` on Apple, `ACTION_TIMEZONE_CHANGED` on Android) and can tell the runtime, as it tells the runner the offset. The module has no cheap read of the zone to compare: the engine's local time did not follow the change in this run, and a new formatter's `resolvedOptions()` costs the formatter the cache exists to avoid.

## Related

`issues/20261110-apple-date-lags-after-sleep-or-a-clock-change.md` is the date and offset the session tells the runner; this is the module's own formatters, which that fix does not reach. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` covers the formatter cache.
