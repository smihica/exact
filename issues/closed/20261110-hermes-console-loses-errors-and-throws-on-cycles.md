# On Hermes, `console.error(error)` logs `{}`, and logging a cyclic value or a BigInt fails the answer

**Status:** Closed
**Resolution:** Hermes console preserves errors, abort-listener diagnostics and record boundaries, handles unserializable values, and propagates execution interrupts.
**Systems:** js/src/console.js (`render`), js/src/shim.cc (`install_console`, `exact_js_take_log`), js/src/engine.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd` the native `console` renders each argument that is an object with `JSON.stringify` and no guard (`js/src/shim.cc:190-198`).

- An `Error`'s `message` and `stack` are not enumerable, so its JSON is `{}`. `catch (e) { console.error(e) }`, the usual way to see why a source failed, writes `{}` to the logs.
- `JSON.stringify` throws for a cycle or a BigInt, and the throw leaves the host function into the app. A log line added to debug an answer makes that answer fail.

LLP 1027 D10 admits `console` as "the affordance an agent editing `app.ts` in the cloud will reach for first", and `docs/reference.md`'s `console` row says only where the lines go.

## The failure

In an answer:

```ts
try { JSON.parse("{bad"); } catch (e) { console.error(e); console.error("load failed", e); }
const row: any = { id: 1, children: [] };
row.children.push({ parent: row });
console.log("row", row);
return "reached the return";
```

Observed: `take_logs()` holds `{}` and `load failed {}`, and the answer is `Failed(Error, "cyclical structure in JSON object")`; the `return` is never reached. `console.log({ n: 1n })` throws `TypeError: Do not know how to serialize a BigInt` the same way.

Expected: the error's name, message and stack in the log, and the two objects written in some form with nothing thrown. Chrome 156 prints `Error: boom` with its stack for `console.error(new Error("boom"))`, `cyclic {name: node, self: Object}` for a cyclic object and `{n: 1n}` for the BigInt. The Console Standard's logger formats any value (https://console.spec.whatwg.org/#logger).

## Verification

Ran the snippet as a data source in a temporary `exact-js` test fixture, through `Module::answer`, and read `Module::take_logs()`: the answer and the log lines are those above. A second source that wrapped each call in `try` reported `TypeError: cyclical structure in JSON object` for the cyclic value and `TypeError: Do not know how to serialize a BigInt` for `{ n: 1n }`. The Chrome lines are from the pinned Chromium (156.0.8078.4).

## Direction

In `render`, write an `Error` as its `name: message`, and its stack when it has one, and catch a failed `JSON.stringify`, falling back to `String(value)`.

## Related

No issue or QUEUE entry names it. `console.trace`, `assert`, `table`, `group`, `dir`, `count` and `time` are `undefined` on Hermes; LLP 1027 D10 says a missing member is missing at its first call, so that is not part of this defect.
