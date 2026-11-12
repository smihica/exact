# `contract verify` reports a divergence for any app that scrolls a list row into view, so `verify` fails on the chat-thread recipe

**Status:** Closed
**Resolution:** The semantics excludes runner-consumed list-row scrolls from host commands while preserving element scrolls, so contract verify agrees on the chat-thread recipe.
**Systems:** semantics/Contract/Runtime.lean (`hostCommands`), semantics/difftest/src/observe.rs, runner/src/runner/commit.rs, semantics/README.md, docs/recipes/chat-thread.md
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

The comparison prints "the commands issued to a host (not `setMode`, which the runner takes
and applies itself)" (`semantics/README.md:64-65`). The row form of `scrollIntoView`,
`scrollIntoView(list, key, block=…)`, is such a command: the runner takes the seven-argument
form out of its commands and runs it in the commit, and it never reaches a host
(`runner/src/runner/commit.rs:722-736`; LLP 1070.000 §10 and §12). The Rust side prints what
`Runner::take_commands` returns (`semantics/difftest/src/observe.rs:178`), so it has no line
for it. The Lean side's `hostCommands` leaves out `setMode` alone
(`semantics/Contract/Runtime.lean:889-894`) and prints it. The two differ at the first
commit that scrolls a row into view, and `contract verify` exits 1.

`bun exact.mjs verify` runs `contract verify` and exits 1 with it.
`docs/start-here.md:36-37` sends every chat screen to `docs/recipes/chat-thread.md`, whose
`toEnd` action is `scrollIntoView("transcript", lastId, block="end")` (line 30).

## The failure

```contract
component App
  state items = ["a", "b", "c"]
  action go
    scrollIntoView("rows", "c", block="end")
  view
    column
      button press=go testId="go"
        text "Go"
      list id="rows" virtualized=true height=100
        for i in items key=i
          text i
```

with `app.test.contract` holding `test "the list scrolls to its last row"` and `tap "go"`:

```
$ bun exact.mjs contract verify app.contract
DIVERGE app.test.contract: the list scrolls to its last row
  at step 1: tap "go"
  runner: view "go" -
  lean:   command scrollIntoView "rows" "c" "end" none none none none
  semantics: agreed on 0 of 4 scripts, …                                  (exit 1)
```

With `scrollIntoView("go")`, the element form, which a host runs, the same app agrees on 4
of 4 scripts and exits 0. The chat-thread recipe's three blocks, in an app `exact new` made:

```
$ bun exact.mjs verify
tests: 1 passed, 0 failed
contract verify: failed (exit 1); …
DIVERGE app.test.contract: a sent message lands; the composer clears
  at step 4: land
  runner: view "composer" -
  lean:   command scrollIntoView "transcript" "<the oracle's id>" "end" none none none none
verify: clean in 6.1 s; contract verify failed (exit 1), as above       (exit 1)
```

Expected: the two sides agree, and `verify` on the recipe exits 0.

## Verification

Ran both commands on a Mac with Lean installed. Without `lake` the check runs the runner
alone (`semantics/difftest/src/verify.rs:302`) and does not see this. No file under
`semantics/corpus` names `scrollIntoView`, so `difftest quick` does not reach it.
`apps/shared-elements` uses the row form in `closeRow`, which its walks never reached; it
agreed on 3 of 3. Not run: the JS comparison (`--js`).

## Direction

Have `hostCommands` leave out a seven-argument `scrollIntoView` as it leaves out `setMode`,
say so in the README's sentence, and add a corpus case.

## Related

`issues/20261110-expansion-check-diverges-on-every-commit.md` is `difftest expansion`'s
`ran` lines. `issues/20261110-js-driver-loses-linked-commands-and-reads-no-select-value.md`
is the JS driver. `QUEUE.md:161` is host operations the oracle has no answer for. None is
this command.
