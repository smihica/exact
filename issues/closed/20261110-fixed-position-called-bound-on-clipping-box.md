# A literal `position="fixed"` on a box that clips or animates is refused as "a bound `position`"

**Status:** Closed
**Resolution:** Invalid position literals reach the row's diagnostic on boxes that clip, scroll, transform or animate, including the viewport-pinning guidance for fixed.
**Systems:** contract/lower/src/contain.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`contain::positioned` (`contract/lower/src/contain.rs:87-98`) checks the `position` of a box that is a containing block: one that clips, scrolls, transforms or animates. A literal `static` has its own message. Every other value goes to `always_positioned` (142-149), which is true only for `relative`, `absolute`, `sticky` or a choice between them. Anything else is refused as "a bound `position`", the message written for a value from run time. A literal the row does not take lands there too, `fixed` above all, and the row's own check, whose message names the edit, never runs.

## The failure

```contract
component App
  view
    column
      column position="fixed" top=0 transition="opacity 200ms"
        text "x"
```

Observed:

```
4:14 [lower-attr-value] a bound `position` on a box that clips, scrolls, transforms or animates: such a box is the containing block of its absolutely positioned descendants on every host, so every value its position can take must be `relative`, `absolute` or `sticky`
```

Expected, as the same box without `transition` reports:

```
4:14 [lower-attr-value] `position="fixed"` is not a valid `position`: expected one of "relative", "absolute", "static", "sticky"; `fixed` is not a row (LLP 1001): pin a box to the viewport with `absolute`, directly inside a viewport-sized root that does not scroll (its content scrolls in a `scroll` beside it)
```

The position is a literal, not bound, and the first message does not say how to pin the box. A header that clips or a toast that fades is how `position: fixed` usually arrives from the web, so the port meets the worse message.

## Verification

At `dd6f5dcd`, `contract build` on the source above printed the first message, and the second with `transition` removed. A temporary test in `contract/cli/tests` compiled the other forms:

| Box | Message |
| --- | --- |
| `column position="fixed" top=0` | the row's |
| `column position="fixed" top=0 opacity=0.5` | the row's |
| `column position="fixed" top=0 overflow="hidden"` | "a bound `position`" |
| `scroll position="fixed" top=0 height=100` | "a bound `position`" |
| `column position="fixd" top=0 overflow="hidden"` (a misspelling) | "a bound `position`" |
| `column position="absolute" top=0 overflow="hidden"` | compiles |

## Direction

In `positioned`, let a string literal that is none of `static`, `relative`, `absolute` and `sticky` pass (`Ok(None)`), so the row's own check refuses it by name. Keep "a bound `position`" for a value that is not a literal.

## Related

No issue, `QUEUE.md` entry or `docs/agent-pitfalls.md` entry quotes this message. `QUEUE.md`, "What LLP 1074 left", says `position: fixed` itself waits on stacking-context painting; this is only about which refusal the author reads.
