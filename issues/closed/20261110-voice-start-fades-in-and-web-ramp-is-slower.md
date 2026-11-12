# The web output smooths a voice's gain more slowly than the native mixer, so short sounds are quieter on the web

**Status:** Closed
**Resolution:** Web voice gains ramp linearly over 10 ms from their current values, matching the native mixer.
**Systems:** game/audio/src/web.rs, game/audio/src/native.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Both outputs start a voice silent and smooth it to the gains the Player sets in the same sync (the first assertion of `gains_ramp_per_sample_over_ten_ms`, `game/audio/src/apple_tests.rs:93`, pins this for the native mixer). The two smoothings are different curves:

- The native mixer, which Apple and Windows share, ramps linearly over `rate * 0.01` samples and is at the gain after 10 ms (`game/audio/src/native.rs:427`, `:494-498` at `dd6f5dcd`).
- The web sets both gains to 0 and calls `set_target_at_time(gain, now, 0.01)` (`game/audio/src/web.rs:99-100`, `:207-208`). `setTargetAtTime` approaches the gain as `1 - e^(-t/0.01)`: 63% at 10 ms, 95% at 30 ms ([Web Audio, setTargetAtTime](https://www.w3.org/TR/webaudio-1.0/#dom-audioparam-settargetattime)).

A sound shorter than about 50 ms never reaches its gain on the web, so one definition is quieter there than on Apple and Windows. `game/audio/README.md:153-154` and `:175` name each smoothing; neither says they differ.

## The failure

Tennis's racket hit (`game/games/tennis/logic/src/court.rs:12-31`): 70 ms with `attack(0.001)`. Rendered at 48 kHz and started at offset 0 with gains (1, 1), as `Player::sync` does:

| | peak | energy against the authored samples |
|---|---|---|
| authored | 0.934 | |
| native mixer | 0.934 | -1.51 dB |
| web | 0.624 | -3.33 dB |

The web plays the hit 1.8 dB below native, with its peak 3.5 dB lower. A 10 ms and a 20 ms square blip come out 2.6 dB below native.

Expected: the same sound has the same level on every output.

## Verification

The native figures are `Mixer::frame` read in a temporary test in `game/audio/src/apple_tests.rs` (from `game/`, `cargo test -p exact-game-audio --lib <name> -- --nocapture`). The web figures are the same samples rendered by an `OfflineAudioContext` in the pinned Chromium (156.0.8078.4) through the graph `web.rs` builds: buffer source, two gains at 0, channel merger, then `setTargetAtTime(1, currentTime, 0.01)`. The rendered curve matched `1 - e^(-t/0.01)` to 3e-7. No game was run in a browser.

## Direction

Smooth the web gain with a 10 ms linear ramp from its current value instead of `setTargetAtTime`. In the same Chromium render, `setValueAtTime(value, now)` followed by `linearRampToValueAtTime(gain, now + 0.01)` came within 0.03 dB of the native mixer for both sounds.

## Related

No issue or `QUEUE.md` entry covers gain smoothing.
