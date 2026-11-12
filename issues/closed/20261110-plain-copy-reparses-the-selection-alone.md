# Copy Plain Text reads the selection as a document of its own, so part of a code block or of inline code loses its `*`, `_` and `#`

**Status:** Closed
**Resolution:** `plain_range` now applies the whole source's skips (hidden markers, fences, atoms, glyphs) to the selected range, as `plain_with` does for the whole, instead of reparsing the fragment with atoms as private-use stand-ins.
**Systems:** markdown/src/segment.rs (`plain_range`), markdown/editor/src/editor.rs, host/web/markup-editor.js, host/apple/Sources/ExactKit (TextAreaMac.swift, TextAreaIOS.swift)
**Severity:** P2
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

At `dd6f5dcd`, `plain_range` (`markdown/src/segment.rs:355-389`) is what the Markdown editor's second copy command runs on every host (⌘⇧C on macOS and the web, "Copy Plain Text" on iOS). It cuts the range out of the source and calls `plain` on the fragment (`segment.rs:379`). `plain` is "the text a reader sees: markers gone" (`segment.rs:346`), but the fragment has lost what decided how its characters read: the fence around it, the backticks, the other half of a `**` pair.

The function already knows this for atoms: it finds them in the whole source, "where a token's surroundings decide it, not in the fragment alone" (`segment.rs:351-354`), and stands each in as a private-use character from U+E000 while the fragment is made plain (`segment.rs:374`). So a U+E000 already in the selection becomes the first atom's label.

The web hands it the copy range, which widens a selection edge that touches hidden syntax to take that syntax (`copy_range`, `markdown/editor/src/editor.rs:797-811`; `host/web/markup-editor.js:541-545`), so all of a code block or all of an inline code span comes out right there. A selection that starts or ends inside the code does not. The Apple hosts hand it `selectedRange()` unwidened (`TextAreaMac.swift:303-305`, `TextAreaIOS.swift:359-362`).

## The failure

In the web editor in Chromium, with the source below, the selection set to the text in the second column, then Mod-Shift-C:

| source | selected | copied | the editor shows |
| --- | --- | --- | --- |
| "```\ntotal = a*b*c\nname = __init__\n```\n" | the second code line | `name = init` | `name = __init__` |
| the same | `a*b*c` | `abc` | `a*b*c` |
| "```\nx = 1\n# a comment\ntotal = a*b*c\n```\n" | the last two code lines | `a comment`, `total = abc` | `# a comment`, `total = a*b*c` |
| "call `x = a*b*c;` here\n" | `a*b*c` | `abc` | `a*b*c` |
| "a **bold word** b\n" | from `word` to the end | `word** b` | `word b` |

Code is the case that matters: what comes out is different code. Selecting both code lines of the first source copied `total = a*b*c`, `name = __init__`, correctly.

`plain_range` itself, given the range of both code lines, returns `total = abc\nname = init`; and for "icon \u{E000} for @ada\n" with `@ada` an atom labelled Ada Lovelace, the whole range gives `icon Ada Lovelace for Ada Lovelace` where `plain_with` gives `icon \u{E000} for Ada Lovelace`.

## Verification

Run. Chromium: a page with `<textarea markup="markdown">` under `host/web/markup-editor.js`, as `host/web/tests/markup-atoms.test.mjs` builds one, `setSelectionRange` on the editor, `navigator.clipboard.writeText` replaced to record, then `ControlOrMeta+Shift+C`; it copied the third column. The function: a test under `markdown/tests` printed the two results above. Not run: the Apple editors; that they pass the selection unwidened is read from the Swift.

## Direction

Take the skips from the whole source's analysis, as `plain_with` does (`segment.rs:392-449`), and apply the ones inside the range to `source[a..b]`. That removes the placeholders and their 0x1900 atom limit.

## Related

LLP 1045 D1 says the command "copies `plain()` of it"; LLP 1045.000 D5 already moved atoms to the whole source. Nothing in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` covers `plain_range`.
