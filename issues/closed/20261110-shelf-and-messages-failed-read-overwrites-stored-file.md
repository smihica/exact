# Shelf and Messages take any failed read of their stored file for a first launch, and the next edit saves the sample data over it

**Status:** Closed
**Resolution:** Shelf, Messages, Agents, Weatherlight and Recorder take only `ENOENT` as no file and fail the read otherwise, so no edit writes over a file they could not read.
**Systems:** apps/shelf (app.ts), apps/messages (app.ts)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Shelf's `load` catches every error from `storage.fs.readFile` except the build's `bake` and goes on with the seed library (`apps/shelf/app.ts:53-80` at `dd6f5dcd`; line 62, "first launch: no file yet"). The next edit applies to that seed and `persist` writes it with `atomicWriteFile` (`app.ts:89-110`), replacing the stored library. Messages does the same: `load` catches everything, keeps the baked conversations and sets `loaded` (`apps/messages/app.ts:20-30`), and the next command persists them (`app.ts:14-19`).

A missing file is one code among several. The host gives it `ENOENT`, and gives a failed read of a file that exists another: `denied`, `EBUSY`, `full` when 256 operations wait, `failed` for the rest (`js/src/prelude.js:793-812,854`). The guide's rule is to tell them apart: "a missing one is code `ENOENT`: answer the empty collection, and branch on `e.code`" (`docs/recipes/files-and-grants.md:65-66`), and `docs/start-here.md:260-262` rethrows every other error. `AGENTS.md` points app authors at `apps/shelf` as "a whole app to grep".

## The failure

Shelf on the Linux host, with a scratch store:

1. Add "My Own Book". `library.json` holds four books.
2. Make the file unreadable (`chmod 000`), standing in for any read that fails, and launch again.
3. Add "Second Book".

- Observed: at launch the log has `storage failed: readFile app:/data/shelf/library.json: failed filesystem: Permission denied (os error 13)`, the list shows "3 of 3 books" and `lib.ready` is true. After step 3 `library.json` holds `Second Book` and the three sample books; `My Own Book` is gone.
- Expected: the library reads as not ready, and no edit is saved until a read has succeeded or found no file.

## Verification

Ran the steps above with `bun scripts/agent.mjs linux --app shelf --storage <name>` on a `host-dev` build and read the file after each drive.

Messages was run through its module's `answer` with a store holding one conversation `ai-9` whose first `readFile` rejects with code `EIO`: after `command("read", "demo", …)` the stored file held the seven baked conversations and not `ai-9`. Not run: Messages on a host.

`apps/agents/app.ts:49-63` and `apps/weatherlight/app.ts:151-153` swallow a failed read the same way and write back afterwards (read, not run).

## Direction

In each `load`, treat only `ENOENT` as no file and rethrow the rest, as the guide's `load` does, so `library` answers `ready: false` and `change` answers `ok: false` without writing.

## Related

None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
