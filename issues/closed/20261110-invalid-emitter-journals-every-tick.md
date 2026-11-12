# An invalid emitter journals the same refusal every tick, and the line names no field

**Status:** Closed
**Resolution:** Emitter refusals name the failing field and its limits, and simulation and rendering journal each reason once until it changes or recovers.
**Systems:** game/engine/src/emitter.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`emitter::step` logs an emitter that fails `validate` on every tick
(`game/engine/src/emitter.rs:463-466`), and the simulation steps emitters after every tick
(`game/engine/src/sim.rs:795`). The journal holds 4,096 lines, so one invalid emitter
replaces the whole journal in 68 seconds at 60 Hz and keeps it full. Other refusals in the
engine journal once: an input overflow once per episode (`sim/queue.rs:116-129`), an
animation error only when it changes (`animation.rs:1165-1168`).

The line is one sentence for every check, `Emitter: invalid shape, rate, lifetime, motion,
keys or bounds #0` (`emitter.rs:257-285`). It names the entity by index and not the value
that is wrong.

## The failure

`spread` is radians in `[0, π]`; an author writes degrees:

```rust
fn setup(w: &mut World, _: &()) {
    let mut cone = Emitter::sparks();
    cone.spread = 30.;
    w.spawn_named("smoke", (Transform::default(), cone));
    w.publish("score", 1.0);
}
```

Observed after `sim.run(70_000.)`: no particles, and all 4,096 journal lines are the
emitter's. The first kept line is `t=1733 tick=104 Emitter: invalid shape, rate, lifetime,
motion, keys or bounds #0`; setup's `spawn` and `publish score` lines are gone. Expected:
one line when the emitter is first refused, and one more if the reason changes, naming the
field.

## Verification

Ran the setup above as a temporary integration test of `exact-game` with an empty tick: after
4,200 ticks the journal held 4,096 lines, all of them the emitter's, and no `publish score`
line. A second run with one emitter whose `spread` is 30 and one whose `lifetime` is 120
journaled the same sentence for each, `#0` and `#1`, on each of its three ticks.

## Direction

Journal an emitter's refusal when it starts or changes, keeping the refused entities in
unsaved world state as the animation runtime keeps `errors` (a field in `EmitterState`
would move every pin), and have `validate` return the failing field.

## Related

A published NaN also journals every tick, from different code
(`game/engine/src/world.rs:1036-1042`). No issue, QUEUE entry or `docs/game-pitfalls.md`
entry covers emitters.
