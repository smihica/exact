# An `iframe` whose `src` is empty, `#` or `?` crashes the app on macOS and iOS

**Status:** Closed
**Resolution:** Empty iframe sources load a blank document; query-only and fragment-only sources report missing files without trapping.
**Systems:** host/apple/webarm/FrameFiles.swift (`WebArm.localBytes`), host/apple/webarm/WebArm.swift (`serve`), host/apple/Sources/ExactKit/WebModule.swift
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`WebArm.localBytes` takes the path out of a scheme-less `src` by indexing what `split` returns (`host/apple/webarm/FrameFiles.swift:211`):

```swift
let path = source.split(separator: "?", maxSplits: 1)[0].split(separator: "#", maxSplits: 1)[0]
```

`split` omits empty subsequences by default, so for `""` and `"?"` the first `split` is empty, for `"#"` the second is, and `[0]` traps. `localFile` (`WebArm.swift:1009-1010`) does the same split with `omittingEmptySubsequences: false` and does not trap. `serve` reaches `localBytes` through `localDocument` (`WebArm.swift:821`) for any `src` that is not a typed local file, five lines before the test that would let an empty `src` through as no document (`:826`, `let src, !src.isEmpty`).

Nothing upstream stops the value. The compiler accepts `iframe src=""` and a bound `src`, the runner sends the empty string as the prop, and `WebViews.update` hands `owner.props["src"]` to the arm with `present` set whatever its length (`host/apple/Sources/ExactKit/WebModule.swift:352`, `:378`, `:420`, `:508-516`).

## The failure

```
component Hello
  state url = "about:blank"
  action empty
    url = ""
  view
    column
      button "empty" press=empty testId="empty"
      iframe src=url width=200 height=100 testId="frame"
```

Observed: pressing `empty` ends the process with `EXC_BREAKPOINT` on the next turn of the main queue. `iframe src=""` ends it at launch.

Expected: an empty `src` shows `about:blank`, as HTML's iframe does (HTML, "shared attribute processing steps for iframe and frame elements": the URL is `about:blank` unless `src` is present and "its value is not the empty string"). `#` and `?` name no file, said in the log as a missing file is.

## Verification

Run in the Mac app: `bun host/apple/build.mjs hello-apple` with `apps/hello/app.contract` as above, plus buttons that set `"#"` and `"?"` and one that counts presses, then `bun scripts/agent.mjs macos --app hello …`:

```
"tap count" "tap empty" …     op 2/6 `tap empty`: the app exited (killed by SIGTRAP)
"tap count" "tap hash" …      op 2/6 `tap hash`: the app exited (killed by SIGTRAP)
"tap count" "tap query" …     op 2/5 `tap query`: the app exited (killed by SIGTRAP)
iframe src="", then tree      op 1/3 `tree`: the app exited (killed by SIGTRAP)
```

With `url` left at `about:blank` the drive runs to its end. The crash reports read:

```
EXC_BREAKPOINT (SIGTRAP), com.apple.main-thread
 0 WebArm.localBytes(_:)                        (libexact_web.dylib)
 1 WebArm.localDocument(_:)
 2 WebArm.serve(error:)
 3 closure #1 in WebArm.scheduleServe()
```

Not run: iOS, which compiles the same source.

## Direction

In `serve`, treat an empty `src` as no `src` (the blank document it already serves for `nil`), and split in `localBytes` as `localFile` does, with `omittingEmptySubsequences: false`.

## Related

No issue, QUEUE entry or pitfall names an empty `src`. `host/apple/Sources/ExactKit/VideoModule.swift:271` already answers an empty `src` on `video` ("Empty src attribute").
