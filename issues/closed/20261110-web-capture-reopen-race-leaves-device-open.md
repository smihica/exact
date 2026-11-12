# On the web, a lease retaken while its track is reopening leaves a microphone or camera capturing

**Status:** Closed
**Resolution:** Each `activate` takes its own lease number and adopts the browser track only while that lease is current; `deactivate` ends it. `packages/media-capture/web/lease.test.mjs` retakes a lease during a slow open and checks no browser track stays live.
**Systems:** packages/media-capture/web/index.js, packages/media-capture/web/video.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

An idle track opens its device again when a consumer takes the lease. At `dd6f5dcd`,
`activate` in `packages/media-capture/web/index.js:81-99` (the microphone) and
`packages/media-capture/web/video.js:111-128` (the camera) awaits `getUserMedia` and then adopts
the browser's track if the lease is still held:

```js
const stream = await media.getUserMedia({ audio: t.constraints });
if (!t.active || t.readyState !== 'live') { for (const s of stream.getTracks()) s.stop(); return; }
adopt(t, stream.getAudioTracks()[0]);
```

`t.active` does not say which `activate` set it. If the lease is given back and taken again
while the first open is pending, a second `getUserMedia` starts, both find `t.active` true, and
the second `adopt` replaces `t.source` without stopping the first. `stopSource` later stops only
`t.source`, so the first browser track captures until the page closes: the camera's light or
the browser's recording indicator stays on, and the app has no handle left to stop it.

**The failure.** A track that is idle (it was used, or no consumer joined within a second of
`getUserMedia`); then, within the device's opening time:

1. a consumer subscribes with `start` (a recorder starts, a viewfinder mounts, or the tab is
   shown again: `video.js:321`),
2. it leaves (the recorder stops, the viewfinder unmounts, the tab is hidden: `video.js:320`),
3. a consumer subscribes again.

Then every consumer leaves. Observed: the track reads `muted`, `idle`, and one browser track is
still `live`. Expected: none.

**Verification.** In a page of the pinned Chromium (156) started with
`--use-fake-device-for-media-stream --use-fake-ui-for-media-stream`: the implementation
imported as written, the track table of `host/web-js/modules.js`, and
`navigator.mediaDevices.getUserMedia` wrapped to record each browser track and to take 300 ms
longer (the fake devices open in 1 ms; a camera does not). After the last consumer left:

```
audio: start, leave, start, 100 ms apart: the track reads muted (idle); browser tracks still live: 1 ("Fake Default Audio Input")
video: the same: 1 ("fake_device_0")
video with a viewfinder, the page hidden, shown, hidden, shown, 100 ms apart (`foreground`): 1
either kind, the steps 400 ms apart, each open finished before the next: 0
```

Not run: a real camera.

**Direction.** Number each `activate` on the track and stop the opened browser track when a
later one has begun, or keep the pending open on the track and have the second `activate` await
it instead of opening another.

**Related.** QUEUE "The camera's open ends" lists other camera gaps, not this.
