# `contract … | head` panics with "failed printing to stdout: Broken pipe" and exits 101

**Status:** Closed
**Resolution:** The contract CLI exits quietly on a closed stdout pipe while preserving error handling for internal pipes.
**Systems:** contract/cli/src/main.rs, contract/cli/src/vocab.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

Every command of the `contract` binary prints with `println!` or `print!`
(`contract/cli/src/main.rs:175`, `:255`, `:337`, `:578`; `contract/cli/src/vocab.rs:56-69`).
Rust starts a process with `SIGPIPE` ignored, so when the reader of a pipe closes it early the
write fails with `EPIPE`, and `println!` panics on a failed write. Any output larger than the
pipe's buffer (64 KiB on macOS) that its reader cuts short therefore ends in a panic message
and exit code 101, which reads as a compiler crash.

The outputs that large are the ones a person or an agent is most likely to cut: `vocab --json`
is 360 KB, `symbols` of Shelf 240 KB, `lean` of Shelf 89 KB.

## The failure

```
$ contract vocab --json | head -1
{
thread 'main' panicked at …/library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
$ contract symbols apps/shelf/app.contract | head -c 50
{"definitions":[{"col":7,"end_col":11,"file":"apps
thread 'main' panicked at …/library/std/src/io/stdio.rs:1166:9:
failed printing to stdout: Broken pipe (os error 32)
```

Expected: the process ends quietly when its reader has gone, as `cat big | head -1` does
(killed by `SIGPIPE`, or exit 0), with nothing on stderr.

## Verification

Ran both pipelines with `target/debug/contract` in zsh. Both printed the panic quoted, and
`${pipestatus[1]}` after the first was 101. `contract vocab --all` (57 KB, under the buffer)
piped to `head -2` exits 0.

## Direction

Restore the default `SIGPIPE` disposition at the top of `run` on Unix (the binary already
depends on `libc` there), or write through a locked `stdout` and treat `BrokenPipe` as the end
of output.

## Related

None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` for a broken pipe.
