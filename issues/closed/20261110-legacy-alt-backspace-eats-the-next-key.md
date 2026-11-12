# Legacy Alt+Backspace and Alt+Enter lose their modifier and turn the next key into Alt+key

**Status:** Closed
**Resolution:** `decode` takes an ESC followed by DEL or a C0 key out of the stream as one Alt chord (`Alt+Backspace`, `Alt+Enter`, `Alt+Tab`, `Control+Alt+<letter>`), so vte never sees the pair and the next key is itself.
**Systems:** host/terminal (term.rs)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

A terminal without the kitty keyboard protocol sends Alt+Backspace as `ESC DEL` and Alt+Enter as `ESC CR` (xterm's "meta sends escape"; tmux; Terminal.app with "Use Option as Meta key"). At `dd6f5dcd` the decoder mishandles both.

`decode` takes DEL out of the stream by hand: it feeds the bytes before it to `vte`, emits Backspace and skips the byte (`host/terminal/src/term.rs:229-238`). For `ESC DEL` the bytes before it are the ESC, so the parser is left in its escape state. `ESC CR` reaches `vte` whole, and `vte` executes a C0 control inside an escape sequence without leaving the escape state (`execute`, `term.rs:119-131`). Either way the parser still expects the final byte of an escape sequence, however long the pause, so the next printable byte arrives in `esc_dispatch` and is reported as `Alt+<key>` (`term.rs:133-138`). `ESC TAB` and `ESC BS` do the same.

## The failure

```rust
let (mut p, mut d) = (vte::Parser::new(), Decoder::default());
decode(&mut p, &mut d, b"\x1b\x7f"); // [Key(Named("Backspace"))]
decode(&mut p, &mut d, b"a");        // [Key(Chord("Alt+a"))]
decode(&mut p, &mut d, b"\x1b\r");   // [Key(Named("Enter"))]
decode(&mut p, &mut d, b"a");        // [Key(Chord("Alt+a"))]
```

Expected: `Alt+Backspace` then `a`; `Alt+Enter` then `a`. Under the kitty protocol the same keys arrive as `CSI 127;3u` and `CSI 13;3u` and decode to those chords.

In a field, Option+Delete removes one character and the next letter typed is dropped: `Alt+a` matches nothing in `field_key` (`host/terminal/src/keys.rs:78-165`). In the coding harness, whose prompt sends on Enter unless Shift or Alt is held (`apps/harness/terminal.contract:172-174`), Alt+Enter sends the prompt where it should break the line, and the first letter of what is typed next is dropped.

## Verification

A temporary test calling `decode` printed the four results above.

In tmux 3.8, `exact-terminal --fullscreen` on a screen with one focused `input` that shows its value: `tmux send-keys a b`, `send-keys M-BSpace`, `send-keys c d` left the value `ad`. The same bytes written to a pseudo-terminal (`ab`, `ESC DEL`, `cd`) left `ad`, also with a two second pause after the `ESC DEL`; `ab`, `DEL`, `cd` left `acd`.

`target/host-dev/harness` in tmux: `send-keys o n e`, `send-keys M-Enter`, `send-keys t w o` submitted `one` (the mock reply began) and left `wo` in the prompt.

Terminal.app was not run.

## Direction

In `decode`, when the byte after an ESC is DEL or a C0 key (CR, TAB, BS), emit the chord (`Alt+Backspace`, `Alt+Enter`, and so on) and feed neither byte to the parser.

## Related

None in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
