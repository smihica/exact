# After a capture session's runtime error an Apple camera track stays muted `interrupted` while the camera runs

**Status:** Closed
**Resolution:** After a runtime error, once the restarted session reports it is running and no interruption has begun since, the capture's tracks are told the interruption is over, so they unmute.
**Systems:** packages/media-capture/apple/VideoCapture.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `VideoCapture.listen` (`packages/media-capture/apple/VideoCapture.swift:517-557`)
turns the session's notifications into the track's mute. An interruption sets a reason and its
end clears it:

```swift
// wasInterruptedNotification
tell(reason)
// interruptionEndedNotification
tell(nil)
```

A runtime error is handled as an interruption that restarts the session (`:550-556`):

```swift
tell("interrupted")
guard let self, !self.holders.isEmpty else { return }
let session = self.session
self.queue.async { session.startRunning() }
```

`tell("interrupted")` sets each holder's `platformMute` (`VideoTracks.interrupted`, `:349-353`),
and outside the agent only `tell(nil)` clears it. AVFoundation posts `interruptionEnded` after an
interruption, not after a runtime error
([`runtimeErrorNotification`](https://developer.apple.com/documentation/avfoundation/avcapturesession/runtimeerrornotification)),
and the handler for `didStartRunningNotification` only writes a journal line (`:519-524`). So
once the session is running again the track still reads `muted: true`,
`muteReason: "interrupted"`. Giving the lease back and taking it again does not help: `lease`
settles from the same `platformMute` (`:329-330`). The track is muted until a later interruption
begins and ends, or the app stops it and asks for a new one.

## The failure

A viewfinder shows a track; media services reset (`AVError.mediaServicesWereReset`, the case
Apple's AVCam sample restarts for). By the code: the session restarts and delivers frames, and
`mediaStreamTrack(track=…)` answers `muted: true` from then on. The viewfinder stays in its muted
state (`packages/camera/apple/Camera.swift:186`) and `takePhoto` answers `InvalidStateError`
(`:204-206`; LLP 1133.008 D1). Expected: `muted` false once the session runs again.

## Verification

The package's own handlers ran on a Mac, with the notifications posted by hand; AVFoundation did
not post them, and no camera was opened. `MediaCapture.swift` and `VideoCapture.swift` were
compiled unchanged with `ExactPackage.swift`, a capture was made with its observers and no
input, and each notification was posted on its session with `NotificationCenter.default.post`:

```
leased: muted=false muteReason=nil
after wasInterrupted: muted=true muteReason=interrupted
after interruptionEnded: muted=false muteReason=nil | session running=true
after runtimeError: muted=true muteReason=interrupted | session running=true
    journal: camera: running=true, interrupted=false
after didStartRunning: muted=true muteReason=interrupted | session running=true
settled again, as a new lease does: muted=true muteReason=interrupted
```

The device check: with a viewfinder up on an iPhone, Settings › Developer › Reset Media Services,
then read the track's `muted`.

## Direction

Clear the reason when the restarted session reports it is running
(`didStartRunningNotification` while no interruption is in force), or after `startRunning()`
returns on the session's queue.

## Related

QUEUE "The camera's open ends" lists other camera gaps, not this.
