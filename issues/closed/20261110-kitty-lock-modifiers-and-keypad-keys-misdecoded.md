# Under the kitty keyboard protocol, Caps Lock or Num Lock disables Ctrl-C, Escape and the arrows, and keypad Enter types a private-use character

**Status:** Closed
**Resolution:** The decoder drops the Caps Lock and Num Lock bits from every key's modifiers, spells super, hyper and meta as `Meta` in a chord so they never type text, and maps the keypad's and other functional keys' Private Use Area numbers to the keys they stand for or to nothing.
**Systems:** host/terminal (term.rs, keys.rs)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

On entry the host writes `CSI > 1 u`, the kitty keyboard protocol's first flag, without asking whether the terminal has it (`host/terminal/src/term/unix.rs:159-161`, at `dd6f5dcd`). It decodes the modifier parameter knowing only shift, alt and ctrl (`host/terminal/src/term.rs:93-109`, `:191-210`).

The protocol sends 1 plus the sum of eight bits: shift 1, alt 2, ctrl 4, super 8, hyper 16, meta 32, caps_lock 64, num_lock 128. A key that produces no text carries the lock bits while the lock is on ("the Lock modifiers are not reported for text producing keys"), and "all non text keypad keys will be reported as separate keys with CSI u encoding, using dedicated numbers", KP_ENTER being 57414 (https://sw.kovidgoyal.net/kitty/keyboard-protocol/).

- `Key::Ctrl` is produced only when `mods & !5 == 0` (`term.rs:202`); with any other bit set the key becomes `Key::Char`.
- For a modifier it has no word for, `with_mods` returns `Key::Chord(name)` with no prefix (`term.rs:95-108`). A `key` handler still hears the name, but the host's defaults and a field's editing keys match `Key::Named` only (`host/terminal/src/keys.rs:84-152`, `:198`, `:226-240`).
- Any number `char::from_u32` accepts becomes `Key::Char` (`term.rs:201-207`), so a keypad key's private-use number is typed into the focused field.

## The failure

| Key, as the protocol sends it | Decoded | Expected |
|---|---|---|
| Ctrl+c, Caps Lock: `CSI 99;69u` | `Char('c')` | `Ctrl('c')` |
| Ctrl+c, Num Lock: `CSI 99;133u` | `Char('c')` | `Ctrl('c')` |
| Ctrl+\, Num Lock: `CSI 92;133u` | `Char('\\')` | `Ctrl('\\')` |
| Escape, Caps Lock: `CSI 27;65u` | `Chord("Escape")` | `Named("Escape")` |
| Down, Num Lock: `CSI 1;129B` | `Chord("ArrowDown")` | `Named("ArrowDown")` |
| Delete, Num Lock: `CSI 3;129~` | `Chord("Delete")` | `Named("Delete")` |
| Super+c: `CSI 99;9u` | `Char('c')` | no text |
| keypad Enter: `CSI 57414u` | `Char('\u{e046}')` | `Named("Enter")` |

With either lock on, Ctrl-C and Ctrl-\ type a letter and do not quit, Escape closes no dialog and the arrows move nothing. kitty, foot and Ghostty turn the protocol on when asked; WezTerm does only with `enable_kitty_keyboard = true`.

## Verification

A temporary test fed each sequence to `decode` and printed the "Decoded" column. Through `Host::key`: an open dialog closed on `CSI 27u` and stayed open on `CSI 27;65u`; with the first of two buttons focused, `CSI B` then Enter pressed the second and `CSI 1;129B` then Enter pressed the first; a focused `input` quit on `CSI 99;5u`, and took `c` on `CSI 99;69u` and U+E046 on `CSI 57414u`. `target/debug/exact-terminal` in a pseudo-terminal kept running after `CSI 99;69u` and after `CSI 92;133u`.

No terminal was run; the byte forms are the specification's. Three terminals' sources send the lock bits: kitty's `key_encoding.c` clears them only when no flag is set, Ghostty's `key_encode.zig` copies `caps_lock` and `num_lock` into the modifier, and foot's `input.c` adds `1 << 6` and `1 << 7`.

## Direction

Mask the lock bits (64 and 128) off the modifier parameter before anything reads it, give super, hyper and meta chord words so they never produce text, and map the protocol's keypad numbers to the keys they stand for.

## Related

None in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
