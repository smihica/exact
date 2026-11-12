# fragment.js keeps every element that ever scrolled, so removed routes are never collected

**Status:** Closed
**Resolution:** Fragment scroll places hold elements weakly, and every history push discards the forward entries it replaces.
**Systems:** host/web/fragment.js, host/web-js/router.js, host/web/glue.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`fragment.js` records where each history entry left its scrollers, so that Back or Forward between two fragments
of one visit can put the place back (LLP 1153 D5). The record is `kept`, a `Map` from the entry's index to a
`Map` keyed by element (`host/web/fragment.js:13`, at `dd6f5dcd`). A window `scroll` listener in the capture
phase adds the target of every scroll event on the page to the current entry's map (`:14-19`), whatever scrolled
and whether or not a fragment is involved.

Nothing takes an element out. The only deletion is in `arm`, for a router change that pushed an entry of the
same visit and asked for a fragment scroll: it drops the maps of that entry and later ones and replaces the map
of the entry it left (`:59-62`). An ordinary push, a replace, a pop, a closed sheet or a menu that scrolled
leaves its elements in the map of the entry that was current, and the next route at that depth adds to the same
map. The keys are strong references, and a scroller kept this way keeps the detached route it was removed with
alive. So each removed route that scrolled stays in memory until the page goes.

The module loads at the first router change that asks for a fragment scroll (`host/web-js/router.js:169`,
`:196`; `host/web/glue.js:621`) and listens from then on: once a link to a fragment is followed.

## The failure

A page that stays on one history entry (`history.state.exact === 1`) does this forty times: mount a route
holding a 200-row scroller, set its `scrollTop`, wait for the scroll event, hold the route in a `WeakRef`,
remove it. Then collect garbage three times (CDP's `HeapProfiler.collectGarbage`).

Observed: with `import './fragment.js'`, 40 of the 40 removed routes are still reachable. Without the import, 0.

Expected: a route that left the page is collectable.

## Verification

Run in the pinned headless Chromium (156) with the real module and real scroll events. It printed
`removed routes still alive after GC, of 40: without fragment.js 0, with fragment.js 40`. Not run: a compiled
app.

## Direction

Hold the elements weakly (a `WeakRef` per place), and drop an entry's map when a push writes its index again,
as navigation.js truncates `written` then. The restore loop already skips an element that is not connected.

## Related

`issues/closed/20261102-web-js-views-retain-removed-dom.md` fixed the same pattern in the JS runtime's `Views`
registry, and `issues/closed/20261102-web-selection-watch-retains-removed-text.md` in navigation.js's selection
watch. Neither covers `kept`. `QUEUE.md`, "The fragment after LLP 1153 D5: five edges the last review left",
lists five other defects of this file.
