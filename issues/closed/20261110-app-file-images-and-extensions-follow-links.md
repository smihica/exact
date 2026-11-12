# An `app:/` image, video or extension file follows a symbolic link that `readFile` and a share refuse

**Status:** Closed
**Resolution:** Every reader of an `app:/` file the app names (an `image`, a video, picture in picture, a tab icon, an iframe's document, Linux's saved copy and an extension's `file`) now opens it below its root with no symbolic link at any part, through one resolver per language (`picker::open` over ibex2's `AppDirectories`; Swift's `AppFileWalk`), as `readFile` does.
**Systems:** host/linux/src/picker.rs (Linux, Windows, Android's native half), host/apple/Sources/ExactKit (`Picker.swift` `AppFiles`, `RasterInput.swift`, `VideoModule.swift`, `PictureInPicture.swift`, `IOS/NavigationTabsIOS.swift`), host/apple/modules/ExactExtensions.swift
**Severity:** P2
**Author:** Claude (Opus 5.5), from Grok 4.7's whole-project review (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

Three readers of an `app:/` path open it from a pinned root handle and follow no symbolic
link at any component:

- the data module's file calls (ibex2's app directories open each component with
  `O_NOFOLLOW`, `vendor/ibex/crates/ibex2/src/stdlib/app_fs_unix.rs:43`);
- Apple's share copy, `copy_confined` (`host/apple/src/picker.rs:325-333`, `:365`), which
  the grammar promises: a shared file is "reached through no symbolic link"
  (`docs/contract-grammar.md:2705`);
- Apple's `iframe` frame files (`host/apple/webarm/FrameFiles.swift:53`, `:99`).

The others join the root and the path's components as text, refuse only `..` and empty
components, and open the result with calls that follow links:

- Linux and Windows: `picker::resolve` (`host/linux/src/picker.rs:248-267`), used for an
  `image` (`host/linux/src/image/assets.rs:94-96`, then `std::fs::read`) and for
  `ExtensionContext.file` on Android (`host/linux/src/canvas/jni_extensions.rs:127-137`,
  `host/android/canvas/src/main/java/dev/exact/canvas/Extensions.kt:184`).
- Apple: `AppFiles.url` (`host/apple/Sources/ExactKit/Picker.swift:59-65`), used for an
  `image` (`RasterInput.swift:81-86`, whose `resourceValues` check reads the link's
  target), a `video` (`VideoModule.swift:265`), picture in picture
  (`PictureInPicture.swift:145`) and a tab icon (`IOS/NavigationTabsIOS.swift:273`); and
  `ExactExtensions.file` (`host/apple/modules/ExactExtensions.swift:166-173`).

A link is easy to make: a TypeScript source with `fs.write` on `app:/data` may call
`symlink` there with any target (`app_fs_unix.rs:749-752`; `docs/reference.md:1807`, from
TypeScript runtime 6). Its
own `readFile` of that path is then refused, but an `image` of it shows the target, and an
extension handed the path after admission against `fs.read app:/data` reads the target,
which no grant covers. On macOS and Linux outside a sandbox that is any file the person can
read; on iOS, any file in the container. Android's `contentUri` is safe:
`Files.uri` canonicalises and refuses a file outside `dataDir` (`Files.kt:54-57`).

`issues/closed/20260901-image-src-symlink-escape.md` fixed the same escape for a relative
`src` under the asset root; `app:/` sources were not part of it.

## To see it

Confirmed by reading code only. With `fs.write app:/data` and `fs.read app:/data`, a
TypeScript data module calls `symlink("/Users/<you>/Pictures/private.png",
"app:/data/x.png")`; `readFile("app:/data/x.png")` is refused, and
`image src="app:/data/x.png"` on macOS or Linux shows the private picture.

## What a fix is

One resolver that opens an `app:/` path as `copy_confined` does: from the pinned root's
handle, each component with `O_NOFOLLOW` (on Windows, refusing a reparse point), and hands
back the open file rather than a path. Images, video,
picture in picture, tab icons and extension files read from that. An extension that needs a
path, not a descriptor, gets one only after the same walk has found no link.
