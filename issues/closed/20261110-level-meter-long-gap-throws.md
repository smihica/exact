# A level meter read after a long gap throws on the web, and stops `<audio-waveform>` for good

**Status:** Closed
**Resolution:** A silent meter counts a gap's time arithmetically and makes only the levels its history keeps, levels are appended without a spread, and the waveform asks for its next frame before it draws.
**Systems:** packages/recording/web/index.js, packages/recording/android/Recording.kt
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

A meter answers "every interval since the last answer". At `dd6f5dcd`,
`packages/recording/web/index.js:192-209` builds one entry per elapsed interval and appends them
with a spread:

```js
if (!m.running || m.muted) fresh = new Array(steps).fill(0);
...
m.levels.push(...fresh);
if (m.levels.length > m.history) m.levels.splice(0, m.levels.length - m.history);
```

A spread passes each entry as an argument, and an engine refuses a call with too many. For a
track that is ended, idle or muted, `steps` is the whole gap at 50 ms a step, so the read throws
`RangeError` once the meter has gone unread for 104 minutes in Chromium, 7 hours in Firefox and
9 hours in WebKit.

`<audio-waveform>` reads its meter only in an animation frame (`:311`), and a hidden tab runs
none. Its frame is

```js
const frame = () => { if (!this.gone) { this.draw(); this.raf = requestAnimationFrame(frame); } };
```

so when `draw` throws, the next frame is never requested and the view stays frozen until it is
unmounted, whatever track it is given afterwards.

**The failure.** A page shows `<audio-waveform track=…>` for a track that ended (`stopTrack`,
or its device went away), or that was used and went idle before the view mounted; such a track
draws silence, by design. The person leaves the tab for two hours and returns. Observed in
Chromium: one `RangeError: Maximum call stack size exceeded` and a waveform that never draws
again. Expected: silence, then bars when it is given a live track. An `audioLevel` reader of
such a track whose poll was suspended as long rejects once the same way. A waveform that is a
live track's consumer keeps it live and is not affected.

**Verification.** In a page of the pinned Chromium (156) with its fake microphone: the view
class on a canvas over a track of `packages/media-capture/web`, the page's own
`requestAnimationFrame`, and the session clock (`now`) moved 104 minutes between two frames.

```
track stopped before the view mounted:        1 of the next 10 frames drew; page error RangeError
track used and idle before the view mounted:  the same
the view is the live track's first consumer:  10 of 10 drew
```

After 103 minutes all ten draw. Firefox 157 throws at 420 minutes and not at 410 (`too many
function arguments`), WebKit 27.2 at 540 and not at 530. Not run: a tab hidden for two hours.

Android has the same shape with a different cost, by reading only (no JDK on this Mac):
`Recording.kt:472` adds one boxed zero per step and `:480` trims with
`while (levels.size > history) levels.removeAt(0)`, each removal shifting the whole list. A
hidden app's track is muted `background` (`MediaCapture.kt:301`; not with
`device.microphone-background`), so a process that slept eight hours with a waveform on screen
runs 576,000 removals over a list that long, on the main thread, in its first frame back.

**Direction.** Only the last `history` levels are kept, so cap the entries materialised at
`history` and add the skipped intervals to `totalSamplesDuration` arithmetically; trim Android's
list once (`subList(0, n).clear()`), as Apple's `removeFirst(n)` does.

**Related.** `issues/20261110-web-serial-large-read-lost.md` is the same spread in
`packages/serial/web/index.js`.
