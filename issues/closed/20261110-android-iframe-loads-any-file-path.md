# Android's `iframe` loads a `src` outside the asset root as a file, with JavaScript on, and its `video` plays only a file path

**Status:** Closed
**Resolution:** Android resolves an `iframe`'s and a `video`'s `src` as Apple does (`Sources.kt`): a scheme-less one only under the asset root, `//` as `https:` for a frame, a video's `http(s)` and `app:/` as a URL and a file, and anything else refused; a frame reads files only for a local page, and its navigations stay in the frame.
**Systems:** host/android/canvas (`Natives.kt`, `VideoPlayer`)
**Severity:** P2
**Author:** Claude (Opus 5.5), from Grok 4.7's whole-project review (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

Apple keeps a scheme-less `iframe` `src` under the asset root: `WebArm.localBytes`
(`host/apple/webarm/FrameFiles.swift:199-219`) strips leading slashes, resolves the path
under `assetBase`, and refuses one that leaves it, which `WebArm.swift:825-829` turns into
an error document. A protocol-relative `src` (`//host/page`) becomes `https:`
(`browserSource`, `host/apple/Sources/ExactKit/WebModule.swift:352`, `:502-506`).

Android does none of this. For an `iframe` (element kind 1) `apply` loads
(`host/android/canvas/src/main/java/dev/exact/canvas/Natives.kt:695-699`):

- an `http:` or `https:` `src` as given;
- anything else as `Uri.fromFile(file(src))`, where `file` (`:664`) is `File(src)` when
  `src` starts with `/`, else `File(assets, src)`, with no normalisation and no check that
  the result stays under `assets` (`filesDir/app`, `ExactActivity.kt:305`).

So `src="/sdcard/Download/page.html"` loads that file when the app can read it,
`src="../../shared_prefs/x.xml"` loads the app's private preferences
(`filesDir/app/../../shared_prefs`), and `src="//example.com/deck"` becomes the file
`/example.com/deck`, which does not exist. The `WebView` has `javaScriptEnabled` and
`allowFileAccess` (`:516-517`) and no `WebViewClient`: a loaded HTML file's scripts run.

`src` may be computed from data (`docs/contract-for-agents.md:1908-1910` builds one with a
template string), so a value the app did not write can choose the file.

Android's WebView defaults keep `allowFileAccessFromFileURLs` and
`allowUniversalAccessFromFileURLs` off, so such a page cannot read further files by
script. What it gets is display, script execution in a `file:` origin, and network
requests.

That the same `WebView` ignores `sandbox` is `QUEUE.md:1733` (LLP 1020.002 §10) and is not
repeated here.

### `video`

A `video` (element kind 0) goes through the same `file` with no scheme test at all:
`p.source(file(src))` (`Natives.kt:686`), and the player an app supplies
(`ExactActivity.video`, `ExactActivity.kt:30`) can take nothing else, since
`VideoPlayer.source` takes a `File` (`Natives.kt:64`). So on Android:

- `src="https://cdn.example/a.mp4"` becomes the file `filesDir/app/https:/cdn.example/a.mp4`
  and does not play; on Apple it plays (`VideoModule.swift:265` keeps an `http(s)` URL
  through `NodeView.resolveSource`, `NodeViewIOS.swift:511-516`). `apps/video-player`
  offers such a source (`app.contract:116`).
- `src="app:/data/episode.mp4"`, a download the data module kept, becomes
  `filesDir/app/app:/data/episode.mp4` and does not play; Apple resolves it with
  `AppFiles.url` (`VideoModule.swift:265`).
- An absolute path or one with `..` plays whatever media file the app can read, as the
  `iframe` loads it. For video this shows the file and runs no script.

No app in the repository supplies an Android `VideoPlayer`, so none fails today.

## To see it

Confirmed by reading code only; no Android device or emulator was run. An app with
`iframe src=path` where `path` is `"../../shared_prefs/<name>.xml"` shows that file on
Android; on iOS and macOS it shows "… is not a file under …". The `video` half was also
confirmed by reading code only: with a `VideoPlayer` supplied, `apps/video-player`'s
"Sample clip (web)" would not play on Android.

## What a fix is

Resolve a scheme-less `src` as `localBytes` does: drop leading slashes, take the path
before `?` and `#`, resolve it under `assets` with `canonicalFile`, and refuse (an error
page, logged) one that leaves the root. Prefix a protocol-relative `src` with `https:`.
Turn off `allowFileAccess` when the source is remote, and set a `WebViewClient` so a
guest's navigations stay in the frame rather than leaving to another app. The sandbox
fix in `QUEUE.md:1733` belongs with this change, since both decide what a guest may do.

`video` should share the resolver, as Apple's does: an `http(s)` source as a URL, `app:/`
through the app's roots (`Native.appFile`), anything else under `assets` or refused. That
needs `VideoPlayer` to take a URL (`android.net.Uri`, which Media3's `MediaItem.fromUri`
and `MediaPlayer.setDataSource(Context, Uri)` both accept) in place of `File`. A refused
source should fail the element as Apple's `sourceError` does (`VideoModule.swift:270`).
