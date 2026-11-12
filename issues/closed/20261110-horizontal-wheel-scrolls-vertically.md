# A horizontal wheel report scrolls a terminal app vertically

**Status:** Closed
**Resolution:** The terminal decoder drops horizontal wheel reports and preserves vertical wheel scrolling.
**Systems:** host/terminal (term.rs)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

In SGR mouse reports (xterm's mode 1006) the wheel is buttons 4 and 5, encoded as buttons 1 and 2 with 64 added: 64 up, 65 down. Buttons 6 and 7, the horizontal wheel, are encoded the same way: 66 left, 67 right (xterm control sequences, "Wheel mice" and "Other buttons").

The decoder at `host/terminal/src/term.rs:165-167` (`dd6f5dcd`) tests bit 64 and then only bit 0:

```rust
if b & 64 != 0 {
    self.out.push(Input::Wheel(x, y, if b & 1 == 0 { -3 } else { 3 }));
}
```

So 66 is taken for wheel up and 67 for wheel down. `Input::Wheel` carries rows only, and `Host::wheel` moves the scroller under the pointer by them (`host/terminal/src/pointer.rs:188-201`). In a terminal that reports the horizontal wheel, a sideways swipe scrolls a list up or down three rows per report, and the sideways part of a diagonal swipe adds to or cancels the vertical part.

## The failure

A full-screen `scroll width="20ch" height="5lh"` of thirty `text "line N"` rows, the reports fed through `decode` to `Host::wheel`:

| Report | First row shown |
|---|---|
| (start) | `line 0` |
| `ESC[<65;3;2M` (wheel down) | `line 3` |
| `ESC[<67;3;2M` (wheel right) | `line 6` |
| `ESC[<66;3;2M` (wheel left) | `line 3` |
| `ESC[<66;3;2M` (wheel left) | `line 0` |

Expected: the two horizontal reports move nothing. The host has no horizontal scrolling (`host/terminal/src/collection.rs:96`), so they should be dropped.

## Verification

A temporary test in `host/terminal/tests` booted the scroller with `Host::boot`, decoded each report with `term::decode` and printed the first row of `host.frame().grid.text()` after each: the table above. No real terminal was run; the button codes are xterm's.

## Direction

Emit `Input::Wheel` only when `b & 2 == 0`.

## Related

Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
