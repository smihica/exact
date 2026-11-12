# Ctrl-\ does not quit an app whose key handler prevents the default

**Status:** Closed
**Resolution:** `Host::key` returns `After::Quit` for Ctrl-\ before it dispatches the key to any handler or shortcut, as LLP 1101 D6 says.
**Systems:** host/terminal (keys.rs)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

LLP 1101 D6: "`Ctrl-C` reaches the app's key handlers first. If none handles it, the host exits cleanly. `Ctrl-\` always exits and cannot be shadowed." The code's own comment repeats it: "Ctrl-\ always leaves" (`host/terminal/src/keys.rs:220`, at `dd6f5dcd`).

`Host::key` treats Ctrl-\ as any other key. It dispatches the key to the app's handler and returns `After::Continue` if the handler called `preventDefault()` (`keys.rs:183-191`), then lets an `aria-keyshortcuts` match take it (`keys.rs:209-218`), and reaches `Key::Ctrl('\\') => return After::Quit` only after both (`keys.rs:221`).

The terminal is in raw mode (`host/terminal/src/term/unix.rs:323-328`), so Ctrl-\ is the byte 0x1c, not SIGQUIT. An app that claims every key (an editor, a game, a handler with a bug) therefore leaves no key that ends it; the person has to kill the process from another terminal.

## The failure

```contract
component App
  state n = 0
  action onKey(key: string)
    n = n + 1
    preventDefault()
  view
    column
      text `keys ${n}`
      input value="" key=onKey autofocus=true
```

Run it full screen and press Ctrl-C, Ctrl-\, Ctrl-\. Observed: the screen reads `keys 3` and the app is still running. Expected: `keys 1` after Ctrl-C, as D6 allows, and the app gone at the first Ctrl-\.

The same handler on the root `column` with nothing focused shadows it too, and so does a button with `aria-keyshortcuts="Control+\\"`: the button is pressed and the app stays.

## Verification

`target/debug/exact-terminal <entry> --fullscreen` in a pseudo-terminal, with the bytes 0x03, 0x1c, 0x1c written 150 ms apart: the last frame read `keys 3` and the process was running. The same byte 0x1c written to an entry with no `preventDefault()` ended the process with exit status 0.

A temporary test decoded 0x1c and passed the key to `Host::key` on a headless host:

```
a focused field whose handler prevents the default: Ctrl('\\') -> Continue; screen "keys 1"
a root handler that prevents the default:           Ctrl('\\') -> Continue; screen "keys 1"
a button with aria-keyshortcuts="Control+\":        Ctrl('\\') -> Continue; screen "hits 1"
no handler:                                         Ctrl('\\') -> Quit
```

## Direction

Return `After::Quit` for `Key::Ctrl('\\')` at the top of `Host::key`, before the handler is dispatched.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`. Under the kitty keyboard protocol with Caps Lock or Num Lock on, Ctrl-\ fails for another reason: it is decoded as a character.
