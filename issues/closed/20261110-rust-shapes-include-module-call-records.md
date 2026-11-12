# `contract rust` writes a struct for every native-module call record, under the compiler's `name__options` names

**Status:** Closed
**Resolution:** Rust data bindings omit unused module call records, preserve records reached by authored shapes and data source signatures, and use camel case for scoped names
**Systems:** contract/cli/src/rust.rs, contract/cli/src/modules.rs
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`contract::rust` generates a struct for every named record in the plan
(`contract/cli/src/rust.rs:86-95`). Its purpose is the data seam: the shapes a Rust data crate
answers with (`rust.rs:1-5`; `docs/reference.md:2622-2624`). But a call of a native module is
desugared into records the compiler declares itself, named `<name>__options` and
`<name>__<platform>` (`contract/cli/src/modules.rs:207-253`). They are not the data crate's to
build: a module's names are answered by the module, their Rust binding is
`contract::module_rust`'s, which renames them (`getCurrentPosition__options` to
`GetCurrentPositionOptions`, `contract/cli/src/module_rust.rs:76-93`), and `contract types`
leaves module sources out of `SourceMap` (`contract/cli/src/typescript.rs:105-111`).

So the shapes file of an app that calls a module carries dozens of structs its data crate
cannot use, and each one draws rustc's `non_camel_case_types` warning in the crate that
`include!`s it. `cargo clippy -- -D warnings`, which the gate runs, makes that an error.

## The failure

```
$ contract rust apps/shelf/app.contract > shapes.rs
$ grep -c "pub struct" shapes.rs
115
$ grep "pub struct [a-z]" shapes.rs | head -3
pub struct addContentIndexEntry__options {
pub struct applyConstraints__apple {
pub struct applyConstraints__options {
```

Shelf's `app.contract` declares 10 shapes. 35 of the 115 structs are call records; two more,
`Point__exact` and `Rect__exact`, are shapes the loader renamed apart
(`contract/cli/src/sources.rs:1042`). Compiled as a library against the built `exact_runner`,
the file gives 37 warnings of the form ``type `write__options` should have an upper camel
case name``. Expected: no call records, and no warning.

## Verification

Ran the commands above, then `rustc --edition 2021 --crate-type lib --extern
exact_runner=target/debug/deps/libexact_runner-….rlib shapes.rs`: it compiles, with the 37
warnings. The one crate that includes this file today, `apps/harness/data`, calls no module
(and allows `clippy::all`, which does not cover this rustc lint), so no build in the tree
shows the warnings. A data crate that does call a module was not built.

## Direction

Leave the records `modules::records` declares out of `contract::rust`, as `typescript` leaves
module sources out, and give a loader-renamed shape a camel-case Rust name as
`module_rust::record_name` does.

## Related

`issues/closed/20261102-contract-rust-emits-illegal-structs.md` made the names legal Rust; it
did not ask which records are generated. QUEUE's "The Bluesky port could use `contract rust`"
is a consumer, not this.
