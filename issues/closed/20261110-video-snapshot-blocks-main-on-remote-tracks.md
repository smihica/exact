# A video or audio with a remote `src` holds the main thread until its server answers

**Status:** Closed
**Resolution:** Apple media snapshots read a cached HDR flag loaded asynchronously for the current item, so remote sources do not block the main thread.
**Systems:** host/apple/videoarm/VideoArm.swift
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

Every message the video arm sends ExactKit carries a state snapshot (`send`, `host/apple/videoarm/VideoArm.swift:308-315`), and the snapshot's `hdr` field reads the asset's tracks synchronously (`:293`, `:637-639`):

```swift
var hdrItem: Bool {
    player.currentItem?.asset.tracks(withMediaType: .video).contains { $0.hasMediaCharacteristic(.containsHDRVideo) } ?? false
}
```

Apple deprecated `tracks(withMediaType:)` in iOS 16 and macOS 13 for `loadTracks(withMediaType:)`: it returns without blocking only "when the data in the `tracks` property is already loaded" ([`tracks(withMediaType:)`](https://developer.apple.com/documentation/avfoundation/avasset/tracks(withmediatype:))), and reading an unloaded property "would block the calling thread for an unknown amount of time" ([`AVAsynchronousKeyValueLoading`](https://developer.apple.com/documentation/avfoundation/avasynchronouskeyvalueloading)).

`update` ends with `emit()` (`:466`) on the main thread, straight after `loadSource` has made the item (`:492`). Nothing has loaded yet, so the read waits on the network. It waits once for each load of a remote source: a changed `src`, `load(id)`, or a play with no item (`:334`, `:424`, `:428`, `:441`). `SharedAssets` keeps only file URLs (`:111`), so each remote load makes a new asset. Later snapshots of the same item return at once. `audio` is the same arm.

## The failure

```
video src="https://slow.example/movie.mp4" controls=true
```

Observed: from the update that sets `src`, the app neither draws nor takes input until the server has answered: one round trip for an error, two for an MP4 (a two-byte range request, then the file). Expected: the update returns at once and the item loads asynchronously, as `AVPlayer` does it; `hdr` is reported once the tracks have loaded.

## Verification

Run on macOS. The arm, compiled with the build's arguments and loaded with `dlopen`, was given a `src` through `exact_video_update` on a local server that waits before each answer. Time inside the call:

| server | first update | second update |
|---|---|---|
| 404 after 4,000 ms | 4,020 ms | 1 ms |
| an MP4, 1,500 ms an answer | 3,023 ms | 1 ms |
| either, with `hdrItem`'s body replaced by `false` | 3 ms | 1 ms |

In the Mac app (`apps/hello` with `video src=source controls=true` and a button that sets `source`), the drive `"tap count" "tap <button>" "tap count" tree` took 1.2 s against a server with no delay and 4.0 s against the 1,500 ms one. `sample` during the wait shows the main thread in `VideoView.update()`, `videoUpdate`, `VideoArm.emit`, `VideoArm.send`, `-[AVAsset tracksWithMediaType:]`, `-[AVURLAsset tracks]`.

Not run: iOS, which compiles the same source.

## Direction

Load the characteristic when the item becomes ready (`loadTracks(withMediaType:)`, then `load(.mediaCharacteristics)`), keep the answer for the item's generation, and report that. A snapshot must read nothing that can wait.

## Related

`QUEUE.md`, "Re-measure VideoArm's position read with many autoplaying videos", is about `currentTime` taking a lock; this read waits on the network. `host/apple/Sources/ExactKit/Picker.swift:194` reads tracks the same way, on a local file.
