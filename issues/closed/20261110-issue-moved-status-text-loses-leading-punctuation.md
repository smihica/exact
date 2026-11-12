# `issue.mjs` drops the leading punctuation of Status text it says it moves verbatim

**Status:** Closed
**Resolution:** Moved status qualifiers retain leading punctuation, line breaks and indentation; hyphenated words remain intact.
**Systems:** scripts/issue.mjs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

The file's safety contract (`scripts/issue.mjs:29-32`) says author text is "MOVED, never
invented, reworded, truncated, or dropped". `splitStatus` (line 420) removes the status
token and then strips every leading separator character from what is left:

```js
const QUALIFIER_LEAD = /^[\s—–:;,.\-]+/;                                   // line 51
return { remainder: value.replace(exact, '').replace(QUALIFIER_LEAD, '').trim(), recognized: true };   // line 427
```

The class holds `-` and `.`, so the strip runs through the separator into the author's own
first word when that word starts with one: a flag, a dotfile, a negative number. `check
--fix` and `close` then write the shortened text and report it as "verbatim".

A second loss on the same path: a Status value that continues on the following lines is
joined with single spaces (`fullValue`, line 182), so a list under it becomes one line.

## The failure

`issues/closed/20261110-x.md` with the header line

```
**Status:** Closed: --fix now moves the qualifier
```

`fixIssue`, which `check --fix` calls and whose result it writes, returns

```
**Status:** Closed
**Resolution:** fix now moves the qualifier
```

with the change "moved the Status-line text into **Resolution:** verbatim: `fix now moves
the qualifier`". Expected: `**Resolution:** --fix now moves the qualifier`.

| Status value | text moved |
|---|---|
| `Closed - .gitignore entry added` | `gitignore entry added` |
| `Closed, -Zbuild-std is pinned` | `Zbuild-std is pinned` |
| `Closed: -1 is now refused` | `1 is now refused` |
| `Open: .exact/ is not ignored` | `exact/ is not ignored` |
| `Open-ended: needs a decision` | `ended: needs a decision` |

`close` on an open issue whose header has `**Status:** Open - --wasm builds only` prints
"kept verbatim in a body note: wasm builds only" and writes that. On

```
**Status:** Open, blocked on:
- the kernel change
- the web host
```

it writes the note `blocked on: - the kernel change - the web host`.

## Verification

Called `splitStatus` and `fixIssue` on the inputs above, and ran
`main(['close', 'issues/<file>', '--resolution', 'fixed by abc'], root)` on issue files in
a directory outside the repository; the outputs above are what they returned and wrote.
Separators alone are handled as intended: `Closed: fixed by abc`, `Closed — fixed` and
`Closed (fixed on main)` keep their text whole.

## Direction

Strip one separator and the whitespace around it, not a run of separator characters. A
hyphen directly after the token (`Open-ended`) joins a word, so that value is not a status
token. Keep the line breaks of a wrapped value when it goes to a body note.

## Related

`issues/closed/20260902-issue-mjs-close-leaves-the-closed.md` is about the index after
`close`, not the moved text.
