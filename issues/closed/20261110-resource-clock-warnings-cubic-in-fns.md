# `resource_clock_warnings` costs the cube of the number of `fn`s on every `contract build`

**Status:** Closed
**Resolution:** Clock advice stops at an unchanged round and resolves local scopes without cloning the declaration map.
**Systems:** contract/analyze (`resource_clock_warnings`), contract/cli build
**Severity:** P3
**Author:** Claude (Fable 5.2), from the 2026-11-10 project review
**Date:** 2026-11-10

## What is wrong

Line references are at commit `dd6f5dcd`.

`resource_clock_warnings` (`contract/analyze/src/lib.rs:840-919`) finds which `fn`s
and derives read `performanceNow()`. It settles them with fixed-count loops that
never stop early (`lib.rs:891-899`):

```rust
for _ in 0..=file.fns.len() {                // F + 1 rounds
    for f in &file.fns {                     // F fns
        let mut scope = functions.clone();   // an F-entry BTreeMap<String, bool>
        …
        functions.insert(f.name.clone(), live(&f.body, &scope));
    }
}
```

That is F + 1 rounds of F clones of an F-entry map: F³, whether or not any `fn`
mentions the clock or the app has a resource. Each component's derives get D + 1
rounds over D derives (`lib.rs:906-910`), which is D² expression walks.
`contract build` and `contract types` call the pass on every run
(`contract/cli/src/build.rs:191`, `contract/cli/src/main.rs:306`).

`contract/cli/tests/growth.rs` states what the compiler is held to: "The
compiler's time grows with the plan, not its square". That test times
`contract::compile`, which does not run this pass.

## The failure

F one-line `fn`s (`fn f0(x: number): number = x + 0` …) above a component with one
state and one `text`. Whole `contract build`, best of five, on the debug binary
(the one `exact contract` runs, `scripts/exact.mjs:653`) and a release one:

| `fn`s | debug | release |
| --- | --- | --- |
| 0 | 0.007 s | 0.006 s |
| 16 | 0.008 s | 0.005 s |
| 100 | 0.077 s | 0.021 s |
| 200 | 0.52 s | 0.12 s |
| 400 | 3.9 s | 0.83 s |

`contract symbols` on the same files takes 0.006 to 0.012 s at every size. With
derives and no `fn` (`derive d0 = s + 0` …): 300 take 0.046 s and 1,200 take
0.79 s (debug), 17 times for 4 times the input.

Expected: time in proportion to the declarations. The values only change from
false to true, so a round that changes nothing ends the loop.

No app in the repository is large enough to feel it: the most `fn`s in one app is
16 (`examples/calendar`) and the most derives in one file 78, where the pass is
not measurable. It costs a tenth of a second from about 110 `fn`s.

## Verification

Timed the builds above with both binaries. A macOS `sample` of the debug binary
during the 400-`fn` build had all 1,337 samples under
`contract::resource_clock_warnings`, called from `build.rs:191`.

## Direction

Stop each loop when a round changes no entry, and remove a `fn`'s parameters from
the lookups instead of cloning the map per `fn` (or settle once in dependency
order, as `settle_derives` does for types).

## Related

`QUEUE.md`, "The Contract compiler's growth test crosses its CPU ratio bound on
main" measures `contract::compile` over component uses and does not reach this
pass.
