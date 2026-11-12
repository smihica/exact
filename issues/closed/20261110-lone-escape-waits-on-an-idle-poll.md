# A lone ESC becomes the Escape key at the next idle poll, not after 25 ms

**Status:** Closed
**Resolution:** The decoder records when a held ESC becomes the Escape key (25 ms after the read that ended in it), the loop bounds its poll by that time as well as the next timer, and every pass that reads no input delivers the Escape key once that time has come, however the poll ended.
**Systems:** host/terminal (term/unix.rs, term.rs)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `decode` holds a read's trailing ESC (`host/terminal/src/term.rs:240-242`): it is the Escape key "only if nothing follows within `ESC_WAIT_MS`" (25 ms, `term.rs:216-219`, `:250`). The loop never measures that time. While an ESC is held it caps the poll timeout at 25 ms (`host/terminal/src/term/unix.rs:427-431`) and delivers Escape only when a poll returns with nothing ready (`unix.rs:450-459`).

1. A poll that returns for a wake (a data module's announcement or a resize, `unix.rs:460-468`) or is interrupted by a signal (`unix.rs:444`) goes round again without flushing, and the next poll waits another 25 ms. While wakes arrive less than 25 ms apart and no timer runs, Escape is never delivered.
2. The timeout is the smaller of 25 ms and the time to the app's next timer (`unix.rs:423-426`). A timer due sooner ends the poll with nothing ready and the ESC is flushed early, so the rest of a sequence split across reads arrives as typed characters. With wakes and a timer both, Escape waits for a timer expiry that no wake beats.

Only a terminal that sends Escape as a bare ESC is affected: Terminal.app, tmux, any terminal without the kitty keyboard protocol.

## The failure

A screen whose root handler lists the keys it hears, alone or with `task spin mount` / `every(N, step)`:

```contract
component App
  state heard = ""
  action keyed(k: string)
    heard = `${heard}${k};`
  view
    column key=keyed
      text `heard [${heard}]`
```

| App | Bytes written | Heard | Expected |
|---|---|---|---|
| no timer | ESC | `Escape` after 28 ms | the same |
| no timer, a data module waking the loop every 10 or 15 ms | ESC | nothing in 3 s | `Escape` after 25 ms |
| `every(100, step)`, a wake every 10 ms | ESC | `Escape` after 2, 103 and 303 ms (three runs) | `Escape` after 25 ms |
| `every(5, step)` | ESC, 10 ms later `[A` | `Escape`, `[`, `A` | `ArrowUp` |
| no timer | ESC, 10 ms later `[A` | `ArrowUp` | the same |

The coding harness runs `every(100, step)` while a turn is busy (`apps/harness/terminal.contract:109-110`), so Esc during a streamed reply is late there, not lost: 26 to 187 ms over five runs on the mock provider.

Other terminal programs time the wait from the ESC byte whatever else arrives: ncurses' `ESCDELAY`, Vim's `'ttimeoutlen'`, tmux's `escape-time`.

## Verification

Ran `target/debug/exact-terminal` in a pseudo-terminal, wrote the bytes with the timing shown and read the handler's text from the output: the table. The waking data module was a temporary binary over `cli::run` whose `DataSource::abort_wake` called the waker from a thread. The harness figure is `target/host-dev/harness` in a pseudo-terminal, from ESC during the streamed reply to `[interrupted]`.

## Direction

Record when the ESC was held. On every pass of the loop, flush it once 25 ms have passed since then and not before, and bound the poll timeout by that deadline as well as by the next timer.

## Related

`issues/closed/20261006-terminal-termination-restoration.md` is about signals, not input. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` names the ESC wait.
