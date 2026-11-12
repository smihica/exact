# Android's background sync keeps session-clock times across launches, so a backoff is measured from the wrong zero and `lastRun` is no date

**Status:** Closed
**Resolution:** Android stores background registration, retry and last-run times as epoch milliseconds, so backoffs keep their deadlines across launches.
**Systems:** packages/background/android/Background.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `BackgroundModule` on Android stamps registrations with `context.now()` and writes the stamps to its preferences: `registered` (`packages/background/android/Background.kt:166`), `lastRun` (line 239) and the backoff's `notBefore` (lines 256 and 258). `read()` loads them in the next process (line 147), and `fire()` and `ran()` compare `notBefore` with that process's `context.now()` (lines 195 to 200, 230).

`context.now()` is the session's clock: milliseconds since this launch (`host/linux/src/canvas.rs:1176`, read through `host/linux/src/canvas/jni_extensions.rs:38` to `44`). A value kept from one launch means nothing in the next. The Apple module converts for that reason: "Persist epoch times: the session clock starts over on each launch" (`packages/background/apple/Background.swift:162`).

Two results:

1. **A backoff is measured from the wrong zero.** LLP 1133.017 D1: a `sync` kept for another try "is not eligible again until a backoff has passed (5 minutes, doubling each attempt, at most an hour". A failure `T` into one launch keeps `notBefore = T + 300000`, and the next launch holds the tag until its own clock passes that, however long ago the failure was. WorkManager does not rescue it: its worker asks `ran()`, which answers `done(true)` for a tag still in backoff (line 230), so the work ends as a success.
2. **`lastRun` is not an instant.** `periodicSyncTags()` answers it (line 134). The runner's list (`runner/src/runner/modules/declared/wake_list.rs:8`) and Apple answer epoch milliseconds; Android answers milliseconds into whichever launch ran the tag.

## The failure

`registerSync(tag="outbox")` with a `sync` handler that fails and registers its tag again, two hours after launch. The process ends (swiped away, or reclaimed), and the app is reopened ten minutes later with the network up.

Observed, by reading: the tag is raised when the new session is 2 h 5 min old; an app whose sessions are shorter never raises it, and it stays in `syncTags()`. Expected: raised at the launch, the five minutes having passed; the module's own header says a `sync` "still fires at the next launch" (lines 14 to 15).

## Verification

Read, not run: Kotlin does not run on the review machine. The arithmetic of `ended`, `fire` and `ran`, ported to a script over a clock that starts at 0 with each launch, printed:

```
failed 120.0 min into launch 1, notBefore kept 7500000; launch 2 is 10.0 min later: fire() waits 125.0 min more; ran() answers done(true), nothing raised; with an epoch stamp: raised at once
failed 0.5 min into launch 1, notBefore kept 330000; launch 2 is 1440.0 min later: fire() waits 5.5 min more; ran() answers done(true), nothing raised; with an epoch stamp: raised at once
```

The device check that settles it: fail a `sync` some minutes into a launch, force-stop the app, reopen it six minutes later, and watch for the tag's `sync` event.

## Direction

Keep epoch milliseconds, as Apple's `now()` does, and convert to the session clock only for the delays handed to `postDelayed` and `setInitialDelay`.

## Related

QUEUE "Background work: what is left" lists Android's unbuilt headless session and registration words, not the clock. No issue covers it.
