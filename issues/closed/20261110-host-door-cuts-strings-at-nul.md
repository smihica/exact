# On Hermes, `store.set` keeps only the part of a value before its first U+0000

**Status:** Closed
**Resolution:** The host door carries its two strings and its reply with their lengths, and the console's lines come back with theirs, so U+0000 crosses whole.
**Systems:** js/src/shim.cc (`install_host`, `exact_js_take_log`), js/src/door.rs, js/src/engine.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd` the host door carries its two string arguments, and its reply, as NUL-terminated C strings. The shim passes `a.c_str()` and `b.c_str()` (`js/src/shim.cc:236-240`), the door reads each with `CStr::from_ptr` (`js/src/door.rs:19-20`), and the shim reads the reply with `std::string(out)` (`shim.cc:241`). A JavaScript string may hold U+0000, whose UTF-8 is a zero byte, so everything from the first one on is dropped, with no error.

Most door traffic is JSON, which escapes U+0000. These are not JSON:

- `store.set(name, value)` (op 3, `js/src/prelude.js:737`): the value is stored cut short.
- `store.get(name)` (op 2): a stored value that holds U+0000 is cut on the way back.
- The console's lines: `exact_js_take_log` joins every line and returns one C string (`shim.cc:513-523`), which `engine.rs:117-128` reads to the first zero byte, so one `console.log("x\0y")` loses the rest of that line and every later line of the batch.

A `store.set` inside `store.exclusive` goes as JSON (op 26, `prelude.js:746`) and keeps the whole value, so the two ways to write one name disagree.

## The failure

In an answer, with `secret.keep k2` and `secret.keep k3` granted and `k3` holding `"a\0b"` from the host:

```ts
const s = "a\u0000b";
store.set("k2", s);
const back = store.get("k2"), seeded = store.get("k3");
console.log("x\u0000y");
console.log("second line");
return JSON.stringify({ direct: s.length, length: back.length, seeded: seeded.length });
```

Observed: the answer is `{"direct":3,"length":1,"seeded":1}`, the runner's `Store::get("k2")` is `Some("a")`, and `take_logs()` is `["x"]`. Expected: `length` and `seeded` 3, the store holding `"a\0b"`, and both log lines whole. A value that is bytes as a string, such as `atob(...)` of a key, holds a zero byte about once in 256 characters.

## Verification

Ran the snippet as a data source in a temporary `exact-js` test, through `Module::answer` with a `Store` seeded with `k3`, and printed the answer, `store.get("k2")` and `Module::take_logs()`: the values above. The `store.exclusive` path is read from the lines cited, not run.

## Direction

Pass lengths with the door's strings (the shim holds `std::string`s on both sides, and Rust can take a pointer and a length), or refuse a string with U+0000 at the door by name. Return the console's lines with a length too.

## Related

QUEUE "A native host refuses a data-module answer holding a lone UTF-16 surrogate" is about another character the seam cannot carry, in an answer, and it is refused aloud. No entry names U+0000.
