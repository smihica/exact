# A pair that begins and ends touching in one physics step is reported as end, then begin

**Status:** Closed
**Resolution:** Collision events and Announce journals preserve each pair's transition order within the step, including begin followed by end.
**Systems:** game/physics/src/step.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`physics::step` collects collision events from two passes: Rapier's step, which detects at the poses the step starts with, and, when a kinematic body moved, a collision-only pass at the poses it ends with (`game/physics/src/step.rs:455-480` at `dd6f5dcd`). It then sorts them with the transition as the last key (`step.rs:496`):

```rust
events.sort_by_key(|e| (e.a, e.b, e.began));
```

`false` sorts before `true`. When a pair begins touching in the first pass and stops in the second, `Physics.events` holds the end before the begin, and the `Announce` journal logs "untouch" before "touch".

The API promises order: `physics::events` returns "This tick's ordered transitions" (`lib.rs:42`), and the README says "Events are sorted; `Announce` journals transitions" (`game/physics/README.md:17`). A game that applies a pair's transitions in that order ends the tick believing the pair touches, and no later event corrects it.

No game in the repository reads `physics::events` or attaches `Announce` yet; `game/recipes/sound-on-contact.md:31` sends authors to these events.

## The failure

A thin static sensor, a ball thrown through it, and a kinematic body 500 m away that starts to move on the tick after the ball entered:

```rust
w.spawn_named("gate", (Transform::default(), physics::Announce, Collider { sensor: true,
    shape: Shape::Box { half: Vec3::new(0.01, 1.0, 1.0) }, ..Collider::default() }));
w.spawn_named("ball", (Transform::at(-1.0, 0.0, 0.0),
    Body { velocity: Vec3::new(6.0, 0.0, 0.0), gravity: 0.0, ..Body::default() },
    Collider { shape: Shape::Sphere { radius: 0.03 }, ..Collider::default() }));
let walker = w.spawn_named("walker", (Transform::at(500.0, 0.0, 0.0),
    Body { kind: BodyKind::Kinematic, ..Body::default() }, Collider::default()));
// 60 Hz; from tick 10 on: walker's position.z += 0.02; every tick: physics::step
```

Observed at tick 10: the `began` flags are `[false, true]` and the journal reads "untouch gate × ball", "touch gate × ball". Expected `[true, false]`. A walker that starts on any other tick gives the begin and the end in two successive ticks.

A kinematic sensor that leaves on the tick whose first pass reports the ball does the same. The capsule controller installs such a sensor.

## Verification

Ran both cases as temporary tests in `game/physics/tests/` (from `game/`, `cargo test -p exact-game-physics --test <name> -- --nocapture`). Each printed the reversed pair and failed its order assertion. With the sort below both passed, as did every test of `exact-game-physics` (52 passed, 3 ignored, the pins of `physics/tests/pins.json` included) and the logic tests of the three games that step physics (asset-fixture with its tick 60 pin, forest, rivals). The games' later proof pins (`game/prove.mjs`) were not run: a pinned digest includes that tick's `Physics.events`, so it moves only if the pinned tick holds such a pair.

## Direction

Sort by the pair alone, `sort_by_key(|e| (e.a, e.b))`. The sort is stable, so a pair's events keep the order the passes produced them, and pairs stay ordered as `Touch` documents (`types.rs:226`).

## Related

No issue or `QUEUE.md` entry covers the order of physics events.
