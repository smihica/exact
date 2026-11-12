# The instance tree keeps an error and four branches nothing reaches

**Status:** Closed
**Resolution:** Instance regions use their validated second arm, and the unused duplicate-key refusal and its classification are removed.
**Systems:** runner/src/instance/errors.rs, runner/src/instance/region.rs, runner/src/instance/foresee.rs, semantics/difftest
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`. Two leftovers describe behaviour the runner no longer has.

`InstanceError::DuplicateKey { region }` (`runner/src/instance/errors.rs:19`) is constructed nowhere. A repeated key is the data's error, absorbed (LLP 1005 §6, 2026-09-22): a `for` numbers the repeats (`runner/src/instance/region.rs:219`) and a virtualized list does the same (`runner/src/instance/collection/rekey.rs:93`). The variant's only other mention is `semantics/difftest/src/lib.rs:308`, which classes a `DuplicateKey` refusal as `"app"`: a refusal the runner cannot produce.

An `if` or `match` region always has two arms. `Plan::validate_semantics` refuses any other count (`plan/src/lib.rs:551`, `PlanError::RegionArms`), and both ways a plan is made run it (`Plan::decode`, `PlanBuilder::finish`; `runner/src/runner.rs:680` relies on that). The instance tree still asks:

- `runner/src/instance/region.rs:45`: `Value::Bool(false) if row.arms.len > 1 => Some(1)`, then `Value::Bool(false) => None` on the next line;
- `runner/src/instance/region.rs:78`: `if row.arms.len > 1 { Some(1) } else { None }`;
- `runner/src/instance/foresee.rs:115` and `:126`: `(row.arms.len > 1).then_some(1)`.

A reader takes from these that a one-arm `if` exists and shows nothing when false. It does not: an `if` without `else` compiles to a second, empty arm (`contract/lower/src/lib.rs:1068`).

## The failure

None at run time: this is code to delete.

## Verification

With the variant and its name in the difftest's table deleted, `cargo check -p exact-runner -p contract -p contract-difftest --all-targets` finished clean. Nothing else in the tree names `InstanceError::DuplicateKey` (the list index's `IndexError::DuplicateKey` is another type, and is raised).

With the one-arm outcome at each of the four sites replaced by a panic:

- `cargo test -p exact-runner --lib --test it`: 629 and 258 passed;
- `cargo test -p contract --test it`: 2,633 passed, 10 ignored;
- `cargo run -p contract-difftest -- quick`: `agreed` over 587 corpus cases and 40 random programs.

None panicked. The edits were reverted.

## Direction

Delete the variant and its name in the difftest's table, and make the four sites take arm 1 for `false` and `none` unconditionally.

## Related

`issues/closed/20260924-windowed-list-duplicate-key-poisons.md` reported the poison a duplicate key caused when the error was still raised.
