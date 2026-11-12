# Two store readers that each forget a name the store does not keep fail every commit with `Cycle`

**Status:** Closed
**Resolution:** Forgetting an absent name preserves the store revision and lets readers settle; host removals and presence changes are retained.
**Systems:** runner/src/store.rs, runner/src/runner/settlement.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`Store::set` of the value already kept changes nothing: no revision and no write, "so a source that writes as it answers does not make the store's readers stale with a value they already have" (`runner/src/store.rs:482-500`, at `dd6f5dcd`). `Store::forget` has no such test (`store.rs:515-529`): it adds 1 to the revision whether or not anything was kept.

A resource that read the store is reused only at the revision it observed, and settlement repeats while a store reader's revision is behind (`runner/src/runner/settlement.rs:1083-1097`). After one pass more than the plan has resources it returns `RunnerError::Cycle` (`settlement.rs:252-256`). Two readers whose answers each forget a name leave each other behind on every pass.

## The failure

```
component App
  resource a = a() as shape Row
  resource b = b() as shape Row
  view
    text `${a.n} ${b.n}`
```

with a data source that grants `secret.keep k`, keeps nothing under `k`, and answers both sources with:

```rust
let _ = store.get("k");
store.forget("k")?;
Ok(Answer::Now(row))
```

| the answer's write | result | answers asked |
|---|---|---|
| `store.forget("k")` | `Runner::boot` returns `Err(Cycle)` | `a b a b a b` |
| `store.set("k", "")` | settles | `a b` |
| `store.forget("k")` only when `store.get("k")` is some | settles | `a b` |
| `store.forget("k")`, one resource | settles | `a` |

When the forgetting starts later (both sources take a state as their argument and forget once it is above 0), the action that changes the state is refused, journaled `act bump refused: Cycle`, its state put back, and refused again on every try.

Expected: forgetting what is not kept changes nothing a reader can see, so the first row settles after `a b`. Web Storage's `removeItem` does nothing for a key that does not exist (HTML Standard, Web storage, `removeItem(key)`).

TypeScript's `store.forget(name)` in an answer is this call (`js/src/door.rs:54`), and `docs/contract-for-agents.md:1231` names it for answers. A helper that clears a session whenever it finds no valid token does this from every answer that uses it, on a signed-out launch.

## Verification

A temporary test under `runner/tests/` booted the plan above with each data source and printed the table and the journal line. Not run: a TypeScript source, and the JS target, whose `Store.set` returns early only for an equal value that is not null (`host/web-js/rt.js:389`) and whose commit re-asks store readers at most four times (`rt.js:187`).

## Direction

In `Store::forget`, add to the revision only when a value was removed or the name's presence state changes (a locked presence name has no value in the snapshot and reads absent afterwards, `runner/src/store/presence.rs:187`). The write to the host stays: the doc comment says the host forgets either way.

## Related

`keeping_what_is_kept_is_no_change` (`store.rs:921`) pins the rule for `set`. No issue or QUEUE entry mentions `forget` and the revision.
