# An overlay's value, and an answer holding a 64 K string, read their numbers one ulp off

**Status:** Closed
**Resolution:** Overlays and replies with captured strings use the correctly rounded JSON reader and preserve JavaScript numbers exactly.
**Systems:** js/src/reply.rs, js/src/lib.rs (overlay), js/value
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

LLP 1027 D2 (as built 2026-09-24) says the JSON on both paths is `exact_js_value::json`, whose reader rounds a fraction or exponent correctly, "so a value JavaScript wrote arrives as exactly its value", because serde_json's reader misread 23.7% of shortest round-trip doubles. At `dd6f5dcd` two native paths still parse the module's reply with serde_json:

- `js/src/reply.rs:15-22`: a reply that captured a string of 65,536 characters or more (`engine.has_reply_strings()`, `js/src/prelude.js:1016`) is parsed by `serde_json::from_str`, then decoded from that tree at `reply.rs:53`.
- `js/src/lib.rs:1113-1121`: every `exact.overlay` reply is parsed by `serde_json::from_str` and decoded by `from_json`.

No crate in the workspace enables serde_json's `float_roundtrip` (`cargo tree --workspace -e features -i serde_json` lists `default` and `std`), so both read a 16 or 17 digit number as a neighbouring double.

## The failure

A module whose `answer` returns `{ n: args[0], big: "" }` for `num` and `{ n: args[0], big: "x".repeat(70000) }` for `numBig`, and whose `overlay` returns `answer` unchanged:

```
x = 985.6906946328695  (bits 0x408ecd868ae85d94)
num(x).n            = 985.6906946328695   same bits
numBig(x).n         = 985.6906946328696   one ulp off
overlay(num(x)).n   = 985.6906946328696   one ulp off
```

Also `212.91890726713459` gives `212.9189072671346` and `479.60756426982596` gives `479.6075642698259` on both paths. Expected: the same double on all three. An overlay that passes a number through therefore gives a value that is not equal to the one in the answer it stands in for, and a source's number changes when a string beside it passes 65,535 characters.

## Verification

Ran a temporary `exact-js` test against a fixture module with those two sources and that overlay, declared `as shape N` (`n: number`, `big: string`): `Module::answer` for `num` and `numBig`, then `Module::overlay` with the `num` answer, for three doubles found by comparing `serde_json::from_str(serde_json::to_string(x))` with `x`. It printed the lines above. Not run: the other serde_json readers of module-written JSON that carry numbers to native code, `js/src/door.rs:132` (`native.call`'s request) and `js/src/lib.rs:702` (`native.later`'s).

## Direction

Parse both replies with `exact_js_value::json` (`json::parse`, or `from_json_text` for the overlay's value) and restore captured strings into that tree, so one reader serves every reply. The web wasm path already reads its overlay reply with `json::reply` (`js/web/src/lib.rs:623`).

## Related

QUEUE "A large reply's owed Hermes collection can wait forever" is about when a captured string's memory is collected, not how the reply is read. `issues/closed/20261010-apple-intl-number-format-half-even.md` is about formatting. No entry names serde_json's reader on these paths.
