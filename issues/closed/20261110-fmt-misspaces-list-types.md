# `contract fmt` writes `key = x` on a `for` row and a gate, and `string[] []` and `option<number[] >` for a list inside a type

**Status:** Closed
**Resolution:** The formatter keeps loop and gate keys tight and consumes both list brackets before formatting an enclosing type.
**Systems:** contract/syntax/src/fmt.rs (`Layout`)
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`Layout` records the tokens that print with no space before them. It misses two
kinds, so `contract fmt` rewrites spellings the guides write tight, and
`fmt --check` reports them.

**The `key=` of a `for` row and of a gate.** `=` stays against a name only when the
name's span is in `Layout::attributes` (`contract/syntax/src/fmt.rs:542-549`).
`Layout::file` adds the `key` of a resource over a list, under the comment "A
list's `key=`, tight as a `for` row's" (`fmt.rs:338-350`). The `for` row is never
added: `Layout::nodes` only descends into its body (`fmt.rs:439`). A task's and a
sequence's gate are not visited.

**A list inside another type.** `Layout::ty` (`fmt.rs:359-378`) handles `T[]` in
the arm written for `option<T>`:

```rust
TypeExpr::List(inner, _) | TypeExpr::Option(inner, _) => {
    let end = self.ty(inner);
    self.type_angles.insert(self.tokens[start + 1].span);
    self.type_angles.insert(self.tokens[end].span);
    end + 1
}
```

For `T[]` the token at `end` is `[`, so the arm returns the index of `]`, one
short. The enclosing type marks that `]` as its own bracket and leaves its real
`[` or `>` unmarked. A lone `T[]` prints well because nothing reads the result.

## The failure

```
shape S
  a: option<number[]>
  b: string[][]
  c: option<string>[]

component App
  state xs = [1, 2]
  state on = true
  state n = 0
  action tick
    n = n + 1
  task t when on key=n
    after(1000, tick)
  view
    column
      for x in xs key=x
        text `${x}${n}`
```

`contract fmt --check` exits 1 and changes four lines:

```
+  a: option<number[] >
+  b: string[] []
+  task t when on key = n
+      for x in xs key = x
```

Expected: no change, as for `c`. The guides write both tight:
`for d in days key=d.key` (`docs/contract-for-agents.md:849`), `for c in colors
key=c` and `option<number>[]` (`docs/contract-grammar.md:2212`, `:659`).

The output is valid and means the same: the rewritten file compiles to a
byte-identical plan and `fmt --check` then accepts it. The cost is the spelling.
`fmt --check apps/caltrain/app.contract` reports every keyed `for` row, 33 apps
write the tight form, and the spaced form is now in seven (`apps/shelf` and
`apps/realworld`, nine rows each; `apps/design-tool/app.contract` holds 2 spaced
and 29 tight).

## Verification

Ran `contract fmt --check` and `contract fmt --stdout` (debug build) on the
program above, then built the original and the formatted copy and compared the
plans with `cmp`. The same spacing came from `number[][][]` (`number[][] []`),
`option<number[][]>`, an action's and a `fn`'s parameter, a `fn`'s result, a
prop, `task t key=n` and `sequence s when on key=n`. A resource's
`for id in ids key=id` stayed tight.

None of the 753 tracked `.contract` files nests a list in another type; packages
declare such types (`packages/contacts/exact.json`, `"option<string[]>"`).

## Direction

Give `TypeExpr::List` its own arm: mark the `[` at `end` and return `end + 2`. In
`Layout::nodes` add a `for` row's `key` word to `attributes` as the resource
branch does, and do the same for a task's and a sequence's gate.

## Related

`QUEUE.md`, "`contract fmt` refuses a typed state": a state's declared type is not
visited at all, a different omission in `Layout::file`. Visiting it will reach the
list arm.
