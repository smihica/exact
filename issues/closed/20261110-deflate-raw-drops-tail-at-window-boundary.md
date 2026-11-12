# `DecompressionStream("deflate-raw")` drops the tail of a valid stream whose last match crosses a 32 KiB boundary, then says the input ended early

**Status:** Closed
**Resolution:** Raw deflate streams drain buffered output after the last input byte, preserving matches across 32 KiB boundaries and closing cleanly.
**Systems:** js/src/compression.rs (`Decompressor::inflate_into`), js/src/streams.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `inflate_into` (`js/src/compression.rs:153-197`) stops asking the inflater for output as soon as the chunk's input is used up (`compression.rs:179`, `Ok(_) if input.is_empty() => break`). miniz_oxide's streaming inflater writes through a 32,768-byte window and returns when the window fills. When the stream's last match crosses that boundary and its end-of-block code lies in bits already read, the call returns `Ok` with all input consumed, part of the match written and the rest held inside the inflater. The loop leaves, so the tail never comes out and `inflated` stays false.

A `deflate` or `gzip` stream has trailer bytes after the deflate data, so the loop runs again. A raw stream has none: `finish` (`compression.rs:143-149`) refuses it.

## The failure

```ts
const cs = new CompressionStream("deflate-raw");
const w = cs.writable.getWriter(); w.write(new Uint8Array(32769)); w.close();
// read cs.readable to its end into `packed` (46 bytes), then:
const ds = new DecompressionStream("deflate-raw");
const dw = ds.writable.getWriter(); for (const c of packed) dw.write(c); dw.close();
// read ds.readable to its end
```

Observed: `TypeError: DecompressionStream: the compressed input ended before its end`, after 32,768 of the 32,769 bytes. With 32,768 bytes, or with `"deflate"` or `"gzip"` at either size, all bytes arrive. Expected: 32,769 bytes and a clean close. Chrome 156's `DecompressionStream`, Bun's `zlib.inflateRawSync` and CPython's `zlib` each inflate the same 46 bytes to 32,769 zeros.

Every stream refused in this review was nearly all one run. Raw streams made by zlib (levels 1, 6 and 9) from 32768·k + j bytes (k 1 to 4, j 1 to 140), each inflated in one `write`:

| Data | Refused |
| --- | --- |
| one byte repeated | 400 of 1,680 |
| a 4-byte pixel repeated | 263 of 1,680 |
| 64 bytes of text, then zeros | 68 of 1,680 |
| text, then 400 zeros | 0 of 1,680 |
| text whose last 200 bytes repeat | 0 of 1,680 |

A flat bitmap or a zero-filled buffer in a ZIP entry is the realistic input.

## Verification

Ran the snippet as a data source in a temporary `exact-js` test: the `TypeError` for `deflate-raw` at 32,769, byte counts for the other five. Ran temporary unit tests in `compression.rs`: of 780 raw streams from the module's own compressor (32768·k + j zeros or text, k 1 to 6, j 0 to 64), 143 are refused, all zeros; the 1,560 `deflate` and `gzip` cases pass. With the change below every case here passes, as do the module's compression tests, and 4,000 sync-flushed zlib messages inflate whole with and without it.

## Direction

When a call wrote output and used up the input, call the inflater again with no input; leave the loop only when such a call writes nothing (it then returns `MZError::Buf`, which the loop already takes as "needs more input").

## Related

`issues/closed/20261023-a-stream-mapper-cannot-yield.md` added the codecs. Their unit tests inflate raw streams of JSON text, none of which ends in a match across a window boundary.
