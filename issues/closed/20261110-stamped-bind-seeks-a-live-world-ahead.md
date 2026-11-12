# A stamped bind on a live canvas seeks on the seekable clock and adds up to one tick of world time

**Status:** Closed
**Resolution:** Stamped binds retain the active live clock, so argument changes do not convert lookahead into elapsed world time.
**Systems:** game/engine/src/sim.rs, game/render/src/surface/args.rs, host/apple, host/linux
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Sim::bind_with` seeks to the host's stamp with `Clock::Seekable` whatever clock the canvas
runs on (`game/engine/src/sim.rs:452-454`). On a live canvas that seek starts from
`exact_world_us()` (`sim.rs:626-642`), which rounds world time up to the last completed
tick. The live clock ran that tick as lookahead (`sim.rs:622-625`), so the rounding turns
lookahead into elapsed time. The seek also drops the live phase and lookahead
(`sim.rs:733-734`), and the next live frame starts again with the full lookahead
(`sim.rs:565-573`). Each such bind adds between nothing and one tick.

A canvas is bound when an argument's value changes (`runner/src/instance.rs:1186`), so an
ordinary frame has no bind. Native hosts always stamp one (`gpu/src/native.rs:840-844`);
the web host does not outside the agent (`host/web/gpu-glue.js:975-978`).

- Linux stamps with its clock's last value (`host/linux/src/surfaces.rs:684-688`) and
  renders at that clock (`host/linux/src/surfaces/pixels.rs:66`), so every bind seeks.
- Apple stamps with the wall clock (`host/apple/Sources/ExactSurfaces/CanvasSeams.swift:162-168`)
  and advances canvases at the display link's target time
  (`host/apple/Sources/ExactKit/Frames.swift:58`, `:112`). A stamp earlier than the last
  frame's returns before the seek (`sim.rs:682-684`), so only a bind applied between a vsync
  and that callback's canvas advance seeks: on macOS, a commit from a frame task or a due
  timer earlier in the callback (`Frames.swift:73-90`).

## The failure

```rust
#[derive(Default, Args)]
pub struct Options { #[live] pub paused: bool, #[live] pub volume: f32 }
// Game::tick does nothing; HZ is 60. Ten seconds of frames:
sim.frame_period(frame);
for i in 0..frames {
    sim.advance(t, Clock::Live);
    sim.bind(&[Value::Bool(false), Value::Number(i as f64 * 0.001)], Some(t + frame * 0.4)).unwrap();
    t += frame;
}
```

Ticks without and with the binds: 90 Hz, 599 and 899; 144 Hz, 599 and 719; 60 Hz with frame
stamps jittered by 1 ms, 600 and 877. Expected: 600. A steady 60 or 120 Hz display gains at
most one tick, because the frame after a bind realigns the tick grid.

So where the display rate is not a multiple of the tick rate, or frame stamps jitter, a game
runs 20 to 50% fast while a live argument changes every frame (a slider being dragged). A
single change (a pause, a setting) skips at most one tick.

## Verification

Ran the loop as a temporary integration test of `exact-game`; the counts are its output.
Also run:

- A stamp equal to the frame's own (Linux), 60 Hz, stamps jittered by 0.05 ms: 600 and 706.
- Frames stamped one period ahead (Apple). A bind half a period before the last frame's
  stamp: no change. A bind 0.3 ms after the previous frame's stamp, every frame for two of
  the ten seconds: 30 more ticks at 75 Hz, 60 at 90 Hz, 24 at 144 Hz.
- Unstamped binds (the web): no change. `Clock::Seekable`, the agent's clock that
  `game/prove.mjs` drives: 599 with and without binds, so no proof shows it.

No host was run; the host call sites were read.

## Direction

Do not seek a live canvas: `WorldSurface::bind_arguments`
(`game/render/src/surface/args.rs:56-84`) knows `self.seekable` and can pass the stamp only
then. Or have `bind_with` advance with `Clock::Live` while `live_time` is set.

## Related

No issue or QUEUE entry names `bind_with`, `exact_world_us` or the lookahead.
