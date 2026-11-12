# A computed animation longhand beside an `animation` list silently drops every animation after the first

**Status:** Closed
**Resolution:** A literal animation shorthand list beside a computed longhand is refused with lower-animation-longhand instead of losing later animations.
**Systems:** contract/lower/src/svg.rs (`compose_animation`)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`compose_animation` (`contract/lower/src/svg.rs:650`) folds `animation` and its longhands into one `animation` row. When any longhand is computed it builds a single animation from a template, taking its defaults from the first entry of the literal shorthand:

```rust
let defaults = base.into_iter().next().unwrap_or_default();   // svg.rs:732
```

The rest of `base` is discarded. The comment above that line says "a literal list cannot be combined with a computed part", and a literal longhand that is a list is refused in this branch (`svg.rs:739-741`). The shorthand's own list is not checked, so its second and later animations vanish with no diagnostic.

## The failure

```
keyframes spin
  to rotate="360deg"
keyframes fade
  to opacity=0
component A
  state frozen = false
  view
    column testId="a" animation="spin 1s linear infinite, fade 2s" animation-play-state=(frozen ? "paused" : "running")
```

Observed: the node's `animation` row holds one animation, `spin`. `fade` never runs.

Expected: both animations, each following the play state (CSS Animations 1 §4: a longhand's list shorter than `animation-name`'s repeats, so the one computed value applies to both), or the refusal the longhand list already gets. With a literal `animation-play-state="paused"` the same shorthand keeps both animations, paused.

Toggling `animation-play-state` from state is the usual reason to write a longhand beside the shorthand; `contract/cli/tests/it/svg.rs:107` covers it with one animation.

## Verification

At `dd6f5dcd`, a temporary test in `contract/cli/tests` compiled, baked and booted the component above (with `state d = "3s"` added) and printed the node's `animation` list for each set of attributes:

| Attributes | Animations on the node |
| --- | --- |
| `animation="spin 1s linear infinite, fade 2s"` | `spin` 1 s infinite, `fade` 2 s |
| the same with `animation-play-state="paused"` | `spin` paused, `fade` paused |
| the same with `animation-play-state=(frozen ? "paused" : "running")` | `spin` |
| the same with `animation-duration=d` | `spin` 3 s |
| `animation-name="spin, fade" animation-duration=d` | refused: ``[lower-animation-longhand] `animation-name` is a list, and another part is computed: a computed animation is one animation`` |

## Direction

In the computed branch, refuse a shorthand whose parsed list has more than one animation, with the message the longhand list gets; or build one template per animation of `base` and join them with commas, so the computed part applies to each.

## Related

No issue or `QUEUE.md` entry is about `animation` longhands, and `docs/agent-pitfalls.md` does not mention the combination. LLP 1055 D5 requires a computed longhand to be a single value; it does not say a list beside it is dropped.
