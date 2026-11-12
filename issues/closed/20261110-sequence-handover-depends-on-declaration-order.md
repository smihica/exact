# Handing over from one sequence to another in one commit plays nothing when the new one is declared first

**Status:** Closed
**Resolution:** Rust and JavaScript stop the owner whose gate is false before arming another sequence, so a handover works in either declaration order.
**Systems:** runner/src/transport.rs, host/web-js/transport.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`Transport::stood` applies a commit's gate readings one sequence at a time, in plan order (`runner/src/transport.rs:784-819`, at `dd6f5dcd`). A sequence whose gate rises while another is the owner is journaled and not armed (`:809-817`). Its gate is recorded as on (`:785`), so it does not arm later either: arming needs a rise.

When one commit turns the playing sequence's gate off and another's on, the result depends on which was declared first. If the stopping sequence comes first, it stops and the other arms. If the arming sequence comes first, it is refused because the other is still the owner at that point in the loop, and then the owner stops. Nothing plays, and nothing will until the gate goes false and true again.

LLP 1125 D2 says a second sequence arming "while one plays" does not play, D4 that a gate turning false stops its sequence at that commit's time, and D2 that a sequence arms from the commit that made its gate true. At that commit's time the first no longer plays. No text orders the two by declaration.

## The failure

```
component Seq
  state pattern = "a"
  action swap
    pattern = pattern == "a" ? "b" : "a"
  sequence a when pattern == "a"
    play([Track(beats=1, loops=0, offset=0, output="", steps=[hit(0, "assets/kick.wav")])], bpm=60)
  sequence b when pattern == "b"
    play([Track(beats=1, loops=0, offset=0, output="", steps=[hit(0, "assets/hat.wav")])], bpm=60)
```

Run `swap` twice. Observed (`Transport::playing`, and the journal):

```
boot (pattern a):            playing=true
    sequence a starts at beat 0, bpm 60
swap to b (declared second): playing=true
    sequence a stopped (gate)
    sequence b starts at beat 0, bpm 60
swap to a (declared first):  playing=false
    sequence a did not play: sequence b is playing
    sequence b stopped (gate)
four seconds later:          playing=false
```

Expected on the second swap: `sequence b stopped (gate)`, then `sequence a starts at beat 0, bpm 60`, as the first swap does in the other direction.

## Verification

A temporary test in `runner/src/transport/tests.rs` ran the component above through `Runner::act` and printed the lines above. The JS target does the same: `host/web-js/transport.js` `stood` (line 348), driven under Bun through `rt.js`'s `commit` with the same two sequences, journaled the same lines and ended `state=stopped reason="gate"`.

With the owner whose gate reads false stopped before the loop in `Transport::stood`, the second swap printed `sequence b stopped (gate)` then `sequence a starts at beat 0, bpm 60`, and the runner's other transport tests passed. That change was not made in `transport.js`, and conformance was not run.

## Direction

In `stood`, stop the owner whose gate turned false before looking at any rising gate, then arm, restart or edit in plan order. Make the same change in `transport.js`.

## Related

`a_second_sequence_does_not_play_while_one_does` (`runner/src/transport/tests.rs:498`) covers a second gate rising while the first keeps playing, which stays as it is. `docs/recipes/sequencer.md:78` advises one sequence with a `key=` for a mode; nothing refuses two. No issue or QUEUE entry covers this.
