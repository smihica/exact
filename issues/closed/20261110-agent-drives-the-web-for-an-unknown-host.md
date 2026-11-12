# The agent driver drives the web build for any host name it does not know

**Status:** Closed
**Resolution:** The driver refuses a host it does not carry, naming the ones it does, instead of driving the web and recording the typed name.
**Systems:** scripts/agent.mjs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`main` takes the first word after the flags as the host and checks only that it is there
(`scripts/agent.mjs:1350`, `:1377`). `open` picks the carrier with a chain of comparisons whose
last arm is the web's (`:747-756`): `macos`, `mac`, `host`, `host-ios`, `linux`, `android`,
`windows` and `ios` each match one arm, and every other string falls to `openWeb`. Nothing refuses
a name outside the list the usage line gives (`:1378`; it omits `windows`, which `open` serves).
The drive then runs on the web, exits 0, and is recorded as a run on the host that was typed
(`:1460`, `recordRun`). A test run (`--test`) takes the same path (`:1352-1364`).

## The failure

In an app `exact new` made, with a web build already there:

```
bun exact.mjs agent iOS tree
bun exact.mjs agent tvos tree
bun exact.mjs agent lniux "tap like" "state liked"
bun exact.mjs test iOS
```

Observed: each exits 0 with the web page's answer, and stderr says only
`web: <app> served at http://127.0.0.1:<port>/ …`. No simulator is started and no native build is
read. `test iOS` prints `2 passed, 0 failed`. `.exact/runs.jsonl` gains

```
{"kind":"run","host":"iOS","target":"iOS","what":"drive tree","result":{"ok":true},…,"build":null}
{"kind":"run","host":"iOS","target":"iOS","what":"test app.test.contract","result":{"tests":true,"passed":2,"failed":0,…},…,"build":null}
```

and `bun scripts/evidence.mjs` lists them as it lists a real run on a host whose builds it does not
identify (`scripts/evidence.mjs:10`, `:70`):

```
iOS: test app.test.contract: 2 passed, 0 failed (agent clock); unchecked (a iOS build is not identified yet)
tvos: drive tree: ok (agent clock); unchecked (a tvos build is not identified yet)
```

Expected: a refusal naming the hosts, as the driver gives for an unknown op (`:1455`). LLP 1146 D2
keeps that record so a later reader can tell what was verified; a drive meant for iOS, tvOS or
Linux that passed on the web is evidence of the wrong thing.

## Verification

Ran the four commands in an app made with `bun scripts/exact.mjs new` against a worktree at
`dd6f5dcd`, then `evidence.mjs` with `EXACT_APP_DIR` naming the app; output and rows are as
quoted. In the repository, `bun scripts/agent.mjs iOS tree` and `bun scripts/agent.mjs lniux "tree
change-station"` print Caltrain's web tree and exit 0, and record hosts `iOS` and `lniux`. With no
web build there, `bun scripts/agent.mjs iOS tree` answers `web dist is not a complete build …; run
… bun host/web/build.mjs caltrain-web`.

## Direction

Refuse a host that is not one of the carriers' names before anything is opened, in `main` and in
`open`, and make the web an explicit arm.

## Related

`bun exact.mjs test <file>` and `agent <op>` hand this code a file name or an op as the host
(`scripts/app-cli.mjs:140`); that is a separate defect in how the optional host is read. Nothing in
`issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names an unknown host.
