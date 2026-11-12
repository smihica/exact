# A background transfer whose `saveTo` names a directory deletes that directory and everything in it

**Status:** Closed
**Resolution:** A `saveTo` that is a folder is refused at `start` (`TypeError`) and never replaced in `place`, which replaces an existing file with `FileManager.replaceItemAt`, keeping it when that fails.
**Systems:** packages/background-transfer/apple/Transfers.swift
**Severity:** P2
**Author:** Claude (Opus 5.5), from GPT-6 Astra's whole-project review (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

When a registration has ended, `registration` moves each response body to its app file
(`packages/background-transfer/apple/Transfers.swift:109`) through `place`
(`Transfers.swift:120-133`), which clears the destination first:

```swift
try? fm.removeItem(at: dest)
do { try fm.moveItem(at: URL(fileURLWithPath: from), to: dest) } catch { … return nil }
```

`FileManager.removeItem` removes a directory with its contents. Nothing checks that `saveTo`
names a file: `start` accepts any path `context.file` resolves (`Transfers.swift:83-86`),
and `context.file` accepts any `app:/data|cache|tmp/<name>…` path
(`host/apple/modules/ExactExtensions.swift:166-173`). The runner admits `saveTo` as an
`fs.write` path (`packages/background-transfer/exact.json:55`,
`runner/src/runner/modules/admit.rs:1013-1015`), so a grant of `fs.write app:/data` covers
`app:/data/notes` whether it is a file or a folder.

Replacing a regular file has a second fault: the old file is removed before the move, and
the error is ignored, so a move that fails leaves neither the old file nor the new one.

`registration` runs on every read of `backgroundFetchRegistration`
(`Transfers.swift:46`) and on the wake (`:139`), so the deletion happens on the first
read after the transfer ends.

## To see it

Confirmed by reading the code only. In an app with `fs.write app:/data` and a nonempty
folder `app:/data/notes`, data code calls
`backgroundFetch({ id: 'x', requests: [{ url, saveTo: 'app:/data/notes' }] })`. When the
download ends, the next read of `backgroundFetchRegistration({ id: 'x' })` deletes
`notes` and its files and puts the downloaded body in its place.
`saveTo: 'app:/data/transfers'` would likewise delete the folders where unsaid bodies of
every registration are kept (`Transfers.swift:97-99`).

## What a fix is

Refuse a `saveTo` that is an existing directory, at `start` (as `NotAllowedError` or a
`TypeError`) and again in `place`, since the folder may appear later. Replace a regular
file with `FileManager.replaceItemAt(_:withItemAt:)`, which keeps the original when the
replacement fails. One decision: whether files should move on a read at all, or once,
when the carrier records the registration's end.
