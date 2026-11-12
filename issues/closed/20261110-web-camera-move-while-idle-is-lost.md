# On the web, `applyConstraints` to another camera is lost when the track is idle

**Status:** Closed
**Resolution:** A move opens the camera moved to even while the track is idle, so the answer names it and the next lease reopens it; an idle track closes it again at once.
**Systems:** packages/media-capture/web/video.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

`applyConstraints` with another `deviceId` or `facingMode` moves a video track to that camera
under the same id (LLP 1133.008 D1), and D6 lists that move among the capture starts "since the
next lease would capture through it". At `dd6f5dcd`,
`packages/media-capture/web/video.js:228-248` does the move only while the browser's track is
open:

```js
t.constraints = next;
if (agent) { ... }
else if (t.source) {
  stopSource(t);
  t.settings = { ...t.settings, deviceId: null };
  try { adopt(t, (await media.getUserMedia({ video: next })).getVideoTracks()[0]); } ...
}
say(`${t.id}: moved to ${t.label}`);
```

An idle track has no `t.source`: its last consumer left, or none joined within a second of
`getUserMedia` (`:213`). For it the new constraints are stored but `t.settings.deviceId` keeps
the old camera's id, and the next lease reopens "by the device the track last used" (`reopen`,
`:103-108`), which deletes `facingMode` and asks for exactly that id:

```js
if (t.settings.deviceId) { delete c.facingMode; c.deviceId = { exact: t.settings.deviceId }; }
```

So the move never happens, the call answers the old settings without an error, and the journal
says "moved".

**The failure.**

```
send s = getUserMedia(video=true)               // opens the back camera
// more than a second later, before a viewfinder shows the track:
send m = applyConstraints(track=…, facingMode="user")
// then the viewfinder mounts
```

Observed: `m` answers the back camera's settings, and the viewfinder opens the back camera.
Expected: the front camera, as the same call gives while the viewfinder is showing.

**Verification.** In a page of the pinned Chromium (156) started with
`--use-fake-device-for-media-stream=device-count=2 --use-fake-ui-for-media-stream`: the
implementation imported as written, the track table of `host/web-js/modules.js`, and
`getUserMedia` wrapped to record what it is asked. A track opened on `fake_device_0`:

```
idle, applyConstraints(deviceId=<fake_device_1>): answers label "fake_device_0"; journal "track-2: moved to fake_device_0";
  when the viewfinder mounts the browser is asked {"video":{"deviceId":{"exact":"<fake_device_0>"}}}
idle, applyConstraints(facingMode="user"): the same ask
with a viewfinder showing, applyConstraints(deviceId=<fake_device_1>): the browser is asked {"video":{"deviceId":"<fake_device_1>"}} and the track moves
```

(Chromium's fake cameras open the first one for a bare `deviceId`, so the wrapper passed a bare
id on as `exact` for the last row to move.) Not run: a phone's cameras, or the Apple and
Android implementations.

**Direction.** Clear `t.settings.deviceId` (and set the asked `facingMode`) whenever a move is
accepted, so `reopen` opens by the new constraints.

**Related.** QUEUE "The camera's open ends" does not list it.
