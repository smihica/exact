# The summary at the top of `app.contract.d.ts` writes an enum as `string`

**Status:** Closed
**Resolution:** TypeScript summaries preserve enum literals and parenthesize enum unions before array suffixes.
**Systems:** contract/cli/src/typescript.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`contract types` opens the declaration file with one comment line per source and per shape,
"what app.ts writes, in brief", "as TypeScript types", so that a builder reads those lines and
not the declarations below (`contract/cli/src/typescript.rs:250-252`, LLP 1116.006 D7). The
summary's type printer, `inline` (`typescript.rs:342-377`), writes every `TypeKind::String`
row as `string`:

```rust
TypeKind::String => "string".into(),
```

The declarations below it do not. An enum is a string row whose fields are its members, and
its declared type is their union (`typescript.rs:44-56`). So the two halves of one file
disagree about an enum field, and the half a builder is told to read is the wider one.

## The failure

```
shape Item
  id: string
  kind: enum<"open", "done">

component App
  resource item = load() as shape Item
  view
    text "hi"
```

`contract types app.contract` prints, in the summary,

```
//   load: (args: []) => Item
//   Item = { id: string; kind: string }
```

and in the declarations

```
type T1 = "open" | "done";
type T2 = { "id": T0; "kind": T1; };
```

Expected summary: `Item = { id: string; kind: "open" | "done" }`. A data module written from
the summary that answers `kind` from a `string` variable does not type-check against
`Sources` (a `string` is not assignable to `"open" | "done"`), and nothing in the lines its
author read says why.

## Verification

Ran `target/debug/contract types` on the source above; the four lines are quoted as printed
(lines 7, 9, 237 and 238 of the output). `tsc` was not run on a module written from the
summary; the type error is TypeScript's ordinary rule for a union of literals.

## Direction

In `inline`, print a string row that has fields as the declarations do: the quoted members
joined by ` | `, in parentheses where `[]` follows.

## Related

None found in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md` for the summary or for enums
in the declaration file.
