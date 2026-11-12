# A marker on a clip's first instant never fires for a one-shot, and a loop skips it on the first pass

**Status:** Closed
**Resolution:** Starting markers fire on first playback for clips, layers and blends and on Animator restarts; phase-preserving transitions do not repeat them, and same-tick model redelivery preserves layer playback.
**Systems:** game/engine/src/animation.rs, game/engine/src/animation/layers.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`crossed` reports a marker when playback moves over it in the half-open interval
`(old, new]` (`game/engine/src/animation.rs:687-702`: forward, `old < mark && new >= mark`;
backward, `new <= mark && old > mark`). Playback starts with `old` equal to the clip's first
instant (`animation.rs:1040-1053`, and the same for a layer, `animation/layers.rs:96-117`),
so a marker there is never inside the interval. Nothing refuses such a marker:
`Animation::marker` accepts `seconds >= 0.` (`animation.rs:112-116`), the bake takes any
time from a glTF animation's `extras.markers` (`game/bake/src/lib.rs:244-255`), and the
engine's own `Rig::walk` puts a leg's `step` marker at 0 (`game/engine/src/rig.rs:629`).

A forward one-shot never fires a marker at 0. A reverse one-shot never fires one at the
clip's duration. A looping clip fires a marker at 0 on each wrap but not when it starts.
The engine's documentation does not say so.

## The failure

A generated rig, a 0.5 s clip, `animation::step` every tick for three seconds:

```rust
w.spawn_named("once", (Transform::default(), hero.clone(),
    Animation::play("flinch").once()
        .marker(0.0, "start").marker(duration * 0.5, "middle").marker(duration, "end")));
w.spawn_named("reverse", (Transform::default(), hero.clone(),
    Animation::play("flinch").once().speed(-1.)
        .marker(0.0, "start").marker(duration * 0.5, "middle").marker(duration, "end")));
w.spawn_named("loop", (Transform::default(), hero, Animation::play("flinch").marker(0.0, "start")));
```

Times each marker fired, observed: `once` start 0, middle 1, end 1; `reverse` start 1,
middle 1, end 0; `loop` start 6 (the wraps at 0.5 s to 3.0 s, none at 0). Expected: a
one-shot fires every marker on the clip once, the one at its starting instant on the first
step, and the loop fires `start` seven times. A sound or a hit cue on the first frame of an
attack clip is the usual case. Godot calls a method key at an animation's start position on
the frame it starts: `AnimationPlayer::_process_playback_data` marks that frame as a seek
(`scene/animation/animation_player.cpp`), and `AnimationMixer` then calls the key found at
that time (`animation_mixer.cpp`).

## Verification

Ran the three entities as a temporary integration test of `exact-game`, counting journal
lines `animation <entity> flinch <marker>`; the counts above are its output. A second run:
a one-shot with markers at 0 and 0.001 s fired only the second, on tick 0; a looping
`Rig::walk` (period 0.993 s, `step` markers at 0 and 0.496 s) first crossed `step` on tick
29, half a cycle in. The `Animator` path (`advance_pair`, `animation.rs:718-781`) uses the
same `crossed` on entering a state at phase 0; that was read, not run.

## Direction

Make the starting instant inclusive on the step that starts playback (`!a.sampled` for
`Animation` and layers, a state entered this step for `Animator`): cross `mark == old`
there.

## Related

No issue, QUEUE entry or `docs/game-pitfalls.md` entry mentions markers.
