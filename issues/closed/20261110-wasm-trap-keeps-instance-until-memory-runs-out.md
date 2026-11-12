# A call that traps in the interpreted Rust executor keeps its memory, and the instance is reused until every larger call fails

**Status:** Closed
**Resolution:** A call that traps marks its interpreted instance trapped, and `Swappable` drops that instance and activates the module afresh, so the next call runs in a new instance with its own memory.
**Systems:** logic/src/wasm.rs, logic/src/lib.rs (the interpreted Rust executor)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `Wasm::call` (`logic/src/wasm.rs:164-208`) allocates the request inside the module, calls `exact_logic_call`, and returns with `?` when the call traps (`wasm.rs:178-187`): a panic (`unreachable`, since the module is built with `panic = "abort"`) or a call that spends its fuel. Nothing unwinds in the module, so the request and whatever the call had allocated stay allocated, and `dealloc` (`wasm.rs:188`) is not reached. `Swappable::call` (`logic/src/lib.rs:197-200`) reports the error and keeps `self.executor`, so the next call runs in the same instance, whose memory is capped at 64 MiB (`wasm.rs:72`). When the leaks fill it, `exact_logic_alloc` traps on growth, and every call whose request needs new memory fails until the app is relaunched or another module replaces this one.

The interpreter is the Rust executor on iOS, on Android in production, and wherever a manifest says `wasm` (`scripts/app.mjs:788-805`); `tiered` runs it until the native image is promoted. It runs once a Rust module has replaced the linked code: a development edit, or a delivered update. Caltrain and Update Lab declare `rust.module`.

## The failure

A module whose `parse` assumes its reply is a JSON object:

```rust
let text = String::from_utf8_lossy(&response.body).into_owned();
let object = text.strip_prefix('{').expect("a JSON object");
```

built for `wasm32-unknown-unknown` and loaded through `Swappable::wasm(..).replacement(..)`. The host alternates a reply that is not JSON (a proxy's error page) with one that is, both 1 MiB:

```
a 1048576-byte JSON reply first: Ok(Now(Number(1048575.0)))
the first reply that is not JSON: Err(Interface("Rust wasm: wasm `unreachable` instruction executed"))
after 20 trapped calls a JSON reply fails: Err(Interface("Rust wasm: growth operation limited"))
and every later one: Err(Interface("Rust wasm: growth operation limited"))
```

With 64 KiB replies the good reply fails after 334 trapped calls. The person sees the source's error state, first for the bad reply, then for every reply.

Expected: a failed call costs that call only. LLP 1029.000 §3 has inputs "borrowed only for a call" and says "Module-local caches may be discarded", so a fresh instance is allowed.

## Verification

Run on macOS: the module above, and a test added to `logic/src/tests.rs` that makes `Swappable::wasm(Fixture).replacement(&plan, &receipt(&plan, &module), module)` with that file's helpers, activates it and calls `parse` in a loop. It printed the lines above. With a `parse` that loops forever in place of the panic, the first error was `Rust wasm: the call used all of its 1068609000 fuel (1048609 input bytes)` and a good reply failed after 20 such calls too. Not run: the web JS target's caller (`host/web-js/rust-data.js:77-81`), which by reading also skips `exact_logic_dealloc` when the call throws.

## Direction

After a trap, drop the instance: it holds leaked memory and state cut off in the middle of a call. `Swappable` keeps the module's bytes and plan, so it can clear `executor` and run `activate_module` again before the next call. Freeing the request alone is not enough (each trap above held about 3 MiB for a 1 MiB reply) and not safe when the trap was inside the allocator.

## Related

`issues/20261110-rust-module-load-failure-leaves-page-blank.md` is the web's load path. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` covers a trap in the interpreter.
