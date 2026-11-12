# A followed MIDI clock's tempo has no bound, so ticks stamped microseconds apart expand a fill without limit

**Status:** Closed
**Resolution:** Followed clocks retain their prior or last valid estimate above 2,500 BPM, and every fill drops tracks exceeding 1,000 loops per second at its actual tempo.
**Systems:** runner/src/transport/clock.rs, runner/src/transport.rs, host/web-js/clockin.js, host/web-js/transport.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

LLP 1125 drops a track that "would loop more than 1,000 times a second at its tempo: the bound that keeps a fill's work finite". `Play::read` checks that bound against the `bpm=` argument (`runner/src/transport.rs:285`). A sequence with `clock=` plays at the window's estimate instead, `60000 * (n - 1) / (elapsed * 24)` (`runner/src/transport/clock.rs:183`), taken once 13 ticks are in the window and `elapsed > 0`; `bpm=` is only the prior. Nothing bounds the estimate, and no host spaces the stamps: the Apple arm passes each packet's `MIDITimeStamp` on (`host/apple/midiarm/MIDIArm.swift:261`), the web's `event.timeStamp` (`host/web/midi-glue.js:85`).

`Transport::follow` then fills to 100 ms past the delivery, and `Play::hits` (`transport.rs:412`) builds and sorts every hit of every loop in that window before the fill's limits apply. A note due within the 50 ms hold takes no place in the MIDI queue (`runner/src/midi.rs:960`), so those are all issued at once.

A gap of 500 ms empties the window, so the trigger is 13 or more ticks stamped close together after a silence: a sender that stalls, then sends what it owes with each tick stamped as it is sent.

## The failure

`runner/src/transport/tests/clock.rs`'s fixture (one 1-beat track, one kick with a note) follows 120 bpm for 2 s, hears nothing for 600 ms, then takes one delivery of 29 ticks `gap` apart ending at the clock's time. Optimized build (`--profile host-dev`):

| gap | estimate, bpm | the delivery | the next 200 ms of clock | MIDI messages issued |
|---|---|---|---|---|
| 20 ms | 120.5 | 17 µs | under 1 µs | 2 |
| 0.1 ms | 25,000 | 84 µs | under 1 µs | 84 |
| 5 µs | 500,000 | 4.2 ms | under 1 µs | 1,668 |
| 1 µs | 2,500,000 | 19 ms | 43 ms | 8,334 |
| 83 ns | 30,120,482 | 123 ms | 4.5 s | 100,402 |

The test's clock is a seek, which runs each of the transport's wakes at its due time. Expected: work and output bounded as a `bpm=` tempo's are, whatever the stamps.

## Verification

- The table is what a temporary test in `runner/src/transport/tests/clock.rs` printed under `cargo test --profile host-dev -p exact-runner --lib`. Start and 16 ticks flushed the same way give the same estimates; under `--release` that delivery took 14 ms at 1 µs, 150 ms at 42 ns and 7.2 s at 1 ns.
- CoreMIDI delivers such stamps. A Swift program sent 16 Timing Clock messages from a virtual source to an input port in one loop, each stamped `mach_absolute_time()` as it was sent. Over eight sends the read block saw them span 6 to 42 µs (an estimate of about 0.9 to 6 million bpm), with stamps as little as one Mach tick (41 ns) apart. Sent with stamp 0 they arrive as one packet, which the estimator already passes over.
- Not run: a MIDI arm end to end, and the JS target, whose `clockin.js:31` is the same estimator and `transport.js:85` the same `bpm=` check.

## Direction

Bound the tempo a followed clock may say, in `Follow::stamp` and `clockin.js` alike: an estimate past clock out's bound (`TICKS_PER_SECOND`, 2,500 bpm) leaves the previous estimate or the prior standing. Then check each track's loop rate against the followed tempo where a segment starts.

## Related

`issues/closed/20260901-clock-seek-unbounded-work.md` bounded a seek of the runner's own clock; this is the MIDI clock a sequence follows. Nothing in `QUEUE.md` or `docs/agent-pitfalls.md` names it.
