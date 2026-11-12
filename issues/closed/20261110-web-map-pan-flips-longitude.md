# Panning the web map moves its centre 180° of longitude at every step

**Status:** Closed
**Resolution:** Web map pans preserve the projected longitude and wrap correctly across the date line in either direction.
**Systems:** packages/maps/web/index.js
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

`exact-map` on the web keeps its camera as latitude and longitude and converts through world
pixels. At `dd6f5dcd` the two conversions in `packages/maps/web/index.js:43-50` are not
inverses:

```js
const project = (lat, lon, z) => { ...; return [(lon + 180) / 360 * s, ...]; };
const unproject = (x, y, z) => { ...; return [..., ((x / s * 360 + 540) % 360) - 180]; };
```

`x / s * 360` is already `lon + 180`, so `unproject` returns `((lon + 720) % 360) - 180`:
`lon + 180` for a western longitude and `lon - 180` for an eastern one. The wrap needs `+ 360`,
not `+ 540`.

`panBy` (`:138-142`) unprojects once, so every pan step puts the centre on the opposite side of
the world: each `pointermove` of a drag (`:231-233`) and each arrow key (`:256-259`). A drag
flickers between the two places and ends on the wrong one when its moves are odd in number;
`moveend` then reports that longitude to the app. `zoomBy` (`:129-136`) unprojects twice and
the errors cancel, so the zoom buttons, `+`, `-` and the wheel work.

**The failure.** Map Demo (`apps/map-demo`, the one app that uses `@exact/maps`) on the web,
centred at longitude −122.404, zoom 15. Click the map and press the right arrow once.

Observed: `moveend` reports longitude 57.599433 and the map shows none of its four pins.
Expected: about −122.4006 (80 px east).

**Verification.** In the pinned Chromium (156), after `bun host/web/build.mjs map-demo`:

```
bun scripts/agent.mjs web --json --app map-demo --size 600x800 "clock settle" "tap map" \
  "type map key ArrowRight" "clock +600 real" "state extensions"
  camera: {"latitude":37.7946,"longitude":57.599433,"zoom":15}
```

A mouse drag of 9 px in three moves (`"tap map drag -9 0 mouse over 48"`) ends at longitude
57.596386. The same drag in four moves (`over 64`) ends at −122.403614, which is right.

No drive shows it. `packages/maps/test/map-demo.mjs` taps a marker, presses the buttons and
turns the wheel; it never drags or presses an arrow key. The driver's default drag (`over 250`)
makes sixteen moves, an even number, so a drive that dragged without `over` would end in the
right place too. Dragging and the arrow keys are steps 4 and 5 of the by-hand list in
`packages/maps/test/checklist.md`.

**Direction.** `((x / s * 360 + 360) % 360) - 180`, written so a negative `x` wraps too
(`((v % 360) + 360) % 360`), and a drive that presses an arrow key and drags in an odd number
of moves.

**Related.** `issues/20261110-web-map-press-released-outside-sticks.md` is the same file's
pointer bookkeeping. QUEUE's LLP 1133.021 entries list unbuilt stages, not this.
