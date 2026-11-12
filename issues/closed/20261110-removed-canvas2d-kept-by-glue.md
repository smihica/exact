# A Canvas 2D node that leaves the tree, and its bitmap, stay reachable for the life of the page

**Status:** Closed
**Resolution:** Both web targets release removed Canvas 2D views' observers, replayers, reported sizes and geometry waits, including on wasm plan reload.
**Systems:** host/web/canvas2d-glue.js, host/web-js/canvas2d.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The leak is in the web host's Canvas 2D glue, which both web targets load. It keeps three maps by view id (`host/web/canvas2d-glue.js:212-215`): `watched` holds the node's element, `canvases` a `Replayer` holding the bitmap's `<canvas>` and its context, `reported` its last size. A `watched` entry is removed only for a canvas gone before its first geometry (`:253`); a `canvases` entry never. The element also stays observed by the glue's `ResizeObserver` (`:235`). The glue has no operation that lets an id go: `op` takes `images`, `frames`, `watch`, `fresh` and lists (`:316-334`).

On the JS target, when a Canvas 2D node's scope ends the engine forgets its record and retires its lifetime in the data module (`host/web-js/canvas2d.js:229-235`, `gone`), and can tell the glue nothing. Its view ids are never reused (`host/web-js/rt.js:714-717`), so nothing overwrites an entry: every Canvas 2D node the page ever mounted is kept with its backing store.

A 300 by 150 canvas at a device pixel ratio of 2 has a 720,000 byte bitmap. A route with a chart keeps one for each visit, and a virtualized list whose rows draw keeps one for each row scrolled in.

## The failure

```
component App
  state on = true
  action flip
    on = not on
  view
    column
      button press=flip testId="flip"
        text "flip"
      if on
        canvas surface=dot() width=300 height=150 testId="c"
```

with an `app.ts` whose `draw` fills a square. Press `flip` 80 times, so the node is mounted 41 times and removed 40. The page keeps a `WeakRef` to each element it saw with that test id and each `<canvas>` under one. After a forced collection, at a device pixel ratio of 2:

| the node under `if on` | elements seen | alive | in the document | live DOM nodes, before and after | bitmaps out of the document |
|---|---|---|---|---|---|
| `column width=300 height=150` | 41 | 1 | 1 | 36, 44 | 0 |
| `canvas width=300 height=150` (no surface) | 82 | 2 | 2 | 37, 45 | 0 |
| `canvas surface=dot() width=300 height=150` | 82 | 82 | 2 | 38, 126 | 28,125 KB |

Expected: the third row as the second.

## Verification

Each app was built with `bun host/web/build.mjs <app> --out dist`, opened in the pinned Chromium with no `?agent` and pressed through Playwright; garbage was collected with CDP `HeapProfiler.collectGarbage`, and the node count is `Performance.getMetrics`. The table is what it printed.

With a `forget(id)` added to the glue (unobserve the element; delete the id from `watched`, `canvases`, `reported` and `waiting`) and called from `gone`, the third row read 4 alive, 38 to 48 nodes and 703 KB: the maps are what holds the rest. That change was reverted.

Not run: the wasm web host, whose removal of a view (`host/web/glue.js:826`) tells the glue nothing either.

## Direction

Give the glue an operation that lets a view id go, as above, and send it from `gone`; the wasm host's removal of a canvas node needs the same.

## Related

`issues/20261110-gpu-canvas-brush-cache-never-releases.md` and `issues/20261110-reload-keeps-gpu-surface-on-reused-view-id.md` are GPU canvases on other hosts. `issues/closed/20261102-web-js-views-retain-removed-dom.md` fixed the same retention in the runtime's `Views` map. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names the 2D glue's maps.
