# The calendar example reads every day at today's UTC offset, so device events across a daylight-saving change show an hour off

**Status:** Closed
**Resolution:** The example passes `time.timeZone` and the local day or month, and the data crate resolves each day's midnights and each instant's local day and time with the zone database (`chrono-tz`), so every read uses the offset in force on that day.
**Systems:** examples/calendar (src/app.contract, data/src/device.rs, data/src/activity.rs)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

The example asks for the person's calendar and activity with `time.utcOffset` (`examples/calendar/src/app.contract:85-86,94-95` at `dd6f5dcd`):

```
resource mineAgenda = calendarDeviceAgenda(day * 86400000 - time.utcOffset * 60000, mineRead, time.utcOffset)
resource mineDays = calendarDeviceDays(month, mineRead, time.utcOffset)
```

`time.utcOffset` is the zone's offset now, not the offset of the day asked (`docs/contract-for-agents.md:2851-2858`: "It is not the offset of an arbitrary timestamp"). The data crate uses that one offset for the day's window (`examples/calendar/data/src/device.rs:72-86`), for the local day an event marks (`device.rs:150-168`) and for the times it prints (`device.rs:173-189`). `activity.rs:104-131,170-181` does the same for steps and workouts.

So on any day whose offset differs from today's, the window is the wrong 24 hours and every time is off by the difference. In a zone with daylight saving that is every day on the other side of the last or next change, about half the year. A month whose six weeks span a change has it without leaving today's month.

## The failure

Today is 2026-11-10 in Los Angeles, so `time.utcOffset` is -480. The person selects 4 July, when the zone is at -420. All four reads are affected:

| Read | On 4 July 2026 | Observed | Expected |
|---|---|---|---|
| `calendarDeviceAgenda` | an event from 9:00 to 10:00 (`utcStart` 16:00 UTC) | `8:00 AM – 9:00 AM`; the window starts at `1783152000000`, 01:00 local | `9:00 AM – 10:00 AM`; window from `1783148400000` |
| `calendarDeviceDays` | an event at 00:30 on 5 July | marks 4 July | marks 5 July |
| `calendarActivityAgenda` | a run from 9:00 to 10:00 | `8:00 AM – 9:00 AM`, window from 01:00 | `9:00 AM – 10:00 AM` |
| `calendarActivityDays` | July's daily steps | asked from `1782633600000`, an hour after local midnight | from `1782630000000` |

The app's own schedules are wall-clock days and are not affected.

## Verification

Ran tests in `device.rs` and `activity.rs` that drive `answer` and `parse` with the arguments the Contract passes, once with -420 and once with -480. With -420 every row above printed the expected value; with -480 each printed the observed one. Not run: a device with a real calendar or Health data.

## Direction

The offset has to be the one in force on the day read, which a Rust data crate cannot get today. Either the crate resolves it from `time.timeZone` with a zone database, or `@exact/calendar` and `@exact/health` answer each record's wall time in the device's zone, which the platform already knows.

## Related

No issue, QUEUE entry or pitfall mentions the example's device reads. The two closed issues about offsets and time zones (`20260927-launch-facts-not-agent-deterministic`, `20260927-linux-no-place-or-seed`) are about launch facts.
