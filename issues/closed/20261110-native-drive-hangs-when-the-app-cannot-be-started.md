# A native drive never ends when its app's binary exists and cannot be executed

**Status:** Closed
**Resolution:** Native spawn errors finish cleanup, return the launch failure, and release the store lock.
**Systems:** scripts/agent-stdio.mjs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`openStdio` starts the app with `spawn` and waits for its end on the child's `exit` event alone
(`scripts/agent-stdio.mjs:99-100`):

```js
const exited = new Promise((r) => child.on('exit', (code, signal) => { r(code ?? signal); fail(…); }));
const close = async () => { …; await waitAtMost(exited, 2000); if (child.exitCode === null && child.signalCode === null) { try { child.kill('SIGKILL'); } catch {} } await exited; };
```

A child that could not be started emits `error` and `close`, and no `exit`. Node documents that
"the 'exit' event may or may not fire after an error has occurred"; under the pinned Bun 1.4.2 it
does not for `EACCES` or `ENOENT`. The `error` handler at `:98` fails the pending ready line, the
`catch` at `:161-163` calls `close()`, and `close()` waits on `exited` for ever. The error that
names the cause is never thrown, nothing is printed, and the driver stays alive holding the
store's lock (`scripts/agent-launch.mjs:216-232`). The web carrier's `exited` resolves on `error`
too (`scripts/agent.mjs:172-175`).

The reach is a binary that exists and cannot be executed. A path with no file is refused before
the spawn (`:42`), the Mac carrier checks the binary's embedded identity first (`:43`), and an
Android drive spawns `adb`, which earlier synchronous calls have already run. So it takes
`EXACT_LINUX_BIN` or `EXACT_WINDOWS_BIN` naming a file without the execute bit, or a directory.

## The failure

```
printf '#!/bin/sh\necho hi\n' > /tmp/not-executable        # mode 0644
EXACT_LINUX_BIN=/tmp/not-executable bun scripts/agent.mjs linux --app hello tree
```

Observed: one line, `linux build freshness unchecked: EXACT_LINUX_BIN names the binary`, then
nothing. The process was still running 45 s later, past the carrier's 20 s ready deadline
(`:111`). A second drive of the same store meanwhile is refused: `--storage default: another drive
of this store is open on this machine …`.

Expected: `launch failed: EACCES: permission denied, posix_spawn '…'` and exit 1 at once.

## Verification

Ran the command above at `dd6f5dcd` under a 45 s bound, and with `EXACT_LINUX_BIN` naming a
directory under a 30 s bound: still running both times, then killed. A child spawned on either
path under this Bun emits `error` (`EACCES`), then `close`, and no `exit`. A path with no file
exits 1 in 0.1 s (`run cargo build … first`). An executable file of the wrong format exits 1 too:
Bun's `spawn` throws `ENOEXEC` synchronously. `EXACT_MAC_BIN` naming the non-executable file exits
1 (`embedded app identity is missing`). The Windows carrier was not run; it spawns its binary on
the same line (`:91`).

## Direction

Resolve `exited` on `error` as well as `exit`, so `close()` returns and the launch error is
thrown. With that change the command above printed the expected line and exited 1 in 0.1 s.

## Related

None in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
`issues/20261110-process-close-waits-out-grace-after-group-ended.md` is the Apple host's process
supervisor, not the driver.
