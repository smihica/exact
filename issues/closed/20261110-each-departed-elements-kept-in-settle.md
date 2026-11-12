# The JS runtime keeps every memo of an ended scope: a list resource's departed elements and a row's bound SVG geometry

**Status:** Closed
**Resolution:** Disposed scopes remove their memos from settlement, releasing departed list resources and SVG bindings.
**Systems:** host/web-js/rt.js, host/web-js/each.js, host/web-js/svg-transform.js
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `memo` pushes each derive's and resource's node onto the module's `Settle` list
(`host/web-js/rt.js:118`), which `settle` walks at every commit (`rt.js:162`) and nothing shortens. That is sound
for a `memo` made once, at the plan's root. Two callers make one inside a scope that ends:

- A resource over a list makes one `res` for each element that joins (`host/web-js/each.js:23`). When an element
  leaves, `each.js:31-37` ends its scope and takes it out of `Resources`, but its node stays in `Settle`. The node's
  function closes over the element's resource, so its last answer, its slots and its closures stay reachable.
- `svgGeometry` makes two memos for each bound geometry attribute of the SVG elements that
  `host/web/src/element.rs:701-714` lists, such as a nested `svg`'s `x` (`host/web-js/svg-transform.js:6-7`,
  `host/web-js/src/rows.rs:218-227`). In a `for` row they outlive the row.

Every element or row that ever existed stays on the heap for the life of the page, and every later commit visits
its dead nodes.

## The failure

One element (or row) replaced at each press, in Chrome; the table is the heap's growth, read after a forced
collection:

```
component App
  state n = 0
  derive ids = [n]
  resource items = item(id) for id in ids as shape string
  action next
    n = n + 1
  view
    column
      button press=next testId="next"
        text "next"
      text `${n} ${items.join(",").length}`
```

with `item` answering a 1 KB string. The SVG case has no resource, and its row is `for id in ids key=id` over
`svg width=40 height=20` holding `svg x=w y=0 width=10 height=10`, with `w` a state.

| departed | list resource | control: one plain resource asked again | row with a bound `x` | control: `x=4` |
|---|---|---|---|---|
| 500 | 2,025 KB | 298 KB | 777 KB | 324 KB |
| 2,000 | 7,040 KB | 315 KB | 2,099 KB | 368 KB |
| 5,000 | 17,061 KB | 344 KB | 4,680 KB | 381 KB |

That is about 3.4 KB kept for each departed element (1 KB of it the answer) and about 0.9 KB for each departed row
with one bound attribute. Expected: flat, as the controls are. A commit that changes nothing took 0.018 ms after
5,000 departed elements (0.006 ms in its control) and 0.035 ms after 5,000 departed rows (0.007 ms in its control).

## Verification

- The two plans and their controls, built with `bun host/web-js/build.mjs` and pressed in Chrome, the heap read
  with `HeapProfiler.collectGarbage` and `Runtime.getHeapUsage`: the table.
- `resEach` over the real `rt.js` and `each.js` in Bun 1.4.2: 4.3 KB kept for each departed element with a
  one-character answer, 199.5 MB after 40,000 departed elements with 1 KB answers. The same run over a copy of
  `rt.js` whose `settle` drops the nodes marked `gone` keeps 0.7 MB at every count, so `Settle` is what holds them.

## Direction

Take a node out of `Settle` when the scope that owns it ends: `memo` can register that with `onEnd` when it has an
owner, or `settle` can skip and compact the nodes marked `gone`.

## Related

`issues/closed/20261102-web-js-views-retain-removed-dom.md` fixed the same kind of retention for the `Views` map
only. `QUEUE.md`'s list-resource entry is about kept answers. Nothing in `issues/` or `QUEUE.md` names `Settle`.
