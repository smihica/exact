# With `SFSpeechRecognizer`, a silent stretch of a file ends its transcription `no-speech`, and the reader thread never exits

**Status:** Closed
**Resolution:** A silent request of a file is now a seam, so the file is read to its end, and the reader thread stops once the recognition stops, aborts or ends instead of waiting for a request that never comes.
**Systems:** packages/speech-recognition/apple/SpeechRecognition.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`RecognizerEngine` (the engine before iOS and macOS 26, or with `apple.engine`) transcribes a
file in requests of 50 seconds of audio each
(`packages/speech-recognition/apple/SpeechRecognition.swift:729-752` at `dd6f5dcd`). The reader
thread ends a request's audio and waits for the main thread to begin the next one (`:745-746`):

```swift
request?.endAudio()
while !aborted { lock.lock(); let same = self.request === request; lock.unlock(); if !same { break }; usleep(20000) }
```

The next request begins in `segmentEnded` (`:721-727`), which treats a file as going on whatever
`continuous` says. The error branch of `answered` does not (`:714-716`):

```swift
// 1110: no speech detected; 301: the request was canceled.
if code == 1110, r.continuous, !stopping { return segmentEnded() }
r.engineEnded(code == 1110 ? "no-speech" : ...)
```

`continuous` is false unless the app writes it, so a request that holds no speech ends the whole
recognition with `error: "no-speech"`. The rest of the file is never read; when the silent
request is the last, a complete transcript ends as an error.

`Recognition.finish` then drops the engine without aborting it (`engine = nil`, `:437`). The
reader thread holds the engine, `aborted` is false and `request` never changes, so the loop above
polls every 20 ms for the life of the process. An ordinary `stopSpeechRecognition` before a
file's last request leaves the same thread: the reader reads at disk speed, so it is already
waiting at the next boundary.

## The failure

On iOS 18, `startSpeechRecognition(path="app:/data/memo.m4a")` for a three-minute memo whose
third 50 seconds are a pause. Expected: the whole file's transcript (LLP 1133.012 D9: `path=`,
with any `continuous`, runs "to the file's end"). By the code: finals for the first 100 seconds,
then `state: "ended"`, `error: "no-speech"`, and one thread left polling.

## Verification

The engine's own code ran on a Mac; `SFSpeechRecognizer` did not (it needs the Speech Recognition
permission, which this review did not ask for). Lines 663-775 were compiled unchanged against
stand-ins for the Speech types and `Recognition`, over a 180 second file; the stand-in answers a
request when its audio ends, with a final result or with error 1110.

| Case | Result |
|---|---|
| no silent request | four finals, `ended`, engine released |
| third request silent | two finals, `ended (no-speech)`, engine still held by its reader thread two seconds later, released by `abort()` |
| third request silent, `continuous` true | a seam, the fourth final, `ended`, engine released |
| `stop()` during the first request | one final, `ended`, engine still held by its reader thread |

The run does not show that Speech reports 1110 for a request with no speech; the code's comment
says it does. The device check: the memo above on iOS 18, or with
`apple.engine="sfSpeechRecognizer"`.

## Direction

Let a file's silent request count as a seam (`r.continuous || file != nil`), and have `finish`
abort the engine, or have the wait loop also stop when the recognition has ended.

## Related

QUEUE "Speech, what is owed" lists the device checks still owed; it does not name this.
