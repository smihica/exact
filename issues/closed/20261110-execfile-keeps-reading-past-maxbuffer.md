# `execFile` keeps every byte a child writes after its output passes `maxBuffer`

**Status:** Closed
**Resolution:** Past `maxBuffer` the host keeps one byte more of the pipe and closes both pipes before it signals, as Node's `execFile` does.
**Systems:** host/apple/src/executor_process_real.rs, desktop processes (LLP 1133.007 D3)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `real::exec` collects a real child's output into two vectors and never stops. When one passes `maxBuffer` it signals the group once and goes on appending (`host/apple/src/executor_process_real.rs:537-546`):

```rust
let into = if err { &mut stderr } else { &mut stdout };
into.extend_from_slice(&bytes);
if into.len() > spec.max_buffer && !overflow {
    overflow = true;
    child.signal(&spec.kill_signal);
    escalate = Some(clock.now() + 5000.0);
}
```

The readers read until the pipes close. A child that exits on `killSignal` costs little. One that outlives it (it ignores the signal or handles it slowly, or `killSignal` is one that ends nothing, such as `SIGCONT`) fills the host's memory for the 5 s until `SIGKILL`, on both pipes: the one that did not overflow has no bound either. `exec_reply` then answers only the first `maxBuffer` bytes of each (`executor_process_supervise.rs:660`).

`maxBuffer` is what bounds a one-shot's memory: the executor admits one-shots against 64 MiB of reply ceilings (`executor_process.rs:44-49`). D3 takes Node's rules, and Node ends the read: "If exceeded, the child process is terminated and any output is truncated" (https://nodejs.org/api/child_process.html#child_processexecfilefile-args-options-callback); its `execFile` destroys both output streams before it signals.

## The failure

```json
{ "kind": "execFile", "program": "/bin/sh", "maxBuffer": 1000, "args": ["-c", "<script>"] }
```

| Script | Answer after | `signal` | Peak resident size grew |
|---|---|---|---|
| `head -c 500 /dev/zero` (no overflow) | 6 ms | none | 0.4 MiB |
| `exec cat /dev/zero` | 5 to 8 ms | `SIGTERM` | 1.2 MiB |
| `trap '' TERM; while :; do head -c 4000000 /dev/zero; sleep 0.1; done` | 5.02 s | `SIGKILL` | 422 MiB |
| `trap '' TERM; head -c 2000 /dev/zero; while :; do head -c 4000000 /dev/zero >&2; sleep 0.1; done` | 5.01 s | `SIGKILL` | 422 MiB |

The last three answers say `"overflow":"stdout"` and carry its first 1000 bytes. Without the `sleep` the growth is whatever a pipe carries in 5 s.

Expected: the host holds about `maxBuffer` bytes of each pipe. Node 26.11.1 on the third script keeps 1000 bytes, reports `ERR_CHILD_PROCESS_STDIO_MAXBUFFER`, and grows 0.4 MiB in 3 s.

## Verification

Temporary tests in `host/apple/src/executor_process_real_tests.rs`, one a process (`cargo test -p exact-apple --lib <name> -- --nocapture --test-threads=1`), read `getrusage`'s `ru_maxrss` before and after the call, after one `execFile` to warm the executor. They printed the table. Node's figures are `child_process.execFile` with `maxBuffer: 1000` on the same scripts. Not run: Linux (same file).

## Direction

Keep at most `max_buffer + 1` bytes of each pipe, and at the first overflow stop reading both (`child.close_pipes()`, which already ends the readers at the drain timeout), so the loop ends when the leader exits.

## Related

`exec_file_keeps_its_limits` checks only that `overflow` is reported. QUEUE.md's desktop process entries (pid reuse, the quit's wait, the member list's buffer) do not cover it.
