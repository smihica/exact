# Pausing a live world moves its picture back by the lookahead, up to one tick

**Status:** Closed
**Resolution:** Pausing retains the live phase and lookahead, so paused frames and the first resumed frame keep the stopped pose.
**Systems:** game/engine/src/sim.rs, game/render/src/surface/render.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The live clock draws at `tick - 1 + alpha`, and `alpha` includes the lookahead
(`alpha_numerator`, `game/engine/src/sim.rs:839-854`). Both pause paths clear `live_time`
and `lookahead_us_hz` (`sim.rs:520-526` in `bind_with`, `sim.rs:709-718` in `advance_with`).
With the lookahead gone, `alpha` falls by it and the drawn instant moves back: one tick on a
60 Hz display with 60 Hz ticks, half a tick at 120 Hz. The renderer uses `sim.alpha()`
paused or not (`game/render/src/surface/render.rs:256`).

## The failure

```rust
// Args { #[live] paused: bool, #[live] volume: f32 }; Game::paused reads `paused`;
// HZ is 60; frame is 1000/60 ms.
sim.frame_period(frame);
for _ in 0..30 { sim.advance(t, Clock::Live); t += frame; }
let drawn = |s: &Sim<G>| s.world().tick() as f64 - 1. + s.alpha() as f64;
// drawn(&sim) == 29.000
sim.bind(&[Value::Bool(true), Value::Number(0.)], None).unwrap();
sim.advance(t, Clock::Live);
// drawn(&sim) == 28.000
```

Observed: the last unpaused frame draws tick 29, every paused frame draws tick 28, and the
first frame after the resume draws 29 again. With the pause stamped 4 ms after the last
frame, as a native host stamps it, the paused frames draw 28.240. Expected: a paused world
keeps the picture it stopped on. A body moving at 10 m/s jumps 17 cm back when the game
pauses, while the simulation stays at tick 29.

## Verification

Ran the snippet as a temporary integration test of `exact-game`. The drawn instant was
29.000 before the pause, 28.000 on two paused frames and 29.000 on the first resumed frame;
with the pause and the resume stamped, 29.000, 28.240, 28.240 and 29.240.

Ran a pause through `WorldSurface` and the renderer on a Mac's GPU (a temporary test in
`game/render/src/surface_tests.rs`, live frames through `fixture::render`): a box moving
0.2 m per tick, and the mean column of its pixels in a frame 512 pixels wide.

| frame | tick | alpha | column |
|---|---|---|---|
| tick 10, running | 10 | 1.00 | 248.0 |
| last running | 11 | 1.00 | 250.0 |
| paused (both frames) | 11 | 0.00 | 248.0 |
| first resumed | 12 | 0.00 | 250.0 |

At 120 Hz the paused frames have alpha 0.50 after 1.00, and the box goes from column 241.0
back to 240.0.

## Direction

Keep the live phase and lookahead across a pause (`paused_clock` already stops the phase
from advancing), or hold `alpha` at its last value while `Game::paused` is true. `save`
computes `exact_world_us()` itself (`game/engine/src/sim/snapshot.rs:54`), so it does not
need `world_us` moved at the pause.

## Related

No issue, QUEUE entry or `docs/game-pitfalls.md` entry covers the picture at a pause.
