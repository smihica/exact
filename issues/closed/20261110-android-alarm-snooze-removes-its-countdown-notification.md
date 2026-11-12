# On Android, Snooze posts the new countdown's notification and the ringing service then removes it

**Status:** Closed
**Resolution:** Snooze and Repeat assign the new countdown a fresh notification code, so queued ring teardown cannot remove it.
**Systems:** packages/alarms/android/AlarmRing.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, an alarm's ring and its countdown share one notification id, the record's `code`. The ring is the foreground service's notification (`foreground(r.code, n)`, `packages/alarms/android/AlarmRing.kt:297`), and a countdown's ongoing notification is posted with `notify(r.code, n)` (`countdown`, line 96). `set` gives a replacement a new code (line 72); `snooze` keeps the record's code (lines 132 to 139).

`snooze` runs in `AlarmReceiver.onReceive`, on the main thread, in this order:

1. `end(c, r)` (line 135) calls `AlarmService.silence`, which only posts `s.end(tag)` to the main looper (line 266), so it runs after `onReceive` returns.
2. `arm(c, r)` (line 138) sets the system alarm and calls `countdown`, which posts the countdown's notification under `r.code`, replacing the ring's.
3. The posted `AlarmService.end(tag)` runs: it takes that code out of `ringing`, calls `NotificationManager.cancel(code)` (line 346) and, the last ring gone, `stopForeground(STOP_FOREGROUND_REMOVE)` (line 348), which removes the service's notification, id `r.code`.

So the notification step 2 posted is removed by step 3. The snoozed countdown still rings at its end (the alarm clock is set and the record reads `countdown`), but the shade shows nothing for it: no title, no chronometer.

## The failure

`scheduleAlarm(tag="timer", title="Tea", duration=60000, snooze=60000)`; at the ring, press Repeat on the notification.

Observed, by reading: the ring's notification goes and no "Timers" notification follows. Expected: the ongoing countdown notification LLP 1133.030 D3 gives a running countdown, as the first countdown has, and as `packages/alarms/test/checklist.md` says of Android's step 3: "Press Repeat: the ring stops and a new one-minute countdown shows in the shade."

Platform references: `Service.stopForeground(STOP_FOREGROUND_REMOVE)` removes the notification given to `startForeground` (https://developer.android.com/reference/android/app/Service#stopForeground(int)); `NotificationManager.notify` with an id already posted replaces that notification (https://developer.android.com/reference/android/app/NotificationManager#notify(int,%20android.app.Notification)).

## Verification

Read, not run: Kotlin does not run on the review machine. The path has no branch: `snooze` is reached only from the ring's notification action, where the service is running and holds the tag. The automated drive does not see it: after Repeat it checks the app's list and that the service stopped (`packages/alarms/test/android-drive.mjs:97` to `98`), not the shade, which it checks only for the first countdown (line 74).

The device check that settles it: press Repeat on a ring, then `adb shell dumpsys notification --noredact` lists no `exact.alarms.countdown` notification for the package.

## Direction

End the ring before the countdown is armed, or give the snoozed countdown a fresh code as `set` does, so the service's teardown and the countdown never name one notification.

## Related

QUEUE has no alarms entry for this and no issue names it.
