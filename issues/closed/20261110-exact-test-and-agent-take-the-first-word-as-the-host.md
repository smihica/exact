# `bun exact.mjs test <file>` and `agent <op> …` take their first word as the host

**Status:** Closed
**Resolution:** `test`, `agent` and `verify` take their first word as the host only when it names one (`hostOf`), so a test file or an op written first stays and the run is the web's.
**Systems:** scripts/app-cli.mjs, scripts/help.mjs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The help gives the host as optional: `bun exact.mjs test [web|ios|…]
[<file.test.contract>|<glob>]… [flags]`, "on a host: web by default" (`scripts/help.mjs:141-142`),
and `bun exact.mjs agent [web|ios|…] [flags] <op> [<op> …]` (`:171`). `run` takes the first
argument as the host whenever it does not start with `-` (`scripts/app-cli.mjs:140`):

```js
const host = (verb === 'test' || verb === 'agent' || verb === 'verify') && rest[0] && !rest[0].startsWith('-') ? rest.shift() : 'web';
```

So a test file or an op written first becomes the host. `drivesWeb` is then false (`:304`), the web
build that `test` and `agent` promise is skipped, and the driver is started with that word as its
host. The driver drives the web for any host it does not know.

## The failure

In an app `exact new` made, after one `bun exact.mjs test`:

```
bun exact.mjs agent "tap like" "state liked"     # prints false, exit 0
bun exact.mjs agent web "tap like" "state liked" # prints "tapped #5 …", then true
```

The first op is dropped without a word and the second runs alone. `.exact/runs.jsonl` records the
drive with `"host":"tap like"`.

```
# edit app.ts, then:
bun exact.mjs test app.test.contract       # exit 3: web build is stale: …/app.ts changed … run … web-build
bun exact.mjs test web app.test.contract   # builds, 2 passed, exit 0
```

Before any build the first form exits 1 with `web dist is not a complete build …`. With a fresh
build and two test files beside the app, `bun exact.mjs test other.test.contract` runs both, not
the one named: the name was shifted out of `rest` before the loop at `:316` looks for test files.

Expected: with no host named, both verbs run on the web as with `web` written, building first.

## Verification

Ran each command in an app made with `bun scripts/exact.mjs new` against a worktree at `dd6f5dcd`;
the results are as stated. `bun exact.mjs agent tree` (one op) prints the driver's usage and exits
2, since no op is left.

## Direction

Shift `rest[0]` as the host only when it is one of the hosts the help names
(`scripts/help.mjs:142`).

## Related

`issues/closed/20261102-push-missing-flag-values-use-defaults.md` is the same family in `push.mjs`.
Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` covers this argument.
