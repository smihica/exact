# `message`, `beforeunload` and MIDI handlers are never removed when their element's scope ends, on the JS target

**Status:** Closed
**Resolution:** Window and MIDI registrations end with their JS scopes; wasm beforeunload listeners end on removal, reload, or editor replacement.
**Systems:** host/web-js/rt.js, host/web-js/midi.js, host/web/glue.js
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Most handlers are listeners on their own element and go with it. At `dd6f5dcd` three are registered somewhere that
outlives the element, and nothing takes them back when the row or arm that built the element ends:

- `onMessage` (`host/web-js/rt.js:911`): `addEventListener("message", …)` on the window, for an `iframe`'s `message`.
- `onUnload` (`rt.js:905`): `addEventListener("beforeunload", …)` on the window.
- `onMIDI` (`rt.js:903`): a pair added to the `Sounds.midi` set, which `host/web-js/midi.js:233-244` walks whole at
  every message, skipping the disconnected and deleting none.

Each closure holds its element and its handler, and through the handler the row's item and state. So every element
that ever had one of these handlers stays reachable for the life of the page, and every later window `message`,
`beforeunload` or MIDI message runs every closure ever registered. The handler itself is still guarded
(`ev.source === e.contentWindow`, `e.isConnected`), so a removed element's action does not run; the cost is the
growth.

The wasm target's glue has the second of these too: `host/web/glue.js:572` (at `fe4632ed`) adds a window `beforeunload`
listener for each element with that handler, and nothing removes it (read, not run).

`on` already has the pattern for a listener that must not outlive its scope: its `l` helper removes `close` and
`scrollend` listeners with `onEnd` (`rt.js:875`).

## The failure

```
component App
  state n = 0
  state heard = 0
  derive ids = [n]
  action next
    n = n + 1
  action got(d: string)
    heard = heard + 1
  action leaving
    heard = heard + 1
  view
    column
      button press=next testId="next"
        text "next"
      for id in ids key=id
        box beforeunload=leaving testId="row"
          iframe "/frame.html" sandbox="allow-scripts" message=got width=60 height=20
```

Each press replaces the one row. After 200 presses the document holds one row and one iframe, and the window holds
201 `message` listeners and 201 `beforeunload` listeners. Expected: one of each.

## Verification

- The plan above, built with `bun host/web-js/build.mjs` and opened in Chrome with the window's `addEventListener`
  and `removeEventListener` counted from before the app loads: `{"rows":1,"iframes":1,"message":1,"beforeunload":1}`
  before, `{"rows":1,"iframes":1,"message":201,"beforeunload":201}` after 200 presses.
- `onMessage`, `onUnload` and `onMIDI` called inside 1,000 scopes that are then ended, over the real `rt.js` in Bun:
  1,000 `message` listeners, 1,000 `beforeunload` listeners and 1,000 pairs in `Sounds.midi` are left.

MIDI was not run in a browser.

## Direction

Register the removal with `onEnd` in each: `removeEventListener` for the two window listeners, `Sounds.midi.delete`
for the pair.

## Related

`issues/closed/20261102-web-selection-watch-retains-removed-text.md` and
`issues/closed/20261102-web-js-views-retain-removed-dom.md` fixed the same kind of retention for selection watches
and motion views; these three registrations are not covered by either. Nothing in `QUEUE.md` or
`docs/agent-pitfalls.md`.
