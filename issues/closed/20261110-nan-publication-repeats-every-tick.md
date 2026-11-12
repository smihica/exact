# A published NaN is journaled and delivered again on every tick

**Status:** Closed
**Resolution:** Unchanged NaNs publish once, including nested values, and publication journals use delivered JSON.
**Systems:** game/engine/src/world.rs, game/engine/src/values.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`World::publish_value` skips a value equal to the one stored
(`game/engine/src/world.rs:1036-1042`). Equality is `Stored`'s derived `PartialEq`
(`game/engine/src/values.rs:6-17`, used by `Incoming::matches`, `values.rs:276-283`), under
which `Stored::Number(NaN)` never equals itself. A NaN published every tick is therefore a
change every tick: one journal line, `published_pending` set so the whole record goes to the
app again, and the mutation epoch moved. `game/engine/README.md:163` says "Publication
delivers and journals only changed values."

The journal line is also not the value: the JSON encoder refuses a non-finite number, and
the line carries its error text and an internal path.

## The failure

```rust
fn tick(w: &mut World, _: &Input, _: &()) {
    let (hits, shots) = (0.0f32, 0.0f32);
    w.publish("accuracy", hits / shots);   // 0 / 0 before the first shot
    w.publish("steady", 1.0);
}
```

After ten ticks, observed: ten journal lines
`publish accuracy: Stored.Number.0: non-finite number is not JSON` against one for `steady`,
and `Sim::take_published` returns `{"accuracy":null,"steady":1}` after each of the ten.
`publish_record` with an `f32` field holding NaN does the same. Expected: one line and one
delivery, as for any unchanged value. At 60 Hz one such value fills the journal's 4,096
lines in 68 seconds.

## Verification

Ran the tick above, and a `publish_record` of a struct with a NaN `f32` field, as a
temporary integration test of `exact-game`, taking the published record after each of ten
ticks. Both printed 10 journal lines for `publish accuracy` and 10 delivered records; the
last line was `t=150 tick=9 publish accuracy: Stored.Number.0: non-finite number is not
JSON`.

## Direction

Compare published numbers by bits with NaN canonicalized, as the save and the hash already
treat them (`f64_bits`, `game/engine/src/data/mod.rs:375-381`), and journal what is
delivered (`null`).

## Related

`issues/closed/20260901-nonfinite-layout-offer.md` is the kernel's NaN, not this. An invalid
emitter also journals every tick, from different code (`game/engine/src/emitter.rs:463-466`).
No QUEUE or pitfalls entry.
