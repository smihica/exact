# A child's `close` and its stream's end wait out `exactForceKillAfter` after its group has ended

**Status:** Closed
**Resolution:** While a closed incarnation's group lives and its escalation is pending, the supervisor looks at the group every 10 ms, so the incarnation ends as soon as the group has.
**Systems:** host/apple/src/executor_process_supervise.rs, host/apple/src/executor_process_clock.rs, desktop processes (LLP 1133.007)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, a supervisor that has signalled a real child's process group looks at the group again only when an input arrives or the grace passes. In `Supervisor::incarnation` (`host/apple/src/executor_process_supervise.rs:438-477`), once the incarnation is closed and `escalate` is set, `escalate` is the only deadline and the loop blocks in `self.rx.recv_timeout(self.clock.timeout(deadline))` (`:473`). On the wall clock that is the whole time to the deadline (`executor_process_clock.rs:128-133`), and a group ending sends no `Input`. The look at `:447` comes microseconds after the signal, and again only with the leader's exit or a pipe's end of file, when members may still be exiting or not yet reaped (`group_alive` is `kill(-pgid, 0)`, which counts a zombie). If the last look finds one, the incarnation ends `exactForceKillAfter` later: 5000 ms by default, at most 600000.

The wait holds back `close`, a keep-alive restart and the stream's end. For a released child it holds the supervisor's thread and its place among the 16 streams (`executor_core.rs:35`, given back at `:1033`). The agent's clock polls every 250 ms, so a drive does not show it.

## The failure

`/bin/sh -c <script>` spawned with `drainTimeout: 300` and `forceKillAfter: 2000` (3000 for the releases):

| Script | What happens | Group gone | Stream or supervisor ended |
|---|---|---|---|
| `sleep 60 >/dev/null 2>&1 & exit 0` | the leader exits, a member holds no pipe | 6 to 34 ms | 2.01 to 2.04 s, in 8 of 8 runs |
| `sleep 60 & exit 0` | the leader exits, a member holds the pipes past the drain | 308 to 319 ms | 2.31 to 2.33 s in 5 of 8 runs |
| `sleep 120 >/dev/null 2>&1 & echo $!; wait`, then released | release | 1 ms after it | 3.01 s after it in 3 of 6 runs |
| `sleep 120 & echo $!; wait`, then released | release | 1 ms after it | 3.00 s after it in 1 of 6 runs |

In the other runs each ended with its group. Nothing follows the signal in the first row, so it always waits; in the rest an end of file or the leader's exit follows, and the wait depends on whether the members were reaped by then. `exit 0` alone ends in 3 to 5 ms.

Expected: each ends when its group has. LLP 1133.007 §17: "a child the signal ends ends the stream at once, as does one whose group is gone before the grace".

## Verification

Temporary tests in `host/apple/src/executor_process_real_tests.rs`, run with `cargo test -p exact-apple --lib <name> -- --nocapture`, printed the table. Each polled `os::group_alive(leader)` every millisecond; the release rows called `core.forget(|_| false)` and waited for the child's entry to leave `shared.processes.children`. Not run: Linux (same file).

## Direction

While an escalation is pending and the incarnation is closed, wait in short steps (`groups::escalate` uses 10 ms) and break as soon as `group_alive()` is false.

## Related

QUEUE.md, "Desktop processes: a quit can hold the process's exit for about 10 s" is the quit's escalation in `executor_process_groups.rs`, which polls. `a_leader_that_exits_first_leaves_no_group_behind` asserts the outcome, not the time.
