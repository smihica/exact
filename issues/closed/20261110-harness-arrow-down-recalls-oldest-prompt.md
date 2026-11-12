# In the harness prompt, ArrowDown from the newest recalled entry shows the oldest instead of the empty draft

**Status:** Closed
**Resolution:** ArrowDown at the newest recalled entry empties the draft, rather than reading `history.at(-1)`, the oldest.
**Systems:** apps/harness (terminal.contract)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

The prompt's history is newest first (`history = [text].concat(history)`, `apps/harness/terminal.contract:155` at `dd6f5dcd`), and `back` is the index of the entry shown, -1 for the draft. ArrowDown reads the entry one step newer (`terminal.contract:193-195`):

```
if key == "ArrowDown" and back >= 0 and not draft.includes("\n")
  draft = match history.at(back - 1) { case some(h) => h, case none => "" }
  back = back - 1
```

At `back == 0` this is `history.at(-1)`. A list's `at` counts a negative index from the end (`docs/contract-grammar.md:1051`: "`[4, 5, 6].at(-1)` is `some(6)`"), so the `none` arm that was meant to empty the draft never runs. The draft becomes the last entry of the list, which is the oldest prompt, and `back` goes to -1. With one entry in the history, ArrowDown leaves that entry in the prompt.

## The failure

Send `/stress 1`, then `/stress 2` (each has a space, so the command completions stay closed). Press ArrowUp, then ArrowDown.

- Observed: ArrowUp shows `/stress 2`; ArrowDown shows `/stress 1`.
- Expected: ArrowDown returns to the empty prompt, which is what the `none` arm writes and what a shell's history does.

Enter then sends the oldest prompt again.

## Verification

Drove the real screen over an offline session: `Host::boot` with `terminal.contract` and `Harness::with_options(Options::offline(None))`, the keys sent as the command line's `press` and `key` send them, the prompt's row read from the terminal emulator. It printed

```
ArrowUp:           "│ › /stress 2 …
ArrowDown:         "│ › /stress 1 …
```

## Direction

Test the index before reading: at `back == 0` the draft is `""`, otherwise `history.at(back - 1)`.

## Related

No issue or QUEUE entry names the harness's history, and `docs/agent-pitfalls.md` has no entry for `at` with an index that can reach -1.
