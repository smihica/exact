# A grouped drag's edge autoscroll nearly stops whenever the pointer is moving

**Status:** Closed
**Resolution:** Grouped edge scrolling keeps its frame clock across pointer moves and restarts it after leaving the scrollable bands.
**Systems:** host/linux/src/presenter/group.rs (the painting host: Linux, Windows, Android)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A grouped drag (LLP 1094 D7) scrolls a list or its board while the ghost is in an edge
band. Each frame `group_edges` advances the scroll by the speed times the time since
`edge_clock` (`host/linux/src/presenter/group.rs:547` to `:549`):

```rust
let dt = ((now - s.edge_clock) / 1000.).clamp(0., 0.05) as f32;
s.edge_clock = now;
```

But `move_group` also writes that clock on every pointer move (`group.rs:188`:
`s.edge_clock = now;`). So a frame counts only the time since the last move, not since the
last frame. A finger on glass or a mouse being aimed sends a move every frame or more, and
the scroll runs at full speed only while the pointer is perfectly still. On Android it does
not run at all in a frame that follows a move: the loop stamps each queued touch and then
the tick from the same clock, microseconds apart (`host/linux/src/android.rs:514`, `:547`,
`:559`).

The other implementations of the rule keep their clock across moves. The in-list drag stops
it only when the row leaves the band (`host/linux/src/presenter/arrange.rs:197` to `:203`,
`:462` to `:467`). The web's pump keeps `pumpTime` from frame to frame
(`host/web/group-glue.js:112` to `:127`). Apple's `updateEdge` runs on every move but
leaves a running timer and its `edgeTime` alone
(`host/apple/Sources/ExactKit/ReorderGroup.swift:335` to `:354`). D7 says "The pump runs
while any candidate is in a band."

## The failure

The kanban fixture of `presenter/arrange_tests.rs` (`BOARD`), a card carried into the
board's right edge band and held there for twelve 60 Hz frames (0.2 s), with a one-point
move before each frame:

```rust
carry(&mut p, "a2", (r.0 + r.2 - 70., r.1 + 30.), 0.);
p.pointer_move(x, y, 30.).unwrap();          // x = the board's right edge - 8
for n in 1..=12 {
    let at = 30. + n as f64 * 1000. / 60.;
    if let Some(lead) = lead { p.pointer_move(x + (n % 2) as f32, y, at - lead).unwrap(); }
    p.tick(at);
}
```

| the pointer | board scrolled |
| --- | --- |
| still | 40.0 (200 points a second, the band's speed there) |
| a move 8 ms before each frame | 19.8 |
| a move 1 ms before each frame | 2.5 |
| a move at each frame's own time | 0.0 |

Expected: 40 in every row, to within the one-point change of position in the band. The
speed is a function of where the ghost is, not of whether the contact moved.

## Verification

A temporary test appended to `presenter/arrange_tests.rs` ran the loop above for each row
and printed the board's scroll offset: the table. Read, not run: the web glue, the Apple
pump, `arrange.rs` and Android's loop. Not run: a finger on an Android device.

## Direction

Do not write `edge_clock` in `move_group`. Start it when the ghost enters a band and stop
it when it leaves, as `arrange.rs` does with its `Option` clock, so the first frame in a
band still counts nothing.

## Related

`issues/20261110-grouped-reorder-drop-leaves-pointer-hold.md` is the drop. The existing
tests hold the pointer still between ticks
(`scroll_end_at_the_group_boundary_completes_once`), so none sees this. Nothing in
`QUEUE.md` or `docs/agent-pitfalls.md` names a grouped drag's edge scroll.
