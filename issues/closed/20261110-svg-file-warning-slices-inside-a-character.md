# An SVG image aborts the app when a refused attribute value has a multi-byte character across its 40th byte

**Status:** Closed
**Resolution:** A refused value is shortened for its warning by characters, so a long CJK family name no longer panics the SVG reader.
**Systems:** host/svg-file/src/lower.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

When a style row refuses an attribute's value, the file reader warns, and shortens
the value for the message with a byte slice (`host/svg-file/src/lower.rs:551-564`):

```rust
if text.len() > 40 { format!("{}…", &text[..40]) }
```

When byte 40 falls inside a multi-byte character the slice panics. The `release`
profile is `panic = "abort"` (`Cargo.toml:66`) and `host-dev`, which development
builds use, inherits it, so the process dies.

One ordinary attribute always reaches this code. `font-family` is a `u16` row (an
index into a plan's font stacks, `kernel/tables/schema.json:3323-3326`), so every
family name a file writes is refused. A font stack that names CJK fonts is longer
than 40 bytes and has three-byte characters, so byte 40 often falls inside one. Any
other refused value of that shape does the same (a `stroke-dasharray`, a `style=""`
declaration).

`exact_svg_file::load` is what the Apple host (`host/apple/src/svg_image.rs:22`), the
Linux host (`host/linux/src/image/svg.rs:206`) and the bake
(`bake/src/receipt.rs:309`) call to read an SVG image, bundled or remote. None
catches a panic.

## The failure

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 20">
  <text x="0" y="15" font-family="'游ゴシック体', YuGothic, '游ゴシック Medium', 'Yu Gothic Medium', sans-serif">こんにちは</text>
</svg>
```

That is a common CSS font stack for Japanese text (Yu Gothic). Observed, from
`exact_svg_file::load`:

```
panicked at host/svg-file/src/lower.rs:560:29:
end byte index 40 is not a char boundary; it is inside 'シ' (bytes 39..42 of string)
```

Expected: the picture, with a warning for what was not read.

## Verification

Run:

- An example binary calling `exact_svg_file::load` on the file above, built with
  `cargo build --profile host-dev`, printed the panic and exited with status 134
  (`SIGABRT`).
- A temporary test in `host/svg-file/tests` under `catch_unwind` showed the same
  panic for the stack without quotes and for `stroke-dasharray` of fourteen `あ`.
  Stacks whose 40th byte is a character boundary only warn
  (`font-family="ヒラギノ角ゴ ProN W3, Hiragino Kaku Gothic ProN, メイリオ, sans-serif"`).

No host was driven; the three callers were read.

## Direction

Cut at a character boundary: the last one at or before byte 40
(`text.char_indices()`), or `text.chars().take(40)`.

## Related

`issues/20261110-svg-file-drops-common-text-values.md` is the refusal of every
`font-family`, which is why this path is reached. `QUEUE.md`, "The compiler's check
of a bundled SVG image",
says a file the parser trims is known only from the host's log; it does not say the
log line can panic.
