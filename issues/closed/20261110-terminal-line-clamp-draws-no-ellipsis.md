# In the terminal a clamped paragraph ends with no ellipsis

**Status:** Closed
**Resolution:** The terminal marks clamped paragraphs and native button titles with an ellipsis on whole grapheme clusters, trimming long lines in linear time.
**Systems:** host/terminal/src/paint.rs (`text`)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`. `line-clamp=N` keeps a paragraph's first N lines and marks the cut
with an ellipsis at the end of the last one: CSS Overflow 4's `line-clamp` sets
`block-ellipsis: auto`
([`line-clamp`](https://drafts.csswg.org/css-overflow-4/#line-clamp)), and the
Linux host shapes one for any `line_clamp > 0` (`host/linux/src/text/flow.rs:230`,
`host/linux/src/text/shaping.rs:840`). LLP 1101 §4 admits `line-clamp` in the
terminal beside `text-overflow: ellipsis`, "on whole cells; `…` is one column".

The terminal's painter draws `…` only for `text-overflow: ellipsis` on a line wider
than its box (`host/terminal/src/paint.rs:707-717`). `wrap` drops the lines past
the clamp (`:701`), and nothing marks the last kept line. A reader cannot tell a
clamped paragraph from a complete one.

## The failure

A 16-column box, 20 by 6 screen:

```
column width="16ch"
  text "one two three four five six seven eight nine ten" line-clamp=2
  text "----------------"
  text "one two three four five six seven eight nine ten" white-space="nowrap" overflow="hidden" text-overflow="ellipsis"
```

Observed:

```
one two three
four five six
----------------
one two three f…
```

Expected: the second row ends in `…` (`four five six…`), as the fourth does.

## Verification

A temporary test in `host/terminal/tests/it/screen.rs` booted the app in
`Mode::Fullscreen`, rendered into the `Vt` emulator and printed its text
(`cargo test -p exact-terminal --test it`). It printed the screen above. The Linux
host's ellipsis was read, not run.

## Direction

When `wrap` dropped lines for the clamp, end the last kept line with `…`, making
room for it as the `text-overflow` branch does.

## Related

`20261110-terminal-table-skips-the-space-rule.md` is the terminal's table.
`20261110-count-button-shortcut-reuses-wrapped-height.md` mentions `line-clamp` on
another host. `QUEUE.md`'s "`line-clamp: auto`" is a value the kernel lacks, not
this ellipsis.
