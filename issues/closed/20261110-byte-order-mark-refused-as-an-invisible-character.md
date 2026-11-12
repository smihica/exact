# A Contract file that starts with a UTF-8 byte order mark is refused, naming a character no one can see

**Status:** Closed
**Resolution:** Contract accepts one leading UTF-8 byte order mark, preserves its source byte offsets, and retains it when formatting.
**Systems:** contract/syntax (lexer)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The lexer takes the source text as it is read (`contract/cli/src/lib.rs:378-389`,
`std::fs::read_to_string`). A leading U+FEFF is not whitespace to
`str::trim_start` (`contract/syntax/src/lexer.rs:121`) and matches no token, so the
first line is refused by the catch-all at `lexer.rs:408-419` with the character
quoted in the message. The character has no width, so the message shows an empty
pair of backquotes. Because the first line did not lex, the parser then refuses
the second line too.

Windows tools write the mark: Windows PowerShell 5.1's `Set-Content -Encoding UTF8`
and `Out-File -Encoding utf8` (Microsoft Learn, "about_Character_Encoding":
`utf8` there is UTF-8 with a BOM), Notepad's "UTF-8 with BOM" and Visual Studio.
Windows is a supported place to write an app (`bun exact.mjs windows --run`; the
CLI's Windows entry point, `contract/cli/src/main.rs:52-61`), and the lexer already
takes the other thing those tools write: the same file with CRLF line ends
compiles.

## The failure

A file whose first three bytes are `EF BB BF`, then a valid app:

```
<U+FEFF>component App
  view
    column
      text "x"
```

`contract build` prints:

```
app.contract:1:1 [syntax-unexpected-char] unexpected `﻿`
 1 | ﻿component App
   | ^
app.contract:2:1 [syntax-expected-declaration] expected `routes`, `font`, `sound`, `shape`, …, `component`, or `members`, found an indented block
 2 |   view
   | ^
```

Expected: the file compiles as it does without the mark. The web's decoders drop a
leading byte order mark (WHATWG Encoding, "UTF-8 decode"), ECMAScript treats
U+FEFF as white space (ECMA-262, "White Space"), and `rustc` skips it.

## Verification

Wrote the four lines with and without the three leading bytes, and with CRLF line
ends, and ran `contract build` (debug build) on each. With the mark: the output
above. Without it, and with CRLF: "0 slots, 0 derives, 0 resources, 0 actions,
2 nodes, 0 regions, 485 bytes". `contract test` and `contract fmt --check` on a
file with the mark print the same `1:1 [syntax-unexpected-char]`. Not run: a
Windows machine.

## Direction

Skip one leading U+FEFF in `Lexer::tokenize_recovering` when the origin is the
start of a file, keeping the first line's columns as they are without it; or strip
it where sources are read.

## Related

`QUEUE.md`, "JS-target grant parity, what is left" mentions U+FEFF in a module's
grant text, a different parser. Nothing about Contract sources in `issues/` or
`docs/agent-pitfalls.md`.
