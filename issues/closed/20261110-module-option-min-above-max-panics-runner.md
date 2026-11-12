# A module option declared with its `min` above its `max` compiles, and a computed value for it panics the runner

**Status:** Closed
**Resolution:** Module declarations reject an option whose min exceeds its max, naming the option before a plan reaches the runner.
**Systems:** `contract/cli/src/declaration.rs`, `runner/src/runner/modules/samples.rs`
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

A module's declaration may bound a numeric option (`{"type", "min", "max", "default"}`,
LLP 1133.013 D12 item 2). Reading the declaration checks only that each of the three
is a number (`contract/cli/src/declaration.rs:776-784`); it does not compare `min`
with `max`.

The runner clamps a computed option with `given.clamp(low, high)`
(`runner/src/runner/modules/samples.rs:114-116`). `f64::clamp` panics when `min > max`
([std](https://doc.rust-lang.org/std/primitive.f64.html#method.clamp)), so a
declaration with the two the wrong way round panics the runner at the first computed
value.

A literal value never reaches the runner: `bounded` refuses every literal for such an
option (`contract/cli/src/modules.rs:453-457`), with a range that cannot be met.

## The failure

The `contract/cli/tests/tide` fixture's declaration, with a `max` added below its
`min`:

```json
"every": { "type": "number", "min": 60000, "max": 1000, "default": 60000 }
```

```
component App
  state gap = 5000
  action periodic
    tideEvery(tag="e1", every=gap)
```

Observed: the app compiles, and `act periodic` panics in `clamp_bounds`:

```
min > max, or either was NaN. min = 60000.0, max = 1000.0
```

With a literal in place of `gap` the compiler refuses 500, 5000, 60000 and 90000
alike:

```
contract-module-bound `tideEvery`'s `every` is from 60000 to 1000, given 5000
```

Expected: the declaration is refused where it is read, naming the option, as an
option whose `min` is not a number is.

## Verification

A temporary test in `contract/cli/tests/it/` copied the fixture, edited its
`exact.json` as above, compiled the app with `contract::compile_path_all`, booted a
runner on the plan and ran `periodic` under `catch_unwind`. It printed the panic
message above, and the refusal for each of the four literals.

## Direction

In `declaration.rs`, refuse an option whose `min` is above its `max` with the check
that already refuses a `min` that is not a number.

## Related

No issue or `QUEUE.md` entry names `clamp_bounds` or a module option's bounds.
