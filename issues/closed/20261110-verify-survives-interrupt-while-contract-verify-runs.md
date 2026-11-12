# `verify` does not stop on Ctrl-C or SIGTERM while `contract verify` is running

**Status:** Closed
**Resolution:** `contractVerify`'s signal listener stops `contract verify` and then ends verify with the signal's code, unless `whenEnded` is there to do it.
**Systems:** scripts/verify-semantics.mjs, scripts/verify.mjs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`bun exact.mjs verify` starts `contract verify` beside the device run. `contractVerify` (`scripts/verify-semantics.mjs:18-19` at `dd6f5dcd`) installs a listener for SIGINT, SIGTERM and SIGHUP that forwards the signal to the child's process group:

```js
const pass = signal => { try { process.kill(-child.pid, signal); } catch { /* It has exited. */ } }, signals = ['SIGINT', 'SIGTERM', 'SIGHUP'];
for (const signal of signals) process.on(signal, pass);
```

A listener replaces the signal's default action, which is to end the process, and `pass` never exits. The listeners are removed when the child exits (line 23). So a signal that arrives while `contract verify` runs ends only that child, and `verify.mjs` carries the device run to its end and reports `contract verify: failed`. That child runs for most of a real app's verify: `contract verify` alone takes 14.5 s on `apps/caltrain` and 19 s on `apps/shelf`.

`whenEnded` (`scripts/verify.mjs:205-210`) does this correctly (`kill(); process.exit(signals[signal])`), but it is installed only while verify runs an app's own server (line 247) or has changed the simulator's text size (line 752). The launcher (`scripts/app-cli.mjs:213-214`) only forwards the signal to `verify.mjs` and waits for it.

## The failure

In an app `exact new` made (no server), `bun exact.mjs verify` was started as the leader of a process group and sent one signal 0.7 s later, while `contract verify` ran.

| Signal | What it ended | What ran on | Exit |
|---|---|---|---|
| SIGINT to the process group, as Ctrl-C sends | `contract verify`, `contract test` | the dev server, both browsers, the first-screen checks and the largest-text pass, for 1.7 s, to `verify: 1 problem (1 failure to compile or drive) in 2.4 s; contract verify failed (exit 1), as above` | 1 |
| SIGTERM to the launcher alone, as a harness's timeout sends | `contract verify` | the whole device run, for 5.9 s: tests, checks, reloads and the relaunch, to `verify: clean in 6.5 s; contract verify failed (exit 1), as above` | 1 |

A second Ctrl-C ends the run only after verify has seen `contract verify` exit: in one run a second SIGINT, 0.6 s after the first, was swallowed as well.

Expected: verify stops its children and exits 130 (143 for SIGTERM), as it does once `contract verify` has finished. The same SIGINT sent 3 s into the run ended it at once with exit 130.

## Verification

Ran the cases above at `dd6f5dcd`, listing the run's processes before and after each signal. A trace of `process.on` in the `verify.mjs` process showed `contractVerify`'s three listeners to be the only signal listeners of a web run. Not run: iOS, or an app with a server.

## Direction

Have the handler stop the child and then exit with the signal's code, as `whenEnded` does, or give `main` one signal path for both runs.

## Related

No issue or QUEUE entry mentions `contractVerify` or verify's signals.
