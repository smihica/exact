# Android's calendar queries all-day occurrences by their UTC instants, and names a recurring one's `recurrenceId` in UTC

**Status:** Closed
**Resolution:** Android calendar queries include all-day occurrences by their local spans and name recurring occurrences by their original local date.
**Systems:** packages/calendar/android/EventStore.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

The Calendar Provider keeps an all-day occurrence at UTC midnights, and `EventStore` reads it back as the local dates it names: `occurrence` converts both instants to the device zone's midnights before it answers `start`, `utcStart` and `utcEnd` (`packages/calendar/android/EventStore.kt:217` to `220`, at `dd6f5dcd`). Two places skip that conversion.

1. **The range query.** `page` passes the caller's range straight to the provider (`occurrences(from.toLong(), end.toLong(), only)`, line 189), whose `instances/when/<begin>/<end>` query keeps rows with `begin <= rangeEnd AND end >= rangeBegin` on the stored instants. The module's own filter, on the converted instants (line 190), can only drop rows. An all-day occurrence whose local span overlaps the range and whose UTC span does not is never returned.
2. **`recurrenceId`.** For a recurring occurrence, `original` is `Instances.BEGIN` or `ORIGINAL_INSTANCE_TIME` (line 223), a UTC midnight for an all-day event, formatted in the device zone unconverted (line 224).

## The failure

An all-day event on 2026-10-23, kept as 2026-10-23T00:00Z to 2026-10-24T00:00Z. The arithmetic of lines 189, 190 and 218 to 224, ported to a script:

| Zone | `calendarEvents` range | The module's `utcStart` to `utcEnd` | Overlaps | Provider returns the row |
|---|---|---|---|---|
| America/Los_Angeles | the whole day | 23T07:00Z to 24T07:00Z | yes | yes |
| America/Los_Angeles | 18:00 to midnight (24T01:00Z to 24T07:00Z) | the same | yes | **no** |
| America/Los_Angeles | the next seven days, from 18:00 | the same | yes | **no** |
| Asia/Tokyo | 01:00 to 03:00 (22T16:00Z to 22T18:00Z) | 22T15:00Z to 23T15:00Z | yes | **no** |

So west of UTC a list of what is still ahead, asked in the evening, loses today's all-day events; east of UTC a range ending in the morning does. Expected: the event, since `calendarEvents` answers "a page of the occurrences that overlap `[start, end)`" (LLP 1133.018 D2).

If that event repeats yearly, its `recurrenceId` reads `2026-10-22T17:00:00` in Los Angeles and `2026-10-23T09:00:00` in Tokyo, beside a `start` of `2026-10-23T00:00:00`. Expected: "an occurrence's original start, LocalDateTime" (LLP 1133.018 D2; RFC 8984 §4.3.1), as Apple answers (`packages/calendar/apple/EventStore.swift:206`).

## Verification

Read, not run: Kotlin does not run on the review machine, and the Android drives of this package have not run (QUEUE, "Contacts and calendar: what the four stages left"). The provider's rule is AOSP's `CalendarProvider2.handleInstanceQuery` (`SQL_WHERE_INSTANCES_BETWEEN`, with no all-day case), and `CalendarInstancesHelper` expands all-day events in UTC. `CalendarContract.Events`: "If allDay is set to 1 eventTimezone must be "UTC" and the time must correspond to a midnight boundary" (https://developer.android.com/reference/android/provider/CalendarContract.Events).

The device check that settles it: on an emulator set to America/Los_Angeles with an all-day event today, read `calendarEvents(start=<18:00 today>, end=<midnight>)`.

## Direction

Ask the provider for the range widened by a day each side (line 190 already trims to the converted instants), and compute an all-day `recurrenceId` from the converted start.

## Related

QUEUE "Contacts and calendar: what the four stages left" says Android is built and not run; it names no defect.
