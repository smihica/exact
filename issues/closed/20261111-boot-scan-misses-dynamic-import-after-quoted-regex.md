# `boot` passes a dynamic import in `navigation.js`: a regular expression holding quotes puts its scanner inside a string

**Status:** Closed
**Resolution:** Boot rejects dynamic import text even after quoted regular expressions; page-store retries load their shared-lock helper through loadAfterPaint.
**Systems:** scripts/boot.mjs, host/web/navigation.js
**Severity:** P2
**Author:** Claude (Fable 5.2), from the fix lanes of the 2026-11-10 project review
**Date:** 2026-11-11

## What is wrong

Line references are at commit `1fb06ccb`.

`scripts/boot.mjs:174` reports a dynamic import in a module loaded before first pixel by testing `codeOnly(source)` for `import(`. `codeOnly` (`boot.mjs:97-128`) blanks comments, strings and templates, and has no state for a regular-expression literal. `host/web/navigation.js:557` holds one with quotes in it:

```js
.replace(/"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/g, '""')
```

The scanner reads its quotes as string delimiters and comes out of the expression inside a "string". From there it takes code for string text and string text for code. `navigation.js:1125` holds a dynamic import:

```js
entry.retry ??= import('./storage-environment.js').then(m => m.retrySecretWrite(key, live, write, locks))
```

The scanner is inside a "string" there, so the import is blanked and the check passes.

## The failure

`bun scripts/boot.mjs` prints "Allowed import paths only" and exits 0 with a dynamic import in one of the two modules it allows before first pixel. Expected: `dynamic import before first pixel in host/web/navigation.js`, which is what it reports when `pageStore` sits in a file with no such expression before it.

Everything else the host loads after paint goes through `loadAfterPaint` (`glue.js:17`; LLP 1047 D5), a script tag, so the two modules hold no other `import(`.

## Verification

`codeOnly`, taken from `boot.mjs` and run over `navigation.js`, returns text with no `import(`; the file's text has one, on line 1125. A copy of its state machine that records each state: line 557 is the first to end inside a "string", 707 line ends follow in that state, and the scanner is in it at the `import(` on line 1125. `glue.js` ends no line inside a string.

The lane for `issues/20261110-line-cap-met-by-packed-lines-and-overflow-files.md` moved `pageStore` into a file of its own and `boot` then failed with the message above.

## Direction

Two changes. The retry loads `storage-environment.js` as the other after-paint pieces load: `pageStore` takes the loader from `glue.js`, and the two modules hold no `import(`. And the scan fails closed, as the file's own comment asks of its HTML tokenizer: either it tests the whole text, comments and strings included, so an allowed module spells `import(` nowhere, or `codeOnly` gains a regular-expression state. A case in `scripts/caps.test.mjs` puts a dynamic import after a regular expression that holds a quote.

## Related

`issues/20261110-line-cap-met-by-packed-lines-and-overflow-files.md`: its split of `navigation.js` waits on this, since a moved `pageStore` fails the check.
