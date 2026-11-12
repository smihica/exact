# An upload marked `exactHeld` is not held, yet it stops being a request with effects

**Status:** Closed
**Resolution:** A held mark leaves a file transfer's effects unchanged, so a kept upload runs until its reply lands.
**Systems:** runner/src/request.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Only a plain HTTP request is held; an upload, whose file is read before it is sent, never is (LLP 1016.000 D9). `Request::held` says the mark "changes nothing" on anything else (`runner/src/request.rs:607-613`, at `dd6f5dcd`), and `Request::is_held` follows that: it is false when `body_from` or `body_to` is set (`request.rs:619-629`).

`Request::has_effects` reads the raw mark instead (`request.rs:382`, `&& !self.held`). So a POST whose body is a file and which carries the mark is neither held nor a request with effects. The runner stamps both answers on its ticket (`runner/src/runner/commit.rs:941-948`).

A request with effects runs to its end when its answer is let go, and the host keeps the app open until it lands (LLP 1146 §2.1, which excepts "a held request"). The marked upload gets neither, and the clocks still wait on it.

## The failure

A Rust source's answer:

```rust
let mut upload = Request::get(url).body_from("app:/data/a");
upload.method = "POST".into();
Ok(Answer::Later(upload.held()))   // or `upload`
```

TypeScript reaches the same request with `fetch(url, { method: 'POST', exactBodyFrom: 'app:/data/a', exactHeld: true })`: the prelude passes `body_from` and `held` together (`js/src/prelude.js:667`).

Observed, for a resource whose newer arguments let the first request go, with a source that keeps what it is offered:

| request | `is_held` | `has_effects` | while out: `effects_in_flight`, agent `background` | let go | its reply |
|---|---|---|---|---|---|
| upload | false | true | true, `{"mustFinish":true}` | offered to the source, still run | taken |
| upload, `held()` | false | false | false, `{"mustFinish":false}` | not offered, cancelled | dropped |
| JSON POST | false | true | true, `{"mustFinish":true}` | offered, still run | taken |
| JSON POST, `held()` | true | false | false, `{"mustFinish":false}` | not offered, cancelled | dropped |

`state.pending` lists the marked upload without `held: true`, as it lists the unmarked one.

Expected: the second row equals the first, since the mark changes nothing on an upload.

## Verification

A temporary test under `runner/tests/` booted a plan with one resource, `row(n)`, over the source above (`keep_sent` returning a token), read `Runner::effects_in_flight`, the agent's `background` op and `state`, ran an action that changes `n`, and then read `Runner::holds` and `Runner::fulfill` for the first ticket. It printed the table. Not run: the TypeScript path, and a host.

## Direction

Have `has_effects` ask `!self.is_held()` in place of `!self.held`.

## Related

`an_upload_is_never_held` (`runner/src/runner/held_tests.rs:175`) pins `is_held` for this request. `a_request_has_effects_only_when_its_method_is_unsafe_over_http` (`runner/src/request_tests.rs:122`) sets `held` only on a plain POST. No app in the repository marks an upload held. No issue or QUEUE entry covers this.
