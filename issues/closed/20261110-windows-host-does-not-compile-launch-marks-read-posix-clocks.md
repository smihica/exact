# The Windows host does not compile for Windows: the painting host's launch marks call POSIX clock functions with no `cfg`

**Status:** Closed
**Resolution:** The Windows host compiles with a process-local monotonic journal clock; POSIX clocks stay on Unix and process-start and DRM timing stay on Linux.
**Systems:** host/linux/src/launch_marks.rs, host/linux/src/lib.rs, host/windows
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `ab72e13e`. `exact-windows` is built on `exact-linux`, the painting host it shares with Linux and Android. That crate compiles `launch_marks` for every target (`host/linux/src/lib.rs:81`), and the module reads its clocks through `libc` with no `cfg`: `libc::clockid_t` and `libc::clock_gettime` (`launch_marks.rs:10-21`), `libc::CLOCK_MONOTONIC` (`:31`, `:37`) and `libc::sysconf(libc::_SC_CLK_TCK)` (`:48`). Only the choice between `CLOCK_BOOTTIME` and `CLOCK_MONOTONIC` is conditional (`:25-32`). The `libc` crate has none of these for a Windows target, so the crate fails to compile there and the Windows host with it.

## The failure

The async lane's tier 2 `windows` step (`scripts/async.mjs:99`):

```sh
env CRATE_CC_NO_DEFAULTS=1 CC_x86_64_pc_windows_gnu='zig cc -target x86_64-windows-gnu' \
  cargo check --locked --keep-going --target x86_64-pc-windows-gnu -p exact-windows -p windows-desk
```

Observed:

```
error[E0425]: cannot find type `clockid_t` in crate `libc`
error[E0425]: cannot find function `clock_gettime` in crate `libc`
error[E0425]: cannot find value `CLOCK_MONOTONIC` in crate `libc`   (twice)
error[E0425]: cannot find function `sysconf` in crate `libc`
error[E0425]: cannot find value `_SC_CLK_TCK` in crate `libc`
error: could not compile `exact-linux` (lib) due to 6 previous errors
```

Expected: the check passes, as AGENTS.md says the hourly tier does for the Windows host with Windows Desk.

## Verification

Ran the command above on a Mac with the `x86_64-pc-windows-gnu` target and Zig, in a checkout whose only difference from `e0d2ea72` is under `host/windows`: the six errors, all in `host/linux/src/launch_marks.rs`. Not run: the ARM64 step (`aarch64-pc-windows-gnullvm`), which compiles the same module, and a build on Windows itself.

## Direction

Give the module a clock on every target it is compiled for: keep the POSIX reads under `cfg(unix)` and give Windows its own monotonic read, or leave the display path's marks out where no display path writes them. `process_start` reads `/proc/self/stat` and already answers `None` elsewhere; its `sysconf` needs the same gate.

## Related

`issues/20261110-async-lane-results-are-not-on-origin.md`: the tier that runs this check reports nowhere on origin, which is how a host can stop compiling unnoticed. No issue or QUEUE entry names `launch_marks`.
