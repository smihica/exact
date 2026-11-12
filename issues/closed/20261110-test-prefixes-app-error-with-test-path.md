# `contract test` prints the app's compile errors under the test file's path

**Status:** Closed
**Resolution:** Property checks report app and imported-file errors at their source paths, and property errors at the test path.
**Systems:** contract/cli/src/main.rs, contract/cli/src/properties.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

When a test file has property lines (`always`, `after`), `contract test` loads the app beside
it to check what the properties read (`contract/cli/src/main.rs:384-402`). Every refusal from
that check is printed with the test file's path in front:

```rust
if let Err(errors) = contract::check_properties_path(&app, &p) {
    for e in errors {
        eprintln!("{input}:{e}");
    }
```

That is right for a property's own refusal, which carries no file and whose line is in the
test file. It is wrong when the app itself does not load, because `property_program`
(`contract/cli/src/properties.rs:40-53`) returns the app's errors through the same `Err`:

- A loader error carries its file, which `CompileError`'s `Display` prints first
  (`contract/cli/src/lib.rs:210-216`). The line then has two paths.
- An expansion error carries none (`properties.rs:51`), so it is printed as a line and column
  of the test file that belong to `app.contract`.

## The failure

`app.test.contract`:

```
always "count is whole"
  expect state count whole
```

With an `app.contract` that has a syntax error on line 4 (`text "hi" oops=`),
`contract test app.test.contract` prints

```
…/app.test.contract:…/app.contract:4:20 [syntax-expected-expression] expected an expression, found end of line
```

With an `app.contract` whose view places an unknown component on line 4 (`Missing()`), it
prints

```
…/app.test.contract:4:5 [syntax-unknown-component] unknown component `Missing`; declared components: `App`
```

and the test file has two lines. `contract build app.contract` reports both under
`…/app.contract`. Expected: the same path here.

## Verification

Ran `target/debug/contract test` on both pairs; the two lines are quoted as printed (exit 1).
A recursive component gives `…/app.test.contract:3:5 [syntax-inline-depth]` the same way.
With a valid app and a property that misspells its state the refusal is
`…/app.test.contract:2:3 [analyze-property-read] …`, which is correct.

## Direction

In `property_program`, give an error with no file the app's path, and in `main.rs` print the
test file's path only before the property check's own refusals.

## Related

None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
