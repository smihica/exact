# `issue.mjs check <path>` passes without reading a path it does not recognise

**Status:** Closed
**Resolution:** Named paths are normalized and read regardless of Git tracking; invalid or unreadable paths exit 2.
**Systems:** scripts/issue.mjs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`runFormatCheck` (`scripts/issue.mjs:666`) selects the files to check by filtering the
tracked list with an exact string match (lines 669 to 671):

```js
const all = trackedIssuePaths(root);            // git ls-files issues/*.md issues/closed/*.md
const selected = scoped ? all.filter((path) => paths.includes(path)) : all;
```

A named path that is not in that list, spelled that way, selects nothing. `commandCheck`
(line 872) then prints "Issue format passed." and exits 0 with no file read. Without
`--fix` it prints no count, so nothing shows that zero files were checked. Four ordinary
inputs do this:

- an issue not yet `git add`ed, which is every issue `issue.mjs new` has just written
  (`new` does not stage it);
- `./issues/<file>.md`;
- an absolute path;
- a path that does not exist.

## The failure

A file holding only `# No header` and a body line, named by its absolute path, and a
tracked issue named with a leading `./`:

```sh
bun scripts/issue.mjs check /tmp/x/issues/20261110-zz.md                        # Issue format passed.   exit 0
bun scripts/issue.mjs check ./issues/20261009-titlebar-env-display-mode.md      # Issue format passed.   exit 0
bun scripts/issue.mjs check issues/no-such-file.md                              # Issue format passed.   exit 0
```

Expected for the first: the four `missing:` diagnostics `validateIssue` gives that text
(Systems, Author, Date, Status). Expected for all: the named file is read, or the command
fails saying it is not an issue file it can check.

## Verification

Ran the three commands above; each printed "Issue format passed." and exited 0. With
`--fix`, which prints the count, the tracked issue named exactly reports "1 issue scanned,
0 rewritten", and the same file named with `./` or by its absolute path reports "0 issues
scanned, 0 rewritten". `validateIssue` on the header-less text returns `missing:Systems`,
`missing:Author`, `missing:Date`, `missing:Status`.

Not run: an untracked file inside the repository's `issues/`. It follows from
`trackedIssuePaths` (line 654), which lists only what `git ls-files` reports.

## Direction

Normalise each named path against `root`, read a named file whether or not git tracks it,
and exit 2 naming any path that is not an issue file.

## Related

`issues/closed/20260902-issue-mjs-close-leaves-the-closed.md` is about `close` and the
index. Nothing covers `check` with paths.
