# A child's UTF-8 output loses a character split across reads when an invalid byte precedes it

**Status:** Closed
**Resolution:** Child UTF-8 output retains incomplete characters across reads even after invalid bytes.
**Systems:** host/apple/src/executor_process_supervise.rs, desktop processes (LLP 1133.007 D2)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `Utf8::decode` (`host/apple/src/executor_process_supervise.rs:112-122`) decodes a spawn's `encoding: "utf8"` output a chunk at a time and holds back a sequence cut off at the chunk's end. It does so only when that cut-off sequence is the buffer's first error:

```rust
let valid = match std::str::from_utf8(&self.0) {
    Ok(_) => self.0.len(),
    Err(e) if e.error_len().is_none() => e.valid_up_to(),
    Err(_) => self.0.len(),
};
```

When an invalid byte comes earlier in the buffer, `error_len()` is `Some` and the third arm decodes the whole buffer lossily, the incomplete tail included. The tail becomes U+FFFD, and its remaining bytes, in the next chunk, become another.

## The failure

```rust
let mut d = Utf8::default();
let first = d.decode(b"\xffab\xe2\x82");   // an invalid byte, then the first two bytes of "€"
let second = d.decode(b"\xac!");           // its last byte
```

Observed: `"\u{fffd}ab\u{fffd}"` then `"\u{fffd}!"`.

Expected: `"\u{fffd}ab"` then `"€!"`. LLP 1133.007 D2 promises a string "decoded across chunk boundaries", and the type's comment names Node's decoder. Node's `StringDecoder` (https://nodejs.org/api/string_decoder.html) gives the expected pair for these two writes, as does the WHATWG UTF-8 decoder (https://encoding.spec.whatwg.org/#utf-8-decoder) through a streaming `TextDecoder`.

A child reaches this when one read of at most 64 KiB holds any invalid byte (binary noise in a log, Latin-1 text) and ends inside a multi-byte character.

## Verification

A temporary test at the end of `executor_process_supervise.rs` with the lines above (`cargo test -p exact-apple --lib <name> -- --nocapture`) printed:

```
first "�ab�" second "�!"
```

A real child, spawned with `encoding: "utf8"`:

```
/bin/sh -c "printf '\377ab\342\202'; sleep 0.3; printf '\254!\n'"
events: spawn, stdout "�ab�", stdout "�!\n", exit, close
```

Without the `\377` the same child's events are `stdout "ab"`, `stdout "€!\n"`. Node 26.11.1 on the same bytes: `StringDecoder` and `TextDecoder` with `{stream: true}` both give `"�ab"`, `"€!"`, and so does `child.stdout.setEncoding("utf8")` on that child.

## Direction

Decode in a loop: emit the valid prefix, replace an invalid sequence of `error_len()` bytes with U+FFFD and go on, and stop, keeping the rest, at an error whose `error_len()` is `None`.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` names the decoder.
