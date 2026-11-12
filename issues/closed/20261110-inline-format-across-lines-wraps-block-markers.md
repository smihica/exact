# Bold, italic, code or strike wraps a line's list, quote or heading marker when the selection takes the line from its start

**Status:** Closed
**Resolution:** Inline formatting wraps each selected line's content after its block prefix.
**Systems:** markdown/src/edit.rs, markdown/editor, host/web/markup-editor.js, host/apple/src/markup.rs
**Severity:** P1
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

With a selection, `inline` puts the markers round the trimmed text of each selected line
(`markdown/src/edit.rs:207` to `:221` at `dd6f5dcd`). It trims white space only. A line the
selection takes from its first column still has its block syntax in the piece, so the opening
marker goes before the `- `, `1. `, `- [ ] `, `> ` or `# `, and the line stops being that
block.

Every line after the first of a selection that crosses lines is taken from its first column.
On the web the first is too: the editor hides a line's marker, and a selection that begins at
the line's first visible character begins at the line's first source offset
(`Editor::select`, `markdown/editor/src/editor.rs:559`, keeps a range's start there).
`Editor::command` passes the selection on unchanged (`editor.rs:766`). The Apple hosts run the
same function over the text view's selection (`host/apple/src/markup.rs:354`).

## The failure

In the web editor, in Chromium:

| source | selection | after ⌘B |
| --- | --- | --- |
| `# Title` | double-click the word, or triple-click | `**# Title**` |
| `- one two` | triple-click | `**- one two**` |
| `- [ ] task` | drag over the text | `**- [ ] task**` |
| `- one\n- two` | drag from inside `one` to inside `two` | `- o**ne**\n**- tw**o` |
| `# Title\n\ntext` | ⌘A | `**# Title**\n\n**text**` |

Each wrapped line loses its block and shows its marker as bold text. Expected `# **Title**`, `- **one two**`, `- [ ] **task**` and `- o**ne**\n- **tw**o`.

The shared function alone:

```rust
exact_markdown::edit("- one\n- two", Range::new(2, 11), Command::Bold).apply("- one\n- two")
// "- **one**\n**- two**"
```

## Verification

Mounted `host/web/markup-editor.js` with its wasm in headless Chromium, as
`host/web/tests/markup-editor.test.mjs` does, selected with the mouse and pressed ⌘B: the
table above, and the same for `> quoted` and `1. one\n2. two`. A temporary test of
`exact-markdown-editor` gave the same sources through `Editor::select` and
`Editor::command`, with italic and strike too; Undo restored each. Not run on an Apple host.

## Direction

For each selected line, start the wrap at the line's content: after
`prefix(source, line).content` (`edit.rs:148`) when the selection begins at or before it.
`blocks` and `newline` already read a line's syntax that way.

## Related

`issues/closed/20261006-markdown-editor-native-dom-reconciliation.md` concerns the web host's
DOM after a multi-line replacement, not the replacement's text. QUEUE's "Markdown editor"
entry lists nothing about a command over a line's marker.
