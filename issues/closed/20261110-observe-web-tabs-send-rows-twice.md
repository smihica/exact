# Observe on the web sends a queued row once per open tab

**Status:** Closed
**Resolution:** A tab appends its rows to the stored queue and sends while holding a Web Lock named for it, re-reading the queue and removing only the rows each request carried, so each row is sent once and none is overwritten.
**Systems:** modules/observe/web/service.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The web service keeps its queue in one `localStorage` key per app (`exact-observe:<id>`, `modules/observe/web/service.js:59`). `start` reads the whole queue into memory once (`:71`), and from then on every `save` writes that tab's copy back over the key (`:63-70`). Nothing says which tab owns a row.

So a second tab opened while any row is queued loads that row too, and both tabs post it. A row waits at least five seconds (the batch timer, `:190`), and for as long as the device is offline or the server asks for a retry (`:172`), so the window is every launch, not a narrow race. Rows a closed tab left behind (a failed keepalive flush) are posted by every tab open at the next dispatch. The server receives the metric or event once per tab, and counts and percentiles built from it are wrong.

The same overwrite loses rows the other way: after one tab's send, its `save` writes a queue without the rows another tab added since, and they survive only in that tab's memory until its next save.

## The failure

A page that imports `service.js` and calls

```js
window.observe = start({ projectId: 'p', endpoint: location.origin + '/collect', 'app.CFBundleIdentifier': 'com.example.tabs' });
```

opened in two tabs of one browser profile. In the first:

```js
observe.event({ kind: 'app.event', name: 'checkout.completed', attributes: {}, wall: Date.now() });
```

then the second tab is opened before five seconds pass.

Observed: two POSTs to `/collect/p/v1/logs`, each carrying `checkout.completed`. Expected: one. The Apple service has one process and a cursor per signal in SQLite (`modules/observe/apple/service/ObserveStore.swift:21`), so it sends each row once.

## Verification

Ran it in Chromium (playwright-core 1.64.1, two pages of one context, a local server counting the POSTs):

```
one tab:  alone.event posted 1 time(s); rows left in localStorage 0
two tabs: checkout.completed posted 2 time(s); rows left in localStorage 0
```

The row lost by the overwrite was read, not run.

## Direction

Give the queue one sender at a time: hold a Web Lock (`navigator.locks.request('exact-observe:<id>', …)`) around a dispatch and re-read the stored queue inside it, removing only the rows that request sent. A tab then appends its own rows to what is stored instead of replacing it.

## Related

No issue or QUEUE entry covers Observe's web queue. The Linux service has the same shape for two processes of one app (`modules/observe/linux/launch.rs:377` reads `queue.json` once, `:447` writes the process's copy over it); that was read, not run.
