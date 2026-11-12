# On the web, `appendFile` to a picked, downloaded or copied-in app file fails, and an empty append empties it

**Status:** Closed
**Resolution:** Web appendFile concatenates Blob-backed contents atomically, preserving their media type and existing bytes on an empty append.
**Systems:** host/web/storage-fs.js (`write`), host/web/picker-glue.js (both web targets load them)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

An app file's IndexedDB record holds its contents as an `ArrayBuffer`, or as a `Blob` when the browser already
has the file: a picked file (`host/web/picker-glue.js:179`), a document copied into app storage
(`host/web/storage-fs.js:390-395`, `copyIn`) and an `exactBodyTo` reply (`:506`). `rename` and `copyFile` keep
the Blob (`:317`, `:331`), so the copy an app keeps in `app:/data` (LLP 1069.002 D4, "keep it") is one too.
`readFile`, `stat`, `blob` and `copyFile` handle both forms. `appendFile` does not (`:205-210`):

```js
const joined = new Uint8Array(previous.contents.byteLength + copied.byteLength);
joined.set(new Uint8Array(previous.contents));
joined.set(new Uint8Array(copied), previous.contents.byteLength);
```

A Blob has no `byteLength`, so `joined` has length 0 and the second `set` writes at offset 0. With bytes to
append that throws; with none it succeeds and stores the empty buffer as the file.

Every web path reaches this `write`: the JS target's module storage (`host/web-js/ts-data.js:319`), the wasm
host's (`host/web/storage.js:53-56`) and a Rust source's storage request (`host/web/storage-request.js:112`).

## The failure

A text file holding `picked ` chosen in a file input, then, under `fs.read` and `fs.write` on `app:/tmp` and
`app:/data`, with `picked` the path its `change` delivered:

```js
await storage.fs.rename(picked, 'app:/data/notes.txt');
await storage.fs.appendFile('app:/data/notes.txt', new TextEncoder().encode('appended'));
await storage.fs.appendFile('app:/data/notes.txt', new Uint8Array());
```

Observed: the first append rejects with `RangeError: offset is out of bounds`, which names nothing the app did
(a JS-target module sees it coded `failed`, `host/web-js/ts-data.js:242-247`; a Rust source's `fs.appendFile`
request is answered `{"error":"offset is out of bounds"}`), and the file still reads `picked `. The second
resolves, and the file is then empty, `stat` size 0.

Expected: `picked appended`, unchanged by the empty append. A native host answers so, since a picked or
downloaded file there is an ordinary file.

## Verification

In the pinned headless Chromium (156), against the real `picker-glue.js`, `storage-fs.js` (`createFileSystem`
under those grants) and `storage-request.js`: a file set on an `<input type="file">` and delivered by the
picker's `change`, then renamed, gave the results above through the file system and through a Rust request
(`"op":"fs.appendFile"` with `"text":"appended"`, then `"text":""`). An entry made by `copyIn` and one made by
an `exactBodyTo` `put` did the same. A file written with `writeFile` read `written appended` after both
appends. On the Mac, `exact-data-host`'s storage given a file placed in the data root, then `fs.copyFile` and
the same two `fs.appendFile` requests, answered `null` twice and the file read `picked appended`, size 15.
Shelf's JS-target build ships this `storage-fs.js`. With the direction below applied, every append above
succeeded. Not run: a module inside a built app.

## Direction

When the previous contents are a Blob, store `new Blob([previous.contents, copied], { type: previous.contents.type })`:
it needs no read inside the transaction and preserves the file's media type.

## Related

LLP 1069.002 A1 notes that `compressImage` writes bytes "so `appendFile` and `copyFile` treat it as any file";
no LLP declares an append to a Blob entry refused. `issues/20261110-rust-storage-write-reports-no-change.md` is
another storage path. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` mentions `appendFile`.
