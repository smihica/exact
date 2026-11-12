# Two recipes fail as written: fetch-loading-error's test at its fifth line, native-module's `app.ts` at the type check

**Status:** Closed
**Resolution:** The fetch recipe tests settled failure and retry; the native recipe's existing grants export type-checks.
**Systems:** docs/recipes/fetch-loading-error.md, docs/recipes/native-module.md, contract/cli/tests/it/docs.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

A recipe is printed whole by `contract vocab <name>` and copied. At `dd6f5dcd` two recipes
fail on their own text when their `contract`, `ts` and `contract-test` blocks are put in an
app `exact new` made as `app.contract`, `app.ts` and `app.test.contract`.

## 1. `fetch-loading-error.md`: the test cannot pass

`docs/recipes/fetch-loading-error.md:62-64`:

```contract-test
  fail fetch "https://jsonplaceholder.typicode.com"
  tap "refresh"
  expect tree has "loading"
```

```
$ bun exact.mjs test web
test "a failed refresh retains rows and retries": FAIL
  line 5: expected testId "loading" present, it was absent (resource feed failed: fetch failed (driver fault): https://jsonplaceholder.typicode.com/posts?_limit=10; it shows its placeholder or last value) …
0 passed, 1 failed
```

`fail fetch` rejects at once (the recipe's own line 75: "it cannot delay"), and an input
step ends with what it settled, "an answer given in the input's turn"
(`docs/contract-grammar.md:943`). So the refresh has already failed when `tap` returns, and
`pending(feed)` is never seen. With that one line removed the test passes.

Expected: the recipe's test passes on its example.

## 2. `native-module.md`: `app.ts` exports no `grants`

The `ts` block (`docs/recipes/native-module.md:41-56`) is a whole module (its imports,
`sources`, `export const answer`) with no `export const grants`. As `app.ts` beside the
recipe's Contract:

```
$ bun exact.mjs contract build app.contract --json
[{"id":"typescript-TS2339","severity":"error","message":"Property 'grants' does not exist on type 'typeof import(\"…/app\")'.","file":"__exact_entry.ts","line":4,…}]
```

With `export const grants = '';` added it prints `[]`. The other six recipes whose `ts`
block exports `answer` also export `grants`.

## Verification

Ran both on the web host. The fetch recipe was run once as written, against the public API
it names, then with the origin in `app.ts` and the test changed to a local server that
answers ten posts: the same failure at line 5, and `1 passed` with the `expect tree has
"loading"` line removed. The native-module build was run with and without the added line.
Not run: a native host.

## Direction

Drop the `expect tree has "loading"` line, or hold the reply with a test server as the
recipe's line 75 suggests, and add the `grants` line. The docs test parses `contract-test`
blocks and never runs them (`contract/cli/tests/it/docs.rs:248-250`), and reads no `ts`
block.

## Related

`QUEUE.md:7` records `docs/recipes/theme.md:7` and `QUEUE.md:9` a doubt about `symbols.md`;
neither is these. `issues/20261110-bake-finds-optional-exports-by-text.md` is the bake's
optional exports, not the required `grants`.
