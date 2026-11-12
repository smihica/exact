# Deleting the only word of a bold or struck line leaves markers that become a rule or an open code fence

**Status:** Closed
**Resolution:** Deleting the contents of an inline pair removes its markers without creating a rule or fence or changing later formatting.
**Systems:** markdown/editor/src/editor.rs, markdown/src/block.rs
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

After an edit, `Editor::simulate` removes syntax that was hidden before and is visible after,
such as half a bold pair (`markdown/editor/src/editor.rs:944` at `dd6f5dcd`). Its test is only
whether the code unit is hidden in the new text (`:966` to `:973`).

When a line holds nothing but one inline pair, deleting its last character leaves the two
markers alone on the line: `****`, `____`, `******`, `~~~~` or four backticks. The block pass
reads the first three as a thematic break (`markdown/src/block.rs:137`) and the last two as an
opening fence (`block.rs:197`). Both are hidden line syntax, so the leftover markers count as
still hidden and stay.

## The failure

In the web editor, in Chromium, on an empty line: ⌘B, type `hi`, Backspace twice. The source
is `****` and the line is drawn as a horizontal rule. A letter typed next gives `****x`, with
the asterisks showing; a third Backspace removes the rule. Typing `**hi**` and deleting the
word does the same.

With strikethrough the leftover opens a fence:

```
intro

~~x~~

next paragraph
```

Click after `x` and press Backspace: the source is `intro\n\n~~~~\n\nnext paragraph`, the
line shows `~~~~`, and everything below is drawn as code.

Expected: the emptied pair goes, as it does for `*a*`, `` `c` ``, `x **b** y`, `- **b**` and
`# ~~t~~`.

## Verification

Mounted `host/web/markup-editor.js` with its wasm in headless Chromium, as
`host/web/tests/markup-editor.test.mjs` does, and typed the two sequences above: the line's
class was `md-rule` after the first, and `md-code` on the lines below the second. A temporary
test of `exact-markdown-editor` gave a `Rule` paragraph for `**b**`, `__b__`, `***c***` and
`> **b**` after Backspace and for a cut of the word in `**word**`, and `Fence` for `~~s~~`
and a double-backtick span.

## Direction

Count a surviving marker as an orphan when the run that hides it afterwards is line syntax
and the run that hid it before was inline, or remove an inline pair whose contents an edit
emptied before restyling.

## Related

Not in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`. `exact_markdown::edit` takes back an
empty pair for a repeated command (`markdown/src/edit.rs:195`); the deletion path has no such
rule.
