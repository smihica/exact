# `contract test` panics when a property's app file declares no component

**Status:** Closed
**Resolution:** Every expansion entry point refuses a file without a component with analyze-no-component, so property checks exit with a diagnostic instead of panicking.
**Systems:** contract/syntax (`inline::expand`), contract/cli (`properties.rs`)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`contract_syntax::expand` and its siblings index the root component without
checking that one exists (`contract/syntax/src/inline.rs:255`,
`let source = &file.components[0];`). A compile never reaches that line with an
empty list, because the type pass refuses first (`analyze-no-component`,
`contract/types/src/lib.rs:1279-1285`). The property check does not go through the
type pass: for a test file that holds an `always`, `after` or `never` property,
`contract test` loads the paired app file and calls `expand` on it directly
(`contract/cli/src/properties.rs:40-53`). The paired file is `app.contract` beside
the test file, else the test file's name with `.test.contract` replaced by
`.contract` (`contract/cli/src/main.rs:387-395`), which is the test file itself
when its name does not end in `.test.contract`.

## The failure

Either of these:

1. `props.contract`, alone in a directory:

   ```
   always "a is one"
     expect state a == 1
   ```

   `contract test props.contract`

2. The same two lines as `app.test.contract`, beside an `app.contract` that holds
   only a shape and a `fn`.

Observed, both, exit status 101:

```
thread 'main' panicked at contract/syntax/src/inline.rs:255:34:
index out of bounds: the len is 0 but the index is 0
```

Expected: the refusal `contract build` gives the same app file,
`app.contract:1:1 [analyze-no-component] a file needs a component`, and exit 1.

Both inputs are already invalid, so no working app is affected; what is lost is
the diagnostic.

## Verification

Ran both with the debug build; each printed the panic above. `contract build` on
the second case's `app.contract` printed the `analyze-no-component` refusal and
exited 1. With no `app.contract` at all beside `app.test.contract`, `contract
test` exits 1 with `[contract-use-unreadable]`, so only a readable file with no
component panics.

## Direction

Have `expand_with_sites` return the `analyze-no-component` refusal when
`file.components` is empty, so every caller of the public `expand*` functions gets
a diagnostic. `check_suggestions_path` takes the same route
(`properties.rs:30-38`) and is fixed with it.

## Related

`issues/closed/20260901-empty-contract-panics.md` (P1) is the same index, reached
through `contract::compile("")`. Its fix refused the empty list in the type pass
and left `expand` as it was; the property check calls `expand` without that
guard. Not in `QUEUE.md` or `docs/agent-pitfalls.md`.
