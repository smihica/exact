# A sound or player that ends during a call takes the call's audio session back to ambient and deactivates it

**Status:** Closed
**Resolution:** Sound and player releases preserve a held call's audio category, mode, options and system-owned activation.
**Systems:** host/apple/Sources/ExactKit/AudioSession.swift, packages/calls
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

LLP 1133.033 D10 rules: "No other path activates or restores while a call claim is held", and "under a call claim each keeps the call's category, mode and options and leaves activation to the system".

`AudioSession` (commit `dd6f5dcd`) keeps the rule in `reconcile` (line 351), `composeSpeechThrowing` (line 202), `restoreAfterCaptures` (line 271) and `claimCapture` (line 243), each of which checks `inCall`. Three paths do not:

- `release(_:)` (line 124): when the last holder goes and no capture is held, it calls `setCategory(.ambient)` and then `giveUpIfIdle()`.
- `playerWent(_:)` (line 142): calls `giveUpIfIdle()`.
- `giveUpIfIdle()` (line 359): calls `setActive(false, options: .notifyOthersOnDeactivation)`.

So in an app whose `audio_session` is `playback`, while `@exact/calls` holds a call claim (`packages/calls/apple/Calls.swift:260`) and no capture claim is held, a video that is muted or removed (`VideoModule.swift:158`, `:201`) or a sound arm that shuts down (`SoundModule.swift:128`) replaces the call's `.playAndRecord` and `.voiceChat` with `.ambient`, a playback-only category. With no player left it also deactivates the session the system activated. A capture claimed after that joins without setting a category (`claimCapture` skips it under `inCall`), so it runs in the ambient session until the call ends.

An `ambient` app is unaffected (both paths return at `category != .ambient`), and so is a call during which a capture claim is held throughout.

## The failure

```swift
AudioSession.category = .playback
try AudioSession.hold(ObjectIdentifier(sound))
AudioSession.playerCame(ObjectIdentifier(player))
let call = AudioSession.claimCall(video: false) { _ in }
AudioSession.callActivated(true)
AudioSession.release(ObjectIdentifier(sound))
AudioSession.playerWent(ObjectIdentifier(player))
let capture = AudioSession.claimCapture(mode: .default, options: [], background: false) { _ in }
```

Observed: the session's category, mode and activation after each step, and the calls that step made on it:

```
in call:            playAndRecord voiceChat  active: true   ["setCategory(playAndRecord, voiceChat)"]
after release:      ambient default          active: true   ["setCategory(ambient, default)"]
after playerWent:   ambient default          active: false  ["setActive(false)"]
after claimCapture: ambient default          active: false  []
```

Expected: while `inCall` is true none of these reaches the session; it stays `playAndRecord`, `voiceChat` and active until `callActivated(false)` and `restoreAfterCall`. With a capture claimed before the release, or in an `ambient` app, the same calls log nothing and the session stays the call's.

## Verification

`AudioSession.swift` was compiled on a Mac with `swiftc` after removing its `#if os(iOS) || os(tvOS)` guard, its `AVFoundation` and `UIKit` imports and its `package` modifiers, and replacing `#available(iOS 27, tvOS 27, *)` with `true`. `AVAudioSession` was a stand-in class that records `setCategory` and `setActive` and holds the category, mode and active flag; `UIApplication.didBecomeActiveNotification` was a stand-in name. Not run on a device or simulator, so what a live CallKit call does when its session is set to ambient or deactivated is not observed.

## Direction

Return early from `release`'s restore and from `giveUpIfIdle` while `inCall`; `restoreAfterCall` already sets the app's category when the last call ends. `AudioSessionIOSTests` has no test with a call claim; one for this sequence would hold the fix.

## Related

QUEUE.md, "Foreground services and calls: what stages 0 to 4 left", lists the call claim as built and names what D10 still lacks (the hold gate, pausing players, the interruption-end activation); this path is not among them. No issue covers it.
