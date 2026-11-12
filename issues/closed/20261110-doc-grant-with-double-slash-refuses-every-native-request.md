# A `doc:` grant with a doubled slash passes the bake and leaves a native host with no grants

**Status:** Closed
**Resolution:** `exact_grants` refuses a `doc:` path with an empty segment, a backslash or a NUL, as ibex2 does, so the bake names the line rather than building an app whose native hosts hold no grants.
**Systems:** grants (`PathPrefix`), vendor/ibex ibex2 grant (`PathPrefix`), bake/src/reach.rs, native executors (Apple, Linux)
**Severity:** P2
**Author:** Claude (Opus 5.5), from Grok 4.7's whole-project review (2026-11-10)
**Date:** 2026-11-10

Line references are at `46f3d608`.

## What is wrong

The two grant parsers disagree on a `doc:` path with an empty segment:

- `exact_grants::PathPrefix::new` (`grants/src/lib.rs:76-104`) skips empty segments
  (`:97`), so `doc:/photos//2024` is `doc:/photos/2024`. `grants/src/quoted_tests.rs:82`
  expects `doc:/chosen///x` to print as `doc:/chosen/x`.
- ibex2's `PathPrefix::new` (`vendor/ibex/crates/ibex2/src/grant.rs:66-99`) returns `None`
  for a `doc:` path containing `//`, `\` or NUL (`:73`), so the `fs.read`/`fs.write` arm of
  `GrantSet::parse` (`:483-490`) fails, and with it the whole set.

The bake checks a Rust data crate's `fn grants()` with `exact_runner::grants::parse`
(`bake/src/reach.rs:428`), which reads each line with `exact_grants`
(`runner/src/grants.rs:45-47`), so the app builds. Its comment (`:393-396`) says the bake
stops a set "a host that cannot parse" because "a host … holds none of it"; that holds
only while the two parsers agree. At launch, Linux and Apple parse with ibex2
(`host/linux/src/executor.rs:52-60`, `host/apple/src/executor_core.rs:1200-1212`), hold no
bindings, and refuse every request, fetch and secrets included, with "the app's grants did
not parse". The web takes the typed set `exact_grants` wrote and admits
`doc:/photos/2024`, so the same app works there.

An `app.ts` app is not affected: `exact-js`'s `Module::inspect` parses its grants with
ibex2 at bake (`js/src/lib.rs:625-628`, called from `js/bake/src/lib.rs:1142`) and fails
the build naming the line.

`host/web/tests/fixtures/grants.json`, which `grants/tests/parity.rs` runs through both
parsers, has no `doc:` path with an empty segment, so the parity test passes.

## To see it

Confirmed by reading code only. A Rust data crate whose `fn grants()` returns
`"net.fetch https://api.example\nfs.read doc:/photos//2024"` bakes; on macOS or Linux its
first fetch to `https://api.example` is refused with the parse error. Adding
`{"spec":"fs.read doc:/a//b","ok":…}` to the fixture fails `parity.rs`'s first assertion
(`native.is_ok() == parsed.is_ok()`).

## What a fix is

One `doc:` path grammar, chosen in one place. Either `exact_grants` refuses `//`, `\` and
NUL in a `doc:` path as ibex2 does (the bake then names the line, as the TypeScript bake
already does), or ibex2 drops its extra check and collapses empty segments as for every
other namespace. The first keeps the stricter rule a `doc:` path was given; which rule is
meant is the decision. Add the case to `grants.json` either way. That a single bad line
costs a native host every grant is the design the bake's check exists to cover
(`issues/closed/20260924-grant-readers-disagree.md`), so it needs no change of its own.
