# On the Mac ⌘A selects the first virtualized list's text wherever the focus is

**Status:** Closed
**Resolution:** On macOS, Select All follows the current list or paragraph context in the receiving window; prose selects that window's mounted paragraphs, and only a reader without paragraph context falls back to its sole list.
**Systems:** host/apple/Sources/ExactKit/Mac/TextSelectionMac.swift
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

`TextSelection.selectAll` (`host/apple/Sources/ExactKit/Mac/TextSelectionMac.swift:753-771`, at `dd6f5dcd`) looks first for any selectable paragraph in a virtualized list's row, anywhere on the page:

```swift
let nodes = paragraphs
if let first = nodes.first(where: { position($0, offset: 0) != nil }),
   let (owner, start) = position(first, offset: 0), ... {
    list = owner; logicalAnchor = start; logicalFocus = end; allListText = true
    ...
    return
}
```

When there is one, the selection is that list's and nothing else: `spanned` (`:812-824`) answers nil for every paragraph outside `list`, and `selectedText` (`:780-783`) returns the list's text. Which paragraph has the focus, and where the person last pressed, is never read. Only a page with no such list gets its paragraphs selected (`:766-770`).

## The failure

```
column
  article
    text "Release notes"
    text "Long prose the person is reading."
  list virtualized=true height=140 estimated-item-height=28
    for n in [1, 2, 3, 4, 5] key=n
      text `comment ${n} body`
  text "Between the lists."
  list virtualized=true height=140 estimated-item-height=28
    for n in [1, 2, 3, 4, 5] key=n
      text `reply ${n} body`
```

Click in a reply, press ⌘A, then ⌘C. Observed: the comments are highlighted and copied. The replies, where the click was, are not, and no ⌘A ever selects them. A click in the prose does the same, leaving out the paragraph that has the focus, also with one list on the page.

Expected: ⌘A acts where the person is. `TextSelectable.swift:34` says "⌘A selects what this admits", and every paragraph here is admitted. The wasm web target's rule for the same logical selection (`host/web/list-selection.js:111-112`) takes the list the selection is in, then the list last pressed, and the page's only list last. The JS web target leaves ⌘A to the browser, which selects the page.

## Verification

A temporary XCTest booted that page in a session, clicked a paragraph (mouse down and up, which makes it first responder), and sent ⌘A and ⌘C with `NSWindow.performKeyEquivalent`. For a click in the prose, in `reply 2 body`, in `Between the lists.` and in `comment 2 body` alike, the only selected ranges were the first list's paragraphs and the pasteboard held `comment 1 body` to `comment 5 body`. A page with the article and one list gave the same after a click in the prose.

## Direction

Choose the list as the web's rule does: the one that holds the focused or last pressed paragraph, then the page's only list, so a reader whose document is one list keeps its ⌘A. A press outside every list is the case to rule: the wasm target still takes the only list, the browser takes the page's text. Selecting a list and the text around it at once needs the list's unmounted rows in the page's order, which is a larger change.

## Related

`issues/closed/20261013-macos-select-all-copies-hidden-text.md` fixed which paragraphs ⌘A admits, not which scope it takes. `TextSelectionMacTests.testACopyAcrossListRowsLeavesOutUnselectableText` ends with ⌘A on a page of five lists after a drag in the first, so it passes either way. `QUEUE.md`'s "The Mac's text and scrolling, what is owed" does not list this.
