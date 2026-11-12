# Return still breaks a list item's inline syntax in two cases: beside a literal backtick in code, and with a selection that starts in the list marker

**Status:** Open
**Systems:** markdown/src/edit.rs, markdown/editor/src/editor.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What remains

This issue was filed for Return inside a formatted list item destroying its formatting and
misplacing the caret. That is fixed: Return keeps bold, emphasis, code and link
destinations on both sides of the break, nested in order, and Backspace at the new item's
start removes only its block prefix. Two cases are left.

**A literal backtick in a code span.** Return next to a backtick inside a code span copies
the span's delimiter beside it. The run no longer matches the opening delimiter, so the
first item's code becomes literal syntax.

```rust
let source = "- ``a`b``";
let change = exact_markdown::edit(source, exact_markdown::Range::caret(6), exact_markdown::Command::Newline);
let after = change.apply(source);
// after == "- ``a```\n- ``b``", plain(&after) == "• ``a```\n• b"
```

Expected plain text, with both halves still code:

| Source | Caret | Expected |
|---|---|---|
| ``- ``a`b`` `` | 6 | `• a` then a backtick, `• b` |
| ``- ``a`b`` `` | 5 | `• a`, `• ` then a backtick and `b` |
| ``- ```a``b``` `` | 7 | `• a` then a backtick, `• ` then a backtick and `b` |

LLP 1045 D2 and `markdown/src/lib.rs` specify code spans without edge-space normalization.
A span whose content starts or ends with a backtick needs padding to separate it from the
delimiter, and under this dialect the padding stays visible. So the fix needs a decision on
code-span padding first.

**A selection that starts in the list marker.** When the selection begins before the
item's content, Return bypasses the inline split and exposes the link's syntax:

```rust
let source = "- [docs](https://example.com)";
let change = exact_markdown::edit(source, exact_markdown::Range::new(0, 5), exact_markdown::Command::Newline);
// change.apply(source) == "\ncs](https://example.com)"
```

Expected: `cs` stays a link to `https://example.com`. With the selection starting at the
content (`Range::new(2, 5)`) the result is right: `"- \n- [cs](https://example.com)"`.

## Verification

Ran both reproductions with the fix for the rest of this issue applied: the results in
the comments above.

## Direction

For the first, decide the padding rule, then serialize each half with valid delimiters and
unchanged displayed text. For the second, apply the inline split whether or not the
selection starts inside the block's content.
