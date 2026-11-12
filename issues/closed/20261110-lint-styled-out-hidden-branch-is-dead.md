# The layout lint's doc says a `display: none` button is refused; the code skips it, `styled_out`'s `hidden` test is dead, and its test asserts no refusal

**Status:** Closed
**Resolution:** The layout lint documents hidden controls as skipped, checks only explicit zero dimensions after that skip, and tests check and bake refusals for zero width and height.
**Systems:** contract/cli/src/lib.rs, contract/cli/tests/it/lint.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`lint` (`contract/cli/src/lib.rs:1344-1452`) refuses a pressable with zero area. Its doc
comment describes the unanswered check (`contract::check`, the web build's) this way
(`lib.rs:1338-1343`):

> What stays is a pressable whose zero area is its own style's — `display: none` on it or an
> ancestor, a zero `width` or `height` — which no answer changes (files diary F13: a hidden
> shortcut button).

The code does the opposite for `display: none`. The pressable arm begins

```rust
if hidden(kernel, node) {
    continue;
}
```

(`lib.rs:1417-1419`), answered or not, so a hidden pressable is never refused. That is the
intended behaviour: the comment on the test says "Hidden controls and descendants have no
pressable layout until shown" (`contract/cli/tests/it/lint.rs:400-402`).

Three things are left over:

1. The doc comment tells a reader that the hidden shortcut button is refused.
2. `styled_out` (`lib.rs:1467-1475`), called only at `lib.rs:1423` after that skip, ends with
   `hidden(kernel, node)`, which cannot be true there. What it decides is a zero `width` or
   `height` alone.
3. `check_refuses_without_data_what_no_answer_could_change` (`lint.rs:403-417`) asserts three
   acceptances and no refusal, so the one refusal `check` still gives unanswered, a zero
   `width` or `height`, has no test.

## The failure

`button "Down" press=go … testId="down"` in a `column`, through `contract::check` and
`contract::bake`:

| the button | `check` | `bake` |
|---|---|---|
| `display="none"` | passes | passes |
| under `column display="none"` | passes | passes |
| `width=0 padding=0 border-width=0` | `bake-zero-size` | `bake-zero-size` |

The doc comment predicts a refusal in the first two rows of the `check` column.

## Verification

A temporary unit test ran the three sources through `contract::check` and `contract::bake`
(a data source that answers nothing) and printed the table above. That the `hidden` call in
`styled_out` is dead follows from the skip: `hidden` reads only the kernel and the node, and
nothing changes between the two calls.

## Direction

Say in the doc comment that a hidden pressable is skipped and that only a zero `width` or
`height` of its own is refused unanswered, reduce `styled_out` to that test, and assert the
zero-`width` refusal in the test whose name promises one.

## Related

None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
