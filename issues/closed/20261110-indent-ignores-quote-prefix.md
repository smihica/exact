# Indent does not nest a quoted list item, and a second Indent ends its quote

**Status:** Closed
**Resolution:** Indent and Outdent insert and remove the two spaces after a line's `>` markers (`block::quote_prefix`'s offset), so a quoted item nests and unnests inside its quote.
**Systems:** markdown/src/edit.rs, host/web/markup-editor.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`indent` adds two spaces at the start of each selected line and removes them from there on
Outdent (`markdown/src/edit.rs:402` to `:423` at `dd6f5dcd`). In a block quote the line starts
with the quote markers, and the list's indent is after them. Two spaces before `>` change
nothing: `quote_prefix` allows up to three (`markdown/src/block.rs:102`), and the item's depth
is counted after the marker. Two more make four, which is no longer a quote marker, and the
line becomes text.

The other block commands place their syntax after the quote markers through `prefix`
(`edit.rs:148`); `indent` is the one that does not.

The web host sends `indent` for Tab in a list item and for ⌘] (`host/web/markup-editor.js:638`,
`:634`).

## The failure

In the web editor, in Chromium, with the source

```
> - a
> - b
```

click after `b` and press Tab. The source becomes `> - a\n  > - b` and nothing moves: both
lines are still bullets at depth 0 in the quote. Press Tab again: `> - a\n    > - b`, and the
second line shows `    > - b` as text.

Expected after one Tab: `> - a\n>   - b`, a bullet at depth 1 in the quote. Outdent has the
mirror fault: Shift-Tab on `>   - b` finds no leading spaces and does nothing.

## Verification

Mounted `host/web/markup-editor.js` with its wasm in headless Chromium, as
`host/web/tests/markup-editor.test.mjs` does, and pressed the keys above; the sources and
drawn lines are as stated. A temporary test of `exact-markdown-editor` gave the same sources
through `Editor::command`, with the styler's paragraphs `Bullet` depth 0 quote 1 twice after
one Indent and a single `Bullet` after two; Tab in an unquoted list gave `- a\n  - b`, depth
1. Not run on an Apple host.

## Direction

Insert and remove the indent after the line's quote markers (`block::quote_prefix`'s offset),
as `prefix` does for a block marker.

## Related

Not in `issues/` or `QUEUE.md`. `markdown/tests/markdown.rs` tests Indent and Outdent on
unquoted lines only.
