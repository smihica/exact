# The terminal host's headless `key Control+ArrowUp` presses Ctrl-A

**Status:** Closed
**Resolution:** `key_of` reads the modifiers in any order and builds the key with the decoder's own `with_mods` and `text_with_mods`, so `Control+ArrowUp` is the decoder's `Control+ArrowUp` chord, and the `key` verb refuses a name that is no key.
**Systems:** host/terminal/src/cli.rs
**Severity:** P3
**Author:** Claude (Opus 5.5), from GPT-6 Astra's whole-project review (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

A headless drive's `key <name>` goes through `key_of` (`host/terminal/src/cli.rs:452`).
For any name beginning `Control+`, `key_of` takes the first character after it
(`cli.rs:52-54`):

```rust
if let Some(c) = name.strip_prefix("Control+") {
    return Key::Ctrl(c.chars().next().unwrap_or('c').to_ascii_lowercase());
}
```

So `Control+ArrowUp` is `Ctrl('a')`, `Control+Enter` is `Ctrl('e')` and
`Control+Shift+Enter` is `Ctrl('s')`. The terminal's own decoder makes a chord of a named
key with Control, `Key::Chord("Control+ArrowUp")` (`host/terminal/src/term.rs:94-109`).
The two reach different handlers: in a field `Ctrl('a')` is Home and `Ctrl('e')` is End
(`host/terminal/src/keys.rs:145`, `:149`), while a textarea inserts a newline for a chord
ending `+Enter` (`keys.rs:153`). A drive cannot press the key a person presses.

## To see it

Confirmed by reading the code only; not run. In a headless drive with a textarea focused,
`key Control+Enter` reaches the field as `Ctrl('e')` and moves the caret to the end of the
line, where the same key pressed in a terminal speaking kitty's keyboard protocol
arrives as `Control+Enter` (`term.rs:190-200`) and inserts a newline.
`key Control+ArrowUp` in a field moves the caret to the start of the line.

## What a fix is

Parse the modifiers and the key as the decoder names them: `Key::Ctrl` only for
`Control+<one character>`, and for a named key the same `Key::Chord` the decoder makes
(share `with_mods`'s spelling). Refuse a name that is neither, as other drive verbs refuse
what they cannot do.
