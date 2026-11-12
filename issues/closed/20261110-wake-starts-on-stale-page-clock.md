# On a live page a wake delivered after an idle spell starts in the past and expires at the next timer or `then`

**Status:** Closed
**Resolution:** Wake delivery refreshes the page clock before starting its budget, preserving the full budget after an idle spell.
**Systems:** host/web-js/wakes.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A wake's budget runs on the page's clock: `deliver` records `current.started ??= clock.now` (`host/web-js/wakes.js:66`), and each time a timer or a `then` fires, `Clocked` expires the wake once `clock.now >= started + budget` (`wakes.js:38`, `:109`; `host/web-js/rt.js:343`).

Outside the agent `clock.now` is the time of the last commit or timer (`rt.js:287`). `deliver` reads it before the handler's commit takes the time (`wakes.js:75`), and nothing on the way in takes it: not the implementation's `wakes.raise` (`host/web-js/modules.js:434`), not `raise`, not `step`. So on a page that has been idle, the wake starts as long ago as the page's last commit. If that is longer than the budget, the first timer or `then` to fire while the wake's work is in flight expires it. A `then` on the handler's own send is enough: it fires when the reply lands.

The wake's own timer is on the wall clock and is right (`wakes.js:67`); the `Clocked` check beats it.

The web's wake is the page's `online` event or the end of a backoff (`packages/background/web/index.js:77`, `:79`), and both follow a spell in which an idle page commits nothing. An expired sync is counted as a failed attempt and fired again after five minutes or more (`index.js:47-58`), so the handler runs again for work that had landed.

## The failure

```
use syncTags, sync, SyncEvent, SyncRegistration from "@exact/background"
…
  mutation pulled as shape Pulled then afterPull
  action heard(tag: string, e: SyncEvent)
    send pulled = pull(tag)
  action afterPull
    done = done + 1
  view
    column sync=heard
```

`pull` awaits one request, which the server holds 500 ms. Register a tag while offline, leave the page untouched for 35 s (the budget is 30 s), and bring the network back:

```
t=60.1 wake sync t1: raised
t=60.1 wake sync t1: heard by heard
t=35585.9 wake sync t1: expired
t=35585.9 wake: expired
```

The server was asked once and `afterPull` ran, yet the tag stays listed, kept with `"attempts":1` and a `notBefore` five minutes on. Expected, and what 3 s of idling gives: `completed`, and the tag retired.

## Verification

Built with `bun host/web/build.mjs <app> --out dist` (the package by `file:`) and opened in the pinned Chromium with no `?agent`; Playwright's offline switch took the network away and back; the lines are the page's journal (`exact.journal`).

With `time()` called in `deliver` before it records `started`, the 35 s run printed `heard` at `t=35081.9` and `completed` at `t=35587.6`, and the tag was retired. That change was reverted.

## Direction

Take the page's time (`rt.js` `time`) before `deliver` records `started`.

## Related

`issues/20261110-data-timer-armed-on-stale-page-clock.md` is the same stale read in `ts-fetch.js`; each needs its own call. `issues/closed/20261102-web-tool-deadline-waits-on-stale-clock.md` fixed it in `calls.js`. `issues/20261110-expired-wake-leaves-queued-sends-writes.md` is what an expired wake leaves behind in the runner.
