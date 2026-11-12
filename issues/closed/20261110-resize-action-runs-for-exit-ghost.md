# A removed element that plays an exit animation runs its `resize` action once more, on the JS target

**Status:** Closed
**Resolution:** On the JS target, resize actions stop when their scope ends, including while an exit ghost remains connected.
**Systems:** host/web-js/observers.js, host/web/presence-glue.js
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `onResize` (`host/web-js/observers.js:11-28`) runs the `resize` action whenever the observer reports
and the element is connected (`observers.js:22`). It does not test whether the scope that built the element has
ended. An element with `-exact-exit-animation` stays connected after its arm or row ends, as a ghost, and the ghost
takes a new box: `host/web/presence-glue.js:137-141, 336-337` positions it at `offsetWidth` by `offsetHeight`, which
are whole pixels. When the element's size was not whole pixels its content box changes, the observer reports, and
the action runs for an element the plan no longer has, with the ghost's rounded size.

The two neighbours guard this. `onIntersect` in the same file tests `gone` (`observers.js:37, 42`), and the wasm
host delivers a size only for a live view (`host/web/glue.js:560`: `views.get(id) === el &&
!retiredViews.has(el)`).

## The failure

```
keyframes leave
  to opacity=0

component App
  state shown = true
  state calls = 0
  state seen = "none"
  action hide
    shown = false
  action fit(w: number, h: number)
    calls = calls + 1
    seen = `${w}x${h}`
  view
    column
      button press=hide testId="hide"
        text "hide"
      text `${calls} ${seen} ${shown}` testId="seen"
      if shown
        box testId="panel" width=200.4 height=80.4 resize=fit -exact-exit-animation="leave 600ms linear both"
```

Before the press the text reads `1 200.390625x80.390625 true`. 100 ms after it, while the ghost fades, it reads
`2 200x80 false`: `fit` ran after `shown` became false. Expected: `1 200.390625x80.390625 false`.

## Verification

The plan above, built with `bun host/web-js/build.mjs` and opened in Chrome on the wall clock, printed the two
lines. The same plan with `width=200 height=80` reads `1 200x80 false` after the press: the ghost's box is the
element's, and nothing more is reported. The wasm host's guard was read, not run. A percentage width or a text's
own width, which are usually not whole pixels, were not run.

## Direction

Give `onResize` the guard `onIntersect` has: set a flag in its `onEnd` and test it before calling `f`.

## Related

`QUEUE.md`, "scrollend gaps on the hosts that have it", asks to verify the same lifetime rule for `scroll` and
`scrollend`; it does not name `resize`. Nothing in `issues/` or `docs/agent-pitfalls.md`.
