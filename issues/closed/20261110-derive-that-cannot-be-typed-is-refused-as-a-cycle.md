# A derive whose type cannot be inferred is refused as depending on itself when no derive cycle exists

**Status:** Closed
**Resolution:** Untyped literals report type-cannot-infer, dependent derives leave the refusal to its cause, and genuine derive cycles retain type-derive-cycle.
**Systems:** contract/types/src/component.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at `dd6f5dcd`. After the derives settle, any derive whose
type is still incomplete is refused as `type-derive-cycle`
(`contract/types/src/component.rs:253-281`), on the stated ground that "what
types only incompletely depends on itself" (`:256`). One other cause is
recognised, an `[]` that nothing types (`empty_list_in`, `:260-268`). Two more
exist and get the cycle message:

1. A `none` that nothing types: `derive d = none`.
2. A read of a state whose type was never fixed: `state picked = none` that no
   action writes, read by a derive.

The comment at `:242-244` says a derive that reads a refused one is "refused
with it, not reported again as a cycle". That covers only a derive whose
inference returned an error (`refused` starts from `Result::is_err`, `:246`),
so a derive that reads an incomplete one is reported as a cycle of its own.

## The failure

```
component App
  derive first = none
  view
    text "x"
```

```
2:3 [type-derive-cycle] cannot infer the type of `first`: it depends on itself through other derives
```

There is no other derive. `derive d = []` in its place gets a true message:
"cannot infer what `[]` holds in `d`: pair it with a typed arm (`match`/`?:`)
or write it where a `T[]` is declared".

```
component App
  state picked = none
  derive current = picked
  view
    text "x"
```

```
3:3 [type-derive-cycle] cannot infer the type of `current`: it depends on itself through other derives
2:3 [type-cannot-infer] cannot infer the type of `picked`: nothing writes a value into it
```

The second line is the cause; the first is false and is printed first. With
`derive d = on ? none : none` and `derive e = d`, both derives get the cycle
message.

Expected: for an untyped `none`, a `type-cannot-infer` message like the one
for `[]`; for a derive that reads an untyped state or an incomplete derive, no
diagnostic of its own.

## Verification

Ran `contract build` (debug) on each program named above: the output shown. A
real cycle (`derive a = b`, `derive b = a`) gets the same message for both
derives, which is correct there.

## Direction

In the loop at `:253`, treat an untyped `none` leaf as `empty_list_in` treats
`[]`; seed `refused` with the derives that read an incomplete state, and
propagate it through derives that are incomplete as well as those that failed.
Keep `type-derive-cycle` for what remains.

## Related

None in `issues/`, `QUEUE.md` or `docs/agent-pitfalls.md`.
