# A marker press released outside the web map makes the next drag never end

**Status:** Closed
**Resolution:** Marker buttons select through click without shared press state, so outside releases and overlapping touches cannot prevent a drag from ending and emitting moveend.
**Systems:** packages/maps/web/index.js
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

At `dd6f5dcd`, `listen` in `packages/maps/web/index.js:205-250` remembers a press on a marker
without capturing the pointer:

```js
const marker = ev.target.closest?.('.marker');
if (marker) { this.pressed = marker.dataset.marker; return; }
```

and its release handler looks at `this.pressed` before anything else:

```js
const up = ev => {
  if (this.pressed) {
    const marker = ev.target.closest?.('.marker');
    if (marker?.dataset.marker === this.pressed) this.select(this.pressed);
    this.pressed = null;
    return;
  }
  if (!pointers.delete(ev.pointerId)) return;
```

A mouse has no implicit capture, so a press on a marker that is released outside the map sends
the map no `pointerup`, and `pressed` stays set. The next gesture is a drag on the map: its
`pointerdown` records the pointer, and its `pointerup` takes the first branch and returns
before `pointers.delete`. The pointer stays recorded, so the map keeps panning with every later
mouse move with no button down, the `dragging` class stays, and the drag emits no `moveend`.
The next click clears it.

**The failure.** With a mouse: press a marker, move off the map, release; then drag the map and
release; then move the mouse.

Observed: the map follows the mouse after the release, and the app hears no `moveend` for the
drag. Expected: the drag ends at its release and `moveend` follows 250 ms later.

**Verification.** Map Demo in the pinned Chromium (156), after `bun host/web/build.mjs
map-demo`. The first drag presses the Ferry Building pin and releases in the header above the
map:

```
bun scripts/agent.mjs web --json --app map-demo --size 600x800 "clock settle" \
  "tap map drag 0 -300 from 540 274 mouse over 64" \
  "tap map drag -40 0 from 300 500 mouse over 64" "clock +600 real" "state extensions" \
  "tap map hover" "tap map wheel 0 -1" "clock +600 real" "state extensions"
```

The first `state extensions` has no camera for the map: the drag reported nothing. Without the
first line it reports longitude −122.402283. The second, after the mouse moved to the map's
centre with no button down, has latitude 37.788428: the map followed the mouse 182 px. Without
the first line the latitude stays 37.7946. (Its longitude is on the far side of the world:
`issues/20261110-web-map-pan-flips-longitude.md`.)

**Direction.** Clear `pressed` at the next `pointerdown`, or capture the pointer on the marker
press so its release always arrives.

**Related.** `issues/20261110-web-map-pan-flips-longitude.md`, the same file's projection.
