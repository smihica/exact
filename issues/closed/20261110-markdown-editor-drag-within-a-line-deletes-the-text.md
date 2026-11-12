# Dragging selected text to another place in the same line of a web Markdown editor deletes it

**Status:** Closed
**Resolution:** Dragging text within a Markdown editor preserves the drop target and commits the move as one undoable edit.
**Systems:** host/web/markup-editor.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`.

A browser moves dragged text in an editable element with two inputs: `deleteByDrag` at the source, then
`insertFromDrop` at the drop (Input Events Level 2, "Input type"). The editor takes the first itself: `kindOf`
maps `deleteByDrag` to a delete over the event's target range (`host/web/markup-editor.js:16`), and `beforeInput`
cancels the browser's own delete, applies it in the shared editor and redraws (`:589-602`, then `after` and
`render`). `render` builds a new element for each line whose record changed and removes the old one
(`:203-212`).

Chromium finds the drop's element and position before it sends `deleteByDrag`. When the drop is in the line
the text came from, that element is the one `render` just removed, and Chromium sends no `insertFromDrop` to a
target that has left the document. The text is deleted and never inserted.

A line here is a source line, so a paragraph that wraps onto several rows is one line: a drag within a
paragraph or a list item loses the text. The shared editor's rules are not involved; the delete they apply is
correct, and the insert never reaches them.

## The failure

A `textarea markup="markdown"` holding `alpha beta gamma delta`; select `beta` and drag it with the mouse to
the end of the line. In the pinned Chromium, beside a plain `textarea` given the same drag:

```
textarea: value "alpha gamma deltabeta"  beforeinput: deleteByDrag, insertFromDrop
markdown: value "alpha  gamma delta"     drop, then beforeinput: deleteByDrag only; the line's element is detached
```

The same happens for `beta` in `alpha **beta** gamma delta` (the result has no `**beta**`), for a word in
`- alpha beta gamma delta`, and in a 260 px wide editor for a word dragged from the first row of one paragraph
to its second. A drag to another line works, with both inputs heard. Undo (⌘Z) puts the text back.

Expected: the text moves, as in a plain field. LLP 1045 lists drop among the editor's edits.

## Verification

Run in the pinned headless Chromium (156) with Playwright's mouse (press on the selection, move, release),
which starts Chromium's own drag of the selection, against the real `markup-editor.js` and
`markup-editor.wasm`; listeners on the document's capture phase recorded the events above. The cause was
checked without the editor: a bare `contenteditable` whose `beforeinput` cancels `deleteByDrag` and replaces
the line's element hears no `insertFromDrop`; one that edits the text node in place hears it. Not run:
Firefox, WebKit, a windowed Chrome.

## Direction

Keep the source line's element in the document between the two inputs: on `deleteByDrag` cancel the browser's
delete and hold its source range without redrawing, and when `insertFromDrop` follows, map the drop point
first, then apply the delete and the insert as one edit. Apply the held delete alone when no insert follows.

## Related

`issues/closed/20261006-markdown-editor-native-dom-reconciliation.md` is multi-line replacement. The open
editor issues `20261110-inline-format-across-lines-wraps-block-markers`,
`20261110-return-in-formatted-item-misplaces-caret`, `20261110-emptied-bold-line-becomes-rule-or-fence` and
`20261110-indent-ignores-quote-prefix` are rules of the shared crate; this is the web host's event handling.
