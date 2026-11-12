# On a live page a data source's `setTimeout` or `AbortSignal.timeout` made after an await fires early

**Status:** Closed
**Resolution:** Data timers and carried timeout deadlines sample the current page clock after an await.
**Systems:** host/web-js/ts-fetch.js, host/web-js/ts-data.js
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

An answer's timer is an entry of the page's clock, due at `clock.now + ms` (`host/web-js/ts-fetch.js:173`, in `arm`). Outside the agent `clock.now` moves only when something takes the time: a commit, or a timer firing (`host/web-js/rt.js:287`, `time`; `rt.js:332`). An answer's continuation after an `await` is neither, and `arm` does not take the time: ts-data.js hands it `{ clock, journal, drive }` (`host/web-js/ts-data.js:26`). So a timer armed after an await is due `ms` after the page's clock last moved, and fires early by however long the answer has awaited since, up to its whole delay. `AbortSignal.timeout` arms the same way (`ts-fetch.js:298`).

LLP 1027.000 D1 (amended 2026-10-28) says a timer is due at the clock "as its call began plus its delay". Natively each continuation is a call that reads the runner's clock (`js/src/binary.js:368-372`). The page module's `after` takes the time before it arms (`host/web-js/hatches.js:386-389`). `docs/reference.md:1061-1063` names a retry's back-off and a fetch's `AbortSignal.timeout` as the uses.

## The failure

```ts
const sleep = (ms: number) => new Promise<void>(r => setTimeout(r, ms));
export async function answer(name: string): Promise<any> {
  await sleep(300);                                   // armed in the answer's own turn
  await fetch('/assets/slow.txt');                    // the server holds it 1500 ms
  await sleep(1000);
  await fetch('/assets/slow.txt');
  try { await fetch('/assets/slower.txt', { signal: AbortSignal.timeout(2000) }); } catch (e) {}   // held 1000 ms
  try { await fetch('/assets/never.txt', { exactTimeout: 1200 } as any); } catch {}                 // held 5000 ms
  await sleep(800);                                   // a retry's back-off
}
```

read by `resource r = steps() as shape string` on a page with nothing else going on. Real time, measured in the source:

| step | took | expected |
|---|---|---|
| `sleep(300)` before any await | 303 ms | 300 ms |
| `sleep(1000)` after the held fetch | 0 ms | 1000 ms |
| the fetch under `AbortSignal.timeout(2000)` | `TimeoutError` after 501 ms | a reply after 1000 ms |
| `sleep(800)` after the request that timed out | 0 ms | 800 ms |

A task on the page that fires every 200 ms keeps the clock at most 200 ms old: the last three then read 999 ms, a reply after 1003 ms, and 687 ms.

## Verification

Built with `bun host/web/build.mjs <app> --out dist` and opened in the pinned Chromium with no `?agent`, from a Bun server that held the assets. The source read the page's `performance.now()`, handed in as a global by an init script (its own is refused), and printed the table.

With `time` added to `answering.page` and called at the top of `arm`, the same run printed 1001 ms, a reply after 1003 ms, and 806 ms, and `bun test host/web-js/answer-timers.test.mjs` passed. That change was reverted.

Under the agent the clock is the driver's and never stale, so authored tests and `answer-timers.test.mjs` (a fixed `clock.now`) cannot see this. No data source in `apps/` arms a timer, so no app there shows it.

## Direction

Take the page's time (`rt.js` `time`, a no-op under the agent and inside a firing timer) before a timer is armed, handed to ts-fetch.js beside `clock` and `drive`. `signalDeadline.arm` reads `clock.now` for the time a signal has left (`ts-fetch.js:223`) and needs the same.

## Related

`issues/20261110-wake-starts-on-stale-page-clock.md` is the same stale read in `wakes.js`, and `issues/closed/20261102-web-tool-deadline-waits-on-stale-clock.md` fixed it in `calls.js`. `issues/20261110-ordered-main-child-loses-runner-clock.md` is a native path where a turn's clock reads zero.
