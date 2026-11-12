# Ctrl-Z does nothing in a terminal app: the host has no suspend or resume

**Status:** Closed
**Resolution:** Unclaimed Ctrl-Z and SIGTSTP finish pending frame output and restore the terminal before stopping; resume restores raw input and repaints without losing or repeating inline history.
**Systems:** host/terminal (term/unix.rs, keys.rs)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

LLP 1101 D6 (Accepted): "`Ctrl-Z` suspends: the host restores the terminal, stops, and repaints in full on `SIGCONT`." Its §6 lists "suspend, restore" in stage 2.

At `dd6f5dcd` none of it exists. Raw mode clears ISIG (`raw.make_raw()`, `host/terminal/src/term/unix.rs:323-328`), so Ctrl-Z arrives as the byte 0x1a and is decoded as `Key::Ctrl('z')` (`host/terminal/src/term.rs:87`). `Host::key` offers it to the app's `key` handler and has no default for it (`host/terminal/src/keys.rs:219-242`). The signal handlers cover SIGINT, SIGTERM, SIGHUP and SIGQUIT only (`unix.rs:56`); nothing handles SIGTSTP or SIGCONT.

## The failure

Run any entry full screen and press Ctrl-Z. Observed: the app stays in the foreground, and a root `key` handler hears the key `z`. Expected: the shell's prompt on a restored terminal, and the app's screen again after `fg`.

A `kill -TSTP` from outside does stop the process, with nothing restored: the shell's prompt is written onto the alternate screen, with mouse reporting, bracketed paste and the kitty keyboard flag still on. After `fg` under bash the app repaints nothing and its terminal is in cooked mode (ICANON and ECHO set, which the app never clears again), where the terminal echoes keys and delivers them a line at a time.

## Verification

Both runs drove `target/debug/exact-terminal <entry> --fullscreen` in a pseudo-terminal, the entry a `column key=keyed` that appends each key it hears to a `text`.

Written 0x1a twice, then 0x03:

```
after 0x1a:        running   ESC[1;8H ... z;]     (the handler's text grew by "z;")
after second 0x1a: running   ESC[1;10H ... z;]
after 0x03:        ... ESC[?1049l, exited 0
```

Under `bash -i` in the pseudo-terminal, so that the app is a job, with `kill -TSTP <pid>` and then `fg`:

```
tty while the app runs: ICANON=0 ECHO=0 ISIG=0
after SIGTSTP: ps stat 'T'
  leave sequence (ESC[?1049l) written: False
  mouse off (ESC[?1000l) written: False
  kitty keyboard pop (ESC[<u) written: False
after fg: ps stat 'S+'
  a frame repainted (ESC[?2026h): False
  tty: ICANON=1 ECHO=1 ISIG=1
```

No other shell was run.

## Direction

On `Key::Ctrl('z')` that no handler claimed, and on SIGTSTP: run `restore`, raise SIGSTOP, and on return enter raw mode again, write the enter sequence and clear `shown` (or `Inline`'s `last`) so the next frame repaints whole. If suspend is not wanted, strike the sentence from LLP 1101 D6.

## Related

`issues/closed/20261006-terminal-termination-restoration.md` covers SIGTERM and SIGHUP. `QUEUE.md`'s "The terminal surface's next work" does not list suspend, and LLP 1101 §11's "Not built" list does not either.
